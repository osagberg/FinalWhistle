//! Per-tick player decision dispatcher — ADR-0006 `dispatch_tick`.
//!
//! ## Design
//!
//! `dispatch_tick` iterates roster slots 0..22. For each slot where
//! `should_decide` fires, it:
//! 1. Checks pre-emption hooks (stubbed to `None` in -iii-a).
//! 2. Evaluates FSM transitions (skeleton: identity) per ADR-0006 §"Concrete
//!    sketch" — the transition runs BEFORE the BT/GK lookup.
//! 3. Routes by role: GK → `goalkeeper_fsm::tick_goalkeeper`; outfield →
//!    `bt::tick_tree` via `SubtreeLibrary`.
//! 4. Applies the returned `PlayerIntent` via `apply_intent` (mutates
//!    `vel_x`/`vel_y`).
//! 5. Increments `local_decision_counter` via `bump_decision_counter()`.
//!
//! ## Slot indexing convention (P2-3)
//!
//! Two different 0/1-indexed values live in this function:
//! - `slot_idx` (0-indexed, 0..22): Vec index into `state.players`.
//! - `roster_slot` (1-indexed, 1..=22): the value `should_decide` expects
//!   per `decision_cadence`'s contract (it subtracts 1 internally).
//! - `formation_slot` (0-indexed, 0..22): same as `slot_idx`; named
//!   separately where it's passed to `formation_position` / `BtContext`
//!   to document which convention is in use.
//!
//! ## Determinism
//!
//! - Roster slots are iterated in fixed order (0..22).
//! - RNG is seeded per ADR-0009: `seed_fn(match_seed, tick, SeedLayer::Decision,
//!   (player_id << 16) | local_decision_counter)`.
//! - In the skeleton tier (-iii-a), no leaf actually draws from the RNG.
//!   The seed is constructed for -iii-b compatibility.
//! - No floats, no HashMap, no clocks, no async.
//!
//! ## Player velocity model (skeleton tier)
//!
//! `apply_intent` for `MoveToPosition` computes a direction vector from the
//! player's current position to the target, then clamps magnitude to
//! `MAX_PLAYER_SPEED`. Direct velocity set (no acceleration model) — adequate
//! for the skeleton tier. -iii-b will add an acceleration ramp.
//!
//! ## Pre-emption hooks (stub)
//!
//! Universal pre-emption hooks (single-chaser claim, foul reaction,
//! set-piece switchover) are stubbed to return `None`. They will be wired
//! in -iii-b / T1-4 once `MatchEvent` exists.

use std::collections::BTreeMap;

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;

use fw_content::{CooldownPolicy, SignatureDefinition, SimBiasSnapshot, StackingPolicy};
use fw_core::{Q32, Tick};

use crate::MatchState;
use crate::bt::{BtContext, LeafKind, Node, Tree, tick_tree};
use crate::decision_cadence::{SeedLayer, seed_fn, should_decide};
use crate::goalkeeper_fsm::tick_goalkeeper;
use crate::role_states::{PlayerIntent, PlayerRoleState};
use crate::signature;
use crate::signature::{
    DEFAULT_FIRING_DURATION_TICKS, SignatureFiring, build_trigger_table, evaluate_signatures,
};
use crate::subtree_library::select_outfield_intent;
use fw_content::{MatchEvent, PassKind, is_shot_on_target};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum player speed in m/s (skeleton tier). Direct vel-set; no
/// acceleration model. -iii-b introduces an acceleration ramp.
///
/// 8 m/s ≈ a brisk run / moderate sprint. Raised from 5 m/s at T1-15
/// so outfield players can close a ~10m gap toward a loose ball within
/// ~10 ticks (~0.17s) rather than never converging at the old 5 m/s jog.
/// In Q32.32 format: 8 × 2^32 = 8 << 32.
const MAX_PLAYER_SPEED: Q32 = Q32::from_raw(8_i64 << 32); // 8.0 in Q32.32

// ---------------------------------------------------------------------------
// Ball-speed constants (T1-3.5)
// ---------------------------------------------------------------------------
// Shot speed: base 20 m/s + up to 15 m/s bonus at peak shooter attrs.
// Full formula: base + bonus × (strength × finishing).
// At mid-range attrs (0.5 × 0.5 = 0.25): 20 + 15 × 0.25 = 23.75 m/s.
// At peak attrs (1.0 × 1.0 = 1.0): 20 + 15 × 1.0 = 35 m/s.
//
// Pass speed: base 15 m/s + up to 10 m/s bonus at peak passer attrs.
// Full formula: base + bonus × (passing × vision).
// At mid-range attrs (0.5 × 0.5 = 0.25): 15 + 10 × 0.25 = 17.5 m/s.
// At peak attrs (1.0 × 1.0 = 1.0): 15 + 10 × 1.0 = 25 m/s.
//
// All values in Q32.32: X m/s = X << 32 raw bits.

/// Base shot speed (m/s) before attribute scaling.
const SHOT_BASE_SPEED_MPS: Q32 = Q32::from_raw(20_i64 << 32);

/// Peak shot speed bonus (m/s) at maximum `strength × finishing` product.
/// Applied as: speed = SHOT_BASE + SHOT_PEAK_BONUS × (strength × finishing).
const SHOT_PEAK_BONUS_MPS: Q32 = Q32::from_raw(15_i64 << 32);

/// Base pass speed (m/s) before attribute scaling.
const PASS_BASE_SPEED_MPS: Q32 = Q32::from_raw(15_i64 << 32);

/// Peak pass speed bonus (m/s) at maximum `passing × vision` product.
/// Applied as: speed = PASS_BASE + PASS_PEAK_BONUS × (passing × vision).
const PASS_PEAK_BONUS_MPS: Q32 = Q32::from_raw(10_i64 << 32);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Ball-speed helper functions (T1-3.5)
// ---------------------------------------------------------------------------

/// Compute the scalar ball speed (m/s) for a shot, attribute-modulated.
///
/// Formula: `SHOT_BASE_SPEED_MPS + SHOT_PEAK_BONUS_MPS × (strength × finishing)`
///
/// Both `strength` and `finishing` are Q32 values in `[0, 1]`. Their product
/// is also in `[0, 1]` (multiplication of two sub-unit Q32 values).
///
/// Pure function — no RNG, no side effects. Q32 arithmetic only.
pub(crate) fn compute_ball_speed_for_shot(shooter: &crate::player::PlayerState) -> Q32 {
    let attr_product =
        shooter.attributes.physical.strength * shooter.attributes.technical.finishing;
    SHOT_BASE_SPEED_MPS + SHOT_PEAK_BONUS_MPS * attr_product
}

/// Compute the scalar ball speed (m/s) for a pass, attribute-modulated.
///
/// Formula: `PASS_BASE_SPEED_MPS + PASS_PEAK_BONUS_MPS × (passing × vision)`
///
/// Both `passing` and `vision` are Q32 values in `[0, 1]`.
///
/// Pure function — no RNG, no side effects. Q32 arithmetic only.
pub(crate) fn compute_ball_speed_for_pass(passer: &crate::player::PlayerState) -> Q32 {
    let attr_product = passer.attributes.technical.passing * passer.attributes.mental.vision;
    PASS_BASE_SPEED_MPS + PASS_PEAK_BONUS_MPS * attr_product
}

/// Compute the ball velocity components (vel_x, vel_y) for a kick from
/// `from_pos` toward `to_pos`, with the given scalar `speed` (m/s).
///
/// Uses cordic-backed `Q32::sqrt` for the normalisation — same path as
/// `separation.rs::resolve_pair`. No `f64` division or `f64::sqrt`.
///
/// ## Zero-distance fallback
///
/// When `from_pos == to_pos` (zero distance), the ball is kicked straight
/// along +X at the given speed. This is deterministic and avoids division by
/// zero — the same convention as `separation.rs`'s EPSILON fallback.
///
/// ## Return value
///
/// `(vel_x, vel_y)` in Q32.32 m/s. The Z component is left unchanged by the
/// caller (aerial trajectory is a Phase-2 concern; for now, kicks are
/// treated as ground-level — ball.vel_z is reset to Q32::ZERO by the caller).
///
/// ## Zero-distance fallback (Codex 2026-05-16 audit code-reviewer Critical #2)
///
/// When `from == to` (zero distance — e.g. clustered players where the
/// passer's nearest-teammate is co-located), this function returns
/// `(Q32::ZERO, Q32::ZERO)` instead of the prior `(speed, Q32::ZERO)`
/// "+X-at-full-speed" fallback. Rationale: the prior fallback could fire
/// the ball along +X at 15–35 m/s with the passer near the positive goal
/// line, producing a **phantom goal** on the next tick attributed to the
/// passer's own team. The zero-velocity fallback means a self-pass-to-
/// coincident-receiver produces no ball motion (the Pass MatchEvent still
/// emits — possession transfer is symbolic — but the ball stays put,
/// which is the safe-default semantics for the degenerate case).
///
/// Production-path risk for the degenerate case is low (separation runs
/// at tick_match step 8 keeping players ≥0.4m apart; identical Q32
/// positions across two slots would need an active-tick mid-overlap
/// before separation fires); flagged here so a future ball-physics audit
/// has the rationale in source.
pub(crate) fn ball_unit_vel(
    from_x: Q32,
    from_y: Q32,
    to_x: Q32,
    to_y: Q32,
    speed: Q32,
) -> (Q32, Q32) {
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    // dist_sq in Q32 — product of two Q32 values (both in metres ≈ [-105, 105]).
    // Squares can be up to ~11025 m², well within Q32's ±2^31 integer range.
    let dist_sq = dx * dx + dy * dy;

    if dist_sq == Q32::ZERO {
        // Degenerate: passer is at the receiver's exact position. Return
        // zero velocity to avoid the phantom-goal risk documented above.
        // Speed parameter is intentionally unused on this branch.
        let _ = speed;
        return (Q32::ZERO, Q32::ZERO);
    }

    // cordic sqrt — Q32-backed, same as separation.rs.
    let dist = dist_sq.sqrt();
    // unit_x = dx / dist; unit_y = dy / dist.
    // Q32 division: (dx / dist) × speed.
    let vel_x = dx / dist * speed;
    let vel_y = dy / dist * speed;
    (vel_x, vel_y)
}

/// Extract the `BiasCategory` as a usize index (0..4) from a `StackingPolicy`.
/// Used to index into `signature_firing[slot][cat_idx]`.
fn stacking_category_idx(policy: &StackingPolicy) -> usize {
    match *policy {
        StackingPolicy::Exclusive { category } => category as usize,
    }
}

// ---------------------------------------------------------------------------
// dispatch_tick
// ---------------------------------------------------------------------------

/// Advance all player decisions by one tick.
///
/// This is the canonical-state-mutating entry point for the per-player
/// decision layer (ADR-0006). Called from `tick_match` after ball physics
/// and the tactic-FSM heartbeat.
///
/// Roster slots 0..22 are iterated in fixed order. For each slot where
/// `should_decide` fires, the role-appropriate runner executes and the
/// returned `PlayerIntent` is applied.
///
/// `sig_definitions` — map from `SignatureId.as_str()` to `SignatureDefinition`,
/// used by the signature dispatcher. Pass `&BTreeMap::new()` when no content
/// store is available (no signatures will fire without definitions).
pub fn dispatch_tick(
    mut state: MatchState,
    sig_definitions: &BTreeMap<String, SignatureDefinition>,
) -> MatchState {
    // T1-4a: match_events is the persistent canonical event stream (replaces
    // the removed signature_memory_events scratch buffer). It is NOT cleared
    // here — events accumulate across the match by design.

    // Build the trigger table once per tick (cheap: it's a BTreeMap of fn ptrs).
    let trigger_table = build_trigger_table();

    // Advance firing windows: clear expired signature_firing entries per (slot, category) lane.
    // Must run before the per-slot decision loop so the stacking check sees
    // up-to-date firing state.
    for slot_idx in 0..22usize {
        for cat_idx in 0..4usize {
            if let Some(firing) = &state.signature_firing[slot_idx][cat_idx]
                && !firing.is_active(state.tick)
            {
                state.signature_firing[slot_idx][cat_idx] = None;
            }
        }
    }

    // T1-3.6: carrier routing pre-pass.
    //
    // Runs BEFORE the per-slot decision loop. Updates role_state for ALL 22
    // slots based on current possession, independently of whether the slot
    // fires a decision this tick. This ensures:
    //   (a) The carrier's role_state is InPossession when their decision tick
    //       arrives — select_outfield_intent routes them to on-ball candidates.
    //   (b) A player whose possession was transferred away (via Pass/Shot in a
    //       previous tick) exits InPossession immediately, not only at their
    //       next decision tick.
    //
    // T1-3.6 self-review P1-1 (type-design-analyzer): this pre-pass delegates
    // to `PlayerRoleState::evaluate_transitions` rather than open-coding the
    // transition table again. There is exactly one source of truth for
    // carrier ↔ non-carrier routing. The per-slot loop below calls the same
    // function via `current_role_state.evaluate_transitions(...)` at the
    // decision tick — but for non-deciding slots, this pre-pass is the only
    // place transitions land. Idempotency holds: calling it twice in the same
    // tick (pre-pass + per-slot loop on a deciding tick) is a no-op on the
    // second call because the role state has already converged.
    for slot_idx in 0..22usize {
        let current = state.players[slot_idx].role_state;
        let next = current.evaluate_transitions(&state, slot_idx);
        state.players[slot_idx].role_state = next;
    }

    for slot_idx in 0..22usize {
        // roster_slot is 1-indexed per decision_cadence::should_decide contract.
        // (should_decide subtracts 1 internally to derive the slot array index.)
        let roster_slot = (slot_idx + 1) as u8; // 1-indexed per spec

        if !should_decide(
            roster_slot,
            &state.decision_slots,
            &state.interrupt_cooldown_until,
            state.tick,
        ) {
            continue;
        }

        // Pre-emption hooks — stubbed; -iii-b wires MatchEvent-driven hooks.
        if let Some(preempt_intent) = preempt_check(&state, slot_idx) {
            apply_intent(&mut state, slot_idx, preempt_intent);
            state.players[slot_idx].bump_decision_counter();
            continue;
        }

        // T1-2b-iv: evaluate signature triggers for this player.
        // Runs before role dispatch so the picked signature (if any) is in
        // `signature_firing[slot_idx]` when utility scoring reads the bias.
        let slot = slot_idx as u8;
        {
            // Clone candidates to avoid aliasing with &mut state below.
            let candidates = state.players[slot_idx].signature_candidates.clone();
            // active_firings: per-category snapshot for stacking check.
            // Clone all 4 lanes so we can pass them to evaluate_signatures
            // while state is borrowed mutably below.
            let active_firings: [Option<SignatureFiring>; 4] = [
                state.signature_firing[slot_idx][0].clone(),
                state.signature_firing[slot_idx][1].clone(),
                state.signature_firing[slot_idx][2].clone(),
                state.signature_firing[slot_idx][3].clone(),
            ];
            if let Some((sig_id, sig_def)) = evaluate_signatures(
                &state,
                slot,
                &candidates,
                sig_definitions,
                &trigger_table,
                &active_firings,
            ) {
                // Determine cooldown end tick from the definition's CooldownPolicy.
                let cooldown_end_tick = match sig_def.cooldown {
                    CooldownPolicy::EveryTicks(n) => Tick::from_raw(state.tick.to_raw() + n as i64),
                    CooldownPolicy::PerMatchCount(_) => {
                        // For PerMatchCount, use the 600-tick default as the
                        // intra-match spacing; the count limit enforced at T2-4.
                        Tick::from_raw(state.tick.to_raw() + 600)
                    }
                };
                // Set cooldown.
                state
                    .signature_cooldowns
                    .insert((slot, sig_id.clone()), cooldown_end_tick);
                // Determine the category of the firing signature from its definition.
                let cat_idx = stacking_category_idx(&sig_def.stacking);
                // Set firing window in the correct category lane.
                state.signature_firing[slot_idx][cat_idx] = Some(SignatureFiring {
                    id: sig_id.clone(),
                    start_tick: state.tick,
                    duration_ticks: DEFAULT_FIRING_DURATION_TICKS,
                });
                // Emit SignatureFirstFired if first time this match.
                // T1-4a: push to match_events (persistent canonical stream),
                // replacing the removed signature_memory_events scratch buffer.
                let first_fired_key = (slot, sig_id.clone());
                if !state.signature_first_fired_seen.contains(&first_fired_key) {
                    state.signature_first_fired_seen.insert(first_fired_key);
                    state.match_events.push(MatchEvent::SignatureFirstFired {
                        player_slot: slot,
                        signature_id: sig_id,
                        tick: state.tick,
                    });
                }
            }
        }

        // Build ADR-0009 RNG for this decision.
        // site = (player_slot << 16) | local_decision_counter — truncated to u32.
        // Per ADR-0009: site is u32; the top 16 bits carry the slot (0..22 fits
        // in 5 bits), the low 16 bits carry the decision counter (u32 with
        // headroom per PlayerState docs). The counter is bounded to u16 range
        // for the site encoding; overflow into the slot bits would produce
        // collisions but is practically impossible in a 90-minute match.
        let counter = state.players[slot_idx].decision_counter();
        let site = ((slot_idx as u32) << 16) | (counter & 0xFFFF);
        // Tick is u32 per ADR-0009 (fw-core::seed::seed_fn takes tick: u32).
        // Tick::to_raw() returns i64; tick is monotonically non-negative so
        // the cast to u32 is safe for ~1 billion ticks (~194 days at 60 Hz).
        let tick_u32 = state.tick.to_raw() as u32;
        // UtilityTieBreak is the correct layer for softmax sampling over
        // utility-scored candidates (ADR-0009 §SeedLayer discriminants).
        // SeedLayer::Decision is reserved for binary decision draws
        // (e.g. GK shot-stopping direction) which are not yet wired.
        let rng_seed = seed_fn(
            state.seed.to_u64(),
            tick_u32,
            SeedLayer::UtilityTieBreak,
            site,
        );
        let mut rng = ChaCha8Rng::seed_from_u64(rng_seed);

        // P1-4: evaluate FSM transitions BEFORE subtree lookup, per ADR-0006
        // §"Concrete sketch". In skeleton tier this is always identity.
        // formation_slot is the 0-indexed slot for BtContext / SubtreeLibrary.
        let formation_slot = slot_idx as u8; // 0-indexed for formation_position / BtContext
        let current_role_state = state.players[slot_idx].role_state;
        let next_role_state = current_role_state.evaluate_transitions(&state, slot_idx);
        // Write back the (possibly updated) role state.
        state.players[slot_idx].role_state = next_role_state;

        let intent = match next_role_state {
            PlayerRoleState::Goalkeeper(gk_state) => {
                let player = &state.players[slot_idx];
                let (new_gk_state, gk_intent) =
                    tick_goalkeeper(gk_state, player, formation_slot, &state.ball, &mut rng);
                // Write back the new GK state.
                state.players[slot_idx].role_state = PlayerRoleState::Goalkeeper(new_gk_state);
                gk_intent
            }
            PlayerRoleState::Defender(_)
            | PlayerRoleState::Midfielder(_)
            | PlayerRoleState::Forward(_) => {
                // ADR-0006 P1-3: outfield roles use FSM-of-BTs. Route through
                // `bt::tick_tree` where the `OutfieldSelect` leaf invokes
                // `select_outfield_intent` (utility-scored softmax).
                // The BtContext carries what the leaf needs to call the select fn.
                let player = &state.players[slot_idx];
                // Resolve active signature bias: COMPOSITE-FOLD across all
                // active per-category lanes (Codex Tier-2 re-audit P1 closure).
                // ADR-0011 §"Stacking policy" allows cross-category concurrent
                // signatures BECAUSE their bias surfaces don't overlap; the
                // composite multiplies each *_mul field across all lanes
                // (Q32::ONE when no firings). Local Option holds the owned
                // SimBiasSnapshot so the &-borrow in BtContext stays valid
                // through the tick_tree call.
                let active_bias_owned: Option<SimBiasSnapshot> =
                    signature::dispatcher::combine_active_biases(
                        &state.signature_firing[slot_idx],
                        sig_definitions,
                    );
                let active_bias: Option<&SimBiasSnapshot> = active_bias_owned.as_ref();
                // Build a minimal tree: single OutfieldSelect leaf. The leaf resolves
                // role state → candidate list → softmax pick inside tick_tree.
                // Content-pack RON trees replace this stub at T2-3.
                let outfield_tree = Tree::new(Node::Leaf(LeafKind::OutfieldSelect));
                let ctx = BtContext {
                    roster_slot: formation_slot,
                    outfield_role_state: Some(next_role_state),
                    player: Some(player),
                    active_bias,
                    select_fn: Some(select_outfield_intent),
                };
                let (_, outfield_intent) = tick_tree(&outfield_tree, &ctx, &mut rng);
                outfield_intent
            }
        };

        apply_intent(&mut state, slot_idx, intent);
        state.players[slot_idx].bump_decision_counter();
    }

    state
}

// ---------------------------------------------------------------------------
// apply_intent
// ---------------------------------------------------------------------------

/// Apply a `PlayerIntent` to a player by mutating their `vel_x`/`vel_y`
/// and emitting any corresponding `MatchEvent` entries.
///
/// ## Event emissions (T1-4a)
///
/// - `AttemptShot` → `MatchEvent::Shot` (before velocity update; happens regardless of outcome)
/// - `AttemptPassShort` → `MatchEvent::Pass { kind: Short }`
/// - `AttemptPassLong` → `MatchEvent::Pass { kind: Long }`
/// - `Cross` → `MatchEvent::Pass { kind: Cross }`
/// - `LayOff` → `MatchEvent::Pass { kind: LayOff }`
///
/// All other intents update velocity only; no event emitted.
///
/// ## T1 approximations
///
/// - `Shot.on_target`: derived from `target_y` within ±3.66 m half-width.
///   No keeper model yet.
/// - `Pass.to_slot`: nearest teammate heuristic — the nearest same-team
///   player to the target point. T2 will refine with passing-lane model.
/// - `Pass.completed`: always `true` in T1 (no contest physics yet).
///
/// ## Velocity model
///
/// All variants with a target use the same velocity-toward-target model:
/// clamp each component to `±MAX_PLAYER_SPEED` independently. No 2D
/// normalisation — diagonal movement is slightly faster than cardinal but
/// acceptable for the skeleton tier.
/// T1 placeholder for `MatchEvent::Pass.completed`. Always `true` until the
/// contest model lands in T2 — at which point this const becomes a function
/// of the contest outcome AND every reference here must become a real bool.
///
/// Why a named const instead of literal `true`: a single rename (grep
/// `T1_PASS_COMPLETED`) surfaces every pass-emission site for T2 wiring.
/// A scattered `completed: true` would silently leak past the contest model
/// for any site the T2 author missed. Codex Tier-2 P1-4 on T1-4a
/// (2026-05-16).
const T1_PASS_COMPLETED: bool = true;

pub fn apply_intent(state: &mut MatchState, slot_idx: usize, intent: PlayerIntent) {
    // Emit events BEFORE mutating velocity. The emission match is EXHAUSTIVE
    // (no `_` wildcard) so that adding a new `PlayerIntent` variant produces
    // a compile error here — forcing the author to decide whether the new
    // variant emits a `MatchEvent` or not. Codex Tier-2 P0-1 on T1-4a
    // (2026-05-16) caught a `_ => {}` catch-all that would have silently
    // dropped events for any future pass-like variant (ThroughBall, Backheel,
    // OneTwo, etc.). Mirrors the T1-2b-iv `intent_to_bias_consideration`
    // wildcard removal lesson (P0-3 fix-pass).
    // T1-3.5: ball mutation + possession state update.
    // Runs BEFORE the velocity-update match below. Mutations are:
    //   AttemptShot: ball.vel toward target, speed from shooter attrs,
    //                possession → None (loose), last_touched_by → shooter.
    //   Pass-class (Short/Long/Cross/LayOff): ball.vel toward to_slot pos,
    //                speed from passer attrs, possession → Some(to_slot),
    //                last_touched_by → from_slot.
    //   Dribble: possession stays with dribbler, last_touched_by → dribbler,
    //            ball.pos snaps to player.pos (ball "at feet"), vel zeroed.
    //   GkDistributeShort/Long: mirror pass treatment from GK slot.
    //   All others: no ball mutation.
    //
    // MatchEvent emission is interleaved here so the event and the ball
    // mutation are co-located (atomic per intent; no split between the two
    // match arms). The possession/last_touched_by updates also happen here
    // so downstream code in tick_match (goal detection, step 7) sees the
    // updated state immediately after apply_intent returns.
    match &intent {
        PlayerIntent::AttemptShot { target_x, target_y } => {
            let shooter_slot = state.players[slot_idx].slot;
            let on_target = is_shot_on_target(*target_y);
            state.match_events.push(MatchEvent::Shot {
                shooter_slot,
                tick: state.tick,
                target_x: *target_x,
                target_y: *target_y,
                on_target,
            });
            // T1-15: snap ball to shooter's feet before kicking. Without this
            // the ball starts from its last physical position (often center
            // spot after kick-off) rather than the shooter's feet, causing
            // shots to travel from center and miss the goal entirely.
            let from_x = state.players[slot_idx].pos_x;
            let from_y = state.players[slot_idx].pos_y;
            state.ball.pos_x = from_x;
            state.ball.pos_y = from_y;
            // T1-3.5: ball mutation — kick toward target.
            let speed = compute_ball_speed_for_shot(&state.players[slot_idx]);
            let (bvx, bvy) = ball_unit_vel(from_x, from_y, *target_x, *target_y, speed);
            state.ball.vel_x = bvx;
            state.ball.vel_y = bvy;
            state.ball.vel_z = Q32::ZERO; // ground-level shot in T1
            // Possession: shot releases the ball.
            state.possession = None;
            state.last_touched_by = Some(shooter_slot);
        }
        PlayerIntent::AttemptPassShort { target_x, target_y } => {
            let from_slot = state.players[slot_idx].slot;
            let to_slot = nearest_teammate_near(state, slot_idx, *target_x, *target_y);
            state.match_events.push(MatchEvent::Pass {
                from_slot,
                to_slot,
                tick: state.tick,
                kind: PassKind::Short,
                completed: T1_PASS_COMPLETED,
            });
            // T1-15: snap ball to passer's feet before computing velocity.
            // Without this, the ball starts from its last physical position
            // (often center spot) rather than the passer's feet, so rolling
            // friction stops the ball before it reaches the receiver.
            let from_x = state.players[slot_idx].pos_x;
            let from_y = state.players[slot_idx].pos_y;
            state.ball.pos_x = from_x;
            state.ball.pos_y = from_y;
            // T1-3.5: ball mutation — kick toward receiver's current position.
            let speed = compute_ball_speed_for_pass(&state.players[slot_idx]);
            let to_x = state.players[to_slot as usize].pos_x;
            let to_y = state.players[to_slot as usize].pos_y;
            let (bvx, bvy) = ball_unit_vel(from_x, from_y, to_x, to_y, speed);
            state.ball.vel_x = bvx;
            state.ball.vel_y = bvy;
            state.ball.vel_z = Q32::ZERO;
            // T1: pass always completes; possession goes to receiver.
            state.possession = Some(to_slot);
            state.last_touched_by = Some(from_slot);
        }
        PlayerIntent::AttemptPassLong { target_x, target_y } => {
            let from_slot = state.players[slot_idx].slot;
            let to_slot = nearest_teammate_near(state, slot_idx, *target_x, *target_y);
            state.match_events.push(MatchEvent::Pass {
                from_slot,
                to_slot,
                tick: state.tick,
                kind: PassKind::Long,
                completed: T1_PASS_COMPLETED,
            });
            // T1-15: snap ball to passer's feet (same pattern as Short/Dribble).
            let from_x = state.players[slot_idx].pos_x;
            let from_y = state.players[slot_idx].pos_y;
            state.ball.pos_x = from_x;
            state.ball.pos_y = from_y;
            let speed = compute_ball_speed_for_pass(&state.players[slot_idx]);
            let to_x = state.players[to_slot as usize].pos_x;
            let to_y = state.players[to_slot as usize].pos_y;
            let (bvx, bvy) = ball_unit_vel(from_x, from_y, to_x, to_y, speed);
            state.ball.vel_x = bvx;
            state.ball.vel_y = bvy;
            state.ball.vel_z = Q32::ZERO;
            state.possession = Some(to_slot);
            state.last_touched_by = Some(from_slot);
        }
        PlayerIntent::Cross { target_x, target_y } => {
            let from_slot = state.players[slot_idx].slot;
            let to_slot = nearest_teammate_near(state, slot_idx, *target_x, *target_y);
            state.match_events.push(MatchEvent::Pass {
                from_slot,
                to_slot,
                tick: state.tick,
                kind: PassKind::Cross,
                completed: T1_PASS_COMPLETED,
            });
            // T1-15: snap ball to crosser's feet before kick.
            let from_x = state.players[slot_idx].pos_x;
            let from_y = state.players[slot_idx].pos_y;
            state.ball.pos_x = from_x;
            state.ball.pos_y = from_y;
            let speed = compute_ball_speed_for_pass(&state.players[slot_idx]);
            let to_x = state.players[to_slot as usize].pos_x;
            let to_y = state.players[to_slot as usize].pos_y;
            let (bvx, bvy) = ball_unit_vel(from_x, from_y, to_x, to_y, speed);
            state.ball.vel_x = bvx;
            state.ball.vel_y = bvy;
            state.ball.vel_z = Q32::ZERO;
            state.possession = Some(to_slot);
            state.last_touched_by = Some(from_slot);
        }
        PlayerIntent::LayOff { target_x, target_y } => {
            let from_slot = state.players[slot_idx].slot;
            let to_slot = nearest_teammate_near(state, slot_idx, *target_x, *target_y);
            state.match_events.push(MatchEvent::Pass {
                from_slot,
                to_slot,
                tick: state.tick,
                kind: PassKind::LayOff,
                completed: T1_PASS_COMPLETED,
            });
            // T1-15: snap ball to passer's feet before kick.
            let from_x = state.players[slot_idx].pos_x;
            let from_y = state.players[slot_idx].pos_y;
            state.ball.pos_x = from_x;
            state.ball.pos_y = from_y;
            let speed = compute_ball_speed_for_pass(&state.players[slot_idx]);
            let to_x = state.players[to_slot as usize].pos_x;
            let to_y = state.players[to_slot as usize].pos_y;
            let (bvx, bvy) = ball_unit_vel(from_x, from_y, to_x, to_y, speed);
            state.ball.vel_x = bvx;
            state.ball.vel_y = bvy;
            state.ball.vel_z = Q32::ZERO;
            state.possession = Some(to_slot);
            state.last_touched_by = Some(from_slot);
        }
        PlayerIntent::Dribble { .. } => {
            // T1-3.5: Dribble — ball stays at the dribbler's feet.
            // pos_x/pos_y updated to player position; vel zeroed (ball moves
            // with player via position snap rather than physics integration).
            // The player's vel_x/vel_y is still set by the velocity-update
            // match below so the player navigates toward the dribble target.
            let dribbler_slot = state.players[slot_idx].slot;
            state.ball.pos_x = state.players[slot_idx].pos_x;
            state.ball.pos_y = state.players[slot_idx].pos_y;
            state.ball.vel_x = Q32::ZERO;
            state.ball.vel_y = Q32::ZERO;
            state.ball.vel_z = Q32::ZERO;
            state.possession = Some(dribbler_slot);
            state.last_touched_by = Some(dribbler_slot);
        }
        PlayerIntent::GkDistributeShort { target_x, target_y } => {
            // Mirror pass: GK distributes to a teammate.
            let from_slot = state.players[slot_idx].slot;
            let to_slot = nearest_teammate_near(state, slot_idx, *target_x, *target_y);
            // No MatchEvent for GK distribution in T1 (commentary in T1-4b).
            // T1-15: snap ball to GK's feet before kick.
            let from_x = state.players[slot_idx].pos_x;
            let from_y = state.players[slot_idx].pos_y;
            state.ball.pos_x = from_x;
            state.ball.pos_y = from_y;
            // T1-3.5: ball mutation toward receiver.
            let speed = compute_ball_speed_for_pass(&state.players[slot_idx]);
            let to_x = state.players[to_slot as usize].pos_x;
            let to_y = state.players[to_slot as usize].pos_y;
            let (bvx, bvy) = ball_unit_vel(from_x, from_y, to_x, to_y, speed);
            state.ball.vel_x = bvx;
            state.ball.vel_y = bvy;
            state.ball.vel_z = Q32::ZERO;
            state.possession = Some(to_slot);
            state.last_touched_by = Some(from_slot);
        }
        PlayerIntent::GkDistributeLong { target_x, target_y } => {
            // Long GK distribution — same pattern as short but uses shot-speed
            // scaling (GKs kick hard) rather than pass-speed scaling.
            let from_slot = state.players[slot_idx].slot;
            let to_slot = nearest_teammate_near(state, slot_idx, *target_x, *target_y);
            // T1-15: snap ball to GK's feet before kick.
            let from_x = state.players[slot_idx].pos_x;
            let from_y = state.players[slot_idx].pos_y;
            state.ball.pos_x = from_x;
            state.ball.pos_y = from_y;
            let speed = compute_ball_speed_for_shot(&state.players[slot_idx]);
            let to_x = state.players[to_slot as usize].pos_x;
            let to_y = state.players[to_slot as usize].pos_y;
            let (bvx, bvy) = ball_unit_vel(from_x, from_y, to_x, to_y, speed);
            state.ball.vel_x = bvx;
            state.ball.vel_y = bvy;
            state.ball.vel_z = Q32::ZERO;
            state.possession = Some(to_slot);
            state.last_touched_by = Some(from_slot);
        }
        // Non-emitting / non-ball-touching variants — enumerated explicitly so
        // adding a new PlayerIntent variant forces a compile error here.
        PlayerIntent::Idle
        | PlayerIntent::MoveToPosition { .. }
        | PlayerIntent::HoldBall { .. }
        | PlayerIntent::TrackBack { .. }
        | PlayerIntent::Press { .. }
        | PlayerIntent::MarkPlayer { .. }
        | PlayerIntent::RunOffBall { .. }
        | PlayerIntent::HoldFormation { .. }
        | PlayerIntent::GkShotStop { .. }
        | PlayerIntent::GkCollectCross { .. }
        | PlayerIntent::GkSweeperRush { .. } => {
            // No MatchEvent emitted; no ball mutation.
            // T2+ may add events for press-trigger / interception / save —
            // wire them at this site, not via a wildcard.
        }
    }

    // Velocity update (same for all target-bearing intents; Idle zeroes vel).
    let p = &mut state.players[slot_idx];
    match intent {
        PlayerIntent::Idle => {
            p.vel_x = Q32::ZERO;
            p.vel_y = Q32::ZERO;
        }

        // All variants with a target use the same velocity-toward-target model.
        // The target semantics differ per variant (aim point / run endpoint /
        // recipient position) but the locomotion physics are identical in this
        // tier: clamp each component to ±MAX_PLAYER_SPEED.
        PlayerIntent::MoveToPosition { target_x, target_y }
        | PlayerIntent::AttemptShot { target_x, target_y }
        | PlayerIntent::AttemptPassShort { target_x, target_y }
        | PlayerIntent::AttemptPassLong { target_x, target_y }
        | PlayerIntent::Cross { target_x, target_y }
        | PlayerIntent::Dribble { target_x, target_y }
        | PlayerIntent::HoldBall { target_x, target_y }
        | PlayerIntent::LayOff { target_x, target_y }
        | PlayerIntent::TrackBack { target_x, target_y }
        | PlayerIntent::Press { target_x, target_y }
        | PlayerIntent::MarkPlayer { target_x, target_y }
        | PlayerIntent::RunOffBall { target_x, target_y }
        | PlayerIntent::HoldFormation { target_x, target_y }
        | PlayerIntent::GkShotStop { target_x, target_y }
        | PlayerIntent::GkCollectCross { target_x, target_y }
        | PlayerIntent::GkSweeperRush { target_x, target_y }
        | PlayerIntent::GkDistributeShort { target_x, target_y }
        | PlayerIntent::GkDistributeLong { target_x, target_y } => {
            let dx = target_x - p.pos_x;
            let dy = target_y - p.pos_y;
            p.vel_x = clamp_speed(dx);
            p.vel_y = clamp_speed(dy);
        }
    }
}

/// Find the nearest same-team player to `(target_x, target_y)`, excluding
/// `passer_slot_idx`. Returns the slot of the nearest teammate, or falls
/// back to the passer's own slot if no teammate exists.
///
/// Team is determined by slot index: slots 0..11 = home, 11..22 = away.
/// Comparison uses Manhattan distance (Q32 integer arithmetic; no sqrt).
///
/// **Panic safety (Codex Tier-2 P1-3 on T1-4a 2026-05-16):** distance
/// computation uses `i128` so the subtraction can't overflow even at the
/// extremes of `Q32`'s `i64` raw range, and the `unsigned_abs()` call can't
/// hit the `i64::MIN.abs()` undefined-behavior path. (`i64::MIN.abs()`
/// panics in debug and is UB in release.)
///
/// **Self-pass guard (Codex Tier-2 Critical on T1-4a 2026-05-16; T1-21
/// hardened to release per Sim/RULES.md §11):** the 22-slot match always has
/// 10 teammates available (11 same-team players minus the passer), so the
/// loop runs ≥10 iterations and `best_slot` is always overwritten. An
/// `assert_ne!` against the passer slot pins this invariant — if a future
/// refactor breaks the team_start/team_end derivation, the assertion fires
/// in BOTH debug + release builds, surfacing the bug at the violation site
/// rather than silently landing a self-pass `MatchEvent::Pass` into
/// canonical state. Pre-T1-21 this was `debug_assert_ne!` which the §11
/// hardening identified as exactly the silent-failure pattern banned.
///
/// T1 approximation — T2 refines with passing-lane model.
fn nearest_teammate_near(
    state: &MatchState,
    passer_slot_idx: usize,
    target_x: Q32,
    target_y: Q32,
) -> u8 {
    let passer_team = if passer_slot_idx < 11 { 0usize } else { 1usize };
    let team_start = passer_team * 11;
    let team_end = team_start + 11;
    let passer_slot = state.players[passer_slot_idx].slot;

    let mut best_slot = passer_slot;
    // i128 distance space — Q32 raw bits fit comfortably; no overflow path.
    let mut best_dist: i128 = i128::MAX;

    let target_x_i128 = target_x.to_bits() as i128;
    let target_y_i128 = target_y.to_bits() as i128;

    for teammate_idx in team_start..team_end {
        if teammate_idx == passer_slot_idx {
            continue;
        }
        let tp = &state.players[teammate_idx];
        // Manhattan distance in i128 (positive by construction via unsigned_abs).
        let dx = (tp.pos_x.to_bits() as i128 - target_x_i128).unsigned_abs() as i128;
        let dy = (tp.pos_y.to_bits() as i128 - target_y_i128).unsigned_abs() as i128;
        let dist = dx + dy;
        if dist < best_dist {
            best_dist = dist;
            best_slot = tp.slot;
        }
    }

    // T1-21 per Sim/RULES.md §11: assert_ne! (release-active) replaces the
    // prior debug_assert_ne!. See the doc-comment Self-pass-guard section above
    // for the rationale. `best_slot != passer_slot` is a load-bearing canonical
    // invariant — a self-pass landing in match_events is a real silent-failure
    // class the §11 hardening exists to prevent.
    assert_ne!(
        best_slot, passer_slot,
        "nearest_teammate_near produced a self-pass for slot_idx={passer_slot_idx} \
         (team {passer_team}, range {team_start}..{team_end}); loop did not find \
         any teammate — check team-boundary derivation"
    );

    best_slot
}

/// Clamp a Q32 delta to `[-MAX_PLAYER_SPEED, +MAX_PLAYER_SPEED]`.
fn clamp_speed(delta: Q32) -> Q32 {
    if delta > MAX_PLAYER_SPEED {
        MAX_PLAYER_SPEED
    } else if delta < -MAX_PLAYER_SPEED {
        -MAX_PLAYER_SPEED
    } else {
        delta
    }
}

// ---------------------------------------------------------------------------
// Pre-emption hooks (stub)
// ---------------------------------------------------------------------------

/// Pre-emption hook — wires loose-ball chase for T1-15.
///
/// Returns `Some(intent)` if a pre-emption fires for this player,
/// `None` to proceed to normal role dispatch.
///
/// When `state.possession == None` (loose ball — ball has been shot or knocked
/// free), the nearest-2 outfield players per team chase the ball's current
/// position. This prevents possession from staying `None` indefinitely while
/// preserving formation Y-spread (routing all 10 outfielders collapses width).
///
/// GKs (slots 0 and 11) are normally excluded — GK routing remains in the
/// goalkeeper FSM — EXCEPT when the ball is within 10m of the GK's own goal
/// line. In that case the GK chases the ball to prevent it from lingering
/// uncontested near the goal (the "ball stranded 2-3m short of goal line"
/// scenario from T1-15).
///
/// Full pre-emption hook (foul reaction, set-piece switchover, etc.) defers
/// to T2+ per ADR-0006. Loose-ball chase is the only live hook in T1.
fn preempt_check(state: &MatchState, slot_idx: usize) -> Option<PlayerIntent> {
    // Only fire when the ball is loose (no current carrier).
    if state.possession.is_some() {
        return None;
    }

    // GK slots: only chase when ball is near their own goal line.
    // Home GK (slot 0): own goal at x = -52.5m.
    // Away GK (slot 11): own goal at x = +52.5m.
    // "Near" = within GK_CHASE_RADIUS_M of the goal line (absolute x distance).
    //
    // In Q32, GOAL_LINE_X = 52.5m stored as Q32::from_raw(52_i64 << 32 | ...)
    // We use a simple integer comparison: if abs(ball_x) > GK_CHASE_THRESHOLD_X,
    // the ball is close enough to the goal line for the GK to chase.
    // GK_CHASE_THRESHOLD_X = 42m (ball within 10m of the 52.5m goal line).
    if slot_idx == 0 || slot_idx == 11 {
        // Threshold: ball must be in the attacking third (>42m from centre)
        // to trigger GK chase. This keeps the GK in position during normal play.
        let bx_bits = state.ball.pos_x.to_bits();
        let bx_abs: u64 = bx_bits.unsigned_abs();
        // 42m in Q32: 42 << 32 = 180_388_203_520_u64
        const THRESHOLD_BITS: u64 = 42_u64 << 32;
        if bx_abs < THRESHOLD_BITS {
            return None; // ball is not near a goal line — let GK FSM decide
        }
        // Ball is near a goal line. Check it's near THIS GK's goal.
        // Home GK (slot 0): defends negative x (bx < 0).
        // Away GK (slot 11): defends positive x (bx > 0).
        let home_gk_side = bx_bits < 0; // true if ball is in home half
        let is_home_gk = slot_idx == 0;
        if home_gk_side != is_home_gk {
            return None; // ball is near the OPPONENT's goal — stay back
        }
        // GK chases the ball.
        return Some(PlayerIntent::MoveToPosition {
            target_x: state.ball.pos_x,
            target_y: state.ball.pos_y,
        });
    }

    // Only route the two outfield players NEAREST the ball toward it.
    // Routing all 10 outfielders collapses Y-formation spread (the
    // team_width invariant catches this). In real football, the nearest
    // 1-2 players chase; others hold shape. T1-15 approximation: nearest
    // 2 from each team chase; the rest hold formation (returning via BT).
    //
    // Compute this player's Manhattan distance to the ball.
    let bx = state.ball.pos_x;
    let by = state.ball.pos_y;
    let p = &state.players[slot_idx];
    let my_dx = (p.pos_x - bx).to_bits().unsigned_abs() as i128;
    let my_dy = (p.pos_y - by).to_bits().unsigned_abs() as i128;
    let my_dist = my_dx + my_dy;

    // Count how many same-team outfield players are closer to the ball.
    let team_start = if slot_idx < 11 { 1usize } else { 12usize };
    let team_end = if slot_idx < 11 { 11usize } else { 22usize };
    let gk_slot = if slot_idx < 11 { 0usize } else { 11usize };

    let closer_count = (team_start..team_end)
        .filter(|&i| {
            if i == slot_idx || i == gk_slot {
                return false;
            }
            let op = &state.players[i];
            let dx = (op.pos_x - bx).to_bits().unsigned_abs() as i128;
            let dy = (op.pos_y - by).to_bits().unsigned_abs() as i128;
            dx + dy < my_dist
        })
        .count();

    // If 2 or more same-team outfielders are closer, hold formation.
    // Only the 2 nearest outfielders chase the ball.
    if closer_count >= 2 {
        return None; // let BT decide (formation hold)
    }

    Some(PlayerIntent::MoveToPosition {
        target_x: state.ball.pos_x,
        target_y: state.ball.pos_y,
    })
}

// ---------------------------------------------------------------------------
// Tests — Chunk 6 (RED → GREEN)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_core::{Q32, Seed, Tick};

    use crate::role_states::Role;
    use crate::{MatchState, tick_match};

    // --- apply_intent ---

    #[test]
    fn apply_intent_idle_zeroes_velocity() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        // Give player 0 a nonzero velocity.
        state.players[0].vel_x = Q32::from_int(3);
        state.players[0].vel_y = Q32::from_int(-2);
        apply_intent(&mut state, 0, PlayerIntent::Idle);
        assert_eq!(state.players[0].vel_x, Q32::ZERO);
        assert_eq!(state.players[0].vel_y, Q32::ZERO);
    }

    #[test]
    fn apply_intent_move_to_position_sets_velocity() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        let p0_x = state.players[0].pos_x;
        let p0_y = state.players[0].pos_y;

        // Target 3 m/s above the player (within MAX_PLAYER_SPEED).
        apply_intent(
            &mut state,
            0,
            PlayerIntent::MoveToPosition {
                target_x: p0_x,
                target_y: p0_y + Q32::from_int(3),
            },
        );
        assert_eq!(state.players[0].vel_x, Q32::ZERO);
        assert_eq!(state.players[0].vel_y, Q32::from_int(3));
    }

    #[test]
    fn apply_intent_clamps_to_max_speed() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        let p0_x = state.players[0].pos_x;
        let p0_y = state.players[0].pos_y;

        // Target 100 m away — delta well beyond MAX_PLAYER_SPEED.
        apply_intent(
            &mut state,
            0,
            PlayerIntent::MoveToPosition {
                target_x: p0_x + Q32::from_int(100),
                target_y: p0_y,
            },
        );
        assert_eq!(state.players[0].vel_x, MAX_PLAYER_SPEED);
    }

    // --- dispatch_tick wired into tick_match ---

    #[test]
    fn tick_match_increments_local_decision_counter_for_decided_players() {
        // Run 15 ticks — at least one player decides per tick (balanced slot).
        let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);
        let mut state = MatchState::initial(seed);
        let initial_counters: Vec<u32> =
            state.players.iter().map(|p| p.decision_counter()).collect();
        assert!(
            initial_counters.iter().all(|&c| c == 0),
            "all counters should start at zero"
        );

        for _ in 0..15 {
            state = tick_match(state, &std::collections::BTreeMap::new());
        }

        // After 15 ticks (one full cadence window), every player should have
        // fired at least once (the balanced slot template guarantees no empty slot).
        for (idx, p) in state.players.iter().enumerate() {
            assert!(
                p.decision_counter() > 0,
                "player slot {idx} had zero decisions after 15 ticks"
            );
        }
    }

    #[test]
    fn dispatch_tick_is_deterministic() {
        let seed = Seed::from_u64(0xCAFE_BABE);
        let s1 = MatchState::initial(seed);
        let s2 = MatchState::initial(seed);

        let empty_defs = BTreeMap::new();
        let r1 = dispatch_tick(s1, &empty_defs);
        let r2 = dispatch_tick(s2, &empty_defs);

        assert_eq!(
            r1.encode_canonical(),
            r2.encode_canonical(),
            "dispatch_tick must produce identical canonical output for the same initial state"
        );
    }

    /// P2-2 renamed from `gk_slot0_moves_toward_goal_line_after_decision`:
    /// the original test only asserted the counter; GK starts AT the goal
    /// line so position delta is zero. This test makes an honest assertion.
    #[test]
    fn gk_slot0_decides_within_15_ticks() {
        let seed = Seed::from_u64(1);
        let mut state = MatchState::initial(seed);

        // Run 15 ticks to cover one full cadence window.
        for _ in 0..15 {
            state = tick_match(state, &std::collections::BTreeMap::new());
        }

        // GK's decision_counter should be at least 1 (they decided).
        assert!(
            state.players[0].decision_counter() >= 1,
            "home GK (slot 0) should have made at least one decision in 15 ticks"
        );

        // Role must still be Goalkeeper.
        assert_eq!(
            state.players[0].role(),
            Role::Goalkeeper,
            "slot 0 should still be a Goalkeeper after 15 ticks"
        );
    }

    /// P2-2 additional coverage: verify GK actually moves when starting away
    /// from the goal line. Constructed directly with a non-goal-line position.
    #[test]
    fn gk_slot0_moves_toward_goal_line_when_displaced() {
        let seed = Seed::from_u64(1);
        let mut state = MatchState::initial(seed);
        // Move the home GK to centre spot (0, 0) — far from their goal line at (-45, 0).
        state.players[0].pos_x = Q32::ZERO;
        state.players[0].pos_y = Q32::ZERO;
        let initial_pos_x = state.players[0].pos_x;

        // Run until GK decides at least once (find the first decision tick).
        for _ in 0..15 {
            state = tick_match(state, &std::collections::BTreeMap::new());
        }

        // After decisions, GK should have moved toward x=-45 (velocity set to negative x).
        // Position should now be less than the initial (0) since the GK moves toward -45.
        assert!(
            state.players[0].pos_x < initial_pos_x,
            "home GK should have moved toward x=-45; pos_x={:?} initial={:?}",
            state.players[0].pos_x,
            initial_pos_x,
        );
    }

    #[test]
    fn all_initial_roles_are_correctly_assigned() {
        let state = MatchState::initial(Seed::from_u64(1));

        // Home team: slots 0..11
        assert_eq!(state.players[0].role(), Role::Goalkeeper); // slot 0
        for i in 1..=4 {
            assert_eq!(
                state.players[i].role(),
                Role::Defender,
                "home slot {i} should be Defender"
            );
        }
        for i in 5..=7 {
            assert_eq!(
                state.players[i].role(),
                Role::Midfielder,
                "home slot {i} should be Midfielder"
            );
        }
        for i in 8..=10 {
            assert_eq!(
                state.players[i].role(),
                Role::Forward,
                "home slot {i} should be Forward"
            );
        }

        // Away team: slots 11..22
        assert_eq!(state.players[11].role(), Role::Goalkeeper); // slot 11
        for i in 12..=15 {
            assert_eq!(
                state.players[i].role(),
                Role::Defender,
                "away slot {i} should be Defender"
            );
        }
        for i in 16..=18 {
            assert_eq!(
                state.players[i].role(),
                Role::Midfielder,
                "away slot {i} should be Midfielder"
            );
        }
        for i in 19..=21 {
            assert_eq!(
                state.players[i].role(),
                Role::Forward,
                "away slot {i} should be Forward"
            );
        }
    }

    #[test]
    fn all_initial_decision_counters_are_zero() {
        let state = MatchState::initial(Seed::from_u64(42));
        for (i, p) in state.players.iter().enumerate() {
            assert_eq!(
                p.decision_counter(),
                0,
                "player slot {i} should have decision_counter == 0 at match-init"
            );
        }
    }

    #[test]
    fn decision_counter_increases_monotonically_per_player() {
        // Counters should never decrease during a match.
        let seed = Seed::from_u64(0xABCDABCD);
        let mut state = MatchState::initial(seed);
        let mut prev_counters = [0u32; 22];

        for _ in 0..60 {
            state = tick_match(state, &std::collections::BTreeMap::new());
            for (i, p) in state.players.iter().enumerate() {
                assert!(
                    p.decision_counter() >= prev_counters[i],
                    "player slot {i} counter went from {} to {} — counters must not decrease",
                    prev_counters[i],
                    p.decision_counter()
                );
                prev_counters[i] = p.decision_counter();
            }
        }
    }

    /// Verify the tick where N decisions are scheduled: check that exactly
    /// the right number of players have their counter incremented.
    #[test]
    fn decisions_fire_only_at_scheduled_ticks() {
        let seed = Seed::from_u64(7);
        let mut state = MatchState::initial(seed);
        let slots = state.decision_slots;

        // Advance to tick 1.
        state = tick_match(state, &std::collections::BTreeMap::new());
        let tick_raw = state.tick.to_raw();

        // Count how many roster slots fire at this tick.
        let expected_deciders = (0..22usize)
            .filter(|&i| {
                tick_raw.rem_euclid(15) as u8 == slots[i]
                    && state.interrupt_cooldown_until[i] <= Tick::from_raw(tick_raw)
            })
            .count();

        // Count how many players have counter == 1 (fired once).
        let actual_deciders = state
            .players
            .iter()
            .filter(|p| p.decision_counter() == 1)
            .count();

        assert_eq!(
            actual_deciders, expected_deciders,
            "at tick {tick_raw}: expected {expected_deciders} decisions but got {actual_deciders}"
        );
    }

    // --- T1-3.5 Chunk 3: ball-speed helper tests ---

    /// Zero attributes: speed equals base (no bonus).
    #[test]
    fn shot_speed_at_zero_attrs_equals_base() {
        use fw_core::PlayerAttributes;
        let mut state = MatchState::initial(Seed::from_u64(1));
        // Zero all attrs on player 9 (home FWD, slot 9).
        state.players[9].attributes = PlayerAttributes::default_zero();
        let speed = compute_ball_speed_for_shot(&state.players[9]);
        assert_eq!(
            speed, SHOT_BASE_SPEED_MPS,
            "at zero attrs, shot speed must equal SHOT_BASE_SPEED_MPS (20 m/s); got {:?}",
            speed
        );
    }

    /// Peak attributes: speed equals base + bonus.
    #[test]
    fn shot_speed_at_peak_attrs_equals_base_plus_bonus() {
        use fw_core::PlayerAttributes;
        let mut state = MatchState::initial(Seed::from_u64(1));
        // Max all attrs on player 9.
        state.players[9].attributes = PlayerAttributes::max_baseline();
        let speed = compute_ball_speed_for_shot(&state.players[9]);
        let expected = SHOT_BASE_SPEED_MPS + SHOT_PEAK_BONUS_MPS; // 35 m/s
        assert_eq!(
            speed, expected,
            "at max attrs (strength=1.0 × finishing=1.0 = 1.0), shot speed must equal \
             SHOT_BASE + SHOT_PEAK = 35 m/s; got {:?}",
            speed
        );
    }

    /// Mid-range attributes: speed between base and base+bonus.
    #[test]
    fn shot_speed_at_mid_attrs_is_between_base_and_max() {
        use fw_core::PlayerAttributes;
        let mut state = MatchState::initial(Seed::from_u64(1));
        state.players[9].attributes = PlayerAttributes::mid_range_baseline();
        let speed = compute_ball_speed_for_shot(&state.players[9]);
        // strength ≈ 0.5, finishing ≈ 0.5 → product ≈ 0.25 → bonus ≈ 3.75 m/s → total ≈ 23.75 m/s
        assert!(
            speed > SHOT_BASE_SPEED_MPS,
            "mid-range attrs must produce speed above base; got {:?}",
            speed
        );
        assert!(
            speed < SHOT_BASE_SPEED_MPS + SHOT_PEAK_BONUS_MPS,
            "mid-range attrs must produce speed below max; got {:?}",
            speed
        );
    }

    /// Pass: zero attrs → base speed.
    #[test]
    fn pass_speed_at_zero_attrs_equals_base() {
        use fw_core::PlayerAttributes;
        let mut state = MatchState::initial(Seed::from_u64(1));
        state.players[5].attributes = PlayerAttributes::default_zero();
        let speed = compute_ball_speed_for_pass(&state.players[5]);
        assert_eq!(
            speed, PASS_BASE_SPEED_MPS,
            "at zero attrs, pass speed must equal PASS_BASE_SPEED_MPS (15 m/s)"
        );
    }

    /// Pass: peak attrs → base + bonus.
    #[test]
    fn pass_speed_at_peak_attrs_equals_base_plus_bonus() {
        use fw_core::PlayerAttributes;
        let mut state = MatchState::initial(Seed::from_u64(1));
        state.players[5].attributes = PlayerAttributes::max_baseline();
        let speed = compute_ball_speed_for_pass(&state.players[5]);
        let expected = PASS_BASE_SPEED_MPS + PASS_PEAK_BONUS_MPS; // 25 m/s
        assert_eq!(
            speed, expected,
            "at max attrs (passing=1.0 × vision=1.0), pass speed must equal 25 m/s"
        );
    }

    // -----------------------------------------------------------------------
    // T1-19: preempt_check behavioral unit tests
    //
    // Background: T1-15 grew preempt_check from "stubbed None" to a 3-policy
    // implementation:
    //   1. Possession-gate: return None if state.possession.is_some().
    //   2. GK chase: slot 0 / 11 chase only when the ball is within 10m of
    //      their OWN goal line (|ball.pos_x| > 42m AND ball on own side).
    //   3. Outfield nearest-2: only fire for the 2 nearest same-team outfielders
    //      (strict-< tiebreak on Manhattan distance).
    //
    // These 5 tests pin each policy + the GK-vs-FSM coexistence invariant
    // documented in ADR-0006's 2026-05-16 amendment. See post-T1 ultimate-review
    // Track A (docs/audits/post-t1-ultimate-review-2026-05-16.md) for the
    // RED coverage-hole analysis that motivated this row.
    // -----------------------------------------------------------------------

    /// Policy 2 negative case: home GK does NOT chase a ball near the AWAY goal.
    /// Mutation discriminator: flipping the `home_gk_side != is_home_gk` predicate
    /// to `==` would make this test fail (preempt would fire, returning Some).
    #[test]
    fn preempt_check_home_gk_does_not_chase_away_ball() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        // Loose ball at +45m (away half, within 10m of the away goal line at +52.5m).
        state.possession = None;
        state.ball.pos_x = Q32::from_int(45);
        state.ball.pos_y = Q32::ZERO;
        state.ball.vel_x = Q32::ZERO;
        state.ball.vel_y = Q32::ZERO;

        let intent = preempt_check(&state, 0); // slot 0 = home GK
        assert!(
            intent.is_none(),
            "home GK (slot 0) must NOT chase a loose ball near the AWAY goal \
             (ball at x=+45m); got {intent:?}"
        );
    }

    /// Policy 2 positive case: home GK chases a loose ball within 10m of own goal line.
    /// Mutation discriminator: raising THRESHOLD_BITS from 42 to e.g. 100 would
    /// cause ball at |x|=43 to early-return None.
    #[test]
    fn preempt_check_home_gk_chases_loose_ball_within_42m_of_own_goal() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        // Loose ball at -43m (home side, within 10m of home goal line at -52.5m).
        state.possession = None;
        state.ball.pos_x = Q32::from_int(-43);
        state.ball.pos_y = Q32::from_int(2);
        state.ball.vel_x = Q32::ZERO;
        state.ball.vel_y = Q32::ZERO;

        let intent = preempt_check(&state, 0);
        match intent {
            Some(PlayerIntent::MoveToPosition { target_x, target_y }) => {
                assert_eq!(
                    target_x, state.ball.pos_x,
                    "preempt MoveToPosition target_x must equal ball.pos_x"
                );
                assert_eq!(
                    target_y, state.ball.pos_y,
                    "preempt MoveToPosition target_y must equal ball.pos_y"
                );
            }
            other => panic!(
                "home GK (slot 0) must chase ball within 10m of own goal line; \
                 expected MoveToPosition, got {other:?}"
            ),
        }
    }

    /// Policy 3: exactly the 2 nearest same-team outfielders preempt-chase a
    /// loose ball. The remaining 3 hold formation (return None).
    /// Mutation discriminator: changing `closer_count >= 2` to `>= 5` would
    /// make all 5 outfielders chase.
    #[test]
    fn preempt_check_outfield_chaser_count_caps_at_2() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        state.possession = None;
        // Place loose ball at the centre spot.
        state.ball.pos_x = Q32::ZERO;
        state.ball.pos_y = Q32::ZERO;
        state.ball.vel_x = Q32::ZERO;
        state.ball.vel_y = Q32::ZERO;

        // Choose 5 home outfielders (slots 1..=5) and place them at strictly
        // distinct Manhattan distances from the ball: 1m, 2m, 3m, 4m, 5m.
        // Strict-distinct distances mean the strict-< tiebreak yields a stable
        // ranking with no ties; the cap policy then routes the 2 nearest.
        for (i, slot) in [1usize, 2, 3, 4, 5].iter().enumerate() {
            state.players[*slot].pos_x = Q32::from_int((i as i32) + 1); // 1, 2, 3, 4, 5 m
            state.players[*slot].pos_y = Q32::ZERO;
        }
        // Park other home outfielders far away so they're not closer than these 5.
        for slot in [6usize, 7, 8, 9, 10] {
            state.players[slot].pos_x = Q32::from_int(40);
            state.players[slot].pos_y = Q32::from_int(20);
        }

        let chasers: Vec<usize> = [1usize, 2, 3, 4, 5]
            .iter()
            .copied()
            .filter(|&slot| preempt_check(&state, slot).is_some())
            .collect();

        assert_eq!(
            chasers.len(),
            2,
            "exactly 2 of the 5 nearest same-team outfielders must preempt-chase; \
             got {} chasers: {:?}",
            chasers.len(),
            chasers
        );
        // The 2 nearest by construction are slots 1 (1m) and 2 (2m).
        assert_eq!(
            chasers,
            vec![1, 2],
            "the nearest 2 outfielders should chase; got {chasers:?}"
        );

        // Determinism sub-assertion: same state → same result on a re-call.
        let second_pass: Vec<usize> = [1usize, 2, 3, 4, 5]
            .iter()
            .copied()
            .filter(|&slot| preempt_check(&state, slot).is_some())
            .collect();
        assert_eq!(
            chasers, second_pass,
            "preempt_check is a pure function over canonical state — \
             re-calling on unchanged state must return identical chaser set"
        );
    }

    /// Policy 1: preempt_check returns None whenever the ball is owned.
    /// Mutation discriminator: deleting the `state.possession.is_some()`
    /// early-return would make preempt fire under possession, returning Some.
    #[test]
    fn preempt_check_only_fires_on_loose_ball() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        // Possession held by home FWD (kickoff convention from MatchState::initial).
        state.possession = Some(9);
        // Place ball deep in own half (would otherwise trigger GK chase).
        state.ball.pos_x = Q32::from_int(-44);
        state.ball.pos_y = Q32::ZERO;
        state.ball.vel_x = Q32::ZERO;
        state.ball.vel_y = Q32::ZERO;

        // GK slot 0: even with ball within 10m of own goal, possession blocks preempt.
        assert!(
            preempt_check(&state, 0).is_none(),
            "preempt_check must return None for GK while possession is held"
        );
        // Outfield slot 1: same — possession blocks before nearest-2 logic runs.
        // Place slot 1 right on the ball so it would otherwise be the closest chaser.
        state.players[1].pos_x = state.ball.pos_x;
        state.players[1].pos_y = state.ball.pos_y;
        assert!(
            preempt_check(&state, 1).is_none(),
            "preempt_check must return None for outfielders while possession is held"
        );
    }

    /// Coexistence invariant: when preempt fires for the GK, dispatch_tick's
    /// `continue;` after `apply_intent` skips the GK FSM (tick_goalkeeper).
    /// Observable: the GK's role_state does NOT transition this tick, even
    /// when ball position + velocity would normally drive an InBoxPositioning
    /// → ShotStopping transition inside the GK FSM.
    /// Mutation discriminator: removing the `continue;` after the preempt
    /// branch would let tick_goalkeeper run and transition the FSM.
    #[test]
    fn preempt_check_does_not_conflict_with_goalkeeper_fsm() {
        use crate::role_states::{GoalkeeperState, PlayerRoleState};

        let seed = Seed::from_u64(0x1234_5678);
        let mut state = MatchState::initial(seed);

        // Force slot 0 to fire its decision at tick 0 (decision_slots[0] = 0
        // means tick.rem_euclid(15) == 0 → fires). MatchState::initial assigns
        // decision_slots from the seed; we override deterministically here.
        state.decision_slots[0] = 0;
        state.interrupt_cooldown_until[0] = Tick::ZERO;
        // (state.tick is already Tick::ZERO at MatchState::initial.)

        // Loose ball deep in the home penalty area, moving TOWARD the home goal:
        //   pos_x = -44m → in own half (bx < 0), in penalty area (bx < -36.5m).
        //   vel_x = -1   → approaching_goal predicate fires inside GK FSM.
        // Without preempt's `continue;`, tick_goalkeeper's evaluate_transitions
        // would route InBoxPositioning → ShotStopping.
        state.possession = None;
        state.ball.pos_x = Q32::from_int(-44);
        state.ball.pos_y = Q32::ZERO;
        state.ball.vel_x = Q32::from_int(-1);
        state.ball.vel_y = Q32::ZERO;

        // Pre-condition: GK starts InBoxPositioning (the MatchState::initial default).
        assert_eq!(
            state.players[0].role_state,
            PlayerRoleState::Goalkeeper(GoalkeeperState::InBoxPositioning),
            "test pre-condition: home GK must start in InBoxPositioning"
        );

        // Sanity: preempt would fire if dispatch consulted it standalone.
        let preempt_intent = preempt_check(&state, 0);
        assert!(
            matches!(preempt_intent, Some(PlayerIntent::MoveToPosition { .. })),
            "test pre-condition: preempt_check must return MoveToPosition for this state; \
             got {preempt_intent:?}"
        );

        // Execute one dispatch_tick. The preempt branch fires + `continue;` skips
        // tick_goalkeeper, so the GK FSM never runs this tick.
        let after = dispatch_tick(state, &BTreeMap::new());

        assert_eq!(
            after.players[0].role_state,
            PlayerRoleState::Goalkeeper(GoalkeeperState::InBoxPositioning),
            "preempt branch must skip GK FSM via `continue;`: GK role_state must \
             remain InBoxPositioning. If this fails, tick_goalkeeper ran and \
             transitioned to ShotStopping (or another state) — meaning preempt + \
             GK FSM both fired this tick, violating ADR-0006's 'preempt OR role \
             dispatch, never both' contract"
        );
    }
}

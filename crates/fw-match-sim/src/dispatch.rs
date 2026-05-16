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
/// 5 m/s ≈ slow jog. Enough for skeleton movement toward formation.
/// In Q32.32 format: 5 × 2^32 = 5 << 32 = 21_474_836_480 as i64.
const MAX_PLAYER_SPEED: Q32 = Q32::from_raw(5_i64 << 32); // 5.0 in Q32.32

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
        }
        // Non-emitting variants — enumerated explicitly so adding a new
        // PlayerIntent variant forces a compile error here (see preamble).
        PlayerIntent::Idle
        | PlayerIntent::MoveToPosition { .. }
        | PlayerIntent::Dribble { .. }
        | PlayerIntent::HoldBall { .. }
        | PlayerIntent::TrackBack { .. }
        | PlayerIntent::Press { .. }
        | PlayerIntent::MarkPlayer { .. }
        | PlayerIntent::RunOffBall { .. }
        | PlayerIntent::HoldFormation { .. }
        | PlayerIntent::GkShotStop { .. }
        | PlayerIntent::GkCollectCross { .. }
        | PlayerIntent::GkSweeperRush { .. }
        | PlayerIntent::GkDistributeShort { .. }
        | PlayerIntent::GkDistributeLong { .. } => {
            // No MatchEvent emitted for movement / defensive / GK intents.
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
/// **Self-pass guard (Codex Tier-2 Critical on T1-4a 2026-05-16):** the
/// 22-slot match always has 10 teammates available (11 same-team players
/// minus the passer), so the loop runs ≥10 iterations and `best_slot` is
/// always overwritten. A `debug_assert_ne!` against the passer slot pins
/// this invariant — if a future refactor breaks the team_start/team_end
/// derivation, the assertion fires in debug builds before the bad event
/// silently lands in `match_events`.
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

    debug_assert_ne!(
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

/// Universal pre-emption hook check. Returns `Some(intent)` if a pre-emption
/// fires for this player, `None` to proceed to normal role dispatch.
///
/// T1-2b-iii-a: always returns `None`. -iii-b / T1-4 wires:
/// - Single-chaser claim (only one player chases the loose ball)
/// - Foul reaction
/// - Set-piece switchover
fn preempt_check(_state: &MatchState, _slot_idx: usize) -> Option<PlayerIntent> {
    None
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
            state = tick_match(state);
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
            state = tick_match(state);
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
            state = tick_match(state);
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
            state = tick_match(state);
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
        state = tick_match(state);
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
}

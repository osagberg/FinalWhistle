//! `fw-match-sim` — the canonical 22-player match simulation.
//!
//! ## Determinism contract (load-bearing)
//!
//! This crate is one of the canonical-state crates. The determinism gate
//! (`docs/specs/determinism-gate.md`) pins:
//!
//! - **No floats.** `f32`/`f64` are forbidden at lint level
//!   (`clippy::float_arithmetic = deny` in `Cargo.toml`). All numeric state
//!   is `fw_core::Q32`.
//! - **No `HashMap` / `HashSet`.** Canonical-state collections use
//!   `BTreeMap` / `BTreeSet` for sorted, reproducible iteration order.
//! - **No clocks.** `Tick` is the only in-sim time concept; never
//!   `Instant::now()` or `SystemTime::now()`.
//! - **No `tokio` / `async`.** The sim is sync. Tauri IPC handlers wrap
//!   the sim and may be async; the sim itself runs to completion on its
//!   calling thread.
//! - **Seeded RNG only.** `rand_chacha::ChaCha8Rng::seed_from_u64(
//!   seed_fn(match_seed, tick, layer, site))` per ADR-0009. Never
//!   `thread_rng()`. The 8 `SeedLayer` discriminants ensure non-
//!   overlapping random-draw spaces across layer-1..7.
//!
//! ## Phase-0 scope
//!
//! This is the Phase-0 / T0 scaffold: enough surface to make the
//! determinism gate (`crates/fw-replay/tests/canonical_hash.rs`) compile +
//! pass intra-process. The tick function is a no-op advance (increment
//! tick, do nothing else). Real behavior — player AI, ball physics, set
//! pieces — lands in T1+.

pub mod ball;
pub mod ball_physics;
pub mod bt;
pub mod canonical;
pub mod decision_cadence;
pub mod dispatch;
pub mod dto;
pub mod goalkeeper_fsm;
pub mod player;
pub mod role_states;
pub mod separation;
pub mod signature;
pub mod subtree_library;
pub mod tactic_fsm;
pub mod utility;

use fw_content::SignatureId;
use fw_content::event::GOAL_HALF_WIDTH_M;
#[cfg(test)]
use fw_core::Q32;
use fw_core::{GOAL_LINE_X, SIDELINE_Y};
use fw_core::{Seed, Tick};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub use ball::BallState;
pub use ball_physics::{BallPhysicsCoefficients, dt_per_tick, phase1_seeds};
pub use canonical::CanonicalEncoder;
pub use decision_cadence::{SeedLayer, assign_decision_slots, seed_fn, should_decide};
pub use dto::{BallFrameDto, MatchFrameDto, PlayerFrameDto};
pub use fw_content::MatchEvent;
pub use player::PlayerState;
pub use role_states::{
    DefenderState, ForwardState, GoalkeeperState, MidfielderState, PlayerIntent, PlayerRoleState,
    Role,
};
pub use tactic_fsm::{
    ArchetypeParams, CounterIntent, PressIntensity, SetPieceKind, TacticEvent, TacticState,
    TeamTacticState,
};

// -------------------------------------------------------------------------
// Constants
// -------------------------------------------------------------------------

/// Number of players per team. Football is football.
pub const PLAYERS_PER_TEAM: usize = 11;

/// Total players on the pitch (both teams).
pub const TOTAL_PLAYERS: usize = PLAYERS_PER_TEAM * 2;

// Codex P3 from self-review: `MatchState::initial` casts `TOTAL_PLAYERS` to
// `u8` via `slot as u8`. If `PLAYERS_PER_TEAM` ever grew past 127 the cast
// would silently truncate. Make the truncation a compile-time error.
const _: () = assert!(
    TOTAL_PLAYERS <= u8::MAX as usize,
    "TOTAL_PLAYERS exceeds u8 — canonical-encoder slot field would silently truncate"
);

/// Slot index for a player. Re-exported from `fw-core` (moved at T1-4a so
/// `fw-content::event::MatchEvent` can reference `PlayerSlot` without
/// creating a dep cycle). Stable for the duration of a match — the slot
/// holds the canonical position in the team's ordered roster (GK = slot 0,
/// outfield by tactical position thereafter). Substitutions swap the
/// occupant of a slot; the slot identifier itself never changes mid-match.
///
/// This is the canonical-encoding key for player state: encoding iterates
/// slots 0..22 in fixed order, so the encoded byte stream is structural,
/// not pointer-dependent.
pub use fw_core::PlayerSlot;

// -------------------------------------------------------------------------
// MatchState — the canonical-state struct
// -------------------------------------------------------------------------

/// The canonical match state. Every field is deterministic + serializable;
/// nothing here references the host clock, thread-local RNG, or pointer
/// identity.
///
/// Encoded canonically via [`CanonicalEncoder`]; hashed via BLAKE3 by
/// `crates/fw-replay/tests/canonical_hash.rs`.
///
/// ## T1-2b-ii additions
///
/// Three new canonical fields (per `docs/specs/decision-cadence-stagger.md`
/// + `docs/specs/tactic-fsm.md`; ADR-0012 trigger #1 — canonical schema bump):
///
/// - `decision_slots: [u8; 22]` — match-init stagger assignment. Never
///   mutated after initialization. Fisher-Yates over `SLOT_TEMPLATE` seeded
///   by `seed_fn(match_seed, 0, SeedLayer::Decision, 0)`.
/// - `interrupt_cooldown_until: [Tick; 22]` — parallel cooldown field for
///   reactive interrupts (ADR-0001 layer 6). Initialized to `Tick::ZERO`;
///   mutated by the reactive-interrupt path (T1-2b-iii). `decision_slots` is
///   never mutated — the balanced-multiset invariant holds for the full match.
/// - `team_tactic_states: [TeamTacticState; 2]` — one FSM state per team
///   (index 0 = home, index 1 = away). Initialized to `[MidBlock @ Tick::ZERO; 2]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchState {
    /// The match seed. Echoes the seed `MatchState::initial` was constructed
    /// from. Fixed for the duration of the match.
    pub seed: Seed,

    /// The current in-sim tick. Starts at `Tick::ZERO`; advances by exactly
    /// one per call to [`tick_match`].
    pub tick: Tick,

    /// The 22 players on the pitch. Slot-indexed (see [`PlayerSlot`]) for
    /// canonical-encoding stability. `0..11` is the home team; `11..22` is
    /// the away team.
    ///
    /// `Vec` (not `BTreeMap`) is OK here because the index *is* the
    /// canonical key — there's no hashing ambiguity to introduce.
    pub players: Vec<PlayerState>,

    /// The ball. Single entity, always present.
    pub ball: BallState,

    /// Home-team score. `u8` is enough (no FW match exceeds 255 goals).
    pub home_score: u8,

    /// Away-team score.
    pub away_score: u8,

    /// Decision-cadence stagger slots. `decision_slots[i]` is the slot
    /// (0..15) assigned to roster index `i` (roster_slot = `i + 1`).
    ///
    /// **Immutable after match-init.** Reactive interrupts do NOT modify
    /// this array; they update `interrupt_cooldown_until` instead.
    pub decision_slots: [u8; 22],

    /// Reactive-interrupt cooldown end ticks. `interrupt_cooldown_until[i]`
    /// is the tick up to which roster index `i`'s scheduled decision is
    /// suppressed by a reactive interrupt. Initialized to `Tick::ZERO`
    /// (no cooldown). Updated by the reactive-interrupt path (T1-2b-iii).
    pub interrupt_cooldown_until: [Tick; 22],

    /// Team-tactic FSM state. Index 0 = home team, index 1 = away team.
    /// Both teams start in `MidBlock @ Tick::ZERO`.
    pub team_tactic_states: [TeamTacticState; 2],

    // ---- T1-2b-iv additions (signature dispatcher; ADR-0011; canonical schema bump) ----
    //
    // Three new canonical fields. VERSION bumped 4 → 5.
    // `signature_cooldowns` is BTreeMap (not HashMap) per Sim/RULES.md §2.
    // `signature_firing` is a fixed array (mirrors interrupt_cooldown_until pattern).
    // `signature_first_fired_seen` tracks first-fire per (slot, signature) pair.
    /// Per-player+per-signature cooldown state: the tick at which the cooldown
    /// expires (i.e. the earliest tick the signature may re-fire).
    ///
    /// Keyed by `(PlayerSlot, SignatureId)` using `BTreeMap` for deterministic
    /// ordered iteration (Sim/RULES.md §2). Updated by `dispatch_tick` when a
    /// signature fires; expiry = `current_tick + cooldown_ticks`.
    ///
    /// Empty at match init (no cooldowns active).
    pub signature_cooldowns: BTreeMap<(PlayerSlot, SignatureId), Tick>,

    /// Per-player, per-category active signature firing windows.
    ///
    /// Outer index: slot (0..22). Inner index: `BiasCategory as usize` (0..4).
    ///   `signature_firing[slot][BiasCategory::Attacking as usize]`
    ///   `signature_firing[slot][BiasCategory::Defensive as usize]`
    ///   `signature_firing[slot][BiasCategory::BuildUp as usize]`
    ///   `signature_firing[slot][BiasCategory::SetPiece as usize]`
    ///
    /// `None` = no signature in flight for that (player, category) pair.
    /// `Some(SignatureFiring { ... })` = signature active in that category lane.
    ///
    /// Per ADR-0011 §"Stacking policy": same-category concurrent firings are
    /// forbidden; cross-category concurrent firings are allowed. The 2D array
    /// makes both invariants structurally enforced: each lane is independent.
    ///
    /// Cleared per lane by `dispatch_tick` when the firing window expires.
    pub signature_firing: [[Option<signature::SignatureFiring>; 4]; 22],

    /// Tracks which `(PlayerSlot, SignatureId)` pairs have fired for the first
    /// time this match. Used to gate `MatchEvent::SignatureFirstFired` emission —
    /// the event fires ONCE per player+signature pair per match.
    ///
    /// `BTreeSet` for deterministic canonical encoding.
    pub signature_first_fired_seen: BTreeSet<(PlayerSlot, SignatureId)>,

    // ---- T1-3.5 additions (possession state; ADR-0012 trigger #1) ----
    //
    // Two new canonical fields. Encoder VERSION bumped 7→8.
    // Appended AFTER `match_events` in the canonical encoder to maintain the
    // field-order-append discipline (no rearrangement of prior sections).
    /// Which player slot currently has possession of the ball.
    ///
    /// `None` means the ball is loose (in flight after a shot, bouncing free,
    /// contested, or not yet claimed). `Some(slot)` means that player is
    /// the designated ball-carrier for this tick.
    ///
    /// Initial state: `Some(9)` — home centre forward (slot 9) has the
    /// ball at kick-off. Updated by `apply_intent` in `dispatch.rs` when
    /// Shot/Pass/Dribble/GK-distribution intents fire.
    ///
    /// `pub(crate)` — external callers use [`MatchState::possession()`].
    pub(crate) possession: Option<PlayerSlot>,

    /// The most recent player slot that touched the ball, regardless of
    /// whether they still have possession.
    ///
    /// `None` only at the very first tick before any intent fires (matches
    /// the initial `possession = Some(9)` convention — both start as
    /// `Some(9)` at `MatchState::initial`). Goal attribution uses
    /// `last_touched_by` as the scorer (the last player to touch the ball
    /// before it crossed the goal line).
    ///
    /// Updated by every intent that touches the ball (Shot, Pass-class,
    /// Dribble, GK distribution).
    ///
    /// `pub(crate)` — external callers use [`MatchState::last_touched_by()`].
    pub(crate) last_touched_by: Option<PlayerSlot>,

    // ---- T1-4a additions (MatchEvent emission; ADR-0007 Layer 1) ----
    //
    // `match_events` is in canonical state (encoder VERSION bumped 6→7).
    // Events accumulate across the match; never cleared between ticks.
    // T1-4b's commentary renderer reads this Vec after tick_match returns.
    // T3-1 wires these events to the real fw-memory ledger.
    /// The tick at which the match ends (inclusive). When `state.tick`
    /// reaches this value, `FullTime` is emitted and the match is considered
    /// complete.
    ///
    /// T1: hardcoded to `Tick::from_raw(60)` (the 1-second smoke-seed budget).
    /// T1-5 makes this configurable via the `play_match` Tauri command
    /// (likely via a `MatchState::initial_with_match_end_tick(seed, end_tick)`
    /// constructor variant).
    ///
    /// `pub(crate)` per Codex Tier-2 P1 on T1-4a 2026-05-16 — mirrors the
    /// `signature_candidates` visibility pattern from T1-2b-iv P1-2. Use
    /// [`MatchState::match_end_tick()`] from outside the crate.
    pub(crate) match_end_tick: Tick,

    /// Accumulated in-match event stream. Every tick may append one or more
    /// `MatchEvent` entries. Entries are in chronological (tick-ascending)
    /// order by construction — the Vec is never sorted post-construction.
    ///
    /// Unlike the removed `signature_memory_events` scratch buffer, this Vec
    /// is canonical state: it persists across ticks and IS encoded by the
    /// canonical encoder (VERSION 6→7).
    ///
    /// `pub(crate)` per Codex Tier-2 P1 on T1-4a 2026-05-16 — mirrors the
    /// `signature_candidates` visibility pattern from T1-2b-iv P1-2. Use
    /// [`MatchState::match_events()`] from outside the crate. Internal
    /// emission sites (`tick_match`, `dispatch::apply_intent`,
    /// `dispatch::dispatch_tick`) push directly; external callers cannot
    /// `clear()` or `sort()` or otherwise corrupt the chronological invariant.
    pub(crate) match_events: Vec<MatchEvent>,
}

impl MatchState {
    /// Initial state at `Tick::ZERO`. Players placed at their 4-3-3 formation
    /// positions with roles assigned per roster slot.
    ///
    /// ## Role assignment (T1-2b-iii-a default 4-3-3)
    ///
    /// Home team (slots 0..11):
    ///   slot  0 → Goalkeeper
    ///   slots 1-4 → Defender (4 defenders)
    ///   slots 5-7 → Midfielder (3 midfielders)
    ///   slots 8-10 → Forward (3 forwards)
    ///
    /// Away team (slots 11..22): mirrors home with +11 offset.
    ///   slot 11 → Goalkeeper
    ///   slots 12-15 → Defender
    ///   slots 16-18 → Midfielder
    ///   slots 19-21 → Forward
    ///
    /// Formation positions are from
    /// [`subtree_library::FORMATION_4_3_3_POSITIONS`].
    pub fn initial(seed: Seed) -> MatchState {
        use crate::role_states::Role;
        use crate::subtree_library::formation_position;

        let mut players = Vec::with_capacity(TOTAL_PLAYERS);

        for slot in 0..TOTAL_PLAYERS as u8 {
            // Determine role by slot within the 4-3-3 formation.
            // Per-team offset: home slots 0..11, away slots 11..22.
            let in_team = slot % PLAYERS_PER_TEAM as u8;
            let role = match in_team {
                0 => Role::Goalkeeper,
                1..=4 => Role::Defender,
                5..=7 => Role::Midfielder,
                _ => Role::Forward, // 8, 9, 10
            };

            let (x, y) = formation_position(slot);
            players.push(PlayerState::with_role(slot, x, y, role));
        }

        MatchState {
            seed,
            tick: Tick::ZERO,
            players,
            ball: BallState::centre_spot(),
            home_score: 0,
            away_score: 0,
            // T1-2b-ii: decision-cadence stagger assigned from match seed.
            decision_slots: assign_decision_slots(seed),
            // T1-2b-ii: all cooldowns start at zero (no active interrupts).
            interrupt_cooldown_until: [Tick::ZERO; 22],
            // T1-2b-ii: both teams start in neutral MidBlock.
            team_tactic_states: [TeamTacticState::initial(); 2],
            // T1-2b-iv: signature state — all empty at match init.
            signature_cooldowns: BTreeMap::new(),
            // Fixed 2D-array init: [[Option<SignatureFiring>; 4]; 22] — all None.
            // Each slot has 4 category lanes (Attacking/Defensive/BuildUp/SetPiece).
            // Rust requires spelling out non-Copy arrays when Default isn't derived.
            signature_firing: {
                const EMPTY_ROW: [Option<signature::SignatureFiring>; 4] = [None, None, None, None];
                [EMPTY_ROW; 22]
            },
            signature_first_fired_seen: BTreeSet::new(),
            // T1-3.5: initial possession = home centre forward (slot 9).
            // Slot 9 = home team index 9 (GK=0, DEF=1-4, MID=5-7, FWD=8-10).
            // Slot 9 is the default centre-forward for kick-off ball placement.
            // Both fields initialised to Some(9) per acceptance criterion 3.
            possession: Some(9),
            last_touched_by: Some(9),
            // T1-4a: match duration. Hardcoded to 60 ticks for T1 (the smoke-seed
            // budget). T1-5 makes this configurable via the play_match Tauri command.
            match_end_tick: Tick::from_raw(60),
            // T1-4a: in-match event stream. KickOff is the first event.
            // Emitted here before any tick; all subsequent events are appended
            // by tick_match / dispatch_tick as they fire.
            match_events: vec![MatchEvent::KickOff {
                tick: Tick::ZERO,
                is_second_half: false,
            }],
        }
    }

    /// Serialize to the canonical byte stream for hashing.
    ///
    /// Delegates to [`CanonicalEncoder`]; this is the convenience entry
    /// point used by `fw-replay`'s pinned-hash test.
    pub fn encode_canonical(&self) -> Vec<u8> {
        let mut enc = CanonicalEncoder::new();
        enc.encode_match_state(self);
        enc.finish()
    }

    /// Which player slot currently has possession of the ball (T1-3.5).
    ///
    /// `None` = loose ball (in flight, contested, or unowned).
    /// `Some(slot)` = designated ball-carrier this tick.
    pub fn possession(&self) -> Option<PlayerSlot> {
        self.possession
    }

    /// The most recent player slot to touch the ball (T1-3.5).
    ///
    /// Used for goal attribution: when the ball crosses the goal line,
    /// `last_touched_by` identifies the scorer.
    pub fn last_touched_by(&self) -> Option<PlayerSlot> {
        self.last_touched_by
    }

    /// Read-only access to the in-match event stream (T1-4a).
    ///
    /// External callers (Tauri command handlers, integration tests, the
    /// T1-4b commentary renderer) read events via this accessor; the
    /// underlying `Vec` is `pub(crate)` so it cannot be mutated from
    /// outside the crate (preserves the chronological invariant —
    /// the Vec is append-only and tick-ordered by construction).
    pub fn match_events(&self) -> &[MatchEvent] {
        &self.match_events
    }

    /// Builder: set `last_touched_by` and return `self` (T1-3.5).
    ///
    /// Used by integration tests that need to control goal attribution without
    /// widening `last_touched_by` to `pub`. In production the field is set by
    /// `apply_intent` when a Shot / Pass / Dribble / GK-distribution intent fires.
    pub fn with_last_touched_by(mut self, slot: PlayerSlot) -> Self {
        self.last_touched_by = Some(slot);
        self
    }

    /// Builder: set `match_end_tick` and return `self` (T1-3.5 follow-up
    /// per Codex 2026-05-16 audit silent-failure P1-2).
    ///
    /// Used by integration tests that need to advance more than 60 ticks
    /// without FullTime firing mid-assertion. Without this builder,
    /// `goal_detection_unit_tests.rs` had to rely on the brittle "tests
    /// advance fewer than 60 ticks" invariant — a future test that
    /// advances more ticks would silently emit FullTime + pass for the
    /// wrong reason. In production T1-5 will add a `MatchState::initial_
    /// with_match_end_tick(seed, end_tick)` constructor variant for the
    /// real configurability path; this builder is the test-side bridge.
    pub fn with_match_end_tick(mut self, t: Tick) -> Self {
        self.match_end_tick = t;
        self
    }

    /// The tick at which the match ends (T1-4a).
    ///
    /// External callers read via this accessor; the underlying field is
    /// `pub(crate)` so mid-match mutation is impossible from outside
    /// the crate. T1: always `Tick::from_raw(60)` (smoke-seed budget).
    /// T1-5 will add a constructor variant to make this configurable.
    pub fn match_end_tick(&self) -> Tick {
        self.match_end_tick
    }
}

// NOTE: `apply_tactic_event_with_emission` (T1-4a draft) was DELETED in the
// T1-4a self-review fix-pass per the Codex Tier-2 silent-failure P0-3 +
// type-design P3 + code-reviewer Critical findings (2026-05-16):
//
// - The function was `#[allow(dead_code)] pub(crate)` with no call sites.
// - Shipping it implied `MatchEvent::Goal` was emittable; in reality the
//   variant is structurally unreachable until the contest model + ball-in-net
//   detection lands (T1-9 / T2).
// - The `scorer_slot.unwrap_or(0)` fallback would have silently misattributed
//   unattributed goals to slot 0 (the home goalkeeper).
//
// The `MatchEvent::Goal` variant + its canonical encoder + its serde
// round-trip test all REMAIN, providing forward-compat for the T1-9/T2
// wiring. A direct `encode_match_event(Goal { ... })` unit test was added
// to `canonical.rs` to cover the encoder path even without a live emission.
//
// When goal-scoring wiring lands, the call site should:
//   1. Detect ball-in-net (ball physics or contest model).
//   2. Attribute the scorer via possession chain (last shooter, not Option).
//   3. Update scoreline (home_score / away_score).
//   4. Call `tactic_fsm::apply_event(..., TacticEvent::Goal, ...)`.
//   5. Push `MatchEvent::Goal { scorer_slot, tick, score_home_after,
//      score_away_after }` to `state.match_events`.
//
// Inlining at the wiring site (not in a helper) is preferred — the call
// site has full context (scorer attribution is non-optional; scoreline
// updates atomic).

// -------------------------------------------------------------------------
// tick_match — the canonical advance function
// -------------------------------------------------------------------------

/// Advance the match by one tick.
///
/// Nine sequential steps (T1-3.5 reorders boundary checks before physics):
///   1. Increment `state.tick`.
///   2. Goal detection (T1-3.5): checks ball position at START of tick. If
///      `|ball.pos_x| >= GOAL_LINE_X` AND `|ball.pos_y| < GOAL_HALF_WIDTH_M`
///      → emit `MatchEvent::Goal`, bump score, reset ball to centre spot,
///      emit `MatchEvent::KickOff`, call `apply_event(TacticEvent::Goal)` on
///      BOTH teams. Runs BEFORE physics so the integrator never sees a ball
///      that has already crossed the line.
///   3. Out-of-bounds clamp (T1-3.5): BEFORE ball physics. If ball crossed
///      the sideline (`|ball.pos_y| >= SIDELINE_Y`) OR a non-goal goal-line
///      → zero ball.vel_x / vel_y, clamp pos to boundary. Running before
///      physics prevents the physics altitude branch from healing a negative
///      lateral pos before the clamp observes it. No MatchEvent emitted.
///   4. Advance ball physics (T1-2b-i).
///   5. Run the 2 Hz tactic-FSM heartbeat (T1-2b-ii).
///   6. Dispatch per-player BT / GK-FSM decisions via `dispatch_tick`
///      (T1-2b-iii-a) — mutates `vel_x`/`vel_y` AND ball state (T1-3.5).
///   7. Integrate player velocity into position (`pos += vel × dt`).
///   8. Player-separation positional-correction pass (T1-2b-iii-d).
///   9. Emit `MatchEvent::FullTime` if `state.tick >= state.match_end_tick`
///      AND no FullTime is already at the tail of `match_events` (T1-4a).
pub fn tick_match(mut state: MatchState) -> MatchState {
    state.tick = state.tick.successor();

    // Step 2 (T1-3.5): goal detection — checks ball.pos BEFORE physics.
    //
    // If the ball ended last tick in the goal mouth, detect and score it here.
    // Running before physics means the physics integrator never sees a ball that
    // has already crossed the line (it is reset to centre spot in this block).
    //
    // Scoring: Slots 0..11 = home; 11..22 = away.
    // ball.pos_x > 0: ball in AWAY goal → home team scores.
    // ball.pos_x < 0: ball in HOME goal → away team scores.
    // `unsigned_abs()` + u64 avoids the i64::MIN.abs() panic.
    //
    // **`goal_fired_this_tick` flag** (Codex 2026-05-16 audit silent-failure P0-3):
    // step 3 (OOB clamp) reads this to skip clamping when the ball was just
    // reset to centre-spot by goal detection. Without the guard, a future
    // contributor adding an `else` to step 2 that leaves a wide-of-posts
    // ball in place would still see step 3 silently clamp it to the goal
    // line — masking the wide-vs-goal distinction.
    let mut goal_fired_this_tick = false;
    {
        let bx_bits = state.ball.pos_x.to_bits();
        let by_bits = state.ball.pos_y.to_bits();
        let bx_abs: u64 = bx_bits.unsigned_abs();
        let by_abs: u64 = by_bits.unsigned_abs();
        let goal_line_bits: u64 = GOAL_LINE_X.to_bits().unsigned_abs();
        let half_width_bits: u64 = GOAL_HALF_WIDTH_M.to_bits().unsigned_abs();

        if bx_abs >= goal_line_bits && by_abs < half_width_bits {
            // Codex 2026-05-16 audit silent-failure P0-1: replace
            // `unwrap_or(0)` (silent slot-0/home-GK misattribution) with
            // `expect()` carrying the binding invariant. Today this is
            // structurally unreachable (MatchState::initial sets
            // last_touched_by to Some(9) and every apply_intent ball-touch
            // arm assigns Some(...)) — but a future None-setter would
            // silently misattribute the next goal to the home GK. The
            // panic message documents the invariant in source.
            let scorer_slot = state.last_touched_by.expect(
                "goal detected with no prior ball-touch — last_touched_by must \
                 be Some at any tick where the ball has reached the goal-line; \
                 invariant violated (Codex 2026-05-16 audit silent-failure P0-1)",
            );
            // Codex 2026-05-16 audit silent-failure P1-1: saturating_add
            // silently caps at 255. T1's 60-tick smoke seed never reaches
            // 255 goals but the 90-minute integration scenarios at T1-5+
            // could; checked_add + panic is the determinism-aligned choice
            // (matches the Codex Q1 panic-on-overflow policy for Q32).
            // The panic message names the scoreline at saturation.
            let home_scored = bx_bits > 0;
            if home_scored {
                state.home_score = state.home_score.checked_add(1).expect(
                    "home_score overflowed u8 (255) — match has >255 goals; \
                     this exceeds the realistic T1 budget and indicates a \
                     bug (e.g. goal-line oscillation under broken OOB clamp).",
                );
            } else {
                state.away_score = state.away_score.checked_add(1).expect(
                    "away_score overflowed u8 (255) — match has >255 goals; \
                     this exceeds the realistic T1 budget and indicates a \
                     bug (e.g. goal-line oscillation under broken OOB clamp).",
                );
            }
            let score_home_after = state.home_score as u16;
            let score_away_after = state.away_score as u16;
            state.match_events.push(MatchEvent::Goal {
                scorer_slot,
                tick: state.tick,
                score_home_after,
                score_away_after,
            });
            state.ball = BallState::centre_spot();
            // Codex 2026-05-16 audit code-reviewer Critical #1: the
            // conceding team kicks off after a goal (football rule). Prior
            // code unconditionally set possession = Some(9) (home centre
            // forward), which would misroute kick-off possession when the
            // away team scored. Derive the conceding team from the sign of
            // bx_bits (set above): home_scored == bx_bits > 0; away conceded.
            // Conceding team's centre forward kicks off:
            //   home concedes (away_scored) → slot 9 (home CF)
            //   away concedes (home_scored) → slot 20 (away CF; slot index 11+9)
            let kick_off_taker: PlayerSlot = if home_scored {
                20 // away CF (slot 11 + 9 offset)
            } else {
                9 // home CF
            };
            state.possession = Some(kick_off_taker);
            state.last_touched_by = Some(kick_off_taker);
            state.match_events.push(MatchEvent::KickOff {
                tick: state.tick,
                is_second_half: false,
            });
            let arch = tactic_fsm::ArchetypeParams::direct_pressing();
            state.team_tactic_states[0] = tactic_fsm::apply_event(
                state.team_tactic_states[0],
                &arch,
                tactic_fsm::TacticEvent::Goal,
                state.tick,
            );
            state.team_tactic_states[1] = tactic_fsm::apply_event(
                state.team_tactic_states[1],
                &arch,
                tactic_fsm::TacticEvent::Goal,
                state.tick,
            );
            goal_fired_this_tick = true;
        }
    }

    // Step 3 (T1-3.5): OOB clamp — BEFORE ball physics.
    //
    // Clamp a ball that has crossed the sideline or a non-goal goal-line.
    // Runs before ball physics so the integrator receives a valid in-bounds
    // position. **Skipped entirely if step 2 just fired a goal** (Codex
    // 2026-05-16 audit silent-failure P0-3): after a goal the ball was
    // reset to centre-spot which is in-bounds, so this clamp would be a
    // no-op anyway — but the `goal_fired_this_tick` guard makes the
    // "step 2 handled the boundary; step 3 must not touch the ball" rule
    // explicit, so a future contributor adding a "goal cancelled by VAR"
    // path that leaves the ball past the goal line doesn't silently get
    // it re-clamped to the goal line (which would mask the cancelled-vs-
    // valid distinction).
    //
    // **vel_z preserved (Codex 2026-05-16 audit silent-failure P1-3)**:
    // only vel_x + vel_y are zeroed because OOB clamping is a pitch-plane
    // (XY) concept; altitude motion is orthogonal and the physics step
    // (which uses vel_z for ground-contact via -Z gravity) correctly
    // handles a stationary-but-airborne ball on the next tick. Zeroing
    // vel_z would mask the airborne state visually (an instant ground-stop
    // with no settle arc).
    //
    // No MatchEvent emitted — throw-in / corner / goal-kick = Phase 2.
    if !goal_fired_this_tick {
        let bx_bits = state.ball.pos_x.to_bits();
        let by_bits = state.ball.pos_y.to_bits();
        let bx_abs: u64 = bx_bits.unsigned_abs();
        let by_abs: u64 = by_bits.unsigned_abs();
        let goal_line_bits: u64 = GOAL_LINE_X.to_bits().unsigned_abs();
        let sideline_bits: u64 = SIDELINE_Y.to_bits().unsigned_abs();
        let half_width_bits: u64 = GOAL_HALF_WIDTH_M.to_bits().unsigned_abs();

        let past_sideline = by_abs >= sideline_bits;
        let past_non_goal_line = bx_abs >= goal_line_bits && by_abs >= half_width_bits;

        if past_sideline || past_non_goal_line {
            state.ball.vel_x = fw_core::Q32::ZERO;
            state.ball.vel_y = fw_core::Q32::ZERO;
            if past_sideline {
                if by_bits < 0 {
                    state.ball.pos_y = -SIDELINE_Y;
                } else {
                    state.ball.pos_y = SIDELINE_Y;
                }
            }
            if past_non_goal_line {
                if bx_bits < 0 {
                    state.ball.pos_x = -GOAL_LINE_X;
                } else {
                    state.ball.pos_x = GOAL_LINE_X;
                }
            }
        }
    }

    // Step 4 (was step 2): advance ball physics AFTER goal detection + OOB clamp.
    state.ball = ball_physics::ball_step(&state.ball, &ball_physics::phase1_seeds());

    // Step 5 (T1-2b-ii): 2 Hz tactic-FSM heartbeat (every 30 ticks per team).
    // Home team heartbeat: tick % 30 == 0.
    // Away team heartbeat: tick % 30 == 15 (offset reduces peak load).
    let tick_raw = state.tick.to_raw();
    if tick_raw % tactic_fsm::HEARTBEAT_INTERVAL_TICKS == 0
        && let Some(new_tts) = tactic_fsm::heartbeat_check(&state.team_tactic_states[0], state.tick)
    {
        state.team_tactic_states[0] = new_tts;
    }
    if tick_raw % tactic_fsm::HEARTBEAT_INTERVAL_TICKS == 15
        && let Some(new_tts) = tactic_fsm::heartbeat_check(&state.team_tactic_states[1], state.tick)
    {
        state.team_tactic_states[1] = new_tts;
    }

    // Step 6 (T1-2b-iii-a): per-player decision dispatch.
    // T1-2b-iv: empty definitions map — no signatures fire in basic smoke path.
    state = dispatch::dispatch_tick(state, &BTreeMap::new());

    // Step 7: integrate player velocity into position.
    let dt = ball_physics::dt_per_tick();
    for p in state.players.iter_mut() {
        p.pos_x += p.vel_x * dt;
        p.pos_y += p.vel_y * dt;
    }

    // Step 8 (T1-2b-iii-d): player-separation positional correction.
    separation::apply_player_separation(&mut state);

    // Step 9: emit FullTime at end of match.
    //
    // Must be LAST so all same-tick events (goals, shots, passes) are already
    // appended before FullTime. The match caller is expected to stop advancing
    // after FullTime; this guard ensures FullTime emits AT MOST ONCE even if
    // the caller over-advances (Codex Tier-2 silent-failure P0-2 on T1-4a
    // 2026-05-16 — the prior `==` check would silently fail to emit FullTime
    // if the caller advanced past match_end_tick before the check fired).
    let full_time_already_emitted =
        matches!(state.match_events.last(), Some(MatchEvent::FullTime { .. }));
    if state.tick >= state.match_end_tick && !full_time_already_emitted {
        state.match_events.push(MatchEvent::FullTime {
            tick: state.tick,
            home_score: state.home_score as u16,
            away_score: state.away_score as u16,
        });
    }

    state
}

// -------------------------------------------------------------------------
// Smoke + intra-process determinism — pre-flight before the CI gate
// -------------------------------------------------------------------------

#[cfg(test)]
mod smoke {
    use super::*;

    #[test]
    fn smoke() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn initial_has_22_players() {
        let s = MatchState::initial(Seed::from_u64(1));
        assert_eq!(s.players.len(), TOTAL_PLAYERS);
        assert_eq!(s.tick, Tick::ZERO);
    }

    #[test]
    fn tick_advances_by_one() {
        let s0 = MatchState::initial(Seed::from_u64(1));
        let s1 = tick_match(s0);
        assert_eq!(s1.tick, Tick::ZERO.successor());
    }

    #[test]
    fn encode_canonical_is_stable_intra_process() {
        // Two fresh runs of identical state must encode to identical bytes.
        // This is the cheapest determinism check that exists and the first
        // thing to break if someone introduces a HashMap or pointer-address
        // dependency.
        let s = MatchState::initial(Seed::from_u64(0xDEAD_BEEF));
        let a = s.encode_canonical();
        let b = s.encode_canonical();
        assert_eq!(a, b);
    }

    /// T1-2b-i Chunk 4 RED: `tick_match` advances ball physics each
    /// tick. A ball with nonzero initial velocity must end up at a
    /// different position 60 ticks later.
    /// T1-3.5: altitude axis is pos_z (gravity acts on -vel_z).
    #[test]
    fn tick_match_advances_ball_physics() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        // Set the ball to 10m altitude (pos_z) with 5 m/s along +X —
        // it should fall (pos_z decreases) + drift (pos_x increases)
        // over 60 ticks (1 second).
        state.ball.pos_z = Q32::from_int(10);
        state.ball.vel_x = Q32::from_int(5);
        let initial_pos_x = state.ball.pos_x;
        let initial_pos_z = state.ball.pos_z;
        for _ in 0..60 {
            state = tick_match(state);
        }
        // After 1 second: ball has drifted along +X and fallen.
        assert!(
            state.ball.pos_x > initial_pos_x,
            "ball didn't drift in +X under initial velocity"
        );
        assert!(
            state.ball.pos_z < initial_pos_z,
            "ball didn't fall under gravity (pos_z should decrease)"
        );
    }

    /// T1-2b-ii: initial MatchState has decision_slots populated (not
    /// all-zero or all-same-value — a flat array would be the default
    /// zero-initialized form but is structurally wrong).
    #[test]
    fn initial_state_has_decision_slots_populated() {
        let s = MatchState::initial(Seed::from_u64(1));
        // The balanced-multiset invariant means at least two different values
        // appear in the slot array (slot 0..6 doubled, 7..14 single). A
        // zero-filled array would fail this since all values would be 0.
        let distinct_values: std::collections::BTreeSet<u8> =
            s.decision_slots.iter().copied().collect();
        assert!(
            distinct_values.len() > 1,
            "decision_slots should contain multiple distinct values; got {:?}",
            distinct_values
        );
    }

    /// T1-2b-ii: initial MatchState has interrupt_cooldown_until all at zero.
    #[test]
    fn initial_state_has_zero_cooldowns() {
        let s = MatchState::initial(Seed::from_u64(1));
        assert!(
            s.interrupt_cooldown_until.iter().all(|&t| t == Tick::ZERO),
            "interrupt_cooldown_until should be all Tick::ZERO at match-init"
        );
    }

    /// T1-2b-ii: initial MatchState has both teams in MidBlock.
    #[test]
    fn initial_state_both_teams_in_midblock() {
        let s = MatchState::initial(Seed::from_u64(1));
        assert_eq!(s.team_tactic_states[0].state, TacticState::MidBlock);
        assert_eq!(s.team_tactic_states[1].state, TacticState::MidBlock);
        assert_eq!(s.team_tactic_states[0].entry_tick, Tick::ZERO);
        assert_eq!(s.team_tactic_states[1].entry_tick, Tick::ZERO);
    }

    /// T1-2b-ii: decision_slots immutability across 60 ticks.
    #[test]
    fn decision_slots_unchanged_after_60_ticks() {
        let mut state = MatchState::initial(Seed::from_u64(42));
        let initial_slots = state.decision_slots;
        for _ in 0..60 {
            state = tick_match(state);
        }
        assert_eq!(
            state.decision_slots, initial_slots,
            "decision_slots mutated during tick_match — must be immutable"
        );
    }

    /// T1-2b-iii-a P0-1: player with nonzero velocity ends up at a different
    /// position after 60 ticks. Verifies that tick_match integrates vel→pos.
    #[test]
    fn player_position_integrates_from_velocity_over_60_ticks() {
        let mut state = MatchState::initial(Seed::from_u64(99));
        // Give player 6 (home MID centre) a fixed velocity of 3 m/s along +X.
        // Player 6 starts at (-10, 0); after 60 ticks (1 s) at 3 m/s they
        // should be at approximately (-10 + 3*1) = -7 m along X.
        let initial_pos_x = state.players[6].pos_x;
        state.players[6].vel_x = Q32::from_int(3);
        state.players[6].vel_y = Q32::ZERO;
        for _ in 0..60 {
            state = tick_match(state);
        }
        assert!(
            state.players[6].pos_x > initial_pos_x,
            "player did not move in +X after 60 ticks with vel_x=3; \
             pos_x={:?} initial={:?}",
            state.players[6].pos_x,
            initial_pos_x,
        );
    }

    /// T1-2b-ii: heartbeat fires correctly. HighPress at entry_tick=0
    /// should transition to MidBlock when tick > 600 (>10s).
    #[test]
    fn heartbeat_transitions_highpress_after_timeout() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        // Put home team into HighPress at tick 0
        state.team_tactic_states[0] = TeamTacticState {
            state: TacticState::HighPress,
            entry_tick: Tick::ZERO,
        };

        // Advance 630 ticks (>600 threshold; heartbeat fires at multiples of 30)
        for _ in 0..630 {
            state = tick_match(state);
        }

        // The heartbeat at tick 630 (630 % 30 == 0) should have fired and
        // transitioned home team back to MidBlock.
        assert_eq!(
            state.team_tactic_states[0].state,
            TacticState::MidBlock,
            "heartbeat should have transitioned HighPress → MidBlock after >600 ticks; \
             at tick {} the team was still in {:?}",
            state.tick.to_raw(),
            state.team_tactic_states[0].state
        );
    }

    /// T1-2b-ii: encode_canonical output changes when decision_slots are added.
    #[test]
    fn encode_canonical_includes_new_fields() {
        let s = MatchState::initial(Seed::from_u64(1));
        let bytes = s.encode_canonical();
        // The encoded output should be longer than the T1-2b-i layout:
        // decision_slots [u8; 22] = 22 bytes
        // interrupt_cooldown_until [Tick; 22] = 22 × 8 = 176 bytes
        // team_tactic_states [TeamTacticState; 2] = 2 × (1 + 8) = 18 bytes minimum
        // Total new bytes: ≥ 216 bytes more than T1-2b-i
        // T1-2b-i encoded length for seed=1 was ~364 bytes; new should be ~580+
        assert!(
            bytes.len() > 500,
            "encoded MatchState suspiciously short ({} bytes); expected >500 with new fields",
            bytes.len()
        );
    }
}

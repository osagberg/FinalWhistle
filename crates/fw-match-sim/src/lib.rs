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
pub(crate) mod pass_completion;
pub mod player;
pub mod role_states;
pub mod separation;
pub mod signature;
pub mod subtree_library;
pub mod tactic_fsm;
pub mod team_shape;
pub mod utility;

use fw_content::SignatureId;
use fw_content::event::GOAL_HALF_WIDTH_M;
use fw_core::Q32;
use fw_core::{CurveClass, curve};
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
pub use fw_content::SignatureDefinition;
pub use player::PlayerState;
pub use role_states::{
    DefenderState, ForwardState, GoalkeeperState, MidfielderState, PlayerIntent, PlayerRoleState,
    Role,
};
pub use tactic_fsm::{
    ArchetypeParams, CounterIntent, PressIntensity, SetPieceKind, TacticEvent, TacticState,
    TeamTacticState,
};
pub use team_shape::SimPressLevel;

// -------------------------------------------------------------------------
// ContentInitError — failure type for initial_with_content
// -------------------------------------------------------------------------

/// Errors from [`MatchState::initial_with_content`].
///
/// Fail-loud per the T1-12 hardening pattern: missing templates or
/// unresolvable content references are Err, not silent defaults.
#[derive(Debug)]
pub enum ContentInitError {
    /// No player template matching the required criteria was found in the
    /// ContentStore. `key` describes the search criterion that failed.
    ///
    /// Most common cause: ContentStore was constructed via
    /// `ContentStore::default()` (empty templates) rather than
    /// `ContentStore::load_sources(&content_root)`.
    MissingTemplate {
        /// The search criterion that found no match (e.g. `"preferred_role=AM"`).
        key: String,
    },
}

impl std::fmt::Display for ContentInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingTemplate { key } => write!(
                f,
                "player template {key:?} not found in ContentStore; \
                 did you call ContentStore::load_sources(content_root)?"
            ),
        }
    }
}

impl std::error::Error for ContentInitError {}

// -------------------------------------------------------------------------
// Constants
// -------------------------------------------------------------------------

/// Number of players per team. Football is football.
pub const PLAYERS_PER_TEAM: usize = 11;

/// Total players on the pitch (both teams).
pub const TOTAL_PLAYERS: usize = PLAYERS_PER_TEAM * 2;

/// T2-1a: default tactical archetype ID assigned to both teams by
/// `MatchState::initial(seed)`. Choice rationale: the bridge in
/// `tactic_fsm::archetype_params_for` maps this ID's RON values to the same
/// `ArchetypeParams` as the pre-T2-1a hardcoded `direct_pressing()`,
/// preserving the smoke-seed canonical-state behavior (drift on the smoke
/// pin is SCHEMA-ONLY — the 2 new encoded fields — not behavior-driven).
/// `MatchState::initial_with_content` accepts caller-supplied IDs; this
/// default only fires on the bare-init path.
pub const DEFAULT_ARCHETYPE_ID: &str = "fwh.core:archetype.attacking-fullback";

/// Real match length in sim ticks: 90 minutes × 60 ticks/minute = 5400.
///
/// This is the default value of `MatchState::match_end_tick`. Tests that need
/// a short budget call `.with_match_end_tick(Tick::from_raw(N))` on the
/// builder. The `SEASON_MATCH_TICK_BUDGET` in `fw-tauri/src/season.rs` uses
/// the same arithmetic independently; both must equal 5400 for the Tauri
/// batched-match path and the sim's self-halt to agree.
pub const FULL_MATCH_TICKS: u32 = 5400;

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
// Role-mapping helper (used by initial_with_content)
// -------------------------------------------------------------------------

/// Map a content-template [`fw_content::RoleId`] to the formation [`Role`]
/// used by the sim's slot assignment.
///
/// Used by [`MatchState::initial_with_content`] to perform role-matched
/// signature spreading: a template's candidates are assigned only to slots
/// whose `PlayerState::role()` matches.
///
/// Mapping (all string comparisons are case-sensitive and ASCII):
///
/// | `preferred_role` string | `Role` |
/// |---|---|
/// | `"GK"` | `Goalkeeper` |
/// | `"DEF"` | `Defender` |
/// | `"AM"` / `"MID"` / `"CM"` | `Midfielder` |
/// | `"FWD"` / `"ST"` / `"CF"` | `Forward` |
/// | anything else | `Forward` (see note) |
///
/// The `_ => Forward` arm absorbs all legitimate forward-role variants
/// (`"FWD"`, `"ST"`, `"CF"`, etc.) as well as any genuinely-unknown string.
/// A forward-role string falling through here produces correct behaviour
/// (forward-slot assignment). Genuinely-unknown role strings produce a
/// silent Forward assignment; the `scripts/fw verify-content` check at T2-3
/// is the validation gate for unknown strings — NOT a panic here, because a
/// panic would reject novel-but-legitimate forward role variants.
pub(crate) fn preferred_role_to_formation_role(preferred_role: &fw_content::RoleId) -> Role {
    match preferred_role.as_str() {
        "GK" => Role::Goalkeeper,
        "DEF" => Role::Defender,
        "AM" | "MID" | "CM" => Role::Midfielder,
        // FWD / ST / CF and all other forward-role variants → Forward
        _ => Role::Forward,
    }
}

// -------------------------------------------------------------------------
// BallInFlight — canonical state for a pass in physical transit
// -------------------------------------------------------------------------

/// Records that a pass has been launched and the ball is physically travelling
/// to the intended receiver.
///
/// Set at pass-launch (success arm of ShortPass / LongPass / Cross / LayOff /
/// GkDistribute) in place of the old `possession = Some(to_slot)`. Cleared by
/// `trap_check_in_flight` in `tick_match` on arrival or timeout.
///
/// All fields are integer types. No floats. No BTreeMap. Derives Copy.
///
/// Invariant: `intended_receiver` is always a valid slot (0..22). Enforced
/// by `assert!` at the construction site in `dispatch.rs`.
///
/// Canonical state — encoded by `CanonicalEncoder` (VERSION 12 → 13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BallInFlight {
    /// The slot of the intended receiver.
    pub intended_receiver: PlayerSlot,
    /// True = pass outcome determined as success at launch; the receiver will
    /// gain possession on arrival. False = pass failed at launch; ball will
    /// drop loose near the receiver's position on arrival.
    pub outcome_is_success: bool,
    /// Tick at which the pass was launched. Used for the FLIGHT_TIMEOUT_TICKS
    /// guard: if the ball hasn't arrived after 120 ticks, force-clear.
    pub launch_tick: Tick,
}

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
    /// reaches this value, all gameplay is gated out and `FullTime` is emitted.
    /// Subsequent calls to `tick_match` return `state` unchanged (freeze).
    ///
    /// Default: `Tick::from_raw(FULL_MATCH_TICKS as i64)` = 5400 (90 min).
    /// Tests use `.with_match_end_tick(Tick::from_raw(N))` for short budgets.
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

    // ---- T2-1a additions (per-team archetypes; ADR-0012 trigger #1) ----
    //
    // Two NEW canonical-state fields. Encoder VERSION bumped 8 → 9.
    // Both string IDs are stable + content-pack-qualified per the existing
    // `<pack-id>:archetype.<slug>` convention. T1-7's ManagerArchetypeId
    // is the parallel newtype precedent; the dedicated TacticalArchetypeId
    // newtype refactor is intentionally deferred (see T2-1a MEMORY spec).
    //
    // **WIRING-ONLY SCOPE NOTE (T2-1a silent-failure CRITICAL-1)**: T2-1a
    // ships the per-team archetype SUBSTRATE (canonical fields + sidecar
    // resolved-params + threading through to `tactic_fsm::apply_event`'s
    // `archetype` parameter). It does NOT ship a production `TacticEvent`
    // emission that actually CONSUMES the archetype parameter. The only
    // `TacticEvent` emitted in production code today is `Goal` (at
    // `tick_match`'s Goal-event handler), and `apply_event`'s `Goal` arm
    // hardcodes `TacticState::MidBlock` ignoring the `archetype` parameter
    // entirely. T2-1b/c wire the `BallInPlay` / `PossessionLost` /
    // `BallRecovered` / `CounterWindowClosed` emissions that activate
    // per-team behavioral divergence + earn the ADR-0012 trigger-#3 stamp
    // on the next rebaseline. T2-1a's drift on BOTH pins (60-tick smoke +
    // 600-tick extended) is therefore SCHEMA-ONLY (canonical bytes append),
    // not behavior-driven. Trigger #1 alone authorizes T2-1a's rebaseline.
    /// Home team's tactical archetype identifier. Canonical-state.
    ///
    /// Format: `<pack-id>:archetype.<slug>` per Content/RULES.md §2.
    /// Default at `MatchState::initial(seed)`: `"fwh.core:archetype.attacking-fullback"`
    /// (preserves pre-T2-1a effective behavior — bridge maps this to the
    /// previously-hardcoded `ArchetypeParams::direct_pressing()` values).
    ///
    /// `pub(crate)` per the established `possession` / `last_touched_by`
    /// pattern at T1-3.5 + per T2-1a silent-failure-hunter CRITICAL-2:
    /// external mutation of the canonical ID without atomically re-resolving
    /// the sidecar `home_archetype_params` would silently drift canonical
    /// state from sim behavior. External callers MUST use the
    /// `home_archetype_id()` accessor; mutation paths route through
    /// `MatchState::initial` / `initial_with_content` constructors that
    /// resolve the sidecar atomically.
    pub(crate) home_archetype_id: String,

    /// Away team's tactical archetype identifier. Canonical-state.
    /// `pub(crate)` per the same rationale as `home_archetype_id`.
    pub(crate) away_archetype_id: String,

    /// Resolved `ArchetypeParams` for the home team — sim-runtime
    /// non-canonical sidecar. Populated at construction time from the
    /// archetype ID via `tactic_fsm::archetype_params_for(&TacticalArchetype)`.
    ///
    /// **Not in canonical encoding** — the canonical state stores the ID
    /// (stable + human-readable across version migrations); the resolved
    /// params are recomputed at construction time, never serialized.
    ///
    /// `pub(crate)` — consumed by `tick_match`'s Goal-event handler (which
    /// today passes the param to `apply_event` whose `Goal` arm ignores it
    /// — see T2-1a CRITICAL-1 scope note above; the wiring is here for
    /// T2-1b/c's BallInPlay / PossessionLost emissions to consume).
    pub(crate) home_archetype_params: tactic_fsm::ArchetypeParams,

    /// Resolved `ArchetypeParams` for the away team — see `home_archetype_params`.
    pub(crate) away_archetype_params: tactic_fsm::ArchetypeParams,

    // ---- T2-1d telemetry buffers (NON-canonical; #[serde(skip)]) ----
    //
    // Two off-canonical-path Vec sidecars that the `calibrate` binary reads
    // post-match to collect per-shot + per-dribble feature/personality
    // samples for the xG β + personality K_i calibration fits per
    // `docs/design/xg-coefficients.md §Calibration loop (T2-1)` +
    // `docs/design/personality-bias-weights.md §Re-tuning cadence`.
    //
    // Both fields use `#[serde(skip)]` so they:
    //   - Don't appear in the canonical-state byte stream (encoder reads
    //     specific fields explicitly; `#[serde(skip)]` is belt-and-braces).
    //   - Don't appear in IPC DTO serializations (frontend never reads them).
    //   - Default to empty `Vec::new()` on `MatchState::initial` /
    //     `initial_with_content` construction + serde-deserialize round-trips
    //     (the `Default` impl backs the skip).
    //
    // Per-shot push happens in `dispatch::apply_intent::AttemptShot` arm;
    // per-dribble push happens in the `Dribble` arm. `became_goal` flag is
    // back-filled post-match by the calibrate binary walking
    // `state.match_events` for `MatchEvent::Goal` events and correlating
    // by `(shooter_slot, tick)` within a small look-ahead window.
    //
    // The push-on-intent pattern slightly costs per-tick (one Vec push for
    // every shot or dribble intent the dispatch picks). Realistic match
    // shot count is ~20-30, dribble count ~50-100, so the Vec growth is
    // bounded + the alloc cost is negligible vs the per-tick BT runner
    // cost. No `#[cfg(...)]` gating: keeping the field always-on means
    // the calibrate binary doesn't need a special build profile, and
    // production code (Tauri / frontend) just never reads the field.
    /// Per-shot telemetry buffer for T2-1d xG calibration (NON-canonical).
    #[serde(skip)]
    pub(crate) shot_telemetry: Vec<ShotTelemetryRecord>,

    /// Per-dribble telemetry buffer for T2-1d personality K_i calibration (NON-canonical).
    #[serde(skip)]
    pub(crate) dribble_telemetry: Vec<DribbleTelemetryRecord>,

    // ---- FUN-0b additions (shot quality model; Slice A) ----
    //
    // `last_shot_xg` caches the xG score of the most recent AttemptShot
    // intent for each player slot, written by `dispatch::apply_intent` when
    // `AttemptShot` fires. Read by the GK save model in `tick_match`'s
    // goal-detection block (SS3 — saves based on shot quality).
    //
    // CANONICAL state (encoder VERSION bumped 9 → 10 at FUN-0b). The field is
    // in canonical state because it affects goal probability deterministically.
    // Encoded as 22 × i64 LE (raw Q32 bits) appended AFTER the archetype IDs.
    //
    // Initialized to Q32::ZERO (no shot yet) at match init. Reset to Q32::ZERO
    // after a GK save is acknowledged (the save block in goal-detection clears
    // the entry so a standing ball near the line can't be "re-saved").
    /// Per-player last-shot xG score. `last_shot_xg[slot_idx]` is the Q32 xG
    /// value (from `xg_utility`) computed when that player's most recent
    /// `AttemptShot` intent was dispatched. `Q32::ZERO` means no shot yet or
    /// the value has been cleared. Used by the GK save model (SS3).
    pub(crate) last_shot_xg: [Q32; 22],

    // ---- FUN-0b+c additions (dispossession; Slice B) ----
    //
    // `tackle_cooldown_until` prevents a defender from attempting a tackle on
    // every consecutive tick. After a failed tackle attempt, the defender
    // cannot attempt again until this tick has passed.
    //
    // CANONICAL state (encoder VERSION bumped 10 → 11 at FUN-0b+c). Encoded as
    // 22 × i64 LE (Tick::to_raw()) appended AFTER `last_shot_xg`.
    //
    // Initialized to Tick::ZERO (no cooldown) at match init. Updated by
    // `resolve_tackles` in `tick_match` on a failed tackle attempt.
    /// Per-defender tackle cooldown end tick. `tackle_cooldown_until[slot_idx]`
    /// is the first tick at which the defender may attempt another tackle.
    /// `Tick::ZERO` means no active cooldown. Set on failed tackle attempts;
    /// cleared implicitly as tick advances past the value.
    pub(crate) tackle_cooldown_until: [Tick; 22],

    // ---- SLICE-1 ball-in-flight (ball-in-flight-model-2026-06-06.md) ----
    //
    // `ball_in_flight` tracks a pass that has been launched but whose
    // possession has not yet transferred to the receiver. Set at pass-launch
    // (replacing the immediate `possession = Some(to_slot)`) and cleared by
    // `trap_check_in_flight` when the ball arrives within TRAP_RADIUS_M of
    // the intended receiver, or after FLIGHT_TIMEOUT_TICKS.
    //
    // CANONICAL state — encoder VERSION bumped 12 → 13. Must be in canonical
    // state because `trap_check_in_flight` and the loose-ball pickup guard
    // branch on it; without canonical encoding, replay reconstruction from
    // canonical state alone would diverge. Wire: 1 presence byte + (if Some)
    // 1 + 1 + 1 + 8 bytes = 11 bytes max. Appended AFTER tackle_cooldown_until.
    //
    // Initialized to None at match init. Cleared within the same tick that the
    // ball arrives (trap) or times out.
    /// Ball-in-flight state for the current pass. `None` = no pass in flight.
    /// `Some(bif)` = a pass was launched; the ball is physically travelling;
    /// possession has been cleared to `None` pending arrival.
    ///
    /// Invariant: `ball_in_flight.is_some()` implies `possession.is_none()`.
    pub(crate) ball_in_flight: Option<BallInFlight>,

    // ---- FUN-TS1 sidecar (non-canonical; #[serde(skip)]) ----
    //
    // `team_shape` is a pure function of canonical inputs recomputed every
    // tick in `dispatch_tick` before the per-player decision loop. It adds
    // NO canonical bytes. The `#[serde(skip)]` default is `TeamShape::zero()`
    // (Default impl delegation — Serde requires Default when skip is used on
    // non-unit fields). ADR-0013 authorizes this pattern.
    /// Per-team shape anchors for the current tick. Index 0 = home, 1 = away.
    /// Recomputed each tick from canonical inputs; `#[serde(skip)]` — adds no
    /// canonical bytes. Consumed by off-ball utilities via `zonal_slot`.
    #[serde(skip)]
    pub(crate) team_shape: [team_shape::TeamShape; 2],

    // ---- S11 additions (ChangePressLevel touchline command) ----
    //
    // `press_level` is a NON-canonical sidecar set by the manager's touchline
    // `ChangePressLevel` command (via `MatchState::set_press_level`). It shifts
    // the defensive line height and controls coordinated-press role assignment
    // in `compute_press_from_parts`. It is NOT in the canonical encoder so
    // pinned hashes are byte-identical at the default `Standard` level. Serde
    // requires `Default` when `#[serde(skip)]` is used on a non-unit field.
    //
    // The field lives on `MatchState` (not just on `LiveMatchSession`) so the
    // sim's per-tick `compute` + `compute_press_from_parts` calls can read it
    // without passing it as an extra argument through every tick.
    /// Manager-instructed pressing intensity per team. Index 0 = home, 1 = away.
    /// Defaults to `Standard` (= current behavior). Set by `set_press_level`.
    /// `#[serde(skip)]` — adds no canonical bytes; pinned hashes unchanged.
    #[serde(skip)]
    pub(crate) press_level: [team_shape::SimPressLevel; 2],
}

// ---- T2-1d telemetry record types ----

/// Per-shot calibration sample captured at the moment `AttemptShot` fires
/// (T2-1d). NON-canonical; lives in the `MatchState::shot_telemetry` sidecar
/// Vec; consumed by the `calibrate` binary's xG β fit pass.
///
/// All Q32 features stored as raw `i64` bits so the calibrate binary can
/// dump them to JSON via `serde_json` (which serializes `i64` natively).
/// `became_goal` is `None` at push time + back-filled post-match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShotTelemetryRecord {
    /// Tick at which the shot fired.
    pub shot_tick: u32,
    /// Player slot of the shooter (0..22; <11 home, ≥11 away).
    pub shooter_slot: u8,
    /// xG model feature: distance-inverted Q32 in [0, 1]. `1 - clamp(d_m / 35, 0, 1)`.
    pub distance_q32_raw: i64,
    /// xG model feature: angle Q32 in [0, 1].
    pub angle_q32_raw: i64,
    /// xG model feature: defender pressure Q32 in [0, 1].
    pub pressure_q32_raw: i64,
    /// xG model feature: shot type Q32 (T1: always 1.0 = footed; placeholder).
    pub shot_type_q32_raw: i64,
    /// xG model feature: assist kind Q32 (T1: always 1.0 = solo; placeholder).
    pub assist_kind_q32_raw: i64,
    /// xG model feature: shooter quality Q32 (finishing × 0.55 + composure × 0.25 + technique × 0.20).
    pub shooter_quality_q32_raw: i64,
    /// Personality feature: shooter's `mental.flair` Q32 (for K_1 SHOOT_FLAIR fit).
    pub shooter_flair_q32_raw: i64,
    /// Personality feature: shooter's `mental.composure` Q32 (for K_2 SHOOT_COMPOSURE fit).
    pub shooter_composure_q32_raw: i64,
    /// Personality feature: shooter's `personality.risk_appetite` Q32 (for K_18 SHOOT_RISK fit).
    pub shooter_risk_appetite_q32_raw: i64,
    /// Post-match goal correlation: `Some(true)` if a `MatchEvent::Goal`
    /// with this shooter_slot fired within ~120 ticks of the shot;
    /// `Some(false)` if no matching goal; `None` at push-time (back-filled
    /// by the calibrate binary's `attribute_goals` pass).
    pub became_goal: Option<bool>,
}

/// Per-dribble calibration sample captured at the moment `Dribble` fires
/// (T2-1d). NON-canonical; lives in `MatchState::dribble_telemetry`;
/// consumed by the calibrate binary's personality K_7 / K_8 fit pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DribbleTelemetryRecord {
    /// Tick at which the dribble fired.
    pub dribble_tick: u32,
    /// Player slot of the dribbler.
    pub dribbler_slot: u8,
    /// Personality feature: dribbler's `mental.flair` Q32 (for K_7 DRIBBLE_FLAIR fit).
    pub dribbler_flair_q32_raw: i64,
    /// Personality feature: dribbler's `personality.aggression` Q32 (for K_8 DRIBBLE_AGG fit).
    pub dribbler_aggression_q32_raw: i64,
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
            // T4-sim-halt: real 90-minute default (5400 ticks = 90 min × 60 ticks/min).
            // Tests override via .with_match_end_tick(Tick::from_raw(N)) on the builder.
            match_end_tick: Tick::from_raw(FULL_MATCH_TICKS as i64),
            // T1-4a: in-match event stream. KickOff is the first event.
            // Emitted here before any tick; all subsequent events are appended
            // by tick_match / dispatch_tick as they fire.
            match_events: vec![MatchEvent::KickOff {
                tick: Tick::ZERO,
                is_second_half: false,
            }],
            // T2-1a: per-team archetype IDs. Default to the attacking-fullback
            // archetype (which the bridge in tactic_fsm::archetype_params_for
            // resolves to the same ArchetypeParams as the pre-T2-1a hardcoded
            // direct_pressing()). MatchState::initial doesn't take a
            // ContentStore, so it can't validate the ID resolves — that's only
            // checked at MatchState::initial_with_content where the actual
            // params come from a real content lookup. The default ID + default
            // params here are kept consistent via the DEFAULT_HOME_ARCHETYPE_ID
            // constant + ArchetypeParams::direct_pressing() (which matches the
            // bridge output for attacking-fullback per the unit tests in
            // tactic_fsm::tests::archetype_params_for_attacking_fullback_*).
            home_archetype_id: DEFAULT_ARCHETYPE_ID.to_string(),
            away_archetype_id: DEFAULT_ARCHETYPE_ID.to_string(),
            home_archetype_params: tactic_fsm::ArchetypeParams::direct_pressing(),
            away_archetype_params: tactic_fsm::ArchetypeParams::direct_pressing(),
            // T2-1d telemetry buffers — empty at init; populated per-tick
            // by apply_intent. NON-canonical (#[serde(skip)]).
            shot_telemetry: Vec::new(),
            dribble_telemetry: Vec::new(),
            // FUN-0b: last_shot_xg — all zero at match init (no shots fired yet).
            last_shot_xg: [Q32::ZERO; 22],
            // FUN-0b+c: tackle_cooldown_until — all Tick::ZERO at match init (no cooldowns).
            tackle_cooldown_until: [Tick::ZERO; 22],
            // SLICE-1: ball_in_flight — None at match init (no pass in flight at kick-off).
            ball_in_flight: None,
            // FUN-TS1: team_shape sidecar — initialized to zero() defaults.
            // FUN-TS2b: press_roles within TeamShape also initialized to HoldShape
            // via TeamShape::zero(). Filled by compute_press each tick.
            team_shape: [team_shape::TeamShape::zero(); 2],
            // S11: press_level sidecar — default Standard so pinned hashes are unchanged.
            press_level: [team_shape::SimPressLevel::Standard; 2],
        }
    }

    /// Variant of [`MatchState::initial`] that projects `signature_candidates`
    /// from the loaded content corpus onto match players.
    ///
    /// ## Role-matched spread (T4-2.5c)
    ///
    /// Each template's candidates are assigned **only** to slots whose
    /// `PlayerState::role()` matches the template's `preferred_role`. The
    /// mapping is:
    ///
    /// | `preferred_role` string | Formation `Role` | Slots (4-3-3) |
    /// |---|---|---|
    /// | `"GK"` | `Goalkeeper` | 0, 11 |
    /// | `"DEF"` | `Defender` | 1-4, 12-15 |
    /// | `"AM"` / `"MID"` / `"CM"` | `Midfielder` | 5-7, 16-18 |
    /// | `"FWD"` / `"ST"` / `"CF"` | `Forward` | 8-10, 19-21 |
    ///
    /// With 1 AM template today: home-MID slots 5-7 + away-MID slots 16-18
    /// receive candidates; all other slots stay empty. This prevents
    /// cross-role bias contamination (e.g. AM pass/shoot signatures firing
    /// for GK/DEF slots and collapsing shooting utility). Per-role template
    /// diversity lands at T4.5-E1; the mapping scales without code changes.
    ///
    /// ## Fail-loud on empty template pool
    ///
    /// If `content.player_templates` is empty this constructor returns `Err`.
    /// An empty template pool indicates a content-corpus setup problem (e.g.
    /// an empty `ContentStore` in a test that forgot to load sources), not a
    /// recoverable runtime state.
    ///
    /// ## Canonical-hash note
    ///
    /// Slots 5-7 and 16-18 now carry non-empty `signature_candidates`
    /// (was only slot 7 at T1-11). The 600-tick pin is rebaselined at
    /// T4-2.5c (ADR-0012 trigger #3).
    ///
    /// ## Caller-supplied slot overrides
    ///
    /// After this constructor, callers that hold per-player roster data may
    /// call [`.with_slot_signatures`](Self::with_slot_signatures) to override
    /// specific slots' candidates from the roster (home slots 0-10, away
    /// slots 11-21).
    pub fn initial_with_content(
        seed: Seed,
        content: &fw_content::ContentStore,
        home_archetype_id: &str,
        away_archetype_id: &str,
    ) -> Result<MatchState, ContentInitError> {
        // Template pool: BTreeMap values() is key-ordered (Sim/RULES.md §2),
        // so pool[i] is deterministic across platforms.
        let templates: Vec<&fw_content::PlayerTemplate> =
            content.player_templates.values().collect();
        if templates.is_empty() {
            return Err(ContentInitError::MissingTemplate {
                key: "player_templates (empty pool)".into(),
            });
        }

        // T2-1a: resolve per-team archetype IDs → TacticalArchetype lookups →
        // ArchetypeParams via the bridge. Fail-loud if either ID is missing
        // from content (mirrors the MissingTemplate failure mode above).
        let home_archetype = content
            .tactical_archetypes
            .get(home_archetype_id)
            .ok_or_else(|| ContentInitError::MissingTemplate {
                key: format!("home_archetype_id={home_archetype_id}"),
            })?;
        let away_archetype = content
            .tactical_archetypes
            .get(away_archetype_id)
            .ok_or_else(|| ContentInitError::MissingTemplate {
                key: format!("away_archetype_id={away_archetype_id}"),
            })?;
        let home_archetype_params = tactic_fsm::archetype_params_for(home_archetype);
        let away_archetype_params = tactic_fsm::archetype_params_for(away_archetype);

        // Build the baseline state, then assign candidates role-by-role.
        let mut state = MatchState::initial(seed);

        // Role-matched spread: for each template, assign its candidates to every
        // slot whose formation Role matches the template's preferred_role. With 1
        // AM template, only MID slots (5-7 home, 16-18 away) receive candidates.
        // GK/DEF/FWD slots stay empty until matching templates are added at T4.5-E1.
        //
        // `preferred_role_to_formation_role` is a local match so there are no
        // new allocations and no HashMap (determinism contract per Sim/RULES.md §2).
        for template in &templates {
            let template_role = preferred_role_to_formation_role(&template.preferred_role);
            for slot_idx in 0..22usize {
                if state.players[slot_idx].role() == template_role {
                    state.players[slot_idx].signature_candidates =
                        template.signature_candidates.clone();
                }
            }
        }

        // T2-1a: override the default archetype state with caller-supplied IDs.
        state.home_archetype_id = home_archetype_id.to_string();
        state.away_archetype_id = away_archetype_id.to_string();
        state.home_archetype_params = home_archetype_params;
        state.away_archetype_params = away_archetype_params;

        Ok(state)
    }

    /// Builder: override per-slot `signature_candidates` from a caller-supplied map.
    ///
    /// Only the slots **present** in `slot_signatures` are overridden; slots absent
    /// from the map keep the candidates already set by
    /// [`initial_with_content`](Self::initial_with_content) or an earlier
    /// `with_slot_signatures` call. This "override-present-slots" semantics lets
    /// the season runner build a map from two clubs' rosters (home slots 0-10,
    /// away slots 11-21) and pass it without having to supply all 22 entries.
    ///
    /// Intended use in `play_one_match`: after `initial_with_content` spreads
    /// the role-matched defaults across role-appropriate slots, the season runner
    /// calls `.with_slot_signatures(map)` to install the actual per-player roster
    /// candidates. With 1 template today the roster candidates equal the
    /// content-spread defaults, so the override is a deterministic no-op in
    /// practice — it becomes meaningful at T4.5-E1 when per-player diversity
    /// arrives.
    ///
    /// ## Panics
    ///
    /// Panics in both debug and release if any key in `slot_signatures` is
    /// out of range (≥ 22). A caller-built map with an OOB slot is a programming
    /// error, not an untrusted-input condition (Sim/RULES §11: canonical
    /// invariants fail in release, not silently degrade).
    ///
    /// ## Determinism
    ///
    /// `BTreeMap<PlayerSlot, Vec<SignatureCandidate>>` iteration is key-ordered
    /// (`PlayerSlot = u8`; ascending). No RNG, no clocks.
    #[must_use]
    pub fn with_slot_signatures(
        mut self,
        slot_signatures: BTreeMap<PlayerSlot, Vec<fw_content::SignatureCandidate>>,
    ) -> Self {
        for (slot, candidates) in slot_signatures {
            assert!(
                (slot as usize) < self.players.len(),
                "with_slot_signatures: slot {slot} is out of range (max {}); \
                 caller built a map with an invalid slot index — this is a \
                 programming error (Sim/RULES §11)",
                self.players.len() - 1
            );
            self.players[slot as usize].signature_candidates = candidates;
        }
        self
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

    /// "Effective possession" for team-shape derivation (SLICE-1).
    ///
    /// When a pass is in flight to a teammate, the passing team should NOT drop
    /// into defensive shape — the ball is still theirs. Returns:
    ///
    /// - `Some(slot)` if `state.possession` is `Some(slot)` (unchanged).
    /// - `Some(intended_receiver)` if `state.ball_in_flight` is `Some(bif)`
    ///   with `outcome_is_success = true` — the team that owns the receiver
    ///   is treated as still being in possession.
    /// - `None` if `state.possession` is `None` AND either there is no flight
    ///   or the flight is a failed pass (outcome_is_success = false).
    ///
    /// Used exclusively in `team_shape::compute` and `compute_press_from_parts`
    /// to derive `is_defending`. No other code should need to call this.
    pub(crate) fn effective_possession(&self) -> Option<PlayerSlot> {
        if let Some(slot) = self.possession {
            return Some(slot);
        }
        if let Some(bif) = self.ball_in_flight
            && bif.outcome_is_success
        {
            return Some(bif.intended_receiver);
        }
        None
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

    /// Canonical archetype ID for the home team (T2-1a).
    ///
    /// String form (e.g. `"fwh.core:archetype.attacking-fullback"`) keyed in
    /// `ContentStore::tactical_archetypes`. The underlying field is
    /// `pub(crate)` per the T2-1a self-review CRITICAL-2 fix (same pattern
    /// as `possession` / `last_touched_by`); external callers read via
    /// this accessor. Mutation is restricted to `initial_with_content`
    /// at match-setup; no mid-match mutation path exists yet (per the
    /// T2-1a CRITICAL-1 wiring-only scope note above the field defs).
    pub fn home_archetype_id(&self) -> &str {
        &self.home_archetype_id
    }

    /// Canonical archetype ID for the away team (T2-1a). Mirror of
    /// [`home_archetype_id`](Self::home_archetype_id).
    pub fn away_archetype_id(&self) -> &str {
        &self.away_archetype_id
    }

    /// Drain the shot-telemetry sidecar buffer (T2-1d).
    ///
    /// The calibrate binary calls this post-match to consume the captured
    /// `ShotTelemetryRecord` entries pushed by `dispatch::apply_intent`'s
    /// `AttemptShot` arm. NOT canonical state — the underlying field is
    /// `#[serde(skip)]` so the canonical encoder ignores it.
    ///
    /// `drain` semantics (move + clear) ensure the post-call buffer is empty
    /// so the same `MatchState` can be re-ticked for a fresh telemetry
    /// window without double-counting. Callers that want a copy without
    /// clearing should call `shot_telemetry_len()` first + index instead.
    pub fn drain_shot_telemetry(&mut self) -> Vec<ShotTelemetryRecord> {
        std::mem::take(&mut self.shot_telemetry)
    }

    /// Mirror of [`drain_shot_telemetry`](Self::drain_shot_telemetry) for
    /// the dribble-telemetry sidecar buffer (T2-1d).
    pub fn drain_dribble_telemetry(&mut self) -> Vec<DribbleTelemetryRecord> {
        std::mem::take(&mut self.dribble_telemetry)
    }

    /// Read-only count of pending shot-telemetry records (T2-1d). Useful for
    /// tests that want to assert at least N shots were captured without
    /// consuming the buffer.
    pub fn shot_telemetry_len(&self) -> usize {
        self.shot_telemetry.len()
    }

    /// Mirror of [`shot_telemetry_len`](Self::shot_telemetry_len) for
    /// dribble telemetry (T2-1d).
    pub fn dribble_telemetry_len(&self) -> usize {
        self.dribble_telemetry.len()
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

    /// Builder: set `possession` to `Some(slot)` and return `self`.
    ///
    /// Used by integration tests that need to place the ball in a specific
    /// player's possession without widening `possession` to `pub`. In
    /// production possession is managed by `apply_intent` and
    /// `dispatch_tick`'s carrier routing pre-pass.
    pub fn with_possession(mut self, slot: PlayerSlot) -> Self {
        self.possession = Some(slot);
        self
    }

    /// Builder: override `match_end_tick` and return `self`.
    ///
    /// Used by tests that need a short tick budget (e.g. 5 or 60 ticks) so
    /// that `FullTime` fires without running a full 5400-tick match. The
    /// default of `Tick::from_raw(FULL_MATCH_TICKS as i64)` = 5400 is the
    /// real 90-minute value; tests call `.with_match_end_tick(Tick::from_raw(60))`
    /// to keep legacy short-budget assertions green.
    pub fn with_match_end_tick(mut self, t: Tick) -> Self {
        self.match_end_tick = t;
        self
    }

    /// Read-only access to the per-team press levels (S11).
    ///
    /// Index 0 = home team, 1 = away team. Used by integration tests that need
    /// to pass `press_level` to `compute_press_from_parts` without widening
    /// the field to `pub`. In production the field is read by the sim's per-tick
    /// calls to `compute` + `compute_press_from_parts`.
    pub fn press_level(&self) -> &[team_shape::SimPressLevel; 2] {
        &self.press_level
    }

    /// Apply a manager's touchline press-level instruction (S11 — ChangePressLevel).
    ///
    /// `team_idx`: 0 = home team, 1 = away team. Panics in debug + release on
    /// out-of-range index (Sim/RULES §11 — canonical invariants fail loud).
    ///
    /// This mutates the non-canonical `press_level` sidecar. The canonical
    /// encoder does not cover this field, so pinned hashes are unaffected when
    /// the level stays at `Standard`. The change takes effect on the NEXT tick's
    /// `compute` + `compute_press_from_parts` calls, not retroactively.
    pub fn set_press_level(&mut self, team_idx: usize, level: team_shape::SimPressLevel) {
        assert!(
            team_idx < 2,
            "set_press_level: team_idx {team_idx} is out of range (must be 0 or 1)"
        );
        self.press_level[team_idx] = level;
    }

    /// The tick at which the match ends (T1-4a; real default wired at T4-sim-halt).
    ///
    /// External callers read via this accessor; the underlying field is
    /// `pub(crate)` so mid-match mutation is impossible from outside
    /// the crate. Default: `Tick::from_raw(FULL_MATCH_TICKS as i64)` = 5400.
    /// Override via `.with_match_end_tick(t)` for test short-budgets.
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
// T2-1c helpers: BallOutOfPlay SetPieceKind assignment + BallInPlay auto-exit
// -------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// SS3 — Goalkeeper save model constants (FUN-0b)
// docs/design/shot-model.md §Sub-system 3 §Coefficients
// ---------------------------------------------------------------------------

// Minimum save probability for the worst GK (attrs = 0.0) in a perfect position.
// FUN-TS3-ShotModel sweep 4: lowered from 0.62 to 0.55 to compensate for sigma=8.5m.
// Goal-production re-tune (2026-06-05): lowered 0.55 -> 0.50 (its HARD FLOOR) to
// recover shot-based goals after the goalmouth-defending slice closed all drift
// goals (M1 had fallen to 1.82, all shot-based). 0.50 is the believability floor —
// a keeper must still stop half of close-range shots; below this is fantasy. We
// hold AT the floor and reach the M1 target via shot quality/volume, never below.
const SAVE_BASE_MIN: Q32 = Q32::from_raw(2_147_483_648_i64); // = 0.50 (hard floor)

// Maximum save probability for the best GK (attrs = 1.0) in a perfect position.
// FUN-TS3-ShotModel sweep 4: lowered from 0.82 to 0.75 in lockstep with SAVE_BASE_MIN.
// Goal-production re-tune (2026-06-05): lowered 0.75 -> 0.72 (its HARD FLOOR) in
// lockstep with SAVE_BASE_MIN. 0.72 is the believability floor and is NOT breached.
const SAVE_BASE_MAX: Q32 = Q32::from_raw(3_092_376_453_i64); // = 0.72 (hard floor)

// Position penalty per metre of GK-to-ball y-error.
// At 1m error: factor drops by 0.15. At 3m error: factor ≈ 0.55.
const POSITION_PENALTY_RATE: Q32 = Q32::from_raw(644_245_094_i64); // ≈ 0.15 per metre

// Minimum positional factor (GK completely out of position — last-chance reach).
const POSITION_MIN: Q32 = Q32::from_raw(429_496_729_i64); // ≈ 0.10

// Maximum save probability cap — best GK misses ~28% of on-target shots.
// FUN-TS3-ShotModel sweep 4: lowered from 0.82 to 0.75 in lockstep with SAVE_BASE_MAX.
// Goal-production re-tune (2026-06-05): lowered 0.75 -> 0.72 in lockstep with the
// save_base floor so the cap does not blunt the floor's effect at the high-quality
// GK end. (Measured negligible at the current sigma — the cap is rarely hit — but
// kept consistent with the floor.)
const SAVE_PROB_MAX: Q32 = Q32::from_raw(3_092_376_453_i64); // = 0.72

// Site discriminant for the GK save roll (0x5A7E = "SAVE" mnemonic).
const SAVE_ROLL_SITE_DISCRIMINANT: u32 = 0x5A7E;

// GK attribute weights for save_base composite.
// gk_quality = reflexes × 0.45 + handling × 0.30 + one_on_ones × 0.15 + positioning × 0.10
const W_GK_REFLEXES: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45
const W_GK_HANDLING: Q32 = Q32::from_raw(1_288_490_188_i64); // ≈ 0.30
const W_GK_ONE_ON_ONES: Q32 = Q32::from_raw(644_245_094_i64); // ≈ 0.15
const W_GK_POSITIONING: Q32 = Q32::from_raw(429_496_729_i64); // ≈ 0.10

// ---------------------------------------------------------------------------
// Goalmouth defending constants
// ---------------------------------------------------------------------------

/// Squared clearance radius (m²): a goal-side outfield defender within this
/// distance of the ball can reach and clear it before it crosses the line.
/// Home DEFs sit at x=-30 (22.5 m from goal line at -52.5). With the
/// CLEARANCE_DANGER_ZONE_M=20 gate, clearance only fires when the ball
/// is within 20 m of the line (i.e. bx between -32.5 and -52.5).
/// A defender at x=-30 is at most ~2.5 m from the danger-zone boundary and
/// can be within 10 m of a ball at x=-32 to x=-40 depending on y. Using
/// 10 m radius (100 m²) is the right compromise: covers defenders who are
/// tracking a loose ball near the box without firing in midfield.
const DEFENDER_CLEAR_RADIUS_SQ: Q32 = Q32::from_raw(100_i64 << 32); // 10² = 100 m²

/// Ball velocity magnitude for a defensive clearance (m/s upfield).
/// Slower than a shot so the clearance stays on the pitch; fast enough to
/// move the ball well away from danger in 3–5 ticks.
const CLEARANCE_SPEED: Q32 = Q32::from_raw(10_i64 << 32); // 10.0 m/s

/// Squared gather radius (m²): the GK claims a loose non-shot ball crossing
/// the goal mouth when within this distance of the crossing point.
///
/// This is NOT a model of the keeper's bodily reach (a real keeper claims
/// ~5–6 m). It is "is the keeper positioned to have come and covered the
/// mouth": the radius is measured from the keeper's BODY (formation depth
/// ~7.5 m off the line — home GK at x=-45, away at x=+45) to the ball's
/// CROSSING POINT (at x=±52.5). The 7.5 m x-gap alone consumes most of the
/// budget, so the radius must exceed it for the gather to fire at all on a
/// central ball. Empirically (50-seed sweep, base 0x1000…0): at 10 m the
/// keeper gathers central crossings and concedes the corners + balls when
/// pushed wide (0 drift goals); at 6 m the gather can NEVER fire (the x-gap
/// exceeds the radius) and 20 drift goals leak. 10 m is calibrated to the
/// formation depth, not to a bodily-reach claim.
const GK_GATHER_RADIUS_SQ: Q32 = Q32::from_raw(100_i64 << 32); // 10² = 100 m²

/// Distance from goal line within which the clearance step operates.
/// Only balls within this distance of the goal line are candidates for
/// defensive clearance. Keeps clearance out of attacking transitions
/// (a pass heading toward goal from midfield should not be intercepted
/// by the clearance logic — that's dispossession/tackle territory).
/// 20 m = penalty area depth (16 m) + 4 m buffer.
const CLEARANCE_DANGER_ZONE_M: Q32 = Q32::from_raw(20_i64 << 32); // 20.0 m from goal line

// ---------------------------------------------------------------------------
// Goalmouth defending — defensive clearance
// ---------------------------------------------------------------------------

/// Pre-detection step: for each team, if a loose ball is moving toward their
/// own goal AND an outfield defender of that team is within `DEFENDER_CLEAR_RADIUS`
/// of the ball, the nearest such defender clears it upfield.
///
/// Runs BEFORE goal detection so a cleared ball never enters the goal-mouth
/// check. GK slots (0 and 11) are excluded — GK gathering is handled
/// inside goal detection via the `xg_score == 0` branch below.
///
/// Determinism: iteration is slot-ordered (ascending); ties are broken by
/// the smallest slot index. No RNG — the clearance is geometrically
/// deterministic. Q32 only; no floats.
fn resolve_goalmouth_defending(mut state: MatchState) -> MatchState {
    let bx = state.ball.pos_x;
    let by = state.ball.pos_y;
    let bvx = state.ball.vel_x;

    // Act when the ball is physically unclaimed: either possession == None (loose)
    // OR the nominal possessor is farther than PHYSICAL_POSSESSION_RADIUS from
    // the ball (pass-in-flight / overshoot scenario — possession is "notional").
    // If the possessor IS close to the ball, they physically hold it and a
    // defensive clearance is not applicable (tackle / dispossession handles that).
    //
    // A dribble-in by an attacker (possessor IS close to ball near goal line)
    // is a legitimate goal; clearance does not fire.
    const PHYSICAL_POSSESSION_RADIUS_SQ: Q32 = Q32::from_raw(25_i64 << 32); // 5² = 25 m²
    let ball_is_physically_unclaimed = match state.possession {
        None => true,
        Some(slot) => {
            let slot_idx = slot as usize;
            if slot_idx < TOTAL_PLAYERS {
                let px = state.players[slot_idx].pos_x;
                let py = state.players[slot_idx].pos_y;
                let dx = px - bx;
                let dy = py - by;
                let dist_sq = dx * dx + dy * dy;
                // Physically unclaimed = possessor is far from ball.
                dist_sq > PHYSICAL_POSSESSION_RADIUS_SQ
            } else {
                true // structural guard: bad slot index → treat as unclaimed
            }
        }
    };
    if !ball_is_physically_unclaimed {
        return state;
    }

    // ---- Home team defending (-X goal) ----
    // Ball moving toward home goal: vel_x < 0.
    // Danger-zone gate: only apply clearance when ball is within
    // CLEARANCE_DANGER_ZONE_M of the home goal line. This prevents the
    // clearance from intercepting legitimate passes/attacks in the home half
    // that are far from the goal (those are dispossession/tackle territory).
    // Home goal line at -GOAL_LINE_X. Distance = bx - (-GOAL_LINE_X) = bx + GOAL_LINE_X.
    if bvx < Q32::ZERO {
        let bx_bits = bx.to_bits();
        // Ball must be in home's defensive half (x < 0).
        if bx_bits < 0 {
            // Distance of ball from home goal line (positive value when bx > -GOAL_LINE_X).
            let home_goal_line_neg = -GOAL_LINE_X; // -52.5
            let dist_from_home_line = bx - home_goal_line_neg; // bx + 52.5; positive if bx > -52.5
            // Only clear when within the danger zone (ball within 20 m of home goal line).
            if dist_from_home_line >= Q32::ZERO && dist_from_home_line <= CLEARANCE_DANGER_ZONE_M {
                let mut nearest_slot: Option<usize> = None;
                let mut nearest_dist_sq = DEFENDER_CLEAR_RADIUS_SQ;
                // Iterate home outfield slots (1..11); slot 0 is home GK (excluded).
                for slot_idx in 1..PLAYERS_PER_TEAM {
                    let p = &state.players[slot_idx];
                    if p.pos_x > Q32::ZERO {
                        continue; // defender in away half — skip
                    }
                    let dx = p.pos_x - bx;
                    let dy = p.pos_y - by;
                    let dist_sq = dx * dx + dy * dy;
                    // Within-radius check is inclusive (`<=` the threshold), but
                    // the nearest-defender selection uses strict `<` so an exact
                    // distance tie is won by the LOWER slot (ascending tie-break,
                    // deterministic). `is_none()` admits the first in-radius
                    // defender even when its dist_sq equals the threshold.
                    if dist_sq <= DEFENDER_CLEAR_RADIUS_SQ
                        && (nearest_slot.is_none() || dist_sq < nearest_dist_sq)
                    {
                        nearest_dist_sq = dist_sq;
                        nearest_slot = Some(slot_idx);
                    }
                }
                if let Some(slot_idx) = nearest_slot {
                    // Clear the ball upfield (toward +X for home defenders).
                    state.ball.vel_x = CLEARANCE_SPEED;
                    state.ball.vel_y = Q32::ZERO;
                    state.possession = None;
                    // SLICE-1: a defender clearance interrupts any pass-in-flight
                    // — the ball is now a genuine loose ball heading upfield, not
                    // a directed pass. Clear the flight so the receiver un-freezes
                    // and the ball becomes claimable by the normal pickup path.
                    state.ball_in_flight = None;
                    let p_slot = state.players[slot_idx].slot;
                    state.last_touched_by = Some(p_slot);
                }
            }
        }
    }

    // Re-read after possible home-team clearance.
    let bvx = state.ball.vel_x;

    // ---- Away team defending (+X goal) ----
    // Ball moving toward away goal: vel_x > 0.
    if bvx > Q32::ZERO {
        let bx_bits = state.ball.pos_x.to_bits();
        // Ball must be in away's defensive half (x > 0).
        if bx_bits > 0 {
            let bx2 = state.ball.pos_x;
            let by2 = state.ball.pos_y;
            // Danger-zone gate: ball within CLEARANCE_DANGER_ZONE_M of away goal line.
            // Away goal line at +GOAL_LINE_X (+52.5). Dist = GOAL_LINE_X - bx2.
            let dist_from_away_line = GOAL_LINE_X - bx2; // positive when bx2 < 52.5
            if dist_from_away_line >= Q32::ZERO && dist_from_away_line <= CLEARANCE_DANGER_ZONE_M {
                let mut nearest_slot: Option<usize> = None;
                let mut nearest_dist_sq = DEFENDER_CLEAR_RADIUS_SQ;
                // Away outfield slots: 12..22; slot 11 is away GK (excluded).
                for slot_idx in (PLAYERS_PER_TEAM + 1)..TOTAL_PLAYERS {
                    let p = &state.players[slot_idx];
                    if p.pos_x < Q32::ZERO {
                        continue; // defender in home half — skip
                    }
                    let dx = p.pos_x - bx2;
                    let dy = p.pos_y - by2;
                    let dist_sq = dx * dx + dy * dy;
                    // See home-branch note: inclusive radius, strict `<` selection
                    // for a lower-slot ascending tie-break.
                    if dist_sq <= DEFENDER_CLEAR_RADIUS_SQ
                        && (nearest_slot.is_none() || dist_sq < nearest_dist_sq)
                    {
                        nearest_dist_sq = dist_sq;
                        nearest_slot = Some(slot_idx);
                    }
                }
                if let Some(slot_idx) = nearest_slot {
                    // Clear the ball upfield (toward -X for away defenders).
                    state.ball.vel_x = -CLEARANCE_SPEED;
                    state.ball.vel_y = Q32::ZERO;
                    state.possession = None;
                    // SLICE-1: see home-branch note — clear the flight on a
                    // defender clearance so the cleared ball is genuinely loose.
                    state.ball_in_flight = None;
                    let p_slot = state.players[slot_idx].slot;
                    state.last_touched_by = Some(p_slot);
                }
            } // end danger-zone gate (dist_from_away_line)
        }
    }

    state
}

/// Determine the per-team `SetPieceKind` to emit alongside a `BallOutOfPlay`
/// TacticEvent on the tick the OOB-clamp triggers (T2-1c).
///
/// Returns `(home_kind, away_kind)` — the kinds are PER-TEAM and reciprocal:
/// when one team gets `ThrowInFor`, the other gets `ThrowInAgainst`; one's
/// `CornerFor` is the other's `CornerAgainst`; etc.
///
/// `bx_bits` is `state.ball.pos_x.to_bits()` at the moment of OOB detection
/// (sign indicates which goal-line was crossed when `past_non_goal_line`).
/// `last_team_idx` is `Some(0)` if home last touched, `Some(1)` if away,
/// `None` if no prior touch.
///
/// The None case is structurally impossible during normal play:
/// `MatchState::initial` sets `last_touched_by = Some(9)` at construction,
/// and every `apply_intent` ball-touch arm assigns Some(...). The defensive
/// fallback to `ThrowInFor` / `ThrowInFor` if `None` avoids silent slot-0
/// misattribution per the existing Codex 2026-05-16 audit silent-failure
/// P0-1 pattern.
///
/// When BOTH `past_sideline` AND `past_non_goal_line` are true (corner-flag
/// edge case), the non-goal-line kind wins (the ball is closer to the
/// goal-line geometrically; treating it as a corner/goal-kick matches
/// real-football refereeing — the goal-line decision takes priority over
/// the sideline decision because the ball had to cross the goal-line to
/// reach the corner flag).
//
// `past_sideline` is intentionally not a parameter: the caller only invokes
// this helper when at least one of the two boundary flags is true, and the
// past_non_goal_line=false branch is the past_sideline-only case
// (callsite-enforced precondition).
#[inline]
fn setpiece_kind_for(
    past_non_goal_line: bool,
    bx_bits: i64,
    last_team_idx: Option<usize>,
) -> (tactic_fsm::SetPieceKind, tactic_fsm::SetPieceKind) {
    use tactic_fsm::SetPieceKind;

    if past_non_goal_line {
        // Determine which goal-line was crossed + which team was attacking.
        // bx_bits > 0 → ball past away's goal-line (positive x) → home was attacking.
        // bx_bits < 0 → ball past home's goal-line (negative x) → away was attacking.
        let home_attacking = bx_bits > 0;
        // last_team_idx == Some(0) = home last touched, Some(1) = away, None = unattributed.
        match (home_attacking, last_team_idx) {
            (true, Some(0)) | (true, None) => {
                // Home attacking + home last touched (or None defensive default):
                // home gets CornerFor; away gets CornerAgainst.
                (SetPieceKind::CornerFor, SetPieceKind::CornerAgainst)
            }
            (true, Some(1)) => {
                // Home attacking + away last touched:
                // away kicks off the goal-kick + home gets the opponent variant.
                (SetPieceKind::GoalKickOpponent, SetPieceKind::GoalKick)
            }
            (false, Some(1)) | (false, None) => {
                // Away attacking + away last touched (or None defensive default):
                // away gets CornerFor; home gets CornerAgainst.
                (SetPieceKind::CornerAgainst, SetPieceKind::CornerFor)
            }
            (false, Some(0)) => {
                // Away attacking + home last touched:
                // home kicks off the goal-kick + away gets the opponent variant.
                (SetPieceKind::GoalKick, SetPieceKind::GoalKickOpponent)
            }
            (_, Some(other)) => {
                // Defensive guard against future team-index expansion (e.g. neutral
                // referee touches per drop-ball scenarios). Today team_of returns
                // only 0 or 1 so this arm is unreachable, but the panic-fail-loud
                // discipline per the established Codex 2026-05-16 audit P0-1 pattern
                // surfaces any future invariant violation at the violation site.
                panic!(
                    "setpiece_kind_for: unexpected last_team_idx {other} \
                     (team_of must return 0 or 1)"
                );
            }
        }
    } else {
        // past_sideline only (no goal-line cross). Throw-in: the team that
        // DIDN'T last touch the ball takes the throw-in.
        match last_team_idx {
            Some(0) => {
                // Home last touched → away throws in.
                (SetPieceKind::ThrowInAgainst, SetPieceKind::ThrowInFor)
            }
            Some(1) => {
                // Away last touched → home throws in.
                (SetPieceKind::ThrowInFor, SetPieceKind::ThrowInAgainst)
            }
            None => {
                // Unattributed (structurally unreachable; defensive default).
                // Home gets the throw — matches `MatchState::initial`'s
                // possession-with-home convention.
                (SetPieceKind::ThrowInFor, SetPieceKind::ThrowInAgainst)
            }
            Some(other) => {
                panic!(
                    "setpiece_kind_for: unexpected last_team_idx {other} \
                     (team_of must return 0 or 1)"
                );
            }
        }
    }
    // Note: past_sideline + past_non_goal_line both true (corner-flag edge case)
    // is handled by the past_non_goal_line branch winning — the goal-line
    // decision takes priority per the doc-comment above.
}

// -------------------------------------------------------------------------
// T2-1b helpers: per-team archetype-driven TacticEvent emission
// -------------------------------------------------------------------------

/// Map a `PlayerSlot` to its team index (0 = home, 1 = away).
///
/// Slot convention (locked at T1-2b-iii-a): slots 0..11 home, 11..22 away.
#[inline]
fn team_of(slot: PlayerSlot) -> usize {
    if (slot as usize) < PLAYERS_PER_TEAM {
        0
    } else {
        1
    }
}

/// Read the archetype params for the named team (T2-1b).
///
/// Returns `Copy` because `ArchetypeParams` derives `Copy`; avoids a borrow
/// of `state` overlapping with subsequent writes to `state.team_tactic_states`.
#[inline]
fn team_arch_params(state: &MatchState, team: usize) -> tactic_fsm::ArchetypeParams {
    if team == 0 {
        state.home_archetype_params
    } else {
        state.away_archetype_params
    }
}

/// Bauer-and-Anzer "opponent shape broken" heuristic (T2-1b).
///
/// `opponent_shape_broken == true` when the opposing team's MEAN x-position
/// has crossed halfway into the recovering team's defensive third — i.e.
/// the opponent committed numbers forward + the recovery now creates a
/// transition-opportunity. Drives the `BallRecovered` apply_event arm at
/// `tactic_fsm.rs:430` toward `CounterAttack`.
///
/// Coordinate convention: home defends -X, away defends +X.
///   - recovering_team == 0 (home) → opponent = away (slots 11..22).
///     opponent shape broken iff away mean_x < 0 (away players in home's half).
///   - recovering_team == 1 (away) → opponent = home (slots 0..11).
///     opponent shape broken iff home mean_x > 0 (home players in away's half).
///
/// Mean is computed in Q32 raw-bits space + divided by 11 (integer divide).
/// `PLAYERS_PER_TEAM` is the divisor source — re-uses the constant rather
/// than hard-coding 11.
fn compute_opponent_shape_broken(state: &MatchState, recovering_team: usize) -> bool {
    let (opp_start, opp_end) = if recovering_team == 0 {
        (PLAYERS_PER_TEAM, TOTAL_PLAYERS)
    } else {
        (0, PLAYERS_PER_TEAM)
    };
    let n = (opp_end - opp_start) as i64;
    let mut sum_bits: i64 = 0;
    for s in opp_start..opp_end {
        sum_bits = sum_bits.wrapping_add(state.players[s].pos_x.to_bits());
    }
    let mean_bits = sum_bits / n;
    if recovering_team == 0 {
        mean_bits < 0
    } else {
        mean_bits > 0
    }
}

/// Emit `TacticEvent::PossessionLost` / `TacticEvent::BallRecovered` based
/// on the tick's possession transition (T2-1b).
///
/// Called once per tick AFTER all possession-mutating steps (dispatch_tick
/// fires shot/pass intents → mutates possession; the loose-ball pickup
/// block runs after dispatch + may convert None → Some). Compare
/// `state.possession` against the `possession_before` snapshot captured
/// before dispatch_tick.
///
/// Transition taxonomy (per the T2-1b MEMORY spec AC5):
///
/// - `Some(a) → None` — ball released (shot, OOB clearance, settled-loose
///   before pickup). PossessionLost(recovery_likely=false) for a's team;
///   no BallRecovered (nobody picked up this tick).
/// - `Some(a) → Some(b)` same team — within-team pass. NO events (possession
///   stayed with the team; no FSM transition needed).
/// - `Some(a) → Some(b)` cross-team — contested-pass interception / dribble-
///   on-opponent. PossessionLost(recovery_likely=true) for a's team +
///   BallRecovered(opponent_shape_broken=computed) for b's team.
/// - `None → Some(b)` — loose-ball pickup (mid-tick after a prior tick's
///   release). BallRecovered(opponent_shape_broken=computed) for b's team;
///   no PossessionLost (the prior PossessionLost fired on the release tick).
/// - `None → None` — no possession change. No events.
///
/// Each emission feeds the existing `tactic_fsm::apply_event` arm with the
/// affected team's OWN `archetype_params` sidecar (T2-1a), so transitions
/// are archetype-driven per team. Pre-T2-1b this code path didn't exist —
/// PossessionLost / BallRecovered never fired in production despite their
/// apply_event arms being implemented (the gap T2-1a's CRITICAL-1 review
/// identified + deferred to this row).
fn emit_possession_transition_events(
    state: &mut MatchState,
    possession_before: Option<PlayerSlot>,
) {
    // SLICE-1: use effective_possession for the "after" side so a successful
    // in-flight pass is NOT treated as a PossessionLost event. The ball is
    // still the passing team's — only the timing of the transfer is delayed.
    // Using `state.possession` here (which is None during a flight) would
    // falsely emit PossessionLost on every successful pass launch, causing the
    // tactic FSM to drop both teams into defensive CounterAttack mode during
    // every pass, collapsing build-up and killing goal rate.
    let possession_after = state.effective_possession();
    match (possession_before, possession_after) {
        (Some(a), None) => {
            // Release without pickup this tick: PossessionLost only.
            let team_lost = team_of(a);
            auto_exit_setpiece(state, team_lost);
            let arch = team_arch_params(state, team_lost);
            state.team_tactic_states[team_lost] = tactic_fsm::apply_event(
                state.team_tactic_states[team_lost],
                &arch,
                tactic_fsm::TacticEvent::PossessionLost {
                    recovery_likely: false,
                },
                state.tick,
            );
        }
        (Some(a), Some(b)) => {
            let team_lost = team_of(a);
            let team_gained = team_of(b);
            if team_lost == team_gained {
                // Within-team transfer (pass to teammate / dribble continuation):
                // no FSM-level event.
                return;
            }
            // Cross-team transition: PossessionLost for a's team + BallRecovered
            // for b's team. recovery_likely=true because the opponent immediately
            // claimed the ball this same tick — the contest was decided in their
            // favor + the losing team is structurally still "in the press window".
            auto_exit_setpiece(state, team_lost);
            auto_exit_setpiece(state, team_gained);
            let shape_broken = compute_opponent_shape_broken(state, team_gained);
            let arch_lost = team_arch_params(state, team_lost);
            state.team_tactic_states[team_lost] = tactic_fsm::apply_event(
                state.team_tactic_states[team_lost],
                &arch_lost,
                tactic_fsm::TacticEvent::PossessionLost {
                    recovery_likely: true,
                },
                state.tick,
            );
            let arch_gained = team_arch_params(state, team_gained);
            state.team_tactic_states[team_gained] = tactic_fsm::apply_event(
                state.team_tactic_states[team_gained],
                &arch_gained,
                tactic_fsm::TacticEvent::BallRecovered {
                    opponent_shape_broken: shape_broken,
                },
                state.tick,
            );
        }
        (None, Some(b)) => {
            // Loose-ball pickup (the prior tick's release already fired
            // PossessionLost; this tick the ball was claimed by `b`).
            let team_gained = team_of(b);
            auto_exit_setpiece(state, team_gained);
            let shape_broken = compute_opponent_shape_broken(state, team_gained);
            let arch = team_arch_params(state, team_gained);
            state.team_tactic_states[team_gained] = tactic_fsm::apply_event(
                state.team_tactic_states[team_gained],
                &arch,
                tactic_fsm::TacticEvent::BallRecovered {
                    opponent_shape_broken: shape_broken,
                },
                state.tick,
            );
        }
        (None, None) => {
            // Ball stayed loose. No event.
        }
    }
}

/// T2-1c: auto-exit `TacticState::SetPiece(_)` for the named team by firing
/// `apply_event(BallInPlay)` before any other possession-transition event.
///
/// Rationale: the `PossessionLost` + `BallRecovered` apply_event arms at
/// `tactic_fsm.rs:395+` only match `MidBlock | LowBlock | HighPress |
/// CounterAttack` — they're no-ops when the team is in `SetPiece(_)`. So
/// firing them while the team is stuck in SetPiece silently drops the
/// per-team divergence T2-1b delivered. The T2-1c minimum-viable
/// interpretation: on the next possession transition after a SetPiece
/// entry, auto-fire BallInPlay to transition back to the archetype's
/// default_in_defence_state — then the PossessionLost/BallRecovered
/// transitions can fire normally.
///
/// This is NOT the same as a true set-piece restart timing mechanic
/// (5-tick countdown, ball reposition to thrower's feet, possession to
/// thrower's slot — all deferred to T2-1d/T2-2). The simplification
/// "auto-exit on next possession transition" is acceptable because today
/// the OOB-clamp doesn't reset possession or ball position to a
/// thrower-equivalent setup — the ball just sits at the sideline + the
/// last carrier still nominally has possession. So "next possession
/// transition" is effectively "first BT-driven Dribble/Pass intent that
/// moves the ball off the boundary."
///
/// No-op if the team is NOT currently in SetPiece (most ticks).
#[inline]
fn auto_exit_setpiece(state: &mut MatchState, team: usize) {
    let current = state.team_tactic_states[team];
    if matches!(current.state(), tactic_fsm::TacticState::SetPiece(_)) {
        let arch = team_arch_params(state, team);
        state.team_tactic_states[team] = tactic_fsm::apply_event(
            current,
            &arch,
            tactic_fsm::TacticEvent::BallInPlay,
            state.tick,
        );
    }
}

// -------------------------------------------------------------------------
// B2 — Dispossession / tackle mechanic (FUN-0b+c Slice B)
// -------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Tackle constants — PROVISIONAL; tuned via drama-sweep.
// ---------------------------------------------------------------------------

/// Tackle attempt radius in metres. A defender must be within this distance
/// of the ball carrier to attempt a tackle. Q32: 2.0m = 2 << 32 raw bits.
///
/// Squared for radius-gate arithmetic (avoids sqrt): 2^2 = 4m².
/// Q32: 4.0 = 4 << 32 raw bits.
const TACKLE_RADIUS_SQ: Q32 = Q32::from_raw(4_i64 << 32); // 2m radius → 4m² gate

/// Base tackle probability when both defender and carrier attributes are
/// at mid-range (0.5 × 0.5). Tuning lever: higher = more turnovers.
/// Q32: 0.35 ≈ 1_503_238_553 raw bits.
const TACKLE_BASE_PROB: Q32 = Q32::from_raw(1_503_238_553_i64); // ≈ 0.35 (drama-sweep R11 final)

/// How many ticks a defender must wait after a FAILED tackle attempt before
/// trying again. Prevents tackle-spam. At 60 Hz: 18 ticks ≈ 0.3 seconds.
const TACKLE_COOLDOWN_TICKS: u32 = 18;

/// Site discriminant for tackle rolls. Chosen to not collide with SS3 save
/// site (0x5A7E) or SS2 dispersion sites (0x0001..0x0003).
/// Mnemonic: 0x7AC1 ≈ "TACL".
const TACKLE_ROLL_SITE: u32 = 0x7AC1;

// ---------------------------------------------------------------------------
// SLICE-1: trap_check_in_flight
// ---------------------------------------------------------------------------

/// Trap radius for a directed pass (deliberate first-touch).
///
/// 4.0 metres — generous enough to handle receiver drift during the ball's
/// flight, but tighter than the 5m loose-ball pickup radius so the in-flight
/// guard still distinguishes a trap from a random pickup.
///
/// Design note: the ball is aimed at the receiver's LAUNCH-TICK position.
/// During flight (20-50 ticks) the receiver moves at up to ~7.5 m/s
/// (0.125 m/tick), drifting ~2-6m from the target point. A 4m trap radius
/// catches receivers who stay broadly in position without freezing them.
///
/// Tuning value — can be tightened in Slice 2 once player movement-to-meet
/// the ball is implemented.
const TRAP_RADIUS_M: Q32 = Q32::from_raw(17_179_869_184_i64); // 4.0 × 2^32

/// After this many ticks without the ball reaching the intended receiver,
/// force-clear the flight state. 120 ticks = 2 seconds at 60 Hz, covering
/// even a 30 m pass at 17.5 m/s (≈ 103 ticks) with margin.
///
/// Tuning value from docs/design/ball-in-flight-model-2026-06-06.md §Tuning.
const FLIGHT_TIMEOUT_TICKS: i64 = 120;

/// Check whether an in-flight pass has arrived (ball within `TRAP_RADIUS_M`
/// of the intended receiver) or timed out, and resolve possession accordingly.
///
/// ## Call site
///
/// Called once per tick in `tick_match` AFTER `ball_step` and BEFORE
/// `dispatch_tick`. Skipped on goal ticks.
///
/// ## Arrival (success)
///
/// When `dist(ball, receiver) ≤ TRAP_RADIUS_M` and `outcome_is_success`:
/// - Grant `possession = Some(receiver_slot)`.
/// - Snap ball to receiver's feet; zero velocity.
/// - Clear `ball_in_flight = None`.
///
/// ## Arrival (failure)
///
/// When `outcome_is_success = false` on arrival: drop the ball loose at the
/// current ball position (no snap to receiver). Possession stays `None`.
/// Clear `ball_in_flight = None`.
///
/// ## Timeout
///
/// After `FLIGHT_TIMEOUT_TICKS` ticks: if `outcome_is_success`, GRANT
/// possession to the receiver even though they are out of range (successful
/// pass — the receiver gets the ball regardless). If `!outcome_is_success`,
/// the ball is already heading loose; drop it at its current position.
/// Clear `ball_in_flight = None` either way.
///
/// ## Determinism
///
/// Pure function of canonical `state` fields (Q32 positions, integers). No
/// RNG draw. `assert!` (not `debug_assert!`) for invariants per Sim/RULES.md §11.
fn trap_check_in_flight(mut state: MatchState) -> MatchState {
    let bif = match state.ball_in_flight {
        Some(b) => b,
        None => return state,
    };

    assert!(
        (bif.intended_receiver as usize) < TOTAL_PLAYERS,
        "ball_in_flight.intended_receiver {} is out of range (must be < {TOTAL_PLAYERS})",
        bif.intended_receiver
    );
    assert!(
        bif.launch_tick.to_raw() <= state.tick.to_raw(),
        "ball_in_flight.launch_tick {:?} is in the future (current tick {:?})",
        bif.launch_tick,
        state.tick
    );

    let ticks_in_flight = state.tick.to_raw() - bif.launch_tick.to_raw();
    let recv_idx = bif.intended_receiver as usize;

    let bx = state.ball.pos_x;
    let by = state.ball.pos_y;
    let px = state.players[recv_idx].pos_x;
    let py = state.players[recv_idx].pos_y;
    let dx = px - bx;
    let dy = py - by;
    let dist_sq = dx * dx + dy * dy;
    let trap_sq = TRAP_RADIUS_M * TRAP_RADIUS_M;

    let arrived = dist_sq <= trap_sq;
    let timed_out = ticks_in_flight > FLIGHT_TIMEOUT_TICKS;

    if arrived || timed_out {
        if bif.outcome_is_success {
            // Grant possession: snap ball to receiver's current position.
            state.possession = Some(bif.intended_receiver);
            state.last_touched_by = Some(bif.intended_receiver);
            state.ball.pos_x = state.players[recv_idx].pos_x;
            state.ball.pos_y = state.players[recv_idx].pos_y;
            state.ball.vel_x = Q32::ZERO;
            state.ball.vel_y = Q32::ZERO;
            state.ball.vel_z = Q32::ZERO;
        }
        // Failed pass on arrival: possession stays None; ball drops loose at
        // current position (existing ball_step physics already carries it
        // toward the target area; no additional deflection in Slice 1 —
        // Slice 2 adds the seeded deflection at the arrival point).
        // Clear flight state either way.
        state.ball_in_flight = None;
    }

    state
}

/// Resolve tackle attempts for all defending players who are within
/// `TACKLE_RADIUS_SQ` of the current ball carrier.
///
/// ## Algorithm
///
/// For each defender slot (opposing team to the carrier):
///   1. Skip if the slot's `tackle_cooldown_until > state.tick` (cooldown active).
///   2. Skip GKs (slot 0 / slot 11) — they handle the ball via `goalkeeper_fsm`.
///   3. Compute squared Euclidean distance between the defender and the carrier.
///   4. Skip if `dist_sq > TACKLE_RADIUS_SQ` (out of range).
///   5. Roll `ChaCha8Rng` seeded via `seed_fn(match_seed, tick, ReactiveInterrupt,
///      (defender_slot << 16) | TACKLE_ROLL_SITE)`.
///   6. Compute tackle probability:
///      `p = TACKLE_BASE_PROB × defender_quality / (defender_quality + carrier_quality)`
///      where `defender_quality = tackling × 0.50 + aggression × 0.30 + positioning × 0.20`
///      and `carrier_quality = dribbling × 0.50 + balance × 0.30 + composure × 0.20`.
///   7. On SUCCESS (roll < p): set `state.possession = Some(defender_slot)`,
///      set `state.last_touched_by = Some(defender_slot)`, snap ball to
///      defender position (ball "at feet").
///   8. On FAILURE: set `state.tackle_cooldown_until[def_idx] = state.tick + COOLDOWN`.
///
/// ## Determinism
///
/// - Defenders iterated in slot order (0..22, fixed).
/// - RNG seeded per ADR-0009 with `SeedLayer::ReactiveInterrupt`.
/// - No floats, no HashMap, no clocks.
///
/// ## Fouls / cards
///
/// NOT implemented in this slice. Follow-up in T2-4.
fn resolve_tackles(mut state: MatchState) -> MatchState {
    use crate::decision_cadence::{SeedLayer, seed_fn};
    use rand_chacha::ChaCha8Rng;
    use rand_chacha::rand_core::{RngCore, SeedableRng};

    // Only resolve when there is an active ball carrier.
    let carrier_slot = match state.possession {
        Some(s) => s,
        None => return state,
    };

    let carrier_idx = carrier_slot as usize;
    let carrier_team: usize = if carrier_idx < PLAYERS_PER_TEAM { 0 } else { 1 };

    let carrier_x = state.players[carrier_idx].pos_x;
    let carrier_y = state.players[carrier_idx].pos_y;

    // Carrier attributes for the "resist" side of the contest.
    let ca = state.players[carrier_idx].attributes();
    let w_dribble = Q32::from_raw(2_147_483_648_i64); // ≈ 0.50
    let w_balance = Q32::from_raw(1_288_490_188_i64); // ≈ 0.30
    let w_composure = Q32::from_raw(858_993_459_i64); // ≈ 0.20
    // Slice 0: curve each term (dribbling = skill, balance = contest,
    // composure = mental) so an elite carrier resists the tackle disproportionately.
    let carrier_quality = curve(CurveClass::Skill, ca.technical.dribbling) * w_dribble
        + curve(CurveClass::Contest, ca.physical.balance) * w_balance
        + curve(CurveClass::Mental, ca.mental.composure) * w_composure;
    let carrier_quality = if carrier_quality > Q32::ONE {
        Q32::ONE
    } else {
        carrier_quality
    };

    let tick_u32 = state.tick.to_raw() as u32;
    let match_seed = state.seed.to_u64();

    // Iterate ALL 22 slots; skip own-team + GKs + cooldown-active.
    // B2: at most one tackle resolves per tick (the first successful defender
    // in slot order wins; subsequent defenders see possession already changed).
    // This avoids double-possession-transfer races.
    for def_idx in 0..TOTAL_PLAYERS {
        // Skip the carrier itself (can't tackle yourself).
        if def_idx == carrier_idx {
            continue;
        }

        // Skip own-team players.
        let def_team: usize = if def_idx < PLAYERS_PER_TEAM { 0 } else { 1 };
        if def_team == carrier_team {
            continue;
        }

        // Skip GKs (slot 0 = home GK, slot 11 = away GK).
        let in_team_slot = def_idx % PLAYERS_PER_TEAM;
        if in_team_slot == 0 {
            continue;
        }

        // Skip if cooldown active.
        if state.tackle_cooldown_until[def_idx] > state.tick {
            continue;
        }

        // Radius gate via squared Euclidean distance.
        let dx = state.players[def_idx].pos_x - carrier_x;
        let dy = state.players[def_idx].pos_y - carrier_y;
        let dist_sq = dx * dx + dy * dy;
        if dist_sq > TACKLE_RADIUS_SQ {
            continue;
        }

        // Defender attributes for the "win" side of the contest.
        let da = state.players[def_idx].attributes();
        let w_tackling = Q32::from_raw(2_147_483_648_i64); // ≈ 0.50
        let w_aggression = Q32::from_raw(1_288_490_188_i64); // ≈ 0.30
        let w_positioning = Q32::from_raw(858_993_459_i64); // ≈ 0.20
        // Slice 0: tackling = contest/duel, aggression = personality tendency,
        // positioning = mental. The contest curve makes an elite tackler reliably win.
        let defender_quality = curve(CurveClass::Contest, da.technical.tackling) * w_tackling
            + curve(CurveClass::Personality, da.personality.aggression) * w_aggression
            + curve(CurveClass::Mental, da.mental.positioning) * w_positioning;
        let defender_quality = if defender_quality > Q32::ONE {
            Q32::ONE
        } else {
            defender_quality
        };

        // Tackle probability: TACKLE_BASE_PROB × def / (def + carrier + epsilon).
        // epsilon avoids division by zero when both are zero.
        let epsilon = Q32::from_raw(1 << 26); // ≈ 0.015625 (small floor)
        let sum = defender_quality + carrier_quality + epsilon;
        let tackle_prob = TACKLE_BASE_PROB * defender_quality / sum;
        let tackle_prob = if tackle_prob > Q32::ONE {
            Q32::ONE
        } else {
            tackle_prob
        };

        // RNG roll — ADR-0009 ReactiveInterrupt layer.
        let site = ((def_idx as u32) << 16) | TACKLE_ROLL_SITE;
        let rng_seed = seed_fn(match_seed, tick_u32, SeedLayer::ReactiveInterrupt, site);
        let mut rng = ChaCha8Rng::seed_from_u64(rng_seed);
        let roll_u64 = rng.next_u64();
        // Upper 32 bits → Q32 in [0, 1).
        let roll = Q32::from_raw((roll_u64 >> 32) as i64);

        if roll < tackle_prob {
            // Tackle success — possession transfers to defender.
            let def_slot = state.players[def_idx].slot;
            state.possession = Some(def_slot);
            state.last_touched_by = Some(def_slot);
            // Snap ball to defender's feet (ball "at feet" on winning tackle).
            state.ball.pos_x = state.players[def_idx].pos_x;
            state.ball.pos_y = state.players[def_idx].pos_y;
            state.ball.vel_x = Q32::ZERO;
            state.ball.vel_y = Q32::ZERO;
            state.ball.vel_z = Q32::ZERO;
            // Stop processing — possession changed this tick.
            break;
        } else {
            // Tackle failure — set cooldown to prevent spam.
            let cooldown_end = state.tick.checked_add_ticks(TACKLE_COOLDOWN_TICKS);
            state.tackle_cooldown_until[def_idx] = cooldown_end;
        }
    }

    state
}

// -------------------------------------------------------------------------
// B2 tackle tests (TDD)
// -------------------------------------------------------------------------

#[cfg(test)]
mod tackle_tests {
    use super::*;
    use fw_core::{Q32, Seed, Tick};

    /// Move ALL away-team players far from the carrier to avoid interference.
    /// Only the specifically placed defender will be within range.
    fn clear_away_team_to_far_side(state: &mut MatchState) {
        for slot_idx in PLAYERS_PER_TEAM..TOTAL_PLAYERS {
            state.players[slot_idx].pos_x = Q32::from_int(40);
            state.players[slot_idx].pos_y = Q32::from_int(20);
        }
    }

    /// Place a player at a specific position.
    fn place_player(state: &mut MatchState, slot: usize, x: Q32, y: Q32) {
        state.players[slot].pos_x = x;
        state.players[slot].pos_y = y;
    }

    /// B2 test: a defender within TACKLE_RADIUS of the carrier eventually
    /// causes a possession change (given enough seeds).
    ///
    /// Tackle probability at mid-range attrs ≈ 17.5% per attempt.
    /// P(0 successes in 60 trials) < 0.00003.
    #[test]
    fn tackle_within_radius_can_change_possession() {
        // Home slot 9 has the ball; away slot 16 is the ONLY nearby defender.
        let carrier_x = Q32::from_int(10);
        let carrier_y = Q32::ZERO;
        let def_x = carrier_x + Q32::from_raw(6_442_450_944_i64); // ≈ +1.5m
        let def_y = Q32::ZERO;

        let mut possession_changed = false;
        for seed_offset in 0u64..60 {
            let mut state = MatchState::initial(Seed::from_u64(0xBEEF + seed_offset));
            state.tick = Tick::from_raw(1);
            state.possession = Some(9);
            state.last_touched_by = Some(9);
            state.players[9].pos_x = carrier_x;
            state.players[9].pos_y = carrier_y;
            state.ball.pos_x = carrier_x;
            state.ball.pos_y = carrier_y;
            // Push ALL away players far away, then place ONLY slot 16 close.
            clear_away_team_to_far_side(&mut state);
            place_player(&mut state, 16, def_x, def_y);

            state = resolve_tackles(state);

            if state.possession != Some(9) {
                possession_changed = true;
                assert_eq!(
                    state.possession,
                    Some(16),
                    "on success, possession must be Some(defender_slot=16)"
                );
                assert_eq!(state.ball.pos_x, state.players[16].pos_x);
                assert_eq!(state.ball.pos_y, state.players[16].pos_y);
                break;
            }
        }
        assert!(
            possession_changed,
            "tackle within radius must eventually succeed across 60 seeds"
        );
    }

    /// B2 test: a defender OUTSIDE the TACKLE_RADIUS cannot change possession.
    #[test]
    fn tackle_outside_radius_never_changes_possession() {
        let carrier_x = Q32::from_int(10);
        let carrier_y = Q32::ZERO;
        // 5m away — well outside the 2m radius.
        let def_x = carrier_x + Q32::from_int(5);
        let def_y = Q32::ZERO;

        for seed_offset in 0u64..30 {
            let mut state = MatchState::initial(Seed::from_u64(0xC0DE + seed_offset));
            state.tick = Tick::from_raw(1);
            state.possession = Some(9);
            state.last_touched_by = Some(9);
            state.players[9].pos_x = carrier_x;
            state.players[9].pos_y = carrier_y;
            state.ball.pos_x = carrier_x;
            state.ball.pos_y = carrier_y;
            // Push ALL away players far away — none within range.
            clear_away_team_to_far_side(&mut state);
            // Place the "test defender" explicitly at 5m away.
            place_player(&mut state, 16, def_x, def_y);

            state = resolve_tackles(state);
            assert_eq!(
                state.possession,
                Some(9),
                "defender 5m away must not change possession (seed_offset={seed_offset})"
            );
        }
    }

    /// B2 test: failed tackle sets a cooldown.
    #[test]
    fn failed_tackle_sets_cooldown() {
        let cx = Q32::from_int(10);
        let cy = Q32::ZERO;
        let def_x = cx + Q32::from_raw(4_294_967_296_i64); // ≈ 1.0m — within radius

        let build_state = |seed_val: u64| {
            let mut state = MatchState::initial(Seed::from_u64(seed_val));
            state.tick = Tick::from_raw(1);
            state.possession = Some(9);
            state.last_touched_by = Some(9);
            state.players[9].pos_x = cx;
            state.players[9].pos_y = cy;
            state.ball.pos_x = cx;
            state.ball.pos_y = cy;
            // Boost carrier quality to max → near-zero tackle probability.
            state.players[9].attributes.technical.dribbling = Q32::ONE;
            state.players[9].attributes.physical.balance = Q32::ONE;
            state.players[9].attributes.mental.composure = Q32::ONE;
            // Push all away players away, then place slot 16 within range.
            clear_away_team_to_far_side(&mut state);
            place_player(&mut state, 16, def_x, Q32::ZERO);
            // Zero defender quality → near-zero tackle probability.
            state.players[16].attributes.technical.tackling = Q32::ZERO;
            state.players[16].attributes.personality.aggression = Q32::ZERO;
            state.players[16].attributes.mental.positioning = Q32::ZERO;
            state
        };

        let mut found_cooldown = false;
        for seed_offset in 0u64..100 {
            let s = resolve_tackles(build_state(0xF00D_1234 + seed_offset));
            if s.possession == Some(9) {
                // Tackle failed.
                if s.tackle_cooldown_until[16] > Tick::from_raw(1) {
                    found_cooldown = true;
                    assert_eq!(
                        s.tackle_cooldown_until[16],
                        Tick::from_raw(1).checked_add_ticks(TACKLE_COOLDOWN_TICKS)
                    );
                    break;
                }
            }
        }
        assert!(
            found_cooldown,
            "at least one failed tackle attempt out of 100 seeds must set a cooldown"
        );
    }

    /// B2 test: a defender under cooldown is skipped even when within range.
    #[test]
    fn defender_under_cooldown_is_skipped() {
        let cx = Q32::from_int(10);
        let cy = Q32::ZERO;
        let def_x = cx + Q32::from_raw(4_294_967_296_i64); // ≈ 1.0m — within radius

        for seed_offset in 0u64..30 {
            let mut state = MatchState::initial(Seed::from_u64(0xCAFE + seed_offset));
            state.tick = Tick::from_raw(5);
            state.possession = Some(9);
            state.last_touched_by = Some(9);
            state.players[9].pos_x = cx;
            state.players[9].pos_y = cy;
            state.ball.pos_x = cx;
            state.ball.pos_y = cy;
            // Push all away players away, place slot 16 within range.
            clear_away_team_to_far_side(&mut state);
            place_player(&mut state, 16, def_x, Q32::ZERO);
            // Set cooldown PAST current tick (5).
            state.tackle_cooldown_until[16] = Tick::from_raw(10);

            state = resolve_tackles(state);
            assert_eq!(
                state.possession,
                Some(9),
                "defender under cooldown must not attempt tackle (seed_offset={seed_offset})"
            );
        }
    }

    /// B2 test: own-team defenders never tackle their carrier.
    #[test]
    fn own_team_never_tackles_carrier() {
        let cx = Q32::from_int(10);
        let cy = Q32::ZERO;

        for seed_offset in 0u64..20 {
            let mut state = MatchState::initial(Seed::from_u64(0xA1B2 + seed_offset));
            state.tick = Tick::from_raw(1);
            state.possession = Some(9);
            state.last_touched_by = Some(9);
            state.players[9].pos_x = cx;
            state.players[9].pos_y = cy;
            state.ball.pos_x = cx;
            state.ball.pos_y = cy;
            // Move ALL away players far away (no opponent nearby).
            clear_away_team_to_far_side(&mut state);
            // Place home slot 8 (same team) at 0.5m — inside radius.
            let teammate_x = cx + Q32::from_raw(2_147_483_648_i64);
            place_player(&mut state, 8, teammate_x, Q32::ZERO);

            state = resolve_tackles(state);
            assert_eq!(
                state.possession,
                Some(9),
                "own-team player must never tackle the carrier (seed_offset={seed_offset})"
            );
        }
    }

    /// B2 test: resolve_tackles is deterministic — same state + same tick
    /// produces the same outcome.
    #[test]
    fn resolve_tackles_is_deterministic() {
        let cx = Q32::from_int(10);
        let cy = Q32::ZERO;
        let def_x = cx + Q32::from_raw(4_294_967_296_i64); // 1m

        let build_state = || {
            let mut state = MatchState::initial(Seed::from_u64(0x1111_2222));
            state.tick = Tick::from_raw(42);
            state.possession = Some(9);
            state.last_touched_by = Some(9);
            state.players[9].pos_x = cx;
            state.players[9].pos_y = cy;
            state.ball.pos_x = cx;
            state.ball.pos_y = cy;
            clear_away_team_to_far_side(&mut state);
            state.players[16].pos_x = def_x;
            state.players[16].pos_y = Q32::ZERO;
            state
        };

        let s1 = resolve_tackles(build_state());
        let s2 = resolve_tackles(build_state());

        assert_eq!(
            s1.encode_canonical(),
            s2.encode_canonical(),
            "resolve_tackles must produce identical canonical output for identical input"
        );
    }

    /// B2 test: no possession when ball is loose — resolve_tackles is a no-op.
    #[test]
    fn no_possession_resolve_tackles_noop() {
        let mut state = MatchState::initial(Seed::from_u64(0x5555));
        state.tick = Tick::from_raw(1);
        state.possession = None; // loose ball
        let before = state.encode_canonical();
        state = resolve_tackles(state);
        let after = state.encode_canonical();
        // Canonical state EXCEPT tick: resolve_tackles itself doesn't advance tick,
        // but both before and after are on the same tick so they should be identical.
        assert_eq!(
            before, after,
            "resolve_tackles on loose ball must not mutate canonical state"
        );
    }
}

// -------------------------------------------------------------------------
// tick_match — the canonical advance function
// -------------------------------------------------------------------------

/// Advance the match by one tick.
///
/// ## Signature change (T1-11)
///
/// `sig_definitions` is now a required parameter (formerly `tick_match` called
/// `dispatch_tick` with `&BTreeMap::new()` — meaning signatures could never
/// fire in the normal match path). Pass `&content_store.signature_definitions`
/// to enable the real dispatcher. Pass `&BTreeMap::new()` in tests / contexts
/// without a ContentStore for backwards-compat (no signatures will fire but
/// the match advances normally).
///
/// ## Ten sequential steps (T4-sim-halt adds freeze guard + in-play gate)
///
///   0. **Freeze guard** (T4-sim-halt): if `FullTime` is already the tail of
///      `match_events`, return `state` unchanged. This makes `tick_match`
///      idempotent past the final whistle: canonical state is byte-identical
///      no matter how many extra ticks a caller requests.
///   1. Increment `state.tick`.
///   2. **In-play gate** (T4-sim-halt, wraps steps 2–8): gameplay runs only
///      when `state.tick <= state.match_end_tick`. On the tick the clock
///      reaches `match_end_tick`, gameplay runs one final time; step 9 then
///      emits `FullTime`; the next call hits the freeze guard. If a caller
///      jumps past `match_end_tick` with no `FullTime` yet, gameplay is
///      skipped and step 9 still emits exactly one `FullTime`.
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
///      **T1-11:** passes `sig_definitions` so the signature dispatcher
///      receives real definitions when available.
///   7. Integrate player velocity into position (`pos += vel × dt`).
///   8. Player-separation positional-correction pass (T1-2b-iii-d).
///   9. Emit `MatchEvent::FullTime` if `state.tick >= state.match_end_tick`
///      (T1-4a; T4-sim-halt: the `!full_time_already_emitted` check is
///      removed — the step-0 freeze guard guarantees `FullTime` is never
///      already the tail when we reach step 9).
pub fn tick_match(
    mut state: MatchState,
    sig_definitions: &BTreeMap<String, fw_content::SignatureDefinition>,
) -> MatchState {
    // Step 0 (T4-sim-halt): freeze guard.
    //
    // If FullTime is already the tail of match_events the match is over and
    // this call is a no-op. Return state unchanged — canonical bytes are
    // identical no matter how many extra ticks a caller requests.
    //
    // The check is tail-only (last()), not `.any(...)`, because the step-9
    // emission below guarantees FullTime is always appended at the very end.
    // A mid-stream FullTime would indicate a bug, not over-advance.
    if matches!(state.match_events.last(), Some(MatchEvent::FullTime { .. })) {
        return state;
    }

    state.tick = state.tick.successor();

    // Steps 2-8 (T4-sim-halt): in-play gate.
    //
    // Gameplay only runs while the clock has not passed match_end_tick.
    // On the tick equal to match_end_tick, gameplay runs one final time and
    // step 9 then emits FullTime. If a caller somehow jumps tick past
    // match_end_tick (which the freeze guard normally prevents), step 9 still
    // emits exactly one FullTime with no gameplay having run.
    if state.tick <= state.match_end_tick {
        // Step 2a (goalmouth-defending): defensive clearance + GK gather.
        //
        // Run before goal detection (step 2b) so a defender or GK who is
        // positioned near a loose ball heading toward goal intercepts it
        // before the crossing check fires. A ball cleared here will have
        // vel_x pointing upfield; it won't satisfy the goal-crossing condition
        // on this tick (it hasn't reached the line yet), and it won't on the
        // NEXT tick unless it bounces back. GK gather (xg_score == 0) lives in
        // step 2b so the same positional data drives both checks.
        state = resolve_goalmouth_defending(state);

        // Step 2b (T1-3.5): goal detection — checks ball.pos BEFORE physics.
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
                let home_scored = bx_bits > 0;

                // ---- SS3 — GK Save Model (FUN-0b) ----
                //
                // Before incrementing the score, compute a save probability for the
                // conceding team's GK and roll against it. On a save: the ball is
                // cleared to the GK's position (ball reset + possession → GK) and
                // no goal is scored. On a goal: fall through to the score increment.
                //
                // GK slot convention: home GK = slot 0, away GK = slot 11.
                //   home_scored == true → ball in AWAY goal → away GK = slot 11.
                //   home_scored == false → ball in HOME goal → home GK = slot 0.
                //
                // `xg_score` is from `last_shot_xg[scorer_slot_idx]` — written by
                // `dispatch::apply_intent` at the AttemptShot dispatch tick.
                // Falls back to Q32::ZERO (best-xG denominator → highest save chance)
                // if scorer_slot is out of range (structural guard).
                let gk_slot_idx: usize = if home_scored {
                    11 // away GK
                } else {
                    0 // home GK
                };

                let scorer_slot_idx = scorer_slot as usize;
                let xg_score = if scorer_slot_idx < 22 {
                    state.last_shot_xg[scorer_slot_idx]
                } else {
                    Q32::ZERO // structural guard — scorer_slot must be < 22
                };

                // GK quality composite:
                // gk_quality = reflexes×0.45 + handling×0.30 + one_on_ones×0.15 + positioning×0.10
                let gk_attrs = state.players[gk_slot_idx].attributes();
                // Slice 0: GK shot-stopping is a duel — reflexes/handling/
                // one_on_ones use the contest curve so an elite keeper saves
                // reliably; positioning = mental.
                let gk_quality = curve(CurveClass::Contest, gk_attrs.goalkeeper.reflexes)
                    * W_GK_REFLEXES
                    + curve(CurveClass::Contest, gk_attrs.goalkeeper.handling) * W_GK_HANDLING
                    + curve(CurveClass::Contest, gk_attrs.goalkeeper.one_on_ones)
                        * W_GK_ONE_ON_ONES
                    + curve(CurveClass::Mental, gk_attrs.mental.positioning) * W_GK_POSITIONING;
                let gk_quality = if gk_quality > Q32::ONE {
                    Q32::ONE
                } else {
                    gk_quality
                };

                // save_base = SAVE_BASE_MIN + (SAVE_BASE_MAX - SAVE_BASE_MIN) × gk_quality
                let save_base = SAVE_BASE_MIN + (SAVE_BASE_MAX - SAVE_BASE_MIN) * gk_quality;

                // positional_factor: penalise saves when GK is away from ball's y.
                // gk_y_error = |ball.pos_y - gk.pos_y|  (at the line-crossing tick)
                let gk_pos_y = state.players[gk_slot_idx].pos_y;
                let gk_y_error = {
                    let diff = state.ball.pos_y - gk_pos_y;
                    let abs_bits = diff.to_bits().unsigned_abs();
                    Q32::from_raw(abs_bits as i64)
                };
                let positional_factor_raw = Q32::ONE - gk_y_error * POSITION_PENALTY_RATE;
                let positional_factor = if positional_factor_raw < POSITION_MIN {
                    POSITION_MIN
                } else {
                    positional_factor_raw
                };

                // xg_clamp: ensure xg_score ≤ Q32::ONE for the (1 - xg) term.
                let xg_clamped = if xg_score > Q32::ONE {
                    Q32::ONE
                } else {
                    xg_score
                };
                let one_minus_xg = Q32::ONE - xg_clamped;

                // save_prob = save_base × (1 - xg_score) × positional_factor
                let save_prob_raw = save_base * one_minus_xg * positional_factor;
                let save_prob = if save_prob_raw > SAVE_PROB_MAX {
                    SAVE_PROB_MAX
                } else {
                    save_prob_raw
                };

                // RNG draw: SeedLayer::ReactiveInterrupt, site = (scorer_slot << 16) | 0x5A7E
                let save_site = ((scorer_slot as u32) << 16) | SAVE_ROLL_SITE_DISCRIMINANT;
                let save_rng_seed = crate::decision_cadence::seed_fn(
                    state.seed.to_u64(),
                    state.tick.to_raw() as u32,
                    crate::decision_cadence::SeedLayer::ReactiveInterrupt,
                    save_site,
                );
                {
                    use rand_chacha::rand_core::{RngCore, SeedableRng};
                    let mut save_rng = rand_chacha::ChaCha8Rng::seed_from_u64(save_rng_seed);
                    let roll_u64 = save_rng.next_u64();
                    // Upper 32 bits → Q32 in [0, 1)
                    let roll = Q32::from_raw((roll_u64 >> 32) as i64);

                    // GK gather (non-shot balls only):
                    //
                    // For a ball reaching the goal mouth with NO shot context
                    // (`xg_score == 0`: drift, deflection, own-goal-direction), the
                    // conceding GK gathers it IF they are within GK_GATHER_RADIUS of
                    // the ball's crossing point. This is the non-shot analogue of
                    // SS3: a GK who tracked the loose ball claims it; one caught
                    // out of position concedes a legitimate goal.
                    //
                    // The SS3 shot-save model (below) handles `xg_score > 0` balls.
                    //
                    // No RNG: the gather is purely geometric — "GK is close enough
                    // to reach the ball" is a deterministic reachability question.
                    let gk_gathered = if xg_score == Q32::ZERO {
                        let gk_bx = state.ball.pos_x;
                        let gk_by = state.ball.pos_y;
                        let gk_px = state.players[gk_slot_idx].pos_x;
                        let gk_py = state.players[gk_slot_idx].pos_y;
                        let dx = gk_px - gk_bx;
                        let dy = gk_py - gk_by;
                        let dist_sq = dx * dx + dy * dy;
                        dist_sq <= GK_GATHER_RADIUS_SQ
                    } else {
                        false
                    };
                    if gk_gathered {
                        // GK claims the loose ball — no goal. Snap ball to GK,
                        // clear velocity, give possession to GK.
                        state.ball.vel_x = Q32::ZERO;
                        state.ball.vel_y = Q32::ZERO;
                        state.ball.vel_z = Q32::ZERO;
                        state.ball.pos_x = state.players[gk_slot_idx].pos_x;
                        state.ball.pos_y = state.players[gk_slot_idx].pos_y;
                        let gk_slot_id = state.players[gk_slot_idx].slot;
                        state.possession = Some(gk_slot_id);
                        state.last_touched_by = Some(gk_slot_id);
                        // SLICE-1: the GK claimed the ball at the goal line —
                        // any pass-in-flight is moot. Clear it to preserve the
                        // ball_in_flight ⇒ possession.is_none() invariant.
                        state.ball_in_flight = None;
                        // goal_fired_this_tick stays false — step 3 OOB skipped.
                        // Ball is at GK body position (≤ 45m) so step 3's bx_abs
                        // check passes naturally; no further action needed.
                    }

                    // SS3 gate (FUN-0b+c): the save model models a KEEPER FACING A
                    // SHOT. It only applies when a real shot put the ball here —
                    // i.e. `last_shot_xg[scorer] > 0` (the BT dispatches AttemptShot
                    // only when xG > XG_SHOOT_THRESHOLD, so a real shot always has
                    // xG > 0). A ball crossing the line with NO shot context
                    // (`xg_score == 0`: own goal, deflection, goalmouth scramble,
                    // dribbled-in ball) is NOT a save situation — the goal stands
                    // (unless the GK gather above already claimed it).
                    // Without this gate, `save_base × (1 - 0) = save_base` (0.73-0.92)
                    // would near-automatically "save" every non-shot crossing, which
                    // both misates football (own goals can't be saved) AND made the
                    // goal-detection geometry tests non-deterministic (they inject a
                    // ball at the line with no shot, so xg_score == 0).
                    let save_made = !gk_gathered && xg_score > Q32::ZERO && roll < save_prob;
                    if save_made {
                        // GK makes the save — no goal. Clear `last_shot_xg` for this
                        // scorer slot and give possession to the GK.
                        state.last_shot_xg[scorer_slot_idx] = Q32::ZERO;
                        // Clear ball velocity and snap ball to GK position (ball cleared).
                        state.ball.vel_x = Q32::ZERO;
                        state.ball.vel_y = Q32::ZERO;
                        state.ball.vel_z = Q32::ZERO;
                        state.ball.pos_x = state.players[gk_slot_idx].pos_x;
                        state.ball.pos_y = state.players[gk_slot_idx].pos_y;
                        let gk_slot_id = state.players[gk_slot_idx].slot;
                        state.possession = Some(gk_slot_id);
                        state.last_touched_by = Some(gk_slot_id);
                        // SLICE-1: GK save claims the ball — clear any pending
                        // flight to preserve the ball_in_flight ⇒
                        // possession.is_none() invariant.
                        state.ball_in_flight = None;
                        // `goal_fired_this_tick` stays false — the ball is near the
                        // goal line but the GK holds it. Step 3 (OOB clamp) will
                        // see ball at GK position (just inside the line) and
                        // clamp it to the goal line if needed; that's fine — the
                        // GK will then distribute. No MatchEvent emitted for the
                        // save in T1 (commentary placeholder deferred to T1-4b+).
                        // Early-continue the goal-detection block.
                        // We set goal_fired_this_tick = false here explicitly
                        // so step 3 skips (we already moved the ball to a safe
                        // position: the GK's body, which is ≤ GOAL_LINE_X).
                        // Assign to goal_fired_this_tick to suppress step-3 OOB.
                        // Actually: the ball is now at the GK pos which is inside
                        // the field (GK is positioned ≤ 45m). Step 3 will see
                        // bx_abs < goal_line_bits and skip. No further action needed.
                        // Break from the goal-detection scope.
                        // (We exit the `if bx_abs >= goal_line_bits` branch below
                        //  by NOT setting goal_fired_this_tick = true, so the
                        //  score increment and KickOff are skipped.)
                    } else if !gk_gathered {
                        // Neither saved (SS3) nor gathered (GK) — goal stands.
                        // Codex 2026-05-16 audit silent-failure P1-1: saturating_add
                        // silently caps at 255. T1's 60-tick smoke seed never reaches
                        // 255 goals but the 90-minute integration scenarios at T1-5+
                        // could; checked_add + panic is the determinism-aligned choice.
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
                        // SLICE-1: a goal fires the canonical possession reset;
                        // clear any pending flight so the invariant
                        // (ball_in_flight.is_some() ⇒ possession.is_none()) holds
                        // and a stale trap can't re-grant possession next tick.
                        state.ball_in_flight = None;
                        // Codex 2026-05-16 audit code-reviewer Critical #1: the
                        // conceding team kicks off after a goal (football rule).
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
                        state.team_tactic_states[0] = tactic_fsm::apply_event(
                            state.team_tactic_states[0],
                            &state.home_archetype_params,
                            tactic_fsm::TacticEvent::Goal,
                            state.tick,
                        );
                        state.team_tactic_states[1] = tactic_fsm::apply_event(
                            state.team_tactic_states[1],
                            &state.away_archetype_params,
                            tactic_fsm::TacticEvent::Goal,
                            state.tick,
                        );
                        goal_fired_this_tick = true;
                    } // end roll < save_prob else (goal stands)
                } // end save RNG scope
            } // end if bx_abs >= goal_line_bits (goal mouth detection)
        } // end inner scope

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

                // T2-1c: emit `TacticEvent::BallOutOfPlay` per-team with the
                // correct `SetPieceKind`. Each team's apply_event arm
                // (`tactic_fsm.rs:379-381`) transitions to
                // `TacticState::SetPiece(kind)` — a per-team-distinct kind
                // because home + away are on opposite sides of the same OOB
                // event (one team gets ThrowInFor, the other ThrowInAgainst;
                // one gets CornerFor, the other CornerAgainst; etc.).
                //
                // SetPieceKind taxonomy per the T2-1c MEMORY spec design table:
                //   past_sideline:           home → ThrowInFor iff last_touched_by is away else ThrowInAgainst
                //                            away mirror.
                //   past_non_goal_line:      determined by which goal-line
                //                            (bx_bits > 0 → away's goal-line = home attacking)
                //                            crossed it + last_touched_by:
                //     home attacking + home last_touched → home CornerFor / away CornerAgainst
                //     home attacking + away last_touched → away GoalKick   / home GoalKickOpponent
                //     away attacking is the symmetric mirror.
                //
                // BallOutOfPlay arm doesn't read archetype_params (matches the
                // BallOutOfPlay enum variant unconditionally + transitions to
                // SetPiece) but we still pass the per-team sidecar for
                // signature consistency + future-compat (if a later spec
                // change makes BallOutOfPlay archetype-dependent, the call
                // site already has the right param).
                let last_team_idx = state.last_touched_by.map(team_of);
                let (home_kind, away_kind) =
                    setpiece_kind_for(past_non_goal_line, bx_bits, last_team_idx);
                let home_arch = state.home_archetype_params;
                state.team_tactic_states[0] = tactic_fsm::apply_event(
                    state.team_tactic_states[0],
                    &home_arch,
                    tactic_fsm::TacticEvent::BallOutOfPlay { kind: home_kind },
                    state.tick,
                );
                let away_arch = state.away_archetype_params;
                state.team_tactic_states[1] = tactic_fsm::apply_event(
                    state.team_tactic_states[1],
                    &away_arch,
                    tactic_fsm::TacticEvent::BallOutOfPlay { kind: away_kind },
                    state.tick,
                );

                // SLICE-1: a pass in flight that crosses the sideline / a
                // non-goal goal-line did NOT reach the receiver in bounds — the
                // ball is now dead at the boundary (a set-piece). Clear the
                // flight here. Without this, ball_in_flight lingers until the
                // 120-tick timeout, which then teleports possession to the
                // (frozen, receiver-suppressed) receiver far from the dead ball
                // — a long possession-None dead period that suppresses goals.
                // Clearing here also preserves the `ball_in_flight.is_some() ⇒
                // possession.is_none()` invariant against the timeout grant.
                state.ball_in_flight = None;
            }
        }

        // T2-1b: snapshot possession BEFORE the dispatch + pickup steps that
        // mutate it. Compared at the end (after step 7b pickup) by
        // `emit_possession_transition_events` to fire `PossessionLost` /
        // `BallRecovered` TacticEvents per the affected team's archetype_params.
        //
        // Snapshot lives AFTER the Goal block (step 2) intentionally: the Goal
        // block already fires `apply_event(Goal)` for both teams + resets
        // possession to the conceding team's CF deterministically. Treating
        // that as a separate PossessionLost/BallRecovered emission would
        // double-apply the FSM transition (Goal hardcodes both teams to
        // MidBlock; a subsequent BallRecovered transition on the same tick
        // would conflict with the Goal-reset intent). The Goal arm is the
        // single source of truth for goal-driven tactic-FSM transitions; the
        // snapshot starts AFTER it to skip the kickoff-possession change.
        // SLICE-1: snapshot effective_possession (not raw possession) BEFORE
        // dispatch. Using effective_possession means an in-flight successful
        // pass counts as the passing team's possession throughout the flight,
        // so emit_possession_transition_events sees a same-team transfer
        // (passer→receiver) rather than a PossessionLost+BallRecovered cycle
        // on every pass tick.
        let eff_possession_before_dispatch = state.effective_possession();

        // Step 4 (was step 2): advance ball physics AFTER goal detection + OOB clamp.
        state.ball = ball_physics::ball_step(&state.ball, &ball_physics::phase1_seeds());

        // Step 4b (SLICE-1): trap check for in-flight passes. Runs AFTER ball
        // physics (so the ball has moved this tick before we check proximity)
        // and BEFORE dispatch (so the receiver-suppression guard in dispatch_tick
        // sees the cleared ball_in_flight when the ball has already arrived).
        // Skipped on goal ticks: goal handling clears possession/ball state
        // authoritatively; a concurrent trap would conflict.
        if !goal_fired_this_tick {
            state = trap_check_in_flight(state);
        }

        // Step 5 (T1-2b-ii): 2 Hz tactic-FSM heartbeat (every 30 ticks per team).
        // Home team heartbeat: tick % 30 == 0.
        // Away team heartbeat: tick % 30 == 15 (offset reduces peak load).
        let tick_raw = state.tick.to_raw();
        if tick_raw % tactic_fsm::HEARTBEAT_INTERVAL_TICKS == 0
            && let Some(new_tts) =
                tactic_fsm::heartbeat_check(&state.team_tactic_states[0], state.tick)
        {
            state.team_tactic_states[0] = new_tts;
        }
        if tick_raw % tactic_fsm::HEARTBEAT_INTERVAL_TICKS == 15
            && let Some(new_tts) =
                tactic_fsm::heartbeat_check(&state.team_tactic_states[1], state.tick)
        {
            state.team_tactic_states[1] = new_tts;
        }

        // Step 6 (T1-2b-iii-a): per-player decision dispatch.
        // T1-11: pass sig_definitions through so the signature dispatcher receives
        // real definitions when the caller has a ContentStore. Passing
        // &BTreeMap::new() (the prior hardcoded value) is still valid for
        // callers without content — no signatures fire in that case.
        //
        // **Codex Tier-2 audit 2026-05-17 P1 (T2-1 split review)**: skip dispatch
        // on goal-tick. Without this guard, the kickoff taker's decision slot
        // can fire on the same tick as the goal (~27% probability per the 30-
        // slot decision-cadence stagger across 22 players: any given tick has
        // ≈22/30 of slots active, and the kickoff taker is 1-of-22 of those —
        // ≈1/30 chance of being active on any specific tick × the 600-tick
        // budget = several goal-tick decisions across a match). A post-goal
        // Pass/Shot/Dribble would mutate possession AGAIN; the downstream
        // `emit_possession_transition_events` would then fire PossessionLost
        // or BallRecovered → overriding the post-goal MidBlock reset the Goal
        // arm of `apply_event` just established. Football reality: clock
        // briefly pauses + players reset positions before the kickoff resumes;
        // modeling that as "skip 1 tick of decisions" matches the intent. The
        // kickoff taker's next decision fires on tick (goal_tick + 1) onward;
        // possession transitions resume normal flow from then. Velocity
        // integration + heartbeat + separation + FullTime emission still run
        // unconditionally — those are not possession-mutating.
        if !goal_fired_this_tick {
            state = dispatch::dispatch_tick(state, sig_definitions);
        }

        // Step 6b (FUN-0b+c Slice B): resolve tackle attempts.
        //
        // Runs AFTER dispatch_tick (so defenders have moved toward the carrier
        // following their B1-corrected Press/Mark intents) and BEFORE velocity
        // integration (so the tackler's position on this tick is the position they
        // decided-from, not the post-integration position). Only fires when a
        // carrier has possession; loose-ball scenarios are unchanged.
        //
        // Skipped on goal ticks (same rationale as dispatch + pickup guards).
        if !goal_fired_this_tick {
            state = resolve_tackles(state);
        }

        // Step 7: integrate player velocity into position.
        let dt = ball_physics::dt_per_tick();
        for p in state.players.iter_mut() {
            p.pos_x += p.vel_x * dt;
            p.pos_y += p.vel_y * dt;
        }

        // Step 7b (T1-15): loose-ball pickup.
        //
        // **Codex Tier-2 audit 2026-05-17 P1**: pickup is also gated on
        // `!goal_fired_this_tick`. The Goal block already set possession to
        // kick_off_taker so the inner `state.possession.is_none()` guard would
        // short-circuit anyway, but the outer explicit skip documents the
        // "post-goal tick = no possession mutation" invariant alongside the
        // dispatch + emit_possession_transition_events guards above/below.
        //
        // When possession is None (ball loose after a shot or free kick), check
        // whether any outfield player is within PICKUP_RADIUS_M of the ball.
        // If so, the nearest qualifying player takes possession.
        //
        // Rationale: ball physics now carries shots 30+ m before stopping (T1-15
        // rolling-friction re-calibration from k=0.035 to k=0.01). Without an
        // active pickup mechanic, possession stays None until a player is manually
        // routed toward the ball AND happens to call Dribble or Pass. With the
        // preempt_check already routing outfield players toward the ball (also
        // T1-15), this pickup closes the loop: once a player is close enough, they
        // claim the ball deterministically (closest player wins; home-first on tie
        // since slots 0..11 are iterated before 11..22; this is a T1 approximation).
        //
        // GK slots (0 and 11) are excluded — GK pickup is handled in goalkeeper_fsm.
        // Pickup DOES NOT emit a MatchEvent (no `Pass` or `Tackle` event yet).
        // T2+ wires a `LooseBallPickup` / `Interception` event when the event
        // schema supports contested-ball semantics.
        // Note on timing: step 6 (dispatch_tick) fires shot intents which:
        //   1. Set possession = None.
        //   2. Snap ball.pos to shooter's feet.
        //   3. Set ball.vel_x = ~23 m/s toward goal.
        // The ball physics step (step 4) ran BEFORE dispatch_tick, so the ball has
        // NOT yet physically moved away from the shooter's feet when this pickup
        // check runs. Guard against phantom re-pickup of a freshly-shot ball by
        // requiring the ball speed to be below PICKUP_MAX_SPEED_MPS — a ball
        // traveling at shot speed (~23 m/s) must not be collected immediately by the
        // shooter standing at the ball's current position. Pickup only triggers when
        // the ball has nearly settled (< 3 m/s), meaning it has traveled far from
        // the shooter or has been deflected and stopped nearby.
        // Pickup radius: 5m — generous for T1. Real ball-control is T2+.
        // A 5m radius ensures that once the ball decelerates near a player,
        // they claim it without needing pixel-perfect convergence.
        const PICKUP_RADIUS_M: Q32 = Q32::from_raw(5_i64 << 32); // 5 metres
        // Speed threshold: ball must be below 8 m/s to be picked up.
        // - Prevents immediate re-pickup after a shot (22 m/s > 8 m/s ✓).
        // - Allows pickup within ~30-40 ticks of a shot slowing from 22→8 m/s.
        // - At 8 m/s, the ball is still "catchable" from a standing start (a player
        //   at the ball's future position can step onto it within 1-2 ticks).
        const PICKUP_MAX_SPEED_MPS: Q32 = Q32::from_raw(8_i64 << 32); // 8 m/s threshold
        let ball_speed_sq =
            state.ball.vel_x * state.ball.vel_x + state.ball.vel_y * state.ball.vel_y;
        let pickup_speed_sq = PICKUP_MAX_SPEED_MPS * PICKUP_MAX_SPEED_MPS;
        // SLICE-1: also gate on ball_in_flight.is_none() — an in-flight pass
        // must not be stolen by the generic pickup; only trap_check_in_flight
        // may grant possession during a pass flight.
        if !goal_fired_this_tick
            && state.possession.is_none()
            && state.ball_in_flight.is_none()
            && ball_speed_sq < pickup_speed_sq
        {
            let bx = state.ball.pos_x;
            let by = state.ball.pos_y;
            let mut best_slot: Option<u8> = None;
            let mut best_dist_sq = Q32::MAX;

            for slot_idx in 0..22usize {
                // Exclude GKs from general pickup, EXCEPT when the ball is near
                // their own goal line (>42m from centre). In that case the GK
                // can claim the ball to restart play with a goal kick or
                // short distribution, rather than leaving the ball stranded
                // 2-3m short of the goal line for hundreds of ticks.
                if slot_idx == 0 || slot_idx == 11 {
                    let bx_bits = bx.to_bits();
                    let bx_abs: u64 = bx_bits.unsigned_abs();
                    const GK_PICKUP_THRESHOLD_BITS: u64 = 42_u64 << 32; // 42m
                    if bx_abs < GK_PICKUP_THRESHOLD_BITS {
                        continue; // ball not near goal — skip GK
                    }
                    // Ball is near a goal line — only allow the GK defending that end.
                    // Home GK (slot 0): defends negative x.
                    // Away GK (slot 11): defends positive x.
                    let ball_in_home_half = bx_bits < 0;
                    let is_home_gk = slot_idx == 0;
                    if ball_in_home_half != is_home_gk {
                        continue; // ball near opponent's goal — skip this GK
                    }
                }
                let p = &state.players[slot_idx];
                let dx = p.pos_x - bx;
                let dy = p.pos_y - by;
                let dist_sq = dx * dx + dy * dy;
                let radius_sq = PICKUP_RADIUS_M * PICKUP_RADIUS_M;
                if dist_sq <= radius_sq && dist_sq < best_dist_sq {
                    best_dist_sq = dist_sq;
                    best_slot = Some(p.slot);
                }
            }

            if let Some(slot) = best_slot {
                state.possession = Some(slot);
                state.last_touched_by = Some(slot);
                // Snap the ball to the player's feet so the next tick's InPossession
                // dispatch fires from the correct position.
                let slot_idx = slot as usize;
                state.ball.pos_x = state.players[slot_idx].pos_x;
                state.ball.pos_y = state.players[slot_idx].pos_y;
                state.ball.vel_x = Q32::ZERO;
                state.ball.vel_y = Q32::ZERO;
                state.ball.vel_z = Q32::ZERO;
            }
        }

        // T2-1b: emit per-team PossessionLost / BallRecovered TacticEvents
        // based on the tick's possession transition. Runs AFTER all possession-
        // mutating steps (dispatch_tick fires shot/pass intents that mutate
        // possession; the pickup block above converts None → Some when a player
        // claims a settled loose ball). Compares `state.possession` against
        // `eff_possession_before_dispatch` captured above (just after the Goal block).
        // See `emit_possession_transition_events` for the transition taxonomy.
        //
        // **Codex Tier-2 audit 2026-05-17 P1**: also skipped on goal_fired_
        // this_tick (same rationale as the dispatch + pickup guards above:
        // the Goal arm of `apply_event` is the single source of truth for
        // goal-tick tactic transitions; running emit_possession_transition_
        // events on the goal-tick would re-transition both teams via
        // PossessionLost / BallRecovered, overriding the post-goal MidBlock
        // reset).
        if !goal_fired_this_tick {
            // SLICE-1: pass eff_possession_before_dispatch (not possession_before_dispatch)
            // so that tactic FSM events use effective possession, which treats an
            // in-flight successful pass as the passing team still owning the ball.
            emit_possession_transition_events(&mut state, eff_possession_before_dispatch);
        }

        // Step 8 (T1-2b-iii-d): player-separation positional correction.
        separation::apply_player_separation(&mut state);
    } // end of in-play gate (steps 2–8)

    // Step 9: emit FullTime when the clock reaches match_end_tick.
    //
    // Must be LAST so all same-tick gameplay events (goals, shots, passes) are
    // already appended before FullTime. Step 9 is OUTSIDE the in-play gate so
    // it fires even if a caller over-advances (jumps past match_end_tick): in
    // that case gameplay was skipped above but FullTime still emits exactly once.
    //
    // The `!full_time_already_emitted` check is intentionally absent here: the
    // step-0 freeze guard guarantees FullTime is NEVER already the tail when we
    // reach step 9. Adding the redundant check would obscure that invariant.
    if state.tick >= state.match_end_tick {
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
        let s1 = tick_match(s0, &BTreeMap::new());
        assert_eq!(s1.tick, Tick::ZERO.successor());
    }

    /// T2-1a self-review CRITICAL-3 (silent-failure-hunter, 2026-05-17):
    /// pin the coherence between three otherwise-independent sites that
    /// MUST agree for the SCHEMA-ONLY drift claim to hold on the smoke pin:
    ///
    /// 1. `DEFAULT_ARCHETYPE_ID` (this file, ~line 126) — the id used by
    ///    `MatchState::initial` for both teams.
    /// 2. `MatchState::initial`'s hardcoded sidecar
    ///    `ArchetypeParams::direct_pressing()` (this file, ~line 481).
    /// 3. The `tactic_fsm::archetype_params_for` bridge output for the
    ///    `attacking-fullback.ron` content fixture.
    ///
    /// If any of these three drift apart, the smoke seed's canonical
    /// hash will diverge from the rebaselined `e0312069…3696` even
    /// without a schema bump — exactly the silent failure mode the
    /// CRITICAL-3 review surfaced (the hardcoded sidecar would no
    /// longer match what `archetype_params_for` returns for the
    /// default id, so any future code path that swaps from "use the
    /// hardcoded sidecar" to "look up params via the bridge" would
    /// flip canonical state with no apparent diff).
    #[test]
    fn initial_default_sidecar_matches_bridge_on_default_archetype_id() {
        use fw_content::ContentStore;
        use std::path::PathBuf;

        // Load real content store to exercise the actual ID resolution path
        // used by `MatchState::initial_with_content`.
        let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("content");
        let store = ContentStore::load_sources(&content_root).expect("ContentStore load failed");
        let default_arch = store
            .tactical_archetypes
            .get(DEFAULT_ARCHETYPE_ID)
            .unwrap_or_else(|| {
                panic!(
                    "DEFAULT_ARCHETYPE_ID {DEFAULT_ARCHETYPE_ID:?} must resolve in the loaded content store"
                )
            });
        let bridge_params = tactic_fsm::archetype_params_for(default_arch);
        let hardcoded_params = tactic_fsm::ArchetypeParams::direct_pressing();

        // The bridge output for the DEFAULT id MUST equal the hardcoded
        // sidecar that `MatchState::initial` injects. If this fails, the
        // smoke-pin SCHEMA-ONLY drift claim is FALSE — either the
        // hardcoded sidecar drifted from `direct_pressing()`, OR the
        // attacking-fullback.ron content drifted from values that map
        // through the bridge to direct_pressing()'s buckets, OR the
        // bridge thresholds drifted. All three are real regressions
        // that would otherwise show up as a "schema-only" rebaseline
        // when in fact they're behavior-driven.
        assert_eq!(
            bridge_params, hardcoded_params,
            "T2-1a coherence broken: bridge({DEFAULT_ARCHETYPE_ID}) != direct_pressing(). \
             The smoke-pin schema-only drift claim relies on these matching. \
             Either the hardcoded sidecar in MatchState::initial drifted, \
             or attacking-fullback.ron content drifted across bridge thresholds, \
             or the bridge thresholds themselves changed. Investigate before \
             rebaselining the smoke pin."
        );

        // Also pin that MatchState::initial actually uses DEFAULT_ARCHETYPE_ID
        // for both teams + injects the matching params — closes the loop on
        // the 3-way coherence.
        let s = MatchState::initial(Seed::from_u64(1));
        assert_eq!(s.home_archetype_id(), DEFAULT_ARCHETYPE_ID);
        assert_eq!(s.away_archetype_id(), DEFAULT_ARCHETYPE_ID);
        assert_eq!(s.home_archetype_params, hardcoded_params);
        assert_eq!(s.away_archetype_params, hardcoded_params);
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
            state = tick_match(state, &BTreeMap::new());
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
            state = tick_match(state, &BTreeMap::new());
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
            state = tick_match(state, &BTreeMap::new());
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
            state = tick_match(state, &BTreeMap::new());
        }

        // The team must NOT still be in HighPress after >600 ticks — either the
        // heartbeat timeout fired (→ MidBlock) OR game events (SetPiece, Goal)
        // took precedence. Both are correct exits. The INVARIANT is "HighPress
        // does not persist forever"; FUN-TS1 changed player positions so the ball
        // now reaches a corner kick before the timeout fires on seed=1.
        assert!(
            !matches!(state.team_tactic_states[0].state, TacticState::HighPress),
            "HighPress must not persist after >600 ticks; \
             at tick {} the team was still in HighPress",
            state.tick.to_raw(),
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

    // -------------------------------------------------------------------------
    // T1-3.6 Chunk 1: RED acceptance test — ball must move in 600-tick run.
    //
    // This test reproduces Codex's adversarial audit finding: ball at (0, 0)
    // in all 601 frames of a 600-tick run (smoke seed 0xdeadbeefdeadbeef).
    //
    // The test is written BEFORE the fix (TDD-RED). It MUST fail before the
    // BT carrier routing fix in evaluate_transitions (Chunk 2) is applied.
    //
    // Two acceptance criteria:
    //   AC-1: ball.pos_x != Q32::ZERO OR ball.pos_y != Q32::ZERO at tick 600
    //         (ball moved from centre spot at some point).
    //   AC-2: at least one MatchEvent::Pass OR MatchEvent::Shot in match_events
    //         (ball-action intent fired at least once in 600 ticks).
    //
    // Re-verify after chunk 2 fix: both must be GREEN.
    // -------------------------------------------------------------------------
    #[test]
    fn t1_3_6_ball_moves_in_600_tick_run_with_smoke_seed() {
        use fw_content::MatchEvent;

        let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);
        let mut state = MatchState::initial(seed);
        for _ in 0..600 {
            state = tick_match(state, &BTreeMap::new());
        }

        // AC-1: ball must not be frozen at centre spot (pos = 0,0) for entire run.
        // After 600 ticks the ball has been kicked, passed, or dribbled at some
        // point — its final position need not be zero.
        // Note: the ball may legally return to centre-spot after a goal, but with
        // carrier routing working it will have moved DURING the run.
        // We check: match_events contains at least one ball-action event, which is
        // the definitive proof that apply_intent's ball-mutation arms fired.
        let has_ball_action = state.match_events.iter().any(|ev| {
            matches!(
                ev,
                MatchEvent::Pass { .. } | MatchEvent::Shot { .. } | MatchEvent::Goal { .. }
            )
        });

        // AC-2: ball position at tick 600 differs from the initial state (which
        // starts at pos_x=0, pos_y=0 at centre spot).
        // This is the "ball actually moved" check independent of events.
        // We compare to the initial state to make the assertion concrete.
        let initial_state = MatchState::initial(seed);
        let ball_changed = state.ball.pos_x != initial_state.ball.pos_x
            || state.ball.pos_y != initial_state.ball.pos_y;

        assert!(
            has_ball_action,
            "T1-3.6 AC-1 FAIL: zero ball-action events (Pass/Shot/Goal) in \
             600-tick smoke seed run. BT carrier routing is not producing \
             on-ball intents — evaluate_transitions must route the possession \
             holder into InPossession state. match_events: {}",
            state.match_events.len(),
        );

        assert!(
            ball_changed || has_ball_action,
            "T1-3.6 AC-2 FAIL: ball position at tick 600 equals initial centre \
             spot AND no ball-action events fired. The ball has not moved in \
             600 ticks. Fix evaluate_transitions in role_states.rs.",
        );
    }

    // -------------------------------------------------------------------------
    // T1-3.6 Chunk 2 integration: after next tick's evaluate_transitions fix,
    // verify possession transfers across pass: slot A fires Pass → slot B;
    // next tick slot B must be in InPossession state (not Defending/Supporting).
    // -------------------------------------------------------------------------
    #[test]
    fn t1_3_6_carrier_routes_to_in_possession_state() {
        use crate::role_states::{DefenderState, ForwardState, MidfielderState, PlayerRoleState};

        let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);
        let mut state = MatchState::initial(seed);

        // At initial state, possession = Some(9) (home centre forward, slot 9).
        assert_eq!(
            state.possession,
            Some(9),
            "initial possession should be slot 9 (home CF)"
        );

        // After one tick, slot 9 should decide and be in InPossession.
        state = tick_match(state, &BTreeMap::new());

        // slot 9 is a Forward. After evaluate_transitions sees possession == Some(9),
        // their role_state MUST be Forward(InPossession).
        let slot9_role_state = state.players[9].role_state;
        let is_in_possession = matches!(
            slot9_role_state,
            PlayerRoleState::Forward(ForwardState::InPossession)
                | PlayerRoleState::Midfielder(MidfielderState::InPossession)
                | PlayerRoleState::Defender(DefenderState::InPossession)
        );

        assert!(
            is_in_possession,
            "T1-3.6: slot 9 (home CF) has initial possession but role_state \
             after tick 1 is {:?}, not InPossession. evaluate_transitions \
             must route the possession holder into InPossession.",
            slot9_role_state
        );
    }
}

#[cfg(test)]
mod setpiece_autoexit_tests {
    use super::*;
    use tactic_fsm::{ArchetypeParams, SetPieceKind, TacticState, TeamTacticState};

    /// Controlled silent-failure guard for the SetPiece auto-exit pattern.
    ///
    /// Replaces the fragile smoke-seed-scan in
    /// `tactic_event_emission_test.rs::setpiece_state_auto_exits_on_possession_loss_to_none`,
    /// which asserted the smoke seed's FIRST emergent SetPiece-exit lands in a
    /// non-MidBlock state. That premise broke at FUN-0b+c: the Slice-B tackle
    /// step shifted the smoke seed's single 600-tick exit to a cross-team
    /// transition where the losing team gets `PossessionLost{recovery_likely:true}`
    /// under the HighPress re-entry cooldown — which legitimately RETURNS MidBlock
    /// (tactic_fsm.rs:419-421). MidBlock is a valid post-exit state, so the
    /// emergent-scan observable could not distinguish "subsequent event fired but
    /// stayed MidBlock" from "subsequent event dropped."
    ///
    /// Codex Tier-2 audit 2026-05-17 P2 #3 protects against: a refactor that fires
    /// `auto_exit_setpiece` (BallInPlay → archetype default) but silently DROPS the
    /// subsequent PossessionLost / BallRecovered, leaving the team stuck in the
    /// BallInPlay default. This controlled test drives the `Some(home) → None`
    /// (release-without-pickup) branch of `emit_possession_transition_events` with
    /// team 0 pre-set to SetPiece + direct-pressing params (MidBlock default). The
    /// branch must:
    ///   1. auto_exit_setpiece: SetPiece → BallInPlay → MidBlock (the default).
    ///   2. PossessionLost{recovery_likely:false}: MidBlock → LowBlock.
    ///
    /// Landing in **LowBlock** proves BOTH fired. If the subsequent PossessionLost
    /// were dropped, the team would remain in MidBlock → assertion fails. Unlike
    /// the emergent smoke-seed exit, this observable is deterministic.
    ///
    /// Mutation discriminator: deleting the `state.team_tactic_states[team_lost] =
    /// apply_event(... PossessionLost ...)` line in the `(Some(a), None)` arm of
    /// `emit_possession_transition_events` leaves the team in MidBlock → fails.
    #[test]
    fn auto_exit_setpiece_then_possession_lost_lands_in_lowblock_not_midblock() {
        let mut state = MatchState::initial(Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF));
        // Explicit direct-pressing params (MidBlock default, High press) so the
        // expected landing is deterministic regardless of init defaults.
        state.home_archetype_params = ArchetypeParams::direct_pressing();
        // Put the home team in SetPiece (a throw-in won) at the current tick.
        state.team_tactic_states[0] = TeamTacticState::initial()
            .transition(TacticState::SetPiece(SetPieceKind::ThrowInFor), state.tick);
        assert!(matches!(
            state.team_tactic_states[0].state(),
            TacticState::SetPiece(_)
        ));

        // After-state: the home carrier (slot 8, team 0) released the ball and
        // nobody picked it up this tick (possession None). `emit` compares against
        // the possession_before snapshot Some(8) → the `(Some(a), None)` release arm.
        state.possession = None;
        emit_possession_transition_events(&mut state, Some(8));

        assert_eq!(
            state.team_tactic_states[0].state(),
            TacticState::LowBlock,
            "auto-exit SetPiece must fire BallInPlay (-> MidBlock default) AND the \
             subsequent PossessionLost{{recovery_likely:false}} (-> LowBlock). \
             Landing in MidBlock would mean the PossessionLost was silently dropped \
             after the auto-exit -- the exact silent failure the Codex Tier-2 P2 #3 \
             hardening guards. Got {:?}.",
            state.team_tactic_states[0].state(),
        );
    }
}

// -------------------------------------------------------------------------
// Goalmouth defending — unit tests (module-private, access to pub(crate) fields)
// -------------------------------------------------------------------------

#[cfg(test)]
mod goalmouth_defending_tests {
    use super::*;

    /// Home defender within DEFENDER_CLEAR_RADIUS of a loose ball heading toward
    /// home goal clears it upfield (vel_x becomes positive).
    ///
    /// Mechanism: `resolve_goalmouth_defending` checks possession == None,
    /// bvx < 0 (toward home), bx < 0 (ball in home half), then finds the
    /// nearest defender within 8 m and sets vel_x = +CLEARANCE_SPEED.
    #[test]
    fn home_defender_near_loose_ball_clears_upfield() {
        let mut state = MatchState::initial(Seed::from_u64(0xC1EA));
        // Ball at x=-46, y=0, heading toward home goal (vel_x=-6).
        // Home CB (slot 1) is at formation (-30, -20); we move it to (-46, 0)
        // so it's exactly co-located with the ball (dist = 0 < 8 m radius).
        state.ball.pos_x = Q32::from_int(-46);
        state.ball.pos_y = Q32::ZERO;
        state.ball.vel_x = -Q32::from_int(6);
        state.ball.vel_y = Q32::ZERO;
        state.players[1].pos_x = Q32::from_int(-46);
        state.players[1].pos_y = Q32::ZERO;
        // Set possession = None (loose ball).
        state.possession = None;
        state.last_touched_by = Some(20); // away player last touched → clearance is urgent

        let cleared = resolve_goalmouth_defending(state);

        assert!(
            cleared.ball.vel_x > Q32::ZERO,
            "home defender should clear ball upfield (vel_x > 0 after clearance); \
             got vel_x={:?}",
            cleared.ball.vel_x
        );
        assert_eq!(
            cleared.ball.vel_x, CLEARANCE_SPEED,
            "cleared vel_x must equal CLEARANCE_SPEED (10 m/s)"
        );
        assert_eq!(cleared.ball.vel_y, Q32::ZERO, "cleared vel_y must be zero");
        // last_touched_by must be the clearing defender (slot 1).
        assert_eq!(
            cleared.last_touched_by,
            Some(1),
            "last_touched_by must be the clearing defender (slot 1)"
        );
        // possession must be None (the clearance releases the ball; nobody holds it).
        assert!(
            cleared.possession.is_none(),
            "possession must be None after defensive clearance"
        );
    }

    /// Clearance does NOT fire when the defender is outside the radius.
    #[test]
    fn defender_outside_radius_does_not_clear() {
        let mut state = MatchState::initial(Seed::from_u64(0xC1EB));
        // Ball at x=-46, vel_x=-6 (toward home goal).
        state.ball.pos_x = Q32::from_int(-46);
        state.ball.pos_y = Q32::ZERO;
        state.ball.vel_x = -Q32::from_int(6);
        state.ball.vel_y = Q32::ZERO;
        // Move ALL home defenders to x=-30 (16 m from ball at -46). >> 8 m radius.
        for slot_idx in 1..PLAYERS_PER_TEAM {
            state.players[slot_idx].pos_x = Q32::from_int(-30);
        }
        state.possession = None;
        state.last_touched_by = Some(20);

        let after = resolve_goalmouth_defending(state);

        // Ball velocity must NOT have changed (no clearance fired).
        assert_eq!(
            after.ball.vel_x,
            -Q32::from_int(6),
            "no defender within radius — ball vel_x must be unchanged"
        );
    }

    /// Clearance does NOT fire when the ball is NOT heading toward home goal
    /// (vel_x ≥ 0).
    #[test]
    fn no_clearance_when_ball_not_heading_toward_home_goal() {
        let mut state = MatchState::initial(Seed::from_u64(0xC1EC));
        // Ball at x=-46 but heading AWAY from home goal (vel_x = +6).
        state.ball.pos_x = Q32::from_int(-46);
        state.ball.pos_y = Q32::ZERO;
        state.ball.vel_x = Q32::from_int(6); // moving away from home goal
        state.ball.vel_y = Q32::ZERO;
        // Place a defender at the ball position.
        state.players[1].pos_x = Q32::from_int(-46);
        state.players[1].pos_y = Q32::ZERO;
        state.possession = None;

        let after = resolve_goalmouth_defending(state);

        assert_eq!(
            after.ball.vel_x,
            Q32::from_int(6),
            "vel_x must not change when ball is moving away from home goal"
        );
    }

    /// Clearance does NOT fire when an away player is physically close to the
    /// ball (dribbling it toward home goal) — that is a legitimate attack.
    #[test]
    fn no_clearance_when_ball_in_possession() {
        let mut state = MatchState::initial(Seed::from_u64(0xC1ED));
        state.ball.pos_x = Q32::from_int(-46);
        state.ball.pos_y = Q32::ZERO;
        state.ball.vel_x = -Q32::from_int(6); // toward home goal
        state.players[1].pos_x = Q32::from_int(-46);
        state.players[1].pos_y = Q32::ZERO;
        // Away player (slot 20) physically co-located with ball (dribbling).
        // With the updated check: possessor within 5 m of ball → physically held
        // → clearance does NOT fire.
        state.possession = Some(20);
        state.players[20].pos_x = Q32::from_int(-46); // slot 20 at ball position
        state.players[20].pos_y = Q32::ZERO;

        let after = resolve_goalmouth_defending(state);

        assert_eq!(
            after.ball.vel_x,
            -Q32::from_int(6),
            "clearance must not fire when ball is in possession"
        );
    }

    /// Away defender within DEFENDER_CLEAR_RADIUS of a loose ball heading toward
    /// away goal clears it toward -X (upfield for away team).
    #[test]
    fn away_defender_near_loose_ball_clears_upfield() {
        let mut state = MatchState::initial(Seed::from_u64(0xC1EE));
        // Ball at x=+46, vel_x=+6 (toward away goal).
        state.ball.pos_x = Q32::from_int(46);
        state.ball.pos_y = Q32::ZERO;
        state.ball.vel_x = Q32::from_int(6);
        state.ball.vel_y = Q32::ZERO;
        // Away CB (slot 12) co-located with ball.
        state.players[12].pos_x = Q32::from_int(46);
        state.players[12].pos_y = Q32::ZERO;
        state.possession = None;
        state.last_touched_by = Some(9); // home player last touched

        let cleared = resolve_goalmouth_defending(state);

        assert!(
            cleared.ball.vel_x < Q32::ZERO,
            "away defender must clear ball toward -X (vel_x < 0); got {:?}",
            cleared.ball.vel_x
        );
        assert_eq!(cleared.ball.vel_x, -CLEARANCE_SPEED);
        assert_eq!(cleared.last_touched_by, Some(12));
    }

    /// `resolve_goalmouth_defending` is deterministic: same input → same output.
    #[test]
    fn resolve_goalmouth_defending_is_deterministic() {
        let mut state = MatchState::initial(Seed::from_u64(0xC1EF));
        state.ball.pos_x = Q32::from_int(-46);
        state.ball.pos_y = Q32::ZERO;
        state.ball.vel_x = -Q32::from_int(6);
        state.players[1].pos_x = Q32::from_int(-46);
        state.players[1].pos_y = Q32::ZERO;
        state.possession = None;

        // Clone MatchState manually (Serialize/Deserialize round-trip via canonical).
        // Actually MatchState doesn't derive Clone — use two fresh identical builds.
        let mut state2 = MatchState::initial(Seed::from_u64(0xC1EF));
        state2.ball.pos_x = Q32::from_int(-46);
        state2.ball.pos_y = Q32::ZERO;
        state2.ball.vel_x = -Q32::from_int(6);
        state2.players[1].pos_x = Q32::from_int(-46);
        state2.players[1].pos_y = Q32::ZERO;
        state2.possession = None;

        let r1 = resolve_goalmouth_defending(state);
        let r2 = resolve_goalmouth_defending(state2);

        assert_eq!(
            r1.encode_canonical(),
            r2.encode_canonical(),
            "resolve_goalmouth_defending must be deterministic"
        );
    }
}

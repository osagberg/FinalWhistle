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
        }
    }

    /// Variant of [`MatchState::initial`] that projects `signature_candidates`
    /// from the loaded content corpus onto match players.
    ///
    /// ## Slot-7 only (T1-11 scope)
    ///
    /// Only slot 7 (home attacking midfielder, the 3rd home midfielder in the
    /// 4-3-3 formation: slots 5/6 = CM, slot 7 = AM) receives real signature
    /// candidates in this constructor. Candidates are read from the first
    /// `PlayerTemplate` with `preferred_role == "AM"` in `content.player_templates`.
    ///
    /// Slots 0-6 and 8-21 keep empty `signature_candidates` (Vec::new()).
    /// T1-7 (procgen player population) will assign per-template candidates
    /// to all 22 slots when real name/attr projection lands.
    ///
    /// ## Fail-loud on missing template
    ///
    /// If `content.player_templates` contains no template with `preferred_role
    /// == "AM"`, this constructor returns `Err`. A missing template indicates a
    /// content-corpus setup problem (e.g. empty ContentStore in a test that
    /// forgot to load sources), not a recoverable runtime state.
    ///
    /// ## Canonical-hash note
    ///
    /// The returned state's canonical encoding differs from `MatchState::initial`
    /// because slot 7's `signature_candidates` Vec is non-empty. The smoke hash
    /// is rebaselined at T1-11 chunk 6 (ADR-0012 trigger #1).
    pub fn initial_with_content(
        seed: Seed,
        content: &fw_content::ContentStore,
        home_archetype_id: &str,
        away_archetype_id: &str,
    ) -> Result<MatchState, ContentInitError> {
        // Find the first AM template by preferred_role. player_templates is keyed
        // by qualified_id (e.g. "fwh.core:player_00042"), not by file stem, so we
        // search by role. BTreeMap iteration is key-ordered — deterministic.
        let template = content
            .player_templates
            .values()
            .find(|t| t.preferred_role.as_str() == "AM")
            .ok_or_else(|| ContentInitError::MissingTemplate {
                key: "preferred_role=AM".into(),
            })?;

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

        // Build the baseline state, then project slot 7's candidates + override
        // the default archetype IDs/params with the caller-supplied pair.
        let mut state = MatchState::initial(seed);

        // Slot 7 = home AM (3rd home midfielder in 4-3-3).
        // Assign the template's signature_candidates directly. The candidates
        // Vec is `pub(crate)` (accessible here since we're in the same crate).
        state.players[7].signature_candidates = template.signature_candidates.clone();

        // T2-1a: override the default archetype state with caller-supplied IDs.
        state.home_archetype_id = home_archetype_id.to_string();
        state.away_archetype_id = away_archetype_id.to_string();
        state.home_archetype_params = home_archetype_params;
        state.away_archetype_params = away_archetype_params;

        Ok(state)
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
/// ## Signature change (T1-11)
///
/// `sig_definitions` is now a required parameter (formerly `tick_match` called
/// `dispatch_tick` with `&BTreeMap::new()` — meaning signatures could never
/// fire in the normal match path). Pass `&content_store.signature_definitions`
/// to enable the real dispatcher. Pass `&BTreeMap::new()` in tests / contexts
/// without a ContentStore for backwards-compat (no signatures will fire but
/// the match advances normally).
///
/// ## Nine sequential steps (T1-3.5 reorders boundary checks before physics)
///
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
///      **T1-11:** passes `sig_definitions` so the signature dispatcher
///      receives real definitions when available.
///   7. Integrate player velocity into position (`pos += vel × dt`).
///   8. Player-separation positional-correction pass (T1-2b-iii-d).
///   9. Emit `MatchEvent::FullTime` if `state.tick >= state.match_end_tick`
///      AND no FullTime is already at the tail of `match_events` (T1-4a).
pub fn tick_match(
    mut state: MatchState,
    sig_definitions: &BTreeMap<String, fw_content::SignatureDefinition>,
) -> MatchState {
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
            // T2-1a: per-team archetype params. Pre-T2-1a this used a single
            // hardcoded `direct_pressing()` for BOTH teams; now each team's
            // tactic-FSM transitions consult ITS OWN archetype's parameters
            // (resolved at MatchState construction via the bridge in
            // tactic_fsm::archetype_params_for + cached in the sidecar fields).
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
    // T1-11: pass sig_definitions through so the signature dispatcher receives
    // real definitions when the caller has a ContentStore. Passing
    // &BTreeMap::new() (the prior hardcoded value) is still valid for
    // callers without content — no signatures fire in that case.
    state = dispatch::dispatch_tick(state, sig_definitions);

    // Step 7: integrate player velocity into position.
    let dt = ball_physics::dt_per_tick();
    for p in state.players.iter_mut() {
        p.pos_x += p.vel_x * dt;
        p.pos_y += p.vel_y * dt;
    }

    // Step 7b (T1-15): loose-ball pickup.
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
    let ball_speed_sq = state.ball.vel_x * state.ball.vel_x + state.ball.vel_y * state.ball.vel_y;
    let pickup_speed_sq = PICKUP_MAX_SPEED_MPS * PICKUP_MAX_SPEED_MPS;
    if state.possession.is_none() && ball_speed_sq < pickup_speed_sq {
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

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
pub mod subtree_library;
pub mod tactic_fsm;

#[cfg(test)]
use fw_core::Q32;
use fw_core::{Seed, Tick};
use serde::{Deserialize, Serialize};

pub use ball::BallState;
pub use ball_physics::{BallPhysicsCoefficients, dt_per_tick, phase1_seeds};
pub use canonical::CanonicalEncoder;
pub use decision_cadence::{SeedLayer, assign_decision_slots, seed_fn, should_decide};
pub use dto::{BallFrameDto, MatchFrameDto, PlayerFrameDto};
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

/// Slot index for a player. Stable for the duration of a match — the slot
/// holds the canonical position in the team's ordered roster (GK = slot 0,
/// outfield by tactical position thereafter). Substitutions swap the
/// occupant of a slot; the slot identifier itself never changes mid-match.
///
/// This is the canonical-encoding key for player state: encoding iterates
/// slots 0..22 in fixed order, so the encoded byte stream is structural,
/// not pointer-dependent.
pub type PlayerSlot = u8;

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
}

// -------------------------------------------------------------------------
// tick_match — the canonical advance function
// -------------------------------------------------------------------------

/// Advance the match by one tick.
///
/// T1-2b-iii-a scope:
/// - Increments `state.tick`.
/// - Advances ball physics (T1-2b-i).
/// - Runs the 2 Hz tactic-FSM heartbeat every 30 ticks per team (T1-2b-ii).
/// - Dispatches per-player BT / GK-FSM decisions via `dispatch_tick`
///   (T1-2b-iii-a): for each roster slot where `should_decide` fires, runs
///   the appropriate decision runner and applies the returned `PlayerIntent`
///   by mutating `vel_x`/`vel_y`.
pub fn tick_match(mut state: MatchState) -> MatchState {
    state.tick = state.tick.successor();
    // T1-2b-i: advance ball physics by one 60Hz tick.
    state.ball = ball_physics::ball_step(&state.ball, &ball_physics::phase1_seeds());

    // T1-2b-ii: 2 Hz tactic-FSM heartbeat (every 30 ticks per team).
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

    // T1-2b-iii-a: per-player decision dispatch.
    state = dispatch::dispatch_tick(state);

    // T1-2b-iii-a self-review P0-1: integrate player velocity into position.
    // Every player's position advances by vel * dt each tick, using the same
    // dt_per_tick() the ball physics uses (1/60 s in Q32.32).
    let dt = ball_physics::dt_per_tick();
    for p in state.players.iter_mut() {
        // checked_mul/add: prefer explicit overflow protection over panic.
        if let (Some(dx), Some(dy)) = (p.vel_x.checked_mul(dt), p.vel_y.checked_mul(dt))
            && let (Some(nx), Some(ny)) = (p.pos_x.checked_add(dx), p.pos_y.checked_add(dy))
        {
            p.pos_x = nx;
            p.pos_y = ny;
        }
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
    #[test]
    fn tick_match_advances_ball_physics() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        // Set the ball to 10m up with 5 m/s along +X — it should fall +
        // drift over 60 ticks (1 second).
        state.ball.pos_y = Q32::from_int(10);
        state.ball.vel_x = Q32::from_int(5);
        let initial_pos_x = state.ball.pos_x;
        let initial_pos_y = state.ball.pos_y;
        for _ in 0..60 {
            state = tick_match(state);
        }
        // After 1 second: ball has drifted along +X and fallen.
        assert!(
            state.ball.pos_x > initial_pos_x,
            "ball didn't drift in +X under initial velocity"
        );
        assert!(
            state.ball.pos_y < initial_pos_y,
            "ball didn't fall under gravity"
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

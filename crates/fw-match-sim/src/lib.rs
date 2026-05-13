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
pub mod canonical;
pub mod dto;
pub mod player;

use fw_core::{Q32, Seed, Tick};
use serde::{Deserialize, Serialize};

pub use ball::BallState;
pub use canonical::CanonicalEncoder;
pub use dto::{BallFrameDto, MatchFrameDto, PlayerFrameDto};
pub use player::PlayerState;

// -------------------------------------------------------------------------
// Constants
// -------------------------------------------------------------------------

/// Number of players per team. Football is football.
pub const PLAYERS_PER_TEAM: usize = 11;

/// Total players on the pitch (both teams).
pub const TOTAL_PLAYERS: usize = PLAYERS_PER_TEAM * 2;

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
}

impl MatchState {
    /// Initial state at `Tick::ZERO`. Players in canonical kick-off
    /// formation (placeholder positions for T0; real formations land in T1
    /// when `fw-content` ships `TeamTemplate`).
    pub fn initial(seed: Seed) -> MatchState {
        let mut players = Vec::with_capacity(TOTAL_PLAYERS);

        // T0 placeholder layout: home on x = -10, away on x = +10. Each
        // player gets a unique y so the canonical encoding has structurally
        // distinct positions. Real kick-off formations + role placement
        // arrive in T1 per `docs/MASTER_PLAN.md` (Phase-1 sim core).
        for slot in 0..TOTAL_PLAYERS as u8 {
            let team = slot / PLAYERS_PER_TEAM as u8; // 0 = home, 1 = away
            let in_team_index = slot % PLAYERS_PER_TEAM as u8;
            let x_sign = if team == 0 { -1 } else { 1 };
            let x = Q32::from_int(10 * x_sign);
            // Spread y across [-5, 5]; in-team index 0..10 maps to that band.
            let y = Q32::from_int(in_team_index as i32) - Q32::from_int(5);
            players.push(PlayerState::at(slot, x, y));
        }

        MatchState {
            seed,
            tick: Tick::ZERO,
            players,
            ball: BallState::centre_spot(),
            home_score: 0,
            away_score: 0,
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
/// Phase-0 scope: increments `state.tick` and returns. Future ticks (T1+)
/// run player BTs, ball physics, set-piece state machines, and so on. The
/// signature `MatchState -> MatchState` lets callers stash intermediate
/// states for rewind/replay; the function takes self-by-value to make the
/// "old state is consumed" semantics explicit (no aliased mutation across
/// ticks).
pub fn tick_match(mut state: MatchState) -> MatchState {
    state.tick = state.tick.successor();
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
}

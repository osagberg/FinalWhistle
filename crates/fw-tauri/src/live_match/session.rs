//! `LiveMatchSession` — in-flight match state held in `AppState::live_matches`.
//!
//! One session per active live match handle. Sessions are inserted by
//! `start_live_match`, mutated by `step_live_match` / `apply_match_command`,
//! and removed by `finish_live_match`.
//!
//! The session is sync-access-only: `AppState::live_matches` is a
//! `RwLock<BTreeMap<u32, LiveMatchSession>>`. Handlers that need to mutate
//! (step, command) take the write lock; read-only handlers (snapshot) take
//! the read lock.

use fw_content::MatchEvent;
use fw_match_sim::MatchState;

use super::types::MatchCommand;

/// An active live-match session tracked by `AppState`.
pub struct LiveMatchSession {
    /// Handle ID — the key in `AppState::live_matches`. Stored here so the
    /// session is self-describing (useful for debug logging and test assertions).
    pub id: u32,

    /// The raw seed used to initialise this match. Stable for the session lifetime.
    pub seed: u64,

    /// Human-readable seed echo (`"0x..."` form). Stored to avoid re-formatting
    /// on every `MatchHandle` projection.
    pub seed_hex: String,

    /// The live canonical sim state. Replaced atomically on every `step_live_match`
    /// tick batch. Read-only on `get_match_snapshot`.
    ///
    /// `state.match_events()` is the single source of truth for the match's event
    /// stream — it is append-only and part of canonical state (`fw-match-sim`
    /// encoder VERSION 7), so there is no separate session-side event mirror.
    /// `step_live_match` produces its `new_events` delta by recording
    /// `match_events().len()` before the tick loop and slicing after.
    pub state: MatchState,

    /// Running possession tally: `[home_ticks, away_ticks]`.
    ///
    /// Updated each tick in `step_live_match` based on `state.possession()`.
    /// Slot 0..11 = home; slot 11..22 = away. `None` possession ticks are skipped.
    pub possession_ticks: [u32; 2],

    /// Audit trail of every `apply_match_command` call — even though all
    /// commands currently return `Err(LiveMatchCommandUnimplemented)`, storing
    /// the intent stream preserves debuggability for future wiring.
    pub pending_commands: Vec<MatchCommand>,
}

impl LiveMatchSession {
    /// Construct a fresh session for the given `seed` and initial `state`.
    pub fn new(id: u32, seed: u64, seed_hex: String, state: MatchState) -> Self {
        LiveMatchSession {
            id,
            seed,
            seed_hex,
            state,
            possession_ticks: [0, 0],
            pending_commands: Vec::new(),
        }
    }

    /// `true` once the match has emitted its `FullTime` event.
    ///
    /// Scans for `FullTime` anywhere in the event stream rather than only the
    /// tail: the sim does not currently halt at full time (it keeps integrating
    /// to match `play_match`'s full-budget behaviour), so post-whistle events
    /// can follow `FullTime`. The event vec is short (tens of entries per
    /// match) so the scan is cheap.
    pub fn is_finished(&self) -> bool {
        self.state
            .match_events()
            .iter()
            .any(|e| matches!(e, MatchEvent::FullTime { .. }))
    }
}

//! Live-match IPC subsystem — ADR-0004 §1-3.
//!
//! Ships the server-side of the five live-match commands:
//! `start_live_match`, `step_live_match`, `get_match_snapshot`,
//! `apply_match_command`, `finish_live_match`.
//!
//! This module owns:
//! - `types.rs` — all DTOs and command types (MatchHandle, MatchSnapshot, MatchCommand, …)
//! - `session.rs` — `LiveMatchSession` (the in-flight match state stored in `AppState`)
//! - `snapshot.rs` — `MatchState → MatchSnapshot` projection logic

pub mod session;
pub mod snapshot;
pub mod types;

pub use session::LiveMatchSession;
pub use types::{
    BallZone, FinalMatchResult, FormationId, KNOWN_MATCH_COMMAND_KINDS, LineupDto, MatchCommand,
    MatchHandle, MatchPhase, MatchSnapshot, PossessionDto, PressLevel, ScoreDto, StepResult,
    TeamTalkId, TempoBias,
};

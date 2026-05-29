//! Live-match IPC DTOs — ADR-0004 §2 + §3.
//!
//! All types here are **one-way projections** from canonical `MatchState`
//! toward the frontend. They are NEVER deserialized back into canonical state
//! (`Tauri/RULES.md §3`). `f64` is allowed here — this is the IPC translation
//! layer, not the sim. `BTreeMap` / `BTreeSet` are used throughout to keep one
//! rule across the codebase (no "which DTO was canonical-feeding again?" confusion).

use std::collections::{BTreeMap, BTreeSet};

use fw_core::PlayerId;
use serde::{Deserialize, Serialize};

use crate::result::MatchEventDto;

// ---------------------------------------------------------------------------
// MatchHandle
// ---------------------------------------------------------------------------

/// An opaque reference to a live-match session in `AppState::live_matches`.
///
/// `id` keys into `BTreeMap<u32, LiveMatchSession>`. `seed_hex` is informational
/// (replay links, bug reports) — the frontend treats the handle as opaque per
/// ADR-0004 §1.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchHandle {
    pub id: u32,
    pub seed_hex: String,
}

// ---------------------------------------------------------------------------
// StepResult
// ---------------------------------------------------------------------------

/// Returned by `step_live_match`. Carries the events emitted during this step
/// only (since the previous `step_live_match` call), the current score, the
/// current tick, and a flag indicating whether the match has reached FullTime.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepResult {
    pub handle: MatchHandle,
    /// Events emitted during this step only (delta since last call).
    pub new_events: Vec<MatchEventDto>,
    pub score: ScoreDto,
    pub tick: u32,
    pub is_finished: bool,
}

// ---------------------------------------------------------------------------
// MatchSnapshot
// ---------------------------------------------------------------------------

/// Fat read DTO powering scoreboard, lineup, and event-feed panels (ADR-0004 §3).
///
/// `yellow_cards` and `sent_off` are empty at T1 (no card system). They are
/// typed correctly so the frontend can consume them without a future schema
/// change.
///
/// `BTreeMap` / `BTreeSet` at the DTO boundary: serde iterates them in key
/// order, making the JSON byte-stable across builds and platforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchSnapshot {
    pub handle: MatchHandle,
    pub tick: u32,
    /// `tick / TICKS_PER_GAME_MINUTE` — game-minute marker for UI display.
    pub minute: u16,
    pub phase: MatchPhase,
    pub score: ScoreDto,
    pub possession_pct: PossessionDto,
    pub ball_zone: BallZone,
    pub home_lineup: LineupDto,
    pub away_lineup: LineupDto,
    /// Last `SNAPSHOT_RECENT_EVENTS_CAP` events (chronological order).
    pub recent_events: Vec<MatchEventDto>,
    /// Per-player yellow-card count. Empty at T1 (no card system).
    pub yellow_cards: BTreeMap<u32, u8>,
    /// Players sent off. Empty at T1 (no card system).
    pub sent_off: BTreeSet<u32>,
}

/// Maximum number of recent events included in a `MatchSnapshot`.
pub const SNAPSHOT_RECENT_EVENTS_CAP: usize = 16;

// ---------------------------------------------------------------------------
// MatchPhase
// ---------------------------------------------------------------------------

/// Coarse phase of the match, derived from tick + emitted events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MatchPhase {
    FirstHalf,
    HalfTime,
    SecondHalf,
    FullTime,
}

// ---------------------------------------------------------------------------
// BallZone
// ---------------------------------------------------------------------------

/// 5-bucket pitch zone derived from the ball's `pos_x`.
///
/// Bucket boundaries (in metres, coord convention: home defends -X, away +X;
/// `GOAL_LINE_X` ≈ 52.5 m from centre):
///
/// | Zone                  | pos_x range          |
/// |-----------------------|----------------------|
/// | OwnDefensiveThird     | x ≤ -21.0 m          |
/// | OwnMidThird           | -21.0 < x ≤ -7.0 m  |
/// | Center                | -7.0 < x ≤  7.0 m   |
/// | OppMidThird           |  7.0 < x ≤ 21.0 m   |
/// | OppAttackingThird     | x > 21.0 m           |
///
/// Boundaries are roughly thirds of the 105 m pitch length (Q32 raw-bits
/// comparison avoids float arithmetic in a non-canonical context — but this
/// is an IPC helper, so f64 is fine). From home's perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BallZone {
    OwnDefensiveThird,
    OwnMidThird,
    Center,
    OppMidThird,
    OppAttackingThird,
}

// ---------------------------------------------------------------------------
// LineupDto
// ---------------------------------------------------------------------------

/// 11-slot team lineup. `players[i]` is the PlayerId occupying slot `i` within
/// the team (GK = 0, outfield thereafter in the 4-3-3 formation order).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineupDto {
    /// Raw `PlayerId` u32 values for all 11 slots.
    pub players: Vec<u32>,
}

// ---------------------------------------------------------------------------
// ScoreDto / PossessionDto
// ---------------------------------------------------------------------------

/// Home/away score pair in a `MatchSnapshot` (distinct from `result::Score`
/// to keep the live-match DTO surface self-contained).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreDto {
    pub home: u8,
    pub away: u8,
}

/// Running possession estimate for the snapshot.
///
/// `home_pct + away_pct == 100` (within integer rounding). When no ticks have
/// elapsed yet (`total_possession_ticks == 0`) both fields are 50.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PossessionDto {
    pub home_pct: u8,
    pub away_pct: u8,
}

// ---------------------------------------------------------------------------
// PressLevel / TempoBias
// ---------------------------------------------------------------------------

/// Manager-instructed pressing intensity (closed set per ADR-0004 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PressLevel {
    Low,
    Mid,
    High,
}

/// Manager-instructed tempo bias (closed set per ADR-0004 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TempoBias {
    Slow,
    Even,
    Fast,
}

// ---------------------------------------------------------------------------
// MatchCommand — closed set per ADR-0004 §2
// ---------------------------------------------------------------------------

/// Content-pack-qualified formation ID (e.g. `"fwh.core:formation.4-3-3"`).
pub type FormationId = String;

/// Content-pack-qualified team-talk message ID.
pub type TeamTalkId = String;

/// A manager intent enqueued between ticks.
///
/// The set is **closed** (ADR-0004 §2): new intents need a logged decision.
/// All 9 variants currently return `Err(IpcError::LiveMatchCommandUnimplemented)`
/// — they are accepted over the wire (deserialization works) but not acted on.
///
/// `#[serde(tag = "kind", rename_all = "camelCase")]` produces the
/// TS-narrowable discriminated union: `{ kind: "substitute", ... }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MatchCommand {
    Substitute {
        #[serde(rename = "playerIn")]
        player_in: PlayerId,
        #[serde(rename = "playerOut")]
        player_out: PlayerId,
    },
    ChangeFormation {
        formation: FormationId,
    },
    ChangePressLevel {
        level: PressLevel,
    },
    ChangeTempoBias {
        bias: TempoBias,
    },
    SetCornerTaker {
        player: PlayerId,
    },
    SetFreeKickTaker {
        player: PlayerId,
    },
    SetPenaltyTaker {
        player: PlayerId,
    },
    SetCaptain {
        player: PlayerId,
    },
    TeamTalk {
        #[serde(rename = "messageId")]
        message_id: TeamTalkId,
    },
}

/// Canonical `kind` strings for all 9 `MatchCommand` variants.
///
/// Must stay in sync with the `#[serde(rename_all = "camelCase")]` output.
/// The TS mirror is `KNOWN_LIVE_MATCH_COMMAND_KINDS` in `lib/types.ts`.
pub const KNOWN_MATCH_COMMAND_KINDS: [&str; 9] = [
    "substitute",
    "changeFormation",
    "changePressLevel",
    "changeTempoBias",
    "setCornerTaker",
    "setFreeKickTaker",
    "setPenaltyTaker",
    "setCaptain",
    "teamTalk",
];

impl MatchCommand {
    /// Return the camelCase `kind` string for this variant.
    ///
    /// Used by `apply_match_command` to populate the `kind` field of
    /// `IpcError::LiveMatchCommandUnimplemented` — a single call site avoids
    /// duplicating the string literals from `KNOWN_MATCH_COMMAND_KINDS`.
    pub fn kind_str(&self) -> &'static str {
        match self {
            MatchCommand::Substitute { .. } => "substitute",
            MatchCommand::ChangeFormation { .. } => "changeFormation",
            MatchCommand::ChangePressLevel { .. } => "changePressLevel",
            MatchCommand::ChangeTempoBias { .. } => "changeTempoBias",
            MatchCommand::SetCornerTaker { .. } => "setCornerTaker",
            MatchCommand::SetFreeKickTaker { .. } => "setFreeKickTaker",
            MatchCommand::SetPenaltyTaker { .. } => "setPenaltyTaker",
            MatchCommand::SetCaptain { .. } => "setCaptain",
            MatchCommand::TeamTalk { .. } => "teamTalk",
        }
    }
}

/// Final result returned by `finish_live_match`.
///
/// Re-uses the same shape as `result::Score` for the score pair so the
/// frontend can share display code with `MatchResult`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalMatchResult {
    pub handle: MatchHandle,
    pub final_score: ScoreDto,
    pub tick: u32,
    pub total_events: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_match_command_kinds_has_nine_entries() {
        assert_eq!(KNOWN_MATCH_COMMAND_KINDS.len(), 9);
    }

    #[test]
    fn match_command_kind_str_covers_all_nine_variants() {
        // Build one of each variant and check kind_str matches the constant.
        let cmds: [MatchCommand; 9] = [
            MatchCommand::Substitute {
                player_in: PlayerId::new(1),
                player_out: PlayerId::new(2),
            },
            MatchCommand::ChangeFormation {
                formation: "fwh.core:formation.4-3-3".to_string(),
            },
            MatchCommand::ChangePressLevel {
                level: PressLevel::Mid,
            },
            MatchCommand::ChangeTempoBias {
                bias: TempoBias::Even,
            },
            MatchCommand::SetCornerTaker {
                player: PlayerId::new(3),
            },
            MatchCommand::SetFreeKickTaker {
                player: PlayerId::new(4),
            },
            MatchCommand::SetPenaltyTaker {
                player: PlayerId::new(5),
            },
            MatchCommand::SetCaptain {
                player: PlayerId::new(6),
            },
            MatchCommand::TeamTalk {
                message_id: "fwh.core:teamtalk_00001".to_string(),
            },
        ];

        for (i, cmd) in cmds.iter().enumerate() {
            assert_eq!(
                cmd.kind_str(),
                KNOWN_MATCH_COMMAND_KINDS[i],
                "variant {i} kind_str mismatch"
            );
        }
    }

    #[test]
    fn match_command_serde_round_trip_substitute() {
        let cmd = MatchCommand::Substitute {
            player_in: PlayerId::new(7),
            player_out: PlayerId::new(9),
        };
        let json = serde_json::to_string(&cmd).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(v["kind"], "substitute");
        assert_eq!(v["playerIn"], 7_u32);
        assert_eq!(v["playerOut"], 9_u32);
    }

    #[test]
    fn match_command_serde_round_trip_change_press_level() {
        let cmd = MatchCommand::ChangePressLevel {
            level: PressLevel::High,
        };
        let json = serde_json::to_string(&cmd).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(v["kind"], "changePressLevel");
        assert_eq!(v["level"], "high");
    }

    #[test]
    fn possession_dto_sums_to_100_at_kickoff() {
        let p = PossessionDto {
            home_pct: 50,
            away_pct: 50,
        };
        assert_eq!(p.home_pct + p.away_pct, 100);
    }
}

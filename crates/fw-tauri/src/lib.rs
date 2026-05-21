//! `fw-tauri` — Tauri command bridge.
//!
//! The IPC boundary between the canonical Rust sim and the SolidJS
//! frontend. The frontend `invoke()`s these commands; the handlers wrap
//! the sim's sync API in async functions (Tauri's IPC layer is async
//! end-to-end, but the sim itself stays sync).
//!
//! ## Architecture
//!
//! - The sim crates (`fw-match-sim`, `fw-memory`, `fw-save`, etc.) are
//!   sync, deterministic, and float-free.
//! - This crate is the *only* one that imports the whole vertical: it
//!   pulls in match-sim + content + scouting + memory + save together to
//!   compose a full command surface.
//! - The Tauri shell binary at `src-tauri/` re-exports these commands via
//!   `tauri::Builder::default().invoke_handler(...)`.
//!
//! ## Determinism is upstream
//!
//! `fw-tauri` does NOT have `clippy::float_arithmetic = deny`. The IPC
//! layer translates Q32 to JSON numbers for the frontend, and SolidJS
//! consumes f64. That translation has to compile. The determinism
//! contract is enforced one crate upstream (`fw-match-sim` et al.).

use fw_match_sim::MatchState;
use serde::Serialize;

// Command handlers live in a sibling module to sidestep the Tauri 2
// `E0255 __cmd__<name> defined multiple times` bug that fires when
// `#[tauri::command]` is applied to a `pub` function inside `lib.rs`.
// See `commands.rs` header for the full reference.
pub mod commands;
pub mod error;
pub mod handshake;
pub mod result;
pub mod season;
pub mod state;

pub use commands::{
    advance_week, get_backend_handshake, get_fixtures, get_squad, get_standings, match_frames,
    play_fixtures, play_match,
};
pub use error::IpcError;
pub use handshake::BackendHandshakeDto;
pub use result::{MatchEventDto, MatchResult, Score};
pub use state::AppState;

/// Maximum number of frames allowed in a single `match_frames` request.
///
/// 7200 = 2 minutes of match time at 60 Hz (20 real-match-minutes × 60s × 6
/// ticks/s = 7200 ticks). Pre-invoke validation in `TauriFrameSource` mirrors
/// this constant — see `frontend/src/lib/types.ts:MAX_FRAMES_PER_REQUEST`.
pub const MAX_FRAMES_PER_REQUEST: u32 = 7200;

// -------------------------------------------------------------------------
// Season DTOs (T2-5) — returned by the 4 season IPC commands
// -------------------------------------------------------------------------

/// Returned by `advance_week`.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvanceWeekSummaryDto {
    pub match_day_played: u16,
    pub matches_played: u16,
    pub season_complete: bool,
}

/// Returned by `play_fixtures`.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayFixturesSummaryDto {
    pub matches_played: u32,
    pub final_match_day: u16,
}

/// One row in the standings table, returned as an element of
/// `Vec<StandingsRowDto>` by `get_standings`.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StandingsRowDto {
    /// `ClubId.raw()` — raw u32 wire form; TS receives as `number`.
    pub club_id: u32,
    pub club_name: String,
    pub played: u16,
    pub wins: u16,
    pub draws: u16,
    pub losses: u16,
    pub goals_for: u16,
    pub goals_against: u16,
    pub goal_difference: i32,
    pub points: u16,
}

/// One player in the squad list returned by `get_squad`.
///
/// Columns: player_id, display name, role family, birth region, scout phenotype
/// labels (human-readable strings from `PhenotypeLabelId::display_label`).
/// Age and contract fields are deliberately absent — they are T4+ career-roster
/// state that `PlayerBio` does not carry.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SquadPlayerDto {
    /// Content-pack-qualified player ID (`fwh.core:player_00042`).
    pub player_id: String,
    /// Full display name.
    pub name: String,
    /// Human-readable role family label (e.g. "Centre-back").
    pub role: String,
    /// Fantasy birth region string.
    pub birth_region: String,
    /// Scout phenotype labels in BTreeSet iteration order (deterministic).
    pub phenotype_labels: Vec<String>,
}

/// One fixture entry in the `get_fixtures(club_id)` response.
///
/// `homeScore` / `awayScore` are `None` for unplayed fixtures (serde omits
/// `null` via `skip_serializing_if`; TS receives `undefined` → consumers
/// treat absent as unplayed).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureWithResultDto {
    pub match_day: u16,
    pub opponent_club_id: u32,
    pub opponent_club_name: String,
    pub is_home: bool,
    pub played: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home_score: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub away_score: Option<u8>,
}

// -------------------------------------------------------------------------
// Frontend DTOs — what the SolidJS side sees
// -------------------------------------------------------------------------

/// A flattened match-state DTO suitable for the frontend. Q32 values are
/// rendered as f64 here — viewer-side use only; the canonical sim still
/// keeps Q32 internally.
///
/// The renderer is free to interpolate / animate / smooth these; the
/// canonical state is always re-derivable from `(seed, tick_count)`.
///
/// Codex pre-T1-2b audit P0 fix (2026-05-13): `#[serde(rename_all =
/// "camelCase")]` added to mirror `Tauri/RULES.md §3` ("Use
/// `#[serde(rename_all = "camelCase")]` on payloads so TS receives
/// `playerName`, not `player_name`"). Prior version emitted snake_case
/// over the wire, which the TS consumers in `frontend/src/lib/types.ts`
/// would have had to compensate for manually.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchStateDto {
    pub seed_hex: String,
    pub tick: i64,
    pub home_score: u8,
    pub away_score: u8,
    pub players: Vec<PlayerDto>,
    pub ball: BallDto,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDto {
    pub slot: u8,
    pub pos_x: f64,
    pub pos_y: f64,
    pub vel_x: f64,
    pub vel_y: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BallDto {
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    pub vel_x: f64,
    pub vel_y: f64,
    pub vel_z: f64,
}

impl MatchStateDto {
    /// Project the canonical `MatchState` into the frontend DTO. Lossy by
    /// design: f64 cannot represent every Q32 exactly, but the renderer
    /// doesn't need exact — only stable-looking.
    pub fn from_state(state: &MatchState) -> MatchStateDto {
        MatchStateDto {
            seed_hex: format!("0x{:016x}", state.seed.to_u64()),
            tick: state.tick.to_raw(),
            home_score: state.home_score,
            away_score: state.away_score,
            players: state
                .players
                .iter()
                .map(|p| PlayerDto {
                    slot: p.slot,
                    pos_x: q32_to_f64(p.pos_x.to_bits()),
                    pos_y: q32_to_f64(p.pos_y.to_bits()),
                    vel_x: q32_to_f64(p.vel_x.to_bits()),
                    vel_y: q32_to_f64(p.vel_y.to_bits()),
                })
                .collect(),
            ball: BallDto {
                pos_x: q32_to_f64(state.ball.pos_x.to_bits()),
                pos_y: q32_to_f64(state.ball.pos_y.to_bits()),
                pos_z: q32_to_f64(state.ball.pos_z.to_bits()),
                vel_x: q32_to_f64(state.ball.vel_x.to_bits()),
                vel_y: q32_to_f64(state.ball.vel_y.to_bits()),
                vel_z: q32_to_f64(state.ball.vel_z.to_bits()),
            },
        }
    }
}

/// Q32-raw-bits to f64. Renderer-side only — never call this on a canonical
/// state value the sim will read back.
///
/// Q32.32 means the raw i64 represents the value × 2^32; we divide by
/// 2^32 to get the f64. Multiplication here is intentional float math
/// and only legal because this crate doesn't have the deny lint.
#[allow(clippy::float_arithmetic)]
fn q32_to_f64(raw_bits: i64) -> f64 {
    const Q32_SCALE: f64 = 4_294_967_296.0; // 2^32
    raw_bits as f64 / Q32_SCALE
}

// ---------------------------------------------------------------------------
// MatchFrameDto — re-exported from fw-match-sim (where it actually lives,
// so the `dump_frames` binary can use it without inverting the dep graph).
// See `crates/fw-match-sim/src/dto.rs` for the type def + projection.
// ---------------------------------------------------------------------------

pub use fw_match_sim::{BallFrameDto, MatchFrameDto, PlayerFrameDto};

// -------------------------------------------------------------------------
// Smoke
// -------------------------------------------------------------------------

#[cfg(test)]
mod smoke {
    use super::*;
    use fw_core::Seed;

    #[test]
    fn smoke() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn dto_round_trips_player_count() {
        let s = MatchState::initial(Seed::from_u64(1));
        let dto = MatchStateDto::from_state(&s);
        assert_eq!(dto.players.len(), fw_match_sim::TOTAL_PLAYERS);
        assert_eq!(dto.tick, 0);
    }

    #[test]
    #[allow(clippy::float_arithmetic)]
    fn q32_to_f64_matches_known_value() {
        // Q32::ONE has raw bits 2^32; dividing by 2^32 gives 1.0.
        assert_eq!(q32_to_f64(1_i64 << 32), 1.0);
        assert_eq!(q32_to_f64(0), 0.0);
    }
}

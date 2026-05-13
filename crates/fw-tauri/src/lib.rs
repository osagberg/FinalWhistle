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
pub use commands::{get_dummy_state, match_frames, play_match};

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

    #[test]
    fn match_frames_tick_count_zero_returns_one_frame() {
        // Codex pre-T1-2b audit P1 pin: `tick_count = 0` is a valid
        // input. The handler returns exactly 1 frame (the initial state).
        // The Vec length contract is `tick_count + 1` everywhere.
        //
        // The command is `async fn` because Tauri requires it, but the
        // body has no `.await` — we can drive the Ready future to
        // completion synchronously via Tauri's bundled runtime.
        let frames =
            tauri::async_runtime::block_on(crate::commands::match_frames("0x1".to_string(), 0))
                .expect("match_frames");
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].tick, 0);
    }

    #[test]
    fn match_frames_returns_tick_count_plus_one_frames() {
        let frames = tauri::async_runtime::block_on(crate::commands::match_frames(
            "0xdeadbeef".to_string(),
            5,
        ))
        .expect("match_frames");
        assert_eq!(frames.len(), 6);
        assert_eq!(frames[0].tick, 0);
        assert_eq!(frames[5].tick, 5);
    }
}

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

use fw_core::Seed;
use fw_match_sim::{tick_match, MatchState};
use serde::Serialize;

// -------------------------------------------------------------------------
// Frontend DTOs — what the SolidJS side sees
// -------------------------------------------------------------------------

/// A flattened match-state DTO suitable for the frontend. Q32 values are
/// rendered as f64 here — viewer-side use only; the canonical sim still
/// keeps Q32 internally.
///
/// The renderer is free to interpolate / animate / smooth these; the
/// canonical state is always re-derivable from `(seed, tick_count)`.
#[derive(Debug, Clone, Serialize)]
pub struct MatchStateDto {
    pub seed_hex: String,
    pub tick: i64,
    pub home_score: u8,
    pub away_score: u8,
    pub players: Vec<PlayerDto>,
    pub ball: BallDto,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlayerDto {
    pub slot: u8,
    pub pos_x: f64,
    pub pos_y: f64,
    pub vel_x: f64,
    pub vel_y: f64,
}

#[derive(Debug, Clone, Serialize)]
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

// -------------------------------------------------------------------------
// Tauri command handlers
// -------------------------------------------------------------------------

/// `play_match(seed_hex, tick_count)` — run a smoke match end-to-end and
/// return the final state as a DTO.
///
/// Phase-0 stub. T1+ adds streaming progress events + early-termination on
/// stoppage time. The `seed_hex` parameter accepts `"0x..."` or bare hex.
#[tauri::command]
pub async fn play_match(seed_hex: String, tick_count: u32) -> Result<MatchStateDto, String> {
    let trimmed = seed_hex.trim_start_matches("0x");
    let raw = u64::from_str_radix(trimmed, 16)
        .map_err(|e| format!("invalid seed_hex {seed_hex:?}: {e}"))?;
    let seed = Seed::from_u64(raw);

    let mut state = MatchState::initial(seed);
    for _ in 0..tick_count {
        state = tick_match(state);
    }

    Ok(MatchStateDto::from_state(&state))
}

/// `get_dummy_state()` — return a fresh `MatchState::initial(seed=1)` as
/// the smallest live IPC round-trip the frontend can render against. Used
/// by the Phase-0 / T0-2 scaffold smoke test in the SolidJS side.
#[tauri::command]
pub async fn get_dummy_state() -> Result<MatchStateDto, String> {
    let state = MatchState::initial(Seed::from_u64(1));
    Ok(MatchStateDto::from_state(&state))
}

// -------------------------------------------------------------------------
// Smoke
// -------------------------------------------------------------------------

#[cfg(test)]
mod smoke {
    use super::*;

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

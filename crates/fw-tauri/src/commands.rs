//! Tauri command handlers — the IPC entry points the frontend invokes.
//!
//! Lives in a separate module from `lib.rs` because of a known Tauri 2
//! limitation: `#[tauri::command]` on a `pub` function inside `lib.rs`
//! produces `E0255 __cmd__<name> defined multiple times` (the macro
//! generates `pub use __cmd__<name>` AND uses the name locally, which
//! clashes inside the crate root). Moving the commands one level down
//! into `mod commands` sidesteps the clash entirely.
//!
//! Reference: <https://github.com/tauri-apps/tauri/discussions/4665>
//!
//! ## What lives here
//!
//! - One `#[tauri::command] pub <async> fn` per IPC surface.
//! - All command bodies call into the sim crates (`fw-match-sim` etc.)
//!   for canonical work; this module is glue + DTO marshalling only.
//!
//! ## What does NOT live here
//!
//! - DTO type definitions (`MatchStateDto`, `PlayerDto`, `BallDto`) — those
//!   stay in `lib.rs` so other consumers can import them without pulling
//!   the command surface.
//! - The `q32_to_f64` projection helper — also in `lib.rs`.

use fw_core::Seed;
use fw_match_sim::{MatchState, tick_match};

use crate::{MatchFrameDto, MatchStateDto};

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

/// `match_frames(seed_hex, tick_count)` — produce a sequence of per-tick
/// frames for the dev-tier 2D tactical board (T1-2a per ADR-0007 Layer 2).
///
/// Returns `Vec<MatchFrameDto>` of length `tick_count + 1` (one entry per
/// tick from `0` through `tick_count` inclusive — the inclusive endpoint
/// gives the renderer a frame to display when the scrubber is parked at
/// the end). Frames are produced by running `tick_match` deterministically
/// for the given seed; the result is byte-identical across runs.
///
/// The frontend `TauriFrameSource` impl calls this command; the
/// `HttpFrameSource` impl reads JSON produced by the
/// `crates/fw-match-sim/src/bin/dump_frames.rs` binary (which uses the
/// same `MatchFrameDto` shape via the camelCase serde convention).
/// Note on `tick_count` semantics: `tick_count = 0` returns a single
/// frame (the initial state at tick 0). The returned Vec length is
/// always `tick_count + 1`. Codex pre-T1-2b audit P1 pin: the
/// `tick_count_zero_returns_one_frame` test below makes this explicit.
#[tauri::command]
pub async fn match_frames(seed_hex: String, tick_count: u32) -> Result<Vec<MatchFrameDto>, String> {
    let trimmed = seed_hex.trim_start_matches("0x");
    let raw = u64::from_str_radix(trimmed, 16)
        .map_err(|e| format!("invalid seed_hex {seed_hex:?}: {e}"))?;
    let seed = Seed::from_u64(raw);

    let mut state = MatchState::initial(seed);
    // tick_count + 1 frames: index 0 is the initial state, index
    // tick_count is the state after `tick_count` advances.
    let total = (tick_count as usize).saturating_add(1);
    let mut frames = Vec::with_capacity(total);
    frames.push(MatchFrameDto::from_state(&state));
    for _ in 0..tick_count {
        state = tick_match(state);
        frames.push(MatchFrameDto::from_state(&state));
    }
    Ok(frames)
}

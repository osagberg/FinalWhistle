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

use crate::MatchStateDto;

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

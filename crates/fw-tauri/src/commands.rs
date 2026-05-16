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

use std::path::Path;

use fw_content::ContentStore;
use fw_core::Seed;
use fw_match_sim::{MatchState, tick_match};

use crate::{MatchFrameDto, MatchStateDto};

/// Default path to the source-content directory, relative to the working directory.
///
/// **T1-5 follow-up (Codex T1-11 P0 fix-pass)**: this is a stop-gap.
/// The proper fix lifts `ContentStore` into `AppState` (a Tauri-managed
/// resource constructed once at app startup) and threads `&AppState`
/// through every command handler — avoiding the per-command load cost
/// plus supporting Tauri's resource resolver for bundled-app paths (T5-1
/// Steam distribution). For T1-11 the relative path works for
/// `pnpm tauri dev` invoked from the project root (the dev workflow we
/// ship today); the production-app bundle path is T5-1's concern.
const DEFAULT_CONTENT_PATH: &str = "content";

/// Env-var override for the content directory path. Used by integration
/// tests (which run from `crates/fw-tauri/` and need to point at the
/// workspace-root `content/` directory). Production reads the default.
const CONTENT_PATH_ENV: &str = "FW_CONTENT_PATH";

/// Load `ContentStore` for the duration of one command invocation.
///
/// **T1-5 follow-up**: replace with `AppState`-cached lookup. Today this
/// reloads RON files on every command call (~10ms per invocation; fine
/// for T1's dev workflow which calls play_match / match_frames at most
/// a few times per session).
///
/// Codex T1-11 P0 fix-pass: the prior code passed `&BTreeMap::new()` for
/// `sig_definitions` and used `MatchState::initial(seed)` — making
/// signatures structurally unreachable in the Tauri IPC path. This loader
/// plus `initial_with_content` projection wire the real signature dispatcher
/// into `play_match` plus `match_frames` so the dev-board scrubber plus future
/// play-match UI see real signature firings.
///
/// Path resolution: `FW_CONTENT_PATH` env var if set (used by integration
/// tests), else `DEFAULT_CONTENT_PATH` ("content"). Env reads are not on
/// the Sim/RULES.md §3 banned list — env-driven configuration is sane
/// here (this is the IPC boundary, not a sim crate).
fn load_content_for_command() -> Result<ContentStore, String> {
    let path = std::env::var(CONTENT_PATH_ENV).unwrap_or_else(|_| DEFAULT_CONTENT_PATH.to_string());
    ContentStore::load_sources(Path::new(&path))
        .map_err(|e| format!("ContentStore::load_sources({path:?}) failed: {e}"))
}

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

    // T1-11 fix-pass (Codex code-reviewer P0): load real ContentStore + use
    // initial_with_content so the signature dispatcher fires in the IPC path.
    // Prior code used MatchState::initial + &BTreeMap::new() → signatures
    // structurally unreachable in production (only test fixtures fired them).
    let content = load_content_for_command()?;
    let mut state = MatchState::initial_with_content(seed, &content)
        .map_err(|e| format!("MatchState::initial_with_content failed: {e}"))?;
    for _ in 0..tick_count {
        state = tick_match(state, &content.signature_definitions);
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

    // T1-11 fix-pass (Codex code-reviewer P0): load real ContentStore + use
    // initial_with_content + thread sig_definitions through tick_match so
    // dev-board scrubber sees signature firings. Same fix as play_match.
    let content = load_content_for_command()?;
    let mut state = MatchState::initial_with_content(seed, &content)
        .map_err(|e| format!("MatchState::initial_with_content failed: {e}"))?;
    // tick_count + 1 frames: index 0 is the initial state, index
    // tick_count is the state after `tick_count` advances.
    let total = (tick_count as usize).saturating_add(1);
    let mut frames = Vec::with_capacity(total);
    frames.push(MatchFrameDto::from_state(&state));
    for _ in 0..tick_count {
        state = tick_match(state, &content.signature_definitions);
        frames.push(MatchFrameDto::from_state(&state));
    }
    Ok(frames)
}

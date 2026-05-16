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
//! ## Testing strategy
//!
//! `tauri::State<'_, T>` is not directly constructable in unit tests —
//! only `tauri::Manager::state()` on a live App produces one. To keep tests
//! fast and framework-free, each command delegates to a corresponding
//! `_inner` free function that takes `&AppState` directly. Tests call the
//! `_inner` variant; the `#[tauri::command]` wrappers are thin forwarding
//! shells validated by the integration test in `crates/fw-tauri/tests/`.

use fw_core::Seed;
use fw_match_sim::{MatchState, tick_match};

use crate::{
    AppState, IpcError, MAX_FRAMES_PER_REQUEST, MatchFrameDto, MatchResult, MatchStateDto,
};

// ---------------------------------------------------------------------------
// Public `#[tauri::command]` wrappers (thin shells; logic in `_inner` fns)
// ---------------------------------------------------------------------------

/// `play_match(seed_hex, tick_count)` — run a match end-to-end and return
/// the final `MatchResult` DTO including the canonical hash and per-event
/// commentary.
///
/// `seed_hex` accepts `"0x..."` or bare hex. Returns `IpcError::InvalidSeed`
/// if malformed; `IpcError::MatchInitFailed` if the sim cannot start.
#[tauri::command]
pub async fn play_match(
    seed_hex: String,
    tick_count: u32,
    state: tauri::State<'_, AppState>,
) -> Result<MatchResult, IpcError> {
    play_match_inner(seed_hex, tick_count, &state).await
}

/// `get_dummy_state()` — return a fresh `MatchState::initial(seed=1)` as
/// the smallest live IPC round-trip the frontend can render against. Used
/// by the Phase-0 / T0-2 scaffold smoke test in the SolidJS side.
///
/// No AppState injection needed — this command is a pure stub. The
/// `IpcError` return type matches the other commands per Tauri/RULES.md §4
/// (uniform typed-error surface across the IPC boundary) even though this
/// stub cannot fail today.
#[tauri::command]
pub async fn get_dummy_state() -> Result<MatchStateDto, IpcError> {
    let state = MatchState::initial(Seed::from_u64(1));
    Ok(MatchStateDto::from_state(&state))
}

/// `match_frames(seed_hex, tick_count)` — produce a sequence of per-tick
/// frames for the dev-tier 2D tactical board.
///
/// Returns `Vec<MatchFrameDto>` of length `tick_count + 1` (one entry per
/// tick from `0` through `tick_count` inclusive).
///
/// Returns `IpcError::TooManyFrames` when `tick_count > MAX_FRAMES_PER_REQUEST`
/// **before** any allocation or sim work is done.
#[tauri::command]
pub async fn match_frames(
    seed_hex: String,
    tick_count: u32,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<MatchFrameDto>, IpcError> {
    match_frames_inner(seed_hex, tick_count, &state).await
}

// ---------------------------------------------------------------------------
// Inner logic (takes `&AppState`; testable without `tauri::State`)
// ---------------------------------------------------------------------------

pub async fn play_match_inner(
    seed_hex: String,
    tick_count: u32,
    state: &AppState,
) -> Result<MatchResult, IpcError> {
    let seed = parse_seed_hex(&seed_hex)?;
    // `?` exercises `From<ContentLoadError> for IpcError` (see error.rs).
    let mut sim_state = MatchState::initial_with_content(seed, state.content())?;
    for _ in 0..tick_count {
        sim_state = tick_match(sim_state, state.signature_definitions());
    }
    MatchResult::from_state(&sim_state, seed_hex, tick_count, state.content())
}

pub async fn match_frames_inner(
    seed_hex: String,
    tick_count: u32,
    state: &AppState,
) -> Result<Vec<MatchFrameDto>, IpcError> {
    // Guard fires BEFORE any allocation or sim work (acceptance criterion 4).
    if tick_count > MAX_FRAMES_PER_REQUEST {
        return Err(IpcError::TooManyFrames {
            requested: tick_count,
            max: MAX_FRAMES_PER_REQUEST,
        });
    }

    let seed = parse_seed_hex(&seed_hex)?;
    // `?` exercises `From<ContentLoadError> for IpcError` (see error.rs).
    let mut sim_state = MatchState::initial_with_content(seed, state.content())?;

    // tick_count + 1 frames: index 0 = initial state, index tick_count = final.
    // The guard above caps tick_count at MAX_FRAMES_PER_REQUEST (= 7200), so
    // `(tick_count as usize) + 1` cannot overflow on any platform with
    // `usize >= u32` (which the workspace requires). The debug_assert pins
    // the invariant at the call site so a future relaxation of the guard
    // fails loudly here rather than silently allocating usize::MAX.
    debug_assert!(tick_count <= MAX_FRAMES_PER_REQUEST);
    let total = (tick_count as usize) + 1;
    let mut frames = Vec::with_capacity(total);
    frames.push(MatchFrameDto::from_state(&sim_state));
    for _ in 0..tick_count {
        sim_state = tick_match(sim_state, state.signature_definitions());
        frames.push(MatchFrameDto::from_state(&sim_state));
    }
    Ok(frames)
}

// ---------------------------------------------------------------------------
// Shared helper
// ---------------------------------------------------------------------------

fn parse_seed_hex(seed_hex: &str) -> Result<Seed, IpcError> {
    let trimmed = seed_hex.trim_start_matches("0x");
    let raw = u64::from_str_radix(trimmed, 16).map_err(|e| IpcError::InvalidSeed {
        input: seed_hex.to_string(),
        reason: e.to_string(),
    })?;
    Ok(Seed::from_u64(raw))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use fw_core::Seed;
    use fw_match_sim::{MatchState, tick_match};

    use super::*;
    use crate::state::AppState;

    fn workspace_content_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("content")
    }

    fn test_app_state() -> AppState {
        AppState::new(&workspace_content_path()).expect("AppState::new in test")
    }

    // ---- parse_seed_hex ----

    #[test]
    fn parse_seed_hex_accepts_0x_prefix() {
        assert_eq!(
            parse_seed_hex("0xdeadbeef").expect("parse").to_u64(),
            0xDEAD_BEEF
        );
    }

    #[test]
    fn parse_seed_hex_accepts_bare_hex() {
        assert_eq!(parse_seed_hex("1").expect("parse bare").to_u64(), 1);
    }

    #[test]
    fn parse_seed_hex_rejects_invalid() {
        let err = parse_seed_hex("0xggg").expect_err("should fail");
        match err {
            IpcError::InvalidSeed { input, .. } => assert!(input.contains("ggg")),
            other => panic!("wrong variant: {other:?}"),
        }
    }

    // ---- MAX_FRAMES guard (chunk 3 acceptance criterion) ----

    #[test]
    fn match_frames_over_max_returns_too_many_frames_before_alloc() {
        let state = test_app_state();
        let err = tauri::async_runtime::block_on(match_frames_inner(
            "0x1".to_string(),
            MAX_FRAMES_PER_REQUEST + 1,
            &state,
        ))
        .expect_err("should reject over-max tick_count");
        match err {
            IpcError::TooManyFrames { requested, max } => {
                assert_eq!(requested, MAX_FRAMES_PER_REQUEST + 1);
                assert_eq!(max, MAX_FRAMES_PER_REQUEST);
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn match_frames_tick_count_zero_returns_one_frame() {
        let state = test_app_state();
        let frames =
            tauri::async_runtime::block_on(match_frames_inner("0x1".to_string(), 0, &state))
                .expect("zero ticks");
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].tick, 0);
    }

    #[test]
    fn match_frames_returns_tick_count_plus_one() {
        let state = test_app_state();
        let frames =
            tauri::async_runtime::block_on(match_frames_inner("0xdeadbeef".to_string(), 5, &state))
                .expect("5 ticks");
        assert_eq!(frames.len(), 6);
        assert_eq!(frames[0].tick, 0);
        assert_eq!(frames[5].tick, 5);
    }

    // ---- play_match ----

    #[test]
    fn play_match_returns_match_result_with_blake3_hash() {
        let state = test_app_state();
        let result = tauri::async_runtime::block_on(play_match_inner(
            "0xdeadbeefdeadbeef".to_string(),
            60,
            &state,
        ))
        .expect("play_match");
        assert!(result.canonical_hash.starts_with("blake3:"));
        assert_eq!(result.canonical_hash.len(), 7 + 64);
        assert_eq!(result.tick_count, 60);
        assert_eq!(result.seed_hex, "0xdeadbeefdeadbeef");
    }

    #[test]
    fn play_match_canonical_hash_matches_independent_computation() {
        let state = test_app_state();

        // Independent BLAKE3 computation.
        let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);
        let mut sim_state = MatchState::initial_with_content(seed, state.content()).expect("init");
        for _ in 0..60 {
            sim_state = tick_match(sim_state, state.signature_definitions());
        }
        let bytes = sim_state.encode_canonical();
        let hash_bytes: [u8; 32] = blake3::hash(&bytes).into();
        let expected = format!(
            "blake3:{}",
            hash_bytes
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );

        // IPC inner path.
        let result = tauri::async_runtime::block_on(play_match_inner(
            "0xdeadbeefdeadbeef".to_string(),
            60,
            &state,
        ))
        .expect("play_match");

        assert_eq!(
            result.canonical_hash, expected,
            "IPC canonical_hash must match independent BLAKE3 computation"
        );
    }

    #[test]
    fn play_match_commentary_preview_length_matches_events() {
        let state = test_app_state();
        let result = tauri::async_runtime::block_on(play_match_inner(
            "0xdeadbeefdeadbeef".to_string(),
            60,
            &state,
        ))
        .expect("play_match");
        assert_eq!(result.commentary_preview.len(), result.match_events.len());
    }

    // ---- IpcError serialization (acceptance criterion 3) ----

    #[test]
    fn too_many_frames_error_serializes_as_discriminated_union() {
        let err = IpcError::TooManyFrames {
            requested: MAX_FRAMES_PER_REQUEST + 1,
            max: MAX_FRAMES_PER_REQUEST,
        };
        let json = serde_json::to_string(&err).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(v["kind"], "tooManyFrames");
        assert_eq!(v["requested"], MAX_FRAMES_PER_REQUEST + 1);
        assert_eq!(v["max"], MAX_FRAMES_PER_REQUEST);
    }
}

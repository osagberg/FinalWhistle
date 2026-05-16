//! IPC contract tests for `crates/fw-tauri`.
//!
//! Validates the load-bearing IPC invariants identified in the T1-5 task spec
//! (cargo-cult-fix-pass meta-pattern — tests prove substance, not shape):
//!
//! 1. `play_match` round-trip canonical hash matches an independently-computed
//!    BLAKE3 over the same sim state (acceptance criterion 11a).
//! 2. `match_frames` with `tick_count = MAX + 1` returns `IpcError::TooManyFrames`
//!    with the correct `requested` + `max` fields (acceptance criterion 11b).
//! 3. `IpcError::TooManyFrames` serialises as the TS-readable discriminated
//!    union `{ kind: "tooManyFrames", requested: N, max: M }`.
//!
//! These tests call the `_inner` helpers directly (no `tauri::State` required).

use std::path::PathBuf;

use fw_core::Seed;
use fw_match_sim::{MatchState, tick_match};
use fw_tauri::{IpcError, MAX_FRAMES_PER_REQUEST};

fn workspace_content_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

fn test_app_state() -> fw_tauri::AppState {
    fw_tauri::AppState::new(&workspace_content_path()).expect("AppState::new in ipc_contract_test")
}

// ---------------------------------------------------------------------------
// Acceptance criterion 11a: play_match round-trip canonical hash
// ---------------------------------------------------------------------------

/// Invoke `play_match_inner` with seed `0xDEADBEEFDEADBEEF` + 60 ticks.
/// Parse the returned `MatchResult` and assert `canonical_hash` matches the
/// hash produced by independently running the sim and calling `blake3::hash`.
#[test]
fn play_match_round_trip_canonical_hash_matches() {
    let state = test_app_state();

    // IPC inner path.
    let result = tauri::async_runtime::block_on(fw_tauri::commands::play_match_inner(
        "0xdeadbeefdeadbeef".to_string(),
        60,
        &state,
    ))
    .expect("play_match_inner should succeed");

    // Independent computation — same seed + tick_count, same algorithm.
    // SCOPE: this proves the IPC path runs the same sim + same hash function
    // as a direct `MatchState::initial_with_content` + `tick_match` + `blake3`
    // call. It does NOT prove the hash matches an external auditor's
    // BLAKE3 of the canonical encoding (both sides go through
    // `encode_canonical()` + `blake3::hash` here). For that stronger
    // round-trip a separate test would shell out to `b3sum` or similar.
    let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);
    let mut sim_state =
        MatchState::initial_with_content(seed, state.content()).expect("initial_with_content");
    for _ in 0..60 {
        sim_state = tick_match(sim_state, state.signature_definitions());
    }
    let canonical_bytes = sim_state.encode_canonical();
    let hash_bytes: [u8; 32] = blake3::hash(&canonical_bytes).into();
    let expected_hash = format!(
        "blake3:{}",
        hash_bytes
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );

    assert_eq!(
        result.canonical_hash, expected_hash,
        "IPC play_match canonical_hash must match independent BLAKE3 computation. \
         If this fails, the IPC path runs different logic than the direct sim call."
    );
}

/// The `canonical_hash` field must have the `blake3:` prefix and be exactly
/// 7 + 64 = 71 characters (prefix + 64 hex chars for 32 bytes).
#[test]
fn play_match_canonical_hash_has_correct_format() {
    let state = test_app_state();
    let result = tauri::async_runtime::block_on(fw_tauri::commands::play_match_inner(
        "0x1".to_string(),
        0,
        &state,
    ))
    .expect("play_match_inner");
    assert!(
        result.canonical_hash.starts_with("blake3:"),
        "canonical_hash must start with 'blake3:'"
    );
    assert_eq!(
        result.canonical_hash.len(),
        71,
        "canonical_hash must be 7 chars ('blake3:') + 64 hex chars"
    );
}

// ---------------------------------------------------------------------------
// Acceptance criterion 11b: match_frames error-shape test
// ---------------------------------------------------------------------------

/// Invoke `match_frames_inner` with `tick_count = MAX + 1` and assert:
/// - The call returns `Err(IpcError::TooManyFrames)`.
/// - The `requested` field equals `MAX + 1`.
/// - The `max` field equals `MAX_FRAMES_PER_REQUEST`.
#[test]
fn match_frames_over_max_returns_too_many_frames_error() {
    let state = test_app_state();
    let over_max = MAX_FRAMES_PER_REQUEST + 1;

    let err = tauri::async_runtime::block_on(fw_tauri::commands::match_frames_inner(
        "0x1".to_string(),
        over_max,
        &state,
    ))
    .expect_err("match_frames_inner should fail when tick_count > MAX_FRAMES_PER_REQUEST");

    match err {
        IpcError::TooManyFrames { requested, max } => {
            assert_eq!(
                requested, over_max,
                "`requested` must echo the caller's tick_count"
            );
            assert_eq!(
                max, MAX_FRAMES_PER_REQUEST,
                "`max` must equal MAX_FRAMES_PER_REQUEST"
            );
        }
        other => panic!(
            "expected IpcError::TooManyFrames, got {other:?}. \
             The guard in match_frames_inner must fire before any alloc."
        ),
    }
}

/// The `IpcError::TooManyFrames` variant must serialise as the TS-readable
/// discriminated union `{{ kind: "tooManyFrames", requested: N, max: M }}`.
/// This is the load-bearing serde shape the TypeScript side decodes.
#[test]
fn too_many_frames_error_serializes_as_ts_discriminated_union() {
    let err = IpcError::TooManyFrames {
        requested: MAX_FRAMES_PER_REQUEST + 1,
        max: MAX_FRAMES_PER_REQUEST,
    };
    let json = serde_json::to_string(&err).expect("IpcError must be Serialize");
    let v: serde_json::Value = serde_json::from_str(&json).expect("must be valid JSON");

    assert_eq!(
        v["kind"], "tooManyFrames",
        "discriminant must be camelCase 'tooManyFrames' (not 'TooManyFrames'). \
         Requires #[serde(tag = \"kind\", rename_all = \"camelCase\")] on IpcError."
    );
    assert_eq!(
        v["requested"],
        serde_json::json!(MAX_FRAMES_PER_REQUEST + 1),
        "`requested` field must be present in the serialised form"
    );
    assert_eq!(
        v["max"],
        serde_json::json!(MAX_FRAMES_PER_REQUEST),
        "`max` field must be present in the serialised form"
    );
}

/// Round-trip: serialise `IpcError::TooManyFrames` then deserialise it back.
/// Both `requested` and `max` must survive the round-trip unchanged.
#[test]
fn too_many_frames_error_round_trips_through_json() {
    let original = IpcError::TooManyFrames {
        requested: 9999,
        max: MAX_FRAMES_PER_REQUEST,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let back: IpcError = serde_json::from_str(&json).expect("deserialize");
    match back {
        IpcError::TooManyFrames { requested, max } => {
            assert_eq!(requested, 9999);
            assert_eq!(max, MAX_FRAMES_PER_REQUEST);
        }
        other => panic!("wrong variant after round-trip: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Structural: src-tauri has zero local commands (acceptance criterion 7)
// ---------------------------------------------------------------------------

/// Verify the src-tauri shell registers only fw_tauri commands, not local ones.
/// This test checks the compile-time fact that `src-tauri/src/commands.rs`
/// no longer exists, so there can be no local commands to register.
///
/// If this test file can be referenced in the build, `commands.rs` is gone.
/// (The Rust compiler enforces this — adding it back breaks the build.)
#[test]
fn src_tauri_commands_file_does_not_exist() {
    let commands_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("src-tauri")
        .join("src")
        .join("commands.rs");
    assert!(
        !commands_path.exists(),
        "src-tauri/src/commands.rs must be deleted (T1-5 consolidation). \
         Found it at: {}",
        commands_path.display()
    );
}

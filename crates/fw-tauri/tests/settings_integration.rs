//! T4-6a integration tests — settings IPC commands.
//!
//! Tests call the `_inner` helpers directly (no `tauri::State` plumbing
//! needed). Each test injects a temp-dir settings path via
//! `AppState::new_with_settings_path` so no live Tauri runtime is required
//! and tests can run concurrently without clobbering each other.

use std::path::PathBuf;

use fw_core::Seed;
use fw_save::load_settings_envelope;
use fw_tauri::AppSettingsDto;
use fw_tauri::commands::{get_settings_inner, set_settings_inner};
use fw_tauri::state::AppState;

fn workspace_content_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

/// Build a fresh `AppState` with a temp-dir settings path.
///
/// Each call creates a fresh temp directory so tests can run in parallel
/// without sharing a settings file.
fn test_state_with_temp_settings() -> (AppState, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let settings_path = dir.path().join("settings.fwcfg");
    let state = AppState::new_with_settings_path(
        &workspace_content_path(),
        Seed::from_u64(0xCAFE_BABE_DEAD_BEEF),
        settings_path,
    )
    .expect("AppState::new_with_settings_path");
    (state, dir)
}

// ---------------------------------------------------------------------------
// AC1: get_settings on a missing file returns defaults
// ---------------------------------------------------------------------------

#[test]
fn get_settings_on_missing_file_returns_defaults() {
    let (state, _dir) = test_state_with_temp_settings();

    // No file has been written — settings_path does not exist yet.
    assert!(
        !state.settings_path().exists(),
        "precondition: settings file must not exist before first write"
    );

    let dto = get_settings_inner(&state).expect("get_settings_inner");

    // Default is light theme + no reduce-motion.
    let json = serde_json::to_value(&dto).expect("serialize");
    assert_eq!(json["theme"], "light", "default theme must be 'light'");
    assert_eq!(
        json["reduceMotion"], false,
        "default reduceMotion must be false"
    );
}

// ---------------------------------------------------------------------------
// AC2: set_then_get round-trips the written values
// ---------------------------------------------------------------------------

#[test]
fn set_then_get_round_trips() {
    let (state, _dir) = test_state_with_temp_settings();

    let settings = AppSettingsDto {
        theme: fw_tauri::ThemePrefDto::Dark,
        reduce_motion: true,
    };

    set_settings_inner(settings, &state).expect("set_settings_inner");

    let loaded = get_settings_inner(&state).expect("get_settings_inner after set");

    let json = serde_json::to_value(&loaded).expect("serialize");
    assert_eq!(json["theme"], "dark", "persisted theme must be 'dark'");
    assert_eq!(
        json["reduceMotion"], true,
        "persisted reduceMotion must be true"
    );
}

// ---------------------------------------------------------------------------
// AC3: settings persist across a fresh AppState at the same path
// ---------------------------------------------------------------------------

#[test]
fn settings_persist_across_appstate() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let settings_path = dir.path().join("settings.fwcfg");

    // AppState A: write dark + reduce_motion.
    {
        let state_a = AppState::new_with_settings_path(
            &workspace_content_path(),
            Seed::from_u64(0x1234),
            settings_path.clone(),
        )
        .expect("state_a");

        set_settings_inner(
            AppSettingsDto {
                theme: fw_tauri::ThemePrefDto::Dark,
                reduce_motion: true,
            },
            &state_a,
        )
        .expect("set via state_a");
    }

    // AppState B: read from the SAME path — settings must survive across
    // separate AppState instances (the file is the source of truth).
    let state_b = AppState::new_with_settings_path(
        &workspace_content_path(),
        Seed::from_u64(0x1234),
        settings_path,
    )
    .expect("state_b");

    let loaded = get_settings_inner(&state_b).expect("get via state_b");
    let json = serde_json::to_value(&loaded).expect("serialize");
    assert_eq!(
        json["theme"], "dark",
        "settings must survive across separate AppState instances"
    );
    assert_eq!(
        json["reduceMotion"], true,
        "reduce_motion must survive across separate AppState instances"
    );
}

// ---------------------------------------------------------------------------
// BK-E-42: get_settings on a missing file still returns defaults (TOCTOU fix
// regression — the NotFound path must behave identically to the old exists() check)
// ---------------------------------------------------------------------------

#[test]
fn get_settings_missing_file_returns_defaults_without_toctou() {
    // This test is intentionally identical in observable outcome to
    // get_settings_on_missing_file_returns_defaults, but it validates that the
    // new fs::read + NotFound path (not exists()+read) preserves first-run behaviour.
    let (state, _dir) = test_state_with_temp_settings();

    assert!(
        !state.settings_path().exists(),
        "precondition: no settings file"
    );

    let dto = get_settings_inner(&state).expect("must succeed on missing file");
    let json = serde_json::to_value(&dto).expect("serialize");
    assert_eq!(json["theme"], "light", "first-run default theme is light");
    assert_eq!(
        json["reduceMotion"], false,
        "first-run default reduceMotion is false"
    );
}

// ---------------------------------------------------------------------------
// BK-E-41: set_settings round-trip — written file is fully decodable (atomic write)
// ---------------------------------------------------------------------------

#[test]
fn set_settings_written_file_is_fully_decodable() {
    // After set_settings_inner, the file on disk must be a valid SettingsEnvelope.
    // A partially-clobbered file (non-atomic write interrupted mid-stream) would
    // decode as garbage; the atomic rename guarantees either the old or the new
    // file is present, never a partial splice.
    let (state, _dir) = test_state_with_temp_settings();

    let settings = fw_tauri::AppSettingsDto {
        theme: fw_tauri::ThemePrefDto::Dark,
        reduce_motion: true,
    };

    set_settings_inner(settings, &state).expect("set_settings_inner");

    // Read the raw bytes and decode through the full envelope path — the same
    // path get_settings_inner uses. If the file is truncated or misaligned this
    // decode fails.
    let bytes = std::fs::read(state.settings_path()).expect("settings file must exist after write");
    load_settings_envelope(&bytes).expect("written file must decode as valid SettingsEnvelope");
}

// ---------------------------------------------------------------------------
// AC4: set followed by another set — second write wins
// ---------------------------------------------------------------------------

#[test]
fn second_set_overwrites_first() {
    let (state, _dir) = test_state_with_temp_settings();

    set_settings_inner(
        AppSettingsDto {
            theme: fw_tauri::ThemePrefDto::Dark,
            reduce_motion: false,
        },
        &state,
    )
    .expect("first set");

    set_settings_inner(
        AppSettingsDto {
            theme: fw_tauri::ThemePrefDto::Light,
            reduce_motion: true,
        },
        &state,
    )
    .expect("second set");

    let loaded = get_settings_inner(&state).expect("get after two sets");
    let json = serde_json::to_value(&loaded).expect("serialize");
    assert_eq!(json["theme"], "light", "second write must overwrite first");
    assert_eq!(
        json["reduceMotion"], true,
        "second write must overwrite first"
    );
}

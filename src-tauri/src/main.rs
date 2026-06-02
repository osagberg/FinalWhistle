// Hide the console window in non-debug Windows builds. Tauri convention.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};

use tauri::Manager;

fn main() {
    // Content path resolution. `FW_CONTENT_PATH` wins (integration tests set
    // it from per-crate CWDs). Otherwise prefer a CWD-relative `content/` if it
    // actually exists (running from the workspace root); else fall back to the
    // workspace-root `content/` resolved from the build-time manifest dir —
    // `tauri dev` launches this binary with CWD = `src-tauri/`, where a bare
    // `content` would miss `content/sources` and panic at startup.
    // (Shipped/bundled builds resolve content via Tauri's resource dir — that
    // wiring is T5 bundling, not dev.)
    let content_path = std::env::var("FW_CONTENT_PATH").unwrap_or_else(|_| {
        let cwd_rel = PathBuf::from("content");
        if cwd_rel.join("sources").is_dir() {
            cwd_rel.to_string_lossy().into_owned()
        } else {
            // CARGO_MANIFEST_DIR = .../src-tauri; the workspace root is its parent.
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../content")
                .to_string_lossy()
                .into_owned()
        }
    });

    // Forward Rust log output to the dev console via tauri-plugin-log. Cheap
    // DX win; disabled in release builds via plugin config.
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        // Build + inject AppState inside `.setup()` so the Tauri `AppHandle`
        // is available to resolve the OS app-config dir — T4-6a settings
        // persist to `<app-config-dir>/settings.fwcfg`, NOT a CWD-relative
        // path (which would silently drift with the launch directory).
        .setup(move |app| {
            let settings_path: PathBuf = match app.path().app_config_dir() {
                Ok(dir) => dir.join("settings.fwcfg"),
                Err(e) => {
                    // Loud fallback (NOT silent): if the platform can't give
                    // us an app-config dir, log it and persist to the working
                    // directory so the app still runs.
                    log::error!(
                        "could not resolve the app-config dir ({e}); settings \
                         will persist to ./settings.fwcfg in the working \
                         directory instead"
                    );
                    PathBuf::from("settings.fwcfg")
                }
            };
            let app_state =
                fw_tauri::AppState::new_with_settings_file(Path::new(&content_path), settings_path)
                    .expect("Failed to load ContentStore — check that content/ directory exists");
            app.manage(app_state);
            Ok(())
        })
        // Wire the IPC command surface. All commands live in fw-tauri::commands.
        // src-tauri has ZERO local commands (T1-5 consolidation; Codex T0 Imp #10).
        .invoke_handler(tauri::generate_handler![
            fw_tauri::commands::play_match,
            fw_tauri::commands::match_frames,
            fw_tauri::commands::get_backend_handshake,
            fw_tauri::commands::advance_week,
            fw_tauri::commands::play_fixtures,
            fw_tauri::commands::get_standings,
            fw_tauri::commands::get_fixtures,
            fw_tauri::commands::get_squad,
            fw_tauri::commands::get_player_detail,
            fw_tauri::commands::advance_season,
            fw_tauri::commands::get_career_overview,
            // T4-5a: live-match command quintet (ADR-0004 §1)
            fw_tauri::commands::start_live_match,
            fw_tauri::commands::step_live_match,
            fw_tauri::commands::get_match_snapshot,
            fw_tauri::commands::finish_live_match,
            fw_tauri::commands::apply_match_command,
            // T4-6a: settings persistence
            fw_tauri::commands::get_settings,
            fw_tauri::commands::set_settings,
            // T4-2.5b: career roster
            fw_tauri::commands::get_roster_for_club,
            // T4-2.5f: scouting
            fw_tauri::commands::get_scout_report,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

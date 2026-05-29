// Hide the console window in non-debug Windows builds. Tauri convention.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::Path;

fn main() {
    // Content path resolution: FW_CONTENT_PATH env var (used by integration
    // tests that run from per-crate CWDs) or the default "content" relative
    // to the app's working directory (`pnpm tauri dev` from project root).
    let content_path = std::env::var("FW_CONTENT_PATH").unwrap_or_else(|_| "content".to_string());
    let app_state = fw_tauri::AppState::new(Path::new(&content_path))
        .expect("Failed to load ContentStore — check that content/ directory exists");

    // Forward Rust log output to the dev console via tauri-plugin-log. Cheap
    // DX win; disabled in release builds via plugin config.
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        // Inject AppState so all command handlers receive it via
        // `tauri::State<'_, AppState>` without loading ContentStore per call.
        .manage(app_state)
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Hide the console window in non-debug Windows builds. Tauri convention.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    // Forward Rust log output to the dev console via tauri-plugin-log. Cheap
    // DX win; disabled in release builds via plugin config.
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        // Wire the IPC command surface. Two paths here:
        //   1. (preferred) `fw_tauri::generate_handler!()` — once the workspace
        //      `fw-tauri` crate exports a `generate_handler!` macro that pulls
        //      in every #[tauri::command] from the sim, content, memory layers,
        //      this is the single line that wires the whole game to the UI.
        //   2. (current) local placeholder commands in `commands::*`. These
        //      return stub `MatchResult`s and `LeagueStanding`s shaped to match
        //      what `fw-tauri` will return when T0-2 → T1-5 lands. The frontend
        //      can develop against the placeholder types and require no changes
        //      when the real backend lands.
        //
        // When fw-tauri lands its handler macro, replace the `.invoke_handler`
        // call below with `fw_tauri::generate_handler()`.
        .invoke_handler(tauri::generate_handler![
            commands::get_dummy_state,
            commands::play_match,
            commands::get_league_standings,
            commands::get_squad,
            commands::list_fixtures,
            // T1-2a: fw_tauri owns the real match_frames implementation (the
            // src-tauri local commands.rs is placeholder-tier until T1-5
            // consolidation per Codex Imp #10). match_frames feeds the
            // dev-tier 2D tactical board via Tauri IPC.
            fw_tauri::commands::match_frames,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

//! `fw-dev-server` — DEV-ONLY HTTP bridge.
//!
//! Exposes every `fw-tauri` command over a local HTTP server at
//! `127.0.0.1:1422`. Intended for use with the Vite dev server's proxy so the
//! browser-preview frontend can drive the REAL backend without a live Tauri
//! runtime.
//!
//! ## Route contract
//!
//! `POST /cmd/:command`
//! Request body = JSON args object (camelCase, matching what the frontend sends).
//! Response on success = HTTP 200 + DTO as JSON.
//! Response on `IpcError` = HTTP 422 + `IpcError` as JSON.
//!
//! ## NEVER put this in the production app dep tree.
//!
//! `src-tauri/Cargo.toml` must NOT depend on `fw-dev-server`. This crate
//! brings in `axum` and `tokio::full`, which would bloat the Tauri bundle.
//!
//! ## Settings / save path
//!
//! Uses `target/dev-harness/` under the workspace root so it NEVER clobbers a
//! real career save. The path is created at startup if absent.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::Value;

use fw_tauri::commands::{
    advance_season_inner, advance_week_inner, apply_match_command_inner, finish_live_match_inner,
    get_career_overview_inner, get_fixtures_inner, get_match_snapshot_inner,
    get_player_detail_inner, get_press_inbox_inner, get_roster_for_club_inner,
    get_scout_report_inner, get_settings_inner, get_squad_inner, get_squad_roster_inner,
    get_standings_inner, load_career_inner, match_frames_inner, play_fixtures_inner,
    play_match_inner, save_career_inner, set_settings_inner, start_live_match_inner,
    step_live_match_inner,
};
use fw_tauri::live_match::types::{MatchCommand, MatchHandle};
use fw_tauri::{AppSettingsDto, AppState};

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

type SharedState = Arc<AppState>;

// ---------------------------------------------------------------------------
// Per-command arg structs
//
// These mirror the arg shapes the frontend sends (camelCase). Each struct
// deserialises from the JSON request body for commands that take arguments.
// Zero-arg commands use `()` / ignore the body.
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayMatchArgs {
    seed_hex: String,
    tick_count: u32,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatchFramesArgs {
    seed_hex: String,
    tick_count: u32,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetFixturesArgs {
    club_id: u32,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetPlayerDetailArgs {
    player_id: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetRosterForClubArgs {
    club_id: u32,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetScoutReportArgs {
    player_id: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartLiveMatchArgs {
    seed_hex: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StepLiveMatchArgs {
    handle: MatchHandle,
    ticks: u32,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetMatchSnapshotArgs {
    handle: MatchHandle,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FinishLiveMatchArgs {
    handle: MatchHandle,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplyMatchCommandArgs {
    handle: MatchHandle,
    command: MatchCommand,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetSettingsArgs {
    settings: AppSettingsDto,
}

// ---------------------------------------------------------------------------
// HTTP error response helper
// ---------------------------------------------------------------------------

/// Serialise an `IpcError` into a 422 Unprocessable Entity response, mirroring
/// the shape Tauri emits on the error path so the frontend's existing
/// `IpcError` narrowing works unchanged.
fn ipc_err(e: fw_tauri::IpcError) -> Response {
    let body = serde_json::to_value(&e).unwrap_or(Value::String(e.to_string()));
    (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
}

// ---------------------------------------------------------------------------
// Dispatch handler
// ---------------------------------------------------------------------------

/// `POST /cmd/:command` — single entry point; dispatch on `:command`.
///
/// Returns HTTP 200 + DTO JSON on success, HTTP 422 + IpcError JSON on error,
/// HTTP 404 for unknown command names.
async fn dispatch(
    AxumPath(command): AxumPath<String>,
    State(state): State<SharedState>,
    body: axum::body::Bytes,
) -> Response {
    // Helper: parse body as JSON into T, or return a 400 with the parse error.
    macro_rules! parse {
        ($T:ty) => {
            match serde_json::from_slice::<$T>(&body) {
                Ok(v) => v,
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "kind": "parseError",
                            "command": command,
                            "error": e.to_string()
                        })),
                    )
                        .into_response()
                }
            }
        };
    }

    // Helper: serialise a successful DTO to JSON, or 500 on serialisation failure.
    macro_rules! ok {
        ($v:expr) => {
            match serde_json::to_value($v) {
                Ok(json) => (StatusCode::OK, Json(json)).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "kind": "serializeError", "error": e.to_string() })),
                )
                    .into_response(),
            }
        };
    }

    match command.as_str() {
        // ---- No-arg commands (body ignored) --------------------------------
        "get_backend_handshake" => {
            ok!(fw_tauri::BackendHandshakeDto::live())
        }
        "advance_week" => match advance_week_inner(&state) {
            Ok(dto) => ok!(dto),
            Err(e) => ipc_err(e),
        },
        "play_fixtures" => match play_fixtures_inner(&state) {
            Ok(dto) => ok!(dto),
            Err(e) => ipc_err(e),
        },
        "get_standings" => match get_standings_inner(&state) {
            Ok(dto) => ok!(dto),
            Err(e) => ipc_err(e),
        },
        "get_squad" => match get_squad_inner(&state) {
            Ok(dto) => ok!(dto),
            Err(e) => ipc_err(e),
        },
        "advance_season" => match advance_season_inner(&state) {
            Ok(dto) => ok!(dto),
            Err(e) => ipc_err(e),
        },
        "get_career_overview" => match get_career_overview_inner(&state) {
            Ok(dto) => ok!(dto),
            Err(e) => ipc_err(e),
        },
        "get_press_inbox" => match get_press_inbox_inner(&state) {
            Ok(dto) => ok!(dto),
            Err(e) => ipc_err(e),
        },
        "get_squad_roster" => match get_squad_roster_inner(&state) {
            Ok(dto) => ok!(dto),
            Err(e) => ipc_err(e),
        },
        "get_settings" => match get_settings_inner(&state) {
            Ok(dto) => ok!(dto),
            Err(e) => ipc_err(e),
        },
        "save_career" => match save_career_inner(&state) {
            Ok(()) => (StatusCode::OK, Json(Value::Null)).into_response(),
            Err(e) => ipc_err(e),
        },
        "load_career" => match load_career_inner(&state) {
            Ok(()) => (StatusCode::OK, Json(Value::Null)).into_response(),
            Err(e) => ipc_err(e),
        },

        // ---- Commands with args (body deserialized per command) ------------
        "play_match" => {
            let args = parse!(PlayMatchArgs);
            match play_match_inner(args.seed_hex, args.tick_count, &state).await {
                Ok(dto) => ok!(dto),
                Err(e) => ipc_err(e),
            }
        }
        "match_frames" => {
            let args = parse!(MatchFramesArgs);
            match match_frames_inner(args.seed_hex, args.tick_count, &state).await {
                Ok(dto) => ok!(dto),
                Err(e) => ipc_err(e),
            }
        }
        "get_fixtures" => {
            let args = parse!(GetFixturesArgs);
            match get_fixtures_inner(args.club_id, &state) {
                Ok(dto) => ok!(dto),
                Err(e) => ipc_err(e),
            }
        }
        "get_player_detail" => {
            let args = parse!(GetPlayerDetailArgs);
            match get_player_detail_inner(&args.player_id, &state) {
                Ok(dto) => ok!(dto),
                Err(e) => ipc_err(e),
            }
        }
        "get_roster_for_club" => {
            let args = parse!(GetRosterForClubArgs);
            match get_roster_for_club_inner(args.club_id, &state) {
                Ok(dto) => ok!(dto),
                Err(e) => ipc_err(e),
            }
        }
        "get_scout_report" => {
            let args = parse!(GetScoutReportArgs);
            match get_scout_report_inner(&args.player_id, &state) {
                Ok(dto) => ok!(dto),
                Err(e) => ipc_err(e),
            }
        }
        "start_live_match" => {
            let args = parse!(StartLiveMatchArgs);
            match start_live_match_inner(args.seed_hex, &state) {
                Ok(dto) => ok!(dto),
                Err(e) => ipc_err(e),
            }
        }
        "step_live_match" => {
            let args = parse!(StepLiveMatchArgs);
            match step_live_match_inner(args.handle, args.ticks, &state) {
                Ok(dto) => ok!(dto),
                Err(e) => ipc_err(e),
            }
        }
        "get_match_snapshot" => {
            let args = parse!(GetMatchSnapshotArgs);
            match get_match_snapshot_inner(args.handle, &state) {
                Ok(dto) => ok!(dto),
                Err(e) => ipc_err(e),
            }
        }
        "finish_live_match" => {
            let args = parse!(FinishLiveMatchArgs);
            match finish_live_match_inner(args.handle, &state) {
                Ok(dto) => ok!(dto),
                Err(e) => ipc_err(e),
            }
        }
        "apply_match_command" => {
            let args = parse!(ApplyMatchCommandArgs);
            match apply_match_command_inner(args.handle, args.command, &state) {
                Ok(()) => (StatusCode::OK, Json(Value::Null)).into_response(),
                Err(e) => ipc_err(e),
            }
        }
        "set_settings" => {
            let args = parse!(SetSettingsArgs);
            match set_settings_inner(args.settings, &state) {
                Ok(()) => (StatusCode::OK, Json(Value::Null)).into_response(),
                Err(e) => ipc_err(e),
            }
        }

        other => {
            log::warn!("fw-dev-server: unknown command: {other}");
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "kind": "unknownCommand",
                    "command": other
                })),
            )
                .into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Content path — mirrors the resolution in src-tauri/main.rs.
    let content_path = std::env::var("FW_CONTENT_PATH").unwrap_or_else(|_| {
        let cwd_rel = PathBuf::from("content");
        if cwd_rel.join("sources").is_dir() {
            cwd_rel.to_string_lossy().into_owned()
        } else {
            // CARGO_MANIFEST_DIR = .../crates/fw-dev-server; workspace root is ../../
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../content")
                .to_string_lossy()
                .into_owned()
        }
    });

    // Dev harness paths — NEVER clobbers a real career save.
    let harness_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/dev-harness");
    std::fs::create_dir_all(&harness_dir)?;

    let settings_path = harness_dir.join("dev-settings.fwcfg");
    let career_save_path = harness_dir.join("dev-career.fwsave");

    log::info!("fw-dev-server: loading content from {content_path}");
    log::info!("fw-dev-server: dev harness at {}", harness_dir.display());

    let mut app_state = AppState::new_with_settings_file(Path::new(&content_path), settings_path)
        .map_err(|e| anyhow::anyhow!("Failed to load ContentStore: {e}"))?;
    app_state.set_career_save_path(career_save_path);

    let shared: SharedState = Arc::new(app_state);

    let app = Router::new()
        .route("/cmd/:command", post(dispatch))
        .with_state(shared);

    // Bind ONLY to localhost — this server must NEVER be accessible outside
    // the dev machine. 0.0.0.0 is explicitly forbidden.
    // Port 1422 (NOT 1421): Vite's HMR websocket uses 1421 under TAURI_DEV_HOST
    // (the Windows cross-host dev path, vite.config.ts), so binding 1421 here
    // would collide and crash Vite (strictPort). 1420 is the Vite dev server.
    let bind_addr = "127.0.0.1:1422";
    log::info!("fw-dev-server: listening on http://{bind_addr}");
    log::info!("fw-dev-server: use ?backend=http in the browser to activate the http mode");

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

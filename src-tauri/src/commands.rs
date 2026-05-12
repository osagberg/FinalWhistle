// T0-2 placeholder command surface.
//
// Every command here returns hand-shaped stub data whose schema matches what
// the real fw-tauri layer will return when T1-5 (`play_match` Tauri command)
// and T2-5 (season-controller commands) land. Frontend code can develop
// against these shapes without churn when the real backend wires up.
//
// Pattern: stable serde shapes here MUST stay in sync with
// `frontend/src/lib/types.ts`. When fw-tauri begins exporting real types,
// re-export them through this module and delete the duplicate structs.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Wire types — see also `frontend/src/lib/types.ts`.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DummyState {
    pub app_version: String,
    pub message: String,
    pub backend_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchEvent {
    pub tick: u32,
    pub minute: u16,
    pub kind: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchResult {
    pub match_id: String,
    pub home_id: String,
    pub away_id: String,
    pub home_score: u8,
    pub away_score: u8,
    pub canonical_hash: String,
    pub events: Vec<MatchEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeagueStanding {
    pub position: u8,
    pub club_id: String,
    pub club_name: String,
    pub played: u8,
    pub won: u8,
    pub drawn: u8,
    pub lost: u8,
    pub goals_for: u16,
    pub goals_against: u16,
    pub goal_difference: i16,
    pub points: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSummary {
    pub player_id: String,
    pub name: String,
    pub age: u8,
    pub role: String,
    pub phenotype_labels: Vec<String>,
    pub contract_end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fixture {
    pub fixture_id: String,
    pub date: String,
    pub home_id: String,
    pub away_id: String,
    pub competition: String,
}

// ---------------------------------------------------------------------------
// Commands. Every signature is `#[tauri::command]`; arguments are camelCase
// from the JS side, the `#[serde(rename_all = "camelCase")]` on payload types
// handles the auto-conversion.
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_dummy_state() -> DummyState {
    DummyState {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        message: "Backend reachable. T0-2 scaffold.".to_string(),
        backend_ready: false,
    }
}

#[tauri::command]
pub fn play_match(seed: String, home_id: String, away_id: String) -> Result<MatchResult, String> {
    // `seed` is a string because JS `BigInt` doesn't round-trip cleanly through
    // serde_json. The frontend stringifies its bigint; this side parses it.
    let _seed: u64 = seed
        .parse()
        .map_err(|e: std::num::ParseIntError| format!("invalid seed: {e}"))?;

    // Placeholder match result — fw-match-sim will replace this at T1-5.
    Ok(MatchResult {
        match_id: format!("placeholder-{home_id}-vs-{away_id}"),
        home_id,
        away_id,
        home_score: 0,
        away_score: 0,
        canonical_hash: "0x0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(),
        events: vec![],
    })
}

#[tauri::command]
pub fn get_league_standings(_league_id: String) -> Vec<LeagueStanding> {
    // fw-tauri T2-5 will replace.
    vec![]
}

#[tauri::command]
pub fn get_squad(_club_id: String) -> Vec<PlayerSummary> {
    // fw-tauri T2-5 will replace.
    vec![]
}

#[tauri::command]
pub fn list_fixtures(_club_id: String) -> Vec<Fixture> {
    // fw-tauri T2-5 will replace.
    vec![]
}

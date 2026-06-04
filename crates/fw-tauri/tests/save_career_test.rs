//! T4-2.5g integration tests — save_career + load_career IPC commands.
//!
//! Tests call the `_inner` helpers directly (no `tauri::State` plumbing
//! needed). Each test uses a temp-dir career-save path via
//! `AppState::new_with_settings_path` + `set_career_save_path` so no live
//! Tauri runtime is required and tests can run concurrently without clobbering
//! each other.
//!
//! ## Acceptance criteria tested
//!
//! AC1 `save_load_round_trips_roster_with_mutated_delta`:
//!   - Mutate a known player's `career_apps` and `ceiling` on the live roster.
//!   - Call `save_career_inner` → file exists.
//!   - Construct a FRESH AppState at the same seed + same temp path.
//!   - Call `load_career_inner` on the fresh state.
//!   - Assert the total roster count is unchanged (regenerate + overlay, not just
//!     the saved delta count).
//!   - Assert the mutated player has the bumped `career_apps` — NOT 0
//!     (proves the overlay fired, not just base-roster regeneration).
//!   - Assert the mutated player has the bumped ceiling (second delta field).
//!
//! AC2 `migrated_empty_roster_regenerates_full_roster`:
//!   - Build a `SaveV4` with an EMPTY roster (simulates a V3-migrated save).
//!   - Write it directly to the temp path.
//!   - Call `load_career_inner` on a fresh AppState.
//!   - Assert the roster has the full expected count (20 clubs × 22 slots = 440).

use std::collections::BTreeMap;
use std::path::PathBuf;

use fw_core::{AbilityCeiling, Q32, Seed};
use fw_memory::SeasonNumber;
use fw_save::{SaveEnvelope, SaveV4, encode};
use fw_tauri::commands::{advance_week_inner, load_career_inner, save_career_inner};
use fw_tauri::state::AppState;

fn workspace_content_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

/// Build a fresh `AppState` with a temp-dir settings path AND career-save path.
///
/// `career_seed` is explicit so tests can construct two AppState instances from
/// the same seed (simulating save → reload with the same world).
fn test_state_with_temp_paths(career_seed: Seed) -> (AppState, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let settings_path = dir.path().join("settings.fwcfg");
    let career_save_path = dir.path().join("career.fwsave");

    let mut state =
        AppState::new_with_settings_path(&workspace_content_path(), career_seed, settings_path)
            .expect("AppState::new_with_settings_path");
    state.set_career_save_path(career_save_path);
    (state, dir)
}

// ---------------------------------------------------------------------------
// AC1: save → load round-trips a mutated roster delta (non-vacuous overlay)
// ---------------------------------------------------------------------------

/// Mutation of `career_apps` + `ceiling` on a roster player survives a
/// save → fresh-AppState-load cycle. If `load_career_inner` merely regenerated
/// the base roster without overlaying the saved deltas, the loaded `career_apps`
/// would be 0 (the base default), failing the assertion.
#[test]
fn save_load_round_trips_roster_with_mutated_delta() {
    let seed = Seed::from_u64(0xDEAD_BEEF_5A4E_0001);
    let (state, dir) = test_state_with_temp_paths(seed);

    // Find the first club and first player in the roster.
    let (target_club_id, target_player_id) = {
        let career = state.career().read().expect("career lock");
        let (&club_id, instances) = career
            .roster
            .iter()
            .next()
            .expect("roster must have at least one club");
        let player_id = instances[0].player_id;
        (club_id, player_id)
    };

    // Mutate career_apps + ceiling on that player.
    let bumped_career_apps: u32 = 99;
    // Bump ceiling to a non-default value: current = Q32(0.5), potential = Q32(1.0).
    // try_new validates 0 ≤ current ≤ potential ≤ 1.
    let half = Q32::from_raw(2_147_483_648_i64); // 0.5 * 2^32
    let bumped_ceiling = AbilityCeiling::try_new(half, Q32::ONE)
        .expect("AbilityCeiling::try_new(0.5, 1.0) is always valid");

    {
        let mut career = state.career().write().expect("career lock");
        let instances = career
            .roster
            .get_mut(&target_club_id)
            .expect("club must be in roster");
        let player = instances
            .iter_mut()
            .find(|p| p.player_id == target_player_id)
            .expect("player must be in instances");
        player.career_apps = bumped_career_apps;
        player.ceiling = bumped_ceiling;
    }

    // Save.
    save_career_inner(&state).expect("save_career_inner must succeed");
    assert!(
        state.career_save_path().exists(),
        "career save file must exist after save_career_inner"
    );

    // Fresh AppState at the same seed + same temp dir.
    let settings_path = dir.path().join("settings.fwcfg");
    let career_save_path = dir.path().join("career.fwsave");
    let mut state2 =
        AppState::new_with_settings_path(&workspace_content_path(), seed, settings_path)
            .expect("AppState::new_with_settings_path (state2)");
    state2.set_career_save_path(career_save_path);

    // Load.
    load_career_inner(&state2).expect("load_career_inner must succeed");

    // AC1a: the total roster count is the full base roster (regenerate + overlay,
    // not just the saved delta count).
    let career2 = state2.career().read().expect("career2 lock");
    let total_players: usize = career2.roster.values().map(|v| v.len()).sum();
    assert!(
        total_players >= 440,
        "loaded roster must have at least 440 players (20 clubs × 22 slots); got {total_players}"
    );

    // AC1b: the mutated player has the bumped career_apps (NOT 0).
    let loaded_instances = career2
        .roster
        .get(&target_club_id)
        .expect("target club must be in loaded roster");
    let loaded_player = loaded_instances
        .iter()
        .find(|p| p.player_id == target_player_id)
        .expect("target player must be in loaded instances");
    assert_eq!(
        loaded_player.career_apps, bumped_career_apps,
        "career_apps must be the saved delta value, NOT the base-roster default of 0; \
         if this is 0, load_career_inner regenerated the base without overlaying the delta"
    );

    // AC1c: the mutated player has the bumped ceiling (full equality check).
    assert_eq!(
        loaded_player.ceiling, bumped_ceiling,
        "ceiling must equal the saved delta value"
    );
}

// ---------------------------------------------------------------------------
// AC2: empty-roster save (migrated from <V4) regenerates the full base roster
// ---------------------------------------------------------------------------

/// An empty-roster `SaveV4` (simulating a V3-migrated save) loads correctly:
/// `load_career_inner` regenerates the full base roster from the career seed
/// without attempting to overlay any deltas. The result must have the full
/// 20×22 = 440 instances.
#[test]
fn migrated_empty_roster_regenerates_full_roster() {
    let seed = Seed::from_u64(0xCAFE_BABE_4E6E_0002);
    let (_state, dir) = test_state_with_temp_paths(seed);

    // Build a SaveV4 with an EMPTY roster — simulates a V3-migrated save.
    let empty_save = SaveV4 {
        career_seed: seed,
        content_pack_version: 1,
        ledger: fw_memory::MemoryLedger::new(),
        season_number: SeasonNumber(0),
        season: None,
        roster: BTreeMap::new(),
        breakthrough_eval_watermark: 0,
    };
    let bytes = encode(&SaveEnvelope::V4(empty_save)).expect("encode empty-roster SaveV4");
    let career_save_path = dir.path().join("career.fwsave");
    std::fs::write(&career_save_path, &bytes).expect("write empty-roster save file");

    // Build a fresh AppState pointed at the written file.
    let settings_path = dir.path().join("settings.fwcfg");
    let mut state =
        AppState::new_with_settings_path(&workspace_content_path(), seed, settings_path)
            .expect("AppState::new_with_settings_path");
    state.set_career_save_path(career_save_path);

    // Load.
    load_career_inner(&state).expect("load_career_inner must succeed on empty-roster save");

    // Assert the full base roster was regenerated.
    let career = state.career().read().expect("career lock");
    let total_players: usize = career.roster.values().map(|v| v.len()).sum();
    assert!(
        total_players >= 440,
        "empty-roster load must regenerate at least 440 players (20 clubs × 22 slots); \
         got {total_players} — the base-roster regeneration path did not fire"
    );

    // All career_apps must be 0 (the base default — no overlay fired).
    let any_nonzero_apps = career
        .roster
        .values()
        .flat_map(|v| v.iter())
        .any(|p| p.career_apps != 0);
    assert!(
        !any_nonzero_apps,
        "all career_apps must be 0 after loading an empty-roster save (no overlay to apply)"
    );
}

// ---------------------------------------------------------------------------
// AC3: last_scout_report survives save → load (ultra-review P1-5 regression)
// ---------------------------------------------------------------------------

/// A player observed in a live session (observation_count > 0 with a populated
/// last_scout_report) must still surface that exact report after a save → reload.
/// SaveV4 persists observation_count but not the report itself; load_career_inner
/// re-derives it deterministically from the career seed. Before the fix every
/// reloaded scouted player reported NotYetObserved — an internally-inconsistent
/// observation_count>0 + last_scout_report==None state that cannot arise live.
#[test]
fn scout_report_survives_save_load_round_trip() {
    let seed = Seed::from_u64(0xDEAD_BEEF_5C00_0003);
    let (state, dir) = test_state_with_temp_paths(seed);

    // Advance one match-day so the starting XIs of the playing clubs get
    // observation_count == 1 + a populated last_scout_report (observe_match_participants).
    advance_week_inner(&state).expect("advance_week_inner must succeed on match-day 0");

    // Capture an observed player's club, id, count, and its live report.
    let (target_club_id, target_player_id, expected_count, expected_report) = {
        let career = state.career().read().expect("career lock");
        let mut found = None;
        'outer: for (&club_id, instances) in career.roster.iter() {
            for inst in instances {
                if inst.observation_count > 0 {
                    if let Some(report) = &inst.last_scout_report {
                        found = Some((
                            club_id,
                            inst.player_id,
                            inst.observation_count,
                            report.clone(),
                        ));
                        break 'outer;
                    }
                }
            }
        }
        found
            .expect("at least one player must be observed (observation_count>0) after advance_week")
    };

    save_career_inner(&state).expect("save_career_inner must succeed");

    // Fresh AppState at the same seed + temp dir → load.
    let settings_path = dir.path().join("settings.fwcfg");
    let career_save_path = dir.path().join("career.fwsave");
    let mut state2 =
        AppState::new_with_settings_path(&workspace_content_path(), seed, settings_path)
            .expect("AppState::new_with_settings_path (state2)");
    state2.set_career_save_path(career_save_path);
    load_career_inner(&state2).expect("load_career_inner must succeed");

    let career2 = state2.career().read().expect("career2 lock");

    // AC3a: the observed player's report survived byte-for-byte (deterministic re-derive).
    let loaded = career2
        .roster
        .get(&target_club_id)
        .expect("target club in loaded roster")
        .iter()
        .find(|p| p.player_id == target_player_id)
        .expect("target player in loaded roster");
    assert_eq!(
        loaded.observation_count, expected_count,
        "observation_count must round-trip"
    );
    assert_eq!(
        loaded.last_scout_report.as_ref(),
        Some(&expected_report),
        "last_scout_report must be re-derived identically after load \
         (was dropped → NotYetObserved before the P1-5 fix)"
    );

    // AC3b: the global invariant — observation_count>0 ⇒ last_scout_report.is_some()
    // — holds across the entire reloaded roster (no inconsistent persisted state).
    for instances in career2.roster.values() {
        for inst in instances {
            if inst.observation_count > 0 {
                assert!(
                    inst.last_scout_report.is_some(),
                    "player {:?} has observation_count={} but no last_scout_report after load \
                     — the re-derive pass missed it (P1-5)",
                    inst.player_id,
                    inst.observation_count,
                );
            }
        }
    }
}

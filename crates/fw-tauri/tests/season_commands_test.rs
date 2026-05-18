//! T2-5 integration tests — season IPC commands.
//!
//! Tests call the `_inner` helpers directly (no `tauri::State` plumbing
//! needed in unit tests). The `#[tauri::command]` wrappers are thin
//! forwarding shells validated by the existing ipc_contract_test.rs pattern.
//!
//! The full-season performance gate is `#[ignore]`-marked by default; run it
//! explicitly:
//!   cargo test --release -p fw-tauri --test season_commands_test \
//!       -- --ignored full_season_perf_under_30s --nocapture

use std::path::PathBuf;

use fw_content::{CLUBS_PER_LEAGUE, MATCH_DAYS_PER_SEASON, MATCHES_PER_SEASON};
use fw_core::Seed;
use fw_tauri::commands::{
    advance_week_inner, get_fixtures_inner, get_standings_inner, play_fixtures_inner,
};
use fw_tauri::state::AppState;

fn workspace_content_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

fn test_state() -> AppState {
    AppState::new(&workspace_content_path()).expect("AppState::new in test")
}

fn test_state_with_seed(seed: u64) -> AppState {
    AppState::new_with_career_seed(&workspace_content_path(), Seed::from_u64(seed))
        .expect("AppState::new_with_career_seed in test")
}

// ---------------------------------------------------------------------------
// advance_week
// ---------------------------------------------------------------------------

#[test]
fn advance_week_plays_ten_matches_on_day_one() {
    let state = test_state();
    let summary = advance_week_inner(&state).expect("advance_week day 1");
    assert_eq!(summary.match_day_played, 1);
    assert_eq!(
        summary.matches_played,
        (CLUBS_PER_LEAGUE / 2) as u16,
        "expected {} matches on match-day 1",
        CLUBS_PER_LEAGUE / 2
    );
    assert!(
        !summary.season_complete,
        "season should not be complete after day 1"
    );
}

#[test]
fn advance_week_increments_current_match_day() {
    let state = test_state();
    advance_week_inner(&state).expect("day 1");
    let day = state.season().read().expect("lock").current_match_day;
    assert_eq!(day, 2, "current_match_day should be 2 after playing day 1");
}

#[test]
fn advance_week_returns_season_complete_after_final_day() {
    // T2-R-A4 sanity pin: assert MATCH_DAYS_PER_SEASON == 38 as a
    // LITERAL. Without this pin, mutating the constant 38→16 would
    // shift both the loop bound AND the final-day assert + the
    // `is_complete()` check, so the test would pass against a broken
    // constant. Pin makes the constant load-bearing.
    assert_eq!(MATCH_DAYS_PER_SEASON, 38, "MATCH_DAYS_PER_SEASON pinned at 38");
    let state = test_state();
    // Fast-forward all 38 days by calling advance_week in a loop.
    let mut last_summary = None;
    for _ in 0..38 {
        last_summary = Some(advance_week_inner(&state).expect("advance_week"));
    }
    let summary = last_summary.expect("at least one advance");
    assert!(
        summary.season_complete,
        "season_complete must be true after match-day 38"
    );
    assert_eq!(summary.match_day_played, 38);
}

#[test]
fn advance_week_errors_when_season_complete() {
    let state = test_state();
    // Complete the season.
    for _ in 0..MATCH_DAYS_PER_SEASON {
        advance_week_inner(&state).expect("advance");
    }
    let err = advance_week_inner(&state).expect_err("should error on completed season");
    match err {
        fw_tauri::IpcError::SeasonComplete => {}
        other => panic!("expected SeasonComplete; got {other:?}"),
    }
}

#[test]
fn advance_week_is_deterministic_same_seed() {
    // Two states with the same seed must produce the same results after one
    // advance_week call.
    let seed = 0xDEAD_BEEF_CAFE_BABE;
    let state_a = test_state_with_seed(seed);
    let state_b = test_state_with_seed(seed);

    advance_week_inner(&state_a).expect("advance a");
    advance_week_inner(&state_b).expect("advance b");

    let results_a = state_a.season().read().expect("lock a").results.clone();
    let results_b = state_b.season().read().expect("lock b").results.clone();
    assert_eq!(
        results_a, results_b,
        "same career seed must produce identical results after one match-day"
    );
}

// ---------------------------------------------------------------------------
// play_fixtures
// ---------------------------------------------------------------------------

#[test]
fn play_fixtures_completes_the_season() {
    let state = test_state();
    let summary = play_fixtures_inner(&state).expect("play_fixtures");
    assert_eq!(
        summary.matches_played, MATCHES_PER_SEASON as u32,
        "play_fixtures must play all {} matches",
        MATCHES_PER_SEASON
    );
    assert_eq!(
        summary.final_match_day, MATCH_DAYS_PER_SEASON,
        "final_match_day should be {}",
        MATCH_DAYS_PER_SEASON
    );
    assert!(
        state.season().read().expect("lock").is_complete(),
        "season should be complete after play_fixtures"
    );
}

#[test]
fn play_fixtures_on_already_complete_season_returns_zero_matches() {
    let state = test_state();
    play_fixtures_inner(&state).expect("first play_fixtures");
    let summary = play_fixtures_inner(&state).expect("second play_fixtures on complete season");
    assert_eq!(
        summary.matches_played, 0,
        "play_fixtures on a complete season should play 0 additional matches"
    );
}

#[test]
fn play_fixtures_is_deterministic_same_seed() {
    let seed = 0x00C0_FFEE_BABE_u64;
    let state_a = test_state_with_seed(seed);
    let state_b = test_state_with_seed(seed);

    play_fixtures_inner(&state_a).expect("play_fixtures a");
    play_fixtures_inner(&state_b).expect("play_fixtures b");

    // T2-R-D3: the prior shape only compared `standings_a[0]` (the
    // league leader) — a bug that scrambled positions 2..20 while
    // leaving position 1 stable would pass. `Season.results` is a
    // BTreeMap<(ClubId, ClubId), MatchOutcome> — field-order stable
    // and the right discriminator for full-season determinism. Mirror
    // the pattern from `advance_week_is_deterministic_same_seed` in
    // this same file.
    let results_a = state_a.season().read().expect("lock a").results.clone();
    let results_b = state_b.season().read().expect("lock b").results.clone();
    assert_eq!(
        results_a, results_b,
        "same career seed must produce identical full-season results BTreeMap"
    );
}

// ---------------------------------------------------------------------------
// get_standings
// ---------------------------------------------------------------------------

#[test]
fn get_standings_returns_twenty_rows() {
    let state = test_state();
    let rows = get_standings_inner(&state).expect("get_standings on empty season");
    assert_eq!(
        rows.len(),
        CLUBS_PER_LEAGUE,
        "standings must have {} rows",
        CLUBS_PER_LEAGUE
    );
}

#[test]
fn get_standings_all_zero_before_any_match_day() {
    let state = test_state();
    let rows = get_standings_inner(&state).expect("get_standings");
    for row in &rows {
        assert_eq!(
            row.points, 0,
            "no matches played yet; all clubs should have 0 pts"
        );
        assert_eq!(row.played, 0);
    }
}

#[test]
fn get_standings_points_tally_after_full_season() {
    let state = test_state();
    play_fixtures_inner(&state).expect("play_fixtures");
    let rows = get_standings_inner(&state).expect("get_standings");

    // Total points: each match awards 3 (win) or 2 (draw via 1+1). For a
    // full 380-match season the total points in the table lie in the range
    // [380 × 2, 380 × 3] = [760, 1140].
    let total_points: u32 = rows.iter().map(|r| r.points as u32).sum();
    assert!(
        (760..=1140).contains(&total_points),
        "total points {total_points} outside expected range [760, 1140]"
    );

    // Every club played exactly 38 matches.
    for row in &rows {
        assert_eq!(
            row.played, 38,
            "club {} played {} matches; expected 38",
            row.club_name, row.played
        );
    }
}

#[test]
fn get_standings_sort_order_points_desc() {
    let state = test_state();
    play_fixtures_inner(&state).expect("play_fixtures");
    let rows = get_standings_inner(&state).expect("get_standings");
    for w in rows.windows(2) {
        let a = &w[0];
        let b = &w[1];
        assert!(
            a.points >= b.points,
            "standings not sorted by points DESC: row {} has {} pts but row {} has {} pts",
            a.club_name,
            a.points,
            b.club_name,
            b.points
        );
    }
}

// ---------------------------------------------------------------------------
// get_fixtures
// ---------------------------------------------------------------------------

#[test]
fn get_fixtures_returns_38_for_valid_club() {
    // T2-R-A3 sanity pin: assert against LITERAL 38, not the
    // derived `(CLUBS_PER_LEAGUE - 1) * 2`. Without this pin, mutating
    // CLUBS_PER_LEAGUE 20→16 would shift both sides of the assert below
    // to 30==30 and the test would pass against a broken constant.
    assert_eq!((CLUBS_PER_LEAGUE - 1) * 2, 38, "fixtures-per-club pinned at 38");
    let state = test_state();
    let first_club_id = state.season().read().expect("lock").league.clubs[0]
        .id
        .raw();
    let fixtures = get_fixtures_inner(first_club_id, &state).expect("get_fixtures");
    assert_eq!(fixtures.len(), 38, "each club should have 38 fixtures");
}

#[test]
fn get_fixtures_returns_club_not_found_for_unknown_id() {
    let state = test_state();
    let err = get_fixtures_inner(99999, &state).expect_err("should error for unknown club");
    match err {
        fw_tauri::IpcError::ClubNotFound { club_id } => assert_eq!(club_id, 99999),
        other => panic!("expected ClubNotFound; got {other:?}"),
    }
}

#[test]
fn get_fixtures_unplayed_before_any_matches() {
    let state = test_state();
    let club_id = state.season().read().expect("lock").league.clubs[0]
        .id
        .raw();
    let fixtures = get_fixtures_inner(club_id, &state).expect("get_fixtures");
    assert!(
        fixtures.iter().all(|f| !f.played),
        "no fixtures should be marked played before any match-days"
    );
    assert!(
        fixtures.iter().all(|f| f.home_score.is_none()),
        "home_score should be None for all unplayed fixtures"
    );
}

#[test]
fn get_fixtures_marks_played_after_advance_week() {
    let state = test_state();
    let club_id = {
        let season = state.season().read().expect("lock");
        // Find the club that plays on match-day 1 (home side of first day-1 fixture).
        season
            .fixtures_for_match_day(1)
            .first()
            .map(|f| f.home.raw())
            .expect("day-1 fixture exists")
    };
    advance_week_inner(&state).expect("advance day 1");
    let fixtures = get_fixtures_inner(club_id, &state).expect("get_fixtures");

    // Exactly 1 fixture should be played (the day-1 one).
    let played_count = fixtures.iter().filter(|f| f.played).count();
    assert_eq!(
        played_count, 1,
        "exactly 1 fixture played after one match-day"
    );

    // That fixture should have scores.
    let played = fixtures
        .iter()
        .find(|f| f.played)
        .expect("one played fixture");
    assert!(
        played.home_score.is_some(),
        "home_score should be Some after played fixture"
    );
    assert!(
        played.away_score.is_some(),
        "away_score should be Some after played fixture"
    );
}

#[test]
fn get_fixtures_has_nineteen_home_and_nineteen_away() {
    let state = test_state();
    let club_id = state.season().read().expect("lock").league.clubs[7]
        .id
        .raw();
    let fixtures = get_fixtures_inner(club_id, &state).expect("get_fixtures");
    let home_count = fixtures.iter().filter(|f| f.is_home).count();
    let away_count = fixtures.iter().filter(|f| !f.is_home).count();
    assert_eq!(home_count, 19, "should have 19 home fixtures");
    assert_eq!(away_count, 19, "should have 19 away fixtures");
}

#[test]
fn get_fixtures_opponent_names_are_non_empty() {
    let state = test_state();
    let club_id = state.season().read().expect("lock").league.clubs[0]
        .id
        .raw();
    let fixtures = get_fixtures_inner(club_id, &state).expect("get_fixtures");
    for f in &fixtures {
        assert!(
            !f.opponent_club_name.is_empty(),
            "opponent_club_name must not be empty (match_day={})",
            f.match_day
        );
    }
}

// ---------------------------------------------------------------------------
// Performance gate — ignored by default, run with --release --ignored
// ---------------------------------------------------------------------------

/// Full-season fast-forward under 30 seconds.
///
/// Run: `cargo test --release -p fw-tauri --test season_commands_test \
///          -- --ignored full_season_perf_under_30s --nocapture`
///
/// Uses `std::time::Instant` (allowed in fw-tauri; blocked only in sim crates
/// per `Sim/RULES.md §3`).
#[test]
#[ignore]
fn full_season_perf_under_30s() {
    let state = test_state();
    let start = std::time::Instant::now();
    play_fixtures_inner(&state).expect("play_fixtures");
    let elapsed = start.elapsed();
    println!(
        "full_season_perf_under_30s: played {} matches in {:.3}s",
        MATCHES_PER_SEASON,
        elapsed.as_secs_f64()
    );
    assert!(
        elapsed < std::time::Duration::from_secs(30),
        "full season ({} matches × {} ticks) took {:.3}s; expected < 30s",
        MATCHES_PER_SEASON,
        fw_tauri::season::SEASON_MATCH_TICK_BUDGET,
        elapsed.as_secs_f64()
    );
}

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
use fw_memory::event::EventClass;
use fw_tauri::commands::{
    advance_season_inner, advance_week_inner, get_fixtures_inner, get_standings_inner,
    play_fixtures_inner,
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
    let day = state
        .career()
        .read()
        .expect("lock")
        .season
        .current_match_day;
    assert_eq!(day, 2, "current_match_day should be 2 after playing day 1");
}

#[test]
fn advance_week_returns_season_complete_after_final_day() {
    // T2-R-A4 sanity pin: assert MATCH_DAYS_PER_SEASON == 38 as a
    // LITERAL. Without this pin, mutating the constant 38→16 would
    // shift both the loop bound AND the final-day assert + the
    // `is_complete()` check, so the test would pass against a broken
    // constant. Pin makes the constant load-bearing.
    assert_eq!(
        MATCH_DAYS_PER_SEASON, 38,
        "MATCH_DAYS_PER_SEASON pinned at 38"
    );
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

    let results_a = state_a
        .career()
        .read()
        .expect("lock a")
        .season
        .results
        .clone();
    let results_b = state_b
        .career()
        .read()
        .expect("lock b")
        .season
        .results
        .clone();
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
        state.career().read().expect("lock").season.is_complete(),
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
    let results_a = state_a
        .career()
        .read()
        .expect("lock a")
        .season
        .results
        .clone();
    let results_b = state_b
        .career()
        .read()
        .expect("lock b")
        .season
        .results
        .clone();
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
    assert_eq!(
        (CLUBS_PER_LEAGUE - 1) * 2,
        38,
        "fixtures-per-club pinned at 38"
    );
    let state = test_state();
    let first_club_id = state.career().read().expect("lock").season.league.clubs[0]
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
    let club_id = state.career().read().expect("lock").season.league.clubs[0]
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
        let career = state.career().read().expect("lock");
        // Find the club that plays on match-day 1 (home side of first day-1 fixture).
        career
            .season
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
    let club_id = state.career().read().expect("lock").season.league.clubs[7]
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
    let club_id = state.career().read().expect("lock").season.league.clubs[0]
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

// ---------------------------------------------------------------------------
// 5-season career integration test (AC4)
// ---------------------------------------------------------------------------

/// AC4: 5-season career — after 5×(play_fixtures + advance_season) the ledger
/// holds ≥5 TitleWon events, at least one Compaction event is present after
/// the 5-season boundary, and season_number reached 5.
///
/// This test is marked `#[ignore]` by default because it takes ~5× the
/// full_season_perf time. Run with:
///   cargo test --release -p fw-tauri --test season_commands_test \
///       -- --ignored five_season_career_integration --nocapture
#[test]
#[ignore]
fn five_season_career_integration() {
    let state = test_state();
    for i in 0u16..5 {
        play_fixtures_inner(&state)
            .unwrap_or_else(|e| panic!("play_fixtures failed on season {i}: {e:?}"));
        advance_season_inner(&state)
            .unwrap_or_else(|e| panic!("advance_season failed on season {i}: {e:?}"));
    }

    let career = state.career().read().expect("career lock");
    assert_eq!(
        career.season_number.0, 5,
        "season_number must be 5 after 5 advances"
    );

    let title_won_count = career
        .ledger
        .iter()
        .filter(|e| matches!(e.event_class, EventClass::TitleWon))
        .count();
    // QA-T4H item 6c: tightened to == 5 (exactly one TitleWon per season).
    // Mutation killed: if advance_season emitted 0 or 2 TitleWon events per season,
    // the `>= 5` assertion would pass vacuously on 2+ events/season; `== 5` pins the
    // invariant that exactly one is emitted per season.
    assert_eq!(
        title_won_count, 5,
        "ledger must have exactly 5 TitleWon events after 5 seasons (one per advance_season); \
         got {title_won_count}"
    );

    let compaction_count = career
        .ledger
        .iter()
        .filter(|e| matches!(e.event_class, EventClass::Compaction))
        .count();
    assert!(
        compaction_count >= 1,
        "ledger must have ≥1 Compaction event after 5-season career, got {compaction_count}"
    );
}

/// Faster non-ignored variant of the 5-season integration test. Runs the
/// 5-season career loop in the default (debug) test profile so CI catches
/// regressions without the `--release --ignored` flag. Each `play_fixtures`
/// call runs at debug speed — tolerable for 5 seasons.
#[test]
fn five_season_career_integration_fast() {
    let state = test_state();
    for i in 0u16..5 {
        play_fixtures_inner(&state)
            .unwrap_or_else(|e| panic!("play_fixtures failed on season {i}: {e:?}"));
        advance_season_inner(&state)
            .unwrap_or_else(|e| panic!("advance_season failed on season {i}: {e:?}"));
    }

    let career = state.career().read().expect("career lock");
    assert_eq!(career.season_number.0, 5);

    let title_won = career
        .ledger
        .iter()
        .filter(|e| matches!(e.event_class, EventClass::TitleWon))
        .count();
    // QA-T4H item 6c: tightened to == 5 (exactly one TitleWon per season).
    assert_eq!(title_won, 5, "exactly 5 TitleWon events; got {title_won}");

    let compaction = career
        .ledger
        .iter()
        .filter(|e| matches!(e.event_class, EventClass::Compaction))
        .count();
    assert!(compaction >= 1, "≥1 Compaction event; got {compaction}");
}

// ---------------------------------------------------------------------------
// T4-2.5c AC-5: season-roster signature wiring
//
// Verifies the full path:
//   CareerState::roster (home + away instances)
//   → build_slot_signatures (role-match filter)
//   → MatchState::with_slot_signatures
//   → MatchState::players[non-slot-7 MID slot].signature_candidates non-empty
//
// This is the spec's AC-5 falsifiable gate: "a test asserts a non-slot-7
// candidate present in the played match's state" from the roster path.
// ---------------------------------------------------------------------------

#[test]
fn ac5_roster_slot_signatures_delivers_candidates_to_non_slot_7_mid() {
    use fw_content::ContentStore;
    use fw_match_sim::{DEFAULT_ARCHETYPE_ID, MatchState};
    use fw_tauri::season::build_slot_signatures;

    let content_path = workspace_content_path();
    let state = test_state();

    // Load the content store (needed for initial_with_content).
    let content = ContentStore::load_sources(&content_path).expect("content load");

    // Read the career roster. The career was built at test_state() construction;
    // grab any two clubs from the current season's fixture list for home+away.
    let career = state.career().read().expect("career lock");
    let league = &career.season.league;
    assert!(
        league.clubs.len() >= 2,
        "need at least 2 clubs to run a fixture"
    );
    let home_club_id = league.clubs[0].id;
    let away_club_id = league.clubs[1].id;

    let home_instances = career
        .roster
        .get(&home_club_id)
        .expect("home club in roster");
    let away_instances = career
        .roster
        .get(&away_club_id)
        .expect("away club in roster");

    // AC-5: build the slot_signatures map from the two clubs' rosters.
    let slot_signatures =
        build_slot_signatures(home_instances.as_slice(), away_instances.as_slice());

    // The map must contain entries for home MID slots (5, 6, 7) — role-match filter.
    // Away MID slots (16, 17, 18) also get entries.
    let home_mid_slots: &[u8] = &[5, 6, 7];
    let away_mid_slots: &[u8] = &[16, 17, 18];
    for &slot in home_mid_slots.iter().chain(away_mid_slots) {
        assert!(
            slot_signatures.contains_key(&slot),
            "slot_signatures must contain home/away MID slot {slot} (role_receives_candidates \
             returns true for in_team 5-7); got keys={:?}",
            slot_signatures.keys().collect::<Vec<_>>()
        );
        assert!(
            !slot_signatures[&slot].is_empty(),
            "slot {slot}'s candidates in slot_signatures must be non-empty (1 AM template)"
        );
    }

    // AC-5 core assertion: apply the override to a real MatchState and verify
    // a non-slot-7 MID slot carries candidates in the final state.
    drop(career); // release read lock before calling initial_with_content
    let sim_state = MatchState::initial_with_content(
        fw_core::Seed::from_u64(0xABCD_1234),
        &content,
        DEFAULT_ARCHETYPE_ID,
        DEFAULT_ARCHETYPE_ID,
    )
    .expect("initial_with_content")
    .with_slot_signatures(slot_signatures);

    // Slot 5 (home MID) must be non-empty — proving the roster path delivers
    // candidates to a non-slot-7 player.
    assert!(
        !sim_state.players[5].signature_candidates().is_empty(),
        "AC-5: home MID slot 5 must carry candidates after roster→slot_signatures wiring; \
         got 0. Check build_slot_signatures + role_receives_candidates."
    );

    // Slot 18 (away MID = 11+7) must also be non-empty.
    assert!(
        !sim_state.players[18].signature_candidates().is_empty(),
        "AC-5: away MID slot 18 must carry candidates after roster→slot_signatures wiring; \
         got 0."
    );

    // GK slot 0: T4-2.5j added a GK player-template (sample-gk.ron), so
    // `initial_with_content` now wires the GK signature (commanding-claim) to
    // slot 0. The career-roster override (`build_slot_signatures`) is MID-only
    // (in_team 5-7), so it does NOT touch slot 0 — proving the AM/MID candidates
    // do not leak to the keeper. (Pre-T4-2.5j slot 0 was empty: no GK template.)
    let slot0_ids: Vec<String> = sim_state.players[0]
        .signature_candidates()
        .iter()
        .map(|c| c.signature_id.as_str().to_owned())
        .collect();
    assert_eq!(
        slot0_ids,
        vec!["fwh.core:signature.commanding-claim".to_string()],
        "AC-5: GK slot 0 must carry exactly the GK signature from the T4-2.5j GK \
         template (the MID-only career override must not leak AM candidates to GK); \
         got {slot0_ids:?}"
    );
}

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

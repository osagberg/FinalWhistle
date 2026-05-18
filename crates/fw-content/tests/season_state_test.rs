//! T2-5: SeasonState — pure-data unit tests.
//!
//! These tests exercise the fw-content layer only: `SeasonState::new`,
//! `apply_result`, `standings`, `fixtures_for_club`, `is_complete`, and
//! `fixtures_for_match_day`. No IPC, no fw-tauri, no async.

use std::path::PathBuf;

use fw_content::{
    CLUBS_PER_LEAGUE, ContentStore, Fixture, MATCH_DAYS_PER_SEASON, MATCHES_PER_SEASON,
    MatchOutcome, SeasonState, generate_league,
};
use fw_core::{ClubId, Seed};

fn load_content() -> ContentStore {
    let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content");
    ContentStore::load_sources(&content_root).expect("ContentStore::load_sources failed")
}

fn make_season(seed: u64) -> (SeasonState, ContentStore) {
    let content = load_content();
    let league =
        generate_league(Seed::from_u64(seed), &content).expect("generate_league failed in test");
    let state = SeasonState::new(league, &content);
    (state, content)
}

// ---------------------------------------------------------------------------
// Construction invariants
// ---------------------------------------------------------------------------

#[test]
fn new_season_state_starts_at_match_day_one() {
    let (state, _) = make_season(0xC0FFEE);
    assert_eq!(state.current_match_day, 1);
}

#[test]
fn new_season_state_has_empty_results() {
    let (state, _) = make_season(0xC0FFEE);
    assert!(
        state.results.is_empty(),
        "no results expected at construction; got {}",
        state.results.len()
    );
}

#[test]
fn new_season_state_has_tactical_archetype_for_every_club() {
    let (state, _) = make_season(0xC0FFEE);
    assert_eq!(
        state.tactical_archetype_ids.len(),
        CLUBS_PER_LEAGUE,
        "expected one tactical_archetype_id per club"
    );
    for club in &state.league.clubs {
        assert!(
            state.tactical_archetype_ids.contains_key(&club.id),
            "club {:?} missing from tactical_archetype_ids",
            club.id
        );
        assert!(
            !state.tactical_archetype_ids[&club.id].is_empty(),
            "club {:?} has empty tactical_archetype_id",
            club.id
        );
    }
}

#[test]
fn season_is_not_complete_at_construction() {
    let (state, _) = make_season(0xBEEF);
    assert!(!state.is_complete());
}

// ---------------------------------------------------------------------------
// is_complete transitions
// ---------------------------------------------------------------------------

#[test]
fn season_is_complete_when_match_day_exceeds_total() {
    let (mut state, _) = make_season(0xBEEF);
    state.current_match_day = MATCH_DAYS_PER_SEASON + 1;
    assert!(state.is_complete());
}

#[test]
fn season_is_not_complete_at_last_match_day() {
    let (mut state, _) = make_season(0xBEEF);
    state.current_match_day = MATCH_DAYS_PER_SEASON;
    assert!(!state.is_complete());
}

// ---------------------------------------------------------------------------
// fixtures_for_match_day
// ---------------------------------------------------------------------------

#[test]
fn fixtures_for_match_day_returns_ten_per_day() {
    // T2-R-A2 sanity pin: assert against LITERAL 10, not the
    // derived `CLUBS_PER_LEAGUE / 2`. Without this pin, mutating
    // CLUBS_PER_LEAGUE 20→16 would shift both sides of the assert
    // below to 8==8 and the test would silently pass against a
    // broken constant.
    assert_eq!(CLUBS_PER_LEAGUE / 2, 10, "fixtures-per-match-day pinned at 10");
    let (state, _) = make_season(0xC0FFEE);
    for day in 1..=MATCH_DAYS_PER_SEASON {
        let day_fixtures = state.fixtures_for_match_day(day);
        assert_eq!(
            day_fixtures.len(),
            10,
            "match-day {day} should have 10 fixtures; got {}",
            day_fixtures.len()
        );
    }
}

#[test]
fn fixtures_for_match_day_zero_returns_empty() {
    let (state, _) = make_season(0xC0FFEE);
    assert!(state.fixtures_for_match_day(0).is_empty());
}

#[test]
fn fixtures_for_match_day_out_of_range_returns_empty() {
    let (state, _) = make_season(0xC0FFEE);
    assert!(
        state
            .fixtures_for_match_day(MATCH_DAYS_PER_SEASON + 1)
            .is_empty()
    );
}

// ---------------------------------------------------------------------------
// apply_result + standings
// ---------------------------------------------------------------------------

#[test]
fn apply_result_increments_played_count() {
    let (mut state, _) = make_season(0xABCD);
    // Scope the borrow so it ends before the mutable apply_result call.
    let fixture: Fixture = {
        let day = state.fixtures_for_match_day(1);
        **day.first().expect("at least one fixture on day 1")
    };
    state.apply_result(
        fixture.home,
        fixture.away,
        MatchOutcome {
            home_score: 2,
            away_score: 1,
        },
    );
    let standings = state.standings();
    let home_row = standings
        .rows
        .iter()
        .find(|r| r.club_id == fixture.home)
        .expect("home club in standings");
    let away_row = standings
        .rows
        .iter()
        .find(|r| r.club_id == fixture.away)
        .expect("away club in standings");

    assert_eq!(home_row.played, 1);
    assert_eq!(away_row.played, 1);
}

#[test]
fn win_awards_three_points_loss_zero() {
    let (mut state, _) = make_season(0xABCD);
    let fixture = {
        let day = state.fixtures_for_match_day(1);
        **day.first().expect("fixture on day 1")
    };
    state.apply_result(
        fixture.home,
        fixture.away,
        MatchOutcome {
            home_score: 3,
            away_score: 0,
        },
    );
    let standings = state.standings();
    let home_row = standings
        .rows
        .iter()
        .find(|r| r.club_id == fixture.home)
        .unwrap();
    let away_row = standings
        .rows
        .iter()
        .find(|r| r.club_id == fixture.away)
        .unwrap();

    assert_eq!(home_row.points, 3, "home winner should have 3 points");
    assert_eq!(home_row.wins, 1);
    assert_eq!(home_row.losses, 0);
    assert_eq!(away_row.points, 0, "away loser should have 0 points");
    assert_eq!(away_row.losses, 1);
}

#[test]
fn draw_awards_one_point_each() {
    let (mut state, _) = make_season(0xABCD);
    let fixture = {
        let day = state.fixtures_for_match_day(1);
        **day.first().expect("fixture on day 1")
    };
    state.apply_result(
        fixture.home,
        fixture.away,
        MatchOutcome {
            home_score: 1,
            away_score: 1,
        },
    );
    let standings = state.standings();
    let home_row = standings
        .rows
        .iter()
        .find(|r| r.club_id == fixture.home)
        .unwrap();
    let away_row = standings
        .rows
        .iter()
        .find(|r| r.club_id == fixture.away)
        .unwrap();

    assert_eq!(home_row.points, 1);
    assert_eq!(home_row.draws, 1);
    assert_eq!(away_row.points, 1);
    assert_eq!(away_row.draws, 1);
}

#[test]
fn goal_difference_is_goals_for_minus_goals_against() {
    let (mut state, _) = make_season(0xABCD);
    let fixture = {
        let day = state.fixtures_for_match_day(1);
        **day.first().expect("fixture on day 1")
    };
    state.apply_result(
        fixture.home,
        fixture.away,
        MatchOutcome {
            home_score: 4,
            away_score: 1,
        },
    );
    let standings = state.standings();
    let home_row = standings
        .rows
        .iter()
        .find(|r| r.club_id == fixture.home)
        .unwrap();
    let away_row = standings
        .rows
        .iter()
        .find(|r| r.club_id == fixture.away)
        .unwrap();

    assert_eq!(home_row.goal_difference, 3);
    assert_eq!(away_row.goal_difference, -3);
}

#[test]
fn standings_has_twenty_rows() {
    let (state, _) = make_season(0xDEAD);
    let standings = state.standings();
    assert_eq!(
        standings.rows.len(),
        CLUBS_PER_LEAGUE,
        "standings must have one row per club"
    );
}

#[test]
fn apply_result_overwrites_prior_result() {
    let (mut state, _) = make_season(0xABCD);
    let fixture = {
        let day = state.fixtures_for_match_day(1);
        **day.first().expect("fixture on day 1")
    };
    // First recording: home wins.
    state.apply_result(
        fixture.home,
        fixture.away,
        MatchOutcome {
            home_score: 2,
            away_score: 0,
        },
    );
    // Overwrite: away wins.
    state.apply_result(
        fixture.home,
        fixture.away,
        MatchOutcome {
            home_score: 0,
            away_score: 1,
        },
    );
    let standings = state.standings();
    let home_row = standings
        .rows
        .iter()
        .find(|r| r.club_id == fixture.home)
        .unwrap();
    let away_row = standings
        .rows
        .iter()
        .find(|r| r.club_id == fixture.away)
        .unwrap();
    // Should reflect overwritten result: away wins.
    assert_eq!(
        away_row.points, 3,
        "overwritten result: away should have 3 pts"
    );
    assert_eq!(
        home_row.points, 0,
        "overwritten result: home should have 0 pts"
    );
    // Only one match recorded (not duplicated).
    assert_eq!(home_row.played, 1);
}

#[test]
fn standings_sort_order_is_points_desc_then_gd_desc_then_gf_desc_then_club_id_asc() {
    let (mut state, _) = make_season(0xC0FFEE);
    // Play exactly match-day 1 (10 matches). Inject known results so we can
    // assert sort order without depending on the sim.
    let day1: Vec<Fixture> = state
        .fixtures_for_match_day(1)
        .into_iter()
        .copied()
        .collect();
    for f in &day1 {
        // All home wins 1-0 so all home clubs get 3pts, all away clubs get 0pts.
        state.apply_result(
            f.home,
            f.away,
            MatchOutcome {
                home_score: 1,
                away_score: 0,
            },
        );
    }
    let standings = state.standings();
    // All 10 home-day-1 clubs have 3 pts; all 10 away clubs have 0 pts.
    // Within the 3-pt group, GD is all +1 so we fall back to club_id ASC.
    let top_half = &standings.rows[..10];
    let bottom_half = &standings.rows[10..];
    for row in top_half {
        assert_eq!(row.points, 3, "top-half row should have 3 points");
    }
    for row in bottom_half {
        assert_eq!(row.points, 0, "bottom-half row should have 0 points");
    }
    // Within the top half (same pts + GD + GF), ascending club_id order.
    let top_ids: Vec<u32> = top_half.iter().map(|r| r.club_id.raw()).collect();
    let mut sorted_ids = top_ids.clone();
    sorted_ids.sort();
    assert_eq!(
        top_ids, sorted_ids,
        "top half should be sorted by club_id ASC as the final tie-break"
    );
}

// ---------------------------------------------------------------------------
// fixtures_for_club
// ---------------------------------------------------------------------------

#[test]
fn fixtures_for_club_returns_38_entries() {
    let (state, _) = make_season(0xC0FFEE);
    let first_club = state.league.clubs[0].id;
    let club_fixtures = state.fixtures_for_club(first_club);
    assert_eq!(
        club_fixtures.len(),
        (CLUBS_PER_LEAGUE - 1) * 2, // 38
        "each club should have 38 fixtures total (19 home + 19 away)"
    );
}

#[test]
fn fixtures_for_club_has_nineteen_home_and_nineteen_away() {
    let (state, _) = make_season(0xC0FFEE);
    let club_id = state.league.clubs[3].id;
    let club_fixtures = state.fixtures_for_club(club_id);
    let home_count = club_fixtures
        .iter()
        .filter(|(f, _)| f.home == club_id)
        .count();
    let away_count = club_fixtures
        .iter()
        .filter(|(f, _)| f.away == club_id)
        .count();
    assert_eq!(
        home_count,
        CLUBS_PER_LEAGUE - 1,
        "should be 19 home fixtures"
    );
    assert_eq!(
        away_count,
        CLUBS_PER_LEAGUE - 1,
        "should be 19 away fixtures"
    );
}

#[test]
fn fixtures_for_unknown_club_returns_empty() {
    let (state, _) = make_season(0xC0FFEE);
    // ClubId(9999) is not in the league.
    let unknown = ClubId::new(9999);
    assert!(state.fixtures_for_club(unknown).is_empty());
}

#[test]
fn fixtures_for_club_outcome_is_some_for_played_and_none_for_unplayed() {
    let (mut state, _) = make_season(0xC0FFEE);
    let club_id = state.league.clubs[0].id;

    // Play match-day 1 for this club's fixture only. Scope the borrow so
    // it ends before apply_result takes &mut self.
    let day1_fixture: Fixture = {
        let day = state.fixtures_for_match_day(1);
        **day
            .iter()
            .find(|f| f.home == club_id || f.away == club_id)
            .expect("club 0 has a day-1 fixture")
    };
    state.apply_result(
        day1_fixture.home,
        day1_fixture.away,
        MatchOutcome {
            home_score: 2,
            away_score: 0,
        },
    );

    let club_fixtures = state.fixtures_for_club(club_id);

    // The played fixture should have Some outcome.
    let played = club_fixtures
        .iter()
        .find(|(f, _)| f.home == day1_fixture.home && f.away == day1_fixture.away)
        .expect("the played fixture is in fixtures_for_club");
    assert!(
        played.1.is_some(),
        "played fixture should have Some(MatchOutcome)"
    );

    // All other 37 fixtures should have None.
    let unplayed_count = club_fixtures.iter().filter(|(_, r)| r.is_none()).count();
    assert_eq!(
        unplayed_count, 37,
        "37 fixtures should still have None outcome"
    );
}

#[test]
fn fixtures_for_club_is_in_match_day_order() {
    let (state, _) = make_season(0xC0FFEE);
    let club_id = state.league.clubs[5].id;
    let club_fixtures = state.fixtures_for_club(club_id);
    let days: Vec<u16> = club_fixtures.iter().map(|(f, _)| f.match_day).collect();
    let mut sorted = days.clone();
    sorted.sort();
    assert_eq!(
        days, sorted,
        "fixtures_for_club should be in match-day order"
    );
}

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

#[test]
fn season_state_new_is_deterministic() {
    // Same seed → byte-identical SeasonState (via serde JSON equality).
    let content = load_content();
    let seed = Seed::from_u64(0xFEEDBEEF);
    let league_a = generate_league(seed, &content).expect("league a");
    let league_b = generate_league(seed, &content).expect("league b");
    let state_a = SeasonState::new(league_a, &content);
    let state_b = SeasonState::new(league_b, &content);
    let json_a = serde_json::to_string(&state_a).expect("serialize a");
    let json_b = serde_json::to_string(&state_b).expect("serialize b");
    assert_eq!(json_a, json_b, "SeasonState::new should be deterministic");
}

#[test]
fn total_fixture_count_in_league_is_three_hundred_eighty() {
    // Sanity guard: if MATCHES_PER_SEASON changes, this alerts the author.
    let (state, _) = make_season(0xDEAD);
    assert_eq!(state.league.fixtures.len(), MATCHES_PER_SEASON);
    assert_eq!(MATCHES_PER_SEASON, 380);
}

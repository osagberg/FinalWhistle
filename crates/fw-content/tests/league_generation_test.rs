//! T2-2: League + fixtures generation tests.
//!
//! Per the T2-2 MEMORY task-spec AC matrix:
//!   - AC3 + AC4: `generate_league` produces 20 clubs + 380 fixtures + full
//!     pair-coverage (each ordered pair (home, away) appears exactly once;
//!     each club plays 19 home + 19 away).
//!   - AC5: determinism — same seed → byte-identical League.
//!   - AC6: divergence — different seeds → different club names.
//!
//! All tests load the real `content/sources/` tree via
//! `ContentStore::load_sources` to exercise the cross-reference path
//! (cultures + tactical_archetypes + managers must all be present + non-empty).

use fw_content::{
    CLUBS_PER_LEAGUE, ContentStore, MATCH_DAYS_PER_SEASON, MATCHES_PER_SEASON, generate_league,
    generate_league_with_teams,
};
use fw_core::{ClubId, Seed};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn load_content() -> ContentStore {
    let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content");
    ContentStore::load_sources(&content_root).expect("ContentStore::load_sources failed")
}

// ---------------------------------------------------------------------------
// AC3: generate_league produces 20 clubs + 380 fixtures with the right structure
// ---------------------------------------------------------------------------

#[test]
fn generate_league_produces_20_clubs_and_380_fixtures() {
    let content = load_content();
    let league = generate_league(Seed::from_u64(0xC0FFEE), &content).expect("generate_league");

    // T2-R-A1 sanity pins: assert against LITERALS, not the named
    // constants the production code reads. Without these pins,
    // mutating CLUBS_PER_LEAGUE 20→16 would shift both sides of the
    // assert below in lockstep, and the test would pass against a
    // broken constant. With the literal pins, the constant is locked
    // at this discovered value FOREVER (or the test names a deliberate
    // change). 20 / 380 / 38 are the production-spec football season
    // counts (Premier League shape).
    assert_eq!(CLUBS_PER_LEAGUE, 20, "CLUBS_PER_LEAGUE pinned at 20");
    assert_eq!(MATCHES_PER_SEASON, 380, "MATCHES_PER_SEASON pinned at 380");
    assert_eq!(
        MATCH_DAYS_PER_SEASON, 38,
        "MATCH_DAYS_PER_SEASON pinned at 38"
    );

    assert_eq!(
        league.clubs.len(),
        CLUBS_PER_LEAGUE,
        "league must have exactly {CLUBS_PER_LEAGUE} clubs; got {}",
        league.clubs.len()
    );
    assert_eq!(
        league.fixtures.len(),
        MATCHES_PER_SEASON,
        "league must have exactly {MATCHES_PER_SEASON} fixtures; got {}",
        league.fixtures.len()
    );

    // Each club has a non-empty display name + a unique ClubId in 1..=20.
    let mut seen_ids: Vec<ClubId> = Vec::with_capacity(CLUBS_PER_LEAGUE);
    for club in &league.clubs {
        assert!(
            !club.display_name.is_empty(),
            "club display_name must be non-empty; got club id={:?}",
            club.id
        );
        assert!(
            !seen_ids.contains(&club.id),
            "club id {:?} appears twice in league.clubs",
            club.id
        );
        seen_ids.push(club.id);
    }

    // League name is non-empty.
    assert!(
        !league.name.is_empty(),
        "league name must be non-empty; got {:?}",
        league.name
    );

    // Match-day range covers 1..=MATCH_DAYS_PER_SEASON inclusive.
    let max_match_day = league.fixtures.iter().map(|f| f.match_day).max().unwrap();
    let min_match_day = league.fixtures.iter().map(|f| f.match_day).min().unwrap();
    assert_eq!(min_match_day, 1, "match_day range must start at 1");
    assert_eq!(
        max_match_day, MATCH_DAYS_PER_SEASON,
        "match_day range must end at MATCH_DAYS_PER_SEASON={MATCH_DAYS_PER_SEASON}"
    );
}

// ---------------------------------------------------------------------------
// AC4: fixture schedule covers all ordered (home, away) pairs exactly once;
// each club plays exactly 19 home + 19 away
// ---------------------------------------------------------------------------

#[test]
fn fixture_schedule_covers_all_pairs_home_and_away() {
    let content = load_content();
    let league = generate_league(Seed::from_u64(0xC0FFEE), &content).expect("generate_league");

    // Each ordered (home, away) pair must appear exactly ONCE.
    let mut pair_count: std::collections::BTreeMap<(ClubId, ClubId), u32> =
        std::collections::BTreeMap::new();
    for f in &league.fixtures {
        assert_ne!(
            f.home, f.away,
            "fixture has same home + away club {:?} — circle method invariant broken",
            f.home
        );
        *pair_count.entry((f.home, f.away)).or_insert(0) += 1;
    }

    // For 20 clubs: 20 × 19 = 380 ordered pairs, each appearing exactly once.
    assert_eq!(
        pair_count.len(),
        MATCHES_PER_SEASON,
        "ordered pair coverage broken: expected {MATCHES_PER_SEASON} unique pairs; got {}",
        pair_count.len()
    );
    for ((home, away), count) in &pair_count {
        assert_eq!(
            *count, 1,
            "pair (home={home:?}, away={away:?}) appears {count} times; expected exactly 1"
        );
    }

    // Per-club home + away counts: each club plays 19 home + 19 away.
    let mut home_counts: std::collections::BTreeMap<ClubId, u32> =
        std::collections::BTreeMap::new();
    let mut away_counts: std::collections::BTreeMap<ClubId, u32> =
        std::collections::BTreeMap::new();
    for f in &league.fixtures {
        *home_counts.entry(f.home).or_insert(0) += 1;
        *away_counts.entry(f.away).or_insert(0) += 1;
    }
    let expected_per_side = (CLUBS_PER_LEAGUE - 1) as u32; // 19 for n=20
    for club in &league.clubs {
        let home_count = home_counts.get(&club.id).copied().unwrap_or(0);
        let away_count = away_counts.get(&club.id).copied().unwrap_or(0);
        assert_eq!(
            home_count, expected_per_side,
            "club {:?} ({}) plays {home_count} home matches; expected {expected_per_side}",
            club.id, club.display_name
        );
        assert_eq!(
            away_count, expected_per_side,
            "club {:?} ({}) plays {away_count} away matches; expected {expected_per_side}",
            club.id, club.display_name
        );
    }

    // Per-match-day: exactly CLUBS_PER_LEAGUE / 2 = 10 fixtures.
    let mut per_day: std::collections::BTreeMap<u16, u32> = std::collections::BTreeMap::new();
    for f in &league.fixtures {
        *per_day.entry(f.match_day).or_insert(0) += 1;
    }
    let expected_per_day = (CLUBS_PER_LEAGUE / 2) as u32; // 10 for n=20
    for day in 1..=MATCH_DAYS_PER_SEASON {
        let count = per_day.get(&day).copied().unwrap_or(0);
        assert_eq!(
            count, expected_per_day,
            "match-day {day} has {count} fixtures; expected {expected_per_day}"
        );
    }

    // Per-match-day: each club appears in exactly ONE fixture (no club
    // plays twice on the same day; the circle method's pair-coverage
    // already implies this but assert explicitly as a redundant guard).
    for day in 1..=MATCH_DAYS_PER_SEASON {
        let mut day_clubs: std::collections::BTreeSet<ClubId> = std::collections::BTreeSet::new();
        for f in league.fixtures.iter().filter(|f| f.match_day == day) {
            assert!(
                day_clubs.insert(f.home),
                "club {:?} appears twice on match-day {day} (home of multiple fixtures)",
                f.home
            );
            assert!(
                day_clubs.insert(f.away),
                "club {:?} appears twice on match-day {day} (away of multiple fixtures)",
                f.away
            );
        }
        assert_eq!(
            day_clubs.len(),
            CLUBS_PER_LEAGUE,
            "match-day {day} must include all {CLUBS_PER_LEAGUE} clubs exactly once; \
             got {} distinct clubs",
            day_clubs.len()
        );
    }
}

// ---------------------------------------------------------------------------
// AC5: determinism — same seed produces byte-identical League
// ---------------------------------------------------------------------------

#[test]
fn generate_league_is_deterministic_for_same_seed() {
    let content = load_content();
    let seed = Seed::from_u64(0xDEADC0DE);

    let league_a = generate_league(seed, &content).expect("generate_league a");
    let league_b = generate_league(seed, &content).expect("generate_league b");

    assert_eq!(
        league_a,
        league_b,
        "generate_league not deterministic: same seed produced different leagues. \
         a.name={:?}; b.name={:?}; a.clubs[0].display_name={:?}; b.clubs[0].display_name={:?}",
        league_a.name,
        league_b.name,
        league_a.clubs[0].display_name,
        league_b.clubs[0].display_name
    );

    // Additionally verify via serde JSON byte-identity for paranoia
    // (catches future PartialEq impls that ignore some field).
    let json_a = serde_json::to_string(&league_a).expect("serialize a");
    let json_b = serde_json::to_string(&league_b).expect("serialize b");
    assert_eq!(
        json_a, json_b,
        "generate_league serde JSON not byte-identical for same seed"
    );
}

// ---------------------------------------------------------------------------
// AC6: different seeds produce different leagues
// ---------------------------------------------------------------------------

#[test]
fn generate_league_differs_across_seeds() {
    let content = load_content();
    let league_a = generate_league(Seed::from_u64(0xAAAA_AAAA), &content).expect("league a");
    let league_b = generate_league(Seed::from_u64(0xBBBB_BBBB), &content).expect("league b");

    // The clubs are procgen'd via different per-club seeds → at LEAST one
    // club's display_name should differ across the two leagues. (Realistic
    // expectation: most or all 20 differ; allowing 1 collision via the
    // any() check gives the markov chain room for an unlikely shared draw.)
    let any_name_differs = league_a
        .clubs
        .iter()
        .zip(league_b.clubs.iter())
        .any(|(a, b)| a.display_name != b.display_name);

    assert!(
        any_name_differs,
        "generate_league produced identical club names for different seeds — \
         either the seed-derivation is broken OR the markov chain is degenerate. \
         a.clubs[0..3]: {:?}; b.clubs[0..3]: {:?}",
        league_a
            .clubs
            .iter()
            .take(3)
            .map(|c| &c.display_name)
            .collect::<Vec<_>>(),
        league_b
            .clubs
            .iter()
            .take(3)
            .map(|c| &c.display_name)
            .collect::<Vec<_>>(),
    );
}

// ---------------------------------------------------------------------------
// T4-2.5b: generate_league_with_teams returns per-club ProcGenTeam
// ---------------------------------------------------------------------------

/// AC4: generate_league_with_teams returns Vec<ProcGenTeam> whose team names
/// match League.clubs[i].display_name and whose players arrays have 22 entries.
#[test]
fn generate_league_with_teams_returns_matching_procgen_teams() {
    let content = load_content();
    let seed = Seed::from_u64(0xDECA_FBAD);
    let (league, procgen_teams) =
        generate_league_with_teams(seed, &content).expect("generate_league_with_teams");

    assert_eq!(
        procgen_teams.len(),
        league.clubs.len(),
        "one ProcGenTeam per club"
    );

    for (i, (club, team)) in league.clubs.iter().zip(procgen_teams.iter()).enumerate() {
        assert_eq!(
            club.display_name, team.team_name,
            "club[{i}] display_name must match ProcGenTeam.team_name; \
             no recomputation allowed"
        );
        assert_eq!(
            team.players.len(),
            22,
            "ProcGenTeam[{i}] must have 22 player names"
        );
        for (slot, player) in team.players.iter().enumerate() {
            assert!(
                !player.first.is_empty() && !player.last.is_empty(),
                "ProcGenTeam[{i}].players[{slot}] has empty name: {player:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// S9: within-league team name uniqueness (non-vacuous, asserts the actual fix)
// ---------------------------------------------------------------------------

/// All 20 clubs in a generated league must have distinct display names.
///
/// This is the non-vacuous guard for the S9 dedup fix.  Before S9, clubs
/// sharing a culture drew independently from a 10-name bank — birthday
/// paradox near-guaranteed at least one collision for 10 clubs per culture.
/// The fix threads a `BTreeSet<String>` of used names per culture through
/// `generate_league_with_teams` so each re-draw avoids taken names.
///
/// Tests two seeds to catch potential degenerate single-seed luck.
#[test]
fn within_league_club_names_are_all_distinct() {
    let content = load_content();

    for &seed_val in &[0xC0FFEE_u64, 0xDEADC0DE_u64] {
        let league = generate_league(Seed::from_u64(seed_val), &content).expect("generate_league");

        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for club in &league.clubs {
            assert!(
                seen.insert(club.display_name.as_str()),
                "seed {seed_val:#x}: duplicate club name {:?} found in league — \
                 S9 dedup is broken",
                club.display_name
            );
        }

        assert_eq!(
            seen.len(),
            CLUBS_PER_LEAGUE,
            "seed {seed_val:#x}: expected {CLUBS_PER_LEAGUE} distinct names, got {}",
            seen.len()
        );
    }
}

/// Uniqueness holds across a range of seeds, not just two cherry-picked values.
///
/// Uses the same 5-seed sweep so the test stays fast while covering more
/// of the seed space.
#[test]
fn within_league_club_names_unique_across_several_seeds() {
    let content = load_content();

    let seeds: &[u64] = &[
        0x0000_0000_0000_0001,
        0xAAAA_AAAA_AAAA_AAAA,
        0xBBBB_BBBB_BBBB_BBBB,
        0x1234_5678_9ABC_DEF0,
        0xFFFF_FFFF_FFFF_FFFF,
    ];

    for &seed_val in seeds {
        let league = generate_league(Seed::from_u64(seed_val), &content).expect("generate_league");

        let names: BTreeSet<&str> = league
            .clubs
            .iter()
            .map(|c| c.display_name.as_str())
            .collect();
        assert_eq!(
            names.len(),
            CLUBS_PER_LEAGUE,
            "seed {seed_val:#x}: {}/{} distinct names — duplicate club names detected. \
             Names: {:?}",
            names.len(),
            CLUBS_PER_LEAGUE,
            league
                .clubs
                .iter()
                .map(|c| c.display_name.as_str())
                .collect::<Vec<_>>()
        );
    }
}

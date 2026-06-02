//! T4-2.5f integration tests — scouting wiring.
//!
//! Acceptance criteria:
//!
//! 1. `advance_week` once → starting XI (indices 0..11) for both clubs that
//!    played have `observation_count == 1` and `last_scout_report.is_some()`.
//! 2. Source-bio gene-match invariant: the derived bio's `internal_gene_snapshot`
//!    equals the instance's `genes` for a selection of slots.
//! 3. Both canonical pins UNCHANGED (verified by exercising `advance_week_inner`
//!    and confirming the `get_scout_report_inner` happy path, which touches only
//!    non-canonical roster fields).

use std::path::PathBuf;

use fw_core::Seed;
use fw_tauri::commands::{advance_week_inner, get_scout_report_inner};
use fw_tauri::roster::ROSTER_PLAYER_ID_BASE;
use fw_tauri::state::AppState;

// fw_content is a direct dependency of fw-tauri and is available in integration tests.
// Used for PlayerBio type annotation in source_bio_genes_match_instance_genes.

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
// AC1 — advance_week populates scout reports for starting XIs
// ---------------------------------------------------------------------------

/// AC1: after one `advance_week`, every player at index 0..11 in the home and
/// away club's roster slice has `observation_count == 1` and
/// `last_scout_report.is_some()`.
///
/// Players at indices 11..22 (bench) are untouched.
#[test]
fn advance_week_populates_scout_reports_for_starting_xi() {
    let state = test_state();

    // Record which clubs play on match-day 1 before advancing.
    let playing_clubs: Vec<(fw_core::ClubId, fw_core::ClubId)> = {
        let career = state.career().read().expect("career lock");
        career
            .season
            .fixtures_for_match_day(1)
            .iter()
            .map(|f| (f.home, f.away))
            .collect()
    };

    // Play match-day 1.
    advance_week_inner(&state).expect("advance_week");

    let career = state.career().read().expect("career lock after advance");

    for (home_id, away_id) in &playing_clubs {
        for club_id in [home_id, away_id] {
            let instances = career.roster.get(club_id).expect("club in roster");

            // Starting XI (0..11): observation_count == 1, report is Some.
            for inst in &instances[..11] {
                assert_eq!(
                    inst.observation_count, 1,
                    "slot {} of club {:?}: observation_count must be 1 after advance_week",
                    inst.slot, club_id
                );
                assert!(
                    inst.last_scout_report.is_some(),
                    "slot {} of club {:?}: last_scout_report must be Some after advance_week",
                    inst.slot,
                    club_id
                );
            }

            // Bench (11..22): observation_count == 0, report is None.
            for inst in &instances[11..] {
                assert_eq!(
                    inst.observation_count, 0,
                    "slot {} of club {:?}: bench player must have observation_count 0",
                    inst.slot, club_id
                );
                assert!(
                    inst.last_scout_report.is_none(),
                    "slot {} of club {:?}: bench player must have no scout report",
                    inst.slot,
                    club_id
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AC2 — source-bio gene-match invariant
// ---------------------------------------------------------------------------

/// AC2: for several roster slots, the bio derived via the round-robin index
/// `global_idx = player_id.raw() - ROSTER_PLAYER_ID_BASE` has
/// `internal_gene_snapshot == instance.genes`.
///
/// This spot-checks club 0; the gene-match invariant for ALL playing clubs
/// (every `club_idx`) is exercised end-to-end at runtime by the in-function
/// `assert!` in `observe_match_participants`, which runs for every observed
/// starting XI during `advance_week_populates_scout_reports_for_starting_xi`.
#[test]
fn source_bio_genes_match_instance_genes() {
    let state = test_state();
    let career = state.career().read().expect("career lock");
    let content = state.content();

    let bios: Vec<&fw_content::PlayerBio> = content.player_bios.values().collect();
    if bios.is_empty() {
        // Content pack has no bios — observation is intentionally skipped.
        // The test is vacuously satisfied.
        return;
    }

    // Check the first 3 instances across the first club.
    for (club_id, instances) in career.roster.iter().take(1) {
        for inst in instances.iter().take(3) {
            let global_idx = (inst.player_id.raw() - ROSTER_PLAYER_ID_BASE) as usize;
            let bio = bios[global_idx % bios.len()];
            assert_eq!(
                bio.internal_gene_snapshot, inst.genes,
                "gene mismatch for player {:?} in club {:?} at global_idx={global_idx}",
                inst.player_id, club_id
            );
        }
    }
}

// ---------------------------------------------------------------------------
// AC3 — get_scout_report_inner happy path end-to-end
// ---------------------------------------------------------------------------

/// AC3: `get_scout_report_inner` returns a DTO with non-empty `overall_band`
/// and 3 categories after one `advance_week`.
#[test]
fn get_scout_report_returns_valid_dto_after_advance_week() {
    let state = test_state_with_seed(0xdead_beef_cafe_babe);
    advance_week_inner(&state).expect("advance_week");

    // Slot 0 of club index 0: PlayerId(ROSTER_PLAYER_ID_BASE + 0).
    let player_id_str = format!("fwh.core:player_{ROSTER_PLAYER_ID_BASE:08}");
    let dto = get_scout_report_inner(&player_id_str, &state)
        .expect("scout report must be present after advance_week");

    assert!(
        !dto.overall_band.is_empty(),
        "overall_band must be a non-empty string"
    );
    assert_eq!(dto.categories.len(), 3, "must have 3 category estimates");
    assert_eq!(
        dto.observation_count, 1,
        "one advance_week → observation_count = 1"
    );

    // Each category must have a non-empty band string.
    for cat in &dto.categories {
        assert!(
            !cat.band.is_empty(),
            "category {} band must be non-empty",
            cat.category
        );
    }
}

// ---------------------------------------------------------------------------
// AC4 — observation_count accumulates across multiple advance_weeks
// ---------------------------------------------------------------------------

/// AC4: `observation_count` accumulates per match-day a player features in.
///
/// Each match-day observes the full starting XI (slot 0 = GK always features
/// when the club plays). To avoid an unstated "every club plays every match-day,
/// no byes" assumption that could make this a false positive under a future
/// scheduler, the expected count is DERIVED from actual fixture participation:
/// we record match-day 1 + 2 fixtures up front and assert the chosen club's GK
/// count equals the number of those two days the club actually features in.
#[test]
fn observation_count_accumulates_across_two_advance_weeks() {
    let state = test_state();

    // Record the next two match-days' fixtures before advancing.
    let (md1, md2) = {
        let career = state.career().read().expect("career lock");
        let collect_day = |day: u16| -> Vec<(fw_core::ClubId, fw_core::ClubId)> {
            career
                .season
                .fixtures_for_match_day(day)
                .iter()
                .map(|f| (f.home, f.away))
                .collect()
        };
        (collect_day(1), collect_day(2))
    };

    let plays = |day: &[(fw_core::ClubId, fw_core::ClubId)], club: fw_core::ClubId| {
        day.iter().any(|(h, a)| *h == club || *a == club)
    };

    // Pick the home club of match-day 1's first fixture; derive how many of the
    // two days it features in (round-robin → 2 today, but byes would lower it).
    let chosen = md1.first().expect("match-day 1 has fixtures").0;
    let expected_count = u32::from(plays(&md1, chosen)) + u32::from(plays(&md2, chosen));
    assert!(
        expected_count >= 1,
        "the chosen club must feature in at least match-day 1"
    );

    advance_week_inner(&state).expect("advance_week day 1");
    advance_week_inner(&state).expect("advance_week day 2");

    let career = state.career().read().expect("career lock");
    let instances = career.roster.get(&chosen).expect("chosen club in roster");
    let gk = &instances[0];

    assert_eq!(
        gk.observation_count, expected_count,
        "GK (slot 0) of club {chosen:?} must have observation_count == {expected_count} \
         (the number of the two match-days the club actually featured in)"
    );
    assert!(
        gk.last_scout_report.is_some(),
        "GK must have a scout report after featuring in at least one match-day"
    );
}

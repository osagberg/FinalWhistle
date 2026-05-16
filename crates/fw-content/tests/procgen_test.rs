//! Integration tests for T1-7 procedural content generation.
//!
//! All 5 acceptance criteria from MEMORY.md T1-7 §7 are covered:
//! (a) same_seed_produces_identical_team
//! (b) two_teams_from_one_seed_have_distinct_players
//! (c) markov_diversity_does_not_echo_input_corpus_verbatim
//! (d) manager_archetype_loads_with_valid_tactical_archetype_reference
//! (e) banned_terms_lint_clean_over_1000_generated_names
//!
//! Uses the on-disk `content/sources/` fixtures for (c), (d), (e) so
//! the tests exercise the real content pipeline (not mocks).

use std::path::PathBuf;

use fw_content::{
    procgen::{ProcGenInputs, generate_team, train_culture_chain},
    runtime::ContentStore,
};
use fw_core::Seed;
use rand::SeedableRng;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn content_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

fn load_store() -> ContentStore {
    ContentStore::load_sources(&content_root())
        .expect("content/sources/ must load cleanly for procgen integration tests")
}

/// The canonical test seed for T1-7.
const PROCGEN_T1_7_SEED: u64 = 0x0000_0000_0001_0007;

// ---------------------------------------------------------------------------
// (a) same_seed_produces_identical_team
// ---------------------------------------------------------------------------

#[test]
fn same_seed_produces_identical_team() {
    let store = load_store();
    let seed = Seed::from_u64(PROCGEN_T1_7_SEED);

    let a = generate_team(
        &store,
        ProcGenInputs {
            culture_id: "fwh.core:culture.anglo",
            tactical_archetype_id: "fwh.core:archetype.low-block-counter",
            manager_archetype_id: "fwh.core:manager.pragmatic-defender",
            seed,
        },
    )
    .expect("generate team A");

    let b = generate_team(
        &store,
        ProcGenInputs {
            culture_id: "fwh.core:culture.anglo",
            tactical_archetype_id: "fwh.core:archetype.low-block-counter",
            manager_archetype_id: "fwh.core:manager.pragmatic-defender",
            seed,
        },
    )
    .expect("generate team B");

    assert_eq!(a.team_name, b.team_name, "team name must be identical");
    assert_eq!(
        a.manager.first, b.manager.first,
        "manager first must be identical"
    );
    assert_eq!(
        a.manager.last, b.manager.last,
        "manager last must be identical"
    );
    for i in 0..22 {
        assert_eq!(
            a.players[i].first, b.players[i].first,
            "player {i} first must be identical"
        );
        assert_eq!(
            a.players[i].last, b.players[i].last,
            "player {i} last must be identical"
        );
    }
}

// ---------------------------------------------------------------------------
// (b) two_teams_from_one_seed_have_distinct_players
// ---------------------------------------------------------------------------

#[test]
fn two_teams_from_one_seed_have_distinct_players() {
    let store = load_store();

    // Team A uses seed N; team B uses seed N+1.
    // Both are generated from the same content but different seeds.
    let team_a = generate_team(
        &store,
        ProcGenInputs {
            culture_id: "fwh.core:culture.anglo",
            tactical_archetype_id: "fwh.core:archetype.low-block-counter",
            manager_archetype_id: "fwh.core:manager.pragmatic-defender",
            seed: Seed::from_u64(PROCGEN_T1_7_SEED),
        },
    )
    .expect("team A");

    let team_b = generate_team(
        &store,
        ProcGenInputs {
            culture_id: "fwh.core:culture.anglo",
            tactical_archetype_id: "fwh.core:archetype.low-block-counter",
            manager_archetype_id: "fwh.core:manager.pragmatic-defender",
            seed: Seed::from_u64(PROCGEN_T1_7_SEED.wrapping_add(1)),
        },
    )
    .expect("team B");

    // Count overlapping display names between the two rosters.
    let set_a: std::collections::BTreeSet<String> =
        team_a.players.iter().map(|p| p.display()).collect();
    let set_b: std::collections::BTreeSet<String> =
        team_b.players.iter().map(|p| p.display()).collect();

    let overlap: std::collections::BTreeSet<&String> = set_a.intersection(&set_b).collect();
    let overlap_count = overlap.len();

    // ≥80% non-overlap = at most 4 shared names out of 22.
    assert!(
        overlap_count <= 4,
        "expected ≤4 shared player names between two teams, got {overlap_count}. \
         Overlap: {overlap:?}"
    );
}

// ---------------------------------------------------------------------------
// (c) markov_diversity_does_not_echo_input_corpus_verbatim
// ---------------------------------------------------------------------------

#[test]
fn markov_diversity_does_not_echo_input_corpus_verbatim() {
    let store = load_store();
    let culture = store
        .cultures
        .get("fwh.core:culture.anglo")
        .expect("anglo culture must be loaded");

    // Build the input bank set for membership checking.
    let input_bank: std::collections::BTreeSet<String> = culture
        .first_name_bank
        .iter()
        .map(|s| s.to_lowercase())
        .collect();

    let chain = train_culture_chain(culture).expect("train chain");

    // Generate 100 first names and count how many are NOT in the input bank.
    let mut novel_count = 0usize;
    for i in 0u64..100 {
        let rng_seed = fw_core::seed_fn(
            PROCGEN_T1_7_SEED,
            0,
            fw_core::SeedLayer::ContentBake,
            i as u32,
        );
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(rng_seed);
        let name = chain.sample(&mut rng).expect("sample");
        if !input_bank.contains(&name.to_lowercase()) {
            novel_count += 1;
        }
    }

    assert!(
        novel_count >= 30,
        "Markov chain should produce ≥30 novel names out of 100 (got {novel_count}). \
         If the chain only echoes the training corpus verbatim it fails the diversity \
         acceptance criterion."
    );
}

// ---------------------------------------------------------------------------
// (d) manager_archetype_loads_with_valid_tactical_archetype_reference
// ---------------------------------------------------------------------------

#[test]
fn manager_archetype_loads_with_valid_tactical_archetype_reference() {
    let store = load_store();

    // The fixture must be present.
    let manager = store
        .managers
        .get("fwh.core:manager.pragmatic-defender")
        .expect("pragmatic-defender manager archetype must be loaded");

    assert_eq!(manager.schema_version, 1);
    assert_eq!(manager.id.as_str(), "fwh.core:manager.pragmatic-defender");
    assert_eq!(manager.display_name, "Pragmatic Defender");

    // The tactical_archetype_id must resolve in the content store.
    let resolved = store
        .tactical_archetypes
        .get(&manager.tactical_archetype_id);
    assert!(
        resolved.is_some(),
        "tactical_archetype_id {:?} on pragmatic-defender must resolve in ContentStore::tactical_archetypes",
        manager.tactical_archetype_id
    );

    // Risk + possession Q32 values are approximately 0.3 and 0.4 respectively.
    // Use f64 conversion for the assertion (bake-time path; acceptable in tests).
    let risk_f64 = manager.risk_appetite.to_bits() as f64 / (1u64 << 32) as f64;
    let poss_f64 = manager.possession_preference.to_bits() as f64 / (1u64 << 32) as f64;
    assert!(
        (risk_f64 - 0.3).abs() < 1e-9,
        "risk_appetite should be ≈ 0.3, got {risk_f64}"
    );
    assert!(
        (poss_f64 - 0.4).abs() < 1e-9,
        "possession_preference should be ≈ 0.4, got {poss_f64}"
    );
}

// ---------------------------------------------------------------------------
// (e) banned_terms_lint_clean_over_1000_generated_names
// ---------------------------------------------------------------------------

/// Banned terms that must not appear in any generated name.
/// Sourced from docs/design/ui-vocabulary.md A.1–A.5 (Category A hard bans).
/// This list covers the capitalized state-nouns and RPG vocabulary that would
/// make generated player names sound like a JRPG.
const BANNED_FRAGMENTS: &[&str] = &[
    "Hush",
    "Awakened",
    "Kismet",
    "Soul",
    "Flow",
    "Canon",
    "Ledger",
    "Resonance",
    "Cascade",
    "Ascendant", // covered in team names too
    "Calling",
    "Weather",
];

#[test]
fn banned_terms_lint_clean_over_1000_generated_names() {
    let store = load_store();
    let culture = store
        .cultures
        .get("fwh.core:culture.anglo")
        .expect("anglo culture must be loaded");

    let chain = train_culture_chain(culture).expect("train chain");

    for i in 0u64..1000 {
        let rng_seed = fw_core::seed_fn(
            PROCGEN_T1_7_SEED.wrapping_add(0x1_0000),
            0,
            fw_core::SeedLayer::ContentBake,
            i as u32,
        );
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(rng_seed);
        let name = chain.sample(&mut rng).expect("sample");

        for banned in BANNED_FRAGMENTS {
            assert!(
                !name.contains(banned),
                "generated name {name:?} contains banned term {banned:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Smoke: generate_team with the real Anglo culture + low-block-counter
// ---------------------------------------------------------------------------

#[test]
fn generate_team_with_real_fixtures_produces_valid_team() {
    let store = load_store();
    let team = generate_team(
        &store,
        ProcGenInputs {
            culture_id: "fwh.core:culture.anglo",
            tactical_archetype_id: "fwh.core:archetype.low-block-counter",
            manager_archetype_id: "fwh.core:manager.pragmatic-defender",
            seed: Seed::from_u64(0x1234),
        },
    )
    .expect("generate_team with real fixtures");

    // Team name must come from the anglo team_name_bank.
    let anglo = store.cultures.get("fwh.core:culture.anglo").unwrap();
    assert!(
        anglo.team_name_bank.contains(&team.team_name),
        "team name {:?} must come from anglo team_name_bank: {:?}",
        team.team_name,
        anglo.team_name_bank
    );

    // All 22 player names + manager names must be non-empty.
    assert!(!team.manager.first.is_empty());
    assert!(!team.manager.last.is_empty());
    for (i, p) in team.players.iter().enumerate() {
        assert!(!p.first.is_empty(), "player {i} first empty");
        assert!(!p.last.is_empty(), "player {i} last empty");
    }
}

// ---------------------------------------------------------------------------
// Elvish culture can also generate a team
// ---------------------------------------------------------------------------

#[test]
fn generate_team_with_elvish_culture_succeeds() {
    let store = load_store();

    // Elvish doesn't have a dedicated manager archetype yet; reuse pragmatic-defender.
    // The manager_archetype_id is validated only for existence, not cultural match.
    let team = generate_team(
        &store,
        ProcGenInputs {
            culture_id: "fwh.core:culture.fantasy-elvish",
            tactical_archetype_id: "fwh.core:archetype.low-block-counter",
            manager_archetype_id: "fwh.core:manager.pragmatic-defender",
            seed: Seed::from_u64(0xE1F1),
        },
    )
    .expect("generate elvish team");

    let elvish = store
        .cultures
        .get("fwh.core:culture.fantasy-elvish")
        .unwrap();
    assert!(
        elvish.team_name_bank.contains(&team.team_name),
        "elvish team name {:?} must come from elvish team_name_bank",
        team.team_name
    );
    assert_eq!(team.players.len(), 22);
}

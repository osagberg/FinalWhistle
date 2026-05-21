//! Integration tests for `PlayerBio` fixtures and `ContentStore` loading (T2-4).
//!
//! AC5: `ContentStore` loads all 22 player bios into `store.player_bios`.
//! AC6: all 22 `player_id`s are distinct; each survives a RON encode→decode
//!      round-trip bit-identical.
//!
//! AC4 (the `PlayerBioValidator` round-trip over all 22 fixtures) lives in
//! `crates/fw-content-baker/tests/player_bio_fixtures_test.rs` — the validator
//! is a `fw-content-baker` type, and exercising it from `fw-content`'s own
//! tests would force a `fw-content → fw-content-baker` dev-dep back-edge.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use fw_content::signature::RoleFamily;
use fw_content::{ContentStore, PhenotypeLabelId, PlayerBio};

/// Resolve the workspace-root `content/` directory from CARGO_MANIFEST_DIR.
/// This crate is at `crates/fw-content/`, so workspace root = `../../`.
fn content_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

/// Load the 22 fixtures directly from `content/sources/player-bios/` via RON,
/// bypassing the `ContentStore` loader (so we can test them before
/// `ContentStore` is the only path).
fn load_all_fixtures() -> Vec<PlayerBio> {
    let bios_dir = content_root().join("sources").join("player-bios");
    assert!(
        bios_dir.is_dir(),
        "content/sources/player-bios/ must exist; dir = {}",
        bios_dir.display()
    );

    let mut paths: Vec<_> = std::fs::read_dir(&bios_dir)
        .expect("read player-bios dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("ron"))
        .collect();
    // Sort for deterministic order.
    paths.sort();
    assert_eq!(
        paths.len(),
        22,
        "expected exactly 22 .ron fixtures in player-bios/"
    );

    paths
        .iter()
        .map(|p| {
            let raw =
                std::fs::read_to_string(p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()));
            ron::de::from_str::<PlayerBio>(&raw)
                .unwrap_or_else(|e| panic!("parse {}: {e}", p.display()))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Fixture roster shape: scout-label coverage + per-role-family labels
// ---------------------------------------------------------------------------

#[test]
fn fixtures_collectively_cover_diverse_scout_labels() {
    let bios = load_all_fixtures();
    let mut all_labels: BTreeSet<PhenotypeLabelId> = BTreeSet::new();
    for bio in &bios {
        for &label in &bio.scout_labels {
            all_labels.insert(label);
        }
    }
    // A roster of 22 should cover at least 30 of the 46 available labels.
    // This is a soft quality bar, not the exact-46 constraint.
    assert!(
        all_labels.len() >= 30,
        "22 fixtures should collectively exercise at least 30 of the 46 scout labels; \
         got {}: {:?}",
        all_labels.len(),
        all_labels
    );
    // Verify each role family's canonical labels appear on a matching player.
    // Spot-check: GK labels appear on a GK.
    let gk_bios: Vec<_> = bios
        .iter()
        .filter(|b| b.role_family == RoleFamily::Goalkeeper)
        .collect();
    let gk_labels: BTreeSet<PhenotypeLabelId> = gk_bios
        .iter()
        .flat_map(|b| b.scout_labels.iter().copied())
        .collect();
    let gk_role_labels = [
        PhenotypeLabelId::SweeperKeeper,
        PhenotypeLabelId::LineKeeper,
        PhenotypeLabelId::CrossClaimer,
    ];
    let gk_role_labels_present = gk_role_labels
        .iter()
        .filter(|l| gk_labels.contains(l))
        .count();
    assert!(
        gk_role_labels_present >= 1,
        "at least one GK role-specific label must appear on a Goalkeeper fixture; \
         got gk_labels = {:?}",
        gk_labels
    );
}

// ---------------------------------------------------------------------------
// AC5: ContentStore loads all 22 player bios
// ---------------------------------------------------------------------------

#[test]
fn content_store_loads_22_player_bios() {
    let store = ContentStore::load_sources(&content_root())
        .expect("ContentStore::load_sources must succeed");

    assert_eq!(
        store.player_bios.len(),
        22,
        "ContentStore must have loaded exactly 22 player bios from content/sources/player-bios/"
    );

    // All 8 role families must appear in the store's bios.
    let families: BTreeSet<u8> = store
        .player_bios
        .values()
        .map(|b| b.role_family as u8)
        .collect();
    assert_eq!(
        families.len(),
        8,
        "all 8 RoleFamily variants must appear in the loaded ContentStore.player_bios"
    );
}

// ---------------------------------------------------------------------------
// AC6: ID uniqueness + RON round-trip stability
// ---------------------------------------------------------------------------

#[test]
fn player_ids_unique_and_round_trip_stable() {
    let bios = load_all_fixtures();

    // All 22 IDs must be distinct.
    let ids: Vec<&str> = bios.iter().map(|b| b.player_id.as_str()).collect();
    let mut id_set: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, id) in ids.iter().enumerate() {
        if let Some(prev) = id_set.insert(id, i) {
            panic!(
                "duplicate player_id {:?} at fixture indices {} and {}",
                id, prev, i
            );
        }
    }
    assert_eq!(id_set.len(), 22, "must have 22 distinct player_ids");

    // Each PlayerBio must survive a RON encode→decode round-trip bit-identically.
    for bio in &bios {
        let encoded = ron::ser::to_string(bio)
            .unwrap_or_else(|e| panic!("RON encode failed for {}: {e}", bio.player_id));
        let decoded: PlayerBio = ron::de::from_str(&encoded)
            .unwrap_or_else(|e| panic!("RON decode failed for {}: {e}", bio.player_id));
        assert_eq!(
            decoded, *bio,
            "RON round-trip produced different result for {}",
            bio.player_id
        );
    }
}

// ---------------------------------------------------------------------------
// Sanity: schema_version is 1 across all fixtures
// ---------------------------------------------------------------------------

#[test]
fn all_fixtures_have_schema_version_1() {
    let bios = load_all_fixtures();
    for bio in &bios {
        assert_eq!(
            bio.schema_version, 1,
            "fixture {} has schema_version {} (expected 1)",
            bio.player_id, bio.schema_version
        );
    }
}

// ---------------------------------------------------------------------------
// Sanity: no fixture has empty scout_labels (validator already catches this,
// but belt-and-suspenders at the integration level)
// ---------------------------------------------------------------------------

#[test]
fn all_fixtures_have_non_empty_scout_labels() {
    let bios = load_all_fixtures();
    for bio in &bios {
        assert!(
            !bio.scout_labels.is_empty(),
            "fixture {} has empty scout_labels",
            bio.player_id
        );
    }
}

// ---------------------------------------------------------------------------
// Mutation-pre-check: PhenotypeLabelId::ALL has exactly 46 entries (non-vacuous)
// ---------------------------------------------------------------------------
// This test is also in player_bio.rs unit tests but duplicated at the
// integration level so the integration test binary also covers it.

#[test]
fn phenotype_all_has_exactly_46_entries() {
    assert_eq!(
        PhenotypeLabelId::ALL.len(),
        46,
        "PhenotypeLabelId::ALL must have exactly 46 entries"
    );
    // Spot-check that specific renamed variants exist (compile error if missing).
    let all: BTreeSet<PhenotypeLabelId> = PhenotypeLabelId::ALL.iter().copied().collect();
    assert!(all.contains(&PhenotypeLabelId::StrugglesUnderScrutiny));
    assert!(all.contains(&PhenotypeLabelId::PowerfulBallStriker));
    // Spot-check absence of removed variants: there is no PlateauRisk or InjuryProne
    // variant in the enum, so this is a compile-time guarantee (no runtime test needed).
    assert_eq!(all.len(), 46);
}

//! Fixture-load smoke tests — confirms every committed `.ron` under
//! `content/sources/` deserializes against its declared Rust type.
//!
//! Scope (T1-1):
//! - `cultures/*.ron`         → `Culture`
//! - `archetypes/*.ron`       → `TacticalArchetype`
//! - `role-affinities/*.ron`  → `RoleAffinityTable` + sum-to-10_000 invariant
//! - `players/*.ron`          → `PlayerTemplate`
//!
//! Wider corpus walking (mod overlays, ID-uniqueness across packs,
//! cross-reference resolution) is the FW-VAL gauntlet's job — that lands
//! in T2-3 with the baker. This file is just the schema-conformance smoke
//! test that gates T1-1.

use std::fs;
use std::path::{Path, PathBuf};

use fw_content::{
    BUILDUP_SPEED_MAX_BPS, BUILDUP_SPEED_MIN_BPS, ContentStore, Culture, PlayerTemplate,
    RoleAffinityTable, RoleId, SignatureDefinition, TacticalArchetype,
};

/// Walk a content directory and apply `parse` to every `.ron` file.
/// Panics with a helpful message on parse failure.
fn for_each_ron<T, F>(dir: &Path, parse: F) -> Vec<(PathBuf, T)>
where
    F: Fn(&str) -> Result<T, ron::error::SpannedError>,
{
    let mut results = Vec::new();
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {dir:?}: {e}"));
    for entry in entries {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("ron") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let parsed = parse(&text).unwrap_or_else(|e| panic!("parse {path:?} failed: {e}"));
        results.push((path, parsed));
    }
    results
}

fn content_sources_root() -> PathBuf {
    // `env!("CARGO_MANIFEST_DIR")` resolves to `crates/fw-content` at compile
    // time. Workspace root is two levels up.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
        .join("sources")
}

#[test]
fn cultures_fixtures_load() {
    let dir = content_sources_root().join("cultures");
    let cultures = for_each_ron::<Culture, _>(&dir, |s| ron::de::from_str(s));
    assert!(
        !cultures.is_empty(),
        "expected at least one culture fixture under {dir:?}"
    );
    for (path, c) in &cultures {
        assert!(
            !c.first_name_bank.is_empty(),
            "{path:?} has empty first_name_bank"
        );
        assert!(
            !c.last_name_bank.is_empty(),
            "{path:?} has empty last_name_bank"
        );
        assert!(
            c.id.starts_with("fwh."),
            "{path:?} id {:?} missing fwh.* prefix",
            c.id
        );
    }
}

#[test]
fn archetypes_fixtures_load_with_u16_bps() {
    let dir = content_sources_root().join("archetypes");
    let archetypes = for_each_ron::<TacticalArchetype, _>(&dir, |s| ron::de::from_str(s));
    assert!(
        !archetypes.is_empty(),
        "expected at least one tactical archetype fixture under {dir:?}"
    );
    for (path, a) in &archetypes {
        assert_eq!(
            a.formation.len(),
            11,
            "{path:?} formation has {} slots (expected 11)",
            a.formation.len()
        );
        // Confirms the buildup_speed_factor_bps field deserialized. Range
        // references `BUILDUP_SPEED_MIN_BPS..=BUILDUP_SPEED_MAX_BPS` from
        // `fw-content::runtime` — Codex audit P3 (2026-05-13): hardcoded
        // 5_000..=15_000 was drift-prone vs. the type-level constants
        // (which permit 5_000..=20_000). Reference the constants so the
        // test moves in lockstep with the spec.
        assert!(
            (BUILDUP_SPEED_MIN_BPS..=BUILDUP_SPEED_MAX_BPS).contains(&a.buildup_speed_factor_bps),
            "{path:?} buildup_speed_factor_bps {} outside sanctioned BUILDUP_SPEED_MIN_BPS..=BUILDUP_SPEED_MAX_BPS range ({BUILDUP_SPEED_MIN_BPS}..={BUILDUP_SPEED_MAX_BPS})",
            a.buildup_speed_factor_bps
        );
    }
}

#[test]
fn role_affinities_fixtures_load_and_normalize() {
    let dir = content_sources_root().join("role-affinities");
    let tables = for_each_ron::<RoleAffinityTable, _>(&dir, |s| ron::de::from_str(s));
    assert!(
        !tables.is_empty(),
        "expected at least one role-affinity-table fixture under {dir:?}"
    );
    for (path, table) in &tables {
        assert_eq!(
            table.schema_version, 1,
            "{path:?} schema_version {} unexpected (T1-1 ships at 1)",
            table.schema_version
        );
        let invalid = table.invalid_roles();
        assert!(
            invalid.is_empty(),
            "{path:?}: {} role(s) with non-10_000 weight sums: {:?}",
            invalid.len(),
            invalid
        );
        let unknown = table.unknown_attribute_keys();
        assert!(
            unknown.is_empty(),
            "{path:?}: {} unknown attribute key(s) (typos / stale renames): {:?}",
            unknown.len(),
            unknown
        );
        // Confirm at least the 3 roles T1-1's done-criteria need.
        for required in ["GK", "CB", "AM"] {
            assert!(
                table.get(&RoleId::new(required)).is_some(),
                "{path:?} missing required role {required:?} for T1-1"
            );
        }
    }
}

#[test]
fn player_fixtures_load() {
    let dir = content_sources_root().join("players");
    let players = for_each_ron::<PlayerTemplate, _>(&dir, |s| ron::de::from_str(s));
    assert!(
        !players.is_empty(),
        "expected at least one player fixture under {dir:?}"
    );
    for (path, p) in &players {
        assert_eq!(
            p.schema_version, 1,
            "{path:?} schema_version {} unexpected (T1-1 ships at 1)",
            p.schema_version
        );
        assert!(
            p.qualified_id.starts_with("fwh."),
            "{path:?} qualified_id {:?} missing fwh.* prefix",
            p.qualified_id
        );
        assert!(!p.display_name.is_empty(), "{path:?} display_name is empty");
        assert!(
            !p.preferred_role.as_str().is_empty(),
            "{path:?} preferred_role is empty"
        );
    }
}

// ---------------------------------------------------------------------------
// T1-3 tests: signature fixtures + canonical-hash-unchanged assertion
// ---------------------------------------------------------------------------

/// All `content/sources/signatures/*.ron` must deserialize as
/// `SignatureDefinition` without error. Schema version must be 1.
#[test]
fn signature_fixtures_load() {
    let dir = content_sources_root().join("signatures");
    let sigs = for_each_ron::<SignatureDefinition, _>(&dir, |s| ron::de::from_str(s));
    assert!(
        !sigs.is_empty(),
        "expected at least one signature fixture under {dir:?}; \
         content/sources/signatures/no-op-stub.ron was added at T1-3"
    );
    for (path, sig) in &sigs {
        assert_eq!(
            sig.schema_version, 1,
            "{path:?} schema_version {} unexpected (T1-3 ships at 1)",
            sig.schema_version
        );
        let id = sig.id.as_str();
        assert!(
            id.contains(":signature."),
            "{path:?} id {id:?} missing ':signature.' segment"
        );
        assert!(
            !sig.display_name.is_empty(),
            "{path:?} display_name is empty"
        );
    }
}

/// The no-op stub must load via `ContentStore::load_sources` without panic,
/// and must appear in `store.signature_definitions`.
#[test]
fn no_op_stub_loads_via_content_store() {
    let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content");
    let store = ContentStore::load_sources(&content_root)
        .expect("ContentStore::load_sources failed — check fixture files");
    let sig = store
        .signature_definitions
        .get("fwh.core:signature.no-op-stub")
        .expect("no-op-stub signature not found in store after load");
    assert_eq!(sig.schema_version, 1);
    assert_eq!(sig.id.as_str(), "fwh.core:signature.no-op-stub");
}

/// Adding `signature_candidates` to `PlayerTemplate` and loading signatures
/// from disk must NOT affect the canonical match-state hash for the smoke
/// seed. PlayerTemplate is template data only; `MatchState::initial` uses
/// hardcoded positions + `mid_range_baseline()`, never loaded templates.
///
/// Hash rebaselined at T1-2b-iv to 18f1776c2f77939d32849dc72e05909caf78b93bf6ce50a1222b28f9c6a5d048
/// (canonical encoder VERSION 4→5; signature state fields added to MatchState).
/// Prior T1-3 hash: 1db6020c7ac3181fac9f73b2e30423708d9fdd55a846e38c8e81c8c7ab59c798
/// If this test fails without an authorized rebaseline, it is a scope-leak
/// signal — check encoder VERSION and canonical schema additions.
#[test]
fn signature_load_does_not_drift_canonical_hash() {
    use fw_core::Seed;
    use fw_match_sim::{MatchState, tick_match};

    const SMOKE_SEED: u64 = 0xdeadbeefdeadbeef;
    const SMOKE_TICKS: u32 = 60;
    // Rebaselined at T1-3.5: ball mutation + possession state + goal detection.
    // Changes: (1) pos_z now altitude axis (physics convention corrected from
    // X+Z=pitch to X+Y=pitch, Z=altitude), (2) possession + last_touched_by fields
    // added to MatchState, (3) encoder VERSION 7→8, (4) tick ordering changed
    // (goal detection + OOB clamp run before physics).
    // ADR-0012 trigger #1 authorized this rebaseline.
    // Prior T1-4a hash (MatchEvent emission, VERSION 6→7):
    //   02ab97d06e60f508f5076aa37cf371263c73d5fc104ab1448989cb5f5627e686
    // Prior T1-2b-fix P1-5/P2-9 hash:
    //   d376ba2624646f19e3061342f5854bc117ed0a35a2b99a13f51a143bc446fa93
    // Prior T1-2b-iv hash:
    //   18f1776c2f77939d32849dc72e05909caf78b93bf6ce50a1222b28f9c6a5d048
    // Represented as raw bytes so we can compare without a hex crate.
    const EXPECTED: [u8; 32] = [
        0x78, 0x2f, 0xcd, 0xe6, 0x5b, 0xa8, 0xa0, 0xfc, 0x12, 0xbb, 0x90, 0xaf, 0x1b, 0x61, 0xf7,
        0x7d, 0x8c, 0xd4, 0x03, 0x10, 0x3a, 0xb3, 0x67, 0x1b, 0x0d, 0x5d, 0x6b, 0x03, 0xe7, 0x5c,
        0x8c, 0x0f,
    ];

    // Load the content store (exercises the new signature loader).
    let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content");
    let _store =
        ContentStore::load_sources(&content_root).expect("ContentStore::load_sources failed");

    // Run the smoke seed through tick_match — independent of the loaded store.
    let seed = Seed::from_u64(SMOKE_SEED);
    let mut state = MatchState::initial(seed);
    for _ in 0..SMOKE_TICKS {
        state = tick_match(state);
    }
    let bytes = state.encode_canonical();
    let actual: [u8; 32] = blake3::hash(&bytes).into();

    assert_eq!(
        actual, EXPECTED,
        "\nCanonical-state hash drifted unexpectedly.\n\
         T1-3.5 rebaselined to 782fcde65ba8a0fc12bb90af1b61f77d8cd403103ab3671b0d5d6b03e75c8c0f\n\
         (ball mutation + possession state + goal detection; physics Z=altitude convention;\n\
          possession/last_touched_by fields; encoder VERSION 7→8; goal+OOB before physics).\n\
         ADR-0012 trigger #1 authorized this rebaseline.\n\
         If this drifts again, it must be an authorized rebaseline — ADR-0012 trigger #1 or #3.\n\
         Actual:   {:02x?}",
        actual
    );
}

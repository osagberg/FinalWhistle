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
    BUILDUP_SPEED_MAX_BPS, BUILDUP_SPEED_MIN_BPS, Culture, PlayerTemplate, RoleAffinityTable,
    RoleId, TacticalArchetype,
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

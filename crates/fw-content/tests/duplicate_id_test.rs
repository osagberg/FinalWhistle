//! T1-12 acceptance criterion: duplicate stable IDs within a content category
//! cause `ContentStore::load_sources` to return `ContentLoadError::DuplicateId`
//! (fail-closed) instead of silently overwriting the first fixture.
//!
//! One test per content category (5 total):
//!   1. culture
//!   2. archetype
//!   3. role_affinity_table
//!   4. player_template
//!   5. signature_definition
//!
//! Each test writes two RON files with the same stable ID into a tmpdir,
//! calls `ContentStore::load_sources`, and asserts the exact `DuplicateId`
//! error shape (kind + id + both paths). TDD mandate: these tests are RED
//! before Chunk 2 inserts `insert_unique` into `load_sources`, GREEN after.
//!
//! Tmpdir uniqueness: `tempfile::TempDir` per the T1-12 code-reviewer P1
//! fix-pass — auto-cleanup avoids the pid-only-uniqueness CI-retry collision
//! the original `make_tmpdir(pid, suffix)` approach was vulnerable to (no
//! cleanup + shared pid under parallel `cargo test` invocations would silently
//! reuse a previous run's leftover `.ron` files).

use std::fs;
use std::path::Path;

use fw_content::{ContentLoadError, ContentStore};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Write `content` to `path`, creating parent dirs as needed.
fn write_fixture(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|e| panic!("create_dir_all {parent:?}: {e}"));
    }
    fs::write(path, content).unwrap_or_else(|e| panic!("write {path:?}: {e}"));
}

// ---------------------------------------------------------------------------
// Minimal RON fixture strings
//
// These are intentionally terse — just enough for the RON parser to accept
// them as the target type. The duplicate-ID check fires BEFORE the commentary
// requirement (cultures/archetypes/players/role-affinities/signatures all load
// before commentary), so the tests never reach the commentary directory check.
// ---------------------------------------------------------------------------

const CULTURE_RON: &str = r#"Culture(
    id: "fwh.core:culture.dup-test",
    name: "Dup Test",
    first_name_bank: ["Alice"],
    last_name_bank: ["Bravo"],
    naming_pattern: "{first} {last}",
    weights: (
        first_alpha_diversity_bps: 5000,
        compound_last_chance_bps: 0,
    ),
)"#;

// FormationSlot needs role: a &str as RoleId — derived Deserialize via
// `#[serde(transparent)]` still works because `RoleId` was transparent-deserializing
// prior to Chunk 5. After Chunk 5 the manual Deserialize is in place.
const ARCHETYPE_RON: &str = r#"TacticalArchetype(
    id: "fwh.core:archetype.dup-test",
    formation: [
        FormationSlot(roster_slot: 1,  role: "GK",  x: -45, z: 0),
        FormationSlot(roster_slot: 2,  role: "RB",  x: -25, z: 20),
        FormationSlot(roster_slot: 3,  role: "RCB", x: -30, z: 8),
        FormationSlot(roster_slot: 4,  role: "LCB", x: -30, z: -8),
        FormationSlot(roster_slot: 5,  role: "LB",  x: -25, z: -20),
        FormationSlot(roster_slot: 6,  role: "RDM", x: -10, z: 6),
        FormationSlot(roster_slot: 7,  role: "LDM", x: -10, z: -6),
        FormationSlot(roster_slot: 8,  role: "RM",  x:  10, z: 14),
        FormationSlot(roster_slot: 9,  role: "CAM", x:  15, z: 0),
        FormationSlot(roster_slot: 10, role: "LM",  x:  10, z: -14),
        FormationSlot(roster_slot: 11, role: "ST",  x:  25, z: 0),
    ],
    press_radius_metres: 20,
    buildup_speed_factor_bps: 9000,
)"#;

// Minimal role-affinity table: single role ("GK") summing to 10_000.
const ROLE_AFFINITY_RON: &str = r#"RoleAffinityTable(
    schema_version: 1,
    id: "fwh.core:role-affinities.dup-test",
    roles: {
        "GK": (weights_bps: {
            "handling": 4000,
            "reflexes": 3000,
            "one_on_ones": 1500,
            "aerial_reach": 1000,
            "command_of_area": 500,
        }),
    },
)"#;

// Minimal player template — all Q32 values use (bits: 0) = 0.0 for brevity.
// ceiling: potential must be >= current for the validator; both 0.0 is valid.
const PLAYER_RON: &str = r#"PlayerTemplate(
    schema_version: 1,
    id: 99,
    qualified_id: "fwh.core:player_00099",
    display_name: "Dup Test Player",
    attributes: (
        technical: (
            finishing: (bits: 0), long_shots: (bits: 0), passing: (bits: 0),
            crossing: (bits: 0), first_touch: (bits: 0), technique: (bits: 0),
            dribbling: (bits: 0), heading: (bits: 0), tackling: (bits: 0),
            marking: (bits: 0), free_kicks: (bits: 0), penalty_taking: (bits: 0),
            corners: (bits: 0), long_throws: (bits: 0),
        ),
        mental: (
            anticipation: (bits: 0), composure: (bits: 0), decisions: (bits: 0),
            vision: (bits: 0), off_the_ball: (bits: 0), positioning: (bits: 0),
            concentration: (bits: 0), bravery: (bits: 0), teamwork: (bits: 0),
            flair: (bits: 0),
        ),
        physical: (
            pace: (bits: 0), acceleration: (bits: 0), stamina: (bits: 0),
            strength: (bits: 0), agility: (bits: 0), balance: (bits: 0),
            jumping_reach: (bits: 0), natural_fitness: (bits: 0),
        ),
        goalkeeper: (
            handling: (bits: 0), reflexes: (bits: 0), one_on_ones: (bits: 0),
            aerial_reach: (bits: 0), command_of_area: (bits: 0), kicking: (bits: 0),
        ),
        personality: (
            determination: (bits: 0), work_rate: (bits: 0), ambition: (bits: 0),
            professionalism: (bits: 0), loyalty: (bits: 0), temperament: (bits: 0),
            pressure_tolerance: (bits: 0), big_match_appetite: (bits: 0),
            adaptability: (bits: 0), aggression: (bits: 0), risk_appetite: (bits: 0),
            selflessness: (bits: 0), consistency: (bits: 0), versatility: (bits: 0),
        ),
        durability: (
            injury_proneness: (bits: 0),
            recovery_rate: (bits: 0),
            dirtiness: (bits: 0),
        ),
    ),
    ceiling: (
        current: (bits: 0),
        potential: (bits: 0),
    ),
    preferred_role: "AM",
    signature_candidates: [],
)"#;

const SIGNATURE_RON: &str = r#"SignatureDefinition(
    schema_version: 1,
    id: SignatureId("fwh.core:signature.dup-test"),
    display_name: "Dup Test Signature",
    role_family: CentralMidfielder,
    trigger: NoOpStub,
    bias_snapshot: (
        shoot_mul:   (bits: 4294967296),
        pass_mul:    (bits: 4294967296),
        dribble_mul: (bits: 4294967296),
        press_mul:   (bits: 4294967296),
        cover_mul:   (bits: 4294967296),
    ),
    presentation: (
        commentary_line_bank_id: "placeholder",
        camera_framing_hint: "default",
        schema_version: 1,
    ),
    cooldown: EveryTicks(600),
    stacking: Exclusive(category: BuildUp),
)"#;

// ---------------------------------------------------------------------------
// Helper: assert DuplicateId error fields match expected values.
// ---------------------------------------------------------------------------

fn assert_duplicate_id_error(
    result: Result<ContentStore, ContentLoadError>,
    expected_kind: &str,
    expected_id: &str,
) {
    let err = result.expect_err("expected DuplicateId error but load_sources succeeded");
    match err {
        ContentLoadError::DuplicateId {
            kind,
            id,
            path_first,
            path_dupe,
        } => {
            assert_eq!(
                kind, expected_kind,
                "DuplicateId.kind mismatch: got {kind:?}, expected {expected_kind:?}"
            );
            assert_eq!(
                id, expected_id,
                "DuplicateId.id mismatch: got {id:?}, expected {expected_id:?}"
            );
            // T1-12 fix-pass per code-reviewer P2: prior guard was
            // `path_first.exists() || !path_first.as_os_str().is_empty()`
            // which short-circuited to always-true on any non-empty path,
            // making the existence check unreachable. Direct existence
            // asserts catch the failure mode that matters: the variant
            // stored a real on-disk path, not e.g. `PathBuf::from("")`.
            assert!(
                path_first.exists(),
                "DuplicateId.path_first must point to an existing file: {path_first:?}"
            );
            assert!(
                path_dupe.exists(),
                "DuplicateId.path_dupe must point to an existing file: {path_dupe:?}"
            );
            assert_ne!(
                path_first, path_dupe,
                "DuplicateId.path_first and path_dupe must differ"
            );
        }
        other => panic!("expected ContentLoadError::DuplicateId, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Tests — one per content category
// ---------------------------------------------------------------------------

#[test]
fn duplicate_culture_id_is_rejected() {
    let tmp = TempDir::new().expect("TempDir::new");
    let root = tmp.path();
    let cultures_dir = root.join("sources").join("cultures");

    // Two files with the same ID. File names differ so both pass the
    // directory walk; only the ID collision is the trigger.
    write_fixture(&cultures_dir.join("a.ron"), CULTURE_RON);
    write_fixture(&cultures_dir.join("b.ron"), CULTURE_RON);

    let result = ContentStore::load_sources(root);
    assert_duplicate_id_error(result, "culture", "fwh.core:culture.dup-test");
}

#[test]
fn duplicate_archetype_id_is_rejected() {
    let tmp = TempDir::new().expect("TempDir::new");
    let root = tmp.path();
    let archetypes_dir = root.join("sources").join("archetypes");

    write_fixture(&archetypes_dir.join("a.ron"), ARCHETYPE_RON);
    write_fixture(&archetypes_dir.join("b.ron"), ARCHETYPE_RON);

    let result = ContentStore::load_sources(root);
    assert_duplicate_id_error(result, "archetype", "fwh.core:archetype.dup-test");
}

#[test]
fn duplicate_role_affinity_id_is_rejected() {
    let tmp = TempDir::new().expect("TempDir::new");
    let root = tmp.path();
    let role_aff_dir = root.join("sources").join("role-affinities");

    write_fixture(&role_aff_dir.join("a.ron"), ROLE_AFFINITY_RON);
    write_fixture(&role_aff_dir.join("b.ron"), ROLE_AFFINITY_RON);

    let result = ContentStore::load_sources(root);
    assert_duplicate_id_error(
        result,
        "role_affinity_table",
        "fwh.core:role-affinities.dup-test",
    );
}

#[test]
fn duplicate_player_template_id_is_rejected() {
    let tmp = TempDir::new().expect("TempDir::new");
    let root = tmp.path();
    let players_dir = root.join("sources").join("players");

    write_fixture(&players_dir.join("a.ron"), PLAYER_RON);
    write_fixture(&players_dir.join("b.ron"), PLAYER_RON);

    let result = ContentStore::load_sources(root);
    assert_duplicate_id_error(result, "player_template", "fwh.core:player_00099");
}

#[test]
fn duplicate_signature_definition_id_is_rejected() {
    let tmp = TempDir::new().expect("TempDir::new");
    let root = tmp.path();
    let sigs_dir = root.join("sources").join("signatures");

    write_fixture(&sigs_dir.join("a.ron"), SIGNATURE_RON);
    write_fixture(&sigs_dir.join("b.ron"), SIGNATURE_RON);

    let result = ContentStore::load_sources(root);
    assert_duplicate_id_error(
        result,
        "signature_definition",
        "fwh.core:signature.dup-test",
    );
}

//! T1-20 acceptance criterion: `ContentStore::load_sources` rejects a
//! `PlayerTemplate.signature_candidates[i].signature_id` that does not resolve
//! in `store.signature_definitions`, returning
//! `ContentLoadError::DanglingReference` with the expected from/to kinds + IDs.
//!
//! Pattern mirrors `duplicate_id_test.rs` (T1-12): TempDir-based isolation,
//! minimal-fixture overlay. The test copies the real `content/sources/` tree
//! into a fresh tempdir so the loader sees a complete + valid content corpus
//! (cultures + archetypes + role-affinities + signatures + commentary + managers
//! all present + cross-ref-clean), then APPENDS a single new player-template
//! RON file whose `signature_candidates[0].signature_id` points at a signature
//! definition that DOES NOT exist. The cross-ref validator (added at T1-20 in
//! `runtime.rs`, after the existing manager → tactical_archetype check) MUST
//! detect this and fail loudly with the dangling-ref error.
//!
//! Why copy real content vs hand-craft minimal fixtures: the commentary loader
//! requires all 6 `MatchEventDiscriminant` variants to be present (the loader
//! returns a hard error on absence per T1-12), so a minimal hand-crafted
//! fixture set would need to author 6 Tracery JSON files alongside cultures /
//! archetypes / role-affinities / signatures / managers. Copying real content
//! is shorter + closer to the actual load path the cross-ref check runs in
//! production.

use std::fs;
use std::path::{Path, PathBuf};

use fw_content::{ContentLoadError, ContentStore};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn real_content_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

/// Recursively copy `src` into `dst`. Creates `dst` if absent.
fn copy_dir_recursive(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap_or_else(|e| panic!("create_dir_all {dst:?}: {e}"));
    for entry in fs::read_dir(src).unwrap_or_else(|e| panic!("read_dir {src:?}: {e}")) {
        let entry = entry.unwrap();
        let path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);
        if path.is_dir() {
            copy_dir_recursive(&path, &dst_path);
        } else {
            fs::copy(&path, &dst_path)
                .unwrap_or_else(|e| panic!("copy {path:?} -> {dst_path:?}: {e}"));
        }
    }
}

/// A fresh PlayerTemplate RON with a unique qualified_id and one dangling
/// signature_candidate. All Q32 attributes use `(bits: 0)` = 0.0 (validators
/// accept zero values; range validation passes; ceiling validates because
/// 0.0 <= 0.0).
///
/// The dangling reference is `fwh.core:signature.does-not-exist` — a slug that
/// does NOT appear in any of the 4 real signature fixtures shipped under
/// `content/sources/signatures/`.
const DANGLING_PLAYER_RON: &str = r#"PlayerTemplate(
    schema_version: 1,
    id: 31337,
    qualified_id: "fwh.core:player_31337",
    display_name: "Dangling Sig Test Player",
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
    signature_candidates: [
        (
            signature_id: SignatureId("fwh.core:signature.does-not-exist"),
            affinity: (bits: 0),
        ),
    ],
)"#;

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[test]
fn dangling_signature_candidate_fails_load_with_structured_error() {
    let tmp = TempDir::new().expect("TempDir::new");
    let root = tmp.path();

    // Copy real content/sources/ into tempdir/sources/.
    copy_dir_recursive(&real_content_root().join("sources"), &root.join("sources"));

    // Append a new player template fixture with a dangling signature_candidate.
    let dangling_path = root.join("sources/players/dangling-sig.ron");
    fs::write(&dangling_path, DANGLING_PLAYER_RON)
        .unwrap_or_else(|e| panic!("write {dangling_path:?}: {e}"));

    // Sanity pre-condition: the qualified_id we're using is genuinely new
    // (no collision with the real content fixtures). If this asserts, the
    // test's qualified_id has drifted into a real fixture and the
    // DuplicateId error would mask the DanglingReference error.
    let result = ContentStore::load_sources(root);
    let err = result.expect_err(
        "load_sources must reject the dangling signature_candidate; \
         got Ok(_) instead — cross-ref check at runtime.rs may have regressed",
    );

    match err {
        ContentLoadError::DanglingReference {
            from_kind,
            from_id,
            from_path,
            to_kind,
            to_id,
        } => {
            assert_eq!(
                from_kind, "player_template",
                "DanglingReference.from_kind must be 'player_template'"
            );
            assert_eq!(
                from_id, "fwh.core:player_31337",
                "DanglingReference.from_id must be the dangling player's qualified_id"
            );
            assert_eq!(
                to_kind, "signature_definition",
                "DanglingReference.to_kind must be 'signature_definition'"
            );
            assert_eq!(
                to_id, "fwh.core:signature.does-not-exist",
                "DanglingReference.to_id must be the unresolved signature ID verbatim"
            );
            // from_path should be a real on-disk RON file. If empty / placeholder,
            // the runtime.rs path-tracking regressed (the `player_paths` BTreeMap
            // wasn't populated alongside `insert_unique`).
            assert!(
                from_path.exists(),
                "DanglingReference.from_path must point to an existing file: {from_path:?}"
            );
            assert!(
                from_path.ends_with("sources/players/dangling-sig.ron"),
                "DanglingReference.from_path must end with the offending fixture: {from_path:?}"
            );
        }
        other => panic!(
            "expected ContentLoadError::DanglingReference \
             (player_template → signature_definition), got: {other:?}"
        ),
    }
}

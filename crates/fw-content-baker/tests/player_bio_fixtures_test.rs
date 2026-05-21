//! T2-4 — `PlayerBioValidator` against the 22 committed `PlayerBio` fixtures.
//!
//! AC4: every fixture in `content/sources/player-bios/` loads via the real
//! `ContentStore::load_sources` path AND passes `PlayerBioValidator`; all 8
//! `RoleFamily` variants appear across the roster.
//!
//! This test lives in `fw-content-baker` (not `fw-content`) deliberately:
//! `PlayerBioValidator` is a `fw-content-baker` type, and `fw-content-baker`
//! already depends on `fw-content` (normal dep). Putting a validator-
//! exercising test in `fw-content`'s own `tests/` would force a
//! `fw-content → fw-content-baker` dev-dependency back-edge — an avoidable
//! crate-graph smell. The validator's tests belong in the validator's crate.

use std::collections::BTreeSet;
use std::path::PathBuf;

use fw_content::ContentStore;
use fw_content::signature::RoleFamily;
use fw_content_baker::validators::{PlayerBioRosterValidator, PlayerBioValidator};

/// Resolve the workspace-root `content/` directory from `CARGO_MANIFEST_DIR`.
/// This crate is at `crates/fw-content-baker/`, so workspace root = `../../`.
fn content_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

#[test]
fn all_22_fixtures_load_and_validate() {
    // Load through the real runtime path — `ContentStore::load_sources` is
    // what the shipping runtime uses, so this exercises the actual loader.
    let store = ContentStore::load_sources(&content_root())
        .expect("ContentStore::load_sources must succeed");

    assert_eq!(
        store.player_bios.len(),
        22,
        "expected exactly 22 player-bio fixtures loaded from \
         content/sources/player-bios/"
    );

    let validator = PlayerBioValidator::new();
    // Collect the NAMED RoleFamily variants (not `as u8` discriminants) so the
    // coverage assertion stays correct-for-the-right-reason if RoleFamily is
    // ever reordered — a discriminant-integer set would misfire silently.
    let mut role_families_seen: BTreeSet<RoleFamily> = BTreeSet::new();

    for bio in store.player_bios.values() {
        validator
            .validate(bio)
            .unwrap_or_else(|e| panic!("PlayerBioValidator rejected {}: {e}", bio.player_id));
        role_families_seen.insert(bio.role_family);
    }

    // All 8 RoleFamily variants must appear across the 22-fixture roster.
    let expected: BTreeSet<RoleFamily> = [
        RoleFamily::Goalkeeper,
        RoleFamily::CentreBack,
        RoleFamily::FullBack,
        RoleFamily::DefensiveMidfielder,
        RoleFamily::CentralMidfielder,
        RoleFamily::AttackingMidfielder,
        RoleFamily::Winger,
        RoleFamily::Striker,
    ]
    .into_iter()
    .collect();
    assert_eq!(
        role_families_seen,
        expected,
        "all 8 RoleFamily variants must appear across the 22 fixtures; missing: {:?}",
        expected.difference(&role_families_seen).collect::<Vec<_>>()
    );
}

/// T3-R-D — the real shipped `fwh.core` pack (22 hand-authored player bios)
/// passes `PlayerBioRosterValidator`. This anchors `MVP_ROSTER_SIZE` to the
/// shipped corpus: if the constant drifts, or a bio fixture is added/removed,
/// this test — and `validate-structural` — fails.
#[test]
fn shipped_pack_passes_the_roster_validator() {
    let store = ContentStore::load_sources(&content_root())
        .expect("ContentStore::load_sources must succeed");

    PlayerBioRosterValidator::new()
        .validate(&store.player_bios)
        .unwrap_or_else(|e| {
            panic!("PlayerBioRosterValidator rejected the shipped fwh.core pack: {e}")
        });
}

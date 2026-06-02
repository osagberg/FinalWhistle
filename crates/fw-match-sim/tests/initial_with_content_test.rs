//! Integration tests for `MatchState::initial_with_content` — T1-11 chunk 2.
//!
//! These tests verify:
//! 1. Slot 7 (home AM/MID) gets `signature_candidates` from sample-am.ron.
//! 2. Role-matched MID slots (5-7, 16-18) have candidates; GK/DEF/FWD empty.
//! 3. Same-seed → same canonical bytes (determinism preserved).
//! 4. Empty ContentStore (no templates) → Err (fail-loud).
//! 5. initial_with_content + tick_match still advances normally.
//! 6. with_slot_signatures overrides only the slots present in the map.

use std::collections::BTreeMap;
use std::path::PathBuf;

use fw_content::{ContentStore, SignatureDefinition};
use fw_core::Seed;
use fw_match_sim::{MatchState, tick_match};

/// Return the absolute path to the `content/` directory used by the test suite.
/// The canonical location is `<workspace_root>/content`.
fn content_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

/// Load the real ContentStore from committed fixtures.
fn load_store() -> ContentStore {
    ContentStore::load_sources(&content_root()).expect("ContentStore::load_sources failed")
}

// ---------------------------------------------------------------------------
// RED test 1: slot 7 has 3 signature_candidates after initial_with_content
// ---------------------------------------------------------------------------

#[test]
fn slot_7_has_signature_candidates_from_sample_am() {
    let store = load_store();
    let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);

    let state = MatchState::initial_with_content(
        seed,
        &store,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
    )
    .expect("initial_with_content should succeed");

    let candidates = state.players[7].signature_candidates();
    assert_eq!(
        candidates.len(),
        3,
        "slot 7 should have exactly 3 signature_candidates from sample-am.ron; got {}",
        candidates.len()
    );

    // Verify the specific IDs from sample-am.ron are present (order preserved from RON).
    let ids: Vec<&str> = candidates.iter().map(|c| c.signature_id.as_str()).collect();
    assert!(
        ids.contains(&"fwh.core:signature.no-op-stub"),
        "slot 7 should have no-op-stub candidate; got {:?}",
        ids
    );
    assert!(
        ids.contains(&"fwh.core:signature.first-time-diagonal-switch"),
        "slot 7 should have first-time-diagonal-switch candidate; got {:?}",
        ids
    );
    assert!(
        ids.contains(&"fwh.core:signature.long-range-strike"),
        "slot 7 should have long-range-strike candidate; got {:?}",
        ids
    );
}

// ---------------------------------------------------------------------------
// T4-2.5c test 2: role-matched MID slots (5-7, 16-18) have candidates;
//                 GK (0, 11), DEF (1-4, 12-15), FWD (8-10, 19-21) empty.
//
// With 1 AM template (preferred_role = "AM" → Role::Midfielder), only the
// 6 midfielder slots receive candidates. All others remain empty until
// per-role templates are added at T4.5-E1.
// ---------------------------------------------------------------------------

#[test]
fn role_matched_mid_slots_have_candidates_others_empty() {
    let store = load_store();
    let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);

    let state = MatchState::initial_with_content(
        seed,
        &store,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
    )
    .expect("initial_with_content should succeed");

    // MID slots in 4-3-3: home 5,6,7 and away 16,17,18 (= 11+5, 11+6, 11+7).
    // These receive AM candidates because preferred_role="AM" maps to Role::Midfielder.
    let mid_slots: &[usize] = &[5, 6, 7, 16, 17, 18];
    for &slot in mid_slots {
        assert!(
            !state.players[slot].signature_candidates().is_empty(),
            "MID slot {slot} should have non-empty candidates from the AM template \
             (preferred_role=AM → Role::Midfielder); got 0"
        );
    }

    // Non-MID slots: GK (0, 11), DEF (1-4, 12-15), FWD (8-10, 19-21) stay empty
    // until matching templates are added at T4.5-E1.
    let non_mid_slots: &[usize] = &[0, 1, 2, 3, 4, 8, 9, 10, 11, 12, 13, 14, 15, 19, 20, 21];
    for &slot in non_mid_slots {
        assert!(
            state.players[slot].signature_candidates().is_empty(),
            "Non-MID slot {slot} should have empty candidates (no template matches its role); \
             got {} candidates",
            state.players[slot].signature_candidates().len()
        );
    }
}

// ---------------------------------------------------------------------------
// RED test 3: determinism — same seed + store → same canonical bytes
// ---------------------------------------------------------------------------

#[test]
fn initial_with_content_is_deterministic() {
    let store = load_store();
    let seed = Seed::from_u64(0xCAFE_BABE_CAFE_BABE);

    let state_a = MatchState::initial_with_content(
        seed,
        &store,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
    )
    .expect("initial_with_content (a) failed");
    let state_b = MatchState::initial_with_content(
        seed,
        &store,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
    )
    .expect("initial_with_content (b) failed");

    assert_eq!(
        state_a.encode_canonical(),
        state_b.encode_canonical(),
        "initial_with_content is not deterministic: two runs with the same seed produced different canonical bytes"
    );
}

// ---------------------------------------------------------------------------
// RED test 4: initial_with_content differs from initial in encoding
// (slot 7's candidates change the canonical bytes)
// ---------------------------------------------------------------------------

#[test]
fn initial_with_content_canonical_differs_from_initial() {
    let store = load_store();
    let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);

    let state_plain = MatchState::initial(seed);
    let state_content = MatchState::initial_with_content(
        seed,
        &store,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
    )
    .expect("initial_with_content failed");

    assert_ne!(
        state_plain.encode_canonical(),
        state_content.encode_canonical(),
        "initial_with_content should produce different canonical bytes than initial (slot 7 candidates differ)"
    );
}

// ---------------------------------------------------------------------------
// T4-2.5c test 5: empty ContentStore (no templates) → Err (fail-loud)
// ---------------------------------------------------------------------------

#[test]
fn initial_with_content_fails_when_template_pool_empty() {
    // Use the default ContentStore which has empty player_templates.
    let empty_store = ContentStore::default();
    let seed = Seed::from_u64(0);

    let result = MatchState::initial_with_content(
        seed,
        &empty_store,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
    );
    assert!(
        result.is_err(),
        "initial_with_content with empty ContentStore (no templates) should return Err"
    );
}

// ---------------------------------------------------------------------------
// T4-2.5c test 6: with_slot_signatures overrides only the slots in the map;
//                 non-overridden slots keep their role-matched candidates
// ---------------------------------------------------------------------------

#[test]
fn with_slot_signatures_overrides_only_present_slots() {
    use fw_content::SignatureCandidate;
    use std::collections::BTreeMap;

    let store = load_store();
    let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);

    // Override slot 7 (home MID — has candidates from AM template) to empty.
    // Override slot 5 (home MID — also has candidates) to empty.
    let mut override_map: BTreeMap<u8, Vec<SignatureCandidate>> = BTreeMap::new();
    override_map.insert(7, Vec::new()); // MID slot 7 → explicitly empty
    override_map.insert(5, Vec::new()); // MID slot 5 → explicitly empty

    let state = MatchState::initial_with_content(
        seed,
        &store,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
    )
    .expect("initial_with_content should succeed")
    .with_slot_signatures(override_map);

    // Slots 5 and 7 were overridden to empty.
    assert!(
        state.players[7].signature_candidates().is_empty(),
        "slot 7 should have 0 candidates after override with empty Vec"
    );
    assert!(
        state.players[5].signature_candidates().is_empty(),
        "slot 5 should have 0 candidates after override with empty Vec"
    );

    // Slots 6, 16, 17, 18 were NOT in the map — should retain AM candidates.
    for slot in [6usize, 16, 17, 18] {
        assert!(
            !state.players[slot].signature_candidates().is_empty(),
            "MID slot {slot} was not in override map; should retain role-matched candidates"
        );
    }

    // Non-MID slots (GK/DEF/FWD) remain empty — they were already empty from
    // role-matched spread and no override can add candidates to them here.
    for slot in [0usize, 1, 2, 3, 4, 8, 9, 10, 11, 12, 13, 14, 15, 19, 20, 21] {
        assert!(
            state.players[slot].signature_candidates().is_empty(),
            "Non-MID slot {slot} should remain empty (no role-matched template)"
        );
    }
}

// ---------------------------------------------------------------------------
// RED test 6: initial_with_content + tick_match advances normally
// ---------------------------------------------------------------------------

#[test]
fn initial_with_content_then_tick_match_advances() {
    let store = load_store();
    let seed = Seed::from_u64(42);
    let empty_sigs: BTreeMap<String, SignatureDefinition> = BTreeMap::new();

    let state = MatchState::initial_with_content(
        seed,
        &store,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
    )
    .expect("initial_with_content failed");
    let after = tick_match(state, &empty_sigs);
    assert_eq!(
        after.tick,
        fw_core::Tick::ZERO.successor(),
        "tick_match after initial_with_content should advance tick by 1"
    );
}

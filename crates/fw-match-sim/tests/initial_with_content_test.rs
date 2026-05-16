//! Integration tests for `MatchState::initial_with_content` — T1-11 chunk 2.
//!
//! These tests verify:
//! 1. Slot 7 (home AM) gets `signature_candidates` from sample-am.ron.
//! 2. All other 21 slots still have empty candidates (T1-7 fills the rest).
//! 3. Same-seed → same canonical bytes (determinism preserved).
//! 4. Missing `sample-am` template → Err (fail-loud).
//! 5. initial_with_content + tick_match still advances normally.

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
// RED test 2: all other slots have empty signature_candidates
// ---------------------------------------------------------------------------

#[test]
fn slots_except_7_have_empty_candidates_after_initial_with_content() {
    let store = load_store();
    let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);

    let state = MatchState::initial_with_content(
        seed,
        &store,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
    )
    .expect("initial_with_content should succeed");

    for (i, player) in state.players.iter().enumerate() {
        if i == 7 {
            continue; // slot 7 is the smoke-anchor — covered by test above
        }
        assert_eq!(
            player.signature_candidates().len(),
            0,
            "slot {} should have 0 signature_candidates (T1-7 procgen fills the rest); got {}",
            i,
            player.signature_candidates().len()
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
// RED test 5: missing sample-am template → Err
// ---------------------------------------------------------------------------

#[test]
fn initial_with_content_fails_when_sample_am_missing() {
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
        "initial_with_content with empty ContentStore should return Err (no sample-am template)"
    );
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

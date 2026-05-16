//! Smoke test: T1-11 chunk 4.
//!
//! Asserts that running `initial_with_content` (slot 7 = AM with 3 candidates)
//! for 600 ticks with real `SignatureDefinition` objects from the content corpus
//! produces at least one `MatchEvent::SignatureFirstFired` for player_slot 7,
//! and that every fired `signature_id` is in the sample-am candidate list.
//!
//! This is the non-vacuous wiring gate: it proves the full path from
//! `ContentStore → initial_with_content → tick_match(sig_definitions)` emits
//! real signature events, not just compiles.

use std::collections::BTreeSet;
use std::path::PathBuf;

use fw_content::{ContentStore, MatchEvent};
use fw_core::Seed;
use fw_match_sim::{MatchState, tick_match};

fn content_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

fn load_store() -> ContentStore {
    ContentStore::load_sources(&content_root()).expect("ContentStore::load_sources failed")
}

#[test]
fn slot_7_fires_at_least_one_signature_in_600_ticks() {
    let store = load_store();
    let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);

    let mut state = MatchState::initial_with_content(
        seed,
        &store,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
    )
    .expect("initial_with_content should succeed");

    // Collect the candidate signature IDs for slot 7 from the loaded state.
    let candidate_ids: BTreeSet<String> = state.players[7]
        .signature_candidates()
        .iter()
        .map(|c| c.signature_id.as_str().to_owned())
        .collect();

    assert!(
        !candidate_ids.is_empty(),
        "slot 7 should have candidate signatures after initial_with_content"
    );

    // Run 600 ticks collecting all SignatureFirstFired events for slot 7.
    //
    // **Per-tick diff scan (Codex T1-11 code-reviewer P1 fix-pass)**: prior
    // implementation scanned the entire match_events Vec on every tick,
    // re-finding every event from tick T on ticks T+1..600 → `slot7_firings`
    // accumulated each fired ID repeated hundreds of times. The `!is_empty()`
    // assertion still passed correctly (one real firing is enough), but the
    // accumulator's count + the per-ID iteration were silently measuring the
    // wrong thing. New implementation tracks the previous match_events Vec
    // length and slices from there — only NEW events appended this tick are
    // scanned. `slot7_firings` now contains at most 3 entries (one per
    // sample-am candidate) since `signature_first_fired_seen` enforces the
    // first-fire-per-(slot,sig_id)-pair invariant in dispatch.rs.
    let mut slot7_firings: Vec<String> = Vec::new();
    let mut prev_event_count = state.match_events().len();
    for _ in 0..600 {
        state = tick_match(state, &store.signature_definitions);
        let events = state.match_events();
        for ev in &events[prev_event_count..] {
            if let MatchEvent::SignatureFirstFired {
                player_slot,
                signature_id,
                ..
            } = ev
                && *player_slot as usize == 7
            {
                slot7_firings.push(signature_id.as_str().to_owned());
            }
        }
        prev_event_count = events.len();
    }

    // Non-vacuous gate: at least one firing must have occurred.
    assert!(
        !slot7_firings.is_empty(),
        "expected ≥1 SignatureFirstFired for slot 7 in 600 ticks; got 0. \
         Check that sig_definitions is non-empty and slot 7 has candidates with \
         trigger bindings. candidate_ids={candidate_ids:?}"
    );

    // All fired IDs must be from the sample-am candidate list.
    for fired_id in &slot7_firings {
        assert!(
            candidate_ids.contains(fired_id.as_str()),
            "slot 7 fired signature {fired_id:?} which is NOT in candidate list {candidate_ids:?}"
        );
    }
}

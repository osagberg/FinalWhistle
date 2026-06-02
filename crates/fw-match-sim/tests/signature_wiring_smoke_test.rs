//! Smoke tests: T1-11 chunk 4 + T4-2.5c non-slot-7 wiring.
//!
//! Test 1 (slot_7_fires_at_least_one_signature_in_600_ticks): asserts that
//! running `initial_with_content` (role-matched MID slots carry AM candidates
//! since T4-2.5c) for 600 ticks with real `SignatureDefinition` objects from the
//! content corpus produces at least one `MatchEvent::SignatureFirstFired` for
//! player_slot 7.
//!
//! Test 2 (non_slot_7_fires_signature_first_fired): asserts that at least one
//! role-matched slot OTHER than slot 7 (i.e. one of the other 5 MID slots:
//! home 5,6 or away 16,17,18) fires a `SignatureFirstFired` event in 600 ticks.
//! This proves the role-matched spread flows through dispatch_tick for
//! non-slot-7 MID players (the T4-2.5c AC-2 falsifiable gate).
//!
//! These are the non-vacuous wiring gates: they prove the full path from
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

// ---------------------------------------------------------------------------
// T4-2.5c AC-2: a non-slot-7 role-matched slot fires SignatureFirstFired
//
// After the role-matched spread, MID slots 5,6,7 (home) and 16,17,18 (away)
// carry AM candidates. This test proves dispatch_tick routes signature firings
// for the 5 OTHER role-matched slots besides slot 7.
//
// Eligible non-slot-7 slots for long-range-strike trigger:
//   home:  5,6 (in_team 5,6 ∈ 5..=10 ✓)
//   away: 16,17,18 (in_team = slot%11 = 5,6,7 ∈ 5..=10 ✓)
// Eligible for first-time-diagonal-switch (in_team ∈ 5..=7):
//   home: 5,6 (in_team 5,6)
//   away: 16,17,18 (in_team 5,6,7)
// ---------------------------------------------------------------------------

#[test]
fn non_slot_7_role_matched_slot_fires_signature_first_fired() {
    let store = load_store();
    let seed = Seed::from_u64(0xFEED_BEEF_CAFE_FADE);

    let mut state = MatchState::initial_with_content(
        seed,
        &store,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
    )
    .expect("initial_with_content should succeed with role-matched spread");

    // Verify: only role-matched MID slots carry candidates (T4-2.5c AC-1 refined).
    let mid_slots: &[usize] = &[5, 6, 7, 16, 17, 18];
    for &slot in mid_slots {
        assert!(
            !state.players[slot].signature_candidates().is_empty(),
            "MID slot {slot} should have non-empty candidates after role-matched spread"
        );
    }
    let non_mid_slots: &[usize] = &[0, 1, 2, 3, 4, 8, 9, 10, 11, 12, 13, 14, 15, 19, 20, 21];
    for &slot in non_mid_slots {
        assert!(
            state.players[slot].signature_candidates().is_empty(),
            "Non-MID slot {slot} should have empty candidates after role-matched spread"
        );
    }

    // Run 600 ticks; collect (slot, sig_id) for all SignatureFirstFired events
    // from role-matched slots OTHER than slot 7.
    let mut non_slot7_firings: Vec<(u8, String)> = Vec::new();
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
                && *player_slot as usize != 7
            {
                non_slot7_firings.push((*player_slot, signature_id.as_str().to_owned()));
            }
        }
        prev_event_count = events.len();
    }

    // Non-vacuous gate: at least one firing from a non-slot-7 MID slot.
    // With 5 additional MID slots carrying AM candidates, at least one of
    // {5,6,16,17,18} should fire within 600 ticks on this seed.
    assert!(
        !non_slot7_firings.is_empty(),
        "expected ≥1 SignatureFirstFired for a role-matched slot ≠ 7 in 600 ticks; got 0. \
         Seed: 0xfeedbeefcafefade. Check that MID slots 5,6,16,17,18 carry candidates \
         AND their trigger predicates fire. Role-matched slots seen: \
         {mid_slots:?}. signature_first_fired_seen slots: {:?}",
        state
            .signature_first_fired_seen
            .iter()
            .filter(|(slot, _)| *slot != 7)
            .map(|(s, _)| s)
            .collect::<Vec<_>>()
    );

    // Confirm the firing came from a role-matched MID slot (not an unexpected slot).
    let first = &non_slot7_firings[0];
    assert!(
        mid_slots.contains(&(first.0 as usize)),
        "non-slot-7 firing came from slot {} which is NOT a role-matched MID slot {:?}",
        first.0,
        mid_slots
    );
}

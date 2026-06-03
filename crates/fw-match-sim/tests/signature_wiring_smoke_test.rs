//! Smoke tests: T1-11 chunk 4 + T4-2.5c non-slot-7 wiring + T4-2.5j all-role wiring.
//!
//! Test 1 (slot_7_fires_at_least_one_signature_in_600_ticks): asserts that
//! running `initial_with_content` (role-matched MID slots carry AM candidates
//! since T4-2.5c) for 600 ticks with real `SignatureDefinition` objects from the
//! content corpus produces at least one `MatchEvent::SignatureFirstFired` for
//! player_slot 7.
//!
//! Test 2 (non_slot_7_fires_signature_first_fired): asserts that at least one
//! role-matched slot OTHER than slot 7 fires a `SignatureFirstFired` event in
//! 600 ticks. T4-2.5j expanded this from MID-only to all 22 slots — GK/DEF/FWD
//! templates now exist so non-MID slots also carry candidates.
//!
//! These are the non-vacuous wiring gates: they prove the full path from
//! `ContentStore → initial_with_content → tick_match(sig_definitions)` emits
//! real signature events, not just compiles.

use std::collections::BTreeSet;
use std::path::PathBuf;

use fw_content::{ContentStore, MatchEvent, RoleFamily};
use fw_core::{Q32, Seed};
use fw_match_sim::signature::build_trigger_table;
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
// T4-2.5j AC-2: a non-slot-7 slot fires SignatureFirstFired
//
// After T4-2.5j's role-matched spread, ALL 22 slots carry candidates:
//   MID (5-7, 16-18):  AM template — diagonal-switch, long-range-strike, etc.
//   GK  (0, 11):       GK template — commanding-claim
//   DEF (1-4, 12-15):  DEF template — overlapping-surge
//   FWD (8-10, 19-21): FWD template — touchline-beat, poachers-dart
//
// This test proves that at least one slot OTHER than slot 7 fires a signature
// within 600 ticks, demonstrating the multi-role spread flows end-to-end.
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

    // T4-2.5j: All 22 slots should have candidates (MID from AM template,
    // GK/DEF/FWD from new role templates).
    for slot in 0..22usize {
        assert!(
            !state.players[slot].signature_candidates().is_empty(),
            "Slot {slot} should have non-empty candidates after T4-2.5j role-matched spread"
        );
    }

    // Run 600 ticks; collect (slot, sig_id) for all SignatureFirstFired events
    // from slots OTHER than slot 7.
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

    // Non-vacuous gate: at least one firing from a slot ≠ 7.
    // With all 22 slots carrying candidates, at least one of the other 21
    // slots should fire within 600 ticks on this seed.
    assert!(
        !non_slot7_firings.is_empty(),
        "expected ≥1 SignatureFirstFired for a slot ≠ 7 in 600 ticks; got 0. \
         Seed: 0xfeedbeefcafefade. Check that all slots carry candidates \
         AND at least one non-slot-7 trigger predicate fires. \
         signature_first_fired_seen slots: {:?}",
        state
            .signature_first_fired_seen
            .iter()
            .map(|(s, _)| s)
            .collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// T4-2.5j self-review (type-design guard): each IMPLEMENTED signature's RON
// `role_family` must agree with its predicate's code slot-gate.
//
// The per-predicate unit tests in `triggers.rs` hardcode the slot they probe
// (the same `in_team` literal the predicate gates on), so they CANNOT catch a
// RON `role_family` that disagrees with the gate, nor a gate-literal typo that
// drifts from the declared family — both would just make the signature
// silently never fire. This test closes that gap: for every signature whose
// id has a trigger binding (the implemented ones; the no-op stub is excluded),
// it maps the RON `role_family` to that family's canonical home-XI slot (per
// `design/signatures.md`) and asserts the predicate actually fires there.
//
// `MatchState::initial` builds every player with `mid_range_baseline()` (all
// attributes 0.5), which clears every signature's 0.45 attribute threshold, so
// a non-fire at the canonical slot can ONLY mean the role gate rejects that
// slot — i.e. the declared family and the code gate have diverged.
#[test]
fn implemented_signature_role_family_agrees_with_predicate_gate() {
    /// Canonical home-XI slot for each role family (4-3-3; see design/signatures.md).
    fn canonical_slot(rf: RoleFamily) -> u8 {
        match rf {
            RoleFamily::Goalkeeper => 0,
            RoleFamily::FullBack => 1,
            RoleFamily::CentreBack => 2,
            RoleFamily::DefensiveMidfielder => 5,
            RoleFamily::CentralMidfielder => 6,
            RoleFamily::AttackingMidfielder => 7,
            RoleFamily::Winger => 8,
            RoleFamily::Striker => 9,
        }
    }

    let store = load_store();
    let table = build_trigger_table();
    // All 22 players carry mid_range_baseline (0.5) attributes → above every
    // 0.45 threshold, so any non-fire is a role-gate rejection, not a low stat.
    let state = MatchState::initial(Seed::from_u64(1));

    let mut checked = 0u32;
    for (id, def) in &store.signature_definitions {
        // Only implemented signatures have a predicate binding.
        let Some(trigger) = table.get(id.as_str()) else {
            continue;
        };
        // The no-op stub is intentionally never-fire; exclude it.
        if id.as_str() == "fwh.core:signature.no-op-stub" {
            continue;
        }

        let slot = canonical_slot(def.role_family);
        let fit = (*trigger)(&state, slot);
        assert!(
            fit > Q32::ZERO,
            "signature {id} declares role_family {:?} (canonical slot {slot}) but its \
             trigger returns ZERO there with a maxed-baseline player — the RON role_family \
             and the predicate's slot-gate have diverged (or the gate excludes the family's \
             canonical slot)",
            def.role_family
        );
        checked += 1;
    }

    // Guard against the test silently checking nothing (e.g. an empty store or
    // a build_trigger_table that lost its bindings).
    assert!(
        checked >= 8,
        "expected ≥8 implemented signatures cross-checked (1 per role family); checked {checked}"
    );
}

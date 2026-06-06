//! Smoke tests: T1-11 chunk 4 + T4-2.5c non-slot-7 wiring + T4-2.5j all-role
//! wiring + the 2026-06-06 signature kickoff-spam fix.
//!
//! Test 1 (signatures_fire_in_a_full_match_and_are_not_a_kickoff_dump): runs a
//! full 90-minute content match and asserts signatures DO fire (the
//! `ContentStore → initial_with_content → tick_match(sig_definitions)` path
//! works) AND that they are SPREAD across the match rather than dumped at
//! kick-off. This is the regression gate for the kickoff-spam fix: pre-fix,
//! every eligible player fired the instant they first decided (100% in minute
//! 0-1); post-fix, signatures fire on a genuine in-play execution moment.
//!
//! Test 2 (non_slot_7_fires_signature_first_fired): asserts that at least one
//! role-matched slot OTHER than slot 7 fires a `SignatureFirstFired` event in
//! a 600-tick window. T4-2.5j expanded this from MID-only to all 22 slots —
//! GK/DEF/FWD templates now exist so non-MID slots also carry candidates.
//!
//! These are the non-vacuous wiring gates: they prove the full path from
//! `ContentStore → initial_with_content → tick_match(sig_definitions)` emits
//! real signature events, not just compiles.

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
fn signatures_fire_in_a_full_match_and_are_not_a_kickoff_dump() {
    // 2026-06-06 kickoff-spam fix: signatures now fire on a genuine in-play
    // EXECUTION moment (real shot / dribble / run / interception), gated by a
    // 60-tick kick-off settle window + the per-signature action gate — NOT the
    // instant a static attribute precondition is first true. This is the
    // regression gate for that fix: across a full 90-minute content match,
    // signatures DO fire (wiring still works) and they are SPREAD across the
    // match instead of dumped at kick-off.
    //
    // Pre-fix behaviour (the bug): every eligible player fired the instant they
    // first decided — all 26 firings landed in minute 0-1, then zero for the
    // remaining 89 minutes. This test fails loudly if that regresses.
    let store = load_store();
    let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);

    let mut state = MatchState::initial_with_content(
        seed,
        &store,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
    )
    .expect("initial_with_content should succeed");

    // Full 90-minute match (5400 ticks at 60 Hz).
    for _ in 0..5400u32 {
        state = tick_match(state, &store.signature_definitions);
    }

    // Collect the tick of every SignatureFirstFired event.
    let firing_ticks: Vec<i64> = state
        .match_events()
        .iter()
        .filter_map(|ev| match ev {
            MatchEvent::SignatureFirstFired { tick, .. } => Some(tick.to_raw()),
            _ => None,
        })
        .collect();
    let total = firing_ticks.len();

    // Wiring gate: signatures DO fire in a full match (the path works).
    assert!(
        total > 0,
        "expected ≥1 SignatureFirstFired across a full match; got 0. \
         The content → dispatch → action-gate firing path is broken."
    );

    // Settle-window gate: NOTHING fires inside the kick-off settle window.
    let in_settle = firing_ticks
        .iter()
        .filter(|t| **t < fw_match_sim::signature::SIGNATURE_SETTLE_TICKS)
        .count();
    assert_eq!(
        in_settle,
        0,
        "no signature may fire inside the {}-tick kick-off settle window; \
         {in_settle} of {total} did (ticks: {firing_ticks:?})",
        fw_match_sim::signature::SIGNATURE_SETTLE_TICKS
    );

    // Anti-kickoff-dump gate: firings must NOT all cluster in the opening
    // minutes. The bug had 100% in minute 0-1; require that strictly more than
    // half the firings land after the first 2 minutes (tick 7200/60 ... here
    // 2 min = 7200 ticks would be too coarse for a low count, so use: at least
    // one firing lands past the first 5 minutes (tick 18000? no — 5 min = 18000
    // is >5400). Use the match-relative band: more than half of the firings
    // must be at or beyond minute 2 (tick 7200 is past full match; use tick
    // 1800 = minute 30 as the "early" boundary is too strict for a low count).
    //
    // The robust, count-insensitive invariant: the firings must SPAN the match,
    // i.e. the LAST firing must be well past the opening minutes. A kickoff
    // dump has every firing in minute 0-1 (tick < 120), so the last firing tick
    // would be < 120. Require the last firing to land past minute 5 (tick 300),
    // which is impossible under the old precondition-dump behaviour.
    let last_tick = *firing_ticks.iter().max().expect("total > 0 checked above");
    let min_per_tick = 60;
    assert!(
        last_tick >= 300,
        "signatures look like a kick-off dump: the LAST of {total} firings was at \
         tick {last_tick} (minute {}), inside the opening minutes. Firings must \
         spread across the match. ticks: {firing_ticks:?}",
        last_tick / min_per_tick
    );

    // Stronger spread check: not every firing is in the opening 2 minutes.
    let early_share = firing_ticks.iter().filter(|t| **t < 120).count();
    assert!(
        early_share < total,
        "all {total} firings landed in minute 0-1 (the kickoff-dump bug). \
         ticks: {firing_ticks:?}"
    );
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

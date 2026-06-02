//! Proptest invariants for the T1-4a `MatchEvent` emission (T1-4a chunk 6).
//!
//! Two invariants:
//! 1. `events_chronological`: `match_events[i+1].tick() >= match_events[i].tick()`
//!    across 100 random seeds × 60 ticks (monotonic non-decreasing).
//! 2. `determinism_across_runs`: same seed run twice → byte-identical
//!    `match_events` Vec (no hidden non-determinism).
//!
//! ## Anti-vacuousness (T1-4a TDD mandate)
//!
//! Per the round-3 cargo-cult-fix-pass lesson: assert positive emission FIRST
//! (at least KickOff + FullTime must be in the Vec), THEN assert the invariant.
//! A test that only checks `is_sorted` on an empty Vec is vacuously satisfied
//! even by a broken impl that emits zero events.

use fw_core::{Seed, Tick};
use fw_match_sim::{MatchEvent, MatchState, tick_match};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Tick extraction helper
// ---------------------------------------------------------------------------

/// Extract the tick from any `MatchEvent` variant. Needed to assert
/// chronological ordering without matching every variant explicitly.
fn event_tick(ev: &MatchEvent) -> Tick {
    match ev {
        MatchEvent::KickOff { tick, .. } => *tick,
        MatchEvent::FullTime { tick, .. } => *tick,
        MatchEvent::Goal { tick, .. } => *tick,
        MatchEvent::Shot { tick, .. } => *tick,
        MatchEvent::Pass { tick, .. } => *tick,
        MatchEvent::SignatureFirstFired { tick, .. } => *tick,
    }
}

/// Run the smoke sim for `tick_count` ticks with the given seed.
///
/// T4-sim-halt: sets `match_end_tick = tick_count` so FullTime fires exactly
/// at the end of the run. The default is now 5400; without this override the
/// short 60-tick budget would produce no FullTime event.
fn run_match(seed_u64: u64, tick_count: u32) -> MatchState {
    let seed = Seed::from_u64(seed_u64);
    let mut state =
        MatchState::initial(seed).with_match_end_tick(Tick::from_raw(tick_count as i64));
    for _ in 0..tick_count {
        state = tick_match(state, &std::collections::BTreeMap::new());
    }
    state
}

// ---------------------------------------------------------------------------
// Invariant 1: events_chronological
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn events_chronological(seed_u64 in 0u64..u64::MAX) {
        let state = run_match(seed_u64, 60);

        // Anti-vacuousness: at least KickOff (tick 0) + FullTime (tick 60) must
        // be present. If this fires, the emission path is broken.
        //
        // Codex Tier-2 silent-failure P3 on T1-4a 2026-05-16: deleted the prior
        // `prop_assume!(!is_empty())` here because KickOff is unconditionally
        // pushed in MatchState::initial — the prop_assume could never trigger,
        // and if a future PR broke KickOff emission the proptest would silently
        // pass via the assume-reject path. The assert! below is the actual gate.
        assert!(
            state.match_events().len() >= 2,
            "match_events must have at least KickOff + FullTime (got {}); \
             emission is broken",
            state.match_events().len()
        );
        assert!(
            matches!(state.match_events()[0], MatchEvent::KickOff { .. }),
            "first event must be KickOff; got {:?}",
            state.match_events()[0]
        );
        assert!(
            matches!(state.match_events().last().unwrap(), MatchEvent::FullTime { .. }),
            "last event must be FullTime; got {:?}",
            state.match_events().last()
        );

        // Monotonic non-decreasing tick invariant.
        for window in state.match_events().windows(2) {
            let t_prev = event_tick(&window[0]);
            let t_next = event_tick(&window[1]);
            assert!(
                t_next >= t_prev,
                "event ordering violated: {:?} (tick {:?}) followed by {:?} (tick {:?})",
                window[0], t_prev, window[1], t_next
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Invariant 2: determinism_across_runs
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn determinism_across_runs(seed_u64 in 0u64..u64::MAX) {
        let state1 = run_match(seed_u64, 60);
        let state2 = run_match(seed_u64, 60);

        // Anti-vacuousness: both runs must have emitted at least KickOff + FullTime.
        assert!(
            state1.match_events().len() >= 2,
            "first run: match_events must have at least KickOff + FullTime"
        );
        assert!(
            state2.match_events().len() >= 2,
            "second run: match_events must have at least KickOff + FullTime"
        );

        // Byte-identical match_events via canonical encoding comparison.
        // encode_canonical() encodes the full MatchState including match_events;
        // if match_events differ, the canonical bytes will differ.
        assert_eq!(
            state1.encode_canonical(),
            state2.encode_canonical(),
            "two runs of seed {seed_u64:#018x} produced different canonical bytes; \
             hidden non-determinism in match_events or canonical state"
        );

        // Also compare directly for a more targeted failure message.
        assert_eq!(
            state1.match_events().len(),
            state2.match_events().len(),
            "event count mismatch across runs"
        );
        for (i, (e1, e2)) in state1.match_events().iter().zip(state2.match_events().iter()).enumerate() {
            assert_eq!(
                e1, e2,
                "match_events[{i}] differs across runs for seed {seed_u64:#018x}"
            );
        }
    }
}

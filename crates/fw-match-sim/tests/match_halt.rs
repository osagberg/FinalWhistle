//! T4-sim-halt: `tick_match` halts at `match_end_tick` + freezes.
//!
//! Three acceptance tests:
//!
//! 1. `match_halts_at_full_time_and_freezes` — advance 30 ticks past a
//!    5-tick match_end_tick; assert exactly ONE FullTime, freeze invariant,
//!    and correct tail.
//!
//! 2. `default_match_end_tick_is_full_match_ticks` — the bare
//!    `MatchState::initial` carries `match_end_tick == FULL_MATCH_TICKS`.
//!
//! 3. `full_time_emits_at_configured_end_tick` — a short-budget match emits
//!    FullTime exactly at the configured end_tick, not before, not after.
//!
//! ## Mutation resistance notes
//!
//! - Removing the freeze guard causes `full_time_count > 1` in test 1.
//! - Removing the in-play gate lets score change after FullTime.
//! - Changing `FULL_MATCH_TICKS` breaks test 2.

use fw_core::{Seed, Tick};
use fw_match_sim::{FULL_MATCH_TICKS, MatchEvent, MatchState, tick_match};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Test 1: halt + freeze
// ---------------------------------------------------------------------------

/// Advance 30 ticks past a 5-tick match.
///
/// Expected:
///   - Exactly ONE `FullTime` in the event stream.
///   - `encode_canonical()` is byte-identical before vs after one MORE call.
///   - `match_events().last()` is `FullTime`.
#[test]
fn match_halts_at_full_time_and_freezes() {
    let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);
    let mut state = MatchState::initial(seed).with_match_end_tick(Tick::from_raw(5));

    // Advance 30 ticks — well past the 5-tick end.
    for _ in 0..30 {
        state = tick_match(state, &BTreeMap::new());
    }

    // Exactly ONE FullTime event in the entire stream.
    let full_time_count = state
        .match_events()
        .iter()
        .filter(|e| matches!(e, MatchEvent::FullTime { .. }))
        .count();
    assert_eq!(
        full_time_count, 1,
        "expected exactly 1 FullTime event after 30 ticks on a 5-tick match; \
         got {full_time_count}. If > 1, the freeze guard is missing or broken."
    );

    // FullTime is the tail.
    assert!(
        matches!(
            state.match_events().last(),
            Some(MatchEvent::FullTime { .. })
        ),
        "last event must be FullTime; got {:?}",
        state.match_events().last()
    );

    // Freeze invariant: one more tick_match call produces byte-identical state.
    let bytes_before = state.encode_canonical();
    let state_after = tick_match(state, &BTreeMap::new());
    let bytes_after = state_after.encode_canonical();
    assert_eq!(
        bytes_before, bytes_after,
        "tick_match after FullTime must be a no-op (freeze); canonical bytes changed"
    );
}

// ---------------------------------------------------------------------------
// Test 2: default match_end_tick == FULL_MATCH_TICKS
// ---------------------------------------------------------------------------

/// The bare `MatchState::initial` must carry the real 90-minute default.
#[test]
fn default_match_end_tick_is_full_match_ticks() {
    // Pin the NUMERIC value (90 min x 60 ticks/min = 5400) as a literal, not via
    // FULL_MATCH_TICKS — otherwise a mutation that sets both the const and the
    // default to a wrong value (e.g. back to the old 60) would pass vacuously.
    assert_eq!(
        FULL_MATCH_TICKS, 5400,
        "FULL_MATCH_TICKS must be 5400 (90 min x 60 ticks/min)"
    );
    let state = MatchState::initial(Seed::from_u64(42));
    assert_eq!(
        state.match_end_tick(),
        Tick::from_raw(5400),
        "MatchState::initial must default to 5400 ticks (90 displayed-min); \
         FULL_MATCH_TICKS={FULL_MATCH_TICKS}"
    );
}

// ---------------------------------------------------------------------------
// Test 3: FullTime emits exactly at the configured end_tick
// ---------------------------------------------------------------------------

/// A 10-tick match must emit FullTime at tick 10, not before, not after.
///
/// Also verifies that no FullTime appears before tick 10 (the in-play gate
/// must not prematurely emit FullTime mid-match).
#[test]
fn full_time_emits_at_configured_end_tick() {
    let end_tick = Tick::from_raw(10);
    let seed = Seed::from_u64(0xCAFE_BABE);
    let mut state = MatchState::initial(seed).with_match_end_tick(end_tick);

    // Advance tick by tick; check no FullTime before end_tick.
    for i in 1..=9u32 {
        state = tick_match(state, &BTreeMap::new());
        assert!(
            !state
                .match_events()
                .iter()
                .any(|e| matches!(e, MatchEvent::FullTime { .. })),
            "FullTime must not appear before match_end_tick=10; appeared at tick {i}"
        );
    }

    // Tick 10 — FullTime fires.
    state = tick_match(state, &BTreeMap::new());
    assert_eq!(
        state.tick, end_tick,
        "tick must equal end_tick after 10 advances"
    );

    let full_time_count = state
        .match_events()
        .iter()
        .filter(|e| matches!(e, MatchEvent::FullTime { .. }))
        .count();
    assert_eq!(
        full_time_count, 1,
        "exactly 1 FullTime must appear at the end tick; got {full_time_count}"
    );

    // Confirm the FullTime tick is correct.
    if let Some(MatchEvent::FullTime { tick, .. }) = state.match_events().last() {
        assert_eq!(
            *tick, end_tick,
            "FullTime.tick must equal match_end_tick; got {:?}",
            tick
        );
    } else {
        panic!("last event is not FullTime");
    }
}

//! Insta snapshot test of `MatchState.match_events` after 60 ticks (T1-4a).
//!
//! This snapshot pins the in-match event stream from the standard smoke seed
//! (0xDEAD_BEEF_DEAD_BEEF). It complements `canonical_hash.rs`'s BLAKE3 pin:
//! the hash catches any drift in the byte stream; this snapshot provides a
//! human-diffable record of which events fired and in what order.
//!
//! ## Anti-vacuousness
//!
//! Per the T1-4a TDD mandate and the round-3 cargo-cult-fix-pass lesson:
//! we assert positive emission FIRST (at least KickOff must appear), THEN
//! take the snapshot. A test that only snapshots an empty Vec would pass
//! on a broken impl that emits no events.

use fw_core::Seed;
use fw_match_sim::{MatchEvent, MatchState, tick_match};

const SMOKE_SEED: u64 = 0xDEAD_BEEF_DEAD_BEEF;
const SMOKE_TICK_COUNT: u32 = 60;

#[test]
fn smoke_seed_60_tick_match_events_snapshot() {
    let seed = Seed::from_u64(SMOKE_SEED);
    // T4-sim-halt: default match_end_tick is now 5400 (real 90 min). Override
    // to 60 so this short-budget snapshot test still fires FullTime at tick 60.
    let mut state = MatchState::initial(seed)
        .with_match_end_tick(fw_core::Tick::from_raw(SMOKE_TICK_COUNT as i64));
    for _ in 0..SMOKE_TICK_COUNT {
        state = tick_match(state, &std::collections::BTreeMap::new());
    }

    // Anti-vacuousness guard: at least KickOff must be present. If this
    // assertion fails, the emission path is broken — the snapshot below
    // would be of an empty (or KickOff-absent) Vec, which is trivially
    // satisfied even by a broken impl.
    assert!(
        !state.match_events().is_empty(),
        "match_events must have at least 1 event (KickOff) after 60 ticks"
    );
    assert!(
        matches!(state.match_events()[0], MatchEvent::KickOff { .. }),
        "first event must be KickOff; got {:?}",
        state.match_events()[0]
    );

    // FullTime must be the last event (emitted at tick 60 = match_end_tick).
    let last = state.match_events().last().expect("at least one event");
    assert!(
        matches!(last, MatchEvent::FullTime { .. }),
        "last event must be FullTime at tick 60; got {:?}",
        last
    );

    // Human-diffable snapshot of the full event stream.
    // If this snapshot drifts, inspect whether the change was intentional
    // (new behavior producing different events) or a regression.
    insta::assert_debug_snapshot!("smoke_seed_60_tick_match_events", state.match_events());
}

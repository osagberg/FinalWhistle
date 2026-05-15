//! Proptest invariants for the T1-2b-iii-a `dispatch_tick` function.
//!
//! Covers acceptance criteria:
//! - AC 8: dispatch_tick determinism over 100 random seeds.
//! - AC 5: `decision_counter()` increments deterministically.

use fw_core::Seed;
use fw_match_sim::{MatchState, dispatch, tick_match};
use proptest::prelude::*;
use std::collections::BTreeMap;

fn arb_seed() -> impl Strategy<Value = u64> {
    any::<u64>()
}

// ---------------------------------------------------------------------------
// dispatch_tick is a pure function
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn dispatch_tick_is_deterministic_over_100_seeds(seed_val in arb_seed()) {
        // Two calls to dispatch_tick from the same initial state must produce
        // byte-identical canonical output.
        let seed = Seed::from_u64(seed_val);
        let s1 = MatchState::initial(seed);
        let s2 = MatchState::initial(seed);

        let empty_defs = BTreeMap::new();
        let r1 = dispatch::dispatch_tick(s1, &empty_defs);
        let r2 = dispatch::dispatch_tick(s2, &empty_defs);

        prop_assert_eq!(
            r1.encode_canonical(),
            r2.encode_canonical(),
            "dispatch_tick must be deterministic for seed 0x{:016x}", seed_val
        );
    }
}

// ---------------------------------------------------------------------------
// Counter only increments -- never decrements
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn decision_counter_never_decrements(seed_val in arb_seed()) {
        let seed = Seed::from_u64(seed_val);
        let mut state = MatchState::initial(seed);
        let mut prev_counters: Vec<u32> = state.players.iter().map(|p| p.decision_counter()).collect();

        for _ in 0..30 {
            state = tick_match(state);
            for (i, p) in state.players.iter().enumerate() {
                prop_assert!(
                    p.decision_counter() >= prev_counters[i],
                    "player slot {}: counter went backwards ({} to {})",
                    i, prev_counters[i], p.decision_counter()
                );
                prev_counters[i] = p.decision_counter();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// All players decide at least once per 15-tick window
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn every_player_decides_at_least_once_per_15_ticks(seed_val in arb_seed()) {
        let seed = Seed::from_u64(seed_val);
        let mut state = MatchState::initial(seed);

        for _ in 0..15 {
            state = tick_match(state);
        }

        // Balanced multiset guarantees every slot has at least one entry.
        // So after 15 ticks (one full window), every player should have
        // decision_counter() >= 1.
        for (i, p) in state.players.iter().enumerate() {
            prop_assert!(
                p.decision_counter() >= 1,
                "player slot {} had 0 decisions in 15 ticks; balanced slot template guarantees 1+",
                i
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Canonical output of tick_match is stable across invocations
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn tick_match_canonical_output_is_stable(seed_val in arb_seed()) {
        let seed = Seed::from_u64(seed_val);

        let run_60 = || {
            let mut s = MatchState::initial(seed);
            for _ in 0..60 {
                s = tick_match(s);
            }
            s.encode_canonical()
        };

        let a = run_60();
        let b = run_60();

        prop_assert_eq!(
            a, b,
            "tick_match must produce stable canonical output for seed 0x{:016x}", seed_val
        );
    }
}

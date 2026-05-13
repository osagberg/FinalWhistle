//! Proptest invariants for the T1-2b-ii decision-cadence stagger.
//!
//! Per `docs/specs/decision-cadence-stagger.md` T1-2b-ii acceptance:
//! 1. **Slot assignment determinism** — same seed → same `[u8; 22]`.
//! 2. **Balanced-multiset invariant** — any seed: 7 slots appear twice,
//!    8 appear once; no slot empty, no slot ≥ 3. Structural, not statistical.
//! 3. **`decision_slots` immutability** — byte-identical before and after
//!    a 600-tick run (reactive interrupts use the parallel cooldown field).
//! 4. **Per-tick decision count in {1, 2}** — structural consequence of
//!    balanced template + Fisher-Yates; true for any tick and any seed.

use fw_core::{Seed, Tick};
use fw_match_sim::decision_cadence::SeedLayer;
use fw_match_sim::{MatchState, assign_decision_slots, should_decide, tick_match};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

fn arb_seed() -> impl Strategy<Value = u64> {
    any::<u64>()
}

// ---------------------------------------------------------------------------
// Invariants
// ---------------------------------------------------------------------------

proptest! {
    /// Invariant 1: slot assignment is deterministic.
    /// Same seed → same `[u8; 22]` slot array on every call.
    #[test]
    fn slot_assignment_is_deterministic(seed_raw in arb_seed()) {
        let seed = Seed::from_u64(seed_raw);
        let a = assign_decision_slots(seed);
        let b = assign_decision_slots(seed);
        prop_assert_eq!(a, b);
    }

    /// Invariant 2: balanced-multiset invariant holds for any seed.
    ///
    /// 7 slots (0..=6) appear exactly twice; 8 slots (7..=14) appear exactly
    /// once. Structural — Fisher-Yates is a permutation of SLOT_TEMPLATE.
    #[test]
    fn balanced_multiset_invariant_holds_for_any_seed(seed_raw in arb_seed()) {
        let seed = Seed::from_u64(seed_raw);
        let slots = assign_decision_slots(seed);

        let mut counts = [0u8; 15];
        for &s in &slots {
            prop_assert!(
                (s as usize) < 15,
                "slot value {} is out of range 0..15",
                s
            );
            counts[s as usize] += 1;
        }

        for (i, &c) in counts.iter().enumerate() {
            if i < 7 {
                prop_assert!(
                    c == 2,
                    "slot {} should have count 2, got {}",
                    i,
                    c
                );
            } else {
                prop_assert!(
                    c == 1,
                    "slot {} should have count 1, got {}",
                    i,
                    c
                );
            }
        }
    }

    /// Invariant 3: `decision_slots` is immutable across a 600-tick run.
    ///
    /// tick_match may update `interrupt_cooldown_until` (future T1-2b-iii)
    /// but must NOT touch `decision_slots`.
    #[test]
    fn decision_slots_immutable_across_600_ticks(seed_raw in arb_seed()) {
        let seed = Seed::from_u64(seed_raw);
        let mut state = MatchState::initial(seed);
        let initial_slots = state.decision_slots;

        for _ in 0..600 {
            state = tick_match(state);
        }

        prop_assert_eq!(
            state.decision_slots,
            initial_slots
        );
    }

    /// Invariant 3b (Codex P1 from self-review — closes the cadence spec test
    /// contract #3 that named "fires 100 random interrupts across a 600-tick
    /// run"): even with synthetic interrupts firing on the parallel cooldown
    /// path, `decision_slots` must stay byte-identical from match-init to
    /// end-of-run. The spec promises interrupts use the PARALLEL field; this
    /// proptest is the negative-witness against a future regression that
    /// accidentally mutates `decision_slots` from the interrupt path.
    ///
    /// The test directly mutates `interrupt_cooldown_until` (the canonical
    /// field that the future T1-2b-iii reactive-interrupt code will write).
    /// `decision_slots` must be untouched regardless.
    #[test]
    fn decision_slots_immutable_under_synthetic_interrupts(
        seed_raw in arb_seed(),
        interrupt_pattern in proptest::collection::vec((0u8..22, 0i64..600), 100)
    ) {
        let seed = Seed::from_u64(seed_raw);
        let mut state = MatchState::initial(seed);
        let initial_slots = state.decision_slots;

        // Drive 600 ticks with 100 synthetic interrupts injected at the
        // proptest-chosen (roster_idx, tick) positions.
        for tick_raw in 0..600i64 {
            for &(idx, fire_at_tick) in &interrupt_pattern {
                if fire_at_tick == tick_raw {
                    // Set the cooldown 15 ticks into the future, as the
                    // spec's reactive-interrupt firing rule prescribes.
                    state.interrupt_cooldown_until[idx as usize] =
                        Tick::from_raw(tick_raw + 15);
                }
            }
            state = tick_match(state);
        }

        prop_assert_eq!(
            state.decision_slots,
            initial_slots,
            "decision_slots mutated under synthetic interrupts — \
             interrupt path violated cadence spec test #3"
        );
    }

    /// Invariant 4: per-tick scheduled decision count is in {1, 2}.
    ///
    /// For any seed and any tick in 0..30 (one full stagger window + one
    /// repeat), the count of roster slots with `should_decide(.., tick) == true`
    /// (ignoring cooldowns) is exactly 1 or 2.
    #[test]
    fn per_tick_decision_count_is_one_or_two_for_any_seed(seed_raw in arb_seed()) {
        let seed = Seed::from_u64(seed_raw);
        let slots = assign_decision_slots(seed);
        let cooldowns = [Tick::ZERO; 22];

        for tick_raw in 0..30i64 {
            let tick = Tick::from_raw(tick_raw);
            let count = (1u8..=22)
                .filter(|&r| should_decide(r, &slots, &cooldowns, tick))
                .count();
            prop_assert!(
                count == 1 || count == 2,
                "tick {}: expected 1 or 2 decisions, got {}",
                tick_raw,
                count
            );
        }
    }

    /// Invariant 4b: total decisions per 15-tick window equals 22.
    ///
    /// Each roster slot fires exactly once in a 15-tick window (every slot
    /// in the template appears at least once; Fisher-Yates preserves counts).
    #[test]
    fn total_decisions_per_window_is_22(seed_raw in arb_seed()) {
        let seed = Seed::from_u64(seed_raw);
        let slots = assign_decision_slots(seed);
        let cooldowns = [Tick::ZERO; 22];

        let total: usize = (0..15i64)
            .map(|tick_raw| {
                let tick = Tick::from_raw(tick_raw);
                (1u8..=22)
                    .filter(|&r| should_decide(r, &slots, &cooldowns, tick))
                    .count()
            })
            .sum();

        prop_assert!(
            total == 22,
            "total decisions in 15-tick window = {}, expected 22",
            total
        );
    }

    /// Invariant: seed_fn is deterministic across calls.
    #[test]
    fn seed_fn_is_stable(
        match_seed in arb_seed(),
        tick in 0i64..1000,
        site in any::<u32>()
    ) {
        let a = fw_match_sim::decision_cadence::seed_fn(
            match_seed, tick, SeedLayer::Decision, site as u64
        );
        let b = fw_match_sim::decision_cadence::seed_fn(
            match_seed, tick, SeedLayer::Decision, site as u64
        );
        prop_assert_eq!(a, b);
    }
}

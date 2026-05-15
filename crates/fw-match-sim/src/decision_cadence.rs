//! Decision cadence + stagger — 22 players at 4 Hz on a 60 Hz integration loop.
//!
//! Implements `docs/specs/decision-cadence-stagger.md` (Tranche 4 spec).
//! Implements ADR-0001 layer 2 + ADR-0009 `SeedLayer::Decision` semantics.
//!
//! ## Cadence math
//!
//! - Integration tick: 60 Hz. Each player position advances every 1/60 s.
//! - Per-player decision cadence: 4 Hz. 15 ticks per decision per player.
//! - 22 players × 4 decisions/sec = 88 decisions/sec total.
//! - Stagger distributes 22 players across the 15-tick window: 7 slots get 2
//!   players each, 8 slots get 1 player each (7×2 + 8×1 = 22).
//!
//! ## Determinism
//!
//! - `assign_decision_slots` uses `ChaCha8Rng` seeded via `seed_fn` per
//!   ADR-0009 (`SeedLayer::Decision`, site=0 reserved for the stagger draw).
//! - `SLOT_TEMPLATE` is a compile-time balanced multiset — Fisher-Yates is a
//!   permutation, so the multiset structure is preserved after shuffling.
//! - `decision_slots: [u8; 22]` is canonical-init state; **never mutated**
//!   during the match. Reactive interrupts use the parallel
//!   `interrupt_cooldown_until: [Tick; 22]` field.
//!
//! ## T1-2b-ii scope
//!
//! `should_decide` is defined here but not yet wired into `tick_match` — that
//! happens at T1-2b-iii when the BT runner exists to dispatch to. This module
//! provides the predicate + slot assignment for canonical-state correctness.

// SeedLayer and seed_fn live in fw-core::seed (ADR-0009 canonical
// implementation with the correct 17-byte buffer layout and 0x10..0x40
// discriminant values). Re-exported from this module so existing callers
// that import from decision_cadence continue to compile unchanged.
use fw_core::{Seed, Tick};
pub use fw_core::{SeedLayer, seed_fn};
// `rand_core` traits via `rand_chacha`'s re-export — avoids a direct `rand`
// dep (Codex P1 from self-review triple: rand was redundant with rand_chacha,
// which already re-exports the RngCore + SeedableRng traits we need).
use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{RngCore, SeedableRng};

// ---------------------------------------------------------------------------
// SLOT_TEMPLATE — balanced multiset
// ---------------------------------------------------------------------------

/// Balanced 22-element slot template. 7 slots appear twice (0..=6),
/// 8 slots appear once (7..=14). 7×2 + 8×1 = 22.
///
/// `assign_decision_slots` Fisher-Yates shuffles this array to produce
/// the per-match `decision_slots: [u8; 22]`.
///
/// The template is a constant — the Fisher-Yates shuffle is a permutation
/// that preserves the multiset structure; any permutation of SLOT_TEMPLATE
/// has exactly the same counts.
pub(crate) const SLOT_TEMPLATE: [u8; 22] = [
    0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, // 7 doubled slots = 14 entries
    7, 8, 9, 10, 11, 12, 13, 14, //  8 single slots =  8 entries
];

// ---------------------------------------------------------------------------
// assign_decision_slots — match-init only
// ---------------------------------------------------------------------------

/// Assign decision slots to the 22 roster positions. Called once at match-init
/// with the match seed; the result is stored in `MatchState.decision_slots`
/// and never mutated again.
///
/// Uses Fisher-Yates (Knuth) shuffle over `SLOT_TEMPLATE` with
/// `ChaCha8Rng::seed_from_u64(seed_fn(match_seed, 0, SeedLayer::Decision, 0))`.
/// Per ADR-0009, `SeedLayer::Decision` with `site=0` is **reserved** for the
/// stagger-shuffle draw; per-player decision draws at tick `t` use different
/// site values.
///
/// Roster slot `i+1` gets `slots[i]` (slots are 1-indexed by convention;
/// array is 0-indexed). Substitutions inherit the departing player's slot
/// rather than reassigning — no mid-match reshuffle.
#[must_use]
pub fn assign_decision_slots(seed: Seed) -> [u8; 22] {
    let rng_seed = seed_fn(seed.to_u64(), 0, SeedLayer::Decision, 0);
    let mut rng = ChaCha8Rng::seed_from_u64(rng_seed);
    let mut slots = SLOT_TEMPLATE;
    // Fisher-Yates in-place. `i` counts down from `slots.len()-1` to `1`.
    // `j` is drawn uniformly in `0..=i`. The `% (i+1)` modulo is the
    // classic approach; the Codex P1 finding on birthday-problem applies to
    // assigning 22 draws into 15 bins via random-modulo, NOT to Fisher-Yates
    // itself (Fisher-Yates is a permutation of a balanced template).
    for i in (1..slots.len()).rev() {
        let j = (rng.next_u32() as usize) % (i + 1);
        slots.swap(i, j);
    }
    slots
}

// ---------------------------------------------------------------------------
// should_decide — per-tick firing predicate
// ---------------------------------------------------------------------------

/// Returns `true` if roster slot `roster_slot` (1-indexed, 1..=22) should
/// fire its decision runner at integration tick `tick`.
///
/// Firing rule (from spec §"Firing rule"):
/// - The assigned slot `s = decision_slots[roster_slot - 1]`.
/// - The tick fires the decision iff `tick.raw() % 15 == s`.
/// - BUT: if `tick < interrupt_cooldown_until[roster_slot - 1]`, the
///   scheduled firing is suppressed (reactive interrupt cooldown).
///
/// The `decision_slots` array is not mutated by this function; neither is
/// `interrupt_cooldown_until`. Both are read-only here.
#[must_use]
pub fn should_decide(
    roster_slot: u8,
    decision_slots: &[u8; 22],
    interrupt_cooldown_until: &[Tick; 22],
    tick: Tick,
) -> bool {
    debug_assert!(
        (1..=22).contains(&roster_slot),
        "roster_slot {roster_slot} out of range 1..=22"
    );
    // Codex P1 from self-review: `(tick.to_raw() % 15) as u8` would silently
    // wrap for negative ticks (Rust's `%` returns a value in `(-15, 0]`,
    // `as u8` wraps to a large u8 that never matches a slot in `0..=14`).
    // The result: `should_decide` would return false for ALL slots at ALL
    // negative ticks with no diagnostic — exactly the kind of silent failure
    // T1-2b-iii's BT runner would step on. Use `rem_euclid` for non-negative
    // modulo semantics, gated by a debug-assert that surfaces the upstream
    // bug (ADR-0009 says tick is monotonic non-negative).
    debug_assert!(
        tick.to_raw() >= 0,
        "should_decide called with negative tick {}; \
         ADR-0009 + cadence spec require tick >= 0",
        tick.to_raw()
    );
    let idx = (roster_slot as usize) - 1;
    // Reactive interrupt cooldown: suppress the scheduled decision if still
    // inside a cooldown window. The next firing at `tick >= cooldown_until`
    // resumes the regular cadence.
    if tick < interrupt_cooldown_until[idx] {
        return false;
    }
    tick.to_raw().rem_euclid(15) as u8 == decision_slots[idx]
}

// ---------------------------------------------------------------------------
// Tests — Chunk 4 (cadence types, assign, should_decide)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_core::{Seed, Tick};

    // ------- SLOT_TEMPLATE structural invariant -------

    #[test]
    fn slot_template_has_22_elements() {
        assert_eq!(SLOT_TEMPLATE.len(), 22);
    }

    #[test]
    fn slot_template_has_balanced_multiset() {
        // 7 doubled slots (0..=6) + 8 single slots (7..=14) = 22
        let mut counts = [0u8; 15];
        for &s in &SLOT_TEMPLATE {
            counts[s as usize] += 1;
        }
        for (i, &c) in counts.iter().enumerate() {
            if i < 7 {
                assert_eq!(
                    c, 2,
                    "slot {i} should appear exactly 2 times in SLOT_TEMPLATE"
                );
            } else {
                assert_eq!(
                    c, 1,
                    "slot {i} should appear exactly 1 time in SLOT_TEMPLATE"
                );
            }
        }
    }

    // ------- assign_decision_slots -------

    #[test]
    fn assign_slots_same_seed_produces_same_array() {
        let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);
        let a = assign_decision_slots(seed);
        let b = assign_decision_slots(seed);
        assert_eq!(a, b, "same seed must produce identical slot arrays");
    }

    #[test]
    fn assign_slots_preserves_balanced_multiset() {
        let seed = Seed::from_u64(42);
        let slots = assign_decision_slots(seed);
        let mut counts = [0u8; 15];
        for &s in &slots {
            counts[s as usize] += 1;
        }
        for (i, &c) in counts.iter().enumerate() {
            if i < 7 {
                assert_eq!(c, 2, "after shuffle, slot {i} should still appear 2 times");
            } else {
                assert_eq!(c, 1, "after shuffle, slot {i} should still appear 1 time");
            }
        }
    }

    #[test]
    fn assign_slots_no_slot_exceeds_2() {
        let seed = Seed::from_u64(0xCAFE_BABE);
        let slots = assign_decision_slots(seed);
        let mut counts = [0u8; 15];
        for &s in &slots {
            counts[s as usize] += 1;
        }
        for (i, &c) in counts.iter().enumerate() {
            assert!(
                c <= 2,
                "slot {i} has count {c} > 2 — multiset invariant broken"
            );
        }
    }

    #[test]
    fn assign_slots_no_slot_is_empty() {
        let seed = Seed::from_u64(123456789);
        let slots = assign_decision_slots(seed);
        let mut counts = [0u8; 15];
        for &s in &slots {
            counts[s as usize] += 1;
        }
        for (i, &c) in counts.iter().enumerate() {
            assert!(
                c >= 1,
                "slot {i} has count 0 — balanced template must guarantee ≥1"
            );
        }
    }

    #[test]
    fn assign_slots_different_seeds_usually_differ() {
        // Two different seeds should (almost certainly) produce different arrays.
        // Not a probabilistic test — if they're the same for seeds 1 and 2,
        // that's a real bug (the shuffle would have to produce the same
        // permutation from a different RNG stream).
        let a = assign_decision_slots(Seed::from_u64(1));
        let b = assign_decision_slots(Seed::from_u64(2));
        // It's technically possible for two different seeds to produce the
        // same permutation, but astronomically unlikely for these two seeds.
        // If this fails in CI, investigate the seed_fn, not the test.
        assert_ne!(
            a, b,
            "different seeds produced identical slot arrays — investigate seed_fn"
        );
    }

    // ------- should_decide -------

    fn zero_cooldowns() -> [Tick; 22] {
        [Tick::ZERO; 22]
    }

    #[test]
    fn should_decide_fires_on_assigned_slot() {
        let slots = assign_decision_slots(Seed::from_u64(1));
        let cooldowns = zero_cooldowns();
        let roster_slot: u8 = 1;
        let assigned_slot = slots[0] as i64;
        // The first tick where tick % 15 == assigned_slot
        let tick = Tick::from_raw(assigned_slot);
        assert!(
            should_decide(roster_slot, &slots, &cooldowns, tick),
            "should_decide didn't fire at the assigned slot tick"
        );
    }

    #[test]
    fn should_decide_does_not_fire_on_wrong_tick() {
        let slots = assign_decision_slots(Seed::from_u64(1));
        let cooldowns = zero_cooldowns();
        let roster_slot: u8 = 1;
        let assigned_slot = slots[0] as i64;
        // One tick after the assigned slot (mod 15). Codex self-review P2
        // pointed out the previous `if` guard could silently elide the
        // assertion if the modular arithmetic ever shifted; assert the
        // unconditional contract instead. `(assigned_slot + 1) % 15` differs
        // from `assigned_slot` for every `assigned_slot in 0..=14` because the
        // increment is +1 mod 15 (the only fixed point of +1 mod 15 would
        // require 15 ≡ 0 mod 15 — there isn't one in [0,15)).
        let off_tick_raw = (assigned_slot + 1) % 15;
        let off_tick = Tick::from_raw(off_tick_raw);
        assert_ne!(
            off_tick_raw, assigned_slot,
            "test setup invalid: off_tick should differ from assigned_slot"
        );
        assert!(
            !should_decide(roster_slot, &slots, &cooldowns, off_tick),
            "should_decide fired at the wrong tick (slot={assigned_slot}, tick={off_tick_raw})"
        );
    }

    #[test]
    fn should_decide_suppressed_by_cooldown() {
        let slots = assign_decision_slots(Seed::from_u64(1));
        let roster_slot: u8 = 3;
        let idx = (roster_slot - 1) as usize;
        let assigned_slot = slots[idx] as i64;
        let fire_tick = Tick::from_raw(assigned_slot);

        // Set a cooldown that extends past fire_tick
        let mut cooldowns = zero_cooldowns();
        cooldowns[idx] = Tick::from_raw(assigned_slot + 5);

        assert!(
            !should_decide(roster_slot, &slots, &cooldowns, fire_tick),
            "should_decide should be suppressed by cooldown"
        );
    }

    #[test]
    fn should_decide_resumes_after_cooldown_expires() {
        let slots = assign_decision_slots(Seed::from_u64(1));
        let roster_slot: u8 = 3;
        let idx = (roster_slot - 1) as usize;
        let assigned_slot = slots[idx] as i64;

        // Cooldown expires at tick 10; next assigned slot at tick assigned_slot + 15
        let mut cooldowns = zero_cooldowns();
        cooldowns[idx] = Tick::from_raw(10);

        // A tick at assigned_slot + 15 (definitely after cooldown, same slot mod)
        let resume_tick = Tick::from_raw(assigned_slot + 15);
        assert!(
            should_decide(roster_slot, &slots, &cooldowns, resume_tick),
            "should_decide should resume after cooldown at tick {}",
            resume_tick.to_raw()
        );
    }

    #[test]
    fn should_decide_per_tick_count_is_one_or_two() {
        // Structural invariant: for any given tick, the number of roster slots
        // that "should decide" (ignoring cooldowns) is exactly 1 or 2.
        // This follows from the balanced template structure.
        let seed = Seed::from_u64(0x0000_00AB_CDEF_1234);
        let slots = assign_decision_slots(seed);
        let cooldowns = zero_cooldowns();
        for tick_raw in 0..30i64 {
            let tick = Tick::from_raw(tick_raw);
            let count = (1u8..=22)
                .filter(|&r| should_decide(r, &slots, &cooldowns, tick))
                .count();
            assert!(
                count == 1 || count == 2,
                "tick {tick_raw}: expected 1 or 2 decisions, got {count}"
            );
        }
    }

    // ------- seed_fn -------

    #[test]
    fn seed_fn_is_deterministic() {
        let a = seed_fn(0xDEAD_BEEF, 100, SeedLayer::Decision, 0);
        let b = seed_fn(0xDEAD_BEEF, 100, SeedLayer::Decision, 0);
        assert_eq!(a, b, "seed_fn must be deterministic");
    }

    #[test]
    fn seed_fn_different_layers_produce_different_seeds() {
        let s0 = seed_fn(42, 0, SeedLayer::Decision, 0);
        let s1 = seed_fn(42, 0, SeedLayer::BallPhysics, 0);
        assert_ne!(s0, s1, "different layers must produce different seeds");
    }

    #[test]
    fn seed_fn_different_ticks_produce_different_seeds() {
        let s0 = seed_fn(42, 0, SeedLayer::Decision, 0);
        let s1 = seed_fn(42, 1, SeedLayer::Decision, 0);
        assert_ne!(s0, s1, "different ticks must produce different seeds");
    }

    #[test]
    fn seed_fn_different_sites_produce_different_seeds() {
        let s0 = seed_fn(42, 0, SeedLayer::Decision, 0);
        let s1 = seed_fn(42, 0, SeedLayer::Decision, 1);
        assert_ne!(s0, s1, "different sites must produce different seeds");
    }
}

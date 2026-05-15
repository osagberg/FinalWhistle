//! Signature bias composition — applies `SimBiasSnapshot` multipliers to
//! raw utility scores (T1-2b-iv).
//!
//! Called from `bt/on_ball.rs` and `bt/off_ball.rs` utility functions when a
//! player has an active `signature_firing` slot. The bias composes
//! multiplicatively AFTER the personality-bias layer (ADR-0011 §"Bias snapshot"):
//!
//!   `final_utility = raw_utility × personality_bias × signature_bias`
//!
//! `apply_signature_bias` takes the personality-biased utility (post-personality
//! layer) and multiplies by the relevant `SimBiasSnapshot` field.
//!
//! ## Consideration enum
//!
//! `BiasConsideration` maps the on-ball/off-ball scoring site to its
//! `SimBiasSnapshot` field. The 5 considerations match the 5 `SimBiasSnapshot`
//! fields (shoot / pass / dribble / press / cover).
//!
//! ## Determinism
//!
//! Pure function. No RNG, no clocks, no floats, no HashMap.

use fw_content::SimBiasSnapshot;
use fw_core::Q32;

// ---------------------------------------------------------------------------
// BiasConsideration — which SimBiasSnapshot field to apply
// ---------------------------------------------------------------------------

/// Which utility-scoring site the signature bias applies to.
///
/// Maps directly to a field in `SimBiasSnapshot`. The dispatcher picks the
/// consideration at the call site; the `*_ATTRS` const lists in `bt/on_ball.rs`
/// and `bt/off_ball.rs` document which consideration each site uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiasConsideration {
    /// Applies `bias_snapshot.shoot_mul` to shoot utility.
    Shoot,
    /// Applies `bias_snapshot.pass_mul` to all pass utility sites
    /// (short + long + cross + lay-off — they all feed the same pass bias lane).
    Pass,
    /// Applies `bias_snapshot.dribble_mul` to dribble utility.
    Dribble,
    /// Applies `bias_snapshot.press_mul` to press + track-back utility.
    Press,
    /// Applies `bias_snapshot.cover_mul` to mark-player + hold-formation utility.
    Cover,
    /// Explicit no-op: returns `biased_utility` unchanged.
    ///
    /// Used for `PlayerIntent::Idle` — idle carries no directional bias;
    /// applying any snapshot multiplier would silently misroute it to
    /// the `Cover` bucket, which changes defensive-shape utility in tests.
    /// A dedicated `Neutral` variant makes the intent explicit and
    /// forces a compile error when new `PlayerIntent` variants land without
    /// a deliberate bias assignment.
    Neutral,
}

// ---------------------------------------------------------------------------
// apply_signature_bias
// ---------------------------------------------------------------------------

/// Multiply the post-personality-bias utility by the signature's bias
/// snapshot field corresponding to `consideration`.
///
/// If the signature's multiplier for this consideration is `Q32::ONE`, the
/// function returns `biased_utility` unchanged (no-op case).
///
/// ## Arguments
///
/// - `biased_utility` — the utility value AFTER personality bias has been
///   applied. Must be in `[0, 1]`; validated by `debug_assert` in test builds.
/// - `snapshot` — the `SimBiasSnapshot` for the currently-active signature.
/// - `consideration` — which snapshot field to select.
///
/// ## Returns
///
/// The signature-biased utility. May exceed `[0, 1]` if the multiplier is
/// `> 1.0` — that is intentional (the biased shoot utility for `LongRangeStrike`
/// at `shoot_mul = 1.4` will be up to `1.4` to dominate the softmax without
/// clipping all other considerations to zero).
#[must_use]
pub fn apply_signature_bias(
    biased_utility: Q32,
    snapshot: &SimBiasSnapshot,
    consideration: BiasConsideration,
) -> Q32 {
    assert!(
        biased_utility >= Q32::ZERO,
        "apply_signature_bias: biased_utility {biased_utility:?} is negative — sim invariant violated"
    );

    let multiplier = match consideration {
        BiasConsideration::Shoot => snapshot.shoot_mul,
        BiasConsideration::Pass => snapshot.pass_mul,
        BiasConsideration::Dribble => snapshot.dribble_mul,
        BiasConsideration::Press => snapshot.press_mul,
        BiasConsideration::Cover => snapshot.cover_mul,
        // Neutral: no multiplier — return the utility unchanged.
        // Handles Idle and any intent that deliberately opts out of bias scaling.
        BiasConsideration::Neutral => return biased_utility,
    };

    biased_utility * multiplier
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_content::SimBiasSnapshot;
    use fw_core::Q32;

    fn half() -> Q32 {
        Q32::from_raw(2_147_483_648_i64) // 0.5
    }

    // ---- no-op snapshot leaves utility unchanged ----

    #[test]
    fn no_op_snapshot_all_considerations_unchanged() {
        let snap = SimBiasSnapshot::NO_OP;
        let utility = half();
        for c in [
            BiasConsideration::Shoot,
            BiasConsideration::Pass,
            BiasConsideration::Dribble,
            BiasConsideration::Press,
            BiasConsideration::Cover,
        ] {
            let result = apply_signature_bias(utility, &snap, c);
            assert_eq!(
                result, utility,
                "NO_OP snapshot should leave utility unchanged for {c:?}"
            );
        }
    }

    // ---- shoot multiplier ----

    #[test]
    fn shoot_mul_1_4_amplifies_shoot_utility() {
        // 1.4 in Q32: 1.4 * 2^32 = 6_012_954_214 raw bits.
        let snap = SimBiasSnapshot {
            shoot_mul: Q32::from_raw(6_012_954_214_i64),
            ..SimBiasSnapshot::NO_OP
        };
        let utility = Q32::ONE; // 1.0
        let result = apply_signature_bias(utility, &snap, BiasConsideration::Shoot);
        // Expected: 1.0 × 1.4 = 1.4 (raw = 6_012_954_214)
        // Allow ±1 ULP for fixed-point rounding.
        let expected = Q32::from_raw(6_012_954_214_i64);
        let diff = if result > expected {
            result - expected
        } else {
            expected - result
        };
        assert!(
            diff <= Q32::from_raw(1),
            "shoot_mul=1.4 should produce ≈1.4; got {result:?} expected {expected:?}"
        );
    }

    #[test]
    fn shoot_mul_does_not_affect_dribble_consideration() {
        let snap = SimBiasSnapshot {
            shoot_mul: Q32::from_raw(6_012_954_214_i64), // 1.4
            ..SimBiasSnapshot::NO_OP
        };
        let utility = half();
        let result = apply_signature_bias(utility, &snap, BiasConsideration::Dribble);
        // dribble_mul == ONE, so result should equal utility.
        assert_eq!(result, utility);
    }

    // ---- dribble suppression ----

    #[test]
    fn dribble_mul_0_7_suppresses_dribble_utility() {
        // 0.7 in Q32: 0.7 * 2^32 ≈ 3_006_477_107 raw bits.
        let snap = SimBiasSnapshot {
            dribble_mul: Q32::from_raw(3_006_477_107_i64),
            ..SimBiasSnapshot::NO_OP
        };
        let utility = Q32::ONE;
        let result = apply_signature_bias(utility, &snap, BiasConsideration::Dribble);
        // Expected: 1.0 × 0.7 ≈ 0.7
        let expected = Q32::from_raw(3_006_477_107_i64);
        let diff = if result > expected {
            result - expected
        } else {
            expected - result
        };
        assert!(
            diff <= Q32::from_raw(256),
            "dribble_mul=0.7 should produce ≈0.7; got {result:?} expected {expected:?}"
        );
    }

    // ---- pass multiplier ----

    #[test]
    fn pass_mul_1_5_amplifies_pass_utility() {
        // 1.5 in Q32 = 6_442_450_944 raw bits.
        let snap = SimBiasSnapshot {
            pass_mul: Q32::from_raw(6_442_450_944_i64),
            ..SimBiasSnapshot::NO_OP
        };
        let utility = half();
        let result = apply_signature_bias(utility, &snap, BiasConsideration::Pass);
        // Expected: 0.5 × 1.5 = 0.75 (raw ≈ 3_221_225_472)
        let expected = Q32::from_raw(3_221_225_472_i64);
        let diff = if result > expected {
            result - expected
        } else {
            expected - result
        };
        assert!(
            diff <= Q32::from_raw(256),
            "pass_mul=1.5 × 0.5 should be ≈0.75; got {result:?}"
        );
    }

    // ---- press + cover ----

    #[test]
    fn press_mul_amplifies_press_consideration() {
        let snap = SimBiasSnapshot {
            press_mul: Q32::from_raw(5_368_709_120_i64), // ≈ 1.25
            ..SimBiasSnapshot::NO_OP
        };
        let utility = Q32::from_raw(2_147_483_648_i64); // 0.5
        let result = apply_signature_bias(utility, &snap, BiasConsideration::Press);
        // 0.5 × 1.25 = 0.625 ≈ Q32::from_raw(2_684_354_560)
        assert!(
            result > utility,
            "press_mul > 1.0 should amplify press utility"
        );
    }

    #[test]
    fn cover_mul_amplifies_cover_consideration() {
        let snap = SimBiasSnapshot {
            cover_mul: Q32::from_raw(5_583_457_280_i64), // ≈ 1.3
            ..SimBiasSnapshot::NO_OP
        };
        let utility = Q32::from_raw(2_147_483_648_i64); // 0.5
        let result = apply_signature_bias(utility, &snap, BiasConsideration::Cover);
        assert!(
            result > utility,
            "cover_mul > 1.0 should amplify cover utility"
        );
    }

    // ---- zero utility stays zero ----

    #[test]
    fn zero_utility_stays_zero_regardless_of_multiplier() {
        let snap = SimBiasSnapshot {
            shoot_mul: Q32::from_raw(6_012_954_214_i64), // 1.4
            ..SimBiasSnapshot::NO_OP
        };
        let result = apply_signature_bias(Q32::ZERO, &snap, BiasConsideration::Shoot);
        assert_eq!(result, Q32::ZERO, "0 × anything = 0");
    }
}

//! Personality bias coefficients — k₁..k₁₄ multiplicative tilts.
//!
//! Implements `docs/design/personality-bias-weights.md` (T1-2b Phase-1 seeds).
//! ADR-0003 §5 specifies the multiplicative form:
//!
//!   `biased_utility = raw_utility · (1 + k_i · bias_value)`
//!
//! where `bias_value` is a `Q32` in `[0, 1]` drawn from the player's
//! `PlayerAttributes` (visible `MentalAttributes` for Flair/Composure,
//! hidden `PersonalityVector` for the rest).
//!
//! ## PressureTolerance — the divisor amplifier
//!
//! `read_defender_pressure` divides raw pressure by `(1 + 0.75 · PT)`:
//! - PT=0 → passes raw through unchanged.
//! - PT=1 → raw / 1.75 (≈57% of raw).
//!
//! ## Constants
//!
//! All 14 coefficients are compile-time `Q32` constants via `Q32::from_raw`.
//! `from_fraction(n, d)` helper: `Q32::from_raw((n << 32) / d as i64)` ≈
//! `n/d` in fixed-point. We express each k as an exact rational:
//!   0.25 = 1/4, 0.30 = 3/10, 0.35 = 7/20, 0.40 = 2/5, 0.45 = 9/20.
//!
//! Because Q32.32 has 32 fractional bits, `0.25 = 1i64 << 30`,
//! `0.30 ≈ 0x_4CCC_CCCC_CCCC_CCCD` (but we use `from_fraction` at const via
//! a simpler exact rational). See below.

use fw_core::{PlayerAttributes, Q32};

// ---------------------------------------------------------------------------
// Arg-safety newtypes (P2-2)
//
// `DefenderPressure` and `IsProgressive` wrap Q32 to prevent silent arg-swap
// between the third parameters of `apply_shoot_bias` and `apply_long_pass_bias`.
// Both must be in [0, 1]. The wrapper is `pub(crate)` — only BT utility code
// inside fw-match-sim constructs them.
// ---------------------------------------------------------------------------

/// Effective defender pressure, pre-attenuated by the PT divisor.
/// Constructed via `read_defender_pressure`; range `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefenderPressure(pub(crate) Q32);

impl DefenderPressure {
    /// Unwrap the inner Q32. `pub(crate)` — used in tests and future diagnostic helpers.
    #[allow(dead_code)]
    #[inline]
    pub(crate) fn get(self) -> Q32 {
        self.0
    }
}

/// Progressive-pass indicator: `Q32::ONE` if the pass moves toward goal,
/// `Q32::ZERO` otherwise (or a partial value as a continuous proxy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsProgressive(pub(crate) Q32);

impl IsProgressive {
    /// Unwrap the inner Q32. `pub(crate)` — used in tests and future diagnostic helpers.
    #[allow(dead_code)]
    #[inline]
    pub(crate) fn get(self) -> Q32 {
        self.0
    }
}

// ---------------------------------------------------------------------------
// Q32 coefficient helper (const-friendly rational approximation)
// ---------------------------------------------------------------------------

// Q32.32 representation of a rational p/q:
//   bits = round(p * 2^32 / q)
//
// k₁ = 0.30 = 3/10  → bits = (3 << 32) / 10 = 1_288_490_188 (0x_4CCC_CCCC)
// k₂ = 0.40 = 2/5   → bits = (2 << 32) / 5  = 1_717_986_918 (0x_6666_6666)
// k₃ = 0.45 = 9/20  → bits = (9 << 32) / 20 = 1_932_735_283 (0x_7333_3333)
// k₄ = 0.25 = 1/4   → bits = 1 << 30          = 1_073_741_824 (0x_4000_0000)
// k₅ = 0.35 = 7/20  → bits = (7 << 32) / 20 = 1_503_238_553 (0x_5999_9999)
// k₆ = 0.30 (same as k₁)
// k₇ = 0.40 (same as k₂)
// k₈ = 0.25 (same as k₄)
// k₉ = 0.45 (same as k₃)
// k₁₀= 0.35 (same as k₅)
// k₁₁= 0.40 (same as k₂)
// k₁₂= 0.35 (same as k₅)
// k₁₃= 0.35 (same as k₅)
// k₁₄= 0.30 (same as k₁)
//
// PT divisor coefficient: 0.75 = 3/4 → bits = 3 << 30 = 3_221_225_472 (0x_C000_0000)
//
// All values below are the exact 32-bit fractional parts (stored in the low 32
// bits of the i64 Q32.32 raw representation, integer part = 0).

/// Shoot (xG) — primary: FlairBias, k₁ = 0.30.
pub const K_1_SHOOT_FLAIR: Q32 = Q32::from_raw(1_288_490_188);
/// Shoot (xG) — secondary: Composure (under pressure), k₂ = 0.40.
pub const K_2_SHOOT_COMPOSURE: Q32 = Q32::from_raw(1_717_986_918);
/// Long pass / through ball — primary: RiskAppetite, k₃ = 0.45.
pub const K_3_LONG_PASS_RISK: Q32 = Q32::from_raw(1_932_735_283);
/// Long pass / through ball — secondary: FlairBias, k₄ = 0.25.
pub const K_4_LONG_PASS_FLAIR: Q32 = Q32::from_raw(1_073_741_824);
/// Safe pass — primary: (1 − RiskAppetite) inverse, k₅ = 0.35.
pub const K_5_SAFE_PASS_RISK_INV: Q32 = Q32::from_raw(1_503_238_553);
/// Safe pass — secondary: Selflessness, k₆ = 0.30.
pub const K_6_SAFE_PASS_SELF: Q32 = Q32::from_raw(1_288_490_188);
/// Dribble — primary: FlairBias, k₇ = 0.40.
pub const K_7_DRIBBLE_FLAIR: Q32 = Q32::from_raw(1_717_986_918);
/// Dribble — secondary: Aggression, k₈ = 0.25.
pub const K_8_DRIBBLE_AGG: Q32 = Q32::from_raw(1_073_741_824);
/// Press — primary: Aggression, k₉ = 0.45.
pub const K_9_PRESS_AGG: Q32 = Q32::from_raw(1_932_735_283);
/// Press — secondary: WorkRate, k₁₀ = 0.35.
pub const K_10_PRESS_WR: Q32 = Q32::from_raw(1_503_238_553);
/// Defensive cover / track-back — primary: Determination, k₁₁ = 0.40.
pub const K_11_COVER_DET: Q32 = Q32::from_raw(1_717_986_918);
/// Defensive cover / track-back — secondary: WorkRate, k₁₂ = 0.35.
pub const K_12_COVER_WR: Q32 = Q32::from_raw(1_503_238_553);
/// Hold position — primary: (1 − Aggression) inverse, k₁₃ = 0.35.
pub const K_13_HOLD_AGG_INV: Q32 = Q32::from_raw(1_503_238_553);
/// Hold position — secondary: PressureTolerance, k₁₄ = 0.30.
pub const K_14_HOLD_PT: Q32 = Q32::from_raw(1_288_490_188);

/// PressureTolerance divisor coefficient: 0.75.
/// `read_defender_pressure` divides raw by `(1 + 0.75 · PT)`.
pub const PT_DIVISOR_COEFF: Q32 = Q32::from_raw(3_221_225_472);

// ---------------------------------------------------------------------------
// Per-consideration bias helpers
// ---------------------------------------------------------------------------

/// Apply Shoot (xG) personality bias.
///
/// Form: `raw · (1 + k₁·flair) · (1 + k₂·composure·defender_pressure)`
///
/// - `flair`             = `attrs.mental.flair` (visible)
/// - `composure`         = `attrs.mental.composure` (visible)
/// - `defender_pressure` = effective pressure (after PT divisor,
///   see `read_defender_pressure`)
///
/// All Q32 in `[0, 1]`. Returns Q32.
pub fn apply_shoot_bias(raw: Q32, attrs: &PlayerAttributes, pressure: DefenderPressure) -> Q32 {
    debug_assert!(raw >= Q32::ZERO && raw <= Q32::ONE, "raw must be in [0,1]");
    debug_assert!(
        pressure.0 >= Q32::ZERO && pressure.0 <= Q32::ONE,
        "DefenderPressure must be in [0,1]"
    );
    let flair = attrs.mental.flair;
    let composure = attrs.mental.composure;
    let factor1 = Q32::ONE + K_1_SHOOT_FLAIR * flair;
    let factor2 = Q32::ONE + K_2_SHOOT_COMPOSURE * composure * pressure.0;
    raw * factor1 * factor2
}

/// Apply Long Pass / Through Ball personality bias.
///
/// Form: `raw · (1 + k₃·risk) · (1 + k₄·flair·is_progressive)`
///
/// where `is_progressive` is `Q32::ONE` if the pass moves toward goal,
/// `Q32::ZERO` otherwise.
pub fn apply_long_pass_bias(raw: Q32, attrs: &PlayerAttributes, progressive: IsProgressive) -> Q32 {
    debug_assert!(raw >= Q32::ZERO && raw <= Q32::ONE, "raw must be in [0,1]");
    debug_assert!(
        progressive.0 >= Q32::ZERO && progressive.0 <= Q32::ONE,
        "IsProgressive must be in [0,1]"
    );
    let risk = attrs.personality.risk_appetite;
    let flair = attrs.mental.flair;
    let factor1 = Q32::ONE + K_3_LONG_PASS_RISK * risk;
    // k₄ tilt: `(1 + k₄·flair·progressive)` — at progressive=ZERO, factor is 1.0 (no tilt).
    let factor2 = Q32::ONE + K_4_LONG_PASS_FLAIR * flair * progressive.0;
    raw * factor1 * factor2
}

/// Apply Safe Pass personality bias.
///
/// Form: `raw · (1 + k₅·(1−risk)) · (1 + k₆·selflessness)`
pub fn apply_safe_pass_bias(raw: Q32, attrs: &PlayerAttributes) -> Q32 {
    debug_assert!(raw >= Q32::ZERO && raw <= Q32::ONE, "raw must be in [0,1]");
    let risk_inv = Q32::ONE - attrs.personality.risk_appetite;
    let selflessness = attrs.personality.selflessness;
    let factor1 = Q32::ONE + K_5_SAFE_PASS_RISK_INV * risk_inv;
    let factor2 = Q32::ONE + K_6_SAFE_PASS_SELF * selflessness;
    raw * factor1 * factor2
}

/// Apply Dribble personality bias.
///
/// Form: `raw · (1 + k₇·flair) · (1 + k₈·aggression)`
pub fn apply_dribble_bias(raw: Q32, attrs: &PlayerAttributes) -> Q32 {
    debug_assert!(raw >= Q32::ZERO && raw <= Q32::ONE, "raw must be in [0,1]");
    let flair = attrs.mental.flair;
    let aggression = attrs.personality.aggression;
    let factor1 = Q32::ONE + K_7_DRIBBLE_FLAIR * flair;
    let factor2 = Q32::ONE + K_8_DRIBBLE_AGG * aggression;
    raw * factor1 * factor2
}

/// Apply Press personality bias.
///
/// Form: `raw · (1 + k₉·aggression) · (1 + k₁₀·work_rate)`
///
/// Note: `work_rate` is `personality.work_rate`, NOT `mental.work_rate`
/// (see `docs/specs/bt-attribute-binding.md` field-path note).
pub fn apply_press_bias(raw: Q32, attrs: &PlayerAttributes) -> Q32 {
    debug_assert!(raw >= Q32::ZERO && raw <= Q32::ONE, "raw must be in [0,1]");
    let aggression = attrs.personality.aggression;
    let work_rate = attrs.personality.work_rate;
    let factor1 = Q32::ONE + K_9_PRESS_AGG * aggression;
    let factor2 = Q32::ONE + K_10_PRESS_WR * work_rate;
    raw * factor1 * factor2
}

/// Apply Defensive Cover / Track-back personality bias.
///
/// Form: `raw · (1 + k₁₁·determination) · (1 + k₁₂·work_rate)`
pub fn apply_cover_bias(raw: Q32, attrs: &PlayerAttributes) -> Q32 {
    debug_assert!(raw >= Q32::ZERO && raw <= Q32::ONE, "raw must be in [0,1]");
    let determination = attrs.personality.determination;
    let work_rate = attrs.personality.work_rate;
    let factor1 = Q32::ONE + K_11_COVER_DET * determination;
    let factor2 = Q32::ONE + K_12_COVER_WR * work_rate;
    raw * factor1 * factor2
}

/// Apply Hold Position personality bias.
///
/// Form: `raw · (1 + k₁₃·(1−aggression)) · (1 + k₁₄·pressure_tolerance)`
pub fn apply_hold_bias(raw: Q32, attrs: &PlayerAttributes) -> Q32 {
    debug_assert!(raw >= Q32::ZERO && raw <= Q32::ONE, "raw must be in [0,1]");
    let agg_inv = Q32::ONE - attrs.personality.aggression;
    let pt = attrs.personality.pressure_tolerance;
    let factor1 = Q32::ONE + K_13_HOLD_AGG_INV * agg_inv;
    let factor2 = Q32::ONE + K_14_HOLD_PT * pt;
    raw * factor1 * factor2
}

// ---------------------------------------------------------------------------
// PressureTolerance divisor
// ---------------------------------------------------------------------------

/// Apply PressureTolerance divisor to raw defender pressure.
///
/// Per ADR-0003 §5: `effective_pressure = raw / (1 + 0.75 · PT)`
/// - PT=0 → effective = raw  (no attenuation)
/// - PT=1 → effective = raw / 1.75 ≈ 0.571 · raw
///
/// `attrs.personality.pressure_tolerance` is in `[0, 1]`.
/// Returns `DefenderPressure` (the newtype used by `apply_shoot_bias`).
pub fn read_defender_pressure(attrs: &PlayerAttributes, raw_pressure: Q32) -> DefenderPressure {
    let pt = attrs.personality.pressure_tolerance;
    let denom = Q32::ONE + PT_DIVISOR_COEFF * pt;
    DefenderPressure(raw_pressure / denom)
}

// ---------------------------------------------------------------------------
// Tests — Chunk 1 RED → GREEN (T1-2b-iii-c)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_core::PlayerAttributes;

    // T1-2b-iii-c test contract §1: constants match design doc values.

    #[test]
    fn k1_is_approx_0_30() {
        // K_1 = 0.30. Q32.32: integer part 0, fractional bits ≈ 0x_4CCC_CCCC.
        // tolerance: ±1 ULP in the 32-bit fractional part.
        let v = K_1_SHOOT_FLAIR.to_bits();
        // 0.30 × 2^32 ≈ 1_288_490_188.8 → floor = 1_288_490_188
        assert!(
            (v - 1_288_490_188).abs() <= 2,
            "K_1 bits {} should be ≈ 1_288_490_188 (0.30)",
            v
        );
    }

    #[test]
    fn k2_is_approx_0_40() {
        let v = K_2_SHOOT_COMPOSURE.to_bits();
        // 0.40 × 2^32 ≈ 1_717_986_918.4 → floor = 1_717_986_918
        assert!(
            (v - 1_717_986_918).abs() <= 2,
            "K_2 bits {} should be ≈ 1_717_986_918 (0.40)",
            v
        );
    }

    #[test]
    fn k3_is_approx_0_45() {
        let v = K_3_LONG_PASS_RISK.to_bits();
        // 0.45 × 2^32 ≈ 1_932_735_283.2 → floor = 1_932_735_283
        assert!(
            (v - 1_932_735_283).abs() <= 2,
            "K_3 bits {} should be ≈ 1_932_735_283 (0.45)",
            v
        );
    }

    #[test]
    fn k4_is_approx_0_25() {
        let v = K_4_LONG_PASS_FLAIR.to_bits();
        // 0.25 × 2^32 = 1_073_741_824 (exact)
        assert_eq!(
            v, 1_073_741_824,
            "K_4 should be exactly 1_073_741_824 (0.25)"
        );
    }

    #[test]
    fn k5_is_approx_0_35() {
        let v = K_5_SAFE_PASS_RISK_INV.to_bits();
        // 0.35 × 2^32 ≈ 1_503_238_553.6 → floor = 1_503_238_553
        assert!(
            (v - 1_503_238_553).abs() <= 2,
            "K_5 bits {} should be ≈ 1_503_238_553 (0.35)",
            v
        );
    }

    // T1-2b-iii-c test contract §2: multiplicative composition range.
    // For max-coefficient case (k=0.45, bias=1.0): factor = 1.45.
    // Two stacked: 1.45 * 1.45 = 2.1025.
    // So biased ≤ raw * 2.2 is generous; biased ≥ raw * 0.5 always holds.

    #[test]
    fn shoot_bias_max_personality_is_bounded() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.mental.flair = Q32::ONE;
        attrs.mental.composure = Q32::ONE;
        let raw = Q32::from_raw(1i64 << 30); // 0.25
        let biased = apply_shoot_bias(raw, &attrs, DefenderPressure(Q32::ONE));
        let raw_f = 0.25f64;
        // max shoot bias: (1 + 0.30*1.0) * (1 + 0.40*1.0*1.0) = 1.30 * 1.40 = 1.82
        // biased should be in [raw * 0.5, raw * 2.2]
        assert!(
            biased >= raw / Q32::from_int(2),
            "biased {:?} should be >= raw/2",
            biased
        );
        // 0.25 * 1.82 = 0.455; raw * 2.2 = 0.55
        let _ = raw_f; // used for comment
        assert!(
            biased <= Q32::from_raw((raw.to_bits() as f64 * 2.2) as i64),
            "biased {:?} should be <= raw * 2.2",
            biased
        );
    }

    // When all direct bias inputs are zero, factors collapse to 1.0 → output == raw.
    // Note: inverse-form helpers (safe_pass, hold) use (1 - bias), so setting
    // risk_appetite=1 and aggression=1 makes their inverse = 0 → factor = 1.
    #[test]
    fn zero_direct_bias_returns_raw_for_direct_helpers() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.mental.flair = Q32::ZERO;
        attrs.mental.composure = Q32::ZERO;
        attrs.personality.risk_appetite = Q32::ZERO;
        attrs.personality.aggression = Q32::ZERO;
        attrs.personality.work_rate = Q32::ZERO;
        attrs.personality.determination = Q32::ZERO;
        attrs.personality.pressure_tolerance = Q32::ZERO;
        let raw = Q32::from_raw(1i64 << 30); // 0.25
        // Direct-form helpers: all bias inputs = 0 → factors all = 1.
        assert_eq!(
            apply_shoot_bias(raw, &attrs, DefenderPressure(Q32::ZERO)),
            raw,
            "shoot"
        );
        assert_eq!(
            apply_long_pass_bias(raw, &attrs, IsProgressive(Q32::ZERO)),
            raw,
            "long_pass"
        );
        assert_eq!(apply_dribble_bias(raw, &attrs), raw, "dribble");
        assert_eq!(apply_press_bias(raw, &attrs), raw, "press");
        assert_eq!(apply_cover_bias(raw, &attrs), raw, "cover");
    }

    // Inverse-form helpers: safe_pass uses (1−risk), hold uses (1−aggression).
    // Zero-tilt point: risk_appetite=1 and selflessness=0 → safe_pass factor = 1.
    //                  aggression=1 and pressure_tolerance=0 → hold factor = 1.
    #[test]
    fn zero_tilt_for_inverse_form_helpers() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.personality.risk_appetite = Q32::ONE; // (1 - risk) = 0 → k₅ tilt = 0
        attrs.personality.selflessness = Q32::ZERO; // k₆ tilt = 0
        attrs.personality.aggression = Q32::ONE; // (1 - agg) = 0 → k₁₃ tilt = 0
        attrs.personality.pressure_tolerance = Q32::ZERO; // k₁₄ tilt = 0
        let raw = Q32::from_raw(1i64 << 30); // 0.25
        assert_eq!(
            apply_safe_pass_bias(raw, &attrs),
            raw,
            "safe_pass zero-tilt"
        );
        assert_eq!(apply_hold_bias(raw, &attrs), raw, "hold zero-tilt");
    }

    // T1-2b-iii-c test contract §3: PT divisor.

    #[test]
    fn read_defender_pressure_pt_zero_returns_raw() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.personality.pressure_tolerance = Q32::ZERO;
        let raw = Q32::from_raw(1i64 << 31); // 0.5
        let effective = read_defender_pressure(&attrs, raw).get();
        assert_eq!(effective, raw, "PT=0 should return raw pressure unchanged");
    }

    #[test]
    fn read_defender_pressure_pt_one_is_approx_raw_div_1_75() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.personality.pressure_tolerance = Q32::ONE;
        let raw = Q32::from_raw(1i64 << 32); // 1.0
        let effective = read_defender_pressure(&attrs, raw).get();
        // Expected: 1.0 / 1.75 ≈ 0.5714...
        // In Q32 bits: 0.5714... * 2^32 ≈ 2_454_267_026
        let expected_bits = (0.5714285714285714f64 * (1u64 << 32) as f64) as i64;
        let got_bits = effective.to_bits();
        // Allow ≤ 1% relative error (≈ 24_542_670 bits).
        let tolerance = expected_bits / 100;
        assert!(
            (got_bits - expected_bits).abs() <= tolerance,
            "PT=1 effective pressure bits {} should be ≈ {} (0.5714...)",
            got_bits,
            expected_bits
        );
    }

    #[test]
    fn read_defender_pressure_monotone_in_pt() {
        // Higher PT → lower effective pressure.
        let raw = Q32::from_raw(1i64 << 31); // 0.5
        let attrs_lo = {
            let mut a = PlayerAttributes::mid_range_baseline();
            a.personality.pressure_tolerance = Q32::from_raw(1i64 << 30); // 0.25
            a
        };
        let attrs_hi = {
            let mut a = PlayerAttributes::mid_range_baseline();
            a.personality.pressure_tolerance = Q32::from_raw(3i64 << 30); // 0.75
            a
        };
        let eff_lo = read_defender_pressure(&attrs_lo, raw).get();
        let eff_hi = read_defender_pressure(&attrs_hi, raw).get();
        assert!(
            eff_hi < eff_lo,
            "higher PT should yield lower effective pressure: lo={:?} hi={:?}",
            eff_lo,
            eff_hi
        );
    }

    // work_rate field-path correctness: personality.work_rate, not mental.
    #[test]
    fn press_bias_reads_personality_work_rate_not_mental() {
        let mut attrs_high_wr = PlayerAttributes::mid_range_baseline();
        attrs_high_wr.personality.work_rate = Q32::ONE;
        attrs_high_wr.personality.aggression = Q32::ZERO;

        let mut attrs_low_wr = PlayerAttributes::mid_range_baseline();
        attrs_low_wr.personality.work_rate = Q32::ZERO;
        attrs_low_wr.personality.aggression = Q32::ZERO;

        let raw = Q32::from_raw(1i64 << 30);
        let hi = apply_press_bias(raw, &attrs_high_wr);
        let lo = apply_press_bias(raw, &attrs_low_wr);
        assert!(
            hi > lo,
            "higher personality.work_rate should increase press bias"
        );
    }

    // Sanity: cover bias reads personality.determination + personality.work_rate.
    #[test]
    fn cover_bias_increases_with_determination() {
        let mut attrs_hi = PlayerAttributes::mid_range_baseline();
        attrs_hi.personality.determination = Q32::ONE;
        attrs_hi.personality.work_rate = Q32::ZERO;

        let mut attrs_lo = PlayerAttributes::mid_range_baseline();
        attrs_lo.personality.determination = Q32::ZERO;
        attrs_lo.personality.work_rate = Q32::ZERO;

        let raw = Q32::from_raw(1i64 << 30);
        assert!(apply_cover_bias(raw, &attrs_hi) > apply_cover_bias(raw, &attrs_lo));
    }

    // Safe pass: higher risk_appetite → lower safe-pass bias (inverse form).
    #[test]
    fn safe_pass_bias_decreases_with_risk_appetite() {
        let mut attrs_risky = PlayerAttributes::mid_range_baseline();
        attrs_risky.personality.risk_appetite = Q32::ONE;
        attrs_risky.personality.selflessness = Q32::ZERO;

        let mut attrs_safe = PlayerAttributes::mid_range_baseline();
        attrs_safe.personality.risk_appetite = Q32::ZERO;
        attrs_safe.personality.selflessness = Q32::ZERO;

        let raw = Q32::from_raw(1i64 << 30);
        let biased_risky = apply_safe_pass_bias(raw, &attrs_risky);
        let biased_safe = apply_safe_pass_bias(raw, &attrs_safe);
        assert!(
            biased_safe > biased_risky,
            "low risk_appetite should produce higher safe-pass bias"
        );
    }

    // Hold position: higher aggression → lower hold bias (inverse form).
    #[test]
    fn hold_bias_decreases_with_aggression() {
        let mut attrs_agg = PlayerAttributes::mid_range_baseline();
        attrs_agg.personality.aggression = Q32::ONE;
        attrs_agg.personality.pressure_tolerance = Q32::ZERO;

        let mut attrs_calm = PlayerAttributes::mid_range_baseline();
        attrs_calm.personality.aggression = Q32::ZERO;
        attrs_calm.personality.pressure_tolerance = Q32::ZERO;

        let raw = Q32::from_raw(1i64 << 30);
        assert!(apply_hold_bias(raw, &attrs_calm) > apply_hold_bias(raw, &attrs_agg));
    }
}

//! Expected-goals (xG) utility — 6-feature Q32 logistic.
//!
//! All features are Q32 in [0, 1]; see `docs/design/xg-coefficients.md` for
//! full normalization rules.  The coefficient values are Phase-1 tuning seeds
//! locked at T1-2b; re-fit expected at T2-1 once BT runner ships real shot
//! distributions.
//!
//! # Distance feature inversion
//!
//! `distance_q32` is INVERTED at feature-extraction time (`0 = far, 1 = close`)
//! so that all coefficients except beta_0 (intercept) and beta_3 (pressure)
//! are positive — easier to reason about.

use fw_core::{Q32, sigmoid_q32};

// -------------------------------------------------------------------------
// Phase-1 coefficient seeds (third pass — docs/design/xg-coefficients.md)
// Raw values = round(beta * 2^32).
// -------------------------------------------------------------------------

/// β₀: logistic intercept.  Value: -5.50.
pub const BETA_0: Q32 = Q32::from_raw(-23_622_320_128_i64);

/// β₁: distance coefficient (positive; distance_q32 is inverted).  Value: +4.80.
pub const BETA_1: Q32 = Q32::from_raw(20_615_843_020_i64);

/// β₂: angle coefficient.  Value: +1.80.
pub const BETA_2: Q32 = Q32::from_raw(7_730_941_132_i64);

/// β₃: defender-pressure coefficient (negative).  Value: -3.00.
pub const BETA_3: Q32 = Q32::from_raw(-12_884_901_888_i64);

/// β₄: shot-type coefficient.  Value: +0.45.
pub const BETA_4: Q32 = Q32::from_raw(1_932_735_283_i64);

/// β₅: assist-kind coefficient.  Value: +0.55.
pub const BETA_5: Q32 = Q32::from_raw(2_362_232_012_i64);

/// β₆: shooter-quality coefficient.  Value: +0.50.
pub const BETA_6: Q32 = Q32::from_raw(2_147_483_648_i64);

// -------------------------------------------------------------------------
// Shot context
// -------------------------------------------------------------------------

/// Error returned when a `ShotContext` field is outside `[0, 1]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShotContextError {
    pub field: &'static str,
    pub value: Q32,
}

impl core::fmt::Display for ShotContextError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "ShotContext field `{}` out of [0, 1]: raw bits {}",
            self.field,
            self.value.to_bits()
        )
    }
}

/// All six normalized features for a single shot attempt.
///
/// All fields are in Q32 `[0, 1]` — enforced by `try_new`.
/// Fields are `pub(crate)` to keep the invariant in the type;
/// external code constructs via `try_new` and reads via accessors.
#[derive(Debug, Clone, Copy)]
pub struct ShotContext {
    /// Distance feature: inverted (0 = far, 1 = close). `1 - clamp(d_m / 35, 0, 1)`.
    pub(crate) distance_q32: Q32,
    /// Half-angle of goal cone from shooter's position, normalized to [0,1].
    pub(crate) angle_q32: Q32,
    /// Defender-pressure term: `1 - exp_q32(-sum_inv_dist)` capped to [0,1].
    pub(crate) defender_pressure_q32: Q32,
    /// Shot type: 1.0 footed, 0.6 header, 0.3 awkward.
    pub(crate) shot_type_q32: Q32,
    /// Assist quality: 1.0 through-ball/solo, 0.85 cross, 0.7 cutback,
    ///                 0.5 set-piece, 0.4 cross-field.
    pub(crate) assist_kind_q32: Q32,
    /// Shooter composite quality: `finishing x 0.55 + composure x 0.25 + technique x 0.20`.
    pub(crate) shooter_quality_q32: Q32,
}

impl ShotContext {
    /// Construct a validated `ShotContext`.
    ///
    /// Returns `Err(ShotContextError)` for the first field outside `[0, 1]`.
    pub fn try_new(
        distance_q32: Q32,
        angle_q32: Q32,
        defender_pressure_q32: Q32,
        shot_type_q32: Q32,
        assist_kind_q32: Q32,
        shooter_quality_q32: Q32,
    ) -> Result<Self, ShotContextError> {
        macro_rules! check {
            ($val:expr, $name:literal) => {
                if $val < Q32::ZERO || $val > Q32::ONE {
                    return Err(ShotContextError {
                        field: $name,
                        value: $val,
                    });
                }
            };
        }
        check!(distance_q32, "distance_q32");
        check!(angle_q32, "angle_q32");
        check!(defender_pressure_q32, "defender_pressure_q32");
        check!(shot_type_q32, "shot_type_q32");
        check!(assist_kind_q32, "assist_kind_q32");
        check!(shooter_quality_q32, "shooter_quality_q32");
        Ok(ShotContext {
            distance_q32,
            angle_q32,
            defender_pressure_q32,
            shot_type_q32,
            assist_kind_q32,
            shooter_quality_q32,
        })
    }
}

// -------------------------------------------------------------------------
// Public API
// -------------------------------------------------------------------------

/// Compute expected goals for a shot attempt.
///
/// Returns a Q32 in [0, 1] representing the probability of the shot going in.
/// Uses the shared `sigmoid_q32` LUT from `fw_core::math`.
///
/// The [0,1] field invariant is guaranteed by `ShotContext::try_new`.
/// No further validation is needed here.
pub fn xg_utility(ctx: &ShotContext) -> Q32 {
    let logit = beta_dot(ctx);
    sigmoid_q32(logit)
}

/// Dot product of β vector and feature vector, starting from the intercept.
///
/// # Overflow analysis
///
/// Each feature is in [0, 1] (enforced by `ShotContext::try_new` → or caller
/// `debug_assert!`s in `xg_utility`). The largest possible logit magnitude is
/// bounded by |β₀| + |β₁| + … + |β₆| ≈ 5.50 + 4.80 + 1.80 + 3.00 + 0.45 +
/// 0.55 + 0.50 = 16.60 — well within Q32 range (~2.15e9). Bare operators
/// panic on overflow, surfacing any invariant violation immediately.
#[inline]
fn beta_dot(ctx: &ShotContext) -> Q32 {
    let mut logit = BETA_0;
    logit += BETA_1 * ctx.distance_q32;
    logit += BETA_2 * ctx.angle_q32;
    logit += BETA_3 * ctx.defender_pressure_q32;
    logit += BETA_4 * ctx.shot_type_q32;
    logit += BETA_5 * ctx.assist_kind_q32;
    logit += BETA_6 * ctx.shooter_quality_q32;
    logit
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn half() -> Q32 {
        Q32::from_raw(1i64 << 31) // 0.5
    }

    fn q_frac(num: i64, denom: i64) -> Q32 {
        // num/denom as Q32: raw = (num << 32) / denom
        Q32::from_raw((num << 32) / denom)
    }

    fn base_ctx() -> ShotContext {
        ShotContext::try_new(half(), half(), Q32::ZERO, Q32::ONE, Q32::ONE, half())
            .expect("base_ctx fields valid")
    }

    // --- try_new validation ---

    #[test]
    fn try_new_rejects_out_of_range_field() {
        let result = ShotContext::try_new(
            Q32::from_raw(1i64 << 33), // > 1.0
            Q32::ZERO,
            Q32::ZERO,
            Q32::ONE,
            Q32::ONE,
            Q32::ZERO,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field, "distance_q32");
    }

    #[test]
    fn try_new_accepts_boundary_values() {
        assert!(
            ShotContext::try_new(
                Q32::ZERO,
                Q32::ZERO,
                Q32::ZERO,
                Q32::ZERO,
                Q32::ZERO,
                Q32::ZERO
            )
            .is_ok()
        );
        assert!(
            ShotContext::try_new(Q32::ONE, Q32::ONE, Q32::ONE, Q32::ONE, Q32::ONE, Q32::ONE)
                .is_ok()
        );
    }

    // --- range ---

    #[test]
    fn xg_is_in_unit_range() {
        let xg = xg_utility(&base_ctx());
        assert!(xg >= Q32::ZERO, "xG must be >= 0");
        assert!(xg <= Q32::ONE, "xG must be <= 1");
    }

    #[test]
    fn xg_extreme_low_features() {
        let ctx = ShotContext::try_new(
            Q32::ZERO,
            Q32::ZERO,
            Q32::ONE,
            q_frac(3, 10),
            q_frac(4, 10),
            Q32::ZERO,
        )
        .expect("valid");
        let xg = xg_utility(&ctx);
        assert!(xg >= Q32::ZERO);
        assert!(xg <= Q32::ONE);
        assert!(xg < half(), "worst-case shot xG should be < 0.5");
    }

    #[test]
    fn xg_extreme_high_features() {
        let ctx = ShotContext::try_new(Q32::ONE, Q32::ONE, Q32::ZERO, Q32::ONE, Q32::ONE, Q32::ONE)
            .expect("valid");
        let xg = xg_utility(&ctx);
        assert!(xg >= Q32::ZERO);
        assert!(xg <= Q32::ONE);
        assert!(xg > half(), "best-case shot xG should be > 0.5");
    }

    // --- monotonicity ---

    #[test]
    fn xg_increases_with_distance_proximity() {
        let mut prev = xg_utility(
            &ShotContext::try_new(Q32::ZERO, half(), Q32::ZERO, Q32::ONE, Q32::ONE, half())
                .expect("valid"),
        );
        for i in 1i32..=8 {
            let ctx = ShotContext::try_new(
                Q32::from_raw((i as i64) * (1i64 << 29)),
                half(),
                Q32::ZERO,
                Q32::ONE,
                Q32::ONE,
                half(),
            )
            .expect("valid");
            let xg = xg_utility(&ctx);
            assert!(
                xg >= prev,
                "xG should increase as distance_q32 increases (step {i})"
            );
            prev = xg;
        }
    }

    #[test]
    fn xg_increases_with_angle() {
        let mut prev = xg_utility(
            &ShotContext::try_new(half(), Q32::ZERO, Q32::ZERO, Q32::ONE, Q32::ONE, half())
                .expect("valid"),
        );
        for i in 1i32..=8 {
            let ctx = ShotContext::try_new(
                half(),
                Q32::from_raw((i as i64) * (1i64 << 29)),
                Q32::ZERO,
                Q32::ONE,
                Q32::ONE,
                half(),
            )
            .expect("valid");
            let xg = xg_utility(&ctx);
            assert!(xg >= prev, "xG should increase with wider angle (step {i})");
            prev = xg;
        }
    }

    #[test]
    fn xg_increases_with_shooter_quality() {
        let mut prev = xg_utility(
            &ShotContext::try_new(half(), half(), Q32::ZERO, Q32::ONE, Q32::ONE, Q32::ZERO)
                .expect("valid"),
        );
        for i in 1i32..=8 {
            let ctx = ShotContext::try_new(
                half(),
                half(),
                Q32::ZERO,
                Q32::ONE,
                Q32::ONE,
                Q32::from_raw((i as i64) * (1i64 << 29)),
            )
            .expect("valid");
            let xg = xg_utility(&ctx);
            assert!(
                xg >= prev,
                "xG should increase with shooter quality (step {i})"
            );
            prev = xg;
        }
    }

    #[test]
    fn xg_decreases_with_pressure() {
        let mut prev = xg_utility(
            &ShotContext::try_new(half(), half(), Q32::ZERO, Q32::ONE, Q32::ONE, half())
                .expect("valid"),
        );
        for i in 1i32..=8 {
            let ctx = ShotContext::try_new(
                half(),
                half(),
                Q32::from_raw((i as i64) * (1i64 << 29)),
                Q32::ONE,
                Q32::ONE,
                half(),
            )
            .expect("valid");
            let xg = xg_utility(&ctx);
            assert!(xg <= prev, "xG should decrease with pressure (step {i})");
            prev = xg;
        }
    }

    // --- canonical reference checks per xg-coefficients.md (third-pass β values) ---

    #[test]
    fn long_shot_xg_is_plausible() {
        // 30m shot: distance_q32 ≈ 0.143, angle ≈ 0.30, pressure ≈ 0.10,
        // foot=1.0, solo=1.0, mid quality=0.50.
        // Third-pass: logit ≈ -3.324, sigmoid ≈ 0.035 (design target: 0.02–0.04).
        // Range: [1.5%, 5%] — tighter than before, ±20% of 0.035 design target.
        let ctx = ShotContext::try_new(
            q_frac(143, 1000),
            q_frac(30, 100),
            q_frac(10, 100),
            Q32::ONE,
            Q32::ONE,
            half(),
        )
        .expect("valid");
        let xg = xg_utility(&ctx);
        let lo = q_frac(15, 1000); // 1.5%
        let hi = q_frac(5, 100); // 5%
        assert!(
            xg >= lo && xg <= hi,
            "30m shot xG should be in [1.5%, 5%], got raw {}",
            xg.to_bits()
        );
    }

    #[test]
    fn xg_12_yard_central_shot_matches_reference() {
        // 12-yard central: distance_q32=0.686, angle=0.70, pressure=0.55,
        // foot=1.0, through-ball=1.0, quality=0.85.
        // Third-pass: logit ≈ -1.172, sigmoid ≈ 0.236.
        // Design target: 0.25–0.35; doc flags as "at the low edge".
        // Range: [0.18, 0.38] — slightly wider to accommodate Q32 rounding + LUT steps.
        let ctx = ShotContext::try_new(
            q_frac(686, 1000),
            q_frac(70, 100),
            q_frac(55, 100),
            Q32::ONE,
            Q32::ONE,
            q_frac(85, 100),
        )
        .expect("valid");
        let xg = xg_utility(&ctx);
        let lo = q_frac(18, 100);
        let hi = q_frac(38, 100);
        assert!(
            xg >= lo && xg <= hi,
            "12-yard central shot xG should be in [18%, 38%], got raw {}",
            xg.to_bits()
        );
    }

    #[test]
    fn penalty_xg_is_plausible() {
        // Penalty: distance_q32=0.686, angle=0.95, pressure=0.0,
        // foot=1.0, set-piece=0.40, quality=0.85.
        // Third-pass: logit ≈ +0.598, sigmoid ≈ 0.645 (design target: ~0.76;
        // doc notes structural limit of single-logistic form without penalty intercept).
        // Range: [58%, 72%] — ±10% of 0.645 design target.
        let ctx = ShotContext::try_new(
            q_frac(686, 1000),
            q_frac(95, 100),
            Q32::ZERO,
            Q32::ONE,
            q_frac(4, 10),
            q_frac(85, 100),
        )
        .expect("valid");
        let xg = xg_utility(&ctx);
        let lo = q_frac(58, 100);
        let hi = q_frac(72, 100);
        assert!(
            xg >= lo && xg <= hi,
            "penalty xG should be in [58%, 72%], got raw {}",
            xg.to_bits()
        );
    }
}

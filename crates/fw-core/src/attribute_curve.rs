//! Non-linear attribute-effect curve — the deterministic Q32 map that makes
//! elite attribute values disproportionately impactful.
//!
//! # Why this exists (Slice 0 of the attribute-effect mandate)
//!
//! Every attribute→effect formula in `fw-match-sim` was LINEAR in the stored
//! attribute: `19→20` bought the same effect delta as `11→12`, so elite was
//! never visibly elite. This module supplies `g_class(a)` — a per-class convex
//! power curve `g(a) = a^γ` (γ > 1) applied at the **effect-magnitude**
//! boundary (when a stored attribute becomes a multiplier / contest weight /
//! physics coefficient / xG feature scale). The stored canonical attribute is
//! never touched; only its value→effect mapping is reshaped.
//!
//! Design contract: `docs/design/attribute-effect-audit-2026-06-06.md` §3.
//!
//! # Shape — gamma power curve, per effect class (§3.2 / §3.3)
//!
//! `g(a) = a^γ`, monotone strictly increasing, `g(0)=0`, `g(1)=1`, with the
//! marginal gain rising toward the top so `g(1.0)-g(0.9) > g(0.6)-g(0.5)`.
//! Five effect classes carry different γ:
//!
//! | Class           | γ    | Role                                              |
//! |-----------------|------|---------------------------------------------------|
//! | Skill           | 1.7  | finishing, passing, dribbling, technique, vision… |
//! | Physical        | 1.4  | pace→v_max, strength→shot speed (physics-capped)  |
//! | Contest / duel  | 1.8  | tackling, marking, balance, jumping, one_on_ones  |
//! | Mental          | 1.6  | composure, anticipation, positioning, decisions   |
//! | Personality     | 1.3  | work_rate, determination, aggression (near-linear)|
//!
//! The compounding of two curved factors in a utility product is the
//! "combinatorial" payoff: a player elite in two combining attributes gets a
//! super-linear joint edge (a 4-factor skill product skews ~23x at the elite
//! end vs an equal mid step, against ~5x for the old linear product).
//!
//! # Determinism (Sim/RULES.md §1, §7)
//!
//! - Pure function of the stored Q32 attribute — same input, same Q32 output,
//!   every platform. No `f32`/`f64` in the runtime path.
//! - 257-entry committed const `[Q32; 257]` LUT per γ (256 segments + endpoint)
//!   over the domain `[0, 1]`, with linear interpolation between entries. The
//!   committed raw `i64` bits live in `attribute_curve_luts.rs`; a const-fn
//!   helper maps them into `Q32` at compile time. No runtime bake, no CORDIC
//!   on the per-tick path — exactly the `math.rs` LUT pattern from T1-10.
//! - The bake source is the f64 reference `a^γ`, exercised by the
//!   drift-detection test in `crates/fw-core/tests/`; regeneration is a
//!   code-review artifact (the committed bytes change), never a runtime
//!   variance.

use crate::Q32;
use crate::attribute_curve_luts;

/// Number of LUT entries: 257 gives 256 uniform segments over `[0, 1]`,
/// step `1/256`.
pub(crate) const CURVE_LUT_N: usize = 257;

/// Effect class selecting the gamma exponent for the non-linear curve.
///
/// Per `docs/design/attribute-effect-audit-2026-06-06.md` §3.3. The class is
/// chosen at the read site by what the attribute *does* (skill expression vs
/// physics ceiling vs winner-take-all duel vs decision quality vs personality
/// tendency), not by which attribute family it lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveClass {
    /// Skill expression (γ = 1.7) — finishing, technique, dribbling, passing,
    /// crossing, vision, first_touch.
    Skill,
    /// Physical ceiling (γ = 1.4) — pace→v_max, strength→shot speed,
    /// acceleration. Physics caps the spread; a gentler curve keeps the band
    /// realistic.
    Physical,
    /// Contest / duel (γ = 1.8) — tackling, marking, balance, jumping_reach,
    /// heading, one_on_ones, aerial_reach. Winner-take-all moments.
    Contest,
    /// Mental composite (γ = 1.6) — composure, anticipation, positioning,
    /// decisions, concentration.
    Mental,
    /// Personality bias (γ = 1.3, near-linear) — work_rate, determination,
    /// aggression, risk_appetite, etc. Tendency dials, not quality.
    Personality,
}

// -------------------------------------------------------------------------
// Const helper: map raw i64 bits -> Q32 array at compile time
// (mirrors math.rs::q32_array_from_raw)
// -------------------------------------------------------------------------

const fn q32_array_from_raw(raw: &[i64; CURVE_LUT_N]) -> [Q32; CURVE_LUT_N] {
    let mut out = [Q32::ZERO; CURVE_LUT_N];
    let mut i = 0;
    while i < CURVE_LUT_N {
        out[i] = Q32::from_raw(raw[i]);
        i += 1;
    }
    out
}

const SKILL_LUT: [Q32; CURVE_LUT_N] = q32_array_from_raw(&attribute_curve_luts::SKILL_LUT_RAW);
const PHYSICAL_LUT: [Q32; CURVE_LUT_N] =
    q32_array_from_raw(&attribute_curve_luts::PHYSICAL_LUT_RAW);
const CONTEST_LUT: [Q32; CURVE_LUT_N] = q32_array_from_raw(&attribute_curve_luts::CONTEST_LUT_RAW);
const MENTAL_LUT: [Q32; CURVE_LUT_N] = q32_array_from_raw(&attribute_curve_luts::MENTAL_LUT_RAW);
const PERSONALITY_LUT: [Q32; CURVE_LUT_N] =
    q32_array_from_raw(&attribute_curve_luts::PERSONALITY_LUT_RAW);

// -------------------------------------------------------------------------
// Public API
// -------------------------------------------------------------------------

/// Apply the non-linear effect curve for the given `class` to a stored
/// attribute `a` in `[0, 1]`.
///
/// Returns `g_class(a) = a^γ` as Q32 in `[0, 1]`, monotone strictly increasing,
/// with `g(0)=0`, `g(1)=1`. Values outside `[0, 1]` saturate to the boundary
/// entry (the same clamp convention as `math.rs`). Pure Q32 — no float, no RNG,
/// no clock.
///
/// This is the single entry-point every sim read site calls to reshape an
/// attribute's value→effect mapping. Do NOT apply it twice to the same value
/// (double-curving is the documented foot-gun — each stored attribute is curved
/// exactly once at its read site).
#[must_use]
#[inline]
pub fn curve(class: CurveClass, a: Q32) -> Q32 {
    let lut = match class {
        CurveClass::Skill => &SKILL_LUT,
        CurveClass::Physical => &PHYSICAL_LUT,
        CurveClass::Contest => &CONTEST_LUT,
        CurveClass::Mental => &MENTAL_LUT,
        CurveClass::Personality => &PERSONALITY_LUT,
    };
    lut_eval(lut, a)
}

// -------------------------------------------------------------------------
// Internal interpolation core (mirrors math.rs::lut_eval, domain [0, 1])
// -------------------------------------------------------------------------

/// Linear interpolation of a `[0, 1]`-domain curve LUT at position `a`.
///
/// The LUT covers `[0, 1]` uniformly in 256 segments (step `1/256`). The
/// integer part of `a * 256` gives the lower index; the fractional part is the
/// interpolation weight `t`. Result = `lo + t * (hi - lo)`.
///
/// # Overflow analysis
///
/// All LUT values are in `[0, 1]`, so `hi - lo` ∈ `[-1, 1]` and `t * delta`
/// ∈ `[-1, 1]`. The `a * 256` scaling: `a ∈ [0, 1)` here (the `a >= ONE`
/// branch returns early), so `scaled ∈ [0, 256)` — fits exactly with no
/// rounding. Bare operators panic on overflow per the house policy; the
/// bounds above make overflow unreachable.
fn lut_eval(lut: &[Q32; CURVE_LUT_N], a: Q32) -> Q32 {
    // Saturate below / above the [0, 1] domain.
    if a <= Q32::ZERO {
        return lut[0];
    }
    if a >= Q32::ONE {
        return lut[CURVE_LUT_N - 1];
    }

    // scaled = a * 256 maps [0, 1) -> [0, 256) exactly in Q32 (no rounding,
    // since 256 is a power of two — the multiply is a left shift of the
    // integer bits).
    const SCALE_256: Q32 = Q32::from_raw(256_i64 << 32);
    let scaled = a * SCALE_256;

    let scaled_bits = scaled.to_bits();
    let lo_idx = (scaled_bits >> 32) as usize; // exact for scaled ∈ [0, 256)
    let hi_idx = (lo_idx + 1).min(CURVE_LUT_N - 1);

    // Fractional part = interpolation weight t (lower 32 bits as Q32 in [0, 1)).
    let t = Q32::from_raw(scaled_bits & 0xFFFF_FFFF);

    let lo = lut[lo_idx];
    let hi = lut[hi_idx];
    lo + t * (hi - lo)
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_CLASSES: [CurveClass; 5] = [
        CurveClass::Skill,
        CurveClass::Physical,
        CurveClass::Contest,
        CurveClass::Mental,
        CurveClass::Personality,
    ];

    /// `n / d` as Q32 (raw = `(n << 32) / d`) — pure integer, no float (the
    /// crate's `float_arithmetic = deny` lint applies to tests too).
    fn qf(n: i64, d: i64) -> Q32 {
        Q32::from_raw((n << 32) / d)
    }

    #[test]
    fn endpoints_are_zero_and_one() {
        for c in ALL_CLASSES {
            assert_eq!(curve(c, Q32::ZERO), Q32::ZERO, "g(0) must be 0 for {c:?}");
            // g(1) == 1 within 2 ULP (the LUT endpoint is from_f64_clamped(1.0)).
            let g1 = curve(c, Q32::ONE);
            let diff = (g1.to_bits() - Q32::ONE.to_bits()).abs();
            assert!(
                diff <= 2,
                "g(1) must be ~1 for {c:?}; got bits {}",
                g1.to_bits()
            );
        }
    }

    #[test]
    fn saturates_outside_unit_range() {
        for c in ALL_CLASSES {
            assert_eq!(curve(c, Q32::from_raw(-1)), curve(c, Q32::ZERO));
            assert_eq!(curve(c, Q32::from_int(2)), curve(c, Q32::ONE));
        }
    }

    #[test]
    fn is_monotone_non_decreasing_dense() {
        // 257 sample points across [0, 1]; each step must not decrease.
        for c in ALL_CLASSES {
            let mut prev = curve(c, Q32::ZERO);
            for i in 1i64..=256 {
                let a = Q32::from_raw((i << 32) / 256);
                let g = curve(c, a);
                assert!(
                    g >= prev,
                    "{c:?} not monotone at i={i}: {:?} < {:?}",
                    g,
                    prev
                );
                prev = g;
            }
        }
    }

    #[test]
    fn is_convex_marginal_increasing_at_top() {
        // The mandate: the elite-end marginal delta must exceed the mid delta.
        // g(1.0)-g(0.9) > g(0.6)-g(0.5) for the skewing classes.
        for c in [
            CurveClass::Skill,
            CurveClass::Physical,
            CurveClass::Contest,
            CurveClass::Mental,
            CurveClass::Personality,
        ] {
            let d_hi = curve(c, Q32::ONE) - curve(c, qf(9, 10));
            let d_mid = curve(c, qf(6, 10)) - curve(c, qf(5, 10));
            assert!(
                d_hi > d_mid,
                "{c:?}: elite marginal delta ({d_hi:?}) must exceed mid delta ({d_mid:?})"
            );
        }
    }

    #[test]
    fn mid_is_near_linear_midpoint_and_top_is_disproportionate() {
        // §3.2: γ=1.7 skill g(0.5) ≈ 0.31 (below the linear 0.5 midpoint),
        // g(1.0) = 1.0 (the disproportionate top). Confirm the curve pulls the
        // mid down while pinning the top.
        let g_mid = curve(CurveClass::Skill, qf(5, 10));
        let g_top = curve(CurveClass::Skill, Q32::ONE);
        // g(0.5) for γ=1.7 ≈ 0.3078; assert it sits in [0.29, 0.33].
        assert!(
            g_mid >= qf(29, 100) && g_mid <= qf(33, 100),
            "skill g(0.5) ≈ 0.31 expected; got {g_mid:?}"
        );
        assert!(
            g_top >= qf(99, 100),
            "skill g(1.0) ≈ 1.0 expected; got {g_top:?}"
        );
    }

    #[test]
    fn personality_is_gentler_than_skill() {
        // γ=1.3 (personality) is closer to linear than γ=1.7 (skill): at the
        // mid, personality g(0.5) > skill g(0.5) (less suppression).
        let p = curve(CurveClass::Personality, qf(5, 10));
        let s = curve(CurveClass::Skill, qf(5, 10));
        assert!(
            p > s,
            "personality (near-linear) must suppress the mid less than skill: {p:?} vs {s:?}"
        );
    }
}

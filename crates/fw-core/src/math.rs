//! Utility math primitives — LUT-based sigmoid and exp over Q32.
//!
//! Both functions are backed by a 257-entry symmetric LUT over the domain
//! [-8, +8] with linear interpolation between entries.  Values outside the
//! domain are clamped (saturated) to the appropriate limit.
//!
//! # T1-10 (Codex 2026-05-16 audit P1 closure)
//!
//! The LUTs are committed `const [Q32; 257]` arrays — NO runtime bake.
//! The raw `i64` bits live in `math_luts.rs` (committed source); a
//! const-fn helper maps them into `Q32` at compile time. The prior design
//! built the LUTs at process startup via `LazyLock` using `f64::exp()`,
//! then quantized to Q32 — a libm/platform-dependent bake step that could
//! silently drift on a libc change. T1-10 moves the bake out of the
//! determinism critical chain entirely; drift becomes a code-review
//! artifact (the committed `i64` bytes change) rather than a runtime
//! variance. See `math_luts.rs` for the regeneration procedure.
//!
//! The per-tick interpolation path is pure Q32 — no f64 anywhere in
//! production code paths.
//!
//! This satisfies ADR-0003 §1 (per-tick arithmetic bit-exact across all
//! platforms) AND `Sim/RULES.md` §1 (no floats in canonical state — now
//! genuinely zero f64 in the runtime, even at process startup).
//!
//! # Domain choice
//! sigmoid(-8) approx 3.35e-4; sigmoid(+8) approx 9.997e-1.  For any
//! reasonable feature value in the match sim those limits are safe clamps.
//! exp(-8) approx 3.35e-4; exp(+8) approx 2981.  The exp LUT covers the
//! same range; exp is only used after a prior sigmoid or within a bounded
//! softmax window.

use crate::Q32;
use crate::math_luts;

// -------------------------------------------------------------------------
// LUT parameters
// -------------------------------------------------------------------------

/// Number of LUT entries.  257 gives step size exactly (16 / 256) = 1/16 = 0.0625.
const LUT_N: usize = 257;

/// Maximum Q32 value covered by the LUT: +8.  Outside this range the function
/// saturates to the boundary entry.
const LUT_MAX: Q32 = Q32::from_raw(8i64 << 32);

/// Minimum Q32 value covered by the LUT: -8.
const LUT_MIN: Q32 = Q32::from_raw(-8i64 << 32);

/// Reciprocal of stride in Q32 (= 16).  Used to convert `x` into a fractional
/// index: `scaled = (x - LUT_MIN) / stride = (x + 8) * 16`.
const LUT_STRIDE_INV: Q32 = Q32::from_raw(16i64 << 32); // 16.0 exactly

// -------------------------------------------------------------------------
// Const helper: map raw i64 bits -> Q32 array at compile time
// -------------------------------------------------------------------------

/// Convert a `[i64; N]` of Q32 raw bits into a `[Q32; N]` at compile time.
///
/// T1-10: the committed LUTs in `math_luts.rs` are stored as `[i64; 257]`
/// (pure data, no fw-core type dependency). This helper rebuilds them as
/// `[Q32; 257]` in `const` context so the public `SIGMOID_LUT` / `EXP_LUT`
/// stay strongly typed without paying a runtime cost.
const fn q32_array_from_raw(raw: &[i64; LUT_N]) -> [Q32; LUT_N] {
    let mut out = [Q32::ZERO; LUT_N];
    let mut i = 0;
    while i < LUT_N {
        out[i] = Q32::from_raw(raw[i]);
        i += 1;
    }
    out
}

// -------------------------------------------------------------------------
// Sigmoid + Exp LUTs (committed const, T1-10)
// -------------------------------------------------------------------------

/// 257-entry sigmoid LUT over [-8, +8].  Entry i corresponds to x = -8 + i * step.
///
/// Backed by committed const raw-bits in `math_luts.rs`. No runtime bake.
///
/// Module-private — only the `lut_eval` helper below reads it. T1-10
/// type-design audit P2-1: prior `pub(crate)` exposed it to every fw-core
/// module (player_attributes, seed, ids, etc.) that has no business
/// reading raw LUT tables. The prior `static LazyLock` was also
/// file-scoped (implicitly private); tightening to module-private matches
/// the prior surface + reduces accidental coupling.
const SIGMOID_LUT: [Q32; LUT_N] = q32_array_from_raw(&math_luts::SIGMOID_LUT_RAW);

/// 257-entry exp LUT over [-8, +8].  Entry i corresponds to x = -8 + i * step.
/// exp(+8) approx 2981 fits comfortably in Q32 (max integer ~2.15e9).
///
/// Backed by committed const raw-bits in `math_luts.rs`. No runtime bake.
/// Module-private (see SIGMOID_LUT comment).
const EXP_LUT: [Q32; LUT_N] = q32_array_from_raw(&math_luts::EXP_LUT_RAW);

// -------------------------------------------------------------------------
// Public API
// -------------------------------------------------------------------------

/// Logistic sigmoid via LUT + linear interpolation.
///
/// - Domain: [-8, +8]; saturates to approx 0 below, approx 1 above.
/// - `sigmoid_q32(Q32::ZERO)` returns exactly `Q32::from_raw(1i64 << 31)` (= 0.5).
///   Guaranteed by the symmetric LUT construction: the centre entry (i=128)
///   is computed at x=0.0; sigmoid(0)=0.5 exactly in f64; and
///   `Q32::from_f64_clamped(0.5)` always produces `Q32::from_raw(1i64 << 31)`.
pub fn sigmoid_q32(x: Q32) -> Q32 {
    lut_eval(&SIGMOID_LUT, x)
}

/// Natural exponential via LUT + linear interpolation.
///
/// - Domain: [-8, +8]; saturates to approx exp(-8) below, approx exp(+8) above.
/// - Always returns a positive (non-negative) value.
pub fn exp_q32(x: Q32) -> Q32 {
    lut_eval(&EXP_LUT, x)
}

// -------------------------------------------------------------------------
// Internal interpolation core
// -------------------------------------------------------------------------

/// Interpolate a LUT at position `x` using linear interpolation.
///
/// The LUT covers `[LUT_MIN, LUT_MAX]` = `[-8, +8]` uniformly.
/// Outside this domain the function saturates to the boundary entry.
///
/// # Pure Q32 — no f64 at per-tick call sites
///
/// The domain span is 16 and the step is 1/16, so:
///
///   scaled = (x + 8) * 16       (Q32 arithmetic, exact — no rounding)
///
/// The integer part of `scaled` (bits 63..32) gives the lower LUT index.
/// The fractional part (bits 31..0) is `t`, the interpolation weight in [0, 1).
/// Linear interpolation: `lo + t * (hi - lo)`.
///
/// Overflow analysis:
///   - For sigmoid: `hi - lo` ≤ 1 (LUT values in [0,1]); `t * delta` ≤ 1. No overflow.
///   - For exp: `hi - lo` ≤ exp(8) ≈ 2981; `t * delta` ≤ 2981 ≪ Q32::MAX (~2.15e9).
///
/// Bare `+` / `-` / `*` operators panic on overflow per Codex Q1. If the
/// invariants above hold they cannot overflow; if they are violated the panic
/// surfaces the bug immediately.
fn lut_eval(lut: &[Q32; LUT_N], x: Q32) -> Q32 {
    // Saturate below / above domain.
    if x <= LUT_MIN {
        return lut[0];
    }
    if x >= LUT_MAX {
        return lut[LUT_N - 1];
    }

    // Shift x up by 8 so the domain starts at 0, then multiply by 16 (= 1/stride).
    // `(x + 8) * 16` maps [-8, +8] → [0, 256] exactly in Q32 — no rounding.
    let shifted = x + Q32::from_int(8); // x ∈ (-8, 8), so shifted ∈ (0, 16) — no overflow
    let scaled = shifted * LUT_STRIDE_INV; // scaled ∈ (0, 256) — no overflow

    // Integer part of scaled = lower LUT index (bits 63..32 of the i64 backing value).
    // Fractional part = interpolation weight t (bits 31..0, interpreted as Q32 in [0, 1)).
    let scaled_bits = scaled.to_bits();
    let lo_idx = (scaled_bits >> 32) as usize; // exact for scaled ∈ [0, 256)
    let hi_idx = (lo_idx + 1).min(LUT_N - 1);

    // t is the fractional part: isolate lower 32 bits and reinterpret as Q32.
    let t = Q32::from_raw(scaled_bits & 0xFFFF_FFFF);

    let lo = lut[lo_idx];
    let hi = lut[hi_idx];

    // linear interpolation: lo + t * (hi - lo)
    // hi - lo may be negative (all decreasing LUTs go right) but Q32 sub handles sign.
    lo + t * (hi - lo)
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- sigmoid tests ---

    #[test]
    fn sigmoid_zero_is_exactly_half() {
        let half = Q32::from_raw(1i64 << 31);
        assert_eq!(
            sigmoid_q32(Q32::ZERO),
            half,
            "sigmoid(0) must be exactly 0.5"
        );
    }

    #[test]
    fn sigmoid_is_monotone_increasing() {
        // 65 evenly-spaced points over [-8, +8].
        let mut prev = sigmoid_q32(Q32::from_raw(-(8i64 << 32)));
        for i in 1i32..=64 {
            let x_raw = -(8i64 << 32) + (i as i64) * (1i64 << 28); // steps of 0.25
            let s = sigmoid_q32(Q32::from_raw(x_raw));
            assert!(
                s >= prev,
                "sigmoid not monotone at step {i}: {s:?} < {prev:?}"
            );
            prev = s;
        }
    }

    #[test]
    fn sigmoid_bounds() {
        // sigmoid(-8) should be very small but > 0.
        let neg8 = Q32::from_raw(-(8i64 << 32));
        let s_neg8 = sigmoid_q32(neg8);
        assert!(s_neg8 > Q32::ZERO, "sigmoid(-8) must be positive");
        // sigmoid(-8) approx 3.35e-4; in Q32 raw that is approx 1_440_000.
        // Must be less than 2M (2^21 = 2_097_152) to confirm it's tiny.
        assert!(
            s_neg8.to_raw() < (1i64 << 21),
            "sigmoid(-8) should be very small, got raw={}",
            s_neg8.to_raw()
        );

        // sigmoid(+8) should be close to 1 but <= 1.
        let pos8 = Q32::from_raw(8i64 << 32);
        let s_pos8 = sigmoid_q32(pos8);
        let one = Q32::from_raw(1i64 << 32);
        assert!(s_pos8 <= one, "sigmoid(+8) must be <= 1");
        // sigmoid(+8) approx 0.9997; in Q32 raw that is approx 4_293_527_000.
        // Check it is greater than 0.999 (raw approx 4_290_672_000).
        assert!(
            s_pos8.to_raw() > 4_290_000_000,
            "sigmoid(+8) should be close to 1, got raw={}",
            s_pos8.to_raw()
        );
    }

    #[test]
    fn sigmoid_saturates_below_domain() {
        let boundary = sigmoid_q32(Q32::from_raw(-(8i64 << 32)));
        let very_neg = sigmoid_q32(Q32::from_raw(-(100i64 << 32)));
        assert_eq!(
            boundary, very_neg,
            "below domain should saturate to sigmoid(-8)"
        );
    }

    #[test]
    fn sigmoid_saturates_above_domain() {
        let boundary = sigmoid_q32(Q32::from_raw(8i64 << 32));
        let very_pos = sigmoid_q32(Q32::from_raw(100i64 << 32));
        assert_eq!(
            boundary, very_pos,
            "above domain should saturate to sigmoid(+8)"
        );
    }

    #[test]
    fn sigmoid_symmetry() {
        // sigmoid(x) + sigmoid(-x) should sum to approx 1 within 2 ULP.
        for i in 1i64..=32 {
            let x = Q32::from_raw(i * (1i64 << 27)); // multiples of 0.125
            let sx = sigmoid_q32(x);
            let snx = sigmoid_q32(Q32::from_raw(-x.to_raw()));
            let sum = sx.to_raw() + snx.to_raw();
            let one_raw = 1i64 << 32;
            let diff = (sum - one_raw).abs();
            assert!(
                diff <= 2,
                "sigmoid symmetry violated at i={i}: sum raw diff from 1 is {diff}"
            );
        }
    }

    // --- exp tests ---

    #[test]
    fn exp_zero_is_one() {
        let e0 = exp_q32(Q32::ZERO);
        let one_raw = 1i64 << 32;
        let diff = (e0.to_raw() - one_raw).abs();
        assert!(diff <= 2, "exp(0) should be 1, got raw {}", e0.to_raw());
    }

    #[test]
    fn exp_is_positive() {
        for i in -32i32..=32 {
            let x = Q32::from_raw((i as i64) * (1i64 << 28)); // steps of 0.25
            let e = exp_q32(x);
            assert!(e > Q32::ZERO, "exp({i}/4) must be positive, got {:?}", e);
        }
    }

    #[test]
    fn exp_is_monotone_increasing() {
        let mut prev = exp_q32(Q32::from_raw(-(8i64 << 32)));
        for i in 1i32..=64 {
            let x_raw = -(8i64 << 32) + (i as i64) * (1i64 << 28);
            let e = exp_q32(Q32::from_raw(x_raw));
            assert!(e >= prev, "exp not monotone at step {i}");
            prev = e;
        }
    }

    #[test]
    fn exp_saturates_outside_domain() {
        let lo = exp_q32(Q32::from_raw(-(8i64 << 32)));
        let very_lo = exp_q32(Q32::from_raw(-(200i64 << 32)));
        assert_eq!(lo, very_lo, "exp saturates below domain");

        let hi = exp_q32(Q32::from_raw(8i64 << 32));
        let very_hi = exp_q32(Q32::from_raw(200i64 << 32));
        assert_eq!(hi, very_hi, "exp saturates above domain");
    }
}

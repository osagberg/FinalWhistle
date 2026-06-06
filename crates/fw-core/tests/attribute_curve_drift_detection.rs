//! Drift-detection test for the committed attribute-effect curve LUTs in
//! `crates/fw-core/src/attribute_curve_luts.rs`.
//!
//! Re-bakes each of the five per-class gamma power-curve LUTs via the f64
//! reference `g(a) = a^γ` and asserts each entry equals the committed const
//! raw bits bit-for-bit. If a future libm/toolchain change drifts `f64::powf`
//! at any of the 257 grid points, this test fails with a diagnostic naming the
//! class, index, input `a`, committed/re-baked raw bits, and the diff.
//!
//! `#[ignore]`-gated: the default test run uses the committed const path which
//! is bit-exact by construction; re-baking via f64 every run would reintroduce
//! the float dependency the committed LUT removed.
//!
//! Run:
//! ```text
//! cargo test --package fw-core --test attribute_curve_drift_detection -- --ignored --nocapture
//! ```
//!
//! On failure: either libm changed (regenerate via the printer test +
//! rebaseline canonical hash) or `attribute_curve_luts.rs` was hand-edited
//! wrong (revert).

use fw_core::{CurveClass, Q32, curve};

const LUT_N: usize = 257;

const CLASSES: [(CurveClass, &str, f64); 5] = [
    (CurveClass::Skill, "SKILL", 1.7),
    (CurveClass::Physical, "PHYSICAL", 1.4),
    (CurveClass::Contest, "CONTEST", 1.8),
    (CurveClass::Mental, "MENTAL", 1.6),
    (CurveClass::Personality, "PERSONALITY", 1.3),
];

/// f64 input `a = i / 256` for LUT entry `i`.
#[allow(clippy::float_arithmetic)] // bake-time-only reference; not in production
fn lut_a_f64(i: usize) -> f64 {
    (i as f64) / ((LUT_N - 1) as f64)
}

/// Grid x in Q32 for entry `i`: `a = i / 256`. At grid points the interpolation
/// weight is zero, so `curve(class, a)` returns the committed LUT entry directly
/// (no interpolation), which is equivalent to indexing the LUT without exposing
/// it as `pub`.
fn grid_a_q32(i: usize) -> Q32 {
    // i / 256 in Q32 raw bits: (i << 32) / 256 = i << 24.
    Q32::from_raw((i as i64) << 24)
}

#[test]
#[ignore]
#[allow(clippy::float_arithmetic)] // bake-time-only reference
fn curve_luts_match_f64_reference_bake() {
    for (class, name, gamma) in CLASSES {
        for i in 0..LUT_N {
            let a = lut_a_f64(i);
            let v = a.powf(gamma).clamp(0.0, 1.0);
            let expected_raw = Q32::from_f64_clamped(v).to_raw();
            let committed_raw = curve(class, grid_a_q32(i)).to_raw();
            assert_eq!(
                committed_raw,
                expected_raw,
                "{name}_LUT drift at i={i} (a={a}, γ={gamma}): committed raw={committed_raw}, \
                 re-baked raw={expected_raw}, diff={}. If libm/toolchain changed, regenerate \
                 attribute_curve_luts.rs via the printer test + rebaseline canonical hash.",
                committed_raw - expected_raw,
            );
        }
    }
}

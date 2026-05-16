//! T1-10 drift-detection test for the committed `SIGMOID_LUT` + `EXP_LUT`
//! in `crates/fw-core/src/math_luts.rs`.
//!
//! # What this test does
//!
//! Re-bakes both 257-entry LUTs via the original f64 formulas the prior
//! `LazyLock` closures used (sigmoid: `1.0 / (1.0 + (-x).exp())`; exp:
//! `x.exp()`) and asserts each entry equals the committed const raw bits
//! bit-for-bit. If a future libc/glibc/macOS-libsystem change drifts
//! `f64::exp()`'s ULP behavior at any of the 257 LUT points, this test
//! fails with a diagnostic naming the index, the f64 input `x`, the
//! committed (expected) raw bits, the just-baked (actual) raw bits, and
//! the bit-diff.
//!
//! # When to run
//!
//! - Before bumping the workspace `rust-toolchain.toml` MSRV
//! - After any system upgrade that might touch libm (glibc / libsystem)
//! - As part of a periodic determinism sanity sweep
//!
//! # Why `#[ignore]`-gated
//!
//! The default test run uses the committed const path which is bit-exact
//! by construction. Re-baking via f64 every test run would defeat the
//! purpose of T1-10 (removing f64 from the production critical chain).
//! This test exists purely as a regeneration-correctness guard; explicit
//! `cargo test -- --ignored` invocation only.
//!
//! Run:
//! ```text
//! cargo test --package fw-core --test lut_drift_detection -- --ignored --nocapture
//! ```
//!
//! # Failure mode
//!
//! If this fails, the committed values in `math_luts.rs` no longer match
//! the f64 reference on this toolchain. Two possible causes:
//! 1. libm changed (or rustc lowering of `f64::exp` changed). Likely needs
//!    a coordinated regeneration of `math_luts.rs` via the printer test
//!    AND an ADR-0012 canonical-hash rebaseline (the LUT values feed
//!    every utility-scored BT decision).
//! 2. `math_luts.rs` was edited by hand and the edit was wrong. Revert
//!    the edit + re-run.
//!
//! See `crates/fw-core/src/math_luts.rs` header for the regeneration
//! procedure.

use fw_core::Q32;
use fw_core::math::{exp_q32, sigmoid_q32};

const LUT_N: usize = 257;

/// Compute the f64 x-coordinate for LUT entry `i`.
///
/// Mirrors the prior `LazyLock` closures' index→x mapping:
///   `x = -DOMAIN_MAX_F64 + (i as f64) * STEP_F64`
/// where `DOMAIN_MAX_F64 = 8.0` and `STEP_F64 = (2.0 * 8.0) / 256.0 = 1.0 / 16.0`.
#[allow(clippy::float_arithmetic)] // bake-time-only reference; not in production
fn lut_x_f64(i: usize) -> f64 {
    const DOMAIN_MAX: f64 = 8.0;
    const STEP: f64 = (2.0 * DOMAIN_MAX) / ((LUT_N - 1) as f64);
    -DOMAIN_MAX + (i as f64) * STEP
}

// Q32 conversion: use `Q32::from_f64_clamped` (T1-10 promoted to `pub` so
// integration tests can re-bake against the canonical rounding mode —
// round-to-nearest-even via `fixed::FixedI64::saturating_from_num`).
// Open-coding the f64→Q32 conversion in the test would silently diverge on
// 1-bit boundary cases (truncation vs round-to-nearest), defeating the
// purpose of the drift detector.

/// Read the committed const SIGMOID_LUT entry `i` via the public surface.
///
/// At the exact LUT grid points (`x = LUT_MIN + i * stride`), `sigmoid_q32`
/// returns the LUT entry directly (zero interpolation weight). Same for
/// `exp_q32`. So calling the public function with the grid x is equivalent
/// to indexing the LUT — without exposing the LUT as `pub`.
fn read_committed_sigmoid(i: usize) -> Q32 {
    let raw = -(8i64 << 32) + (i as i64) * (1i64 << 28); // x = -8 + i * (1/16) in Q32
    sigmoid_q32(Q32::from_raw(raw))
}

fn read_committed_exp(i: usize) -> Q32 {
    let raw = -(8i64 << 32) + (i as i64) * (1i64 << 28);
    exp_q32(Q32::from_raw(raw))
}

#[test]
#[ignore]
#[allow(clippy::float_arithmetic)] // bake-time-only reference
fn sigmoid_lut_matches_f64_reference_bake() {
    for i in 0..LUT_N {
        let x = lut_x_f64(i);
        let s_f64: f64 = 1.0 / (1.0 + (-x).exp());
        let s_clamped = s_f64.clamp(0.0, 1.0);
        let expected_raw = Q32::from_f64_clamped(s_clamped).to_raw();
        let committed_raw = read_committed_sigmoid(i).to_raw();
        assert_eq!(
            committed_raw,
            expected_raw,
            "SIGMOID_LUT drift at i={i} (x={x}): committed raw={committed_raw}, re-baked raw={expected_raw}, diff={}. \
             If libm/toolchain changed, regenerate math_luts.rs via the printer test + rebaseline canonical hash per ADR-0012.",
            committed_raw - expected_raw,
        );
    }
}

#[test]
#[ignore]
#[allow(clippy::float_arithmetic)] // bake-time-only reference
fn exp_lut_matches_f64_reference_bake() {
    for i in 0..LUT_N {
        let x = lut_x_f64(i);
        let e_f64: f64 = x.exp();
        let expected_raw = Q32::from_f64_clamped(e_f64).to_raw();
        let committed_raw = read_committed_exp(i).to_raw();
        assert_eq!(
            committed_raw,
            expected_raw,
            "EXP_LUT drift at i={i} (x={x}): committed raw={committed_raw}, re-baked raw={expected_raw}, diff={}. \
             If libm/toolchain changed, regenerate math_luts.rs via the printer test + rebaseline canonical hash per ADR-0012.",
            committed_raw - expected_raw,
        );
    }
}

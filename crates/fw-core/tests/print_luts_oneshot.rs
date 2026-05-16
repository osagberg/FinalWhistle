//! T1-10 regeneration tool — re-bakes `SIGMOID_LUT` + `EXP_LUT` from the
//! f64 reference formulas and prints the result as Rust source literals
//! suitable for paste into `src/math_luts.rs`.
//!
//! # When to run
//!
//! Only when `lut_drift_detection` fails (libm/toolchain change) AND a
//! coordinated regeneration is authorized per ADR-0012 (will rebaseline
//! the canonical hash since math LUTs feed every utility-scored BT
//! decision).
//!
//! ```text
//! cargo test --package fw-core --test print_luts_oneshot -- --ignored --nocapture --test-threads=1
//! ```
//!
//! Capture stdout, extract the two `pub(crate) const ... = [ ... ];`
//! blocks, paste-replace the corresponding blocks in `src/math_luts.rs`.
//! Re-run `lut_drift_detection -- --ignored` to confirm.
//!
//! # T1-10 Tier-2 fix-pass: re-bake from f64, NOT echo committed const
//!
//! Both silent-failure-hunter + code-reviewer independently flagged that
//! the prior printer called `sigmoid_q32(lut_entry_x(i))` — which reads
//! from the committed `const SIGMOID_LUT` and prints it back out. After
//! a real drift event the prior printer would emit the STALE committed
//! values, not the new f64-baked truth — making the documented
//! "regeneration procedure" a workflow trap. This rewrite shares the
//! exact same f64-bake path as `lut_drift_detection.rs` so the printer
//! IS the regeneration source of truth on the current toolchain.
//!
//! # Why `#[ignore]`-gated
//!
//! The default test run doesn't need 514 i64 literals printed every time.
//! Explicit `cargo test -- --ignored` invocation only.

use fw_core::Q32;

const LUT_N: usize = 257;

/// Compute the f64 x-coordinate for LUT entry `i`.
///
/// Mirrors `lut_drift_detection::lut_x_f64` exactly (intentionally
/// duplicated rather than imported — integration tests don't share a
/// crate, and re-defining the formula here keeps the printer
/// self-contained for the regeneration workflow).
///
/// `x = -DOMAIN_MAX_F64 + (i as f64) * STEP_F64` where
/// `DOMAIN_MAX_F64 = 8.0` and `STEP_F64 = (2.0 * 8.0) / 256.0 = 1.0 / 16.0`.
#[allow(clippy::float_arithmetic)]
fn lut_x_f64(i: usize) -> f64 {
    const DOMAIN_MAX: f64 = 8.0;
    const STEP: f64 = (2.0 * DOMAIN_MAX) / ((LUT_N - 1) as f64);
    -DOMAIN_MAX + (i as f64) * STEP
}

/// Format a single i64 raw-bits value as a `<value>_i64` literal, with
/// the 4-per-line wrapping the existing math_luts.rs layout uses.
fn print_raw(i: usize, raw: i64) {
    let trailing = if i == LUT_N - 1 { "" } else { "," };
    if i.is_multiple_of(4) {
        print!("    ");
    }
    print!("{raw}_i64{trailing}");
    if (i + 1).is_multiple_of(4) || i == LUT_N - 1 {
        println!();
    } else {
        print!(" ");
    }
}

#[test]
#[ignore]
#[allow(clippy::float_arithmetic)]
fn print_sigmoid_lut_raw() {
    println!("pub(crate) const SIGMOID_LUT_RAW: [i64; 257] = [");
    for i in 0..LUT_N {
        let x = lut_x_f64(i);
        let s: f64 = 1.0 / (1.0 + (-x).exp());
        let s_clamped = s.clamp(0.0, 1.0);
        let raw = Q32::from_f64_clamped(s_clamped).to_raw();
        print_raw(i, raw);
    }
    println!("];");
}

#[test]
#[ignore]
#[allow(clippy::float_arithmetic)]
fn print_exp_lut_raw() {
    println!("pub(crate) const EXP_LUT_RAW: [i64; 257] = [");
    for i in 0..LUT_N {
        let x = lut_x_f64(i);
        let e: f64 = x.exp();
        let raw = Q32::from_f64_clamped(e).to_raw();
        print_raw(i, raw);
    }
    println!("];");
}

//! Regeneration tool for the non-linear attribute-effect curve LUTs
//! (`src/attribute_curve_luts.rs`).
//!
//! Re-bakes the five per-class gamma power-curve LUTs from the f64 reference
//! `g(a) = a^γ` and prints them as Rust source literals suitable for paste
//! into `src/attribute_curve_luts.rs`.
//!
//! # When to run
//!
//! Only when `attribute_curve_drift_detection` fails (libm/toolchain change
//! to `f64::powf`) AND a coordinated regeneration is authorized — it
//! rebaselines the canonical hash since the curve feeds every utility-scored
//! BT decision + every physics coefficient.
//!
//! ```text
//! cargo test --package fw-core --test print_attribute_curve_luts -- --ignored --nocapture --test-threads=1
//! ```
//!
//! Capture stdout, extract the five `pub(crate) const ..._LUT_RAW = [ ... ];`
//! blocks, paste-replace the corresponding blocks in
//! `src/attribute_curve_luts.rs`. Re-run `attribute_curve_drift_detection`
//! to confirm.
//!
//! Mirrors `print_luts_oneshot.rs`: this printer IS the regeneration source of
//! truth on the current toolchain (it bakes from the f64 reference, it does NOT
//! echo committed const values).

use fw_core::Q32;

const LUT_N: usize = 257;

/// γ per class, per `docs/design/attribute-effect-audit-2026-06-06.md` §3.3.
/// (name, gamma) — name matches the committed const identifier.
const CLASSES: [(&str, f64); 5] = [
    ("SKILL", 1.7),
    ("PHYSICAL", 1.4),
    ("CONTEST", 1.8),
    ("MENTAL", 1.6),
    ("PERSONALITY", 1.3),
];

/// f64 x-coordinate for LUT entry `i`: `a = i / 256` over `[0, 1]`.
#[allow(clippy::float_arithmetic)]
fn lut_a_f64(i: usize) -> f64 {
    (i as f64) / ((LUT_N - 1) as f64)
}

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
fn print_attribute_curve_luts_raw() {
    for (name, gamma) in CLASSES {
        println!("pub(crate) const {name}_LUT_RAW: [i64; 257] = [");
        for i in 0..LUT_N {
            let a = lut_a_f64(i);
            let v = a.powf(gamma).clamp(0.0, 1.0);
            let raw = Q32::from_f64_clamped(v).to_raw();
            print_raw(i, raw);
        }
        println!("];");
        println!();
    }
}

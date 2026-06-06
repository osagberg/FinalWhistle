//! Proptest invariants for the non-linear attribute-effect curve
//! (`fw_core::attribute_curve`).
//!
//! The curve `g_class(a)` must be monotone non-decreasing (a higher attribute
//! is never worse), output within `[0, 1]`, and disproportionate at the elite
//! end (the `g(1.0)-g(0.9) > g(0.6)-g(0.5)` mandate). These properties are the
//! contract every sim read site relies on; a violation would let a curve regress
//! a player's effect or break the "elite skews" guarantee.

use fw_core::{CurveClass, Q32, curve};
use proptest::prelude::*;

const ALL_CLASSES: [CurveClass; 5] = [
    CurveClass::Skill,
    CurveClass::Physical,
    CurveClass::Contest,
    CurveClass::Mental,
    CurveClass::Personality,
];

/// Strategy over Q32 values in `[0, 1]` (raw bits `0 ..= 2^32`).
fn unit_q32() -> impl Strategy<Value = Q32> {
    (0i64..=(1i64 << 32)).prop_map(Q32::from_raw)
}

proptest! {
    /// Monotone non-decreasing: a higher attribute is never worse.
    #[test]
    fn curve_is_monotone(a in unit_q32(), b in unit_q32()) {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        for c in ALL_CLASSES {
            let g_lo = curve(c, lo);
            let g_hi = curve(c, hi);
            prop_assert!(
                g_hi >= g_lo,
                "{c:?} not monotone: g({lo:?})={g_lo:?} > g({hi:?})={g_hi:?}"
            );
        }
    }

    /// Output stays within `[0, 1]` for any input in `[0, 1]`.
    #[test]
    fn curve_output_in_unit_range(a in unit_q32()) {
        for c in ALL_CLASSES {
            let g = curve(c, a);
            prop_assert!(g >= Q32::ZERO, "{c:?} g({a:?})={g:?} < 0");
            prop_assert!(g <= Q32::ONE, "{c:?} g({a:?})={g:?} > 1");
        }
    }
}

/// Determinism: the curve is a pure function — same input, same output, every
/// call. (Cross-platform parity is gated by the committed LUT + the canonical
/// hash; this catches intra-process state leakage.)
#[test]
fn curve_is_deterministic() {
    let samples = [
        0i64,
        1 << 28,
        1 << 30,
        (1 << 31) + 7,
        (1 << 32) - 1,
        1 << 32,
    ];
    for c in ALL_CLASSES {
        for raw in samples {
            let a = Q32::from_raw(raw);
            assert_eq!(curve(c, a), curve(c, a), "{c:?} non-deterministic at {a:?}");
        }
    }
}

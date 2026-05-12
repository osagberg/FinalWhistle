//! Q32.32 fixed-point — the determinism primitive for Final Whistle.
//!
//! Top 32 bits store the signed integer part (two's-complement); bottom 32
//! bits store the fractional part. Range ≈ ±2.147e9; precision is
//! 2^-32 ≈ 2.328e-10.
//!
//! Used for every canonical-state quantity (positions, velocities, scores,
//! ball physics state, timers, derived trajectory values). NEVER use
//! `f32` / `f64` in canonical state — the
//! `#![deny(clippy::float_arithmetic)]` lint at the crate root of
//! `fw-match-sim` enforces this.
//!
//! Cross-platform reproducibility is the architectural floor per
//! `docs/specs/determinism-gate.md` §1–§3 and `docs/DESIGN_DOC.md` §5
//! "Determinism contract". This module is the foundation; `MatchState`,
//! `CanonicalEncoder`, and the replay-corpus pinned-hash gate all
//! transitively rely on Q32 being bit-identical across Mac / Windows /
//! Linux.
//!
//! Backed by the `fixed` crate, which provides correct integer-bit-exact
//! arithmetic. We wrap `FixedI64<U32>` in a newtype so the rest of the
//! codebase imports a single, well-named primitive (`Q32`) rather than
//! sprinkling `FixedI64<U32>` everywhere.

use cordic::CordicNumber;
use fixed::types::extra::U32;
use fixed::FixedI64;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// The underlying fixed-point type. Signed 64-bit, 32 fractional bits.
pub type Q32Inner = FixedI64<U32>;

/// Q32.32 fixed-point number — the determinism primitive.
///
/// Used for all canonical-state quantities (positions, velocities, scores,
/// ball physics state, timers, derived trajectory values). NEVER use
/// `f32` / `f64` in canonical state.
///
/// See module docs + `docs/specs/determinism-gate.md` for the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Q32(pub Q32Inner);

// -------------------------------------------------------------------------
// Constants
// -------------------------------------------------------------------------

impl Q32 {
    /// Zero. Raw bits = 0.
    pub const ZERO: Q32 = Q32(Q32Inner::ZERO);

    /// One. Raw bits = 2^32.
    pub const ONE: Q32 = Q32(Q32Inner::ONE);

    /// One ULP — the smallest positive non-zero representable value.
    /// Raw bits = 1, magnitude ≈ 2.328e-10.
    pub const EPSILON: Q32 = Q32(Q32Inner::DELTA);

    /// Largest representable value. Raw bits = `i64::MAX`.
    pub const MAX: Q32 = Q32(Q32Inner::MAX);

    /// Most negative representable value. Raw bits = `i64::MIN`.
    pub const MIN: Q32 = Q32(Q32Inner::MIN);

    // ---- Factories ------------------------------------------------------

    /// Construct from a 32-bit signed integer. Always safe — every `i32`
    /// fits in the Q32.32 integer range.
    #[inline]
    pub const fn from_int(n: i32) -> Q32 {
        // `FixedI64::<U32>::const_from_int` is const-fn and exact for any
        // i32. We do the cast through i64 explicitly so the const-fn
        // chain compiles on stable.
        Q32(Q32Inner::const_from_int(n as i64))
    }

    /// Additive identity. Equivalent to `Q32::ZERO`; provided as a
    /// function form to match the spec's API surface.
    #[inline]
    pub const fn zero() -> Q32 {
        Q32::ZERO
    }

    /// Multiplicative identity. Equivalent to `Q32::ONE`.
    #[inline]
    pub const fn one() -> Q32 {
        Q32::ONE
    }

    /// Construct from the raw underlying `i64`. Use ONLY for fixture
    /// authoring + deserialization. Normal sim-side code uses
    /// `from_int` or arithmetic operators.
    #[inline]
    pub const fn from_raw(bits: i64) -> Q32 {
        Q32(Q32Inner::from_bits(bits))
    }

    // ---- Accessors ------------------------------------------------------

    /// Underlying raw `i64`. Stable across runs + platforms. Exposed for
    /// fixture authoring + canonical encoding + debug tooling. Normal
    /// sim code uses arithmetic, not this.
    #[inline]
    pub const fn to_bits(self) -> i64 {
        self.0.to_bits()
    }

    // ---- Arithmetic (checked) ------------------------------------------
    //
    // Bare `+` / `-` / `*` / `/` operators use the underlying `fixed`
    // crate's `Wrapping`-style semantics on overflow in release mode.
    // For canonical-state arithmetic we want checked — silent wraparound
    // is forbidden. Callers in sim crates use these `checked_*` methods.

    /// Checked addition. Returns `None` on overflow.
    #[inline]
    pub fn checked_add(self, rhs: Q32) -> Option<Q32> {
        self.0.checked_add(rhs.0).map(Q32)
    }

    /// Checked subtraction. Returns `None` on overflow.
    #[inline]
    pub fn checked_sub(self, rhs: Q32) -> Option<Q32> {
        self.0.checked_sub(rhs.0).map(Q32)
    }

    /// Checked multiplication. Returns `None` on overflow.
    #[inline]
    pub fn checked_mul(self, rhs: Q32) -> Option<Q32> {
        self.0.checked_mul(rhs.0).map(Q32)
    }

    /// Checked division. Returns `None` on overflow or divide-by-zero.
    #[inline]
    pub fn checked_div(self, rhs: Q32) -> Option<Q32> {
        self.0.checked_div(rhs.0).map(Q32)
    }

    /// Absolute value. Returns `None` on `Q32::MIN` (whose negation does
    /// not fit).
    #[inline]
    pub fn checked_abs(self) -> Option<Q32> {
        self.0.checked_abs().map(Q32)
    }

    /// Sign: -1, 0, or 1.
    #[inline]
    pub fn signum(self) -> i32 {
        match self.0.cmp(&Q32Inner::ZERO) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }

    // ---- Min / Max ------------------------------------------------------

    #[inline]
    pub fn min(self, other: Q32) -> Q32 {
        if self.0 <= other.0 {
            self
        } else {
            other
        }
    }

    #[inline]
    pub fn max(self, other: Q32) -> Q32 {
        if self.0 >= other.0 {
            self
        } else {
            other
        }
    }

    // ---- Sqrt (CORDIC) --------------------------------------------------

    /// Non-negative square root via CORDIC. Panics on negative input.
    ///
    /// CORDIC is an integer-only iterative algorithm with bit-exact
    /// cross-platform behavior. The `cordic` crate hashes Q32 values to a
    /// fixed iteration count, so the result is deterministic regardless of
    /// host CPU.
    ///
    /// # Panics
    ///
    /// Panics if `self` is negative. Sqrt of a negative Q32 is undefined.
    #[inline]
    pub fn sqrt(self) -> Q32 {
        assert!(
            self.0 >= Q32Inner::ZERO,
            "Q32::sqrt called on negative value: {:?}",
            self
        );
        // `cordic::sqrt` is generic over `CordicNumber`; FixedI64<U32>
        // implements it. The iteration count is fixed (no early-exit
        // based on convergence in floating-point sense) so the result
        // is bit-identical across platforms.
        Q32(cordic::sqrt(self.0))
    }
}

// -------------------------------------------------------------------------
// Operator overloads (use checked_* in canonical-state crates)
// -------------------------------------------------------------------------

impl Add for Q32 {
    type Output = Q32;
    #[inline]
    fn add(self, rhs: Q32) -> Q32 {
        Q32(self.0 + rhs.0)
    }
}

impl Sub for Q32 {
    type Output = Q32;
    #[inline]
    fn sub(self, rhs: Q32) -> Q32 {
        Q32(self.0 - rhs.0)
    }
}

impl Mul for Q32 {
    type Output = Q32;
    #[inline]
    fn mul(self, rhs: Q32) -> Q32 {
        Q32(self.0 * rhs.0)
    }
}

impl Div for Q32 {
    type Output = Q32;
    #[inline]
    fn div(self, rhs: Q32) -> Q32 {
        Q32(self.0 / rhs.0)
    }
}

impl Neg for Q32 {
    type Output = Q32;
    #[inline]
    fn neg(self) -> Q32 {
        Q32(-self.0)
    }
}

impl AddAssign for Q32 {
    #[inline]
    fn add_assign(&mut self, rhs: Q32) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Q32 {
    #[inline]
    fn sub_assign(&mut self, rhs: Q32) {
        self.0 -= rhs.0;
    }
}

impl MulAssign for Q32 {
    #[inline]
    fn mul_assign(&mut self, rhs: Q32) {
        self.0 *= rhs.0;
    }
}

impl DivAssign for Q32 {
    #[inline]
    fn div_assign(&mut self, rhs: Q32) {
        self.0 /= rhs.0;
    }
}

// -------------------------------------------------------------------------
// Display (canonical decimal form with 10 fractional digits per FW-VAL-A-018
// reference convention; matches the FW C# Fixed.ToString())
// -------------------------------------------------------------------------

impl fmt::Display for Q32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Render via the underlying FixedI64<U32> Display which produces
        // a decimal-string. The fixed crate's Display is platform-stable.
        // For canonical 10-fractional-digit form we'd hand-roll; for now
        // delegate to the underlying impl.
        write!(f, "{}", self.0)
    }
}

// -------------------------------------------------------------------------
// Tests — Phase-0 determinism contract for Q32 itself
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------------
    // Arithmetic round-trips
    // ---------------------------------------------------------------------

    #[test]
    fn from_int_addition_round_trips() {
        // 2 + 3 == 5 — the simplest non-trivial check.
        assert_eq!(Q32::from_int(2) + Q32::from_int(3), Q32::from_int(5));
    }

    #[test]
    fn from_int_subtraction_round_trips() {
        assert_eq!(Q32::from_int(7) - Q32::from_int(4), Q32::from_int(3));
        assert_eq!(Q32::from_int(0) - Q32::from_int(5), Q32::from_int(-5));
    }

    #[test]
    fn from_int_multiplication_round_trips() {
        assert_eq!(Q32::from_int(6) * Q32::from_int(7), Q32::from_int(42));
        assert_eq!(Q32::from_int(-3) * Q32::from_int(4), Q32::from_int(-12));
    }

    #[test]
    fn from_int_division_round_trips() {
        // 12 / 4 = 3 exact.
        assert_eq!(Q32::from_int(12) / Q32::from_int(4), Q32::from_int(3));
        // 1 / 2 = 0.5; in Q32 that's raw = 2^31.
        let half = Q32::from_int(1) / Q32::from_int(2);
        assert_eq!(half.to_bits(), 1i64 << 31);
    }

    #[test]
    fn zero_one_identity() {
        assert_eq!(Q32::zero(), Q32::ZERO);
        assert_eq!(Q32::one(), Q32::ONE);
        assert_eq!(Q32::from_int(0), Q32::zero());
        assert_eq!(Q32::from_int(1), Q32::one());
    }

    #[test]
    fn negation_round_trip() {
        let x = Q32::from_int(123);
        assert_eq!(-(-x), x);
        assert_eq!(-x, Q32::from_int(-123));
    }

    // ---------------------------------------------------------------------
    // Checked arithmetic — overflow signals via Option
    // ---------------------------------------------------------------------

    #[test]
    fn checked_add_overflow_returns_none() {
        assert_eq!(Q32::MAX.checked_add(Q32::from_int(1)), None);
    }

    #[test]
    fn checked_sub_overflow_returns_none() {
        assert_eq!(Q32::MIN.checked_sub(Q32::from_int(1)), None);
    }

    #[test]
    fn checked_div_by_zero_returns_none() {
        assert_eq!(Q32::from_int(1).checked_div(Q32::zero()), None);
    }

    #[test]
    fn checked_abs_min_returns_none() {
        // |MIN| does not fit (its absolute magnitude is 2^31, but the
        // positive max is 2^31 - epsilon).
        assert_eq!(Q32::MIN.checked_abs(), None);
    }

    // ---------------------------------------------------------------------
    // Sqrt — cross-platform invariant
    // ---------------------------------------------------------------------

    #[test]
    fn sqrt_perfect_square_round_trips() {
        // sqrt(4) ≈ 2; allow one-ULP error from CORDIC convergence.
        let two = Q32::from_int(2);
        let four = Q32::from_int(4);
        let r = four.sqrt();
        let diff = if r > two { r - two } else { two - r };
        // One ULP of Q32 is Q32::EPSILON. CORDIC's convergence is bounded
        // by a small multiple; allow up to 4 ULP for robustness.
        assert!(
            diff.to_bits().abs() <= 4,
            "sqrt(4) ≈ 2 expected; got {:?} (raw bits {}, diff bits {})",
            r,
            r.to_bits(),
            diff.to_bits()
        );
    }

    #[test]
    fn sqrt_two_is_bit_identical_across_runs() {
        // The cross-platform-determinism load-bearing assertion: sqrt(2)
        // must produce a SPECIFIC bit pattern on every host. We do not
        // pin the literal here (CORDIC's exact output depends on the
        // crate version's iteration count) — but we DO assert that two
        // independent calls on the same machine produce bit-identical
        // results. The pinned canonical-state hash in
        // crates/fw-replay/tests/canonical_hash.rs is the platform-parity
        // gate; this test catches non-determinism within one process.
        let a = Q32::from_int(2).sqrt();
        let b = Q32::from_int(2).sqrt();
        assert_eq!(
            a.to_bits(),
            b.to_bits(),
            "sqrt(2) non-deterministic within process — sqrt is leaking state"
        );
    }

    #[test]
    fn sqrt_zero_is_zero() {
        assert_eq!(Q32::zero().sqrt(), Q32::zero());
    }

    #[test]
    #[should_panic(expected = "Q32::sqrt called on negative value")]
    fn sqrt_of_negative_panics() {
        let _ = Q32::from_int(-1).sqrt();
    }

    // ---------------------------------------------------------------------
    // Min / max / signum
    // ---------------------------------------------------------------------

    #[test]
    fn min_max_signum() {
        let a = Q32::from_int(3);
        let b = Q32::from_int(7);
        assert_eq!(a.min(b), a);
        assert_eq!(a.max(b), b);
        assert_eq!(a.signum(), 1);
        assert_eq!((-a).signum(), -1);
        assert_eq!(Q32::zero().signum(), 0);
    }

    // ---------------------------------------------------------------------
    // Serde round-trips (RON + bincode)
    // ---------------------------------------------------------------------

    #[test]
    fn serde_ron_round_trip() {
        // RON is the corpus fixture format. Q32 must round-trip through
        // RON identically — the underlying FixedI64<U32> uses a string
        // representation in RON.
        let original = Q32::from_int(42) + Q32::from_int(1) / Q32::from_int(4);
        let serialized = ron::to_string(&original).expect("ron serialize");
        let restored: Q32 = ron::from_str(&serialized).expect("ron deserialize");
        assert_eq!(
            original.to_bits(),
            restored.to_bits(),
            "RON round-trip altered raw bits: serialized = {}",
            serialized
        );
    }

    #[test]
    fn serde_bincode_round_trip() {
        // bincode is the save-file format. Q32 must round-trip byte-for-byte.
        let original = Q32::from_int(-17) + Q32::from_int(3) / Q32::from_int(8);
        let bytes = bincode::serialize(&original).expect("bincode serialize");
        let restored: Q32 = bincode::deserialize(&bytes).expect("bincode deserialize");
        assert_eq!(original.to_bits(), restored.to_bits());
    }

    #[test]
    fn serde_transparent_means_q32_serializes_as_inner() {
        // The #[serde(transparent)] attribute is load-bearing: Q32 must
        // serialize identically to its FixedI64<U32> inner. If a future
        // contributor removes the attribute, save files break silently.
        // We assert this by serializing Q32 and Q32Inner to bincode and
        // comparing byte-for-byte.
        let value = Q32::from_int(7);
        let q32_bytes = bincode::serialize(&value).unwrap();
        let inner_bytes = bincode::serialize(&value.0).unwrap();
        assert_eq!(
            q32_bytes, inner_bytes,
            "Q32 must serialize transparently as its inner FixedI64<U32>; \
             #[serde(transparent)] was likely removed"
        );
    }

    // ---------------------------------------------------------------------
    // Constants
    // ---------------------------------------------------------------------

    #[test]
    fn one_raw_bits_are_2_pow_32() {
        // ONE's raw representation is 2^32 — this is the definition of
        // Q32.32. If this fails, the U32 fractional-bits type parameter
        // was wrong.
        assert_eq!(Q32::ONE.to_bits(), 1i64 << 32);
    }

    #[test]
    fn epsilon_raw_bits_are_one() {
        assert_eq!(Q32::EPSILON.to_bits(), 1);
    }

    #[test]
    fn from_int_max_min_round_trip() {
        // The largest exact i32 we can shift into Q32.32.
        assert_eq!(Q32::from_int(i32::MAX).to_bits(), (i32::MAX as i64) << 32);
        assert_eq!(Q32::from_int(i32::MIN).to_bits(), (i32::MIN as i64) << 32);
    }
}

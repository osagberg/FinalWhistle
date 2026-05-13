//! `Tick` — the monotonic in-sim time counter.
//!
//! The sim runs at a fixed step. Each call to `tick_match` advances state by
//! exactly one `Tick`. There is no real clock involved at any point;
//! wall-clock time is a renderer-side interpolation concern only.
//!
//! `Tick` is `i64` (not `u64`) so subtraction yields a signed delta without
//! drama. Negative ticks are not valid in canonical state but are useful as
//! signed offsets in scheduling math.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, AddAssign, Sub, SubAssign};

use crate::q32::Q32;

/// The canonical sim tick rate, in ticks per second. 60 Hz matches the
/// pre-pivot FW C# sim and the Phase-0 smoke-fixture cadence
/// (`SMOKE_TICK_COUNT = 60` ≈ 1 in-sim second).
///
/// Changing this value is a determinism-corpus-invalidating event; every
/// pinned hash in the corpus is implicitly conditioned on this constant.
pub const TICKS_PER_SECOND: i64 = 60;

/// A monotonic in-sim tick. Construct via `Tick::ZERO` + `successor()`,
/// `Tick::from_raw(i64)`, or arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tick(i64);

impl Tick {
    /// The starting tick. `MatchState::initial(seed)` lives at `Tick::ZERO`.
    pub const ZERO: Tick = Tick(0);

    /// Construct from a raw `i64`. Use for fixture authoring + deserialization.
    #[inline]
    pub const fn from_raw(raw: i64) -> Tick {
        Tick(raw)
    }

    /// Raw underlying `i64`. Used for serialization + canonical encoding.
    #[inline]
    pub const fn to_raw(self) -> i64 {
        self.0
    }

    /// The next tick. Saturates at `i64::MAX` (the sim has no business
    /// running that long; a saturating successor avoids a panic mid-match).
    #[inline]
    pub const fn successor(self) -> Tick {
        Tick(self.0.saturating_add(1))
    }

    /// Convert from whole seconds. `Tick::from_seconds(1) == Tick::from_raw(60)`.
    #[inline]
    pub const fn from_seconds(seconds: i64) -> Tick {
        Tick(seconds.saturating_mul(TICKS_PER_SECOND))
    }

    /// Convert to whole seconds (integer-truncated). The inverse of
    /// `from_seconds`. Fractional ticks (e.g. tick 90 at 60 Hz = 1.5 s) are
    /// truncated; use `to_q32_seconds` if you need precision.
    #[inline]
    pub const fn to_seconds(self) -> i64 {
        self.0 / TICKS_PER_SECOND
    }

    /// Convert to seconds as a `Q32`. Bit-exact across platforms.
    #[inline]
    pub fn to_q32_seconds(self) -> Q32 {
        // Q32 doesn't have a const "from i64" because i64 doesn't fit in
        // the Q32 integer-part range in general. We do the divide in fixed
        // point: ticks / TICKS_PER_SECOND. Both operands fit Q32 trivially
        // for any reasonable tick count.
        //
        // CAUTION: For tick counts > i32::MAX (≈ 2.1 billion ticks ≈ 400
        // days of sim), this would overflow. The sim is single-match-scoped
        // and never runs that long; the assert below documents the invariant.
        debug_assert!(
            self.0.abs() <= i32::MAX as i64,
            "Tick::to_q32_seconds called with tick {} that exceeds Q32 \
             integer range; this should not happen in canonical-sim code",
            self.0
        );
        Q32::from_int(self.0 as i32) / Q32::from_int(TICKS_PER_SECOND as i32)
    }
}

impl Add for Tick {
    type Output = Tick;
    #[inline]
    fn add(self, rhs: Tick) -> Tick {
        Tick(self.0.saturating_add(rhs.0))
    }
}

impl Sub for Tick {
    type Output = Tick;
    #[inline]
    fn sub(self, rhs: Tick) -> Tick {
        Tick(self.0.saturating_sub(rhs.0))
    }
}

impl AddAssign for Tick {
    #[inline]
    fn add_assign(&mut self, rhs: Tick) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

impl SubAssign for Tick {
    #[inline]
    fn sub_assign(&mut self, rhs: Tick) {
        self.0 = self.0.saturating_sub(rhs.0);
    }
}

impl fmt::Display for Tick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t{}", self.0)
    }
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_and_successor() {
        assert_eq!(Tick::ZERO.to_raw(), 0);
        assert_eq!(Tick::ZERO.successor().to_raw(), 1);
        assert_eq!(Tick::ZERO.successor().successor().to_raw(), 2);
    }

    #[test]
    fn from_seconds_uses_tick_rate() {
        assert_eq!(Tick::from_seconds(0), Tick::ZERO);
        assert_eq!(Tick::from_seconds(1).to_raw(), TICKS_PER_SECOND);
        assert_eq!(Tick::from_seconds(2).to_raw(), 2 * TICKS_PER_SECOND);
    }

    #[test]
    fn to_seconds_round_trips_whole() {
        let t = Tick::from_seconds(7);
        assert_eq!(t.to_seconds(), 7);
    }

    #[test]
    fn arithmetic_uses_saturating_at_extremes() {
        // saturate, don't panic
        assert_eq!(
            Tick::from_raw(i64::MAX) + Tick::from_raw(1),
            Tick::from_raw(i64::MAX)
        );
        assert_eq!(
            Tick::from_raw(i64::MIN) - Tick::from_raw(1),
            Tick::from_raw(i64::MIN)
        );
    }

    #[test]
    fn display_is_t_prefixed() {
        assert_eq!(format!("{}", Tick::from_raw(42)), "t42");
    }
}

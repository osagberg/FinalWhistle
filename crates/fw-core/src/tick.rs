//! `Tick` — the monotonic in-sim time counter.
//!
//! The sim runs at a fixed step. Each call to `tick_match` advances state by
//! exactly one `Tick`. There is no real clock involved at any point;
//! wall-clock time is a renderer-side interpolation concern only.
//!
//! `Tick` is `i64` (not `u64`) so subtraction yields a signed delta without
//! drama. Negative ticks are not valid in canonical state but are useful as
//! signed offsets in scheduling math.
//!
//! ## Arithmetic policy (T1-21 — Sim/RULES.md §11 alignment)
//!
//! Arithmetic operators on `Tick` (`+`, `-`, `+=`, `-=`) panic on overflow /
//! underflow via `i64::checked_add().expect()` / `checked_sub().expect()`.
//! `Tick::successor()` and `Tick::from_seconds()` also panic on overflow.
//!
//! This matches `Q32`'s panic-on-overflow default (the `fixed` crate's
//! debug-AND-release-checked arithmetic) and forbids the silent-failure
//! pattern §11 bans: a release-build invariant violation (e.g. tactic-FSM
//! cooldown math computing `entry_tick > now_tick`) was previously silently
//! saturated to `i64::MIN`, which then satisfied any `tick_diff <= cooldown`
//! check trivially, masking the underlying invariant break.
//!
//! Saturation is still available as an explicit opt-in via the named methods
//! `Tick::clamping_add` and `Tick::clamping_sub`. Use these only when the
//! caller has a documented `// SAFETY:` rationale for why saturating
//! semantics are correct for that specific site (rare; most call sites want
//! panic-on-overflow).
//!
//! Pre-T1-21 behavior used `saturating_*` by default — see git blame on this
//! file for the prior implementation. The 2026-05-16 ultimate-review Track C
//! P1 surfaced the divergence; `Sim/RULES.md` §11 codified the policy; T1-21
//! aligned this implementation.

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

    /// The next tick.
    ///
    /// Panics on `i64::MAX` overflow per the T1-21 panic-on-overflow policy.
    /// The sim has no business running that long (~170 million years at 60Hz);
    /// a panic at the boundary is the §11-aligned semantic — a release-build
    /// reach to `i64::MAX` indicates a real bug (e.g. a stuck loop appending
    /// successors), not a graceful end-of-match.
    #[inline]
    pub fn successor(self) -> Tick {
        Tick(self.0.checked_add(1).expect(
            "Tick::successor overflowed i64::MAX — sim has no business \
             running this long; a release-build reach to i64::MAX indicates \
             a real bug (e.g. stuck loop appending successors)",
        ))
    }

    /// Convert from whole seconds. `Tick::from_seconds(1) == Tick::from_raw(60)`.
    ///
    /// Panics on `i64::MAX` overflow per the T1-21 panic-on-overflow policy.
    /// The argument is `i64` seconds; `seconds * 60` overflows above
    /// `i64::MAX / 60 ≈ 1.5e17` seconds (~4.9 trillion years). Not a real
    /// concern for sim code; the panic exists to surface programmer error
    /// (e.g. passing an unbounded user input as `seconds`).
    #[inline]
    pub fn from_seconds(seconds: i64) -> Tick {
        Tick(seconds.checked_mul(TICKS_PER_SECOND).expect(
            "Tick::from_seconds overflowed i64::MAX — `seconds * 60` exceeds \
             the i64 range; check whether the input is unbounded user data",
        ))
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
        //
        // debug_assert OK here because: caller-guaranteed by the sim's
        // single-match scope. A 400-day match is structurally impossible
        // (match_end_tick is bounded at construction time); this debug-only
        // guard is a pure development-time sanity check, NOT a load-bearing
        // canonical-state invariant. Per Sim/RULES.md §11 "Allowed" clause.
        debug_assert!(
            self.0.abs() <= i32::MAX as i64,
            "Tick::to_q32_seconds called with tick {} that exceeds Q32 \
             integer range; this should not happen in canonical-sim code",
            self.0
        );
        Q32::from_int(self.0 as i32) / Q32::from_int(TICKS_PER_SECOND as i32)
    }

    // ---------------------------------------------------------------------
    // Explicit opt-in saturating arithmetic (T1-21 per Sim/RULES.md §11)
    //
    // These methods exist for callers that genuinely want saturating
    // semantics — for example, a UI-side "snap to the end of the match"
    // computation that should produce a finite tick even if the input
    // exceeds the i64 range. Per §11, every call site of these methods
    // owes an inline `// SAFETY:`-style comment justifying why saturation
    // (vs panic-on-overflow) is the right semantic for that specific
    // call. Default operator arithmetic (`+`, `-`, `+=`, `-=`) panics
    // instead, which is the correct choice for canonical / gameplay code.
    // ---------------------------------------------------------------------

    /// Saturating addition. Result is clamped to `[i64::MIN, i64::MAX]`.
    ///
    /// **Opt-in only** — production canonical-state code uses `+` (which
    /// panics on overflow per the T1-21 policy). Call this method only
    /// when the caller has a documented `// SAFETY:` rationale for why
    /// saturation is correct (e.g. UI clamp paths, dev-tooling math).
    #[inline]
    pub const fn clamping_add(self, rhs: Tick) -> Tick {
        Tick(self.0.saturating_add(rhs.0))
    }

    /// Saturating subtraction. Result is clamped to `[i64::MIN, i64::MAX]`.
    ///
    /// **Opt-in only** — see `clamping_add` for the same opt-in semantics.
    #[inline]
    pub const fn clamping_sub(self, rhs: Tick) -> Tick {
        Tick(self.0.saturating_sub(rhs.0))
    }

    // ---------------------------------------------------------------------
    // Typed cooldown-math helpers (T1-23 per Codex post-followup-review
    // Finding #1)
    //
    // Cooldown / elapsed-time math in `fw-match-sim::tactic_fsm` +
    // `dispatch` + `signature` was hand-rolling `tick.to_raw() ± ...` raw
    // i64 arithmetic, bypassing the T1-21 panic-on-overflow operator
    // policy. These helpers expose the cooldown idioms as typed methods
    // that funnel through `i64::checked_*` so the §11 release-active
    // panic discipline applies to the production hot path, not just to
    // the operators no production code actually uses.
    //
    // These are the canonical alternative to direct `.to_raw()` arithmetic
    // for cooldown math. Direct `.to_raw()` should be reserved for:
    //   - serialization / canonical encoding (read only)
    //   - test fixture authoring
    //   - genuinely-signed-offset uses (rare; see file prologue)
    // ---------------------------------------------------------------------

    /// Return the number of ticks elapsed since `entry`, as a `u32`.
    ///
    /// Panics if `self < entry` — that would mean negative elapsed time,
    /// which is a cooldown-math invariant violation (e.g. `entry_tick`
    /// somehow lives in the future). Panics also if the difference exceeds
    /// `u32::MAX` ticks (~828 days at 60Hz; well past any realistic match).
    ///
    /// Why `u32` return: elapsed-tick counts in sim code feed comparisons
    /// against u32-typed `EveryTicks(u32)` / `DEFAULT_FIRING_DURATION_TICKS`
    /// / similar constants. Returning u32 keeps the comparison type-aligned
    /// without an explicit `as u32` at the call site that would silently
    /// truncate on overflow.
    ///
    /// T1-23 introduced this helper. Pre-T1-23 cooldown sites did
    /// `(now_tick.to_raw() - entry_tick.to_raw()) as u32` which silently
    /// underflows-then-truncates on invariant violation, hiding the bug.
    #[inline]
    pub fn checked_elapsed_since(self, entry: Tick) -> u32 {
        let diff = self.0.checked_sub(entry.0).expect(
            "Tick::checked_elapsed_since underflowed i64 — invariant violation \
             in cooldown math; use Tick::clamping_sub for opt-in saturation \
             (Sim/RULES.md §11)",
        );
        assert!(
            diff >= 0,
            "Tick::checked_elapsed_since called with entry={entry:?} > self={self:?}; \
             negative elapsed time is a cooldown-math invariant violation \
             (Sim/RULES.md §11)",
        );
        u32::try_from(diff).unwrap_or_else(|_| {
            panic!(
                "Tick::checked_elapsed_since diff {diff} exceeds u32::MAX \
                 (~828 days at 60Hz); upstream caller has a stuck-loop bug \
                 or entry_tick is from a different match"
            )
        })
    }

    /// Return `self + n` ticks as a new `Tick`, panicking on `i64` overflow.
    ///
    /// Cooldown-end math idiom: `state.tick.checked_add_ticks(cooldown_n)`
    /// computes the tick at which the cooldown expires. Pre-T1-23 sites did
    /// `Tick::from_raw(state.tick.to_raw() + n as i64)` which silently
    /// wraps on i64 overflow (impossible in practice but the explicit
    /// panic surfaces a stuck-loop bug at the violation site).
    ///
    /// Why `u32` argument: cooldown durations across the project are
    /// uniformly `u32` (`CooldownPolicy::EveryTicks(u32)`,
    /// `DEFAULT_FIRING_DURATION_TICKS: u32`, etc.). Restricting to u32
    /// rejects negative durations at compile time + avoids the `as i64`
    /// cast at call sites.
    #[inline]
    pub fn checked_add_ticks(self, n: u32) -> Tick {
        Tick(self.0.checked_add(n as i64).expect(
            "Tick::checked_add_ticks overflowed i64::MAX — invariant \
             violation; use Tick::clamping_add for opt-in saturation \
             (Sim/RULES.md §11)",
        ))
    }
}

impl Add for Tick {
    type Output = Tick;
    /// Panics on `i64` overflow per the T1-21 panic-on-overflow policy.
    /// Use `Tick::clamping_add` for opt-in saturation.
    #[inline]
    fn add(self, rhs: Tick) -> Tick {
        Tick(self.0.checked_add(rhs.0).expect(
            "Tick + Tick overflowed i64::MAX — invariant violation; use \
             Tick::clamping_add for opt-in saturation (Sim/RULES.md §11)",
        ))
    }
}

impl Sub for Tick {
    type Output = Tick;
    /// Panics on `i64` underflow per the T1-21 panic-on-overflow policy.
    /// Use `Tick::clamping_sub` for opt-in saturation.
    ///
    /// The specific failure mode this catches: cooldown math computing
    /// `now_tick - entry_tick` where `entry_tick > now_tick` (e.g. a
    /// tactic-FSM bug that set entry_tick in the future). Previously
    /// saturating to `i64::MIN` silently satisfied any `diff < cooldown`
    /// check; panic-on-underflow surfaces the bug at the violation site.
    #[inline]
    fn sub(self, rhs: Tick) -> Tick {
        Tick(self.0.checked_sub(rhs.0).expect(
            "Tick - Tick underflowed i64::MIN — invariant violation \
             (typically: subtracting a larger tick from a smaller one in \
             cooldown / elapsed-time math); use Tick::clamping_sub for \
             opt-in saturation (Sim/RULES.md §11)",
        ))
    }
}

impl AddAssign for Tick {
    /// Panics on overflow per the T1-21 panic-on-overflow policy.
    #[inline]
    fn add_assign(&mut self, rhs: Tick) {
        self.0 = self.0.checked_add(rhs.0).expect(
            "Tick += Tick overflowed i64::MAX — invariant violation; use \
             Tick::clamping_add for opt-in saturation (Sim/RULES.md §11)",
        );
    }
}

impl SubAssign for Tick {
    /// Panics on underflow per the T1-21 panic-on-overflow policy.
    #[inline]
    fn sub_assign(&mut self, rhs: Tick) {
        self.0 = self.0.checked_sub(rhs.0).expect(
            "Tick -= Tick underflowed i64::MIN — invariant violation; use \
             Tick::clamping_sub for opt-in saturation (Sim/RULES.md §11)",
        );
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
    fn display_is_t_prefixed() {
        assert_eq!(format!("{}", Tick::from_raw(42)), "t42");
    }

    // ---------------------------------------------------------------------
    // T1-21: panic-on-overflow alignment to Q32's policy (Sim/RULES.md §11)
    //
    // The pre-T1-21 implementation used `saturating_*` arithmetic which
    // silently capped at i64::MIN/MAX. Tactic-FSM cooldown math like
    // `now_tick - entry_tick` where `entry_tick > now_tick` (an invariant
    // violation) would saturate to i64::MIN, which then satisfied any
    // `diff < cooldown` check trivially, masking the underlying bug.
    //
    // The post-T1-21 implementation panics on overflow / underflow. The
    // tests below pin this — they `#[should_panic]` on the exact extremes
    // that were silently saturating before.
    //
    // Saturation is still available as an explicit opt-in via the named
    // `Tick::clamping_*` methods, tested in `clamping_methods_saturate_at_extremes`.
    // ---------------------------------------------------------------------

    /// T1-21 AC4 — re-interpreted scope: the MASTER_PLAN row's literal example
    /// (`Tick::ZERO - Tick::from_raw(1)`) yields `Tick::from_raw(-1)`, which
    /// is well within `i64`'s range; per the file prologue, negative ticks
    /// are explicitly "useful as signed offsets in scheduling math" — they
    /// are not invalid at the arithmetic layer (only in canonical-state
    /// fields where semantic validity is asserted upstream). The genuine
    /// underflow case `Tick::from_raw(i64::MIN) - Tick::from_raw(1)` IS an
    /// `i64` underflow + panics per the policy; that's tested below in
    /// `arithmetic_panics_on_subtraction_underflow`.
    ///
    /// This test verifies that subtracting in the negative-result zone does
    /// NOT panic — pinning the documented signed-offset affordance against
    /// accidental stricter-semantic regression. If a future change wants to
    /// ban negative-result Tick subtraction (would require auditing every
    /// `.to_raw()`-based subtraction site for signed-offset uses), this test
    /// must flip to `#[should_panic]` + the file prologue must update.
    #[test]
    fn tick_subtraction_into_negative_zone_does_not_panic() {
        let diff = Tick::ZERO - Tick::from_raw(1);
        assert_eq!(
            diff.to_raw(),
            -1,
            "Tick::ZERO - 1 must produce Tick::from_raw(-1) per the signed-offset \
             affordance documented at the top of this file; only true i64 \
             over/underflow at the extremes panics per T1-21 / Sim/RULES.md §11"
        );
    }

    /// Addition at i64::MAX overflows → panics.
    #[test]
    #[should_panic(expected = "Tick + Tick overflowed")]
    fn arithmetic_panics_on_addition_overflow() {
        let _overflow = Tick::from_raw(i64::MAX) + Tick::from_raw(1);
    }

    /// Subtraction crossing i64::MIN underflows → panics.
    #[test]
    #[should_panic(expected = "Tick - Tick underflowed")]
    fn arithmetic_panics_on_subtraction_underflow() {
        let _underflow = Tick::from_raw(i64::MIN) - Tick::from_raw(1);
    }

    /// AddAssign at i64::MAX overflows → panics.
    #[test]
    #[should_panic(expected = "Tick += Tick overflowed")]
    fn add_assign_panics_on_overflow() {
        let mut t = Tick::from_raw(i64::MAX);
        t += Tick::from_raw(1);
    }

    /// SubAssign crossing i64::MIN underflows → panics.
    #[test]
    #[should_panic(expected = "Tick -= Tick underflowed")]
    fn sub_assign_panics_on_underflow() {
        let mut t = Tick::from_raw(i64::MIN);
        t -= Tick::from_raw(1);
    }

    /// `Tick::successor()` panics at i64::MAX (no graceful saturation).
    #[test]
    #[should_panic(expected = "Tick::successor overflowed")]
    fn successor_panics_at_i64_max() {
        let _next = Tick::from_raw(i64::MAX).successor();
    }

    /// `Tick::from_seconds` panics on overflow.
    #[test]
    #[should_panic(expected = "Tick::from_seconds overflowed")]
    fn from_seconds_panics_on_overflow() {
        // i64::MAX seconds * 60 ticks/s overflows.
        let _t = Tick::from_seconds(i64::MAX);
    }

    /// Explicit opt-in saturation via `Tick::clamping_add` / `clamping_sub`.
    /// These do NOT panic — they saturate at i64::MAX/i64::MIN. Required
    /// for the rare call site that wants saturating semantics with a
    /// documented `// SAFETY:` rationale.
    #[test]
    fn clamping_methods_saturate_at_extremes() {
        assert_eq!(
            Tick::from_raw(i64::MAX).clamping_add(Tick::from_raw(1)),
            Tick::from_raw(i64::MAX),
            "clamping_add must saturate at i64::MAX, not panic",
        );
        assert_eq!(
            Tick::from_raw(i64::MIN).clamping_sub(Tick::from_raw(1)),
            Tick::from_raw(i64::MIN),
            "clamping_sub must saturate at i64::MIN, not panic",
        );
        // Non-extreme inputs: clamping_* behave identically to + / -.
        assert_eq!(
            Tick::from_raw(5).clamping_add(Tick::from_raw(3)),
            Tick::from_raw(8),
        );
        assert_eq!(
            Tick::from_raw(5).clamping_sub(Tick::from_raw(3)),
            Tick::from_raw(2),
        );
    }

    // -----------------------------------------------------------------
    // T1-23: typed cooldown-math helpers (Codex post-followup-review
    // Finding #1)
    // -----------------------------------------------------------------

    /// `checked_elapsed_since` returns the elapsed-tick count for valid
    /// inputs (entry ≤ self), as a u32 ready for u32-typed cooldown
    /// comparisons.
    #[test]
    fn checked_elapsed_since_returns_elapsed_count_for_valid_inputs() {
        assert_eq!(Tick::from_raw(10).checked_elapsed_since(Tick::ZERO), 10);
        assert_eq!(
            Tick::from_raw(600).checked_elapsed_since(Tick::from_raw(100)),
            500
        );
        assert_eq!(
            Tick::from_raw(7).checked_elapsed_since(Tick::from_raw(7)),
            0
        );
    }

    /// `checked_elapsed_since` panics when entry > self (cooldown
    /// invariant violation: entry_tick somehow in the future).
    #[test]
    #[should_panic(expected = "checked_elapsed_since called with entry=")]
    fn checked_elapsed_since_panics_when_entry_in_future() {
        let _underflow = Tick::ZERO.checked_elapsed_since(Tick::from_raw(1));
    }

    /// `checked_elapsed_since` panics on i64 underflow at the extreme
    /// (entry near i64::MAX, self near i64::MIN).
    #[test]
    #[should_panic(expected = "checked_elapsed_since underflowed i64")]
    fn checked_elapsed_since_panics_on_i64_underflow_at_extremes() {
        let _underflow = Tick::from_raw(i64::MIN).checked_elapsed_since(Tick::from_raw(1));
    }

    /// `checked_elapsed_since` panics when the difference exceeds u32::MAX
    /// (~828 days at 60Hz; only reachable via deliberately huge inputs).
    #[test]
    #[should_panic(expected = "exceeds u32::MAX")]
    fn checked_elapsed_since_panics_when_diff_exceeds_u32_max() {
        let _too_big = Tick::from_raw(i64::from(u32::MAX) + 1).checked_elapsed_since(Tick::ZERO);
    }

    /// `checked_add_ticks` adds non-negative duration as a Tick.
    #[test]
    fn checked_add_ticks_adds_duration() {
        assert_eq!(Tick::ZERO.checked_add_ticks(60), Tick::from_raw(60));
        assert_eq!(
            Tick::from_raw(100).checked_add_ticks(500),
            Tick::from_raw(600)
        );
        assert_eq!(Tick::from_raw(7).checked_add_ticks(0), Tick::from_raw(7));
    }

    /// `checked_add_ticks` panics on i64 overflow.
    #[test]
    #[should_panic(expected = "checked_add_ticks overflowed i64::MAX")]
    fn checked_add_ticks_panics_on_i64_overflow() {
        let _overflow = Tick::from_raw(i64::MAX).checked_add_ticks(1);
    }
}

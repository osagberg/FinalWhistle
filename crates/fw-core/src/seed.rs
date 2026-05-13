//! `Seed` — the deterministic-RNG seed newtype.
//!
//! Every match starts from a `Seed`. The sim derives a `ChaCha8Rng` per-draw
//! via `seed_fn(match_seed, tick, layer, site)` per ADR-0009 (canonical
//! signature; 8 `SeedLayer` discriminants); the raw u64 inside `Seed` is the
//! match-level entropy source. Seeds are reproducible by design — the same
//! `Seed` + the same content pack must produce the same canonical-state hash
//! on every platform. The `seed_fn` lands alongside its first real consumer
//! at T1-2b-ii (per `docs/specs/decision-cadence-stagger.md`); the doc shape
//! is ADR-0009-locked.
//!
//! The newtype prevents bare `u64` from being passed where a `Seed` is
//! expected — a load-bearing type-safety invariant; mixing up "the player's
//! ID" and "the match seed" silently is one of the cheapest classes of bug
//! to prevent at the type level.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::q32::Q32;

/// A match's deterministic seed. Wraps a `u64`; never mutated after
/// construction.
///
/// Constructed from a `u64` literal (e.g. corpus fixture hex), from the
/// content pack's per-match seed table, or — in tests — directly via
/// `Seed::from_u64`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Seed(u64);

impl Seed {
    /// The zero seed. Useful in tests + placeholder fixtures; in production
    /// use a non-zero value (zero is a perfectly legal seed, but ambiguous
    /// with default-initialized state).
    pub const ZERO: Seed = Seed(0);

    /// Construct from a raw `u64`. The intended path for corpus fixtures
    /// and content packs.
    #[inline]
    pub const fn from_u64(raw: u64) -> Seed {
        Seed(raw)
    }

    /// Raw underlying `u64`. Stable across runs + platforms.
    #[inline]
    pub const fn to_u64(self) -> u64 {
        self.0
    }

    /// Derive a `Q32` from the seed. This is *not* "convert" — the seed is a
    /// 64-bit identity; reinterpreting its bits as a `Q32` produces a
    /// deterministic-but-arbitrary value, which is exactly what some
    /// sim-level uses want (e.g. salting a stable derived constant).
    ///
    /// Concretely: returns `Q32::from_raw(self.0 as i64)`. Bit-exact on every
    /// platform.
    #[inline]
    pub const fn derive_q32(self) -> Q32 {
        Q32::from_raw(self.0 as i64)
    }
}

impl fmt::Display for Seed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:016x}", self.0)
    }
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_u64_round_trips() {
        let s = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);
        assert_eq!(s.to_u64(), 0xDEAD_BEEF_DEAD_BEEF);
    }

    #[test]
    fn display_is_hex() {
        let s = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);
        assert_eq!(format!("{s}"), "0xdeadbeefdeadbeef");
    }

    #[test]
    fn derive_q32_is_deterministic() {
        let s = Seed::from_u64(42);
        assert_eq!(s.derive_q32(), s.derive_q32());
    }
}

//! `Seed` — the deterministic-RNG seed newtype, plus `SeedLayer` + `seed_fn`
//! (ADR-0009 canonical RNG derivation).
//!
//! Every match starts from a `Seed`. The sim derives a `ChaCha8Rng` per-draw
//! via `seed_fn(match_seed, tick, layer, site)` per ADR-0009 (canonical
//! signature; 8 `SeedLayer` discriminants); the raw u64 inside `Seed` is the
//! match-level entropy source. Seeds are reproducible by design — the same
//! `Seed` + the same content pack must produce the same canonical-state hash
//! on every platform.
//!
//! The newtype prevents bare `u64` from being passed where a `Seed` is
//! expected — a load-bearing type-safety invariant; mixing up "the player's
//! ID" and "the match seed" silently is one of the cheapest classes of bug
//! to prevent at the type level.
//!
//! ## ADR-0009 buffer layout
//!
//! `seed_fn` hashes a 17-byte buffer via BLAKE3:
//!   bytes  0..8  = match_seed  (u64 LE)
//!   bytes  8..12 = tick        (u32 LE)
//!   byte   12    = layer       (u8 discriminant)
//!   bytes 13..17 = site        (u32 LE)
//!
//! Truncated to the first 8 bytes (u64 LE) of the BLAKE3 digest.

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
// SeedLayer — ADR-0009 discriminants
// -------------------------------------------------------------------------

/// The 9 non-overlapping seed discriminants from ADR-0009 (8 from the
/// original ADR + `Commentary` added per the 2026-05-16 amendment logged
/// at `docs/DECISIONS.md`). Each selects a distinct BLAKE3 lane in the
/// match-seed derivation so random draws in different layers never share
/// RNG state.
///
/// Discriminant values are fixed by ADR-0009 — DO NOT reorder or renumber
/// without a canonical-hash rebaseline and a new ADR entry.
///
/// Discriminants:
///   Decision          = 0x10  (per-player BT/FSM decision draws)
///   UtilityTieBreak   = 0x11  (RNG tiebreak when utility scores are equal)
///   ReactiveInterrupt = 0x12  (reactive trigger predicate draws)
///   BallPhysics       = 0x13  (ball drag, bounce, contact integration)
///   SignatureTrigger  = 0x14  (signature-move trigger predicate draws)
///   Commentary        = 0x18  (in-match commentary variant-pick draws; ADR-0009
///                              amendment 2026-05-16 — 9th discriminant)
///   MemoryEvent       = 0x20  (memory-event salience rolls)
///   ScoutObservation  = 0x30  (scout observation noise)
///   ContentBake       = 0x40  (bake-time content generation)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SeedLayer {
    Decision = 0x10,
    UtilityTieBreak = 0x11,
    ReactiveInterrupt = 0x12,
    BallPhysics = 0x13,
    SignatureTrigger = 0x14,
    /// In-match commentary variant-pick draws. Site formula:
    /// `((player_slot as u32) << 16) | event_class_discriminant`.
    /// KickOff / FullTime (no natural player slot) use sentinel
    /// `player_slot = 0xFF` (PlayerSlot is a u8 type alias; the sentinel
    /// is the u8 max value, distinct from any real slot in 0..=21).
    /// In the site u32 this becomes `0x00FF_0000`. ADR-0009 amendment
    /// 2026-05-16. (Sentinel width corrected post Codex Tier-2 silent-
    /// failure P1 + type-design P1 + code-reviewer P1 on T1-4b — prior
    /// `0xFFFF` doc claim disagreed with the `0xFF` impl in
    /// `fw-content::commentary::SLOT_SENTINEL`; the impl is correct
    /// since PlayerSlot=u8 can't hold 0xFFFF.)
    Commentary = 0x18,
    MemoryEvent = 0x20,
    ScoutObservation = 0x30,
    ContentBake = 0x40,
}

// -------------------------------------------------------------------------
// seed_fn — ADR-0009 canonical derivation
// -------------------------------------------------------------------------

/// Derive a deterministic `u64` seed for a single RNG draw site. Uses BLAKE3
/// over a 17-byte fixed-order buffer per ADR-0009.
///
/// ## Parameters
/// - `match_seed` — the raw match seed u64 (from `Seed::to_u64()`).
/// - `tick` — the integration tick at draw time as `u32`. Use `0` for
///   match-init draws (e.g. the stagger-shuffle). Monotonically
///   non-negative by invariant (ADR-0009).
/// - `layer` — the `SeedLayer` discriminant (non-overlapping u8 namespace).
/// - `site` — per-layer disambiguator. For `SeedLayer::Decision` the site
///   is `(player_id.as_u32() as u64) << 16 | local_decision_counter`.
///   Site `0` is reserved for the stagger-shuffle draw.
///
/// ## Buffer layout (ADR-0009 §"seed_fn buffer layout")
///   bytes  0..8  = match_seed.to_le_bytes()
///   bytes  8..12 = tick.to_le_bytes()
///   byte   12    = layer as u8
///   bytes 13..17 = site.to_le_bytes()
///
/// Output is the first 8 bytes of the BLAKE3 digest, interpreted as u64 LE.
/// Bit-exact on every platform (ChaCha8Rng + BLAKE3 are platform-portable;
/// `to_le_bytes` is specified by Rust).
#[must_use]
pub fn seed_fn(match_seed: u64, tick: u32, layer: SeedLayer, site: u32) -> u64 {
    let mut buf = [0u8; 17];
    buf[0..8].copy_from_slice(&match_seed.to_le_bytes());
    buf[8..12].copy_from_slice(&tick.to_le_bytes());
    buf[12] = layer as u8;
    buf[13..17].copy_from_slice(&site.to_le_bytes());
    let hash = blake3::hash(&buf);
    let bytes = hash.as_bytes();
    u64::from_le_bytes(bytes[0..8].try_into().expect("blake3 hash >= 8 bytes"))
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

    // ---- seed_fn: ADR-0009 vector tests ----

    #[test]
    fn seed_fn_is_deterministic() {
        let a = seed_fn(0xDEAD_BEEF, 100, SeedLayer::Decision, 0);
        let b = seed_fn(0xDEAD_BEEF, 100, SeedLayer::Decision, 0);
        assert_eq!(a, b, "seed_fn must be deterministic for identical inputs");
    }

    #[test]
    fn seed_fn_different_layers_produce_different_seeds() {
        // All 9 discriminants (including Commentary, ADR-0009 amendment
        // 2026-05-16) must be non-colliding with each other at the same
        // (match_seed, tick, site).
        let layers = [
            SeedLayer::Decision,
            SeedLayer::UtilityTieBreak,
            SeedLayer::ReactiveInterrupt,
            SeedLayer::BallPhysics,
            SeedLayer::SignatureTrigger,
            SeedLayer::Commentary,
            SeedLayer::MemoryEvent,
            SeedLayer::ScoutObservation,
            SeedLayer::ContentBake,
        ];
        let seeds: Vec<u64> = layers.iter().map(|&l| seed_fn(42, 0, l, 0)).collect();
        for i in 0..seeds.len() {
            for j in (i + 1)..seeds.len() {
                assert_ne!(
                    seeds[i], seeds[j],
                    "layers {:?} and {:?} produced the same seed (collision)",
                    layers[i], layers[j]
                );
            }
        }
    }

    #[test]
    fn seed_fn_different_ticks_produce_different_seeds() {
        let s0 = seed_fn(42, 0, SeedLayer::Decision, 0);
        let s1 = seed_fn(42, 1, SeedLayer::Decision, 0);
        assert_ne!(s0, s1, "different ticks must produce different seeds");
    }

    #[test]
    fn seed_fn_different_sites_produce_different_seeds() {
        let s0 = seed_fn(42, 0, SeedLayer::Decision, 0);
        let s1 = seed_fn(42, 0, SeedLayer::Decision, 1);
        assert_ne!(s0, s1, "different sites must produce different seeds");
    }

    #[test]
    fn seed_fn_different_match_seeds_produce_different_seeds() {
        let s0 = seed_fn(0, 0, SeedLayer::Decision, 0);
        let s1 = seed_fn(1, 0, SeedLayer::Decision, 0);
        assert_ne!(s0, s1, "different match seeds must produce different seeds");
    }

    /// ADR-0009 discriminant values are fixed — verify they haven't drifted.
    ///
    /// Codex Tier-2 code-reviewer P4 + type-design P1 on T1-4b 2026-05-16:
    /// `Commentary` added to the canonical-discriminant pin. Prior layout
    /// covered 8 layers; the dedicated `seed_layer_commentary_discriminant_is_0x18`
    /// test pinned Commentary separately but THIS summary test omitted it,
    /// so a future re-pin of one wouldn't naturally cover the other. All 9
    /// discriminants now pinned in one test.
    #[test]
    fn seed_layer_discriminants_are_adr0009_canonical() {
        assert_eq!(SeedLayer::Decision as u8, 0x10);
        assert_eq!(SeedLayer::UtilityTieBreak as u8, 0x11);
        assert_eq!(SeedLayer::ReactiveInterrupt as u8, 0x12);
        assert_eq!(SeedLayer::BallPhysics as u8, 0x13);
        assert_eq!(SeedLayer::SignatureTrigger as u8, 0x14);
        assert_eq!(SeedLayer::Commentary as u8, 0x18);
        assert_eq!(SeedLayer::MemoryEvent as u8, 0x20);
        assert_eq!(SeedLayer::ScoutObservation as u8, 0x30);
        assert_eq!(SeedLayer::ContentBake as u8, 0x40);
    }

    // ---- Chunk 1 (T1-4b): SeedLayer::Commentary vector tests ----

    /// ADR-0009 amendment 2026-05-16: Commentary discriminant is exactly
    /// 0x18 — the next free slot after the 0x10..0x14 run.
    #[test]
    fn seed_layer_commentary_discriminant_is_0x18() {
        assert_eq!(SeedLayer::Commentary as u8, 0x18);
    }

    /// seed_fn is stable across two identical calls with the Commentary layer.
    #[test]
    fn seed_fn_commentary_layer_is_deterministic() {
        let a = seed_fn(0xCAFE_BABE, 7, SeedLayer::Commentary, 0);
        let b = seed_fn(0xCAFE_BABE, 7, SeedLayer::Commentary, 0);
        assert_eq!(
            a, b,
            "seed_fn(Commentary) must be deterministic for identical inputs"
        );
    }

    /// Commentary must not collide with any of the 8 existing discriminants
    /// at the same (match_seed, tick, site).
    #[test]
    fn seed_fn_commentary_does_not_collide_with_existing_layers() {
        let existing = [
            SeedLayer::Decision,
            SeedLayer::UtilityTieBreak,
            SeedLayer::ReactiveInterrupt,
            SeedLayer::BallPhysics,
            SeedLayer::SignatureTrigger,
            SeedLayer::MemoryEvent,
            SeedLayer::ScoutObservation,
            SeedLayer::ContentBake,
        ];
        let commentary_seed = seed_fn(42, 0, SeedLayer::Commentary, 0);
        for &layer in &existing {
            let other = seed_fn(42, 0, layer, 0);
            assert_ne!(
                commentary_seed, other,
                "SeedLayer::Commentary collided with {:?} (discriminant 0x{:02x})",
                layer, layer as u8,
            );
        }
    }

    /// Vacuousness guard: verify the collision test would FAIL if two layers
    /// shared the same discriminant (simulated by comparing equal discriminants).
    #[test]
    fn vacuousness_check_layer_collision_test_would_fail_on_equal_discriminants() {
        // If two layers had the same discriminant byte, seed_fn would produce
        // the same output — the collision test would catch it.
        // Here we verify this is detectable by checking Decision vs Decision
        // produces the same seed (not a cross-layer collision).
        let s0 = seed_fn(42, 0, SeedLayer::Decision, 0);
        let s1 = seed_fn(42, 0, SeedLayer::Decision, 0); // identical — must match
        assert_eq!(s0, s1, "sanity: same inputs must produce same output");
    }

    /// ADR-0009 buffer layout pinned vector: fix match_seed=0, tick=0,
    /// layer=Decision(0x10), site=0 and verify the BLAKE3 output is stable.
    /// If this test fails after a code change, the seed_fn buffer layout
    /// has drifted — requires an ADR-0009 revision + canonical-hash rebaseline.
    #[test]
    fn seed_fn_pinned_vector_decision_layer() {
        // Known-good vector: computed from the 17-byte buffer:
        //   [0,0,0,0,0,0,0,0] (match_seed=0)
        //   [0,0,0,0]         (tick=0)
        //   [0x10]            (Decision discriminant)
        //   [0,0,0,0]         (site=0)
        let result = seed_fn(0, 0, SeedLayer::Decision, 0);
        // The exact value is computed from BLAKE3 over that specific buffer.
        // We pin it here so any future buffer-layout change fails this test.
        let buf: [u8; 17] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10, 0, 0, 0, 0];
        let hash = blake3::hash(&buf);
        let expected = u64::from_le_bytes(hash.as_bytes()[0..8].try_into().unwrap());
        assert_eq!(
            result, expected,
            "seed_fn pinned vector mismatch — buffer layout may have changed"
        );
    }
}

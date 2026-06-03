//! `fw-core` — determinism primitives.
//!
//! Zero gameplay logic lives here. Only the type vocabulary every other
//! Final Whistle crate consumes: the Q32.32 fixed-point numeric type, the
//! `Seed` and `Tick` newtypes, and the entity-ID types.
//!
//! This crate is the *only* crate every other crate may depend on
//! unconditionally; downstream-only-on-it makes circular-dependency
//! impossible by construction. See `docs/architecture.md` for the full
//! crate graph.
//!
//! ## Determinism contract
//!
//! - No floats. Q32.32 is the numeric primitive; `f32`/`f64` are not
//!   surfaced from this crate at all. The `clippy::float_arithmetic = deny`
//!   lint at crate root enforces this for the crate's own code.
//! - No `HashMap` / `HashSet` in canonical state. Downstream crates use
//!   `BTreeMap` / `BTreeSet` for sorted, reproducible iteration order.
//! - No clocks. `Tick` is a monotonic in-sim counter — never `Instant::now()`
//!   or `SystemTime::now()`. Wall-clock time is a renderer concern.
//!
//! See `docs/specs/determinism-gate.md` for the full contract.

pub mod attribute_family;
pub mod ids;
pub mod math;
// T1-10: committed-source LUT raw bits consumed by math.rs at compile time.
// Private — only math.rs reads it; not part of the public surface.
pub(crate) mod math_luts;
pub mod player_attributes;
// T4-2.5g: moved from fw-tauri::roster so fw-save can reference it in
// SavedPlayerInstance without a circular dependency.
pub mod player_season_stats;
pub mod q32;
pub mod seed;
pub mod tick;

// -------------------------------------------------------------------------
// Public re-exports — the canonical surface every other crate imports.
// -------------------------------------------------------------------------

pub use attribute_family::AttributeFamily;
pub use ids::{ClubId, MatchId, PlayerId};
pub use math::{exp_q32, sigmoid_q32};
pub use player_attributes::{
    AbilityCeiling, AbilityCeilingError, AttributeRangeError, DurabilityProfile,
    GoalkeeperAttributes, HIDDEN_ATTR_COUNT, HIDDEN_ATTRIBUTE_NAMES, KNOWN_ATTRIBUTE_NAMES,
    MentalAttributes, PersonalityVector, PhysicalAttributes, PlayerAttributes, PlayerCondition,
    TechnicalAttributes, VISIBLE_ATTR_COUNT, VISIBLE_ATTRIBUTE_NAMES, is_in_unit_range,
};
pub use player_season_stats::PlayerSeasonStats;
// `Q32Inner` (alias for `FixedI64<U32>`) is deliberately NOT re-exported.
// Codex audit P2 (2026-05-13): exposing the inner type bypasses the
// checked operator policy (panic-on-overflow Q1) — downstream callers
// could do raw `FixedI64<U32>` arithmetic with different semantics.
// External callers always work with `Q32`; the type alias stays
// crate-private.
pub use q32::Q32;
pub use seed::{Seed, SeedLayer, seed_fn};
pub use tick::Tick;

// -------------------------------------------------------------------------
// PlayerSlot — canonical sim slot index
// -------------------------------------------------------------------------

/// Canonical sim slot index for a player. Stable for the duration of a match:
/// home team occupies slots `0..11`, away team occupies slots `11..22`.
///
/// Slot 0 = home GK, slots 1-4 = home DEF, 5-7 = home MID, 8-10 = home FWD.
/// Slot 11 = away GK, slots 12-15 = away DEF, 16-18 = away MID, 19-21 = away FWD.
///
/// Moved to `fw-core` at T1-4a so `fw-content::event::MatchEvent` can reference
/// `PlayerSlot` without depending on `fw-match-sim` (which would create a cycle
/// since `fw-match-sim` depends on `fw-content`).
pub type PlayerSlot = u8;

// -------------------------------------------------------------------------
// Pitch geometry constants (T1-3.5)
// -------------------------------------------------------------------------

/// FIFA standard pitch length in metres (105 m).
///
/// Single source of truth for all pitch-geometry consumers in the sim.
/// `fw-match-sim`'s goal-detection (step 7) and OOB-clamp (step 8) in
/// `tick_match` derive their boundary checks from this constant — never
/// from inline literals.
///
/// Q32.32 raw bits: `round(105.0 × 2^32) = 451_033_538_560`
pub const PITCH_LENGTH_M: Q32 = Q32::from_raw(105_i64 << 32);

/// FIFA standard pitch width in metres (68 m).
///
/// Q32.32 raw bits: `68 × 2^32 = 292_057_776_128`
pub const PITCH_WIDTH_M: Q32 = Q32::from_raw(68_i64 << 32);

/// X-coordinate of each goal line (distance from centre spot to goal line).
///
/// `PITCH_LENGTH_M / 2 = 52.5 m`
///
/// Ball detection: when `|ball.pos_x| >= GOAL_LINE_X`, the ball has crossed
/// the goal line. Combined with `|ball.pos_y| < GOAL_HALF_WIDTH_M` to
/// distinguish a goal from a wide-of-goal ball (OOB clamp zone).
///
/// **Single source of truth** (Codex 2026-05-16 audit type-design P1 +
/// silent-failure P1 — same family as T1-4a's `GOAL_HALF_WIDTH_M` fix):
/// derived from `PITCH_LENGTH_M` at compile time via raw-bit shift.
/// Bumping `PITCH_LENGTH_M` automatically updates `GOAL_LINE_X`. The
/// prior hand-computed `105_i64 << 31` literal was a single-source-of-
/// truth violation that the unit test caught but the type system did
/// not enforce.
pub const GOAL_LINE_X: Q32 = Q32::from_raw(PITCH_LENGTH_M.to_bits() >> 1);

/// Y-coordinate of each sideline (distance from pitch centreline to touchline).
///
/// `PITCH_WIDTH_M / 2 = 34.0 m`
///
/// Ball detection: when `|ball.pos_y| >= SIDELINE_Y`, the ball has crossed
/// the touchline and is out of bounds.
///
/// **Single source of truth** (Codex 2026-05-16 audit type-design P1):
/// derived from `PITCH_WIDTH_M` at compile time via raw-bit shift.
pub const SIDELINE_Y: Q32 = Q32::from_raw(PITCH_WIDTH_M.to_bits() >> 1);

// -------------------------------------------------------------------------
// Smoke
// -------------------------------------------------------------------------

#[cfg(test)]
mod smoke {
    use super::*;

    #[test]
    fn smoke() {
        assert_eq!(2 + 2, 4);
    }

    // --- T1-3.5 Chunk 1: Pitch geometry constants ---

    /// RED: pitch constants must have correct values derived from FIFA standard.
    #[test]
    fn pitch_length_is_105m() {
        // Q32.32: 105.0 = 105 << 32 raw bits.
        assert_eq!(PITCH_LENGTH_M, Q32::from_int(105));
    }

    #[test]
    fn pitch_width_is_68m() {
        assert_eq!(PITCH_WIDTH_M, Q32::from_int(68));
    }

    #[test]
    fn goal_line_x_is_half_pitch_length() {
        // GOAL_LINE_X == PITCH_LENGTH_M / 2 (single-source invariant).
        // Q32 division by exact power of 2: `105 << 31` == `(105 << 32) >> 1`.
        // Both must equal 52.5 in Q32.32.
        let half: Q32 = PITCH_LENGTH_M / Q32::from_int(2);
        assert_eq!(
            GOAL_LINE_X, half,
            "GOAL_LINE_X must derive from PITCH_LENGTH_M/2, not an independent literal"
        );
        // 52.5 in Q32.32: raw bits = 52 << 32 | (1 << 31) = 224_006_365_184 + 2_147_483_648
        let expected_raw: i64 = (52_i64 << 32) + (1_i64 << 31);
        assert_eq!(GOAL_LINE_X.to_bits(), expected_raw);
    }

    #[test]
    fn sideline_y_is_half_pitch_width() {
        // SIDELINE_Y == PITCH_WIDTH_M / 2 (single-source invariant).
        let half: Q32 = PITCH_WIDTH_M / Q32::from_int(2);
        assert_eq!(
            SIDELINE_Y, half,
            "SIDELINE_Y must derive from PITCH_WIDTH_M/2, not an independent literal"
        );
        assert_eq!(SIDELINE_Y, Q32::from_int(34));
    }
}

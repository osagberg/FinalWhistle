//! `Scout` archetype definition — the immutable type identity of a scout.
//!
//! Track record lives in save-state, not here. The Path-B MVP ships the
//! `BasicScoutUncertainty` archetype; Path-A reserved slots are present in
//! the enum for schema stability.
//!
//! Per `design/scouting.md §"Type contract"`.

use fw_core::Q32;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Tuning constants for the basic-uncertainty archetype
// ---------------------------------------------------------------------------

/// Path-B category-estimate noise amplitude (± around true mean).
/// Raw bits: `round(0.10 × 2^32)` = 429_496_730
pub const BASIC_SCOUT_OBSERVATION_NOISE: Q32 = Q32::from_raw(429_496_730_i64);

/// Half-width of a `GeneCategoryEstimate` `[low, high]` band.
/// Raw bits: `round(0.12 × 2^32)` = 515_396_076
pub const BASIC_SCOUT_BAND_HALF_WIDTH: Q32 = Q32::from_raw(515_396_076_i64);

/// Lower bound of the per-label confidence draw.
/// Raw bits: `round(0.40 × 2^32)` = 1_717_986_918
pub const LABEL_CONFIDENCE_MIN: Q32 = Q32::from_raw(1_717_986_918_i64);

/// Upper bound of the per-label confidence draw.
/// Raw bits: `round(0.95 × 2^32)` = 4_080_218_931
/// 0.95 × 2^32 = 4_080_218_931.2 — within i64 range since 2^32 = 4_294_967_296.
pub const LABEL_CONFIDENCE_MAX: Q32 = Q32::from_raw(4_080_218_931_i64);

/// Fallback overall confidence when a player has no scout labels.
/// Value: exactly `0.5`. Raw bits: `round(0.5 × 2^32)` = 2_147_483_648.
pub const NO_LABEL_DEFAULT_CONFIDENCE: Q32 = Q32::from_raw(2_147_483_648_i64);

// ---------------------------------------------------------------------------
// ScoutArchetypeKind
// ---------------------------------------------------------------------------

/// Scout archetype variant. `#[repr(u8)]` with explicit discriminants — reordering
/// is a visible diff; discriminants are locked per canonical-state-crate discipline.
///
/// Path B ships `BasicScoutUncertainty`; the rest are reserved for Path A (T4+).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ScoutArchetypeKind {
    /// Path B — the T3-5 MVP archetype.
    BasicScoutUncertainty = 0,
    /// Path A (reserved) — overweights physical, misses mental.
    PhysicalProfiler = 1,
    /// Path A (reserved) — overweights technical + mental.
    TechnicalPurist = 2,
    /// Path A (reserved) — accurate in-region, noisy elsewhere.
    RegionalExpert = 3,
    /// T4+ expansion (reserved).
    TempoReader = 4,
    /// T4+ expansion (reserved).
    AcademySpotter = 5,
    /// T4+ expansion (reserved).
    SetPieceSpecialist = 6,
}

// ---------------------------------------------------------------------------
// CategoryBiasesError
// ---------------------------------------------------------------------------

/// Error returned when `CategoryBiases::try_new` is called with an invalid config.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CategoryBiasesError {
    #[error("narrative_flag bias must be zero; scouts never observe narrative flags")]
    NarrativeFlagNonZero,
}

// ---------------------------------------------------------------------------
// CategoryBiases
// ---------------------------------------------------------------------------

/// Category-level observation-bias weights for a scout archetype.
///
/// `narrative_flag` MUST be `0` — enforced by `try_new` / `validate`.
/// For Path B all four are `0` (neutral fog).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryBiases {
    pub physical: Q32,
    pub mental: Q32,
    pub technical: Q32,
    /// MUST be zero. Scouts never observe the narrative-flag gene category.
    pub narrative_flag: Q32,
}

impl CategoryBiases {
    /// Construct and validate. Returns `Err` if `narrative_flag != 0`.
    pub fn try_new(
        physical: Q32,
        mental: Q32,
        technical: Q32,
        narrative_flag: Q32,
    ) -> Result<Self, CategoryBiasesError> {
        let biases = CategoryBiases {
            physical,
            mental,
            technical,
            narrative_flag,
        };
        biases.validate()?;
        Ok(biases)
    }

    /// Validate the constraint that `narrative_flag` is zero.
    ///
    /// Returns `Err(CategoryBiasesError::NarrativeFlagNonZero)` if it is not.
    pub fn validate(&self) -> Result<(), CategoryBiasesError> {
        if self.narrative_flag != Q32::ZERO {
            return Err(CategoryBiasesError::NarrativeFlagNonZero);
        }
        Ok(())
    }

    /// All-zero biases — the neutral-fog configuration for Path B.
    pub const fn all_zero() -> Self {
        CategoryBiases {
            physical: Q32::ZERO,
            mental: Q32::ZERO,
            technical: Q32::ZERO,
            narrative_flag: Q32::ZERO,
        }
    }
}

// ---------------------------------------------------------------------------
// Scout
// ---------------------------------------------------------------------------

/// Immutable scout archetype definition.
///
/// Track record (reliability accumulating from outcomes over seasons) lives in
/// the save file, never here. This is the static type identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scout {
    /// Content-pack-qualified archetype ID.
    pub archetype_id: String,
    pub kind: ScoutArchetypeKind,
    /// Player-facing display name.
    pub display_name: String,
    /// Player-facing description.
    pub ui_description: String,
    /// Category-level bias weights. All zero for Path B.
    pub biases: CategoryBiases,
    /// Regions the scout is familiar with. Empty for Path B.
    pub familiar_regions: Vec<String>,
    /// Noise amplitude ∈ [0, 1] for category-estimate draws.
    pub base_observation_noise: Q32,
    /// Regional noise penalty ∈ [0, 1]. Path A only; zero for Path B.
    pub regional_noise_penalty: Q32,
}

impl Scout {
    /// The canonical Path-B `BasicScoutUncertainty` archetype with tuning-seed values.
    ///
    /// `archetype_id = "fwh.core:scout.basic-uncertainty"` per the design doc.
    pub fn basic_uncertainty() -> Scout {
        Scout {
            archetype_id: "fwh.core:scout.basic-uncertainty".to_string(),
            kind: ScoutArchetypeKind::BasicScoutUncertainty,
            display_name: "General scout".to_string(),
            ui_description: "A generalist observer with no particular bias.".to_string(),
            biases: CategoryBiases::all_zero(),
            familiar_regions: vec![],
            base_observation_noise: BASIC_SCOUT_OBSERVATION_NOISE,
            regional_noise_penalty: Q32::ZERO,
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_uncertainty_constructor_builds_correct_kind() {
        let scout = Scout::basic_uncertainty();
        assert_eq!(scout.kind, ScoutArchetypeKind::BasicScoutUncertainty);
        assert_eq!(scout.archetype_id, "fwh.core:scout.basic-uncertainty");
    }

    #[test]
    fn basic_uncertainty_has_all_zero_biases() {
        let scout = Scout::basic_uncertainty();
        assert_eq!(scout.biases, CategoryBiases::all_zero());
    }

    #[test]
    fn scout_archetype_kind_discriminants_are_locked() {
        assert_eq!(ScoutArchetypeKind::BasicScoutUncertainty as u8, 0);
        assert_eq!(ScoutArchetypeKind::PhysicalProfiler as u8, 1);
        assert_eq!(ScoutArchetypeKind::TechnicalPurist as u8, 2);
        assert_eq!(ScoutArchetypeKind::RegionalExpert as u8, 3);
        assert_eq!(ScoutArchetypeKind::TempoReader as u8, 4);
        assert_eq!(ScoutArchetypeKind::AcademySpotter as u8, 5);
        assert_eq!(ScoutArchetypeKind::SetPieceSpecialist as u8, 6);
    }

    #[test]
    fn label_confidence_min_is_approximately_0_40() {
        let expected_raw: i64 = (0.40_f64 * (1_u64 << 32) as f64).round() as i64;
        let diff = (LABEL_CONFIDENCE_MIN.to_bits() - expected_raw).abs();
        assert!(diff <= 1, "LABEL_CONFIDENCE_MIN off by {diff}");
    }

    #[test]
    fn label_confidence_max_is_approximately_0_95() {
        let expected_raw: i64 = (0.95_f64 * (1_u64 << 32) as f64).round() as i64;
        let diff = (LABEL_CONFIDENCE_MAX.to_bits() - expected_raw).abs();
        assert!(diff <= 1, "LABEL_CONFIDENCE_MAX off by {diff}");
    }

    #[test]
    fn observation_noise_is_approximately_0_10() {
        let expected_raw: i64 = (0.10_f64 * (1_u64 << 32) as f64).round() as i64;
        let diff = (BASIC_SCOUT_OBSERVATION_NOISE.to_bits() - expected_raw).abs();
        assert!(diff <= 1, "BASIC_SCOUT_OBSERVATION_NOISE off by {diff}");
    }

    #[test]
    fn band_half_width_is_approximately_0_12() {
        let expected_raw: i64 = (0.12_f64 * (1_u64 << 32) as f64).round() as i64;
        let diff = (BASIC_SCOUT_BAND_HALF_WIDTH.to_bits() - expected_raw).abs();
        assert!(diff <= 1, "BASIC_SCOUT_BAND_HALF_WIDTH off by {diff}");
    }
}

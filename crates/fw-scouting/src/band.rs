//! `UncertaintyBand` — the "fog displayed as text, not numbers" deliverable.
//!
//! Maps a `Q32` confidence value to one of four football-native text bands.
//! Per `design/scouting.md §"UncertaintyBand"` and §"T3 tuning seeds".

use fw_core::Q32;
use serde::{Deserialize, Serialize};

use crate::report::GeneCategoryEstimate;

// ---------------------------------------------------------------------------
// Tuning constants — NOT DECISIONS-locked. Revise freely during balancing.
// ---------------------------------------------------------------------------

/// confidence < 0.35 → Hunch
/// Raw bits: `round(0.35 × 2^32)` = 1_503_238_554
pub const UNCERTAINTY_BAND_HUNCH_MAX: Q32 = Q32::from_raw(1_503_238_554_i64);

/// [0.35, 0.60) → Tentative
/// Raw bits: `round(0.60 × 2^32)` = 2_576_980_378
pub const UNCERTAINTY_BAND_TENTATIVE_MAX: Q32 = Q32::from_raw(2_576_980_378_i64);

/// [0.60, 0.82) → Confident; >= 0.82 → Settled
/// Raw bits: `round(0.82 × 2^32)` = 3_521_873_183
pub const UNCERTAINTY_BAND_CONFIDENT_MAX: Q32 = Q32::from_raw(3_521_873_183_i64);

// ---------------------------------------------------------------------------
// UncertaintyBand
// ---------------------------------------------------------------------------

/// Textual confidence band for a scout observation.
///
/// Displayed to the player as football-native prose — never as raw numbers.
/// Exhaustive match in `display_label` ensures compile-time completeness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UncertaintyBand {
    /// confidence < 0.35 — only a feeling, barely actionable.
    Hunch,
    /// 0.35 ≤ confidence < 0.60 — some evidence but still uncertain.
    Tentative,
    /// 0.60 ≤ confidence < 0.82 — reasonably sure.
    Confident,
    /// confidence ≥ 0.82 — scout has a firm read.
    Settled,
}

impl UncertaintyBand {
    /// Map a `Q32 ∈ [0, 1]` confidence to a band.
    ///
    /// Thresholds per `design/scouting.md §"T3 tuning seeds"`.
    #[must_use]
    pub fn from_confidence(c: Q32) -> UncertaintyBand {
        if c < UNCERTAINTY_BAND_HUNCH_MAX {
            UncertaintyBand::Hunch
        } else if c < UNCERTAINTY_BAND_TENTATIVE_MAX {
            UncertaintyBand::Tentative
        } else if c < UNCERTAINTY_BAND_CONFIDENT_MAX {
            UncertaintyBand::Confident
        } else {
            UncertaintyBand::Settled
        }
    }

    /// Player-facing football-native label. Exhaustive match — adding a new
    /// variant without an arm is a compile error.
    ///
    /// Banned-terms-clean: no capitalized mystical nouns, no "+N stat" forms.
    #[must_use]
    pub fn display_label(&self) -> &'static str {
        match self {
            UncertaintyBand::Hunch => "a hunch",
            UncertaintyBand::Tentative => "a tentative read",
            UncertaintyBand::Confident => "a confident read",
            UncertaintyBand::Settled => "a settled read",
        }
    }
}

impl GeneCategoryEstimate {
    /// Derive a band from this estimate's `[low, high]` width.
    ///
    /// `width = high - low`; `effective_confidence = clamp(1 - width, 0, 1)`.
    /// A narrow band → high confidence → higher band.
    #[must_use]
    pub fn band(&self) -> UncertaintyBand {
        let width = self.high - self.low;
        // clamp(1 - width, 0, 1)
        let effective_confidence = (Q32::ONE - width).max(Q32::ZERO).min(Q32::ONE);
        UncertaintyBand::from_confidence(effective_confidence)
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hunch_max_is_approximately_0_35() {
        // 0.35 × 2^32 ≈ 1_503_238_553.6
        // We verify the const is within ±1 ULP of expected.
        let expected_raw: i64 = (0.35_f64 * (1_u64 << 32) as f64).round() as i64;
        let diff = (UNCERTAINTY_BAND_HUNCH_MAX.to_bits() - expected_raw).abs();
        assert!(diff <= 1, "HUNCH_MAX raw bits off by {diff}");
    }

    #[test]
    fn from_confidence_zero_is_hunch() {
        assert_eq!(
            UncertaintyBand::from_confidence(Q32::ZERO),
            UncertaintyBand::Hunch
        );
    }

    #[test]
    fn from_confidence_one_is_settled() {
        assert_eq!(
            UncertaintyBand::from_confidence(Q32::ONE),
            UncertaintyBand::Settled
        );
    }

    #[test]
    fn band_display_labels_all_non_empty() {
        for band in [
            UncertaintyBand::Hunch,
            UncertaintyBand::Tentative,
            UncertaintyBand::Confident,
            UncertaintyBand::Settled,
        ] {
            assert!(!band.display_label().is_empty());
        }
    }
}

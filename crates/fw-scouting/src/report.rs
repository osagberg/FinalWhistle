//! Report types — `GeneCategory`, `GeneCategoryEstimate`, `LabelEstimate`, `ScoutReport`.
//!
//! These are the canonical data shapes output by `observe_player`. The `band.rs`
//! module adds the `GeneCategoryEstimate::band` method via an `impl` block there.
//!
//! Per `design/scouting.md §"Type contract"`.

use fw_content::PhenotypeLabelId;
use fw_core::Q32;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// GeneCategory — 3 variants; no NarrativeFlag by design
// ---------------------------------------------------------------------------

/// Gene category observable by a scout.
///
/// Deliberately has NO `NarrativeFlag` variant — narrative flags are never
/// scout-observable (compile-time exclusion). See `design/scouting.md §"Locked decisions"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneCategory {
    Physical,
    Mental,
    Technical,
}

// ---------------------------------------------------------------------------
// GeneCategoryEstimateError
// ---------------------------------------------------------------------------

/// Error returned when `GeneCategoryEstimate::try_new` or `validate` detects an
/// invalid configuration.
///
/// Mirrors the `CategoryBiasesError` pattern in `scout.rs` and the T2-4
/// `GeneSnapshot::validate()` precedent.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GeneCategoryEstimateError {
    #[error("low ({low}) > high ({high}): GeneCategoryEstimate invariant violated")]
    LowExceedsHigh { low: Q32, high: Q32 },
    #[error("bound out of [0, 1]: low={low}, high={high}")]
    OutOfRange { low: Q32, high: Q32 },
}

// ---------------------------------------------------------------------------
// GeneCategoryEstimate
// ---------------------------------------------------------------------------

/// A scout's estimated `[low, high]` band for a player's category-level score.
///
/// Invariant: `low <= high`, both clamped to `[0, 1]`.
/// The default display surface renders `self.band()` as text, never raw numbers.
///
/// Fields are `pub` to allow direct construction in tests and direct serde
/// round-trip; full encapsulation (`#[serde(try_from)]`) is a deferred follow-up
/// per the T2-4 `GeneSnapshot` precedent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneCategoryEstimate {
    pub category: GeneCategory,
    /// Lower bound of the scout's estimate; `Q32 ∈ [0, 1]`.
    pub low: Q32,
    /// Upper bound of the scout's estimate; `Q32 ∈ [0, 1]`.
    pub high: Q32,
}

impl GeneCategoryEstimate {
    /// Construct and validate a `GeneCategoryEstimate`.
    ///
    /// Returns `Err` if `low > high` or if either bound is outside `[0, 1]`.
    pub fn try_new(
        category: GeneCategory,
        low: Q32,
        high: Q32,
    ) -> Result<Self, GeneCategoryEstimateError> {
        let est = GeneCategoryEstimate {
            category,
            low,
            high,
        };
        est.validate()?;
        Ok(est)
    }

    /// Validate the `low <= high` and `[0, 1]` bounds invariants.
    pub fn validate(&self) -> Result<(), GeneCategoryEstimateError> {
        if self.low < Q32::ZERO || self.high > Q32::ONE {
            return Err(GeneCategoryEstimateError::OutOfRange {
                low: self.low,
                high: self.high,
            });
        }
        if self.low > self.high {
            return Err(GeneCategoryEstimateError::LowExceedsHigh {
                low: self.low,
                high: self.high,
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// LabelEstimate
// ---------------------------------------------------------------------------

/// "This scout thinks this player carries this phenotype label, with this confidence."
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabelEstimate {
    pub label: PhenotypeLabelId,
    /// Scout's confidence in this label; `Q32 ∈ [0, 1]`.
    pub confidence: Q32,
}

// ---------------------------------------------------------------------------
// ScoutReport
// ---------------------------------------------------------------------------

/// A single scout's observation of a player.
///
/// Structured data is canonical; prose is a rendered artifact (deferred to
/// a later `narrative-director` row). `ScoutReport` is event-class-free:
/// the career-loop emitter selects which `MemoryEvent` class to emit per report.
///
/// Per `design/scouting.md §"Type contract"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoutReport {
    /// Content-pack-qualified archetype ID for the scout that generated this report.
    pub scout_archetype_id: String,
    /// Content-pack-qualified ID of the observed player.
    pub player_id: String,
    /// Overall confidence in this report; `Q32 ∈ [0, 1]`.
    /// Arithmetic mean of `label_estimates[].confidence`; `0.5` if no labels.
    pub confidence: Q32,
    /// One entry per true label in `PlayerBio.scout_labels`, in `BTreeSet` iteration order.
    pub label_estimates: Vec<LabelEstimate>,
    /// Exactly 3 entries: Physical, Mental, Technical — in that order.
    pub category_estimates: Vec<GeneCategoryEstimate>,
}

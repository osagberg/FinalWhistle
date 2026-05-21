//! Internal gene model for `PlayerBio` (T2-4).
//!
//! These types represent the 22-field gene snapshot per `design/player-generation.md`
//! §"Internal gene model (LOCKED — 22 fields, 4 categories)". They are NEVER
//! surfaced as "genes" in the UI. Phenotype labels surface instead.
//!
//! All numeric fields are `Q32` (`fixed::FixedI64<U32>`) per `Sim/RULES.md §1`.
//! `growth_curve` and `mentality` range −1.0..+1.0; all others 0.0..+1.0.
//!
//! `NarrativeFlag` is defined here (fw-content's domain — the gene model belongs
//! to fw-content). `fw-memory` carries a parallel copy for `BreakthroughContext`
//! — deduplication is a logged follow-up, NOT T2-4 scope.

use std::collections::BTreeSet;

use fw_core::Q32;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// GeneRangeError — carries the invariant so it travels with the type
// ---------------------------------------------------------------------------

/// Validation error returned by `GeneSnapshot::validate`.
///
/// Lives in `fw-content` so the range invariant travels with the type and
/// callers outside the baker (future sim code, tests, proc-gen) can enforce
/// it without depending on `fw-content-baker`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeneRangeError {
    /// A unit-range field (`Q32 ∈ [0, 1]`) was out of range.
    UnitOutOfRange { field: &'static str, value: Q32 },
    /// A signed-range field (`Q32 ∈ [-1, +1]`) was out of range.
    SignedOutOfRange { field: &'static str, value: Q32 },
}

impl std::fmt::Display for GeneRangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeneRangeError::UnitOutOfRange { field, value } => {
                write!(f, "gene field {field:?} = {value:?} is outside [0, 1]")
            }
            GeneRangeError::SignedOutOfRange { field, value } => {
                write!(f, "gene field {field:?} = {value:?} is outside [-1, +1]")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// §A. Physical profile (7 fields)
// ---------------------------------------------------------------------------

/// Physical gene profile. All fields are `Q32 ∈ [0, 1]` except `growth_curve`
/// which ranges `[-1, +1]` (negative = early-peak; positive = late-bloomer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalGenes {
    /// Observable directly by scouts; Q32 ∈ [0, 1].
    pub height_ceiling: Q32,
    /// Lean / athletic / strong tradeoff signal; Q32 ∈ [0, 1].
    pub frame_density: Q32,
    /// Pace-vs-stamina tradeoff; Q32 ∈ [0, 1].
    pub fast_twitch_ratio: Q32,
    /// Late-match performance signal; Q32 ∈ [0, 1].
    pub stamina_recovery: Q32,
    /// Early-peak (−1) vs late-bloomer (+1); Q32 ∈ [-1, +1].
    pub growth_curve: Q32,
    /// Longevity — scouts can only guess; Q32 ∈ [0, 1].
    pub aging_curve: Q32,
    /// Observable over seasons through injury history; Q32 ∈ [0, 1].
    pub injury_resilience: Q32,
}

// ---------------------------------------------------------------------------
// §B. Mental profile (6 fields)
// ---------------------------------------------------------------------------

/// Mental gene profile. All fields `Q32 ∈ [0, 1]` except `mentality`
/// which ranges `[-1, +1]` (negative = introvert-grinder; positive = extrovert-charisma).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MentalGenes {
    /// Reading-the-game observable; Q32 ∈ [0, 1].
    pub pattern_recognition: Q32,
    /// Pressure-observed; Q32 ∈ [0, 1].
    pub composure_floor: Q32,
    /// Time-to-decision observable; Q32 ∈ [0, 1].
    pub decision_velocity: Q32,
    /// Improvement-over-time observable; Q32 ∈ [0, 1].
    pub learning_rate: Q32,
    /// Training-willingness + loyalty tradeoff; Q32 ∈ [0, 1].
    pub ambition: Q32,
    /// Introvert-grinder (−1) vs extrovert-charisma (+1); Q32 ∈ [-1, +1].
    pub mentality: Q32,
}

// ---------------------------------------------------------------------------
// §C. Technical affinities (5 fields)
// ---------------------------------------------------------------------------

/// Technical affinity profile. All fields `Q32 ∈ [0, 1]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TechnicalAffinities {
    /// Preferred left-foot work; Q32 ∈ [0, 1].
    pub left_foot: Q32,
    /// Header / high-ball affinity; Q32 ∈ [0, 1].
    pub aerial: Q32,
    /// Free-kick + penalty affinity; Q32 ∈ [0, 1].
    pub dead_ball: Q32,
    /// Power-shot + volley affinity; Q32 ∈ [0, 1].
    pub striking: Q32,
    /// Control + trap affinity; Q32 ∈ [0, 1].
    pub first_touch: Q32,
}

// ---------------------------------------------------------------------------
// §D. Narrative trigger flags (4 variants — NEVER called "Soul genes")
// ---------------------------------------------------------------------------

/// Narrative trigger flags per `design/player-generation.md §D`.
///
/// These are architectural flags, NOT lexical labels. Player-facing UI surfaces
/// them through narrative events ("Something clicked for him today"), NOT as
/// capitalized mystical state-nouns. See `docs/design/ui-vocabulary.md`.
///
/// Variant names mirror the §D field names from the design doc:
/// - `flow_access` → `FlowAccess`
/// - `peak_ceiling_high` → `PeakCeilingHigh`
/// - `late_bloomer` → `LateBloomer`
/// - `awakening_dormant` → `AwakeningDormant`
///
/// NOTE: `fw-memory` carries a parallel `NarrativeFlag` in `breakthrough.rs`
/// for `BreakthroughContext`. That copy is NOT modified by T2-4. Deduplication
/// (likely by moving to `fw-core`) is a logged follow-up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NarrativeFlag {
    /// ~1.5% carriers. Sustained-readiness states in high-stakes matches,
    /// unlocked by a specific qualifying match event.
    FlowAccess,
    /// ~0.5% carriers. Raised cap on peak-state expression; raised by first
    /// qualifying high-stakes career event.
    PeakCeilingHigh,
    /// Variable rarity per cohort. Dormant until a specific career event
    /// (relegation decisive goal, cup-run moment, etc.).
    LateBloomer,
    /// ~0.2% carriers. Very-late-career explosive trait activation (post-25).
    AwakeningDormant,
}

// ---------------------------------------------------------------------------
// GeneSnapshot — the full internal record
// ---------------------------------------------------------------------------

/// Complete internal gene snapshot for one player. Stored in
/// `PlayerBio.internal_gene_snapshot`. Never surfaced to the UI directly —
/// only phenotype labels from `PlayerBio.scout_labels` appear in the player card
/// or scout report. The advanced tooltip surfaces scout-ESTIMATED ranges, not
/// this true snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneSnapshot {
    /// 7 physical gene fields (§A above).
    pub physical: PhysicalGenes,
    /// 6 mental gene fields (§B above).
    pub mental: MentalGenes,
    /// 5 technical affinity fields (§C above).
    pub technical: TechnicalAffinities,
    /// Activated narrative-trigger flags from §D. BTreeSet for deterministic
    /// serde iteration order per `Sim/RULES.md §2`.
    pub narrative_flags: BTreeSet<NarrativeFlag>,
}

// ---------------------------------------------------------------------------
// GeneSnapshot::validate — range invariant travels with the type
// ---------------------------------------------------------------------------

impl GeneSnapshot {
    /// Validate all 22 gene fields are within their declared ranges.
    ///
    /// Most fields: `Q32 ∈ [0, 1]`.
    /// `growth_curve` + `mentality`: `Q32 ∈ [-1, +1]`.
    ///
    /// Returns the FIRST out-of-range field found.
    pub fn validate(&self) -> Result<(), GeneRangeError> {
        let neg_one = Q32::from_int(-1);

        let check_unit = |val: Q32, field: &'static str| -> Result<(), GeneRangeError> {
            if val < Q32::ZERO || val > Q32::ONE {
                return Err(GeneRangeError::UnitOutOfRange { field, value: val });
            }
            Ok(())
        };

        let check_signed = |val: Q32, field: &'static str| -> Result<(), GeneRangeError> {
            if val < neg_one || val > Q32::ONE {
                return Err(GeneRangeError::SignedOutOfRange { field, value: val });
            }
            Ok(())
        };

        // §A Physical (7)
        check_unit(self.physical.height_ceiling, "height_ceiling")?;
        check_unit(self.physical.frame_density, "frame_density")?;
        check_unit(self.physical.fast_twitch_ratio, "fast_twitch_ratio")?;
        check_unit(self.physical.stamina_recovery, "stamina_recovery")?;
        check_signed(self.physical.growth_curve, "growth_curve")?;
        check_unit(self.physical.aging_curve, "aging_curve")?;
        check_unit(self.physical.injury_resilience, "injury_resilience")?;

        // §B Mental (6)
        check_unit(self.mental.pattern_recognition, "pattern_recognition")?;
        check_unit(self.mental.composure_floor, "composure_floor")?;
        check_unit(self.mental.decision_velocity, "decision_velocity")?;
        check_unit(self.mental.learning_rate, "learning_rate")?;
        check_unit(self.mental.ambition, "ambition")?;
        check_signed(self.mental.mentality, "mentality")?;

        // §C Technical (5)
        check_unit(self.technical.left_foot, "left_foot")?;
        check_unit(self.technical.aerial, "aerial")?;
        check_unit(self.technical.dead_ball, "dead_ball")?;
        check_unit(self.technical.striking, "striking")?;
        check_unit(self.technical.first_touch, "first_touch")?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke: construct a default-like GeneSnapshot and round-trip via RON.
    #[test]
    fn gene_snapshot_ron_round_trip() {
        let snap = GeneSnapshot {
            physical: PhysicalGenes {
                height_ceiling: Q32::from_raw(2_147_483_648_i64), // 0.5
                frame_density: Q32::from_raw(2_147_483_648_i64),
                fast_twitch_ratio: Q32::from_raw(2_147_483_648_i64),
                stamina_recovery: Q32::from_raw(2_147_483_648_i64),
                growth_curve: Q32::ZERO,
                aging_curve: Q32::from_raw(2_147_483_648_i64),
                injury_resilience: Q32::from_raw(2_147_483_648_i64),
            },
            mental: MentalGenes {
                pattern_recognition: Q32::from_raw(2_147_483_648_i64),
                composure_floor: Q32::from_raw(2_147_483_648_i64),
                decision_velocity: Q32::from_raw(2_147_483_648_i64),
                learning_rate: Q32::from_raw(2_147_483_648_i64),
                ambition: Q32::from_raw(2_147_483_648_i64),
                mentality: Q32::ZERO,
            },
            technical: TechnicalAffinities {
                left_foot: Q32::ZERO,
                aerial: Q32::from_raw(2_147_483_648_i64),
                dead_ball: Q32::from_raw(2_147_483_648_i64),
                striking: Q32::from_raw(2_147_483_648_i64),
                first_touch: Q32::from_raw(2_147_483_648_i64),
            },
            narrative_flags: {
                let mut s = BTreeSet::new();
                s.insert(NarrativeFlag::LateBloomer);
                s
            },
        };

        let encoded = ron::ser::to_string(&snap).expect("ron encode");
        let decoded: GeneSnapshot = ron::de::from_str(&encoded).expect("ron decode");
        assert_eq!(decoded, snap);
    }

    #[test]
    fn narrative_flag_4_variants_ordered() {
        // BTreeSet iteration order mirrors Ord. Verify all 4 variants exist.
        let mut s = BTreeSet::new();
        s.insert(NarrativeFlag::FlowAccess);
        s.insert(NarrativeFlag::PeakCeilingHigh);
        s.insert(NarrativeFlag::LateBloomer);
        s.insert(NarrativeFlag::AwakeningDormant);
        assert_eq!(s.len(), 4);
    }

    fn valid_gene_snapshot() -> GeneSnapshot {
        let half = Q32::from_raw(2_147_483_648_i64); // 0.5
        GeneSnapshot {
            physical: PhysicalGenes {
                height_ceiling: half,
                frame_density: half,
                fast_twitch_ratio: half,
                stamina_recovery: half,
                growth_curve: Q32::ZERO,
                aging_curve: half,
                injury_resilience: half,
            },
            mental: MentalGenes {
                pattern_recognition: half,
                composure_floor: half,
                decision_velocity: half,
                learning_rate: half,
                ambition: half,
                mentality: Q32::ZERO,
            },
            technical: TechnicalAffinities {
                left_foot: Q32::ZERO,
                aerial: half,
                dead_ball: half,
                striking: half,
                first_touch: half,
            },
            narrative_flags: BTreeSet::new(),
        }
    }

    #[test]
    fn gene_snapshot_validate_ok_on_valid_snapshot() {
        assert!(valid_gene_snapshot().validate().is_ok());
    }

    #[test]
    fn gene_snapshot_validate_rejects_height_ceiling_above_one() {
        let mut snap = valid_gene_snapshot();
        // 5.0 — well above the [0,1] bound.
        snap.physical.height_ceiling = Q32::from_int(5);
        let err = snap.validate().expect_err("must reject height_ceiling = 5");
        match err {
            GeneRangeError::UnitOutOfRange { field, .. } => {
                assert_eq!(field, "height_ceiling");
            }
            other => panic!("expected UnitOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn gene_snapshot_validate_accepts_growth_curve_at_neg_one_boundary() {
        let mut snap = valid_gene_snapshot();
        snap.physical.growth_curve = Q32::from_int(-1); // exactly -1.0 — valid
        assert!(
            snap.validate().is_ok(),
            "growth_curve = -1 must be accepted"
        );
    }
}

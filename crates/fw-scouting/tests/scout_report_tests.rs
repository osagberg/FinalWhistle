//! Integration tests for the `fw-scouting` scout-uncertainty model (T3-5).
//!
//! Test coverage per the AC-to-test matrix:
//! - Serde round-trip of `ScoutReport` (all sub-types populated)
//! - `observe_player` determinism: same inputs → same output; different career_seed → different output
//! - `CategoryBiases` validator: nonzero `narrative_flag` is rejected; zero is accepted
//! - `UncertaintyBand`: all 4 variants have non-empty unique `display_label()`; `from_confidence` boundary cases
//! - Report correctness: exactly 3 `category_estimates`, each with `low <= high`; `label_estimates` count == scout_labels count

use std::collections::BTreeSet;

use fw_content::{
    GeneSnapshot, MentalGenes, PhenotypeLabelId, PhysicalGenes, PlayerBio, TechnicalAffinities,
};
use fw_core::Q32;
use fw_scouting::{
    CategoryBiases, GeneCategory, GeneCategoryEstimate, LabelEstimate, Scout, ScoutReport,
    UncertaintyBand, observe_player,
};

// ---------------------------------------------------------------------------
// Q32 constants for test boundary values (no f64 in test code)
// Raw bits: round(value × 2^32)
// ---------------------------------------------------------------------------

/// 0.1 as Q32 — raw bits round(0.1 × 2^32) = 429_496_730
const Q_0_1: Q32 = Q32::from_raw(429_496_730_i64);
/// 0.2 as Q32 — raw bits round(0.2 × 2^32) = 858_993_459
const Q_0_2: Q32 = Q32::from_raw(858_993_459_i64);
/// 0.3 as Q32 — raw bits round(0.3 × 2^32) = 1_288_490_189
const Q_0_3: Q32 = Q32::from_raw(1_288_490_189_i64);
/// 0.34 as Q32 — raw bits round(0.34 × 2^32) = 1_460_288_881
const Q_0_34: Q32 = Q32::from_raw(1_460_288_881_i64);
/// 0.35 as Q32 — raw bits round(0.35 × 2^32) = 1_503_238_554
const Q_0_35: Q32 = Q32::from_raw(1_503_238_554_i64);
/// 0.4 as Q32 — raw bits round(0.4 × 2^32) = 1_717_986_918
const Q_0_4: Q32 = Q32::from_raw(1_717_986_918_i64);
/// 0.45 as Q32 — raw bits round(0.45 × 2^32) = 1_932_735_283
const Q_0_45: Q32 = Q32::from_raw(1_932_735_283_i64);
/// 0.5 as Q32 — raw bits round(0.5 × 2^32) = 2_147_483_648
const Q_0_5: Q32 = Q32::from_raw(2_147_483_648_i64);
/// 0.59 as Q32 — raw bits round(0.59 × 2^32) = 2_534_030_705
const Q_0_59: Q32 = Q32::from_raw(2_534_030_705_i64);
/// 0.6 as Q32 — raw bits round(0.6 × 2^32) = 2_576_980_378
const Q_0_6: Q32 = Q32::from_raw(2_576_980_378_i64);
/// 0.7 as Q32 — raw bits round(0.7 × 2^32) = 3_006_477_107
const Q_0_7: Q32 = Q32::from_raw(3_006_477_107_i64);
/// 0.8 as Q32 — raw bits round(0.8 × 2^32) = 3_435_973_837
const Q_0_8: Q32 = Q32::from_raw(3_435_973_837_i64);
/// 0.81 as Q32 — raw bits round(0.81 × 2^32) = 3_478_923_510
const Q_0_81: Q32 = Q32::from_raw(3_478_923_510_i64);
/// 0.82 as Q32 — raw bits round(0.82 × 2^32) = 3_521_873_183
const Q_0_82: Q32 = Q32::from_raw(3_521_873_183_i64);
/// 0.9 as Q32 — raw bits round(0.9 × 2^32) = 3_865_470_566
const Q_0_9: Q32 = Q32::from_raw(3_865_470_566_i64);

// ---------------------------------------------------------------------------
// Test fixture helpers
// ---------------------------------------------------------------------------

fn half() -> Q32 {
    Q_0_5
}

/// Build a minimal `PlayerBio` for test use.
fn make_player_bio_with_labels(labels: BTreeSet<PhenotypeLabelId>) -> PlayerBio {
    use fw_content::signature::RoleFamily;
    use fw_content::{
        AttackingRun, CommentaryHandles, CurvePoint, DefensiveShape, PLAYER_BIO_SCHEMA_VERSION,
        PlayingInstincts, PressingTrigger, PressureResponse,
    };
    use std::collections::BTreeMap;

    let h = half();
    let z = Q32::ZERO;

    PlayerBio {
        schema_version: PLAYER_BIO_SCHEMA_VERSION,
        player_id: "fwh.core:player_00001".to_string(),
        content_pack_version: "1.0.0".to_string(),
        display_name_full: "Test Player".to_string(),
        display_name_short: "T. Player".to_string(),
        role_family: RoleFamily::Striker,
        birth_region: "Testville".to_string(),
        playing_instincts: PlayingInstincts {
            defensive_shape_preference: DefensiveShape::Compact,
            attacking_run_preference: AttackingRun::InBehind,
            pressing_trigger: PressingTrigger::Aggressive,
            risk_appetite: h,
        },
        pressure_response: PressureResponse {
            stakes_to_performance_curve: vec![CurvePoint {
                stakes: z,
                performance_delta: z,
            }],
            composure_floor: h,
        },
        development_hooks: vec![],
        signature_candidates: vec![],
        scout_labels: labels,
        commentary_handles: CommentaryHandles {
            preferred_nouns: vec!["the player".to_string()],
            preferred_verbs: vec!["runs".to_string()],
        },
        rivalry_compatibility: BTreeMap::new(),
        alumni_of: vec![],
        tactical_dna_fragments: vec![],
        internal_gene_snapshot: GeneSnapshot {
            physical: PhysicalGenes {
                height_ceiling: h,
                frame_density: h,
                fast_twitch_ratio: h,
                stamina_recovery: h,
                growth_curve: z,
                aging_curve: h,
                injury_resilience: h,
            },
            mental: MentalGenes {
                pattern_recognition: h,
                composure_floor: h,
                decision_velocity: h,
                learning_rate: h,
                ambition: h,
                mentality: z,
            },
            technical: TechnicalAffinities {
                left_foot: z,
                aerial: h,
                dead_ball: h,
                striking: h,
                first_touch: h,
            },
            narrative_flags: BTreeSet::new(),
        },
    }
}

fn basic_scout() -> Scout {
    Scout::basic_uncertainty()
}

// ---------------------------------------------------------------------------
// AC: CategoryBiases validator
// ---------------------------------------------------------------------------

#[test]
fn category_biases_rejects_nonzero_narrative_flag() {
    // narrative_flag = 0.1 (any nonzero value must be rejected)
    let result = CategoryBiases::try_new(Q32::ZERO, Q32::ZERO, Q32::ZERO, Q_0_1);
    assert!(
        result.is_err(),
        "CategoryBiases::try_new must return Err when narrative_flag is nonzero"
    );
}

#[test]
fn category_biases_accepts_zero_narrative_flag() {
    let result = CategoryBiases::try_new(Q32::ZERO, Q32::ZERO, Q32::ZERO, Q32::ZERO);
    assert!(
        result.is_ok(),
        "CategoryBiases::try_new must return Ok when narrative_flag is zero"
    );
}

// ---------------------------------------------------------------------------
// AC: UncertaintyBand display labels
// ---------------------------------------------------------------------------

#[test]
fn uncertainty_band_display_labels_non_empty_and_unique() {
    let bands = [
        UncertaintyBand::Hunch,
        UncertaintyBand::Tentative,
        UncertaintyBand::Confident,
        UncertaintyBand::Settled,
    ];
    let mut seen = std::collections::BTreeSet::new();
    for band in &bands {
        let label = band.display_label();
        assert!(
            !label.is_empty(),
            "display_label for {band:?} must not be empty"
        );
        assert!(
            seen.insert(label),
            "display_label {label:?} is duplicated for {band:?}"
        );
    }
}

#[test]
fn uncertainty_band_from_confidence_boundary_cases() {
    // Thresholds from design/scouting.md §"T3 tuning seeds":
    //   < 0.35  → Hunch
    //   [0.35, 0.60) → Tentative
    //   [0.60, 0.82) → Confident
    //   >= 0.82 → Settled

    // Just below Hunch max (0.34)
    assert_eq!(
        UncertaintyBand::from_confidence(Q_0_34),
        UncertaintyBand::Hunch,
        "confidence 0.34 must be Hunch"
    );
    // At Tentative low boundary (0.35)
    assert_eq!(
        UncertaintyBand::from_confidence(Q_0_35),
        UncertaintyBand::Tentative,
        "confidence 0.35 must be Tentative"
    );
    // Just below Tentative max (0.59)
    assert_eq!(
        UncertaintyBand::from_confidence(Q_0_59),
        UncertaintyBand::Tentative,
        "confidence 0.59 must be Tentative"
    );
    // At Confident low boundary (0.60)
    assert_eq!(
        UncertaintyBand::from_confidence(Q_0_6),
        UncertaintyBand::Confident,
        "confidence 0.60 must be Confident"
    );
    // Just below Confident max (0.81)
    assert_eq!(
        UncertaintyBand::from_confidence(Q_0_81),
        UncertaintyBand::Confident,
        "confidence 0.81 must be Confident"
    );
    // At Settled boundary (0.82)
    assert_eq!(
        UncertaintyBand::from_confidence(Q_0_82),
        UncertaintyBand::Settled,
        "confidence 0.82 must be Settled"
    );
    // Zero → Hunch
    assert_eq!(
        UncertaintyBand::from_confidence(Q32::ZERO),
        UncertaintyBand::Hunch
    );
    // One → Settled
    assert_eq!(
        UncertaintyBand::from_confidence(Q32::ONE),
        UncertaintyBand::Settled
    );
}

// ---------------------------------------------------------------------------
// AC: observe_player determinism
// ---------------------------------------------------------------------------

#[test]
fn observe_player_is_deterministic() {
    let scout = basic_scout();
    let mut labels = BTreeSet::new();
    labels.insert(PhenotypeLabelId::PureFinisher);
    labels.insert(PhenotypeLabelId::Poacher);
    let bio = make_player_bio_with_labels(labels);
    let career_seed = 42_u64;
    let observation_id = 0_u32;

    let report_a = observe_player(&scout, &bio, career_seed, observation_id);
    let report_b = observe_player(&scout, &bio, career_seed, observation_id);
    assert_eq!(
        report_a, report_b,
        "observe_player with identical inputs must produce equal ScoutReports"
    );
}

#[test]
fn observe_player_different_career_seed_produces_different_report() {
    let scout = basic_scout();
    let mut labels = BTreeSet::new();
    labels.insert(PhenotypeLabelId::PureFinisher);
    labels.insert(PhenotypeLabelId::Poacher);
    labels.insert(PhenotypeLabelId::LateBloomer);
    let bio = make_player_bio_with_labels(labels);
    let observation_id = 0_u32;

    let report_a = observe_player(&scout, &bio, 1_u64, observation_id);
    let report_b = observe_player(&scout, &bio, 99999_u64, observation_id);

    // The reports must differ in at least the category estimates or label confidences.
    assert_ne!(
        report_a, report_b,
        "observe_player with different career_seed must produce different ScoutReports"
    );
}

// ---------------------------------------------------------------------------
// AC: report correctness
// ---------------------------------------------------------------------------

#[test]
fn observe_player_returns_exactly_three_category_estimates() {
    let scout = basic_scout();
    let bio = make_player_bio_with_labels(BTreeSet::new());
    let report = observe_player(&scout, &bio, 42, 0);
    assert_eq!(
        report.category_estimates.len(),
        3,
        "ScoutReport must have exactly 3 category_estimates"
    );
}

#[test]
fn observe_player_category_estimates_in_order_physical_mental_technical() {
    let scout = basic_scout();
    let bio = make_player_bio_with_labels(BTreeSet::new());
    let report = observe_player(&scout, &bio, 42, 0);
    assert_eq!(
        report.category_estimates[0].category,
        GeneCategory::Physical
    );
    assert_eq!(report.category_estimates[1].category, GeneCategory::Mental);
    assert_eq!(
        report.category_estimates[2].category,
        GeneCategory::Technical
    );
}

#[test]
fn observe_player_category_estimates_all_have_low_le_high() {
    let scout = basic_scout();
    let bio = make_player_bio_with_labels(BTreeSet::new());
    let report = observe_player(&scout, &bio, 42, 0);
    for est in &report.category_estimates {
        assert!(
            est.low <= est.high,
            "category estimate {:?}: low ({:?}) > high ({:?})",
            est.category,
            est.low,
            est.high
        );
    }
}

#[test]
fn observe_player_label_estimates_count_matches_scout_labels() {
    let scout = basic_scout();

    // With 3 labels
    let mut labels = BTreeSet::new();
    labels.insert(PhenotypeLabelId::PureFinisher);
    labels.insert(PhenotypeLabelId::Poacher);
    labels.insert(PhenotypeLabelId::LateBloomer);
    let bio3 = make_player_bio_with_labels(labels);
    let report3 = observe_player(&scout, &bio3, 42, 0);
    assert_eq!(
        report3.label_estimates.len(),
        3,
        "label_estimates count must match player's scout_labels count"
    );

    // With 0 labels
    let bio0 = make_player_bio_with_labels(BTreeSet::new());
    let report0 = observe_player(&scout, &bio0, 42, 0);
    assert_eq!(
        report0.label_estimates.len(),
        0,
        "label_estimates must be empty when player has no scout_labels"
    );
}

#[test]
fn observe_player_no_labels_falls_back_to_half_confidence() {
    let scout = basic_scout();
    let bio = make_player_bio_with_labels(BTreeSet::new());
    let report = observe_player(&scout, &bio, 42, 0);
    // When no labels, overall confidence must be 0.5
    assert_eq!(
        report.confidence, Q_0_5,
        "report confidence must be 0.5 when player has no scout labels"
    );
}

// ---------------------------------------------------------------------------
// AC: serde round-trip
// ---------------------------------------------------------------------------

#[test]
fn scout_report_ron_round_trip() {
    let scout = basic_scout();
    let mut labels = BTreeSet::new();
    labels.insert(PhenotypeLabelId::PureFinisher);
    labels.insert(PhenotypeLabelId::Poacher);
    let bio = make_player_bio_with_labels(labels);
    let report = observe_player(&scout, &bio, 42, 0);

    let encoded = ron::ser::to_string(&report).expect("RON encode must succeed");
    let decoded: ScoutReport = ron::de::from_str(&encoded).expect("RON decode must succeed");
    assert_eq!(
        decoded, report,
        "ScoutReport must survive a RON round-trip bit-identical"
    );
}

#[test]
fn scout_report_all_sub_types_populated_round_trip() {
    // Construct a ScoutReport with all fields manually populated and round-trip it.
    let report = ScoutReport {
        scout_archetype_id: "fwh.core:scout.basic-uncertainty".to_string(),
        player_id: "fwh.core:player_00001".to_string(),
        confidence: Q_0_7,
        label_estimates: vec![
            LabelEstimate {
                label: PhenotypeLabelId::PureFinisher,
                confidence: Q_0_8,
            },
            LabelEstimate {
                label: PhenotypeLabelId::Poacher,
                confidence: Q_0_6,
            },
        ],
        category_estimates: vec![
            GeneCategoryEstimate {
                category: GeneCategory::Physical,
                low: Q_0_3,
                high: Q_0_7,
            },
            GeneCategoryEstimate {
                category: GeneCategory::Mental,
                low: Q_0_4,
                high: Q_0_6,
            },
            GeneCategoryEstimate {
                category: GeneCategory::Technical,
                low: Q_0_2,
                high: Q_0_8,
            },
        ],
    };

    let encoded = ron::ser::to_string(&report).expect("RON encode");
    let decoded: ScoutReport = ron::de::from_str(&encoded).expect("RON decode");
    assert_eq!(decoded, report);
}

// ---------------------------------------------------------------------------
// AC: GeneCategoryEstimate::band
// ---------------------------------------------------------------------------

#[test]
fn gene_category_estimate_band_wide_range_gives_hunch() {
    // width = 0.8 → effective_confidence = 0.2 → Hunch
    let est = GeneCategoryEstimate {
        category: GeneCategory::Physical,
        low: Q_0_1,
        high: Q_0_9,
    };
    assert_eq!(est.band(), UncertaintyBand::Hunch);
}

#[test]
fn gene_category_estimate_band_narrow_range_gives_settled() {
    // width = 0.05 → effective_confidence = 0.95 → Settled
    let est = GeneCategoryEstimate {
        category: GeneCategory::Mental,
        low: Q_0_45,
        high: Q_0_5,
    };
    assert_eq!(est.band(), UncertaintyBand::Settled);
}

#[test]
fn gene_category_estimate_band_zero_width_is_settled() {
    // zero width → effective_confidence = 1.0 → Settled
    let est = GeneCategoryEstimate {
        category: GeneCategory::Technical,
        low: half(),
        high: half(),
    };
    assert_eq!(est.band(), UncertaintyBand::Settled);
}

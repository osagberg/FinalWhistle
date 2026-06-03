//! Proptest invariants + insta snapshot for `observe_player` (Sim/RULES.md §8).
//!
//! Invariants verified:
//! - Every report has exactly 3 category_estimates.
//! - Each category_estimate: `low <= high`, both in `[0, 1]`.
//! - `confidence` is in `[0, 1]`.
//! - `label_estimates.len()` == `player.scout_labels.len()`.
//!
//! Snapshot pins the exact output of `observe_player` for a fixed
//! `(Scout::basic_uncertainty(), fixture_bio, career_seed=42, observation_id=0)`.

use std::collections::BTreeSet;

use fw_content::{
    GeneSnapshot, MentalGenes, PhenotypeLabelId, PhysicalGenes, PlayerBio, TechnicalAffinities,
};
use fw_core::{PlayerId, Q32};
use fw_scouting::{Scout, observe_player};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Shared fixture
// ---------------------------------------------------------------------------

fn half() -> Q32 {
    Q32::from_raw(2_147_483_648_i64) // 0.5
}

/// Fixed `PlayerBio` used for both proptest and the insta snapshot.
///
/// `scout_labels` is passed in so proptest can vary it (zero labels is valid).
fn make_fixture_bio(labels: BTreeSet<PhenotypeLabelId>) -> PlayerBio {
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

// ---------------------------------------------------------------------------
// Insta snapshot
// ---------------------------------------------------------------------------

#[test]
fn observe_player_snapshot_basic_uncertainty_seed42() {
    let scout = Scout::basic_uncertainty();
    let bio = make_fixture_bio(BTreeSet::new());
    // subject=PlayerId(1) — stable fixture value; fixes F2 (site was hardcoded 0).
    let report = observe_player(&scout, &bio, 42, 0, PlayerId::new(1));
    insta::assert_ron_snapshot!("observe_player_basic_uncertainty_seed42", report);
}

// ---------------------------------------------------------------------------
// Proptest invariants
// ---------------------------------------------------------------------------

/// A small subset of `PhenotypeLabelId` variants for proptest strategies.
///
/// We use a fixed list rather than trying to enumerate all 46 variants via
/// proptest: the property we are testing is structural (counts, bounds), not
/// label-identity dependent. Two labels suffice to exercise the non-empty path.
fn label_strategy() -> impl Strategy<Value = BTreeSet<PhenotypeLabelId>> {
    prop::collection::btree_set(
        prop_oneof![
            Just(PhenotypeLabelId::PureFinisher),
            Just(PhenotypeLabelId::Poacher),
            Just(PhenotypeLabelId::LateBloomer),
        ],
        0..=3,
    )
}

proptest! {
    #[test]
    fn observe_player_structural_invariants(
        career_seed: u64,
        observation_id: u32,
        subject_raw: u32,
        labels in label_strategy(),
    ) {
        let scout = Scout::basic_uncertainty();
        let label_count = labels.len();
        let bio = make_fixture_bio(labels);
        let subject = PlayerId::new(subject_raw);
        let report = observe_player(&scout, &bio, career_seed, observation_id, subject);

        // Exactly 3 category estimates.
        prop_assert_eq!(
            report.category_estimates.len(),
            3,
            "expected 3 category_estimates, got {}",
            report.category_estimates.len()
        );

        // Each estimate: low <= high, both in [0, 1].
        for est in &report.category_estimates {
            prop_assert!(
                est.low >= Q32::ZERO,
                "category {:?}: low {:?} < 0",
                est.category, est.low
            );
            prop_assert!(
                est.high <= Q32::ONE,
                "category {:?}: high {:?} > 1",
                est.category, est.high
            );
            prop_assert!(
                est.low <= est.high,
                "category {:?}: low {:?} > high {:?}",
                est.category, est.low, est.high
            );
        }

        // Overall confidence in [0, 1].
        prop_assert!(
            report.confidence >= Q32::ZERO,
            "confidence {:?} < 0",
            report.confidence
        );
        prop_assert!(
            report.confidence <= Q32::ONE,
            "confidence {:?} > 1",
            report.confidence
        );

        // label_estimates count == player's scout_labels count.
        prop_assert_eq!(
            report.label_estimates.len(),
            label_count,
            "label_estimates count {} != scout_labels count {}",
            report.label_estimates.len(),
            label_count
        );
    }
}

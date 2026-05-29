//! Gene→family PA/CA bridge.
//!
//! Implements the formula from `docs/design/progression.md` §"Gene→family
//! PA/CA bridge (T4-2.5a)" verbatim. Given a player's `GeneSnapshot` and
//! their `AbilityCeiling`, produces per-family PA and CA estimates on the
//! 1..=200 scale used by the breakthrough meter.
//!
//! ## Why this lives in fw-content
//!
//! `GeneSnapshot` is an fw-content type. `AbilityCeiling` is fw-core.
//! `AttributeFamily` is now fw-core. All three dependencies are available
//! here without creating a crate cycle.
//!
//! ## Determinism
//!
//! - Pure function: same (genes, ceiling) → same output on every platform.
//! - Q32 fixed-point only. No `f32`/`f64`.
//! - No RNG. No clocks.
//! - `BTreeMap` for output (Sim/RULES §2).

use std::collections::BTreeMap;

use fw_core::{AbilityCeiling, AttributeFamily, Q32};

use crate::gene::GeneSnapshot;

// -------------------------------------------------------------------------
// Compile-time weight constants — round(w × 2^32)
// Verified against progression.md §"Anchor table".
// -------------------------------------------------------------------------

// Blending dials
/// GENE_WEIGHT = 0.75; raw bits = round(0.75 × 2^32) = 3_221_225_472
const GENE_WEIGHT: Q32 = Q32::from_raw(3_221_225_472_i64);
/// CEIL_WEIGHT = 0.25; raw bits = round(0.25 × 2^32) = 1_073_741_824
const CEIL_WEIGHT: Q32 = Q32::from_raw(1_073_741_824_i64);

// Finishing: striking(0.45), first_touch(0.35), fast_twitch_ratio(0.20)
const W_FINISHING_STRIKING: Q32 = Q32::from_raw(1_932_735_283_i64); // 0.45
const W_FINISHING_FIRST_TOUCH: Q32 = Q32::from_raw(1_503_238_554_i64); // 0.35
const W_FINISHING_FAST_TWITCH: Q32 = Q32::from_raw(858_993_459_i64); // 0.20

// Passing: pattern_recognition(0.40), decision_velocity(0.35), first_touch(0.25)
const W_PASSING_PATTERN: Q32 = Q32::from_raw(1_717_986_918_i64); // 0.40
const W_PASSING_DECISION: Q32 = Q32::from_raw(1_503_238_554_i64); // 0.35
const W_PASSING_FIRST_TOUCH: Q32 = Q32::from_raw(1_073_741_824_i64); // 0.25

// DefensiveAnticipation: pattern_recognition(0.50), decision_velocity(0.30), composure_floor(0.20)
const W_DEFANT_PATTERN: Q32 = Q32::from_raw(2_147_483_648_i64); // 0.50
const W_DEFANT_DECISION: Q32 = Q32::from_raw(1_288_490_189_i64); // 0.30
const W_DEFANT_COMPOSURE: Q32 = Q32::from_raw(858_993_459_i64); // 0.20

// AerialPresence: aerial(0.50), height_ceiling(0.30), frame_density(0.20)
const W_AERIAL_AERIAL: Q32 = Q32::from_raw(2_147_483_648_i64); // 0.50
const W_AERIAL_HEIGHT: Q32 = Q32::from_raw(1_288_490_189_i64); // 0.30
const W_AERIAL_FRAME: Q32 = Q32::from_raw(858_993_459_i64); // 0.20

// Composure: composure_floor(0.45), mentality*(0.35), ambition(0.20)
const W_COMPOSURE_FLOOR: Q32 = Q32::from_raw(1_932_735_283_i64); // 0.45
const W_COMPOSURE_MENTALITY: Q32 = Q32::from_raw(1_503_238_554_i64); // 0.35
const W_COMPOSURE_AMBITION: Q32 = Q32::from_raw(858_993_459_i64); // 0.20

// Pace: fast_twitch_ratio(0.60), growth_curve*(0.40)
const W_PACE_FAST_TWITCH: Q32 = Q32::from_raw(2_576_980_378_i64); // 0.60
const W_PACE_GROWTH: Q32 = Q32::from_raw(1_717_986_918_i64); // 0.40

// Stamina: stamina_recovery(0.45), aging_curve(0.35), injury_resilience(0.20)
const W_STAMINA_RECOVERY: Q32 = Q32::from_raw(1_932_735_283_i64); // 0.45
const W_STAMINA_AGING: Q32 = Q32::from_raw(1_503_238_554_i64); // 0.35
const W_STAMINA_INJURY: Q32 = Q32::from_raw(858_993_459_i64); // 0.20

// WorkRate: ambition(0.40), learning_rate(0.35), mentality†(0.25)
const W_WORKRATE_AMBITION: Q32 = Q32::from_raw(1_717_986_918_i64); // 0.40
const W_WORKRATE_LEARNING: Q32 = Q32::from_raw(1_503_238_554_i64); // 0.35
const W_WORKRATE_MENTALITY: Q32 = Q32::from_raw(1_073_741_824_i64); // 0.25

// DeadBallDelivery: dead_ball(0.55), first_touch(0.25), left_foot(0.20)
const W_DEADBALL_DEAD_BALL: Q32 = Q32::from_raw(2_362_232_013_i64); // 0.55
const W_DEADBALL_FIRST_TOUCH: Q32 = Q32::from_raw(1_073_741_824_i64); // 0.25
const W_DEADBALL_LEFT_FOOT: Q32 = Q32::from_raw(858_993_459_i64); // 0.20

// Leadership: mentality*(0.50), composure_floor(0.30), ambition(0.20)
const W_LEADERSHIP_MENTALITY: Q32 = Q32::from_raw(2_147_483_648_i64); // 0.50
const W_LEADERSHIP_COMPOSURE: Q32 = Q32::from_raw(1_288_490_189_i64); // 0.30
const W_LEADERSHIP_AMBITION: Q32 = Q32::from_raw(858_993_459_i64); // 0.20

// -------------------------------------------------------------------------
// Signed-field normalization helpers
// -------------------------------------------------------------------------

/// Standard normalization (`*`): maps signed Q32 ∈ [−1,+1] → [0,1].
/// `norm = (gene + Q32::ONE) >> 1`
///
/// Exact in Q32: add ONE → [0,2]; right-shift 1 bit → [0,1].
/// Used for: `mentality` (Composure, Leadership, Pace) and `growth_curve` (Pace).
#[inline]
fn norm_signed_standard(gene: Q32) -> Q32 {
    // (gene + ONE) is in [0, 2]; raw bits in [0, 2^33].
    // >> 1 on the raw bits halves it → [0, 2^32] = [0, Q32::ONE].
    Q32::from_raw((gene.to_bits() + Q32::ONE.to_bits()) >> 1)
}

/// Inverted normalization (`†`): maps signed Q32 ∈ [−1,+1] → [1,0].
/// `norm = Q32::ONE − norm_signed_standard(gene)`
///
/// Used for: `mentality` in WorkRate only (introvert-grinder pole → 1.0).
#[inline]
fn norm_signed_inverted(gene: Q32) -> Q32 {
    Q32::ONE - norm_signed_standard(gene)
}

// -------------------------------------------------------------------------
// Integer extraction from Q32
// -------------------------------------------------------------------------

/// Extract the integer part (floor toward −∞) of a Q32 value.
///
/// Q32.32 raw bits: top 32 bits are the signed integer part.
/// Right-shift by 32 gives the floor integer. Does NOT clamp — callers are
/// responsible for keeping values in a range that fits i32 (values in
/// [0, 200+ε] always fit; out-of-range inputs are the caller's bug).
#[inline]
fn q32_to_int(v: Q32) -> i32 {
    (v.to_bits() >> 32) as i32
}

// -------------------------------------------------------------------------
// Per-family gene score
// -------------------------------------------------------------------------

/// Compute the weighted gene score for a single family.
/// All anchor fields must already be in [0,1] (unit range or normalized).
/// Result is Q32 ∈ [0,1].
fn gene_score_finishing(g: &GeneSnapshot) -> Q32 {
    W_FINISHING_STRIKING * g.technical.striking
        + W_FINISHING_FIRST_TOUCH * g.technical.first_touch
        + W_FINISHING_FAST_TWITCH * g.physical.fast_twitch_ratio
}

fn gene_score_passing(g: &GeneSnapshot) -> Q32 {
    W_PASSING_PATTERN * g.mental.pattern_recognition
        + W_PASSING_DECISION * g.mental.decision_velocity
        + W_PASSING_FIRST_TOUCH * g.technical.first_touch
}

fn gene_score_defensive_anticipation(g: &GeneSnapshot) -> Q32 {
    W_DEFANT_PATTERN * g.mental.pattern_recognition
        + W_DEFANT_DECISION * g.mental.decision_velocity
        + W_DEFANT_COMPOSURE * g.mental.composure_floor
}

fn gene_score_aerial_presence(g: &GeneSnapshot) -> Q32 {
    W_AERIAL_AERIAL * g.technical.aerial
        + W_AERIAL_HEIGHT * g.physical.height_ceiling
        + W_AERIAL_FRAME * g.physical.frame_density
}

fn gene_score_composure(g: &GeneSnapshot) -> Q32 {
    let mentality_normed = norm_signed_standard(g.mental.mentality);
    W_COMPOSURE_FLOOR * g.mental.composure_floor
        + W_COMPOSURE_MENTALITY * mentality_normed
        + W_COMPOSURE_AMBITION * g.mental.ambition
}

fn gene_score_pace(g: &GeneSnapshot) -> Q32 {
    let growth_normed = norm_signed_standard(g.physical.growth_curve);
    W_PACE_FAST_TWITCH * g.physical.fast_twitch_ratio + W_PACE_GROWTH * growth_normed
}

fn gene_score_stamina(g: &GeneSnapshot) -> Q32 {
    W_STAMINA_RECOVERY * g.physical.stamina_recovery
        + W_STAMINA_AGING * g.physical.aging_curve
        + W_STAMINA_INJURY * g.physical.injury_resilience
}

fn gene_score_work_rate(g: &GeneSnapshot) -> Q32 {
    let mentality_normed = norm_signed_inverted(g.mental.mentality);
    W_WORKRATE_AMBITION * g.mental.ambition
        + W_WORKRATE_LEARNING * g.mental.learning_rate
        + W_WORKRATE_MENTALITY * mentality_normed
}

fn gene_score_dead_ball(g: &GeneSnapshot) -> Q32 {
    W_DEADBALL_DEAD_BALL * g.technical.dead_ball
        + W_DEADBALL_FIRST_TOUCH * g.technical.first_touch
        + W_DEADBALL_LEFT_FOOT * g.technical.left_foot
}

fn gene_score_leadership(g: &GeneSnapshot) -> Q32 {
    let mentality_normed = norm_signed_standard(g.mental.mentality);
    W_LEADERSHIP_MENTALITY * mentality_normed
        + W_LEADERSHIP_COMPOSURE * g.mental.composure_floor
        + W_LEADERSHIP_AMBITION * g.mental.ambition
}

// -------------------------------------------------------------------------
// PA/CA derivation helpers
// -------------------------------------------------------------------------

/// Compute PA for a single family from gene_score and ceiling.potential.
/// Returns i16 in 1..=200.
fn family_pa(gene_score: Q32, potential: Q32) -> i16 {
    let pa_score = GENE_WEIGHT * gene_score + CEIL_WEIGHT * potential;
    // Maps [0,1] → [1,200]: pa_score × 199 + 1
    let pa_raw = pa_score * Q32::from_int(199) + Q32::ONE;
    q32_to_int(pa_raw).clamp(1, 200) as i16
}

/// Compute CA for a single family from gene_score, ceiling.current/potential,
/// and the already-computed pa_i16. Returns i16 in 1..=pa_i16.
fn family_ca(gene_score: Q32, ceiling: AbilityCeiling, pa_i16: i16) -> i16 {
    // realized_fraction = current / potential; 0 if potential == 0.
    let realized_fraction = if ceiling.potential() == Q32::ZERO {
        Q32::ZERO
    } else {
        // Q32 / Q32 uses 128-bit intermediate internally (checked_div),
        // panics only on divide-by-zero — guarded above.
        ceiling.current() / ceiling.potential()
    };

    let ca_score = gene_score * realized_fraction;
    let ca_raw = ca_score * Q32::from_int(199) + Q32::ONE;
    let ca_i16 = q32_to_int(ca_raw).clamp(1, 200) as i16;
    ca_i16.min(pa_i16)
}

// -------------------------------------------------------------------------
// Public API
// -------------------------------------------------------------------------

/// Per-family PA and CA estimates on the 1..=200 scale.
///
/// Returned by `gene_family_pa_ca`. Named fields prevent silent pa/ca swap
/// at call sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyPaCa {
    /// Per-family Potential Ability on the 1..=200 scale.
    pub pa: BTreeMap<AttributeFamily, i16>,
    /// Per-family Current Ability on the 1..=200 scale. Guaranteed `ca ≤ pa`.
    pub ca: BTreeMap<AttributeFamily, i16>,
}

/// Compute per-family PA and CA estimates from a player's genes and ceiling.
///
/// All 10 families are always present in `FamilyPaCa.pa` and `FamilyPaCa.ca`.
/// Values are on the 1..=200 scale. Guaranteed: `1 ≤ ca ≤ pa ≤ 200`.
///
/// Pure function: same (genes, ceiling) → same output everywhere. No RNG,
/// no clocks, Q32-only arithmetic, BTreeMap output.
///
/// # Preconditions
///
/// Callers pass validated genes: unit-range fields in [0, 1], signed fields
/// in [−1, +1]. Out-of-range input either silently clamps (if the Q32
/// arithmetic stays representable) or panics on Q32 overflow. Gene validation
/// belongs upstream at generation (T4-2.5b); this function does not validate.
///
/// ## Formula
///
/// See `docs/design/progression.md` §"Gene→family PA/CA bridge (T4-2.5a)".
#[must_use]
pub fn gene_family_pa_ca(genes: &GeneSnapshot, ceiling: AbilityCeiling) -> FamilyPaCa {
    let scores: [(AttributeFamily, Q32); 10] = [
        (AttributeFamily::Finishing, gene_score_finishing(genes)),
        (AttributeFamily::Passing, gene_score_passing(genes)),
        (
            AttributeFamily::DefensiveAnticipation,
            gene_score_defensive_anticipation(genes),
        ),
        (
            AttributeFamily::AerialPresence,
            gene_score_aerial_presence(genes),
        ),
        (AttributeFamily::Composure, gene_score_composure(genes)),
        (AttributeFamily::Pace, gene_score_pace(genes)),
        (AttributeFamily::Stamina, gene_score_stamina(genes)),
        (AttributeFamily::WorkRate, gene_score_work_rate(genes)),
        (
            AttributeFamily::DeadBallDelivery,
            gene_score_dead_ball(genes),
        ),
        (AttributeFamily::Leadership, gene_score_leadership(genes)),
    ];

    let mut pa = BTreeMap::new();
    let mut ca = BTreeMap::new();

    for (family, gene_score) in scores {
        let pa_val = family_pa(gene_score, ceiling.potential());
        let ca_val = family_ca(gene_score, ceiling, pa_val);
        pa.insert(family, pa_val);
        ca.insert(family, ca_val);
    }

    FamilyPaCa { pa, ca }
}

/// Clamp a per-family PA/CA breakthrough or regressive delta to the 1..=200
/// scale, preserving `ca ≤ pa`.
///
/// `delta_pa` and `delta_ca` are signed integers (positive = breakthrough
/// uplift, negative = regressive collapse). Returns `(new_pa, new_ca)` both
/// clamped to `1..=200` with `new_ca ≤ new_pa`.
///
/// The actual `AbilityCeiling` write (`redraw_ceiling`) happens in the career
/// loop at T4-2.5d, where `redraw_ceiling` is reachable inside `fw-core`.
/// This function operates purely on the per-family integer scale.
#[must_use]
pub fn apply_family_delta(pa_i16: i16, ca_i16: i16, delta_pa: i16, delta_ca: i16) -> (i16, i16) {
    let new_pa = (pa_i16 + delta_pa).clamp(1, 200);
    let new_ca = (ca_i16 + delta_ca).clamp(1, 200).min(new_pa);
    (new_pa, new_ca)
}

// -------------------------------------------------------------------------
// Tests (TDD: these were written before the implementation above)
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::gene::{MentalGenes, PhysicalGenes, TechnicalAffinities};

    // Utility: build a GeneSnapshot from f64-approximated values (test-only,
    // uses Q32::from_f64_clamped which is bake-time only but fine in tests).
    fn q(v: f64) -> Q32 {
        Q32::from_f64_clamped(v)
    }

    fn zero_genes() -> GeneSnapshot {
        GeneSnapshot {
            physical: PhysicalGenes {
                height_ceiling: Q32::ZERO,
                frame_density: Q32::ZERO,
                fast_twitch_ratio: Q32::ZERO,
                stamina_recovery: Q32::ZERO,
                growth_curve: Q32::ZERO, // 0 in [-1,1], normalized to 0.5
                aging_curve: Q32::ZERO,
                injury_resilience: Q32::ZERO,
            },
            mental: MentalGenes {
                pattern_recognition: Q32::ZERO,
                composure_floor: Q32::ZERO,
                decision_velocity: Q32::ZERO,
                learning_rate: Q32::ZERO,
                ambition: Q32::ZERO,
                mentality: Q32::ZERO, // 0 in [-1,1], normalized to 0.5
            },
            technical: TechnicalAffinities {
                left_foot: Q32::ZERO,
                aerial: Q32::ZERO,
                dead_ball: Q32::ZERO,
                striking: Q32::ZERO,
                first_touch: Q32::ZERO,
            },
            narrative_flags: BTreeSet::new(),
        }
    }

    fn max_genes() -> GeneSnapshot {
        GeneSnapshot {
            physical: PhysicalGenes {
                height_ceiling: Q32::ONE,
                frame_density: Q32::ONE,
                fast_twitch_ratio: Q32::ONE,
                stamina_recovery: Q32::ONE,
                growth_curve: Q32::ONE, // +1 in [-1,1], normalized to 1.0
                aging_curve: Q32::ONE,
                injury_resilience: Q32::ONE,
            },
            mental: MentalGenes {
                pattern_recognition: Q32::ONE,
                composure_floor: Q32::ONE,
                decision_velocity: Q32::ONE,
                learning_rate: Q32::ONE,
                ambition: Q32::ONE,
                mentality: Q32::ONE, // +1 in [-1,1], normalized to 1.0
            },
            technical: TechnicalAffinities {
                left_foot: Q32::ONE,
                aerial: Q32::ONE,
                dead_ball: Q32::ONE,
                striking: Q32::ONE,
                first_touch: Q32::ONE,
            },
            narrative_flags: BTreeSet::new(),
        }
    }

    fn mid_ceiling() -> AbilityCeiling {
        // potential=0.5, current=0.25
        AbilityCeiling::try_new(q(0.25), q(0.5)).expect("valid ceiling")
    }

    fn max_ceiling() -> AbilityCeiling {
        AbilityCeiling::try_new(Q32::ONE, Q32::ONE).expect("valid ceiling")
    }

    fn zero_ceiling() -> AbilityCeiling {
        // Pathological: potential=0. PA=1, CA=1.
        AbilityCeiling::try_new(Q32::ZERO, Q32::ZERO).expect("valid ceiling")
    }

    // -----------------------------------------------------------------------
    // RED test 1: all 10 families always present
    // -----------------------------------------------------------------------

    #[test]
    fn all_ten_families_always_emitted() {
        let FamilyPaCa { pa, ca } = gene_family_pa_ca(&mid_genes(), mid_ceiling());
        assert_eq!(pa.len(), 10, "PA map must have 10 families");
        assert_eq!(ca.len(), 10, "CA map must have 10 families");
        for family in AttributeFamily::ALL {
            assert!(pa.contains_key(&family), "PA missing {family:?}");
            assert!(ca.contains_key(&family), "CA missing {family:?}");
        }
    }

    fn mid_genes() -> GeneSnapshot {
        // All unit fields at 0.5; signed fields at 0 (midpoint).
        let half = q(0.5);
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

    // -----------------------------------------------------------------------
    // RED test 2: zero unit genes + mid ceiling → PA/CA always ≥ 1
    // Uses mid_ceiling (not zero_ceiling) because the floor=1 boundary is
    // tested more precisely by zero_genes_zero_ceiling_finishing_gives_one.
    // This test guards the general "no zero output with a non-zero ceiling"
    // property; signed-field families produce PA > 1 here because
    // mentality=0 / growth_curve=0 normalize to 0.5.
    // -----------------------------------------------------------------------

    #[test]
    fn zero_unit_genes_mid_ceiling_pa_ca_at_least_one() {
        // Unit-range anchors = 0; signed anchors (mentality=0, growth_curve=0)
        // normalize to 0.5 (midpoint), so gene_score > 0 for families that
        // use them. PA is always ≥ 1.
        let FamilyPaCa { pa, ca } = gene_family_pa_ca(&zero_genes(), mid_ceiling());
        for family in AttributeFamily::ALL {
            let p = *pa.get(&family).unwrap();
            let c = *ca.get(&family).unwrap();
            assert!(p >= 1, "PA for {family:?} must be ≥ 1, got {p}");
            assert!(c >= 1, "CA for {family:?} must be ≥ 1, got {c}");
        }
    }

    // -----------------------------------------------------------------------
    // RED test 3: families with ONLY unit-range anchors → PA=1 when all
    // unit anchors are zero AND ceiling.potential=0.
    // Finishing (no signed anchors): PA = pa_score×199+1 with pa_score=0 → PA=1.
    // -----------------------------------------------------------------------

    #[test]
    fn zero_genes_zero_ceiling_finishing_gives_one() {
        // Finishing anchors: striking(unit), first_touch(unit), fast_twitch_ratio(unit)
        // All zero + zero ceiling → gene_score=0, pa_score=0 → PA=1.
        let FamilyPaCa { pa, ca } = gene_family_pa_ca(&zero_genes(), zero_ceiling());
        let p = *pa.get(&AttributeFamily::Finishing).unwrap();
        let c = *ca.get(&AttributeFamily::Finishing).unwrap();
        assert_eq!(
            p, 1,
            "Finishing PA must be 1 with zero unit anchors + zero ceiling"
        );
        assert_eq!(
            c, 1,
            "Finishing CA must be 1 with zero unit anchors + zero ceiling"
        );
    }

    // -----------------------------------------------------------------------
    // RED test 4: max genes for a family with no †-inverted anchors → PA=200
    // Finishing uses only unit-range anchors; all at 1.0 + max ceiling → PA=200.
    // -----------------------------------------------------------------------

    #[test]
    fn max_genes_max_ceiling_finishing_caps_at_200() {
        let FamilyPaCa { pa, ca } = gene_family_pa_ca(&max_genes(), max_ceiling());
        // Finishing: striking(0.45)×1 + first_touch(0.35)×1 + fast_twitch(0.20)×1 = 1.0
        // pa_score = 1×0.75 + 1×0.25 = 1.0 → pa_raw = 1×199+1 = 200. PA=200.
        let p = *pa.get(&AttributeFamily::Finishing).unwrap();
        let c = *ca.get(&AttributeFamily::Finishing).unwrap();
        assert_eq!(p, 200, "Finishing PA must be 200 at max");
        assert_eq!(c, 200, "Finishing CA must be 200 at max");
    }

    // WorkRate with max extrovert genes (mentality=+1): inverted mentality → 0
    // contribution. PA < 200 is CORRECT per spec (WorkRate rewards introverts).
    #[test]
    fn workrate_with_max_extrovert_not_200() {
        // WorkRate uses †-inverted mentality: mentality=+1 → norm_inv=0.
        // gene_score = 0.40×ambition + 0.35×learning_rate + 0.25×0 = 0.75 at max.
        // pa_score = 0.75×0.75 + 1×0.25 = 0.8125 → PA≈162 (not 200).
        let FamilyPaCa { pa, ca: _ } = gene_family_pa_ca(&max_genes(), max_ceiling());
        let p = *pa.get(&AttributeFamily::WorkRate).unwrap();
        assert!(
            p < 200,
            "WorkRate PA with max extrovert should be < 200 (inverted mentality)"
        );
        assert!(p >= 1, "WorkRate PA must still be ≥ 1");
    }

    // All 10 families: values in 1..=200 and ca ≤ pa, regardless of gene combination.
    #[test]
    fn all_families_in_range_for_max_and_zero_genes() {
        for genes in [zero_genes(), max_genes()] {
            for ceiling in [zero_ceiling(), mid_ceiling(), max_ceiling()] {
                let FamilyPaCa { pa, ca } = gene_family_pa_ca(&genes, ceiling);
                for family in AttributeFamily::ALL {
                    let p = *pa.get(&family).unwrap();
                    let c = *ca.get(&family).unwrap();
                    assert!(
                        (1..=200).contains(&p),
                        "PA out of range for {family:?}: {p}"
                    );
                    assert!(
                        (1..=200).contains(&c),
                        "CA out of range for {family:?}: {c}"
                    );
                    assert!(c <= p, "CA ({c}) > PA ({p}) for {family:?}");
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // RED test 5: ca ≤ pa invariant always holds
    // -----------------------------------------------------------------------

    #[test]
    fn ca_never_exceeds_pa() {
        for genes in [zero_genes(), mid_genes(), max_genes()] {
            for ceiling in [zero_ceiling(), mid_ceiling(), max_ceiling()] {
                let FamilyPaCa { pa, ca } = gene_family_pa_ca(&genes, ceiling);
                for family in AttributeFamily::ALL {
                    let p = *pa.get(&family).unwrap();
                    let c = *ca.get(&family).unwrap();
                    assert!(
                        c <= p,
                        "CA ({c}) > PA ({p}) for {family:?}; violates invariant"
                    );
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // RED test 6: Finishing worked example — PA ≈ 112±1, CA ≈ 64±1
    // From progression.md §"Worked example — Finishing, mid-range striker":
    // striking=0.60, first_touch=0.45, fast_twitch_ratio=0.60
    // potential=0.60, current=0.35
    // -----------------------------------------------------------------------

    #[test]
    fn finishing_worked_example_within_one() {
        let genes = GeneSnapshot {
            physical: PhysicalGenes {
                height_ceiling: q(0.5),
                frame_density: q(0.5),
                fast_twitch_ratio: q(0.60),
                stamina_recovery: q(0.5),
                growth_curve: Q32::ZERO,
                aging_curve: q(0.5),
                injury_resilience: q(0.5),
            },
            mental: MentalGenes {
                pattern_recognition: q(0.5),
                composure_floor: q(0.5),
                decision_velocity: q(0.5),
                learning_rate: q(0.5),
                ambition: q(0.5),
                mentality: Q32::ZERO,
            },
            technical: TechnicalAffinities {
                left_foot: Q32::ZERO,
                aerial: q(0.5),
                dead_ball: q(0.5),
                striking: q(0.60),
                first_touch: q(0.45),
            },
            narrative_flags: BTreeSet::new(),
        };
        let ceiling = AbilityCeiling::try_new(q(0.35), q(0.60)).expect("valid ceiling");

        let result = gene_family_pa_ca(&genes, ceiling);
        let pa_finishing = *result.pa.get(&AttributeFamily::Finishing).unwrap();
        let ca_finishing = *result.ca.get(&AttributeFamily::Finishing).unwrap();

        // progression.md expects PA≈112, CA≈64 (±1 for Q32 truncation)
        assert!(
            (111..=113).contains(&pa_finishing),
            "Finishing PA expected 111-113, got {pa_finishing}"
        );
        assert!(
            (63..=65).contains(&ca_finishing),
            "Finishing CA expected 63-65, got {ca_finishing}"
        );
    }

    // -----------------------------------------------------------------------
    // RED test 7: apply_family_delta clamping and ca ≤ pa invariant
    // -----------------------------------------------------------------------

    #[test]
    fn delta_apply_normal_uplift() {
        // 112 + 5 = 117, 64 + 3 = 67. Both in range; ca (67) ≤ pa (117).
        let (new_pa, new_ca) = apply_family_delta(112, 64, 5, 3);
        assert_eq!(new_pa, 117);
        assert_eq!(new_ca, 67);
        assert!(new_ca <= new_pa);
    }

    #[test]
    fn delta_apply_overflow_clamps_to_200() {
        // 198 + 10 = 208 → clamps to 200.
        let (new_pa, new_ca) = apply_family_delta(198, 198, 10, 10);
        assert_eq!(new_pa, 200);
        assert_eq!(new_ca, 200);
    }

    #[test]
    fn delta_apply_underflow_clamps_to_one() {
        // 5 - 20 = -15 → clamps to 1.
        let (new_pa, new_ca) = apply_family_delta(5, 3, -20, -20);
        assert_eq!(new_pa, 1);
        assert_eq!(new_ca, 1);
    }

    #[test]
    fn delta_apply_ca_clamped_to_pa_when_delta_ca_would_exceed() {
        // If ca delta overshoots pa: ca is clamped down to new_pa.
        // new_pa = 100 + 5 = 105. new_ca raw = 90 + 20 = 110 → clamped to 105.
        let (new_pa, new_ca) = apply_family_delta(100, 90, 5, 20);
        assert_eq!(new_pa, 105);
        assert_eq!(new_ca, 105);
    }

    #[test]
    fn delta_apply_regressive_preserves_ca_le_pa() {
        // Both go down; ensure ca stays ≤ pa.
        let (new_pa, new_ca) = apply_family_delta(80, 60, -10, -5);
        assert_eq!(new_pa, 70);
        assert_eq!(new_ca, 55);
        assert!(new_ca <= new_pa);
    }

    // -----------------------------------------------------------------------
    // RED test 8: serde roundtrip for AttributeFamily from fw-core
    // (mirrors test in fw-core but verifies the re-export chain works)
    // -----------------------------------------------------------------------

    #[test]
    fn attribute_family_serde_roundtrip_via_fw_content_path() {
        for family in AttributeFamily::ALL {
            let encoded = ron::ser::to_string(&family).expect("ron encode");
            let decoded: AttributeFamily = ron::de::from_str(&encoded).expect("ron decode");
            assert_eq!(decoded, family);
        }
    }
}

// -----------------------------------------------------------------------
// Proptest invariant: 1 ≤ ca ≤ pa ≤ 200 for every family over random
// GeneSnapshot + AbilityCeiling.
// -----------------------------------------------------------------------

#[cfg(test)]
mod proptest_invariants {
    use std::collections::BTreeSet;

    use proptest::prelude::*;

    use super::*;
    use crate::gene::{MentalGenes, PhysicalGenes, TechnicalAffinities};

    /// Strategy: Q32 in [0,1] via raw bits in [0, 2^32].
    fn q32_unit() -> impl Strategy<Value = Q32> {
        (0i64..=(1i64 << 32)).prop_map(Q32::from_raw)
    }

    /// Strategy: Q32 in [-1,+1] via raw bits in [-(2^32), 2^32].
    fn q32_signed() -> impl Strategy<Value = Q32> {
        (-(1i64 << 32)..=(1i64 << 32)).prop_map(Q32::from_raw)
    }

    fn arb_physical() -> impl Strategy<Value = PhysicalGenes> {
        (
            q32_unit(),
            q32_unit(),
            q32_unit(),
            q32_unit(),
            q32_signed(), // growth_curve
            q32_unit(),
            q32_unit(),
        )
            .prop_map(
                |(
                    height_ceiling,
                    frame_density,
                    fast_twitch_ratio,
                    stamina_recovery,
                    growth_curve,
                    aging_curve,
                    injury_resilience,
                )| PhysicalGenes {
                    height_ceiling,
                    frame_density,
                    fast_twitch_ratio,
                    stamina_recovery,
                    growth_curve,
                    aging_curve,
                    injury_resilience,
                },
            )
    }

    fn arb_mental() -> impl Strategy<Value = MentalGenes> {
        (
            q32_unit(),
            q32_unit(),
            q32_unit(),
            q32_unit(),
            q32_unit(),
            q32_signed(), // mentality
        )
            .prop_map(
                |(
                    pattern_recognition,
                    composure_floor,
                    decision_velocity,
                    learning_rate,
                    ambition,
                    mentality,
                )| MentalGenes {
                    pattern_recognition,
                    composure_floor,
                    decision_velocity,
                    learning_rate,
                    ambition,
                    mentality,
                },
            )
    }

    fn arb_technical() -> impl Strategy<Value = TechnicalAffinities> {
        (q32_unit(), q32_unit(), q32_unit(), q32_unit(), q32_unit()).prop_map(
            |(left_foot, aerial, dead_ball, striking, first_touch)| TechnicalAffinities {
                left_foot,
                aerial,
                dead_ball,
                striking,
                first_touch,
            },
        )
    }

    fn arb_genes() -> impl Strategy<Value = crate::gene::GeneSnapshot> {
        (arb_physical(), arb_mental(), arb_technical()).prop_map(|(physical, mental, technical)| {
            crate::gene::GeneSnapshot {
                physical,
                mental,
                technical,
                narrative_flags: BTreeSet::new(),
            }
        })
    }

    /// Strategy for a valid AbilityCeiling (current ≤ potential, both in [0,1]).
    fn arb_ceiling() -> impl Strategy<Value = AbilityCeiling> {
        (0i64..=(1i64 << 32), 0i64..=(1i64 << 32)).prop_map(|(a, b)| {
            let lo = Q32::from_raw(a.min(b));
            let hi = Q32::from_raw(a.max(b));
            AbilityCeiling::try_new(lo, hi).expect("valid ceiling from ordered pair")
        })
    }

    proptest! {
        /// Core invariant: for all random GeneSnapshot + AbilityCeiling,
        /// every family satisfies 1 ≤ ca ≤ pa ≤ 200.
        #[test]
        fn gene_family_pa_ca_invariant_1_le_ca_le_pa_le_200(
            genes in arb_genes(),
            ceiling in arb_ceiling(),
        ) {
            let FamilyPaCa { pa: pa_map, ca: ca_map } = gene_family_pa_ca(&genes, ceiling);
            for family in AttributeFamily::ALL {
                let pa = *pa_map.get(&family).unwrap();
                let ca = *ca_map.get(&family).unwrap();
                prop_assert!(pa >= 1, "PA < 1 for {family:?}: pa={pa}");
                prop_assert!(pa <= 200, "PA > 200 for {family:?}: pa={pa}");
                prop_assert!(ca >= 1, "CA < 1 for {family:?}: ca={ca}");
                prop_assert!(ca <= pa, "CA ({ca}) > PA ({pa}) for {family:?}");
            }
        }
    }
}

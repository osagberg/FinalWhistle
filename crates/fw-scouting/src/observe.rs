//! Path-B report generation — `observe_player`.
//!
//! Deterministic, pure function. Identical `(scout, player_bio, career_seed, observation_id)`
//! → byte-identical `ScoutReport`. Seeded via ADR-0009 `SeedLayer::ScoutObservation`.
//!
//! Algorithm per `design/scouting.md §"Path-B report generation"`.

use fw_content::PlayerBio;
use fw_core::{PlayerId, Q32, SeedLayer, seed_fn};
use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{RngCore, SeedableRng};

use crate::report::{GeneCategory, GeneCategoryEstimate, LabelEstimate, ScoutReport};
use crate::scout::{
    BASIC_SCOUT_BAND_HALF_WIDTH, LABEL_CONFIDENCE_MAX, LABEL_CONFIDENCE_MIN,
    NO_LABEL_DEFAULT_CONFIDENCE, Scout,
};

/// Generate a `ScoutReport` from a single scout's observation of a player.
///
/// Deterministic and pure: identical inputs → identical output.
///
/// # Parameters
/// - `scout` — the observing scout archetype (must be `BasicScoutUncertainty` for Path B).
/// - `player_bio` — the player being observed (genes + `scout_labels` are read).
/// - `career_seed` — the career-level seed (analogous to `match_seed` in ADR-0009).
/// - `observation_id` — monotonically-increasing per-player observation counter (used as `tick`).
/// - `subject` — the roster `PlayerId` of the player being observed. This is set
///   verbatim as `ScoutReport.player_id` and used as the RNG site; the
///   `subject ↔ player_bio` correspondence (that the bio is the one whose genes
///   belong to this roster player) is the CALLER's responsibility — the only
///   production caller, `fw_tauri::season::observe_match_participants`, enforces it
///   with a release-mode `assert!(bio.internal_gene_snapshot == instance.genes)`.
///
/// # Determinism
/// One `ChaCha8Rng` per call, seeded from
/// `seed_fn(career_seed, observation_id, SeedLayer::ScoutObservation, subject.raw())`.
/// Using `subject.raw()` as the RNG site ensures noise is keyed per-roster-player,
/// so two distinct roster players sharing the same bio receive independent reports
/// (F2 fix — prior to this the site was hardcoded `0`, making byte-identical reports
/// for different players when they shared a bio and the same `observation_id`).
/// All draws are sequential from this single stream.
pub fn observe_player(
    scout: &Scout,
    player_bio: &PlayerBio,
    career_seed: u64,
    observation_id: u32,
    subject: PlayerId,
) -> ScoutReport {
    let rng_seed = seed_fn(
        career_seed,
        observation_id,
        SeedLayer::ScoutObservation,
        subject.raw(),
    );
    let mut rng = ChaCha8Rng::seed_from_u64(rng_seed);

    let noise_amp = scout.base_observation_noise;

    // Step 2: Category estimates — Physical, Mental, Technical in order.
    let category_estimates =
        observe_categories(&player_bio.internal_gene_snapshot, noise_amp, &mut rng);

    // Step 3: Label estimates — one per true label in BTreeSet iteration order.
    let label_estimates = observe_labels(player_bio, &mut rng);

    // Step 4: Overall confidence = mean of label confidences; 0.5 if no labels.
    let confidence = compute_overall_confidence(&label_estimates);

    // Step 5: Assemble.
    ScoutReport {
        scout_archetype_id: scout.archetype_id.clone(),
        player_id: subject,
        confidence,
        label_estimates,
        category_estimates,
    }
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// Draw category estimates for all three categories in declared order.
fn observe_categories(
    genes: &fw_content::GeneSnapshot,
    noise_amp: Q32,
    rng: &mut ChaCha8Rng,
) -> Vec<GeneCategoryEstimate> {
    let two = Q32::from_int(2);

    vec![
        observe_one_category(
            GeneCategory::Physical,
            physical_true_mean(genes),
            noise_amp,
            two,
            rng,
        ),
        observe_one_category(
            GeneCategory::Mental,
            mental_true_mean(genes),
            noise_amp,
            two,
            rng,
        ),
        observe_one_category(
            GeneCategory::Technical,
            technical_true_mean(genes),
            noise_amp,
            two,
            rng,
        ),
    ]
}

/// Observe a single gene category: draw a noisy center and emit `[low, high]`.
fn observe_one_category(
    category: GeneCategory,
    true_mean: Q32,
    noise_amp: Q32,
    two: Q32,
    rng: &mut ChaCha8Rng,
) -> GeneCategoryEstimate {
    // Draw noise uniform in [-noise_amp, +noise_amp).
    // Pattern: frac ∈ [0, 1) → noise = frac * 2 * noise_amp - noise_amp
    let frac = uniform_01(rng);
    // frac * 2 * noise_amp gives [0, 2*noise_amp); subtract noise_amp for [-noise_amp, +noise_amp)
    let noise = frac * (two * noise_amp) - noise_amp;

    // center = clamp(true_mean + noise, 0, 1)
    // SAFETY: `true_mean + noise` legitimately exceeds [0,1] (noise can push the sum
    // above 1 or below 0); clamping the displayed center to [0,1] is by design.
    let center = (true_mean + noise).max(Q32::ZERO).min(Q32::ONE);

    let half_width = BASIC_SCOUT_BAND_HALF_WIDTH;
    // SAFETY: a band must stay in [0,1]. Clamping narrows the band slightly when
    // center is near the extremes; this is accepted-by-design — a clearly elite or
    // poor category reads tighter near the boundary, which is visually coherent.
    let low = (center - half_width).max(Q32::ZERO);
    let high = (center + half_width).min(Q32::ONE);

    // By construction: `low = center - hw` and `high = center + hw` with `hw >= 0`
    // (BASIC_SCOUT_BAND_HALF_WIDTH is a positive constant) and both are clamped to
    // [0,1] above, so `low <= center <= high` always holds. `expect` fires in both
    // debug and release; it is unreachable given the arithmetic above.
    GeneCategoryEstimate::try_new(category, low, high).expect(
        "low <= high guaranteed by construction: center ∓ half_width, both clamped to [0,1]",
    )
}

/// Observe label confidences for all labels in `BTreeSet` iteration order.
fn observe_labels(player_bio: &PlayerBio, rng: &mut ChaCha8Rng) -> Vec<LabelEstimate> {
    // LABEL_CONFIDENCE_MAX - LABEL_CONFIDENCE_MIN = the range width.
    let range_width = LABEL_CONFIDENCE_MAX - LABEL_CONFIDENCE_MIN;

    player_bio
        .scout_labels
        .iter()
        .map(|&label| {
            // confidence uniform in [LABEL_CONFIDENCE_MIN, LABEL_CONFIDENCE_MAX)
            let frac = uniform_01(rng);
            let confidence = LABEL_CONFIDENCE_MIN + frac * range_width;
            LabelEstimate { label, confidence }
        })
        .collect()
}

/// Compute overall confidence: mean of label confidences, or 0.5 if no labels.
fn compute_overall_confidence(label_estimates: &[LabelEstimate]) -> Q32 {
    if label_estimates.is_empty() {
        return NO_LABEL_DEFAULT_CONFIDENCE;
    }
    // Sum of confidences ÷ count. Use checked arithmetic to accumulate.
    let count = i32::try_from(label_estimates.len())
        .expect("scout_labels count is bounded by the 46-variant phenotype-label enum");
    let sum: Q32 = label_estimates
        .iter()
        .fold(Q32::ZERO, |acc, le| acc + le.confidence);
    sum / Q32::from_int(count)
}

/// Draw a uniform value in `[0, 1)` from the RNG.
///
/// Pattern from `fw-match-sim::utility::softmax`: `(next_u64() >> 32) as i64`
/// maps u64 high 32 bits → Q32 fractional bits, giving exactly `[0, 1)`.
#[inline]
fn uniform_01(rng: &mut ChaCha8Rng) -> Q32 {
    Q32::from_raw((rng.next_u64() >> 32) as i64)
}

// ---------------------------------------------------------------------------
// Category mean helpers — arithmetic mean of each category's gene fields
// ---------------------------------------------------------------------------

/// Arithmetic mean of the 6 physical *quality* gene fields.
///
/// Excludes `growth_curve` (signed `[-1, +1]` trajectory parameter — dimensionally
/// incoherent in a `[0, 1]` level estimate; a late-bloomer gene says nothing about
/// current physical level). All 6 included fields are `[0, 1]`, so the mean is
/// provably `[0, 1]`. The `.max(ZERO).min(ONE)` clamp is retained as cheap
/// defense-in-depth but is unreachable in practice given `[0,1]` inputs.
fn physical_true_mean(genes: &fw_content::GeneSnapshot) -> Q32 {
    let p = &genes.physical;
    let six = Q32::from_int(6);
    let sum = p.height_ceiling
        + p.frame_density
        + p.fast_twitch_ratio
        + p.stamina_recovery
        + p.aging_curve
        + p.injury_resilience;
    // All inputs are [0,1]; clamp is unreachable in practice.
    (sum / six).max(Q32::ZERO).min(Q32::ONE)
}

/// Arithmetic mean of the 5 mental *quality* gene fields.
///
/// Excludes `mentality` (signed `[-1, +1]` disposition parameter — dimensionally
/// incoherent in a `[0, 1]` level estimate). All 5 included fields are `[0, 1]`,
/// so the mean is provably `[0, 1]`. The `.max(ZERO).min(ONE)` clamp is retained as
/// cheap defense-in-depth but is unreachable in practice given `[0,1]` inputs.
fn mental_true_mean(genes: &fw_content::GeneSnapshot) -> Q32 {
    let m = &genes.mental;
    let five = Q32::from_int(5);
    let sum = m.pattern_recognition
        + m.composure_floor
        + m.decision_velocity
        + m.learning_rate
        + m.ambition;
    // All inputs are [0,1]; clamp is unreachable in practice.
    (sum / five).max(Q32::ZERO).min(Q32::ONE)
}

/// Arithmetic mean of the 5 technical *quality* gene fields.
///
/// All 5 fields are `[0, 1]`, so the mean is provably `[0, 1]`. The
/// `.max(ZERO).min(ONE)` clamp is retained as cheap defense-in-depth but is
/// unreachable in practice given `[0,1]` inputs.
fn technical_true_mean(genes: &fw_content::GeneSnapshot) -> Q32 {
    let t = &genes.technical;
    let five = Q32::from_int(5);
    let sum = t.left_foot + t.aerial + t.dead_ball + t.striking + t.first_touch;
    // All inputs are [0,1]; clamp is unreachable in practice.
    (sum / five).max(Q32::ZERO).min(Q32::ONE)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use fw_content::{GeneSnapshot, MentalGenes, PhysicalGenes, TechnicalAffinities};

    use super::*;

    fn half() -> Q32 {
        Q32::from_raw(2_147_483_648_i64) // 0.5
    }

    fn all_half_genes() -> GeneSnapshot {
        let h = half();
        GeneSnapshot {
            physical: PhysicalGenes {
                height_ceiling: h,
                frame_density: h,
                fast_twitch_ratio: h,
                stamina_recovery: h,
                growth_curve: Q32::ZERO, // signed; zero keeps the mean tidy
                aging_curve: h,
                injury_resilience: h,
            },
            mental: MentalGenes {
                pattern_recognition: h,
                composure_floor: h,
                decision_velocity: h,
                learning_rate: h,
                ambition: h,
                mentality: Q32::ZERO, // signed
            },
            technical: TechnicalAffinities {
                left_foot: Q32::ZERO,
                aerial: h,
                dead_ball: h,
                striking: h,
                first_touch: h,
            },
            narrative_flags: BTreeSet::new(),
        }
    }

    #[test]
    fn physical_mean_all_half_quality_genes_equals_half() {
        // all_half_genes() sets all 6 physical quality fields to 0.5 (growth_curve is
        // zero but excluded). Mean of 6 × 0.5 / 6 = exactly 0.5.
        let genes = all_half_genes();
        let mean = physical_true_mean(&genes);
        // Must equal 0.5 within ±1 ULP — any formula regression will break this.
        let diff = (mean.to_bits() - half().to_bits()).abs();
        assert!(
            diff <= 1,
            "physical_true_mean of all-0.5 quality genes must be 0.5; raw diff = {diff}"
        );
    }

    #[test]
    fn mental_mean_all_half_quality_genes_equals_half() {
        // all_half_genes() sets all 5 mental quality fields to 0.5 (mentality is
        // zero but excluded). Mean of 5 × 0.5 / 5 = exactly 0.5.
        let genes = all_half_genes();
        let mean = mental_true_mean(&genes);
        let diff = (mean.to_bits() - half().to_bits()).abs();
        assert!(
            diff <= 1,
            "mental_true_mean of all-0.5 quality genes must be 0.5; raw diff = {diff}"
        );
    }

    #[test]
    fn uniform_01_stays_in_unit_range() {
        use rand_chacha::rand_core::SeedableRng;
        let mut rng = ChaCha8Rng::seed_from_u64(12345);
        for _ in 0..1000 {
            let v = uniform_01(&mut rng);
            assert!(
                v >= Q32::ZERO && v < Q32::ONE,
                "uniform_01 out of [0,1): {v:?}"
            );
        }
    }
}

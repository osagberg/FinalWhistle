//! Top-N softmax sampling for action selection.
//!
//! Per ADR-0003 §6: sort candidates by utility descending, take the top 3,
//! compute weights via `exp_q32(u / T)`, then sample proportionally using a
//! seeded ChaCha8Rng.
//!
//! `temperature → 0` reduces to argmax (deterministic best-pick — test escape
//! hatch via `temperature = Q32::EPSILON`).
//! `temperature → infinity` approaches uniform sampling over the top N.
//!
//! The RNG must be seeded with `seed_fn(match_seed, tick, SeedLayer::UtilityTieBreak,
//! decision_id)` by the caller — this module does not create its own RNG.

use fw_core::{Q32, exp_q32};
use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::RngCore;

/// Maximum candidates included in softmax.  ADR-0003 §6 sets this to 3.
pub const SOFTMAX_TOP_N: usize = 3;

/// Phase-1 temperature tuning seed: 0.15 in Q32.
pub const DEFAULT_TEMPERATURE: Q32 = Q32::from_raw(644_245_094_i64); // 0.15

// -------------------------------------------------------------------------
// Public API
// -------------------------------------------------------------------------

/// Select one candidate from the top-N by softmax weight.
///
/// - `candidates` — `(action_id, utility)` pairs; need not be sorted.
/// - `rng` — caller-seeded ChaCha8Rng from `seed_fn(...)`.
/// - `temperature` — controls sharpness; use `DEFAULT_TEMPERATURE` in production.
///   Pass `Q32::EPSILON` for near-deterministic argmax (testing).
///
/// Returns `Some(action_id)` of the selected candidate, or `None` if
/// `candidates` is empty. Callers should `.expect("non-empty candidate slice")`
/// when the slice is guaranteed non-empty by the call-site invariant.
pub fn pick_top_n_softmax<ActionId: Copy>(
    candidates: &[(ActionId, Q32)],
    rng: &mut ChaCha8Rng,
    temperature: Q32,
) -> Option<ActionId> {
    if candidates.is_empty() {
        return None;
    }

    // Sort descending by utility; take top N.
    let mut sorted: Vec<(ActionId, Q32)> = candidates.to_vec();
    sorted.sort_by_key(|item| std::cmp::Reverse(item.1));
    let top: &[(ActionId, Q32)] = if sorted.len() > SOFTMAX_TOP_N {
        &sorted[..SOFTMAX_TOP_N]
    } else {
        &sorted
    };

    // If only one candidate, return immediately.
    if top.len() == 1 {
        return Some(top[0].0);
    }

    // Temperature near-zero: argmax (already top[0] after sort).
    if temperature <= Q32::EPSILON {
        return Some(top[0].0);
    }

    // Compute softmax weights: w_i = exp_q32(u_i / T).
    //
    // Overflow analysis: utility ∈ [0, 1]; temperature > EPSILON (checked above).
    // scaled = u / T ≤ 1 / EPSILON ≈ 4.3e9 >> LUT_MAX (8). LUT saturates so
    // weight ≤ exp(8) ≈ 2981. With top-3: weight_sum ≤ 3 × 2981 ≈ 8943 << Q32::MAX.
    // rand_frac ∈ [0, 1); rand_frac * weight_sum ≤ 1 × 8943 — no overflow.
    // Bare operators panic on violation.
    let mut weights: Vec<Q32> = Vec::with_capacity(top.len());
    let mut weight_sum = Q32::ZERO;
    for &(_, util) in top {
        let scaled = util / temperature;
        let w = exp_q32(scaled);
        weight_sum += w;
        weights.push(w);
    }

    if weight_sum <= Q32::ZERO {
        return Some(top[0].0);
    }

    // Sample uniformly in [0, weight_sum).
    // Convert rng output (u64) to Q32 in [0, 1), then scale.
    let rand_u64 = rng.next_u64();
    // Use upper 32 bits as fractional Q32 in [0, 1). Shift makes it non-negative
    // as i64 (top bit of u32 range → 0 in i64 sign bit when stored in 64 bits).
    let rand_frac = Q32::from_raw((rand_u64 >> 32) as i64);
    let threshold = rand_frac * weight_sum;

    // Walk cumulative weights.
    let mut cumulative = Q32::ZERO;
    for (i, &w) in weights.iter().enumerate() {
        cumulative += w;
        if cumulative >= threshold {
            return Some(top[i].0);
        }
    }

    // Fallback (rounding): last candidate.
    Some(top[top.len() - 1].0)
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::rand_core::SeedableRng;

    #[allow(clippy::float_arithmetic)]
    fn q(v: f64) -> Q32 {
        // Test helper only — not canonical state.
        Q32::from_raw((v * (1u64 << 32) as f64) as i64)
    }

    fn rng() -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(0xdeadbeef_cafebabe)
    }

    #[test]
    fn empty_candidates_returns_none() {
        let mut r = rng();
        let result: Option<u32> = pick_top_n_softmax(&[], &mut r, DEFAULT_TEMPERATURE);
        assert_eq!(result, None);
    }

    #[test]
    fn single_candidate_always_returns_it() {
        let mut r = rng();
        let result: Option<u32> =
            pick_top_n_softmax(&[(42u32, q(0.5))], &mut r, DEFAULT_TEMPERATURE);
        assert_eq!(result, Some(42));
    }

    #[test]
    fn zero_temperature_returns_argmax() {
        let mut r = rng();
        let candidates = vec![(1u32, q(0.3)), (2u32, q(0.9)), (3u32, q(0.1))];
        let result = pick_top_n_softmax(&candidates, &mut r, Q32::EPSILON).unwrap();
        assert_eq!(result, 2, "argmax should return action 2 (utility 0.9)");
    }

    #[test]
    fn high_utility_dominates_at_low_temperature() {
        let candidates = vec![(1u32, q(0.9)), (2u32, q(0.1)), (3u32, q(0.05))];
        let temp = q(0.05);
        let mut count_best = 0u32;
        for seed in 0u64..20 {
            let mut r = ChaCha8Rng::seed_from_u64(seed);
            if pick_top_n_softmax(&candidates, &mut r, temp).unwrap() == 1 {
                count_best += 1;
            }
        }
        assert!(
            count_best >= 18,
            "low temperature should mostly pick best action: {count_best}/20"
        );
    }

    #[test]
    fn all_candidates_reachable_at_high_temperature() {
        let candidates = vec![
            (1u32, q(0.8)),
            (2u32, q(0.75)),
            (3u32, q(0.70)),
            (4u32, q(0.01)), // outside top-3; should never be selected
        ];
        let temp = q(5.0);
        let mut seen = [false; 5];
        for seed in 0u64..100 {
            let mut r = ChaCha8Rng::seed_from_u64(seed);
            let id = pick_top_n_softmax(&candidates, &mut r, temp).unwrap() as usize;
            if id < 5 {
                seen[id] = true;
            }
        }
        assert!(seen[1], "action 1 should be reachable");
        assert!(seen[2], "action 2 should be reachable");
        assert!(seen[3], "action 3 should be reachable");
        assert!(!seen[4], "action 4 should NOT be reachable (outside top-3)");
    }

    #[test]
    fn deterministic_for_same_seed() {
        let candidates = vec![(10u32, q(0.5)), (20u32, q(0.4)), (30u32, q(0.3))];
        let mut r1 = ChaCha8Rng::seed_from_u64(12345);
        let mut r2 = ChaCha8Rng::seed_from_u64(12345);
        assert_eq!(
            pick_top_n_softmax(&candidates, &mut r1, DEFAULT_TEMPERATURE),
            pick_top_n_softmax(&candidates, &mut r2, DEFAULT_TEMPERATURE),
            "same seed must produce same result"
        );
    }

    #[test]
    fn only_top_three_selected() {
        let candidates = vec![
            (1u32, q(0.9)),
            (2u32, q(0.8)),
            (3u32, q(0.7)),
            (4u32, q(0.6)), // rank 4 — excluded
            (5u32, q(0.5)), // rank 5 — excluded
        ];
        let temp = q(1.0);
        for seed in 0u64..50 {
            let mut r = ChaCha8Rng::seed_from_u64(seed);
            let id = pick_top_n_softmax(&candidates, &mut r, temp).unwrap();
            assert!(
                id <= 3,
                "seed {seed}: action {id} selected; only top-3 (actions 1-3) should be reachable"
            );
        }
    }
}

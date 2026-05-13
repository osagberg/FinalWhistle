//! Pressing intensity — product-form per Bauer et al. 2025.
//!
//! P_press = 1 - Π_i (1 - P_arrive_i(carrier_position))
//!
//! where P_arrive_i is computed using the same time-to-intercept logic as
//! `pitch_control`.  Returns Q32 in [0, 1]: 0 = nobody pressing, 1 = at
//! least one defender guaranteed to arrive instantly.

use crate::utility::pitch_control::{PlayerSnapshot, SIGMA, TAU_REACT, player_tau};
use fw_core::{Q32, sigmoid_q32};

// -------------------------------------------------------------------------
// Public API
// -------------------------------------------------------------------------

/// Compute the aggregate pressing intensity on a ball carrier.
///
/// - `carrier` — kinematic state of the player in possession.
/// - `defenders` — list of defending players' snapshots.
///
/// Returns Q32 in [0, 1].
pub fn pressing_intensity(carrier: &PlayerSnapshot, defenders: &[PlayerSnapshot]) -> Q32 {
    if defenders.is_empty() {
        return Q32::ZERO;
    }

    // The "point" we're evaluating control at is the carrier's current position.
    let point = carrier.pos;

    // Compute each defender's tau to the carrier position.
    // Use carrier's tau to itself as the mean reference (it's TAU_REACT = 0.1s — already there).
    let carrier_tau = TAU_REACT; // carrier is at the point; only reaction time counts.

    // Product form: start with 1.0 complement, multiply (1 - P_arrive_i) for each defender.
    //
    // Overflow analysis: all values are in [0, 1] throughout; multiplication of values
    // in [0, 1] stays in [0, 1] — no overflow possible. Bare operators panic on violation.
    let mut product_complement = Q32::ONE; // Π (1 - P_arrive_i)

    for snap in defenders {
        let tau_i = player_tau(snap, point);
        // P_arrive_i = sigmoid((carrier_tau - tau_i) / sigma)
        // When tau_i < carrier_tau, the defender arrives before the carrier
        // would react → high arrival probability.
        let delta = carrier_tau - tau_i;
        let scaled = delta / SIGMA;
        let p_arrive = sigmoid_q32(scaled);

        // (1 - p_arrive) is in [0, 1] since p_arrive ∈ [0, 1].
        let complement = Q32::ONE - p_arrive;
        product_complement *= complement;
    }

    // P_press = 1 - product_complement; product_complement ∈ [0, 1].
    Q32::ONE - product_complement
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::float_arithmetic)]
    fn q(v: f64) -> Q32 {
        // Test helper only — not canonical state.
        Q32::from_raw((v * (1u64 << 32) as f64) as i64)
    }

    fn snap_at(px: f64, py: f64, vmax: f64) -> PlayerSnapshot {
        PlayerSnapshot {
            pos: (q(px), q(py)),
            vel: (Q32::ZERO, Q32::ZERO),
            v_max: q(vmax),
        }
    }

    #[test]
    fn no_defenders_gives_zero_pressing() {
        let carrier = snap_at(0.0, 0.0, 7.0);
        assert_eq!(pressing_intensity(&carrier, &[]), Q32::ZERO);
    }

    #[test]
    fn pressing_is_in_unit_range() {
        let carrier = snap_at(10.0, 0.0, 7.0);
        let defenders = vec![snap_at(12.0, 0.0, 8.0), snap_at(15.0, 3.0, 7.0)];
        let p = pressing_intensity(&carrier, &defenders);
        assert!(p >= Q32::ZERO, "pressing must be >= 0");
        assert!(p <= Q32::ONE, "pressing must be <= 1");
    }

    #[test]
    fn more_defenders_increase_pressing() {
        // Each additional nearby defender should increase pressing.
        let carrier = snap_at(10.0, 0.0, 7.0);
        let p1 = pressing_intensity(&carrier, &[snap_at(12.0, 0.0, 8.0)]);
        let p2 = pressing_intensity(
            &carrier,
            &[snap_at(12.0, 0.0, 8.0), snap_at(11.0, 2.0, 8.0)],
        );
        assert!(
            p2 >= p1,
            "two defenders should press at least as hard as one"
        );
    }

    #[test]
    fn very_far_defenders_give_near_zero_pressing() {
        // Defenders 100m away with modest speed — pressing should be tiny.
        let carrier = snap_at(0.0, 0.0, 7.0);
        let defenders = vec![snap_at(100.0, 0.0, 7.0)];
        let p = pressing_intensity(&carrier, &defenders);
        let half = Q32::from_raw(1i64 << 31);
        assert!(
            p < half,
            "very distant defenders should give low pressing, got raw {}",
            p.to_bits()
        );
    }

    #[test]
    fn defender_at_same_position_gives_high_pressing() {
        // A defender already at the carrier's position has tau_i = carrier_tau (both react);
        // delta = 0, sigmoid(0) = 0.5, so pressing = 0.5.  Must be >= half.
        let carrier = snap_at(5.0, 5.0, 7.0);
        let defenders = vec![snap_at(5.0, 5.0, 8.0)]; // same spot
        let p = pressing_intensity(&carrier, &defenders);
        let half = Q32::from_raw(1i64 << 31);
        assert!(
            p >= half,
            "defender at carrier position should give pressing >= 0.5, got raw {}",
            p.to_bits()
        );
    }
}

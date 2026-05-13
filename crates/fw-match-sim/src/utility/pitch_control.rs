//! Spearman-style pitch control — closed-form time-to-intercept per player.
//!
//! For each query point P on the pitch, each player i has:
//!
//!   tau_i(P) = tau_react + ||P - pos_i|| / v_max_i + alpha * angular_penalty(vel_i, P - pos_i)
//!
//! From tau values we derive per-player arrival probabilities via sigmoid:
//!
//!   P_arrive_i(P) = sigmoid((mean_tau(P) - tau_i(P)) / sigma)
//!
//! Attacker control = sum of P_arrive_i over attackers (normalized to [0,1]).
//! Defender control = sum of P_arrive_i over defenders.
//!
//! Phase-1 kinematic constants live here as consts; tuning lives in
//! `docs/design/pitch-control-kinematics.md` (to be authored at T2-1).

use fw_core::{Q32, sigmoid_q32};

// -------------------------------------------------------------------------
// Kinematic constants (Phase-1 tuning seeds)
// -------------------------------------------------------------------------

/// Reaction time per player: 0.1 seconds in Q32.  Raw = 0.1 * 2^32.
pub const TAU_REACT: Q32 = Q32::from_raw(429_496_730_i64); // 0.1 s

/// Sigmoid spread parameter sigma: 0.45 in Q32.  Controls the softness of
/// the arrival-probability sigmoid.
pub const SIGMA: Q32 = Q32::from_raw(1_932_735_283_i64); // 0.45

/// Angular penalty coefficient alpha: 0.5 in Q32.  Scales the angular
/// deviation cost; units are (seconds / radian).
pub const ALPHA: Q32 = Q32::from_raw(2_147_483_648_i64); // 0.5

// -------------------------------------------------------------------------
// Player snapshot
// -------------------------------------------------------------------------

/// Minimal kinematic state for a player at a single tick, used by pitch control
/// and pressing calculations. Constructed from canonical state by the caller.
#[derive(Debug, Clone, Copy)]
pub struct PlayerSnapshot {
    /// Position (pitch metres from centre): (x, y).
    pub pos: (Q32, Q32),
    /// Velocity direction + magnitude (metres/second): (vx, vy).
    pub vel: (Q32, Q32),
    /// Maximum sprint speed for this player (metres/second).
    pub v_max: Q32,
}

// -------------------------------------------------------------------------
// Outcome
// -------------------------------------------------------------------------

use fw_core::PlayerId;

/// Result of evaluating pitch control at a single point.
///
/// Invariant: `attacker_control + defender_control + neutral_control == Q32::ONE`.
/// Enforced by construction in `pitch_control()`.
#[derive(Debug, Clone, Copy)]
pub struct PitchControlOutcome {
    /// Attacker aggregate arrival probability ∈ [0, 1].
    pub attacker_control: Q32,
    /// Defender aggregate arrival probability ∈ [0, 1].
    pub defender_control: Q32,
    /// Residual probability unaccounted for by either team ∈ [0, 1].
    /// Absorbs "nobody controls this zone" mass; ensures the three values sum to 1.
    pub neutral_control: Q32,
    /// Id of the player with the fastest arrival (smallest tau).
    /// `None` if both slices are empty.
    pub fastest_arrival: Option<PlayerId>,
    /// Tau of the fastest-arriving player (seconds in Q32).
    /// `None` if both slices are empty.
    pub fastest_arrival_tau: Option<Q32>,
}

// -------------------------------------------------------------------------
// Public API
// -------------------------------------------------------------------------

/// Evaluate pitch control at a single point.
///
/// - `point` — the (x, y) pitch coordinate to query (metres).
/// - `attackers` — slice of (PlayerId, PlayerSnapshot) for the attacking team.
/// - `defenders` — slice of (PlayerId, PlayerSnapshot) for the defending team.
///
/// Returns `PitchControlOutcome` with arrival probabilities in [0, 1].
pub fn pitch_control(
    point: (Q32, Q32),
    attackers: &[(PlayerId, PlayerSnapshot)],
    defenders: &[(PlayerId, PlayerSnapshot)],
) -> PitchControlOutcome {
    // Compute tau for every player.
    let mut all_taus: Vec<(PlayerId, Q32, bool)> = Vec::new(); // (id, tau, is_attacker)

    for &(id, ref snap) in attackers {
        all_taus.push((id, player_tau(snap, point), true));
    }
    for &(id, ref snap) in defenders {
        all_taus.push((id, player_tau(snap, point), false));
    }

    if all_taus.is_empty() {
        return PitchControlOutcome {
            attacker_control: Q32::ZERO,
            defender_control: Q32::ZERO,
            neutral_control: Q32::ONE,
            fastest_arrival: None,
            fastest_arrival_tau: None,
        };
    }

    // Mean tau across all players — used as the sigmoid midpoint.
    let mean_tau = mean_tau(&all_taus);

    // Raw aggregate arrival probabilities per team (each in [0, n_players]).
    let raw_att = sum_arrive_prob(&all_taus, mean_tau, true);
    let raw_def = sum_arrive_prob(&all_taus, mean_tau, false);

    // Normalize to enforce attacker + defender + neutral = 1.
    // Denominator = max(raw_att + raw_def, 1) to avoid divide-by-zero when
    // both teams are empty (guard — the early-return above handles the actually
    // empty case; this guards against both teams having zero players in the
    // all_taus slice after the loop).
    let raw_sum = raw_att + raw_def;
    let denom = if raw_sum > Q32::ONE {
        raw_sum
    } else {
        Q32::ONE
    };
    let attacker_control = (raw_att / denom).min(Q32::ONE);
    let defender_control = (raw_def / denom).min(Q32::ONE);
    // neutral absorbs any residual mass so the invariant holds exactly.
    let combined = attacker_control + defender_control;
    let neutral_control = if combined <= Q32::ONE {
        Q32::ONE - combined
    } else {
        Q32::ZERO
    };

    // Fastest-arriving player.
    let (fastest_id, fastest_tau) = all_taus
        .iter()
        .min_by_key(|(_, tau, _)| *tau)
        .map(|&(id, tau, _)| (id, tau))
        .unwrap(); // safe: all_taus is non-empty

    PitchControlOutcome {
        attacker_control,
        defender_control,
        neutral_control,
        fastest_arrival: Some(fastest_id),
        fastest_arrival_tau: Some(fastest_tau),
    }
}

// -------------------------------------------------------------------------
// Internals
// -------------------------------------------------------------------------

/// Time-to-intercept for player `snap` to reach `point`.
///
/// tau = tau_react + ||P - pos|| / v_max + alpha * angular_penalty(vel, P - pos)
///
/// # Overflow analysis
///
/// Pitch dimensions are ~105 × 68 m; `dx`, `dy` ≤ 105. `dx * dx` ≤ 11025,
/// `dy * dy` ≤ 4624. dist² ≤ ~15650, dist ≤ 125. v_max ≤ 12 m/s (sprint),
/// so travel ≤ 125/12 ≈ 10.4 s. `angular_penalty` ≤ π ≈ 3.14 (radians).
/// `ALPHA * penalty` ≤ 0.5 × π ≈ 1.57. tau ≤ 0.1 + 10.4 + 1.57 ≈ 12.1 —
/// well within Q32 range. Bare operators panic on violation.
pub(crate) fn player_tau(snap: &PlayerSnapshot, point: (Q32, Q32)) -> Q32 {
    let dx = point.0 - snap.pos.0;
    let dy = point.1 - snap.pos.1;

    // dist = sqrt(dx² + dy²); both squares are non-negative.
    let dist = (dx * dx + dy * dy).sqrt();

    // Travel time: dist / v_max. If v_max is zero, treat as very slow.
    let travel = if snap.v_max > Q32::ZERO {
        dist / snap.v_max
    } else {
        Q32::MAX
    };

    // Angular penalty: alpha * angle between vel and (P - pos).
    let angle_penalty = angular_penalty(snap.vel, (dx, dy));
    let alpha_penalty = ALPHA * angle_penalty;

    // Sum — all components are non-negative and bounded (see overflow analysis).
    TAU_REACT + travel + alpha_penalty
}

/// Angular penalty: the angle (in radians, Q32) between `vel` and `dir`.
///
/// Returns the angle in [0, π] radians. Returns 0 if either vector is zero.
///
/// # Overflow analysis
///
/// Velocity components are bounded by v_max ≤ 12 m/s; direction components are
/// bounded by pitch dimensions ≤ 125 m. Squares: vel² ≤ 144, dir² ≤ 15625.
/// Both are well within Q32 range. Magnitudes (via sqrt) are bounded.
/// Dot product: vel_x * dir_x ≤ 12 × 125 = 1500; sum ≤ 3000 — within Q32.
/// Bare operators panic on violation.
fn angular_penalty(vel: (Q32, Q32), dir: (Q32, Q32)) -> Q32 {
    // If player is stationary, no directional penalty — they turn immediately.
    if vel.0 == Q32::ZERO && vel.1 == Q32::ZERO {
        return Q32::ZERO;
    }
    // If destination is at current position, no angle to penalize.
    if dir.0 == Q32::ZERO && dir.1 == Q32::ZERO {
        return Q32::ZERO;
    }

    let vel_mag = (vel.0 * vel.0 + vel.1 * vel.1).sqrt();
    let dir_mag = (dir.0 * dir.0 + dir.1 * dir.1).sqrt();

    if vel_mag == Q32::ZERO || dir_mag == Q32::ZERO {
        return Q32::ZERO;
    }

    // Dot product: bounded by vel_mag * dir_mag (from Cauchy-Schwarz).
    let dot = vel.0 * dir.0 + vel.1 * dir.1;

    // cos θ = dot / (|vel| × |dir|). Denominator > 0 (both mags checked above).
    let denom = vel_mag * dir_mag;
    let cos_theta = dot / denom;

    // Clamp cos_theta to [-1, 1] before passing to acos (guards against
    // tiny CORDIC-rounding overshoots). Bare clamp — values are very close
    // to the boundary so no overflow risk.
    let cos_clamped = cos_theta.max(Q32::from_raw(-(1i64 << 32))).min(Q32::ONE);

    // acos via Q32::acos (backed by cordic). Result in [0, π] radians.
    cos_clamped.acos()
}

/// Mean tau across all players.
///
/// # Overflow analysis
///
/// tau per player ≤ ~12 s (see `player_tau` overflow analysis). With 22 players,
/// sum ≤ 22 × 12 = 264 — well within Q32. Bare operators panic on violation.
fn mean_tau(taus: &[(PlayerId, Q32, bool)]) -> Q32 {
    if taus.is_empty() {
        return Q32::ZERO;
    }
    let mut sum = Q32::ZERO;
    for &(_, tau, _) in taus {
        sum += tau;
    }
    let n = Q32::from_raw((taus.len() as i64) << 32);
    sum / n
}

/// Sum arrival probabilities for one team.
///
/// P_arrive_i = sigmoid((mean_tau - tau_i) / sigma)
///
/// # Overflow analysis
///
/// `delta = mean_tau - tau_i` is bounded by the tau range (~[-12, +12]). After
/// dividing by SIGMA (0.45), the sigmoid argument is in roughly [-27, +27];
/// sigmoid saturates at ±8 so the result is in [0, 1]. sum ≤ 11 (11 outfield
/// players + GK = 12 max per team). Bare operators panic on violation.
fn sum_arrive_prob(taus: &[(PlayerId, Q32, bool)], mean_tau: Q32, is_attacker: bool) -> Q32 {
    let mut total = Q32::ZERO;
    for &(_, tau, attacker) in taus {
        if attacker != is_attacker {
            continue;
        }
        let delta = mean_tau - tau;
        let scaled = delta / SIGMA;
        let prob = sigmoid_q32(scaled);
        total += prob;
    }
    total
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

    fn pid(n: u32) -> PlayerId {
        PlayerId(n)
    }

    fn snap_at(px: f64, py: f64, vmax: f64) -> PlayerSnapshot {
        PlayerSnapshot {
            pos: (q(px), q(py)),
            vel: (Q32::ZERO, Q32::ZERO),
            v_max: q(vmax),
        }
    }

    #[test]
    fn empty_slices_return_zero_control() {
        let outcome = pitch_control((Q32::ZERO, Q32::ZERO), &[], &[]);
        assert_eq!(outcome.attacker_control, Q32::ZERO);
        assert_eq!(outcome.defender_control, Q32::ZERO);
        assert!(outcome.fastest_arrival.is_none());
    }

    #[test]
    fn closer_player_has_higher_control() {
        // Attacker at (0,0), defender at (20, 0). Query point at (5, 0).
        // Attacker is much closer — should have higher attacker control.
        let point = (q(5.0), q(0.0));
        let attackers = vec![(pid(1), snap_at(0.0, 0.0, 8.0))];
        let defenders = vec![(pid(2), snap_at(20.0, 0.0, 8.0))];
        let out = pitch_control(point, &attackers, &defenders);
        assert!(
            out.attacker_control > out.defender_control,
            "closer attacker should control: att={:?} def={:?}",
            out.attacker_control,
            out.defender_control
        );
    }

    #[test]
    fn control_values_in_unit_range() {
        let point = (q(10.0), q(5.0));
        let attackers = vec![
            (pid(1), snap_at(5.0, 5.0, 7.0)),
            (pid(2), snap_at(8.0, 3.0, 7.5)),
        ];
        let defenders = vec![
            (pid(3), snap_at(12.0, 5.0, 8.0)),
            (pid(4), snap_at(15.0, 7.0, 7.0)),
        ];
        let out = pitch_control(point, &attackers, &defenders);
        assert!(out.attacker_control >= Q32::ZERO);
        assert!(out.attacker_control <= Q32::ONE);
        assert!(out.defender_control >= Q32::ZERO);
        assert!(out.defender_control <= Q32::ONE);
    }

    #[test]
    fn fastest_arrival_is_nearest_stationary_player() {
        // Two stationary attackers; one much closer.
        let point = (q(10.0), q(0.0));
        let attackers = vec![
            (pid(10), snap_at(8.0, 0.0, 7.0)),  // 2m away — faster
            (pid(11), snap_at(25.0, 0.0, 7.0)), // 15m away — slower
        ];
        let out = pitch_control(point, &attackers, &[]);
        assert_eq!(
            out.fastest_arrival,
            Some(pid(10)),
            "nearest player should be fastest"
        );
    }

    #[test]
    fn player_tau_increases_with_distance() {
        let snap = snap_at(0.0, 0.0, 7.0);
        let tau_near = player_tau(&snap, (q(2.0), q(0.0)));
        let tau_far = player_tau(&snap, (q(20.0), q(0.0)));
        assert!(
            tau_far > tau_near,
            "further point should take longer to reach"
        );
    }

    #[test]
    fn angular_penalty_zero_for_stationary() {
        let vel = (Q32::ZERO, Q32::ZERO);
        let dir = (q(5.0), q(0.0));
        assert_eq!(angular_penalty(vel, dir), Q32::ZERO);
    }

    #[test]
    fn angular_penalty_zero_for_aligned_velocity() {
        // Velocity perfectly aligned with direction — angle = 0.
        let vel = (q(3.0), Q32::ZERO);
        let dir = (q(5.0), Q32::ZERO);
        let penalty = angular_penalty(vel, dir);
        // Should be near-zero (rounding may introduce a tiny value).
        assert!(
            penalty.to_bits() < 1_000_000,
            "aligned vel penalty should be near zero, got raw {}",
            penalty.to_bits()
        );
    }
}

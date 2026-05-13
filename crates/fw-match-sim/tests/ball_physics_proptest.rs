//! Proptest invariants for the T1-2b-i ball physics integrator.
//!
//! Per `docs/MASTER_PLAN.md` T1-2b-i done-criteria:
//! - **Energy monotone-decreasing**: kinetic energy at tick N ≤ kinetic
//!   energy at tick 0 (with a small overshoot epsilon for bounce-tick
//!   transients — see test rationale below).
//! - **Never goes infinite**: no Q32 overflow over 1800 ticks (30s) for
//!   any plausible initial state.
//! - **Bounce coefficients in archetype range**: `BallPhysicsCoefficients`
//!   validators reject super-balls + velocity-reversing drag. (The
//!   `BallPhysicsCoefficients::is_well_formed` predicate covers this in
//!   the lib tests; this proptest sanity-checks that the predicate
//!   matches plausible random inputs.)
//!
//! The integrator is a pure function; proptest seeds the initial state
//! with bounded random Q32 values. No system clocks, no thread RNG.

use fw_core::Q32;
use fw_match_sim::{BallPhysicsCoefficients, BallState, ball_physics, phase1_seeds};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Squared magnitude of a 3-vector, in Q32. Used as a kinetic-energy
/// proxy (real KE is `½ m v²`; with mass = 1 the constants drop out).
fn vel_squared(b: &BallState) -> Q32 {
    b.vel_x * b.vel_x + b.vel_y * b.vel_y + b.vel_z * b.vel_z
}

// ---------------------------------------------------------------------------
// Strategies — bounded random initial states
// ---------------------------------------------------------------------------

/// A Q32 value in `[-max, max]`. The proptest macro can't directly
/// construct Q32 from a float; we go via integer-then-divide using
/// Q32's panic-on-overflow operators.
fn q32_in_range(max_abs: i32) -> impl Strategy<Value = Q32> {
    (-max_abs..=max_abs).prop_map(Q32::from_int)
}

/// A random ball state with bounded position + velocity + spin.
/// Position bound 100m on each axis (well inside pitch-ish);
/// velocity bound 50 m/s (faster than a real shot but inside Q32 range
/// without overflow risk over 1800 ticks);
/// spin bound 30 rad/s (real ball spin maxes around 10 rad/s; 30 is
/// generous).
fn arb_ball_state() -> impl Strategy<Value = BallState> {
    (
        q32_in_range(100),
        q32_in_range(100),
        q32_in_range(100),
        q32_in_range(50),
        q32_in_range(50),
        q32_in_range(50),
        q32_in_range(30),
        q32_in_range(30),
        q32_in_range(30),
    )
        .prop_map(|(px, py, pz, vx, vy, vz, sx, sy, sz)| BallState {
            pos_x: px,
            pos_y: py.max(Q32::ZERO), // ball above-ground only (Y ≥ 0)
            pos_z: pz,
            vel_x: vx,
            vel_y: vy,
            vel_z: vz,
            spin_x: sx,
            spin_y: sy,
            spin_z: sz,
        })
}

// ---------------------------------------------------------------------------
// Invariants
// ---------------------------------------------------------------------------

proptest! {
    /// Energy never INJECTED: with phase1 coefficients (drag > 0,
    /// bounce_retention < 1, rolling_friction > 0, gravity > 0), the
    /// ball loses kinetic + potential energy over time. We assert the
    /// weaker "post-tick velocity² ≤ pre-tick velocity² + ε" because:
    ///
    /// - Gravity adds vertical KE when the ball is rising (it converts
    ///   potential energy to KE). Over a single tick this can mean
    ///   |v|² increases. We allow this transient.
    /// - Bounce momentarily flips the sign but reduces magnitude
    ///   (retention < 1).
    /// - Magnus (with phase1 coupling = 0) is a no-op.
    ///
    /// Over a 1-tick step, |v_new|² ≤ |v_old|² + 2 · g · dt · |v_old|.
    /// The epsilon below is a generous bound that covers the gravity
    /// term across the strategy's velocity range (up to 50 m/s).
    /// Anything beyond this is energy injection — a real bug.
    #[test]
    fn energy_doesnt_grow_unboundedly_in_one_tick(state in arb_ball_state()) {
        let coeffs = phase1_seeds();
        let before_v_sq = vel_squared(&state);
        let after = ball_physics::ball_step(&state, &coeffs);
        let after_v_sq = vel_squared(&after);

        // Epsilon for the gravity transient: 2 * g * dt * v_max where
        // v_max is the strategy's 50 m/s bound. 2 * 9.81 * (1/60) * 50
        // ≈ 16.35. We round up to 20 and double for safety margin
        // (proptest is fuzzing the strategy's edge cases; we want the
        // bound loose enough to never false-positive but tight enough
        // to catch real energy injection).
        let epsilon = Q32::from_int(40);
        prop_assert!(
            after_v_sq <= before_v_sq + epsilon,
            "energy injection: before_v² = {:?}, after_v² = {:?}, ε = {:?}",
            before_v_sq,
            after_v_sq,
            epsilon
        );
    }

    /// Over a long run (1800 ticks = 30s), the integrator never panics
    /// from Q32 overflow. The Codex Q1 operator policy panics on
    /// overflow rather than wrapping silently; this proptest exercises
    /// the integrator's interior arithmetic across the strategy's range.
    ///
    /// `tick_match` calls `ball_step` once per tick; we call `ball_step`
    /// directly here so a single panic surfaces as a proptest counter-
    /// example with shrinking, not a generic "the test failed".
    #[test]
    fn no_overflow_over_30s_simulation(state in arb_ball_state()) {
        let coeffs = phase1_seeds();
        let mut s = state;
        for _ in 0..1800 {
            s = ball_physics::ball_step(&s, &coeffs);
        }
        // Sanity: after 1800 ticks the ball should be settled or
        // bounded. We don't check exact position, just that the
        // integrator didn't blow up.
        prop_assert!(s.pos_y >= Q32::ZERO, "ball below ground after long run");
    }

    /// The `is_well_formed` validator on `BallPhysicsCoefficients`
    /// rejects out-of-range inputs deterministically.
    #[test]
    fn coefficients_validator_rejects_out_of_range(
        gravity_int in -100i32..=100i32,
        drag_pct in -200i32..=200i32,
        magnus_pct in -200i32..=200i32,
        bounce_pct in -200i32..=200i32,
        friction_pct in -200i32..=200i32,
    ) {
        let coeffs = BallPhysicsCoefficients {
            gravity: Q32::from_int(gravity_int),
            linear_drag: Q32::from_int(drag_pct) / Q32::from_int(100),
            magnus_coupling: Q32::from_int(magnus_pct) / Q32::from_int(100),
            bounce_retention: Q32::from_int(bounce_pct) / Q32::from_int(100),
            rolling_friction: Q32::from_int(friction_pct) / Q32::from_int(100),
        };
        // The validator should match the explicit predicate.
        let all_in_range = gravity_int >= 0
            && (0..=100).contains(&drag_pct)
            && (0..=100).contains(&magnus_pct)
            && (0..=100).contains(&bounce_pct)
            && (0..=100).contains(&friction_pct);
        prop_assert_eq!(coeffs.is_well_formed(), all_in_range);
    }
}

//! Ball physics integrator — deterministic Q32 semi-implicit Euler.
//!
//! Ported from the v1 design at `MatchSim/Sim/BallPhysics.cs` (NOT code
//! — Rust idioms only). Forces applied per 60Hz tick: gravity, linear
//! drag, Magnus (when spin and coupling are both non-zero), then
//! semi-implicit position update, ground bounce + rolling friction.
//!
//! ## Coordinate convention
//!
//! X + Z form the pitch plane (X = attacking axis; Z = touchline-to-
//! touchline). Y is altitude; gravity acts on -Y. Ground is the half-space
//! `Y <= 0`; the integrator clamps to `Y = 0` on contact. Coordinate
//! convention matches FW v1 + the `frontend/src/routes/Dev/TacticalBoard.tsx`
//! renderer convention.
//!
//! ## Determinism contract
//!
//! - Q32 throughout (no `f32` / `f64` outside `crate::dto`'s viewer-only
//!   module). Crate-wide `#[lints.clippy.float_arithmetic = "deny"]`.
//! - No `Instant::now()`, no `thread_rng`, no `HashMap`. The integrator
//!   is a pure function `(state, coefficients) -> state`.
//! - The fixed timestep `DT_PER_TICK` is `1/60` in Q32 (raw bits
//!   `(2^32) / 60 = 71_582_788` plus the integer part contribution; we
//!   build it as `Q32::ONE / Q32::from_int(60)` so the division uses
//!   the panic-on-overflow Q32 operator policy from Codex Q1).
//!
//! ## Magnus stub at T1
//!
//! `phase1_seeds().magnus_coupling` is `Q32::ZERO`. This is the stub
//! policy from v1's `BallPhysics.cs` doc-comment carried into T1: the
//! Magnus block is structurally present in `ball_step` for future tuning
//! (curved passes + finishing at T1-2b-iii), but contributes zero to the
//! integrator output today. The spin × velocity cross product still runs
//! (deterministically); the result is multiplied by zero and added.

use fw_core::Q32;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Coefficients
// ---------------------------------------------------------------------------

/// Tuning seeds for the ball-physics integrator. Mix of continuous-time
/// SI quantities (`gravity` in m/s²) and per-step dimensionless
/// coefficients (`linear_drag`, `magnus_coupling`, `bounce_retention`,
/// `rolling_friction` — all already absorbing `dt` per the v1
/// design doc). Re-tunable in `phase1_seeds()`; not committed to
/// canonical state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BallPhysicsCoefficients {
    /// Gravitational acceleration (m/s²). Continuous SI; the integrator
    /// scales by `DT_PER_TICK` each step.
    pub gravity: Q32,
    /// Linear air-drag coefficient, per-step dimensionless.
    /// `v_new = v * (1 - C_d)`. Must be in `[0, 1]`; >= 1 would reverse
    /// velocity each tick. This is the dimensionless coefficient, NOT
    /// the drag force itself.
    pub linear_drag: Q32,
    /// Magnus coupling, per-step dimensionless. `v_new +=
    /// magnus_coupling * (spin × v)`. Zero for T1 (see module doc).
    pub magnus_coupling: Q32,
    /// Vertical bounce retention `e`. Post-bounce `v.y = -e * v.y`.
    /// Must be in `[0, 1]`; >= 1 would be a super-ball.
    pub bounce_retention: Q32,
    /// Rolling friction, per-step. `v.{x,z}_new = v.{x,z} * (1 - μ)`.
    /// Must be in `[0, 1]`.
    pub rolling_friction: Q32,
}

impl BallPhysicsCoefficients {
    /// Predicate: are all coefficients in their valid range?
    /// `linear_drag`, `bounce_retention`, `rolling_friction`, and
    /// `magnus_coupling` must be in `[0, 1]`; `gravity` must be `>= 0`
    /// (negative gravity would float the ball — physically wrong + a
    /// silent bug source).
    #[must_use]
    pub fn is_well_formed(&self) -> bool {
        self.gravity >= Q32::ZERO
            && in_unit_range(self.linear_drag)
            && in_unit_range(self.magnus_coupling)
            && in_unit_range(self.bounce_retention)
            && in_unit_range(self.rolling_friction)
    }
}

fn in_unit_range(q: Q32) -> bool {
    q >= Q32::ZERO && q <= Q32::ONE
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// 60 Hz integration tick rate. Mirrors `fw_core::tick::TICKS_PER_SECOND`
/// without taking a dep on the constant (Q32 division is cheap; the
/// duplication is local + this module pins the contract).
const TICKS_PER_SECOND: i32 = 60;

/// Phase-1 (T1) seed coefficients. Values lifted from v1's
/// `BallPhysics.cs::Phase3SeedValues`, with `magnus_coupling` zeroed per
/// the stub policy for T1 playability. Hand-tuned re-tune is expected
/// during T1-2b-iii visual playtests and again at the T1 exit gate.
#[must_use]
pub fn phase1_seeds() -> BallPhysicsCoefficients {
    BallPhysicsCoefficients {
        // 9.81 m/s² = 981/100. Q32 division uses the panic-on-overflow
        // operator policy.
        gravity: Q32::from_int(981) / Q32::from_int(100),
        // 0.02 per-step linear drag.
        linear_drag: Q32::from_int(2) / Q32::from_int(100),
        // STUB for T1 — Magnus contributes zero behaviorally; the
        // structure is present in ball_step for T1-2b-iii.
        magnus_coupling: Q32::ZERO,
        // 0.55 bounce retention.
        bounce_retention: Q32::from_int(55) / Q32::from_int(100),
        // 0.25 rolling friction.
        rolling_friction: Q32::from_int(25) / Q32::from_int(100),
    }
}

/// Fixed-tick integration timestep (1/60 s) as Q32. Per-step force
/// scaling for the gravity term (gravity is continuous SI; drag /
/// Magnus / friction / bounce are per-step coefficients that already
/// absorb dt). Public so test helpers can construct synthetic single-
/// tick advances at the same dt the integrator uses.
#[must_use]
pub fn dt_per_tick() -> Q32 {
    Q32::ONE / Q32::from_int(TICKS_PER_SECOND)
}

// ---------------------------------------------------------------------------
// Integrator
// ---------------------------------------------------------------------------

/// Step the ball one 60Hz tick forward. Pure function: same input ⇒
/// same output across runs and platforms (the determinism floor).
///
/// Sequence per tick (matches FW v1 carry-forward design):
/// 1. Gravity: `v.y -= g * dt`.
/// 2. Linear drag: `v *= (1 - C_d)`.
/// 3. Magnus: `v += C_m * (spin × v)` (cross product). Per phase1
///    seeds `C_m = 0`, the cross product runs deterministically but
///    contributes zero.
/// 4. Semi-implicit position update: `p += v_new * dt` (uses POST-force
///    velocity, hence "semi-implicit" — gives better stability than
///    explicit Euler for stiff systems).
/// 5. Ground contact: if `p.y <= 0`, clamp `p.y = 0`; on a fresh
///    air-to-ground crossing with `v.y < 0`, bounce by flipping +
///    scaling `v.y *= -bounce_retention`; otherwise zero out `v.y` for
///    rolling state. Apply rolling friction to `v.x` + `v.z` when
///    settled (post-clamp `v.y <= 0`).
#[must_use]
pub fn ball_step(state: &crate::BallState, coeffs: &BallPhysicsCoefficients) -> crate::BallState {
    // Debug-only invariant gates. is_well_formed has no production
    // callers without this assert (Codex P1 — silent-failure-hunter +
    // type-design-analyzer convergent): out-of-range coefficients
    // would silently produce velocity-reversing drag or super-ball
    // bounce and look like a determinism regression. The
    // pos_y >= 0 gate catches upstream bugs that put the ball below
    // ground; the contact step clamps regardless, but the upstream
    // mutation source deserves to surface, not get laundered.
    debug_assert!(
        coeffs.is_well_formed(),
        "ball_step called with malformed coefficients: {coeffs:?}"
    );
    debug_assert!(
        state.pos_y >= Q32::ZERO,
        "ball_step entered with subterranean ball: pos_y = {:?}",
        state.pos_y
    );

    let dt = dt_per_tick();
    let started_on_ground = state.pos_y <= Q32::ZERO;

    // 1. Gravity: v.y -= g * dt. (Continuous SI; dt-scaled here.)
    let mut vx = state.vel_x;
    let mut vy = state.vel_y - coeffs.gravity * dt;
    let mut vz = state.vel_z;

    // 2. Linear drag (per-step coefficient; absorbs dt).
    let drag_retention = Q32::ONE - coeffs.linear_drag;
    vx *= drag_retention;
    vy *= drag_retention;
    vz *= drag_retention;

    // 3. Magnus: v += C_m * (spin × v). Cross-product layout —
    //    spin × v = (sy·vz - sz·vy, sz·vx - sx·vz, sx·vy - sy·vx).
    //    With phase1 magnus_coupling = 0, this contributes zero
    //    deterministically; the multiply still runs so the Q32 path
    //    is exercised identically regardless of coupling value.
    let mx = state.spin_y * vz - state.spin_z * vy;
    let my = state.spin_z * vx - state.spin_x * vz;
    let mz = state.spin_x * vy - state.spin_y * vx;
    vx += coeffs.magnus_coupling * mx;
    vy += coeffs.magnus_coupling * my;
    vz += coeffs.magnus_coupling * mz;

    // 4. Semi-implicit position update: p += v_new * dt. `py` is `mut`
    //    because the ground-contact step rewrites it; `px`/`pz` aren't
    //    rewritten (out-of-bounds detection is T1-4's MatchEvent layer).
    let px = state.pos_x + vx * dt;
    let mut py = state.pos_y + vy * dt;
    let pz = state.pos_z + vz * dt;

    // 5. Ground contact.
    if py <= Q32::ZERO {
        let crossed_into_ground_from_air = !started_on_ground && vy < Q32::ZERO;

        // Clamp to the ground plane. Subterranean drift would compound
        // across season-long replays.
        py = Q32::ZERO;

        // `did_bounce` gates rolling friction explicitly. Relying on the
        // sign of post-bounce vy (Codex P0 finding) is brittle: a
        // bounce_retention of Q32::ZERO (valid per is_well_formed) leaves
        // vy at exactly zero, which would falsely satisfy the
        // "settled" guard below and double-decay horizontal velocity on
        // the same tick as the contact. The flag separates "this tick
        // performed a bounce" from "this tick is rolling on the ground".
        let mut did_bounce = false;
        if crossed_into_ground_from_air {
            // Bounce: vertical velocity flipped + scaled by retention.
            vy = -(coeffs.bounce_retention * vy);
            did_bounce = true;
        } else if vy < Q32::ZERO {
            // Ball was already grounded; gravity nudged vy slightly
            // negative this tick. Zero it rather than introduce a tiny
            // spurious bounce.
            vy = Q32::ZERO;
        }

        // Rolling friction: applies only when settled — explicitly NOT
        // on a bounce tick, regardless of post-bounce vy magnitude.
        if !did_bounce && vy <= Q32::ZERO {
            let roll_retention = Q32::ONE - coeffs.rolling_friction;
            vx *= roll_retention;
            vz *= roll_retention;
        }
    }

    // Bound pos_x / pos_z aren't clamped — out-of-bounds detection is
    // T1-4's MatchEvent layer. Spin advances rigidly through this step
    // (no spin damping in T1; T1-2b-iii adds spin-drag if needed).
    crate::BallState {
        pos_x: px,
        pos_y: py,
        pos_z: pz,
        vel_x: vx,
        vel_y: vy,
        vel_z: vz,
        spin_x: state.spin_x,
        spin_y: state.spin_y,
        spin_z: state.spin_z,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ball::BallState;

    /// T1-2b-i Chunk 2 RED: phase1 seeds match v1 design values.
    /// Drift from these numbers needs to be deliberate (tuning pass) +
    /// trip the canonical-hash regression — this test pins the source
    /// values, not the resulting hash.
    #[test]
    fn phase1_seed_values() {
        let s = phase1_seeds();
        // 9.81 → bits = round(9.81 * 2^32). Test by reconstructing.
        assert_eq!(s.gravity, Q32::from_int(981) / Q32::from_int(100));
        assert_eq!(s.linear_drag, Q32::from_int(2) / Q32::from_int(100));
        // Magnus is the T1 stub.
        assert_eq!(s.magnus_coupling, Q32::ZERO);
        assert_eq!(s.bounce_retention, Q32::from_int(55) / Q32::from_int(100));
        assert_eq!(s.rolling_friction, Q32::from_int(25) / Q32::from_int(100));
    }

    #[test]
    fn phase1_seeds_are_well_formed() {
        assert!(phase1_seeds().is_well_formed());
    }

    #[test]
    fn validator_rejects_super_ball_bounce() {
        let mut bad = phase1_seeds();
        bad.bounce_retention = Q32::from_int(2); // super-ball
        assert!(!bad.is_well_formed());
    }

    #[test]
    fn validator_rejects_velocity_reversing_drag() {
        let mut bad = phase1_seeds();
        bad.linear_drag = Q32::from_int(2); // > 1 would flip velocity
        assert!(!bad.is_well_formed());
    }

    #[test]
    fn validator_rejects_negative_gravity() {
        let mut bad = phase1_seeds();
        bad.gravity = -Q32::from_int(5);
        assert!(!bad.is_well_formed());
    }

    #[test]
    fn validator_rejects_friction_above_one() {
        let mut bad = phase1_seeds();
        bad.rolling_friction = Q32::from_int(2);
        assert!(!bad.is_well_formed());
    }

    #[test]
    fn dt_per_tick_is_one_sixtieth() {
        assert_eq!(dt_per_tick(), Q32::ONE / Q32::from_int(60));
    }

    // -----------------------------------------------------------------
    // T1-2b-i Chunk 3 — ball_step integrator
    // -----------------------------------------------------------------

    /// A ball at rest on the ground should stay at rest. Gravity is the
    /// only force applied per tick to a non-ground ball; on the ground
    /// it's neutralised by the ground-contact clamp.
    #[test]
    fn ball_at_rest_on_ground_stays_at_rest() {
        let before = BallState::centre_spot();
        let after = ball_step(&before, &phase1_seeds());
        assert_eq!(after.pos_y, Q32::ZERO);
        assert_eq!(after.vel_y, Q32::ZERO);
        assert_eq!(after.vel_x, Q32::ZERO);
        assert_eq!(after.vel_z, Q32::ZERO);
    }

    /// A ball dropped from altitude with no horizontal velocity should
    /// accelerate downward by `g * dt` in one tick.
    #[test]
    fn dropped_ball_gains_downward_velocity_from_gravity() {
        let before = BallState {
            pos_y: Q32::from_int(5), // 5m up
            ..BallState::centre_spot()
        };
        let after = ball_step(&before, &phase1_seeds());
        // vel_y should be negative (going down) after one tick.
        assert!(
            after.vel_y < Q32::ZERO,
            "post-step vel_y = {:?}",
            after.vel_y
        );
        // Magnitude: g * dt = 9.81 / 60 ≈ 0.1635 m/s after 1 tick. With
        // 0.02 drag applied AFTER gravity, vel_y ≈ -0.1635 * 0.98 =
        // -0.16023. We assert range to leave room for Q32 rounding.
        let expected_min = -Q32::from_int(17) / Q32::from_int(100); // -0.17
        let expected_max = -Q32::from_int(15) / Q32::from_int(100); // -0.15
        assert!(
            after.vel_y > expected_min && after.vel_y < expected_max,
            "vel_y after 1 tick of free fall = {:?}; expected ~-0.16",
            after.vel_y
        );
    }

    /// Horizontal velocity decays under linear drag. Ball moving along +X
    /// at altitude loses 2% per tick (phase1_seeds drag = 0.02). Test
    /// uses `pos_y = 5` (5m up) to isolate AIR drag from the rolling
    /// friction that ground contact would apply.
    #[test]
    fn horizontal_velocity_decays_under_drag() {
        let before = BallState {
            pos_y: Q32::from_int(5),  // 5m up — air drag only
            vel_x: Q32::from_int(10), // 10 m/s along +X
            ..BallState::centre_spot()
        };
        let after = ball_step(&before, &phase1_seeds());
        // 10 * 0.98 = 9.8 m/s — assert vel_x is between 9.7 and 9.9.
        assert!(
            after.vel_x > Q32::from_int(97) / Q32::from_int(10)
                && after.vel_x < Q32::from_int(99) / Q32::from_int(10),
            "vel_x after 1 tick of drag at altitude = {:?}; expected ~9.8",
            after.vel_x
        );
    }

    /// A ball moving downward through the ground plane in one tick
    /// bounces back up with reduced magnitude (e = 0.55 retention).
    /// The integrator clamps Y to zero on contact then flips + scales
    /// vel_y.
    #[test]
    fn ground_bounce_reverses_and_dampens_vertical_velocity() {
        let before = BallState {
            pos_y: Q32::ONE / Q32::from_int(10), // 0.1m up
            vel_y: -Q32::from_int(10),           // 10 m/s downward
            ..BallState::centre_spot()
        };
        let after = ball_step(&before, &phase1_seeds());
        // Post-bounce: vel_y = -e * vel_y_after_gravity. Gravity adds
        // -9.81/60 ≈ -0.16 to vel_y first, then drag scales by 0.98,
        // then bounce flips + scales by 0.55. So roughly
        // vel_y ≈ -0.55 * 0.98 * (-10 - 0.16) ≈ +5.48 m/s.
        assert!(
            after.vel_y > Q32::from_int(50) / Q32::from_int(10), // > 5.0
            "post-bounce vel_y = {:?}; expected ~5.5",
            after.vel_y
        );
        assert_eq!(after.pos_y, Q32::ZERO, "ball not clamped to ground");
    }

    /// A ball rolling on the ground (vel_y = 0, on ground) loses
    /// horizontal velocity to rolling friction in addition to drag.
    /// μ = 0.25, so 75% retained per tick on x/z; 0.98 drag stacks on
    /// the same axes.
    #[test]
    fn ground_ball_loses_horizontal_velocity_to_friction() {
        let before = BallState {
            vel_x: Q32::from_int(10),
            // Ball is on the ground; vel_y starts at zero.
            ..BallState::centre_spot()
        };
        let after = ball_step(&before, &phase1_seeds());
        // vel_x after one tick: drag scales by 0.98, then rolling
        // friction scales by 0.75. 10 * 0.98 * 0.75 = 7.35 m/s.
        assert!(
            after.vel_x > Q32::from_int(72) / Q32::from_int(10)  // > 7.2
                && after.vel_x < Q32::from_int(75) / Q32::from_int(10), // < 7.5
            "vel_x after 1 tick of rolling friction = {:?}; expected ~7.35",
            after.vel_x
        );
    }

    /// Codex P0 regression — a bounce_retention of exactly zero must
    /// NOT cause rolling friction to fire on the same tick as the
    /// contact. Pre-fix, vy would land at exactly zero after the bounce
    /// branch, satisfy the `vy <= Q32::ZERO` rolling-friction guard, and
    /// double-decay horizontal velocity. The `did_bounce` flag prevents
    /// that. We assert horizontal velocity is only decayed by drag
    /// (× 0.98), not drag-then-friction (× 0.98 × 0.75).
    #[test]
    fn zero_bounce_retention_does_not_apply_rolling_friction_on_contact() {
        let coeffs = BallPhysicsCoefficients {
            bounce_retention: Q32::ZERO,
            ..phase1_seeds()
        };
        let before = BallState {
            pos_y: Q32::ONE / Q32::from_int(10), // 0.1m up
            vel_x: Q32::from_int(10),
            vel_y: -Q32::from_int(10), // diving downward — bounce tick
            ..BallState::centre_spot()
        };
        let after = ball_step(&before, &coeffs);
        // Post-step vel_y is zero (bounce flipped × 0 = 0).
        assert_eq!(
            after.vel_y,
            Q32::ZERO,
            "vel_y should be zero after dead bounce"
        );
        // Horizontal velocity should be 10 * 0.98 = 9.8 (drag only).
        // If rolling friction leaked, it would be 10 * 0.98 * 0.75 = 7.35.
        assert!(
            after.vel_x > Q32::from_int(97) / Q32::from_int(10),
            "vel_x = {:?}; rolling friction leaked onto bounce tick \
             (expected drag-only ≈ 9.8, friction-stacked would be ≈ 7.35)",
            after.vel_x
        );
    }

    /// Pure-function determinism: same input → same output, regardless
    /// of call order or context.
    #[test]
    fn ball_step_is_pure_function() {
        let state = BallState {
            pos_x: Q32::from_int(3),
            pos_y: Q32::from_int(2),
            vel_x: Q32::from_int(5),
            vel_y: Q32::from_int(-1),
            spin_z: Q32::from_int(1),
            ..BallState::centre_spot()
        };
        let coeffs = phase1_seeds();
        let a = ball_step(&state, &coeffs);
        let b = ball_step(&state, &coeffs);
        assert_eq!(a, b);
    }
}

//! Ball physics integrator — deterministic Q32 semi-implicit Euler.
//!
//! Ported from the v1 design at `MatchSim/Sim/BallPhysics.cs` (NOT code
//! — Rust idioms only). Forces applied per 60Hz tick: gravity, linear
//! drag, Magnus (when spin and coupling are both non-zero), then
//! semi-implicit position update, ground bounce + rolling friction.
//!
//! ## Coordinate convention (T1-3.5 corrected)
//!
//! X + Y form the pitch plane (X = attacking axis; Y = lateral/touchline).
//! Z is altitude; gravity acts on -Z. Ground is the half-space `Z <= 0`;
//! the integrator clamps to `Z = 0` on contact. Rolling friction acts on
//! `vx` and `vy` when the ball is settled (pz = 0, not bouncing).
//!
//! This matches the authoritative 2D pitch convention used throughout
//! `fw-match-sim` (dispatch.rs, SIDELINE_Y, GOAL_HALF_WIDTH_M).
//! The prior doc-comment (X+Z = pitch, Y = altitude) was incorrect —
//! it conflicted with all pitch-geometry consumers. Fixed at T1-3.5.
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
    // Debug-only invariant gate. is_well_formed has no production
    // callers without this assert (Codex P1 — silent-failure-hunter +
    // type-design-analyzer convergent): out-of-range coefficients
    // would silently produce velocity-reversing drag or super-ball
    // bounce and look like a determinism regression.
    //
    // NOTE (T1-3.5, with comment corrected post-Codex 2026-05-16 audit):
    // The integrator uses the corrected coordinate convention — gravity
    // acts on `-vel_z`, ground contact is `pos_z <= 0` — so `pos_y` can
    // legitimately be any value in `[-SIDELINE_Y, +SIDELINE_Y]` without
    // triggering an altitude branch. The OOB clamp in `tick_match` step 3
    // (NOT step 8 — the prior comment cited the spec ordering before the
    // T1-3.5 ordering reversal; OOB clamp now runs BEFORE physics) zeros
    // ball.vel_x/vel_y + clamps pos to the boundary; this integrator
    // receives an in-bounds ball.
    debug_assert!(
        coeffs.is_well_formed(),
        "ball_step called with malformed coefficients: {coeffs:?}"
    );

    // T1-3.5 coordinate convention (corrected from ball_physics.rs v1):
    //
    //   pos_x / vel_x — pitch-length axis (attacking direction).
    //   pos_y / vel_y — lateral pitch axis (touchline-to-touchline). Matches
    //                   the 2D pitch convention in dispatch.rs, SIDELINE_Y,
    //                   GOAL_HALF_WIDTH_M, and all pitch-geometry constants.
    //                   Can be negative (left/south side of pitch).
    //   pos_z / vel_z — altitude axis. Ground = pos_z = 0. Gravity acts on
    //                   -vel_z. Ground contact clamps pos_z to 0.
    //
    // Prior v1 convention (`ball.rs` doc-comment) had X+Z as the pitch plane
    // and Y as altitude. This conflicted with all pitch-geometry consumers.
    // T1-3.5 fixes the physics to match the authoritative 2D convention.
    // Canonical hash re-baselines under ADR-0012 trigger #1.

    let dt = dt_per_tick();

    // Altitude tracking uses pos_z / vel_z.
    let started_on_ground = state.pos_z <= Q32::ZERO;

    // 1. Gravity: v.z -= g * dt. (Continuous SI; dt-scaled here.)
    let mut vx = state.vel_x;
    let mut vy = state.vel_y; // lateral — gravity does NOT act on lateral
    let mut vz = state.vel_z - coeffs.gravity * dt; // altitude vel

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

    // 4. Semi-implicit position update: p += v_new * dt. `pz` is `mut`
    //    because the ground-contact step rewrites it; `px`/`py` aren't
    //    rewritten by physics (OOB detection is the `tick_match` step 2/3).
    let px = state.pos_x + vx * dt;
    let py = state.pos_y + vy * dt; // lateral — no altitude clamping
    let mut pz = state.pos_z + vz * dt; // altitude — clamped on contact

    // 5. Ground contact.
    if pz <= Q32::ZERO {
        let crossed_into_ground_from_air = !started_on_ground && vz < Q32::ZERO;

        // Clamp to the ground plane. Subterranean drift would compound
        // across season-long replays.
        pz = Q32::ZERO;

        // `did_bounce` gates rolling friction explicitly. Relying on the
        // sign of post-bounce vz (Codex P0 finding) is brittle: a
        // bounce_retention of Q32::ZERO (valid per is_well_formed) leaves
        // vz at exactly zero, which would falsely satisfy the
        // "settled" guard below and double-decay horizontal velocity on
        // the same tick as the contact. The flag separates "this tick
        // performed a bounce" from "this tick is rolling on the ground".
        let mut did_bounce = false;
        if crossed_into_ground_from_air {
            // Bounce: vertical velocity flipped + scaled by retention.
            vz = -(coeffs.bounce_retention * vz);
            did_bounce = true;
        } else if vz < Q32::ZERO {
            // Ball was already grounded; gravity nudged vz slightly
            // negative this tick. Zero it rather than introduce a tiny
            // spurious bounce.
            vz = Q32::ZERO;
        }

        // Rolling friction: applies only when settled — explicitly NOT
        // on a bounce tick, regardless of post-bounce vz magnitude.
        if !did_bounce && vz <= Q32::ZERO {
            let roll_retention = Q32::ONE - coeffs.rolling_friction;
            vx *= roll_retention;
            vy *= roll_retention;
        }
    }

    // pos_x / pos_y aren't clamped — out-of-bounds detection is the
    // tick_match step 2/3 (goal detection + OOB clamp). Spin advances
    // rigidly through this step (no spin damping in T1).
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

    /// **T1-3.5 fix-pass regression test** per Codex 2026-05-16 audit
    /// type-design P1: gravity acts on the altitude axis (`vel_z`), NOT
    /// on either pitch-plane axis (`vel_x`/`vel_y`). The prior coordinate
    /// convention had `pos_y` as altitude — visually equivalent in the
    /// 6 existing ball-physics tests (which only checked numeric outputs,
    /// not which axis they apply to) but semantically wrong. This test
    /// pins the correct axis so a future regression to "Y = altitude"
    /// fails loudly.
    ///
    /// Setup: ball at rest at `(0, 0, 5)` with `vel = (0, 5, 0)` — i.e.
    /// 5m altitude, no vertical motion, 5m/s lateral motion.
    ///
    /// Expectations after one tick:
    /// - `vel_z` becomes negative (gravity pulled it down).
    /// - `vel_y` stays positive but slightly reduced by drag (NOT zero,
    ///   NOT gravity-pulled).
    /// - `pos_z` decreases (ball falls).
    /// - `pos_y` increases (lateral motion advances).
    #[test]
    fn gravity_acts_on_altitude_axis_z_not_lateral_axis_y() {
        use fw_core::Q32;
        let coeffs = phase1_seeds();
        let initial = BallState {
            pos_x: Q32::ZERO,
            pos_y: Q32::ZERO,
            pos_z: Q32::from_int(5), // 5m altitude
            vel_x: Q32::ZERO,
            vel_y: Q32::from_int(5), // 5 m/s lateral
            vel_z: Q32::ZERO,
            spin_x: Q32::ZERO,
            spin_y: Q32::ZERO,
            spin_z: Q32::ZERO,
        };
        let after = ball_step(&initial, &coeffs);

        // Gravity decreased vel_z (altitude axis).
        assert!(
            after.vel_z < Q32::ZERO,
            "vel_z must be negative after gravity tick; got {:?}",
            after.vel_z.to_bits()
        );
        // vel_y (lateral) stayed positive — only drag, no gravity pull.
        assert!(
            after.vel_y > Q32::ZERO,
            "vel_y must stay positive (lateral motion, no gravity); got {:?}",
            after.vel_y.to_bits()
        );
        // vel_y reduced by linear_drag (was 5, now 5 × (1 - 0.02) ≈ 4.9).
        // NOT zero (would mean gravity hit the lateral axis = wrong convention).
        assert!(
            after.vel_y > Q32::from_int(4),
            "vel_y must retain most of its initial 5 m/s (only drag, no gravity); got {:?}",
            after.vel_y.to_bits()
        );
        // pos_z fell (ball descending).
        assert!(
            after.pos_z < Q32::from_int(5),
            "pos_z must decrease from initial 5m (ball falling); got {:?}",
            after.pos_z.to_bits()
        );
        // pos_y advanced (lateral motion).
        assert!(
            after.pos_y > Q32::ZERO,
            "pos_y must increase from 0 (lateral motion); got {:?}",
            after.pos_y.to_bits()
        );
    }

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
    /// T1-3.5: altitude axis is now pos_z/vel_z.
    #[test]
    fn ball_at_rest_on_ground_stays_at_rest() {
        let before = BallState::centre_spot();
        let after = ball_step(&before, &phase1_seeds());
        assert_eq!(after.pos_z, Q32::ZERO); // altitude stays at 0
        assert_eq!(after.vel_z, Q32::ZERO); // no vertical velocity
        assert_eq!(after.vel_x, Q32::ZERO);
        assert_eq!(after.vel_y, Q32::ZERO); // no lateral drift at rest
    }

    /// A ball dropped from altitude with no horizontal velocity should
    /// accelerate downward by `g * dt` in one tick.
    /// T1-3.5: altitude is pos_z; gravity acts on vel_z.
    #[test]
    fn dropped_ball_gains_downward_velocity_from_gravity() {
        let before = BallState {
            pos_z: Q32::from_int(5), // 5m up (altitude)
            ..BallState::centre_spot()
        };
        let after = ball_step(&before, &phase1_seeds());
        // vel_z should be negative (going down) after one tick.
        assert!(
            after.vel_z < Q32::ZERO,
            "post-step vel_z = {:?}",
            after.vel_z
        );
        // Magnitude: g * dt = 9.81 / 60 ≈ 0.1635 m/s after 1 tick. With
        // 0.02 drag applied AFTER gravity, vel_z ≈ -0.1635 * 0.98 =
        // -0.16023. We assert range to leave room for Q32 rounding.
        let expected_min = -Q32::from_int(17) / Q32::from_int(100); // -0.17
        let expected_max = -Q32::from_int(15) / Q32::from_int(100); // -0.15
        assert!(
            after.vel_z > expected_min && after.vel_z < expected_max,
            "vel_z after 1 tick of free fall = {:?}; expected ~-0.16",
            after.vel_z
        );
    }

    /// Horizontal velocity decays under linear drag. Ball moving along +X
    /// at altitude loses 2% per tick (phase1_seeds drag = 0.02). Test
    /// uses `pos_z = 5` (5m up) to isolate AIR drag from the rolling
    /// friction that ground contact would apply.
    #[test]
    fn horizontal_velocity_decays_under_drag() {
        let before = BallState {
            pos_z: Q32::from_int(5),  // 5m up (altitude) — air drag only
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
    /// The integrator clamps Z to zero on contact then flips + scales
    /// vel_z. T1-3.5: altitude axis is pos_z/vel_z.
    #[test]
    fn ground_bounce_reverses_and_dampens_vertical_velocity() {
        let before = BallState {
            pos_z: Q32::ONE / Q32::from_int(10), // 0.1m up (altitude)
            vel_z: -Q32::from_int(10),           // 10 m/s downward
            ..BallState::centre_spot()
        };
        let after = ball_step(&before, &phase1_seeds());
        // Post-bounce: vel_z = -e * vel_z_after_gravity. Gravity adds
        // -9.81/60 ≈ -0.16 to vel_z first, then drag scales by 0.98,
        // then bounce flips + scales by 0.55. So roughly
        // vel_z ≈ -0.55 * 0.98 * (-10 - 0.16) ≈ +5.48 m/s.
        assert!(
            after.vel_z > Q32::from_int(50) / Q32::from_int(10), // > 5.0
            "post-bounce vel_z = {:?}; expected ~5.5",
            after.vel_z
        );
        assert_eq!(after.pos_z, Q32::ZERO, "ball not clamped to ground");
    }

    /// A ball rolling on the ground (vel_z = 0, on ground) loses
    /// horizontal velocity to rolling friction in addition to drag.
    /// μ = 0.25, so 75% retained per tick on x/y; 0.98 drag stacks on
    /// the same axes. T1-3.5: altitude is pos_z; rolling friction now
    /// acts on vx and vy (pitch-plane velocities).
    #[test]
    fn ground_ball_loses_horizontal_velocity_to_friction() {
        let before = BallState {
            vel_x: Q32::from_int(10),
            // Ball is on the ground (pos_z = 0); vel_z starts at zero.
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
    /// contact. Pre-fix, vz would land at exactly zero after the bounce
    /// branch, satisfy the `vz <= Q32::ZERO` rolling-friction guard, and
    /// double-decay horizontal velocity. The `did_bounce` flag prevents
    /// that. We assert horizontal velocity is only decayed by drag
    /// (× 0.98), not drag-then-friction (× 0.98 × 0.75).
    /// T1-3.5: altitude axis is pos_z/vel_z.
    #[test]
    fn zero_bounce_retention_does_not_apply_rolling_friction_on_contact() {
        let coeffs = BallPhysicsCoefficients {
            bounce_retention: Q32::ZERO,
            ..phase1_seeds()
        };
        let before = BallState {
            pos_z: Q32::ONE / Q32::from_int(10), // 0.1m up (altitude)
            vel_x: Q32::from_int(10),
            vel_z: -Q32::from_int(10), // diving downward — bounce tick
            ..BallState::centre_spot()
        };
        let after = ball_step(&before, &coeffs);
        // Post-step vel_z is zero (bounce flipped × 0 = 0).
        assert_eq!(
            after.vel_z,
            Q32::ZERO,
            "vel_z should be zero after dead bounce"
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

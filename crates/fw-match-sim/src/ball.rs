//! Ball state — canonical 9-component vector (position + velocity + spin)
//! consumed by the ball-physics integrator (`crate::ball_physics`).
//!
//! T1-2b-i (2026-05-13) extended `BallState` from the T0 6-field
//! placeholder (position + velocity) to include `spin_{x,y,z}: Q32`.
//! Magnus-coupled bounce in `ball_step` reads spin × velocity; without
//! spin in canonical state, Magnus would be a future schema bump every
//! time it's actually wired up to the BT runner (T1-2b-iii). Per ADR-0012
//! trigger #1 (canonical schema bump) the pinned BLAKE3 hash re-baselines
//! in this same commit; the rebaseline marker lives in the T1-2b-i commit
//! body.
//!
//! ## Coordinate convention (Codex 2026-05-16 audit P0; corrected at T1-3.5)
//!
//! **X + Y form the pitch plane; Z is altitude.**
//! - `pos_x` / `vel_x`: pitch-length (attacking) axis. Home attacks toward +X.
//!   `GOAL_LINE_X = ±52.5m` (pitch length 105m).
//! - `pos_y` / `vel_y`: lateral (touchline) axis. `SIDELINE_Y = ±34.0m`
//!   (pitch width 68m).
//! - `pos_z` / `vel_z`: altitude. Ground = `pos_z = 0`; gravity acts on `-vel_z`.
//!   The integrator clamps `pos_z` to `0` on contact (z-axis ground bounce only).
//!
//! **Prior (incorrect) convention**: pre-T1-3.5 this doc-comment claimed
//! X + Z = pitch plane / Y = altitude. That contradicted the pitch-geometry
//! convention used everywhere else in `fw-match-sim` (SIDELINE_Y checks
//! `|pos_y|`; goal-line checks `|pos_x|`). T1-3.5 corrected `ball_physics.rs`
//! to gravity-on-(-Z) + rolling-friction-on-XY pitch-plane and rewrote
//! this doc-comment to match. The single source of truth for the convention
//! now lives here + in `ball_physics.rs`'s module doc-comment.
//!
//! Spin is angular velocity (rad/s); Magnus = `coeff * (spin × v)`.
//!
//! ## Schema history
//!
//! T1-2b-i (2026-05-13) extended `BallState` from the T0 6-field
//! placeholder (position + velocity) to include `spin_{x,y,z}: Q32`.
//! Magnus-coupled bounce in `ball_step` reads spin × velocity; without
//! spin in canonical state, Magnus would be a future schema bump every
//! time it's actually wired up to the BT runner (T1-2b-iii). Per ADR-0012
//! trigger #1 (canonical schema bump) the pinned BLAKE3 hash re-baselines
//! in this same commit; the rebaseline marker lives in the T1-2b-i commit
//! body.

use fw_core::Q32;
use serde::{Deserialize, Serialize};

/// Ball world-space state. Canonical sim state — every field
/// participates in the BLAKE3 canonical-state hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BallState {
    /// Position. (0, 0, 0) = centre spot on the ground.
    pub pos_x: Q32,
    pub pos_y: Q32,
    pub pos_z: Q32,

    /// Velocity in m/s.
    pub vel_x: Q32,
    pub vel_y: Q32,
    pub vel_z: Q32,

    /// Spin (angular velocity) in rad/s. Read by the Magnus term in the
    /// ball-physics integrator. Zero in phase1 seeds because
    /// `magnus_coupling = 0` for T1; spin is reserved for T1-2b-iii's
    /// curved passes + finishing.
    pub spin_x: Q32,
    pub spin_y: Q32,
    pub spin_z: Q32,
}

impl BallState {
    /// Centre-spot kick-off state — at the origin, zero velocity, zero
    /// spin. The deterministic initial state for the T0 corpus fixture.
    pub fn centre_spot() -> BallState {
        BallState {
            pos_x: Q32::ZERO,
            pos_y: Q32::ZERO,
            pos_z: Q32::ZERO,
            vel_x: Q32::ZERO,
            vel_y: Q32::ZERO,
            vel_z: Q32::ZERO,
            spin_x: Q32::ZERO,
            spin_y: Q32::ZERO,
            spin_z: Q32::ZERO,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// T1-2b-i Chunk 1 RED: centre_spot must populate spin fields too.
    /// Magnus-coupled bounces need spin in canonical state from day one;
    /// the `phase1_seeds` magnus_coupling = 0 stub keeps Magnus a no-op
    /// behaviorally while preserving the schema.
    #[test]
    fn centre_spot_has_zero_spin() {
        let b = BallState::centre_spot();
        assert_eq!(b.spin_x, Q32::ZERO);
        assert_eq!(b.spin_y, Q32::ZERO);
        assert_eq!(b.spin_z, Q32::ZERO);
    }
}

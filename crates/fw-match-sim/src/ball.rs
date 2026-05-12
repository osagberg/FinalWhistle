//! Ball state — placeholder. Full deterministic physics ports in T1.
//!
//! The pre-pivot FW C# sim implemented bespoke Q32 ball physics (drag,
//! Magnus, bounce) per the "no Unity PhysX in canonical state" rule. That
//! port lands in T1 (`docs/MASTER_PLAN.md` Phase-1 sim core). T0 carries
//! the type forward as a structural placeholder so `MatchState`'s shape
//! is stable from the very first commit.

use fw_core::Q32;
use serde::{Deserialize, Serialize};

/// Ball world-space state.
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
}

impl BallState {
    /// Centre-spot kick-off state — at the origin, zero velocity.
    pub fn centre_spot() -> BallState {
        BallState {
            pos_x: Q32::ZERO,
            pos_y: Q32::ZERO,
            pos_z: Q32::ZERO,
            vel_x: Q32::ZERO,
            vel_y: Q32::ZERO,
            vel_z: Q32::ZERO,
        }
    }
}

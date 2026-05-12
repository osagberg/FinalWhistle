//! Per-player canonical state.
//!
//! Phase-0 scope: position + velocity in Q32, plus the slot index that pins
//! the player to a canonical-encoding position. Behavior-tree state,
//! signature-readiness, fatigue, etc. all land in later phases. The struct
//! is intentionally small so the Phase-0 canonical-hash baseline is easy to
//! reason about; subsequent phases extend it with `#[serde(default)]`
//! fields where backward compatibility is needed.

use fw_core::Q32;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::PlayerSlot;

/// Per-player canonical state. Slot-indexed inside `MatchState::players`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerState {
    /// Slot index 0..22. Pinned for the duration of the match; copied here
    /// for self-describing encoding (canonical-encoder reads it back out
    /// rather than threading the outer index through).
    pub slot: PlayerSlot,

    /// World-space position in metres. (0, 0) is the centre spot.
    /// `pos_x` runs goal-to-goal; `pos_y` runs touchline-to-touchline.
    pub pos_x: Q32,
    pub pos_y: Q32,

    /// World-space velocity in m/s. Zero in T0 (no movement); will be
    /// driven by the BT runner in T1+.
    pub vel_x: Q32,
    pub vel_y: Q32,

    /// Stamina / fatigue / readiness / other scalar per-player state. Stays
    /// a `BTreeMap<u16, Q32>` so individual keys can be added without
    /// changing the struct shape — the canonical encoder iterates in sorted
    /// key order so the encoding is stable regardless of insertion order.
    ///
    /// Key allocation is fixed at the content-pack level; see
    /// `fw-content::PlayerScalarKey` (T1+).
    #[serde(default)]
    pub scalars: BTreeMap<u16, Q32>,
}

impl PlayerState {
    /// Construct a player at `(x, y)` with zero velocity and no scalars.
    /// Phase-0 placeholder used by `MatchState::initial`.
    pub fn at(slot: PlayerSlot, x: Q32, y: Q32) -> PlayerState {
        PlayerState {
            slot,
            pos_x: x,
            pos_y: y,
            vel_x: Q32::ZERO,
            vel_y: Q32::ZERO,
            scalars: BTreeMap::new(),
        }
    }
}

//! Behavior archetypes — the hand-authored library of player BTs the sim
//! draws from.
//!
//! T0 stub. The full archetype catalog + BT serialization format land in
//! T1 alongside the runner port.

use serde::{Deserialize, Serialize};

/// A behavior-archetype handle. Player templates reference archetypes by
/// qualified ID; the runner resolves them at match-init time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BehaviorArchetype {
    /// Content-pack-qualified textual ID (`fwh.core:archetype_box-to-box-8`).
    pub qualified_id: String,

    /// Display name for debug surfaces / scout reports.
    pub display_name: String,
}

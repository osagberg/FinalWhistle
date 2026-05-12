//! Player templates + gene vector.
//!
//! T0 stub. The gene model lands in T3 (scouting) and T4 (development);
//! this file pins the type surface so dependent crates can compile.

use fw_core::{PlayerId, Q32};
use serde::{Deserialize, Serialize};

/// A player-template entry from a content pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerTemplate {
    /// Stable handle. Allocated by the content compiler at pack-bake time.
    pub id: PlayerId,

    /// Content-pack-qualified textual ID (`fwh.core:player_00042`).
    pub qualified_id: String,

    /// Display name. T2 will localize.
    pub display_name: String,

    /// The bounded gene vector — typically clamped to 0.0..1.0 in Q32 form,
    /// with rare narrative-breakthrough exceptions per DESIGN_DOC §3
    /// (Pillar 3). Indexing convention lives in `design/progression.md`
    /// (owed in Phase-4); T3 introduces a typed accessor.
    #[serde(default)]
    pub genes: GeneVector,
}

/// Bounded gene vector. Length is fixed at compile time once T3 ships the
/// schema; T0 carries a `Vec` so the type compiles without prejudging the
/// final shape.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GeneVector(pub Vec<Q32>);

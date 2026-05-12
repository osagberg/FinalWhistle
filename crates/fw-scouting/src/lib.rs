//! `fw-scouting` — scouting model.
//!
//! Phase-0 scope: empty type surface that compiles. T3 introduces the
//! disagreeing-biased-scouts model per DESIGN_DOC §3 (Pillar 4) and
//! `design/scouting.md` (owed Phase-3).

use fw_core::{PlayerId, Q32};
use serde::{Deserialize, Serialize};

/// A single scout's noisy observation of a player gene.
///
/// T0 placeholder shape. T3 expands with bias terms, recency, and the
/// per-scout per-region noise model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoutReport {
    /// The player observed.
    pub player_id: PlayerId,

    /// The gene index reported on. Real schema in T3.
    pub gene_index: u16,

    /// The scout's estimate of that gene's value, in Q32.
    pub estimate: Q32,
}

// -------------------------------------------------------------------------
// Smoke
// -------------------------------------------------------------------------

#[cfg(test)]
mod smoke {
    #[test]
    fn smoke() {
        assert_eq!(2 + 2, 4);
    }
}

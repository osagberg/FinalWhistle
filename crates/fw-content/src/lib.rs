//! `fw-content` — content schema + RON loader stubs.
//!
//! Phase-0 scope: type vocabulary only. The RON loader, content-pack
//! validation (FW-VAL gauntlet), and ID-resolution table land in T2 per
//! `docs/MASTER_PLAN.md`.
//!
//! Content packs ship as RON files keyed by stable content-pack-qualified
//! IDs (`fwh.core:player_00042`). Schema-versioned; load-time forward
//! migration only — no in-place mutation of disk content.

pub mod archetype;
pub mod player;
pub mod team;
pub mod runtime;

pub use archetype::BehaviorArchetype;
pub use player::{GeneVector, PlayerTemplate};
pub use team::TeamTemplate;
pub use runtime::{
    ContentKind, ContentLoadError, ContentStore, Culture, CultureWeights,
    FormationSlot, TacticalArchetype, derive_seed,
};

use thiserror::Error;

/// Errors the content loader can raise. Surfaced via `fw-tauri` to the
/// frontend as structured user-facing strings.
#[derive(Debug, Error)]
pub enum ContentError {
    #[error("RON parse failure in {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: ron::error::SpannedError,
    },

    #[error("schema version {found} not supported; expected ≤ {max}")]
    UnsupportedSchema { found: u32, max: u32 },

    #[error("content-pack-qualified ID {0:?} malformed; expected `<pack>:<kind>_<index>`")]
    MalformedId(String),
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

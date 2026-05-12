//! `fw-replay` — replay corpus + canonical-hash regression gate.
//!
//! The Phase-0 acceptance test (`tests/canonical_hash.rs`) is the
//! load-bearing entry point. This library module is the corpus loader:
//! the supporting machinery the test and the `scripts/fw replay` CLI both
//! consume.
//!
//! See `docs/specs/determinism-gate.md` for the full contract.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One entry in the corpus. Each fixture file deserializes into this shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReplayCorpusEntry {
    /// Schema version of the fixture file itself. Currently `1`.
    pub schema_version: u32,

    /// Seed as a hex string (e.g. `"0xdeadbeefdeadbeef"`). The filename
    /// convention mirrors this so a glob over `fixtures/` is sufficient
    /// to enumerate the corpus.
    pub seed: String,

    /// Number of ticks the sim is advanced before hashing.
    pub tick_count: u32,

    /// `blake3:<64-hex>` digest of `encode_canonical()` after `tick_count`
    /// ticks. All-zeros means "placeholder — fill on first CI green pass."
    pub expected_hash: String,

    /// Free-form metadata. Not load-bearing; useful for the human reader.
    #[serde(default)]
    pub metadata: ReplayMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReplayMetadata {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub generated_at: String,
    #[serde(default)]
    pub notes: String,
}

/// Errors the corpus loader can raise.
#[derive(Debug, Error)]
pub enum CorpusError {
    #[error("corpus fixture not found at {0}")]
    NotFound(PathBuf),

    #[error("I/O error reading corpus fixture: {0}")]
    Io(#[from] std::io::Error),

    #[error("RON parse failure: {0}")]
    Parse(#[from] ron::error::SpannedError),
}

/// Load and parse a single corpus entry. The path is the RON file directly.
pub fn load_entry(path: impl AsRef<Path>) -> Result<ReplayCorpusEntry, CorpusError> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(CorpusError::NotFound(path.to_path_buf()));
    }
    let raw = std::fs::read_to_string(path)?;
    let entry: ReplayCorpusEntry = ron::from_str(&raw)?;
    Ok(entry)
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

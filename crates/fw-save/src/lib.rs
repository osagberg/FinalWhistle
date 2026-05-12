//! `fw-save` — save-file format + version migration.
//!
//! Phase-0 scope: the `SaveV1` enum stub. The four-tests-per-bump migration
//! discipline (forward-migration + callback-preservation +
//! forward-incompat-failure + round-trip-byte-identical) lands in T5 per
//! `design/specs/save-migration-fixtures.md` (owed Phase-5).
//!
//! ## Format
//!
//! Wire format is bincode 2. The outer envelope is the schema-versioned
//! enum; new variants append a tag rather than shifting an existing one,
//! so old saves remain parseable.
//!
//! Saves are NOT canonical-state-equivalent — they hold the career-level
//! state (career seed + ledger + content-pack version), not per-tick state.
//! A loaded save replays its match history from the seed to reproduce
//! canonical state on demand.

use fw_core::Seed;
use fw_memory::MemoryLedger;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The save-file envelope. Versioned via the enum tag so old saves remain
/// parseable across schema bumps.
///
/// Migration discipline (T5+): a new variant `SaveV2` is added; the loader
/// matches both, with `SaveV1 -> SaveV2` forward-migration logic in a
/// separate `migrate.rs` module. Older variants are NEVER deleted —
/// removing a variant breaks "load this 6-month-old save."
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaveEnvelope {
    /// Schema v1 — the Phase-0 / T0 stub.
    V1(SaveV1),
}

/// Schema v1 payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveV1 {
    /// The career's deterministic seed.
    pub career_seed: Seed,

    /// Content-pack version this save was authored against. Mismatch is a
    /// loader concern (T5 spec).
    pub content_pack_version: u32,

    /// The career ledger. Replays produce the rest of the world state on
    /// demand from this + the seed.
    pub ledger: MemoryLedger,
}

/// Errors the save loader can raise.
#[derive(Debug, Error)]
pub enum SaveError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("bincode encode failure: {0}")]
    Encode(#[from] bincode::error::EncodeError),

    #[error("bincode decode failure: {0}")]
    Decode(#[from] bincode::error::DecodeError),
}

/// Encode a save envelope to bincode bytes.
pub fn encode(envelope: &SaveEnvelope) -> Result<Vec<u8>, SaveError> {
    let cfg = bincode::config::standard();
    Ok(bincode::serde::encode_to_vec(envelope, cfg)?)
}

/// Decode a save envelope from bincode bytes.
pub fn decode(bytes: &[u8]) -> Result<SaveEnvelope, SaveError> {
    let cfg = bincode::config::standard();
    let (envelope, _consumed) = bincode::serde::decode_from_slice(bytes, cfg)?;
    Ok(envelope)
}

// -------------------------------------------------------------------------
// Smoke
// -------------------------------------------------------------------------

#[cfg(test)]
mod smoke {
    use super::*;

    #[test]
    fn smoke() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn encode_decode_round_trip() {
        let env = SaveEnvelope::V1(SaveV1 {
            career_seed: Seed::from_u64(0xCAFEBABE),
            content_pack_version: 1,
            ledger: MemoryLedger::new(),
        });
        let bytes = encode(&env).expect("encode");
        let restored = decode(&bytes).expect("decode");
        assert_eq!(env, restored);
    }
}

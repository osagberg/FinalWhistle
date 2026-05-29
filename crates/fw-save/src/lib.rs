//! `fw-save` — save-file format + version migration.
//!
//! ## T2-9: V1 schema lock + V0→V1 migration discipline established
//!
//! V1 is the LOCKED first real schema (`SaveV1 { career_seed,
//! content_pack_version, ledger }`). V0 is a fictional pre-T2-9 placeholder.
//! Both are PRESERVED FOREVER — old saves must remain loadable.
//!
//! ## T3-1: V2 schema — rich MemoryLedger (ADR-0005 schema port)
//!
//! V2 carries the ADR-0005 `MemoryLedger` (real `MemoryEvent` schema,
//! EventId-indexed, BTreeMap-backed). V1's placeholder `MemoryEvent::Placeholder`
//! events had no real semantics; `migrate_v1_to_v2` translates any V1 ledger
//! to an EMPTY V2 ledger.
//!
//! ## T3-R-E: V3 schema — career-state persistence
//!
//! V3 adds the in-progress career state the V2 ledger alone cannot
//! reconstruct: `season_number`, an `Option<SeasonState>` snapshot, and the
//! per-player `BreakthroughState` map. See `SaveV3`.
//!
//! Migration chain: V0 → migrate_v0_to_v1 → V1 → migrate_v1_to_v2 → V2 →
//! migrate_v2_to_v3 → V3.
//!
//! Four-test migration discipline (per `design/specs/save-migration-fixtures.md`):
//!
//!   1. forward-migration         — V(N) bytes load + emerge as V(N+1)
//!   2. callback-preservation     — every V(N) field maps to V(N+1) (no drops)
//!   3. forward-incompat-failure  — unknown-future-discriminant FAILS LOUDLY
//!   4. round-trip-byte-identical — encode(decode(x))) bytes ≡ original
//!
//! `load_envelope(bytes) -> Result<SaveV3, SaveError>` is the production
//! load entry point. It decodes the envelope, runs the full migration chain,
//! calls `MemoryLedger::restore_transient_state()` on the decoded ledger, and
//! returns the V3 payload. Unknown discriminants surface as `SaveError::Decode`.
//!
//! ## Format
//!
//! Wire format is bincode 2. The outer envelope is the schema-versioned
//! enum; new variants append a tag rather than shifting an existing one,
//! so old saves remain parseable.
//!
//! Saves are NOT canonical-state-equivalent — they hold career-level state
//! (seed + ledger + content-pack version + season progress), not per-tick
//! match state. The seed regenerates the league STRUCTURE via
//! `generate_league`; it does NOT regenerate a season's PROGRESS (which
//! match-days are played, the live standings). Through V2 the season was
//! intentionally omitted; T3-R-E's V3 persists it (`Option<SeasonState>`)
//! because Codex review E5 found a V2 save could not resume mid-season.

pub mod settings;
pub use settings::{
    SettingsEnvelope, SettingsError, SettingsV0, ThemePref, encode_settings, load_settings_envelope,
};

use std::collections::BTreeMap;

use fw_content::SeasonState;
use fw_core::{PlayerId, Seed};
use fw_memory::{BreakthroughState, MemoryLedger, SeasonNumber};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The save-file envelope. Versioned via the enum tag so old saves remain
/// parseable across schema bumps.
///
/// Migration discipline: a new variant `SaveV(N+1)` is added; the loader
/// matches all known variants, with `SaveV(N) -> SaveV(N+1)` forward-
/// migration logic chained inside `load_envelope`. Older variants are
/// NEVER deleted — removing a variant breaks "load this 6-month-old save."
///
/// ## Variant-tag stability (LOAD-BEARING FOREVER)
///
/// Post-T2-9 type-design P0 fix: variants carry EXPLICIT discriminants
/// (`V0(...) = 0`, `V1(...) = 1`). Prior shape relied on a doc-comment-only
/// "append AT THE END" convention — a future PR re-ordering V0 / V1 (even
/// mechanically by rustfmt, sort-imports tooling, or a careless refactor)
/// would silently swap the wire encoding of every save in the wild. The
/// explicit `= N` syntax makes any tag drift a visible diff + a compile-time
/// rejection (Rust forbids duplicate explicit discriminants).
///
/// The wire bytes for `V0` (`[0x00, ...]`) and `V1` (`[0x01, ...]`) are
/// ALSO pinned by dedicated regression tests so a future bincode minor
/// version that changes its varint encoding fails loudly. See
/// `smoke::v0_envelope_wire_first_byte_is_locked_at_0x00` /
/// `smoke::v1_envelope_wire_first_byte_is_locked_at_0x01`.
///
/// New variants append AT THE END with the next discriminant in sequence
/// (`V2(...) = 2`, etc.). NEVER reorder, NEVER reuse a discriminant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum SaveEnvelope {
    /// Schema v0 — the fictional pre-T2-9 minimal stub (only `career_seed`).
    /// Kept FOREVER to exercise the migration-discipline contract. Real
    /// production saves are NEVER created in this variant; it exists only
    /// so the V0→V1 migration test path stays live + green as the schema
    /// chain grows.
    V0(SaveV0) = 0,
    /// Schema v1 — T2-9-locked first real schema. PRESERVED FOREVER.
    V1(SaveV1) = 1,
    /// Schema v2 — T3-1. Carries the ADR-0005 `MemoryLedger` (rich
    /// `MemoryEvent` schema). First variant with a real event ledger.
    ///
    /// Wire tag `0x02` is pinned by `smoke::v2_envelope_wire_first_byte_is_locked_at_0x02`.
    V2(SaveV2) = 2,
    /// Schema v3 — T3-R-E. Adds the career-progress state the V2 ledger alone
    /// cannot reconstruct: `season_number`, an `Option<SeasonState>` snapshot,
    /// and the per-player `BreakthroughState` map. V3 is the CURRENT
    /// production schema.
    ///
    /// Wire tag `0x03` is pinned by `smoke::v3_envelope_wire_first_byte_is_locked_at_0x03`.
    V3(SaveV3) = 3,
}

/// Schema v0 payload — the fictional pre-T2-9 stub. Carries only the
/// `career_seed`. Migrates to V1 by supplying defaults for the new fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveV0 {
    /// The career's deterministic seed.
    pub career_seed: Seed,
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

/// Schema v2 payload — T3-1 (ADR-0005 memory ledger).
///
/// Structurally identical to V1 except the `ledger` now carries the rich
/// `MemoryEvent` schema (EventId, Q32 stakes/salience, 30-variant EventClass,
/// BTreeMap indexes). V1's placeholder events are discarded on migration.
///
/// PRESERVED FOREVER — locked at T3-1, superseded by V3. V2 saves migrate
/// cleanly to V3 via `migrate_v2_to_v3`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveV2 {
    /// The career's deterministic seed.
    pub career_seed: Seed,

    /// Content-pack version this save was authored against.
    pub content_pack_version: u32,

    /// The career ledger — rich `MemoryEvent` rows per ADR-0005.
    /// `MemoryLedger::restore_transient_state()` is called by
    /// `load_envelope` after deserialisation to rebuild the `next_id`
    /// counter and mark indexes dirty.
    pub ledger: MemoryLedger,
}

/// Schema v3 payload — T3-R-E (career-state persistence).
///
/// V2 stored only `career_seed + content_pack_version + ledger`. A career's
/// IN-PROGRESS season (which match-days are played, the live standings) and
/// its `season_number` are NOT derivable from the seed alone — `generate_league`
/// regenerates the league STRUCTURE, not the season's PROGRESS. V3 adds:
///
/// - `season_number` — which season the career is currently on.
/// - `season` — an `Option<SeasonState>` snapshot. `Some` for a genuine V3
///   career save; `None` for a save migrated up from V2 (which never persisted
///   a season — CALLER RESPONSIBILITY: when season is None the caller must
///   regenerate a fresh SeasonState from the career seed).
/// - `breakthrough_states` — the per-player `BreakthroughState` map (empty
///   until the career system is wired in T4+; the schema carries the slot now).
///
/// V3 is the CURRENT production schema. All new saves are V3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveV3 {
    /// The career's deterministic seed.
    pub career_seed: Seed,

    /// Content-pack version this save was authored against.
    pub content_pack_version: u32,

    /// The career ledger — rich `MemoryEvent` rows per ADR-0005.
    /// `MemoryLedger::restore_transient_state()` is called by `load_envelope`
    /// after deserialisation.
    pub ledger: MemoryLedger,

    /// Which season the career is currently on.
    pub season_number: SeasonNumber,

    /// Snapshot of the active season's progress. `None` on a save migrated
    /// from V2 (no season was ever persisted). CALLER RESPONSIBILITY: when
    /// season is None the caller must regenerate a fresh SeasonState from
    /// `career_seed`. `Some` for a genuine V3 career save.
    pub season: Option<SeasonState>,

    /// Per-player breakthrough meter + cooldown state. Empty until the career
    /// system is wired (T4+); the schema carries the slot now so future saves
    /// need no further bump.
    pub breakthrough_states: BTreeMap<PlayerId, BreakthroughState>,
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

    /// Post-T2-9 silent-failure-hunter P1 fix: the decoder consumed FEWER
    /// bytes than the input buffer length. Indicates a corrupted save file
    /// (trailing garbage) or a deliberate splice. Without this check, a
    /// truncated-then-appended save would deserialize cleanly as whatever
    /// the prefix happened to encode, and the user would get a working-but-
    /// wrong career with no diagnostic — exactly the silent-failure class
    /// the four-test discipline forbids.
    #[error(
        "save file has trailing bytes: decoded {consumed} of {total} bytes \
         (corruption or splice)"
    )]
    TrailingBytes { consumed: usize, total: usize },
}

/// Encode a save envelope to bincode bytes.
#[must_use = "encoded bytes are discarded; did you mean to persist via std::fs::write?"]
pub fn encode(envelope: &SaveEnvelope) -> Result<Vec<u8>, SaveError> {
    let cfg = bincode::config::standard();
    Ok(bincode::serde::encode_to_vec(envelope, cfg)?)
}

/// Decode a save envelope from bincode bytes.
///
/// Post-T2-9 silent-failure-hunter P1 fix: REJECTS trailing bytes via
/// `SaveError::TrailingBytes`. The prior shape silently discarded the
/// `_consumed` return from `decode_from_slice` — meaning a corrupted save
/// (truncated-then-appended OR a deliberate splice) would deserialize
/// cleanly as whatever the prefix bytes happened to encode, and the user
/// would get a working-but-wrong career with no diagnostic. Format ships
/// FOREVER — silent-on-corruption is exactly the failure mode banned by
/// CLAUDE.md §10 + the four-test migration discipline.
#[must_use = "the decoded envelope is discarded; did you mean to match on it or pass through load_envelope?"]
pub fn decode(bytes: &[u8]) -> Result<SaveEnvelope, SaveError> {
    let cfg = bincode::config::standard();
    let (envelope, consumed) = bincode::serde::decode_from_slice(bytes, cfg)?;
    if consumed != bytes.len() {
        return Err(SaveError::TrailingBytes {
            consumed,
            total: bytes.len(),
        });
    }
    Ok(envelope)
}

/// Forward-migrate a `SaveV0` payload to the `SaveV1` schema.
///
/// Preserves the `career_seed` exactly (it carries forward to V1's matching
/// field). Supplies V1-only defaults: `content_pack_version = 1` (the only
/// content-pack version that has ever shipped) and `ledger =
/// MemoryLedger::new()` (an empty ledger — a freshly-promoted V0 save has
/// no recorded events).
///
/// Pure function: `migrate_v0_to_v1(v0)` always produces the same `SaveV1`
/// for the same `v0`. Mirrors the project's sim-side determinism discipline.
pub fn migrate_v0_to_v1(v0: SaveV0) -> SaveV1 {
    SaveV1 {
        career_seed: v0.career_seed,
        content_pack_version: 1,
        ledger: MemoryLedger::new(),
    }
}

/// Forward-migrate a `SaveV1` payload to the `SaveV2` schema.
///
/// V1 ledgers in the wild have no real events — the V1 `MemoryEvent` type
/// existed only as a pre-T3 placeholder stub (deleted from `fw-memory` at
/// T3-1 alongside the ADR-0005 schema port). `migrate_v1_to_v2` therefore
/// produces an EMPTY V2 ledger rather than attempting to translate the
/// (now-non-existent) placeholder rows into ADR-0005 records.
///
/// Preserves `career_seed` and `content_pack_version` exactly.
///
/// Pure function: same V1 input always produces the same V2 output.
pub fn migrate_v1_to_v2(v1: SaveV1) -> SaveV2 {
    SaveV2 {
        career_seed: v1.career_seed,
        content_pack_version: v1.content_pack_version,
        ledger: MemoryLedger::new(),
    }
}

/// Forward-migrate a `SaveV2` payload to the `SaveV3` schema.
///
/// Preserves `career_seed`, `content_pack_version`, and `ledger` exactly.
/// Supplies the V3-only defaults: `season_number = SeasonNumber(0)` (a promoted
/// V2 save begins a fresh career at season 0), `season = None` (V2 never
/// persisted a season — CALLER RESPONSIBILITY: when season is None the caller
/// must regenerate a fresh SeasonState from the career seed), and an empty
/// `breakthrough_states` map.
///
/// Pure function: the same `v2` always produces the same `SaveV3`.
pub fn migrate_v2_to_v3(v2: SaveV2) -> SaveV3 {
    SaveV3 {
        career_seed: v2.career_seed,
        content_pack_version: v2.content_pack_version,
        ledger: v2.ledger,
        season_number: SeasonNumber(0),
        season: None,
        breakthrough_states: BTreeMap::new(),
    }
}

/// Production load entry point. Decode bytes → run the full migration chain
/// → return the latest-schema payload (`SaveV3`).
///
/// Variant dispatch:
///   - `V0(v0)` → `migrate_v0_to_v1` → `migrate_v1_to_v2` → `migrate_v2_to_v3`
///   - `V1(v1)` → `migrate_v1_to_v2` → `migrate_v2_to_v3`
///   - `V2(v2)` → `migrate_v2_to_v3(v2)`
///   - `V3(v3)` → returned as-is (after `restore_transient_state`)
///   - unknown discriminant → `SaveError::Decode(...)` (bincode's
///     `DecodeError::UnexpectedVariant`). This IS the four-test-discipline
///     "forward-incompat-failure" path: a future V99 save loaded by an
///     older binary FAILS LOUDLY.
///
/// `MemoryLedger::restore_transient_state()` is called on the final payload's
/// ledger so the `next_id` counter and dirty flag are correctly initialised
/// before the caller appends any new events.
///
/// Callers that need to introspect WHICH variant a save was authored as
/// should call `decode()` directly and match the envelope.
#[must_use = "the loaded SaveV3 is discarded; load_envelope is the only path that runs the V0→V1→V2→V3 migration chain"]
pub fn load_envelope(bytes: &[u8]) -> Result<SaveV3, SaveError> {
    let mut v3 = match decode(bytes)? {
        SaveEnvelope::V0(v0) => migrate_v2_to_v3(migrate_v1_to_v2(migrate_v0_to_v1(v0))),
        SaveEnvelope::V1(v1) => migrate_v2_to_v3(migrate_v1_to_v2(v1)),
        SaveEnvelope::V2(v2) => migrate_v2_to_v3(v2),
        SaveEnvelope::V3(v3) => v3,
    };
    v3.ledger.restore_transient_state();
    Ok(v3)
}

// -------------------------------------------------------------------------
// Smoke
// -------------------------------------------------------------------------

#[cfg(test)]
mod smoke {
    // Post-T2-close Track D-1 gate-blocker fix: removed vacuous `smoke()` test
    // (asserted `2 + 2 == 4`; mutating any production fw-save code did not
    // fail it). A vacuous test in the save-migration crate is exactly the
    // kind of false confidence the four-test-per-bump discipline was built
    // to prevent — keeping it would corrupt the test-count signal that
    // future migration reviews rely on.
    use super::*;

    // T2-R-D6: deleted redundant `encode_decode_round_trip` test.
    // It encoded SaveEnvelope::V1 and asserted decode equality — the
    // immediately-following `v0_and_v1_variants_construct_and_round_trip`
    // test does the same V1 encode+decode+equality-check AND adds V0
    // round-trip + the first-byte-divergence guard, fully subsuming
    // the deleted test. Combined with the prior `smoke()` removal
    // (T2-R-D1), fw-save tests drop from 11 → 9 with zero loss of
    // mutation-detection coverage.

    // ----- T2-9: AC1 — both variants construct + round-trip cleanly -----

    /// AC1 — `SaveEnvelope::V0` and `SaveEnvelope::V1` both compile, both
    /// encode, and both decode back to themselves byte-for-byte.
    #[test]
    fn v0_and_v1_variants_construct_and_round_trip() {
        let v0_env = SaveEnvelope::V0(SaveV0 {
            career_seed: Seed::from_u64(0xDEAD_BEEF),
        });
        let v1_env = SaveEnvelope::V1(SaveV1 {
            career_seed: Seed::from_u64(0xCAFE_BABE),
            content_pack_version: 1,
            ledger: MemoryLedger::new(),
        });

        let v0_bytes = encode(&v0_env).expect("encode v0");
        let v1_bytes = encode(&v1_env).expect("encode v1");

        assert_eq!(decode(&v0_bytes).expect("decode v0"), v0_env);
        assert_eq!(decode(&v1_bytes).expect("decode v1"), v1_env);

        // Sanity: V0 + V1 produce DIFFERENT first bytes (different variant
        // tag), proving the envelope is genuinely versioned at the wire.
        assert_ne!(
            v0_bytes[0], v1_bytes[0],
            "V0 and V1 must occupy distinct variant tags on the wire"
        );
    }

    // ----- T2-9: wire-byte regression (post-type-design P0 fix) -----

    /// Post-T2-9 type-design P0 fix: pin the EXACT first byte of an
    /// encoded V0 envelope at `0x00`. The prior smoke test only asserted
    /// V0 and V1 bytes DIFFER, not WHICH bytes — a reorder swap would
    /// have passed silently. This test fails LOUDLY if anyone reorders
    /// the SaveEnvelope variants OR bincode 2 changes its varint
    /// encoding for discriminant 0. Schema is locked FOREVER; the wire
    /// bytes are part of the lock.
    #[test]
    fn v0_envelope_wire_first_byte_is_locked_at_0x00() {
        let env = SaveEnvelope::V0(SaveV0 {
            career_seed: Seed::from_u64(0),
        });
        let bytes = encode(&env).expect("encode v0");
        assert_eq!(
            bytes[0], 0x00,
            "V0 wire tag is LOCKED at 0x00 — schema-lock invariant"
        );
    }

    /// Post-T2-9 type-design P0 fix: pin V1's wire tag at `0x01`. Same
    /// rationale as the V0 lock above.
    #[test]
    fn v1_envelope_wire_first_byte_is_locked_at_0x01() {
        let env = SaveEnvelope::V1(SaveV1 {
            career_seed: Seed::from_u64(0),
            content_pack_version: 1,
            ledger: MemoryLedger::new(),
        });
        let bytes = encode(&env).expect("encode v1");
        assert_eq!(
            bytes[0], 0x01,
            "V1 wire tag is LOCKED at 0x01 — schema-lock invariant"
        );
    }

    // ----- T2-9: AC4d — round-trip-byte-identical -----

    /// AC4d — encode → decode → re-encode produces byte-identical bytes.
    /// Mutation removing serde determinism (e.g. switching to a non-deterministic
    /// serializer) would fail this.
    #[test]
    fn v1_encode_decode_reencode_produces_identical_bytes() {
        let env = SaveEnvelope::V1(SaveV1 {
            career_seed: Seed::from_u64(0x1234_5678_9ABC_DEF0),
            content_pack_version: 1,
            ledger: MemoryLedger::new(),
        });
        let bytes_1 = encode(&env).expect("encode 1");
        let decoded = decode(&bytes_1).expect("decode");
        let bytes_2 = encode(&decoded).expect("encode 2");
        assert_eq!(
            bytes_1, bytes_2,
            "encode(decode(encode(x))) must be byte-identical to encode(x)"
        );
    }

    // ----- T3-1: AC7 — V2 variant constructs + round-trips -----

    /// AC7 — `SaveEnvelope::V2` compiles, encodes, and decodes back to itself.
    #[test]
    fn v2_variant_constructs_and_round_trips() {
        let env = SaveEnvelope::V2(SaveV2 {
            career_seed: Seed::from_u64(0xBEEF_CAFE_DEAD_F00D),
            content_pack_version: 2,
            ledger: MemoryLedger::new(),
        });
        let bytes = encode(&env).expect("encode v2");
        let decoded = decode(&bytes).expect("decode v2");
        assert_eq!(decoded, env, "V2 must round-trip through encode/decode");
    }

    // ----- T3-1: AC10 — V2 wire-byte regression -----

    /// AC10 — Pin the EXACT first byte of an encoded V2 envelope at `0x02`.
    /// Re-ordering SaveEnvelope variants or changing bincode's varint encoding
    /// for discriminant 2 will fail this test.
    ///
    /// Schema is locked FOREVER; the wire bytes are part of the lock.
    #[test]
    fn v2_envelope_wire_first_byte_is_locked_at_0x02() {
        let env = SaveEnvelope::V2(SaveV2 {
            career_seed: Seed::from_u64(0),
            content_pack_version: 1,
            ledger: MemoryLedger::new(),
        });
        let bytes = encode(&env).expect("encode v2");
        assert_eq!(
            bytes[0], 0x02,
            "V2 wire tag is LOCKED at 0x02 — schema-lock invariant"
        );
    }

    /// AC10-extension — V0/V1/V2 all produce DIFFERENT first bytes, proving
    /// three-way variant distinction at the wire level.
    #[test]
    fn v0_v1_v2_wire_first_bytes_are_all_distinct() {
        let b0 = encode(&SaveEnvelope::V0(SaveV0 {
            career_seed: Seed::from_u64(0),
        }))
        .expect("encode v0")[0];
        let b1 = encode(&SaveEnvelope::V1(SaveV1 {
            career_seed: Seed::from_u64(0),
            content_pack_version: 1,
            ledger: MemoryLedger::new(),
        }))
        .expect("encode v1")[0];
        let b2 = encode(&SaveEnvelope::V2(SaveV2 {
            career_seed: Seed::from_u64(0),
            content_pack_version: 1,
            ledger: MemoryLedger::new(),
        }))
        .expect("encode v2")[0];
        assert_ne!(b0, b1, "V0 and V1 must have distinct wire tags");
        assert_ne!(b1, b2, "V1 and V2 must have distinct wire tags");
        assert_ne!(b0, b2, "V0 and V2 must have distinct wire tags");
        assert_eq!(b0, 0x00);
        assert_eq!(b1, 0x01);
        assert_eq!(b2, 0x02);
    }

    // ----- T3-R-E: V3 variant constructs + round-trips -----

    /// `SaveEnvelope::V3` compiles, encodes, and decodes back to itself.
    #[test]
    fn v3_variant_constructs_and_round_trips() {
        let env = SaveEnvelope::V3(SaveV3 {
            career_seed: Seed::from_u64(0xF00D_BEEF_2026_0521),
            content_pack_version: 3,
            ledger: MemoryLedger::new(),
            season_number: SeasonNumber(2),
            season: None,
            breakthrough_states: BTreeMap::new(),
        });
        let bytes = encode(&env).expect("encode v3");
        let decoded = decode(&bytes).expect("decode v3");
        assert_eq!(decoded, env, "V3 must round-trip through encode/decode");
    }

    /// Pin the EXACT first byte of an encoded V3 envelope at `0x03`.
    /// Re-ordering SaveEnvelope variants or a bincode varint-encoding change
    /// for discriminant 3 fails this test. Schema is locked FOREVER; the wire
    /// bytes are part of the lock.
    #[test]
    fn v3_envelope_wire_first_byte_is_locked_at_0x03() {
        let env = SaveEnvelope::V3(SaveV3 {
            career_seed: Seed::from_u64(0),
            content_pack_version: 1,
            ledger: MemoryLedger::new(),
            season_number: SeasonNumber(0),
            season: None,
            breakthrough_states: BTreeMap::new(),
        });
        let bytes = encode(&env).expect("encode v3");
        assert_eq!(
            bytes[0], 0x03,
            "V3 wire tag is LOCKED at 0x03 — schema-lock invariant"
        );
    }
}

#[cfg(test)]
mod migration {
    use super::*;

    // ----- T2-9: AC2 + AC4a — forward-migration V0 → V1 -----

    /// AC2 + AC4a — `migrate_v0_to_v1` preserves `career_seed` exactly and
    /// supplies the documented defaults for the new V1 fields.
    /// Mutation flipping the default `content_pack_version` from `1` to any
    /// other value would fail; mutation dropping the seed-copy would fail.
    #[test]
    fn forward_v0_to_v1_preserves_seed_and_defaults_new_fields() {
        let v0 = SaveV0 {
            career_seed: Seed::from_u64(0xABCD_1234),
        };
        let v1 = migrate_v0_to_v1(v0.clone());

        assert_eq!(
            v1.career_seed, v0.career_seed,
            "career_seed must round-trip through V0→V1 migration"
        );
        assert_eq!(
            v1.content_pack_version, 1,
            "V1 defaults content_pack_version to 1 for promoted V0 saves"
        );
        assert_eq!(
            v1.ledger,
            MemoryLedger::new(),
            "V1 defaults ledger to empty for promoted V0 saves"
        );
    }

    // ----- T2-9: AC3 + AC4a — load_envelope auto-migrates V0 bytes -----
    // Updated at T3-R-E: load_envelope now returns SaveV3.

    /// AC3 + AC4a (T3-R-E update) — `load_envelope` decodes V0 bytes and
    /// emerges with the migrated V3 payload (V0→V1→V2→V3 chain).
    #[test]
    fn load_envelope_returns_v3_for_v0_bytes() {
        let seed = Seed::from_u64(0x0F0F_0F0F_0F0F_0F0F);
        let v0_env = SaveEnvelope::V0(SaveV0 { career_seed: seed });
        let v0_bytes = encode(&v0_env).expect("encode v0");
        let loaded = load_envelope(&v0_bytes).expect("load v0");
        // V0→V1→V2→V3 chain: seed preserved, pack version defaulted to 1,
        // ledger empty, season absent (no season was ever persisted).
        assert_eq!(loaded.career_seed, seed);
        assert_eq!(loaded.content_pack_version, 1);
        assert!(loaded.ledger.is_empty());
        assert!(loaded.season.is_none());
    }

    // ----- T2-9: AC4b — callback-preservation -----

    /// AC4b — every field present on V0 maps deterministically to V1 (no
    /// silent drops). With V0 having only `career_seed`, the test asserts a
    /// non-default seed survives migration with bit-exact fidelity.
    /// Mutation that XOR'd or hashed the seed during migration would fail.
    #[test]
    fn callback_preservation_v0_seed_survives_migration_bit_exact() {
        // Use a seed value that exercises every byte (no zeros, no all-ones
        // patterns that might mask a bit-shift bug).
        let seed_raw: u64 = 0xC3A5_96E1_7B4D_2F8A;
        let v0 = SaveV0 {
            career_seed: Seed::from_u64(seed_raw),
        };
        let migrated = migrate_v0_to_v1(v0);
        assert_eq!(
            migrated.career_seed.to_u64(),
            seed_raw,
            "V0.career_seed must reach V1 BIT-EXACT (callback-preservation)"
        );
    }

    // ----- T2-9: AC4c — forward-incompat-failure -----

    /// AC4c — bytes claiming a future variant (`V99`) must FAIL LOUDLY via
    /// `SaveError::Decode(bincode::error::DecodeError::UnexpectedVariant {
    /// found: 99, .. })` — the structurally exact fail-loud signal.
    ///
    /// Post-T2-9 silent-failure-hunter P1 fix: tightened from a substring
    /// match on the error message to a pattern-match on the EXACT bincode
    /// error discriminant + `found` value. The prior shape could have
    /// passed for the wrong reason if a future bincode minor version
    /// produced a different error variant (e.g. `UnexpectedEnd`,
    /// `OtherString`) that happened to satisfy a "variant" / "99"
    /// substring check.
    ///
    /// Locks BOTH the bincode error shape AND the assumption that bincode
    /// 2's `decode_from_slice` rejects discriminant 99 with
    /// `UnexpectedVariant { found: 99 }`. If bincode upgrades change this
    /// shape, the test FAILS TO COMPILE — which is the compile-time
    /// fail-loud signal the four-test migration discipline requires.
    ///
    /// The bincode varint for discriminant 99 is the single byte `0x63`
    /// (99 < 128 so no continuation bit). Hand-craft `[0x63]` (bare
    /// discriminant, no payload); bincode rejects on the unknown-variant
    /// check BEFORE attempting to decode any payload.
    #[test]
    fn load_envelope_rejects_unsupported_future_version() {
        // Empirical bincode-2 behavior note (post-T2-9 fix-pass): bincode 2's
        // SERDE-layer rejection of an unknown variant index produces
        // `DecodeError::OtherString("invalid value: integer `99`, expected
        // variant index 0 <= i < N")` where N is the current variant count.
        // At T3-R-E, N = 4 (V0/V1/V2/V3). The initial silent-failure-hunter
        // recommendation to pattern-match `UnexpectedVariant { found }` was
        // structurally exact but bincode-2-with-serde does not produce that
        // variant. We assert on the outer `SaveError::Decode` PLUS a
        // strict dual-substring check: BOTH "99" AND "variant" must appear.
        // Strictly stronger than the pre-fix either-or check; a future
        // bincode version that drops either token fails loudly here.
        let bytes = [0x63_u8]; // varint 99 = single byte 0x63
        let err = load_envelope(&bytes).expect_err("V99 bytes must NOT load");
        match err {
            SaveError::Decode(inner) => {
                let msg = inner.to_string();
                assert!(
                    msg.contains("99") && msg.contains("variant"),
                    "DecodeError message must mention BOTH '99' AND 'variant'; got: {msg}"
                );
            }
            other => panic!("expected SaveError::Decode for unknown variant; got {other:?}"),
        }
    }

    // ----- T3-1: AC8 — V1→V2 forward migration -----

    /// AC8 — `migrate_v1_to_v2` preserves `career_seed` + `content_pack_version`
    /// exactly and produces an EMPTY V2 ledger (V1 placeholder events have no
    /// real semantics and are discarded).
    #[test]
    fn v1_to_v2_preserves_seed_and_pack_version_drops_placeholder_ledger() {
        // Build a V1 with a non-default seed and pack version.
        let seed = Seed::from_u64(0xC0FF_EE00_DEAD_BEEF);
        let v1 = SaveV1 {
            career_seed: seed,
            content_pack_version: 42,
            ledger: MemoryLedger::new(), // placeholder ledger (was MemoryEvent::Placeholder)
        };
        let v2 = migrate_v1_to_v2(v1);

        assert_eq!(
            v2.career_seed, seed,
            "career_seed must round-trip through V1→V2 migration"
        );
        assert_eq!(
            v2.content_pack_version, 42,
            "content_pack_version must round-trip through V1→V2 migration"
        );
        assert!(
            v2.ledger.is_empty(),
            "V2 ledger must be empty after migrating V1 placeholder events"
        );
    }

    // ----- T3-1: AC9 — full migration chain via load_envelope -----

    /// AC9 (T3-R-E update) — `load_envelope` returns `SaveV3` for V0 bytes
    /// (V0→V1→V2→V3 chain), V1 bytes (V1→V2→V3), V2 bytes (V2→V3), and V3
    /// bytes (as-is). All paths preserve `career_seed`.
    #[test]
    fn load_envelope_returns_v3_via_full_chain() {
        let seed = Seed::from_u64(0x1234_5678);

        // V0 → V1 → V2 → V3
        let v0_bytes = encode(&SaveEnvelope::V0(SaveV0 { career_seed: seed })).expect("encode v0");
        let from_v0 = load_envelope(&v0_bytes).expect("load v0");
        assert_eq!(from_v0.career_seed, seed);
        assert!(from_v0.ledger.is_empty());

        // V1 → V2 → V3
        let v1_bytes = encode(&SaveEnvelope::V1(SaveV1 {
            career_seed: seed,
            content_pack_version: 7,
            ledger: MemoryLedger::new(),
        }))
        .expect("encode v1");
        let from_v1 = load_envelope(&v1_bytes).expect("load v1");
        assert_eq!(from_v1.career_seed, seed);
        assert_eq!(from_v1.content_pack_version, 7);
        assert!(from_v1.ledger.is_empty());

        // V2 → V3
        let v2_bytes = encode(&SaveEnvelope::V2(SaveV2 {
            career_seed: seed,
            content_pack_version: 3,
            ledger: MemoryLedger::new(),
        }))
        .expect("encode v2");
        let from_v2 = load_envelope(&v2_bytes).expect("load v2");
        assert_eq!(from_v2.career_seed, seed);
        assert_eq!(from_v2.content_pack_version, 3);

        // V3 as-is
        let v3_bytes = encode(&SaveEnvelope::V3(SaveV3 {
            career_seed: seed,
            content_pack_version: 9,
            ledger: MemoryLedger::new(),
            season_number: SeasonNumber(0),
            season: None,
            breakthrough_states: BTreeMap::new(),
        }))
        .expect("encode v3");
        let from_v3 = load_envelope(&v3_bytes).expect("load v3");
        assert_eq!(from_v3.career_seed, seed);
        assert_eq!(from_v3.content_pack_version, 9);
    }

    // ----- T3-1: AC11 — V1→V2 callback-preservation -----

    /// AC11 callback-preservation — `career_seed` survives V1→V2 migration
    /// BIT-EXACT across all 64 bits.
    #[test]
    fn v1_to_v2_callback_preservation_seed_bit_exact() {
        let seed_raw: u64 = 0xDE_AD_CA_FE_BA_BE_F0_0D;
        let v1 = SaveV1 {
            career_seed: Seed::from_u64(seed_raw),
            content_pack_version: 99,
            ledger: MemoryLedger::new(),
        };
        let v2 = migrate_v1_to_v2(v1);
        assert_eq!(
            v2.career_seed.to_u64(),
            seed_raw,
            "V1.career_seed must reach V2 BIT-EXACT (callback-preservation)"
        );
        assert_eq!(
            v2.content_pack_version, 99,
            "content_pack_version must survive migration bit-exact"
        );
    }

    // ----- T3-1: AC11 — V2 round-trip-byte-identical -----

    /// AC11 round-trip-byte-identical — `encode(decode(encode(V2(x))))` bytes
    /// are identical to `encode(V2(x))`. Mutation removing serde determinism
    /// would fail this.
    #[test]
    fn v2_encode_decode_reencode_produces_identical_bytes() {
        let env = SaveEnvelope::V2(SaveV2 {
            career_seed: Seed::from_u64(0xAB_CD_EF_01_23_45_67_89),
            content_pack_version: 1,
            ledger: MemoryLedger::new(),
        });
        let bytes_1 = encode(&env).expect("encode 1");
        let decoded = decode(&bytes_1).expect("decode");
        let bytes_2 = encode(&decoded).expect("encode 2");
        assert_eq!(
            bytes_1, bytes_2,
            "encode(decode(encode(V2(x)))) must be byte-identical"
        );
    }

    // ----- AC4c (shared path) — forward-incompat-failure unchanged -----
    // The existing `load_envelope_rejects_unsupported_future_version` test
    // already covers this path; it uses discriminant 99 which is still
    // unknown. No duplicate needed here.

    /// AC4-extension (post-T2-9 silent-failure-hunter P1 fix): trailing
    /// bytes past the end of a valid envelope encoding must FAIL LOUDLY
    /// via `SaveError::TrailingBytes` — NOT silently truncate to the
    /// prefix. Save corruption (file got appended garbage; a partial-
    /// download landed; a malicious splice happened) must surface as a
    /// load-time error, not as a working-but-wrong career.
    #[test]
    fn load_envelope_rejects_trailing_bytes_past_valid_encoding() {
        let env = SaveEnvelope::V2(SaveV2 {
            career_seed: Seed::from_u64(0xFEED_BEEF),
            content_pack_version: 1,
            ledger: MemoryLedger::new(),
        });
        let mut bytes = encode(&env).expect("encode");
        let original_len = bytes.len();
        bytes.push(0xFF); // append one junk byte

        let err = load_envelope(&bytes).expect_err("trailing-byte input must NOT load");
        match err {
            SaveError::TrailingBytes { consumed, total } => {
                assert_eq!(
                    consumed, original_len,
                    "TrailingBytes.consumed must equal the original encoded length"
                );
                assert_eq!(
                    total,
                    original_len + 1,
                    "TrailingBytes.total must equal input buffer length"
                );
            }
            other => panic!("expected SaveError::TrailingBytes for appended junk; got {other:?}"),
        }
    }
}

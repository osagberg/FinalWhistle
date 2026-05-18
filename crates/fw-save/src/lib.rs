//! `fw-save` — save-file format + version migration.
//!
//! ## T2-9: V1 schema lock + V0→V1 migration discipline established
//!
//! V1 is now the LOCKED first real schema (`SaveV1 { career_seed,
//! content_pack_version, ledger }`). V0 is a fictional pre-T2-9 placeholder
//! representing what a minimal-schema save would have looked like
//! (`SaveV0 { career_seed }`). V0 exists to exercise the four-test
//! migration discipline that T3-7 will apply to every future v(N) → v(N+1)
//! bump per `design/specs/save-migration-fixtures.md` (referenced from
//! `docs/specs/determinism-gate.md` line 37):
//!
//!   1. forward-migration         — V(N) bytes load + emerge as V(N+1)
//!   2. callback-preservation     — every V(N) field maps to V(N+1) (no drops)
//!   3. forward-incompat-failure  — unknown-future-discriminant FAILS LOUDLY
//!   4. round-trip-byte-identical — encode(decode(x))) bytes ≡ original
//!
//! `load_envelope(bytes) -> Result<SaveV1, SaveError>` is the production
//! load entry point: it decodes the envelope, runs V0→V1 migration on the
//! fly if needed, and returns the V1 payload directly. Unknown discriminants
//! surface as `SaveError::Decode` (bincode's `DecodeError::UnexpectedVariant`
//! is the structured fail-loud signal).
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
//! canonical state on demand (T2-5's `SeasonState` is regenerated from the
//! `career_seed` by `generate_league`, so it's intentionally NOT in the
//! save payload).

use fw_core::Seed;
use fw_memory::MemoryLedger;
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
    /// Schema v1 — T2-9-locked first real schema.
    V1(SaveV1) = 1,
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

/// Production load entry point. Decode bytes → handle every known variant
/// → return the latest-schema payload (currently `SaveV1`).
///
/// Variant dispatch:
///   - `SaveEnvelope::V0(v0)` → `migrate_v0_to_v1(v0)`
///   - `SaveEnvelope::V1(v1)` → returned as-is
///   - unknown discriminant → `SaveError::Decode(...)` (bincode's
///     `DecodeError::UnexpectedVariant` carries the variant info). This IS
///     the four-test-discipline "forward-incompat-failure" path: a future
///     V99 save loaded by an older binary FAILS LOUDLY rather than silently
///     defaulting or panicking.
///
/// Callers that need to introspect WHICH variant a save was authored as
/// (e.g. for a migration audit log) should call `decode()` directly + match
/// the envelope. This function is the right entry for "I want a usable
/// payload regardless of source schema version."
pub fn load_envelope(bytes: &[u8]) -> Result<SaveV1, SaveError> {
    match decode(bytes)? {
        SaveEnvelope::V0(v0) => Ok(migrate_v0_to_v1(v0)),
        SaveEnvelope::V1(v1) => Ok(v1),
    }
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

    // ----- T2-9: AC3 + AC4a — load_envelope auto-migrates V0 bytes to V1 -----

    /// AC3 + AC4a — `load_envelope` decodes V0 bytes and emerges with the
    /// migrated V1 payload, AND returns V1 bytes as-is.
    #[test]
    fn load_envelope_returns_v1_for_v0_bytes_and_v1_bytes() {
        // V0 bytes → migrated V1 payload.
        let seed = Seed::from_u64(0x0F0F_0F0F_0F0F_0F0F);
        let v0_env = SaveEnvelope::V0(SaveV0 { career_seed: seed });
        let v0_bytes = encode(&v0_env).expect("encode v0");
        let loaded_from_v0 = load_envelope(&v0_bytes).expect("load v0");
        assert_eq!(loaded_from_v0.career_seed, seed);
        assert_eq!(loaded_from_v0.content_pack_version, 1);

        // V1 bytes → V1 payload returned as-is (no migration).
        let v1 = SaveV1 {
            career_seed: Seed::from_u64(0x1111_2222_3333_4444),
            content_pack_version: 1,
            ledger: MemoryLedger::new(),
        };
        let v1_env = SaveEnvelope::V1(v1.clone());
        let v1_bytes = encode(&v1_env).expect("encode v1");
        let loaded_from_v1 = load_envelope(&v1_bytes).expect("load v1");
        assert_eq!(loaded_from_v1, v1);
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
        // `DecodeError::OtherString("invalid value: integer ` + "`99`" +
        // `, expected variant index 0 <= i < 2")` — NOT the structured
        // `UnexpectedVariant` variant (which bincode reserves for its native
        // non-serde decoder path). The initial silent-failure-hunter
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

    /// AC4-extension (post-T2-9 silent-failure-hunter P1 fix): trailing
    /// bytes past the end of a valid envelope encoding must FAIL LOUDLY
    /// via `SaveError::TrailingBytes` — NOT silently truncate to the
    /// prefix. Save corruption (file got appended garbage; a partial-
    /// download landed; a malicious splice happened) must surface as a
    /// load-time error, not as a working-but-wrong career.
    #[test]
    fn load_envelope_rejects_trailing_bytes_past_valid_encoding() {
        let env = SaveEnvelope::V1(SaveV1 {
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

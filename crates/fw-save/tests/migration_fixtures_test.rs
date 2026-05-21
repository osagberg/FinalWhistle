//! Committed-fixture migration verifier — T3-7.
//!
//! ## Purpose
//!
//! `v1_to_v2_migration_test.rs` (T3-1) constructs `SaveV1`/`SaveV2` envelopes
//! IN-CODE, encodes them, then decodes them — it always encodes with the
//! CURRENT encoder. T3-7 is distinct: it commits FROZEN on-disk binary save
//! bytes and verifies they still load. A committed fixture is frozen input —
//! if a future decoder or schema change breaks backward compat, these tests
//! catch it where the encode-then-decode-this-run tests cannot. This is the
//! genuine "a real 6-month-old save still loads" regression guard.
//!
//! ## Four-test discipline (CLAUDE.md §9 — authority)
//!
//! Every schema version bump owes four tests:
//!   1. forward-migration         — old bytes load + emerge as the current schema
//!   2. callback-preservation     — every old-schema field maps to new schema (no drops)
//!   3. forward-incompat-failure  — unknown-future-discriminant FAILS LOUDLY
//!   4. round-trip-byte-identical — encode(decode(frozen_bytes)) == frozen_bytes
//!
//! T3-7 adds a fifth test for the V0→V1→V2 full chain.
//!
//! ## Fixture definitions (load-bearing; must match README.md)
//!
//! `v1_sample.fwsave`:
//!   SaveEnvelope::V1(SaveV1 {
//!       career_seed: Seed::from_u64(0x5A5E_F1C7_0001_0002),
//!       content_pack_version: 1,
//!       ledger: MemoryLedger::new(),
//!   })
//!   Wire bytes: [0x01, ...] (V1 tag = 0x01)
//!
//! `v0_sample.fwsave`:
//!   SaveEnvelope::V0(SaveV0 {
//!       career_seed: Seed::from_u64(0xA0B1_C2D3_E4F5_0001),
//!   })
//!   Wire bytes: [0x00, ...] (V0 tag = 0x00)
//!
//! `v99_future.fwsave`:
//!   Hand-crafted bytes: [0x63] — bincode-2 varint for discriminant 99.
//!   99 < 128, so no continuation bit; the discriminant is the single byte
//!   0x63. bincode rejects unknown variants before attempting payload decode.
//!   This mirrors the byte construction in `lib.rs` migration::load_envelope_rejects_unsupported_future_version.
//!
//! ## Regeneration
//!
//! Run:
//!   cargo test -p fw-save --test migration_fixtures_test -- --ignored regenerate_fixtures
//!
//! The `#[ignore]`-gated `regenerate_fixtures` test writes all three files.
//! Run it once to bootstrap; re-run any time the encoder changes (requires an
//! intentional schema bump + re-pin, per T3-7 discipline).

use std::path::PathBuf;

use fw_core::Seed;
use fw_memory::MemoryLedger;
use fw_save::{SaveEnvelope, SaveError, SaveV0, SaveV1, decode, encode, load_envelope};

// -------------------------------------------------------------------------
// Fixture path helpers
// -------------------------------------------------------------------------

/// The `fixtures/save-migration/v0001-to-v0002/` directory, resolved relative
/// to `fw-save`'s Cargo manifest dir (which is `crates/fw-save/`), so the
/// repo-root `fixtures/` is two levels up.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..") // crates/
        .join("..") // repo root
        .join("fixtures")
        .join("save-migration")
        .join("v0001-to-v0002")
}

fn v1_sample_path() -> PathBuf {
    fixtures_dir().join("v1_sample.fwsave")
}

fn v0_sample_path() -> PathBuf {
    fixtures_dir().join("v0_sample.fwsave")
}

fn v99_future_path() -> PathBuf {
    fixtures_dir().join("v99_future.fwsave")
}

// -------------------------------------------------------------------------
// Documented fixture values (single source of truth; mirror README.md)
// -------------------------------------------------------------------------

/// The career seed encoded into `v1_sample.fwsave`.
const V1_SAMPLE_SEED: u64 = 0x5A5E_F1C7_0001_0002;

/// The content_pack_version encoded into `v1_sample.fwsave`.
const V1_SAMPLE_CONTENT_PACK_VERSION: u32 = 1;

/// The career seed encoded into `v0_sample.fwsave`.
const V0_SAMPLE_SEED: u64 = 0xA0B1_C2D3_E4F5_0001;

// -------------------------------------------------------------------------
// Chunk 1: #[ignore]-gated fixture regeneration
// -------------------------------------------------------------------------
// TDD-exemption: this is data-authoring code that WRITES the committed binary
// fixtures to disk. TDD applies to the verifier tests (chunk 2) that READ
// those fixtures and assert behavior. The regen test is the bootstrap
// mechanism, not the system under test; running it once produces the frozen
// inputs that the verifier tests exercise.

/// Bootstrap / idempotent regeneration of all three fixture binaries.
///
/// This test is `#[ignore]`-gated — it must NOT run in CI (it would
/// overwrite the committed fixtures on the CI runner's checkout, making
/// the byte-identity tests flap). Run manually once to bootstrap:
///
///   cargo test -p fw-save --test migration_fixtures_test -- --ignored regenerate_fixtures
///
/// After regeneration, commit the three `.fwsave` files. Re-run only when
/// the encoder changes (new schema bump + explicit re-pin decision).
///
/// The fixture values written here are the canonical definitions; the
/// README.md and the constants above mirror them. If you change the
/// constants, re-run this test to update the binaries, then verify
/// all five committed-fixture verifier tests still pass.
#[test]
#[ignore = "fixture regeneration — run manually to bootstrap or re-pin; not for CI"]
fn regenerate_fixtures() {
    let dir = fixtures_dir();
    std::fs::create_dir_all(&dir).expect("create fixtures dir");

    // --- v1_sample.fwsave ---
    let v1_env = SaveEnvelope::V1(SaveV1 {
        career_seed: Seed::from_u64(V1_SAMPLE_SEED),
        content_pack_version: V1_SAMPLE_CONTENT_PACK_VERSION,
        ledger: MemoryLedger::new(),
    });
    let v1_bytes = encode(&v1_env).expect("encode V1 fixture");
    assert_eq!(
        v1_bytes[0], 0x01,
        "V1 wire tag must be 0x01 — schema-lock invariant"
    );
    std::fs::write(v1_sample_path(), &v1_bytes).expect("write v1_sample.fwsave");

    // --- v0_sample.fwsave ---
    let v0_env = SaveEnvelope::V0(SaveV0 {
        career_seed: Seed::from_u64(V0_SAMPLE_SEED),
    });
    let v0_bytes = encode(&v0_env).expect("encode V0 fixture");
    assert_eq!(
        v0_bytes[0], 0x00,
        "V0 wire tag must be 0x00 — schema-lock invariant"
    );
    std::fs::write(v0_sample_path(), &v0_bytes).expect("write v0_sample.fwsave");

    // --- v99_future.fwsave ---
    // bincode-2 encodes enum discriminant 99 as a single varint byte: 0x63.
    // 99 < 128 so no continuation bit. This mirrors the byte construction
    // in `lib.rs` migration::load_envelope_rejects_unsupported_future_version.
    // We intentionally hand-craft this rather than encoding through SaveEnvelope
    // (which has no V99 variant), so the fixture exercises the actual on-wire
    // shape of an unknown future save version.
    let v99_bytes: Vec<u8> = vec![0x63_u8];
    std::fs::write(v99_future_path(), &v99_bytes).expect("write v99_future.fwsave");

    println!("Fixtures written to: {}", dir.display());
    println!(
        "  v1_sample.fwsave  {} bytes  (first byte 0x{:02x})",
        v1_bytes.len(),
        v1_bytes[0]
    );
    println!(
        "  v0_sample.fwsave  {} bytes  (first byte 0x{:02x})",
        v0_bytes.len(),
        v0_bytes[0]
    );
    println!(
        "  v99_future.fwsave {} bytes  (first byte 0x{:02x})",
        v99_bytes.len(),
        v99_bytes[0]
    );
}

// -------------------------------------------------------------------------
// Chunk 2: committed-fixture verifier tests (5 tests)
// -------------------------------------------------------------------------
// These tests READ the committed frozen binaries and assert migration
// behavior. Each must fail if:
//   - the production migration code regresses (e.g. migrate_v1_to_v2 zeros
//     the career_seed instead of preserving it), OR
//   - a fixture's documented value drifts (e.g. v1_sample.fwsave is replaced
//     with a file encoding a different seed).
// Neither encode-then-decode-this-run nor in-code migration tests can catch
// both of these independently — the frozen fixture is the irreplaceable layer.

// -------------------------------------------------------------------------
// AC2: forward-migration (CLAUDE.md §9 test 1)
// -------------------------------------------------------------------------

/// `load_envelope` on the committed `v1_sample.fwsave` bytes produces a
/// `SaveV2` with the exact documented `career_seed` + `content_pack_version`
/// and an empty ledger (V1 placeholder ledger drops on migration).
///
/// Non-vacuousness: mutating `migrate_v1_to_v2` to zero `career_seed` would
/// fail the `assert_eq!(v2.career_seed.to_u64(), V1_SAMPLE_SEED)`. Mutating
/// it to change `content_pack_version` would fail the pack-version assertion.
/// Replacing the fixture with bytes encoding a different seed would fail both.
#[test]
fn fixture_v1_forward_migrates_to_v2() {
    let bytes = std::fs::read(v1_sample_path())
        .expect("read v1_sample.fwsave — run `regenerate_fixtures` (--ignored) to bootstrap");
    let v2 = load_envelope(&bytes)
        .expect("v1_sample.fwsave must load via V1→V2 migration without error");

    assert_eq!(
        v2.career_seed.to_u64(),
        V1_SAMPLE_SEED,
        "forward-migration: career_seed from committed v1 fixture must match documented value"
    );
    assert_eq!(
        v2.content_pack_version, V1_SAMPLE_CONTENT_PACK_VERSION,
        "forward-migration: content_pack_version from committed v1 fixture must match documented value"
    );
    assert!(
        v2.ledger.is_empty(),
        "forward-migration: V1 placeholder ledger must migrate to an empty V2 ledger"
    );
}

// -------------------------------------------------------------------------
// AC3: callback-preservation (CLAUDE.md §9 test 2)
// -------------------------------------------------------------------------

/// Every field on V1 maps to V2 in the committed fixture's migration output —
/// no silent drops. Asserts bit-exact values for `career_seed` (all 64 bits)
/// and `content_pack_version` (all 32 bits) against the README-documented
/// fixture values.
///
/// Non-vacuousness: this is a separate test from `fixture_v1_forward_migrates_to_v2`
/// so a future refactor that forgets to assert `content_pack_version` in the
/// forward-migration test doesn't leave callback-preservation unchecked.
/// Mutating `migrate_v1_to_v2` to drop or transform any V1 field must fail
/// at least one of these two assertions.
#[test]
fn fixture_v1_all_fields_preserved() {
    let bytes = std::fs::read(v1_sample_path())
        .expect("read v1_sample.fwsave — run `regenerate_fixtures` (--ignored) to bootstrap");
    let v2 = load_envelope(&bytes)
        .expect("v1_sample.fwsave must load cleanly for callback-preservation check");

    // career_seed: all 64 bits preserved
    assert_eq!(
        v2.career_seed.to_u64(),
        V1_SAMPLE_SEED,
        "callback-preservation: career_seed must be BIT-EXACT across V1→V2 (all 64 bits)"
    );

    // content_pack_version: all 32 bits preserved
    assert_eq!(
        v2.content_pack_version, V1_SAMPLE_CONTENT_PACK_VERSION,
        "callback-preservation: content_pack_version must be BIT-EXACT across V1→V2 (all 32 bits)"
    );

    // ledger: V1 placeholder events → empty V2 ledger (no data dropped, because
    // V1 placeholder events had no real semantics — the empty mapping IS
    // preservation of the intent).
    assert!(
        v2.ledger.is_empty(),
        "callback-preservation: placeholder ledger maps to empty V2 ledger (no data lost)"
    );
}

// -------------------------------------------------------------------------
// AC4: forward-incompat-failure (CLAUDE.md §9 test 3)
// -------------------------------------------------------------------------

/// Loading the committed `v99_future.fwsave` bytes via `load_envelope` MUST
/// fail with `SaveError::Decode` whose message contains BOTH "99" AND "variant".
///
/// This is the "forward-incompat-failure" test: an old binary must loudly
/// reject a save written by a future binary it doesn't understand. Silence
/// here is the exact failure mode the four-test discipline forbids.
///
/// The dual-substring requirement (`"99"` AND `"variant"`) matches the rigor
/// of `lib.rs::migration::load_envelope_rejects_unsupported_future_version`.
/// bincode 2's serde adapter produces:
///   `invalid value: integer \`99\`, expected variant index 0 <= i < N`
/// Both substrings are guaranteed by that message format; a future bincode
/// version that drops either fails loudly here.
///
/// Non-vacuousness: replacing the fixture with a valid V1/V2 save would
/// cause `load_envelope` to return `Ok(...)`, failing `expect_err`. Replacing
/// it with a file that produces an error NOT mentioning "99" would fail the
/// substring assertions.
#[test]
fn fixture_v99_fails_loudly() {
    let bytes = std::fs::read(v99_future_path())
        .expect("read v99_future.fwsave — run `regenerate_fixtures` (--ignored) to bootstrap");

    let err = load_envelope(&bytes)
        .expect_err("v99_future.fwsave must NOT load successfully — forward-incompat-failure");

    match err {
        SaveError::Decode(inner) => {
            let msg = inner.to_string();
            assert!(
                msg.contains("99") && msg.contains("variant"),
                "forward-incompat-failure: error message must mention BOTH '99' AND 'variant'; got: {msg}"
            );
        }
        other => panic!(
            "forward-incompat-failure: expected SaveError::Decode for unknown discriminant 99; got {other:?}"
        ),
    }
}

// -------------------------------------------------------------------------
// AC5: round-trip-byte-identical (CLAUDE.md §9 test 4)
// -------------------------------------------------------------------------

/// `encode(decode(committed_v1_bytes))` is byte-identical to the committed
/// bytes. This proves TWO things simultaneously:
///   1. The committed fixture IS a canonical encoding (no trailing garbage,
///      no non-canonical varint, no padding).
///   2. The codec is stable against frozen input — a future serde-change that
///      introduces non-determinism in the encoding would fail this.
///
/// Non-vacuousness: mutating `encode` to append a version byte would produce
/// different-length bytes and fail `assert_eq!`. Mutating `decode` to succeed
/// on non-canonical encodings would only be caught here if the fixture itself
/// were non-canonical (it's not — `regenerate_fixtures` produces canonical
/// output). However, a regression that changes the encode output IS caught:
/// re-encoding the decoded envelope must reproduce the same bytes.
#[test]
fn fixture_v1_round_trip_byte_identical() {
    let bytes = std::fs::read(v1_sample_path())
        .expect("read v1_sample.fwsave — run `regenerate_fixtures` (--ignored) to bootstrap");

    let envelope = decode(&bytes)
        .expect("decode of committed v1_sample.fwsave must succeed for round-trip test");
    let re_encoded =
        encode(&envelope).expect("re-encode of decoded v1_sample.fwsave envelope must succeed");

    assert_eq!(
        bytes, re_encoded,
        "round-trip-byte-identical: encode(decode(committed_v1_bytes)) must equal committed_v1_bytes"
    );
}

// -------------------------------------------------------------------------
// AC6: V0→V1→V2 full chain (T3-7 extension beyond CLAUDE.md §9 core four)
// -------------------------------------------------------------------------

/// Loading the committed `v0_sample.fwsave` via `load_envelope` traverses the
/// full V0→V1→V2 chain and produces a `SaveV2` with the documented seed.
///
/// V0→V1 defaults `content_pack_version = 1` and `ledger = empty`.
/// V1→V2 preserves those defaults. The final payload must reflect all three
/// hops.
///
/// Non-vacuousness: mutating `migrate_v0_to_v1` to drop `career_seed` would
/// fail the seed assertion. Mutating it to set a non-1 `content_pack_version`
/// default would fail the pack-version assertion. Replacing the fixture with
/// a V1 file would change the test's migration-path coverage (V0 start → V1
/// direct decode, losing the V0→V1 hop) without failing the assertions —
/// except that a V1 file has a different first byte (0x01 vs 0x00), which is
/// pinned by `regenerate_fixtures`. The idempotency check in the self-review
/// verification section catches this at the regen level.
#[test]
fn fixture_v0_traverses_full_chain() {
    let bytes = std::fs::read(v0_sample_path())
        .expect("read v0_sample.fwsave — run `regenerate_fixtures` (--ignored) to bootstrap");

    // Verify the fixture really IS a V0 save (first wire byte = 0x00).
    assert_eq!(
        bytes[0], 0x00,
        "v0_sample.fwsave first byte must be 0x00 (V0 tag) — wrong fixture loaded?"
    );

    let v2 =
        load_envelope(&bytes).expect("v0_sample.fwsave must load via V0→V1→V2 chain without error");

    assert_eq!(
        v2.career_seed.to_u64(),
        V0_SAMPLE_SEED,
        "full-chain: career_seed from committed v0 fixture must match documented value"
    );
    // V0→V1 migration defaults content_pack_version to 1.
    assert_eq!(
        v2.content_pack_version, 1,
        "full-chain: V0→V1 migration must default content_pack_version to 1"
    );
    assert!(
        v2.ledger.is_empty(),
        "full-chain: V0→V1→V2 chain must produce an empty ledger"
    );
}

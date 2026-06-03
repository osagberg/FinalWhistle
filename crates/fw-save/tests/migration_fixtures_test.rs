//! Committed-fixture migration verifier — T3-7 + T4-2.5g.
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
//! T3-7 adds a fifth test for the full V0→V1→V2→V3 chain; T3-R-C adds three
//! more for the frozen non-empty-ledger V2 fixture; T3-R-E adds three more for
//! the frozen V3 career fixture (decode, byte-identical re-encode, resume).
//! T4-2.5g adds four more for the frozen V4 career fixture (decode,
//! round-trip-byte-identical, resumes, V3→V4 forward-migration).
//!
//! ## Fixture definitions (load-bearing; must match README.md)
//!
//! `v4_career_sample.fwsave` (T4-2.5g):
//!   SaveEnvelope::V4(SaveV4 {
//!       career_seed: Seed::from_u64(0x7E57_C0DE_0004_0005),
//!       content_pack_version: 1,
//!       ledger: <2 plain MemoryEvents>,
//!       season_number: SeasonNumber(3),
//!       season: Some(<SeasonState from generate_league(0xCAFE_F00D)>),
//!       roster: <3 hand-built SavedPlayerInstances with distinct non-default deltas>,
//!       breakthrough_eval_watermark: 7,
//!   })
//!   Wire bytes: [0x04, ...] (V4 tag = 0x04). The ONLY frozen V4 fixture;
//!   roster is non-empty so the round-trip exercises `SavedPlayerInstance` serde.
//!
//! ## Regeneration
//!
//! Run:
//!   cargo test -p fw-save --test migration_fixtures_test -- --ignored regenerate_fixtures
//!
//! The `#[ignore]`-gated `regenerate_fixtures` test writes all six files.
//! Run it once to bootstrap; re-run any time the encoder changes (requires an
//! intentional schema bump + re-pin, per T3-7 discipline).

use std::collections::BTreeMap;
use std::path::PathBuf;

use fw_content::{ContentStore, SeasonState, generate_league};
use fw_core::{AbilityCeiling, ClubId, MatchId, PlayerId, PlayerSeasonStats, Q32, Seed, Tick};
use fw_memory::{
    BreakthroughState, CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter,
    EmitterKind, Emotion, EntityRef, EventClass, EventId, MemoryEvent, MemoryLedger, Participant,
    ParticipantRole, SeasonNumber, SourceId,
};
use fw_save::{
    SaveEnvelope, SaveError, SaveV0, SaveV1, SaveV2, SaveV3, SaveV4, SavedPlayerInstance, decode,
    encode, load_envelope,
};

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

fn v2_nonempty_path() -> PathBuf {
    fixtures_dir().join("v2_nonempty_ledger_sample.fwsave")
}

fn v3_career_path() -> PathBuf {
    fixtures_dir().join("v3_career_sample.fwsave")
}

fn v4_career_path() -> PathBuf {
    fixtures_dir().join("v4_career_sample.fwsave")
}

/// Workspace-root `content/` directory — for building the V3 fixture's real
/// `SeasonState` snapshot via `generate_league`.
fn content_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
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

/// The career seed encoded into `v2_nonempty_ledger_sample.fwsave`.
const V2_NONEMPTY_SEED: u64 = 0x7E57_C0DE_0002_0003;

/// The content_pack_version encoded into `v2_nonempty_ledger_sample.fwsave`.
const V2_NONEMPTY_CONTENT_PACK_VERSION: u32 = 1;

/// Total row count of the frozen `v2_nonempty_ledger_sample.fwsave` ledger:
/// 2 plain `MemoryEvent`s + 1 `Compaction` event.
const V2_NONEMPTY_LEDGER_LEN: usize = 3;

/// The career seed encoded into `v3_career_sample.fwsave`.
const V3_CAREER_SEED: u64 = 0x7E57_C0DE_0003_0004;

/// The `season_number` encoded into `v3_career_sample.fwsave`.
const V3_CAREER_SEASON_NUMBER: u16 = 2;

/// Seed handed to `generate_league` when building the V3 fixture's `SeasonState`.
const V3_CAREER_LEAGUE_SEED: u64 = 0xCAFE_F00D;

/// The career seed encoded into `v4_career_sample.fwsave`.
const V4_CAREER_SEED: u64 = 0x7E57_C0DE_0004_0005;

/// The `season_number` encoded into `v4_career_sample.fwsave`.
const V4_CAREER_SEASON_NUMBER: u16 = 3;

/// The `breakthrough_eval_watermark` encoded into `v4_career_sample.fwsave`.
/// `u64` to match the `SaveV4` field (fixed-width wire type). NOTE: bincode-2
/// varint encodes `usize` and `u64` identically, so this type change vs the
/// original `usize` does NOT alter the frozen fixture bytes — the byte-identical
/// round-trip test still holds against the committed `v4_career_sample.fwsave`.
const V4_CAREER_WATERMARK: u64 = 7;

/// Seed for the V4 fixture's `SeasonState` (same league seed as V3 — reuse
/// the baked season so the fixture bytes stay minimal and the content
/// dependency is clear).
const V4_CAREER_LEAGUE_SEED: u64 = 0xCAFE_F00D;

/// Number of `SavedPlayerInstance` rows in the V4 fixture's roster.
/// The fixture has one club with 3 players (hand-built — fw-save tests must
/// not depend on fw-tauri's `build_roster_from_league`).
const V4_ROSTER_INSTANCE_COUNT: usize = 3;

/// Sentinel: the `career_apps` value on the FIRST `SavedPlayerInstance` in
/// the V4 fixture. Must be non-zero to prove the overlay is non-vacuous.
const V4_SENTINEL_CAREER_APPS: u32 = 17;

// -------------------------------------------------------------------------
// Non-empty V2 ledger fixture construction
// -------------------------------------------------------------------------
// The v0/v1 fixtures all carry EMPTY ledgers (`MemoryLedger::new()`), so the
// round-trip-byte-identical test never exercises the `MemoryEvent` serde
// surface. `v2_nonempty_ledger_sample.fwsave` freezes a ledger with real rows
// so a backward-compat regression in `MemoryEvent` encoding (a field reorder,
// an `EventClass` discriminant shift) is caught against frozen bytes.

/// Build a deterministic plain `MemoryEvent` for the non-empty V2 fixture.
/// Mirrors the field shape of `fw-memory`'s internal `make_event` test helper.
/// `event_id` and `salience` are overwritten by `MemoryLedger::append`.
fn sample_memory_event(season: u16, event_class: EventClass) -> MemoryEvent {
    MemoryEvent {
        event_id: EventId(0),
        schema_version: 1,
        season: SeasonNumber(season),
        tick: Some(Tick::ZERO),
        career_date: CareerDate {
            year: 1,
            day_of_year: 1,
        },
        emitter: Emitter {
            kind: EmitterKind::MatchEngine,
            source_id: SourceId::Match(MatchId::new(0)),
        },
        participants: vec![Participant {
            role: ParticipantRole::Subject,
            entity: EntityRef::Player(PlayerId::new(1)),
        }],
        event_class,
        stakes: Q32::ZERO,
        emotion: Emotion::Neutral,
        consequence: vec![Consequence::None],
        callback_eligibility: CallbackEligibility::Immediate,
        salience: Q32::ZERO,
        decay_function: DecayFunction::Never,
    }
}

/// Build the non-empty V2 ledger frozen into `v2_nonempty_ledger_sample.fwsave`:
/// a season-0 event + a season-5 event, then `compact(SeasonNumber(5))` which
/// nulls the season-0 event's tick and appends one `Compaction` event. Result:
/// 3 rows — 2 plain `MemoryEvent`s (one `tick: None`, one `tick: Some`) + 1
/// `Compaction`. Fully deterministic: no clocks, no RNG, sequential `EventId`s.
fn build_v2_nonempty_ledger() -> MemoryLedger {
    let mut ledger = MemoryLedger::new();
    ledger.append(sample_memory_event(0, EventClass::DebutSenior));
    ledger.append(sample_memory_event(5, EventClass::LegacyGoal));
    ledger.compact(SeasonNumber(5));
    ledger
}

/// Build the `SaveEnvelope::V3` frozen into `v3_career_sample.fwsave`: a
/// 2-event ledger, `season_number = 2`, and a real `Some(SeasonState)`
/// snapshot generated from the shipped content pack. The `Some(SeasonState)`
/// exercises the V3-only `Option<SeasonState>` serde surface against frozen
/// bytes — the v0/v1/v2 fixtures cannot (V2 has no season field).
fn build_v3_career_envelope() -> SaveEnvelope {
    let content = ContentStore::load_sources(&content_root())
        .expect("load content/ for the V3 fixture's SeasonState snapshot");
    let league = generate_league(Seed::from_u64(V3_CAREER_LEAGUE_SEED), &content)
        .expect("generate_league must succeed against the shipped pack");
    let season = SeasonState::new(league, &content);

    let mut ledger = MemoryLedger::new();
    ledger.append(sample_memory_event(0, EventClass::DebutSenior));
    ledger.append(sample_memory_event(1, EventClass::LegacyGoal));

    SaveEnvelope::V3(SaveV3 {
        career_seed: Seed::from_u64(V3_CAREER_SEED),
        content_pack_version: 1,
        ledger,
        season_number: SeasonNumber(V3_CAREER_SEASON_NUMBER),
        season: Some(season),
        breakthrough_states: std::collections::BTreeMap::new(),
    })
}

/// Build a deterministic `SavedPlayerInstance` for the V4 fixture.
///
/// Hand-constructed values — fw-save tests must NOT depend on fw-tauri's
/// `build_roster_from_league` (that would create a circular dependency).
/// The documented sentinel values (`V4_SENTINEL_CAREER_APPS`, etc.) are
/// embedded here so assertions in the verifier tests stay non-vacuous.
fn build_saved_player_instance(
    player_id: u32,
    club_id: u32,
    slot: u8,
    career_apps: u32,
) -> SavedPlayerInstance {
    SavedPlayerInstance {
        player_id: PlayerId::new(player_id),
        club_id: ClubId::new(club_id),
        slot,
        ceiling: AbilityCeiling::try_new(Q32::ZERO, Q32::ONE)
            .expect("AbilityCeiling::try_new(0, 1) is always valid"),
        breakthrough_state: BreakthroughState::new(),
        season_stats: PlayerSeasonStats {
            appearances: career_apps as u16,
            goals: 0,
            assists: 0,
            minutes_played: career_apps * 90,
            average_rating_numerator: 0,
            rating_sample_count: 0,
        },
        career_apps,
        observation_count: (career_apps / 5).max(1),
    }
}

/// Build the `SaveEnvelope::V4` frozen into `v4_career_sample.fwsave`: a
/// 2-event ledger, `season_number = 3`, a real `Some(SeasonState)` snapshot,
/// a non-empty roster (3 hand-built `SavedPlayerInstance`s in one club with
/// distinct `career_apps` values so the round-trip test is non-vacuous), and
/// `breakthrough_eval_watermark = V4_CAREER_WATERMARK`.
///
/// fw-save tests never import fw-tauri (no `build_roster_from_league`), so
/// `SavedPlayerInstance`s are constructed directly with known field values.
fn build_v4_career_envelope() -> SaveEnvelope {
    let content = ContentStore::load_sources(&content_root())
        .expect("load content/ for the V4 fixture's SeasonState snapshot");
    let league = generate_league(Seed::from_u64(V4_CAREER_LEAGUE_SEED), &content)
        .expect("generate_league must succeed against the shipped pack");
    let season = SeasonState::new(league, &content);

    let mut ledger = MemoryLedger::new();
    ledger.append(sample_memory_event(0, EventClass::DebutSenior));
    ledger.append(sample_memory_event(2, EventClass::LegacyGoal));

    // One club with 3 hand-built instances. ClubId(1) / PlayerIds 1_000_000..=1_000_002
    // are plausible roster ids (≥ ROSTER_PLAYER_ID_BASE = 1_000_000) without
    // importing fw-tauri's ROSTER_PLAYER_ID_BASE constant.
    const BASE: u32 = 1_000_000;
    const CLUB: u32 = 1;
    let mut roster: BTreeMap<ClubId, Vec<SavedPlayerInstance>> = BTreeMap::new();
    roster.insert(
        ClubId::new(CLUB),
        vec![
            build_saved_player_instance(BASE, CLUB, 0, V4_SENTINEL_CAREER_APPS),
            build_saved_player_instance(BASE + 1, CLUB, 1, 9),
            build_saved_player_instance(BASE + 2, CLUB, 2, 3),
        ],
    );

    SaveEnvelope::V4(SaveV4 {
        career_seed: Seed::from_u64(V4_CAREER_SEED),
        content_pack_version: 1,
        ledger,
        season_number: SeasonNumber(V4_CAREER_SEASON_NUMBER),
        season: Some(season),
        roster,
        breakthrough_eval_watermark: V4_CAREER_WATERMARK,
    })
}

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
/// all eleven committed-fixture verifier tests still pass.
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

    // --- v2_nonempty_ledger_sample.fwsave ---
    // A V2 save whose ledger carries real rows — 2 plain MemoryEvents + 1
    // Compaction event (appended by compact() at the 5-season boundary). The
    // v0/v1 fixtures all carry EMPTY ledgers; this is the only frozen fixture
    // that exercises the MemoryEvent serde surface against backward-compat drift.
    let v2_env = SaveEnvelope::V2(SaveV2 {
        career_seed: Seed::from_u64(V2_NONEMPTY_SEED),
        content_pack_version: V2_NONEMPTY_CONTENT_PACK_VERSION,
        ledger: build_v2_nonempty_ledger(),
    });
    let v2_bytes = encode(&v2_env).expect("encode V2 fixture");
    assert_eq!(
        v2_bytes[0], 0x02,
        "V2 wire tag must be 0x02 — schema-lock invariant"
    );
    std::fs::write(v2_nonempty_path(), &v2_bytes).expect("write v2_nonempty_ledger_sample.fwsave");

    // --- v3_career_sample.fwsave ---
    // A V3 career save: a 2-event ledger + season_number 2 + a real
    // Some(SeasonState) snapshot. This is the only frozen fixture that
    // exercises the V3 Option<SeasonState> serde surface against drift.
    let v3_env = build_v3_career_envelope();
    let v3_bytes = encode(&v3_env).expect("encode V3 fixture");
    assert_eq!(
        v3_bytes[0], 0x03,
        "V3 wire tag must be 0x03 — schema-lock invariant"
    );
    std::fs::write(v3_career_path(), &v3_bytes).expect("write v3_career_sample.fwsave");

    // --- v4_career_sample.fwsave ---
    // A V4 career save: a 2-event ledger + season_number 3 + a real
    // Some(SeasonState) snapshot + a non-empty roster (3 hand-built
    // SavedPlayerInstances with distinct career_apps). This is the ONLY frozen
    // fixture that exercises the SavedPlayerInstance serde surface against drift.
    let v4_env = build_v4_career_envelope();
    let v4_bytes = encode(&v4_env).expect("encode V4 fixture");
    assert_eq!(
        v4_bytes[0], 0x04,
        "V4 wire tag must be 0x04 — schema-lock invariant"
    );
    std::fs::write(v4_career_path(), &v4_bytes).expect("write v4_career_sample.fwsave");

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
    println!(
        "  v2_nonempty_ledger_sample.fwsave {} bytes  (first byte 0x{:02x})",
        v2_bytes.len(),
        v2_bytes[0]
    );
    println!(
        "  v3_career_sample.fwsave {} bytes  (first byte 0x{:02x})",
        v3_bytes.len(),
        v3_bytes[0]
    );
    println!(
        "  v4_career_sample.fwsave {} bytes  (first byte 0x{:02x})",
        v4_bytes.len(),
        v4_bytes[0]
    );
}

// -------------------------------------------------------------------------
// Chunk 2: committed-fixture verifier tests (11 tests)
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
/// current-schema (`SaveV4`) payload with the exact documented `career_seed` +
/// `content_pack_version` and an empty ledger (V1 placeholder ledger drops on
/// migration). Roster is empty (V1 predates roster persistence).
///
/// Non-vacuousness: mutating `migrate_v1_to_v2` to zero `career_seed` would
/// fail the `assert_eq!(loaded.career_seed.to_u64(), V1_SAMPLE_SEED)`. Mutating
/// it to change `content_pack_version` would fail the pack-version assertion.
/// Replacing the fixture with bytes encoding a different seed would fail both.
#[test]
fn fixture_v1_forward_migrates() {
    let bytes = std::fs::read(v1_sample_path())
        .expect("read v1_sample.fwsave — run `regenerate_fixtures` (--ignored) to bootstrap");
    let loaded = load_envelope(&bytes)
        .expect("v1_sample.fwsave must load via the V1→V2→V3→V4 migration chain without error");

    assert_eq!(
        loaded.career_seed.to_u64(),
        V1_SAMPLE_SEED,
        "forward-migration: career_seed from committed v1 fixture must match documented value"
    );
    assert_eq!(
        loaded.content_pack_version, V1_SAMPLE_CONTENT_PACK_VERSION,
        "forward-migration: content_pack_version from committed v1 fixture must match documented value"
    );
    assert!(
        loaded.ledger.is_empty(),
        "forward-migration: V1 placeholder ledger must migrate to an empty ledger"
    );
    assert!(
        loaded.roster.is_empty(),
        "forward-migration: V1 save predates roster persistence; roster must be empty"
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
/// Non-vacuousness: this is a separate test from `fixture_v1_forward_migrates`
/// so a future refactor that forgets to assert `content_pack_version` in the
/// forward-migration test doesn't leave callback-preservation unchecked.
/// Mutating `migrate_v1_to_v2` to drop or transform any V1 field must fail
/// at least one of these two assertions.
#[test]
fn fixture_v1_all_fields_preserved() {
    let bytes = std::fs::read(v1_sample_path())
        .expect("read v1_sample.fwsave — run `regenerate_fixtures` (--ignored) to bootstrap");
    let loaded = load_envelope(&bytes)
        .expect("v1_sample.fwsave must load cleanly for callback-preservation check");

    // career_seed: all 64 bits preserved
    assert_eq!(
        loaded.career_seed.to_u64(),
        V1_SAMPLE_SEED,
        "callback-preservation: career_seed must be BIT-EXACT across the V1→V3 chain (all 64 bits)"
    );

    // content_pack_version: all 32 bits preserved
    assert_eq!(
        loaded.content_pack_version, V1_SAMPLE_CONTENT_PACK_VERSION,
        "callback-preservation: content_pack_version must be BIT-EXACT across the V1→V3 chain (all 32 bits)"
    );

    // ledger: V1 placeholder events → empty ledger (no data dropped, because
    // V1 placeholder events had no real semantics — the empty mapping IS
    // preservation of the intent).
    assert!(
        loaded.ledger.is_empty(),
        "callback-preservation: placeholder ledger maps to an empty ledger (no data lost)"
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
// AC6: V0→V1→V2→V3 full chain (T3-7 extension beyond CLAUDE.md §9 core four)
// -------------------------------------------------------------------------

/// Loading the committed `v0_sample.fwsave` via `load_envelope` traverses the
/// full V0→V1→V2→V3 chain and produces a current-schema payload with the
/// documented seed.
///
/// V0→V1 defaults `content_pack_version = 1` and `ledger = empty`.
/// V1→V2 and V2→V3 preserve those defaults. The final payload must reflect
/// every hop.
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

    let loaded = load_envelope(&bytes)
        .expect("v0_sample.fwsave must load via the V0→V1→V2→V3 chain without error");

    assert_eq!(
        loaded.career_seed.to_u64(),
        V0_SAMPLE_SEED,
        "full-chain: career_seed from committed v0 fixture must match documented value"
    );
    // V0→V1 migration defaults content_pack_version to 1.
    assert_eq!(
        loaded.content_pack_version, 1,
        "full-chain: V0→V1 migration must default content_pack_version to 1"
    );
    assert!(
        loaded.ledger.is_empty(),
        "full-chain: the V0→V1→V2→V3 chain must produce an empty ledger"
    );
}

// -------------------------------------------------------------------------
// AC7: non-empty V2 ledger fixture (T3-R-C — Codex E6 frozen-fixture gap)
// -------------------------------------------------------------------------

/// `decode` on the committed `v2_nonempty_ledger_sample.fwsave` yields a
/// `SaveEnvelope::V2` whose ledger carries the frozen rows — 2 plain
/// `MemoryEvent`s + exactly 1 `Compaction`. This is the only frozen fixture
/// with a NON-EMPTY ledger; the v0/v1 fixtures carry empty ledgers and so
/// cannot catch a `MemoryEvent` serde regression.
///
/// Non-vacuousness: replacing the fixture with an empty-ledger save fails the
/// `ledger.len()` assertion; a fixture missing the `Compaction` row fails the
/// compaction-count assertion; a fixture encoding a different seed fails the
/// `career_seed` assertion.
#[test]
fn fixture_v2_nonempty_ledger_decodes() {
    let bytes = std::fs::read(v2_nonempty_path()).expect(
        "read v2_nonempty_ledger_sample.fwsave — run `regenerate_fixtures` (--ignored) to bootstrap",
    );

    assert_eq!(
        bytes[0], 0x02,
        "v2_nonempty_ledger_sample.fwsave first byte must be 0x02 (V2 tag) — wrong fixture loaded?"
    );

    let env =
        decode(&bytes).expect("decode of committed v2_nonempty_ledger_sample.fwsave must succeed");
    let SaveEnvelope::V2(v2) = env else {
        panic!("v2_nonempty_ledger_sample.fwsave must decode as SaveEnvelope::V2, got {env:?}");
    };

    assert_eq!(
        v2.career_seed.to_u64(),
        V2_NONEMPTY_SEED,
        "decoded V2 career_seed must match the documented fixture value"
    );
    assert_eq!(
        v2.ledger.len(),
        V2_NONEMPTY_LEDGER_LEN,
        "frozen V2 ledger must carry {V2_NONEMPTY_LEDGER_LEN} rows"
    );

    let compaction_count = v2
        .ledger
        .iter()
        .filter(|e| matches!(e.event_class, EventClass::Compaction))
        .count();
    assert_eq!(
        compaction_count, 1,
        "frozen V2 ledger must carry exactly 1 Compaction event"
    );
    let plain_count = v2
        .ledger
        .iter()
        .filter(|e| !matches!(e.event_class, EventClass::Compaction))
        .count();
    assert_eq!(
        plain_count, 2,
        "frozen V2 ledger must carry 2 plain (non-Compaction) MemoryEvents"
    );
}

/// `encode(decode(committed_v2_bytes))` is byte-identical to the committed
/// bytes — the non-empty-ledger analogue of `fixture_v1_round_trip_byte_identical`.
/// A `MemoryEvent` serde regression (a field reorder, an `EventClass`
/// discriminant shift, a varint-encoding change) produces different bytes
/// here where the empty-ledger fixtures stay silent.
///
/// Non-vacuousness: mutating `encode` to append or drop any byte fails the
/// length-and-content `assert_eq!`; the frozen non-empty ledger means the
/// `MemoryEvent` rows are genuinely re-encoded, not skipped.
#[test]
fn fixture_v2_nonempty_round_trip_byte_identical() {
    let bytes = std::fs::read(v2_nonempty_path()).expect(
        "read v2_nonempty_ledger_sample.fwsave — run `regenerate_fixtures` (--ignored) to bootstrap",
    );

    let envelope = decode(&bytes)
        .expect("decode of committed v2_nonempty_ledger_sample.fwsave must succeed for round-trip");
    let re_encoded = encode(&envelope)
        .expect("re-encode of decoded v2_nonempty_ledger_sample.fwsave envelope must succeed");

    assert_eq!(
        bytes, re_encoded,
        "round-trip-byte-identical: encode(decode(committed_v2_bytes)) must equal committed_v2_bytes"
    );
}

/// `load_envelope` on the committed `v2_nonempty_ledger_sample.fwsave` migrates
/// the V2 payload up to the current schema (V2→V3 carries the ledger through
/// unchanged) and `restore_transient_state` rebuilds the transient `next_id`
/// from the 3 frozen rows, so the next appended event takes `EventId(3)`.
///
/// Non-vacuousness: if `load_envelope` skipped `restore_transient_state`,
/// `next_id` would stay at its `#[serde(skip)]` default of 0 and the new
/// append would collide at `EventId(0)`, failing the assertion.
#[test]
fn fixture_v2_nonempty_loads_and_restores_transient_state() {
    let bytes = std::fs::read(v2_nonempty_path()).expect(
        "read v2_nonempty_ledger_sample.fwsave — run `regenerate_fixtures` (--ignored) to bootstrap",
    );

    let mut loaded = load_envelope(&bytes)
        .expect("v2_nonempty_ledger_sample.fwsave must load via load_envelope without error");

    assert_eq!(
        loaded.ledger.len(),
        V2_NONEMPTY_LEDGER_LEN,
        "the loaded ledger must carry {V2_NONEMPTY_LEDGER_LEN} rows"
    );

    // restore_transient_state (run by load_envelope) rebuilt next_id from the
    // 3 frozen rows — the next append must take EventId(3), not EventId(0).
    let new_id = loaded
        .ledger
        .append(sample_memory_event(6, EventClass::LegacyGoal));
    assert_eq!(
        new_id,
        EventId(3),
        "load_envelope must restore next_id so a post-load append continues the EventId sequence"
    );
}

// -------------------------------------------------------------------------
// AC8: frozen V3 career fixture (T3-R-E — career-state persistence)
// -------------------------------------------------------------------------

/// `decode` on the committed `v3_career_sample.fwsave` yields a
/// `SaveEnvelope::V3` carrying the documented `season_number`, a `Some`
/// season snapshot, and a 2-row ledger. This is the only frozen fixture that
/// exercises the V3 `Option<SeasonState>` serde surface — the v0/v1/v2
/// fixtures predate the season field.
///
/// Non-vacuousness: a fixture with `season: None` fails the `season.is_some()`
/// assertion; a fixture encoding a different season_number or seed fails those
/// assertions; an empty-ledger fixture fails the `ledger.len()` assertion.
#[test]
fn fixture_v3_career_decodes() {
    let bytes = std::fs::read(v3_career_path()).expect(
        "read v3_career_sample.fwsave — run `regenerate_fixtures` (--ignored) to bootstrap",
    );

    assert_eq!(
        bytes[0], 0x03,
        "v3_career_sample.fwsave first byte must be 0x03 (V3 tag) — wrong fixture loaded?"
    );

    let env = decode(&bytes).expect("decode of committed v3_career_sample.fwsave must succeed");
    let SaveEnvelope::V3(v3) = env else {
        panic!("v3_career_sample.fwsave must decode as SaveEnvelope::V3, got {env:?}");
    };

    assert_eq!(
        v3.career_seed.to_u64(),
        V3_CAREER_SEED,
        "decoded V3 career_seed must match the documented fixture value"
    );
    assert_eq!(
        v3.season_number,
        SeasonNumber(V3_CAREER_SEASON_NUMBER),
        "decoded V3 season_number must match the documented fixture value"
    );
    assert!(
        v3.season.is_some(),
        "the frozen V3 fixture must carry a Some(SeasonState) snapshot"
    );
    assert_eq!(v3.ledger.len(), 2, "the frozen V3 ledger must carry 2 rows");
}

/// `encode(decode(committed_v3_bytes))` is byte-identical to the committed
/// bytes. A `SeasonState` serde regression (a field reorder in the league /
/// standings types) produces different bytes here where the season-less
/// v0/v1/v2 fixtures stay silent.
#[test]
fn fixture_v3_career_round_trip_byte_identical() {
    let bytes = std::fs::read(v3_career_path()).expect(
        "read v3_career_sample.fwsave — run `regenerate_fixtures` (--ignored) to bootstrap",
    );

    let envelope =
        decode(&bytes).expect("decode of committed v3_career_sample.fwsave must succeed");
    let re_encoded =
        encode(&envelope).expect("re-encode of decoded v3_career_sample.fwsave must succeed");

    assert_eq!(
        bytes, re_encoded,
        "round-trip-byte-identical: encode(decode(committed_v3_bytes)) must equal committed_v3_bytes"
    );
}

/// `load_envelope` on the committed `v3_career_sample.fwsave` resumes the
/// career at the saved season — `season_number` + the `Some(SeasonState)`
/// snapshot + the ledger all survive a real frozen-file load. Now migrates to
/// V4 (roster = empty, watermark = 0, all V3 fields preserved).
#[test]
fn fixture_v3_career_resumes_at_correct_season() {
    let bytes = std::fs::read(v3_career_path()).expect(
        "read v3_career_sample.fwsave — run `regenerate_fixtures` (--ignored) to bootstrap",
    );

    // load_envelope returns SaveV4 (V3 → migrate_v3_to_v4).
    let v4 = load_envelope(&bytes).expect("v3_career_sample.fwsave must load via load_envelope");

    assert_eq!(
        v4.season_number,
        SeasonNumber(V3_CAREER_SEASON_NUMBER),
        "the frozen V3 career must resume at the saved season number (migrated to V4)"
    );
    assert!(
        v4.season.is_some(),
        "the frozen V3 season snapshot must survive a V3→V4 migration + load_envelope round-trip"
    );
    assert_eq!(
        v4.career_seed.to_u64(),
        V3_CAREER_SEED,
        "career_seed intact on V3→V4 migration"
    );
    assert_eq!(
        v4.ledger.len(),
        2,
        "the ledger is intact on V3→V4 migration"
    );
    assert!(
        v4.roster.is_empty(),
        "roster must be empty on a V3→V4 migration (no deltas in V3 save)"
    );
    assert_eq!(
        v4.breakthrough_eval_watermark, 0,
        "watermark must be 0 on a V3→V4 migration"
    );
}

// -------------------------------------------------------------------------
// AC9: frozen V4 career fixture (T4-2.5g — mutable-subset roster)
// -------------------------------------------------------------------------

/// `decode` on the committed `v4_career_sample.fwsave` yields a
/// `SaveEnvelope::V4` carrying the documented `season_number`, a `Some`
/// season snapshot, a non-empty roster, and the correct watermark.
///
/// Non-vacuousness: an empty-roster fixture fails the `roster.len()` assertion;
/// a fixture encoding a different sentinel `career_apps` fails the delta
/// assertion; a different seed or season_number fail those assertions.
#[test]
fn fixture_v4_career_decodes() {
    let bytes = std::fs::read(v4_career_path()).expect(
        "read v4_career_sample.fwsave — run `regenerate_fixtures` (--ignored) to bootstrap",
    );

    assert_eq!(
        bytes[0], 0x04,
        "v4_career_sample.fwsave first byte must be 0x04 (V4 tag) — wrong fixture loaded?"
    );

    let env = decode(&bytes).expect("decode of committed v4_career_sample.fwsave must succeed");
    let SaveEnvelope::V4(v4) = env else {
        panic!("v4_career_sample.fwsave must decode as SaveEnvelope::V4, got {env:?}");
    };

    assert_eq!(
        v4.career_seed.to_u64(),
        V4_CAREER_SEED,
        "decoded V4 career_seed must match the documented fixture value"
    );
    assert_eq!(
        v4.season_number,
        SeasonNumber(V4_CAREER_SEASON_NUMBER),
        "decoded V4 season_number must match the documented fixture value"
    );
    assert!(
        v4.season.is_some(),
        "the frozen V4 fixture must carry a Some(SeasonState) snapshot"
    );
    assert_eq!(v4.ledger.len(), 2, "the frozen V4 ledger must carry 2 rows");
    assert_eq!(
        v4.breakthrough_eval_watermark, V4_CAREER_WATERMARK,
        "decoded V4 breakthrough_eval_watermark must match documented fixture value"
    );

    // Roster: 1 club with V4_ROSTER_INSTANCE_COUNT instances.
    let total_instances: usize = v4.roster.values().map(|v| v.len()).sum();
    assert_eq!(
        total_instances, V4_ROSTER_INSTANCE_COUNT,
        "frozen V4 roster must carry {V4_ROSTER_INSTANCE_COUNT} instances"
    );

    // Sentinel: the first instance in the only club must have V4_SENTINEL_CAREER_APPS.
    let first_instance = v4
        .roster
        .values()
        .next()
        .and_then(|v| v.first())
        .expect("roster must have at least one instance");
    assert_eq!(
        first_instance.career_apps, V4_SENTINEL_CAREER_APPS,
        "first roster instance career_apps must equal the documented sentinel value"
    );
}

/// `encode(decode(committed_v4_bytes))` is byte-identical to the committed
/// bytes. A `SavedPlayerInstance` serde regression (a field reorder, a varint
/// change) produces different bytes here.
#[test]
fn fixture_v4_career_round_trip_byte_identical() {
    let bytes = std::fs::read(v4_career_path()).expect(
        "read v4_career_sample.fwsave — run `regenerate_fixtures` (--ignored) to bootstrap",
    );

    let envelope =
        decode(&bytes).expect("decode of committed v4_career_sample.fwsave must succeed");
    let re_encoded =
        encode(&envelope).expect("re-encode of decoded v4_career_sample.fwsave must succeed");

    assert_eq!(
        bytes, re_encoded,
        "round-trip-byte-identical: encode(decode(committed_v4_bytes)) must equal committed_v4_bytes"
    );
}

/// `load_envelope` on the committed `v4_career_sample.fwsave` resumes the
/// career with the full roster intact — `season_number`, `Some(SeasonState)`,
/// ledger, non-empty roster, and watermark all survive.
#[test]
fn fixture_v4_career_resumes() {
    let bytes = std::fs::read(v4_career_path()).expect(
        "read v4_career_sample.fwsave — run `regenerate_fixtures` (--ignored) to bootstrap",
    );

    let v4 = load_envelope(&bytes).expect("v4_career_sample.fwsave must load via load_envelope");

    assert_eq!(
        v4.season_number,
        SeasonNumber(V4_CAREER_SEASON_NUMBER),
        "the frozen V4 career must resume at the saved season number"
    );
    assert!(
        v4.season.is_some(),
        "the frozen V4 season snapshot must survive a load_envelope round-trip"
    );
    assert_eq!(
        v4.career_seed.to_u64(),
        V4_CAREER_SEED,
        "career_seed intact on resume"
    );
    assert_eq!(v4.ledger.len(), 2, "ledger intact on resume");
    assert_eq!(
        v4.breakthrough_eval_watermark, V4_CAREER_WATERMARK,
        "watermark intact on resume"
    );

    let total_instances: usize = v4.roster.values().map(|v| v.len()).sum();
    assert_eq!(
        total_instances, V4_ROSTER_INSTANCE_COUNT,
        "roster count intact on resume"
    );
}

/// `load_envelope` on the committed `v3_career_sample.fwsave` migrates forward
/// to V4. The V3 fields (`career_seed`, `content_pack_version`, `ledger`,
/// `season_number`, `season`) survive unchanged; `roster` is empty and
/// `breakthrough_eval_watermark` is 0.
///
/// This is the `fixture_v3_forward_migrates_to_v4` test — the "AC1
/// forward-migration" test for the V3→V4 hop, distinct from the
/// `fixture_v3_career_resumes_at_correct_season` test above (which checks the
/// full load_envelope resume path, not just the migration correctness).
#[test]
fn fixture_v3_forward_migrates_to_v4() {
    let bytes = std::fs::read(v3_career_path()).expect(
        "read v3_career_sample.fwsave — run `regenerate_fixtures` (--ignored) to bootstrap",
    );

    let v4 = load_envelope(&bytes)
        .expect("v3_career_sample.fwsave must load via V3→V4 migration chain without error");

    // V3 fields must survive migration unchanged.
    assert_eq!(
        v4.career_seed.to_u64(),
        V3_CAREER_SEED,
        "forward-migration V3→V4: career_seed must be preserved"
    );
    assert_eq!(
        v4.content_pack_version, 1,
        "forward-migration V3→V4: content_pack_version must be preserved"
    );
    assert_eq!(
        v4.season_number,
        SeasonNumber(V3_CAREER_SEASON_NUMBER),
        "forward-migration V3→V4: season_number must be preserved"
    );
    assert!(
        v4.season.is_some(),
        "forward-migration V3→V4: Some(SeasonState) must survive migration"
    );
    assert_eq!(
        v4.ledger.len(),
        2,
        "forward-migration V3→V4: 2-event ledger must survive migration"
    );

    // V4-only defaults.
    assert!(
        v4.roster.is_empty(),
        "forward-migration V3→V4: roster must be empty (V3 has no roster data)"
    );
    assert_eq!(
        v4.breakthrough_eval_watermark, 0,
        "forward-migration V3→V4: watermark must be 0 (V3 never had one)"
    );
}

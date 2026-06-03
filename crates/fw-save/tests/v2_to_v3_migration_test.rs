//! T3-R-E — V2 → V3 forward-migration discipline + the V3 career-resume test.
//!
//! ## Four-test migration discipline (CLAUDE.md §9)
//!
//!   1. forward-migration         — V2 bytes load as V3 via `load_envelope`
//!   2. callback-preservation     — every V2 field maps to V3 (no silent drops)
//!   3. forward-incompat-failure  — covered by the shared V99 reject test
//!      `lib.rs::migration::load_envelope_rejects_unsupported_future_version`
//!      (discriminant 99 is unknown to a V0..=V3 loader; no duplicate needed —
//!      the same precedent T3-1 set for the V2 bump)
//!   4. round-trip-byte-identical — `encode(decode(V3 bytes)) ≡ V3 bytes`
//!
//! Plus the T3-R-E acceptance test: a career resumes from a V3 save at the
//! correct season with the ledger intact.

use std::collections::BTreeMap;
use std::path::PathBuf;

use fw_content::{ContentStore, SeasonState, generate_league};
use fw_core::{MatchId, PlayerId, Q32, Seed, Tick};
use fw_memory::{
    CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind, Emotion,
    EntityRef, EventClass, EventId, MemoryEvent, MemoryLedger, Participant, ParticipantRole,
    SeasonNumber, SourceId,
};
use fw_save::{SaveEnvelope, SaveV2, SaveV3, decode, encode, load_envelope, migrate_v2_to_v3};

// -------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------

/// Workspace-root `content/` directory (this crate is at `crates/fw-save/`).
fn content_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

/// A deterministic plain `MemoryEvent` for building a non-empty test ledger.
fn sample_event(season: u16, class: EventClass) -> MemoryEvent {
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
        event_class: class,
        stakes: Q32::ZERO,
        emotion: Emotion::Neutral,
        consequence: vec![Consequence::None],
        callback_eligibility: CallbackEligibility::Immediate,
        salience: Q32::ZERO,
        decay_function: DecayFunction::Never,
    }
}

/// A two-event ledger — non-empty so migration field-preservation is genuine.
fn ledger_with_two_events() -> MemoryLedger {
    let mut ledger = MemoryLedger::new();
    ledger.append(sample_event(0, EventClass::DebutSenior));
    ledger.append(sample_event(1, EventClass::LegacyGoal));
    ledger
}

/// Build a real `SeasonState` from the shipped content pack — exercises the
/// `Some(SeasonState)` serde surface that `Option<SeasonState>` adds in V3.
fn real_season() -> SeasonState {
    let content = ContentStore::load_sources(&content_root())
        .expect("load content/ for SeasonState construction");
    let league = generate_league(Seed::from_u64(0xCAFE_F00D), &content)
        .expect("generate_league must succeed against the shipped pack");
    SeasonState::new(league, &content)
}

// -------------------------------------------------------------------------
// 1. forward-migration
// -------------------------------------------------------------------------

/// V2 bytes loaded via `load_envelope` emerge as a `SaveV4` (V2→V3→V4 chain)
/// with the V2 fields preserved + the documented migration defaults:
/// `season_number = 0`, `season = None`, `roster = {}`, `watermark = 0`.
///
/// (T4-2.5g: `load_envelope` now returns `SaveV4`; the `breakthrough_states`
/// field from `SaveV3` is superseded by `roster` in `SaveV4`.)
#[test]
fn v2_bytes_load_as_v4_via_load_envelope() {
    let seed = Seed::from_u64(0x1234_5678_9ABC_DEF0);
    let bytes = encode(&SaveEnvelope::V2(SaveV2 {
        career_seed: seed,
        content_pack_version: 7,
        ledger: ledger_with_two_events(),
    }))
    .expect("encode v2");

    // load_envelope returns SaveV4 since T4-2.5g.
    let v4 = load_envelope(&bytes).expect("V2 bytes must load as V4 via load_envelope");

    // V2 fields preserved through the V2→V3→V4 chain.
    assert_eq!(v4.career_seed, seed, "career_seed must survive V2→V4 chain");
    assert_eq!(
        v4.content_pack_version, 7,
        "content_pack_version must survive"
    );
    assert_eq!(v4.ledger.len(), 2, "the ledger must survive V2→V4 intact");
    // V4 defaults for a promoted V2 save.
    assert_eq!(
        v4.season_number,
        SeasonNumber(0),
        "a promoted V2 save defaults season_number to 0"
    );
    assert!(
        v4.season.is_none(),
        "a promoted V2 save has no persisted season"
    );
    assert!(
        v4.roster.is_empty(),
        "a promoted V2 save has no roster deltas (roster is empty)"
    );
    assert_eq!(
        v4.breakthrough_eval_watermark, 0,
        "a promoted V2 save defaults breakthrough_eval_watermark to 0"
    );
}

// -------------------------------------------------------------------------
// 2. callback-preservation
// -------------------------------------------------------------------------

/// Every `SaveV2` field maps to `SaveV3` with no silent drops: `career_seed`
/// bit-exact, `content_pack_version` exact, `ledger` intact. The three new V3
/// fields take their documented defaults.
#[test]
fn v2_to_v3_all_fields_accounted_for_callback_preservation() {
    let seed_raw: u64 = 0xABCD_1234_5678_9F0E;
    let v3 = migrate_v2_to_v3(SaveV2 {
        career_seed: Seed::from_u64(seed_raw),
        content_pack_version: 42,
        ledger: ledger_with_two_events(),
    });

    assert_eq!(
        v3.career_seed.to_u64(),
        seed_raw,
        "career_seed must reach V3 BIT-EXACT (callback-preservation)"
    );
    assert_eq!(
        v3.content_pack_version, 42,
        "content_pack_version must survive V2→V3 exact"
    );
    assert_eq!(v3.ledger.len(), 2, "ledger must survive V2→V3 intact");
    assert_eq!(
        v3.season_number,
        SeasonNumber(0),
        "V3 default: season_number = 0"
    );
    assert!(v3.season.is_none(), "V3 default: season = None");
    assert!(
        v3.breakthrough_states.is_empty(),
        "V3 default: breakthrough_states empty"
    );
}

// -------------------------------------------------------------------------
// 4. round-trip-byte-identical
// -------------------------------------------------------------------------

/// `encode(decode(encode(V3(x))))` is byte-identical to `encode(V3(x))` for a
/// populated career — a non-empty ledger AND a `Some(SeasonState)`, so the new
/// V3 serde surface is genuinely exercised, not skipped.
#[test]
fn v3_round_trip_byte_identical_with_populated_career() {
    let env = SaveEnvelope::V3(SaveV3 {
        career_seed: Seed::from_u64(0xAB_CD_EF_01_23_45_67_89),
        content_pack_version: 1,
        ledger: ledger_with_two_events(),
        season_number: SeasonNumber(3),
        season: Some(real_season()),
        breakthrough_states: BTreeMap::new(),
    });
    let bytes_1 = encode(&env).expect("encode 1");
    let decoded = decode(&bytes_1).expect("decode");
    let bytes_2 = encode(&decoded).expect("encode 2");
    assert_eq!(
        bytes_1, bytes_2,
        "encode(decode(encode(V3(x)))) must be byte-identical"
    );
}

// -------------------------------------------------------------------------
// T3-R-E acceptance — a career resumes from a V3 save at the correct season
// -------------------------------------------------------------------------

/// A V3 career save carrying `season_number = 3`, a persisted
/// `Some(SeasonState)`, and a non-empty ledger — encoded then loaded via
/// `load_envelope` — resumes at the correct season with the season snapshot
/// and ledger intact (migrated to V4 via V3→V4 chain).
#[test]
fn v3_career_resumes_at_correct_season() {
    let seed = Seed::from_u64(0x7E50_07E5_07E5_07E5);
    let bytes = encode(&SaveEnvelope::V3(SaveV3 {
        career_seed: seed,
        content_pack_version: 1,
        ledger: ledger_with_two_events(),
        season_number: SeasonNumber(3),
        season: Some(real_season()),
        breakthrough_states: BTreeMap::new(),
    }))
    .expect("encode v3");

    // load_envelope returns SaveV4 since T4-2.5g (V3→V4 migration).
    let v4 = load_envelope(&bytes).expect("V3 bytes must load via load_envelope (→ SaveV4)");

    assert_eq!(
        v4.season_number,
        SeasonNumber(3),
        "the career resumes at the saved season number"
    );
    assert!(
        v4.season.is_some(),
        "the persisted season snapshot survives the V3→V4 migration"
    );
    assert_eq!(v4.career_seed, seed, "career_seed intact");
    assert_eq!(v4.ledger.len(), 2, "the ledger is intact on resume");
    assert!(
        v4.roster.is_empty(),
        "roster is empty on a V3→V4 migration (no prior roster data)"
    );
}

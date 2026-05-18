//! T2-R7(e) + T3-1 update — SaveV2 encode → decode → re-encode byte identity
//! across seed-space + ledger-shape variation.
//!
//! Updated at T3-1: `MemoryEvent::Placeholder` removed; replaced with a
//! minimal real `MemoryEvent` using the ADR-0005 schema. The V1 round-trip
//! test is preserved for V1 coverage (now an empty-ledger test since V1
//! accepts empty ledgers); the V2 test exercises the full event schema.
//!
//! Two invariants this proptest defends:
//!
//! 1. **Round-trip byte identity.** `encode(decode(encode(env)))` ==
//!    `encode(env)` for every `(career_seed, content_pack_version,
//!    ledger)` triple the strategy generates.
//!
//! 2. **Decode-equality.** `decode(encode(env)) == env` for every case.
//!
//! Cross-reference: post-T2 ultimate-review doc at
//! `docs/audits/post-t2-ultimate-review-2026-05-18.md` Track D-Survey-3.

use fw_core::{MatchId, PlayerId, Q32, Seed, Tick};
use fw_memory::{
    CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind, Emotion,
    EntityRef, EventClass, EventId, MemoryEvent, MemoryLedger, Participant, ParticipantRole,
    SeasonNumber, SourceId,
};
use fw_save::{SaveEnvelope, SaveV1, SaveV2, decode, encode};
use proptest::prelude::*;

/// Build a minimal `MemoryEvent` from raw parts. Uses `DebutSenior` as the
/// canonical simple event class for proptest coverage.
fn minimal_event_from(player_raw: u32, season_raw: u16, match_raw: u32) -> MemoryEvent {
    MemoryEvent {
        event_id: EventId(0), // overwritten by append; fine for direct Vec building
        schema_version: 1,
        season: SeasonNumber(season_raw),
        tick: Some(Tick::ZERO),
        career_date: CareerDate {
            year: 1,
            day_of_year: 1,
        },
        emitter: Emitter {
            kind: EmitterKind::MatchEngine,
            source_id: SourceId::Match(MatchId::new(match_raw)),
        },
        participants: vec![Participant {
            role: ParticipantRole::Subject,
            entity: EntityRef::Player(PlayerId::new(player_raw)),
        }],
        event_class: EventClass::DebutSenior,
        stakes: Q32::ZERO,
        emotion: Emotion::Pride,
        consequence: vec![Consequence::None],
        callback_eligibility: CallbackEligibility::Immediate,
        salience: Q32::ZERO,
        decay_function: DecayFunction::Never,
    }
}

/// Build a `MemoryLedger` via direct `events.push` (bypassing `append` so we
/// can control event_id for deterministic serde output). Each event gets its
/// position as event_id.
fn ledger_from(events: Vec<(u32, u16, u32)>) -> MemoryLedger {
    let mut l = MemoryLedger::new();
    for (player, season, match_id) in events {
        l.append(minimal_event_from(player, season, match_id));
    }
    l
}

/// Strategy generating an arbitrary `SaveV1` payload. V1 ledgers are always
/// empty (V1 placeholder events no longer constructible post-T3-1). Kept to
/// verify V1 encode/decode round-trip still works after V2 addition.
fn arb_save_v1() -> impl Strategy<Value = SaveV1> {
    (any::<u64>(), any::<u32>()).prop_map(|(seed_raw, pack_ver)| SaveV1 {
        career_seed: Seed::from_u64(seed_raw),
        content_pack_version: pack_ver,
        ledger: MemoryLedger::new(),
    })
}

/// Strategy generating an arbitrary `SaveV2` payload with 0..=8 real events.
fn arb_save_v2() -> impl Strategy<Value = SaveV2> {
    (
        any::<u64>(),
        any::<u32>(),
        // 0..=8 events; each is (player_id, season, match_id)
        prop::collection::vec((any::<u32>(), any::<u16>(), any::<u32>()), 0..=8),
    )
        .prop_map(|(seed_raw, pack_ver, events)| SaveV2 {
            career_seed: Seed::from_u64(seed_raw),
            content_pack_version: pack_ver,
            ledger: ledger_from(events),
        })
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        ..ProptestConfig::default()
    })]

    /// V1 round-trip byte identity (empty ledger — V1 schema is locked at
    /// T2-9; real events are V2 now).
    #[test]
    fn v1_round_trip_byte_identical_across_payloads(v1 in arb_save_v1()) {
        let env = SaveEnvelope::V1(v1.clone());

        let bytes_1 = encode(&env).expect("encode #1 must succeed");
        let restored = decode(&bytes_1).expect("decode must succeed");
        prop_assert_eq!(
            restored.clone(),
            env.clone(),
            "decode(encode(env)) must equal env"
        );
        let bytes_2 = encode(&restored).expect("encode #2 must succeed");
        prop_assert_eq!(
            bytes_1,
            bytes_2,
            "encode(decode(encode(env))) must be byte-identical"
        );
    }

    /// V2 round-trip byte identity across the full (seed, pack_ver, events)
    /// surface. This is the primary regression guard for the ADR-0005 schema.
    #[test]
    fn v2_round_trip_byte_identical_across_payloads(v2 in arb_save_v2()) {
        let env = SaveEnvelope::V2(v2.clone());

        let bytes_1 = encode(&env).expect("encode #1 must succeed");
        let restored = decode(&bytes_1).expect("decode must succeed");
        prop_assert_eq!(
            restored.clone(),
            env.clone(),
            "decode(encode(env)) must equal env"
        );
        let bytes_2 = encode(&restored).expect("encode #2 must succeed");
        prop_assert_eq!(
            bytes_1,
            bytes_2,
            "encode(decode(encode(env))) must be byte-identical"
        );
    }
}

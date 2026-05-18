//! 1000-event round-trip perf test.
//!
//! `#[ignore]`-gated — run explicitly via:
//!
//!   cargo test --release -p fw-save --test perf_test -- --ignored thousand_event_round_trip_under_100ms
//!
//! **Release-mode-only contract.** The 100ms wall-clock budget assumes
//! `--release`; debug builds are 10-30× slower and are not bound by this
//! test. The `#[ignore]` gate prevents CI from running it in the default
//! `cargo test` sweep; CI runs `cargo test --release --workspace` which also
//! skips `#[ignore]`-gated tests (they require `-- --ignored` to be
//! included).
//!
//! The single use of `std::time::Instant::now()` here is the documented
//! exception to `Sim/RULES.md §3` (no clocks in sim/content/memory crates).
//! `fw-save` tests are non-sim, non-canonical-state; measuring wall-clock
//! elapsed is the explicit purpose of this test.
//!
//! AC6 target: encode + decode of a 1000-event `SaveV2` in < 100ms.

#[ignore = "perf test — run with --release and -- --ignored"]
#[test]
fn thousand_event_round_trip_under_100ms() {
    use fw_core::{MatchId, PlayerId, Q32, Seed, Tick};
    use fw_memory::{
        CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind, Emotion,
        EntityRef, EventClass, EventId, MemoryEvent, MemoryLedger, Participant, ParticipantRole,
        SeasonNumber, SourceId,
    };
    use fw_save::{SaveEnvelope, SaveV2, encode, load_envelope};
    use std::time::{Duration, Instant};

    // Build a 1000-event ledger.
    let mut ledger = MemoryLedger::new();
    // Rotate through a handful of EventClass variants to exercise codec
    // paths for multiple variants.
    let classes = [
        EventClass::DebutSenior,
        EventClass::LegacyGoal,
        EventClass::HatTrickScored,
        EventClass::CupFinalWin,
        EventClass::TitleWon,
        EventClass::InjuryLongTerm,
        EventClass::RivalryFormed,
        EventClass::BrokenPromise,
    ];

    for i in 0u32..1000 {
        let class = classes[(i as usize) % classes.len()].clone();
        ledger.append(MemoryEvent {
            event_id: EventId(0), // overwritten by append
            schema_version: 1,
            season: SeasonNumber((i / 38) as u16),
            tick: Some(Tick::ZERO),
            career_date: CareerDate {
                year: 1 + (i / 365) as u16,
                day_of_year: 1 + (i % 365) as u16,
            },
            emitter: Emitter {
                kind: EmitterKind::MatchEngine,
                source_id: SourceId::Match(MatchId::new(i)),
            },
            participants: vec![Participant {
                role: ParticipantRole::Subject,
                entity: EntityRef::Player(PlayerId::new(i % 22)),
            }],
            event_class: class,
            stakes: Q32::ZERO,
            emotion: Emotion::Neutral,
            consequence: vec![Consequence::None],
            callback_eligibility: CallbackEligibility::Immediate,
            salience: Q32::ZERO,
            decay_function: DecayFunction::Never,
        });
    }

    assert_eq!(
        ledger.len(),
        1000,
        "ledger must have 1000 events before timing"
    );

    let env = SaveEnvelope::V2(SaveV2 {
        career_seed: Seed::from_u64(0xC0DE_CAFE_FEED_BEEF),
        content_pack_version: 1,
        ledger,
    });

    // --- Timed section starts here ---
    let start = Instant::now();

    let bytes = encode(&env).expect("encode 1000-event SaveV2");
    let loaded = load_envelope(&bytes).expect("load_envelope 1000-event SaveV2");

    let elapsed = start.elapsed();
    // --- Timed section ends ---

    // Correctness check: loaded ledger has 1000 events.
    assert_eq!(
        loaded.ledger.len(),
        1000,
        "decoded ledger must have 1000 events"
    );

    assert!(
        elapsed < Duration::from_millis(100),
        "1000-event encode+decode must complete in <100ms on release build; \
         elapsed: {elapsed:?}. If this fires in debug mode you ran the test \
         without --release — see file header."
    );
}

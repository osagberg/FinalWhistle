//! Integration tests for the five memory-ledger readers.
//!
//! Uses a shared 3-season seeded ledger fixture (~30 events spanning seasons
//! 0-2, multiple clubs and players, mixed event classes and decay functions).
//! Each of the five readers is queried against this fixture and the result
//! shape + non-empty invariants are verified.
//!
//! ## Fixture design
//!
//! The fixture models a realistic career arc:
//! - Season 0: player debuts, signs contract, scores a legacy goal.
//! - Season 1: cup final win, title won, a derby controversy, a rival formed.
//! - Season 2: breakthrough, hat-trick, sold under protest, long-term injury.
//!
//! Players: p1 (main subject), p2 (rival / mentor), p3 (club B).
//! Clubs: club_a (p1 + p2's primary club), club_b (p3's club).

use fw_core::{ClubId, MatchId, PlayerId, Q32, Tick};
use fw_memory::readers::{
    SalienceFilter, coach::CoachReader, fan::FanReader, press::PressReader,
    salience::SalienceReader, scout::ScoutReader,
};
use fw_memory::{
    CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind, Emotion,
    EntityRef, EventClass, EventId, MemoryEvent, MemoryLedger, Participant, ParticipantRole,
    PressTopic, SeasonNumber, SourceId,
};

// -------------------------------------------------------------------------
// Fixture IDs
// -------------------------------------------------------------------------

const P1: PlayerId = PlayerId::new(1); // main subject
const P2: PlayerId = PlayerId::new(2); // rival / mentor
const P3: PlayerId = PlayerId::new(3); // club_b player
const CLUB_A: ClubId = ClubId::new(10);
const CLUB_B: ClubId = ClubId::new(20);

// -------------------------------------------------------------------------
// Helper builders
// -------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)] // test fixture builder — inline struct construction is clear
fn player_event(
    player: PlayerId,
    club: ClubId,
    class: EventClass,
    stakes: Q32,
    emotion: Emotion,
    season: u16,
    tick_raw: i64,
    decay: DecayFunction,
    eligibility: CallbackEligibility,
) -> MemoryEvent {
    MemoryEvent {
        event_id: EventId(0),
        schema_version: 1,
        season: SeasonNumber(season),
        tick: Some(Tick::from_raw(tick_raw)),
        career_date: CareerDate {
            year: season + 1,
            day_of_year: 1,
        },
        emitter: Emitter {
            kind: EmitterKind::CareerSystem,
            source_id: SourceId::Club(club),
        },
        participants: vec![
            Participant {
                role: ParticipantRole::Subject,
                entity: EntityRef::Player(player),
            },
            Participant {
                role: ParticipantRole::Counterparty,
                entity: EntityRef::Club(club),
            },
        ],
        event_class: class,
        stakes,
        emotion,
        consequence: vec![Consequence::None],
        callback_eligibility: eligibility,
        salience: stakes, // overwritten by ledger.append
        decay_function: decay,
    }
}

fn match_event(
    player: PlayerId,
    class: EventClass,
    stakes: Q32,
    emotion: Emotion,
    season: u16,
    tick_raw: i64,
) -> MemoryEvent {
    MemoryEvent {
        event_id: EventId(0),
        schema_version: 1,
        season: SeasonNumber(season),
        tick: Some(Tick::from_raw(tick_raw)),
        career_date: CareerDate {
            year: season + 1,
            day_of_year: 100,
        },
        emitter: Emitter {
            kind: EmitterKind::MatchEngine,
            source_id: SourceId::Match(MatchId::new(0)),
        },
        participants: vec![Participant {
            role: ParticipantRole::Subject,
            entity: EntityRef::Player(player),
        }],
        event_class: class,
        stakes,
        emotion,
        consequence: vec![Consequence::None],
        callback_eligibility: CallbackEligibility::Immediate,
        salience: stakes,
        decay_function: DecayFunction::Never,
    }
}

// -------------------------------------------------------------------------
// Shared fixture builder
// -------------------------------------------------------------------------

/// Build a ~30-event ledger covering 3 seasons, 3 players, 2 clubs.
/// Returns the ledger + the tick we use as "now" in reader queries.
fn build_fixture() -> (MemoryLedger, Tick) {
    let mut ledger = MemoryLedger::new();

    // ---- Season 0 (ticks 0-599) ----

    // p1 debuts for club_a
    ledger.append(player_event(
        P1,
        CLUB_A,
        EventClass::DebutSenior,
        Q32::from_raw(1i64 << 30), // 0.25 — modest
        Emotion::Pride,
        0,
        10,
        DecayFunction::Never,
        CallbackEligibility::Immediate,
    ));

    // p1 signs contract (renewal accepted)
    ledger.append(player_event(
        P1,
        CLUB_A,
        EventClass::ContractRenewalAccepted,
        Q32::from_raw(1i64 << 30), // 0.25
        Emotion::Hope,
        0,
        50,
        DecayFunction::Linear {
            lifetime_ticks: 10_000,
        },
        CallbackEligibility::Immediate,
    ));

    // p1 scores a legacy goal (high stakes, Never decay)
    ledger.append(match_event(
        P1,
        EventClass::LegacyGoal,
        Q32::from_raw((1i64 << 31) + (1i64 << 30)), // 0.75
        Emotion::Joy,
        0,
        300,
    ));

    // p2 mentors p1 (relational, mid-stakes)
    ledger.append(player_event(
        P2,
        CLUB_A,
        EventClass::MentorTeammate,
        Q32::from_raw(1i64 << 31), // 0.5
        Emotion::Hope,
        0,
        400,
        DecayFunction::Never,
        CallbackEligibility::Immediate,
    ));

    // p3 debut for club_b (separate club)
    ledger.append(player_event(
        P3,
        CLUB_B,
        EventClass::DebutClub,
        Q32::from_raw(1i64 << 30), // 0.25
        Emotion::Pride,
        0,
        200,
        DecayFunction::Never,
        CallbackEligibility::Immediate,
    ));

    // p1 hat-trick
    ledger.append(match_event(
        P1,
        EventClass::HatTrickScored,
        Q32::from_raw(1i64 << 31), // 0.5
        Emotion::Joy,
        0,
        550,
    ));

    // ---- Season 1 (ticks 600-1199) ----

    // Cup final win — club_a, very high stakes
    ledger.append(player_event(
        P1,
        CLUB_A,
        EventClass::CupFinalWin,
        Q32::ONE,
        Emotion::Joy,
        1,
        700,
        DecayFunction::Never,
        CallbackEligibility::Immediate,
    ));

    // Title won — also high stakes
    ledger.append(player_event(
        P1,
        CLUB_A,
        EventClass::TitleWon,
        Q32::from_raw((1i64 << 31) + (1i64 << 30)), // 0.75
        Emotion::Joy,
        1,
        800,
        DecayFunction::Never,
        CallbackEligibility::Immediate,
    ));

    // p2 also gets cup final win credit (same event from p2's side)
    ledger.append(player_event(
        P2,
        CLUB_A,
        EventClass::CupFinalWin,
        Q32::ONE,
        Emotion::Joy,
        1,
        700,
        DecayFunction::Never,
        CallbackEligibility::Immediate,
    ));

    // Derby controversy — p1, moderate stakes, Never decay
    ledger.append(player_event(
        P1,
        CLUB_A,
        EventClass::DerbyControversy,
        Q32::from_raw(1i64 << 31), // 0.5
        Emotion::Anger,
        1,
        850,
        DecayFunction::Never,
        CallbackEligibility::Immediate,
    ));

    // Rivalry formed (p1 and another player)
    ledger.append(match_event(
        P1,
        EventClass::RivalryFormed,
        Q32::from_raw(1i64 << 31), // 0.5
        Emotion::Anger,
        1,
        900,
    ));

    // p1 promised youth minutes
    ledger.append(player_event(
        P1,
        CLUB_A,
        EventClass::PromisedYouthMinutes,
        Q32::from_raw(1i64 << 30), // 0.25
        Emotion::Hope,
        1,
        950,
        DecayFunction::Linear {
            lifetime_ticks: 5_000,
        },
        CallbackEligibility::Immediate,
    ));

    // Promotion won for club_b (p3)
    ledger.append(player_event(
        P3,
        CLUB_B,
        EventClass::PromotionWon,
        Q32::from_raw(1i64 << 31), // 0.5
        Emotion::Joy,
        1,
        1100,
        DecayFunction::Never,
        CallbackEligibility::Immediate,
    ));

    // p3 international call-up
    ledger.append(match_event(
        P3,
        EventClass::InternationalCallUp,
        Q32::from_raw(1i64 << 31), // 0.5
        Emotion::Pride,
        1,
        1150,
    ));

    // p1 big match scar (e.g. cup semifinal miss) — exponential decay
    ledger.append(match_event(
        P1,
        EventClass::BigMatchScar,
        Q32::from_raw(1i64 << 31), // 0.5
        Emotion::Disappointment,
        1,
        1000,
    ));

    // ---- Season 2 (ticks 1200-1799) ----

    // p1 breakthrough moment — highest stakes, Never decay
    ledger.append(match_event(
        P1,
        EventClass::BreakthroughMoment,
        Q32::ONE,
        Emotion::Pride,
        2,
        1300,
    ));

    // p1 another legacy goal
    ledger.append(match_event(
        P1,
        EventClass::LegacyGoal,
        Q32::from_raw((1i64 << 31) + (1i64 << 30)), // 0.75
        Emotion::Joy,
        2,
        1400,
    ));

    // p1 sold under protest (fan culture, high stakes)
    ledger.append(player_event(
        P1,
        CLUB_A,
        EventClass::SoldUnderProtest,
        Q32::from_raw(3_006_477_107_i64), // ≈ 0.7
        Emotion::Anger,
        2,
        1500,
        DecayFunction::Never,
        CallbackEligibility::Immediate,
    ));

    // p1 long-term injury
    ledger.append(match_event(
        P1,
        EventClass::InjuryLongTerm,
        Q32::from_raw(1i64 << 31), // 0.5
        Emotion::Disappointment,
        2,
        1600,
    ));

    // p2 signature first fired
    ledger.append(match_event(
        P2,
        EventClass::SignatureFirstFired,
        Q32::from_raw(1i64 << 31), // 0.5
        Emotion::Joy,
        2,
        1350,
    ));

    // p2 transfer requested
    ledger.append(player_event(
        P2,
        CLUB_A,
        EventClass::TransferRequested,
        Q32::from_raw(1i64 << 31), // 0.5
        Emotion::Anger,
        2,
        1450,
        DecayFunction::Exponential {
            half_life_ticks: 600,
        },
        CallbackEligibility::Immediate,
    ));

    // p2 transfer refused
    ledger.append(player_event(
        P2,
        CLUB_A,
        EventClass::TransferRefused,
        Q32::from_raw(1i64 << 30), // 0.25
        Emotion::Anger,
        2,
        1460,
        DecayFunction::Exponential {
            half_life_ticks: 600,
        },
        CallbackEligibility::Immediate,
    ));

    // p3 unbeaten run ended (fan culture for club_b)
    ledger.append(player_event(
        P3,
        CLUB_B,
        EventClass::UnbeatenRunEnded,
        Q32::from_raw(1i64 << 31), // 0.5
        Emotion::Disappointment,
        2,
        1600,
        DecayFunction::Never,
        CallbackEligibility::Immediate,
    ));

    // p3 contract renewal rejected by club
    ledger.append(player_event(
        P3,
        CLUB_B,
        EventClass::ContractRenewalRejected,
        Q32::from_raw(1i64 << 30), // 0.25
        Emotion::Anger,
        2,
        1700,
        DecayFunction::Linear {
            lifetime_ticks: 8_000,
        },
        CallbackEligibility::Immediate,
    ));

    // Compaction system event (Never-eligible, should be excluded from press)
    ledger.append(MemoryEvent {
        event_id: EventId(0),
        schema_version: 1,
        season: SeasonNumber(2),
        tick: Some(Tick::from_raw(1750)),
        career_date: CareerDate {
            year: 3,
            day_of_year: 300,
        },
        emitter: Emitter {
            kind: EmitterKind::CareerSystem,
            source_id: SourceId::None,
        },
        participants: vec![],
        event_class: EventClass::Compaction,
        stakes: Q32::ZERO,
        emotion: Emotion::Neutral,
        consequence: vec![Consequence::CompactionDrop { dropped_count: 5 }],
        callback_eligibility: CallbackEligibility::Never,
        salience: Q32::ZERO,
        decay_function: DecayFunction::Never,
    });

    // p1 retirement (high stakes, Never decay, AfterCompaction eligibility)
    ledger.append(player_event(
        P1,
        CLUB_A,
        EventClass::Retirement,
        Q32::from_raw(3_006_477_107_i64), // ≈ 0.7
        Emotion::Neutral,
        2,
        1780,
        DecayFunction::Never,
        CallbackEligibility::AfterCompaction,
    ));

    // "now" tick is 1800 — all events are in the past.
    let now_tick = Tick::from_raw(1800);
    (ledger, now_tick)
}

// -------------------------------------------------------------------------
// Integration test 1: SalienceReader
// -------------------------------------------------------------------------

#[test]
fn integration_salience_reader_top_n_and_filter() {
    let (mut ledger, now_tick) = build_fixture();

    // Top 5 overall: should contain BreakthroughMoment (1.0) + CupFinalWin (1.0)
    // and other high-salience events.
    let top5 = SalienceReader::top_n(&mut ledger, 5, SalienceFilter::None, now_tick);
    assert_eq!(top5.len(), 5, "top 5 overall must return 5 events");

    // Verify descending salience order.
    let mut prev_sal = Q32::MAX;
    for ev in &top5 {
        let sal = fw_memory::project_salience(ev, now_tick);
        assert!(
            sal <= prev_sal,
            "salience must be descending; got {sal:?} after {prev_sal:?}"
        );
        prev_sal = sal;
    }

    // Filter by p1: only p1 events.
    let p1_events = SalienceReader::top_n(&mut ledger, 20, SalienceFilter::BySubject(P1), now_tick);
    assert!(!p1_events.is_empty(), "p1 has events in the fixture");
    for ev in &p1_events {
        let is_p1 = ev
            .participants
            .iter()
            .any(|p| matches!(p.entity, EntityRef::Player(id) if id == P1));
        assert!(is_p1, "all returned events must touch p1");
    }

    // Filter by LegacyGoal class (discriminant 2): only LegacyGoal events.
    let legacy_goals = SalienceReader::top_n(&mut ledger, 10, SalienceFilter::ByClass(2), now_tick);
    assert!(!legacy_goals.is_empty(), "fixture has LegacyGoal events");
    for ev in &legacy_goals {
        assert_eq!(ev.event_class, EventClass::LegacyGoal);
    }
}

// -------------------------------------------------------------------------
// Integration test 2: PressReader
// -------------------------------------------------------------------------

#[test]
fn integration_press_reader_candidates() {
    let (mut ledger, now_tick) = build_fixture();

    // PlayerMilestone topic: BreakthroughMoment, LegacyGoal, HatTrickScored, DebutSenior, etc.
    let player_milestone =
        PressReader::candidates(&mut ledger, PressTopic::PlayerMilestone, now_tick);
    assert!(
        !player_milestone.is_empty(),
        "fixture has PlayerMilestone-class eligible events"
    );
    // Verify all returned are eligible (not Never).
    for ev in &player_milestone {
        assert_ne!(
            ev.callback_eligibility,
            CallbackEligibility::Never,
            "Never-eligible events must be excluded from press candidates"
        );
        assert!(
            PressTopic::PlayerMilestone
                .class_discriminants()
                .contains(&ev.event_class.discriminant()),
            "returned event {:?} must be in PlayerMilestone topic",
            ev.event_class
        );
    }

    // ContractTransfer topic: SoldUnderProtest, TransferRequested, etc.
    let contract = PressReader::candidates(&mut ledger, PressTopic::ContractTransfer, now_tick);
    assert!(
        !contract.is_empty(),
        "fixture has ContractTransfer-class events"
    );
    // Verify descending salience order.
    let mut prev_sal = Q32::MAX;
    for ev in &contract {
        let sal = fw_memory::project_salience(ev, now_tick);
        assert!(
            sal <= prev_sal,
            "press candidates must be descending salience"
        );
        prev_sal = sal;
    }

    // Compaction event must NOT appear in any topic (it is Never-eligible and
    // its class is not in any topic's discriminant set).
    let all_topics = [
        PressTopic::PlayerMilestone,
        PressTopic::ContractTransfer,
        PressTopic::MatchResult,
        PressTopic::Relational,
    ];
    for topic in all_topics {
        let cands = PressReader::candidates(&mut ledger, topic, now_tick);
        for ev in &cands {
            assert_ne!(
                ev.event_class,
                EventClass::Compaction,
                "Compaction (Never-eligible) must not appear in press candidates for {topic:?}"
            );
        }
    }
}

// -------------------------------------------------------------------------
// Integration test 3: FanReader
// -------------------------------------------------------------------------

#[test]
fn integration_fan_reader_fan_callbacks() {
    let (mut ledger, now_tick) = build_fixture();

    // Club A has: CupFinalWin, TitleWon, DerbyControversy, SoldUnderProtest → fan culture.
    let club_a_out = FanReader::fan_callbacks(&mut ledger, CLUB_A, i64::MAX, now_tick);
    assert!(
        !club_a_out.events.is_empty(),
        "club A has fan-culture events in the fixture"
    );
    // All returned events must be fan-culture class.
    for eid in &club_a_out.events {
        let ev = ledger.get_by_id(*eid).unwrap();
        assert!(
            fw_memory::FAN_CULTURE_CLASS_DISCRIMINANTS.contains(&ev.event_class.discriminant()),
            "all fan callbacks must be fan-culture class; got {:?}",
            ev.event_class
        );
    }
    // Emotion tally must sum to the event count.
    let tally = &club_a_out.emotion_tally;
    let tally_sum =
        tally.neutral + tally.joy + tally.anger + tally.pride + tally.disappointment + tally.hope;
    assert_eq!(
        tally_sum as usize,
        club_a_out.events.len(),
        "emotion tally sum must equal event count"
    );

    // Club B has: PromotionWon, UnbeatenRunEnded → fan culture.
    let club_b_out = FanReader::fan_callbacks(&mut ledger, CLUB_B, i64::MAX, now_tick);
    assert!(
        !club_b_out.events.is_empty(),
        "club B has fan-culture events in the fixture"
    );

    // Recency test: narrow window to last 400 ticks (1400-1800).
    // CupFinalWin (tick 700) is outside; SoldUnderProtest (tick 1500) is inside.
    let recent_a = FanReader::fan_callbacks(&mut ledger, CLUB_A, 400, now_tick);
    for eid in &recent_a.events {
        let ev = ledger.get_by_id(*eid).unwrap();
        if let Some(t) = ev.tick {
            let elapsed = now_tick.to_raw() - t.to_raw();
            assert!(
                elapsed <= 400,
                "event outside recency window slipped through"
            );
        }
    }
}

// -------------------------------------------------------------------------
// Integration test 4: ScoutReader
// -------------------------------------------------------------------------

#[test]
fn integration_scout_reader_perceived_events() {
    let (mut ledger, now_tick) = build_fixture();

    // Scout on p1: should have many events (debut, legacy goals, cup final, etc.)
    let p1_view = ScoutReader::perceived_events(&mut ledger, P1, now_tick);
    assert!(
        p1_view.len() >= 5,
        "p1 should have at least 5 events visible to a scout; got {}",
        p1_view.len()
    );
    // All returned events must involve p1.
    for ev in &p1_view {
        let is_p1 = ev
            .participants
            .iter()
            .any(|p| matches!(p.entity, EntityRef::Player(id) if id == P1));
        assert!(is_p1, "all scout-visible events must involve p1");
    }

    // Verify descending salience.
    let mut prev_sal = Q32::MAX;
    for ev in &p1_view {
        let sal = fw_memory::project_salience(ev, now_tick);
        assert!(sal <= prev_sal, "scout events must be descending salience");
        prev_sal = sal;
    }

    // Scout on p3: separate club, different event profile.
    let p3_view = ScoutReader::perceived_events(&mut ledger, P3, now_tick);
    assert!(!p3_view.is_empty(), "p3 has events in the fixture");
    // Ensure no p1 events appear in p3's view.
    for ev in &p3_view {
        let has_p1 = ev
            .participants
            .iter()
            .any(|p| matches!(p.entity, EntityRef::Player(id) if id == P1));
        assert!(!has_p1, "p1 events must not appear in p3's scout view");
    }

    // Unknown player: empty.
    let unknown = ScoutReader::perceived_events(&mut ledger, PlayerId::new(999), now_tick);
    assert!(
        unknown.is_empty(),
        "unknown player returns empty scout view"
    );
}

// -------------------------------------------------------------------------
// Integration test 5: CoachReader
// -------------------------------------------------------------------------

#[test]
fn integration_coach_reader_player_signals() {
    let (mut ledger, now_tick) = build_fixture();

    // Club A's coach sees p1 and p2 (both have club_a events).
    let club_a_signals = CoachReader::player_signals(&mut ledger, CLUB_A, now_tick);
    assert!(!club_a_signals.is_empty(), "club A has players with events");
    // p1 must be in the map.
    assert!(
        club_a_signals.contains_key(&P1),
        "p1 (club_a player) must appear in coach signals"
    );
    // p2 must be in the map.
    assert!(
        club_a_signals.contains_key(&P2),
        "p2 (club_a player) must appear in coach signals"
    );
    // p3 (club_b player) must NOT be in club_a's map.
    assert!(
        !club_a_signals.contains_key(&P3),
        "p3 (club_b player) must not appear in club_a coach signals"
    );

    // Each player's event list must be sorted descending by projected salience.
    for (pid, events) in &club_a_signals {
        let mut prev_sal = Q32::MAX;
        for ev in events {
            let sal = fw_memory::project_salience(ev, now_tick);
            assert!(
                sal <= prev_sal,
                "coach events for player {:?} must be descending salience; got {sal:?} after {prev_sal:?}",
                pid
            );
            prev_sal = sal;
        }
    }

    // Club B's coach sees only p3.
    let club_b_signals = CoachReader::player_signals(&mut ledger, CLUB_B, now_tick);
    assert!(
        club_b_signals.contains_key(&P3),
        "p3 must appear in club_b coach signals"
    );
    assert!(
        !club_b_signals.contains_key(&P1),
        "p1 must not appear in club_b coach signals"
    );
}

//! T4-2.5L — QA-1 compaction retention corpus (D3).
//!
//! Builds 100 deterministic synthetic ledgers (no RNG, no content, no clocks),
//! each covering 10 seasons of events for several players, then calls
//! `compact(SeasonNumber(10))` and asserts three invariants:
//!
//! (a) ALL non-Compaction events are physically retained after compaction —
//!     `ledger.len() == original_count + 1` (only one Compaction marker added).
//!
//! (b) Every event with `season.0 + 5 <= 10` (i.e. seasons 0-5) has `tick == None`;
//!     every event with `season.0 + 5 > 10` (i.e. seasons 6-9) has `tick` unchanged.
//!
//! (c) For each (ledger, player) pair, `SalienceReader::top_n(20, BySubject(pid))`
//!     still returns ≥1 season-0 event. The survival rate across all (ledger,player)
//!     pairs must be ≥ 0.95. A guard asserts ≥ (100 × PLAYERS_PER_LEDGER) pairs
//!     were checked so the assertion cannot pass silently on zero pairs.
//!
//! ## Corpus design (deterministic, no RNG)
//!
//! Event distribution per ledger `i` (0..100) across seasons 0..10 for 5 players:
//! - Each season gets `3 + (i % 4)` events per player, deterministically.
//! - Event class rotates over three classes: DebutSenior / LegacyGoal / BreakthroughMoment.
//! - Stakes is a fixed ladder: season 0 → Q32::ONE, season 1-4 → THREE_QUARTERS,
//!   season 5-9 → HALF. All events use `DecayFunction::Never` so salience is stable.
//! - Season-0 player events always use `stakes = Q32::ONE` (maximum salience),
//!   ensuring they rank in the top-20 by subject regardless of later events.
//!
//! ## Determinism contract
//!
//! No `rand`, no `f32`/`f64`, no `Instant::now()`, no `HashMap`/`HashSet`.
//! All iteration is over fixed-length arrays or `0..N` ranges. Must produce
//! identical results on macOS / Windows / Linux.

use fw_core::{PlayerId, Q32, Tick};
use fw_memory::event::{
    CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind, Emotion,
    EntityRef, EventClass, EventId, MemoryEvent, Participant, ParticipantRole, SeasonNumber,
    SourceId,
};
use fw_memory::ledger::MemoryLedger;
use fw_memory::readers::{SalienceFilter, salience::SalienceReader};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of synthetic ledgers in the corpus.
const LEDGER_COUNT: usize = 100;

/// Number of seasons per ledger.
const SEASONS: u16 = 10;

/// Number of players per ledger.
const PLAYERS_PER_LEDGER: usize = 5;

/// Player ids for the synthetic ledgers. Fixed, well above content-bio range.
const PLAYER_IDS: [u32; PLAYERS_PER_LEDGER] =
    [2_000_001, 2_000_002, 2_000_003, 2_000_004, 2_000_005];

/// Q32 stakes values at each tier.
fn stakes_season_0() -> Q32 {
    Q32::ONE // 1.0 — maximum salience
}
fn stakes_season_1_4() -> Q32 {
    Q32::from_raw(3 * (1i64 << 32) / 4) // 0.75
}
fn stakes_season_5_9() -> Q32 {
    Q32::from_raw(1i64 << 31) // 0.5
}

/// Stakes for `season`.
fn stakes_for_season(season: u16) -> Q32 {
    match season {
        0 => stakes_season_0(),
        1..=4 => stakes_season_1_4(),
        _ => stakes_season_5_9(),
    }
}

/// Rotate event class by index.
fn class_for_index(idx: usize) -> EventClass {
    match idx % 3 {
        0 => EventClass::DebutSenior,
        1 => EventClass::LegacyGoal,
        _ => EventClass::BreakthroughMoment,
    }
}

// ---------------------------------------------------------------------------
// Synthetic ledger builder
// ---------------------------------------------------------------------------

/// Build one synthetic ledger for corpus index `ledger_idx`.
///
/// - 5 players, 10 seasons, `3 + (ledger_idx % 4)` events per (player, season).
/// - All events use `DecayFunction::Never` and `CallbackEligibility::Immediate`.
/// - Returns the built ledger and the count of non-Compaction events (= original count).
fn build_ledger(ledger_idx: usize) -> (MemoryLedger, usize) {
    let mut ledger = MemoryLedger::new();
    let events_per_season = 3 + (ledger_idx % 4); // 3, 4, 5, or 6 events per (player, season)

    for (player_idx, &player_raw) in PLAYER_IDS.iter().enumerate() {
        let player_id = PlayerId::new(player_raw);

        for season in 0..SEASONS {
            let stakes = stakes_for_season(season);

            for event_idx in 0..events_per_season {
                // Deterministic class rotation keyed on (ledger_idx, player_idx, season, event_idx)
                // so different ledgers / players / seasons get different but stable distributions.
                let class_key = ledger_idx
                    .wrapping_add(player_idx * 7)
                    .wrapping_add(season as usize * 13)
                    .wrapping_add(event_idx * 3);
                let event_class = class_for_index(class_key);

                let event = MemoryEvent {
                    event_id: EventId(0), // overwritten by append
                    schema_version: 1,
                    season: SeasonNumber(season),
                    tick: Some(Tick::from_raw(
                        // Deterministic tick: season * 1000 + event_idx * 10
                        (season as i64) * 1000 + (event_idx as i64) * 10,
                    )),
                    career_date: CareerDate {
                        year: season + 1,
                        day_of_year: 1 + (event_idx as u16 * 30).min(364),
                    },
                    emitter: Emitter {
                        kind: EmitterKind::CareerSystem,
                        source_id: SourceId::Player(player_id),
                    },
                    participants: vec![Participant {
                        role: ParticipantRole::Subject,
                        entity: EntityRef::Player(player_id),
                    }],
                    event_class,
                    stakes,
                    emotion: Emotion::Joy,
                    consequence: vec![Consequence::None],
                    callback_eligibility: CallbackEligibility::Immediate,
                    salience: stakes, // will be overwritten by append (compute_salience = stakes)
                    decay_function: DecayFunction::Never,
                };
                ledger.append(event);
            }
        }
    }

    let original_count = ledger.len();
    (ledger, original_count)
}

// ---------------------------------------------------------------------------
// Corpus test
// ---------------------------------------------------------------------------

/// QA-1 compaction retention corpus: 100 ledgers × 10 seasons.
///
/// Asserts three invariants (a), (b), (c) as described in the module doc.
#[test]
fn compaction_retention_corpus_100_ledgers_10_seasons() {
    let compact_season = SeasonNumber(SEASONS); // SeasonNumber(10)

    let mut survival_pass: usize = 0; // pairs where ≥1 season-0 event survived top-20
    let mut survival_total: usize = 0; // total (ledger, player) pairs checked

    for ledger_idx in 0..LEDGER_COUNT {
        let (mut ledger, original_count) = build_ledger(ledger_idx);

        // ---- Compact ----
        let compacted = ledger.compact(compact_season);

        // (a) ALL non-Compaction events are physically retained.
        //     compact() appends exactly ONE Compaction event when in_window_count > 0.
        //     Since seasons 0..5 have events (season.0 + 5 <= 10 for season 0..5),
        //     at least some events are in the compaction window → compacted > 0.
        assert!(
            compacted > 0,
            "ledger {ledger_idx}: compact() must have found events in the 5-season window \
             (seasons 0..5 satisfy season.0 + 5 <= 10); got compacted_count=0"
        );

        let expected_len = original_count + 1; // original + 1 Compaction marker
        assert_eq!(
            ledger.len(),
            expected_len,
            "ledger {ledger_idx}: all non-Compaction events must be physically retained; \
             expected len={expected_len} (original {original_count} + 1 Compaction), \
             got {}",
            ledger.len(),
        );

        // (b) Tick nulling respects the 5-season boundary.
        //     Seasons 0..=5 satisfy season.0 + 5 <= 10 → tick must be None
        //       (season 5: 5 + 5 = 10 <= 10, so it IS compacted).
        //     Seasons 6..=9 satisfy season.0 + 5 > 10 → tick must be unchanged (Some).
        for event in ledger.iter() {
            // Skip the Compaction event itself — it is emitted by compact() with tick=None.
            if matches!(event.event_class, EventClass::Compaction) {
                continue;
            }

            let season_val = event.season.0;
            let in_compaction_window = (season_val as u32) + 5 <= (compact_season.0 as u32);

            if in_compaction_window {
                assert_eq!(
                    event.tick,
                    None,
                    "ledger {ledger_idx}: event in season {season_val} (window: season+5={} <= 10) \
                     must have tick == None after compact(); got {:?}",
                    season_val as u32 + 5,
                    event.tick,
                );
            } else {
                assert!(
                    event.tick.is_some(),
                    "ledger {ledger_idx}: event in season {season_val} (season+5={} > 10) \
                     must have tick.is_some() after compact() (not in window); got tick=None",
                    season_val as u32 + 5,
                );
            }
        }

        // (c) Season-0 events survive in SalienceReader::top_n top-20 per player.
        for &player_raw in &PLAYER_IDS {
            let player_id = PlayerId::new(player_raw);

            let top20 = SalienceReader::top_n(
                &mut ledger,
                20,
                SalienceFilter::BySubject(player_id),
                Tick::ZERO, // now_tick: all events have DecayFunction::Never, so tick is irrelevant
            );

            // Count season-0 events in the top-20.
            let season_0_in_top20 = top20.iter().filter(|e| e.season.0 == 0).count();

            survival_total += 1;
            if season_0_in_top20 >= 1 {
                survival_pass += 1;
            }
        }
    }

    // Guard: corpus must have checked at least LEDGER_COUNT × PLAYERS_PER_LEDGER pairs.
    let minimum_pairs = LEDGER_COUNT * PLAYERS_PER_LEDGER;
    assert_eq!(
        survival_total, minimum_pairs,
        "corpus check must cover exactly {minimum_pairs} (ledger, player) pairs; \
         got {survival_total} — check the loop bounds"
    );

    // (c) Survival rate ≥ 0.95 across all pairs.
    // Computed as integer arithmetic to avoid f64 in test output.
    // survival_pass / survival_total >= 0.95 ↔ survival_pass * 100 >= survival_total * 95
    let pass_pct_numerator = survival_pass * 100;
    let threshold_numerator = survival_total * 95;
    assert!(
        pass_pct_numerator >= threshold_numerator,
        "season-0 event survival rate must be ≥ 95%; \
         got {}/{} = {}% \
         ({} pairs with ≥1 season-0 event in top-20 out of {} total pairs). \
         Check that season-0 stakes=Q32::ONE (maximum) keeps them ranked above \
         later events (stakes=0.75 or 0.5) across all 100 ledgers.",
        survival_pass,
        survival_total,
        (survival_pass * 100) / survival_total,
        survival_pass,
        survival_total,
    );
}

// ---------------------------------------------------------------------------
// QA-T4H item 9: stakes-inverted variant — proves DecayFunction::Never (not stakes)
// ---------------------------------------------------------------------------

/// Stakes-inverted variant: proves `DecayFunction::Never` is the survival driver,
/// NOT high stakes.
///
/// Setup: two groups of events per player per ledger, both in season 0:
/// - Group A: `DecayFunction::Never`, stakes = HALF (low).
///   At a large `now_tick` (past any Linear lifetime), projected salience = HALF
///   (no-decay semantic: salience never falls).
/// - Group B: `DecayFunction::Linear { lifetime_ticks: 10 }`, stakes = ONE (high).
///   At `now_tick = 10_000` (well past lifetime=10), projected salience = ZERO
///   (fully decayed despite high emission stakes).
///
/// After querying with `now_tick = 10_000`, the top-N must be dominated by Group A
/// events (proj sal = HALF) rather than Group B events (proj sal = ZERO).
///
/// This is the true "stakes-inverted" scenario from the audit: Group B has HIGHER
/// stakes than Group A, but `DecayFunction::Never` is what carries Group A's survival
/// in `top_n`. If we used high stakes for both, high stakes would be the driver.
/// If we used `DecayFunction::Never` for both, decay wouldn't be tested.
///
/// Mutation killed: if `project_salience` ignored `DecayFunction::Never` and always
/// returned ZERO for old events, Group A would also have projected salience ZERO and
/// the assertion that Group A events are in the top-N would fail.
#[test]
fn compaction_stakes_inverted_decay_never_exercises_no_decay_semantic() {
    use fw_memory::readers::{SalienceFilter, salience::SalienceReader};

    // Group A: DecayFunction::Never, low stakes (HALF).
    const STAKES_A: Q32 = Q32::from_raw(1i64 << 31); // HALF
    // Group B: DecayFunction::Linear (short lifetime), high stakes (ONE).
    const STAKES_B: Q32 = Q32::ONE;
    const LIFETIME_B: u32 = 10; // very short — fully decayed at now_tick=10_000

    // Query time: well past Group B's lifetime, so Group B projected sal = ZERO.
    let now_tick_query = Tick::from_raw(10_000);

    let mut group_a_in_top: usize = 0;
    let mut group_b_in_top: usize = 0;
    let mut total_queries: usize = 0;

    for ledger_idx in 0..LEDGER_COUNT {
        let mut ledger = MemoryLedger::new();
        let player_id = PlayerId::new(PLAYER_IDS[0]); // single player for simplicity

        // Build 3 Group-A events (Never-decay, low stakes) and 3 Group-B events (Linear, high stakes).
        // Both groups use tick = 0 so elapsed at now_tick=10_000 is 10_000 for Group B.
        for i in 0usize..3 {
            // Group A: Never-decay, stakes=HALF. Tick = 0 (emission tick, but decay never applies).
            let class_key_a = ledger_idx * 3 + i;
            let event_a = MemoryEvent {
                event_id: EventId(0),
                schema_version: 1,
                season: SeasonNumber(0),
                tick: Some(Tick::ZERO), // emitted at tick 0
                career_date: CareerDate {
                    year: 1,
                    day_of_year: 1,
                },
                emitter: Emitter {
                    kind: EmitterKind::CareerSystem,
                    source_id: SourceId::Player(player_id),
                },
                participants: vec![Participant {
                    role: ParticipantRole::Subject,
                    entity: EntityRef::Player(player_id),
                }],
                event_class: class_for_index(class_key_a),
                stakes: STAKES_A,
                emotion: Emotion::Pride,
                consequence: vec![Consequence::None],
                callback_eligibility: CallbackEligibility::Immediate,
                salience: STAKES_A, // overwritten by append, but set for clarity
                decay_function: DecayFunction::Never, // key: no decay
            };
            ledger.append(event_a);

            // Group B: Linear decay, stakes=ONE (high). Tick = 0. Fully decayed at now_tick=10_000.
            let class_key_b = ledger_idx * 3 + i + 100;
            let event_b = MemoryEvent {
                event_id: EventId(0),
                schema_version: 1,
                season: SeasonNumber(0),
                tick: Some(Tick::ZERO), // emitted at tick 0; decays by lifetime_ticks=10
                career_date: CareerDate {
                    year: 1,
                    day_of_year: 2,
                },
                emitter: Emitter {
                    kind: EmitterKind::CareerSystem,
                    source_id: SourceId::Player(player_id),
                },
                participants: vec![Participant {
                    role: ParticipantRole::Subject,
                    entity: EntityRef::Player(player_id),
                }],
                event_class: class_for_index(class_key_b),
                stakes: STAKES_B,
                emotion: Emotion::Joy,
                consequence: vec![Consequence::None],
                callback_eligibility: CallbackEligibility::Immediate,
                salience: STAKES_B, // overwritten by append
                decay_function: DecayFunction::Linear {
                    lifetime_ticks: LIFETIME_B,
                },
            };
            ledger.append(event_b);
        }

        // Query top-3 at now_tick_query (well past Group B's lifetime).
        // Expected: Group A events (proj sal=HALF) dominate; Group B (proj sal=ZERO) are last.
        let top3 = SalienceReader::top_n(
            &mut ledger,
            3,
            SalienceFilter::BySubject(player_id),
            now_tick_query,
        );

        total_queries += 1;

        for ev in &top3 {
            match ev.decay_function {
                DecayFunction::Never => group_a_in_top += 1,
                DecayFunction::Linear { .. } => group_b_in_top += 1,
                _ => {}
            }
        }
    }

    // Guard: corpus ran.
    assert_eq!(
        total_queries, LEDGER_COUNT,
        "must have run {LEDGER_COUNT} queries"
    );

    // Group A events must dominate the top-3 in all 100 ledgers.
    // Group A has projected sal = HALF (DecayFunction::Never keeps it alive).
    // Group B has projected sal = ZERO (Linear-decayed to zero at now_tick=10_000).
    // Therefore, ALL top-3 slots across all ledgers must be Group A events.
    assert_eq!(
        group_b_in_top, 0,
        "Group B (Linear, high stakes, fully decayed at now_tick=10_000) must appear \
         ZERO times in the top-3 across all {LEDGER_COUNT} ledgers; got {group_b_in_top}. \
         DecayFunction::Never is what carries Group A's survival — not its stakes (HALF < ONE). \
         Mutation killed: if project_salience ignored DecayFunction::Never and returned ZERO \
         for all events, Group A would also be ZERO and both groups would tie.",
    );
    assert_eq!(
        group_a_in_top,
        LEDGER_COUNT * 3, // 100 ledgers × 3 Group-A events each in the top-3
        "Group A (Never-decay, low stakes) must fill all top-3 slots across all ledgers; \
         got {group_a_in_top} (expected {})",
        LEDGER_COUNT * 3,
    );
}

// ---------------------------------------------------------------------------
// QA-T4H item 12-test: compaction decaying-event reader-order
// ---------------------------------------------------------------------------

/// A `Linear`-decay high-stakes OLD event and a newer event: assert the intended
/// pre-compaction vs post-compaction `SalienceReader` order.
///
/// ## Setup
///
/// - OLD event: season 0, `tick = Some(Tick(0))`, `salience = ONE`, `Linear` decay
///   with `lifetime_ticks = 100`. At `now_tick = 200` (well past lifetime), its
///   projected salience = ZERO (fully decayed).
/// - NEW event: season 5, `tick = Some(Tick(5000))`, `salience = HALF`, `Never` decay.
///   At `now_tick = 200`, its projected salience = HALF (no decay; tick > now_tick so
///   elapsed ≤ 0 guard fires → returns full salience).
///
/// Pre-compaction at `now_tick = 200`:
///   OLD projected salience = ZERO (elapsed=200 >= lifetime=100).
///   NEW projected salience = HALF.
///   Order: NEW first, OLD second (or OLD excluded from top-1).
///
/// After compaction at `SeasonNumber(10)` (season 0 is in the 5-season window):
///   OLD tick → None. `project_salience(OLD, any_tick) = event.salience = ONE` (no-decay fallback).
///   NEW tick unchanged (season 5, 5+5=10 <= 10 → also compacted! tick → None too).
///   NEW projected salience = HALF (emission salience, no-decay fallback).
///   Order: OLD first (ONE > HALF), NEW second.
///
/// The order JUMPS: a previously-zero-salience event becomes the top-ranked event
/// after compaction because tick=None triggers the no-decay fallback at full emission
/// salience. This pins the "timeless anchor" semantic.
///
/// Mutation killed: if `project_salience` did NOT apply the `tick=None → no-decay`
/// fallback (e.g. returned ZERO for None tick), OLD would remain ZERO post-compaction
/// and the order would not flip → assertion fails.
#[test]
fn compaction_decaying_event_jumps_to_top_after_tick_nulled() {
    use fw_core::MatchId;
    use fw_memory::readers::{SalienceFilter, project_salience, salience::SalienceReader};

    let player_id = PlayerId::new(9_000_001);
    let now_tick_before = Tick::from_raw(200); // well past OLD's lifetime=100

    // Helper: create a MemoryEvent with given properties.
    let make_ev = |season: u16, tick_raw: i64, salience: Q32, decay: DecayFunction| MemoryEvent {
        event_id: EventId(0),
        schema_version: 1,
        season: SeasonNumber(season),
        tick: Some(Tick::from_raw(tick_raw)),
        career_date: CareerDate {
            year: season + 1,
            day_of_year: 1,
        },
        emitter: Emitter {
            kind: EmitterKind::MatchEngine,
            source_id: SourceId::Match(MatchId::new(0)),
        },
        participants: vec![Participant {
            role: ParticipantRole::Subject,
            entity: EntityRef::Player(player_id),
        }],
        event_class: EventClass::LegacyGoal,
        stakes: salience,
        emotion: Emotion::Joy,
        consequence: vec![Consequence::None],
        callback_eligibility: CallbackEligibility::Immediate,
        salience, // will be overwritten by ledger.append
        decay_function: decay,
    };

    // OLD event: high salience but short-lived — decays to zero before now_tick.
    let old_event = make_ev(
        0,        // season 0 — in the 5-season compaction window at SeasonNumber(10)
        0,        // tick = 0
        Q32::ONE, // emission salience = ONE
        DecayFunction::Linear {
            lifetime_ticks: 100,
        },
    );

    // NEW event: lower salience, Never decays.
    // season 5: 5+5=10 <= 10 → ALSO in the compaction window at SeasonNumber(10).
    // We use a future tick (5000) so elapsed at now_tick=200 is negative → guard fires → full sal.
    let new_event = make_ev(
        5,
        5000, // tick = 5000 > 200 → elapsed = 200-5000 = -4800 → guard → full salience
        Q32::from_raw(1i64 << 31), // HALF
        DecayFunction::Never,
    );

    let mut ledger = MemoryLedger::new();
    let old_id = ledger.append(old_event);
    let new_id = ledger.append(new_event);

    // Pre-compaction: at now_tick=200, OLD is fully decayed (proj sal = ZERO).
    // NEW has tick=5000 > 200 → elapsed ≤ 0 → proj sal = HALF (emission).
    // Verify the pre-compaction projected saliences directly.
    {
        let old_ev = ledger.get_by_id(old_id).expect("old event");
        let new_ev = ledger.get_by_id(new_id).expect("new event");
        let old_proj = project_salience(old_ev, now_tick_before);
        let new_proj = project_salience(new_ev, now_tick_before);

        assert_eq!(
            old_proj,
            Q32::ZERO,
            "pre-compaction: OLD event at tick=0 with lifetime=100 must have \
             projected salience = ZERO at now_tick=200 (elapsed=200 >= lifetime=100)"
        );
        assert_eq!(
            new_proj,
            Q32::from_raw(1i64 << 31), // HALF
            "pre-compaction: NEW event at tick=5000 must have projected salience = HALF \
             at now_tick=200 (elapsed ≤ 0 guard → full emission salience)"
        );
    }

    // Pre-compaction reader order at now_tick=200: NEW (HALF) > OLD (ZERO).
    let pre_compact_top2 = SalienceReader::top_n(
        &mut ledger,
        2,
        SalienceFilter::BySubject(player_id),
        now_tick_before,
    );
    assert_eq!(pre_compact_top2.len(), 2, "must have 2 events");
    assert_eq!(
        pre_compact_top2[0].event_id, new_id,
        "pre-compaction: NEW must rank first (proj sal=HALF > OLD's ZERO) at now_tick=200"
    );
    assert_eq!(
        pre_compact_top2[1].event_id, old_id,
        "pre-compaction: OLD must rank second (proj sal=ZERO)"
    );

    // Compact at SeasonNumber(10).
    // Both season-0 and season-5 events are in the window (season.0+5 <= 10).
    // Both get tick → None.
    ledger.compact(SeasonNumber(10));

    // Verify both events now have tick == None.
    assert_eq!(
        ledger
            .get_by_id(old_id)
            .expect("old event post-compact")
            .tick,
        None,
        "OLD event (season 0) must have tick == None after compact at SeasonNumber(10)"
    );
    assert_eq!(
        ledger
            .get_by_id(new_id)
            .expect("new event post-compact")
            .tick,
        None,
        "NEW event (season 5) must have tick == None after compact at SeasonNumber(10) \
         (season 5 + 5 = 10 <= 10)"
    );

    // Post-compaction reader order: now_tick is irrelevant because both ticks are None.
    // project_salience(tick=None) = event.salience (no-decay fallback).
    //   OLD: salience = ONE (set by ledger.append using stakes=ONE).
    //   NEW: salience = HALF (set by ledger.append using stakes=HALF).
    // Order: OLD first (ONE > HALF), NEW second.
    //
    // The order has FLIPPED vs pre-compaction. This is the "timeless anchor" semantic:
    // compaction reclassifies a decayed event as having its original emission salience.
    let post_compact_top2 = SalienceReader::top_n(
        &mut ledger,
        2,
        SalienceFilter::BySubject(player_id),
        now_tick_before, // now_tick doesn't matter (both tick=None), but pass same value for consistency
    );
    assert_eq!(
        post_compact_top2.len(),
        2,
        "must still have 2 events post-compact"
    );
    assert_eq!(
        post_compact_top2[0].event_id, old_id,
        "post-compaction: OLD must rank FIRST (timeless anchor: proj sal reverts to emission \
         salience ONE, no-decay fallback); order must have flipped vs pre-compaction. \
         Mutation killed: if project_salience did not apply tick=None → no-decay fallback, \
         OLD's projected salience would remain ZERO and NEW would stay first."
    );
    assert_eq!(
        post_compact_top2[1].event_id, new_id,
        "post-compaction: NEW must rank second (proj sal = HALF, no-decay fallback)"
    );
}

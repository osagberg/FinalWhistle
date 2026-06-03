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

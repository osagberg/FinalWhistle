//! `FanReader` — fan-culture-tagged events for the fan-mood surface.
//!
//! Returns fan-culture events for a specific club within a recency window,
//! plus a simple `Emotion` tally across those events.
//!
//! ## Cadence
//!
//! Once per match-day cycle.
//!
//! ## Fan-culture class set (T3-2 structural subset)
//!
//! Events must belong to the fixed fan-culture class set defined in
//! `FAN_CULTURE_CLASS_DISCRIMINANTS` (readers/mod.rs):
//! LegacyGoal, CupFinalWin, CupFinalLoss, TitleWon, PromotionWon,
//! RelegationSuffered, DerbyControversy, SoldUnderProtest, UnbeatenRunEnded.
//!
//! ## Recency window
//!
//! The `recency_window_ticks` parameter restricts results to events emitted
//! within `recency_window_ticks` of `now_tick`. Events with `tick == None`
//! (compacted) are always included — their tick granularity was dropped by
//! compaction, so they are treated as "timeless" historical anchors.
//!
//! Events with `tick == Some(t)` where `now_tick - t > recency_window_ticks`
//! are excluded.
//!
//! ## Club filter
//!
//! Only events where `club_id` participates in any role are included.
//! Uses the `by_club` lazy index for O(log n) club pre-filtering.

use fw_core::{ClubId, Tick};

use crate::event::{EventId, MemoryEvent};
use crate::ledger::MemoryLedger;
use crate::readers::{
    EmotionTally, FAN_CULTURE_CLASS_DISCRIMINANTS, FanReaderOutput, project_salience,
};

/// Stateless fan-culture reader.
pub struct FanReader;

impl FanReader {
    /// Fan-culture events for `club` within the recency window.
    ///
    /// Returns `FanReaderOutput` with:
    /// - `events`: sorted by projected salience descending (ties: event_id asc).
    /// - `emotion_tally`: count of each `Emotion` variant across the result set.
    ///
    /// ## Parameters
    ///
    /// - `ledger`: the career ledger (mutably borrowed to warm indexes).
    /// - `club`: only events where this club participates in any role.
    /// - `recency_window_ticks`: maximum elapsed ticks from emission to now.
    ///   Pass `i64::MAX` for no recency restriction.
    /// - `now_tick`: current sim tick for recency + salience projection.
    #[must_use]
    pub fn fan_callbacks(
        ledger: &mut MemoryLedger,
        club: ClubId,
        recency_window_ticks: i64,
        now_tick: Tick,
    ) -> FanReaderOutput {
        // Collect EventIds for this club from the index.
        let club_event_ids: Vec<EventId> = ledger.by_club(club).to_vec();

        // Resolve to event refs, apply fan-culture + recency filters.
        let mut candidates: Vec<&MemoryEvent> = club_event_ids
            .iter()
            .filter_map(|&eid| ledger.get_by_id(eid))
            .filter(|e| {
                // Fan-culture class filter.
                if !FAN_CULTURE_CLASS_DISCRIMINANTS.contains(&e.event_class.discriminant()) {
                    return false;
                }
                // Recency window: compacted events (tick=None) always pass.
                match e.tick {
                    None => true,
                    Some(emission_tick) => {
                        let elapsed = now_tick.to_raw() - emission_tick.to_raw();
                        elapsed <= recency_window_ticks
                    }
                }
            })
            .collect();

        // Sort: projected salience desc, then event_id asc.
        candidates.sort_by(|a, b| {
            let sa = project_salience(a, now_tick);
            let sb = project_salience(b, now_tick);
            sb.cmp(&sa).then_with(|| a.event_id.cmp(&b.event_id))
        });

        // Build tally and collect event_ids.
        let mut tally = EmotionTally::default();
        let event_ids: Vec<EventId> = candidates
            .iter()
            .map(|e| {
                tally.record(e.emotion);
                e.event_id
            })
            .collect();

        FanReaderOutput {
            events: event_ids,
            emotion_tally: tally,
        }
    }
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{
        CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind, Emotion,
        EntityRef, EventClass, EventId, MemoryEvent, Participant, ParticipantRole, SeasonNumber,
        SourceId,
    };
    use crate::ledger::MemoryLedger;
    use fw_core::{ClubId, PlayerId, Q32, Tick};

    fn make_club_event(
        club: ClubId,
        class: EventClass,
        emotion: Emotion,
        stakes: Q32,
        emission_tick: Tick,
    ) -> MemoryEvent {
        MemoryEvent {
            event_id: EventId(0),
            schema_version: 1,
            season: SeasonNumber(0),
            tick: Some(emission_tick),
            career_date: CareerDate {
                year: 1,
                day_of_year: 1,
            },
            emitter: Emitter {
                kind: EmitterKind::CareerSystem,
                source_id: SourceId::Club(club),
            },
            participants: vec![Participant {
                role: ParticipantRole::Subject,
                entity: EntityRef::Club(club),
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

    fn make_player_event(player: PlayerId, club: ClubId, class: EventClass) -> MemoryEvent {
        MemoryEvent {
            event_id: EventId(0),
            schema_version: 1,
            season: SeasonNumber(0),
            tick: Some(Tick::ZERO),
            career_date: CareerDate {
                year: 1,
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
            stakes: Q32::from_raw(1i64 << 31),
            emotion: Emotion::Anger,
            consequence: vec![Consequence::None],
            callback_eligibility: CallbackEligibility::Immediate,
            salience: Q32::from_raw(1i64 << 31),
            decay_function: DecayFunction::Never,
        }
    }

    fn half() -> Q32 {
        Q32::from_raw(1i64 << 31)
    }
    fn three_quarters() -> Q32 {
        Q32::from_raw((1i64 << 31) + (1i64 << 30))
    }

    /// AC4: includes fan-culture events for the club, excludes non-fan-culture classes.
    #[test]
    fn fan_callbacks_includes_fan_culture_and_excludes_others() {
        let mut ledger = MemoryLedger::new();
        let c = ClubId::new(1);

        // Fan-culture events for club C → should be included.
        ledger.append(make_club_event(
            c,
            EventClass::LegacyGoal,
            Emotion::Joy,
            half(),
            Tick::ZERO,
        ));
        ledger.append(make_club_event(
            c,
            EventClass::DerbyControversy,
            Emotion::Anger,
            three_quarters(),
            Tick::ZERO,
        ));
        // Non-fan-culture event for club C → should be excluded.
        ledger.append(make_club_event(
            c,
            EventClass::DebutSenior,
            Emotion::Pride,
            half(),
            Tick::ZERO,
        ));

        let out = FanReader::fan_callbacks(&mut ledger, c, i64::MAX, Tick::ZERO);

        assert_eq!(out.events.len(), 2, "only fan-culture events included");
        // Verify no DebutSenior slipped through.
        for eid in &out.events {
            let ev = ledger.get_by_id(*eid).unwrap();
            assert!(
                crate::readers::FAN_CULTURE_CLASS_DISCRIMINANTS
                    .contains(&ev.event_class.discriminant()),
                "all returned events must be fan-culture class"
            );
        }
    }

    /// AC4: event outside the recency window is excluded.
    #[test]
    fn fan_callbacks_excludes_events_outside_recency_window() {
        let mut ledger = MemoryLedger::new();
        let c = ClubId::new(2);

        // Event at tick 0 — window of 50 ticks → elapsed at now_tick=100 is 100 > 50.
        ledger.append(make_club_event(
            c,
            EventClass::CupFinalWin,
            Emotion::Joy,
            half(),
            Tick::ZERO,
        ));
        // Event at tick 80 — elapsed at now_tick=100 is 20 ≤ 50 → included.
        ledger.append(make_club_event(
            c,
            EventClass::TitleWon,
            Emotion::Joy,
            three_quarters(),
            Tick::from_raw(80),
        ));

        let out = FanReader::fan_callbacks(&mut ledger, c, 50, Tick::from_raw(100));

        assert_eq!(
            out.events.len(),
            1,
            "only the recent event should be included"
        );
        let included = ledger.get_by_id(out.events[0]).unwrap();
        assert_eq!(included.event_class, EventClass::TitleWon);
    }

    /// AC4: emotion tally counts each included event's emotion exactly once.
    #[test]
    fn fan_callbacks_emotion_tally_counts_correctly() {
        let mut ledger = MemoryLedger::new();
        let c = ClubId::new(3);
        let p = PlayerId::new(10);

        // Joy × 2, Anger × 1
        ledger.append(make_club_event(
            c,
            EventClass::CupFinalWin,
            Emotion::Joy,
            half(),
            Tick::ZERO,
        ));
        ledger.append(make_club_event(
            c,
            EventClass::TitleWon,
            Emotion::Joy,
            three_quarters(),
            Tick::ZERO,
        ));
        ledger.append(make_player_event(p, c, EventClass::SoldUnderProtest));

        let out = FanReader::fan_callbacks(&mut ledger, c, i64::MAX, Tick::ZERO);

        assert_eq!(out.events.len(), 3);
        assert_eq!(out.emotion_tally.joy, 2, "two Joy events");
        assert_eq!(out.emotion_tally.anger, 1, "one Anger event");
        assert_eq!(out.emotion_tally.neutral, 0);
    }
}

//! `PressReader` — callback-eligible events for press-conference slot-filling.
//!
//! Returns a ranked candidate list for a given press topic frame. At T3-2
//! this is structural only — Tracery slot-filling (rendering the candidate
//! events into prose via the phrase bank) lands at T3-3.
//!
//! ## Cadence
//!
//! Per press conference (~weekly in-game cadence).
//!
//! ## Callback eligibility
//!
//! Only events whose `callback_eligibility` permits recall are included.
//! `CallbackEligibility::Never` events (system events such as `Compaction`)
//! are excluded; `Immediate` and `AfterCompaction` events are included.

use fw_core::Tick;

use crate::event::{CallbackEligibility, MemoryEvent};
use crate::ledger::MemoryLedger;
use crate::readers::{PressTopic, project_salience};

/// Stateless press-conference reader.
///
/// Returns callback-eligible events whose class is in the requested topic
/// frame, ranked by projected salience. Structural only at T3-2 — Tracery
/// slot-filling is T3-3.
pub struct PressReader;

impl PressReader {
    /// Candidate events for a press-conference topic.
    ///
    /// Returns events from `ledger` that:
    /// 1. Have `callback_eligibility != CallbackEligibility::Never`.
    /// 2. Have `event_class.discriminant()` in `topic.class_discriminants()`.
    ///
    /// Sorted by projected salience descending; ties broken by `event_id`
    /// ascending (deterministic).
    #[must_use]
    pub fn candidates(
        ledger: &mut MemoryLedger,
        topic: PressTopic,
        now_tick: Tick,
    ) -> Vec<&MemoryEvent> {
        let topic_classes = topic.class_discriminants();

        let mut candidates: Vec<&MemoryEvent> = ledger
            .events
            .iter()
            .filter(|e| {
                // Exclude never-eligible events.
                e.callback_eligibility != CallbackEligibility::Never
                    // Restrict to topic class set.
                    && topic_classes.contains(&e.event_class.discriminant())
            })
            .collect();

        candidates.sort_by(|a, b| {
            let sa = project_salience(a, now_tick);
            let sb = project_salience(b, now_tick);
            sb.cmp(&sa).then_with(|| a.event_id.cmp(&b.event_id))
        });

        candidates
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
    use crate::readers::PressTopic;
    use fw_core::{MatchId, PlayerId, Q32, Tick};

    fn make_event(class: EventClass, stakes: Q32, eligibility: CallbackEligibility) -> MemoryEvent {
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
                source_id: SourceId::Match(MatchId::new(0)),
            },
            participants: vec![Participant {
                role: ParticipantRole::Subject,
                entity: EntityRef::Player(PlayerId::new(1)),
            }],
            event_class: class,
            stakes,
            emotion: Emotion::Neutral,
            consequence: vec![Consequence::None],
            callback_eligibility: eligibility,
            salience: stakes,
            decay_function: DecayFunction::Never,
        }
    }

    fn half() -> Q32 {
        Q32::from_raw(1i64 << 31)
    }
    fn three_quarters() -> Q32 {
        Q32::from_raw((1i64 << 31) + (1i64 << 30))
    }

    /// AC3: candidates returns only callback-eligible events in-topic, ranked desc.
    #[test]
    fn candidates_returns_eligible_in_topic_events_ranked() {
        let mut ledger = MemoryLedger::new();

        // PlayerMilestone includes: LegacyGoal(2), DebutSenior(24), HatTrickScored(3), etc.
        // In-topic (PlayerMilestone) + eligible → included
        ledger.append(make_event(
            EventClass::LegacyGoal,
            half(),
            CallbackEligibility::Immediate,
        ));
        // In-topic + eligible, higher salience → ranked first
        ledger.append(make_event(
            EventClass::HatTrickScored,
            three_quarters(),
            CallbackEligibility::Immediate,
        ));
        // In-topic + NOT eligible → excluded
        ledger.append(make_event(
            EventClass::DebutSenior,
            Q32::ONE,
            CallbackEligibility::Never,
        ));
        // Out-of-topic (MatchResult) + eligible → excluded
        ledger.append(make_event(
            EventClass::CupFinalWin,
            Q32::ONE,
            CallbackEligibility::Immediate,
        ));

        let result = PressReader::candidates(&mut ledger, PressTopic::PlayerMilestone, Tick::ZERO);

        // Only 2 should be included: LegacyGoal and HatTrickScored
        assert_eq!(
            result.len(),
            2,
            "expected exactly 2 in-topic eligible events"
        );
        // HatTrickScored (0.75) > LegacyGoal (0.5)
        assert_eq!(result[0].event_class, EventClass::HatTrickScored);
        assert_eq!(result[1].event_class, EventClass::LegacyGoal);
    }

    /// AC3: a non-eligible event of an in-topic class is EXCLUDED.
    #[test]
    fn never_eligible_event_is_excluded_even_if_in_topic() {
        let mut ledger = MemoryLedger::new();

        // Compaction is system class (discriminant 29), Never eligibility.
        ledger.append(make_event(
            EventClass::Compaction,
            Q32::ONE,
            CallbackEligibility::Never,
        ));
        // Also test an in-topic class explicitly marked Never.
        ledger.append(make_event(
            EventClass::LegacyGoal,
            Q32::ONE,
            CallbackEligibility::Never,
        ));

        let result = PressReader::candidates(&mut ledger, PressTopic::PlayerMilestone, Tick::ZERO);
        assert!(result.is_empty(), "Never-eligible events must be excluded");
    }

    /// AC3: an eligible event of an out-of-topic class is EXCLUDED.
    #[test]
    fn out_of_topic_event_is_excluded_even_if_eligible() {
        let mut ledger = MemoryLedger::new();

        // ContractTransfer event — not in PlayerMilestone topic.
        ledger.append(make_event(
            EventClass::SoldUnderProtest,
            Q32::ONE,
            CallbackEligibility::Immediate,
        ));
        // Relational event — also not in PlayerMilestone.
        ledger.append(make_event(
            EventClass::RivalryFormed,
            Q32::ONE,
            CallbackEligibility::Immediate,
        ));

        let result = PressReader::candidates(&mut ledger, PressTopic::PlayerMilestone, Tick::ZERO);
        assert!(result.is_empty(), "Out-of-topic events must be excluded");
    }
}

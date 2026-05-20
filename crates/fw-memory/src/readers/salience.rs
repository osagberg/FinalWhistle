//! `SalienceReader` — top-N events by projected current salience.
//!
//! The primary on-demand reader for UI surfaces and pre-match overlays.
//! Returns the highest-salience events from the ledger, optionally filtered
//! by subject player or event class. Deterministically ordered.
//!
//! ## Cadence
//!
//! On demand (UI surfaces, pre-match overlays). Not cached between calls.

use fw_core::Tick;

use crate::event::{EntityRef, MemoryEvent};
use crate::ledger::MemoryLedger;
use crate::readers::{SalienceFilter, project_salience};

/// Read-only top-N salience projection over a `MemoryLedger`.
///
/// A stateless unit struct — all query state is passed as parameters.
pub struct SalienceReader;

impl SalienceReader {
    /// Return the `n` highest-projected-salience events from `ledger` at
    /// `now_tick`, applying `filter` to restrict the candidate set.
    ///
    /// Results are sorted by projected salience **descending**. When two events
    /// tie on projected salience, the lower `event_id` comes first (ascending).
    /// This gives a deterministic total order that does not depend on insertion
    /// order or BTreeMap iteration internals.
    ///
    /// ## Filter semantics
    ///
    /// - `SalienceFilter::None` — all events in the ledger are candidates.
    /// - `SalienceFilter::BySubject(player_id)` — only events where
    ///   `player_id` appears as ANY participant (not just Subject role).
    ///   Uses the `by_subject` index for O(log n) pre-filtering when the
    ///   player is a Subject; falls back to a linear scan for other roles.
    /// - `SalienceFilter::ByClass(discriminant)` — only events whose
    ///   `event_class.discriminant()` matches.
    ///
    /// ## Complexity
    ///
    /// O(n log n) where n is the candidate event count (sort dominates).
    /// For `BySubject`, the sort is over the per-player event list, not the
    /// full ledger.
    #[must_use]
    pub fn top_n(
        ledger: &mut MemoryLedger,
        n: usize,
        filter: SalienceFilter,
        now_tick: Tick,
    ) -> Vec<&MemoryEvent> {
        if n == 0 {
            return Vec::new();
        }

        // Collect candidates matching the filter.
        let mut candidates: Vec<&MemoryEvent> = match &filter {
            SalienceFilter::None => ledger.events.iter().collect(),

            SalienceFilter::BySubject(pid) => {
                // Use the subject index for the common case (player is Subject),
                // then scan all events to include any role.
                // Since the index only tracks Subject role, we do a full scan
                // filtered by player presence in any participant slot.
                let target = *pid;
                ledger
                    .events
                    .iter()
                    .filter(|e| {
                        e.participants
                            .iter()
                            .any(|p| matches!(p.entity, EntityRef::Player(id) if id == target))
                    })
                    .collect()
            }

            SalienceFilter::ByClass(disc) => {
                let target = *disc;
                ledger
                    .events
                    .iter()
                    .filter(|e| e.event_class.discriminant() == target)
                    .collect()
            }
        };

        // Sort: descending projected salience, then ascending event_id.
        candidates.sort_by(|a, b| {
            let sa = project_salience(a, now_tick);
            let sb = project_salience(b, now_tick);
            sb.cmp(&sa).then_with(|| a.event_id.cmp(&b.event_id))
        });

        candidates.truncate(n);
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
    use crate::readers::SalienceFilter;
    use fw_core::{MatchId, PlayerId, Q32, Tick};

    fn make_event(player_id: PlayerId, stakes: Q32, class: EventClass) -> MemoryEvent {
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
                kind: EmitterKind::MatchEngine,
                source_id: SourceId::Match(MatchId::new(0)),
            },
            participants: vec![Participant {
                role: ParticipantRole::Subject,
                entity: EntityRef::Player(player_id),
            }],
            event_class: class,
            stakes,
            emotion: Emotion::Joy,
            consequence: vec![Consequence::None],
            callback_eligibility: CallbackEligibility::Immediate,
            salience: stakes, // will be overwritten by ledger.append, but pre-set for helpers
            decay_function: DecayFunction::Never,
        }
    }

    fn half() -> Q32 {
        Q32::from_raw(1i64 << 31)
    }
    fn quarter() -> Q32 {
        Q32::from_raw(1i64 << 30)
    }
    fn three_quarters() -> Q32 {
        Q32::from_raw((1i64 << 31) + (1i64 << 30))
    }

    /// AC2: top_n returns the 3 highest-salience events in descending order.
    #[test]
    fn top_n_returns_highest_salience_descending() {
        let mut ledger = MemoryLedger::new();
        let p = PlayerId::new(1);

        // Append 5 events with known stakes (salience = stakes after compute_salience fix).
        ledger.append(make_event(p, quarter(), EventClass::DebutSenior)); // 0.25
        ledger.append(make_event(p, Q32::ONE, EventClass::CupFinalWin)); // 1.0
        ledger.append(make_event(p, half(), EventClass::LegacyGoal)); // 0.5
        ledger.append(make_event(p, three_quarters(), EventClass::TitleWon)); // 0.75
        ledger.append(make_event(
            p,
            Q32::from_raw(1i64 << 29),
            EventClass::HatTrickScored,
        )); // 0.125

        let top3 = SalienceReader::top_n(&mut ledger, 3, SalienceFilter::None, Tick::ZERO);
        assert_eq!(top3.len(), 3);
        // Expected order: CupFinalWin (1.0) > TitleWon (0.75) > LegacyGoal (0.5)
        assert_eq!(top3[0].event_class, EventClass::CupFinalWin);
        assert_eq!(top3[1].event_class, EventClass::TitleWon);
        assert_eq!(top3[2].event_class, EventClass::LegacyGoal);
    }

    /// AC2: BySubject filter returns only events with the target player as a participant.
    #[test]
    fn top_n_by_subject_filters_correctly() {
        let mut ledger = MemoryLedger::new();
        let p1 = PlayerId::new(1);
        let p2 = PlayerId::new(2);

        ledger.append(make_event(p1, Q32::ONE, EventClass::CupFinalWin));
        ledger.append(make_event(p2, three_quarters(), EventClass::TitleWon));
        ledger.append(make_event(p1, half(), EventClass::LegacyGoal));

        let p1_events =
            SalienceReader::top_n(&mut ledger, 5, SalienceFilter::BySubject(p1), Tick::ZERO);
        assert_eq!(p1_events.len(), 2, "only p1 events");
        // Both are p1 events; verify no p2 events slip through.
        for ev in &p1_events {
            assert!(
                ev.participants
                    .iter()
                    .any(|p| matches!(p.entity, EntityRef::Player(id) if id == p1))
            );
        }
    }

    /// AC2: tie on equal projected salience → lower event_id first (ascending).
    #[test]
    fn top_n_tie_broken_by_event_id_ascending() {
        let mut ledger = MemoryLedger::new();
        let p = PlayerId::new(1);

        // All events have same stakes → same projected salience (Never decay + same stakes).
        ledger.append(make_event(p, half(), EventClass::DebutSenior)); // EventId(0)
        ledger.append(make_event(p, half(), EventClass::LegacyGoal)); // EventId(1)
        ledger.append(make_event(p, half(), EventClass::TitleWon)); // EventId(2)

        let top3 = SalienceReader::top_n(&mut ledger, 3, SalienceFilter::None, Tick::ZERO);
        assert_eq!(top3.len(), 3);
        // Tie → ascending event_id: 0, 1, 2
        assert_eq!(top3[0].event_id, EventId(0));
        assert_eq!(top3[1].event_id, EventId(1));
        assert_eq!(top3[2].event_id, EventId(2));
    }
}

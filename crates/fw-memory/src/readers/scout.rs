//! `ScoutReader` — per-scout perceived event subset about a target player.
//!
//! Returns the projected-salience-ranked event list about a target player,
//! as visible to a scout. At T3-2 this is **structural only** — the
//! per-archetype omission/distortion that makes scouting uncertain (a scout
//! with a "direct-pressing" archetype bias underweights technical events and
//! overweights physical ones) lands at T3-5 (scout-uncertainty system).
//!
//! ## T3-2 structural scope
//!
//! - Returns ALL events that touch `target_player` in any participant role
//!   (not just Subject — scouts see events where the player was a Witness,
//!   Counterparty, etc.).
//! - Sorted by projected salience descending; ties by event_id ascending.
//! - No omission or distortion — archetype bias is T3-5.
//!
//! ## Cadence
//!
//! Per scout report (on demand).
//!
//! ## DEFERRED (T3-5)
//!
//! Per-archetype omission and distortion are intentionally absent. A scout
//! with a regional or archetype blind spot may omit events or rescale their
//! projected salience based on the scout's `archetype_bias` field. That
//! layer is T3-5 (scout-uncertainty system). Until then, every scout sees
//! the same unfiltered view of the target's event history.

use fw_core::{PlayerId, Tick};

use crate::event::{EntityRef, MemoryEvent};
use crate::ledger::MemoryLedger;
use crate::readers::project_salience;

/// Stateless scout reader.
///
/// At T3-2 this is structural only — no archetype-bias distortion.
/// See module-level doc for the T3-5 deferral.
pub struct ScoutReader;

impl ScoutReader {
    /// Events about `target_player` as perceived by a scout.
    ///
    /// Returns all events where `target_player` appears in any participant
    /// role. Sorted by projected salience descending; ties broken by
    /// `event_id` ascending.
    ///
    /// Returns an empty `Vec` for unknown players (no events in the ledger
    /// that touch the player).
    ///
    /// ## T3-5 deferral
    ///
    /// Archetype-bias distortion (per-scout omission / salience rescaling)
    /// is deliberately absent at T3-2. The scout sees the full, unfiltered
    /// event history.
    #[must_use]
    pub fn perceived_events(
        ledger: &mut MemoryLedger,
        target_player: PlayerId,
        now_tick: Tick,
    ) -> Vec<&MemoryEvent> {
        let mut candidates: Vec<&MemoryEvent> = ledger
            .events
            .iter()
            .filter(|e| {
                e.participants
                    .iter()
                    .any(|p| matches!(p.entity, EntityRef::Player(id) if id == target_player))
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
    use fw_core::{MatchId, PlayerId, Q32, Tick};

    fn make_event_for_player(
        player: PlayerId,
        role: ParticipantRole,
        class: EventClass,
        stakes: Q32,
    ) -> MemoryEvent {
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
                role,
                entity: EntityRef::Player(player),
            }],
            event_class: class,
            stakes,
            emotion: Emotion::Neutral,
            consequence: vec![Consequence::None],
            callback_eligibility: CallbackEligibility::Immediate,
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

    /// AC5: perceived_events returns only events touching target_player.
    #[test]
    fn perceived_events_returns_only_target_player_events() {
        let mut ledger = MemoryLedger::new();
        let p1 = PlayerId::new(1);
        let p2 = PlayerId::new(2);

        ledger.append(make_event_for_player(
            p1,
            ParticipantRole::Subject,
            EventClass::LegacyGoal,
            Q32::ONE,
        ));
        ledger.append(make_event_for_player(
            p2,
            ParticipantRole::Subject,
            EventClass::TitleWon,
            three_quarters(),
        ));
        ledger.append(make_event_for_player(
            p1,
            ParticipantRole::Subject,
            EventClass::HatTrickScored,
            half(),
        ));

        let p1_events = ScoutReader::perceived_events(&mut ledger, p1, Tick::ZERO);
        assert_eq!(p1_events.len(), 2, "only p1 events returned");
        for ev in &p1_events {
            assert!(
                ev.participants
                    .iter()
                    .any(|p| matches!(p.entity, EntityRef::Player(id) if id == p1))
            );
        }
    }

    /// AC5: events where target is a non-Subject participant (Witness) are still included.
    #[test]
    fn perceived_events_includes_non_subject_participant_roles() {
        let mut ledger = MemoryLedger::new();
        let p = PlayerId::new(5);

        // Player as Witness — should still be included.
        ledger.append(make_event_for_player(
            p,
            ParticipantRole::Witness,
            EventClass::DerbyControversy,
            half(),
        ));
        // Player as Subject.
        ledger.append(make_event_for_player(
            p,
            ParticipantRole::Subject,
            EventClass::LegacyGoal,
            three_quarters(),
        ));

        let events = ScoutReader::perceived_events(&mut ledger, p, Tick::ZERO);
        assert_eq!(events.len(), 2, "both roles included");
        // Higher salience first: LegacyGoal (0.75) > DerbyControversy (0.5)
        assert_eq!(events[0].event_class, EventClass::LegacyGoal);
        assert_eq!(events[1].event_class, EventClass::DerbyControversy);
    }

    /// AC5: returns empty Vec for an unknown player (no events touching them).
    #[test]
    fn perceived_events_returns_empty_for_unknown_player() {
        let mut ledger = MemoryLedger::new();
        let p1 = PlayerId::new(1);
        let unknown = PlayerId::new(99);

        ledger.append(make_event_for_player(
            p1,
            ParticipantRole::Subject,
            EventClass::LegacyGoal,
            Q32::ONE,
        ));

        let events = ScoutReader::perceived_events(&mut ledger, unknown, Tick::ZERO);
        assert!(events.is_empty(), "unknown player returns empty");
    }
}

//! `CoachReader` — per-own-club-player event signals for selection and training.
//!
//! Returns a `BTreeMap<PlayerId, Vec<&MemoryEvent>>` covering every player
//! at `club` who has ledger events, with each player's events sorted by
//! projected salience descending (ties: event_id ascending).
//!
//! ## T3-2 structural scope
//!
//! The coach sees all events about own-club players, ordered by current
//! projected salience. This is the structural foundation for T3-4's
//! breakthrough-readiness and regressive-risk computation.
//!
//! ## Cadence
//!
//! Once per training-week cycle.
//!
//! ## DEFERRED (T3-4)
//!
//! Breakthrough-readiness and regressive-risk computation are intentionally
//! absent at T3-2. The T3-4 coach-AI layer will consume this reader's output
//! to compute `signature_readiness` and `regressive_pressure` per
//! (player, attribute_family) pair. Until T3-4 ships, `CoachReader` returns
//! the plain sorted event list without any readiness or pressure signals.
//!
//! ## Club filter
//!
//! Only players who appear as Subject in an event where `club_id` is also a
//! participant are included. The `by_club` index gives the O(log n) club
//! pre-filter; the Subject player is extracted from each event's participant
//! list. Events with no Subject player participant are skipped.
//!
//! A player at club D is NOT included when querying club C, even if the
//! event mentions both clubs.

use std::collections::BTreeMap;

use fw_core::{ClubId, PlayerId, Tick};

use crate::event::{EntityRef, EventId, MemoryEvent, ParticipantRole};
use crate::ledger::MemoryLedger;
use crate::readers::project_salience;

/// Stateless coach reader.
///
/// At T3-2 this is structural only — no breakthrough-readiness or
/// regressive-risk computation. See module-level doc for the T3-4 deferral.
pub struct CoachReader;

impl CoachReader {
    /// Per-own-club-player event lists for coach AI decisions.
    ///
    /// Returns a `BTreeMap<PlayerId, Vec<&MemoryEvent>>` where:
    /// - Keys are `PlayerId`s of players who are `Subject` in events that
    ///   also involve `club` as a club participant.
    /// - Values are the player's events sorted by projected salience
    ///   descending, ties by event_id ascending.
    ///
    /// Players at other clubs are absent from the map. Players at `club`
    /// with no events are absent (empty map entry is not created).
    ///
    /// ## T3-4 deferral
    ///
    /// Breakthrough-readiness (`signature_readiness`) and regressive-risk
    /// (`regressive_pressure`) per (player, attribute_family) are NOT computed
    /// here. That layer is T3-4 (coach AI system).
    #[must_use]
    pub fn player_signals(
        ledger: &mut MemoryLedger,
        club: ClubId,
        now_tick: Tick,
    ) -> BTreeMap<PlayerId, Vec<&MemoryEvent>> {
        // Get all EventIds where this club participates.
        let club_event_ids: Vec<EventId> = ledger.by_club(club).to_vec();

        // Build a BTreeMap: PlayerId → collected &MemoryEvent refs.
        // We need to collect by player first, then sort each player's list.
        let mut player_event_ids: BTreeMap<PlayerId, Vec<EventId>> = BTreeMap::new();

        for eid in club_event_ids {
            if let Some(event) = ledger.get_by_id(eid) {
                // Find the Subject player participant in this event.
                let subject_player = event.participants.iter().find_map(|p| {
                    if p.role == ParticipantRole::Subject
                        && let EntityRef::Player(pid) = p.entity
                    {
                        Some(pid)
                    } else {
                        None
                    }
                });

                if let Some(pid) = subject_player {
                    player_event_ids.entry(pid).or_default().push(eid);
                }
            }
        }

        // Now resolve EventIds to &MemoryEvent and sort each player's list.
        // We need to do this as a second pass to avoid conflicting borrows.
        let mut result: BTreeMap<PlayerId, Vec<&MemoryEvent>> = BTreeMap::new();

        for (pid, eids) in &player_event_ids {
            let mut events: Vec<&MemoryEvent> = eids
                .iter()
                .filter_map(|&eid| ledger.get_by_id(eid))
                .collect();

            events.sort_by(|a, b| {
                let sa = project_salience(a, now_tick);
                let sb = project_salience(b, now_tick);
                sb.cmp(&sa).then_with(|| a.event_id.cmp(&b.event_id))
            });

            result.insert(*pid, events);
        }

        result
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

    fn make_player_club_event(
        player: PlayerId,
        club: ClubId,
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
    fn quarter() -> Q32 {
        Q32::from_raw(1i64 << 30)
    }

    /// AC6: player_signals BTreeMap has exactly the club-C player keys.
    #[test]
    fn player_signals_includes_only_club_players() {
        let mut ledger = MemoryLedger::new();
        let club_c = ClubId::new(1);
        let club_d = ClubId::new(2);
        let p1 = PlayerId::new(10);
        let p2 = PlayerId::new(11);
        let p3 = PlayerId::new(12);
        let p4_other_club = PlayerId::new(99);

        ledger.append(make_player_club_event(
            p1,
            club_c,
            EventClass::LegacyGoal,
            Q32::ONE,
        ));
        ledger.append(make_player_club_event(
            p2,
            club_c,
            EventClass::TitleWon,
            half(),
        ));
        ledger.append(make_player_club_event(
            p3,
            club_c,
            EventClass::CupFinalWin,
            three_quarters(),
        ));
        // Player at club D — should NOT appear in club C results.
        ledger.append(make_player_club_event(
            p4_other_club,
            club_d,
            EventClass::LegacyGoal,
            Q32::ONE,
        ));

        let signals = CoachReader::player_signals(&mut ledger, club_c, Tick::ZERO);

        assert_eq!(signals.len(), 3, "exactly 3 club-C players");
        assert!(signals.contains_key(&p1));
        assert!(signals.contains_key(&p2));
        assert!(signals.contains_key(&p3));
        assert!(
            !signals.contains_key(&p4_other_club),
            "club-D player absent"
        );
    }

    /// AC6: each player's Vec is projected-salience-ordered.
    #[test]
    fn player_signals_events_are_salience_ordered() {
        let mut ledger = MemoryLedger::new();
        let club = ClubId::new(1);
        let p = PlayerId::new(5);

        // Three events for player p at different salience levels.
        ledger.append(make_player_club_event(
            p,
            club,
            EventClass::DebutSenior,
            quarter(),
        )); // 0.25
        ledger.append(make_player_club_event(
            p,
            club,
            EventClass::LegacyGoal,
            Q32::ONE,
        )); // 1.0
        ledger.append(make_player_club_event(
            p,
            club,
            EventClass::HatTrickScored,
            three_quarters(),
        )); // 0.75

        let signals = CoachReader::player_signals(&mut ledger, club, Tick::ZERO);

        let p_events = signals.get(&p).expect("player must be in map");
        assert_eq!(p_events.len(), 3);
        // Order: LegacyGoal (1.0) > HatTrickScored (0.75) > DebutSenior (0.25)
        assert_eq!(p_events[0].event_class, EventClass::LegacyGoal);
        assert_eq!(p_events[1].event_class, EventClass::HatTrickScored);
        assert_eq!(p_events[2].event_class, EventClass::DebutSenior);
    }

    /// AC6: club-D player is absent from club-C query.
    #[test]
    fn player_signals_excludes_other_club_players() {
        let mut ledger = MemoryLedger::new();
        let club_c = ClubId::new(1);
        let club_d = ClubId::new(2);
        let p_d = PlayerId::new(77);

        // Only events for club D.
        ledger.append(make_player_club_event(
            p_d,
            club_d,
            EventClass::TitleWon,
            Q32::ONE,
        ));
        ledger.append(make_player_club_event(
            p_d,
            club_d,
            EventClass::CupFinalWin,
            half(),
        ));

        let signals = CoachReader::player_signals(&mut ledger, club_c, Tick::ZERO);
        assert!(
            signals.is_empty(),
            "club-C map must be empty when no club-C events exist"
        );
    }
}

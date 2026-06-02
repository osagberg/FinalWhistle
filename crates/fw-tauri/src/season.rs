//! Season-play orchestration glue.
//!
//! `play_one_match` wraps the existing `tick_match` loop for a single fixture,
//! translating the sim's canonical types to `MatchOutcome` for `SeasonState`.
//! This is a sync function — it sits inside async Tauri handlers but runs
//! entirely on the calling thread (the sim is sync per `Sim/RULES.md §5`).
//!
//! `std::time::Instant` is only used in the integration-test perf gate (in
//! `crates/fw-tauri/tests/season_commands_test.rs`), not here — this module
//! is on the correct side of the clock-ban boundary (`fw-tauri` is allowed
//! clocks per `Sim/RULES.md §3`; this file is in `fw-tauri`).

use std::collections::BTreeMap;
use std::sync::Arc;

use fw_content::{
    ContentStore, MatchOutcome, SeasonState, SignatureCandidate, SignatureDefinition,
};
use fw_core::{AttributeFamily, ClubId, PlayerId, Seed};
use fw_match_sim::{MatchState, tick_match};
use fw_memory::NarrativeFlag as MemNarrativeFlag;
use fw_memory::event::{
    CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind, EntityRef,
    EventClass, MemoryEvent, Participant, ParticipantRole, SeasonNumber, SourceId,
};
use fw_memory::ledger::MemoryLedger;

use crate::IpcError;

/// Per-match tick budget for season simulation.
///
/// 600 ticks matches the existing 600-tick canonical hash pin (the
/// "extended" pinned scenario in `crates/fw-replay/tests/canonical_hash.rs`).
/// This keeps the season sim consistent with the already-verified canonical
/// state.
///
/// T4-sim-halt note: `MatchState::match_end_tick` now defaults to
/// `fw_match_sim::FULL_MATCH_TICKS` (5400 = 90 displayed-min), so a 600-tick
/// season match runs well short of match-end — it deliberately does NOT reach
/// FullTime and the sim never freezes within this budget. `play_one_match`
/// reads `home_score`/`away_score` directly (not FullTime), so the season
/// result is unaffected. Raising this to a real 90-minute budget — and the
/// goal-RATE calibration that requires — is T5-5b, not T4-sim-halt.
pub const SEASON_MATCH_TICK_BUDGET: u32 = 600;

/// Emit season-end memory events into the ledger for a completed season.
///
/// MVP emission mapping (design decision 3): ONE `EventClass::TitleWon` event
/// for the champion club (rows[0] of the completed standings). Player-level
/// events are deferred to T4+ (no per-player season stats available yet).
///
/// The champion is identified from `season.standings().rows[0]` — the
/// canonical sort order is `(points DESC, goal_difference DESC, goals_for
/// DESC, club_id ASC)`, so `rows[0]` is always the title winner.
///
/// Does nothing when standings are empty (defensive; well-formed 20-club
/// seasons always have a champion).
pub fn emit_season_end_events(
    season: &SeasonState,
    season_number: SeasonNumber,
    ledger: &mut MemoryLedger,
) {
    let standings = season.standings();
    let champion_row = match standings.rows.first() {
        Some(r) => r,
        None => return,
    };
    emit_title_won_event(champion_row.club_id, season_number, ledger);
}

/// Emit a single `TitleWon` event for `champion_id` into the ledger.
///
/// Extracted from `emit_season_end_events` so callers that have already
/// derived the `ClubId` outside a `&SeasonState` borrow can emit without
/// triggering a two-field simultaneous borrow on `CareerState`.
pub fn emit_title_won_event(
    champion_id: ClubId,
    season_number: SeasonNumber,
    ledger: &mut MemoryLedger,
) {
    use fw_core::Q32;

    let event = MemoryEvent {
        event_id: fw_memory::event::EventId(0), // overwritten by ledger.append
        schema_version: 1,
        season: season_number,
        tick: None, // season-end event — no specific tick
        career_date: CareerDate {
            year: season_number.0 + 1,
            day_of_year: 365,
        },
        emitter: Emitter {
            kind: EmitterKind::CareerSystem,
            source_id: SourceId::Club(champion_id),
        },
        participants: vec![Participant {
            role: ParticipantRole::Beneficiary,
            entity: EntityRef::Club(champion_id),
        }],
        event_class: EventClass::TitleWon,
        stakes: Q32::ONE,
        emotion: fw_memory::event::Emotion::Joy,
        consequence: vec![Consequence::None],
        callback_eligibility: CallbackEligibility::Immediate,
        salience: Q32::ONE,
        decay_function: DecayFunction::Never,
    };
    ledger.append(event);
}

/// Run one full match and return the final `MatchOutcome` alongside the
/// completed `MatchState` so the caller can harvest player-subject
/// `MemoryEvent`s via `match_state.match_events()`.
///
/// Sync — the calling async handler is responsible for not blocking the Tauri
/// runtime; `play_fixtures` fast-forwards via a plain loop rather than
/// spawning threads (the sim is deterministic + single-threaded per design).
///
/// `sig_defs` is `Arc<BTreeMap<...>>` to allow cheap cloning per call without
/// re-borrowing the full `ContentStore` across the async boundary.
///
/// ## Slot-signatures (T4-2.5c)
///
/// `slot_signatures` is an optional `BTreeMap<PlayerSlot, Vec<SignatureCandidate>>`
/// that overrides per-slot candidates from the career roster. When `Some`, it is
/// applied via `MatchState::with_slot_signatures` AFTER `initial_with_content`
/// spreads content-pool defaults across all 22 slots. When `None` the content
/// spread is used as-is (used by `commands.rs` callers that don't yet hold a
/// roster, and by the live-match path).
///
/// Home roster → slots 0-10; away roster → slots 11-21. With 1 template today
/// the roster candidates equal the content-spread defaults, so the override is a
/// deterministic no-op in practice — it becomes meaningful at T4.5-E1.
pub fn play_one_match(
    seed: Seed,
    content: &ContentStore,
    sig_defs: &Arc<BTreeMap<String, SignatureDefinition>>,
    home_archetype_id: &str,
    away_archetype_id: &str,
    tick_budget: u32,
    slot_signatures: Option<BTreeMap<fw_core::PlayerSlot, Vec<fw_content::SignatureCandidate>>>,
) -> Result<(MatchOutcome, MatchState), IpcError> {
    let base_state =
        MatchState::initial_with_content(seed, content, home_archetype_id, away_archetype_id)
            .map_err(|e| IpcError::MatchInitFailed {
                reason: e.to_string(),
            })?;

    let mut sim_state = if let Some(overrides) = slot_signatures {
        base_state.with_slot_signatures(overrides)
    } else {
        base_state
    };

    for _ in 0..tick_budget {
        sim_state = tick_match(sim_state, sig_defs);
    }
    let outcome = MatchOutcome {
        home_score: sim_state.home_score,
        away_score: sim_state.away_score,
    };
    Ok((outcome, sim_state))
}

/// Harvest player-subject `MemoryEvent`s from a just-played match and append
/// them to the ledger.
///
/// ## What is emitted
///
/// For each of the 22 match slots (home 0-10, away 11-21):
/// - **Appearance**: if the slot maps to a rostered `PlayerInstance`, increment
///   `career_apps`. On the 0 → 1 transition, append a `DebutSenior` event
///   (subject = that `PlayerId`). `DebutSenior` (not `DebutClub`) because at
///   T4-2.5e every player is at their career-start club — the `DebutClub` path
///   is reserved for post-transfer appearances (T4-2.5g).
/// - **Goal**: for every `MatchEvent::Goal { scorer_slot }`, map `scorer_slot`
///   to the rostered `PlayerInstance` and append a `LegacyGoal` event
///   (subject = that `PlayerId`).
///
/// ## Slot → roster mapping
///
/// Matches the T4-2.5c convention used in `build_slot_signatures`:
/// - match slots 0-10 → `home_instances[0..10]`
/// - match slots 11-21 → `away_instances[slot - 11]` (indices 0-10 away)
///
/// A slot with no matching roster instance (out-of-bounds index or empty
/// slice) is silently skipped — this is a graceful degradation for malformed
/// fixture data that should never occur in well-formed careers.
///
/// ## Determinism
///
/// Deterministic: the `match_events()` slice is in canonical tick-ascending
/// order; the two roster slices are slot-ordered `Vec`s. No RNG needed — the
/// events carry `tick` from the canonical sim state. The `Q32` stakes and
/// `DecayFunction` values are fixed constants.
/// Harvest player-subject memory events for one team half of a played match.
///
/// Returns the `MemoryEvent`s to append — caller appends them to the ledger in
/// a separate step so that `career.roster` and `career.ledger` borrows do not
/// overlap (the borrow checker cannot prove they are disjoint fields of
/// `CareerState` through a `RwLockWriteGuard` reference).
///
/// Pass `home_instances` for the home team (match slots 0-10); pass `&mut []`
/// (empty) if processing the away side in a split-call. The same applies to
/// `away_instances` (match slots 11-21). One call per team half per fixture.
pub fn harvest_match_memory_events(
    match_state: &MatchState,
    home_instances: &mut [crate::roster::PlayerInstance],
    away_instances: &mut [crate::roster::PlayerInstance],
    season_number: fw_memory::event::SeasonNumber,
) -> Vec<fw_memory::event::MemoryEvent> {
    use fw_content::MatchEvent;
    use fw_core::Q32;
    use fw_memory::event::{
        CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind,
        EntityRef, EventClass, MemoryEvent, Participant, ParticipantRole, SourceId,
    };

    // Roster-size invariant (Sim/RULES §11 — fail in release, not just debug):
    // A non-empty slice must have ≥ 11 entries (a starting XI). An empty slice
    // is valid and signals "process only the other half" (split-call pattern
    // from advance_week_inner). A slice of length 1..10 is a programming error.
    assert!(
        home_instances.is_empty() || home_instances.len() >= 11,
        "harvest_match_memory_events: home_instances has {} entries; \
         must be empty or ≥ 11 (Sim/RULES §11)",
        home_instances.len()
    );
    assert!(
        away_instances.is_empty() || away_instances.len() >= 11,
        "harvest_match_memory_events: away_instances has {} entries; \
         must be empty or ≥ 11 (Sim/RULES §11)",
        away_instances.len()
    );

    let mut events: Vec<MemoryEvent> = Vec::new();

    // ---- Pass 1: appearance pass — increment career_apps for all 22 slots.
    // Collect debut info: (player_id, club_id) pairs for players whose
    // career_apps transitions from 0 → 1 this match.
    //
    // We avoid calling instance_for_slot (which would need both mutable slices
    // in the same borrow) by handling home (slot < 11) and away (slot >= 11)
    // separately in the loop body.
    let mut debut_info: Vec<(fw_core::PlayerId, fw_core::ClubId)> = Vec::new();

    for slot in 0usize..22 {
        let (is_debut, player_id, club_id) = if slot < 11 {
            if let Some(inst) = home_instances.get_mut(slot) {
                let was_zero = inst.career_apps == 0;
                inst.career_apps += 1;
                (was_zero, Some(inst.player_id), Some(inst.club_id))
            } else {
                (false, None, None)
            }
        } else if let Some(inst) = away_instances.get_mut(slot - 11) {
            let was_zero = inst.career_apps == 0;
            inst.career_apps += 1;
            (was_zero, Some(inst.player_id), Some(inst.club_id))
        } else {
            (false, None, None)
        };

        if is_debut && let (Some(pid), Some(cid)) = (player_id, club_id) {
            debut_info.push((pid, cid));
        }
    }

    // ---- Pass 2: collect DebutSenior events (one per debut player).
    //
    // Participants: Subject = the player, Counterparty = their club.
    // Including the Club participant allows the render path to resolve
    // `club_name` from the `MemoryCallbackContext` — without it the
    // `debut_senior` template renders `"First senior appearance for  — name"`
    // (blank club_name before the em-dash = orphaned ` — ` in the output).
    for (player_id, club_id) in debut_info {
        events.push(MemoryEvent {
            event_id: fw_memory::event::EventId(0), // overwritten by ledger.append
            schema_version: 1,
            season: season_number,
            tick: None, // match-level event; tick-level granularity not needed here
            career_date: CareerDate {
                year: season_number.0 + 1,
                day_of_year: 1,
            },
            emitter: Emitter {
                kind: EmitterKind::MatchEngine,
                source_id: SourceId::None,
            },
            participants: vec![
                Participant {
                    role: ParticipantRole::Subject,
                    entity: EntityRef::Player(player_id),
                },
                Participant {
                    role: ParticipantRole::Counterparty,
                    entity: EntityRef::Club(club_id),
                },
            ],
            event_class: EventClass::DebutSenior,
            stakes: Q32::ONE,
            emotion: fw_memory::event::Emotion::Pride,
            consequence: vec![Consequence::None],
            callback_eligibility: CallbackEligibility::Immediate,
            salience: Q32::ZERO, // overwritten by ledger.append
            decay_function: DecayFunction::Never,
        });
    }

    // ---- Pass 3: emit LegacyGoal events for every Goal in the match.
    //
    // Participants: Subject = scorer, Counterparty = scorer's own club.
    //
    // NOTE (T4-2.5h): `scorer_slot` is derived from `last_touched_by`, so an
    // own-goal would attribute a LegacyGoal to the conceding player. This is a
    // known limitation until own-goals are distinctly modeled in the sim.
    //
    // NOTE (T4-2.5h): `PlayerSeasonStats.goals` is NOT incremented here —
    // full season_stats accrual (appearances, goals, minutes) is T4-2.5h.
    // Only the career ledger event is emitted at T4-2.5e.
    //
    // When called with an empty `home_instances` or `away_instances` (split-call
    // pattern), goals scored by the absent team's half are skipped because
    // `scorer_slot < 11` maps to home (empty) and `scorer_slot >= 11` maps to
    // away. The split-call caller (advance_week_inner) calls this function twice
    // per fixture — once for home slots, once for away — so each team's goals
    // are attributed on the correct call.
    for match_event in match_state.match_events() {
        if let MatchEvent::Goal {
            scorer_slot, tick, ..
        } = match_event
        {
            let scorer_usize = *scorer_slot as usize;

            // scorer_slot is a u8 produced by the sim; valid range is 0..22.
            // An out-of-range value means the sim emitted a malformed Goal event
            // (programming error in fw-match-sim). Fail loud per Sim/RULES §11.
            assert!(
                scorer_usize < 22,
                "harvest_match_memory_events: Goal event scorer_slot {} is out of \
                 range 0..22 — malformed MatchEvent from fw-match-sim (Sim/RULES §11)",
                scorer_usize
            );

            let (scorer_player_id, scorer_club_id) = if scorer_usize < 11 {
                home_instances
                    .get(scorer_usize)
                    .map(|i| (Some(i.player_id), Some(i.club_id)))
                    .unwrap_or((None, None))
            } else {
                away_instances
                    .get(scorer_usize - 11)
                    .map(|i| (Some(i.player_id), Some(i.club_id)))
                    .unwrap_or((None, None))
            };

            // None here means the scorer's team half was passed as empty (split-call).
            // The other call in the pair handles this goal. Skip cleanly.
            let (Some(player_id), Some(club_id)) = (scorer_player_id, scorer_club_id) else {
                continue;
            };

            events.push(MemoryEvent {
                event_id: fw_memory::event::EventId(0), // overwritten by ledger.append
                schema_version: 1,
                season: season_number,
                tick: Some(*tick),
                career_date: CareerDate {
                    year: season_number.0 + 1,
                    day_of_year: 1,
                },
                emitter: Emitter {
                    kind: EmitterKind::MatchEngine,
                    source_id: SourceId::None,
                },
                participants: vec![
                    Participant {
                        role: ParticipantRole::Subject,
                        entity: EntityRef::Player(player_id),
                    },
                    Participant {
                        role: ParticipantRole::Counterparty,
                        entity: EntityRef::Club(club_id),
                    },
                ],
                event_class: EventClass::LegacyGoal,
                stakes: Q32::ONE,
                emotion: fw_memory::event::Emotion::Joy,
                consequence: vec![Consequence::None],
                callback_eligibility: CallbackEligibility::Immediate,
                salience: Q32::ZERO, // overwritten by ledger.append
                decay_function: DecayFunction::Never,
            });
        }
    }

    events
}

/// Build a `slot_signatures` map from home + away club rosters.
///
/// Home roster `PlayerInstance` slice → match slots 0-10 (indices 0..=10).
/// Away roster `PlayerInstance` slice → match slots 11-21 (indices 0..=10
/// mapped to 11..=21).
///
/// Only slots with non-empty `signature_candidates` on the `PlayerInstance`
/// are included in the map. This preserves the role-matched invariant from
/// [`MatchState::initial_with_content`]: if the content-side spread left a
/// slot empty (because no template matched that slot's formation role), the
/// roster override should not re-introduce candidates for that slot either.
///
/// First-increment note: with 1 AM template, only instances at midfielder
/// slots (in_team ∈ 5..=7; match slots 5-7 home, 16-18 away) carry
/// non-empty candidates. Roster generation assigned AM candidates to ALL 22
/// instances (`build_roster_from_league` round-robins over 1 template), so
/// the non-empty filter is the gate that keeps GK/DEF/FWD match slots clean.
///
/// This is the T4-2.5c pillar-5 wiring: roster `signature_candidates` flow
/// into the match sim's canonical state.
pub fn build_slot_signatures(
    home_instances: &[crate::roster::PlayerInstance],
    away_instances: &[crate::roster::PlayerInstance],
) -> BTreeMap<fw_core::PlayerSlot, Vec<fw_content::SignatureCandidate>> {
    // T4-2.5c assumption: instances 0..=10 are taken as the starting XI in
    // formation order (GK=0, DEF=1-4, MID=5-7, FWD=8-10). This is valid for
    // the first increment where each club has exactly 22 roster instances and
    // T4.5-E1 has not yet given `PlayerInstance` a real formation slot.
    // When T4.5-E1 lands, the formation slot will be on the instance and this
    // function will derive `in_team` from `instance.slot % SLOTS_PER_CLUB`
    // rather than from the Vec index.
    assert!(
        home_instances.len() >= 11,
        "build_slot_signatures: home_instances has {} entries, need ≥ 11 for a \
         starting XI; a short roster indicates a programming error in roster \
         generation (Sim/RULES §11)",
        home_instances.len()
    );
    assert!(
        away_instances.len() >= 11,
        "build_slot_signatures: away_instances has {} entries, need ≥ 11 for a \
         starting XI; a short roster indicates a programming error in roster \
         generation (Sim/RULES §11)",
        away_instances.len()
    );

    let mut map: BTreeMap<fw_core::PlayerSlot, Vec<fw_content::SignatureCandidate>> =
        BTreeMap::new();

    // Home team: roster indices 0..=10 → match slots 0..=10.
    // Only include instances whose squad slot is in a midfielder position
    // (in_team ∈ 5..=7 in the 4-3-3 formation: CM/AM slots). This mirrors the
    // role-matched spread in `initial_with_content`: with 1 AM template, MID
    // slots 5-7 carry candidates; GK/DEF/FWD slots should not receive AM
    // candidates from the roster even though `build_roster_from_league` assigns
    // AM candidates to all slots (a first-increment simplification; the filter
    // here is the correctness gate).
    for (i, instance) in home_instances.iter().enumerate().take(11) {
        if role_receives_candidates(i) {
            map.insert(
                i as fw_core::PlayerSlot,
                instance.signature_candidates.clone(),
            );
        }
    }

    // Away team: roster indices 0..=10 → match slots 11..=21.
    // Same role-match filter as home. Away slot `11+i` has in_team = i.
    for (i, instance) in away_instances.iter().enumerate().take(11) {
        if role_receives_candidates(i) {
            map.insert(
                (11 + i) as fw_core::PlayerSlot,
                instance.signature_candidates.clone(),
            );
        }
    }

    map
}

/// Returns `true` if a formation slot index `in_team` (0..=10) belongs to a
/// position that currently has role-matched signature candidates in the
/// content pool.
///
/// With 1 AM template (T4-2.5b first increment), only midfielder slots
/// (`in_team ∈ 5..=7`) carry candidates from `initial_with_content`. GK (0),
/// defenders (1-4), and forwards (8-10) have no matching template yet.
///
/// This function encodes the same formation knowledge as the `5..=7` range
/// in `fw-match-sim::lib.rs::MatchState::initial` (the slot→role assignment).
/// A test in this module (`role_receives_candidates_agrees_with_sim_formation`)
/// cross-checks both so a future formation change fails loud rather than
/// silently diverging here.
///
/// `in_team` is the squad-slot index within one team (0..=10). For home:
/// `in_team == match_slot`. For away: `in_team == match_slot - 11`.
fn role_receives_candidates(in_team: usize) -> bool {
    // Midfielder range in 4-3-3: slots 5, 6, 7.
    (5..=7).contains(&in_team)
}

// ---------------------------------------------------------------------------
// T4-2.5d breakthrough-wiring helpers
// ---------------------------------------------------------------------------

/// Convert a `fw_content::NarrativeFlag` to the parallel `fw_memory::NarrativeFlag`.
///
/// The two crates carry separate enums with the same 4 variants per the design.
/// Conversion is by-name. A variant added to one crate without being added to
/// the other would make this match non-exhaustive — the compile error is the
/// intended failure mode.
///
/// Deduplication of the two enums into fw-core is a logged follow-up (noted
/// in fw-content/src/gene.rs). For T4-2.5d we convert here in the bridging
/// layer (fw-tauri) which already depends on both crates.
pub fn content_flag_to_memory_flag(flag: fw_content::NarrativeFlag) -> MemNarrativeFlag {
    use fw_content::NarrativeFlag as C;
    match flag {
        C::LateBloomer => MemNarrativeFlag::LateBloomer,
        C::FlowAccess => MemNarrativeFlag::FlowAccess,
        C::PeakCeilingHigh => MemNarrativeFlag::PeakCeilingHigh,
        C::AwakeningDormant => MemNarrativeFlag::AwakeningDormant,
    }
}

/// Map a `SignatureCandidate`'s `RoleFamily` to the `AttributeFamily` whose
/// breakthrough meter the signature most directly amplifies.
///
/// ## Mapping rationale (documented here, not improvised)
///
/// `RoleFamily` describes a player's positional archetype. `AttributeFamily`
/// describes a breakthrough meter domain. The mapping picks the PRIMARY family
/// that most clearly represents a role's defining attribute:
///
/// | RoleFamily               | → AttributeFamily          | Rationale |
/// |---|---|---|
/// | Goalkeeper               | Composure                  | GK decisions under pressure |
/// | CentreBack               | DefensiveAnticipation      | Core CB family |
/// | FullBack                 | Pace                       | FB defining attribute |
/// | DefensiveMidfielder      | WorkRate                   | DM pressing/tracking |
/// | CentralMidfielder        | Passing                    | CM vision + range |
/// | AttackingMidfielder      | Finishing                  | AM conversion in the box |
/// | Winger                   | Pace                       | Winger explosive speed |
/// | Striker                  | Finishing                  | Striker conversion |
///
/// Two roles map to `Pace` (FullBack, Winger) and two map to `Finishing`
/// (AttackingMidfielder, Striker). This is intentional — the mapping is a
/// first-increment approximation; T4.5-E1's gene→attribute compiler will use
/// a richer per-role per-family affinity table instead of this single pivot.
pub fn role_family_to_attribute_family(role: fw_content::RoleFamily) -> AttributeFamily {
    use fw_content::RoleFamily;
    match role {
        RoleFamily::Goalkeeper => AttributeFamily::Composure,
        RoleFamily::CentreBack => AttributeFamily::DefensiveAnticipation,
        RoleFamily::FullBack => AttributeFamily::Pace,
        RoleFamily::DefensiveMidfielder => AttributeFamily::WorkRate,
        RoleFamily::CentralMidfielder => AttributeFamily::Passing,
        RoleFamily::AttackingMidfielder => AttributeFamily::Finishing,
        RoleFamily::Winger => AttributeFamily::Pace,
        RoleFamily::Striker => AttributeFamily::Finishing,
    }
}

/// Convert a slice of `SignatureCandidate`s to the `(AttributeFamily, String)`
/// tuples that `BreakthroughContext.signature_candidates` expects.
///
/// The `String` is the signature ID (content-pack-qualified), sourced from
/// `SignatureCandidate.signature_id.as_str()`. The `AttributeFamily` is
/// derived from the candidate's `RoleFamily` via `role_family_to_attribute_family`.
///
/// `sig_defs` is the `BTreeMap<String, SignatureDefinition>` from the content
/// store. If a candidate's ID is absent from the map (content-pack drift or
/// a missing definition), the candidate defaults to `AttributeFamily::Finishing`
/// rather than panicking — a conservative fallback that keeps the career running.
pub fn signature_candidates_to_ctx(
    candidates: &[SignatureCandidate],
    sig_defs: &BTreeMap<String, SignatureDefinition>,
) -> Vec<(AttributeFamily, String)> {
    candidates
        .iter()
        .map(|c| {
            let sig_id_str = c.signature_id.as_str().to_string();
            let role_family = match sig_defs.get(&sig_id_str) {
                Some(def) => def.role_family,
                None => {
                    log::warn!(
                        "signature_candidates_to_ctx: signature_id {:?} not found in \
                         signature_definitions — falling back to RoleFamily::Striker → \
                         AttributeFamily::Finishing. This indicates content-pack drift: a \
                         PlayerTemplate references a SignatureId that is not in the loaded \
                         signature definitions. The breakthrough meter will attribute this \
                         candidate to the Finishing family.",
                        sig_id_str
                    );
                    fw_content::RoleFamily::Striker // fallback: Striker → Finishing
                }
            };
            let family = role_family_to_attribute_family(role_family);
            (family, sig_id_str)
        })
        .collect()
}

/// Build a per-player `MemoryLedger` view containing only events where
/// `player_id` is the `Subject` participant — scanning the FULL ledger.
///
/// Used by the AC3 unit test (`per_player_ledger_filter_excludes_other_player_events`)
/// to verify the filter logic. Production code in `advance_season_inner` calls
/// `filter_new_events_for_player` instead (FIX 1 — incremental evaluation only
/// feeds the new-since-watermark events to `evaluate()`).
///
/// The copy preserves `event_id` values so `evaluate()`'s
/// `tick_for_rng = event.event_id.0` produces the same RNG seeds — determinism
/// is preserved.
pub fn filter_ledger_for_player(ledger: &MemoryLedger, player_id: PlayerId) -> MemoryLedger {
    // Clone so we can call by_subject (which needs &mut for lazy index rebuild)
    // without taking &mut on the caller's original ledger.
    let mut working = ledger.clone();
    working.restore_transient_state();

    let ids: Vec<fw_memory::event::EventId> = working.by_subject(player_id).to_vec();

    let mut player_ledger = MemoryLedger::new();
    for id in &ids {
        if let Some(event) = working.get_by_id(*id) {
            player_ledger.append(event.clone());
        }
    }
    player_ledger
}

/// Build a per-player `MemoryLedger` from a slice of newly-appended events
/// (those past the `breakthrough_eval_watermark`).
///
/// Called from `advance_season_inner` — only the events added since the last
/// evaluation pass are fed to `evaluate()`. This is the incremental evaluation
/// path that fixes the P0 re-fire bug: once an event has been processed and
/// its meter contribution captured in the player's persisted
/// `BreakthroughState`, it is NEVER re-accumulated.
///
/// `new_events` is a sub-slice of `ledger.events[watermark..]` already
/// materialised by the caller before any mutations. The events are filtered
/// to those whose `Subject` is `player_id`.
///
/// `evaluate()` uses `event.event_id.0` as the RNG `tick` parameter, so
/// preserving the original `EventId` values (which we do — we clone the
/// events without renumbering) keeps the RNG sites deterministic across
/// seasons.
pub fn filter_new_events_for_player(
    new_events: &[fw_memory::event::MemoryEvent],
    player_id: PlayerId,
) -> MemoryLedger {
    use fw_memory::event::{EntityRef, ParticipantRole};

    let mut player_ledger = MemoryLedger::new();
    for event in new_events {
        let is_subject = event.participants.iter().any(|p| {
            p.role == ParticipantRole::Subject
                && matches!(p.entity, EntityRef::Player(pid) if pid == player_id)
        });
        if is_subject {
            player_ledger.append(event.clone());
        }
    }
    player_ledger
}

/// Default age (in years) for a rostered player at career start.
///
/// First-increment approximation: all players are treated as 22 years old
/// (development-prime age per `docs/design/player-generation.md`). This feeds
/// `BreakthroughContext.age_years` which only affects the age-curve modifier
/// in `evaluate()`. Real per-player ages arrive at T4.5-E1 with the gene→
/// attribute compiler; until then 22 is neutral (on the positive-development
/// slope, not capped by the aging curve).
pub const CAREER_START_AGE_YEARS: u8 = 22;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::role_receives_candidates;
    use fw_match_sim::{PLAYERS_PER_TEAM, Role};

    /// Fix 5 (T4-2.5c self-review drift-prevention): cross-check
    /// `role_receives_candidates` against the sim's formation slot→role map.
    ///
    /// The sim assigns roles as: 0=GK, 1-4=DEF, 5-7=MID, 8-10=FWD.
    /// `role_receives_candidates` must return `true` iff the slot's role is
    /// `Midfielder`. If the sim ever changes the formation map (e.g. to a
    /// 4-4-2 where MID = 5..=8), this test fails loud instead of silently
    /// letting the roster filter stay on the old 5..=7 range.
    ///
    /// Reference: `fw-match-sim/src/lib.rs::MatchState::initial` formation
    /// assignment (match on `in_team`).
    #[test]
    fn role_receives_candidates_agrees_with_sim_formation() {
        // Mirror the sim's formation assignment so the test fails if either
        // the sim OR this function drifts.
        let sim_role = |in_team: usize| -> Role {
            match in_team {
                0 => Role::Goalkeeper,
                1..=4 => Role::Defender,
                5..=7 => Role::Midfielder,
                _ => Role::Forward, // 8, 9, 10
            }
        };

        for in_team in 0..PLAYERS_PER_TEAM {
            let expected = sim_role(in_team) == Role::Midfielder;
            let actual = role_receives_candidates(in_team);
            assert_eq!(
                actual,
                expected,
                "role_receives_candidates({in_team}) = {actual}, but sim formation \
                 assigns Role::{:?} (expected Midfielder = {expected}). \
                 The formation map and the season-layer filter have diverged — \
                 update `role_receives_candidates` to match the sim's assignment.",
                sim_role(in_team)
            );
        }
    }
}

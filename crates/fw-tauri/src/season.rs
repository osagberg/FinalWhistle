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

use fw_content::{ContentStore, MatchOutcome, SeasonState, SignatureDefinition};
use fw_core::{ClubId, Seed};
use fw_match_sim::{MatchState, tick_match};
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

/// Run one full match and return the final `MatchOutcome`.
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
) -> Result<MatchOutcome, IpcError> {
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
    Ok(MatchOutcome {
        home_score: sim_state.home_score,
        away_score: sim_state.away_score,
    })
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

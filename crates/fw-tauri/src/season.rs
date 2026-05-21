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
/// state. Real 90-minute match realism is deferred to later work.
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
pub fn play_one_match(
    seed: Seed,
    content: &ContentStore,
    sig_defs: &Arc<BTreeMap<String, SignatureDefinition>>,
    home_archetype_id: &str,
    away_archetype_id: &str,
    tick_budget: u32,
) -> Result<MatchOutcome, IpcError> {
    let mut sim_state =
        MatchState::initial_with_content(seed, content, home_archetype_id, away_archetype_id)
            .map_err(|e| IpcError::MatchInitFailed {
                reason: e.to_string(),
            })?;
    for _ in 0..tick_budget {
        sim_state = tick_match(sim_state, sig_defs);
    }
    Ok(MatchOutcome {
        home_score: sim_state.home_score,
        away_score: sim_state.away_score,
    })
}

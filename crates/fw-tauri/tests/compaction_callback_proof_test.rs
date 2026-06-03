//! T4-2.5L — cross-decade callback proof + career-end event tests.
//!
//! ## D2: cross-decade callback proof
//!
//! Proves that a high-stakes season-1 `DebutSenior` event injected before an
//! 8-season run:
//! (a) is still physically present in the ledger after compaction (tick == None,
//!     but the event row is not deleted — compaction nulls tick only);
//! (b) still renders non-empty, non-fallback callback prose via
//!     `get_player_detail_inner` in season 8.
//!
//! The injection is deterministic (known roster player P = lowest-PlayerId at
//! slot 0 of the lowest ClubId). The run is deterministic (same seed, same
//! content). This is `advance_n_seasons(&state, 8)` — season-1 events satisfy
//! `season.0 + 5 = 1 + 5 = 6 <= 8`, so they are compacted.
//!
//! ## D1 check: RegressiveCollapse at career end
//!
//! Proves that after 10 complete seasons, the ledger contains at least one
//! `RegressiveCollapse` event (emitted by the terminal-season wiring).

use std::path::PathBuf;

use fw_core::{PlayerId, Q32, Seed, Tick};
use fw_memory::event::{
    CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind, EntityRef,
    EventClass, MemoryEvent, Participant, ParticipantRole, SeasonNumber, SourceId,
};
use fw_tauri::commands::{advance_season_inner, get_player_detail_inner, play_fixtures_inner};
use fw_tauri::season::CAREER_END_SEASON;
use fw_tauri::state::AppState;

fn workspace_content_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

fn test_state_with_seed(seed: u64) -> AppState {
    AppState::new_with_career_seed(&workspace_content_path(), Seed::from_u64(seed))
        .expect("AppState::new_with_career_seed in compaction_callback_proof_test")
}

/// Advance through N complete seasons.
fn advance_n_seasons(state: &AppState, n: u32) {
    for _ in 0..n {
        play_fixtures_inner(state).expect("play_fixtures_inner");
        advance_season_inner(state).expect("advance_season_inner");
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a `DebutSenior` memory event for `player_id` in season 1.
///
/// Uses `stakes = Q32::ONE` and `DecayFunction::Never` so the event's projected
/// salience is constant across all later seasons — it will always rank in the
/// top-5 for this player regardless of what else accrues.
fn make_debut_event(player_id: PlayerId) -> MemoryEvent {
    MemoryEvent {
        event_id: fw_memory::event::EventId(0), // overwritten by ledger.append
        schema_version: 1,
        season: SeasonNumber(1), // season 1 → will be compacted by season 8
        tick: Some(Tick::ZERO),
        career_date: CareerDate {
            year: 2,
            day_of_year: 42,
        },
        emitter: Emitter {
            kind: EmitterKind::MatchEngine,
            source_id: SourceId::None,
        },
        participants: vec![Participant {
            role: ParticipantRole::Subject,
            entity: EntityRef::Player(player_id),
        }],
        event_class: EventClass::DebutSenior,
        stakes: Q32::ONE,
        emotion: fw_memory::event::Emotion::Pride,
        consequence: vec![Consequence::None],
        callback_eligibility: CallbackEligibility::Immediate,
        salience: Q32::ONE, // overwritten by ledger.append via compute_salience
        decay_function: DecayFunction::Never,
    }
}

// ---------------------------------------------------------------------------
// D2(a+b): cross-decade callback survives compaction
// ---------------------------------------------------------------------------

/// A season-1 `DebutSenior` event for a known roster player P:
/// - is still present in the ledger after 8 seasons (events are never deleted).
/// - has `tick == None` (compacted: season.0 + 5 = 6 <= 8).
/// - produces non-empty, non-fallback callback prose in `get_player_detail_inner`.
///
/// Player P is the slot-0 instance at the lowest ClubId in the roster
/// (deterministic under `BTreeMap` ordering + the roster's slot-ordered Vec).
#[test]
fn cross_decade_callback_survives_compaction() {
    let state = test_state_with_seed(0xfeed_beef_cafe_fade);

    // Resolve player P: lowest ClubId → slot 0 (index 0 in the Vec).
    let (player_p_id, player_p_raw) = {
        let career = state.career().read().expect("career read lock");
        let first_club_id = *career
            .roster
            .keys()
            .next()
            .expect("roster must have at least one club");
        let instances = career
            .roster
            .get(&first_club_id)
            .expect("first club in roster");
        let inst = instances
            .first()
            .expect("club roster must have at least one instance");
        (inst.player_id, inst.player_id.raw())
    };

    // Inject a season-1 DebutSenior for P.
    let injected_event_id = {
        let mut career = state.career().write().expect("career write lock");
        career.ledger.append(make_debut_event(player_p_id))
    };

    // Advance 8 seasons.
    advance_n_seasons(&state, 8);

    // (a) The injected event is still in the ledger, with tick == None
    //     (compacted: season 1 + 5 = 6 <= 8).
    {
        let career = state.career().read().expect("career read lock");

        let injected = career
            .ledger
            .get_by_id(injected_event_id)
            .expect("injected event must still be present in the ledger after compaction");

        assert_eq!(
            injected.tick, None,
            "injected season-1 event must have tick == None after 8 seasons of compaction \
             (season.0 + 5 = 6 <= 8 satisfies the compaction window)"
        );
    }

    // (b) get_player_detail_inner returns non-empty, non-fallback callbacks.
    //
    // P is a roster player (id >= ROSTER_PLAYER_ID_BASE), so we must pass the
    // raw numeric id form that the roster-path routing branch handles.
    let player_id_str = format!("fwh.core:player_{:05}", player_p_raw);
    let dto = get_player_detail_inner(&player_id_str, &state)
        .expect("get_player_detail_inner must succeed for a rostered player after 8 seasons");

    assert!(
        !dto.memory_callbacks.is_empty(),
        "memory_callbacks must be non-empty for player {:?} (id={}) after 8 seasons; \
         the injected DebutSenior (stakes=ONE, DecayFunction::Never) must be in the top-5",
        player_p_id,
        player_p_raw,
    );

    let fallback = "a notable moment in the career";
    for cb in &dto.memory_callbacks {
        assert_ne!(
            cb.as_str(),
            fallback,
            "memory_callbacks must not contain the error fallback phrase \
             '{}'; got: {:?}",
            fallback,
            dto.memory_callbacks,
        );
        assert!(
            !cb.is_empty(),
            "each memory callback must be non-empty; got empty string in {:?}",
            dto.memory_callbacks,
        );
    }
}

// ---------------------------------------------------------------------------
// D1: RegressiveCollapse emitted at career end (season CAREER_END_SEASON)
// ---------------------------------------------------------------------------

/// After CAREER_END_SEASON complete seasons, the ledger contains at least one
/// `RegressiveCollapse` event.
///
/// This proves the terminal-season wiring in `advance_season_inner` fires when
/// `new_season_num.0 == CAREER_END_SEASON`.
#[test]
fn regressive_collapse_emitted_at_career_end() {
    let state = test_state_with_seed(0xfeed_beef_cafe_fade);

    advance_n_seasons(&state, CAREER_END_SEASON as u32);

    let career = state.career().read().expect("career read lock");

    let has_regressive_collapse = career
        .ledger
        .iter()
        .any(|e| matches!(e.event_class, EventClass::RegressiveCollapse));

    assert!(
        has_regressive_collapse,
        "after {} seasons a RegressiveCollapse event must be in the ledger \
         (terminal-season career-end wiring, DECISIONS 2026-06-03 T4-2.5L D1); \
         ledger has {} events total",
        CAREER_END_SEASON,
        career.ledger.len(),
    );

    // Also confirm the season number advanced to CAREER_END_SEASON (sanity check
    // that advance_n_seasons ran the right number of iterations).
    assert_eq!(
        career.season_number.0, CAREER_END_SEASON,
        "career season_number must equal CAREER_END_SEASON after {} advances",
        CAREER_END_SEASON,
    );
}

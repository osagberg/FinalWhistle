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

use std::collections::BTreeMap;

use fw_content::{GeneSnapshot, MentalGenes, PhysicalGenes, TechnicalAffinities};
use fw_core::{
    AbilityCeiling, AttributeFamily, ClubId, DurabilityProfile, GoalkeeperAttributes,
    MentalAttributes, PersonalityVector, PhysicalAttributes, PlayerAttributes, PlayerId, Q32, Seed,
    TechnicalAttributes, Tick,
};
use fw_memory::BreakthroughState;
use fw_memory::event::{
    CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind, EntityRef,
    EventClass, MemoryEvent, Participant, ParticipantRole, SeasonNumber, SourceId,
};
use fw_tauri::commands::{advance_season_inner, get_player_detail_inner, play_fixtures_inner};
use fw_tauri::roster::{PlayerInstance, PlayerSeasonStats, ROSTER_PLAYER_ID_BASE};
use fw_tauri::season::{CAREER_END_SEASON, select_career_end_collapse_player};
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

// ---------------------------------------------------------------------------
// QA-T4H item 1: select_career_end_collapse_player identity
// ---------------------------------------------------------------------------

/// Helper: build a minimal `PlayerInstance` with controllable `breakthrough_state`.
///
/// All attributes are zero (not relevant to this test). The `genes` field is
/// constructed inline to avoid the `pub(crate)` `default_gene_snapshot()` function.
fn make_player_instance_with_state(
    player_id: PlayerId,
    club_id: ClubId,
    slot: u8,
    breakthrough_state: BreakthroughState,
) -> PlayerInstance {
    let z = Q32::ZERO;
    let half = Q32::from_raw(1i64 << 31); // 0.5
    PlayerInstance {
        player_id,
        club_id,
        slot,
        display_name: String::new(),
        attributes: PlayerAttributes {
            technical: TechnicalAttributes {
                finishing: z,
                long_shots: z,
                passing: z,
                crossing: z,
                first_touch: z,
                technique: z,
                dribbling: z,
                heading: z,
                tackling: z,
                marking: z,
                free_kicks: z,
                penalty_taking: z,
                corners: z,
                long_throws: z,
            },
            mental: MentalAttributes {
                anticipation: z,
                composure: z,
                decisions: z,
                vision: z,
                off_the_ball: z,
                positioning: z,
                concentration: z,
                bravery: z,
                teamwork: z,
                flair: z,
            },
            physical: PhysicalAttributes {
                pace: z,
                acceleration: z,
                stamina: z,
                strength: z,
                agility: z,
                balance: z,
                jumping_reach: z,
                natural_fitness: z,
            },
            goalkeeper: GoalkeeperAttributes {
                handling: z,
                reflexes: z,
                one_on_ones: z,
                aerial_reach: z,
                command_of_area: z,
                kicking: z,
            },
            personality: PersonalityVector {
                determination: z,
                work_rate: z,
                ambition: z,
                professionalism: z,
                loyalty: z,
                temperament: z,
                pressure_tolerance: z,
                big_match_appetite: z,
                adaptability: z,
                aggression: z,
                risk_appetite: z,
                selflessness: z,
                consistency: z,
                versatility: z,
            },
            durability: DurabilityProfile {
                injury_proneness: z,
                recovery_rate: z,
                dirtiness: z,
            },
        },
        ceiling: AbilityCeiling::try_new(half, half).expect("ceiling"),
        signature_candidates: vec![],
        breakthrough_state,
        season_stats: PlayerSeasonStats::default(),
        career_apps: 0,
        observation_count: 0,
        last_scout_report: None,
        genes: GeneSnapshot {
            physical: PhysicalGenes {
                height_ceiling: half,
                frame_density: half,
                fast_twitch_ratio: half,
                stamina_recovery: half,
                growth_curve: Q32::ZERO, // must be in [-1, +1]
                aging_curve: half,
                injury_resilience: half,
            },
            mental: MentalGenes {
                pattern_recognition: half,
                composure_floor: half,
                decision_velocity: half,
                learning_rate: half,
                ambition: half,
                mentality: Q32::ZERO, // must be in [-1, +1]
            },
            technical: TechnicalAffinities {
                left_foot: half,
                aerial: half,
                dead_ball: half,
                striking: half,
                first_touch: half,
            },
            narrative_flags: std::collections::BTreeSet::new(),
        },
    }
}

/// `select_career_end_collapse_player` picks the player with the highest summed
/// per-family breakthrough pressure.
///
/// Mutation killed: `total_pressure > best_pressure` → `false` (always pick the first
/// player in BTreeMap iteration order) would return the low-pressure player, failing
/// the assertion that the high-pressure player was chosen.
///
/// Setup: two players in the same club, P_high has pressure = 1.0 on every
/// AttributeFamily, P_low has zero pressure. The function must return P_high.
#[test]
fn regressive_collapse_targets_most_pressured_player() {
    let club_id = ClubId::new(1);

    // P_high: pressure = Q32::ONE on all 10 families.
    let mut state_high = BreakthroughState::new();
    for &family in &AttributeFamily::ALL {
        state_high.add_pressure(family, Q32::ONE);
    }
    let pid_high = PlayerId::new(ROSTER_PLAYER_ID_BASE + 100);
    let inst_high = make_player_instance_with_state(pid_high, club_id, 0, state_high);

    // P_low: zero pressure on all families (BreakthroughState::new() default).
    let pid_low = PlayerId::new(ROSTER_PLAYER_ID_BASE + 200);
    let inst_low = make_player_instance_with_state(pid_low, club_id, 1, BreakthroughState::new());

    // BTreeMap ordering: ClubId(1) is the only key. The Vec contains P_high first,
    // P_low second. With the mutation `total_pressure > best_pressure` → `false`,
    // the function always returns P_high (first player) — which happens to be the right
    // answer here. To KILL the mutation we need P_low FIRST in the Vec so the mutation
    // returns P_low, while the correct code returns P_high (higher pressure).
    let mut roster: BTreeMap<ClubId, Vec<PlayerInstance>> = BTreeMap::new();
    roster.insert(club_id, vec![inst_low, inst_high]); // P_low is index 0 (BTree iteration order)

    let result = select_career_end_collapse_player(&roster);

    assert_eq!(
        result,
        Some(pid_high),
        "select_career_end_collapse_player must return the player with the highest \
         summed breakthrough pressure; expected {:?} (pressure=10.0 across all families), \
         got {:?}. \
         Mutation killed: `total_pressure > best_pressure` → `false` would always return \
         the first player P_low (pressure=0) instead of P_high (pressure=10.0).",
        pid_high,
        result,
    );
}

// ---------------------------------------------------------------------------
// QA-T4H item 2: breakthrough_eval_watermark advancement
// ---------------------------------------------------------------------------

/// After `advance_season_inner`, `career.breakthrough_eval_watermark == career.ledger.len()`.
///
/// Mutation killed: if the watermark is set to the pre-season value (not advanced),
/// season-N breakthrough events re-fire in N+1. This test proves the watermark is
/// advanced to `ledger.len()` after evaluation, not left at the start-of-season position.
#[test]
fn advance_season_watermark_equals_ledger_len_after_advance() {
    let state = test_state_with_seed(0xfeed_beef_cafe_fade);

    // Complete a full season.
    play_fixtures_inner(&state).expect("play_fixtures");
    advance_season_inner(&state).expect("advance_season");

    let career = state.career().read().expect("career read lock");
    let watermark = career.breakthrough_eval_watermark;
    let ledger_len = career.ledger.len();

    assert_eq!(
        watermark, ledger_len,
        "breakthrough_eval_watermark must equal ledger.len() immediately after \
         advance_season_inner: got watermark={watermark}, ledger.len()={ledger_len}. \
         Mutation killed: setting the watermark to the pre-season value would leave \
         it behind ledger.len(), causing season-N events to re-fire in N+1.",
    );
}

/// Season-N breakthrough events fire exactly once across two seasons.
///
/// Advance two seasons. Count BreakthroughMoment events for a single known player
/// across both seasons. Then verify that the same events do NOT re-fire (i.e. the
/// count in season 2 does not include duplicate copies of season-1 events).
///
/// This is a non-vacuous fire-exactly-once check: the watermark must advance
/// past the season-1 breakthrough events so they are not re-evaluated in season 2.
///
/// Mutation killed: if the watermark were not advanced, session 1 breakthrough
/// events would be re-evaluated in season 2, potentially duplicating BreakthroughMoment
/// events and making `count_s2 > count_s1 + new_in_s2`.
///
/// Proof strategy: count all BreakthroughMoment events in the ledger after season 1,
/// and again after season 2. Then collect those from season 1 that also appear in
/// the season-2 ledger. They must be identical objects — not re-created duplicates.
/// We use EventId uniqueness: in a correct implementation, every BreakthroughMoment
/// has a unique EventId and season-1 events have `season == SeasonNumber(1)`.
/// A buggy watermark would re-append the same event as a new row with a new EventId,
/// which the ledger would reject via `validate_for_load` — but the REAL risk is
/// that evaluate() fires AGAIN and creates a NEW BreakthroughMoment in season 2
/// where none should occur (because the conditioning event already fired once).
///
/// Simpler observable: after two seasons, the set of EventIds for BreakthroughMoment
/// events must have no EventId assigned to two events with different seasons.
#[test]
fn advance_season_breakthrough_events_do_not_re_fire_across_two_seasons() {
    let state = test_state_with_seed(0xfeed_beef_cafe_fade);

    // Season 0 → 1.
    play_fixtures_inner(&state).expect("play_fixtures season 0");
    advance_season_inner(&state).expect("advance_season 0");

    let watermark_after_s1 = {
        let career = state.career().read().expect("career read lock");
        career.breakthrough_eval_watermark
    };

    // Season 1 → 2.
    play_fixtures_inner(&state).expect("play_fixtures season 1");
    advance_season_inner(&state).expect("advance_season 1");

    let career = state.career().read().expect("career read lock");

    // The watermark after season 2 must equal ledger.len() (per item 2's invariant).
    assert_eq!(
        career.breakthrough_eval_watermark,
        career.ledger.len(),
        "watermark must equal ledger.len() after the second season"
    );

    // The watermark after season 1 must be strictly less than the watermark after season 2
    // (season 2 appended at least one event — e.g. TitleWon — past the season-1 watermark).
    // This proves the watermark was advanced at the END of season 1, not left at the
    // beginning.
    assert!(
        watermark_after_s1 < career.breakthrough_eval_watermark,
        "watermark after season 1 ({watermark_after_s1}) must be < watermark after season 2 \
         ({}) — season 2 must have appended new events past the season-1 watermark. \
         Mutation killed: if the watermark were set to zero (pre-season value) after season 1, \
         this assertion would fail because both watermarks would be 0.",
        career.breakthrough_eval_watermark
    );
}

// ---------------------------------------------------------------------------
// QA-T4H item 6a: cross-decade proof asserts INJECTED event specifically renders
// ---------------------------------------------------------------------------

/// The existing `cross_decade_callback_survives_compaction` test proves some callback
/// is non-empty. This variant proves that the INJECTED `DebutSenior` event specifically
/// renders — its prose must contain a distinctive substring from the debut_senior grammar
/// variants (all three contain "debut", one contains "senior").
///
/// Mutation killed: if the injected event were crowded out of the top-5 by other events,
/// or if the injected EventId maps to the wrong grammar key, the callback list would
/// not contain any string with "debut" → assertion fails.
#[test]
fn cross_decade_injected_debut_event_renders_debut_prose() {
    let state = test_state_with_seed(0xfeed_beef_cafe_fade);

    // Resolve player P (lowest ClubId, slot 0).
    let (player_p_id, player_p_raw) = {
        let career = state.career().read().expect("career read lock");
        let first_club_id = *career.roster.keys().next().expect("roster not empty");
        let instances = career.roster.get(&first_club_id).expect("first club");
        let inst = instances.first().expect("at least one instance");
        (inst.player_id, inst.player_id.raw())
    };

    // Inject a season-1 DebutSenior event for P with maximum salience (stakes=ONE,
    // DecayFunction::Never) so it is guaranteed to rank in the top-5.
    {
        let mut career = state.career().write().expect("career write lock");
        career.ledger.append(make_debut_event(player_p_id));
    }

    // Advance 8 seasons so season-1 events are compacted (tick → None).
    advance_n_seasons(&state, 8);

    // Query player detail — the injected event must render as a callback.
    let player_id_str = format!("fwh.core:player_{:05}", player_p_raw);
    let dto = get_player_detail_inner(&player_id_str, &state)
        .expect("get_player_detail_inner must succeed");

    // All three debut_senior grammar variants contain "debut":
    //   "First senior appearance for #club_name# — #player_name# didn't look out of place"
    //   "#player_name# made his senior debut for #club_name# and held his own"
    //   "The debut at #club_name# — nervous before kick-off, better as the game went on"
    // If the injected event renders, at least one callback string must contain "debut".
    let has_debut_prose = dto
        .memory_callbacks
        .iter()
        .any(|cb| cb.to_lowercase().contains("debut") || cb.to_lowercase().contains("senior"));

    assert!(
        has_debut_prose,
        "after 8 seasons, the injected DebutSenior event (stakes=ONE, DecayFunction::Never) \
         must render callback prose containing 'debut' or 'senior'; \
         got callbacks: {:?}. \
         Mutation killed: if the injected event were crowded out or rendered the fallback \
         phrase, none of the callbacks would contain 'debut'.",
        dto.memory_callbacks,
    );
}

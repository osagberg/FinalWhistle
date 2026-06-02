//! T4-2.5d integration tests — breakthrough wiring into `advance_season_inner`.
//!
//! Acceptance criteria:
//!
//! 1. A 5-season career on seed `0xfeedbeefcafefade` produces ≥1 BreakthroughMoment
//!    in the ledger (pillar-3 end-to-end proof).
//! 2. A BreakthroughMoment's `delta_pa` is applied to the SPECIFIC player identified
//!    by the event's Subject participant (not just "any" player).
//! 3. `evaluate()` is called with per-player-filtered events — a 2-player fixture
//!    proves player B's events don't drive player A's meters.
//! 4. `PlayerInstance.genes` is populated at career-start (non-default).
//! 5. Both replay pins UNCHANGED (non-canonical path — verified separately via
//!    `cargo test -p fw-replay`).
//!
//! P0 invariant: evaluating an empty new-event window returns no outcomes, proving
//! the incremental evaluation path does not re-fire from historical events.

use std::path::PathBuf;

use fw_core::Seed;
use fw_memory::event::EventClass;
use fw_tauri::commands::{advance_season_inner, play_fixtures_inner};
use fw_tauri::state::AppState;

fn workspace_content_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

fn test_state_with_seed(seed: u64) -> AppState {
    AppState::new_with_career_seed(&workspace_content_path(), Seed::from_u64(seed))
        .expect("AppState::new_with_career_seed in test")
}

/// Advance through N complete seasons: play all fixtures then advance the season.
fn advance_n_seasons(state: &AppState, n: u32) {
    for _ in 0..n {
        play_fixtures_inner(state).expect("play_fixtures_inner");
        advance_season_inner(state).expect("advance_season_inner");
    }
}

// ---------------------------------------------------------------------------
// AC1 — ≥1 BreakthroughMoment in the ledger after 5 seasons
// ---------------------------------------------------------------------------

/// AC1: a 5-season career on the canonical seed produces at least one
/// `BreakthroughMoment` event in the ledger.
#[test]
fn five_season_career_produces_at_least_one_breakthrough_moment() {
    let state = test_state_with_seed(0xfeed_beef_cafe_fade);
    advance_n_seasons(&state, 5);

    let career = state.career().read().expect("career lock");

    let breakthrough_count = career
        .ledger
        .iter()
        .filter(|e| matches!(e.event_class, EventClass::BreakthroughMoment))
        .count();

    assert!(
        breakthrough_count >= 1,
        "5 seasons on seed 0xfeedbeefcafefade must produce ≥1 BreakthroughMoment in the \
         ledger; got {breakthrough_count}. Check that evaluate() is called, the ledger \
         filter passes player-subject events, and at least one player had a \
         signature-candidate + sufficient readiness after 5 seasons."
    );
}

// ---------------------------------------------------------------------------
// AC2 (FIX 7 — strengthened) — delta applied to the CORRECT player's ceiling
// ---------------------------------------------------------------------------

/// AC2: the BreakthroughMoment event's Subject player_id is the SAME player
/// whose `ceiling.potential()` increased.
///
/// This test extracts the Subject participant from the first BreakthroughMoment
/// event, looks up that specific player in the roster, and asserts their
/// potential is higher than before. An off-by-one write (applying the delta to
/// the wrong player) would cause this test to fail.
#[test]
fn breakthrough_delta_applied_to_correct_player_ceiling() {
    use fw_core::PlayerId;
    use fw_memory::event::{EntityRef, ParticipantRole};
    use std::collections::BTreeMap;

    let state = test_state_with_seed(0xfeed_beef_cafe_fade);

    // Capture all potentials before any advance.
    let initial_potentials: BTreeMap<PlayerId, fw_core::Q32> = {
        let career = state.career().read().expect("lock");
        career
            .roster
            .values()
            .flat_map(|v| v.iter())
            .map(|inst| (inst.player_id, inst.ceiling.potential()))
            .collect()
    };

    advance_n_seasons(&state, 5);

    let career = state.career().read().expect("career lock");

    // Find the first BreakthroughMoment and extract its Subject player_id.
    let first_breakthrough = career
        .ledger
        .iter()
        .find(|e| matches!(e.event_class, EventClass::BreakthroughMoment));

    let Some(bt_event) = first_breakthrough else {
        // AC1 owns the "must fire" guarantee. If we reach here, AC1 has already
        // failed or will fail — skip AC2 rather than emit a misleading error.
        return;
    };

    let subject_player_id: Option<PlayerId> = bt_event.participants.iter().find_map(|p| {
        if p.role == ParticipantRole::Subject
            && let EntityRef::Player(pid) = p.entity
        {
            return Some(pid);
        }
        None
    });

    let subject_id = subject_player_id
        .expect("BreakthroughMoment event must have a Subject participant with a Player entity");

    // Find that specific player in the current roster.
    let subject_inst = career
        .roster
        .values()
        .flat_map(|v| v.iter())
        .find(|inst| inst.player_id == subject_id)
        .unwrap_or_else(|| {
            panic!(
                "BreakthroughMoment subject player_id {:?} not found in roster",
                subject_id
            )
        });

    let before = initial_potentials
        .get(&subject_id)
        .copied()
        .unwrap_or_else(|| panic!("no initial potential recorded for {:?}", subject_id));

    assert!(
        subject_inst.ceiling.potential() > before,
        "BreakthroughMoment fired for player {:?} but their ceiling.potential() did not \
         increase: before={:?} after={:?}. apply_breakthrough_delta was not called or \
         the delta was applied to a different player.",
        subject_id,
        before,
        subject_inst.ceiling.potential()
    );
}

// ---------------------------------------------------------------------------
// P0 regression invariant — incremental evaluation: same events → no re-fire
// ---------------------------------------------------------------------------

/// P0 guard: evaluating the same historical events a second time (with the
/// watermark already advanced past them) does NOT produce a second breakthrough.
///
/// This is a unit-level test of the incremental evaluation contract, NOT an
/// end-to-end integration test of the full season loop. It directly exercises
/// `filter_new_events_for_player` with an empty slice (simulating a season
/// with no new player-subject events) and verifies that `evaluate()` returns
/// no outcomes.
///
/// The actual P0 bug was: `filter_ledger_for_player` (full-ledger scan) was
/// called every season instead of `filter_new_events_for_player` (new-only
/// slice). This test proves the new path returns zero outcomes on empty input,
/// which is the core invariant. The integration test
/// `five_season_career_produces_at_least_one_breakthrough_moment` validates
/// that the wiring still fires when real new events arrive.
#[test]
fn incremental_evaluation_does_not_re_fire_on_empty_new_event_window() {
    use fw_content::{ContentStore, gene_family_pa_ca};
    use fw_core::{Seed, Tick};
    use fw_memory::event::CareerDate;
    use fw_memory::{BreakthroughContext, evaluate};

    let content_root = workspace_content_path();
    let content = ContentStore::load_sources(&content_root).expect("ContentStore::load_sources");
    let sig_defs = std::sync::Arc::new(content.signature_definitions.clone());

    // Build a state with 5 seasons played to accumulate a real BreakthroughState.
    let state = test_state_with_seed(0xfeed_beef_cafe_fade);
    advance_n_seasons(&state, 5);

    // Find a player who fired a breakthrough (has a non-zero breakthrough_state).
    let (first_bt_player_id, player_ceiling, player_genes, player_bt_state, player_sig_candidates) = {
        let career = state.career().read().expect("lock");

        // Find a player with at least one BreakthroughMoment in the ledger.
        use fw_memory::event::{EntityRef, ParticipantRole};
        let bt_player_id: Option<fw_core::PlayerId> = career
            .ledger
            .iter()
            .filter(|e| matches!(e.event_class, EventClass::BreakthroughMoment))
            .find_map(|e| {
                e.participants.iter().find_map(|p| {
                    if p.role == ParticipantRole::Subject
                        && let EntityRef::Player(pid) = p.entity
                    {
                        return Some(pid);
                    }
                    None
                })
            });

        let Some(pid) = bt_player_id else {
            // No breakthrough fired across 5 seasons — the P0 test can't run.
            // AC1 covers the "must fire" guarantee.
            return;
        };

        let inst = career
            .roster
            .values()
            .flat_map(|v| v.iter())
            .find(|i| i.player_id == pid)
            .expect("bt player must be in roster");

        (
            pid,
            inst.ceiling,
            inst.genes.clone(),
            inst.breakthrough_state.clone(),
            inst.signature_candidates.clone(),
        )
    };

    // Build a BreakthroughContext from the player's current state.
    let family_pa_ca = gene_family_pa_ca(&player_genes, player_ceiling);
    let narrative_flags: Vec<fw_memory::NarrativeFlag> = player_genes
        .narrative_flags
        .iter()
        .map(|&f| fw_tauri::season::content_flag_to_memory_flag(f))
        .collect();
    let sig_candidates =
        fw_tauri::season::signature_candidates_to_ctx(&player_sig_candidates, &sig_defs);
    let ctx = BreakthroughContext {
        player_id: first_bt_player_id,
        pa_by_family: family_pa_ca.pa,
        ca_by_family: family_pa_ca.ca,
        narrative_flags,
        signature_candidates: sig_candidates,
        age_years: fw_tauri::season::CAREER_START_AGE_YEARS,
        career_date: CareerDate {
            year: 6,
            day_of_year: 365,
        },
    };

    // Simulate an "empty new-event window": no new player-subject events for this player.
    let empty_slice: &[fw_memory::event::MemoryEvent] = &[];
    let empty_ledger =
        fw_tauri::season::filter_new_events_for_player(empty_slice, first_bt_player_id);
    assert_eq!(
        empty_ledger.len(),
        0,
        "empty slice must produce empty ledger"
    );

    // Evaluate against the empty new-event window.
    let mut bt_state_copy = player_bt_state.clone();
    let outcomes = evaluate(
        &empty_ledger,
        &ctx,
        &mut bt_state_copy,
        Seed::from_u64(0xfeed_beef_cafe_fade).to_u64(),
        Tick::ZERO,
    );

    // An empty new-event window MUST produce zero outcomes.
    // If the old bug were present here (passing the full historical ledger),
    // outcomes would be non-empty because the old events would re-accumulate meters.
    assert!(
        outcomes.is_empty(),
        "evaluate() against an empty new-event window must return no outcomes; \
         got {} outcomes. This means evaluate() accumulated meters from zero events \
         and fired — which is impossible unless the breakthrough gate logic is broken. \
         (If you see this with a non-empty ledger being passed accidentally, the \
         filter_new_events_for_player or advance_season_inner watermark logic regressed.)",
        outcomes.len()
    );
}

// ---------------------------------------------------------------------------
// AC3 — per-player ledger filter: player B events don't drive player A's meters
// ---------------------------------------------------------------------------

/// AC3: the per-player ledger filter is correct.
#[test]
fn per_player_ledger_filter_excludes_other_player_events() {
    use fw_core::{PlayerId, Q32};
    use fw_memory::event::{
        CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind,
        EntityRef, MemoryEvent, Participant, ParticipantRole, SeasonNumber, SourceId,
    };
    use fw_memory::ledger::MemoryLedger;

    let player_a = PlayerId::new(1_000_001);
    let player_b = PlayerId::new(1_000_002);

    let make_event = |pid: PlayerId, class: EventClass| -> MemoryEvent {
        MemoryEvent {
            event_id: fw_memory::event::EventId(0), // overwritten by append
            schema_version: 1,
            season: SeasonNumber(0),
            tick: None,
            career_date: CareerDate {
                year: 1,
                day_of_year: 1,
            },
            emitter: Emitter {
                kind: EmitterKind::MatchEngine,
                source_id: SourceId::None,
            },
            participants: vec![Participant {
                role: ParticipantRole::Subject,
                entity: EntityRef::Player(pid),
            }],
            event_class: class,
            stakes: Q32::ONE,
            emotion: fw_memory::event::Emotion::Joy,
            consequence: vec![Consequence::None],
            callback_eligibility: CallbackEligibility::Immediate,
            salience: Q32::ZERO,
            decay_function: DecayFunction::Never,
        }
    };

    let mut ledger = MemoryLedger::new();
    // 3 events for A, 2 for B.
    ledger.append(make_event(player_a, EventClass::DebutSenior));
    ledger.append(make_event(player_a, EventClass::LegacyGoal));
    ledger.append(make_event(player_b, EventClass::DebutSenior));
    ledger.append(make_event(player_a, EventClass::HatTrickScored));
    ledger.append(make_event(player_b, EventClass::LegacyGoal));

    // Use the public filter helper exposed by fw-tauri.
    let filtered_a = fw_tauri::season::filter_ledger_for_player(&ledger, player_a);
    let filtered_b = fw_tauri::season::filter_ledger_for_player(&ledger, player_b);

    assert_eq!(
        filtered_a.len(),
        3,
        "filtered ledger for player A must have 3 events"
    );
    assert_eq!(
        filtered_b.len(),
        2,
        "filtered ledger for player B must have 2 events"
    );

    for ev in filtered_a.iter() {
        let has_a = ev.participants.iter().any(|p| {
            p.role == ParticipantRole::Subject
                && matches!(p.entity, EntityRef::Player(pid) if pid == player_a)
        });
        assert!(
            has_a,
            "filtered_a contains an event not subject-indexed to player_a: {:?}",
            ev.event_class
        );
    }

    for ev in filtered_b.iter() {
        let has_b = ev.participants.iter().any(|p| {
            p.role == ParticipantRole::Subject
                && matches!(p.entity, EntityRef::Player(pid) if pid == player_b)
        });
        assert!(
            has_b,
            "filtered_b contains an event not subject-indexed to player_b: {:?}",
            ev.event_class
        );
    }

    // F1: filter_ledger_for_player must PRESERVE the global career EventIds
    // (A's events were appended at ids 0,1,3; B's at 2,4), NOT renumber them
    // via append. A revert to append-renumbering would yield [0,1,2] / [0,1].
    let ids_a: Vec<u32> = filtered_a.iter().map(|e| e.event_id.0).collect();
    assert_eq!(
        ids_a,
        vec![0, 1, 3],
        "filter_ledger_for_player must preserve A's global EventIds (0,1,3); got {ids_a:?}"
    );
    let ids_b: Vec<u32> = filtered_b.iter().map(|e| e.event_id.0).collect();
    assert_eq!(
        ids_b,
        vec![2, 4],
        "filter_ledger_for_player must preserve B's global EventIds (2,4); got {ids_b:?}"
    );
}

// ---------------------------------------------------------------------------
// F1 — the production filter preserves GLOBAL career EventIds (no renumber)
// ---------------------------------------------------------------------------

/// F1 regression: `filter_new_events_for_player` (the PRODUCTION path used by
/// `advance_season_inner`) must preserve each event's global career `EventId`,
/// NOT renumber it to a within-batch index. Pre-fix it built the per-player
/// view via `MemoryLedger::append`, which renumbered ids to 0,1,2,… so
/// `evaluate()`'s `tick_for_rng = event.event_id.0` keyed on the batch index
/// → identical breakthrough deltas across seasons. This test fails if anyone
/// reverts the filter to `append` (the filtered ids would become 0,1 not 2,3).
#[test]
fn filter_new_events_for_player_preserves_global_event_ids() {
    use fw_core::{PlayerId, Q32};
    use fw_memory::event::{
        CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind,
        EntityRef, MemoryEvent, Participant, ParticipantRole, SeasonNumber, SourceId,
    };
    use fw_memory::ledger::MemoryLedger;

    let player_a = PlayerId::new(1_000_001);
    let player_b = PlayerId::new(1_000_002);

    let make_event = |pid: PlayerId, class: EventClass| -> MemoryEvent {
        MemoryEvent {
            event_id: fw_memory::event::EventId(0), // overwritten by append below
            schema_version: 1,
            season: SeasonNumber(0),
            tick: None,
            career_date: CareerDate {
                year: 1,
                day_of_year: 1,
            },
            emitter: Emitter {
                kind: EmitterKind::MatchEngine,
                source_id: SourceId::None,
            },
            participants: vec![Participant {
                role: ParticipantRole::Subject,
                entity: EntityRef::Player(pid),
            }],
            event_class: class,
            stakes: Q32::ONE,
            emotion: fw_memory::event::Emotion::Joy,
            consequence: vec![Consequence::None],
            callback_eligibility: CallbackEligibility::Immediate,
            salience: Q32::ZERO,
            decay_function: DecayFunction::Never,
        }
    };

    // Build a career ledger so `append` stamps the real global ids 0..=4.
    let mut ledger = MemoryLedger::new();
    ledger.append(make_event(player_a, EventClass::DebutSenior)); // id 0
    ledger.append(make_event(player_b, EventClass::DebutSenior)); // id 1
    ledger.append(make_event(player_a, EventClass::LegacyGoal)); // id 2
    ledger.append(make_event(player_a, EventClass::HatTrickScored)); // id 3
    ledger.append(make_event(player_b, EventClass::LegacyGoal)); // id 4

    // Mirror advance_season_inner: the new-events batch is the slice past a
    // watermark, carrying their global ids (here ids 1..=4 after skip(1)).
    let new_events: Vec<MemoryEvent> = ledger.iter().skip(1).cloned().collect();

    // Player A's events in the [1..=4] window are ids 2 and 3.
    let filtered = fw_tauri::season::filter_new_events_for_player(&new_events, player_a);

    let ids: Vec<u32> = filtered.iter().map(|e| e.event_id.0).collect();
    assert_eq!(
        ids,
        vec![2, 3],
        "filter_new_events_for_player must PRESERVE global EventIds (2, 3); \
         got {ids:?}. If this is [0, 1] the filter regressed to append-renumber \
         (the F1 bug — breakthrough RNG would key on the within-batch index)."
    );
}

// ---------------------------------------------------------------------------
// AC4 — genes populated at career start
// ---------------------------------------------------------------------------

/// AC4: every `PlayerInstance.genes` is populated (non-trivially) at career start.
#[test]
fn genes_populated_at_career_start() {
    use fw_content::GeneSnapshot;

    let state = test_state_with_seed(0xfeed_beef_cafe_fade);
    let career = state.career().read().expect("career lock");

    let total: usize = career.roster.values().map(|v| v.len()).sum();
    assert_eq!(total, 440, "20 clubs × 22 = 440 players");

    let mut seen: Vec<&GeneSnapshot> = Vec::new();
    for inst in career.roster.values().flat_map(|v| v.iter()) {
        if !seen.iter().any(|g| **g == inst.genes) {
            seen.push(&inst.genes);
        }
    }

    assert!(
        seen.len() >= 2,
        "career roster must contain ≥2 distinct GeneSnapshots; \
         got {} — round-robin from 22 bios over 440 players should produce 22 distinct snapshots.",
        seen.len()
    );
}

// ---------------------------------------------------------------------------
// fw-core: AbilityCeiling::apply_breakthrough_delta unit tests
// ---------------------------------------------------------------------------

#[test]
fn ability_ceiling_apply_breakthrough_delta_bumps_potential() {
    use fw_core::{AbilityCeiling, Q32};

    let half = Q32::from_raw(Q32::ONE.to_bits() / 2);
    let third = Q32::from_raw(Q32::ONE.to_bits() * 30 / 100);
    let mut ceiling = AbilityCeiling::try_new(third, half).expect("valid ceiling");

    let pot_before = ceiling.potential();
    let cur_before = ceiling.current();

    ceiling.apply_breakthrough_delta(5, 2);

    assert!(
        ceiling.potential() > pot_before,
        "delta_pa=5 must raise potential"
    );
    assert!(
        ceiling.current() > cur_before,
        "delta_ca=2 must raise current"
    );
    assert!(
        ceiling.current() <= ceiling.potential(),
        "invariant current ≤ potential violated"
    );
}

#[test]
fn ability_ceiling_apply_breakthrough_delta_clamps_at_one() {
    use fw_core::{AbilityCeiling, Q32};

    let near_one = Q32::from_raw(Q32::ONE.to_bits() - 10);
    let mut ceiling = AbilityCeiling::try_new(near_one, near_one).expect("valid ceiling");

    ceiling.apply_breakthrough_delta(200, 200);

    assert_eq!(
        ceiling.potential(),
        Q32::ONE,
        "potential must clamp to Q32::ONE"
    );
    assert_eq!(
        ceiling.current(),
        Q32::ONE,
        "current must clamp to Q32::ONE"
    );
}

#[test]
fn ability_ceiling_apply_breakthrough_delta_zero_delta_is_noop() {
    use fw_core::{AbilityCeiling, Q32};

    let half = Q32::from_raw(Q32::ONE.to_bits() / 2);
    let third = Q32::from_raw(Q32::ONE.to_bits() / 3);
    let mut ceiling = AbilityCeiling::try_new(third, half).expect("valid ceiling");
    let pot = ceiling.potential();
    let cur = ceiling.current();

    ceiling.apply_breakthrough_delta(0, 0);

    assert_eq!(ceiling.potential(), pot, "zero delta_pa must be a no-op");
    assert_eq!(ceiling.current(), cur, "zero delta_ca must be a no-op");
}

#[test]
fn ability_ceiling_apply_breakthrough_delta_current_never_exceeds_potential() {
    use fw_core::{AbilityCeiling, Q32};

    let val = Q32::from_raw(Q32::ONE.to_bits() * 80 / 100);
    let mut ceiling = AbilityCeiling::try_new(val, val).expect("valid ceiling");

    ceiling.apply_breakthrough_delta(0, 10);

    assert!(
        ceiling.current() <= ceiling.potential(),
        "current must never exceed potential after apply_breakthrough_delta"
    );
}

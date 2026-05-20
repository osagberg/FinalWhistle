//! Integration tests for the breakthrough mechanism (T3-4).
//!
//! ## AC7 — Cadence test
//!
//! The acceptance target: across a 5-season career, 1-3 breakthroughs fire
//! per player on average. This test builds a synthetic representative
//! multi-season `MemoryLedger` for a cohort of players and asserts that the
//! COHORT MEAN is in [1.0, 3.0].
//!
//! ## Cadence derivation (from progression.md §"Cadence math")
//!
//! Assumptions (Phase-3 seeds):
//! - 6 salient events per season per player spread across families.
//! - Median family_relevance weight for a relevant event: 0.15.
//! - Median event salience (stakes): 0.40.
//! - Per-event readiness delta: 0.40 × 0.15 = 0.06.
//! - From residue (0.15) to 0.92 requires (0.92 − 0.15) / 0.06 ≈ 12.8 events.
//! - At 1.5 relevant events/season/family → ~8.5 seasons per single family.
//! - For a top-flight starter with 2 signature candidates (2 active families):
//!   2 × (5 / 10) = 1.0 breakthroughs in 5 seasons minimum.
//!   Higher cadence players with 2-3 candidates → 2-3 breakthroughs.
//!
//! ## Synthetic harness design
//!
//! We build a cohort of 20 players:
//! - 10 "top-flight starters" with 2 signature candidates (high-cadence).
//! - 10 "depth players" with 0 signature candidates (low-cadence).
//!
//! Each player's ledger gets a stream of events that produce accumulated
//! readiness following the above cadence model. We use a mix of event classes
//! representative of a real season (LegacyGoal, CupFinalWin, BigMatchScar,
//! MentorTeammate, etc.) at realistic stakes.
//!
//! To make breakthroughs fire deterministically, top-flight starters also
//! receive enough high-stakes gating events (LegacyGoal, CupFinalWin) that
//! the gate can trip once readiness crosses the threshold.

use std::collections::BTreeMap;

use fw_core::MatchId;
use fw_core::{PlayerId, Q32, Tick};
use fw_memory::event::{
    CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind, Emotion,
    EntityRef, EventClass, EventId, MemoryEvent, Participant, ParticipantRole, SeasonNumber,
    SourceId,
};
use fw_memory::ledger::MemoryLedger;
use fw_memory::{
    AttributeFamily, BREAKTHROUGH_THRESHOLD, BreakthroughContext, BreakthroughKind,
    BreakthroughState, NarrativeFlag, accumulate, evaluate,
};

// -------------------------------------------------------------------------
// Helper: build a synthetic career event
// -------------------------------------------------------------------------

fn make_career_event(
    class: EventClass,
    player_id: PlayerId,
    stakes_raw: i64,
    season: u16,
) -> MemoryEvent {
    MemoryEvent {
        event_id: EventId(0), // overwritten by append
        schema_version: 1,
        season: SeasonNumber(season),
        tick: Some(Tick::ZERO),
        career_date: CareerDate {
            year: 1 + season,
            day_of_year: 200,
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
        stakes: Q32::from_raw(stakes_raw),
        emotion: Emotion::Joy,
        consequence: vec![Consequence::None],
        callback_eligibility: CallbackEligibility::Immediate,
        salience: Q32::from_raw(stakes_raw),
        decay_function: DecayFunction::Never,
    }
}

// -------------------------------------------------------------------------
// Synthetic season builder
// -------------------------------------------------------------------------

/// Build one "season" of events for a player.
///
/// Matches the progression.md §"Cadence math" assumptions:
/// ~6 salient events per season at median salience ~0.40, spread across families.
///
/// Event mix per season (realistic, not exceptional):
/// - 1 ContractRenewalAccepted (Composure + WorkRate + Leadership small boost)
/// - 1 InternationalCallUp (small boosts across many families)
/// - 1 RivalryFormed (5 families × 0.05 each)
/// - 1 BigMatchScar (regressive pressure on Composure, WorkRate, Leadership)
/// - 1 DebutClub or FormerClubReunion (minor boosts)
/// - 1 SignatureFirstFired (several small boosts)
///
/// Major events (LegacyGoal, CupFinalWin, PromotionWon) are NOT included here;
/// they appear only once per career (see `build_career_major_events`).
/// This matches the cadence math assumption that LegacyGoal/CupFinalWin are
/// rare events, not weekly occurrences.
///
/// Stakes are set at 0.40 (median per progression.md).
fn build_season_events(player_id: PlayerId, season: u16) -> Vec<MemoryEvent> {
    // median stakes ≈ 0.40 in Q32: round(0.40 × 2^32) = 1_717_986_918
    let med_stakes = 1_717_986_918_i64;

    vec![
        make_career_event(
            EventClass::ContractRenewalAccepted,
            player_id,
            med_stakes,
            season,
        ),
        make_career_event(
            EventClass::InternationalCallUp,
            player_id,
            med_stakes,
            season,
        ),
        make_career_event(EventClass::RivalryFormed, player_id, med_stakes, season),
        make_career_event(EventClass::BigMatchScar, player_id, med_stakes, season),
        make_career_event(EventClass::FormerClubReunion, player_id, med_stakes, season),
        make_career_event(
            EventClass::SignatureFirstFired,
            player_id,
            med_stakes,
            season,
        ),
    ]
}

/// Build the 1-2 major events that fire ONCE across a 5-season career.
///
/// These correspond to the "gating events" from progression.md — the events
/// that actually trigger the breakthrough gate. We place them at the END of
/// the career (season 4) so meters have had time to accumulate.
///
/// `player_type`: 0 = top-flight (LegacyGoal + CupFinalWin), 1 = depth (no major events).
fn build_career_major_events(player_id: PlayerId, player_type: u8) -> Vec<MemoryEvent> {
    // high stakes ≈ 0.80 in Q32: round(0.80 × 2^32) = 3_435_973_837
    let high_stakes = 3_435_973_837_i64;

    if player_type == 0 {
        // Top-flight starter: one LegacyGoal (potential Finishing gate) and
        // one PromotionWon (Composure/WorkRate/Leadership gate) at season 4.
        vec![
            make_career_event(EventClass::LegacyGoal, player_id, high_stakes, 4),
            make_career_event(EventClass::PromotionWon, player_id, high_stakes, 4),
        ]
    } else {
        // Depth player: no major events — they rarely or never fire.
        vec![]
    }
}

// -------------------------------------------------------------------------
// AC7: cadence — 1-3 breakthroughs per player per 5-season career
// -------------------------------------------------------------------------

#[test]
fn cadence_one_to_three_per_player_per_5_season_career() {
    const SEASONS: u16 = 5;
    const COHORT_SIZE: u32 = 20;
    // 10 top-flight starters (2 signature candidates each) + 10 depth players.
    const TOP_FLIGHT_COUNT: u32 = 10;

    let career_seed = 0xDEAD_BEEF_DEAD_BEEFu64;
    let mut total_breakthroughs: u32 = 0;

    for p_idx in 0..COHORT_SIZE {
        let player_id = PlayerId::new(100 + p_idx);
        let is_top_flight = p_idx < TOP_FLIGHT_COUNT;

        // Build a 5-season ledger with routine events + end-of-career major events.
        let mut ledger = MemoryLedger::new();
        for season in 0..SEASONS {
            for ev in build_season_events(player_id, season) {
                ledger.append(ev);
            }
        }
        // Add 1-2 major career events for top-flight starters.
        let player_type = if is_top_flight { 0u8 } else { 1u8 };
        for ev in build_career_major_events(player_id, player_type) {
            ledger.append(ev);
        }

        // Build PA/CA maps: top-flight starters have 100/70; depth players 80/60.
        let pa_base: i16 = if is_top_flight { 100 } else { 80 };
        let ca_base: i16 = if is_top_flight { 70 } else { 55 };
        let mut pa = BTreeMap::new();
        let mut ca = BTreeMap::new();
        for &f in &AttributeFamily::ALL {
            pa.insert(f, pa_base);
            ca.insert(f, ca_base);
        }

        // Top-flight starters have 2 signature candidates (Finishing + Composure).
        let signature_candidates = if is_top_flight {
            vec![
                (
                    AttributeFamily::Finishing,
                    "fwh.core:signature.long_range_strike".to_string(),
                ),
                (
                    AttributeFamily::Composure,
                    "fwh.core:signature.composure_under_pressure".to_string(),
                ),
            ]
        } else {
            vec![]
        };

        // Narrative flags: give top-flight players a LateBloomer flag to add gate paths.
        let narrative_flags = if is_top_flight {
            vec![NarrativeFlag::LateBloomer]
        } else {
            vec![]
        };

        let ctx = BreakthroughContext {
            player_id,
            pa_by_family: pa,
            ca_by_family: ca,
            narrative_flags,
            signature_candidates,
            age_years: 24,
            // career_date at end of season 5 (used for cooldown checks)
            career_date: CareerDate {
                year: SEASONS + 1,
                day_of_year: 365,
            },
        };

        let mut state = BreakthroughState::new();
        let outcomes = evaluate(&ledger, &ctx, &mut state, career_seed, Tick::ZERO);

        // Count only POSITIVE breakthroughs (not regressive collapses).
        let positive_count = outcomes
            .iter()
            .filter(|o| !matches!(o.kind, BreakthroughKind::RegressiveCollapse))
            .count() as u32;
        total_breakthroughs += positive_count;
    }

    let mean_numerator = total_breakthroughs;
    let mean_denominator = COHORT_SIZE;

    // Mean must be in [1.0, 3.0] i.e. mean_numerator / mean_denominator ∈ [1.0, 3.0].
    // Equivalent to: mean_denominator × 1 ≤ mean_numerator ≤ mean_denominator × 3.
    let min_total = mean_denominator;
    let max_total = mean_denominator * 3;

    assert!(
        mean_numerator >= min_total && mean_numerator <= max_total,
        "AC7 CADENCE MISS: mean breakthroughs/player = {}/{} (target: {}-{}). \
        If this fires, progression.md tuning seeds may need adjustment — \
        DO NOT silently re-tune. Report to main thread with this observed ratio.",
        mean_numerator,
        mean_denominator,
        min_total,
        max_total,
    );
}

// -------------------------------------------------------------------------
// AC5: determinism across identical runs
// -------------------------------------------------------------------------

#[test]
fn evaluate_is_deterministic_across_runs() {
    let player_id = PlayerId::new(200);
    let career_seed = 0xCAFE_BABE_1234_5678u64;

    let mut ledger = MemoryLedger::new();
    for season in 0..3u16 {
        for ev in build_season_events(player_id, season) {
            ledger.append(ev);
        }
    }

    let mut pa = BTreeMap::new();
    let mut ca = BTreeMap::new();
    for &f in &AttributeFamily::ALL {
        pa.insert(f, 100i16);
        ca.insert(f, 70i16);
    }

    let ctx = BreakthroughContext {
        player_id,
        pa_by_family: pa.clone(),
        ca_by_family: ca.clone(),
        narrative_flags: vec![NarrativeFlag::LateBloomer],
        signature_candidates: vec![(
            AttributeFamily::Finishing,
            "fwh.core:signature.long_range_strike".to_string(),
        )],
        age_years: 26,
        career_date: CareerDate {
            year: 4,
            day_of_year: 365,
        },
    };

    let mut state1 = BreakthroughState::new();
    let outcomes1 = evaluate(&ledger, &ctx, &mut state1, career_seed, Tick::ZERO);

    let mut state2 = BreakthroughState::new();
    let outcomes2 = evaluate(&ledger, &ctx, &mut state2, career_seed, Tick::ZERO);

    assert_eq!(
        outcomes1.len(),
        outcomes2.len(),
        "run 1 and run 2 must produce the same count"
    );
    for (o1, o2) in outcomes1.iter().zip(outcomes2.iter()) {
        assert_eq!(
            o1.delta_pa, o2.delta_pa,
            "delta_pa must be identical across runs"
        );
        assert_eq!(
            o1.delta_ca, o2.delta_ca,
            "delta_ca must be identical across runs"
        );
        assert_eq!(o1.family, o2.family, "family must be identical across runs");
    }
}

// -------------------------------------------------------------------------
// Regressive collapse fires and is bounded
// -------------------------------------------------------------------------

#[test]
fn regressive_collapse_fires_with_pressure_and_gating_event() {
    let player_id = PlayerId::new(300);

    // Build a ledger with enough BigMatchScar events to push Composure pressure
    // to the regressive threshold (0.90), then add a high-stakes BigMatchScar gating event.
    //
    // BigMatchScar → Composure: neg_relevance = 0.30.
    // Each BigMatchScar at stakes=0.5 contributes: 0.5 × 0.30 = 0.15 to regressive_pressure.
    // From 0 to 0.90: requires 0.90 / 0.15 = 6 events.
    // We add 8 to ensure we're over the threshold even after the gate check.
    let mut ledger = MemoryLedger::new();
    let high_stakes = 3_435_973_837_i64; // 0.80
    let med_stakes = 2_147_483_648_i64; // 0.50

    for i in 0..8u16 {
        ledger.append(make_career_event(
            EventClass::BigMatchScar,
            player_id,
            med_stakes,
            i / 2,
        ));
    }
    // Final high-stakes gating event.
    ledger.append(make_career_event(
        EventClass::BigMatchScar,
        player_id,
        high_stakes,
        4,
    ));

    let mut pa = BTreeMap::new();
    let mut ca = BTreeMap::new();
    for &f in &AttributeFamily::ALL {
        pa.insert(f, 100i16);
        ca.insert(f, 70i16);
    }

    let ctx = BreakthroughContext {
        player_id,
        pa_by_family: pa,
        ca_by_family: ca,
        narrative_flags: vec![],
        signature_candidates: vec![],
        age_years: 28,
        career_date: CareerDate {
            year: 5,
            day_of_year: 200,
        },
    };

    let mut state = BreakthroughState::new();
    let outcomes = evaluate(&ledger, &ctx, &mut state, 0xABCD, Tick::ZERO);

    let regressive: Vec<_> = outcomes
        .iter()
        .filter(|o| matches!(o.kind, BreakthroughKind::RegressiveCollapse))
        .collect();

    assert!(
        !regressive.is_empty(),
        "regressive collapse should have fired"
    );

    // Career floor: max(20, 70-30) = 40. PA=100 → new_pa must be >= 40.
    for r in &regressive {
        let new_pa = 100i16 + r.delta_pa;
        assert!(
            new_pa >= 40,
            "new_pa {} must be >= career floor 40 (max(20, ca-30))",
            new_pa
        );
        assert!(r.delta_pa < 0, "regressive delta_pa must be negative");
    }
}

// -------------------------------------------------------------------------
// Kind-2 (latent-flag unlock) fires when flag is present
// -------------------------------------------------------------------------

#[test]
fn latent_flag_unlock_kind2_fires_when_flag_present() {
    let player_id = PlayerId::new(400);

    // Build enough readiness for Finishing to cross the threshold.
    // LegacyGoal → Finishing relevance = 0.45.
    // At stakes=0.80: delta = 0.80 × 0.45 = 0.36 per event.
    // From 0 to 0.92: need ceil(0.92 / 0.36) = 3 events.
    let mut ledger = MemoryLedger::new();
    let high_stakes = 3_435_973_837_i64;
    for i in 0..3u16 {
        ledger.append(make_career_event(
            EventClass::LegacyGoal,
            player_id,
            high_stakes,
            i,
        ));
    }

    let mut pa = BTreeMap::new();
    let mut ca = BTreeMap::new();
    for &f in &AttributeFamily::ALL {
        pa.insert(f, 100i16);
        ca.insert(f, 70i16);
    }

    // Player has LateBloomer flag, no signature candidates.
    let ctx = BreakthroughContext {
        player_id,
        pa_by_family: pa,
        ca_by_family: ca,
        narrative_flags: vec![NarrativeFlag::LateBloomer],
        signature_candidates: vec![],
        age_years: 30,
        career_date: CareerDate {
            year: 4,
            day_of_year: 1,
        },
    };

    let mut state = BreakthroughState::new();
    let outcomes = evaluate(&ledger, &ctx, &mut state, 0x1234, Tick::ZERO);

    let kind2: Vec<_> = outcomes
        .iter()
        .filter(|o| matches!(o.kind, BreakthroughKind::LatentFlagUnlock { .. }))
        .collect();

    assert!(
        !kind2.is_empty(),
        "Kind-2 breakthrough should fire when flag present"
    );
    // Verify the flag is carried in the kind.
    match &kind2[0].kind {
        BreakthroughKind::LatentFlagUnlock { flag } => {
            assert_eq!(*flag, NarrativeFlag::LateBloomer);
        }
        _ => panic!("expected LatentFlagUnlock kind"),
    }
}

// -------------------------------------------------------------------------
// meter accumulate unit from integration path
// -------------------------------------------------------------------------

#[test]
fn accumulate_over_full_career_ledger_is_monotonically_bounded() {
    let player_id = PlayerId::new(500);

    // 5 seasons × 6 events = 30 events. All readiness values must stay ≤ Q32::ONE.
    let mut state = BreakthroughState::new();
    let mut ledger = MemoryLedger::new();

    for season in 0..5u16 {
        for ev in build_season_events(player_id, season) {
            let stored = ledger.append(ev);
            let event = ledger.get_by_id(stored).unwrap();
            accumulate(&mut state, event, Tick::ZERO);
        }
    }

    for &family in &AttributeFamily::ALL {
        let r = state.readiness(family);
        let p = state.pressure(family);
        assert!(r <= Q32::ONE, "readiness[{:?}] must not exceed 1.0", family);
        assert!(p <= Q32::ONE, "pressure[{:?}] must not exceed 1.0", family);
        assert!(
            r >= Q32::ZERO,
            "readiness[{:?}] must be non-negative",
            family
        );
        assert!(
            p >= Q32::ZERO,
            "pressure[{:?}] must be non-negative",
            family
        );
    }
}

// -------------------------------------------------------------------------
// Cooldown cross-family independence
// -------------------------------------------------------------------------

#[test]
fn cooldowns_are_independent_per_family() {
    // A breakthrough in Finishing should not block a Composure breakthrough.
    let player_id = PlayerId::new(600);

    // Pre-set both readiness meters at threshold using the clamped public API.
    let mut state = BreakthroughState::new();
    // add_readiness clamps; set to exactly ONE (>= threshold) then meter is above threshold.
    state.add_readiness(AttributeFamily::Finishing, BREAKTHROUGH_THRESHOLD);
    state.add_readiness(AttributeFamily::Composure, BREAKTHROUGH_THRESHOLD);

    // Record a recent Finishing fire (within cooldown) using the public setter.
    state.set_last_positive_fire(
        AttributeFamily::Finishing,
        CareerDate {
            year: 1,
            day_of_year: 350,
        },
    );
    // No Composure cooldown set.

    let mut pa = BTreeMap::new();
    let mut ca = BTreeMap::new();
    for &f in &AttributeFamily::ALL {
        pa.insert(f, 100i16);
        ca.insert(f, 70i16);
    }

    // career_date is only 10 days after the Finishing fire.
    // Player has both Finishing and Composure candidates so both families CAN fire
    // when their other conditions are met.
    let ctx = BreakthroughContext {
        player_id,
        pa_by_family: pa,
        ca_by_family: ca,
        narrative_flags: vec![],
        signature_candidates: vec![
            (
                AttributeFamily::Finishing,
                "fwh.core:signature.long_range_strike".to_string(),
            ),
            (
                AttributeFamily::Composure,
                "fwh.core:signature.composure_under_pressure".to_string(),
            ),
        ],
        age_years: 26,
        career_date: CareerDate {
            year: 1,
            day_of_year: 360,
        },
    };

    let mut ledger = MemoryLedger::new();
    // CupFinalWin: valid gate for BOTH Finishing (via its table) and Composure.
    // But Finishing is blocked by cooldown (only 10 days since last fire; cooldown = 365 days).
    ledger.append(make_career_event(
        EventClass::CupFinalWin,
        player_id,
        3_435_973_837_i64,
        0,
    ));

    let outcomes = evaluate(&ledger, &ctx, &mut state, 0xBEEF, Tick::ZERO);

    // Finishing should be blocked (recent fire), Composure should be allowed.
    let finishing_fires = outcomes
        .iter()
        .any(|o| o.family == AttributeFamily::Finishing);
    let composure_fires = outcomes.iter().any(|o| {
        o.family == AttributeFamily::Composure
            && !matches!(o.kind, BreakthroughKind::RegressiveCollapse)
    });

    assert!(
        !finishing_fires,
        "Finishing should be blocked by its cooldown"
    );
    assert!(
        composure_fires,
        "Composure should fire independently of Finishing cooldown"
    );
}

// -------------------------------------------------------------------------
// Consequence variants serde round-trip
// -------------------------------------------------------------------------

#[test]
fn consequence_pa_redraw_serde_round_trips() {
    use serde_json;

    let c = Consequence::PaRedraw {
        family: AttributeFamily::Finishing,
        delta_pa: 6,
        delta_ca: 3,
    };
    let json = serde_json::to_string(&c).expect("serialize PaRedraw");
    let decoded: Consequence = serde_json::from_str(&json).expect("deserialize PaRedraw");
    assert_eq!(c, decoded, "PaRedraw must serde round-trip correctly");
}

#[test]
fn consequence_pa_reduction_redraw_serde_round_trips() {
    use serde_json;

    let c = Consequence::PaReductionRedraw {
        family: AttributeFamily::Composure,
        delta_pa: -7,
        delta_ca: -3,
    };
    let json = serde_json::to_string(&c).expect("serialize PaReductionRedraw");
    let decoded: Consequence = serde_json::from_str(&json).expect("deserialize PaReductionRedraw");
    assert_eq!(
        c, decoded,
        "PaReductionRedraw must serde round-trip correctly"
    );
}

#[test]
fn consequence_signature_activated_serde_round_trips() {
    use serde_json;

    let c = Consequence::SignatureActivated {
        signature_id: "fwh.core:signature.long_range_strike".to_string(),
    };
    let json = serde_json::to_string(&c).expect("serialize SignatureActivated");
    let decoded: Consequence = serde_json::from_str(&json).expect("deserialize SignatureActivated");
    assert_eq!(
        c, decoded,
        "SignatureActivated must serde round-trip correctly"
    );
}

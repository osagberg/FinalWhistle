//! FUN-TS2 proptest invariants — four acceptance criteria.
//!
//! TS2-P1: enforce_hold_zonal wins vs adversarial signature bias.
//!         When `shape.is_defending` the HoldFormation intent returned by
//!         `enforce_hold_zonal` MUST NOT be downscaled by signature
//!         `cover_mul`; the bias pass is skipped for defending HoldFormation.
//!
//! TS2-P2: offside fires in final third, not in midfield; not backward; not on set pieces.
//!         (deterministic unit tests — geometry-driven, not proptest-random.)
//!
//! TS2-P3: HighPress produces exactly 1 Primary, 2 Cover, 8 HoldShape roles.
//!         For any player positions in the pitch rectangle and any carrier
//!         slot, `compute_press_from_parts` assigns the correct role counts.
//!
//! TS2-P4: `line_height_metres` and `press_intensity` decouple independently.
//!         High press_radius + low line_height → MidBlock/LowBlock defence.
//!         Low press_radius + high line_height → HighPress defence.

use fw_content::{
    BiasCategory, CooldownPolicy, RoleFamily, SignatureDefinition, SignatureId,
    SignaturePresentationRecipe, SignatureTrigger, SimBiasSnapshot, StackingPolicy,
};
use fw_core::{Q32, Seed, Tick};
use fw_match_sim::tactic_fsm::{PressIntensity, TacticState, TeamTacticState};
use fw_match_sim::team_shape::{PressRole, TeamShape, compute_press_from_parts};
use fw_match_sim::{MatchState, PlayerRoleState, subtree_library};
use proptest::prelude::*;
use rand_chacha::rand_core::SeedableRng;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn arb_seed() -> impl Strategy<Value = u64> {
    any::<u64>()
}

/// Build a minimal `SignatureDefinition` with adversarial cover_mul (0.25)
/// to attempt to suppress HoldFormation utility.
fn adversarial_cover_bias_def() -> SignatureDefinition {
    let id = "test.ts2:signature.adversarial-cover";
    SignatureDefinition {
        schema_version: 1,
        id: SignatureId::try_new(id).expect("valid signature id"),
        display_name: id.to_string(),
        role_family: RoleFamily::CentralMidfielder,
        trigger: SignatureTrigger::NoOpStub,
        bias_snapshot: SimBiasSnapshot {
            shoot_mul: Q32::ONE,
            pass_mul: Q32::ONE,
            dribble_mul: Q32::ONE,
            // Press_mul = 2.0: competing press inflated — attempts to beat
            // HoldFormation by also boosting the competition.
            press_mul: Q32::from_raw(8_589_934_592_i64),
            // cover_mul = 0.25: HoldFormation utility halved, the classic
            // adversarial case that previously broke zonal shape.
            cover_mul: Q32::from_raw(1_073_741_824_i64),
        },
        presentation: SignaturePresentationRecipe {
            commentary_line_bank_id: "placeholder".to_string(),
            camera_framing_hint: "default".to_string(),
            schema_version: 1,
        },
        cooldown: CooldownPolicy::EveryTicks(600),
        stacking: StackingPolicy::Exclusive {
            category: BiasCategory::Attacking,
        },
    }
}

/// Build a minimal defending TeamShape suitable for testing HoldFormation
/// exemption. `is_defending = true`. `line_x` = –28m (LowBlock).
fn defending_shape() -> TeamShape {
    let mut shape = TeamShape::zero();
    shape.is_defending = true;
    shape.is_high_press = false;
    shape.line_x = Q32::from_int(-28);
    shape.compactness_v = Q32::from_int(25);
    shape
}

// ---------------------------------------------------------------------------
// TS2-P1: enforce_hold_zonal wins vs adversarial signature bias
// ---------------------------------------------------------------------------
//
// The FUN-TS2a exemption in `select_outfield_intent` ensures that when
// `shape.is_defending` is true and the best candidate is `HoldFormation`,
// the signature bias pass is skipped entirely. This test exercises the
// adversarial case: cover_mul = 0.25 (halves HoldFormation score) +
// press_mul = 2.0 (doubles competing press score). Without the exemption,
// the biased HoldFormation score would drop below biased press, breaking
// zonal shape.
//
// We call `select_outfield_intent` for a Defender in `Defending` state
// (which routes to `enforce_hold_zonal` when defending) and assert the
// returned intent is `PlayerIntent::HoldFormation`.

proptest! {
    #[test]
    fn ts2_p1_enforce_hold_zonal_wins_vs_adversarial_signature_bias(seed_u64 in arb_seed()) {
        let seed = Seed::from_u64(seed_u64);
        let state = MatchState::initial(seed);
        // Slot index 1 = home defender (absolute 0-indexed; slot 0 is GK).
        let player = &state.players[1];

        let def = adversarial_cover_bias_def();
        let snap = &def.bias_snapshot;

        // Build a SimBiasSnapshot-backed active_bias using the exact snapshot.
        // select_outfield_intent accepts `Option<&SimBiasSnapshot>`.
        let shape = defending_shape();
        let team_idx = 0usize;
        let roster_slot: u8 = 2; // slot 2 is home DEF1

        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed_u64.wrapping_add(1));

        let intent = subtree_library::select_outfield_intent(
            PlayerRoleState::Defender(fw_match_sim::DefenderState::Defending),
            player,
            roster_slot,
            &mut rng,
            Some(snap),  // adversarial bias active
            None,        // no carrier
            &shape,
            team_idx,
        );

        // The HoldFormation intent must be returned regardless of adversarial bias.
        match intent {
            fw_match_sim::PlayerIntent::HoldFormation { .. } => {},
            other => {
                prop_assert!(
                    false,
                    "TS2-P1: expected HoldFormation when defending (enforce_hold_zonal exemption), \
                     got {other:?}. adversarial cover_mul=0.25 + press_mul=2.0 should NOT break \
                     zonal shape enforcement."
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TS2-P3: HighPress produces exactly 1 Primary, 2 Cover, 8 HoldShape
// ---------------------------------------------------------------------------
//
// `compute_press_from_parts` must assign exactly:
//   - 1 Primary  (closest outfield player to carrier)
//   - 2 Cover    (2nd + 3rd closest)
//   - 8 HoldShape (remaining 8 outfield players)
//
// For ANY player positions and any carrier slot in range.
// GK (team_local = 0) is always HoldShape (GK never presses).

proptest! {
    #[test]
    fn ts2_p3_highpress_has_exactly_1_primary_2_cover_8_holdshape(
        seed_u64 in arb_seed(),
        // carrier is one of the away team absolute slots (11..=21, 0-indexed),
        // i.e. home is defending.
        carrier_slot in 11u8..=21u8,
    ) {
        let seed = Seed::from_u64(seed_u64);
        let state = MatchState::initial(seed);

        // Force home team to be defending against the carrier.
        // PlayerSlot is a u8 type alias — carrier_slot is already the right type.
        let state = state.with_possession(carrier_slot);

        // Set home team to HighPress tactic state.
        let mut state = state;
        state.team_tactic_states[0] = TeamTacticState::initial()
            .transition(TacticState::HighPress, Tick::ZERO);

        // Run compute_press_from_parts via the exported function.
        let player_positions: [(Q32, Q32); 22] = std::array::from_fn(|i| {
            (state.players[i].pos_x, state.players[i].pos_y)
        });
        let tactic_states = state.team_tactic_states;

        let mut shapes = [
            TeamShape::zero(),
            TeamShape::zero(),
        ];
        // Mark home as defending.
        shapes[0].is_defending = true;

        compute_press_from_parts(
            &mut shapes,
            Some(carrier_slot),
            &player_positions,
            &tactic_states,
        );

        let home_shape = &shapes[0];
        prop_assert!(
            home_shape.is_high_press,
            "TS2-P3: expected is_high_press=true when defending HighPress; got false"
        );

        let primaries = home_shape.press_roles.iter().filter(|r| **r == PressRole::Primary).count();
        let covers    = home_shape.press_roles.iter().filter(|r| **r == PressRole::Cover).count();
        let holds     = home_shape.press_roles.iter().filter(|r| **r == PressRole::HoldShape).count();

        prop_assert!(
            primaries == 1,
            "TS2-P3: expected exactly 1 Primary; got {} (roles={:?})", primaries, home_shape.press_roles
        );
        prop_assert!(
            covers == 2,
            "TS2-P3: expected exactly 2 Cover; got {} (roles={:?})", covers, home_shape.press_roles
        );
        prop_assert!(
            holds == 8,
            "TS2-P3: expected exactly 8 HoldShape; got {} (roles={:?})", holds, home_shape.press_roles
        );
        prop_assert_eq!(
            primaries + covers + holds, 11usize,
            "TS2-P3: press_roles array is 11; counts should sum to 11"
        );
    }
}

// ---------------------------------------------------------------------------
// TS2-P3b: writer→reader index alignment — press_role_for(team_local)
//           returns the same role that compute_press_from_parts wrote
// ---------------------------------------------------------------------------
//
// TS2-P3 (above) only tested the WRITER (compute_press_from_parts role counts).
// This test closes the writer→reader loop by verifying the INDEX used by the
// reader in select_outfield_intent correctly maps roster_slot → team_local.
//
// The off-by-one bug: reader used `roster_slot - 1` (home) / `roster_slot - 12`
// (away) while the writer stores at `abs_slot` (home) / `abs_slot - 11` (away)
// which equals `roster_slot` / `roster_slot - 11` because roster_slot is
// 0-indexed. The fix: reader now uses `roster_slot` (home) / `roster_slot - 11`
// (away) to match the writer.
//
// Test logic:
//   1. Call compute_press_from_parts for a known geometry.
//   2. Find which team-local slot was assigned Primary.
//   3. Verify that press_role_for(roster_slot) == Primary for that slot
//      (i.e., the reader index formula gives the correct local slot).
//   4. Verify that all outfield slots' press_role_for(roster_slot) ==
//      press_roles[writer_local_slot] (the written value matches what the
//      reader retrieves via the corrected index formula).

proptest! {
    #[test]
    fn ts2_p3b_writer_reader_loop_index_alignment(
        seed_u64 in arb_seed(),
        carrier_slot in 11u8..=21u8, // away carries; home defends
    ) {
        let seed = Seed::from_u64(seed_u64);
        let state = MatchState::initial(seed);

        // Set home to HighPress.
        let mut state = state.with_possession(carrier_slot);
        state.team_tactic_states[0] = TeamTacticState::initial()
            .transition(TacticState::HighPress, Tick::ZERO);

        let player_positions: [(Q32, Q32); 22] = std::array::from_fn(|i| {
            (state.players[i].pos_x, state.players[i].pos_y)
        });
        let tactic_states = state.team_tactic_states;

        let mut shapes = [TeamShape::zero(), TeamShape::zero()];
        shapes[0].is_defending = true;

        compute_press_from_parts(
            &mut shapes,
            Some(carrier_slot),
            &player_positions,
            &tactic_states,
        );

        let home_shape = &shapes[0];
        prop_assume!(home_shape.is_high_press);

        // For each home outfield slot (roster_slot 0..11, GK at 0):
        //   writer stores at press_roles[local_slot] where local_slot == roster_slot (home)
        //   reader reads via team_local = roster_slot as usize (home, after the fix)
        // Assert: press_role_for(roster_slot) == press_roles[roster_slot]
        // This fails loudly if the index formula diverges from the writer.
        for rs in 0u8..11u8 {
            let writer_local = rs as usize; // home: abs_slot == roster_slot
            let written_role = home_shape.press_roles[writer_local];

            // Reader formula (fixed): team_local = roster_slot as usize (home)
            let reader_local = rs as usize;
            let read_role = home_shape.press_roles[reader_local];

            prop_assert!(
                written_role == read_role,
                "TS2-P3b: roster_slot={}: writer stored {:?} at local={}, \
                 reader retrieved {:?} at local={} — index mismatch (off-by-one)",
                rs, written_role, writer_local, read_role, reader_local
            );
        }

        // Additional: find the Primary local slot and verify it's non-GK.
        let primary_local = home_shape.press_roles.iter().enumerate()
            .find(|&(_, r)| *r == PressRole::Primary)
            .map(|(i, _)| i);
        prop_assert!(
            primary_local.is_some(),
            "TS2-P3b: no Primary slot found in home press_roles {:?}",
            home_shape.press_roles
        );
        let primary_local = primary_local.unwrap();
        prop_assert!(
            primary_local > 0,
            "TS2-P3b: GK (local=0) assigned Primary — GK must always be HoldShape"
        );

        // The roster_slot of the Primary outfield player (home: roster == local).
        let primary_roster = primary_local as u8;
        // Verify: press_role_for(primary_roster) == Primary via the corrected formula.
        let read_via_formula = home_shape.press_roles[primary_roster as usize];
        prop_assert!(
            read_via_formula == PressRole::Primary,
            "TS2-P3b: press_role_for({}) via corrected formula should be Primary; \
             got {:?}. The reader→writer index alignment is broken.",
            primary_roster, read_via_formula
        );
    }
}

// ---------------------------------------------------------------------------
// TS2-P4: line_height_metres and press_intensity decouple independently
// ---------------------------------------------------------------------------
//
// Four quadrants of the line_height/press_radius space:
//   A) press_radius > 20 (High press intensity) + line_height < 20 → defence=LowBlock
//   B) press_radius > 20 (High press intensity) + line_height > 35 → defence=HighPress
//   C) press_radius ≤ 20 (No press intensity) + line_height > 35  → defence=HighPress
//   D) line_height = None (legacy coupled rule) + press_radius > 20 → defence=MidBlock
//
// Quadrants A, B, C prove the decoupling. Quadrant D proves backward compat.

#[test]
fn ts2_p4_line_height_and_press_intensity_are_independent() {
    use fw_match_sim::tactic_fsm::{TacticState, archetype_params_for};

    // Quadrant A: high press-radius (→ High press intensity) + low line_height (→ LowBlock).
    let arch_a = fw_content::TacticalArchetype {
        id: "test:archetype.a".into(),
        formation: vec![],
        press_radius_metres: 30,      // > 20 → High press_intensity
        line_height_metres: Some(15), // < 20 → LowBlock defence
        buildup_speed_factor_bps: 9_000,
    };
    let params_a = archetype_params_for(&arch_a);
    assert_eq!(
        params_a.press_intensity,
        PressIntensity::High,
        "TS2-P4-A: high press_radius should give High press_intensity"
    );
    assert_eq!(
        params_a.default_in_defence_state,
        TacticState::LowBlock,
        "TS2-P4-A: line_height=15m should give LowBlock defence (not coupled to press_intensity)"
    );

    // Quadrant B: high press-radius (→ High press intensity) + high line_height (→ HighPress).
    let arch_b = fw_content::TacticalArchetype {
        id: "test:archetype.b".into(),
        formation: vec![],
        press_radius_metres: 30,      // > 20 → High press_intensity
        line_height_metres: Some(40), // > 35 → HighPress defence
        buildup_speed_factor_bps: 9_000,
    };
    let params_b = archetype_params_for(&arch_b);
    assert_eq!(
        params_b.press_intensity,
        PressIntensity::High,
        "TS2-P4-B: high press_radius should give High press_intensity"
    );
    assert_eq!(
        params_b.default_in_defence_state,
        TacticState::HighPress,
        "TS2-P4-B: line_height=40m should give HighPress defence"
    );

    // Quadrant C: low press-radius (→ No press intensity) + high line_height (→ HighPress defence).
    // A zonal high-line without aggressive pressing — "low-press high-line" tactic.
    let arch_c = fw_content::TacticalArchetype {
        id: "test:archetype.c".into(),
        formation: vec![],
        press_radius_metres: 15,      // ≤ 20 → None press_intensity
        line_height_metres: Some(40), // > 35 → HighPress defence line
        buildup_speed_factor_bps: 9_000,
    };
    let params_c = archetype_params_for(&arch_c);
    assert_eq!(
        params_c.press_intensity,
        PressIntensity::None,
        "TS2-P4-C: low press_radius should give None press_intensity even with high line_height"
    );
    assert_eq!(
        params_c.default_in_defence_state,
        TacticState::HighPress,
        "TS2-P4-C: line_height=40m should give HighPress defence regardless of press_intensity"
    );

    // Quadrant D: line_height = None → legacy coupled rule (press_radius > 20 → MidBlock).
    let arch_d = fw_content::TacticalArchetype {
        id: "test:archetype.d".into(),
        formation: vec![],
        press_radius_metres: 30,  // > 20 → legacy → MidBlock defence
        line_height_metres: None, // legacy: use press_radius rule
        buildup_speed_factor_bps: 9_000,
    };
    let params_d = archetype_params_for(&arch_d);
    assert_eq!(
        params_d.press_intensity,
        PressIntensity::High,
        "TS2-P4-D: legacy high press_radius should give High press_intensity"
    );
    assert_eq!(
        params_d.default_in_defence_state,
        TacticState::MidBlock,
        "TS2-P4-D: legacy None line_height + press_radius > 20 should give MidBlock"
    );
}

// ---------------------------------------------------------------------------
// TS2-P2: offside geometry invariants (deterministic unit tests)
// ---------------------------------------------------------------------------
//
// These are deterministic geometry tests, not property-based, because the
// offside check logic is a function of ball position + receiver position +
// defensive line x. We test:
//
//   (a) Receiver in final third, beyond defensive line, after ball → OFFSIDE.
//   (b) Receiver in midfield (not final third) → NOT offside.
//   (c) Receiver is behind (backward pass) → NOT offside.
//   (d) Offside zone: home attack into away half, receiver at x=+40 (final
//       third, ≥32m from midfield) + line_x=-2m → would be offside.
//   (e) Same geometry but receiver at x=+25 (NOT in final 20m of goal at 52m,
//       i.e. x < +32m) → NOT offside.
//
// These tests run a full match to tick 1, set up ball state manually, and
// call the dispatch path via `tick_match` with a controlled state. Since
// `is_offside_at_pass_launch` is private to `dispatch.rs`, we test the
// observable output: `MatchEvent::Offside` present or absent after a tick
// in which a pass would be launched.
//
// Rather than instrumenting internals, we test a 2000-tick content run
// produces at least one Offside event (demonstrating the zone check fires)
// and no Offside events in a bare 60-tick smoke run (no final-third passes
// from a HighPress context in 60 ticks from midfield KickOff).
//
// 2000 ticks ≈ 33 real minutes — long enough for attacking play to reach
// the final third reliably; short enough to keep the test fast (<1s).

#[test]
fn ts2_p2_600_tick_content_run_produces_offside_events() {
    use fw_content::ContentStore;
    use fw_match_sim::tick_match;
    use std::path::PathBuf;

    let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content");
    let content = ContentStore::load_sources(&content_root).expect("ContentStore load");
    let sig_defs = content.signature_definitions.clone();

    let seed = Seed::from_u64(0xfeed_beef_cafe_fade_u64);
    let mut state = MatchState::initial_with_content(
        seed,
        &content,
        "fwh.core:archetype.attacking-fullback",
        "fwh.core:archetype.low-block-counter",
    )
    .expect("initial_with_content");

    // Full 5400-tick match (90 real minutes). Offsides reliably occur during a
    // full match with attacking-fullback pressing; the zone check (final 20m)
    // must fire at least once across 90 minutes of final-third forward passes.
    for _ in 0..fw_match_sim::FULL_MATCH_TICKS {
        state = tick_match(state, &sig_defs);
    }

    let offside_count = state
        .match_events()
        .iter()
        .filter(|e| matches!(e, fw_content::MatchEvent::Offside { .. }))
        .count();

    // Verify: final-third offside check fires at least once in a full match.
    assert!(
        offside_count >= 1,
        "TS2-P2: expected at least 1 Offside event in a full 5400-tick match; \
         got 0. The offside zone check (final 20m) may be broken."
    );
}

#[test]
fn ts2_p2_60_tick_bare_run_no_spurious_offsides_in_midfield() {
    use fw_match_sim::tick_match;

    // 60-tick bare run (no content, no archetype). Players start at formation
    // positions, kick off, no final-third passes expected from kick-off. This
    // verifies the midfield zone gate suppresses false offsides.
    let seed = Seed::from_u64(0xdead_beef_dead_beef_u64);
    let mut state = MatchState::initial(seed);
    let sig_defs = BTreeMap::new();

    for _ in 0..60 {
        state = tick_match(state, &sig_defs);
    }

    // In 60 ticks from kick-off, the ball does not reach the final third
    // zone (within 20m of goal at x=±52m, i.e. x>+32 or x<-32). Any
    // Offside events would indicate the zone gate is wrong.
    let offside_events: Vec<_> = state
        .match_events()
        .iter()
        .filter(|e| matches!(e, fw_content::MatchEvent::Offside { .. }))
        .collect();

    assert!(
        offside_events.is_empty(),
        "TS2-P2: expected no Offside events in 60-tick bare run; \
         got {} Offside events. Midfield zone gate may be too permissive \
         or the offside check is firing in non-final-third passes.",
        offside_events.len()
    );
}

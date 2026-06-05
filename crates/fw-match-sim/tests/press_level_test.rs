//! S11 — ChangePressLevel bite-proof tests.
//!
//! Requirement: show that setting `High` vs `Low` press level produces a
//! MEASURABLE, CORRECT difference in the sim's defensive-shape output
//! (pressing line position + coordinated press-role assignments).
//!
//! The tests here trace the press-level input all the way to the shape output
//! so the assertion compares real behavioral output, not a tautology.
//!
//! Determinism: all arithmetic Q32, no floats, no clocks.

use fw_core::{Seed, Tick};
use fw_match_sim::tactic_fsm::{TacticState, TeamTacticState};
use fw_match_sim::team_shape::{PressRole, SimPressLevel, compute, compute_press_from_parts};
use fw_match_sim::{MatchState, tick_match};

// ---------------------------------------------------------------------------
// Helper — build a MatchState with specific tactic state + press level
// ---------------------------------------------------------------------------

fn mk_state(
    home_tactic: TacticState,
    home_press: SimPressLevel,
    away_tactic: TacticState,
    away_press: SimPressLevel,
) -> MatchState {
    let seed = Seed::from_u64(0xBEEF_CAFE_DEAD_1234);
    let mut state = MatchState::initial(seed);
    state.team_tactic_states[0] = TeamTacticState::initial().transition(home_tactic, Tick::ZERO);
    state.team_tactic_states[1] = TeamTacticState::initial().transition(away_tactic, Tick::ZERO);
    state.set_press_level(0, home_press);
    state.set_press_level(1, away_press);
    state
}

// ---------------------------------------------------------------------------
// Test 1 — pressing line: High advances, Low retreats, Standard unchanged
// ---------------------------------------------------------------------------

/// High press raises the home team's defensive line compared to Standard.
/// Low press lowers it. Direction: home team defends -x goal, so "forward"
/// = more positive x.
#[test]
fn press_level_high_pushes_line_forward_low_retreats_home_midblock() {
    let state_standard = mk_state(
        TacticState::MidBlock,
        SimPressLevel::Standard,
        TacticState::MidBlock,
        SimPressLevel::Standard,
    );
    let state_high = mk_state(
        TacticState::MidBlock,
        SimPressLevel::High,
        TacticState::MidBlock,
        SimPressLevel::Standard,
    );
    let state_low = mk_state(
        TacticState::MidBlock,
        SimPressLevel::Low,
        TacticState::MidBlock,
        SimPressLevel::Standard,
    );

    let shape_standard = compute(0, &state_standard);
    let shape_high = compute(0, &state_high);
    let shape_low = compute(0, &state_low);

    // High: line_x should be more positive (further forward) than Standard.
    assert!(
        shape_high.line_x > shape_standard.line_x,
        "High press must push home line_x forward vs Standard: \
         high={:?} standard={:?}",
        shape_high.line_x,
        shape_standard.line_x
    );

    // Low: line_x should be more negative (deeper) than Standard.
    assert!(
        shape_low.line_x < shape_standard.line_x,
        "Low press must pull home line_x back vs Standard: \
         low={:?} standard={:?}",
        shape_low.line_x,
        shape_standard.line_x
    );

    // Ordering: Low < Standard < High.
    assert!(
        shape_low.line_x < shape_high.line_x,
        "Low line_x must be strictly less than High line_x: \
         low={:?} high={:?}",
        shape_low.line_x,
        shape_high.line_x
    );
}

/// Same test for the away team (team_idx=1): away defends +x goal, so
/// "forward" for them = less positive x (toward the home half).
#[test]
fn press_level_high_pushes_line_forward_low_retreats_away_midblock() {
    let state_standard = mk_state(
        TacticState::MidBlock,
        SimPressLevel::Standard,
        TacticState::MidBlock,
        SimPressLevel::Standard,
    );
    let state_high = mk_state(
        TacticState::MidBlock,
        SimPressLevel::Standard,
        TacticState::MidBlock,
        SimPressLevel::High,
    );
    let state_low = mk_state(
        TacticState::MidBlock,
        SimPressLevel::Standard,
        TacticState::MidBlock,
        SimPressLevel::Low,
    );

    let shape_standard = compute(1, &state_standard);
    let shape_high = compute(1, &state_high);
    let shape_low = compute(1, &state_low);

    // Away High: line_x should be less positive (more forward into home half).
    assert!(
        shape_high.line_x < shape_standard.line_x,
        "High press must push away line_x forward (less positive) vs Standard: \
         high={:?} standard={:?}",
        shape_high.line_x,
        shape_standard.line_x
    );

    // Away Low: line_x should be more positive (deeper into their own half).
    assert!(
        shape_low.line_x > shape_standard.line_x,
        "Low press must pull away line_x back (more positive) vs Standard: \
         low={:?} standard={:?}",
        shape_low.line_x,
        shape_standard.line_x
    );
}

// ---------------------------------------------------------------------------
// Test 2 — press-role assignment: High enables Primary/Cover from MidBlock
// ---------------------------------------------------------------------------

/// CORE BITE-PROOF TEST.
///
/// With SimPressLevel::High, a defending team in MidBlock tactic state must
/// get Primary + Cover press-role assignments (as if it were HighPress).
/// With SimPressLevel::Low the same team must get all HoldShape (even if the
/// tactic says HighPress). With SimPressLevel::Standard only HighPress gives
/// Primary/Cover.
///
/// The assertion chain:
///   1. High + MidBlock → Primary/Cover roles assigned (at least one Primary).
///   2. Low + HighPress → all HoldShape (Low suppresses even HighPress).
///   3. Standard + MidBlock → all HoldShape (Standard does not elevate MidBlock).
///   4. Standard + HighPress → Primary/Cover assigned (unchanged baseline).
#[test]
fn press_roles_reflect_press_level_not_just_tactic_state() {
    // Seed with home team having possession (slot 9) so the away team
    // is defending — compute_press_from_parts only assigns roles to the
    // defending team.
    let seed = Seed::from_u64(0xABCD_EF01_2345_6789);

    // Build state variants for the away team (team_idx=1, defender when home has ball).
    let mk = |away_tactic: TacticState, away_press: SimPressLevel| -> MatchState {
        let mut s = MatchState::initial(seed);
        // Home keeps possession throughout (slot 9 = home CF, initial state).
        s.team_tactic_states[1] = TeamTacticState::initial().transition(away_tactic, Tick::ZERO);
        s.set_press_level(1, away_press);
        s
    };

    // Helper: count Primary/Cover roles in the away team (local slots 0..11).
    let count_active_roles = |state: &MatchState| -> usize {
        // Recompute both compute (shape) and compute_press_from_parts.
        let mut shapes = [
            fw_match_sim::team_shape::TeamShape::zero(),
            fw_match_sim::team_shape::TeamShape::zero(),
        ];
        shapes[0] = compute(0, state);
        shapes[1] = compute(1, state);

        let pos_snap: [(fw_core::Q32, fw_core::Q32); 22] = {
            let mut arr = [(fw_core::Q32::ZERO, fw_core::Q32::ZERO); 22];
            for (i, p) in state.players.iter().enumerate() {
                arr[i] = (p.pos_x, p.pos_y);
            }
            arr
        };
        compute_press_from_parts(
            &mut shapes,
            state.possession(),
            &pos_snap,
            &state.team_tactic_states,
            state.press_level(),
        );

        // Count non-HoldShape roles for the away team (team-local slots 0..11).
        shapes[1]
            .press_roles
            .iter()
            .filter(|&&r| r != PressRole::HoldShape)
            .count()
    };

    // (1) High + MidBlock → Primary/Cover roles assigned (must have ≥ 3 active).
    let state_high_mid = mk(TacticState::MidBlock, SimPressLevel::High);
    let active_high_mid = count_active_roles(&state_high_mid);
    assert!(
        active_high_mid >= 3,
        "High press + MidBlock: expected ≥ 3 Primary/Cover roles \
         (1 Primary + 2 Cover); got {active_high_mid}"
    );

    // (2) Low + HighPress → all HoldShape (Low suppresses Primary/Cover).
    let state_low_high = mk(TacticState::HighPress, SimPressLevel::Low);
    let active_low_high = count_active_roles(&state_low_high);
    assert_eq!(
        active_low_high, 0,
        "Low press + HighPress: expected 0 Primary/Cover roles; got {active_low_high}"
    );

    // (3) Standard + MidBlock → all HoldShape (no elevation of MidBlock).
    let state_std_mid = mk(TacticState::MidBlock, SimPressLevel::Standard);
    let active_std_mid = count_active_roles(&state_std_mid);
    assert_eq!(
        active_std_mid, 0,
        "Standard press + MidBlock: expected 0 Primary/Cover roles; got {active_std_mid}"
    );

    // (4) Standard + HighPress → Primary/Cover assigned (unchanged baseline behavior).
    let state_std_high = mk(TacticState::HighPress, SimPressLevel::Standard);
    let active_std_high = count_active_roles(&state_std_high);
    assert!(
        active_std_high >= 3,
        "Standard press + HighPress: expected ≥ 3 Primary/Cover roles; got {active_std_high}"
    );
}

// ---------------------------------------------------------------------------
// Test 3 — default preservation: Standard == current behavior (no drift)
// ---------------------------------------------------------------------------

/// Running a match at Standard press level must produce byte-identical output
/// to a match state built without ever calling set_press_level (the prior
/// default). This directly proves the pin-preservation invariant.
#[test]
fn standard_press_level_is_byte_identical_to_default() {
    let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);

    // Path A: never call set_press_level (the historical path).
    let mut state_a = MatchState::initial(seed);
    for _ in 0..60u32 {
        state_a = tick_match(state_a, &std::collections::BTreeMap::new());
    }

    // Path B: explicitly set Standard for both teams.
    let mut state_b = MatchState::initial(seed);
    state_b.set_press_level(0, SimPressLevel::Standard);
    state_b.set_press_level(1, SimPressLevel::Standard);
    for _ in 0..60u32 {
        state_b = tick_match(state_b, &std::collections::BTreeMap::new());
    }

    // Canonical encoding must be byte-identical.
    let bytes_a = state_a.encode_canonical();
    let bytes_b = state_b.encode_canonical();
    assert_eq!(
        bytes_a, bytes_b,
        "Standard press level must produce byte-identical canonical state \
         to the historical default path — pin drift detected"
    );
}

// ---------------------------------------------------------------------------
// Test 4 — insta snapshot of High press shape at tick 0
// ---------------------------------------------------------------------------

#[test]
fn press_level_high_home_midblock_shape_snapshot() {
    let state = mk_state(
        TacticState::MidBlock,
        SimPressLevel::High,
        TacticState::MidBlock,
        SimPressLevel::Standard,
    );
    let shape = compute(0, &state);
    // Snapshot the line_x raw bits so CI catches any drift in the offset constant.
    insta::assert_debug_snapshot!("press_level_high_home_midblock_line_x", shape.line_x);
}

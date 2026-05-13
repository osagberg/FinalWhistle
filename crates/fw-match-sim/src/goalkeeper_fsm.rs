//! Goalkeeper pure-FSM decision functions.
//!
//! ADR-0006 §"Decision": "Goalkeeper is pure FSM, no inner BT. GK behavior
//! is mode-dominated; ~10-12 states each implemented as a small Rust function."
//!
//! ## Transition predicates (T1-2b-iii-c P0-2)
//!
//! `evaluate_transitions` uses spatial inputs from `BallState` to decide the
//! GK's next mode. Priority order (highest → lowest):
//!
//! 1. `ShotStopping` — ball in own penalty area AND ball moving toward own goal.
//! 2. `SweeperKeeperRush` — ball in own half outside penalty area AND moving
//!    toward own goal.
//! 3. `DistributingFromHand` — ball very close to GK's goal line (proxy for
//!    GK in possession after catch).
//! 4. `InBoxPositioning` — default.
//!
//! All Q32 constants use pitch coordinates: X = attacking axis
//! (home goal at x≈-45, away goal at x≈+45). Z = touchline axis.
//!
//! ## Determinism
//!
//! `evaluate_transitions` is pure — no floats in canonical state, no RNG
//! draws, no clocks. Inputs are Q32 from canonical `BallState`.

use rand_chacha::ChaCha8Rng;

use crate::ball::BallState;
use crate::role_states::{GoalkeeperState, PlayerIntent};
use fw_core::Q32;

// ---------------------------------------------------------------------------
// Spatial thresholds (all in metres, Q32.32)
// ---------------------------------------------------------------------------

/// Penalty-area boundary: within 16m of goal line (proxy for 18-yard box).
/// Home penalty area: pos_x < -29  (−45 + 16).
/// Away penalty area: pos_x > +29  (+45 − 16).
const PENALTY_AREA_DEPTH: Q32 = Q32::from_raw(16_i64 << 32); // 16.0

/// Home goal line x-coordinate (−45 m).
const HOME_GOAL_X: Q32 = Q32::from_raw(-45_i64 << 32); // −45.0

/// Away goal line x-coordinate (+45 m).
const AWAY_GOAL_X: Q32 = Q32::from_raw(45_i64 << 32); // +45.0

/// Distribution trigger: ball within 3 m of GK goal line (proxy for catch).
const DISTRIBUTION_THRESHOLD: Q32 = Q32::from_raw(3_i64 << 32); // 3.0

// ---------------------------------------------------------------------------
// GK tick entry point
// ---------------------------------------------------------------------------

/// Tick the goalkeeper FSM for one decision cycle.
///
/// Returns `(new_state, intent)`. `evaluate_transitions` uses spatial
/// inputs from `ball` to select the appropriate mode.
///
/// `roster_slot` is 0-indexed (slot 0 = home GK, slot 11 = away GK).
#[must_use]
pub fn tick_goalkeeper(
    current_state: GoalkeeperState,
    roster_slot: u8,
    ball: &BallState,
    _rng: &mut ChaCha8Rng,
) -> (GoalkeeperState, PlayerIntent) {
    let next_state = evaluate_transitions(current_state, roster_slot, ball);
    let intent = dispatch_state(next_state, roster_slot);
    (next_state, intent)
}

/// Evaluate FSM transitions using ball-position predicates.
///
/// Priority:
/// 1. ShotStopping — ball in own penalty area + ball approaching own goal.
/// 2. SweeperKeeperRush — ball in own half (outside penalty area) + approaching goal.
/// 3. DistributingFromHand — ball within `DISTRIBUTION_THRESHOLD` of goal line.
/// 4. InBoxPositioning — default.
fn evaluate_transitions(
    _current: GoalkeeperState,
    roster_slot: u8,
    ball: &BallState,
) -> GoalkeeperState {
    let is_home = (roster_slot as usize) < 11;
    let bx = ball.pos_x;
    let bvx = ball.vel_x;

    if is_home {
        // Home GK: goal line at x = −45; penalty area x < −29 (= −45 + 16).
        let penalty_boundary = HOME_GOAL_X + PENALTY_AREA_DEPTH; // = −29
        let in_own_half = bx < Q32::ZERO;
        let in_penalty_area = bx < penalty_boundary;
        // Ball approaching home goal: vel_x < 0.
        let approaching_goal = bvx < Q32::ZERO;
        // Dist of ball from goal line.
        let dist_from_line = if bx > HOME_GOAL_X {
            bx - HOME_GOAL_X
        } else {
            Q32::ZERO
        };

        if in_penalty_area && approaching_goal {
            GoalkeeperState::ShotStopping
        } else if in_own_half && !in_penalty_area && approaching_goal {
            GoalkeeperState::SweeperKeeperRush
        } else if dist_from_line < DISTRIBUTION_THRESHOLD {
            GoalkeeperState::DistributingFromHand
        } else {
            GoalkeeperState::InBoxPositioning
        }
    } else {
        // Away GK: goal line at x = +45; penalty area x > +29 (= +45 − 16).
        let penalty_boundary = AWAY_GOAL_X - PENALTY_AREA_DEPTH; // = +29
        let in_own_half = bx > Q32::ZERO;
        let in_penalty_area = bx > penalty_boundary;
        // Ball approaching away goal: vel_x > 0.
        let approaching_goal = bvx > Q32::ZERO;
        // Dist of ball from goal line.
        let dist_from_line = if bx < AWAY_GOAL_X {
            AWAY_GOAL_X - bx
        } else {
            Q32::ZERO
        };

        if in_penalty_area && approaching_goal {
            GoalkeeperState::ShotStopping
        } else if in_own_half && !in_penalty_area && approaching_goal {
            GoalkeeperState::SweeperKeeperRush
        } else if dist_from_line < DISTRIBUTION_THRESHOLD {
            GoalkeeperState::DistributingFromHand
        } else {
            GoalkeeperState::InBoxPositioning
        }
    }
}

/// Dispatch to the per-state intent function.
pub(crate) fn dispatch_state(state: GoalkeeperState, roster_slot: u8) -> PlayerIntent {
    match state {
        GoalkeeperState::InBoxPositioning => gk_in_box_positioning(roster_slot),
        GoalkeeperState::SweeperKeeperRush => gk_sweeper_rush(roster_slot),
        GoalkeeperState::ShotStopping => gk_shot_stopping(roster_slot),
        GoalkeeperState::DistributingFromHand => gk_distributing_from_hand(roster_slot),
        GoalkeeperState::DistributingFromFeet => gk_distributing_from_feet(roster_slot),
        GoalkeeperState::PenaltyStance => gk_penalty_stance(roster_slot),
        GoalkeeperState::SetPieceWall => gk_set_piece_wall(roster_slot),
        GoalkeeperState::Recovering => gk_recovering(roster_slot),
    }
}

// ---------------------------------------------------------------------------
// Per-state intent stubs — skeleton tier
// ---------------------------------------------------------------------------

/// Goal-line position for the GK (from the formation table).
fn gk_goal_line_position(roster_slot: u8) -> (Q32, Q32) {
    use crate::subtree_library::formation_position;
    formation_position(roster_slot)
}

fn gk_in_box_positioning(roster_slot: u8) -> PlayerIntent {
    let (target_x, target_y) = gk_goal_line_position(roster_slot);
    PlayerIntent::MoveToPosition { target_x, target_y }
}

fn gk_sweeper_rush(roster_slot: u8) -> PlayerIntent {
    let (gx, gy) = gk_goal_line_position(roster_slot);
    let target_x = if (roster_slot as usize) < 11 {
        gx + Q32::from_int(5) // home GK rushes toward +x
    } else {
        gx - Q32::from_int(5) // away GK rushes toward −x
    };
    PlayerIntent::GkSweeperRush {
        target_x,
        target_y: gy,
    }
}

fn gk_shot_stopping(roster_slot: u8) -> PlayerIntent {
    let (target_x, target_y) = gk_goal_line_position(roster_slot);
    PlayerIntent::GkShotStop { target_x, target_y }
}

fn gk_distributing_from_hand(roster_slot: u8) -> PlayerIntent {
    let dist_slot: u8 = if (roster_slot as usize) < 11 { 1 } else { 12 };
    use crate::subtree_library::formation_position;
    let (target_x, target_y) = formation_position(dist_slot);
    PlayerIntent::GkDistributeShort { target_x, target_y }
}

fn gk_distributing_from_feet(roster_slot: u8) -> PlayerIntent {
    let dist_slot: u8 = if (roster_slot as usize) < 11 { 6 } else { 17 };
    use crate::subtree_library::formation_position;
    let (target_x, target_y) = formation_position(dist_slot);
    PlayerIntent::GkDistributeLong { target_x, target_y }
}

fn gk_penalty_stance(roster_slot: u8) -> PlayerIntent {
    let (target_x, target_y) = gk_goal_line_position(roster_slot);
    PlayerIntent::MoveToPosition { target_x, target_y }
}

fn gk_set_piece_wall(roster_slot: u8) -> PlayerIntent {
    let (target_x, target_y) = gk_goal_line_position(roster_slot);
    PlayerIntent::MoveToPosition { target_x, target_y }
}

fn gk_recovering(roster_slot: u8) -> PlayerIntent {
    let (target_x, target_y) = gk_goal_line_position(roster_slot);
    PlayerIntent::MoveToPosition { target_x, target_y }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::ChaCha8Rng;
    use rand_chacha::rand_core::SeedableRng;

    fn mk_rng() -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(0)
    }

    fn ball_at(bx: i32, bvx: i32) -> BallState {
        BallState {
            pos_x: Q32::from_int(bx),
            pos_y: Q32::ZERO,
            pos_z: Q32::ZERO,
            vel_x: Q32::from_int(bvx),
            vel_y: Q32::ZERO,
            vel_z: Q32::ZERO,
            spin_x: Q32::ZERO,
            spin_y: Q32::ZERO,
            spin_z: Q32::ZERO,
        }
    }

    // --- evaluate_transitions predicates (home GK, slot 0) ---

    #[test]
    fn home_gk_ball_in_penalty_area_approaching_gives_shot_stopping() {
        // Ball at x=−33 (inside penalty area boundary at −29) moving toward home goal (vel_x < 0).
        let ball = ball_at(-33, -5);
        let state = evaluate_transitions(GoalkeeperState::InBoxPositioning, 0, &ball);
        assert_eq!(
            state,
            GoalkeeperState::ShotStopping,
            "ball in penalty area + approaching goal must give ShotStopping"
        );
    }

    #[test]
    fn home_gk_ball_in_own_half_approaching_gives_sweeper_rush() {
        // Ball at x=−15 (own half, outside penalty area) moving toward home goal.
        let ball = ball_at(-15, -3);
        let state = evaluate_transitions(GoalkeeperState::InBoxPositioning, 0, &ball);
        assert_eq!(
            state,
            GoalkeeperState::SweeperKeeperRush,
            "ball in own half + approaching goal must give SweeperKeeperRush"
        );
    }

    #[test]
    fn home_gk_ball_near_goal_line_gives_distributing() {
        // Ball at x=−43 (within 3m of goal line at −45), not approaching.
        let ball = ball_at(-43, 2);
        let state = evaluate_transitions(GoalkeeperState::InBoxPositioning, 0, &ball);
        assert_eq!(
            state,
            GoalkeeperState::DistributingFromHand,
            "ball near goal line must give DistributingFromHand"
        );
    }

    #[test]
    fn home_gk_ball_in_attacking_half_gives_in_box_positioning() {
        // Ball in attacking half — GK defaults to InBoxPositioning.
        let ball = ball_at(20, 3);
        let state = evaluate_transitions(GoalkeeperState::InBoxPositioning, 0, &ball);
        assert_eq!(
            state,
            GoalkeeperState::InBoxPositioning,
            "ball in attacking half must give InBoxPositioning"
        );
    }

    // --- evaluate_transitions predicates (away GK, slot 11) ---

    #[test]
    fn away_gk_ball_in_penalty_area_approaching_gives_shot_stopping() {
        // Ball at x=+33 (inside away penalty area at +29) moving toward away goal (vel_x > 0).
        let ball = ball_at(33, 5);
        let state = evaluate_transitions(GoalkeeperState::InBoxPositioning, 11, &ball);
        assert_eq!(state, GoalkeeperState::ShotStopping);
    }

    #[test]
    fn away_gk_ball_in_own_half_approaching_gives_sweeper_rush() {
        let ball = ball_at(15, 3);
        let state = evaluate_transitions(GoalkeeperState::InBoxPositioning, 11, &ball);
        assert_eq!(state, GoalkeeperState::SweeperKeeperRush);
    }

    #[test]
    fn away_gk_ball_near_goal_line_gives_distributing() {
        let ball = ball_at(43, -2);
        let state = evaluate_transitions(GoalkeeperState::InBoxPositioning, 11, &ball);
        assert_eq!(state, GoalkeeperState::DistributingFromHand);
    }

    // --- tick_goalkeeper wiring ---

    #[test]
    fn tick_goalkeeper_shot_stopping_scenario_reaches_shot_stopping_state() {
        let ball = ball_at(-33, -5);
        let (state, intent) =
            tick_goalkeeper(GoalkeeperState::InBoxPositioning, 0, &ball, &mut mk_rng());
        assert_eq!(state, GoalkeeperState::ShotStopping);
        assert!(
            matches!(intent, PlayerIntent::GkShotStop { .. }),
            "ShotStopping must produce GkShotStop intent; got {:?}",
            intent
        );
    }

    #[test]
    fn tick_goalkeeper_sweeper_scenario_reaches_sweeper_state() {
        let ball = ball_at(-15, -3);
        let (state, intent) =
            tick_goalkeeper(GoalkeeperState::InBoxPositioning, 0, &ball, &mut mk_rng());
        assert_eq!(state, GoalkeeperState::SweeperKeeperRush);
        assert!(matches!(intent, PlayerIntent::GkSweeperRush { .. }));
    }

    // --- Goal-line intent ---

    #[test]
    fn home_gk_slot0_in_box_positioning_returns_goal_line_position() {
        let ball = ball_at(20, 3); // attacking half → InBoxPositioning
        let (_, intent) =
            tick_goalkeeper(GoalkeeperState::InBoxPositioning, 0, &ball, &mut mk_rng());
        match intent {
            PlayerIntent::MoveToPosition { target_x, target_y } => {
                assert_eq!(target_x, Q32::from_int(-45), "home GK should target x=−45");
                assert_eq!(target_y, Q32::from_int(0), "home GK should target y=0");
            }
            other => panic!(
                "GK returned unexpected intent {:?}; expected MoveToPosition",
                other
            ),
        }
    }

    #[test]
    fn away_gk_slot11_in_box_positioning_returns_goal_line_position() {
        let ball = ball_at(-20, -3); // attacking half for away → InBoxPositioning
        let (_, intent) =
            tick_goalkeeper(GoalkeeperState::InBoxPositioning, 11, &ball, &mut mk_rng());
        match intent {
            PlayerIntent::MoveToPosition { target_x, target_y } => {
                assert_eq!(target_x, Q32::from_int(45), "away GK should target x=+45");
                assert_eq!(target_y, Q32::from_int(0), "away GK should target y=0");
            }
            other => panic!(
                "GK returned unexpected intent {:?}; expected MoveToPosition",
                other
            ),
        }
    }

    // --- All state variants produce a non-Idle intent ---

    #[test]
    fn every_gk_state_produces_non_idle_intent() {
        let all_states = [
            GoalkeeperState::InBoxPositioning,
            GoalkeeperState::SweeperKeeperRush,
            GoalkeeperState::ShotStopping,
            GoalkeeperState::DistributingFromHand,
            GoalkeeperState::DistributingFromFeet,
            GoalkeeperState::PenaltyStance,
            GoalkeeperState::SetPieceWall,
            GoalkeeperState::Recovering,
        ];
        for state in all_states {
            let intent = dispatch_state(state, 0);
            assert!(
                !matches!(intent, PlayerIntent::Idle),
                "GK state {:?} produced Idle; expected a positional intent",
                state
            );
        }
    }

    // --- dispatch_state produces correct intent variants ---

    #[test]
    fn dispatch_state_sweeper_rush_returns_gk_sweeper_rush_intent() {
        let intent = dispatch_state(GoalkeeperState::SweeperKeeperRush, 0);
        assert!(
            matches!(intent, PlayerIntent::GkSweeperRush { .. }),
            "dispatch_state(SweeperKeeperRush) should produce GkSweeperRush intent; got {:?}",
            intent
        );
    }

    #[test]
    fn dispatch_state_shot_stopping_returns_gk_shot_stop_intent() {
        let intent = dispatch_state(GoalkeeperState::ShotStopping, 0);
        assert!(
            matches!(intent, PlayerIntent::GkShotStop { .. }),
            "dispatch_state(ShotStopping) should produce GkShotStop intent; got {:?}",
            intent
        );
    }

    #[test]
    fn dispatch_state_distributing_from_hand_returns_gk_distribute_short() {
        let intent = dispatch_state(GoalkeeperState::DistributingFromHand, 0);
        assert!(
            matches!(intent, PlayerIntent::GkDistributeShort { .. }),
            "dispatch_state(DistributingFromHand) should produce GkDistributeShort; got {:?}",
            intent
        );
    }

    #[test]
    fn dispatch_state_distributing_from_feet_returns_gk_distribute_long() {
        let intent = dispatch_state(GoalkeeperState::DistributingFromFeet, 0);
        assert!(
            matches!(intent, PlayerIntent::GkDistributeLong { .. }),
            "dispatch_state(DistributingFromFeet) should produce GkDistributeLong; got {:?}",
            intent
        );
    }

    // --- Determinism: same inputs → same outputs ---

    #[test]
    fn gk_tick_is_deterministic() {
        let ball = ball_at(-33, -5); // ShotStopping scenario
        let (s1, i1) = tick_goalkeeper(GoalkeeperState::InBoxPositioning, 0, &ball, &mut mk_rng());
        let (s2, i2) = tick_goalkeeper(GoalkeeperState::InBoxPositioning, 0, &ball, &mut mk_rng());
        assert_eq!(s1, s2);
        assert_eq!(i1, i2);
    }
}

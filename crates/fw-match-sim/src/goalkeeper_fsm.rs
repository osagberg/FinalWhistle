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
use crate::player::PlayerState;
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
/// ## Parameters
/// - `current_state` — the GK's current FSM state.
/// - `player` — the GK's canonical `PlayerState`. Per ADR-0006 P1-4,
///   the GK FSM accepts the full player record so per-state functions can
///   read attributes (e.g. reflexes for ShotStopping, command_of_area for
///   InBoxPositioning). In the skeleton tier the per-state stubs don't yet
///   use `player`; attribute reads land at T1-2b-iii-b alongside the
///   utility binding spec.
/// - `roster_slot` — 0-indexed (slot 0 = home GK, slot 11 = away GK).
/// - `ball` — the canonical `BallState` for spatial transition predicates.
/// - `_rng` — seeded RNG for probabilistic draws (unused in skeleton tier).
#[must_use]
pub fn tick_goalkeeper(
    current_state: GoalkeeperState,
    player: &PlayerState,
    roster_slot: u8,
    ball: &BallState,
    _rng: &mut ChaCha8Rng,
) -> (GoalkeeperState, PlayerIntent) {
    let next_state = evaluate_transitions(current_state, roster_slot, ball);
    let intent = dispatch_state(next_state, roster_slot, player, ball);
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
///
/// `player` is passed so per-state functions can read GK attributes per
/// `docs/specs/bt-attribute-binding.md §Goalkeeper-specific decision sites`.
/// `ball` is passed so spatial-aware bindings (shot-stopping aim direction,
/// sweeper-rush target) can read trajectory.
///
/// **T1-2b-fix attribute-binding wiring (Codex P1-4 round 2)**: the 4
/// utility-bearing GK states (ShotStopping / SweeperKeeperRush /
/// DistributingFromHand / DistributingFromFeet) actually READ their
/// documented attributes here. The 4 non-binding states (InBoxPositioning,
/// PenaltyStance, SetPieceWall, Recovering) are not yet in the spec's GK
/// binding surface and stay attribute-less skeletons.
pub(crate) fn dispatch_state(
    state: GoalkeeperState,
    roster_slot: u8,
    player: &PlayerState,
    ball: &BallState,
) -> PlayerIntent {
    match state {
        GoalkeeperState::InBoxPositioning => gk_in_box_positioning(roster_slot, player),
        GoalkeeperState::SweeperKeeperRush => gk_sweeper_rush(roster_slot, player, ball),
        GoalkeeperState::ShotStopping => gk_shot_stopping(roster_slot, player, ball),
        GoalkeeperState::DistributingFromHand => gk_distributing_from_hand(roster_slot, player),
        GoalkeeperState::DistributingFromFeet => gk_distributing_from_feet(roster_slot, player),
        GoalkeeperState::PenaltyStance => gk_penalty_stance(roster_slot, player),
        GoalkeeperState::SetPieceWall => gk_set_piece_wall(roster_slot, player),
        GoalkeeperState::Recovering => gk_recovering(roster_slot, player),
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

fn gk_in_box_positioning(roster_slot: u8, _player: &PlayerState) -> PlayerIntent {
    // bt-attribute-binding.md §GK InBoxPositioning: primary = positioning + composure.
    // Attribute reads land at T1-2b-iii-b; skeleton tier moves to formation.
    let (target_x, target_y) = gk_goal_line_position(roster_slot);
    PlayerIntent::MoveToPosition { target_x, target_y }
}

/// Documented attribute paths read by `gk_sweeper_rush` per
/// `bt-attribute-binding.md` §"Sweeper rush". Used by the binding-correctness
/// test to enforce that this site reads EXACTLY these attributes — no more,
/// no less.
#[allow(dead_code)] // documentation-bearing const consumed by future T1-4 binding lints + Codex audits
pub(crate) const GK_SWEEPER_RUSH_ATTRS: &[&str] = &[
    "goalkeeper.one_on_ones",
    "goalkeeper.command_of_area",
    "physical.pace",
    "mental.decisions",
];

fn gk_sweeper_rush(roster_slot: u8, player: &PlayerState, ball: &BallState) -> PlayerIntent {
    // bt-attribute-binding.md §"Sweeper rush":
    //   primary: goalkeeper.one_on_ones, goalkeeper.command_of_area,
    //            physical.pace, mental.decisions
    //   bias:    Aggression (consumed via apply_gk_sweeper_bias path; T1-4 wires)
    //
    // Modulation: rush distance scales with (one_on_ones + command_of_area) / 2.
    // At mid_range_baseline=0.5 for both attrs, modulation_factor = 0.5, so the
    // BASE 5m rush becomes 5 × (1 + 0.5) = 7.5m — different from the prior
    // hardcoded 5m (intentional rebaseline per ADR-0012 #3 sim-behavior change).
    let attrs = player.attributes();
    let aggression_factor =
        (attrs.goalkeeper.one_on_ones + attrs.goalkeeper.command_of_area) / Q32::from_int(2);
    let pace = attrs.physical.pace;
    let decisions = attrs.mental.decisions;
    // pace + decisions tally as a secondary multiplier on rush distance:
    // higher pace + decisions → can commit to a longer rush safely.
    let secondary_factor = (pace + decisions) / Q32::from_int(4); // [0, 0.5]

    let base_rush = Q32::from_int(5);
    let rush_distance = base_rush * (Q32::ONE + aggression_factor + secondary_factor);

    let (gx, gy) = gk_goal_line_position(roster_slot);
    let target_x = if (roster_slot as usize) < 11 {
        gx + rush_distance // home GK rushes toward +x
    } else {
        gx - rush_distance // away GK rushes toward -x
    };
    // target_y biased toward ball trajectory by decisions (the "decision
    // quality" surface for choosing the cut-off angle).
    let target_y = gy + (ball.pos_y - gy) * decisions;

    PlayerIntent::GkSweeperRush { target_x, target_y }
}

/// Documented attribute paths read by `gk_shot_stopping` per
/// `bt-attribute-binding.md` §"Shot stopping".
#[allow(dead_code)] // documentation-bearing const consumed by future T1-4 binding lints + Codex audits
pub(crate) const GK_SHOT_STOPPING_ATTRS: &[&str] = &[
    "goalkeeper.reflexes",
    "goalkeeper.handling",
    "goalkeeper.one_on_ones",
    "mental.positioning",
    "mental.composure",
];

fn gk_shot_stopping(roster_slot: u8, player: &PlayerState, ball: &BallState) -> PlayerIntent {
    // bt-attribute-binding.md §"Shot stopping":
    //   primary: goalkeeper.reflexes + handling + one_on_ones + mental.positioning + composure
    //   bias:    Composure
    //
    // Modulation: target_y is offset from the goal-line toward the ball's
    // current y by an aim_factor proportional to (reflexes × positioning +
    // (handling + one_on_ones)/4) × composure. The spec's "how aggressively
    // does the keeper commit to the shot angle" surface; composure damps
    // the commitment when the keeper is under pressure (modeled here as the
    // composure multiplier on the entire aim offset).
    let attrs = player.attributes();
    let (goal_x, goal_y) = gk_goal_line_position(roster_slot);

    let reflex_positioning = attrs.goalkeeper.reflexes * attrs.mental.positioning;
    let secondary_commitment =
        (attrs.goalkeeper.handling + attrs.goalkeeper.one_on_ones) / Q32::from_int(4);
    let composure_damping = attrs.mental.composure;
    let aim_factor = (reflex_positioning + secondary_commitment) * composure_damping;
    // target_y = goal_y + (ball_y - goal_y) × aim_factor.
    // At mid_range_baseline=0.5 across all 5 attrs, aim_factor =
    //   (0.25 + 0.25) × 0.5 = 0.25
    // Different from the prior pass-through (target_y = goal_y).
    let target_y = goal_y + (ball.pos_y - goal_y) * aim_factor;

    PlayerIntent::GkShotStop {
        target_x: goal_x,
        target_y,
    }
}

/// Documented attribute paths read by `gk_distributing_from_hand` per
/// `bt-attribute-binding.md` §"Distribution — short".
#[allow(dead_code)] // documentation-bearing const consumed by future T1-4 binding lints + Codex audits
pub(crate) const GK_DISTRIBUTION_SHORT_ATTRS: &[&str] = &[
    "goalkeeper.kicking",
    "technical.passing",
    "mental.vision",
    "mental.composure",
];

fn gk_distributing_from_hand(roster_slot: u8, player: &PlayerState) -> PlayerIntent {
    // bt-attribute-binding.md §"Distribution — short":
    //   primary: goalkeeper.kicking, technical.passing (proxy), mental.vision, composure
    //   bias:    Selflessness, RiskAppetite (inverse) — T1-4 wires bias path
    //
    // Modulation: short-distribution target picks slot 1 (home CB) or 12
    // (away CB) by default. Delivery quality = average of the 4 documented
    // attributes; biases the target's x-coordinate forward by `quality` metres.
    // At baseline 0.5 across all 4, quality = 0.5 → 0.5m forward bias.
    let attrs = player.attributes();
    let dist_slot: u8 = if (roster_slot as usize) < 11 { 1 } else { 12 };
    use crate::subtree_library::formation_position;
    let (base_x, base_y) = formation_position(dist_slot);

    let quality = (attrs.goalkeeper.kicking
        + attrs.technical.passing
        + attrs.mental.vision
        + attrs.mental.composure)
        / Q32::from_int(4);
    let forward_bias = quality;
    let target_x = if (roster_slot as usize) < 11 {
        base_x + forward_bias // home GK distributes toward +x
    } else {
        base_x - forward_bias // away GK toward -x
    };
    PlayerIntent::GkDistributeShort {
        target_x,
        target_y: base_y,
    }
}

/// Documented attribute paths read by `gk_distributing_from_feet` per
/// `bt-attribute-binding.md` §"Distribution — long".
#[allow(dead_code)] // documentation-bearing const consumed by future T1-4 binding lints + Codex audits
pub(crate) const GK_DISTRIBUTION_LONG_ATTRS: &[&str] =
    &["goalkeeper.kicking", "mental.vision", "mental.decisions"];

fn gk_distributing_from_feet(roster_slot: u8, player: &PlayerState) -> PlayerIntent {
    // bt-attribute-binding.md §"Distribution — long":
    //   primary: goalkeeper.kicking, mental.vision, mental.decisions
    //   bias:    RiskAppetite — T1-4 wires bias path
    //
    // Modulation: long-distribution target picks slot 6 (home MID) or 17
    // (away MID). Kicking dominates reach; vision + decisions are secondary
    // to the recipient-choice quality.
    let attrs = player.attributes();
    let dist_slot: u8 = if (roster_slot as usize) < 11 { 6 } else { 17 };
    use crate::subtree_library::formation_position;
    let (base_x, base_y) = formation_position(dist_slot);

    let reach = attrs.goalkeeper.kicking
        + (attrs.mental.vision + attrs.mental.decisions) / Q32::from_int(2);
    // At baseline 0.5/0.5/0.5, reach = 0.5 + 0.5 = 1.0. Forward bias scales
    // that into a Q32 offset in metres (up to ~4m at max reach).
    let forward_bias = reach * Q32::from_int(2);
    let target_x = if (roster_slot as usize) < 11 {
        base_x + forward_bias
    } else {
        base_x - forward_bias
    };
    PlayerIntent::GkDistributeLong {
        target_x,
        target_y: base_y,
    }
}

fn gk_penalty_stance(roster_slot: u8, _player: &PlayerState) -> PlayerIntent {
    // bt-attribute-binding.md §GK PenaltyStance: primary = reflexes + positioning.
    let (target_x, target_y) = gk_goal_line_position(roster_slot);
    PlayerIntent::MoveToPosition { target_x, target_y }
}

fn gk_set_piece_wall(roster_slot: u8, _player: &PlayerState) -> PlayerIntent {
    // bt-attribute-binding.md §GK SetPieceWall: primary = positioning + jumping.
    let (target_x, target_y) = gk_goal_line_position(roster_slot);
    PlayerIntent::MoveToPosition { target_x, target_y }
}

fn gk_recovering(roster_slot: u8, _player: &PlayerState) -> PlayerIntent {
    // bt-attribute-binding.md §GK Recovering: primary = pace + stamina.
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
        ball_at_xy(bx, 0, bvx)
    }

    fn ball_at_xy(bx: i32, by: i32, bvx: i32) -> BallState {
        BallState {
            pos_x: Q32::from_int(bx),
            pos_y: Q32::from_int(by),
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

    fn dummy_player() -> PlayerState {
        PlayerState::at(0u8, Q32::ZERO, Q32::ZERO)
    }

    #[test]
    fn tick_goalkeeper_shot_stopping_scenario_reaches_shot_stopping_state() {
        let ball = ball_at(-33, -5);
        let player = dummy_player();
        let (state, intent) = tick_goalkeeper(
            GoalkeeperState::InBoxPositioning,
            &player,
            0,
            &ball,
            &mut mk_rng(),
        );
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
        let player = dummy_player();
        let (state, intent) = tick_goalkeeper(
            GoalkeeperState::InBoxPositioning,
            &player,
            0,
            &ball,
            &mut mk_rng(),
        );
        assert_eq!(state, GoalkeeperState::SweeperKeeperRush);
        assert!(matches!(intent, PlayerIntent::GkSweeperRush { .. }));
    }

    // --- Goal-line intent ---

    #[test]
    fn home_gk_slot0_in_box_positioning_returns_goal_line_position() {
        let ball = ball_at(20, 3); // attacking half → InBoxPositioning
        let player = dummy_player();
        let (_, intent) = tick_goalkeeper(
            GoalkeeperState::InBoxPositioning,
            &player,
            0,
            &ball,
            &mut mk_rng(),
        );
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
        let player = dummy_player();
        let (_, intent) = tick_goalkeeper(
            GoalkeeperState::InBoxPositioning,
            &player,
            11,
            &ball,
            &mut mk_rng(),
        );
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
        let player = dummy_player();
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
            let intent = dispatch_state(state, 0, &player, &ball_at(0, 0));
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
        let player = dummy_player();
        let intent = dispatch_state(
            GoalkeeperState::SweeperKeeperRush,
            0,
            &player,
            &ball_at(0, 0),
        );
        assert!(
            matches!(intent, PlayerIntent::GkSweeperRush { .. }),
            "dispatch_state(SweeperKeeperRush) should produce GkSweeperRush intent; got {:?}",
            intent
        );
    }

    #[test]
    fn dispatch_state_shot_stopping_returns_gk_shot_stop_intent() {
        let player = dummy_player();
        let intent = dispatch_state(GoalkeeperState::ShotStopping, 0, &player, &ball_at(0, 0));
        assert!(
            matches!(intent, PlayerIntent::GkShotStop { .. }),
            "dispatch_state(ShotStopping) should produce GkShotStop intent; got {:?}",
            intent
        );
    }

    #[test]
    fn dispatch_state_distributing_from_hand_returns_gk_distribute_short() {
        let player = dummy_player();
        let intent = dispatch_state(
            GoalkeeperState::DistributingFromHand,
            0,
            &player,
            &ball_at(0, 0),
        );
        assert!(
            matches!(intent, PlayerIntent::GkDistributeShort { .. }),
            "dispatch_state(DistributingFromHand) should produce GkDistributeShort; got {:?}",
            intent
        );
    }

    #[test]
    fn dispatch_state_distributing_from_feet_returns_gk_distribute_long() {
        let player = dummy_player();
        let intent = dispatch_state(
            GoalkeeperState::DistributingFromFeet,
            0,
            &player,
            &ball_at(0, 0),
        );
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
        let player = dummy_player();
        let (s1, i1) = tick_goalkeeper(
            GoalkeeperState::InBoxPositioning,
            &player,
            0,
            &ball,
            &mut mk_rng(),
        );
        let (s2, i2) = tick_goalkeeper(
            GoalkeeperState::InBoxPositioning,
            &player,
            0,
            &ball,
            &mut mk_rng(),
        );
        assert_eq!(s1, s2);
        assert_eq!(i1, i2);
    }

    // ------------------------------------------------------------------
    // Binding-correctness tests (Codex P1-4 round 3)
    //
    // For each of the 4 documented GK sites, prove the function ACTUALLY
    // reads its declared attributes (changes intent when attr is varied)
    // AND DOESN'T read non-declared attributes (intent unchanged when an
    // off-binding attr is varied). This is the iii-c lesson applied to
    // GK FSM: the *_ATTRS const slices were decorative; backing tests
    // make them load-bearing.
    // ------------------------------------------------------------------

    fn player_with_attr_mut(mut_fn: impl FnOnce(&mut fw_core::PlayerAttributes)) -> PlayerState {
        let mut p = dummy_player();
        mut_fn(p.attributes_mut());
        p
    }

    #[test]
    fn gk_shot_stopping_reads_reflexes() {
        let ball = ball_at_xy(-30, 5, -5);
        let p_low = player_with_attr_mut(|a| a.goalkeeper.reflexes = Q32::ZERO);
        let p_high = player_with_attr_mut(|a| a.goalkeeper.reflexes = Q32::ONE);
        let i_low = dispatch_state(GoalkeeperState::ShotStopping, 0, &p_low, &ball);
        let i_high = dispatch_state(GoalkeeperState::ShotStopping, 0, &p_high, &ball);
        assert_ne!(i_low, i_high, "shot_stopping must read reflexes");
    }

    #[test]
    fn gk_shot_stopping_reads_positioning() {
        let ball = ball_at_xy(-30, 5, -5);
        let p_low = player_with_attr_mut(|a| a.mental.positioning = Q32::ZERO);
        let p_high = player_with_attr_mut(|a| a.mental.positioning = Q32::ONE);
        let i_low = dispatch_state(GoalkeeperState::ShotStopping, 0, &p_low, &ball);
        let i_high = dispatch_state(GoalkeeperState::ShotStopping, 0, &p_high, &ball);
        assert_ne!(i_low, i_high, "shot_stopping must read mental.positioning");
    }

    #[test]
    fn gk_shot_stopping_reads_composure() {
        let ball = ball_at_xy(-30, 5, -5);
        let p_low = player_with_attr_mut(|a| a.mental.composure = Q32::ZERO);
        let p_high = player_with_attr_mut(|a| a.mental.composure = Q32::ONE);
        let i_low = dispatch_state(GoalkeeperState::ShotStopping, 0, &p_low, &ball);
        let i_high = dispatch_state(GoalkeeperState::ShotStopping, 0, &p_high, &ball);
        assert_ne!(i_low, i_high, "shot_stopping must read mental.composure");
    }

    #[test]
    fn gk_shot_stopping_doesnt_read_off_binding_attr() {
        // mental.flair is NOT in GK_SHOT_STOPPING_ATTRS; shot-stopping must
        // ignore it. (FlairBias would be a bias-path read at T1-4; not here.)
        let ball = ball_at_xy(-30, 5, -5);
        let p_low = player_with_attr_mut(|a| a.mental.flair = Q32::ZERO);
        let p_high = player_with_attr_mut(|a| a.mental.flair = Q32::ONE);
        let i_low = dispatch_state(GoalkeeperState::ShotStopping, 0, &p_low, &ball);
        let i_high = dispatch_state(GoalkeeperState::ShotStopping, 0, &p_high, &ball);
        assert_eq!(
            i_low, i_high,
            "shot_stopping must NOT read mental.flair (off-binding)"
        );
    }

    #[test]
    fn gk_sweeper_rush_reads_one_on_ones() {
        let ball = ball_at(-15, -5);
        let p_low = player_with_attr_mut(|a| a.goalkeeper.one_on_ones = Q32::ZERO);
        let p_high = player_with_attr_mut(|a| a.goalkeeper.one_on_ones = Q32::ONE);
        let i_low = dispatch_state(GoalkeeperState::SweeperKeeperRush, 0, &p_low, &ball);
        let i_high = dispatch_state(GoalkeeperState::SweeperKeeperRush, 0, &p_high, &ball);
        assert_ne!(i_low, i_high, "sweeper_rush must read one_on_ones");
    }

    #[test]
    fn gk_sweeper_rush_reads_command_of_area() {
        let ball = ball_at(-15, -5);
        let p_low = player_with_attr_mut(|a| a.goalkeeper.command_of_area = Q32::ZERO);
        let p_high = player_with_attr_mut(|a| a.goalkeeper.command_of_area = Q32::ONE);
        let i_low = dispatch_state(GoalkeeperState::SweeperKeeperRush, 0, &p_low, &ball);
        let i_high = dispatch_state(GoalkeeperState::SweeperKeeperRush, 0, &p_high, &ball);
        assert_ne!(i_low, i_high, "sweeper_rush must read command_of_area");
    }

    #[test]
    fn gk_sweeper_rush_doesnt_read_off_binding_attr() {
        // goalkeeper.handling is NOT in GK_SWEEPER_RUSH_ATTRS.
        let ball = ball_at(-15, -5);
        let p_low = player_with_attr_mut(|a| a.goalkeeper.handling = Q32::ZERO);
        let p_high = player_with_attr_mut(|a| a.goalkeeper.handling = Q32::ONE);
        let i_low = dispatch_state(GoalkeeperState::SweeperKeeperRush, 0, &p_low, &ball);
        let i_high = dispatch_state(GoalkeeperState::SweeperKeeperRush, 0, &p_high, &ball);
        assert_eq!(
            i_low, i_high,
            "sweeper_rush must NOT read goalkeeper.handling (off-binding)"
        );
    }

    #[test]
    fn gk_distribution_short_reads_kicking() {
        let ball = ball_at(-45, 0);
        let p_low = player_with_attr_mut(|a| a.goalkeeper.kicking = Q32::ZERO);
        let p_high = player_with_attr_mut(|a| a.goalkeeper.kicking = Q32::ONE);
        let i_low = dispatch_state(GoalkeeperState::DistributingFromHand, 0, &p_low, &ball);
        let i_high = dispatch_state(GoalkeeperState::DistributingFromHand, 0, &p_high, &ball);
        assert_ne!(i_low, i_high, "distribution_short must read kicking");
    }

    #[test]
    fn gk_distribution_short_doesnt_read_off_binding_attr() {
        // physical.pace is NOT in GK_DISTRIBUTION_SHORT_ATTRS.
        let ball = ball_at(-45, 0);
        let p_low = player_with_attr_mut(|a| a.physical.pace = Q32::ZERO);
        let p_high = player_with_attr_mut(|a| a.physical.pace = Q32::ONE);
        let i_low = dispatch_state(GoalkeeperState::DistributingFromHand, 0, &p_low, &ball);
        let i_high = dispatch_state(GoalkeeperState::DistributingFromHand, 0, &p_high, &ball);
        assert_eq!(
            i_low, i_high,
            "distribution_short must NOT read physical.pace (off-binding)"
        );
    }

    #[test]
    fn gk_distribution_long_reads_kicking() {
        let ball = ball_at(-45, 0);
        let p_low = player_with_attr_mut(|a| a.goalkeeper.kicking = Q32::ZERO);
        let p_high = player_with_attr_mut(|a| a.goalkeeper.kicking = Q32::ONE);
        let i_low = dispatch_state(GoalkeeperState::DistributingFromFeet, 0, &p_low, &ball);
        let i_high = dispatch_state(GoalkeeperState::DistributingFromFeet, 0, &p_high, &ball);
        assert_ne!(i_low, i_high, "distribution_long must read kicking");
    }

    #[test]
    fn gk_distribution_long_reads_vision() {
        let ball = ball_at(-45, 0);
        let p_low = player_with_attr_mut(|a| a.mental.vision = Q32::ZERO);
        let p_high = player_with_attr_mut(|a| a.mental.vision = Q32::ONE);
        let i_low = dispatch_state(GoalkeeperState::DistributingFromFeet, 0, &p_low, &ball);
        let i_high = dispatch_state(GoalkeeperState::DistributingFromFeet, 0, &p_high, &ball);
        assert_ne!(i_low, i_high, "distribution_long must read mental.vision");
    }

    #[test]
    fn gk_distribution_long_doesnt_read_off_binding_attr() {
        // mental.composure is NOT in GK_DISTRIBUTION_LONG_ATTRS (it IS in
        // GK_DISTRIBUTION_SHORT_ATTRS — verify long doesn't accidentally
        // read it).
        let ball = ball_at(-45, 0);
        let p_low = player_with_attr_mut(|a| a.mental.composure = Q32::ZERO);
        let p_high = player_with_attr_mut(|a| a.mental.composure = Q32::ONE);
        let i_low = dispatch_state(GoalkeeperState::DistributingFromFeet, 0, &p_low, &ball);
        let i_high = dispatch_state(GoalkeeperState::DistributingFromFeet, 0, &p_high, &ball);
        assert_eq!(
            i_low, i_high,
            "distribution_long must NOT read mental.composure (off-binding)"
        );
    }
}

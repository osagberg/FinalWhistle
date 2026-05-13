//! Goalkeeper pure-FSM decision functions.
//!
//! ADR-0006 §"Decision": "Goalkeeper is pure FSM, no inner BT. GK behavior
//! is mode-dominated; ~10-12 states each implemented as a small Rust function."
//!
//! ## T1-2b-iii-a scope (skeleton tier)
//!
//! Every state function returns a stub `PlayerIntent::MoveToPosition` toward
//! the GK's goal-line position (derived from the roster slot — slot 0 is
//! home GK, slot 11 is away GK). Real positioning + shot-stopping + sweeper-
//! keeper logic arrives at -iii-b.
//!
//! `GoalkeeperState::evaluate_transitions` always returns
//! `GoalkeeperState::InBoxPositioning` in this skeleton tier. The FSM
//! transitions are structurally correct but behaviorally trivial until
//! -iii-b wires up the predicate inputs.
//!
//! ## Determinism
//!
//! GK state functions are pure — no floats in canonical state, no RNG draws
//! in the skeleton tier, no clocks. The `ChaCha8Rng` argument is accepted for
//! -iii-b compatibility (shot-stopping prediction will need it).

use rand_chacha::ChaCha8Rng;

use crate::role_states::{GoalkeeperState, PlayerIntent};
use fw_core::Q32;

// ---------------------------------------------------------------------------
// GK tick entry point
// ---------------------------------------------------------------------------

/// Tick the goalkeeper FSM for one decision cycle.
///
/// Returns `(new_state, intent)`. In the skeleton tier, `new_state` is always
/// `InBoxPositioning` regardless of the current state, and `intent` is always
/// `MoveToPosition` toward the goal line.
///
/// `roster_slot` is used to determine which goal line (slot 0 = home GK at
/// x=-45; slot 11 = away GK at x=+45).
///
/// Per ADR-0006 §"Concrete sketch":
/// ```text
/// let next = self.evaluate_transitions(world, player);
/// let intent = match next { ... };
/// (next, intent)
/// ```
#[must_use]
pub fn tick_goalkeeper(
    current_state: GoalkeeperState,
    roster_slot: u8,
    _rng: &mut ChaCha8Rng,
) -> (GoalkeeperState, PlayerIntent) {
    let next_state = evaluate_transitions(current_state);
    let intent = dispatch_state(next_state, roster_slot);
    (next_state, intent)
}

/// Evaluate FSM transitions.
///
/// Skeleton tier: always returns `InBoxPositioning`. -iii-b wires spatial
/// inputs (ball position, shot incoming, etc.) to make this real.
fn evaluate_transitions(_current: GoalkeeperState) -> GoalkeeperState {
    // T1-2b-iii-a: always in-box-positioning. Real transition predicates
    // land when the spatial world view (-iii-b) provides shot-incoming,
    // distribution-needed, set-piece-kind, etc.
    GoalkeeperState::InBoxPositioning
}

/// Dispatch to the per-state intent function.
fn dispatch_state(state: GoalkeeperState, roster_slot: u8) -> PlayerIntent {
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
///
/// `roster_slot` is the 0-indexed formation slot (0 = home GK, 11 = away GK).
/// Panics if the slot is out of range — consistent with P2-1 fix in
/// `formation_position`.
fn gk_goal_line_position(roster_slot: u8) -> (Q32, Q32) {
    // Delegate to formation_position which now asserts rather than clamping.
    use crate::subtree_library::formation_position;
    formation_position(roster_slot)
}

fn gk_in_box_positioning(roster_slot: u8) -> PlayerIntent {
    let (x, y) = gk_goal_line_position(roster_slot);
    PlayerIntent::MoveToPosition {
        target_x: x,
        target_y: y,
    }
}

fn gk_sweeper_rush(roster_slot: u8) -> PlayerIntent {
    // Skeleton: same as in-box. -iii-b: rush toward the ball carrier.
    gk_in_box_positioning(roster_slot)
}

fn gk_shot_stopping(roster_slot: u8) -> PlayerIntent {
    // Skeleton: stay on goal line. -iii-b: dive toward shot trajectory.
    gk_in_box_positioning(roster_slot)
}

fn gk_distributing_from_hand(roster_slot: u8) -> PlayerIntent {
    // Skeleton: stay on goal line. -iii-b: move to distribution position.
    gk_in_box_positioning(roster_slot)
}

fn gk_distributing_from_feet(roster_slot: u8) -> PlayerIntent {
    gk_in_box_positioning(roster_slot)
}

fn gk_penalty_stance(roster_slot: u8) -> PlayerIntent {
    gk_in_box_positioning(roster_slot)
}

fn gk_set_piece_wall(roster_slot: u8) -> PlayerIntent {
    gk_in_box_positioning(roster_slot)
}

fn gk_recovering(roster_slot: u8) -> PlayerIntent {
    gk_in_box_positioning(roster_slot)
}

// ---------------------------------------------------------------------------
// Tests — Chunk 4 (RED → GREEN)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::ChaCha8Rng;
    use rand_chacha::rand_core::SeedableRng;

    fn mk_rng() -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(0)
    }

    // --- Skeleton: always InBoxPositioning ---

    #[test]
    fn gk_tick_always_returns_in_box_positioning_state() {
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
            let (next_state, _intent) = tick_goalkeeper(state, 0, &mut mk_rng());
            assert_eq!(
                next_state,
                GoalkeeperState::InBoxPositioning,
                "skeleton tier must always return InBoxPositioning; got {:?} for state {:?}",
                next_state,
                state
            );
        }
    }

    // --- Goal-line intent ---

    #[test]
    fn home_gk_slot0_returns_goal_line_position() {
        let (_, intent) = tick_goalkeeper(GoalkeeperState::InBoxPositioning, 0, &mut mk_rng());
        match intent {
            PlayerIntent::MoveToPosition { target_x, target_y } => {
                assert_eq!(target_x, Q32::from_int(-45), "home GK should target x=-45");
                assert_eq!(target_y, Q32::from_int(0), "home GK should target y=0");
            }
            PlayerIntent::Idle => panic!("GK returned Idle; expected MoveToPosition"),
        }
    }

    #[test]
    fn away_gk_slot11_returns_goal_line_position() {
        let (_, intent) = tick_goalkeeper(GoalkeeperState::InBoxPositioning, 11, &mut mk_rng());
        match intent {
            PlayerIntent::MoveToPosition { target_x, target_y } => {
                assert_eq!(target_x, Q32::from_int(45), "away GK should target x=+45");
                assert_eq!(target_y, Q32::from_int(0), "away GK should target y=0");
            }
            PlayerIntent::Idle => panic!("GK returned Idle; expected MoveToPosition"),
        }
    }

    // --- All state variants produce MoveToPosition (not Idle) ---

    #[test]
    fn every_gk_state_produces_move_to_position_intent() {
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
            let (_, intent) = tick_goalkeeper(state, 0, &mut mk_rng());
            assert!(
                matches!(intent, PlayerIntent::MoveToPosition { .. }),
                "GK state {:?} produced Idle; expected MoveToPosition",
                state
            );
        }
    }

    // --- Determinism: same inputs → same outputs ---

    #[test]
    fn gk_tick_is_deterministic() {
        let state = GoalkeeperState::ShotStopping;
        let (s1, i1) = tick_goalkeeper(state, 0, &mut mk_rng());
        let (s2, i2) = tick_goalkeeper(state, 0, &mut mk_rng());
        assert_eq!(s1, s2);
        assert_eq!(i1, i2);
    }
}

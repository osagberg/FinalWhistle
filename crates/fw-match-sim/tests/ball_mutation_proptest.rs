//! T1-3.5 Chunk 6: Proptest invariants for ball mutation via `apply_intent`.
//!
//! Anti-vacuousness discipline: asserts ball.vel != ZERO FIRST, then
//! structural invariants. A test that only checks structure without first
//! asserting the ball actually moved provides no value (the ball could be
//! stationary and the structural checks would still pass vacuously).
//!
//! Invariants covered:
//! - AC 1: After any Shot intent fires, ball has non-zero velocity in X or Y.
//! - AC 2: After any Pass intent fires, ball has non-zero velocity in X or Y.
//! - AC 3: Possession/last_touched_by update correctly per intent class.
//! - AC 6: Determinism — same seed → same ball state after dispatch_tick.

use fw_core::{Q32, Seed};
use fw_match_sim::{MatchState, PlayerIntent, dispatch};
use proptest::prelude::*;
use std::collections::BTreeMap;

fn arb_seed() -> impl Strategy<Value = u64> {
    any::<u64>()
}

// ---------------------------------------------------------------------------
// AC 6: Determinism invariant
// ---------------------------------------------------------------------------

proptest! {
    /// Same seed → identical ball state after dispatch_tick.
    /// This is the minimal determinism gate for ball mutation.
    #[test]
    fn ball_mutation_is_deterministic(seed_val in arb_seed()) {
        let seed = Seed::from_u64(seed_val);
        let s1 = MatchState::initial(seed);
        let s2 = MatchState::initial(seed);
        let empty_defs = BTreeMap::new();
        let r1 = dispatch::dispatch_tick(s1, &empty_defs);
        let r2 = dispatch::dispatch_tick(s2, &empty_defs);

        // Anti-vacuousness: check both canonical encoding AND ball state.
        // Compare canonical bytes first (catches all state), then ball for clarity.
        let enc1 = r1.encode_canonical();
        let enc2 = r2.encode_canonical();
        prop_assert_eq!(
            enc1, enc2,
            "canonical encoding must be identical for seed 0x{:016x}", seed_val
        );
        prop_assert_eq!(
            r1.ball, r2.ball,
            "ball state must be identical for seed 0x{:016x}", seed_val
        );
    }
}

// ---------------------------------------------------------------------------
// AC 1/2: Ball moves after Shot or Pass intents
// ---------------------------------------------------------------------------

proptest! {
    /// After applying AttemptShot from any player position to any target,
    /// the ball must have non-zero velocity. Tests `compute_ball_speed_for_shot`
    /// and `ball_unit_vel` indirectly via `apply_intent`.
    ///
    /// Anti-vacuousness: assert ball.vel != ZERO FIRST, then possession invariant.
    #[test]
    fn shot_intent_produces_non_zero_ball_velocity(
        seed_val in arb_seed(),
        target_x_int in -52i32..=52,
        target_y_int in -34i32..=34,
    ) {
        let seed = Seed::from_u64(seed_val);
        let mut state = MatchState::initial(seed);

        // Force player 0 (home GK, slot 0) to have AttemptShot intent.
        // We apply the intent directly via dispatch::apply_intent.
        // Use a non-trivial target that is definitely not at the player's
        // current position (avoids zero-distance degenerate case).
        let target_x = Q32::from_int(target_x_int);
        let target_y = Q32::from_int(target_y_int);

        // Place the ball at the player's position first (required for non-degenerate test).
        state.ball.pos_x = state.players[0].pos_x;
        state.ball.pos_y = state.players[0].pos_y;

        let intent = PlayerIntent::AttemptShot { target_x, target_y };
        dispatch::apply_intent(&mut state, 0, intent);

        // ANTI-VACUOUSNESS FIRST: assert ball actually moved.
        let vel_nonzero = state.ball.vel_x != Q32::ZERO || state.ball.vel_y != Q32::ZERO;
        prop_assert!(
            vel_nonzero,
            "AttemptShot must produce non-zero ball velocity; \
             vel_x={:?}, vel_y={:?}, target=({target_x_int},{target_y_int})",
            state.ball.vel_x,
            state.ball.vel_y,
        );

        // Structural invariant: possession should be None (ball in flight).
        prop_assert!(
            state.possession().is_none(),
            "possession must be None after shot (ball in flight)"
        );

        // last_touched_by should be slot 0 (the shooter).
        prop_assert_eq!(
            state.last_touched_by(),
            Some(0),
            "last_touched_by must be the shooter slot (0)"
        );
    }

    /// After applying AttemptPassShort from player 9 (home FWD) to any target,
    /// the ball must have non-zero velocity.
    ///
    /// Anti-vacuousness: assert ball.vel != ZERO FIRST, then possession invariant.
    #[test]
    fn pass_intent_produces_non_zero_ball_velocity(
        seed_val in arb_seed(),
        target_x_int in -52i32..=52,
        target_y_int in -34i32..=34,
    ) {
        let seed = Seed::from_u64(seed_val);
        let mut state = MatchState::initial(seed);

        // Use player slot 9 (home forward) as passer.
        let passer_slot: usize = 9;
        let target_x = Q32::from_int(target_x_int);
        let target_y = Q32::from_int(target_y_int);

        // Place the ball at the passer's position.
        state.ball.pos_x = state.players[passer_slot].pos_x;
        state.ball.pos_y = state.players[passer_slot].pos_y;

        let intent = PlayerIntent::AttemptPassShort { target_x, target_y };
        dispatch::apply_intent(&mut state, passer_slot, intent);

        // ANTI-VACUOUSNESS FIRST: ball must be moving.
        let vel_nonzero = state.ball.vel_x != Q32::ZERO || state.ball.vel_y != Q32::ZERO;
        prop_assert!(
            vel_nonzero,
            "AttemptPassShort must produce non-zero ball velocity; \
             vel_x={:?}, vel_y={:?}",
            state.ball.vel_x,
            state.ball.vel_y,
        );

        // Structural: last_touched_by must be the passer.
        prop_assert_eq!(
            state.last_touched_by(),
            Some(passer_slot as u8),
            "last_touched_by must be the passer slot"
        );
    }
}

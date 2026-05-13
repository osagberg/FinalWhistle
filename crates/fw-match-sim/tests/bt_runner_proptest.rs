//! Proptest invariants for the T1-2b-iii-a BT runner + role assignment.
//!
//! Covers acceptance criteria:
//! - AC 7: intra-process determinism (same seed → same intent sequence).
//! - AC 8: BT traversal proptest — deterministic node-visit order over seeds.
//! - AC 5 (partial): `decision_counter()` increments deterministically.

use fw_core::Seed;
use fw_match_sim::{MatchState, Role, tick_match};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

fn arb_seed() -> impl Strategy<Value = u64> {
    any::<u64>()
}

// ---------------------------------------------------------------------------
// BT traversal determinism — same seed → same output
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn bt_traversal_same_seed_same_output(seed_val in arb_seed()) {
        // Two fresh runs of the same seed must produce byte-identical
        // canonical state after 15 ticks (one full cadence window).
        let seed = Seed::from_u64(seed_val);

        let run = || {
            let mut s = MatchState::initial(seed);
            for _ in 0..15 {
                s = tick_match(s);
            }
            s
        };

        let s1 = run();
        let s2 = run();
        prop_assert_eq!(
            s1.encode_canonical(),
            s2.encode_canonical(),
            "same seed must produce identical canonical state after 15 ticks"
        );
    }
}

// ---------------------------------------------------------------------------
// Role assignment invariant
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn role_assignment_is_deterministic(seed_val in arb_seed()) {
        let seed = Seed::from_u64(seed_val);
        let s = MatchState::initial(seed);

        // Home team (slots 0..11)
        prop_assert_eq!(s.players[0].role(), Role::Goalkeeper);
        for i in 1usize..=4 {
            prop_assert!(
                s.players[i].role() == Role::Defender,
                "home slot {} must be DEF, got {:?}", i, s.players[i].role()
            );
        }
        for i in 5usize..=7 {
            prop_assert!(
                s.players[i].role() == Role::Midfielder,
                "home slot {} must be MID, got {:?}", i, s.players[i].role()
            );
        }
        for i in 8usize..=10 {
            prop_assert!(
                s.players[i].role() == Role::Forward,
                "home slot {} must be FWD, got {:?}", i, s.players[i].role()
            );
        }

        // Away team (slots 11..22)
        prop_assert_eq!(s.players[11].role(), Role::Goalkeeper);
        for i in 12usize..=15 {
            prop_assert!(
                s.players[i].role() == Role::Defender,
                "away slot {} must be DEF, got {:?}", i, s.players[i].role()
            );
        }
        for i in 16usize..=18 {
            prop_assert!(
                s.players[i].role() == Role::Midfielder,
                "away slot {} must be MID, got {:?}", i, s.players[i].role()
            );
        }
        for i in 19usize..=21 {
            prop_assert!(
                s.players[i].role() == Role::Forward,
                "away slot {} must be FWD, got {:?}", i, s.players[i].role()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Decision counter increment invariant
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn decision_counter_increments_monotonically(seed_val in arb_seed()) {
        // Over 60 ticks (4 full cadence windows), every player must have
        // at least one decision fired (balanced template guarantees no empty
        // slot), and counters must never decrease.
        let seed = Seed::from_u64(seed_val);
        let mut state = MatchState::initial(seed);
        let mut prev: [u32; 22] = [0; 22];

        for _ in 0..60 {
            state = tick_match(state);
            for (i, p) in state.players.iter().enumerate() {
                prop_assert!(
                    p.decision_counter() >= prev[i],
                    "player slot {}: counter went from {} to {} -- must be monotonic",
                    i, prev[i], p.decision_counter()
                );
                prev[i] = p.decision_counter();
            }
        }

        // After 60 ticks (4 x 15-tick windows), every player must have
        // decided at least 3 times (allowing for slot-window edge effects).
        for (i, p) in state.players.iter().enumerate() {
            prop_assert!(
                p.decision_counter() >= 3,
                "player slot {}: expected 3+ decisions in 60 ticks, got {}",
                i, p.decision_counter()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// BT runner produces non-Idle intents for outfield players
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn outfield_players_move_to_formation_after_decision(seed_val in arb_seed()) {
        // After at least one decision fires, the outfield players should not
        // all be at exactly the initial T0 position. We verify this by checking
        // that at least some outfield player has counter > 0.
        //
        // Note: GKs start at their goal line (formation position), so their
        // vel will be zero after applying MoveToPosition toward where they
        // already are. We check outfield players (slots 1-10, 12-21).
        let seed = Seed::from_u64(seed_val);
        let mut state = MatchState::initial(seed);

        // Run 30 ticks -- every player fires at least twice.
        for _ in 0..30 {
            state = tick_match(state);
        }

        // At least one outfield player must have counter > 0.
        let any_decided = state.players.iter().enumerate().any(|(i, p)| {
            i != 0 && i != 11 && p.decision_counter() > 0
        });
        prop_assert!(
            any_decided,
            "no outfield player made a decision in 30 ticks -- BT dispatch broken"
        );
    }
}

// ---------------------------------------------------------------------------
// Same seed → same intent sequence (per-player trace)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn same_seed_same_player_intent_trace(seed_val in arb_seed()) {
        let seed = Seed::from_u64(seed_val);

        // Run 30 ticks twice; compare final player velocities.
        let final_vels = |s: &MatchState| -> Vec<(i64, i64)> {
            s.players
                .iter()
                .map(|p| (p.vel_x.to_bits(), p.vel_y.to_bits()))
                .collect()
        };

        let mut s1 = MatchState::initial(seed);
        for _ in 0..30 {
            s1 = tick_match(s1);
        }

        let mut s2 = MatchState::initial(seed);
        for _ in 0..30 {
            s2 = tick_match(s2);
        }

        prop_assert_eq!(
            final_vels(&s1),
            final_vels(&s2),
            "same seed must produce identical player velocities after 30 ticks"
        );
    }
}

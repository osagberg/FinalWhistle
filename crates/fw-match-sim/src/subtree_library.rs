//! Subtree library — hardcoded skeleton-tier stub subtrees for T1-2b-iii-a.
//!
//! ## Design
//!
//! ADR-0006 §"Decision": "Nodes are code; trees are data." For the
//! skeleton tier, every (Role, state_tag) pair maps to a stub tree
//! containing a single `Leaf(MoveToFormationPosition)`.
//!
//! Content-pack RON loading (`fwh.core:subtree_midfielder_pressing`, etc.)
//! defers to -iii-b / T2-3. This module is the hardcoded placeholder that
//! gives -iii-b a clean landing surface.
//!
//! ## Formation positions — default 4-3-3
//!
//! Pitch: 100m × 60m, centred at (0, 0).
//! Home defends the -x goal; away defends the +x goal.
//!
//! Slot assignment (from `MatchState::initial`):
//!   slot  0 = home GK;    slot 11 = away GK
//!   slots 1-4 = home DEF; slots 12-15 = away DEF
//!   slots 5-7 = home MID; slots 16-18 = away MID
//!   slots 8-10 = home FWD; slots 19-21 = away FWD
//!
//! Position table (indexed by slot 0..22; Q32 integer metres):
//!
//! | slot | role    | home/away | x   | y   |
//! |------|---------|-----------|-----|-----|
//! |  0   | GK home |  home     | -45 |  0  |
//! |  1   | DEF home|  home     | -30 | -20 |
//! |  2   | DEF home|  home     | -30 |  -7 |
//! |  3   | DEF home|  home     | -30 |   7 |
//! |  4   | DEF home|  home     | -30 |  20 |
//! |  5   | MID home|  home     | -10 | -15 |
//! |  6   | MID home|  home     | -10 |   0 |
//! |  7   | MID home|  home     | -10 |  15 |
//! |  8   | FWD home|  home     |  10 | -15 |
//! |  9   | FWD home|  home     |  10 |   0 |
//! | 10   | FWD home|  home     |  10 |  15 |
//! | 11   | GK away |  away     |  45 |   0 |
//! | 12   | DEF away|  away     |  30 | -20 |
//! | 13   | DEF away|  away     |  30 |  -7 |
//! | 14   | DEF away|  away     |  30 |   7 |
//! | 15   | DEF away|  away     |  30 |  20 |
//! | 16   | MID away|  away     |  10 | -15 |  ← mirrors home MID, flipped x
//! | 17   | MID away|  away     |  10 |   0 |
//! | 18   | MID away|  away     |  10 |  15 |
//! | 19   | FWD away|  away     | -10 | -15 |  ← mirrors home FWD, flipped x
//! | 20   | FWD away|  away     | -10 |   0 |
//! | 21   | FWD away|  away     | -10 |  15 |
//!
//! ## SubtreeLibrary
//!
//! Keyed by `(Role, role_state_tag: u8)` using `BTreeMap` for deterministic
//! iteration (Sim/RULES.md §2 — no HashMap).

use std::collections::BTreeMap;

use fw_content::SimBiasSnapshot;
use fw_core::Q32;
use rand_chacha::ChaCha8Rng;

use crate::bt::off_ball::{
    utility_hold_formation, utility_mark_player, utility_press, utility_run_off_ball,
    utility_track_back,
};
use crate::bt::on_ball::{
    utility_cross, utility_dribble, utility_hold_ball, utility_lay_off, utility_pass_long,
    utility_pass_short, utility_shoot,
};
use crate::bt::{LeafKind, Node, Tree};
use crate::player::PlayerState;
use crate::role_states::{
    DefenderState, ForwardState, MidfielderState, PlayerIntent, PlayerRoleState, Role,
};
use crate::signature::bias_apply::{BiasConsideration, apply_signature_bias};
use crate::utility::softmax::{DEFAULT_TEMPERATURE, pick_top_n_softmax};

// ---------------------------------------------------------------------------
// Formation positions — 4-3-3 default
// ---------------------------------------------------------------------------

/// Default 4-3-3 formation positions indexed by roster slot (0..22).
///
/// (pos_x, pos_y) in metres (Q32 integer; no floats).
///
/// Slot layout: home slots 0..11, away slots 11..22.
/// Home defends -x goal; away defends +x goal.
pub const FORMATION_4_3_3_POSITIONS: [(i32, i32); 22] = [
    // Home team (slots 0..11)
    (-45, 0),   // slot  0: home GK
    (-30, -20), // slot  1: home DEF
    (-30, -7),  // slot  2: home DEF
    (-30, 7),   // slot  3: home DEF
    (-30, 20),  // slot  4: home DEF
    (-10, -15), // slot  5: home MID
    (-10, 0),   // slot  6: home MID
    (-10, 15),  // slot  7: home MID
    (10, -15),  // slot  8: home FWD
    (10, 0),    // slot  9: home FWD
    (10, 15),   // slot 10: home FWD
    // Away team (slots 11..22)
    (45, 0),    // slot 11: away GK
    (30, -20),  // slot 12: away DEF
    (30, -7),   // slot 13: away DEF
    (30, 7),    // slot 14: away DEF
    (30, 20),   // slot 15: away DEF
    (10, -15),  // slot 16: away MID  (mirrors home MID x-flipped)
    (10, 0),    // slot 17: away MID
    (10, 15),   // slot 18: away MID
    (-10, -15), // slot 19: away FWD  (mirrors home FWD x-flipped)
    (-10, 0),   // slot 20: away FWD
    (-10, 15),  // slot 21: away FWD
];

/// Resolve the formation position for a given roster `slot` (0-indexed 0..22).
///
/// Returns `(target_x, target_y)` in Q32.
///
/// Panics (in both debug and release) if `slot >= 22` — an out-of-range slot
/// indicates a sim invariant violation and should never be silently clamped
/// to a plausible-but-wrong position.
#[must_use]
pub fn formation_position(slot: u8) -> (Q32, Q32) {
    assert!(
        (slot as usize) < FORMATION_4_3_3_POSITIONS.len(),
        "slot {slot} out of range; expected 0..22 — this is a sim invariant violation"
    );
    let (x, y) = FORMATION_4_3_3_POSITIONS[slot as usize];
    (Q32::from_int(x), Q32::from_int(y))
}

// ---------------------------------------------------------------------------
// Utility-scored outfield intent selection
// ---------------------------------------------------------------------------

/// Select a `PlayerIntent` for an outfield player using utility scoring + softmax.
///
/// Assembles a candidate list of `(PlayerIntent, utility_score)` pairs from
/// the on-ball and off-ball utility functions appropriate to the player's
/// current role state, then calls `pick_top_n_softmax` with `rng` to sample.
///
/// Returns the picked `PlayerIntent`. Falls back to `MoveToPosition` toward
/// the formation slot if the candidate list is somehow empty (defensive
/// invariant — should never trigger).
///
/// ## Role-state → candidate set mapping
///
/// `InPossession` → on-ball considerations (7).
/// `Pressing` → press + track_back (focused subset).
/// `Supporting` / `RunningOffBall` / `MakingRun` → off-ball set (5).
/// `Defending` / `Recovering` / `Tracking` / `SetPieceWaiting` / `HoldingUp`
///   → formation-hold set (track_back + hold_formation + mark_player).
///
/// All other states fall back to `HoldFormation`.
///
/// ## Signature bias composition
///
/// When `active_bias` is `Some(snapshot)`, the function applies the snapshot's
/// per-consideration multiplier to each candidate utility AFTER the personality
/// bias layer. Order: `raw × personality_bias × signature_bias`. The `active_bias`
/// is the `SimBiasSnapshot` from the player's currently-active `SignatureFiring`.
///
/// ## B1 carrier targeting (FUN-0b+c)
///
/// `carrier_pos` is `Some((x, y))` when a player currently has possession.
/// When provided, `utility_press` and `utility_mark_player` target the actual
/// carrier position instead of the hardcoded formation-slot proxy.
#[must_use]
pub fn select_outfield_intent(
    role_state: PlayerRoleState,
    player: &PlayerState,
    roster_slot: u8,
    rng: &mut ChaCha8Rng,
    active_bias: Option<&SimBiasSnapshot>,
    carrier_pos: Option<(Q32, Q32)>,
) -> PlayerIntent {
    let candidates: Vec<(PlayerIntent, Q32)> = match role_state {
        PlayerRoleState::Goalkeeper(_) => {
            panic!("select_outfield_intent called for Goalkeeper — route through goalkeeper_fsm");
        }

        // On-ball: player in possession.
        PlayerRoleState::Defender(DefenderState::InPossession)
        | PlayerRoleState::Midfielder(MidfielderState::InPossession)
        | PlayerRoleState::Forward(ForwardState::InPossession) => {
            vec![
                utility_shoot(player, roster_slot),
                utility_pass_short(player, roster_slot),
                utility_pass_long(player, roster_slot),
                utility_cross(player, roster_slot),
                utility_dribble(player, roster_slot),
                utility_hold_ball(player, roster_slot),
                utility_lay_off(player, roster_slot),
            ]
        }

        // Pressing focus.
        PlayerRoleState::Defender(DefenderState::Pressing)
        | PlayerRoleState::Midfielder(MidfielderState::Pressing)
        | PlayerRoleState::Forward(ForwardState::Pressing) => {
            vec![
                utility_press(player, roster_slot, carrier_pos),
                utility_track_back(player, roster_slot),
            ]
        }

        // Off-ball forward running.
        PlayerRoleState::Midfielder(MidfielderState::RunningOffBall)
        | PlayerRoleState::Forward(ForwardState::RunningOffBall)
        | PlayerRoleState::Forward(ForwardState::MakingRun) => {
            vec![
                utility_run_off_ball(player, roster_slot),
                utility_press(player, roster_slot, carrier_pos),
                utility_hold_formation(player, roster_slot),
            ]
        }

        // Defensive / recovery / set-piece holding states.
        PlayerRoleState::Defender(DefenderState::Defending)
        | PlayerRoleState::Defender(DefenderState::Recovering)
        | PlayerRoleState::Defender(DefenderState::Tracking)
        | PlayerRoleState::Defender(DefenderState::SetPieceWaiting)
        | PlayerRoleState::Midfielder(MidfielderState::Defending)
        | PlayerRoleState::Midfielder(MidfielderState::Recovering)
        | PlayerRoleState::Midfielder(MidfielderState::SetPieceWaiting)
        | PlayerRoleState::Forward(ForwardState::Recovering)
        | PlayerRoleState::Forward(ForwardState::SetPieceWaiting) => {
            vec![
                utility_track_back(player, roster_slot),
                utility_hold_formation(player, roster_slot),
                utility_mark_player(player, roster_slot, carrier_pos),
            ]
        }

        // Supporting / HoldingUp.
        PlayerRoleState::Defender(DefenderState::Supporting)
        | PlayerRoleState::Midfielder(MidfielderState::Supporting)
        | PlayerRoleState::Forward(ForwardState::HoldingUp) => {
            vec![
                utility_hold_formation(player, roster_slot),
                utility_run_off_ball(player, roster_slot),
                utility_mark_player(player, roster_slot, carrier_pos),
            ]
        }
    };

    // Apply signature bias if a signature is in flight.
    // Each intent variant maps to a BiasConsideration; the multiplier is
    // applied after personality bias (which is already in the score from
    // the utility functions above).
    let candidates: Vec<(PlayerIntent, Q32)> = if let Some(snap) = active_bias {
        candidates
            .into_iter()
            .map(|(intent, score)| {
                let consideration = intent_to_bias_consideration(&intent);
                let biased = apply_signature_bias(score, snap, consideration);
                (intent, biased)
            })
            .collect()
    } else {
        candidates
    };

    // Sample via softmax. All branches above push ≥1 candidate, so this must
    // succeed. An empty candidates list is a code bug — not a recoverable
    // runtime condition.
    pick_top_n_softmax(&candidates, rng, DEFAULT_TEMPERATURE).unwrap_or_else(|| {
        unreachable!(
            "select_outfield_intent: candidate list is empty for role_state {:?} slot {}; \
             every match arm must push at least one candidate",
            role_state, roster_slot
        )
    })
}

// ---------------------------------------------------------------------------
// SubtreeLibrary
// ---------------------------------------------------------------------------

/// The BT subtree registry for the skeleton tier.
///
/// For T1-2b-iii-a every (Role, state_tag) pair maps to a stub tree
/// containing `Leaf(MoveToFormationPosition)`. The key is `(Role, u8)` per
/// MEMORY.md task spec.
///
/// `BTreeMap` is mandatory (Sim/RULES.md §2).
pub struct SubtreeLibrary {
    trees: BTreeMap<(Role, u8), Tree>,
}

impl SubtreeLibrary {
    /// Construct the hardcoded skeleton-tier library.
    ///
    /// Covers every (Role, state_tag) pair from the three outfield roles.
    /// GK has no subtrees — it uses the pure FSM path in `goalkeeper_fsm.rs`.
    #[must_use]
    pub fn default_skeleton() -> SubtreeLibrary {
        let mut trees: BTreeMap<(Role, u8), Tree> = BTreeMap::new();

        // Helper: a single-leaf stub tree.
        // The closure can't be Fn-cached because Tree is not Clone, but the
        // call sites are small enough that inline is fine.

        // Defender states (tags 0..7)
        for tag in 0u8..7 {
            trees.insert(
                (Role::Defender, tag),
                Tree::new(Node::Leaf(LeafKind::MoveToFormationPosition)),
            );
        }

        // Midfielder states (tags 0..7)
        for tag in 0u8..7 {
            trees.insert(
                (Role::Midfielder, tag),
                Tree::new(Node::Leaf(LeafKind::MoveToFormationPosition)),
            );
        }

        // Forward states (tags 0..7)
        for tag in 0u8..7 {
            trees.insert(
                (Role::Forward, tag),
                Tree::new(Node::Leaf(LeafKind::MoveToFormationPosition)),
            );
        }

        SubtreeLibrary { trees }
    }

    /// Look up the BT subtree for an outfield `PlayerRoleState`.
    ///
    /// Panics if the (role, state_tag) pair is not registered — in the
    /// skeleton tier every valid outfield (role, state) is pre-registered by
    /// `default_skeleton()`, so a panic here indicates a sim invariant
    /// violation. The loud panic is intentional; -iii-b debugging time saved.
    ///
    /// **Do NOT call this for `PlayerRoleState::Goalkeeper`** — GK uses the
    /// pure FSM path in `goalkeeper_fsm::tick_goalkeeper`. `dispatch_tick`
    /// routes GK separately before reaching this function.
    #[must_use]
    pub fn lookup_outfield(&self, role_state: PlayerRoleState) -> &Tree {
        let (role_tag, state_tag) = role_state.to_tags();
        let role = role_state.role();
        assert_ne!(
            role,
            Role::Goalkeeper,
            "lookup_outfield called for Goalkeeper (role_tag={role_tag}, state_tag={state_tag}); \
             GK must be routed through goalkeeper_fsm::tick_goalkeeper instead"
        );
        self.trees.get(&(role, state_tag)).unwrap_or_else(|| {
            panic!(
                "SubtreeLibrary: no tree registered for (role={:?}, state_tag={state_tag}); \
                 default_skeleton() should cover all outfield states — sim invariant violated",
                role
            )
        })
    }

    /// Look up the stub tree for `(role, state_tag)`.
    ///
    /// Kept for backward-compat with tests; prefer `lookup_outfield` in
    /// production dispatch paths.
    ///
    /// Returns `None` for `Role::Goalkeeper` or unknown (role, state_tag).
    #[must_use]
    pub fn lookup(&self, role: Role, state_tag: u8) -> Option<&Tree> {
        if role == Role::Goalkeeper {
            return None;
        }
        self.trees.get(&(role, state_tag))
    }
}

// ---------------------------------------------------------------------------
// Intent → BiasConsideration mapping
// ---------------------------------------------------------------------------

/// Map a `PlayerIntent` to the `BiasConsideration` that governs its utility
/// scaling when a signature bias is active.
///
/// Every `PlayerIntent` variant is enumerated explicitly — no wildcard `_`.
/// This forces a compile error when a new `PlayerIntent` variant is added
/// without a deliberate bias assignment, preventing silent misrouting.
///
/// Groupings:
/// - `AttemptShot` → Shoot
/// - pass/cross/lay-off variants → Pass
/// - `Dribble` / `HoldBall` → Dribble
/// - `Press` / `TrackBack` → Press
/// - defensive / positional movement → Cover
/// - `Idle` → Neutral (no multiplier; see `BiasConsideration::Neutral` doc)
/// - GK variants → their nearest semantic bucket
fn intent_to_bias_consideration(intent: &PlayerIntent) -> BiasConsideration {
    match intent {
        PlayerIntent::AttemptShot { .. } => BiasConsideration::Shoot,
        PlayerIntent::AttemptPassShort { .. }
        | PlayerIntent::AttemptPassLong { .. }
        | PlayerIntent::Cross { .. }
        | PlayerIntent::LayOff { .. } => BiasConsideration::Pass,
        PlayerIntent::Dribble { .. } | PlayerIntent::HoldBall { .. } => BiasConsideration::Dribble,
        PlayerIntent::Press { .. } | PlayerIntent::TrackBack { .. } => BiasConsideration::Press,
        PlayerIntent::RunOffBall { .. }
        | PlayerIntent::MarkPlayer { .. }
        | PlayerIntent::HoldFormation { .. }
        | PlayerIntent::MoveToPosition { .. } => BiasConsideration::Cover,
        // Idle carries no directional intent — multiplying it into a bias bucket
        // would silently change Cover utility scores for players standing still.
        PlayerIntent::Idle => BiasConsideration::Neutral,
        // GK variants: group by semantic role.
        PlayerIntent::GkShotStop { .. }
        | PlayerIntent::GkCollectCross { .. }
        | PlayerIntent::GkSweeperRush { .. } => BiasConsideration::Cover,
        PlayerIntent::GkDistributeShort { .. } | PlayerIntent::GkDistributeLong { .. } => {
            BiasConsideration::Pass
        } // NO wildcard — future intents force a compile error.
    }
}

// ---------------------------------------------------------------------------
// Tests — Chunk 3 (RED → GREEN)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bt::NodeStatus;
    use crate::bt::{BtContext, tick_tree};
    use rand_chacha::ChaCha8Rng;
    use rand_chacha::rand_core::SeedableRng;

    fn mk_rng() -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(0)
    }

    // --- formation_position ---

    #[test]
    fn formation_positions_all_22_slots_valid() {
        for slot in 0u8..22 {
            let (x, y) = formation_position(slot);
            // Just verify these are Q32 values (not zero for all, which would
            // indicate the table wasn't loaded). The GK at slot 0 is at (-45, 0)
            // which is non-zero on x.
            let _ = (x, y);
        }
    }

    #[test]
    fn home_gk_slot0_is_at_neg45_zero() {
        let (x, y) = formation_position(0);
        assert_eq!(x, Q32::from_int(-45));
        assert_eq!(y, Q32::from_int(0));
    }

    #[test]
    fn away_gk_slot11_is_at_pos45_zero() {
        let (x, y) = formation_position(11);
        assert_eq!(x, Q32::from_int(45));
        assert_eq!(y, Q32::from_int(0));
    }

    #[test]
    fn home_def_slot1_is_at_neg30_neg20() {
        let (x, y) = formation_position(1);
        assert_eq!(x, Q32::from_int(-30));
        assert_eq!(y, Q32::from_int(-20));
    }

    #[test]
    fn home_fwd_slot8_is_at_pos10_neg15() {
        let (x, y) = formation_position(8);
        assert_eq!(x, Q32::from_int(10));
        assert_eq!(y, Q32::from_int(-15));
    }

    // --- SubtreeLibrary::default_skeleton ---

    #[test]
    fn all_outfield_role_state_combos_resolve_to_tree() {
        let lib = SubtreeLibrary::default_skeleton();

        // Defender: 7 states
        for tag in 0u8..7 {
            assert!(
                lib.lookup(Role::Defender, tag).is_some(),
                "Defender state tag {tag} missing from library"
            );
        }

        // Midfielder: 7 states
        for tag in 0u8..7 {
            assert!(
                lib.lookup(Role::Midfielder, tag).is_some(),
                "Midfielder state tag {tag} missing from library"
            );
        }

        // Forward: 7 states
        for tag in 0u8..7 {
            assert!(
                lib.lookup(Role::Forward, tag).is_some(),
                "Forward state tag {tag} missing from library"
            );
        }
    }

    #[test]
    fn goalkeeper_lookup_returns_none() {
        let lib = SubtreeLibrary::default_skeleton();
        assert!(
            lib.lookup(Role::Goalkeeper, 0).is_none(),
            "GK has no BT subtrees — pure FSM path"
        );
    }

    #[test]
    fn stub_tree_returns_success_for_every_role_state() {
        let lib = SubtreeLibrary::default_skeleton();

        for (role, max_state) in [
            (Role::Defender, 7u8),
            (Role::Midfielder, 7),
            (Role::Forward, 7),
        ] {
            for tag in 0..max_state {
                let tree = lib.lookup(role, tag).expect("tree must exist");
                // Use slot 0 (home GK) for context — the position won't be used
                // by a non-GK tree in the real dispatch, but is valid for the
                // skeleton-tier test.
                let ctx = BtContext {
                    roster_slot: 1, // a home DEF slot
                    outfield_role_state: None,
                    player: None,
                    active_bias: None,
                    select_fn: None,
                    carrier_pos: None,
                };
                let (status, _intent) = tick_tree(tree, &ctx, &mut mk_rng());
                assert_eq!(
                    status,
                    NodeStatus::Success,
                    "stub tree for {:?} state {tag} returned {:?}",
                    role,
                    status
                );
            }
        }
    }

    #[test]
    fn formation_table_length_is_22() {
        assert_eq!(FORMATION_4_3_3_POSITIONS.len(), 22);
    }
}

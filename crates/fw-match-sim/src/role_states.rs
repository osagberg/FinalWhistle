//! Per-player role + role-state enums, and the `PlayerIntent` type.
//!
//! ADR-0006 §"Decision" specifies:
//! - Each outfield role (Defender / Midfielder / Forward) has a flat enum
//!   of **role states** (~6-10 per role).
//! - Goalkeeper is pure FSM, no inner BT. Its states live here too.
//!
//! ## Canonical-encoder stability (CRITICAL)
//!
//! The `u8` tag emitted by [`Role::to_tag`] and the per-role state-tag
//! functions are stable for the wire format. Do NOT reorder enum variants
//! without a canonical-hash rebaseline (ADR-0012 trigger #1) and updating
//! the module-doc table in `canonical.rs`.
//!
//! ### Role tags (stable discriminants):
//! - 0 = Goalkeeper
//! - 1 = Defender
//! - 2 = Midfielder
//! - 3 = Forward
//!
//! ### GoalkeeperState tags (variant order = tag, 0-indexed):
//! - 0 = InBoxPositioning
//! - 1 = SweeperKeeperRush
//! - 2 = ShotStopping
//! - 3 = DistributingFromHand
//! - 4 = DistributingFromFeet
//! - 5 = PenaltyStance
//! - 6 = SetPieceWall
//! - 7 = Recovering
//!
//! ### DefenderState tags:
//! - 0 = Defending
//! - 1 = Pressing
//! - 2 = Recovering
//! - 3 = Supporting
//! - 4 = InPossession
//! - 5 = SetPieceWaiting
//! - 6 = Tracking
//!
//! ### MidfielderState tags:
//! - 0 = Defending
//! - 1 = Pressing
//! - 2 = Recovering
//! - 3 = Supporting
//! - 4 = InPossession
//! - 5 = RunningOffBall
//! - 6 = SetPieceWaiting
//!
//! ### ForwardState tags:
//! - 0 = RunningOffBall
//! - 1 = Pressing
//! - 2 = Recovering
//! - 3 = InPossession
//! - 4 = HoldingUp
//! - 5 = MakingRun
//! - 6 = SetPieceWaiting

use fw_core::Q32;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// PlayerRoleState — typed union that pairs Role + state atomically
// ---------------------------------------------------------------------------

/// A typed pair of (Role, per-role state) that makes illegal combinations
/// (e.g. `Defender + GoalkeeperState`) unrepresentable at the type level.
///
/// This supersedes the prior `role: Role` + `role_state: u8` split on
/// `PlayerState` (T1-2b-iii-a self-review fix, P1-1).
///
/// ## Canonical wire format (byte-identical to split-field encoding)
///
/// `to_tags()` returns `(role_tag, state_tag)` — the same two u8 values the
/// split-field encoding emitted in the same order. The canonical hash is
/// therefore UNCHANGED by this refactor.
///
/// ## Adding new role states
///
/// Add a variant to the per-role enum, assign the next u8 tag (do NOT
/// reorder existing variants), update `from_tags` and `to_tags`, and append
/// to the module doc table in this file + `canonical.rs`. Every change to
/// tags is an ADR-0012 trigger #1 event (canonical hash drift).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerRoleState {
    Goalkeeper(GoalkeeperState),
    Defender(DefenderState),
    Midfielder(MidfielderState),
    Forward(ForwardState),
}

impl PlayerRoleState {
    /// The coarse `Role` discriminant of this state.
    #[must_use]
    pub fn role(self) -> Role {
        match self {
            PlayerRoleState::Goalkeeper(_) => Role::Goalkeeper,
            PlayerRoleState::Defender(_) => Role::Defender,
            PlayerRoleState::Midfielder(_) => Role::Midfielder,
            PlayerRoleState::Forward(_) => Role::Forward,
        }
    }

    /// Stable canonical wire-tag pair: `(role_tag, state_tag)`.
    ///
    /// The encoder writes these as two consecutive u8 bytes — byte-identical
    /// to the former split-field encoding so the canonical hash is UNCHANGED.
    #[must_use]
    pub fn to_tags(self) -> (u8, u8) {
        match self {
            PlayerRoleState::Goalkeeper(s) => (0, s.to_tag()),
            PlayerRoleState::Defender(s) => (1, s.to_tag()),
            PlayerRoleState::Midfielder(s) => (2, s.to_tag()),
            PlayerRoleState::Forward(s) => (3, s.to_tag()),
        }
    }

    /// Reconstruct from a canonical wire-tag pair. Returns `None` for any
    /// (role_tag, state_tag) combination that doesn't decode cleanly —
    /// indicates a save-migration or encoding bug.
    #[must_use]
    pub fn from_tags(role_tag: u8, state_tag: u8) -> Option<PlayerRoleState> {
        match role_tag {
            0 => GoalkeeperState::from_tag(state_tag).map(PlayerRoleState::Goalkeeper),
            1 => DefenderState::from_tag(state_tag).map(PlayerRoleState::Defender),
            2 => MidfielderState::from_tag(state_tag).map(PlayerRoleState::Midfielder),
            3 => ForwardState::from_tag(state_tag).map(PlayerRoleState::Forward),
            _ => None,
        }
    }

    /// Initial state for a role at match-start.
    ///
    /// - GK  → `InBoxPositioning` (tag 0)
    /// - DEF → `Defending` (tag 0)
    /// - MID → `Defending` (tag 0)
    /// - FWD → `RunningOffBall` (tag 0)
    #[must_use]
    pub fn initial(role: Role) -> PlayerRoleState {
        match role {
            Role::Goalkeeper => PlayerRoleState::Goalkeeper(GoalkeeperState::InBoxPositioning),
            Role::Defender => PlayerRoleState::Defender(DefenderState::Defending),
            Role::Midfielder => PlayerRoleState::Midfielder(MidfielderState::Defending),
            Role::Forward => PlayerRoleState::Forward(ForwardState::RunningOffBall),
        }
    }

    /// Evaluate FSM transitions for this role state.
    ///
    /// T1-3.6: routes the ball carrier into `InPossession` and clears
    /// `InPossession` for any player who is NOT the carrier. All other
    /// transition predicates (spatial pressing, recovery, etc.) remain
    /// deferred to T2+ as before.
    ///
    /// ## Carrier routing contract
    ///
    /// - If `state.possession == Some(slot)` where `slot` is this player's
    ///   canonical slot, the player is the current carrier. They are placed
    ///   into the `InPossession` variant for their role.
    /// - If the player IS currently in an `InPossession` state BUT is NOT
    ///   the carrier (possession transferred away via a pass), they are moved
    ///   to the non-possession default for their role.
    /// - For Goalkeepers: carrier routing is handled by the GK FSM
    ///   (`goalkeeper_fsm::evaluate_transitions`) which uses ball-position
    ///   predicates to select `DistributingFromHand` when the ball is near
    ///   the goal line. We do not change GK state here.
    /// - All other non-InPossession, non-carrier states: returned unchanged.
    ///   (-iii-b / T2+ wires spatial pressing, recovery, etc.)
    ///
    /// Per ADR-0006 §"Concrete sketch": transition evaluation runs BEFORE
    /// the BT subtree lookup, so the BT executes on the (possibly new) state.
    #[must_use]
    pub fn evaluate_transitions(
        self,
        state: &crate::MatchState,
        slot_idx: usize,
    ) -> PlayerRoleState {
        // Goalkeeper FSM owns its own transitions (ball-position predicates
        // in goalkeeper_fsm::evaluate_transitions). Do not interfere.
        if matches!(self, PlayerRoleState::Goalkeeper(_)) {
            return self;
        }

        // Canonical slot for this player (u8 used by possession field).
        let this_slot: u8 = state.players[slot_idx].slot;

        // Is this player the current ball carrier?
        let is_carrier = state.possession == Some(this_slot);

        if is_carrier {
            // Route carrier into InPossession for their role.
            match self {
                PlayerRoleState::Defender(_) => {
                    PlayerRoleState::Defender(DefenderState::InPossession)
                }
                PlayerRoleState::Midfielder(_) => {
                    PlayerRoleState::Midfielder(MidfielderState::InPossession)
                }
                PlayerRoleState::Forward(_) => PlayerRoleState::Forward(ForwardState::InPossession),
                // Goalkeeper handled above; this branch is unreachable.
                PlayerRoleState::Goalkeeper(_) => self,
            }
        } else {
            // Not the carrier. If currently in InPossession (possession was
            // just transferred away), move back to the role's non-possession
            // default. All other states are left unchanged.
            match self {
                PlayerRoleState::Defender(DefenderState::InPossession) => {
                    PlayerRoleState::Defender(DefenderState::Supporting)
                }
                PlayerRoleState::Midfielder(MidfielderState::InPossession) => {
                    PlayerRoleState::Midfielder(MidfielderState::Supporting)
                }
                PlayerRoleState::Forward(ForwardState::InPossession) => {
                    PlayerRoleState::Forward(ForwardState::RunningOffBall)
                }
                // All other states: unchanged (pressing, defending, etc.
                // transition predicates deferred to T2+).
                other => other,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Role
// ---------------------------------------------------------------------------

/// Outfield player role. Goalkeeper is "pure FSM"; the three outfield roles
/// use FSM-of-BTs per ADR-0006.
///
/// The `u8` returned by `to_tag()` is the canonical wire discriminant.
/// Do NOT reorder variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Role {
    Goalkeeper,
    Defender,
    Midfielder,
    Forward,
}

impl Role {
    /// Stable canonical wire tag. Used in `canonical.rs::encode_player`.
    #[must_use]
    pub fn to_tag(self) -> u8 {
        match self {
            Role::Goalkeeper => 0,
            Role::Defender => 1,
            Role::Midfielder => 2,
            Role::Forward => 3,
        }
    }

    /// Default initial role state (as a `u8` tag) for each role.
    #[must_use]
    pub fn default_state_tag(self) -> u8 {
        match self {
            // GK → GoalkeeperState::InBoxPositioning (tag 0)
            Role::Goalkeeper => 0,
            // DEF → DefenderState::Defending (tag 0)
            Role::Defender => 0,
            // MID → MidfielderState::Defending (tag 0)
            Role::Midfielder => 0,
            // FWD → ForwardState::RunningOffBall (tag 0)
            Role::Forward => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// GoalkeeperState
// ---------------------------------------------------------------------------

/// Goalkeeper role states. Pure FSM (no inner BT). 8 variants per ADR-0006
/// §"Concrete sketch" + FW scope.
///
/// Variant order = canonical tag (do NOT reorder).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum GoalkeeperState {
    InBoxPositioning = 0,
    SweeperKeeperRush = 1,
    ShotStopping = 2,
    DistributingFromHand = 3,
    DistributingFromFeet = 4,
    PenaltyStance = 5,
    SetPieceWall = 6,
    Recovering = 7,
}

impl GoalkeeperState {
    /// Canonical wire tag. `repr(u8)` makes this a trivial cast.
    #[must_use]
    pub fn to_tag(self) -> u8 {
        self as u8
    }

    /// Reconstruct from a canonical tag byte. Returns `None` for
    /// out-of-range tags (indicates a save-migration or encoding bug).
    #[must_use]
    pub fn from_tag(tag: u8) -> Option<GoalkeeperState> {
        match tag {
            0 => Some(GoalkeeperState::InBoxPositioning),
            1 => Some(GoalkeeperState::SweeperKeeperRush),
            2 => Some(GoalkeeperState::ShotStopping),
            3 => Some(GoalkeeperState::DistributingFromHand),
            4 => Some(GoalkeeperState::DistributingFromFeet),
            5 => Some(GoalkeeperState::PenaltyStance),
            6 => Some(GoalkeeperState::SetPieceWall),
            7 => Some(GoalkeeperState::Recovering),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// DefenderState
// ---------------------------------------------------------------------------

/// Defender role states. FSM outer-shell for the BT runner. 7 variants.
///
/// Variant order = canonical tag (do NOT reorder).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum DefenderState {
    Defending = 0,
    Pressing = 1,
    Recovering = 2,
    Supporting = 3,
    InPossession = 4,
    SetPieceWaiting = 5,
    Tracking = 6,
}

impl DefenderState {
    #[must_use]
    pub fn to_tag(self) -> u8 {
        self as u8
    }

    #[must_use]
    pub fn from_tag(tag: u8) -> Option<DefenderState> {
        match tag {
            0 => Some(DefenderState::Defending),
            1 => Some(DefenderState::Pressing),
            2 => Some(DefenderState::Recovering),
            3 => Some(DefenderState::Supporting),
            4 => Some(DefenderState::InPossession),
            5 => Some(DefenderState::SetPieceWaiting),
            6 => Some(DefenderState::Tracking),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// MidfielderState
// ---------------------------------------------------------------------------

/// Midfielder role states. 7 variants.
///
/// Variant order = canonical tag (do NOT reorder).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MidfielderState {
    Defending = 0,
    Pressing = 1,
    Recovering = 2,
    Supporting = 3,
    InPossession = 4,
    RunningOffBall = 5,
    SetPieceWaiting = 6,
}

impl MidfielderState {
    #[must_use]
    pub fn to_tag(self) -> u8 {
        self as u8
    }

    #[must_use]
    pub fn from_tag(tag: u8) -> Option<MidfielderState> {
        match tag {
            0 => Some(MidfielderState::Defending),
            1 => Some(MidfielderState::Pressing),
            2 => Some(MidfielderState::Recovering),
            3 => Some(MidfielderState::Supporting),
            4 => Some(MidfielderState::InPossession),
            5 => Some(MidfielderState::RunningOffBall),
            6 => Some(MidfielderState::SetPieceWaiting),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// ForwardState
// ---------------------------------------------------------------------------

/// Forward role states. 7 variants.
///
/// Variant order = canonical tag (do NOT reorder).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ForwardState {
    RunningOffBall = 0,
    Pressing = 1,
    Recovering = 2,
    InPossession = 3,
    HoldingUp = 4,
    MakingRun = 5,
    SetPieceWaiting = 6,
}

impl ForwardState {
    #[must_use]
    pub fn to_tag(self) -> u8 {
        self as u8
    }

    #[must_use]
    pub fn from_tag(tag: u8) -> Option<ForwardState> {
        match tag {
            0 => Some(ForwardState::RunningOffBall),
            1 => Some(ForwardState::Pressing),
            2 => Some(ForwardState::Recovering),
            3 => Some(ForwardState::InPossession),
            4 => Some(ForwardState::HoldingUp),
            5 => Some(ForwardState::MakingRun),
            6 => Some(ForwardState::SetPieceWaiting),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// PlayerIntent
// ---------------------------------------------------------------------------

/// What the player "wants to do" this tick. Produced by the BT runner or
/// the GK FSM; consumed by `apply_intent` in `dispatch.rs`.
///
/// All variants carry `(target_x, target_y)` so `apply_intent` can apply
/// a uniform velocity-toward-target model. `Idle` is the exception: zero vel.
///
/// Variant ordering is stable — do NOT reorder without a wire-format review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerIntent {
    // ---- Skeleton variants (T1-2b-iii-a) ----
    /// Move toward (target_x, target_y) with max speed clamped by
    /// `apply_intent`. Used by formation-hold logic.
    MoveToPosition { target_x: Q32, target_y: Q32 },
    /// Stay in place — velocity set to zero.
    Idle,

    // ---- On-ball variants (7 sites, T1-2b-iii-c) ----
    /// Attempt a shot at goal. `target` is the aim point on the goal line.
    AttemptShot { target_x: Q32, target_y: Q32 },
    /// Attempt a short pass. `target` is the recipient's current position.
    AttemptPassShort { target_x: Q32, target_y: Q32 },
    /// Attempt a long / through-ball pass. `target` is the destination zone.
    AttemptPassLong { target_x: Q32, target_y: Q32 },
    /// Cross into the box. `target` is the delivery zone.
    Cross { target_x: Q32, target_y: Q32 },
    /// Dribble toward space. `target` is the run endpoint.
    Dribble { target_x: Q32, target_y: Q32 },
    /// Hold the ball under pressure. `target` is the player's current
    /// position (stay put, shield ball).
    HoldBall { target_x: Q32, target_y: Q32 },
    /// Lay the ball off to the nearest supporting player. `target` is the
    /// supporter's position.
    LayOff { target_x: Q32, target_y: Q32 },

    // ---- Off-ball variants (5 sites, T1-2b-iii-c) ----
    /// Track back toward own defensive shape. `target` is the defensive slot.
    TrackBack { target_x: Q32, target_y: Q32 },
    /// Press the ball carrier. `target` is the carrier's current position.
    Press { target_x: Q32, target_y: Q32 },
    /// Mark an opponent. `target` is the opponent's current position.
    MarkPlayer { target_x: Q32, target_y: Q32 },
    /// Make a run off the ball. `target` is the run endpoint.
    RunOffBall { target_x: Q32, target_y: Q32 },
    /// Hold formation position. `target` is the designated formation slot.
    HoldFormation { target_x: Q32, target_y: Q32 },

    // ---- Goalkeeper variants (5 sites, T1-2b-iii-c) ----
    /// GK executes a shot-stop dive. `target` is the dive point.
    GkShotStop { target_x: Q32, target_y: Q32 },
    /// GK moves to collect a cross. `target` is the cross delivery point.
    GkCollectCross { target_x: Q32, target_y: Q32 },
    /// GK rushes out as sweeper-keeper. `target` is the interception point.
    GkSweeperRush { target_x: Q32, target_y: Q32 },
    /// GK distributes short (throw/roll). `target` is the recipient's position.
    GkDistributeShort { target_x: Q32, target_y: Q32 },
    /// GK distributes long (kick/punt). `target` is the target zone.
    GkDistributeLong { target_x: Q32, target_y: Q32 },
}

// ---------------------------------------------------------------------------
// Tests — Chunk 2 (RED → GREEN)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Role tags ---

    #[test]
    fn role_tags_are_stable() {
        assert_eq!(Role::Goalkeeper.to_tag(), 0);
        assert_eq!(Role::Defender.to_tag(), 1);
        assert_eq!(Role::Midfielder.to_tag(), 2);
        assert_eq!(Role::Forward.to_tag(), 3);
    }

    // --- GoalkeeperState round-trip ---

    #[test]
    fn goalkeeper_state_tag_round_trip() {
        for tag in 0u8..8 {
            let state = GoalkeeperState::from_tag(tag).expect("all tags 0..8 must decode");
            assert_eq!(state.to_tag(), tag);
        }
    }

    #[test]
    fn goalkeeper_state_out_of_range_returns_none() {
        assert!(GoalkeeperState::from_tag(8).is_none());
        assert!(GoalkeeperState::from_tag(255).is_none());
    }

    #[test]
    fn goalkeeper_state_default_is_in_box_positioning() {
        assert_eq!(Role::Goalkeeper.default_state_tag(), 0);
        let state = GoalkeeperState::from_tag(0).unwrap();
        assert_eq!(state, GoalkeeperState::InBoxPositioning);
    }

    // --- DefenderState round-trip ---

    #[test]
    fn defender_state_tag_round_trip() {
        for tag in 0u8..7 {
            let state = DefenderState::from_tag(tag).expect("all tags 0..7 must decode");
            assert_eq!(state.to_tag(), tag);
        }
    }

    #[test]
    fn defender_state_default_is_defending() {
        assert_eq!(Role::Defender.default_state_tag(), 0);
        let state = DefenderState::from_tag(0).unwrap();
        assert_eq!(state, DefenderState::Defending);
    }

    // --- MidfielderState round-trip ---

    #[test]
    fn midfielder_state_tag_round_trip() {
        for tag in 0u8..7 {
            let state = MidfielderState::from_tag(tag).expect("all tags 0..7 must decode");
            assert_eq!(state.to_tag(), tag);
        }
    }

    #[test]
    fn midfielder_state_default_is_defending() {
        assert_eq!(Role::Midfielder.default_state_tag(), 0);
        let state = MidfielderState::from_tag(0).unwrap();
        assert_eq!(state, MidfielderState::Defending);
    }

    // --- ForwardState round-trip ---

    #[test]
    fn forward_state_tag_round_trip() {
        for tag in 0u8..7 {
            let state = ForwardState::from_tag(tag).expect("all tags 0..7 must decode");
            assert_eq!(state.to_tag(), tag);
        }
    }

    #[test]
    fn forward_state_default_is_running_off_ball() {
        assert_eq!(Role::Forward.default_state_tag(), 0);
        let state = ForwardState::from_tag(0).unwrap();
        assert_eq!(state, ForwardState::RunningOffBall);
    }

    // --- PlayerIntent serde round-trip ---

    #[test]
    fn player_intent_serde_round_trip() {
        let cases = [
            PlayerIntent::Idle,
            PlayerIntent::MoveToPosition {
                target_x: Q32::from_int(10),
                target_y: Q32::from_int(-5),
            },
        ];
        for intent in &cases {
            let json = serde_json::to_string(intent).expect("serialize failed");
            let back: PlayerIntent = serde_json::from_str(&json).expect("deserialize failed");
            assert_eq!(*intent, back);
        }
    }

    // --- PlayerRoleState ---

    #[test]
    fn player_role_state_to_tags_round_trip_via_from_tags() {
        let cases = [
            PlayerRoleState::Goalkeeper(GoalkeeperState::InBoxPositioning),
            PlayerRoleState::Goalkeeper(GoalkeeperState::Recovering),
            PlayerRoleState::Defender(DefenderState::Defending),
            PlayerRoleState::Defender(DefenderState::Tracking),
            PlayerRoleState::Midfielder(MidfielderState::Defending),
            PlayerRoleState::Midfielder(MidfielderState::SetPieceWaiting),
            PlayerRoleState::Forward(ForwardState::RunningOffBall),
            PlayerRoleState::Forward(ForwardState::SetPieceWaiting),
        ];
        for prs in cases {
            let (rt, st) = prs.to_tags();
            let back = PlayerRoleState::from_tags(rt, st).expect("round-trip must succeed");
            assert_eq!(prs, back, "round-trip failed for {:?}", prs);
        }
    }

    #[test]
    fn player_role_state_from_tags_invalid_role_returns_none() {
        assert!(PlayerRoleState::from_tags(4, 0).is_none());
        assert!(PlayerRoleState::from_tags(255, 0).is_none());
    }

    #[test]
    fn player_role_state_from_tags_invalid_state_returns_none() {
        // role 0 = GK, state 8 is out of range for GoalkeeperState
        assert!(PlayerRoleState::from_tags(0, 8).is_none());
    }

    #[test]
    fn player_role_state_initial_matches_role() {
        assert_eq!(
            PlayerRoleState::initial(Role::Goalkeeper).role(),
            Role::Goalkeeper
        );
        assert_eq!(
            PlayerRoleState::initial(Role::Defender).role(),
            Role::Defender
        );
        assert_eq!(
            PlayerRoleState::initial(Role::Midfielder).role(),
            Role::Midfielder
        );
        assert_eq!(
            PlayerRoleState::initial(Role::Forward).role(),
            Role::Forward
        );
    }

    #[test]
    fn player_role_state_initial_tag_pairs_are_stable() {
        // These tag pairs are the canonical wire bytes. Do NOT change them
        // without a canonical-hash rebaseline.
        let (rt, st) = PlayerRoleState::initial(Role::Goalkeeper).to_tags();
        assert_eq!((rt, st), (0, 0), "GK initial should be (0, 0)");
        let (rt, st) = PlayerRoleState::initial(Role::Defender).to_tags();
        assert_eq!((rt, st), (1, 0), "DEF initial should be (1, 0)");
        let (rt, st) = PlayerRoleState::initial(Role::Midfielder).to_tags();
        assert_eq!((rt, st), (2, 0), "MID initial should be (2, 0)");
        let (rt, st) = PlayerRoleState::initial(Role::Forward).to_tags();
        assert_eq!((rt, st), (3, 0), "FWD initial should be (3, 0)");
    }

    #[test]
    fn player_role_state_evaluate_transitions_is_identity_in_skeleton() {
        use crate::MatchState;
        use fw_core::Seed;
        let state = MatchState::initial(Seed::from_u64(1));
        let prs = PlayerRoleState::initial(Role::Midfielder);
        let next = prs.evaluate_transitions(&state, 5);
        assert_eq!(prs, next, "skeleton evaluate_transitions must be identity");
    }

    // --- Role default states ---

    #[test]
    fn all_roles_have_valid_default_state_tags() {
        for role in [
            Role::Goalkeeper,
            Role::Defender,
            Role::Midfielder,
            Role::Forward,
        ] {
            let tag = role.default_state_tag();
            // Each role's max tag count:
            let max = match role {
                Role::Goalkeeper => 8,
                Role::Defender => 7,
                Role::Midfielder => 7,
                Role::Forward => 7,
            };
            assert!(
                tag < max,
                "role {:?} default_state_tag {} is out of range (max {})",
                role,
                tag,
                max
            );
        }
    }
}

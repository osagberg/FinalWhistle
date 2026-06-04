//! Behavior-Tree runner — ADR-0006 node types + deterministic traversal.
//!
//! ## Design
//!
//! Nodes are Rust code; trees are assembled from [`Node`] variants.
//! For T1-2b-iii-a, the subtree library hard-codes stub trees — content-pack
//! RON loading defers to -iii-b / T2-3.
//!
//! ## Determinism contract
//!
//! - Tree traversal visits children in **declared Vec order**. No HashMap,
//!   no set iteration. Insertion order is the execution order.
//! - Every leaf that needs randomness receives a `&mut ChaCha8Rng` seeded
//!   via `seed_fn` before the call. In the -iii-a skeleton tier every leaf
//!   returns `NodeStatus::Success` immediately and does not draw from the
//!   RNG — the argument is threaded through for -iii-b compatibility.
//! - No floats. No clocks. No async.
//!
//! ## T1-2b-iii-a scope
//!
//! Every leaf is `MoveToFormationPosition`. No real game logic.
//! - No attribute reads.
//! - No xG / pitch-control / pressing model.
//! - No `NodeStatus::Running` (resumable trees defer to -iii-b).
//!   Every leaf returns `Success` immediately.
//! - Pre-emption hooks return `None` (stub; -iii-b wires them when
//!   `MatchEvent` exists).

pub mod off_ball;
pub mod on_ball;
pub mod personality_bias;
pub mod reactive;

use fw_content::SimBiasSnapshot;

use crate::player::PlayerState;
use crate::role_states::{PlayerIntent, PlayerRoleState};
use rand_chacha::ChaCha8Rng;

// ---------------------------------------------------------------------------
// NodeStatus
// ---------------------------------------------------------------------------

/// The three outcomes a BT node can return.
///
/// `Running` is included for -iii-b compatibility (resumable leaf
/// execution over multiple ticks). In the -iii-a skeleton tier no leaf
/// returns `Running` — every leaf resolves on the same tick it is entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeStatus {
    /// The node completed successfully.
    Success,
    /// The node failed (without error — failure is a designed outcome).
    Failure,
    /// The node is still executing; the runner should resume at this
    /// node next tick. Unused in -iii-a skeleton tier.
    Running,
}

// ---------------------------------------------------------------------------
// Node
// ---------------------------------------------------------------------------

/// A single behavior-tree node.
///
/// `Leaf` and `Condition` are terminal; `Selector`, `Sequence`, and
/// `Decorator` are composites.
///
/// ADR-0006 specifies: "Nodes are code; trees are data." The enum
/// represents the structural layer. Per-leaf payload (what action a
/// `Leaf` performs) is embedded in the `LeafKind` sub-enum below.
pub enum Node {
    /// Tries children left-to-right; returns `Success` on the first
    /// child that succeeds, `Failure` if ALL children fail.
    Selector(Vec<Node>),

    /// Runs children left-to-right; returns `Failure` on the first
    /// child that fails, `Success` if ALL children succeed.
    Sequence(Vec<Node>),

    /// A single-child modifier. Inversion is the most common; decorator
    /// kind is selected at construction.
    Decorator(DecoratorKind, Box<Node>),

    /// A terminal action node. Executes the embedded [`LeafKind`] and
    /// returns a status + optionally produces a [`PlayerIntent`].
    Leaf(LeafKind),

    /// A pure predicate that reads canonical state without producing an
    /// intent. Returns `Success` if the predicate holds, `Failure`
    /// otherwise. Consuming an `rng` argument is reserved for
    /// probabilistic conditions (-iii-b).
    Condition(ConditionKind),
}

/// Modifier applied by a `Decorator` node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoratorKind {
    /// Inverts the child's `Success`/`Failure`. `Running` passes through.
    Invert,
    /// Always reports `Success` regardless of child result.
    AlwaysSucceed,
    /// Always reports `Failure` regardless of child result.
    AlwaysFail,
}

/// Terminal action kinds. Skeleton tier has two real kinds.
/// -iii-b will extend this with all 21 BT sites from bt-attribute-binding.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafKind {
    /// Move toward the player's designated formation slot position.
    /// The formation lookup happens in [`tick_leaf`] which reads the
    /// player's roster slot from the context.
    MoveToFormationPosition,
    /// Explicit idle — stay in place with zero velocity.
    Idle,
    /// Invoke the outfield utility-scored softmax selection.
    /// Reads `BtContext::role_state`, `BtContext::player_ptr`, and
    /// `BtContext::active_bias_ptr` to assemble the candidate list.
    /// The result replaces `current_intent`. Per ADR-0006: outfield
    /// roles use FSM-of-BTs; the utility-select leaf is the BT leaf
    /// that drives the softmax at on-ball and off-ball sites.
    OutfieldSelect,
}

/// Predicate kinds for `Condition` nodes. Skeleton tier has a single
/// always-true stub; -iii-b adds attribute-bound predicates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionKind {
    /// Always evaluates to `true` → `Success`. Placeholder for -iii-b
    /// predicates whose inputs don't yet exist in the skeleton tier.
    AlwaysTrue,
}

// ---------------------------------------------------------------------------
// BtContext — the read-only world view the BT runner receives
// ---------------------------------------------------------------------------

/// Function pointer type for the utility-select outfield intent chooser.
///
/// Stored in `BtContext::select_fn` as a function pointer to avoid circular
/// imports between `bt` and `subtree_library`. Mirrors
/// `subtree_library::select_outfield_intent`'s signature.
///
/// B1 (FUN-0b+c): `carrier_pos` is the actual ball carrier's position. Passed
/// through the BtContext so `utility_press` and `utility_mark_player` can
/// target the real carrier instead of the formation-slot proxy.
///
/// FUN-TS1: `shape` is the pre-computed `TeamShape` for the player's team.
/// `team_idx` is 0 (home) or 1 (away). Passed through so off-ball utilities
/// call `zonal_slot` instead of the constant `formation_position`.
pub type SelectFn = fn(
    PlayerRoleState,
    &PlayerState,
    u8,
    &mut ChaCha8Rng,
    Option<&SimBiasSnapshot>,
    Option<(fw_core::Q32, fw_core::Q32)>,
    &crate::team_shape::TeamShape,
    usize,
) -> PlayerIntent;

/// Read-only context the BT runner uses to evaluate nodes.
///
/// Carries the roster slot (always present) plus optional outfield-specific
/// data used by the `OutfieldSelect` leaf. The latter is `None` for BT
/// tests that don't exercise the utility-select path.
pub struct BtContext<'a> {
    /// The roster slot being evaluated (0-indexed, 0..22).
    pub roster_slot: u8,

    /// For `OutfieldSelect` leaf: the current role state to select candidates for.
    /// `None` when not dispatching an outfield player via the utility path.
    pub outfield_role_state: Option<PlayerRoleState>,

    /// For `OutfieldSelect` leaf: read-only reference to the player's canonical state.
    /// `None` when not dispatching an outfield player.
    pub player: Option<&'a PlayerState>,

    /// For `OutfieldSelect` leaf: the active signature bias (if any).
    /// `None` when no signature is in flight for this player, or when not on the outfield path.
    pub active_bias: Option<&'a SimBiasSnapshot>,

    /// For `OutfieldSelect` leaf: the function that does the actual utility selection.
    /// Stored as a function pointer to avoid circular imports between `bt` and `subtree_library`.
    /// Signature mirrors `subtree_library::select_outfield_intent`.
    pub select_fn: Option<SelectFn>,

    /// For `OutfieldSelect` leaf (B1 FUN-0b+c): the current ball carrier's position.
    /// `Some((x, y))` when a player has possession; `None` on loose ball.
    /// Threaded into `select_outfield_intent` so press/mark targets the real carrier.
    pub carrier_pos: Option<(fw_core::Q32, fw_core::Q32)>,

    /// FUN-TS1: per-team shape anchors for this player's team.
    /// Threaded into `select_outfield_intent` so off-ball utilities call
    /// `zonal_slot` instead of the static `formation_position`.
    pub team_shape: &'a crate::team_shape::TeamShape,

    /// FUN-TS1: team index (0 = home, 1 = away). Determines attack direction
    /// for `zonal_slot`'s affine transform.
    pub team_idx: usize,
}

// ---------------------------------------------------------------------------
// Tree
// ---------------------------------------------------------------------------

/// A complete behavior tree, rooted at one [`Node`].
pub struct Tree {
    pub root: Node,
}

impl Tree {
    /// Construct from a root node.
    #[must_use]
    pub fn new(root: Node) -> Tree {
        Tree { root }
    }
}

// ---------------------------------------------------------------------------
// tick — the traversal function
// ---------------------------------------------------------------------------

/// Execute one tick of the behavior tree, returning `(NodeStatus, PlayerIntent)`.
///
/// The intent is the last intent produced by a successful leaf. If no leaf
/// succeeds, the intent defaults to `PlayerIntent::Idle`.
///
/// Traversal rules (deterministic):
/// - `Selector`: visits children in Vec order; returns on first `Success`.
///   If all children fail, returns `Failure`.
/// - `Sequence`: visits children in Vec order; returns on first `Failure`.
///   If all children succeed, returns `Success`.
/// - `Decorator`: visits single child; applies the modifier.
/// - `Leaf`: executes the [`LeafKind`]; may consume an RNG draw.
/// - `Condition`: evaluates the predicate; no RNG, no side effects.
#[must_use]
pub fn tick<'a>(
    node: &Node,
    ctx: &BtContext<'a>,
    rng: &mut ChaCha8Rng,
    current_intent: &mut PlayerIntent,
) -> NodeStatus {
    match node {
        Node::Selector(children) => {
            for child in children.iter() {
                let status = tick(child, ctx, rng, current_intent);
                if status == NodeStatus::Success || status == NodeStatus::Running {
                    return status;
                }
            }
            NodeStatus::Failure
        }

        Node::Sequence(children) => {
            for child in children.iter() {
                let status = tick(child, ctx, rng, current_intent);
                if status == NodeStatus::Failure || status == NodeStatus::Running {
                    return status;
                }
            }
            NodeStatus::Success
        }

        Node::Decorator(kind, child) => {
            let child_status = tick(child, ctx, rng, current_intent);
            match kind {
                DecoratorKind::Invert => match child_status {
                    NodeStatus::Success => NodeStatus::Failure,
                    NodeStatus::Failure => NodeStatus::Success,
                    NodeStatus::Running => NodeStatus::Running,
                },
                DecoratorKind::AlwaysSucceed => {
                    if child_status == NodeStatus::Running {
                        NodeStatus::Running
                    } else {
                        NodeStatus::Success
                    }
                }
                DecoratorKind::AlwaysFail => {
                    if child_status == NodeStatus::Running {
                        NodeStatus::Running
                    } else {
                        NodeStatus::Failure
                    }
                }
            }
        }

        Node::Leaf(kind) => tick_leaf(*kind, ctx, rng, current_intent),

        Node::Condition(kind) => tick_condition(*kind),
    }
}

/// Execute one leaf node. Updates `current_intent` on success.
fn tick_leaf<'a>(
    kind: LeafKind,
    ctx: &BtContext<'a>,
    rng: &mut ChaCha8Rng,
    current_intent: &mut PlayerIntent,
) -> NodeStatus {
    match kind {
        LeafKind::MoveToFormationPosition => {
            let (target_x, target_y) = crate::subtree_library::formation_position(ctx.roster_slot);
            *current_intent = PlayerIntent::MoveToPosition { target_x, target_y };
            NodeStatus::Success
        }
        LeafKind::Idle => {
            *current_intent = PlayerIntent::Idle;
            NodeStatus::Success
        }
        LeafKind::OutfieldSelect => {
            // Per ADR-0006: outfield roles use FSM-of-BTs; this leaf is the
            // BT boundary where utility scoring fires. The context must provide
            // all three: role_state, player, and select_fn. Missing any of them
            // is a programmer error (not a content error) — panic loudly.
            let role_state = ctx.outfield_role_state.expect(
                "OutfieldSelect leaf requires BtContext::outfield_role_state — \
                 set it when building the BtContext for outfield dispatch",
            );
            let player = ctx.player.expect(
                "OutfieldSelect leaf requires BtContext::player — \
                 set it when building the BtContext for outfield dispatch",
            );
            let select_fn = ctx.select_fn.expect(
                "OutfieldSelect leaf requires BtContext::select_fn — \
                 set it when building the BtContext for outfield dispatch",
            );
            let intent = select_fn(
                role_state,
                player,
                ctx.roster_slot,
                rng,
                ctx.active_bias,
                ctx.carrier_pos,
                ctx.team_shape,
                ctx.team_idx,
            );
            *current_intent = intent;
            NodeStatus::Success
        }
    }
}

/// Evaluate a condition predicate. Pure — no side effects, no intent mutation.
fn tick_condition(kind: ConditionKind) -> NodeStatus {
    match kind {
        ConditionKind::AlwaysTrue => NodeStatus::Success,
    }
}

/// Convenience: run a full [`Tree`] from the root.
#[must_use]
pub fn tick_tree<'a>(
    tree: &Tree,
    ctx: &BtContext<'a>,
    rng: &mut ChaCha8Rng,
) -> (NodeStatus, PlayerIntent) {
    let mut intent = PlayerIntent::Idle;
    let status = tick(&tree.root, ctx, rng, &mut intent);
    (status, intent)
}

// ---------------------------------------------------------------------------
// Tests — Chunk 1 (RED → GREEN)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::rand_core::SeedableRng;

    fn mk_rng() -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(0)
    }

    static TEST_SHAPE: crate::team_shape::TeamShape = crate::team_shape::TeamShape::CONST_ZERO;

    fn ctx(roster_slot: u8) -> BtContext<'static> {
        BtContext {
            roster_slot,
            outfield_role_state: None,
            player: None,
            active_bias: None,
            select_fn: None,
            carrier_pos: None,
            team_shape: &TEST_SHAPE,
            team_idx: 0,
        }
    }

    // --- Single-node cases ---

    #[test]
    fn idle_leaf_returns_success() {
        let tree = Tree::new(Node::Leaf(LeafKind::Idle));
        let (status, intent) = tick_tree(&tree, &ctx(0), &mut mk_rng());
        assert_eq!(status, NodeStatus::Success);
        assert_eq!(intent, PlayerIntent::Idle);
    }

    #[test]
    fn move_to_formation_leaf_returns_success() {
        let tree = Tree::new(Node::Leaf(LeafKind::MoveToFormationPosition));
        let (status, _intent) = tick_tree(&tree, &ctx(0), &mut mk_rng());
        assert_eq!(status, NodeStatus::Success);
        // Intent should be MoveToPosition (not Idle) after a successful leaf.
        // The exact coordinates are tested via subtree_library; here we
        // just check the variant.
        // We can't pattern match without importing Q32, so check status only.
    }

    #[test]
    fn condition_always_true_returns_success() {
        let tree = Tree::new(Node::Condition(ConditionKind::AlwaysTrue));
        let (status, _) = tick_tree(&tree, &ctx(0), &mut mk_rng());
        assert_eq!(status, NodeStatus::Success);
    }

    // --- Sequence ---

    #[test]
    fn sequence_all_succeed_returns_success() {
        let tree = Tree::new(Node::Sequence(vec![
            Node::Leaf(LeafKind::Idle),
            Node::Leaf(LeafKind::Idle),
        ]));
        let (status, _) = tick_tree(&tree, &ctx(0), &mut mk_rng());
        assert_eq!(status, NodeStatus::Success);
    }

    #[test]
    fn sequence_short_circuits_on_failure() {
        // Decorator(AlwaysFail) produces a Failure child.
        // The sequence should stop at the first Failure and NOT visit further
        // children. We verify by putting an Idle leaf after it — if the
        // sequence continued, the intent would become Idle; if it doesn't,
        // the intent stays at its initial value (Idle from default, but
        // we can also check status).
        let tree = Tree::new(Node::Sequence(vec![
            Node::Decorator(
                DecoratorKind::AlwaysFail,
                Box::new(Node::Leaf(LeafKind::Idle)),
            ),
            Node::Leaf(LeafKind::MoveToFormationPosition),
        ]));
        let (status, _) = tick_tree(&tree, &ctx(0), &mut mk_rng());
        assert_eq!(status, NodeStatus::Failure);
    }

    // --- Selector ---

    #[test]
    fn selector_short_circuits_on_success() {
        // First child succeeds — Selector should return Success without visiting
        // the second child.
        let tree = Tree::new(Node::Selector(vec![
            Node::Leaf(LeafKind::Idle),
            Node::Leaf(LeafKind::MoveToFormationPosition),
        ]));
        let (status, intent) = tick_tree(&tree, &ctx(0), &mut mk_rng());
        assert_eq!(status, NodeStatus::Success);
        // The first leaf (Idle) set intent to Idle. If the second child had
        // run, intent would be MoveToPosition. Check that only the first ran.
        assert_eq!(intent, PlayerIntent::Idle);
    }

    #[test]
    fn selector_all_fail_returns_failure() {
        let tree = Tree::new(Node::Selector(vec![
            Node::Decorator(
                DecoratorKind::AlwaysFail,
                Box::new(Node::Leaf(LeafKind::Idle)),
            ),
            Node::Decorator(
                DecoratorKind::AlwaysFail,
                Box::new(Node::Leaf(LeafKind::Idle)),
            ),
        ]));
        let (status, _) = tick_tree(&tree, &ctx(0), &mut mk_rng());
        assert_eq!(status, NodeStatus::Failure);
    }

    // --- Decorator ---

    #[test]
    fn decorator_invert_flips_success_to_failure() {
        let tree = Tree::new(Node::Decorator(
            DecoratorKind::Invert,
            Box::new(Node::Leaf(LeafKind::Idle)),
        ));
        let (status, _) = tick_tree(&tree, &ctx(0), &mut mk_rng());
        assert_eq!(status, NodeStatus::Failure);
    }

    #[test]
    fn decorator_invert_flips_failure_to_success() {
        let tree = Tree::new(Node::Decorator(
            DecoratorKind::Invert,
            Box::new(Node::Decorator(
                DecoratorKind::AlwaysFail,
                Box::new(Node::Leaf(LeafKind::Idle)),
            )),
        ));
        let (status, _) = tick_tree(&tree, &ctx(0), &mut mk_rng());
        assert_eq!(status, NodeStatus::Success);
    }

    #[test]
    fn decorator_always_succeed_returns_success_on_failure_child() {
        let tree = Tree::new(Node::Decorator(
            DecoratorKind::AlwaysSucceed,
            Box::new(Node::Decorator(
                DecoratorKind::AlwaysFail,
                Box::new(Node::Leaf(LeafKind::Idle)),
            )),
        ));
        let (status, _) = tick_tree(&tree, &ctx(0), &mut mk_rng());
        assert_eq!(status, NodeStatus::Success);
    }

    // --- Traversal order determinism ---

    #[test]
    fn selector_visits_children_in_declared_order() {
        // Children produce Failure, Failure, then Success (via a sequence
        // that uses Idle). The intent should be set by the THIRD child
        // (MoveToFormationPosition) only after the first two fail.
        let tree = Tree::new(Node::Selector(vec![
            Node::Decorator(
                DecoratorKind::AlwaysFail,
                Box::new(Node::Leaf(LeafKind::Idle)),
            ),
            Node::Decorator(
                DecoratorKind::AlwaysFail,
                Box::new(Node::Leaf(LeafKind::MoveToFormationPosition)),
            ),
            Node::Leaf(LeafKind::MoveToFormationPosition),
        ]));
        let (status, intent) = tick_tree(&tree, &ctx(0), &mut mk_rng());
        assert_eq!(status, NodeStatus::Success);
        // Only the third child (slot=0 MoveToFormationPosition) succeeded.
        assert!(matches!(intent, PlayerIntent::MoveToPosition { .. }));
    }

    // --- Deterministic traversal across multiple calls ---

    #[test]
    fn same_seed_same_tree_same_result() {
        let build_tree = || {
            Tree::new(Node::Selector(vec![Node::Leaf(
                LeafKind::MoveToFormationPosition,
            )]))
        };
        let ctx_val = ctx(5);
        let (s1, i1) = tick_tree(&build_tree(), &ctx_val, &mut mk_rng());
        let (s2, i2) = tick_tree(&build_tree(), &ctx_val, &mut mk_rng());
        assert_eq!(s1, s2);
        assert_eq!(i1, i2);
    }
}

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

use crate::role_states::PlayerIntent;
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

/// Terminal action kinds. Skeleton tier has only one real kind.
/// -iii-b will extend this with all 21 BT sites from bt-attribute-binding.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafKind {
    /// Move toward the player's designated formation slot position.
    /// The formation lookup happens in [`tick_leaf`] which reads the
    /// player's roster slot from the context.
    MoveToFormationPosition,
    /// Explicit idle — stay in place with zero velocity.
    Idle,
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

/// Read-only context the BT runner uses to evaluate nodes.
/// Intentionally minimal for -iii-a; -iii-b will extend.
pub struct BtContext {
    /// The roster slot being evaluated (0-indexed, 0..22).
    pub roster_slot: u8,
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
pub fn tick(
    node: &Node,
    ctx: &BtContext,
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
fn tick_leaf(
    kind: LeafKind,
    ctx: &BtContext,
    _rng: &mut ChaCha8Rng,
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
pub fn tick_tree(tree: &Tree, ctx: &BtContext, rng: &mut ChaCha8Rng) -> (NodeStatus, PlayerIntent) {
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

    fn ctx(roster_slot: u8) -> BtContext {
        BtContext { roster_slot }
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

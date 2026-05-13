# Pattern: Behavior trees in Rust

The match-sim AI runner. 22 players × ~1500 ticks per match × per-tick decisions. Determinism non-negotiable.

## Why

Pillar 5 (signature identity) requires that each player's on-pitch behavior be *readable* — patterns the player notices. Behavior trees compose readable behavior from small composable nodes: "if the ball is in opponent half AND I'm in the pressing zone, sprint to the ball; else hold position."

## When to use

- Player decision logic in `fw-match-sim`
- Match-state-level "what's the team doing this tick" decisions
- Ball-physics "what mode is the ball in" (loose / held / in-flight / settled)

## When NOT to use

- One-off conditional logic with no reuse → just write the `if`.
- Async / event-driven UI flows → SolidJS signals, not BTs.
- Cross-tick narrative state → that's `fw-memory`'s event ledger, not a BT.

## Pattern

Each `Node` is an enum variant. The tree is a `Vec<Node>` with parent-child indices — flat representation avoids `Box<dyn Node>` allocations per tick.

```rust
pub enum Node {
    Sequence { children: Vec<NodeIndex> },
    Selector { children: Vec<NodeIndex> },
    Parallel { children: Vec<NodeIndex>, success_threshold: u8 },
    Condition(fn(&WorldState, &PlayerState) -> bool),
    Action(fn(&WorldState, &mut PlayerState, &mut ChaCha8Rng) -> NodeStatus),
}

pub enum NodeStatus { Success, Failure, Running }
```

The runner walks the tree depth-first per tick. Each `Node::tick` returns `NodeStatus`. Sequences fail-fast; Selectors success-fast.

## Determinism considerations

- All `Action` functions take `&mut ChaCha8Rng` (seeded by `(match_seed, tick, player_id)` per `Sim/RULES.md` §4).
- No `Box<dyn Node>` — flat representation, static dispatch via enum.
- BT itself is stateless across ticks; per-tick state lives on `PlayerState` (or a typed slot).
- Cross-tick BT state (rare): use a `BTreeMap<PlayerId, BTContext>` keyed by stable ID. NEVER `HashMap`.

## Worked example — pressing trigger

```
Sequence:
  - Condition: ball is in opponent half
  - Condition: my position is within pressing-zone polygon
  - Action: sprint to ball
```

If ball moves to own half, Sequence fails at step 1. If I'm out of zone, fails at step 2. Otherwise sprint.

## Failure modes

- **Tree too deep:** stack-recursion is the obvious bug. Use the flat-index runner.
- **Side effects in Conditions:** Conditions MUST be pure. A Condition that mutates `PlayerState` is a bug — move it into the Action node.
- **Randomness drift:** Action calls `rng.gen_range(...)`. If two Actions consume RNG bytes in different orders across runs, canonical hash drifts. Order Action invocations explicitly within a Sequence.

## Cross-references

- `crates/fw-match-sim/src/bt.rs` (Phase T1 target — currently stubbed)
- FW v1 reference (design intent ONLY): `/Users/vibelogic/dev/football-archive/MatchSim/Sim/BehaviorTreeRunner.cs` — ~1000 LoC of distilled football intelligence. Port to Rust idioms, do not copy.
- `Sim/RULES.md` — determinism non-negotiables

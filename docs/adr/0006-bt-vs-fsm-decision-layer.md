# ADR-0006 — BT vs FSM for the per-player decision layer

**Status:** Proposed

**Date:** 2026-05-13

**Decider:** Claude (synthesis from sports-sim research wave + existing-Rust-sims read-through, 2026-05-13) + Codex (pending pre-T1-2b audit)

---

## Context

ADR-0001 locks the seven-layer match-engine stack over a 60 Hz integration tick and explicitly defers the **per-player decision representation** to this ADR.

The earlier research recommendation (`docs/research/sports-sims/04-ai-techniques-bt-uai-goap-htn.md`, `docs/research/sports-sims/00-synthesis.md`) strongly favored a Behavior Tree backbone, framed partly by a "~3000 LoC match-sim budget" that has since been retracted (`docs/DESIGN_DOC.md` §1). Under that framing, ZOXEXIVO's per-role FSM was characterised as out of budget — `forwarders/states/running/mod.rs` alone is 2,141 lines. With the budget removed, the choice reopens on **architectural merit**.

Non-negotiables, unchanged from ADR-0001: deterministic decision layer (no `thread_rng`, no `HashMap`-iteration-dependent logic, no float arithmetic on canonical state); RNG seeded from `(match_seed, tick, layer_tag, decision_id)`; data-driven where possible so content packs extend behavior without recompiling; composes with the team-tactic FSM (above), utility selector (inside, at on-ball events), influence maps (off-ball targets), and personality bias vector.

Two reference shapes set the poles. **OFM** has no per-player decision layer — out of scope as a model. **ZOXEXIVO** runs a per-role FSM with ~15-20 states per outfield role. State-name visibility is excellent for debugging, but per-state files grow because every state hand-rolls its priority gates, and a new cross-cutting condition requires touching every state where it might fire.

## Decision

We will use a **hybrid FSM-of-Behavior-Trees**: a per-role finite state machine for the coarse role state, with each state's per-tick behavior implemented as a Behavior Tree.

- Each outfield role (defender, midfielder, forward) has a flat enum of **role states** — a coarse catalogue (~6-10 per role) covering the legible "what is this player doing" labels: `Defending`, `Pressing`, `Recovering`, `Supporting`, `InPossession`, `RunningOffBall`, `SetPieceWaiting`. Final per-role lists land in `docs/specs/decision-layer-state-catalogue.md` under T1-2b.
- Each role state owns a **Behavior Tree** that runs when that state is active — selector / sequence / decorator / leaf nodes in a small (~10-30 leaf) tree. Trees share subtrees across states and roles via a content-pack-loaded library of named subtrees (`fwh.core:subtree_pressure_carrier`, etc.).
- **Role-state transitions** evaluate at dispatch time, before the active state's BT runs. Each state declares a short priority-ordered list of `(predicate, target_state)`. Cross-cutting concerns route through a small set of **universal pre-emption hooks** at the dispatcher (analogous to ZOXEXIVO's `should_force_takeball` / `should_yield_takeball`, but defined once rather than per-state).
- **Goalkeeper is pure FSM**, no inner BT. GK behavior is mode-dominated (in-box positioning, sweeper-keeper rush, penalty stance, shot-stopping, distribution); ~10-12 states each implemented as a small Rust function read more clearly than they would as trees.
- **Outfield roles use FSM-of-BTs.** Defender / midfielder / forward share the same runner trait; role-state enum and subtree library differ per role.
- **Utility-scored selector nodes** are a first-class BT node type at on-ball decision points (pass / shoot / dribble / hold), per ADR-0001. The same utility scorer fires regardless of which role state owns the surrounding BT.
- **Personality bias** applies at utility-selector scores (multiplicative) and at probabilistic BT decorator nodes (e.g. `Probability(p * (1 + FlairBias))`). The bias vector does **not** leak into role-state transitions; those stay deterministic-by-predicate.
- **Authorship split.** The role-state catalogue, universal pre-emption hooks, and BT leaf primitives are Rust in `fw-match-sim`. The BT subtrees themselves are **content-pack data** (RON, schema-versioned, content-pack-qualified IDs per `Content/RULES.md`), loaded at match-init and validated by FW-VAL. Nodes are code; trees are data — the Halo BT-editor split.

## Consequences

### Positive

- **State name + tree composability.** The current role state is a readable enum variant — free symbolic handle for logs, commentary salience tagging, and the replay viewer. Inside the state, the BT is small and authored as data: new behavior is "add a leaf plus a subtree", not "edit every state file".
- **Cross-cutting concerns route once.** Universal pre-emption hooks (single-chaser claim, foul reaction, set-piece switchover) live at the dispatcher, not duplicated per state. This is the load-bearing fix for ZOXEXIVO's per-state-file bloat.
- **Content packs extend behavior.** Modders and the bake-time content compiler can ship new BT subtrees without recompiling. Adding a new role state requires Rust — correct boundary, since role states are structural, subtrees are content.
- **Determinism is straightforward.** Transitions are pure predicates over canonical state. BT leaf execution consumes RNG via the seeded `(match_seed, tick, decision_id)` draw, identical to other ADR-0001 layers.
- **Goalkeeper gets the representation that fits.** Pure FSM avoids inventing BT leaves like "decide if this is a sweeper-keeper moment" that are clearer as a state transition.
- **Utility selector composes uniformly.** A single BT node type at on-ball decision points regardless of surrounding role state; personality bias multiplies in at one place.

### Negative

- **Two representations to maintain.** Both an FSM dispatcher and a BT runner. We mitigate by keeping the FSM dispatcher small (transitions + pre-emption hooks) and the BT runner the more featured of the two.
- **"Where does this behavior live?" is a real authoring question.** A new behavior might be a new role state, a new subtree inside an existing state, a new leaf primitive, or a new universal pre-emption hook. We owe a written authoring guide (`docs/specs/decision-layer-authoring.md`, to land alongside T1-2b) so the choice is not ad-hoc.
- **Content-pack-driven BT data adds a validation surface.** Trees authored in RON need FW-VAL coverage (referenced subtrees resolve, leaf primitives exist in the engine, no cycles, bounded depth).

### Neutral

- **Rollback path.** If FSM-of-BTs proves to add more cost than value during T1-2b, the fallback is to drop the role-state FSM and route the BT subtrees via a top-level selector. The subtree library is the load-bearing piece; the FSM around it is the convenience layer. Re-pinning this ADR as Superseded is straightforward if the spike shows the role-state catalogue does not earn its keep.

## Alternatives considered

- **Pure Behavior Tree** (the original research recommendation). Uniform runner; subtrees compose; new behavior is a new leaf. Rejected because "what is this player doing?" has no first-class answer — debugging and commentary salience tagging want a symbolic role state, and a pure BT requires synthesising one from the active leaf path. Goalkeeper especially does not fit a flat tree (mode-dominated behavior reads as named states, not decorator gates).
- **Pure per-role FSM (ZOXEXIVO-style).** Named states for every behavior; per-state file is a unit of debugging. Rejected because per-state files become priority cascades that re-implement the same gates everywhere (the 2,141-line `forwarders/states/running/mod.rs` cautionary tale), and shared cross-role behaviors must be re-implemented per state rather than authored once as a subtree.
- **Single global ENUM of states, each a BT.** Structurally close to the pick, but mixing forward and defender states into one type pessimises pattern-matching and scopes the subtree library poorly. Splitting by role gives the same shape with cleaner types.
- **HTN at the team layer, BTs per player (Killzone 2/3).** Rejected per the research recommendation: football's action space is too shallow to justify planner machinery; the tactic FSM plus utility selector covers the same authoring space at lower complexity.
- **GOAP per player (F.E.A.R.).** Same rejection as HTN, plus replanning at 8 Hz across 22 agents would be bursty work for 1-2-deep action chains.

## Concrete sketch

Goalkeeper — pure FSM, no inner BT:

```rust
pub enum GoalkeeperState {
    InBoxPositioning, SweeperKeeperRush, ShotStopping,
    DistributingFromHand, DistributingFromFeet,
    PenaltyStance, SetPieceWall, Recovering,
}

impl GoalkeeperState {
    pub fn tick(self, world: &World, player: PlayerId, rng: &mut ChaCha8Rng)
        -> (Self, PlayerIntent)
    {
        let next = self.evaluate_transitions(world, player);
        let intent = match next {
            Self::InBoxPositioning => gk_in_box_positioning(world, player),
            Self::SweeperKeeperRush => gk_sweeper_rush(world, player),
            Self::ShotStopping => gk_shot_stopping(world, player, rng),
            // ...
        };
        (next, intent)
    }
}
```

Midfielder — FSM-of-BTs:

```rust
pub enum MidfielderState {
    Defending, Pressing, Recovering, Supporting,
    InPossession, RunningOffBall, SetPieceWaiting,
}

impl MidfielderState {
    pub fn tick(self, world: &World, player: PlayerId,
                trees: &SubtreeLibrary, rng: &mut ChaCha8Rng)
        -> (Self, PlayerIntent)
    {
        let next = self.evaluate_transitions(world, player);
        let tree_id = trees.lookup(Role::Midfielder, next);
        let (status, intent) = trees.run(tree_id, world, player, rng);
        let final_state = match status {
            NodeStatus::Success | NodeStatus::Running => next,
            NodeStatus::Failure => self.fallback_state(),
        };
        (final_state, intent)
    }
}
```

A subtree authored as content-pack data:

```ron
Subtree(
    id: "fwh.core:subtree_midfielder_pressing",
    schema_version: 1,
    root: Selector([
        Sequence([
            Condition(InRange(target: BallCarrier, distance_q32: "0x6.0")),
            Leaf(EngageDuel),
        ]),
        Sequence([
            Condition(HasPassingLane(target: BallCarrier)),
            Leaf(CloseDownPassingLane),
        ]),
        Leaf(MoveToInfluenceMapTarget(map: Danger, polarity: Reduce)),
    ]),
)
```

The dispatcher runs the pre-emption hooks once, then routes by role:

```rust
pub fn dispatch_tick(world: &mut World, rng: &mut ChaCha8Rng) {
    for player_id in world.decision_slice_this_tick() {
        if let Some(intent) = preempt::evaluate(world, player_id) {
            world.apply_intent(player_id, intent);
            continue;
        }
        let intent = match world.role_of(player_id) {
            Role::Goalkeeper => gk_tick(world, player_id, rng),
            Role::Defender   => def_tick(world, player_id, rng),
            Role::Midfielder => mid_tick(world, player_id, rng),
            Role::Forward    => fwd_tick(world, player_id, rng),
        };
        world.apply_intent(player_id, intent);
    }
}
```

`dispatch_tick` is what the canonical-hash regression test in `crates/fw-replay/tests/canonical_hash.rs` exercises across the macOS / Windows / Linux CI matrix.

## References

- `docs/DESIGN_DOC.md` §3 (pillars), §1 (scope ambition reframe)
- `docs/adr/0001-match-engine-architecture.md` (the seven-layer stack this ADR specialises)
- `docs/research/sports-sims/04-ai-techniques-bt-uai-goap-htn.md` (BT recommendation; HTN/GOAP rejection rationale)
- `docs/research/sports-sims/00-synthesis.md` (BT-centric proposal; budget-reframe caveat)
- `docs/research/existing-rust-sims/04-open-football-engine.md` (ZOXEXIVO FSM-per-role taxonomy, universal pre-emption hooks, priority-cascade pathology)
- `docs/research/existing-rust-sims/01-openfootmanager-engine.md` (no per-player decision layer; out of scope as a model)
- `docs/research/existing-rust-sims/00-synthesis.md` (BT-vs-FSM tradeoff under the budget reframe)
- `.claude/rules/Sim/RULES.md` §1-§10 (determinism contract this ADR must hold)
- `.claude/rules/Content/RULES.md` (content-pack ID + schema-versioning rules for BT subtree data)
- Future spec docs to author alongside T1-2b: `docs/specs/decision-layer-state-catalogue.md`, `docs/specs/decision-layer-authoring.md`

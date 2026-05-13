# ADR-0001 — Match engine architecture

**Status:** Proposed (amended 2026-05-13 per Codex full-project audit P1)

**Date:** 2026-05-13 (amended same-day)

**Decider:** Claude (synthesis from research wave 2026-05-13) + Codex (pending pre-T1-2b audit)

**Amendments:** 2026-05-13 — Codex audit P1 fixes: (1) per-player decision runner cadence 8 Hz → 4 Hz (60/8 = 7.5 was not the claimed clean 15-tick window; 60/4 = 15 IS clean and matches FM's reference baseline; responsiveness carried by layer 6 reactive interrupts at 60 Hz). (2) Personality bias vector size 8 → 14 hidden attributes (mirrors ADR-0002's 14-field PersonalityVector; the original "8-element" claim contradicted ADR-0002 silently). The 7-consideration × 14-vector mapping table is now Tranche 4 deliverable at `docs/design/personality-bias-weights.md`.

---

## Context

Phase T1-2b implements the 22-player match runner that is the heart of Final Whistle's simulation. Before any decision-layer code lands, we need to lock the overall AI architecture so that the per-crate work (BT runner, influence maps, tactic FSM, steering) composes coherently and the determinism contract (`Sim/RULES.md`) holds end-to-end.

Constraints, in priority order:

1. **Pillar fidelity.** The five pillars in `docs/DESIGN_DOC.md` §3 require an engine that emits salient events (pillar 2), supports breakthrough-shaped progression (pillar 3), surfaces 24 readable signature moves (pillar 5), and produces tactically-legible match flow (pillar 1). The architecture must accommodate all five — text-first presentation does not relax the simulation requirements; it only relaxes the rendering requirements.
2. **Determinism non-negotiables.** Q32.32 in canonical state; BLAKE3 cross-OS regression; `ChaCha8Rng::seed_from_u64(seed_fn(match_seed, tick, layer, site))` via the canonical seed function (ADR-0009); `BTreeMap` / `Vec` / `IndexMap` only; no `tokio` / async in `fw-match-sim`; no runtime LLM. These are clippy-enforced in sim crates and gate the cross-OS CI matrix.
3. **Procedural fantasy only.** No real-world licensed names. The architecture must not depend on real-world data plumbed at runtime; any structural analytics priors (e.g. xT transition matrices) are baked offline into Q32 lookup tables.
4. **Maintainability under Claude + solo human.** The architecture must be debuggable by reading it, instrumentable without ceremony, and grow in well-defined seams rather than ad-hoc cascades. The reference projects we read both fail this — ZOXEXIVO's `forwarders/states/running/mod.rs` is 2,141 lines of priority cascade; OFM punts on per-player decisions entirely. We take neither path.

The 2026-05-13 scope reframe (`docs/DESIGN_DOC.md` §1) removed LoC and hour budgets as constraints. The earlier `00-synthesis.md` recommendations that hedged toward "lean" choices for budget reasons need to be re-read against pillar fit instead. Specifically: FSM-per-role and influence-map fidelity are no longer cost-bounded; they are clarity-bounded and verification-bounded.

This ADR locks the **stack shape, cadence, and tactic granularity**. The per-player decision representation (BT vs FSM) is deferred to ADR-0006 — both compose with the rest of this stack, and the choice should be made after a small T1-2b spike rather than pre-committed here.

## Decision

We will build the match engine as a **layered, multi-rate AI stack** with seven layers and four distinct cadences over a single 60 Hz integration tick. From top to bottom:

| # | Layer | Cadence | Role |
|---|---|---|---|
| 1 | Team tactic state machine | event-driven + 2 Hz heartbeat | Five states: `HIGH_PRESS`, `MID_BLOCK`, `LOW_BLOCK`, `COUNTER_ATTACK`, `SET_PIECE`. Per team. Parameterises every layer below. |
| 2 | Per-player decision runner | **4 Hz, staggered across 22 players** | The decision skeleton. Same policy both teams. Representation deferred to ADR-0006. Cadence amended 2026-05-13 per Codex audit P1 (clean 60/4=15 math + matches FM's reference cadence; see Cadence rationale below). |
| 3 | Utility selector at on-ball decision points | only at on-ball events | Pass / shoot / dribble / hold. Scores via xG, xT-delta, pitch-control queries. |
| 4 | Personality bias vector (14 hidden attributes per ADR-0002) | per-decision multiplicative bias | The full 14-element PersonalityVector from ADR-0002: Determination, WorkRate, Ambition, Professionalism, Loyalty, Temperament, PressureTolerance, BigMatchAppetite, Adaptability, Aggression, RiskAppetite, Selflessness, Consistency, Versatility. The 7-consideration × 14-vector mapping table lands in `docs/design/personality-bias-weights.md` (Tranche 4 / pre-T1-2b). Bias nudged by salient `MemoryEvent`s. Amended 2026-05-13 per Codex audit P1 (was "8 hidden attributes", which contradicted ADR-0002). |
| 5 | Influence maps (`danger`, `support`, `space`) | **8 Hz** | 32×24 grid baseline. Off-ball positioning consumes maps via index lookup; no agent-vs-agent reasoning. Cadence unchanged — research-recommended 5–10 Hz band; 8 Hz means decisions (4 Hz) sample either fresh maps or 1-tick-stale, which is fine. |
| 6 | Reactive interrupt predicates | 60 Hz | Cheap per-tick predicates: ball state changed, marker arrived, shot incoming. Can preempt the slow-cadence decision mid-action. |
| 7 | Reynolds-style steering | 60 Hz | Separation / arrive / pursue. Pure Q32 arithmetic. Renders decision intent as motion. |

**Cadence rationale.**

- **60 Hz integration tick.** This is the canonical-state advancement rate — ball physics, player position integration, reactive interrupts, steering. We pick 60 Hz over ZOXEXIVO's 100 Hz because the precision is unnecessary for our text-first surface, and we pick 60 Hz over FM's 4 Hz because we need finer granularity for ball-physics events (deflections, ricochets, near-touches) and signature-move trigger windows — these are the readable atoms of pillar 5, and 250 ms of dead time between checks is too coarse for them.
- **4 Hz per-player decision runner, staggered (amended 2026-05-13 per Codex audit P1).** Research recommended 4–10 Hz. The original ADR picked 8 Hz, but the arithmetic for "60 Hz integration / 8 Hz decisions = 15-tick window" doesn't work: 60/8 = 7.5 ticks per decision, which would have forced either an explicit fractional-tick accumulator or a 7-or-8 alternating stagger. Cleaner pick: **4 Hz**, which gives an integer 15-tick window per player and matches FM's reference cadence exactly. Per-player decisions every 250 ms; with 22 players staggered evenly that's ≈1.47 decisions per integration tick (≈22 decisions per 15-tick window). The responsiveness argument that motivated 8 Hz is carried by layer 6 (reactive interrupts at 60 Hz) — those preempt mid-action whenever state changes warrant, so the slow-cadence decision runner doesn't need to be the responsive layer.
- **Tactic state machine: event-driven + 2 Hz heartbeat.** Transitions fire on possession change, shot, set-piece award, scoreline-tipped events. The 2 Hz heartbeat catches drift the events miss (e.g. gradual territorial shift). This is slower than ZOXEXIVO's 10-tick refresh (10 Hz at 100 Hz tick rate) because tactic state is meant to be *coarse* — it parameterises everything below, so churn here ripples expensively. Event-driven transitions with a slow heartbeat keep the team intent stable enough to read in commentary.
- **Influence map regeneration: 8 Hz.** Research-recommended 5–10 Hz band; 8 Hz means decisions (4 Hz) sample maps that are either fresh or 1-tick-stale (since 60/8 = 7.5 ticks per map regen vs. 60/4 = 15 ticks per decision). That's well within the staleness budget — a half-stale map at a 250 ms decision boundary doesn't drift the spatial picture meaningfully. We considered moving maps to 4 Hz to match decisions exactly, but the research-recommended floor for influence maps is 5 Hz, and the small extra cost of 8 Hz buys cleaner reactivity to ball movement.

**Tactic state granularity.** Five states, confirmed from the research recommendation: `HIGH_PRESS`, `MID_BLOCK`, `LOW_BLOCK`, `COUNTER_ATTACK`, `SET_PIECE`. This is the same coarse taxonomy used by ZOXEXIVO (`HighPress / MidBlock / LowBlock / BuildUp / Progression / Attack / AttackingTransition / DefensiveTransition`) collapsed onto our axes. We drop their `BuildUp` / `Progression` / `Attack` split because those are *phases of possession* rather than *tactical intent* — they're useful labels for commentary salience but they're derivable from `(in_possession, ball_zone)` rather than being independent state. We collapse their `AttackingTransition` / `DefensiveTransition` into `COUNTER_ATTACK` (attacking transition) and the leading edge of `HIGH_PRESS` (defensive transition); explicit transition states proved valuable in their codebase but our event-driven transitions on possession change make them redundant. `SET_PIECE` is a hard mode where free-kick / corner / throw-in / penalty subroutines own the player decision space.

**Influence map resolution.** 32×24 grid (768 cells). Research presented 16×12 as the conservative baseline upgradable to 32×24 if cost allowed. With the budget reframe, we take the higher fidelity up front: 32×24 gives ~3.3 m cells on a 105×68 m pitch — fine enough to distinguish channel runs from half-space runs, which our 24-signature taxonomy needs. Three maps (`danger`, `support`, `space`). Re-generated at 8 Hz (independent of the 4 Hz decision runner — see Cadence rationale). Cell update is bounded by player-count × propagation radius; 32×24 × 8 Hz is well inside any plausible compute envelope.

**Determinism contract.** The full stack is sync-only inside `fw-match-sim`. Each layer's RNG draw seeds from `seed_fn(match_seed, tick, layer, site)` where `layer` is a stable `SeedLayer` discriminant per ADR-0009 and `site` is the per-layer disambiguator (e.g. `(player_id << 16) | slot` for `SeedLayer::Decision`, `decision_id` for `SeedLayer::UtilityTieBreak`). The function lives in `fw-core::seed`. All collections that participate in canonical state use `BTreeMap` or `Vec`. The 60 Hz integration tick is the canonical-hash sampling point — pinned BLAKE3 hashes in `crates/fw-replay/tests/canonical_hash.rs` sample at the end of fixed-scenario tick budgets.

## Consequences

### Positive

- **Each layer has one job.** Tactic state owns intent; the decision runner owns choice; utility selectors own on-ball arithmetic; bias vectors own character; influence maps own off-ball positioning; reactive predicates own responsiveness; steering owns motion. Bugs localise; instrumentation slots into one layer at a time.
- **Cadences match the cost shape.** Cheap reactive predicates at 60 Hz; influence-map regeneration at 8 Hz; per-player decisions at 4 Hz; coarse tactic intent at heartbeat-plus-events. Total compute is dominated by the 4–8 Hz layers, each firing only a handful of times per second of match wall-time.
- **Personality bias is composable, not combinatorial.** Eight hidden attributes multiplicatively bias utility scores at on-ball decision points. Same decision skeleton for all 22 players; character emerges from the bias product. Matches CK3 / The Sims pattern (`03-non-sport-emergent-sims.md`).
- **Off-ball positioning becomes tractable.** Influence-map lookups replace the 21-other-agent reasoning that would otherwise blow up combinatorially. This is the load-bearing trick for 22-agent coordination at any quality bar; without it, even a high-budget engine becomes spaghetti.
- **Reactive interrupts preserve responsiveness without per-tick churn.** A player can react to a deflected ball within one 60 Hz tick (~17 ms) without running the full decision pass. Cheap predicates ("am I now the closest defender?", "did the ball state change?") gate early re-evaluation.
- **Determinism contract survives intact.** Every layer is sync, seeded, and uses ordered collections. Cross-OS BLAKE3 regression hashes the integration-tick output; sub-tick layers do not emit canonical state.
- **Frontend never drives the sim.** Tactic state and decision representation are entirely sim-owned; the Tauri IPC boundary reads projections (`SquadDTO`, `MatchFrameDTO`) and enqueues intents (`MatchCommand`) for the next tick (`Tauri/RULES.md` §2).

### Negative

- **Six rates is more moving parts than a single-rate engine.** Reviewers (Codex, future contributors, future-self) need to hold the cadence table when reasoning about the engine. We mitigate by stamping the cadence on every layer's entry point as a doc comment and asserting it in tests.
- **Tactic state transitions must be well-defined.** Five states × event-driven transitions implies a transition matrix. If it's loose, transitions thrash; if it's tight, transitions surprise. We commit to an explicit transition table in `docs/specs/tactic-fsm.md` before T1-2b implementation.
- **Influence-map debugging needs visualisation tooling.** Without a heatmap renderer in the dev UI, off-ball bugs are hard to read. This is a T1-2b sub-task, not a free pass.
- **Bias vector multiplication can collapse personality at the extremes.** If three biases all push the same way, a player with all three at 1.5× gets a 3.375× tilt — likely too sharp. We will clamp the bias product per-decision (per `docs/specs/bt-attribute-binding.md`, authored alongside T1-1).
- **4 Hz decision cadence matches FM's reference baseline.** Per-player decisions every 250 ms. Originally the ADR set this to 8 Hz "for responsiveness", but per Codex audit P1 (2026-05-13) the math (60/8 = 7.5) was wrong AND the responsiveness argument is better carried by layer 6 (reactive interrupts at 60 Hz). The 4 Hz baseline halves decision compute relative to 8 Hz, with no real responsiveness loss because reactive interrupts preempt mid-cadence whenever ball state changes.

### Neutral

- **The BT-vs-FSM choice is deferred to ADR-0006.** Both compose with this architecture: the decision runner sits in the layer-2 slot regardless of representation. Picking the wrong representation costs us layer 2 only; the rest of the stack is unaffected.
- **xT lookup table provenance: resolved in ADR-0003.** Hand-authored transition matrix in `crates/fw-content/src/xt/transitions.ron` (mod-overridable). Real-world-data fitting rejected for procedural-fantasy pillar.
- **Stat-distribution gate and two-engine cross-check** (OOTP-style + EHM-style, per synthesis) are deferred to T2. Both compose cleanly above this stack.

## Alternatives considered

- **Single-rate 60 Hz BT, full re-evaluation every tick.** Rejected: 22 × 60 = 1320 BT walks/sec is well within compute, but it's wasteful (most decisions don't change tick-to-tick) and it muddies the cadence story — there's no clean place for influence maps or tactic state to live. The layered-cadence model is structurally clearer at no meaningful compute cost.
- **ZOXEXIVO-style per-role FSM with no utility / no bias vector / no influence maps.** Rejected: this is the path open-football took, and the cost is a 2,141-line single-state file because everything that would otherwise live in a utility selector or an influence-map lookup ends up as a priority cascade in the FSM body. The per-role FSM remains viable for layer 2 (ADR-0006 will decide) but not at the cost of folding layers 3-5 into it.
- **OFM-style minute-event stochastic abstraction.** Rejected: incompatible with pillar 5 (24 readable signature moves at 60 Hz cadence), pillar 1 (the procedural fantasy world needs to be *seen* moving, not narrated as aggregate samples), and the determinism floor (their `f64` zone arithmetic and `HashMap` accumulators violate our sim rules outright). OFM is a useful reference for the `MatchCommand` IPC shape and the zone-classification overlay, but not for the decision substrate.
- **GOAP / HTN at the per-player layer.** Rejected: research finding (`04-ai-techniques-bt-uai-goap-htn.md`) — football's action space is too shallow to justify planner machinery; chains are 1-2 deep. GOAP would also complicate the determinism story (A* expansion order over hashed action sets is fragile). Revisit only if T2 set-piece authoring genuinely needs multi-step plan composition — and even then, RoboCup-style scripted setplays are likely the right tool.
- **VAEP-style learned action-valuation.** Rejected: gradient-boosted tree models are non-deterministic across platforms in practice, breaking the cross-OS BLAKE3 floor. xG + xT closed-form is sufficient and citation-grounded. Listed in synthesis as ruled-out for the right reason (pillar, not budget).
- **EA / FIFA-style ML-augmented behaviour at runtime.** Ruled out by `CLAUDE.md` §3 — no runtime ML, no runtime LLM. Bake-time content compilation only.
- **Two-engine pattern (lean Dixon-Coles closed-form + full match engine cross-check) as the primary architecture.** Rejected as primary: Dixon-Coles is a calibration reference for aggregate season outputs, not a per-match engine that emits the salient-event ledger pillar 2 requires. Adopted instead as a T2 verification layer above this stack.

## References

- `docs/DESIGN_DOC.md` §1 (scope ambition, 2026-05-13 reframe) and §3 (five pillars)
- `docs/MASTER_PLAN.md` Phase T1 (delivery context for T1-2b)
- `CLAUDE.md` §3 (tech stack, determinism stack) and §7 (code style + determinism patterns)
- `.claude/rules/Sim/RULES.md` (the binding determinism contract)
- `docs/research/sports-sims/00-synthesis.md` (cross-cutting architecture recommendation)
- `docs/research/sports-sims/01-football-manager-match-engine.md` (FM 4 Hz cadence, mid-slice interrupt pattern, hierarchical tactic composition)
- `docs/research/sports-sims/03-non-sport-emergent-sims.md` (personality-as-bias-vector, memory-as-thoughts patterns from DF / CK3 / RimWorld / The Sims)
- `docs/research/sports-sims/04-ai-techniques-bt-uai-goap-htn.md` (layered BT + utility + influence-maps + steering stack; GOAP/HTN rejection)
- `docs/research/sports-sims/05-football-analytics-xg-xt-vaep.md` (closed-form math driving utility scores)
- `docs/research/existing-rust-sims/01-openfootmanager-engine.md` (minute-event approach — adopted `MatchCommand` shape, rejected per-player AI substrate)
- `docs/research/existing-rust-sims/04-open-football-engine.md` (ZOXEXIVO 10 ms / 100 Hz FSM approach — adopted full-tick / light-tick alternation idea, rejected priority-cascade scale and float / HashMap canonical state)
- Prior ADRs: none (this is the first)
- Pending companion: `docs/specs/tactic-fsm.md` (transition table), `docs/specs/bt-attribute-binding.md` (utility-score × attribute mapping), `docs/specs/decision-cadence-stagger.md` (per-player decision phase-offset scheme). All authored alongside T1-1 / T1-2b.
- Pending decision (`/log-decision`): xT lookup table provenance (hand-author vs StatsBomb open data structural fit).
- ADR-0006 (landed 2026-05-13): per-player decision representation — resolved as FSM-of-Behavior-Trees hybrid. Outfield roles use ~6-10 coarse role states each owning a small BT; goalkeeper is pure FSM. Universal pre-emption hooks at dispatcher level. This ADR commits to the layer-2 slot; ADR-0006 fills it.

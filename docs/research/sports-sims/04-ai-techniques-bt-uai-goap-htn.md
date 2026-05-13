# AI techniques for 22-agent football — research notes

**Researched:** 2026-05-13
**For:** Final Whistle T1-2b architecture choice

## Sources

- [Handling Complexity in the Halo 2 AI — Damian Isla, GDC 2005](https://www.gamedeveloper.com/programming/gdc-2005-proceeding-handling-complexity-in-the-i-halo-2-i-ai) — canonical BT-in-games paper. [primary]
- [Building a Better Battle: Halo 3 AI Objectives — Isla, GDC 2008 (slides)](https://web.cs.wpi.edu/~rich/courses/imgd4000-d09/lectures/halo3.pdf) — team-tactic layer over BTs. [primary]
- [Building the AI of F.E.A.R. with GOAP — Jeff Orkin, Gamasutra](https://www.gamedeveloper.com/design/building-the-ai-of-f-e-a-r-with-goal-oriented-action-planning) — STRIPS-derived planner in a shipped AAA. [primary]
- [Applying GOAP to Games — Orkin, paper](https://www.semanticscholar.org/paper/Applying-Goal-Oriented-Action-Planning-to-Games-Orkin/0c35d00a015c93bac68475e8e1283b02701ff46b) — academic write-up of the F.E.A.R. system. [primary]
- [A Hierarchically-Layered MP Bot System for FPS — Tim Verweij, VU Amsterdam 2007 (Killzone 2 thesis)](https://www.guerrilla-games.com/media/News/Files/VUA07_Verweij_Hierarchically-Layered-MP-Bot_System.pdf) — HTN bots, role-layered. [primary]
- [Hierarchical AI for Multiplayer Bots in Killzone 3 — Game AI Pro Ch. 29](http://www.gameaipro.com/GameAIPro/GameAIPro_Chapter29_Hierarchical_AI_for_Multiplayer_Bots_in_Killzone_3.pdf) — production write-up. [primary]
- [HTN Planning in Decima (Horizon Zero Dawn) — Guerrilla](https://www.guerrilla-games.com/read/htn-planning-in-decima) — HTN + utility hybrid in shipped AAA. [primary]
- [Improving AI Decision Modeling Through Utility Theory — Dave Mark & Kevin Dill, GDC 2010 (slides)](https://media.gdcvault.com/gdc10/slides/MarkDill_ImprovingAIUtilityTheory.pdf) — utility-AI canonical talk. [primary]
- [Modular Tactical Influence Maps — Dave Mark, Game AI Pro 2 Ch. 30](https://www.gameaipro.com/GameAIPro2/GameAIPro2_Chapter30_Modular_Tactical_Influence_Maps.pdf) — influence-map design patterns. [primary]
- [Spatial Reasoning for Strategic Decision Making — Kevin Dill, Game AI Pro 2 Ch. 31](http://www.gameaipro.com/GameAIPro2/GameAIPro2_Chapter31_Spatial_Reasoning_for_Strategic_Decision_Making.pdf) — influence maps in RTS, applicable to pitch zones. [primary]
- [Steering Behaviors for Autonomous Characters — Craig Reynolds, GDC 1999](https://www.red3d.com/cwr/steer/gdc99/) — separation/alignment/cohesion definitive source. [primary]
- [Building Utility Decisions into Your Existing Behavior Tree — Bill Merrill, Game AI Pro Ch. 10](http://www.gameaipro.com/GameAIPro/GameAIPro_Chapter10_Building_Utility_Decisions_into_Your_Existing_Behavior_Tree.pdf) — the BT+utility hybrid pattern. [primary]
- [Coordination in multi-agent RoboCup teams — Sci. Direct](https://www.sciencedirect.com/science/article/abs/pii/S0921889001001373) — soccer-specific role assignment. [secondary]
- [Multi-robot coordination using Setplays — RoboCup MSL & sim leagues](https://www.sciencedirect.com/science/article/abs/pii/S0957415810000851) — set-piece coordination for 11-a-side. [secondary]

## Per-technique findings

### Behavior Trees

- **Halo 2 (Bungie, 2004)** popularised BTs in AAA; Isla's GDC 2005 paper is the canonical reference. Trickle-down decision-making: from root downward, selectors and sequences pick a leaf each tick.
- Cost is cheap: O(depth) per tick, typically <500 ns per agent on modern CPUs for ~50-node trees. 22 agents at 60Hz = ~1320 evaluations/sec, trivial.
- Strengths: debuggable (you can trace exactly which node fired), composable (subtrees compose), easily authored by designers in tools (Halo had a custom editor).
- Weaknesses: fixed structure — a BT can't *plan* a novel action chain. State explosion when many conditions cross-cut; "is in possession AND opponent within 3m AND has passing lane" creates duplicated branches.
- **Halo 3 added a higher Objectives layer above the per-NPC BTs** (Isla 2008) — exactly the "team tactic → individual behaviour" three-layer model football needs. Squad-level objectives ("press the ball carrier", "fall back to box") parameterise per-agent BTs.

### Utility AI

- **The Sims (Maxis, 2000)** is the canonical visible case: each object advertises a "score" against each Sim need, the Sim picks the action with the highest combined utility.
- Dave Mark / Kevin Dill formalised the technique at the GDC AI Summit (2010, 2012). Pattern: enumerate considerations → response curves (linear, quadratic, logistic, bell) → multiply or add into a score → highest score wins.
- **Killzone 2's squad-level decisions** use utility-scored options on top of the HTN; Decima's HZD AI is HTN + utility hybrid.
- Cost: O(actions × considerations) per agent per evaluation. For football "should I pass / dribble / shoot / hold", ~4 actions × ~6 considerations = 24 lookups — negligible.
- Strengths: graceful degradation under conflicting signals; tuning lives in data curves rather than code; "do the least-bad thing" emerges automatically. Weaknesses: harder to debug ("why did it pick X?" requires inspecting scores); designers can't read it as fluently as a BT.

### Goal-Oriented Action Planning

- **F.E.A.R. (Monolith, 2005)** — Jeff Orkin's GOAP, STRIPS-derived. NPC has goals (KillEnemy, Patrol) and actions with precondition/effect tuples; A* over action space finds a plan.
- Cost: A* search per goal-change. F.E.A.R. ran ~6-8 agents on Xbox-era hardware; planning was bursty (only re-plans on world-state change). 22 agents replanning continuously would be expensive but is rarely necessary if replans are event-driven.
- Strengths: emergent action chains that designers didn't author ("blow open door → flank → suppressive fire"). Weaknesses: action library design is the real cost; debugging requires plan-trace tooling; planner thrash if preconditions oscillate.
- **Likely overkill for football**: action space is small (pass, shoot, dribble, tackle, mark, run-off-ball) and the chains are 1-2 deep. Sweat-the-tactics-layer effort, not the action-planner-layer effort.

### Hierarchical Task Networks

- **Killzone 2/3 (Guerrilla, 2009/2011)** — Verweij's thesis + Game AI Pro Ch. 29. HTN decomposes a high-level task ("AttackPosition") into method alternatives, each yielding a subtask sequence terminating in primitive actions.
- **Horizon Zero Dawn (2017)** — HTN in Decima for the machines; combined with utility considerations + group-coordination layer. The clearest published precedent for "high-level team tactic decomposes into per-agent action sequences."
- Cost: planning is intermittent (when current plan completes or invalidates); each plan is O(branching × depth). Killzone bots planned at <1ms per replan on PS3.
- Strengths: natural fit for "team plays high press → for each player, decompose to role-appropriate sequence". Designers think in tasks, not actions. Weaknesses: more upfront machinery than BTs; HTN authoring tools are scarcer than BT tools.

### Steering Behaviors

- **Craig Reynolds' boids (1986, SIGGRAPH 1987; GDC 1999 expansion)** — separation + alignment + cohesion as additive force vectors. Cheap (O(neighbours)) and emergent.
- Football needs these implicitly: separation (don't bunch), cohesion (maintain shape), seek (pursue ball / mark man), arrive (slow into position), pursue (intercept moving ball).
- Cost: ~22 × ~10 neighbour-checks per tick = ~220 ops, trivial. Spatial hashing brings it down further if needed.
- Strengths: solves the "low-level locomotion looks plausible" problem with almost no decision logic. Weaknesses: pure steering produces flocking, not football — you need a decision layer above to set targets. Steering is the **output** of the decision pipeline, not a substitute for one.

### Influence Maps

- Standard in RTS AI from the 1990s onwards (Age of Empires-era pathfinding cost maps, more sophisticated in Empire Earth / Kohan II / Supreme Commander). Dave Mark's Modular Tactical Influence Maps and Kevin Dill's Spatial Reasoning chapters in Game AI Pro 2 are the design-pattern references.
- Football maps directly: "danger" map (opposition influence), "support" map (teammate influence), "space" map (open zones = inverse of combined), "threat-to-goal" gradient.
- Cost: O(grid cells × propagation radius) per update; usually updated at 5-10 Hz, not 60 Hz. 100×60 grid at 10 Hz = ~60k cell ops/sec — cheap.
- Strengths: agents query "where should I be?" with a single map lookup instead of reasoning about 21 other agents. Naturally captures off-ball positioning, which is 90% of football. Weaknesses: tuning is fiddly; needs visualisation tooling to debug.

## Hybrid approaches in shipped games

Pure single-technique AI is rare in AAA. Documented combinations:

- **Halo 3** — per-agent BTs orchestrated by a squad-level Objectives system. Top layer assigns roles ("flanker", "suppressor"); BTs execute role-appropriate behaviour. This is the closest shipped precedent to "team tactic → role → individual" for combat-style coordination. [Isla 2008]
- **Killzone 2/3** — HTN at the squad level for objectives ("clear bunker", "advance to waypoint"), per-bot BTs for primitive execution, plus tactical map (influence-like) for spatial queries. [Verweij 2007; Game AI Pro Ch. 29]
- **Horizon Zero Dawn** — HTN + utility considerations + group-coordination layer; the utility piece scores HTN method alternatives. [Decima HTN talk]
- **BT + Utility selectors** — Bill Merrill's Game AI Pro Ch. 10 documents how to add a utility-scored selector node to an existing BT. Keeps BT's debuggability while letting numeric tuning live in score functions. Widely adopted post-2013 across AAA.
- **RoboCup soccer-sim teams** — almost universally use *role assignment* (often utility-scored or market-auction-based) above per-player decision logic, with "setplays" as scripted multi-agent sequences for restarts. This is the direct football precedent; coordination is explicitly hierarchical (formation → role → individual). [RoboCup setplays paper; Coordination in multi-agent RoboCup teams]
- **No primary source** describes FIFA's or eFootball's internal architecture in detail; both are closed-source, and recent EA marketing emphasises ML-augmented behaviour. Treat as data points, not blueprints.

## Recommended architecture for Final Whistle T1-2b

- **Backbone: Behavior Tree per player.** Halo-proven, debuggable, cheap, composable, easy to snapshot for determinism tests. Maps cleanly to ~30-50 leaves covering pass/shoot/dribble/tackle/mark/run/cover. Fits the ~3000 LoC sim budget.
- **Above the BT: a Team Tactic layer (Halo-3-Objectives-style).** A simple state machine or rule table — "high press" / "mid block" / "low block" / "counter" — that parameterises every BT with per-role intent for the current tick. The BT reads the tactic; the tactic does not need to be a planner.
- **Inside the BT: utility-scored selectors at the on-the-ball decision points** (Merrill pattern). The choice between "pass / shoot / dribble / hold" is a utility selector; everything else stays a plain BT. Keeps the BT readable while putting numeric design knobs where they matter most — the on-ball moment.
- **Off-ball positioning: influence maps.** Update at 5-10 Hz, not 60 Hz. Three maps suffice: danger, support, space. Off-ball BT leaves query the map for "where should I be?" instead of doing 21-agent reasoning. This is what makes 22-agent tactics tractable.
- **Locomotion: steering behaviors as the BT's *output*.** Every BT leaf produces a steering target (point, agent, vector), not raw position. Separation + arrive + pursue compose the actual movement. Q32-deterministic — Reynolds maths is purely arithmetic, no floats required.
- **Explicitly avoid GOAP and HTN at T1-2b.** Football's action space is too shallow to justify planner machinery; the tactic-layer + utility-selector combo gives 80% of the decision quality at 20% of the complexity. Revisit HTN only if T2 set-piece authoring (corners, throw-ins, free-kicks) demands multi-step plan composition — and even then, **scripted setplays** (RoboCup-style) are likely the right tool.

## Open questions

- **How often does the Team Tactic layer reconsider?** Per-tick (60 Hz) is wasteful; per-second is too laggy. Likely event-driven (turnover, shot, set-piece) plus a 2-5 Hz heartbeat. Worth a small spike at T1-2b.
- **Influence-map grid resolution.** 100×60 (1m cells) vs 50×30 (2m cells) vs 25×15 (4m cells). Determinism + cost favour coarser; tactical fidelity favours finer. Needs prototyping against breakthroughs telemetry.
- **Utility-selector score determinism.** Multiple actions with equal top score → tie-break must be deterministic. Stable order + low-bit RNG nudge keyed on `(match_seed, tick, event_id)` per §4 of Sim/RULES. Cheap, but spec it in the BT runner doc.
- **Role assignment for off-ball runs.** RoboCup uses market-auction allocation; Halo uses Objectives-system assignment. Football's role layer is more structured (formations pre-assign roles), so probably formation-driven with situational overrides — but is "the nearest defender presses" a tactic-layer call or an emergent BT outcome? Both work; pick one and document.
- **Replan / re-evaluation triggers.** Pure BT re-evaluates every tick; that may be wasteful given many leaves are "stay on current run". A "behaviour cooldown" pattern (don't reconsider for N ticks unless world state changes meaningfully) keeps cost predictable for 22×60Hz.

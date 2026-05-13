# Synthesis — sports-sim research wave, 2026-05-13

**Inputs:** 8 parallel research agents covering FM match engine, other deep-sim sports, non-sport emergent sims, AI techniques, football analytics, verification/QA, player attributes, and bibliography. Each agent's full notes live in this directory as `01-…` through `08-…`. This doc is what T1-2b should actually be built against.

**Important caveat (2026-05-13 reframe):** several of this synthesis's recommendations were originally framed around a "~3000 LoC budget for match-sim" constraint that has since been retracted (see `docs/DESIGN_DOC.md` §1 "Scope ambition"). The pillar constraints (determinism, no-runtime-LLM, procedural-fantasy, text-first) all remain. What changes is that **the BT-vs-FSM debate, attribute count, subsystem depth, influence-map fidelity, and signature catalogue size are now unbounded by LoC** — choose what serves the design, not what fits the budget. Specifically:
- **VAEP is still ruled out** (gradient-boosted model = non-deterministic across platforms, breaks pillar) but for the right reason, not for budget.
- **FSM per-role state taxonomies (ZOXEXIVO-style ~80 states with 2k-line state files) are back on the table** — choice is clarity-vs-composition now, not bytes.
- **Attribute count grows from "24+8 recommended for tight budget" to "whatever the design needs"** — FM-class (~56) or beyond is on the table; 32 was a research-paper midpoint, not a cap.
- **Influence maps can be full-pitch continuous rather than 16×12 grid** if it produces better positioning.
- **Subsystems (psychology, chemistry, coach AI, referee, training, transfer market, board, media)** each as fully-realized modules rather than scaffolded stubs.

---

## TL;DR — the composed architecture

The research converged on a single coherent engine shape. Stack from top to bottom:

| Layer | Rate | Role |
|---|---|---|
| Team tactic state machine | ~4 Hz, transitions on game-state events | "High press / mid block / low block / counter / set-piece". Parameterizes everything below. |
| Per-player behavior tree | 4-10 Hz, staggered across the 22 players | The decision skeleton. Readable + debuggable. Same tree both teams (policy-symmetric). |
| Utility selector inside the BT | only fires at on-ball events | Pass / shoot / dribble / hold. Scores candidates via xG, xT-delta, pitch-control. Bias vector tilts the score. |
| Personality bias vector (8 hidden attributes) | per-player, baked at generation, nudged by salient `MemoryEvent`s | Multiplicative bias on utility scores. Aggression, FlairBias, Composure, RiskAppetite, WorkRate, Selflessness, Determination, PressureTolerance. |
| Influence maps (danger, support, space) | 5-10 Hz | The 22-agent coordination trick. Players consume the maps for off-ball positioning, never reason about 21 other agents directly. |
| Reactive interrupt predicates | 60 Hz | Cheap per-tick checks: "ball state changed", "marker arrived", "shot incoming". Can preempt the slow-cadence BT mid-decision. |
| Steering output | 60 Hz | Reynolds separation/arrive/pursue. Pure Q32 arithmetic. The BT picks an intent; steering renders it as motion. |

Six rates, but each is cheap. Total compute is dominated by influence-map regeneration + the BT root walks; both are bounded.

This is the **FM match engine pattern (Agent 1) layered with CK3-style personality bias (Agent 3) over a Halo-style BT-with-utility-selectors (Agent 4), driven by football-analytics math (Agent 5), with the determinism + verification floor that exceeds every shipped competitor (Agents 2, 6).**

---

## The math, concretely

These are the formulae that drive the utility scoring inside the BT. All Q32-friendly; all closed-form; all citable to public sources.

**Shot utility = xG.** Closed-form logistic:

```
xG(distance_to_goal, angle_to_goal, defender_pressure) =
  sigmoid(β0 + β1·distance + β2·angle + β3·pressure)
```

`β` coefficients hand-tuned (T1-2b) or fit against public StatsBomb data (T2 if needed). A sigmoid look-up table covers the math in Q32. Distance + angle alone hits ~85% of full-model AUC per the analytics literature — so we don't need 20-feature shot models to be credibly realistic.

**Pass utility = xT-delta.** Pre-bake a 16×12 grid of expected-threat values at content-bake time:

```
xT[x, y] = shoot_prob[x, y]·goal_prob[x, y]
        + move_prob[x, y] · Σ transition[(x,y) → (z,w)] · xT[z, w]
```

Solve the Bellman fixed-point offline. Ship as a 192-entry Q32 lookup table. Pass scoring at runtime is `xT[destination] - xT[origin]` — zero math beyond two table lookups. The bake provenance is the open question: hand-author the transition matrix, or fit against StatsBomb open data (which is technically real-world but only structural, not licensed players/clubs)? Flagged in `docs/research/sports-sims/05-football-analytics-xg-xt-vaep.md` for `/log-decision`.

**Press trigger = compound predicate**, validated by Bauer & Anzer's 5-second-rule study:

```
press_trigger =
    ball_just_lost
  ∧ ticks_since_loss < 5 seconds
  ∧ distance_to_ball < tactic.press_radius
  ∧ team_tactic == HIGH_PRESS
```

When true, the player's BT enters the press subtree. PPDA (passes per defensive action) is a derived stat we measure for T1-9 assertions, not a BT input.

**Pitch control = Spearman-style per-point query.** Don't compute over the whole pitch. When a player needs to know "who arrives first at point P?":

```
τᵢ = τ_react + distance(player_i, P) / v_max + angular_penalty
P_arrive(i) = sigmoid((mean(τ) - τᵢ) / σ)
```

Query only at decision points (shot target, pass receiver candidates, press anchor). 22 × point ≈ 22 closed-form evaluations per decision — cheap. **Pressing intensity** reuses this kinematics — `1 − Π(1 − P_arrive,i)` over the defending team gives a single intensity scalar.

**Influence maps** for off-ball positioning. Three maps over a 16×12 grid (or 32×24 if budget allows): `danger`, `support`, `space`. Regenerated every 5-10 Hz from current player positions. Each player's off-ball BT consults the maps via index lookup — no agent-vs-agent reasoning. This is the load-bearing trick for 22-agent coordination without combinatorial blow-up.

---

## Player attribute schema (Agent 7)

**32 attributes per player.** 24 visible, 8 hidden. All `Q32` in canonical state.

The **24 visible** split asymmetrically — football is less physical-dominated than basketball, so the proposed split is 9 technical / 9 mental / 6 physical:

- **Technical (9):** Finishing, Passing, FirstTouch, Dribbling, Crossing, LongShots, Heading, Tackling, Marking
- **Mental (9):** Anticipation, Composure, Decisions, Vision, OffTheBall, Positioning, Concentration, Bravery, Teamwork
- **Physical (6):** Pace, Acceleration, Stamina, Strength, Agility, Balance

The **8 hidden** ARE the personality bias vector. Same fields that bias the BT utility scores:

- **Determination, PressureTolerance, FlairBias, WorkRate, Aggression, Selflessness, RiskAppetite, Composure**

Each visible attribute is a Q32 in [0, 1]; each hidden is also Q32 in [0, 1] (no FM-style 1-20 integer scale — we're not constrained by display).

**Current Ability vs Potential Ability:** keep both. CA is the sum (weighted by role) of current visible attributes; PA is the cap that ageing curves and breakthroughs push CA toward or past. PA itself is mutable — but only by salient `MemoryEvent`s (this is our pillar 3 differentiator, not the bolt-on cinematic breakouts that shipped sims do on top of linear XP).

**Aging curves are universally asymmetric** across every sport: 5-8 year ramp from debut age, peak 27-29, gentle decline through early 30s, cliff at 33-35. Public analytics graphs match this within tolerance. Position-dependent only at margins (GKs peak later, sprinters peak earlier). Implement as a `Q32 → Q32` lookup table per attribute family.

---

## Memory ledger (Pillar 2) + breakthroughs (Pillar 3)

Agent 3's "memory-as-decaying-thoughts" pattern from RimWorld/DF maps directly:

- Every salient event lands in the event ledger as a timestamped `MemoryEvent`.
- Each event has a `salience` score (Q32, 0..1) and a `decay_function` (linear, exponential, or "never decays").
- Readers project the ledger into context-specific text (commentary, scout reports, press headlines).
- **Breakthroughs are not bolt-on cinematic events.** They're salience-gated ledger triggers: when accumulated salience around a player's signature trait crosses a threshold, a `MemoryEvent::Breakthrough` redraws PA in the relevant attribute family.

Three concrete shipping sim patterns for scout uncertainty (Pillar 4):
- **OOTP**: separate `true_ratings` vs `scout_ratings` stores. Two tables, scout-perceived gets updated as scout observes player.
- **FOF**: colored attribute *ranges* keyed to scout skill. Low-skill scout sees "Finishing: 5-15"; high-skill scout sees "Finishing: 11-13"; truth is "Finishing: 12".
- **CK3**: active/inactive trait inheritance — recessive-gene model for hidden attributes surfacing across generations.

Recommend **FOF's range model** for FW since it directly maps to our pillar's "biased scouts disagree" framing, and surfaces uncertainty in the UI without needing two parallel attribute tables.

---

## What we're doing differently from the shipped industry

Two findings from the research that recur across multiple agents:

**Our determinism floor exceeds every shipped sports sim.** FM, OOTP, FOF, EHM, EA Sports — none publicly commit to cross-OS canonical-hash regression. Most use thread RNG and are non-deterministic by construction. Our Q32 + BLAKE3 + ChaCha8Rng + BTreeMap-only pinned-hash matrix is **genre-leading discipline**. This isn't a "we have a feature they don't" — it's a structural choice that makes everything downstream cheaper to verify, reproduce, and mod.

**Our "growth lives in the ledger first" approach is genuinely differentiated.** Every shipped sports sim ships breakthrough mechanics as cinematic bolt-ons on top of a linear XP / training base. Madden, NBA 2K, FM, Front Office Football — all the same pattern. Pillar 3 ("rare narrative growth moments redraw player ceilings") really is novel. The research found no shipped precedent for "the memory ledger IS the development system."

Both findings should land in the DESIGN_DOC as positioning claims, not just internal motivation.

---

## What we should adopt that we're missing

Two patterns from the shipped industry we don't currently have:

**OOTP-style stat-distribution gate.** Add a CI gate that runs N-season sims at low fidelity and KS-tests against real-world distribution bands: goals/match (~2.7), shots/match (~12-14), completion rate (~80%), top-scorer-concentration, card distribution, home-advantage band, etc. This catches behavioral drift that proptest invariants miss. Lands as a T2 row, after we have content for full seasons.

**EHM-style two-engine cross-check.** A lean Dixon-Coles-style closed-form stat model that simulates aggregate season outputs cheaply, used as the calibration reference for the full match engine. When the full ME produces wildly different aggregates than Dixon-Coles for the same input, that's a flag. Also a T2 row.

---

## Architecture decisions to lock in before T1-2b implementation

These need either `/log-decision` entries or commit-body documentation before the first BT runner code lands:

1. **BT root re-evaluation cadence.** Recommend 4 Hz baseline, staggered across the 22 players so 5-6 players re-evaluate per tick (rotating). Reactive interrupts at 60 Hz can preempt mid-decision when state changes warrant. This is the agent-1 FM pattern.
2. **Team tactic state machine — granularity.** Recommend 4-5 tactical states (HIGH_PRESS, MID_BLOCK, LOW_BLOCK, COUNTER_ATTACK, SET_PIECE) with explicit transition rules. Per-team, transitions on game-state events (lost ball, gained ball, time pressure, scoreline pressure).
3. **Influence map resolution.** Recommend 16×12 grid baseline (192 cells), upgradable to 32×24 if budget allows. Regenerate at 5-10 Hz. Three maps: danger / support / space.
4. **xT LUT bake provenance.** Hand-author the transition matrix OR fit against StatsBomb open data? Open. Needs `/log-decision` — touches the "no real-world licensed data" pillar (data is structural not nominal, but worth a clear decision).
5. **Utility-tie-break determinism.** When two actions have identical utility scores, how do we deterministically pick one? Recommend: `ChaCha8Rng::seed_from_u64(seed_fn(match_seed, tick, decision_id)).gen_range(0..n_tied)`. Sim/RULES §4 contract preserved.
6. **`signature_readiness` ticker.** Q32 in [0, 1]. Per player. Ticks up when salient events accumulate around a player's primary attribute family. Breakthrough fires at threshold + per-pillar narrative gate. Distinct from CA/PA system.
7. **`docs/specs/bt-attribute-binding.md`** — new spec doc, authored alongside T1-1, mapping each BT decision to the attribute(s) it consumes. Prevents "Finishing affects passing" type bugs.

---

## Recommended next steps before `/next` T1-1

1. **Read `openfootmanager`** (Agent 8 bibliography) — same stack as us (Rust + Tauri + JS frontend). Strongest architectural prior art on GitHub. Could save weeks; could be a cautionary tale. Either way, we shouldn't reinvent without reading.
2. **Read Karun Singh's xT post + Dave Mark's two utility-AI GDC talks** (bibliography top-10). 2-3 hours of reading; covers ~70% of the formula + decision architecture we'll write.
3. **Log decisions on the 7 architecture items above.** Either in `docs/DECISIONS.md` via `/log-decision`, or in commit bodies during T1-1 implementation. Don't let any of them slip silently into the code.
4. **Optional: Codex pre-T1-2b audit** (per phase-T0 postmortem recommendation). Hand Codex this synthesis + the 8 detail docs; they audit the architecture before any sim code is written. Same model as the pre-T0 audit that caught 14 real findings.

---

## Cross-references

- `docs/research/sports-sims/01-football-manager-match-engine.md` — Agent 1 (FM 4Hz cadence, mid-slice interrupts)
- `docs/research/sports-sims/02-other-deep-sim-sports.md` — Agent 2 (canonical event ledger, continuous-physics, determinism floor)
- `docs/research/sports-sims/03-non-sport-emergent-sims.md` — Agent 3 (personality-as-bias, utility-picks-BT-runs, memory-as-thoughts)
- `docs/research/sports-sims/04-ai-techniques-bt-uai-goap-htn.md` — Agent 4 (layered BT/utility/IM/steering stack, recommend against GOAP/HTN)
- `docs/research/sports-sims/05-football-analytics-xg-xt-vaep.md` — Agent 5 (xG/xT/pitch-control formulae, press triggers)
- `docs/research/sports-sims/06-verification-qa-in-sims.md` — Agent 6 (9 concrete assertions for T1-9, OOTP stat-gate, EHM two-engine)
- `docs/research/sports-sims/07-player-attributes-progression.md` — Agent 7 (32-attribute schema, aging curves, three scout-uncertainty patterns)
- `docs/research/sports-sims/08-bibliography.md` — Agent 8 (~55 high-signal links, top-10 reading list, openfootmanager flagged)
- `docs/design/dev-verification.md` — three-layer dev verification strategy (informed by Agent 6)
- `docs/MASTER_PLAN.md` Phase T1 — where this synthesis lands as implementation

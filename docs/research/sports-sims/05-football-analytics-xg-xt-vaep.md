# Football analytics models for Final Whistle — research notes

**Researched:** 2026-05-13
**For:** T1-2b BT runner — what models inform shooting / passing / pressing decisions

## Sources

- [Introducing Expected Threat — Karun Singh](https://karun.in/blog/expected-threat.html) — 16x12 grid + Bellman iteration. [devblog, primary]
- [Actions Speak Louder than Goals (VAEP) — Decroos et al., KDD 2019](https://arxiv.org/pdf/1802.07127) — GBM action valuer. [academic, primary]
- [Beyond Expected Goals — Spearman, SSAC 2018](https://www.researchgate.net/publication/327139841_Beyond_Expected_Goals) / [Physics-Based Pass Probabilities — Spearman, SSAC 2017](https://www.researchgate.net/publication/315166647_Physics-Based_Modeling_of_Pass_Probabilities_in_Soccer) — pitch control. [academic, primary]
- [Pressing Intensity — Bauer et al., arXiv 2501.04712](https://arxiv.org/html/2501.04712) — closed-form pressure from pitch-control. [academic, primary]
- [Data-driven counterpressing detection — Bauer & Anzer, DMKD 2021](https://link.springer.com/article/10.1007/s10618-021-00763-7) — operationalizes the 5-second rule. [academic, primary]
- [xG: improving model performance — PLOS One 2023](https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0282295) — xG feature set. [academic, primary]
- [Spearman on Liverpool pitch-control — Training Ground Guru](https://archive.trainingground.guru/articles/william-spearman-how-liverpool-create-pitch-control). [talk, secondary]
- [xT vs VAEP — Van Roy et al.](https://tomdecroos.github.io/reports/xt_vs_vaep.pdf) — when they disagree. [academic, primary]
- [PPDA — Premier League](https://www.premierleague.com/en/news/4250153/passes-per-defensive-action-explained), [Wyscout Glossary](https://dataglossary.wyscout.com/ppda/). [secondary]
- [FiveThirtyEight SPI methodology](https://fivethirtyeight.com/methodology/how-our-club-soccer-predictions-work/) — xG + market value. [devblog, primary]
- [StatsBomb 360 Freeze Frames](https://blogarchive.statsbomb.com/news/statsbomb-360-freeze-frame-viewer-a-new-release-in-statsbomb-iq/) and [xPass 360](https://blogarchive.statsbomb.com/articles/soccer/xpass-360-upgrading-expected-pass-xpass-models/). [devblog, secondary]

## Models worth consuming

### Expected Goals (xG)
Closed-form logistic: `P(goal) = 1 / (1 + e^-(β₀ + Σ βᵢxᵢ))`. Public-model inputs: distance, angle subtended by goalmouth, body part, shot type (open play / set piece / penalty / counter), assist type (cross/cutback/through-ball), defender pressure. PSxG adds on-target end-location; non-shot xG values build-up positions. Distance+angle alone hits ~85% of full-model AUC — angle is load-bearing.

### Expected Threat / VAEP / Possession Value
- **xT** (Karun Singh): Bellman fixed-point on a 16×12 zone grid. For zone `(x,y)`:
  `xT[x,y] = s[x,y]·g[x,y]  +  m[x,y]·Σ_zw T[(x,y)→(z,w)] · xT[z,w]`
  with `s` = shoot freq, `g` = goal-conversion-from-shoot, `m` = move freq (`s+m=1`), `T` = move transition matrix. Iterate to convergence (~5 sweeps). Action value = `xT[dst] − xT[src]`.
- **VAEP** (Decroos et al.): GBM classifiers for `P_score` and `P_concede` over the next 10 actions, conditioned on previous 3. Action value = `ΔP_score − ΔP_concede`. More expressive than xT but **not closed-form** — needs a fitted XGBoost.

### Pitch Control (Spearman)
Per-point control probability via time-to-intercept:
`τᵢ = τ_react + dist/v_max + angular_penalty(θ)`
`p_arrive(t) = 1 / (1 + e^(-(t − τᵢ)/σ))` (logistic — heavier tail than Gaussian)
`P_team_control = 1 − Π_attackers(1 − p_arr,i)` integrated against ball-travel-time.

### Pressing triggers / counterpressing
- **PPDA**: `opp_passes_in_60%_of_pitch / (tackles + intercepts + challenges + fouls)`. High-press 4–8, mid-block 9–12, low-block 13+. Trivial.
- **Pressing Intensity** (Bauer et al., 2025): `P_press_on_carrier = 1 − Π_i (1 − p_arr,i,carrier)` — closed-form, reuses Spearman kinematics.
- **5-second rule** (Klopp/Pep): Bauer & Anzer 2021 confirm a ~5s post-loss press-density spike. Heuristic: ball-loss + `dist_nearest_teammate < r` + `t_since_loss < 5s` → switch BT to "press" subtree.

## Tractability filter (Q32 fixed-point, 60Hz)

> **Note (2026-05-13 reframe):** an earlier version of this section was framed around a "3000 LoC budget" that has since been retracted (see `docs/DESIGN_DOC.md` §1 "Scope ambition" and `docs/DECISIONS.md` 2026-05-13 LoC-retraction entry). The tractability table below remains accurate against the *determinism* constraint (no XGBoost, no float trees, no full-pitch per-pixel eval at 60Hz), but the rule-outs framed as "fits in N lines" or "LoC budget" should be re-read as "tractable in deterministic Q32 closed-form." VAEP stays ruled out — gradient-boosted trees over floats are non-deterministic across platforms, breaks Pillar 2's canonical-hash gate.

| Model | Tractable as-is? | Cheapest faithful approximation |
|---|---|---|
| **xG (logistic)** | Yes. ~6 fixed-point multiplies + a `sigmoid_q32` (LUT or Padé). Tiny. | Distance+angle 2-feature variant fits in <20 LoC. |
| **xT grid** | Yes. **Bake offline**, ship as a 192-entry `[Q32; 192]` LUT keyed by `(zone_x, zone_y)`. Pass-action value = LUT[dst] − LUT[src]. ~Zero runtime cost. | Already the cheapest form. |
| **VAEP** | **No.** XGBoost at 60Hz × 22 players is infeasible and non-deterministic across platforms (float trees). | Replace with xT delta + a small hand-coded context bonus (defender_pressure, recent_loss_flag). Honors the "score the action" lesson without ML. |
| **Spearman pitch control** | Partial. Per-pixel field for 22 players at 60Hz = budget killer. **Closed-form per-point query is cheap** (`τᵢ` + sigmoid per defender = ~22 mults). | Query pitch-control **only at decision points** (shot target, pass target, press anchor) — not as a full field. Use `cordic` for the trig in `angular_penalty`. |
| **Pressing Intensity (1−Π(1−p))** | Yes. Reuses the per-point pitch-control evaluator. | — |
| **PPDA** | Yes, trivially — accumulator over an event window. | — |
| **5-second window** | Yes. Single `Tick` delta vs. last-loss tick. | — |

## Direct application to Final Whistle T1-2b

- **Shooting uses xG as base utility:** **Yes.** Logistic xG over `(distance, angle, body_part, pressure)` is the shot-selection utility, a BT leaf evaluator. `pressure` = local pitch-control from defenders' side.
- **Passing uses xT/VAEP:** **Yes — xT only.** Bake the 192-zone xT LUT at content-pipeline time (deterministic, committed RON). Pass utility = `xT[dst] − xT[src] + α · P_completion`. Drop VAEP — GBMs violate the no-ML and determinism floor (gradient-boosted trees over floats are non-deterministic across platforms). The "score the action, not the outcome" lesson survives via xT-delta.
- **Pressing trigger uses real-world heuristic:** **Yes.** Compound: `ball_loss_event` AND `tick_since_loss < ticks_per_5s` AND `dist(self, ball) < press_radius` AND `team_press_intent == HIGH` → BT switches to `counterpress` subtree. PPDA is a derived match-stat for the UI, not a BT input. Klopp vs Pep baked as a team-tactic profile (heavy-metal vs. positional-recovery).
- **Positioning uses pitch-control:** **Partial — closed-form per-point only.** No full field. BT positioning evaluators query `time_to_arrive(self, target)` vs. nearest 2–3 opponents using Spearman's `τᵢ`. Cheaper fallback: Taki-Hasegawa velocity-weighted Voronoi (single nearest-defender check), ~5× cheaper.

## Open questions

- **xT bake provenance:** train on a public dataset (StatsBomb open data) or hand-author from first principles? Hand-authored avoids real-world licensed-data taint per CLAUDE.md §3; empirical-bake calibrates better. Likely hand-author for v1.
- **Pitch-control σ tuning:** logistic steepness governs how "deterministic" the sim feels. Coefficient lives in a design doc, not SPEC, per `design-docs/RULES.md §4`.
- **xG `pressure` input without 360 freeze-frames:** likely `nearest_defender_dist` + `defenders_in_cone(carrier→goal, 30°)` — both cheap from canonical state.
- **PSxG vs. pre-shot xG:** commit to **pre-shot xG** for BT decisions; PSxG is post-hoc commentary only.
- **Per-tactic press profile:** does `team_tactic.press_intensity` modulate only the trigger threshold, or also `press_radius` and the 5s window? Systems-designer call at balancing time.

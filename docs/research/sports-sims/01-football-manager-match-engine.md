# Football Manager match engine — research notes

**Researched:** 2026-05-13
**For:** Final Whistle T1-2b (22-player BT runner)

## Sources

- [Match Engine AI in FM21 — footballmanager.com](https://www.footballmanager.com/news/match-engine-ai-fm21) — primary SI devblog. Confirms the quarter-second "slice", mid-slice re-decision, attribute-gated action sets. [primary]
- [Match AI and Animation — footballmanager.com](https://www.footballmanager.com/features/match-ai-and-animation) — SI feature page. AI feeds animation selection (IK + selection criteria using incoming/outgoing speeds, velocity). [primary]
- [Tactics — FM24 manual, community.sports-interactive.com](https://community.sports-interactive.com/sigames-manual/football-manager-2024/tactics-r4960/) — SI's own manual: how team mentality, role, duty, individual instructions compose. [primary]
- [Dispelling 10 Common FM Misconceptions — footballmanager.com](https://www.footballmanager.com/guides/dispelling-10-common-football-manager-misconceptions) — Miles Jacobson quote: "hundreds of thousands of decisions… every match." [primary]
- [Miles Jacobson FFT interview](https://www.fourfourtwo.com/features/miles-jacobson-how-we-make-football-manager-future-and-where-you-come-it) — 5-year ME plan, sim-first philosophy. [primary]
- [TGS2025 — Jacobson on FM26 rebuild, GamerBraves](https://www.gamerbraves.com/tgs2025-studio-director-miles-jacobson-on-rebuilding-football-manager-26-from-the-ground-up/) — FM26 moved to Unity; legacy proprietary engine retired. [primary, paraphrased]
- [Attribute Weights — FMDataLab](https://www.fmdatalab.com/tutorials/attribute-weights) — community reverse-engineered per-role attribute weight tables. [secondary]
- [Passion4FM — Mentality Ladder, Duty, Fluidity](https://www.passion4fm.com/football-manager-mentality-ladder-player-duty-fluidity/) — team mentality cascades into per-player mentality bias. [secondary]
- [Hidden Attributes guide — Passion4FM](https://www.passion4fm.com/football-manager-guide-to-hidden-attributes/) — catalogue of ~16 hidden attributes (Consistency, Important Matches, Dirtiness, Pressure, Adaptability, etc.). [secondary]
- [Quora "How does the FM algorithm work?"](https://www.quora.com/How-does-the-algorithm-of-the-game-Football-Manager-work) — anecdotal but consistent with primary sources. [secondary]

## Key findings

- **Tick granularity is 4 Hz** — players + officials make one decision per "slice" = 0.25 s of in-match time. Animation/positional interpolation runs faster; AI decisioning does not. Confirmed in the FM21 devblog (primary).
- **FM21 added mid-slice re-decision** — a player can abandon their planned action inside the same slice if context changes. So it's not pure tick-blocking; there's a reactive interrupt layer.
- **~50 visible attributes + ~16 hidden attributes per player.** Visible split: Technical (~14), Mental (~14), Physical (~8), plus GK-specific. Hidden include Consistency, Important Matches, Pressure, Dirtiness, Versatility, Adaptability, Injury Proneness, Ambition, Loyalty, etc. CA is a *derived* weighted sum, not an input.
- **Attributes are weighted per role**, not globally. Each role (e.g. "Inverted Wing-Back — Support") has its own weight vector; the same player has different role-suitability scores. Community has reverse-engineered the weight tables ([FMDataLab]). Decisions, Acceleration, Agility weight heavily across nearly all roles.
- **Tactic composition is hierarchical:** team mentality → role + duty (per slot) → individual instructions (per player) → in-slice decision. Mentality biases a "ladder" of risk; duty biases attack/defend leaning; the role shapes the *manner* of approach; player instructions tune within that. The match engine resolves all four layers into per-slice action weights.
- **Attributes gate which actions are even available.** SI: "shot selection and spatial awareness are tied into the player's attributes so more skilful players will have a wider variety of shots at their disposal." Low-skill players have a smaller action menu, not just worse rolls — important architecturally.
- **No public confirmation of BT vs FSM vs utility AI.** SI uses the words "decision" and "evaluation" but never names the structure. The mid-slice re-decision behaviour reads more like *utility AI* (per-slice scoring with a re-eval trigger) than a strict BT, but this is informed inference, not a primary quote.
- **FM26 = Unity rewrite** of the renderer + animation pipeline; the AI/decision layer was carried forward and extended, not rewritten from scratch. FM24's "engine changes" were ball-physics + locomotion only — the famed full rewrite was always slated for FM25/FM26.
- **Animation is driven by AI output, not vice versa.** AI decides "pass left foot, low, 18m"; animation selection criteria (incoming/outgoing speed + velocity + IK) pick the clip. Decision layer is engine-shell-agnostic.
- **"Hundreds of thousands of decisions per match"** (Jacobson). 90 min × 60 s × 4 Hz × 23 actors ≈ 1.24M decision points. Matches that order of magnitude — supports the 4 Hz reading.

## Direct application to Final Whistle T1-2b

Our 60 Hz tick rate is far finer-grained than FM's 4 Hz decision cadence. That's a deliberate choice for ball physics + signature-move triggering, but **we should not be running a full BT decision pass on every tick for every player — that's 22×60 = 1320 BT evaluations/sec, way over budget for ~3000 LoC**. FM's two-layer split (slow decision cadence + reactive interrupt) is the architecture to copy.

- We should run BT root re-evaluation on a **4–10 Hz cadence per player** (~6–15 ticks at 60 Hz), staggered across players to spread cost. Between decisions, players execute the *current action plan* (move toward target, prepare shot) tick-by-tick.
- We should add a **reactive interrupt** (cheap predicate check every tick) — ball state change, "I'm now closest to ball", got tackled — that forces an early BT re-eval. This matches FM21's mid-slice re-decision and keeps the sim responsive without 60 Hz BT churn.
- We should structure player attributes as **typed Q32 vectors with per-role weight tables** baked at content-load. Role suitability = dot product. This lets us evolve the role catalogue in content packs without touching `fw-match-sim`.
- We should make **available-action sets attribute-gated**, not just success-probability-modulated. A low-Composure player simply does not have "shoot from 25m under pressure" in their BT action menu. This is cheaper than rolling and discarding, and gives us text-commentary hooks ("never even considered shooting").
- Informational only: SI's hierarchical tactics (team → role/duty → player) maps cleanly onto our planned BT blackboard. We can implement it as three stacked context layers the BT reads from, not three different runners.

## Open questions

- **Exact data structure** SI uses (BT? utility AI? hierarchical FSM?) is not publicly stated. Worth a targeted look at the few SI dev talks (Tom Markiewicz, Paul Collyer have given the odd conference talk) if we want stronger grounding.
- **How tactical familiarity** (a known FM mechanic — squads "learn" tactics over weeks) modulates per-slice decision quality. Useful for our team-cohesion model.
- **AI manager substitution + in-match tactical-change logic** — separate from on-pitch player AI; not covered here. Queue for a follow-up note when we reach T2 manager AI.
- **Cross-platform determinism** — FM is not deterministic across platforms (different RNG, multi-threaded ME). Confirms we're solving a harder problem than they are; no prior art to lift here.

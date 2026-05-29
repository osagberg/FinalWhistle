---
description: Stable design contract for Final Whistle (Rust + Tauri pivot). Pairs with MASTER_PLAN.md for delivery order. Cross-refs to forthcoming design/* sub-docs.
last_verified: 2026-05-29
---

# Final Whistle — Design Doc v1

> Stable design contract. Last updated: 2026-05-13.
> Canonical pair: `docs/DESIGN_DOC.md` + `docs/MASTER_PLAN.md`.
>
> This doc defines the game. `MASTER_PLAN.md` defines delivery order.

---

## 1. Product Definition

### Genre
Deep football management simulation with RPG-depth career memory, set in a procedurally generated fantasy football world.

### Core Promise
A football manager where the world has memory and players have specific, expressive identities — and every save is a different world.

### Player Fantasy
"I run a club inside a living fictional pyramid. The kid I cut becomes someone else's captain. The derby I threw is what the fans still throw at me four seasons later. My striker's late-bloomer breakthrough in the cup final is a story only my save has."

### Commercial Positioning
- Premium one-time purchase: $20 Early Access → $30 1.0.
- Steam-first, Mac + Windows + Linux native (Mac-first development).
- FM-disillusioned + Crusader Kings emergent-stories + Caves of Qud procedural-depth crossover.
- Update cadence is whatever the project earns through quality, not a hardcoded timetable.

### Scope ambition
Final Whistle is targeted to **match or exceed Football Manager on simulation depth**, in a procedurally-generated world rather than a licensed one. Implementation scope is bounded by the **non-negotiables** below (determinism contract, no-runtime-LLM, text-first surface, procedural-fantasy worldbuilding), not by LoC counts or developer-hours. Per-system depth grows until it credibly delivers the pillar's promise — match engine, memory ledger, scouting model, breakthrough triggers, and signature catalogue are all unbounded by line count.

What this means in practice:
- **Match engine**: a real physical simulation (positions, physics, per-player AI) at a fidelity competitive with FM's match engine — not a stat-event sampler. Expected to run into tens of thousands of LoC across multiple subsystems; that's the right shape, not bloat.
- **Player model**: rich attribute system (visible + hidden + personality bias vector + form/morale/fatigue + chemistry + memories) with whatever attribute count the design pillars need. The 24+8 number in earlier scoping was a research-paper recommendation, not a cap. Bumping to FM-scale (~56) or beyond is on the table.
- **Subsystems**: psychology, chemistry, coach AI, referee model, training, scouting, transfer market, media, board relations — each as a fully-realized module when its phase lands, not "scaffolded stubs that maybe expand later."
- **Signature moves**: the 24 in the design doc is the initial catalogue. The architecture supports unbounded growth via content packs.
- **AI architecture**: behavior trees + utility scoring + influence maps + steering + personality bias vector compose as planned. FSM per-role state taxonomies (a prior-art alternative surveyed during T1 design research) are also viable now that we're not budget-constrained — the choice is about clarity vs composability, not bytes.

What this does NOT lift:
- **Determinism contract** — cross-OS BLAKE3 canonical-hash regression, Q32.32 fixed-point in canonical state, BTreeMap-only in sim crates, `ChaCha8Rng` seeded via the canonical `seed_fn(match_seed, tick, layer, site)` (ADR-0009), no `tokio`/`async` in `fw-match-sim` or `fw-memory`. These are pillars.
- **No runtime LLM** — bake-time only. Architectural choice.
- **Procedural fantasy only** — no real licensed data, ever.
- **Text-first shipping surface** — no 3D viewer in the shipped game. The 2D tactical board is the dev verification surface AND the shipped match-day surface.
- **Maintainability under Claude+human** — code is structured for an LLM+human pair to navigate, with strict layering, named subsystems, and clear ownership per agent role.

### Target Audience
- **Primary:** FM-disillusioned 25-45 looking for a different football-management fantasy.
- **Secondary:** OOTP / Crusader Kings / Caves of Qud players comfortable with dense systems + emergent narrative.
- **Not for:** FIFA / eFootball arcade-football players, licensed-club purists, 15-minute-session casuals.
- **Age rating target:** PEGI 12 / ESRB T.

---

## 2. Non-Negotiable Rules

1. **No real-world licensed players, clubs, leagues, kits, or competitions.** The procedural fantasy world IS the USP; licensing would dilute it and waste the LLM advantage.
2. **Deterministic simulation with a pinned replay corpus.** Same seed → identical canonical state hash on Mac + Windows + Linux. Every commit re-verifies the corpus.
3. **No runtime LLM calls.** All generated text (names, biographies, scout prose, headlines, manager quotes, fan reactions) is baked at content-pack build time into structured artifacts. Runtime sampling is deterministic.
4. **Text-first presentation.** Match-day surface is a 2D tactical board + dense football-native commentary + live tabular numbers. No 3D viewer, no manga-broadcast cinematic mode.
5. **Fixed-point canonical state in the sim.** Floats are forbidden in canonical match state and event ledger. Q32.32 default; viewer-side interpolation may use floats.
6. **Event-sourced career memory is append-only.** No retroactive mutation of ledger entries. Supersession via new events that cite prior ones.
7. **Football-native UI vocabulary.** No capitalized mystical state-nouns ("The Hush", "Calling", "Canon", "Awakened", "+5 Finishing"). Commentary-grade copy only.
8. **Mod-friendly data layer from day one.** Content packs use RON files with stable content-pack-qualified IDs. Editor UX may ship later; data shape never blocks modding.
9. **Solo-dev scope discipline.** Every proposed feature lives in exactly one of: Product MVP, Architecture-from-day-one, Dev pipeline, Deferred. Features outside those buckets get cut, not parked.
10. **No multiplayer, no server-side anything, no live ops.** Single-player premium product.

---

## 3. Design Pillars

Five pillars define what makes Final Whistle Final Whistle. A feature that doesn't serve at least one pillar is a candidate for cutting.

### Pillar 1 — Procedural Fantasy World (the LLM USP)
**Intent:** Every save is a different world. Bake-time content compilation generates the entire football universe — nations with credible cultural priors, six-tier pyramids, ~96 clubs with histories and rivalries, ~2000-2400 players each with an Identity Packet, manager archetypes, scout personalities, press outlets, fan cultures. **Expression in gameplay:** start a save and the league table, the legends list, the regional rivalries, the player names, the club crests, the press tone, and the cultural quirks are all unique. No two saves share the same Welsh-coded second-tier title-chaser. This is the moat FM structurally cannot copy and the lane only an LLM-friendly content pipeline can occupy.

### Pillar 2 — Careers That Remember
**Intent:** A single append-only event ledger records every consequential moment (cup finals, broken promises, derbies thrown, youth sold, breakthrough goals). Years later, readers surface those events as press callbacks, fan chants, rival recall, NPC dialog, returning antagonists. **Expression in gameplay:** the kid you cut becomes another club's captain and faces you in the cup final eight seasons later, and the commentary, the pre-match board, and the rival manager's press quote all reference it specifically — not via a templated "former player returns" generic, but with the actual ledger event slotted into the dialog. This is the retention hook.

### Pillar 3 — Breakthrough-Driven Development
**Intent:** Players grow because of what happened to them, not because XP accumulated. Each player carries latent affinities + narrative flags (`late_bloomer`, `flow_access`, `peak_ceiling_high`, regressive `fragile` / `confidence_fractured` / `ceiling_compressed`). Career events accumulate `signature_readiness ∈ [0,1]`. At threshold + matching match situation, a development event fires — surfaced as text recap with football-grade phrasing ("Something clicked today. Scouts will revise."). Permanent. Irreversible. Equal gravity for regressive triggers. **Expression in gameplay:** development is rare, earned, and memorable. A career may yield 1-3 breakthroughs total. Manager influence is indirect: tactics, selection, training, promises, pressure exposure — never mid-match QTEs.

### Pillar 4 — Scouting Uncertainty
**Intent:** The strategic loop is built on the gap between hidden ground truth (the Identity Packet's internal gene snapshot) and observed knowledge (scout reports, match observations, career outcomes). Scouts are biased observers with archetypes (`physical_profiler`, `technical_purist`, `regional_expert`, ...), regional familiarity, blind spots, and accumulating track records. Reports disagree. The player triangulates truth across multiple scouts over time. **Expression in gameplay:** signing a player is a bet against partial information. A late-bloomer midfielder looks like a slow winger to one scout and an underrated passer to another; you weight each scout by their accumulated track record on similar archetypes; you watch a match yourself; you decide. Truth emerges over seasons, not in one tooltip.

### Pillar 5 — Signature Identity
**Intent:** Every meaningful player has 1-3 readable on-pitch signature moves that express identity beyond stat numbers. 24 pre-authored signatures, 3 per role family across 8 role families (Goalkeeper, Centre-back, Full-back, Defensive midfielder, Central midfielder, Attacking midfielder, Winger, Striker). Each signature is football-readable: "Looks for early crosses", "Arrives late in the box", "Cuts inside onto his stronger foot". Each carries trigger conditions, a sim bias, a presentation recipe in the text recap, and authored counterplay. Earned via career accumulation, not stat-assigned. **Expression in gameplay:** your striker isn't "Finishing 17 / Composure 16" — he's the lad who runs the blind-side near-post curl and finishes first-time. Opposition tactics counter the signature, not the stat line.

---

## 4. Core Loop

### First 30 seconds
Pick a starter club from the six-tier procedural pyramid. Quickstart highlights notable archetypes (decaying-giant-tier-2 / rising-academy-tier-3 / backs-against-the-wall-tier-5 / etc.). See club crest, squad list, opening-fixture press question.

### First 10 minutes
First match-day. Pick formation + starting XI from the tactical board. Hit "Play match." Match-day surface: 2D tactical-board top-down view, live numerical state (score, possession, momentum, key events), continuous football-native commentary text. At full-time, a structured post-match report surfaces: stand-out performance, a press quote, one event written to the memory ledger. Tutorial layer is woven into the surface — no blocking tooltip wall.

### First hour
Run a season cycle: scout players, evaluate disagreeing reports, sign or pass, set tactics, play matches (tactical-board or auto-sim), respond to press, manage contract renewals, navigate the transfer window. Salience-gated narrative events surface 5-8 times per season — they're moments the ledger will remember.

### Hour 10+
Multi-season career. Players you trained reach breakthroughs. Players you sold come back as opponents. The kid you broke a promise to is now your rival's captain. The world has accumulated its own specific history. Promotion/relegation pressure compounds across seasons. Youth intake feeds the next generation; ledger-tracked alumni create lineage flavor in scout reports and commentary.

### Advancement state
- **Per match:** sim ticks 60Hz; pinned-hash replay corpus regression-tested on every commit.
- **Per season:** league standings, cup progression, transfer windows, contract cycles, board confidence.
- **Per career:** ledger grows monotonically; compaction at 5-season boundary (hot log for recent; summarized state for older — preserves callback eligibility, drops tick granularity).

---

## 5. Match Simulation Layer

### What's simulated (canonical state)
- 22 players, 11-vs-11, 60Hz fixed-timestep simulation.
- Position, velocity, role, behavior-tree node, fatigue, ball physics state — all in Q32.32 fixed-point.
- Custom deterministic ball physics: gravity, linear air drag, Magnus force, ground bounce, rolling friction, semi-implicit Euler integrator. NOT a third-party physics engine.
- Behavior-tree-driven role assignments per manager archetype (YAML-authored: `direct-pressing`, `low-block-counter`, ... 20-30 archetypes at MVP).
- Steering-target movement: BT outputs `desired_position` + `desired_speed`; deterministic actuator applies accel / decel / turn-rate / max-speed caps.
- Signature bias layer: active signatures modulate field-level decisions per the field's stacking policy (additive / additive-with-diminishing-returns + hard caps).
- Match-event emission to memory ledger (goals, breakthroughs, milestones, derbies, upsets).

### What's NOT simulated
- 3D player models, real-time animation, free camera, broadcast cinematic shots. The viewer is a 2D tactical board.
- Stadium audio mixing, weather visual effects, crowd choreography (these may exist as text/UI flavor; no rendering cost).
- Networked play of any kind.
- Out-of-match player simulation at sub-day granularity (training is daily-tick, not minute-by-minute).

### Determinism contract
- **Pinned canonical-state hash regression corpus.** Every commit runs the corpus (`scripts/fw verify`) on Mac + Windows + Linux via CI. Drift on any platform fails merge.
- **Fixed-point Q32.32 everywhere in canonical state.** No `f64` / `f32` in the sim's authoritative path.
- **Deterministic RNG:** `ChaCha8Rng` seeded via the canonical `seed_fn(match_seed, tick, layer, site)` (ADR-0009). Layers are non-overlapping (`Decision` / `UtilityTieBreak` / `ReactiveInterrupt` / `BallPhysics` / `SignatureTrigger` / `MemoryEvent` / `ScoutObservation` / `ContentBake`); `site` disambiguates within a layer. No `HashMap` iteration in sim paths; deterministic containers (`BTreeMap` / `BTreeSet` / `Vec`) only.
- **Replay seed per match:** every match carries `match_seed: u64`; replay reconstructs frame-identical canonical state.

### Presentation contract
- **Text recap (primary).** Football-native commentary, no menu vocabulary. Three text tiers per moment: live ticker, post-action callout, post-match report.
- **2D tactical board (secondary).** Top-down pitch, dotted lines for off-ball runs, player markers with role + ID overlay, optional heat-map / pressure-map toggles. Built in PixiJS in the SolidJS frontend.
- **Live numerical surface.** Score, possession %, xG, momentum, pressure, key-event count — dense tabular UI, always visible.
- **Two pacing modes:** real-time-with-pause (active management) and auto-sim-to-event (skip to next salience-band-N event).

---

## 6. Content Pipeline

The LLM USP runs at bake-time, not runtime. The entire procedural world is compiled to versioned content packs before the game ever ships a build.

### Bake-time corpus
LLM-authored at content-compiler build time, reviewed, committed as structured RON:
- **Player names** per cultural region (cohort-shaped: 60% native, 30% cross-region, 10% foreign).
- **Club names + crests + colors** per pyramid tier + regional culture.
- **Player biographies** — short factual summaries used as scout-report flavor.
- **Scout-report prose templates** keyed by archetype + observed phenotype label.
- **News headline templates** per event class (breakthrough goal / sacking / derby result / upset / contract drama).
- **Manager quote templates** keyed by archetype + outcome class.
- **Fan reaction templates** per fan-base mood + recent result.
- **Commentary phrase banks** per match phase / event class / signature execution.

Every artifact is reviewed, linted (no real-player names, no real club names, PEGI-12 safe vocabulary, no banned-state nouns), and committed. The committed RON IS the source of truth — not the prompt, not the model. Regeneration produces a new delta pack with bumped version.

### Runtime sampling
At runtime: deterministic. No LLM calls.
- **Naming:** Markov-chain name generators seeded by region + cohort.
- **Template slot-filling:** Tracery-style grammars with `ChaCha8Rng` keyed by `(career_id, event_id)`.
- **Phrase variation:** weighted sampling from the committed phrase banks.

Manifest metadata records which model + seed + prompt-hash produced each pack (audit trail). Save files reference pack IDs + versions; missing-pack fallback uses the base pack's placeholder generator with a UI badge.

---

## 7. Career / Memory Layer

### Event-sourced ledger
Single append-only ledger of structured `MemoryEvent` records. Every meaningful moment emits exactly one record at emission time. Records are immutable. Schema is versioned (`schema_version: u16`); load-time migration handles upgrades.

```
MemoryEvent {
  event_id, match_id?, season, tick: Option<Tick>, career_date
  emitter: { kind, source_id }
  participants: [{ role, entity_id }]
  what: EventClass
  stakes: Q32             // in [0, 1] semantically; canonical Q32.32 fixed-point
  emotion: enum
  consequence: [{ kind, delta }]
  callback_eligibility: { recall_after_seasons, recall_tags, expires_after_seasons? }
  salience: Q32           // in [0, 1] semantically; computed at emission; canonical Q32
  schema_version: u16
}
```

> **Determinism note (per Codex pre-T0 audit + `Sim/RULES.md` §1, ADR-0005):** `stakes` and `salience` are `Q32` in canonical state. No u16 / f32 / f64 representation in `fw-memory`. The `[0, 1]` notation is conceptual (the value range); the wire type is the underlying `Q32` (`fixed::FixedI64<U32>`). ADR-0005 supersedes earlier drafts that implied a u16-basis-points representation.

### Salience scoring (formula locked; weights are tuning seeds)
```
salience = clamp(
    0.4 * stakes
  + 0.2 * participant_prominence_avg
  + 0.2 * event_class_base_weight
  + 0.1 * rivalry_boost
  + 0.1 * rarity_boost,
  0, 1)
```
Band cutoffs (tuning seeds): `debug-only < 0.3 ≤ routine < 0.6 ≤ notable < 0.85 ≤ season-defining`. Surfacing-time salience adds reader-side modifiers (callback age, player attention).

### Readers (queries, not stores)
1. **Alumni DB** — "any player on the opponent who ever played for us?" Surfaces pre-match overlay + commentary callback.
2. **Rival recall** — "prior high-salience scoreline / event between these clubs?"
3. **Promise tracking** — "all PromiseMade events with player X?" Surfaces in contract UI; broken promises emit `BrokenPromise`.
4. **Big-match scars** — "high-stakes match upcoming — last 1-2 high-salience events at this stake-class?"
5. **Press / fan callbacks** — "recent events relevant to this press conference / fan sentiment surface?"

### Surface mechanisms
Press conferences, fan sentiment polls, board summaries, scout-report flavor, in-match commentary, post-match recap, NPC dialog templates — every player-facing surface that consumes the ledger goes through Tracery-style slot-filling against the committed prose banks. The ledger event provides the structured slots; the template provides the language.

### Compaction
At 5-season boundary, older events compact to summarized state (preserves callback eligibility, drops tick-level granularity). Hot log holds recent seasons. Ceiling: 5-8 player-facing-band events per season — enforced by salience threshold, not by authoring quota.

---

## 8. Scope Discipline — What's IN and OUT

### MVP scope

The following are confirmed IN for EA / MVP launch (2026-05-29 decision — see DECISIONS.md):

- Single procedurally generated nation, one six-tier pyramid (~96 clubs), ~2000-2400 procedural players — full LLM-baked content pipeline.
- 22-player matches, BT-driven, deterministic sim, pinned-hash corpus.
- 2D tactical-board match viewer + text commentary + live numbers.
- Formation + starting XI selection. Auto-sim or follow-along pacing.
- Basic scouting with single-scout-report uncertainty (Scout Disagreement is conditional — Month-4 feel-prototype gate per §13 OQ3).
- League fixtures + one cup competition + promotion/relegation.
- Transfer window + contracts (basic).
- Append-only event ledger emitting from match events.
- 24 authored signatures (3 per role family × 8 role families; ≥8 with implemented predicates at EA, all 24 authored).
- Multi-season careers with ledger compaction at 5-season boundary.
- 5-8 salience-gated narrative events per season surfacing via press / fan / NPC templates.
- Breakthrough development triggers (3 kinds: signature awakening, latent-flag unlock, regressive collapse) — text-recap presentation.
- 20-30 manager archetypes (BT-authored), rival-manager ecosystem.
- Save / load with schema versioning + content-pack-qualified IDs.

### IN — Phase 1-2 (vertical slice through first playable season)
- 22-player matches, BT-driven, deterministic sim, pinned-hash corpus.
- 2D tactical-board match viewer + text commentary + live numbers.
- Formation + starting XI selection. Auto-sim or follow-along pacing.
- Basic scouting with single-scout-report uncertainty (Scout Disagreement is conditional — Phase 3 feel-prototype gate).
- League fixtures + one cup competition + promotion/relegation between bottom of pyramid and your assigned tier.
- Transfer window + contracts (basic).
- Append-only event ledger emitting from match events.

### IN — Phase 3+ (MVP / EA launch)
- 24 authored signatures (3 per role family × 8 role families).
- Multi-season careers with ledger compaction at 5-season boundary.
- 5-8 salience-gated narrative events per season surfacing via press / fan / NPC templates.
- Breakthrough development triggers (3 kinds: signature awakening, latent-flag unlock, regressive collapse) — text-recap presentation.
- Scout Disagreement (if Month-4 prototype gate passes) — 5-8 scout archetypes, accumulating track records.
- 20-30 manager archetypes (BT-authored), rival-manager ecosystem.
- Save / load with schema versioning + content-pack-qualified IDs.

### OUT — Phase 1-2
- Tactical instructions beyond formation + starting XI (no player roles, no individual instructions, no set-piece routines).
- Set pieces (corners / free kicks / penalties as scripted-outcome stubs only).
- Youth academy (data seeded for Phase 3+; surfacing later).
- Finances, sponsors, board interactions, stadium expansion.
- International football, national teams, World Cup.

### OUT — Permanently
- Real-world licensed players, clubs, leagues, kits, competitions.
- 3D match viewer (any form — including iso, broadcast, behind-camera).
- Manga / anime cinematic presentation layer (dropped from the original Unity FW).
- Multiplayer of any kind (LAN, online, hot-seat, asynchronous).
- Mobile port.
- Runtime LLM calls.
- Live-ops, server-side anything, social-media integration.

---

## 9. Visual Identity

### Surface aesthetic
Text-first, dense, professional, slightly opinionated. The reference is FM-class information density but with the discipline of a modern web product — typographic hierarchy, scannable tables, accessible color, no over-clicked navigation. Not chrome-heavy "futuristic UI." Not low-fi retro. Clean.

### Color / type stack (initial seeds — Phase 1 tuning)
- **Display:** Anton (compressed condensed) for headers + scoreline.
- **Data:** JetBrains Mono for numerical tables, replay hashes, technical surfaces.
- **Body:** Inter or Rajdhani for press / commentary / NPC dialog.
- **Palette:** muted-pitch-green primary, accent-flag yellow + red for state, neutral charcoal text on warm-off-white background. Dark mode is a first-class theme, not a recolor.
- **Slight anime influence permitted** in iconography weight + impact-frame post-match cards. NOT in match-day rendering. The Unity-era manga-broadcast aesthetic is dropped.

### Match-day surface
2D top-down tactical board (PixiJS):
- Pitch with markings to FIFA-spec dimensions, rendered as flat vector.
- 22 player dots, kit-color-coded, identity-overlay shows shirt number + role badge.
- Ball as distinguishable marker.
- Live arrows for active runs / pass intent (toggleable layer).
- Heat-map / pressure-map toggles (off by default; power-user feature).
- Commentary panel right side; numerical surface top bar; tactical controls bottom bar.

### Out-of-match surface
HTML/CSS dense management UI. Player cards with phenotype labels (NOT raw genes), signature list, contract block, development bar. Squad screen, league table, fixture list, transfer center, scouting board, press inbox — all tabular, all scannable, all keyboard-navigable.

---

## 10. USPs vs FM

| Axis | FM | Final Whistle |
|---|---|---|
| **World** | Licensed real-world database; one shared reality | Procedural fantasy; every save a different world |
| **Memory** | Stat history; some news; no cross-decade callbacks | Event-sourced ledger; surface in dialog years later |
| **Player identity** | 20+ numeric attributes; "Personality" string | 24 readable signatures + Identity Packet + breakthroughs |
| **Scouting** | Fog-over-numbers (single source, hidden true values) | Disagreeing biased scouts with accumulating track records |
| **Cadence** | One annual full-price release | Solo-dev continuous patching during EA → 1.0 |
| **Platform parity** | PC parity; Mac historically lagging | Mac-first dev; Steam Deck Verified at launch via Linux build |
| **Moddability** | SI's tools + licensed-data constraints | RON content packs, stable IDs, no licensing constraints |
| **Visual** | 3D match engine (resource-heavy) | 2D tactical board + text (lower spec, higher pace) |

The bet is that FM's depth-promise can be matched by a focused solo dev IF the licensing weight is replaced by procedural generation, the 3D engine is replaced by text + tactical board, and the memory + signature systems do the work that "personality strings" and "news items" do badly today.

---

## 11. Architecture overview (ADR index)

This is the one-page view of how the engine composes. Detail is in the ADRs under `docs/adr/`; this section is the index + narrative. Section authored 2026-05-13, after the pre-T1 research + reframe wave.

### 11.1 Match engine (ADR-0001 + ADR-0006)

Seven layers, all deterministic, composed top-down:

1. **Team tactic FSM** — 5 coarse states (HIGH_PRESS / MID_BLOCK / LOW_BLOCK / COUNTER_ATTACK / SET_PIECE). Event-driven plus a 2Hz heartbeat. Parameterizes every layer below.
2. **Per-player decision runner, staggered at 4Hz** — Hybrid FSM-of-Behavior-Trees. Outfield roles each have ~6-10 coarse role states; each state owns a small BT (~10-30 leaves). Goalkeeper is pure FSM. Universal pre-emption hooks at dispatcher level. Nodes are Rust; trees are content-pack RON. Cadence amended 2026-05-13 per Codex full-project audit Tranche 3 — 60 Hz / 4 Hz = 15-tick window per player, clean math + FM-baseline cadence.
3. **Utility-scored selector nodes** inside the BTs, firing at on-ball decision points (pass / shoot / dribble / hold). Scored by xG / xT / pitch-control math.
4. **Personality bias vector** (the full 14-element PersonalityVector from ADR-0002 — 8 elements drive the match-tick mapping table in ADR-0003 §5; the remaining 6 carry over into longer-tail systems) multiplies into utility scores per consideration. Updated 2026-05-13 to mirror ADR-0002's 14-field model (was incorrectly "8 of the 17 hidden attributes").
5. **Influence maps** (danger / support / space) on a 32×24 grid, regenerated at 8Hz — **independent cadence** from the 4 Hz decision runner. Decisions sample either fresh maps or 1-tick-stale maps (since 60/8 = 7.5 ticks per regen vs. 60/4 = 15 ticks per decision). Players consume the maps for off-ball positioning — never reason about 21 other agents directly.
6. **Reactive interrupts at 60Hz** — cheap predicates (ball-state changed, marker arrived, shot incoming) can preempt mid-decision. This is the responsiveness layer; the 4 Hz decision cadence is the steady-state layer.
7. **Reynolds-style steering at 60Hz** turns intent into Q32 locomotion. Pure arithmetic.

The utility math is closed-form throughout (xG = 6-feature sigmoid LUT, xT = 192-entry Q32 LUT from a baked Bellman fixed-point, pitch-control = Spearman closed-form per-decision-point, pressing = Bauer-and-Anzer 5-second rule + intensity formula). All coefficients hand-authored — no StatsBomb-fit, no XGBoost, nothing that breaks pillar 1 or the determinism contract. Detail: ADR-0003.

### 11.2 Player model (ADR-0002)

55 attributes per player. 38 visible (14 technical / 10 mental / 8 physical / 6 GK-specific, all on a flat struct so BT decision sites never branch on role) + 17 hidden (14 personality + 3 durability). The personality vector is the multiplicative bias on utility scores from §11.1.

All values `Q32` in `[0, 1]`; UI projects to 1-20 at the DTO boundary per ADR-0004. CA is derived, PA is stored; PA is mutable only via `MemoryEvent::Breakthrough` (which is what makes pillar 3 architecturally real, not a bolt-on). Five condition layers on a separate `PlayerCondition` struct: form / morale / match_fitness / sharpness / signature_readiness. Role-affinity weights live in content-pack RON, not Rust. Scout uncertainty (pillar 4) is FOF-style ranges keyed to scout skill.

### 11.3 Memory ledger + breakthroughs (ADR-0005)

Append-only `Vec<MemoryEvent>` per career, indexed via `BTreeMap<CareerId, _>`. Schema-versioned. 28 event classes at schema_version=1. Stakes and salience are `Q32`, not the f32 the earlier DESIGN_DOC §7 sketch implied. Three decay functions (Never / Linear / Exponential), applied at read-time projection only — the ledger itself is never mutated.

Five readers project the ledger into context: `SalienceReader`, `PressReader`, `FanReader`, `ScoutReader`, `CoachReader`. Each runs at its own cadence (per-event for press; per-tick for salience; per-action for scout).

Breakthroughs require all three of: `signature_readiness` threshold reached, a narrative-gating event firing, and a per-attribute-family cooldown elapsed. PA redraws upward in the family with a partial CA lift. Regressive collapse is the symmetric mechanism (`regressive_pressure` ticker; PA redraws downward, bounded by a career-floor; reversible only via subsequent breakthroughs in the same family). Mod-compatible via an `UnknownEventClass` opaque-bytes variant that still participates in the canonical BLAKE3 hash but is ignored by core readers.

### 11.4 IPC surface (ADR-0004)

5-command quintet for live match: `start_live_match` (returns an opaque `MatchHandle`), `step_live_match(handle, ticks)`, `get_match_snapshot(handle)`, `apply_match_command(handle, MatchCommand)`, `finish_live_match(handle)`. Handle-based dispatch supports concurrent matches at T2-5.

`MatchCommand` is a closed-set 9-variant enum: Substitute, ChangeFormation, ChangePressLevel, ChangeTempoBias, SetCornerTaker, SetFreeKickTaker, SetPenaltyTaker, SetCaptain, TeamTalk. Typed enums, no f64 sliders. Applied at tick boundaries.

Frame streaming: Tauri events (`match:frame`) at ~10Hz; PixiJS interpolates to 30fps. Compact fixed-size `[PlayerPosDTO; 22]` array. Scoreboard panels stay on 1Hz polling. Diagnostic mode is a parallel `match:diagnostic-frame` event channel gated by a `fw-tauri/diagnostic` feature flag + runtime toggle, carrying tagged decision reasons + BT-trace tails.

Async on the IPC quintet only; sim crates stay sync (firewall). TS types hand-mirrored at `frontend/src/lib/types.ts`, drift caught by an `insta`-snapshot smoke test in `crates/fw-tauri/tests/dto_schema_smoke.rs`. Revisit `ts-rs` / `specta` at T4.

### 11.5 Dev verification (ADR-0007 + ADR-0008)

Three layers, accepted via the design doc at `docs/design/dev-verification.md` and now formalized:

- **Layer 1 — Diagnostic commentary** (T1-4): rich event-by-event log with position + decision context. Surfaces brain-dead behavior from text alone.
- **Layer 2 — Dev-tier 2D tactical board** (T1-2a): minimal PixiJS top-down viewer with dots + ball + tick scrubber. Always-on for dev. Same canvas as the shipped match-day surface gets at T4 polish. **Per ADR-0008** the board reads frames from either Tauri IPC (default) or a JSON fixture (browser-dev mode, selected by URL param), which lets Claude Preview MCP drive visual verification without a Tauri runtime — unlocking AI-paced "is this football?" iteration.
- **Layer 3 — Behavioral proptest invariants + pair-seed comparison tests** (T1-9): "GK within 30m of own goal 95%+ of ticks", "team width 35-65m during in-possession", "pair-seed tactic-flip changes possession in expected direction", etc.

The canonical-hash regression test (cross-OS BLAKE3 pinned per scenario) sits orthogonal as bedrock. Two T2 candidates flagged: OOTP-style stat-distribution CI gate, EHM-style two-engine cross-check (lean Dixon-Coles as calibration reference).

### 11.6 Cross-cutting infrastructure (ADR-0009 + ADR-0010 + ADR-0011 + ADR-0012 + ADR-0013 + ADR-0014 + ADR-0015)

Seven additional ADRs added 2026-05-13 in response to the Codex full-project audit:

- **ADR-0009 — RNG seed derivation.** Single canonical `seed_fn(match_seed, tick, layer, site) -> u64` (BLAKE3 over 17-byte buffer, truncated). Eight `SeedLayer` discriminants (`Decision` / `UtilityTieBreak` / `ReactiveInterrupt` / `BallPhysics` / `SignatureTrigger` / `MemoryEvent` / `ScoutObservation` / `ContentBake`). Resolves drift across 4 docs that previously cited different seed tuples.
- **ADR-0010 — Save format.** bincode 2 + zstd + forward-only migrations. Magic `"FWS1"` + u32 version prefix uncompressed for cheap version-check. Mod-load fingerprint via BLAKE3 of sorted (mod_id, mod_version) pairs.
- **ADR-0011 — Signature system.** 24-signature catalogue (3 × 8 role families). `SignatureDefinition { trigger, bias_snapshot, presentation, cooldown }`. Per-player affinity via `PlayerTemplate.signature_candidates` (T1-3 schema change; v1 carry-forward). Softmax dispatch. Stacking across category boundaries; counterplay via defensive signatures with cancellation predicates.
- **ADR-0012 — Pinned-hash rebaseline policy.** Four legitimate triggers (canonical schema bump / encoder change / documented sim-behavior change / cross-OS divergence repair). Commit-body marker `canonical hash: REBASELINED (trigger: N; ...)`. Three-layer guard from commit `eb0b952e` stays untouched.
- **ADR-0013 — Licensed-data policy.** Three-layer enforcement: banned-terms lint (CI) + FW-VAL validator (bake-time, cosine + Soundex against ~50k-name corpus) + human review (long tail). Per-bake audit report committed alongside RON. Post-ship collision fix flow defined.
- **ADR-0014 — Runtime AI / content boundary.** Shipped game makes ZERO LLM calls. All generation at bake-time via `fw-content-baker` (dev tool, not shipped). Bake manifest captures `model_id + prompt_hash + seed + corpus_version`. Rebake policy + future-proofing path documented.
- **ADR-0015 — Phase-gate review policy.** Three tiers: Tier 1 = per-task self-review on ≥100 LoC (auto). Tier 2 = mid-phase targeted Codex audit (fires on 5 explicit criteria including schema lock + new canonical-state surface). Tier 3 = phase-boundary full audit at every `/done`. T1-1 demonstrated phase-boundary-only was too coarse.

### 11.7 What's not in this overview

Content-pack pipeline, manager AI, training, transfer market, board relations, media, season scheduling — each owns its own design doc as it lands per phase. See `docs/MASTER_PLAN.md` for phase order and `docs/design/*.md` for per-system specs.

---

## 12. Detailed sub-docs (to author in Phase 1)

The following will live under `design/` and be referenced from this doc. None exist yet in the new repo; all need authoring before their Phase 1 implementation work begins.

- `design/match-sim.md` — Rust MatchSim architecture, fixed-point primitives, BT runner, ball physics, pinned-hash corpus contract. (Inherits from the C# `match-engine.md`; rewrites for Rust idioms.)
- `design/careers.md` — Event-sourced memory ledger, salience formula, five readers, callback-tag schema, compaction strategy. (Inherits from `event-sourced-memory.md`.)
- `design/scouting.md` — Scout archetype catalog, report data shape, track-record system, disagreement vs uncertainty fallback. (Inherits from `scout-disagreement.md`.)
- `design/signatures.md` — 24-signature catalog with role-family grouping, sim bias schema, stacking policy, presentation recipe in text-recap form. (Inherits from `signatures.md`; presentation section rewritten for text-first.)
- `design/progression.md` — Identity Packet, internal gene model, phenotype label catalog, breakthrough trigger kinds, regressive triggers. (Inherits from `player-generation.md` + `breakthrough-moments.md`.)
- `design/content-pipeline.md` — Bake-time LLM corpus structure, Tracery + Markov runtime sampling, content-pack manifest, version migration. (New doc — replaces Unity-era `production-pipeline.md`.)
- `design/worldbuilding.md` — Regional priors, cultural cohorts, pyramid tier structure, club generation. (Inherits from existing `worldbuilding.md`.)
- `design/ui-vocabulary.md` — Football-native copy rules, banned-term lint, commentary tone guide. (Inherits from existing `ui-vocabulary.md`.)
- `design/modding.md` — Content-pack format (RON), stable-ID rules, mod-load order, sandbox boundary. (Inherits from existing `modding.md`.)

Architecture-bearing system additions require an ADR per `.claude/rules/design-docs/RULES.md` before the design doc lands.

---

## 13. Open Questions (resolve in next design pass)

1. **Pyramid scope at launch.** RESOLVED 2026-05-29 — one nation, 6 tiers, ~96 clubs; two-nation pyramid is post-EA. See DECISIONS.md 2026-05-29. The full 6-tier ~96-club pyramid + ~2000-player compiler + LLM bake pipeline are EA-critical (T4.5 phase).
2. **How procedural is too procedural.** Player names + club names + biographies are clearly bake-time. Are tactical doctrines, manager archetypes, and signature presentation banks also procedural, or hand-authored? Working assumption: archetypes + signatures hand-authored; commentary banks procedural. Resolution gate: Phase 1 commentary-bank prototype.
3. **Scout Disagreement vs Scout Uncertainty.** Inherited as conditional-MVP from the Unity FW. Gate is a feel-prototype with 3 external testers per the original `scout-disagreement.md`. Re-run the gate in the Rust/Tauri prototype or accept the prior outcome? Working assumption: re-run, because the surface (text-first dense UI) changes the affective response.
4. **Live-pause pacing model.** Real-time-with-pause vs auto-sim-to-event vs both. Working assumption: both, toggleable per match. Resolution gate: Phase 1 first playable.
5. **Save schema vs content-pack version coupling.** How do mid-career content-pack updates interact with active saves? Inherited delta-pack rules say additive-only + missing-pack fallback; verify the Rust serde + RON pipeline can hold that contract. Resolution gate: Phase 2 save-migration spike.
6. **Steam Workshop integration target.** Mod data shape is locked from Phase 1; UX target (in-game browser vs Workshop-as-folder-drop) is open. Resolution gate: Phase 6.
7. **Commentary depth ceiling.** How many distinct phrase variations per event class is "enough" before it reads as repetitive across a 200-hour career? Working assumption: 8-12 per common event, 3-5 per rare. Resolution gate: Phase 3 user-test.
8. **Tactical-board interaction depth.** Read-only viewer, or click-to-drag formation adjustments mid-match? Working assumption: read-only at MVP; click-to-drag is post-EA polish. Resolution gate: Phase 2 viewer spike.

---

*Authored 2026-05-13. Revise at each phase transition.*

# SPEC.md — Final Whistle

> Living work plan. Phase list, task checkboxes, decisions log. **Decisions log is append-only — enforced by hook.**
>
> Authored 2026-04-22. Updated every completed task (checkboxes) + every new decision (log, append-only).

---

## Current state

- **Active phase:** Phase 0 — Kickoff 🟡
- **Gate to next:** design-doc open questions resolved; Month-3 brutal-slice spec reviewed + accepted
- **Active task:** review `design/` doc scaffolds; resolve open questions

---

## Phases

### Phase 0 — Kickoff 🟡 ACTIVE
**Goal**: design bible's open questions resolved; ready to set up engineering.

- [x] Fill `PROJECT_CONTEXT.md` — pitch, audience, tone, 4-bucket scope split
- [x] Fill `TECH_APPROACH.md` — MatchSim architecture, determinism discipline, content pipeline
- [x] Scaffold 11 design docs with real content (purpose / locked decisions / MVP boundary / deferred / open questions / prototype gate)
- [x] Seed 19 initial decisions into decisions log
- [x] MCP inventory + plugin queue + global config (tier-capabilities.json)
- [ ] Review + resolve open questions in `design/overview.md`
- [ ] Review + resolve open questions in `design/month-3-vertical-slice.md`
- [ ] Review + resolve open questions in `design/match-engine.md` (fixed-point format, ball physics spec lock)
- [ ] Review + resolve open questions in `design/semantic-cinema.md` (7 shot types authored specs)
- [ ] Review + resolve open questions in `design/event-sourced-memory.md` (ledger schema lock)
- [ ] Review + resolve open questions in `design/signatures.md` (24-signature catalog draft)
- [ ] Review + resolve open questions in `design/scout-disagreement.md` (Month-4 prototype spec)
- [ ] Review + resolve open questions in `design/breakthrough-moments.md` (trigger conditions)
- [ ] Review + resolve open questions in `design/player-generation.md` (internal gene model finalize)
- [ ] Review + resolve open questions in `design/worldbuilding.md` (fictional nation scope lock)
- [ ] Review + resolve open questions in `design/ui-vocabulary.md` (banned-terms lint authored)
- [ ] `/refresh-docs` green

**Gate to Phase 1**: every design doc's open-questions section resolved; Month-3 brutal vertical slice signed off.

---

### Phase 1 — Setup ⚪ PENDING
**Goal**: machine ready, accounts ready, harness wired, first commit + remote pushed.

- [ ] Install Unity 6 LTS (pin version at Phase 3 kickoff) via Unity Hub with Mac + Win + Linux Build Support
- [ ] Install Blender (deferred-3D pipeline ready)
- [ ] Install VS Code with C# extension (or Rider)
- [ ] Account prerequisites: GitHub (exists), Steam Direct deferred to Phase 8
- [ ] `gh repo create Vibelogic/FinalWhistle --private --source=. --push` (user-gated)
- [ ] CI stub from `~/dev/blueprint/ci-cd/github-actions-unity.yml.template` adapted for MatchSim.Tests matrix (Win/Mac/Linux)
- [ ] Asset licensing tracker initialized
- [ ] Smoke-test slash commands: `/status`, `/next`, `/log-decision`
- [ ] Plugin install via slash commands (feature-dev / pr-review-toolkit / hookify)

**Gate to Phase 2**: machine + accounts + remote verified; `/next` picks up first Phase 2 task.

---

### Phase 2 — Design Bible ⚪ PENDING
**Goal**: every system design doc locked; engineering can start without guessing.

- [ ] `design/overview.md` locked
- [ ] `design/match-engine.md` locked (fixed-point format decided, ball physics spec)
- [ ] `design/semantic-cinema.md` locked (all 7 shot types fully specified)
- [ ] `design/event-sourced-memory.md` locked (ledger schema, compaction strategy)
- [ ] `design/signatures.md` locked (draft of all 24, starting set of 6 specified for Phase 3)
- [ ] `design/scout-disagreement.md` locked (Month-4 prototype spec)
- [ ] `design/breakthrough-moments.md` locked (trigger conditions + cinematic emphasis)
- [ ] `design/player-generation.md` locked (internal gene model → identity packet pipeline)
- [ ] `design/worldbuilding.md` locked (fictional nation, pyramid structure, cultural flavor)
- [ ] `design/ui-vocabulary.md` locked (banned-terms lint, approved-phrasing catalog)
- [ ] ADRs written for every load-bearing system decision
- [ ] `design/modding.md` — data architecture constraints every system must respect
- [ ] `design/accessibility.md` — target accessibility features for EA
- [ ] `design/content_policy.md` — PEGI 12 boundaries
- [ ] `/audit` green on Phase-2 checks

**Gate to Phase 3**: design bible complete; ADRs for every system that locks architecture.

---

### Phase 3 — Unity Bootstrap + MatchSim Prototype ⚪ PENDING
**Goal**: MatchSim runs deterministic on 22 players with custom ball physics; Unity + URP + 2D viewer prototype with 3 of 7 shot types.

- [ ] Create `MatchSim.csproj` as pure-C# class library
- [ ] Create `MatchSim.Tests.csproj` with xUnit
- [ ] Implement `Fixed` struct (Q32.32 canonical format)
- [ ] Implement `Tick` deterministic timestep loop
- [ ] Implement `Seed` (match + event seed derivation)
- [ ] Implement `Ball` custom deterministic physics (ground roll, air kick, bounce, friction; spin/Magnus stub acceptable for Month-3)
- [ ] Implement `Player` state machine (22 agents, basic movement + kick-ball)
- [ ] Author 2 behavior-tree manager archetypes in YAML (e.g., "Direct Pressing" + "Low-Block Counter")
- [ ] xUnit tests for determinism (hash canonical state after N ticks; compare Win/Mac/Linux via CI matrix)
- [ ] Create `unity-project/` via Unity Hub URP template
- [ ] Install Unity packages: UniTask, Addressables, Recorder, Input System, Localization, UI Toolkit (built-in)
- [ ] Install CoplayDev unity-mcp via Packages/manifest.json
- [ ] Assembly Definitions skeleton
- [ ] Addressables groups initialized
- [ ] First scenes: `Boot.unity` + `MatchViewer.unity`
- [ ] 2D semantic cinema prototype: `tactical-wide` + `diagonal-attack-lane` + `pass-shot-impact`
- [ ] Match-replay skill end-to-end (seed → headless match → viewer capture)
- [ ] Unity MCP handshake verified
- [ ] Devlog clips Month 2-3 published (first external audience signal)

**Gate to Phase 4 — MONTH 3 MATCH-ENGINE GATE**: *A stranger watches a 2D match for three minutes and understands drama, momentum, and player identity without reading a design doc.* FAIL = extend Phase 3 by one cycle; do not proceed.

---

### Phase 4 — Scout Disagreement Prototype + First Signatures ⚪ PENDING
**Goal**: Scout Disagreement feel-tested (gate for MVP inclusion); 3-6 signatures authored end-to-end.

- [ ] Implement internal player gene model + identity-packet compiler
- [ ] Scout Disagreement prototype: 3 scout archetypes, different biases, generate disagreeing reports on same player
- [ ] Feel prototype playtest (2 weeks max)
- [ ] MONTH-4 GATE: Scout Disagreement feel-test verdict logged as decision
- [ ] Author 3-6 signatures end-to-end: trigger conditions + sim bias + presentation recipe + counterplay
- [ ] Breakthrough moments prototype (match-flow cinematic emphasis, no pause)
- [ ] Closed itch build for ~10 trusted testers
- [ ] Retention data collected from itch testers

**Gate to Phase 5**: Scout Disagreement verdict logged; first signatures playable; retention data says keep going.

---

### Phase 5 — Vertical Slice ⚪ PENDING
**Goal**: one full season playable end-to-end; all 7 shot types; ledger operational.

- [ ] All 7 semantic-cinema shot types implemented
- [ ] Full season schedule (league + cup)
- [ ] Transfer market prototype (direct negotiation, no agents)
- [ ] Event-sourced memory ledger operational
- [ ] 3 memory readers (Alumni DB + rival recall + big-match scars)
- [ ] Starting set of 12 signatures playable
- [ ] Post-match report generation from templates
- [ ] Press/fan sentiment text via templates (no LLM at runtime)
- [ ] Save/load v1 (schema version 1, content pack v1)
- [ ] Month-6 public demo (conditional on itch retention)

**Gate to Phase 6**: one full season plays end-to-end; players want to start a second season.

---

### Phase 6 — Core Systems ⚪ PENDING
**Goal**: all 24 signatures + ~20-30 manager archetypes + save migrations tested + content pack v1 compiled.

- [ ] All 24 signatures authored (3 per role family × 8 role families)
- [ ] 20-30 manager archetypes in YAML
- [ ] Manager AI tuning via balance harness
- [ ] Content pack v1 compiled via AI Content Compiler: ~96 clubs, ~2000-2400 players, regional-flavor naming + cultural priors
- [ ] Save schema v2 + `migrate_v1_to_v2` tested
- [ ] Balance harness produces 10K-season sweep; key distributions documented
- [ ] 2 additional memory readers (promise tracking + press/fan callbacks)
- [ ] 5-8 salience-gated narrative event templates per season category
- [ ] Month-8 Steam Next Fest (conditional on first 10 minutes being sharp)
- [ ] Steam page draft: description, tags, first screenshots

**Gate to Phase 7**: systems architecturally complete; tuning happens during content scaling.

---

### Phase 7 — Content Scaling + Polish ⚪ PENDING
**Goal**: content-complete at EA target; polished to anti-FM26-regression standard.

- [ ] UI polish pass: navigation depth ≤ 2 clicks to any common action
- [ ] Performance pass: MatchSim + viewer on mid-range 2026 hardware within frame budget
- [ ] Balance harness production passes tuned
- [ ] Localization: English at launch; extract all user-facing strings to tables
- [ ] Accessibility: subtitles, colorblind, remappable controls, large-text UI, reduce-motion toggle
- [ ] QA pass: full season playthrough, bug triage
- [ ] Telemetry hooks (opt-in only)
- [ ] Crash reporting integration

**Gate to Phase 8**: game plays start-to-end at EA scope without blockers.

---

### Phase 8 — EA Launch (Month 12) ⚪ PENDING
**Goal**: Steam EA release button pressed.

- [ ] Steamworks SDK integration (achievements, stats, cloud saves, Workshop-ready scaffolding)
- [ ] Steam page finalized: description, 8-12 screenshots, trailer
- [ ] Age rating questionnaire (PEGI 12 / ESRB T via Steam)
- [ ] Steam Direct $100 paid
- [ ] Release candidate build + smoke-tested on clean machines
- [ ] Launch trailer + marketing assets
- [ ] EA launch date locked + public
- [ ] Day-1 patch prepared

**Gate to Phase 9**: game is live on Steam EA.

---

### Phase 9 — Post-EA ⚪ PENDING
**Goal**: sustainable support; 1.0 planning; 3D R&D begins ONLY if audience signal justifies.

- [ ] Hotfix cadence (week-1 critical)
- [ ] Community feedback triage
- [ ] Review-response strategy
- [ ] Audience-signal gate: does the game deserve 3D investment?
- [ ] If yes: 3D match engine R&D begins (Tripo / Hunyuan3D / Cascadeur subscriptions activate)
- [ ] Coaching Lineage surfacing (data was seeded at bootstrap; now expose)
- [ ] Manager Archetype Forge (Claude-generates BTs from English briefs)
- [ ] Counterfactual Development Lab (if/trained/as projections)
- [ ] Physical Load as Narrative Debt polish
- [ ] Workshop editor UX (data architecture was ready at bootstrap; now build)

---

## Backlog (unordered, for future phases)

- Multi-nation expansion (post-1.0 content push)
- Roguelike "Legend Run" condensed-career mode
- Second language pass: JP / ES / PT / DE
- Audio commentary voice-acting (ElevenLabs evaluation) — conditional on player demand
- Counterfactual Development Lab full UI
- Manager Archetype Forge English-to-YAML generator
- Physical Load as Narrative Debt (injury system polish)
- Dynasty / lineage mechanics (if audience retains + requests)
- Steam Deck Verified certification push
- Cross-save sharing / "Legend Exchange" async social
- Modding Workshop editor UX
- 3D match engine (scope after audience signal)

---

## Decisions log (append-only — hook-enforced)

> This section is immutable. Do NOT edit past entries. To supersede a prior
> decision, append a NEW entry citing the prior one. The
> `.claude/hooks/protect-decisions-log.sh` hook rejects Edits/Writes that mutate
> any existing `- **YYYY-MM-DD**` bullet. Use `/log-decision` to append.

- **<YYYY-MM-DD>** — **<Decision headline>**. Reasoning: <short why>.
- **2026-04-22** — **Project bootstrapped from blueprint v2.** Composed profile: 60% sim-management + 30% action-character + 10% narrative trimmings. Intake across 5 rounds (including GPT-5.5 design-partner rounds 3-4). Research scope active for Phase 0-2; contract to `rich` at Phase 2 lock.
- **2026-04-22** — **2D-first MVP committed; 3D explicitly deferred.** Reasoning: Rematch-tier 3D is 30-person-team work over years; solo dev in 12 months is fantasy. Commit 2D stylized manga-broadcast viewer as the final identity, not a waypoint. 3D only post-EA contingent on audience-signal gate.
- **2026-04-22** — **MatchSim architectural split.** Pure-C# `MatchSim.csproj` with zero UnityEngine references. Fixed-point canonical state (format Q16.16 vs Q24.8 decided at Phase 3 Week 1). Enables headless balance harness, xUnit tests, cross-platform deterministic replay.
- **2026-04-22** — **Custom deterministic ball physics (not Unity PhysX).** Rocket League lesson. Magnus force + air drag in fixed-point, lockstep with MatchSim. Allows controlled Tsubasa-curl signature trajectories while staying physically grounded.
- **2026-04-22** — **No capitalized state nouns in player-facing UI.** Banned visible names: "The Hush", "Weather", "Calling", "Canon", "Seven", "Kismet", "Soul", "The Author". Internal floats (`momentum`, `rhythm`, `pressure`, `team_cohesion`, `signature_readiness`) surface via football-native commentary only. See `design/ui-vocabulary.md` for lint.
- **2026-04-22** — **Event-sourced Career Memory as single architectural pattern.** Every meaningful event emits structured record to append-only ledger. Five reader subsystems (alumni / rival recall / promise tracking / big-match scars / press-fan callbacks) are readers, not separate systems. Compaction at 5-season boundary.
- **2026-04-22** — **Fully fictional football world with England-readable grammar.** No real places, no real clubs, no alternate-history framing. Fictional nation(s) with credible football culture, pyramid structure, regional rivalries. Avoids licensing risk + tonal uncanny-valley.
- **2026-04-22** — **24 pre-authored signatures, 3 per role family x 8 role families.** Not composable atoms. Each signature = role-specific football behavior + trigger conditions + sim bias + execution modifier + presentation recipe + counterplay. UI surfaces via football copy ("Looks for early crosses"), never power names.
- **2026-04-22** — **Breakthrough moments are match-flow cinematic, not pause-QTE.** Sim continues deterministically; viewer punches in with panel/impact beat; post-match report confirms development change. Manager influence is tactics/selection/training/promises/pressure — not mid-match pop-up choice.
- **2026-04-22** — **Coaching Lineage: data seeded at bootstrap; surfacing deferred post-MVP.** Alumni tactical-DNA fields exist in schema from Phase 3; rival-manager tactical inheritance becomes surfaced system only post-EA. Avoids 10-year payoff blocking 10-minute demo.
- **2026-04-22** — **Scout Disagreement: conditional MVP gated on Month-4 feel prototype.** Gate criterion: "Does disagreement create interesting decisions, or does it just obscure truth?" Pass means MVP inclusion. Fail means fall back to simpler scout uncertainty system.
- **2026-04-22** — **Narrative ceiling: 5-8 salience-gated events per season.** Not 30. Depth via remembering the right 5 things, not 50 shallow event spam. Salience scored on stakes x rarity x character involvement x rivalry x callback age x player attention.
- **2026-04-22** — **Tone anchor: Giant Killing + Aoashi + occasional anime exaggeration.** Grounded football first; heightened moments second. Not Ted Lasso (too warm), not Blue Lock (too extreme), not mythic-ancestry.
- **2026-04-22** — **Manager characterization minimal.** Doctrine + reputation + history through choices, not authored inner monologue. No "manager trauma system" at MVP.
- **2026-04-22** — **Mod-ready data architecture from day one; editor UX deferred.** Stable IDs (content-pack-qualified), schema versions, content packs, import validation, Workshop-ready assumptions baked in. Phase 0 Modding ADR constrains every system. Editor UX ships post-EA.
- **2026-04-22** — **AI-native content pipeline via bake-time compiler.** spec to JSON to validation to lint to sim sanity to content pack to import. No runtime LLMs. All player bios, match reports, press quotes rendered from templates with runtime slot-filling from event-ledger state.
- **2026-04-22** — **Behavior-tree manager archetypes (YAML), not ML-Agents.** Deterministic, debuggable, balance-harness reproducible. 20-30 hand-authored archetypes in MVP; Manager Archetype Forge (Claude-generates BTs from English) is post-EA content-scaling tool.
- **2026-04-22** — **EA scope: one fictional six-tier pyramid (~96 clubs, ~2000-2400 players).** Multi-nation deferred post-1.0. Depth over breadth.
- **2026-04-22** — **Price: $20 EA -> $30 1.0.** Solo indie RPG-management pricing. Not FM-institutional-tier. Steam Direct $100 at Phase 8.
- **2026-04-23** — **Q32.32 fixed-point is the canonical MatchSim format.** Reasoning: Q16.16 risks multiplication overflow and Q24.8 is too coarse for ball/player trajectory work. Q32.32 keeps deterministic math simple and precise; downgrade only if Phase 3 profiling proves fixed-point arithmetic is the bottleneck.

---

*Authored 2026-04-22. Updated every completed task (checkboxes) + every new decision (log, append-only).*

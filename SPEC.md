# SPEC.md — Final Whistle

> Living work plan. Phase list, task checkboxes, decisions log. **Decisions log is append-only — enforced by hook.**
>
> Authored 2026-04-22. Updated every completed task (checkboxes) + every new decision (log, append-only).

---

## Current state

- **Active phase:** Phase 2 — Design Bible 🟡
- **Gate to next:** design bible complete; ADRs for every system that locks architecture
- **Active task:** Phase-2 ADR authoring, ordered to unblock Phase-3 playable slice first — ShotTypeSO → Viewer rendering → Production pipeline → Golden replay corpus / save fixtures → remaining ADRs

---

## Phases

### Phase 0 — Kickoff ✅ COMPLETE (2026-04-24)
**Goal**: design bible's open questions resolved; ready to set up engineering.

- [x] Fill `PROJECT_CONTEXT.md` — pitch, audience, tone, 4-bucket scope split
- [x] Fill `TECH_APPROACH.md` — MatchSim architecture, determinism discipline, content pipeline
- [x] Scaffold 11 design docs with real content (purpose / locked decisions / MVP boundary / deferred / open questions / prototype gate)
- [x] Seed 19 initial decisions into decisions log
- [x] MCP inventory + plugin queue + global config (tier-capabilities.json)
- [x] Review + resolve open questions in `design/overview.md`
- [x] Review + resolve open questions in `design/month-3-vertical-slice.md`
- [x] Review + resolve open questions in `design/match-engine.md` (fixed-point format, ball physics spec lock)
- [x] Review + resolve open questions in `design/semantic-cinema.md` (7 shot types authored specs)
- [x] Review + resolve open questions in `design/event-sourced-memory.md` (ledger schema lock)
- [x] Review + resolve open questions in `design/signatures.md` (24-signature catalog draft)
- [x] Review + resolve open questions in `design/scout-disagreement.md` (Month-4 prototype spec)
- [x] Review + resolve open questions in `design/breakthrough-moments.md` (trigger conditions)
- [x] Review + resolve open questions in `design/player-generation.md` (internal gene model finalize)
- [x] Review + resolve open questions in `design/worldbuilding.md` (fictional nation scope lock)
- [x] Review + resolve open questions in `design/ui-vocabulary.md` (banned-terms lint authored)
- [x] `/refresh-docs` green

**Gate to Phase 1**: every design doc's open-questions section resolved; Month-3 brutal vertical slice signed off.

---

### Phase 1 — Setup ✅ COMPLETE (2026-04-24)
**Goal**: machine ready, accounts ready, harness wired, first commit + remote pushed.

- [x] Install Unity 6 LTS (pin version at Phase 3 kickoff) via Unity Hub with Mac + Win + Linux Build Support
- [ ] Install Blender (deferred-3D pipeline ready)
- [ ] Install VS Code with C# extension (or Rider)
- [x] Account prerequisites: GitHub (exists), Steam Direct deferred to Phase 8 (2026-04-24 — GitHub remote created `osagberg/FinalWhistle` private; Steam Direct $100 tracked in SETUP.md §10 trigger table for Phase 8)
- [x] `gh repo create osagberg/FinalWhistle --private --source=. --remote=origin` (2026-04-24 — created under personal namespace; `vibelogic` org exists but dev accounts not members. Transfer to publisher org deferred to Phase 8 if needed for Steam branding)
- [x] CI stub from `~/dev/blueprint/ci-cd/github-actions-unity.yml.template` adapted for MatchSim.Tests matrix (Win/Mac/Linux) — **superseded 2026-04-24** by Task 58 Tier-A workflow (`.github/workflows/fast-pr-ci.yml`) per `design/production-pipeline.md` tiered approach. Unity CI moves to Phase 3 manual-dispatch Tier B; dotnet matrix lands in Phase 3 inside the umbrella
- [x] Asset licensing tracker initialized
- [x] Phase-1 lint rule: full `scripts/lint-banned-terms.py` implementing Category-A (hard-ban, no exemption) + Category-B (inline `ui-lint:allow` exemption with term/reason/reviewer audit) + sentinel-exemption blocks (`ui-lint:ignore-start` / `ui-lint:ignore-end`). Scope: UI code + runtime content packs + rendered player-facing outputs. CI emits exemption report reviewed before EA content lock + every RC. Banned-term source is `design/ui-vocabulary.md` Categories A.1-A.5 including real-world place-name analogues (2026-04-24 — wired as `fw banned-terms`, integrated into `fw verify` umbrella + Tier-A CI; lint green clean across repo; zero Category-B exemptions in current codebase)
- [ ] Smoke-test slash commands: `/status`, `/next`, `/log-decision`
- [ ] Plugin install via slash commands (feature-dev / pr-review-toolkit / hookify)
- [ ] GitHub Actions budget cap set (stop on overage; Free 2k or Pro 3k included minutes per `design/production-pipeline.md`)
- [x] Fast PR CI (Tier A) workflow `.github/workflows/fast-pr-ci.yml` authored (2026-04-24 — single `Verify (Tier A umbrella)` job calls `scripts/fw verify`, which currently runs `verify-docs` + `banned-terms` sequentially. New checks (dotnet-test / determinism / content-lint / save-migration) wire into the `fw verify` umbrella as their Phase-3/6 deliverables land — no workflow edit needed per check)
- [x] `.github/PULL_REQUEST_TEMPLATE.md` authored (summary / why / test plan / breaking-changes / linked SPEC task)
- [x] `.github/ISSUE_TEMPLATE/bug_report.md` + `feature_request.md` authored
- [ ] Branch protection configured — **BLOCKED on plan upgrade**: GitHub Free does not allow branch protection on private repos (verified 2026-04-24 via `gh api`). Current posture: local-discipline-only per `docs/ops/branch-protection.md §0`. Unblocks on Pro/Team or making repo public; flip logged as SPEC entry when triggered
- [x] `scripts/fw` local command front-door skeleton (bash / makefile, no paid task runner) with verify / test / replay / content-lint / build-local / package-playtest stubs
- [x] `docs/ops/backup-restore.md` authored — GitHub + Time Machine + content-pack snapshot policy

**Gate to Phase 2**: machine + accounts + remote verified; `/next` picks up first Phase 2 task.

**Phase 1 ✅ COMPLETE (2026-04-24).** Machine (Unity installed), accounts (GitHub active + remote pushed, Steam deferred to Phase 8), remote (`osagberg/FinalWhistle` private, CI green end-to-end via Tier-A `fw verify` umbrella). Low-urgency user-actions carried over as open `[ ]` (Blender install is explicit Phase-3 trigger per SETUP.md §4; VS Code / Rider is user editor preference; slash-command smoke + plugin install happen in a fresh session; Actions $0 budget cap is a ~1-min GH UI step the user will handle soon). Branch protection is blocked on plan upgrade (GitHub Free constraint). None of these gate Phase 2.

---

### Phase 2 — Design Bible 🟡 ACTIVE
**Goal**: every system design doc locked; ADRs authored for every load-bearing system decision.

**Active posture (2026-04-24 promotion):** all 11 design docs had their open-questions resolved in Phase 0 via 12 consolidated SPEC entries. Phase 2 is now primarily about ADR authoring. Task ordering below prioritizes ADRs that unblock Phase-3's real risk — first deterministic MatchSim + watchable 2D viewer — before tidying-only ADRs.

**Design-doc locks** — all 11 substantively locked via Phase-0 2026-04-24 open-question resolutions. Remaining Phase-2 work is the ADR authoring below (system-level architecture commitments) + the three new design docs (modding / accessibility / content_policy):

- [x] `design/overview.md` locked (Phase 0 / 2026-04-24 — pillar tiebreaker, title, quickstart archetypes, nation framing)
- [x] `design/match-engine.md` locked (Phase 0 / 2026-04-24 — Q32.32 fixed-point, ball-physics structure, steering-target movement, Month-3 in-match scope)
- [x] `design/semantic-cinema.md` locked (Phase 0 / 2026-04-24 — 7-shot vocabulary, ShotTypeSO draft schema, rendering stack, typography w/ scoreline override). **ADR below.**
- [x] `design/event-sourced-memory.md` locked (Phase 0 / 2026-04-24 — salience structure, CallbackTag schema, 46-entry PascalCase event enum, three-tier compaction, load-time migration). **ADR below.**
- [x] `design/signatures.md` locked (Phase 0 / 2026-04-24 — 24-sig catalog with dependency metadata, tier-weighted affinity distribution, field-level capped stacking). **ADR below.**
- [x] `design/scout-disagreement.md` locked (Phase 0 / 2026-04-24 — Month-4 feel-prototype spec w/ 3 archetypes + one-remediation-pass ceiling). **ADR below (conditional-MVP).**
- [x] `design/breakthrough-moments.md` locked (Phase 0 / 2026-04-24 — cinema 3-5s, two-tier text, silent-first-near-miss, pillar-tiebreaker live-fire rule). No new ADR (composes existing schemas).
- [x] `design/player-generation.md` locked (Phase 0 / 2026-04-24 — 22-field gene model, 46-label phenotype catalog, ID-stability correction, affinity P(k) tier-weighted). **ADR below.**
- [x] `design/worldbuilding.md` locked (Phase 0 / 2026-04-24 — Caldren nation, 8 regions, 96-club pyramid, three-cup structure, compiler-only analogues). No new ADR.
- [x] `design/ui-vocabulary.md` locked (Phase 0 / 2026-04-24 — Categories A.1-A.5, sentinel exemptions, 140-template flatter pool, British-football tone default). No new ADR; lint shipped Phase 1.

**Phase-2 ADRs — ordered to unblock Phase-3 first (playable slice = first deterministic MatchSim + watchable 2D viewer):**

- [ ] ADRs written for every load-bearing system decision
- [x] **ADR (1): `ShotTypeSO` schema + Addressables grouping** (2026-04-24 — `design/adr/adr-0001-shot-type-so-schema.md` **Accepted** after user review pass: ChainConditionId registry-backed (no scripted predicates in content packs), explicit deterministic-selection contract (priority + Id-tiebreak + base-then-mod-pack precedence + forbidden nondeterminism sources + replay-seed variation path), per-content-pack Addressables grouping with 3-label convention)
- [x] **ADR (2): Viewer rendering pipeline + URP custom-pass ordering** (2026-04-24 — `design/adr/adr-0002-viewer-rendering-pipeline.md` **Accepted** with the Knowledge Risk MEDIUM gate (URP Render Graph API verification at Phase-3 Week 1 spike against pinned Unity 6 LTS URP 17+) baked in. Self-review tightening: `fw shader-audit` promoted to explicit Phase-3 SPEC task to enforce no-`_Time`-in-FW-shaders discipline via Tier-A CI)
- [x] **ADR (3): Production pipeline** (2026-04-24 — `design/adr/adr-0003-production-pipeline.md` **Accepted** after user tightenings: self-hosted-runner acceptance gate (4 conditions — trigger-restricted, explicit labels, restricted label set, blast-radius declared — checked BEFORE any runner registers), stale commit-hash anchor removed. 5-tier model + 5-channel build metadata + runner policy + cost discipline + release-gate rules formalized. Four rejected alternatives)
- [x] **Golden replay corpus format specified** (2026-04-24 — `design/specs/golden-replay-corpus.md`. JSON fixture schema v1 with corpus_schema_version, match_seed, content_pack_version, archetype_ids, reduce_motion, sim_length_ticks, tick_rate_hz, expected block (final_score, key_event_hashes, final_canonical_state_hash, pass_activation_log_hash), verification_scope enum. Tier-A smoke seed pinned `0xdeadbeefdeadbeef`. Authoring runbook + CI contracts + growth policy included. 3 open questions deferred to Phase-3 Week 1. Unblocks Phase-3 determinism hash CI gate)
- [x] **Save migration fixture policy specified** (2026-04-24 — `design/specs/save-migration-fixtures.md`. Four-test-per-schema-bump discipline (forward-migration + callback-preservation + forward-incompat + round-trip); fixtures append-only at `MatchSim.Tests/fixtures/saves/`; Tier-A smoke + Tier-D full-matrix CI contracts; per-subject versioning; ~5 fixtures at Phase 3, ~15 at Phase 8. Schema-bump PR without fixture + 4 tests is unmergeable)
- [x] ADR (4): MemoryEvent schema, callback-tag enum, compaction tiers, and migration framework (2026-04-24 — `design/adr/adr-0004-memory-event-schema.md` authored as **Proposed**; locks MemoryEvent struct + Q32.32 Salience + CallbackTag registry with consuming-reader validation + three-tier compaction with N_quota cap + load-time MigrationChain. Four rejected alternatives: mutable salience, single-tier keep-everything, lazy-per-read migration, string-tag callback registry. First real user of save-migration-fixture spec)
- [ ] ADR (5): SignatureSO schema — content-pack IDs, dependency metadata, scope enum, stacking policy per MatchSim field, Identity Packet affinity-roll integration (seeded by 2026-04-24 Signature system resolution). Phase 3 authors 3 signatures end-to-end.
- [ ] ADR (6): IdentityPacket / AI Content Compiler — IdentityPacket schema, phenotype enum governance, affinity-count rolls, content-pack ID rules (no pack-minor in entity IDs), canonical-artifact discipline, scout visibility mapping (seeded by 2026-04-24 Player-generation resolution). Phase 3 needs 22 packets (hand-authored acceptable).
- [ ] ADR (7): Scout archetype schema + ScoutReport schema + callback/event integration + fallback behavior if Month-4 gate fails (seeded by 2026-04-24 Scout Disagreement resolution; conditional-MVP — architecture slot reserved regardless of gate outcome). Phase-4 dependency.
- [ ] Content-pack validation contract specified: duplicate/legal-sensitive names + missing localizations + invalid phenotype/signature/event-class IDs + unresolved content-pack-qualified IDs + real-world-analogue leakage + banned UI vocabulary. Tier A subset + Tier D full. (Phase-6 implementation.)
- [ ] Phase-6 evaluation: Tier-A smoke-seed single-vs-rotation — measure Phase-3 `fw replay` runtime, expand to 3-seed rotation ONLY if per-run budget stays inside Tier-A's 5-minute ceiling (policy locked in `design/specs/golden-replay-corpus.md`)
- [ ] Artifact retention policy specified — GitHub-hosted runner artifact TTLs, Tier-C local-upload summary retention, RC-build archival policy.
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
- [ ] Author `MatchSim.Tests/SerializationContract.cs` — stable order for entities / events / Q32.32 fields. **Gates Phase-3 Week-2 golden-replay-corpus fixture authoring** per `design/specs/golden-replay-corpus.md` open questions. Cross-platform hash parity depends on this
- [ ] Author `scripts/fw shader-audit` — greps `FinalWhistle.Viewer.Rendering` shaders for banned `_Time` references per ADR-0002 determinism discipline. Wires into `fw verify` umbrella once authored so Tier-A CI catches regressions immediately. **Gates ADR-0002 validation criterion "no `_Time` references in viewer shaders"**
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
- [ ] Local MatchSim CI scripts implemented via `scripts/fw` — `fw test`, `fw replay <seed>`, `fw verify` match Tier-A behavior locally
- [ ] GitHub dotnet-test matrix (Tier A subset — Win/Mac/Linux) green on `MatchSim.Tests` determinism suite
- [ ] Deterministic replay hash check: ONE canonical corpus seed runs green in Tier A (full corpus deferred to Tier C/D)
- [ ] Unity smoke build workflow `.github/workflows/unity-smoke.yml` exists as **manual-dispatch only** — no PR trigger. One target (Windows OR WebGL) for now
- [ ] `scripts/fw build-local` produces a local Unity build (current platform only) for solo-dev verification

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
- [ ] `docs/ops/playtest-distribution.md` authored — itch.io private-restricted page flow, feedback collection (Google Form or in-build markdown), build signing/hashing
- [ ] In-build "Export bug bundle" button ships: save + last-N replay seeds + rolling logs + settings + content-pack version. Testers email/itch-message zip; no cloud ingest
- [ ] `scripts/fw package-playtest` produces signed build zip ready for itch upload

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
- [ ] Crash / log bundle exporter implemented — rotating local logs (bounded size), exportable diagnostics zip, no PII by default, no cloud endpoint
- [ ] `docs/ops/crash-logs-telemetry.md` authored — local-first discipline, optional anonymous playtest JSON opt-in with SQLite/CSV ingest

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
- [ ] Balance harness Tier-C (local / self-hosted) produces 10K-sweep summary artifacts uploadable to GitHub; full replay-corpus regen + diff runnable locally
- [ ] Save compatibility fixtures checked into `MatchSim.Tests/fixtures/saves/` — v1 fixture + v1→v2 migration test + callback-preservation test + forward-incompat failure test (per Phase-2 policy)
- [ ] Content-pack validator (full) implemented — Tier-A subset already running; Tier-D full suite (legal-sensitive-names diff, AI-content disclosure check)
- [ ] Golden replay corpus v1 checked in — seeds + expected hashes + regeneration command in `scripts/fw`

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
- [ ] Tier-D Release Candidate workflow `.github/workflows/release-candidate.yml` green end-to-end — full dotnet matrix + full Unity builds + content-pack validator full suite + save migration matrix + asset-license audit + banned-term exemption audit
- [ ] Tier-E Steam deploy workflow `.github/workflows/steam-deploy.yml` authored with **manual-approval gate** — never auto-deploys on tag
- [ ] `docs/ops/release-channels.md` authored — dev/tester-closed/demo/ea/hotfix channel metadata + validation tiers
- [ ] Release checklist copied to version-specific doc (e.g., `release/v0.1.0-ea-checklist.md`)
- [ ] Rollback build tested on clean machine — confirmed restores prior EA build
- [ ] AI-content disclosure metadata checked (manifest of bake-time LLM uses per Steam / regulatory requirements)

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
<!-- ui-lint:ignore-start reason="internal dev-tool feature name (proper noun), not player-facing verb usage" -->
- [ ] Manager Archetype Forge (Claude-generates BTs from English briefs)
<!-- ui-lint:ignore-end -->
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
<!-- ui-lint:ignore-start reason="internal dev-tool feature name" -->
- Manager Archetype Forge English-to-YAML generator
<!-- ui-lint:ignore-end -->
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

<!-- ui-lint:ignore-start reason="append-only decisions log; meta-references to banned terms are intentional historical record" -->

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

- **2026-04-24** — **Overview pillar questions resolved (design/overview.md Q1-Q4).** Consolidated entry per user preference. (1) **Nation framing:** single named fictional nation with England-readable football grammar; actual nation name owned by `design/worldbuilding.md`, not duplicated in overview. (2) **Product title:** "Final Whistle" locked as working / product title. Formal trademark + Steam-name clearance deferred to Phase 8 launch prep. Known existing non-AAA uses (`finalwhistle.es` daily-mini-game, `finalwhistle.club` community product) are clearance-pass flags, not blockers. (3) **Quickstart club archetypes — 4 locked for EA:** decaying-giant (tier 2), rising-academy (tier 3), mid-table-survivalist (tier 1), backs-against-the-wall (tier 5). (4) **Pillar tiebreaker (P1 Memory vs P3 Watchability):** Memory wins by default. If a callback would interrupt a high-leverage live match sequence, watchability temporarily wins and the callback is queued to the next natural surface (dead ball, half-time, full-time, or post-match report). Callbacks are deferred, never suppressed. High-leverage = score margin ≤ 1 in final 10 in-game minutes, or any cup / promotion / relegation / derby / title-deciding sequence.

- **2026-04-24** — **Month-3 vertical-slice gate parameters resolved (design/month-3-vertical-slice.md Q1-Q4 + observer-pool lockdown).** Consolidated entry. (1) **Match type:** opening-day league fixture, two stylistically distinct fictional teams; no cup final, no title decider, no derby — the gate tests baseline legibility and must not be flattered by rivalry-driven stakes. Derby / cup stress tests move to Phase 5. (2) **First 3 signatures (names exact per `design/signatures.md`):** #20 Low cutback from the byline (winger), #22 Blind-side near-post run (striker), #13 First-time diagonal switch (CM). (3) **Gate artifact:** local build OR one continuous ~3-minute recording shown privately to 5 cold observers; no public / itch build for the gate itself. Short 30-60s clips are for devlog / audience-signal posts and do not count as the gate artifact. (4) **Pass criterion:** ≥4 of 5 football-literate cold observers (casual fans ~10+ matches/year, not project collaborators) responding privately in writing before any discussion can describe both (a) the match's emotional arc and (b) at least one specific player's style in football-native language. **Fail modes:** "boring" (watchability failed) or "confusing" (legibility failed) — route the fix to the failing pillar; do not paper over by adding features. (5) **Observer-pool lockdown:** if 5 plausible football-literate cold observers cannot be named by end of Month 2, the gate is at risk. Fallback: recruit a tiny private test pool via trusted friends / Discord / private itch keys. Criterion is not weakened; the recruiting problem is solved separately.

- **2026-04-24** — **Match-engine open questions resolved.** Q32.32 remains canonical per 2026-04-23. Month-3 ball physics uses Q32.32 position/velocity/spin, semi-implicit Euler at 60Hz, gravity, linear drag, ground bounce, rolling friction, radius-based possession checks, goal-plane detection, and touchline transitions; Magnus structure exists but may run with zero coefficient for the Month-3 gate if curve reads noisy. Player movement uses steering-target BT output plus deterministic fixed-point actuator caps; switching to continuous force integration requires a superseding SPEC decision or ADR. Month-3 excludes substitutions, injuries, fouls, cards, stoppage time, and VAR; Phase 4 introduces fouls/set pieces, cards, substitutions, basic injuries, then stoppage time. VAR remains deferred indefinitely. Numeric coefficients (`g`, `C_d`, `C_m`, `e`, `μ_step`) are fixed-step tuning seeds in `design/match-engine.md`, NOT SPEC-locked values.

- **2026-04-24** — **Semantic Cinema open questions resolved.** The 7-shot vocabulary is locked through the Month-3 gate; expansion beyond 7 requires a superseding SPEC decision. Shot authoring uses Addressable ShotTypeSO assets with stable IDs, framing parameters, stakes/memory modulation, chain rules, fallback shot, overlay template set, and reduce-motion variant; Phase 2 must add an ADR for ShotTypeSO schema and grouping. Viewer effects use URP fullscreen passes for screen-tone and impact frames, per-player trail meshes/sprites for motion lines, and UI Toolkit overlays for panel/text composition with fallback if masking is brittle; Phase 2 must add an ADR for viewer rendering pipeline and pass ordering. Typography stack is Anton for display impact, JetBrains Mono for data/stat/scout/debug, and Rajdhani for body/commentary/menu text; persistent scoreline uses Rajdhani SemiBold or JetBrains Mono, not Anton. Font licensing is verified in the Phase-1 asset tracker.

- **2026-04-24** — **Event-sourced memory open questions resolved.** Salience locks its formula structure and semantic bands, while weights/cutoffs remain Phase-6 tuning seeds; callback age and player attention are reader-side surfacing modifiers, not emission-time salience inputs. Callback tags are a fixed MVP enum with consuming-reader metadata, extensible post-EA through schema/content-pack versioning. Event classes use a versioned PascalCase enum with a tight ~35-40 starter set; growth beyond MVP requires schema review. Compaction uses three tiers: season-defining events hard-preserved, notable events compact-preserved with callback-essential fields, lower events aggregated only; top 5% salience events per season are hard-preserved with a per-season cap. Save/load uses load-time forward migration of the save envelope and MemoryEvents; no downgrades. Every schema bump requires migration tests. Phase 2 must add an ADR for MemoryEvent schema, callback tags, compaction, and migration.

- **2026-04-24** — **Signature system open questions resolved.** The 24-signature catalog is locked with dependency metadata, including Phase-4+ scheduling flags for set-piece/foul-dependent signatures. #19 is corrected to "Cuts inside onto his stronger foot"; #6 is scoped as a defensive-line effect, not a global team buff. Signature affinity count follows a power-law tail including rare zero-affinity players, with distribution tuned by generation tier and balance sweeps. Multi-signature effects stack through field-level capped policies, not generic softmax. Readiness uses a default threshold with per-signature override. Counterplay surfaces through scout reports for known signatures. Phase 2 must author a SignatureSO/schema ADR covering content-pack IDs, dependencies, scope, stacking policy, and Identity Packet affinity integration.

- **2026-04-24** — **Scout Disagreement open questions resolved.** Month-4 feel prototype uses 3 scout archetypes: physical_profiler, technical_purist, and regional_expert. Reports are canonical structured data rendered into deterministic prose templates. The user facilitates but does not count toward the gate; pass requires at least 2 of 3 external management-game-literate testers to satisfy all three criteria: scout-specific trust attribution, at least one changed sign/avoid decision versus neutral aggregate, and affective response that frames scouts as interesting models rather than noise. Fail modes are RNG-fail, ignore-fail, and overload-fail, with one short remediation pass allowed before final fallback to Scout Uncertainty. The prototype uses 10 hand-authored Identity Packet stubs and minimal MemoryEvent writes for scout track-record feedback. Phase 2 must pre-seed an ADR for Scout archetype schema, ScoutReport schema, callback/event integration, and fallback behavior if the Month-4 gate fails.

- **2026-04-24** — **Breakthrough Moments open questions resolved.** Cinema beat duration locked to 3-5s range with default Phase-3 tuning seed 3s; 5s reserved for genuinely high-stakes beats. Overlay text is a two-tier observational pattern; all system/progression vocabulary ("Signature unlocked," "Awakened," "XP gained," mystical state nouns) is banned and enforced via `ui-vocabulary.md` lint — text describes football behavior, not progression mechanics. Near-miss handling: silent on first same-match occurrence, post-match stat-card after 2nd+ to prevent farming. Regressive triggers have equal gravity to positive breakthroughs (same duration, same shot chain, tone modulation through existing semantic-cinema channels). Pillar-tiebreaker interaction: during normal play, breakthrough cinema defers to the next natural surface (dead ball → half-time → post-match); during a high-leverage sequence, the cinema may fire immediately only if the triggering action is the resolving beat (shot / save / tackle / final pass). Live play is never interrupted mid-sequence; dead-ball breakthroughs fire immediately because the natural surface already exists. No new ADR — composes existing ShotTypeSO, SignatureSO, MemoryEvent, and UI-vocabulary schemas.

- **2026-04-24** — **Player-generation open questions resolved.** The internal model is locked at 22 fields across physical, mental, technical, and narrative-flag categories; growth requires a schema bump. Player-facing phenotype labels use a stable content-pack-qualified enum with an MVP ceiling of 50 labels; role-specific labels are expanded to cover all role families, while systemic or stigmatizing labels such as "Plateau Risk" are removed or moved into scout prose. Advanced numeric scout details default off and expose only scout-estimated ranges, never true internal values. Content generation is reproducible by canonical checked-in JSON/content-pack artifacts, not by assuming LLM bit determinism. Content-pack deltas are additive-only; stable player IDs do not encode minor pack versions. Signature affinity count uses cohort-weighted power-law tuning seeds, and Scout Disagreement reads category-level visibility weights only. Phase 2 must author an IdentityPacket / AI Content Compiler ADR covering schema, phenotype enum governance, affinity rolls, content-pack IDs, and scout visibility.

- **2026-04-24** — **Worldbuilding open questions resolved.** The fictional nation is named **Caldren** — preferred over Cresland because it reads as a grounded fictional football nation, supports clean league/cup naming (Caldren Premier Division, Caldren National Cup, Caldren Football Association), and avoids awkward demonym forms. Cresland remains the fallback if formal clearance fails during Phase-8 launch prep. Caldren is used as in-game setting context, not Steam-facing branding. MVP uses 8 regions with compiler-internal real-world analogue notes only; user-facing region names are fictionalised and analogue leakage is blocked by a Phase-1 lint rule. EA content pack v1 contains 96 fully simulated clubs across six tiers: 20 / 24 / 16 / 14 / 12 / 10. This represents the fully simulated slice, not the entire off-screen Caldren football ecosystem. Cup structure locks three competitions: all-tier National Cup, top-two-tier League Cup, and tier-3-to-6 Trophy. Small-tier season format (repeat fixtures vs cross-group phase) flagged as Phase-6 decision point. RegionPriors remain governed by the IdentityPacket / AI Content Compiler ADR.

- **2026-04-24** — **UI vocabulary open questions resolved.** Player-facing vocabulary lint covers UI code, runtime content, content-pack JSON, and rendered player-facing doc/content outputs. `design/ui-vocabulary.md` uses explicit ignore sentinels only around banned-term catalog sections; no whole-file self-whitelist. Category-A terms have no exemption path (expanded with 2026-04-24 additions: system/progression vocabulary, genetics/bloodline terms, stigmatizing phenotype framings, real-world place-name analogues). Category-B terms may use audited inline exemptions with exact term, specific reason, and reviewer handle; exemption reports are reviewed before EA content lock / release candidates. Commentary overlay templates use flatter per-shot-type pools of 15-30 templates, targeting ~140 MVP match-flow templates, with stake/memory filters and slot variables providing variation. Default English tone is British-football vernacular; other locales use native football idiom and locale-specific banned-term lists. Template governance folds into the Phase-2 AI Content Compiler ADR.

- **2026-04-24** — **Production pipeline planning pass (GPT-5.5 report).** Authored `design/production-pipeline.md` as authoritative CI/CD + release-ops plan. Core posture: GitHub is source-of-truth for code/docs/PRs and cheap PR-gate CI; Unity CI is slow, license-sensitive, expensive (macOS especially) and runs manual-dispatch only through Phase 7; heavy sim sweeps (10K-match, balance harness, replay corpus regen, full Unity matrix) run local or on self-hosted runner; release CI is manual-approval only. Five workflow tiers: A (fast PR, ≤5 min Linux), B (Unity smoke, manual dispatch), C (heavy local/self-hosted), D (release candidate, tagged + manual), E (Steam deploy, manual approval only). Build channels: dev / tester-closed / demo / ea / hotfix. Core deliverables owed by the pipeline: golden replay corpus (Phase-2 spec, Phase-3 implement), save migration fixtures (Phase-2 spec, Phase-6 implement), content-pack validator (Phase-2 spec, Phase-6 full), local `scripts/fw` command front-door (Phase-3), in-build bug-bundle export + itch.io distribution (Phase-4), local-first crash/log exporter with opt-in anonymous telemetry (Phase-5+), backup policy (Phase-1). No paid pipeline services through MVP. Phase-1/2/3/4/5/6/8 SPEC tasks pre-seeded. Phase-2 ADR pre-seeded for production pipeline itself. TECH_APPROACH.md to add Production Pipeline section cross-referencing this doc.

- **2026-04-24** — **Phase 1 ✅ COMPLETE; Phase 2 🟡 promoted; ADR order prioritized for Phase-3 playable-slice unblock.** Phase 1 closed with all Claude-actionable work shipped (Tier-A CI green, `fw verify` umbrella includes verify-docs + banned-terms lint, runbooks for branch protection + Actions budget cap written, repo public-committed on `osagberg/FinalWhistle`). Low-urgency user-actions (Blender install / VS Code editor / slash-smoke / plugin install / Actions $0 cap) roll over as open Phase-1 `[ ]` and do NOT gate Phase 2 per solo-dev convention. Stale cleanup: Task 52 (Unity CI stub from blueprint) marked `[x] (superseded)` by Tier-A workflow; Task 50 (account prerequisites) marked `[x]` on remote creation. Phase 2 design-doc locks marked `[x]` across all 11 docs (substantively locked via Phase-0 2026-04-24 open-question resolutions). Phase-2 ADR order reprioritized per GPT-5.5 2026-04-24 guidance to feed Phase-3's real risk (first deterministic MatchSim + watchable 2D viewer) rather than tidy-doc order: (1) ShotTypeSO, (2) Viewer rendering pipeline, (3) Production pipeline, (4) Golden replay corpus format, (5) Save migration fixture policy, then MemoryEvent / SignatureSO / IdentityPacket / Scout archetype. Phase-2 gate unchanged: design bible complete + ADRs for every load-bearing system decision.

<!-- ui-lint:ignore-end -->

---

*Authored 2026-04-22. Updated every completed task (checkboxes) + every new decision (log, append-only).*

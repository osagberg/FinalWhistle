# Final Whistle — Migration Audit

**Audit date:** 2026-05-13
**Source repo:** `/Users/vibelogic/dev/football/`
**Target:** new Rust + Tauri 2 + SolidJS, text-first management sim repo. Procedural fantasy football. FM-killer.
**Frozen snapshot:** `/Users/vibelogic/dev/football-archive/`

Classifications:
- **DESIGN-SOURCE** — content/patterns carry forward; the file itself does not.
- **REFERENCE** — keep accessible in archive; consult during reimplementation; do not import.
- **WORKFLOW-PORT** — copy the file (or its pattern) into the new repo's toolchain.
- **DROP** — irrelevant under the new stack/scope.

---

## 1. Top-line summary

The Final Whistle codebase is **97% Unity-stack scaffolding around a small core of genuinely portable game design**. The carry-forward is concentrated in five design docs (signatures, event-sourced memory, player generation, scout disagreement, ui-vocabulary) plus two specs (golden-replay-corpus pattern, save-migration-fixtures pattern) plus the deterministic-sim discipline encoded in `MatchSim/Sim/Fixed.cs` and the canonical-state encoder. The 11 ADRs, the entire `unity-project/`, the `Viewer.*` adapter tree, the dots-renderer/3D-pipeline/semantic-cinema/anime-presentation surface, and 10 of 15 project agents drop because they encode Unity rendering, ScriptableObject authoring, URP custom passes, and a watchable-match pillar that the new text-first scope explicitly removes. The workflow disciplines — SPEC/STATUS/CHANGELOG cadence, append-only decisions log, banned-terms lint, ADR rhythm, pinned-hash regression fixtures, the `scripts/fw` front-door pattern — are the most leveraged carry-forward. Agent-bus stays optional: it is heavyweight relative to a leaner solo+single-reviewer cycle and should port only if Codex review remains an integral phase gate. New repo target: under 50 docs+config files. The brainstorm folder, all dialog history, the entire dots-adapter blueprint, and the 3D-pipeline placeholder are pure archive.

---

## 2. Path-by-path classification

### 2.1 Top-level docs

| Path | Classification | Notes |
|---|---|---|
| `CLAUDE.md` | DESIGN-SOURCE | Take the section structure (§1 What this project is / §2 Source-of-truth map / §3 Tech stack locked / §4 Tooling / §5 Workflow / §6 Style / §7 Pitfalls / §8 First-session directive) verbatim; rewrite all Unity/MatchSim/3D-pipeline content. Mandatory delegation table (§6.3) is workflow gold — port the pattern, rebuild the rotation for Rust/Tauri/Solid. |
| `PROJECT_CONTEXT.md` | DESIGN-SOURCE | Rewrite the pitch (no Unity, no 3D viewer, no anime-cinema pillar). Keep the "careers that remember + players are specific" pillars; drop "every match is watchable." Reframe USPs around text-sim depth + procedural-world + memory ledger. |
| `SPEC.md` (423 lines) | REFERENCE | Phase structure + decisions-log style ports as a pattern; line content is Unity/phase-gate-specific and does not. Decisions log itself is REFERENCE for back-archaeology only ("why did we pick Q32.32?"). |
| `STATUS.md` | DROP | Pure state pointer at one moment in Phase 3 of the old project. Cadence pattern carries via the workflow port. |
| `CHANGELOG.md` (2723 lines) | DROP | Old project's shipping log. Append-only cadence ports via workflow; lines do not. |
| `TECH_APPROACH.md` (432 lines) | REFERENCE | Determinism discipline (fixed-point, deterministic seeding, canonical-state encoding) is portable principle, not file content. The Unity asmdef/Addressables/URP material is all DROP. Rewrite from scratch for Rust crates + Tauri IPC + Solid signals. |
| `TOOLING.md` | DESIGN-SOURCE | Catalog *shape* — MCPs / plugins / subagents / CLIs / hooks with adopt/skip reasoning — ports. Specific entries (Unity AI Assistant, blender-mcp, unity-check skill) drop entirely. |
| `SETUP.md` | DROP | Unity Hub + dotnet + macOS asset-store workflow. New stack has none of this. |
| `AGENTS.md` (10 lines) | WORKFLOW-PORT | Trivial Codex pointer — port as-is with updated paths. |
| `FinalWhistle.slnx` / `global.json` / `.blueprint-version` | DROP | C#/.NET solution metadata. |
| `.mcp.json` | DESIGN-SOURCE | Strip Unity MCPs; keep the file as the place where new-repo MCPs (e.g. context7, github) land. |
| `.gitignore` / `.gitattributes` | WORKFLOW-PORT | Strip Unity-specific patterns (`Library/`, `Temp/`, `.meta`, etc.); keep dotfile + IDE patterns. |

### 2.2 Design folder (`design/`)

| Path | Classification | Notes |
|---|---|---|
| `design/README.md` | DESIGN-SOURCE | Replace with new-repo index. |
| `design/overview.md` | DESIGN-SOURCE | Three pillars: keep #1 (memory) + #2 (specific players); cut #3 (watchable match — supersession needed). Quickstart archetypes, pillar-tiebreaker, MVP boundary all transfer with text-only reframing. |
| `design/signatures.md` (236 lines) | DESIGN-SOURCE | The **24-signature catalog by role family** is the single most load-bearing carry-forward in the repo. Re-render names as text-sim outputs ("Looks for early crosses" surfaces in commentary lines + scout reports, not via shot-type cinematics). The lifecycle (latent → earning → breakthrough → active), stacking policy (field-level caps, not softmax), affinity-distribution table all transfer. |
| `design/event-sourced-memory.md` (241 lines) | DESIGN-SOURCE | **MemoryEvent schema, 5-input salience formula, 5 readers (alumni / rival-recall / promise / big-match-scar / press-fan), 3-tier compaction with per-season top-5% quota, load-time forward migration** — all directly portable; the new Rust impl owes the same shape. The 42-event-class starter set may need a haircut once "Every match is watchable" is dropped (SignatureExecuted presentation events become surface-only). |
| `design/player-generation.md` (315 lines) | DESIGN-SOURCE | **Internal 22-field gene model (7 physical / 6 mental / 5 technical / 4 narrative-flag) + Identity Packet output + 46-label phenotype catalog + cohort-weighted affinity rolls + bake-time AI content compiler pipeline** all port. Drop the "Phase-3 fixtures vs Phase-4 compiler" timing — the new project can start with the compiler. |
| `design/breakthrough-moments.md` (151 lines) | DESIGN-SOURCE | Three trigger kinds (signature awakening / latent potential / regressive). Drop the 3-5s cinematic timing + chain_rules + reduce-motion variant; in text-sim, breakthroughs become structured post-match report beats. Two-tier observational text pattern (Quiet → Match-specific) carries verbatim. |
| `design/scout-disagreement.md` (207 lines) | DESIGN-SOURCE | Conditional-MVP gate, 3 archetypes (`physical_profiler` / `technical_purist` / `regional_expert`), staged-time feedback loop, fail-mode taxonomy — all transferable. **This system is more important in a text-first sim** than in the original because the scouting screen IS the main player-evaluation surface. Prioritize. |
| `design/worldbuilding.md` (199 lines) | DESIGN-SOURCE | Nation/regions/pyramid structure + cultural priors + cup competitions + compiler-only-analogue lint discipline all port. The Caldren name itself is optional (rebrand if new project changes title). |
| `design/match-engine.md` (138 lines) | DESIGN-SOURCE | **Deterministic sim discipline** (fixed-point canonical state, 60Hz tick, seeded RNG, CI matrix on three OSes) carries; the specific Q32.32 + ball-physics + steering-target choices may simplify under a text-sim model (do you even need ball physics if the user never watches a match?). Re-evaluate scope at port-time: a text sim might reduce to event-driven match-state ticks rather than 60Hz physics. |
| `design/production-pipeline.md` (333 lines) | DESIGN-SOURCE | Tiered CI/CD model (A fast-PR / B smoke / C heavy-local / D RC / E deploy), GitHub-Actions budget posture, `scripts/fw` front-door, build-channel naming, backup policy — all port. Drop Tier-B Unity-smoke (no Unity) and the macOS-budget rationale (no Unity Editor minutes). |
| `design/ui-vocabulary.md` (211 lines) | DESIGN-SOURCE | **Banned-terms lint is mandatory** in a text-sim — even more load-bearing than in the original because every UI surface is text. Categories A.1-A.5 all transfer except A.5 (real-place-analogues — re-derive if new project picks different region names). Sentinel-comment exemption mechanism + Category-B audit discipline + tone-register pattern all port verbatim. |
| `design/modding.md` (233 lines) | DESIGN-SOURCE | **12-constraint data-architecture contract** (stable IDs, schema versions, content packs, registry-backed IDs, base-then-mod precedence) is the mod-readiness floor. All 12 transfer; the Unity Addressables-grouping (#3) becomes filesystem/manifest grouping in Rust. |
| `design/content_policy.md` (209 lines) | DESIGN-SOURCE | PEGI 12 boundary table (mature themes in scope: dressing-room tension, ageing-star arcs, relegation anxiety, derby hostility, etc.) is the dramatic palette — directly applicable. AI-content disclosure metadata is a Steam-publish requirement that ports unchanged. |
| `design/accessibility.md` (234 lines) | REFERENCE | Five EA-target features (reduce-motion / colorblind / remap-controls / large-text / subtitles). Reduce-motion mostly disappears in a text-sim (no motion-lines or impact-flash). Colorblind palette + text-scale + remap-controls + subtitles port. Read as a checklist, not a verbatim copy. |
| `design/3d-pipeline.md` (146 lines) | DROP | Spike-gate contract for cel-shaded 3D. Project pivot drops 3D entirely. |
| `design/anime-presentation-budget.md` (85 lines) | DROP | Eight anime-coded visual surfaces for the dots adapter. None apply to text. |
| `design/semantic-cinema.md` (194 lines) | DROP | 7-shot-type camera grammar. Entirely viewer-stack-specific. The "stakes + memory modulate presentation" *principle* is portable to commentary template selection — note it but do not import the doc. |
| `design/month-3-vertical-slice.md` (118 lines) | DROP | One specific Unity-Editor playable-match milestone. Replace with a text-sim Month-3 gate spec written from scratch. |
| `design/brainstorm/01-genetics-system.md` | REFERENCE | Useful for "why we landed on 22-field bounded gene model"; do not import. |
| `design/brainstorm/02-fm26-gap-analysis.md` | DESIGN-SOURCE | **Competitive analysis directly relevant to the FM-killer thesis.** Top-5 differentiators table, top-3 risks. Re-evaluate for 2026 FM27 landscape at port-time but most of the structural arguments (license-free, fictional-universe lock-in, RPG-depth, AI-native content density, emergent narrative) hold or strengthen in text-first. |
| `design/brainstorm/03-anime-sports-conventions.md` | REFERENCE | Anime-trope mining. Drops most of its value if the anime-presentation pillar is gone. |
| `design/brainstorm/04-cutting-edge-systems.md` | REFERENCE | Tech-opportunity survey. Mostly Unity/3D-specific; the bake-time-LLM-worldbuilding and MCP-native-dev-loop arguments port. |
| `design/brainstorm/05-original-ip-pivot.md` | REFERENCE | Five world-setting pitches. May be useful for naming/tone but not load-bearing. |
| `design/specs/golden-replay-corpus.md` (216 lines) | DESIGN-SOURCE | **Format pattern is the most important transferable thing in the entire repo for the new sim's regression discipline.** Append-only JSON fixtures, content-pack-version + archetype-pair inputs, pinned hashes for canonical state + event stream, Tier-A smoke / Tier-C local-regenerate / Tier-D full-matrix CI contracts, "generator owns ordering" rule. The `pass_activation_log_hashes` viewer-side surface drops (no viewer); everything else carries. |
| `design/specs/save-migration-fixtures.md` (209 lines) | DESIGN-SOURCE | Four-tests-per-bump discipline (forward-migration + callback-preservation + forward-incompat-failure + round-trip-byte-identical) + per-schema-version fixture accumulation policy. Directly portable. |
| `design/specs/football-rules-matrix.md` (87 lines) | DESIGN-SOURCE | Football-rules-by-row matrix (goal-detection, touchline, offside, fouls, cards, subs, stoppage, etc.) with Phase-3 simplifications + canonical-impact + tests-owed + promotion-trigger columns. Format ports; rows may need re-derivation for text-sim. |
| `design/specs/content-pack-validation-contract.md` (262 lines) | DESIGN-SOURCE | Per-check FW-VAL ID scheme, red-team fixture-per-check requirement, failure-message convention, Tier-A/Tier-D split. Ports cleanly. |
| `design/specs/artifact-retention-policy.md` (248 lines) | REFERENCE | Five retention tiers + per-class TTLs. Re-derive for the new repo's actual artifact set; the *categorisation principle* (ephemeral / short / release-tied / permanent-in-repo / local-only) is the carry-forward. |

### 2.3 ADRs (`design/adr/`)

| Path | Classification | Notes |
|---|---|---|
| `adr-0001-shot-type-so-schema.md` | DROP | ShotTypeSO + Addressables grouping. Viewer-specific. |
| `adr-0002-viewer-rendering-pipeline.md` | DROP | Superseded; URP custom-pass ordering. |
| `adr-0003-production-pipeline.md` | DESIGN-SOURCE | Tier-A/B/C/D/E CI/CD model + manual-approval-only Steam deploy through EA. Re-render in Rust/Tauri terms. |
| `adr-0004-memory-event-schema.md` (394 lines) | DESIGN-SOURCE | MemoryEvent schema, CallbackTag registry with consuming-reader metadata, 3-tier compaction, load-time migration framework. Directly portable to Rust. |
| `adr-0005-signature-so-schema.md` (404 lines) | DESIGN-SOURCE | Signature schema, scope enum, field-level capped stacking, display/enum-id separation. Drop the ScriptableObject framing; the data shape and stacking rules port. |
| `adr-0006-identity-packet-compiler.md` (406 lines) | DESIGN-SOURCE | IdentityPacket data shape + AI Content Compiler pipeline + manifest audit trails + canonical-artifact discipline + content-pack ID rules. Directly portable. |
| `adr-0007-scout-archetype-schema.md` (389 lines) | DESIGN-SOURCE | Scout archetype + ScoutReport schema + Path B (Scout Uncertainty) fallback. Directly portable. |
| `adr-0008-shot-presentation-contract.md` | DROP | Renderer-agnostic viewer contract. No viewer in new scope. |
| `adr-0009-dots-phase-render-adapter.md` | DROP | Sprite-on-pitch adapter spec. |
| `adr-0011-unity-ai-assistant-mcp-migration.md` | DROP | Unity MCP routing. Not applicable. |
| `adr-0012-autonomous-implementation-protocol.md` (226 lines) | REFERENCE | The autonomous Tier-2 implementation protocol (task-spec → Claude implements → Codex reviews via agent-bus → commit gate). Heavyweight; port only if the new project keeps Codex-as-phase-gate-reviewer. Otherwise drop. |

### 2.4 Code

| Path | Classification | Notes |
|---|---|---|
| `MatchSim/Sim/Fixed.cs` + `Vector3Fixed.cs` | REFERENCE | Q32.32 fixed-point arithmetic — proves the "checked-math, integer-bit-equivalent across OS" discipline works. In Rust, port using `i128` or a fixed-point crate; the *discipline* is the carry-forward, not the C# code. |
| `MatchSim/Sim/CanonicalEncoder.cs` + `MatchCanonicalState.cs` | REFERENCE | The locked-order serialization pattern (Tick + Ball + 22 PlayerStates + score + OutOfPlay + KeyEvents) is the canonical-state-hash floor. Re-derive for whatever the text-sim state shape is. |
| `MatchSim/Sim/BehaviorTreeRunner.cs` + `BehaviorTreeArchetypes.cs` + `*.yaml` archetypes | DESIGN-SOURCE | BT-driven role defaults + manager archetypes loaded from YAML — pattern transfers. The actual 60Hz player-movement BT logic is sim-specific; a text-sim probably has a higher-level event-tree, not a per-tick BT. |
| `MatchSim/Sim/BallPhysics.cs` + `MatchRules.cs` + `PlayerActuator.cs` etc. | DROP | 60Hz physics for the dots viewer. Re-derive a text-sim match-tick model from scratch. |
| `MatchSim/Sim/Signature*.cs` (`SignatureRules.cs`, `SignatureConfig.cs`, `SignatureCooldownState.cs`, etc.) | REFERENCE | Signature evaluation rules. Read as guidance when re-implementing in Rust; the C# code does not port. |
| `MatchSim/Sim/KeyEvent.cs` + `KeyEventKind.cs` | DESIGN-SOURCE | The KeyEvent enum + canonical encoding shape are the canonical-event-stream contract. Re-derive in Rust matching event-class catalog in event-sourced-memory.md. |
| `MatchSim/Content/IdentityPacket.cs` + `IdentityPacketValidator.cs` | DESIGN-SOURCE | The validation rules (regex for ID format, gene-range clamping, schema-version forward-migration) are the discipline; the C# code does not port but the validation surface does. |
| `MatchSim/Content/archetypes/direct-pressing.yaml` + `low-block-counter.yaml` | DESIGN-SOURCE | YAML schema for manager archetypes (name + description + formation + press-radius + buildup-speed-factor). Re-derive for text-sim manager-style data shape. |
| `MatchSim/Content/identity-packets/*/01-11.json` (22 files) | DESIGN-SOURCE | 22 hand-authored IdentityPacket fixtures. The JSON schema is portable; the values may need re-derivation if text-sim has different gene fields. Useful as seed data for the new project's first compiler run. |
| `MatchSim/Memory/*.cs` (Ledger, SalienceEngine, PressFanReader, BreakthroughReader, etc.) | REFERENCE | Implementation of event-sourced memory. Read for "we already worked out the integration sequence"; do not port C# directly. |
| `MatchSim.Tests/Sim/*.cs` (28 test files) | DESIGN-SOURCE (as patterns) | The test *intents* (FixedDeterminismTests, MatchDeterminismTests, GoalkeeperBehaviorTests, OffBallFormationTranslationTests, PassPressureScoringTests, SeparationTests) name the regression surface the new sim must also cover. Hash literals are dead; test classes are dead; the *what to test* matrix is the carry-forward. |
| `MatchSim.Tests/fixtures/replay-corpus/0xdeadbeefdeadbeef.json` + `0xfeedbeefcafefade.json` | DESIGN-SOURCE | **Fixture format ports verbatim** (corpus_schema_version + match_seed + content_pack_version + home_archetype_id + away_archetype_id + reduce_motion + sim_length_ticks + tick_rate_hz + expected{final_score, key_event_hashes, final_canonical_state_hash, pass_activation_log_hashes, locale_pin} + verification_scope + generated_at + generated_by). Hash values themselves are dead. |
| `MatchSim.Tests/fixtures/save-migration/` | DROP | Empty directory; pattern lives in the spec. |
| `MatchSim.Tests/Memory/*.cs` (4 tests) | REFERENCE | Read for integration-test shape. |
| `MatchSim.Tests/Content/IdentityPacket*Tests.cs` | REFERENCE | Same. |
| `unity-project/**` | DROP | Entire Unity project — DLL drops, scenes, shaders, prefabs, dots-adapter, ProjectSettings, Packages, plans. Frozen in archive only. |
| `steam-release/asset-licensing-tracker.csv` | DROP | Asset licensing for Unity-pipeline 3D/anim plans. New repo derives its own at Phase 8. |

### 2.5 Scripts

| Path | Classification | Notes |
|---|---|---|
| `scripts/fw` (750 lines) | WORKFLOW-PORT (pattern) | The **`scripts/fw` front-door pattern is the highest-leverage workflow port** — single bash CLI with `verify` umbrella + `verify-docs` + `banned-terms` + `shader-audit` + `test` + `replay <seed> --compare-corpus` + `save-migration-test` + stubs for future commands. Re-derive `fw` for new stack: keep `verify-docs`/`banned-terms`/`test`/`replay`/`save-migration-test`; drop `shader-audit`/`build-unity-plugins`/`verify-unity-plugins`; add `tauri build`/`solid lint`/`cargo test` equivalents. |
| `scripts/lint-banned-terms.py` (375 lines) | WORKFLOW-PORT | **Port verbatim.** Source-of-truth catalog reads from `design/ui-vocabulary.md`; sentinel-comment exemption mechanism; Category-A vs B vs C; supports `--report` for exemption audit JSON. Update the file-scope globs for the new repo's source tree. |
| `scripts/agent-bus` (1036 lines) | REFERENCE | Heavy CLI. Port only if Codex-review-as-phase-gate remains an integral workflow. If the new project goes solo+Claude without an external reviewer, drop entirely. If Codex stays in the loop at phase boundaries only (not continuous review), a leaner version may emerge; treat the JSONL append-only spec as the carry-forward, not the script. |

### 2.6 .claude/ skills

| Path | Classification | Notes |
|---|---|---|
| `.claude/skills/duo-debate/SKILL.md` | REFERENCE | Tier-1 architectural-debate orchestrator. Heavyweight; port only if agent-bus stays. |
| `.claude/skills/duo-implement/SKILL.md` | REFERENCE | Tier-2 autonomous-coding-task orchestrator. Same caveat. |
| `.claude/skills/codex-review-loop/SKILL.md` | REFERENCE | Codex-CLI-side polling reviewer. Same caveat. |
| `.claude/skills/check-reviews/SKILL.md` | REFERENCE | Claude-side review-inbox with cascade-prevention. Same caveat. |
| `.claude/skills/unity-check/` | DROP | Three-level Unity verification skill. |
| `.claude/skills/unity-webgl-builder/` | DROP | Unity WebGL builder skill. |
| `.claude/skills/unity-audio-generator/` | DROP | Procedural placeholder audio for Unity. |
| `.claude/skills/match-replay/SKILL.md` | REFERENCE | Seed → headless replay → viewer-frame capture. New project drops the Unity-viewer half; the "seed → canonical determinism check + structured event dump" half can be re-built as a thinner skill. |
| `.claude/skills/state-dump/` | DROP | Unity Editor IDumpable state dump via Editor menu. |
| `.claude/skills/github-pages-deploy/` | REFERENCE | Generic GH-Pages deploy via Unity WebGL — if the new project ever publishes a browser build (Tauri can target web), reread; the WebGL-specific bits drop. |
| `.agents/skills/*` | DROP | Codex-side duplicates of the same Unity-bound skills. |

### 2.7 .claude/ commands (slash commands)

| Path | Classification | Notes |
|---|---|---|
| `.claude/commands/next.md` + `done.md` + `log-decision.md` + `status.md` | WORKFLOW-PORT | Base four. Port verbatim with updated phase-gate references. |
| `.claude/commands/audit.md` + `refresh-docs.md` + `architecture-decision.md` + `gate-check.md` + `milestone-review.md` | WORKFLOW-PORT | High-value process commands. Port and reskin. |
| `.claude/commands/design-system.md` + `design-review.md` + `create-stories.md` + `dev-story.md` + `story-done.md` + `story-readiness.md` | WORKFLOW-PORT | Design/implementation cadence commands. Port. |
| `.claude/commands/brainstorm.md` + `quick-design.md` + `balance-check.md` + `code-review.md` + `hotfix.md` + `release-checklist.md` | WORKFLOW-PORT | Port. |
| `.claude/commands/architecture-review.md` + `create-architecture.md` + `create-epics.md` + `map-systems.md` + `review-all-gdds.md` + `art-bible.md` | REFERENCE | Re-evaluate at port-time — most port, but `art-bible.md` is largely irrelevant in a text-first project. |
| `.claude/commands/regression-suite.md` + `smoke-check.md` | DROP | Unity-check L2/L3 wrappers. Replace with `cargo test` / `pnpm test` shells. |
| `.claude/commands/bootstrap.md` + `start.md` + `help.md` + `expand-studio.md` + `contract-scope.md` + `deep-research.md` | WORKFLOW-PORT | Generic blueprint commands. Port. |

### 2.8 .claude/ agents (15 project-specific subagents)

| Path | Classification | Notes |
|---|---|---|
| `creative-director.md` | WORKFLOW-PORT | Pillar guardian + scope arbitration. Universally useful. |
| `game-designer.md` | WORKFLOW-PORT | Core-loop / mechanics author. Universally useful. |
| `systems-designer.md` | WORKFLOW-PORT | **Higher value in text-sim** — formulas, curves, balance harness, economy. Port. |
| `narrative-director.md` | WORKFLOW-PORT | Event-sourced memory surfacing + scout-prose templates. Direct carry-forward. |
| `producer.md` | WORKFLOW-PORT | Phase-gate enforcement + cross-discipline coordination. Universal. |
| `lead-programmer.md` | WORKFLOW-PORT | Reskin from SOLID/C# to Rust idioms. |
| `qa-lead.md` | WORKFLOW-PORT | Test-strategy + AC review. Universal. |
| `gameplay-programmer.md` | REFERENCE | Heavily Unity/MatchSim-flavored prose. Replace with a "sim-programmer" agent re-derived for Rust. |
| `engine-programmer.md` | DROP | Unity scene management / Addressables / GC / Profiler. Replace with a "platform-programmer" for Tauri/IPC if needed. |
| `ui-programmer.md` | REFERENCE | UI Toolkit / UXML / Canvas. Rewrite as "frontend-programmer" for SolidJS. |
| `art-director.md` | DROP | 2D manga-broadcast visual identity. Text-sim has no art bible. |
| `technical-director.md` | DROP | Unity package adoption + render-pipeline + URP/HDRP. Reskin entirely as Rust/Tauri architecture authority OR drop. |
| `unity-specialist.md` | DROP | |
| `unity-ui-specialist.md` | DROP | |
| `.claude/agents/README.md` | WORKFLOW-PORT | Tier 1 / Tier 2 / Tier 3 structure + escalation map + authoring pattern all port. |

Roster recommendation for the new repo: **5 agents** — creative-director, game-designer, systems-designer, narrative-director, lead-programmer (renamed to "rust-programmer" or kept generic) + qa-lead + producer = 7 if producer/qa-lead included. Expansion via `/expand-studio` is the pattern.

### 2.9 .claude/ hooks

| Path | Classification | Notes |
|---|---|---|
| `protect-decisions-log.sh` | WORKFLOW-PORT | Append-only SPEC.md decisions-log enforcement. Direct port. |
| `update-status-timestamp.sh` | WORKFLOW-PORT | Stop hook rewrites STATUS.md `**Last updated**` line. Direct port. |
| `pr-review-reminder.sh` | WORKFLOW-PORT | PreToolUse on `git commit` — soft reminder when ≥100 LoC of code in staged diff. Update extension list (`.rs` / `.ts` / `.tsx` / `Cargo.toml`). |
| `session-start.sh` + `session-stop.sh` | WORKFLOW-PORT | Print live phase + last 3 commits at session boot; print files modified + commits-this-session at stop. Port. |
| `pre-compact.sh` + `post-compact.sh` | WORKFLOW-PORT | Snapshot pre-compact context → restore post-compact. Direct port. |
| `protect-dialog-append-only.sh` | REFERENCE | Agent-bus topic immutability. Port only if agent-bus ports. |
| `canonical-hash-guard.sh` | WORKFLOW-PORT (pattern) | PreToolUse on `git commit` — runs targeted canonical-hash regression test before allowing commit. Re-derive for the Rust sim's hash test once it exists. The *pattern* of "structural floor even if review missed it" is the carry-forward. |
| `detect-gaps.sh` | WORKFLOW-PORT | SessionStart follow-up — surface missing design docs / stub-only folders. Port. |
| `log-agent.sh` + `log-agent-stop.sh` | WORKFLOW-PORT | Audit trail of subagent invocations. Direct port. |
| `pre-build-verify.sh` | DROP | Unity batchmode build pre-flight check. |
| `validate-assets.sh` | DROP | Unity Assets/ naming convention check. |
| `validate-commit.sh` | WORKFLOW-PORT | Pre-commit secret scan + JSON validity + decisions-log preservation + TODO/FIXME warnings. Direct port. |
| `validate-push.sh` | WORKFLOW-PORT | Warn on force-push to main. Direct port. |

### 2.10 .claude/ rules (path-scoped coding rules)

| Path | Classification | Notes |
|---|---|---|
| `.claude/rules/CSharp/RULES.md` | DROP | C#/UniTask/async-Task rules. Replace with `Rust/RULES.md` for Rust idioms. |
| `.claude/rules/Addressables/RULES.md` | DROP | Unity Addressables. |
| `.claude/rules/Assemblies/RULES.md` | DROP | Unity asmdef boundary discipline. Replace with `Crates/RULES.md` for Cargo workspace structure. |
| `.claude/rules/ScriptableObjects/RULES.md` | DROP | Unity-specific. |
| `.claude/rules/Scripts/AI` `Characters` `Combat` `Core` `CoreMechanic` `Debug` `Dialog` `Editor` `Outfits` `Pipeline` `Stats` `UI` `Viewer` | DROP | Unity scripts trees. Most are template residue from blueprint init; never customized for FW. |
| `.claude/rules/Scripts/MatchSim/RULES.md` | DESIGN-SOURCE | "MatchSim is source of truth; Unity renders it, never drives it. Pure C#, zero UnityEngine. Q32.32 fixed-point. Deterministic RNG. Structured MatchEvents." Reskin as Rust + Tauri-frontend-never-drives-sim. |
| `.claude/rules/Scripts/Memory/RULES.md` | DESIGN-SOURCE | Append-only ledger; reader/writer split; deterministic event IDs; salience explicit. Port. |
| `.claude/rules/Scripts/Players/RULES.md` | DESIGN-SOURCE | Stable IDs, internal genes invisible, phenotype labels only, no "genes/bloodline/DNA" in UI. Port. |
| `.claude/rules/Scripts/Signatures/RULES.md` | DESIGN-SOURCE | Behavior + trigger + sim bias + presentation recipe + counterplay; football-readable copy. Port. |
| `.claude/rules/Scripts/Pipeline/RULES.md` | REFERENCE | AI content compiler pipeline rules. Port the discipline. |
| `.claude/rules/Scripts/Viewer/RULES.md` | DROP | |
| `.claude/rules/design-docs/RULES.md` | WORKFLOW-PORT | Frontmatter discipline, cross-ref-must-resolve, formula-vs-prose, no-duplicate-source-of-truth, design-doc-as-contract. Direct port. |
| `.claude/rules/tests/RULES.md` | WORKFLOW-PORT (pattern) | EditMode-vs-PlayMode-vs-Performance split is Unity-specific; the *discipline* of "tests are documentation that runs, regression-test-per-bug, deterministic test data" is universal. Reskin for Rust + `cargo test` + property-testing crates. |

### 2.11 Content

| Path | Classification | Notes |
|---|---|---|
| `MatchSim/Content/archetypes/*.yaml` | DESIGN-SOURCE | Manager-archetype YAML schema (covered above). |
| `MatchSim/Content/identity-packets/*/*.json` | DESIGN-SOURCE | 22 hand-authored IdentityPackets (covered above). |

### 2.12 Dialog (agent-bus topics)

| Path | Classification | Notes |
|---|---|---|
| `dialog/README.md` + `dialog/example-topic.jsonl` | REFERENCE | Protocol explainer + worked example. Port only if agent-bus ports. |
| `dialog/2026-05-*.jsonl` (16 topic files) | REFERENCE | Historical agent-bus debate transcripts (Slice 7 / MCP migration / ADR-0012 / signatures / polish-pass round 1-3). **Pure archive value** — useful for "why did we decide X" lookups during reimplementation. Do not import into new repo. |

### 2.13 Tests fixtures (covered above)

`MatchSim.Tests/fixtures/replay-corpus/*.json` — DESIGN-SOURCE for format, dead for hashes.
`MatchSim.Tests/fixtures/save-migration/` — DROP (empty).

### 2.14 docs/ + design-templates/ + steam-release/

| Path | Classification | Notes |
|---|---|---|
| `docs/tooling/agent-bus-spec.md` (442 lines) | REFERENCE | Detailed agent-bus protocol spec. Port only if agent-bus ports. |
| `docs/tooling/unity-mcp-routing.md` + `unity-mcp-playbook.md` | DROP | Unity MCP routing matrix + playbook. |
| `docs/ops/actions-budget.md` | WORKFLOW-PORT | GitHub Actions minutes budget posture (Free 2k / Pro 3k). Direct port. |
| `docs/ops/backup-restore.md` | WORKFLOW-PORT | Backup policy (covered in production-pipeline.md). Direct port. |
| `docs/ops/branch-protection.md` | WORKFLOW-PORT | Migration runbook from direct-to-main to PR-only mode. Direct port. |
| `docs/plans/dots-adapter-blueprint.md` (437 lines) | DROP | Detailed 7-slice dots-adapter implementation plan. |
| `docs/screenshots/*.png` | DROP | All Unity dots-viewer screenshots. |
| `design-templates/*` (14 templates) | WORKFLOW-PORT | 14 canonical doc skeletons: ADR, architecture-traceability, game-concept, GDD, game-pillars, hud-design, playtest-report, postmortem, release-checklist, release-notes, systems-index, test-evidence, test-plan, ux-spec. Most port directly; `hud-design.md` may not apply in text-sim. |
| `steam-release/asset-licensing-tracker.csv` | DROP | Unity-stack licensing. New project re-derives at Phase 8. |
| `.github/PULL_REQUEST_TEMPLATE.md` | WORKFLOW-PORT | PR template with summary/why/linked/test-plan/breaking-changes/checklist. Direct port; strip the Unity-specific bullets. |
| `.github/workflows/fast-pr-ci.yml` | WORKFLOW-PORT (pattern) | Three independent jobs (static checks Linux-only; cross-OS test matrix; cross-OS plugin-reproducibility). Direct pattern port; replace dotnet test with `cargo test` + `pnpm test`. |
| `.github/ISSUE_TEMPLATE/` | WORKFLOW-PORT | Bug-report + feature-request templates. Direct port. |
| `.claude/bootstrap/` + `.claude/context-scopes.json` + `.claude/statusline.sh` | REFERENCE | Blueprint v2 init artefacts. Reuse the blueprint-bootstrap rhythm rather than copying. |

---

## 3. Patterns worth preserving

Short prose extracts — the *discipline* to import, not the verbatim text.

### 3.1 Pinned-hash-per-fixture regression discipline

For any deterministic system (canonical sim state, content-pack-encoded output, save-file shape, ledger compaction): commit a small set of append-only fixture files where each fixture has inputs + expected output hashes. The hashes are pinned literals in either the fixture file or a sibling test. Drift in any hash = either a real regression or an intentional change requiring a "regenerate corpus + reviewer-approved diff" PR. Tier-A smoke is one seed/fixture; Tier-D full-matrix is all of them across Win/Mac/Linux. The generator owns the canonical ordering of fields inside fixture files — humans never hand-maintain it — so cross-host JSON canonicalization is reproducible. This is the single discipline most worth importing.

### 3.2 Append-only decisions log

Single file (`SPEC.md` in the old project) holds a chronological, append-only list of dated decision bullets. To supersede an earlier decision, append a new entry citing the prior one — never edit. A PreToolUse hook enforces the invariant by rejecting Edit operations that would mutate any line matching `^- \*\*\d{4}-`. The new entry must contain the old bullet as a literal substring. Six months later the reasoning trail is intact and grep-able.

### 3.3 STATUS / CHANGELOG cadence

`STATUS.md` is one file rewritten on every `/done`; auto-timestamped via Stop hook so the "Last updated" line stays honest without dev memory. `CHANGELOG.md` is append-only human-readable; every `[x]` in SPEC has a CHANGELOG line. The pair captures "where are we now" (STATUS) + "how did we get here" (CHANGELOG) in two cheap files. Resist letting STATUS grow into a recap — it's a state pointer, not a diary.

### 3.4 ADR rhythm

ADRs in `design/adr/adr-NNNN-<slug>.md` with status Proposed → Accepted → Superseded-by lifecycle. Status field is binding: never modify an Accepted ADR; supersede via a new numbered ADR that links back. Each ADR has Engine Compatibility (or Stack Compatibility) + Dependencies + Decision + Consequences + Verification-Required fields. The dialog/agent-bus topic name is cited in the review-trail bullet on the ADR if external review pass was involved.

### 3.5 Banned-terms lint with sentinel exemptions

Categories A.1-A.5 hard-ban (no exemption); B inline-exempt via `// ui-lint:allow term="..." reason="..." reviewer="..."` requiring all three attributes; C context-allowed. Source-of-truth catalog is one design doc; lint script reads from it. Sentinel-comment region exemption (`<!-- ui-lint:ignore-start -->`/`-end`) for the catalog doc itself + for sections that intentionally describe banned vocabulary. Exemptions audited at every release-candidate gate. The discipline matters more in a text-sim than a 3D one — every UI surface is the lint target.

### 3.6 Mandatory subagent rotation table

CLAUDE.md §6.3 contains a "Task class → required agent" table that names the subagent rotation for each work category before any code is written. `/next` must name the task class and required agent(s) explicitly. Skipping the mandated agent requires a one-liner in the commit body explaining why. The discipline forces the main thread to stay in coordination mode (not authoring 800 LoC into its own context) and exercises the otherwise-unused subagents. Audit data from the old project showed ~50% of project agents had zero invocations before the rotation table existed.

### 3.7 Agent-bus append-only JSONL

If a multi-model review cycle (Claude + Codex + user) is part of the workflow, sharing context via copy-paste loses framing and encourages agreement loops. The `dialog/<topic>.jsonl` append-only protocol with `in_reply_to` sha256 threading + severity-tagged claim/counter/evidence/ack/note/decision event types gives both models a shared addressable transcript. Pre-commit hook enforces append-only on the JSONL files. Worth importing only if external review is structurally in the loop; for solo+Claude (no Codex), the lighter "STATUS.md + decisions log" trail is enough.

### 3.8 Five-tier CI/CD model

Tier A (fast PR, ≤5min, every push, Linux + cross-OS for the determinism gate only), Tier B (manual-dispatch smoke), Tier C (local-only heavy sweeps + harness regen), Tier D (RC gate, full matrix, infrequent), Tier E (manual-approval-only deploy). Each tier has a budget. Higher tiers do not auto-run. Hard cap on GitHub Actions minutes; overage off by default. Architecture from day one, not feature-MVP. Steam-direct ($100) and platform builds deferred to Phase 8 trigger.

### 3.9 `scripts/fw` front-door pattern

Single bash CLI that umbrellas every Tier-A check + every local helper. Subcommands print "not yet implemented at this phase" rather than silently succeeding. The umbrella (`fw verify`) calls each sub-check in sequence so Tier-A logic is one path: same script locally as on CI. New subcommands wire in here as deliverables land. Avoid dependency-heavy task runners (Just, Task, Nx) until Bash hurts.

### 3.10 Mod-readiness as architectural posture

Content-pack-qualified stable IDs (`fwh.core:player_00042`); schema versions on every persisted subject; load-time forward migration with no downgrades; per-pack content-pack-grouped Addressables / filesystem layout; deterministic base-then-mod-pack precedence with lexicographic tiebreak; registry-backed IDs (no inline strings) for anything content packs reference; pack-minor versions never leak into entity IDs. Twelve-constraint contract spelled out in `design/modding.md`. The whole package is what makes Workshop loadability free at EA instead of a refactor.

### 3.11 Canonical-state encoder + deterministic-seed RNG

The deterministic-sim floor: a `CanonicalEncoder` writes the entire match-state snapshot in a locked field order, byte-equal across OSes; SHA-256 over that buffer is the canonical state hash. Per-tick stochastic events derive from `(match_seed, tick, event_id)` rather than platform RNG. Locked-order JSON serialization for save/replay. Round-trip test asserts byte-identical reserialize.

---

## 4. Carryforward checklist

Files / patterns to literally copy or near-verbatim port into the new repo:

**Bash / Python (port verbatim or with trivial pathing edits):**
- [ ] `scripts/lint-banned-terms.py` (375 lines)
- [ ] `.claude/hooks/protect-decisions-log.sh`
- [ ] `.claude/hooks/update-status-timestamp.sh`
- [ ] `.claude/hooks/pr-review-reminder.sh`
- [ ] `.claude/hooks/session-start.sh`
- [ ] `.claude/hooks/session-stop.sh`
- [ ] `.claude/hooks/pre-compact.sh`
- [ ] `.claude/hooks/post-compact.sh`
- [ ] `.claude/hooks/detect-gaps.sh`
- [ ] `.claude/hooks/log-agent.sh` + `log-agent-stop.sh`
- [ ] `.claude/hooks/validate-commit.sh`
- [ ] `.claude/hooks/validate-push.sh`

**Bash (port with substantive rewrite):**
- [ ] `scripts/fw` — reuse the subcommand pattern; rebuild the verify umbrella for Rust/Tauri/Solid (`cargo test`, `cargo fmt`, `cargo clippy`, `pnpm lint`, `pnpm test`, doc/banned-terms, replay-seed determinism, save-migration smoke)
- [ ] `.claude/hooks/canonical-hash-guard.sh` — port pattern; bind to whatever the new sim's canonical-state regression test is named

**.claude/commands (port verbatim with phase-list edits):**
- [ ] `next.md` / `done.md` / `log-decision.md` / `status.md` / `audit.md` / `refresh-docs.md` / `architecture-decision.md` / `gate-check.md` / `milestone-review.md` / `design-system.md` / `design-review.md` / `create-stories.md` / `dev-story.md` / `story-done.md` / `story-readiness.md` / `brainstorm.md` / `quick-design.md` / `balance-check.md` / `code-review.md` / `hotfix.md` / `release-checklist.md` / `bootstrap.md` / `start.md` / `help.md` / `expand-studio.md` / `contract-scope.md` / `deep-research.md`

**.claude/agents (port and reskin):**
- [ ] `creative-director.md` / `game-designer.md` / `systems-designer.md` / `narrative-director.md` / `producer.md` / `lead-programmer.md` / `qa-lead.md`
- [ ] `.claude/agents/README.md` (Tier 1/2/3 structure + escalation map)

**Design-doc carry-forward (port content, rewrite for text-sim scope):**
- [ ] `design/signatures.md` — 24 signatures by role family, lifecycle, stacking, affinity-distribution
- [ ] `design/event-sourced-memory.md` — MemoryEvent schema, 5-input salience, 5 readers, 3-tier compaction, load-time migration
- [ ] `design/player-generation.md` — 22-field gene model, IdentityPacket, 46-label phenotype catalog, compiler pipeline
- [ ] `design/breakthrough-moments.md` — three trigger kinds, two-tier observational text, no-mid-match-QTE
- [ ] `design/scout-disagreement.md` — 3 archetypes, staged-time feedback, conditional-MVP gate, fail-mode taxonomy
- [ ] `design/worldbuilding.md` — fictional nation, 8 regions, six-tier pyramid (20/24/16/14/12/10), three-cup structure, RegionPriors compiler seeding
- [ ] `design/ui-vocabulary.md` — banned-terms lint catalog Categories A-C + sentinel-comment exemption mechanism + commentary template pool structure
- [ ] `design/modding.md` — 12-constraint mod-readiness contract
- [ ] `design/content_policy.md` — PEGI 12 boundary table + AI-content disclosure metadata posture
- [ ] `design/production-pipeline.md` — five-tier CI/CD model + build channels + backup policy
- [ ] `design/match-engine.md` — deterministic sim discipline (re-derive scope for text-tick model)
- [ ] `design/overview.md` — pillars (drop watchable-match), pillar-tiebreaker pattern, MVP boundary

**Specs carry-forward (port pattern, may need re-derivation):**
- [ ] `design/specs/golden-replay-corpus.md` — append-only fixture format + Tier-A/C/D contract + generator-owns-order rule
- [ ] `design/specs/save-migration-fixtures.md` — four-tests-per-bump discipline + per-version fixture accumulation
- [ ] `design/specs/content-pack-validation-contract.md` — FW-VAL ID scheme + red-team-fixture-per-check + failure-message convention
- [ ] `design/specs/football-rules-matrix.md` — football-rules-by-row matrix structure

**ADRs carry-forward (port content, re-author in new stack terms):**
- [ ] `adr-0003-production-pipeline.md`
- [ ] `adr-0004-memory-event-schema.md`
- [ ] `adr-0005-signature-so-schema.md` (rename — no ScriptableObjects)
- [ ] `adr-0006-identity-packet-compiler.md`
- [ ] `adr-0007-scout-archetype-schema.md`

**Templates carry-forward:**
- [ ] `design-templates/architecture-decision-record.md`
- [ ] `design-templates/game-design-document.md`
- [ ] `design-templates/game-pillars.md`
- [ ] `design-templates/test-plan.md`
- [ ] `design-templates/test-evidence.md`
- [ ] `design-templates/playtest-report.md`
- [ ] `design-templates/release-checklist-template.md`
- [ ] `design-templates/release-notes.md`
- [ ] `design-templates/systems-index.md`
- [ ] `design-templates/architecture-traceability.md`
- [ ] `design-templates/postmortem.md`
- [ ] `design-templates/game-concept.md`

**GitHub scaffolding (port and reskin):**
- [ ] `.github/PULL_REQUEST_TEMPLATE.md`
- [ ] `.github/workflows/fast-pr-ci.yml` — three-job pattern (static / cross-OS test matrix / cross-OS reproducibility)
- [ ] `.github/ISSUE_TEMPLATE/`

**Top-level docs (port shape, fresh content):**
- [ ] `CLAUDE.md` — port section structure 1-8 verbatim, rewrite tech-stack + tooling for Rust/Tauri/Solid
- [ ] `PROJECT_CONTEXT.md` — rewrite around the FM-killer text-first thesis
- [ ] `AGENTS.md` — port the 10-line Codex pointer
- [ ] `TOOLING.md` — port catalog shape, rebuild MCP/plugin/CLI entries
- [ ] `.claude/settings.json` (hook + permission map — re-derive)

**Optional carry-forward (port only if Codex-as-phase-gate stays):**
- [ ] `scripts/agent-bus` + `dialog/README.md` + `docs/tooling/agent-bus-spec.md` + the four `.claude/skills/` (duo-debate, duo-implement, codex-review-loop, check-reviews) + `.claude/hooks/protect-dialog-append-only.sh` + `adr-0012-autonomous-implementation-protocol.md`

**Content seeds (port verbatim, possibly re-derive once compiler is in Rust):**
- [ ] `MatchSim/Content/identity-packets/*/*.json` (22 hand-authored IdentityPackets — useful as compiler-input seed data)
- [ ] `MatchSim/Content/archetypes/direct-pressing.yaml` + `low-block-counter.yaml` (manager-archetype schema)
- [ ] `MatchSim.Tests/fixtures/replay-corpus/0xdeadbeefdeadbeef.json` (fixture format reference; hashes dead)

**Target footprint:** the carry-forward list above is ~50 files. Add an empty `SPEC.md` + `STATUS.md` + `CHANGELOG.md` + a new `Cargo.toml` + a Tauri/Solid scaffold and the new repo starts under the 50-file docs+config ceiling.

# Bootstrap installables catalog

> Read by Claude during `/bootstrap` Phase B-C. Single source of truth for every MCP / plugin / Unity package / Asset Store asset Claude might install, with precise install commands.

## How this file is used

For each project, Claude picks a **subset** of items below based on intake answers (see CUSTOMIZATION.md for the mapping). Claude then:
1. Runs `claude mcp list` to check what's already installed (user-scoped)
2. Installs missing items via `claude mcp add -s <scope> ...`
3. Writes plugin commands to `scripts/install-plugins.txt` for user to paste into the session
4. Adds Unity packages + Asset Store items to `SPEC.md` Phase 3 task list (not installed at bootstrap — Phase 3 concern)

---

## Section 1 — MCPs

### User-scoped (install once per machine)

Detect-before-install: `claude mcp list | grep -q "^<name>"` to skip if already present.

#### desktop-commander
```bash
claude mcp add -s user desktop-commander "npx -y @wonderwhy-er/desktop-commander"
```
**Purpose:** filesystem + process control beyond Bash scope.
**Always install.** No project is too small for this.

#### context7
```bash
claude mcp add -s user context7 "npx -y @context7/mcp-server"
```
**Purpose:** library docs lookup. Better than web search for API questions.
**Always install.**

#### github
```bash
claude mcp add -s user github "npx -y @modelcontextprotocol/server-github"
```
**Purpose:** repo / issue / PR / release workflow.
**Install if:** `commercial_intent != private` (anything pushing to GitHub). Prereq: user has run `gh auth login`.

#### chrome
```bash
claude mcp add -s user chrome "npx -y @sparfenyuk/mcp-chrome"
```
**Purpose:** web-app interaction + research.
**Install always** (research is universal).

### Project-scoped (install per-project)

#### unity-mcp (CoplayDev)
Two-part install:
1. Unity side: add to `Packages/manifest.json` in Phase 3 (not at bootstrap — Unity project doesn't exist yet):
   ```json
   "com.coplaydev.unity-mcp": "https://github.com/CoplayDev/unity-mcp.git?path=/MCPForUnity"
   ```
2. Claude side (deferred to Phase 3 Unity Bootstrap task):
   ```bash
   # Run after Unity project exists:
   claude mcp add -s project unity-mcp <launcher-command-from-CoplayDev-docs>
   ```

**At bootstrap:** do NOT install. Add to SPEC.md Phase 3 task list as a checkbox.

### MCPs deferred to trigger

Add to `SETUP.md §10` trigger table, don't install at bootstrap:

| MCP | Trigger |
|---|---|
| AI Forge (Blender pipeline) | Phase 5+ Blender bottleneck |
| Figma MCP (if UI-heavy) | Phase 6 UI pass needs Figma sync |
| Steam MCP (if available) | Phase 8 Steam submission |

---

## Section 2 — Claude Code plugins

Plugins install via `/plugin install <name>` inside Claude Code — **cannot be run as a tool call by Claude**. At bootstrap, Claude writes these to `scripts/install-plugins.txt` for the user to paste.

### Always recommend

- `/plugin install feature-dev` — code-architect + code-explorer + code-reviewer. Any non-trivial feature benefits.
- `/plugin install pr-review-toolkit` — code-reviewer, silent-failure-hunter, type-design-analyzer, etc. Pre-merge QA essentials.
- `/plugin install hookify` — convert conversation mistakes into hooks. High value over project lifetime.

### Recommend if narrative game

- None — per-character voice bibles are a pattern, not a plugin.

### Recommend if author-of-Claude-plugins later

- `/plugin install plugin-dev` — for authoring custom plugins.

### Skip list (evaluated + rejected)

Document in `TOOLING.md §8` (anti-patterns). Catches future Claude sessions from re-re-installing.

---

## Section 3 — Claude Code skills

### Ships in-project (pre-installed under `.claude/skills/`)

Blueprint v2 ships these skills in the skeleton — no install step required:

| Skill | Folder | Purpose |
|---|---|---|
| `unity-check` | `.claude/skills/unity-check/` | 3-level Unity verification (L1 compile / L2 runtime / L3 visual). Invoke before any "scene works" claim. |
| `state-dump` | `.claude/skills/state-dump/` | McpRemoteControl god-mode runtime snapshot. Dumps live scene graph, SO values, Ink state. |
| `unity-webgl-builder` | `.claude/skills/unity-webgl-builder/` | Headless WebGL build pipeline (Phase 3 handoff + Phase 8 demo). |
| `github-pages-deploy` | `.claude/skills/github-pages-deploy/` | Push WebGL builds to GitHub Pages for internal review. |
| `unity-audio-generator` | `.claude/skills/unity-audio-generator/` | Procedural audio stub generator (Phase 5-6 iteration). |

Skills trigger automatically per their description; don't pre-announce them.

### External skills available on-demand

- `anthropic-skills:skill-creator` — if user wants to author custom skills for this project
- `anthropic-skills:pptx` / `docx` / `xlsx` / `pdf` — document workflows
- `anthropic-skills:canvas-design` — design asset workflows (Phase 5-8 marketing)
- `anthropic-skills:internal-comms` — dev blog / team comms (Phase 8+)
- `anthropic-skills:schedule` — recurring task scheduling

**Most external skills surface via the `anthropic-skills` plugin if installed.** Don't individually install — let them trigger on-demand.

---

## Section 3.5 — Agent roster

Blueprint v2 ships 14 core agents under `.claude/agents/` in the skeleton. Every project gets the full roster:

| Tier | Agent | Role |
|---|---|---|
| Director | `creative-director` | Vision + pitch + narrative arc |
| Director | `technical-director` | Architecture + risk + tech strategy |
| Director | `producer` | Phase gates + scope discipline + sprint rhythm |
| Director | `art-director` | Visual target + pipeline decisions |
| Director | `narrative-director` | Voice consistency + cast coherence |
| Director | `qa-lead` | Test strategy + gate criteria |
| Lead | `lead-programmer` | Code standards + architecture reviews |
| Lead | `game-designer` | Mechanics + balance + feel |
| Specialist | `systems-designer` | Economy + progression + emergent loops |
| Specialist | `gameplay-programmer` | Combat + player character + input |
| Specialist | `engine-programmer` | Performance + memory + build pipeline |
| Specialist | `ui-programmer` | HUD + menu frameworks |
| Specialist | `unity-specialist` | Unity-idiomatic patterns, asmdef, Addressables |
| Specialist | `unity-ui-specialist` | UI Toolkit / UGUI specifics |

**Scope visibility:** `minimal` / `standard` scopes make these available but not proactively announced; `rich` / `studio` / `research` scopes surface them in `/help` and may auto-delegate.

**Extended roster** (loaded only at `studio` / `research` scope via `/expand-studio`): audio-director, ai-programmer, technical-artist, level-designer, writer, world-builder, accessibility-specialist, devops-engineer, unity-shader-specialist, unity-dots-specialist, unity-addressables-specialist, localization-lead. These do not ship as files in the skeleton — they're spawned on demand via the Task tool when scope permits.

---

## Section 3.6 — Document templates

Blueprint v2 ships 14 design templates under `templates/design-templates/` in the skeleton. Projects reference (not copy) these at design time:

| Template | When referenced |
|---|---|
| `game-concept.md` | Phase 0 — initial pitch refinement |
| `game-pillars.md` | Phase 0 — lock 3-5 pillars |
| `game-design-document.md` | Phase 2 — canonical GDD skeleton |
| `systems-index.md` | Phase 2-6 — canonical list of systems |
| `ux-spec.md` | Phase 5-6 — per-screen UX specs |
| `hud-design.md` | Phase 5-6 — HUD layout + feedback |
| `architecture-decision-record.md` | Any — one ADR per locked decision |
| `architecture-traceability.md` | Phase 3+ — trace requirements to code |
| `test-plan.md` | Phase 5+ — gate criteria |
| `test-evidence.md` | Phase 7+ — gate-check artifacts |
| `playtest-report.md` | Phase 5+ — each external playtest |
| `release-checklist-template.md` | Phase 8 — launch gate |
| `release-notes.md` | Phase 8-9 — per-build notes |
| `postmortem.md` | Phase 9 — after launch |

**Bootstrap behavior:** do NOT copy templates into project root. They stay under `templates/design-templates/` as references; Claude reads and applies them when the relevant phase task hits.

---

## Section 3.7 — Asset pipelines

Blueprint v2 ships 10 pipeline stubs under `asset-pipelines/` in the skeleton. **Usage is deferred until phase triggers fire** — at bootstrap, Claude adds them to the trigger table but does not activate any.

| Pipeline stub | Trigger |
|---|---|
| `char-build.md` | Phase 4 — first character authoring |
| `scene-build.md` | Phase 3-5 — scene building at scale |
| `outfit-bake.md` | Phase 4-5 — outfit variant pipeline |
| `cg-render.md` | Phase 5-6 — CG / cutscene rendering |
| `sfx-gen.md` | Phase 5-6 — SFX pass |
| `music-gen.md` | Phase 5-6 — music pass |
| `vo-gen.md` | Phase 6-7 — VO pass (if narrative-heavy) |
| `trans-gen.md` | Phase 7 — localization |
| `3d-asset-gen.md` | Phase 3-5 — procedural/AI 3D asset generation |
| `2d-asset-gen.md` | Phase 3-7 — 2D asset / portrait pipeline |

Flagging: for each pipeline relevant to intake (e.g., narrative-heavy → vo-gen), Claude adds a one-line note in SPEC.md Phase N's task list pointing at the stub.

---

## Section 3.8 — Patterns library

Blueprint v2 ships ~25 patterns under `patterns/`. Split across scope tiers:

**Core patterns (visible at all scopes):**
- `game-prompt-engineering.md`, `fsm.md`, `event-driven.md`, `object-pooling.md`, `dependency-injection.md`, `behavior-trees.md`, `spatial-partitioning.md`, `unit-testing.md`, `save-load.md`, `accessibility.md`, `ecs-intro.md`, `narrative-subagent-pattern.md`

**Phase/workflow patterns (visible at standard+):**
- `phase-gate-workflow.md`, `budget-tier-trigger-table.md` (studio proactive-loads these)

**Studio-tier patterns (surfaced at studio+ scope):**
- `game-ai-runtime/` subtree — `claude-runtime.md`, `dialogue-runtime.md`, `npc-memory-pattern.md`, `safety-filters.md`
- `save-system/` subtree — `json-serializer.md`, `version-migration.md`, `cloud-sync.md`
- `community-feedback-automation.md`, `analytics-opt-in.md`

**Additional studio references:**
- `steam-release/marketing-automation.md`, `steam-release/platform-build-matrix.md`
- `unity/scene-validation-patterns.md`, `unity/unity-test-framework-setup.md`, `unity/unity-profiler-integration.md`

At bootstrap, Claude does not activate any pattern — patterns are reference material. Claude reads them when the matching task triggers.

---

## Section 4 — Unity packages (via Packages/manifest.json at Phase 3)

Claude doesn't install these at bootstrap. It updates `SPEC.md` Phase 3 "Install Unity packages" task with the specific list per project.

**Scope + tier influence on package selection:** if `default_scope = studio` or `research`, assume the project will need broader editor tooling (Odin Inspector, Unity Profiler integration per `unity/unity-profiler-integration.md`, Unity Test Framework per `unity/unity-test-framework-setup.md`). Add those to the Phase 3 trigger table with lower trigger thresholds.

If `context_window = 200K` (tighter context), no Unity package change — tier affects Claude Code orientation, not Unity runtime.

### Always add to Phase 3 task list

```json
"com.cysharp.unitask": "https://github.com/Cysharp/UniTask.git?path=src/UniTask/Assets/Plugins/UniTask",
"com.unity.addressables": "2.9.1",
"com.unity.animation.rigging": "1.3.0",
"com.unity.recorder": "5.1.2",
"com.coplaydev.unity-mcp": "https://github.com/CoplayDev/unity-mcp.git?path=/MCPForUnity"
```

### Add if `narrative_weight >= medium`

```json
"com.inkle.ink-unity-integration": "https://github.com/inkle/ink-unity-integration.git?path=Packages/Ink"
```

### Add if `character_fidelity = 3d_anime` + `character_pipeline = vroid`

```json
"com.vrmc.univrm": "https://github.com/vrm-c/UniVRM.git?path=/Packages/VRM",
"com.vrmc.gltf": "https://github.com/vrm-c/UniVRM.git?path=/Packages/UniGLTF",
"jp.lilxyzw.liltoon": "https://github.com/lilxyzw/lilToon.git?path=Assets/lilToon#2.3.2"
```

### Add if localization planned

```json
"com.unity.localization": "1.5.x"
```

---

## Section 5 — Asset Store assets (purchase gated by SETUP.md §10 trigger table)

Claude adds these to the trigger table with specific pain points, NOT purchased at bootstrap.

### Character-pipeline

- **Animancer Pro** ($80) — trigger: Mecanim state graph breakdown, Phase 4+
- **FinalIK** ($90) — trigger: hand-contact / foot-IK needed, Phase 4+
- **Auto-Rig Pro** ($40 Blender addon) — trigger: Mixamo auto-bind fails, Phase 4+

### Cloth / physics

- **Magica Cloth 2** ($50) — trigger: first cloth-sim scene, Phase 4-5
- **Obi Cloth** ($110) — trigger: tearable cloth needed (rare), Phase 5+

### Editor UX

- **Odin Inspector** ($55) — trigger: SO authoring pain at Phase 3+
- **Rainbow Folders** (free) — install opportunistically if user wants

### Environment

- Genre-specific asset bundles — add to trigger table with "first venue needs dressing" trigger

### Visual

- **Amplify Shader Editor** ($80) — trigger: Shader Graph can't express needed effect
- **Substance Painter** (Adobe sub) — trigger: manual texture iteration too slow
- **lilToon** (free, git-URL) — add automatically if `3d_anime` character fidelity

---

## Section 6 — CLIs expected on host

Claude checks these, reports any missing (user installs, not Claude):

```bash
command -v gh       || echo "MISSING: gh (brew install gh)"
command -v git-lfs  || echo "MISSING: git-lfs (brew install git-lfs)"
command -v jq       || echo "MISSING: jq (brew install jq)"
command -v node     || echo "MISSING: node (brew install node)"
command -v python3  || echo "MISSING: python3 (system)"
```

At bootstrap, run these checks. If any missing, note in the handoff message as manual-user-action.

---

## Section 7 — External accounts + tier prerequisites

Claude lists these in the handoff message. User creates accounts manually:

| Account | When needed | Cost |
|---|---|---|
| GitHub | Phase 1 | free |
| Unity ID | Phase 1 | free |
| Steam Direct | Phase 8 | $100 one-time |
| Apple Dev | Phase 8 if Mac App Store | $99/yr |
| Adobe ID (Mixamo) | Phase 4 if character pipeline uses Mixamo | free |
| itch.io | optional | free |

**Context tier prerequisites:**

| Intake answer | Required on-disk file | Bootstrap action |
|---|---|---|
| `context_window = 1M` | `~/.claude/tier-capabilities.json` with `context_window: 1M` | Create from `tier-capabilities.json.template` if missing |
| `context_window = 200K` or `unknown` | no requirement | — |
| `default_scope = research` + `context_window = 200K` | — | Flag conflict; downgrade to `rich` with a note |

**Scope prerequisites:**

| Scope | Minimum context tier | Note |
|---|---|---|
| `minimal` / `standard` | 200K | No prerequisites |
| `rich` | 200K (works, tight) / 1M (recommended) | Standard default for 1M |
| `studio` | 1M recommended | Proactive-load ~300KB eats 200K budget |
| `research` | 1M required | ~800KB proactive-load; only viable on 1M tier |

---

## Section 8 — Services (post-launch, never at bootstrap)

- **Sentry / Backtrace** (crash reporting) — Phase 7-8
- **Discord server** (community) — Phase 8 marketing
- **Newsletter** (if doing dev-blog marketing) — Phase 7-8
- **Analytics** (opt-in only) — Phase 7-8

Never installed at bootstrap. Mentioned in `SETUP.md` trigger table for future reference.

---

## How Claude uses this file

Pseudocode for the install phase:

```
intake = parse intake answers
profile = match profile from PROFILES.md or compose custom

# User-scoped MCPs (machine-level)
existing_mcps = bash("claude mcp list")
for mcp in ["desktop-commander", "context7", "chrome"]:
    if mcp not in existing_mcps:
        bash(install_command_from_this_file)

if intake.commercial_intent != "private" and "github" not in existing_mcps:
    if bash("gh auth status").returncode != 0:
        user_message("Please run `gh auth login` in another terminal, then tell me when done")
        wait
    bash("claude mcp add -s user github ...")

# Project-scoped MCPs — DEFER unity-mcp to Phase 3 (project doesn't exist yet)
# Write note to SPEC.md

# Plugins — cannot be tool-called; queue for user
plugin_commands = []
plugin_commands.append("/plugin install feature-dev")
plugin_commands.append("/plugin install pr-review-toolkit")
plugin_commands.append("/plugin install hookify")
# ... etc based on intake
write_file(".claude/bootstrap/scripts/install-plugins.txt", plugin_commands)

# Unity packages — update SPEC.md Phase 3 task with project-specific list
unity_packages = ["UniTask", "Addressables", ...]  # always
if intake.narrative_weight >= medium:
    unity_packages.append("Ink-Unity-Integration")
# ... etc
edit_spec_md_phase_3_package_task(unity_packages)

# CLI checks — report missing, don't auto-install
missing_clis = []
for cli in ["gh", "git-lfs", "jq", "node", "python3"]:
    if bash(f"command -v {cli}").returncode != 0:
        missing_clis.append(cli)

# Accounts — list in handoff, never automate
accounts_needed = derive_from_intake(intake)

# Asset Store assets — populate SETUP.md §10 trigger table with project-specific pain points
edit_setup_md_trigger_table(intake)
```

Result: Claude installs everything automatable, queues everything that requires user interaction, and leaves a clean handoff message.

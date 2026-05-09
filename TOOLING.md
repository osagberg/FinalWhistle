# TOOLING.md — Final Whistle MCP / plugin / subagent / CLI / hook catalog

> Canonical list of every tool Claude Code can use in this project. Adopt/skip decisions live here. Anti-patterns (evaluated and rejected) at the bottom.
>
> Authored 2026-04-22. Adopt/skip decisions logged in SPEC.md; this file catalogs.

---

## 1. MCP servers

### Baseline (installed at bootstrap — user-scope)

| Server | Status | Install | Purpose |
|---|---|---|---|
| **context7** | ✅ active | `claude mcp add -s user context7 ...` (already present; skipped) | Library docs lookup; beats web search for API questions |
| **github** | ✅ active | already present | Repo / issue / PR / release workflow |
| **blender-mcp** | ✅ active | already present | Hunyuan3D + Hyper3D text-to-3D access, PolyHaven / Sketchfab browsing. Deferred-3D pipeline tool. |

### Intentionally skipped at bootstrap

| Server | Reason |
|---|---|
| **chrome** | Claude Code has `WebSearch` + `WebFetch` tools natively. Redundant install. |
| **desktop-commander** | Claude Code has `Read` / `Write` / `Edit` / `Bash` tools natively. Redundant install. |

### Phase 3 — Unity-specific

> **Routing authority:** [`docs/tooling/unity-mcp-routing.md`](docs/tooling/unity-mcp-routing.md) (per [ADR-0011](design/adr/adr-0011-unity-ai-assistant-mcp-migration.md)).
> **Read it on every subagent invocation that may touch the Editor.**

| Server | Status | Install | Purpose |
|---|---|---|---|
| **UnityAIAssistant** (`com.unity.ai.assistant 2.7.0-pre.3`) — **PRIMARY** | ✅ active (2026-05-09) | Package in `unity-project/Packages/manifest.json` (pinned `2.7.0-pre.3`); relay binary auto-installed by Editor on first run to `~/.unity/relay/relay_mac_arm64.app/...` (Apple Silicon; equivalents on Intel/Windows/Linux); registered in `.mcp.json` as stdio relay with `--mcp --project-path /Users/vibelogic/dev/football/unity-project`. **Requires active Unity Pro/Enterprise seat** (`MaxDirect = 1` direct connection slot per seat). Approval flow: `Edit > Project Settings > AI > Unity MCP Server` → Pending Connections → Allow `claude-code`. | 52 first-party tools: `Unity_RunCommand` (C# in-Editor execution with `IRunCommand` template), `Unity_ManageEditor`, `Unity_ManageMenuItem` (execute any `[MenuItem]` by path), `Unity_SceneView_Capture2DScene` / `Unity_SceneView_CaptureMultiAngleSceneView` / `Unity_Camera_Capture` (visual L2 evidence), 11 `Unity_Profiler_*` tools, `Unity_AssetGeneration_*` (32 first-party AI models — Flux 2, Gemini 3.x, GPT Image 1.5, Tripo P1, ElevenLabs, Lyria 3, etc., **bake-time only**), semantic C# edits via `Unity_ScriptApplyEdits`, `Unity_GetSha`, `Unity_FindInFile`, `Unity_Grep`. See [routing table](docs/tooling/unity-mcp-routing.md) + [playbook](docs/tooling/unity-mcp-playbook.md). |
| **UnityMCP** (CoplayDev `com.coplaydev.unity-mcp`) — **FALLBACK** | ✅ active (2026-04-28; retained as fallback per ADR-0011) | Package via `unity-project/Packages/manifest.json` (pinned to commit SHA `b92c05a25820cfc9f59ce4094eb46aaec8632ea2`) + Unity-side `Window → MCP for Unity → Configure` writes the project-scoped `.mcp.json` entry as `UnityMCP` on `http://localhost:8080/mcp`. **No PostToolUse refresh hook needed** — `manage_script` already calls `AssetDatabase.ImportAsset` + `RequestScriptCompilation` internally. | Fallback for capability gaps the official MCP doesn't cover: `manage_packages` (UPM install/remove — official `Unity_PackageManager_ExecuteAction` is off-by-default until user-toggled), `manage_prefabs` / `manage_animation` / `manage_vfx` / `manage_probuilder` (granular APIs not in official), `batch_execute` (transactional). Also entitlement-failure recovery if Pro seat lapses. Retirement gates documented in [routing table](docs/tooling/unity-mcp-routing.md#deprecation-candidates-coplaydev-retirement-gates). |

### Deferred (trigger-based; Phase-5/6 3D spike or later fallback)

| Server | Trigger | Notes |
|---|---|---|
<!-- ui-lint:ignore-start reason="deferred MCP service named 'AI Forge' — proper noun, not verb usage" -->
| AI Forge | Phase-5 3D-pipeline spike evaluation | Blender pipeline tier-selection; install only if it materially improves the spike |
<!-- ui-lint:ignore-end -->
| Additional 3D MCPs | Phase-5/6 3D spike need | Evaluate only after the license audit identifies a concrete workflow gap |

---

## 2. Claude Code plugins (installed)

Verified at `~/.claude/plugins/installed_plugins.json` 2026-04-30. Slash commands + subagents are live in every Claude Code session opened in this project folder.

| Plugin | Status | Purpose | Why for this project |
|---|---|---|---|
| **feature-dev** | ✅ active | code-architect + code-explorer + code-reviewer subagents | MatchSim + viewer are non-trivial systems; subagents essential |
| **pr-review-toolkit** | ✅ active | code-reviewer, silent-failure-hunter, type-design-analyzer, pr-test-analyzer, comment-analyzer, code-simplifier | Pre-merge QA; mandated by CLAUDE.md §6.3 on substantial code commits; reminded by `.claude/hooks/pr-review-reminder.sh` (non-blocking) |
| **hookify** | ✅ active | Create hooks from unwanted-behavior analysis | Used 2026-04-30 to author the `pr-review-reminder.sh` enforcement reminder pattern |

### Evaluated and skipped

| Plugin | Decision | Reason |
|---|---|---|
| **plugin-dev** | skip | Not authoring Claude plugins for this project |
| **anthropic-skills** | skip at MVP | Document workflows (pptx/docx/etc.) not needed for Final Whistle MVP. Reconsider at Phase 8 marketing. |

---

## 3. Subagents (via `Agent` tool)

**Built-in:**

| Subagent | Use case |
|---|---|
| `general-purpose` | Open-ended research, multi-step |
| `Explore` | Codebase exploration (fast, read-only; always prefer for >3-query research) |
| `Plan` | Implementation strategy for complex features |

**From blueprint v2 project agents (`.claude/agents/`):**

- `creative-director` — vision / pitch / tone integrity
- `technical-director` — architecture / tech risk / framework decisions
- `producer` — phase gates / scope discipline / sprint rhythm
- `art-director` — visual target, dots polish bar, Phase-5 3D spike review
- `narrative-director` — event-sourced memory templates / salience tuning
- `qa-lead` — test strategy / gate criteria
- `lead-programmer` — code standards / architecture reviews
- `game-designer` — mechanics / balance / feel
- `systems-designer` — economy / progression / emergent loops (heavy rotation for this project)
- `gameplay-programmer` — match-sim / signatures / player systems (heavy rotation)
- `engine-programmer` — performance / memory / MatchSim optimization (heavy rotation)
- `ui-programmer` — HUD / menu frameworks (heavy rotation — FM26-regression-anti-pattern focus)
- `unity-specialist` — Unity-idiomatic patterns, asmdef, Addressables
- `unity-ui-specialist` — UI Toolkit UXML/USS specifics (heavy rotation — management UI is load-bearing)

**From feature-dev plugin (installed via slash command):**

- `code-architect`, `code-explorer`, `code-reviewer`

**From pr-review-toolkit plugin:**

- `silent-failure-hunter`, `type-design-analyzer`, `pr-test-analyzer`, `comment-analyzer`, `code-simplifier`

---

## 4. Slash commands

Installed at `.claude/commands/` (31 shipped with blueprint v2):

Base six:
- `/next`, `/done`, `/status`, `/audit`, `/refresh-docs`, `/log-decision`

Extended (25 more, available on-demand per scope):
- Sprint / phase management
- Architecture authoring
- Review commands
- Production coordination
- Release preparation

See `.claude/commands/` for full manifest.

---

## 5. Hooks

At `.claude/hooks/` (13 installed):

| Hook | Event | Purpose |
|---|---|---|
| `protect-decisions-log.sh` | PreToolUse (Edit/Write) | Enforce SPEC.md decisions log append-only |
| `update-status-timestamp.sh` | Stop | Rewrite STATUS.md "Last updated" |
| Plus 11 more shipped with blueprint v2 (session, validation, cross-platform) | | |

Unity MCP refresh — handled by the tools themselves, not a hook:
- **`UnityAIAssistant`** (official, primary): `Unity_ScriptApplyEdits` / `Unity_ApplyTextEdits` / `Unity_CreateScript` / `Unity_DeleteScript` all trigger asset re-import + script compilation internally. Use `Unity_ManageEditor GetState` to poll `IsCompiling` / `IsUpdating` after a script change before invoking the next tool.
- **`UnityMCP`** (CoplayDev, fallback): `manage_script` calls `AssetDatabase.ImportAsset(... ForceSynchronousImport | ForceUpdate)` + `CompilationPipeline.RequestScriptCompilation()` internally on every script edit. The previous `refresh-unity-on-script.sh` PostToolUse hook (matched `mcp__UnityMCP__manage_script`) was removed in commit `47997fc` per Codex audit P2-04 confirming it was redundant.

---

## 6. CLIs verified at bootstrap

| CLI | Status | Install | Purpose |
|---|---|---|---|
| `gh` | ✅ present | `brew install gh` | GitHub workflow |
| `git` | ✅ present | system | Source control |
| `git-lfs` | ✅ present | `brew install git-lfs` | Unity binary assets |
| `jq` | ✅ present | `brew install jq` | JSON munging in scripts |
| `node` / `npx` | ✅ present | `brew install node` | MCP servers |
| `python3` | ✅ present | system | Hook scripts |
| `unity` | not yet | Unity Hub at Phase 3 | batchmode builds |

---

## 7. External services

| Service | Account | Monthly | Use case |
|---|---|---|---|
| GitHub | osagberg (personal namespace; `vibelogic` org reserved, Phase-8 transfer optional per CLAUDE §5.6) | free | source + CI |
| Steam Direct | TBD Phase 8 | $100 one-time | per-title |
| Unity Asset Store | Unity ID | per-asset | Animancer / DOTween / Odin |
| OpenAI (GPT Image 2) | existing | existing | concept art / portraits |
| Anthropic (Claude API) | existing | existing (prompt-caching economics) | AI Content Compiler at bake time |
| Sentry / crash reporting | TBD Phase 7 | free tier | post-launch observability |

Deferred until trigger:
- Tripo v3.0 / Rodin (Hyper3D) / Cascadeur — Phase-5 commercial-license audit + spike only; Phase-6 production only after spike-green
- Suno / Udio — Phase 6 music pass if audio direction needs it
- ElevenLabs — Post-EA only, conditional on player demand for commentary VO

---

## 8. Anti-patterns — hard no-list

Tools evaluated and rejected. Includes the WHY so future sessions don't re-re-evaluate.

- **Stale / unmaintained Unity MCPs** — anything updated <6 months ago. Unity ecosystem moves fast.
- **Multiple Unity MCPs simultaneously, both active in tool dispatch** — they don't conflict on transport (official is stdio relay; CoplayDev is HTTP :8080), but they will fight for the same Editor main thread under load. The migration documented in [ADR-0011](design/adr/adr-0011-unity-ai-assistant-mcp-migration.md) intentionally retains both registered in `.mcp.json` — official as primary, CoplayDev as fallback — but routing must obey [`docs/tooling/unity-mcp-routing.md`](docs/tooling/unity-mcp-routing.md). Do not invoke parallel tool calls hitting both servers in the same logical operation.
- **Community C# hot-reload plugins with <50 stars** — editor instability risk.
- **Ollama-backed Unity MCPs** — latency unusable vs local tools.
- **Token-spam skills** — any skill that imports multi-KB docs on session start. Prefer lazy-load patterns.
- **Ink / Yarn Spinner** — narrative is event-sourced systemic, not scripted branching.
- **ML-Agents for tactical AI** — training cost + opacity + non-determinism. Behavior trees + hand-authored archetypes win.
- **Runtime LLMs (local or cloud)** — inference cost breaks match-day flow. Bake-time only.
- **Unity PhysX in canonical MatchSim path** — not deterministic enough for cross-platform replay. Custom fixed-point sim only.
- **uGUI for primary UI** — UI Toolkit is the target; uGUI only as fallback for documented UIT bugs.
- **VRoid / UniVRM pipeline** — not in scope. Phase-5/6 3D-pipeline candidate stack (per `design/3d-pipeline.md`) uses 3D-asset-generator tools (Tripo / Rodin / Hunyuan3D candidates) + Blender rigging + AI-assisted animation tool (Cascadeur candidate) + URP cel-shader, not the VRM pipeline. Revisitable via new decisions-log entry if Phase-5 license-audit + spike-feasibility identifies VRM as a better fit than current candidate stack.
- **Monthly subscriptions before their phase trigger** — explicit hard rule; exceptions require decisions-log entry. 3D tooling triggers start at Phase-5 license-audit/spike, not before.
<!-- ui-lint:ignore-start reason="anti-patterns list explicitly naming banned vocabulary" -->
- **"The Hush" / "Weather" / "Calling" / "Canon" / "Seven" / capitalized mystical vocabulary** — never ships as visible system names. Football-native UI copy only.
<!-- ui-lint:ignore-end -->

---

*Authored 2026-04-22. Adopt/skip decisions go in SPEC.md decisions log; this file just catalogs.*

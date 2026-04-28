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

| Server | Status | Install | Purpose |
|---|---|---|---|
| **UnityMCP** (CoplayDev `com.coplaydev.unity-mcp`) | ✅ active (2026-04-28) | Package via `unity-project/Packages/manifest.json` (pinned to commit SHA) + Unity-side `Window → MCP for Unity → Configure` writes the project-scoped `.mcp.json` entry as `UnityMCP` (Pascal case) on `http://localhost:8080/mcp`. CoplayDev distinguishes the npm/UPM package name (`com.coplaydev.unity-mcp`) from the MCP server registration name (`UnityMCP`); both refer to the same tool, in different contexts | Edit scenes + read console + drive Editor APIs + run tests from Claude. **No PostToolUse refresh hook needed** — `manage_script` already calls `AssetDatabase.ImportAsset` + `RequestScriptCompilation` internally, so script edits made through MCP auto-import without manual focus. (The earlier `refresh-unity-on-script.sh` hook was removed in commit `47997fc` after Codex audit P2-04 confirmed it was redundant.) |

### Deferred (trigger-based; Phase-5/6 3D spike or later fallback)

| Server | Trigger | Notes |
|---|---|---|
<!-- ui-lint:ignore-start reason="deferred MCP service named 'AI Forge' — proper noun, not verb usage" -->
| AI Forge | Phase-5 3D-pipeline spike evaluation | Blender pipeline tier-selection; install only if it materially improves the spike |
<!-- ui-lint:ignore-end -->
| Additional 3D MCPs | Phase-5/6 3D spike need | Evaluate only after the license audit identifies a concrete workflow gap |

---

## 2. Claude Code plugins (queued for install)

Install via `/plugin install <name>` inside a fresh Claude Code session opened in the project folder.

| Plugin | Purpose | Why for this project |
|---|---|---|
| **feature-dev** | code-architect + code-explorer + code-reviewer subagents | MatchSim + viewer are non-trivial systems; subagents essential |
| **pr-review-toolkit** | code-reviewer, silent-failure-hunter, type-design-analyzer, pr-test-analyzer, comment-analyzer, code-simplifier | Pre-merge QA; MatchSim determinism needs type-design rigor |
| **hookify** | Create hooks from unwanted-behavior analysis | When conversation mistakes should become hooks |

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

UnityMCP refresh — handled by the tool itself, not a hook:
- The previous `refresh-unity-on-script.sh` PostToolUse hook (matched `mcp__UnityMCP__manage_script`) was removed in commit `47997fc` per Codex audit P2-04. CoplayDev's `manage_script` tool already calls `AssetDatabase.ImportAsset(... ForceSynchronousImport | ForceUpdate)` + `CompilationPipeline.RequestScriptCompilation()` internally on every script edit, so the hook was redundant. No PostToolUse refresh wiring is needed.

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
- **Multiple Unity MCPs simultaneously** — they conflict on message channels.
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

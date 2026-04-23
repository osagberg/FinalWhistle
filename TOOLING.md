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

### Phase 3 — Unity-specific (deferred)

| Server | Install | Purpose |
|---|---|---|
| **unity-mcp** (CoplayDev) | Package via `Packages/manifest.json` + `claude mcp add -s project unity-mcp <launcher>` | Edit scenes + read console + run tests from Claude. Install after Unity project exists. |

### Deferred (trigger-based; post-EA 3D push)

| Server | Trigger | Notes |
|---|---|---|
| AI Forge | Phase 9 post-EA 3D push | Blender pipeline tier-selection |
| Additional 3D MCPs | Post-EA 3D R&D | Evaluate at Phase 9 if audience signal justifies |

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
- `art-director` — visual target (minimal role at 2D MVP; activates at Phase 9 3D push)
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

At `.claude/hooks/` (14 installed):

| Hook | Event | Purpose |
|---|---|---|
| `protect-decisions-log.sh` | PreToolUse (Edit/Write) | Enforce SPEC.md decisions log append-only |
| `update-status-timestamp.sh` | Stop | Rewrite STATUS.md "Last updated" |
| Plus 9 more shipped with blueprint v2 (session, validation, cross-platform) | | |

Phase 3 activation note:
- `refresh-unity-on-script.sh` already exists, but only becomes useful after Unity MCP is installed and the Unity project exists.

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
| GitHub | Vibelogic | free | source + CI |
| Steam Direct | TBD Phase 8 | $100 one-time | per-title |
| Unity Asset Store | Unity ID | per-asset | Animancer / DOTween / Odin |
| OpenAI (GPT Image 2) | existing | existing | concept art / portraits |
| Anthropic (Claude API) | existing | existing (prompt-caching economics) | AI Content Compiler at bake time |
| Sentry / crash reporting | TBD Phase 7 | free tier | post-launch observability |

Deferred to post-EA 3D push:
- Tripo v3.0, Rodin (Hyper3D), Suno/Udio, Cascadeur, ElevenLabs

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
- **VRoid / UniVRM pipeline** — no 3D at MVP; defer indefinitely.
- **Monthly subscriptions before Phase 9** — explicit hard rule; exceptions require decisions-log entry.
- **"The Hush" / "Weather" / "Calling" / "Canon" / "Seven" / capitalized mystical vocabulary** — never ships as visible system names. Football-native UI copy only.

---

*Authored 2026-04-22. Adopt/skip decisions go in SPEC.md decisions log; this file just catalogs.*

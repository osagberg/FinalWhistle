# CLAUDE.md — Final Whistle project contract

> **Read this first in every new session.** Authoritative onboarding contract. What the project is, how it's decided, what Claude must respect, where canonical info lives.
>
> This file is SHORT by design. It does not duplicate other docs — it points at them.
>
> Authored 2026-04-22. Fork of `~/dev/blueprint/templates/CLAUDE.md`.

---

## 1. What this project is

**Final Whistle is a football management RPG where careers remember.** Build a club, develop signature players, survive season-defining moments, and watch old decisions return years later as rivals, legends, regrets, and revenge. PC / Steam only. PEGI 12.

**Working title:** Final Whistle
**Studio:** Vibelogic
**Genre:** sports / management / sim / RPG-progression
**Target platforms:** Windows + Mac + Linux
**Commercial target:** Steam EA → 1.0 ($20 EA → $30 1.0)
**Target audience:** FM-disillusioned players + anime-sports-curious 18-35, primary vector via management-sim depth + emotional memory, not licensed realism.

The game is simultaneously a shippable product and a proof-point for a solo AI-native production. Every asset goes through bake-time AI pipelines; the thesis is "one human + Claude + modern tools can ship something genuinely good end-to-end." Tone anchor: Giant Killing + Aoashi + occasional anime exaggeration. Grounded football first; heightened moments second.

**Core loop (what the player does):** Sign/develop players → set tactics → play matches (semantic-cinema 7-shot-type camera grammar; rendered via the active viewer adapter — dots-prototype Phase-3-onward, candidate cel-shaded 3D Phase-5/6+ pending spike) → respond to events that remember past choices → deal with consequences seasons later.

**Unique selling points (Phase 0 refinement candidates):**
- **Careers that remember** — event-sourced memory ledger surfaces old decisions as NPC callbacks, rival recall, press/fan sentiment, years after the fact
- **Signature actions** — every meaningful player has 1-3 football-readable moves that express identity; earned, not stat-assigned
- **Stylized match viewer with semantic-cinema grammar** — 7-shot-type camera vocabulary modulated by stakes + memory; renderer-agnostic contract per ADR-0008. Phase-3-onward dots-prototype validates sim through a sprite-on-pitch adapter (ADR-0009) held to a shippable polish bar; cel-shaded 3D is a candidate shipping visual gated on the Phase-5/6 production-feasibility spike per `design/3d-pipeline.md`. Dots may ship at EA if the spike fails; no public "3D coming in 1.0" dated promise. (Supersedes 2026-04-22 "2D-first MVP / 3D deferred post-EA" framing per 2026-04-26 decisions-log entry.)
- **Breakthrough-driven player development with soft caps + rare narrative exceptions** — bounded internal gene model (0.0–1.0 clamped); developmental ceilings are typically soft-capped per role/attribute distribution, but narrative breakthroughs (per `design/breakthrough-moments.md`) can redraw those ranges for specific players who earn them. Pillar promise reads "your players grow *because of what happened to them*", not "no ceiling." Replaces the prior 2026-04-22 "Unbounded RPG progression" framing per 2026-04-30 SPEC USP-4 honesty pass: the prior wording promised more than the bounded gene model + balance-harness can deliver. `design/progression.md` owed Phase 4.

**Full pitch:** `PROJECT_CONTEXT.md`.

---

## 2. Source-of-truth map

Read in this order at session start:

| Doc | Role |
|---|---|
| `CLAUDE.md` (this file) | Onboarding contract. Read first. |
| `PROJECT_CONTEXT.md` | Project pitch, tone, audience, 4-bucket scope split. |
| `SPEC.md` | Living work plan + phase list + **decisions log (append-only)**. |
| `STATUS.md` | Current state — active phase, next action, blockers, recent milestones. Updated every task. |
| `CHANGELOG.md` | Append-only human-readable ship log. |
| `SETUP.md` | Authoritative setup procedure. Trigger table for deferred purchases. |
| `TOOLING.md` | Canonical catalog of MCPs, plugins, subagents, CLIs, hooks. Adopt/skip decisions. |
| `TECH_APPROACH.md` | Engineering blueprint (MatchSim architecture, determinism discipline, AI content compiler). |
| `design/**` | Per-project design docs. Authoritative for intent (not implementation). |
| `.claude/agents/*.md` | Per-subagent voice / behavior specs. |

**Archived — do NOT import content from:** none at bootstrap.

---

## 3. Tech stack — LOCKED

See `TECH_APPROACH.md` for full blueprint. One-line summary:

- **Engine:** Unity 6, currently pinned to tech-stream `6000.4.4f1` (LTS migration re-evaluated at Phase 7 per SPEC 2026-04-28 decisions-log entry) + URP 17.4.0, Windows + Mac + Linux
- **Canonical sim:** `MatchSim.csproj` — pure C#, zero UnityEngine references, Q32.32 fixed-point arithmetic for cross-platform deterministic replay
- **Ball physics:** custom deterministic sim (Rocket League lesson), not Unity PhysX
- **UI layer:** UI Toolkit (UXML/USS) — data-bindable, hot-reload, modern
- **Async:** UniTask
- **Data:** ScriptableObjects for content; YAML for behavior-tree archetypes; content packs with stable IDs + schema versions
- **Loading:** Addressables
- **Audio:** FMOD Studio integration (free for indies)
- **Steam:** Steamworks.NET
- **Rendering:** renderer-agnostic ADR-0008 ShotPresentationContract → adapter implementations. ADR-0009 dots-phase adapter (Phase-3-onward sprite-on-pitch) is shipping-quality candidate. Cel-shaded 3D adapter (ADR-0010, conditional on Phase-5/6 production-feasibility spike per `design/3d-pipeline.md`) is candidate shipping visual. EA visual locked at Phase-7/8 by spike outcome — three outcomes per 2026-04-26 decisions-log entry: spike-green → 3D ships; spike-yellow/red → dots ships if polish bar met (no public 3D promise); dots-not-strong-enough → delay EA
- **AI:** bake-time only (content compiler). No runtime LLMs. No ML-Agents (behavior trees + hand-authored manager archetypes).

**Budget model:** Tier 1 bootstrap (buy-on-pain). Existing owned assets remain tracked in SETUP; GPT Image 2 in use for concepts. 3D-asset / animation subscriptions are deferred until the Phase-5 commercial-license audit + production-feasibility spike; Phase-6 production spend requires spike-green outcome. Steam Direct $100 at Phase 8.

**Ruled out:**
- Unreal / Godot: Unity's Mac Editor + URP + Mono-derived build pipeline fits solo workflow
- HDRP: fights the URP cel-shader candidate stack, kills mobile-port optionality, overkill for solo viewer work
- Ink / Yarn Spinner: narrative is event-sourced systemic, not scripted
- VRM-first character pipeline: not the current 3D candidate stack; revisit only if Phase-5 license-audit + spike-feasibility proves it beats the generator → cleanup/rigging → AI-assisted-animation path
- Runtime local LLMs: inference cost breaks match-day flow; bake-time delivers same variety at zero runtime cost
- ML-Agents RL tactical AI: training cost + opacity + non-determinism; behavior trees win here
- Mobile port: deferred indefinitely (revisit post-1.0 if ever)
- Steam Deck Verified at launch: deferred post-launch (Linux build exists; cert work later)

---

## 4. Tooling (MCP, plugins, subagents, CLIs, hooks)

**Canonical catalog:** `TOOLING.md`. Read that for adopt/skip decisions and install procedures.

Installed at bootstrap (user-scope, already present):
- `context7` (library docs)
- `github` (repo / PR / issue workflow)
- `blender-mcp` (3D asset-generation / Blender workflow access — deferred 3D pipeline)

Intentionally skipped:
- `chrome` — WebSearch/WebFetch cover web needs
- `desktop-commander` — Read/Write/Edit/Bash cover filesystem + process

Plugins (verified installed 2026-04-30 at `~/.claude/plugins/installed_plugins.json`):
- `feature-dev` — code-architect / code-explorer / code-reviewer subagents
- `pr-review-toolkit` — silent-failure-hunter / type-design-analyzer / code-reviewer / pr-test-analyzer / comment-analyzer / code-simplifier (mandated by §6.3 on substantial code commits)
- `hookify` — used to author `pr-review-reminder.sh`; available for future hook-from-conversation work

Phase-3-day-1 adoption:
- CoplayDev `unity-mcp` (project-scoped, post-Unity-project-creation)

---

## 5. Workflow contract

### 5.1 Phase structure

Phases declared in `SPEC.md`. Exactly one 🟡 ACTIVE at a time. Each phase has a gate. `/next` picks the first `[ ]` task in the active phase. `/done` marks complete + updates STATUS + CHANGELOG.

### 5.2 Decisions log

`SPEC.md` decisions log is **append-only**. Enforced by `.claude/hooks/protect-decisions-log.sh`. To supersede an earlier decision, append a NEW entry citing the prior one.

Use `/log-decision` to append.

### 5.3 STATUS + CHANGELOG

- `STATUS.md` — updated every completed task. Timestamp auto-maintained by `.claude/hooks/update-status-timestamp.sh` (Stop hook).
- `CHANGELOG.md` — append-only human-readable. Every `[x]` in SPEC should have a matching CHANGELOG line.

### 5.4 Slash commands

Project-scoped:
- `/next`, `/done`, `/audit`, `/refresh-docs`, `/log-decision`, `/status` (base six)
- Plus the extended 25 shipped with blueprint v2 skeleton

### 5.5 Hooks

At `.claude/hooks/`:
- `protect-decisions-log.sh` (PreToolUse on SPEC.md) — append-only enforcement
- `update-status-timestamp.sh` (Stop) — rewrites STATUS.md timestamp
- `pr-review-reminder.sh` (PreToolUse on `Bash(git commit*)`) — soft-reminds Claude to run pr-review-toolkit subagents on substantial code commits (≥100 insertions touching `.cs` / `.py` / `.sh` / shader / asmdef / csproj). **Non-blocking; reminder-only.** Cannot prove subagents actually ran. The §6.3 mandate is the binding rule; this hook only reduces the "I forgot" failure mode.

### 5.6 Git workflow

**Remote:** `github.com:osagberg/FinalWhistle.git` (private — created 2026-04-24). The `vibelogic` GitHub org exists as a reserved name but neither authenticated gh account is a member, so personal namespace is used. Transfer to a proper publisher org is a one-click Phase-8 operation if Steam branding requires it.

Branch strategy:
- `main` — shippable only
- `develop` — integration
- `feat/<name>` — per-feature
- `fix/<name>` — per-fix

Current solo-dev posture (user-confirmed 2026-04-26): direct scoped commits
to `main` are allowed while GitHub Free blocks private-repo branch protection.
Run `scripts/fw verify` before committing. PR gating (CI green +
code-reviewer-agent pass + manual review) returns when branch protection is
available or the user asks for PR mode.

---

## 6. Style + behavior rules

### 6.1 Code + doc style

- **No speculative abstractions.** Bug fix doesn't need surrounding cleanup. Three similar lines beats a premature abstraction.
- **Comments:** default none. Write one only when WHY is non-obvious (hidden constraint, subtle invariant, specific-bug workaround). Never narrate WHAT.
- **No emojis** in code/docs unless explicitly requested.
- **No capitalized state nouns in user-facing UI.** Internal floats (`momentum`, `rhythm`, `pressure`, `team_cohesion`, `signature_readiness`) stay invisible to players. Surface via football-native commentary ("Form: rising", "He's locked in", "The stadium has gone quiet"). See `design/ui-vocabulary.md` for the full rule.
- **Markdown links** to project files (`[match-engine](design/match-engine.md)`). Full URLs for GitHub PRs/issues.

### 6.2 Dev-flow discipline

- `TodoWrite` for multi-step work when genuinely useful. Mark complete as each step lands, don't batch.
- Parallel tool calls when independent; sequential when dependent.
- Dedicated tools (Read/Edit/Write/Glob/Grep) beat Bash for the same job.
- `Explore` subagent for codebase research >3 queries deep.
- Verify before recommending from memory — check current state before acting.

### 6.3 Delegation discipline (subagents are not optional)

Solo-dev project; the main thread's context window is precious. Use subagents for substantial work; the main thread does coordination + multi-file orchestration only. Catalogued in `TOOLING.md §3` + `.claude/agents/` (15 project-specialized agents).

**MUST delegate to a subagent (do NOT do these in the main thread):**

- Substantial code authoring (>200 LoC of new code in one area) — match the subagent's specialty:
  - `gameplay-programmer` — match-sim, signatures, player systems
  - `engine-programmer` — performance-critical hot paths, MatchSim primitive math
  - `unity-specialist` — Unity-side asmdefs, Addressables, package decisions, Editor-API scripts
  - `unity-ui-specialist` — UI Toolkit UXML/USS, management screens
  - `ui-programmer` — HUD, menu frameworks
  - `narrative-director` — memory templates, salience scoring, callback prose
  - `systems-designer` — economy, progression, balance formulas
- **Code review on uncommitted changes ≥100 LoC of code, BEFORE commit** — run all three: `pr-review-toolkit:silent-failure-hunter` (catches try/catch suppression / fallback-on-error / silent failure paths) + `pr-review-toolkit:type-design-analyzer` (audits new types for invariant strength + encapsulation) + `feature-dev:code-reviewer` (general bugs / logic / security / convention drift). Optionally `lead-programmer` for architecture-bearing changes. **Hook-reminded, not hook-enforced**: `.claude/hooks/pr-review-reminder.sh` fires PreToolUse on `git commit` and emits a soft stderr reminder when the staged diff exceeds the threshold + touches code files. The hook is non-blocking by design — it does NOT prove the subagents actually ran; it only flags that they should have. Honest framing per Codex audit 2026-04-30: this is process discipline reinforced by a reminder, not enforcement. The mandate (this bullet) is the binding rule; the hook reduces the "I forgot" failure mode but cannot replace running the subagents. If you skip them, document the reason in the commit body. Rationale: across the 2026-04-28 → 2026-04-30 audit cycles Codex consistently caught issues these subagents would have caught first locally. Running them before Codex review tightens the loop and makes Codex's review focus on the cross-model insights (different-bones-different-blindspots) rather than the kind of issue any code reviewer would flag.
- Codebase exploration spanning >3 queries — `feature-dev:code-explorer` (or built-in `Explore`).
- Feature design before implementation — `feature-dev:code-architect` (returns blueprint with files-to-create / data-flows / build-sequence).
- New ADR / GDD authoring — match-specialty director (`technical-director` / `game-designer` / `narrative-director` / `art-director`).
- Cross-discipline coordination at phase boundaries — `producer`.

**MAY stay in the main thread:**

- Single-file edits ≤100 LoC.
- SPEC / STATUS / CHANGELOG sync after `/done`.
- Multi-file orchestration where the main thread holds the cross-file context.
- Driving MCP tools (Unity / GitHub / blender) directly when the action is one logical operation.
- Reading + summarizing subagent reports.

**Smell test:** if you're about to do >30 minutes of focused work in one area, that's a subagent. The main thread should be reading subagent reports, not authoring lines.

**Established cross-model rhythm** (separate from subagent delegation): Claude drafts → external Codex review pass → user pastes findings back → Claude applies → flip Accepted. Codex consistently catches edge cases + invariant violations Claude misses. Do not skip this cycle on architecturally load-bearing work (ADRs, primitives, contracts, decisions-log entries).

### 6.4 Risky actions — confirm first

Destructive / shared-state / third-party-upload actions need user confirmation. Examples: `rm -rf`, `git reset --hard`, `git push --force`, Steam uploads, public posts. Auto mode shifts default to execute-without-asking but does NOT waive safety rules.

### 6.5 UI / feature verification

For dots/viewer work: run in Unity Editor, verify frame-accurate rendering + determinism replay via seed. Don't claim viewer work succeeds without a scene-capture.

For MatchSim work: verify via xUnit tests AND headless balance-harness sweep. Floating-point reproducibility tested on Windows + Mac + Linux (GitHub Actions).

---

## 7. Common pitfalls — don't

- Don't propose capitalized state-nouns for anything player-facing. Football-native vocabulary only.
- Don't attempt 3D code work before the Phase-5/6 production-feasibility spike per `design/3d-pipeline.md`. Dots-prototype is the Phase-3-onward validation visual; 3D is a candidate shipping layer pending spike outcome (per 2026-04-26 decisions-log entry).
- Don't build Coaching Lineage surfacing pre-MVP. Data seeded; surfacing post-MVP.
- Don't add runtime LLM calls for anything player-facing.
- Don't allow Unity PhysX into canonical MatchSim state. Custom deterministic sim only; Unity physics exists only in viewer interpolation.
- Don't overengineer early-phase setup (buy-on-pain; Phase-N+ tooling waits for Phase-N trigger).
- Don't bypass hooks (`--no-verify`, etc.) unless explicitly requested.
- Don't add features / docs / abstractions beyond the task requested.
- Don't amend commits — create new ones.
- Don't push to `main` directly unless the user has explicitly confirmed
  direct-to-main solo-dev mode for the current phase. In that mode, keep
  commits scoped and run `scripts/fw verify` before committing; PR-only returns
  when branch protection becomes available or the user asks for it.

---

## 8. First-session directive (fresh Claude Code session in this folder)

1. Read `CLAUDE.md` (this file) — **including §6.3 delegation discipline**. Subagent rotation is mandatory, not optional.
2. Read `PROJECT_CONTEXT.md`, `STATUS.md`, `SPEC.md` current state block.
3. Run `claude mcp list` via Bash — confirm installed MCPs match TOOLING.md catalog. The Phase-3-onward Unity MCP server registers as `UnityMCP` (Pascal); the underlying CoplayDev package is `com.coplaydev.unity-mcp` (UPM). If `UnityMCP` is supposed to be active for the current phase but disconnected, ask the user to start the Unity MCP server (`Window → MCP for Unity → Start Server`).
4. Read `TOOLING.md` to confirm plugin / MCP / subagent / skill state. **`feature-dev`, `pr-review-toolkit`, `hookify` are all installed at user-scope (verified 2026-04-30)** — if a fresh-Claude-session lookup at `~/.claude/plugins/installed_plugins.json` shows any are missing, surface to user as a 2-min high-leverage install before continuing.
5. Skim `.claude/agents/` so subagent delegation is top-of-mind from turn 1.
6. Check `git status` + `git log -3` for recent state.
7. Report current phase + active task + blockers + any TOOLING.md gaps.
8. Wait for user instruction OR auto-run `/next` if auto mode active.

**Anti-pattern to avoid** (logged 2026-04-28): running multiple Phase-3 sessions doing Unity work without realizing `unity-mcp` was queued in TOOLING.md but never installed. Cost: weeks of "describe menu paths to user" instead of driving the Editor directly. Do not repeat by skipping step 4.

---

*Authored 2026-04-22. Fork of blueprint v2. Cross-refs kept current via `/refresh-docs`.*

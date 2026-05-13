# CLAUDE.md — Final Whistle project contract (Rust pivot, v2)

> Read this first in every new session. Authoritative onboarding contract.
> v2 is the 2026-05-13 pivot from Unity+C# to Rust+Tauri+SolidJS. The pre-pivot
> blueprint is preserved at git tag `v0-pre-pivot-2026-05-13` and in sibling
> `/Users/vibelogic/dev/football-archive/`. Do not import from the archive
> except via the carry-forward catalog in `docs/archive/MIGRATION_AUDIT.md`.

---

## 1. What this project is

**Final Whistle is a procedural-fantasy football manager where careers remember and worlds are unique.** Steam-first; Mac + Windows + Linux. PEGI 12. Premium one-time purchase ($20 EA → $30 1.0).

**Scope ambition.** This is an ambitious deep-simulation project. Implementation scope is bounded by the architecture pillars (below) + maintainability under a Claude+human pair, NOT by LoC counts, hours, or "what a single developer could write." Football Manager's match engine is a useful low watermark, not a ceiling — beating FM on depth is a valid target. Per-system implementation may run into the tens or hundreds of thousands of lines; the only question that matters is whether each line is honest, deterministic, and worth shipping. Effort estimates in MASTER_PLAN are removed for this reason — "1 day" / "1 week" tags mean nothing when the dev is Claude.

Five pillars (see `docs/DESIGN_DOC.md` §3 for full text):
1. **Procedural fantasy world** — every save is a different world; LLM-baked content packs. No real licensed data, ever.
2. **Careers that remember** — append-only event ledger surfaces decisions years later.
3. **Breakthrough-driven development** — players grow because of what happened, not XP.
4. **Scouting uncertainty** — disagreeing biased scouts; truth emerges over seasons.
5. **Signature identity** — readable on-pitch moves, not stat lines. (The "24 signature" number is initial scope, not a cap.)

Text-first presentation. 2D tactical board + dense commentary. No 3D viewer. No runtime LLMs.

Full pitch: `docs/DESIGN_DOC.md`.

---

## 2. Source-of-truth map

Read at session start, in order:

| Doc | Role |
|---|---|
| `CLAUDE.md` (this file) | Onboarding contract. |
| `docs/DESIGN_DOC.md` | Stable design contract. Pillars, rules, scope discipline. |
| `docs/MASTER_PLAN.md` | Phase list + delivery order + acceptance gates. |
| `MEMORY.md` | Working memory for the active implementation context. |
| `docs/DECISIONS.md` | Append-only decisions log. Supersede via new entries citing prior. |
| `STATUS.md` | Current phase, active task, blockers. Auto-stamped on `/done`. |
| `CHANGELOG.md` | Append-only human-readable ship log. |
| `docs/archive/` | Historical (Unity-era) docs. Reference only; never delete. |
| `.claude/agents/*.md` | Per-subagent voice + behavior specs (7 agents — see `.claude/agents/README.md`). |
| `.claude/rules/*/RULES.md` | Path-scoped rules (Rust / Sim / Tauri / Frontend / Content / design-docs). |
| `.claude/skills/next/SKILL.md` | Canonical 9-step `/next` workflow manual. |
| `.claude/context-scopes.json` | Declares 3 context scopes (`minimal` / `standard` / `rich`); active scope at `.claude/.current-scope` (default: `standard`). |
| `REFERENCES.md` | Pivot provenance — what carries forward from FW v1. |

---

## 3. Tech stack — LOCKED

See `docs/architecture.md` for full detail. Summary:

- **Workspace:** Cargo workspace, ~8 crates: `fw-core`, `fw-match-sim`, `fw-content`, `fw-scouting`, `fw-memory`, `fw-replay`, `fw-save`, `fw-tauri`.
- **Engine:** Rust 1.95+ (edition 2024). MSRV bumped from 1.85 at T0-12 because `fixed@1.31` (Q32.32 backing crate) requires rustc 1.93+. No async in the sim layer; no Tokio in `fw-match-sim` or `fw-memory`.
- **Determinism:** Q32.32 fixed-point via `fixed` crate. `ChaCha8Rng` seeded by `(match_seed, tick, event_id)`. `BTreeMap` / `BTreeSet` / `Vec` only — `HashMap` banned in canonical paths (clippy-enforced).
- **Regression floor:** pinned canonical-state hashes. `insta` snapshot tests + `proptest` invariants. GH Actions matrix `[macos-14, windows-latest, ubuntu-22.04]` — drift on any platform blocks merge.
- **App shell:** Tauri 2. IPC boundary is the sim ↔ UI contract; the UI never drives canonical state.
- **Frontend:** TypeScript + SolidJS + Tailwind. TanStack Table for dense tabular surfaces. PixiJS for the 2D tactical board. ECharts for analytics.
- **Content:** RON files with stable content-pack-qualified IDs (`fwh.core:player_00042`). Schema-versioned; load-time forward migration only.
- **AI:** bake-time only. LLM-authored content packs reviewed + committed as RON. No runtime LLM calls anywhere.
- **Distribution:** Steam Direct ($100 at Phase 8). Tauri builds for the three OSes; Steam Deck via Linux build.

**Ruled out:** real licensed data, 3D rendering, manga-broadcast cinematic mode, multiplayer, mobile, runtime LLMs, ML-Agents, Unity revival.

---

## 4. Workflow contract

### 4.1 The primary command: `/next`

`/next` is the end-to-end loop. It:
1. Picks the next unblocked item from `docs/MASTER_PLAN.md`.
2. Drafts the task spec inline (acceptance criteria, files-in-scope, files-out-of-scope).
3. Names the task class + required subagent (per §5).
4. Implements the change (subagent if substantial; main thread otherwise).
5. Runs `scripts/fw verify` (fmt + clippy + cargo test + pnpm test + banned-terms lint + canonical-hash regression).
6. Self-reviews via `pr-review-toolkit:silent-failure-hunter` + `pr-review-toolkit:type-design-analyzer` + `feature-dev:code-reviewer` on any commit ≥100 LoC of code.
7. Commits with structured message + updates `STATUS.md` + appends to `CHANGELOG.md`.
8. Pauses on: out-of-scope file touch, canonical-hash drift, design-doc change, `DECISIONS.md` mutation, `scripts/fw verify` red after one fix attempt, budget escalation.

The user describes a task in plain English. `/next` does the rest. They review the diff at the next check-in.

### 4.2 Supporting commands (small set)

- `/commit` — manual structured commit (rarely needed; `/next` commits automatically).
- `/log-decision` — append a dated entry to `docs/DECISIONS.md` (hook-enforced append-only).
- `/done` — close a phase: verify acceptance gate, sync STATUS/CHANGELOG/MASTER_PLAN, print the `gh pr create` command for Codex review.
- `/status` — read project state in <150 words.
- `/audit` — read-only health sweep (STATUS staleness, plan integrity, determinism violations, banned-terms, etc.).

Six commands total. That's the full set. No `/refresh-docs`, no `/gate-check`, no `/duo-*` — the blueprint pruned aggressively for this project.

### 4.3 Decisions log

`docs/DECISIONS.md` is append-only. Enforced by `.claude/hooks/protect-decisions.sh` (PreToolUse on the file; rejects edits that mutate any line matching `^- \*\*\d{4}-`). Supersede via new entries citing prior bullets verbatim.

### 4.4 STATUS / CHANGELOG / MASTER_PLAN cadence

- `STATUS.md` — rewritten on every `/done`. State pointer, not a diary. Timestamp auto-stamped by Stop hook.
- `CHANGELOG.md` — append-only human-readable. Every shipped item in MASTER_PLAN gets a line.
- `docs/MASTER_PLAN.md` — single source of truth for delivery order. Updated on every `src/` change.

### 4.5 Git workflow

Solo-dev direct-to-`main` is the default while GitHub Free blocks private-repo branch protection. Run `scripts/fw verify` before every commit. Phase-gate commits open a PR for Codex review (§6). No `--no-verify`. Create new commits — never amend.

---

## 5. Subagent rotation (slim)

Seven agents do the work. Main thread coordinates. `/next` MUST name the task class + required agent before code is written; skipping requires a one-liner in the commit body. Per-agent voice + responsibilities in `.claude/agents/*.md`.

| Task class | Indicator | Required agent |
|---|---|---|
| **Sim / Rust** (≥100 LoC in `fw-match-sim`, `fw-memory`, `fw-replay`, `fw-save`, `fw-core`, `fw-content`, `fw-scouting`) | New canonical-state surface, BT runner change, ball physics, ledger reader | `gameplay-programmer` |
| **Balance / formulas / progression** | Salience weights, gene curves, signature trigger thresholds, economy | `systems-designer` |
| **Content / narrative / templates** | RON authoring, Tracery template banks, scout-prose, commentary phrase banks, memory-event readers | `narrative-director` |
| **Architecture / cross-crate / IPC** | New crate, crate-boundary change, Tauri command surface, save schema bump, ADR | `lead-programmer` |
| **Frontend / UI** | SolidJS components, Tauri command handlers, TanStack Table, PixiJS 2D tactical board, ECharts, Tailwind v3 | `ui-programmer` |
| **QA / test design** | Acceptance criteria, insta snapshots, proptest invariants, FW-VAL content checks, save-migration fixtures | `qa-lead` |
| **Phase-boundary coordination** | Gate check, scope negotiation, milestone review, Codex-review handoff | `producer` |

**Self-review is mandatory before commit on any change ≥100 LoC of code.** `/next` runs all three: `pr-review-toolkit:silent-failure-hunter` + `pr-review-toolkit:type-design-analyzer` + `feature-dev:code-reviewer`. Soft-reminded by `.claude/hooks/pr-review-reminder.sh`; the mandate is the binding rule.

Codebase exploration spanning >3 queries: `feature-dev:code-explorer` (or built-in `Explore`). Feature design before implementation: `feature-dev:code-architect`.

May stay in the main thread: single-file edits ≤100 LoC, STATUS/CHANGELOG/MASTER_PLAN sync, multi-file orchestration where the main thread holds cross-file context, reading + summarizing subagent reports.

---

## 6. Phase-gate review (Codex)

Codex reviews at **phase boundaries only**, not per task. Per-task self-review (§5) is the inner loop; Codex is the outer loop.

At each phase gate:
1. Run `/done` — verifies the phase's acceptance gate, syncs ledgers, prints the `gh pr create` command (user runs it).
2. Hand the PR URL to Codex (separate CLI session). Codex reviews via filesystem + PR comments.
3. Apply findings via `/next` cycles on the same branch.
4. Merge once Codex acks.

No agent-bus per-slice cycle. No `dialog/<topic>.jsonl` for routine work.

---

## 7. Code style + dev-flow rules

- **No speculative abstractions.** Three similar lines beats a premature trait.
- **Comments:** default none. Write one only when WHY is non-obvious. Never narrate WHAT.
- **No emojis** in code/docs unless explicitly requested.
- **Determinism patterns** (clippy-enforced where possible):
  - `HashMap` / `HashSet` banned in `fw-match-sim`, `fw-memory`, `fw-replay`, `fw-save`, `fw-content`. Use `BTreeMap` / `BTreeSet`.
  - `f32` / `f64` banned in canonical match state, `MatchEvent`, `MemoryEvent`. Use the Q32.32 newtype.
  - No `Instant::now()`, `SystemTime::now()`, or `thread_rng()` in sim/content/memory crates.
  - No `tokio` / `async` in `fw-match-sim` or `fw-memory`. Tauri command handlers may be async; the sim is sync.
- **Banned-terms lint** (`scripts/fw banned-terms`): no capitalized mystical state-nouns ("The Hush", "Awakened", "+5 Finishing"). Football-native commentary copy only. Catalog in `design/ui-vocabulary.md`. Sentinel-comment exemption: `// ui-lint:allow term="..." reason="..." reviewer="..."`.
- **Internal floats stay invisible to players.** `momentum`, `salience`, `signature_readiness`, `team_cohesion` are numerics in the sim; the UI surfaces them as commentary text only.
- **Stable IDs everywhere.** Content-pack-qualified (`fwh.core:player_00042`); schema-versioned; no inline string IDs for anything content packs reference.

---

## 8. Risky actions — confirm first

Destructive / shared-state / third-party-upload actions need explicit user confirmation. Examples: `rm -rf`, `git reset --hard`, `git push --force`, Steam uploads, public posts, mutating `unity-project/` or `docs/archive/`, regenerating the content-pack corpus, bumping save schema version. Auto mode shifts default to execute-without-asking but does NOT waive safety rules.

---

## 9. UI / feature verification

- **Sim work:** `cargo test --workspace` must pass with pinned canonical-state hashes intact on the macOS dev box; CI runs the cross-OS matrix. Drift on any platform fails the gate. New behavior gets a `proptest` invariant + an `insta` snapshot.
- **Frontend work:** `pnpm test` + `pnpm lint` clean. Manual look-see screenshot attached to the commit (for tactical-board or dense-UI changes).
- **Save / migration work:** every schema bump owes four tests (forward-migration + callback-preservation + forward-incompat-failure + round-trip-byte-identical) per `design/specs/save-migration-fixtures.md`.
- **Content-pack work:** `scripts/fw verify-content` (FW-VAL checks per `design/specs/content-pack-validation-contract.md`).
- Don't claim "done" without `scripts/fw verify` green.

---

## 10. Common pitfalls — don't

- Don't put `f32` / `f64` in canonical match state, `MatchEvent`, or `MemoryEvent`. Q32.32 only.
- Don't put `HashMap` / `HashSet` in sim / memory / replay / save / content crates. `BTreeMap` only.
- Don't put `tokio` / `async` in the sim layer. Tauri IPC handlers may be async; the sim is sync.
- Don't call an LLM at runtime. Bake-time content compiler only.
- Don't add real-world licensed names, clubs, kits, or competitions.
- Don't propose capitalized state-nouns in player-facing UI. Football-native vocabulary only.
- Don't mutate `docs/DECISIONS.md` historical entries. Append + supersede.
- Don't bypass hooks (`--no-verify`, etc.).
- Don't amend commits — create new ones.
- Don't grow STATUS.md into a diary. It's a state pointer.
- Don't add features / abstractions / docs beyond the task spec.

---

## 11. First-session directive

1. Read `CLAUDE.md` (this file), including §5 subagent rotation table and §7 determinism patterns.
2. Read `docs/DESIGN_DOC.md` pillars + rules + scope discipline.
3. Read `docs/MASTER_PLAN.md` current phase + next acceptance gate.
4. Skim `MEMORY.md` + `STATUS.md` for working state.
5. Run `claude mcp list` — confirm `context7` + `github` connect. (No Unity MCPs in v2.)
6. Run `git status` + `git log -3` for repo state.
7. Report current phase + active task + blockers in <150 words (i.e. invoke `/status`).
8. Wait for user instruction OR auto-run `/next` if auto mode is active.

---

## 12. Communication style

The user is technical but attention-span-limited. When reporting status, decisions, audit findings, or anything else not-code: **plain English first, lists only when they genuinely help.** Technical depth stays — formatting just stops being a wall.

- One good sentence beats six bullets. "T0 done; 22 players tick deterministically and the canonical hash agrees across macOS/Win/Linux" beats a 12-row table of test results.
- Tables when they actually save scanning — verdicts, scoreboards, before/after comparisons. Not for "here are five things I did."
- Don't pad with severity tags / scoring rubrics / process meta-commentary unless asked.
- For decisions: give a one-line recommendation + your reasoning, then the alternatives, then stop. Don't enumerate every consideration.
- For code review or audit output: lead with the verdict (Accept / Revise / Reject), then the 2-3 findings that matter, then stop.
- "Here's where we are, here's what I just did, here's what's next" is the right shape for status reports — three short paragraphs, not three sections of bullets.

When a list IS the right format (e.g., a numbered procedure, a verdict table, file paths to read in order), use it without apology. The rule is "lists serve communication," not "lists are banned."

---

*Authored 2026-05-13. v2 pivot from Unity+C# to Rust+Tauri. Blueprint reconciled in-place from `/Users/vibelogic/dev/blueprint/` (slim, Rust-flavored). Revise at each phase transition.*

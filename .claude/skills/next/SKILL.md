---
name: next
description: End-to-end workflow for shipping one MASTER_PLAN task. Picks the next unblocked task, drafts a task-spec, implements via the right subagent, runs `just test` + `just lint`, auto-runs pr-review-toolkit self-review on commits ≥100 LoC of code, commits with a structured message, updates STATUS/CHANGELOG/DECISIONS, and stops. Designed for braindead-easy iteration — the user types `/next`, this runs the full loop, returns one shipped commit, and waits for the next `/next`. Pauses on ambiguity, out-of-scope file touches, repeated test/lint failure, design-doc changes, schema drift, or unauthorized hash drift. Replaces FW v1 `/duo-implement`'s per-slice Codex review with per-task self-review + phase-gate Codex review.
triggers:
  - next
  - /next
  - next task
  - ship the next task
  - pick next task
  - work on next
---

# /next — the primary workflow command

One `/next` = one MASTER_PLAN task shipped end-to-end. Multiple `/next` invocations work through the phase until the phase ends or the user stops.

This skill is the manual. The slash command at `.claude/commands/next.md` is a thin invoker. If something goes weird, read the relevant step below.

## Design intent

- **Braindead-easy UX.** User types `/next`. Claude does everything. Returns one shipped commit. Repeat.
- **One control point per task.** `/next` does NOT auto-loop. After each task lands, the user re-invokes. This gives the human a built-in checkpoint without bloating the workflow with confirmation prompts.
- **Self-review baked in.** No external Codex round-trip per task. `pr-review-toolkit` triple runs automatically on any commit ≥100 LoC of code. Codex enters at phase boundaries via `/phase-gate`, not per task.
- **Lighter than FW v1 `/duo-implement`.** No agent-bus topic per task. No `ScheduleWakeup` polling loop. No per-slice commit-proposal/ack ceremony. Smaller failure surface; fewer escalation paths.
- **Hard stops over hopeful retries.** Two test failures → pause. One unauthorized scope touch → pause. The user is cheap to ask; a hidden bad commit is expensive.

## What changed vs FW v1 `/duo-implement`

| Aspect | FW v1 `/duo-implement` | FW v2 `/next` |
|---|---|---|
| Codex review cadence | Per slice via agent-bus | Per phase via `/phase-gate` PR |
| Self-review | Manual (mandate, soft-reminded by hook) | Auto-invoked on ≥100 LoC code commits |
| Coordination | `ScheduleWakeup` polling + `dialog/<topic>.jsonl` | None — direct execution |
| Escalation triggers | 12 distinct triggers across spec + skill | 9 triggers, plain language |
| Cost expectation | $5-15 per task (relay overhead) | $1-4 per task (no relay) |
| Task spec | Structured RON-ish in topic body | Plain inline in MEMORY.md |
| Hash-drift handling | Hook blocks; agent escalates | Hook blocks; agent escalates (same) |
| What user sees | Topic transcript + final commit | Just the commit + a summary line |

The bet: agent-bus per-slice review was correct for the Unity-era slice ladder where Codex caught Claude's blindspots on visual/runtime evidence (e.g. Slice 7's static-ball miss). For a Rust pivot where the inner loop is `cargo test` + canonical-hash regression + clippy lints — all reproducible, all deterministic, all enforced at commit time by hooks — Codex's review value moves to phase-boundary architecture review. Per-task gets self-review; per-phase gets Codex.

If a task IS architecturally load-bearing (new crate, new contract, ADR-bearing), the user runs `/duo-debate` (Tier-1, no repo changes) first, then `/next` executes the settled design. `/next` does NOT debate; it implements.

---

## The 9-step workflow

### Step 1 — Pick the task

Read in this order:
1. `docs/MASTER_PLAN.md` — find the task.
2. `STATUS.md` — note the active phase + any blockers.
3. `MEMORY.md` — note any IN_PROGRESS task or carry-over context.

**Selection rules** (in priority order):

1. If the slash command was invoked with an explicit task ID (`/next T2-04`):
   - Verify it exists, status is TODO or IN_PROGRESS, and all `depends_on` tasks are DONE.
   - If not eligible, report why + suggest alternatives. Stop.

2. Else if MEMORY.md has a `## Current task` block with status IN_PROGRESS:
   - Resume that task. Skip Step 2's IN_PROGRESS flip (already done).
   - Print: "Resuming T<id>: <title> (IN_PROGRESS since <date>)".

3. Else walk MASTER_PLAN.md in declared order (phases top-down, tasks top-down within a phase):
   - Find the first task with `status: TODO` whose `depends_on` are all `status: DONE`.
   - If multiple top-priority candidates tie, pick the lowest task ID lexically.

4. If no eligible task exists in the active phase:
   - All-DONE in active phase → report "Phase <N> complete; run `/phase-gate` for Codex review, then `/done` to close + promote next phase."
   - All-TODO blocked → report "Phase <N> has TODOs but all are blocked. Blockers: <list>." Stop.

### Step 2 — Spec the task

**Mark IN_PROGRESS in MASTER_PLAN.md.** Edit the task's status field + add `started: YYYY-MM-DD`.

**Write a task-spec block in `MEMORY.md`** under a `## Current task` heading. The format:

```markdown
## Current task

- **id:** T<id>
- **title:** <title>
- **started:** YYYY-MM-DD
- **task class:** <one of: sim-rust | balance-formulas | content-narrative | architecture-cross-crate | phase-boundary>
- **required subagent:** <per CLAUDE.md §5 rotation table>

### Acceptance criteria (falsifiable)
- <criterion 1>
- <criterion 2>

### Files in scope
- <glob or path>
- <glob or path>

### Files out of scope (do NOT touch — escalate if needed)
- docs/DESIGN_DOC.md
- docs/DECISIONS.md
- CLAUDE.md
- docs/MASTER_PLAN.md  (status flip is the only allowed mutation)
- design/**.md
- <task-specific additions>

### Intentionally NOT done in this task
- <thing the user might expect but is out-of-scope>
- <thing deferred to later task T<id>>

### Plan (3-7 chunks; skip if trivial)
- [ ] Chunk 1: <one-line>
- [ ] Chunk 2: <one-line>
- [ ] Chunk 3: <one-line>
```

**Ambiguity gate.** If acceptance criteria require creative judgment (UI naming, narrative tone, visual styling, balance values not in the task description), **PAUSE and ask the user**. Do NOT proceed and decide on their behalf. Examples:
- "The UI string for the breakthrough text recap" → ASK
- "The salience weights for the new event class" → ASK
- "Whether to put the new crate at workspace root or under `crates/`" → ASK (architecture decision)
- "The exact threshold value for the regressive-collapse trigger" → ASK (balance)
- "Add tests for the existing `salience_score` function" → PROCEED (mechanical)

Re-confirm scope reading: if the task description references a design doc, read the relevant section of that design doc NOW. Cite it in the MEMORY task-spec under "Design references".

### Step 3 — Plan implementation

**Trivial tasks** (one file, <30 minutes, mechanical): skip the chunk list. Just code.

**Non-trivial tasks**: write the 3-7 chunk plan in the MEMORY block (Step 2 template above). Each chunk is one logical unit of work — one function, one module, one test fixture, one IPC command. Order them so verification is possible after every chunk (tests can run after chunk N).

If the chunk count would exceed 7, the task is too big — **PAUSE and recommend the user split it** into sub-tasks via MASTER_PLAN edits before continuing.

### Step 4 — Implement

**Subagent selection** (per `CLAUDE.md §5`):

| Task class | Subagent |
|---|---|
| Sim / Rust (≥100 LoC in `fw-match-sim`, `fw-memory`, `fw-replay`, `fw-save`, `fw-core`) | `gameplay-programmer` |
| Balance / formulas / progression | `systems-designer` |
| Content / narrative / templates | `narrative-director` |
| Architecture / cross-crate / IPC | `lead-programmer` |
| Phase-boundary coordination | `producer` |

**Main-thread exceptions** (no subagent needed):
- Single-file edits ≤100 LoC.
- MASTER_PLAN / STATUS / CHANGELOG / MEMORY sync writes.
- Trivial mechanical changes (renames, clippy autofix follow-ups).

**Tests-first when adding behavior:**
- New sim behavior → `insta` snapshot test in the same crate's `tests/` dir.
- New canonical-state surface → add a fixture seed to `fw-replay` corpus + verify the pinned hash regenerates intentionally (user must approve hash drift in the task-spec).
- New invariant → `proptest` invariant under `proptest/` in the relevant crate.
- New IPC command → integration test in `fw-tauri` that round-trips through the IPC boundary.

**Determinism patterns are non-negotiable** (per `CLAUDE.md §7`):
- `HashMap` / `HashSet` banned in sim crates. `BTreeMap` / `BTreeSet` only. Clippy enforces.
- `f32` / `f64` banned in canonical match state, `MatchEvent`, `MemoryEvent`. Q32.32 newtype only.
- No `Instant::now()`, `SystemTime::now()`, `thread_rng()` in sim / content / memory crates.
- No `tokio` / `async` in `fw-match-sim` or `fw-memory`. Tauri IPC handlers may be async.

As each chunk completes, **tick its checkbox in MEMORY.md's plan block**. This is the only progress signal the user sees mid-task; keep it current.

### Step 5 — Verify

After all chunks land:

```sh
just test    # runs cargo test --workspace + pnpm test (frontend)
just lint    # runs cargo fmt --check + cargo clippy --all-targets -- -D warnings + pnpm lint
```

If `just` is not present, fall back to:
```sh
cargo fmt --all -- --check
cargo clippy --all-targets --workspace -- -D warnings
cargo test --workspace
# frontend (only if frontend files touched):
pnpm --filter ./ui lint
pnpm --filter ./ui test
# umbrella (banned-terms, canonical-hash, content-pack-validate, etc.):
scripts/fw verify
```

**Failure handling:**

1. **First test failure:** read the failing test output. Fix the issue. Re-run `just test`. If green, proceed.
2. **Second test failure on the same suite:** PAUSE. Report the failure to the user with the test output + your diagnosis. Do NOT keep grinding. Hand back.
3. **Lint failure:** run `cargo fmt --all && cargo clippy --all-targets --workspace --fix --allow-dirty --allow-staged` ONCE. If `just lint` is then green, proceed. If still red after autofix, PAUSE and report.
4. **`scripts/fw verify` umbrella failure** (banned-terms / canonical-hash / content-pack-validate): treat the same as a test failure — one fix attempt, then pause.

**Canonical-hash drift gate** (separate from verify):
- If a sim-crate change drifts the pinned canonical-state hash AND the task-spec authorized this drift (acceptance criterion explicitly says "regenerate pinned hashes"): re-run `scripts/fw bake-corpus` (or equivalent) and continue.
- If drift is UNAUTHORIZED: **PAUSE**. Report to user. Do not auto-rebaseline.

### Step 6 — Self-review (auto, no user action)

**Trigger:** the staged diff (post-Step 5, pre-commit) is ≥100 LoC of code (`.rs`, `.ts`, `.tsx`, `.js`, `.json` schema files; excludes pure `.md` doc changes).

Quick diff measure:
```sh
git diff --cached --stat | tail -1   # or summed `+lines` from --numstat for code files only
```

If ≥100 LoC of code, **auto-invoke all three** via the `Skill` tool:
1. `pr-review-toolkit:silent-failure-hunter` — catches `try/catch` suppression, fallback-on-error, silent failure paths, `unwrap_or_default` swallowing real errors.
2. `pr-review-toolkit:type-design-analyzer` — audits new types for invariant strength + encapsulation. Especially valuable for new Rust types (newtypes vs primitives, public field vs builder, sealed traits).
3. `feature-dev:code-reviewer` — general bugs / logic / security / convention drift.

Pass each agent: the task title, the relevant file paths from MEMORY's "Files in scope", and a one-line summary of intent.

**Findings handling:**

- **P0 / P1 findings:** fix them in-place. Re-run `just test` + `just lint` to confirm no regression. Re-run the relevant self-review agent on the changed area to confirm the fix lands. (One re-run cap per agent per task — if a P0 keeps recurring, PAUSE.)
- **P2 / P3 findings:** do NOT fix mid-task. Capture them in the commit message under a "Known follow-ups" block (see Step 7 commit format). Optionally call `mcp__ccd_session__spawn_task` to flag a self-contained follow-up if it's a real bug worth a dedicated session.

If a self-review agent itself errors / hangs / returns garbage: log the error in the commit body under "Self-review notes" but do NOT block the commit on agent infrastructure failure. The lint + test + canonical-hash gates are the binding gates; self-review is a quality multiplier, not a blocker.

### Step 7 — Commit

**Stage exactly the files in `Files in scope`** from MEMORY + the two always-staged files:
- The MEMORY.md update (Step 8 will clear the current-task block; commit the pre-clear version with the task-complete state)
- The MASTER_PLAN.md status flip from IN_PROGRESS to DONE

Do NOT use `git add -A` or `git add .`. Stage by explicit path.

**Commit message format** (HEREDOC; preserves formatting):

```sh
git commit -m "$(cat <<'EOF'
<type>(<scope>): <title> [T<id>]

What:
- <one-line: what changed>
- <one-line: what changed>

Why:
- <one-line: rationale tied to acceptance criterion>

Verification:
- just test: PASS (<N> tests, +<delta> new)
- just lint: PASS
- canonical hash: <UNCHANGED | REBASELINED-authorized | n/a>
- self-review: <silent-failure-hunter OK | type-design-analyzer OK | code-reviewer OK | n/a if <100 LoC>

Known follow-ups (P2/P3 from self-review, optional):
- <finding> — defer to T<id>

Task: docs/MASTER_PLAN.md T<id>

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

**Commit types** (small vocabulary):
- `feat` — new behavior
- `fix` — bug fix
- `refactor` — internal change, no behavior delta
- `test` — tests-only commit
- `docs` — docs-only (rare from `/next`; `/log-decision` and `/done` cover most doc cases)
- `chore` — tooling, CI, lint config

**Commit scopes** (single Rust workspace):
- `sim` — `fw-match-sim`
- `memory` — `fw-memory`
- `replay` — `fw-replay`
- `save` — `fw-save`
- `content` — `fw-content`
- `scouting` — `fw-scouting`
- `core` — `fw-core`
- `tauri` — `fw-tauri`
- `ui` — TS/SolidJS frontend
- `corpus` — pinned-hash regression corpus
- `docs` — top-level docs
- `ci` — `.github/workflows/**` or `scripts/fw`

**Hooks that fire on commit** (do not bypass):
- `.claude/hooks/canonical-hash-guard.sh` — re-runs pinned canonical-state hash test; blocks commit on drift unless task-spec authorized rebaseline.
- `.claude/hooks/protect-decisions.sh` — rejects mutations to historical entries in `docs/DECISIONS.md`.
- `.claude/hooks/validate-commit.sh` — secrets scan + append-only check on protected docs.

If a hook blocks: read the hook output. If you can fix the cause in-place (e.g. unstage `docs/DECISIONS.md` and re-stage as a NEW append), do so and retry. If you cannot fix without scope expansion: **PAUSE + report**. Do NOT `--no-verify`.

### Step 8 — Update docs

**`STATUS.md`:** timestamp is auto-stamped by the Stop hook (`.claude/hooks/update-status-timestamp.sh`). Update the body to reflect the new active task + any blockers cleared by this commit. Keep it under 150 words; it's a state pointer, not a diary.

**`CHANGELOG.md`:** append one line under the current phase's section:
```markdown
- YYYY-MM-DD — T<id> <title> — <one-line summary> (commit <short-sha>)
```

**`docs/DECISIONS.md`:** append IF the task made a decision worth recording (new contract, new convention, ADR-supporting choice, tradeoff resolution). Format:
```markdown
- **YYYY-MM-DD — T<id> — <decision title>** — <one-paragraph context + decision + consequence>. Supersedes: <prior entry if any>.
```
The append-only hook will block any mutation of prior entries. If superseding, cite the prior bullet verbatim in the new entry; do NOT edit the old one.

**`MEMORY.md`:** clear the `## Current task` block. Append a one-line bullet to a `## Recently completed` rolling list (keep last ~10 entries; older ones drop off):
```markdown
- YYYY-MM-DD — T<id> <title> — commit <short-sha>
```

These four doc updates were already staged in Step 7 if MEMORY.md / MASTER_PLAN.md were in the commit. CHANGELOG.md, DECISIONS.md, and STATUS.md changes are staged + committed as part of the same commit (they're always in-scope for `/next`-driven task completion). The STATUS.md timestamp hook fires on session Stop, not on commit; the body changes go in the commit.

### Step 9 — Loop or stop (the user decides)

Print a one-line summary to stdout:
```
Completed T<id>: <title> at commit <short-sha>. Next up: T<next-id>: <next-title>.
```

If no next task is eligible (phase complete / all blocked):
```
Completed T<id>: <title> at commit <short-sha>. Phase <N> is complete — run /phase-gate for Codex review, then /done to promote Phase <N+1>.
```

or:
```
Completed T<id>: <title> at commit <short-sha>. Remaining tasks in Phase <N> are blocked: <list>. Resolve blockers before /next.
```

**Do NOT auto-pick the next task.** Stop here. The user invokes `/next` again when ready.

This is a deliberate design choice: every commit is a checkpoint the user can review at their own pace. Auto-looping would make this a runaway process; one-task-per-invocation keeps the human in the loop without making them confirm every step.

---

## Pause-and-ask triggers (explicit summary)

The skill **STOPS** (does not commit, does not continue) and hands back to the user when any of these fire:

1. **Acceptance criteria ambiguous** (Step 2) — creative judgment needed for naming / tone / styling / load-bearing tuning.
2. **Out-of-scope file touch needed** (Step 4) — implementation requires editing a file not in MEMORY's "Files in scope" list.
3. **`just test` red after 2 fix attempts** (Step 5) — diagnose + hand back. Do not grind.
4. **`just lint` red after autofix** (Step 5) — diagnose + hand back.
5. **`scripts/fw verify` umbrella red after 1 fix** (Step 5) — banned-terms, content-pack-validate, etc.
6. **Design-doc-level architecture change required mid-task** (any step) — needs `/duo-debate` or `/log-decision` first; not `/next`'s job.
7. **New third-party crate required** (Step 4) — user approves the Cargo.toml addition; security + maintenance review is theirs to do.
8. **Canonical state schema change** (Step 5/7) — re-baselining pinned hash is authorized in the task-spec OR escalate.
9. **Unauthorized canonical-hash drift** (Step 7) — hook fires; if not authorized in task-spec, pause + investigate.
10. **Pre-commit hook block that can't be auto-fixed** (Step 7) — never `--no-verify`; fix root cause or pause.

For each, the pause message format is:
```
PAUSED on T<id>: <trigger>.
Current state: <one-line — what's done, what's not>.
Decision needed: <specific question>.
Resume after: <user response / specific file edit / approve a Cargo.toml addition / etc.>.
```

---

## Failure handling cookbook

### Test failure
- **First failure:** read test output → diagnose → fix → re-run `just test`. Proceed if green.
- **Second failure (same suite):** PAUSE. Print the test output + your diagnosis + your suspicion about root cause. Hand back.
- **Test passes but coverage is missing for new behavior:** add the missing test, re-run, proceed.

### Lint failure
- **Format:** `cargo fmt --all` + restage. Proceed.
- **Clippy:** `cargo clippy --all-targets --workspace --fix --allow-dirty --allow-staged` ONCE. Re-run `just lint`. If still red, PAUSE (don't try to manually rewrite clippy violations beyond one autofix pass).

### Pre-commit hook block
- **Canonical-hash-guard:** if task-spec authorized: regenerate corpus + restage. If not: PAUSE.
- **Protect-decisions:** the commit tried to MUTATE an existing DECISIONS.md entry. Append a new entry instead. Restage.
- **Validate-commit (secrets):** read the matched line. If false positive (variable name like `password_field`), the hook should have a sentinel — add the sentinel + restage. If real secret, REMOVE it from the diff + PAUSE.

### Disk full / git error / OOM / network failure
- Report the OS-level error verbatim. PAUSE. Do not retry blindly — these are usually environmental.

### Subagent infra failure (self-review agent hangs / errors)
- Log under "Self-review notes" in the commit body: "<agent> errored: <message>; lint + test + hash all green so proceeding". DO NOT block the commit on agent infrastructure failure (Step 6 spells out this exception).

### Multiple chunks blocked by the same upstream issue
- If chunk 1 reveals that the task design is broken (e.g. the proposed API doesn't compose with an existing trait), PAUSE at chunk 1 boundary. Do NOT try to redesign mid-task. The task-spec is wrong; the user needs to update it (probably via `/log-decision`).

---

## What `/next` deliberately does NOT do

- **Run Codex review.** Codex enters at phase boundaries via `/phase-gate`, which opens a PR with the phase's accumulated commits. Per-task Codex review is the v1 model we're replacing.
- **Push to remote.** User pushes when they want; `/next` commits locally.
- **Create PRs.** `/phase-gate` does this at phase boundaries.
- **Run the Tauri app for manual playtest.** User does this via `cargo tauri dev` or similar. `/next` doesn't open the GUI.
- **Generate / bake the content corpus.** Separate `/bake-content` command (when authored) handles LLM bake-time pipeline at content milestones.
- **Mutate design docs.** `design/**.md`, `docs/DESIGN_DOC.md`, `CLAUDE.md`, `TECH_APPROACH.md` are out-of-scope. Use `/log-decision` or `/duo-debate` for architecture work.
- **Mutate `docs/DECISIONS.md` historical entries.** Append-only via Step 8 (hook-enforced).
- **Re-baseline canonical hashes unless authorized.** Drift is suspicious by default.
- **Loop autonomously.** One `/next` = one task. User re-invokes.

---

## Cost expectations

- One `/next` task ≈ **$1-4 USD** at current Claude pricing assuming reasonable cache hit rate (no agent-bus overhead, no per-slice review relay, no `ScheduleWakeup` cache invalidation).
- Self-review triple adds ~$0.50-2 on commits ≥100 LoC.
- Codex cost lives in `/phase-gate` (separate Codex CLI session), not here.

Compare: FW v1 `/duo-implement` was $5-15 per task because of the agent-bus relay + reviewer-ack polling + per-slice review ceremony.

---

## Sanity checklist before invoking

The skill auto-checks these at Step 1; listed here for the user's mental model:

- [ ] Is `docs/MASTER_PLAN.md` up to date? (If `STATUS.md` says Phase 3 but MASTER_PLAN's Phase-3 list is empty, something's broken — run `/refresh-docs`.)
- [ ] Is `just test` currently green on `main`? (If no, the next `/next` will pause on Step 5; fix baseline first.)
- [ ] Is the task description in MASTER_PLAN clear about acceptance criteria + scope? (If no, Step 2 will pause; clean up the task entry first.)
- [ ] Is a `/duo-debate` needed first because the task is architecturally load-bearing? (If yes, debate, then `/next` executes the settled design.)

---

## Cross-references

- Slash command: `.claude/commands/next.md` (thin invoker)
- Subagent rotation: `CLAUDE.md §5`
- Determinism patterns: `CLAUDE.md §7`
- Decisions log: `docs/DECISIONS.md` (append-only, hook-enforced)
- MASTER_PLAN format: `docs/MASTER_PLAN.md`
- Phase-boundary review: `/phase-gate` (Codex CLI review pass via PR)
- Architectural debate: `/duo-debate` (Tier-1, no repo changes)
- Decision logging: `/log-decision`
- Phase closing: `/done`
- Project state read: `/status`

---

*Authored 2026-05-13. Designed for the FW Rust pivot. Replaces FW v1 `/duo-implement` for steady-state per-task work; `/duo-implement` is retired in v2 and lives in `docs/archive/` for reference only.*

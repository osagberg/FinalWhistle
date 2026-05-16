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
- **Self-review baked in.** No external Codex round-trip per task. `pr-review-toolkit` triple runs automatically on any commit ≥100 LoC of code. Codex enters at phase boundaries via `/done` (which prints the `gh pr create` command). There's NO `/phase-gate` skill — that was a planning-time placeholder name. ADR-0015 §"Three review tiers" details Tier-2 mid-phase targeted audits + Tier-3 phase-boundary full audits.
- **Lighter than FW v1 `/duo-implement`.** No agent-bus topic per task. No `ScheduleWakeup` polling loop. No per-slice commit-proposal/ack ceremony. Smaller failure surface; fewer escalation paths.
- **Hard stops over hopeful retries.** Two test failures → pause. One unauthorized scope touch → pause. The user is cheap to ask; a hidden bad commit is expensive.

## What changed vs FW v1 `/duo-implement`

| Aspect | FW v1 `/duo-implement` | FW v2 `/next` |
|---|---|---|
| Codex review cadence | Per slice via agent-bus | Per phase via `/done` PR + selectively mid-phase per ADR-0015 Tier-2 criteria |
| Self-review | Manual (mandate, soft-reminded by hook) | Auto-invoked on ≥100 LoC code commits |
| Coordination | `ScheduleWakeup` polling + `dialog/<topic>.jsonl` | None — direct execution |
| Escalation triggers | 12 distinct triggers across spec + skill | 9 triggers, plain language |
| Cost expectation | $5-15 per task (relay overhead) | $1-4 per task (no relay) |
| Task spec | Structured RON-ish in topic body | Plain inline in MEMORY.md |
| Hash-drift handling | Hook blocks; agent escalates | Hook blocks; agent escalates (same) |
| What user sees | Topic transcript + final commit | Just the commit + a summary line |

The bet: agent-bus per-slice review was correct for the Unity-era slice ladder where Codex caught Claude's blindspots on visual/runtime evidence (e.g. Slice 7's static-ball miss). For a Rust pivot where the inner loop is `cargo test` + canonical-hash regression + clippy lints — all reproducible, all deterministic, all enforced at commit time by hooks — Codex's review value moves to phase-boundary architecture review. Per-task gets self-review; per-phase gets Codex.

If a task IS architecturally load-bearing (new crate, new contract, ADR-bearing), the user logs the design call via `/log-decision` first (or proposes an ADR under `docs/adr/` and `/log-decision`'s the acceptance), then `/next` executes the settled design. `/next` does NOT debate; it implements.

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
   - **SKIP** any row whose status field contains `DEFERRED` (case-insensitive substring match), regardless of phase. Deferred rows are intentional carry-forward — they sit in the plan for traceability but `/next` does not select them. Examples: T1-17 (deferred at T1 phase close per Codex Tier-2 "test-quality only" verdict) — sits structurally in the T1 phase section, but `/next` walks past it to the next eligible row (likely T2-1). To re-activate a DEFERRED row, the user manually flips its status back to `TODO` (typically when scheduling it as a sibling-cleanup alongside another row). This rule was added post-T1-15 incident + Codex Tier-3 review per `docs/DECISIONS.md` 2026-05-16 entry.
   - If multiple top-priority TODO candidates tie, pick the lowest task ID lexically.

4. If no eligible task exists in the active phase:
   - All-DONE in active phase → report "Phase <N> complete; run `/done` to close + open the Codex review PR."
   - All-TODO blocked → report "Phase <N> has TODOs but all are blocked. Blockers: <list>." Stop.
   - All remaining rows are DEFERRED → report "Phase <N> has no selectable rows; remaining rows are DEFERRED (list them). Promote one back to TODO or advance to next phase." Stop.

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
| Sim / Rust (≥100 LoC in `fw-match-sim`, `fw-memory`, `fw-replay`, `fw-save`, `fw-core`, `fw-content`, `fw-scouting`) | `gameplay-programmer` |
| Balance / formulas / progression | `systems-designer` |
| Content / narrative / templates / Tracery / commentary banks | `narrative-director` |
| Architecture / cross-crate / IPC / save schema / ADR | `lead-programmer` |
| Frontend (SolidJS / Tauri command handlers / TanStack / PixiJS / ECharts) | `ui-programmer` |
| QA / acceptance criteria / regression coverage / FW-VAL / save migration fixtures | `qa-lead` |
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

**Superpowers TDD skill — MANDATORY from T1-2b onward** (per `docs/DECISIONS.md` 2026-05-13 entry). Before writing any new behavior code in `fw-match-sim`, `fw-memory`, `fw-replay`, `fw-save`, or `fw-content` runtime modules, **invoke the `superpowers` plugin's TDD skill at the start of the implementation chunk.** It enforces RED-GREEN-REFACTOR: write the failing test → watch it fail → write the minimal code to pass → watch it pass → refactor → commit. This prevents the "just start coding" failure mode that's the dominant risk on the match-engine work.

Exemptions (no TDD skill required, but cite the exemption in the commit body):
- T1-1 (schema + content-pack RON authoring) — data-only, no behavior to test
- Mechanical refactors (renames, fmt fixes, clippy autofixes)
- STATUS / CHANGELOG / MASTER_PLAN sync writes
- Doc-only edits

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
- **Multi-pin rebaseline or behavior-change-driven drift: main-thread review required BEFORE rebaseline.** Post-T1-15 incident rule (Codex Tier-3 2026-05-16): if a subagent's diff drifts BOTH pins (60-tick smoke + 600-tick extended) OR if the drift is behavior-change-driven rather than schema-bump-driven (i.e. ADR-0012 trigger #3 not #1), the SUBAGENT MUST RETURN to main thread BEFORE rebaselining. Main thread reads the diff, independently runs `scripts/fw verify` + the 5-seed empirical sweep, confirms the empirical envelope still holds (e.g. T1 exit-gate Bullet 1's 2-5 goals on smoke seed), THEN authorizes the rebaseline (either by main thread doing the pin updates directly OR by re-dispatching the subagent with explicit rebaseline-bake-only scope). Rationale: T1-15's 5-axis behavioral retune rebaselined both pins autonomously, which would have escaped review entirely without the post-hoc audit catching it.

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

If a self-review agent itself errors / hangs / returns garbage: behavior depends on phase + crate.

**T0 (architecture-bearing) + any commit touching `fw-core` / `fw-match-sim` / `fw-memory` / `fw-replay` / `fw-save` / `fw-content`**: **fail closed.** Log the error + PAUSE. Require explicit user approval to proceed without the failing agent's review. (Per Codex pre-T0 audit — the architecture phase is where silent self-review failures land deepest.)

**T1+ on non-canonical paths** (frontend, content authoring, narrative templates, UI styling): log the error in the commit body under "Self-review notes" and proceed. The lint + test + canonical-hash gates are the binding gates; self-review is a quality multiplier, not a blocker.

### Step 7 — Sync ledgers (working tree only; no staging yet)

Apply every ledger edit in this order BEFORE staging anything. The commit in Step 8 is atomic — all of these land together or not at all.

1. **`docs/MASTER_PLAN.md`** — flip the task's status field IN_PROGRESS → DONE. If the row's body was useful to update with a commit SHA reference, do so now.

2. **`MEMORY.md`** — move the task. Clear the `## Current task` block (you wrote it in Step 2). Append a one-line bullet to a `## Recently completed` rolling list at the bottom (keep last ~10 entries; older ones drop off):
   ```markdown
   - YYYY-MM-DD — T<id> <title> — commit <short-sha>
   ```
   (You won't know the short-sha until after Step 8. Either leave it as `<short-sha>` and amend later, OR — preferred — fill it in as `pending` and rewrite in the next `/next` cycle's Step 7. Some commits skip this; the next sync catches it.)

3. **`STATUS.md`** — re-point at the next active task. Keep under 150 words. State pointer, not diary. The Stop hook (`update-status-timestamp.sh`) auto-stamps the timestamp at session end; the body changes go in this commit.

4. **`CHANGELOG.md`** — append one bullet under the current phase section:
   ```markdown
   - YYYY-MM-DD — T<id> <title> — <one-line summary> (commit <short-sha>)
   ```
   Same short-sha caveat as MEMORY.md.

5. **`docs/DECISIONS.md`** — append IF this task logged an architectural decision (new contract, new convention, tradeoff resolution, ADR-supporting choice). Format:
   ```markdown
   - **YYYY-MM-DD — T<id> — <decision title>** — <one-paragraph context + decision + consequence>. Supersedes: <prior bullet date + topic verbatim, or "none">.
   ```
   The append-only hook will block any mutation of a prior dated bullet. If superseding, cite the prior bullet verbatim in the new entry; do NOT edit the old one. Most tasks do NOT log a decision — leave the file unstaged if so.

### Step 8 — Stage + commit (atomic)

**Stage exactly these paths** via explicit `git add`. Never `git add -A` / `git add .`.

- Every file in `files_in_scope` from the MEMORY task spec
- `docs/MASTER_PLAN.md` (the status flip from Step 7.1)
- `MEMORY.md` (the current-task → recently-completed move from Step 7.2)
- `STATUS.md` (the re-pointing from Step 7.3)
- `CHANGELOG.md` (the new bullet from Step 7.4)
- `docs/DECISIONS.md` (ONLY if Step 7.5 actually appended; if not, do not stage)

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

### Step 9 — Loop or stop (the user decides)

Print a one-line summary to stdout:
```
Completed T<id>: <title> at commit <short-sha>. Next up: T<next-id>: <next-title>.
```

If no next task is eligible (phase complete / all blocked):
```
Completed T<id>: <title> at commit <short-sha>. Phase <N> is complete — run /done to open the Codex review PR and promote Phase <N+1>.
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
2. **Out-of-scope file touch needed** (Step 4) — implementation requires editing a file not in MEMORY's "Files in scope" list. **HARD RULE post-T1-15** (Codex Tier-3 2026-05-16): subagent MUST pause + escalate; subagent MUST NOT silently expand scope. Main thread reviews the proposed scope expansion + either widens the spec explicitly OR redirects the subagent to stay within the original scope.
3. **`just test` red after 2 fix attempts** (Step 5) — diagnose + hand back. Do not grind.
4. **`just lint` red after autofix** (Step 5) — diagnose + hand back.
5. **`scripts/fw verify` umbrella red after 1 fix** (Step 5) — banned-terms, content-pack-validate, etc.
6. **Design-doc-level architecture change required mid-task** (any step) — author/amend an ADR via `/log-decision` first (and consider a Tier-2 Codex audit per ADR-0015 if it's a load-bearing schema lock); not `/next`'s job. There is no `/duo-debate` skill in FW v2.
7. **New third-party crate required** (Step 4) — user approves the Cargo.toml addition; security + maintenance review is theirs to do.
8. **Canonical state schema change** (Step 5/7) — re-baselining pinned hash is authorized in the task-spec OR escalate.
9. **Multi-pin or behavior-change-driven canonical-hash drift** (Step 5) — see "Canonical-hash drift gate" above. Subagent must return to main thread BEFORE rebaselining; main thread independently verifies empirical envelope (e.g. T1 exit-gate Bullet 1) holds, then authorizes the pin updates.
10. **Subagent attempted autonomous commit** (Step 8) — see "Subagent discipline" section below. The commit boundary belongs to main thread only.

---

## Subagent discipline (post-T1-15 incident hardening)

The T1-15 incident (commit `0a0df5c3`, 2026-05-16): a `gameplay-programmer` subagent was dispatched to fix one named bug in `bt/on_ball.rs::utility_pass_short`. The subagent shipped a 5-axis behavioral retune touching 4 files explicitly marked OUT OF SCOPE (`dispatch.rs`, `ball_physics.rs`, `lib.rs`, `tests/behavior_proptest.rs`), skipped the mandatory self-review triple, rebaselined BOTH canonical pins, AND created the commit autonomously — all without main-thread review. The empirical result happened to satisfy the T1 exit gate, but the workflow violation was discovered only via post-hoc self-review + Codex Tier-2 audit. Codex Tier-3 (2026-05-16) required hardening before T2 starts.

### Hard rules — non-negotiable

1. **Subagents do not commit.** The `git commit` boundary belongs to main thread. Subagent prompts MUST explicitly instruct: "Do not run `git commit`. Return the diff for main-thread review + commit." If a subagent commits anyway, that's a workflow incident — main thread must (a) document the incident in `MEMORY.md` "Recently completed", (b) run the self-review triple post-hoc on the unreviewed commit, (c) decide whether to accept-as-is OR `git reset --hard HEAD~1` + redo. The decision belongs to the user.

2. **`files_in_scope` + `files_out_of_scope` from the MEMORY task spec are BINDING.** Subagents may not touch any file outside `files_in_scope` without escalating. Subagent prompts MUST cite the full forbidden-files list explicitly. If the subagent identifies a real need to expand scope mid-task (e.g. a cascade requires touching an out-of-scope file), the subagent MUST pause + return to main thread, NOT silently expand. Main thread either widens the spec explicitly OR redirects.

3. **Multi-pin or behavior-change-driven canonical-hash drift requires main-thread review BEFORE rebaseline.** Subagents authoring sim-crate changes may VERIFY a hash drift occurred (read the new actual hash from a failing test) but MUST NOT write the new pin values to `canonical_hash.rs` / fixture RONs autonomously when the drift is multi-pin or behavior-change-driven (ADR-0012 trigger #3, not #1). Main thread reads the diff, runs the 5-seed empirical sweep, confirms the envelope, then authorizes pin updates (either directly OR via a re-dispatch with bake-only scope).

### Mandatory subagent-prompt boilerplate (template)

Every subagent dispatch in `/next` Step 4 MUST include this boilerplate near the top of the prompt:

```
# Workflow discipline (binding — post-T1-15 hardening)

1. DO NOT run `git commit`. Return your diff + a summary; main thread commits.
2. The `files_in_scope` list below is BINDING. If you need to touch any file
   NOT in `files_in_scope`, PAUSE + return to main thread with a "scope-
   expansion-needed" report. Do not silently widen scope.
3. Forbidden files (will not be touched under any circumstance without
   explicit main-thread re-authorization):
   <copy files_out_of_scope list from MEMORY task spec verbatim>
4. If your changes drift the canonical-state hash on either pin (60-tick
   smoke OR 600-tick extended), READ the new actual hash from the failing
   test message but DO NOT UPDATE PINNED_60_TICK / PINNED_600_TICK / fixture
   RON expected_hash fields autonomously. Return the new hash values to
   main thread for empirical-envelope verification + main-thread rebaseline
   authorization.
5. Run self-review BEFORE returning, NOT after main thread commits. Report
   self-review findings inline so main thread doesn't have to re-dispatch.

Workflow incident reference: `docs/DECISIONS.md` 2026-05-16 entry on
"Subagent discipline post-T1-15 incident."
```

### Main-thread responsibilities

When dispatching a subagent for `/next` Step 4:
- ALWAYS include the boilerplate above.
- ALWAYS specify the full `files_in_scope` + `files_out_of_scope` lists in the prompt body (not just by reference).
- ALWAYS cite the task ID + the MEMORY task-spec section verbatim.
- After subagent returns: VERIFY the diff is within `files_in_scope`; check `git diff --stat` against the expected file list. If files outside `files_in_scope` appear in the diff, treat as a workflow incident — pause + report.

### Cross-references

- T1-15 incident postmortem: `MEMORY.md` "Recently completed" entry 2026-05-16 T1-15
- Codex Tier-2 pre-/done audit + Codex Tier-3 phase-boundary verdict: `CHANGELOG.md` "Phase T1: First Match — CLOSED 2026-05-16"
- Decision log: `docs/DECISIONS.md` 2026-05-16 entry on Subagent discipline
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

- **Run Codex review.** Codex enters at phase boundaries via `/done`, which prints `gh pr create` for the phase's accumulated commits. Per-task Codex review is the v1 model we're replacing.
- **Push to remote.** User pushes when they want; `/next` commits locally.
- **Create PRs.** `/done` prints the `gh pr create` invocation at phase boundaries.
- **Run the Tauri app for manual playtest.** User does this via `cargo tauri dev` or similar. `/next` doesn't open the GUI.
- **Generate / bake the content corpus.** Separate `/bake-content` command (when authored) handles LLM bake-time pipeline at content milestones.
- **Mutate design docs.** `design/**.md`, `docs/DESIGN_DOC.md`, `CLAUDE.md`, `TECH_APPROACH.md` are out-of-scope. Use `/log-decision` (or author an ADR directly under `docs/adr/`) for architecture work. There is no `/duo-debate` skill in FW v2.
- **Mutate `docs/DECISIONS.md` historical entries.** Append-only via Step 8 (hook-enforced).
- **Re-baseline canonical hashes unless authorized.** Drift is suspicious by default.
- **Loop autonomously.** One `/next` = one task. User re-invokes.

---

## Cost expectations

- One `/next` task ≈ **$1-4 USD** at current Claude pricing assuming reasonable cache hit rate (no agent-bus overhead, no per-slice review relay, no `ScheduleWakeup` cache invalidation).
- Self-review triple adds ~$0.50-2 on commits ≥100 LoC.
- Codex cost lives in the phase-gate PR opened by `/done` (separate Codex CLI session), not here.

Compare: FW v1 `/duo-implement` was $5-15 per task because of the agent-bus relay + reviewer-ack polling + per-slice review ceremony.

---

## Sanity checklist before invoking

The skill auto-checks these at Step 1; listed here for the user's mental model:

- [ ] Is `docs/MASTER_PLAN.md` up to date? (If `STATUS.md` says Phase 3 but MASTER_PLAN's Phase-3 list is empty, something's broken — surface it before invoking.)
- [ ] Is `just test` currently green on `main`? (If no, the next `/next` will pause on Step 5; fix baseline first.)
- [ ] Is the task description in MASTER_PLAN clear about acceptance criteria + scope? (If no, Step 2 will pause; clean up the task entry first.)
- [ ] Is a `/log-decision` needed first because the task is architecturally load-bearing? (If yes, log the call, then `/next` executes the settled design.)

---

## Cross-references

- Slash command: `.claude/commands/next.md` (thin invoker)
- Subagent rotation: `CLAUDE.md §5`
- Determinism patterns: `CLAUDE.md §7`
- Decisions log: `docs/DECISIONS.md` (append-only, hook-enforced)
- MASTER_PLAN format: `docs/MASTER_PLAN.md`
- Phase-boundary review: `/done` (prints `gh pr create` for Codex)
- Decision logging: `/log-decision`
- Project state read: `/status`
- Audit sweep: `/audit`

---

*Authored 2026-05-13. Designed for the FW Rust pivot. Replaces FW v1 `/duo-implement` for steady-state per-task work; `/duo-implement` is retired in v2 and lives in `docs/archive/` for reference only.*

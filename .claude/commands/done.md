---
description: Close the current phase — verify gate, sync ledgers, hand off a copy-paste Codex review prompt.
---

# /done — phase completion

Run when the current phase's acceptance gate is believed to pass. NOT a per-task command — that's `/next`.

## Pipeline

### 1. Verify phase scope is complete

Read `docs/MASTER_PLAN.md`. Find the active phase. Confirm every task row is DONE (no TODO, no BLOCKED). If any are open, stop and list them.

### 2. Re-run full verify

```bash
scripts/fw verify
```

Must be green (fmt + clippy + cargo test --workspace + pnpm test + banned-terms + canonical-hash regression). Any failure = stop, report, do not proceed.

### 3. Verify the phase's acceptance gate

Each phase in MASTER_PLAN has an explicit acceptance gate. Read it. Execute every check it specifies. Examples:

- **T0 (foundation):** pinned canonical SHA matches on macOS + Windows + Linux CI; Tauri shell opens.
- **T1 (sim core):** 90-min match completes; replay round-trip byte-identical; proptest invariants hold over 10k matches.
- **Content phase:** `scripts/fw verify-content` green on seeded corpus; FW-VAL checks pass.
- **UI phase:** screenshot of tactical board + commentary feed attached; `pnpm test` + `pnpm lint` green.
- **Save schema bump:** all four migration tests present + green.

If any acceptance check fails, stop and report which.

### 4. Append CHANGELOG entry

Append a phase-summary block to `CHANGELOG.md`:

```
## Phase <N>: <name> — YYYY-MM-DD

- <bullet per shipped MASTER_PLAN item, grouped by crate/area>
- Canonical-state hash: <BLAKE3 short>
- Tests: <n new>, all green on [macos-14, windows-latest, ubuntu-22.04]
- Decisions logged: <count> (see docs/DECISIONS.md)
```

### 5. Rewrite STATUS.md

STATUS.md is a state pointer, NOT a diary. Replace body with:
- Current phase pointer → **next** phase
- Active task: `(none — awaiting Codex review of phase <N>)`
- Blockers: `(none)` or list
- Last green verify: timestamp
- Last canonical hash: BLAKE3 short

Stop hook auto-stamps the file timestamp.

### 5.5 Multi-track ultimate-review at phase boundary (post-2026-05-16 hardening — Codex workflow improvement #7)

Per the 2026-05-16 Codex ultimate-review verdict: at phase close (before the Step 6 Codex hand-off), dispatch a multi-track adversarial review combining Claude + Codex. The split is load-bearing — Claude is strong on implementation-drift / test-quality / systemic-pattern detection; Codex is strong on adversarial red-team + property explosion at scale. **The two together find ~2× what either finds alone**, per the convergence patterns surfaced in the 2026-05-16 audit (`docs/audits/post-t1-ultimate-review-2026-05-16.md`).

Skip ONLY if the prior phase shipped < 5 commits OR the phase deliverable is doc-only.

**At T1 close, the multi-track shape was:**

| Track | Owner | Lens | Output target |
|---|---|---|---|
| A | Claude `feature-dev:code-explorer` | Mutation-test analysis (mental mutation map; no `cargo-mutants` runs) | shared review file Track A |
| B | Claude `pr-review-toolkit:code-reviewer` | Architectural drift (docs vs code, both directions) | shared review file Track B |
| C | Claude `pr-review-toolkit:silent-failure-hunter` | Whole-codebase silent-failure sweep (NOT commit-diff-scoped) | shared review file Track C |
| D | Claude `qa-lead` | Test-the-tests (vacuous patterns + redundancy + coverage holes the test names imply but don't cover + insta snapshot review) | shared review file Track D |
| E | Codex CLI (parallel session) | Adversarial red-team (4 goals: break canonical hash silently; make content pack pass validation while semantically invalid; malicious mod overlay; find a determinism leak) | shared review file Track E |
| F | Codex CLI (parallel session) | Property explosion (PROPTEST_CASES bump 256 → 10,000 on key invariants; intra-process determinism count bump) | shared review file Track F |

**Setup:** main thread creates `docs/audits/post-<phase>-ultimate-review-<YYYY-MM-DD>.md` with section anchors for each track. Subagents dispatched with mandatory boilerplate (no commits, no file edits outside the shared review file, read-only). Codex prompt drafted as a single copy-paste block + handed to user to paste in parallel terminal.

**Review scope** is every row shipped in the phase window — **including rows rolled in from earlier phases** (rows promoted back from `DEFERRED`, or earlier-numbered rows that were actually built during this phase). The audit file's scope section + the Step-6 Codex prompt MUST name those rolled-in rows explicitly so the review covers them, not just the rows whose ID matches the current phase number.

**Consolidation:** main thread reads all 6 tracks when complete, writes a consolidated verdict section to the same file with: severity-sorted findings; cross-track convergence patterns (the highest-value signal); recommended new MASTER_PLAN rows (gate-blocking vs opportunistic); ACCEPT / REVISE / REJECT for the phase close.

Findings classification:
- **Gate-blocker** (rare): fix BEFORE the Step 6 Codex hand-off
- **Pre-next-phase recommended**: add MASTER_PLAN row, land before next phase's first /next
- **Inline cleanup**: fold into next-touching commit
- **Doc-only follow-up**: single docs commit

The phase tag (`v<X>.<Y>.<Z>-<phase-name>`) is created AFTER the ultimate-review verdict lands + any gate-blockers are addressed.

This step adds ~30-60 min wall-clock + ~$15-30 in agent spend per phase close. Acceptable cost given the bug-class catches.

### 6. Hand off the Codex phase-gate review prompt

**Do NOT run `gh pr create`.** Codex CLI reviews against the local filesystem — no PR, no `gh`, no `git push` is needed for the review. Instead, print ONE copy-paste-ready prompt block for the user to paste into a separate Codex CLI session. (Permanent change, 2026-05-21 user direction — this replaces the old PR-based hand-off for all `/done` phase gates.)

Print the block below (DO NOT execute anything), with every `<...>` placeholder filled in from the actual phase state:

```
Phase-gate review — Final Whistle Phase <N>: <name>

Review the whole codebase at its current `main` state for the close of Phase <N>.

SCOPE — every row shipped in the Phase <N> window, INCLUDING rows rolled in from
earlier phases (rows promoted back from DEFERRED, or earlier-numbered rows actually
built during this phase). Rows in scope: <explicit list, e.g. T3-1..T3-9 + T2-4 + T2-7>.
Commit range: <first-sha>..<last-sha> (<N> commits). Lens is whole-codebase +
cross-task + adversarial — NOT commit-by-commit (the per-task self-review triple
already covered each diff).

ACCEPTANCE GATE — see docs/MASTER_PLAN.md Phase <N> exit gate. Confirm every
criterion is genuinely met (substance, not shape).

VERIFY STATE — `scripts/fw verify` green on the macOS dev box; canonical-state
hashes <BLAKE3 short> (<UNCHANGED | REBASELINED-<reason>>).

FOCUS — <2-3 phase-specific risk areas>; determinism leaks; silent failures;
stale-doc drift (CLAUDE.md / ADRs / specs vs code); vacuous tests; save-schema
integrity.

OUTPUT — post a verdict (ACCEPT / REVISE / REJECT) + severity-sorted findings to
docs/audits/post-<phase>-codex-gate-<YYYY-MM-DD>.md, or hand the findings back in
this session for the developer to paste.
```

The user runs Codex CLI in a separate terminal with this prompt, then brings the verdict + findings back. Apply any gate-blockers via `/next` before the phase tag is created.

### 7. Print phase-summary report (<300 words)

- Phase name + duration (commit timestamps first → last)
- LoC delta (crates touched)
- Commits count + most-impactful 3 by message
- Decisions added (count + topics)
- Risks carried into next phase (2-3 bullets, sourced from MEMORY.md + open BLOCKED rows)
- Suggested first task for next phase (from MASTER_PLAN)

Then stop. Next steps belong to Codex review + user merge decision.

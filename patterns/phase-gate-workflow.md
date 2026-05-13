# Pattern: Phase-gate workflow with Codex review

Solo-dev rhythm: `/next` per task, `/done` per phase, Codex reviews the phase PR.

## Why

Per-task Codex review (the FW v1 model) costs $5-15 per slice in relay overhead. Phase-gate review captures architectural concerns at meaningful boundaries while letting deterministic gates (clippy, cargo test, canonical-hash regression, banned-terms lint) catch per-task issues automatically.

## The loop

```
/next → /next → /next → ... → /done → Codex review → merge → /next on Phase N+1
```

## Per-task: /next

See `.claude/skills/next/SKILL.md` for the 9-step pipeline. In summary:
1. Pick first TODO from `docs/MASTER_PLAN.md` whose deps are DONE.
2. Spec inline.
3. Dispatch to the required agent (per CLAUDE.md §5 rotation table).
4. Verify (`scripts/fw verify`).
5. Self-review on ≥100 LoC (three subagents in parallel).
6. Commit + sync STATUS/CHANGELOG/MASTER_PLAN.

No Codex involvement at the task level. Determinism gates + self-review carry the per-task quality floor.

## Per-phase: /done

When every task in the current phase is DONE:
1. `/done` — verifies the phase's acceptance gate, runs full `scripts/fw verify`, appends a CHANGELOG block, rewrites STATUS as a state pointer pointing at the next phase.
2. Prints (does NOT execute) the `gh pr create` command.
3. User runs `gh pr create` to open the phase PR against `main`.

## Codex review

1. User hands the PR URL to Codex (separate CLI session, filesystem-based review).
2. Codex reads `git log <PR-base>..<PR-head>`, the design docs touched, the CHANGELOG entry, the pinned-hash status.
3. Codex comments on the PR for:
   - Architectural concerns (new crate boundaries, IPC contract changes)
   - Determinism violations
   - Scope-creep (features beyond MASTER_PLAN)
   - Missing tests on canonical-state-bearing changes
4. User applies findings via `/next` cycles on the same branch (the phase's tasks are DONE but Codex-fix follow-ups are new TODOs added to MASTER_PLAN).
5. Re-request review. Repeat until Codex acks.
6. Merge.

## What Codex does NOT review

- Per-task code review on routine changes — that's pr-review-toolkit subagents in `/next` step 7.
- Test failures — those are caught by `scripts/fw verify` at commit time.
- Banned-terms violations — caught by lint.
- Canonical-hash drift — caught by `.claude/hooks/canonical-hash-guard.sh`.

## Failure modes

- **Phase too big:** Codex returns a flood of findings. Phase scope was too ambitious. Split next phase smaller. Track in MASTER_PLAN.
- **Phase too small:** Codex returns nothing useful. Phase scope was too narrow — wasted Codex round-trip. Group future phases bigger.
- **Codex unavailable:** ship the phase to a branch, hold merge until Codex catches up. Don't merge unreviewed.

## When phase boundary is unclear

A "phase" is what MASTER_PLAN says it is. T0 is foundation. T1 is sim core. T2 is content. Etc. If the phase boundary is hand-wavy, the `producer` agent owns clarifying it via MASTER_PLAN edits before any `/done`.

## Cross-references

- `.claude/skills/next/SKILL.md` — per-task loop
- `.claude/commands/done.md` — phase-close procedure
- `docs/MASTER_PLAN.md` — phase definitions + acceptance gates
- `CLAUDE.md` §6 — phase-gate review

---
description: Primary end-to-end workflow command. Picks the next unblocked task from docs/MASTER_PLAN.md, implements it fully, runs verification + self-review, and commits. One invocation == one task shipped.
argument-hint: "[optional: task ID like T2-04 to force a specific task]"
allowed-tools: Read, Edit, Write, Bash, Glob, Grep, Skill, mcp__ccd_session__mark_chapter, mcp__ccd_session__spawn_task
---

# /next — Ship one task end-to-end

`/next` is **THE** primary workflow command for the Final Whistle Rust pivot. Type `/next`, get one MASTER_PLAN task shipped + committed. Repeat to keep working through the phase.

This command is a thin invoker. **All workflow logic lives in the `/next` skill** at `.claude/skills/next/SKILL.md` — that's the manual for what happens, why, and what to do when things go weird.

## Behavior

Invoke the `next` skill. The skill executes the full 9-step workflow:

1. **Pick task** — first TODO with DONE deps in `docs/MASTER_PLAN.md` (or resume the current IN_PROGRESS one). Argument override picks a specific task.
2. **Spec the task** — write inline task-spec to `MEMORY.md`; mark IN_PROGRESS in MASTER_PLAN.
3. **Plan implementation** — 3-7 chunk plan in MEMORY for non-trivial tasks; skip for trivial.
4. **Implement** — code via the §5 mandatory subagent for the task class.
5. **Verify** — `just test` + `just lint` (with one autofix attempt on lint).
6. **Self-review** — auto-invoke pr-review-toolkit triple on commits ≥100 LoC code; fix P0/P1 findings.
7. **Commit** — structured message; in-scope files + MEMORY.md + MASTER_PLAN.md status flip to DONE.
8. **Update docs** — append CHANGELOG entry; append DECISIONS.md if the task made a decision; clear MEMORY current-task block.
9. **Print summary + stop** — does NOT auto-pick the next task. User invokes `/next` again.

## Pause-and-ask triggers

The skill hard-stops + reports to the user when any of these fire:

- Acceptance criteria ambiguous (creative judgment needed)
- A required change would touch files outside MASTER_PLAN's declared scope for this task
- `just test` red after 2 fix attempts
- `just lint` red after autofix attempt
- Design-doc-level architecture change required mid-task
- New third-party crate needed (user approves the dependency)
- Canonical state schema change (re-baseline pinned hash)
- Hash-drift hook fires unexpectedly (not authorized in task-spec)
- Pre-commit hook block that can't be auto-fixed

## What `/next` does NOT do

- Run Codex review — that's `/done` at phase boundaries, not per task.
- Push to remote — user pushes manually when ready.
- Create PRs — `/done` prints the `gh pr create` invocation for the phase PR.
- Run the game in Tauri — user does manual playtest.
- Bake content corpus — separate `/bake-content` command at content milestones.

## Usage

```
/next                  # pick the next unblocked task
/next T2-04            # force a specific task (must still have DONE deps)
```

## After `/next` returns

The skill prints a one-line summary:
```
Completed T<id>: <title> at commit <SHA>. Next up: T<next-id>.
```

To continue the phase, run `/next` again. To stop, just don't.

## See also

- Full workflow manual: `.claude/skills/next/SKILL.md`
- Phase boundaries: `/done`
- Decision logging: `/log-decision`
- Project state: `/status`

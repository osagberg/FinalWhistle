---
description: Comprehensive end-of-phase review — feature completeness, quality, risk, go/no-go
argument-hint: "[milestone-name | current]"
---

# /milestone-review — end-of-phase comprehensive review

Generates a milestone progress review: feature completeness, quality metrics, risk assessment, go/no-go recommendation. Run at milestone checkpoints or when evaluating readiness for a deadline.

**Phase:** 4-7. Pairs with `/gate-check` — milestone-review is narrative + metric; gate-check is binary verdict.

## Procedure

1. **Parse args.**
   - `current` (default) — most recently-active phase in `SPEC.md`
   - `<milestone-name>` — a specific phase or named milestone
2. **Load** milestone definition:
   - From `SPEC.md` phase block (acceptance criteria + task list)
   - From `production/milestones/<name>.md` if project uses milestone files
   - `STATUS.md` recent milestones section
3. **Load sprint reports** (if present) under `production/sprints/*.md` within this milestone range.
4. **Scan codebase health.**
   - Count `TODO` / `FIXME` / `HACK` markers across `Assets/_Project/Scripts/**`
   - Note their locations + severity
   - Check `production/risk-register/` if present
5. **Spawn Producer subagent** (or `general-purpose` with Producer persona) — primary coordinator for milestone reviews.
6. **Assemble the review document:**
   - **Overview** — target date, current date, days remaining, sprints/tasks completed (X/Y)
   - **Feature Completeness** — three tables: Fully Complete | In Progress | Deferred (each with Acceptance Criteria + Test Status)
   - **Quality Metrics** — TODO/FIXME/HACK counts, open bugs by severity (S1/S2/S3), `/smoke-check` last verdict, `/regression-suite` last verdict
   - **Risk Assessment** — top 5 risks, likelihood × impact, mitigation
   - **Budget / Scope** — tasks added mid-milestone, tasks deferred out
   - **Go / No-Go Recommendation** — GO / GO-WITH-CAVEATS / NO-GO with rationale
7. **Write** to `reviews/milestone-<name>-<date>.md`.
8. **Recommend next step:**
   - GO → `/gate-check <next-phase>` to formally transition
   - GO-WITH-CAVEATS → surface caveats, user decides
   - NO-GO → list blocking items, recommend `/hotfix` or extended sprint

## If args provided

- `current` / `<name>` — scoping

## If phase has no closed sprints / tasks

Fail: "Phase <name> has no completed work. Milestone review is premature — use `/status` instead."

## Output

- `reviews/milestone-<name>-<date>.md`
- Console: GO / GO-WITH-CAVEATS / NO-GO + top concerns

## Related

- Typical follow-ups (GO): `/gate-check`, `/next` (new phase)
- Typical follow-ups (NO-GO): `/hotfix` for blockers, re-run milestone review
- Invokes agents: `producer`, optionally `technical-director` + `creative-director` (for full review mode)
- Invokes skills: cross-checks output of `/smoke-check` and `/regression-suite`
- Reads files: `SPEC.md`, `STATUS.md`, `production/sprints/**`, `production/risk-register/**`, Unity codebase
- Writes files: `reviews/milestone-<name>-<date>.md`

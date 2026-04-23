---
description: Deeper project orientation after /bootstrap — role options, workflow map, next-step recommendation
argument-hint: "[no arguments]"
---

# /start — orient to the project

Run this **after** `/bootstrap` has completed. Where `/bootstrap` sets the project up, `/start` explains how to use it.

Use this when:
- First session after bootstrap, you want a guided tour
- Returning after a break and you want a refresher on what the blueprint offers
- Handing the project off to another Claude session

## Procedure

1. Read `CLAUDE.md`, `PROJECT_CONTEXT.md`, `SPEC.md` "Current state", `STATUS.md`
2. Detect project state:
   - Is there an active phase? Which one?
   - How many tasks are `[x]` done vs `[ ]` pending in the active phase?
   - Are there unfilled template markers? (flag + suggest `/refresh-docs`)
3. Present a short **Where you are** summary (≤6 lines)
4. Present the **workflow map** grouped by phase. Do not list all 32 commands — pick 6-10 most relevant to current phase:
   - Phase 0-1 (concept): `/brainstorm`, `/map-systems`, `/art-bible`, `/design-system`
   - Phase 2 (design): `/design-system`, `/review-all-gdds`, `/design-review`, `/quick-design`
   - Phase 3 (tech setup): `/create-architecture`, `/architecture-decision`, `/architecture-review`
   - Phase 4-5 (production): `/create-epics`, `/create-stories`, `/story-readiness`, `/dev-story`, `/code-review`, `/story-done`, `/smoke-check`
   - Phase 6 (polish): `/balance-check`, `/regression-suite`, `/milestone-review`
   - Phase 7 (release): `/release-checklist`, `/hotfix`
5. Ask with `AskUserQuestion` what the user wants to do next. Offer 3-4 options based on current phase. Never auto-run.

## If project not bootstrapped

If no `SPEC.md` or `STATUS.md` exists: tell the user to run `/bootstrap` first and stop.

## If state files contradict each other

Flag it (e.g. SPEC says phase 3 active but STATUS shows phase 2 task in-progress). Recommend `/refresh-docs` before proceeding.

## Output

- 1-paragraph **Where you are**
- Phase-appropriate workflow map (commands with one-line descriptions)
- A single next-step recommendation the user can accept or redirect

## Related

- Typical follow-ups: `/next`, `/brainstorm`, `/help`, `/status`
- Invokes agents: none (read-only orientation)
- Invokes skills: none
- Reads files: `CLAUDE.md`, `PROJECT_CONTEXT.md`, `SPEC.md`, `STATUS.md`
- Writes files: none

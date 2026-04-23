---
description: Summarise current project state in under 150 words
---

# /status — where are we

Fast snapshot for starting / resuming work.

## Procedure

1. Read `STATUS.md` in full
2. Read `SPEC.md` — "Current state" block + "Phases" list scan
3. Read last 3 entries of `CHANGELOG.md`
4. Report to user:
   - **Phase**: current phase number, name, ACTIVE / COMPLETE / PENDING status
   - **Progress**: `X of Y tasks done` in active phase
   - **Active task**: what's currently in progress (or "none, awaiting /next")
   - **Blockers**: from STATUS.md
   - **Next action**: concrete next step

## Constraints

- Stay under 150 words in the report
- No pleasantries; just the facts
- If the state files look inconsistent (e.g. SPEC says phase 2 active but STATUS says phase 1 in progress), flag it and recommend `/refresh-docs`

---
description: Mark the current task complete, update SPEC / STATUS / CHANGELOG
---

# /done — ship current task

Close out the active task and keep all state files in sync.

## Procedure

1. Read `SPEC.md` — identify the task you were just working on (the one in progress)
2. Verify acceptance criteria:
   - If obvious from the task description and what you did, proceed
   - If ambiguous, ask user to confirm
3. Update `SPEC.md`:
   - Change `[ ]` to `[x]` on the completed task
   - Add a brief completion note inline (date + key outcome, see examples in existing entries)
4. Append to `CHANGELOG.md`:
   - Under today's date (create new `## YYYY-MM-DD` heading if needed)
   - One concise section describing what shipped, where it lives, how to verify
5. Update `STATUS.md`:
   - `Last updated` → today's date (hook maintains this but safety-check anyway)
   - `Currently working on` → next task in SPEC (or "nothing — awaiting /next")
   - `Next action` → concrete action user or Claude should take
   - Add to `Recent milestones` if this was a phase-advancing ship
6. If this was the last task in the current phase:
   - Check phase gate conditions
   - If gates pass: mark phase `✅ COMPLETE`; promote next phase to `🟡 ACTIVE`
   - If next phase has only placeholder tasks, flesh them out now based on TECH_APPROACH.md §8 + blueprint reference patterns
7. Report to user:
   - What shipped (one sentence)
   - What's next
   - Any blockers or flags

## If the task shouldn't actually be marked done

- If work is partial: describe what's left, keep `[ ]` status, don't run the rest of the procedure
- If the task revealed it was wrong/obsolete: propose striking it and updating SPEC; require user confirmation

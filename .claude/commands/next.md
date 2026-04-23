---
description: Pick up the next unblocked task from SPEC.md and start working
---

# /next — pick up the next task

Autonomously continue project work.

## Procedure

1. Read `SPEC.md`
2. Find the phase currently marked 🟡 ACTIVE
3. Identify the first task in that phase marked `[ ]` (pending, not `[x]` done)
4. Check task dependencies:
   - If the task is user-action (install something, purchase something, create account, authenticate to something): ask the user to do it and wait, don't try to do it yourself
   - If the task is Claude-action: execute it
5. Before starting a Claude-action task:
   - Announce clearly what you're picking up (one sentence)
   - State acceptance criteria (how we'll know it's done)
   - If auto mode is not active, confirm with user before starting
6. Execute the task
7. When complete, invoke `/done`

## If no pending task in active phase

- Check the phase's gate conditions (stated in SPEC.md under each phase)
- If all gates pass: mark phase ✅ COMPLETE, promote next phase to 🟡 ACTIVE, flesh out its tasks if not detailed, report to user
- If gates don't pass: report which gates are blocking, propose remediation

## If no active phase at all

Project is either complete or in an undefined state. Report to user and ask what to do.

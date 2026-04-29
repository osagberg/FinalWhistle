---
description: Pick up the next unblocked task from SPEC.md and start working
---

# /next — pick up the next task

Autonomously continue project work.

## Procedure

1. Read `SPEC.md`.
2. Find the phase currently marked 🟡 ACTIVE.
3. Identify the first task in that phase marked `[ ]` (pending, not `[x]` done). If `STATUS.md → "Next /next picks up"` lists a foundation-first override, prefer that ordering; otherwise SPEC order wins.
4. Check task dependencies:
   - If the task is **user-action** (install something, purchase something, create account, authenticate to something): ask the user to do it and wait, don't try to do it yourself.
   - If the task is **Claude-action**: continue.
5. **Classify the task before starting** (discipline gate):

   | Class | Indicator | Required tooling pass |
   |---|---|---|
   | **Trivial** | <100 LoC, single file, mechanical (typo / comment / config tweak) | Self-review only; commit directly. `unity-check` and `pr-review-toolkit` N/A. |
   | **Substantial code** | ≥100 LoC of `.cs` / `.py` / `.sh` / shader / asmdef | `pr-review-toolkit:silent-failure-hunter` + `:type-design-analyzer` + `feature-dev:code-reviewer` BEFORE commit per CLAUDE.md §6.3. Hook-reminded at commit time. |
   | **Unity-side** | Touches `unity-project/` scenes / scripts / assets / asmdefs | `unity-check` skill at L1 (compile via UnityMCP `refresh_unity` + `read_console`) minimum. L2/L3 if behavior/visuals are the deliverable. |
   | **MatchSim source** | Touches `MatchSim/**` | `scripts/fw build-unity-plugins` AFTER the change to refresh the DLL drop at `unity-project/Assets/Plugins/MatchSim/`. |
   | **Architecture** | New ADR / SPEC decisions-log entry / cross-system contract change | Append to SPEC decisions log via `/log-decision`. Route the design pass to the appropriate director subagent (`technical-director` / `game-designer` / `narrative-director` / `art-director`). |

   A task can hit multiple classes (e.g., a MatchSim feature change is both "Substantial code" + "MatchSim source"). Run all applicable passes.
6. Before executing a Claude-action task:
   - Announce clearly what you're picking up (one sentence).
   - State acceptance criteria (how we'll know it's done).
   - **Name the classification** above + which tooling pass(es) will run.
   - If auto mode is not active, confirm with user before starting.
7. Execute the task.
8. When complete, invoke `/done` — it owns the verification stack (see `.claude/commands/done.md`).

## If no pending task in active phase

- Check the phase's gate conditions (stated in SPEC.md under each phase).
- If all gates pass: mark phase ✅ COMPLETE, promote next phase to 🟡 ACTIVE, flesh out its tasks if not detailed, report to user.
- If gates don't pass: report which gates are blocking, propose remediation.

## If no active phase at all

Project is either complete or in an undefined state. Report to user and ask what to do.

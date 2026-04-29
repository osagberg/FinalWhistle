---
description: Mark the current task complete, update SPEC / STATUS / CHANGELOG
---

# /done — ship current task

Close out the active task and keep all state files in sync. **`/done` is the verification gate**: nothing about the task is "done" until the verification stack below is green.

## Verification stack (MUST pass before marking `[x]`)

For ANY task touching code, run **in this order**:

1. **`scripts/fw verify`** — green (verify-docs + banned-terms + shader-audit-stub + dotnet test). Non-negotiable; if a sub-check fails, fix before flipping `[x]`.
2. **For substantial code changes (≥100 LoC of `.cs` / `.py` / `.sh` / shader / asmdef / csproj)** — pr-review-toolkit subagent pass per CLAUDE.md §6.3:
   - `pr-review-toolkit:silent-failure-hunter`
   - `pr-review-toolkit:type-design-analyzer`
   - `feature-dev:code-reviewer`
   The pre-commit reminder hook (`.claude/hooks/pr-review-reminder.sh`) emits a soft reminder; that hook does NOT replace this requirement. If you skip the toolkit, document why in the commit body (e.g., "trivial mechanical edit"; "doc-only commit").
3. **For MatchSim source changes** — run `scripts/fw build-unity-plugins` to refresh the DLL drop at `unity-project/Assets/Plugins/MatchSim/`. The cross-machine reproducibility contract (SPEC 2026-04-29 decisions log) requires the committed DLLs to round-trip cleanly from any machine.
4. **For Unity scene / script / asset changes** — invoke the `unity-check` skill at the appropriate level:
   - **L1 (compile)**: `UnityMCP refresh_unity` + `read_console` showing zero errors AND zero code warnings (transport-level MCP-WebSocket telemetry blips are not code regressions; call them out explicitly when present rather than overclaiming "zero warnings").
   - **L2 (runtime)**: enter Play mode, exit cleanly, verify scene state via `state-dump` skill if behavior is the deliverable.
   - **L3 (visual)**: screenshot/capture for viewer-adapter rendering changes.
5. **`fw verify` (re-run)** if any of steps 2-4 modified files. Final-state must be green.

## Procedure

1. Read `SPEC.md` — identify the task you were just working on (the one in progress).
2. Verify acceptance criteria:
   - If obvious from the task description and what you did, proceed.
   - If ambiguous, ask user to confirm.
3. **Run the verification stack above. Any failure → STOP, fix, re-run; do not continue this procedure.**
4. Update `SPEC.md`:
   - Change `[ ]` to `[x]` on the completed task.
   - Add a brief completion note inline (date + key outcome, see examples in existing entries).
5. Append to `CHANGELOG.md`:
   - Under today's date (create new `## YYYY-MM-DD` heading if needed).
   - One concise section describing what shipped, where it lives, how to verify.
   - Cite which subagent/tooling pass ran (or which were skipped + why).
6. Update `STATUS.md`:
   - `Last updated` → today's date (Stop hook maintains this; safety-check anyway).
   - `Currently working on` → next task in SPEC (or "nothing — awaiting /next").
   - `Next action` → concrete action user or Claude should take.
   - Add to `Recent milestones` if this was a phase-advancing ship.
7. If this was the last task in the current phase:
   - Check phase gate conditions.
   - If gates pass: mark phase `✅ COMPLETE`; promote next phase to `🟡 ACTIVE`.
   - If next phase has only placeholder tasks, flesh them out now based on TECH_APPROACH.md §8 + blueprint reference patterns.
8. Commit (the `pr-review-reminder.sh` hook fires; the `validate-commit.sh` chain runs).
9. Report to user:
   - What shipped (one sentence).
   - What's next.
   - Any blockers, flags, or skipped verification steps.

## If the task shouldn't actually be marked done

- If work is partial: describe what's left, keep `[ ]` status, don't run the rest of the procedure.
- If the task revealed it was wrong/obsolete: propose striking it and updating SPEC; require user confirmation.

## Skip conditions (rare; document in commit body)

- **Trivial commits** (typo / comment-only / SPEC/STATUS/CHANGELOG sync): may skip pr-review-toolkit per CLAUDE.md §6.3 smell test ("smell test: if you're about to do >30 minutes of focused work in one area, that's a subagent"). Below the threshold, a self-review pass is sufficient.
- **No-Unity commits** (pure MatchSim or pure-doc): step 4 (`unity-check`) is N/A.
- **No-MatchSim commits** (pure Unity-side or pure-doc): step 3 (`build-unity-plugins`) is N/A.

When skipping, name the skipped step in the commit body (e.g., "`unity-check` N/A: no Unity-side files touched").

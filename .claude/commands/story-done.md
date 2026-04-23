---
description: Mark story done with verification checks — AC coverage + code review + unity-check
argument-hint: "<story-file-path>"
---

# /story-done — close a story

End-of-story completion gate. Verifies every AC against the implementation, checks for silent GDD/ADR deviations, prompts code review, runs unity-check, updates story status, surfaces the next ready story.

**Phase:** 5. Run after `/dev-story` + `/code-review`. Distinct from project-level `/done` — that closes a phase task; this closes one story.

## Procedure

1. **Find the story.**
   - `<story-path>` provided → use that
   - No arg → check `production/session-state/active.md` or find in-progress stories in `sprint-status.yaml`
2. **Read story file** — extract Acceptance Criteria, Test Evidence path, Out of Scope, Governing ADR.
3. **AC verification.** For each AC item:
   - Identify the code change that implements it
   - Identify the test that covers it (in Test Evidence file)
   - Mark verified / unverified / partially-verified
4. **Test execution.**
   - Run the declared test file. If Unity: invoke `.claude/skills/unity-check/SKILL.md` at L2 (runtime).
   - If any AC is visual/feel: invoke unity-check at L3 (visual, with screenshot capture).
   - All tests pass + AC coverage complete → continue. Any fail → stop + report.
5. **GDD / ADR deviation check.** Grep the implementation for:
   - Any hard-coded value the GDD says comes from config
   - Any pattern the Governing ADR forbids
   - Any Out-of-Scope item that crept in
   If found: document as **Deviation** (not silent). Either fix or add `/log-decision` entry for intentional deviation.
6. **Code review gate.** Confirm `/code-review` was run (look for review report). If not, invoke it now and wait for completion.
7. **Update story file:**
   - Status: `Complete`
   - Completion date
   - Deviations (if any)
   - Final test evidence (run output path)
8. **Update `sprint-status.yaml`** — flip story status to `done`.
9. **Append to `CHANGELOG.md`** — one line under today's date pointing to the story.
10. **Surface next.** Glob `ready-for-dev` stories; recommend the next dependency-unblocked one.

## If args provided

- `<story-path>` → close that story

## If AC not fully covered

Do NOT mark done. Report unverified ACs + suggest returning to `/dev-story` for coverage.

## If test file doesn't exist

Block. "Story declared Test Evidence at <path> but file doesn't exist. Implementation incomplete."

## Output

- Updated story file (Status: Complete + deviations + evidence)
- `sprint-status.yaml` updated
- `CHANGELOG.md` updated
- Console: next ready story

## Related

- Typical follow-ups: `/dev-story <next>`, `/smoke-check` if sprint ending
- Invokes agents: optionally `qa-lead` for a cross-check review if in `full` review mode
- Invokes skills: `.claude/skills/unity-check/SKILL.md` (L2 / L3 as applicable), `/code-review` (if not already run)
- Reads files: story file, Test Evidence file, governing ADR, `sprint-status.yaml`
- Writes files: story file, `sprint-status.yaml`, `CHANGELOG.md`

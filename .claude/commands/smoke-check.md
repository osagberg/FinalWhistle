---
description: Quick smoke test — invokes unity-check L1 + L2
argument-hint: "[sprint | quick | --platform pc|mac|all]"
---

# /smoke-check — critical-path smoke test

Gate between "implementation done" and "ready for QA hand-off / phase gate". Runs compile + runtime verification via the `unity-check` skill, plus fast critical-path checks.

**Rule:** a build that fails smoke check does not go to QA or the next phase.

**Phase:** 5-7. Run after a sprint's stories are implemented; pre-commit on Unity-code changes; as input to `/gate-check`.

## Procedure

1. **Parse args.**
   - `sprint` (default) — full smoke check against current sprint's stories
   - `quick` — skip coverage scan, fast re-check only
   - `--platform pc|mac|all` — per-platform variants
2. **Detect test setup.**
   - `tests/` directory exists? If not, stop: "Run `/test-setup` or create manually."
   - Unity Test Framework configured? (check `unity-project/Packages/manifest.json`)
   - CI workflow present? Note for report.
3. **Invoke `unity-check` skill at L1.** Follow `.claude/skills/unity-check/SKILL.md` — compile verification, no missing scripts, no broken asmdef refs. If L1 fails, STOP, report, fix before continuing.
4. **Invoke `unity-check` skill at L2.** Runtime verification — play-mode smoke scene or batchmode entry. All critical-path assertions pass.
5. **Scene-meta validator** (if project has `scene-meta.yaml` convention) — every scene has a companion meta file. Fail if drift.
6. **Sprint story spot-checks** (sprint mode only). For each story closed this sprint:
   - Test file at declared Test Evidence path exists + compiles
   - Last test run result was PASS
   - Story file Status: Complete
7. **Platform variants** (if `--platform` passed):
   - `pc` — keyboard / mouse / windowed-mode assertions
   - `mac` — Apple Silicon native assertions, Metal renderer check
   - `all` — per-platform verdict table
8. **Verdict:** PASS / FAIL. No CONCERNS tier — smoke is binary.
9. **Write report** to `reviews/smoke-<date>.md` with per-check status.

## If args provided

- `sprint` / `quick` — as above
- `--platform <name>` — add platform-specific checks

## If unity-check L1 fails

STOP. Don't run L2. Report compile errors with file:line. User must fix before re-running.

## If no Unity project

If `unity-project/` not present, degrade to compile-only check via `dotnet build` (if applicable) and assertion-only smoke. Flag in report.

## Output

- `reviews/smoke-<date>.md`
- Console: PASS / FAIL + per-check detail

## Related

- Typical follow-ups (PASS): `/regression-suite update`, `/gate-check`, `/milestone-review`
- Typical follow-ups (FAIL): fix compile/runtime errors, re-run
- Invokes agents: none (L1/L2 are script-driven)
- Invokes skills: `.claude/skills/unity-check/SKILL.md` (L1 mandatory, L2 always)
- Reads files: `unity-project/`, `tests/**`, `production/sprint-status.yaml`
- Writes files: `reviews/smoke-<date>.md`

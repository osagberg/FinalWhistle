---
description: Scaffold implementation of one story — loads context, spawns programmer, writes code + test
argument-hint: "<story-path>"
---

# /dev-story — implement one story

Bridges planning and code. Reads a story file in full, assembles every context the programmer needs (story + GDD requirement + ADR guidance + control manifest), routes to the correct specialist agent, drives implementation to completion including the test file.

**Phase:** 5 (Production). The core implementation skill — run after `/story-readiness` PASS, before `/code-review` and `/story-done`.

## Procedure

1. **Find the story.**
   - `<story-path>` provided → read it directly
   - No arg → check active session state; else glob ready stories and ask via `AskUserQuestion`
2. **Load full context** (before any implementation):
   - Story file — extract TR-ID, governing ADR, AC, Dependencies, Out-of-Scope, Test Evidence path, Manifest Version
   - `docs/architecture/tr-registry.yaml` — look up current requirement text (source of truth; story body may be stale)
   - Governing ADR — Decision + Implementation Guidelines + Engine Compatibility sections
   - `docs/architecture/control-manifest.md` — layer rules (required / forbidden patterns, perf guardrails)
   - Any blocking files: if TR registry or governing ADR missing, **STOP** and report which.
3. **Manifest-version check.** Compare story's embedded Manifest Version vs current control-manifest header date. If different, ask via `AskUserQuestion` whether to update-and-implement-with-current-rules, implement-with-old-rules, or stop.
4. **Dependency validation.** For each story in Dependencies, verify Status: Complete. If any is not, **STOP** and report.
5. **Route to specialist.** Based on story type:
   - Logic → `gameplay-programmer` if installed, else `general-purpose`
   - UI → `ui-programmer` or `unity-ui-specialist`
   - Visual/Feel → `unity-shader-specialist` or `technical-artist`
   - Integration → `engine-programmer` or `unity-specialist`
   - Config/Data → `lead-programmer`
   Spawn via `Agent` tool with full context block assembled above.
6. **Implement code + test** in the specialist's session. Test path is the story's Test Evidence field.
7. **Per-AC verification.** For each Acceptance Criterion, confirm the implementation covers it. If an AC cannot be verified without runtime, invoke `.claude/skills/unity-check/SKILL.md` at L2 for Unity projects.
8. **Update session state.** Write what was implemented to `production/session-state/active.md`.
9. **Recommend next step:** `/code-review <changed-files>` then `/story-done <story-path>`

## If args provided

- `<story-path>` → implement that story

## If story is NOT READY

If `/story-readiness` hasn't been run or last verdict was NEEDS WORK / BLOCKED: stop and recommend `/story-readiness` first.

## If implementation fails compile / test

Do NOT mark story done. Leave in-progress. Report failure + file:line + proposed fix. Re-spawn specialist with error context.

## Output

- Source code in `Assets/_Project/Scripts/<System>/`
- Test file at the story's Test Evidence path
- Updated `production/session-state/active.md`

## Related

- Typical follow-ups: `/code-review <files>`, `/story-done <path>`
- Invokes agents: one of `gameplay-programmer`, `ui-programmer`, `unity-specialist`, `engine-programmer`, `lead-programmer` (by story type)
- Invokes skills: `.claude/skills/unity-check/SKILL.md` (L1 mandatory, L2 for runtime-affecting, L3 for visual)
- Reads files: story file, TR registry, governing ADR, control-manifest, CLAUDE.md
- Writes files: production code, test file, `production/session-state/active.md`

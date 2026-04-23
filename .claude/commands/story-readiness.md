---
description: Verify a story is implementation-ready BEFORE starting — READY / NEEDS WORK / BLOCKED
argument-hint: "<story-file-path | all | sprint>"
---

# /story-readiness — pre-dev validation gate

Validates that a story file contains everything a developer needs to begin implementation — no mid-sprint design interruptions, no guessing, no ambiguous acceptance criteria. Read-only; never edits stories.

**Phase:** 4-5. Run immediately before `/dev-story` on any story. Verdict: READY / NEEDS WORK / BLOCKED.

## Procedure

1. **Parse scope arg.**
   - `<path>` — one specific story
   - `all` — every story under `production/epics/`
   - `sprint` — stories with status `ready-for-dev` or `in-progress` in sprint-status.yaml
   - No arg — ask via `AskUserQuestion`
2. **For each story, check:**
   - **Story header completeness** — title, ID, type, TR-ID, governing ADR, Manifest Version present?
   - **AC specificity** — every AC item is concrete + testable? (no "works well", "feels good")
   - **Dependencies resolvable** — every listed dependency story exists and has Status: Complete (or story explicitly deferred)?
   - **Governing ADR exists** — file at declared path, Status: Accepted?
   - **TR-ID present in registry** — if `tr-registry.yaml` exists, the TR-ID resolves?
   - **Test Evidence path** — declared path is valid (follows `tests/<system>/test_*` convention)?
   - **Out-of-Scope present** — story has explicit boundaries? (prevents scope creep mid-dev)
   - **Open Questions empty** — if any "Open Question / Needs Design Input" bullet is present → BLOCKED
3. **Classify each story:**
   - **READY** — all checks pass
   - **NEEDS WORK** — minor gaps (missing Out-of-Scope, test path convention, etc.) — listed + fixable
   - **BLOCKED** — open design questions, missing governing ADR, unresolved dependency
4. **Report.** Table per story: ID | Verdict | Gaps (if any)
5. **If NEEDS WORK**, ask if user wants help filling gaps (don't auto-fill).
6. **If BLOCKED**, recommend specific remediation (`/quick-design` for design gap, `/architecture-decision` for ADR gap, `/create-stories` for dependency gap).

## If args provided

- `<path>` / `all` / `sprint` — as above

## If no stories exist

Fail: "No stories found under `production/epics/`. Run `/create-stories <epic>` first."

## Output

- Console table + per-story gap lists
- No file writes (read-only)

## Related

- Typical follow-ups (READY): `/dev-story <path>`
- Typical follow-ups (NEEDS WORK / BLOCKED): fix gaps → re-run
- Invokes agents: none (lightweight read-only checks)
- Invokes skills: none
- Reads files: story files, epic files, `docs/architecture/tr-registry.yaml`, governing ADRs
- Writes files: none

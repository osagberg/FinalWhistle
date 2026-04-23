---
description: Scaffold a hotfix branch + checklist for urgent post-release fixes
argument-hint: "<bug-id or short description>"
---

# /hotfix — emergency fix workflow

Emergency-fix workflow that bypasses the normal sprint process with a full audit trail. Creates a hotfix record, branch, and checklist. Ensures the fix is tested and backported correctly.

**Phase:** 7 (Release) — post-launch. Also valid pre-launch for S1/S2 blockers found during `/gate-check release`.

**Explicit invocation only.** Do not auto-invoke on context matching.

## Procedure

1. **Parse args.** Bug ID or short description (required). If missing, ask for a one-line description.
2. **Assess severity** via `AskUserQuestion`:
   - **S1 (Critical)** — game unplayable, data loss, security vulnerability → hotfix immediately
   - **S2 (Major)** — significant feature broken, workaround exists → hotfix within 24h
   - **S3 (Minor)** — not a hotfix candidate. Stop and recommend normal bug-fix flow (`/quick-design` + `/dev-story`).
3. **Draft hotfix record:**
   ```markdown
   ## Hotfix: <Short Description>
   Date: <today>
   Severity: S1 / S2
   Reporter: <who found it>
   Status: IN PROGRESS

   ### Problem
   <what is broken + player impact>

   ### Root Cause
   <to be filled during investigation>

   ### Fix
   <to be filled during implementation>

   ### Testing
   <what was tested and how>

   ### Approvals
   - [ ] Fix reviewed (/code-review)
   - [ ] Regression test authored + passing
   - [ ] /smoke-check PASS
   - [ ] Release approved

   ### Rollback Plan
   <how to revert if the fix breaks more>
   ```
4. **Write** to `production/hotfixes/hotfix-<date>-<short-name>.md` after user confirmation.
5. **Create hotfix branch** (user confirms):
   - `git checkout -b hotfix/<short-name>` from the release tag or `main`
   - Do NOT auto-push
6. **Investigate + implement.**
   - Reproduce the bug; confirm repro steps
   - Identify root cause; update hotfix record
   - Implement minimal fix (no feature work, no refactor — narrowest possible diff)
   - Author regression test that would have caught the original bug
7. **Verify:**
   - Invoke `.claude/skills/unity-check/SKILL.md` at L2 + L3 if visual
   - Invoke `/code-review` on changed files
   - Invoke `/smoke-check`
8. **Update hotfix record** — Fix section, Testing section, tick Approval checkboxes.
9. **Log decision** via `/log-decision` — one line citing the hotfix with severity + outcome.
10. **Recommend next step:** merge hotfix branch → release tag → `/release-checklist` verify → push release.

## If args provided

- `<description>` → skeleton pre-filled with that description

## If S3 or lower

Stop. "S3 doesn't warrant a hotfix. Use normal flow: `/quick-design` → `/create-stories` → `/dev-story`."

## If branch already exists

Ask whether to switch to existing hotfix branch or create a new one with suffix.

## Output

- `production/hotfixes/hotfix-<date>-<short-name>.md`
- New git branch (uncommitted until user pushes)
- Source code + regression test

## Related

- Typical follow-ups: `/code-review`, `/smoke-check`, merge → tag
- Invokes agents: `lead-programmer` for fix implementation; `qa-lead` for verification in `full` review mode
- Invokes skills: `.claude/skills/unity-check/SKILL.md`, `/code-review`, `/smoke-check`, `/log-decision`
- Reads files: bug report if referenced, relevant code
- Writes files: `production/hotfixes/hotfix-<date>-<short-name>.md`, source fix, test file, `SPEC.md` decisions log

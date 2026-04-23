---
description: Audit all project docs for staleness / contradictions and propose fixes
---

# /refresh-docs — keep docs fresh

Systematic staleness audit across the project's docs. Propose fixes; execute after user approval.

## Procedure

### 1. Collect doc set
Read in order:
- `CLAUDE.md`
- `PROJECT_CONTEXT.md`
- `TECH_APPROACH.md`
- `SPEC.md`
- `STATUS.md`
- `CHANGELOG.md`
- `SETUP.md`
- `TOOLING.md`
- `design/**/*.md`
- `design/**/*.yaml`
- `.claude/agents/**/*.md` (if any)

### 2. Run checks

**A. Archived-reference check** — project docs should not reference retired entities from `design/archive/*` or external archive folders:
- Identify per-project retired entities (names, features, systems) from the decisions log (entries flagged "DROPPED", "REJECTED", "SUPERSEDED")
- Grep active docs for those names
- Exceptions: `PROJECT_CONTEXT.md` and `SPEC.md` decisions log may reference these as deliberate archive pointers

**B. Contradiction check** — flag when two docs disagree:
- Engine version: all docs must match (Unity version string in CLAUDE.md / TECH_APPROACH / SETUP)
- Render pipeline (URP version): all docs must match
- Folder paths (project name propagation after any rename)
- Stack component names (e.g., "Magica Cloth 2" not "Magica Cloth"; "lilToon" not "LilToon")
- Cost / budget figures (keep in sync or pick a single authoritative location)
- Version pins of installed tools

**C. Task-status drift check** — SPEC.md task statuses should agree with CHANGELOG:
- For each `[x]` in SPEC, confirm a corresponding entry exists in CHANGELOG
- For each entry in CHANGELOG, confirm the matching SPEC task is `[x]`
- For each `🟡 ACTIVE` phase, confirm STATUS.md "Currently working on" reflects a task from that phase

**D. Placeholder check** — scan for unfilled template markers:
- Double-brace project/studio/genre markers left over from templates
- Fill-in, TBD, TODO, FIXME, or XXX markers outside intentional template files

**E. Orphan check** (from Phase 3 onward):
- `ScriptableObject` files not referenced from any scene or other SO
- Narrative files (`.ink` / `.yarn`) not referenced from any Dialog Runner SO
- `AnimationClip` files not referenced from any Animator / Animancer state

**F. Asset licensing coverage** (from Phase 3 onward):
- Every third-party asset in `Packages/` + `Assets/Plugins/` has a row in `asset-licensing-tracker.csv`

### 3. Report

Produce a table:
```
| Severity | File | Line | Issue | Proposed fix |
|----------|------|------|-------|--------------|
| high     | ...  | ...  | ...   | ...          |
```

Severity:
- `high` — contradictions, archived-references in active docs, task-status drift, missing license tracking
- `medium` — placeholder markers, stale cost figures, orphan assets
- `low` — style inconsistencies, ordering

### 4. Execute fixes

After user approval (per-finding or bulk approve):
- Apply edits
- Append a line to CHANGELOG: `/refresh-docs pass — fixed N findings`
- Update `STATUS.md` "Last updated" if anything changed

## Constraints

- Don't touch archived / retired folders — read-only reference
- Don't rewrite history in CHANGELOG (append-only)
- Don't modify decisions log entries in SPEC.md (append-only, supersede via new entries)

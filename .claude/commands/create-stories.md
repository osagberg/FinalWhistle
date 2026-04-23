---
description: Break one epic into implementable stories — each with TR-ID, ADR refs, AC, test path
argument-hint: "<epic-slug | epic-path>"
---

# /create-stories — break an epic into stories

A story is a single implementable behavior — small enough to complete in one focused session, self-contained, fully traceable to a GDD requirement and an ADR decision. Stories are what developers pick up via `/dev-story`.

**Phase:** 4-5. Run per epic, in dependency order. Output: `production/epics/<epic-slug>/story-NNN-<slug>.md` files.

## Procedure

1. **Parse arg.** `<epic-slug>` or full path to an `EPIC.md`. If missing, glob `production/epics/*/EPIC.md` and ask which epic via `AskUserQuestion`.
2. **Load context:**
   - The epic file (`EPIC.md`)
   - Governing GDD (path from epic)
   - Governing ADRs (list from epic)
   - `docs/architecture/control-manifest.md` if exists (layer rules)
   - `docs/architecture/tr-registry.yaml` if exists
3. **Precondition.** Epic status must be "Ready" (not "Not Started — GDD in flux"). If not, stop.
4. **Spawn Lead Programmer subagent** (`lead-programmer` if installed; else `general-purpose` with LP persona).
5. **Decompose epic into stories.** Use Story Seed List from EPIC.md as starting point. For each story:
   - Number sequentially (story-001, story-002, ...)
   - Classify type: **Logic** / **Integration** / **Visual-Feel** / **UI** / **Config-Data**
   - Map to GDD requirement TR-ID
   - Identify governing ADR
   - Author acceptance criteria (testable, verbatim)
   - List dependencies (other stories that must be done first)
   - Specify test evidence path (`tests/<system>/test_<story>_<scenario>.cs`)
   - Define Out of Scope bullets
   - Embed Manifest Version (today's control-manifest header date)
6. **Author each** `production/epics/<epic-slug>/story-NNN-<slug>.md` incrementally.
7. **Update `production/sprint-status.yaml`** — add new stories with status `ready-for-dev`.
8. **Recommend next step:** `/story-readiness <first-story-path>`

## If args provided

- `<epic-slug>` — resolve to `production/epics/<slug>/EPIC.md`
- Full path — read directly

## If epic has no Story Seed List

Fail: "Epic missing Story Seed List. Re-run `/create-epics <slug>` to author seeds first."

## If stories already exist for this epic

Don't overwrite. Ask: "Stories already exist for this epic. Append new ones, regenerate, or cancel?" via `AskUserQuestion`.

## Output

- `production/epics/<epic-slug>/story-NNN-<slug>.md` × N
- `production/sprint-status.yaml` updated

## Related

- Typical follow-ups: `/story-readiness <path>`, `/dev-story <path>`
- Invokes agents: `lead-programmer`
- Invokes skills: none
- Reads files: epic file, governing GDD, governing ADRs, control-manifest, tr-registry
- Writes files: story files under `production/epics/<epic-slug>/`, `production/sprint-status.yaml`

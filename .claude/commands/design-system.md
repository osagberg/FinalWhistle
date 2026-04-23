---
description: Design a new mechanic/system with Game Designer — outputs a full GDD
argument-hint: "<system-name> [retrofit <path>]"
---

# /design-system — author a GDD

Guided, section-by-section Game Design Document authoring for one system. Gathers context from existing docs, walks through 8 required sections, writes incrementally.

**Phase:** 2 (Design). Output: `design/gdd/<system-name>.md`.

## Procedure

1. **Parse args.** System name required (kebab-case). If missing, read `design/systems-index.md` and ask which system to design next. If `retrofit <path>`, enter retrofit mode (fill missing sections only, never overwrite existing content).
2. **Load context** before asking user anything:
   - `design/game-concept.md` — fail if missing ("run `/brainstorm` first")
   - `design/pillars.md` — pillars constrain scope
   - `design/systems-index.md` — fail if missing ("run `/map-systems` first")
   - Related GDDs (dependency systems listed in systems-index)
   - `design/art/art-bible.md` if exists (visual constraints)
3. **Spawn Game Designer subagent** with loaded context. Use `game-designer` agent if installed; else `general-purpose` with Game Designer persona.
4. **Author 8 required sections**, one at a time via `AskUserQuestion`:
   1. Overview (one paragraph)
   2. Player Fantasy (intended feeling)
   3. Detailed Rules (unambiguous mechanics)
   4. Formulas (all math with variables declared)
   5. Edge Cases (failure modes, boundary conditions)
   6. Dependencies (other systems this touches, bidirectional)
   7. Tuning Knobs (balance levers exposed to designer)
   8. Acceptance Criteria (testable outcomes)
5. **Write file** after each section (incremental, not single dump). User can course-correct between sections.
6. **Update `design/systems-index.md`** — flip this system's status from "Not Started" → "Designed"
7. **Recommend next step:** `/design-review <path>` then `/design-system` for next system

## If args provided

- `<system-name>` → author GDD for that name
- `retrofit <path>` → fill-in mode for existing incomplete GDD

## If system not in systems-index

Warn: "System not in systems-index. Add it, or run `/map-systems` to refresh the decomposition." Require user confirmation before proceeding.

## Output

- `design/gdd/<system-name>.md` (8 sections, all real content, no placeholders)
- `design/systems-index.md` updated

## Related

- Typical follow-ups: `/design-review`, `/design-system <next>`, `/review-all-gdds`
- Invokes agents: `game-designer`
- Invokes skills: none
- Reads files: `design/game-concept.md`, `design/pillars.md`, `design/systems-index.md`, related GDDs
- Writes files: `design/gdd/<system-name>.md`, `design/systems-index.md`

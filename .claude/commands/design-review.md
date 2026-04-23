---
description: Review a single GDD for completeness, consistency, implementability
argument-hint: "<path-to-design-doc> [--depth full|lean|solo]"
---

# /design-review — single-GDD review

Reviews one Game Design Document for completeness, internal consistency, implementability, and adherence to project design standards. Run before handing a GDD to programmers.

Distinct from `/review-all-gdds` (cross-GDD consistency + holism). This is per-doc.

**Phase:** 2-3. Verdict: APPROVED / NEEDS REVISION / MAJOR REVISION NEEDED.

## Procedure

1. **Parse args.**
   - `<path>` required — path to GDD file
   - `--depth` (optional): `full` (default; spawns specialists) / `lean` (single-session) / `solo` (minimal)
2. **Load target** GDD in full. Read `CLAUDE.md`, `design/game-concept.md`, `design/pillars.md`.
3. **Dependency graph validation.** For every system in the Dependencies section, glob `design/gdd/` — flag missing refs.
4. **Lore/narrative alignment.** If the project has narrative docs, spot-check against tone/world rules.
5. **Completeness check** — verify all 8 required sections present + substantive:
   - [ ] Overview
   - [ ] Player Fantasy
   - [ ] Detailed Rules
   - [ ] Formulas (with variables declared)
   - [ ] Edge Cases
   - [ ] Dependencies
   - [ ] Tuning Knobs
   - [ ] Acceptance Criteria (testable)
6. **Consistency check:**
   - Formulas use variables defined in Detailed Rules
   - AC items cover all major mechanics
   - Tuning Knobs map to actual values in formulas
7. **Implementability check:**
   - Can a programmer build this without asking design questions?
   - Are edge cases complete enough to code against?
   - Does it specify data-driven values clearly?
8. **Spawn Game Designer subagent** (in `full` depth) for holistic design critique. For narrative-heavy systems, also spawn `narrative-director`.
9. **Verdict:**
   - **APPROVED** — no gaps, ready for `/create-epics` or `/dev-story`
   - **NEEDS REVISION** — ≤3 small gaps fixable in one pass
   - **MAJOR REVISION NEEDED** — structural problems, missing sections, unimplementable
10. **Write report** to `design/reviews/<doc-name>-review-<date>.md` with findings + verdict.

## If args provided

- `<path>` → review that GDD
- `--depth full|lean|solo` → analysis depth

## If GDD path not found

Fail: "GDD not found at `<path>`. List with `ls design/gdd/`."

## If GDD is in retrofit state

Flag sections still marked `[To be designed]` as NEEDS REVISION (incomplete = not reviewable).

## Output

- `design/reviews/<doc-name>-review-<date>.md` (findings + verdict)
- Console: verdict + top findings

## Related

- Typical follow-ups (APPROVED): `/create-epics` or continue `/design-system`
- Typical follow-ups (NEEDS/MAJOR REVISION): `/design-system retrofit <path>` to fix gaps
- Invokes agents: `game-designer`, optionally `narrative-director`
- Invokes skills: none
- Reads files: target GDD, related GDDs, `game-concept.md`, `pillars.md`, `CLAUDE.md`
- Writes files: `design/reviews/<doc-name>-review-<date>.md`

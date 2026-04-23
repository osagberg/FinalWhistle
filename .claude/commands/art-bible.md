---
description: Author or review the project's visual style reference (gates asset production)
argument-hint: "[retrofit]"
---

# /art-bible — visual identity spec

Section-by-section authoring of the Art Bible. Locks visual identity before asset production begins. Every asset decision downstream should be traceable back to this doc.

**Phase:** 2 (after `/brainstorm`, before any asset work). Output: `design/art/art-bible.md`.

## Procedure

1. **Parse args.** If `retrofit`, enter fill-in mode — identify incomplete sections in existing file, author only those, never overwrite.
2. **Required read.** `design/game-concept.md` — fail if missing ("run `/brainstorm` first"). Extract: pitch, pillars, **Visual Identity Anchor** section if present.
3. **Detect retrofit automatically** if `design/art/art-bible.md` exists. Build section-status table; present to user; only work incomplete sections.
4. **Spawn Art Director subagent** (`art-director` if installed; else `general-purpose` with Art Director persona).
5. **Author 9 required sections**, one at a time via `AskUserQuestion`:
   1. Visual Identity Statement (one-line rule + 3-5 supporting principles)
   2. Color Palette (primary, accent, neutrals — with hex values)
   3. Lighting & Atmosphere (mood, time-of-day logic, shadow style)
   4. Character Art Direction (silhouette, proportion, detail density)
   5. Environment & Level Art (biome style, scale, composition rules)
   6. UI Visual Language (fonts, iconography, HUD tone)
   7. VFX & Particle Style (stylized vs realistic, color behavior)
   8. Asset Standards (resolution, poly budgets, naming conventions)
   9. Style Prohibitions ("never do X" — cast-iron no-list)
6. **Write** incrementally. User course-corrects between sections.
7. **Flag render pipeline + shader stack alignment.** If `TECH_APPROACH.md` declares URP + lilToon (or similar), confirm the Art Bible targets those.
8. **Recommend next step:** `/design-system <first-system>` (asset production can now begin in parallel)

## If args provided

- `retrofit` → fill-in mode only, per file-state table

## If project is non-character / non-narrative

Skip sections 4 + 7 if the game has no characters / no VFX by design. Document the skip explicitly in Section 1.

## Output

- `design/art/art-bible.md` (9 sections — all real content or explicitly-skipped)
- Console: "Art Bible authored. Asset production unblocked."

## Related

- Typical follow-ups: `/design-system`, `/design-review design/art/art-bible.md`
- Invokes agents: `art-director`
- Invokes skills: none
- Reads files: `design/game-concept.md`, `TECH_APPROACH.md`, existing `design/art/art-bible.md` (retrofit)
- Writes files: `design/art/art-bible.md`

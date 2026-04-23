---
description: Decompose concept into systems; produce dependency graph + priority-ordered index
argument-hint: "[next]"
---

# /map-systems — systems decomposition + dependency graph

Break the approved game concept into individual systems. Map dependencies. Prioritize design order. Produces the authoritative systems index used by every downstream skill.

**Phase:** 2 (Design). Output: `design/systems-index.md`.

## Procedure

1. **Parse args.**
   - No arg → full decomposition workflow (Phases 1-5)
   - `next` → pick highest-priority "Not Started" system from existing index; hand off to `/design-system`
2. **Required reads.**
   - `design/game-concept.md` — fail if missing ("run `/brainstorm` first")
   - `design/pillars.md` if it exists — pillars constrain priority
   - `design/systems-index.md` if it exists — resuming, not restarting
3. **Spawn Game Designer subagent** with loaded context.
4. **Decomposition** (5 phases interactively):
   1. **Identify** — list every system implied by the concept + pillars (brain-dump, don't prune yet)
   2. **Classify by layer** — Foundation / Core / Feature / Presentation
   3. **Map dependencies** — for each system, list which systems it reads from and writes to (bidirectional). Detect cycles; flag them as design problems.
   4. **Prioritize** — MVP / Stretch / Post-launch. Use pillars as tiebreaker.
   5. **Sequence** — topological order by dependency, breaking cycles where needed
5. **Author `design/systems-index.md`** with one row per system:
   ```
   | System | Layer | Priority | Status | Reads | Writes | GDD |
   ```
6. **Produce dependency diagram** (Mermaid text block inside the index) — one `graph TD` showing system → system edges, grouped by layer
7. **List conflicts** — cycles, missing owners, ambiguous boundaries — at bottom of index as "Open Questions"
8. **Recommend next step:** `/design-system <first-MVP-system>`

## If args provided

- `next` → short-circuit: read index, pick highest-priority "Not Started", suggest `/design-system <name>`

## If concept missing

Fail clearly: "No `design/game-concept.md`. Run `/brainstorm` first."

## Output

- `design/systems-index.md` (table + Mermaid graph + open questions)
- Console summary: N systems identified, M MVP-tier, cycles flagged

## Related

- Typical follow-ups: `/design-system <name>`, `/review-all-gdds`, `/art-bible`
- Invokes agents: `game-designer`
- Invokes skills: none
- Reads files: `design/game-concept.md`, `design/pillars.md`
- Writes files: `design/systems-index.md`

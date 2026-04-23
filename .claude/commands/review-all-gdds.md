---
description: Cross-check all GDDs for contradictions, stale refs, dominant strategies, pillar drift
argument-hint: "[full | consistency | design-theory | since-last-review]"
---

# /review-all-gdds — holistic cross-GDD audit

Reads every GDD simultaneously. Does two reviews that cannot be done per-GDD in isolation:

1. **Consistency** — contradictions, stale references, ownership conflicts between documents
2. **Design theory** — dominant strategies, broken economies, cognitive overload, pillar drift, competing progression loops

Distinct from `/design-review` (single-GDD completeness check).

**Phase:** 2-3 (before `/create-architecture`) and anytime a GDD is significantly revised. Output: `design/reviews/all-gdds-<date>.md`.

## Procedure

1. **Parse focus arg** (default `full`):
   - `consistency` — cross-GDD checks only
   - `design-theory` — holism checks only
   - `since-last-review` — only GDDs modified since last review (git-based)
2. **Load** all `design/gdd/*.md` + `design/pillars.md` + `design/systems-index.md`
3. **Spawn Game Designer** (or both `game-designer` and `narrative-director` for narrative-heavy projects) with full doc set loaded
4. **Consistency pass:**
   - Cross-ref stat/formula names — same stat defined differently in two GDDs = flag
   - Dependency reality-check — if GDD A says "depends on B", does B's GDD exist and agree?
   - Ownership conflicts — two systems claiming the same mechanic/state
   - Stale references — GDD mentions "see X" where X was retired
5. **Design theory pass:**
   - **Dominant strategies** — is there a combined tactic that trivializes intended play?
   - **Degenerate progression** — a loop that grinds optimal/easy instead of varied
   - **Pillar drift** — systems that subtly contradict the ranked pillars
   - **Cognitive load** — resource/state count per moment ≤ player can hold
   - **Economy sanity** — faucets / sinks roughly balance
6. **Report** to `design/reviews/all-gdds-<date>.md` with severity (HIGH / MEDIUM / LOW) + file + specific fix proposal for each finding
7. **Verdict:** PASS / CONCERNS / FAIL. CONCERNS = non-blocking; FAIL blocks `/create-architecture`.
8. **Recommend next step** based on verdict

## If args provided

- `full` / `consistency` / `design-theory` / `since-last-review` — as above

## If no GDDs exist

Fail: "No GDDs found in `design/gdd/`. Run `/design-system` for your MVP systems first."

## Output

- `design/reviews/all-gdds-<date>.md` (table of findings + verdict)
- Console verdict summary

## Related

- Typical follow-ups: fix findings → re-run until PASS, then `/create-architecture`
- Invokes agents: `game-designer`, optionally `narrative-director`
- Invokes skills: none
- Reads files: `design/gdd/**/*.md`, `design/pillars.md`, `design/systems-index.md`
- Writes files: `design/reviews/all-gdds-<date>.md`

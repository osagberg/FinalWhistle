---
description: Break a phase into epics (one per architectural module)
argument-hint: "[system-name | layer: foundation|core|feature|presentation | all]"
---

# /create-epics — break a phase into epics

An epic is a named, bounded body of work tied to one architectural module. Defines **what** needs to be built and **who owns it**. Does not prescribe implementation steps — that's what stories do.

**Phase:** 4-5 (Pre-Production / Production). Run per layer as you approach that layer. Output: `production/epics/<epic-slug>/EPIC.md` + `production/epics/index.md`.

## Procedure

1. **Parse args:**
   - `all` — process all systems in layer order
   - `layer: <name>` — one layer only (foundation / core / feature / presentation)
   - `<system-name>` — one specific system
   - No arg — ask via `AskUserQuestion`
2. **Precondition check.** `/architecture-review` must be PASS (or CONCERNS with override). If not, stop.
3. **Load inputs** (scoped to in-scope systems only):
   - `design/systems-index.md`
   - In-scope GDDs (status: Designed / Approved)
   - `docs/architecture/architecture.md` (module ownership)
   - Relevant ADRs (via architecture's module-to-ADR map)
   - `docs/architecture/tr-registry.yaml` if exists (GDD requirement index)
4. **Spawn Technical Director subagent** for module ownership scoping + Producer if installed for sprint sizing.
5. **Author one EPIC.md per in-scope system**, each with:
   - Title + epic-slug + status (Not Started)
   - Governing GDD (path) + Governing ADR list
   - Scope (in-scope / out-of-scope bullets)
   - Engine Risk (post-cutoff APIs, known Unity/URP risks)
   - Dependencies (other epics required first)
   - Acceptance Criteria (epic-level — what makes this module "done")
   - Story Seed List (bullet list, NOT full stories yet — those come from `/create-stories`)
6. **Update `production/epics/index.md`** — master table of all epics with status
7. **Recommend next step:** `/create-stories <first-foundation-epic>`

## If args provided

- `all` — author all layers in dependency order (Foundation → Core → Feature → Presentation)
- `layer: <name>` — one layer
- `<system-name>` — one epic

## If a system has no GDD

Flag + skip that system. Recommend `/design-system <name>` to fill the gap before epic-ing it.

## If the phase isn't ready for this layer

If you're asked to create Feature-layer epics but Core isn't approaching done: warn. "Core-layer epics still in progress. Feature GDDs may drift — recommend waiting." Require explicit user override.

## Output

- `production/epics/<slug>/EPIC.md` × N
- `production/epics/index.md`

## Related

- Typical follow-ups: `/create-stories <epic-slug>`
- Invokes agents: `technical-director`, optionally `producer`
- Invokes skills: none
- Reads files: `design/systems-index.md`, in-scope GDDs, `architecture.md`, relevant ADRs
- Writes files: `production/epics/<slug>/EPIC.md`, `production/epics/index.md`

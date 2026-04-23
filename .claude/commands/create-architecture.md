---
description: Author the master architecture document from all approved GDDs
argument-hint: "[full | layers | data-flow | api-boundaries | adr-audit]"
---

# /create-architecture — master architecture blueprint

Produces `docs/architecture/architecture.md` — the whole-system technical blueprint that translates every approved GDD into a concrete engineering plan. Sits between design and implementation. Must exist before sprint planning begins.

Distinct from `/architecture-decision` (point decisions / ADRs). This is the whole-system context that gives ADRs their meaning.

**Phase:** 3 (Technical Setup). Output: `docs/architecture/architecture.md` + initial ADR list.

## Procedure

1. **Parse focus arg** (default `full`):
   - `full` — all sections
   - `layers` — system layer diagram only
   - `data-flow` — module-to-module data flow only
   - `api-boundaries` — API definitions only
   - `adr-audit` — audit existing ADRs for engine-compatibility gaps only
2. **Load context** (critical — do not skip):
   - `CLAUDE.md` (engine stack)
   - `TECH_APPROACH.md` (locked tech choices)
   - All `design/gdd/*.md` (source requirements)
   - `design/systems-index.md` (layer + dependency truth)
   - `design/pillars.md`
   - `docs/engine-reference/<engine>/VERSION.md` + `breaking-changes.md` + `current-best-practices.md` if present — flag post-cutoff APIs
3. **Precondition check.** `/review-all-gdds` must be PASS (or CONCERNS with user override). If not, stop: "Architecture built on inconsistent GDDs inherits inconsistencies. Run `/review-all-gdds` first."
4. **Spawn Technical Director subagent** with full context. Use `technical-director` if installed; else `general-purpose` with TD persona + Unity-URP grounding.
5. **Author sections**, one at a time via `AskUserQuestion`:
   1. Layer Architecture (Foundation / Core / Feature / Presentation; assembly definitions)
   2. Module Registry (one row per module: name, owner, consumers, assembly)
   3. Data Flow (how state moves between modules)
   4. API Boundaries (public interfaces between layers)
   5. Persistence (save format, versioning, migration)
   6. Engine Integration (URP config, plugins, shader pipeline)
   7. Asset Pipeline (import, addressables, build layout)
   8. Required ADRs (list of decisions that must be recorded via `/architecture-decision`)
6. **Write** `docs/architecture/architecture.md` incrementally.
7. **Output Required ADR list** at the end — user runs `/architecture-decision` for each.
8. **Recommend next step:** `/architecture-decision <first ADR title>`

## If args provided

- `layers` / `data-flow` / `api-boundaries` → focused pass on that section only; existing sections preserved
- `adr-audit` → read all existing ADRs, check for missing Engine Compatibility sections

## If GDDs incomplete

Fail fast — don't author architecture on partial GDDs. Recommend `/design-system` for missing systems.

## Output

- `docs/architecture/architecture.md`
- Required ADR list (at bottom of the doc + in console)

## Related

- Typical follow-ups: `/architecture-decision` (×N), `/architecture-review`
- Invokes agents: `technical-director`
- Invokes skills: none
- Reads files: `CLAUDE.md`, `TECH_APPROACH.md`, `design/gdd/**`, `design/systems-index.md`, engine-reference library
- Writes files: `docs/architecture/architecture.md`

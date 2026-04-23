---
description: Author a new ADR with structured Context / Decision / Consequences
argument-hint: "<title> | retrofit <path>"
---

# /architecture-decision — author an ADR

Create an Architecture Decision Record. Every significant technical choice (library, pattern, boundary, format) gets one. ADRs give the decisions log its engineering depth.

**Phase:** 3 (initial batch from `/create-architecture`'s Required ADR list) + any phase when a new architectural choice emerges. Output: `docs/architecture/adr-<NNN>-<slug>.md`.

## Procedure

1. **Parse args.**
   - `<title>` — author a new ADR with that title
   - `retrofit <path>` — fill missing sections of an existing ADR (don't overwrite existing content)
   - No args — ask for title via `AskUserQuestion`
2. **Number the ADR** — glob `docs/architecture/adr-*.md`, find highest NNN, increment.
3. **Load context:**
   - `docs/architecture/architecture.md` (parent blueprint)
   - All related GDDs (via systems-index lookup on the decision's domain)
   - Prior ADRs this might supersede or depend on
   - Engine-reference breaking-changes.md (for post-cutoff API risk)
4. **Spawn Technical Director subagent** with loaded context.
5. **Author sections** via `AskUserQuestion`, one at a time:
   - **Status** — Proposed / Accepted / Deprecated / Superseded (default Proposed)
   - **Context** — problem, constraints, alternatives considered (list 2-4)
   - **Decision** — what was chosen, in imperative voice
   - **Consequences** — what becomes easier, what becomes harder, what new risks exist
   - **GDD Requirements Addressed** — TR-IDs from design docs
   - **ADR Dependencies** — which prior ADRs this depends on / supersedes
   - **Engine Compatibility** — post-cutoff API notes, known Unity/URP risks
   - **Implementation Guidelines** — what the programmer should follow
6. **Write** `docs/architecture/adr-<NNN>-<slug>.md`
7. **If this supersedes a prior ADR**, update the prior ADR's Status to `Superseded by ADR-<NNN>` (user confirmation)
8. **Update `SPEC.md` decisions log** via `/log-decision` — one line pointing to the ADR
9. **Recommend next step:** next item from `architecture.md`'s Required ADR list, or `/architecture-review` if list complete

## If args provided

- `<title>` → new ADR with that title
- `retrofit <path>` → fill-in mode (only missing sections authored)

## If the decision is actually a design choice (not architectural)

Redirect: "This sounds like a design choice, not an architectural one. Use `/log-decision` for a lightweight record, or `/quick-design` for a spec."

## Output

- `docs/architecture/adr-<NNN>-<slug>.md`
- `SPEC.md` decisions log updated
- Console: "ADR-<NNN> authored. Next Required ADR: ..."

## Related

- Typical follow-ups: next ADR from Required list, `/architecture-review`
- Invokes agents: `technical-director`
- Invokes skills: `/log-decision`
- Reads files: `docs/architecture/architecture.md`, prior ADRs, engine-reference
- Writes files: `docs/architecture/adr-<NNN>-<slug>.md`, `SPEC.md` decisions log

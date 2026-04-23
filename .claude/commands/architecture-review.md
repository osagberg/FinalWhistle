---
description: Validate architecture against all GDDs — traceability + coverage + engine compatibility
argument-hint: "[full | coverage | consistency | engine | single-gdd <path>]"
---

# /architecture-review — architecture gate-check

Validates that the body of architectural decisions covers every GDD requirement, is internally consistent, and correctly targets the pinned engine version. The architecture equivalent of `/design-review`. PASS/CONCERNS/FAIL verdict.

**Phase:** 3 closing, and anytime a new ADR or GDD is added mid-production. Gate between Technical Setup and Pre-Production.

## Procedure

1. **Parse focus** (default `full`):
   - `coverage` — traceability only (which GDD requirements have no ADR)
   - `consistency` — cross-ADR conflict detection only
   - `engine` — engine-compatibility audit only
   - `single-gdd <path>` — one GDD's architecture coverage
2. **Load** every `docs/architecture/adr-*.md`, `docs/architecture/architecture.md`, all GDDs, `design/systems-index.md`, engine-reference library.
3. **Spawn Technical Director subagent.**
4. **Traceability matrix** — build GDD requirement → ADR mapping:
   - For each GDD, extract requirements (TR-IDs from Acceptance Criteria + Formulas)
   - For each requirement, find the governing ADR
   - Output table; flag requirements with no ADR as **Coverage Gaps**
5. **Cross-ADR conflicts** — read every ADR's Decision + Consequences:
   - Two ADRs mandating incompatible patterns (e.g., one says "Addressables-only", another uses `Resources.Load`)
   - Dependency cycles in ADR graph (ADR-A depends on ADR-B which depends on ADR-A)
   - Stale references (ADR cites a superseded prior ADR)
6. **Engine compatibility** — per-ADR check:
   - Does the ADR cite a post-cutoff API that isn't in `docs/engine-reference/<engine>/breaking-changes.md`?
   - Does it use a deprecated API from `deprecated-apis.md`?
7. **Verdict:**
   - **PASS** — no coverage gaps, no conflicts, all ADRs engine-compatible
   - **CONCERNS** — ≤2 MEDIUM-severity findings, no blockers (user may override)
   - **FAIL** — any HIGH-severity finding (coverage gap on an MVP system, cross-ADR conflict, engine incompatibility)
8. **Write report** to `docs/architecture/reviews/review-<date>.md`
9. **Recommend next step:**
   - PASS → `/gate-check technical-setup` (phase transition)
   - CONCERNS → review findings, decide override vs fix
   - FAIL → list specific ADRs to add or fix

## If args provided

- `coverage` / `consistency` / `engine` / `single-gdd <path>` — scoped focus

## If no ADRs exist

Fail: "No ADRs found. Run `/create-architecture` then `/architecture-decision` per Required ADR item."

## Output

- `docs/architecture/reviews/review-<date>.md` (traceability matrix, conflicts, verdict)
- Console: PASS / CONCERNS / FAIL + top-3 findings

## Related

- Typical follow-ups (PASS): `/gate-check technical-setup`, `/create-epics`
- Typical follow-ups (FAIL): `/architecture-decision` for gaps
- Invokes agents: `technical-director`
- Invokes skills: none
- Reads files: all ADRs, `architecture.md`, all GDDs, `design/systems-index.md`, engine-reference
- Writes files: `docs/architecture/reviews/review-<date>.md`

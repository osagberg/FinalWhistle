---
description: Traceability matrix mapping design doc requirements → ADRs → implementation → tests. Living doc; kept current by /architecture-review.
---

<!-- USAGE
Living document. Regenerated (or audited) by the /architecture-review skill
after each ADR or GDD change. Do not edit manually unless fixing an error.

The matrix exists for one reason: prove every GDD requirement has an ADR that
satisfies it, and every shipped system has a test that covers it. Gaps are
technical debt — visible, prioritized, tracked.

Cross-refs:
  - design-templates/architecture-decision-record.md (ADR rows)
  - design-templates/game-design-document.md         (GDD requirement rows)
  - design-templates/test-plan.md                    (test file rows)
  - design-templates/systems-index.md                (system → row pivot)
-->

# Architecture Traceability: {{PROJECT_NAME}}

**Last Updated**: <fill-in: YYYY-MM-DD>
**Engine**: Unity 6 LTS <fill-in: version>
**GDDs Indexed**: <fill-in: N>
**ADRs Indexed**: <fill-in: M>
**Last Review**: <fill-in: link to docs/architecture/review-YYYY-MM-DD.md>

---

## Coverage Summary

| Status | Count | Percentage |
|---|---|---|
| Covered (ADR + impl + test) | <fill-in> | <fill-in>% |
| Partial (ADR exists, impl or test missing) | <fill-in> | <fill-in>% |
| Gap (no ADR coverage) | <fill-in> | <fill-in>% |
| **Total requirements** | **<fill-in>** | |

---

## Traceability Matrix

One row per technical requirement. A technical requirement is any GDD
statement implying a specific architectural decision: data structures,
performance budgets, engine capabilities, cross-system communication,
persistence.

| Req ID | GDD | System | Requirement Summary | ADR(s) | Script(s) | SO(s) | Test File | Status |
|---|---|---|---|---|---|---|---|---|
| TR-<fill-in: 001> | [match-engine.md](../design/match-engine.md) | MatchSim | <fill-in: "same seed produces same canonical hash"> | ADR-0003 | `MatchSim/Sim/Tick.cs` | `MatchConfigSO.cs` | `MatchSim.Tests/DeterminismTest.cs` | Covered |
| TR-<fill-in: 002> | <fill-in> | <fill-in> | <fill-in> | — | — | — | — | Gap |

---

## Gaps by Layer (Work-Order)

Prioritize fixing gaps top-down. Foundation-layer gaps block everything
downstream.

### Foundation Layer Gaps (BLOCKING — must close before coding)

- [ ] TR-<fill-in>: <fill-in: requirement> — Suggested ADR: *"<fill-in: title>"*

### Core Layer Gaps (close before Core system is built)

- [ ] TR-<fill-in>: <fill-in>

### Feature Layer Gaps (close before feature sprint)

- [ ] TR-<fill-in>: <fill-in>

### Presentation Layer Gaps (can defer to implementation time)

- [ ] TR-<fill-in>: <fill-in>

---

## Cross-ADR Conflicts

Pairs of ADRs making contradictory claims. Must be resolved — write a
superseding ADR that picks one side and marks the other Superseded.

| Conflict ID | ADR A | ADR B | Type | Status |
|---|---|---|---|---|
| CONFLICT-<fill-in: 001> | ADR-<fill-in> | ADR-<fill-in> | <fill-in: data ownership / API contract / lifecycle> | <fill-in: Unresolved / Resolved by ADR-NNNN> |

---

## ADR → GDD Coverage (Reverse Index)

For each ADR, which GDD requirements does it address?

| ADR | Title | GDD Requirements Addressed | Engine Knowledge Risk |
|---|---|---|---|
| ADR-<fill-in: 0001> | <fill-in: title> | TR-<fill-in>, TR-<fill-in> | <fill-in: LOW/MED/HIGH> |

---

## Superseded Requirements

Requirements that existed when an ADR was written, but have since changed in
the GDD. The ADR may need updating.

| Req ID | GDD | What Changed | Affected ADR | Action |
|---|---|---|---|---|
| TR-<fill-in> | <fill-in: path> | <fill-in: old rule → new rule> | ADR-<fill-in> | <fill-in: ADR needs update / re-verify> |

---

## How to Use This Document

**When writing a new ADR** — add it to the "ADR → GDD Coverage" table. Mark the
requirements it satisfies as Covered in the matrix.

**When approving a GDD change** — scan the matrix for requirements from that
GDD, check whether the change invalidates any existing ADR. Add to "Superseded
Requirements" if so.

**When running `/architecture-review`** — the skill regenerates this document
from the current state of GDDs + ADRs + code.

**Gate check** — the Pre-Production gate requires zero Foundation Layer gaps.
The Pre-Release gate requires zero Unresolved Cross-ADR Conflicts.

---

## Next Steps

- [ ] Run `/architecture-review` after every ADR change
- [ ] Close Foundation Layer gaps before Phase 4 impl begins
- [ ] Resolve all Cross-ADR Conflicts before release gate

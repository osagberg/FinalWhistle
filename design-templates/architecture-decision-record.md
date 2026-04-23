---
description: ADR — Architecture Decision Record. One per significant technical decision. Numbered ADR-001, ADR-002... Append-only once Accepted.
---

<!-- USAGE
Create one ADR per architectural decision that you want to look back on in 6
months and understand "why did we do that". Number sequentially — ADR-001,
ADR-002. File naming: `docs/architecture/adr-0001-[slug].md`.

**HARD RULE — never edit after "Accepted"**. To reverse an accepted ADR,
create a new ADR that marks the old one "Superseded by ADR-NNNN" and cite it.
This matches the append-only decisions log pattern.

When to write an ADR:
- Choosing a library / plugin / framework where alternatives exist
- Architecting a system boundary (which asmdef, which SO layout)
- Decisions that will cost non-trivial rework if reversed later
- Any decision where engine knowledge risk is MEDIUM or HIGH

When NOT to write an ADR:
- Small tactical choices (variable names, local refactors)
- Decisions covered by existing ADRs (cite the existing one)

Cross-refs:
  - design-templates/architecture-traceability.md (ADR coverage of GDD requirements)
  - design-templates/game-design-document.md      (ADRs satisfy GDD requirements)
  - SPEC.md decisions log                         (ADRs are per-project; decisions log is per-session)
-->

# ADR-<fill-in: NNNN>: <fill-in: Title>

## Status

<fill-in: Proposed | Accepted | Deprecated | Superseded by ADR-MMMM>

## Date

<fill-in: YYYY-MM-DD>

## Last Verified

<fill-in: YYYY-MM-DD — when this ADR was last re-read and confirmed accurate
against the current engine version. Update even if nothing changed.>

## Decision Makers

<fill-in: who was involved>

---

## Summary

<fill-in: 2 sentences. Problem + chosen approach. Scannable — a skill checking
20 ADRs reads this to decide whether to go deeper.>

## Engine Compatibility

| Field | Value |
|---|---|
| Engine | Unity 6 LTS (<fill-in: specific version, e.g., 6000.4.3f1>) |
| Domain | <fill-in: Rendering / Physics / UI / Audio / Animation / Input / Scripting / Core> |
| Knowledge Risk | <fill-in: LOW (pre-cutoff) / MEDIUM (near cutoff, verify) / HIGH (post-cutoff, must verify)> |
| References Consulted | <fill-in: docs, URLs, context7 queries> |
| Post-Cutoff APIs Used | <fill-in: specific APIs or "None"> |
| Verification Required | <fill-in: behaviors to test against target Unity version before ship> |

> If Knowledge Risk is MEDIUM/HIGH, this ADR must be re-validated on Unity
> version upgrades. Flag as Superseded and write a replacement.

## Dependencies

| Field | Value |
|---|---|
| Depends On | <fill-in: ADR-NNNN must be Accepted first, or "None"> |
| Enables | <fill-in: ADR-NNNN that this unlocks> |
| Blocks | <fill-in: epic/story that can't start until this is Accepted> |

---

## Context

### Problem Statement

<fill-in: what problem forces a decision now? cost of not deciding?>

### Current State

<fill-in: how does it work today? what's wrong?>

### Constraints

- <fill-in: technical — engine limits, platform reqs>
- <fill-in: timeline — deadline pressure>
- <fill-in: resource — solo dev time, expertise>
- <fill-in: compatibility — existing systems>

### Requirements

- <fill-in: functional reqs>
- <fill-in: performance req — specific + measurable>

---

## Decision

<fill-in: the specific decision, detailed enough to implement without further
clarification.>

### Architecture Sketch

```
<fill-in: ASCII diagram — components, data flow direction, key interfaces>
```

### Key Interfaces

```csharp
// <fill-in: pseudocode or real C# interface — the contract this ADR creates>
```

### Implementation Guidelines

<fill-in: specific guidance for the programmer — asmdef boundaries, threading
rules, allocation constraints.>

---

## Alternatives Considered

### Alternative 1: <fill-in: name>

- **Description** — <fill-in>
- **Pros** — <fill-in>
- **Cons** — <fill-in>
- **Rejected because** — <fill-in>

### Alternative 2: <fill-in: name>

<fill-in: same structure>

---

## Consequences

### Positive

- <fill-in>

### Negative (Accepted Tradeoffs)

- <fill-in>

### Neutral

- <fill-in>

---

## Performance Implications

| Metric | Before | Expected After | Budget |
|---|---|---|---|
| CPU frame time | <fill-in> | <fill-in> | <fill-in> |
| Memory | <fill-in> | <fill-in> | <fill-in> |
| Build size | <fill-in> | <fill-in> | <fill-in> |

---

## GDD Requirements Addressed

Every ADR must trace to at least one GDD requirement OR explicitly declare
itself foundational.

| GDD | System | Requirement | How This ADR Satisfies It |
|---|---|---|---|
| <fill-in: path> | <fill-in> | <fill-in: e.g., "hitbox resolution within 1 frame"> | <fill-in> |

If foundational (no direct GDD dep): *"Foundational — enables: <fill-in: what
systems this unlocks>"*

---

## Migration Plan (if replacing existing approach)

1. <fill-in: step — what changes, how to verify>
2. <fill-in>

**Rollback**: <fill-in: how to revert if this decision proves wrong>

---

## Validation Criteria

- [ ] <fill-in: measurable criterion 1>
- [ ] <fill-in: criterion 2>
- [ ] <fill-in: performance criterion>

---

## Related

- <fill-in: related ADRs — supersedes / contradicts / depends on>
- <fill-in: code files once implemented — use markdown links>

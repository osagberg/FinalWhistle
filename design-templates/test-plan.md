---
description: Test plan per feature/sprint. Classifies each story as EditMode / PlayMode / Manual / Playtest. Drives QA work order.
---

<!-- USAGE
Create one test plan per sprint or per feature. Maps stories to verification
method: EditMode test (unit), PlayMode test (integration), manual QA, or
playtest. Generated first-pass by /qa-plan skill; refined by hand.

Test classifications:
  - Logic            → EditMode unit test
  - Integration      → PlayMode test (coroutine, scene, async)
  - Visual/Feel      → manual + screenshot evidence
  - UI               → manual step-through
  - Config/Data      → SO validator + spot-check

Cross-refs:
  - design-templates/test-evidence.md         (post-hoc record of what passed)
  - design-templates/game-design-document.md  (AC traces from GDD into this plan)
  - .claude/rules/tests/RULES.md              (how tests must be structured)
-->

# Test Plan: <fill-in: Sprint / Feature Name>

**Date**: <fill-in: YYYY-MM-DD>
**Scope**: <fill-in: N stories across N systems>
**Engine**: Unity 6 LTS <fill-in: version>
**Sprint file**: <fill-in: path to sprint plan if applicable>

---

## Story Coverage Summary

| Story | Type | Automated? | Manual? |
|---|---|---|---|
| <fill-in> | Logic | EditMode — `Tests/EditMode/<System>/<Story>Test.cs` | — |
| <fill-in> | Integration | PlayMode — `Tests/PlayMode/<System>/<Story>Test.cs` | smoke-check |
| <fill-in> | Visual/Feel | — | screenshot + sign-off |
| <fill-in> | UI | — | manual step-through |
| <fill-in> | Config/Data | SO validator | spot-check values in-game |

**Totals**: <fill-in: N Logic / N Integration / N Visual/Feel / N UI / N Config/Data>

---

## Automated Tests

### <fill-in: Story Title> — Logic (EditMode)

**File**: `Assets/_Project/Tests/EditMode/<fill-in: System>/<fill-in: Story>Test.cs`

**What to test**:
- <fill-in: formula from GDD — e.g., "damage = base * (1 + str/100) holds for str ∈ {0, 50, 100, 200}">
- <fill-in: named state transition>
- <fill-in: side effect that should fire / should not fire>

**Edge cases**:
- Zero / minimum input
- Maximum / boundary input
- Invalid / null input
- GDD-declared edge cases

**Estimated count**: ~<fill-in: N> unit tests

---

### <fill-in: Story Title> — Integration (PlayMode)

**File**: `Assets/_Project/Tests/PlayMode/<fill-in: System>/<fill-in: Story>Test.cs`

**What to test**:
- <fill-in: cross-system — e.g., "applying buff updates PlayerStatsSO and fires StatsChanged event">
- <fill-in: round-trip — e.g., "save → load restores all fields byte-identical">
- <fill-in: scene lifecycle — e.g., "loading scene B unloads scene A's Addressables handles">

---

## Manual QA

### <fill-in: Story Title> — Visual/Feel

**Method**: screenshot / video capture + designer sign-off
**Evidence file**: `docs/qa/evidence/<fill-in: slug>.md` (see [test-evidence.md](test-evidence.md))
**Sign-off**: <fill-in: designer / art-lead / self>

- [ ] <fill-in: specific observable condition — e.g., "hit flash appears on the frame of impact, not the frame after">
- [ ] <fill-in: another falsifiable condition>

### <fill-in: Story Title> — UI

**Method**: manual step-through against UX spec
**Evidence file**: `docs/qa/evidence/<fill-in: slug>.md`

- [ ] <fill-in: each acceptance criterion from [ux-spec.md](ux-spec.md) translated into a check>

---

## Smoke Test Scope

Critical paths before QA hand-off:

1. Game boots to main menu without crash.
2. New game / new session starts.
3. <fill-in: primary mechanic changed this sprint>.
4. <fill-in: system with regression risk from this sprint>.
5. Save / load cycle completes without data loss.
6. Performance on target hardware stays within budget (see TECH_APPROACH.md).

---

## Playtest Requirements

| Story | Goal | Min Sessions | Target Player Type |
|---|---|---|---|
| <fill-in> | <fill-in: question to answer> | <fill-in: N> | <fill-in: new / experienced> |

Sign-off → [playtest-report.md](playtest-report.md), stored at
`docs/qa/playtests/<fill-in: sprint>-<fill-in: slug>.md`.

If none required: *No playtest sessions this sprint.*

---

## Performance Tests (if applicable)

Use `Unity.PerformanceTesting`. Budgets from TECH_APPROACH.md / relevant ADR.

| Story | Metric | Budget | Test File |
|---|---|---|---|
| <fill-in> | <fill-in: frame time / mem / load> | <fill-in> | `Tests/Performance/<fill-in>Test.cs` |

---

## Definition of Done (this sprint)

- [ ] All AC verified — automated pass OR documented manual evidence
- [ ] Test file exists for every Logic + Integration story and passes
- [ ] Manual evidence document exists for every Visual/Feel + UI story
- [ ] Smoke test passes before hand-off
- [ ] No regressions introduced (prior-sprint tests still pass)
- [ ] Code reviewed (self-review against rules/* OR peer review)
- [ ] Story marked complete via `/done` (updates SPEC + CHANGELOG + STATUS)

**Stories requiring playtest sign-off**: <fill-in: list or "none">

---

## Results (fill in after testing)

| Story | Automated | Manual | Result | Notes |
|---|---|---|---|---|
| <fill-in> | PASS | — | PASS | |
| <fill-in> | — | PASS | PASS | |
| <fill-in> | FAIL | — | BLOCKED | <fill-in: reason> |

---

## Bugs Found

| ID | Story | Severity | Description | Status |
|---|---|---|---|---|
| BUG-<fill-in> | <fill-in> | <fill-in: S1-S4> | <fill-in> | Open |

---

## Sign-Off

- **Developer** — <fill-in> — <fill-in: date>
- **QA (self)** — <fill-in> — <fill-in: date>

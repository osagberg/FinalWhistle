---
description: Post-hoc test execution record. What was tested, results, anomalies, sign-off. Per-story or per-sprint.
---

<!-- USAGE
Written after executing the plan in test-plan.md. One evidence doc per
Visual/Feel or UI story — these can't be asserted by an automated test alone,
so the record IS the evidence. Logic/Integration stories produce PASS/FAIL
output from the Unity Test Runner and usually don't need this.

Store at: `docs/qa/evidence/<story-slug>.md`
Adjacent screenshots: same folder or `docs/qa/evidence/<story-slug>/`.

Cross-refs:
  - design-templates/test-plan.md           (the plan this evidence answers)
  - design-templates/game-design-document.md (AC come from GDD)
  - design-templates/playtest-report.md     (if session-based)
-->

# Test Evidence: <fill-in: Story Title>

**Story**: <fill-in: path to story file / SPEC.md ref>
**Story Type**: <fill-in: Visual/Feel | UI | Playtest>
**Date**: <fill-in: YYYY-MM-DD>
**Tester**: <fill-in: name / "self">
**Build / Commit**: <fill-in: version + git short sha>
**Platform**: <fill-in: e.g., macOS 15 / Unity 6000.4.3f1 Editor Play Mode>

---

## What Was Tested

<fill-in: one paragraph describing the feature or behavior validated. Include
the AC numbers from the story that this evidence covers.>

**Acceptance criteria covered**: <fill-in: AC-1, AC-2, AC-3>

---

## Acceptance Criteria Results

| # | Criterion (verbatim from story) | Result | Notes |
|---|---|---|---|
| AC-1 | <fill-in: exact criterion text> | PASS / FAIL | <fill-in> |
| AC-2 | <fill-in> | PASS / FAIL | <fill-in> |
| AC-3 | <fill-in> | PASS / FAIL | <fill-in> |

---

## Screenshots / Video

Store captures adjacent to this doc or under `<story-slug>/` subfolder.

| # | File | What It Shows | AC |
|---|---|---|---|
| 1 | `<fill-in: filename.png>` | <fill-in: brief description> | AC-1 |
| 2 | `<fill-in: filename.png>` | <fill-in> | AC-2 |

If video: note the timestamp ranges and what each segment demonstrates.

---

## Test Conditions

- **Game state at start** — <fill-in: e.g., "fresh save, player at level 1, no items, Main scene loaded">
- **Platform / hardware** — <fill-in>
- **Framerate during test** — <fill-in: e.g., "stable 60fps" or "~45fps — within budget per TECH_APPROACH">
- **Special setup** — <fill-in: e.g., "DebugManager.UnlockAll() invoked to bypass gating">

---

## Observations

Noteworthy things that didn't cause a FAIL but are worth recording. Candidates
for polish tickets.

- <fill-in: observation — e.g., "combat text jitters briefly on first spawn; not reproducible after">
- <fill-in>

If nothing notable: *No significant observations.*

---

## Anomalies + Bugs Found

Open new tickets for anything that caused a FAIL or that you'd want revisited.

| ID | Severity | Description | Status |
|---|---|---|---|
| BUG-<fill-in> | <fill-in: S1-S4> | <fill-in> | Open |

---

## Sign-Off

Required before the story can be marked COMPLETE via `/done`.

| Role | Name | Date | Status |
|---|---|---|---|
| Developer (impl) | <fill-in> | <fill-in> | [ ] Approved |
| Design / Art review | <fill-in> | <fill-in> | [ ] Approved |
| QA (self) | <fill-in> | <fill-in> | [ ] Approved |

Any sign-off may be marked `Deferred — <fill-in: reason>`. Deferred sign-offs
must be resolved before the story advances past sprint review.

---

## Regression Watch

If this story fixed a bug, add a row to the regression suite so it can't
silently come back.

| Regression Test Added? | Location | Covers |
|---|---|---|
| <fill-in: Y/N> | <fill-in: test file path> | <fill-in: bug ID> |

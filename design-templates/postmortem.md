---
description: After-action report structure. What went well / what didn't / what we'd change / action items. Per-milestone or per-release.
---

<!-- USAGE
Written after a milestone completes or a release ships. Scheduled within ~2
weeks of the event while details are fresh. Not a blame document — the goal
is to extract repeatable lessons and identify systemic issues.

One post-mortem per: Phase gate crossed, major release, abandoned feature,
significant pivot. Archived alongside the project — never deleted.

Cross-refs:
  - design-templates/release-checklist-template.md (if this post-mortem is for a release)
  - design-templates/playtest-report.md            (playtest trends feed in)
  - SPEC.md decisions log                          (corrections may flow back)
  - CHANGELOG.md                                   (events referenced here)
-->

# Post-Mortem: <fill-in: Milestone / Release / Pivot Name>

**Date**: <fill-in: YYYY-MM-DD>
**Facilitator**: <fill-in: self / producer>
**Period Covered**: <fill-in: start> → <fill-in: end>
**Participants**: <fill-in: self / collaborators>

---

## Summary

<fill-in: 2-3 sentences. What this milestone/project set out to do, what it
actually delivered, and the one-sentence verdict (success / partial / missed /
pivoted).>

---

## Goals vs Results

| Goal | Target | Result | Status |
|---|---|---|---|
| <fill-in> | <fill-in: measurable target> | <fill-in: actual> | <fill-in: Met / Partial / Missed> |
| <fill-in> | <fill-in> | <fill-in> | <fill-in> |

---

## Timeline

Key events in chronological order. Focus on turning points — not every commit.

| Date | Event | Impact |
|---|---|---|
| <fill-in> | <fill-in: what happened> | <fill-in: how it changed trajectory> |

---

## What Went Well

### <fill-in: Category — e.g., Technical Execution>

- **What** — <fill-in: description>
- **Why it worked** — <fill-in: root cause>
- **How to repeat** — <fill-in: action to keep doing>

### <fill-in: Category — e.g., Tooling Discipline>

- **What** — <fill-in>
- **Why it worked** — <fill-in>
- **How to repeat** — <fill-in>

### <fill-in: Category — e.g., Scope Management>

- **What** — <fill-in>
- **Why it worked** — <fill-in>
- **How to repeat** — <fill-in>

---

## What Went Poorly

Focus on systems and root causes, not individuals. Every "what went poorly"
should produce at least one action item.

### <fill-in: Category — e.g., Estimation Accuracy>

- **What** — <fill-in: description>
- **Root cause** — <fill-in: the real WHY — "too busy" is not a root cause>
- **Impact** — <fill-in: time / quality / morale cost>
- **Prevention** — <fill-in: how to avoid next time>

### <fill-in: Category — e.g., Scope Creep>

- **What** — <fill-in>
- **Root cause** — <fill-in>
- **Impact** — <fill-in>
- **Prevention** — <fill-in>

### <fill-in: Category>

- **What** — <fill-in>
- **Root cause** — <fill-in>
- **Impact** — <fill-in>
- **Prevention** — <fill-in>

---

## What We'd Do Differently

One-line summaries of the biggest lessons. The "if I had the time machine"
list.

1. <fill-in>
2. <fill-in>
3. <fill-in>

---

## Key Metrics

| Metric | Target | Actual | Notes |
|---|---|---|---|
| Tasks completed | <fill-in> | <fill-in> | <fill-in> |
| Bugs found | — | <fill-in> | |
| Bugs fixed | — | <fill-in> | |
| Estimation accuracy (plan vs actual) | 100% | <fill-in>% | <fill-in> |
| Scope changes | 0 | <fill-in> | <fill-in> |
| Days over target | 0 | <fill-in> | <fill-in> |

---

## Lessons Learned

Specific enough to alter future behavior. Vague lessons ("communicate better")
don't count.

1. **<fill-in: lesson>** — <fill-in: what changes in future work>
2. **<fill-in: lesson>** — <fill-in>
3. **<fill-in: lesson>** — <fill-in>

---

## Action Items

| # | Action | Owner | Deadline | Status |
|---|---|---|---|---|
| 1 | <fill-in: specific + testable> | <fill-in> | <fill-in: YYYY-MM-DD> | Open |
| 2 | <fill-in> | <fill-in> | <fill-in> | Open |

Action items flow into: SPEC.md next-phase tasks, CHANGELOG fixes, or new ADRs
for systemic corrections.

---

## Acknowledgments

<fill-in: exceptional contributions — self-recognition OK for solo dev.
Documenting what specifically worked reinforces it.>

---

## Follow-Up Review

**Scheduled date**: <fill-in: +3 months from post-mortem date>
**Who tracks action-item closure**: <fill-in: self>

Before closing this post-mortem at follow-up review, confirm:
- [ ] All P0/P1 action items closed
- [ ] Lessons applied to in-flight work — visible in SPEC + CHANGELOG
- [ ] Prevention measures in place (hooks, rules, validators) for top issue

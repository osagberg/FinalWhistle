---
description: Playtest session report — participant (anonymized) / scenario / observed behaviors / bug list / qualitative feedback.
---

<!-- USAGE
Written after a playtest session (self-playtest, friends-and-family, external
tester). One report per session. Anonymize participant info for privacy —
reference them as "Tester A / Tester B" or a one-letter handle unless they
have consented to be named in project records.

Playtest purpose is to answer ONE question declared up-front in the Session
Goal. Don't fish for generic feedback — target a specific hypothesis.

Cross-refs:
  - design-templates/test-plan.md             (playtest requirements declared there)
  - design-templates/test-evidence.md         (companion for non-playtest manual QA)
  - design-templates/game-design-document.md  (playtest validates GDD AC)
  - design-templates/postmortem.md            (playtest trends feed post-mortem)
-->

# Playtest Report: <fill-in: Session ID / Date>

**Session Date**: <fill-in: YYYY-MM-DD>
**Session Length**: <fill-in: minutes>
**Build / Commit**: <fill-in: version + git sha>
**Facilitator**: <fill-in: self / other>
**Report Author**: <fill-in>

---

## Session Goal

**Primary hypothesis tested**: <fill-in: one sentence, e.g., "Does the stealth
mechanic read clearly enough that new players use it within the first 10
minutes without explicit tutorial?">

**Success criterion**: <fill-in: observable outcome that confirms/denies the
hypothesis — e.g., "Tester successfully hides from 2+ patrols using stealth
without hint being shown.">

---

## Participant Profile (Anonymized)

| Attribute | Detail |
|---|---|
| Handle | Tester <fill-in: A / B / 01> |
| Experience level | <fill-in: new to genre / mid-core / hardcore> |
| Platform | <fill-in: PC KB+M / controller / Steam Deck> |
| Prior sessions with this build | <fill-in: 0 / N> |
| Known biases (if any) | <fill-in: e.g., "loves stealth games — results may over-index"> |

> Do not record: name, contact info, location, or any PII beyond what's needed
> to interpret results. Participants referenced by handle only.

---

## Scenario

**What the tester was asked to do**:

<fill-in: scenario brief. What scene did they start in? What state? What
goal? What instructions were given, what were withheld?>

**What they were NOT told**:

<fill-in: deliberately-withheld info — e.g., "tester was not told stealth
existed; discovery of it is part of what's being tested">

---

## Observed Behaviors

Record what actually happened, in rough chronological order. Observation > interpretation.

| Time | Event | Observation |
|---|---|---|
| <fill-in: 0:00> | Session start | <fill-in: started in Main scene, paused 3s at main menu> |
| <fill-in: 0:30> | <fill-in: event> | <fill-in: what the tester did> |
| <fill-in: 2:15> | <fill-in> | <fill-in> |

### Key moments

- **Moment of confusion** — <fill-in: when + what + how long>
- **Moment of delight** — <fill-in>
- **Unexpected solution** — <fill-in>
- **Stuck point** — <fill-in>

---

## Hypothesis Result

**Primary hypothesis (from §Session Goal)**: <fill-in: CONFIRMED / DENIED / INCONCLUSIVE>

**Evidence**:

<fill-in: cite specific observed behaviors that support the verdict>

**Confidence**: <fill-in: High / Medium / Low>

---

## Qualitative Feedback

Paraphrased or direct quotes from the tester. Mark direct quotes with `> `.

- <fill-in: paraphrased — "tester said they didn't notice the health bar during combat">
- > <fill-in: direct quote — "I kept trying to jump, but I thought the button was broken">
- <fill-in>

---

## Bugs Found

| ID | Severity | Description | Repro Steps | Status |
|---|---|---|---|---|
| BUG-<fill-in> | <fill-in: S1-S4> | <fill-in> | <fill-in: 1. 2. 3.> | Open |

---

## Design Issues (Non-bug)

Things that work as designed but felt wrong to the tester. Candidates for
design revision.

- <fill-in: issue — e.g., "crafting menu too deep — tester gave up after 3 submenu levels">
- <fill-in>

---

## Action Items

| # | Action | Priority | Owner | Target Sprint |
|---|---|---|---|---|
| 1 | <fill-in> | <fill-in: P0-P3> | <fill-in> | <fill-in> |

---

## Data Handling

- [ ] Session video (if recorded) stored in gitignored `Captures/playtests/<fill-in: session-id>/`
- [ ] PII removed from this report
- [ ] Participant notified of conclusion if requested

---

## Next Session

**Recommended follow-up**: <fill-in: re-test after fixes, or new hypothesis>
**Participant type needed**: <fill-in: same profile / different profile>

---
description: 3-5 design pillars + anti-pillars for the game. The tiebreaker when two design decisions conflict. Approved once, amended rarely.
---

<!-- USAGE
Written after game-concept.md is approved. Pillars are constraints, not
features: a good pillar must be falsifiable (make a claim that could be wrong)
and must force saying "no" to some plausible ideas. If a pillar never resolves
a design conflict, it's too vague.

Amending pillars is a major event — triggers a full GDD re-review. Log every
pillar change as a decision in SPEC.md.

Cross-refs:
  - design-templates/game-concept.md   (pillars derive from §Core Fantasy + §USPs)
  - design-templates/game-design-document.md (every GDD cites the pillars served)
  - SPEC.md decisions log              (pillar approvals + amendments go here)
-->

# Game Pillars: {{PROJECT_NAME}}

**Status**: <fill-in: Draft | Approved>
**Version**: <fill-in: 1.0>
**Last Updated**: <fill-in: YYYY-MM-DD>

---

## What Pillars Are

Pillars are 3-5 non-negotiable principles that define this game's identity.
Every design, art, audio, narrative, and technical decision must serve at least
one pillar. When two choices conflict, the higher-ranked pillar wins.

A good pillar is:

- **Falsifiable** — makes a testable claim. "Fun gameplay" isn't a pillar.
  "Combat rewards patience over aggression" is.
- **Constraining** — forces saying no to something. If it never eliminates an
  option, it's too vague.
- **Cross-departmental** — shapes design, art, audio, narrative, AND engineering.
- **Memorable** — team can recite all pillars from memory.

---

## Core Fantasy Served

<fill-in: one paragraph restating the core fantasy from game-concept.md. All
pillars trace back to this.>

---

## The Pillars (Ranked by Conflict Priority)

### Pillar 1: <fill-in: name>

**Definition**: <fill-in: one falsifiable sentence>

**Design test**: <fill-in: "If we're debating between X and Y, this pillar says
we choose Z.">

**What this means per department**:

| Department | Constraint |
|---|---|
| Design | <fill-in> |
| Art | <fill-in> |
| Audio | <fill-in> |
| Narrative | <fill-in> |
| Engineering | <fill-in> |

**Serves**: <fill-in: examples of decisions that embody this>
**Violates**: <fill-in: examples of decisions that betray this>

---

### Pillar 2: <fill-in: name>

**Definition**: <fill-in>

**Design test**: <fill-in>

| Department | Constraint |
|---|---|
| Design | <fill-in> |
| Art | <fill-in> |
| Audio | <fill-in> |
| Narrative | <fill-in> |
| Engineering | <fill-in> |

**Serves**: <fill-in>
**Violates**: <fill-in>

---

### Pillar 3: <fill-in: name>

**Definition**: <fill-in>

**Design test**: <fill-in>

| Department | Constraint |
|---|---|
| Design | <fill-in> |
| Art | <fill-in> |
| Audio | <fill-in> |
| Narrative | <fill-in> |
| Engineering | <fill-in> |

**Serves**: <fill-in>
**Violates**: <fill-in>

---

### Pillar 4: <fill-in: name — optional, delete if only 3>

<fill-in: same structure>

### Pillar 5: <fill-in: name — optional>

<fill-in: same structure>

---

## Anti-Pillars — What This Game Is NOT

Anti-pillars prevent scope creep. Every "no" protects the "yes." A good
anti-pillar is something the team might plausibly want to do. "NOT a racing
game" is obvious and useless. "NOT an open-world game" is useful if the genre
could plausibly support it.

- **NOT <fill-in: thing>** — <fill-in: why this is excluded and what it would cost>
- **NOT <fill-in: thing>** — <fill-in>
- **NOT <fill-in: thing>** — <fill-in>

---

## Conflict Resolution

When pillars conflict, use the priority ranking above. Process:

1. Identify which pillars are in tension.
2. The higher-ranked pillar wins.
3. If the lower-ranked pillar can be partially served without compromising the
   higher one, do so.
4. Document the decision in the relevant GDD or ADR.
5. If pillars seem fundamentally irreconcilable, escalate to revise the pillar
   set itself (major event — log in SPEC.md decisions).

---

## Validation Checklist

Before marking Approved:

- [ ] Count is 3-5 (no more, no fewer)
- [ ] Each pillar is falsifiable (could be wrong)
- [ ] Each pillar constrains — forces saying no to plausible ideas
- [ ] Each pillar has cross-departmental implications
- [ ] Each pillar has a concrete design test
- [ ] At least 3 explicit anti-pillars defined
- [ ] Pillars priority-ranked for conflict resolution
- [ ] Every pillar traces back to the core fantasy
- [ ] Team can recite all pillars from memory

---

## Next Steps

- [ ] Pillar review with creative lead / solo self-review
- [ ] Cite these pillars in [game-concept.md](game-concept.md)
- [ ] Update every existing [game-design-document.md](game-design-document.md) with Pillars Served field
- [ ] Log approval in `SPEC.md` decisions log

---
description: Full Game Design Document template. Per-system or whole-game. Formulas stay as formulas, never prose. Eight sections Overview through Scope.
---

<!-- USAGE
One GDD per system (e.g., combat.md, progression.md, economy.md) OR one master
GDD for small games. Created after game-concept.md is approved. Each system
GDD becomes the source of truth for that system — all ADRs and implementation
trace back here.

Rule: formulas stay as formulas. "Damage scales with strength" is not enough —
write `damage = base * (1 + str/100)`. Prose descriptions of math drift;
executable-looking formulas don't.

Cross-refs:
  - design-templates/game-concept.md              (parent — concept feeds GDD)
  - design-templates/game-pillars.md              (every GDD cites pillars served)
  - design-templates/systems-index.md             (GDD is indexed here)
  - design-templates/architecture-decision-record.md (ADRs reference GDD requirements)
  - design-templates/test-plan.md                 (test plan verifies GDD formulas)
-->

# GDD: {{PROJECT_NAME}} — <fill-in: System Name>

**Status**: <fill-in: Draft | In Review | Approved | Implemented>
**Author**: <fill-in>
**Last Updated**: <fill-in: YYYY-MM-DD>
**Last Verified**: <fill-in: YYYY-MM-DD — when the doc was last re-read against current code>
**Pillars Served**: <fill-in: pillar names from game-pillars.md>

---

## 1. Overview

<fill-in: 2-3 paragraphs. What this system is, what the player does with it,
why it exists, and how it contributes to the core fantasy. Written so a new
team member can understand the system without reading further.>

**Quick reference** — Layer: `<fill-in: Foundation | Core | Feature | Presentation>` · Priority: `<fill-in: MVP | Vertical Slice | Alpha | Full>` · Key deps: `<fill-in: system names or None>`

---

## 2. Pillars

Which game pillars does this system serve, and how?

| Pillar | How This System Serves It |
|---|---|
| <fill-in: pillar name> | <fill-in: concrete mechanical contribution> |
| <fill-in: pillar name> | <fill-in> |

---

## 3. Core Loop

How this system's moment-to-moment play feels.

- **Input** — <fill-in: what player does>
- **Response** — <fill-in: what the game does back>
- **Feedback** — <fill-in: visual + audio + haptic>
- **Stakes** — <fill-in: what succeeds or fails from the action>

---

## 4. Mechanics & Rules

Precise, unambiguous rules. A programmer should implement this section without
asking questions.

### 4.1 Rules

1. <fill-in: numbered rule>
2. <fill-in>
3. <fill-in>

### 4.2 States & Transitions

| State | Entry Condition | Exit Condition | Behavior |
|---|---|---|---|
| <fill-in> | <fill-in> | <fill-in> | <fill-in> |

### 4.3 Formulas

Formulas stay as formulas. Every variable typed, ranged, and sourced.

#### <fill-in: Formula name — e.g., Damage>

```
damage = base * (1 + str / 100) * crit_mult
```

| Variable | Type | Range | Source | Description |
|---|---|---|---|---|
| `base` | float | 1–999 | weapon SO | Weapon base damage |
| `str` | int | 0–200 | stats SO | Player strength stat |
| `crit_mult` | float | 1.0 or 2.0 | combat rules | 2.0 on crit, 1.0 otherwise |

**Expected output range**: <fill-in>
**Edge case**: <fill-in: e.g., clamp below 1 → minimum 1 damage>

### 4.4 Edge Cases

| Scenario | Expected Behavior | Rationale |
|---|---|---|
| <fill-in: what if X is zero?> | <fill-in> | <fill-in> |
| <fill-in: both effects trigger?> | <fill-in> | <fill-in> |

---

## 5. Progression

How the player's engagement with this system deepens over time.

- **Early game** — <fill-in: introductory mechanics and power level>
- **Mid game** — <fill-in: added complexity, new tools>
- **Late game** — <fill-in: mastery expression, edge cases rewarded>

| Tuning Knob | Current | Safe Range | Effect ↑ | Effect ↓ |
|---|---|---|---|---|
| <fill-in> | <fill-in> | <fill-in> | <fill-in> | <fill-in> |

---

## 6. Narrative Integration

How this system intersects with story, character, and world.

- **Diegetic framing** — <fill-in: in-world explanation for the system>
- **Characters affected** — <fill-in: which cast members this touches>
- **Story beats driven by** — <fill-in: plot moments that use this system>

---

## 7. Audio / Visual

### 7.1 Visual Requirements

| Event | Visual Feedback | Priority |
|---|---|---|
| <fill-in> | <fill-in> | <fill-in: High/Med/Low> |

### 7.2 Audio Requirements

| Event | Audio Feedback | Priority |
|---|---|---|
| <fill-in> | <fill-in> | <fill-in> |

### 7.3 Game Feel Targets

| Action | Max Latency (ms) | Frame Budget @60fps | Feel Goal |
|---|---|---|---|
| <fill-in> | <fill-in> | <fill-in> | <fill-in: snappy / weighty / floaty> |

---

## 8. Scope

### 8.1 In Scope

- <fill-in>
- <fill-in>

### 8.2 Out of Scope (Explicit)

- <fill-in: defer to alpha>
- <fill-in: defer to post-launch>

### 8.3 Content Inventory

| Content Type | MVP Count | Alpha Count | Full Vision Count |
|---|---|---|---|
| <fill-in: e.g., weapons> | <fill-in: 3> | <fill-in: 12> | <fill-in: 30> |

---

## Cross-References

| This Document References | Target Doc | Specific Element | Nature |
|---|---|---|---|
| <fill-in: e.g., "combo multiplier"> | `design/gdd/score.md` | `combo_multiplier` | Data dependency |

---

## Acceptance Criteria

- [ ] <fill-in: testable criterion 1>
- [ ] <fill-in: criterion 2>
- [ ] System update completes within <fill-in>ms per frame
- [ ] No hardcoded values — all tuning via ScriptableObject

---

## Open Questions

| Question | Owner | Deadline |
|---|---|---|
| <fill-in> | <fill-in> | <fill-in> |

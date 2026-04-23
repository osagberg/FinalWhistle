---
description: UX specification for a single screen or flow. Wireframe + interaction flows + accessibility + platform variants.
---

<!-- USAGE
One UX spec per screen or flow (inventory, settings, pause menu, dialogue,
save/load, main menu). For persistent gameplay overlays (health bar, minimap,
ammo counter) use hud-design.md instead.

Scope test: if the player navigates to it explicitly → UX spec. If it appears
while the player is controlling their character → HUD.

Cross-refs:
  - design-templates/game-design-document.md (parent — GDD drives what UI exists)
  - design-templates/hud-design.md            (sibling — HUD overlays live there)
  - design-templates/architecture-decision-record.md (ADRs on UI Toolkit vs uGUI, etc.)
  - design-templates/test-plan.md             (UI test plans reference this)
-->

# UX Spec: {{PROJECT_NAME}} — <fill-in: Screen/Flow Name>

**Status**: <fill-in: Draft | Approved | Implemented>
**Screen ID**: <fill-in: code/ticket identifier, e.g., `InventoryScreen`>
**Platform Targets**: <fill-in: PC | Steam Deck | Console>
**Related GDDs**: <fill-in: links to GDD § UI Requirements>
**Related ADRs**: <fill-in: ADR on UI Toolkit / uGUI, input model, etc.>
**Accessibility Tier**: <fill-in: Basic | Standard | Comprehensive | Exemplary>

---

## 1. Purpose + Player Need

**What player need does this screen serve?**

<fill-in: one paragraph. Name the real human need, not the system function.
Bad: "Displays the player's current items." Good: "Lets the player decide what
to equip for the next encounter without breaking the mental model of the game
world.">

**Player goal** (what the player wants): <fill-in: one sentence testable>

**Game goal** (what the system needs from this interaction): <fill-in>

---

## 2. Player Context on Arrival

| Question | Answer |
|---|---|
| What was the player just doing? | <fill-in> |
| Emotional state? | <fill-in: tense / calm / curious / frustrated> |
| Cognitive load? | <fill-in: high — tracking enemies / low — safe state> |
| Primary use case on this screen? | <fill-in> |
| What are they afraid of? | <fill-in: missing something / irreversible mistake> |

**Emotional design target**: <fill-in: one sentence on the feeling this screen should produce>

---

## 3. Navigation + Modal Behavior

**Hierarchy**:
```
<fill-in: Root → Parent → This Screen → Children>
```

**Modal?** <fill-in: Modal (blocks below) / Non-modal / Overlay (pauses) / Overlay-live (game continues)>

**Entry points**:

| Trigger | Source | Data In |
|---|---|---|
| <fill-in> | <fill-in: screen/state> | <fill-in: payload> |

**Exit points**:

| Exit Action | Destination | Data Committed |
|---|---|---|
| <fill-in: Back/B> | <fill-in> | <fill-in> |

---

## 4. Wireframe

```
<fill-in: ASCII wireframe. Suggested chars:
 ┌ ┐ └ ┘ │ ─ for borders
 [ ] for buttons
 { } for content areas
 ... for scrollable regions
 ● for focused element on open>

┌───────────────────────────────────────┐
│  [← Back]    SCREEN TITLE    [Menu]   │  ← HEADER
├───────────────────────────────────────┤
│  ● first focused element              │
│  ...                                  │
├───────────────────────────────────────┤
│  [Primary Action]     [Secondary]     │  ← ACTION BAR
└───────────────────────────────────────┘
```

### Zones

| Zone | Size | Scrollable | Overflow |
|---|---|---|---|
| Header | full × 10% | no | truncate with ellipsis |
| <fill-in> | <fill-in> | <fill-in> | <fill-in> |

### Components Inventory

| Component | Type | Zone | Reuses Existing? |
|---|---|---|---|
| <fill-in> | <fill-in: Button/Label/List> | <fill-in> | <fill-in: yes — NavButton / no — new> |

**Primary focus on open**: <fill-in: which element is focused>

---

## 5. States + Variants

| State | Trigger | Visual | Behavior | Notes |
|---|---|---|---|---|
| Loading | <fill-in> | Skeleton shimmer | All inputs disabled except Close | Should resolve <500ms |
| Empty | <fill-in> | EmptyState component | <fill-in> | No disabled buttons — hide them |
| Populated | <fill-in> | <fill-in> | <fill-in> | Default happy path |
| Error | <fill-in> | Retry + Close | <fill-in> | Don't expose tech detail |
| <fill-in> | <fill-in> | <fill-in> | <fill-in> | <fill-in> |

---

## 6. Interaction Map

### Navigation

| Input | Platform | Action | Feedback |
|---|---|---|---|
| Arrow keys / D-Pad | all | Move focus within zone | Focus ring + soft tick |
| Tab / R1 | KB / Gamepad | Next zone | Distinct zone-change tone |
| Mouse click | PC | Select + focus | Click SFX + pressed state |

### Actions

| Input | Context | Action | Feedback |
|---|---|---|---|
| Enter / A | <fill-in: context> | <fill-in> | <fill-in> |
| Esc / B | any | Close + commit | Exit transition |

---

## 7. Data Requirements

UI reads data; UI does not own data. UI fires events; UI does not mutate state
directly.

| Data Element | Source System | Update Trigger | Format | Null Handling |
|---|---|---|---|---|
| <fill-in> | <fill-in: SO / service> | <fill-in: on event X> | <fill-in> | <fill-in: empty state> |

---

## 8. Events Fired

| Player Action | Event | Payload | Receiver | Notes |
|---|---|---|---|---|
| <fill-in> | <fill-in: `EquipItemRequested`> | `{itemId, slot}` | Equipment service | Fires and waits for ack |

---

## 9. Transition + Animation

| Transition | Duration (ms) | Easing | Skipped by Reduced Motion? |
|---|---|---|---|
| Screen enter | 250 | Ease out cubic | Yes — instant at 0ms |
| Screen exit | 200 | Ease in cubic | Yes |
| Content update | 120 | Linear | Yes |

---

## 10. Accessibility

**Contrast** — all text ≥4.5:1 vs background in all states.

**Colorblind safety** — no element relies on color alone. Use icon/shape/label
as redundant indicator.

**Focus** — always visible. Focus trap inside modals. Tab order = reading order.

**Text scaling** — support 75% → 150% without clipping.

**Reduced motion** — all transitions listed in §9 instant at 0ms.

**Screen reader** — key state changes announced:

| State Change | Announcement |
|---|---|
| Screen opens | "<fill-in: title>. <fill-in: count> items." |
| Item focused | "<fill-in: name>. <fill-in: key detail>." |

---

## 11. Platform Variants

| Platform | Adaptation |
|---|---|
| PC (KB+M) | Base layout |
| Steam Deck | <fill-in: touch targets ≥ 44×44, controller-first defaults> |
| Gamepad-only | <fill-in: no mouse-dependent actions, full D-Pad coverage> |

---

## 12. Acceptance Criteria

- [ ] All elements reachable by keyboard (Tab + arrow)
- [ ] All elements reachable by gamepad (D-Pad + face buttons)
- [ ] Focus visible at all times
- [ ] All states render without overlap/truncation at 1280×720 min resolution
- [ ] All events in §8 fire with correct payload on all exit paths
- [ ] Screen does not write directly to any system (only fires events)
- [ ] Reduced Motion setting instants all §9 transitions
- [ ] All text ≥4.5:1 contrast in all states

---

## Open Questions

| Q | Owner | Deadline |
|---|---|---|
| <fill-in> | <fill-in> | <fill-in> |

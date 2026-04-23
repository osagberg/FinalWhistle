---
description: HUD design — persistent in-game overlays (health, ammo, minimap, subtitles). Single doc per game; zones, states, urgency, accessibility.
---

<!-- USAGE
One HUD design doc per game. Specifies every element that overlays the game
world during active play. For screens the player navigates explicitly
(inventory, menus, pause), use ux-spec.md.

Scope test: if it's visible while the player is controlling their character
without pausing, it belongs here.

Cross-refs:
  - design-templates/ux-spec.md              (sibling — menu screens)
  - design-templates/game-design-document.md (every GDD with UI needs cites this)
  - design-templates/architecture-decision-record.md (ADR on HUD framework)
-->

# HUD Design: {{PROJECT_NAME}}

**Status**: <fill-in: Draft | Approved | Implemented>
**Platform Targets**: <fill-in: PC / Steam Deck / Console>
**Accessibility Tier**: <fill-in: Basic | Standard | Comprehensive | Exemplary>

---

## 1. Philosophy

**Relationship with on-screen info**:

<fill-in: one paragraph design stance. Example — "The world is the interface.
If the player has to look away from the environment to survive, the HUD has
failed.">

**Default when ambiguous**: <fill-in: SHOW / HIDE / CONTEXTUAL>

**Rule of Necessity**: "A HUD element earns its place when <fill-in: specific
condition>."

---

## 2. Information Architecture

Every game-world info type gets an explicit decision. No "we'll figure that out later."

| Info Type | Always Show | Contextual | On Demand | Hidden (Diegetic) | Reasoning |
|---|---|---|---|---|---|
| Health | <fill-in: x> | | | | <fill-in> |
| Primary resource | | <fill-in: x — show when consuming> | | | <fill-in> |
| Secondary resource | | | <fill-in: x — menu only> | | <fill-in> |
| Minimap | <fill-in> | <fill-in> | <fill-in> | <fill-in> | <fill-in> |
| Quest objective | | <fill-in> | | | <fill-in> |
| Enemy health | | <fill-in: combat only> | | | <fill-in> |
| Status effects | | <fill-in: while active> | | | <fill-in> |
| Subtitles | <fill-in: while dialogue plays> | | | | Accessibility req |
| Tutorial prompts | | <fill-in: first-time only> | | | <fill-in> |
| Interaction prompt | | <fill-in: in range> | | | <fill-in> |

---

## 3. Layout Zones

```
 ┌───────────────────────────────────────────┐  SAFE ZONE (platform-dependent margin)
 │ [TOP-LEFT]       [TOP-CENTER]  [TOP-RIGHT]│
 │  Health          Quest name    Ammo/cd    │
 │                                           │
 │                                           │
 │              [CENTER]                     │  KEEP MINIMAL
 │              Crosshair / prompt           │
 │                                           │
 │ [BOTTOM-LEFT]   [BOTTOM-CENTER] [BOTTOM-RT]│
 │  Minimap        Subtitles       Toasts    │
 └───────────────────────────────────────────┘
```

| Zone | Position | Primary Elements | Max Simultaneous |
|---|---|---|---|
| Top Left | safe margin | <fill-in> | <fill-in> |
| Top Center | safe margin | <fill-in> | 1 — single-message zone |
| Top Right | safe margin | <fill-in> | <fill-in> |
| Center | screen center ±15% | Crosshair, interaction prompt | 1 active |
| Bottom Left | safe margin | <fill-in> | <fill-in> |
| Bottom Center | safe margin | Subtitles, tutorial prompts | 2 coexist max |
| Bottom Right | safe margin | Notification toasts | 3 stacked |

**Safe zone margins**:

| Platform | All sides |
|---|---|
| PC windowed | 0% |
| PC fullscreen | 3% |
| Steam Deck | 5% |
| Console (TV) | 10% (action-safe) |

---

## 4. Element Specifications

One row per element. Full detail block below table.

| Element | Zone | Always Visible | Trigger | Data Source | Update Freq | Accessibility Alt |
|---|---|---|---|---|---|---|
| Health bar | Top Left | <fill-in: yes> | — | `PlayerStatsSO` | on change | Text label `current/max` |
| Ammo counter | Top Right | <fill-in: contextual> | weapon equipped | weapon SO | on fire | text-only fallback |
| Minimap | Bottom Left | <fill-in> | — | nav service | realtime | compass strip fallback |
| Crosshair | Center | <fill-in: contextual> | ranged weapon | aim service | realtime | enlarge option |
| Subtitles | Bottom Center | — | dialogue plays | dialogue svc | per-line | THIS IS the a11y feature |
| Toasts | Bottom Right | — | event | multiple | on event | screen-reader announce |

### Element Detail — Health Bar

- **Visual** — <fill-in: horizontal fill, left→right, color coded by urgency>
- **Data shown** — current HP as fill %, numerical label `current/max`
- **Update** — lerp 150ms per change; flash on large hits (>25%)
- **Urgency states**:
  - Normal (>50%) — green fill, no pulse
  - Caution (25–50%) — yellow fill, slow pulse 0.25Hz
  - Critical (<25%) — red fill, pulse 1Hz, vignette
- **Player customizable** — opacity, position (any corner)

### Element Detail — <fill-in: next element>

<fill-in: same structure>

---

## 5. HUD States by Gameplay Context

| Context | Shown | Hidden | Modified | Transition |
|---|---|---|---|---|
| Exploration | Minimap, quest obj (faded) | Ammo, crosshair, damage nums | Health to 40% opacity | fade 500ms |
| Combat | Health, ammo, crosshair, enemy bars | Quest obj | Minimap scale down 15% | instant |
| Dialogue | Subtitles, speaker name | All gameplay | — | fade 300ms |
| Cinematic | Subtitles only | Everything | Letterbox | 400ms |
| Menu open | None | All | — | 150ms |
| Death | Death overlay | All gameplay | Desaturate 800ms | 600ms |

---

## 6. Notification System

| Type | Zone | Duration (ms) | Max Simultaneous | Priority | Combat-Aware? |
|---|---|---|---|---|---|
| Item pickup | Bottom Right | 2000 | 3 stacked | Low | queued during combat |
| XP gain | Bottom Right | 1500 | 1 (merges) | Very Low | queued |
| Level up | Center | persistent | 1 | High | interrupts |
| Quest update | Top Center | 4000 | 1 | Medium | no |
| Critical warning | Edge + center | while condition active | 1/type | Critical | never suppressed |

**Rules**:
1. Low-priority notifications queue during combat; flush after exit.
2. Same-type notifications within 500ms merge (`Item Pickup x3`).
3. Critical warnings bypass all queues.

---

## 7. Visual Budget

| Constraint | Limit |
|---|---|
| Max simultaneous elements | 8 |
| HUD area — exploration | ≤12% of screen |
| HUD area — combat | ≤22% |
| Center 40% zone occupancy | ≤5% (crosshair + prompts only) |
| Min contrast ratio | 4.5:1 (WCAG AA) |
| Max background panel opacity | 65% (preserve world) |
| Min element size @ min resolution | 40px icons / 18px text |

---

## 8. Accessibility

### Colorblind

| Element | Risk | Mitigation |
|---|---|---|
| Health bar | Red-green | Icon pulse + vignette redundant |
| Damage numbers | Red/green | `-`/`+` prefix redundant |
| Status effects | Color-tinted icons | Distinct shapes per status; color secondary |

### Text Scaling (75–150%)

All HUD text must reflow cleanly. Damage numbers don't scale (world-space) —
documented limit.

### Motion Sensitivity (Reduced Motion on)

| Animation | Replacement |
|---|---|
| Low-HP pulse | Static fill |
| Damage number float | Instant appear/disappear |
| Toast slide-in | Instant at final position |
| Level-up scale | Static card |

### Subtitles

- **Default**: ON
- **Max chars/line**: 42
- **Max lines visible**: 2
- **Speaker ID**: name + colon prefix (never color alone)
- **Background panel**: 70% opacity black
- **Min font size**: 24px at 1080p reference
- **Persistence**: audio-duration + 300ms

### Player Controls

| Setting | Range | Default |
|---|---|---|
| HUD opacity (global) | 0–100% | 100% |
| HUD text scale | 75–150% | 100% |
| Damage numbers | on/off | on |
| Minimap | on / off / compass | on |
| Reduced motion | on/off | off |
| High contrast | on/off | off |

---

## 9. Acceptance Criteria

- [ ] All elements within platform safe zone on all targets
- [ ] No two elements overlap in any documented context
- [ ] HUD area ≤12% exploration / ≤22% combat
- [ ] All text ≥4.5:1 contrast vs all backgrounds it appears over
- [ ] No element relies on color as sole differentiator
- [ ] Subtitles appear for all voiced lines and stay until audio ends
- [ ] Reduced Motion instants all animated transitions
- [ ] Text scale 150% does not overflow any container
- [ ] HUD repositionable (Health, Minimap, Abilities) per accessibility req
- [ ] Notification queue flushes correctly on level transition

---

## Open Questions

| Q | Owner | Deadline |
|---|---|---|
| <fill-in> | <fill-in> | <fill-in> |

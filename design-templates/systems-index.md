---
description: Registry of every system in the game with cross-refs. Maps system → GDD → SO files → scripts → ADRs. Single source of truth for "what ships."
---

<!-- USAGE
Living document. Updated every time a system is added, removed, or renamed.
This is the canonical list — if a system isn't in this table, it doesn't
exist. Blueprint's /map-systems skill generates the initial dependency graph;
you maintain it manually thereafter.

Cross-refs:
  - design-templates/game-concept.md         (decomposed into systems here)
  - design-templates/game-design-document.md (every system has one)
  - design-templates/architecture-traceability.md (cross-check: system → ADR)
  - Assets/_Project/ScriptableObjects/**     (SO files referenced below)
  - Assets/_Project/Scripts/**               (script files referenced below)
-->

# Systems Index: {{PROJECT_NAME}}

**Status**: <fill-in: Draft | Approved>
**Last Updated**: <fill-in: YYYY-MM-DD>
**Source Concept**: [design/game-concept.md](game-concept.md)

---

## Overview

<fill-in: one paragraph — mechanical scope of this game. What kind of systems
does it need? Reference core loop and pillars. The big picture of what gets
designed + built.>

---

## Systems Registry

One row per system. Columns cross-reference everything that defines or
implements it.

| # | System | Category | Priority | Status | GDD | ScriptableObjects | Scripts | ADRs | Depends On |
|---|---|---|---|---|---|---|---|---|---|
| 1 | <fill-in: e.g., MatchSim> | Core | MVP | <fill-in: Designed/In-Impl/Shipped> | [match-engine.md](../design/match-engine.md) | `MatchConfigSO.cs` | `MatchSim/` | ADR-0003 | Fixed, Seed |
| 2 | <fill-in> | <fill-in> | <fill-in> | <fill-in> | <fill-in> | <fill-in> | <fill-in> | <fill-in> | <fill-in> |

Mark inferred systems (not in original concept) with `(inferred)` in the name.

---

## Categories

| Category | Description |
|---|---|
| **Core** | Foundation — player controller, input, physics, camera, scene mgmt, state machines |
| **Gameplay** | What makes the game fun — combat, AI, abilities, interaction |
| **Progression** | How the player grows — XP, skills, unlocks, achievements |
| **Economy** | Resource cycles — currency, loot, crafting, shops |
| **Persistence** | Save + settings + profile management |
| **UI** | HUD, menus, inventory, dialogue UI, notifications |
| **Audio** | Music, SFX buses, ambient, adaptive audio |
| **Narrative** | Dialogue, quests, cutscenes, lore |
| **Meta** | Telemetry (if any), tutorials, accessibility options |

Remove categories that don't apply. Add custom ones as needed.

---

## Priority Tiers

| Tier | Gate | Design Urgency |
|---|---|---|
| **MVP** | First playable — "is the loop fun?" | Design FIRST |
| **Vertical Slice** | One complete polished area | Design SECOND |
| **Alpha** | All features rough, placeholder content OK | Design THIRD |
| **Full Vision** | Content-complete, polish + edge cases | Design as needed |

---

## Dependency Layers

Design + build top to bottom.

### Foundation Layer (no dependencies)

1. <fill-in: system> — <fill-in: why foundational>

### Core Layer (depends on Foundation)

1. <fill-in: system> — depends on: <fill-in>

### Feature Layer (depends on Core)

1. <fill-in: system> — depends on: <fill-in>

### Presentation Layer (depends on Features)

1. <fill-in: system> — depends on: <fill-in>

### Polish Layer (depends on everything)

1. <fill-in: system> — depends on: <fill-in>

---

## Recommended Design + Build Order

Combines dependency sort and priority tier. Design GDD + write first-pass code
for each before starting the next. Parallel OK within a single layer.

| Order | System | Priority | Layer | Est. Effort |
|---|---|---|---|---|
| 1 | <fill-in> | MVP | Foundation | <fill-in: S/M/L> |
| 2 | <fill-in> | MVP | Foundation | <fill-in> |

Effort: S = 1 session, M = 2-3 sessions, L = 4+ sessions.

---

## Circular Dependencies

Circular deps require either breaking the cycle with an interface or designing
the pair simultaneously. Document each one found.

- <fill-in: None found> OR
- <fill-in: SystemA ↔ SystemB — resolution: extract `IFoo` interface into Core>

---

## High-Risk Systems

Systems that are technically unproven, design-uncertain, or scope-dangerous.
Prototype these early regardless of tier.

| System | Risk Type | Risk | Mitigation |
|---|---|---|---|
| <fill-in> | <fill-in: Technical/Design/Scope> | <fill-in> | <fill-in> |

---

## Progress Tracker

| Metric | Count |
|---|---|
| Total systems identified | <fill-in> |
| GDDs started | <fill-in> |
| GDDs approved | <fill-in> |
| Systems implemented | <fill-in> |
| MVP systems designed | <fill-in>/<fill-in> |

---

## Next Steps

- [ ] Approve enumeration (creative lead / self-review)
- [ ] Write GDD for each MVP-tier system via [game-design-document.md](game-design-document.md)
- [ ] Prototype highest-risk system early
- [ ] First ADR for any system with MEDIUM or HIGH knowledge risk via [architecture-decision-record.md](architecture-decision-record.md)
- [ ] Populate [architecture-traceability.md](architecture-traceability.md) as systems get ADRs

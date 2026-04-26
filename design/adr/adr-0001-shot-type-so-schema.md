---
description: ADR-0001 — ShotTypeSO schema + Addressables grouping. Formalizes the semantic-cinema shot-authoring contract locked in Phase 0.
---

# ADR-0001: ShotTypeSO schema + Addressables grouping

## Status

**Accepted** — 2026-04-24. Tightened on ChainCondition pinning + deterministic-selection mechanism + Addressables-grouping explicit rule per user review.

**Supersession note — 2026-04-26:** ADR-0002 was superseded by ADR-0008 / ADR-0009 during the visual-target pivot. Where this ADR mentions ADR-0002, `ViewerRenderer`, or old 3D timing, read that as the renderer-agnostic `ShotPresentationContract` + active renderer adapter. The `ShotTypeSO` schema remains Accepted.

## Date

2026-04-24

## Last Verified

2026-04-24

## Decision Makers

osagberg (project owner), GPT-5.5 (design partner), Claude (workhorse author).

---

## Summary

Encode each of the 7 semantic-cinema shot types as a Unity `ScriptableObject` asset loaded via Addressables with content-pack-qualified stable IDs. Schema carries framing, modulation strengths, data-driven chain rules, fallback shot, accessibility `reduce_motion_variant`, and overlay-template references — so the "semantic cinema grammar" stays authored in data, not baked into C#.

## Engine Compatibility

| Field | Value |
|---|---|
| Engine | Unity 6 LTS (exact patch pinned at Phase 3 kickoff) |
| Domain | Rendering / Content | <!-- ui-lint:allow term="domain" reason="ADR template canonical field name for engine-compat area" reviewer="osagberg" -->
| Knowledge Risk | LOW — `ScriptableObject` + Addressables are stable well-documented Unity APIs |
| References Consulted | `design/semantic-cinema.md` 2026-04-24 resolution, `design/production-pipeline.md`, `design/player-generation.md` ID-stability rules |
| Post-Cutoff APIs Used | None |
| Verification Required | Addressables-group build + runtime-load smoke under Unity 6 LTS during Phase 3 Week 1 |

## Dependencies

| Field | Value |
|---|---|
| Depends On | None |
| Enables | ADR-0008 (ShotPresentationContract) + ADR-0009 (dots-phase adapter) — viewer bridge / active adapter reads the active `ShotTypeSO`'s framing + modulation; ADR-0004 (MemoryEvent schema) — chain-rule `condition` field includes memory-hit predicates |
| Blocks | Phase 3 Week 2-3 dots-phase viewer prototype (3 of 7 shot types); Phase 5 full 7-shot rollout |

---

## Context

### Problem Statement

`design/semantic-cinema.md` (2026-04-24 resolution) locks 7 shot types with stakes + memory modulation, data-driven chain rules (so `pass-shot-impact → crowd-reaction → aftermath-freeze` does NOT become hardcoded C# glue), a `fallback_shot_category` to prevent frozen-camera failure modes, and an accessibility `reduce_motion_variant`. That resolution specified the *shape*; this ADR specifies the **authoring asset + grouping strategy** so programmers can implement without further clarification.

### Current State

No Unity project exists yet (Phase 3 creates it). No shot authoring format exists. Design doc has the schema draft but not the asset-shape commitment.

### Constraints

- **Content-pack-qualified IDs, no pack-minor in entity IDs** — inherits the ID-stability rule from ADR candidates 6 (IdentityPacket) + the 2026-04-24 Player-generation resolution.
- **Mod-ready from day one** — per `SPEC.md` 2026-04-22 decision "Mod-ready data architecture from day one; editor UX deferred." Shot authoring must survive content-pack replacement/extension.
- **Solo-dev authoring cost** — SO assets with inspector editing win over per-shot prefab hierarchies.
- **Determinism** — shot selection + chain-rule evaluation must be deterministic (consumes the same MatchSim event stream; rendering selections are audit-trailed in replays).
- **Renderer-adapter compatibility** — the Phase-3 dots adapter and any Phase-5/6 3D adapter spike must be able to consume the same authored `ShotTypeSO` assets. The framing fields generalize across dots-camera placement and future 3D camera placement.

### Requirements

- **Functional:** 7 shot types authorable as data, not code; chain rules data-driven; per-shot reduce-motion variant; content-pack-loadable via Addressables; stable IDs across pack versions.
- **Performance:** shot-SO lookup on MatchSim event emission is O(1) via a pre-built `shot_category → ShotTypeSO` dictionary (built at scene load, never rebuilt per frame).
- **Mod-pack-loadability:** content packs can ship additional `ShotTypeSO` assets (post-EA Workshop editor); existing packs' IDs never mutate.

---

## Decision

### Shape

Each of the 7 shot types is a Unity `ScriptableObject` asset stored at `unity-project/Assets/Content/ShotTypes/` in the base content pack. Mod packs ship their own `ShotTypes/` under a content-pack-qualified namespace. Loading flows through Addressables, grouped per content pack. UI Toolkit (UXML/USS) is NOT used for shot composition — UXML is reserved for HUD overlay positioning per `design/semantic-cinema.md`.

### Architecture Sketch

```
┌─────────────────────┐         ┌──────────────────────┐
│ MatchSim event emit │ ──────> │ ShotSelector (C#)    │
└─────────────────────┘         │                      │
                                │  • reads event class │
                                │  • queries memory    │
                                │  • computes stakes   │
                                │  • resolves shot via │
                                │    dictionary lookup │
                                │  • applies chain rule│
                                │  • picks reduce-motion│
                                │    variant if enabled│
                                └─────────┬────────────┘
                                          │ ShotTypeSO
                                          v
                          ┌──────────────────────────┐
                          │ ShotPresentationContract │
                          │ + active adapter         │
                          │ (ADR-0008/0009) consumes │
                          │ framing + modulation +   │
                          │ overlay template set     │
                          └──────────────────────────┘

Addressables label: "shot-type"
Group per content pack: "core", "mod-<name>"
Resolver: ShotTypeCatalog (scene-load singleton)
```

### Key Interfaces

```csharp
// Asset — authored by designers via Unity Inspector.
[CreateAssetMenu(menuName = "FinalWhistle/Shot Type")]
public sealed class ShotTypeSO : ScriptableObject
{
    [Tooltip("Content-pack-qualified stable ID. e.g. 'fwh.core:shot.tactical-wide'. Never mutates once shipped.")]
    public string Id;

    public ShotCategory Category;                // enum: 7 values, see design/semantic-cinema.md

    public FramingParams Framing;                // pitch_tilt_degrees, camera_distance, target_anchor_rule
    public ModulationStrength Modulation;        // stakes [0,1], memory [0,1], crowd [0,1]
    public List<ChainRule> ChainRules;           // data-driven follow-on selection
    public ShotCategory FallbackCategory;        // when no chain rule fires

    public ReduceMotionVariant ReduceMotion;     // accessibility: flags + hold-tick override

    public int DefaultHoldTicks;                 // 60Hz steps
    public int MaxHoldTicks;

    public List<OverlayTemplateReference> OverlayTemplates;  // projected to ShotTypeDefinition for ADR-0008
}

public enum ShotCategory
{
    TacticalWide, DiagonalAttackLane, PlayerIsolation,
    DuelPanel, PassShotImpact, CrowdReaction, AftermathFreeze,
}

[Serializable]
public struct ChainRule
{
    public ShotCategory NextCategory;
    public ChainConditionId ConditionId;  // registry-backed, deterministic; see below
    public int Priority;                  // lower = higher priority; ties broken by ShotTypeSO.Id
    public int MinTicks;                  // 60Hz steps
    public int MaxTicks;
}

// ChainConditionId is a content-pack-qualified stable identifier resolved
// at scene-load against a registry of PURE DETERMINISTIC condition evaluators.
// NO arbitrary scripted predicates in content packs (would break determinism +
// open a sandbox-escape surface for Workshop mods).
// MVP registry: enum-like set of named conditions (e.g. "goal-scored",
// "near-miss", "high-salience-callback-hit", "shot-resolved-terminal-action").
// Additional memory-related condition ids are defined by ADR-0004 (MemoryEvent
// schema); ADR-0004 must only add validator-friendly deterministic predicates.
public readonly struct ChainConditionId { public string Value { get; } }

// Runtime lookup.
public interface IShotTypeCatalog
{
    ShotTypeSO Resolve(ShotCategory category);  // base pack + active mod packs
    IReadOnlyList<ShotTypeSO> AllLoaded();
    bool IsReduceMotionActive();
}
```

### Addressables grouping

**Group by content pack, NOT by shot category.** 7 base shot SOs is not enough volume to justify per-category unload control; per-pack grouping matches mod-pack lifecycle.

- **Group: `shot-types-fwh.core`** — contains the 7 base shot SOs.
- **Group per mod pack: `shot-types-mod-<packname>`** — contains that pack's additional shot SOs (post-EA Workshop).

**Labels applied to every `ShotTypeSO` asset:**
- `shot-type` — required, for catalog-wide load
- `content-pack:<pack-id>` — required, e.g. `content-pack:fwh.core`
- `shot-category:<category>` — optional, enables category-filtered queries if needed later

**Loading:** at scene-load, `ShotTypeCatalog` loads all assets with label `shot-type`, indexes by `Id` + `Category`, wires chain-rule references, and warm-starts the dictionary. No per-frame allocation.

**Unloading:** mod packs unload by `content-pack:` label when the pack is disabled; base pack is resident for the session.

### Deterministic shot selection — contract

"Given the same MatchSim event stream + same loaded pack set + same reduce-motion setting, shot choices must be bit-reproducible (same `Id`s in the same order)." Explicit implementation contract:

1. **Chain-rule evaluation order:** rules within a `ShotTypeSO` are evaluated in **ascending `Priority` order** (lower first). When multiple rules share the same `Priority`, ties are broken by **stable `ShotTypeSO.Id` ordering** (lexicographic on the content-pack-qualified id string).
2. **Active-pack precedence:** the base pack (`fwh.core`) is resolved first; enabled mod packs follow in **deterministic order sorted by `pack_id` lexicographically** (falls back to Addressables load order only if two mod packs share an id, which is itself a validator error).
3. **Forbidden inputs to shot selection:** NO wall-clock time, NO `UnityEngine.Time.*` except in viewer interpolation (never in selection logic), NO `System.Random` / `UnityEngine.Random`, NO unordered collection iteration (`HashSet`/`Dictionary` iteration is not order-stable — always iterate via sorted keys or explicit priority lists).
4. **Variation mechanism (when later needed):** do NOT introduce nondeterminism. Use a **replay-recorded deterministic viewer seed** — emit the seed into the replay artifact so any given replay reproduces identical shot choices across platforms + sessions.
5. **Cross-platform validation:** Phase-3 Week-2 CI matrix (Linux via Tier-A umbrella; Win/Mac via Phase-3 `fw replay` once implemented) must produce identical shot-choice sequence hashes for the canonical replay corpus seeds.

### Implementation Guidelines

- **Asmdef boundary:** `FinalWhistle.Viewer.ShotAuthoring` (asset-side, no runtime deps beyond UnityEngine.ScriptableObject), `FinalWhistle.Viewer.Contracts` (pure-C# `ShotTypeDefinition` projection + `ViewerEvent` contracts per ADR-0008), and `FinalWhistle.Viewer.EventBridge` (pure-C# shot selection + chain-rule evaluation over projected definitions).
- **MatchSim.csproj NEVER depends on `ShotTypeSO`** — the sim emits events; the viewer selects shots. Strict split per `TECH_APPROACH.md §3`.
- **ID validation at bake-time:** the Phase-1 banned-terms lint + Phase-6 content-pack validator must check `ShotTypeSO.Id` follows `fwh.<pack>:shot.<category>-<variant?>` format and does NOT embed pack-minor version. Same validator enforces the per-pack Addressables-group convention above.
- **Reduce-motion wiring:** `IsReduceMotionActive()` reads the accessibility settings SO (authored under `design/accessibility.md` — Phase-2 design doc). When true, every shot's `ReduceMotionVariant` overrides the default framing + disables impact flashes + extends hold-tick ranges.
- **ChainConditionId registry:** Phase-3 ships the MVP registry as a static `Dictionary<string, Func<ShotSelectionContext, bool>>` in `FinalWhistle.Viewer.Runtime`. Content packs reference condition ids as strings in `ShotTypeSO.ChainRules[].ConditionId.Value`; the catalog validates every referenced id resolves at scene-load (failure is a load-time error, never a silent runtime no-op).

---

## Alternatives Considered

### Alternative 1: Hardcoded shot definitions in C#

- **Description** — Enum + static-table shot definitions; no SOs.
- **Pros** — Simplest possible; no asset load; compile-time type safety.
- **Cons** — Violates "mod-ready data architecture from day one" (2026-04-22 SPEC decision). Modding requires code recompilation. Content-pack versioning becomes meaningless for shots. Chain rules become C# switch statements — the exact failure mode `design/semantic-cinema.md` called out ("quietly becomes hardcoded glue").
- **Rejected because** — foundational contradiction with the mod-ready pillar and the data-driven chain-rule requirement.

### Alternative 2: UXML per shot

- **Description** — Author each shot as a UXML layout with named camera anchor elements.
- **Pros** — Leverages UI Toolkit tooling; hot-reload.
- **Cons** — UXML is structured for UI tree composition, not camera framing. Fields like `pitch_tilt_degrees` and `camera_distance` would live in USS custom properties, which is abusive. No native support for chain rules. UXML overlay usage for the HUD is correct — but camera composition is a different abstraction.
- **Rejected because** — tool misfit. UXML stays in its lane (HUD overlay) per `design/semantic-cinema.md` 2026-04-24 resolution §Q3.

### Alternative 3: Per-scene prefab hierarchies

- **Description** — Each shot is a prefab with camera + anchor + overlay as child GameObjects.
- **Pros** — Visually editable in Scene view.
- **Cons** — Doesn't compose with Addressables groups cleanly; scales poorly to mod packs (prefab references break); chain rules still need non-prefab authoring.
- **Rejected because** — doesn't serve the content-pack + modding requirement.

---

## Consequences

### Positive

- Data-driven shot grammar survives the full lifecycle from Phase 3 prototype through post-EA Workshop editor.
- Chain rules in data eliminate the "hardcoded glue" failure mode `design/semantic-cinema.md` explicitly warned against.
- Accessibility variant is authored, not patched — reduce-motion discipline baked in from the first rendered frame.
- MatchSim stays Unity-free (no `ShotTypeSO` reference in `MatchSim.csproj`) — preserves the deterministic-sim architecture per ADR-0003 candidate (Production pipeline) + `TECH_APPROACH.md §3`.

### Negative (Accepted Tradeoffs)

- Adds a content-pack governance surface to the Phase-6 content validator (new ID-format check).
- Requires Addressables setup in Phase 3 before any shot renders — can't skip to hardcoded and port later (would violate ID stability).
- Solo-dev authoring cost: 7 shot SOs × 7 fields each = meaningful inspector work. One-time; pays for itself across 30+ hours of playtest iteration.

### Neutral

- Shot-selection C# code (`ShotSelector`) remains trivial regardless of schema richness — it's a dictionary lookup + chain-rule evaluator.
- Future 3D shot extension (Phase-5/6 spike conditional) reuses the same SO shape with 3D framing fields, not a schema rewrite.

---

## Performance Implications

| Metric | Before | Expected After | Budget |
|---|---|---|---|
| CPU frame time (shot selection per event) | n/a (no impl) | O(1) dict lookup + chain-rule eval | <0.1ms per event |
| Memory | n/a | ~7 SOs × <1KB each base pack; mod packs scale linearly | <32KB for 7 base shots |
| Build size | n/a | ~7KB per pack for the SO assets | negligible vs scene assets |

Shot-catalog lookup is never a frame-rate concern — it fires O(events per match) ≈ hundreds per match, each <0.1ms. Chain-rule min/max ticks are integer comparisons.

---

## GDD Requirements Addressed

| GDD | System | Requirement | How This ADR Satisfies It |
|---|---|---|---|
| `design/semantic-cinema.md` | Renderer-agnostic viewer grammar | 7 shot types with stakes + memory modulation | ShotTypeSO schema carries `Category` + `ModulationStrength` + `FramingParams` |
| `design/semantic-cinema.md` | Shot chaining | Data-driven, not hardcoded | `ChainRule[]` field + `IShotTypeCatalog.Resolve` |
| `design/semantic-cinema.md` | Accessibility | Reduce-motion variant baked in | `ReduceMotionVariant` per shot |
| `design/production-pipeline.md` | Content-pack validator | ID stability, no pack-minor in IDs | Cross-ref to Player-generation 2026-04-24 ID rules |
| `SPEC.md` 2026-04-22 | Mod-ready architecture | Data authoring, not code | Addressables grouping per content pack |

---

## Migration Plan

Not applicable — no prior implementation to migrate from. Greenfield.

**Rollback:** if the ScriptableObject approach proves wrong in Phase 3 prototyping (unlikely given the alternatives' worse tradeoffs), supersede via a new ADR citing this one. No deployed content exists until Phase 4, so rollback cost is limited to Phase-3 authoring time.

---

## Validation Criteria

- [ ] Phase 3 Week 2: 3 `ShotTypeSO` assets authored (`tactical-wide`, `diagonal-attack-lane`, `pass-shot-impact`) and loaded via Addressables at runtime.
- [ ] Phase 3 Week 3: chain rule fires end-to-end on goal event: `pass-shot-impact` → `crowd-reaction` → `aftermath-freeze` with durations pulled from SO data (no hardcoded chain).
- [ ] Phase 3 Week 3: reduce-motion scene-load path swaps shots to their `ReduceMotionVariant`; changing the setting takes effect on viewer scene reload per ADR-0008 / accessibility discipline.
- [ ] Phase 3 Week 4 (Month-3 gate): observer cold-watch of the 3-shot slice produces no "camera froze" complaints (fallback category working).
- [ ] Phase 6: content-pack validator confirms every `ShotTypeSO.Id` matches the stable-ID regex and unique-Id constraint across loaded packs.
- [ ] Shot selection deterministic across 10K-match balance-harness sweeps — same event stream produces identical shot-choice sequence.

---

## Related

- Supersedes: none (first ADR).
- Depends on: `design/semantic-cinema.md` 2026-04-24 resolution (schema source).
- Enables: ADR-0008 (ShotPresentationContract) + ADR-0009 (dots-phase render adapter — consumes `ShotTypeSO`); ADR-0004 (MemoryEvent schema — chain-rule conditions reference memory hits).
- Cross-refs: `design/production-pipeline.md` (content-pack validation), `design/player-generation.md` (ID-stability rule pattern).
- Code (once implemented): `unity-project/Assets/_Project/Viewer/ShotAuthoring/` + `unity-project/Assets/_Project/Viewer/Runtime/ShotTypeCatalog.cs` (paths tentative, finalize at Phase 3 bootstrap).

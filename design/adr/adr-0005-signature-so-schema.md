---
description: ADR-0005 — SignatureSO schema, scope enum, dependency metadata, field-level capped stacking, display/enum-id separation, deterministic evaluation. Formalizes the signature-system resolution as the Pillar-2 authoring contract.
---

# ADR-0005: SignatureSO schema — scope, dependencies, stacking, display/enum separation

## Status

**Proposed** — pending user review before Accepted.

## Date

2026-04-24

## Last Verified

2026-04-24

## Decision Makers

osagberg (project owner), GPT-5.5 (design partner on 2026-04-24 signature-system resolution), Claude (workhorse).

---

## Summary

Lock the Pillar-2 signature authoring contract: `SignatureSO` is a Unity ScriptableObject with content-pack-qualified stable ID, a `Scope` enum (`player` / `defensive_line` / `press_unit` / `set_piece_context`), dependency metadata that gates scheduling + validation (never runtime semantics), field-level capped stacking with deterministic evaluation ordering, and strict separation between internal enum IDs (lint-stable) and player-facing `display_name` / `ui_description` (banned-term lint targets). Event emission uses `SignatureAwakened` / `SignatureExecuted` from `FinalWhistle.Memory.Contracts` (ADR-0004) — not duplicated strings. Latent affinity is explicitly NOT stored here; it lives in `IdentityPacket` per ADR-0006.

## Engine Compatibility

| Field | Value |
|---|---|
| Engine | Unity 6 LTS (exact patch pinned at Phase 3 kickoff) |
| Domain | Gameplay / Content | <!-- ui-lint:allow term="domain" reason="ADR template canonical field name for engine-compat area" reviewer="osagberg" -->
| Knowledge Risk | LOW — `ScriptableObject` + Addressables + enum-backed IDs are stable Unity + C# patterns, already exercised by ADR-0001 (ShotTypeSO) |
| References Consulted | `design/signatures.md` 2026-04-24 resolution, ADR-0001 (ShotTypeSO — Addressables-grouping-by-pack + registry-backed ID pattern reused), ADR-0004 (MemoryEvent — event-class names), `design/ui-vocabulary.md` Category-A.2/A.4 (banned-term lint targets) |
| Post-Cutoff APIs Used | None |
| Verification Required | Phase-3 Week-2 end-to-end: author 3 signature SOs (#20 / #22 / #13 per Month-3 slice); verify they emit `SignatureAwakened` / `SignatureExecuted` events through `FinalWhistle.Memory.Contracts` and trigger sim bias correctly |

## Dependencies

| Field | Value |
|---|---|
| Depends On | ADR-0004 (MemoryEvent — event class names consumed as constants; CallbackTag registry pattern referenced), ADR-0001 (Addressables-grouping-per-pack pattern reused) |
| Enables | ADR-0006 (IdentityPacket — hosts `signature_candidates` referencing `SignatureSO.Id`; affinity distribution authored there, not here), Phase-3 3-signatures-end-to-end authoring |
| Blocks | Month-3 slice signatures (#20 Low cutback from byline, #22 Blind-side near-post run, #13 First-time diagonal switch); Phase-5 12-signature playable; Phase-6 all-24-signatures |

---

## Context

### Problem Statement

`design/signatures.md` 2026-04-24 locked the 24-signature catalog with dependency metadata, scope enum, tier-weighted affinity distribution, and field-level capped stacking. It named the shape but deferred the Unity-authoring contract. This ADR locks that contract so Phase-3 can author the first 3 signature SOs without interpretation questions, and so ADR-0006 can reference a specific `SignatureSO.Id` shape when IdentityPacket's affinity-rolling lands.

### Current State

No Unity project exists. No signature SOs authored. `design/signatures.md` has the full 24-signature catalog with per-signature dependency flags + scope markers + trigger prose; this ADR formalizes the authoring asset.

### Constraints

- **Content-pack-qualified stable IDs; no pack-minor in IDs** — inherits ADR-0006 ID-stability rule + `design/player-generation.md` 2026-04-24 §Q5 resolution.
- **Event emission via Memory.Contracts, not duplicate strings** — `SignatureSO` references `EventClass.SignatureAwakened` and `EventClass.SignatureExecuted` as enum constants, NEVER as inline string literals. Rename at the Memory.Contracts side = schema bump there; `SignatureSO` recompiles against the new enum and keeps working.
<!-- ui-lint:ignore-start reason="technical prose describing the signature-awakening mechanic" -->
- **Latent affinity never stored in `SignatureSO`** — affinity (who can awaken this) is a per-player property on `IdentityPacket.signature_candidates[]`. `SignatureSO` is the *signature*, not the *carrier*.
<!-- ui-lint:ignore-end -->
- **Banned-term lint polices player-facing copy** — `design/ui-vocabulary.md` Category-A lint targets `display_name` + `ui_description` + `presentation_recipe.overlay_text_bank[]`. Internal enum IDs (`Id`, scope enum values, field names) are lint-exempt. This is why the two surfaces are schema-separated, not just stylistically different.
- **Determinism** — same set of active signatures + same MatchSim state = same stacked sim-bias deltas, bit-identical across platforms. Stacking evaluation is ordered; no dictionary iteration without a stable sort key.
- **Cross-platform Q32.32 for deltas** — `sim_bias` delta values are Q32.32 per `design/match-engine.md`; stacking math is fixed-point.

### Requirements

- **Functional:** `SignatureSO` authorable via Unity inspector; Addressables-grouped per content pack; references `FinalWhistle.Memory.Contracts` event-class constants; carries non-behavioral dependency metadata; field-level capped stacking with deterministic evaluation; display/ui separate from enum IDs.
- **Performance:** sim-bias lookup per active signature is O(1) dictionary (built at scene-load); stacking evaluation per frame is O(active signatures × affected fields) where both are small constants.
- **Mod-pack-loadability:** content packs can ship additional `SignatureSO` assets (post-EA Workshop); existing packs' IDs never mutate.

---

## Decision

### Shape

Each signature is a Unity `ScriptableObject` asset at `unity-project/Assets/Content/Signatures/` in the base content pack. Mod packs ship their own `Signatures/` namespace. Addressables grouping matches ADR-0001's per-content-pack pattern. UI Toolkit (UXML/USS) is NOT used for signature authoring — it's reserved for HUD overlay per ADR-0001/ADR-0002.

### Key Interfaces

```csharp
using FinalWhistle.Memory.Contracts;   // EventClass.SignatureAwakened / .SignatureExecuted

[CreateAssetMenu(menuName = "FinalWhistle/Signature")]
public sealed class SignatureSO : ScriptableObject
{
    // --- Internal enum / registry IDs — lint-EXEMPT ---
    [Tooltip("Content-pack-qualified stable ID, e.g. 'fwh.core:signature.low-cutback-from-byline'. Never mutates.")]
    public string Id;

    public RoleFamily RoleFamily;
    public SignatureScope Scope;                      // see enum below

    // --- Player-facing copy — lint-TARGET (banned-term Category A + B per design/ui-vocabulary.md) ---
    [Tooltip("Football-copy only, e.g. 'Low cutback from the byline'. Lint-enforced.")]
    public string DisplayName;
    [Tooltip("Short football-native description. Lint-enforced.")]
    public string UiDescription;

    // --- Non-behavioral metadata — gates scheduling + validation, not runtime ---
    public SignatureDependencies Dependencies;        // see below

    // --- Readiness threshold — per-signature override of the 0.85 default ---
    public Fixed ReadinessThreshold;                  // Q32.32 [0,1]; 0 means "use project default"

    // --- Trigger conditions — AND'd at evaluation time ---
    public ImmutableArray<TriggerCondition> TriggerConditions;

    // --- Sim biases with per-field capped stacking ---
    public ImmutableArray<SimBiasField> SimBias;

    // --- Execution modifier — how ball physics bends during signature execution ---
    public ExecutionModifier ExecutionModifier;

    // --- Presentation recipe — shot-type preference + overlay text bank ---
    public PresentationRecipe PresentationRecipe;

    // --- Counterplay — opponent tactical responses ---
    public ImmutableArray<CounterplayOption> Counterplay;

    // --- Event-class references via Memory.Contracts — NO STRING DUPLICATION ---
    // These are compile-time references; if Memory.Contracts renames the enum, this
    // file fails to compile and the rename is caught at build time, not runtime.
    public const EventClass EmitsOnAwaken = EventClass.SignatureAwakened;
    public const EventClass EmitsOnExecute = EventClass.SignatureExecuted;
}

public enum SignatureScope
{
    Player,              // default — effect applies to the signature-holder only
    DefensiveLine,       // e.g. #6 "Calls the line"; effect applies to the defensive unit
    PressUnit,           // e.g. #15 "Press trigger"; effect applies to the pressing unit
    SetPieceContext      // e.g. #3 "Reads the set piece"; effect gated by set-piece context
}

public readonly struct SignatureDependencies
{
    // Non-behavioral — these gate scheduling (when the signature can ship) and
    // validation (lint catches dependency violations), NOT runtime semantics.
    // A signature with an unsatisfied dependency simply doesn't load; the sim
    // does not branch on dependency state at runtime.
    public ImmutableArray<SystemDependency> Requires;
}

public readonly struct SystemDependency
{
    public string SystemName;            // e.g. "set_pieces", "fouls_and_cards", "defensive_shape_coherence"
    public ushort MinPhase;              // earliest phase this dependency is satisfied
}

[Serializable]
public struct SimBiasField
{
    public SimBiasFieldId FieldId;       // enum — registry-backed, one per named MatchSim bias field
    public Fixed Delta;                  // Q32.32 — applied to field total
    public StackingMode Mode;            // Additive | AdditiveWithDiminishingReturns
    public Fixed MinDelta;               // Q32.32 — hard lower cap after all stacking
    public Fixed MaxDelta;               // Q32.32 — hard upper cap after all stacking
    public DiminishingCurve? DiminishingCurve;  // optional; required when Mode == AdditiveWithDiminishingReturns
}

public enum StackingMode
{
    Additive,
    AdditiveWithDiminishingReturns,
}

public readonly struct DiminishingCurve
{
    public Fixed PerAdditionalStack;     // Q32.32 [0,1]; multiplier applied to each additional stack's contribution
}
```

### Deterministic stacking evaluation (locked)

When multiple signatures active on the same player target the same `SimBiasField`:

1. **Collect all contributing `SimBiasField` entries** (across active signatures on this player).
2. **Sort by stable `SignatureSO.Id` ascending.** (Follows the ADR-0001 deterministic-selection Id-tiebreak pattern.)
3. **For `StackingMode.Additive`:** running total += delta for each entry.
4. **For `StackingMode.AdditiveWithDiminishingReturns`:** running total += delta × `DiminishingCurve.PerAdditionalStack ^ stack_index` — where `stack_index` starts at 0 for the first contribution and increments per entry.
5. **Clamp final total to `[MinDelta, MaxDelta]`** — the cap wins no matter how many stacks accrue. This is the Pillar-2 no-god-tier guarantee.
6. **Balance-harness CI sweep** (Phase 6) exercises every realistic (active-signature-set × affected-field) combination and flags any that breach caps without clamping OR produce dominant-strategy outliers.

No dictionary iteration without a stable sort key. No wall-clock, no `Random.*`, no `Time.*` — same determinism rules as ADR-0001 / ADR-0004.

### Addressables grouping

Identical pattern to ADR-0001:

- **Group: `signatures-fwh.core`** — 24 base signature SOs.
- **Group per mod pack: `signatures-mod-<packname>`** — Workshop additions post-EA.
- **Labels per asset:** `signature` (required), `content-pack:<pack-id>` (required), `role-family:<role>` (optional for filtered queries).

### Event emission (no string duplication)

`SignatureSO` holds `EmitsOnAwaken = EventClass.SignatureAwakened` and `EmitsOnExecute = EventClass.SignatureExecuted` as compile-time `const` references to `FinalWhistle.Memory.Contracts.EventClass`. At runtime, awakening / execution paths call into `ILedgerAppender.Emit` with `EmitsOnAwaken` / `EmitsOnExecute` directly — never with inline strings. Rename at the Memory.Contracts side = schema bump there + recompile catches the reference here at build time (never runtime drift).

<!-- ui-lint:ignore-start reason="section describing where affinity / awakening data lives; technical mechanic prose" -->
### Latent affinity — lives in IdentityPacket, NOT here

Per the 2026-04-24 player-generation resolution: affinity (who can awaken a signature) is a per-player property stored in `IdentityPacket.signature_candidates[]` with `(signature_id, affinity_weight)`. `SignatureSO` is the definition of *what the signature is and how it works*; `IdentityPacket` is *who can awaken it*. This ADR does not commit any affinity storage into `SignatureSO`.

Cross-ref: `design/signatures.md` §Affinity-distribution is authoritative-reference-to `design/player-generation.md` §Affinity-count-distribution. ADR-0006 (upcoming) commits the `IdentityPacket.signature_candidates[]` shape.
<!-- ui-lint:ignore-end -->

### Display copy separation — lint targets only here

- `DisplayName` + `UiDescription` + `PresentationRecipe.OverlayTextBank[]` are the **ONLY** fields player-facing banned-term lint (`scripts/lint-banned-terms.py` Categories A.1-A.5 + B) scans inside SignatureSO assets.
- Internal IDs (`Id`, `RoleFamily`, `Scope` enum values, `SimBiasFieldId` values, `SystemDependency.SystemName`) are **exempt** from Category-A/B lint. They're developer-surface, not player-surface.
- Phase-6 content-pack validator enforces this split: Category-A lint failure on an internal-ID field means the validator rule is wrong, not the asset. Author-facing error.

### Architecture Sketch

```
┌──────────────────────┐         ┌───────────────────────────┐
│  IdentityPacket      │ ──affinity_weight──► SignatureSO.Id │
│  (per-player, ADR-06)│         └─────────┬─────────────────┘
└──────────────────────┘                   │ SignatureSO asset
                                           v
                               ┌──────────────────────┐
                               │ SignatureCatalog     │  (scene-load singleton)
                               │  - Id → SignatureSO  │  indexes by Id,
                               │  - RoleFamily filter │  Addressables-loaded
                               │  - Scope filter      │
                               └─────────┬────────────┘
                                         │ ResolveActiveSignatures(player)
                                         v
┌────────────────────┐         ┌────────────────────────┐
│ MatchSim event     │ ──────> │ SignatureRuntime       │ ─► stacked SimBias deltas
│ (via Memory.       │         │  - trigger evaluation  │    (clamped to Min/Max)
│   Contracts DTO)   │         │  - sim-bias stacking   │
└────────────────────┘         │  - EmitsOnAwaken/      │ ─► ILedgerAppender.Emit
                               │    Execute event emit  │    (EventClass from Contracts)
                               └────────────────────────┘
```

### Implementation Guidelines

- **Asmdef boundary:** `FinalWhistle.Signatures.Authoring` (SO + asset-time inspector) + `FinalWhistle.Signatures.Runtime` (catalog + stacking + event emission). Runtime depends on `FinalWhistle.Memory.Contracts`; does NOT depend on `FinalWhistle.Memory` (persistence) directly.
- **`MatchSim.csproj` depends on neither** — signatures live in the Unity side; MatchSim receives sim-bias deltas via a thin DTO boundary (`FinalWhistle.Signatures.SimBiasSnapshot`) that Unity pushes each tick. Preserves the pure-C# MatchSim per `TECH_APPROACH.md §3`.
- **`SimBiasFieldId` registry** is authored as a static enum + registry in `FinalWhistle.Signatures.Runtime`; content packs reference field IDs by name; the catalog validates every referenced ID resolves at scene-load.
- **Display-copy lint integration:** the Phase-6 content-pack validator extracts `DisplayName` + `UiDescription` + `OverlayTextBank[]` into a flat string array and passes it to `scripts/lint-banned-terms.py` as the lint target. Internal IDs are NOT extracted.
- **Readiness-threshold default** is a project-level constant (not a magic number in every SO): `SignatureAuthoring.DefaultReadinessThreshold = 0.85 (Q32.32)`. `SignatureSO.ReadinessThreshold = 0` is the sentinel for "use default" — the catalog resolves at load.

---

## Alternatives Considered

### Alternative 1: Inline event-class strings instead of Memory.Contracts references

- **Description** — Store `"SignatureAwakened"` / `"SignatureExecuted"` as string literals in each SignatureSO; look up the enum at emission.
- **Pros** — No cross-project dependency from SignatureSO authoring to Memory.Contracts.
- **Cons** — Silent drift when the event-class enum renames. A typo or stale string becomes a runtime "event not emitted" bug that surfaces only in specific replay edge cases. Defeats the cross-doc exact-match discipline from ADR-0004.
- **Rejected because** — exactly the failure mode ADR-0004's cross-doc constraint closes. Compile-time enum references make rename-drift a build error, which is the right failure mode.

<!-- ui-lint:ignore-start reason="alternative-rejection prose describing the awakening-data data model" -->
### Alternative 2: Latent affinity stored in `SignatureSO`

- **Description** — Each `SignatureSO` lists who can awaken it (e.g., an affinity-weight-by-role-family table inside the asset).
- **Pros** — One asset per signature carries all signature-related data.
- **Cons** — Inverts the data model. Affinity is a *player* property (some players can awaken this; most can't). Storing affinity on the signature forces per-player reverse-indexing at load time and tightly couples player generation to signature asset shape. Violates single-responsibility — `SignatureSO` is *what the signature is*; `IdentityPacket` is *who the player is*.
- **Rejected because** — architectural inversion. Also contradicts the 2026-04-24 player-generation resolution §Q2 which placed affinity distribution in `IdentityPacket`.
<!-- ui-lint:ignore-end -->

### Alternative 3: Runtime-branching dependency metadata

- **Description** — Let `SignatureDependencies` gate runtime behavior (e.g., `if (dependency unsatisfied) { skip trigger evaluation }`).
- **Pros** — Single code path for "signature exists but dependency not yet met."
- **Cons** — Introduces hidden runtime branches that depend on non-deterministic project-state (which Phase are we in?). Makes balance-harness sweeps harder (are we testing the Phase-4 behavior or Phase-6 behavior?). Mixes content-authoring concerns with runtime semantics.
- **Rejected because** — dependency metadata should gate **what ships**, not **how it behaves at runtime**. If a signature depends on Phase-4 fouls-and-cards and Phase 4 hasn't shipped, the signature simply isn't in the loaded content pack — the validator excludes it. Runtime code never sees it.

### Alternative 4: Dictionary/HashMap iteration for stacking

- **Description** — Iterate active signatures via `Dictionary<string, SignatureSO>.Values` when stacking deltas.
- **Pros** — Simple; standard collection pattern.
- **Cons** — `Dictionary.Values` iteration order is not stable across .NET implementations or even across single-process reruns in some edge cases. Breaks replay-hash determinism because the stacking order affects diminishing-returns math.
- **Rejected because** — same reasoning as ADR-0001's forbidden-inputs section. Stable sort by `Id` ascending is the deterministic path.

### Alternative 5: Unified `DisplayCopy` field (no separation from enum IDs)

- **Description** — One string field per signature that serves both the banned-term lint AND the internal ID.
- **Pros** — Simpler asset shape.
<!-- ui-lint:ignore-start reason="prose naming example banned tokens that would trip the unified-field design" -->
- **Cons** — Either the field is lint-scanned and internal IDs become illegal when they collide with banned vocabulary (e.g., if any future content-pack uses "Plateau" or "Forge" in a feature name, the internal ID trips the lint), OR the field is lint-exempt and player-facing copy escapes the vocabulary lint entirely.
<!-- ui-lint:ignore-end -->
- **Rejected because** — the separation is the point. Developer-surface IDs need stability and expressiveness; player-surface copy needs football-native vocabulary enforcement. Two fields is cheap.

---

## Consequences

### Positive

- Compile-time enum references from `SignatureSO` → `Memory.Contracts` make cross-doc rename drift a build error, not a replay-corpus regression.
- Scope enum makes team-level effects (defensive line, press unit) structurally authorable without re-opening individual-vs-team modeling questions.
- Non-behavioral dependency metadata keeps runtime semantics clean — signatures either load or don't; no in-sim dependency branches.
- Field-level capped stacking with stable Id-tiebreak ordering preserves the replay determinism contract.
- Display/ID separation lets the banned-term lint be strict on player-facing text without constraining developer-surface identifiers.
<!-- ui-lint:ignore-start reason="Consequences summary prose; describes where the awakening logic lives" -->
- Latent affinity correctly lives in `IdentityPacket` — player generation owns who-can-awaken; signatures own what-the-signature-does.
<!-- ui-lint:ignore-end -->

### Negative (Accepted Tradeoffs)

- Two fields (`Id` + `DisplayName`) instead of one means per-signature authoring always fills two strings. Minor per-signature cost; pays back through lint discipline.
- Per-signature asset authoring cost (24 × ~8 fields) is non-trivial; mitigated by Unity inspector tooling and the fact that the catalog is one-shot at Phase 5-6.
- `SimBiasFieldId` registry is a new coupling surface — any new MatchSim bias field requires a registry entry. Validator catches unregistered references, but it's one more thing to remember.

### Neutral

- Latent affinity landing in ADR-0006 means cross-ADR readers must understand the split. Cross-refs are explicit; no hidden coupling.
- `FinalWhistle.Signatures.SimBiasSnapshot` DTO boundary is one more interface between Unity and MatchSim; pays for itself via pure-C# sim discipline.

---

## Performance Implications

| Metric | Target | Notes |
|---|---|---|
| SignatureCatalog lookup | O(1) via dictionary | Built at scene-load; 24 base SOs + mod packs |
| Stacking evaluation per player per tick | O(active signatures × affected fields) | Small constants; <0.05ms per player |
| Event emission per signature trigger | O(1) — direct `ILedgerAppender.Emit` call | Enum constants, no string lookup |
| Memory footprint per SignatureSO | ~2-4KB | 24 × ~3KB = ~72KB base pack |

Phase-6 balance-harness sweeps exercise realistic stacking combinations; the perf budget is generous at signature counts.

---

## GDD Requirements Addressed

| GDD | System | Requirement | How This ADR Satisfies It |
|---|---|---|---|
| `design/signatures.md` 2026-04-24 | 24-signature catalog with dependency metadata | Scope enum + SystemDependency + content-pack-qualified IDs | SignatureSO schema + SignatureScope enum + Dependencies |
| `design/signatures.md` §stacking-policy | Field-level capped stacking, not softmax | `SimBiasField.MinDelta/MaxDelta` + StackingMode enum + Id-tiebreak ordering | Stacking evaluation section above |
| `design/signatures.md` §display-names | Football-copy only; lint-enforced | DisplayName + UiDescription + OverlayTextBank separated from internal IDs | Display/ID separation |
| `design/ui-vocabulary.md` Category-A/B | Banned-term lint on player-facing copy | Lint extractor targets only DisplayName + UiDescription + OverlayTextBank | Implementation Guidelines |
| ADR-0004 | Event emission via Memory.Contracts | `const EventClass` references, no inline strings | Key Interfaces section |
| `design/player-generation.md` 2026-04-24 §Q2 | Latent affinity in IdentityPacket, not signatures | Explicit non-commitment in this ADR | "Latent affinity" section + ADR-0006 handoff |

---

## Migration Plan

Not applicable — greenfield. First signatures author at Phase-3 Week 2.

**Rollback:** if any ADR-0005 commitment proves wrong in Phase-3 prototype, supersede via a new ADR. Schema stability pre-EA is high because no saved data references `SignatureSO.Id`s yet.

---

## Validation Criteria

- [ ] Phase 3 Week 2: 3 SignatureSOs authored (#20 / #22 / #13 per Month-3 slice) and Addressable-loaded.
- [ ] Phase 3 Week 3: event emission path fires `SignatureAwakened` / `SignatureExecuted` via `ILedgerAppender.Emit` — verified by ledger inspection, no string literals in flight.
- [ ] Phase 3 Week 3: stacking evaluation produces identical results across Win/Mac/Linux Tier-A CI matrix for a scenario with 2 signatures active on the same player.
- [ ] Phase 3 Week 4 (Month-3 gate): signatures visibly alter MatchSim play (cold-observer confirmable).
- [ ] Phase 6: Phase-6 content-pack validator catches a red-team test where an unregistered `SimBiasFieldId` is referenced.
- [ ] Phase 6: Phase-6 content-pack validator catches a red-team test where a banned Category-A term is injected into `DisplayName` OR `UiDescription` (and is NOT flagged for the signature's internal `Id`).
- [ ] Phase 6: Phase-6 content-pack validator catches cross-doc event-class name drift (e.g., renaming `SignatureExecuted` → `SignatureUsed` in Memory.Contracts triggers a compile error in SignatureSO's const reference, caught at build time).
- [ ] Balance harness (Phase 6): every `SimBiasField` field-ID sweeps through realistic stacking combinations; no breach of Min/Max caps; no dominant-strategy outliers.

---

## Related

- Depends on: ADR-0004 (MemoryEvent — event-class constants), ADR-0001 (Addressables-grouping-per-pack pattern).
- Enables: ADR-0006 (IdentityPacket — `signature_candidates[]` references `SignatureSO.Id`; affinity distribution authoritative there).
- Cross-refs: `design/signatures.md` 2026-04-24 resolution (source), `design/ui-vocabulary.md` Categories A + B (lint targets), `design/player-generation.md` §Q2 (latent affinity lives there).
- Code (once implemented): `unity-project/Assets/_Project/Signatures/` (paths tentative, finalize Phase-3 bootstrap).

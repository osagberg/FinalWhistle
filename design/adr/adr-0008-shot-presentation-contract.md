---
description: ADR-0008 — ShotPresentationContract / ViewerEvent. Renderer-agnostic presentation contract derived from MatchSim events for Viewer adapters. Same shot identity + same sim events drive any renderer adapter (dots / 3D / future variants). Supersedes ADR-0002.
---

# ADR-0008: ShotPresentationContract / ViewerEvent — renderer-agnostic viewer interface

## Status

**Accepted** — 2026-04-27 after GPT-5.5 review pass (Concerns verdict; 4 P1 + 4 P2 findings applied) + Codex review pass (stale-reference cleanup; sign-off "no remaining blocking findings"). Originally Proposed 2026-04-26 per visual-target supersession decisions-log entry: ADR-0008 introduces the renderer-agnostic contract layer required to support multiple viewer adapters (dots-prototype Phase-3-onward; cel-shaded 3D shipping candidate gated on Phase-5/6 spike). Append-only from this point forward — supersession only via a new ADR.

## Date

2026-04-26

## Last Verified

2026-04-27

## Decision Makers

osagberg (project owner), GPT-5.5 (design partner who flagged this requirement as P1), Claude (workhorse author).

---

## Summary

Decouple MatchSim from any specific renderer by introducing a pure-C# `ShotPresentationContract` data layer + `ViewerEvent` stream. MatchSim emits canonical sim events; a deterministic viewer bridge derives `ViewerEvent`s that reference shot identities (authored as `ShotTypeSO` per ADR-0001, projected into pure-C# `ShotTypeDefinition`s before bridge use) + carry adapter-agnostic shot-modulation parameters (stakes, memory-hits, participants, deterministic seed). Renderer adapters (dots per ADR-0009; 3D per future ADR-0010 conditional on spike) consume `ViewerEvent`s and produce frames per their adapter's rendering style. The contract is what the viewer bridge guarantees; the adapter is what the player sees.

## Engine Compatibility

| Field | Value |
|---|---|
| Engine | Unity 6, currently pinned to tech-stream `6000.4.4f1` (LTS migration re-evaluated at Phase 7 per SPEC 2026-04-28 decisions log); contract itself is pure-C# inside `Viewer.Contracts` (Unity-free) so engine version is irrelevant to contract correctness |
| Domain | Architecture / Viewer interface | <!-- ui-lint:allow term="domain" reason="ADR template canonical field name for engine-compat area" reviewer="osagberg" -->
| Knowledge Risk | LOW — pure-C# data records + simple event stream; no engine APIs in the contract layer |
| References Consulted | 2026-04-26 visual-target supersession decisions-log entry, ADR-0001 ShotTypeSO, ADR-0002 (Superseded), `design/semantic-cinema.md`, `design/3d-pipeline.md` placeholder, GPT-5.5 review P1 finding |
| Post-Cutoff APIs Used | None |
| Verification Required | Phase-3 Week-2 dots adapter (ADR-0009) consumes the contract end-to-end. Phase-5 spike validates 3D adapter consumes the same contract without sim-side changes. |

## Dependencies

| Field | Value |
|---|---|
| Depends On | ADR-0001 (`ShotTypeSO` authoring asset projected to pure `ShotTypeDefinition` that contract events reference); ADR-0004 (`MemoryEvent` source for memory-hit modulation); ADR-0005 (`SignatureSO` source for signature-trigger viewer events) |
| Enables | ADR-0009 (dots-phase render adapter — first adapter consuming this contract); ADR-0010 (3D cel-shaded render adapter — conditional, Phase-5/6+); future renderer adapters without re-implementing sim emission |
| Blocks | Phase-3 Week-2 dots viewer authoring (until ADR-0009 lands too); Phase-5 production-spike (3D adapter consumes the same contract) |
| Supersedes | ADR-0002 (Viewer rendering pipeline — original stylized-2D-illustrated pipeline is moot under renderer-agnostic posture; ADR-0002 preserved for history per append-only ADR discipline; ADR-0002's specific URP-pass pipeline becomes one possible adapter implementation, not a system-level commitment) |

---

## Context

### Problem statement

The 2026-04-26 visual-target supersession says we ship two renderer adapters (potentially: dots through Phase-3-onward + 3D as candidate shipping visual). Both consume bridged `ViewerEvent`s derived from MatchSim's structured event stream and `ShotTypeDefinition` identities. Without a contract layer between sim and renderer, each adapter would re-implement event interpretation, modulation logic, and shot selection — risking semantic drift between adapters (e.g., the same `pass-shot-impact` shot fires for different sim conditions in dots vs 3D).

ADR-0002 originally specified a stylized-2D-illustrated rendering pipeline directly tied to sim output (URP custom passes for screen-tone, motion-line trails, impact flash, panel overlays). That coupling was acceptable when 2D-illustrated was the only target. With dots + 3D both candidates, the coupling is wrong shape: it conflates "what does the sim describe" (semantic) with "how is it rendered" (presentation).

### Current state

- MatchSim is renderer-free per the Contracts/impl asmdef split (TECH_APPROACH.md §3.1).
- `ShotTypeSO` (ADR-0001) carries shot identity + framing parameters + chain rules + reduce-motion variants.
- ADR-0002 specified ONE rendering pipeline; now superseded.
- No formal sim → viewer event interface exists yet; it would be authored greenfield in Phase 3.

### Constraints

- **MatchSim must stay Unity-free and presentation-adapter-free.** MatchSim emits canonical match events only. The presentation contract layer lives in sibling `Viewer.Contracts`; deterministic conversion from match events + pure `ShotTypeDefinition` content into `ViewerEvent`s is owned by a pure-C# `Viewer.EventBridge`.
- **Determinism preserved.** ViewerEvents include a deterministic seed (already locked via `Seed(match_seed, tick, event_id)` per ADR-0001 forbidden-nondeterminism). Adapters must not introduce non-deterministic interpretation of the contract.
- **Reduce-motion is adapter-aware** per `design/accessibility.md`. The contract carries a `reduce_motion: bool` modulation flag; each adapter implements its own reduce-motion behavior consuming the flag (ADR-0001 `reduce_motion_variant` substitution applies per-adapter).
- **Mod-pack loadability preserved.** Mods reference `ShotTypeSO` identities by `ContentPackQualifiedId` per `design/modding.md §1`; the contract layer doesn't change this.
- **Renderer adapters consume the contract; they do not extend it.** Adapters can ignore parts of the contract but cannot define new fields. New contract fields go through ADR-supersession or a contract-version bump.

---

## Decision

Introduce a `ShotPresentationContract` data layer + `ViewerEvent` stream in a new pure-C# `Viewer.Contracts` package. MatchSim remains canonical-sim-only. `Viewer.ShotAuthoring` loads Unity `ShotTypeSO` assets and projects them into pure-C# `ShotTypeDefinition` DTOs. `Viewer.EventBridge` consumes the ordered MatchSim event stream + `ShotTypeDefinition` catalog + memory-reader outputs and emits deterministic `ViewerEvent`s. Renderer adapters live in their own Unity-side projects and consume the contract through dependency on `Viewer.Contracts`.

### `ViewerEvent` schema (Phase-3 implementation contract)

```csharp
public sealed record ViewerEvent
{
    // Stable identity + ordering. Bridge assigns ViewerEventId from canonical
    // sim event order at emission time; once assigned, immutable. Pass-
    // activation trace ordering is (StartTick, ViewerEventId) — never adapter-
    // local sort. SourceEventId / SourceEventOrdinal carry the upstream sim
    // event identity verbatim for provenance + replay debugging.
    public ulong ViewerEventId;                        // bridge-assigned; monotonically increasing per match
    public ulong SourceEventId;                        // canonical sim event id (ADR-0004)
    public int   SourceEventOrdinal;                   // index of this ViewerEvent within the source event's emission burst (0 if 1:1)

    // Shot identity — base is what was authored (ShotTypeSO.Id from ADR-0001);
    // effective is what the bridge resolved AFTER reduce-motion substitution.
    // Adapters render `EffectiveShotTypeId`. `BaseShotTypeId` is provenance —
    // visible in pass-activation trace + debug overlay only.
    public ContentPackQualifiedId BaseShotTypeId;      // pre-substitution shot identity
    public ContentPackQualifiedId EffectiveShotTypeId; // post-substitution; adapters render THIS one
    public bool ReduceMotionApplied;                   // true iff bridge substituted reduce_motion_variant

    // Temporal window
    public Tick StartTick;                             // when in the sim this event begins
    public Tick EndTick;                               // when it ends; viewer interpolates between
    public Seed Seed;                                  // deterministic per (match, tick, event-id)

    // Modulation parameters (adapter-agnostic; each adapter renders these per its style)
    public Fixed StakesNormalized;                     // [0,1] from competition + scoreline + ledger relevance
    public Fixed MemoryRelevance;                      // [0,1] memory-hit intensity for participants
    public ContentPackQualifiedId? FocalSubject;       // explicit focal player; adapters use this for selection-ring / camera-target hints. Null if no single focal subject (e.g. tactical-wide).
    public ImmutableArray<ContentPackQualifiedId> ParticipantPlayerIds;
    public ImmutableArray<MemoryHit> MemoryHits;       // structured callbacks the adapter may surface

    // Event provenance (for replay determinism + adapter debugging)
    public EventClass SourceEventClass;                // ADR-0004 enum; the sim event that triggered this viewer event
    public ContentPackQualifiedId? SourceEntityId;     // if a player / signature triggered, the entity
}

public readonly struct MemoryHit
{
    public ContentPackQualifiedId ParticipantId;
    public CallbackTag Tag;                            // ADR-0004 tag enum
    public Fixed Salience;                             // [0,1]
    public ContentPackQualifiedId CallbackLineId;       // localized/lint-scanned line asset; adapter resolves by locale
    public ImmutableArray<CallbackSlotValue> Slots;     // deterministic slot values; no runtime prose generation
}

public readonly struct CallbackSlotValue
{
    public string SlotName;                             // e.g. "player_name", "club_name", "minute". Sorted ordinally — see Determinism contract §slot rules below.
    public ContentPackQualifiedId? EntityId;            // PREFERRED for player/club/signature slots; resolves locale at adapter-render time
    public string? LiteralValue;                        // fixed values only; never generated prose. Format with InvariantCulture; never with current-culture (locale drift would enter the trace).
}

public sealed record ShotTypeDefinition
{
    public ContentPackQualifiedId Id;                   // projected from ADR-0001 ShotTypeSO.Id
    public ShotCategory Category;
    public FramingParams Framing;
    public ModulationStrength Modulation;
    public ImmutableArray<ChainRuleDefinition> ChainRules;
    public ShotCategory FallbackCategory;
    public ReduceMotionVariant ReduceMotion;
    public ImmutableArray<OverlayTemplateReference> OverlayTemplates;
}
```

### Adapter interface (Unity-side, NOT MatchSim.Contracts)

```csharp
public interface IShotPresentationAdapter
{
    // Adapter declares which renderer it is; viewer system selects by this.
    AdapterId Id { get; }

    // Process a stream of ViewerEvents. Determinism: same input = same pass-activation trace.
    void ConsumeEvents(IEnumerable<ViewerEvent> events);

    // Adapter-specific reduce-motion handling. Adapter declares whether it
    // honors reduce_motion at scene-load-time (per ADR-0002 structural posture)
    // or per-frame (less preferred but allowed if adapter justifies).
    ReduceMotionStrategy ReduceMotionStrategy { get; }

    // Adapter renders the configured Pitch + the active ViewerEvents.
    // Pixel output is not part of the deterministic replay hash.
    void RenderFrame(PitchView pitchView, IReadOnlyList<ActiveViewerEvent> activeEvents);
}

public sealed record PitchView
{
    // Snapshot of the canonical sim state the adapter renders for this frame.
    // Bridge produces this from MatchSim canonical state at the requested
    // RenderTick; adapters MUST NOT read MatchSim state directly.
    public Tick RenderTick;
    public Fixed PitchLengthMetres;
    public Fixed PitchWidthMetres;
    public Vector2Fixed BallPosition;                  // Q32.32 fixed-point; (0,0) = pitch center
    public Vector2Fixed BallVelocity;
    public ContentPackQualifiedId? BallCarrierPlayerId; // null if loose ball

    // Player snapshots sorted by (TeamSide, ContentPackQualifiedId) — ordinal
    // string compare on the qualified id. Stable across runs and platforms.
    public ImmutableArray<PlayerSnapshot> Players;
}

public readonly struct PlayerSnapshot
{
    public ContentPackQualifiedId PlayerId;
    public TeamSide Side;                              // Home / Away
    public Vector2Fixed Position;
    public Vector2Fixed Velocity;
    public byte JerseyNumber;
}

public sealed record ActiveViewerEvent
{
    public ViewerEvent Event;                          // the underlying ViewerEvent — full schema above
    public Fixed Progress;                             // [0,1] = (RenderTick - StartTick) / (EndTick - StartTick); pinned at 1.0 once EndTick passed
    // Active set ordered by (StartTick, ViewerEventId). Adapters MUST iterate
    // in supplied order; they MUST NOT re-sort.
}

public enum AdapterId : ushort
{
    Dots         = 1,                                  // ADR-0009
    CelShaded3d  = 2,                                  // ADR-0010 conditional on Phase-5 spike
    // Future adapters reserve IDs here; new adapters require ADR +
    // decisions-log entry. The registry is intentionally CODE-OWNED, not
    // content-pack-loadable: a community / mod adapter is a code plugin
    // shipped via trusted-build channel (separate Steam beta branch, signed
    // sideload, etc.), NOT a normal Workshop-style content pack. Workshop
    // packs extend ShotTypeSO + content per ADR-0001 + design/modding.md;
    // they cannot register a new AdapterId.
}

public enum TeamSide : byte
{
    Home = 1,
    Away = 2,
}

public enum ReduceMotionStrategy : byte
{
    SceneLoadTime = 1,                                 // disable features at scene-load (ADR-0002 posture)
    PerFrame      = 2,                                 // runtime-branch; requires adapter justification
}
```

### Contract package boundary

`Viewer.Contracts` owns:
- `ViewerEvent` record
- `MemoryHit` struct
- `CallbackSlotValue` struct
- `ShotTypeDefinition` pure DTO projection of ADR-0001 `ShotTypeSO`
- `AdapterId` enum (registry; closed code-owned per `design/modding.md §5`)
- `ReduceMotionStrategy` enum

`Viewer.ShotAuthoring` owns Unity asset loading/projection:
- Addressables-loaded `ShotTypeSO` assets from ADR-0001
- projection to `ShotTypeDefinition` DTOs
- validation that the projection is deterministic and lossless for bridge-required fields

`Viewer.EventBridge` owns deterministic conversion:
- ordered MatchSim event stream → `ViewerEvent` stream
- `ShotTypeDefinition` selection + chain-rule evaluation
- memory-reader output → `MemoryHit` references
- pass-activation trace emission

`Viewer.Core` (Unity-side) owns:
- `IShotPresentationAdapter` interface
- adapter registry
- scene-load-time adapter selection
- frame dispatch
- `PitchView` + `ActiveViewerEvent` runtime types

`Viewer.Adapters.Dots` owns the dots implementation (per ADR-0009).

`Viewer.Adapters.CelShaded3d` owns the 3D implementation (per ADR-0010 conditional).

### Adapter selection

Single-adapter-active per scene. Adapter selection by:
1. Settings-panel choice (player chooses dots OR 3D when both are available).
2. Build-time configuration (some builds may be dots-only; others may include 3D).
3. Reduce-motion + adapter compatibility check (an adapter that doesn't support reduce-motion at all gets rejected if reduce-motion is on).

No live adapter swap mid-match. Switching adapters requires scene reload (consistent with ADR-0002's reduce-motion scene-load-time posture inherited here).

### Determinism contract

ViewerEvent derivation by `Viewer.EventBridge` is deterministic per `Seed(match_seed, tick, event_id)`. Adapters MUST be deterministic at the semantic trace level given the same ViewerEvent stream input — this is what makes the golden-replay-corpus `pass_activation_log_hashes` work (per `design/specs/golden-replay-corpus.md` + `design/accessibility.md` paired-fixture pattern). Pixel output is intentionally not hashed because cross-GPU / cross-driver differences are acceptable.

Adapter-side `_Time`-based shaders / `Random` calls / `DateTime.Now` etc. are forbidden in gameplay-affecting renders per ADR-0001 forbidden-nondeterminism + ADR-0002 (now superseded but rule preserved). Visual-only effects (pure cosmetic noise, decorative crowd ambient) are exempt; they don't enter the determinism log hash.

#### Ordering rules (locked at v1)

These rules are required to keep the pass-activation trace stable across machines, locales, and adapters:

- **ViewerEvent stream order:** sorted by `(StartTick ascending, ViewerEventId ascending)`. `ViewerEventId` is bridge-assigned monotonically per match.
- **PitchView.Players:** sorted by `(TeamSide ascending, PlayerId ordinal ascending)`. Ordinal compare via `StringComparer.Ordinal` — never `CurrentCulture`, never `InvariantCultureIgnoreCase` (case-folding is locale-tunable in some cultures).
- **MemoryHit.Slots:** sorted by `SlotName` ordinal ascending. Bridge enforces this at emission time so adapters never need to re-sort.
- **CallbackSlotValue.LiteralValue:** formatted with `CultureInfo.InvariantCulture` always. Numeric values use `ToString("R", InvariantCulture)` round-trippable form.

#### Pass-activation trace fields (what enters the hash)

The pass-activation trace is the deterministic semantic record per ViewerEvent. Each entry contains:

- `ViewerEventId` (ordering)
- `StartTick`
- `BaseShotTypeId.AsCanonicalString()` (ordinal)
- `EffectiveShotTypeId.AsCanonicalString()` (ordinal)
- `ReduceMotionApplied` (bool)
- `FocalSubject?.AsCanonicalString()` or empty
- `SourceEventClass`
- `SourceEventId`
- adapter-declared feature-toggle states (each adapter exposes a stable list of toggle names + bool/byte values; adapters with NO toggles produce an empty section)

What's deliberately EXCLUDED from the trace hash:

- `MemoryHit.CallbackLineId` rendered prose — the line ID is included; the localized text is not. Locale-rendered text drifts across builds + locale packs without any bug; trapping it would create false-positive hash mismatches.
- `MemoryHit.Slots[*].LiteralValue` rendered display — the structured value is included; any post-render display-formatting is not.

If a future fixture pins a specific locale (e.g., `en_GB`), the rendered text MAY enter the trace for that fixture, but only via an explicit `locale_pin: "en_GB"` field on the corpus fixture (per `design/specs/golden-replay-corpus.md`).

### Reduce-motion adapter-awareness

Per `design/accessibility.md`, reduce-motion is structurally scene-load-time. The substitution boundary is **locked at the bridge, not the adapter**:

- **`Viewer.EventBridge` performs the `reduce_motion_variant` substitution exactly once** per ViewerEvent, at emission time. The result is `EffectiveShotTypeId` (rendered) + `BaseShotTypeId` (provenance). `ReduceMotionApplied` records whether substitution actually happened (a shot may not have a `reduce_motion_variant` defined; in that case `EffectiveShotTypeId == BaseShotTypeId` and the flag is false).
- **Adapters do NOT re-substitute.** Adapters render `EffectiveShotTypeId` verbatim. They MAY further disable adapter-specific features (motion-line trails, camera-rhythm easing curves, etc.) when `ReduceMotionApplied == true` — but those feature-toggle states must be declared in the adapter's pass-activation trace section so the trace remains stable.

Each adapter declares its own reduce-motion feature-toggle list:

- **Dots adapter (ADR-0009):** `dots.trails_enabled`, `dots.camera_rhythm_simplified`, `dots.shot_transition_speed_modifier` (Q32.32 fixed-point ratio).
- **3D adapter (ADR-0010 conditional):** `3d.motion_lines_enabled`, `3d.impact_flash_enabled`, `3d.cel_shader_static` (inherits ADR-0002 specific intent in the 3D context).

Both adapters consume the same bridge-resolved `EffectiveShotTypeId`; their adapter-specific toggle states differ and enter the pass-activation trace per the rules above. The corpus `reduce_motion` fixture field (per `design/specs/golden-replay-corpus.md`) tests reduce-motion paths produce stable pass-activation traces while sharing identical sim canonical-state and `key_event_hashes`. Corpus schema v1 stores `pass_activation_log_hashes` as an adapter-keyed object from day one (initially `{ "dots": "..." }`); adding the 3D adapter is `{ "dots": "...", "celShaded3d": "..." }` — same v1 schema, no migration.

---

## Alternatives considered

### Alternative 1 — keep ADR-0002's coupled rendering pipeline; layer 3D on top later (REJECTED)

The original pipeline was tied to stylized-2D-illustrated. Adding a 3D path on top would either duplicate the event-interpretation logic (semantic drift risk) or force the 3D adapter to fake being a 2D renderer. Both fail the renderer-agnostic posture per the supersession.

### Alternative 2 — emit framebuffer-level instructions from the sim ("draw a circle here, draw a sprite there") (REJECTED)

Tightly couples sim to specific rendering primitives. Mods can't extend; new adapters can't reinterpret. Same anti-pattern as Unity-coupled MatchSim that we already rejected.

### Alternative 3 — emit only raw sim events; let viewer compute everything (REJECTED)

Viewer would re-implement shot selection, modulation logic, memory-hit resolution per adapter. Drift risk + duplicated work + semantic ambiguity. The whole point of `ShotTypeSO` (ADR-0001), projected to pure `ShotTypeDefinition`, is data-driven shot identity; we already commit to that — `ViewerEvent` carries the resolved shot identity as the contract's primary content.

### Alternative 4 — put the contract inside `MatchSim.Contracts` (REJECTED)

Avoids one extra asmdef but leaks presentation vocabulary into the canonical sim package. Rejected because the 2026-04-26 supersession explicitly preserves renderer-free MatchSim. `Viewer.Contracts` is a pure-C# sibling; the extra boundary is worth the architectural cleanliness.

### Alternative 5 — version the contract per-adapter (REJECTED)

Per-adapter contract versioning would let adapters evolve independently. Sounds clean but breaks the single-source-of-truth: the same sim event would emit different `ViewerEvent` shapes per adapter, defeating renderer-agnostic posture. Single-version contract; both adapters consume the same shape.

---

## Consequences

### Positive

- **Renderer-agnostic.** Same sim works with dots, 3D, future adapters without sim-side changes.
- **Reversible.** If 3D fails the spike, dots adapter remains shipping-quality without architecture rewrite. If dots is later replaced with stylized-2D-illustrated post-EA, that's a third adapter consuming the same contract.
- **Mod-pack-friendly.** Mods reference `ShotTypeSO` per `ContentPackQualifiedId`; the contract resolves these adapter-agnostically.
- **Test-friendly.** Adapter-determinism tests run independently of sim-determinism tests. Phase-3 corpus schema v1 stores `pass_activation_log_hashes` as an adapter-keyed object from day one (initially `{ "dots": "..." }`); the 3D adapter enters by adding `"celShaded3d": "..."` — same v1 schema, no migration required.
- **Accessibility-clean.** `reduce_motion` flag flows through the contract; adapters interpret per their style; `golden-replay-corpus.md` paired-fixture pattern works for both.
- **Memory-callback unified.** `MemoryHit` carries callback line IDs + deterministic slot values. Both adapters resolve the same localized, lint-scanned text (per accessibility + content-policy + ui-vocabulary discipline); only the visual presentation differs.

### Negative / Risks

- **Knowledge Risk LOW** but **Migration Cost MEDIUM:** ADR-0002's drafted URP pipeline machinery (screen-tone passes, motion-line trail meshes, impact-flash, UI Toolkit panel overlays) is now scoped to a specific adapter's implementation rather than a system-level commitment. Some sketches in ADR-0002 are still useful for the eventual 3D adapter; some are 2D-illustrated-specific and effectively retired.
- **Adapter coverage requirement.** Every viewer feature now has to ask "which adapter(s) does this apply to?" — added cognitive overhead. Mitigation: most features are sim-side and adapter-agnostic by default; only renderer-specific features (cel-shader / dots-camera-rhythm / etc.) carry adapter scope.
- **Contract evolution cost.** Adding a field to `ViewerEvent` is a schema bump to `Viewer.Contracts` that all adapters must absorb. Phase-3 Week-1 needs to lock the v1 contract carefully so v2 doesn't come too soon. Open question for Phase-3 review: is the v1 contract above complete enough for Phase-3 dots prototype?
- **Projection surface.** `ShotTypeSO` is a Unity asset, but `Viewer.EventBridge` is pure C#. The `ShotTypeSO` → `ShotTypeDefinition` projection is a new seam that needs fixture coverage so Unity-side asset fields and pure bridge fields cannot drift.

### Neutral

- **One new pure-C# contracts package.** `Viewer.Contracts` becomes the consumer-facing viewer schema package; `MatchSim.Contracts` remains canonical simulation schema only.
- **Determinism story unchanged.** Same `Seed` derivation; same canonical-state hash; adapter-keyed `pass_activation_log_hashes` cover each compiled adapter's semantic pass-activation trace, not rendered pixels.

---

## Validation criteria

- [ ] Phase-3 Week-2: dots adapter (ADR-0009) consumes `ViewerEvent`s end-to-end and produces a 3-shot-type prototype rendering. Confirms the contract is implementable.
- [ ] Phase-3 Week-2: golden-replay-corpus paired fixtures (`<seed>.json` + `<seed>.reduce-motion.json`) pass for the dots adapter via adapter-keyed `pass_activation_log_hashes["dots"]`. Confirms adapter semantic-trace determinism.
- [ ] Phase-3 Week-2: synthetic ViewerEvent fixture (per ADR-0009 §Tier-A CI integration) covers all three Week-2 shot types; trace is non-empty + stable. Confirms the smoke test isn't passing on an empty trace.
- [ ] Phase-3 Week-3: a synthetic "second adapter" stub (just logs `ViewerEvent`s to JSON, doesn't render) passes the same contract consumption tests + populates `pass_activation_log_hashes["stub"]`. Confirms renderer-agnostic posture + adapter-keyed corpus schema.
- [ ] Phase-5: 3D-pipeline spike (per `design/3d-pipeline.md`) implements ADR-0010 against the same contract without sim-side changes. Confirms the contract holds across adapters.
- [ ] No `_Time` / `Random` / `DateTime.Now` references in adapter renderers per `fw shader-audit` + asmdef-level lint. Confirms determinism preserved.
- [ ] `Viewer.EventBridge` ordering tests: ViewerEventId monotonic per match; PitchView.Players ordinal-sorted; MemoryHit.Slots ordinal-sorted. Confirms ordering rules locked at v1.

---

## Open questions

1. **Callback line shape.** `MemoryHit.CallbackLineId` assumes one localized line asset + deterministic slots per callback. Multi-line / subtitle-specific variants / template-with-runtime-slots? Pairs with `design/event-sourced-memory.md` reader-side rendering. Resolve at Phase-3 when `MemoryEvent → MemoryHit` conversion is implemented. (NOTE: 2026-04-27 review pass locked the slot-ordering + locale-rendering rules in §Determinism contract; what remains open is the line-asset structure itself, not its hash treatment.)

2. **Audio-cue hook + debug-overlay metadata.** Deferred to a future v2 contract bump if Phase-3 Week-2 dots-adapter authoring surfaces a need. Audio is currently routed via FMOD-side event names tied to `EffectiveShotTypeId`; debug overlays consume the pass-activation trace directly. If neither is sufficient, file a contract v2 RFC with concrete fields. Default posture: don't add fields speculatively.

3. **Polish-bar measurability.** ADR-0009 §Polish bar locks the operational observer-task script; this ADR's role is structural only. If the observer-task pass/fail rates surface contract-level gaps (e.g., adapters can't communicate "signature fired" without an extra field), revisit here.

(2026-04-27 review pass closed the prior open questions:
**Contract v1 completeness** — locked: `ViewerEventId`, `SourceEventId`, `SourceEventOrdinal`, `BaseShotTypeId`, `EffectiveShotTypeId`, `ReduceMotionApplied`, `FocalSubject` added.
**PitchView abstraction** — locked minimum shape (RenderTick, pitch dimensions, ball position/velocity, ball-carrier, sorted PlayerSnapshots).
**ActiveViewerEvent vs ViewerEvent** — locked: `ActiveViewerEvent` wraps `ViewerEvent` + `Progress`; ordered active set.)

---

## Cross-references

- **2026-04-26 SPEC decisions-log entry** — visual-target supersession (this ADR's authority)
- **`design/3d-pipeline.md`** — adapter-specific 3D pipeline + spike-gate criteria
- **ADR-0001 ShotTypeSO** — shot identity referenced by `ViewerEvent.BaseShotTypeId` / `ViewerEvent.EffectiveShotTypeId`
- **ADR-0002 (Superseded)** — original stylized-2D rendering pipeline; preserved for history
- **ADR-0004 MemoryEvent** — `EventClass` + `CallbackTag` + `MemoryHit` source
- **ADR-0005 SignatureSO** — signature-trigger event source
- **ADR-0009 dots-phase render adapter** (Proposed sibling) — first consumer of this contract
- **ADR-0010 3D cel-shaded render adapter** (NOT pre-authored; conditional on Phase-5 spike)
- **`design/semantic-cinema.md`** — 7-shot vocabulary; rendering implementation now adapter-specific
- **`design/accessibility.md`** — reduce-motion adapter-aware contract
- **`design/specs/golden-replay-corpus.md`** — `pass_activation_log_hashes` is adapter-keyed from schema v1 (`"dots"` initially; `"celShaded3d"` when ADR-0010 enters); key_event_hashes are sim-side
- **`design/specs/content-pack-validation-contract.md`** — `FW-VAL-D-011` (added at this supersession) covers 3D-asset commercial-rights for the eventual 3D adapter

## Changelog within this doc

- **2026-04-26** — Authored as Proposed per visual-target supersession decisions-log entry. Supersedes ADR-0002. ViewerEvent + ShotPresentationContract schemas drafted. Post-review cleanup moved the contract into pure-C# `Viewer.Contracts` + `Viewer.EventBridge` so `MatchSim.Contracts` remains canonical-sim-only, replaced pre-rendered callback text with callback line IDs + deterministic slots, and clarified that pass-activation hashes cover semantic traces rather than rendered pixels. AdapterId enum reserves Dots=1 + CelShaded3d=2; future adapters require ADR + decisions-log entry. Five rejected alternatives captured. Four open questions for Phase-3 Week-1 resolution. Awaits user / GPT-5.5 review pass before flipping to Accepted.
- **2026-04-27** — GPT-5.5 review pass landed (Concerns verdict; 4 P1 + 4 P2 findings). Applied: (P1-1) added `ViewerEventId` + `SourceEventId` + `SourceEventOrdinal` for stable identity + ordering; (P1-2) split `ShotTypeId` into `BaseShotTypeId` + `EffectiveShotTypeId` + `ReduceMotionApplied` so adapters render the bridge-resolved effective shot only; (P1-3) locked minimum `PitchView` (RenderTick / pitch dimensions / ball position+velocity / ball-carrier / sorted PlayerSnapshots) + `ActiveViewerEvent` (wraps ViewerEvent + Progress) shapes — they are now part of the contract, not deferred; (P1-4) aftermath-freeze sim-time-pause language was actually in ADR-0009 §7-shot-table, fixed there in companion edit. (P2-5) added §Determinism contract ordering rules: stream order is `(StartTick, ViewerEventId)`, PitchView.Players ordinal-sorted by `(TeamSide, PlayerId)`, MemoryHit.Slots ordinal-sorted by SlotName, LiteralValue formatted with InvariantCulture; pass-activation trace fields enumerated + locale-rendered prose explicitly excluded unless corpus pins a locale. (P2-6) corpus `pass_activation_log_hashes` is adapter-keyed from v1 (initially `{ "dots": "..." }`); `golden-replay-corpus.md` updated in companion edit; no future schema migration. (P2-8) synthetic ViewerEvent fixture posture added to validation criteria so the 60-tick smoke can't pass on an empty trace. AdapterId enum gained an explicit comment that the registry is code-owned: community/mod adapters are code plugins via trusted-build channel, NOT Workshop-style content packs. Open questions list reduced from 4 to 3 (CallbackLine shape narrowed, audio/debug deferred to v2 RFC if needed, polish-bar measurability cross-references ADR-0009). Status remains Proposed; awaits Codex review pass before flipping to Accepted.
- **2026-04-27** — Codex review pass + Accept flip. Codex confirmed all eight GPT-5.5 findings landed correctly and applied stale-reference cleanup across `pass_activation_log_hash` (singular → adapter-keyed plural) in 4 docs (ADR-0008 §Consequences + §Cross-references; ADR-0009 §Cross-references; SPEC.md task-list note for golden-corpus spec; CHANGELOG.md two retroactive entries). Two additional stale references (`design/specs/artifact-retention-policy.md §Determinism-replay reproducibility-by-tag`, `design/accessibility.md §Replay/viewer test expectations`) caught at flip-time and cleaned. Codex sign-off: "no remaining blocking findings; ADR-0008 + ADR-0009 OK to flip Proposed → Accepted." Status now **Accepted**; append-only from this point forward.

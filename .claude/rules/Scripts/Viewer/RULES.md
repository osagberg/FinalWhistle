---
paths:
  - "unity-project/Assets/Viewer/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Viewer — renderer-agnostic shot presentation

Per ADR-0008 + ADR-0009 (both Accepted 2026-04-27): the viewer renders MatchSim canonical events through a renderer-agnostic `ShotPresentationContract`. The Phase-3-onward dots-prototype (sprite-on-pitch, ADR-0009) is the Month-3 validation visual + a candidate shipping layer; cel-shaded 3D is a candidate shipping layer gated on the Phase-5/6 production-feasibility spike per `design/3d-pipeline.md`. Visual choice is a renderer-adapter decision, not sim-bearing — same `ShotTypeSO` IDs drive every adapter.

Asmdef structure (per `unity-project/Assets/Viewer/`):

- `Viewer.Contracts` — pure-C# DTOs (ViewerEvent / ShotTypeDefinition / ShotTypeCatalog / ShotCategory / AdapterId / ReduceMotionStrategy / SignatureRecipeMetadata / MemoryHit / CallbackSlotValue) **+ `Viewer.EventBridge`** (deterministic conversion from ordered MatchSim canonical events to ViewerEvents — consumes already-projected pure `ShotTypeDefinition` DTOs). **`noEngineReferences: true`** + **`autoReferenced: false`** — UnityEngine-free at compile time, same architectural posture as MatchSim. Asmdef-level enforcement, not just convention. EventBridge home is locked here per SPEC 2026-04-30 Codex round-4 entry: hosting the bridge in `Viewer.Core` would inherit UnityEngine access (Time / Random / GameObject / scene state) and silently violate ADR-0008's deterministic-conversion contract; locking at the Contracts layer makes any `using UnityEngine` in bridge code a compile error rather than a reviewer-discipline issue.
- `Viewer.Core` — Unity-side adapter registry + **`IShotPresentationAdapter`** interface + **`PitchView`** (Q32.32 → `UnityEngine.Vector3` boundary) + **`ActiveViewerEvent`** (runtime wrapper resolving `ShotTypeCatalog` lookup once per event-window entry) + the future `ShotTypeSO → ShotTypeDefinition` projection seam (Phase-4+). `PitchView` + `ActiveViewerEvent` live HERE — not in `Viewer.Contracts` — because they depend on `UnityEngine.Vector3` which the Contracts asmdef forbids. SPEC 2026-04-30 dots-adapter blueprint Decision 1 locks this. `autoReferenced: false`.
- `Viewer.Adapters.Dots` — concrete dots-prototype implementation; consumes Core + Contracts + URP runtime. `autoReferenced: false`.

Reference graph: `Adapters.Dots → Core → Contracts`. One-way; Contracts is the apex.

## MUST

- Treat MatchSim canonical events as read-only input; viewer code cannot author, modify, or correct canonical match state. The bridge (`FinalWhistle.Viewer.Contracts.Viewer.EventBridge`) DERIVES `ViewerEvent`s from MatchSim events; it never emits canonical sim state.
- Honor `noEngineReferences: true` on `Viewer.Contracts`. A `using UnityEngine` there fails compilation by design.
- Map ViewerEvents into the locked 7-shot vocabulary from `design/semantic-cinema.md` (`tactical-wide` / `diagonal-attack-lane` / `pass-shot-impact` / `player-isolation` / `aftermath-freeze` / `crowd-reaction` / `tunnel-vision-press`). Adapters render shots; Contracts identifies them.
- Resolve `ReduceMotion` ONCE at the bridge boundary per ADR-0008: `BaseShotTypeId` → `EffectiveShotTypeId` substitution + `ReduceMotionApplied` flag travels on the ViewerEvent. Adapters consume the resolved variant.
- Order ViewerEvents by `(StartTick, ViewerEventId)` per ADR-0008 §Determinism contract. Stable across replays.
- Never read `_Time` / `_SinTime` / `_CosTime` / `_DeltaTime` / `_TimeParameters` / ShaderGraph `TimeNode` in viewer-adapter shaders. The match-replay corpus pins adapter-keyed pass-activation hashes per seed; frame-time intrinsics break replay reproducibility. `scripts/fw shader-audit` enforces this on `Assets/Viewer/**` at Tier-A CI.
- Verify visual work with runtime evidence: UnityMCP screenshot/capture or exported match-replay clip. Comments + diff alone are insufficient for adapter-rendering changes.
- Respect `design/ui-vocabulary.md` — no capitalized state nouns in overlays or commentary.

## SHOULD

- Keep camera/shot recipes data-driven via `ShotTypeSO` ScriptableObjects projected through `Viewer.Core` to pure-C# `ShotTypeDefinition` DTOs in Contracts. Code reads DTOs, not SOs.
- Use stakes + memory hits as modulation inputs to existing shot types, not new bespoke ones.
- Hold to ADR-0009's polish bar (kit discrimination / identity legibility / camera rhythm / signature presentation cues / commentary integration / reduce-motion). Dots may ship at EA per the 2026-04-26 visual-target supersession outcome (c2); not debug-quality.
- Prefer `UI Toolkit` (UXML/USS) for overlay text + identity panels per CLAUDE.md tech stack lock; UGUI fallback only when UI Toolkit lacks the surface.
- Log selected `EffectiveShotTypeId` + source `ViewerEventId` for replay debugging. Adapter-keyed `pass_activation_log_hashes` per `design/specs/golden-replay-corpus.md` v1.

## AVOID

- Per-signature unique cinematics before all 7 base shot types work end-to-end + the polish bar passes the Month-3 observer rubric.
- Decorative 3D dependencies in the dots adapter — that's the 3D adapter's domain (conditional on Phase-5/6 spike outcome).
- UI Toolkit overlays that cover critical match action (the dots viewer reads through occlusion).
- Hardcoded text strings outside localization/content tables. Commentary templates live in content packs.
- Authoring a 2D-illustrated / manga-broadcast asset path. The 2D-stylized-manga-as-final-identity framing was superseded by ADR-0008/0009 (see SPEC decisions log 2026-04-26 + 2026-04-26 correction-pass entries).

## References

- [ADR-0008 ShotPresentationContract](../../../../../design/adr/adr-0008-shot-presentation-contract.md)
- [ADR-0009 dots-phase render adapter](../../../../../design/adr/adr-0009-dots-phase-render-adapter.md)
- [design/semantic-cinema.md](../../../../../design/semantic-cinema.md)
- [design/ui-vocabulary.md](../../../../../design/ui-vocabulary.md)
- [design/month-3-vertical-slice.md](../../../../../design/month-3-vertical-slice.md)
- [design/3d-pipeline.md](../../../../../design/3d-pipeline.md) — sibling 3D adapter (conditional on Phase-5 spike)
- [design/specs/golden-replay-corpus.md](../../../../../design/specs/golden-replay-corpus.md) — adapter-keyed pass-activation hashes

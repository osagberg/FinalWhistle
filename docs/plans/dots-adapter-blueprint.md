# Dots-Phase Render Adapter — Implementation Blueprint

**Authored 2026-04-30** by `feature-dev:code-architect` for SPEC line 149 — the dots-phase render adapter prototype per ADR-0009. First runtime-rendering consumer of the Phase-3 hardened `Viewer.Contracts.EventBridge` stream. Anchors: ADR-0008 (renderer-agnostic shot-presentation contract) + ADR-0009 (dots-phase adapter scope + polish bar).

> Subsequent /next iterations should read this file before spawning slice-specific subagents. Each slice ships as one /next + /done cycle.

---

## Codebase State Summary

**What already exists:**

- `unity-project/Assets/Viewer/Contracts/` — 9 hardened types: `ViewerEvent`, `ShotTypeDefinition`, `ShotTypeCatalog`, `ShotCategory`, `EventBridge`, `AdapterId`, `ReduceMotionStrategy`, `MemoryHit`, `CallbackSlotValue`, `SignatureRecipeMetadata`. Asmdef: `noEngineReferences: true`, precompiled ref to `FinalWhistle.MatchSim.dll`.
- `unity-project/Assets/Viewer/Core/Viewer.Core.asmdef` — shell only; references `Viewer.Contracts` + `FinalWhistle.MatchSim.dll` precompiled. No implementation files yet.
- `unity-project/Assets/Viewer/Adapters/Dots/Viewer.Adapters.Dots.asmdef` — shell only; references `Viewer.Core`, `Viewer.Contracts`, `Unity.RenderPipelines.Universal.Runtime`. No implementation files.
- `MatchSimulationRunner.RunTicks` — fully operational, signature-aware, `SignatureCooldownState` on `MatchSimulationState`.
- 22 `IdentityPacket` JSON fixtures loaded via `IdentityPackets.LoadAll()`.
- 3 active signatures (`LowCutback` #20, `BlindSideNearPostRun` #22, `FirstTimeDiagonalSwitch` #13) emitting `KeyEvent` + `SignaturePresentationRecipe` streams.
- `EventBridge.Derive()` — pure-static, determinism-clean, returns ordered `IReadOnlyList<ViewerEvent>`.

**What does NOT exist yet (everything the dots adapter needs):**

- `IShotPresentationAdapter` interface (lives in `Viewer.Core`)
- `PitchView` DTO and `ActiveViewerEvent` runtime wrapper (in `Viewer.Core`)
- `ShotTypeSO` ScriptableObject + authoring (in `Viewer.Adapters.Dots`)
- `DotsMatchDirector` MonoBehaviour — scene orchestrator
- `PitchRenderer` — URP SpriteRenderer quads for the pitch surface
- `DotPool` — 22 player dots + 1 ball dot, pooled `SpriteRenderer`s
- `ShotCamera` for shot framing (`tactical-wide`, `diagonal-attack-lane`, `pass-shot-impact`)
- URP custom render passes (screen-tone, impact-frame flash)
- UI Toolkit overlay (scoreboard, commentary, signature title-card)
- Identity/kit tinting from `IdentityPacket` `RoleFamily` + team side

---

## A. Architecture Overview

### Asmdef chain

```
Viewer.Contracts   (noEngineReferences=true, pure-C# DTOs + EventBridge + ShotTypeCatalog)
       ↑
Viewer.Core        (Unity-side: IShotPresentationAdapter, PitchView, ActiveViewerEvent,
                   ShotTypeSO → ShotTypeDefinition projection seam)
       ↑
Viewer.Adapters.Dots   (concrete MonoBehaviours + Shaders + UXML/USS + SOs)
```

`Viewer.Contracts` is the apex. Nothing flows down. No engine deps allowed in Contracts.

### Runtime data flow per tick

```
MatchSimulationRunner.RunTicks(state, ...)       [headless; pure C#]
        ↓
state.KeyEvents + state.SignatureRecipes          [canonical stream, read-only]
        ↓
EventBridge.Derive(state, seed, reduceMotion)     [pure-static; Viewer.Contracts]
        ↓ IReadOnlyList<ViewerEvent>
DotsMatchDirector.AdvanceTick()                   [MonoBehaviour; Viewer.Adapters.Dots]
  → maps ViewerEvent window to ActiveViewerEvent
  → calls IShotPresentationAdapter.PresentShot(active)
        ↓
DotsAdapterRoot.PresentShot(active)               [implements IShotPresentationAdapter]
  → DotPool.UpdatePositions(state.HomeTeam, state.AwayTeam, state.Ball)
  → ShotCamera.Frame(active)
  → OverlayController.Push(active)
  → URP custom passes receive stakes float uniform
        ↓
URP camera output → screen
```

The sim runs at 60 ticks/second (logical), Unity renders at display framerate. The `DotsMatchDirector` drives the sim in 1-tick-per-`FixedUpdate` batches so FixedUpdate rate (= 60 Hz when `Time.fixedDeltaTime = 1/60f`) matches the sim rate. `DotPool` interpolates dot screen positions between ticks in `Update` using the previous and current `PlayerState.Position` values converted from Q32.32 to `Vector3` — this interpolation is viewer-only and never touches canonical state.

### MonoBehaviour vs ScriptableObject vs static-class decisions

| Component | Type | Reason |
|---|---|---|
| `DotsMatchDirector` | MonoBehaviour (scene singleton via `[DefaultExecutionOrder]`) | Owns lifecycle, drives sim ticks, holds `ViewerEvent` queue |
| `DotsAdapterRoot` | MonoBehaviour, implements `IShotPresentationAdapter` | Concrete dispatch to sub-systems; one per scene |
| `DotPool` | MonoBehaviour | Needs `FixedUpdate`/`LateUpdate` access for interpolation; manages 23 `SpriteRenderer` children |
| `ShotCamera` | MonoBehaviour | Drives `Camera` component each frame; reads `ActiveViewerEvent.ShotCategory` |
| `OverlayController` | MonoBehaviour | Drives `UIDocument`; owns UXML tree lifetime |
| `ScreenTonePass` | `ScriptableRendererFeature` + `ScriptableRenderPass` | URP pipeline insertion; scene-global |
| `ImpactFramePass` | `ScriptableRendererFeature` + `ScriptableRenderPass` | URP pipeline insertion; event-triggered |
| `ShotTypeSO` | `ScriptableObject` | Data-driven framing parameters; 3 assets for Phase-3 shot types |
| `IdentityTintTable` | `ScriptableObject` | Maps `RoleFamily` + `TeamSide` → `Color`; single shared asset |
| `IShotPresentationAdapter` | Interface in `Viewer.Core` | Adapter contract; future 3D adapter implements same interface |
| `PitchView`, `ActiveViewerEvent` | Pure-C# classes in `Viewer.Core` | DTOs; no MonoBehaviour needed |
| `ShotTypeCatalog` | Static class in `Viewer.Contracts` | Already exists; Phase-3 hard-coded catalog |

---

## B. Sub-Task Ladder

### Slice 1 — `IShotPresentationAdapter` + `PitchView` + `ActiveViewerEvent` contracts in `Viewer.Core`

**What gets demonstrated:** The Viewer.Core asmdef compiles with the three contract types that every downstream slice depends on. No visual output — this is the foundation other slices mount on.

**Files to create:**
- `unity-project/Assets/Viewer/Core/IShotPresentationAdapter.cs`
- `unity-project/Assets/Viewer/Core/PitchView.cs`
- `unity-project/Assets/Viewer/Core/ActiveViewerEvent.cs`

**Files to modify:** none expected; the existing `Viewer.Core.asmdef` should already permit these new files.

**Key type shapes:**

```csharp
// Viewer.Core / IShotPresentationAdapter.cs
public interface IShotPresentationAdapter
{
    AdapterId AdapterId { get; }
    void Initialize(PitchView pitch);
    void PresentShot(ActiveViewerEvent active);
    void Teardown();
}

// Viewer.Core / PitchView.cs
public sealed class PitchView
{
    public float PitchLengthMeters { get; }   // 105f
    public float PitchWidthMeters { get; }    // 68f
    public Vector3 Origin { get; }            // world-space centre of pitch quad
    public float WorldUnitsPerMeter { get; }  // 1f default; 1 metre → 1 Unity unit
    public Vector3 FixedToWorld(Vector3Fixed pos);
}
// Note: Codex round-1 follow-up against b8d400f renamed the original
// MetersPerUnit field — the math multiplied (units-per-metre semantics) but
// the name read like a divisor, so a non-default scale would have inverted
// the world. WorldUnitsPerMeter is the canonical name + matches the multiply.

// Viewer.Core / ActiveViewerEvent.cs
public sealed class ActiveViewerEvent
{
    public ViewerEvent Event { get; }
    public ShotCategory ShotCategory { get; }
    public ShotTypeDefinition ShotDef { get; }
    public float StakesFloat { get; }
    public int ElapsedTicks { get; }
}
```

**Dependencies:** None (first slice). Anchors: ADR-0008 §"Adapter interface".

**Acceptance criteria:**
- `Viewer.Core.asmdef` compiles with zero errors in Unity Editor (L1 verify)
- All three types accessible from `Viewer.Adapters.Dots` asmdef (compile-time)
- `PitchView.FixedToWorld` unit-tested via EditMode test

**Required agents:** `lead-programmer` (architecture review of the interface shape) + `feature-dev:code-architect` (confirm three-type surface is future-proof for the 3D adapter without API churn).

**Verification path:**
- L1: compile via UnityMCP `run_tests` (EditMode compile check) — **UnityMCP required; flag if disconnected**
- L2: not applicable (no runtime behaviour)

**LoC estimate:** ~150 LoC. Within /next scope.

---

### Slice 2 — Static pitch quad + `DotPool` rendering 22 players + ball visible on screen

**What gets demonstrated:** Opening the viewer scene shows a green pitch rectangle with 23 coloured circles (11 home, 11 away, 1 ball) at their archetype formation positions. No camera animation or events yet — but something is on screen.

**Files to create:**
- `unity-project/Assets/Viewer/Adapters/Dots/DotsMatchDirector.cs` — stub MonoBehaviour wiring `MatchSimulationRunner` + `EventBridge`; for this slice, just initialises positions
- `unity-project/Assets/Viewer/Adapters/Dots/DotPool.cs` — manages 23 `SpriteRenderer` GameObjects
- `unity-project/Assets/Viewer/Adapters/Dots/PitchQuad.cs` — 105×68 green quad via `Mesh` + `MeshFilter` + `MeshRenderer`; UV-mapped for future screen-tone pass
- `unity-project/Assets/Viewer/Adapters/Dots/IdentityTintTable.cs`
- `unity-project/Assets/Viewer/Adapters/Dots/IdentityTintTable.asset` — `RoleFamily` × `TeamSide` → `Color`
- `unity-project/Assets/Viewer/Adapters/Dots/Scenes/DotsViewer.unity`
- `unity-project/Assets/Viewer/Adapters/Dots/Sprites/dot_player_home.png`
- `unity-project/Assets/Viewer/Adapters/Dots/Sprites/dot_player_away.png`
- `unity-project/Assets/Viewer/Adapters/Dots/Sprites/dot_ball.png`

**Pitch geometry:** mesh at world origin, half-extents `(52.5f, 0f, 34f)` in XZ plane (Y=0 is the pitch surface). Camera top-down orthographic, `orthographicSize = 38f`, `rotation = Quaternion.Euler(90, 0, 0)`. **Hand-rolled — no Cinemachine** for Phase-3 dots adapter.

**Dependencies:** Slice 1.

**Acceptance criteria:** Scene loads; 23 dots visible at formation; home vs away colour legible at full pitch scale; GK distinct.

**Required agents:** `unity-specialist`. Async `art-director` review of palette.

**Verification path:** L1 compile + L2 UnityMCP screenshot.

**LoC estimate:** ~350 LoC.

---

### Slice 3 — Live sim playback: 22 players + ball move in real time

**What gets demonstrated:** Sim runs at 60 ticks/second in `FixedUpdate`. Dots move continuously in football-shaped patterns (pressing, formations, ball chasing). First moment it feels like a match.

**Files to create:**
- `unity-project/Assets/Viewer/Adapters/Dots/DotsAdapterRoot.cs` — implements `IShotPresentationAdapter`; for this slice `PresentShot` is no-op stub.

**Files to modify:**
- `DotsMatchDirector.cs` — `FixedUpdate` calls `MatchSimulationRunner.RunTicks(state, ..., ticks: 1)`; consumes `EventBridge.Derive` and tracks high-water-mark `_lastProcessedViewerEventId`
- `DotPool.cs` — sub-tick interpolation in `Update`

**Key decision: re-derive full `ViewerEvent` list once per tick** (not incremental). At Phase-3 KeyEvent counts (~10 per match) this is negligible. Phase-4 may need an incremental API; Decision 4 covers this.

**Dependencies:** Slices 1 + 2.

**Acceptance criteria:** Football-shaped movement; no stutter; canonical 60-tick smoke hash unchanged; `fw shader-audit` clean.

**Required agents:** `unity-specialist` + `gameplay-programmer` review of the FixedUpdate timing + chunked-RunTicks behaviour against `SignatureCooldownState` persistence.

**Verification path:** L1 + L2 + canonical-hash regression test.

**LoC estimate:** ~250 LoC.

---

### Slice 4 — Camera shot framing for `tactical-wide`, `diagonal-attack-lane`, `pass-shot-impact`

**What gets demonstrated:** When a `ViewerEvent` fires (goal, signature execution), the camera cuts to the appropriate framing for that shot type. The three Phase-3 shot types are visually distinct.

**Files to create:**
- `unity-project/Assets/Viewer/Adapters/Dots/ShotCamera.cs`
- `unity-project/Assets/Viewer/Adapters/Dots/ShotTypeSO.cs`
- `unity-project/Assets/Viewer/Adapters/Dots/ShotTypes/tactical-wide.asset`
- `unity-project/Assets/Viewer/Adapters/Dots/ShotTypes/diagonal-attack-lane.asset`
- `unity-project/Assets/Viewer/Adapters/Dots/ShotTypes/pass-shot-impact.asset`

**Phase-3 framing table:**

| Shot | Description | `orthographicSize` | tilt (Euler X) | target anchor |
|---|---|---|---|---|
| `tactical-wide` | Full pitch overhead; default fallback | 38f | 90f | ball XZ |
| `diagonal-attack-lane` | Zoomed follow of an attacking lane; near-isometric | 20f | 75f | ball ↔ focal-subject midpoint |
| `pass-shot-impact` | Tight close-in on receiver/shooter at contact | 12f | 80f | focal-subject; null → ball |

`ShotCamera` Lerps `orthographicSize` and `transform.rotation` over `ShotTypeSO.TransitionTicks` (default 12 ticks = 0.2s). Target tracked in `LateUpdate` after `DotPool.Sync`.

**`diagonal-attack-lane` is adapter-local heuristic** (not bridge-emitted) — Decision 3.

**Dependencies:** Slices 1-3. Anchors: `design/semantic-cinema.md` §"7 shot types" + §Q2.

**Acceptance criteria:** Goal events trigger `pass-shot-impact`; default = `tactical-wide`; wide ball motion → `diagonal-attack-lane`; smooth Lerp over 0.2s; no edge-clipping at any framing.

**Required agents:** `unity-specialist` + async `art-director` review of framing aesthetics after L2.

**Verification path:** L1 + L2 multi-screenshot comparison + L3 eye-test.

**LoC estimate:** ~320 LoC.

---

### Slice 5 — URP custom passes: screen-tone overlay + impact-frame flash

**What gets demonstrated:** On `pass-shot-impact` event at `StakesNormalized >= 0.7f`, brief white-flash impact frame. Diagonal screen-tone overlay during `aftermath-freeze`. Anime presentation budget items 1 + 2.

**Files to create:**
- `unity-project/Assets/Viewer/Adapters/Dots/Shaders/ScreenTone.hlsl` — fullscreen URP custom pass; `_StakesNormalized` + `_ElapsedTicks` uniforms. **No `_Time` reads.**
- `unity-project/Assets/Viewer/Adapters/Dots/Shaders/ImpactFrame.hlsl` — `_FlashIntensity` uniform; decays over `_FlashDecayTicks`
- `unity-project/Assets/Viewer/Adapters/Dots/Passes/ScreenToneRendererFeature.cs` — `ScriptableRendererFeature` injects after `RenderPassEvent.AfterRenderingPostProcessing`
- `unity-project/Assets/Viewer/Adapters/Dots/Passes/ImpactFrameRendererFeature.cs` — same pattern; ordered after ScreenTone

**Files to modify:**
- `DotsMatchDirector.cs` — drive `_StakesNormalized` per frame; trigger `_FlashIntensity` on goals + high-stakes signatures
- URP renderer asset — add both features

**Reduce-motion**: when `ViewerEvent.ReduceMotionApplied == true`, skip impact frame; reduce screen-tone strength to 0. Adapter-specific feature-toggle per ADR-0008 §"Reduce-motion adapter-awareness".

**Dependencies:** Slices 3 + 4.

**Acceptance criteria:** Flash visible ~5 ticks post-goal; screen-tone visible during aftermath-freeze; both suppressed on reduce-motion path; `fw shader-audit` clean.

**Required agents:** `unity-specialist` + `engine-programmer` HLSL review + pr-review-toolkit triple before commit.

**Verification path:** L1 (compile + shader-audit) + L2 multi-state screenshot.

**LoC estimate:** ~380 LoC.

---

### Slice 6 — UI Toolkit overlay: scoreboard + commentary line + signature title-card

**What gets demonstrated:** Scoreboard always on screen; commentary line pushes in on `ViewerEvent`; signature title-card appears on signature execution. Anime presentation budget items 3 + 7 + 8.

**Files to create:**
- `unity-project/Assets/Viewer/Adapters/Dots/UI/DotsOverlay.uxml` — scoreboard (top), commentary feed (bottom-left), signature title-card (centre-bottom)
- `unity-project/Assets/Viewer/Adapters/Dots/UI/DotsOverlay.uss` — Anton (title-card), JetBrains Mono (scoreboard digits), Rajdhani (commentary)
- `unity-project/Assets/Viewer/Adapters/Dots/OverlayController.cs` — drives `UIDocument`; `SetScore`, `SetMinute`, `PushCommentary`, `ShowTitleCard`
- `unity-project/Assets/Viewer/Adapters/Dots/UI/Fonts/` — Anton / JetBrains Mono / Rajdhani Font Assets (SIL OFL 1.1)
- `unity-project/Assets/Viewer/Adapters/Dots/CommentaryTemplates.cs` — Phase-3 hand-authored pool of 5 strings per shot type (15 total)

**Vocabulary contract:** all strings pass `design/ui-vocabulary.md` Category-A lint. Football-native British register only. No capitalized state nouns. Examples: `"He strikes!"` / `"Gets onto the end of it."` / `"It's in. The home side pull ahead."`

**Dependencies:** Slices 3 + 4 (needs `ActiveViewerEvent.SignatureMetadata`).

**Acceptance criteria:** Scoreboard updates correctly; minute increments; commentary visible per event; title-card on signature events with football-vocabulary display name; zero banned terms; overlay <10% pitch occlusion.

**Required agents:** `unity-ui-specialist` + `narrative-director` reviews the 15 templates pre-commit.

**Verification path:** L1 + L2 multi-screenshot + L3 typography eye-test.

**LoC estimate:** ~420 LoC. May warrant 6a (panels + controller) + 6b (templates + font wiring) split if practice exceeds 500.

---

### Slice 7 — Identity cues, selection ring, motion lines, observer rubric pass

**What gets demonstrated:** 3-minute clip passes the solo-dev Month-3 observer rubric. Selection ring on focal player during `PlayerIsolation` shots; signature execution fires radial motion-line burst.

**Files to create:**
- `unity-project/Assets/Viewer/Adapters/Dots/SelectionRing.cs`
- `unity-project/Assets/Viewer/Adapters/Dots/MotionLineEmitter.cs`
- `unity-project/Assets/Viewer/Adapters/Dots/Sprites/motion_line.png` (2×16 white)
- `unity-project/Assets/Viewer/Adapters/Dots/Sprites/selection_ring.png` (64×64 outline)

**Files to modify:**
- `DotsAdapterRoot.cs` — full `PresentShot` routing
- `DotPool.cs` — `IndexForFocalSubject(string)` resolution

**Camera rhythm 3-tier:** `CadenceTier` { Calm, Standard, Tension } driven by `StakesNormalized`. `ShotCamera.TransitionTicks` per tier (24 / 12 / 6).

**Pressure indicator:** USS `background-color` Lerp on scoreboard panel, transparent → faint amber, proportional to `StakesNormalized`.

**Dependencies:** All prior slices.

**Acceptance criteria (observer rubric):** Solo-dev watches 3-minute clip; can identify which team is winning, attacking ends, notable actions, dramatic moments. Selection ring on focal players; motion-line burst on signatures fades within 20 ticks; perceptible camera-rhythm shift across stakes; pressure-indicator tint visible. All 642 + 21 tests still green; `fw shader-audit` clean; zero banned terms.

**Required agents:** `unity-specialist` + `art-director` final visual review + `narrative-director` final commentary pass + pr-review-toolkit triple.

**Verification path:** L1 + L2 multi-screenshot + L3 3-minute Unity Recorder clip + 6-task rubric self-assessment.

**LoC estimate:** ~380 LoC.

---

**Total adapter LoC across all 7 slices:** ~1,450–1,700 LoC.

---

## C. Key Contract Decisions (SPEC Decisions-Log Candidates)

**Decision 1** — `IShotPresentationAdapter` is 4-method (`Initialize` / `PresentShot` / `Teardown` / `AdapterId`) and lives in `Viewer.Core` (not `Viewer.Contracts`). Rationale: the interface needs `PitchView` which uses `UnityEngine.Vector3`. Future 3D adapter (ADR-0010) implements the same interface. Tradeoff: a future headless-test adapter would need a `FixedToWorld` seam split.

**Decision 2** — Camera is hand-rolled orthographic; **no Cinemachine** for Phase-3 dots adapter. Cinemachine 3.1.6 is installed but introducing it for a locked-orthographic top-down view is premature complexity. The 3D adapter is the natural Cinemachine consumer. Avoids `CinemachineBrain` entangling with the deterministic tick loop. Tradeoff: future split-screen rigs would need extending `ShotCamera`.

**Decision 3** — `tactical-wide` → `diagonal-attack-lane` transition is **adapter-local heuristic**, not a bridge-emitted `ViewerEvent`. Adapter enriches to `diagonal-attack-lane` when ball Z-velocity exceeds threshold. Phase-4: when `KeyEventKind.ThroughBallLaunched` lands, the bridge emits the event and the heuristic retires.

**Decision 4** — `EventBridge.Derive` called **once-per-tick (re-derive, not incremental)**. O(n) in KeyEvent count. Phase-3 KeyEvent counts are <10 per 90-min match — negligible. Phase-4 should add a `DeriveFrom(state, seed, fromKeyEventIndex, bool reduceMotion)` overload backward-compatible with current callers when KeyEvent counts grow (fouls / cards / subs).

**Decision 5** — Player dot identity uses **`RoleFamily` + `TeamSide` colour tinting**, NOT jersey-number rendering at Phase-3. 23 TextMeshPro instances at 60fps is an allocation budget concern; jersey numbers unreadable at tactical-wide zoom. Phase-4 `player-isolation` shot adds World Space jersey label scoped to focal player only.

---

## D. Open Questions / Risks

**Q1** — Does the new `PitchView.FixedToWorld(Vector3Fixed) → UnityEngine.Vector3` signature compile across the asmdef boundary? The `Viewer.Core` asmdef has `overrideReferences: true` + `precompiledReferences: ["FinalWhistle.MatchSim.dll"]`. Slice 1 L1 verifies this; if it fails, the return type may need `(float x, float y, float z)` tuple and the caller composes `Vector3`.

**Q2** — `diagonal-attack-lane` `ShotTypeDefinition` is NOT registered in `ShotTypeCatalog`. Either add it to the catalog (Phase-3 catalog change in `Viewer.Contracts`), or store framing parameters directly in the `ShotTypeSO` asset without catalog lookup. Decision needed before Slice 4.

**Q3** — `FocalSubject` field uses `"viewer.focal:home.06"` format. `DotPool.IndexForFocalSubject` resolves to dot index. Confirm jersey numbers on `PlayerState` are 1–11 per side in the Phase-3 smoke fixture, not 0-based.

**Q4** — **UnityMCP is currently DISCONNECTED.** All L1/L2 verifications require it. User must restart (`Window → MCP for Unity → Start Server`) before each slice's L1.

---

## E. Subagent Invocation Cheatsheet

**`lead-programmer` (Slice 1 — interface shape review)**
Include: `unity-project/Assets/Viewer/Contracts/ViewerEvent.cs`, `design/adrs/ADR-0008-shot-presentation-contract.md`, the proposed `IShotPresentationAdapter` shape. Question: "Does this 4-method interface leave the 3D adapter (ADR-0010) able to bolt on without API churn? Is `PitchView` in `Viewer.Core` the right home or should it be in `Viewer.Contracts` with a FixedToWorld pure-C# method?"

**`feature-dev:code-architect` (Slice 1 — pre-implementation blueprint pass)**
Include: this blueprint + `Viewer.Core.asmdef` + `Viewer.Contracts.asmdef`. Constraint: "Future-proof for both dots + cel-shaded 3D adapters per ADR-0008/0009."

**`unity-specialist` (Slices 2, 3, 4, 7)**
Include: `Viewer.Adapters.Dots.asmdef` + `Packages/manifest.json` + slice description. Constraint: "no Cinemachine for Phase-3; hand-rolled orthographic camera; no `_Time` shader globals."

**`unity-ui-specialist` (Slice 6)**
Include: `design/ui-vocabulary.md` Category-A bans + `design/semantic-cinema.md` §Q4 typography stack + `design/anime-presentation-budget.md` §"Signature title-card". Constraint: "scoreboard uses Rajdhani SemiBold for digits; Anton reserved for goal splash / title-card only; no UGUI."

**`engine-programmer` (Slice 5 — HLSL URP passes)**
Include: `design/anime-presentation-budget.md` §"Impact frames" + §"Screen-tone", `.claude/rules/Scripts/Viewer/RULES.md` (`_Time` ban + `fw shader-audit`). Constraint: "all time inputs must be explicit int uniforms set by the adapter from ElapsedTicks; fullscreen pass must not stall the render thread at 60fps."

**`gameplay-programmer` (Slice 3 — tick-loop review)**
Include: `MatchSim/Sim/MatchSimulationRunner.cs` + `MatchSimulationState.cs` + Slice 3 description. Question: "Is `FixedUpdate` at `Time.fixedDeltaTime = 1/60f` calling `RunTicks(ticks: 1)` the correct wiring? Can `SignatureCooldownState` on `MatchSimulationState` survive chunked 1-tick `RunTicks` calls?" (We know it can per Codex round-9; this is double-confirmation in the live Editor context.)

**`narrative-director` (Slice 6 — commentary template review)**
Include: `design/ui-vocabulary.md` (full file) + `design/semantic-cinema.md` §"7 shot types" + the 15 drafted strings. Question: "Are these strings lint-clean, football-native British register, correctly timed?"

**`art-director` (Slices 2 + 7 — palette + final visual review)**
Include: `design/anime-presentation-budget.md` (full) + ADR-0009 §polish bar + UnityMCP screenshots. Slice 2 question: "Does the role-family colour palette give instant home/away + role discrimination at tactical-wide zoom?" Slice 7 question: "Does the 3-minute clip meet the Month-3 observer rubric — drama, momentum, identity readable without a design doc?"

**`pr-review-toolkit` (all slices ≥100 LoC pre-commit)**
Three sub-agents in parallel: `:silent-failure-hunter` + `:type-design-analyzer` + `feature-dev:code-reviewer`. Include: the slice's diff.

---

*Authored 2026-04-30 by `feature-dev:code-architect`. Subsequent /next iterations should read this document first before spawning slice-specific agents.*

---

## In-flight handoff — Slice 1 (2026-04-30, mid-session)

**Status:** Slice 1 code authored + reviewed; pre-commit verification BLOCKED on UnityMCP tool-catalog refresh; user opted to restart the Claude session for a fresh MCP catalog.

**Files on disk (uncommitted):**
- `unity-project/Assets/Viewer/Core/IShotPresentationAdapter.cs` (NEW; ~50 LoC)
- `unity-project/Assets/Viewer/Core/PitchView.cs` (NEW; ~115 LoC after review fixes)
- `unity-project/Assets/Viewer/Core/ActiveViewerEvent.cs` (NEW; ~85 LoC after review fixes)
- `unity-project/Assets/Viewer/Tests/EditMode/PitchViewTests.cs` (NEW; ~245 LoC; covers PitchView constructor invariants + FixedToWorld precision (incl. fractional-corner + non-unity-scale precision pins) + ActiveViewerEvent projection)
- `unity-project/Assets/Viewer/Tests/EditMode/Viewer.Tests.EditMode.asmdef` (MODIFIED; added `FinalWhistle.Viewer.Core` reference)
- `.claude/rules/Scripts/Viewer/RULES.md` (MODIFIED; doc-drift fix — `PitchView` + `ActiveViewerEvent` enumerated under `Viewer.Core`, not `Viewer.Contracts`, with the UnityEngine.Vector3 rationale)
- `SPEC.md` (MODIFIED; observer-pool scrapped; dots-adapter Decisions 1-4 logged in append-only decisions section)
- `STATUS.md` (MODIFIED; semantic-slice-complete + dots-adapter-next paragraphs)
- `docs/plans/dots-adapter-blueprint.md` (NEW; this file)

**pr-review-toolkit triple already ran** (silent-failure-hunter / type-design-analyzer / feature-dev:code-reviewer in parallel). Findings applied:
- P1 (code-reviewer): `PitchView` validation reordered — finite checks before positivity (NaN was passing positivity check via IEEE 754 false comparison)
- P1 (code-reviewer): `PitchView.FixedToWorld` keeps multiplication in double precision (was casting to float before scaling)
- P2 (code-reviewer + silent-failure-hunter): added `FixedToWorld_FractionalCorner_HoldsSubMillimetreAccuracy` + `FixedToWorld_NonUnityScaleAtCornerMagnitude_PreservesAccuracy` tests that actually exercise the float-cliff guard
- P2 (silent-failure-hunter): added `<exception cref>` XML doc on `ActiveViewerEvent` constructor for `KeyNotFoundException` propagation
- P2 (code-reviewer): `Viewer/RULES.md` updated to reflect `PitchView` + `ActiveViewerEvent` live in `Viewer.Core` (was stale, said Contracts)
- P2/P3 (type-design-analyzer): `IsInitialized` lifecycle guard + `StakesFloat` clamp assert deferred — Phase-3 acceptable (doc-only is current discipline; would add Slice-2+ if observed drift)

**MatchSim regression baseline:** `scripts/fw verify` ran clean — 642/642 tests passing, banned-terms clean, shader-audit clean (no viewer-adapter shaders yet), plugin-drop matches publish. **Pinned 60-tick `MatchCanonicalState` hash unchanged** (`sha256:7e851976f6a5eea467797e90400ca030c6ab955e21c2f92466cffa00c880f50e`) — slice 1 doesn't touch MatchSim source.

**What the next Claude session needs to do:**

1. Read this blueprint file (you're reading it now).
2. Verify UnityMCP tool surface is back via `ToolSearch` query "unity" — should now return `mcp__UnityMCP__refresh_unity` + `read_console` + `run_tests` etc.
3. Run `mcp__UnityMCP__refresh_unity` with `force=request` to trigger asset-database import + script compilation.
4. Run `mcp__UnityMCP__read_console` and check for compile errors. Expected: zero errors, zero warnings on `Viewer.Core` + `Viewer.Tests.EditMode`.
5. Run `mcp__UnityMCP__run_tests` filter on `FinalWhistle.Viewer.Tests.EditMode` (EditMode mode). Expected: previous 21 EditMode tests still pass + the new ~14 PitchView / ActiveViewerEvent tests pass too. Total target: ~35 EditMode tests green.
6. If any compile errors, fix them before /done. Most likely failure modes: cref ambiguity (the architect uses `<see cref>` syntax with potentially-ambiguous overloads — fix by switching to `<c>` formatting); test-asmdef reference resolution (verify the new `FinalWhistle.Viewer.Core` reference compiles).
7. Once L1 + L2 (compile + tests) green, invoke `/done`. The done skill owns the verification stack — it will mark SPEC line 149 partially with a slice-1-shipped note, append CHANGELOG, bump STATUS.
8. Next /next picks up Slice 2 (static pitch quad + DotPool) per Section B above.

**One open question to resolve at Slice 4 (not blocking Slice 1):**
- Q2 from Section D: `diagonal-attack-lane` is NOT in `ShotTypeCatalog` yet (only `tactical-wide` / `player-isolation` / `pass-shot-impact` / `aftermath-freeze` + the player-isolation reduce-motion variant are registered). Slice 4 will need to either (a) add `diagonal-attack-lane` to the catalog with new `ShotCategory.DiagonalAttackLane = 2` enum entry usage + new shot ID constant, or (b) store framing parameters directly in the `ShotTypeSO` asset without catalog lookup. Lean toward (a) for consistency with how the other shots are structured. Decide at Slice 4 start.

*Handoff written 2026-04-30 mid-session by Claude. Next session: pick up at step 1 above.*

---
description: ADR-0002 — Viewer rendering pipeline + URP custom-pass ordering. Formalizes the semantic-cinema rendering stack (screen-tone, impact-frame flash, motion-line trails, panel/text composition) into an ordered URP render-feature contract.
---

# ADR-0002: Viewer rendering pipeline + URP custom-pass ordering

**Accepted** — 2026-04-24. Self-review tightening: `fw shader-audit` Phase-3 deliverable promoted to explicit SPEC task. Knowledge Risk MEDIUM gate (URP Render Graph verification at Phase-3 Week 1) remains explicit in this ADR body — ADR is Accepted with that verification gate baked in, not after it.

## Status

**Accepted** (see date above).

## Date

2026-04-24

## Last Verified

2026-04-24

## Decision Makers

osagberg (project owner), GPT-5.5 (design partner), Claude (workhorse author).

---

## Summary

Author the 2D semantic-cinema viewer as a URP pipeline with four composable render features — screen-tone and impact-frame flash as fullscreen HLSL passes, motion-line trails as per-sprite meshes, and panel/text composition as UI Toolkit overlay (with custom-mesh fallback). Pass ordering is fixed and documented so compositing never depends on accidental render-queue sort.

## Engine Compatibility

| Field | Value |
|---|---|
| Engine | Unity 6 LTS (exact patch pinned at Phase 3 kickoff) + URP 17+ |
| Domain | Rendering | <!-- ui-lint:allow term="domain" reason="ADR template canonical field name for engine-compat area" reviewer="osagberg" -->
| Knowledge Risk | **MEDIUM** — URP Render Graph API is newer (Unity 6 default); custom render features still work but Render Graph recompilation differs from pre-6 SRP. Must verify against pinned Unity 6 LTS at Phase 3 Week 1. |
| References Consulted | `design/semantic-cinema.md` 2026-04-24 resolution, ADR-0001 (ShotTypeSO), Unity URP docs via context7 (pull at Phase 3 implementation) |
| Post-Cutoff APIs Used | URP Render Graph custom pass registration (Unity 6 LTS default — may differ in minor API surface from pre-cutoff docs) |
| Verification Required | Phase 3 Week 1: spike the two fullscreen HLSL passes against Unity 6 LTS URP 17+ on the actual pinned version. If Render Graph compilation ordering is incompatible with the spec below, supersede this ADR before Week 2 authoring work. |

## Dependencies

| Field | Value |
|---|---|
| Depends On | ADR-0001 (ShotTypeSO — source of per-shot modulation strength + reduce-motion variant) |
| Enables | Phase 3 Week 2-3 2D viewer prototype; ADR-0004 (MemoryEvent) — stakes modulation consumes memory-hit salience |
| Blocks | Phase 3 viewer rendering; Month-3 gate artifact (legibility test depends on rendering reading as "manga-broadcast," not "default URP 2D sprites") |

---

## Context

### Problem Statement

`design/semantic-cinema.md` locks the rendering stack shape (screen-tone HLSL pass, impact-frame flash HLSL pass, per-player motion-line trails, UI Toolkit overlay with custom-mesh fallback). That resolution specified *what* renders each element. This ADR specifies *how they compose* — pass ordering, render-feature registration, per-shot modulation wiring, and reduce-motion path — so Phase 3 can implement without inventing the pipeline architecture.

### Current State

No Unity project exists. No render features authored.

### Constraints

- **Unity 6 LTS + URP 17+** — pinned at Phase 3.
- **2D-first** — no 3D camera or deferred-rendering complexity. Cel-shading is out; flat-shaded manga-broadcast style is in.
- **Deterministic visual reproducibility** — for Month-3 gate, the same replay seed must produce the same rendered sequence on the same hardware. Shader-level dithering, time-based noise, etc. must seed from the deterministic viewer seed (ADR-0001 §deterministic-selection), not wall-clock.
- **Performance** — match-day rendering must hit frame budget on mid-range 2026 hardware (Phase 7 target). At Phase 3 prototype, four render features can't blow the per-frame budget.
- **Accessibility** — reduce-motion variant per shot (ADR-0001) must fully disable impact flash + motion-line trails; screen-tone reduces to a static overlay, not time-modulated.

### Requirements

- **Functional:** 4 render features registered in a single URP Renderer asset, composable per shot, reduce-motion-aware, deterministic-seeded.
- **Performance:** ≤4 fullscreen draw calls per frame for base pipeline (screen-tone + impact-flash + UI overlay + sprite pass); per-sprite trail meshes use batched mesh renderers (≤1 extra draw call per on-pitch player visible in-frame).
- **Maintainability:** each render feature is authored as an independent `ScriptableRendererFeature` asset, toggleable for debug / A-B comparison.

---

## Decision

### Pipeline shape

Single URP Renderer Asset (`FinalWhistleViewer2D`). Four `ScriptableRendererFeature` entries, registered in a **fixed, documented order** so the composite never depends on accidental queue sorting.

### Pass ordering (fixed, documented, enforced)

Rendered per-frame in this exact order:

| # | Pass | Injection point | Scope | Draw calls |
|---|---|---|---|---|
| 1 | **Scene sprite pass** | `BeforeRenderingOpaques` → `AfterRenderingTransparents` (URP default 2D sprite path) | scene-local | one per sprite batch |
| 2 | **Motion-line trail pass** | `AfterRenderingTransparents` (before fullscreen effects) | agent-local meshes | one per visible player with non-zero velocity |
| 3 | **Screen-tone fullscreen pass** | `AfterRenderingPostProcessing` (stakes-modulated intensity) | fullscreen HLSL | 1 |
| 4 | **Impact-frame flash fullscreen pass** | `AfterRenderingPostProcessing` → `AfterRendering` (event-triggered, transient) | fullscreen HLSL | 1 (only when event active) |
| 5 | **UI Toolkit overlay** | URP overlay camera stack (above everything else) | UI layer | batched per UI draw |

**Why this order:** motion lines must render OVER sprite pass (so trails overlay the player) but UNDER screen-tone (so tone applies uniformly). Impact-flash lands AFTER screen-tone so it overrides tone during the white-flash beat. UI overlay sits above all of it so HUD text never gets dithered by screen-tone.

### Architecture Sketch

```
┌───────────────────────┐
│  URP Renderer Asset   │
│  FinalWhistleViewer2D │
└─────────┬─────────────┘
          │
          ├─ (1) Scene Sprite Pass            [default 2D]
          │
          ├─ (2) MotionLineTrailFeature       [per-player mesh; reduce-motion OFF it]
          │
          ├─ (3) ScreenToneFeature            [fullscreen HLSL; stakes-modulated]
          │       └─ reads active ShotTypeSO.Modulation.stakes
          │       └─ reads viewer deterministic seed (no wall-clock)
          │
          ├─ (4) ImpactFrameFlashFeature      [fullscreen HLSL; event-triggered]
          │       └─ reduce-motion: disables entirely
          │       └─ triggered via ShotSelector event (ADR-0001)
          │
          └─ (5) UI Toolkit overlay camera    [panel splits + text]
                  └─ fallback: UIMeshOverlayFeature if UIT masks misbehave
```

### Key Interfaces

```csharp
// Each feature is a ScriptableRendererFeature asset, toggleable in-editor.

public sealed class ScreenToneFeature : ScriptableRendererFeature
{
    [SerializeField] Shader _toneShader;       // custom HLSL
    [SerializeField] Material _sharedMaterial; // one instance, re-used per frame

    public override void AddRenderPasses(ScriptableRenderer renderer, ref RenderingData data)
    {
        var intensity = ViewerState.Current.StakesModulatedToneIntensity;   // from active ShotTypeSO
        if (ViewerState.Current.ReduceMotion)
            intensity = ViewerState.Current.ReduceMotionStaticToneIntensity;
        _pass.Setup(_sharedMaterial, intensity, ViewerState.Current.DeterministicSeed);
        renderer.EnqueuePass(_pass);
    }
}

public sealed class ImpactFrameFlashFeature : ScriptableRendererFeature
{
    // Similar shape. Pass.Setup no-ops unless ViewerState flags an active impact event.
    // Reduce-motion: feature.SetActive(false) at scene-load; never registers.
}

public sealed class MotionLineTrailFeature : ScriptableRendererFeature
{
    // Draws trail meshes for on-pitch players whose |velocity| > threshold.
    // Reduce-motion: feature.SetActive(false); all trails hidden.
}

// Central state consumed by passes — updated per shot change by ShotSelector (ADR-0001).
public static class ViewerState
{
    public static ViewerFrameState Current;   // struct, replaced per frame
}

public readonly struct ViewerFrameState
{
    public float StakesModulatedToneIntensity;
    public float ReduceMotionStaticToneIntensity;
    public bool  ReduceMotion;
    public ulong DeterministicSeed;            // from replay seed; feeds shader noise
    public ShotTypeSO ActiveShot;
}
```

### Deterministic rendering contract

Cross-platform reproducibility for Month-3 gate replays. Same rules as ADR-0001 §deterministic-selection, extended to shader inputs:

1. **Shader noise / dither uses the deterministic viewer seed** passed as uniform. NO `_Time`-based noise. NO per-instance random that isn't seeded.
2. **Floating-point shader math is accepted as non-deterministic across GPUs** — what must be deterministic is the **choice of which pass runs with which parameters**, not the exact pixel values. Replay artifact stores the pass-activation log (feature enabled/disabled + parameters), not rendered frames. Visual-regression capture at Phase 3 Week 2 uses keyframe hashes with tolerance thresholds, not bit-identical pixel compare.
3. **Per-frame pass selection is replay-seeded** — same seed + same ShotTypeSO sequence = same pass activation log, verifiable in CI without a GPU.

### Implementation Guidelines

- **Asmdef boundary:** `FinalWhistle.Viewer.Rendering` (custom render features + passes); no dependency from `MatchSim.csproj`.
- **One `RendererAsset`** (`FinalWhistleViewer2D`) — debug/A-B variants live as additional RendererAsset files, switchable via the URP asset settings for isolated feature testing.
- **Shader code lives at** `unity-project/Assets/_Project/Viewer/Rendering/Shaders/` with `.hlsl` includes shared across passes (noise utilities, seed-based dither).
- **No `_Time` uniforms in any FinalWhistle shader** — enforced by a Phase-3 `fw shader-audit` tool (tracked as explicit SPEC task under Phase 3) that greps viewer shaders for `_Time` references. Gate runs inside `fw verify` umbrella once authored so the discipline enters Tier-A CI immediately.
- **Reduce-motion is a scene-load-time flag** that disables features entirely at feature-registration time — not a per-frame early-exit, to avoid shader compilation variants unused at runtime.
- **UI Toolkit overlay + mesh-fallback:** ship UIT overlay as default; if a specific panel-split effect requires `UIMeshOverlayFeature` fallback (e.g., hatched split that UIT masks can't do), author the fallback per-panel, NOT pipeline-global.

---

## Alternatives Considered

### Alternative 1: Everything in UI Toolkit (no custom render features)

- **Description** — Author screen-tone, impact flash, motion lines, panel splits all as UI Toolkit overlay elements on top of the default 2D pipeline.
- **Pros** — Simpler asset model; one overlay, no shader authoring.
- **Cons** — UI Toolkit is not built for velocity-driven per-sprite effects (motion lines trail a moving player). UIT shader access is limited; screen-tone-as-UI-overlay doesn't compose cleanly with sprite layer (z-order inversion issues). Impact-flash-as-UIT-animation has timing precision issues vs a render-pass event.
- **Rejected because** — UIT's strengths (HUD, menus, text) don't extend to fullscreen post-effects or agent-local trails. `design/semantic-cinema.md` 2026-04-24 resolution already made this call.

### Alternative 2: Single mega-feature with all effects

- **Description** — One `ScriptableRendererFeature` that encapsulates all four effects with internal flags.
- **Pros** — Single feature to register.
- **Cons** — Couples unrelated effects (screen-tone has nothing to do with motion lines). Debug / A-B toggling becomes harder. Violates single-responsibility at the feature level.
- **Rejected because** — the feature-per-effect shape is the standard URP pattern and makes A-B debugging trivial at Phase 3 prototype.

### Alternative 3: Post-processing stack via Volume framework

- **Description** — Author effects as URP Volume overrides (like Bloom, Color Grading).
- **Pros** — Native framework for fullscreen post-effects.
- **Cons** — Volume framework is designed for zone-based environmental effects (player walks into a Volume, it blends in). Our effects are shot-type-driven, not zone-driven — wrong abstraction. Volume blending between shots would fight the hard-cut semantic-cinema grammar.
- **Rejected because** — abstraction mismatch. The shot is the primitive, not the zone.

---

## Consequences

### Positive

- Composable, toggleable, individually-debuggable render features.
- Clear ordering contract prevents compositing regressions ("why does the screen-tone suddenly appear above the HUD?").
- Reduce-motion path is structural (features disabled at registration), not runtime-branching — cleaner accessibility posture.
- Replay-correctness via pass-activation log keeps determinism verifiable without GPU pixel-compare.

### Negative (Accepted Tradeoffs)

- Four custom render features is meaningful Unity scaffolding cost at Phase 3 (estimated 3-5 days authoring + verification against Unity 6 LTS URP 17+).
- Shader authoring burden (two custom HLSL passes) — first-time cost for solo dev; pays back through the rest of the viewer work.
- **Knowledge Risk MEDIUM** on Render Graph: if Unity 6 LTS URP 17+ changes custom-pass registration semantics from what this ADR assumes, we supersede before Phase 3 Week 2.

### Neutral

- Asset count: +1 Renderer Asset + 4 ScriptableRendererFeature assets + 2-3 HLSL shader files + 1 Material per fullscreen pass. Manageable.
- UI Toolkit overlay stays UIT-first; mesh-fallback is per-panel, not pipeline-wide — keeps the default path simple.

---

## Performance Implications

| Metric | Before | Expected After | Budget |
|---|---|---|---|
| Draw calls per frame (base scene) | n/a | ~4 fullscreen + N sprite batches + M player trails + 1-2 UI batches | ≤20 total draw calls for Month-3 slice |
| Shader compilation variants | n/a | 4 features × 1-2 variants each (reduce-motion path pre-baked) | ≤10 variants total |
| CPU frame time (render-feature overhead) | n/a | <0.5ms for pass enqueuing + setup | 16ms/frame @ 60fps; render setup is tiny slice |
| GPU frame time (Month-3 slice) | n/a | Fullscreen passes are cheap on 2026 mid-range hardware | TBD at Phase 7 perf pass; Month-3 slice just needs to render |

Serious perf tuning is Phase 7. Phase 3 Month-3 slice just needs to not drop frames on solo-dev's Apple Silicon Mac during the gate test.

---

## GDD Requirements Addressed

| GDD | System | Requirement | How This ADR Satisfies It |
|---|---|---|---|
| `design/semantic-cinema.md` | Rendering stack | Screen-tone + impact-frame + motion-lines + UIT overlay | Four features, ordered, composable |
| `design/semantic-cinema.md` | Stakes modulation | Tone intensity modulates with stakes | `ViewerFrameState.StakesModulatedToneIntensity` pulled from active `ShotTypeSO` |
| `design/semantic-cinema.md` | Reduce-motion accessibility | Impact flashes + motion lines disabled; tone static | Feature-registration-level disable (structural, not runtime branch) |
| ADR-0001 | ShotTypeSO consumption | Rendering reads per-shot framing + modulation + reduce-motion variant | `ViewerState.Current.ActiveShot` refreshed on shot change |
| `design/production-pipeline.md` | Replay determinism | Same seed + same packs = same rendered sequence | Pass-activation log stored in replay artifact; shader uniforms seeded from deterministic viewer seed |

---

## Migration Plan

Not applicable — greenfield.

**Rollback:** if Unity 6 LTS URP 17+ at Phase 3 Week 1 proves incompatible with the ordering / Render Graph assumptions here, supersede via a replacement ADR citing this one. Phase-3 viewer authoring doesn't start until Week 2, so rollback cost is bounded to Week-1 spike time.

---

## Validation Criteria

- [ ] Phase 3 Week 1: spike fullscreen HLSL pass registration against pinned Unity 6 LTS URP 17+. Verify `AfterRenderingPostProcessing` injection point behaves per this ADR's ordering contract.
- [ ] Phase 3 Week 2: 4 features registered in `FinalWhistleViewer2D` renderer; each individually toggleable via debug inspector.
- [ ] Phase 3 Week 2: stakes-modulated screen-tone visibly varies between `tactical-wide` (low stakes) and `aftermath-freeze` (high stakes) shots in the prototype.
- [ ] Phase 3 Week 3: impact-frame flash fires on the goal-scored event during a Month-3 slice recording.
- [ ] Phase 3 Week 3: motion-line trails visible on running players; disappear instantly when reduce-motion is enabled.
- [ ] Phase 3 Week 3: UI Toolkit overlay (scoreline per ADR-0001's font policy) renders above all other features correctly.
- [ ] Phase 3 Week 4 (Month-3 gate): cold observers describe the rendered match as "manga-broadcast-looking," not "default Unity 2D sprites" — subjective but decisive for gate outcome.
- [ ] Replay determinism: pass-activation log for canonical replay seeds matches across Win/Mac/Linux CI matrix (pixel-compare NOT required; pass-log compare is).
- [ ] Shader audit: no `_Time` references in any `FinalWhistle.Viewer.Rendering` shader (Phase-3 `fw shader-audit` deliverable).

---

## Related

- Depends on: ADR-0001 (ShotTypeSO — modulation strength + reduce-motion variant source).
- Enables: Phase 3 viewer prototype; ADR-0004 (MemoryEvent — memory salience feeds stakes modulation).
- Cross-refs: `design/semantic-cinema.md` 2026-04-24 resolution (source), `design/production-pipeline.md` (replay determinism), `design/accessibility.md` (Phase-2 deliverable; reduce-motion settings source).
- Code (once implemented): `unity-project/Assets/_Project/Viewer/Rendering/` (paths tentative, finalize Phase 3 bootstrap).

---
description: 3D cel-shaded shipping-visual pipeline placeholder. Tooling-agnostic at the contract level; specific tool candidates listed for Phase-5 spike-gate evaluation. Spike-gate criteria + animation contract + licensing requirements + alternatives + open questions all owed before any 3D code lands.
last_verified: 2026-04-26
status: Phase 2 placeholder authored 2026-04-26 per visual-target supersession. Full pipeline spec owed at Phase 5 immediately before the production-feasibility spike. ADR-0010 (3D shipping render adapter) is NOT pre-authored; it lands only if the spike succeeds. If the spike fails, this doc captures the alternatives that become live (dots-only EA + post-EA 3D R&D / 3D cut-ins only / dots-match-with-3D-replay-moments).
---

# 3D Pipeline — placeholder + spike-gate contract

## Purpose

Final Whistle's shipping visual identity is candidate-cel-shaded-3D, gated on a Phase-5/6 production-feasibility spike per the 2026-04-26 visual-target supersession decisions-log entry. This doc is the placeholder where pipeline specification lands when the spike approaches. It also captures, NOW, the spike-gate criteria + animation contract surface + licensing-audit requirements + fallback alternatives — so when Phase 5 arrives, the Go/No-Go decision has criteria pre-locked and the architecture has known rollback paths.

This doc is **renderer-adapter-specific**. The renderer-agnostic presentation contract that ANY adapter (dots / 3D / future variants) consumes is in ADR-0008 `ShotPresentationContract`. This doc concerns the 3D adapter only.

## Locked decisions (per 2026-04-26 visual-target supersession)

- **3D is candidate, not committed.** No public "3D coming in 1.0" promise. EA can ship dots if the spike fails and dots hit polish bar; EA delays if dots also fall short.
- **Vendor-agnostic core, tooling-specific in this doc only.** Core docs (PROJECT_CONTEXT / CLAUDE / TECH_APPROACH / ADRs) speak of "3D-asset generator," "AI-assisted animation tool," "retargeting tool." Tool names appear in this doc and only this doc; tooling can change without ADR churn.
- **Renderer-free MatchSim is preserved.** Sim emits `ShotPresentationContract` events per ADR-0008. Gameplay never depends on 3D-only information. The dots adapter (ADR-0009) and the 3D adapter (ADR-0010, conditional) consume the same contract.
- **Licensing as first-class gate.** AI-content disclosure manifest extends with generator / plan-license-tier / prompt-source-refs / human-edit-steps / commercial-rights-proof / generated-asset-hash. New `FW-VAL-D-011` enforces. All 3D-tooling commercial-licenses verified BEFORE the spike begins (Phase-5 SPEC task).

## The Phase-5/6 production-feasibility spike

A vertical slice that proves the pipeline produces a usable football scene end-to-end. Below the bar = 3D not viable for shipping; alternatives engaged.

### Mandatory deliverables (all required for "spike green")

1. **Multi-player scene.** ≥6-10 visible players in one rendered Unity scene. Not one solo character.
2. **Two distinct kits.** Home + away kit variants on the same body type, materially distinguishable at gameplay camera distance.
3. **One body-type variant.** ≥1 alternate body type beyond the default (e.g., heavier centre-back vs lean winger). Proves rig retargeting works.
4. **Locomotion.** Run / walk / jog cycles. Smooth, readable, cel-shaded with outlines.
5. **One duel.** Two players contesting the ball — football body language, plausible animation blending. Tests animation-state composition + ball-contact markers.
6. **One signature with ball-contact markers.** Pick one of the 24 signatures from `design/signatures.md`, author its animation end-to-end, prove ball-contact markers fire on the right frame, prove integration with sim-side `MemoryEvent` emission.
7. **Cel shader + outline.** URP Shader Graph (or HLSL) cel-shader pass + outline pass (post-process or vertex-extrusion) producing the anime-football aesthetic. Visually defensible at gameplay camera distance.
8. **Unity import + LOD.** Asset pipeline imports cleanly; LODs configured; scene runs in Unity Editor + a Player build.
9. **Target FPS on minimum hardware.** Currently provisional minimum: integrated-graphics laptop circa 2022, 1080p, 60 FPS sustained at gameplay camera distance with the multi-player scene rendering. Concrete minimum hardware spec locks at Phase-5 alongside the spike.
10. **Repeatable export/import.** The pipeline can produce N additional players from the same toolchain at the same quality without manual rework per player. Proves it scales beyond the spike.
11. **Commercial-rights manifest complete.** Every tool used in the spike has documented commercial-rights coverage (license tier verified, asset hashes recorded, prompts/source refs preserved). `FW-VAL-D-011` passes against the spike's content-pack manifest.

### Spike outcomes

- **Green (all 11 deliverables hit at quality):** Author ADR-0010 (3D shipping render adapter); land it through the standard ADR-review rhythm; promote 3D to shipping visual; full Phase-6/7 production scaling begins.
- **Yellow (8-10 deliverables; some quality compromises):** Decision point — extend spike for one cycle to close gaps OR commit to dots-EA + post-EA 3D-R&D path. No public 3D promise either way.
- **Red (≤7 deliverables OR critical pipeline failure):** Engage alternatives. Dots is the EA visual if it hits its own polish bar; 3D is post-EA R&D only.

## Animation contract surface

Owed: explicit rig + animation contract that any 3D adapter (Phase-5 spike or later) honors. Captured here because the contract is what makes the rendering renderer-adapter-clean. If this section grows past ~30 sub-points or starts containing version-locked decisions, promote to a dedicated ADR.

### To lock at Phase-5 spike preparation

- **Rig standard.** Bone hierarchy + naming convention + scale + orientation. Compatible with retargeting tools. Provisional candidate: humanoid-extended (Mecanim-compatible humanoid + extra finger / facial / cloth-attachment bones); lock at spike kickoff.
- **Clip format.** Animation clip type (Unity AnimationClip / Animancer / glTF), root-motion semantics, frame-rate baseline (30fps authored, sample-as-needed).
- **Event markers.** Generic markers any clip can carry: `step-impact` (foot-down for step audio), `ball-contact` (frame the boot meets the ball), `signature-trigger` (when the sim event should fire from this animation), `animation-end` (resolve next-state).
- **Ball-contact marker contract.** Critical for sim-viewer determinism. The animation declares the contact frame; the sim records the deterministic contact tick; the viewer interpolates between them. Spec MUST specify what happens when sim-determined contact tick and animation contact frame disagree (animation skip / re-time / sim re-emit?).
- **Retargeting rules.** Rules for adapting an animation authored on body-type-A to body-type-B. Constraint: visually plausible across all in-MVP body types without per-player re-authoring. Tooling: TBD at spike kickoff.
- **Fallback animations.** Every signature animation has a fallback locomotion-and-shoot generic animation that fires if the signature-specific clip is missing (e.g. mod-pack references a signature whose authored animation doesn't ship). Prevents content-pack-loadability blockers from rendering bugs.
- **24-signature animation budget.** Realistic per-signature authoring time + dependency on retargeting tool + Cascadeur-or-equivalent AI-assisted-animation feasibility. Phase-5 spike validates by authoring ONE signature end-to-end; Phase-6 production scales (or signals "20 signatures realistic at this quality, drop 4" — schema-bump to `design/signatures.md` if needed).

## Licensing requirements (first-class gate)

Per the 2026-04-26 supersession entry §(e). Every 3D-tooling decision faces these checks BEFORE inclusion in the spike or shipping pipeline:

| Field | What it captures |
|---|---|
| `generator` | Tool / service name + version producing the asset |
| `plan_license_tier` | Specific commercial-rights tier active (e.g. paid Pro plan vs free non-commercial) |
| `prompt_source_refs` | Inputs used (text prompts, reference images, human-authored sketches) — preserved for reproducibility audit |
| `human_edit_steps` | What human modifications applied to the generated output (rig adjustments, mesh cleanup, texture-paint passes) |
| `commercial_rights_proof` | Documented evidence the output is cleared for commercial Steam release at the plan tier |
| `generated_asset_hash` | SHA-256 of the as-imported asset, recorded in pack manifest for audit |

Asset-licensing-tracker (`steam-release/asset-licensing-tracker.csv`) extends with these columns at the next schema bump (Phase-5 SPEC task). `FW-VAL-D-011` content-pack-validator check enforces presence of all six fields per generated 3D asset. Failure to produce documented commercial-rights proof = asset blocked from RC bundle.

### Current candidate tool stack (subject to spike validation + license recheck)

These are the tools currently believed to support the spike. None are committed; all are evaluated at Phase-5 license-audit before the spike begins. If any fails the audit, the spike substitutes equivalents.

| Role | Candidate | Notes |
|---|---|---|
| 3D-asset generator (characters) | Tripo v3 | Plan-specific commercial rights; verify Pro/paid tier covers Steam-commercial use |
| 3D-asset generator (hero / stadium) | Rodin Gen-2 (Hyper3D) | Credit-based; commercial coverage requires paid plan |
| AI-assisted animation | Cascadeur Pro / Teams | Free is non-commercial + export-limited; commercial use requires paid tier |
| Cloth simulation | Magica Cloth 2 | Unity Asset Store EULA; already owned ($50 sunk) |
| Rigging / cleanup | Blender | GPL — no commercial-rights friction |
| Cel shader | URP Shader Graph + custom HLSL | Engine-licensed; no third-party rights friction |
| Reference / fallback model gen | Hunyuan3D | Free / open-weights; commercial coverage needs license-text re-read at Phase-5 |
| Toon-shader reference | lilToon | Open / permissive; verify license terms at Phase-5 |

## Alternatives if the spike fails

Captured now so the fallback is decidable without re-deriving the alternative space. Listed in order of decreasing visual ambition:

### Alternative A — Dots EA + 3D signature cut-ins only (post-EA)

Match runs as dots throughout the play. Signature executions trigger a brief illustrated-or-3D cut-in (≤2s) that breaks the dots flow for the moment of impact, then returns to dots. Cut-ins can be 3D OR illustrated 2D. This is far cheaper than full 3D match rendering, still gives marketing moments, and preserves the dots polish-bar investment.

### Alternative B — Low-poly mannequins before AI-generated characters

Before betting on AI-generated 3D characters, ship cel-shaded low-poly mannequin models (uniform body type, kits as material swaps, generic faces). Proves the camera + animation + cel-shader stack work end-to-end without taking on AI-character-generation risk. Players-as-individuals visual quality drops, but it's a valid solo-dev path. Could ship at EA OR be used as Phase-5 spike fallback to de-risk the 3D pipeline.

### Alternative C — Dots match + 3D replay moments post-match

Match plays as dots; post-match "highlight reel" renders the season's salience-gated key moments in 3D. Short clips, ~3-8 per season per club followed. Players watch the season's drama in 3D after the fact. Stepping stone if live 3D match readability is too expensive but the marketing visual is needed.

### Alternative D — Dots-only forever, post-EA 3D evaluated at audience-signal gate

The original 2026-04-22 commitment. Visual moat is "stylized dots done well," competing on sim depth + memory + signatures + tactical depth. Post-EA 3D R&D evaluated only if audience signal justifies. Honest fallback if the spike + alternatives all prove unviable for solo + AI-tooling.

## Open questions

Captured 2026-04-26; resolved at Phase-5 spike kickoff or sooner if pressure surfaces:

1. **Generated likeness risk.** Are AI-generated character faces unique enough to avoid likeness-conflict with real players? What audit step ensures no inadvertent likeness collisions with real footballers? Pairs with `design/content_policy.md §No real-person likenesses`.

2. **Skin / body / age variation coverage.** What's the minimum variation matrix the pipeline must produce to populate ~2000 players without obvious reuse? Skin tone bands × body type bands × age-appearance bands × hair variants — combinatorial explosion. Trade-off between variety + production cost.

3. **Trailer quality.** Even if in-game 3D is solo-dev-cel-shaded-quality, marketing trailer footage may need higher polish than runtime renders. Is a separate trailer pipeline owed (offline-rendered cinematic at higher quality + manual polish), or does in-game rendering have to suffice?

4. **Stadium / crowd scope.** Stadium 3D = potentially expensive. Crowd 3D = potentially very expensive. Decide at Phase-5 spike: stylized minimal stadium frame + abstract crowd? Photoreal stadium + stylized crowd? Pure-skybox stadium + cardboard-cutout crowd? "Full crowd" is a known scope trap; default to abstract.

5. **Minimum hardware spec.** Provisional baseline (integrated-graphics 1080p 60fps) needs concrete validation: which integrated GPU (Intel UHD vs Iris Xe vs Apple M-series)? What's the steam-deck-verified-relevant subset? Locks at Phase-5 alongside the spike's target-FPS deliverable.

6. **Signature-animation budget realism.** Solo dev × 24 signatures × cel-shaded animation + ball interaction. The Phase-5 spike validates 1 signature; Phase-6 scales. If "20 signatures realistic at this quality, drop 4" emerges, which 4 signatures get dropped? Schema-bump to `design/signatures.md` + `design/match-rating.md` cross-ref.

7. **Retargeting fallback coverage.** If retargeting tool fails on body-type-X, what's the failure mode — animation-look-broken, animation-doesn't-play, animation-substitutes-fallback? Spec the failure-mode contract before spike.

8. **AI-content-disclosure granularity for 3D.** Current `FW-VAL-D-005` covers pack-level disclosure. Per-asset 3D-character disclosure (this player's face was AI-generated, this kit was hand-painted) — does the player UI surface this anywhere, or is pack-level disclosure sufficient per Steam 2025 policy? Pairs with `design/content_policy.md §AI-content-disclosure granularity` Phase-3+ open question.

## Cross-references

- **2026-04-26 SPEC decisions-log entry** — visual-target supersession (this doc's authority)
- **ADR-0008 ShotPresentationContract** (owed) — renderer-agnostic contract this adapter consumes
- **ADR-0009 dots-phase render adapter** (owed) — sibling adapter that ships as Phase-3 prototype + EA fallback
- **ADR-0002 (Superseded)** — original viewer rendering pipeline; preserved for history per append-only ADR discipline
- **`design/signatures.md`** — 24-signature catalog; animations authored against this catalog at Phase-5/6
- **`design/semantic-cinema.md`** — 7-shot vocabulary applies to ALL adapters (rendered differently per adapter)
- **`design/accessibility.md`** — reduce-motion path applies adapter-aware: dots has different reduce-motion than 3D
- **`design/content_policy.md`** — no-real-person-likenesses policy + AI-content-disclosure
- **`design/specs/content-pack-validation-contract.md`** — `FW-VAL-D-005` AI-content disclosure (extended at this supersession to add `FW-VAL-D-011` for 3D-asset commercial-rights)
- **`SETUP.md §3 + §10`** — Tier 3 budget tier activation (now Phase-5/6, was Phase-9); 3D-tooling subscription triggers
- **`steam-release/asset-licensing-tracker.csv`** — schema bump owed at Phase 5 to add 3D-asset license columns

## Changelog within this doc

- **2026-04-26** — Authored as Phase-2 placeholder per visual-target supersession decisions-log entry. Spike-gate criteria locked (11 deliverables); animation contract surface owed (rig standard / clip format / event markers / ball-contact markers / retargeting / fallback); licensing-audit requirements first-class; current candidate tool stack listed (Tripo / Rodin / Cascadeur / Hunyuan3D / Magica Cloth 2 / Blender / lilToon — all subject to Phase-5 license-audit); four alternatives captured in priority order; eight open questions for Phase-5 resolution. Full pipeline spec lands at Phase-5 immediately before spike. ADR-0010 (3D shipping adapter) NOT pre-authored; conditional on spike-green outcome.

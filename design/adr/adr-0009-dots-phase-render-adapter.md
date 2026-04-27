---
description: ADR-0009 — Dots-phase render adapter. Sprite-on-pitch + minimal overlay viewer for Phase-3 sim-validation prototype. Consumes ADR-0008 ShotPresentationContract. Held to a shippable polish bar (tactical clarity / identity overlays / camera rhythm / readable possession-pressure / signature presentation) because dots may ship at EA per outcome (c2) of 2026-04-26 visual-target supersession.
---

# ADR-0009: Dots-phase render adapter

## Status

**Accepted** — 2026-04-27 after GPT-5.5 review pass (Concerns verdict; one P1 + two P2 findings applied + cross-ADR P1 propagation from ADR-0008) + Codex review pass (stale-reference cleanup; sign-off "no remaining blocking findings"). Originally Proposed 2026-04-26 per visual-target supersession decisions-log entry: ADR-0009 is the first consumer of ADR-0008's ShotPresentationContract and the renderer for Phase-3-onward sim-validation. Append-only from this point forward — supersession only via a new ADR.

## Date

2026-04-26

## Last Verified

2026-04-27

## Decision Makers

osagberg (project owner), GPT-5.5 (design partner), Claude (workhorse author).

---

## Summary

Sprite-on-pitch dots adapter implementing `IShotPresentationAdapter` (per ADR-0008). Renders 22 player sprites (dots) on a top-down pitch, with kit-color discrimination + identity overlays + camera rhythm + signature presentation cues. Held to a shippable polish bar because dots may ship at EA if the Phase-5 3D-pipeline spike (per `design/3d-pipeline.md`) fails. Designed to be cheap to build, sufficient for sim-validation Month-3 gate, and good enough to be a defensible EA visual if dots is the shipping outcome.

## Engine Compatibility

| Field | Value |
|---|---|
| Engine | Unity 6 LTS + URP 17+; minimal URP feature surface (no custom HLSL passes) |
| Domain | Rendering / Viewer adapter | <!-- ui-lint:allow term="domain" reason="ADR template canonical field name for engine-compat area" reviewer="osagberg" -->
| Knowledge Risk | LOW — sprite-on-pitch is well-trodden pattern; 2D Toolkit + Sprite Renderer + UI Toolkit overlays. No new engine APIs. |
| References Consulted | 2026-04-26 visual-target supersession entry, ADR-0008 ShotPresentationContract, `design/semantic-cinema.md` 7-shot vocabulary (interpreted dots-style), `design/accessibility.md` reduce-motion adapter posture, `design/3d-pipeline.md` (sibling adapter) |
| Post-Cutoff APIs Used | None |
| Verification Required | Phase-3 Week-2: dots adapter renders 3 of 7 shot types end-to-end against `ViewerEvent` stream; Month-3 gate: 5 cold observers verify drama-+-identity readability per `design/month-3-vertical-slice.md` |

## Dependencies

| Field | Value |
|---|---|
| Depends On | ADR-0008 (ShotPresentationContract — this adapter implements `IShotPresentationAdapter`); ADR-0001 (ShotTypeSO — shot identity); ADR-0004 (MemoryEvent — memory-hit callback text source); `design/accessibility.md` (reduce-motion adapter scope); `design/ui-vocabulary.md` (overlay text discipline) |
| Enables | Phase-3 Month-3 gate (sim-readability test through dots renderer); EA-launch visual fallback if 3D-pipeline spike fails |
| Blocks | Phase-3 Week-2 viewer prototype; Phase-3 devlog clips |

---

## Context

### Problem statement

The 2026-04-26 visual-target supersession says dots-prototype is the Phase-3-onward validation layer + may ship at EA if the 3D-pipeline spike fails. GPT's review explicitly said dots must NOT be treated as throwaway debug UI: if dots ships, they need polish — tactical clarity, identity overlays, camera rhythm, readable possession-pressure, signature presentation. This ADR commits the polish bar + the implementation strategy.

### Current state

- No Unity project exists yet; Phase-3 Week-2 creates it.
- ADR-0008 Proposed defines the contract this adapter consumes.
- `design/semantic-cinema.md` 7-shot vocabulary is renderer-agnostic; this ADR specifies how each shot type renders in dots-mode.
- `design/3d-pipeline.md` placeholder defines the parallel 3D adapter; this ADR is its sibling.

### Constraints

- **Polish bar (must hit, not optional).** Dots is potentially the shipping visual. Cannot be skinned debug UI. Specifically: kits visually distinct at gameplay camera distance + signature triggers visible at-a-glance + commentary text legibly synchronized + crowd-audio-text overlay readable + post-match summary surface present.
- **Cheap to build (relative to 3D).** Production cost target: ≤4 weeks of solo-dev Phase-3 effort to first playable; ≤8 weeks to full polish bar.
- **Implements ADR-0008 contract verbatim.** No adapter-specific extensions to the contract; the adapter consumes what's there.
- **Reduce-motion structurally.** Per `design/accessibility.md`: scene-load-time disable of motion-line trails on dots, slowed shot-transition timing, simplified camera rhythm.
- **Determinism preserved.** Per ADR-0008 + ADR-0001: no `_Time`-driven shaders that affect gameplay-visible elements; no `Random` outside seeded source; the dots adapter's semantic pass-activation trace is stable across runs. Pixel output is visually QA'd, not replay-hashed.
- **Mod-pack-loadable.** Mods reference shot types by `ContentPackQualifiedId`; dots adapter resolves these via the same Addressables groups per ADR-0001.

---

## Decision

Implement a `Viewer.Adapters.Dots` Unity-side asmdef. Render with sprites on a top-down pitch + UI Toolkit overlays. Honor the 7-shot vocabulary by interpreting each shot type as a dots-specific framing + camera-rhythm pattern.

### Core rendering primitives

- **Pitch:** orthographic top-down sprite view of the pitch. Pitch sprite includes lines + center circle + penalty boxes. Background dampens (slightly desaturated green) so kit colors pop.
- **Player dot:** circular sprite, kit-primary color fill, kit-secondary color outline, jersey-number label inside (small, JetBrains Mono per `design/semantic-cinema.md` typography stack). Dot diameter chosen for readability at 1080p (provisional ~24px; tunable Phase-3 Week-2).
- **Ball:** smaller white sprite with subtle drop-shadow.
- **Player identity overlay (on hover / focus):** name + position-label + match-rating-so-far floats above the dot in a tooltip-styled UI Toolkit element.
- **Selection ring:** when a `ViewerEvent.ParticipantPlayerIds` element is the focal player (e.g. signature trigger / pass-shot-impact), a colored ring around the dot signals "this player is currently the focal subject." Ring color reflects shot intensity.
- **Camera:** orthographic with controlled zoom + pan. No 3D perspective; no parallax. Camera target derives from `ViewerEvent` participants + ball position.
- **No motion-line trails on dots by default.** A short fading-trail (last ~6 ticks of position history) on running players, OFF when reduce-motion is enabled. Lower visual cost than 3D motion lines; preserves "fast players are visibly fast."

### 7-shot interpretation in dots

Each shot type from `design/semantic-cinema.md` renders dots-style:

| Shot type | Dots interpretation |
|---|---|
| `tactical-wide` | Default zoom; full pitch visible; routine play |
| `diagonal-attack-lane` | Camera tilts toward the attacking team's third + zooms slightly; subtle desaturation everywhere except the attacking lane (UI-Toolkit overlay mask) |
| `player-isolation` | Camera zooms onto one player; their dot enlarges + name + position-label overlay appears with a subtle background-card |
| `duel-panel` | Camera zooms onto two players in contention; both dots get duel-rings; UI-Toolkit split-overlay with their names side-by-side |
| `pass-shot-impact` | Brief micro-zoom + flash on the ball-receiver dot or shot moment; UI-Toolkit overlay text ("Driven low across the box") |
| `crowd-reaction` | Camera pulls slightly back; commentary-text overlay surges with crowd-audio subtitle (e.g. *"Stadium has erupted"*); kit colors briefly desaturate to background |
| `aftermath-freeze` | **Viewer playback holds the rendered frame for 1.5-2.5s** (per stakes); canonical MatchSim time and event stream continue uninterrupted. The viewer's render-tick lags real-time during the hold; on release, the adapter catches up by replaying buffered ViewerEvents at accelerated playback speed (or skipping interpolation directly to the current sim tick if the buffer overflowed). Larger UI-Toolkit overlay text card (post-event prose); slight zoom on relevant dot. Pause/resume here is a presentation-layer concept ONLY — sim never pauses. |

Reduce-motion variants (per ADR-0001 `reduce_motion_variant`):
- `pass-shot-impact` → flash collapses to brief overlay-text-only
- `crowd-reaction` → desaturation + zoom-back disabled; overlay text only
- `aftermath-freeze` → hold extended +30% per `design/accessibility.md` table

### Polish bar (the "shippable dots" criteria)

Owed at Phase-3 Month-3 gate; verified by 5 cold observer test:

1. **Kit discrimination.** Home + away kits at any pitch position visually distinguishable at single glance.
2. **Player identity legibility.** Player number readable inside the dot; hovering the dot surfaces full name + role-position label.
3. **Possession indicator.** Ball-control color subtly tints the closest dot; visible even in tactical-wide.
4. **Pressure indicator.** A team in defensive shape pressing high renders with subtle "compressed" pitch-line emphasis on UI-Toolkit overlay (lines redrawn between defenders showing the defensive shape). Possession + pressure together communicate game state without reading commentary.
5. **Signature presentation.** Signature triggers fire a recognizable visual cue: dot-flash + selection-ring + UI-Toolkit overlay quote in football-native vocabulary. Player understands a signature happened without reading match text.
6. **Camera rhythm.** Camera transitions between shot types are smooth (eased), not jump-cut. Routine play uses tactical-wide; signature / shot / goal punches in; aftermath holds and releases. Match feels watched, not surveilled.
7. **Commentary integration.** UI-Toolkit overlay text is timed with `ViewerEvent` start-tick; never lags by visible amount. Reads as commentary, not subtitles to a video.
8. **Crowd-audio subtitle integration.** Subtitle toggle (per `design/accessibility.md`) surfaces `(Stadium hush)` / `(Home crowd erupts)` overlays at salience-gated moments.
9. **Post-match summary.** Post-match screen surfaces ratings, key events, scout-readable phenotype updates per `design/player-generation.md`. Players who stepped away during the match can reconstruct the story.
10. **Reduce-motion polish.** Reduce-motion path is tested + visually verified to work at the Month-3 gate (paired corpus fixture per `design/specs/golden-replay-corpus.md`).

If the polish bar is not hit at Month-3 gate, the project doesn't advance to Phase 4 — that's the gate's purpose per `design/month-3-vertical-slice.md`.

#### Observer-task script (operationalizes the ≥4-of-5 cold-observer gate)

The Month-3 gate runs each cold observer through the same scripted protocol. They watch a 3-minute dots-rendered match clip (single fixture seed; reduce-motion variant viewed last for the reduce-motion-readability task). After viewing, they answer six tasks. Each task has a binary pass/fail bar. The observer passes the gate iff they pass ≥5 of 6 tasks. The match passes the gate iff ≥4 of 5 observers pass.

| # | Task | Pass criterion |
|---|---|---|
| 1 | **Identify ball carrier at the 30s, 90s, and 150s marks.** | ≥2 of 3 correct (visual-only — no scoreboard / commentary peek). |
| 2 | **Identify which team is pressing high and which is sitting deep.** | Correct attribution at both 60s and 120s marks. Either team naming or kit-color naming OK. |
| 3 | **Name the focal player when a `player-isolation` / `pass-shot-impact` shot fires.** | ≥1 of any focal-shot moments correctly named (jersey number OR name OR position-label all count). |
| 4 | **Did a signature fire during this clip? If yes, which player?** | If a signature fired: correctly identifies it fired AND names the player. If no signature fired: correctly says no signature fired. |
| 5 | **Why did the last high-stakes moment matter?** (Free response.) | Observer's answer references at least one of: scoreline / form-rising-or-falling / memory callback / signature / rivalry. Does not require football-domain vocabulary; "this guy hates that other guy because of last season" counts. |
| 6 | **Reduce-motion readability.** Observer re-watches the same clip with reduce-motion ON. | Observer reports they could still follow the match. Optional follow-up: "Did you notice the difference?" — either yes or no answer is fine, the question is whether legibility held. |

**Operationalization:**
- Tasks 1 + 2 + 4 + 6 are binary by construction.
- Task 3 is binary; the observer either named someone correctly or didn't.
- Task 5 is the only judgment call. Two reviewers (project owner + one other) score independently; disagreement = observer fails task 5. If two-reviewer scoring is unavailable, default to project-owner-only scoring with a written note for each fail.

This replaces "drama legible" as a vibe-check with a falsifiable rubric. Failure modes the rubric is designed to surface: dots-too-similar (task 1 fails), pressure-indicator-not-readable (task 2 fails), signature-presentation-cue-missed (task 4 fails), narrative-context-not-conveyed (task 5 fails).

### Scene-load-time + adapter selection

- Settings panel shows adapter selection IF multiple adapters compiled into build. Phase-3 prototype builds dots-only; Phase-5 spike-green build can compile both.
- Reduce-motion toggle in same settings panel; flips on scene reload (per `design/accessibility.md` + ADR-0008 `ReduceMotionStrategy.SceneLoadTime`).
- No live adapter-swap mid-match.

### Tier-A CI integration

- **Phase-3 Week-2 synthetic fixture** (lands first; gates the corpus seed). A hand-authored `ViewerEvent` stream covers all three Week-2 shot types (`tactical-wide` + `diagonal-attack-lane` + `pass-shot-impact`) with explicit reduce-motion + non-reduce-motion variants. The dots adapter consumes this stream end-to-end and emits a non-empty pass-activation trace. Fixture pinned at `MatchSim.Tests/fixtures/synthetic/dots-three-shot-types.json`. This ensures the smoke test cannot pass on an empty trace — the canonical risk if a 60-tick sim seed produces no shot changes before the full sim loop exists.
- **Phase-3 Week-3+ corpus seed** (lands once MatchSim ball + player + 2 BT archetypes can produce meaningful events). Consume 60 sim ticks (1 game-second) headlessly through `Viewer.EventBridge` + dots adapter, emit the dots adapter's semantic `pass_activation_log`, hash, compare via adapter-keyed `pass_activation_log_hashes["dots"]` per ADR-0008. Fixture pinned in `MatchSim.Tests/fixtures/replay-corpus/0xdeadbeefdeadbeef.json` (already established as Tier-A smoke seed per `design/specs/golden-replay-corpus.md`). Visual smoke captures are separate artifacts, not cross-platform hash inputs.
- The two fixtures are kept separate forever: synthetic-fixture proves the adapter consumes the contract correctly; corpus seed proves end-to-end sim-+-adapter determinism. Either passing without the other = signal something is wrong, not "good enough."
- `fw shader-audit` per Phase-3 SPEC task ensures no `_Time` references in dots-adapter shaders. Knowledge Risk LOW because dots adapter doesn't need custom HLSL — built on URP defaults + Sprite Renderer.

---

## Alternatives considered

### Alternative 1 — full FM-style dots (jersey numbers + name labels visible always) (REJECTED)

Information-dense, but cluttered at 1080p with 22 dots + ball + shape overlays. Hover-to-reveal is more honest about football-management cognitive load — you focus on a few players at a time, not all 22.

### Alternative 2 — dots as colored discs only (no jersey numbers) (REJECTED)

Loses identity at-a-glance. Polish bar requires identity legibility.

### Alternative 3 — dots with full-time motion-line trails (always on) (REJECTED)

Visual noise; conflicts with reduce-motion path; clutters at high pace. Short fading trails (~6 ticks) preserves speed-readability at lower cost; trails-off in reduce-motion.

### Alternative 4 — debug UI rendering (skipping polish bar) (REJECTED)

Per GPT review: dots may ship at EA. Cannot be debug-quality. Polish bar is the contract.

### Alternative 5 — embed mini-illustrated portrait cut-ins on signature events (REJECTED for Phase 3, possibly DEFERRED)

The "Alternative A" from `design/3d-pipeline.md §Alternatives if the spike fails`: dots-EA + 3D-or-illustrated cut-ins for signatures. NOT in scope of this ADR; if the 3D spike fails and dots ships at EA, illustrated-cut-ins could augment dots in a future ADR. Current ADR is dots-only.

---

## Consequences

### Positive

- **Cheap to build, cheap to maintain.** Sprite-on-pitch + URP-default rendering. No custom HLSL passes; minimal Shader Graph; no per-player rig + animation production.
- **Polish bar produces a defensible shipping visual** if 3D-pipeline spike fails. EA is not at the mercy of 3D timing.
- **Determinism-clean.** No `_Time`-affected gameplay rendering; semantic pass-activation trace is stable.
- **Reduce-motion-clean.** Reduce-motion path is structural (scene-load-time feature disable); accessibility paired-fixture pattern works from day one.
- **Mod-pack-clean.** Dots adapter resolves shot types from any pack via Addressables per ADR-0001. Mods extend, don't override.
- **Sim-validation-clean.** Month-3 gate measures sim readability through this adapter; observers don't have to "see through" production-quality 3D to evaluate sim drama. Gate becomes more honest about whether the sim is interesting.

### Negative / Risks

- **Marketing cost if dots ships at EA.** Steam screenshots + trailer are dots-rendered; no sweeping cinematic 3D footage. The "we're a different sports sim because we're 2D-stylized" pitch becomes "we're focused on sim depth + memory + signatures, with a stylized dots viewer" — defensible but harder to differentiate at a glance on a Steam page.
- **Two-adapter maintenance once 3D is shipping.** Dots adapter must continue to work even after 3D ships. Validator + corpus fixtures track both. Ongoing test cost.
- **Player perception risk.** Some players will see "dots viewer" and think "lazy dev" before reading the framing. Marketing must explicitly position dots as a visual identity choice, not a production shortcut. Pairs with `design/3d-pipeline.md §Alternatives` discussion.
- **Polish bar is genuinely demanding.** A good dots-style football viewer is harder than it looks. Camera rhythm + pressure indicator + signature presentation are real design challenges; budget Phase-3 effort accordingly.

### Neutral

- **Asmdef structure clean.** `Viewer.Adapters.Dots` is its own Unity-side asmdef; depends on `Viewer.Contracts` + `Viewer.Core` and consumes bridged `ViewerEvent`s. It does not depend directly on `MatchSim.Contracts`.
- **No new content-pack types.** Dots adapter consumes existing `ShotTypeSO` content packs.

---

## Validation criteria

- [ ] Phase-3 Week-2: `Viewer.Adapters.Dots` asmdef compiles + renders 3 shot types (`tactical-wide` + `diagonal-attack-lane` + `pass-shot-impact`) against `ViewerEvent` stream.
- [ ] Phase-3 Week-2: synthetic ViewerEvent fixture (`MatchSim.Tests/fixtures/synthetic/dots-three-shot-types.json`) consumed end-to-end produces a non-empty, deterministic pass-activation trace. Hashes match across Win/Mac/Linux.
- [ ] Phase-3 Week-3: paired corpus fixtures (`<seed>.json` + `<seed>.reduce-motion.json`) pass for the dots adapter via adapter-keyed `pass_activation_log_hashes["dots"]`. Lands once MatchSim ball + player + 2 BT archetypes can produce meaningful events.
- [ ] Phase-3 Week-4: signature trigger renders the polish-bar visual cue (dot-flash + selection-ring + overlay quote).
- [ ] Phase-3 end-of-Month-3: 5 cold observers (per `design/month-3-vertical-slice.md`) run through the §Polish bar > Observer-task script. ≥4 of 5 pass (each observer passes iff they pass ≥5 of 6 tasks).
- [ ] No `_Time` references in dots adapter shaders per `fw shader-audit`.
- [ ] Tier-A CI smoke (synthetic-fixture path Week-2; corpus-seed path Week-3+) emits a deterministic dots pass-activation trace; `pass_activation_log_hashes["dots"]` matches pinned corpus value across Win/Mac/Linux. Rendered screenshot/video smoke is verified separately and not pixel-hashed.
- [ ] Aftermath-freeze viewer hold confirmed to NOT pause MatchSim canonical time: integration test runs a 5-second match clip with two aftermath-freeze events; canonical state hash + event stream are identical with-vs-without observer-side viewer attached.

---

## Open questions

1. **Dot diameter + pitch dimensions.** Provisional 24px @ 1080p; tunable. Visual-design pass at Phase-3 Week-2 picks final values.

2. **Pressure-indicator visual language.** "Compressed pitch-line emphasis" is sketched, not specified. UI-Toolkit overlay shape — connect-the-defenders polygon? Heatmap shading? Tactical-pass-ASCII style? Resolves at Phase-3 Week-2.

3. **Hover-vs-tap discoverability.** PC-only at MVP per CLAUDE.md scope. Mouse-hover for identity overlay is fine; what about click-to-pin for one player's overlay? Keyboard navigation through dots? Pairs with `design/accessibility.md §Remappable controls`.

4. **Crowd-audio subtitle visual integration.** Subtitle text bottom-center per accessibility spec; aftermath-freeze overlay also uses bottom area. Collision rule was "subtitle offsets up when aftermath-freeze active" — how the dots adapter implements that visually.

5. **Camera-rhythm easing curves.** Smooth-eased camera transitions need an easing-function lock. Easing curve choice affects how matches "feel"; worth a Phase-3 Week-3 visual-feel test.

---

## Cross-references

- **2026-04-26 SPEC decisions-log entry** — visual-target supersession (this ADR's authority)
- **ADR-0008 ShotPresentationContract** — contract this adapter implements
- **ADR-0010** (NOT pre-authored) — sibling 3D adapter, conditional on Phase-5 spike
- **ADR-0002 (Superseded)** — original viewer rendering pipeline; preserved for history
- **ADR-0001 ShotTypeSO** — shot identity + reduce-motion variant
- **`design/semantic-cinema.md`** — 7-shot vocabulary; this ADR specifies dots interpretation
- **`design/accessibility.md`** — reduce-motion adapter posture + subtitle integration
- **`design/ui-vocabulary.md`** — overlay text discipline (banned-terms lint applies)
- **`design/month-3-vertical-slice.md`** — gate criterion that this adapter is measured against
- **`design/specs/golden-replay-corpus.md`** — paired fixtures + v1 adapter-keyed `pass_activation_log_hashes`; dots reads/writes `pass_activation_log_hashes["dots"]`
- **`design/3d-pipeline.md`** — sibling 3D adapter pipeline + spike-gate criteria

## Changelog within this doc

- **2026-04-26** — Authored as Proposed per visual-target supersession decisions-log entry. First consumer of ADR-0008 ShotPresentationContract. Polish bar locked (10 criteria). 7-shot dots interpretation table. Reduce-motion adapter posture. Tier-A CI smoke wiring. Five rejected alternatives. Five open questions for Phase-3 Week-2/3 resolution. Awaits user / GPT-5.5 review pass before flipping to Accepted.
- **2026-04-27** — GPT-5.5 review pass landed (Concerns verdict; one P1 + two P2 findings against this ADR + cross-ADR P1 propagation from ADR-0008). Applied: (P1-4) `aftermath-freeze` row in §7-shot interpretation table rewritten to lock the viewer-vs-sim time boundary — viewer holds the rendered frame; canonical MatchSim time + event stream continue uninterrupted; on release, adapter accelerates playback or skips interpolation. (P2-7) Polish bar gained an operational §Observer-task script: 6 binary tasks (ball-carrier identification at 30s/90s/150s, pressing-team identification at 60s/120s, focal-player naming on player-isolation/pass-shot-impact, signature-fired identification, why-the-last-high-stakes-moment-mattered free-response, reduce-motion readability) with explicit pass/fail criteria. Replaces "drama legible" vibe-check with falsifiable rubric. Each observer passes iff ≥5 of 6 tasks pass; ≥4 of 5 observers pass = gate green. (P2-8) §Tier-A CI integration split into two separate fixtures: synthetic ViewerEvent fixture (Week 2; covers all 3 shot types; non-empty trace guaranteed) lands FIRST; corpus seed (Week 3+; end-to-end sim+adapter; lands once meaningful sim events are producible) lands SECOND. Either passing without the other is treated as signal-something-wrong, not "good enough." §Validation criteria + §Cross-references updated to reflect adapter-keyed `pass_activation_log_hashes["dots"]` path from ADR-0008 v1 + sim-time-not-pausing integration test. Status remains Proposed; awaits Codex review pass before flipping to Accepted.
- **2026-04-27** — Codex review pass + Accept flip. Codex confirmed all GPT-5.5 findings landed correctly + applied stale-reference cleanup on this ADR's §Cross-references (singular → adapter-keyed plural). Codex sign-off: "no remaining blocking findings; ADR-0008 + ADR-0009 OK to flip Proposed → Accepted." Status now **Accepted**; append-only from this point forward. ADR-0010 (3D adapter) remains NOT-pre-authored and conditional on Phase-5 spike outcome.

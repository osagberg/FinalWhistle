---
description: Phase-3/4 guardrail spec for anime-inflected presentation in the dots adapter (and conditional 3D adapter). Enumerates the anime-coded signal surfaces the dots viewer must ship with so anime-curious players see the genre's DNA even if 3D slips. NO mid-match QTE — football-correct mandate stands.
last_verified: 2026-04-30
status: Phase-3/4 placeholder spec — full surface specs land alongside Phase-3 EventBridge + Phase-4 dots-adapter polish-bar work; this doc locks the budget shape so future authoring stays disciplined.
---

# Anime Presentation Budget — guardrail spec

## Purpose

The Final Whistle pillar promise is "anime-inflected presentation, not anime mechanics." This doc names the **specific anime-coded surfaces** the dots adapter must ship to deliver on that promise — so anime-curious players see the genre's signature signals even if the cel-shaded 3D adapter slips per the Phase-5/6 production-feasibility spike outcome (per `design/3d-pipeline.md` + 2026-04-26 visual-target supersession).

It is a **budget**, not a wish-list. Each surface is named, scoped, and tied to a Phase-3-or-Phase-4 trigger. Surfaces NOT listed here are out of the anime-coded budget and require a new SPEC entry to add.

## Locked decisions

- **Anime-coded budget is presentation-only.** Mid-match QTE is HARD-BANNED — `design/breakthrough-moments.md` already rejects it on football-correctness grounds + the 2026-04-24 month-3-vertical-slice resolution. Any future request to add input-driven beats during a live match requires a new SPEC decision superseding this spec + the 2026-04-24 resolution.
- **The 7-shot semantic-cinema grammar is the cinematography vocabulary.** `tactical-wide` / `diagonal-attack-lane` / `player-isolation` / `duel-panel` / `pass-shot-impact` / `crowd-reaction` / `aftermath-freeze` per `design/semantic-cinema.md`. Anime DNA enters via STAKES/MEMORY modulation of those shots, NOT via new shot types.
- **Dots adapter ships the anime-coded budget.** It is NOT contingent on the 3D spike succeeding — the dots viewer (per ADR-0009 polish bar) must independently deliver the genre signal. The 3D adapter (per ADR-0010 conditional) inherits the same budget if it ships, with adapter-specific implementations.
- **Anime-coded surfaces are stakes-modulated.** A signature in a friendly scrimmage gets a brief title-card; the same signature in a cup final at 0-0 with 5 minutes left gets the full motion-line + impact-frame stylization. Salience-driven presentation prevents the viewer from over-firing.
- **Localized text never hard-codes anime vocabulary.** The viewer overlay text follows `design/ui-vocabulary.md` Category-A bans + Category-B exemption discipline. Anime-coded means visual treatment + camera + sound, not "stadium has gone Hush" copy.

## The budget — eight surfaces

| Surface | What it is | Phase-3 minimum (dots) | Phase-4+ expansion | Cross-ref |
|---|---|---|---|---|
| **Impact frames** | Brief 1-3 frame freeze + tone-shift on the moment of contact (shot, key tackle, signature execution). Manga-panel beat. | At least 1 impact frame on `pass-shot-impact` shot type when STAKES≥high. | Frame-count + tone palette modulated by salience tier + memory-hit context. | ADR-0009 polish bar §camera rhythm; `design/semantic-cinema.md` |
| **Screen-tone / motion-line variants** | Background screen-tone overlay + radial motion lines around moving subjects during peak action moments. | Static screen-tone applied during `aftermath-freeze`; motion lines during the dot's run-up to a signature execution. | Multiple tone palettes per stakes tier; motion-line density modulated by player speed + signature affinity. | ADR-0009 |
| **Signature title-card / cut-in language** | Brief overlay name-card when a signature executes — "First-time diagonal switch" with player name + role icon. | Title-card on each of the 3 Phase-3 active signatures (#13, #20, #22) when they fire; UI-Toolkit overlay; reduce-motion variant per ADR-0001. | Signature-specific cut-in art (Phase-4 art pipeline trigger); per-signature title-card timings tuned by salience. | `design/signatures.md`, ADR-0001, `design/breakthrough-moments.md` |
| **Camera rhythm** | Shot duration + transition cadence varies by stakes. Anime sports cinema typically holds tactical-wide longer in early-match calm, then accelerates cuts during late-game tension. | Three cadence tiers (calm / standard / tension) keyed to stakes; transitions between them are time-windowed not abrupt. | Stakes input expands to include memory-hit weight + per-team form; cadence library expands beyond three tiers. | ADR-0009 polish bar §camera rhythm |
| **Pressure indicator** | Visual indicator of mounting stakes / momentum without using banned vocabulary. Examples: stadium-tone shift, crowd-density pulse, sideline-camera intrusion. | One visual pressure surface (recommendation: stadium-tone shift on `crowd-reaction` shot) implemented for the Month-3 gate. | Multi-surface pressure vocabulary (tone + crowd + sideline + commentary cadence) reading combined inputs. | ADR-0009 polish bar §pressure indicator |
| **Aftermath-freeze emphasis** | Post-event freeze on a key player or moment (after a goal, after a signature, after a sending-off Phase 4+). | Freeze beat on goals; reduce-motion-aware substitution per ADR-0001. | Freeze duration salience-modulated; player-isolation framing customizable per signature. | ADR-0008/0009; reduce-motion ADR-0001 |
| **Signature-specific commentary cadence** | Commentary line timing + emphasis matches the signature's cinematic beat. The "ohh!" lands on impact, not 2 seconds late. | Commentary lines authored with explicit timing markers tied to ViewerEvent.StartTick + duration. | Per-signature emphasis library; localized across `en_GB` first per `content_policy.md`. | `design/event-sourced-memory.md`; `design/ui-vocabulary.md` |
| **UI-Toolkit overlay typography rhythm** | The overlay-text typography stack (Anton display / Rajdhani body per CLAUDE.md tech stack) emphasizes anime-coded moments via type-size pulse + slight letter-spacing modulation. | Title-card uses the locked stack; standard match overlays stay calm. | Type-rhythm system (e.g. type-pulse on goals/signatures) layered on top of base typography; reduce-motion-aware. | CLAUDE.md tech stack §UI; `design/ui-vocabulary.md` |

## Out of budget — explicit no-list

- **Mid-match QTE.** `design/breakthrough-moments.md` rejected this on football-correctness grounds. No exceptions without new SPEC decision.
<!-- ui-lint:ignore-start reason="meta-reference to banned overlay vocabulary; this doc explicitly forbids it" -->
- **Capitalized state-noun overlays** (`The Hush`, `The Calling`, etc.) — banned by `design/ui-vocabulary.md` Category A.1.
<!-- ui-lint:ignore-end -->
- **Screen-shake on every collision** — overuse trivializes the impact-frame budget.
- **Speed-line spam outside signature executions or peak-stakes moments** — overuse fights camera rhythm.
- **Anime-character cut-ins for every player** — title-cards are signature-execution-only at Phase 3; per-player cut-ins are a Phase-4+ art-pipeline cost we don't pre-promise.

## MVP boundary

**Month-3 gate (Phase 3):** the dots adapter ships at least the underlined Phase-3-minimum cells in the table — impact frames on `pass-shot-impact`, signature title-cards on the 3 active signatures, three-tier camera rhythm, stadium-tone pressure indicator, post-goal aftermath freeze. Together these constitute "an anime-curious observer watching a 3-minute match recognizes genre signal" — the load-bearing claim per the Month-3 observer rubric.

**EA (Phase 8):** all 8 surfaces shipping at the dots-adapter polish bar (per ADR-0009). 3D adapter (if spike-green) inherits the same budget with adapter-specific implementations. If 3D spike-yellow/red, dots ships EA with the full 8 surfaces; per the 2026-04-26 supersession, no public 3D promise is dated.

## Deferred

- **Per-signature cut-in art** — Phase-4 trigger when the signature catalog expands beyond the 3 Phase-3 actives.
- **Adaptive presentation library** (e.g., a recommender that picks the right impact-frame palette per match flow) — Phase 6.
- **Audio-side anime-coded budget** (musical-cue language, vocal commentary cadence variants) — Phase 6 audio pass; complementary to this spec but tracked separately because audio-direction is a different authoring pipeline.

## Test policy

- **Each Phase-3 surface has a synthetic-fixture test** that exercises the bridge → adapter → overlay pipeline for that surface independently. ADR-0009 §Polish-bar observer rubric calls these out — they're the load-bearing tests for Phase-3 polish-bar verification.
- **Reduce-motion variants** for each surface tested via the reduce-motion-substitution path per ADR-0001. Flag stays sticky until explicitly cleared.
- **Salience modulation** verified by varying STAKES inputs while holding sim state constant — the same fixture under low-stakes and high-stakes must produce visibly different presentation outputs.

## Cross-refs

- ADR-0001 — reduce-motion substitution applies to every surface in the budget.
- ADR-0008 — `ShotPresentationContract` binds the budget to canonical sim events.
- ADR-0009 — dots-adapter polish bar; this doc's table maps directly to the polish-bar items.
- ADR-0010 (conditional) — 3D adapter inherits the budget if it ships per Phase-5/6 spike outcome.
- `design/semantic-cinema.md` — 7-shot grammar; this doc's surfaces are stakes-modulated layers ON TOP of the grammar.
- `design/breakthrough-moments.md` — football-correct mandate; QTE rejection.
- `design/signatures.md` — 24-signature catalog; title-card / cut-in surfaces target signature executions.
- `design/3d-pipeline.md` — 3D-spike outcome scope.
- `design/ui-vocabulary.md` — banned-terms; overlay text discipline.
- `design/specs/golden-replay-corpus.md` — pass-activation log captures structural fields per surface (no rendered prose unless `locale_pin` is set).

## Authoring trigger

This spec is a placeholder authored 2026-04-30 per the Codex round-4 follow-up plan + audit-07 P0 anime-coded-gameplay finding. Per Codex's guidance: "Author `design/anime-presentation-budget.md` as a Phase-3/4 guardrail so dots still has anime signals if 3D slips."

The full per-surface implementation specs land alongside the Phase-3 Viewer.EventBridge + Phase-4 dots-adapter polish-bar work. This doc locks the **shape** of the budget so future authoring stays inside the eight named surfaces; surfaces NOT listed here require a new SPEC entry to add.

## Changelog

- **2026-04-30** — Initial placeholder authored per Codex round-4 follow-up plan, commit #5 of 6 (Design/State Sync). Eight anime-coded surfaces enumerated as the Phase-3/4 guardrail budget; explicit no-list locks out QTE, capitalized state-nouns, and speed-line spam. Authoring trigger for full per-surface specs is Phase-3 EventBridge / Phase-4 polish-bar work.

---
description: 7-shot-type camera grammar. How MatchSim events render as semantic cinema. Renderer-agnostic per ADR-0008; both dots-phase (ADR-0009) and candidate cel-shaded 3D (ADR-0010 conditional) adapters consume the same vocabulary with adapter-specific rendering.
last_verified: 2026-04-26
status: Phase 0 open questions resolved; shot-count locked through Month-3 gate, ShotTypeSO schema (ADR-0001) carries chain rules + reduce-motion + framing parameters consumed by both adapters. Rendering stack split: ADR-0008 ShotPresentationContract is renderer-agnostic; ADR-0009 dots-adapter + ADR-0010 (conditional) 3D-adapter are renderer-specific. Typography stack (Anton display / JetBrains Mono data / Rajdhani body, scoreline override) shared across adapters.
---

# Semantic Cinema — renderer-agnostic 7-shot vocabulary

> **2026-04-26 visual-target supersession.** This document originally framed the 7-shot vocabulary as a stylized-2D-illustrated rendering pipeline. After supersession: the 7-shot vocabulary is **renderer-agnostic** (locked in ADR-0008 `ShotPresentationContract`); each adapter (ADR-0009 dots / ADR-0010 candidate 3D) interprets the shot identities + modulation per its own rendering capabilities. Per-adapter interpretation tables live in the adapter ADRs (ADR-0009 §7-shot interpretation in dots; ADR-0010 owed). The canonical shot grammar below is what both adapters honor.

## Purpose

Answer "how does a deterministic MatchSim event stream render as a watchable match that reads as drama, not data?" The shot vocabulary is renderer-agnostic per ADR-0008 `ShotPresentationContract`; same shot identities drive the dots adapter (Phase-3-onward, ADR-0009) and any future 3D adapter (conditional on the Phase-5/6 production-feasibility spike per `design/3d-pipeline.md`).

## Locked decisions

See SPEC.md 2026-04-22. Summary:

- **7 shot types** form the vocabulary. Not 15. Not unique per event.
- Stakes + memory state **modulate** (intensity / color / paneling / text / timing), not swap the vocabulary.
- Manga-broadcast aesthetic: diagonal compositions, screen-tone, motion lines on runs, impact frames on contacts, state-driven colour grading.
- UI Toolkit for all overlay UI; 2D renderer via URP with custom passes for screen-tone + motion-line + impact-frame effects.

## The 7 shot types

| # | Shot type | Primary use | Secondary modulation |
|---|---|---|---|
| 1 | `tactical-wide` | Broadcast-style overview; default fallback; possession phases | Pull-back intensity on routine; drift on tempo shifts |
| 2 | `diagonal-attack-lane` | Attacking run emphasized; pitch tilted 15-25° | Steeper angle + saturation bump on stakes |
| 3 | `player-isolation` | Close on one player — pre-kick focus, pre-decision moment, reaction | Depth-of-field bloom + desaturation on memory-relevant figures |
| 4 | `duel-panel` | Split-panel 1v1 emphasis (challenge, header, shoulder-to-shoulder) | Hard split + screen-tone hatching on high-stakes |
| 5 | `pass-shot-impact` | Freeze-frame on contact moment (shot, cross, key pass) | Motion-line intensity + white-flash on goal/worldie |
| 6 | `crowd-reaction` | Stylized crowd cutaway (goal, near-miss, hostile reception) | Crowd density + color grade per home/away state |
| 7 | `aftermath-freeze` | Static panel held after key event (goal celebration, red card, injury) | Hold-length + text overlay weighted by event salience |

**Event → shot mapping** (MatchSim emits structured events; renderer selects shot):

| MatchSim event | Default shot | Stake-elevated shot |
|---|---|---|
| Ball rolling, transitional | `tactical-wide` | `tactical-wide` |
| Through-ball launched | `diagonal-attack-lane` | `diagonal-attack-lane` + higher angle |
| Shot taken | `pass-shot-impact` | `pass-shot-impact` + crowd-reaction cut |
| Goal scored | `pass-shot-impact` → `crowd-reaction` → `aftermath-freeze` | add `player-isolation` on scorer before aftermath |
| Signature triggered | `player-isolation` → `pass-shot-impact` | add pre-panel text overlay ("He arrives late in the box") |
| Duel / tackle | `duel-panel` | `duel-panel` + screen-tone |
| Foul / card | `aftermath-freeze` | add `player-isolation` on player card-receiver |
| Half-time / full-time | `crowd-reaction` → `aftermath-freeze` | weight crowd by result-context |

## Stakes modulation

A "stakes" score derives from:

- Competition (friendly: 0.1 / league midseason: 0.4 / derby: 0.7 / cup final: 1.0)
- Scoreline context (draw with minute weighting; losing by 1 in final 15 = high; blowout = low)
- Ledger relevance (any player involved has a high-salience prior event? boosts)

`stakes ∈ [0,1]` modulates:

- Intensity of post-processing (bloom, saturation, grain)
- Colour-grade target (cool desaturation for routine; warmer + higher contrast as stakes rise)
- Paneling aggressiveness (soft splits at low stakes; hard hatched splits at high)
- Overlay text weight + duration
- Hold timing on `aftermath-freeze` (0.5s routine → 2.5s cup final)

## Memory modulation

The memory ledger is queried per shot for "does any visible participant have relevant prior salience?" Hits trigger subtle visual + textual callbacks:

- Rim-light + name-tag "formerly Oldtown Rangers" on ex-player duels
- Pre-match overlay quotes from past ledger entries
- Cup-final `aftermath-freeze` surfaces "his first cup final since the 2031 humiliation" if applicable

Memory modulation is CHEAP (lookup + tag) but massively legible. Players SEE the callbacks without reading UI.

## MVP boundary

At Month 3 slice: 3 of 7 shot types (`tactical-wide`, `diagonal-attack-lane`, `pass-shot-impact`). Minimum stakes modulation (just routine vs not-routine). No memory modulation yet.

At Month 12 EA: all 7 shot types. Full stakes modulation. Full memory modulation. ~15 overlay text templates per shot type for variety.

## Deferred

- Per-signature unique cinematics (map into existing 7 shots)
- Motion capture (not part of the Phase-5/6 3D candidate stack; AI-assisted animation spike owns that decision)
- Replay scrubbing / social export (post-EA)
- User-configurable camera preferences (post-EA)

## Resolved (2026-04-24)

See SPEC.md decisions log entry `2026-04-24 — Semantic Cinema open questions resolved`.

### Q1 — Shot-count (7) locked through Month-3 gate; post-gate review, not an open question

The 7-shot vocabulary is locked through the Month-3 match-engine gate. Expansion beyond 7 requires a **superseding SPEC decision** — do not quietly grow the vocabulary during Phase 3 prototyping.

After the Month-3 gate, review whether the 3-shot prototype (`tactical-wide`, `diagonal-attack-lane`, `pass-shot-impact`) felt **thin**, **busy**, or **correctly scoped** before implementing the remaining 4. This is a gate-triggered review point, not a reopening of the count.

### Q2 — Camera authoring: Addressable `ShotTypeSO` assets

Each shot type is encoded as a **ScriptableObject asset, loaded via Addressables**, with content-pack-qualified stable IDs. UI Toolkit (UXML / USS) is for overlay composition only — not for camera framing.

Phase-2 ADR required: **ShotTypeSO schema + Addressables grouping.** Draft below is the Phase-0 lock; Phase-2 ADR finalizes the exact type definitions + grouping rules.

**ShotTypeSO draft schema:**

```
ShotTypeSO {
  id: ContentPackQualifiedId      // stable, mod-ready
  shot_category: enum {
    tactical-wide,
    diagonal-attack-lane,
    player-isolation,
    duel-panel,
    pass-shot-impact,
    crowd-reaction,
    aftermath-freeze
  }

  framing: {
    pitch_tilt_degrees: f32
    camera_distance: f32
    target_anchor_rule: enum { ball, scorer, duel-midpoint, crowd-section, fixed }
  }

  modulation_strength: {
    stakes: f32  [0,1]            // how strongly stakes warps this shot
    memory: f32  [0,1]            // how strongly memory callbacks warp this shot
    crowd:  f32  [0,1]            // how strongly crowd state tints this shot
  }

  chain_rules: [
    {
      next_shot_category: enum,
      condition: ChainCondition,  // e.g., "goal-scored", "near-miss", "high-salience-callback-hit"
      min_ticks: i32              // 60Hz-step minimum hold before transitioning
      max_ticks: i32
    }
  ]

  fallback_shot_category: enum    // if chain condition fails / no rule matches

  reduce_motion_variant: {
    impact_flash: bool             // disable fullscreen white-flash
    motion_lines: bool             // disable trail meshes
    panel_split_hold_ticks: i32    // longer hold, no hard hatching
  }

  default_hold_ticks: i32          // 60Hz-step default
  max_hold_ticks: i32

  overlay_template_set: [TemplateId]  // references templates, not inline text
}
```

**Why the additions past the obvious framing fields:**
- `chain_rules` makes the `pass-shot-impact → crowd-reaction → aftermath-freeze` cascade **data-driven**. Without this, shot chaining becomes hardcoded glue and the "semantic cinema grammar" quietly collapses into C#.
- `fallback_shot_category` ensures the renderer always has a shot to fall back to when no rule fires — prevents the "frozen camera" failure mode.
- `reduce_motion_variant` is accessibility-first. Impact flashes + motion lines are accessibility debt fast; every shot ships with a reduce-motion variant baked in, not bolted on.
- `modulation_strength` lets shots opt out of heavy stakes/memory warping — `tactical-wide` probably has low stakes-strength (it should stay readable even in a cup final); `aftermath-freeze` is the inverse.

### Q3 — Rendering stack

Screen-tone: **URP custom fullscreen HLSL pass.** Scene-global, stakes-modulated.
Impact frame: **separate URP fullscreen HLSL pass.** Scene-global, event-triggered.
Motion lines: **per-player trail mesh / sprite trail**, velocity-driven. Agent-local.
Panel splits / text composition: **UI Toolkit overlay elements**, with masks / textures where they behave. If UI Toolkit masking fights a given effect, fall back to a **custom mesh overlay** for that specific panel — do not over-commit to USS masks as the only mechanism.

Phase-2 ADR required: **viewer rendering pipeline + URP custom-pass ordering.**

| Effect | Technique | Scope |
|---|---|---|
| Screen-tone pattern | URP custom fullscreen pass (HLSL) | scene-global, stakes-modulated |
| Impact-frame flash | URP custom fullscreen pass (HLSL) | scene-global, event-triggered |
| Motion-line trails | per-sprite / per-player trail mesh | agent-local, velocity-driven |
| Panel splits / text | UI Toolkit overlay (masks/textures if they behave; custom mesh fallback if not) | UI layer |

### Q4 — Typography stack (Phase-3 provisional)

| Typeface | Role | Where it appears |
|---|---|---|
| **Anton** | Display impact / headlines | Goal splash, full-time graphic, `aftermath-freeze` headline text, pre-match splash |
| **JetBrains Mono** | Data / stat / scout / debug | In-match stat tickers, player-isolation stat overlays, scout labels, debug HUD |
| **Rajdhani** | Body / commentary / menu | Inline commentary lines, memory-callback tags ("formerly Oldtown Rangers"), menu body, tooltips |

**Scoreline override:** the persistent always-on scoreboard uses **Rajdhani SemiBold** or **JetBrains Mono** for digits, **not Anton**. Anton is too condensed for small-footprint always-on UI; it keeps its impact role for splash / aftermath moments.

Font licensing — all three typefaces ship under **SIL OFL 1.1** (Anton / JetBrains Mono / Rajdhani). JetBrains Mono's source-code repository is Apache-2.0 separately; only the typeface files ship with the game. Verified in `steam-release/asset-licensing-tracker.csv` 2026-04-24, not taken on reputation.

## Prototype gate

**Phase 3 Week 3:** 3-shot-type prototype rendered from a real MatchSim event stream.
**Phase 3 Week 4 (Month-3 gate):** external-observer legibility — cold observers describe drama + momentum + one player's style from watching 3 minutes of 3-shot output.

Pass = scale to 7 shots in Phase 5. Fail = extend Phase 3, do not add shots until 3-shot feels right.

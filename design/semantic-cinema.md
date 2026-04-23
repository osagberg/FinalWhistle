---
description: 2D viewer 7-shot-type camera grammar. How MatchSim events render as manga-broadcast cinema.
last_verified: 2026-04-22
status: scaffolded; awaiting Phase 2 lock
---

# Semantic Cinema — 2D viewer grammar

## Purpose

Answer "how does a deterministic MatchSim event stream render as a watchable 2D match that reads as drama, not data?"

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
- Motion capture (no 3D at MVP)
- Replay scrubbing / social export (post-EA)
- User-configurable camera preferences (post-EA)

## Open questions (Phase 2 lock)

1. **Are 7 shot types the right count?** GPT-5.5 locked 7 as minimum-viable. Too few risks repetition; too many risks UX complexity + authoring bloat. Review after Month-3 slice: does 3-shot slice feel thin?
2. **Camera authoring format** — how is each shot-type composition encoded? ScriptableObject with Cinemachine-like parameters, or UXML layout with named camera positions? Recommend SO for runtime data-driven selection.
3. **Screen-tone + motion-line rendering technique** — custom HLSL fullscreen pass vs per-sprite overlay? Recommend custom HLSL pass for screen-tone (cheap, uniform); per-sprite motion-line trails for runs.
4. **Text overlay typography hierarchy** — Anton for shot-type-defining text, JetBrains Mono for stat callouts, Rajdhani for body. Confirm or revise at Phase 2.

## Prototype gate

**Phase 3 Week 3:** 3-shot-type prototype rendered from a real MatchSim event stream.
**Phase 3 Week 4 (Month-3 gate):** external-observer legibility — cold observers describe drama + momentum + one player's style from watching 3 minutes of 3-shot output.

Pass = scale to 7 shots in Phase 5. Fail = extend Phase 3, do not add shots until 3-shot feels right.

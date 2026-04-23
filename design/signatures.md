---
description: 24-signature catalog. 3 per role family. Football-readable behaviors + trigger conditions + sim bias + presentation recipe + counterplay.
last_verified: 2026-04-22
status: scaffolded; awaiting Phase 2 catalog draft
---

# Signatures — player identity as learned match actions

## Purpose

Answer "what mechanically distinguishes one player from another in the way they actually play on the pitch, beyond stat numbers?"

## Locked decisions

See SPEC.md 2026-04-22. Summary:

- **24 pre-authored signatures.** Not composable atoms.
- **3 per role family × 8 role families.**
- **Each signature = role-specific football behavior + trigger conditions + sim bias + execution modifier + presentation recipe + counterplay.**
- **No power names.** UI surfaces football-readable copy ("Looks for early crosses" / "Arrives late in the box"). Never "Savant Shot" / "Weapon" / "Signature Move™" branding in player-facing text.
- **Earned, not stat-assigned.** A player's signature-readiness grows under specific trigger conditions; breakthrough moments unlock them.

## Role families (8)

1. Goalkeeper
2. Centre-back
3. Full-back / wing-back
4. Defensive / holding midfielder
5. Central midfielder
6. Attacking midfielder / #10
7. Winger
8. Striker / centre-forward

## 24-signature catalog (draft proposal — Phase 2 lock)

### Goalkeeper (3)

1. **Commands his area** — comes for every cross within 6-yard-box range; sim bias: +cross-claim probability; trigger: consecutive successful claims builds readiness; counterplay: attackers drag GK with decoy runs
2. **Fast long release** — throws long to winger within 2 seconds of possession; sim bias: +transition-trigger; trigger: fast-break situations played; counterplay: press the release target
3. **Reads the set piece** — positions 1-2 yards off typical for corner kicks, times reaction 0.2s earlier; sim bias: +set-piece save prob; trigger: successful set-piece reads accumulate

### Centre-back (3)

4. **Front-foot interception** — steps out of line to intercept through-ball; sim bias: +interception pre-commitment; trigger: successful interceptions in risky zones; counterplay: run-in-behind variations
5. **Back-post header** — attacks back post on attacking set pieces; sim bias: +xG on back-post crosses; trigger: time in final third on set plays; counterplay: mark back post tightly
6. **Calls the line** — co-ordinates offside trap, triggers line pushes; sim bias: +defensive-shape coherence for partners; trigger: minutes as captain / vocal role

### Full-back / wing-back (3)

7. **Underlap into cutback lane** — runs inside of winger to arrive in cutback zone; sim bias: +cutback xAssist; trigger: minutes with winger partner; counterplay: cover the half-space
8. **Recovery burst** — 5-10m sprint speed under chase-situation; sim bias: +defensive-recovery; trigger: times caught upfield and recovered; counterplay: exhaust stamina first
9. **Early whipped cross** — delivers cross before reaching byline; sim bias: +early-cross freq + xAssist from deep; trigger: minutes as wing-back role; counterplay: block crosses inside 18-yard line

### Defensive / holding midfielder (3)

10. **Screens the back four** — positions between attack and defense, intercepts shifted-ball; sim bias: +anticipation in half-space; trigger: minutes in DM role with coaching emphasis; counterplay: play around via wingbacks
11. **Tactical foul recognition** — commits smart fouls to stop counters; sim bias: +foul in transition; trigger: yellow-but-not-red discipline history; counterplay: free-kick specialist in opposition
12. **Breaks lines** — passes vertically through press lines; sim bias: +line-breaking-pass freq; trigger: minutes in possession-system team; counterplay: tight press on DM

### Central midfielder (3)

13. **First-time diagonal switch** — plays cross-field switch ball one-touch; sim bias: +diagonal-switch trigger; trigger: successful switches built up over time; counterplay: compress the pitch
14. **Late arriving in the box** — times forward run into box for crosses; sim bias: +late-box-arrival xG; trigger: minutes in box-to-box role; counterplay: track the runner
15. **Press trigger** — initiates team press on cue (opponent back-pass, poor touch); sim bias: +press-intensity when triggered; trigger: minutes in high-press system; counterplay: play out from back cleanly

### Attacking midfielder / #10 (3)

16. **Finds the half-space** — drifts into half-space pocket between FB and CB; sim bias: +half-space reception freq; trigger: minutes as #10; counterplay: hand-off assignment
17. **Through-ball vision** — plays defense-splitting through-balls; sim bias: +through-ball xAssist; trigger: successful through-balls history; counterplay: high defensive line
18. **Pre-shot feint** — dummies shot to sell defender; sim bias: +shot-cut-back sequences; trigger: minutes in creator role; counterplay: don't commit on first move

### Winger (3)

19. **Cuts inside on weaker foot** — drives from outside to inside to shoot / pass; sim bias: +cut-inside trigger freq; trigger: minutes in inverted-winger role; counterplay: show him down the line
20. **Early whipped cross** (winger variant) — crosses early from deep width; sim bias: +early-cross freq; trigger: minutes as traditional winger; counterplay: block crosses, overload far post
21. **Takes the fullback on 1v1** — isolation dribble; sim bias: +1v1-duel win rate; trigger: 1v1 duel history; counterplay: double-team

### Striker / centre-forward (3)

22. **Blind-side near-post run** — curves run off defender's shoulder to near post; sim bias: +near-post xG; trigger: minutes as #9; counterplay: zonal-mark near post
23. **Drops deep to link** — comes short to receive between lines; sim bias: +drop-deep freq + link-up xAssist; trigger: minutes as false-9; counterplay: CBs follow him; DMs cover
24. **Finishes first-time** — prefers one-touch finish, won't set up; sim bias: +first-time-finish xG; trigger: first-time-finish history; counterplay: deny the decisive pass

## Signature lifecycle

1. **Latent** — generated player has 1-3 signature affinities in Identity Packet; none active
2. **Earning** — trigger conditions accumulate `signature_readiness ∈ [0, 1]` float per affinity
3. **Breakthrough** — at readiness threshold (default 0.85), a triggered match event awakens the signature (see `breakthrough-moments.md`)
4. **Active** — signature is now visible on player card; sim bias + presentation recipe apply
5. **Evolving (deferred)** — post-MVP, awakened signatures can evolve via continued use

## Signature data shape (Phase 2 lock)

```
Signature {
    id: stable string
    role_family: enum
    display_name: football-copy text ("Looks for early crosses")
    ui_description: short text ("Delivers cross before reaching byline")
    trigger_conditions: [  // all AND'd
        { kind: "minutes_in_role", role: "WB", threshold: 900 }
        { kind: "event_count", event: "successful_early_cross", threshold: 8 }
    ]
    sim_bias: {  // numerical biases applied to MatchSim when signature active
        early_cross_freq: +0.15
        xA_from_deep: +0.08
    }
    execution_modifier: {  // how ball physics is biased during signature execution
        curve_multiplier: 1.2
        power_variance: -0.05  // more consistent
    }
    presentation_recipe: {
        shot_type_preference: ["player-isolation", "pass-shot-impact"]
        overlay_text_bank: [  // for stake-modulated selection
            "He looks for the early ball.",
            "Cross comes in before the byline.",
            "He whips it in first time."
        ]
    }
    counterplay: [
        { kind: "team_instruction", instruction: "Block crosses inside 18" }
    ]
}
```

## MVP boundary

At Month 3 slice: 3 signatures authored end-to-end, representing 3 role families. Full lifecycle (latent → earning → breakthrough → active) demonstrable.

At Month 5 vertical slice: 12 signatures authored (1-2 per role family, broad coverage).

At Month 12 EA: all 24 signatures authored, balance-harness-tuned, UI copy reviewed.

## Deferred

- Signature evolution (awakened-form stages) — post-MVP
- User-authored signatures via content packs — Workshop post-EA
- Per-signature unique cinematics beyond shot-type mapping — no, they use the 7-shot vocabulary
- Composable signature atoms — rejected

## Open questions (Phase 2 lock)

1. **Exact signature catalog** — above is draft proposal; user review required before Phase 2 lock.
2. **Latent-affinity count per player** — 1-3 signature affinities per Identity Packet; what decides the count? Proposal: internal gene model weights (see `player-generation.md`).
3. **Multi-signature interaction** — when a player awakens 2-3 signatures over a career, do they compound or conflict? Proposal: compound with diminishing returns; no hard conflict rules at MVP.
4. **Readiness thresholds** — default 0.85; per-signature tuning via balance harness?
5. **Counterplay surfacing** — how is counterplay revealed to the player? Opponent scout reports? Tactical opposition-analysis UI? Recommend scout reports at MVP.

## Prototype gate

**Phase 3 Week 4 (Month-3 slice):** 3 signatures fully playable in the slice. Each has visible sim bias + stylized presentation via the 3 shot types. At least one triggers a breakthrough in the single-match slice.

**Phase 5 gate:** 12 signatures playable. Balance harness confirms no signature dominates or bricks player's game.

**Phase 6 gate:** all 24 signatures authored; counterplay viability confirmed via harness (signatures can be countered by tactical choices).

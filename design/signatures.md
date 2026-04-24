---
description: 24-signature catalog. 3 per role family. Football-readable behaviors + trigger conditions + sim bias + presentation recipe + counterplay.
last_verified: 2026-04-24
status: Phase 0 open questions resolved; 24-sig catalog locked with dependency metadata, #19 + #6 edits applied, field-level stacking, tier-weighted affinity distribution, scout-report counterplay surface. One Phase-2 ADR pre-seeded.
---

# Signatures — player identity as learned match actions

## Purpose

Answer "what mechanically distinguishes one player from another in the way they actually play on the pitch, beyond stat numbers?"

## Locked decisions

See SPEC.md 2026-04-22. Summary:

- **24 pre-authored signatures.** Not composable atoms.
- **3 per role family × 8 role families.**
- **Each signature = role-specific football behavior + trigger conditions + sim bias + execution modifier + presentation recipe + counterplay.**
<!-- ui-lint:ignore-start reason="locked decision naming banned UI-power-name vocabulary" -->
- **No power names.** UI surfaces football-readable copy ("Looks for early crosses" / "Arrives late in the box"). Never "Savant Shot" / "Weapon" / "Signature Move™" branding in player-facing text.
<!-- ui-lint:ignore-end -->
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
3. **Reads the set piece** — positions 1-2 yards off typical for corner kicks, times reaction 0.2s earlier; sim bias: +set-piece save prob; trigger: successful set-piece reads accumulate; depends on: Phase-4 set pieces

### Centre-back (3)

4. **Front-foot interception** — steps out of line to intercept through-ball; sim bias: +interception pre-commitment; trigger: successful interceptions in risky zones; counterplay: run-in-behind variations
5. **Back-post header** — attacks back post on attacking set pieces; sim bias: +xG on back-post crosses; trigger: time in final third on set plays; counterplay: mark back post tightly; depends on: Phase-4 set pieces
6. **Calls the line** — co-ordinates offside trap, triggers line pushes; sim bias: +defensive-shape coherence for partners; **scope:** `defensive_line` (authored from one player's identity, effect applies to the defensive unit — not a global team buff); trigger: minutes as captain / vocal role; depends on: Phase-4 shape-coherence scoring

### Full-back / wing-back (3)

7. **Underlap into cutback lane** — runs inside of winger to arrive in cutback zone; sim bias: +cutback xAssist; trigger: minutes with winger partner; counterplay: cover the half-space
8. **Recovery burst** — 5-10m sprint speed under chase-situation; sim bias: +defensive-recovery; trigger: times caught upfield and recovered; counterplay: exhaust stamina first
9. **Early whipped cross** — delivers cross before reaching byline; sim bias: +early-cross freq + xAssist from deep; trigger: minutes as wing-back role; counterplay: block crosses inside 18-yard line

### Defensive / holding midfielder (3)

10. **Screens the back four** — positions between attack and defense, intercepts shifted-ball; sim bias: +anticipation in half-space; trigger: minutes in DM role with coaching emphasis; counterplay: play around via wingbacks
11. **Tactical foul recognition** (UI copy: *"Stops counters early"*) — commits smart fouls to stop counters; sim bias: +foul in transition; trigger: yellow-but-not-red discipline history; counterplay: free-kick specialist in opposition; depends on: Phase-4 fouls + cards
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

19. **Cuts inside onto his stronger foot** — drives from outside to inside to shoot / pass (inverted winger on the opposite flank from stronger foot); sim bias: +cut-inside trigger freq; trigger: minutes in inverted-winger role; counterplay: show him down the line
20. **Low cutback from the byline** — drives outside and cuts the ball back low from the byline; sim bias: +cutback chance after successful wide carry; trigger: byline entries as winger; counterplay: double up wide and protect penalty spot
21. **Takes the fullback on 1v1** — isolation dribble; sim bias: +1v1-duel win rate; trigger: 1v1 duel history; counterplay: double-team

### Striker / centre-forward (3)

22. **Blind-side near-post run** — curves run off defender's shoulder to near post; sim bias: +near-post xG; trigger: minutes as #9; counterplay: zonal-mark near post
23. **Drops deep to link** — comes short to receive between lines; sim bias: +drop-deep freq + link-up xAssist; trigger: minutes as false-9; counterplay: CBs follow him; DMs cover
24. **Finishes first-time** — prefers one-touch finish, won't set up; sim bias: +first-time-finish xG; trigger: first-time-finish history; counterplay: deny the decisive pass

<!-- ui-lint:ignore-start reason="technical description of the awakening lifecycle mechanic; internal design-doc prose, not player-facing" -->
## Signature lifecycle

1. **Latent** — generated player has 1-3 signature affinities in Identity Packet; none active
2. **Earning** — trigger conditions accumulate `signature_readiness ∈ [0, 1]` float per affinity
3. **Breakthrough** — at readiness threshold (default 0.85), a triggered match event awakens the signature (see `breakthrough-moments.md`)
4. **Active** — signature is now visible on player card; sim bias + presentation recipe apply
5. **Evolving (deferred)** — post-MVP, awakened signatures can evolve via continued use
<!-- ui-lint:ignore-end -->

## Signature data shape (Phase 2 lock)

Full schema + content-pack grouping + stacking-policy-per-field finalized in Phase-2 ADR (see SPEC).

```
Signature {
    id: ContentPackQualifiedId            // stable, mod-safe
    role_family: enum
    display_name: football-copy text      // "Looks for early crosses" — player-facing
    ui_description: short text            // "Delivers cross before reaching byline"

    scope: enum {                          // what the signature's effect reaches
        player,                            // default: self-only
        defensive_line,                    // e.g. #6 "Calls the line"
        press_unit,                        // e.g. #15 "Press trigger"
        set_piece_context                  // e.g. #3, #5
    }

    dependencies: [SystemDependency]       // scheduling metadata, not gameplay
    // e.g. { system: "set_pieces", min_phase: 4 }
    //      { system: "fouls_and_cards", min_phase: 4 }
    //      { system: "defensive_shape_coherence", min_phase: 4 }

    readiness_threshold: f32 [0,1]         // default 0.85 via project-wide constant;
                                           // per-signature override allowed

    trigger_conditions: [  // all AND'd
        { kind: "minutes_in_role", role: "WB", threshold: 900 },
        { kind: "event_count", event_class: SignatureExecuted_Candidate, threshold: 8 }
    ]

    sim_bias: [   // list of field-level effects, each with a stacking policy
        {
            field: "early_cross_freq",
            delta: +0.15,
            stacking: {
                mode: enum { additive, additive_with_diminishing_returns },
                min_delta: -0.50,          // hard lower cap regardless of stacks
                max_delta: +0.50,          // hard upper cap regardless of stacks
                diminishing_curve: optional // e.g. { per_additional_stack: 0.5 }
            }
        },
        // ...
    ]

    execution_modifier: {  // how ball physics is biased during signature execution
        curve_multiplier: 1.2,
        power_variance: -0.05              // more consistent
    }

    presentation_recipe: {
        shot_type_preference: ["player-isolation", "pass-shot-impact"],
        overlay_text_bank: [                // for stake-modulated selection
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

## Signature stacking policy

<!-- ui-lint:ignore-start reason="technical stacking-policy description; internal prose about the awakening mechanic" -->
When a player awakens 2+ signatures whose `sim_bias` entries target the same MatchSim field, effects **stack under the field's stacking policy** — not under a single generic rule.
<!-- ui-lint:ignore-end -->

**Rules:**
1. **Additive by default.** Each active signature contributes its `delta` to the field's running total.
2. **Diminishing returns where tagged.** Fields prone to runaway (`first_time_finish_xG`, `1v1_duel_win_rate`) declare `additive_with_diminishing_returns` with a `diminishing_curve` that de-weights each additional stack.
<!-- ui-lint:ignore-start reason="technical stacking-cap rule describing the awaken mechanic" -->
3. **Hard per-field caps.** Every `sim_bias` field declares `min_delta` + `max_delta` bounds. The summed post-stacking value is clamped to those bounds. No signature configuration — no matter how many awaken — can push a field past its cap.
<!-- ui-lint:ignore-end -->
<!-- ui-lint:ignore-start reason="technical rule about signature awakening behaviour" -->
4. **No hand-authored conflict rules at MVP.** Two signatures that logically disagree both fire under their own trigger contexts; the player behaves as each context dictates. Phase-6 balance harness flags any overlap that reliably breaks the game; narrow conflict rules are added then, not now.
<!-- ui-lint:ignore-end -->
5. **Balance-harness CI check:** every `sim_bias` field gets a sweep across plausible signature-overlap configurations; fields that breach caps without clamping or produce dominant strategies are flagged.

**Why not softmax:** softmax is a categorical-probability tool; we need scalar clamping per field, not probability normalization. Field-level caps are more debuggable and explicitly list-authored per signature field.

## Affinity distribution (cross-doc: see `design/player-generation.md`)

Each generated player's Identity Packet rolls `affinity_count ∈ {0, 1, 2, 3}` with a **power-law tail** — most players have 1 affinity, rare players have 3.

**The roll is tier-weighted, not uniform:**

- **Top-flight starters** rarely roll 0. Zero-affinity players are not supposed to feel like everyday Premier-equivalent players.
- **Lower-tier players, depth squads, late-career journeymen, low-ceiling generated cohorts** carry the bulk of the 0-affinity mass.
- **3-affinity players are rare across every tier** — they're the once-in-a-generation players the ledger remembers forever.

**Phase-6 tuning seeds (NOT SPEC — authoritative in `design/player-generation.md`):**
| Cohort | P(0) | P(1) | P(2) | P(3) |
|---|---|---|---|---|
| Top-flight starter | 0.02 | 0.60 | 0.32 | 0.06 |
| Mid-tier starter | 0.08 | 0.62 | 0.25 | 0.05 |
| Lower-tier / depth / journeyman | 0.20 | 0.60 | 0.18 | 0.02 |

Overall population P(0) ≈ 0.10 falls out of the weighted distribution without forcing top-flight rosters to feel signatureless.

## MVP boundary

At Month 3 slice: 3 signatures authored as active behaviors, representing 3 role families. The slice proves signatures alter MatchSim choices and 2D presentation. Full lifecycle (latent → earning → breakthrough → active) begins in Phase 4.

At Month 5 vertical slice: 12 signatures authored (1-2 per role family, broad coverage).

At Month 12 EA: all 24 signatures authored, balance-harness-tuned, UI copy reviewed.

<!-- ui-lint:ignore-start reason="deferred-item description of awakened mechanic" -->
## Deferred

- Signature evolution (awakened-form stages) — post-MVP
<!-- ui-lint:ignore-end -->
- User-authored signatures via content packs — Workshop post-EA
- Per-signature unique cinematics beyond shot-type mapping — no, they use the 7-shot vocabulary
- Composable signature atoms — rejected

## Resolved (2026-04-24)

See SPEC.md decisions log entry `2026-04-24 — Signature system open questions resolved`. Phase-2 ADR pre-seeded.

1. **24-signature catalog locked** with dependency metadata. No rotations. Two catalog edits applied:
   - **#19** → "Cuts inside onto his stronger foot" (was "weaker foot" — football-wrong for inverted winger)
   - **#6 Calls the line** scoped as `defensive_line`, not a global team buff — authored from one player's identity, effect applies to the unit
   - **#11** alternate player-facing copy *"Stops counters early"* noted for internal name "Tactical foul recognition"
   - **Dependency tags on #3, #5, #6, #11** — scheduling metadata for Phase-4+ system availability (set pieces, fouls/cards, defensive-shape coherence). Not a reason to weaken the 24.
2. **Affinity distribution** follows a **power-law tail weighted by generation tier** (cohort, age, role, ceiling). See the "Affinity distribution" section above. Zero-affinity players cluster in lower tiers, depth squads, late-career journeymen — not top-flight starters.
3. **Multi-signature stacking** uses **field-level capped policies**, NOT a generic softmax. Each `sim_bias` field declares additive vs additive-with-diminishing-returns mode + hard `min_delta` / `max_delta` caps. Balance harness sweeps for broken overlaps in Phase 6. No hand-authored conflict rules at MVP.
4. **Readiness threshold** — rule locked (default threshold with per-signature override, tuned by balance harness). Numeric default (`0.85`) is a design-doc starting value, NOT SPEC-locked.
5. **Counterplay surfaces through scout reports** — and ONLY for **observed / scouted signatures**, never latent affinities. Works with the Scout Disagreement system if it passes the Month-4 gate, and with basic scouting (simpler certainty levels) if it doesn't. Same scout-report UI surface either way.

## Prototype gate

**Phase 3 Week 4 (Month-3 slice):** 3 signatures fully playable in the slice. Each has visible sim bias + stylized presentation via the 3 shot types. No breakthrough lifecycle required yet.

**Phase 5 gate:** 12 signatures playable. Balance harness confirms no signature dominates or bricks player's game.

**Phase 6 gate:** all 24 signatures authored; counterplay viability confirmed via harness (signatures can be countered by tactical choices).

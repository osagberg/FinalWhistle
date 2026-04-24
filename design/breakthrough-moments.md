---
description: Match-flow cinematic development triggers. Signature awakenings + latent-allele triggers that permanently change a player, without match-pause QTE.
last_verified: 2026-04-24
status: Phase 0 open questions resolved; cinema duration + text-tier + near-miss + regressive-parity + pillar-tiebreaker interaction locked. No new ADR — composes existing schemas.
---

# Breakthrough Moments — development as cinema

## Purpose

Answer "how do players permanently change mid-career via in-match events, in a way that feels earned and football-grounded, without QTE pop-ups pulling the player out of the management fantasy?"

## Locked decisions

See SPEC.md 2026-04-22. Summary:

- **Match-flow cinematic, NOT pause-QTE.** Sim continues deterministically. Viewer punches in with short panel/impact beat. Post-match report confirms development change. No mid-match decision window.
- **Manager influence happens indirectly** — via tactics, selection, training, promises made, pressure exposure over seasons. Not mid-match pop-ups.
- **Permanent.** Awakenings are irreversible. Regressive triggers (bust, fragile, glass-ceiling-lock) are also permanent in the other direction.

## Trigger kinds

### Kind 1 — Signature awakening

Player has latent signature affinity with `signature_readiness ∈ [0, 1]` accumulating from trigger-condition events over seasons. At `readiness >= 0.85` AND a matching in-match situation occurs (e.g., winger at 0.87 readiness on "early-whipped-cross" attempts a cross-in-deep-width situation), a cinematic awakening triggers:

- `player-isolation` shot on the player
- `pass-shot-impact` on the action
- `crowd-reaction` cutaway
- `aftermath-freeze` with overlay text ("He's found something.")
- Post-match report emits a MemoryEvent `SignatureAwakened`
- Signature now active on player card

### Kind 2 — Latent potential trigger

Identity Packet may contain latent-potential flags. These are internal narrative triggers, never player-facing mystical terms:

- `late_bloomer` — ceiling raises after specific event (e.g., scoring decisive goal in relegation-6-pointer)
- `dormant_pressure_composure` — pressure-response curve raises after surviving chosen high-pressure match
- `flow_access` — enables sustained-readiness states in high-stakes contexts after first qualifying match

Triggers match events in the ledger. When conditions match, a quiet panel beat + post-match emission. No "THE HUSH UNLOCKED" popup. Instead: "Something clicked today. Scouts will revise."

### Kind 3 — Regressive triggers (negative breakthroughs)

Same mechanic in reverse. Long-accumulated physical-load-without-relief / confidence-collapse / injury-chain produces permanent negative change:

- `fragile` locks in after 3 consecutive recurrences of specific injury class
- `confidence_fractured` after specific chain of humiliations
- `ceiling_compressed` after age + deterioration conditions

Presented with equal gravity via cinema: `aftermath-freeze` with muted palette + overlay ("He's not the same since the derby."). Emit negative MemoryEvent.

## Manager influence (indirect only)

Managers shape breakthrough likelihood through:

- **Tactics** — playing a young CM in a box-to-box role accumulates "late arriving in the box" readiness
- **Selection** — minutes in role matter; rotating a player out of his affinity role stalls readiness
- **Training** — training focus biases readiness accumulation toward specific signatures
- **Promises** — ledger-tracked PromiseMade events affect confidence curve
- **Pressure exposure** — playing a player in high-stakes matches builds or fractures composure depending on outcomes + composure-floor gene

No mid-match pop-up ever lets the player "choose" a breakthrough. The manager earns it over sessions of right/wrong choices.

## MVP boundary

At Month 3 slice: no full breakthrough lifecycle. The slice may include one simple persistent post-match development event, but breakthrough triggering, cinematic emphasis, and latent unlock rules start in Phase 4.

At Month 12 EA:
- All three trigger kinds operational
- 5-10 latent-potential trigger classes
- Balance-harness-tuned awakening pacing (so awakenings feel earned, not random)
- Salience-gated so breakthroughs contribute to the 5-8-events-per-season ceiling

## Deferred

- Signature evolution (awakened signature evolves via continued use) — post-MVP
- Awakenings triggering rivalry / relationship events with other players — post-MVP
- External trainers / mentors triggering specific awakenings — post-MVP

## Resolved (2026-04-24)

See SPEC.md decisions log entry `2026-04-24 — Breakthrough Moments open questions resolved`. No new ADR — composes ShotTypeSO (chain_rules), SignatureSO (readiness threshold), MemoryEvent emission, and `ui-vocabulary.md` lint.

### Q1 — Cinema beat duration

**3-5s range locked; default Phase-3 tuning seed 3s; longer durations earned through stakes.**

8s dropped from consideration — even cup-final-scale cinema at 8s reads as "the game paused to tell me a thing." 5s is reserved for genuinely high-stakes beats (cup-final breakthroughs, relegation-decider awakenings). Default 3s means ordinary-stakes breakthroughs stay close to broadcast goal-cutaway length.

Phase-3 Week 4 A/B-tests 3s / 4s / 5s variants against the Month-3 gate observers; final chosen value lives in the design doc as a Phase-3 tuning seed, not SPEC.

### Q2 — Overlay text tone

**Two-tier pattern with strict no-system-vocabulary rule.**

**Tier 1 — Quiet observational phrase** (fires with the panel beat):
> *"He's found something."* / *"That's new."* / *"Third time today."*

**Tier 2 — Match-specific follow-up** (fires in `aftermath-freeze` or post-match report):
> *"He cuts inside again — and this time he goes through."* / *"Mendez has been looking for that run all half."*

**Banned vocabulary (enforced via `design/ui-vocabulary.md` lint):**
- ~~"Signature unlocked"~~, ~~"Awakened"~~, ~~"The Hush"~~, ~~"Calling"~~, ~~"Canon"~~ — no mystical / capitalized state nouns
- ~~"XP gained"~~, ~~"Level up"~~, ~~"+5 finishing"~~ — no progression-mechanic menu vocabulary

Text describes football behavior, not progression mechanics. If copy could appear in a live broadcast commentator's line, it's probably right. If it could appear on a stat-sheet readout, it's probably wrong.

### Q3 — Near-miss handling

**Silent first near-miss; post-match stat-card after 2nd+ same-match near-miss.**

- **1st near-miss in a match:** silent. No stat-card. Readiness accumulates quietly as usual.
- **2nd+ near-miss same match:** post-match stat-card — *"Found the cutback position twice today. Not quite there yet."*
- **Never** a live-match "Close!" popup — that's the farming failure mode.

Explicit near-miss surfacing trains players to game the system (selecting for near-miss conditions rather than natural development). Silence-until-pattern keeps awakenings feeling earned.

### Q4 — Regressive trigger parity

**Same gravity as positive breakthroughs.** Same cinema duration range (3-5s), same shot chain, same two-tier text pattern, same post-match MemoryEvent emission weight. Visual tone modulates via existing semantic-cinema channels (muted palette, quieter crowd layer, `aftermath-freeze` overlay text from loss-toned templates).

Pillar-1 says *consequences stick*. A save where triumphs get 5-second cinema and ruinations get a stat-line is a save that remembers only the good bits. That's not the pillar.

### Q5 — Pillar-tiebreaker interaction (when a breakthrough triggers during live play)

Consequence of the 2026-04-24 overview lock: memory wins by default, but watchability temporarily wins in high-leverage live-match sequences, with callbacks deferred to the next natural surface. Breakthrough cinema is a memory-pillar write that interrupts the viewer, so the tiebreaker applies — with a narrow live-fire exception.

**Rule (locked):**

1. **Normal play (not a high-leverage sequence):** breakthrough cinema fires **at the next natural surface** — next dead ball → half-time → full-time / post-match report. Not interrupting live play.
2. **Terminal action of a high-leverage sequence:** if the triggering action IS the resolving beat — **the shot, save, tackle, or final pass that resolves the chance** — the cinema may fire **immediately after the action resolves**. The action earned the moment.
3. **Never interrupt an unresolved attacking or defensive sequence mid-flow.** Readiness crossing a threshold is not an excuse to freeze a live counter-attack. The sequence finishes; the beat follows if the resolving action earned it; otherwise defer to the next natural surface.
4. **Dead-ball breakthroughs fire immediately.** The natural surface already exists — a free kick, corner, throw-in, or goal-celebration beat IS the surface. No deferral needed.

The mistake to avoid is freezing a live counterattack because readiness accumulated during the sequence. The correct behavior lets the counterattack finish, then awards the cinema only if the resolving action itself earned it.

**Cross-ref:** this rule's implementation is a `chain_rules` condition on the shot SO's breakthrough-chain — `condition: resolving_action_of_sequence`. Data-driven, not hardcoded. See `design/semantic-cinema.md`.

## Prototype gate

**Phase 4 first-signatures gate:** one signature breakthrough triggers during a controlled match and the cinema beat works. External-observer gate: cold observer describes the moment as meaningful without being told it's a feature.

**Phase 5 vertical slice:** all 3 trigger kinds playable across a full season. Salience tuning in progress; awakenings feel "earned" per 5-tester judgment.

**Phase 6:** balance harness confirms awakening cadence (target: ~1-3 per player per career, not 15+).

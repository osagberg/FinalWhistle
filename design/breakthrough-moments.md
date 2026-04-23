---
description: Match-flow cinematic development triggers. Signature awakenings + latent-allele triggers that permanently change a player, without match-pause QTE.
last_verified: 2026-04-22
status: scaffolded; awaiting Phase 2 lock
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

## Open questions (Phase 2 lock)

1. **Cinema beat duration** — target 3-6 seconds. How disruptive is that to match-watching flow? Prototype in Phase 3 Week 4 against 3-second, 5-second, 8-second variants.
2. **Overlay text tone** — "He's found something." vs "He cuts inside again." vs "Third time today, and now it's his." Recommend two-tier: quiet observational phrase first, then match-specific follow-up. Avoid mystical framing ("awakened", "the hush", etc.).
3. **Failure modes** — a near-miss breakthrough (readiness 0.83 in qualifying situation) — silent? Small non-cinema stat-card acknowledgment? Recommend small stat-card acknowledgment so user feels progress.
4. **Regressive trigger visibility** — positive breakthroughs pause sim for 3-6s cinema; negative breakthroughs same? Or quieter? Recommend same gravity — memory Pillar 1 means bad events stick as emphatically as good.

## Prototype gate

**Phase 4 first-signatures gate:** one signature breakthrough triggers during a controlled match and the cinema beat works. External-observer gate: cold observer describes the moment as meaningful without being told it's a feature.

**Phase 5 vertical slice:** all 3 trigger kinds playable across a full season. Salience tuning in progress; awakenings feel "earned" per 5-tester judgment.

**Phase 6:** balance harness confirms awakening cadence (target: ~1-3 per player per career, not 15+).

---
description: Top-level game overview, pillars, experience goals, 4-bucket scope split pointer.
last_verified: 2026-04-24
status: Phase 0 open questions resolved; pillar tiebreaker + quickstart-archetype count + title + nation-framing locked
---

# Final Whistle — Overview

## Purpose

Answer the question "what game are we making?" at a level every sub-system design doc can reference. This is the top-of-tree doc; everything else specializes.

## Locked decisions

See SPEC.md 2026-04-22 entries. Summary:

- Football management RPG with RPG-depth player development + event-sourced career memory
- Fully fictional football world, England-readable grammar
- 2D stylized manga-broadcast match viewer (not 3D, not a waypoint to 3D)
- Tone: Giant Killing + Aoashi + occasional anime exaggeration
- No capitalized state nouns in player-facing UI
- PEGI 12 / ESRB T; Steam PC only; Windows + Mac + Linux
- $20 EA → $30 1.0; Month-12 EA target

## Pillars

Three pillars drive every design trade-off. When systems conflict, the one that better serves a pillar wins.

### Pillar 1 — Careers that remember

The game's one unambiguous promise is that consequences stick. A 17-year-old you sold can return as a rival's captain. A derby you threw for the league cup haunts your next contract talks. A cup final gamble gets cited by journalists a decade later. **Mechanically, every meaningful event is a structured record in an append-only ledger**; surfacing systems read the ledger and produce callbacks.

### Pillar 2 — Players are specific

<!-- ui-lint:ignore-start reason="technical prose describing how signatures awaken; internal design-doc, not player-facing" -->
FM's core complaint is "every player feels the same." Final Whistle's signature system + internal gene model + identity-packet compiler ensure every player has a distinct playing-style coherence: one is a late-bloomer ball-striker who needs confidence minutes before his signature awakens; another is a clinical set-piece natural with fragile composure. **Mechanically, the identity packet couples playing instincts + pressure response + development hooks + signature affinities + scout labels into a single data shape per player**.
<!-- ui-lint:ignore-end -->

### Pillar 3 — Every match is watchable

FM's 3D match is skip-after-10-minutes. Final Whistle's viewer uses a renderer-agnostic 7-shot semantic-cinema grammar + state-driven color grading + stake-modulated intensity. Phase-3 proves it through the dots adapter; cel-shaded 3D remains a spike-gated candidate shipping layer. **Mechanically, every bridged ViewerEvent gets rendered through one of 7 shot types; stakes (cup final vs friendly) and memory state (does anyone in this moment have prior ledger relevance?) modulate intensity/paneling/timing/text**.

## MVP boundary (Month-12 EA)

In: all three pillars shipped with evidence via Month-12 brutal slice (see `month-3-vertical-slice.md` for the first proof).

Out at EA:
- Guaranteed 3D match engine (cel-shaded 3D ships only if the Phase-5/6 production-feasibility spike is green)
- Multi-nation pyramid
- Coaching Lineage surfacing (data seeded, exposed post-EA)
- Social-media / live-ops / server-side anything
- Mobile port

## Deferred (post-EA if audience signal passes)

- 3D cel-shaded match engine as v2.0 visual update
- Roguelike "Legend Run" condensed-career mode
- Multi-nation expansion
- Coaching Lineage full surfacing
- Workshop editor UX
- Counterfactual Development Lab
- Dynasty / lineage mechanics (if audience retains + requests)

## Resolved (2026-04-24)

See SPEC.md decisions log entry `2026-04-24 — Overview pillar questions resolved`.

1. **Nation framing.** Single named fictional nation with England-readable football grammar (6-tier pyramid, promotion/relegation, cup competitions, home/away fixtures). The actual nation name is owned by [`worldbuilding.md`](worldbuilding.md) — not duplicated here.
2. **Product title.** "Final Whistle" is locked as the working / product title. Formal trademark + Steam-name clearance is deferred to Phase 8 launch prep; known existing non-AAA uses include `finalwhistle.es` (daily football mini-game) and `finalwhistle.club` (football community product). Neither is a blocker; both are flags for the clearance pass.
3. **Quickstart club archetypes — 4 locked.**
   - Decaying giant, tier 2
   - Rising academy, tier 3
   - Mid-table survivalist, tier 1
   - Backs-against-the-wall, tier 5
4. **Pillar tiebreaker (P1 Memory vs P3 Watchability).** Memory wins by default. If a memory callback would interrupt a **high-leverage live match sequence**, watchability temporarily wins and the callback is queued to the next natural surface (dead ball, half-time, full-time, or post-match report). Callbacks are **deferred, never suppressed**.

   **High-leverage** = score margin ≤ 1 in the final 10 in-game minutes, **or** any cup / promotion / relegation / derby / title-deciding sequence.

## Prototype gate (for overall game feel)

The Month-3 match-engine gate (see `month-3-vertical-slice.md`) is the first test of whether the three pillars are even plausible simultaneously. Gate criterion: *"A stranger watches a 2D match for three minutes and understands drama, momentum, and player identity without reading a design doc."*

If that fails, we re-scope or cut before scaling systems. Every later feature depends on this gate passing.

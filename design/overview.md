---
description: Top-level game overview, pillars, experience goals, 4-bucket scope split pointer.
last_verified: 2026-04-22
status: scaffolded; awaiting Phase 0 open-question resolution
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

FM's core complaint is "every player feels the same." Final Whistle's signature system + internal gene model + identity-packet compiler ensure every player has a distinct playing-style coherence: one is a late-bloomer ball-striker who needs confidence minutes before his signature awakens; another is a clinical set-piece natural with fragile composure. **Mechanically, the identity packet couples playing instincts + pressure response + development hooks + signature affinities + scout labels into a single data shape per player**.

### Pillar 3 — Every match is watchable

FM's 3D match is skip-after-10-minutes. Final Whistle's 2D viewer uses a 7-shot semantic-cinema grammar + state-driven color grading + stake-modulated intensity. **Mechanically, every MatchSim event gets rendered through one of 7 shot types; stakes (cup final vs friendly) and memory state (does anyone in this moment have prior ledger relevance?) modulate intensity/paneling/timing/text**.

## MVP boundary (Month-12 EA)

In: all three pillars shipped with evidence via Month-12 brutal slice (see `month-3-vertical-slice.md` for the first proof).

Out at EA:
- 3D match engine
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

## Open questions (resolve before Phase 1)

1. **Which single fictional nation** anchors EA? Proposal: an original nation with England-readable league grammar (6-tier pyramid, promotion/relegation, cup competitions, home/away fixtures). Name TBD at `worldbuilding.md` lock. Alternative: no named nation (just "the league"). Recommend named.
2. **User-facing game title framing** — is "Final Whistle" locked? Alternatives considered and declined: "The Long Memory", "The Author", subtitles. Confirm lock.
3. **First-playable club archetype mix** — the Month-12 EA offers quickstart-club choice. How many archetypes? Proposal: 4 (decaying giant / rising academy / mid-table survivalist / struggling lower-tier). Confirm or revise.
4. **Explicit pillar tiebreaker** — if Pillar 1 (memory) and Pillar 3 (watchability) conflict (e.g., a callback disrupts a key match-moment), which wins? Proposal: Pillar 1 — memory always wins because it's the game's unique promise.

## Prototype gate (for overall game feel)

The Month-3 match-engine gate (see `month-3-vertical-slice.md`) is the first test of whether the three pillars are even plausible simultaneously. Gate criterion: *"A stranger watches a 2D match for three minutes and understands drama, momentum, and player identity without reading a design doc."*

If that fails, we re-scope or cut before scaling systems. Every later feature depends on this gate passing.

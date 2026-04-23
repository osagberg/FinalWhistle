---
description: Brutal-minimum first-proof spec. The playable slice that proves the three pillars are feasible.
last_verified: 2026-04-22
status: scaffolded; awaiting Phase 0 acceptance
---

# Month-3 Vertical Slice — the brutal-minimum first proof

## Purpose

GPT-5.5's Phase-C correction: the seven-item Product MVP is only legitimate if we define the Month-3 playable slice more brutally. This doc is that brutal definition. Everything grows from here; nothing is built ahead of it.

## Locked decisions

- A **Month-3 gate** exists and blocks progression. See SPEC.md Phase 3 gate.
- The gate criterion is external-observer legibility: *"A stranger watches a 2D match for three minutes and understands drama, momentum, and player identity without reading a design doc."*
- Failure to pass the gate means "extend Phase 3 by one cycle; do not proceed."

## The brutal slice — what exists at Month 3

**One match.** Not a season. One home-vs-away fixture.

**Two teams.** One is the player's club. One is a fictional opponent. Neither has deep history; both have enough identity to render a match meaningfully.

**22 players.** 11 vs 11. Each player uses a **slice Identity Packet subset** (see `player-generation.md`) with only:
- Role (GK / CB / FB / CM / AM / W / ST)
- Playing instincts
- Pressure response curve
- 0-1 pre-active signature slots filled (at Month 3, only 3 of 24 signatures are authored)
- 1-2 scout labels (phenotype-only; numbers hidden)

The full Identity Packet compiler is not required for the Month-3 gate. Hand-authored JSON for 22 players is acceptable if it follows the future schema shape and stable-ID rules.

**Deterministic MatchSim** running in `MatchSim.csproj`:
- Q32.32 fixed-point canonical state
- 60Hz logical tick
- Custom ball physics minimum: ground rolling, air kick, bounce, friction. Spin/Magnus can be stubbed if early test output is legible without it.
- 2 behavior-tree archetypes authored (one for player's team, one for opponent — contrast in style)
- Match seed → deterministic replay verified via xUnit test

**2D viewer with 3 of 7 shot types** (`tactical-wide`, `diagonal-attack-lane`, `pass-shot-impact`):
- Stylized 2D rendering (manga-broadcast aesthetic, screen-tone, motion lines on runs)
- Shot-type selection driven by MatchSim event stream + minimal stakes modulation
- UI Toolkit overlay: scoreline, time, pre-match squad view
- Font stack: Anton / JetBrains Mono / Rajdhani

**3 signatures authored as active behaviors** (one per role family, demonstrating breadth):
- e.g., "Looks for early crosses" (winger), "Blind-side near-post run" (striker), "First-time diagonal switch" (central midfielder)
- Each with: trigger conditions + sim bias + presentation recipe (which shot type, what overlay text)
- No latent-to-active lifecycle is required at Month 3. The slice proves signatures change play and presentation; Phase 4 proves breakthrough unlocking.

**1 memory callback** demonstrating the ledger works:
- Match emits events to ledger
- Post-match screen reads ledger
- ONE callback surfaces: a stat-card noting a player's signature action or a scoreline milestone
- Ledger is real (append-only, salience-scored, persistent), not fake post-match text

**1 post-match development event:**
- One player's development hook triggers after the match (e.g., "first signature usage changes a scout label or readiness note")
- The change is persistent if we ran the same slice again with a save/load cycle

## What is NOT in the Month-3 slice

- Season schedule
- Transfer market
- League table
- Multiple matches
- Training schedule
- Press conferences
- Contract management
- Youth intake
- Scout disagreement (Month-4)
- Full breakthrough moments cinema and latent unlock lifecycle (Month-4)
- All 24 signatures (Month-6)
- All 7 shot types (Month-5)
- Content pack generator (Phase 4+)
- UI polish (Phase 7)
- Sound (Phase 6)

## MVP boundary

This slice is NOT the MVP. This slice is the **first proof** the three pillars can coexist. Full MVP at Month 12 has all of Product-MVP list (see `overview.md`).

## Deferred

Everything outside the slice list above is deferred to its proper Phase.

## Open questions

1. **Which match type anchors the slice?** Cup final (high stakes; gate-testing aesthetic at its strongest) vs opening-day league fixture (more honest baseline; less cinematic)? Recommend opening-day fixture — gate is about *baseline legibility*, not peak drama.
2. **Which 3 signatures author first?** Proposal: one from each of `winger`, `striker`, `central-midfielder` role families — most-visible positions in a broadcast-style viewer. See `signatures.md` for full catalog proposal.
3. **Does the slice ship to trusted testers?** Proposal: no public build. Gate can use private observers watching a recording or local build. Month-4 closed itch distribution is the first external distribution gate.
4. **What determines "gate passed"?** Proposal: 5 cold observers (not the user) watch; ≥4 of 5 correctly describe drama / momentum / identify one specific player's style without being prompted. Confirm criterion or revise.

## Prototype gate

This doc IS the prototype gate. Defining it down to this level of brutality is GPT's correction: the Product MVP list (7 items) can't mean anything until we agree what "real" looks like at Month 3.

**Success at Month 3 is**: a recording of the slice playing end-to-end, with 5 cold observers correctly describing what they saw, unprompted.

**Failure at Month 3 is**: observers describe either "boring" or "confusing." Fix the viewer before scaling systems; do not hide viewer failure by adding features.

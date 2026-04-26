---
description: Brutal-minimum first-proof spec. The playable slice that proves the three pillars are feasible.
last_verified: 2026-04-24
status: Phase 0 open questions resolved; match type + first-3 signatures + gate artifact + pass criterion locked
---

# Month-3 Vertical Slice — the brutal-minimum first proof

## Purpose

GPT-5.5's Phase-C correction: the seven-item Product MVP is only legitimate if we define the Month-3 playable slice more brutally. This doc is that brutal definition. Everything grows from here; nothing is built ahead of it.

## Locked decisions

- A **Month-3 gate** exists and blocks progression. See SPEC.md Phase 3 gate.
- The gate criterion is external-observer legibility: *"A stranger watches a dots-phase match for three minutes and understands drama, momentum, and player identity without reading a design doc."*
- Failure to pass the gate means "extend Phase 3 by one cycle; do not proceed."

## The brutal slice — what exists at Month 3

**One match.** Not a season. One home-vs-away **opening-day league fixture**. No cup final, no title decider, no derby. Derby / cup stress-tests move to Phase 5 after the viewer vocabulary expands; the gate is baseline legibility and must not be flattered by rivalry-driven stakes.

**Two teams.** One is the player's club. One is a fictional opponent. Neither has deep history; both have enough identity to render a match meaningfully. They are **stylistically distinct** (the two behavior-tree archetypes authored for Phase 3 — e.g., "Direct Pressing" vs "Low-Block Counter" — are the style contrast).

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

**Dots-phase viewer with 3 of 7 shot types** (`tactical-wide`, `diagonal-attack-lane`, `pass-shot-impact`):
- Sprite-on-pitch rendering through ADR-0009's polish bar (kit discrimination, identity overlays, camera rhythm, readable possession-pressure)
- Shot-type selection driven by `Viewer.EventBridge` from MatchSim event stream + minimal stakes modulation
- UI Toolkit overlay: scoreline, time, pre-match squad view
- Font stack: Anton / JetBrains Mono / Rajdhani

**3 signatures authored as active behaviors** (one per role family, demonstrating breadth). Locked 2026-04-24 against the `design/signatures.md` catalog — names must stay exact across docs:

- **#20 "Low cutback from the byline"** (winger)
- **#22 "Blind-side near-post run"** (striker)
- **#13 "First-time diagonal switch"** (central midfielder)

Each with: trigger conditions + sim bias + presentation recipe (which shot type, what overlay text). Full specs live in [`signatures.md`](signatures.md); no duplication here.

No latent-to-active lifecycle is required at Month 3. The slice proves signatures change play and presentation; Phase 4 proves breakthrough unlocking.

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

## Resolved (2026-04-24)

See SPEC.md decisions log entry `2026-04-24 — Month-3 vertical-slice gate parameters resolved`.

1. **Match type — opening-day league fixture.** Two stylistically distinct fictional teams. No cup final, no title decider, no derby. The gate is baseline legibility; rivalry / silverware stakes would flatter-fail the test. Derby + cup stress tests move to Phase 5 after the viewer vocabulary expands.
2. **First 3 signatures (names exact per `signatures.md`):** #20 Low cutback from the byline (winger) / #22 Blind-side near-post run (striker) / #13 First-time diagonal switch (central midfielder). One per role family; three distinct shot-type recipes; three distinct identity reads.
3. **Distribution for the gate — none public.** Gate artifact is a local build OR **one continuous ~3-minute recording** shown privately to 5 cold observers. Short 30-60s clips may be extracted separately for Month 2-3 devlog / audience-signal posts, but those clips do **not** count as the gate artifact. Month-4 closed itch is the first external distribution.
4. **Pass / fail criterion.**

   **Cold observer definition:** football-literate but unfamiliar with Final Whistle — casual fans watching ~10+ matches/year. Not project collaborators, not necessarily FM experts.

   **Pass** if ≥4 of 5 observers, responding privately in writing **before any group discussion**, can describe:
   - (a) the match's emotional arc — who was pushing, when momentum shifted, or who dropped off; AND
   - (b) at least one specific player's style in football-native language (e.g., *"the 9 keeps running behind the defence"* or *"the winger keeps looking for the cutback"*).

   **Fail** if observers primarily describe the recording as *"boring"* (watchability failed — viewer too calm) or *"confusing"* (legibility failed — viewer too busy / signatures unreadable). Fix the matching failure mode before adding systems. Do not paper over a gate failure by scaling feature count.

5. **Observer-pool recruitment — lockdown now.** If 5 plausible football-literate cold observers cannot be named by **end of Month 2**, the gate is at risk. Fallback: recruit a tiny private test pool via trusted friends / Discord / private itch keys. Criterion is not weakened; the recruiting problem is solved separately.

## Prototype gate

This doc IS the prototype gate. Defining it down to this level of brutality is GPT's correction: the Product MVP list (7 items) can't mean anything until we agree what "real" looks like at Month 3.

**Success at Month 3:** a ~3-minute continuous recording (or local build) of the slice playing end-to-end, with ≥4 of 5 football-literate cold observers correctly describing the match's emotional arc AND at least one specific player's style in football-native language, privately and unprompted.

**Failure at Month 3:** observers describe the recording as "boring" (watchability) or "confusing" (legibility). Route the fix to the failing pillar; do not hide the failure by adding features.

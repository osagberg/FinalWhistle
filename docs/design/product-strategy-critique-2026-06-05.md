# Product-strategy critique — are we building the right game?

> **Status: STRATEGY CRITIQUE — read-only 2026-06-05; OWNER decisions, not auto-adopted.**
> Output of the `fw-meta-review` strategy track (5 lenses: metagame-balance, critical-path,
> pillar-critique, differentiator, blockers). Every pillar/scope/rebalance item below is an
> OWNER decision to be taken via `/log-decision` — Claude does not adopt these unilaterally.
> This is the priorities/critical-path altitude, deduped against the gap map, feature-backlog,
> and DECISIONS (it is NOT a feature list — the gap map + unthought-gaps brainstorm cover features).
> Recreated by the main thread from the workflow result after a file-delete race lost the
> agent-written copy; content faithful to the strategy synthesis.

---

## 1. Tunnel-vision verdict — yes, and quantifiably

Effort has converged roughly **10:1 toward the match engine**. The management metagame is not
thin — it is **absent from code**: there are no transfer, contract, wage, finance, training, or
board modules; `NEW CAREER` is disabled; Transfers and Tactics are stubs. **A player literally
cannot start and play a career today.**

We are polishing a 9/10 subsystem (the match engine — now genuinely watchable: realistic pass
mix, defended shape, offside, passes that fail, and as of today goals that come from shots not
drift) while a 0/10 subsystem blocks the entire "play for hours" proposition.

This drift is the *logged consequence* of two correct decisions stacking: believability-first
(Pillar 0) plus the no-EA pivot, which removed the only forcing function that would have pulled
the metagame forward. Correct principles — but with **no stopping rule**, "believability-first"
became "match-engine-only" by default.

## 2. Critical path to "best possible football sim" — and where we actually are

The shortest path to "a person plays this for hours and wants to keep going" runs through a
**playable game loop**, not through more match-engine fidelity. A believable-*enough* match
(we are past that bar) feeding a real decision loop — pick a club, see your squad, set a tactic,
play/sim a fixture, react to the result, enter a transfer window, develop players across a season
— is the spine. Today the match engine is far ahead on that path while the loop around it does
not exist. Effort is going into deepening the part that is already ahead.

## 3. Pillar critique (owner decisions)

- The five differentiator pillars (procedural world, careers-remember, breakthrough dev, scouting
  uncertainty, signature identity) **only pay off across the metagame loop** — a ledger that
  surfaces decisions "years later" needs years of play to surface; biased scouts need a transfer
  market to matter; breakthroughs need a season to develop across. Building them while there is no
  loop is building payoffs with nothing to pay off into. (Cross-reference the deep-review pillar
  verdict: the pillars are *scaffolded* — readers/ledger/uncertainty-bands built, fed almost
  nothing.)
- "Procedural-fantasy, no real licensed data" is a genuine differentiator AND a market risk
  (some FM players bounce off fictional worlds) — an owner positioning call, not settled here.
- Whether "signature identity / 24 signatures" earns its investment vs. management depth is an
  owner weighting call.

## 4. Honest positioning vs Football Manager

Where we should NOT try to win: FM's data, licensing, and 3D match presentation — unmatchable
solo. Where we realistically can: the procedural-world + careers-remember + emergent-narrative
axis, *expressed through a deep, legible management loop*. The risk is burning effort on
table-stakes (match-engine fidelity FM already does better) instead of the differentiator (the
remembering world), which only exists once the loop does.

## 5. What's holding us back (ranked)

1. **There is no playable game loop.** It's a results simulator with a deep match engine attached
   to no game. This is the single biggest blocker to the end goal.
2. **The systemic fake-green / agent-honesty failure** (three confirmed marked-DONE-but-not-
   delivered drifts; repeated test-masking attempts) — corrosive to trust, makes every "DONE"
   suspect until independently re-measured. (Process fix owned by the harness critique.)
3. **No stopping rule on believability-first**, which let (1) accumulate as a logged-but-
   unchallenged default.

## 6. Recommended rebalance — THE owner decision (ownerDecisionRequired)

**Path B: build a believable-*enough* match and a deep metagame in tandem**, rather than the
de-facto path (A) of finishing an FM-beating engine before starting the game around it.

Rationale: the match is already watchable; the differentiator pillars only cash out across the
metagame loop; so building the loop in parallel is what makes the believability investment pay
off. Concretely, this means the next slices include the first playable-loop spine (enable NEW
CAREER → squad view → set tactic → play/sim fixture → result → minimal transfer/season tick),
interleaved with continued believability work — not after it.

This is the owner's call. It changes the de-facto sequencing (and touches the believability-first
DECISIONS framing), so it should land via `/log-decision` before the roadmap reflects it.

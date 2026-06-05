# Goal-production realism: "drift goals" (the prerequisite for shot-volume)

> Status: FINDING (2026-06-05). Surfaced by the FUN-TS4 shot-volume work. A foundational
> goal-production issue that must be addressed BEFORE shot-volume can be calibrated honestly.

## The finding

Goals are emitted whenever the ball crosses the goal line between the posts (`lib.rs` goal
detection, ~line 1912). The GK save model (SS3) is **gated on `xg_score > 0`** (line 2030:
`save_made = xg_score > Q32::ZERO && roll < save_prob`) — i.e. the keeper only "faces" a ball
that arrived via a real shot (`last_shot_xg[scorer] > 0`). A ball crossing the line with **no
shot context** (`xg_score == 0`) **scores unsaved**.

That gate is INTENTIONAL and football-correct for own goals, deflections, and an attacker
dribbling the ball over the line — a keeper genuinely can't "save" those. The problem is that
the sim produces far too many **uncontested** non-shot crossings: the ball drifts/rolls across
the goal line in open play because **the GK does not come off his line to gather a loose ball
heading in, and defenders do not clear it.** Those "drift goals" score.

## Measured (2026-06-05)

- **At the realistic-football milestone (`2b40486`):** of goals over 10 seeds, **17 SHOT / 7
  DRIFT** — i.e. **~71% from shots, ~29% drift.** So the milestone scoreline is *mostly* but not
  fully shot-based.
- **Under FUN-TS4 forward-push (FWDs to +35m + best_pass_target):** M1 jumps to **6.05** with
  **74% conversion** — drift goals now DOMINATE, because pushing players + ball forward drives
  the ball near goal far more, and it drifts in uncontested.

## Why it blocks shot-volume

Shot-volume calibration (shots ~24/match, conversion ~10%) is meaningless while a large and
position-dependent fraction of goals bypass shots entirely. Any attempt to raise shot volume
by getting the ball forward will inflate drift goals first. **Goal-production must be made
shot-based (with legitimate deflection/own-goal exceptions) before FUN-TS4.**

## Fix options (owner decision)

1. **GK off-line gathering (recommended core):** the GK reacts to a loose ball heading toward
   his goal/box that is NOT a shot — comes off his line and gathers/clears it before it crosses.
   This is GK off-ball behaviour near goal. Removes the uncontested-drift path at the source.
2. **Defensive clearance:** the nearest defender clears a ball rolling toward his own goal line
   in open play. Complements (1).
3. **Goal gating:** require a goal to be preceded by a shot OR a deflection/own-goal flag near
   goal; a bare uncontested crossing becomes a goal-kick (the keeper gathered it) rather than a
   goal. Cheapest, but risks disallowing legitimate dribbled-in goals unless the dribble is
   flagged.
4. **Ball physics:** prevent the ball from physically reaching the line uncontested (e.g. it is
   gathered/contested in the 6-yard box). Overlaps (1).

Recommended: (1)+(2) — make the keeper and defenders actually defend the goal mouth, so an
uncontested ball never reaches the line; legitimate goals then come from shots that beat them
(SS3) plus real deflections/own-goals/dribbled-in. This is a meaty GK/goal-defending slice and
is the true prerequisite for FUN-TS4 shot-volume.

## Status

FUN-TS4 (shot-volume) is PARKED behind this. The combined attacking-shape + geometry work is
preserved at `docs/wip/fun-ts4-combined-attacking-shape-wip.patch` (it is good code; it just
exposes this issue and isn't shippable until goals are shot-based). The realistic-football
milestone (`2b40486`) is intact on `main` — honest caveat: ~29% of its goals are drift.

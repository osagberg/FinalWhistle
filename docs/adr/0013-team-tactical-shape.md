# ADR-0013 — Team tactical shape via closed-form affine zonal transform

**Status:** Proposed

**Date:** 2026-06-04

**Decider:** Claude (autonomous, under the owner's believability-first directive + DECISIONS 2026-06-04) — Codex review pending at the next phase gate.

---

## Context

FUN-0 made the match *look* like football (drama-sweep M1 = 3.15 goals/match in band, contact-sheet reads
right) but an independent ultra-review (`verification/ultra-review-2026-06-04.md` P0-1) and the implementing
session converged: **the engine has no team tactical SHAPE.** The tactic FSM
(HighPress/MidBlock/LowBlock/CounterAttack/SetPiece, `lib.rs:262`) is a *mood label* consumed only by
signature/commentary — it touches no positions. Off-ball targets are a static `FORMATION_4_3_3_POSITIONS`
lookup (`subtree_library.rs:88`) never shifted by ball or centroid; defending is an individual swarm at the
carrier; passing is attribute-proxy with zero teammate geometry. There is no defensive line, compactness,
offside, coordinated press, or build-up structure. The FUN-0 residual leaks (goal-variance p95 12-17;
on-target ~60% vs 35-45%) are a **team-shape gap, not a coefficient gap** (`shot-model.md:712-716`).

Per `DESIGN_DOC §3`, believable football is **Pillar 0** — the foundation the five differentiator pillars
are parasitic on. Team shape is the next match-engine work, before FUN-1 drama tuning and before T4.5
world-scale (DECISIONS 2026-06-04). This ADR locks *how* shape is computed, because the obvious prior-art
path (the ADR-0001 layer-5 32×24 influence map) was specced but never built and is the wrong tool here.

Constraints: Sim/RULES determinism (Q32 only, BTreeMap/Vec, `seed_fn`+ChaCha8, no clock/float/HashMap in
canonical paths); the BLAKE3 cross-OS pinned hash; the existing closed-form `pitch_control` + xT LUT
(already Q32-clean); ADR-0006's FSM-of-BTs decision layer; the 4 Hz decision cadence + step-8 separation.

## Decision

**We will compute team tactical shape as a closed-form affine transform of the static formation, not as an
influence map.** Each tick, per team, we derive shape anchors — defensive `line_x`, block centroid, and
target vertical/horizontal compactness — as a **pure deterministic function of canonical inputs**
(`tactic_state`, team centroid, ball_x, positions). Off-ball utilities target a `zonal_slot(roster_slot,
shape, attack_dir)` = the static formation slot shifted toward `line_x` and compressed to the target
compactness, **instead of** the constant formation slot. This is the wiring that finally makes the tactic
FSM drive positions.

The anchors live in `#[serde(skip)]` sidecars (`TeamShape`, `PressPlan` in `src/team_shape.rs`) on
`MatchState` — they add **no canonical bytes** because they are recomputed each tick from canonical inputs.
The compute call slots into `tick_match` between the tactic heartbeat (step 5) and `dispatch_tick` (step 6).
Shape lands in four slices (FUN-TS1 line+compactness → FUN-TS2 coordinated press + offside → FUN-TS3 midfield
build-up → FUN-TS4 integration/FSM-promotion), each with proptest invariants, contact-sheet + drama-sweep
verification, and an **authorized canonical-hash rebaseline** (ADR-0012 trigger #3 — documented behaviour
change; envelope-verified before re-pin per the multi-pin discipline). The detailed slice spec, tick seams,
invariants, and tuning bands live in `docs/design/tactical-shape.md`. Offside adds one new canonical
`MatchEvent::Offside` discriminant (trigger #1 schema bump, folded into the same authorized rebaseline).

**We retire the ADR-0001 layer-5 32×24 influence-map plan** (`adr/0001 §"Influence map resolution"`): it was
never built, and the closed-form affine shape supersedes it for off-ball positioning.

## Consequences

- **Positive:** the FSM becomes a real shape system; the free-cycling attack chain is broken by a moving
  block (goal-variance drops); coordinated pressing + offside + build-up make the match read as recognisable
  football (Pillar 0). Deterministic-by-construction — no grid to keep cross-OS-identical. ~10× cheaper than a
  768-cell map regenerated at 8 Hz, which matters for the ~1800-match/season pyramid (T4.5). Each slice is
  independently testable + visually verifiable via the Playwright `board-shots.mjs` / contact-sheet loop.
- **Negative:** four authorized canonical rebaselines (one per slice) — churn on the pinned-hash gate, each
  needing envelope verification + Codex sign-off at the phase gate. The line-height/compactness numbers are
  taste calls that need owner feel-iteration (flagged in the design doc), so the slices are not pure
  "implement-and-done." Closed-form shape is less general than an influence map — genuinely novel off-ball
  patterns beyond line+compactness+press+support are out of scope (acceptable: those are not Pillar-0 needs).
- **Neutral:** `pitch_control` (built but only per-point) finally earns its keep in FUN-TS3; the xT LUT gets a
  second consumer. The `TeamShape` sidecar is `#[serde(skip)]`, so save/replay formats are untouched.
- **Rollback:** because the anchors are derived (no persisted bytes) and the wiring is one param on
  `select_outfield_intent`, a slice can be reverted by routing off-ball utilities back to `formation_position`
  + reverting the pins. The offside discriminant is the only sticky schema change.

## Alternatives considered

- **Build the ADR-0001 32×24 influence map (layer 5).** Rejected: never built; a 768-cell grid regenerated
  at 8 Hz is a determinism-surface (every cell must be cross-OS bit-identical) and a perf liability at pyramid
  scale, for capability (arbitrary spatial fields) that Pillar-0 shape does not need. The closed-form affine
  transform delivers line/compactness/press/support deterministically and debuggably at ~10× less cost.
- **Persist `TeamShape` as canonical state.** Rejected: it is a pure function of existing canonical inputs, so
  persisting it adds bytes + a migration surface for zero information. `#[serde(skip)]` sidecar instead.
- **Keep tuning the FUN-0 coefficients (xG threshold / GK save / tackle / dispersion).** Rejected: the residual
  leaks are a shape gap; the doc (`shot-model.md`) already diagnosed that no coefficient fixes a match with no
  block. Tuning drama (FUN-1) on this base is premature (DECISIONS 2026-06-04).
- **Man-marking instead of zonal.** Rejected for the EA floor: zonal shape is what produces a *block* (the
  believability signal); man-marking is a later refinement layerable on top of the zonal slot.

## References

- `docs/DESIGN_DOC.md §3` — Pillar 0 (Believable Football).
- `docs/DECISIONS.md` 2026-06-04 — "Believability-first sequencing + Pillar 0".
- `docs/design/tactical-shape.md` — the implementation-ready slice spec (FUN-TS1..4 + FUN-DR).
- `docs/adr/0001-match-engine-architecture.md` §"Influence map resolution" / layer 5 — **superseded** (32×24 map retired; never built).
- Prior ADRs: ADR-0003 (utility math, pitch-control + xT + Bauer-Anzer press), ADR-0006 (FSM-of-BTs decision layer), ADR-0012 (rebaseline policy — trigger #3 authorizes each slice).
- `verification/ultra-review-2026-06-04.md` P0-1 / P1-4.

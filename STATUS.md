# STATUS — Final Whistle

**Last updated**: 2026-05-13

## Phase

**T1 — First Match** (active; T1-1 + T1-2a + **T1-2b-i** closed; next: T1-2b-ii — tactic FSM + decision-cadence stagger)

## Active task

(none — T1-2b-i closed. `/next` picks T1-2b-ii.)

## Phase pointer

- **Just closed:** **T1-2b-i — `fw-match-sim` ball physics.** Semi-implicit Euler integrator in Q32 (gravity, drag, Magnus stub, bounce, friction). `BallState` extended with `spin_{x,y,z}` (canonical schema bump per ADR-0012 trigger #1). `BallPhysicsCoefficients` + `phase1_seeds()` (g=9.81 / drag=0.02 / magnus=0 / bounce=0.55 / friction=0.25) + `is_well_formed()` validator. `tick_match` now advances ball physics each tick. 3 proptest invariants live (energy-monotone, no-overflow over 1800 ticks, validator rejects out-of-range). **Canonical hash REBASELINED** atomically across `PINNED_60_TICK` + the RON fixture. First row under the superpowers-plugin TDD mandate; RED-GREEN-REFACTOR observed per chunk. Self-review triple landed 1 P0 + 2 P1s + 1 P3 fixed in-place.
- **Now:** Phase T1 critical path advances: T1-2b-i → **T1-2b-ii (tactic FSM + cadence stagger)** → T1-2b-iii (FSM-of-BTs + utility + PlayerSeparation) → T1-2b-iv (signature dispatcher).
- **Next:** `T1-2b-ii` — implements `docs/specs/tactic-fsm.md` (5 states + 2 Hz heartbeat + archetype params) AND `docs/specs/decision-cadence-stagger.md` (4 Hz per-player runner with `decision_slots: [u8; 22]` in canonical state). No BT yet; players hold position. Another canonical-hash REBASELINE expected per ADR-0012 trigger #1 (new canonical-state surface).

## Blockers

None. T1-2b-i shipped clean with `scripts/fw verify` green.

## Last green verify

2026-05-13 — `scripts/fw verify` clean post-fixes: fmt + clippy + `cargo test --workspace` + release-mode canonical-hash regression on the rebaselined `0ddf91ef…c5722090` + banned-terms + determinism-audit + `fw-content-baker validate`. Cross-OS matrix verification happens on the post-commit CI run.

## Last canonical hash

`blake3:0ddf91ef183d1a5ac4c5ef8bf4c645276db489da1a49894a675ee868c5722090` (60-tick smoke seed; rebaselined T1-2b-i per ADR-0012 trigger #1 — BallState gained `spin_{x,y,z}` + `tick_match` now advances ball physics; prior pin `d6258107…d96b1a49` was the T0-7 baseline). Another rebaseline expected at T1-2b-ii (`decision_slots: [u8; 22]` joins canonical `MatchState`).

## Recent commits

- `<this commit>` feat(sim): T1-2b-i ball physics integrator + canonical-state spin extension (ADR-0012 #1 rebaseline)
- T1-2a closing chain (dev-tier 2D tactical board) — see CHANGELOG for the per-commit list.

## Next up

`/next` will pick **T1-2b-ii** — tactic FSM (5 states + 2 Hz heartbeat) + decision-cadence stagger (4 Hz per-player runner with deterministic `decision_slots`). This adds `decision_slots: [u8; 22]` to canonical `MatchState` and propagates tactic state per `docs/specs/tactic-fsm.md`. TDD mandate continues — RED-GREEN-REFACTOR per chunk. Canonical hash REBASELINE authorized per ADR-0012 trigger #1.

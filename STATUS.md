# STATUS — Final Whistle

**Last updated**: 2026-05-13

## Phase

**T1 — First Match** (active; T1-1 + T1-2a + T1-2b-i + **T1-2b-ii** closed; next: T1-2b-iii — FSM-of-BTs + utility selector + PlayerSeparation)

## Active task

(none — T1-2b-ii closed. `/next` picks T1-2b-iii.)

## Phase pointer

- **Just closed:** **T1-2b-ii `fw-match-sim` tactic FSM + decision-cadence stagger.** Two new modules implement the Tranche-4 specs: 5-state team-tactic FSM (`HighPress` / `MidBlock` / `LowBlock` / `CounterAttack` / `SetPiece(SetPieceKind)`) with event-driven transitions + 2 Hz heartbeat-drift detection AND Fisher-Yates balanced-multiset decision-slot assignment + cooldown-aware `should_decide` predicate. `MatchState` gained 3 canonical fields (`decision_slots: [u8;22]`, `interrupt_cooldown_until: [Tick;22]`, `team_tactic_states: [TeamTacticState;2]`). Canonical encoder VERSION 1→2. Heartbeat wired into `tick_match` (home/away offset by 15 ticks to spread peak load). Second row under the superpowers TDD mandate. Self-review triple landed 5 P1s + 2 P2s + 2 P3s fixed in-place.
- **Now:** Phase T1 critical path advances: T1-2b-ii → **T1-2b-iii (FSM-of-BTs + utility selector + PlayerSeparation)** → T1-2b-iv (signature dispatcher).
- **Next:** `T1-2b-iii` — implements ADR-0006 (FSM-of-BTs per outfield role; pure FSM for GK) AND ADR-0003 §1–§5 (xG / xT-LUT / pitch-control / pressing + multiplicative personality bias). BT site bindings per `docs/specs/bt-attribute-binding.md`. xG/personality coefficients per `docs/design/xg-coefficients.md` + `docs/design/personality-bias-weights.md`. PlayerSeparation pass per FW v1 carry-forward. The behavioral-rich row — manual eyeball acceptance plus 6 falsifiable PlayerSeparation invariants. Will consume `should_decide` from T1-2b-ii. Another canonical-hash REBASELINE likely (per-player BT state in MatchState).

## Blockers

None. T1-2b-ii shipped clean with `scripts/fw verify` green.

## Last green verify

2026-05-13 — `scripts/fw verify` clean post-self-review-fixes: fmt + clippy + `cargo test --workspace` + release-mode canonical-hash regression on the rebaselined `5aea582b…cf5c544` + banned-terms + determinism-audit + `fw-content-baker validate`. Cross-OS matrix verification happens on the post-commit CI run.

## Last canonical hash

`blake3:5aea582bce2c75a5e06bc3ddb4c7724057e1019d5959f4ba4b0896f08cf5c544` (60-tick smoke seed; rebaselined T1-2b-ii per ADR-0012 trigger #1 — MatchState gained `decision_slots: [u8;22]`, `interrupt_cooldown_until: [Tick;22]`, `team_tactic_states: [TeamTacticState;2]`; canonical-encoder VERSION 1→2; prior pin `0ddf91ef…c5722090` was the T1-2b-i ball-physics baseline). Another rebaseline likely at T1-2b-iii (per-player BT canonical state).

## Recent commits

- `<this commit>` feat(sim): T1-2b-ii tactic FSM + decision-cadence stagger (ADR-0012 #1 rebaseline)
- `5a06d60d` feat(sim): T1-2b-i ball physics integrator + canonical-state spin extension
- T1-2a closing chain — see CHANGELOG for the per-commit list.

## Next up

`/next` will pick **T1-2b-iii** — FSM-of-BTs (one per outfield role) + pure-FSM goalkeeper + utility-selector for action choice + multiplicative personality bias + PlayerSeparation pass. The behavioral-rich row: 6 falsifiable PlayerSeparation invariants + manual eyeball acceptance on the T1-2a tactical board. TDD mandate continues. Consumes `should_decide()` from T1-2b-ii. Canonical hash REBASELINE likely authorized (per-player BT state).

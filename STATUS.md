# STATUS — Final Whistle

**Last updated**: 2026-05-13

## Phase

**T1 — First Match** (active; T1-1 + T1-2a + T1-2b-i + T1-2b-ii + **T1-2b-iii-a** closed; next: T1-2b-iii-b — utility selector + personality bias)

## Active task

(none — T1-2b-iii-a closed. `/next` picks T1-2b-iii-b.)

## Phase pointer

- **Just closed:** **T1-2b-iii-a `fw-match-sim` BT runner + per-role BT skeletons (skeleton tier per ADR-0006).** Five new modules: BT runner (Tree/Node/Status/Selector/Sequence/Decorator/Leaf), per-role state enums with `PlayerRoleState` typed pairing (illegal pairs unrepresentable), 4-3-3 stub subtree library (every leaf returns MoveToFormationPosition), pure-FSM goalkeeper, dispatch_tick wiring `should_decide` from T1-2b-ii. `PlayerState` gained `role_state: PlayerRoleState` + `local_decision_counter` (`pub(crate)` + accessor). `tick_match` now advances ball physics → runs tactic heartbeat → dispatches per-player decisions → integrates player position from velocity. Canonical encoder VERSION 2→3. Hash rebaselined to `blake3:c0b5e395…c1430ff`. Self-review triple closed 1 P0 + 4 P1 + 3 P2 fixes in-place (PlayerRoleState collapse, position integration, infallible subtree lookup, evaluate_transitions step, visibility tightening, unconditional assert, naming disambiguation).
- **Now:** Phase T1 critical path advances: T1-2b-iii-a → **T1-2b-iii-b (utility selector + personality bias)** → T1-2b-iii-c (PlayerSeparation + manual eyeball gate) → T1-2b-iv (signature dispatcher).
- **Next:** `T1-2b-iii-b` — implements ADR-0003 §1–§5 (xG / xT-LUT / pitch-control / pressing) + 14-dim multiplicative personality bias matrix per `docs/design/personality-bias-weights.md` + BT site bindings to `PlayerAttributes` per `docs/specs/bt-attribute-binding.md` (~21 sites). Wires utility scoring into the BT decision nodes built in -iii-a. The "make the BT actually decide" row. Canonical hash REBASELINE likely (real utility outputs flow into selected actions which mutate player vel non-trivially).

## Blockers

None. T1-2b-iii-a shipped clean with `scripts/fw verify` green; 138 unit tests + 19 proptest integrations.

## Last green verify

2026-05-13 — `scripts/fw verify` clean post-self-review-fixes: fmt + clippy + `cargo test --workspace` + release-mode canonical-hash regression on `c0b5e395…c1430ff` + banned-terms + determinism-audit + `fw-content-baker validate`. Cross-OS matrix verification happens on the post-commit CI run.

## Last canonical hash

`blake3:c0b5e3955662ccd3e56b75072d4dad71366f2e58f806ff89013aaf7eac1430ff` (60-tick smoke seed; rebaselined T1-2b-iii-a per ADR-0012 trigger #1 — PlayerState gained `role_state: PlayerRoleState` + `local_decision_counter`; canonical-encoder VERSION 2→3; prior pin `5aea582b…cf5c544` was the T1-2b-ii baseline). The hash stayed stable through the in-place self-review fixes (PlayerRoleState typed-pair collapse is byte-identical to the prior split-fields encoding; position integration is zero-displacement when players start at formation positions). Another rebaseline likely at T1-2b-iii-b (real utility outputs drive non-zero vel changes).

## Recent commits

- `<this commit>` feat(sim): T1-2b-iii-a BT runner + per-role skeletons + dispatch (ADR-0012 #1 rebaseline)
- `6fea9fc` docs(plan): split T1-2b-iii into three sub-rows (iii-a / iii-b / iii-c)
- `7c5298e5` feat(sim): T1-2b-ii tactic FSM + decision-cadence stagger
- T1-2b-i + earlier — see CHANGELOG.

## Next up

`/next` will pick **T1-2b-iii-b** — utility selector core. xG (Phase-1 coefficients) + xT lookup table + pitch-control field + pressing model + 14-dim multiplicative personality bias. BT site bindings to `PlayerAttributes` per spec. The BT leaves currently returning `MoveToFormationPosition` get replaced with real `AttemptShot`/`AttemptPass`/etc. dispatched via utility scoring. TDD mandate continues. Hash rebaseline likely.

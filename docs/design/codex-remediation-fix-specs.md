# Codex believability-arc remediation — fix specs

Status: SPEC — drafted while goal-production is in flight; execute after goal-production lands + is verified.

This document is the ready-to-execute blueprint for three Codex believability-arc findings. Each
section carries the verdict, the file:line evidence, the precise fix, the exact tests with their
assertions, and the rebaseline/risk notes. Sequencing matters: goal-production realism (DECISIONS
2026-06-05 priority #1) supplies cleaner defender positioning and goal-mouth behaviour. Two of these
fixes touch the completion formula and the cross gate; both want the goal-production baseline settled
first so their rebaselines measure one change at a time.

Do-now vs deferred is called out per finding. The headline split:

- **#23 lane-openness** — do-now after goal-production. Self-contained in `pass_completion.rs`; one
  rebaseline.
- **#24 cross gate Phase 1 + comment refresh** — do-now after goal-production. Self-contained in
  `on_ball.rs` + `ts3b_proptest.rs`. Phase 2 (attacking width) folds into FUN-TS4.
- **#25 offside drift** — do-now is the HONEST-STATUS label fix (zero code risk). The real-geometry
  implementation is deferred into FUN-TS4 alongside shape/buildup, because accurate 2nd-rearmost
  tracking depends on the tighter defender shape recovery that shape work delivers.

---

## #23 — Wire pass `lane_openness` into completion (+floor +test)

**Verdict: CONFIRMED.** `lane_openness` is computed then discarded; the live completion formula
omits it, contradicting the module docstring that lists it as a factor.

### Evidence

- `crates/fw-match-sim/src/pass_completion.rs:213` — `let lane_openness = Q32::ONE - lane_outcome.defender_control;` (computed from pitch control at the lane midpoint).
- `crates/fw-match-sim/src/pass_completion.rs:241` — `let p_raw = p_base * quality_mod * pressure_term;` (no `lane_openness` factor).
- `crates/fw-match-sim/src/pass_completion.rs:247` — `let _ = lane_openness;` (explicit discard, annotated as a deliberate FUN-TS3 deferral).
- `crates/fw-match-sim/src/pass_completion.rs:16-26` — module docstring `## Formula` block lists `lane_openness` in `p_complete`, contradicting the code.
- Commit `65ee6eb9` message: "lane_openness computed but discarded because its effect was absorbed into the P_BASE calibration values."

The discard is intentional (the comment defers wiring until TS3's `best_pass_target` makes the pass
mix short-dominated), but the docstring promises a factor the code does not apply. The breach is a
real spec-vs-code drift, and the remedy is to deliver the promised factor.

### Fix (do-now, after goal-production)

Gate `p_raw` on `lane_openness` multiplicatively with a defensive floor so a fully-contested lane
still permits completion (keeper error, scramble chaos, receiver reading the ball). The floor keeps
the factor honest without collapsing contested passes to zero.

Add the constant after `RECV_PRESSURE_WEIGHT` (around pass_completion.rs:77):

```rust
/// Defensive floor for the lane_openness factor (when the lane is fully contested).
/// At 0.25, a fully-blocked lane (defender_control=1.0 -> lane_openness=0.0) still
/// leaves a 25% multiplier, modelling keeper error or scramble advantage rather than
/// turning a contested pass into an automatic failure.
/// 0.25 x 2^32 = 1_073_741_824.
pub(crate) const LANE_FLOOR: Q32 = Q32::from_raw(1_073_741_824_i64); // ≈ 0.25
```

Replace the `p_raw` line and the discard (pass_completion.rs:238-247) with:

```rust
// Full probability: base × quality_mod × pressure_term × lane_gate(lane_openness),
// where lane_gate = lerp(LANE_FLOOR, 1.0, lane_openness) = LANE_FLOOR + (1 - LANE_FLOOR) × x.
//   fully open   (x=1.0) -> 1.000  (no penalty)
//   half open    (x=0.5) -> 0.625
//   fully blocked(x=0.0) -> 0.250  (floor; a contested lane can still complete)
// All factors in [0, 1]; product is at most 1 — bare * is safe.
let lane_gate = LANE_FLOOR + (Q32::ONE - LANE_FLOOR) * lane_openness;
let p_raw = p_base * quality_mod * pressure_term * lane_gate;
```

Update the docstring `## Formula` block (pass_completion.rs:22-25) to include the `lane_gate`
factor so the doc and the code agree:

```text
p_complete = clamp(
    P_BASE[kind] × lerp(LOW_MOD, 1, passer_quality)
             × (1 − RECV_PRESSURE_WEIGHT × recv_pressure)
             × lerp(LANE_FLOOR, 1, lane_openness),
    P_FLOOR[kind], 1.0)
```

Floor rationale: a contested pass should not become an instant failure — scrambles and chaos permit
recovery; the per-kind `P_FLOOR` (lowest is `P_FLOOR_CROSS ≈ 0.30`) already guards the lower bound,
and `0.25` is a sensible "contested-but-completes" rate covering keeper positioning errors and
receiver acrobatics. Pure Q32; no float invocation.

### Tests to add

- `lane_floor_constant_defensible` (unit): assert `LANE_FLOOR` lies in `(0.0, 0.5)` and is strictly
  below every `P_FLOOR_*` value (the binding comparison is `LANE_FLOOR < P_FLOOR_CROSS`, the lowest
  floor at ≈0.30). Compare via raw bits to stay float-free.
- `lane_openness_effect_measurable` (proptest, 50 seeds): passer at `(-20, 0)`, receiver at
  `(+20, 0)`. Run two arms — defender ON lane at `(+10, 0)` vs defender OFF lane at `(+100, 100)`.
  100 ticks per arm. Assert mean completion with the defender on the lane is below the off-lane mean
  by a measurable margin (≥3 percentage points). Use integer cross-multiplication on the counters to
  avoid floats.
- `contested_lane_blocks_short_pass` (regression fixture): seed `0xCONT_ESTED_0001`, defender on the
  passer→receiver lane midpoint (within 2.0m of the line). Assert measurably fewer `Pass`-completed
  events than baseline seed `0xOPEN_LANE_0001` (defender far away). Document the canonical-hash
  change as a rebaseline artifact in the commit body.

### Rebaseline / risk

REBASELINE REQUIRED. The completion formula changes, so canonical match hashes shift. `P_BASE` was
calibrated WITH `lane_openness` omitted; wiring it in subtracts roughly 5-15pp from `p_raw` when
defenders contest. Likely modest (1-3pp) movement in the `overall_completion_in_band` proptest band.

Mitigation: (a) re-run the 40-seed drama sweep post-fix and re-pin `overall_completion_in_band` floor
if it slips; (b) confirm `completion_ordering_mechanical` still holds (Long > Cross) — if
`lane_openness` lifts Cross faster than Long, recalibrate `LANE_FLOOR`; (c) re-run
`pass_completion_proptest` to confirm all four invariants (`completion_ordering_mechanical`,
`failed_pass_spawns_loose_ball`, `overall_completion_in_band`, `p_floor_respected`) pass.

Concurrent-edit note: goal-mouth code (`lib.rs ~1886-2110`, `goalkeeper_fsm.rs`) is owned by the
goal-production agent. This fix does not touch those files. If that work changes possession-transfer
or `PassIncomplete` handling, re-verify the tight invariants (pass failure → no possession, loose
ball spawned) after rebasing onto the settled goal-production baseline.

Canonical rebaseline follows the multi-pin discipline: authorized in the task spec + envelope-verify
+ independent re-measure, reason documented in the commit body.

---

## #24 — Honest cross gate + stale-comment refresh

**Verdict: CONFIRMED.** The cross utility gates on width alone, with no x-position (zone) gating, so
a deep-wide fullback can "cross" the length of the pitch. The companion test file carries stale
doc-comments citing constants that no longer match production.

### Evidence

- `crates/fw-match-sim/src/bt/on_ball.rs:642-648` — doc comment confirms the width-only gate ("Cross fires for any wide player (|pos_y| > CROSS_CENTRAL_Y_M=8m), regardless of x-position").
- `crates/fw-match-sim/src/bt/on_ball.rs:684-685` — `// Width-only gate: no attacking-third condition.` then `let cross_gate = wide_pos;`.
- `crates/fw-match-sim/src/bt/on_ball.rs:696-703` — `raw × suppressor` clamped to `Q32::ONE` to prevent the `apply_cross_bias` assert panic.
- `crates/fw-match-sim/src/bt/personality_bias.rs:199` — `apply_cross_bias` assert enforces raw ∈ [0,1] (the clamp is load-bearing against this).
- `crates/fw-match-sim/tests/ts3b_proptest.rs:3-7` — claims `ZONE_SHORT_BOOST=3.0` in zones 0-8; production applies 3.2 universally.
- `crates/fw-match-sim/tests/ts3b_proptest.rs:44-45,72-81` — cites `MIDFIELD_ZONE=12`, `CROSS_CENTRAL_Y_M=10m`, `CROSS_WIDE_RANGE_M=24m`, `CROSS_BASE_SUPPRESS=0.30`, `CROSS_GATE_COEFF=0.70`, `CROSS_MIN_X_M=8m` — all differ from production (6/15, 8m, 26m, 0.28, 2.5, and non-existent respectively).

A deep-wide fullback (≈10m from goal, ≈25m off the centre-line) targets `target_x ≈ 40m`
(on_ball.rs:708-712), turning a long diagonal into something the BT scores as a cross. The
attacking-third gate was removed because in-possession-while-wide-AND-attacking was rare; the fix is
to restore zone gating without re-introducing the 0%-cross collapse.

### Fix — Phase 1 (do-now, after goal-production)

Restore x-position (zone) gating to the cross suppressor, scoped to `on_ball.rs` + `ts3b_proptest.rs`
only.

Add a per-zone factor array (zones 0-15, attacking direction):

- Zones 0-10 (deep own / mid): `0.0` — suppress crosses entirely, width irrelevant.
- Zones 11-14 (late mid → early final third): stepped/interpolated `0.3 → 0.8`.
- Zone 15 (final third): `1.0` — the full width gate applies.

Modify `utility_cross` (on_ball.rs:685) so the gate multiplies width by the zone factor:

```rust
// Zone-gated cross gate (FUN-TS3b-fix): width only matters in the final third.
// A deep-wide fullback (zone ≤ 10) is suppressed regardless of |pos_y|.
let cross_gate = wide_pos * CROSS_ZONE_FACTOR_BY_ZONE[zone_x];
```

Keep the clamp at on_ball.rs:696-703 — it is load-bearing (see risks). The clamp stays a no-op at
current mid-attrs and activates only when real PlayerTemplate attrs arrive.

Comment refresh (do-now, same commit):

- `on_ball.rs:642-648` — rewrite the Attempt-2 note: the gate is now zone-gated, not width-only.
- `on_ball.rs` zone-short note — state `ZONE_SHORT_BOOST=3.2` applied universally (not 3.0 in zones 0-8).
- `ts3b_proptest.rs:3-7` — replace the Attempt-2 descriptor with the real production values.
- `ts3b_proptest.rs:44-45,72-81` — replace every cited constant with the current value:
  - `MIDFIELD_ZONE=12` → the real `LONG_THRESHOLD_ZONE`/`LONG_NO_SUPPRESS_ZONE` pair (6/15).
  - `CROSS_CENTRAL_Y_M` 10m → 8m.
  - `CROSS_WIDE_RANGE_M` 24m → 26m.
  - `CROSS_BASE_SUPPRESS` 0.30 → 0.28.
  - `CROSS_GATE_COEFF` 0.70 → 2.5.
  - Remove the `CROSS_MIN_X_M` reference (non-existent in production).
  - Recompute the worked example: `wide_pos = (28-8)/26 = 0.769`; `suppressor = 0.28 + 2.5×0.615 = 1.814` (clamped to 1.0).

### Fix — Phase 2 (folds into FUN-TS4)

Attacking-width shape/buildup work is NOT part of the gate fix. The gate operates on `pos_x` +
`pos_y` only (already available). Getting players into final-third-wide positions in the first place
is formation/lane-openness redesign — that is FUN-TS4 shape work. Phase 1 lets a final-third-wide
player cross; Phase 2 ensures players actually arrive there with the ball. Keep the concerns
separate; do not over-gate Phase 1 to compensate for missing Phase 2.

### Tests to add (Phase 1)

- `deep_wide_cross_suppression`: player at `(10m, 25m)` (zone ≈10, deep-wide fullback). Assert
  `cross_util < 0.10 × short_util` — strong suppression, the diagonal-as-cross is gone.
- `final_third_wide_cross_competitive`: player at `(45m, 25m)` (zone 15, final-third winger). Assert
  `cross_util ≥ 0.75 × short_util` — zone_factor=1.0 + width applied keeps the genuine cross viable.
- `zone_interpolation_midfield`: player at `(20m, 25m)` (zone 11, late-mid wide). Assert
  `0.3 ≤ cross_util / baseline_cross_util ≤ 0.8` — the interpolation band is live.
- `cross_rate_match_gate`: drama sweep, 20 seeds × 600 ticks. Assert deep-wide crosses (zones 0-10)
  are below 5% of wide cross attempts — the distribution shows a sharp drop between final-third-wide
  and deep-wide scenarios.

### Rebaseline / risk

CROSS_CLAMP_ASSERT INTERACTION: the clamp at on_ball.rs:696-703 is load-bearing — it prevents the
`apply_cross_bias` assert (personality_bias.rs:199) from panicking when `suppressor > 1.0` (e.g.
y=34m/zone15: `0.28 + 2.5×1.0 = 2.78`). Do NOT remove it. It is a no-op at current mid-attrs
(raw ≈ 0.163) and necessary once real attrs land.

SHAPE FEEDBACK LOOP: Phase 1 alone may push cross% below the 3-10% floor, because deep-wide crosses
disappear. That is intentional — the shortfall is the signal that Phase-2 shape/buildup work is
owed. Do not over-tune Phase 1 to hit the floor; the target cross% is set AFTER FUN-TS4 shape work
lands, not before.

ZONE_FACTOR CALIBRATION: the thresholds (0.0 zones 0-10, ramp 11-14, 1.0 zone 15) are initial
guesses. The gate mechanism is the fix; exact factors iterate via `deep_wide_cross_suppression` +
the drama sweep. REBASELINE: cross-rate behaviour changes, so canonical hashes shift — authorized +
envelope-verify + independent re-measure, reason in the commit body. Re-run `ts3b_proptest` and
confirm `completion_ordering_mechanical` (Long > Cross) survives the new cross mix.

CONCURRENT EDITOR: goal-detection + goalkeeper code (`lib.rs ~1886-2110`, `goalkeeper_fsm.rs`,
`dispatch.rs`) is owned by another agent. This fix touches `on_ball.rs` + `ts3b_proptest.rs` ONLY —
no file overlap. If `cargo test` shows goal-related failures, treat them as pre-existing (fixed
concurrently); focus on the cross + pass-kind tests.

---

## #25 — FUN-TS2 offside spec-vs-code drift

**Verdict: CONFIRMED — a real DONE-vs-code drift.** MASTER_PLAN markets "cover-shadow" and an
"offside line that follows the last defender"; the shipped code delivers neither. The implementation
is sound and deterministic — the breach is the label, not the logic.

### Evidence

- `docs/MASTER_PLAN.md:438` — claims "cover-shadow" and "offside line that follows the last defender".
- `crates/fw-match-sim/src/subtree_library.rs:254-262` — the Cover role calls `enforce_hold_zonal` (static zonal hold; no cover-shadow geometry, no repositioning relative to the Primary presser or carrier).
- `crates/fw-match-sim/src/dispatch.rs:1619` — `let offside_line = state.team_shape[opp_team_idx].line_x;` — the static tactic-target line, explicitly NOT the 2nd-rearmost defender.
- `crates/fw-match-sim/src/dispatch.rs:1607-1618` — the code comments the choice as a deliberate T1 simplification (target line is stable, avoids false offside from defender drift; T2 to revisit with IFAB 2nd-rearmost).
- `docs/design/football-authenticity-gap-map.md:65-70` — flags this as a claimed-but-undelivered defect.
- `docs/DECISIONS.md` 2026-06-05 — flags FUN-TS2 offside as needing verification for cover-shadow and last-defender line.

### Fix — Option 1 (HONEST STATUS — do-now, zero code risk)

Relabel the spec to match the shipped T1 simplifications; no code change.

- `docs/MASTER_PLAN.md:438` — remove the "cover-shadow" and "offside line follows the last defender"
  claims. State what shipped: a static tactic-target-line offside check (`team_shape.line_x`) and a
  zonal-hold Cover role (`enforce_hold_zonal`). Mark cover-shadow geometry + IFAB 2nd-rearmost line
  as deferred to FUN-TS4.
- Align commit `11330476`'s description (where the false claim originated) — note the T1
  simplifications in the FUN-TS4 follow-up, do not amend history.

The implementation is deterministic and football-legible; this option closes the breach by telling
the truth about what shipped.

### Fix — Option 2 (REAL GEOMETRY — deferred into FUN-TS4)

Deliver the promised behaviour:

- Cover-shadow: the Cover role dynamically positions relative to the Primary presser's anticipated
  movement (jockeying/shadowing), instead of a static zonal hold.
- Offside line: switch the check to the actual 2nd-rearmost defender position instead of
  `team_shape.line_x`.

This is non-trivial and is exactly the work the T1 simplification deferred. It depends on tighter
defender shape recovery — which the FUN-TS4 shape/buildup work provides — so it folds into FUN-TS4
rather than landing standalone.

### Recommendation

Ship Option 1 now (honest label, no risk). Fold Option 2 into FUN-TS4, after goal-production realism
lands (DECISIONS priority #1) — goal-production supplies the cleaner defender positioning that
accurate 2nd-rearmost tracking needs.

### Tests to add

- Cover-hold fidelity (with Option 1 — documents the shipped behaviour): in `ts2_proptest.rs` TS2-P3,
  assert the Cover press role in HighPress maintains the exact `enforce_hold_zonal` target — the
  Cover intent never carries a jockeying/shadow offset relative to the Primary presser. This pins the
  honest current behaviour so a future cover-shadow change is a deliberate, visible diff.
- Offside-line staticity (with Option 1): a doc unit test in `dispatch.rs` (or `ts2_proptest.rs`)
  asserting `offside_line == team_shape[opp_team_idx].line_x` at every pass-launch tick, independent
  of actual defender positions — demonstrating the static line does not track the 2nd-rearmost.
- With Option 2 (FUN-TS4): replace the two staticity tests with their dynamic counterparts — Cover
  intent carries a shadow offset toward the Primary presser's anticipated path; offside line equals
  the 2nd-rearmost defender x within tolerance.

### Rebaseline / risk

Option 1: zero code risk; no canonical-hash change.

Option 2: (a) tight coupling — if defender shape recovery is insufficient, per-tick offside
recalculation re-creates the false-offside cascades that motivated the T1 simplification; (b)
performance — querying the 2nd-rearmost on each attacking pass is a repeated O(11) scan;
(c) regression — switching to a real offside line after 600+ ticks of stabilization can spike false
offsides mid-match, forcing re-tuning of block formation + offside-zone depth; (d) canonical
rebaseline — any offside behaviour change requires re-pinning under the multi-pin discipline
(authorized + envelope-verify + independent re-measure). Complete goal-production realism first; it
provides the cleaner defender positioning for accurate 2nd-rearmost tracking.

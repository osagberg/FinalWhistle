# Dynamic positioning model — attribute-and-context-driven off-ball movement
date: 2026-06-06
status: DESIGN — implementation ready, Layer 1 first

---

## Purpose and provenance

This document designs the replacement for the current flat positioning model, where every
player's movement target is a formation slot anchored at a static `line_x` and their speed is
a constant 8 m/s regardless of attributes.

**Confirmed current state (code-verified against main branch):**

- `team_shape.rs:393` `zonal_slot` returns a target that is a pure function of formation table
  + `TeamShape.line_x` + compactness. No opponent position. No possession phase. No ball_x.
- `dispatch.rs:109` `MAX_PLAYER_SPEED: Q32 = Q32::from_raw(8_i64 << 32)` — uniform for all 22
  players. Pace and acceleration are read in utility SCORE computation only, never in the
  movement-execution path.
- `team_shape.rs:277–369` `compute()` sets `line_x` from the tactic-FSM state
  (LowBlock/MidBlock/HighPress/CounterAttack) but does NOT shift the block relative to
  `ball_x` or to whether the team is in settled possession. The block sits at a static
  formation baseline regardless of phase.
- `match-fidelity-defects-2026-06-06.md` Campaign log: two onside-clamp attempts failed
  because the defensive line (2nd-rearmost defender) is stuck ~20m short of the goal line.
  The root cause is that the block never translates up/down the pitch with possession phase.

**The composition formula this design implements:**

```
target(slot) = role_base_slot(slot, tactic_state)
             ⊕ phase_translation(possession, ball_x, tactic_state, team_idx)
             ⊕ local_adjustment(slot, attributes, game_state)
```

Layers are strictly additive on the x-axis (forward/back) with independent y contributions.
Layer 1 is prerequisite for everything else. Layers 2 and 3 design the attribute-driven
adjustments that follow after the foundation is stable.

---

## Layer 1: Team-phase translation + pace-scaled speed

### 1.1 The problem being fixed

The defensive line for MidBlock home is `line_x = -13m`. The formation table places home
defenders at `x = -30m` and home forwards at `x = +10m`. After `zonal_slot` with
`compactness_v = 30`, `line_x = -13`:

- Home DEF target_x = -13m (defender anchors at line_x)
- Home FWD target_x = -13 + 40 × (30/40) = -13 + 30 = +17m

Away DEF (defending +x) with their MidBlock `line_x = +13`:

- Away DEF target_x = +13m
- Away FWD target_x = +13 - 30 = -17m

When home has settled possession in the opposition half (ball_x ≈ +25m), home's attacking
block should be at approximately:

- Home DEF at around +5m to +10m (pushed from -13m up the pitch)
- Home FWD at around +30m to +38m (pressing the box)
- Away DEF at around +35m to +40m (high line to hold attackers onside)

None of this happens today. Home defenders sit at -13m whether the team is attacking or
defending; the whole block never moves with possession.

### 1.2 Phase-translation formula

Define a scalar `phase_tx` (signed metres, positive = toward opponent goal) that shifts the
entire block along the x-axis on top of the tactic-state `line_x`. The shift is computed once
per tick in `team_shape::compute()` and stored in an extended `TeamShape`.

**Input variables:**
- `ball_x`: current ball x position in metres (Q32, range [-52.5, +52.5])
- `possession`: `Some(carrier_slot)` or `None`
- `tactic_state`: the team's current `TacticState`
- `team_idx`: 0 = home (attacking +x), 1 = away (attacking -x)
- `is_defending`: already computed in `compute()` from `possession`

**Lookup: settled-possession phase shift**

When `!is_defending` (this team has the ball) the block advances toward the opponent goal.
The shift scales with ball_x, reflecting how far play has progressed into the opponent half.

Define the normalised ball penetration for team 0 (home, attacks +x):
```
ball_penetration_home = clamp((ball_x + 52.5) / 105.0, 0, 1)
  — 0 at our goal line, 1 at their goal line
```

For away (team 1, attacks -x), mirror:
```
ball_penetration_away = clamp((52.5 - ball_x) / 105.0, 0, 1)
```

In Q32 without floats, and using the pitch half-length of 52.5m rounded to 53 for the integer
path (acceptable for a soft tuning parameter):

```
// team 0 (home, attacks +x):
//   raw_pen = ball_x + 52 (shift range [0, 104] m)
//   penetration = clamp(raw_pen, 0, 104) / 104  ← approximate 1/104 × raw_pen
//   Approximation: inv104 = 2^32 / 104 ≈ 41_329_227 raw Q32 bits
raw_pen_home = clamp(ball_x + Q32::from_int(52), Q32::ZERO, Q32::from_int(104))
penetration_home = raw_pen_home * Q32::from_raw(41_329_227_i64)  // ≈ /104

// team 1 (away, attacks -x):
raw_pen_away = clamp(Q32::from_int(52) - ball_x, Q32::ZERO, Q32::from_int(104))
penetration_away = raw_pen_away * Q32::from_raw(41_329_227_i64)
```

**Phase translation magnitude table (SOFT — Phase-1 tuning values):**

| tactic_state | max_attack_tx | max_defend_tx |
|---|---|---|
| LowBlock | +6m | -4m |
| MidBlock | +14m | -6m |
| HighPress | +22m | -8m |
| CounterAttack | +18m | -6m |
| SetPiece | 0m (no shift) | 0m |

`max_attack_tx` is the maximum forward shift applied when the team has the ball at the
opponent's goal line (penetration = 1.0). `max_defend_tx` is the maximum backward pull when
the team is out of possession and the opponent has the ball at the defending team's goal line.

Formula for a team in possession:
```
phase_tx = penetration × max_attack_tx
```

Formula for a team defending:
```
opponent_penetration = 1.0 - penetration  // how far the opponent has advanced
phase_tx = -(opponent_penetration × max_defend_tx)
// negative = pull back toward own goal
```

When possession is `None` (loose ball), use the last `is_defending` value (or default to `true`
with zero tx — conservative fallback that preserves pre-existing block position).

**Sign convention:**

`phase_tx` is always expressed as "metres toward the opponent goal." For home (team_idx=0) this
is `+x`; for away (team_idx=1) this is `-x`. The transform in `zonal_slot` adds `phase_tx` to
`line_x` before computing the slot target, using the team_idx sign already embedded in `line_x`.

**Why these numbers:**

- MidBlock max_attack_tx = +14m: home defenders at -13m advance to -13+14 = +1m (just past
  centre) with the ball at the opponent goal line. That is a realistic high defensive line for
  a team in settled attacking possession — defending from the halfway line rather than their
  own half. With compactness_v = 30m and scale, home MIDs advance to ~+16m and home FWDs to
  ~+25m, approaching the opponent box. This unblocks the offside clamp: the 2nd-rearmost
  defender at +1m correctly makes the offside line near centre, so forwards can stand at +20m
  without being flagged.

- MidBlock max_defend_tx = -6m: away defenders at +13m retreat an additional 6m to +19m when
  the home team has the ball near the away goal. The away FWDs at -17m are correct for a team
  sitting deep — they are about 35m from the home goal, a realistic deep-sitting compact block.

- HighPress max_attack_tx = +22m: home defenders advance from +2m (their tactic baseline) to
  +24m — a very high line appropriate for a team pressing in the opponent's half.

- The defend-side pulls are intentionally smaller than the attack-side advances. The block
  should NOT drop all the way to the goal line when defending deep — LowBlock's -28m baseline
  is already a low defensive line; pulling a further 4m on top is sufficient.

### 1.3 TeamShape changes

Add one field to `TeamShape`:

```rust
/// Possession-phase x-translation, applied on top of line_x in zonal_slot.
/// Signed metres toward the opponent goal (team-neutral sign convention;
/// applied as +phase_tx for home, -phase_tx for away within zonal_slot).
/// Non-canonical sidecar (#[serde(skip)]); recomputed from canonical inputs.
pub phase_tx: Q32,
```

`TeamShape::zero()` sets `phase_tx: Q32::ZERO` (preserves pre-existing byte output for the
`serde(skip)` default, which is not serialised anyway).

In `compute()` (file: `crates/fw-match-sim/src/team_shape.rs`, function `compute`), after
the `is_defending` calculation, compute `phase_tx` per §1.2 and store it.

In `zonal_slot()` (file: `crates/fw-match-sim/src/team_shape.rs`, function `zonal_slot`),
the x-transform becomes:

```
// Before: target_x = shape.line_x + relative_x * scale_v
// After:
effective_line_x = shape.line_x + phase_tx_for_team
target_x = effective_line_x + relative_x * scale_v
```

Where `phase_tx_for_team` is `shape.phase_tx` for home (team_idx=0) and `-shape.phase_tx` for
away (team_idx=1). This respects the team-neutral sign convention.

**Worked example — MidBlock, home in possession at ball_x = +30m:**

```
raw_pen_home = clamp(30 + 52, 0, 104) = 82
penetration_home = 82 / 104 ≈ 0.789

phase_tx = 0.789 × 14 ≈ 11.0m

effective_line_x (home) = -13 + 11 = -2m
home DEF target_x: -2 + 0 × scale = -2m  (just in home half; onside line near centre)
home MID target_x: -2 + 20 × (30/40) = -2 + 15 = +13m  (in opponent half)
home FWD target_x: -2 + 40 × (30/40) = -2 + 30 = +28m  (approaching opponent box)
```

**Worked example — MidBlock, away defending at ball_x = +30m:**

```
raw_pen_away = clamp(52 - 30, 0, 104) = 22
penetration_away = 22 / 104 ≈ 0.212  (home has ball 30m into away half)
opponent_penetration_for_away = 1.0 - 0.212 = 0.788
phase_tx (signed, toward away goal = -x) = -(0.788 × 6) ≈ -4.7m
away effective_line_x = +13 + 4.7 = +17.7m  (defenders pushed back toward +x goal)
away DEF target_x: +17.7m
away FWD target_x: +17.7 - 30 = -12.3m  (away strikers near halfway, tucked in)
```

### 1.4 Pace-scaled movement speed

**Current state:** `MAX_PLAYER_SPEED: Q32 = Q32::from_raw(8_i64 << 32)` — same for all
players. Pace and acceleration attributes have zero effect on actual movement. (file:
`dispatch.rs:109`, `apply_vel_toward_target` reads no attributes.)

**Design: per-player top speed derived from pace**

Replace the constant with a per-player cap computed before `apply_vel_toward_target` is called:

```
// pace is Q32 in [0, 1]; represents the player's sprint speed attribute.
// Range: a pace=0 player runs at V_BASE (a slow jog); pace=1 runs at V_PEAK.
// V_BASE = 6.5 m/s — minimum: slow, laboured recovery run.
// V_PEAK = 9.0 m/s — maximum: elite sprinter at full tilt.
// Formula: v_max = V_BASE + pace × (V_PEAK - V_BASE)
//        = 6.5 + pace × 2.5
//
// In Q32 raw bits:
//   V_BASE = 6.5 × 2^32 = 27_917_287_424
//   V_RANGE = 2.5 × 2^32 = 10_737_418_240
//   v_max = V_BASE + player.attributes.physical.pace × V_RANGE

const V_BASE: Q32 = Q32::from_raw(27_917_287_424_i64);  // 6.5 m/s
const V_RANGE: Q32 = Q32::from_raw(10_737_418_240_i64); // 2.5 m/s delta
fn player_v_max(pace: Q32) -> Q32 {
    V_BASE + pace * V_RANGE
}
```

This replaces the use of `MAX_PLAYER_SPEED` in `apply_vel_toward_target` for outfield players.
The GK uses a separate constant (GK top speed is typically lower; use 7.2 m/s = `30_924_800_000`
raw as the fixed GK cap — do not apply pace scaling to GK until goalkeeper attributes are
richer).

**Worked examples:**

- Player A: pace = 0.72 → v_max = 6.5 + 0.72 × 2.5 = 6.5 + 1.8 = 8.3 m/s. Close to the
  old constant (reasonable for an above-average athlete).
- Player B: pace = 0.50 → v_max = 6.5 + 0.50 × 2.5 = 6.5 + 1.25 = 7.75 m/s. The mid-range
  baseline player moves a bit slower — noticeable in build-up situations.
- Player C: pace = 0.90 → v_max = 6.5 + 0.90 × 2.5 = 6.5 + 2.25 = 8.75 m/s. Fast winger;
  visibly quicker than a slow centre-back.
- Player D: pace = 0.30 → v_max = 6.5 + 0.30 × 2.5 = 6.5 + 0.75 = 7.25 m/s. Slow striker;
  close to a centre-back in pace terms.

**Acceleration model (defer to Layer 2 implementation):** The per-player cap is the binding
constraint. In Layer 1 the speed still snaps to the cap instantly (no ramp). A full accel ramp
is a Layer 2 addition:

```
// Layer 2 addition (after Layer 1 is measured stable):
// Ramp: v_effective = min(v_current + accel_rate × dt, v_max_for_player)
// accel_rate = A_BASE + player.attributes.physical.acceleration × A_RANGE
// A_BASE = 3.0 m/s²; A_RANGE = 4.0 m/s² (total 3–7 m/s² range)
// At 60 Hz (dt = 1/60s): per-tick speed increment = 3–7 × 1/60 = 0.05–0.117 m/s
// A player at v=0 reaches v_max in: 8.3 / 5.0 ≈ 1.66s for accel=0.5 → 100 ticks.
```

The ramp is deferred because it adds a per-player velocity state that may interact with the
canonical hash. Layer 1 strictly keeps the stat-→-movement link to one attribute (pace), which
is already read in utility scoring, making the change easy to verify.

**Implementation notes:**

- `apply_vel_toward_target` currently reads no arguments beyond the `(state, slot_idx,
  target_x, target_y)` call at `dispatch.rs:846`. It needs `player_v_max(player.attributes.
  physical.pace)` passed in or computed inside.
- `MAX_PLAYER_SPEED_SQ` (dispatch.rs:114) must also become per-player; compute
  `v_max_sq = v_max * v_max` once before the magnitude check.
- The GK constant (7.2 m/s) replaces `MAX_PLAYER_SPEED` in the `Goalkeeper` match arm of
  `apply_intent`.
- `pass_completion.rs:126` `const V_MAX: Q32` (used for `PlayerSnapshot` in pitch control)
  should become a per-player field; for Layer 1 leave it at the existing 8 m/s — the impact
  on pitch-control geometry is second-order. Update in Layer 2 when the accel model lands.

### 1.5 Layer 1 — files and functions to change

| File | Function / constant | Change |
|---|---|---|
| `team_shape.rs` | `TeamShape` struct | Add `pub phase_tx: Q32` field |
| `team_shape.rs` | `TeamShape::zero()` | Set `phase_tx: Q32::ZERO` |
| `team_shape.rs` | `compute()` | Compute `phase_tx` from ball_x + possession + tactic_state (§1.2) |
| `team_shape.rs` | `zonal_slot()` | Apply `effective_line_x = line_x + phase_tx_for_team` before the x transform |
| `dispatch.rs` | `MAX_PLAYER_SPEED`, `MAX_PLAYER_SPEED_SQ` | Demote to defaults for GK; add `player_v_max(pace)` helper |
| `dispatch.rs` | `apply_vel_toward_target` (or call site in `apply_intent`) | Pass per-player v_max; use `v_max * v_max` for the sq-magnitude check |

`compute()` needs access to `state.ball.pos_x` — it currently takes `&MatchState`. That is
already available. No signature changes.

**What stays the same:** `zonal_slot`'s public signature does not change; callers pass the same
`(roster_slot, &shape, team_idx)`. The change is internal to the `(target_x, target_y)` it
returns.

### 1.6 Layer 1 goal-regression risk

**Risk: MODERATE-HIGH.** Phase translation moves the block up/down the pitch, which changes:

1. How far attackers are from goal at the moment they shoot → changes xG score → changes M1.
2. Whether the offside check (pass-launch zone gate) fires more or less often → changes
   pass-completion rate.
3. Whether defenders are nearer the ball when a pass is played → changes `lane_openness` in
   `resolve_pass_completion`.

The direction of effect is unclear until measured. The design anticipates that pushing the
attacking block up will increase shot xG (attackers closer) while also activating more
offside rejections (raising the effective block reduces onside space). Net effect on goals is
unknown.

**Mitigation:**
- Gate Layer 1 on the goal-regression check before re-pinning the canonical hash.
- Target: M1 remains in the range [2.0, 3.2] goals/match (the current calibrated band is
  ~2.5; allow ±25% drift before intervention).
- If goals spike: reduce `max_attack_tx` in the MidBlock/HighPress rows (they most strongly
  move FWDs toward goal).
- If goals collapse: the more likely scenario, given that pushing the block up may activate
  more offsides. Reduce `max_attack_tx` to a conservative first pass (e.g. 8m for MidBlock,
  14m for HighPress) and measure.

**Recommend first deployment values (conservative):**

| tactic_state | max_attack_tx | max_defend_tx |
|---|---|---|
| LowBlock | +4m | -3m |
| MidBlock | +9m | -5m |
| HighPress | +14m | -7m |
| CounterAttack | +11m | -5m |

Step to the full Phase-1 values only after verifying M1 is in band. Record each sweep in the
comments of `team_shape.rs` per the pattern already established there.

---

## Layer 2: Attribute-driven local game-reading adjustments

Design now; implement after Layer 1 is stable and measured.

### 2.1 Defenders — anticipation-driven lane cover and marking

**Motivation:** currently `utility_mark_player` targets the ball carrier's position for all
marking defenders. No defender ever steps into a passing lane ahead of the ball, and no
defender shadows a specific runner. High-anticipation/interception defenders should
measurably arrive at dangerous passing lanes earlier.

**Design: lane-cover target shift**

Compute a per-defender `lane_offset` that pulls their target toward the most dangerous
uncovered passing option. The offset is a secondary nudge on top of the Layer 1 target; it
does not override zonal structure.

Inputs available at decision time (all in `BtContext`):
- `carrier_pos: Option<(Q32, Q32)>` — the ball carrier's world position.
- `player.attributes.mental.anticipation` — how quickly the player reads the situation.
- `player.attributes.technical.interception` — read from the player's attribute set.
- `player.attributes.mental.positioning` — spatial sense.

**Weighting formula for lane-cover weight:**

```
lane_cover_weight = anticipation × 0.50 + interception × 0.30 + positioning × 0.20
// All three in Q32 [0, 1]; weights sum to 1.0.
// lane_cover_weight ∈ [0, 1].
```

**Offset magnitude:** the lane-cover offset shifts the defender's target_x toward the
carrier's x by at most `LANE_COVER_MAX = 8m`:

```
// When the carrier is ahead of the defender (common when defending deep):
// offset_x = lane_cover_weight × LANE_COVER_MAX × sign_toward_carrier_x
// Limit: offset must not move the defender past the carrier's x (overrun).
// LANE_COVER_MAX = 8m, Q32 raw = 34_359_738_368.
```

This means a high-anticipation/interception defender (weight ≈ 0.9) steps up ~7.2m toward the
carrier's passing range, while a low-quality defender (weight ≈ 0.3) steps up only ~2.4m.

**Per-runner marking target:**

For the `utility_mark_player` intent, extend from "target the carrier" to "target the most
dangerous untracked runner":

1. Scan the 3 nearest opponents not currently marked by a teammate (O(10) scan at decision
   time, slot order for determinism).
2. Score each by `threat = (1 - dist_to_opp_goal / 52.5) × (1 - dist_to_this_defender / 30)`.
   The first term rewards proximity to goal; the second rewards proximity to this defender
   (a defender marks runners they can reach, not the one 40m away).
3. The highest-threat unmarked runner becomes the `mark_target`.
4. The defender's target = `mark_target.pos + approach_offset`, where `approach_offset` is a
   short vector (2m) between the mark_target and their own goal (defend from the goal side).

**Worked example:**

Player A: anticipation = 0.78, interception = 0.65, positioning = 0.71.

```
lane_cover_weight = 0.78 × 0.50 + 0.65 × 0.30 + 0.71 × 0.20
                  = 0.390 + 0.195 + 0.142 = 0.727

offset_x = 0.727 × 8 = 5.8m (steps up ~6m toward the carrier's lane)
```

Player B: anticipation = 0.35, interception = 0.28, positioning = 0.40.

```
lane_cover_weight = 0.35 × 0.50 + 0.28 × 0.30 + 0.40 × 0.20
                  = 0.175 + 0.084 + 0.080 = 0.339

offset_x = 0.339 × 8 = 2.7m (much less proactive)
```

At 60 Hz and 7.8 m/s speed, player A covers 5.8m in ~0.45s. Player B covers 2.7m in
~0.21s. The high-anticipation defender gets to the lane approximately 0.24s earlier — enough
to noticeably affect whether a pass into that zone completes. This difference is visible in
the pitch-control calc (the defender's presence reduces `lane_openness` in
`resolve_pass_completion`).

### 2.2 Midfielders — hybrid cover/support

Midfielders in `is_defending` mode apply the same lane-cover weight as defenders (§2.1) but
with a lower `LANE_COVER_MAX = 5m` (they must also remain available for a counterattack
transition, so overcommitting is costly).

When `!is_defending` (team in possession), midfielders apply a **support-run offset**:

```
support_weight = off_the_ball × 0.40 + decisions × 0.40 + teamwork × 0.20
support_offset_x = support_weight × 6m  // create width or depth for the carrier
```

The direction of support_offset is away from the carrier's x (creating a passing option
behind them rather than running past).

### 2.3 Attackers — timed runs into space

**Motivation:** `utility_run_off_ball` currently targets `zonal_slot(roster_slot, shape,
team_idx)` — the same location as `hold_formation`. The two intents are functionally
identical in terms of movement target. An attacker with high `off_the_ball` should run
into the space BEHIND the last defender rather than standing in a zonal slot.

**Design: space-run target**

When a forward selects `RunOffBall` intent, compute a `run_target` that:

1. Starts from `zonal_slot` as the base.
2. Adds a forward offset (`run_forward_m`) clamped such that `run_target_x` does not exceed
   the 2nd-rearmost opponent x minus 1m (onside clamp). This is the Layer 2 integration of
   the offside clamp — it only applies to the `RunOffBall` intent, not to block-holding.

```
run_forward_m = off_the_ball × 0.55 + anticipation × 0.30 + decisions × 0.15
              // weight sum = 1.0; run_forward_m ∈ [0, 1] (normalised)
              // Scale to metres: run_m = run_forward_m × RUN_MAX_M
              // RUN_MAX_M = 12m (SOFT — Phase-2 tuning value).

run_target_x = base_x + (run_m × sign_toward_opp_goal)
run_target_x = clamp(run_target_x, own_goal_x_for_team, offside_line_x - 1m)
```

The `offside_line_x` is the 2nd-rearmost opponent's x (an O(10) per-tick scan for the
opponent team, sorted by absolute x descending, second element — cheaper than it sounds at
10 players). Computing it per-slot would be wasteful; compute it once per team per tick and
store in `TeamShape` as a non-canonical sidecar field `pub offside_line_x: Q32`.

**Worked example:**

Forward A: off_the_ball = 0.82, anticipation = 0.74, decisions = 0.61.
Opponent 2nd-rearmost defender at x = +5m (home perspective). Base zonal_slot_x = +17m.

```
run_forward_m (normalised) = 0.82 × 0.55 + 0.74 × 0.30 + 0.61 × 0.15
                           = 0.451 + 0.222 + 0.0915 = 0.765
run_m = 0.765 × 12 = 9.18m

run_target_x_unclamped = +17 + 9.18 = +26.18m
offside_clamp = +5m - 1m = +4m  (forward would be offside at +26m)
run_target_x = clamp(+26.18, -52.5, +4) = +4m  (onside)
```

Forward B: off_the_ball = 0.40, anticipation = 0.45, decisions = 0.50.
Same opponent line at +5m. Base zonal_slot_x = +17m.

```
run_forward_m = 0.40 × 0.55 + 0.45 × 0.30 + 0.50 × 0.15 = 0.22 + 0.135 + 0.075 = 0.43
run_m = 0.43 × 12 = 5.16m
run_target_x = +17 + 5.16 = +22.16m → clamped to +4m (also onside; both forwards clamp,
  but forward A would have run further if not blocked)
```

Note that both players are clamped to onside in this scenario. The difference matters when
the defensive line is higher (e.g. opponent line at +35m):

```
Forward A: run_target = +17 + 9.18 = +26.18 → clamp to min(+34, ...) → +26.18m (runs into box)
Forward B: run_target = +17 + 5.16 = +22.16 → +22.16m (stays further out)
```

Player A reaches the box; Player B creates a passing option further from goal. Both are
attribute-distinguishable and both are onside.

### 2.4 Layer 2 goal-regression risk

**Risk: LOW-MODERATE.** The layer-1 translation provides the foundation; the local adjustments
within layer 2 are smaller in magnitude (max 12m for a forward run, max 8m for a defensive
step-up). The most sensitive interaction is the onside clamp in the forward run: it prevents
the old "forwards loiter 20m past the line" failure mode, but if the opponent's line is very
low (a deep-sitting LowBlock), the clamp may strand attackers well short of goal.

Monitor: average FWD shooting distance before/after Layer 2. If FWDs are shooting from >25m
more than 60% of the time, `RUN_MAX_M` is too conservative. If they are offside-flagged on
more than 10% of attacking opportunities, the `offside_line_x - 1m` margin needs widening.

---

## Layer 3: Interception as movement + attribute-driven pick

Design now; implement after Layer 2 is stable.

### 3.1 Problem

`resolve_pass_completion` (`pass_completion.rs:193`) computes interception probability from
the defender's geometric proximity to the passing lane (via `pitch_control` at the lane
midpoint). The defender's `interception` attribute is NEVER READ in this function. A player
with `interception = 0.95` and a player with `interception = 0.10` have identical effect on
`lane_openness`. The two consequences:

1. Positioning (getting to the lane in Layer 2) now matters — but the quality of the
   intercept itself still does not.
2. "High interceptions" as an attribute identity is invisible in play.

### 3.2 Design: attribute-multiplied intercept probability

Extend `resolve_pass_completion` to read the interception attribute of the nearest defending
player in the lane.

**Step 1: identify the lane defender**

After computing `mid_x, mid_y` (lane midpoint), find the nearest defending team player to
that point (the same O(10) scan that pitch_control runs internally, but return the player
index rather than just the control value):

```rust
// scan defenders for nearest to (mid_x, mid_y); slot-order tiebreak
// result: (nearest_def_slot_idx, dist_sq_to_lane_midpoint)
```

**Step 2: attribute multiplier**

```
base_intercept_prob = (1 - lane_openness)  // existing term; 0 = open, 1 = contested
interception_quality = interception × 0.60 + anticipation × 0.25 + pace × 0.15
// intercept_quality ∈ [0, 1]; quality-modulated probability of actually winning the ball
// (vs. it cannoning off or the pass threading through)

adjusted_intercept_factor = base_intercept_prob × lerp(0.40, 1.20, interception_quality)
// lerp(0.40, 1.20, q): poor interceptor (q=0) → 0.40 (contested lane still mostly completes)
//                       average (q=0.5) → 0.80
//                       elite (q=1.0) → 1.20 (capped at 1.0 via final clamp)
//
// Use the adjusted_intercept_factor to replace the raw (1 - lane_openness) term
// in the p_complete formula. The existing LANE_FLOOR mechanism is preserved.
```

The revised lane gate:
```
lane_gate = LANE_FLOOR + (1 - LANE_FLOOR) × (1 - adjusted_intercept_factor)
          = LANE_FLOOR + (1 - LANE_FLOOR) × lane_openness_adjusted
```

Where `lane_openness_adjusted = 1 - adjusted_intercept_factor`. The formula structure is
unchanged; only the effective openness value gains an attribute multiplier.

**Worked example:**

Pass lane is moderately contested: `lane_openness = 0.53` (base).

Defender C: interception = 0.88, anticipation = 0.72, pace = 0.65.
```
interception_quality = 0.88 × 0.60 + 0.72 × 0.25 + 0.65 × 0.15
                     = 0.528 + 0.180 + 0.0975 = 0.806

adjusted_intercept_factor = (1 - 0.53) × lerp(0.40, 1.20, 0.806)
                           = 0.47 × (0.40 + 0.806 × 0.80)
                           = 0.47 × (0.40 + 0.645)
                           = 0.47 × 1.045 = 0.491  (capped at 0.47 since > base)
// cap: adjusted_intercept_factor = min(adjusted, 1.0 - LANE_FLOOR_EFFECTIVE)
// The cap preserves the invariant that a pass through even an elite defender on the lane
// still has a LANE_FLOOR probability of completing.

lane_openness_adjusted = 1 - 0.491 = 0.509
lane_gate = 0.70 + 0.30 × 0.509 = 0.70 + 0.153 = 0.853
```

vs. the baseline (no attribute adjustment):
```
lane_gate_baseline = 0.70 + 0.30 × 0.53 = 0.70 + 0.159 = 0.859
```

The high-interception defender lowers completion by an additional ~0.6pp in this lane. That is
small in isolation but compounds: a build-up sequence of 5 passes through the same high-quality
defensive midfielder drops completion by ~3pp across the chain, which is visible in match stats.

Defender D: interception = 0.22, anticipation = 0.35, pace = 0.50.
```
interception_quality = 0.22 × 0.60 + 0.35 × 0.25 + 0.50 × 0.15 = 0.132 + 0.0875 + 0.075 = 0.295
adjusted_intercept_factor = 0.47 × lerp(0.40, 1.20, 0.295) = 0.47 × (0.40 + 0.295 × 0.80)
                           = 0.47 × 0.636 = 0.299
lane_openness_adjusted = 1 - 0.299 = 0.701
lane_gate = 0.70 + 0.30 × 0.701 = 0.70 + 0.210 = 0.910
```

The low-interception defender effectively lets the pass through more freely — the lane_gate is
higher (0.910 vs 0.853), meaning the pass completion probability is ~5.7pp higher through
this defender's lane than through Defender C's lane for the same geometric contest. This is
the "good at interceptions actually matters" signal.

### 3.3 Layer 3 goal-regression risk

**Risk: LOW.** The interception multiplier is bounded between `lerp(0.40, 1.20, q)` and is
capped at the existing LANE_FLOOR. It redistributes completion probability within the already-
calibrated band rather than shifting the mean. A mid-quality team of defenders (interception ≈
0.50) produces nearly identical results to the current baseline. The effect is visible only
in differentiated match-ups: a high-interception midfield vs a low-quality passer.

Verify with the drama_sweep over 40 seeds: standard deviation of goals per match should not
change more than 0.3 from the Layer 2 baseline.

---

## Determinism notes

All arithmetic in Layers 1–3 is Q32. Specific checks:

- Phase translation (`phase_tx`): pure Q32 linear interpolation. No RNG. No floats. The
  `inv104` approximation for the penetration normalisation introduces a maximum error of
  `1/104 - inv104_q32_value` which is less than 1 ULP at Q32 precision — acceptable for a
  soft tuning parameter.
- Per-player v_max: Q32 multiply. No RNG. Deterministic from `pace` attribute in canonical
  state.
- Lane-cover offset and run offset: Q32 linear combinations of attribute values. No RNG.
  Pure functions of canonical inputs (attributes + positions + possession).
- Interception quality: Q32 linear combination. The `lerp` is `A + q × (B - A)`, both
  Q32-friendly. No RNG.
- The `offside_line_x` scan (O(10) per team per tick) uses `BTreeMap`-independent slot-order
  iteration over `state.players[0..11]` and `state.players[11..22]` — fully deterministic.
- No new `HashMap` anywhere in the design. All new sidecar fields on `TeamShape` are
  `#[serde(skip)]` (non-canonical).

---

## Implementation plan summary

### Slice 1: Layer 1a — phase translation (implement first, highest priority)

**Files:** `team_shape.rs` (struct + compute + zonal_slot)
**Test additions required:**
- Unit test: `phase_tx` is positive when `!is_defending` (team in possession in opp half).
- Unit test: `phase_tx` is negative (or zero) when `is_defending`.
- Proptest invariant: for MidBlock, team 0 in possession at ball_x > 0, effective_line_x for
  home DEF must be > -13m and < +10m (pushed up, not into the opponent's net).
- Proptest invariant: for any tactic state and possession, home DEF target_x must be ≤ home
  FWD target_x (block ordering preserved).
- Insta snapshot after the canonical hash re-pins.
- Drama-sweep M1 check before re-pinning: must stay in [2.0, 3.2].

### Slice 2: Layer 1b — pace-scaled speed (low coupling, can follow immediately)

**Files:** `dispatch.rs` (`player_v_max` helper, `apply_vel_toward_target` signature or call
site, GK constant)
**Test additions required:**
- Unit test: `player_v_max(Q32::ZERO) == V_BASE`.
- Unit test: `player_v_max(Q32::ONE) == V_BASE + V_RANGE`.
- Proptest invariant: per-tick displacement ≤ `player_v_max(pace) × dt` for all dt = 1/60s.
- Insta snapshot re-pin (pace change does not change WHICH action is chosen, only movement
  speed — hash should only drift if position integration is part of the canonical bytes).

### Slice 3: Layer 2 — defender lane-cover + forward run-target (after Slice 1 measured)

**Files:** `bt/off_ball.rs` (`utility_mark_player`, `utility_run_off_ball`),
`team_shape.rs` (add `offside_line_x: Q32` sidecar field, populate in `compute()`),
`dispatch.rs` (thread offside_line_x into BtContext or read from shape at call site)
**Test additions required:**
- Unit test: high-anticipation defender (anticipation = 0.9) selects a mark target closer to
  the carrier than a low-anticipation defender (0.2) in an identical game state.
- Proptest invariant: when `!is_defending` and `off_the_ball > 0.7`, forward `RunOffBall`
  target_x must be ≤ `offside_line_x - 1m` (onside).
- Drama-sweep: average FWD shooting distance should decrease (attackers closer to goal).

### Slice 4: Layer 3 — interception attribute in pass completion (after Slice 3 measured)

**Files:** `pass_completion.rs` (`resolve_pass_completion`, add `nearest_lane_defender` helper)
**Test additions required:**
- Unit test: high-interception defender (0.88) in the lane produces lower completion
  probability than low-interception defender (0.22) with identical geometry.
- Unit test: lane_gate is in [LANE_FLOOR, 1.0] for all attribute combinations.
- Drama-sweep: stddev of goals per match should not change more than 0.3.

---

## Phase-1 tuning values (conservative deployment)

Recorded here per design-docs/RULES.md §4. Not in SPEC or DECISIONS.

**Layer 1 — phase translation constants:**

| Name | Value | Notes |
|---|---|---|
| `PHASE_TX_MAX_ATTACK_LOW` | 4m | LowBlock attack shift |
| `PHASE_TX_MAX_ATTACK_MID` | 9m | MidBlock attack shift |
| `PHASE_TX_MAX_ATTACK_HIGH` | 14m | HighPress attack shift |
| `PHASE_TX_MAX_ATTACK_COUNTER` | 11m | Counter attack shift |
| `PHASE_TX_MAX_DEFEND_LOW` | 3m | LowBlock defend retreat |
| `PHASE_TX_MAX_DEFEND_MID` | 5m | MidBlock defend retreat |
| `PHASE_TX_MAX_DEFEND_HIGH` | 7m | HighPress defend retreat |
| `PHASE_TX_MAX_DEFEND_COUNTER` | 5m | Counter defend retreat |
| `PHASE_PENETRATION_PITCH_LEN` | 104 | Normalisation denominator (≈ pitch length - 1) |
| `inv104` (Q32 raw) | 41_329_227 | 2^32 / 104, rounded |

**Layer 1 — speed constants:**

| Name | Q32 raw | m/s equivalent | Notes |
|---|---|---|---|
| `V_BASE` | 27_917_287_424 | 6.5 m/s | Pace = 0 top speed |
| `V_RANGE` | 10_737_418_240 | 2.5 m/s | Speed delta from pace=0 to pace=1 |
| `V_GK` | 30_924_800_000 | 7.2 m/s | Fixed GK top speed |

**Layer 2 — game-reading constants:**

| Name | Value | Notes |
|---|---|---|
| `LANE_COVER_MAX_DEF` | 8m | Max defender step-up toward carrier lane |
| `LANE_COVER_MAX_MID` | 5m | Max midfielder step-up |
| `RUN_MAX_M` | 12m | Max forward run beyond zonal_slot (onside-clamped) |
| `ONSIDE_MARGIN` | 1m | Buffer below offside_line_x for forward run clamp |
| `APPROACH_OFFSET` | 2m | Defender positions goal-side of marked runner |

**Layer 3 — interception constants:**

| Name | Value | Notes |
|---|---|---|
| `INTERCEPT_QUALITY_W_INTERCEPTION` | 0.60 | Weight on interception attribute |
| `INTERCEPT_QUALITY_W_ANTICIPATION` | 0.25 | Weight on anticipation attribute |
| `INTERCEPT_QUALITY_W_PACE` | 0.15 | Weight on pace attribute |
| `INTERCEPT_LOW_MOD` | 0.40 | Multiplier for quality=0 defender |
| `INTERCEPT_HIGH_MOD` | 1.20 | Multiplier for quality=1 defender (capped to 1.0 by LANE_FLOOR) |

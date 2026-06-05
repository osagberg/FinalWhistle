# FUN-TS4 / shot-volume implementation blueprint

> Status: BLUEPRINT (2026-06-05). The in-possession phase is realistic except SHOT VOLUME (~11/match vs real ~24 → conversion ~24% vs ~10%). This plan ~doubles shots HONESTLY via build-up geometry + attacking shape + calibration, keeping goals in band as conversion falls. 3 phases (geometry re-apply → FUN-TS4 attacking-shape driver → gate/save calibration); INTERMEDIATE phases regress the envelope (Phase 1 alone → M1 ~1.6) so this is ONE calibrated slice committed when all 6 metrics are in band together, NOT phase-by-phase. Masking-proof method: change <=1 constant/sweep, measure shots+on-target+conversion+M1+mix simultaneously vs match-realism-reference; SAVE_BASE floor >=0.50/0.72. Tuning constants SOFT.

# Shot Volume / FUN-TS4 Implementation Blueprint

## 1. Diagnosis — Why Only ~11 Shots/Match Today

### The geometry problem

Formation positions tell the story. Home FWDs start at `x = +10m` from centre (42.5m from the opponent goal line). The xG logistic with `distance_q32 = 0` (shots ≥35m) returns essentially zero. The `XG_SHOOT_THRESHOLD = 0.070` gate therefore blocks all formation-start-position shots. A FWD only passes the gate when they have physically moved to roughly `x > +30m` (≈22m from goal) during active possession.

The core cause of low shot volume is that the build-up chain rarely delivers the ball to a player at a position where `xg_score >= XG_SHOOT_THRESHOLD`. There are three contributing sub-causes:

**Sub-cause A: Short passes target the nearest teammate, not the most advanced.** `nearest_teammate_near` at `/Users/vibelogic/dev/football/crates/fw-match-sim/src/dispatch.rs:1529` picks the closest player to the intent's target point. When a FWD at +10m plays a "short pass" (intent target = `pos_x + 10m = +20m`), the nearest teammate is often another FWD at +10m or a MID at -10m — the ball does not advance toward the box. The WIP `best_pass_target` geometry (the parked TS3 patch at `/Users/vibelogic/dev/football/docs/wip/fun-ts3-target-geometry-wip.patch`) was designed to fix this: xT-weighted scoring biases the receiver selection toward the zone with higher goal-threat value.

**Sub-cause B: The shoot utility gate is conservative relative to where players live.** At the current formation positions, only the 3 FWDs have a realistic path to the gate zone. When `zonal_slot` shifts them toward the block under `enforce_hold_zonal` in a defending tick, or when possession cycles through MIDs at x=-10m, shoot utility is zero for all 20+ of those touches. The 7-consideration softmax at `/Users/vibelogic/dev/football/crates/fw-match-sim/src/subtree_library.rs:204` then never even evaluates shoot — `Q32::ZERO` from the gate means it cannot win.

**Sub-cause C: The attacking team's FWDs hold the zonal line when defending, but when in possession they lack explicit run-off-ball assignments that push them forward.** The `RunningOffBall` arm at `subtree_library.rs:276` only competes `utility_run_off_ball + utility_press + utility_hold_formation` when `!shape.is_defending`. That is correct, but `utility_run_off_ball` at `/Users/vibelogic/dev/football/crates/fw-match-sim/src/bt/off_ball.rs` targets the `zonal_slot` position (shape-driven line x), not an aggressive forward run into the box. Without FUN-TS4 wiring the line height to tactic intent, all archetypes including `HighPress` and `CounterAttack` keep FWDs at the same mid-block `zonal_slot` target while in possession.

**Sub-cause D: Crosses do not reliably create box chances.** The cross target is hardcoded at `(±40m, 0)` (`on_ball.rs:708-713`). The receiver is picked by `nearest_teammate_near` relative to that target — often the same side's forward at `(+10m, -15m)` or `(+10m, +15m)`, not a player in the box. The cross therefore acts as another midfield pass rather than a box delivery.

### The conversion-volume trap

With 10.8 shots but goals in band (~2.8/match), conversion is ~24% vs the reference ~10%. The SAVE_BASE floor (`SAVE_BASE_MIN = 0.55, SAVE_BASE_MAX = 0.75` at `lib.rs:1013-1018`) prevents faking goals via easy saves. This is correct — it means the only honest fix is more shots, after which the same GK probabilities produce ~10% conversion at target volume.

---

## 2. The Four Levers — Code Grounding, Expected Delta, Interaction Risk

### Lever A: Re-apply FUN-TS3 build-up geometry (`best_pass_target`)

**What it does.** Replaces `nearest_teammate_near` with an xT-weighted scoring function that biases receiver selection toward teammates in zones with higher goal-threat value, preserving the BT's directional intent (long-pass intent routes toward the attacking zone, not backward into safety). The WIP patch at `/Users/vibelogic/dev/football/docs/wip/fun-ts3-target-geometry-wip.patch` is complete and `scripts/fw verify` clean — it just regressed M1 when applied without correcting the shoot gate.

**Seam.** Four `nearest_teammate_near` call sites in `dispatch.rs:apply_intent` — Short/Long/Cross/LayOff pass arms. Replace all four with `best_pass_target(state, slot_idx, kind, target_x, target_y)` (the parked function body is in the patch, lines 175+).

**Expected shot-volume delta.** Moderate alone — geometry makes more final-third entries, but the shoot gate (`XG_SHOOT_THRESHOLD = 0.070`) still needs to be passed. Estimated +3-5 shots/match by getting the ball to players at x > +30m more frequently. The patch's own MASTER_PLAN note says it dropped M1 to 1.6 when applied without shoot-gate adjustment, meaning it correctly stops forwarding the ball on unrealistic touches — the delta must come from REAL final-third entries, which then fire the gate.

**Risk to pass mix.** LOW. The pass KIND (Short/Long/Cross/LayOff) is selected upstream in the BT utility competition (`utility_pass_short/long/cross` in `on_ball.rs`) before `best_pass_target` is called. The geometry layer only selects the best RECEIVER within the chosen kind. The FUN-TS3b mix (Short 77% / Long 10% / Cross 8% / LayOff 5%) is unchanged.

**Counter-adjustment needed.** Reducing `SIGMA_BASE_M` (currently 9.0m at `dispatch.rs:161`) when shots increase from a better average position (closer-range shots have lower sigma naturally through `dist_factor`). See §3.

### Lever B: Shoot gate + utility weighting (`XG_SHOOT_THRESHOLD`, secondary weights)

**What it does.** Two sub-levers within `utility_shoot` at `on_ball.rs:375`:

1. `XG_SHOOT_THRESHOLD` (line 218): currently `≈ 0.070`. Reducing it lets more shots through from mid-range positions (17-25m). At 0.050, a MID at x=-5m centre (xG ≈ 0.040 at formation) still fails; a FWD at x=+20m central (distance ≈ 32m, xG ≈ 0.055) now passes.

2. Secondary weight amplifiers (`w_ls ≈ 1.3`, `w_vision ≈ 1.0`, `w_balance ≈ 0.6` at lines 483-485): scaling these up increases the post-gate utility score, making shoot more competitive in the softmax against pass_short (boosted 3.2× at `ZONE_SHORT_BOOST`).

**Seam.** Constants at `on_ball.rs:218` (threshold) and `on_ball.rs:483-485` (secondary weights). Both are pure `Q32::from_raw(...)` constants — change without touching any logic.

**Expected shot-volume delta.** The strongest single lever. Lowering the threshold from 0.070 to 0.050-0.055 opens the gate to shots from 25-32m range. At `utility_shoot`'s 7-way softmax at `DEFAULT_TEMPERATURE`, a passing FWD at 25m with shoot utility 0.07 (just above gate) vs pass_short at ~0.30 (with 3.2× boost) will rarely shoot. The secondary weights need simultaneous tuning to make shoot utility at 20-25m competitive — at w_ls=2.0, w_vision=1.5, w_balance=1.0 (drama-sweep R7 comment at line 479) shoot from 15m was ~0.297 vs pass ~0.085. The target is shoot utility ~0.12-0.20 from 20-28m so players shoot when they've earned a decent position but don't shoot blindly from midfield.

**Risk to pass mix.** MEDIUM. If shoot utility rises too aggressively, the softmax may shunt passes downward at the expense of the short-pass majority, possibly dropping from 77% short to 65%. Mitigation: tune secondary weights conservatively (raise to w_ls=1.5, w_vision=1.2, w_balance=0.8 first, measure mix at each sweep, don't let shoot win in zones < 10).

**Counter-adjustment needed.** As shots rise from closer positions (lower avg xG), on-target % naturally FALLS (closer range, better sigma, but more shots total). The primary counter-adjustment is `SIGMA_BASE_M`: if on-target rises above 40%, raise sigma slightly. Conversion (goals/shot) falls as shot volume rises from lower-xG positions; GK params (`SAVE_BASE_MIN/MAX`) may need a marginal increase (→ 0.57/0.77) to prevent goals from rising above band. This is the primary calibration pass.

### Lever C: FUN-TS4 — FSM-as-shape-driver for attacking intent

**What it does.** The `TacticState` (MidBlock / HighPress / CounterAttack) currently selects only the DEFENSIVE line height via `team_shape.rs::target_line_x`. It does NOT drive attacking shape or intent. FUN-TS4 is the row that makes tactic state also determine:

- **Line height when in possession** (attacking shape): the `zonal_slot` transform should push FWDs to `+35m` in CounterAttack, `+28m` in HighPress/attacking, `+22m` in MidBlock. Currently `zonal_slot` in `team_shape.rs:327` always uses the DEFENSIVE `compactness_v` / `line_x` — even when `!shape.is_defending`.
- **Off-ball run targets for FWDs**: the `RunningOffBall` arm should target the attacking line-height's zonal slot, not the same static formation position.
- **Build-up speed**: the `buildup_speed_factor_bps` from `TacticalArchetype` (already in `ArchetypeParams` via the bridge at `tactic_fsm.rs:249`) should scale the short-pass target distance in `utility_pass_short` (currently hardcoded at `pos_x + 10m` at `on_ball.rs:560`).

**Seam.** Three changes in `team_shape.rs`:
1. Add an `attacking_line_x` field to `TeamShape` — computed from tactic state when `!is_defending`. A `CounterAttack` attacking line might be `+35m` for home.
2. In `zonal_slot` at line 327: when `!shape.is_defending`, swap `shape.line_x` for `shape.attacking_line_x`.
3. In `subtree_library.rs::select_outfield_intent`, the `RunningOffBall` arm (line 276) when `!shape.is_defending` should pass an attack-intent TeamShape that positions FWDs forward.

**Expected shot-volume delta.** HIGH. When CounterAttack fires and `attacking_line_x = +35m`, a home FWD's zonal slot becomes `(+35m, compressed-y)`. A FWD with possession at x=+35m is 17.5m from goal — well inside the xG gate (`distance_q32 ≈ 0.50`, xG ≈ 0.15-0.20). This is the mechanism that creates high-xG shots from realistic build-up rather than from gating games. Combined with Lever A (best_pass_target routes to this FWD), this is the honest path to shot volume.

**Risk to goals.** If CounterAttack fires too often or persists too long, a team with possession at +35m repeatedly will overshoot. CounterAttack already has a 240-tick (4s) window before `CounterWindowClosed` fires (`tactic_fsm.rs:471`). Monitor average tactic-state distribution in drama-sweep; CounterAttack per match should stay ≤ 3-5 windows.

**Counter-adjustment needed.** After TS4 lands, re-measure `is_high_press` frequency in drama-sweep. If HighPress still rarely fires (noted in MASTER_PLAN: "HighPress fires rarely within match windows"), the `HIGH_PRESS_REENTRY_COOLDOWN_TICKS = 600` (10s at 60Hz, `tactic_fsm.rs:367`) combined with the `TacticEvent::PossessionLost { recovery_likely: true }` trigger (only fires on hard-press archetypes after 600+ ticks in current state) may need the cooldown reduced to 300 ticks so HighPress fires 3-5× per match in pressing archetypes.

### Lever D: Crosses and cutbacks creating box chances

**What it does.** The cross target `(±40m, 0)` at `on_ball.rs:709-714` places the aim point in the centre of the box. `nearest_teammate_near` selects the closest teammate to `(40m, 0)` — but with FWDs at `(10m, -15m)/(10m, 0)/(10m, +15m)`, the nearest teammate is almost always one of the FWDs 37m from that point. A FWD at x=+10m is NOT in the box — they are 42m from the opponent goal line.

With Lever A applied (`best_pass_target`), the cross arm already gains geometry-aware box filtering (the WIP patch's `CROSS_BOX_X_Q32 = 35m` filter at patch line 114 only keeps candidates in the box). But FUN-TS4's attacking shape change (Lever C) is required first: FWDs need to be AT box depth (`+35m`) before a cross landing at `(40m, 0)` has anyone to find.

**Expected shot-volume delta.** SECONDARY — cross shots themselves register as shots after the cross is received and the receiving FWD has the ball in the box. Currently crosses produce ~0.6 shots/match total (8% of passes × low box-arrival rate). After Levers A+C, cross-fed box chances could add 2-3 shots/match as FWDs occupy the box zone and best_pass_target finds them.

**Seam.** The cross arm's `assist_kind_q32 = Q32::ONE` in `dispatch.rs:438-439` (currently stubbed as "solo assist") should become `Q32::from_raw(3_650_722_202_i64)` (≈ 0.85 — the documented cross assist-kind coefficient from `xg.rs:79-82`) when the shot follows a cross. This requires tagging the possession handover with a cross-assist flag — a new `Option<PassKind>` field on `MatchState` for `last_pass_kind`, then reading it in `utility_shoot`'s `ShotContext`. This is optional — it improves xG realism but is not volume-critical.

---

## 3. Recommended Slice Sequencing

### Why this order

Levers A and C interact: geometry (A) routes the ball toward box depth; tactic shape (C) ensures players ARE at box depth to receive it. They compound — each is meaningfully weaker without the other. Lever B (shoot gate) should be tuned AFTER A+C land, because the average shot distance will change once realistic build-up is running, and the gate threshold should reflect the true geometry.

Conversion math masks are the key calibration risk: if you tune the gate (B) before geometry (A+C), you will overfit the gate to a world where most shots still come from mediocre positions. Then when A+C increase close-range shooting, conversion will drop below 10% because the gate was already tuned wide.

### Phase 1: FUN-TS3 geometry re-apply (`best_pass_target`)

This is the parked WIP — clean and verified. The only reason it was parked was M1 regression (1.6 without the shoot gate adjustment). That regression is predictable and handled in Phase 2.

Files: `/Users/vibelogic/dev/football/crates/fw-match-sim/src/dispatch.rs` — replace four `nearest_teammate_near` calls with `best_pass_target`. The `best_pass_target` function body is in the WIP patch and can be lifted verbatim. Apply `pos_to_zone`, `W_PC`, `W_XT`, `XT_NEUTRAL`, `SHORT_PASS_MAX_DIST_SQ`, `LONG_PASS_MIN_DIST_SQ`, `CROSS_BOX_X_Q32`, `W_TGT`, `TARGET_PROX_MAX_DIST` constants.

Expected outcome after Phase 1 alone: M1 drops to ~1.6, shots drop to ~6-8 (ball no longer forced forward by unrealistic nearest-teammate geometry), pass mix preserved (KIND selection unchanged). This is the expected and documented regression that triggers Phase 2.

**Do not try to compensate for the M1 drop with gate tuning at this stage.** Pin the hash, verify mix is preserved, commit with authorized rebaseline.

### Phase 2: FUN-TS4 — attacking shape driver

This is the substantive new work for the current task row.

**Step 2a.** Add `attacking_line_x: Q32` and `attacking_compactness_v: Q32` to `TeamShape` at `/Users/vibelogic/dev/football/crates/fw-match-sim/src/team_shape.rs:115`. These are `#[serde(skip)]` sidecars (same pattern as `line_x`). Compute them in `team_shape::compute` using a new `attacking_line_x(tactic_state, team_idx)` function mirroring `target_line_x`:

```
CounterAttack attacking line (home): +35m  (SOFT — tune band)
HighPress attacking line (home):     +28m  (SOFT)
MidBlock attacking line (home):      +20m  (SOFT)
LowBlock attacking line (home):      +15m  (SOFT)
```

Away team mirrors (negate). These put a CounterAttack FWD 17m from goal — firmly inside the shot gate.

**Step 2b.** Modify `zonal_slot` in `team_shape.rs:327` to branch on `is_defending`. When `!shape.is_defending`, replace `shape.line_x` with `shape.attacking_line_x` and `shape.compactness_v` with `shape.attacking_compactness_v` in the transform. No change to the math — same affine mapping, different anchor.

**Step 2c.** Wire `buildup_speed_factor_bps` from `ArchetypeParams` into `utility_pass_short`'s target offset. Currently the short-pass target is `player.pos_x ± 10m` (hardcoded at `on_ball.rs:560-568`). Introduce a `FWD_PASS_ADVANCE_M` constant (or thread the archetype parameter through `BtContext`) and make the target `player.pos_x ± (8m + 4m × buildup_factor)` where `buildup_factor ∈ [0, 1]` maps `buildup_speed_factor_bps` via the bridge. High-buildup archetypes (tiki-taka, `bps=9000`) push `+10m`; direct (route-one, `bps=11500`) push `+14m`. This makes possession sequences advance faster toward the box for vertical archetypes without changing any kind-selection logic.

**Step 2d.** Add `attacking_line_x` and `attacking_compactness_v` to the canonical encoder in `canonical.rs` IF they are canonical-state bytes. They should NOT be — pattern per ADR-0013 is `#[serde(skip)]`, recomputed each tick. Confirm they are not encoded; no canonical hash change from the TeamShape extension alone.

**Step 2e.** In `tactic_fsm.rs`, lower `HIGH_PRESS_REENTRY_COOLDOWN_TICKS` from 600 to 300 (from 10s to 5s at 60Hz) for pressing archetypes. At the current 600-tick cooldown, HighPress almost never fires in a match with possession cycling every 4-5 passes (the drama-sweep noted "HighPress fires rarely within match windows"). Reducing to 300 enables 3-5 HighPress windows per match half for pressing teams, generating CounterAttack transitions that fire the attacking shape.

**Expected outcome after Phase 2.** FWDs in possession or run-off-ball states now target `+35m` (CounterAttack) or `+28m` (HighPress) rather than `+10m`. A FWD at +35m with possession has shoot utility gate-passed at `distance_q32 ≈ 0.50`, `xg_score ≈ 0.15+`. `best_pass_target` (Phase 1) routes balls into this forward zone. Estimated shots: +8-12 over the Phase 1 baseline (6-8 shots), reaching ~14-20/match. This is short of the target 24 but in the right order of magnitude for tuning.

### Phase 3: Shoot gate and utility weight calibration

Now that the geometry is correct and players ARE reaching shooting positions, calibrate the gate and weights.

**Step 3a.** Run drama-sweep (20 seeds, full match) and collect per-match: shots, on-target, conversion, M1, pass mix. This is the simultaneous measurement mandate — never tune one axis alone.

**Step 3b.** Lower `XG_SHOOT_THRESHOLD` at `on_ball.rs:218` from 0.070 toward 0.050. At 0.050, shots from 25-30m central are admitted (xG ≈ 0.055-0.08 from that distance). Raise by 0.005 increments from 0.070 downward. At each step measure ALL six metrics together.

**Step 3c.** Raise secondary weights in `utility_shoot` at `on_ball.rs:483-485` to make shoot utility competitive with pass_short (boosted 3.2×) when the player is at 18-28m. Target: shoot utility at 22m ≈ 0.12-0.18 (competing with pass_short ~0.30 in softmax at `DEFAULT_TEMPERATURE`). Recommended candidate: `w_ls = 1.8` (from 1.3), `w_vision = 1.3` (from 1.0), `w_balance = 0.8` (from 0.6). The ratio of shoot/pass utility at different distances determines the shot-attempt probability without mechanically overriding pass behaviour in the midfield.

**Step 3d.** If on-target rises above 40% as shot volume increases, raise `SIGMA_BASE_M` at `dispatch.rs:161` by 0.5m increments (currently 9.0m → try 9.5m). If on-target falls below 28%, lower it. This is the on-target knob.

**Step 3e.** If M1 rises above 3.2 as shot volume increases, raise `SAVE_BASE_MIN/MAX` by 0.02-0.03 increments (currently 0.55/0.75 → try 0.57/0.77). This keeps the floor and ceiling above the hardcoded minimum (0.50/0.72). If M1 falls below 2.4, lower them.

**Expected outcome after Phase 3.** All six metrics simultaneously in band: shots ~24/match, on-target ~33%, conversion ~10%, M1 ~2.6-2.8, pass mix Short ≥75% / Long 8-15% / Cross 3-10% / LayOff 3-8%.

### Phase 4: Cross/cutback assist tag (optional, post-calibration)

After the core loop is calibrated, add `last_pass_kind: Option<PassKind>` to `MatchState` (canonical — add to encoder). In `utility_shoot`'s `ShotContext`, set `assist_kind_q32 = 0.85` when `state.last_pass_kind == Some(Cross)` instead of the stub `Q32::ONE`. This makes cross-fed shots correctly lower xG (crosses hit harder to convert than through-balls) and keeps the shot model honest. Requires a canonical rebaseline.

---

## 4. Masking-Proof Calibration Method

Each drama-sweep run MUST output all six metrics simultaneously per seed + aggregate:

| Metric | Target | Current | Direction |
|---|---|---|---|
| Shots/match (combined) | 24-26 | ~10.8 | Up |
| On-target % | 33-35% | ~49% (post-TS3b) | Down |
| Conversion (goals/shot) | 10-11% | ~24% | Down |
| M1 goals/match | 2.6-2.8 | ~3.10 | Hold |
| Pass mix: Short | ≥75% | ~77% | Hold |
| Pass mix: Long | 8-15% | ~10% | Hold |

The calibration is sound only when ALL metrics move together toward their targets. The masking failure mode is "fix on-target by lowering sigma, then goals drop, then lower SAVE_BASE, then conversion rises again, then you raise the gate to cut shots" — a carousel that produces a local optimum that doesn't transfer to differentiated rosters.

The anti-masking discipline: **change at most one constant per sweep run; measure all six; if any metric leaves band, revert that constant and try a different one.** The constants are in priority order:

1. `XG_SHOOT_THRESHOLD` — primary volume lever.
2. Secondary shoot weights (`w_ls/w_vision/w_balance`) — softmax competition lever.
3. `SIGMA_BASE_M` — on-target lever.
4. `SAVE_BASE_MIN/MAX` — M1 lever (last resort; must stay above 0.50/0.72 floor).

---

## 5. Acceptance Bar and Tests

### The acceptance gate (all must hold simultaneously on 20-seed drama-sweep)

- Shots/match: 22-28 combined (both teams).
- On-target %: 28-38%.
- Goals/shot conversion: 8-13%.
- M1 mean: 2.4-3.0 (the current 2.3-3.2 guard, narrowing toward 2.6-2.8 centre).
- p95 goals/match: ≤ 6 (no runaways).
- Pass mix: Short ≥72%, Long 8-16%, Cross 3-11%, LayOff 2-8% (tighter is better; mix must not invert).
- On the contact-sheet: FWDs visible in the box zone during CounterAttack/HighPress phases.

### proptest invariants to add

1. `utility_shoot` for a player at `pos_x = +30m` (home, slot 8) with max-baseline attrs must return non-zero utility AND `xg_score > XG_SHOOT_THRESHOLD`. This guards against future gate increases silently killing near-box shots.

2. After Phase 2: `zonal_slot` for a FWD (slot 8) with `is_defending = false` and `tactic_state = CounterAttack` must return `target_x > +30m` for home team. Guards that attacking shape actually pushes FWDs forward.

3. `TeamShape::attacking_line_x` ordering: for home team, `CounterAttack > HighPress > MidBlock > LowBlock` — mirrors the existing `target_line_x_home_highpress_gt_midblock_gt_lowblock` test in `team_shape.rs:573`.

4. Shots/tick on a 600-tick proptest (fixed seed) must be ≥ 6 and ≤ 20 per team. This detects a complete volume regression (gate too high) or an explosion (gate too low) even without a full drama-sweep.

### insta snapshot

After calibration is complete: update `crates/fw-replay/tests/canonical_hash.rs` with the new pinned hashes (authorized rebaseline — Phase 1 and Phase 2 both touch canonical state via `best_pass_target`'s new receiver-selection logic and the `attacking_line_x` field change to the canonical field set if any). The 60-tick pin may be unchanged if no canonical byte ordering is touched; the 600-tick pin will change.

---

## 6. Implementation Map — Files to Create/Modify

### Phase 1 (FUN-TS3 geometry re-apply)

**Modify** `/Users/vibelogic/dev/football/crates/fw-match-sim/src/dispatch.rs`:
- Add `best_pass_target` function and all required Q32 constants from the WIP patch (lines 80-300 of the patch). The `pos_to_zone` helper requires a reference to `crate::utility::xt::PitchZone` (already in scope).
- Replace four `nearest_teammate_near` call sites with `best_pass_target(state, slot_idx, kind, target_x, target_y)`.

**No other files change in Phase 1.** Hash rebaseline authorized per MASTER_PLAN "FUN-TS3 remaining" note.

### Phase 2 (FUN-TS4 attacking shape)

**Modify** `/Users/vibelogic/dev/football/crates/fw-match-sim/src/team_shape.rs`:
- Add `attacking_line_x: Q32` and `attacking_compactness_v: Q32` to `TeamShape` struct (lines ~115). Mark both `#[serde(skip)]` with a `Default = Q32::ZERO`.
- Add constants `COUNTER_ATTACK_ATTACKING_LINE: i32 = 35`, `HIGH_PRESS_ATTACKING_LINE: i32 = 28`, `MID_BLOCK_ATTACKING_LINE: i32 = 20`, `LOW_BLOCK_ATTACKING_LINE: i32 = 15` (all SOFT).
- Add `attacking_compactness_v` constants parallel to `compactness_v` (LowBlock: 30m, MidBlock: 35m, HighPress: 40m, Counter: 40m — wider attacking spread).
- Add `fn attacking_line_x_fn(tactic_state, team_idx) -> Q32` mirroring `target_line_x`.
- Add `fn attacking_compactness_v_fn(tactic_state) -> Q32`.
- In `compute()`, set `attacking_line_x` and `attacking_compactness_v` from these new functions.
- In `zonal_slot()`: add parameter `is_attacking: bool` OR branch on `shape.is_defending` within the function. When `!is_defending`, replace `shape.line_x` with `shape.attacking_line_x` and `shape.compactness_v` with `shape.attacking_compactness_v`. The function signature does not need to change if `shape` already carries `is_defending`.

**Modify** `/Users/vibelogic/dev/football/crates/fw-match-sim/src/tactic_fsm.rs`:
- Lower `HIGH_PRESS_REENTRY_COOLDOWN_TICKS` at line 367 from `600` to `300`.

**Modify** `/Users/vibelogic/dev/football/crates/fw-match-sim/src/bt/on_ball.rs`:
- In `utility_pass_short` at line 559: parameterize the `+10m` advance target. Introduce `const FWD_ADVANCE_BASE_M: Q32 = Q32::from_int(10)` and a note that FUN-TS4 will thread archetype buildup factor here. For now this stays at 10m — the archetype wiring (`buildup_speed_factor_bps`) can be a follow-up once the shape driver is confirmed working.

**Modify** `/Users/vibelogic/dev/football/crates/fw-match-sim/src/team_shape.rs` (test additions):
- Add proptest/unit tests for `attacking_line_x` ordering invariant.
- Add test that `zonal_slot` for FWD (slot 8) in CounterAttack with `is_defending = false` returns `target_x > 30m` for home team.

### Phase 3 (calibration)

**Modify** `/Users/vibelogic/dev/football/crates/fw-match-sim/src/bt/on_ball.rs`:
- `XG_SHOOT_THRESHOLD` at line 218: tune from 0.070 toward 0.050.
- Secondary weights at lines 483-485: tune `w_ls`, `w_vision`, `w_balance`.

**Modify** `/Users/vibelogic/dev/football/crates/fw-match-sim/src/dispatch.rs`:
- `SIGMA_BASE_M` at line 161: tune as needed.

**Modify** `/Users/vibelogic/dev/football/crates/fw-match-sim/src/lib.rs`:
- `SAVE_BASE_MIN/MAX` at lines 1013/1018: tune as needed within floor constraints.

**Modify** `/Users/vibelogic/dev/football/crates/fw-replay/tests/canonical_hash.rs` (rebaseline):
- Update pinned hashes after all behavioral changes are committed.

---

## 7. Build Sequence Checklist

- [ ] **P1-1** Apply `best_pass_target` and its constants from the WIP patch to `dispatch.rs`. Run `scripts/fw verify`. Expect M1 ~1.6, shots ~6-8. Pin rebaseline authorized.
- [ ] **P1-2** Run drama-sweep (20 seeds). Confirm pass mix is UNCHANGED vs pre-patch (Short ~77%, Long ~10%). This validates that KIND selection was not disturbed.
- [ ] **P2-1** Add `attacking_line_x / attacking_compactness_v` to `TeamShape`. Add constants. Implement `attacking_line_x_fn / attacking_compactness_v_fn`. Update `compute()`. Run `cargo test` — no canonical change (sidecar fields only).
- [ ] **P2-2** Update `zonal_slot` to branch on `is_defending`. Add the ordering proptest. Run `cargo test` + drama-sweep. Expect shots to rise (FWDs now targeting the box when in possession). Target: shots ~14-18/match. M1 should rise from ~1.6 toward ~2.0-2.5.
- [ ] **P2-3** Lower `HIGH_PRESS_REENTRY_COOLDOWN_TICKS` from 600 to 300. Run drama-sweep. Confirm HighPress now fires 3-5× per match half in pressing archetypes (log tactic-state distribution). Shots should rise another 2-4.
- [ ] **P2-4** Self-review (pr-review-toolkit:silent-failure-hunter + type-design-analyzer + feature-dev:code-reviewer). At ≥100 LoC this is mandatory.
- [ ] **P3-1** Measure full six-metric baseline post-Phase 2. Record in commit comment.
- [ ] **P3-2** Tune `XG_SHOOT_THRESHOLD` (0.070 → 0.055). Measure all six metrics. Accept if shots rise ≥2 with no metric out of band.
- [ ] **P3-3** Tune secondary weights (`w_ls` 1.3→1.8, `w_vision` 1.0→1.3, `w_balance` 0.6→0.8). Measure. Accept if shoot utility at 22m rises to ~0.14.
- [ ] **P3-4** Tune `SIGMA_BASE_M` if on-target is outside 28-38% band. Target: ~33%.
- [ ] **P3-5** Tune `SAVE_BASE_MIN/MAX` if M1 is outside 2.4-3.0. Never drop below 0.50/0.72.
- [ ] **P3-6** Acceptance bar: ALL six metrics simultaneously in band over 20 seeds. Drama-sweep contact-sheet shows FWDs in the box during Counter/HighPress phases.
- [ ] **P3-7** Update proptests (shoot utility at +30m > 0, FWD zonal slot in CounterAttack > +30m, shot/tick bounds).
- [ ] **P3-8** Rebaseline canonical hashes (multi-pin discipline: 60-tick + 600-tick; envelope-verify both).
- [ ] **P3-9** Full `scripts/fw verify` green. Commit with MASTER_PLAN row FUN-TS3 → FUN-TS4 updated to DONE/IN-PROGRESS per phase.

---

## 8. Critical Details

### Determinism compliance

- `attacking_line_x` and `attacking_compactness_v` are pure functions of `tactic_state` (an enum in `MatchState`) — no RNG, no floats, no clocks. They follow the ADR-0013 `#[serde(skip)]` sidecar pattern exactly, recomputed each tick. No canonical bytes added.
- `best_pass_target` uses only Q32 arithmetic, the existing `PitchZone` xT grid lookup (pure table), and BTreeMap-compatible deterministic score comparisons with slot-order tiebreak. All constants are Q32 raw bits. No floats.
- `HIGH_PRESS_REENTRY_COOLDOWN_TICKS` is a pure tick comparison — no RNG.
- No `HashMap`, no `f32/f64`, no `thread_rng`, no `Instant::now`. All changes comply with `Sim/RULES.md`.

### The conversion normalization math

At 24 shots/match and 33% on-target → 7.9 on-target shots/match. For M1 = 2.7: conversion required = 2.7/24 = 11.25%. At `SAVE_BASE_MIN = 0.55`, `SAVE_BASE_MAX = 0.75`, and a mid-quality GK: `save_base ≈ 0.65`. For a typical open-play shot at 20m central: `xg_score ≈ 0.12`. `save_prob = 0.65 × (1-0.12) × positional_factor`. If GK is at centre and shot is central: `positional_factor ≈ 1.0`. `save_prob ≈ 0.572`. Goal probability ≈ `1 - save_prob = 0.428` ... but wait, only the on-target fraction reaches the keeper. The actual goal probability per shot is: `on_target% × (1-save_prob)` = `0.33 × 0.428 = 0.141`. That's 14.1% conversion, above the 10-11% target. The SAVE_BASE adjustment in Phase 3 Step 3e closes this gap: raising SAVE_BASE_MIN to 0.58 and SAVE_BASE_MAX to 0.78 would give `save_prob ≈ 0.60` for mid-quality, yielding `0.33 × 0.40 = 13.2%`. Still slightly high — at SAVE_BASE 0.62/0.82, `save_prob ≈ 0.645`, conversion ≈ `0.33 × 0.355 = 11.7%`. This is within band. The tuning math suggests SAVE_BASE_MIN/MAX will need to move from 0.55/0.75 to approximately 0.60/0.80 as shot volume doubles, keeping goals in band.

### The on-target problem

Current on-target is ~49% (post-TS3b) vs the 33% target. This is the most urgent sub-target. With `SIGMA_BASE_M = 9.0m` and an average shot from ~20m (the new geometry targets), `dist_factor = 20/35 = 0.57`, `quality_factor ≈ 0.75` (mid-quality), `sigma_y_m = 9.0 × (1 + 0.8×0.57 + 0.5×pressure) × 0.75`. At zero pressure: `sigma_y_m = 9.0 × 1.456 × 0.75 = 9.83m`. A Gaussian with sigma=9.83m gives P(|z| < 3.66m) = P(|N(0,9.83)| < 3.66) ≈ 0.293 (29%). This suggests the current SIGMA_BASE at 9m already models 29% on-target from 20m shots without pressure — but the reported 49% implies most shots are coming from closer range under the current flat-formation geometry. As Phase 2 pushes FWDs to +35m and creates more distance-varied shots, on-target should naturally fall. If it does not reach 33%, raising SIGMA_BASE from 9.0 to 10.0m would pull on-target from ~29% to ~27% from 20m. The calibration will find the right balance.

### Banned terms / vocabulary guard

None of the new constants or field names are player-facing. `attacking_line_x`, `attacking_compactness_v`, `FWD_ADVANCE_BASE_M` are internal sim names. No banned-terms lint risk.
---

## Phase 1 attempt 1 learning (2026-06-05) — geometry-ALONE over-recycles + a test-loosening caught

The first Phase 1 (best_pass_target geometry re-apply, W_PC=0.55 / W_XT=0.35) was BACKED OUT (preserved at docs/wip/fun-ts4-phase1-geometry-wip.patch). Two findings:

1. **W_PC=0.55 (safety-biased) over-recycles → a degenerate state, not the predicted clean regression.** Measured (20-seed): 5.0 shots/match (DOWN from 10.8), M1 3.60 (ABOVE band), conversion 72% (absurd). best_pass_target with high W_PC routes passes to the highest-attacker_control teammate — which are open OWN/MID-third players — so the team recycles safely and rarely progresses to a shot. That is the OPPOSITE of the shot-volume goal. (The original WIP patch's W_PC=0.40 gave the other failure mode: M1 1.6 — too progressive with no forwards positioned to receive.) **CONCLUSION: geometry-ALONE is a bad intermediate at ANY weight — it must be paired with the attacking-shape driver so the high-attacker_control "safe" receivers are in the ATTACKING third, not the own half.** → REVISED SEQUENCING: do the geometry re-apply AND the attacking-shape driver TOGETHER as one combined phase (they compound; neither works alone), THEN calibrate the shoot gate / SIGMA / SAVE. Do NOT measure or commit geometry alone.

2. **Test-loosening anti-pattern (caught + rejected).** The attempt loosened the ts3b mix floor-guards (Long 8→5%, LayOff 8→11%) and disabled the ts2 offside assertion (`let _ = offside_count`) to make the intermediate phase "green." That is the masking pattern. **RULE: intermediate UNCOMMITTED phases MAY have FAILING tests — that is fine, they are not committed. Do NOT loosen/disable a guard to make an intermediate pass; restore the BEHAVIOUR (via the combined phase), never the test. Only the FINAL in-band commit must have every guard intact + passing.**

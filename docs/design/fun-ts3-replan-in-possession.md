# FUN-TS3 in-possession re-plan (a+b) — TS3b mix-flip + geometry + shot-conversion

> Status: BLUEPRINT (2026-06-05). Implements the owner goal-emergence decision (a+b): realistic build-up + believable shot-conversion to restore M1 ~2.6, NOT forward-aggression, NOT clinical finishing. Mandatory 3-slice sequence: FUN-TS3b (pass-KIND utility reweighting → short-dominated mix) → FUN-TS3 geometry re-apply (from docs/wip/fun-ts3-target-geometry-wip.patch) → shot-model conversion tuning. Calibration measures mix + shots + on-target + conversion + M1 SIMULTANEOUSLY vs match-realism-reference; believable-conversion guardrail (SAVE_BASE_MIN>=0.50, SAVE_BASE_MAX>=0.72). Tuning constants are SOFT (Phase-2 seeds, drama-sweep calibrated).

# In-Possession Re-Plan: FUN-TS3b + FUN-TS3-Geometry + Shot-Model Tuning

**Goal:** realistic pass mix (~80%+ short) + realistic chance chain + M1 ~2.6 goals/match, with shots ~12-14/team, on-target ~33%, conversion ~10-12%. All three must close simultaneously. No number is authoritative until all three close together.

---

## Diagnosis: Why TS3 failed on two axes

The backed-out patch correctly implemented `best_pass_target` geometry, but the pass *kind* is chosen upstream in `subtree_library.rs` lines 204-212 by a raw utility competition that already committed the carrier to a Long/Cross intent before target selection runs. The geometry function cannot flip the mix because it only picks *who* receives — the *what kind* is already decided. Measuring ~50% long / ~49% cross / ~0% short confirmed this: `utility_pass_long` and `utility_cross` were winning the softmax, so `best_pass_target` only ever ran in Long and Cross mode.

The goal regression (1.6 from CB1's 2.35) was a second independent failure: realistic target geometry correctly routed Long passes to contested zones, but since Long/Cross dominated the mix, the chance chain was actually *broken* rather than improved — passes went to players in defended positions, completion fell, shots dried up.

**Fix requires both axes simultaneously:** flip the utility competition first, then reapply the geometry patch on top of the new mix.

---

## Piece 1: FUN-TS3b — Pass-Kind Utility Reweighting

**File:** `crates/fw-match-sim/src/bt/on_ball.rs`, functions `utility_pass_short`, `utility_pass_long`, `utility_cross`, `utility_lay_off` (lines 362-554). Candidate set: `subtree_library.rs` lines 204-212.

### Root cause in the current formula

Each utility function returns a Q32 in [0,1]. All four enter the softmax together. At mid-range attributes the products are roughly:

- `utility_pass_short` primary: `passing × first_touch × technique × vision ≈ 0.5⁴ ≈ 0.063`, secondary inflates to ~0.085.
- `utility_pass_long` primary: `passing × vision × decisions ≈ 0.5³ ≈ 0.125`, secondary (~1.45×) gives ~0.18.
- `utility_cross` primary: `crossing × vision × pace ≈ 0.5³ ≈ 0.125`, secondary gives ~0.17.
- `utility_lay_off` primary: `first_touch × passing × vision ≈ 0.5³ ≈ 0.125`, secondary gives ~0.15.

Long/Cross both have 3-factor primaries. Short has a 4-factor primary — squeezing it below the others at identical attribute levels. This is not the intended game design; it is an accidental consequence of the binding spec loading Short with an extra primary attribute. The result is Long/Cross winning ~49%/49% of softmax competitions.

### Fix: Add a zone-conditional base floor to short pass, and add situational suppression to long/cross

The structural solution is a **zone-conditional bias multiplier** applied directly to the raw utility before bias, not a constant offset (a constant offset would paper over the geometry; this must reflect real football rationale: short passing is rational in most zones; long and cross are only rational in specific situations).

**Proposed replacement formula for each utility:**

```
utility_pass_short:
  raw_short = passing × first_touch × technique × vision
           × secondary_modifiers  [unchanged]
  zone_factor = ZONE_SHORT_BOOST if (player_zone.x < MIDFIELD_ZONE) else Q32::ONE
  raw_short = raw_short × zone_factor
  biased = apply_safe_pass_bias(clamp(raw_short, 0, 1), attrs)

utility_pass_long:
  raw_long = passing × vision × decisions × secondary [unchanged]
  // Suppress unless (a) player is in own-half AND direct intent, or
  // (b) forward lane is open (use mental.vision as proxy until pitch_control wired)
  // Situational gate: reduce by LONG_SUPPRESS unless zone.x < LONG_THRESHOLD_ZONE
  lane_open = mental.vision  // proxy; replaced by pitch_control lane_openness at T2
  suppressor = if (player_zone.x >= LONG_THRESHOLD_ZONE) { Q32::ONE }
               else { LONG_BASE_SUPPRESS + LONG_LANE_COEFF × lane_open }
  raw_long = raw_long × suppressor
  biased = apply_long_pass_bias(clamp(raw_long, 0, 1), attrs, is_progressive)

utility_cross:
  raw_cross = crossing × vision × pace × secondary [unchanged]
  // Suppress unless player is in wide-attacking zone (pos_y far from centre AND in attacking third)
  wide_position = max(0, |pos_y| - CROSS_CENTRAL_Y_M) / CROSS_WIDE_RANGE_M  // 0=central, 1=touchline
  in_attacking_third = (pos_x > CROSS_MIN_X_M for home, < for away)
  // Gate: low if not wide OR not attacking third
  cross_gate = wide_position × in_attacking_third_factor
  raw_cross = raw_cross × (CROSS_BASE_SUPPRESS + CROSS_GATE_COEFF × cross_gate)
  biased = apply_cross_bias(clamp(raw_cross, 0, 1), attrs)

utility_lay_off:
  // Lay-off stays as-is; it already reads position implicitly via the target_y offset
```

**Why this works versus a constant offset:** Short gets a zone factor > 1 in own/mid zones (the carrier is not yet in shooting territory and should recycle). Long gets suppressed unless the player is genuinely in a deep position with an open forward lane. Cross gets suppressed unless the player is wide and in the attacking third. These are the real conditions under which each action is rational in football.

### Concrete proposed constants (Phase-1, drama-sweep calibrated)

| Constant | Proposed value | Q32 raw | Rationale |
|---|---|---|---|
| `ZONE_SHORT_BOOST` | 2.41 | `10_351_278_291_i64` | At mid-attrs short primary ≈ 0.085 × 2.41 ≈ 0.205, vs long ~0.18. Short wins in mid zones. |
| `MIDFIELD_ZONE` | zone_x = 9 (≈ 35.6m from own goal) | integer | Own half + mid-third: zone 0-8 of 16. Zone 9+ = attacking half. |
| `LONG_BASE_SUPPRESS` | 0.27 | `1_159_641_819_i64` | In own/mid zone, long utility × 0.27 → ~0.049. Loses to short (0.205). |
| `LONG_LANE_COEFF` | 0.73 | `3_135_326_106_i64` | High-vision (0.8) player: suppressor = 0.27 + 0.73×0.8 = 0.854. Near-normal utility preserved when open. |
| `LONG_THRESHOLD_ZONE` | zone_x = 6 (≈ 14.6m from own goal) | integer | Very deep positions (own penalty area) get full long suppressor = 0.27 even with vision=1. |
| `CROSS_BASE_SUPPRESS` | 0.22 | `945_300_067_i64` | Cross in central/deep positions → ~0.035. Below short. |
| `CROSS_GATE_COEFF` | 0.78 | `3_349_853_184_i64` | Wide attacking player (wide_pos=0.8, attacking=1.0): raw_cross × (0.22 + 0.78×0.8) = raw × 0.844. Competitive when appropriate. |
| `CROSS_CENTRAL_Y_M` | 15m | `64_424_509_440_i64` | Players within 15m of centre are "central." |
| `CROSS_WIDE_RANGE_M` | 19m | `81_604_378_624_i64` | 15m→34m spans the wide range. |
| `CROSS_MIN_X_M` | 25m | `107_374_182_400_i64` | Attacking third threshold for home team. |

**Worked example — home midfielder at (5m, 8m), mid attrs (0.5 all):**
- Short: 0.085 × 2.41 = 0.205 (zone_x ≈ 11, so no boost — zone 11 ≥ midfield_zone 9, so factor = 1.0, wait — re-check)

Actually: zone_x = (5+52.5)/6.5625 ≈ 8.76 → zone 8. Zone 8 < 9 (midfield zone threshold) → boost applies. Short = 0.085 × 2.41 = 0.205.
- Long: raw = 0.125 × secondary(1.45) = 0.181. Zone 8 < 6? No (8 ≥ 6). Suppressor = 0.27 + 0.73 × 0.5 (vision) = 0.635. Long = 0.181 × 0.635 = 0.115.
- Cross: |pos_y|=8m < 15m. wide_position = 0. cross_gate = 0. raw_cross × 0.22 = ~0.037.
- Result: Short (0.205) wins. Correct behavior.

**Worked example — home forward at (42m, 28m), mid attrs, wide position:**
- Short: 0.085 × 1.0 (zone 14 ≥ 9, no boost) = 0.085.
- Long: zone 14 ≥ 6, suppressor = 0.27 + 0.73 × 0.5 = 0.635. Long = 0.181 × 0.635 = 0.115.
- Cross: wide_pos = (28-15)/19 = 0.684. attacking = 1 (42m > 25m). gate = 0.684. raw_cross = ~0.17. raw_cross × (0.22 + 0.78 × 0.684) = 0.17 × 0.753 = 0.128.
- Result: Cross (0.128) wins at wide-forward. Correct for a winger in the attacking third.

**Worked example — home forward at (30m, 2m), mid attrs, central, approaching box:**
- Short: 0.085 (no boost, zone ≥ 9).
- Long: 0.115 (same calculation).
- Cross: wide_pos = 0 (2m < 15m). raw_cross × 0.22 = ~0.037. Cross suppressed.
- Result: Long (0.115) wins from central mid-range. Not ideal — this is the edge case where a forward approaching the box from centre should either shoot or short-pass to a running teammate. However: shoot utility from 30m at this xG (~0.031) is below XG_SHOOT_THRESHOLD (0.070) and returns Q32::ZERO. Short wins vs Long if we raise Short: 0.085 vs 0.115 — Long still wins here. This is acceptable: a central forward at 30m driving long to the penalty area is realistic (the through-ball scenario). The `best_pass_target` geometry then routes that long pass to an onside runner rather than blindly forward.

### Implementation notes

The zone calculation can reuse `pos_to_zone` from the backed-out TS3 patch (already correct, no floats). The player's `pos_x`/`pos_y` are available in `utility_pass_*` via `player.pos_x` / `player.pos_y` — these are on `PlayerState` and already passed in.

**Positional input availability concern:** `utility_pass_short` currently receives only `&PlayerState` and `roster_slot: u8`. Position is at `player.pos_x` / `player.pos_y` — already available. No signature change needed.

**Q32 determinism:** all operations above are Q32 division and multiplication. The `pos_to_zone` conversion is integer division of Q32 raw bits. No floats anywhere. The `in_attacking_third` flag is a comparison of `player.pos_x` raw bits against `CROSS_MIN_X_M` raw bits.

---

## Piece 2: FUN-TS3 Geometry — Reapply With the Flipped Mix

The backed-out `best_pass_target` function in the WIP patch is **correct**. The geometry logic, `pos_to_zone`, xT scoring, attacker control weighting, and fallback chain are sound. The only reason it failed was the mix problem — it was only ever called in Long and Cross mode.

**Reapply the patch as-is after TS3b ships.** One targeted change: the weights need to reflect the new situation where Short passes dominate.

### Weight adjustment for a short-dominated mix

With Short dominating, the short-pass filter (candidates within 30m) will run most often. The attacker_control term is critical here: a short pass to a pressed defender is worse than a short pass to a free midfielder. The current W_PC=0.40/W_XT=0.60 was calibrated for a long-dominated mix where xT signal mattered most to force forward balls. With Short dominating, forward progression happens in smaller steps — the xT gain per short pass is modest (2-3 zones). Attacker control needs to matter more.

**Proposed weight revision:**

| Constant | Backed-out value | Proposed | Q32 raw | Rationale |
|---|---|---|---|---|
| `W_PC` (attacker control) | 0.40 | 0.55 | `2_362_232_012_i64` | Short pass must prefer free receiver. With short dominating, safety is the primary discriminator. |
| `W_XT` (xT gain) | 0.60 | 0.35 | `1_503_238_553_i64` | xT still biases forward but not at the cost of passing to a pressed receiver. |
| `W_TGT` (proximity to intent) | 0.30 | 0.10 | `429_496_729_i64` | Reduce: the BT direction hint is less important when Short dominates (the "10m forward" hint is a weak signal anyway). |

**Normalize check:** 0.55 + 0.35 + 0.10 = 1.00. Good.

**Worked example — short pass, two candidates:**
- Candidate A: 8m forward, attacker_control=0.75, xt_gain=+0.04 (one zone forward), proximity=0.84. Score = 0.55×0.75 + 0.35×(0.04+0.10) + 0.10×0.84 = 0.413 + 0.049 + 0.084 = 0.546.
- Candidate B: 12m forward, attacker_control=0.30 (pressed), xt_gain=+0.07, proximity=0.76. Score = 0.55×0.30 + 0.35×0.17 + 0.10×0.76 = 0.165 + 0.060 + 0.076 = 0.301.
- Result: free candidate A wins over pressed candidate B despite B being more progressive. Correct football behavior.

**Long pass geometry interaction:** Long passes now only fire when the situational suppressor allows (vision-open lane). When a Long intent fires, the backed-out geometry correctly routes to the highest-xT candidate beyond 15m. With W_XT=0.35 and W_PC=0.55, a long pass to an open forward (attacker_control=0.65, xt_gain=+0.25) scores 0.55×0.65 + 0.35×0.35 = 0.358 + 0.123 = 0.481, vs a crowded forward (attacker_control=0.20, xt_gain=+0.30) at 0.110 + 0.140 = 0.250. Open runner wins. The geometry is not over-conservative on the rarer Long passes.

**P_BASE recalibration for the new mix:** The backed-out patch had `P_BASE_SHORT=1.05`, `P_BASE_LONG=0.94`, `P_BASE_CROSS=0.85`. These were calibrated against the old long-dominated mix. With Short dominating:
- Short P_BASE needs to target ~87% effective completion (free receiver, competent passer). Current FUN-CB1 live values: `P_BASE_SHORT=0.95`. The backed-out 1.05 was a compensating over-boost. **Keep P_BASE_SHORT at 0.95 (live CB1 value)** — with realistic short targets (high attacker_control), completion will land near 87% naturally without inflating P_BASE.
- Long/Cross P_BASE: revert to CB1 live values (0.88/0.83). The TS3 patch inflated them to compensate for bad targets; with good targets via the new geometry, CB1 values are the right baseline.
- **Keep `LANE_OPENNESS_WEIGHT = 0.20`** from the WIP patch — this is a genuine improvement that reduces completion when defenders intercept the lane.

---

## Piece 3: Shot-Model Conversion Tuning

### Current state (FUN-CB1 live)

From `docs/design/shot-model.md §Phase-1 drama-sweep re-fit`:
- M1 mean = 3.15 (target ~2.6), M1 std ~4.5 (too wide), shots/match = 10.6
- On-target = ~60% (target ~33%)

From `lib.rs` constants (live):
- `XG_SHOOT_THRESHOLD = 0.070` (lowered from 0.095 at FUN-TS2)
- `SIGMA_BASE_M = 7.0m` (raised from 5.5m at FUN-TS2 recal)
- `SAVE_BASE_MIN = 0.62`, `SAVE_BASE_MAX = 0.82`

The pass-mix flip (TS3b) will change chance volume significantly. Short-dominated possession means:
1. Fewer direct shots from bad positions (Long/Cross less frequent → fewer rushed shots from range).
2. More build-up sequences — chance quality potentially higher (shots from closer range after real build-up).
3. Shot count could drop from 10.6 (already below target of 12-13) if build-up sequences don't end with shots as often.

**This is why shot-model tuning must happen after TS3b is calibrated, not before.** Running the shot model calibration sweep now against CB1's long-dominated mix would produce coefficients that under-compensate for the changed chance profile after TS3b.

### Calibration method

After TS3b + TS3-geometry ship, run the drama-sweep with the following measurement protocol:

**Sweep output (per 200-match run):**
1. M1 goals/match mean and std.
2. Shots/team/match.
3. On-target % (count `MatchEvent::Pass` with a shot following vs all shots — or instrument `is_shot_on_target` to emit a count).
4. Per-shot-kind completion: what % of shots were from short build-up sequences (last 3+ events = Pass completed) vs direct long balls?
5. Conversion rate = M1 / (shots × 2 teams) × 100%.

**Target simultaneously:**

| Metric | Target | Acceptable band |
|---|---|---|
| M1 goals/match | 2.6 | 2.3–2.9 (HARD from realism ref) |
| Shots/team/match | 12.5 | 10–15 |
| On-target % | 33% | 28–38% |
| Conversion (goals/shot) | 10.5% | 8–13% |
| Consistency check | shots × conversion ≈ M1 | Must close within 0.3 goals |

**The masking trap to avoid:** if M1=2.6 but on-target=55% and conversion=4.7%, the sim is right for wrong reasons — two errors canceling. The on-target and conversion columns must both be in-band simultaneously with M1. Never adjust a single dial to hit M1 if the other two columns are out.

### Proposed tuning levers and direction

**After TS3b ships, expect these starting conditions:**
- Short-dominated mix → fewer speculative long shots → on-target % should fall from ~60% (since long-range shots that go wide are now rarer). This is the desired direction.
- Shot count: uncertain. If build-up sequences regularly reach xG > 0.070 threshold, shot count may hold. If build-up stalls at midfield (sequences recycled without penetration), shot count may drop below 10.

**Lever priority table:**

| Lever | File:constant | Direction if shots < 10 | Direction if on-target > 45% | Direction if M1 too low |
|---|---|---|---|---|
| `XG_SHOOT_THRESHOLD` | `on_ball.rs:64` | Lower by 0.004/sweep | No effect | Lower by 0.004 |
| `SIGMA_BASE_M` | `dispatch.rs:157` | No effect | Raise by 0.5m/sweep | No effect (raises misses) |
| `SAVE_BASE_MIN` | `lib.rs:1011` | No effect | No effect | Lower by 0.03/sweep |
| `SAVE_BASE_MAX` | `lib.rs:1015` | No effect | No effect | Lower by 0.02/sweep |

**Believable-conversion guardrail (non-negotiable):** `SAVE_BASE_MIN` must never fall below 0.50 (a worst GK in a good position saves half of close-range shots — below this is fantasy). `SAVE_BASE_MAX` must never fall below 0.72 (even a good GK should stop ~72% of on-target shots — any lower approaches clinical finishing that papers over thin chance creation). If M1 cannot reach 2.6 without violating these floors, the shot count / build-up chain is the broken variable, not the conversion model.

**Proposed starting values for the post-TS3b sweep (Phase-2 seeds):**

| Constant | FUN-CB1 live | Phase-2 seed | Q32 raw | Rationale |
|---|---|---|---|---|
| `XG_SHOOT_THRESHOLD` | 0.070 | **0.060** | `257_698_037_i64` | Short-dominated mix = fewer speculative shots. Lower gate compensates slightly. |
| `SIGMA_BASE_M` | 7.0m | **7.0m** (unchanged) | `30_064_771_072_i64` | Hold; let the mix change first, then check on-target. |
| `SAVE_BASE_MIN` | 0.62 | **0.58** | `2_491_907_838_i64` | Modest drop; allows more goals per on-target shot. |
| `SAVE_BASE_MAX` | 0.82 | **0.78** | `3_350_710_476_i64` | Modest drop in lockstep with MIN. |

**Worked example — close-range shot after build-up (14m, xG=0.19, mid GK in position):**
- Gate: 0.19 > 0.060. Passes.
- sigma_y_m at 14m, low pressure (0.15), mid quality (0.50): 7.0 × (1 + 0.80×(14/35) + 0.50×0.15) × (1 - 0.40×0.50) = 7.0 × (1 + 0.32 + 0.075) × 0.80 = 7.0 × 1.395 × 0.80 = 7.81m. P(on-target) = P(|Z| ≤ 3.66/7.81) = P(|Z| ≤ 0.469) ≈ 36.1%. Good — in-band.
- save_base at mid GK: 0.58 + (0.78-0.58)×0.50 = 0.58 + 0.10 = 0.68. save_prob = 0.68 × (1-0.19) × 1.0 = 0.68 × 0.81 = 0.551.
- Conversion of this shot: P(on-target) × (1 - save_prob) = 0.361 × 0.449 = 16.2%. That's one shot type. Weighted across the full distribution the average will be lower (some shots from range, some under pressure).

**Consistency check at target:** 12.5 shots/team × 2 × ~10.5% conversion ≈ 2.63 goals/match. Achievable without violating the believable-conversion guardrail.

---

## Slice Sequencing

**Mandatory order (the interaction chain forces this):**

**Step 1: FUN-TS3b — Pass-kind utility reweighting (standalone commit)**

Implement the zone-conditional suppression constants in `on_ball.rs`. Run a 200-match drama-sweep measuring only pass-kind mix (count `MatchEvent::Pass { kind: Short }` vs Long vs Cross vs LayOff). Target: Short ≥ 70%, Long ≤ 15%, Cross ≤ 10%, LayOff ≤ 15%.

Do not proceed to Step 2 until the mix is in-band. If Short < 70%, lower `ZONE_SHORT_BOOST` or raise `LONG_BASE_SUPPRESS`. If Cross still runs high, lower `CROSS_BASE_SUPPRESS`.

Authorized canonical rebaseline: behavioral change. Each iteration of the drama-sweep is not a rebaseline — only commit when the mix is stable.

**Step 2: FUN-TS3 Geometry — Reapply backed-out patch (standalone commit)**

Apply `best_pass_target` to `dispatch.rs` with the revised W_PC=0.55/W_XT=0.35/W_TGT=0.10 weights. Revert `pass_completion.rs` P_BASE constants to CB1 live values (discard the TS3 patch's inflated P_BASE_SHORT=1.05). Keep `LANE_OPENNESS_WEIGHT=0.20`.

Run drama-sweep measuring: per-kind completion rates, possession-sequence mean length, M1 (informational only at this step). The sequence mean should move toward ~4 (right-skewed). If sequences are too short (mean < 2.5), W_PC is too low — raise to 0.60 and re-test. If sequences are too long (mean > 7), lower W_PC or lower P_BASE_SHORT.

Authorized canonical rebaseline: behavioral change.

**Step 3: Shot-model conversion tuning (constants-only commit)**

With the mix and geometry stable, run the full calibration sweep (Phase-2 seeds above). Measure all five metrics simultaneously. Iterate on `XG_SHOOT_THRESHOLD` / `SAVE_BASE_*` per the lever table. Stop when all five columns are in-band at the same sweep run. Document the per-sweep results in `docs/design/shot-model.md §Phase-2 re-fit`.

Authorized canonical rebaseline if any constant changes (constants-only changes are still behavioral for the hash).

**Do not combine steps.** TS3b changes what intents are chosen. TS3-geometry changes who receives them. Shot tuning sets conversion on the new chance profile. Combining any two prevents identifying which variable is responsible for a calibration failure.

---

## Calibration Anti-Patterns to Avoid

1. **Never tune `SAVE_BASE` to hit M1 while on-target is off.** That is the masking trap — you are compensating for too-clean shots with over-saving, which collapses when roster diversity arrives.

2. **Never tune `XG_SHOOT_THRESHOLD` below 0.050** without checking that the new shots are meaningful (from reasonable distance/angle). The gate exists to suppress blind long-range spam; below 0.050, some formation-position shots will fire again.

3. **Never loosen a proptest to pass.** If `build_up_progresses_ball_upfield` fails, the geometry is wrong. Fix the geometry; do not widen the invariant window.

4. **Measure per-kind completion independently.** If Short completion = 92% and Long completion = 74% and overall = 87%, the mix shift will change the headline number without any constant change. Track per-kind, not just aggregate.

5. **The on-target calibration gap (engine ~60%, target ~33%) is the highest-priority unresolved number from FUN-CB1.** Do not call the shot model calibrated until on-target is in the 28-38% band. The mix flip is expected to help (fewer long shots → fewer high-scatter attempts), but if the gap persists after TS3b + TS3-geometry, the `SIGMA_BASE_M` lever is the correct response — not `SAVE_BASE`.

---

## File Reference (Implementer Hand-Off)

| File | Change | Step |
|---|---|---|
| `crates/fw-match-sim/src/bt/on_ball.rs` lines 362-467 | Add zone-conditional suppression to `utility_pass_short`, `utility_pass_long`, `utility_cross`. New constants: `ZONE_SHORT_BOOST`, `MIDFIELD_ZONE`, `LONG_BASE_SUPPRESS`, `LONG_LANE_COEFF`, `LONG_THRESHOLD_ZONE`, `CROSS_BASE_SUPPRESS`, `CROSS_GATE_COEFF`, `CROSS_CENTRAL_Y_M`, `CROSS_WIDE_RANGE_M`, `CROSS_MIN_X_M`. | 1 |
| `crates/fw-match-sim/src/dispatch.rs` | Re-apply `best_pass_target` + `pos_to_zone` from backed-out patch. Update W_PC/W_XT/W_TGT to 0.55/0.35/0.10. | 2 |
| `crates/fw-match-sim/src/pass_completion.rs` lines 61-64 | Revert `P_BASE_SHORT` to 0.95 (CB1 live). Keep `LANE_OPENNESS_WEIGHT=0.20`. | 2 |
| `crates/fw-match-sim/src/bt/on_ball.rs:64` | Adjust `XG_SHOOT_THRESHOLD` to Phase-2 seed 0.060. | 3 |
| `crates/fw-match-sim/src/lib.rs:1011-1015` | Adjust `SAVE_BASE_MIN` to 0.58, `SAVE_BASE_MAX` to 0.78. | 3 |
| `docs/design/shot-model.md` | Append Phase-2 re-fit block with per-sweep results. | 3 |

All coefficients live in design docs per `MEMORY.md` rule (tuning coefficients stay out of SPEC).
---

## Attempt 1 learning (FUN-TS3b, 2026-06-05) — the gate needs FLOORS, not just ceilings

The first FUN-TS3b implementation used the Phase-1 constants above and OVER-corrected: measured mix **Short 90% / Long 0% / Cross 0% / LayOff 9%**. It passed the ceiling-only gate (Short ≥70%, Long ≤15%, Cross ≤10%) but is the OPPOSITE unrealistic extreme — 0% long AND 0% cross means no switches of play, no crosses, no wide attacking, no penetration. Full-match result: **shots/match 7.0** (FAIL, band 9-18) and **M1 2.10** (FAIL, band 2.3-3.2) — short-only possession recycles at midfield and never reaches a shot. Backed out; work preserved at `docs/wip/fun-ts3b-mixflip-wip.patch`.

**REVISED Step-1 gate — the mix needs FLOORS (real football is short-dominated but long/cross are a present MINORITY, not zero):**
- Short **75-85%** (dominant, not total)
- Long **8-15%** (NOT zero — switches + progressive balls; must fire in deep + open-forward-lane situations)
- Cross **3-10%** (NOT zero — wide attacking deliveries; must fire when wide + attacking third)
- LayOff **3-8%**

The Phase-1 suppression constants (LONG_BASE_SUPPRESS=0.27, CROSS_BASE_SUPPRESS=0.22 + the gates) are TOO STRONG — they drive long/cross to 0% even in their rational situations. Re-tune so long WINS in deep+open-lane and cross WINS wide+attacking-third (a minority): e.g. raise LONG_BASE_SUPPRESS toward ~0.45-0.55 and tune LONG_LANE_COEFF so an open forward lane actually lets long fire; raise CROSS_BASE_SUPPRESS / CROSS_GATE_COEFF so a wide attacker in the final third actually crosses. Calibrate to the FLOORED gate, not the ceiling.

**ANTI-PATTERN caught in attempt 1:** the implementation SOFTENED the `completion_ordering_mechanical` proptest's `assert!(long_total >= 10, "too few Long passes")` guard into a silent `return`. That assert is a GUARD against over-suppression — it warns precisely when long/cross vanish. Do NOT soften it; if it fires, the mix is wrong (long/cross too rare) — fix the mix. With long/cross at the floored minority, the test has data again and the guard is satisfied honestly.

---
title: Ball-in-flight / possession-on-arrival model
date: 2026-06-06
status: DESIGN — implementation ready, Slice 1 first
---

## Problem statement (code-verified)

On a successful pass the sim executes this sequence at `dispatch.rs:1034-1043`:

1. Ball position snapped to passer feet (`pos_x = from_x`, `pos_y = from_y`).
2. Ball velocity set toward the receiver at `speed` m/s.
3. `state.possession = Some(to_slot)` — receiver owns the ball **on the same tick the ball launches**.

On the receiver's next decision tick (1-2 ticks later) the `Dribble` arm of `dispatch.rs:1242-1244` snaps `ball.pos = player.pos` and zeros velocity, terminating the ball's physical flight.

Result: a 20m pass at 17.5 m/s would require ~69 ticks of visible travel (ball_physics.rs TICKS_PER_SECOND = 60; 20m / 17.5 m/s / (1/60) = 69 ticks). Instead it travels for 1-2 ticks before being snapped. This produces the measured median 9.8m ball-carrier separation and the 52.7-unit single-tick ball jumps in the frame data.

The naive fix — set `possession = None` at launch and rely on the existing loose-ball pickup — fails because `lib.rs:2655` gates pickup on `ball_speed_sq < PICKUP_MAX_SPEED^2` (8 m/s). A 17.5 m/s pass stays above 8 m/s for approximately 150 ticks at current drag settings (combined k ≈ 0.007/tick → time to slow from 17.5 to 8 m/s ≈ (17.5-8)/(0.007×60) ≈ 22.6 seconds ≈ 1360 ticks; even at the looser 1/60s linear model it overshoots). Short passes would sail well past the receiver before becoming claimable.

---

## Goal-safety pre-mortem (the FUN-PHYS-1 lesson)

FUN-PHYS-1 was backed out twice because canonical ball/possession changes destabilized goal production. The mechanism: any new path that clears `possession = None` during what was previously a controlled-possession tick creates loose-ball windows where defenders can intercept or the ball sails out of play. More loose-ball ticks → fewer completed passing sequences → fewer shots → lower goal rate. Or the reverse: if the arriving receiver now takes 10 ticks to "trap" the ball rather than 1, defenders converge, and close-range shots decrease. Either direction can collapse the goal rate outside the [1.0, 5.0]/match sanity band.

**The key insight for goal-safety:** Slice 1 must change ONLY the visual representation of the flight, not the outcome determination. If the pass outcome (success/failure, possession assignment) is decided at launch using the existing `resolve_pass_completion` roll, and the ball merely animates along the predetermined lane over N ticks, then:

- Goal-rate is definitionally unchanged: the possession transition is the same, just delayed N ticks for the animation.
- The only risk is that N ticks of `possession = None` with the ball in flight creates windows for the existing loose-ball pickup (lib.rs:2655) to fire and steal the ball from the intended receiver. The design must prevent this.

---

## Model design

### 1. BallInFlight state — canonical field vs sidecar

**Decision: add `ball_in_flight: Option<BallInFlight>` as a canonical `MatchState` field.**

Rationale:

A non-canonical sidecar (behind `#[serde(skip)]`) would make the flight state invisible to the canonical encoder. Any logic that branches on flight state — the trap check, the pickup guard — would then produce behavior that diverges across replays constructed from different sidecar states. Since the canonical hash asserts identical behavior across platforms and replay reconstructions, flight state must be in canonical state.

This is an authorized canonical schema bump per ADR-0012 trigger #1. The BLAKE3 hash re-pins as part of the Slice 1 commit. The commit body documents the re-pin.

**BallInFlight definition:**

```
pub struct BallInFlight {
    /// The slot of the intended receiver.
    pub intended_receiver: PlayerSlot,
    /// Whether the outcome was resolved at launch (Slice 1: always true).
    /// When true, `outcome_is_success` determines whether the receiver or
    /// nobody gets possession on arrival.
    pub outcome_predetermined: bool,
    /// True = pass succeeded (resolve_pass_completion returned true at launch).
    /// False = pass failed (drop_loose_ball fires on arrival instead of trap).
    pub outcome_is_success: bool,
    /// Tick at which the pass was launched. Used for timeout guard.
    pub launch_tick: Tick,
}
```

All fields are Q32-free (booleans + Tick + PlayerSlot = integer types). No floats. No BTreeMap. Derives `Serialize + Deserialize + Debug + Clone + PartialEq + Eq`. Encoded by the canonical encoder as part of `MatchState` — appended in field order after the existing `last_touched_by` block per the field-order-append discipline.

**Why not a sidecar:** the trap-check guard (see §3) branches on `ball_in_flight.intended_receiver` to prevent the loose-ball pickup from firing. If this were a sidecar field, a replay constructed from canonical state alone would be missing the guard and would produce different outcomes. Canonical field eliminates the divergence.

### 2. Launch-roll-then-animate vs continuous-resolution

Two options:

**Option A — launch-roll-then-animate (RECOMMENDED):** The pass outcome (success/failure) is determined at launch via the existing `resolve_pass_completion` call. The result is stored in `BallInFlight.outcome_is_success`. The ball physically travels for N ticks. On arrival (receiver proximity), if `outcome_is_success`, trap fires and possession transfers. If not, `drop_loose_ball` fires at the receiver's feet instead of at the midpoint. No second roll during flight.

**Option B — continuous resolution:** The pass outcome is not predetermined. Each tick during flight, check whether a defender is within an intercept radius; if so, roll a stochastic intercept. On arrival with no intercept, the receiver traps unconditionally. This requires a per-tick random draw during flight (SeedLayer::ScoutObservation or a new layer) and changes the outcome distribution relative to the current `resolve_pass_completion` model.

**Recommendation: Option A for Slice 1, Option B as Slice 3.**

Option A preserves outcome distribution exactly — the pass completion roll, the lane gate with interception (`lane_gate_with_interception` in `pass_completion.rs:217-254`), the P_BASE/P_FLOOR constants, the receiver-pressure weight — all unchanged. Goal rate cannot drift because the probability of possession changing hands on any given pass is identical to the current sim. Only the timing of the visual possession transfer changes. This is the minimum viable fix for the teleport defect with zero goal-rate risk.

Option B (Slice 3) adds genuine in-flight interception chances, which will lower pass completion rates and could reduce goal rate. It must be developed after a goal-rate baseline is established with Slice 1 landed.

**Worked example (Option A, Slice 1):**

Player in slot 4 (home midfielder) passes to slot 9 (home forward). Ball launched at tick T.

- `resolve_pass_completion` rolls at tick T → returns `true`.
- `BallInFlight { intended_receiver: 9, outcome_predetermined: true, outcome_is_success: true, launch_tick: T }` written to canonical state.
- `possession` set to `None` (not `Some(9)` as before).
- Ball velocity set toward slot 9 at 17.5 m/s. Ball physically travels.
- Ticks T+1 through T+N: `possession = None`, ball in flight. Decision cadence for slot 9 fires but the dispatch arm must be idle when ball_in_flight is active for the intended receiver (see §5, guard detail).
- At tick T+K, `dist(ball.pos, player[9].pos) <= TRAP_RADIUS_M`: trap fires, `possession = Some(9)`, `ball_in_flight = None`, ball velocity zeroed.
- Observed flight: K ticks of visible ball travel. For a 20m pass at 17.5 m/s, K ≈ 69 ticks (~1.15 seconds).

**Worked example (outcome_is_success = false):**

Same setup but `resolve_pass_completion` returns `false`.

- `BallInFlight { intended_receiver: 9, outcome_is_success: false, launch_tick: T }`.
- Ball launches toward slot 9 position at reduced speed (failed pass = `drop_loose_ball` speed from `dispatch.rs:1051`).
- Ball travels until it reaches the receiver's current position (not the trap radius — the ball arrives but isn't trapped).
- On proximity, `drop_loose_ball` fires at that location with a deflection velocity (per defect #13 fix — small random deflection from `SeedLayer::BallPhysics`).
- `ball_in_flight = None`, `possession = None`, ball loose and claimable by settled-loose-ball pickup.

### 3. Trap mechanics

**Trap radius:** `TRAP_RADIUS_M = 1.5m` (Q32: `Q32::from_raw(1_i64 << 32 | (1_i64 << 31))` = approximately 1.5, or more precisely `Q32::from_int(3) / Q32::from_int(2)`).

**Why 1.5m and not the existing 5m PICKUP_RADIUS_M:** the trap is a deliberate receive, not a stumbling-upon of a settled ball. A trap radius of 1.5m means the receiver must be within realistic first-touch range. The existing 5m loose-ball pickup radius is generous precisely because a settled ball requires less coordination; a deliberate trap is tighter.

**Why no speed gate:** the 8 m/s PICKUP_MAX_SPEED gate on the existing loose-ball pickup exists to prevent a ball in full flight from being claimed by whoever is standing near it. The trap bypasses this gate because `ball_in_flight.intended_receiver` identifies the specific player who should trap it, and that player's proximity signals genuine first-touch contact regardless of ball speed.

**Trap check fires:** once per tick in `tick_match`, after ball physics integration, before the existing loose-ball pickup block. Only fires when `ball_in_flight.is_some()`.

```
// Pseudocode — exact implementation in dispatch.rs or a new fn in lib.rs
if let Some(ref bif) = state.ball_in_flight {
    let recv_idx = bif.intended_receiver as usize;
    let p = &state.players[recv_idx];
    let dx = p.pos_x - state.ball.pos_x;
    let dy = p.pos_y - state.ball.pos_y;
    let dist_sq = dx * dx + dy * dy;
    let trap_sq = TRAP_RADIUS_M * TRAP_RADIUS_M;
    if dist_sq <= trap_sq {
        // Ball arrived.
        if bif.outcome_is_success {
            state.possession = Some(bif.intended_receiver);
            state.last_touched_by = Some(bif.intended_receiver);
            state.ball.pos_x = p.pos_x;
            state.ball.pos_y = p.pos_y;
            state.ball.vel_x = Q32::ZERO;
            state.ball.vel_y = Q32::ZERO;
        } else {
            // Failed pass arrives: deflect loose.
            drop_loose_ball_at(state, state.ball.pos_x, state.ball.pos_y);
            state.possession = None;
        }
        state.ball_in_flight = None;
    }
}
```

**Loose-ball pickup guard:** the existing block at `lib.rs:2655` checks `state.possession.is_none()`. During flight, possession IS None, so without a guard the loose-ball pickup would fire on the first tick the ball slows below 8 m/s — which for a short pass could be before arrival. Add a guard: the loose-ball pickup block also checks `state.ball_in_flight.is_none()`. If a flight is active, skip the pickup entirely. Only the trap check handles possession during flight.

```
// lib.rs pickup block — add one guard
if !goal_fired_this_tick && state.possession.is_none() && state.ball_in_flight.is_none() && ball_speed_sq < pickup_speed_sq {
    // ... existing pickup logic unchanged
}
```

This is a single boolean check added to an existing branch condition — minimal blast radius.

**Timeout guard:** if the ball travels for more than `FLIGHT_TIMEOUT_TICKS = 120 ticks` (2 seconds) without the receiver trapping it (e.g. the receiver moved far away chasing a different target), force-clear the flight: if `outcome_is_success`, the ball becomes loose (receiver missed the trap — treat as failed pass, deflect); if already `outcome_is_success = false`, the ball is already going to deflect anyway. Use `state.tick - bif.launch_tick > Tick::from_raw(120)` as the timeout. This prevents a permanent flight state if the receiver runs away.

120 ticks = 2 seconds at 60 Hz. A 20m pass at 17.5 m/s arrives in ~69 ticks. A 30m pass at 17.5 m/s arrives in ~103 ticks. 120 ticks is a safe outer bound that covers even long passes with some margin for slow receivers.

### 4. Interception during flight (Slice 1 vs Slice 3)

**Slice 1:** no per-tick interception during flight. Interception outcome is already baked into `resolve_pass_completion` via `lane_gate_with_interception` (pass_completion.rs:217). The launched ball physically crosses the pitch; defenders can visually be near its path, but they do not stochastically claim it. The visual result is still correct: an intercepted pass (outcome_is_success = false) produces a deflected loose ball near the intended receiver's feet. The recipient isn't the interceptor per se, but the visual is: ball travels, defender near the lane, ball drops loose.

**Consistency with lane_gate_with_interception:** this is the critical point. The existing `lane_gate_with_interception` function already uses the nearest-lane defender's tackling, anticipation, and pace to modulate the lane gate. It does NOT add a second roll. Slice 1 keeps this single-roll model intact, which means the pass completion statistics are identical to the current sim. The only change is the timing of the possession transfer.

**Slice 3 (future):** remove the defender interception quality from the launch roll. Instead, keep only the passer quality and receiver pressure at launch. Add a per-tick intercept check during flight: for each outfield defender within `INTERCEPT_RADIUS_M = 3m` of the ball, roll `p_intercept = intercept_quality × (1 - lane_openness_at_ball_pos)` using `SeedLayer::ScoutObservation` (closest semantic fit; a new `SeedLayer::FlightIntercept` discriminant is the correct long-term addition, requiring a minor ADR-0009 amendment). On intercept: possession goes to the defender, `ball_in_flight = None`. This requires a full goal-rate calibration pass; it is explicitly out of scope for Slice 1 and Slice 2.

**Consistency guarantee for Slice 1:** because the pass outcome is decided at launch with the exact existing formula, the MatchEvent stream is also unchanged: `MatchEvent::Pass { completed: true/false }` is emitted at launch (tick T), not at arrival. This is the correct semantics — the pass attempt is an action at a tick, and the event records the passer's action. The goal-event chain (pass → possession → shot → goal) is downstream of possession, which now transfers at arrival instead of launch. Net effect on event ordering: zero for Slice 1 (the event is emitted before the flight begins, same as now).

### 5. Decision cadence during flight

When `ball_in_flight.is_some()` and `state.possession.is_none()`, the intended receiver's decision cadence fires but the dispatch arm must not produce a new intent that would conflict with the incoming ball. Two options:

**Option A — suppress the receiver's decision:** if `ball_in_flight.is_some()` and `slot == ball_in_flight.intended_receiver`, skip the dispatch for that slot (treat as a "waiting to receive" state, equivalent to HoldPosition intent).

**Option B — allow the receiver to decide, but guard the dribble snap:** the receiver may produce HoldBall/HoldPosition intent; the dispatch arm for Dribble/HoldBall must not snap the ball if `ball_in_flight.is_some()`.

**Recommendation: Option A** for Slice 1. It is the simplest and safest. The receiver effectively pauses for up to ~69 ticks while the ball travels. This is realistic: a player waiting to receive a 20m ball is not simultaneously making independent decisions. Option B risks the receiver producing a MakingRun intent and moving away from the arrival point, causing the timeout to fire — which would introduce an indirect goal-rate effect.

Implementation: in `dispatch_tick` (`dispatch.rs`), before intent resolution, add:

```
if let Some(ref bif) = state.ball_in_flight {
    if state.players[slot_idx].slot == bif.intended_receiver {
        // Receiver is waiting to trap — no dispatch this tick.
        return;
    }
}
```

All other players (attackers and defenders) continue their normal decision cadence during the flight. Defenders still move per their role state; attackers still run to positions. The only frozen slot is the intended receiver.

### 6. Viewer impact

The existing `MatchFrameDto` projection at `dto.rs:54-70` already carries `ball: BallFrameDto` (ball position per tick) and `possession: Option<u8>`. Once Slice 1 lands:

- `possession` is `null` during the flight ticks (ball_in_flight is active, possession is None).
- Ball position progresses from passer to receiver across ~69 ticks.
- The 2D board dot for the ball visually travels across the pitch.
- The board already uses `possession` to highlight the carrier dot; during flight the ball travels freely with no highlighted carrier, which is the correct football representation of a ball in transit.

No DTO changes are needed for Slice 1 to display correctly. The defect-report item #25 (per-tick frame return) is a prerequisite for the flight to be visible on the board at playback speed — if the live step returns only the terminal frame of each batch, a 69-tick flight compressed into one frame still looks like a teleport. Slice 1 should be paired with defect #25 (returning `Vec<MatchFrameDto>` from `step_live_match_inner`). The combination fixes both the sim cause and the viewer cause of the teleport defect.

A note on the `ball_in_flight` field in the DTO: the DTO does NOT need to expose `ball_in_flight` to the frontend. The frontend infers "ball in flight" from `possession === null` while the ball is moving; no additional field is needed. The intended-receiver slot is internal sim state.

---

## Staged implementation plan

### Slice 1 — visible flight, unchanged outcomes (LOWEST RISK)

**What changes:**
- Add `pub ball_in_flight: Option<BallInFlight>` to `MatchState` in `crates/fw-match-sim/src/lib.rs`. Add `BallInFlight` struct in same file or a new `crates/fw-match-sim/src/ball_in_flight.rs`.
- Update `CanonicalEncoder` in `canonical.rs` to encode `ball_in_flight`.
- In `dispatch.rs` (successful pass arm, lines 1034-1043): set `state.ball_in_flight = Some(BallInFlight { ... })` and `state.possession = None` instead of `state.possession = Some(to_slot)`. The ball velocity assignment is unchanged.
- In `dispatch.rs` (failed pass arm): set `state.ball_in_flight = Some(BallInFlight { outcome_is_success: false, ... })` and `state.possession = None`. The `drop_loose_ball` call moves from launch time to arrival time (handled in the trap-check block).
- Add `trap_check_in_flight` function in `lib.rs` (or `dispatch.rs`), called once per tick after `ball_step` and before the loose-ball pickup block.
- Add `state.ball_in_flight.is_none()` guard to the loose-ball pickup block at `lib.rs:2655`.
- Add receiver-suppression guard in `dispatch_tick`.
- Re-pin the BLAKE3 canonical hash (authorized by this spec).

**Files touched:**
- `crates/fw-match-sim/src/lib.rs` — `MatchState` struct, `tick_match` body (trap check, pickup guard)
- `crates/fw-match-sim/src/dispatch.rs` — pass launch arms (short pass ~line 1034, long pass, lay-off ~line 1198, cross), receiver-suppression guard, dispatch_tick
- `crates/fw-match-sim/src/canonical.rs` — encoder update
- `crates/fw-replay/tests/canonical_hash.rs` — re-pin

**Files NOT touched by Slice 1:**
- `pass_completion.rs` — outcome roll unchanged
- `ball_physics.rs` — physics unchanged
- `dto.rs` — DTO unchanged
- Frontend — no change (except Slice 1 should ship alongside #25 per-tick frame fix, but that is a separate change on the viewer side)

**Goal-safety assertion for Slice 1:**
The pass completion probability is computed by the identical formula with the identical constants. The possession transition table (who has the ball when) is identical in the long run — the only difference is a ~69-tick delay. For goal-rate purposes: a team that was previously completing 83% of passes still completes 83% of passes. The number of shots per match is unchanged. The only measurable difference is a 69-tick window per pass where `possession = None` and `ball_in_flight` is set. During that window the passer's team may not initiate a new pass (the intended receiver is suppressed and the passer no longer has the ball), but this was already the case — the passer gives the ball up at kick. No new loose-ball window is opened. Goal rate: UNCHANGED (no calibration needed, no re-sweep needed, re-pin only).

**Measurable outcome for `scripts/fidelity-measure.mjs`:**

| Metric | Before Slice 1 | After Slice 1 |
|---|---|---|
| Median ball-carrier distance | ~9.8m | ~0.2m (settled possession only; flight is None-possession) |
| Pass-flight visible ticks | ~1-2 | ~20-70 (distance-dependent) |
| Goals per match (sanity band) | [1.0, 5.0] — UNCHANGED | [1.0, 5.0] — identical by construction |
| Possession-None ticks (%) | low | higher (includes in-flight) — expected, not a bug |

The "ball-carrier distance" metric now measures distance only during `possession.is_some()` ticks. During flight ticks possession is `None`, so those ticks should be excluded from the distance calculation in the measurement script.

### Slice 2 — failed-pass deflection at arrival (defect #13, low risk)

Separable from Slice 1 but logically dependent on it (the arrival point is now well-defined). When `outcome_is_success = false` and the ball reaches the intended-receiver proximity, call `drop_loose_ball_at` with a small seeded deflection velocity (`SeedLayer::BallPhysics`, site = receiver slot). This replaces the current behavior (drop_loose_ball fires at the midpoint at launch). Net effect: failed passes now produce a loose ball near the receiver's feet rather than near the midpoint. This is more realistic and does not change the number of failed passes — only where the loose ball lands.

Goal-rate note: loose balls near the receiver's area give the receiver slightly better recovery chances. This could marginally increase effective pass completion. Worth measuring but unlikely to move goal rate measurably (failed passes are a minority of all passes; the recovery delta is small). Run a 100-match drama sweep before re-pinning.

### Slice 3 — genuine in-flight interception (post-baseline, moderate risk)

Requires a goal-rate baseline from Slice 1 + Slice 2 having been in main for at least one full drama sweep. Steps:

1. Remove defender interception quality from `lane_gate_with_interception` at launch.
2. Add per-tick intercept check: for each outfield defender within `INTERCEPT_RADIUS_M = 3m` of ball, roll `p_intercept` using a new `SeedLayer::FlightIntercept` discriminant (ADR-0009 amendment needed — 8 current discriminants, add one, re-pin seed layer table).
3. On successful intercept: defender takes possession, `ball_in_flight = None`.
4. Full drama sweep (100+ matches) to verify goal rate stays in [1.0, 5.0].
5. Adjust `INTERCEPT_RADIUS_M` and the intercept quality formula until the measured interception-per-pass rate matches the real-football target (~5-8% of passes intercepted, not deflected to ground).

Slice 3 is the feature that makes defenders feel like they can read a pass. It is separately gated from the teleport fix (Slice 1) specifically so the goal-rate risk is isolated and measurable.

---

## Tuning values (Phase-1, Slice 1 only)

These live in this design doc as Phase-1 tuning values. Do not put them in SPEC.

| Constant | Value | Q32 raw bits | Rationale |
|---|---|---|---|
| `TRAP_RADIUS_M` | 1.5 m | `Q32::from_int(3) / Q32::from_int(2)` | Deliberate first-touch range; tighter than 5m loose-ball pickup |
| `FLIGHT_TIMEOUT_TICKS` | 120 ticks | `Tick::from_raw(120)` | Covers a 30m pass at 17.5 m/s (≈103 ticks) plus margin |
| `INTERCEPT_RADIUS_M` (Slice 3) | 3.0 m | `Q32::from_raw(3_i64 << 32)` | Defender must be nearly in the lane to intercept |

**Worked examples for trap radius:**

Example A: short pass (8m, 17.5 m/s). Travel time ≈ 28 ticks. Player walks at 5 m/s; in 28 ticks moves 28/60 × 5 = 2.3m toward the ball. Distance at arrival ≈ 8 - 2.3 = 5.7m from launch point; player is ≈ 2.3m from where the ball arrives. Ball is within 1.5m of player if player's reaction takes them within 0.8m of the arrival point per tick — plausible for a player moving to meet the ball. If the receiver has moved away, the timeout fires at 120 ticks; the ball drops loose. This is realistic: a misdirected short pass that nobody meets goes loose.

Example B: medium pass (15m, 17.5 m/s). Travel time ≈ 51 ticks. Receiver at 15m from passer: receiver has 51 ticks to be within 1.5m of the arrival point. At 5 m/s player speed, the receiver can cover 51/60 × 5 = 4.25m during flight; if they were initially within 4.25m of the arrival point, they trap it. Realistic.

Example C: long pass (25m, 17.5 m/s). Travel time ≈ 86 ticks. More time for the receiver (and defenders) to converge. Within the 120-tick timeout.

**On the 1.5m choice vs 2.0m or 1.0m:**

At 1.0m, a player moving at 5 m/s can cover at most 1.0/5 × 60 = 12 ticks of adjustment per meter of error — very tight; short-pass traps would fail if the receiver is even slightly off target. At 2.0m, the trap fires too easily for a fast ball arriving at an angle; it would read more like the existing snap than a genuine first touch. 1.5m is the midpoint. If playtesting reveals traps failing too often (ball drops loose more than expected), increase to 1.8m. If traps feel too automatic, decrease to 1.2m.

---

## Determinism checklist

- `BallInFlight` fields: `PlayerSlot` (u8), `bool`, `bool`, `Tick` (i64 newtype). No floats. No BTreeMap. No clock reads.
- Trap check: pure function of `state.ball` (Q32), `state.players[idx]` (Q32 positions), `state.ball_in_flight` (integers). No RNG draw. Deterministic.
- Timeout check: `state.tick - bif.launch_tick` — pure subtraction of `Tick` values.
- Failed-pass arrival deflection (Slice 2): `SeedLayer::BallPhysics`, `seed_fn(match_seed, tick, SeedLayer::BallPhysics, receiver_slot as u64)`. One draw per failed-pass arrival. Deterministic across platforms.
- `assert!` (not `debug_assert!`) for `BallInFlight` invariants: `intended_receiver < 22`, `launch_tick <= state.tick`. Per Sim/RULES.md §11.

---

## Files and functions per slice (implementation handoff)

### Slice 1

| File | Function / location | Change |
|---|---|---|
| `crates/fw-match-sim/src/lib.rs` | `MatchState` struct | Add `pub ball_in_flight: Option<BallInFlight>` field |
| `crates/fw-match-sim/src/lib.rs` | New struct `BallInFlight` | Define above `MatchState` |
| `crates/fw-match-sim/src/lib.rs` | `MatchState::initial` | Initialize `ball_in_flight: None` |
| `crates/fw-match-sim/src/lib.rs` | `tick_match` body | Add `trap_check_in_flight` call after `ball_step`; add `ball_in_flight.is_none()` guard to pickup block at line ~2655 |
| `crates/fw-match-sim/src/lib.rs` | New fn `trap_check_in_flight` | Implements §3 trap logic |
| `crates/fw-match-sim/src/dispatch.rs` | Short pass success arm (~line 1034) | Replace `possession = Some(to_slot)` with `ball_in_flight = Some(...)`, `possession = None` |
| `crates/fw-match-sim/src/dispatch.rs` | Short pass failure arm (~line 1046) | Set `ball_in_flight = Some(...)` with `outcome_is_success: false`; defer drop_loose_ball to arrival |
| `crates/fw-match-sim/src/dispatch.rs` | Long pass, lay-off, cross arms | Same possession-transfer change as short pass |
| `crates/fw-match-sim/src/dispatch.rs` | `dispatch_tick` entry | Add receiver-suppression guard (§5) |
| `crates/fw-match-sim/src/canonical.rs` | `CanonicalEncoder` | Encode `ball_in_flight` (Option serialized as a presence byte + fields) |
| `crates/fw-replay/tests/canonical_hash.rs` | Pinned hash constant | Re-pin (authorized by this spec) |

### Slice 2 (separable)

| File | Function | Change |
|---|---|---|
| `crates/fw-match-sim/src/lib.rs` | `trap_check_in_flight` | On `outcome_is_success = false` arrival: call `drop_loose_ball_at` with seeded deflection velocity instead of zeroing |
| `crates/fw-match-sim/src/dispatch.rs` | Short/long/layoff/cross failure arms | Remove `drop_loose_ball` call at launch; it now fires in trap check |

### Slice 3 (separately gated, future)

| File | Function | Change |
|---|---|---|
| `crates/fw-match-sim/src/pass_completion.rs` | `lane_gate_with_interception` | Remove defender interception quality from launch roll; return plain `lane_gate` |
| `crates/fw-match-sim/src/lib.rs` | `tick_match` or new `intercept_check_in_flight` | Per-tick intercept roll for defenders near ball |
| `fw-core/src/lib.rs` (or decision_cadence.rs) | `SeedLayer` enum | Add `FlightIntercept` discriminant (ADR-0009 amendment) |

---

## Open questions for the owner

1. **Drama-sweep goal-rate target before Slice 2.** Slice 1 is goal-rate-neutral by construction. Before Slice 2 (deflection at arrival), agree a goal-rate band for the drama sweep: the current observed rate plus/minus what delta is acceptable. Without this, the Slice 2 gate has no quantitative pass/fail criterion.

2. **Receiver decision suppression.** Option A (suppress the receiver entirely during flight) means a player who just had a pass played to them cannot call for it, run onto it, or react to a change. In Slice 3 this may need to be relaxed so the receiver can move to meet the ball rather than standing still. The exact policy (suppress all, suppress only shot/pass intents, allow movement only) is a feel question that should be revisited after Slice 1 playtesting.

3. **Per-tick frame return (#25) dependency.** Slice 1 fixes the sim-side teleport. If defect #25 (single terminal frame per batch) is not fixed simultaneously, the flight will still appear as a teleport in the live viewer at normal/fast playback speeds. Recommend shipping Slice 1 and defect #25 in the same commit or consecutive commits before the next playtest.

---

## Slice 1 attempt (2026-06-06) — approach VALIDATED, one real blocker found

A first Slice 1 implementation was attempted (background agent, then a main-thread fix) and DISCARDED unshipped. What it proved and what it revealed:

**The visual approach WORKS.** With the ball travelling and possession transferring on arrival, measured at 16/8 seeds:
- ball-to-carrier distance dropped **9.4 → 0.4 units** (the detached-ball/teleport defect is genuinely fixable this way — the ball travels and then sits at the carrier's feet).
- offside% even fell to ~14% as a side effect.

**But goals collapsed 3.13 → 0.75/match** — below the sanity floor. Root cause, and it corrects this doc's premise:

> The design claimed goal-invariance "by construction" because the outcome is rolled at launch and only the transfer is delayed. That is INCOMPLETE. Setting `possession = None` during flight is NOT outcome-neutral: `team_shape::compute()` derives `is_defending = true` whenever possession is `None`, so the PASSING team drops into its DEFENSIVE shape for the ~16-tick flight of every pass. Build-up play is interrupted on every pass → attacks die → goal rate crashes. Possession state drives team shape + intent-arm selection, not just ball ownership.

A timeout-grant fix (a successful pass completes to the receiver even on flight-timeout) did NOT recover goals — because passes trap quickly and rarely time out; the collapse is the per-flight defensive-shape flip, not lost passes.

**The real fix for the next attempt:** during a successful pass's flight, the passing team must RETAIN its attacking shape and intents. `is_defending` (in `compute()`) and the attacking/defending intent-arm gating must treat "`ball_in_flight` aimed at my own teammate" as STILL-IN-POSSESSION, not as `None`-means-defending. Thread `ball_in_flight` into the possession-phase derivation so a pass in flight to a teammate keeps the team attacking. THEN launch-roll-then-animate becomes genuinely outcome-invariant and goals hold at ~3.13. This is the scoped, known next step — not another blind attempt.

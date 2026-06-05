# FUN-PHYS-1 — collision-aware movement: OWNER-DECISION options

> Status: DECISION DOC (2026-06-05). Exposed by FUN-CB1's loose-ball scrambles: the sim has no collision-aware movement (separation.rs is position-only; apply_vel_toward_target re-issues convergence velocity each decision tick → players clip through each other). A naive global velocity-damp fixes it but SUPPRESSES GOALS (M1 2.35→2.15). CB1 shipped a lateral-offset mitigation in drop_loose_ball. This doc lays out the options for the owner to choose a direction. **Recommendation: Option B (waypoint deflection), Option C as fallback.** See MASTER_PLAN row FUN-PHYS-1 + STATUS OWNER-DECISION.

# FUN-PHYS-1 — Collision-Aware Player Movement: Decision-Ready Options Analysis

## What the code actually does (grounded)

The tick loop in `/Users/vibelogic/dev/football/crates/fw-match-sim/src/lib.rs` runs steps in this order:

1. Ball physics (step 4)
2. Tactic-FSM heartbeat (step 5)
3. `dispatch_tick` (step 6) — each player whose decision cadence fires chooses a `PlayerIntent` and calls `apply_intent`, which calls `apply_vel_toward_target(dx, dy)` (`dispatch.rs:1746`). This sets `vel_x`/`vel_y` **unconditionally** to the unit-vector toward the target at `MAX_PLAYER_SPEED` (8 m/s).
4. Loose-ball pickup (step 7b)
5. Velocity integration: `pos += vel × dt` (step 7)
6. `apply_player_separation` (step 8) — position-only nudge, never touches velocity (`separation.rs:115`).

The root cause is the ordering of 3 and 6. Every 15 ticks, two preempt-chasers both receive `MoveToPosition { target: ball_pos }` from the nearest-2 policy (`dispatch.rs:1786`). They sprint toward the same point at 8 m/s. Between their decision ticks, `apply_player_separation` pushes their positions apart, but on the very next decision tick `apply_vel_toward_target` re-issues the full convergence velocity because the target is still the same ball point. The separation pass cannot win this race: velocity is re-issued every 15 ticks, and the position nudge is at most `half_overlap × direction`, which is smaller than the distance the velocity will cover in the next integration step. The result is sustained overlap for 40–60+ ticks.

`drop_loose_ball` (`dispatch.rs:1392`) mitigates the worst case by offsetting the ball drop point 0.4m laterally away from the nearest opponent, breaking the head-on approach vector. That is the CB1 partial fix. The measured outcome: the specific 150mm/62-tick clip-through seed became CORDIC-ringing-only after CB1. The root cause — that `apply_vel_toward_target` is oblivious to other players — remains.

The acceptance gate for FUN-PHYS-1: no two players clip through each other for more than 1 tick in a 5400-tick match across all 40 drama-sweep seeds, with M1 remaining in [2.3, 3.2].

---

## Option A — Per-pair avoidance force applied before the velocity cap (boids-style steering)

**How it works.** After `apply_vel_toward_target` computes the intended `(vel_x, vel_y)` for a player, but before writing it to state, scan that player's 21 potential overlap partners. For each partner within a repulsion radius (say 0.6m — 1.5 × `MIN_PLAYER_DISTANCE`), compute a repulsion vector from partner toward self, scaled by `(repulsion_radius - dist) / repulsion_radius`. Sum the repulsion vectors and add them to the intended velocity, then re-normalise to `MAX_PLAYER_SPEED`. Write the adjusted velocity.

This lives entirely in `apply_vel_toward_target` or as a post-processing step in `apply_intent` before the velocity write. The scan is per-player (21 checks at the decision tick), so cost is O(n) per deciding player per tick, bounded by the cadence (at most 2 players decide per tick in the balanced slot scheme).

**Determinism feasibility.** Fully feasible. The loop is `for j in 0..22 where j != i` in ascending order — same BTreeMap/Vec ordering discipline that `apply_player_separation` already uses. Q32 for all distances and vector arithmetic. `Q32::sqrt` via CORDIC (same path as `separation.rs:149`). No RNG, no clocks. The repulsion radius and weight constants are Q32 literals.

**Goal-rate risk.** This is the critical problem. The avoidance force is applied to all player pairs, not just non-attacking ones. A striker running onto a through-ball is converging toward a point where a defender is already positioned — this is exactly what avoidance would damp. The naive global damp measured a 0.2 goals drop (M1 2.35 → 2.15). A boids force is softer than a global damp — it deflects rather than zeroes velocity — but the direction of deflection for a striker closing on a packed box is directly away from goal, which is worse than zeroing it. The risk is approximately equal to or worse than the global damp because the perpendicular deflection still removes the inward closing component near defenders.

You could gate the repulsion: only apply it for players on the same team, or only for defenders. But teams routinely overlap with teammates too (two CBs covering each other, two strikers making runs into the same zone). Same-team-only repulsion would suppress those legitimate runs. The gating gets complicated quickly.

**Impact on FUN-TS1/TS2 shape + press.** Pressing is exactly two opponents converging on the same carrier position. If the avoidance radius is 0.6m and two pressers are both 1.2m from a carrier, they both deflect off each other and the press collapses. The coordinated press model (FUN-TS2) is already shipped and depends on players being able to converge on a common target zone. This approach directly fights the press system.

**Implementation cost.** Low-to-medium. The logic fits in ~40 lines of Q32 arithmetic inside `dispatch.rs`. The main cost is the calibration loop: finding a repulsion weight that avoids clip-through without damping attacks requires the drama-sweep measurement cycle.

**Believability payoff.** Modest. The visual effect at the 2D-dot level is players that push slightly off-course near opponents. Since FW is text-first and 2D presentation, this has negligible visual return relative to the implementation risk.

**Verdict.** High goal-rate risk. Directly conflicts with the press model. Not recommended as the primary fix.

---

## Option B — Ghost/waypoint movement: route around when another player is closer to the target

**How it works.** In the preempt-check nearest-2 chase path, before issuing `MoveToPosition { target: ball }`, check whether a same-team or opponent player is within `CLEARANCE_RADIUS` of the straight line between the chaser and the ball. If so, compute a lateral waypoint — offset the direct path by `WAYPOINT_OFFSET_M` perpendicular to the approach vector, signed by the blocker's side — and issue `MoveToPosition { target: waypoint }` instead of `MoveToPosition { target: ball }`. The waypoint is computed fresh each decision tick so it tracks dynamically.

The seam: `preempt_check` in `dispatch.rs:1771`. The straight-line obstruction check is a dot-product test: does the potential blocker's projection onto the approach vector fall between 0 and the distance to ball, and is its perpendicular offset less than `CLEARANCE_RADIUS`? Q32 arithmetic, no sqrt needed for the projection (only for the perpendicular distance, which can use the cordic sqrt).

**Determinism feasibility.** Feasible. The check iterates 22 slots in ascending order. Q32 dot-product and perpendicular-distance arithmetic. The waypoint direction (left/right of approach) needs a deterministic sign — use the blocker's Y coordinate relative to the approach midpoint (same convention as `drop_loose_ball`'s lateral offset). No RNG needed; this is a purely geometric deflection.

**Goal-rate risk.** This approach is targeted only at the loose-ball chase path (preempt nearest-2 policy, which fires only when `possession == None`). It does not touch the BT on-ball paths, the press path, or the goal-run paths. When possession is established, the BT takes over and waypoint logic is not active. The goal-rate suppression from the naive global damp came precisely because that damp was active during attacking convergence. This approach bypasses attacking scenarios structurally, not via a gate.

The residual risk: if the deflection makes the second chaser arrive later than they would have, they miss some loose-ball pickups, slightly reducing the number of transitions from loose → possession, which could subtly affect the game rhythm. This is much smaller in magnitude than the goal-damp effect because it affects only the seconds-long scramble window, not the entire attack phase.

**Impact on FUN-TS1/TS2 shape + press.** Minimal. This only activates on loose ball (possession == None). The press (FUN-TS2b) fires when possession is established and the pressing team targets the carrier — that code path does not go through preempt_check's nearest-2 arm. Shape tracking (FUN-TS1) targets zonal slots, not the ball — also unaffected.

**Implementation cost.** Medium. The waypoint computation is 25–35 lines of Q32 geometry inside `preempt_check`. The obstruction scan is another 15–20 lines. The main complexity is handling the edge cases: what if the obstruction is the nearest-2 player's teammate? (Use opponent-only obstruction check — teammates arriving on the same ball should not deflect each other since they will contest possession, not clip through.) What if there is no clean waypoint (multiple obstructions on both sides)? Fall through to direct chase, which is what happens today.

**Believability payoff.** Good. In the loose-ball scramble — the case that exposed FUN-PHYS-1 — players will curve around each other instead of driving through. This is exactly what football players do in scrambles: they adjust their angle of approach.

**Verdict.** Targeted, believable, low goal-rate risk. The recommended direction.

---

## Option C — Collision response: damp only the inward-closing velocity component when overlapping, gated on possession phase or distance-to-goal

**How it works.** In the separation pass (step 8), when two players are within `MIN_PLAYER_DISTANCE`, additionally project each player's velocity onto the line between them and zero or damp the component that is closing the gap (the inward component). Leave the tangential component (the part perpendicular to the pair axis) untouched. Apply the damp only under a gate: skip the damp if either player is within a goal-proximity radius (e.g. 18m of the opponent goal, i.e. in or near the box) or if possession is established and one player is the carrier.

The seam: `separation.rs:129`, inside `resolve_pair`. After computing `corr_x`/`corr_y`, additionally read both players' velocities and zero their inward-closing components.

**Determinism feasibility.** Feasible. The pair iteration is already in ascending (i, j) order. The projection is Q32 dot-product with no sqrt needed (you have `dist` already). The gate condition (possession, distance-to-goal) reads canonical state that is already available via `state.possession` and player positions. The only new CORDIC call needed is if the goal-proximity gate uses a proper distance check; it can be approximated with a raw `pos_x.abs() > threshold` comparison (Q32 bit comparison), avoiding the sqrt entirely.

**Goal-rate risk.** The gate is supposed to prevent exactly the scenario where the global damp killed goals. But the gate is tricky to calibrate correctly. The measured 0.2-goal drop from the global damp came from every overlapping pair being damped, including attackers in the box. If you exclude the box (18m gate), you protect striker runs in the final third — but the midfield convergence that sets up those runs still gets damped. A midfielder running into space to receive a pass who overlaps a defender midfield will have their velocity damped, which could slow the attack just enough to prevent a shot opportunity. The effect is likely smaller than the global damp (which damped all attack phases) but not provably zero without a drama-sweep run.

The gate also has a subtle correctness problem: when possession switches from None to Some mid-scramble, the gate flips. But the velocity damping in the separation pass happens after the possession update (step 7b pickup happens before step 8). So the gate correctly reflects the possession state for that tick. Still, the boundary condition needs a proptest invariant.

**Impact on FUN-TS1/TS2 shape + press.** The separation pass runs on all 231 pairs regardless of role. Two pressers converging on a carrier will have their closing velocity damped when they overlap. This is actually reasonable — two players occupying the same space and pressing from the same side is redundant football, so damping it is arguably correct. The press utility (FUN-TS2b) coordinates by assigning different press roles (`Closer`, `Cover`, `Block`) per `compute_press_from_parts`; these roles target different approach angles, so legitimate coordinated press players should not overlap in the first place. The gate still provides a safety valve for the in-box case.

**Implementation cost.** Low. The inward-component projection is 8–10 lines inside `resolve_pair`. Adding a `MatchState` reference to `resolve_pair` (currently only takes `state: &mut MatchState`) is already the pattern, so no signature change needed. The gate logic is 3–5 lines.

**Believability payoff.** Moderate. This is a physically honest response — it stops players from clipping through each other — but it applies during defensive/midfield convergence too. The visual result is two players who slow down as they approach each other, which is more like they are physically impeding each other. That is actually somewhat realistic: contact slows both players.

**Verdict.** Cleaner than the global damp but still carries goal-rate risk from midfield damping. The gate adds complexity. A reasonable secondary tool but not sufficient as the only fix.

---

## Option D — Keep the per-scenario mitigations (CB1's loose-ball offset), accept brief overlaps

**How it works.** Do nothing new. The CB1 lateral-offset in `drop_loose_ball` already broke the worst measured clip-through. Rely on the position-only separation pass to correct any residual overlaps within 1–2 ticks. Accept that at the 2D-dot presentation level, a 1-tick overlap of two 0.4m-diameter dots at 60 Hz is invisible.

**Determinism feasibility.** Already shipped. Fully deterministic.

**Goal-rate risk.** Zero — no new code.

**Impact on FUN-TS1/TS2.** None.

**Implementation cost.** Zero.

**Believability payoff.** Poor at the scenario level. The acceptance gate explicitly requires no two players to clip for more than 1 tick across 40 seeds. The CB1 offset fixes the loose-ball head-on case for the specific seed measured, but `separation.rs`'s own module doc acknowledges that a sufficiently adversarial equidistant drop can still cause multi-tick overlap. This approach does not pass the acceptance gate as written.

**Verdict.** Fails the stated acceptance gate. Only viable if the gate is relaxed. Football Manager players visually overlap all the time at 2D; the gate could be relaxed to "no sustained clip-through beyond 3 ticks" and this option becomes defensible for the EA window. That is a product decision, not a technical one.

---

## Option E — Hybrid: waypoint deflection for loose-ball chase + velocity-component damping only for same-team pairs (not recommended separately, but worth naming as a fallback)

This is Option B + a restricted form of Option C applied only to same-team player pairs (two defenders converging on a mark, two strikers both making the same run). Same-team clips are less believable than opponent clips and do not affect goal-scoring (a striker clipping through a teammate is not damping an attack). This hybrid has the lowest risk profile of any velocity-modifying approach but is more complex to implement. Mention it as the escalation path if Option B alone proves insufficient.

---

## Recommendation

**Primary: Option B — Waypoint deflection in the loose-ball chase path.**

This is the correct surgical fix. The FUN-PHYS-1 exposure site is the `preempt_check` nearest-2 loose-ball chase, which is also where the CB1 mitigation sits. Extending that seam with a lateral waypoint when an opponent occupies the direct approach path is:

- Targeted to the actual failure mode (two chasers, same target point)
- Structurally excluded from attacking convergence (the gate is possession == None, which is true only during the scramble)
- Compatible with the press model (FUN-TS2b fires only under possession)
- Deterministic with existing Q32 geometric primitives
- Around 50–60 lines of code
- Consistent with the CB1 philosophy (geometric deflection, not velocity physics)

The implementation seam: `dispatch.rs::preempt_check` (around line 1771). Before issuing `MoveToPosition { target: ball }` for the nearest-2 arm, scan opponent slots in ascending order for any player whose perpendicular distance to the (chaser → ball) segment is less than `CLEARANCE_RADIUS` (suggest 0.6m = `Q32::from_raw(2_576_980_377)`) and whose projection along the segment is between 0.5m and `dist_to_ball - 0.5m`. If found, compute a lateral waypoint: `ball_pos + perpendicular_unit × WAYPOINT_OFFSET_M`, where `perpendicular_unit` is the 90-degree rotation of the approach unit vector, signed by the blocker's Y offset from the approach line. `WAYPOINT_OFFSET_M = MIN_PLAYER_DISTANCE × 1.5 = 0.6m`. This ensures the two chasers approach from diverging angles and the separation pass holds them apart for the remaining approach.

**Fallback: if Option B alone does not pass all 40 seeds.** Add Option C's inward-component damping to `separation.rs::resolve_pair` with the in-box gate (skip when `|opponent_goal_x - player.pos_x| < 20m`). This is the velocity-modification escape hatch that is safe to apply if waypoint deflection leaves residual overlap in edge cases where both chasers are already adjacent when the ball drops.

**The honest assessment for the owner.** Football sims universally tolerate some player overlap. FM players visually clip. The question is whether brief overlaps matter at FW's 2D-dot presentation level (they probably do not) versus whether they matter for canonical correctness (they do, since two players at the same position producing identical CORDIC-noise displacements is a determinism surface). The acceptance gate as written ("no more than 1 tick") is strict and may require both the waypoint approach and the component-damping fallback. If the gate were relaxed to "no sustained clip beyond 3 ticks," Option D (status quo) probably passes on most seeds already, and you could defer FUN-PHYS-1 past the FUN-TS4 shape work with minimal risk — because once the press model assigns different approach angles by design, the two-chasers-same-point scenario becomes structurally less common anyway.

The recommendation is Option B as the primary investment now, with the understanding that it is low-risk enough to ship before FUN-TS4, and Option C as the fallback to add during the drama-sweep validation pass if any seeds still produce multi-tick clips.

---

## Concrete file:line seams for Option B (recommended)

- `/Users/vibelogic/dev/football/crates/fw-match-sim/src/dispatch.rs` — `preempt_check` function (around line 1771): add the waypoint computation block inside the nearest-2 outfield arm, after the distance scan and before the `Some(MoveToPosition { ... })` return.
- `/Users/vibelogic/dev/football/crates/fw-match-sim/src/separation.rs` — `MIN_PLAYER_DISTANCE` constant (line 76): reuse as the base for `CLEARANCE_RADIUS = MIN_PLAYER_DISTANCE × 1.5` in the waypoint computation.
- `/Users/vibelogic/dev/football/crates/fw-match-sim/src/dispatch.rs` — `apply_vel_toward_target` (line 1746): no change required; the waypoint target is substituted upstream so this function receives the deflected direction naturally.
- New proptest invariant in `/Users/vibelogic/dev/football/crates/fw-match-sim/tests/match_event_proptest.rs`: "for all seeds in the drama-sweep set, no two players have squared distance < MIN_PLAYER_DISTANCE_SQ for more than N consecutive ticks" — this is the acceptance-gate test.

The canonical hash will need rebaselining after this change (it modifies player positions via different velocities → different position integration results). That is ADR-0012 trigger #3 (documented sim-behaviour change), authorized per the FUN-PHYS-1 task row. Use the multi-pin discipline (envelope-verify before re-pin, leave the 60-tick smoke pin unchanged if possible, rebaseline only the 600-tick pin).
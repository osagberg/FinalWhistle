---
title: Match fidelity defect report — sim + viewer
date: 2026-06-06
status: DRAFT — code-audit workflow + main-thread measurement, pending owner review
provenance: 8-subsystem code-audit workflow (file:line-traced) cross-checked against direct per-tick frame measurement (seed 0xfeedface, 1500 ticks). Every owner-reported symptom confirmed from code AND data.
---

## Measured ground truth (seed 0xfeedface, 1500 ticks, current main)

Read directly from the per-tick frame data the sim emits (dump_frames), not from tests:

- **Ball detached from its carrier** — median **9.8 units (~11m)** between the ball and the player who has possession; **>5m apart on 71%** of possession-ticks (max 41 units). The ball does not sit at the dribbler feet.
- **Offside unenforced** — **42%** of ticks have >=1 attacker beyond the 2nd-last defender; **29%** have >=2; longest unbroken offside stretch **168 ticks (~2.8 min)**. Should be ~0%.
- **Teleport / possession thrash** — possession oscillates between nearby players; the ball snaps between their positions; largest single-tick ball move **52.7 units** vs max ball velocity 23.75 (= discrete repositioning, not travel). Restarts reset the ball to the centre spot (0,0) in one tick.
- **Ball leaves the pitch (x=-52, past the goal line) with no out-of-play handling** — it drifts out and gets re-centred; no throw-in / goal-kick / corner.
- **Ball velocity decoupled from movement** — reported ball speed ~16/tick while position moves ~0.3/tick.

These numbers are the empirical half; the code-traced root causes follow.

---

## Verification notes (main-thread, post-audit — READ before acting on the table)

The audit below is a strong, code-traced map but is **not gospel** — each finding must be verified against code before a fix is attempted. Two checks done so far:

- **#1 (teleport / ball detached from carrier): CONFIRMED.** `dispatch.rs:1026` sets `state.possession = Some(to_slot)` on the *same tick* the pass launches (lines 1019-1024 give the ball its travel velocity). The receiver owns the ball while it is still in flight — exactly why the ball measures a median ~10 units from its possessor. Fix is canonical and sits in the **FUN-PHYS-1 area that previously destabilized goal production**, so it is the highest-risk change in the list.
- **#2 (away strikers loiter behind the line): AUDIT MISDIAGNOSED.** The claimed `zonal_slot` "sign bug" is NOT a bug — the home/away mirror (`relative_x = +40` home / `-40` away, paired with the mirrored `line_x`) is intentional and correct; applying the audit fix would break home positioning and not fix away. The **real cause**: `zonal_slot` (team_shape.rs:393-448) positions every player purely from their own team shape (`line_x + formation_offset x scale`) with **no reference to the opponent last-defender line and no onside clamp**. Forwards routinely sit beyond the opposition line, and with offside unenforced (#4) they loiter there. Correct fix is an **opponent-relative onside clamp on forward targets** (canonical, effort M-L), not a sign flip.

Treat every other table row as "diagnosis plausible, verify against code before fixing."

### Fix-path note — #1 (the core teleport / detached ball)

- **Tick granularity is fine** (1 tick = 1/60s of ball physics, ball_physics.rs:101). A 20m pass *should* take ~69 ticks — very watchable. The teleport is NOT a granularity limit.
- **Naive fix is blocked:** setting possession=None on launch and relying on the existing loose-ball pickup fails because pickup only fires under 8 m/s (lib.rs:2651-2655); a 17.5 m/s pass travels ~150 ticks before slowing that far, so short passes overshoot the receiver. The instant possession-snap exists precisely to dodge this.
- **Correct fix:** a BallInFlight state with the intended receiver trapping the ball on proximity (any speed), defenders in the path intercepting. Canonical, moderate-large, and re-enters the FUN-PHYS-1 code backed out before for destabilizing goals. Must be gated on a goal-count regression check.

---

# Final Whistle — Match Fidelity Defect Report

## 1. Headline — the honest state of the match

The match is a **believable-looking shell, not yet real football.** The infrastructure is genuinely strong: a deterministic 90-minute tick loop, a real xG-driven shot model, a pass-completion contest, a goalkeeper FSM, zonal team shape, and per-tick position frames flowing to the 2D board. What is missing is the connective tissue that makes those parts behave like a game: **players hold static formation targets instead of playing off the ball, the offside rule is effectively switched off in open play, possession transfers instantly so the ball never visibly travels, and there are no restarts, fouls, cards, half-time, or laws of the game.** The shape moves, the ball moves, goals get scored — but nobody marks, nobody runs, nobody is ever flagged, and play never stops.

Your four reported symptoms all traced to confirmed root causes in code:

**1. "Offside is ignored — no flag, no enforcement as positioning."** Confirmed two-part cause. (a) The pass-launch offside check only fires when the receiver is within 20m of the opponent's goal line — an explicit zone gate at `dispatch.rs:1592-1605` (`OFFSIDE_ZONE_DEPTH = 20`). With both teams in MidBlock, forwards sit around 17m from centre and defenders around 13m, so the forward is past the line but well inside the 32.5m cutoff — the check returns early and never fires. (b) There is **no positional offside at all** — `is_offside_at_pass_launch` is reactive (it only runs the instant a pass is played); nothing constrains where a forward may stand between passes. So a striker can stand 5m beyond the last defender forever and never be flagged. There is also no visual flag and no offside line on the board.

**2. "Two opponent strikers loiter behind the defensive line (unmarked, clearly offside)."** Confirmed, and there are **two independent bugs** producing this. First, an away-team coordinate-sign bug: `zonal_slot` computes `relative_x = form_x - form_defender_x` with `form_defender_x = +30` for the away team (`team_shape.rs:418-421`). Away forwards sit at form_x = -10 (`FORMATION_4_3_3_POSITIONS` slots 19-21, confirmed), so relative_x = -40 always — pushing away forwards deep toward the home goal regardless of phase. The transform's sign convention assumes "forward is at greater x than defender," which holds for home but is inverted for away. Second, marking simply does not exist: `evaluate_transitions` (`role_states.rs:172-221`, confirmed) only handles carrier↔non-carrier routing; transitions to Tracking/Pressing are commented "deferred to T2+," so defenders never step to a runner and `utility_mark_player` only ever targets the carrier, never a specific striker.

**3. "Passes teleport — only ~20% of flight visible."** Confirmed, with **a sim cause and a viewer cause stacked on top of each other.** Sim cause (the dominant one): on a successful pass `state.possession = Some(to_slot)` is set on the *same tick* the ball is launched (`dispatch.rs:1026`, confirmed). The receiver is instantly the carrier; on their next decision they pick HoldBall/Dribble, which snaps the ball to their feet with velocity zeroed (`dispatch.rs:1207-1232`, confirmed). The ball's 17.5 m/s physical flight — ~69 ticks for a 20m pass — is aborted in 1-2 ticks. Viewer cause stacked on top: `step_live_match_inner` returns a single end-of-batch `MatchFrameDto` (`commands.rs:2207`, confirmed `frame:`, not a Vec), so even the 1-2 ticks of real flight are collapsed — at x3/fast speed only the terminal frame of each batch reaches the board.

**4. "A bunch of other things wrong."** Confirmed many. No restarts (throw-ins/corners/goal-kicks freeze the ball on the boundary and never reassign possession — `lib.rs:2459` clamps but never clears possession, confirmed). No fouls, no cards, no penalties. No half-time — `is_second_half` is hardcoded `false` at every kickoff site (`lib.rs:674`, `lib.rs:2404`, confirmed) and there is no tick-2700 branch, so both halves play in the same direction with no break. The keeper stands 7.5m off his line (`gk_goal_line_position` returns ±45 not ±52.5, confirmed via `formation_position`). Saves emit no event. Role-state machine is frozen — Pressing/MakingRun/Tracking are unreachable, so the coordinated press logic and all dynamic runs are dead code. On-ball passes target geometric points ("10m ahead," "the box centre"), not actual teammates.

**Confirmed-from-code vs inferred.** Everything in symptoms 1-4 above is confirmed by direct reads of the cited lines. Two items remain *inferred* and are flagged in the table: the `is_defending` race condition for away forwards (a plausible alternate path the audit could not rule out without a runtime trace), and the save-model *calibration* being inverted in absolute terms (the direction is correct in code; whether the magnitudes are wrong needs a goal-rate measurement, not a code read).

---

## 2. Prioritized defect table

Sorted by severity, then surfacing safe viewer-only quick wins ahead of delicate canonical work within each tier.

| # | Symptom | Root cause | File refs | Fix risk | Sev | Effort | Fix approach |
|---|---------|-----------|-----------|----------|-----|--------|--------------|
| 1 | Passes teleport; ~20% of flight visible | Possession transfers on the same tick the pass launches; receiver snaps ball to feet next tick, aborting the ~69-tick flight | `dispatch.rs:1018-1029`, `dispatch.rs:1207-1232`, `lib.rs:2694-2704` | **canonical** | **P0** | L | Decouple "ball arrives" from "possession transfer": add a BallInFlight state, set possession=None on launch, grant possession only when dist(ball,receiver) < pickup radius; preempt check must respect the flight flag |
| 2 | Away strikers loiter deep behind the line | `zonal_slot` sign bug: `relative_x = form_x - form_defender_x` = -40 for away FWDs, pushing them toward the home goal every tick | `team_shape.rs:418-421`, `team_shape.rs:433`, `subtree_library.rs:107` | **canonical** | **P0** | S | Make relative_x direction-correct per team: `if team_idx==0 { form_x - form_defender_x } else { form_defender_x - form_x }`, and clamp a *defending* FWD's target_x so it cannot go deeper than line_x ± margin |
| 3 | Throw-ins/corners/goal-kicks never taken; ball freezes on the boundary, last carrier keeps possession | OOB clamp zeros velocity + clamps position but never clears possession or reassigns a restart-taker | `lib.rs:2459`, `lib.rs:1570`, `lib.rs:1595` | **canonical** | **P0** | L | On OOB: clear possession, set restart type from `setpiece_kind_for`, snap ball to restart spot, assign possession to the designated taker, freeze others for a brief window. (Minimum viable interim fix: set `possession = None` so the dead carrier stops dispatching phantom passes from the boundary) |
| 4 | Offside never enforced in open play | 20m zone gate suppresses the check everywhere but the final third; the most common MidBlock scenario is silently exempt | `dispatch.rs:1584-1605`, `team_shape.rs:393-448`, `bt/off_ball.rs:243-278` | **canonical** | **P1** | M | Drop/lower `OFFSIDE_ZONE_DEPTH` to the halfway line (apply check anywhere past the defensive line). Do this *after* fix #2 stabilizes FWD positions, then monitor goal rate before adding a BT onside clamp |
| 5 | No marking — defenders never track a specific runner | Spatial role-state transitions (Defending→Tracking/Pressing) deferred to T2+; `evaluate_transitions` only routes carrier↔non-carrier; `utility_mark_player` only ever targets the carrier | `role_states.rs:147-221`, `bt/off_ball.rs:211-241`, `subtree_library.rs:215-265` | **canonical** | **P1** | XL | Add spatial transition predicates: opponent within press/track radius → Tracking/Pressing; wire Tracking intent to `utility_mark_player` with the marked opponent's position (per-tick opponent scan, no schema change) |
| 6 | No interception during pass flight | Pass completion is a single binary roll at kick time; no per-tick check of defenders in the lane | `pass_completion.rs:193-294`, `dispatch.rs:998-1044` | **canonical** | **P1** | L | After BallInFlight (#1) exists, per-tick check whether a defender is within intercept radius of the ball; stochastic intercept/deflect seeded via SeedLayer::Decision |
| 7 | No fouls; clean and reckless tackles identical | `resolve_tackles` explicitly "Fouls/cards NOT implemented"; no Foul variant in MatchEvent | `lib.rs:1666`, `fw-content/src/event.rs:118` | **canonical** | **P1** | L | Add `MatchEvent::Foul`; roll a foul probability in `resolve_tackles`; award FreeKickFor at the foul position (mirror `apply_offside`); add yellow/red accumulation |
| 8 | No penalties ever awarded | PenaltyFor/Against enum variants exist but are never emitted; no box-foul detection | `tactic_fsm.rs:70`, `lib.rs:1322` | **canonical** | **P1** | M | After fouls (#7): if foul position is inside the penalty area, award PenaltyFor; add a basic spot-kick 1v1 vs the save model |
| 9 | On-ball passes go to geometric points, not teammates | Pass targets hardcoded ("10m ahead", "±25m midfield", box-centre); spatial inputs not threaded into the BT | `bt/on_ball.rs:16-27`, `bt/on_ball.rs:565-645`, `dispatch.rs:1494-1541` | **canonical** | **P1** | XL | Thread teammate positions into the BT context; score passes over real open teammates using `pitch_control` (T1-4/T2-1 spatial milestone) |
| 10 | Keeper stands 7.5m off his line; easy diagonal goals | `gk_goal_line_position` delegates to `formation_position` → ±45, not the ±52.5 goal line | `goalkeeper_fsm.rs:190-193`, `subtree_library.rs:91,103` | **canonical** | **P1** | S | Use the real goal-line x (HOME/AWAY_GOAL_X), optionally 1-2m off-line; rebaseline |
| 11 | Tactic-state changes (HighPress/LowBlock/Counter) produce no visible shape change | `compute_press_from_parts` reads the non-canonical `SimPressLevel` sidecar, not the TacticState FSM; FSM state never reaches the shape | `tactic_fsm.rs:383-484`, `team_shape.rs`, `bt/off_ball.rs:138-210` | **canonical** | **P2** | M | Map TacticState → effective press level inside shape compute so the FSM drives line height |
| 12 | No half-time; both halves same direction; phase stuck on "First Half" | `is_second_half` hardcoded false at all kickoff sites; no tick-2700 branch; `compute_phase` always returns FirstHalf | `lib.rs:674,2404`, `snapshot.rs:133-146`, `event.rs:122-123` | **canonical** | **P2** | L | At tick 2700 emit HalfTime + second-half KickOff, reset possession, swap direction (negate pos_x). Add `MatchEvent::HalfTime`. (DTO-only half: `compute_phase` can return SecondHalf by tick threshold — see #20) |
| 13 | Failed pass: ball stops dead mid-pitch | `drop_loose_ball` sets position but never gives the ball a deflection velocity | `dispatch.rs:1407-1490` | **canonical** | **P2** | S | Assign a small random deflection velocity (seeded via SeedLayer::BallPhysics) so the ball rolls loose |
| 14 | Dribble: ball glued to feet, never rolls ahead | Dribble snaps ball to player pos with velocity zeroed every tick | `dispatch.rs:1207-1232` | **canonical** | **P2** | M | Tethered-ball model: ball = player pos + small forward offset with low velocity; physics rolls it naturally |
| 15 | No save event; goals appear with no visible keeper action | save_made branch returns silently; no `MatchEvent::Save` | `lib.rs:2334-2365`, `goalkeeper_fsm.rs:260-289` | **both** | **P2** | S→M | Add `MatchEvent::Save`; emit on save; render a save annotation on the board. (Viewer half can follow once the variant exists) |
| 16 | Keeper barely tracks the ball's angle (covers only 25% of offset) | `aim_factor ≈ 0.25` at baseline attributes leaves the far post open | `goalkeeper_fsm.rs:274-283` | **canonical** | **P2** | S | Raise base aim_factor to ≥0.6 (≈0.9 for a top keeper); recalibrate positional_factor; rebaseline |
| 17 | Offside line is a tactic-state step function, not the 2nd-rearmost defender | `is_offside_at_pass_launch` reads `team_shape.line_x` (FSM target), not actual defender positions | `dispatch.rs:1618-1637`, `team_shape.rs:239-251` | **canonical** | **P2** | S→M | Replace line_x with a per-tick O(11) scan for the 2nd-rearmost opponent x; do after defender shape (#5) is stable |
| 18 | No width variation; teams hold 35m span in and out of possession | `compactness_h` is a constant 35 (FUN-TS3 deferred) | `team_shape.rs:134,306` | **canonical** | **P2** | S | Widen to ~44m when `!is_defending`; one-liner; rebaseline |
| 19 | **Offside line not shown on the board** | `MatchFrameDto` carries no offside-line x; `drawPitchLines` draws only static markings | `dto.rs:54-70,98-120`, `TacticalBoard.tsx`, `pitch-coords.ts:87-178` | **viewer-only** | **P2** | S | Add `offsideLineHomeX/AwayX` to the DTO from `team_shape[*].line_x`; draw dashed vertical lines on the board |
| 20 | UI shows "First Half" at minute 80; half-time is a client tick-hack | `compute_phase` always returns FirstHalf | `snapshot.rs:133-146`, `LiveMatch.tsx:45-49`, `types.rs:102-107` | **viewer-only** | **P3** | S | Return SecondHalf when tick > 2700; HalfTime at the boundary. DTO-only, no canonical touch |
| 21 | Slow-mode ball travel still compressed | Viewer amplification of #1: 1 frame per 100ms batch | `LiveMatch.tsx:162-175,216`, `TacticalBoard.tsx:434-445` | **viewer-only** | **P1** | S | Resolves once #1 + #25 land; interim: increase ticks-per-call in slow modes |
| 22 | Offside event badged neutral grey; no flag flash | `badgeClass` maps Offside to grey; no event overlay on the board | `LiveMatch.tsx:308-332`, `Match.tsx:149-154`, `TacticalBoard.tsx` | **viewer-only** | **P3** | S | Flag-amber badge; flash the offending player's dot for ~1s using `offending_slot` |
| 23 | Stiff/mechanical player movement (4Hz decisions, no accel) | Players hold last velocity for ~15 ticks; velocity set instantly | `decision_cadence.rs:9`, `dispatch.rs:39` | **viewer-only** | **P3** | S | Sim is correct; viewer can smooth-interpolate player positions between frames |
| 24 | Sparse event feed; long quiet stretches | Key-moment filter shows only Goal/Shot/etc | `decision_cadence.rs`, `match-events.ts` | **viewer-only** | **P3** | S | Add PassIncomplete (turnover) to the key-moment list for momentum legibility |
| 25 | Live step sends only the terminal frame of each batch | `step_live_match_inner` returns a single `frame`, not per-tick frames | `commands.rs:2194-2210`, `LiveMatch.tsx:801-803` | **both** | **P1** | M | Return `Vec<MatchFrameDto>` (one per simulated tick); frontend appends all. DTO + frontend change, sim unchanged |
| 26 | Shots aimed at ±52, 0.5m short of the ±52.5 goal line | `target_x = Q32::from_int(52)` sentinel | `bt/on_ball.rs:512-516`, `fw-core/src/lib.rs:96-105` | **canonical** | **P3** | S | Use `GOAL_LINE_X`; rebaseline |
| 27 | Shot type / assist kind hardcoded to footed/solo | `shot_type_q32 = assist_kind_q32 = Q32::ONE` at all call sites; xG coefficients never vary | `bt/on_ball.rs:451-453`, `dispatch.rs:440-441`, `xg.rs:77-83` | **canonical** | **P3** | M | Derive shot type/assist from preceding MatchEvents (Cross→header, LayOff→assist); reconstructable, no new state; rebaseline |
| 28 | Pressure proxy = (1 − composure), not real defender proximity | Spatial pressure deferred; `shot_pressure_feature_q32` exists but only used for telemetry | `bt/on_ball.rs:16-27,429-431`, `dispatch.rs:265-297` | **canonical** | **P3** | L | Wire the existing spatial-pressure scan into the BT context (T2-1 spatial milestone); rebaseline |
| 29 | No build-up runs; forwards hold static zonal targets in possession | MakingRun never reached (#5); `utility_run_off_ball` targets the same static zonal_slot as hold_formation | `subtree_library.rs:276-345`, `bt/off_ball.rs:250` | **canonical** | **P2** | L | Once MakingRun is wired, give run_off_ball a ball-relative/space-behind-the-line target distinct from hold |
| 30 | Save-model magnitudes may be inverted in absolute terms | `(1-xg)` direction is correct, but constants were calibrated on proxy xG | `lib.rs:2256-2262,1063-1084` | **canonical** | **P2** | M | *Inferred, not confirmed.* Measure goal rates first; if close-range conversion is too low, recalibrate SAVE_BASE / restructure as `SAVE_BASE × positional × (1 − xg·w)` |

---

## 3. Recommended fix sequence

The ordering principle: **ship the safe viewer-only wins immediately (they cannot change the canonical hash), then take the canonical-sim fixes one at a time through the hash gate in dependency order, then build out the laws of the game as a dedicated track.** Note up front: FUN-PHYS-1 (ball physics / steering) was attempted and **backed out** because it destabilized goal production — every canonical change below that touches ball movement or positioning must be measured against goal rate before re-pinning.

### (a) Safe viewer-only quick wins — do first, batch freely

These live in `frontend/` or in the one-way DTO projection and **cannot** change match outcome or the canonical hash. They make the existing (broken) behavior *visible*, which is the precondition for trusting the canonical fixes afterward.

1. **#19 Offside-line overlay** — add `offsideLineHomeX/AwayX` to `MatchFrameDto` and draw dashed lines. This is the single highest-value viewer change: it makes the loitering/offside bug visible on screen, so you can *watch* the canonical fixes work.
2. **#25 Per-tick frame return** — return `Vec<MatchFrameDto>` from the live step. This is "both" risk only because it touches the Rust DTO and TS together, but it does not touch canonical state. It is the viewer half of the teleport fix and should land alongside the overlay so flight is visible once #1 lands.
3. **#20 Match-phase from server**, **#22 offside badge + flash**, **#23 player smoothing**, **#24 event-feed density** — small polish, batch together.

Why first: none can regress the hash, they unblock visual verification of everything else, and they directly improve the "believable shell" feel with zero sim risk.

### (b) Canonical-sim fidelity fixes — one at a time through the hash gate

Each of these changes match state and therefore the pinned canonical hash. Per the Sim rules and the FUN-PHYS-1 lesson, take them **strictly serially**: implement → run drama_sweep / goal-rate check → insta snapshot → proptest invariant → trace-claimed-factors-to-output → re-pin → commit. Ordering is dependency-driven:

1. **#2 Away-FWD zonal_slot sign bug (P0, S)** — do this *first*. It is small, self-contained, and it fixes the most visible "loitering" symptom. Critically, several later fixes (offside enforcement #4, offside line #17, marking #5) assume forwards sit in sane positions; doing #2 first establishes a correct baseline for them. Add a proptest: a defending team's FWD target_x must never be deeper than its own line_x.
2. **#10 Keeper on his line (P1, S)** — independent, small, clearly improves goal realism. Good early win to validate the rebaseline loop.
3. **#1 Decouple possession from ball arrival (P0, L)** — the teleport fix and the foundation for #6 (interception) and #14 (dribble). This is the big one: it introduces a BallInFlight state and changes when possession transfers. Measure goal rate carefully — this is exactly the class of change FUN-PHYS-1 destabilized. Pair the merge with viewer #25 so flight becomes visible.
4. **#13 deflection velocity on failed pass**, **#14 tethered dribble**, **#26 shot target = goal line** — small ball-movement honesty fixes, each behind its own gate, sequenced after #1 since they interact with ball state.
5. **#5 Marking / spatial role transitions (P1, XL)** — wire one transition first (nearest-opponent → Tracking, target the marked runner). This unlocks the dead Pressing/press-role code. Largest single fidelity gain; needs its own careful goal-rate sweep.
6. **#4 Remove the offside zone gate (P1, M)** — only *after* #2 and #5 so forwards hold legal positions and defenders hold their line; otherwise removing the gate will fire false offsides everywhere. Monitor goal production before/after.
7. **#17 2nd-rearmost offside line (P2)** — after #5, when defender shape is stable enough that a per-tick line is not jittery.
8. **#16 keeper angle**, **#18 possession width**, **#11 FSM→shape coupling**, **#29 build-up runs**, **#30 save calibration** — second-wave shape/keeper realism, each gated. #30 should be driven by a measured goal rate, not a code read.
9. **#27 shot type/assist**, **#28 real spatial pressure** — depend on the broader spatial-input milestone (threading positions into the BT context); fold into that work.

### (c) Larger systems — laws of the game (dedicated track)

These are net-new subsystems, not bug fixes, and should run as the existing Laws-of-the-Game track rather than inline:

1. **#3 Real restarts** (throw-in/corner/goal-kick: reposition ball + taker + brief freeze). The minimum-viable interim — clearing `possession = None` on OOB so the dead carrier stops dispatching phantom passes from the boundary — is small enough to slot into track (b) early and stops a real correctness bug now.
2. **#7 Fouls + cards**, then **#8 penalties** (penalties depend on foul detection landing first).
3. **#12 Half-time + second-half direction swap** (canonical) — its viewer-only half (#20) ships in track (a); the canonical kickoff/direction-swap belongs here.
4. **#15 Save event** (canonical variant + viewer annotation), **#6 in-flight interception** (depends on #1's BallInFlight), **#9 real pass targeting** (depends on the spatial-input milestone).

Ordering rationale: restarts before fouls (a foul awards a free-kick, which needs a working restart mechanic), fouls before penalties (a penalty is a foul-in-box), and everything law-related after the positional fixes in (b) so that, e.g., a free-kick is taken from a sane position with sane player shape around it.

---

## 4. Open questions — for the owner to watch or decide

1. **Goal-rate target before touching ball movement.** FUN-PHYS-1 was backed out for destabilizing goal production. Before #1 (possession decoupling), #13/#14 (ball movement), and #30 (save calibration), what is the acceptable goals-per-match band? Without an agreed number, the rebaseline trace-to-output step has no pass/fail criterion. **Owner decision needed.**
2. **Save-model calibration (#30) — confirmed direction, unconfirmed magnitude.** The code's `(1-xg)` term is directionally correct; whether a top keeper saving ~32% of a 0.55-xG chance is *wrong* is a calibration question that needs a measured conversion rate, not a code read. Owner should watch a few matches and judge whether close-range finishing feels too hard.
3. **Away-FWD loitering — second possible path (inferred).** Fix #2 (the sign bug) is confirmed. But the in-possession audit also flagged a *possible* `is_defending` race where away forwards get locked into the defensive zonal slot. The audit could not confirm this from code alone. Recommend adding the proposed proptest invariant (when team A has possession, `team_shape[A].is_defending` must be false) — if it ever fails, there is a second bug behind #2.
4. **Offside enforcement: BT clamp vs hard reactive flag.** After removing the zone gate (#4), do you want forwards to *self-regulate* (BT clamps run targets to stay onside, so flags are rare — realistic) or to genuinely get *flagged* (more visible, more punishing)? This is a feel decision that changes how the match reads.
5. **Scope of the laws-of-the-game track.** Full restart mechanics (walls, 10-yard enforcement, designated takers, corner deliveries) are a large build. Owner should decide the depth target: a believable abstraction (ball relocates, brief freeze, play resumes) versus full set-piece simulation. This sets the size of track (c).
6. **Skip-mode frame handling (#25).** At 300 ticks/batch (skip), returning every frame is a large IPC payload. Owner/eng should decide a downsampling policy (e.g. keep every Nth frame in skip mode) so the per-tick-frame change does not bloat skip-ahead.

---

## Campaign log (2026-06-06) — what was attempted, and the real blocker

Two onside-clamp attempts were made and both reverted (never pushed). The findings:

- **Attempt 1** (clamp the RunOffBall target to the opponent's line_x): measured NO-OP — offside% and goals unchanged from baseline. Wrong lever (forwards loiter via the HoldFormation/zonal path, not RunOffBall) and wrong line (line_x is the block centroid ~13m, not the real offside line). The agent reported a 2.85/45.3 improvement that did NOT reproduce on independent re-measure.
- **Attempt 2** (clamp inside zonal_slot to the TRUE 2nd-rearmost-defender x): the clamp fired (canonical hash changed, confirming it was live) but **goals collapsed 2.25 -> ~0.9 and offside got WORSE**, so it was disabled and reverted.

**The real blocker (key finding):** the team block does NOT translate up/down the pitch with possession phase. Defenders sit at their ~+30m formation baseline whether their team is attacking or defending. Consequences:

- The "offside line" (the 2nd-rearmost defender) is stuck at ~+30m, ~20m short of the goal line.
- Clamping forwards onside to that static line traps them ~20m from goal, so scoring collapses.
- Because the block never compresses or advances, a static defensive line and the attackers interleave in ways that read as constant offside (~49% of ticks).

**Implication for sequencing:** offside (both the positional clamp AND pass-launch enforcement) is BLOCKED behind a more fundamental fix — **possession-phase team translation** (the whole block pushes up when attacking and drops when defending; defenders advance with play). That foundation must land first. Likewise the detached-ball/teleport (#1) is blocked behind reworking the instant possession-transfer model. These are not independent pick-off bugs; they are 2-3 entangled STRUCTURAL gaps that must be fixed in order. Quick clamps on top of the current static geometry do not work (proven twice). This is planned match-engine work, not a quick-fix session.

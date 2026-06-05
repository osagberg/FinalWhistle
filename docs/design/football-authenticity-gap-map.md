# Football-authenticity gap map — measured current state + prioritised holes

**Status:** AUDIT-REFRESH (2026-06-05). Supersedes the per-domain verdicts in
`football-fidelity-audit.md` (2026-06-04) with (a) **measured** baseline numbers, (b) what has
**shipped since** that audit (defensive shape, realistic pass mix, shot model, passes-can-fail),
(c) the **drift-goals** discovery that reframed goal production, and (d) an **11-dimension** sweep
that closes the completeness gaps the prior 7-domain audit left open.

**Method:** 11 parallel dimension audits, each reading the actual sim code (`fw-match-sim`,
`fw-content`) and the roadmap, classifying every mechanism HAVE / PARTIAL / SPECCED-NOT-BUILT with
file/line evidence and a hidden-anomaly flag. **Standard:** real aggregate football (no licensed
data — the procedural-fantasy pillar holds). This is a steering document; it does not change the
contract.

---

## Measured baseline (drama_sweep @ HEAD `fcb3943f`)

Clean release build, no source edits. `drama_sweep` runs N full 5400-tick matches; seeds
deterministic (`base 0x1000000000000000`, `seed_i = base + i`).

| Metric | 50-seed | Guard band | Verdict |
|---|---|---|---|
| M1 goals/match (mean) | **2.66** | 2.3–3.2 | PASS |
| M1 goals/match (std) | 1.78 | 0.8–1.6 | FAIL — over-dispersed (per-seed range 0–6, 7/50 goalless) |
| M1 p95 | 6.0 | ≤7 | PASS |
| M8 shots/match (mean) | **10.7** | 9–18 | PASS |
| M8 on-target% | **39.1%** | ~30–55% (informational) | in range, not yet guarded |

Reproduce: `target/release/drama_sweep --seeds 50 --content content --summary-only` from repo root.

**The standing caveat the headline numbers hide.** `drama_sweep` counts `Goal` events but does NOT
tag shot-vs-drift provenance. At the realistic-football milestone (`2b40486`, 10 seeds) the goal
split was **17 SHOT / 7 DRIFT = ~71% shot-based, ~29% drift** — a "drift goal" being a ball that
crosses the line uncontested in open play and scores *unsaved* because the SS3 save model is gated
on `xg_score > 0`. So roughly **one goal in four on the current scoreline is not shot-based** — it
is a silently fabricated outcome. The 2.66 mean and 10.7 shots are honest as far as they go, but
~29% of the goals behind that mean have no shot cause. This is the single most important number on
the page and it is the one the sweep cannot see.

---

## What has shipped since the 2026-06-04 audit

The prior audit's Tier-B "already PLANNED" list has advanced. Confirmed shipped:

- **FUN-TS1 — held zonal defensive block** (line height + compactness). The back line now holds a
  coordinated zonal shape. DONE.
- **FUN-TS2 — coordinated pressing lines + offside emission.** `PressRole` (1 Primary + 2 Cover +
  HoldShape) routed in HighPress; the `Offside` event is now emitted (disc 6) and drives an IFK
  restart via `BallOutOfPlay`. DONE — **with a spec-vs-code caveat, see below.**
- **FUN-TS3b — realistic pass mix.** Pass-KIND utility reweighting: Short 77% / Long 10.4% /
  Cross 7.8% / LayOff 4.7% (floored), no goal-chain regression. SHIPPED (`f21516a1`).
- **FUN-TS3 (in flight) — shot model.** `compute_shot_dispersion_and_xg` + SS3 save model;
  on-target tuned into band. IN-PROGRESS (build-up geometry re-apply pending).
- **FUN-CB1 — passes can fail.** Retired the `T1_PASS_COMPLETED = true` stub; `PassIncomplete` +
  loose-ball drop now create the first real open-play turnovers. DONE — this is the **only**
  contested-ball mechanic shipped, and it is what newly populates the transition/loose-ball paths.

**Two drift findings reframed the roadmap since the audit:**

1. **Drift goals** (`goal-production-drift-goals.md`): ~29% of milestone goals bypass shots. FUN-TS4
   shot-volume calibration is correctly **PARKED** behind closing this — raising shot volume
   inflates drift goals first.
2. **FUN-TS2 spec-vs-code drift (FLAG TO OWNER).** The MASTER_PLAN FUN-TS2 row (line 438) claims
   "cover-shadow" and "an offside line that follows the last defender." The shipped code delivers
   neither: the Cover press-role just holds zonal (`subtree_library.rs:254-262`, no cover-shadow
   geometry) and offside is flagged at pass-launch against the **static tactic-target `line_x`**,
   explicitly NOT the actual rearmost-but-one defender (`dispatch.rs:1607-1619`). FUN-TS2 is marked
   DONE; the row over-claims. Reconcile the row text or open a follow-up.

---

## §1. The prioritised gap map

Ranked by **severity × hidden-anomaly-risk × how foundational the gap is to believability**.
"Hidden-anomaly risk" = the gap silently *fabricates or mis-awards an outcome* (a goal, a
possession change) with no football cause and no event trace — the drift-goals pattern. These rank
above honestly-absent gaps of equal severity, because an absent mechanic is visibly missing while a
fabricated outcome corrupts the scoreline invisibly and poisons every downstream metric (drama
sweep, conversion calibration, careers-that-remember ledger).

| Rank | Gap | Sev | Hidden anomaly | Foundational | Status |
|---|---|---|---|---|---|
| **1** | **Goalkeeping — off-line gathering, save model, angles, cross-claiming** | critical | YES | keystone | PARTIAL / specced (T19) |
| **2** | **Goal-mouth defending — clearances, balls rolling toward own goal, blocks** | high | YES | keystone | PARTIAL (twin of #1) |
| **3** | **Loose balls / rebounds / 50-50s / keeper parries / deflections** | high | YES | high | PARTIAL (FUN-CB2/CB3) |
| **4** | **Ball physics — the ball never leaves the ground (vel_z, spin)** | high | YES | high | PARTIAL |
| **5** | **Shot variety + height axis (over-the-bar impossible)** | high | YES | high | PARTIAL |
| **6** | **Positional play & role distinction (all 16 archetypes play 4-3-3)** | high | YES | high | PARTIAL (FUN-TI1/TI2) |
| **7** | **Defending mechanics — interceptions, marking, live offside trap, jockeying** | high | YES | medium | PARTIAL |
| **8** | **Game-state / tempo — score+clock never reach the FSM** | high | no | keystone | SPECCED (FUN-LM2/LM5) |
| **9** | **Transitions & phases — CounterAttack state is an inert label** | high | no | high | PARTIAL (FUN-TS4) |
| **10** | **Stamina / fatigue / subs / injuries — the 89th min == the 1st** | high | no | keystone | SPECCED (FUN-LM1/LM4/PI3) |
| **11** | **Restarts & set pieces — ~50 restarts/match mis-handled, 0% set-piece goals** | high | YES | medium | PARTIAL (FUN-LAW1) |
| **12** | **Fouls / cards / referee — match is foul-less and card-less** | high | no | medium | SPECCED-NOT-BUILT (FUN-LAW2/3) |

### Why this order (and confirming the in-flight drift-goals fix sits at the top)

**The drift-goals / goal-production fix (#1 + #2) is correctly the top priority — confirmed, not
just inherited.** The evidence makes the ranking unambiguous: a ball heading at an empty net is
never defended (no keeper off his line, no defender clearance) so it crosses and scores with
`xg_score == 0`, bypassing the entire save gate (`lib.rs:2030`). This is not a missing feature —
it is an active outcome-fabricator that already accounts for ~29% of goals and that **dominates**
(74% conversion, M1 6.05) the moment FUN-TS4 pushes players forward. It poisons every downstream
number: shot-volume calibration is meaningless while a quarter of goals aren't shots, the drama
sweep's M1 mean is inflated by phantom goals, and the careers-ledger records "goals" with no
believable provenance. Everything else competes for second.

**The four-way #1/#2/#3/#4 cluster is one problem with four faces:** the goal mouth and the
six-yard box have no contest physics. The keeper doesn't gather (#1), defenders don't clear (#2),
loose balls resolve as a geometric closest-wins race with zero attribute contest (#3), and the ball
can't leave the ground so there is no bounce/arc/deflection to make who-reaches-it vary (#4). Fix
the goal mouth and three of these improve together. This is why they sit above the genuinely-absent
dynamics gaps even though those are also "high."

**The dynamics cluster (#8/#9/#10) is high-severity but NOT hidden-anomaly.** Score+clock never
reaching the FSM, the inert CounterAttack label, and zero fatigue are real believability holes —
the 89th minute is mechanically identical to the 1st, and the drama sweep can only *measure* late
drama the engine cannot *cause*. But none of them fabricates an outcome: the failure mode is
"nothing fires," not "a phantom goal counts." An absent mechanic is recoverable and visible; a
fabricated goal silently corrupts the record. So they rank below the goal-mouth cluster despite
equal severity — exactly the discipline the drift-goals finding teaches.

**Restarts (#11) is a sleeper hidden-anomaly.** It is filed as "missing set-piece routines," but the
real defect is that restart possession is mis-awarded by a non-football proxy: the ball clamps to
the line, then nearest-body loose-ball pickup (home-first on tie) decides who restarts — instead of
the rule (ball to the team that did NOT touch it last). A "corner" can silently restart with the
team that conceded it. ~50 restarts/match are resolved this way with no event trace, and the real
~20%-of-goals set-piece channel is structurally absent. That is the drift-goals pattern at the
boundary, which is why it outranks fouls/cards (#12) despite both being FUN-LAW work.

**Positional play (#6) carries a subtle hidden-anomaly the prior audit under-weighted.** Even after
FUN-TI1 wires `archetype.formation`, every player still draws from the *same* on-ball/off-ball
utility set — a full-back and a striker differ only by their (x,y) slot anchor, not by behaviour.
So *who shoots and who assists* is an emergent artifact of which 4-3-3 slot a body occupies on the
flat-0.5 mirror substrate, not earned positional play. Once differentiated rosters land, this will
silently distort the scorer/assister distribution — the same "outcome from a static input rather
than a real cause" class. Note also that the prior audit's A6 evidence ("empty formation vecs") is
now **STALE**: `attacking-fullback.ron` *does* author distinct RB/LB positions; they are simply
unread by the sim.

---

## §2. Per-dimension evidence

Each row: the real-football behaviour, the sim today, and the load-bearing file/line citations.
Status `PARTIAL` = mechanism exists but is inert/stubbed; `SPECCED-NOT-BUILT` = honest absence with
an implementation-ready spec.

### 1 — Goalkeeping (PARTIAL, critical, hidden-anomaly)
The keeper is a pure 8-state FSM that emits a positional intent; the "save" is a DECOUPLED RNG roll
at goal-line detection, gated on `xg_score > 0`. **Off-line gathering is absent** — `SweeperKeeperRush`
only fires for a ball already in the keeper's own half, outside the box, with `vel_x` toward goal;
a slow ball drifting into the box does not pull the keeper out, and the rush is a velocity-target
with no physical interception. **Angle-narrowing is absent** — `gk_shot_stopping` returns
`target_x: goal_x`, glued to the line. **Cross-claiming is dead** — `GkCollectCross` is defined but
produced by nothing. A save emits NO `MatchEvent` (invisible to commentary/replay/momentum).
- `lib.rs:2030` (save gated on `xg_score > 0` — the drift path), `goalkeeper_fsm.rs:119-150`
  (rush conditions), `goalkeeper_fsm.rs:283-288` (`target_x: goal_x`), `role_states.rs:489`
  (`GkCollectCross` orphaned), `lib.rs:2048-2049` (no save event), `goal-production-drift-goals.md`,
  MASTER_PLAN T19.

### 2 — Goal-mouth defending (PARTIAL, high, hidden-anomaly)
Defenders do not clear a ball rolling toward their own goal in open play; there is no clearance
action at all. Combined with #1, an uncontested ball reaches the line and scores. This is the
defensive half of the ~29% drift share.
- `goal-production-drift-goals.md:18,41`, `event.rs` (no `Clearance`/`Block` variant),
  `goalkeeper_fsm.rs:96-188` (no clearance/catch/punch GK action).

### 3 — Loose balls (PARTIAL, high, hidden-anomaly)
Loose balls resolve as a deterministic geometric race: nearest outfielder within 5m, ball speed
<8 m/s, claims it; ties break home-first by slot order. NO attribute is consulted — no 50-50 roll,
no reaction stagger. GK saves are binary save-or-goal (ball snapped to keeper's feet); no parry,
spill, rebound. No in-flight block/deflection exists anywhere. Failed-pass balls drop *stationary*
(spec calls for scatter velocity; unbuilt).
- `lib.rs:2342-2402` (closest-wins, home-first), `lib.rs:2031-2061` (binary save, no parry),
  `dispatch.rs:1396-1480` (`drop_loose_ball` stationary + lateral-offset hack),
  `believability-specs.md:420-796` (FUN-CB2/CB3 specced), MASTER_PLAN:451-453.

### 4 — Ball physics (PARTIAL, high, hidden-anomaly)
The integrator is real and good (gravity, drag, Magnus cross-product, semi-implicit Euler, ground
bounce, rolling friction — all Q32). But **the ball never leaves the ground**: every kick hardcodes
`vel_z = Q32::ZERO` and spin stays zero, so the bounce branch, the gravity arc, and the entire
Magnus block are dead code in every real match. A cross is computed with the identical ground-skim
formula as a short pass. There is no FUN-PHYS track for the *ball* (FUN-PHYS-1 is player-player
collision). A Ferrari engine with no fuel line.
- `ball_physics.rs:174-305` (real integrator), `ball_physics.rs:124-126` (`magnus_coupling=0`),
  `dispatch.rs:974,1014,1069,1123,1177,1218,1239` (every kind sets `vel_z=0`),
  `dispatch.rs:498-526` (`ball_unit_vel` has no z), `shot-model.md:493-498` (over-the-bar deferred).

### 5 — Shot variety (PARTIAL, high, hidden-anomaly)
One footed ground-level shot type; a single target_y-only dispersion model; a fixed
strength×finishing power scalar; **no height axis (over-the-bar impossible)**; no
header/volley/one-on-one/tap-in/penalty/FK distinctions. Penalty/FK FSM states exist but are never
entered. Commentary narrates shot variety ("over the bar", "blocked") the engine doesn't compute —
a cosmetic leak that misrepresents the sim.
- `bt/on_ball.rs:457,508` (only `AttemptShot`), `dispatch.rs:974` (`vel_z=0`),
  `dispatch.rs:316-447` (single dispersion + power scalar), `shot.tracery.json:8-16` (decoupled
  miss copy), `shot-model.md` open-Q#2 (target_z deferred).

### 6 — Positional play & role distinction (PARTIAL, high, hidden-anomaly)
Four coarse roles (GK/DEF/MID/FWD). Every team plays one hardcoded 4-3-3 — `initial_with_content`
never reads `archetype.formation`. Within a role, behaviour is role-agnostic: all InPossession
states share the same 7 on-ball utilities, off-ball `utility_run_off_ball` returns to the static
zonal slot (the +10m forward advance was removed), so a "run in behind" is a slot return. No
overlaps, channel runs, decoy/space-creation, or depth-by-role.
- `role_states.rs:234-239` (4 roles), `lib.rs:728-789` (formation unread),
  `subtree_library.rs:89-114` (single 4-3-3 source), `bt/off_ball.rs:250-278` (slot-return run),
  `attacking-fullback.ron:23-40` (formation IS authored — audit A6 evidence now stale),
  `believability-specs.md:824-925` (FUN-TI1/TI2).

### 7 — Defending mechanics (PARTIAL, high, hidden-anomaly)
One real contested mechanic: `resolve_tackles` (2m radius, ~0.35 base prob, attribute-weighted),
emits no event, at most one per tick. Interceptions: no model (`predicate_through_ball_intercept`
defined but unwired). Marking: zonal-only (`utility_mark_player` dead in the defensive phase).
Offside trap: NOT a live step-up — flagged against the static tactic-target `line_x`, not the actual
last defender. Jockeying/cover-shadow: absent.
- `lib.rs:1417-1540` (tackles, no event), `reactive.rs:7-9,43,117` (intercept unwired),
  `subtree_library.rs:300-345` (zonal sole candidate), `dispatch.rs:1554-1628,1607-1619` (static
  offside line), `event.rs` (no Tackle/Interception/Clearance/Block/Foul).

### 8 — Game-state / tempo (SPECCED-NOT-BUILT, high)
Score and clock never reach the tactic FSM — a side 3-0 down at 85' selects the identical state as
0-0 at 5'. The heartbeat implements ONLY the HighPress 600-tick timeout; the call site passes only
`state.tick`. `is_second_half` is hardcoded false everywhere (dead canonical weight). No
stoppage/added time (`match_end_tick` fixed at 5400). The drama sweep's M4-M7 late-drama metrics
have no causal source.
- `tactic_fsm.rs:24-25` (explicit deferral), `tactic_fsm.rs:510-521` (only HighPress timeout),
  `lib.rs:2241-2252` (call site passes only tick), `lib.rs:655,2101` (`is_second_half=false`),
  `believability-specs.md:16-393` (FUN-LM1-5), MASTER_PLAN:445-449.

### 9 — Transitions & phases (PARTIAL, high)
Turnover detection and the `CounterAttack` FSM state already fire correctly in production — but
entering it changes nothing. The BTs never read `tactic_state` (the only in-possession reader is the
set-piece offside exemption). The CounterAttack shape parameters apply only when `is_defending`, but
a counter-attacking team is by definition in possession, so they're dead for the attacking side.
`CounterWindowClosed` is never emitted, so the counter never decays. No recovery-run disorganisation
on loss; no rest-defence retention. Broken and organised play are behaviourally identical.
- `tactic_fsm.rs:53,442-477,510-521`, `lib.rs:1195-1212` (crude `opponent_shape_broken` proxy),
  `lib.rs:1244-1318` (turnover detection fires), `dispatch.rs:1564` (only tactic_state reader),
  `team_shape.rs:277-289` (CounterAttack shape dead for attacker). FUN-TS4 is the wire-up row.

### 10 — Stamina / subs / injuries (SPECCED-NOT-BUILT, high)
None of it exists in-match. `match_fitness` is declared as a draining condition but has no writer
and no reader; movement is a hard 8.0 m/s cap for everyone every tick; `stamina` is a static utility
multiplier that never changes. No bench, no `subs_used`, no `Substitution` event. No half-time
(`is_second_half` never set true). No in-match injury roll despite `injury_proneness` being
canonical. The 89th minute plays identically to the 1st.
- `player_attributes.rs:937,944-949` (inert `match_fitness`), `dispatch.rs:109` (flat 8.0 m/s),
  `bt/off_ball.rs:147,179,266` (static stamina), `event.rs:117-251` (no Substitution/PlayerInjured),
  `believability-specs.md:22-103,264-389,1118-1187` (FUN-LM1/LM4/LM5/PI3).

### 11 — Restarts & set pieces (PARTIAL, high, hidden-anomaly)
All 11 restart classes exist as an enum and the FSM enters `SetPiece(kind)`, and OOB geometry is
correctly computed — but there is NO restart: the ball clamps to the boundary, NO possession
reassignment, NO ball-to-spot, NO taker, NO countdown, NO event. Nearest-body pickup decides who
restarts (mis-award). Every `SetPiece(_)` collapses to a neutral mid-block (no box-crashing, no
wall, no penalty arrangement). The dead `SetPieceWaiting`/`PenaltyStance`/`SetPieceWall` states are
never assigned. The only working restart is the offside IFK. 0% of goals route through set pieces
vs ~20% real.
- `tactic_fsm.rs:59-74` (enum), `lib.rs:1072-1144` (geometry computed), `lib.rs:2156-2215`
  (clamp, no reassignment), `team_shape.rs:208-224` (collapses to mid-block),
  `role_states.rs:285-411` (dead states), MASTER_PLAN:441-444 (FUN-LAW1-4).

### 12 — Fouls / cards / referee (SPECCED-NOT-BUILT, high)
Almost entirely absent — the match emits 8 event discriminants, none a Foul/Card/FreeKick/Penalty.
The tackle failure branch just sets a cooldown (consequence-free; no contact test, no whistle).
Zero advantage logic, zero referee actor/strictness, zero booking code. `penalty_taking`/`free_kicks`
are canonical but consumed by nothing. Honestly absent rather than silently broken — a match today
is foul-less and card-less. The spec (laws-of-the-game.md) is strong and code-verified.
- `event.rs:118-296` (8 variants), `lib.rs:1532-1536` (consequence-free tackle fail),
  `laws-of-the-game.md §3/§5/§6`, MASTER_PLAN:441-444 (FUN-LAW1-4, gated on FUN-TS4).

---

## §3. Completeness critic — dimensions the prior 7-domain audit did NOT cover

The 2026-06-04 audit was organised by *attribute/decision domain* (defensive phase, attacking,
on-ball skills, attributes, psychology, match-state, set-pieces). This refresh adds the
*event-and-physics* dimensions it folded in as one-liners or omitted:

- **Goalkeeping as its own dimension** — the prior audit treated the GK only inside "defensive
  phase." It is the #1 gap and deserved its own audit; the drift-goals finding proves it.
- **Ball physics / aerial trajectory / spin** — the prior audit mentioned `heading`/`jumping` as
  dead attributes (A10) but never audited the *ball*: that it never leaves the ground is a
  foundational physics gap, not an attribute gap.
- **Shot variety / shot-type taxonomy** — folded into "only shooting is well-modelled"; the
  no-height-axis hidden anomaly was not surfaced.
- **Loose balls / rebounds / deflections / second balls** — not a dimension in the prior audit;
  it is the loose-ball twin of drift-goals.
- **Transitions & phases of play** (broken vs organised, counter decay, rest-defence) — touched as
  "rest-defense" inside A11; the inert-CounterAttack-FSM finding is new.
- **Restarts as mis-awarded possession** — the prior audit filed set-pieces as "post-EA routines";
  the silent restart mis-award is a new and sharper framing.

**Dimensions still NOT covered by EITHER audit (flagged for completeness, lower priority):**
throw-in routine and long-throw specialists; goal celebrations / restart-after-goal timing as
narrative beats; weather/pitch-condition effects; crowd/home advantage as a measurable coefficient
(prior audit noted "no home advantage" in passing, never a dimension); VAR / goal-line technology
(out of scope for a fantasy manager, note and drop); ball-retrieval / multi-ball / time between
phases. None of these is believability-critical for EA; record them in the deferred register.

---

## §4. Recommended next 3–4 slices

The discipline from the drift-goals finding is decisive: **close the hidden-anomaly outcome-fabricators
before building any new outcome-producer.** Calibrating shot volume, fatigue, or set-piece chances on
top of a goal stream that is ~29% phantom is fitting models to corrupted data.

1. **T19 — Goal-mouth defending (drift-goals fix). KEYSTONE, do first.** Build the recommended
   (a)+(b) from `goal-production-drift-goals.md`: GK off-line gathering (broaden the FSM so the
   keeper proactively comes off his line for any loose/slow ball heading into his box, and make the
   rush physically gather/clear via a real interception radius) + nearest-defender clearance of a
   ball rolling toward his own goal. Result: uncontested balls never reach the line; goals are
   shot-based (SS3) plus legitimate deflections/own-goals/dribbled-in. This unblocks FUN-TS4 and
   removes ~29% phantom goals from every metric. Pairs naturally with adding a `Save` MatchEvent
   (saves are currently invisible) and a keeper parry/spill branch (closes the loose-ball rebound
   gap #3 at the same seam).

2. **FUN-PHYS-BALL — give the ball a z-axis (new row). HIGH, do early.** Add a `vel_z`/spin launch
   primitive and wire it into Cross, lofted AttemptPassLong, shots-over-bar, and GK distribution so
   the existing bounce/gravity/Magnus integrator goes live, plus a `target_z` height dispersion on
   shots (the 0x0004-0x0006 site bytes are already reserved in shot-model.md). This closes the
   over-the-bar / cross-too-high anomaly (#4/#5) and is the precondition for FUN-CB3 aerial duels to
   read as real — today CB3 would roll an aerial contest on a ball physically on the floor. Do this
   *before* further on-target/conversion calibration, or the band-tuning fits a 2D model to 3D
   football.

3. **FUN-LM cluster — make the match live (FUN-LM5 half-time → LM2 score+clock→FSM → LM1 fatigue).
   HIGH, keystone for dynamics.** Sequence per believability-specs §cross-cutting to minimise
   rebaselines. FUN-LM2 (thread `score_lead` + `ticks_remaining` into the heartbeat) is the causal
   engine the drama sweep's M4-M7 metrics currently only *measure*; FUN-LM1 fatigue (wire the inert
   `match_fitness` into a per-tick Q32 drain, and fatigue-scale `MAX_PLAYER_SPEED` so tired legs
   actually run slower) gives "late games open up" a physical cause. Calibrate the fatigue drain to
   ~15-25% (the seed ~5% is ~5x too light) before pinning.

4. **FUN-TI1 — formation wiring (near-pure data-wiring). HIGH, cheap, high-value.** Add
   `apply_archetype_formation` in `initial_with_content` to read the already-authored
   `archetype.formation`. This alone makes back-three / false-9 / attacking-fullback place
   differently — an authorized multi-pin rebaseline. Flag in the gap register that this delivers
   positional *placement* but not role-differentiated *behaviour* (#6's deeper half: a full-back and
   a striker still share one utility set); schedule FUN-TI2 (role-conditioned off-ball runs +
   on-ball weighting) as the follow-up that makes positions behave distinctly.

**Sequencing note for the owner:** FUN-LAW (fouls/cards/penalties, #12) and FUN-CB2/CB3
(dribble-1v1, aerial duels, #3 tail) are correctly gated behind FUN-TS4 / a believable block — a
foul model or aerial-duel model run against today's still-evolving shape would cluster
unrealistically. They are honestly absent, not silently broken, so they do not threaten outcome
integrity the way slices 1-2 do. Hold them after the goal-mouth + ball-physics + living-match work.

---

## Cross-references
- `docs/design/goal-production-drift-goals.md` — the #1/#2 finding and its fix options (T19).
- `docs/design/football-fidelity-audit.md` — the prior 7-domain audit this refreshes.
- `docs/design/believability-specs.md` — FUN-LM/CB/TI/PI implementation-ready specs.
- `docs/design/{shot-model,laws-of-the-game,match-realism-reference,drama-model}.md` — domain specs.
- `docs/MASTER_PLAN.md` Tier F (FUN-TS/LM/CB/TI/PI/LAW/DR) + T19 — the planned work cross-checked here.
- `docs/DESIGN_DOC.md` Pillar 0 (believable football) — the standard this audits against.

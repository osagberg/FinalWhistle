---
title: Attribute-effect audit + non-linear/21-ceiling/specialization design
date: 2026-06-06
status: DRAFT — attribute-effect-audit workflow (6 code-traced group audits + opus synthesis), pending owner review
provenance: read-only audit of all 55 attributes in crates/fw-match-sim + fw-core; steers DECISIONS 2026-06-06 attribute-effect philosophy. Verify any specific wiring claim against code before acting.
---

# Attribute-Effect Design — From Inert Stats to a Living Match

> Phase steering doc for the attribute-effect mandate (DECISIONS 2026-06-06, "Attribute-effect philosophy"). Source: per-group attribute audits of `crates/fw-match-sim` + `crates/fw-core/src/player_attributes.rs`. Build-out calibration policy applies throughout: **track goals, don't gate**.

---

## 1. Executive summary

The match sim reads **55 player attributes** across six families (technical, mental, physical, goalkeeper, personality, durability). Today they fall out roughly thirds:

| Status | Count | Meaning |
|---|---|---|
| **Wired** | 17 | Has a real, multi-tick, combinatorial effect on movement and/or outcome rolls. |
| **Partial** | 19 | Read somewhere, but the effect is a single-tick softmax nudge, a tiny additive tilt (±10%), a proxy double-use, or a static value with no fatigue/contest backing. |
| **Inert** | 19 | Serialised into canonical state and scaled in fixtures, but **never read by any match logic.** Zero behavioural consequence. |

**The headline gap is not the inert count — it is the shape of the wired ones.** Almost every "wired" attribute is wired as a **decision multiplier** (it changes which intent wins the per-tick softmax auction) and only a handful change **physics** (`pace`→`v_max`, `strength`/`finishing`→shot speed, `passing`/`vision`→pass ball speed). The match is **static-per-input**: there is no fatigue accumulator, no aerial-duel system, no dead-ball / set-piece restart, no foul/injury system, no first-touch control window, and no turning-cost model. As a result:

- **Five attributes are doing almost all the differentiating work** because they touch physics or the xG gate. The other ~50 mostly re-weight a softmax whose primary terms (products spanning 0→1) swamp the secondary tilts.
- **Whole subsystems are missing, not just wires.** The 5 set-piece technicals (`free_kicks`, `penalty_taking`, `corners`, `long_throws`, plus `heading` aerially) and the 3 durability attributes (`injury_proneness`, `recovery_rate`, `dirtiness`) cannot be "wired" — there is nothing to wire them into. They need their host systems built first (dead-ball restarts, aerial contest, fatigue, fouls/cards/injuries).
- **The top end is flat.** Every formula is linear in attribute value, so 19→20 buys the same delta as 11→12. Elite is incremental, not visibly elite. There is no representation for a value above 20.
- **Several "proxy" wirings hide the lack of a real attribute.** `tackling` doubles as interception (`off_ball.rs:388` comment), `command_of_area` is merged with `one_on_ones` into one rush factor, `concentration`/`teamwork`/`positioning`'s hold-formation paths are bypassed entirely when `shape.is_defending` (`subtree_library.rs:343`) so they only fire in possession.

This doc gives (§2) the master audit matrix, (§3) a deterministic non-linear effect curve so elite skews results, (§4) the "21" super-ceiling tied to the signature system, (§5) the specialization-visibility model, and (§6) a prioritized, goal-tracked wiring plan that front-loads the movement-shaping attributes per the dynamic-positioning work already underway.

---

## 2. The attribute-effect matrix

Status legend: **W** wired · **P** partial · **I** inert. "One-tick?" = whether the *current* effect is confined to a single decision/outcome tick (true) or already shapes multiple ticks (false).

### Technical (15 — W7 / P1 / I7)

| Attribute | Status | Current effect (file refs) | One-tick? | Intended effect | Combines with |
|---|---|---|---|---|---|
| finishing | **W** | Primary in shoot utility `shooter_quality = finishing·0.55 + composure·0.25 + technique·0.20` (`on_ball.rs:435`) feeding the xG gate; shot ball speed `base + bonus·(strength·finishing)` (`dispatch.rs:473`); shot sigma (`dispatch.rs:371`); gates PoachersDart (`triggers.rs:427`). Explicitly excluded from track_back/hold/defensive utilities. | yes | **outcome + movement.** Add: a clinical finisher makes smarter box runs (off-ball run-quality toward goal) → multi-tick shape. Non-linear top end visibly lifts close-range conversion under pressure. | composure, technique, strength, vision, balance |
| long_shots | **P** | Doccomment claims "primary" but formula uses it as a post-gate additive `(1 + 1.3·long_shots)` (`on_ball.rs:497`) — only amplifies a shot that *already* passed xG; never fires a shot from distance. Also `(1 + 0.20·long_shots)` in pass_long (`on_ball.rs:603`), long-pass completion weight 0.20 (`pass_completion.rs:461`), gates LongRangeStrike. | yes | **decision + outcome.** Should gate the xG distance feature directly (low long_shots ⇒ negligible xG >25m); should feed shot ball speed; multi-tick: high long_shots player is *attracted* to range, shaping where they advance. | finishing, vision, passing |
| passing | **W** | Multiplicative primary in pass_short/pass_long utilities (`on_ball.rs:541,596`); passer-quality composites (`pass_completion.rs:454,461`); pass ball speed `base + bonus·(passing·vision)` (`dispatch.rs:485`); GK short distribution (`goalkeeper_fsm.rs:316`); gates LegacySwitch. | no | Well-wired. **Gap:** target *selection* uses a fixed formation offset, not passing-weighted receiver quality. Elite passing should skew *which* receiver gets the ball. No 21-tier signature yet. | first_touch, technique, vision, decisions, long_shots |
| crossing | **W** | Primary in cross utility `crossing·vision·pace` (`on_ball.rs:664`); cross completion `crossing·0.50 + technique·0.20 + vision·0.30` (`pass_completion.rs:466`); gates OverlappingSurge, TouchlineBeat. | no | **Gap:** fixed cross target_x (`on_ball.rs:720`) regardless of attacker runs. Elite crossing should modulate delivery dispersion (like shot sigma) and target the actual runner; non-linear top end absent. | vision, pace, first_touch, anticipation, technique, dribbling |
| first_touch | **W** | Primary in pass_short / lay_off utilities (`on_ball.rs:541,801`); secondary in cross; short/layoff completion weight 0.30 (`pass_completion.rs:454`). | no | **Gap:** no effect on *time to control* a received ball — a poor first touch should add a control window (briefly reduced speed + raised tackle vulnerability). Needs a multi-tick state flag on ball receipt. | passing, technique, vision |
| technique | **W** | Primary in pass_short + dribble utilities (`on_ball.rs:541,737`); additive 0.20 in shooter_quality (`on_ball.rs:437`, `dispatch.rs:324`); completion weights. | no | **Gap:** no distinguishing effect from passing/dribbling at the top end (weak-foot execution, unusual trajectories). Currently only a small flat additive. | passing, first_touch, vision, dribbling, agility, finishing, composure |
| dribbling | **W** | Primary in dribble utility `dribbling·technique·agility·acceleration` (`on_ball.rs:736`); carry-resistance side of tackle contest `dribbling·0.50 + balance·0.30 + composure·0.20` (`lib.rs:1905`); gates TouchlineBeat. | no | **Gap:** directionless (back-dribble == goalward); linear tackle prob. A 21 dribbler should have non-linear superiority over the defender. | technique, agility, acceleration, balance, composure |
| heading | **I** | Serialised (`canonical.rs:657`) + scaled in drama_sweep; **never read in sim logic.** No aerial-duel system; `shot_type` hard-coded footed at all call sites. | n/a | **NEW SYSTEM.** Aerial contest when ball_z > threshold: `jumping_reach·heading` of contesters; winner gets a headed shot (header shot-type) or clearance/pass. | strength, jumping_reach |
| tackling | **W** | Primary in tackle outcome `tackling·0.50 + aggression·0.30 + positioning·0.20` (`lib.rs:1957`); lane-cover weight 0.30 (`off_ball.rs:347`); interception quality `tackling·0.60 + anticipation·0.25 + pace·0.15` (`pass_completion.rs:234`); gates ScreeningInterception. | no | Well-wired but **double-duty** as interception (proxy per `off_ball.rs:388`). Split out interception (T4.5-E1); make tackle a pure physical-challenge outcome; elite tacklers get a slightly larger effective radius (fixed `TACKLE_RADIUS_SQ` today). | aggression, positioning, anticipation, pace |
| marking | **W** | Primary in mark utility `marking·anticipation·pace·concentration` (`off_ball.rs:242`); gates BodyShield, ScreeningInterception. | no | **Gap:** marks the *carrier* position (`off_ball.rs:256`), not a designated opponent — functions as "press toward ball" not "shadow my man", losing the zone-vs-man distinction. | anticipation, pace, concentration, strength, aggression, positioning, tackling |
| free_kicks | **I** | Serialised only; **no dead-ball system** (`lib.rs:2669` "throw-in / corner / goal-kick = Phase 2"). | n/a | **NEW SYSTEM.** Direct FK placement accuracy (xG modifiers) + curve/power (sigma). Needs set-piece restarts. | composure, technique |
| penalty_taking | **I** | Serialised only; no penalty event/shootout. | n/a | **NEW SYSTEM.** Penalty placement + keeper deception vs GK; needs penalty event + taker-vs-keeper resolution. | composure |
| corners | **I** | Serialised only; corner restart not simulated. | n/a | **NEW SYSTEM.** Delivery targeting + in/outswing curve into aerial zones; combines with heading for corner shots. Needs restart + aerial system. | heading, jumping_reach |
| long_throws | **I** | Serialised only; no throw-in sim. | n/a | **NEW SYSTEM.** Specialist throw routine = cross-equivalent flick-on chance; length from `long_throws + strength`. Needs restart + aerial system. | strength, heading |
| (preferred/weak foot) | **I** | Footedness gene exists; sim has **zero foot references** (MASTER_PLAN FUN-PI1). | n/a | Penalise weak-side shot/pass/dribble; drive cut-in-vs-overlap side. (Tracked as FUN-PI1, listed here for completeness.) | finishing, crossing, dribbling |

### Mental (12 — W4 / P6 / I1+1 deferred)

| Attribute | Status | Current effect (file refs) | One-tick? | Intended effect | Combines with |
|---|---|---|---|---|---|
| anticipation | **W** | Primary in four off-ball utilities (track_back/press/mark/run — `off_ball.rs:169,201,242,282`); drives lane_cover_offset (weight 0.50, `off_ball.rs:346`); interception quality (`pass_completion.rs:234`); in (unwired) reactive predicates. | no | Throttle how *early* a player reacts to runs; gate reactive interrupts once wired into dispatch_tick (T1-4). Lane-cover already gives real spatial effect. | positioning, bravery, tackling, pace, stamina, acceleration, marking, concentration, off_the_ball |
| composure | **W** | Best-wired mental: hold_ball primary (`on_ball.rs:774`); two shot paths (shooter_quality + raw_pressure proxy `on_ball.rs:429`); GK aim damping (`goalkeeper_fsm.rs:277`); tackle carrier_quality 0.20 (`lib.rs:1899`); flair shoot-bias. | no | Add decision cadence under pressure + foul likelihood — needs a pressure-state variable that doesn't exist. | strength, balance, finishing, technique, decisions, teamwork, reflexes, positioning |
| decisions | **P** | Only a 0.10 secondary additive at four sites (`on_ball.rs:547,778`; `off_ball.rs:426`; GK `goalkeeper_fsm.rs:352`). Tests *enforce* no effect on cross/dribble/lay_off/press/mark. Max spread ±10%, swamped by 0→1 primary products. | yes | **Modulate softmax temperature**: high decisions ⇒ lower temperature ⇒ near-argmax (picks the best option), not a flat per-utility tilt. | composure, finishing, positioning, teamwork, concentration |
| vision | **W** | Primary in all pass/cross/lay_off utilities; completion composites; GK distribution; progressive-pass proxy; gates diagonal_switch. | no | Shape **scanning radius** (the candidate set considered) once best-target geometry lands. | passing, first_touch, technique, crossing, pace, decisions, long_shots |
| off_the_ball | **P** | Primary in run_off_ball utility → real `RunOffBall` movement (`off_ball.rs:282`), but muted: when `is_defending`, hold (score 1.0) dominates (`subtree_library.rs:306`); only competes in possession; tests bar it from shoot/pass. | no | Should pick *which space* (channel / near post / behind the last man), not a fixed zonal slot; gate signature runs. Muted by hold competition. | pace, acceleration, anticipation, flair, stamina |
| positioning | **W** | Primary in track_back; lane-cover weight 0.20; hold_formation primary `positioning·teamwork·concentration`; tackle quality 0.20 (`lib.rs:1951`); GK reflex_positioning + save composite. | no | Should set *where the default zonal slot sits* (read the line higher), not just step-up aggression; unlock reactive predicates (T1-4). | anticipation, bravery, tackling, pace, stamina, teamwork, concentration, reflexes, composure |
| concentration | **P** | Primary in mark + hold_formation; secondary 0.10 in track_back. Hold path bypassed when defending (`subtree_library.rs:343`); no outcome roll. | yes | Should **decay under fatigue** → late-match lapse events; gate reliability of reactive interrupts. | marking, anticipation, pace, positioning, teamwork |
| bravery | **I** | Only in unwired `predicate_ball_reached_defensive_third` (`reactive.rs:47`, "Not wired into dispatch_tick — defers to T1-4"). | n/a | Commit to 50/50s + aerials, hold the line vs a press, shoot under contact. Lands when T1-4 wires reactive interrupts. | positioning, anticipation |
| teamwork | **P** | Primary in hold_formation; secondary 0.10–0.20 in track_back / lay_off. Hold path bypassed when defending; barred from pass_long by test. | yes | **Team-aggregate** behaviour: compress block width/depth with mean teamwork; trigger overlap runs; prefer wall passes. Needs a team-reading path. | positioning, concentration, composure, first_touch, passing, vision |
| flair | **P** | Secondary 0.10 in run_off_ball + dribble; personality biases (shoot K1=0.30 strongest, long-pass, dribble, cross); shot/dribble telemetry (logging only). All single-tick tilts. | yes | Drive **signature selection probability + menu breadth** (rabona, chip, diagonal switch); raise softmax temperature slightly so high-flair players are less predictable. | dribbling, technique, agility, off_the_ball, risk_appetite, aggression |
| leadership | **I** | (FM-parity gap flagged in DECISIONS 2026-06-06; not yet a field.) | n/a | Team-aggregate morale/cohesion + late-game composure radius. Net-new attribute. | composure, determination, teamwork |

### Physical (9 — W1 / P6 / I2)

| Attribute | Status | Current effect (file refs) | One-tick? | Intended effect | Combines with |
|---|---|---|---|---|---|
| pace | **W** | Sets `v_max = 6.5 + pace·2.5 m/s` every tick for every outfield intent (`dispatch.rs:1513`) — true physics; plus interception weight 0.15; primary in four off-ball utilities; cross primary; GK rush; three signature gates. | no | Add sprint-contest outcomes (foot-race to loose ball), press-reach geometry; combine with acceleration for top-speed-vs-burst. | acceleration, stamina, anticipation, positioning, crossing, off_the_ball |
| acceleration | **P** | Three utility products (press/run/dribble) + PoachersDart gate. **Does not feed `v_max`** — an extreme accelerator moves identically to an average one at the same pace; only picks dribble/run slightly more. | yes | **Time-to-top-speed** (exponential ramp): high accel reaches v_max in fewer ticks ⇒ lethal in tight space. Two-axis profile burst vs cruise. | pace, off_the_ball, anticipation |
| stamina | **P** | In track_back / press / run products + OverlappingSurge gate. **Static** — value at tick 1 == tick 5400; no fatigue accumulator. Permanently inflates pressing utility rather than preserving it. | yes | Drive a **fatigue accumulator**: degrade effective pace/accel/press as f(sprint distance / stamina). 20-stamina near-full at tick 5400; 1-stamina visibly slower by minute 60. | pace, anticipation, positioning |
| strength | **P** | Shot ball speed `20 + 15·(strength·finishing)` (`dispatch.rs:473`) — real physics; hold_ball primary; mark secondary; signature gates. **Not** in tackle carrier_quality (`lib.rs:1898`). | no | Combine with jumping_reach for aerials; enter the tackle contest (strong players harder to dispossess); visible hold-up play. | finishing, composure, balance, jumping_reach |
| agility | **P** | Single use: dribble utility primary (`on_ball.rs:738`). No turning radius / recovery model. | yes | **Turning cost**: low agility ⇒ larger min turn radius (more ticks to reverse) ⇒ pressers cut you off. Multi-tick: paid every direction-change tick. | dribbling, technique, acceleration, balance |
| balance | **P** | hold_ball primary; dribble secondary; shoot secondary (0.60, notably high); tackle carrier_quality 0.30 (`lib.rs:1906`, fires every in-range tick); mark secondary; in unwired predicate_marker_arrived. | no | Primary **contact-resistance** stat: stay on feet after a tackle (model stumble, not just possession transfer); control under pressure; clean landings on headers. | strength, composure, dribbling |
| jumping_reach | **I** | Only serialised (`canonical.rs:682`) + drama_sweep scale. **Most egregious physical gap.** | n/a | **NEW SYSTEM.** Gate aerial duels: highest `jumping_reach·strength` in contest radius wins the header; GK punch success on crosses; set-piece differentiation. | strength, heading |
| natural_fitness | **I** | Only serialised (`canonical.rs:683`) + scale. | n/a | Recovery rate between sprints + match-to-match; the fatigue *refill* rate vs stamina's *tank size*; could gate second-half signature availability. | stamina, recovery_rate |

### Goalkeeper (6 — P5 / I1)

| Attribute | Status | Current effect (file refs) | One-tick? | Intended effect | Combines with |
|---|---|---|---|---|---|
| reflexes | **P** | Dominant 0.45 weight in `gk_quality` → save_prob roll (`lib.rs:2436`); primary in shot-stop aim_factor `reflexes·positioning` (`goalkeeper_fsm.rs:274`); in unwired predicate_shot_incoming. | no | Scale GK **lateral movement speed** in ShotStopping (multi-tick body repositioning) so a wider y-range of shots is reachable; feed save_prob as a non-linear top-end bonus (21 dives visibly wider than 15). Today body position isn't compared to ball y in the save model. | positioning, composure, handling, one_on_ones |
| handling | **P** | Second-largest 0.30 weight in `gk_quality` → save_prob; secondary `(handling+one_on_ones)/4` in aim_factor. | no | **Split catch vs parry**: save_prob (reflexes+positioning) decides the stop; a second `hold_prob` (handling) decides clean catch vs parry → rebound second-ball contest. Multi-tick: poor handling spawns rebounds across the match. | reflexes, one_on_ones, positioning, composure |
| one_on_ones | **P** | Averaged with command_of_area into `aggression_factor` scaling rush_distance every SweeperKeeperRush tick (`goalkeeper_fsm.rs:226`) — real movement; aim_factor secondary; smallest 0.15 weight in gk_quality. | no | Add a **duel-outcome roll** when GK + ball converge during a rush (win / win-but-spill / lose) keyed to one_on_ones vs attacker finishing/dribbling. Today rush distance changes but no duel resolves. | command_of_area, pace, decisions, handling |
| aerial_reach | **I** | Only in `commanding_claim` signature fit-score (`triggers.rs:271`). Not in any FSM state, not in save composite. | n/a | **NEW (rides aerial system).** GK catchment radius on crosses/corners — claim before attackers reach the ball → cuts set-piece xG; multi-tick zone added to GK body during InBoxPositioning/ShotStopping. | handling, command_of_area, jumping_reach |
| command_of_area | **P** | Merged with one_on_ones into rush_distance (`goalkeeper_fsm.rs:226`); signature fit-score. Not in save composite. Confirmed NOT read by shot-stopping. | no | **Split from one_on_ones**: drive the *decision to come for high balls* during InBoxPositioning (distinct from ground 1v1 courage); combine with aerial_reach for the claim radius. | one_on_ones, aerial_reach, handling, pace, decisions |
| kicking | **P** | Forward-bias on distribution target_x (hand `goalkeeper_fsm.rs:315`, feet `:351`) — multi-tick but minor; target is always a fixed slot, ball always arrives (no completion roll). | no | Determine distribution **range + accuracy**: gate short-vs-long by kicking vs distance threshold; feed a pass-completion roll (parallel to outfield model). | passing, vision, composure, decisions |

### Personality (15 — P8 / I7)

| Attribute | Status | Current effect (file refs) | One-tick? | Intended effect | Combines with |
|---|---|---|---|---|---|
| determination | **P** | Utility bias at three defensive sites (cover/mark/hold, k=0.40 each, `personality_bias.rs:221,251,322`). Only nudges intent probability, not the movement target. | yes | Multi-tick **persistence**: keep pressing/tracking when behind, recover faster after being beaten (reduced defensive-intent cooldown or transition speed bonus). | professionalism, work_rate |
| work_rate | **P** | Bias at four sites (press/cover/cross/run, k=0.35, `personality_bias.rs:310,322,193,235`). No speed/distance change. | yes | Ground-covered: lower stamina-decay rate, wider press-trigger radius, transitional speed bonus. | aggression, determination, risk_appetite |
| ambition | **I** | Not read in sim; feeds gene-score dev formulas (`breakthrough_input.rs:162`) — career growth, not match. | n/a | Raise attempt-generating actions under pressure (shots from distance, harder pass); modulate xG gate / long-pass suppressor with risk_appetite. | risk_appetite |
| professionalism | **P** | Bias at one site (hold_formation, k=0.35, `personality_bias.rs:251`). No positional consequence. | yes | Tighten the **shape-discipline radius** (allowed drift from zonal slot), especially late/tired/losing; keep defensive shape longer in the tactic-FSM. | determination |
| loyalty | **I** | Serialised only. | n/a | Career/transfer attribute; match-day use: mild home/big-match composure-determination bonus for current club. | composure, determination |
| temperament | **I** | Serialised only. | n/a | Gate foul/simulation risk (low temperament ⇒ aggression converts to fouls); with aggression produces cards. No foul system to hook yet. | aggression |
| pressure_tolerance | **P** | Attenuates felt pressure `÷(1 + 0.75·PT)` (`personality_bias.rs:355`) feeding shoot + hold bias. Real + propagated, utility-tick only. | yes | Modulate pass_completion under a press (smaller accuracy penalty near defenders); cleaner receive-turn-play-forward. | risk_appetite, aggression |
| big_match_appetite | **I** | Serialised only; MatchState has no importance field. | n/a | Scale composure/finishing/PT by match importance. Needs a match-importance scalar. | composure, finishing, pressure_tolerance |
| adaptability | **I** | Serialised only. | n/a | Reduce out-of-position penalty / mid-match tactic change cost. Needs a role-affinity mismatch metric. | versatility |
| aggression | **P** | Bias at press (k=0.45)/dribble/hold-inverse; BodyShield gate + fit-score. Single-tick + one signature window. | yes | Foul probability (via temperament), physical duels won, 50/50 probability. BodyShield is the closest to combinatorial multi-tick but only starts a window. | work_rate, risk_appetite, temperament |
| risk_appetite | **P** | Four sites: shoot (k=0.40), long-pass (k=0.45 primary), safe-pass inverse, run-off-ball; shot telemetry. Best breadth of the group. | yes | Affect pass_completion (tighter windows ⇒ lower completion); lower xG gate for speculative shots. | work_rate, pressure_tolerance |
| selflessness | **P** | Two sites: lay_off (k=0.35), safe-pass (k=0.30). | yes | Run selection: boost decoy/off-ball runs (with work_rate), lower shoot for a consistent pass preference. | risk_appetite_inv |
| consistency | **I** | Serialised only. Most tractable inert one. | n/a | Modulate **per-match effective-attribute variance**: low consistency ⇒ wider game-to-game fluctuation around the canonical value. Needs a per-match scaling hook. | (all) |
| versatility | **I** | Serialised only. | n/a | Reduce out-of-position attribute degradation. Needs role-affinity mismatch hook. | adaptability |

### Durability (3 — I3) — all blocked on missing host systems

| Attribute | Status | Current effect (file refs) | One-tick? | Intended effect | Combines with |
|---|---|---|---|---|---|
| injury_proneness | **I** | Serialised + validation only; fouls/injury "NOT implemented … T2-4" (`lib.rs:1882`). | n/a | **NEW SYSTEM.** Tackle/collision rolls a `MatchEvent::InjuryOccurred` (× tackler dirtiness, ÷ carrier strength) that degrades pace/accel for the rest of the match — persistent multi-tick ripple. | (tackler) dirtiness, strength |
| recovery_rate | **I** | Serialised only; no fatigue model; `PlayerState.scalars` empty in all match paths (`player.rs:70`). | n/a | **NEW SYSTEM.** Rate the per-player stamina scalar rebounds after sprint/press; `recovery_rate · stamina` = late-match pace floor. Multi-tick over the full 90. | stamina, natural_fitness |
| dirtiness | **I** | Serialised only; tackle model excludes it (`lib.rs:1900`). | n/a | **NEW SYSTEM.** Widen foul radius → `MatchEvent::FoulCommitted`; amplify injury on landed tackle; accumulate to `BookingIssued` (yellow/red → 11v10 persistent shape change). 21-dirtiness = near-certain late booking (visible liability). | injury_proneness, aggression, temperament |

---

## 3. The non-linear effect curve

### 3.1 Goal

Make **elite visibly elite**. Replace the implicit linear `effect = attr` with a deterministic Q32 map `g: [0,1] → [0,1]` such that the marginal gain rises toward the top: `g(20)−g(19) > g(12)−g(11) > g(2)−g(1)`. The curve is applied at the **effect-magnitude** boundary — when a stored attribute is converted into a multiplier, a contest weight, an xG feature scale, a movement-speed coefficient, or a dispersion (sigma) — **not** to the canonical stored value, which stays the raw 1..=20 → Q32 `[0,1]` encoding the gene compiler produces.

### 3.2 Shape — gamma (convex power) with an elite kicker

Attributes are stored as Q32 in `[0,1]` (1→~0.05, 20→1.0). Define the effect transform as a **convex power curve** with exponent γ > 1:

```
g(a) = a^γ      with γ = 1.7   (default for "skill expression" attributes)
```

A power curve is the right primitive because (a) it is monotone and smooth, (b) γ alone tunes how front-loaded vs back-loaded the gains are, and (c) it is trivially deterministic in Q32: `a^γ` for fixed γ is `exp(γ · ln a)`, and we already have CORDIC on Q32 (`q32.rs` exposes `sqrt`/`acos`; `exp`/`ln` come from the same `cordic` family). For the small fixed set of exponents we use, a **precomputed 256-entry Q32 lookup table with linear interpolation between entries** is both faster and avoids per-tick CORDIC cost — and a LUT is perfectly deterministic and cross-platform (no float, no transcendental rounding divergence). The LUT is generated once at content-bake time and committed, or generated in a `const`/`OnceLock` init from integer math.

Why γ ≈ 1.7 rather than 2.0: at γ=2 the bottom half is almost flat (a 10/20 player expresses only 25% of the effect), which over-punishes mid-tier squads and makes the league feel binary. γ=1.7 keeps mid-tier meaningful while still curving the top:

| Stored attr (1..=20) | linear g=a | γ=1.7 g(a) | marginal Δ (γ=1.7) |
|---|---|---|---|
| 10 | 0.50 | 0.31 | — |
| 11 | 0.55 | 0.355 | 0.045 |
| 12 | 0.60 | 0.40 | 0.045 |
| 18 | 0.90 | 0.835 | — |
| 19 | 0.95 | 0.915 | 0.080 |
| 20 | 1.00 | 1.000 | 0.085 |

The 19→20 step (0.085) is **~1.9× the 11→12 step** (0.045) — elite skews, exactly the mandate, without a discontinuity.

### 3.3 Per-class exponents

One γ for everything would be wrong — a poacher's finishing should curve harder than a journeyman's stamina. Assign γ by effect role, stored as a tuning table in `docs/design/attribute-effect-curves.md` (coefficients live in design docs per design-docs/RULES.md §4, not in SPEC):

| Effect class | γ | Rationale |
|---|---|---|
| **Skill expression** (finishing, technique, dribbling, passing, crossing, vision, first_touch) | 1.7 | Elite technicians should feel disproportionately better; this is where "GOAT" reads. |
| **Physical ceiling** (pace→v_max, strength→shot speed, acceleration ramp) | 1.4 | Physics caps the spread (v_max only ranges 6.5–9.0); a gentler curve keeps the band realistic but still rewards the top. |
| **Contest / duel** (tackling, marking, balance, jumping_reach, heading, one_on_ones, aerial_reach) | 1.8 | Duels are winner-take-all moments; the curve should make the elite defender/keeper *reliably* win, not marginally. |
| **Mental composite** (composure, anticipation, positioning, decisions, concentration) | 1.6 | Read/decision quality compounds across many ticks; moderate convexity avoids runaway. |
| **Personality bias** (work_rate, determination, aggression, risk_appetite, …) | 1.3 (near-linear) | These are *tendency* dials, not *quality*; over-curving them would make extreme personalities erratic rather than characterful. |

### 3.4 Where the curve binds in existing code

The transform is inserted at the read site, leaving formulas otherwise intact:

- **Utility products** (`on_ball.rs`, `off_ball.rs`): each primary factor `attr` becomes `g_class(attr)` before the product. Because products of curved factors compound, a player elite in *two* combining attributes (e.g. dribbling + agility) gets a super-linear joint edge — directly serving the "combinatorial" mandate.
- **Outcome composites** (`pass_completion.rs`, `lib.rs` tackle, `lib.rs` gk_quality): apply `g` to each weighted term before the weighted sum so the elite end of the *dominant* term (e.g. finishing in shooter_quality) pulls hard.
- **Physics coefficients**: `v_max = 6.5 + g_phys(pace)·2.5`; shot speed `bonus·g_phys(strength)·g_skill(finishing)`; sigma shrinks with `g_skill(shooter_quality)`.
- **xG logistic** (`xg.rs`): the distance feature is scaled by `g_skill(long_shots)` so low-long_shots players generate negligible xG from range (fixes the §2 long_shots gap).

### 3.5 Determinism guardrails

- The LUT is `[Q32; 257]` per γ (256 segments + endpoint), values from `a^γ` computed once via integer/CORDIC at init; lookup is `idx = (a · 256).floor()` + linear interp in Q32. No `f32`/`f64` in the canonical path (Tauri DTO conversion to f64 stays at the UI boundary only).
- The curve is a **pure function of the stored attribute** — same input, same Q32 output, every platform. It is exercised by a proptest invariant (`g` monotone non-decreasing; `g(0)=0`, `g(1)=1`; marginal-Δ increasing) and an insta snapshot of `g` at 21 sample points, gated by the canonical-hash regression.

---

## 4. The "21" super-ceiling

### 4.1 What a 21 *is*

The 1..=20 scale maps to Q32 `[0,1]`. A "21" is a value **above 1.0** in the same Q32 space — the gene→attribute compiler (T4.5-E0) may emit, for an extraordinarily rare gene draw, an attribute whose `AbilityCeiling` permits a current value of e.g. **Q32 ≈ 1.05** (the raw scale's "21"). Q32 is `FixedI64<U32>` and represents values >1.0 natively; nothing in the type stops it. The constraint is **rarity + gating**, not representation.

**Rarity gate (GOAT scarcity, 1–2 players per ~20 years):** the compiler only permits a 21 when *all* of:
1. The gene draw lands in the extreme tail (the existing cohort-weighted P(0,1,2,3) signature-affinity distribution already models a long tail; the 21 sits beyond the P3 affinity tier).
2. The player carries the `peak_ceiling_high` narrative flag (one of the 4 existing `NarrativeFlag`s).
3. `AbilityCeiling.potential` is itself in the >1.0 band — so the 21 must be *earned through development* (breakthrough lifts current toward an already-super-max potential), not granted at generation. This ties the 21 to the breakthrough pillar: a 21 is the visible payoff of a generational talent realising a generational ceiling.

Across a ~2000-player pyramid regenerated per save, the tail mass is tuned (in `docs/design/attribute-effect-curves.md`) so expected 21-holders ≈ 0.05–0.10 *per attribute* at world-gen, climbing only as a handful of `peak_ceiling_high` prodigies break through over a decade-long career — landing the "1–2 per 20 years" target. The AbilityCeiling type is the enforcement point: a save cannot deserialise an attribute >1.0 unless the ceiling sanctions it (validated like the existing `n.current ≤ potential` invariant).

### 4.2 How a 21 *unlocks an action*, not just a probability

This is the crucial difference from §3. The non-linear curve makes 20 *better* than 19; the 21 must make the player do something **others physically cannot attempt**. Mechanism: a 21 **opens a new branch in the decision tree / a new signature predicate** that is *gated behind attr > 1.0*.

The signature system already exists (`signature/triggers.rs`, `SeedLayer::SignatureTrigger`, candidates per slot). Each signature trigger has a **gate** (threshold) and a **fit-score**. A 21-gated signature simply sets its gate above 1.0, so only a super-ceiling player can ever pass it:

| Attribute at 21 | Unlocked action (new gate `> 1.0`) | Reads as |
|---|---|---|
| **vision / passing** | `OverTheTopThread` — a dropped over-the-top curling pass into a runner's path. Adds a pass *type* unavailable below 21 (curve + weight that the normal pass model never selects), targeting the runner's *future* position, not current. | "He's seen it before anyone — bent it over the top into the run." |
| **pace** | `BurstGear` — a transient v_max ceiling lift above 9.0 m/s for a short window, available only when the gate passes; the accelerator the rest of the league cannot match in a foot-race. | "There's another gear there nobody else has." |
| **finishing** | `ImpossibleAngle` — unlocks a shot decision from an xG-zone the normal gate suppresses (tight angle, under pressure) that no sub-21 finisher will attempt. | "From there? He doesn't miss from there." |
| **dribbling** | `TheEscape` — a dribble-out-of-a-double-team branch with a non-linear tackle-resistance edge that only the 21 wins reliably. | "Two on him and he's gone through both." |
| **reflexes** (GK) | `TheImpossibleSave` — a save reachable from a y-coverage band beyond any sub-21 keeper's dive. | "That should have been a goal. How has he reached that?" |

Because the gate is a hard threshold above the normal cap, a 21 is **categorical**, not incremental: the action exists or it doesn't. The fit-score (the §3 curve at the super-max) then makes execution *reliable* once unlocked.

### 4.3 Visibility — on the pitch and in commentary

- **On the pitch (PixiJS board):** a 21 signature emits a distinct `MatchEvent` that the board renders with an emphasis frame (the existing per-tick scrub already surfaces events). The over-the-top thread draws a visibly different ball trajectory; BurstGear shows the carrier pulling clear of the chase pack.
- **In commentary:** the 21 signature carries its own Tracery template slot bank (≥3 variants each, per Content/RULES §4) authored by the narrative-director. **Football-native vocabulary only** — no "+1 above max", no capitalized mystical state-noun. The copy describes the *moment* ("over the top, into the run, and he's in"), never the number. This satisfies the banned-terms lint by construction.
- **Career memory:** a 21 signature firing in a real match is exactly the kind of event the append-only ledger (Pillar 2) should remember — a `LegacyGoal`/`SignatureMoment` `MemoryEvent` the player surfaces years later ("the night he bent one over the top in the cup final").

### 4.4 Dependencies

- **T4.5-E0 gene→attribute compiler** must emit the >1.0 band only under the rarity gate (§4.1). Until E0 lands, a 21 can be hand-seeded in a single `PlayerTemplate` for testing the signature unlock path.
- **AbilityCeiling** is the representation + validation gate for the super-max (current ≤ potential, both allowed >1.0 only with `peak_ceiling_high`).
- **Signature predicate set** must add the 21-gated predicates; the EA floor is 8-of-24 predicates implemented (all 24 authored) per the roadmap re-baseline, so the 21-gated ones can be authored now and implemented as the slice lands.

---

## 5. Specialization visibility

The mandate's fifth point: an extreme attribute must produce a **visible on-pitch moment a player can be built around**. The curve (§3) and the 21 (§4) supply the mechanism; specialization is the *experience* — and it must read without ever showing a stat line.

**The chain is: extreme attribute → biased decision/movement → a recurring, recognizable on-pitch pattern → a tactic shaped around it → commentary that names the pattern, not the number.**

Worked examples:

- **The poacher (elite finishing + off_the_ball, with the §6 box-run wiring):** the player repeatedly times runs onto the last shoulder and finishes first-time. The manager builds around it by feeding crosses and through-balls; the board shows the same near-post dart again and again; commentary: "he lives on that last shoulder." The specialization is *legible from watching*, which is the text-first presentation pillar.
- **The destroyer (elite tackling + anticipation + the split-out interception):** steps into passing lanes, kills attacks before they form. Built around as the screen in front of the back line. The lane-cover step-up (already wired) plus the larger effective tackle radius (§2 gap) makes it spatially visible.
- **The metronome (elite passing + vision + the passing-weighted receiver selection, §2 gap):** consistently finds the best open receiver; the team's progression routes visibly bend through them.
- **The athlete (elite pace + acceleration ramp + stamina fatigue model):** still sprinting past tired defenders at minute 85 when others have slowed — a specialization that only *emerges* because fatigue is modelled (§6). Built around with a high line and transitions.

**Why combination matters for visibility:** a single elite attribute in isolation often reads as noise; it is the **combinatorial product of two curved factors** (§3.4) plus a **biasing personality** (work_rate, risk_appetite) that produces a *repeated, recognizable* pattern. The audit's `combinesWith` lists are therefore the specialization design surface — each pairing is a buildable identity.

The frontend obligation (UI never drives canonical state): surface specialization as **commentary + the tactical board's recurring visual**, plus scout-prose ("looks sharper in the box") — never a raw numeric stat-delta callout. This is enforced by the banned-terms lint over `frontend/src/`.

---

## 6. Prioritized wiring plan

Ordering principle (from the mandate + the dynamic-positioning work already underway): **movement-shaping attributes first** (they ripple across every subsequent tick and are the cheapest path to "the match feels alive"), then **contest systems** (turn partial duel attributes into real outcomes), then the **net-new subsystems** (set-pieces, fouls/injuries) that unlock the inert clusters wholesale. Each slice ships as a **measured, goal-tracked increment** — the build-out calibration policy: instrument an outcome metric, observe it move in the intended direction, **do not gate** on hitting a number.

The non-linear curve (§3) is **Slice 0** — it lands first because every later slice's "elite skews" behaviour rides on it, and it touches only the read sites of already-wired attributes (low blast radius, no new system).

| # | Slice | Attributes activated | New system? | Tracked goal (observe, don't gate) |
|---|---|---|---|---|
| **0** | **Non-linear effect curve** (§3 LUT + per-class γ at read sites) | All 17 wired + every later one | no | Elite-vs-mid outcome gap widens; 19→20 marginal > 11→12; canonical hash re-pinned with documented reason. |
| **1** | **Fatigue accumulator** (per-player stamina scalar in `PlayerState.scalars`, drains on high-velocity ticks, recharges by recovery_rate) | stamina P→W, recovery_rate I→W, natural_fitness I→W, work_rate (decay rate), concentration (lapse) | yes (small) | Late-match effective pace/press visibly degrade for low-stamina players; high-stamina near-flat to tick 5400. |
| **2** | **Acceleration ramp + agility turning cost** (v_max ramp = f(acceleration); min turn radius = f(agility)) | acceleration P→W, agility P→W, balance (turn stability) | yes (small) | A high-accel/avg-pace player wins tight-space situations; low-agility carriers get cut off more often. |
| **3** | **Off-ball run targeting + box runs** (run picks a *space*: channel/near-post/behind-line; finishers run goalward) | off_the_ball P→W, finishing (movement half), flair (run style), selflessness (decoy runs), teamwork (overlaps) | no (extends positioning model) | Possession-phase player spread increases; finisher box-entry frequency up; recurring run patterns visible on the board. |
| **4** | **Aerial duel system** (contest when ball_z > threshold; `jumping_reach·heading` winner; header shot-type into xG) | jumping_reach I→W, heading I→W, strength (header power), aerial_reach (GK), command_of_area (claim decision) | yes (major) | Headers appear in match output; tall/strong players win aerials; GK claims cut cross→shot conversion. |
| **5** | **Contested-ball depth: first-touch control window + GK catch-vs-parry + 1v1 duel** | first_touch (control window), handling (parry split), one_on_ones (duel roll), strength (tackle contest), balance (stumble) | yes (medium) | Poor first touch raises dispossession near a press; poor-handling GK spawns rebounds; GK 1v1 outcomes vary by one_on_ones. |
| **6** | **Decision-quality & scanning** (decisions → softmax temperature; vision → scanning radius / best-receiver selection; passing-weighted target) | decisions P→W, vision (scanning), passing (target selection), pressure_tolerance (pass under press) | no | High-decisions players pick the argmax option more often; passing skews *which* receiver gets the ball. |
| **7** | **Personality persistence & shape discipline** (determination/work_rate → defensive cooldown + transition speed; professionalism → shape-discipline radius; pressure_tolerance → pass_completion penalty) | determination, work_rate, professionalism, aggression (50/50 duels), risk_appetite (pass windows) — all P→W | no | High-work-rate players cover more ground; disciplined players hold shape late; risky passers complete tighter windows less. |
| **8** | **Foul / card / injury system** (dirtiness → foul radius + cards; injury_proneness → injury event; temperament → foul conversion) | dirtiness I→W, injury_proneness I→W, temperament I→W, aggression (foul link) | yes (major) | Fouls/cards/injuries appear; dirty players book themselves into liabilities; injuries cause 11v10 shape ripple. |
| **9** | **Set-piece restarts** (dead-ball: free kicks, corners, throw-ins, penalties → delivery + aerial finish) | free_kicks, penalty_taking, corners, long_throws — all I→W; kicking (GK) → completion roll | yes (major) | Set-piece goals appear; specialists measurably better delivery; corners feed the §4 aerial system. |
| **10** | **Per-match variance & context** (consistency → per-match attr variance; big_match_appetite → importance scaling; loyalty/adaptability/versatility → role-fit + home boosts) | consistency, big_match_appetite, loyalty, adaptability, versatility — all I→partial | needs match-importance + role-fit hooks | Inconsistent players vary game-to-game; out-of-position penalties appear. |
| **11** | **21-gated signature predicates** (the unlock actions in §4.2) | the elite-tier readout for the curve | rides signature system | A seeded 21 fires its unique action visibly; commentary + ledger record it. |

**Sequencing rationale.** Slices 0–3 are the movement spine — they make the match feel alive and turn the largest cluster of *partial* attributes into real ones with the smallest new code. Slice 4 (aerial) is promoted high despite being a new system because `jumping_reach` is the single most egregious inert gap and it unblocks five attributes plus the corner/throw routines downstream. Slices 8–9 are the heaviest (whole new systems) and clear the entire set-piece + durability inert clusters at once. Slice 11 is last because it is the *payoff* surface — it only reads well once the underlying attributes (§3 curve, the contest systems) are doing honest work.

Every slice owes the standard sim discipline: an `insta` snapshot of the new behaviour's canonical state at a known tick, a `proptest` invariant for the property it preserves, and `scripts/fw verify` green with the canonical-hash regression intact (re-pinned only when the slice spec authorizes it, with the reason in the commit body).

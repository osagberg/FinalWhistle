# Believability slice specs (DRAFT — captured from the spec-pipeline lane, 2026-06-04)

**Status:** DRAFT / implementation-ready specs. Produced by the parallel design-spec lane (`systems-designer` agents, grounded in `football-fidelity-audit.md` + `match-realism-reference.md` + the actual code seams). Each cluster will be split into a polished per-system design doc + MASTER_PLAN-wired as the engine lane reaches it. **Read the relevant cluster before implementing FUN-LM*/FUN-CB*/FUN-TI*/FUN-PI*.** Tuning numbers here are SOFT seeds for the drama-sweep loop.

**Determinism:** every mechanic below is specced Q32 / BTreeMap / seed_fn-keyed / assert-not-debug_assert and tags its canonical-surface impact (sidecar vs new-field vs new-event → rebaseline class).


---

# Living-Match cluster (FUN-LM1-5: fatigue, score-state→FSM, momentum, subs, half-time)

I have everything I need. Now I'll produce the spec.

---

## LIVING-MATCH cluster — FUN-LM1 through FUN-LM5

**Audit anchors:** A1 (fatigue), A2 (score+clock FSM), A3 (momentum), A4 (substitutions). Realism anchors from `match-realism-reference.md` cited inline. Firmness tags (HARD/SOFT) match that doc.

---

### FUN-LM1 — In-match fatigue (per-tick `match_fitness` drain)

#### Mechanic

`PlayerCondition.match_fitness` exists in `fw-core/src/player_attributes.rs:944` and is declared as a per-match draining condition value — it is currently inert (no writer, no reader). This sub-mechanic wires it as a canonical per-player scalar tracked in `PlayerState.scalars` via a fixed key.

**Canonical key:** introduce `pub const SCALAR_MATCH_FITNESS: u16 = 0x0010;` in `fw-match-sim` (or `fw-core`). All 22 players initialise to `Q32::ONE` at match-init. The scalar persists in `PlayerState.scalars` (already a `BTreeMap<u16, Q32>`, encoded canonically) — no new struct field needed, no schema bump beyond adding the key.

**Per-tick drain (step 5.5 — after FUN-TS1 shape compute, before dispatch):**

```
activity_load(slot, state) ->
  if in_press_role(slot, state)      => LOAD_PRESS
  elif moving faster than JOG_THRESH => LOAD_RUN
  else                               => LOAD_WALK

drain(slot) = BASE_DRAIN
            + activity_load(slot, state) * ACTIVITY_SCALE
            - player.attributes.physical.stamina * STAMINA_RELIEF
            - player.attributes.physical.natural_fitness * FITNESS_RELIEF

new_mf = max(MF_FLOOR, current_mf - drain)
```

All arithmetic in Q32. `max` comparison is Q32 compare — no float. No saturation on sim newtypes; if drain would bring mf below `MF_FLOOR`, clamp with explicit branch and `assert!(new_mf >= MF_FLOOR)`.

**Utility multiplier (in `BtContext`, passed into `select_outfield_intent`):**

```
fatigue_factor(mf) = MF_FACTOR_FLOOR + mf * (Q32::ONE - MF_FACTOR_FLOOR)
```

This produces `[MF_FACTOR_FLOOR, 1.0]`. The multiplier scales the **physical and technical utility branches only** — mental utility (vision, decisions, anticipation) degrades slower than physical output in real football. Concretely: any utility that reads `pace`, `acceleration`, `stamina`, `tackling`, `dribbling`, or press-intensity is post-multiplied by `fatigue_factor`. Commentary-surfaced only as "looking heavy-legged" / "the press is breaking down" — no numeric exposed to the player.

**SeedLayer:** none — drain is deterministic, no random draw. Activity classification reads position/velocity from canonical state.

**Canonical-surface impact:** new scalar key in existing `PlayerState.scalars` BTreeMap + the scalar init loop in `MatchState::initial`. This is a canonical field addition (trigger #1) requiring a multi-pin rebaseline. **Rebaseline class: SCHEMA + BEHAVIORAL** (positions drift as fatigued players hold shape less precisely).

#### Determinism shape

All Q32. No float. No RNG for the drain itself. The activity classification uses only `vel_x`, `vel_y` (canonical), and `press_role` from the FUN-TS2 `PressPlan` sidecar (`#[serde(skip)]` — OK since PressPlan is recomputed each tick before this step). The `cordic` crate is not needed — drain is affine arithmetic on already-known scalars.

#### Tuning bands (all SOFT — iterate via drama-sweep)

| Parameter | Seed value | Basis |
|---|---|---|
| `BASE_DRAIN` per tick | `Q32::from_raw(162)` ≈ 3.8e-8/tick → total drain over 5400 ticks ≈ 0.021 before activity offset | **SOFT.** Targeting ~15% net mf loss by 90' for a mid-stamina player; the actual FM-style observation is a press visibly breaks down after 60-70 min. |
| `LOAD_WALK` | `Q32::ZERO` (baseline) | **SOFT.** Standing/holding shape costs nothing extra beyond BASE. |
| `LOAD_RUN` | `Q32::from_raw(322)` ≈ 7.5e-8/tick | **SOFT.** Running doubles drain; calibrate so sprint over 90 min = ~30% total drain. |
| `LOAD_PRESS` | `Q32::from_raw(644)` ≈ 1.5e-7/tick | **SOFT.** Pressing = 4× baseline. The Footballizer/Coaches' Voice press-fatigue study (cited in `match-realism-reference.md` §3) confirms press effectiveness decays with fatigue; this operationalises that claim. **HARD ordering: press > run > walk.** |
| `STAMINA_RELIEF` | `Q32::from_raw(80)` | **SOFT.** A max-stamina player (1.0) reduces drain by ~1.9e-8/tick. |
| `FITNESS_RELIEF` | `Q32::from_raw(40)` | **SOFT.** Additive with stamina relief. |
| `MF_FLOOR` | `Q32::from_raw(1_717_986_918)` ≈ 0.40 | **SOFT.** Floor at 40% — even exhausted players don't become useless. Iterate. |
| `MF_FACTOR_FLOOR` | `Q32::from_raw(1_288_490_188)` ≈ 0.30 | **SOFT.** At mf=0.40 (floor), fatigue_factor ≈ 0.72 — a 28% utility cut. Noticeable without being catastrophic. |
| `JOG_THRESH` speed | `Q32::from_raw(3_i64 << 32)` = 3 m/s | **SOFT.** Players moving faster than jogging pace classified as RUN. |

**Worked example A:** Mid-stamina player (stamina=0.5, natural_fitness=0.5), walking baseline for full 90 min.
- `drain_per_tick` = 162 + 0 - 0.5×80 - 0.5×40 = 162 - 40 - 20 = 102 raw/tick
- Over 5400 ticks: 102 × 5400 = 550_800 raw ≈ 0.000128 in Q32 ≈ 1.3% total drain. Barely tired — realistic for a deep-sitting midfielder who rarely sprints.

**Worked example B:** Same player, pressing (LOAD_PRESS) for 30% of the match (1620 ticks), running for 40% (2160 ticks), walking 30% (1620 ticks).
- Press portion: (162 + 644 - 60) × 1620 = 746 × 1620 = 1_208_520
- Run portion: (162 + 322 - 60) × 2160 = 424 × 2160 = 915_840
- Walk portion: 102 × 1620 = 165_240
- Total raw = 2_289_600 → Q32 ≈ 0.00053 (× 2^32) → ~0.053% ... wait, Q32 raw bits: 2_289_600 / 2^32 ≈ 0.000533 in [0,1] units. 90' total drain ≈ 0.05 from Q32::ONE. That's only 5% drain — too light. The seed values need tuning upward after drama-sweep. Recalibrate BASE_DRAIN at first run.

**Worked example C:** Max-stamina (stamina=1.0) vs. min-stamina (0.0) player, full pressing 90 min.
- Max: drain/tick = 162 + 644 - 80 - 40 = 686; total = 3_704_400 raw ≈ 0.086 → ~8.6% drain.
- Min: drain/tick = 162 + 644 - 0 - 0 = 806; total = 4_352_400 raw ≈ 0.101 → ~10.1% drain.
- The ordering is correct (higher stamina = less drain). The gap is small (~1.5pp) at these seed values — BASE_DRAIN and ACTIVITY_SCALE should be increased ~5× in the first tuning pass to achieve the intended ~15-25% total drain spread for heavy activity. The examples reveal that the raw seeds are undertuned; drama-sweep is the calibration loop.

#### Proptest invariants

1. `match_fitness_monotone_decreasing` — for any player, `mf[t+1] <= mf[t]` for all ticks (drain never reverses mid-match; recovery is FUN-LM5 half-time only).
2. `match_fitness_floor_respected` — `mf[t] >= MF_FLOOR` for all ticks, all players. Violation = `assert!` fires in production.
3. `high_stamina_retains_more_fitness` — same seed, two players differing only in `physical.stamina` (0.2 vs 0.8), same activity load for 5400 ticks → high-stamina player ends with higher mf.
4. `press_drain_exceeds_run_drain` — two players with identical attributes, one classified PRESS and one RUN for 100 ticks → press player has lower mf at tick 100.

#### Acceptance

- Drama-sweep: measure mean utility reduction in ticks 4200-5400 (70'-90') vs ticks 0-1800 (0'-30'). Target: physical utility branches ~10-25% lower in the final phase vs the opening phase at SOFT seed values (iterate). No guard on the exact number — existence of a detectable gradient is the bar.
- Contact-sheet: a trailing team's press should visibly loosen in the 85th-minute frames as fatigue multiplier reduces press utility weight.

---

### FUN-LM2 — Score+clock → tactic FSM

#### Mechanic

The `tactic_fsm` heartbeat (`tick_match` step 5, `lib.rs:2236-2247`) fires every 30 ticks per team (2 Hz) but today only implements the `HighPress` timeout rule (`heartbeat_check`, `tactic_fsm.rs`). Score and clock never reach it (noted in audit A2; deferred comment in T1-2b-iii).

**New heartbeat arms** added to `heartbeat_check` (or a new `score_clock_check` called from the same step-5 seam):

```
fn score_clock_check(
    tts: TeamTacticState,
    params: &ArchetypeParams,
    score_lead: i8,           // positive = this team is leading
    ticks_remaining: u32,
    mf_mean: Q32,             // team mean match_fitness for FUN-LM1 interaction
) -> Option<TeamTacticState>
```

**Transition table (determistic predicates — no RNG):**

| Predicate | New state | Commentary read |
|---|---|---|
| `score_lead <= -2` AND `ticks_remaining <= CHASE_WINDOW` | `HighPress` (or stay) | "going all-out" |
| `score_lead <= -1` AND `ticks_remaining <= DESPERATION_WINDOW` | `HighPress` | "hunting an equaliser" |
| `score_lead >= 1` AND `ticks_remaining <= HOLD_WINDOW` | `LowBlock` | "sitting deep" |
| `score_lead >= 2` AND `ticks_remaining <= HOLD_EARLY_WINDOW` | `LowBlock` | "protecting the lead" |
| `score_lead >= 1` AND `mf_mean < MF_TIRED_THRESHOLD` | bias LowBlock utility; no forced state | "conserving energy" |

The "bias LowBlock utility without forced state" row is not a transition — it's a weight fed into `ArchetypeParams.default_in_defence_state` at the heartbeat, so fatigued leaders organically shift rather than snap. This keeps the FSM clean and avoids the snap-transition robotics the FUN-TS1 Lipschitz invariant already guards against.

**`score_lead`:** derived each tick in `tick_match` from `state.home_score` and `state.away_score` — already canonical fields. Not a new field.

**`ticks_remaining`:** `state.match_end_tick.to_raw() - state.tick.to_raw()` — derived, not stored.

**SeedLayer:** none. All predicates are pure comparisons on canonical state.

**Canonical-surface impact:** no new canonical fields. Behavioural change (tactic states flip differently) → **BEHAVIORAL-ONLY rebaseline** (ADR-0012 trigger #3). Single-pin rebaseline if implemented in isolation.

#### Determinism shape

All integer comparisons on canonical `u8` scores and `i64` ticks. Q32 mf_mean = `sum of mf scalars / 10` (for the 10 outfielders per team) using Q32 division. Cordic not needed.

#### Tuning bands

| Parameter | Value | Firmness | Basis |
|---|---|---|---|
| `CHASE_WINDOW` (≥2 down) | 900 ticks = 15 min | **SOFT** | A 2-goal trailing side goes all-out with ~15 min left in real football. ORDERING is HARD (earlier for larger deficit). |
| `DESPERATION_WINDOW` (1 down) | 540 ticks = 9 min | **SOFT** | Mirrors real patterns: a team 1 down with ~10 min left commits. Late-goal anchor: ~25% of goals in final 15 min (HARD, `match-realism-reference.md` §4) — this window enables the mechanism. |
| `HOLD_WINDOW` (1 up) | 600 ticks = 10 min | **SOFT** | Leading sides drop to LowBlock with ~10 min left; drama-sweep M4 comeback rate (target ~15-17%, SOFT from §4) is the calibration. |
| `HOLD_EARLY_WINDOW` (2+ up) | 1200 ticks = 20 min | **SOFT** | 2-goal lead → deeper block 20 min earlier than 1-goal lead. 2+ goal comeback is ~0.7% (HARD from §4) — this window should make comebacks genuinely rare, not impossible. |
| `MF_TIRED_THRESHOLD` | `Q32::from_raw(2_576_980_378)` ≈ 0.60 | **SOFT** | Team is "tired" when mean mf < 60%. |

**Worked example:** Home team is 1-0 up at tick 4860 (81st minute). `ticks_remaining` = 5400 - 4860 = 540 = `DESPERATION_WINDOW` exactly. Away team triggers the desperation arm → `HighPress`. Home team: `score_lead=+1`, `ticks_remaining=540` < `HOLD_WINDOW=600` → `LowBlock`. Now away presses a sitting home block — the textbook late-game pattern.

**Worked example:** Home 1-0, tick 4860, home team mean mf = 0.55 < MF_TIRED_THRESHOLD. Home does NOT snap to LowBlock (already there from the HOLD_WINDOW arm). The mf bias just reinforces the utility weighting. No double-transition.

#### Proptest invariants

1. `trailing_side_raises_aggression_near_end` — for any state where `score_lead <= -1` AND `ticks_remaining <= DESPERATION_WINDOW`, the home FSM state is `HighPress` within 2 heartbeat intervals (60 ticks).
2. `leading_side_drops_block_near_end` — for any state where `score_lead >= 1` AND `ticks_remaining <= HOLD_WINDOW`, the FSM state is `LowBlock` within 2 heartbeat intervals.
3. `score_clock_predicates_are_deterministic` — same `(score_lead, ticks_remaining, mf_mean, tts)` tuple always produces the same `Option<TeamTacticState>`.
4. `blowout_hold_earlier_than_one_goal_hold` — for score_lead=2 and score_lead=1, the state is `LowBlock` at a strictly earlier tick.

#### Acceptance

Drama-sweep metric: M4 comeback rate should land near 15-17% (SOFT; `match-realism-reference.md` §4). M7 late-winner rate should rise with this mechanic active. Contact-sheet: at tick 4800 (80'), a trailing team's shape should visibly pull forward vs a leading team's visible withdrawal.

---

### FUN-LM3 — In-match momentum (sidecar)

#### Mechanic

A `#[serde(skip)]` per-team sidecar on `MatchState`, exactly the pattern of `team_shape` (FUN-TS1, `lib.rs:517-519`).

```rust
pub(crate) struct MomentumState {
    /// Q32 in [0, 1]. 0.5 = neutral; >0.5 = this team has momentum.
    value: Q32,
    /// Tick at which the last momentum impulse landed.
    last_impulse_tick: Tick,
}
```

Field in `MatchState`:
```rust
#[serde(skip)]
pub(crate) team_momentum: [MomentumState; 2],
```

Default via `MomentumState::neutral()` = `{ value: Q32 midpoint, last_impulse_tick: Tick::ZERO }`.

**Impulse events** (processed in `tick_match` at the event-emission sites, AFTER canonical events are pushed):

| Event | Home impulse | Away impulse |
|---|---|---|
| `MatchEvent::Goal { score_home_after > score_away_after }` (home goal) | +`GOAL_IMPULSE` | -`GOAL_IMPULSE` |
| `MatchEvent::Goal` (away goal) | -`GOAL_IMPULSE` | +`GOAL_IMPULSE` |
| GK save (within the existing save-block, `lib.rs:~1010-1024`) | +/- `SAVE_IMPULSE` to the saving team | mirror |

Red cards and big chances saved are post-FUN-LM3 additions when those events exist.

**Decay (per tick, step 5.5 alongside fatigue):**

```
gap = team_momentum.value - MOMENTUM_NEUTRAL
new_value = MOMENTUM_NEUTRAL + gap * MOMENTUM_DECAY_FACTOR
```

`MOMENTUM_DECAY_FACTOR` is Q32 slightly below `Q32::ONE` (e.g. `Q32::ONE - DECAY_RATE_PER_TICK`). Over ~600 ticks (10 min) this should decay a goal impulse to ~25% of its original magnitude. The decay is geometric — no exponential/float needed since it's just repeated Q32 multiplication.

**Consumption (in `dispatch.rs` utility selection):** the sidecar's `value` is read by `select_outfield_intent` as a bias passed through `BtContext`:

```
momentum_bias(team_idx) -> Q32
  let m = state.team_momentum[team_idx].value
  // Positive momentum → slightly higher press utility weight
  // Negative momentum → slightly higher hold/recover utility weight
  // Returns a signed scalar centred on 0; clipped to ±MOMENTUM_CAP
```

Internally: `momentum_bias = (value - MOMENTUM_NEUTRAL) * MOMENTUM_SCALE`. This is added to the existing `personality_bias` composite before utility selection — momentum is an in-match personality nudge, not a separate decision tree.

**SeedLayer:** `SeedLayer::MemoryEvent` — momentum nudges are deterministic (no RNG needed for decay or impulse). The impulse magnitude is a constant, not drawn. Name a `site` of `0xMOM0` for any future probabilistic momentum component; today no draw is needed.

**Canonical-surface impact:** `#[serde(skip)]` — zero canonical bytes. No rebaseline from the sidecar itself. Behavioural change (press utility shifts) → **BEHAVIORAL-ONLY rebaseline** if paired with FUN-LM2 or FUN-LM1 (fold into those rebaselines). Standalone: the sidecar adds no canonical bytes, so the momentum bias alone requires a behavioral rebaseline (trigger #3).

#### Determinism shape

Decay: Q32 multiply. Clamp: Q32 compare + branch (no saturating_*). No float, no RNG. The `team_momentum` sidecar is recomputed on event emission and decays each tick — it is NOT a pure function of canonical inputs (unlike TeamShape), so it must persist between ticks as a sidecar. The `#[serde(skip)]` is correct because the sidecar is reconstructed by replaying the canonical event stream if a state is reloaded from a save mid-match. Document this rehydration requirement in a `// INVARIANT:` comment.

#### Tuning bands

| Parameter | Value | Firmness |
|---|---|---|
| `MOMENTUM_NEUTRAL` | `Q32::from_raw(1i64 << 31)` = 0.5 | HARD (structural — the symmetry point) |
| `GOAL_IMPULSE` | `Q32::from_raw(429_496_729)` ≈ 0.10 | SOFT. A goal swings momentum by 10pp; after decay over 10 min, ~2.5pp residual. Matches intuition without dominating. |
| `SAVE_IMPULSE` | `Q32::from_raw(107_374_182)` ≈ 0.025 | SOFT. A save is a quarter of a goal's momentum swing. |
| `MOMENTUM_CAP` | `Q32::from_raw(858_993_459)` ≈ 0.20 | SOFT. Cap the bias at ±20pp from neutral. Multiple goals in quick succession shouldn't compound unboundedly. |
| `MOMENTUM_DECAY_FACTOR` per tick | `Q32::from_raw(4_294_967_252)` ≈ 1 - 1e-8/tick → half-life ~693 ticks ≈ 11.5 min | SOFT. Target: a goal impulse visible for ~10 min then faded. Iterate on contact-sheet via drama-sweep commentary-event density. |
| `MOMENTUM_SCALE` (bias magnitude) | `Q32::from_raw(214_748_365)` ≈ 0.05 | SOFT. At max momentum (0.70 = neutral + CAP), bias = 0.20 × 0.05 = 0.01 utility addend — small nudge, not a flip. |

**Worked example:** Away team scores at tick 2700 (45'). `team_momentum[1]` impulse: +0.10 → `value` = 0.60. Home drops to 0.40. Over the next 300 ticks (5 min), decay: each tick `gap` shrinks by `(1 - DECAY_FACTOR)`. At tick 3000: `gap` ≈ 0.10 × (1 - 3e-6)^300 ≈ 0.10 × 0.999 ≈ 0.099 — barely decayed. The current DECAY_FACTOR produces a very long half-life; the seed should be lowered to target ~600-tick half-life. Again, drama-sweep calibrates this.

#### Proptest invariants

1. `momentum_neutral_without_impulse` — from `MomentumState::neutral()`, after N ticks with no events, `|value - MOMENTUM_NEUTRAL| < epsilon` (decay converges back toward neutral).
2. `goal_raises_scoring_team_momentum` — immediately after a `Goal` event, the scoring team's `value > MOMENTUM_NEUTRAL` and the conceding team's `value < MOMENTUM_NEUTRAL`.
3. `momentum_bounded_by_cap` — for any sequence of events, `|value - MOMENTUM_NEUTRAL| <= MOMENTUM_CAP` always.
4. `momentum_symmetric` — if home and away receive identical impulse sequences, `team_momentum[0].value == team_momentum[1].value` at all ticks (no team-index bias in the formula).

#### Acceptance

Commentary-event density: the `narrative-director` agent will key "in the ascendancy" / "the crowd is behind them" commentary variants on `momentum_bias > THRESHOLD`. Run drama-sweep: verify that goal events cluster within the momentum-elevated window at higher-than-baseline rate (the causal direction the audit's A3 gap describes). No hard numeric target — presence of the positive correlation is the bar.

---

### FUN-LM4 — Substitutions

#### Mechanic

**Bench representation:** a new `#[serde(skip)]` `SubState` sidecar — no canonical field needed beyond the existing `PlayerState.scalars` which already holds `match_fitness`. The sub decision is a manager-policy lookup; the actual roster swap is the only mutation that touches canonical state.

**Sub triggers (checked at the heartbeat step, every 30 ticks):**

```rust
fn should_substitute(
    slot_idx: usize,
    mf: Q32,
    score_lead: i8,
    ticks_remaining: u32,
    subs_used: u8,
    params: &ArchetypeParams,
) -> bool {
    subs_used < 3          // FIFA 3-sub rule (post-T3 expandable to 5)
    && (mf < MF_SUB_THRESHOLD                        // fatigue sub
        || (score_lead < 0 && ticks_remaining < TACTICAL_SUB_WINDOW)  // chasing
        || (score_lead > 0 && ticks_remaining < PROTECT_SUB_WINDOW))  // protecting
}
```

**Canonical new field:** `subs_used: [u8; 2]` on `MatchState` (one per team). This IS canonical because it gates the substitution logic (a side that has used 3 subs cannot sub again — affects all downstream tactic decisions). **Canonical schema bump required** → authorized multi-pin rebaseline (ADR-0012 trigger #1 + #3).

**Bench:** `bench: [[Option<PlayerState>; 5]; 2]` in `MatchState` — canonical, since bench players carry `PlayerAttributes` that affect post-sub quality. Alternatively, the bench is seeded at match-init from the manager's content archetype and stored canonically. `PlayerState` is already `Serialize + Deserialize`.

**Sub execution (in `tick_match`, between step 5 and step 6):**

1. For each team, iterate outfield slots (1-10 or 12-21) in slot order.
2. Find the slot with the lowest `match_fitness` scalar where `should_substitute` is true.
3. Seeded choice of bench player: `ChaCha8Rng::seed_from_u64(seed_fn(match_seed, tick, SeedLayer::Decision, (team_idx as u64) << 32 | slot_idx as u64))` — draw one bench player index from the remaining bench (a seeded shuffle of the bench Vec, slot-order tiebreak).
4. Swap `state.players[slot_to_replace]` with the chosen bench player. The incoming player has `match_fitness = Q32::ONE` (fresh legs).
5. Emit `MatchEvent::Substitution { off_slot, on_slot, tick }` — **new discriminant 7** (append after `Offside` discriminant 6). Update `subs_used[team_idx] += 1`.
6. `apply_event(TacticEvent::Substitution, ...)` — the FSM arm for Substitution is a no-op transition (stays in current state) at this stage; the shape sidecar recomputes naturally next tick.

**SeedLayer:** `SeedLayer::Decision`, `site = (team_idx << 32) | slot_idx`. Names the bench-player draw.

**Canonical-surface impact:** new `subs_used` field + `bench` field + new `MatchEvent::Substitution` discriminant → **SCHEMA + BEHAVIORAL rebaseline**, multi-pin discipline.

#### Determinism shape

The trigger predicate is pure comparison. The bench-player selection uses one ChaCha8 draw per substitution event (not per tick). No float. The incoming player's `PlayerState` must be constructed with all required canonical fields; their `scalars` map contains `SCALAR_MATCH_FITNESS = Q32::ONE` (fresh legs). `attributes` come from the bench content data.

#### Tuning bands

| Parameter | Value | Firmness |
|---|---|---|
| `MF_SUB_THRESHOLD` | `Q32::from_raw(2_147_483_648)` ≈ 0.50 | SOFT. Sub a player when they've lost ~50% fitness. Iterate. |
| `TACTICAL_SUB_WINDOW` | 900 ticks = 15 min | SOFT. Chasing-game sub within last 15 min. |
| `PROTECT_SUB_WINDOW` | 600 ticks = 10 min | SOFT. Protection sub within last 10 min. |
| Subs per team | 3 (FIFA cap) | HARD (law). |

**Worked example:** At tick 3600 (60'), a midfielder with `match_fitness = 0.48` triggers `MF_SUB_THRESHOLD`. Score is 0-0, `subs_used[home] = 0`. `should_substitute` returns true. A bench midfielder is selected via seeded draw. The midfielder is replaced; incoming player has `match_fitness = 1.0`. The drama-sweep should show a spike in press activity from the team that just subbed — fresh-legged presser effect.

**Worked example — late tactical sub:** Home team is 1-0 up at tick 4500 (75'). `PROTECT_SUB_WINDOW = 600 ticks`, so `ticks_remaining = 900 > 600` — no protect sub yet. At tick 4800 (80'), `ticks_remaining = 600` triggers the protect window. If `subs_used[home] < 3`, the lowest-fitness outfielder is replaced with a fresh defender. Believable game-management.

#### Proptest invariants

1. `subs_used_monotone_nondecreasing` — `subs_used[t+1] >= subs_used[t]` for all ticks and both teams.
2. `max_three_subs_per_team` — `subs_used[team] <= 3` for all ticks (canonical field invariant; `assert!` in the sub execution path).
3. `incoming_player_full_fitness` — immediately after a `Substitution` event, the incoming player's `match_fitness` scalar = `Q32::ONE`.
4. `sub_only_fires_when_threshold_met` — no `Substitution` event emitted when `should_substitute` returns false for all slots.

#### Acceptance

Drama-sweep: measure press intensity (mean press-role assignments per tick) in the 30 ticks before vs 30 ticks after a substitution. Expect a measurable uplift post-sub. Verify `subs_used` accumulates correctly across a full-match run. Snapshot test a match with a seeded scenario where a sub fires at a known tick.

---

### FUN-LM5 — Half-time seam

#### Mechanic

Half-time is currently a stub — the match runs 5400 ticks as a continuous simulation. This sub-mechanic adds the half-time break as a deterministic seam at tick 2700.

**Tick 2700 detection** (in `tick_match`, before step 4 ball physics, aligned with the existing `match_end_tick` halt pattern):

```rust
let half_time_tick = Tick::from_raw(FULL_MATCH_TICKS as i64 / 2); // = 2700

if state.tick == half_time_tick && !state.is_second_half {
    // 1. Emit MatchEvent::HalfTime { tick: state.tick } — new discriminant 8.
    // 2. Apply micro stamina recovery to all players.
    // 3. Set state.is_second_half = true.
    // 4. Reset both teams to MidBlock FSM state.
    // 5. Flip attack direction (negate all player pos_x and vel_x, flip possession to away CF kickoff).
    // 6. Skip the rest of the tick_match loop body (return early after emit).
}
```

**New canonical field:** `is_second_half: bool` on `MatchState`. Schema bump (trigger #1).

**New `MatchEvent` discriminant 8:** `MatchEvent::HalfTime { tick: Tick }`.

**Micro stamina recovery** (applied to all 22 players at tick 2700):
```
new_mf = min(Q32::ONE, current_mf + HALF_TIME_RECOVERY)
```

No RNG. Deterministic application to all players in slot order.

**Attack direction flip:** home team attacks toward positive x in the first half; swap to negative x in the second half. Players' `pos_x` is negated; `vel_x` is negated. The `zonal_slot` helper in `team_shape` must account for `attack_dir` already (it does via the `attack_dir` parameter per `tactical-shape.md:96`). The canonical encoding is affected — flip must happen in a single atomic step within `tick_match` before the next canonical encode.

**SeedLayer:** none — micro recovery and direction flip are deterministic arithmetic.

**Canonical-surface impact:** new `is_second_half` bool + new `HalfTime` discriminant + new `bench` init for second-half subs + position negations → **SCHEMA + BEHAVIORAL rebaseline** (multi-pin).

#### Tuning bands

| Parameter | Value | Firmness |
|---|---|---|
| `HALF_TIME_RECOVERY` | `Q32::from_raw(429_496_729)` ≈ 0.10 | **SOFT.** A 15-minute break restores ~10% match fitness. Real football: players recover significantly in the interval; FM-style observations suggest ~10-15% recovery is realistic. HARD direction: recovery > 0. |
| Half-time tick | 2700 | **HARD.** 45 minutes × 60 ticks/minute = 2700. Non-negotiable — the constant `FULL_MATCH_TICKS / 2`. |

**Worked example:** A forward with `match_fitness = 0.62` at tick 2700 receives HALF_TIME_RECOVERY of 0.10 → `new_mf = min(1.0, 0.72)` = 0.72. A fully fresh GK with `match_fitness = 0.97` → `new_mf = min(1.0, 1.07)` = 1.0 (clamped). Correct behaviour in both cases.

#### Proptest invariants

1. `half_time_emitted_exactly_once` — across a full 5400-tick match, exactly one `MatchEvent::HalfTime` is present in `match_events`.
2. `match_fitness_increases_at_half_time` — for all players, `mf[2701] >= mf[2700]` (recovery applied).
3. `is_second_half_monotone` — `is_second_half` is false for ticks 0-2699 and true for ticks 2700-5400 (never reverts).
4. `possession_flips_at_half_time` — at tick 2700, the losing team's CF (or the lower-score team's CF on draw) has possession (the conventional second-half kickoff rule; home-first on a draw matching the `MatchState::initial` convention).

#### Acceptance

- Insta snapshot: run a 2700-tick + 1-tick match (tick 2701 only); verify `is_second_half = true`, `HalfTime` event present, all player `match_fitness` scalars increased by exactly `HALF_TIME_RECOVERY` (clamped to 1.0).
- Drama-sweep: verify mean `match_fitness` across all players at tick 2701 is higher than at tick 2699 by approximately `HALF_TIME_RECOVERY`.

---

## Cross-cutting concerns

**Rebaseline cascade:** FUN-LM1 introduces a canonical schema bump (new scalar key in `scalars`). FUN-LM4 introduces `subs_used`, `bench`, and new `MatchEvent` discriminants 7 and 8 (counting `HalfTime`). FUN-LM5 introduces `is_second_half` and `HalfTime` discriminant. The recommended implementation order is **LM5 → LM2 → LM3 → LM1 → LM4** to minimize rebaseline count: LM5 is the smallest canonical change and unblocks the half-time seam that LM1/LM4 build on. LM2 and LM3 are behavioural-only once LM5 exists. LM1 and LM4 are the largest canonical bumps and should be a single rebaseline.

**Dependency on FUN-TS2:** FUN-LM1's `activity_load` reads `press_role` from the FUN-TS2 `PressPlan` sidecar. If FUN-TS2 is not yet shipped, substitute: classify PRESS as `tactic_state == HighPress AND distance_to_ball < PRESS_ENGAGEMENT_RADIUS`. Less precise but deterministic and independent.

**Dependency on T4.5-E0 (gene compiler) + FUN-DR:** fatigue differentiation and sub quality are invisible until players have differentiated attributes. FUN-LM1-5 are coded against the 55-field attribute model that already exists; the mechanics will show their full effect only once FUN-DR retires the mirror-team substrate.

**Dependency on FUN-TS4 FSM integration:** FUN-LM2's FSM transitions should layer on top of the `archetype_params` already threaded through `apply_event`. The score+clock arm reads `params.default_in_defence_state` to bias the transition target (a high-press archetype forced into LowBlock when 2-0 up should be less willing than a counter-attack archetype — wire the archetype's `LowBlock` preference through the existing `ArchetypeParams` struct rather than hardcoding the transition).

**Files the gameplay-programmer will touch:**
- `crates/fw-match-sim/src/lib.rs` — tick_match step-5 seam (heartbeat extensions), half-time detection, FUN-LM4 sub execution, FUN-LM5 direction flip
- `crates/fw-match-sim/src/tactic_fsm.rs` — `score_clock_check` function, new `Substitution` TacticEvent arm
- `crates/fw-match-sim/src/team_shape.rs` — `MomentumState` struct, momentum decay per-tick
- `crates/fw-match-sim/src/dispatch.rs` — `BtContext` gains `fatigue_factor: Q32` + `momentum_bias: Q32` parameters; `select_outfield_intent` applies them
- `crates/fw-content/src/event.rs` — append `Substitution` (discriminant 7) and `HalfTime` (discriminant 8); update discriminant table + `MatchEventDiscriminant` enum + `encode_match_event` in `canonical.rs`
- `crates/fw-match-sim/src/canonical.rs` — encode new canonical fields (`subs_used`, `bench`, `is_second_half`, scalar key initialisation)
- `crates/fw-core/src/player_attributes.rs` — add `pub const SCALAR_MATCH_FITNESS: u16 = 0x0010;` (or the equivalent location)

**Invariant style:** all canonical invariants (`subs_used <= 3`, `mf >= MF_FLOOR`, `is_second_half monotone`) use `assert!` not `debug_assert!` per Sim/RULES.md §11.


---

# Contested-Ball cluster (FUN-CB1-3: passes-can-fail, dribble-1v1, aerial+first-touch)

I have sufficient context now. Here is the implementation-ready spec:

---

## Contested-Ball Cluster — FUN-CB1 / FUN-CB2 / FUN-CB3

**Audit IDs:** A5 (passes can fail), A9 (dribble as 1v1), A10 (aerial / cross / first-touch).
**Canonical-surface class:** three new `MatchEvent` variants (discriminants 7/8/9) → ADR-0012 authorized multi-pin rebaseline required. No new canonical fields on `MatchState` beyond the events vec. No new `#[serde(skip)]` sidecars needed.
**File anchors:** `crates/fw-content/src/event.rs` (new variants), `crates/fw-match-sim/src/dispatch.rs` (`apply_intent` — the sole mutation site for all three), `crates/fw-match-sim/src/lib.rs` (`resolve_tackles` — CB2 hooks here in parallel).

---

## FUN-CB1 — Passes Can Fail

### Mechanic

The stub `T1_PASS_COMPLETED: bool = true` (dispatch.rs:877) is retired. Every pass-class intent — `AttemptPassShort`, `AttemptPassLong`, `Cross`, `LayOff` — runs a completion draw inside `apply_intent`, immediately after `to_slot` is resolved via `nearest_teammate_near` and before ball velocity is set.

**Step-by-step (inside `apply_intent`, each pass arm):**

1. Compute `lane_openness`: call `pitch_control()` (utility/pitch_control.rs — already built, zero call sites today) at the midpoint between passer and `to_slot`. Pass passer's team as `attackers`, opponent team as `defenders`. Use `PlayerSnapshot { pos: player.pos_x/y, vel: player.vel_x/y, v_max: MAX_PLAYER_SPEED }` constructed inline from `state.players`. The result's `defender_control` is the lane-crowding scalar; `lane_openness = Q32::ONE - outcome.defender_control`.

2. Compute `passer_quality`: attribute composite varied by pass kind.
   - Short/LayOff: `passing × 0.55 + technique × 0.25 + first_touch × 0.20`
   - Long: `passing × 0.50 + vision × 0.30 + long_shots × 0.20`
   - Cross: `crossing × 0.55 + technique × 0.25 + vision × 0.20`

3. Compute `receiver_pressure`: call `pitch_control()` at `to_slot`'s position. `receiver_pressure = opponent_control` at that point (same construction, attackers = passer's team, defenders = opponent).

4. Completion probability:
   ```
   p_complete = P_BASE[kind] × passer_quality × lane_openness × (Q32::ONE - RECV_PRESSURE_WEIGHT × receiver_pressure)
   ```
   Clamp to `[P_FLOOR, Q32::ONE]`.

5. Roll `ChaCha8Rng` seeded via `seed_fn(match_seed, tick, SeedLayer::Decision, (from_slot as u32) << 16 | 0xCB01)`. Draw `next_u64()`, upper 32 bits → Q32 in `[0,1)`.

6. On **success** (draw < p_complete): existing ball-velocity + possession logic unchanged. `Pass.completed = true`.

7. On **failure**: emit `Pass { completed: false, ... }` then:
   - If it was a forward pass: spawn turnover. Set `state.possession = None`, snap ball to a point 40% of the way from passer to receiver (loose-ball halfway dropped in the lane), `state.ball.vel_x/y` = zero (ball stops mid-pitch for the loose-ball chase). `state.last_touched_by = Some(from_slot)`.
   - If it was a backward/lateral pass (LayOff, or short pass where `to_slot.pos_x` is behind passer): same loose-ball drop but at 20% of the way (miscontrol closer to feet).
   - Emit new `MatchEvent::PassIncomplete` (discriminant 7) — see below.

"Forward" is determined by sign: home passer forward if `to_slot.pos_x > passer.pos_x`; away passer forward if `to_slot.pos_x < passer.pos_x`.

**Tick-loop seam:** `apply_intent` in `dispatch.rs`, inside each of the four pass arms, replacing the `T1_PASS_COMPLETED` constant. No new step in `tick_match`'s step order; this is pure mutation inside step 6's dispatch.

### Determinism Shape

All Q32. The `pitch_control()` function is already Q32-clean (no floats in canonical paths — the test helper `fn q(v: f64)` is test-only). The two `pitch_control()` calls per pass are pure functions of player positions (canonical) and return `PitchControlOutcome` (not stored — consumed inline). No new canonical fields; the only canonical output is the `Pass { completed: bool }` field (already on the struct) and the new `PassIncomplete` event variant.

SeedLayer for the completion roll: `SeedLayer::Decision` (binary decision draw, matching the table in ADR-0009). Site: `(from_slot as u32) << 16 | 0xCB01` — distinct from the tackle site `0x7AC1` (dispatch.rs:1380) and the shot-dispersion sites `0x0001..0x0003`.

Rebaseline class: **new MatchEvent variant** + behaviour change on every pass tick → ADR-0012 trigger #3 (documented sim-behaviour change). Multi-pin rebaseline authorized per the CB cluster task.

### Tuning Bands

Targeting the pass completion ordering from match-realism-reference §2 (HARD — ordering is load-bearing):

| Kind | Target completion % | Firmness | `P_BASE` seed |
|---|---|---|---|
| Backward (LayOff) | ~92–96% | HARD ordering | `P_BASE = 0.97` (raw `4_167_849_984`) |
| Short | ~85–90% | HARD ordering | `P_BASE = 0.90` (raw `3_865_470_565`) |
| Long | ~70–78% | HARD ordering | `P_BASE = 0.75` (raw `3_221_225_472`) |
| Cross | ~60–68% | SOFT (style-dependent) | `P_BASE = 0.65` (raw `2_791_728_742`) |

`RECV_PRESSURE_WEIGHT = 0.35` (raw `1_503_238_553`) — receiver under heavy pressure (defender_control ≈ 0.8) reduces p_complete by ~28pp; that's the mechanism that makes forward passes into a press fail more than backward recycling.

`P_FLOOR = 0.15` (raw `644_245_094`) — a world-class passer under maximum pressure in a crowded lane still completes 15% of the time. Prevents 0% completion absurdities.

At mirror-team baseline (all attributes 0.5, balanced pitch control ~0.5/0.5):
- Short: `0.90 × 0.69 × 0.5 × (1 - 0.35×0.5) ≈ 0.90 × 0.69 × 0.5 × 0.825 ≈ 0.256`... that's too low. Recalibrate: `passer_quality` at 0.5-attribute mid-baseline ≈ 0.5 (linear composite), so the product must not triple-penalise. Correct: apply `lane_openness` additively via `(1 + 0.50 × lane_openness)` modifier rather than multiplicatively:

Revised formula (SOFT, iterate via drama-sweep):
```
p_complete = clamp(P_BASE[kind] × passer_quality × (1 - RECV_PRESSURE_WEIGHT × receiver_pressure), P_FLOOR, 1.0)
```
Drop the multiplicative `lane_openness` from the base formula; instead use lane_openness as an additive bonus: when `lane_openness > 0.6`, multiply p_complete by `1 + 0.20 × (lane_openness - 0.6)` (reward clean space). This keeps mid-baseline short passes at `0.90 × 0.5 × (1 - 0.35×0.5) ≈ 0.90 × 0.5 × 0.825 ≈ 0.371`... still low vs the 85–90% target.

**Key insight:** `passer_quality` at 0.5 attributes is `~0.50`; the formula needs to map 0.5 attributes to ~0.90–0.95 completion for a clean short pass. The correct structure is:

```
p_complete = P_BASE[kind] × (1 - QUALITY_REDUCTION × (1 - passer_quality)) × (1 - RECV_PRESSURE_WEIGHT × receiver_pressure)
```

Where `QUALITY_REDUCTION = 0.40` — a poor passer (quality=0) reduces P_BASE by 40%, a world-class passer (quality=1) doesn't reduce it at all. At mid-baseline (quality=0.5, pressure=0.5):
- Short: `0.90 × (1 - 0.40×0.5) × (1 - 0.35×0.5) = 0.90 × 0.80 × 0.825 ≈ 0.594`. Still low.

The correct resolution: `P_BASE` must already embed the "average player, average conditions" target, so `P_BASE[Short] = 0.90` is the RESULT at mid-baseline, not the unconstrained ceiling. Reframe as:

**Final formula (SOFT — iterate from here):**
```
p_complete = clamp(
  P_BASE[kind] × passer_quality_modifier × lane_pressure_modifier,
  P_FLOOR[kind], 1.0)

passer_quality_modifier = lerp(LOW_MOD, 1.0, passer_quality)
  where LOW_MOD = 0.70  // weakest passer → 70% of P_BASE

lane_pressure_modifier = Q32::ONE - RECV_PRESSURE_WEIGHT × receiver_pressure
  where RECV_PRESSURE_WEIGHT = 0.25
```

At mid-baseline short: `0.90 × lerp(0.70, 1.0, 0.5) × (1 - 0.25×0.5) = 0.90 × 0.85 × 0.875 ≈ 0.670`. That's still below target.

The right recognition: `pitch_control` gives `defender_control ≈ 0.50` at exact balance — but a short ground pass to a nearby teammate will have LOWER defender_control at the midpoint (they're close; the passer's team arrives faster). Expect `receiver_pressure ≈ 0.25–0.35` in normal play for a short pass; `receiver_pressure ≈ 0.55–0.70` for a pressed receiver. So at mid-baseline short, `lane_pressure_modifier ≈ 1 - 0.25×0.30 ≈ 0.925`. Revised: `0.90 × 0.85 × 0.925 ≈ 0.708`. Tag as SOFT — the drama-sweep loop will converge these to the HARD ordering anchors within 2–3 runs.

**Phase-1 tuning seeds (SOFT — drama-sweep targets, not pins):**

| Dial | Value | Raw bits |
|---|---|---|
| `P_BASE_SHORT` | 0.90 | `3_865_470_566` |
| `P_BASE_LONG` | 0.75 | `3_221_225_472` |
| `P_BASE_CROSS` | 0.65 | `2_791_728_742` |
| `P_BASE_LAYOFF` | 0.95 | `4_080_218_931` |
| `LOW_MOD` | 0.70 | `3_006_477_107` |
| `RECV_PRESSURE_WEIGHT` | 0.25 | `1_073_741_824` |
| `P_FLOOR_SHORT` | 0.35 | `1_503_238_553` |
| `P_FLOOR_LONG` | 0.20 | `858_993_459` |
| `P_FLOOR_CROSS` | 0.15 | `644_245_094` |
| `P_FLOOR_LAYOFF` | 0.50 | `2_147_483_648` |

The ordering constraint (LayOff > Short > Long > Cross) is the HARD gate, not the absolute percentages. Tune `P_BASE` values until N=1000 mirror-team sim produces observed completion rates in those bands.

### Proptest Invariants

1. **Ordering preserved:** over N=500 seeded matches, mean(short_completion_rate) > mean(long_completion_rate) > mean(cross_completion_rate) with p=1.0 (ordering is mechanical, not statistical).
2. **Failed passes emit loose ball:** for every `MatchEvent::PassIncomplete`, `state.possession == None` at the tick the event fires and `state.last_touched_by == Some(from_slot)`.
3. **Overall completion in band:** N=1000 matches, mean(all-pass completion rate) ∈ [0.78, 0.90] at mirror-team baseline (match-realism-reference §2 anchor: HARD 83–86%).
4. **P_FLOOR respected:** no seeded scenario produces p_complete < P_FLOOR for any pass kind — verify by checking `Pass { completed: false }` events against a theoretically-maximal-pressure fixture.

### Acceptance

Drama-sweep against the pass-completion ordering after implementation. Pass if:
- backward/layoff completion > short completion > long completion > cross completion (ordering) across 200 matches
- Overall mean completion ∈ [0.78, 0.90] at mirror baseline
- Failed forward passes generate turnovers that appear as `PassIncomplete` events with `state.possession = None` in the event log (contact-sheet-readable)

### Open Questions / Dependencies

- FUN-TS3 (midfield build-up) will add real pass-target geometry (not `nearest_teammate_near`) — the completion draw plugs in unchanged; only `to_slot` resolution improves.
- `PlayerCondition.match_fitness` (fatigue, A1) is a future multiplier on `passer_quality_modifier` — the formula is factored to accept it: `passer_quality_modifier × fatigue_factor`.
- At mirror baseline (all 0.5 attributes) the pitch_control taus will be symmetric — completion rates won't differentiate by team quality until FUN-DR + T4.5-E0 land. This is expected and correct.

---

## FUN-CB2 — Dribble as 1v1 Contest

### Mechanic

Current `Dribble` intent in `apply_intent` (dispatch.rs:1100-approx) snaps ball to feet and advances 8m — no contest. This slice adds a defender-proximity check that intercepts the Dribble arm BEFORE the advance resolves.

**Step-by-step (inside `apply_intent`, `PlayerIntent::Dribble` arm):**

1. Before advancing the ball: scan all opposing-team players within `DRIBBLE_CONTEST_RADIUS = 3.0m` (raw `12_884_901_888`). Iterate in slot order (0..22); pick the FIRST opponent slot within radius (closest-first sorting would require a sort — instead use first-in-slot-order as the tiebreak, matching `resolve_tackles` discipline at lib.rs:1450).

2. If no opponent in radius: existing behaviour unchanged (ball snaps to feet, vel zeroed, advance target set). No event emitted; not a contest.

3. If opponent found (`defender_slot`): resolve the 1v1 contest.

   **Contest quality scores (Q32, in [0,1]):**
   ```
   attacker_q = dribbling × 0.45 + agility × 0.30 + flair × 0.15 + technique × 0.10
   defender_q = marking × 0.40 + positioning × 0.35 + anticipation × 0.15 + strength × 0.10
   ```

   **Outcome probability:**
   ```
   p_beat = DRIBBLE_BASE_BEAT × attacker_q / (attacker_q + defender_q)
   p_stall = DRIBBLE_BASE_STALL (flat — no momentum; the dribbler retains possession but doesn't advance)
   p_dispossessed = Q32::ONE - p_beat - p_stall  (absorbs the remainder)
   ```
   Clamp so all three are non-negative; p_beat + p_stall ≤ 1.

4. Roll `ChaCha8Rng` seeded via `seed_fn(match_seed, tick, SeedLayer::ReactiveInterrupt, (dribbler_slot as u32) << 16 | 0xCB02)`. Draw `next_u64()` → Q32 unit scalar `r`.

5. **Outcome branches:**
   - `r < p_beat` → **Clean beat.** Advance the ball 8m toward the Dribble target (existing logic). Emit `MatchEvent::DribbleBeat { dribbler_slot, defender_slot, tick }` (discriminant 8). Set `tackle_cooldown_until[defender_slot_idx] = tick + DRIBBLE_BEAT_COOLDOWN`.
   - `r < p_beat + p_stall` → **Stall.** Snap ball to dribbler's feet; vel zeroed; do NOT advance. No MatchEvent (internal state — commentary handles this via possession-unchanged observation). Defender not cooled down.
   - `r >= p_beat + p_stall` → **Dispossessed.** Set `state.possession = None`. Snap ball to the midpoint between dribbler and defender (contested loose ball). `state.last_touched_by = Some(dribbler_slot)`. Emit `MatchEvent::Dispossessed { dispossessed_slot, defender_slot, tick }` (discriminant 9). Set `tackle_cooldown_until[defender_slot_idx] = tick + TACKLE_COOLDOWN` (same value as existing tackle system).

**Tick-loop seam:** `apply_intent` inside the `PlayerIntent::Dribble` arm. No new tick-match step. The existing `resolve_tackles` in `lib.rs` runs AFTER `dispatch_tick` (step 6); CB2 runs INSIDE step 6 during `apply_intent`. This is correct — the dribble decision and its immediate 1v1 resolution are co-tick.

Note: `tackle_cooldown_until` is already indexed by `def_idx` (the slot index 0..22), not a BTreeMap — no HashMap introduced.

### Determinism Shape

Q32 throughout. SeedLayer: `SeedLayer::ReactiveInterrupt` (same as `resolve_tackles` — a reactive contest triggered by proximity, not a scheduled decision). Site `0xCB02` is distinct from the tackle site `0x7AC1`.

`DribbleBeat` and `Dispossessed` are new `MatchEvent` variants in canonical state (`match_events: Vec<MatchEvent>`). This triggers ADR-0012 rebaseline (same authorized cluster as CB1/CB3).

The defender scan iterates 0..22 in fixed slot order — deterministic; the first slot within radius wins (same tiebreak policy as `resolve_tackles` which uses "first successful defender in slot order").

### Tuning Bands

Real football: a professional dribbler in 1v1 completes (beats defender OR retains) roughly 50–65% of attempted dribbles in open play; outright dispossession ~35–50% (varies heavily by player type). This is SOFT — no single-source aggregate number is stable.

| Dial | Seed | Firmness | Notes |
|---|---|---|---|
| `DRIBBLE_CONTEST_RADIUS` | 3.0m (raw `12_884_901_888`) | SOFT | Slightly wider than the tackle radius (implicit ~2m). Triggers earlier — the defender doesn't need to be touching the ball. |
| `DRIBBLE_BASE_BEAT` | 0.55 (raw `2_362_232_012`) | SOFT | At equal quality (q_att = q_def = 0.5): p_beat = 0.55 × 0.5 / (0.5+0.5) = 0.275. |
| `DRIBBLE_BASE_STALL` | 0.20 (raw `858_993_459`) | SOFT | Stall: dribbler holds, but doesn't advance; no event. |
| Implied p_dispossessed at equal quality | 1 - 0.275 - 0.20 = 0.525 | SOFT | Mid-quality wingers are dispossessed slightly more often than they beat their man. Adjust by raising DRIBBLE_BASE_BEAT if contact-sheet feels too turnover-heavy. |
| `DRIBBLE_BEAT_COOLDOWN` | 45 ticks (0.75s) | SOFT | A beaten defender needs ~1s to recover; slightly shorter than they'd realistically need because the game sim runs at 60 Hz discrete ticks, not continuous. |

**Worked example — Dribbler A (all attrs 0.70) vs Defender B (all attrs 0.55):**
- `attacker_q = 0.70×0.45 + 0.70×0.30 + 0.70×0.15 + 0.70×0.10 = 0.70`
- `defender_q = 0.55×0.40 + 0.55×0.35 + 0.55×0.15 + 0.55×0.10 = 0.55`
- `p_beat = 0.55 × 0.70 / (0.70 + 0.55) = 0.55 × 0.56 = 0.308`
- `p_stall = 0.20`
- `p_dispossessed = 0.492`
- If `r = 0.25`: beat → `DribbleBeat` emitted. If `r = 0.45`: stall. If `r = 0.70`: dispossessed → loose ball.

**Worked example — Elite dribbler C (dribbling=0.90, agility=0.85, flair=0.80, technique=0.75) vs mid defender (marking=0.55, positioning=0.55, anticipation=0.50, strength=0.50):**
- `attacker_q = 0.90×0.45 + 0.85×0.30 + 0.80×0.15 + 0.75×0.10 = 0.405 + 0.255 + 0.12 + 0.075 = 0.855`
- `defender_q = 0.55×0.40 + 0.55×0.35 + 0.50×0.15 + 0.50×0.10 = 0.22 + 0.1925 + 0.075 + 0.05 = 0.5375`
- `p_beat = 0.55 × 0.855 / (0.855+0.5375) = 0.55 × 0.614 = 0.338`
- `p_stall = 0.20`, `p_dispossessed = 0.462`
- Elite drbblers are still dispossessed significantly — the formula correctly rewards quality without guaranteeing success.

### Proptest Invariants

1. **No contest outside radius:** when no opponent is within `DRIBBLE_CONTEST_RADIUS`, the existing advance logic fires with no `DribbleBeat` or `Dispossessed` event.
2. **Loose-ball invariant on dispossession:** after a `Dispossessed` event, `state.possession == None` and `state.last_touched_by == Some(dribbler_slot)` at that tick.
3. **Cooldown respected:** the slot index of `defender_slot` is cooled down after a beat (`tackle_cooldown_until[def_idx] > tick`); the same defender cannot trigger a contest on the immediately following tick.
4. **Probability bounds:** `p_beat + p_stall` is always ≤ `Q32::ONE` regardless of attribute inputs (assert in non-test code, not `debug_assert!`).

### Acceptance

Contact-sheet readable: a 1000-tick match with a winger (high dribbling/agility) vs a fullback (high marking/positioning) should show a visually spread pattern of Beat / Stall / Dispossession events in the `match_events` Vec. Drama-sweep: confirm `Dispossessed` events produce the loose-ball-chase preemption (outfield nearest-2 chase) within 1–3 ticks. Beat events should trail into a visible gap in defender coverage before re-closing.

### Open Questions / Dependencies

- FUN-CB2 uses `tackle_cooldown_until` which is already indexed by slot (0..22) in `MatchState`. Confirm it is `[Tick; 22]` not a Map (if it is, no change needed; if it is a Vec, same semantics).
- The `flair` attribute feeds CB2's attacker quality; it is also in the existing `utility_dribble` bias (`mental.flair`). This is intentional — flair biases the decision to dribble AND the outcome quality when challenged. The dual role is documented.
- FUN-CB2 is independent of FUN-TS2 press roles (press targets the carrier with normal `Mark`/`Press` BT intents; CB2 triggers when a dribbler's 8m advance brings them within 3m of a defender who is already proximate). They can coexist without gating.

---

## FUN-CB3 — Aerial Duels, Contested Crosses, First-Touch Failure

### Mechanic

Three sub-resolutions, all wired inside `apply_intent`. The attributes `heading`, `jumping_reach`, and `strength` (physical) are currently dead in all sim paths — this slice wires them.

#### Sub-resolution A: First-Touch Failure on Pass Receipt

When a `Pass { completed: true }` results in possession transfer (`state.possession = Some(to_slot)`): before finalizing possession, check if the receiver is under pressure and likely to miscontrol.

1. Compute `recv_pressure_q32` via `pitch_control()` at `to_slot`'s position (same as CB1 uses for `receiver_pressure`, so this can be the cached value from CB1 if both are implemented together; otherwise compute it independently — same call, same Q32 result).

2. Compute `first_touch_quality = first_touch × 0.60 + composure × 0.25 + technique × 0.15` for the receiver.

3. First-touch failure probability:
   ```
   p_miscontrol = MISCONTROL_BASE × (Q32::ONE - first_touch_quality) × recv_pressure_q32
   ```

4. Roll `SeedLayer::ReactiveInterrupt`, site `(to_slot as u32) << 16 | 0xCB03`.

5. On **failure** (draw < p_miscontrol): set `state.possession = None`. Ball stays at `to_slot`'s position (the miscontrol scatters it at feet). `state.last_touched_by = Some(to_slot)`. Emit `MatchEvent::FirstTouchFailure { player_slot: to_slot, tick }` (discriminant 10).

6. On **success**: possession completes normally. No new event.

#### Sub-resolution B: Aerial Duel on High Balls (Long Pass and Cross)

When intent is `AttemptPassLong` or `Cross` (the two pass kinds where a high-ball contest is realistic): after `to_slot` is resolved and before the completion draw, scan for the nearest opposing-team player to `to_slot` within `AERIAL_CONTEST_RADIUS = 4.5m` (raw `19_327_352_832`).

If an aerial contest opponent (`aerial_opp_slot`) is found:

1. **Attacker aerial quality:**
   ```
   att_aerial = heading × 0.40 + jumping_reach × 0.35 + strength × 0.15 + anticipation × 0.10
   ```

2. **Defender aerial quality:**
   ```
   def_aerial = heading × 0.40 + jumping_reach × 0.35 + strength × 0.15 + positioning × 0.10
   ```

3. Contest outcome:
   ```
   p_att_wins = att_aerial / (att_aerial + def_aerial)
   ```

4. Roll `SeedLayer::ReactiveInterrupt`, site `(to_slot as u32) << 16 | 0xCB04`.

5. On attacker win (r < p_att_wins): normal completion proceeds (already contested — treat as aerial controlled). Emit `MatchEvent::AerialDuelWon { winner_slot: to_slot, loser_slot: aerial_opp_slot, tick }` (discriminant 11).

6. On defender win (r >= p_att_wins): the pass is intercepted aerially. `state.possession = None`. Ball snaps to midpoint of `to_slot` and `aerial_opp_slot` positions. `state.last_touched_by = Some(to_slot)`. The existing completion roll (CB1) is SKIPPED on aerial duels — the aerial contest supersedes it. Emit `MatchEvent::AerialDuelWon { winner_slot: aerial_opp_slot, loser_slot: to_slot, tick }`.

**Ordering within `apply_intent` for Long/Cross arms:** (1) aerial check, (2) if no aerial contest: CB1 completion draw, (3) ball velocity set.

#### Sub-resolution C: Cross Into Box → Contested Delivery

A cross (`PlayerIntent::Cross`) that is NOT caught by an aerial duel still resolves via CB1 completion draw with a lower `P_BASE` (the `P_BASE_CROSS = 0.65` seed from CB1 already encodes this). The new behaviour is: a completed cross that reaches `to_slot` still triggers the first-touch failure check (Sub-resolution A applies), AND if the crossing target (`to_slot`) has low `heading` but the cross is a high ball (proxied by whether `to_slot.pos_x` is within the box, i.e., within 16.5m of the goal line), apply a `heading_penalty` to first_touch_quality:

```
effective_first_touch = first_touch_quality × (1 - HEADING_CROSS_WEIGHT × (Q32::ONE - heading))
```

`HEADING_CROSS_WEIGHT = 0.35` (raw `1_503_238_553`). A striker with heading=0.2 who receives a cross has their first_touch_quality penalised; a striker with heading=0.85 is roughly unaffected. This wires `heading` to cross delivery without a full separate aerial resolution for in-box crosses.

### Determinism Shape

All Q32. Three new SeedLayer sites, all `SeedLayer::ReactiveInterrupt` — reactive contests triggered by proximity, not scheduled decisions. Sites:
- `0xCB03` — first-touch failure
- `0xCB04` — aerial duel outcome

The five new `MatchEvent` variants (discriminants 7–11) are all in canonical state. `event.rs` discriminant table must be extended:

| Discriminant | Variant | Notes |
|---|---|---|
| 7 | `PassIncomplete { from_slot, to_slot, tick, kind }` | Replaces the `completed: false` bool with an explicit event for commentary routing |
| 8 | `DribbleBeat { dribbler_slot, defender_slot, tick }` | Clean dribble-past event |
| 9 | `Dispossessed { dispossessed_slot, defender_slot, tick }` | Loss of possession in a dribble contest |
| 10 | `FirstTouchFailure { player_slot, tick }` | Miscontrol on receipt under pressure |
| 11 | `AerialDuelWon { winner_slot, loser_slot, tick }` | High-ball contest result |

All are `Serialize + Deserialize`, no `f32/f64`, `PlayerSlot` and `Tick` from `fw-core`. The `MatchEventDiscriminant::all()` array extends to 12. The cross-crate test in `fw-content/tests/event_discriminant_test.rs` must be updated to pin all 12.

`MatchEvent` size: with 5 new variants, the existing `sz <= 64` assert may need revision — `AerialDuelWon` has two `PlayerSlot` (u8) + `Tick` (i64-backed) = ~10 bytes + discriminant. `DribbleBeat` and `Dispossessed` are the same shape. None of these exceed the current size bound (the largest variant is `SignatureFirstFired` at ~33 bytes). Assert holds.

Rebaseline class: new variant additions to `MatchEvent` + new behaviour in `apply_intent` → ADR-0012 trigger #3 (documented sim-behaviour change). Authorized per the CB cluster. Multi-pin: pin CB1, CB2, CB3 as a single rebaseline commit with the full cluster spec cited.

### Tuning Bands

**First-touch failure (Sub-A):**

| Dial | Seed | Firmness |
|---|---|---|
| `MISCONTROL_BASE` | 0.22 (raw `945_280_512`) | SOFT |
| Effective miscontrol at mid-baseline (first_touch=0.5, pressure=0.5) | `0.22 × 0.5 × 0.5 = 0.055` (~5.5%) | SOFT target |
| Effective miscontrol under heavy press (first_touch=0.3, pressure=0.75) | `0.22 × 0.7 × 0.75 = 0.116` (~12%) | SOFT — occasional miscontrol under pressure |

Real football: elite first touch rarely fails even under heavy pressure (~2–5%); a average player under press fails noticeably but not constantly (~8–15%). The mid-baseline 5.5% is intentionally conservative — it adds texture without overwhelming.

**Aerial duel (Sub-B):**

| Dial | Seed | Firmness |
|---|---|---|
| `AERIAL_CONTEST_RADIUS` | 4.5m (raw `19_327_352_832`) | SOFT |
| At equal aerial quality | p_att_wins = 0.50 | HARD (by construction of the formula) |
| Tall striker (heading=0.80, jumping=0.80, strength=0.70, anticipation=0.60) vs mid CB (heading=0.65, jumping=0.65, strength=0.70, positioning=0.65) | att=0.80×0.40+0.80×0.35+0.70×0.15+0.60×0.10=0.32+0.28+0.105+0.06=0.765; def=0.65×0.40+0.65×0.35+0.70×0.15+0.65×0.10=0.26+0.2275+0.105+0.065=0.6575; p_att_wins=0.765/(0.765+0.6575)=0.538 | SOFT — tall striker wins ~54% of aerial duels vs solid CB |
| Winger (heading=0.30) vs CB (heading=0.75) | att≈0.30×0.40+0.60×0.35+0.55×0.15+0.50×0.10=0.12+0.21+0.0825+0.05=0.4625; def≈0.75×0.40+0.72×0.35+0.65×0.15+0.65×0.10=0.30+0.252+0.0975+0.065=0.7145; p_att_wins=0.393 | SOFT — wingers lose most aerial duels vs CBs (correct) |

**Cross heading penalty (Sub-C):**

| Dial | Seed | Firmness |
|---|---|---|
| `HEADING_CROSS_WEIGHT` | 0.35 (raw `1_503_238_553`) | SOFT |
| Striker heading=0.80 | penalty: `1 - 0.35×0.20 = 0.93` first_touch multiplier | SOFT |
| Striker heading=0.20 | penalty: `1 - 0.35×0.80 = 0.72` first_touch multiplier | SOFT — 28% reduction in effective first touch; noticeably more miscotrol from crosses |

### Proptest Invariants

1. **Aerial supersedes completion:** when an `AerialDuelWon { winner_slot: def }` fires on a Long/Cross, `state.possession == None` (loose ball) at that tick. No `PassIncomplete` event co-fires for the same pass (the aerial resolution is mutually exclusive with the CB1 completion draw).
2. **First-touch failure → loose ball:** every `FirstTouchFailure` event is followed by `state.possession == None` and `state.last_touched_by == Some(player_slot)`.
3. **Heading wires to aerial:** for otherwise-identical players, the one with higher `heading + jumping_reach` wins aerial contests strictly more often across N=1000 seeded draws (monotone in the composite score).
4. **Heading cross penalty is bounded:** `effective_first_touch ∈ [Q32::ZERO, Q32::ONE]` for all valid attribute inputs (assert in non-test code, not `debug_assert!`).

### Acceptance

- `cargo test --workspace` with pinned canonical-hash updated for the CB cluster.
- Drama-sweep: cross-into-box sequences that previously teleported possession now show a realistic mix of `AerialDuelWon`, `FirstTouchFailure`, and controlled receipt in the `match_events` stream. Target: ~20–35% of box crosses result in an aerial duel or first-touch failure at mirror-team baseline (SOFT — iterate).
- Contact-sheet: a sequence of 200 ticks with an active winger shows `DribbleBeat` and `Dispossessed` events in the log at a rate consistent with 1v1 encounter frequency (~1 per 60–90 ticks when a winger is in possession).
- `fw-content/tests/event_discriminant_test.rs` updated to pin all 12 discriminants.

### Open Questions / Dependencies

- **FUN-TS2 / FUN-TS3 sequencing:** CB3 aerial resolution uses `nearest_teammate_near` for cross targets (the current T1 heuristic). Once FUN-TS3 introduces real support-angle geometry for receiver positioning, the aerial contest will automatically improve — CB3 doesn't need to wait for FUN-TS3, but the two compose cleanly.
- **`tackle_cooldown_until` index type:** the existing code at lib.rs:1380 indexes by `def_idx` (usize 0..22). CB2 uses the same pattern. Confirm the field is `[Tick; 22]` or `Vec<Tick>` (slot-indexed), not a map.
- **`MatchEvent` size assert:** extending to 12 variants, confirm `sz <= 64` still holds after `AerialDuelWon { winner_slot: PlayerSlot, loser_slot: PlayerSlot, tick: Tick }` — it should (~11 bytes + padding), but the programmer must verify and update the size probe test if it trips.
- **Loose-ball physics after miscontrol/dispossession:** the ball currently has `vel_x/y = Q32::ZERO` when snapped mid-pitch. CB1/CB2/CB3 loose balls should get a small random scatter velocity so the loose-ball chase doesn't converge on a point-stationary ball. Propose: `SeedLayer::BallPhysics`, site `0xCB05`, draw to produce `vel = ~0.5–1.5 m/s` in a random direction. Tag as a follow-on fix rather than a blocker — static loose ball still triggers the preempt-check nearest-2 chase correctly.
- **T4.5-E0 (gene compiler):** heading/jumping/strength differentiation only shows its full effect once real player gene-derived attributes replace the 0.5 mirror baseline. The aerial contest code is correct now; the tuning will need a re-sweep once differentiated rosters land.

---

**Files for `gameplay-programmer`:**

- `/Users/vibelogic/dev/football/crates/fw-content/src/event.rs` — add discriminants 7–11
- `/Users/vibelogic/dev/football/crates/fw-match-sim/src/dispatch.rs` — all three sub-mechanics land in `apply_intent`; retire `T1_PASS_COMPLETED`
- `/Users/vibelogic/dev/football/crates/fw-match-sim/src/utility/pitch_control.rs` — no changes needed; it is called from dispatch.rs as-is
- `/Users/vibelogic/dev/football/crates/fw-content/tests/event_discriminant_test.rs` — update discriminant pin table to 12 entries
- `/Users/vibelogic/dev/football/docs/design/match-realism-reference.md` — the realism anchors this spec cites (§2 for pass completion ordering, §3 for aerial/press context)


---

# Tactical-Identity + Player-Individuality (FUN-TI1-2, FUN-PI1-3)

Good. I now have all necessary data. Let me produce the spec:

---

## FUN-TI1 / FUN-TI2 / FUN-PI1 / FUN-PI2 / FUN-PI3 — Tactical Identity + Player Individuality

**Spec date:** 2026-06-04. **Owner:** systems-designer (numbers) + gameplay-programmer (Rust). **Rebaseline class summary** at the end of each sub-slice.

---

## FUN-TI1 — Formation wiring (audit A6)

### Mechanic

`MatchState::initial_with_content` (`crates/fw-match-sim/src/lib.rs:728`) already resolves `home_archetype` and `away_archetype` from `ContentStore::tactical_archetypes`. `TacticalArchetype.formation: Vec<FormationSlot>` carries per-slot `(roster_slot: u8, role: String, x: i16, z: i16)` (`crates/fw-content/src/runtime.rs:448`). The gap: after constructing the base state via `MatchState::initial(seed)`, the code overwrites the two archetype IDs but never reads `archetype.formation` to place players. `FORMATION_4_3_3_POSITIONS` (`subtree_library.rs:89`) remains the sole placement source.

**Step-by-step wiring:**

1. After `MatchState::initial(seed)` in `initial_with_content` (~`lib.rs:763`), call a new `apply_archetype_formation(team_idx, archetype, &mut state.players)` helper for both home (team 0, slots 0..11) and away (team 1, slots 11..22).

2. `apply_archetype_formation` iterates `archetype.formation` (already Vec-ordered, deterministic). For each `FormationSlot { roster_slot, role, x, z }`:
   - `slot_idx = team_offset + (roster_slot - 1)` (roster_slot is 1-indexed per the content type).
   - Away team: negate x (away defends +x goal): `placed_x = if team_idx == 1 { -x } else { x }`.
   - Set `state.players[slot_idx].pos_x = Q32::from_int(placed_x as i32)`.
   - Set `state.players[slot_idx].pos_y = Q32::from_int(z as i32)`.
   - Set `state.players[slot_idx].role` by mapping the `role` string through the existing `preferred_role_to_formation_role(&RoleId)` (already in `lib.rs:186`).

3. If `archetype.formation` is empty (currently several RON files), emit a `log::warn!` and **fall through to `FORMATION_4_3_3_POSITIONS`** (no panic — maintains playability until archetypes are fully authored). This is safe because `initial_with_content` already validates that the archetype ID exists.

4. `formation_position(slot)` in `subtree_library.rs:124` is still used by the zonal-slot machinery (FUN-TS1). After this change it is consulted only for slots not covered by the archetype's formation vec (GK=slot 0/11 if missing), and as the fallback for the `MatchState::initial(seed)` (bare) path. No change to `formation_position` itself.

**Tick seam:** construction time only (`initial_with_content`). No per-tick cost. The `TeamShape` sidecar (FUN-TS1) already reads zonal slots from the static formation positions as the baseline; after FUN-TI1 those baseline positions reflect the authored formation. The `zonal_slot` function in `team_shape.rs` continues to work — it shifts the static slot, whatever it is.

### Determinism shape

Pure struct-field assignment from a `Vec<FormationSlot>` iterated in insertion order. No RNG draw. No float. `Q32::from_int` on the `i16` coordinates. No canonical-hash impact from the positions themselves (positions are non-canonical, derived from content; `PlayerState.pos_x/pos_y` ARE canonical — so position changes do change the canonical hash). **Rebaseline class: ADR-0012 trigger #3 (documented sim-behaviour change).** Authorized scope: multi-pin rebaseline. The 60-tick smoke pin and 600-tick extended pin will drift. The rebaseline is mandatory at this task.

### Tuning bands

| Band | Value | Firmness | Source |
|---|---|---|---|
| Formation slot x fallback | `FORMATION_4_3_3_POSITIONS` | FIRM | Structural fallback only |
| Away team x-flip | negate x | FIRM | Coordinate convention (home defends -x goal) |
| Empty formation warning threshold | `len == 0` | FIRM | No SOFT dial here |

The 16 authored archetypes have empty `formation` vecs today — a narrative-director + gameplay-programmer task is required to fill them in RON before FUN-TI1 produces visible differentiation. That is content work, not sim work. FUN-TI1 ships the wiring; the content fills the wire.

### Proptest invariants

1. `formation_player_count_within_bounds` — for any archetype with a non-empty formation, `apply_archetype_formation` places exactly `archetype.formation.len()` players, all within slot range `[team_offset, team_offset+11)`.
2. `away_team_x_is_negated_vs_home` — given the same archetype applied to both teams, away slot x = `-(home slot x)` for every matching roster_slot pair.
3. `fallback_to_433_when_formation_empty` — when `archetype.formation.is_empty()`, all 11 slots retain the positions produced by `FORMATION_4_3_3_POSITIONS`.
4. `formation_positions_within_pitch_bounds` — every placed `pos_x ∈ [-52, 52]` and `pos_y ∈ [-34, 34]` after application.

### Acceptance

- `scripts/fw verify` green; new rebaseline committed with `// FUN-TI1 rebaseline`.
- An `insta` snapshot of initial player positions for a non-433 archetype (e.g. `attacking-fullback`) shows positions distinct from `FORMATION_4_3_3_POSITIONS`.
- Contact-sheet (`board-shots.mjs`) over 60 ticks with two different archetypes shows visibly different initial spreads.

### Open questions / dependencies

- Content: 16 archetype RON files need `formation` vecs populated. This is narrative-director / content work and can be done in parallel.
- FUN-TS1 `zonal_slot` reads from the static formation position; after FUN-TI1 it correctly reads the per-archetype placed position. No code change needed to `zonal_slot` — the static position was already the per-slot anchor, now it's just the right one.

---

## FUN-TI2 — Live manager identity + runtime role-fit (audit A12, A15)

### Mechanic

**Part A: Manager identity in pass/shot/tempo utilities**

`ManagerArchetype` (`crates/fw-content/src/manager.rs`) has `risk_appetite: Q32` and `possession_preference: Q32`, both validated `[0, 1]`. They exist in `ContentStore::managers` and are loaded from RON. They are currently unused in the sim.

The wiring path:

1. Extend `MatchState` with two new `#[serde(skip)]` sidecar fields: `home_manager_risk: Q32` and `away_manager_risk: Q32` (and corresponding `possession_preference` variants), initialized from the matched `ManagerArchetype` at `initial_with_content`. **These are non-canonical sidecars** — the canonical field is the manager ID (already in `fw-tauri` season state, not yet in `MatchState`). For now, thread the two Q32 values directly into the match as sidecar scalars derived at construction from the club's archetype.

   Alternatively (cleaner): embed them in `ArchetypeParams` (already a non-canonical sidecar in `MatchState` as `home_archetype_params`/`away_archetype_params`). Extend `ArchetypeParams` in `tactic_fsm.rs` with `risk_appetite: Q32` and `possession_preference: Q32` fields and populate them in `archetype_params_for`.

   **Recommended: fold into `ArchetypeParams`** — it's already threaded through `BtContext` and available at decision sites. `ArchetypeParams` is already non-canonical (`#[serde(skip)]` equivalent — it's a pure derivation of the canonical archetype ID string).

2. In `bt/on_ball.rs`, `utility_shoot` and `utility_pass_short`/`utility_pass_long` gain a `risk_appetite: Q32` parameter threaded through `BtContext`. **Formula:**

   ```
   shot_bias = lerp(Q32::from_raw(858_993_459), Q32::from_raw(1_288_490_189), risk_appetite)
   // lerp(0.2, 0.3, risk_appetite) — additive utility bonus atop existing xG score
   ```

   Worked example: Player with `xg_utility = 0.45`. Manager `risk_appetite = 0.75` → `shot_bias = lerp(0.20, 0.30, 0.75) = 0.275`. Effective shot consideration = `0.45 + 0.275 = 0.725`. Manager `risk_appetite = 0.25` → `shot_bias = 0.225`. Same player: `0.45 + 0.225 = 0.675`. The delta (~5pp) is small enough not to override player quality but large enough to shift shot-vs-pass trade-offs across 5400 ticks.

3. `possession_preference` tilts `utility_hold_ball` vs `utility_pass_short`. High possession-preference → stronger hold-ball utility weight:

   ```
   hold_bias = lerp(Q32::from_raw(429_496_729), Q32::from_raw(1_073_741_824), possession_preference)
   // lerp(0.10, 0.25, possession_preference)
   ```

**Part B: Role-fit penalty (A15)**

`RoleAffinityTable` exists in `fw-content::role_affinity`. Each player template declares a `preferred_role`. At `initial_with_content`, when a player is placed into a formation slot whose role differs from their `preferred_role`, derive a `role_fit_penalty: Q32` per slot.

1. Add `role_fit_penalty: [Q32; 22]` as a `#[serde(skip)]` sidecar on `MatchState` (or embed in the per-player telemetry — but a parallel array is cheaper and consistent with the existing `last_shot_xg` pattern).

2. At init: for each slot, `role_fit_penalty[slot_idx] = if formation_role == preferred_role { Q32::ZERO } else { ROLE_FIT_PENALTY }`.

   **`ROLE_FIT_PENALTY` = `Q32::from_raw(644_245_094)` ≈ 0.15.** Rationale: an out-of-position player is roughly 15pp worse at every utility consideration — consistent with FM's positional suitability "natural/accomplished/competent" tiers which collapse to roughly a 10–20% utility discount in matched-role simulations.

3. In `select_outfield_intent` (`subtree_library.rs:137`), multiply every utility candidate score by `(Q32::ONE - role_fit_penalty[slot_idx])` before softmax. Out-of-position players don't disappear from selection — they just softmax-lower, so a CB at RW is still capable but less likely to produce the right action.

   Worked example: CB at RW. Dribble utility = 0.55 (decent dribbling stat). After penalty: `0.55 × (1 - 0.15) = 0.4675`. vs. a natural RW at the same dribbling stat: `0.55`. The CB's crossing utility (lower base stat) similarly discounted. Over 5400 ticks, crossing attempts, shot decisions, and wide-attacking behaviour drop measurably.

**Tick seam:** Part A hooks `utility_shoot` / `utility_pass_short` / `utility_pass_long` / `utility_hold_ball` in `bt/on_ball.rs`. Part B hooks `select_outfield_intent` in `subtree_library.rs`. Both at the utility-computation step, before softmax. No new canonical state.

### Determinism shape

All Q32 arithmetic. `lerp(a, b, t) = a + (b - a) * t` — Q32-clean if `t ∈ [0,1]` (guaranteed by `ManagerArchetype` validator). No RNG draw. No new canonical fields. **Rebaseline class: ADR-0012 trigger #3 (behaviour change — utility scores shift).** Multi-pin rebaseline required.

### Tuning bands

| Band | Value | Firmness | Source |
|---|---|---|---|
| `risk_appetite` shot-bias range | [0.20, 0.30] additive | SOFT | Taste — iterate on shot-rate per-match |
| `possession_preference` hold-bias range | [0.10, 0.25] additive | SOFT | Taste — iterate on possession-spell length |
| `ROLE_FIT_PENALTY` | 0.15 | SOFT | FM analogue; iterate once differentiated rosters (FUN-DR) are in |

The ORDERING (high-risk manager shoots more, out-of-role player is weaker) is FIRM. The magnitudes are SOFT — locked to the design-doc after drama-sweep validation on differentiated rosters.

### Proptest invariants

1. `high_risk_manager_shoots_more` — same seed, two teams with `risk_appetite ∈ {0.1, 0.9}` respectively: the high-risk team produces ≥ mean(shots/match × 1.10) over 30 seeds.
2. `role_fit_penalty_strictly_reduces_utility` — for any out-of-position slot, all utility scores post-penalty are strictly less than or equal to pre-penalty (no accidental amplification).
3. `in_position_slot_has_zero_penalty` — `role_fit_penalty[slot_idx] == Q32::ZERO` when `formation_role == preferred_role`.

### Acceptance

- Drama-sweep run over 50 seeds shows statistically distinguishable `shot_count/match` between `risk_appetite = 0.85` and `risk_appetite = 0.15` archetypes (p < 0.10 Mann-Whitney on raw counts).
- An out-of-position player (e.g. slot tagged DEF placed at FWD) produces fewer `Shot` events per 90 ticks than a matched-position forward with the same attributes (verified by `insta` snapshot of intent distribution).

### Open questions / dependencies

- FUN-TI2 Part B only produces visible results once FUN-DR (differentiated rosters) retires the 0.5-mirror substrate — on mirror teams all roles are equally mediocre. Wire it now; calibrate after FUN-DR.
- Manager IDs need to be threaded from the season runner into `initial_with_content`. Currently `initial_with_content` takes archetype IDs but not manager IDs. The match state does not store `manager_id`. Proposed: pass `home_manager: Option<&ManagerArchetype>` and `away_manager: Option<&ManagerArchetype>` to `initial_with_content`, defaulting to `risk_appetite = 0.5 / possession_preference = 0.5` when `None`. Season runner (`fw-tauri/src/season.rs`) already has `ManagerArchetype` in scope.

---

## FUN-PI1 — Footedness (audit A14)

### Mechanic

The `left_foot: Q32 ∈ [0,1]` field exists in `GeneSnapshot::TechnicalAffinities` (`crates/fw-content/src/gene.rs:103`). The sim has zero foot references today. `PlayerState` carries `PlayerAttributes` which does not include a footedness scalar — it's a gene field, not an attribute. The gene→attribute compiler (T4.5-E0) will eventually map `left_foot` gene → derived attributes; until then we derive footedness directly from the gene at match init.

**Step 1: Derive per-player foot scalars at match init**

Add two new `#[serde(skip)]` parallel arrays to `MatchState`:
- `preferred_foot_is_left: [bool; 22]` — `left_foot_gene >= 0.5` (i.e., `Q32::from_raw(1i64 << 31)`).
- `weak_foot_penalty: [Q32; 22]` — `(Q32::ONE - |left_foot_gene - Q32::from_raw(1i64 << 31)| × 2).max(Q32::ZERO)` clamped to `[0, 1]`.

The formula: `left_foot_gene` ranges 0 (pure right-footer) to 1 (pure left-footer). `0.5` is ambipedal. Distance from 0.5 is the "strong foot advantage." The weak-foot penalty is stronger the more one-footed the player.

Worked example: `left_foot = 0.85` → preferred = left. Distance from 0.5 = 0.35. `weak_foot_penalty = clamp(0.35 × 2, 0, 0.70) = 0.70`. Strong foot acts normally. Weak-foot actions penalised by 70%.

`left_foot = 0.52` → preferred = left. Distance = 0.02. `weak_foot_penalty = clamp(0.02 × 2, 0, 0.04) = 0.04`. Near-ambipedal player; barely penalised.

`left_foot = 0.15` → preferred = right. Distance = 0.35. `weak_foot_penalty = 0.70`.

**Formula in Q32:**
```
dist = |left_foot_gene - HALF|          // Q32 abs, no cordic needed
raw_penalty = dist * Q32::from_int(2)   // Q32 multiply
weak_foot_penalty = raw_penalty.min(Q32::ONE)
```
This is pure Q32 arithmetic. No cordic. No float.

**Step 2: Apply at action sites**

A player acts on their weak side when the action direction opposes their preferred foot. "Direction" is a Q32 sign: ball position y relative to player y. Define `action_is_weak_side(player_slot, target_y, state) -> bool`:
- If preferred-left player: weak side = positive target_y (ball is to the player's right → cuts on the right → weak foot).
- If preferred-right player: weak side = negative target_y.

Apply in `bt/on_ball.rs` at:
- `utility_shoot`: if `action_is_weak_side`, multiply utility by `(Q32::ONE - weak_foot_penalty[slot_idx])`.
- `utility_cross`: same multiplier. Weak-foot winger crossing on the "wrong" side is less effective.
- `utility_dribble`: when dribbling toward weak side, multiply utility by `(Q32::ONE - weak_foot_penalty[slot_idx] / 2)`. Half-penalty — dribbling is less sensitive than striking.

Apply in `subtree_library.rs` at intent selection for `AttemptShot`: when the shot is on the weak side, reduce the shot's softmax score pre-pick by `weak_foot_penalty`. This drives the cut-in side naturally — a right-footed winger on the left side has higher utility cutting inward (shooting on the preferred foot) than crossing (crossing on the weak foot).

**SeedLayer:** no additional random draw for footedness itself — it is deterministic from the gene scalar. The existing `SeedLayer::Decision` draws already cover action selection.

**Tick seam:** `bt/on_ball.rs` utility functions, pre-softmax. `subtree_library.rs::select_outfield_intent`. Both at the utility-computation step.

### Determinism shape

All Q32 arithmetic on `[0,1]`-bounded inputs. The `|...|` absolute value is sign-flip + max — no cordic. No RNG draw for footedness. `preferred_foot_is_left` and `weak_foot_penalty` are `#[serde(skip)]` sidecars — zero canonical bytes. **Rebaseline class: ADR-0012 trigger #3 (behaviour change — shot/cross/dribble utilities shift).** Multi-pin rebaseline required.

### Tuning bands

| Band | Value | Firmness | Source |
|---|---|---|---|
| Ambipedal threshold | `left_foot == 0.5` | FIRM | Mathematical symmetry |
| Weak-foot penalty scaling | `distance × 2`, clamped `[0, 1]` | SOFT | Taste; real-world ~70% of elite players have a meaningful weak-foot deficiency |
| Dribble penalty divisor | `/ 2` (half penalty vs shooting) | SOFT | Dribbling is body-skill, less foot-dependent than striking |
| Cut-in detection | sign of `target_y - player_y` | FIRM | Geometric |

A pure right-footer (left_foot = 0.0) has `weak_foot_penalty = 1.0` — every weak-side shot is at 0× utility. That is too extreme for the flat-0.5 mirror substrate; it is correct once differentiated rosters arrive. Until FUN-DR, the penalty should be gated behind `if PlayerState has non-default gene scalar` (but the mirror substrate has `left_foot = 0.0`, which means all players are pure right-footers — also unrealistic). **Recommendation:** until T4.5-E0 populates `left_foot` from the gene compiler, default `left_foot = 0.5` (ambipedal) at `initial_with_content` when no gene data exists, to avoid degenerate all-right-footer behavior.

### Proptest invariants

1. `ambipedal_player_has_zero_weak_foot_penalty` — `left_foot = Q32::from_raw(1i64 << 31)` → `weak_foot_penalty = Q32::ZERO`.
2. `pure_one_footer_has_max_penalty` — `left_foot = Q32::ZERO` → `weak_foot_penalty = Q32::ONE`.
3. `weak_side_shot_utility_strictly_less_than_strong_side` — same player, same position, weak-side target → lower shot utility (non-zero penalty player only).
4. `preferred_foot_monotonic_with_gene` — `left_foot > 0.5` → `preferred_foot_is_left = true`; `left_foot < 0.5` → `preferred_foot_is_left = false`.

### Acceptance

- Insta snapshot: a right-footed winger (left_foot = 0.1) placed on the left side produces more `Dribble` intents toward the centre (cutting in) than `Cross` intents in a 300-tick run.
- Drama-sweep: no regression on M1 (goal mean stays in band) — footedness penalises weak-side shots but should not materially reduce total goals if players naturally favour their strong side.

### Open questions / dependencies

- **T4.5-E0 dependency:** the gene→attribute compiler will eventually surface `left_foot` as a derived `PlayerAttributes` field. Until then, read `GeneSnapshot::TechnicalAffinities::left_foot` directly at `initial_with_content`. This requires `PlayerState` to carry (or have accessible) the gene snapshot, which currently it does not — `PlayerState.attributes` is populated by `PlayerState::with_role` using `mid_range_baseline()`. The immediate approach: pass an optional `&GeneSnapshot` per slot at `initial_with_content` time (parallel to `with_slot_signatures`), populate `weak_foot_penalty` and `preferred_foot_is_left` from it. When absent, default to `left_foot = 0.5`.
- Flagged as **partially gated on T4.5-E0** for full differentiation, but the sidecar wiring can ship without T4.5-E0.

---

## FUN-PI2 — Form / consistency variance (audit A7)

### Mechanic

`PlayerCondition.form: Q32` and `personality.consistency: Q32` both exist in `fw-core::player_attributes` and are declared on `PlayerState` — but `form` is read by zero BT call sites today.

**Pre-match seeded form draw:**

At `initial_with_content` (or better: at the start of each match's construction, before the first tick), for each outfield player slot, draw a `form_modifier: Q32` from `SeedLayer::MemoryEvent`:

```
site = (match_seed_u64 as u32) ^ (slot as u32)   // per-slot uniqueness
rng = ChaCha8Rng::seed_from_u64(seed_fn(match_seed, Tick::ZERO, SeedLayer::MemoryEvent, site))
raw_u64: u64 = rng.next_u64()
// Map to [-1, +1] signed range by treating upper half as negative
signed_unit = (raw_u64 as i64) >> 32            // top 32 bits as signed i32 → Q32 in [-1, +1)
form_draw_q32 = Q32::from_raw(signed_unit as i64)
```

The draw width is set by `consistency`:

```
// High consistency → narrow draw (player performs near mean every game)
// Low consistency → wide draw
draw_width = Q32::ONE - personality.consistency   // [0, 1]; low consistency = wide
form_modifier = Q32::from_raw(1i64 << 31)        // baseline = 0.5 (neutral)
              + form_draw_q32 * draw_width / 2    // half-amplitude so mean stays ~0.5
```

Worked example A (consistent player, consistency = 0.85):
- `draw_width = 0.15`. `form_draw_q32 = -0.6` (bad luck day). `form_modifier = 0.5 + (-0.6 × 0.15 / 2) = 0.5 - 0.045 = 0.455`. Player is 4.5pp below their mean — a noticeably off day, but not catastrophic.

Worked example B (inconsistent player, consistency = 0.20):
- `draw_width = 0.80`. Same bad draw (-0.6): `form_modifier = 0.5 + (-0.6 × 0.80 / 2) = 0.5 - 0.24 = 0.26`. Player at 26% of normal — clearly misfiring. On a good day: `form_draw_q32 = +0.7` → `0.5 + (0.7 × 0.80 / 2) = 0.78` — an outstanding match.

**Wiring into utilities:**

Store `form_modifier` in `PlayerCondition.form` field at init. In `select_outfield_intent` (`subtree_library.rs`), multiply the entire utility vector by `player.attributes().condition.form` before softmax. This means a low-form player has uniformly lower utilities — they're more likely to mis-select (a bad day reads as hesitation, poor decisions), which matches the mental attribute model.

`PlayerCondition.form` is already canonical in `PlayerState`. **Setting it at init is therefore a canonical-state change.** However: if `form` is currently always initialized to `Q32::ZERO` (which it is — `PlayerState::with_role` calls `mid_range_baseline()` which sets form=0.5 or the `PlayerCondition` default), verify the current init value. If currently `Q32::ZERO`, the form field change at init **is** a canonical drift and needs a rebaseline. If currently `0.5` (mid_range_baseline), it only changes when a non-`0.5` draw fires, which is every match with real genes.

Actually the cleanest canonical-surface approach: keep `form` in `PlayerCondition` as canonical (it already is serialized), and write the per-match draw into it at `initial_with_content`. The 600-tick pin will drift. Authorized rebaseline.

**SeedLayer:** `SeedLayer::MemoryEvent` for the form draw. Site = `match_seed_low32 XOR slot_idx`. Rationale: form is a pre-match draw tied to the player+match pairing — closest to a memory/condition event.

### Determinism shape

One `ChaCha8Rng` draw per player slot at match init. `SeedLayer::MemoryEvent`, site = `(match_seed as u32) ^ (slot as u32)`. Q32 arithmetic throughout. `form` is already in canonical `PlayerCondition` — writing it at init changes canonical bytes. **Rebaseline class: ADR-0012 trigger #1 (canonical schema bump) if `form` was always 0 before; trigger #3 (behaviour change) if already variable.** Based on the code (`PlayerState::with_role` sets `mid_range_baseline()` which has `form: Q32::ONE` in `PlayerCondition` — need to verify), expect trigger #3. Multi-pin rebaseline required.

### Tuning bands

| Band | Value | Firmness | Source |
|---|---|---|---|
| Form draw distribution | uniform signed draw scaled by `(1 - consistency)` | SOFT | Taste — beta distribution would be richer but overkill |
| Max form swing (pure inconsistent) | `±0.40` around 0.5 baseline | SOFT | Iterate on drama-sweep M7 (swing variance) |
| Min form swing (pure consistent) | `±0.05` around 0.5 | SOFT | Consistent players should not be identical every game |
| SeedLayer | `MemoryEvent` | FIRM | Contract (non-overlapping discriminants per ADR-0009) |

The 0.5 baseline for `draw_width / 2` halving is the main lever: `draw_width / 2` keeps the mean form near 0.5 across large samples. If this is changed to `draw_width / 1`, the mean remains 0.5 but variance doubles — likely too much.

### Proptest invariants

1. `consistent_player_narrower_form_variance` — over 100 seed draws, `variance(form_modifier | consistency=0.9) < variance(form_modifier | consistency=0.1)`.
2. `form_draw_in_unit_range` — for all seeds and consistency values, `form_modifier ∈ [Q32::ZERO, Q32::ONE]` (no overflow, no underflow; the `/ 2` halving + 0.5 baseline guarantees `[0, 1]`).
3. `form_draw_is_seed_deterministic` — same match_seed, same slot → same `form_modifier` on every call.

### Acceptance

- `drama_sweep` M7 (game-level swing) shows measurable increase over mirror baseline.
- An insta snapshot of two 300-tick runs with the same archetype pair but different seeds shows differing `form_modifier` values (the canonical form field differs between seeds).

### Open questions / dependencies

- `PlayerCondition` fields other than `form` (`morale`, `match_fitness`, `sharpness`, `signature_readiness`) remain INERT for this slice. Do not wire them here.
- Gated behind FUN-DR for full visible effect — on mirror teams, form variance is visible only in action frequencies, not in quality differentiation.

---

## FUN-PI3 — In-match injuries (audit A8)

### Mechanic

`InjuryLongTerm` is an existing `MemoryEvent` class (discriminant 27, `crates/fw-memory/src/event.rs:303`). `DurabilityProfile.injury_proneness: Q32` exists on every player. No in-match injury roll fires today.

**Per-tick seeded roll:**

After `resolve_tackles` in `tick_match` (`lib.rs` — the tackle-resolution step, which already mutates cooldowns and handles dispossession), for each slot involved in a tackle contest that tick:

```
site = (slot as u32) << 16 | tick.to_raw() as u32  // per-slot-per-tick unique
rng = ChaCha8Rng::seed_from_u64(seed_fn(match_seed, tick, SeedLayer::ScoutObservation, site))
// ScoutObservation is the only unused SeedLayer at match-time for this kind of rare event
// Alternative: allocate a 9th SeedLayer::InjuryRoll — see open questions

roll: u32 = rng.next_u32()
threshold = INJURY_BASE_RATE × injury_proneness × contact_factor × fatigue_factor
// all Q32; contact_factor from tackle_type; fatigue_factor from match_fitness
if Q32::from_raw((roll >> 1) as i64) < threshold:
    // injury fires
```

**Formula in Q32:**
```
contact_factor = Q32::from_raw(429_496_729)   // 0.10 — base rate per contact; SOFT
fatigue_factor = Q32::ONE                      // until A1 (fatigue) is wired
threshold = contact_factor × injury_proneness
```

`INJURY_BASE_RATE`: the probability of injury per tackle-contact for a median player (injury_proneness = 0.5). Target: ~0.3 injuries per 90 minutes per team (real football: top-flight teams see ~0.2–0.5 match-ending injuries per match combined; most are soft-tissue, not match-ending). Per team, that is roughly 1 injury per 5 matches. Over 5400 ticks with ~25 tackle contacts per match per team: `threshold_median ≈ 0.3 / (25 × 2) ≈ 0.006` per contact.

`contact_factor = Q32::from_raw(25_769_803)` ≈ 0.006 (SOFT dial — this is the primary tuning knob).

Worked example: High-proneness player (injury_proneness = 0.8), typical tackle. `threshold = 0.006 × 0.8 = 0.0048`. Roll is Q32 uniform `[0, 1)`. P(injury) ≈ 0.48%. Per 5400 ticks (~25 contacts): `1 - (1 - 0.0048)^25 ≈ 11%` per match — that player gets injured once every ~9 matches. Low-proneness player (0.2): `0.6% × 0.2 / 0.5 = 0.0024` → `6%` per match → once every ~16 matches. These feel plausible for an "injury-prone" archetype vs. durable player.

**On injury trigger:**

1. Push `MatchEvent::PlayerInjured { slot: PlayerSlot, tick: Tick }` — **new discriminant 7** (append-only, updates encoder VERSION, updates `MatchEventDiscriminant::all()`).
2. Set `state.players[slot_idx].is_injured = true` — **new canonical bool field** on `PlayerState`.
3. The injured player's `select_outfield_intent` returns `MoveToPosition { target: player.pos }` (hold position — they're hobbling). If `is_injured == true`, their decision slot fires no BT tree: return `PlayerIntent::MoveToPosition { target_x: player.pos_x, target_y: player.pos_y }` (stub for staying put).
4. For the "10-man" scenario: a sub is not yet modelled (no bench). The injured player holds position but contributes nothing. The team effectively plays 10 outfield players. This is the correct realism-first behavior — a player collapses and the team plays on shorthanded.
5. Push a `MemoryEvent::InjuryLongTerm` to the match's pending memory events (to be flushed to the ledger post-match by T3-1's wiring). For now, accumulate in a `#[serde(skip)]` `Vec<MemoryEvent>` sidecar on `MatchState` (same pattern as `shot_telemetry`). The existing `InjuryLongTerm` event class in `fw-memory` already has breakthrough effects (Pace -0.15, Stamina -0.30, etc.) authored.

**Tick seam:** after `resolve_tackles` in `tick_match`. Only fires for slots involved in a tackle event that tick (not a blanket per-tick-per-player roll — that would be 22 rolls/tick = 475,200 draws per match for a zero-injury scenario, which is wasteful and inflates the RNG call budget).

### Determinism shape

One `ChaCha8Rng` draw per tackle-involved slot per tick. **SeedLayer:** `SeedLayer::ScoutObservation` is the only 8-entry discriminant not yet consumed in-match. However, this creates a naming mismatch. **Recommended: extend `SeedLayer` with a 9th variant `SeedLayer::InjuryRoll = 8`** — this is a canonical-surface change to the enum (no canonical hash impact unless the enum is serialized; it is not — `SeedLayer` is never serialized to canonical bytes, only used in `seed_fn`'s u64 discriminant arithmetic). The discriminant 8 does not overlap any existing discriminant.

New canonical fields: `is_injured: [bool; 22]` on `MatchState` as a parallel array (or on `PlayerState` directly). The `MatchEvent::PlayerInjured` variant is new canonical. **Rebaseline class: ADR-0012 trigger #1 (canonical schema bump: new event variant + new state field) + trigger #3 (behaviour change).** Authorized multi-pin rebaseline.

### Tuning bands (grounded in realism reference)

| Band | Value | Firmness | Source |
|---|---|---|---|
| Base contact injury rate | ≈ 0.006 per contact | SOFT | Derived from ~0.3 injuries/90min / 25 contacts; MEDIUM firmness |
| Injury-proneness modifier | linear scalar on base rate | SOFT | Not enough public data on per-player injury rates to set FIRM |
| Fatigue modifier | `Q32::ONE` (inert) until A1 ships | FIRM (structural) | Must not use inert fields for tuning |
| Match injury rate target | ~0.06–0.12 injuries/match (both teams) | SOFT | Top-flight: ~0.2–0.5 actual; fantasy world may run lower |
| 10-man duration | full remainder of match | SOFT | No sub → player holds position until FullTime |

**HARD anchor (match-realism-reference §4):** player injuries are not explicitly quantified in the reference doc. Use the ordering: match-ending injuries are rare spectacles (~5–10% of top-flight matches have any), not routine events. Gate the rate so drama-sweep shows injuries in the 5–15% of match band.

### Proptest invariants

1. `injury_roll_is_seeded_deterministic` — same match_seed, tick, slot → same injury decision.
2. `injury_proneness_monotonic` — all else equal, `P(injury | proneness=0.9) > P(injury | proneness=0.1)` across 1000 simulated rolls.
3. `injured_player_holds_position` — after an `is_injured = true` flag, the player's `pos_x`/`pos_y` changes by at most `MAX_PLAYER_SPEED × 1` per tick (they stop running forward).
4. `match_event_includes_player_injured` — whenever `is_injured` flips true, a `MatchEvent::PlayerInjured` is present in `match_events` at the corresponding tick.

### Acceptance

- `drama_sweep` over 50 seeds: injury count/match in `[0.03, 0.25]` band (both teams combined); no match exceeds 3 injuries.
- `insta` snapshot of a high-proneness player (`injury_proneness = 0.95`) run over 600 ticks: injury event appears at least once across 10 seeded runs.
- `encode_match_event(PlayerInjured { slot: 3, tick: Tick::from_raw(240) })` unit test in `canonical.rs` (mirrors the existing `Goal` encoder test pattern added after T1-4a).

### Open questions / dependencies

- **SeedLayer extension:** adding discriminant 8 (`InjuryRoll`) is a code change to `decision_cadence.rs` (`SeedLayer` enum). Not canonical, but affects all `seed_fn` call semantics — needs a comment explaining the 9-discriminant expansion.
- **Forced substitution:** no bench model exists yet. The player stays on the pitch, injured. A real sub system (A4) is the correct follow-up. Do NOT simulate a sub here — that is A4 scope.
- **Memory ledger flush:** `fw-memory` `InjuryLongTerm` wiring from match events to the persistent ledger is T3-1 scope. The sidecar Vec accumulates it for T3-1 to flush; FUN-PI3 only needs to populate the sidecar.
- **Fatigue modifier:** FUN-PI3 leaves `fatigue_factor = Q32::ONE` (neutral) until A1 (in-match fatigue drain) ships. The formula is written to accept it; the wire-up is a one-line change when A1 lands.

---

## Cross-slice rebaseline summary

| Slice | Canonical change | Rebaseline class | New SeedLayer draws |
|---|---|---|---|
| FUN-TI1 | `pos_x`/`pos_y` shift at init | Trigger #3 | None |
| FUN-TI2 | None (sidecar only) | Trigger #3 (behaviour) | None |
| FUN-PI1 | None (sidecar only) | Trigger #3 (behaviour) | None |
| FUN-PI2 | `PlayerCondition.form` written at init | Trigger #3 (form was non-zero) | 1 per slot at init (SeedLayer::MemoryEvent) |
| FUN-PI3 | New `PlayerState.is_injured`, new `MatchEvent::PlayerInjured` | Trigger #1 + #3 | 1 per tackle-involved slot per tick (SeedLayer::InjuryRoll discriminant 8) |

All five slices require authorized multi-pin rebaselines. The recommended delivery order: FUN-TI1 (data wiring, cheapest) → FUN-TI2 (utility wiring, no canonical bytes) → FUN-PI1 (footedness, no canonical bytes) → FUN-PI2 (form draw, one canonical field touched) → FUN-PI3 (injury, new canonical event + field, most surface area).

**FUN-PI3 specifically depends on:** tackle-contact tracking already existing in `resolve_tackles` (`lib.rs`). Verify that `resolve_tackles` returns or identifies which slots were involved in a contest before adding the injury-roll site there.

**All five slices are partially or fully gated on T4.5-E0 + FUN-DR for meaningful calibration.** The mechanics are correct to ship now; the numbers will tune once differentiated rosters (FUN-DR) retire the flat-0.5 mirror substrate. Ship with drama-sweep guards confirming no M1 regression.

---

**Relevant file paths:**
- `/Users/vibelogic/dev/football/crates/fw-match-sim/src/lib.rs` (initial_with_content seam ~line 728)
- `/Users/vibelogic/dev/football/crates/fw-match-sim/src/subtree_library.rs` (FORMATION_4_3_3_POSITIONS:89, select_outfield_intent:137)
- `/Users/vibelogic/dev/football/crates/fw-match-sim/src/bt/on_ball.rs` (utility_shoot, utility_cross, utility_dribble)
- `/Users/vibelogic/dev/football/crates/fw-match-sim/src/tactic_fsm.rs` (ArchetypeParams struct)
- `/Users/vibelogic/dev/football/crates/fw-match-sim/src/decision_cadence.rs` (SeedLayer enum — add discriminant 8)
- `/Users/vibelogic/dev/football/crates/fw-content/src/runtime.rs` (TacticalArchetype:402, FormationSlot:448)
- `/Users/vibelogic/dev/football/crates/fw-content/src/manager.rs` (ManagerArchetype risk_appetite:176, possession_preference:181)
- `/Users/vibelogic/dev/football/crates/fw-content/src/gene.rs` (TechnicalAffinities::left_foot:103)
- `/Users/vibelogic/dev/football/crates/fw-core/src/player_attributes.rs` (PlayerCondition:944, DurabilityProfile:353)
- `/Users/vibelogic/dev/football/crates/fw-content/src/event.rs` (MatchEvent discriminant table — append PlayerInjured as discriminant 7)
- `/Users/vibelogic/dev/football/crates/fw-memory/src/event.rs` (InjuryLongTerm discriminant 27 — existing ledger class)

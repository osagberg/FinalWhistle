# open-football (ZOXEXIVO) — match engine + AI read-through

**Read on:** 2026-05-13
**Repo size:** ~217 KLoC total — but the match-engine slice we care about is `src/core/src/match/` at **~16,200 LoC** (engine.rs 98 KB / ~3.4k lines + tactical.rs 1,576 + rating.rs 2,231 + forwarders/states/running/mod.rs alone at **2,141 lines**). The remaining ~200 KLoC is database/web/competitions/awards/transfers — out of scope here.
**Workspace deps:** `chrono`, `rand 0.10.1`, **`rayon 1.12`**, `serde`, `nalgebra 0.34`, `itertools`, `deunicode` (`src/core/Cargo.toml:18–25`). Top-level `Cargo.toml` adds `tokio` (full features) at the binary layer.
**Edition:** 2024.
**Match-engine entry point:** `FootballEngine::<840, 545>::play(...)` — pitch dimensions are **const generics** on the type (`game.rs:64`).
**Single dir worth reading:** `src/core/src/match/engine/`. The `src/core/src/ai/` directory is NOT match AI — it's an `AiService` trait abstraction for **LLM-style external batch requests** (`ai/mod.rs:13–23`, `ai/request.rs:1–18`) used by club/transfer logic. Match AI is entirely under `match/engine/player/strategies/`.

---

## Tick structure

**10 ms fixed-Δt, ~100 Hz nominal — but half the ticks skip AI evaluation, so effective AI rate is 50 Hz.**

- Tick constant: `MATCH_TIME_INCREMENT_MS: u64 = 10` (`engine/context.rs:15`).
- Main loop: `play_inner()` (`engine/engine.rs:770–875`), `while context.increment_time() { ... }` (line 791).
- `tick_parity & 1 == 0 → game_tick_light` (ball + player movement only), else `game_tick_inner` (ball + AI re-eval + events) (`engine.rs:821–825`). Comment on line 819: "Full tick: ball + player AI + events / Light tick: ball + player movement only (no AI re-evaluation)".
- Coach evaluation: every **500 ticks (~5 s)** (`engine.rs:798`).
- Team tactical refresh: every **10 ticks** (`engine.rs:789, 813`). Comment on 810–812: "too fast and we chase flicker in the ball-owner signal; too slow and transition windows (≤50 ticks) lose resolution."
- Half length: `MATCH_HALF_TIME_MS` constants drive the half/ET/PK state machine in `state/manager.rs`. `increment_time` returns `false` when `new_time >= half + stoppage` (`context.rs:208–214`).
- Integration is Euler: `position += velocity` directly in `MatchPlayer::move_to` (`player/player.rs:407–421`), with NaN/Inf salvage clamping position back to `start_position` (line 427).
- Player movement is gated through a max-speed clamp + a ball-carrier multiplier (0.75–1.00 of off-ball speed depending on dribbling composite, `player/state.rs:113–135`).

---

## AI architecture

**Per-role hierarchical FSM. No behaviour tree. No utility scoring (at the architecture level — there's local utility-style scoring inside state-transition helpers, but no global utility loop).**

Architecture:

1. `PlayerState` is a flat enum with 5 outer variants, one per role plus `Injured` (`player/state.rs:14–21`). Inner state enums per role:
   - `GoalkeeperState`, `DefenderState`, `MidfielderState`, `ForwardState` — each ~15-20 variants.
   - Forward states (`strategies/forwarders/states/`): `Standing, Walking, Running, Shooting, Passing, Crossing, Dribbling, Finishing, Tackling, Pressing, Heading, Returning, Resting, CreatingSpace, RunningInBehind, CrossReceiving, Assisting, Intercepting, TakeBall`.
   - Defender states: parallel set incl. `Marking, Tracking, Covering, Clearing, Guarding, HoldingLine, PushingUp` (`strategies/defenders/states/`).
2. Dispatch: `PlayerFieldPositionGroup::process()` (`strategies/processor.rs:32–76`) matches on role → calls the per-role `Strategies::process(state, processor)`.
3. Each state implements `StateProcessingHandler` (`processor.rs:18–29`) with three methods: `process()` (decide transition + emit events), `velocity()` (per-tick velocity contribution), `process_conditions()` (side-effects e.g. fatigue accumulation).
4. **Universal pre-emption hooks** at dispatch time before the per-state handler runs: `should_force_takeball` and `should_yield_takeball` (processor.rs:52–252). These guarantee exactly-one chaser for a loose ball with **8.0u hysteresis** to stop tick-to-tick chase ping-pong (lines 151–166). This is one of the more pragmatic patterns in the codebase.
5. Steering layer: simple `SteeringBehavior` enum — `Seek / Arrive / Pursuit / Evade / Wander / Flee / FollowPath` (`behaviours/steering.rs:4–34`). Velocity functions in each state usually compose one of these with a target vector chosen by hand-written rules.

**State `process` bodies are priority-ordered if/else cascades, NOT scored utility evaluators.** Forward `Running` (`forwarders/states/running/mod.rs`) is 2,141 lines. It has "Priority 0, 0.5, 0.6, 0.8" comments and ~13 branching gates (search for `// Priority` in lines 305–567). Atomic counters guard each gate (`shot_gate_stats`, lines 27–77) so the dev can read the waterfall.

Team-level tactical state (`engine/tactical.rs:23–53`) is a `GamePhase` enum: `BuildUp, Progression, Attack, AttackingTransition, DefensiveTransition, MidBlock, LowBlock, HighPress`. Player states branch on phase before falling back to local heuristics — this is the closest thing to a top-down tactic layer.

---

## Player state

`MatchPlayer` (`player/player.rs:26–100`) is heavy:
- `id: u32`, `team_id: u32`
- `position: Vector3<f32>`, `start_position: Vector3<f32>`, `velocity: Vector3<f32>` — **`f32` via `nalgebra::Vector3`**.
- `state: PlayerState`, `in_state_time: u64`
- `player_attributes: PlayerAttributes`, `skills: PlayerSkills`, `attributes: PersonAttributes`, `traits: Vec<PlayerTrait>`
- `tactical_position: TacticalPositions`, `waypoint_manager: WaypointManager`, `cached_waypoints: Vec<Vector3<f32>>`
- `memory: PlayerMemory`, `statistics: MatchPlayerStatistics`, `fatigue_accumulator: f32`
- Match-time bookkeeping: `tackle_cooldown: u16`, `pending_shot_reason: Option<&'static str>`, `yellow_cards: u8`, `fouls_committed: u8`, `is_sent_off: bool`, `entry_match_time_ms: u64`, `last_pressure_tick: u64`, `is_force_match_selection: bool`, `birth_date: NaiveDate`.

Skills are `u8` 0–100 inside `PlayerSkills` but every composite (`skill_composites::*`) divides by 20.0 and returns `f32`. Finishing in shooting state: `let finishing = ctx.player.skills.technical.finishing / 20.0` (`forwarders/states/shooting/mod.rs:44`).

**No fixed-point. No quantization. No serde on canonical match state.** This is a pure `f32` Euclidean sim.

---

## Decision points

**Decentralised, per-player, per-tick — but state transitions are the only place "decisions" happen.** A player in `Running` doesn't say "I shoot" mid-stride — `Running.process()` returns `StateChangeResult::with_forward_state(Shooting)`, then on the NEXT tick `Shooting.process()` runs and emits the shoot event.

Concrete entry points for the "should I X?" decisions:
- **Shoot/pass/hold decision helper:** `evaluate_forward_shot_decision()` (`strategies/common/players/ops/forward_shot_decision.rs`, returns `ShotDecision::{Shoot, Pass, Hold}`) — called from `Running` (transition gate) and from `Shooting` itself as a last-mile sanity check (`forwarders/states/shooting/mod.rs:57–67`). This IS a scored decision, internally — but called from inside the priority-cascade not in place of it.
- **Pass selection:** `PassEvaluator` (`strategies/common/passing/`).
- **TakeBall claim:** centralised single-chaser logic in `processor.rs:179–252` — strictly-closest teammate by `landing_position` distance, with id tie-break, hysteresis on yield.
- **Coach evaluations:** every 500 ticks. `evaluate_situational_shape()` (`engine.rs:894–963`) probes for formation pivots with a 12-minute hysteresis cooldown to prevent thrashing.

Decisions are tagged with `&'static str` reasons (`pending_shot_reason`, `PASS_REASON`-style codes) so the per-match log shows which code path fired — a built-in debugging affordance worth borrowing.

---

## Tactics

- Top-level enum `MatchTacticType` (lookups via `crate::Tactics::with_reason`, `engine.rs:946`). Standard formations (4-4-2, 4-3-3, etc.).
- `TacticsSelector::situational_shape(current, is_home, score_diff, minutes)` (`engine.rs:931–933`) returns `Option<MatchTacticType>` for in-match shape changes.
- Per-player tactical position comes from `TacticalPositions` + a hand-curated waypoint set per position (`player/waypoints/`, `tactics/positions.rs`). Players follow waypoints in formation when they're not the chaser and not in an explicit ball-related state (`player.rs:481–522`).
- Team-level phase recomputed every 10 ticks via `refresh_tactical_states()` (`engine.rs:1183` onward). Possession + field-tilt counters accumulate in `MatchCoach.cum_possession_ticks` / `cum_field_tilt_ticks` and feed coach rolling metrics.
- Coach instructions (tempo, press intensity) multiply through to player velocity at integration time: `velocity * processing_ctx.team().coach_instruction().tempo_multiplier()` (`processor.rs:304–307`).

---

## async / tokio analysis

**Match engine itself is fully synchronous.** No `async fn`, no `.await` anywhere under `match/engine/`. `play_inner` is a plain blocking loop.

Async lives at two outer layers:
- `simulator/mod.rs:60` — `FootballSimulator::simulate(...)` is `async`, but only to `.await` the **LLM-style** `AiBatchProcessor::execute(...)` (`simulator/mod.rs:169`, `ai/batch.rs:7–14`) and to keep a `Future` shape for the web layer.
- Top-level binary's `tokio` is for the HTTP server (`Cargo.toml:23`).

**rayon is used both inside the engine surroundings AND for award/transfer pipelines.** Match-level: `MatchPlayEnginePool` (`match/pool.rs:21–58`) parallelises whole-match resolution via `matches.into_par_iter().map(|m| m.play())`. That's fine for cross-match parallelism — but combined with the `rand::rng()` calls below it kills determinism unless seeds are threaded explicitly (they're not).

---

## Determinism story

**They don't care.** Every RNG call I found uses `rand::rng()` — the thread-local, OS-seeded, unseedable default RNG. Examples:
- `engine/engine.rs:854, 867, 1601` (substitution timing, time-wasting).
- `player/strategies/defenders/states/tackling/mod.rs:233`, `clearing/mod.rs:51`.
- `player/strategies/goalkeepers/states/tackling/mod.rs:105`, `clearing/mod.rs:70`, `passing/mod.rs:174`.
- `player/strategies/forwarders/states/tackling/mod.rs:299`.
- `player/strategies/midfielders/states/tackling/mod.rs:163`.
- `player/strategies/common/players/player.rs:126, 194`.
- `player/events/players.rs:1047, 1838, 2594`.

No `ChaCha`, no `StdRng::seed_from_u64`, no `match_seed` plumbed anywhere. No pinned canonical-state hash. No BLAKE3. `match/result.rs:130–203` uses `std::collections::HashMap<u32, Vec<...>>` for `players`, `player_states`, `last_state_ids` — **hash-randomised iteration order in the canonical output**.

Same `(home_squad, away_squad)` ⇒ different match every run. This is by design ("Tired of FM... the simulation logic, mechanics, and football intelligence keep improving" — README). The sim is non-reproducible and not intended to be.

---

## Code quality

Honest read: **mostly an engine, partly a beast.** Verdict by signal:

- **Solo-maintainable?** Marginal. Single committer credit (Artemov Ivan, `Cargo.toml:7`). The hot files are huge — `forwarders/states/running/mod.rs` at 2,141 lines, `engine/engine.rs` at ~3.4k lines, `engine/rating.rs` at 2,231, `engine/tactical.rs` at 1,576. One person clearly DOES maintain it, but the load-bearing files would resist a second contributor without significant refactoring.
- **Comment quality is excellent for the hot paths** — the `should_force_takeball` rationale (processor.rs:170–252), the float-NaN guard reasoning (player/player.rs:411–456), the chase hysteresis story (processor.rs:144–156), the tackle-cooldown justification (player/player.rs:298–308) are all genuinely instructive. The author has clearly hit each of these bugs and is writing future-self notes. We should steal the style.
- **No tests outside two modules.** `engine/intelligence_tests.rs` (464 lines) + `engine/match_realism_tests.rs` (287 lines) are basically behavioral spot checks, not regression coverage. No property-based tests, no canonical-state snapshots, no determinism harness (because it isn't deterministic).
- **Lots of dead `match-logs` instrumentation.** Atomic counters for shot gates, tackle gates, etc. — these are debug scaffolding compiled out of release. Useful pattern; messy in-place.
- **f32 everywhere, raw nalgebra Vector3, no newtypes.** `team_id: u32` not `TeamId(u32)`. `player_id` likewise. Position is a bare `Vector3<f32>`. Would not survive Final Whistle's `Rust/RULES.md`.

Code-to-insight ratio: ~30%. The remaining 70% is hand-rolled priority cascade and boilerplate getters/setters. The **insight** that matters fits in ~3k LoC — the rest is volume.

---

## What's worth adopting (5)

1. **Per-role flat-enum FSM dispatch** (`processor.rs:32–76`). Cheaper than a generic BT runner for our 22-player budget. We've been assuming BT — this is evidence a FSM-per-role can ship a believable match without it.
2. **Universal pre-emption hooks at dispatch time, not per-state** (`processor.rs:52–60`). The single-chaser logic with hysteresis (`should_force_takeball` / `should_yield_takeball`) is the kind of cross-cutting concern every state would otherwise duplicate. Build that lever into our dispatcher BEFORE we have the bug.
3. **Team-level `GamePhase` enum recomputed at coarser cadence than per-player AI** (`tactical.rs:23–53`, refreshed every 10 ticks per `engine.rs:813`). Player states branch on phase first, local heuristics second. Cleanly resolves the "11 independent agents" anti-pattern called out in the file's own header comment.
4. **Tagged `&'static str` decision reasons on emitted events** (`pending_shot_reason`, `PASS_REASON_*`). Free observability — match log shows WHICH code path fired the shot. Cheap to bake in now; impossible to retrofit cleanly later.
5. **Full-tick / light-tick alternation** (`engine.rs:819–825`). AI re-evaluation at 50 Hz with movement integration at 100 Hz halves the CPU cost of decision logic for free, and is invisible to players. We've been planning fixed 60 Hz everything; this is a cheap win.

## What's worth avoiding (5)

1. **`rand::rng()` everywhere.** Not just "we have to fix this" — the whole architecture assumes each tick can spin up a new thread-local RNG. Our sim must thread a `ChaCha8Rng` seeded by `(match_seed, tick, event_id)` through every decision site. Decide that early, not late.
2. **`f32` `Vector3` as the canonical position type.** The cross-platform divergence Final Whistle's sim spec calls out (`Sim/RULES.md` §1) would bite immediately if we copied this. The 8.0u hysteresis on chase yield is partly a band-aid for float jitter that wouldn't exist with Q32.
3. **Hand-rolled priority cascades in 2,000-line state files** (`forwarders/states/running/mod.rs`). The "Priority 0, 0.5, 0.6, 0.8" comment ladder is what utility scoring is supposed to replace. If we go FSM, every state body must stay under ~200 lines or we'll inherit the same maintenance tax.
4. **`HashMap` in canonical match output** (`match/result.rs:130–203`). Iteration order is non-deterministic. Our `MatchResult` must use `BTreeMap` or `Vec<(Id, T)>` from the start — `Sim/RULES.md` §2 is exactly this rule.
5. **No determinism harness, no canonical-hash regression test.** They can ship without one because matches are presentation only — but a manager game where saves can be reloaded and viewed REQUIRES the regression floor. Without one, every refactor risks silent drift that's only caught by a player noticing the same match plays out differently after an update.

---

## Open questions

1. **How does their per-state decision helper `evaluate_forward_shot_decision` score Shoot vs Pass vs Hold?** Worth a deeper read in `strategies/common/players/ops/forward_shot_decision.rs` — it's the only place that looks utility-shaped.
2. **`PlayerMemory` decays every 100 ticks** (`player/state.rs:60–62`). What does it actually remember and how big does it grow? Relevant to our T2 "careers that remember" pillar if we crib the in-match-memory pattern.
3. **`PlayerSkills` u8 + `/ 20.0` composites everywhere — is the composite layer (`skill_composites::*`) stable enough to lift as a structural template?** They wire ~30+ composite functions (`gk_shot_stopping`, `defensive_duel`, `pass_selection`, `off_ball_attack`, `movement_speed_with_ball`, ...). Worth a side report if T1-3a player-attributes ends up needing the same surface area.
4. **Waypoint following + crowding-yield logic** (`player.rs:481–522`) — the 12-unit "shoulder-to-shoulder" peel-off rule with lower-id deterministic tie-break is exactly the kind of anti-bunching heuristic our 22-agent steering layer will need. Verify whether it survives without the rest of their `WaypointManager` machinery.
5. **Cross-match rayon parallelism in `MatchPlayEnginePool`** — we don't need this for T1-2b (we run one match interactive), but for batch season simulation it's interesting. With seeded RNG it becomes safe.

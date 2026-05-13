# Existing Rust football sims — synthesis brief

> **Caveat (2026-05-13 reframe):** this brief was authored under a "~3000 LoC match-sim budget" framing that has since been retracted (see `docs/DESIGN_DOC.md` §1 "Scope ambition"). Recommendations of the form "X is too big for our budget" — particularly around FSM-with-large-state-taxonomies and ZOXEXIVO's per-role catalog — should be re-read as "the choice is now open, decide on clarity-vs-composition merits, not LoC." Pillar constraints (determinism, no-runtime-LLM, procedural-fantasy only, text-first) all remain.

**Compared:** 2026-05-13
**Projects:** openfootmanager (57K LoC) + ZOXEXIVO/open-football (217K LoC)
**For:** Final Whistle T1-2b architecture decisions

## TL;DR (one paragraph)

OFM is a small, legible, event-driven minute-by-minute simulator with a clean
Tauri+React shell — a near-perfect *shape* match for FW's T1 first-match work,
but it has zero determinism discipline (f64 everywhere, `HashMap` in the report
crate, no cross-OS hashing) and no deep AI. ZOXEXIVO is a maximalist 10ms-tick
spatial simulator with steering behaviours, per-role state machines (~80
states), psychology/chemistry/coach/referee subsystems, and **rayon
parallelism + tokio + runtime LLM hooks** — architecturally everything we want
to *avoid coupling to*, but a goldmine of design patterns we should
selectively port. The right strategy for T1 is to **steal OFM's match-skeleton
shape and event grammar** (it fits our 3000-LoC budget cleanly), then
**steal ZOXEXIVO's domain subsystems as design references** (psychology,
chemistry, calibration harness, role-state-machines) WITHOUT importing their
non-deterministic implementation choices. Neither project is a reference
implementation — they're inverse poles, and FW slots between them with
determinism, schema versioning, and bake-time content as our differentiators.

## Project profiles

### openfootmanager

- 57K LoC Rust + TypeScript. 4-crate workspace: `domain`, `db`, `engine`,
  `ofm_core` (`src-tauri/Cargo.toml:18`). Engine crate is ~6.3K LoC total.
- Edition 2024 on `engine` + `ofm_core`. Engine deps are minimal: `log`, `rand`,
  `serde` only (`crates/engine/Cargo.toml`).
- Match-sim is **minute-driven, zone-driven, action-resolution** (1-3 actions
  per minute, possession contest at minute end —
  `engine/src/live_match/simulation.rs:155-201`). Zones are a 5-enum
  (`Midfield`, `attacking_third`, `attacking_box`, etc.). 30-ish event types
  (`engine/src/event.rs:17-63`).
- Tauri+React+SQLite+i18n. Hand-rolled live-match loop in the frontend with
  speed control (`src/components/match/MatchLive.tsx:33-80`).
- Optimizing for: **legibility + ship-ability**. Solo-dev hobby pace. No
  determinism guarantees, no benchmarks, no calibration harness.

### ZOXEXIVO/open-football

- 217K LoC Rust + Askama templates. 3-crate workspace (`core`, `database`,
  `web`) with **65K LoC just in `src/core/src/match/`**.
- Edition 2024, but uses `tokio = "full"`, `rayon`, `nalgebra` for 3D vectors,
  `dashmap`-style sharing throughout (`Cargo.toml`, `src/core/Cargo.toml`).
- Match-sim is **10ms-tick spatial simulation** with Vector3 positions
  (`engine/context.rs:15` — `MATCH_TIME_INCREMENT_MS: u64 = 10`). Per-role state
  machines: ~21 states per outfield role, ~21 for keepers
  (subdirs under `engine/player/strategies/{forwarders,midfielders,defenders,
  goalkeepers}/states/`). Reynolds-style steering (`Seek/Arrive/Pursuit/Evade/
  Wander/Flee/FollowPath` — `engine/player/behaviours/steering.rs:4-34`).
- Has rich subsystems: chemistry (412 LoC), coach (517), psychology (469),
  referee (281), set_pieces (876), tactical (1576), rating (2231) — they live
  alongside the engine in `src/core/src/match/engine/`.
- **Has runtime LLM hooks** (`src/core/src/ai/mod.rs:13-23` — `AiService` trait
  with `execute_batch`). LLMs decide club behaviour at runtime.
- Optimizing for: **autonomous world simulation at scale** (256-core
  benchmarks in README). Match engine is sovereign; UI is read-only HTTP.

## Top 5 — STEAL from openfootmanager

1. **The minute-by-minute action-resolution skeleton** —
   `engine/src/live_match/simulation.rs:155-201`'s `play_minute` is exactly the
   shape FW's T1 first-match wants: increment minute → deplete stamina → resolve
   1-3 actions → possession contest → check phase transitions → return
   `MinuteResult`. It's ~50 lines, deterministic given the RNG, and self-evident.
2. **Zone-routed action dispatch** —
   `engine/src/live_match/zone_resolution.rs:14-28`'s `resolve_action` switches
   on `ball_zone` and dispatches to `resolve_shot` / `resolve_attacking_third`
   / `resolve_midfield` / `resolve_buildup`. Trivial to make deterministic in
   Q32, and maps onto our planned 2D-tactical-board zones 1:1.
3. **The 30-event-type flat enum with optional player IDs** —
   `engine/src/event.rs:6-63`. `MatchEvent` is a struct with `minute`,
   `event_type`, `side`, `zone`, `player_id: Option<String>`,
   `secondary_player_id: Option<String>`. Builder pattern (`with_player`,
   `with_secondary`). FW should adopt this shape almost verbatim, but with
   typed `PlayerId(u32)` and `Tick(u32)` instead of strings + minutes.
4. **`MinuteResult` as the IPC return type** —
   `engine/src/live_match/simulation.rs:23-32` — every tick returns a struct
   carrying minute, phase, new events, scores, possession, ball-zone,
   `is_finished`. Maps cleanly to a Tauri `step_live_match(minutes: u32)`
   command and the frontend's incremental render. This is the pattern for our
   first IPC contract.
5. **Trait-bonus + play-style modifier as separable functions** —
   `crates/engine/src/shared.rs` exposes `trait_bonus(player,
   TraitContext::X)` and `play_style_modifier(style, phase, attacking)`. Pure
   functions, easy to test, easy to swap. We should mirror this pattern for
   FW's `signature_bonus` + `tactic_modifier` so personality bias and play
   style stay decoupled from the action resolver.

## Top 5 — AVOID from openfootmanager

1. **`f64` arithmetic throughout the resolver** — `zone_resolution.rs:50, 96,
   178, 245` etc. all do `f64` skill blends and `rng.random_range(0.0..1.0f64)`
   gates. We can't carry this in. Q32 with the same expression shape works
   fine; the design lesson transfers, the implementation does not.
2. **`String` player IDs threaded everywhere** — `event.rs:12`,
   `live_match/zone_resolution.rs:361` (`self.sent_off.insert(fouler_id.
   to_string())`). Every event allocates. FW's `PlayerId(u32)` newtype + content-
   pack-qualified resolution at the boundary is strictly better.
3. **`HashMap` in the report crate** — `engine/src/report.rs:2, 88, 123, 307`.
   The `player_stats: HashMap<String, PlayerMatchStats>` will hash-randomize
   iteration order and break canonical-state hashing on the spot.
4. **No determinism contract** — there's no seed pinning, no canonical-state
   hash, no cross-OS test matrix. Two OFM runs of the "same" match on the
   same machine probably *are* deterministic given a seeded RNG, but nothing
   in the test surface (`crates/engine/tests/simulation_tests.rs`,
   `live_match_tests.rs`) asserts byte-equality of state. We have to be
   stricter from day one.
5. **The minute-resolution time budget hides positional play** —
   `play_minute` runs 1-3 actions per minute. That's fine for text commentary
   but fundamentally cannot express "the ball was 5 metres off the byline
   when the cross came in" — which our 24-signature-move pillar may eventually
   want. Don't bake the minute-tick assumption into types we'll need to
   subdivide later. (Our research synthesis already calls for 4Hz BT; just
   note this is why OFM can't do what we plan.)

## Top 5 — STEAL from ZOXEXIVO

1. **The per-role state-machine taxonomy** —
   `src/core/src/match/engine/player/strategies/{forwarders,midfielders,
   defenders,goalkeepers}/states/` each have ~21 sub-states (Running,
   CreatingSpace, Pressing, Tackling, Returning, Walking, Finishing, Heading,
   TakeBall, etc.). This is a *catalogue of legible behaviours* that maps
   directly onto our BT-leaf actions. Don't import their implementation —
   import the *taxonomy*.
2. **The `StateProcessingHandler` trait pattern** —
   `engine/player/strategies/processor.rs:18-29`: every state implements
   `process(ctx) -> Option<StateChangeResult>`, `velocity(ctx) ->
   Option<Vector3>`, `process_conditions(ctx)`. Clean separation between
   "what state am I in" (transitions) and "what force am I applying" (steering).
   FW's BT-leaf trait should have the same triple.
3. **The match calibration harness** — `src/core/src/match/calibration.rs:21-60`
   defines `MatchCalibrationStats` with 30+ accumulators (goals, shots, xG,
   passes, dribbles, pressures, errors_leading_to_shot, etc.) that aggregate
   across N matches and a `report_lines` API with acceptance ranges. This is
   exactly the rig we need to validate that simulated leagues produce
   plausible numbers. Build a leaner version as part of T2/T3.
4. **PsychologyState as an opt-in per-player overlay** —
   `engine/psychology.rs:17-32` has `PsychState { confidence, nervousness,
   momentum_boost, mistake_memory_tick, goal_involvement_tick }` per player,
   clamped at consumer side, modifying skill via `SkillModifiers`
   (psychology.rs:58-73). Maps directly onto our 8-element personality bias
   vector idea. The pattern of "transient state + skill_modifiers translator"
   is clean.
5. **Steering primitives as an enum, not a trait** —
   `engine/player/behaviours/steering.rs:4-34` defines `SteeringBehavior` as a
   7-variant enum (`Seek`, `Arrive`, `Pursuit`, `Evade`, `Wander`, `Flee`,
   `FollowPath`). Each variant carries its parameters. `calculate(&self,
   player) -> SteeringOutput`. Easier to serialize, easier to test, easier to
   reason about than a `Box<dyn SteeringBehavior>`. This matches our
   `Rust/RULES.md` preference for monomorphic generics over dyn.

## Top 5 — AVOID from ZOXEXIVO

1. **`HashMap` in the sim** — `engine/engine.rs:29` imports
   `std::collections::HashMap` and uses it for the player-lookup map (`:1944,
   1951, 1955`). Same problem as OFM but worse because the engine is much
   bigger. Our `BTreeMap`-only rule is non-negotiable; do not back-port their
   structures.
2. **`f32`/`f64` in canonical state + `nalgebra::Vector3<f32>` everywhere** —
   `engine/engine.rs:55-66` (the `SkillAccumulator`), `engine/steering.rs`
   (every Vector3). Cross-platform float drift will guarantee canonical-hash
   drift across our macOS/Windows/Linux CI matrix. Q32 (and `cordic` for trig)
   is mandatory; do not adopt nalgebra in `fw-match-sim`.
3. **`rayon` + `tokio` + parallel match running** —
   `src/core/src/club/board/manager_market.rs:30, 625-630, 713-715` uses
   `par_iter` heavily, and `simulator/mod.rs:60-64` is `async fn simulate`.
   Rayon's work-stealing thread pool is a determinism hazard (ordering of
   reductions). `Sim/RULES.md §10` bans both. Their parallelism unlocks 256-
   core throughput; we trade that for reproducibility.
4. **Runtime LLM via `AiService` trait** — `src/core/src/ai/mod.rs:13-23` plus
   `request.rs` defines `PendingAiRequest` with `query: String, format:
   String, handler: AiResponseHandler` — the simulator core calls out to LLMs
   at runtime. This directly violates FW's "AI is bake-time only" pillar
   (CLAUDE.md §3). Don't import the pattern even partially.
5. **The 2321-LoC `engine.rs` and 2231-LoC `rating.rs` files** —
   `engine/engine.rs` (2321), `engine/rating.rs` (2231), `engine/tactical.rs`
   (1576). These are walls of code with very high incidental complexity. We
   have a 3000-LoC budget for the entire match-sim crate; this is the
   anti-pattern. Their internal organisation (one giant impl block per
   subsystem) does not split cleanly. Don't model `fw-match-sim` on it.

## Three things FW does better than both

1. **Determinism contract is binding, not aspirational.** Both projects use
   seeded RNGs but neither pins canonical-state hashes across OSes. Both use
   `f32`/`f64` in canonical state; OFM uses `HashMap` in the report crate and
   ZOXEXIVO in the engine. FW's `Sim/RULES.md §1-§10` is enforced by clippy
   (`#[deny(clippy::float_arithmetic)]`), the pinned regression test
   (`fw-replay/tests/canonical_hash.rs`), and the cross-OS CI matrix. Neither
   competitor could ship a "replay this match byte-identical on a Steam Deck
   five years later" feature. We can.
2. **Schema-versioned content packs with forward migrations.** Neither
   project has the content-pack-qualified ID scheme (`fwh.core:player_00042`),
   versioned RON schemas, or bake-time LLM authoring committed as RON. OFM
   has SQLite saves; ZOXEXIVO has a separate `open-football-database` repo
   with `serde_json` blobs. FW's `fw-content` migration story
   (`crates/fw-content/src/migrations/<N>_to_<N+1>.rs`) is meaningfully more
   stable.
3. **Sim-sovereign IPC boundary.** Both projects let the runtime layer drive
   state (OFM's React component calls `step_live_match` and stores state
   client-side; ZOXEXIVO's web crate reads database snapshots). FW's `Tauri/
   RULES.md §2` makes the sim sovereign and the UI strictly a read-projection
   — a tighter contract that prevents UI bugs from corrupting canonical state.
   This is invisible until phase-7 multiplayer-spectator-replay work, when it
   becomes load-bearing.

## Three concerns FW shares with both

1. **The "single resolver function gets fat" gravity well.** OFM's
   `zone_resolution.rs` is already 383 lines for a minute-tick resolver. ZOXEXIVO's
   `engine.rs` is 2321 lines. Both started small. FW's 4Hz BT runner +
   utility-scorer + 24 signature moves will pull the same way. Plan for
   sub-module splits *before* the first resolver lands.
2. **Calibration drift over content-pack additions.** ZOXEXIVO needed a
   `MatchCalibrationStats` rig (`calibration.rs:21-60`) precisely because
   30-system match engines drift unpredictably when subsystems land. OFM
   doesn't have one yet and its tests can't catch league-level drift. FW will
   need a calibration harness by T3 at the latest — every signature-move add,
   every salience-weight tweak, every BT-leaf add will need a "does the
   league still produce realistic numbers" gate.
3. **Match-engine vocabulary leaks into UI.** OFM exposes `EventType` enum
   variants directly to the React component (`MatchLive.tsx` switches on
   them). ZOXEXIVO has 80+ player states the rating system has to translate
   into displayable form. FW's banned-terms lint + "internal floats stay
   invisible to players" rule (`CLAUDE.md §7`) is correct, but enforcing it
   on every new sim subsystem will be ongoing work, not a one-shot.

## Recommendation for /next T1-1

**Proceed as planned, with one schema-design adjustment.** OFM validates that
a 30-event-type flat enum + `MinuteResult`-shaped IPC return + zone-routed
action dispatch is a *sufficient* skeleton for a first-match milestone in a
2-3K LoC budget — and that's an extremely strong existence proof for our T1
scope. ZOXEXIVO validates that the per-role state-machine taxonomy + steering
enum + psychology/chemistry overlays + calibration harness are the right
*next-layer* primitives once the skeleton holds. The adjustment: when we
schema the T1 `MatchEvent` (analogue of `crates/engine/src/event.rs:17-63`),
include from day one a `player_id: PlayerId` newtype (not `String`), a
`tick: Tick(u32)` field (not `minute: u8`), and a non-optional `zone:
TacticalZone` enum keyed by attacking side — so we don't have to migrate the
event shape when T1-2b BT leaves and 4Hz ticking land. Everything else stays
on the existing T1-1 plan.

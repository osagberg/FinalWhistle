# DX-2 — Match-quality inspection: frame glitch-detectors + contact-sheet renderer

Status: DONE (2026-06-04). Owner: `gameplay-programmer`.

## Match length + tick→minute mapping (determined at DX-2)

A full match is **5400 ticks**. Constant: `fw_match_sim::FULL_MATCH_TICKS = 5400`.

- **dt = 1/60 s per tick** (60 ticks per simulated second).
- **90 minutes × 60 ticks/minute = 5400 ticks.**
- **tick → minute: `minute = tick / 60` (integer division).**
- `FullTime` fires when `state.tick >= state.match_end_tick` (default `Tick::from_raw(5400)`).
- The 900-tick runs in DX-1 were slices: 900/60 = 15 simulated minutes, not a full match.

## Thresholds (v1, updated 2026-06-04 REVISE pass)

**IMPORTED** = value pulled from a canonical sim constant at compile time (drift-safe).
**PHYSICAL CAP** = derived from engine physical caps (max speed × dt = 1/60 s); no sim constant.

| Threshold | Value | Source | Derivation / rationale |
|---|---|---|---|
| `GOAL_LINE_X` | 52.5 m | IMPORTED: `fw_core::GOAL_LINE_X` | Pitch half-length |
| `SIDELINE_Y` | 34.0 m | IMPORTED: `fw_core::SIDELINE_Y` | Pitch half-width |
| `GOAL_HALF_WIDTH` | ≈3.66 m | IMPORTED: `fw_content::event::GOAL_HALF_WIDTH_M` | Half of 7.32 m standard goal |
| `MIN_PLAYER_DISTANCE` | 0.4 m | IMPORTED: `fw_match_sim::separation::MIN_PLAYER_DISTANCE` | Separation invariant |
| `FULL_MATCH_TICKS` | 5400 | IMPORTED: `fw_match_sim::FULL_MATCH_TICKS` | 90 min × 60 ticks/min |
| `MAX_BALL_TRAVEL_PER_TICK` | 1.0 m | PHYSICAL CAP | Peak shot 35 m/s × 1/60 s = 0.583 m; 1.0 m = 1.7× cap |
| `PLAYER_RADIUS` | 0.5 m | PHYSICAL CAP | Approximate player body radius for phasing contact proxy |
| `MAX_PLAYER_TRAVEL_PER_TICK` | 0.15 m | PHYSICAL CAP | Max player 8 m/s × 1/60 s = 0.133 m; 0.15 m = 1.1× cap |
| `OVERLAP_K_TICKS` | 5 ticks | PHYSICAL CAP | Transient (1 tick) OK; 5 consecutive = persistent bug |
| `OFF_PITCH_K_TICKS` | 5 ticks | PHYSICAL CAP | Brief off-pitch during restart OK; 5 consecutive = bug |
| `STALL_WINDOW` | 60 ticks | PHYSICAL CAP | 1 simulated second of total freeze |
| `STALL_MOTION_THRESHOLD` | 0.01 m/tick | PHYSICAL CAP | Per-entity threshold for "effectively frozen" |
| `PHANTOM_GOAL_WINDOW` | ±10 ticks | PHYSICAL CAP | ±0.167 s around score change |

`systems-designer` owns final tuning of the PHYSICAL CAP values; the IMPORTED values
track their source constants automatically and must not be duplicated as literals.

## Problem

The DX-1 harness lets an agent watch a match in the browser, but you can only
screenshot a frame at a time — you cannot see, at a glance, that the dots aren't
really playing football, that the ball teleports into the goal, or that it phases
through players. Manual per-tick screenshotting does not scale and misses most of
the match. We have a deterministic engine; the right move is to turn "watch for
glitches" into **computable functions over the frame stream**, and to render a
whole match as **one viewable artifact**.

This is also the substrate for the automated fun-evaluation (drama metrics +
LLM-judge): the same frame data feeds both glitch-detection and drama-scoring.

## Inputs

`dump_frames` already emits the full per-tick canonical projection:
`Vec<MatchFrameDto>` — every tick's 22 player positions + the ball position and
velocity (+ score, + events). One seed × N ticks = one JSON file. Everything below
is a PURE FUNCTION of that JSON. No sim change; no canonical-state touch; both pins
stay byte-identical.

## Tool A — glitch-detectors

A dev analyzer (new bin, e.g. `crates/fw-replay/src/bin/inspect_frames.rs`, or a
`scripts/fw inspect-frames` front-door) that reads a frames JSON and emits a
structured report: per-detector flag counts + the first offending tick + detail,
and an aggregate "glitch rate" when run across many seeds. Each detector is a pure
function over consecutive frames returning `Vec<Flag { tick, kind, detail }>`.

Detector set (v1):

1. **Ball teleport** — `|ball.pos(t) − ball.pos(t−1)| > MAX_BALL_TRAVEL_PER_TICK`.
   Catches the ball jumping discontinuously (e.g. "glitches into goal").
2. **Ball phasing through a player** — the swept ball segment `[pos(t−1) → pos(t)]`
   passes within `PLAYER_RADIUS` of a player, but no touch / possession-change /
   interaction event fired that tick. Catches the ball going through bodies.
3. **Goal without crossing the line** — score increments at tick `t`, but the ball
   path in the surrounding window never crossed the goal-line x within the
   goal-mouth y-range. Catches phantom goals.
4. **Persistent player overlap** — two players closer than `MIN_PLAYER_DISTANCE`
   (the separation invariant, 0.4 m) for `> K` consecutive ticks. Transient overlap
   during separation resolution is fine; persistent is a bug.
5. **Impossible player velocity** — player position delta `> MAX_PLAYER_TRAVEL_PER_TICK`.
6. **Ball off-pitch** — ball outside pitch bounds with no restart event in-window.
7. **Stall** — ball + players effectively frozen for a long stretch while in play
   (the T4-sim-halt class, as a regression guard).

Thresholds (`MAX_BALL_TRAVEL_PER_TICK`, `PLAYER_RADIUS`, `MIN_PLAYER_DISTANCE`,
`MAX_PLAYER_TRAVEL_PER_TICK`, `K`, stall window) are tuning constants derived from
the engine's physical caps (max ball/player speed × dt = 1/60 s) → `systems-designer`
owns the values; they live here, not in SPEC.

Determinism payoff: every flag is reproducible from `(seed, tick)`, and the analyzer
runs across a full match × thousands of seeds to produce a glitch-rate that a tuning
change can be measured against — exactly the controlled-experiment loop determinism
buys.

## Tool B — GIF / contact-sheet filmstrip

A headless renderer (Node script under `frontend/scripts/`, or a Rust `image`-crate
bin) that turns a frames JSON into ONE viewable artifact:

- **Animated GIF** — pitch + colored dots (home/away) + ball, one GIF frame per
  every Nth tick, so the whole match's motion reads in a single file.
- **Contact-sheet PNG** (alternative/companion) — a grid of mini-pitches (every Nth
  tick), so motion is scannable without playback.

It does not need to be pretty — just legible enough to judge "are they playing
football or drifting?" and to spot the emergent weirdness the detectors don't
encode. The output is a file an agent reads directly (image read) or a human opens.
Reusing the production `<TacticalBoard>` draw logic headlessly is possible but
PixiJS-headless is painful; a standalone pitch+dots renderer is simpler and
sufficient.

## Acceptance

- The detectors FIRE on the current known-broken match (the DX-1 5-5 / 900-tick run
  already shows teleport-class and clustering symptoms) and on a deliberately
  glitched fixture; they stay quiet on a clean reference run.
- The GIF/contact-sheet renders a full match legibly from a frames JSON.
- Both tools are pure functions of `dump_frames` output; `scripts/fw verify` green;
  both canonical pins UNCHANGED; no sim/canonical change.

## DX-2 findings — broken match (seed 0xfeedbeefcafefade, 5400 ticks)

Run date: 2026-06-04 (v1 detectors). Updated after REVISE pass (v1.1 detectors — same
match, corrected BallPhasingPlayer logic, added evaluable_frame_pairs + warnings fields).

v1 (initial) run:

| Detector | Flags | First flag |
|---|---|---|
| BallTeleport | 249 | tick 8: ball jumped 9.797m |
| BallPhasingPlayer | 747 | tick 9: segment within 0.130m of slot 9, no possession change |
| PhantomGoal | 0 | — |
| PersistentPlayerOverlap | 40 | tick 4: slots 6+20 overlap 5+ ticks |
| ImpossiblePlayerVelocity | **32,737** | tick 1: slot 5 moved 0.279m/tick |
| BallOffPitch | 0 | — |
| Stall | 0 | — |
| **Total** | **33,773** | |

v1.1 numbers: see "Re-run after REVISE" section below.

Key findings:

1. **ImpossiblePlayerVelocity dominates (32,737 flags out of 33,773 total).** The velocity cap in `dispatch.rs` is 8 m/s, which is 0.133 m/tick at dt=1/60 s. Players are exceeding this constantly. The dispatcher applies a `clamp_velocity_component` but something is setting positions directly (not via velocity) — likely the `MoveToPosition` / `HoldFormation` intents that overwrite `vel_x`/`vel_y` directly and the integration step adds the full velocity in one tick regardless of distance. This means the position-delta threshold of 0.15m needs revisiting relative to the actual position-update code path, OR the position updates are bypassing the velocity cap entirely. **This is the primary fix target for FUN-0.**

2. **BallTeleport (249 flags): ball jumps ~10m on tick 8.** The kick-off ball placement sets ball at (9.8, 0) directly — this is a teleport from center at (0,0). Not a physics bug; it's an initialization step. Still, it flags as a teleport. The detector is working correctly; FUN-0 may want to skip tick 1 for ball-teleport or distinguish initialization teleports.

3. **BallPhasingPlayer (747 flags): ball sweeps through players without possession change.** Given the high ImpossiblePlayerVelocity rate (players moving ~2x the physical cap), ball-player proximity events are frequent. This is a downstream symptom of the velocity/position bug.

4. **PersistentPlayerOverlap (40 flags): separation module not holding.** Players collide and the separation resolver is not keeping them apart for 5+ ticks. A secondary symptom of the velocity issue.

5. **PhantomGoal = 0, BallOffPitch = 0, Stall = 0:** goals ARE correlating with ball crossing the line; the ball stays on pitch; and the match isn't stalling. These are positives.

Contact-sheet PNG: `target/contact-sheet-feedbeefcafefade.png` (generated 2026-06-04).
Inspect with `scripts/fw inspect-frames <frames.json>`.

### Re-run after REVISE (v1.1 — corrected phasing logic, canonical constant imports)

Run date: 2026-06-04. Seed 0xfeedbeefcafefade, 5400 ticks, 5401 frames, 5400 evaluable pairs.
Status: OK. Warnings: BallPhasingPlayer lower-bound caveat only.

| Detector | v1 flags | v1.1 flags | Delta | Notes |
|---|---|---|---|---|
| BallTeleport | 249 | 249 | — | Unchanged |
| BallPhasingPlayer | 747 | **970** | +223 | Higher: v1 suppressed entire frames on possession change; v1.1 checks uninvolved players. The fix surfaces 30% more phasing events. Count is still a lower bound. |
| PhantomGoal | 0 | 0 | — | Goals correlate with goal-line crossing |
| PersistentPlayerOverlap | 40 | **34** | −6 | Slightly lower: v1.1 keys `reported` set on slot pairs not array indices; re-reporting semantics changed slightly |
| ImpossiblePlayerVelocity | 32,737 | 32,737 | — | Unchanged — dominant bug |
| BallOffPitch | 0 | 0 | — | Ball stays in bounds |
| Stall | 0 | 0 | — | No freezes |
| **Total** | **33,773** | **33,990** | +217 | |

Key delta: BallPhasingPlayer increased 747→970 (+30%) because the P1 fix no longer
goes blind when possession changes — uninvolved players are now correctly checked.
The first flag shifted from tick 9 (slot 9, no possession change) to tick 8 (slot 17,
during a 9→8 possession change, correctly flagging an uninvolved third party).

### Known limitations / deferred follow-ups (do NOT implement now)

- **Detector-name String→enum refactor.** Today detector names are `String` values;
  a typo in a filter would silently match nothing. Defer until there are 3+ callers
  that pattern-match on names.
- **Stall "in-play" gate is score-only.** A score change in the window suppresses the
  stall flag, but a match that stalls at 0-0 indefinitely still fires correctly. The
  edge case (stall during a dead-ball restart where score doesn't change) is deferred.
- **contact-sheet/overlap 11-vs-n_players seam.** The renderer hardcodes `slot < 11`
  for home/away colour, which is correct for the canonical 22-player layout but would
  mislabel a shorter test fixture. Defer until a fixture with non-22 players exists.

## Why this is the fun-evaluation substrate

Glitch-detectors answer "is it physically coherent?"; the GIF answers "does it look
like football?"; the drama metrics (fun-pivot) answer "is it gripping?". All three
are computed from the same deterministic frame/event stream, which is how the
automated fun-evaluation (and eventually director mode) judges a match without a
human in the loop.

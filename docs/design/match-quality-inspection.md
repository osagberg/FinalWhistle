# DX-2 — Match-quality inspection: frame glitch-detectors + GIF filmstrip

Status: SPEC (TODO). Owner split: `gameplay-programmer` (detectors) + `ui-programmer`
(renderer) + `systems-designer` (thresholds). Depends on DX-1 (the dev play-harness)
only conceptually; the tooling reads `dump_frames` output directly and needs no server.

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

## Why this is the fun-evaluation substrate

Glitch-detectors answer "is it physically coherent?"; the GIF answers "does it look
like football?"; the drama metrics (fun-pivot) answer "is it gripping?". All three
are computed from the same deterministic frame/event stream, which is how the
automated fun-evaluation (and eventually director mode) judges a match without a
human in the loop.

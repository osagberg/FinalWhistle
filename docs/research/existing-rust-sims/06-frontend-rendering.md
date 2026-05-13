# 06 — Frontend rendering & match viewers

Compares OpenFootManager (OFM; React 19 + Tauri 2) and ZOXEXIVO's open-football
(server-rendered Askama + PixiJS) to inform T1-2a's SolidJS + PixiJS v8 tactical
board.

Bottom line: **only one of the two projects renders a live 2D pitch — open-football
does, OFM does not**. Open-football's player is the more relevant reference and
is, conveniently, also Pixi-based. OFM is the better reference for the IPC
plumbing (Tauri command + `step_live_match` + snapshot pull).

---

## OFM frontend match-rendering

**Stack:** React 19, Tauri 2 IPC, Tailwind v4, Zustand, react-router v7,
i18next. No canvas/SVG/Pixi anywhere in the live-match path
(`/Users/vibelogic/dev/openfootmanager-readonly/package.json:14-30`).

**There is no pitch viewer.** `MatchLive.tsx` is a scoreboard + tabbed
event-feed UI with no spatial rendering at all
(`/Users/vibelogic/dev/openfootmanager-readonly/src/components/match/MatchLive.tsx:172-409`).
The `MatchSnapshot` DTO carries `ball_zone: string` (a zone label) and per-team
formations, never XY coordinates
(`/Users/vibelogic/dev/openfootmanager-readonly/src/components/match/types.ts:63-87`).
The closest thing to a pitch view is `components/tactics/TacticsPitch.tsx` — a
CSS-grid lineup-formation board for pre-match setup using `<button>`s arranged
by `getPitchRowWidth` / `getPitchSlotWidth`
(`/Users/vibelogic/dev/openfootmanager-readonly/src/components/tactics/TacticsPitch.tsx:206-262`),
plus a "mini pitch visualization" in `SubPanel.tsx:236` — also static lineup
slots, not live positions.

In other words: OFM is text-first, zone-coarse, no spatial layer. That's a real
design choice, not an absence we should patch.

## ZOXEXIVO web frontend overview

Server-rendered HTML via Askama templates (`askama.toml`, `layout.html`,
`*/get/index.html` per area). The match page is the lone client-heavy view: a
~720-line inline `<script>` block driving PixiJS v8
(`/Users/vibelogic/dev/open-football-readonly/src/web/src/match/get/index.html:222-717`).
Everything else (league tables, squad pages, player history) is static SSR.

Match-recording is opt-in behind `--match-recording-enabled`; if disabled the
page shows a placeholder
(`/Users/vibelogic/dev/open-football-readonly/src/web/src/match/get/index.html:113-121`).
So live tactical view is gated behind explicit flags even in their own product.

## IPC + state-streaming patterns

**OFM — polling-with-step (Tauri command request/response):**
- `invoke<MinuteResult[]>("step_live_match", { minutes: 1 })` per timer tick
  (`MatchLive.tsx:58`).
- After step, full snapshot pull: `invoke<MatchSnapshot>("get_match_snapshot")`
  (`MatchLive.tsx:73`).
- Timer cadence driven by `SPEED_MS`: paused / 2000 / 800 / 200 / 10ms
  (`types.ts:198-204`). "Instant" is just a 10ms timer, not a one-shot.
- Commands also work the other way — user intents go via
  `invoke("apply_match_command", { command: { Substitute: {...} } })`
  (`MatchLive.tsx:138-141`). Returns a fresh snapshot. UI never owns state.

**Open-football — chunked-replay fetch (HTTP, post-match):**
- Match recording is **written to disk after the match completes**, not
  streamed live. Frontend fetches `/api/match/{id}/metadata` then
  `/api/match/{id}/chunk/{n}` lazily as the scrubber crosses chunk boundaries
  (`index.html:296-307`, `chunk.rs:27-66`).
- Default chunk duration is 300_000 ms (5 minutes of match time;
  `chunk.rs:90`); compact per-frame format is
  `[timestamp_ms, x, y, z?]` (`index.html:309-310`).
- Chunks served **gzip-encoded** straight from disk (`chunk.rs:50-63`). Two
  chunks pre-fetched: current + next (`index.html:498-502`).

That's the key split: **OFM is live request/response with the sim sovereign on
the Rust side; open-football is post-match replay-from-blob**. Our T1-2a wants
live, so OFM's IPC pattern is the model — pulling deltas via a Tauri command at
a UI-driven cadence.

## Render perf observations

Open-football's player is a tight Pixi v8 implementation worth borrowing
verbatim:
- **30fps target with throttle gate** — `TARGET_FPS = 30`, `FRAME_INTERVAL =
  1000/30`; tick exits early if elapsed < frame interval
  (`index.html:271-274, 481-486`).
- **Pixi `Application` created once in `init()`**; `app.ticker.add(tick)`
  drives the loop (`index.html:598-606, 685-687`). No re-init on data update.
- **Temporal-coherence index hints** — each entity's last-used array index is
  remembered (`lastBallIdx`, `lastPlayerIdx[id]`) so `findIndexNear` scans 1-2
  steps in the common case and only falls back to binary search on scrubber
  jumps (`index.html:267-269, 361-386`).
- **Player graphics built once, mutated in place** — `createPlayerGraphic`
  returns a `PIXI.Container` with border, fill, shirt number, name (text
  re-rendered only if changed); render loop updates only `position.x/y` and a
  z-derived `scale` (`index.html:406-442, 519-563`).
- **Position interpolation between samples** — `interpolatePosition` linearly
  blends `(timestamp, x, y, z)` pairs so 30fps render is smooth even when
  sample data is sparser (`index.html:388-404`).
- **Background SVG loaded as a Pixi Sprite once**, scaled via
  `app.stage.scale.set(scale)` on resize (`index.html:611-615, 690-713`).

## What FW T1-2a should adopt

1. **Pixi v8 lifecycle: init once, mutate forever.** Pixi `Application` in
   `onMount`, `app.destroy()` in `onCleanup` (Solid equivalent of OFM's
   pattern). Confirms `.claude/rules/Frontend/RULES.md` §4. The render loop is
   the Pixi ticker, never a Solid `createEffect`.
2. **30fps target + frame-interval gate.** 22 dots + ball + a pitch sprite is
   well under what 60fps Pixi can do, but 30fps halves CPU; for a dev surface
   that's the right trade-off, especially on the macOS dev box.
3. **Sample-and-interpolate, not sample-per-frame.** The sim emits a
   `MatchFrameDTO` per tick (sub-second), the renderer interpolates
   between the two bracketing frames at 30fps. Decouples sim cadence from
   render cadence — exactly what the determinism gate wants.
4. **OFM's IPC shape, slimmed.** A `step_match` command that returns a delta
   plus a `get_match_frame(tick)` that returns the full positional state. Tick
   scrubber pulls `get_match_frame(t)`; live mode auto-increments. UI never
   pushes canonical state back (matches `.claude/rules/Tauri/RULES.md` §2).
5. **Temporal-coherence indexing on the scrubber.** When we add the tick
   scrubber, the renderer remembers the last frame index per entity for
   O(1) advance; binary-search fallback only on jumps (`index.html:361-386`
   is a clean 25-line reference).
6. **Compact frame format.** `[tick, x, y]` tuples (Q32 → f64 at the DTO
   boundary per `Tauri/RULES.md` §3) keep the wire payload tiny vs.
   per-player named objects.

## What FW T1-2a should avoid

1. **Don't follow OFM's full-snapshot-pull cadence for a 22-dot live view.**
   `invoke<MatchSnapshot>("get_match_snapshot")` at 800ms is fine for an event
   feed; at 30fps it's a serialize-everything-every-frame anti-pattern. Send
   compact frame DTOs only.
2. **Don't follow open-football's chunked-replay model for live play.** It's a
   post-match player; chunks come from disk after the match finishes. We need
   live, and live + 5-minute chunks would feel like a YouTube buffer.
3. **Don't put the Pixi render loop inside a Solid effect.** Same trap as in
   React with `useEffect` — every signal change tries to re-init. Pixi ticker
   only; Solid signals only feed the scene-graph mutators
   (`Frontend/RULES.md` §4 already calls this out).
4. **Don't rebuild scene graph per frame.** Open-football builds player
   graphics once at `startMatch` and on substitute-on
   (`index.html:340-350, 624-635`); per-frame work is just `position.x/y` +
   `scale.set`. A naive "rebuild children on data change" approach falls
   apart fast with 23 sprites.
5. **Don't serialize positions as named maps if you can help it.** Open-football
   sends `players: { "<id>": [[t,x,y,z], ...], ... }` (`index.html:329-355`).
   That's fine for replay; for live, prefer dense parallel arrays indexed by
   slot — saves both JSON and `Object.entries` cost in the hot path.
6. **Don't share text rendering with position rendering in the same Graphics
   object.** Open-football builds a `PIXI.Container` per player with separate
   `Graphics` for border/fill and separate `Text` for number/name
   (`index.html:519-563`). One re-layout of text shouldn't trigger a fill
   re-draw.

## Open questions

- **Frame DTO cadence vs. tick cadence.** Sim emits per-tick events; do we
  send a positional snapshot every tick (e.g. 1Hz match seconds) or every N
  ticks? Affects scrubber granularity and IPC bandwidth. Open-football's
  per-sample timestamps suggest "as often as positions meaningfully change"
  with renderer interpolation filling gaps.
- **Scrubber rewind semantics.** Open-football's scrubber is a pure
  replay-time index — they can scrub forward and backward freely because all
  positions are pre-baked. FW's determinism gives us replay-from-seed for
  free, but jumping backwards mid-live-match either replays from kickoff or
  caches the live frames in memory. Probably the latter, capped at the
  match's tick count.
- **Pre-T4 dev surface vs. shipped polish.** The dev-verification version of
  T1-2a wants the simplest thing that proves "the sim says where the players
  are." That's probably 22 colored circles on a green rectangle, no
  animation curves, no text labels. Open-football's full implementation is
  what T4 polish looks like, not what T1-2a Day-1 looks like.
- **SolidJS-specific Pixi gotchas.** OFM dodges this by not rendering. We'll
  need to confirm the Pixi-init-in-`onMount` / cleanup-in-`onCleanup` pattern
  doesn't trip Solid's HMR. (Probably fine; flagged for T1-2a kickoff.)

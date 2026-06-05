/*
 * Production PixiJS v8 tactical board — T4-1.
 *
 * Renders a FIFA pitch + 22 player dots + 1 ball from `MatchFrameDTO[]` with
 * smooth interpolated playback driven by the Pixi ticker.
 *
 * Playback model:
 *   Canonical frames are recorded at 60Hz (1 frame per sim tick). The ticker
 *   advances a fractional "playback cursor" each frame using wall-clock delta
 *   time at 1× real-time. Player/ball positions are LERP-ed between the two
 *   bracketing frames (floor + ceil) on every ticker step. This eliminates the
 *   jitter that discrete frame snapping would produce.
 *
 *   The lerp is display-only: it only interpolates screen positions; it NEVER
 *   mutates or re-simulates canonical state. The board is a read-only view.
 *   (CLAUDE.md §7 "UI never drives canonical state".)
 *
 * Controls: play/pause button + a tick scrubber. Speed controls (1×/4×/16×)
 * are T4-5 — out of scope here.
 *
 * Solid lifecycle:
 *   Application created ONCE in onMount (await Application.init).
 *   Ticker added in onMount, removed + app destroyed in onCleanup.
 *   `destroyed` flag guards the HMR race where onCleanup fires before the
 *   async init promise resolves.
 *   Signal changes never trigger an Application rebuild.
 *   Sprite x/y are mutated directly inside the ticker callback — not via
 *   createEffect — so the render loop is entirely outside Solid's reactive
 *   graph (Frontend/RULES.md §4).
 *
 * Colors: home 0x2563eb · away 0xf59e0b · ball 0xffffff · pitch 0x16a34a
 * Radii:  player 6px · ball 4px
 * Slots:  0–10 home, 11–21 away (fw-match-sim convention)
 */

import { Application, Graphics } from "pixi.js";
import {
  createEffect,
  createSignal,
  onCleanup,
  onMount,
  Show,
  type JSX,
} from "solid-js";
import type { MatchFrameDTO } from "~/lib/types";
import {
  AWAY_COLOR,
  BALL_COLOR,
  BALL_RADIUS,
  HOME_COLOR,
  LINE_COLOR,
  PITCH_COLOR,
  PLAYER_RADIUS,
  drawPitchLines,
  lerp,
  pitchLayout,
  simToCanvas,
} from "~/lib/pitch-coords";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CANVAS_W = 840;
const CANVAS_H = 560;

// Canonical frame rate in the sim (ticks per second). Positions advance one
// frame per tick; 1× real-time playback = 60 ticks/second.
const SIM_HZ = 60;

// S6 ball height: 1 sim-metre of posZ lifts the ball dot by this many canvas
// pixels. A typical high ball peaks around 20m; at 4px/m that is 80px of lift
// — clearly visible without flying off-screen at a 560px-tall canvas.
const BALL_Z_PX_PER_M = 4;

// S6: ball dot scale range as a function of posZ.
// Ground ball → 1.0 (BALL_RADIUS as drawn). Peak loft (~20m) → 1.5×.
const BALL_Z_SCALE_MAX = 1.5;
const BALL_Z_SCALE_REF = 20; // metres at which scale reaches BALL_Z_SCALE_MAX

// S5 possession ring: drawn as a thin circle around the carrier dot.
const CARRIER_RING_RADIUS = PLAYER_RADIUS + 4; // px outside the filled dot
const CARRIER_RING_WIDTH = 1.5; // px stroke width

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface TacticalBoardProps {
  /** Frames to replay. Pass [] for an empty board (no dots rendered). */
  frames: readonly MatchFrameDTO[];
  /**
   * When true, the board automatically snaps the cursor to the last frame
   * whenever new frames arrive. Used by the live-match route (S3b) so
   * accumulating step results advance the board without requiring Play.
   *
   * When false (default), the board uses its own play/pause/scrub controls.
   * This preserves the batch-replay behaviour in /match and /dev/board.
   */
  followLatest?: boolean;
  /** Optional CSS class applied to the outer wrapper div. */
  class?: string;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function TacticalBoard(props: TacticalBoardProps): JSX.Element {
  // Playback cursor — a fractional frame index in [0, frames.length - 1].
  // Stored as a plain mutable ref (not a signal) because it is written inside
  // the ticker callback every frame; signals trigger Solid's reactive graph
  // which must not run inside the Pixi render loop.
  let cursor = 0;

  // The scrubber and info readout ARE reactive (user-visible, updated at a
  // human-perceptible rate). We sync them from the ticker at ~10Hz to avoid
  // taxing Solid's diffing with 60 updates/second.
  const [displayTick, setDisplayTick] = createSignal(0);
  const [isPlaying, setIsPlaying] = createSignal(false);

  // Plain mutable mirrors for use inside the Pixi ticker callback.
  // Solid signals and props are reactive; reading them inside the ticker arrow
  // function triggers the solid/reactivity lint and risks unintended tracking
  // outside a tracked scope. These refs are kept in sync with their reactive
  // sources via createEffect (for frames length) and setPlayingBoth (for playing).
  let playingRef = false;
  // framesLengthRef is updated reactively via createEffect below.
  let framesLengthRef = 0;

  // One-shot flag: tracks whether the first non-empty frames batch has been
  // applied. Guards the auto-apply/auto-start effect so that a late-arriving
  // frame batch does NOT yank playback back to frame 0 while the user is
  // mid-scrub. Once true it never resets for the lifetime of this board mount.
  const [firstFramesApplied, setFirstFramesApplied] = createSignal(false);

  // Reactive signal that flips to true once the async Pixi Application.init()
  // has resolved and sprites are allocated. This lets createEffect below track
  // Pixi readiness as a reactive dependency — plain `let` refs are invisible to
  // Solid's reactive graph and cannot be used as trigger conditions in effects.
  const [pixiReady, setPixiReady] = createSignal(false);

  /** Update both the reactive signal (for UI) and the imperative ref (for ticker). */
  function setPlayingBoth(value: boolean): void {
    playingRef = value;
    setIsPlaying(value);
  }

  // Keep framesLengthRef in sync with props.frames — runs in a tracked scope
  // (createEffect) so the reactive read is legitimate.
  createEffect(() => {
    framesLengthRef = props.frames.length;
  });

  // followLatest mode (S3b): when props.followLatest is true, snap the cursor
  // to the last frame whenever new frames arrive. This is how the live-match
  // route streams step results onto the board without requiring the user to
  // press Play — each new frame from stepLiveMatch advances the display
  // automatically. We only apply this when Pixi is ready (sprites allocated).
  //
  // The effect is lightweight: it reads props.frames.length (tracked) and
  // props.followLatest (tracked). When followLatest is false (the default for
  // all batch / dev / replay uses) the effect short-circuits immediately, so
  // the existing board behaviour is completely unchanged.
  createEffect(() => {
    if (!props.followLatest) return;
    const len = props.frames.length;
    if (len === 0 || !pixiReady()) return;
    const lastIdx = len - 1;
    cursor = lastIdx;
    setDisplayTick(lastIdx);
    applyPositions(lastIdx);
  });

  // Apply frame 0 the FIRST time BOTH conditions are true: (a) frames have
  // arrived and (b) the Pixi app has initialised. This fixes the blank board —
  // the dots were parked off-screen because the only applyPositions(0) call
  // raced against the async frame load and always saw []. Handles both races:
  //   1. Board mounts before the async IPC frame load — pixiReady fires first,
  //      frames arrive later; the effect re-runs when frames length changes.
  //   2. Frames arrive before Pixi init completes — the effect re-runs when
  //      pixiReady flips to true.
  // The firstFramesApplied guard makes it run at most ONCE per mount so a later
  // frames update never resets the cursor while the user scrubs. We do NOT
  // auto-start playback: the board shows the formation paused and the user (or
  // the live-match step loop, which drives the board externally) controls play.
  createEffect(() => {
    const len = props.frames.length;
    const ready = pixiReady();
    if (len === 0 || !ready || firstFramesApplied()) return;
    setFirstFramesApplied(true);
    cursor = 0;
    setDisplayTick(0);
    applyPositions(0);
  });

  // Pixi imperative refs — not reactive state.
  let canvasHost!: HTMLDivElement;
  let app: Application | undefined;
  let playerDots: Graphics[] = [];
  let ballDot: Graphics | undefined;
  // S5: possession carrier ring + tether line. Both are mutated each frame inside
  // applyPositions — never via Solid effects (Frontend/RULES.md §4).
  let carrierRing: Graphics | undefined;
  let tetherLine: Graphics | undefined;
  // S6: ground-shadow ellipse for the ball. Drawn at ball's ground position
  // (posX/posY, no Z offset); shrinks as the ball rises.
  let ballShadow: Graphics | undefined;
  let destroyed = false;

  // Ticker callback ref so we can remove it on cleanup.
  type TickerCb = (ticker: { deltaMS: number }) => void;
  let tickerCb: TickerCb | undefined;

  // Throttle: how many ticker calls have elapsed since the last displayTick sync.
  let tickerCallsSinceSync = 0;
  const SYNC_EVERY = 6; // sync displayTick roughly every 6 ticks at 60fps ≈ 10Hz

  const layout = pitchLayout(CANVAS_W, CANVAS_H);

  // ---------------------------------------------------------------------------
  // Helpers
  // ---------------------------------------------------------------------------

  /** Return the total number of frames - 1 (max scrubber index). */
  const maxIndex = () => Math.max(0, props.frames.length - 1);

  /**
   * Position all sprites from the given fractional cursor value.
   * Interpolates positions between the floor and ceil frame.
   * Pure imperative — does NOT touch Solid signals.
   *
   * S5 — possession carrier ring + tether:
   *   Reads frame.possession (slot index or null). If non-null, moves the
   *   carrier ring to that player's interpolated position and draws a tether
   *   line from carrier to ball. Both are cleared (alpha 0) when possession is
   *   null. Updated every frame in the ticker path.
   *
   * S6 — ball height:
   *   Reads ball.posZ (metres). Offsets the ball dot upward by posZ *
   *   BALL_Z_PX_PER_M canvas pixels and scales the dot slightly larger.
   *   The ground-shadow ellipse stays at the ground position and shrinks
   *   as posZ increases.
   */
  function applyPositions(cur: number): void {
    const frames = props.frames;
    if (frames.length === 0 || !ballDot) return;

    const lo = Math.floor(cur);
    const hi = Math.min(lo + 1, frames.length - 1);
    const t = cur - lo;

    const frameA = frames[lo];
    const frameB = frames[hi];
    if (!frameA || !frameB) return;

    // Player dot positions.
    for (let slot = 0; slot < 22; slot++) {
      const dot = playerDots[slot];
      if (!dot) continue;

      const pA = frameA.players[slot];
      const pB = frameB.players[slot];
      if (!pA || !pB) continue;

      const [ax, ay] = simToCanvas(pA.posX, pA.posY, layout);
      const [bx, by] = simToCanvas(pB.posX, pB.posY, layout);
      dot.x = lerp(ax, bx, t);
      dot.y = lerp(ay, by, t);
    }

    // -----------------------------------------------------------------------
    // S6 — ball height + ground shadow.
    // Interpolate the ball's ground position (posX/posY) for both the shadow
    // and the ball dot, then lift the dot by posZ.
    // -----------------------------------------------------------------------
    const [bax, bay] = simToCanvas(frameA.ball.posX, frameA.ball.posY, layout);
    const [bbx, bby] = simToCanvas(frameB.ball.posX, frameB.ball.posY, layout);
    // Ground (projected) position — used for the shadow and tether origin.
    const groundX = lerp(bax, bbx, t);
    const groundY = lerp(bay, bby, t);
    // Interpolate posZ for height lift (metres → canvas px offset).
    const posZ = lerp(frameA.ball.posZ, frameB.ball.posZ, t);
    const liftPx = posZ * BALL_Z_PX_PER_M;
    // Ball dot: raised position.
    ballDot.x = groundX;
    ballDot.y = groundY - liftPx;
    // Scale the dot slightly as the ball rises.
    const ballScale = 1 + Math.min(posZ / BALL_Z_SCALE_REF, 1) * (BALL_Z_SCALE_MAX - 1);
    ballDot.scale.set(ballScale);

    // Ground shadow: stays at groundX/groundY, shrinks as ball rises.
    if (ballShadow) {
      ballShadow.x = groundX;
      ballShadow.y = groundY;
      // Shadow shrinks from full size at posZ=0 to ~30% at posZ=BALL_Z_SCALE_REF.
      const shadowScale = Math.max(0.3, 1 - posZ / BALL_Z_SCALE_REF);
      ballShadow.scale.set(shadowScale, shadowScale * 0.5); // flat ellipse on the pitch plane
      // Only show the shadow when the ball is noticeably airborne.
      ballShadow.alpha = posZ > 0.5 ? 0.45 : 0;
    }

    // -----------------------------------------------------------------------
    // S5 — possession carrier ring + tether line.
    // possession is the slot of the carrier, or null for loose/dead ball.
    // -----------------------------------------------------------------------
    const possession = frameA.possession; // use lo-frame for snap decisiveness
    const carrierSlot = possession !== null && possession >= 0 && possession < 22
      ? possession
      : null;

    if (carrierRing) {
      if (carrierSlot !== null) {
        const carrierDot = playerDots[carrierSlot];
        if (carrierDot) {
          carrierRing.x = carrierDot.x;
          carrierRing.y = carrierDot.y;
          carrierRing.alpha = 1;
        } else {
          carrierRing.alpha = 0;
        }
      } else {
        carrierRing.alpha = 0;
      }
    }

    if (tetherLine) {
      // Redraw the tether each frame — it changes endpoint every tick.
      tetherLine.clear();
      if (carrierSlot !== null) {
        const carrierDot = playerDots[carrierSlot];
        if (carrierDot) {
          tetherLine.setStrokeStyle({ width: 1, color: LINE_COLOR, alpha: 0.55 });
          tetherLine
            .moveTo(carrierDot.x, carrierDot.y)
            .lineTo(groundX, groundY)
            .stroke();
        }
      }
    }
  }

  // ---------------------------------------------------------------------------
  // onMount — build the Pixi scene once
  // ---------------------------------------------------------------------------

  onMount(() => {
    void (async () => {
      const created = new Application();
      await created.init({
        width: CANVAS_W,
        height: CANVAS_H,
        backgroundColor: PITCH_COLOR,
        antialias: true,
        resolution: window.devicePixelRatio || 1,
        autoDensity: true,
      });

      if (destroyed) {
        created.destroy(true, { children: true });
        return;
      }

      app = created;
      canvasHost.appendChild(created.canvas);

      // Static pitch lines — drawn once.
      const lines = new Graphics();
      drawPitchLines(lines, layout);
      created.stage.addChild(lines);

      // Pre-allocate 22 player circles (slots 0–10 home, 11–21 away).
      for (let slot = 0; slot < 22; slot++) {
        const dot = new Graphics();
        const color = slot <= 10 ? HOME_COLOR : AWAY_COLOR;
        dot.fill({ color });
        dot.circle(0, 0, PLAYER_RADIUS).fill();
        // Park off-screen until the first frame is applied.
        dot.x = -100;
        dot.y = -100;
        created.stage.addChild(dot);
        playerDots.push(dot);
      }

      // S5 — carrier ring: a thin circle rendered over the carrier's dot.
      // Drawn below the ball so the ball sits on top. Parked off-screen and
      // made invisible until the first possession frame is applied.
      const ring = new Graphics();
      ring.setStrokeStyle({ width: CARRIER_RING_WIDTH, color: LINE_COLOR, alpha: 1 });
      ring.circle(0, 0, CARRIER_RING_RADIUS).stroke();
      ring.x = -200;
      ring.y = -200;
      ring.alpha = 0;
      created.stage.addChild(ring);
      carrierRing = ring;

      // S5 — tether line: redrawn each frame between carrier and ball ground pos.
      // Allocated as an empty Graphics; content is set in applyPositions.
      const tether = new Graphics();
      created.stage.addChild(tether);
      tetherLine = tether;

      // S6 — ball ground shadow: an ellipse at ball ground position.
      // Rendered below the ball dot. Parked off-screen, alpha 0 until airborne.
      const shadow = new Graphics();
      shadow.fill({ color: 0x000000, alpha: 0.6 });
      shadow.ellipse(0, 0, BALL_RADIUS * 1.4, BALL_RADIUS * 0.7).fill();
      shadow.x = -200;
      shadow.y = -200;
      shadow.alpha = 0;
      created.stage.addChild(shadow);
      ballShadow = shadow;

      // Ball dot — rendered on top of shadow, below carrier ring.
      const ball = new Graphics();
      ball.fill({ color: BALL_COLOR });
      ball.circle(0, 0, BALL_RADIUS).fill();
      ball.x = -100;
      ball.y = -100;
      created.stage.addChild(ball);
      ballDot = ball;

      // Ticker-driven playback loop. deltaMS is wall-clock ms since last frame.
      // We advance the cursor by (deltaMS / 1000) * SIM_HZ ticks/s to hit 1×
      // real-time. Positions are LERP-ed between the bracketing frames.
      //
      // solid/reactivity: this callback is the Pixi render loop — an intentional
      // imperative escape hatch per Frontend/RULES.md §4 ("Render loop via the
      // Pixi ticker, NOT via Solid effects"). Setters (setDisplayTick, setPlayingBoth)
      // are called here to sync UI state FROM the render loop at a throttled rate;
      // the reactive graph is NOT being read — only written. This is the standard
      // Pixi-in-Solid pattern and the disable is load-bearing.
      // eslint-disable-next-line solid/reactivity
      tickerCb = ({ deltaMS }: { deltaMS: number }) => {
        // Use plain mutable refs — reading Solid signals/props here would
        // trigger solid/reactivity lint (untracked reactive read outside a
        // tracked scope). Both refs are kept in sync via setPlayingBoth and
        // the createEffect above.
        if (!playingRef) return;

        const max = Math.max(0, framesLengthRef - 1);
        if (max === 0) return;

        cursor = Math.min(cursor + (deltaMS / 1000) * SIM_HZ, max);
        applyPositions(cursor);

        // Sync reactive tick readout at a throttled rate.
        tickerCallsSinceSync++;
        if (tickerCallsSinceSync >= SYNC_EVERY) {
          tickerCallsSinceSync = 0;
          setDisplayTick(Math.round(cursor));
        }

        // Auto-stop at end of sequence.
        if (cursor >= max) {
          cursor = max;
          setPlayingBoth(false);
          setDisplayTick(max);
        }
      };

      created.ticker.add(tickerCb);

      // Signal that Pixi is ready — all sprites are allocated and the ticker
      // is running. The createEffect above tracks pixiReady() as a reactive
      // dependency; flipping it here triggers that effect to re-evaluate so
      // it can call applyPositions(0) and start playback even if frames had
      // already arrived before this async init completed.
      // The legacy applyPositions(0) call that was here raced against the async
      // frame load and always saw [] — it is superseded by the createEffect.
      setPixiReady(true);
    })();
  });

  // ---------------------------------------------------------------------------
  // onCleanup — tear down Pixi; remove ticker listener
  // ---------------------------------------------------------------------------

  onCleanup(() => {
    destroyed = true;
    if (app) {
      if (tickerCb) {
        app.ticker.remove(tickerCb);
        tickerCb = undefined;
      }
      app.destroy(true, { children: true });
      app = undefined;
      playerDots = [];
      ballDot = undefined;
      carrierRing = undefined;
      tetherLine = undefined;
      ballShadow = undefined;
    }
  });

  // ---------------------------------------------------------------------------
  // Controls
  // ---------------------------------------------------------------------------

  function handlePlayPause(): void {
    const playing = !isPlaying();
    // If at the end, restart from the beginning.
    if (playing && cursor >= maxIndex() && maxIndex() > 0) {
      cursor = 0;
      setDisplayTick(0);
    }
    setPlayingBoth(playing);
    // Apply current frame immediately so scrubbed position shows before ticker fires.
    applyPositions(cursor);
  }

  function handleScrub(value: number): void {
    const clamped = Math.max(0, Math.min(value, maxIndex()));
    cursor = clamped;
    setDisplayTick(clamped);
    applyPositions(clamped);
  }

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  const hasFrames = () => props.frames.length > 0;
  const currentFrame = () => props.frames[Math.round(displayTick())] ?? null;

  return (
    <div class={`space-y-3 ${props.class ?? ""}`}>
      {/* Info readout */}
      <div
        class="font-mono text-sm text-ink-mute dark:text-paper-subtle px-1"
        aria-live="polite"
        aria-label="Frame info"
      >
        <Show
          when={hasFrames()}
          fallback={<span>No frames loaded.</span>}
        >
          <span>
            {currentFrame()
              ? `Tick: ${currentFrame()!.tick} / ${maxIndex()} | Seed: ${currentFrame()!.seedHex} | Score: ${currentFrame()!.homeScore}–${currentFrame()!.awayScore}`
              : `Tick: ${displayTick()} / ${maxIndex()}`}
          </span>
        </Show>
      </div>

      {/* PixiJS canvas mount point */}
      <div
        ref={(el) => {
          canvasHost = el;
        }}
        class="fw-panel overflow-hidden bg-pitch-600 rounded"
        style={{ width: `${CANVAS_W}px`, height: `${CANVAS_H}px` }}
        aria-label="2D tactical board"
        role="img"
      />

      {/* Controls row: play/pause + scrubber */}
      <div class="flex items-center gap-3 px-1">
        {/* Play / pause button */}
        <button
          type="button"
          class="px-3 py-1 text-xs font-mono rounded border focus:outline-none focus:ring-2 focus:ring-pitch-400 bg-pitch-500 text-white border-pitch-600 hover:bg-pitch-600 disabled:opacity-40 disabled:cursor-not-allowed"
          onClick={handlePlayPause}
          disabled={!hasFrames()}
          aria-label={isPlaying() ? "Pause playback" : "Play replay"}
          aria-pressed={isPlaying()}
        >
          {isPlaying() ? "Pause" : "Play"}
        </button>

        {/* Tick scrubber */}
        <label
          class="text-xs font-mono text-ink-mute dark:text-paper-subtle shrink-0"
          for="tactical-board-scrubber"
        >
          Tick
        </label>
        <input
          id="tactical-board-scrubber"
          type="range"
          min="0"
          max={maxIndex()}
          value={displayTick()}
          class="flex-1 accent-pitch-500"
          aria-label="Scrub to tick"
          disabled={!hasFrames()}
          onInput={(e) => {
            const n = parseInt(e.currentTarget.value, 10);
            if (Number.isFinite(n)) {
              setPlayingBoth(false);
              handleScrub(n);
            }
          }}
          onKeyDown={(e) => {
            // Allow arrow key scrubbing without triggering play/pause.
            if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
              e.stopPropagation();
            }
          }}
        />
        <span
          class="text-xs font-mono text-ink-mute dark:text-paper-subtle shrink-0 w-20 text-right"
          aria-live="off"
        >
          {displayTick()} / {maxIndex()}
        </span>
      </div>
    </div>
  );
}

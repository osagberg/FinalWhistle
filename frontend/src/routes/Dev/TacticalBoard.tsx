/*
 * Dev-tier 2D tactical board (T1-2a).
 *
 * PixiJS lifecycle: the Application is created ONCE in onMount and destroyed in
 * onCleanup. Signal changes (currentTick) mutate existing sprite positions; they
 * NEVER trigger a rebuild of the Application or stage — that would leak WebGL
 * contexts under HMR. See Frontend/RULES.md §4.
 *
 * Coordinate mapping: sim origin is the pitch centre, +X toward away goal, +Y
 * toward home touchline. Canvas origin is top-left. We translate to canvas-space
 * by: canvasX = cx + (posX / pitchHalfLength) * halfLen,
 *     canvasY = cy - (posY / pitchHalfWidth) * halfWidth.
 * The Y axis is flipped because canvas +Y is downward.
 *
 * Slot conventions from fw-match-sim: slots 0–10 = home side, 11–21 = away side.
 */

import { Application, Graphics } from "pixi.js";
import { createEffect, createSignal, onCleanup, onMount, type JSX } from "solid-js";
import { useSearchParams } from "@solidjs/router";
import type { MatchFrameDTO } from "~/lib/types";
import { frameSourceFromUrlParams } from "./FrameSource";

// FIFA pitch dimensions (m). Half-values used for coordinate mapping.
const PITCH_HALF_LEN_M = 52.5; // 105 / 2
const PITCH_HALF_WID_M = 34.0; // 68 / 2
const PADDING_M = 4;

// Visual radii (px) — kept small so 22 dots are legible at board size.
const PLAYER_RADIUS = 6;
const BALL_RADIUS = 4;

// Colors: home = steel-blue / away = amber; ball = white.
const HOME_COLOR = 0x2563eb; // Tailwind blue-600
const AWAY_COLOR = 0xf59e0b; // Tailwind amber-400
const BALL_COLOR = 0xffffff;
const PITCH_COLOR = 0x16a34a; // Tailwind green-600
const LINE_COLOR = 0xffffff;

const CANVAS_W = 840;
const CANVAS_H = 560;

// Derive the scale + offset that fits the pitch into the canvas with uniform padding.
function pitchLayout(w: number, h: number) {
  const scaleX = w / (PITCH_HALF_LEN_M * 2 + PADDING_M * 2);
  const scaleY = h / (PITCH_HALF_WID_M * 2 + PADDING_M * 2);
  const s = Math.min(scaleX, scaleY);
  return {
    s,
    cx: w / 2,
    cy: h / 2,
    halfLen: PITCH_HALF_LEN_M * s,
    halfWid: PITCH_HALF_WID_M * s,
  };
}

/** Convert sim coordinates (metres, centred) to canvas pixels. */
function simToCanvas(
  simX: number,
  simY: number,
  layout: ReturnType<typeof pitchLayout>,
): [number, number] {
  const canvasX = layout.cx + (simX / PITCH_HALF_LEN_M) * layout.halfLen;
  // Canvas Y is inverted vs sim Y.
  const canvasY = layout.cy - (simY / PITCH_HALF_WID_M) * layout.halfWid;
  return [canvasX, canvasY];
}

function drawPitchLines(g: Graphics, layout: ReturnType<typeof pitchLayout>): void {
  const { cx, cy, halfLen, halfWid, s } = layout;
  g.setStrokeStyle({ width: 2, color: LINE_COLOR, alpha: 0.8 });

  // Outer boundary.
  g.rect(cx - halfLen, cy - halfWid, halfLen * 2, halfWid * 2).stroke();
  // Halfway line.
  g.moveTo(cx, cy - halfWid).lineTo(cx, cy + halfWid).stroke();
  // Centre circle (9.15m radius).
  g.circle(cx, cy, 9.15 * s).stroke();
  // Centre spot.
  g.fill({ color: LINE_COLOR, alpha: 0.8 });
  g.circle(cx, cy, 2).fill();

  // Penalty boxes (16.5m deep × 40.32m wide).
  const penDepth = 16.5 * s;
  const penHalfWid = 20.16 * s;
  g.rect(cx - halfLen, cy - penHalfWid, penDepth, penHalfWid * 2).stroke();
  g.rect(cx + halfLen - penDepth, cy - penHalfWid, penDepth, penHalfWid * 2).stroke();
}

export default function TacticalBoard(): JSX.Element {
  const [searchParams] = useSearchParams();
  // Reconstruct search string from params so frameSourceFromUrlParams can parse it.
  const search = () =>
    "?" + new URLSearchParams(searchParams as Record<string, string>).toString();

  const [frames, setFrames] = createSignal<MatchFrameDTO[]>([]);
  const [currentTick, setCurrentTick] = createSignal(0);
  const [loadError, setLoadError] = createSignal<string | null>(null);

  // PixiJS refs. Kept outside of signals — they're not reactive state; they're
  // imperative WebGL handles whose lifecycle is tied to onMount/onCleanup.
  let canvasHost!: HTMLDivElement;
  let app: Application | undefined;
  let playerDots: Graphics[] = [];
  let ballDot: Graphics | undefined;
  let destroyed = false;

  const layout = pitchLayout(CANVAS_W, CANVAS_H);

  // Expose debug surface in dev builds so Claude Preview can scrub frames
  // without using the range input.
  function exposeFwDev() {
    if (import.meta.env.DEV) {
      (window as { fwDev?: unknown }).fwDev = {
        scrubTo: (n: number) =>
          setCurrentTick(Math.max(0, Math.min(n, frames().length - 1))),
        currentTick: () => currentTick(),
        frameCount: () => frames().length,
      };
    }
  }

  onMount(() => {
    // Construct the FrameSource synchronously inside onMount so any
    // URL-config errors surface in the board's loadError UI instead of
    // bubbling up the route boundary as an uncaught exception. Codex
    // Tier-2 audit P1 (2026-05-13): the prior version called
    // `frameSourceFromUrlParams(search())` at component-body top level,
    // before the async try/catch — so `?source=bogus` threw during
    // onMount and the user saw a broken route with no error context.
    let source: ReturnType<typeof frameSourceFromUrlParams>;
    try {
      source = frameSourceFromUrlParams(search());
    } catch (e) {
      setLoadError(e instanceof Error ? e.message : String(e));
      return;
    }

    void (async () => {
      // Build the PixiJS Application once; bail out if the component was
      // torn down before the async init finished (fast HMR teardown).
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

      // Static pitch lines — drawn once, never updated.
      const lines = new Graphics();
      drawPitchLines(lines, layout);
      created.stage.addChild(lines);

      // Pre-allocate 22 player circles. Positions are updated reactively.
      for (let slot = 0; slot < 22; slot++) {
        const dot = new Graphics();
        const color = slot <= 10 ? HOME_COLOR : AWAY_COLOR;
        dot.fill({ color });
        dot.circle(0, 0, PLAYER_RADIUS).fill();
        // Park off-screen until the first frame loads.
        dot.x = -100;
        dot.y = -100;
        created.stage.addChild(dot);
        playerDots.push(dot);
      }

      // Ball circle — rendered on top of player dots.
      const ball = new Graphics();
      ball.fill({ color: BALL_COLOR });
      ball.circle(0, 0, BALL_RADIUS).fill();
      ball.x = -100;
      ball.y = -100;
      created.stage.addChild(ball);
      ballDot = ball;

      exposeFwDev();

      // Fetch frames. This may call Tauri IPC or fetch a static JSON file.
      try {
        const loaded = await source.loadFrames();
        setFrames(loaded);
      } catch (e) {
        setLoadError(e instanceof Error ? e.message : String(e));
      }
    })();
  });

  onCleanup(() => {
    destroyed = true;
    if (app) {
      app.destroy(true, { children: true });
      app = undefined;
      playerDots = [];
      ballDot = undefined;
    }
  });

  // React to currentTick changes by repositioning the pre-allocated sprites.
  // This is the only reactive path for the scene graph — no Application rebuild.
  createEffect(() => {
    const tick = currentTick();
    const all = frames();
    if (all.length === 0 || !ballDot) return;

    const frame = all[Math.min(tick, all.length - 1)];
    if (!frame) return;

    for (const player of frame.players) {
      const dot = playerDots[player.slot];
      if (!dot) continue;
      const [px, py] = simToCanvas(player.posX, player.posY, layout);
      dot.x = px;
      dot.y = py;
    }

    const [bx, by] = simToCanvas(frame.ball.posX, frame.ball.posY, layout);
    ballDot.x = bx;
    ballDot.y = by;
  });

  const totalTicks = () => Math.max(0, frames().length - 1);
  const currentFrame = () => frames()[currentTick()];

  return (
    <div class="space-y-3">
      {/* Info readout above the canvas */}
      <div
        class="font-mono text-sm text-ink-mute dark:text-paper-subtle px-1"
        aria-live="polite"
        aria-label="Frame info"
      >
        {currentFrame()
          ? `Tick: ${currentFrame()!.tick} / ${totalTicks()} | Seed: ${currentFrame()!.seedHex} | Score: ${currentFrame()!.homeScore}–${currentFrame()!.awayScore}`
          : loadError()
            ? `Error: ${loadError()}`
            : "Loading frames…"}
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

      {/* Tick scrubber */}
      <div class="flex items-center gap-3 px-1">
        <label
          class="text-xs font-mono text-ink-mute dark:text-paper-subtle shrink-0"
          for="tick-scrubber"
        >
          Tick
        </label>
        <input
          id="tick-scrubber"
          type="range"
          min="0"
          max={totalTicks()}
          value={currentTick()}
          class="flex-1 accent-pitch-500"
          aria-label="Scrub to tick"
          onInput={(e) => {
            const n = parseInt(e.currentTarget.value, 10);
            if (Number.isFinite(n)) setCurrentTick(Math.max(0, Math.min(n, totalTicks())));
          }}
        />
        <span class="text-xs font-mono text-ink-mute dark:text-paper-subtle shrink-0 w-16 text-right">
          {currentTick()} / {totalTicks()}
        </span>
      </div>
    </div>
  );
}

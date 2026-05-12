/*
 * PixiJS v8 tactical board. T0-2 ships an empty green pitch + ID badge so the
 * Match route compiles and renders end-to-end. The real 22-dot interpolation
 * + ball trail + role badges land at T4-1.
 *
 * Pitch dimensions: FIFA-spec 105m × 68m, rendered with a tunable scale.
 * Coordinate system: origin at centre, +x toward away goal, +y toward home
 * touchline. The fw-match-sim Q32 positions will map directly when wired.
 *
 * Solid integration: we initialize Pixi inside `onMount` and tear down inside
 * `onCleanup` to keep dev-mode HMR from leaking WebGL contexts. PixiJS v8
 * uses `await Application.init()` instead of v7's sync `new Application` — the
 * change-over is a common upgrade footgun, hence the `void`-ed promise pattern.
 */

import { Application, Container, Graphics } from "pixi.js";
import { onCleanup, onMount, type JSX } from "solid-js";

export interface TacticalBoardProps {
  /** CSS width hint for the wrapper. */
  width?: number;
  /** CSS height hint for the wrapper. */
  height?: number;
}

// FIFA pitch dimensions (m).
const PITCH_LENGTH_M = 105;
const PITCH_WIDTH_M = 68;
const PADDING_M = 4;

export default function TacticalBoard(props: TacticalBoardProps): JSX.Element {
  let host!: HTMLDivElement;
  let app: Application | undefined;
  let destroyed = false;

  onMount(() => {
    const target = host;
    const width = props.width ?? target.clientWidth ?? 800;
    const height = props.height ?? target.clientHeight ?? 520;

    // Pixi v8 init is async; track destruction so a race during HMR teardown
    // doesn't attach a leaked canvas to a torn-down wrapper.
    void (async () => {
      const created = new Application();
      await created.init({
        width,
        height,
        backgroundColor: 0x2d6e3e, // pitch-500 from Tailwind
        antialias: true,
        resolution: window.devicePixelRatio || 1,
        autoDensity: true,
      });
      if (destroyed) {
        created.destroy(true, { children: true });
        return;
      }
      app = created;
      target.appendChild(created.canvas);

      drawPitch(created, width, height);
    })();
  });

  onCleanup(() => {
    destroyed = true;
    if (app) {
      app.destroy(true, { children: true });
      app = undefined;
    }
  });

  return (
    <div
      ref={(el) => {
        host = el;
      }}
      class="fw-panel overflow-hidden bg-pitch-600"
      style={{
        width: props.width ? `${props.width}px` : "100%",
        height: props.height ? `${props.height}px` : "520px",
      }}
      aria-label="Tactical board placeholder"
    />
  );
}

function drawPitch(app: Application, width: number, height: number): void {
  const stage = new Container();
  app.stage.addChild(stage);

  // Scale so the pitch + padding fits the canvas with equal margins.
  const scaleX = width / (PITCH_LENGTH_M + 2 * PADDING_M);
  const scaleY = height / (PITCH_WIDTH_M + 2 * PADDING_M);
  const s = Math.min(scaleX, scaleY);
  const cx = width / 2;
  const cy = height / 2;

  const halfLen = (PITCH_LENGTH_M / 2) * s;
  const halfWidth = (PITCH_WIDTH_M / 2) * s;

  const lines = new Graphics();
  lines.setStrokeStyle({ width: 2, color: 0xffffff, alpha: 0.85 });

  // Outer rectangle.
  lines.rect(cx - halfLen, cy - halfWidth, halfLen * 2, halfWidth * 2).stroke();
  // Halfway line.
  lines.moveTo(cx, cy - halfWidth).lineTo(cx, cy + halfWidth).stroke();
  // Centre circle (9.15m radius).
  lines.circle(cx, cy, 9.15 * s).stroke();
  // Centre spot.
  lines.fill({ color: 0xffffff, alpha: 0.85 });
  lines.circle(cx, cy, 2).fill();

  // Penalty boxes (16.5m × 40.32m).
  const penLen = 16.5 * s;
  const penHalfWidth = 20.16 * s;
  lines
    .rect(cx - halfLen, cy - penHalfWidth, penLen, penHalfWidth * 2)
    .stroke();
  lines
    .rect(cx + halfLen - penLen, cy - penHalfWidth, penLen, penHalfWidth * 2)
    .stroke();

  stage.addChild(lines);
}

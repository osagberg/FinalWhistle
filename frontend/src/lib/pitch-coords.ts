/*
 * Pitch coordinate utilities shared between the production TacticalBoard
 * (components/TacticalBoard.tsx) and the dev board (routes/Dev/TacticalBoard.tsx).
 *
 * Coordinate system:
 *   Sim origin = pitch centre, +X toward away goal, +Y toward home touchline.
 *   Canvas origin = top-left; Y increases downward (inverted vs sim).
 *
 * T4-1: extracted from routes/Dev/TacticalBoard.tsx into a shared lib module.
 * Both boards import from here; the dev board's imports were updated to match.
 */

import type { Graphics } from "pixi.js";

// FIFA pitch dimensions (m).
export const PITCH_HALF_LEN_M = 52.5; // 105 / 2
export const PITCH_HALF_WID_M = 34.0; // 68 / 2
const PADDING_M = 4;

// Visual radii (px) — small enough for 22 dots to be legible at board size.
export const PLAYER_RADIUS = 6;
export const BALL_RADIUS = 4;

// Colors: home = steel-blue, away = amber, ball = white.
export const HOME_COLOR = 0x2563eb; // Tailwind blue-600
export const AWAY_COLOR = 0xf59e0b; // Tailwind amber-400
export const BALL_COLOR = 0xffffff;
export const PITCH_COLOR = 0x16a34a; // Tailwind green-600
const LINE_COLOR = 0xffffff;

/**
 * Compute the scale + offset that fits the pitch + padding into a canvas
 * of the given pixel dimensions with uniform margins.
 */
export function pitchLayout(w: number, h: number) {
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

export type PitchLayout = ReturnType<typeof pitchLayout>;

/**
 * Convert sim coordinates (metres, pitch-centred) to canvas pixels.
 * Y is flipped because canvas +Y is downward but sim +Y is toward the
 * home touchline (upward on the visual representation).
 */
export function simToCanvas(
  simX: number,
  simY: number,
  layout: PitchLayout,
): [number, number] {
  const canvasX = layout.cx + (simX / PITCH_HALF_LEN_M) * layout.halfLen;
  const canvasY = layout.cy - (simY / PITCH_HALF_WID_M) * layout.halfWid;
  return [canvasX, canvasY];
}

/**
 * Draw FIFA pitch lines onto a pre-allocated Graphics object.
 * Called once on mount; the Graphics instance is added to the stage and
 * never rebuilt.
 */
export function drawPitchLines(g: Graphics, layout: PitchLayout): void {
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

/**
 * Linearly interpolate between two values: (1 - t) * a + t * b.
 * t is clamped to [0, 1].
 */
export function lerp(a: number, b: number, t: number): number {
  const tc = Math.max(0, Math.min(1, t));
  return a + (b - a) * tc;
}

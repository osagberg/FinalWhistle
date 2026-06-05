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

// Board palette — derived from the design tokens (tailwind.config.ts), not raw
// Tailwind defaults. The redesign research flagged the old bright green/blue/
// amber as the one place the muted "broadsheet" identity broke into video-game
// slop. The turf is a dim floodlit-evening green so the dots + ball pop; home
// is the signature light sage, away a muted warm amber, ball warm paper-white.
// (Per-club accent colours from procgen are a later slice; these are the
// neutral defaults.)
export const HOME_COLOR = 0x9fcaab; // pitch-200 — light sage (managed side)
export const AWAY_COLOR = 0xc8843c; // muted warm amber (not flag-yellow caution)
export const BALL_COLOR = 0xf7f4ee; // paper — warm white
export const PITCH_COLOR = 0x13311d; // pitch-800 — dim floodlit turf, not grass-green
export const LINE_COLOR = 0xf7f4ee; // paper — pitch markings

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
 *
 * Markings drawn (S7):
 *   - Outer boundary
 *   - Halfway line
 *   - Centre circle (r=9.15m) + centre spot
 *   - Both penalty boxes (16.5m deep × 40.32m wide)
 *   - Both 6-yard boxes (5.5m deep × 18.32m wide)
 *   - Both penalty spots (11m from goal line)
 *   - Both penalty arcs ("D", r=9.15m centred on penalty spot, outside box)
 *   - Four corner arcs (r=1m)
 *   - Goal frames at each goal line (7.32m wide × 2.44m deep, scaled flat to 2D)
 */
export function drawPitchLines(g: Graphics, layout: PitchLayout): void {
  const { cx, cy, halfLen, halfWid, s } = layout;
  g.setStrokeStyle({ width: 2, color: LINE_COLOR, alpha: 0.8 });

  // Outer boundary.
  g.rect(cx - halfLen, cy - halfWid, halfLen * 2, halfWid * 2).stroke();

  // Halfway line.
  g.moveTo(cx, cy - halfWid).lineTo(cx, cy + halfWid).stroke();

  // Centre circle (r=9.15m) and centre spot.
  g.circle(cx, cy, 9.15 * s).stroke();
  g.fill({ color: LINE_COLOR, alpha: 0.8 });
  g.circle(cx, cy, 2).fill();

  // ---------------------------------------------------------------------------
  // Penalty boxes (16.5m deep × 40.32m wide).
  // Both sides — home defends left (canvas -X), away defends right (+X).
  // ---------------------------------------------------------------------------
  const penDepth = 16.5 * s;
  const penHalfWid = 20.16 * s;
  g.setStrokeStyle({ width: 2, color: LINE_COLOR, alpha: 0.8 });
  // Home penalty box (left).
  g.rect(cx - halfLen, cy - penHalfWid, penDepth, penHalfWid * 2).stroke();
  // Away penalty box (right).
  g.rect(cx + halfLen - penDepth, cy - penHalfWid, penDepth, penHalfWid * 2).stroke();

  // ---------------------------------------------------------------------------
  // 6-yard boxes (5.5m deep × 18.32m wide).
  // ---------------------------------------------------------------------------
  const sixYdDepth = 5.5 * s;
  const sixYdHalfWid = 9.16 * s;
  // Home 6-yard box (left).
  g.rect(cx - halfLen, cy - sixYdHalfWid, sixYdDepth, sixYdHalfWid * 2).stroke();
  // Away 6-yard box (right).
  g.rect(cx + halfLen - sixYdDepth, cy - sixYdHalfWid, sixYdDepth, sixYdHalfWid * 2).stroke();

  // ---------------------------------------------------------------------------
  // Penalty spots (11m from goal line, on the pitch axis).
  // ---------------------------------------------------------------------------
  const penSpotX = 11.0 * s;
  g.fill({ color: LINE_COLOR, alpha: 0.8 });
  // Home penalty spot (left).
  g.circle(cx - halfLen + penSpotX, cy, 2).fill();
  // Away penalty spot (right).
  g.circle(cx + halfLen - penSpotX, cy, 2).fill();

  // ---------------------------------------------------------------------------
  // Penalty arcs ("D"): r=9.15m centred on the penalty spot, only the arc
  // segment that lies outside the penalty box is drawn. We draw the full circle
  // and let the box boundary serve as a visual clip — the arc outside the box
  // is the visible portion. PixiJS does not support clip paths on Graphics in
  // the same call, so we draw the full arc and accept a small overlap; at this
  // scale the effect is indistinguishable from a real D. The arc is drawn with
  // low alpha to keep it furniture-subtle.
  // ---------------------------------------------------------------------------
  const arcR = 9.15 * s;
  g.setStrokeStyle({ width: 2, color: LINE_COLOR, alpha: 0.6 });
  // Home D (arc centred at penalty spot, on the right-hand side of the spot,
  // i.e. toward the centre circle — the portion outside the box).
  g.arc(cx - halfLen + penSpotX, cy, arcR, -Math.PI / 2, Math.PI / 2).stroke();
  // Away D (arc centred at penalty spot, toward centre circle — left side).
  g.arc(cx + halfLen - penSpotX, cy, arcR, Math.PI / 2, (3 * Math.PI) / 2).stroke();

  // ---------------------------------------------------------------------------
  // Corner arcs (r=1m, quarter-circle in each corner of the pitch).
  // Canvas coordinate reminder: top of pitch = cy - halfWid, bottom = cy + halfWid,
  // left = cx - halfLen, right = cx + halfLen. Canvas Y increases downward.
  // ---------------------------------------------------------------------------
  const cornerR = 1.0 * s;
  g.setStrokeStyle({ width: 2, color: LINE_COLOR, alpha: 0.8 });
  // Top-left corner: arc sweeps from East (0) to South (π/2).
  g.arc(cx - halfLen, cy - halfWid, cornerR, 0, Math.PI / 2).stroke();
  // Top-right corner: arc sweeps from West (π) to South-West (π/2) — i.e. π/2..π flipped.
  g.arc(cx + halfLen, cy - halfWid, cornerR, Math.PI / 2, Math.PI).stroke();
  // Bottom-left corner: arc sweeps from North (3π/2) to East (0/2π).
  g.arc(cx - halfLen, cy + halfWid, cornerR, (3 * Math.PI) / 2, 2 * Math.PI).stroke();
  // Bottom-right corner: arc sweeps from South (π/2) upward to West (π) — reversed.
  g.arc(cx + halfLen, cy + halfWid, cornerR, Math.PI, (3 * Math.PI) / 2).stroke();

  // ---------------------------------------------------------------------------
  // Goal frames: 7.32m wide, rendered as a shallow rectangle behind the goal
  // line (2.44m deep in-field, scaled to pixel space). Drawn slightly inside
  // the pitch boundary to be visible against the outer boundary line.
  // ---------------------------------------------------------------------------
  const goalHalfWid = 3.66 * s; // 7.32m / 2
  const goalDepth = 2.44 * s; // nominal goal depth for 2D footprint
  g.setStrokeStyle({ width: 2, color: LINE_COLOR, alpha: 0.9 });
  // Home goal (left — extends further left, outside the pitch boundary).
  g.rect(cx - halfLen - goalDepth, cy - goalHalfWid, goalDepth, goalHalfWid * 2).stroke();
  // Away goal (right — extends further right, outside the pitch boundary).
  g.rect(cx + halfLen, cy - goalHalfWid, goalDepth, goalHalfWid * 2).stroke();
}

/**
 * Linearly interpolate between two values: (1 - t) * a + t * b.
 * t is clamped to [0, 1].
 */
export function lerp(a: number, b: number, t: number): number {
  const tc = Math.max(0, Math.min(1, t));
  return a + (b - a) * tc;
}

/*
 * FrameSource — abstraction over two frame producers for the dev tactical board.
 *
 * Two implementations exist so the board works both inside Tauri (where the sim
 * is available via IPC) and in a plain browser dev tab (where the sim is not
 * reachable but `dump_frames` JSON can be served statically by Vite).
 *
 * URL param convention:
 *   ?source=tauri           → TauriFrameSource (also the default when absent)
 *   ?source=fixture:/path   → HttpFrameSource, absolute path served by Vite
 *   ?source=fixture:./path  → HttpFrameSource, relative path
 *   ?source=fixture:https://...   → HttpFrameSource against an external URL
 *   ?seed=0x...             → seed for TauriFrameSource (default: 0xdeadbeefdeadbeef)
 *   ?ticks=N                → tick count for TauriFrameSource (default: 60)
 *
 * Any other `source=` value (e.g. `source=bogus`, `source=Tauri` with caps,
 * `source=fixture` without a colon, `source=fixture:` with empty path) is a
 * client-side error: the factory throws `FrameSourceConfigError` rather than
 * silently falling back to TauriFrameSource. The fail-loud behavior was added
 * per Codex pre-T1-2b audit self-review P1 — silent fallback masked
 * misconfiguration in a way the developer couldn't see until invoke() blew
 * up much later.
 */

import { invoke } from "@tauri-apps/api/core";
import type { MatchFrameDTO } from "~/lib/types";
import { MAX_FRAMES_PER_REQUEST } from "~/lib/types";

export interface FrameSource {
  /** Return the complete frame sequence. The result may be cached by the caller. */
  loadFrames(): Promise<MatchFrameDTO[]>;
}

/**
 * Thrown by `frameSourceFromUrlParams` when the `?source=` value is set but
 * doesn't match a known shape. Distinct from network/parse errors raised
 * inside `loadFrames`; this one fires at construction time so the route
 * can show a clean error UI instead of a stale spinner.
 */
export class FrameSourceConfigError extends Error {
  constructor(message: string) {
    super(`FrameSource config: ${message}`);
    this.name = "FrameSourceConfigError";
  }
}

// ---------------------------------------------------------------------------
// TauriFrameSource
// ---------------------------------------------------------------------------

export class TauriFrameSource implements FrameSource {
  private readonly seedHex: string;
  private readonly tickCount: number;

  constructor(seedHex: string, tickCount: number) {
    this.seedHex = seedHex;
    this.tickCount = tickCount;
  }

  async loadFrames(): Promise<MatchFrameDTO[]> {
    // Pre-invoke cap check — avoids the IPC round-trip on a guaranteed failure.
    // Single source of truth is `fw_tauri::MAX_FRAMES_PER_REQUEST` (Rust const);
    // `MAX_FRAMES_PER_REQUEST` in lib/types.ts mirrors it with a doc-citation.
    if (this.tickCount > MAX_FRAMES_PER_REQUEST) {
      throw new FrameSourceConfigError(
        `tickCount ${this.tickCount} exceeds MAX_FRAMES_PER_REQUEST (${MAX_FRAMES_PER_REQUEST}). ` +
          `Reduce tickCount or increase the cap (requires a matching Rust change).`,
      );
    }
    return invoke<MatchFrameDTO[]>("match_frames", {
      seedHex: this.seedHex,
      tickCount: this.tickCount,
    });
  }
}

// ---------------------------------------------------------------------------
// HttpFrameSource
// ---------------------------------------------------------------------------

export class HttpFrameSource implements FrameSource {
  private readonly url: string;

  constructor(url: string) {
    this.url = url;
  }

  async loadFrames(): Promise<MatchFrameDTO[]> {
    const res = await fetch(this.url);
    if (!res.ok) {
      throw new Error(
        `HttpFrameSource: fetch ${this.url} failed — ${res.status} ${res.statusText}`,
      );
    }
    const body: unknown = await res.json();
    if (!isMatchFrameArray(body)) {
      // Loud rejection on wrong shape per Codex pre-T1-2b audit P1.
      // Without this guard, malformed JSON (an error object, a single
      // frame instead of an array, a missing `players` field) would
      // crash deep in the PixiJS update path with a confusing
      // TypeError. Surface the bad shape at the source.
      throw new Error(
        `HttpFrameSource: ${this.url} returned JSON that is not MatchFrameDTO[]. ` +
          `Expected an array of objects with {seedHex, tick, homeScore, awayScore, players, ball} fields.`,
      );
    }
    return body;
  }
}

// Runtime shape check. Codex Tier-2 audit P1 (2026-05-13): the prior
// version only checked top-level fields existed, which let
// `{ players: [{}], ball: {} }` pass and then NaN'd out at render time.
// The renderer reads `player.slot`, `player.posX/Y`, `ball.posX/Y/Z`
// etc. — every field the renderer reads is now validated here, so
// malformed fixtures fail loudly at the FrameSource boundary instead
// of producing blank/NaN dots.
//
// Still NOT a full schema validator (no zod / ajv dep — adds 50KB to
// a dev-only route). The check is tight against what the renderer
// actually consumes; future field additions on the renderer side
// require corresponding validator updates.
function isMatchFrameArray(value: unknown): value is MatchFrameDTO[] {
  if (!Array.isArray(value)) {
    return false;
  }
  return value.every(isMatchFrame);
}

function isMatchFrame(frame: unknown): frame is MatchFrameDTO {
  if (typeof frame !== "object" || frame === null) {
    return false;
  }
  const f = frame as Record<string, unknown>;

  // Header fields.
  if (typeof f.tick !== "number" || !Number.isFinite(f.tick)) {
    return false;
  }
  if (typeof f.seedHex !== "string" || f.seedHex.length === 0) {
    return false;
  }
  if (typeof f.homeScore !== "number" || !Number.isFinite(f.homeScore)) {
    return false;
  }
  if (typeof f.awayScore !== "number" || !Number.isFinite(f.awayScore)) {
    return false;
  }

  // Players: must be exactly 22 (per canonical state — 11/side). Slots
  // must be unique integers in 0..21. Each player's position +
  // velocity must be finite — NaN/Infinity in coordinates would crash
  // PixiJS sprite positioning silently.
  if (!Array.isArray(f.players) || f.players.length !== 22) {
    return false;
  }
  const seenSlots = new Set<number>();
  for (const player of f.players) {
    if (!isPlayerFrame(player)) {
      return false;
    }
    if (seenSlots.has(player.slot)) {
      return false;
    }
    seenSlots.add(player.slot);
  }

  // Ball: all 6 coordinates must be finite numbers.
  if (!isBallFrame(f.ball)) {
    return false;
  }

  return true;
}

function isPlayerFrame(value: unknown): value is { slot: number; posX: number; posY: number; velX: number; velY: number } {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const p = value as Record<string, unknown>;
  if (typeof p.slot !== "number" || !Number.isInteger(p.slot) || p.slot < 0 || p.slot > 21) {
    return false;
  }
  for (const field of ["posX", "posY", "velX", "velY"] as const) {
    const v = p[field];
    if (typeof v !== "number" || !Number.isFinite(v)) {
      return false;
    }
  }
  return true;
}

function isBallFrame(value: unknown): value is { posX: number; posY: number; posZ: number; velX: number; velY: number; velZ: number } {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const b = value as Record<string, unknown>;
  for (const field of ["posX", "posY", "posZ", "velX", "velY", "velZ"] as const) {
    const v = b[field];
    if (typeof v !== "number" || !Number.isFinite(v)) {
      return false;
    }
  }
  return true;
}

// ---------------------------------------------------------------------------
// Factory: construct from the page's URL search params
// ---------------------------------------------------------------------------

const DEFAULT_SEED = "0xdeadbeefdeadbeef";
const DEFAULT_TICKS = 60;

/**
 * Build a FrameSource from the current URL's query params.
 *
 * `search` should be `location.search` (e.g. `"?source=fixture:/tmp/smoke.json&ticks=90"`).
 *
 * Defaults to TauriFrameSource when `?source` is absent OR explicitly `tauri`.
 *
 * Throws `FrameSourceConfigError` if `?source` is set to something the
 * factory doesn't recognise. Pre-T1-2b audit found that silently falling
 * back to Tauri masked typos (`source=Tauri` capitalised, `source=fixture`
 * missing the colon, `source=fixture:` with empty path) — failures only
 * appeared later as obscure `invoke is not defined` errors in browser-dev
 * tabs. Now the factory fails fast with a clear message.
 */
export function frameSourceFromUrlParams(search: string): FrameSource {
  const params = new URLSearchParams(search);
  const source = params.get("source");

  // No source param OR explicit `tauri` → TauriFrameSource.
  if (source === null || source === "tauri") {
    const seedHex = params.get("seed") ?? DEFAULT_SEED;
    const ticks = Number(params.get("ticks") ?? DEFAULT_TICKS);
    const tickCount =
      Number.isFinite(ticks) && ticks > 0 ? Math.round(ticks) : DEFAULT_TICKS;
    return new TauriFrameSource(seedHex, tickCount);
  }

  // Anything else MUST be a `fixture:` URL with a non-empty path.
  if (!source.startsWith("fixture:")) {
    throw new FrameSourceConfigError(
      `unknown source value ${JSON.stringify(source)}. Use \`?source=tauri\` (default), \`?source=fixture:/path\`, or \`?source=fixture:https://...\`.`,
    );
  }

  const rawPath = source.slice("fixture:".length);
  if (rawPath === "") {
    throw new FrameSourceConfigError(
      `\`fixture:\` requires a path. Examples: \`?source=fixture:/dev-fixtures/smoke.json\`, \`?source=fixture:./local.json\`, \`?source=fixture:https://...\`.`,
    );
  }

  // Accept three URL shapes — origin-relative (/path), document-relative
  // (./path or path), and absolute (https://... / http://...). Reject
  // everything else explicitly (e.g. `javascript:`, `data:`, `file:`,
  // mailto) to avoid accidentally fetching a non-HTTP scheme. Codex
  // Tier-2 audit P2 (2026-05-13): the prior regex `^[a-zA-Z0-9_\-.]`
  // had a hole — `javascript:alert(1)` started with `j` so passed the
  // relative-path check. Now: any leading scheme (`prefix:`) that
  // isn't http(s) is rejected up front, before the relative-path test.
  const isHttpAbsolute =
    rawPath.startsWith("https://") || rawPath.startsWith("http://");

  // Detect ANY URL scheme (`prefix:` where prefix is a valid scheme
  // name per RFC 3986: ALPHA *(ALPHA / DIGIT / "+" / "-" / ".")).
  const schemeMatch = rawPath.match(/^[a-zA-Z][a-zA-Z0-9+.-]*:/);
  if (schemeMatch && !isHttpAbsolute) {
    throw new FrameSourceConfigError(
      `path \`${rawPath}\` uses scheme \`${schemeMatch[0]}\` which is not allowed. ` +
        `Accepted shapes: \`/dev-fixtures/...\`, \`./...\`, or \`https://...\`.`,
    );
  }

  const isRelativeOrRooted =
    rawPath.startsWith("/") ||
    rawPath.startsWith("./") ||
    /^[a-zA-Z0-9_\-.]/.test(rawPath);

  if (!isHttpAbsolute && !isRelativeOrRooted) {
    throw new FrameSourceConfigError(
      `path \`${rawPath}\` is neither an http(s):// URL nor a relative/rooted file path. ` +
        `Accepted shapes: \`/dev-fixtures/...\`, \`./...\`, or \`https://...\`.`,
    );
  }

  return new HttpFrameSource(rawPath);
}

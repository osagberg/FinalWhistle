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

// Minimal runtime shape check. Fast (typeof-only on a few headline
// fields), loud (returns false → loadFrames throws), and doesn't try to
// be a full schema validator — `ajv`-style validation would force a 50KB
// JSON-schema dep into a dev-only route. The check matches what the
// renderer actually reads in TacticalBoard.tsx.
function isMatchFrameArray(value: unknown): value is MatchFrameDTO[] {
  if (!Array.isArray(value)) {
    return false;
  }
  return value.every((frame) => {
    if (typeof frame !== "object" || frame === null) {
      return false;
    }
    const f = frame as Record<string, unknown>;
    if (typeof f.tick !== "number") {
      return false;
    }
    if (typeof f.seedHex !== "string") {
      return false;
    }
    if (!Array.isArray(f.players)) {
      return false;
    }
    if (typeof f.ball !== "object" || f.ball === null) {
      return false;
    }
    return true;
  });
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
  // everything else (e.g. `javascript:`, `data:`, mailto) to avoid
  // accidentally fetching a non-HTTP scheme. The dev-only context limits
  // the threat surface, but the explicit allowlist is cheap.
  const isHttpAbsolute =
    rawPath.startsWith("https://") || rawPath.startsWith("http://");
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

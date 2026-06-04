/*
 * Typed Tauri command bridge.
 *
 * Every wrapper takes camelCase TS arguments, hands them to `invoke()`, and
 * returns the typed payload. When fw-tauri exposes new commands, add a
 * wrapper here — routes never touch `invoke` directly.
 *
 * T1-5 cleanup:
 *   - getLeagueStandings / getSquad / listFixtures deleted (T0-2 stubs;
 *     real commands land at T2-6/T2-7 alongside the season controller).
 *   - playMatch signature harmonized: was (seed, homeId, awayId) → now
 *     (seed: bigint, tickCount: number) matching fw-tauri's play_match.
 *   - Seed→hex conversion is this wrapper's responsibility (JS BigInt
 *     doesn't round-trip through serde_json; we convert to "0x..." string
 *     and let Rust parse it back as u64).
 */

import type { BackendHandshake, MatchResult } from "./types";
import {
  httpBackendActive as httpBackendActiveImpl,
  isBackendHandshake,
  isMatchResult,
  safeInvoke,
} from "./runtime-validators";

/**
 * Returns the backend handshake payload — Home page's liveness check.
 *
 * Codex 2026-05-16 Tier-2 fix-pass: renamed from `getDummyState` (which
 * after T1-5 consolidation called `get_dummy_state` returning a
 * `MatchStateDto`, mismatched with Home.tsx's `appVersion/message/backendReady`
 * read path). Now the wrapper, the Rust command, and the consumer all agree
 * on the `{ appVersion, message, backendReady }` shape.
 *
 * T1-3.6: wrapped in `safeInvoke` for runtime shape validation per Codex's
 * post-T1-7 adversarial-audit P1 — backend wire-shape drift now throws
 * `IpcShapeError` at the IPC seam instead of NPEing deep in consumer code.
 */
export async function getBackendHandshake(): Promise<BackendHandshake> {
  return safeInvoke("get_backend_handshake", {}, isBackendHandshake);
}

/**
 * Run a single match for `tickCount` ticks from the given seed.
 *
 * The seed is converted to a `"0x<16-hex-chars>"` string before invoking
 * because JS BigInt cannot round-trip through serde_json (serde sees a number,
 * not a u64). The Rust side parses it back with `u64::from_str_radix`.
 *
 * T1-3.6: wrapped in `safeInvoke` for runtime shape validation.
 */
export async function playMatch(
  seed: bigint,
  tickCount: number,
): Promise<MatchResult> {
  const seedHex = "0x" + seed.toString(16).padStart(16, "0");
  return safeInvoke("play_match", { seedHex, tickCount }, isMatchResult);
}

/**
 * Best-effort detection of "are we running inside Tauri?". Useful during
 * `pnpm dev` in a plain browser tab (no Tauri runtime) so the UI can render
 * with stub data instead of erroring on every `invoke`.
 */
export function isTauri(): boolean {
  // Tauri 2 exposes `__TAURI_INTERNALS__` instead of v1's `__TAURI__`.
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Returns `true` when the DEV-ONLY HTTP backend bridge is active.
 *
 * Activation: DEV-only + either
 *   VITE_FW_BROWSER_BACKEND=http  (env var, e.g. in `.env.local`)
 *   ?backend=http in the URL      (handy for one-off preview tabs)
 *
 * The HTTP bridge talks to the `fw-dev-server` binary at 127.0.0.1:1422,
 * reached via Vite's dev proxy (see `vite.config.ts` `/__cmd` → `/cmd`).
 *
 * NEVER returns `true` in a production build. Orthogonal to `isTauri()`.
 *
 * Implemented in `runtime-validators.ts` to avoid the circular-import cycle
 * (`tauri.ts` → `runtime-validators.ts` → `tauri.ts`). Re-exported here so
 * route components have a single import location.
 */
export const httpBackendActive: () => boolean = httpBackendActiveImpl;

/**
 * Returns `true` when a real backend is reachable — either via Tauri IPC or
 * the DEV-ONLY HTTP bridge (`?backend=http`).
 *
 * Use this instead of `isTauri()` at gates that should work with both the
 * production backend and the dev HTTP bridge.
 */
export function backendAvailable(): boolean {
  return isTauri() || httpBackendActiveImpl();
}

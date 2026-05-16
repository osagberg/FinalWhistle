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

import { invoke } from "@tauri-apps/api/core";
import type { DummyState, MatchResult } from "./types";

/** Returns a stub backend handshake payload. Used by Home as a liveness check. */
export async function getDummyState(): Promise<DummyState> {
  return invoke<DummyState>("get_dummy_state");
}

/**
 * Run a single match for `tickCount` ticks from the given seed.
 *
 * The seed is converted to a `"0x<16-hex-chars>"` string before invoking
 * because JS BigInt cannot round-trip through serde_json (serde sees a number,
 * not a u64). The Rust side parses it back with `u64::from_str_radix`.
 */
export async function playMatch(
  seed: bigint,
  tickCount: number,
): Promise<MatchResult> {
  const seedHex = "0x" + seed.toString(16).padStart(16, "0");
  return invoke<MatchResult>("play_match", { seedHex, tickCount });
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

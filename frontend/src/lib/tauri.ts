/*
 * Typed Tauri command bridge.
 *
 * Every wrapper takes camelCase TS arguments, hands them to `invoke()`, and
 * returns the typed payload. When fw-tauri lands real commands, the function
 * signatures here are the single migration point — the routes / components
 * never touch `invoke` directly.
 *
 * BigInt handling: serde_json doesn't speak bigint, so seeds round-trip as
 * decimal strings. The wrapper accepts `bigint` and stringifies; the Rust
 * side parses back to `u64`.
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  DummyState,
  Fixture,
  LeagueStanding,
  MatchResult,
  PlayerSummary,
} from "./types";

/** Returns a stub backend handshake payload. Used by Home as a liveness check. */
export async function getDummyState(): Promise<DummyState> {
  return invoke<DummyState>("get_dummy_state");
}

/** Run a single match between two procedural clubs at the given seed. */
export async function playMatch(
  seed: bigint,
  homeId: string,
  awayId: string,
): Promise<MatchResult> {
  return invoke<MatchResult>("play_match", {
    seed: seed.toString(),
    homeId,
    awayId,
  });
}

/** Standings for a given competition. */
export async function getLeagueStandings(leagueId: string): Promise<LeagueStanding[]> {
  return invoke<LeagueStanding[]>("get_league_standings", { leagueId });
}

/** Squad list for a given club. */
export async function getSquad(clubId: string): Promise<PlayerSummary[]> {
  return invoke<PlayerSummary[]>("get_squad", { clubId });
}

/** Fixture list for a given club. */
export async function listFixtures(clubId: string): Promise<Fixture[]> {
  return invoke<Fixture[]>("list_fixtures", { clubId });
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

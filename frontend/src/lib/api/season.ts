/**
 * Season controller IPC wrappers (T2-5).
 *
 * Thin `invoke()` wrappers over the four Tauri commands:
 *   `advance_week`, `play_fixtures`, `get_standings`, `get_fixtures`.
 *
 * These are the ONLY functions that should call `invoke` for season-related
 * commands — route components import from here, not from `@tauri-apps/api/core`
 * directly, so the command-name strings have a single authoritative location.
 */
import { invoke } from "@tauri-apps/api/core";
import type {
  AdvanceWeekSummary,
  FixtureWithResult,
  PlayFixturesSummary,
  StandingsRow,
} from "../types";

/**
 * Advance the season by one match-day.
 *
 * Plays all 10 fixtures on the current match-day deterministically and
 * bumps the match-day counter by 1.
 *
 * Rejects with an `IpcError` of kind `"seasonComplete"` if called after the
 * 38th match-day has already been played.
 */
export async function advanceWeek(): Promise<AdvanceWeekSummary> {
  return invoke<AdvanceWeekSummary>("advance_week");
}

/**
 * Fast-forward all remaining fixtures in one call.
 *
 * Plays every unplayed match-day sequentially and returns a summary with
 * the total matches played and the final match-day reached.
 *
 * If the season is already complete, returns `{ matchesPlayed: 0, finalMatchDay: 38 }`.
 */
export async function playFixtures(): Promise<PlayFixturesSummary> {
  return invoke<PlayFixturesSummary>("play_fixtures");
}

/**
 * Fetch the current league standings (20 rows).
 *
 * Sort order: points DESC, then goal difference DESC, then goals for DESC,
 * then club ID ASC (deterministic tie-break).
 */
export async function getStandings(): Promise<StandingsRow[]> {
  return invoke<StandingsRow[]>("get_standings");
}

/**
 * Fetch all 38 fixtures for the given club (19 home + 19 away), in
 * match-day order.
 *
 * Each entry includes the result if already played. Rejects with an
 * `IpcError` of kind `"clubNotFound"` if `clubId` is not in the current
 * league.
 *
 * Note: Tauri's command-arg deserializer accepts camelCase from the frontend.
 * The Rust handler receives `club_id: u32` and Tauri maps `{ clubId }` → it
 * automatically.
 */
export async function getFixtures(clubId: number): Promise<FixtureWithResult[]> {
  return invoke<FixtureWithResult[]>("get_fixtures", { clubId });
}

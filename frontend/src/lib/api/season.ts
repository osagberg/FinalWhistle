/**
 * Season controller IPC wrappers (T2-5).
 *
 * Thin `safeInvoke()` wrappers over the four Tauri commands:
 *   `advance_week`, `play_fixtures`, `get_standings`, `get_fixtures`.
 *
 * These are the ONLY functions that should call `invoke` for season-related
 * commands — route components import from here, not from `@tauri-apps/api/core`
 * directly, so the command-name strings have a single authoritative location.
 *
 * Post-T2-close Track C-2 gate-blocker fix: all 4 commands route through
 * `safeInvoke<T>(cmd, args, guard)` per the T1-3.6 audit-response pattern.
 * Bare `invoke<T>()` casts to T without runtime validation; backend DTO drift
 * silently NPEs deep in League.tsx / Transfers.tsx. The guards below catch
 * shape drift at the IPC seam + throw `IpcShapeError` with a payload preview.
 */
import {
  isAdvanceWeekSummary,
  isFixtureWithResultArray,
  isPlayFixturesSummary,
  isStandingsRowArray,
  safeInvoke,
} from "../runtime-validators";
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
 *
 * Throws `IpcShapeError` if the backend returns a payload that doesn't match
 * the `AdvanceWeekSummary` shape.
 */
export async function advanceWeek(): Promise<AdvanceWeekSummary> {
  return safeInvoke("advance_week", {}, isAdvanceWeekSummary);
}

/**
 * Fast-forward all remaining fixtures in one call.
 *
 * Plays every unplayed match-day sequentially and returns a summary with
 * the total matches played and the final match-day reached.
 *
 * If the season is already complete, returns `{ matchesPlayed: 0, finalMatchDay: 38 }`.
 *
 * Throws `IpcShapeError` if the backend returns a payload that doesn't match
 * the `PlayFixturesSummary` shape.
 */
export async function playFixtures(): Promise<PlayFixturesSummary> {
  return safeInvoke("play_fixtures", {}, isPlayFixturesSummary);
}

/**
 * Fetch the current league standings (20 rows).
 *
 * Sort order: points DESC, then goal difference DESC, then goals for DESC,
 * then club ID ASC (deterministic tie-break).
 *
 * Throws `IpcShapeError` if the response is not an array of valid
 * `StandingsRow` shapes.
 */
export async function getStandings(): Promise<StandingsRow[]> {
  return safeInvoke("get_standings", {}, isStandingsRowArray);
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
 *
 * Throws `IpcShapeError` if the response is not an array of valid
 * `FixtureWithResult` shapes.
 */
export async function getFixtures(clubId: number): Promise<FixtureWithResult[]> {
  return safeInvoke("get_fixtures", { clubId }, isFixtureWithResultArray);
}

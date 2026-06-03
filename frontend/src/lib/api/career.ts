/**
 * Career loop IPC wrappers (T3-9).
 *
 * Thin `safeInvoke()` wrappers over the two Tauri commands:
 *   `advance_season`, `get_career_overview`.
 *
 * These are the ONLY functions that should call `invoke` for career-related
 * commands — route components import from here, not from `@tauri-apps/api/core`
 * directly, so the command-name strings have a single authoritative location.
 *
 * Both commands route through `safeInvoke<T>(cmd, args, guard)` per the
 * T1-3.6 audit-response pattern. Bare `invoke<T>()` casts to T without
 * runtime validation; backend DTO drift silently NPEs deep in Career.tsx.
 * The guards catch shape drift at the IPC seam + throw `IpcShapeError`
 * with a payload preview.
 */
import {
  isAdvanceSeasonSummary,
  isCareerOverview,
  isPressInboxDto,
  safeInvoke,
} from "../runtime-validators";
import type { AdvanceSeasonSummary, CareerOverview, PressInboxDto } from "../types";

/**
 * Advance to the next season.
 *
 * Closes out the current season, records the champion, fires any cross-season
 * memory callbacks, and initialises the next season's fixture list.
 *
 * Rejects with an `IpcError` of kind `"seasonNotComplete"` if not all
 * current-season fixtures have been played.
 *
 * Throws `IpcShapeError` if the backend returns a payload that doesn't match
 * the `AdvanceSeasonSummary` shape.
 */
export async function advanceSeason(): Promise<AdvanceSeasonSummary> {
  return safeInvoke("advance_season", {}, isAdvanceSeasonSummary);
}

/**
 * Fetch the current career overview.
 *
 * Returns the current season number, per-season champion history (oldest
 * first), and any cross-season memory-event callbacks ready to surface.
 *
 * Throws `IpcShapeError` if the backend returns a payload that doesn't match
 * the `CareerOverview` shape.
 */
export async function getCareerOverview(): Promise<CareerOverview> {
  return safeInvoke("get_career_overview", {}, isCareerOverview);
}

/**
 * Fetch the press inbox for the current career.
 *
 * Returns the current season number and a list of press items ordered by
 * projected salience descending (event_id ascending tiebreak), capped at 20.
 * Empty `items` is the normal state at career start — it is NOT an error
 * condition.
 *
 * Throws `IpcShapeError` if the backend returns a payload that doesn't match
 * the `PressInboxDto` shape.
 */
export async function getPressInbox(): Promise<PressInboxDto> {
  return safeInvoke("get_press_inbox", {}, isPressInboxDto);
}

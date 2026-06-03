/**
 * Scout report IPC wrapper (T4-F4).
 *
 * Thin `safeInvoke()` wrapper over the `get_scout_report` Tauri command.
 * Single authoritative location for the command-name string so drift
 * between src-tauri registration and this call site is trivially greppable.
 *
 * The Tauri command is `get_scout_report(player_id: String)` — the Tauri
 * framework camelCases the arg to `playerId` on the wire, mirroring how
 * `getPlayerDetail` passes its id in `api/player.ts`.
 *
 * Error variants:
 *   `{ kind: "notYetObserved", playerId }` — roster player never observed.
 *   `{ kind: "playerNotFound", playerId }` — non-roster / content-bio id.
 *
 * Both are IpcError values thrown by the underlying `invoke`; callers should
 * distinguish them via `err.kind`.
 */
import { isScoutReportDto, safeInvoke } from "../runtime-validators";
import type { ScoutReportDto } from "../types";

/**
 * Fetch the latest scouting report for a roster player.
 *
 * `playerId` is the CONTENT-PACK-QUALIFIED id string (e.g.
 * `"fwh.core:player_00042"`) — exactly the same form `getPlayerDetail` takes.
 * The backend's `parse_player_id_suffix` REQUIRES the `:`-then-`_` form and
 * routes by the numeric suffix: suffix ≥ `ROSTER_PLAYER_ID_BASE` (1_000_000)
 * → a roster player (a roster id of 1_000_000 arrives as
 * `"fwh.core:player_01000000"`); a smaller suffix is a content-bio id and
 * yields `playerNotFound` (scouting is a roster-player feature). A bare number
 * string like `"42"` does NOT parse (no `:`) → `playerNotFound`.
 *
 * Throws `IpcShapeError` if the backend payload doesn't match `ScoutReportDto`.
 * Throws `{ kind: "notYetObserved", playerId }` if a roster player has not yet
 * been observed in a match. Throws `{ kind: "playerNotFound", playerId }` if
 * the id is not a roster player (content-bio id, or an unparseable id).
 */
export async function getScoutReport(playerId: string): Promise<ScoutReportDto> {
  return safeInvoke("get_scout_report", { playerId }, isScoutReportDto);
}

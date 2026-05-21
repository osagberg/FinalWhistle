/**
 * Player detail IPC wrapper (T3-6).
 *
 * Thin `safeInvoke()` wrapper over the `get_player_detail` Tauri command.
 * Single authoritative location for the command-name string so drift
 * between src-tauri registration and this call site is trivially greppable.
 */
import { isPlayerDetail, safeInvoke } from "../runtime-validators";
import type { PlayerDetail } from "../types";

/**
 * Fetch the full player detail DTO for a content-pack-qualified player ID.
 *
 * Returns a `PlayerDetail` with three blocks:
 *   - `phenotype`: bio data from the content store (name, role, region, labels).
 *   - `memoryCallbacks`: rendered career moment strings (empty when ledger is empty).
 *   - `contractStatus`: `null` until T4 career-roster layer.
 *
 * Throws `IpcShapeError` if the backend returns a payload that doesn't match
 * the `PlayerDetail` shape. Throws a serialised `IpcError` (including
 * `{ kind: "playerNotFound", playerId }`) if the player is not found.
 */
export async function getPlayerDetail(playerId: string): Promise<PlayerDetail> {
  return safeInvoke("get_player_detail", { playerId }, isPlayerDetail);
}

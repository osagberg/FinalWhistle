/**
 * Squad IPC wrapper (T2-7).
 *
 * Thin `safeInvoke()` wrapper over the `get_squad` Tauri command.
 * This is the only function that should call `invoke` for squad-related
 * commands — route components import from here so the command-name string
 * has a single authoritative location.
 *
 * Post-T1-3.6 pattern: all commands route through `safeInvoke<T>(cmd, args, guard)`
 * so backend DTO drift is caught at the IPC seam rather than propagating as
 * silent NPEs deep in Squad.tsx.
 */
import { isSquadPlayerArray, safeInvoke } from "../runtime-validators";
import type { SquadPlayer } from "../types";

/**
 * Fetch all players from the content store.
 *
 * Returns the 22-player pool in BTreeMap key order (content-pack-qualified
 * ID order — deterministic). Columns: name, role, birth region, phenotype
 * labels (human-readable strings). Age and contract are absent — they are
 * T4+ career-roster state.
 *
 * Throws `IpcShapeError` if the backend returns a payload that doesn't match
 * the `SquadPlayer[]` shape.
 */
export async function getSquad(): Promise<SquadPlayer[]> {
  return safeInvoke("get_squad", {}, isSquadPlayerArray);
}

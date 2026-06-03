/**
 * Squad IPC wrapper (T2-7 / T4-2.5h).
 *
 * Thin `safeInvoke()` wrappers over squad-related Tauri commands.
 * This is the only file that should call `invoke` for squad-related
 * commands — route components import from here so the command-name string
 * has a single authoritative location.
 *
 * Post-T1-3.6 pattern: all commands route through `safeInvoke<T>(cmd, args, guard)`
 * so backend DTO drift is caught at the IPC seam rather than propagating as
 * silent NPEs deep in Squad.tsx.
 */
import {
  isSquadPlayerArray,
  isSquadRosterDto,
  safeInvoke,
} from "../runtime-validators";
import type { SquadPlayer, SquadRosterDto } from "../types";

/**
 * Fetch all players from the content store (bio pool).
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

/**
 * Fetch the default club's squad roster from the career state.
 *
 * The "default club" is the lowest ClubId in the career roster — a
 * placeholder until career-start club selection is implemented.
 *
 * Returns a `SquadRosterDto` containing the club name and 22 slot-ordered
 * player rows with season stats (appearances, goals, assists, minutes).
 *
 * Throws `IpcShapeError` if the backend returns a payload that doesn't match
 * the `SquadRosterDto` shape.
 */
export async function getSquadRoster(): Promise<SquadRosterDto> {
  return safeInvoke("get_squad_roster", {}, isSquadRosterDto);
}

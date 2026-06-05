/**
 * New-career flow IPC wrappers (B4).
 *
 * Three commands for the career-start flow:
 *   `new_career`           — seeds a fresh world from a hex seed string.
 *   `get_clubs`            — returns the 20 clubs in the current league.
 *   `select_managed_club`  — sets the player's managed club for this career.
 *   `load_career`          — loads a previously saved career from disk.
 *   `save_career`          — persists the current career to disk.
 *
 * Pattern mirrors career.ts / season.ts: safeInvoke for shaped returns,
 * inline void guard (v === undefined || v === null) for unit-returning commands.
 */
import {
  isClubChoiceDtoArray,
  safeInvoke,
} from "../runtime-validators";
import type { ClubChoiceDto } from "../types";

/** Void guard — Tauri unit commands return null on the JS side. */
const isVoid = (v: unknown): v is undefined =>
  v === undefined || v === null;

/**
 * Start a fresh career world from a hex seed.
 *
 * `seedHex` must be a string like `"0xfeedbeefcafefade"`.
 * Re-seeds + regenerates the league, rosters, and season in place.
 *
 * Rejects with `IpcError::InvalidSeed`          — seed string malformed.
 * Rejects with `IpcError::LeagueGenerationFailed` — content store error.
 * Rejects with `IpcError::LockPoisoned`           — state lock corrupted.
 */
export async function newCareer(seedHex: string): Promise<void> {
  await safeInvoke("new_career", { seedHex }, isVoid);
}

/**
 * Fetch the 20 clubs in the current league.
 *
 * Called after `newCareer` to populate the club-selection list. Each entry
 * has a numeric `clubId` and a display `clubName`.
 *
 * Throws `IpcShapeError` if the payload doesn't match `ClubChoiceDto[]`.
 */
export async function getClubs(): Promise<ClubChoiceDto[]> {
  return safeInvoke("get_clubs", {}, isClubChoiceDtoArray);
}

/**
 * Set the player's managed club for the current career.
 *
 * `clubId` is the raw u32 from a `ClubChoiceDto`.
 *
 * Rejects with `IpcError::ClubNotFound` when `clubId` is not in the league.
 */
export async function selectManagedClub(clubId: number): Promise<void> {
  await safeInvoke("select_managed_club", { clubId }, isVoid);
}

/**
 * Load a previously saved career from disk.
 *
 * Rejects with `IpcError::SaveLoadFailed` on disk or decode error.
 */
export async function loadCareer(): Promise<void> {
  await safeInvoke("load_career", {}, isVoid);
}

/**
 * Persist the current career to disk.
 *
 * Rejects with `IpcError::SaveLoadFailed` on encode or write failure.
 */
export async function saveCareer(): Promise<void> {
  await safeInvoke("save_career", {}, isVoid);
}

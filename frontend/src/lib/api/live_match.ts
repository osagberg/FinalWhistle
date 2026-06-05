/**
 * Live-match IPC wrappers (T4-5a).
 *
 * Five `safeInvoke`-wrapped functions matching the ADR-0004 §1 command quintet.
 * Each wrapper validates the backend payload before returning — backend DTO
 * drift throws `IpcShapeError` at the IPC seam rather than silently propagating
 * as NPEs into the live-match UI.
 */

import {
  isFinalMatchResult,
  isMatchHandle,
  isMatchSnapshot,
  isStepResult,
  safeInvoke,
} from "../runtime-validators";
import type {
  FinalMatchResult,
  MatchCommand,
  MatchHandle,
  MatchSnapshot,
  StartLiveMatchForFixtureArgs,
  StepResult,
} from "../types";

/**
 * Allocate a new live-match session.
 *
 * Returns a `MatchHandle` that must be passed to all subsequent live-match
 * commands. The seed is the `"0x..."` hex string form.
 */
export async function startLiveMatch(seedHex: string): Promise<MatchHandle> {
  return safeInvoke("start_live_match", { seedHex }, isMatchHandle);
}

/**
 * Start a live-match session for a specific real fixture.
 *
 * Constructs the `MatchState` identically to `advance_week`'s AI-sim path
 * (same fixture seed, same per-club archetype IDs, same slot-signature
 * overrides). When no in-match decisions are made, stepping to completion
 * produces the same final score and canonical state as `advance_week` would
 * for the same fixture.
 *
 * @param args.homeClubId - Raw `ClubId` u32 of the home club.
 * @param args.awayClubId - Raw `ClubId` u32 of the away club.
 * @throws `IpcError::ClubNotFound` if either club is not in the current league.
 * @throws `IpcError::LeagueGenerationFailed` if no fixture exists for the pair.
 */
export async function startLiveMatchForFixture(
  args: StartLiveMatchForFixtureArgs,
): Promise<MatchHandle> {
  return safeInvoke(
    "start_live_match_for_fixture",
    { homeClubId: args.homeClubId, awayClubId: args.awayClubId },
    isMatchHandle,
  );
}

/**
 * Advance the live match by `ticks` simulation ticks.
 *
 * Returns events emitted during this call (delta), the current score, and
 * a flag indicating whether the match has reached FullTime.
 *
 * @throws `IpcError::TooManyFrames` when `ticks > MAX_FRAMES_PER_REQUEST`.
 * @throws `IpcError::MatchInitFailed` when the handle is not found.
 */
export async function stepLiveMatch(
  handle: MatchHandle,
  ticks: number,
): Promise<StepResult> {
  return safeInvoke("step_live_match", { handle, ticks }, isStepResult);
}

/**
 * Read the current match state as a fat DTO.
 *
 * Non-mutating. Powers scoreboard, lineup, and event-feed panels.
 *
 * @throws `IpcError::MatchInitFailed` when the handle is not found.
 */
export async function getMatchSnapshot(
  handle: MatchHandle,
): Promise<MatchSnapshot> {
  return safeInvoke("get_match_snapshot", { handle }, isMatchSnapshot);
}

/**
 * Remove the live-match session and return the final result.
 *
 * After this call the handle is invalid. Further calls with the same handle
 * will return `IpcError::MatchInitFailed`.
 *
 * @throws `IpcError::MatchInitFailed` when the handle is not found.
 */
export async function finishLiveMatch(
  handle: MatchHandle,
): Promise<FinalMatchResult> {
  return safeInvoke("finish_live_match", { handle }, isFinalMatchResult);
}

/**
 * Enqueue a manager intent.
 *
 * All 9 `MatchCommand` variants currently throw `IpcError::LiveMatchCommandUnimplemented`.
 * The command is recorded in the session's audit trail before the error is returned.
 *
 * @throws `IpcError::LiveMatchCommandUnimplemented` always (T4-5a).
 * @throws `IpcError::MatchInitFailed` when the handle is not found.
 */
export async function applyMatchCommand(
  handle: MatchHandle,
  command: MatchCommand,
): Promise<void> {
  // `apply_match_command` returns `()` on success — always `undefined` over IPC.
  // Use a trivial guard that accepts `undefined`.
  await safeInvoke(
    "apply_match_command",
    { handle, command },
    (v): v is undefined => v === undefined || v === null,
  );
}

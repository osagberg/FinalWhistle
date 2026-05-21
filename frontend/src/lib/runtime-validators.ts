/**
 * Runtime shape validators for IPC payloads — T1-3.6 Codex audit response.
 *
 * Codex's post-T1-7 adversarial multi-agent audit P1: `invoke<T>()` casts
 * its result to `T` without runtime validation. Backend wire-shape drift
 * (e.g. the MatchEvent enum→DTO regression Codex caught at Tier-2 post-T1-6)
 * silently lands in TypeScript at the call site + the resulting `T`-shaped
 * `any` propagates into Solid signals, deep into the PixiJS render path,
 * and finally NPEs miles from the source. This module ships per-DTO type
 * guards (mirroring the `isIpcError` pattern shipped at T1-5) + a
 * `safeInvoke<T>(cmd, args, guard)` wrapper that validates before returning.
 *
 * # Coverage
 *
 * Three guards for the three commands fw-tauri ships:
 *   - `isMatchResult` — `playMatch` return shape
 *   - `isMatchFrameDTO` — `match_frames` array-element shape
 *   - `isBackendHandshake` — `getBackendHandshake` return shape
 *
 * Each guard checks:
 *   1. Top-level shape (object, not null/undefined/primitive)
 *   2. Every required field present + correct primitive type
 *   3. Nested object/array shapes via recursive guards
 *
 * # Failure mode
 *
 * On shape mismatch, `safeInvoke` throws an `IpcShapeError` carrying the
 * command name + a brief reason + the actual payload (truncated for log
 * sanity). The catch site can distinguish this from `IpcError` (typed
 * backend errors) via `instanceof IpcShapeError`.
 *
 * # Not a schema validator
 *
 * No zod/io-ts dependency (50KB+ for a dev-only command surface). Hand-
 * written guards are tight against what each consumer actually reads;
 * adding a new field to a DTO requires a corresponding guard update.
 * Trade-off explicit per Codex Tier-2 pattern at T1-5's IpcError narrowing.
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  AdvanceWeekSummary,
  BackendHandshake,
  FixtureWithResult,
  MatchEvent,
  MatchEventKind,
  MatchFrameDTO,
  MatchResult,
  PlayFixturesSummary,
  PlayerDetail,
  PlayerPhenotype,
  Score,
  SquadPlayer,
  StandingsRow,
} from "./types";

// ---------------------------------------------------------------------------
// IpcShapeError — thrown by safeInvoke on guard failure
// ---------------------------------------------------------------------------

export class IpcShapeError extends Error {
  readonly command: string;
  readonly reason: string;
  readonly payloadPreview: string;

  constructor(command: string, reason: string, payload: unknown) {
    const preview = JSON.stringify(payload, null, 0).slice(0, 200);
    super(
      `IPC shape mismatch on '${command}': ${reason}. Payload preview: ${preview}`,
    );
    this.name = "IpcShapeError";
    this.command = command;
    this.reason = reason;
    this.payloadPreview = preview;
  }
}

// ---------------------------------------------------------------------------
// Closed-union helper for MatchEventKind
// ---------------------------------------------------------------------------

const KNOWN_MATCH_EVENT_KINDS = new Set([
  "Goal",
  "Shot",
  "Pass",
  "KickOff",
  "HalfTime",
  "FullTime",
  "Card",
  "Substitution",
  "SignatureFirstFired",
] as const) satisfies ReadonlySet<MatchEventKind>;

function isMatchEventKind(v: unknown): v is MatchEventKind {
  return (
    typeof v === "string" &&
    (KNOWN_MATCH_EVENT_KINDS as ReadonlySet<string>).has(v)
  );
}

// ---------------------------------------------------------------------------
// Type guards
// ---------------------------------------------------------------------------

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

export function isScore(v: unknown): v is Score {
  if (!isObject(v)) return false;
  return typeof v.home === "number" && typeof v.away === "number";
}

export function isMatchEvent(v: unknown): v is MatchEvent {
  if (!isObject(v)) return false;
  if (typeof v.tick !== "number" || !Number.isFinite(v.tick)) return false;
  if (typeof v.minute !== "number" || !Number.isFinite(v.minute)) return false;
  if (!isMatchEventKind(v.kind)) return false;
  // `description` is optional + serde-omitted when None on Rust side.
  if (v.description !== undefined && typeof v.description !== "string") {
    return false;
  }
  return true;
}

export function isMatchResult(v: unknown): v is MatchResult {
  if (!isObject(v)) return false;
  if (!isScore(v.finalScore)) return false;
  if (typeof v.canonicalHash !== "string") return false;
  if (!v.canonicalHash.startsWith("blake3:")) return false;
  if (!Array.isArray(v.matchEvents)) return false;
  if (!v.matchEvents.every(isMatchEvent)) return false;
  if (typeof v.seedHex !== "string") return false;
  if (typeof v.tickCount !== "number") return false;
  if (!Array.isArray(v.commentaryPreview)) return false;
  if (!v.commentaryPreview.every((s) => typeof s === "string")) return false;
  return true;
}

function isPlayerFrameDTO(
  v: unknown,
): v is MatchFrameDTO["players"][number] {
  if (!isObject(v)) return false;
  // T1-3.6 self-review (silent-failure-hunter P3-1 + type-design-analyzer P2-2):
  // slot must be an integer in 0..21 to match canonical PlayerSlot=u8 bounds.
  // The prior `typeof === "number" && isFinite` check accepted 3.14, -1, 99,
  // any of which would silently highlight the wrong dot on the tactical board.
  if (typeof v.slot !== "number" || !Number.isInteger(v.slot)) return false;
  if (v.slot < 0 || v.slot > 21) return false;
  for (const key of ["posX", "posY", "velX", "velY"] as const) {
    const val = v[key];
    if (typeof val !== "number" || !Number.isFinite(val)) return false;
  }
  return true;
}

function isBallFrameDTO(v: unknown): v is MatchFrameDTO["ball"] {
  if (!isObject(v)) return false;
  for (const key of [
    "posX",
    "posY",
    "posZ",
    "velX",
    "velY",
    "velZ",
  ] as const) {
    const val = v[key];
    if (typeof val !== "number" || !Number.isFinite(val)) return false;
  }
  return true;
}

export function isMatchFrameDTO(v: unknown): v is MatchFrameDTO {
  if (!isObject(v)) return false;
  // T1-3.6 self-review (silent-failure-hunter P3-2 / type-design-analyzer
  // P2-2 / code-reviewer P2): this guard absorbs what `FrameSource.ts`'s
  // older `isMatchFrame` used to enforce, so HttpFrameSource + TauriFrameSource
  // share ONE source of truth. The folded-in checks are:
  //   - seedHex non-empty (catches malformed fixtures missing the seed)
  //   - player slots are unique (catches a duplicate-slot regression in
  //     dump_frames that would silently overwrite dots on the tactical board)
  //   - possession bounded to 0..21 (matches canonical PlayerSlot=u8 range)
  if (typeof v.seedHex !== "string" || v.seedHex.length === 0) return false;
  if (typeof v.tick !== "number" || !Number.isFinite(v.tick)) return false;
  if (typeof v.homeScore !== "number") return false;
  if (typeof v.awayScore !== "number") return false;
  if (!Array.isArray(v.players)) return false;
  if (v.players.length !== 22) return false;
  if (!v.players.every(isPlayerFrameDTO)) return false;
  // Slot uniqueness — two frames with `slot: 3` would silently shadow each
  // other on the tactical board renderer.
  const seenSlots = new Set<number>();
  for (const p of v.players as { slot: number }[]) {
    if (seenSlots.has(p.slot)) return false;
    seenSlots.add(p.slot);
  }
  if (!isBallFrameDTO(v.ball)) return false;
  // `possession` is `Option<u8>` on Rust → null | number-bounded-to-0..21.
  // Reject 3.14, -1, 99, NaN, Infinity — every one of which would render
  // a wrong-dot highlight that's hard to debug after-the-fact.
  if (v.possession === null) {
    // fine — loose ball / pre-kickoff
  } else if (
    typeof v.possession === "number" &&
    Number.isInteger(v.possession) &&
    v.possession >= 0 &&
    v.possession <= 21
  ) {
    // fine — valid slot
  } else {
    return false;
  }
  return true;
}

export function isMatchFrameDTOArray(v: unknown): v is MatchFrameDTO[] {
  return Array.isArray(v) && v.every(isMatchFrameDTO);
}

export function isBackendHandshake(v: unknown): v is BackendHandshake {
  if (!isObject(v)) return false;
  if (typeof v.appVersion !== "string") return false;
  if (typeof v.message !== "string") return false;
  if (typeof v.backendReady !== "boolean") return false;
  return true;
}

// ---------------------------------------------------------------------------
// T2-9 phase-close Track C-2 gate-blocker fix: runtime shape guards for the
// 4 T2-5 season-controller IPC commands. Prior `season.ts` invoke wrappers
// skipped runtime validation entirely — undoing the T1-3.6 audit response
// one phase later. Backend DTO drift on advance_week / play_fixtures /
// get_standings / get_fixtures would silently NPE deep in League.tsx /
// Transfers.tsx. These guards close the gap.
// ---------------------------------------------------------------------------

function isU16(v: unknown): v is number {
  return (
    typeof v === "number" &&
    Number.isInteger(v) &&
    v >= 0 &&
    v <= 65535
  );
}

function isU32(v: unknown): v is number {
  return (
    typeof v === "number" &&
    Number.isInteger(v) &&
    v >= 0 &&
    v <= 4294967295
  );
}

function isI32(v: unknown): v is number {
  return (
    typeof v === "number" &&
    Number.isInteger(v) &&
    v >= -2147483648 &&
    v <= 2147483647
  );
}

function isU8OrNull(v: unknown): boolean {
  if (v === null || v === undefined) return true;
  return typeof v === "number" && Number.isInteger(v) && v >= 0 && v <= 255;
}

export function isAdvanceWeekSummary(v: unknown): v is AdvanceWeekSummary {
  if (!isObject(v)) return false;
  if (!isU16(v.matchDayPlayed)) return false;
  if (!isU16(v.matchesPlayed)) return false;
  if (typeof v.seasonComplete !== "boolean") return false;
  return true;
}

export function isPlayFixturesSummary(v: unknown): v is PlayFixturesSummary {
  if (!isObject(v)) return false;
  if (!isU32(v.matchesPlayed)) return false;
  if (!isU16(v.finalMatchDay)) return false;
  return true;
}

export function isStandingsRow(v: unknown): v is StandingsRow {
  if (!isObject(v)) return false;
  if (!isU32(v.clubId)) return false;
  if (typeof v.clubName !== "string") return false;
  for (const key of [
    "played",
    "wins",
    "draws",
    "losses",
    "goalsFor",
    "goalsAgainst",
    "points",
  ] as const) {
    if (!isU16(v[key])) return false;
  }
  if (!isI32(v.goalDifference)) return false;
  return true;
}

export function isStandingsRowArray(v: unknown): v is StandingsRow[] {
  return Array.isArray(v) && v.every(isStandingsRow);
}

export function isFixtureWithResult(v: unknown): v is FixtureWithResult {
  if (!isObject(v)) return false;
  if (!isU16(v.matchDay)) return false;
  if (!isU32(v.opponentClubId)) return false;
  if (typeof v.opponentClubName !== "string") return false;
  if (typeof v.isHome !== "boolean") return false;
  if (typeof v.played !== "boolean") return false;
  // homeScore + awayScore are Option<u8> serde-skipped when None — accept
  // undefined OR a u8-bounded integer.
  if (!isU8OrNull(v.homeScore)) return false;
  if (!isU8OrNull(v.awayScore)) return false;
  return true;
}

export function isFixtureWithResultArray(v: unknown): v is FixtureWithResult[] {
  return Array.isArray(v) && v.every(isFixtureWithResult);
}

// ---------------------------------------------------------------------------
// T2-7 squad guards
// ---------------------------------------------------------------------------

export function isSquadPlayer(v: unknown): v is SquadPlayer {
  if (!isObject(v)) return false;
  if (typeof v.playerId !== "string") return false;
  if (typeof v.name !== "string") return false;
  if (typeof v.role !== "string") return false;
  if (typeof v.birthRegion !== "string") return false;
  if (!Array.isArray(v.phenotypeLabels)) return false;
  if (!v.phenotypeLabels.every((l) => typeof l === "string")) return false;
  return true;
}

export function isSquadPlayerArray(v: unknown): v is SquadPlayer[] {
  return Array.isArray(v) && v.every(isSquadPlayer);
}

// ---------------------------------------------------------------------------
// T3-6 PlayerDetail guards
// ---------------------------------------------------------------------------

function isPlayerPhenotype(v: unknown): v is PlayerPhenotype {
  if (!isObject(v)) return false;
  if (typeof v.playerId !== "string") return false;
  if (typeof v.name !== "string") return false;
  if (typeof v.role !== "string") return false;
  if (typeof v.birthRegion !== "string") return false;
  if (!Array.isArray(v.phenotypeLabels)) return false;
  if (!v.phenotypeLabels.every((l) => typeof l === "string")) return false;
  return true;
}

export function isPlayerDetail(v: unknown): v is PlayerDetail {
  if (!isObject(v)) return false;
  if (!isPlayerPhenotype(v.phenotype)) return false;
  if (!Array.isArray(v.memoryCallbacks)) return false;
  if (!v.memoryCallbacks.every((s) => typeof s === "string")) return false;
  // contractStatus is string | null
  if (v.contractStatus !== null && typeof v.contractStatus !== "string") {
    return false;
  }
  return true;
}

// ---------------------------------------------------------------------------
// safeInvoke — invoke + runtime-validate the response
// ---------------------------------------------------------------------------

/**
 * `invoke()` wrapper that validates the returned payload via `guard` before
 * casting to `T`. On guard failure throws `IpcShapeError` with the command
 * name + a payload preview, so the catch site can distinguish runtime shape
 * drift from `IpcError` (typed backend errors).
 *
 * Use this for every Tauri command that returns a structured payload —
 * the cost is small (~10 microseconds per validate) and the failure mode
 * goes from "deep PixiJS NPE" to "clear shape mismatch at the IPC seam."
 */
export async function safeInvoke<T>(
  command: string,
  args: Record<string, unknown>,
  guard: (v: unknown) => v is T,
): Promise<T> {
  const raw: unknown = await invoke(command, args);
  if (!guard(raw)) {
    throw new IpcShapeError(
      command,
      "response failed runtime shape guard",
      raw,
    );
  }
  return raw;
}

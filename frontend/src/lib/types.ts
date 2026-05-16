/*
 * Wire types — MUST stay in sync with fw-tauri Rust DTOs.
 *
 * Naming convention: TypeScript uses camelCase; Rust uses snake_case but the
 * fw-tauri structs carry `#[serde(rename_all = "camelCase")]` so the boundary
 * is camelCase on both sides.
 *
 * T1-5: LeagueStanding / Fixture / PlayerSummary deleted (were T0-2 stubs;
 * the real types land at T2-6/T2-7 via fw-tauri season controller commands).
 * MatchResult / Score / IpcError added to match fw-tauri::result + fw-tauri::error.
 */

// ---------------------------------------------------------------------------
// Liveness check — returned by get_backend_handshake
//
// Codex 2026-05-16 Tier-2 fix-pass: renamed from `DummyState` returned by
// `get_dummy_state` after T1-5 consolidation accidentally repurposed
// `get_dummy_state` to return `MatchStateDto` (sim state) while leaving
// Home.tsx still reading `appVersion`/`message`/`backendReady`. Now the
// command + the type names match what Home.tsx actually consumes.
// ---------------------------------------------------------------------------

export interface BackendHandshake {
  appVersion: string;
  message: string;
  backendReady: boolean;
}

// ---------------------------------------------------------------------------
// Match types — returned by play_match
// ---------------------------------------------------------------------------

export interface Score {
  home: number;
  away: number;
}

/**
 * Match event kinds the sim emits.
 *
 * T1-6 fix-pass per type-design P1 + silent-failure P3: this is a CLOSED
 * discriminated union (no `| string` escape hatch). When the sim adds a new
 * variant in `fw-content::event::MatchEvent`, this type must be updated in
 * lockstep — at which point every exhaustive `switch (kind)` in the UI
 * fails to compile, surfacing the drift loudly. The prior `| string` form
 * widened the union to `string` and defeated exhaustiveness in
 * `eventLabel` / `badgeClass` switches that returned the raw kind string
 * silently for unknown variants.
 *
 * For forward-compat with sim variants the UI hasn't been updated for yet,
 * add a `parseMatchEvent` boundary function that maps unknown strings to
 * a sentinel `"Unknown"` variant — but only after a variant is actually
 * added. Today the sim catalogue + UI catalogue are in lockstep.
 */
export type MatchEventKind =
  | "Goal"
  | "Shot"
  | "Pass"
  | "KickOff"
  | "HalfTime"
  | "FullTime"
  | "Card"
  | "Substitution"
  | "SignatureFirstFired";

export interface MatchEvent {
  tick: number;
  minute: number;
  kind: MatchEventKind;
  description?: string;
}

/**
 * Full match result returned by `play_match`.
 *
 * `canonicalHash` is `"blake3:<64-hex-chars>"` — the BLAKE3 digest of
 * `MatchState::encode_canonical()` after `tickCount` ticks. Use for
 * pinned-corpus regression in QA.
 *
 * `commentaryPreview` is pre-rendered prose, one line per `matchEvents`
 * entry, so the Match page shows a text recap without a second round-trip.
 */
export interface MatchResult {
  finalScore: Score;
  canonicalHash: string;
  matchEvents: MatchEvent[];
  seedHex: string;
  tickCount: number;
  commentaryPreview: string[];
}

// ---------------------------------------------------------------------------
// IpcError — discriminated union matching fw-tauri::IpcError serde shape.
//
// `#[serde(tag = "kind", rename_all = "camelCase")]` on the Rust side means:
//   { kind: "tooManyFrames", requested: 7201, max: 7200 }
//   { kind: "invalidSeed", input: "0xggg", reason: "..." }
//   { kind: "matchInitFailed", reason: "..." }
// ---------------------------------------------------------------------------

export type IpcError =
  | { kind: "tooManyFrames"; requested: number; max: number }
  | { kind: "invalidSeed"; input: string; reason: string }
  | { kind: "matchInitFailed"; reason: string };

// ---------------------------------------------------------------------------
// Frame-cap constant — mirrors `fw_tauri::MAX_FRAMES_PER_REQUEST`.
//
// Single source of truth is the Rust const. This TS mirror is validated by
// the IPC contract test in `crates/fw-tauri/tests/ipc_contract_test.rs`.
// Update both when the cap changes.
// ---------------------------------------------------------------------------

/** Maximum frames per `match_frames` request (= 2 min at 60 Hz). */
export const MAX_FRAMES_PER_REQUEST = 7200 as const;

// ---------------------------------------------------------------------------
// T1-2a tactical board DTOs — mirrors fw-match-sim::dto (camelCase serde)
// ---------------------------------------------------------------------------

export interface PlayerFrameDTO {
  slot: number;
  posX: number;
  posY: number;
  velX: number;
  velY: number;
}

export interface BallFrameDTO {
  posX: number;
  posY: number;
  posZ: number;
  velX: number;
  velY: number;
  velZ: number;
}

/** Per-tick snapshot from the sim. One entry per integration step. */
export interface MatchFrameDTO {
  seedHex: string;
  tick: number;
  homeScore: number;
  awayScore: number;
  players: PlayerFrameDTO[];
  ball: BallFrameDTO;
  /**
   * Slot index (0-21) of the player currently in possession, or `null`
   * for loose-ball / set-piece pause states.
   *
   * T1-3.6: added so the dev board (and future Tauri consumers) can
   * visualize the carrier — prior frames misleadingly displayed a
   * dead-ball game even when the sim had assigned possession.
   * Mirrors `MatchFrameDto.possession: Option<u8>` on the Rust side.
   */
  possession: number | null;
}

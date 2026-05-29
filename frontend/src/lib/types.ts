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
//   { kind: "seasonComplete" }                                     (T2-5)
//   { kind: "clubNotFound", clubId: 99999 }                        (T2-5)
//   { kind: "lockPoisoned", lock: "season" }                       (T2-5)
//
// Post-T2-5 code-reviewer P0 fix: prior union omitted the three season-
// controller variants. Any TS caller pattern-matching on `IpcError` would
// silently fall through on those discriminants — a real coverage gap given
// `advance_week` is called in a loop driven by checking for `seasonComplete`.
// ---------------------------------------------------------------------------

export type IpcError =
  | { kind: "tooManyFrames"; requested: number; max: number }
  | { kind: "invalidSeed"; input: string; reason: string }
  | { kind: "matchInitFailed"; reason: string }
  | { kind: "seasonComplete" }
  | { kind: "clubNotFound"; clubId: number }
  | { kind: "lockPoisoned"; lock: string }
  | { kind: "playerNotFound"; playerId: string }
  | { kind: "seasonNotComplete" }
  | {
      kind: "liveMatchCommandUnimplemented";
      /** camelCase command kind, e.g. `"substitute"`. */
      commandKind: string;
    };

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
// T2-5 season controller DTOs — mirrors fw-tauri season command return types
// ---------------------------------------------------------------------------

/** Returned by `advance_week`. */
export interface AdvanceWeekSummary {
  matchDayPlayed: number;
  matchesPlayed: number;
  seasonComplete: boolean;
}

/** Returned by `play_fixtures`. */
export interface PlayFixturesSummary {
  matchesPlayed: number;
  finalMatchDay: number;
}

/** One row in the league standings table, returned by `get_standings`. */
export interface StandingsRow {
  clubId: number;
  clubName: string;
  played: number;
  wins: number;
  draws: number;
  losses: number;
  goalsFor: number;
  goalsAgainst: number;
  goalDifference: number;
  points: number;
}

/**
 * One fixture entry returned by `get_fixtures(clubId)`.
 *
 * `homeScore` / `awayScore` are absent (not serialized) for unplayed
 * fixtures — the Rust side uses `skip_serializing_if = "Option::is_none"`.
 * Consumers should check `played` rather than presence of score fields.
 */
export interface FixtureWithResult {
  matchDay: number;
  opponentClubId: number;
  opponentClubName: string;
  isHome: boolean;
  played: boolean;
  homeScore?: number;
  awayScore?: number;
}

// ---------------------------------------------------------------------------
// T2-7 squad DTO — mirrors fw-tauri::SquadPlayerDto
// ---------------------------------------------------------------------------

/**
 * One player row returned by `get_squad`.
 *
 * Age and contract are absent by design — they are T4+ career-roster state
 * that `PlayerBio` does not carry. Phenotype labels are human-readable strings
 * (e.g. "Explosive first step"), never raw Rust enum identifiers.
 */
export interface SquadPlayer {
  playerId: string;
  name: string;
  role: string;
  birthRegion: string;
  phenotypeLabels: string[];
}

// ---------------------------------------------------------------------------
// T3-6 Player detail DTO — mirrors fw-tauri::PlayerDetailDto (camelCase serde)
// ---------------------------------------------------------------------------

/**
 * Player phenotype block — name, role, region, phenotype labels.
 *
 * Sourced from PlayerBio in the content store. Age and contract are absent
 * by design — they are T4+ career-roster state.
 */
export interface PlayerPhenotype {
  playerId: string;
  name: string;
  role: string;
  birthRegion: string;
  phenotypeLabels: string[];
}

/**
 * Full player detail returned by `get_player_detail`.
 *
 * Three blocks:
 * - `phenotype`: bio data from the content store.
 * - `memoryCallbacks`: rendered career moment strings (empty when ledger is empty).
 * - `contractStatus`: `null` until T4 career-roster layer.
 */
export interface PlayerDetail {
  phenotype: PlayerPhenotype;
  memoryCallbacks: string[];
  contractStatus: string | null;
}

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

// ---------------------------------------------------------------------------
// T4-5a live-match DTOs — mirrors fw-tauri::live_match::types (camelCase serde)
// ---------------------------------------------------------------------------

/**
 * An opaque reference to an active live-match session.
 *
 * `id` is the key in AppState's live-match map. `seedHex` is informational.
 * The frontend treats the handle as opaque (ADR-0004 §1).
 */
export interface MatchHandle {
  id: number;
  seedHex: string;
}

/** Score pair within a live-match context (separate from `Score` in MatchResult). */
export interface LiveScoreDto {
  home: number;
  away: number;
}

/** Possession percentages. `homePct + awayPct === 100` (within rounding). */
export interface PossessionDto {
  homePct: number;
  awayPct: number;
}

/** Coarse match phase derived from tick + emitted events. */
export type MatchPhase = "firstHalf" | "halfTime" | "secondHalf" | "fullTime";

/**
 * 5-bucket pitch zone from home's perspective.
 * Derived from ball pos_x (home defends negative X, attacks positive X).
 */
export type BallZone =
  | "ownDefensiveThird"
  | "ownMidThird"
  | "center"
  | "oppMidThird"
  | "oppAttackingThird";

/** 11-slot team lineup. `players[i]` is the raw PlayerId u32 for slot `i`. */
export interface LineupDto {
  players: number[];
}

/** Returned by `step_live_match`. */
export interface StepResult {
  handle: MatchHandle;
  /** Events emitted during this step only (delta since the previous call). */
  newEvents: MatchEvent[];
  score: LiveScoreDto;
  tick: number;
  isFinished: boolean;
}

/**
 * Fat read DTO returned by `get_match_snapshot`.
 *
 * Powers scoreboard, lineup, and event-feed panels (ADR-0004 §3).
 * `yellowCards` and `sentOff` are empty at T1 (no card system).
 */
export interface MatchSnapshot {
  handle: MatchHandle;
  tick: number;
  minute: number;
  phase: MatchPhase;
  score: LiveScoreDto;
  possessionPct: PossessionDto;
  ballZone: BallZone;
  homeLineup: LineupDto;
  awayLineup: LineupDto;
  /** Last 16 events in chronological order. */
  recentEvents: MatchEvent[];
  /** Per-player yellow-card count. Empty at T1. */
  yellowCards: Record<number, number>;
  /** Players sent off (raw PlayerId u32 values). Empty at T1. */
  sentOff: number[];
}

/** Returned by `finish_live_match`. */
export interface FinalMatchResult {
  handle: MatchHandle;
  finalScore: LiveScoreDto;
  tick: number;
  totalEvents: number;
}

// ---------------------------------------------------------------------------
// MatchCommand — closed discriminated union (ADR-0004 §2)
// ---------------------------------------------------------------------------

export type PressLevel = "low" | "mid" | "high";
export type TempoBias = "slow" | "even" | "fast";

/**
 * Manager intent, enqueued between ticks.
 *
 * Closed set: new variants need a logged decision (ADR-0004 §2).
 * All 9 variants currently return `IpcError::LiveMatchCommandUnimplemented`.
 * `playerId` fields carry raw u32 values matching `PlayerState::slot`.
 */
export type MatchCommand =
  | { kind: "substitute"; playerIn: number; playerOut: number }
  | { kind: "changeFormation"; formation: string }
  | { kind: "changePressLevel"; level: PressLevel }
  | { kind: "changeTempoBias"; bias: TempoBias }
  | { kind: "setCornerTaker"; player: number }
  | { kind: "setFreeKickTaker"; player: number }
  | { kind: "setPenaltyTaker"; player: number }
  | { kind: "setCaptain"; player: number }
  | { kind: "teamTalk"; messageId: string };

/**
 * Canonical `kind` strings for all 9 `MatchCommand` variants.
 *
 * Mirrors `KNOWN_MATCH_COMMAND_KINDS` in `crates/fw-tauri/src/live_match/types.rs`.
 * The `satisfies` clause pins this tuple to the `MatchCommand["kind"]` union
 * so a new variant in Rust produces a TS compile error here.
 */
export const KNOWN_LIVE_MATCH_COMMAND_KINDS = [
  "substitute",
  "changeFormation",
  "changePressLevel",
  "changeTempoBias",
  "setCornerTaker",
  "setFreeKickTaker",
  "setPenaltyTaker",
  "setCaptain",
  "teamTalk",
] as const satisfies readonly MatchCommand["kind"][];

// ---------------------------------------------------------------------------
// T3-9 career-loop DTOs — mirrors fw-tauri career command return types
// ---------------------------------------------------------------------------

/** One season entry in the career history list, returned as part of CareerOverview. */
export interface ChampionHistoryEntry {
  season: number;
  championClubName: string;
}

/**
 * Returned by `advance_season`.
 *
 * Rejects with `IpcError { kind: "seasonNotComplete" }` if the current
 * season's fixtures have not all been played.
 */
export interface AdvanceSeasonSummary {
  completedSeason: number;
  championClubName: string;
  newSeasonNumber: number;
  compactionFired: boolean;
}

/**
 * Returned by `get_career_overview`.
 *
 * `history` is ordered oldest-to-newest. `crossSeasonCallbacks` are
 * rendered memory-event strings surfaced at season-advance time.
 */
export interface CareerOverview {
  seasonNumber: number;
  history: ChampionHistoryEntry[];
  crossSeasonCallbacks: string[];
}

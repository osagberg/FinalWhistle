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
  | "SignatureFirstFired"
  | "Offside"
  // FUN-CB1: failed pass — spawns loose ball, clears possession.
  | "PassIncomplete";

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
    }
  | {
      kind: "settingsLoadFailed";
      /** Human-readable decode failure reason. Not shown in player-facing UI. */
      reason: string;
    }
  | {
      kind: "notYetObserved";
      /** Content-pack-qualified player id, e.g. `"fwh.core:player_00042"`. */
      playerId: string;
    }
  | {
      kind: "leagueGenerationFailed";
      /** Human-readable reason from the content store. Not shown raw in player-facing UI. */
      reason: string;
    }
  | {
      kind: "saveLoadFailed";
      /** Save I/O or decode failure reason (disk full, permission, corrupted/future save). Not shown raw in player-facing UI. */
      reason: string;
    };

// ---------------------------------------------------------------------------
// T4-6a settings DTOs — mirrors fw-tauri::AppSettingsDto (camelCase serde)
// ---------------------------------------------------------------------------

/** Colour scheme preference. Mirrors `ThemePrefDto` in fw-tauri. */
export type ThemePref = "light" | "dark";

/**
 * App settings payload returned by `get_settings` / accepted by `set_settings`.
 *
 * `theme`: `"light"` or `"dark"`.
 * `reduceMotion`: when `true`, the UI applies `.reduce-motion` to suppress
 * CSS transitions and animations.
 */
export interface AppSettingsDto {
  theme: ThemePref;
  reduceMotion: boolean;
}

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
// T4-2.5b roster DTO — mirrors fw-tauri::roster_dto::PlayerRosterDto
// ---------------------------------------------------------------------------

/**
 * One player row returned by `get_roster_for_club(clubId)`.
 *
 * Contains identity + slot + season statistics. No overall / rating number —
 * internal metrics are surfaced as commentary only (CLAUDE.md §7).
 *
 * Stats are zero at career start; accumulated by `update_player_stats_from_match`
 * in T4-2.5e/h.
 */
export interface PlayerRosterDto {
  /** Raw u32 career-unique player handle. */
  playerId: number;
  /** Display name for this player. */
  name: string;
  /** Raw u32 of the owning club. */
  clubId: number;
  /** Squad slot (0 = GK, 1–21 = outfield). */
  slot: number;
  /** Appearances this season. */
  appearances: number;
  /** Goals this season. */
  goals: number;
  /** Assists this season. */
  assists: number;
  /** Minutes played this season. */
  minutesPlayed: number;
}

// ---------------------------------------------------------------------------
// T4-2.5h squad-roster DTO — mirrors fw-tauri::roster_dto::SquadRosterDto
// ---------------------------------------------------------------------------

/**
 * Returned by `get_squad_roster`.
 *
 * Bundles the displayed club identity with its 22-player roster rows. The club
 * is the player's chosen managed club when `isManaged` is true; otherwise it is
 * the lowest-ClubId placeholder (no club chosen yet, or the managed club is
 * absent from the current roster).
 */
export interface SquadRosterDto {
  /** Raw u32 of the club being displayed. */
  clubId: number;
  /** Display name resolved from the current season's league. */
  clubName: string;
  /** 22 slot-ordered player rows. */
  players: PlayerRosterDto[];
  /**
   * True when this is the player's chosen managed club; false when showing the
   * lowest-ClubId placeholder. Drives whether the screen shows the
   * "no club selected" placeholder label.
   */
  isManaged: boolean;
}

/**
 * One club in the club-selection list. Mirrors `fw-tauri::commands::ClubChoiceDto`.
 * Returned by `get_clubs`.
 */
export interface ClubChoiceDto {
  /** Raw u32 ClubId. */
  clubId: number;
  /** Club display name from the current league. */
  clubName: string;
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

// M2a: start_live_match_for_fixture args — mirrors the command parameters in
// `fw-tauri::commands::start_live_match_for_fixture`. Returns a `MatchHandle`.
export interface StartLiveMatchForFixtureArgs {
  /** Raw `ClubId` u32 of the home club. Must exist in the current league. */
  homeClubId: number;
  /** Raw `ClubId` u32 of the away club. Must exist in the current league. */
  awayClubId: number;
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
  /**
   * Position frame projected from the live session's `MatchState` at
   * `result.tick`. Contains 22 player entries (slots 0-21) plus ball with
   * full 3D position and velocity. Shares the `MatchFrameDTO` shape so the
   * tactical board can consume it without re-simming independently. The frame
   * is a one-way read projection (Tauri IPC §3) — never written back.
   */
  frame: MatchFrameDTO;
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

// ---------------------------------------------------------------------------
// T4-2.5k press-inbox DTOs — mirrors fw-tauri::press_dto (camelCase serde)
// ---------------------------------------------------------------------------

/**
 * Topic discriminant for a press-inbox item.
 *
 * Mirrors the wire strings emitted by `fw_memory::readers::PressTopic` (mapped
 * to these strings in `get_press_inbox_inner`, fw-tauri `commands.rs`). There is
 * no Rust `PressTopicDto` type — the DTO field is a plain `String` on the wire.
 * Closed union — adding a new variant requires an IPC-contract update.
 */
export type PressTopicDto =
  | "playerMilestone"
  | "contractTransfer"
  | "matchResult"
  | "relational";

/**
 * One press-inbox item returned as part of `PressInboxDto`.
 *
 * `eventId` is the raw u32 EventId from the memory ledger.
 * `season` is the u16 season index in which the event occurred.
 * `eventClass` is the raw u32 event class discriminant (informational only;
 *   the UI uses `topic` for display routing).
 * `headline` is a rendered football-native prose string.
 * `managerQuote` is an optional short quote; null when the sim did not
 *   generate a quote for this event.
 */
export interface PressItemDto {
  eventId: number;
  season: number;
  eventClass: number;
  topic: PressTopicDto;
  headline: string;
  managerQuote: string | null;
}

/**
 * Returned by `get_press_inbox`.
 *
 * `seasonNumber` is the current season (mirrors CareerOverview.seasonNumber).
 * `items` is ordered by projected salience descending (event_id ascending
 * tiebreak), capped at 20. May be empty at career start.
 */
export interface PressInboxDto {
  seasonNumber: number;
  items: PressItemDto[];
}

// ---------------------------------------------------------------------------
// T4-F4 scouting DTOs — mirrors fw-tauri::roster_dto (camelCase serde)
// ---------------------------------------------------------------------------

/**
 * Per-category estimate from a scout observation.
 *
 * `category` is one of `"Physical"`, `"Mental"`, `"Technical"`.
 * `band` is a football-native uncertainty label (e.g. `"a confident read"`).
 * `low` / `high` are f64 in [0, 1] — NOT shown as raw numbers in the UI.
 */
export interface CategoryEstimateDto {
  /** Closed set, mirroring Rust `fw_scouting::GeneCategory` (always these 3). */
  category: "Physical" | "Mental" | "Technical";
  low: number;
  high: number;
  band: string;
}

/**
 * Per-label estimate from a scout observation.
 *
 * `label` is the human-readable phenotype label (e.g. `"Pure finisher"`).
 * `confidence` is f64 in [0, 1] — NOT shown as raw number in the UI.
 * `band` is a football-native uncertainty label.
 */
export interface LabelEstimateDto {
  label: string;
  confidence: number;
  band: string;
}

/**
 * Scouting report DTO returned by `get_scout_report`.
 *
 * `playerId` is the raw u32 roster PlayerId (distinct from the content-pack
 * string id used by PlayerDetail). `confidence` and category `low`/`high`
 * values are f64 in [0, 1] — surface as band text only, never raw numbers.
 * `categories` always has 3 entries (Physical, Mental, Technical in order).
 */
export interface ScoutReportDto {
  playerId: number;
  confidence: number;
  overallBand: string;
  observationCount: number;
  categories: CategoryEstimateDto[];
  labels: LabelEstimateDto[];
}

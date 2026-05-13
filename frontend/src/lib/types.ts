/*
 * Wire types — MUST stay in sync with `src-tauri/src/commands.rs`.
 *
 * Naming convention: TypeScript uses camelCase; Rust uses snake_case but the
 * commands.rs structs carry `#[serde(rename_all = "camelCase")]` so the
 * boundary is camelCase on both sides. When fw-tauri begins exporting real
 * serde structs (T1-5+), these types should be regenerated from the Rust
 * surface (candidate tool: `specta` or `ts-rs` — decision deferred to T2-5).
 */

export interface DummyState {
  appVersion: string;
  message: string;
  backendReady: boolean;
}

export type MatchEventKind =
  | "Goal"
  | "Shot"
  | "Pass"
  | "KickOff"
  | "HalfTime"
  | "FullTime"
  | "Card"
  | "Substitution";

export interface MatchEvent {
  tick: number;
  minute: number;
  kind: MatchEventKind | string; // open enum at scaffold time
  description: string;
}

export interface MatchResult {
  matchId: string;
  homeId: string;
  awayId: string;
  homeScore: number;
  awayScore: number;
  /** 32-byte hash, `0x`-prefixed hex string. Used for pinned-corpus regression. */
  canonicalHash: string;
  events: MatchEvent[];
}

export interface LeagueStanding {
  position: number;
  clubId: string;
  clubName: string;
  played: number;
  won: number;
  drawn: number;
  lost: number;
  goalsFor: number;
  goalsAgainst: number;
  goalDifference: number;
  points: number;
}

export interface PlayerSummary {
  playerId: string;
  name: string;
  age: number;
  role: string;
  /**
   * Football-native labels (e.g. "late bloomer", "fragile", "early-crosser").
   * NEVER raw gene numbers. See DESIGN_DOC.md §2 rule 7 + ui-vocabulary.md.
   */
  phenotypeLabels: string[];
  contractEnd: string; // ISO date
}

export interface Fixture {
  fixtureId: string;
  date: string; // ISO date
  homeId: string;
  awayId: string;
  competition: string;
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
}

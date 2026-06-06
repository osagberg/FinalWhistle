/*
 * LiveMatch route — S3b / M2b / S10 / S12.
 *
 * "Nail being able to watch a good match." — owner brief
 *
 * The primary surface for the paced live-match experience. The player watches
 * key moments unfold in real time on the 2D tactical board, with a persistent
 * scoreline, match clock, and a rolling key-moment strip. High-frequency
 * pass events are hidden; the auto-sim-to-event mode pauses at every
 * goal/shot/card/offside so the player can absorb what happened before the
 * next incident.
 *
 * Architecture:
 *   - On mount: three paths, in priority order:
 *       1. FIXTURE path (M2b): location.state has { homeClubId, awayClubId }
 *          (numbers) → calls startLiveMatchForFixture({ homeClubId, awayClubId }).
 *          This produces the same deterministic MatchState as the AI-sim path in
 *          advance_week, so the watched result == the AI-sim result. Supplied by
 *          Home's "Watch this match" button.
 *       2. CAREER-NEXT path: no fixture in state but a career is active (the
 *          sidebar "Match" link lands here) → fetch the managed club's fixtures,
 *          derive the next unplayed (home, away) pair via the same helper Home
 *          uses, and call startLiveMatchForFixture for it. The sidebar always
 *          opens the next real fixture, not a throwaway demo.
 *       3. SEED/DEV path (fallback): no fixture in state AND no active career /
 *          no unplayed fixture → calls startLiveMatch(seedHex) with state.seedHex
 *          or a random hex. This is the dev direct-entry demo.
 *   - Step loop (setInterval): calls stepLiveMatch(handle, ticksPerCall)
 *     at the configured pace. Each step result appends to the filtered event
 *     feed and pushes result.frame onto the growing frames array.
 *   - TacticalBoard consumes the growing frames array with followLatest=true,
 *     which snaps the cursor to the latest frame without requiring Play.
 *   - Speed modes:
 *       "auto"  — 3 ticks/call; pauses automatically on a key moment (the
 *                 owner's default: "jump moment-to-moment").
 *       "x1"    — 1 tick/call; continuous.
 *       "x3"    — 3 ticks/call; continuous.
 *       "fast"  — 60 ticks/call; continuous, ~1 sim-minute/wall-second.
 *       "skip"  — 300 ticks/call; as fast as the backend permits; no pause.
 *   - Finish: loop stops on isFinished; finishLiveMatch called; final result shown.
 *
 * S10 — Touchline panel:
 *   A compact panel visible during play (not after finish) offering 3 manager
 *   instructions. Each fires applyMatchCommand(handle, command). The backend
 *   currently returns IpcError { kind: "liveMatchCommandUnimplemented" } for
 *   all commands; that specific error is treated as BENIGN — shown as a calm
 *   inline ack ("Instruction noted — takes effect in a future build"). Any
 *   other error propagates to describeRouteError. Issuing an instruction does
 *   NOT pause or interrupt the step loop.
 *
 * S12 — Half-time pause (UI-only):
 *   When the match clock first crosses 45' (tick >= 2700), the step loop
 *   pauses automatically ONCE and shows a half-time interstitial: current
 *   scoreline + key-moment recap + "Resume second half" button. Gated by a
 *   one-shot flag (halfTimeFired) so a later tick batch cannot re-trigger it,
 *   and so it does not fire if the user skipped past 45'. Does not double-
 *   pause with the auto-sim-to-event logic. The sim has no half-time event;
 *   the frontend detects the tick threshold.
 *
 * Result model (v1 — read-only deterministic preview):
 *   Watching is a READ-ONLY deterministic preview of the fixture. advance_week
 *   remains the AUTHORITATIVE play of the round — it commits the result to the
 *   season state. Because startLiveMatchForFixture uses the same seed + lineups
 *   as advance_week's AI-sim path, the watched result == the committed result
 *   when no in-match commands are issued.
 *
 *   Watching does NOT mutate season standings or advance the match day.
 *
 *   M3 hook: once M3 makes watching the authoritative play of the user's own
 *   fixture, advance_week should skip that fixture (already resolved by the
 *   user's watch). No rewrite of this route needed — the IPC surface is the
 *   same; only the season-controller behaviour changes on the Rust side.
 *
 * IPC (read-only; never mutates canonical state — CLAUDE.md §7):
 *   startLiveMatch, startLiveMatchForFixture, stepLiveMatch, finishLiveMatch,
 *   applyMatchCommand from ~/lib/api/live_match.ts
 *
 * Rules compliance:
 *   - No `any` (Frontend/RULES.md §6)
 *   - Solid signals / stores not React hooks (Frontend/RULES.md §1)
 *   - setInterval cleanup in onCleanup (Frontend/RULES.md §1 — side-effect cleanup)
 *   - PixiJS board created once, follows latest frame (Frontend/RULES.md §4)
 *   - ARIA: speed buttons are aria-pressed; event feed is aria-live; board is aria-label
 *   - Touchline buttons keyboard-reachable with aria-labels; interstitial has role="dialog"
 *   - Football-native copy; no banned mystical vocabulary (Content/RULES.md §5)
 *   - No visible stat numbers (CLAUDE.md §7 "invisible floats")
 */

import {
  createMemo,
  createSignal,
  For,
  lazy,
  onCleanup,
  onMount,
  Show,
  Suspense,
  type JSX,
} from "solid-js";
import { useLocation, useNavigate } from "@solidjs/router";
import ErrorBoundary from "~/components/ErrorBoundary";
import { describeRouteError, isIpcError } from "~/lib/route-errors";
import { backendAvailable } from "~/lib/tauri";
import {
  startLiveMatch,
  startLiveMatchForFixture,
  stepLiveMatch,
  finishLiveMatch,
  applyMatchCommand,
} from "~/lib/api/live_match";
import { getFixtures } from "~/lib/api/season";
import { deriveNextFixtureClubIds } from "~/lib/fixtures";
import { isCareerActive, selectedClubId } from "~/lib/state";
import { isKeyMomentKind } from "~/lib/match-events";
import type {
  FinalMatchResult,
  MatchCommand,
  MatchEvent,
  MatchEventKind,
  MatchFrameDTO,
  MatchHandle,
} from "~/lib/types";

// TacticalBoard is heavy (PixiJS). Lazy-load so the bundle cost is only paid
// when the live-match route is entered.
const TacticalBoard = lazy(() => import("~/components/TacticalBoard"));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Generate a random u64 hex string for dev entry (no seed from router). */
function randomSeedHex(): string {
  const hi = Math.floor(Math.random() * 0xffffffff);
  const lo = Math.floor(Math.random() * 0xffffffff);
  return (
    "0x" +
    hi.toString(16).padStart(8, "0") +
    lo.toString(16).padStart(8, "0")
  );
}

/** Convert a sim tick count to a football match minute (60 ticks = 1 minute). */
function tickToMinute(tick: number): number {
  return Math.floor(tick / 60);
}

/** Format tick as a match-clock string: "45'" or "90+2'". */
function formatClock(tick: number): string {
  const min = tickToMinute(tick);
  if (min <= 45) return `${min}'`;
  if (min <= 90) return `${min}'`;
  const extra = min - 90;
  return `90+${extra}'`;
}

// ---------------------------------------------------------------------------
// Speed modes
// ---------------------------------------------------------------------------

/**
 * Speed mode discriminant.
 *
 * "auto"  — 3 ticks/call; auto-pauses on key moments (default).
 * "x1"    — 1 tick/call; continuous.
 * "x3"    — 3 ticks/call; continuous.
 * "fast"  — 60 ticks/call; ~1 min/s; continuous.
 * "skip"  — 300 ticks/call; skip to end.
 */
export type SpeedMode = "auto" | "x1" | "x3" | "fast" | "skip";

function ticksPerCall(mode: SpeedMode): number {
  switch (mode) {
    case "auto":
      return 3;
    case "x1":
      return 1;
    case "x3":
      return 3;
    case "fast":
      return 60;
    case "skip":
      return 300;
  }
}

/** Label shown on the speed button for each mode. */
function speedLabel(mode: SpeedMode): string {
  switch (mode) {
    case "auto":
      return "Auto";
    case "x1":
      return "×1";
    case "x3":
      return "×3";
    case "fast":
      return "Fast";
    case "skip":
      return "Skip";
  }
}

/** ARIA label for the speed button for each mode. */
function speedAriaLabel(mode: SpeedMode): string {
  switch (mode) {
    case "auto":
      return "Auto mode — pauses at key moments";
    case "x1":
      return "Speed × 1 — continuous play";
    case "x3":
      return "Speed × 3 — continuous play";
    case "fast":
      return "Fast — roughly one minute per second";
    case "skip":
      return "Skip to end";
  }
}

const SPEED_MODES: SpeedMode[] = ["auto", "x1", "x3", "fast", "skip"];

// Step interval in milliseconds. At x1 (1 tick/call) this gives ~16.7ms
// between steps, i.e. the loop runs at roughly 60fps wall-clock. At higher
// ticks-per-call the interval stays the same; more sim time passes per wall
// tick. The board's Pixi ticker drives its own 60fps interpolation pass;
// these IPC calls simply supply new frames for it to follow.
const STEP_INTERVAL_MS = 100;

// ---------------------------------------------------------------------------
// Half-time threshold (S12)
// ---------------------------------------------------------------------------

// Tick threshold for half-time detection. 45 minutes × 60 ticks/minute.
const HALF_TIME_TICK = 2700;

// ---------------------------------------------------------------------------
// Touchline instructions (S10)
// ---------------------------------------------------------------------------

/**
 * One entry in the Touchline panel.
 *
 * `label` is the football-native button copy.
 * `ariaLabel` is the screen-reader label.
 * `command` is the MatchCommand payload to enqueue.
 */
interface TouchlineInstruction {
  label: string;
  ariaLabel: string;
  command: MatchCommand;
}

/**
 * The 3 touchline instructions surfaced to the manager.
 *
 * Variants chosen as the most intuitive press-level + tempo controls from the
 * MatchCommand union. `changePressLevel` and `changeTempoBias` have no player-
 * ID dependencies, making them safe to expose without a squad-slot picker.
 */
const TOUCHLINE_INSTRUCTIONS: TouchlineInstruction[] = [
  {
    label: "Press high",
    ariaLabel: "Instruct the team to press high up the pitch",
    command: { kind: "changePressLevel", level: "high" },
  },
  {
    label: "Sit deep",
    ariaLabel: "Instruct the team to drop deep and hold shape",
    command: { kind: "changePressLevel", level: "low" },
  },
  {
    label: "Play quicker",
    ariaLabel: "Instruct the team to play at a faster tempo",
    command: { kind: "changeTempoBias", bias: "fast" },
  },
];

// ---------------------------------------------------------------------------
// Event badge helpers (shared visual language with Match.tsx)
// ---------------------------------------------------------------------------

function isGoalKind(kind: MatchEventKind): boolean {
  return kind === "Goal";
}

function eventLabel(kind: MatchEventKind): string {
  switch (kind) {
    case "Goal":
      return "GOAL";
    case "Shot":
      return "Shot";
    case "Pass":
      return "Pass";
    case "KickOff":
      return "Kick-off";
    case "HalfTime":
      return "Half-time";
    case "FullTime":
      return "Full-time";
    case "Card":
      return "Card";
    case "Substitution":
      return "Sub";
    case "SignatureFirstFired":
      return "Signature";
    case "Offside":
      return "Offside";
    case "PassIncomplete":
      return "Lost";
    default: {
      const _exhaustive: never = kind;
      throw new Error(
        `LiveMatch eventLabel: unhandled MatchEventKind. kind=${JSON.stringify(_exhaustive)}`,
      );
    }
  }
}

function badgeClass(kind: MatchEventKind): string {
  switch (kind) {
    case "Goal":
      return "bg-pitch-500 text-white font-bold";
    case "KickOff":
    case "HalfTime":
    case "FullTime":
      return "bg-ink-subtle text-paper dark:bg-midnight-subtle dark:text-paper";
    case "Card":
      return "bg-flag-yellow text-ink";
    case "Shot":
    case "Pass":
    case "Substitution":
    case "SignatureFirstFired":
    case "Offside":
    case "PassIncomplete":
      return "bg-paper-bold text-ink-subtle dark:bg-midnight-subtle dark:text-paper-subtle";
    default: {
      const _exhaustive: never = kind;
      throw new Error(
        `LiveMatch badgeClass: unhandled MatchEventKind. kind=${JSON.stringify(_exhaustive)}`,
      );
    }
  }
}

// ---------------------------------------------------------------------------
// Mock fallback — browser-preview mode (no Tauri)
// ---------------------------------------------------------------------------

/** Mock handle returned by startLiveMatch in browser-preview mode. */
function makeMockHandle(seedHex: string): MatchHandle {
  return { id: 0, seedHex };
}

/**
 * Simulate a single step in browser-preview mode.
 *
 * Each call advances the mock tick by `ticks` and occasionally emits a key
 * event. The match ends at tick 5400 (90 minutes).
 */
function makeMockStep(
  handle: MatchHandle,
  currentTick: number,
  ticks: number,
): {
  handle: MatchHandle;
  newEvents: MatchEvent[];
  score: { home: number; away: number };
  tick: number;
  isFinished: boolean;
  frame: MatchFrameDTO;
} {
  const nextTick = Math.min(currentTick + ticks, 5400);
  const isFinished = nextTick >= 5400;
  const newEvents: MatchEvent[] = [];

  // Emit KickOff at tick 0.
  if (currentTick === 0 && nextTick >= 0) {
    newEvents.push({ tick: 0, minute: 0, kind: "KickOff", description: "Kick-off." });
  }
  // Emit HalfTime around tick 2700.
  if (currentTick < 2700 && nextTick >= 2700) {
    newEvents.push({ tick: 2700, minute: 45, kind: "HalfTime", description: "Half-time." });
  }
  // Emit a Goal at tick 1080 (18').
  if (currentTick < 1080 && nextTick >= 1080) {
    newEvents.push({ tick: 1080, minute: 18, kind: "Goal", description: "Clean finish from close range." });
  }
  // Emit a Shot at tick 3600 (60').
  if (currentTick < 3600 && nextTick >= 3600) {
    newEvents.push({ tick: 3600, minute: 60, kind: "Shot", description: "Stinging drive, parried away." });
  }
  // Emit FullTime at 5400.
  if (isFinished) {
    newEvents.push({ tick: 5400, minute: 90, kind: "FullTime", description: "Full time." });
  }

  const frame: MatchFrameDTO = {
    seedHex: handle.seedHex,
    tick: nextTick,
    homeScore: nextTick >= 1080 ? 1 : 0,
    awayScore: 0,
    players: Array.from({ length: 22 }, (_, slot) => ({
      slot,
      posX: (slot % 5) * 10 - 20 + (nextTick / 5400) * 5,
      posY: (slot % 4) * 8 - 12,
      velX: 0,
      velY: 0,
    })),
    ball: {
      posX: (nextTick / 5400) * 40 - 20,
      posY: Math.sin(nextTick / 100) * 15,
      posZ: 0,
      velX: 0,
      velY: 0,
      velZ: 0,
    },
    possession: null,
  };

  return {
    handle,
    newEvents,
    score: { home: nextTick >= 1080 ? 1 : 0, away: 0 },
    tick: nextTick,
    isFinished,
    frame,
  };
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

interface LiveScorelineProps {
  home: number;
  away: number;
  clock: string;
  seedHex: string;
}

function LiveScoreline(props: LiveScorelineProps): JSX.Element {
  return (
    <div
      class="fw-panel p-3 flex items-center justify-center gap-4"
      aria-label="Live score"
      role="region"
    >
      <span class="font-mono text-xs text-ink-mute dark:text-paper-subtle tabular-nums w-12 text-right">
        {props.clock}
      </span>
      <span class="font-display text-3xl tabular-nums text-pitch-600 dark:text-pitch-300">
        {props.home}
      </span>
      <span class="text-ink-mute dark:text-paper-subtle font-mono text-lg select-none">
        –
      </span>
      <span class="font-display text-3xl tabular-nums text-pitch-600 dark:text-pitch-300">
        {props.away}
      </span>
      <span class="font-mono text-xs text-ink-mute dark:text-paper-subtle tabular-nums w-12">
        {/* right spacer to balance clock */}
      </span>
    </div>
  );
}

interface KeyMomentItemProps {
  event: MatchEvent;
}

function KeyMomentItem(props: KeyMomentItemProps): JSX.Element {
  return (
    <li
      class={`flex items-start gap-2 py-1 px-2 rounded text-sm ${isGoalKind(props.event.kind) ? "bg-pitch-50 dark:bg-pitch-950/40" : ""}`}
    >
      <span
        class="font-mono text-xs text-ink-mute dark:text-paper-subtle w-8 pt-0.5 shrink-0 text-right"
        aria-label={`Minute ${props.event.minute}`}
      >
        {props.event.minute}&apos;
      </span>
      <span
        class={`text-xs font-mono px-1 py-0.5 rounded shrink-0 ${badgeClass(props.event.kind)}`}
      >
        {eventLabel(props.event.kind)}
      </span>
      <Show when={props.event.description}>
        <span class="text-xs text-ink-subtle dark:text-paper-subtle">
          {props.event.description}
        </span>
      </Show>
    </li>
  );
}

// ---------------------------------------------------------------------------
// HalfTimeInterstitial (S12)
// ---------------------------------------------------------------------------

interface HalfTimeInterstitialProps {
  homeScore: number;
  awayScore: number;
  keyEvents: MatchEvent[];
  onResume: () => void;
}

/**
 * Half-time interstitial panel.
 *
 * Shows the first-half scoreline and a brief recap of key moments so far.
 * The "Resume second half" button restarts the step loop.
 *
 * Announced to screen readers via role="dialog" + aria-live="polite" on the
 * status line so assistive technology surfaces the pause automatically.
 */
function HalfTimeInterstitial(props: HalfTimeInterstitialProps): JSX.Element {
  return (
    <div
      class="fw-panel p-4 space-y-3 border border-pitch-300 dark:border-pitch-700 bg-pitch-50 dark:bg-pitch-950/40"
      role="dialog"
      aria-label="Half-time"
      aria-modal="false"
    >
      <h2
        class="font-display text-base text-pitch-700 dark:text-pitch-300"
        id="half-time-heading"
      >
        Half-time
      </h2>

      {/* Scoreline */}
      <p
        class="font-mono text-2xl tabular-nums text-pitch-600 dark:text-pitch-300"
        aria-label={`Half-time score: ${props.homeScore} – ${props.awayScore}`}
      >
        {props.homeScore}
        <span class="px-2 text-ink-mute dark:text-paper-subtle">–</span>
        {props.awayScore}
      </p>

      {/* First-half recap */}
      <Show
        when={props.keyEvents.length > 0}
        fallback={
          <p class="text-xs text-ink-mute dark:text-paper-subtle italic">
            Quiet first half — nothing to report.
          </p>
        }
      >
        <ul
          class="space-y-0.5"
          aria-label="First half key moments"
        >
          <For each={props.keyEvents}>
            {(ev) => (
              <li class="text-xs text-ink-subtle dark:text-paper-subtle flex gap-2 items-baseline">
                <span class="font-mono w-8 text-right shrink-0 text-ink-mute dark:text-paper-subtle">
                  {ev.minute}&apos;
                </span>
                <span
                  class={`font-mono px-1 py-0.5 rounded text-xs shrink-0 ${badgeClass(ev.kind)}`}
                >
                  {eventLabel(ev.kind)}
                </span>
                <Show when={ev.description}>
                  <span>{ev.description}</span>
                </Show>
              </li>
            )}
          </For>
        </ul>
      </Show>

      {/* Resume button */}
      <button
        type="button"
        class="mt-1 px-4 py-1.5 rounded text-sm font-mono bg-pitch-500 text-white hover:bg-pitch-600 focus:outline-none focus:ring-2 focus:ring-pitch-400"
        onClick={() => props.onResume()}
        aria-label="Resume second half"
      >
        Resume second half
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// TouchlinePanel (S10)
// ---------------------------------------------------------------------------

/**
 * Acknowledgement state for a touchline instruction.
 *
 * `null`        — no instruction issued yet (or ack has cleared).
 * `"pending"`   — IPC call in flight.
 * `"noted"`     — backend returned LiveMatchCommandUnimplemented (benign).
 * `"error"`     — unexpected IPC error (surfaced as descriptive text, not a
 *                 red alert — the step loop continues regardless).
 */
type TouchlineAckState =
  | null
  | { status: "pending"; label: string }
  | { status: "noted"; label: string }
  | { status: "error"; label: string; detail: string };

interface TouchlinePanelProps {
  handle: MatchHandle;
  disabled: boolean;
}

/**
 * Compact touchline instructions panel (S10).
 *
 * Surfaced during play; hidden after full-time. Each button fires
 * applyMatchCommand with the corresponding MatchCommand. The step loop is
 * not paused or interrupted by issuing an instruction.
 *
 * Error handling:
 *   - IpcError { kind: "liveMatchCommandUnimplemented" } → benign ack:
 *     "Instruction noted — takes effect in a future build."
 *   - Any other error → football-native detail string (no red alert; the
 *     match continues; the manager can try again).
 */
function TouchlinePanel(props: TouchlinePanelProps): JSX.Element {
  const [ack, setAck] = createSignal<TouchlineAckState>(null);

  async function handleInstruction(instr: TouchlineInstruction): Promise<void> {
    setAck({ status: "pending", label: instr.label });
    try {
      await applyMatchCommand(props.handle, instr.command);
      // Success path: command accepted (will be implemented in S11).
      setAck({ status: "noted", label: instr.label });
    } catch (e: unknown) {
      if (isIpcError(e) && e.kind === "liveMatchCommandUnimplemented") {
        // Expected for all commands in the current build — treat as benign.
        setAck({ status: "noted", label: instr.label });
      } else {
        const copy = describeRouteError(e, { what: "the instruction" });
        setAck({ status: "error", label: instr.label, detail: copy.detail });
      }
    }
  }

  return (
    <section
      aria-label="Touchline instructions"
      class="fw-panel px-3 py-2 space-y-2"
    >
      <h3 class="text-xs font-mono text-ink-mute dark:text-paper-subtle uppercase tracking-wide">
        Touchline
      </h3>

      <div class="flex flex-wrap gap-2" role="group" aria-label="Manager instructions">
        <For each={TOUCHLINE_INSTRUCTIONS}>
          {(instr) => {
            const isThisPending = () => {
              const a = ack();
              return a !== null && a.status === "pending" && a.label === instr.label;
            };
            return (
              <button
                type="button"
                class="px-3 py-1 text-xs font-mono rounded border focus:outline-none focus:ring-2 focus:ring-pitch-400 bg-paper dark:bg-midnight-panel text-ink-subtle dark:text-paper-subtle border-paper-bold dark:border-midnight-line hover:border-pitch-400 disabled:opacity-40 disabled:cursor-not-allowed"
                onClick={() => void handleInstruction(instr)}
                disabled={props.disabled || isThisPending()}
                aria-label={instr.ariaLabel}
                aria-busy={isThisPending()}
              >
                {instr.label}
              </button>
            );
          }}
        </For>
      </div>

      {/* Inline acknowledgement — never a red alert */}
      <Show when={ack()}>
        {(a) => (
          <p
            class={`text-xs font-mono ${
              a().status === "error"
                ? "text-flag-amber dark:text-flag-amber"
                : "text-ink-mute dark:text-paper-subtle"
            }`}
            aria-live="polite"
            aria-atomic="true"
          >
            {a().status === "noted"
              ? `${a().label} — instruction noted, takes effect in a future build.`
              : a().status === "pending"
                ? `${a().label}…`
                : `Could not send ${a().label.toLowerCase()}: ${(a() as { status: "error"; detail: string }).detail}`}
          </p>
        )}
      </Show>
    </section>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

/**
 * Location state shapes accepted by this route.
 *
 * FIXTURE path (M2b): { homeClubId: number; awayClubId: number } — navigating
 *   from the Home hub's "Watch this match" button. Calls
 *   startLiveMatchForFixture().
 *
 * CAREER-NEXT / SEED-DEV path: { seedHex: string } or no state — the sidebar
 *   "Match" link, dev direct-entry, or the old Match route. When a career is
 *   active we resolve the managed club's next fixture; otherwise we fall back to
 *   startLiveMatch(seedHex).
 */
type LiveMatchState =
  | { homeClubId: number; awayClubId: number }
  | { seedHex?: string }
  | null;

function isFixtureState(
  state: LiveMatchState,
): state is { homeClubId: number; awayClubId: number } {
  return (
    state !== null &&
    typeof state === "object" &&
    "homeClubId" in state &&
    "awayClubId" in state &&
    typeof (state as { homeClubId: unknown }).homeClubId === "number" &&
    typeof (state as { awayClubId: unknown }).awayClubId === "number"
  );
}

export default function LiveMatch(): JSX.Element {
  const location = useLocation<LiveMatchState>();
  const navigate = useNavigate();

  const locationState = location.state as LiveMatchState;

  // Seed fallback for the dev path (used only when locationState is not a fixture).
  const initialSeed =
    !isFixtureState(locationState) && locationState !== null
      ? ((locationState as { seedHex?: string }).seedHex ?? randomSeedHex())
      : randomSeedHex();

  // ---------------------------------------------------------------------------
  // Core state
  // ---------------------------------------------------------------------------

  const [handle, setHandle] = createSignal<MatchHandle | null>(null);
  const [liveFrames, setLiveFrames] = createSignal<MatchFrameDTO[]>([]);
  const [keyEvents, setKeyEvents] = createSignal<MatchEvent[]>([]);
  const [homeScore, setHomeScore] = createSignal(0);
  const [awayScore, setAwayScore] = createSignal(0);
  const [currentTick, setCurrentTick] = createSignal(0);
  // currentTick is also used to drive browser-preview mock steps.
  const [isRunning, setIsRunning] = createSignal(false);
  const [isFinished, setIsFinished] = createSignal(false);
  const [finalResult, setFinalResult] = createSignal<FinalMatchResult | null>(null);
  const [errorMsg, setErrorMsg] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(true);

  // Speed mode — default is "auto" (pause on key moments).
  const [speedMode, setSpeedMode] = createSignal<SpeedMode>("auto");
  // Paused flag — separate from speed mode so the user can pause/resume.
  const [paused, setPaused] = createSignal(false);
  // Auto-mode paused-at-event flag — set when auto mode pauses on a key moment.
  const [autoEventPaused, setAutoEventPaused] = createSignal(false);

  // S12 — Half-time pause state.
  // halfTimePaused: true while the half-time interstitial is showing.
  const [halfTimePaused, setHalfTimePaused] = createSignal(false);
  // One-shot guard: once fired, the half-time pause never fires again.
  // Stored as a plain mutable variable (not a signal) so it cannot be
  // accidentally read inside a reactive scope and create spurious dependencies.
  let halfTimeFired = false;

  // Interval handle — cleared in onCleanup.
  let intervalId: ReturnType<typeof setInterval> | undefined;
  // Re-entrancy guard for doStep (the interval can fire before the prior
  // async step resolves on a slow IPC round-trip).
  let stepInFlight = false;

  // Track mock tick in browser-preview mode.
  let mockTick = 0;

  // ---------------------------------------------------------------------------
  // Step logic
  // ---------------------------------------------------------------------------

  async function doStep(): Promise<void> {
    const h = handle();
    // Re-entrancy guard: the interval fires every STEP_INTERVAL_MS regardless
    // of whether the previous step's async IPC has resolved. On a slow round-
    // trip two doStep calls would otherwise race — double frame accumulation,
    // double finish/clearInterval. Allow only one step in flight at a time.
    if (!h || isFinished() || stepInFlight) return;
    stepInFlight = true;

    const mode = speedMode();
    const ticks = ticksPerCall(mode);

    try {
      let result: Awaited<ReturnType<typeof stepLiveMatch>>;

      if (!backendAvailable()) {
        // Browser-preview mock path.
        const step = makeMockStep(h, mockTick, ticks);
        mockTick = step.tick;
        result = step;
      } else {
        result = await stepLiveMatch(h, ticks);
      }

      // Accumulate the frame.
      setLiveFrames((prev) => [...prev, result.frame]);

      // Update scoreline + clock.
      setHomeScore(result.score.home);
      setAwayScore(result.score.away);
      setCurrentTick(result.tick);

      // S12 — Half-time one-shot pause.
      // Fire only once, only when the loop is actively running, AND only
      // when the current speed mode is not "skip" (the user intentionally
      // wants to skip past half-time). In "skip" mode we set the fired flag
      // so it does not re-trigger on a later slower-mode batch.
      if (!halfTimeFired && result.tick >= HALF_TIME_TICK && !result.isFinished) {
        halfTimeFired = true;
        if (mode !== "skip") {
          clearInterval(intervalId);
          intervalId = undefined;
          setIsRunning(false);
          setHalfTimePaused(true);
          // Return here: do not apply auto-event-pause logic over the top of
          // the half-time pause. The event accumulation above is already done.
          // The finally block will clear stepInFlight.
          return;
        }
        // "skip" mode: mark fired but do not pause — fall through.
      }

      // Filter key events from this step's delta.
      const newKeyEvents = result.newEvents.filter((ev) =>
        isKeyMomentKind(ev.kind),
      );
      if (newKeyEvents.length > 0) {
        setKeyEvents((prev) => [...prev, ...newKeyEvents]);
      }

      if (result.isFinished) {
        clearInterval(intervalId);
        intervalId = undefined;
        setIsRunning(false);
        setIsFinished(true);

        // Call finishLiveMatch to clean up the session.
        try {
          if (!backendAvailable()) {
            // Mock final result.
            setFinalResult({
              handle: h,
              finalScore: result.score,
              tick: result.tick,
              totalEvents: keyEvents().length,
            });
          } else {
            const final = await finishLiveMatch(h);
            setFinalResult(final);
          }
        } catch (e: unknown) {
          // finishLiveMatch failure is non-fatal — the match has ended; we just
          // can't get the server-side final result. Log and continue.
          console.error("[LiveMatch] finishLiveMatch failed:", e);
        }
        return;
      }

      // Auto mode: pause on a meaningful moment so the manager jumps beat to
      // beat — but NOT on kick-off (the match would stall at the opening
      // whistle before any play). Half-time / goals / shots / cards still pause.
      const pauseworthy = newKeyEvents.filter((ev) => ev.kind !== "KickOff");
      if (mode === "auto" && pauseworthy.length > 0) {
        setPaused(true);
        setAutoEventPaused(true);
        clearInterval(intervalId);
        intervalId = undefined;
        setIsRunning(false);
      }
    } catch (e: unknown) {
      clearInterval(intervalId);
      intervalId = undefined;
      setIsRunning(false);
      const copy = describeRouteError(e, { what: "the match" });
      setErrorMsg(`${copy.headline}: ${copy.detail}`);
    } finally {
      stepInFlight = false;
    }
  }

  function startLoop(): void {
    if (intervalId !== undefined) return;
    if (isFinished()) return;
    setIsRunning(true);
    setPaused(false);
    setAutoEventPaused(false);
    setHalfTimePaused(false);
    intervalId = setInterval(() => {
      void doStep();
    }, STEP_INTERVAL_MS);
  }

  function stopLoop(): void {
    if (intervalId !== undefined) {
      clearInterval(intervalId);
      intervalId = undefined;
    }
    setIsRunning(false);
    setPaused(true);
    setAutoEventPaused(false);
  }

  // S12: resume from half-time interstitial — dismisses the panel and
  // restarts the step loop exactly like a normal Resume.
  function handleHalfTimeResume(): void {
    startLoop();
  }

  // ---------------------------------------------------------------------------
  // Mount: start the session + kick off the loop
  // ---------------------------------------------------------------------------

  /**
   * Resolve the managed club's next unplayed fixture as a (home, away) pair.
   *
   * Used by the CAREER-NEXT entry path (sidebar "Match" link with no router
   * state). Reuses the same `getFixtures` + `deriveNextFixtureClubIds` logic the
   * Home hub uses, so the sidebar opens exactly the fixture Home would watch.
   * Returns null when there is no active career, no managed club, or no unplayed
   * fixture remaining — the caller then falls back to the SEED/DEV demo.
   */
  async function resolveCareerNextFixture(): Promise<{
    homeClubId: number;
    awayClubId: number;
  } | null> {
    if (!isCareerActive()) return null;
    const id = selectedClubId();
    if (id === null) return null;
    const numericId = parseInt(id, 10);
    if (isNaN(numericId)) return null;
    const fixtures = await getFixtures(numericId);
    return deriveNextFixtureClubIds(fixtures, id);
  }

  onMount(() => {
    void (async () => {
      try {
        let h: MatchHandle;
        if (!backendAvailable()) {
          // Browser-preview mock path — fixture context is noted but mock uses
          // the same handle shape regardless of entry path.
          h = makeMockHandle(initialSeed);
        } else if (isFixtureState(locationState)) {
          // FIXTURE path (M2b): start from the real fixture so the watched
          // result is deterministically equal to the AI-sim result.
          h = await startLiveMatchForFixture({
            homeClubId: locationState.homeClubId,
            awayClubId: locationState.awayClubId,
          });
        } else {
          // CAREER-NEXT path: the sidebar "Match" link lands here with no
          // router state. When a career is active, play the managed club's next
          // unplayed fixture; otherwise drop to the SEED/DEV demo below.
          const careerFixture = await resolveCareerNextFixture();
          if (careerFixture) {
            h = await startLiveMatchForFixture(careerFixture);
          } else {
            // SEED/DEV path (fallback): start from a seed — dev direct-entry.
            h = await startLiveMatch(initialSeed);
          }
        }
        setHandle(h);
        setLoading(false);
        // Begin in auto mode immediately.
        startLoop();
      } catch (e: unknown) {
        const copy = describeRouteError(e, { what: "the match" });
        setErrorMsg(`${copy.headline}: ${copy.detail}`);
        setLoading(false);
      }
    })();
  });

  onCleanup(() => {
    if (intervalId !== undefined) {
      clearInterval(intervalId);
      intervalId = undefined;
    }
  });

  // Speed mode changes are handled imperatively in handleSpeedChange below.
  // We intentionally avoid a createEffect here because reading isRunning()
  // inside an effect that also writes to intervalId would create a reactive
  // dependency that triggers on every isRunning() change — including the ones
  // caused by startLoop() itself — producing an infinite interval-restart loop.

  // ---------------------------------------------------------------------------
  // Derived
  // ---------------------------------------------------------------------------

  const clock = createMemo(() => formatClock(currentTick()));
  const matchMinute = createMemo(() => tickToMinute(currentTick()));

  const statusLabel = createMemo(() => {
    if (loading()) return "Getting teams on the pitch…";
    if (isFinished()) return "Full time";
    if (halfTimePaused()) return "Half-time";
    if (autoEventPaused()) {
      const last = keyEvents().at(-1);
      if (last) return `Paused — ${eventLabel(last.kind)} in the ${matchMinute()}'`;
      return "Paused at a key moment";
    }
    if (paused()) return "Paused";
    const mode = speedMode();
    if (mode === "auto") return "Following the match";
    return `Running at ${speedLabel(mode)}`;
  });

  // ---------------------------------------------------------------------------
  // Handlers
  // ---------------------------------------------------------------------------

  function handlePlayPause(): void {
    if (isFinished()) return;
    if (isRunning()) {
      stopLoop();
    } else {
      startLoop();
    }
  }

  function handleSpeedChange(mode: SpeedMode): void {
    setSpeedMode(mode);
    // If the loop is currently running, restart the interval at the new cadence.
    // This is imperative rather than reactive (no createEffect) to avoid the
    // isRunning() re-trigger cycle.
    if (isRunning() && intervalId !== undefined) {
      clearInterval(intervalId);
      intervalId = undefined;
      if (mode !== "auto") setAutoEventPaused(false);
      intervalId = setInterval(() => {
        void doStep();
      }, STEP_INTERVAL_MS);
    }
  }

  function handleResume(): void {
    startLoop();
  }

  function handleBackToCareer(): void {
    navigate("/career");
  }

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  return (
    <ErrorBoundary>
      <div class="space-y-3">
        {/* Browser-preview banner */}
        <Show when={!backendAvailable()}>
          <div
            class="rounded border border-flag-amber bg-flag-amber/10 px-3 py-2 text-xs font-mono text-ink-subtle dark:text-paper-subtle"
            role="status"
          >
            Preview mode (no Tauri backend). Match uses a mock sim.
          </div>
        </Show>

        {/* Error */}
        <Show when={errorMsg()}>
          <div
            class="fw-panel p-2 bg-flag-red/5 border border-flag-red/20 text-xs font-mono text-flag-red"
            role="alert"
          >
            {errorMsg()}
          </div>
        </Show>

        {/* Loading */}
        <Show when={loading()}>
          <div class="fw-panel p-4 text-sm text-ink-mute dark:text-paper-subtle">
            Getting teams on the pitch…
          </div>
        </Show>

        <Show when={!loading()}>
          {/* ---------------------------------------------------------------- */}
          {/* Scoreline                                                         */}
          {/* ---------------------------------------------------------------- */}
          <LiveScoreline
            home={homeScore()}
            away={awayScore()}
            clock={clock()}
            seedHex={handle()?.seedHex ?? initialSeed}
          />

          {/* ---------------------------------------------------------------- */}
          {/* Status bar + play/pause + speed controls                         */}
          {/* ---------------------------------------------------------------- */}
          <div class="fw-panel px-3 py-2 flex flex-wrap items-center gap-3">
            {/* Status label */}
            <span
              class="text-xs font-mono text-ink-mute dark:text-paper-subtle flex-1"
              aria-live="polite"
              aria-label="Match status"
            >
              {statusLabel()}
            </span>

            {/* Play / Pause button */}
            <Show when={!isFinished()}>
              <button
                type="button"
                class="px-3 py-1 text-xs font-mono rounded border focus:outline-none focus:ring-2 focus:ring-pitch-400 bg-pitch-500 text-white border-pitch-600 hover:bg-pitch-600 disabled:opacity-40 disabled:cursor-not-allowed"
                onClick={handlePlayPause}
                aria-pressed={isRunning()}
                aria-label={isRunning() ? "Pause the match" : "Resume the match"}
              >
                {isRunning() ? "Pause" : "Resume"}
              </button>
            </Show>

            {/* Speed controls — disabled after finish */}
            <Show when={!isFinished()}>
              <div
                class="flex gap-1"
                role="group"
                aria-label="Simulation speed"
              >
                <For each={SPEED_MODES}>
                  {(mode) => (
                    <button
                      type="button"
                      class={`px-2 py-0.5 text-xs font-mono rounded border focus:outline-none focus:ring-2 focus:ring-pitch-400 ${
                        speedMode() === mode
                          ? "bg-pitch-500 text-white border-pitch-600"
                          : "bg-paper dark:bg-midnight-panel text-ink-subtle dark:text-paper-subtle border-paper-bold dark:border-midnight-line hover:border-pitch-400"
                      }`}
                      onClick={() => {
                        handleSpeedChange(mode);
                        // If paused and user picks a speed, also resume.
                        if (!isRunning()) {
                          handleResume();
                        }
                      }}
                      aria-pressed={speedMode() === mode}
                      aria-label={speedAriaLabel(mode)}
                    >
                      {speedLabel(mode)}
                    </button>
                  )}
                </For>
              </div>
            </Show>
          </div>

          {/* ---------------------------------------------------------------- */}
          {/* S12: Half-time interstitial                                       */}
          {/* ---------------------------------------------------------------- */}
          <Show when={halfTimePaused() && !isFinished()}>
            <HalfTimeInterstitial
              homeScore={homeScore()}
              awayScore={awayScore()}
              keyEvents={keyEvents()}
              onResume={handleHalfTimeResume}
            />
          </Show>

          {/* ---------------------------------------------------------------- */}
          {/* S10: Touchline instructions panel                                 */}
          {/* ---------------------------------------------------------------- */}
          <Show when={!isFinished() && handle()}>
            {(h) => (
              <TouchlinePanel
                handle={h()}
                disabled={false}
              />
            )}
          </Show>

          {/* ---------------------------------------------------------------- */}
          {/* Auto-pause: "Resume" CTA when paused at a key moment             */}
          {/* ---------------------------------------------------------------- */}
          <Show when={autoEventPaused() && !isFinished()}>
            <div class="fw-panel px-3 py-2 flex items-center justify-between gap-3 bg-pitch-50 dark:bg-pitch-950/30 border border-pitch-200 dark:border-pitch-800">
              <span class="text-xs font-mono text-pitch-700 dark:text-pitch-300">
                {(() => {
                  const last = keyEvents().at(-1);
                  return last
                    ? `${eventLabel(last.kind)} — ${last.description ?? "key moment"}`
                    : "Key moment";
                })()}
              </span>
              <button
                type="button"
                class="px-3 py-1 text-xs font-mono rounded border focus:outline-none focus:ring-2 focus:ring-pitch-400 bg-pitch-500 text-white border-pitch-600 hover:bg-pitch-600"
                onClick={handleResume}
                aria-label="Continue watching the match"
              >
                Continue
              </button>
            </div>
          </Show>

          {/* ---------------------------------------------------------------- */}
          {/* 2D Tactical board                                                 */}
          {/* ---------------------------------------------------------------- */}
          <Show
            when={liveFrames().length > 0}
            fallback={
              <div class="fw-panel p-3 text-xs text-ink-mute dark:text-paper-subtle">
                Board will appear once play gets underway…
              </div>
            }
          >
            <Suspense
              fallback={
                <div class="fw-panel p-4 text-sm text-ink-mute dark:text-paper-subtle">
                  Loading board…
                </div>
              }
            >
              <div class="fw-panel p-2">
                <TacticalBoard frames={liveFrames()} followLatest={!isFinished()} />
              </div>
            </Suspense>
          </Show>

          {/* ---------------------------------------------------------------- */}
          {/* Key-moment feed                                                   */}
          {/* ---------------------------------------------------------------- */}
          <section aria-label="Key moments" class="space-y-1">
            <h2 class="font-display text-base text-pitch-600 dark:text-pitch-300 px-1">
              Key moments
            </h2>
            <ul
              class="fw-panel divide-y divide-paper-bold dark:divide-midnight-line"
              aria-label="Key moment list"
              aria-live="polite"
              aria-atomic="false"
            >
              <Show
                when={keyEvents().length > 0}
                fallback={
                  <li class="text-xs text-ink-mute italic p-2">
                    Waiting for the first moment of note…
                  </li>
                }
              >
                <For each={keyEvents()}>
                  {(ev) => <KeyMomentItem event={ev} />}
                </For>
              </Show>
            </ul>
          </section>

          {/* ---------------------------------------------------------------- */}
          {/* Final result panel                                                */}
          {/* ---------------------------------------------------------------- */}
          <Show when={isFinished() && finalResult()}>
            {(final) => (
              <div
                class="fw-panel p-4 space-y-3"
                aria-label="Final result"
                role="region"
              >
                <h2 class="font-display text-lg text-pitch-600 dark:text-pitch-300">
                  Full time
                </h2>
                <p class="text-sm text-ink-subtle dark:text-paper-subtle">
                  Final score:{" "}
                  <span class="font-mono tabular-nums text-pitch-600 dark:text-pitch-300">
                    {final().finalScore.home} – {final().finalScore.away}
                  </span>
                  {" "}&middot; {final().totalEvents} events recorded
                </p>
                <p class="text-xs font-mono text-ink-mute dark:text-paper-subtle">
                  Seed: {final().handle.seedHex}
                </p>
                <button
                  type="button"
                  class="px-4 py-1.5 rounded text-sm font-mono bg-pitch-500 text-white hover:bg-pitch-600 focus:outline-none focus:ring-2 focus:ring-pitch-400"
                  onClick={handleBackToCareer}
                  aria-label="Back to career"
                >
                  Back to career
                </button>
              </div>
            )}
          </Show>
        </Show>
      </div>
    </ErrorBoundary>
  );
}

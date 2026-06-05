/*
 * LiveMatch route — S3b.
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
 *   - On mount: startLiveMatch(seed) — seed comes from router location state
 *     (same pattern as ClubSelection) or a random hex if navigating directly.
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
 * IPC (read-only; never mutates canonical state — CLAUDE.md §7):
 *   startLiveMatch, stepLiveMatch, finishLiveMatch from ~/lib/api/live_match.ts
 *
 * Rules compliance:
 *   - No `any` (Frontend/RULES.md §6)
 *   - Solid signals / stores not React hooks (Frontend/RULES.md §1)
 *   - setInterval cleanup in onCleanup (Frontend/RULES.md §1 — side-effect cleanup)
 *   - PixiJS board created once, follows latest frame (Frontend/RULES.md §4)
 *   - ARIA: speed buttons are aria-pressed; event feed is aria-live; board is aria-label
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
import { describeRouteError } from "~/lib/route-errors";
import { backendAvailable } from "~/lib/tauri";
import {
  startLiveMatch,
  stepLiveMatch,
  finishLiveMatch,
} from "~/lib/api/live_match";
import { isKeyMomentKind } from "~/lib/match-events";
import type {
  FinalMatchResult,
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
// Main component
// ---------------------------------------------------------------------------

export default function LiveMatch(): JSX.Element {
  const location = useLocation<{ seedHex?: string }>();
  const navigate = useNavigate();

  // Seed: from router state or a fresh random hex for dev direct-entry.
  const initialSeed =
    (location.state as { seedHex?: string } | null)?.seedHex ?? randomSeedHex();

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

  // ---------------------------------------------------------------------------
  // Mount: start the session + kick off the loop
  // ---------------------------------------------------------------------------

  onMount(() => {
    void (async () => {
      try {
        let h: MatchHandle;
        if (!backendAvailable()) {
          h = makeMockHandle(initialSeed);
        } else {
          h = await startLiveMatch(initialSeed);
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

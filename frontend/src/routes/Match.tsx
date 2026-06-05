/*
 * Match page — T1-6.
 *
 * Text-first per DESIGN_DOC §3 pillar 1. The scoreline + event list + commentary
 * are the primary read. The PixiJS dev board is opt-in behind a toggle.
 *
 * Layout (after Play):
 *   header  — seed input | ticks input | Play button
 *   scoreline — HH:MM  Home N – N Away  (goals emphasised)
 *   main column — event list with minute markers + tag badges
 *   aside — commentary preview (pre-rendered prose from backend)
 *   [optional] dev board — FrameSource-driven scrubbable 2D pitch
 *
 * IPC:
 *   playMatch(seed, tickCount) → MatchResult   (read-only; never mutates sim)
 *   Dev board separately calls match_frames IPC (via Dev/TacticalBoard FrameSource).
 *
 * Rules compliance:
 *   - No `any` (Frontend/RULES.md §6)
 *   - Solid signals/memos not React hooks (Frontend/RULES.md §1)
 *   - IpcError narrowed via type guard + exhaustiveness check (high-substance req)
 *   - Dev board toggle destroys Pixi context on unmount (Frontend/RULES.md §4)
 *   - Keyboard nav + ARIA labels (Frontend/RULES.md §8)
 *   - Banned-terms compliant (football-native vocabulary only)
 */

import {
  createMemo,
  createSignal,
  For,
  lazy,
  Show,
  Suspense,
  type JSX,
} from "solid-js";
import ErrorBoundary from "~/components/ErrorBoundary";
import { describeRouteError } from "~/lib/route-errors";
import { backendAvailable, playMatch } from "~/lib/tauri";
import type {
  MatchEvent,
  MatchEventKind,
  MatchResult,
  MatchFrameDTO,
} from "~/lib/types";
import { MAX_FRAMES_PER_REQUEST } from "~/lib/types";
import { TauriFrameSource } from "~/routes/Dev/FrameSource";

// ---------------------------------------------------------------------------
// Production TacticalBoard — lazy-imported so the PixiJS bundle only loads
// when toggled on. T4-1: this replaces the DevTacticalBoard inline hack.
// The production board accepts a `frames` prop directly (no URL-param coupling).
// ---------------------------------------------------------------------------
const ProductionTacticalBoard = lazy(() => import("~/components/TacticalBoard"));

// IpcError type was imported above for the existing `IpcError`-typed imports.
// T4-4: isIpcError / formatIpcError were only used by describeError which is
// now replaced by describeRouteError from ~/lib/route-errors. The IpcError
// union import is still needed for the type annotation in Match.tsx's imports.

// ---------------------------------------------------------------------------
// Event-type filter — S2: key-moments view
//
// This is an event-TYPE filter, not a salience filter. compute_salience() is
// an identity function today (all stakes hardcoded to Q32::ONE) and
// MatchEvent carries no salience field at all — routing this through
// salience would be dishonest. The filter simply hides high-frequency
// ball-movement events (Pass / PassIncomplete) so the feed focuses on
// match-defining moments.
//
// Kinds suppressed when keyMomentsOnly === true:
//   "Pass" | "PassIncomplete"
//
// Kinds always shown:
//   "Goal" | "Shot" | "KickOff" | "HalfTime" | "FullTime"
//   | "Offside" | "Card" | "Substitution" | "SignatureFirstFired"
// ---------------------------------------------------------------------------

/**
 * Returns true for kinds that are suppressed by the key-moments type filter.
 * When keyMomentsOnly is false this function is never called.
 */
function isHighFrequencyKind(kind: MatchEventKind): boolean {
  return kind === "Pass" || kind === "PassIncomplete";
}

// ---------------------------------------------------------------------------
// Event-list helpers
// ---------------------------------------------------------------------------

// T1-6 fix-pass per type-design P1 + silent-failure P3: typed-narrow
// helpers below take `MatchEventKind` (closed union) not `string`. Adding a
// new variant in `lib/types.ts` produces a compile error at the `never`
// default arm in `eventLabel`, surfacing the drift instead of silently
// rendering the raw kind string in a UI badge.

function isGoal(kind: MatchEventKind): boolean {
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
      return "Substitution";
    case "SignatureFirstFired":
      return "Signature";
    case "Offside":
      return "Offside";
    // FUN-CB1: failed pass — ball released, possession lost.
    case "PassIncomplete":
      return "Lost";
    default: {
      // Post-T2-close Track C-1 gate-blocker fix: throw not return — see
      // formatIpcError above for full rationale. A future MatchEventKind
      // variant that lands without a case arm here used to silently render
      // the raw object as `[object Object]` in event-list badges.
      const _exhaustive: never = kind;
      throw new Error(
        `eventLabel: unhandled MatchEventKind — types.ts / eventLabel drift. kind=${JSON.stringify(_exhaustive)}`,
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
    // FUN-CB1: neutral badge — same group as other ball-in-play events.
    // eslint-disable-next-line no-fallthrough
    case "PassIncomplete":
      return "bg-paper-bold text-ink-subtle dark:bg-midnight-subtle dark:text-paper-subtle";
    default: {
      // Post-T2-close Track C-1 gate-blocker fix: throw not return. Prior
      // pattern returned `_exhaustive` which evaluates to the raw object at
      // runtime, then propagated as a literal `class=` attribute value of
      // `[object Object]` — a CSS-invalid string that browsers ignore,
      // silently dropping the badge styling.
      const _exhaustive: never = kind;
      throw new Error(
        `badgeClass: unhandled MatchEventKind — types.ts / badgeClass drift. kind=${JSON.stringify(_exhaustive)}`,
      );
    }
  }
}

// ---------------------------------------------------------------------------
// Mock result for browser-preview fallback
// ---------------------------------------------------------------------------

function makeMockResult(seedHex: string, tickCount: number): MatchResult {
  return {
    finalScore: { home: 2, away: 1 },
    canonicalHash: "blake3:" + "0".repeat(64),
    matchEvents: [
      { tick: 0, minute: 0, kind: "KickOff", description: "Kick-off." },
      { tick: 540, minute: 9, kind: "Goal", description: "Goal to home side." },
      { tick: 1620, minute: 27, kind: "Shot", description: "Shot on target." },
      { tick: 2700, minute: 45, kind: "HalfTime", description: "Half-time." },
      {
        tick: 3240,
        minute: 54,
        kind: "Goal",
        description: "Goal to away side.",
      },
      {
        tick: 4500,
        minute: 75,
        kind: "Goal",
        description: "Goal to home side.",
      },
      { tick: 5400, minute: 90, kind: "FullTime", description: "Full-time." },
    ],
    commentaryPreview: [
      "The referee's whistle starts proceedings.",
      "Neat finish from the forward.",
      "The keeper is called into action.",
      "The sides head in level.",
      "A clinical header levels the match.",
      "Late drama as the home side retake the lead.",
      "Three points secured after a hard-fought contest.",
    ],
    seedHex,
    tickCount,
  };
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

interface ScorelineProps {
  result: MatchResult;
}

function Scoreline(props: ScorelineProps): JSX.Element {
  return (
    <div
      class="fw-panel p-4 flex items-center justify-center gap-6"
      aria-label="Final score"
      role="region"
    >
      <span class="font-display text-4xl tabular-nums text-pitch-600 dark:text-pitch-300">
        {props.result.finalScore.home}
      </span>
      <span class="text-ink-mute dark:text-paper-subtle font-mono text-xl select-none">
        –
      </span>
      <span class="font-display text-4xl tabular-nums text-pitch-600 dark:text-pitch-300">
        {props.result.finalScore.away}
      </span>
      {/* T1-6 fix-pass per silent-failure P2 + type-design P3: prior label
          rendered "HT <seedHex>" which reads as "Half-time <hex>" in
          football vocabulary — confusing the seed for a half-time score.
          Explicit "Seed:" prefix removes the misnomer. */}
      <span class="text-xs font-mono text-ink-mute dark:text-paper-subtle ml-2">
        Seed: {props.result.seedHex}
      </span>
    </div>
  );
}

interface EventRowProps {
  event: MatchEvent;
}

function EventRow(props: EventRowProps): JSX.Element {
  const eventIsGoal = () => isGoal(props.event.kind);

  return (
    <li
      class={`flex items-start gap-2 py-1 px-2 rounded text-sm ${eventIsGoal() ? "bg-pitch-50 dark:bg-pitch-950/40" : ""}`}
    >
      {/* Minute marker */}
      <span
        class="font-mono text-xs text-ink-mute dark:text-paper-subtle w-8 pt-0.5 shrink-0 text-right"
        aria-label={`Minute ${props.event.minute}`}
      >
        {props.event.minute}&apos;
      </span>
      {/* Kind badge */}
      <span
        class={`text-xs font-mono px-1 py-0.5 rounded shrink-0 ${badgeClass(props.event.kind)}`}
      >
        {eventLabel(props.event.kind)}
      </span>
      {/* Description */}
      <Show when={props.event.description}>
        <span class="text-xs text-ink-subtle dark:text-paper-subtle">
          {props.event.description}
        </span>
      </Show>
    </li>
  );
}

// ---------------------------------------------------------------------------
// Seed + ticks form helpers
// ---------------------------------------------------------------------------

const DEFAULT_SEED_HEX = "0xfeedbeefcafefade";
// 5400 ticks = 90 minutes at 1 tick/second resolution. Fits well under
// MAX_FRAMES_PER_REQUEST (7200). Real match-time calibration is T1-9.
const DEFAULT_TICK_COUNT = 900;

function parseSeedBigInt(hex: string): bigint | null {
  const trimmed = hex.trim();
  try {
    // Accept "0x..." prefix or bare hex.
    const normalised = trimmed.startsWith("0x") ? trimmed : "0x" + trimmed;
    const val = BigInt(normalised);
    // Clamp to u64 range.
    if (val < 0n || val > 0xffffffffffffffffn) return null;
    return val;
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export default function Match(): JSX.Element {
  // Seed as hex string (user edits raw hex; we parse to BigInt on submit).
  const [seedInput, setSeedInput] = createSignal(DEFAULT_SEED_HEX);
  const [tickInput, setTickInput] = createSignal(String(DEFAULT_TICK_COUNT));

  const [result, setResult] = createSignal<MatchResult | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [errorMsg, setErrorMsg] = createSignal<string | null>(null);
  const [showDevBoard, setShowDevBoard] = createSignal(false);

  // S2: key-moments event-type filter (default ON).
  // Suppresses Pass + PassIncomplete so the feed shows match-defining moments
  // only. Named as a type filter — NOT a salience filter (salience is
  // degenerate today; MatchEvent has no salience field).
  const [keyMomentsOnly, setKeyMomentsOnly] = createSignal(true);

  // Parsed values — derived memos so the form stays reactive.
  const seedBigInt = createMemo<bigint | null>(() =>
    parseSeedBigInt(seedInput()),
  );
  // T1-6 fix-pass per silent-failure P1: the prior memo returned
  // DEFAULT_TICK_COUNT on invalid input ("abc" / "0" / "-50"), which made
  // `ticksValid()` silently true → Play button enabled → match ran with the
  // default ignored input + no user feedback. NaN-on-invalid surfaces the
  // failure via the same `aria-invalid` + disabled-Play path `seedValid`
  // uses for malformed hex.
  const tickCount = createMemo<number>(() => {
    const raw = tickInput().trim();
    if (raw === "") return Number.NaN;
    const n = parseInt(raw, 10);
    return Number.isFinite(n) ? n : Number.NaN;
  });
  const seedValid = createMemo(() => seedBigInt() !== null);
  const ticksValid = createMemo(() => {
    const n = tickCount();
    return Number.isFinite(n) && n > 0 && n <= MAX_FRAMES_PER_REQUEST;
  });
  const canPlay = createMemo(
    () => !busy() && seedValid() && ticksValid(),
  );

  // The seed + ticks used for the last completed run — used to load frames
  // for the production tactical board.
  const [lastSeedHex, setLastSeedHex] = createSignal<string | null>(null);
  const [lastTicks, setLastTicks] = createSignal<number>(DEFAULT_TICK_COUNT);
  // Frames loaded for the last run's seed — passed to the production board.
  const [boardFrames, setBoardFrames] = createSignal<MatchFrameDTO[]>([]);
  const [framesLoading, setFramesLoading] = createSignal(false);
  // Set when the post-run tactical-board frame load fails — surfaced as a
  // board-local notice DISTINCT from the legitimate "no frames" empty state,
  // so an IPC failure is never silently indistinguishable from an empty board.
  const [framesError, setFramesError] = createSignal<string | null>(null);

  const onPlay = async () => {
    setErrorMsg(null);
    const seed = seedBigInt();
    if (seed === null) {
      setErrorMsg("Invalid seed. Enter a hex value such as 0xfeedbeefcafefade.");
      return;
    }
    const ticks = tickCount();
    // T1-6 fix-pass: tickCount() can be NaN (invalid input); guard explicitly
    // so the error message is helpful rather than confusing.
    if (!Number.isFinite(ticks) || ticks <= 0 || ticks > MAX_FRAMES_PER_REQUEST) {
      setErrorMsg(
        `Tick count must be a number between 1 and ${MAX_FRAMES_PER_REQUEST}.`,
      );
      return;
    }

    setBusy(true);
    try {
      if (!backendAvailable()) {
        // Browser-preview fallback — return a reasonable mock so the surface
        // renders without a live sim. The mock includes goals + a full event
        // list so the minute-marker + badge rendering is exercised.
        setResult(makeMockResult(seedInput(), ticks));
      } else {
        setResult(await playMatch(seed, ticks));
      }
      setLastSeedHex(seedInput());
      setLastTicks(ticks);
      // Load frames for the production board after a successful run.
      // Frames are loaded in the background; the board renders as they arrive.
      // In pure browser-preview mode (no backend) we skip frame loading.
      if (backendAvailable()) {
        setFramesLoading(true);
        setBoardFrames([]);
        setFramesError(null);
        void (async () => {
          try {
            // Use `lastSeedHex()` (the seed that actually ran, captured above)
            // — NOT the live `seedInput()`, which the user may have edited
            // since pressing Play.
            const source = new TauriFrameSource(lastSeedHex()!, ticks);
            const loaded = await source.loadFrames();
            setBoardFrames(loaded);
          } catch (e: unknown) {
            // Frame-load failure is non-fatal — the text recap already shows —
            // but it must NOT be silent: log it + surface a board-local notice
            // distinct from the legitimate "no frames" empty state. Do not
            // overwrite a match error.
            console.error("[Match] tactical-board frame load failed:", e);
            setFramesError(e instanceof Error ? e.message : String(e));
          } finally {
            setFramesLoading(false);
          }
        })();
      }
    } catch (e: unknown) {
      // Use describeRouteError for the match-play error — avoids leaking raw
      // err.message into the UI for non-IpcError throws.
      const copy = describeRouteError(e, { what: "the match" });
      setErrorMsg(`${copy.headline}: ${copy.detail}`);
    } finally {
      setBusy(false);
    }
  };

  const goalEvents = createMemo(
    () => result()?.matchEvents.filter((ev) => isGoal(ev.kind)) ?? [],
  );

  // S2: filtered views for the event list and commentary aside.
  // Both memos use the same index-based predicate so the two lists stay in
  // sync — commentaryPreview[i] corresponds to matchEvents[i] (1:1 parallel
  // arrays from the backend). Filtering by index keeps them aligned after
  // Pass / PassIncomplete rows are dropped.
  const filteredEventIndices = createMemo<number[]>(() => {
    const r = result();
    if (!r) return [];
    const filter = keyMomentsOnly();
    return r.matchEvents.reduce<number[]>((acc, ev, i) => {
      if (!filter || !isHighFrequencyKind(ev.kind)) acc.push(i);
      return acc;
    }, []);
  });

  // Filter the events array directly (clean MatchEvent[] type); the commentary
  // aside is kept in lockstep by reusing the SAME index set against the
  // parallel commentaryPreview array. Both narrow away the `| undefined` that
  // strict indexed access introduces — the indices are always in range.
  const filteredEvents = createMemo<MatchEvent[]>(() => {
    const r = result();
    if (!r) return [];
    const filter = keyMomentsOnly();
    return r.matchEvents.filter((ev) => !filter || !isHighFrequencyKind(ev.kind));
  });

  const filteredCommentary = createMemo<string[]>(() => {
    const r = result();
    if (!r) return [];
    return filteredEventIndices()
      .map((i) => r.commentaryPreview[i])
      .filter((line): line is string => line !== undefined);
  });

  return (
    // ErrorBoundary catches only synchronous throws from the reactive graph
    // (createResource fetchers, createMemo, JSX render). The Match route's
    // async work — `onPlay` and the fire-and-forget frame-load — handles its
    // own failures imperatively via `errorMsg()` / `framesError`; this
    // boundary is the safety net for any future render-time throw inside the
    // route, not for those async paths.
    <ErrorBoundary>
      <div class="space-y-4">
        {/* T1-6 fix-pass per silent-failure P3: in browser-preview mode
            (running `pnpm dev` outside Tauri) the Play button returns a
            mock MatchResult. Without this banner a developer can think
            they exercised the real IPC path; the all-zero blake3 hash is
            visible but only in the small commentary aside footer. */}
        <Show when={!backendAvailable()}>
          <div
            class="rounded border border-flag-amber bg-flag-amber/10 px-3 py-2 text-xs font-mono text-ink-subtle dark:text-paper-subtle"
            role="status"
          >
            Preview mode (no Tauri). Play returns a mock result, not a real
            sim run.
          </div>
        </Show>
        {/* ---------------------------------------------------------------- */}
        {/* Header: seed + ticks inputs + Play button                        */}
        {/* ---------------------------------------------------------------- */}
        <header class="fw-panel p-3 flex flex-wrap items-end gap-3">
          <div class="flex-1 min-w-[180px]">
            <label
              class="block text-xs font-mono text-ink-mute dark:text-paper-subtle mb-0.5"
              for="seed-input"
            >
              Seed (hex)
            </label>
            <input
              id="seed-input"
              type="text"
              class={`w-full font-mono text-sm px-2 py-1 rounded border bg-paper dark:bg-midnight-panel text-ink dark:text-paper ${
                seedValid()
                  ? "border-paper-bold dark:border-midnight-line"
                  : "border-flag-red"
              } focus:outline-none focus:ring-2 focus:ring-pitch-500`}
              value={seedInput()}
              aria-label="Match seed (hexadecimal)"
              aria-invalid={!seedValid()}
              onInput={(e) => setSeedInput(e.currentTarget.value)}
            />
          </div>
          <div class="w-28">
            <label
              class="block text-xs font-mono text-ink-mute dark:text-paper-subtle mb-0.5"
              for="ticks-input"
            >
              Ticks
            </label>
            <input
              id="ticks-input"
              type="number"
              min="1"
              max={MAX_FRAMES_PER_REQUEST}
              class={`w-full font-mono text-sm px-2 py-1 rounded border bg-paper dark:bg-midnight-panel text-ink dark:text-paper ${
                ticksValid()
                  ? "border-paper-bold dark:border-midnight-line"
                  : "border-flag-red"
              } focus:outline-none focus:ring-2 focus:ring-pitch-500`}
              value={tickInput()}
              aria-label={`Tick count (1–${MAX_FRAMES_PER_REQUEST})`}
              aria-invalid={!ticksValid()}
              onInput={(e) => setTickInput(e.currentTarget.value)}
            />
          </div>
          <button
            type="button"
            class="px-4 py-1.5 rounded text-sm font-mono bg-pitch-500 text-white hover:bg-pitch-600 disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus:ring-2 focus:ring-pitch-400"
            onClick={() => void onPlay()}
            disabled={!canPlay()}
            aria-label={busy() ? "Simulating match…" : "Play match"}
          >
            {busy() ? "Simulating…" : "Play match"}
          </button>
        </header>

        {/* Error display */}
        <Show when={errorMsg()}>
          <div
            class="fw-panel p-2 bg-flag-red/5 border border-flag-red/20 text-xs font-mono text-flag-red"
            role="alert"
          >
            {errorMsg()}
          </div>
        </Show>

        {/* ---------------------------------------------------------------- */}
        {/* Scoreline (visible only after a run)                             */}
        {/* ---------------------------------------------------------------- */}
        <Show when={result()}>
          {(r) => <Scoreline result={r()} />}
        </Show>

        {/* ---------------------------------------------------------------- */}
        {/* Main content: event list + commentary aside                      */}
        {/* ---------------------------------------------------------------- */}
        <Show
          when={result()}
          fallback={
            <div class="fw-panel p-4 text-sm text-ink-mute dark:text-paper-subtle">
              Press <span class="font-mono font-bold">Play match</span> to run
              the sim. The event list and commentary recap will appear here.
            </div>
          }
        >
          {(r) => (
            <div class="grid grid-cols-1 lg:grid-cols-[1fr_300px] gap-4">
              {/* Event list */}
              <section aria-label="Match events">
                <div class="flex items-center gap-3 mb-2">
                  <h2 class="font-display text-lg text-pitch-600 dark:text-pitch-300">
                    Events
                  </h2>
                  {/* S2: event-type filter toggle — hides Pass / PassIncomplete.
                      This is NOT a salience filter (salience is degenerate today). */}
                  <button
                    type="button"
                    class={`px-2 py-0.5 text-xs font-mono rounded border focus:outline-none focus:ring-2 focus:ring-pitch-400 ${
                      keyMomentsOnly()
                        ? "bg-pitch-500 text-white border-pitch-600"
                        : "bg-paper dark:bg-midnight-panel text-ink-subtle dark:text-paper-subtle border-paper-bold dark:border-midnight-line hover:border-pitch-400"
                    }`}
                    onClick={() => setKeyMomentsOnly((v) => !v)}
                    aria-pressed={keyMomentsOnly()}
                    aria-label={
                      keyMomentsOnly()
                        ? "Showing key moments — click to show all events"
                        : "Showing all events — click to show key moments only"
                    }
                  >
                    {keyMomentsOnly() ? "Key moments" : "All events"}
                  </button>
                </div>
                <Show when={goalEvents().length > 0}>
                  <p class="text-xs text-ink-mute dark:text-paper-subtle mb-2 font-mono">
                    Goals:{" "}
                    {goalEvents()
                      .map((ev) => `${ev.minute}'`)
                      .join(", ")}
                  </p>
                </Show>
                <ul
                  class="fw-panel divide-y divide-paper-bold dark:divide-midnight-line"
                  aria-label="Match event list"
                >
                  <For
                    each={filteredEvents()}
                    fallback={
                      <li class="text-xs text-ink-mute italic p-2">
                        No events recorded.
                      </li>
                    }
                  >
                    {(ev) => <EventRow event={ev} />}
                  </For>
                </ul>
              </section>

              {/* Commentary aside — filtered in sync with the event list via
                  the same index predicate (S2 key-moments type filter). */}
              <aside aria-label="Commentary">
                <h2 class="font-display text-lg text-pitch-600 dark:text-pitch-300 mb-2">
                  Commentary
                </h2>
                <div class="fw-panel p-3 space-y-1">
                  <For
                    each={filteredCommentary()}
                    fallback={
                      <p class="text-xs text-ink-mute italic">
                        No commentary available.
                      </p>
                    }
                  >
                    {(line) => (
                      <p class="text-xs text-ink-subtle dark:text-paper-subtle leading-relaxed">
                        {line}
                      </p>
                    )}
                  </For>
                  <p class="pt-2 text-xs font-mono text-ink-mute dark:text-paper-subtle break-all border-t border-paper-bold dark:border-midnight-line mt-2">
                    {r().canonicalHash}
                  </p>
                </div>
              </aside>
            </div>
          )}
        </Show>

        {/* ---------------------------------------------------------------- */}
        {/* Tactical board toggle (opt-in; not the default surface)          */}
        {/* T4-1: production board wired to last-run frames via TauriFrameSource. */}
        {/* ---------------------------------------------------------------- */}
        <Show when={result() !== null}>
          <div class="flex items-center gap-2">
            <button
              type="button"
              class={`px-3 py-1 text-xs font-mono rounded border focus:outline-none focus:ring-2 focus:ring-pitch-400 ${
                showDevBoard()
                  ? "bg-pitch-500 text-white border-pitch-600"
                  : "bg-paper dark:bg-midnight-panel text-ink-subtle dark:text-paper-subtle border-paper-bold dark:border-midnight-line hover:border-pitch-400"
              }`}
              onClick={() => setShowDevBoard((v) => !v)}
              aria-pressed={showDevBoard()}
              aria-label={
                showDevBoard() ? "Hide tactical board" : "Show tactical board"
              }
            >
              {showDevBoard() ? "Hide tactical board" : "Show tactical board"}
            </button>
            <Show when={showDevBoard()}>
              <span class="text-xs text-ink-mute dark:text-paper-subtle font-mono">
                {lastSeedHex()} · {lastTicks()} ticks
                <Show when={framesLoading()}>
                  <span class="ml-1 italic"> · loading frames…</span>
                </Show>
                <Show when={framesError()}>
                  <span class="ml-1 text-rose-600 dark:text-rose-400">
                    {" "}
                    · frame load failed: {framesError()}
                  </span>
                </Show>
              </span>
            </Show>
          </div>
        </Show>

        {/* Production tactical board — lazy-loaded; onCleanup destroys Pixi
            Application when toggled off (Frontend/RULES.md §4).
            Frames are passed directly as a prop — no URL-param coupling. */}
        <Show when={showDevBoard()}>
          <Suspense
            fallback={
              <div class="fw-panel p-4 text-sm text-ink-mute dark:text-paper-subtle">
                Loading board…
              </div>
            }
          >
            <div class="fw-panel p-2">
              <ProductionTacticalBoard frames={boardFrames()} />
            </div>
          </Suspense>
        </Show>
      </div>
    </ErrorBoundary>
  );
}

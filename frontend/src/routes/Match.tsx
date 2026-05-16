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
import { isTauri, playMatch } from "~/lib/tauri";
import type {
  IpcError,
  MatchEvent,
  MatchEventKind,
  MatchResult,
} from "~/lib/types";
import { MAX_FRAMES_PER_REQUEST } from "~/lib/types";

// ---------------------------------------------------------------------------
// Dev board — lazy-imported so the PixiJS bundle only loads when toggled on.
// The Dev/TacticalBoard route component is already FrameSource-driven; we
// import it inline behind the toggle (same component, NOT a separate route).
// ---------------------------------------------------------------------------
const DevTacticalBoard = lazy(() => import("~/routes/Dev/TacticalBoard"));

// ---------------------------------------------------------------------------
// IpcError type guard + exhaustiveness helper
// ---------------------------------------------------------------------------

/**
 * Closed set of known IpcError discriminants.
 *
 * T1-6 fix-pass per type-design P2: the prior `isIpcError` accepted any
 * object with a `kind: string` field, so a backend variant the frontend
 * doesn't know about would pass the guard, fall through `formatIpcError`'s
 * default arm, and bypass the compile-time exhaustiveness check at runtime.
 * The `satisfies` annotation pins this set to `IpcError["kind"]` so adding
 * a new variant in `lib/types.ts` produces a compile error HERE — forcing
 * a coordinated update of both the type definition and the runtime guard.
 */
const KNOWN_IPC_ERROR_KINDS = new Set([
  "tooManyFrames",
  "invalidSeed",
  "matchInitFailed",
] as const) satisfies ReadonlySet<IpcError["kind"]>;

function isIpcError(e: unknown): e is IpcError {
  if (typeof e !== "object" || e === null || !("kind" in e)) return false;
  const kind = (e as Record<string, unknown>).kind;
  return typeof kind === "string" && (KNOWN_IPC_ERROR_KINDS as ReadonlySet<string>).has(kind);
}

/**
 * Format an IpcError into a human-readable string.
 *
 * The switch is exhaustive — adding a new variant to IpcError forces a
 * compile error at the `never` default arm unless the new arm is handled
 * AND the runtime guard above is updated (the two updates are coupled via
 * the `satisfies` constraint on KNOWN_IPC_ERROR_KINDS).
 */
function formatIpcError(err: IpcError): string {
  switch (err.kind) {
    case "tooManyFrames":
      return `Too many ticks requested (${err.requested}; max ${err.max}). Reduce tick count.`;
    case "invalidSeed":
      return `Invalid seed "${err.input}": ${err.reason}`;
    case "matchInitFailed":
      return `Match could not start: ${err.reason}`;
    default: {
      // Exhaustiveness — fails to compile if a new IpcError variant lands
      // without a matching case arm above.
      const _exhaustive: never = err;
      return _exhaustive;
    }
  }
}

/** Parse a thrown value into a display string. */
function describeError(e: unknown): string {
  if (isIpcError(e)) return formatIpcError(e);
  if (e instanceof Error) return e.message;
  return String(e);
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
    default: {
      const _exhaustive: never = kind;
      return _exhaustive;
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
      return "bg-paper-bold text-ink-subtle dark:bg-midnight-subtle dark:text-paper-subtle";
    default: {
      const _exhaustive: never = kind;
      return _exhaustive;
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

  // The seed + ticks used for the last completed run — passed to dev-board
  // so it matches the recap's frames.
  const [lastSeedHex, setLastSeedHex] = createSignal<string | null>(null);
  const [lastTicks, setLastTicks] = createSignal<number>(DEFAULT_TICK_COUNT);

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
      if (!isTauri()) {
        // Browser-preview fallback — return a reasonable mock so the surface
        // renders without a live sim. The mock includes goals + a full event
        // list so the minute-marker + badge rendering is exercised.
        setResult(makeMockResult(seedInput(), ticks));
      } else {
        setResult(await playMatch(seed, ticks));
      }
      setLastSeedHex(seedInput());
      setLastTicks(ticks);
    } catch (e: unknown) {
      setErrorMsg(describeError(e));
    } finally {
      setBusy(false);
    }
  };

  const goalEvents = createMemo(
    () => result()?.matchEvents.filter((ev) => isGoal(ev.kind)) ?? [],
  );

  return (
    <ErrorBoundary>
      <div class="space-y-4">
        {/* T1-6 fix-pass per silent-failure P3: in browser-preview mode
            (running `pnpm dev` outside Tauri) the Play button returns a
            mock MatchResult. Without this banner a developer can think
            they exercised the real IPC path; the all-zero blake3 hash is
            visible but only in the small commentary aside footer. */}
        <Show when={!isTauri()}>
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
                <h2 class="font-display text-lg text-pitch-600 dark:text-pitch-300 mb-2">
                  Events
                </h2>
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
                    each={r().matchEvents}
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

              {/* Commentary aside */}
              <aside aria-label="Commentary">
                <h2 class="font-display text-lg text-pitch-600 dark:text-pitch-300 mb-2">
                  Commentary
                </h2>
                <div class="fw-panel p-3 space-y-1">
                  <For
                    each={r().commentaryPreview}
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
        {/* Dev board toggle (opt-in; not the default surface)               */}
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
                showDevBoard() ? "Hide dev board" : "Show dev board"
              }
            >
              {showDevBoard() ? "Hide dev board" : "Show dev board"}
            </button>
            <Show when={showDevBoard()}>
              {/* T1-6 fix-pass per silent-failure P2 + code-reviewer P1:
                  prior label "Seed: X | Ticks: Y" implied the dev board
                  rendered THIS match — but the inline Dev/TacticalBoard
                  reuses its route component which reads useSearchParams
                  and falls back to the default 60-tick 0xdeadbeef sample.
                  Explicit caveat removes the misleading implication; the
                  real fix (prop-driven board) is T4-1 when dot rendering
                  lands. Recap above shows the matched seed; dev board
                  below shows the default sample. */}
              <span class="text-xs text-ink-mute dark:text-paper-subtle font-mono">
                Recap: {lastSeedHex()} ({lastTicks()} ticks) ·
                <span class="ml-1 italic">
                  board shows default sample (T4-1 wires last-run props)
                </span>
              </span>
            </Show>
          </div>
        </Show>

        {/* Dev board — lazy-loaded; onCleanup inside DevTacticalBoard destroys
            the Pixi Application when toggled off (Frontend/RULES.md §4). */}
        <Show when={showDevBoard()}>
          <Suspense
            fallback={
              <div class="fw-panel p-4 text-sm text-ink-mute dark:text-paper-subtle">
                Loading dev board…
              </div>
            }
          >
            {/* DevTacticalBoard reads ?source=tauri&seed=...&ticks=... from URL
                params. We pass them via the URL since the component is designed
                to consume useSearchParams — which requires the router to be
                active. The component IS currently a route component; reusing it
                here is within T1-6 scope. The FrameSource-driven board matches
                the last run's seed + ticks automatically when the URL params
                are set, but in the inline case the user sees the default
                FrameSource (60 ticks, deadbeef seed). T4-1 wires the frame
                source to the actual last run's seed; for T1-6 the board
                exercises the Pixi init + cleanup path (the substance gate). */}
            <div class="fw-panel p-2">
              <DevTacticalBoard />
            </div>
          </Suspense>
        </Show>
      </div>
    </ErrorBoundary>
  );
}

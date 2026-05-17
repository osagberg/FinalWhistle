/*
 * League page — T2-6.
 *
 * Text-first per DESIGN_DOC.md §3. Standings table (TanStack Table v8) is the
 * primary read. Two season action buttons drive the sim via IPC.
 *
 * IPC (read-only via season.ts wrappers):
 *   getStandings()   → StandingsRow[]          — loaded on mount
 *   advanceWeek()    → AdvanceWeekSummary      — "Advance Week" button
 *   playFixtures()   → PlayFixturesSummary     — "Play Fixtures" button
 *
 * Rules compliance:
 *   - No `any` (Frontend/RULES.md §6)
 *   - Solid signals / createResource, not React hooks (Frontend/RULES.md §1)
 *   - IpcError exhaustive switch + never discriminant (matches Match.tsx pattern)
 *   - Dark-mode tokens on every color-bearing class (Frontend/RULES.md §2)
 *   - Keyboard-accessible buttons with aria-label (Frontend/RULES.md §8)
 *   - Column defs split into ~/lib/columns/league.columns.ts (Frontend/RULES.md §3)
 *   - UI never drives canonical state — buttons enqueue via IPC only
 */

import { createResource, createSignal, Show, type JSX } from "solid-js";
import DataTable from "~/components/DataTable";
import {
  advanceWeek,
  getStandings,
  playFixtures,
} from "~/lib/api/season";
import { leagueColumns } from "~/lib/columns/league.columns";
import type {
  AdvanceWeekSummary,
  IpcError,
  PlayFixturesSummary,
  StandingsRow,
} from "~/lib/types";

// Total match-days in a season. Named constant rather than literal 38 in
// matchDayHeader so any future schedule-shape change has a single update site.
// Mirrors fw_content::MATCH_DAYS_PER_SEASON = 38.
const TOTAL_MATCH_DAYS = 38;

// Action outcome states — replaces the prior `actionError: unknown` signal so
// the React render branch knows whether the most-recent action SUCCEEDED with
// a summary OR FAILED with an error. Without this the user could click
// "Advance Week" against a complete season, get a no-op success that returns
// `{ matchesPlayed: 0, finalMatchDay: 38 }`, and see ZERO feedback in the UI
// — they'd click again, and again. Post-T2-6 silent-failure-hunter P1 fix.
type ActionOutcome =
  | { ok: true; kind: "advanceWeek"; summary: AdvanceWeekSummary }
  | { ok: true; kind: "playFixtures"; summary: PlayFixturesSummary }
  | { ok: false; error: IpcError | Error };

// ---------------------------------------------------------------------------
// IpcError type guard + exhaustiveness helper
//
// Mirrors the pattern in routes/Match.tsx — redefined here to keep this route
// self-contained (no cross-route import of internal helpers per project
// conventions; shared helpers go in ~/lib if ever needed by 3+ routes).
//
// The `satisfies` annotation pins KNOWN_IPC_ERROR_KINDS to IpcError["kind"],
// so adding a new variant to the IpcError union in lib/types.ts produces a
// compile error here, forcing a coordinated update of both sets.
// ---------------------------------------------------------------------------

const KNOWN_IPC_ERROR_KINDS = new Set([
  "tooManyFrames",
  "invalidSeed",
  "matchInitFailed",
  "seasonComplete",
  "clubNotFound",
  "lockPoisoned",
] as const) satisfies ReadonlySet<IpcError["kind"]>;

function isIpcError(e: unknown): e is IpcError {
  if (typeof e !== "object" || e === null || !("kind" in e)) return false;
  const kind = (e as Record<string, unknown>).kind;
  return (
    typeof kind === "string" &&
    (KNOWN_IPC_ERROR_KINDS as ReadonlySet<string>).has(kind)
  );
}

/**
 * Format an IpcError into a human-readable string.
 *
 * Exhaustive switch — adding a new IpcError variant forces a compile error at
 * the `never` default arm unless the arm is handled AND the runtime guard
 * above is updated.
 */
function formatIpcError(err: IpcError): string {
  switch (err.kind) {
    case "tooManyFrames":
      return `Too many ticks requested (${err.requested}; max ${err.max}).`;
    case "invalidSeed":
      return `Invalid seed "${err.input}": ${err.reason}`;
    case "matchInitFailed":
      return `Match could not start: ${err.reason}`;
    case "seasonComplete":
      return "The season is already complete — no more match-days to play.";
    case "clubNotFound":
      return `Club id ${err.clubId} was not found in the current league.`;
    case "lockPoisoned":
      return `Internal state was corrupted by a prior error (lock: ${err.lock}). Please restart the app.`;
    default: {
      // Post-T2-6 silent-failure-hunter P1 fix: prior code returned
      // `_exhaustive` which is typed `never` at compile-time but evaluates to
      // the actual runtime `err` object — concatenating into a template
      // literal yields `[object Object]` instead of useful diagnostic text.
      // The whole point of this pattern is to fail LOUD on a future variant
      // drift, not feed garbage to the alert region.
      const _exhaustive: never = err;
      throw new Error(
        `formatIpcError: unhandled IpcError variant — KNOWN_IPC_ERROR_KINDS / formatIpcError drift. err=${JSON.stringify(_exhaustive)}`,
      );
    }
  }
}

/** Describe any thrown value as a display string. */
function describeError(e: unknown): string {
  if (isIpcError(e)) return formatIpcError(e);
  if (e instanceof Error) return e.message;
  return String(e);
}

// ---------------------------------------------------------------------------
// Match-day header helper
//
// `played` from the first row is canonical for the entire table because
// advance_week is atomic — every club's `played` increments together.
// ---------------------------------------------------------------------------

function matchDayHeader(firstRow: StandingsRow): string {
  const p = firstRow.played;
  if (p === 0) return "Pre-season";
  if (p >= TOTAL_MATCH_DAYS) return "Season complete";
  return `Match-day ${p + 1} of ${TOTAL_MATCH_DAYS}`;
}

/** Normalise an arbitrary thrown value into the structured shape ActionOutcome wants. */
function normaliseError(e: unknown): IpcError | Error {
  if (isIpcError(e)) return e;
  if (e instanceof Error) return e;
  return new Error(String(e));
}

/** Render an ActionOutcome to a user-facing line. */
function describeActionOutcome(outcome: ActionOutcome): string {
  if (outcome.ok) {
    if (outcome.kind === "advanceWeek") {
      const s = outcome.summary;
      return s.seasonComplete
        ? `Match-day ${s.matchDayPlayed} played (${s.matchesPlayed} matches) — season complete.`
        : `Match-day ${s.matchDayPlayed} played (${s.matchesPlayed} matches).`;
    }
    // playFixtures
    const s = outcome.summary;
    return s.matchesPlayed === 0
      ? "No matches to play — the season is already complete."
      : `Played ${s.matchesPlayed} matches through match-day ${s.finalMatchDay}.`;
  }
  return describeError(outcome.error);
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export default function League(): JSX.Element {
  // Standings fetch error — stored in a typed signal so it's fully reactive and
  // observable in the jsdom test environment (avoids SolidJS dev-mode
  // castError disconnection when createResource rejects with a plain object).
  //
  // Post-T2-6 silent-failure-hunter P1 fix: type narrowed from `unknown` to
  // `IpcError | Error | null` so the consumer doesn't have to re-narrow at
  // every read site; normalised via `normaliseError()` in the fetcher's
  // catch arm.
  const [standingsError, setStandingsError] = createSignal<IpcError | Error | null>(
    null,
  );

  // Resource: drives getStandings() — refetched after each action.
  // The fetcher wraps getStandings() in a try/catch so errors land in the
  // local `standingsError` signal rather than propagating through
  // SolidJS's reactive error handler (which becomes an unhandled rejection
  // in jsdom and never updates the DOM).
  //
  // Post-T2-6 silent-failure-hunter P1 fix: returns `null` on failure
  // (distinct from `[]` which means "league is genuinely empty"). The render
  // branches gate on `standingsError()` first so a fetch-failed state
  // displays the alert WITHOUT also rendering "Pre-season" in the header
  // (the prior `[]` fallback caused that inconsistent dual state).
  const [standings, { refetch }] = createResource<StandingsRow[] | null>(
    async () => {
      setStandingsError(null);
      try {
        return await getStandings();
      } catch (e: unknown) {
        // Log to the dev console — the signal-only path used to silently
        // discard stack traces on navigation-away. `console.error` survives
        // route changes and surfaces in the tauri-plugin-log devtools.
        // eslint-disable-next-line no-console
        console.error("[League] getStandings failed:", e);
        setStandingsError(normaliseError(e));
        return null;
      }
    },
  );

  // Action state — shared across both buttons (only one action runs at a time).
  // `lastOutcome` replaces the prior `actionError` signal: it carries BOTH
  // success-with-summary and failure-with-error so the user gets visible
  // feedback on every action (the prior shape gave no success feedback at
  // all — see post-T2-6 silent-failure-hunter P1 fix on summary discard).
  const [actionPending, setActionPending] = createSignal(false);
  const [lastOutcome, setLastOutcome] = createSignal<ActionOutcome | null>(null);

  const runAdvanceWeek = async (): Promise<void> => {
    setActionPending(true);
    setLastOutcome(null);
    try {
      const summary = await advanceWeek();
      await refetch();
      setLastOutcome({ ok: true, kind: "advanceWeek", summary });
    } catch (e: unknown) {
      setLastOutcome({ ok: false, error: normaliseError(e) });
    } finally {
      setActionPending(false);
    }
  };

  const runPlayFixtures = async (): Promise<void> => {
    setActionPending(true);
    setLastOutcome(null);
    try {
      const summary = await playFixtures();
      await refetch();
      setLastOutcome({ ok: true, kind: "playFixtures", summary });
    } catch (e: unknown) {
      setLastOutcome({ ok: false, error: normaliseError(e) });
    } finally {
      setActionPending(false);
    }
  };

  return (
    <div class="space-y-4">
      {/* Header.
          Post-T2-6 silent-failure-hunter P1 fix: the sub-header is suppressed
          when standingsError() is set, so a fetch-failed state shows ONLY the
          alert below + no misleading "Pre-season" line (the prior `[]`
          fallback caused that inconsistent dual state). */}
      <header>
        <h1 class="font-display text-3xl text-pitch-600 dark:text-pitch-300">
          League
        </h1>
        <Show when={!standingsError()}>
          <Show
            when={standings()?.[0]}
            fallback={
              <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
                Pre-season
              </p>
            }
          >
            {(first) => (
              <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
                {matchDayHeader(first())}
              </p>
            )}
          </Show>
        </Show>
      </header>

      {/* Season action buttons.
          Post-T2-6 code-reviewer P1 fix: aria-label REMOVED from both buttons.
          aria-label on a button with visible text is an ARIA anti-pattern
          (Frontend/RULES.md §8 mandates aria-label on ICON-ONLY buttons; these
          are text-labeled). The accessible name now derives from the visible
          text content directly, eliminating the maintenance drift trap where
          aria-label and visible text could silently diverge under future copy
          changes. The `role="group" aria-label="Season actions"` wrapper
          remains the right place to label the button-group. */}
      <div class="flex gap-2" role="group" aria-label="Season actions">
        <button
          type="button"
          class="px-4 py-1.5 rounded text-sm font-mono bg-pitch-500 text-white hover:bg-pitch-600 disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus:ring-2 focus:ring-pitch-400"
          onClick={() => void runAdvanceWeek()}
          disabled={actionPending()}
        >
          {actionPending() ? "Working…" : "Advance Week"}
        </button>
        <button
          type="button"
          class="px-4 py-1.5 rounded text-sm font-mono bg-pitch-500 text-white hover:bg-pitch-600 disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus:ring-2 focus:ring-pitch-400"
          onClick={() => void runPlayFixtures()}
          disabled={actionPending()}
        >
          {actionPending() ? "Working…" : "Play Fixtures (Fast-Forward)"}
        </button>
      </div>

      {/* Action outcome — displayed inline below buttons.
          Carries both success-with-summary and failure-with-error so the
          user always gets visible feedback on every action (post-T2-6
          silent-failure-hunter P1 fix: prior shape discarded summary DTOs
          + no success feedback meant repeat-click confusion against a
          no-op completed season). */}
      <Show when={lastOutcome()}>
        {(outcome) => (
          <div
            class={`text-sm font-mono ${outcome().ok ? "text-pitch-600 dark:text-pitch-300" : "text-flag-red"}`}
            role="status"
            aria-live="polite"
          >
            {describeActionOutcome(outcome())}
          </div>
        )}
      </Show>

      {/* Standings table: loading / error / data states.
          standingsError signal is set by the resource fetcher's try/catch —
          fully reactive + updates the DOM in all environments, avoiding
          SolidJS dev-mode createResource rejection propagation.
          Post-T2-6 silent-failure-hunter P1 fix: the fetcher returns `null`
          on failure (distinct from `[]` which means "league genuinely empty").
          Render gates `standingsError()` first so a fetch-failed state
          displays ONLY the alert, NOT also the inconsistent "Pre-season"
          header that the prior `[]` fallback caused. */}
      <Show
        when={standingsError()}
        fallback={
          <Show
            when={!standings.loading && standings() !== null}
            fallback={
              <div class="fw-panel p-4 text-sm text-ink-mute dark:text-paper-subtle">
                Loading standings…
              </div>
            }
          >
            <DataTable
              columns={leagueColumns}
              data={standings() ?? []}
              emptyMessage="No standings yet — click Advance Week to play the first match-day."
            />
          </Show>
        }
      >
        {(err) => (
          <div
            class="fw-panel p-4 text-sm font-mono text-flag-red"
            role="alert"
          >
            Failed to load standings: {describeError(err())}
          </div>
        )}
      </Show>
    </div>
  );
}

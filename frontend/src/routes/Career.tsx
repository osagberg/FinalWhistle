/*
 * Career page — T3-9.
 *
 * Text-first per DESIGN_DOC.md §3. Three panels:
 *   1. Season header — current season number.
 *   2. Champion history — per-season champion list (oldest-to-newest).
 *   3. From past seasons — cross-season memory-event callbacks.
 *
 * One action button: "Advance to next season" → calls `advance_season`,
 * refetches overview, renders a typed outcome line.
 *
 * IPC (read-only via career.ts wrappers):
 *   getCareerOverview()  → CareerOverview         — loaded on mount
 *   advanceSeason()      → AdvanceSeasonSummary   — action button
 *
 * Rules compliance:
 *   - No `any` (Frontend/RULES.md §6)
 *   - Solid signals / createResource + createSignal, not React hooks
 *     (Frontend/RULES.md §1)
 *   - IpcError exhaustive switch + never discriminant (mirrors League.tsx pattern)
 *   - Dark-mode tokens on every color-bearing class (Frontend/RULES.md §2)
 *   - Keyboard-accessible button with visible text label (Frontend/RULES.md §8)
 *   - UI never drives canonical state — button enqueues via IPC only
 */

import { createResource, createSignal, For, Show, type JSX } from "solid-js";
import { advanceSeason, getCareerOverview } from "~/lib/api/career";
import ErrorBoundary from "~/components/ErrorBoundary";
import Loading from "~/components/Loading";
import { IpcShapeError } from "~/lib/runtime-validators";
import { describeRouteError } from "~/lib/route-errors";
import type {
  AdvanceSeasonSummary,
  CareerOverview,
  IpcError,
} from "~/lib/types";

// ---------------------------------------------------------------------------
// Action outcome — carries both success-with-summary and failure-with-error
// so the user always gets visible feedback (mirrors League.tsx ActionOutcome).
// ---------------------------------------------------------------------------

type ActionOutcome =
  | { ok: true; summary: AdvanceSeasonSummary }
  | { ok: false; error: IpcError | Error };

// ---------------------------------------------------------------------------
// IpcError type guard + exhaustiveness helper
//
// Self-contained per project convention — mirrors Squad.tsx / League.tsx /
// Player.tsx. The `satisfies` annotation pins KNOWN_IPC_ERROR_KINDS to
// IpcError["kind"] so adding a new variant forces a compile error here.
// ---------------------------------------------------------------------------

const KNOWN_IPC_ERROR_KINDS = new Set([
  "tooManyFrames",
  "invalidSeed",
  "matchInitFailed",
  "seasonComplete",
  "clubNotFound",
  "lockPoisoned",
  "playerNotFound",
  "seasonNotComplete",
  // T4-5a: live-match command variant.
  "liveMatchCommandUnimplemented",
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
 * Describe any thrown value as a single-line, football-native display string
 * for the action-button outcome aria-live row.
 *
 * Routes through `describeRouteError` (the T4-4 shared helper) — that
 * function holds the exhaustive `IpcError` switch + the narrative-director
 * voice + the no-raw-`err.message`-leak guarantee. The local
 * `formatIpcError`/IpcShapeError-specific path was deleted at T4-4 self-
 * review fix-pass; the generic fallback now covers IpcShapeError and any
 * other non-IpcError throw with football-native copy.
 */
function describeError(e: unknown): string {
  const copy = describeRouteError(e, { what: "the action" });
  return `${copy.headline}. ${copy.detail}`;
}

/** Normalise any thrown value into the structured shape ActionOutcome wants. */
function normaliseError(e: unknown): IpcError | Error {
  if (isIpcError(e)) return e;
  if (e instanceof Error) return e;
  return new Error(String(e));
}

/** Render an ActionOutcome to a user-facing line. */
function describeActionOutcome(outcome: ActionOutcome): string {
  if (outcome.ok) {
    const s = outcome.summary;
    return `Season ${s.completedSeason} complete — ${s.championClubName} are champions. Season ${s.newSeasonNumber} has begun.`;
  }
  return describeError(outcome.error);
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export default function Career(): JSX.Element {
  return (
    <ErrorBoundary label="Career">
      <CareerInner />
    </ErrorBoundary>
  );
}

function CareerInner(): JSX.Element {
  // Fetch error — typed signal so it's fully reactive in the jsdom test env.
  const [overviewError, setOverviewError] = createSignal<IpcError | Error | null>(
    null,
  );

  // Resource: drives getCareerOverview() on mount; refetched after each action.
  const [overview, { refetch }] = createResource<CareerOverview | null>(
    async () => {
      setOverviewError(null);
      try {
        return await getCareerOverview();
      } catch (e: unknown) {
        if (e instanceof IpcShapeError) {
          // eslint-disable-next-line no-console
          console.error(
            "[Career] get_career_overview DTO contract drift:",
            e.command,
            e.reason,
            e.payloadPreview,
          );
        } else {
          // eslint-disable-next-line no-console
          console.error("[Career] getCareerOverview failed:", e);
        }
        setOverviewError(normaliseError(e));
        return null;
      }
    },
  );

  // Action state.
  const [actionPending, setActionPending] = createSignal(false);
  const [lastOutcome, setLastOutcome] = createSignal<ActionOutcome | null>(null);

  const runAdvanceSeason = async (): Promise<void> => {
    setActionPending(true);
    setLastOutcome(null);
    try {
      const summary = await advanceSeason();
      await refetch();
      setLastOutcome({ ok: true, summary });
    } catch (e: unknown) {
      setLastOutcome({ ok: false, error: normaliseError(e) });
    } finally {
      setActionPending(false);
    }
  };

  return (
    <div class="space-y-4">
      {/* Header */}
      <header>
        <h1 class="font-display text-3xl text-pitch-600 dark:text-pitch-300">
          Career
        </h1>
        <Show when={!overviewError()}>
          <Show when={overview()}>
            {(ov) => (
              <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
                Season {ov().seasonNumber}
              </p>
            )}
          </Show>
        </Show>
      </header>

      {/* Action button */}
      <div>
        <button
          type="button"
          class="px-4 py-1.5 rounded text-sm font-mono bg-pitch-500 text-white hover:bg-pitch-600 disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus:ring-2 focus:ring-pitch-400"
          onClick={() => void runAdvanceSeason()}
          disabled={actionPending()}
        >
          {actionPending() ? "Working…" : "Advance to next season"}
        </button>
      </div>

      {/* Action outcome — displayed inline below button. */}
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

      {/* Loading state first — no white-flash before spinner. */}
      <Show when={overview.loading}>
        <Loading message="Loading career overview…" />
      </Show>

      {/* Overview panels: error / data states (only when not loading). */}
      <Show when={!overview.loading}>
        <Show
          when={overviewError()}
          fallback={
            <Show when={overview() !== null}>
              <Show when={overview()}>
              {(ov) => (
                <div class="space-y-4">
                  {/* Panel 1: Champion history */}
                  <section
                    aria-label="Season champion history"
                    class="fw-panel space-y-2 p-4"
                  >
                    <h2 class="text-lg font-semibold text-ink dark:text-paper">
                      Season champions
                    </h2>
                    <Show
                      when={ov().history.length > 0}
                      fallback={
                        <p class="text-sm text-ink-mute dark:text-paper-subtle">
                          No seasons completed yet — advance a season to start building champion history.
                        </p>
                      }
                    >
                      <ol
                        class="space-y-1"
                        aria-label="Per-season champion list"
                      >
                        <For each={ov().history}>
                          {(entry) => (
                            <li class="flex gap-4 text-sm">
                              <span class="w-20 shrink-0 font-mono text-ink-mute dark:text-paper-subtle">
                                Season {entry.season}
                              </span>
                              <span class="text-ink dark:text-paper">
                                {entry.championClubName}
                              </span>
                            </li>
                          )}
                        </For>
                      </ol>
                    </Show>
                  </section>

                  {/* Panel 2: Cross-season callbacks */}
                  <section
                    aria-label="From past seasons"
                    class="fw-panel space-y-2 p-4"
                  >
                    <h2 class="text-lg font-semibold text-ink dark:text-paper">
                      From past seasons
                    </h2>
                    <Show
                      when={ov().crossSeasonCallbacks.length > 0}
                      fallback={
                        <p class="text-sm text-ink-mute dark:text-paper-subtle">
                          No past-season moments yet — advance a season to build career history.
                        </p>
                      }
                    >
                      <ul
                        class="space-y-1"
                        aria-label="Cross-season memory callbacks"
                      >
                        <For each={ov().crossSeasonCallbacks}>
                          {(cb) => (
                            <li class="text-sm text-ink dark:text-paper">
                              {cb}
                            </li>
                          )}
                        </For>
                      </ul>
                    </Show>
                  </section>
                </div>
              )}
            </Show>
          </Show>
        }
        >
          {(err) => {
            const copy = describeRouteError(err(), { what: "the career overview" });
            return (
              <div
                class="fw-panel p-4 text-sm text-rose-600 dark:text-rose-400"
                role="alert"
              >
                <p class="font-semibold">{copy.headline}</p>
                <p class="mt-1 text-ink-subtle dark:text-paper-subtle">{copy.detail}</p>
              </div>
            );
          }}
        </Show>
      </Show>
    </div>
  );
}

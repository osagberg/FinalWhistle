/*
 * Career page — T3-9, extended T4-2.5k.
 *
 * Text-first per DESIGN_DOC.md §3. Four panels:
 *   1. Season header — current season number.
 *   2. Champion history — per-season champion list (oldest-to-newest).
 *   3. From past seasons — cross-season memory-event callbacks.
 *   4. Press inbox — press items from the memory ledger (T4-2.5k).
 *
 * One action button: "Advance to next season" → calls `advance_season`,
 * refetches overview, renders a typed outcome line.
 *
 * IPC (read-only via career.ts wrappers):
 *   getCareerOverview()  → CareerOverview         — loaded on mount
 *   getPressInbox()      → PressInboxDto           — loaded on mount (isolated)
 *   advanceSeason()      → AdvanceSeasonSummary   — action button
 *
 * Press inbox is an ISOLATED resource (mirrors T4-F4 Player.tsx scout pattern):
 * a press-inbox failure never breaks the champion-history / callback panels.
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
import { advanceSeason, getCareerOverview, getPressInbox } from "~/lib/api/career";
import ErrorBoundary from "~/components/ErrorBoundary";
import Loading from "~/components/Loading";
import { IpcShapeError } from "~/lib/runtime-validators";
import { describeRouteError } from "~/lib/route-errors";
import type {
  AdvanceSeasonSummary,
  CareerOverview,
  IpcError,
  PressInboxDto,
  PressItemDto,
  PressTopicDto,
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
  // T4-6a: settings variant.
  "settingsLoadFailed",
  // T4-F4: scouting variants.
  "notYetObserved",
  "leagueGenerationFailed",
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

// ---------------------------------------------------------------------------
// Topic → human-readable label mapping
//
// Maps the closed `PressTopicDto` union to football-native display labels.
// Exhaustiveness is enforced by the switch default never-branch.
// ---------------------------------------------------------------------------

function pressTopicLabel(topic: PressTopicDto): string {
  switch (topic) {
    case "playerMilestone": return "Milestone";
    case "matchResult":     return "Result";
    case "contractTransfer": return "Transfer";
    case "relational":      return "Story";
    default:
      // Exhaustiveness check: if a new topic variant is added to PressTopicDto
      // without updating this switch, the assignment to `never` fails at compile time.
      return ((_: never) => "Press")(topic);
  }
}

// ---------------------------------------------------------------------------
// Press inbox outcome discriminant
//
// Isolated from the overview resource so a press failure never blocks the
// champion-history or callbacks panels (mirrors T4-F4 ScoutOutcome pattern).
// ---------------------------------------------------------------------------

type PressOutcome =
  | { kind: "ok"; inbox: PressInboxDto }
  | { kind: "error"; err: IpcError | Error };

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

  // Press inbox — isolated resource, never blocks the overview panels.
  const [pressOutcome, setPressOutcome] = createSignal<PressOutcome | null>(null);

  const [_pressResource] = createResource<null>(async () => {
    setPressOutcome(null);
    try {
      const inbox = await getPressInbox();
      setPressOutcome({ kind: "ok", inbox });
    } catch (e: unknown) {
      if (e instanceof IpcShapeError) {
        // eslint-disable-next-line no-console
        console.error(
          "[Career] get_press_inbox DTO contract drift:",
          e.command,
          e.reason,
          e.payloadPreview,
        );
      } else {
        // eslint-disable-next-line no-console
        console.error("[Career] getPressInbox failed:", e);
      }
      setPressOutcome({ kind: "error", err: normaliseError(e) });
    }
    return null;
  });

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

      {/* Panel 3: Press inbox — isolated from overview resource */}
      <PressInboxSection pressOutcome={pressOutcome()} />
    </div>
  );
}

// ---------------------------------------------------------------------------
// PressInboxSection — isolated panel for the press inbox
//
// Receives a `PressOutcome | null` — null means the resource is still loading.
// A loading state renders nothing (no jank on first paint — the overview
// panels render independently). Errors render a modest inline note, not a
// full-page alert, so the rest of Career stays usable.
// ---------------------------------------------------------------------------

interface PressInboxSectionProps {
  pressOutcome: PressOutcome | null;
}

function PressInboxSection(props: PressInboxSectionProps): JSX.Element {
  return (
    <>
      {/* Error state — modest note, not a page-level alert */}
      <Show when={props.pressOutcome?.kind === "error"}>
        <section
          aria-label="Press inbox"
          class="fw-panel space-y-2 p-4"
        >
          <h2 class="text-lg font-semibold text-ink dark:text-paper">
            Press inbox
          </h2>
          <p class="text-sm text-ink-mute dark:text-paper-subtle">
            The press inbox couldn't be loaded right now.
          </p>
        </section>
      </Show>

      {/* Data state */}
      <Show
        when={
          props.pressOutcome?.kind === "ok"
            ? (props.pressOutcome as { kind: "ok"; inbox: PressInboxDto }).inbox
            : null
        }
      >
        {(inbox) => (
          <section
            aria-label="Press inbox"
            class="fw-panel space-y-3 p-4"
          >
            <h2 class="text-lg font-semibold text-ink dark:text-paper">
              Press inbox
            </h2>

            <Show
              when={inbox().items.length > 0}
              fallback={
                <p class="text-sm text-ink-mute dark:text-paper-subtle">
                  No press yet — play a season to make headlines.
                </p>
              }
            >
              <ul
                class="space-y-3"
                aria-label="Press items"
              >
                <For each={inbox().items}>
                  {(item: PressItemDto) => (
                    <li class="border-b border-ink-subtle/10 pb-3 last:border-b-0 last:pb-0 dark:border-paper-subtle/10">
                      {/* Topic label + season metadata row */}
                      <div class="mb-1 flex items-center gap-2 text-xs text-ink-mute dark:text-paper-subtle">
                        <span
                          class="rounded bg-pitch-100 px-1.5 py-0.5 font-mono text-pitch-700 dark:bg-pitch-900 dark:text-pitch-300"
                          aria-label={`Topic: ${pressTopicLabel(item.topic)}`}
                        >
                          {pressTopicLabel(item.topic)}
                        </span>
                        <span>Season {item.season}</span>
                      </div>

                      {/* Headline */}
                      <p class="text-sm text-ink dark:text-paper">
                        {item.headline}
                      </p>

                      {/* Manager quote — only when non-null */}
                      <Show when={item.managerQuote !== null}>
                        <p class="mt-1 text-sm italic text-ink-subtle dark:text-paper-subtle">
                          "{item.managerQuote}"
                        </p>
                      </Show>
                    </li>
                  )}
                </For>
              </ul>
            </Show>
          </section>
        )}
      </Show>
    </>
  );
}

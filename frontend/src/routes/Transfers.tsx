/*
 * Transfers page — T2-8.
 *
 * UI shell stub. Surfaces the current transfer-window state
 * (summer / winter / closed) derived from the current match-day via the
 * existing `getStandings()` IPC, and tells the player that transfer
 * mechanics themselves land at Phase T3.
 *
 * Window-state mapping is a PURE function in `~/lib/transfer-window.ts`
 * (deterministic + boundary-tested). This component is the IPC consumer +
 * render shell only.
 *
 * Rules compliance:
 *   - SolidJS only (createResource, Show) — no React patterns
 *   - No `any` (TS strict)
 *   - Dark + light mode tokens on every color-bearing class
 *   - Football-native vocabulary; no banned terms
 */

import { createResource, Show, type JSX } from "solid-js";
import ErrorBoundary from "~/components/ErrorBoundary";
import { getStandings } from "~/lib/api/season";
import {
  computeTransferWindowState,
  type WindowState,
} from "~/lib/transfer-window";
import type { StandingsRow } from "~/lib/types";

/**
 * Tailwind class for the window-state pill. Distinct colors for open vs
 * closed so the at-a-glance read is unambiguous. Each color-bearing class
 * has a `dark:` companion per Frontend/RULES.md §2.
 */
function windowPillClass(state: WindowState): string {
  if (state.kind === "closed") {
    return "fw-pill bg-paper-bold text-ink-mute dark:bg-midnight-line dark:text-paper-subtle";
  }
  // Both open windows (summer + winter) get the pitch-green accent.
  return "fw-pill bg-pitch-500 text-white dark:bg-pitch-600";
}

/**
 * Derive current match-day from a standings array. All clubs play once per
 * match-day in the atomic advance_week model, so `played` on the first row
 * is canonical.
 *
 * Post-T2-8 silent-failure-hunter P1 fix: returns `null` for empty / missing
 * inputs (distinct from `0` which means "pre-season, zero fixtures played").
 * Conflating "no data" with "pre-season" was the silent-failure surface —
 * an unexpected empty standings array (future regression, missing season,
 * etc.) would have rendered a confident "Summer window" pill. Returning
 * `null` forces the consumer to render a distinct "unavailable" state.
 */
function currentMatchDayFromStandings(
  standings: StandingsRow[] | null | undefined,
): number | null {
  if (!standings || standings.length === 0) return null;
  return standings[0]!.played;
}

/** Friendly text for any thrown standings-fetch failure.
 *
 * SolidJS's createResource wraps non-Error rejections via
 * `castError(err) = new Error("Unknown error", { cause: err })`, so the
 * original IpcError lives in `.cause`. We try `cause.kind` first (the
 * IpcError discriminator), then fall back to `e.kind` for the unwrapped
 * case, then `e.message`, then `String(e)`. This keeps the user-visible
 * text informative regardless of how the error was thrown. */
function describeStandingsError(e: unknown): string {
  // Solid-wrapped IpcError: look inside .cause for the kind discriminator.
  if (e instanceof Error && typeof e.cause === "object" && e.cause !== null && "kind" in e.cause) {
    const kind = (e.cause as Record<string, unknown>).kind;
    if (typeof kind === "string") return `IPC error (${kind})`;
  }
  // Raw IpcError-shaped object (unwrapped path; defensive).
  if (typeof e === "object" && e !== null && "kind" in e) {
    const kind = (e as Record<string, unknown>).kind;
    if (typeof kind === "string") return `IPC error (${kind})`;
  }
  if (e instanceof Error) return e.message;
  return String(e);
}

export default function Transfers(): JSX.Element {
  return (
    <ErrorBoundary label="Transfers">
      <TransfersInner />
    </ErrorBoundary>
  );
}

function TransfersInner(): JSX.Element {
  // Post-T2-8 silent-failure-hunter P1 fix: NO try/catch around getStandings.
  // The prior shape laundered every backend failure into `[]` → "Summer
  // window — open" — a confidently-wrong UI that hid lockPoisoned / IPC
  // failures / serde mishaps behind a friendly pill. Let createResource
  // surface errors via `standings.error`; the render branches gate on
  // `.loading` / `.error` / data presence explicitly.
  const [standings] = createResource<StandingsRow[]>(getStandings);

  return (
    <div class="space-y-4">
      <header>
        <h1 class="font-display text-3xl text-pitch-600 dark:text-pitch-300">
          Transfers
        </h1>
        <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
          Window state tracks the current match-day; transfer mechanics
          are coming soon.
        </p>
      </header>

      <section class="fw-panel p-4 space-y-2">
        <h2 class="font-display text-lg text-ink-bold dark:text-paper-bold">
          Window state
        </h2>
        <Show
          when={!standings.loading}
          fallback={
            <span class="text-sm text-ink-mute dark:text-paper-subtle italic">
              Loading…
            </span>
          }
        >
          <Show
            when={!standings.error}
            fallback={
              <span
                class="text-sm text-flag-red font-mono"
                role="alert"
              >
                Couldn't read season state — window status unavailable
                ({describeStandingsError(standings.error)}).
              </span>
            }
          >
            {/* Solid's `<Show when={0}>` would treat 0 as falsy, so we wrap
                the match-day in `{day: number}` — an object is always truthy
                — and unwrap inside. This preserves the valid-pre-season
                case (`played: 0`) which would otherwise render the
                fallback. */}
            <Show
              when={(() => {
                const md = currentMatchDayFromStandings(standings());
                return md !== null ? { day: md } : null;
              })()}
              fallback={
                <span
                  class="text-sm text-ink-mute dark:text-paper-subtle italic"
                  role="status"
                >
                  Season not loaded — no standings available yet.
                </span>
              }
            >
              {(wrapped) => {
                const state = computeTransferWindowState(wrapped().day);
                return (
                  <div class="flex items-center gap-3">
                    <span class={windowPillClass(state)}>{state.label}</span>
                    <span class="text-xs text-ink-subtle dark:text-paper-subtle font-mono">
                      Match-day {wrapped().day}
                    </span>
                  </div>
                );
              }}
            </Show>
          </Show>
        </Show>
      </section>

      <section class="fw-panel p-4 text-sm text-ink-subtle dark:text-paper-subtle">
        <p>
          Transfer mechanics are coming — bids, negotiations, and contracts.
          The window-state pill above will become interactive once those
          arrive.
        </p>
      </section>
    </div>
  );
}

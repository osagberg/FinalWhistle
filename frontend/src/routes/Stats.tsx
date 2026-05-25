/*
 * Stats page — T4-2.
 *
 * Per-team analytics dashboard: three ECharts charts bound to two IPC commands.
 *   1. Ranked horizontal bar — one bar per club, sorted by a selectable stat.
 *   2. GF-vs-GA scatter — one point per club with club-name tooltip.
 *   3. Cumulative-points trend — line chart for a user-selected club.
 *
 * IPC (read-only via season.ts wrappers):
 *   getStandings()          → StandingsRow[]         — ranked bar + scatter
 *   getFixtures(clubId)     → FixtureWithResult[]    — cumulative-points trend
 *
 * Rules compliance:
 *   - No `any` (Frontend/RULES.md §6)
 *   - Solid signals / createResource, not React hooks (Frontend/RULES.md §1)
 *   - Dark-mode tokens on every color-bearing class (Frontend/RULES.md §2)
 *   - Keyboard-accessible controls with labels (Frontend/RULES.md §8)
 *   - UI never drives canonical state — read-only IPC only
 */

import {
  createMemo,
  createResource,
  createSignal,
  For,
  Show,
  type JSX,
} from "solid-js";
import Stat from "~/components/Stat";
import Loading from "~/components/Loading";
import ErrorBoundary from "~/components/ErrorBoundary";
import { getStandings, getFixtures } from "~/lib/api/season";
import { describeRouteError, isIpcError } from "~/lib/route-errors";
import {
  buildRankedBarOption,
  buildScatterOption,
  buildTrendOption,
  type StatKey,
} from "~/lib/stats-charts";
import type { IpcError, StandingsRow } from "~/lib/types";

// Normalize any thrown value into the IpcError | Error union the error
// signals carry. `unknown` would lie about what the signal contains; this
// guarantees describeRouteError gets a typed input and matches the pattern
// used by Squad / League / Career / Player.
function normaliseError(e: unknown): IpcError | Error {
  if (isIpcError(e)) return e;
  if (e instanceof Error) return e;
  return new Error(String(e));
}

// ---------------------------------------------------------------------------
// Stat selector options
// ---------------------------------------------------------------------------

const STAT_OPTS: Array<{ key: StatKey; label: string }> = [
  { key: "points", label: "Points" },
  { key: "goalDifference", label: "Goal difference" },
  { key: "goalsFor", label: "Goals for" },
  { key: "goalsAgainst", label: "Goals against" },
  { key: "wins", label: "Wins" },
  { key: "draws", label: "Draws" },
  { key: "losses", label: "Losses" },
  { key: "played", label: "Played" },
];

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export default function Stats(): JSX.Element {
  return (
    <ErrorBoundary label="Stats">
      <StatsInner />
    </ErrorBoundary>
  );
}

function StatsInner(): JSX.Element {
  // Standings resource: drives ranked bar + scatter.
  const [standingsError, setStandingsError] = createSignal<IpcError | Error | null>(null);

  const [standings] = createResource<StandingsRow[] | null>(async () => {
    setStandingsError(null);
    try {
      return await getStandings();
    } catch (e: unknown) {
      // eslint-disable-next-line no-console
      console.error("[Stats] getStandings failed:", e);
      // Normalise to the IpcError | Error union the signal carries; preserves
      // the IpcError shape for describeRouteError + the Error class for
      // any non-IPC throw.
      setStandingsError(normaliseError(e));
      return null;
    }
  });

  // Selected stat for the ranked bar chart.
  const [selectedStat, setSelectedStat] = createSignal<StatKey>("points");

  // Selected club for the trend chart.
  // Defaults to the first club in standings once loaded.
  const [selectedClubId, setSelectedClubId] = createSignal<number | null>(null);

  // Reactive: when standings arrive and no club is selected, pick the first.
  const effectiveClubId = createMemo<number | null>(() => {
    const rows = standings();
    if (!rows || rows.length === 0) return null;
    const chosen = selectedClubId();
    if (chosen !== null) return chosen;
    return rows[0]!.clubId;
  });

  // Trend resource: re-fetches when the effective club changes.
  const [fixturesError, setFixturesError] = createSignal<IpcError | Error | null>(null);

  const [fixtures] = createResource(effectiveClubId, async (clubId) => {
    if (clubId === null) return null;
    setFixturesError(null);
    try {
      const fx = await getFixtures(clubId);
      // Contract guard: a played fixture must carry both scores. The Rust DTO
      // guarantees this (only unplayed fixtures skip the score fields), but a
      // played fixture with a missing score would otherwise be silently
      // counted as a 0-0 draw by cumulativePoints. Fail loud at the IPC seam.
      for (const f of fx) {
        if (f.played && (f.homeScore == null || f.awayScore == null)) {
          throw new Error(
            `played fixture (match-day ${f.matchDay}) is missing a score`,
          );
        }
      }
      return fx;
    } catch (e: unknown) {
      // eslint-disable-next-line no-console
      console.error("[Stats] getFixtures failed:", e);
      // Normalise to the IpcError | Error union the signal carries.
      setFixturesError(normaliseError(e));
      return null;
    }
  });

  // Derived: effective club name (for chart title).
  const effectiveClubName = createMemo<string>(() => {
    const rows = standings();
    const id = effectiveClubId();
    if (!rows || id === null) return "";
    return rows.find((r) => r.clubId === id)?.clubName ?? "";
  });

  // Chart options — recomputed reactively when inputs change.
  const barOption = createMemo(() => {
    const rows = standings();
    if (!rows || rows.length === 0) return null;
    return buildRankedBarOption(rows, selectedStat());
  });

  const scatterOption = createMemo(() => {
    const rows = standings();
    if (!rows || rows.length === 0) return null;
    return buildScatterOption(rows);
  });

  const trendOption = createMemo(() => {
    const fx = fixtures();
    // Gate on played-fixture count so the trend chart and the "No matches
    // played" empty state are mutually exclusive — an empty chart frame
    // sitting above the notice is a not-clean empty state (AC5).
    if (!fx || fx.filter((f) => f.played).length === 0) return null;
    return buildTrendOption(fx, effectiveClubName());
  });

  return (
    <div class="space-y-6">
      <header>
        <h1 class="font-display text-3xl text-pitch-600 dark:text-pitch-300">
          Stats
        </h1>
        <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
          League-wide analytics for the current season.
        </p>
      </header>

      {/* Standings error */}
      <Show when={standingsError()}>
        {(err) => {
          const copy = describeRouteError(err(), { what: "the standings" });
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

      {/* Loading */}
      <Show when={standings.loading}>
        <Loading message="Loading stats…" />
      </Show>

      {/* Empty state */}
      <Show
        when={
          !standings.loading &&
          standings() !== null &&
          (standings()?.length ?? 0) === 0 &&
          !standingsError()
        }
      >
        <div class="fw-panel p-6 text-sm text-ink-mute dark:text-paper-subtle text-center">
          No standings yet — advance at least one match-day to see stats.
        </div>
      </Show>

      {/* Charts — only render when standings are available */}
      <Show when={standings() && (standings()?.length ?? 0) > 0 && !standingsError()}>
        {/* Ranked bar chart */}
        <section class="space-y-3">
          <div class="flex flex-wrap items-center gap-3">
            <h2 class="text-base font-display text-ink dark:text-paper">
              Clubs ranked by
            </h2>
            {/* Stat selector */}
            <div class="flex items-center gap-2">
              <label
                for="stat-select"
                class="text-sm text-ink-mute dark:text-paper-subtle sr-only"
              >
                Select stat
              </label>
              <select
                id="stat-select"
                class="rounded border border-ink-mute/30 dark:border-midnight-line bg-paper dark:bg-midnight-panel text-ink dark:text-paper text-sm px-2 py-1 focus:outline-none focus:ring-2 focus:ring-pitch-400"
                value={selectedStat()}
                onChange={(e) => setSelectedStat(e.currentTarget.value as StatKey)}
                aria-label="Select stat for ranked chart"
              >
                <For each={STAT_OPTS}>
                  {(opt) => <option value={opt.key}>{opt.label}</option>}
                </For>
              </select>
            </div>
          </div>
          <Show when={barOption()}>
            {(opt) => <Stat option={opt()} height={320} />}
          </Show>
        </section>

        {/* GF vs GA scatter */}
        <section class="space-y-2">
          <h2 class="text-base font-display text-ink dark:text-paper">
            Goals for vs Goals against
          </h2>
          <Show when={scatterOption()}>
            {(opt) => <Stat option={opt()} height={280} />}
          </Show>
        </section>

        {/* Per-club cumulative points trend */}
        <section class="space-y-3">
          <div class="flex flex-wrap items-center gap-3">
            <h2 class="text-base font-display text-ink dark:text-paper">
              Points over the season
            </h2>
            <div class="flex items-center gap-2">
              <label
                for="club-select"
                class="text-sm text-ink-mute dark:text-paper-subtle sr-only"
              >
                Select club
              </label>
              <select
                id="club-select"
                class="rounded border border-ink-mute/30 dark:border-midnight-line bg-paper dark:bg-midnight-panel text-ink dark:text-paper text-sm px-2 py-1 focus:outline-none focus:ring-2 focus:ring-pitch-400"
                value={effectiveClubId() ?? ""}
                onChange={(e) =>
                  setSelectedClubId(Number(e.currentTarget.value))
                }
                aria-label="Select club for points trend"
              >
                <For each={standings() ?? []}>
                  {(row) => (
                    <option value={row.clubId}>{row.clubName}</option>
                  )}
                </For>
              </select>
            </div>
          </div>

          {/* Fixtures error */}
          <Show when={fixturesError()}>
            {(err) => {
              const copy = describeRouteError(err(), { what: "the fixtures" });
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

          <Show when={fixtures.loading}>
            <Loading message="Loading fixtures…" />
          </Show>

          <Show when={trendOption()}>
            {(opt) => <Stat option={opt()} height={260} />}
          </Show>

          {/* Trend empty state: fixtures loaded but none played yet */}
          <Show
            when={
              !fixtures.loading &&
              fixtures() !== null &&
              (fixtures()?.filter((f) => f.played).length ?? 0) === 0 &&
              !fixturesError()
            }
          >
            <div class="fw-panel p-4 text-sm text-ink-mute dark:text-paper-subtle">
              No matches played yet for {effectiveClubName()}.
            </div>
          </Show>
        </section>
      </Show>
    </div>
  );
}

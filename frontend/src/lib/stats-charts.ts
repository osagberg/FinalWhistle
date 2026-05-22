/*
 * Pure chart-option builders for the /stats route (T4-2).
 *
 * No SolidJS. No IPC. All functions are deterministic, side-effect-free,
 * and unit-testable in isolation.
 *
 * Three charts:
 *   buildRankedBarOption  — horizontal bar, one bar per club, sorted by a selectable stat.
 *   buildScatterOption    — GF vs GA scatter, one point per club.
 *   buildTrendOption      — cumulative-points line over played fixtures for one club.
 */

import type { EChartsCoreOption } from "echarts/core";
import type { StandingsRow, FixtureWithResult } from "./types";

// ---------------------------------------------------------------------------
// StatKey — the set of columns selectable in the ranked bar chart
// ---------------------------------------------------------------------------

export type StatKey =
  | "points"
  | "goalDifference"
  | "goalsFor"
  | "goalsAgainst"
  | "wins"
  | "draws"
  | "losses"
  | "played";

// Human-readable axis label for each stat key.
const STAT_LABEL: Record<StatKey, string> = {
  points: "Points",
  goalDifference: "Goal difference",
  goalsFor: "Goals for",
  goalsAgainst: "Goals against",
  wins: "Wins",
  draws: "Draws",
  losses: "Losses",
  played: "Played",
};

// ---------------------------------------------------------------------------
// Ranked horizontal bar chart
// ---------------------------------------------------------------------------

/**
 * Build a horizontal bar chart option sorted descending by `stat`.
 *
 * Categories are ordered so the highest value is at the top. ECharts'
 * yAxis for a horizontal bar places the last category at the top by
 * default, so we sort ascending (lowest first) and let ECharts render
 * them bottom-to-top — giving the highest bar at the top visually.
 */
export function buildRankedBarOption(
  rows: StandingsRow[],
  stat: StatKey,
): EChartsCoreOption {
  // Sort ascending so ECharts renders the highest value at the top.
  const sorted = [...rows].sort((a, b) => a[stat] - b[stat]);
  const categories = sorted.map((r) => r.clubName);
  const values = sorted.map((r) => r[stat]);

  return {
    tooltip: {
      trigger: "axis",
      axisPointer: { type: "shadow" },
    },
    grid: { left: 120, right: 24, top: 16, bottom: 32, containLabel: false },
    xAxis: {
      type: "value",
      name: STAT_LABEL[stat],
      nameLocation: "middle",
      nameGap: 24,
    },
    yAxis: {
      type: "category",
      data: categories,
      axisLabel: { fontSize: 11 },
    },
    series: [
      {
        type: "bar",
        data: values,
        itemStyle: { color: "#4d7c0f" },
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// GF vs GA scatter chart
// ---------------------------------------------------------------------------

/**
 * Build a scatter chart plotting goals-for on the x-axis and
 * goals-against on the y-axis. Each point is one club; the club name
 * appears in the tooltip.
 */
export function buildScatterOption(rows: StandingsRow[]): EChartsCoreOption {
  const data = rows.map((r) => ({
    value: [r.goalsFor, r.goalsAgainst] as [number, number],
    name: r.clubName,
  }));

  return {
    tooltip: {
      trigger: "item",
      formatter: (params: unknown) => {
        const p = params as { name: string; value: [number, number] };
        return `${p.name}<br/>GF: ${p.value[0]}, GA: ${p.value[1]}`;
      },
    },
    grid: { left: 48, right: 24, top: 24, bottom: 40 },
    xAxis: {
      type: "value",
      name: "Goals for",
      nameLocation: "middle",
      nameGap: 28,
    },
    yAxis: {
      type: "value",
      name: "Goals against",
      nameLocation: "middle",
      nameGap: 40,
    },
    series: [
      {
        type: "scatter",
        data,
        symbolSize: 10,
        itemStyle: { color: "#4d7c0f" },
      },
    ],
  };
}

// ---------------------------------------------------------------------------
// Cumulative-points trend line
// ---------------------------------------------------------------------------

/**
 * Compute cumulative points from a list of fixtures for one club.
 *
 * Only played fixtures are counted; unplayed fixtures (where `played` is
 * false) are skipped. Returns a parallel array of cumulative point totals,
 * one entry per played fixture in match-day order.
 *
 * Per-fixture points: win = 3, draw = 1, loss = 0.
 * The club's goals are `isHome ? homeScore : awayScore`; opponent goals
 * are the other score field.
 *
 * The returned array is monotonic non-decreasing.
 */
export function cumulativePoints(fixtures: FixtureWithResult[]): number[] {
  const result: number[] = [];
  let total = 0;
  for (const f of fixtures) {
    if (!f.played) continue;
    // `?? 0` satisfies the `number | undefined` field type. A played fixture
    // always carries both scores (Rust DTO contract; the Stats route also
    // guards this at the IPC seam), so the default is unreachable in practice.
    const myGoals = f.isHome ? (f.homeScore ?? 0) : (f.awayScore ?? 0);
    const theirGoals = f.isHome ? (f.awayScore ?? 0) : (f.homeScore ?? 0);
    if (myGoals > theirGoals) total += 3;
    else if (myGoals === theirGoals) total += 1;
    // loss: +0
    result.push(total);
  }
  return result;
}

/**
 * Build a line chart showing cumulative points for `clubName` across
 * the played fixtures (in match-day order, skipping unplayed fixtures).
 */
export function buildTrendOption(
  fixtures: FixtureWithResult[],
  clubName: string,
): EChartsCoreOption {
  const points = cumulativePoints(fixtures);
  // x-axis labels: 1, 2, 3, … up to played fixture count.
  const xLabels = points.map((_, i) => String(i + 1));

  return {
    tooltip: {
      trigger: "axis",
      formatter: (params: unknown) => {
        const arr = params as Array<{ dataIndex: number; value: number }>;
        const p = arr[0];
        if (!p) return "";
        return `Match-day ${p.dataIndex + 1}: ${p.value} pts`;
      },
    },
    grid: { left: 48, right: 24, top: 24, bottom: 40 },
    xAxis: {
      type: "category",
      data: xLabels,
      name: "Fixture",
      nameLocation: "middle",
      nameGap: 28,
    },
    yAxis: {
      type: "value",
      name: "Points",
      nameLocation: "middle",
      nameGap: 40,
      min: 0,
    },
    series: [
      {
        type: "line",
        name: clubName,
        data: points,
        smooth: false,
        itemStyle: { color: "#4d7c0f" },
        lineStyle: { width: 2 },
        symbol: "circle",
        symbolSize: 5,
      },
    ],
  };
}

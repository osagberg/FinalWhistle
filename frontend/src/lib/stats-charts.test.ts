/*
 * Unit tests for stats-charts.ts (T4-2).
 *
 * All functions are pure — no SolidJS, no IPC. Tests exercise:
 *   - buildRankedBarOption: sort order changes with stat selection (non-vacuous
 *     fixture ensures different clubs top for different stats).
 *   - buildScatterOption: series length + per-club data alignment.
 *   - cumulativePoints / buildTrendOption: monotonic non-decreasing, correct
 *     final total, correct length (unplayed fixtures excluded).
 */

import { describe, expect, it } from "vitest";
import {
  buildRankedBarOption,
  buildScatterOption,
  buildTrendOption,
  cumulativePoints,
  type StatKey,
} from "./stats-charts";
import type { StandingsRow, FixtureWithResult } from "./types";

// ---------------------------------------------------------------------------
// Fixture data — designed so "points" and "goalsAgainst" leaders differ
// ---------------------------------------------------------------------------

// Aardvark has the most points; Brindlewood has the most goals against.
const STANDINGS: StandingsRow[] = [
  {
    clubId: 1,
    clubName: "Aardvark FC",
    played: 10,
    wins: 8,
    draws: 1,
    losses: 1,
    goalsFor: 20,
    goalsAgainst: 5,
    goalDifference: 15,
    points: 25,
  },
  {
    clubId: 2,
    clubName: "Brindlewood City",
    played: 10,
    wins: 5,
    draws: 2,
    losses: 3,
    goalsFor: 15,
    goalsAgainst: 18,
    goalDifference: -3,
    points: 17,
  },
  {
    clubId: 3,
    clubName: "Cormorant Athletic",
    played: 10,
    wins: 2,
    draws: 1,
    losses: 7,
    goalsFor: 8,
    goalsAgainst: 12,
    goalDifference: -4,
    points: 7,
  },
];

// ---------------------------------------------------------------------------
// buildRankedBarOption
// ---------------------------------------------------------------------------

describe("buildRankedBarOption", () => {
  it("sorts by points descending: Aardvark appears as top category", () => {
    const opt = buildRankedBarOption(STANDINGS, "points");
    // ECharts renders categories bottom-to-top so ascending sort puts the
    // winner last in the data array (which appears at the top visually).
    const yAxis = opt.yAxis as { data: string[] };
    const categories = yAxis.data;
    expect(categories[categories.length - 1]).toBe("Aardvark FC");
    // Lowest-points club is first in the array (rendered at the bottom).
    expect(categories[0]).toBe("Cormorant Athletic");
  });

  it("sorts by goalsAgainst descending: Brindlewood (18) appears at top", () => {
    const opt = buildRankedBarOption(STANDINGS, "goalsAgainst");
    const yAxis = opt.yAxis as { data: string[] };
    const categories = yAxis.data;
    // Brindlewood has 18 GA (highest), Aardvark has 5 GA (lowest).
    expect(categories[categories.length - 1]).toBe("Brindlewood City");
    expect(categories[0]).toBe("Aardvark FC");
  });

  it("category order differs between 'points' and 'goalsAgainst' (non-vacuous)", () => {
    const byPts = buildRankedBarOption(STANDINGS, "points");
    const byGA = buildRankedBarOption(STANDINGS, "goalsAgainst");
    const ptsCats = (byPts.yAxis as { data: string[] }).data;
    const gaCats = (byGA.yAxis as { data: string[] }).data;
    // The top club differs between the two sorts.
    expect(ptsCats[ptsCats.length - 1]).not.toBe(gaCats[gaCats.length - 1]);
  });

  it("values array aligns with sorted categories", () => {
    const opt = buildRankedBarOption(STANDINGS, "wins");
    const yAxis = opt.yAxis as { data: string[] };
    const series = opt.series as Array<{ data: number[] }>;
    expect(yAxis.data.length).toBe(STANDINGS.length);
    expect(series[0]!.data.length).toBe(STANDINGS.length);
    // Last category (top of visual chart) should have the most wins.
    const lastIdx = yAxis.data.length - 1;
    expect(yAxis.data[lastIdx]).toBe("Aardvark FC"); // 8 wins
    expect(series[0]!.data[lastIdx]).toBe(8);
  });

  it("handles all stat keys without throwing", () => {
    const statKeys: StatKey[] = [
      "points", "goalDifference", "goalsFor", "goalsAgainst",
      "wins", "draws", "losses", "played",
    ];
    for (const key of statKeys) {
      expect(() => buildRankedBarOption(STANDINGS, key)).not.toThrow();
    }
  });
});

// ---------------------------------------------------------------------------
// buildScatterOption
// ---------------------------------------------------------------------------

describe("buildScatterOption", () => {
  it("series data length equals number of rows", () => {
    const opt = buildScatterOption(STANDINGS);
    const series = opt.series as Array<{ data: unknown[] }>;
    expect(series[0]!.data.length).toBe(STANDINGS.length);
  });

  it("a known club's datum deep-equals [goalsFor, goalsAgainst]", () => {
    const opt = buildScatterOption(STANDINGS);
    const series = opt.series as Array<{
      data: Array<{ value: [number, number]; name: string }>;
    }>;
    const aardvark = series[0]!.data.find((d) => d.name === "Aardvark FC");
    expect(aardvark).toBeDefined();
    expect(aardvark!.value).toEqual([20, 5]); // goalsFor=20, goalsAgainst=5
  });

  it("Brindlewood City datum equals [15, 18]", () => {
    const opt = buildScatterOption(STANDINGS);
    const series = opt.series as Array<{
      data: Array<{ value: [number, number]; name: string }>;
    }>;
    const brind = series[0]!.data.find((d) => d.name === "Brindlewood City");
    expect(brind!.value).toEqual([15, 18]);
  });
});

// ---------------------------------------------------------------------------
// cumulativePoints
// ---------------------------------------------------------------------------

describe("cumulativePoints", () => {
  // Fixtures: W (3pts), D (1pt), L (0pts), unplayed (skip).
  // Club is home for first fixture, away for rest.
  const FIXTURES: FixtureWithResult[] = [
    // Home win (2-1): club gets 3pts.
    {
      matchDay: 1, opponentClubId: 2, opponentClubName: "Brindlewood City",
      isHome: true, played: true, homeScore: 2, awayScore: 1,
    },
    // Away draw (1-1): club gets 1pt.
    {
      matchDay: 2, opponentClubId: 3, opponentClubName: "Cormorant Athletic",
      isHome: false, played: true, homeScore: 1, awayScore: 1,
    },
    // Away loss (0-2): club gets 0pts.
    {
      matchDay: 3, opponentClubId: 4, opponentClubName: "Dunby Rovers",
      isHome: false, played: true, homeScore: 2, awayScore: 0,
    },
    // Unplayed — must be skipped.
    {
      matchDay: 4, opponentClubId: 5, opponentClubName: "Elwick Town",
      isHome: true, played: false,
    },
  ];

  it("is monotonic non-decreasing", () => {
    const pts = cumulativePoints(FIXTURES);
    for (let i = 1; i < pts.length; i++) {
      expect(pts[i]!).toBeGreaterThanOrEqual(pts[i - 1]!);
    }
  });

  it("final element equals total points (3 + 1 + 0 = 4)", () => {
    const pts = cumulativePoints(FIXTURES);
    expect(pts[pts.length - 1]).toBe(4);
  });

  it("length equals played-fixture count (3 played, 1 unplayed = length 3)", () => {
    const pts = cumulativePoints(FIXTURES);
    expect(pts.length).toBe(3);
  });

  it("cumulative progression is [3, 4, 4]", () => {
    const pts = cumulativePoints(FIXTURES);
    expect(pts).toEqual([3, 4, 4]);
  });

  it("returns empty array when no fixtures are played", () => {
    const none: FixtureWithResult[] = [
      {
        matchDay: 1, opponentClubId: 2, opponentClubName: "Brindlewood City",
        isHome: true, played: false,
      },
    ];
    expect(cumulativePoints(none)).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// buildTrendOption
// ---------------------------------------------------------------------------

describe("buildTrendOption", () => {
  const PLAYED_FIXTURES: FixtureWithResult[] = [
    { matchDay: 1, opponentClubId: 2, opponentClubName: "B", isHome: true, played: true, homeScore: 3, awayScore: 0 },
    { matchDay: 2, opponentClubId: 3, opponentClubName: "C", isHome: false, played: true, homeScore: 1, awayScore: 1 },
    { matchDay: 3, opponentClubId: 4, opponentClubName: "D", isHome: true, played: false },
  ];

  it("series data length equals played-fixture count (skipping unplayed)", () => {
    const opt = buildTrendOption(PLAYED_FIXTURES, "Aardvark FC");
    const series = opt.series as Array<{ data: number[] }>;
    expect(series[0]!.data.length).toBe(2); // 2 played, 1 unplayed
  });

  it("series data is monotonic non-decreasing", () => {
    const opt = buildTrendOption(PLAYED_FIXTURES, "Aardvark FC");
    const series = opt.series as Array<{ data: number[] }>;
    const data = series[0]!.data;
    for (let i = 1; i < data.length; i++) {
      expect(data[i]!).toBeGreaterThanOrEqual(data[i - 1]!);
    }
  });

  it("final series value equals total points", () => {
    const opt = buildTrendOption(PLAYED_FIXTURES, "Aardvark FC");
    const series = opt.series as Array<{ data: number[] }>;
    const data = series[0]!.data;
    // Win (3pts) + draw (1pt) = 4pts total.
    expect(data[data.length - 1]).toBe(4);
  });
});

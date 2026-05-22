/*
 * Stats page — Vitest tests (T4-2).
 *
 * Mirror League.test.tsx for mock patterns. Tests:
 *   - Route renders a heading.
 *   - getStandings rejecting → error notice (role="alert").
 *   - getStandings resolving [] → empty state.
 */

import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@solidjs/testing-library";
import type { StandingsRow, FixtureWithResult } from "~/lib/types";

// ---------------------------------------------------------------------------
// Module mocks — hoisted before component import
// ---------------------------------------------------------------------------

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

vi.mock("~/lib/api/season", () => ({
  getStandings: vi.fn(),
  advanceWeek: vi.fn(),
  playFixtures: vi.fn(),
  getFixtures: vi.fn(),
}));

// echarts/core mock: avoid real canvas init in jsdom.
vi.mock("echarts/core", () => {
  const fakecharts = {
    use: vi.fn(),
    init: vi.fn(() => ({
      setOption: vi.fn(),
      resize: vi.fn(),
      dispose: vi.fn(),
    })),
  };
  return { ...fakecharts, default: fakecharts };
});
vi.mock("echarts/charts", () => ({ LineChart: {}, BarChart: {}, ScatterChart: {} }));
vi.mock("echarts/components", () => ({
  GridComponent: {},
  TooltipComponent: {},
  LegendComponent: {},
  TitleComponent: {},
}));
vi.mock("echarts/renderers", () => ({ CanvasRenderer: {} }));

// Import AFTER mocks are hoisted.
import Stats from "./Stats";
import { getStandings, getFixtures } from "~/lib/api/season";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const FIXTURE_TWO_ROWS: StandingsRow[] = [
  {
    clubId: 1,
    clubName: "Aardvark FC",
    played: 5,
    wins: 4,
    draws: 1,
    losses: 0,
    goalsFor: 12,
    goalsAgainst: 3,
    goalDifference: 9,
    points: 13,
  },
  {
    clubId: 2,
    clubName: "Brindlewood City",
    played: 5,
    wins: 3,
    draws: 1,
    losses: 1,
    goalsFor: 9,
    goalsAgainst: 5,
    goalDifference: 4,
    points: 10,
  },
];

const FIXTURE_FIXTURES: FixtureWithResult[] = [
  {
    matchDay: 1, opponentClubId: 2, opponentClubName: "Brindlewood City",
    isHome: true, played: true, homeScore: 2, awayScore: 0,
  },
  {
    matchDay: 2, opponentClubId: 3, opponentClubName: "Cormorant Athletic",
    isHome: false, played: false,
  },
];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Stats page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getStandings).mockResolvedValue(FIXTURE_TWO_ROWS);
    vi.mocked(getFixtures).mockResolvedValue(FIXTURE_FIXTURES);
  });

  it("renders a Stats heading", async () => {
    render(() => <Stats />);

    await waitFor(() => {
      expect(screen.getByRole("heading", { name: /stats/i })).toBeInTheDocument();
    });
  });

  it("shows loading state before standings resolve", () => {
    vi.mocked(getStandings).mockImplementation(
      () => new Promise(() => { /* pending */ }),
    );

    render(() => <Stats />);
    expect(screen.getByText(/loading stats/i)).toBeInTheDocument();
  });

  it("shows error notice when getStandings rejects", async () => {
    vi.mocked(getStandings).mockRejectedValue(new Error("Network failure"));

    render(() => <Stats />);

    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });

    const alert = screen.getByRole("alert");
    expect(alert.textContent).toContain("Failed to load standings");
    expect(alert.textContent).toContain("Network failure");
  });

  it("shows empty state when standings resolve to an empty array", async () => {
    vi.mocked(getStandings).mockResolvedValue([]);

    render(() => <Stats />);

    await waitFor(() => {
      expect(screen.getByText(/no standings yet/i)).toBeInTheDocument();
    });
  });

  it("renders stat selector and club selector once standings load", async () => {
    render(() => <Stats />);

    await waitFor(() => {
      expect(screen.getByRole("combobox", { name: /select stat/i })).toBeInTheDocument();
    });

    expect(screen.getByRole("combobox", { name: /select club/i })).toBeInTheDocument();
  });

  it("club selector lists clubs from standings", async () => {
    render(() => <Stats />);

    await waitFor(() => {
      expect(screen.getByRole("option", { name: "Aardvark FC" })).toBeInTheDocument();
    });

    expect(screen.getByRole("option", { name: "Brindlewood City" })).toBeInTheDocument();
  });

  it("shows a fixtures error when a played fixture is missing a score", async () => {
    // Malformed: played === true but no scores. The IPC-seam contract guard
    // must reject this loudly rather than letting cumulativePoints score it 0-0.
    vi.mocked(getFixtures).mockResolvedValue([
      {
        matchDay: 1,
        opponentClubId: 2,
        opponentClubName: "Brindlewood City",
        isHome: true,
        played: true,
      },
    ]);

    render(() => <Stats />);

    await waitFor(() => {
      const alerts = screen.getAllByRole("alert");
      expect(
        alerts.some((a) => a.textContent?.includes("Failed to load fixtures")),
      ).toBe(true);
    });
  });
});

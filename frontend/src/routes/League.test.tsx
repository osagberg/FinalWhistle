/*
 * League page — Vitest tests (T2-6).
 *
 * Substance requirements:
 *   AC1 — 10-column table renders correct headers (#, Club, P, W, D, L, GF,
 *          GA, GD, Pts) — 8 stat cols + position + club name.
 *   AC2 — standings data sourced from IPC via getStandings() on mount.
 *   AC4 — "Advance Week" and "Play Fixtures" buttons call the correct IPC fn,
 *          disable while pending, and re-enable on completion.
 *   AC5 — loading fallback, empty-state, and IPC error states render.
 *
 * Mocking strategy:
 *   - ~/lib/api/season is mocked globally. Each test configures per-function
 *     mock behaviour via vi.mocked().
 *   - @tauri-apps/api/core mocked so invoke() never throws outside Tauri.
 */

import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, within } from "@solidjs/testing-library";
import type { StandingsRow } from "~/lib/types";

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

// Import AFTER mocks are hoisted.
import League from "./League";
import {
  getStandings,
  advanceWeek,
  playFixtures,
} from "~/lib/api/season";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const FIXTURE_THREE_ROWS: StandingsRow[] = [
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
  {
    clubId: 3,
    clubName: "Cormorant Athletic",
    played: 5,
    wins: 1,
    draws: 0,
    losses: 4,
    goalsFor: 4,
    goalsAgainst: 11,
    goalDifference: -7,
    points: 3,
  },
];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("League page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getStandings).mockResolvedValue(FIXTURE_THREE_ROWS);
    vi.mocked(advanceWeek).mockResolvedValue({
      matchDayPlayed: 6,
      matchesPlayed: 10,
      seasonComplete: false,
    });
    vi.mocked(playFixtures).mockResolvedValue({
      matchesPlayed: 330,
      finalMatchDay: 38,
    });
  });

  // AC1: correct column headers (10 columns total).
  it("renders ten column headers: #, Club, P, W, D, L, GF, GA, GD, Pts", async () => {
    render(() => <League />);

    // Wait for the table to render (loading state clears after standings resolve).
    await waitFor(() => {
      expect(screen.getAllByRole("columnheader")).toHaveLength(10);
    });

    const headers = screen.getAllByRole("columnheader");
    const headerTexts = headers.map((h) => h.textContent?.trim() ?? "");

    // Position + Club + 8 stat columns = 10
    expect(headerTexts).toContain("#");
    expect(headerTexts).toContain("Club");
    expect(headerTexts).toContain("P");
    expect(headerTexts).toContain("W");
    expect(headerTexts).toContain("D");
    expect(headerTexts).toContain("L");
    expect(headerTexts).toContain("GF");
    expect(headerTexts).toContain("GA");
    expect(headerTexts).toContain("GD");
    expect(headerTexts).toContain("Pts");
    // Exactly 10 headers — adding or removing a column fails this assertion.
    expect(headers).toHaveLength(10);
  });

  // AC2: standings data sourced from IPC on mount.
  it("loads standings from getStandings() on mount and renders club names", async () => {
    render(() => <League />);

    await waitFor(() => {
      expect(screen.getByText("Aardvark FC")).toBeInTheDocument();
    });

    expect(screen.getByText("Brindlewood City")).toBeInTheDocument();
    expect(screen.getByText("Cormorant Athletic")).toBeInTheDocument();
    // IPC was called exactly once on mount.
    expect(vi.mocked(getStandings)).toHaveBeenCalledTimes(1);
  });

  // AC4a: Advance Week calls advanceWeek(), disables while pending.
  it("Advance Week button calls advanceWeek and disables while pending", async () => {
    let resolveAdvance!: (value: { matchDayPlayed: number; matchesPlayed: number; seasonComplete: boolean }) => void;
    vi.mocked(advanceWeek).mockImplementation(
      () =>
        new Promise((res) => {
          resolveAdvance = res;
        }),
    );

    render(() => <League />);

    // Wait for initial render to settle.
    await waitFor(() =>
      expect(screen.getByRole("button", { name: /advance week/i })).toBeInTheDocument(),
    );

    const btn = screen.getByRole("button", { name: /advance week/i });
    fireEvent.click(btn);

    // While pending, the button should be disabled.
    await waitFor(() => {
      expect(btn).toBeDisabled();
    });

    // IPC was called.
    expect(vi.mocked(advanceWeek)).toHaveBeenCalledTimes(1);

    // Resolve the action — button re-enables.
    resolveAdvance({ matchDayPlayed: 6, matchesPlayed: 10, seasonComplete: false });
    await waitFor(() => {
      expect(btn).not.toBeDisabled();
    });
  });

  // AC4b: Play Fixtures calls playFixtures().
  it("Play Fixtures button calls playFixtures and disables while pending", async () => {
    let resolveFixtures!: (value: { matchesPlayed: number; finalMatchDay: number }) => void;
    vi.mocked(playFixtures).mockImplementation(
      () =>
        new Promise((res) => {
          resolveFixtures = res;
        }),
    );

    render(() => <League />);

    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: /play fixtures/i }),
      ).toBeInTheDocument(),
    );

    const btn = screen.getByRole("button", { name: /play fixtures/i });
    fireEvent.click(btn);

    await waitFor(() => {
      expect(btn).toBeDisabled();
    });

    expect(vi.mocked(playFixtures)).toHaveBeenCalledTimes(1);

    resolveFixtures({ matchesPlayed: 330, finalMatchDay: 38 });
    await waitFor(() => {
      expect(btn).not.toBeDisabled();
    });
  });

  // AC4c: both buttons show "Working…" while pending.
  it("both buttons show 'Working…' text while an action is pending", async () => {
    // Hold advanceWeek open indefinitely for this test.
    vi.mocked(advanceWeek).mockImplementation(
      () => new Promise(() => {/* never resolves */}),
    );

    render(() => <League />);

    await waitFor(() =>
      expect(screen.getByRole("button", { name: /advance week/i })).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByRole("button", { name: /advance week/i }));

    await waitFor(() => {
      // Both buttons render "Working…" while any action is pending.
      const workingBtns = screen.getAllByRole("button", { name: /working/i });
      expect(workingBtns).toHaveLength(2);
    });
  });

  // AC5a: loading state shows fallback copy.
  it("shows loading fallback before standings resolve", () => {
    // Returning a never-resolving promise keeps the component in loading state.
    vi.mocked(getStandings).mockImplementation(
      () => new Promise(() => {/* pending */}),
    );

    render(() => <League />);

    expect(screen.getByText(/loading standings/i)).toBeInTheDocument();
  });

  // AC5b: error from getStandings shows the error state.
  it("shows error state when getStandings rejects", async () => {
    vi.mocked(getStandings).mockRejectedValue({
      kind: "lockPoisoned",
      lock: "season",
    });

    render(() => <League />);

    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });

    const alert = screen.getByRole("alert");
    expect(alert.textContent).toContain("Failed to load standings");
    // The lockPoisoned message should be surfaced.
    expect(alert.textContent).toContain("season");
  });

  // AC5c: IpcError from advanceWeek renders inline below buttons.
  // Post-T2-6 silent-failure-hunter P1 fix: action errors render in a
  // `role="status"` region (which carries BOTH success-with-summary and
  // failure-with-error) — distinct from the standings `role="alert"` panel
  // so two simultaneous failures don't collide in `getByRole("alert")`.
  it("shows inline action error when advanceWeek throws an IpcError", async () => {
    vi.mocked(advanceWeek).mockRejectedValue({
      kind: "seasonComplete",
    });

    render(() => <League />);

    await waitFor(() =>
      expect(screen.getByRole("button", { name: /advance week/i })).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByRole("button", { name: /advance week/i }));

    await waitFor(() => {
      const statusRegion = screen.getByRole("status");
      expect(statusRegion.textContent).toContain("season is already complete");
    });
  });

  // AC4d (post-T2-6 silent-failure-hunter P1 fix): success outcome shows
  // visible feedback derived from the AdvanceWeekSummary DTO — replaces the
  // prior shape which threw the summary away + gave the user no feedback at
  // all on a successful no-op.
  it("shows success outcome message after advanceWeek resolves", async () => {
    vi.mocked(advanceWeek).mockResolvedValue({
      matchDayPlayed: 7,
      matchesPlayed: 10,
      seasonComplete: false,
    });

    render(() => <League />);

    await waitFor(() =>
      expect(screen.getByRole("button", { name: /advance week/i })).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByRole("button", { name: /advance week/i }));

    await waitFor(() => {
      const statusRegion = screen.getByRole("status");
      // The summary's matchDayPlayed (7) + matchesPlayed (10) must appear in
      // the visible text — mutation removing summary plumbing would fail this.
      expect(statusRegion.textContent).toContain("Match-day 7");
      expect(statusRegion.textContent).toContain("10 matches");
    });
  });

  // AC5d: empty-state copy when standings returns an empty array.
  it("shows empty-state message when standings is an empty array", async () => {
    vi.mocked(getStandings).mockResolvedValue([]);

    render(() => <League />);

    // Wait for loading state to clear — the empty-state copy lives in the
    // DataTable which only renders after the resource settles.
    await waitFor(() => {
      expect(screen.getByText(/no standings yet/i)).toBeInTheDocument();
    });
  });

  // ---------------------------------------------------------------------------
  // Sort coverage (post-T2-6 silent-failure-hunter P1 #5 fix).
  //
  // The original test suite had ZERO sort coverage. That gap let a real
  // correctness bug land in the position column: `cell: (info) => info.row.index + 1`
  // uses the ORIGINAL-array index, not the post-sort visual position. After
  // any user-driven sort, the # column would silently render the wrong
  // ordering — the most-read column on a standings table.
  //
  // These tests pin the fixed behaviour: `info.table.getRowModel().rows.indexOf(row) + 1`.
  // ---------------------------------------------------------------------------

  // A SHUFFLED fixture — input order does NOT match any sortable column's
  // canonical order. This is the right shape for the position-column P0 fix
  // proof: original-array indexes and post-sort visual indexes are guaranteed
  // to differ on at least one row no matter which column is sorted.
  // Original-array order: Brindlewood(10pts, idx 0), Aardvark(13pts, idx 1),
  //                       Cormorant(3pts, idx 2).
  const SHUFFLED_THREE_ROWS: StandingsRow[] = [
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
      clubId: 3,
      clubName: "Cormorant Athletic",
      played: 5,
      wins: 1,
      draws: 0,
      losses: 4,
      goalsFor: 4,
      goalsAgainst: 11,
      goalDifference: -7,
      points: 3,
    },
  ];

  // Click Pts header — TanStack v8 sorts numeric columns DESC on first click.
  // So Pts DESC = Aardvark(13), Brindlewood(10), Cormorant(3) — distinct from
  // the input order [Brindlewood, Aardvark, Cormorant].
  it("clicking Pts header sorts rows by points descending; clicking again ascends", async () => {
    vi.mocked(getStandings).mockResolvedValue(SHUFFLED_THREE_ROWS);

    render(() => <League />);

    await waitFor(() =>
      expect(screen.getByText("Aardvark FC")).toBeInTheDocument(),
    );

    // Pre-sort: input order — Brindlewood, Aardvark, Cormorant.
    const initialRows = within(screen.getByRole("table"))
      .getAllByRole("row")
      .slice(1);
    expect(initialRows[0]?.textContent).toContain("Brindlewood City");
    expect(initialRows[1]?.textContent).toContain("Aardvark FC");
    expect(initialRows[2]?.textContent).toContain("Cormorant Athletic");

    // First click — DESC by points: Aardvark(13), Brindlewood(10), Cormorant(3).
    const ptsHeader = screen.getByRole("columnheader", { name: /^Pts/i });
    fireEvent.click(ptsHeader);

    await waitFor(() => {
      const sortedRows = within(screen.getByRole("table"))
        .getAllByRole("row")
        .slice(1);
      expect(sortedRows[0]?.textContent).toContain("Aardvark FC");
      expect(sortedRows[2]?.textContent).toContain("Cormorant Athletic");
    });

    // Second click — ASC: Cormorant(3), Brindlewood(10), Aardvark(13).
    fireEvent.click(ptsHeader);

    await waitFor(() => {
      const sortedRows = within(screen.getByRole("table"))
        .getAllByRole("row")
        .slice(1);
      expect(sortedRows[0]?.textContent).toContain("Cormorant Athletic");
      expect(sortedRows[2]?.textContent).toContain("Aardvark FC");
    });
  });

  // The P0 fix proof: after sorting DESC by Pts (first click on numeric col),
  // the # column must show 1 / 2 / 3 against the NEW visual order
  // (Aardvark=1, Brindlewood=2, Cormorant=3) — NOT the original-array
  // indexes (which would render 2 / 1 / 3 against those visual positions).
  it("position column re-numbers 1/2/3 against the current sorted order", async () => {
    vi.mocked(getStandings).mockResolvedValue(SHUFFLED_THREE_ROWS);

    render(() => <League />);

    await waitFor(() =>
      expect(screen.getByText("Aardvark FC")).toBeInTheDocument(),
    );

    // Sort DESC by Pts (first click on numeric column).
    const ptsHeader = screen.getByRole("columnheader", { name: /^Pts/i });
    fireEvent.click(ptsHeader);

    await waitFor(() => {
      const rows = within(screen.getByRole("table"))
        .getAllByRole("row")
        .slice(1);

      // Visual order after sort: Aardvark (Pts=13) first → position 1.
      //                          Brindlewood (Pts=10) second → position 2.
      //                          Cormorant (Pts=3) last → position 3.
      // Each row's first <td> is the position cell.
      const firstCellOf = (row: HTMLElement): string =>
        row.querySelector("td")?.textContent?.trim() ?? "";

      // CRITICAL: these positions must reflect the VISUAL order, not the
      // original-array order. With the buggy `row.index + 1` cell renderer,
      // Aardvark (originally idx 1) would render "2" in position 1 of the
      // visual table, Brindlewood (idx 0) would render "1" in position 2,
      // Cormorant (idx 2) would render "3" in position 3 — internally
      // inconsistent and silently wrong.
      expect(firstCellOf(rows[0]!)).toBe("1");
      expect(firstCellOf(rows[1]!)).toBe("2");
      expect(firstCellOf(rows[2]!)).toBe("3");

      // Sanity: rows ARE in the expected visual order.
      expect(rows[0]?.textContent).toContain("Aardvark FC");
      expect(rows[1]?.textContent).toContain("Brindlewood City");
      expect(rows[2]?.textContent).toContain("Cormorant Athletic");
    });
  });

  // The position column carries enableSorting: false — clicking its header
  // does not change row order.
  it("position column is not sortable (clicking # header is a no-op)", async () => {
    render(() => <League />);

    await waitFor(() =>
      expect(screen.getByText("Aardvark FC")).toBeInTheDocument(),
    );

    // Snapshot the initial order.
    const initialOrder = within(screen.getByRole("table"))
      .getAllByRole("row")
      .slice(1)
      .map((r) => r.textContent ?? "");

    // Click # header.
    const positionHeader = screen.getByRole("columnheader", { name: /^#/ });
    fireEvent.click(positionHeader);

    // Order unchanged.
    const afterClickOrder = within(screen.getByRole("table"))
      .getAllByRole("row")
      .slice(1)
      .map((r) => r.textContent ?? "");
    expect(afterClickOrder).toEqual(initialOrder);
  });
});

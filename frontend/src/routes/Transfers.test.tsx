/*
 * Transfers page — Vitest smoke tests (T2-8).
 *
 * Verifies the IPC → derivation → render chain works end-to-end:
 *   1. Window-state label renders (AC1)
 *   2. State changes based on current match-day (AC2)
 *   3. Mechanics-deferred copy is present (AC4)
 *
 * Pure-function coverage of `computeTransferWindowState` lives in
 * `transfer-window.test.ts` — this file only exercises the render
 * integration.
 */

import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@solidjs/testing-library";
import type { StandingsRow } from "~/lib/types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

vi.mock("~/lib/api/season", () => ({
  getStandings: vi.fn(),
  advanceWeek: vi.fn(),
  playFixtures: vi.fn(),
  getFixtures: vi.fn(),
}));

import Transfers from "./Transfers";
import { getStandings } from "~/lib/api/season";

function rowAtMatchDay(played: number): StandingsRow {
  return {
    clubId: 1,
    clubName: "Aardvark FC",
    played,
    wins: 0,
    draws: 0,
    losses: 0,
    goalsFor: 0,
    goalsAgainst: 0,
    goalDifference: 0,
    points: 0,
  };
}

describe("Transfers page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // AC1 + AC2 (summer branch): match-day 0 ⇒ summer label.
  it("renders 'Summer window — open' for pre-season (match-day 0)", async () => {
    vi.mocked(getStandings).mockResolvedValue([rowAtMatchDay(0)]);

    render(() => <Transfers />);

    await waitFor(() => {
      expect(screen.getByText(/summer window/i)).toBeInTheDocument();
    });

    // Sanity: the "Match-day N" label reflects the actual value.
    expect(screen.getByText(/match-day 0/i)).toBeInTheDocument();
  });

  // AC2 (winter branch): match-day 19 ⇒ winter label.
  it("renders 'Winter window — open' for match-day 19 (mid-season)", async () => {
    vi.mocked(getStandings).mockResolvedValue([rowAtMatchDay(19)]);

    render(() => <Transfers />);

    await waitFor(() => {
      expect(screen.getByText(/winter window/i)).toBeInTheDocument();
    });

    expect(screen.getByText(/match-day 19/i)).toBeInTheDocument();
  });

  // AC2 (closed branch): match-day 5 ⇒ closed label (in-season).
  it("renders 'Closed' for an in-season match-day (e.g. day 5)", async () => {
    vi.mocked(getStandings).mockResolvedValue([rowAtMatchDay(5)]);

    render(() => <Transfers />);

    await waitFor(() => {
      // Must match "Closed" but not "Closed — alongside-other-text" ambiguity.
      // The pill text is exactly "Closed" per WindowState.label contract.
      expect(screen.getByText("Closed")).toBeInTheDocument();
    });
  });

  // AC4: the mechanics-deferred panel copy is present.
  // Post-T2-8 code-reviewer P1 fix: dropped the internal "Phase T3"
  // milestone label from player-facing copy per Frontend/RULES.md §9
  // (football-native vocabulary only). Test asserts the football-vernacular
  // "coming" framing instead.
  it("renders the 'transfer mechanics are coming' deferral copy", async () => {
    vi.mocked(getStandings).mockResolvedValue([rowAtMatchDay(0)]);

    render(() => <Transfers />);

    await waitFor(() => {
      // The "coming soon" phrase appears in BOTH the header subtitle and
      // the dedicated deferral panel. Match the panel-specific phrasing
      // ("bids, negotiations, and contracts") to avoid the multi-match
      // ambiguity that would otherwise raise TestingLibraryElementError.
      expect(
        screen.getByText(/bids, negotiations, and contracts/i),
      ).toBeInTheDocument();
    });
  });

  // Post-T2-8 silent-failure-hunter P1 fix: getStandings error renders an
  // EXPLICIT error message — NOT a confidently-wrong "Summer window" default.
  // The prior shape laundered backend failures into a friendly pill which
  // hid lockPoisoned / IPC failures from the user; this test pins the
  // honest-display contract.
  it("renders explicit error message when getStandings errors", async () => {
    vi.mocked(getStandings).mockRejectedValue({
      kind: "lockPoisoned",
      lock: "season",
    });

    render(() => <Transfers />);

    await waitFor(() => {
      // Must NOT show the summer-window pill — that would be the
      // silently-wrong outcome.
      expect(screen.queryByText(/summer window/i)).not.toBeInTheDocument();
    });
    // Must show the error region with the IpcError context.
    const alert = screen.getByRole("alert");
    expect(alert.textContent).toContain("window status unavailable");
    expect(alert.textContent).toContain("lockPoisoned");
  });

  // Post-T2-8 silent-failure-hunter P1 fix: empty standings array is
  // distinct from "pre-season" — renders the "season not loaded" status,
  // NOT a default summer pill.
  it("renders 'season not loaded' status when standings is empty (distinct from pre-season)", async () => {
    vi.mocked(getStandings).mockResolvedValue([]);

    render(() => <Transfers />);

    await waitFor(() => {
      expect(screen.getByRole("status").textContent).toContain(
        "Season not loaded",
      );
    });
    expect(screen.queryByText(/summer window/i)).not.toBeInTheDocument();
  });
});

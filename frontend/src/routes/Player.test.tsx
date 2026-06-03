/*
 * Player detail page — Vitest tests (T3-6, extended T4-F4).
 *
 * AC coverage (original T3-6):
 *   AC1 — Three blocks render: phenotype, career moments, contract.
 *   AC2 — getPlayerDetail() called on mount with the route param id.
 *   AC3 — Phenotype labels + memory callback strings render as text.
 *   AC4 — Loading state shows fallback copy.
 *   AC5 — IPC error state shows role="alert" with error text.
 *   AC6 — Empty memoryCallbacks shows "No notable career moments yet."
 *
 * AC coverage (T4-F4 scout-report section):
 *   AC7  — Valid report renders overall band + 3 category band labels + label bands.
 *   AC8  — notYetObserved rejection renders graceful "no scouting read yet" copy,
 *           NOT an error banner (no role="alert").
 *   AC9  — playerNotFound rejection omits the scout section without breaking
 *           the phenotype and career-moments render.
 *   AC10 — Profile label ("Profile:") is the bio-truth surface; "Scout traits:"
 *          label was removed at T4-F4 to distinguish bio from uncertain read.
 *
 * Mocking strategy:
 *   - ~/lib/api/player is mocked globally for getPlayerDetail.
 *   - ~/lib/api/scout is mocked globally for getScoutReport.
 *   - @solidjs/router's useParams is mocked to return a stable id.
 *   - @tauri-apps/api/core mocked so invoke() never throws outside Tauri.
 */

import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@solidjs/testing-library";
import type { PlayerDetail, ScoutReportDto } from "~/lib/types";

// ---------------------------------------------------------------------------
// Module mocks — hoisted before component import
// ---------------------------------------------------------------------------

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(null),
}));

vi.mock("~/lib/api/player", () => ({
  getPlayerDetail: vi.fn(),
}));

vi.mock("~/lib/api/scout", () => ({
  getScoutReport: vi.fn(),
}));

// Stable route param — id that would come from /player/:id.
vi.mock("@solidjs/router", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@solidjs/router")>();
  return {
    ...actual,
    useParams: vi.fn().mockReturnValue({ id: "fwh.core:player_00001" }),
  };
});

// Import AFTER mocks are hoisted.
import Player from "./Player";
import { getPlayerDetail } from "~/lib/api/player";
import { getScoutReport } from "~/lib/api/scout";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const FIXTURE_PLAYER_DETAIL: PlayerDetail = {
  phenotype: {
    playerId: "fwh.core:player_00001",
    name: "Emeka Thorne",
    role: "Striker",
    birthRegion: "Ashvale",
    phenotypeLabels: ["Pure finisher", "Poacher"],
  },
  memoryCallbacks: [
    "Made his debut for the first team on a wet Tuesday.",
    "Scored his first senior goal against a high defensive line.",
  ],
  contractStatus: null,
};

const FIXTURE_PLAYER_NO_CALLBACKS: PlayerDetail = {
  phenotype: {
    playerId: "fwh.core:player_00002",
    name: "Seren Voss",
    role: "Goalkeeper",
    birthRegion: "Brackwater",
    phenotypeLabels: [],
  },
  memoryCallbacks: [],
  contractStatus: null,
};

const FIXTURE_SCOUT_REPORT: ScoutReportDto = {
  playerId: 42,
  confidence: 0.72,
  overallBand: "a confident read",
  observationCount: 7,
  categories: [
    { category: "Physical", low: 0.6, high: 0.8, band: "a tentative read" },
    { category: "Mental", low: 0.65, high: 0.85, band: "a confident read" },
    { category: "Technical", low: 0.55, high: 0.75, band: "a settled read" },
  ],
  labels: [
    { label: "Pure finisher", confidence: 0.8, band: "a confident read" },
    { label: "Poacher", confidence: 0.5, band: "a tentative read" },
  ],
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Player detail page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getPlayerDetail).mockResolvedValue(FIXTURE_PLAYER_DETAIL);
    // Default: scout section loading — keeps tests that don't care about it
    // from flaking on timing. Individual tests override as needed.
    vi.mocked(getScoutReport).mockReturnValue(new Promise(() => { /* pending */ }));
  });

  // AC1: three blocks render (phenotype / career moments / contract).
  it("renders phenotype, career moments, and contract sections", async () => {
    render(() => <Player />);

    await waitFor(() => {
      expect(screen.getByRole("region", { name: /player profile/i })).toBeInTheDocument();
    });

    expect(screen.getByRole("region", { name: /career moments/i })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: /contract details/i })).toBeInTheDocument();
  });

  // AC2: getPlayerDetail called on mount with the route param id.
  it("calls getPlayerDetail on mount with the decoded player id", async () => {
    render(() => <Player />);

    await waitFor(() => {
      expect(vi.mocked(getPlayerDetail)).toHaveBeenCalledTimes(1);
    });

    expect(vi.mocked(getPlayerDetail)).toHaveBeenCalledWith("fwh.core:player_00001");
  });

  // AC3a: phenotype block shows name, role, region.
  it("renders player name, role, and birth region in the phenotype block", async () => {
    render(() => <Player />);

    await waitFor(() => {
      expect(screen.getByText("Emeka Thorne")).toBeInTheDocument();
    });

    expect(screen.getByText("Striker")).toBeInTheDocument();
    expect(screen.getByText("Ashvale")).toBeInTheDocument();
  });

  // AC3b: phenotype labels render under "Profile:" (not "Scout traits:" — T4-F4 rename).
  it("renders profile labels as comma-joined human-readable text under Profile:", async () => {
    render(() => <Player />);

    await waitFor(() => {
      expect(screen.getByText(/pure finisher/i)).toBeInTheDocument();
    });

    const traitsText = screen.getByText(/pure finisher/i).textContent ?? "";
    expect(traitsText).toContain(",");
    expect(traitsText).not.toContain("[");
    expect(traitsText).not.toContain("]");
  });

  // AC10: bio labels surface is "Profile:" not the old "Scout traits:".
  it("shows 'Profile:' label for bio truth, not 'Scout traits:'", async () => {
    render(() => <Player />);

    await waitFor(() => {
      expect(screen.getByText("Emeka Thorne")).toBeInTheDocument();
    });

    // "Profile:" should appear somewhere in the phenotype section.
    // getByText (not queryByText) for a positive assertion — queryByText returns
    // null on miss, producing a confusing "received null" rather than a clean
    // "unable to find element" failure.
    expect(screen.getByText(/profile:/i)).toBeInTheDocument();
    // "Scout traits:" was the pre-T4-F4 label — must not appear (queryByText is
    // correct here: negative assertion).
    expect(screen.queryByText(/scout traits:/i)).not.toBeInTheDocument();
  });

  // AC3c: memory callback strings render as list items.
  it("renders memory callback strings as list items in career moments", async () => {
    render(() => <Player />);

    await waitFor(() => {
      expect(screen.getByText(/made his debut/i)).toBeInTheDocument();
    });

    expect(screen.getByText(/scored his first senior goal/i)).toBeInTheDocument();

    const list = screen.getByRole("list", { name: /memory callbacks/i });
    expect(list.querySelectorAll("li")).toHaveLength(2);
  });

  // AC4: loading state shows fallback copy before promise resolves.
  it("shows loading fallback before getPlayerDetail resolves", () => {
    vi.mocked(getPlayerDetail).mockImplementation(
      () => new Promise(() => {/* pending */}),
    );

    render(() => <Player />);

    expect(screen.getByText(/loading player/i)).toBeInTheDocument();
  });

  // AC5a: IPC error (lockPoisoned) shows role="alert" with football-native copy.
  it("shows error alert when getPlayerDetail rejects with IpcError", async () => {
    vi.mocked(getPlayerDetail).mockRejectedValue({
      kind: "lockPoisoned",
      lock: "memory_ledger",
    });

    render(() => <Player />);

    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });

    const alert = screen.getByRole("alert");
    // lockPoisoned copy mentions the lock name in the detail.
    expect(alert.textContent).toContain("memory_ledger");
    // Must NOT show raw err.message.
    expect(alert.textContent).not.toContain("Cannot read properties");
  });

  // AC5b: playerNotFound error surfaces the player id in the message.
  it("shows playerNotFound message when player id is absent from content store", async () => {
    vi.mocked(getPlayerDetail).mockRejectedValue({
      kind: "playerNotFound",
      playerId: "fwh.core:player_99999",
    });

    render(() => <Player />);

    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });

    const alert = screen.getByRole("alert");
    expect(alert.textContent).toContain("fwh.core:player_99999");
  });

  // AC6: empty memoryCallbacks shows honest empty-state copy.
  it("shows 'No notable career moments yet' when memoryCallbacks is empty", async () => {
    vi.mocked(getPlayerDetail).mockResolvedValue(FIXTURE_PLAYER_NO_CALLBACKS);

    render(() => <Player />);

    await waitFor(() => {
      expect(screen.getByText("Seren Voss")).toBeInTheDocument();
    });

    expect(
      screen.getByText(/no notable career moments yet/i),
    ).toBeInTheDocument();
  });

  // AC6b: contract deferred placeholder renders when contractStatus is null.
  it("renders deferred contract placeholder when contractStatus is null", async () => {
    render(() => <Player />);

    await waitFor(() => {
      expect(screen.getByText("Emeka Thorne")).toBeInTheDocument();
    });

    expect(
      screen.getByText(/contract details arrive with the career-roster layer/i),
    ).toBeInTheDocument();
  });

  // ---------------------------------------------------------------------------
  // AC7: valid scout report renders overall band, category bands, and labels.
  // ---------------------------------------------------------------------------

  it("renders scouting report with overall band, category bands, and label bands", async () => {
    vi.mocked(getScoutReport).mockResolvedValue(FIXTURE_SCOUT_REPORT);

    render(() => <Player />);

    await waitFor(() => {
      // Scout section heading must appear.
      expect(screen.getByRole("region", { name: /scouting report/i })).toBeInTheDocument();
    });

    const section = screen.getByRole("region", { name: /scouting report/i });

    // Overall band text renders (NOT the raw 0.72 number).
    expect(section.textContent).toContain("a confident read");
    // Raw confidence number must NOT appear.
    expect(section.textContent).not.toContain("0.72");

    // Category band labels for all three categories.
    expect(section.textContent).toContain("Physical");
    expect(section.textContent).toContain("Mental");
    expect(section.textContent).toContain("Technical");
    expect(section.textContent).toContain("a tentative read");
    expect(section.textContent).toContain("a settled read");

    // Per-label rows.
    expect(section.textContent).toContain("Pure finisher");
    expect(section.textContent).toContain("Poacher");

    // Observation count shown as "watched N times".
    expect(section.textContent).toContain("7");
    expect(section.textContent).toContain("times");
  });

  it("does not show raw low/high numbers in the scouting report", async () => {
    vi.mocked(getScoutReport).mockResolvedValue(FIXTURE_SCOUT_REPORT);

    render(() => <Player />);

    await waitFor(() => {
      expect(screen.getByRole("region", { name: /scouting report/i })).toBeInTheDocument();
    });

    const section = screen.getByRole("region", { name: /scouting report/i });
    // Raw numeric values from the DTO must not be visible.
    expect(section.textContent).not.toContain("0.6");
    expect(section.textContent).not.toContain("0.8");
    expect(section.textContent).not.toContain("0.65");
  });

  // ---------------------------------------------------------------------------
  // AC8: notYetObserved → graceful muted note; no role="alert".
  // ---------------------------------------------------------------------------

  it("renders graceful 'no scouting read yet' message for notYetObserved, not an error banner", async () => {
    vi.mocked(getScoutReport).mockRejectedValue({
      kind: "notYetObserved",
      playerId: "fwh.core:player_00001",
    });

    render(() => <Player />);

    await waitFor(() => {
      // The phenotype block must still load.
      expect(screen.getByText("Emeka Thorne")).toBeInTheDocument();
    });

    // The scout section itself must appear.
    await waitFor(() => {
      expect(screen.getByRole("region", { name: /scouting report/i })).toBeInTheDocument();
    });

    const section = screen.getByRole("region", { name: /scouting report/i });
    // Graceful copy present.
    expect(section.textContent).toMatch(/no scouting read yet/i);
    // Must NOT be a red error banner.
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  // ---------------------------------------------------------------------------
  // AC9: playerNotFound → scout section omitted; phenotype/career still render.
  // ---------------------------------------------------------------------------

  it("omits the scout section when playerNotFound is returned, without breaking phenotype/career blocks", async () => {
    vi.mocked(getScoutReport).mockRejectedValue({
      kind: "playerNotFound",
      playerId: "fwh.core:player_00001",
    });

    render(() => <Player />);

    await waitFor(() => {
      // Phenotype block must still render.
      expect(screen.getByText("Emeka Thorne")).toBeInTheDocument();
    });

    // Scout section must NOT appear (content-bio player; scouting N/A).
    // Wait for the scout resource to SETTLE first…
    await waitFor(() => {
      expect(screen.queryByRole("region", { name: /scouting report/i })).not.toBeInTheDocument();
    });

    // …THEN assert the phenotype + career blocks are still intact AFTER the
    // scout-resource failure settled — proving the two resources are isolated
    // (the whole point of AC9). Asserting before the settle wouldn't prove it.
    expect(screen.getByText("Emeka Thorne")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: /career moments/i })).toBeInTheDocument();

    // No error banner either.
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});

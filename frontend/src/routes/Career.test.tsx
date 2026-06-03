/*
 * Career page — Vitest tests (T3-9).
 *
 * Substance requirements (AC6):
 *   - Season number renders from getCareerOverview() on mount.
 *   - Champion history table renders; honest empty-state when history is [].
 *   - "Advance to next season" button calls advanceSeason(); disables while pending.
 *   - Success outcome line renders on resolve.
 *   - `seasonNotComplete` rejection renders a clear "finish the current season" message.
 *   - Loading state shows fallback copy.
 *   - IPC-error state shows alert panel.
 *   - Empty crossSeasonCallbacks renders the empty-state copy.
 *   - Non-empty crossSeasonCallbacks renders the list items.
 *
 * Mocking strategy:
 *   - ~/lib/api/career is mocked globally. Each test configures per-function
 *     mock behaviour via vi.mocked().
 *   - @tauri-apps/api/core mocked so invoke() never throws outside Tauri.
 */

import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@solidjs/testing-library";
import type { CareerOverview } from "~/lib/types";

// ---------------------------------------------------------------------------
// Module mocks — hoisted before component import
// ---------------------------------------------------------------------------

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue({}),
}));

vi.mock("~/lib/api/career", () => ({
  getCareerOverview: vi.fn(),
  advanceSeason: vi.fn(),
  getPressInbox: vi.fn(),
}));

// Import AFTER mocks are hoisted.
import Career from "./Career";
import { getCareerOverview, advanceSeason, getPressInbox } from "~/lib/api/career";
import type { PressInboxDto } from "~/lib/types";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const OVERVIEW_EMPTY: CareerOverview = {
  seasonNumber: 1,
  history: [],
  crossSeasonCallbacks: [],
};

const PRESS_INBOX_EMPTY: PressInboxDto = {
  seasonNumber: 1,
  items: [],
};

const PRESS_INBOX_WITH_ITEMS: PressInboxDto = {
  seasonNumber: 3,
  items: [
    {
      eventId: 1,
      season: 0,
      eventClass: 24,
      topic: "playerMilestone",
      headline: "Emeka Thorne made his senior debut and showed composure beyond his years",
      managerQuote: null,
    },
    {
      eventId: 47,
      season: 1,
      eventClass: 22,
      topic: "matchResult",
      headline: "Northshire Town are champions",
      managerQuote: "The lads gave everything. A night nobody will forget.",
    },
    {
      eventId: 63,
      season: 2,
      eventClass: 9,
      topic: "contractTransfer",
      headline: "Seren Voss signed a contract extension",
      managerQuote: null,
    },
    {
      eventId: 88,
      season: 2,
      eventClass: 18,
      topic: "relational",
      headline: "A partnership built over two seasons is drawing attention from rival clubs",
      managerQuote: null,
    },
  ],
};

const OVERVIEW_WITH_HISTORY: CareerOverview = {
  seasonNumber: 3,
  history: [
    { season: 1, championClubName: "Aardvark FC" },
    { season: 2, championClubName: "Brindlewood City" },
  ],
  crossSeasonCallbacks: [
    "Last season's golden boot winner is showing early form this campaign.",
    "The defensive unit that conceded only 18 goals last season remains intact.",
  ],
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Career page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getCareerOverview).mockResolvedValue(OVERVIEW_EMPTY);
    vi.mocked(getPressInbox).mockResolvedValue(PRESS_INBOX_EMPTY);
    vi.mocked(advanceSeason).mockResolvedValue({
      completedSeason: 1,
      championClubName: "Aardvark FC",
      newSeasonNumber: 2,
      compactionFired: false,
    });
  });

  // getCareerOverview is called on mount.
  it("calls getCareerOverview on mount", async () => {
    render(() => <Career />);

    await waitFor(() => {
      expect(vi.mocked(getCareerOverview)).toHaveBeenCalledTimes(1);
    });
  });

  // Season number renders from overview.
  it("renders the current season number from overview", async () => {
    render(() => <Career />);

    await waitFor(() => {
      expect(screen.getByText(/season 1/i)).toBeInTheDocument();
    });
  });

  // Season number renders with history.
  it("renders season 3 when overview.seasonNumber is 3", async () => {
    vi.mocked(getCareerOverview).mockResolvedValue(OVERVIEW_WITH_HISTORY);

    render(() => <Career />);

    await waitFor(() => {
      expect(screen.getByText(/season 3/i)).toBeInTheDocument();
    });
  });

  // Champion history renders.
  it("renders champion history entries when history is non-empty", async () => {
    vi.mocked(getCareerOverview).mockResolvedValue(OVERVIEW_WITH_HISTORY);

    render(() => <Career />);

    await waitFor(() => {
      expect(screen.getByText("Aardvark FC")).toBeInTheDocument();
    });

    expect(screen.getByText("Brindlewood City")).toBeInTheDocument();
    // Season labels.
    expect(screen.getByText(/season 1/i)).toBeInTheDocument();
    expect(screen.getByText(/season 2/i)).toBeInTheDocument();
  });

  // Empty history shows honest empty-state copy.
  it("shows empty-state copy when champion history is empty", async () => {
    render(() => <Career />);

    await waitFor(() => {
      expect(
        screen.getByText(/no seasons completed yet/i),
      ).toBeInTheDocument();
    });
  });

  // Cross-season callbacks render when non-empty.
  it("renders cross-season callbacks when present", async () => {
    vi.mocked(getCareerOverview).mockResolvedValue(OVERVIEW_WITH_HISTORY);

    render(() => <Career />);

    await waitFor(() => {
      expect(
        screen.getByText(/golden boot winner/i),
      ).toBeInTheDocument();
    });

    expect(
      screen.getByText(/defensive unit/i),
    ).toBeInTheDocument();
  });

  // Empty crossSeasonCallbacks shows honest empty-state.
  it("shows empty-callbacks copy when crossSeasonCallbacks is empty", async () => {
    render(() => <Career />);

    await waitFor(() => {
      expect(
        screen.getByText(/no past-season moments yet/i),
      ).toBeInTheDocument();
    });
  });

  // Advance button calls advanceSeason.
  it("Advance to next season button calls advanceSeason", async () => {
    render(() => <Career />);

    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: /advance to next season/i }),
      ).toBeInTheDocument(),
    );

    const btn = screen.getByRole("button", { name: /advance to next season/i });
    fireEvent.click(btn);

    await waitFor(() => {
      expect(vi.mocked(advanceSeason)).toHaveBeenCalledTimes(1);
    });
  });

  // Button disables while pending.
  it("Advance button disables while advanceSeason is pending", async () => {
    let resolveAdvance!: (value: {
      completedSeason: number;
      championClubName: string;
      newSeasonNumber: number;
      compactionFired: boolean;
    }) => void;
    vi.mocked(advanceSeason).mockImplementation(
      () =>
        new Promise((res) => {
          resolveAdvance = res;
        }),
    );

    render(() => <Career />);

    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: /advance to next season/i }),
      ).toBeInTheDocument(),
    );

    const btn = screen.getByRole("button", { name: /advance to next season/i });
    fireEvent.click(btn);

    await waitFor(() => {
      expect(btn).toBeDisabled();
    });

    // Resolve — button re-enables.
    resolveAdvance({
      completedSeason: 1,
      championClubName: "Aardvark FC",
      newSeasonNumber: 2,
      compactionFired: false,
    });

    await waitFor(() => {
      expect(btn).not.toBeDisabled();
    });
  });

  // Button shows "Working…" while pending.
  it("button shows Working… text while pending", async () => {
    vi.mocked(advanceSeason).mockImplementation(
      () => new Promise(() => {/* never resolves */}),
    );

    render(() => <Career />);

    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: /advance to next season/i }),
      ).toBeInTheDocument(),
    );

    fireEvent.click(
      screen.getByRole("button", { name: /advance to next season/i }),
    );

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: /working/i }),
      ).toBeInTheDocument();
    });
  });

  // Success outcome shows readable summary line.
  it("shows success outcome message after advanceSeason resolves", async () => {
    render(() => <Career />);

    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: /advance to next season/i }),
      ).toBeInTheDocument(),
    );

    fireEvent.click(
      screen.getByRole("button", { name: /advance to next season/i }),
    );

    await waitFor(() => {
      const statusRegion = screen.getByRole("status");
      // Season number, club name, new season number must appear.
      expect(statusRegion.textContent).toContain("Season 1");
      expect(statusRegion.textContent).toContain("Aardvark FC");
      expect(statusRegion.textContent).toContain("Season 2");
    });
  });

  // seasonNotComplete rejection renders a clear message.
  it("shows clear 'finish the current season' message on seasonNotComplete rejection", async () => {
    vi.mocked(advanceSeason).mockRejectedValue({ kind: "seasonNotComplete" });

    render(() => <Career />);

    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: /advance to next season/i }),
      ).toBeInTheDocument(),
    );

    fireEvent.click(
      screen.getByRole("button", { name: /advance to next season/i }),
    );

    await waitFor(() => {
      const statusRegion = screen.getByRole("status");
      // T4-4 self-review fix-pass: action-button outcome now routes through
      // describeRouteError. The seasonNotComplete copy is football-native
      // ("There are still fixtures to play" / "Play out the remaining games
      // first.") — assert the load-bearing fixtures phrase.
      expect(statusRegion.textContent?.toLowerCase()).toContain(
        "fixtures to play",
      );
      // Must not leak raw exception text from the old e.message path.
      expect(statusRegion.textContent).not.toContain("Cannot read properties");
    });
  });

  // Loading state shows fallback copy.
  it("shows loading fallback before overview resolves", () => {
    vi.mocked(getCareerOverview).mockImplementation(
      () => new Promise(() => {/* pending */}),
    );

    render(() => <Career />);

    expect(screen.getByText(/loading career overview/i)).toBeInTheDocument();
  });

  // IPC error from getCareerOverview shows alert panel with football-native copy.
  it("shows error alert when getCareerOverview rejects", async () => {
    vi.mocked(getCareerOverview).mockRejectedValue({
      kind: "lockPoisoned",
      lock: "career",
    });

    render(() => <Career />);

    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });

    const alert = screen.getByRole("alert");
    // lockPoisoned copy mentions the lock name in the detail.
    expect(alert.textContent).toContain("career");
    // Must NOT show raw err.message.
    expect(alert.textContent).not.toContain("Cannot read properties");
  });

  // ---------------------------------------------------------------------------
  // Press inbox panel — T4-2.5k
  // ---------------------------------------------------------------------------

  // (a) Press items render: headline and topic label visible.
  it("renders press items with headline text and topic label", async () => {
    vi.mocked(getPressInbox).mockResolvedValue(PRESS_INBOX_WITH_ITEMS);

    render(() => <Career />);

    await waitFor(() => {
      expect(
        screen.getByText(/emeka thorne made his senior debut/i),
      ).toBeInTheDocument();
    });

    // All four headlines must appear.
    expect(screen.getByText(/northshire town are champions/i)).toBeInTheDocument();
    expect(screen.getByText(/seren voss signed a contract extension/i)).toBeInTheDocument();
    expect(screen.getByText(/a partnership built over two seasons/i)).toBeInTheDocument();

    // Topic labels: all four topic types from the fixture.
    expect(screen.getByText("Milestone")).toBeInTheDocument();
    expect(screen.getByText("Result")).toBeInTheDocument();
    expect(screen.getByText("Transfer")).toBeInTheDocument();
    expect(screen.getByText("Story")).toBeInTheDocument();
  });

  // Manager quote renders when non-null; absent when null.
  it("renders manager quote when present, omits when null", async () => {
    vi.mocked(getPressInbox).mockResolvedValue(PRESS_INBOX_WITH_ITEMS);

    render(() => <Career />);

    // The champions item has a non-null managerQuote — it must appear.
    await waitFor(() => {
      expect(screen.getByText(/the lads gave everything/i)).toBeInTheDocument();
    });

    // The contract-transfer item has managerQuote: null — the unique phrase
    // "nobody will forget" is from the champions quote, which is non-null.
    // Cross-check: the debut, transfer, and relational items have null quotes —
    // their headlines appear but their null-quotes do not add any extra text.
    // We verify the *only* quoted text in the panel belongs to the champions item.
    const inboxSection = screen.getByRole("region", { name: /press inbox/i });
    // The champions quote text is present exactly once.
    expect(inboxSection.textContent).toContain("nobody will forget");
    // The seren voss item (transfer, null quote) has no quote text present.
    // There is no unique phrase to check absence of since managerQuote is null.
    // Instead assert count: only one quote wrapping (italic) element for the
    // one non-null quote.
    const italicElements = inboxSection.querySelectorAll("p.italic");
    expect(italicElements.length).toBe(1);
    expect(italicElements.item(0).textContent).toContain("lads gave everything");
  });

  // (b) Empty items renders football-native empty-state copy, NOT an error.
  it("shows 'No press yet' empty-state when items array is empty", async () => {
    vi.mocked(getPressInbox).mockResolvedValue(PRESS_INBOX_EMPTY);

    render(() => <Career />);

    await waitFor(() => {
      expect(screen.getByText(/no press yet/i)).toBeInTheDocument();
    });

    // Must not be a role="alert" — empty inbox is not an error condition.
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  // (c) getPressInbox rejection degrades the press section gracefully without
  //     breaking the overview panels (isolation guarantee).
  it("press section degrades gracefully on getPressInbox rejection without breaking overview", async () => {
    vi.mocked(getCareerOverview).mockResolvedValue(OVERVIEW_WITH_HISTORY);
    vi.mocked(getPressInbox).mockRejectedValue(new Error("network error"));

    render(() => <Career />);

    // Overview panels must still render.
    await waitFor(() => {
      expect(screen.getByText("Aardvark FC")).toBeInTheDocument();
    });

    expect(screen.getByText("Brindlewood City")).toBeInTheDocument();

    // Press section shows graceful inline note.
    await waitFor(() => {
      const pressSection = screen.getByRole("region", { name: /press inbox/i });
      expect(pressSection.textContent).toMatch(/couldn't be loaded/i);
    });

    // The overview error alert must NOT appear — failure is isolated.
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  // Press inbox section heading renders when data is available.
  it("renders the Press inbox section heading when items load", async () => {
    vi.mocked(getPressInbox).mockResolvedValue(PRESS_INBOX_EMPTY);

    render(() => <Career />);

    await waitFor(() => {
      expect(screen.getByRole("region", { name: /press inbox/i })).toBeInTheDocument();
    });
  });
});

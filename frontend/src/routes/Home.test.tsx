/*
 * Home route — Vitest tests.
 *
 * Two modes under test:
 *
 *   PRE-CAREER (isCareerActive() === false):
 *     AC1  — wordmark "FINAL WHISTLE" renders.
 *     AC2  — tagline renders.
 *     AC3  — "NEW CAREER" button is present and enabled.
 *     AC4  — "LOAD SAVE" button is present and enabled.
 *     AC5  — Settings link renders and points to /settings.
 *     AC6  — Backend status shows "checking backend…" while pending.
 *     AC7  — Backend status resolves to label on success.
 *     AC8  — NEW CAREER click navigates to /new-career with seedHex state.
 *     AC9  — LOAD SAVE click calls loadCareer(); navigates to /squad on success.
 *     AC10 — LOAD SAVE shows a football-native error on saveLoadFailed.
 *     AC11 — No stale diagnostic panel content.
 *
 *   ACTIVE CAREER (isCareerActive() === true):
 *     AC12 — Hub renders; main-menu elements (wordmark, NEW CAREER btn) absent.
 *     AC13 — Bite header: club name rendered from managedClubName signal.
 *     AC14 — Bite header: league position rendered from standings (ordinal).
 *     AC15 — Bite header: next fixture rendered (opponent + H/A + match day).
 *     AC16 — Press feed: items rendered from getPressInbox().
 *     AC17 — Quick-link cards present for Squad / League / Fixtures / Next match.
 *     AC18 — Honesty: missing league position slot is OMITTED (not em-dashed).
 *     AC19 — Honesty: no unplayed fixtures → next-fixture slot absent.
 *     AC20 — Honesty: empty press inbox → fallback prose, no list.
 *
 * Mocking strategy:
 *   - ~/lib/state: careerId / selectedClubId / managedClubName / seasonNumber
 *     / isCareerActive signals replaced with controllable vi.fn() getters.
 *   - ~/lib/tauri: isTauri() + getBackendHandshake() controllable.
 *   - ~/lib/api/new-career: loadCareer() controllable.
 *   - ~/lib/api/season: getStandings() + getFixtures() controllable.
 *   - ~/lib/api/career: getPressInbox() controllable.
 *   - @tauri-apps/api/core: invoke mocked to prevent throws outside Tauri.
 */

import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@solidjs/testing-library";
import { MemoryRouter, Route } from "@solidjs/router";

// ---------------------------------------------------------------------------
// Module mocks — hoisted before component imports
// ---------------------------------------------------------------------------

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue({}),
}));

vi.mock("~/lib/tauri", () => ({
  isTauri: vi.fn(),
  getBackendHandshake: vi.fn(),
}));

vi.mock("~/lib/api/new-career", () => ({
  loadCareer: vi.fn(),
  newCareer: vi.fn(),
  getClubs: vi.fn(),
  selectManagedClub: vi.fn(),
}));

vi.mock("~/lib/api/season", () => ({
  getStandings: vi.fn(),
  getFixtures: vi.fn(),
  advanceWeek: vi.fn(),
  playFixtures: vi.fn(),
}));

vi.mock("~/lib/api/career", () => ({
  getPressInbox: vi.fn(),
  getCareerOverview: vi.fn(),
  advanceSeason: vi.fn(),
}));

// State mock: we need to control the isCareerActive computed value.
// Replace with explicit stubs; the signal itself does not matter here.
vi.mock("~/lib/state", () => ({
  isCareerActive: vi.fn().mockReturnValue(false),
  managedClubName: vi.fn().mockReturnValue(null),
  selectedClubId: vi.fn().mockReturnValue(null),
  seasonNumber: vi.fn().mockReturnValue(null),
  careerId: vi.fn().mockReturnValue(null),
  setCareerId: vi.fn(),
  setSelectedClubId: vi.fn(),
  setManagedClubName: vi.fn(),
  setSeasonNumber: vi.fn(),
  theme: vi.fn().mockReturnValue("light"),
  setTheme: vi.fn(),
  reduceMotion: vi.fn().mockReturnValue(false),
  setReduceMotion: vi.fn(),
}));

// Import AFTER mocks are hoisted.
import Home from "./Home";
import { isTauri, getBackendHandshake } from "~/lib/tauri";
import { loadCareer } from "~/lib/api/new-career";
import { getStandings, getFixtures } from "~/lib/api/season";
import { getPressInbox } from "~/lib/api/career";
import {
  isCareerActive,
  managedClubName,
  selectedClubId,
  seasonNumber,
} from "~/lib/state";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function renderHome(): ReturnType<typeof render> {
  return render(() => (
    <MemoryRouter>
      <Route path="/" component={Home} />
      <Route path="/new-career" component={() => <div>club selection</div>} />
      <Route path="/squad"      component={() => <div>squad</div>} />
    </MemoryRouter>
  ));
}

/** Default standings — managed club (id 1) sits 3rd of 4. */
const STANDINGS_4 = [
  { clubId: 10, clubName: "Northgate United", played: 5, wins: 4, draws: 1, losses: 0, goalsFor: 10, goalsAgainst: 3, goalDifference: 7, points: 13 },
  { clubId: 20, clubName: "Westbrook City",   played: 5, wins: 3, draws: 1, losses: 1, goalsFor: 8,  goalsAgainst: 5, goalDifference: 3, points: 10 },
  { clubId: 1,  clubName: "Ashfield FC",      played: 5, wins: 2, draws: 1, losses: 2, goalsFor: 7,  goalsAgainst: 7, goalDifference: 0, points: 7  },
  { clubId: 30, clubName: "Morrow Athletic",  played: 5, wins: 0, draws: 0, losses: 5, goalsFor: 1,  goalsAgainst: 12, goalDifference: -11, points: 0 },
];

/** Minimal fixture list — match day 6 unplayed (away). */
const FIXTURES_WITH_NEXT = [
  { matchDay: 1, opponentClubId: 10, opponentClubName: "Northgate United", isHome: true,  played: true,  homeScore: 2, awayScore: 1 },
  { matchDay: 6, opponentClubId: 20, opponentClubName: "Westbrook City",   isHome: false, played: false },
];

/** All fixtures played. */
const FIXTURES_ALL_PLAYED = [
  { matchDay: 1, opponentClubId: 10, opponentClubName: "Northgate United", isHome: true,  played: true, homeScore: 1, awayScore: 0 },
  { matchDay: 2, opponentClubId: 20, opponentClubName: "Westbrook City",   isHome: false, played: true, homeScore: 0, awayScore: 0 },
];

/** Sample press items. */
const PRESS_ITEMS = [
  { eventId: 42, season: 1, eventClass: 100, topic: "matchResult" as const, headline: "Ashfield claim a hard-fought point at Westbrook.", managerQuote: "We showed character." },
  { eventId: 41, season: 1, eventClass: 200, topic: "playerMilestone" as const, headline: "Kane Thorpe makes his 50th appearance.", managerQuote: null },
];

// ---------------------------------------------------------------------------
// Common setup
// ---------------------------------------------------------------------------

beforeEach(() => {
  // Reset all mocks.
  vi.mocked(isTauri).mockReset();
  vi.mocked(getBackendHandshake).mockReset();
  vi.mocked(loadCareer).mockReset();
  vi.mocked(getStandings).mockReset();
  vi.mocked(getFixtures).mockReset();
  vi.mocked(getPressInbox).mockReset();
  vi.mocked(isCareerActive).mockReset();
  vi.mocked(managedClubName).mockReset();
  vi.mocked(selectedClubId).mockReset();
  vi.mocked(seasonNumber).mockReset();

  // Default: pre-career menu.
  vi.mocked(isCareerActive).mockReturnValue(false);
  vi.mocked(managedClubName).mockReturnValue(null);
  vi.mocked(selectedClubId).mockReturnValue(null);
  vi.mocked(seasonNumber).mockReturnValue(null);

  // Pre-career Tauri defaults.
  vi.mocked(isTauri).mockReturnValue(true);
  vi.mocked(getBackendHandshake).mockReturnValue(new Promise<never>(() => {}));

  // Hub API defaults — never resolves unless overridden; avoids flaky races.
  vi.mocked(getStandings).mockReturnValue(new Promise<never>(() => {}));
  vi.mocked(getFixtures).mockReturnValue(new Promise<never>(() => {}));
  vi.mocked(getPressInbox).mockReturnValue(new Promise<never>(() => {}));
});

// ---------------------------------------------------------------------------
// Pre-career: main menu tests
// ---------------------------------------------------------------------------

describe("Home — pre-career main menu", () => {
  it("AC1: renders the FINAL WHISTLE wordmark", () => {
    renderHome();
    expect(screen.getByText("FINAL WHISTLE")).toBeInTheDocument();
  });

  it("AC2: renders the tagline", () => {
    renderHome();
    expect(screen.getByText(/every career leaves a mark/i)).toBeInTheDocument();
  });

  it("AC3: NEW CAREER button is present and enabled (BK-FE-1)", () => {
    renderHome();
    const btn = screen.getByRole("button", { name: /new career/i });
    expect(btn).toBeInTheDocument();
    expect(btn).not.toBeDisabled();
  });

  it("AC4: LOAD SAVE button is present and enabled (BK-FE-2)", () => {
    renderHome();
    const btn = screen.getByRole("button", { name: /load save/i });
    expect(btn).toBeInTheDocument();
    expect(btn).not.toBeDisabled();
  });

  it("AC5: Settings link is present and points to /settings", () => {
    renderHome();
    const link = screen.getByRole("link", { name: /settings/i });
    expect(link).toBeInTheDocument();
    expect(link).toHaveAttribute("href", "/settings");
  });

  it("AC6: shows 'checking backend…' while the backend check is in flight", () => {
    renderHome();
    expect(screen.getByText(/checking backend/i)).toBeInTheDocument();
  });

  it("AC7: shows the resolved backend label on success", async () => {
    vi.mocked(getBackendHandshake).mockResolvedValue({
      appVersion: "9.9.9",
      message: "Pitch is live",
      backendReady: true,
    });
    renderHome();
    await waitFor(() => {
      expect(screen.getByText(/backend ready/i)).toBeInTheDocument();
    });
    expect(screen.getByText(/v9\.9\.9/)).toBeInTheDocument();
  });

  it("AC8: NEW CAREER click navigates to /new-career", async () => {
    renderHome();
    fireEvent.click(screen.getByRole("button", { name: /new career/i }));
    await waitFor(() => {
      expect(screen.getByText("club selection")).toBeInTheDocument();
    });
  });

  it("AC9: LOAD SAVE click calls loadCareer() and navigates to /squad on success", async () => {
    vi.mocked(loadCareer).mockResolvedValue(undefined);
    renderHome();
    fireEvent.click(screen.getByRole("button", { name: /load save/i }));
    await waitFor(() => {
      expect(vi.mocked(loadCareer)).toHaveBeenCalledTimes(1);
    });
    await waitFor(() => {
      expect(screen.getByText("squad")).toBeInTheDocument();
    });
  });

  it("AC10: LOAD SAVE shows a football-native error on saveLoadFailed", async () => {
    vi.mocked(loadCareer).mockRejectedValue({ kind: "saveLoadFailed", reason: "disk full" });
    renderHome();
    fireEvent.click(screen.getByRole("button", { name: /load save/i }));
    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });
    const alert = screen.getByRole("alert");
    // No raw error message leak.
    expect(alert.textContent).not.toContain("disk full");
    // Football-native headline from route-errors.ts.
    expect(alert.textContent).toMatch(/save couldn't be read/i);
  });

  it("AC11: does not render stale diagnostic panel content", () => {
    renderHome();
    expect(screen.queryByText("Backend handshake")).not.toBeInTheDocument();
    expect(screen.queryByText(/pinging backend/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/quick actions/i)).not.toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// Active career: hub tests
// ---------------------------------------------------------------------------

describe("Home — active-career hub", () => {
  /**
   * Helper: configure state signals for an active career with managed club id 1.
   */
  function activateCareer(): void {
    vi.mocked(isCareerActive).mockReturnValue(true);
    vi.mocked(managedClubName).mockReturnValue("Ashfield FC");
    vi.mocked(selectedClubId).mockReturnValue("1");
    vi.mocked(seasonNumber).mockReturnValue(1);
  }

  it("AC12: hub renders; main-menu wordmark and NEW CAREER button are absent", async () => {
    activateCareer();
    vi.mocked(getStandings).mockResolvedValue(STANDINGS_4);
    vi.mocked(getFixtures).mockResolvedValue(FIXTURES_WITH_NEXT);
    vi.mocked(getPressInbox).mockResolvedValue({ seasonNumber: 1, items: [] });
    renderHome();
    // Hub heading (club name) present.
    await waitFor(() => expect(screen.getByRole("heading", { level: 1 })).toBeInTheDocument());
    // Pre-career elements absent.
    expect(screen.queryByText("FINAL WHISTLE")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /new career/i })).not.toBeInTheDocument();
  });

  it("AC13: bite header shows the managed club name from the signal", async () => {
    activateCareer();
    vi.mocked(getStandings).mockResolvedValue([]);
    vi.mocked(getFixtures).mockResolvedValue([]);
    vi.mocked(getPressInbox).mockResolvedValue({ seasonNumber: 1, items: [] });
    renderHome();
    await waitFor(() => {
      expect(screen.getByRole("heading", { level: 1, name: /ashfield fc/i })).toBeInTheDocument();
    });
  });

  it("AC14: bite header shows league position as an ordinal (3rd of 4)", async () => {
    activateCareer();
    vi.mocked(getStandings).mockResolvedValue(STANDINGS_4);
    vi.mocked(getFixtures).mockResolvedValue([]);
    vi.mocked(getPressInbox).mockResolvedValue({ seasonNumber: 1, items: [] });
    renderHome();
    await waitFor(() => {
      // "3rd" must appear somewhere on the page.
      expect(screen.getByText(/3rd/i)).toBeInTheDocument();
    });
    // "of 4" also present.
    expect(screen.getByText(/of 4/i)).toBeInTheDocument();
  });

  it("AC15: bite header shows next fixture — opponent, home/away tag, match day", async () => {
    activateCareer();
    vi.mocked(getStandings).mockResolvedValue(STANDINGS_4);
    vi.mocked(getFixtures).mockResolvedValue(FIXTURES_WITH_NEXT);
    vi.mocked(getPressInbox).mockResolvedValue({ seasonNumber: 1, items: [] });
    renderHome();
    await waitFor(() => {
      expect(screen.getByText(/westbrook city/i)).toBeInTheDocument();
    });
    // Away marker — the fixture is away. The "@" and opponent name are in the
    // same <p> as sibling text nodes. Check that the "Next fixture" label
    // paragraph is present, then verify the opponent paragraph's textContent.
    expect(screen.getByText(/next fixture/i)).toBeInTheDocument();
    // The opponent <p> contains "@" (away indicator) + the club name.
    const opponentPara = screen.getByText(/westbrook city/i).closest("p");
    expect(opponentPara?.textContent).toMatch(/@/);
    // Match day label.
    expect(screen.getByText(/match day 6/i)).toBeInTheDocument();
  });

  it("AC16: press feed renders items from getPressInbox()", async () => {
    activateCareer();
    vi.mocked(getStandings).mockResolvedValue([]);
    vi.mocked(getFixtures).mockResolvedValue([]);
    vi.mocked(getPressInbox).mockResolvedValue({ seasonNumber: 1, items: PRESS_ITEMS });
    renderHome();
    await waitFor(() => {
      expect(screen.getByText(/ashfield claim a hard-fought point/i)).toBeInTheDocument();
    });
    expect(screen.getByText(/kane thorpe makes his 50th appearance/i)).toBeInTheDocument();
    // Manager quote rendered where present.
    expect(screen.getByText(/we showed character/i)).toBeInTheDocument();
  });

  it("AC17: quick-link cards present for Squad, League, Fixtures, Next match", async () => {
    activateCareer();
    vi.mocked(getStandings).mockResolvedValue([]);
    vi.mocked(getFixtures).mockResolvedValue([]);
    vi.mocked(getPressInbox).mockResolvedValue({ seasonNumber: 1, items: [] });
    renderHome();
    await waitFor(() => {
      expect(screen.getByRole("link", { name: /squad/i })).toBeInTheDocument();
    });
    expect(screen.getByRole("link", { name: /league/i })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /fixtures/i })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /next match/i })).toBeInTheDocument();
  });

  it("AC18: honesty — league position slot is ABSENT when standings return empty", async () => {
    activateCareer();
    vi.mocked(getStandings).mockResolvedValue([]);
    vi.mocked(getFixtures).mockResolvedValue([]);
    vi.mocked(getPressInbox).mockResolvedValue({ seasonNumber: 1, items: [] });
    renderHome();
    // Wait for loading to finish (press section loads with empty state).
    await waitFor(() => {
      expect(screen.getByText(/nothing in the press yet/i)).toBeInTheDocument();
    });
    // No ordinal like "1st", "2nd" etc. — position slot must be omitted entirely.
    expect(screen.queryByText(/\d+(st|nd|rd|th)/)).not.toBeInTheDocument();
    // "League position" label must not appear anywhere.
    expect(screen.queryByText(/league position/i)).not.toBeInTheDocument();
  });

  it("AC19: honesty — next-fixture slot is ABSENT when all fixtures are played", async () => {
    activateCareer();
    vi.mocked(getStandings).mockResolvedValue(STANDINGS_4);
    vi.mocked(getFixtures).mockResolvedValue(FIXTURES_ALL_PLAYED);
    vi.mocked(getPressInbox).mockResolvedValue({ seasonNumber: 1, items: [] });
    renderHome();
    await waitFor(() => {
      // League position present (standings loaded) — managed club is 3rd.
      expect(screen.getByText(/3rd/i)).toBeInTheDocument();
    });
    // Next fixture label must not appear — the slot is omitted entirely.
    expect(screen.queryByText(/next fixture/i)).not.toBeInTheDocument();
    // No opponent club names from the unplayed fixture list visible.
    expect(screen.queryByText(/westbrook city/i)).not.toBeInTheDocument();
  });

  it("AC20: honesty — empty press inbox shows fallback prose, not an empty list", async () => {
    activateCareer();
    vi.mocked(getStandings).mockResolvedValue([]);
    vi.mocked(getFixtures).mockResolvedValue([]);
    vi.mocked(getPressInbox).mockResolvedValue({ seasonNumber: 1, items: [] });
    renderHome();
    await waitFor(() => {
      expect(
        screen.getByText(/nothing in the press yet/i),
      ).toBeInTheDocument();
    });
    // No list items rendered.
    expect(screen.queryAllByRole("listitem")).toHaveLength(0);
  });
});

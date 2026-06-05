/*
 * ClubSelection page — Vitest tests (B4).
 *
 * Substance requirements:
 *   AC1 — on mount, calls newCareer(seedHex) then getClubs().
 *   AC2 — renders a listbox with 20 club options after setup.
 *   AC3 — loading state renders the "Generating league…" message.
 *   AC4 — setup IPC error shows a football-native alert (no raw err.message).
 *   AC5 — clicking a club calls selectManagedClub(clubId) then navigates to /squad.
 *   AC6 — selectManagedClub failure shows a football-native error alert.
 *   AC7 — the seed line renders (world seed).
 *   AC8 — keyboard: ArrowDown moves focus; Enter selects the focused item.
 *
 * Mocking strategy:
 *   - ~/lib/api/new-career is mocked globally; each test configures per-function
 *     mock behaviour via vi.mocked().
 *   - ~/lib/state is mocked so setCareerId / setSelectedClubId / setManagedClubName
 *     calls don't touch real reactive state in the test environment.
 *   - @solidjs/router's useLocation is mocked to inject the seed state;
 *     useNavigate returns a spy so navigation can be asserted.
 *   - @tauri-apps/api/core invoke mocked so nothing throws outside Tauri.
 */

import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@solidjs/testing-library";
import type { ClubChoiceDto } from "~/lib/types";

// ---------------------------------------------------------------------------
// Module mocks — hoisted before component import
// ---------------------------------------------------------------------------

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue({}),
}));

vi.mock("~/lib/api/new-career", () => ({
  newCareer: vi.fn(),
  getClubs: vi.fn(),
  selectManagedClub: vi.fn(),
  loadCareer: vi.fn(),
  saveCareer: vi.fn(),
}));

vi.mock("~/lib/state", () => ({
  setCareerId: vi.fn(),
  setSelectedClubId: vi.fn(),
  setManagedClubName: vi.fn(),
  setSeasonNumber: vi.fn(),
  selectedClubId: vi.fn(() => null),
  careerId: vi.fn(() => null),
  managedClubName: vi.fn(() => null),
  seasonNumber: vi.fn(() => null),
  isCareerActive: vi.fn(() => false),
  theme: vi.fn(() => "light"),
  setTheme: vi.fn(),
  reduceMotion: vi.fn(() => false),
  setReduceMotion: vi.fn(),
}));

// Mock router hooks — useLocation returns a fixed seed state; useNavigate
// returns a spy so navigation can be asserted without a real router.
const navigateSpy = vi.fn();
vi.mock("@solidjs/router", () => ({
  useLocation: vi.fn(() => ({
    state: { seedHex: "0xdeadbeefdeadbeef" },
    pathname: "/new-career",
    search: "",
    hash: "",
    query: {},
  })),
  useNavigate: vi.fn(() => navigateSpy),
  // Minimal A stub — the ClubSelection component does not render <A> in a path
  // that matters for these tests (SeedRow uses <button>, not <A>).
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  A: (_props: { href: string; children: unknown }) => null,
}));

// Import AFTER mocks are hoisted.
import ClubSelection from "./ClubSelection";
import {
  newCareer,
  getClubs,
  selectManagedClub,
} from "~/lib/api/new-career";
import { setCareerId, setSelectedClubId, setManagedClubName } from "~/lib/state";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const FIXTURE_CLUBS: ClubChoiceDto[] = Array.from({ length: 20 }, (_, i) => ({
  clubId: i + 1,
  clubName: `Club ${i + 1}`,
}));

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("ClubSelection page (B4)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    navigateSpy.mockReset();
    vi.mocked(newCareer).mockResolvedValue(undefined);
    vi.mocked(getClubs).mockResolvedValue(FIXTURE_CLUBS);
    vi.mocked(selectManagedClub).mockResolvedValue(undefined);
  });

  // AC1: on mount calls newCareer then getClubs.
  it("calls newCareer() then getClubs() on mount (AC1)", async () => {
    render(() => <ClubSelection />);

    await waitFor(() => {
      expect(vi.mocked(newCareer)).toHaveBeenCalledTimes(1);
    });
    expect(vi.mocked(newCareer)).toHaveBeenCalledWith("0xdeadbeefdeadbeef");

    await waitFor(() => {
      expect(vi.mocked(getClubs)).toHaveBeenCalledTimes(1);
    });
  });

  // AC2: renders 20 club options.
  it("renders 20 club options in a listbox (AC2)", async () => {
    render(() => <ClubSelection />);

    await waitFor(() => {
      expect(screen.getByRole("listbox")).toBeInTheDocument();
    });

    const options = screen.getAllByRole("option");
    expect(options).toHaveLength(20);
    expect(options[0]?.textContent).toContain("Club 1");
    expect(options[19]?.textContent).toContain("Club 20");
  });

  // AC3: loading state.
  it("shows 'Generating league…' while setup is in flight (AC3)", () => {
    vi.mocked(newCareer).mockImplementation(
      () => new Promise(() => { /* pending */ }),
    );
    render(() => <ClubSelection />);
    expect(screen.getByText(/generating league/i)).toBeInTheDocument();
  });

  // AC4: setup IPC error — football-native alert.
  it("shows football-native error on leagueGenerationFailed (AC4)", async () => {
    vi.mocked(newCareer).mockRejectedValue({
      kind: "leagueGenerationFailed",
      reason: "content store error",
    });

    render(() => <ClubSelection />);

    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });

    const alert = screen.getByRole("alert");
    // Raw reason must not leak.
    expect(alert.textContent).not.toContain("content store error");
    // Football-native copy from describeRouteError.
    expect(alert.textContent).toMatch(/league couldn't be assembled/i);
  });

  // AC5: clicking a club calls selectManagedClub and navigates.
  it("clicking a club calls selectManagedClub(clubId) then navigates to /squad (AC5)", async () => {
    render(() => <ClubSelection />);

    await waitFor(() => {
      expect(screen.getByRole("listbox")).toBeInTheDocument();
    });

    const options = screen.getAllByRole("option");
    const firstClub = options[0];
    if (!firstClub) throw new Error("fixture: no club options rendered");

    fireEvent.click(firstClub);

    await waitFor(() => {
      expect(vi.mocked(selectManagedClub)).toHaveBeenCalledWith(1);
    });

    await waitFor(() => {
      expect(navigateSpy).toHaveBeenCalledWith("/squad");
    });

    // State was updated.
    expect(vi.mocked(setCareerId)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(setSelectedClubId)).toHaveBeenCalledWith("1");
    expect(vi.mocked(setManagedClubName)).toHaveBeenCalledWith("Club 1");
  });

  // AC6: selectManagedClub failure shows football-native error.
  it("shows football-native error when selectManagedClub rejects with clubNotFound (AC6)", async () => {
    vi.mocked(selectManagedClub).mockRejectedValue({
      kind: "clubNotFound",
      // Use an id that doesn't appear as a word in the football-native copy.
      clubId: 55555,
    });

    render(() => <ClubSelection />);

    await waitFor(() => {
      expect(screen.getByRole("listbox")).toBeInTheDocument();
    });

    const options = screen.getAllByRole("option");
    const firstClub = options[0];
    if (!firstClub) throw new Error("fixture: no club options rendered");

    fireEvent.click(firstClub);

    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });

    // Football-native copy from describeRouteError clubNotFound.
    const alert = screen.getByRole("alert");
    expect(alert.textContent).toMatch(/club isn't on the league's books/i);
    // describeRouteError embeds the clubId in the detail — that's correct and football-native.
    // What we guard against is raw technical exception strings (err.message, stack traces).
    expect(alert.textContent).not.toContain("Cannot read properties");
    expect(alert.textContent).not.toContain("invoke");
  });

  // AC7: seed line renders.
  it("renders the world seed line (AC7)", async () => {
    render(() => <ClubSelection />);

    await waitFor(() => {
      expect(screen.getByRole("listbox")).toBeInTheDocument();
    });

    expect(screen.getByText(/world seed/i)).toBeInTheDocument();
    expect(screen.getByText(/0xdeadbeefdeadbeef/)).toBeInTheDocument();
  });

  // AC8: keyboard ArrowDown + Enter.
  it("ArrowDown then Enter selects the second club (AC8)", async () => {
    render(() => <ClubSelection />);

    await waitFor(() => {
      expect(screen.getByRole("listbox")).toBeInTheDocument();
    });

    const listbox = screen.getByRole("listbox");

    // Move focus down to index 1.
    fireEvent.keyDown(listbox, { key: "ArrowDown" });
    // Enter selects the focused item (Club 2, clubId: 2).
    fireEvent.keyDown(listbox, { key: "Enter" });

    await waitFor(() => {
      expect(vi.mocked(selectManagedClub)).toHaveBeenCalledWith(2);
    });
  });

  // Accessibility: re-roll button has an aria-label.
  it("re-roll button has an accessible label", async () => {
    render(() => <ClubSelection />);

    await waitFor(() => {
      expect(screen.getByRole("listbox")).toBeInTheDocument();
    });

    const reroll = screen.getByRole("button", { name: /re-roll world seed/i });
    expect(reroll).toBeInTheDocument();
  });
});

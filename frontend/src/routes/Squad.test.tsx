/*
 * Squad page — Vitest tests (T4-2.5h).
 *
 * Substance requirements:
 *   AC1 — 5-column table renders correct headers (Player, Role, Apps, Goals, Minutes).
 *   AC2 — roster data sourced from IPC via getSquadRoster() on mount.
 *   AC3 — rows render from a fixture; stats are displayed correctly.
 *   AC4 — loading, IPC error, and empty states render correctly.
 *   AC5 — club name placeholder shown in the sub-header.
 *
 * Mocking strategy:
 *   - ~/lib/api/squad is mocked globally. Each test configures per-function
 *     mock behaviour via vi.mocked().
 *   - @tauri-apps/api/core mocked so invoke() never throws outside Tauri.
 */

import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@solidjs/testing-library";
import type { SquadRosterDto } from "~/lib/types";

// ---------------------------------------------------------------------------
// Module mocks — hoisted before component import
// ---------------------------------------------------------------------------

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue({}),
}));

vi.mock("~/lib/api/squad", () => ({
  getSquad: vi.fn(),
  getSquadRoster: vi.fn(),
}));

// Import AFTER mocks are hoisted.
import Squad from "./Squad";
import { getSquadRoster } from "~/lib/api/squad";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const FIXTURE_ROSTER: SquadRosterDto = {
  clubId: 1,
  clubName: "Ashvale United",
  isManaged: false,
  players: [
    {
      playerId: 1000000,
      name: "Emeka Thorne",
      clubId: 1,
      slot: 0,
      appearances: 3,
      goals: 0,
      assists: 0,
      minutesPlayed: 270,
    },
    {
      playerId: 1000001,
      name: "Seren Voss",
      clubId: 1,
      slot: 5,
      appearances: 3,
      goals: 2,
      assists: 1,
      minutesPlayed: 270,
    },
    {
      playerId: 1000002,
      name: "Orin Dake",
      clubId: 1,
      slot: 9,
      appearances: 1,
      goals: 1,
      assists: 0,
      minutesPlayed: 90,
    },
    {
      // Bench/depth slot (≥11). Must render role "Sub", NOT a fabricated
      // formation position (the T4-2.5h self-review P1 fix — slots 11–21 are
      // this club's reserves, not a second team's XI).
      playerId: 1000011,
      name: "Bex Harlow",
      clubId: 1,
      slot: 11,
      appearances: 0,
      goals: 0,
      assists: 0,
      minutesPlayed: 0,
    },
  ],
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Squad page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getSquadRoster).mockResolvedValue(FIXTURE_ROSTER);
  });

  // AC1: correct column headers (5 columns total).
  it("renders five column headers: Player, Role, Apps, Goals, Minutes", async () => {
    render(() => <Squad />);

    await waitFor(() => {
      expect(screen.getAllByRole("columnheader")).toHaveLength(5);
    });

    const headers = screen.getAllByRole("columnheader");
    const headerTexts = headers.map((h) => h.textContent?.trim() ?? "");

    expect(headerTexts).toContain("Player");
    expect(headerTexts).toContain("Role");
    expect(headerTexts).toContain("Apps");
    expect(headerTexts).toContain("Goals");
    expect(headerTexts).toContain("Minutes");
  });

  // AC2: data sourced from IPC on mount.
  it("calls getSquadRoster() on mount and renders player names", async () => {
    render(() => <Squad />);

    await waitFor(() => {
      expect(screen.getByText("Emeka Thorne")).toBeInTheDocument();
    });

    expect(screen.getByText("Seren Voss")).toBeInTheDocument();
    expect(screen.getByText("Orin Dake")).toBeInTheDocument();
    expect(vi.mocked(getSquadRoster)).toHaveBeenCalledTimes(1);
  });

  // AC3a: rows render stats correctly.
  it("renders appearances, goals, and minutes from fixture data", async () => {
    render(() => <Squad />);

    await waitFor(() => {
      expect(screen.getByText("Emeka Thorne")).toBeInTheDocument();
    });

    // Seren Voss: goals == 2, appearances == 3, minutes == 270.
    // Multiple cells with same value — check at least one occurrence.
    const goalCells = screen.getAllByText("2");
    expect(goalCells.length).toBeGreaterThan(0);

    const minuteCells = screen.getAllByText("270");
    expect(minuteCells.length).toBeGreaterThan(0);

    // Orin Dake: minutes == 90.
    expect(screen.getByText("90")).toBeInTheDocument();
  });

  // AC3b: role derived from slot correctly.
  it("renders role derived from slot (GK/MID/FWD for the XI; Sub for the bench)", async () => {
    render(() => <Squad />);

    await waitFor(() => {
      expect(screen.getByText("Emeka Thorne")).toBeInTheDocument();
    });

    // slot 0 → GK, slot 5 → MID, slot 9 → FWD.
    expect(screen.getByText("GK")).toBeInTheDocument();
    expect(screen.getByText("MID")).toBeInTheDocument();
    expect(screen.getByText("FWD")).toBeInTheDocument();
    // slot 11 (bench/depth) → "Sub", NOT a fabricated formation position
    // (T4-2.5h self-review P1: no slot-11 away-shift on a single-club roster).
    expect(screen.getByText("Sub")).toBeInTheDocument();
  });

  // AC4a: loading state shows fallback copy.
  it("shows loading fallback before roster resolves", () => {
    vi.mocked(getSquadRoster).mockImplementation(
      () => new Promise(() => {/* pending */}),
    );

    render(() => <Squad />);

    expect(screen.getByText(/loading squad/i)).toBeInTheDocument();
  });

  // AC4b: IPC error shows the error alert with football-native copy.
  it("shows error state when getSquadRoster rejects with IpcError", async () => {
    vi.mocked(getSquadRoster).mockRejectedValue({
      kind: "lockPoisoned",
      lock: "career",
    });

    render(() => <Squad />);

    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });

    const alert = screen.getByRole("alert");
    expect(alert.textContent).not.toContain("Cannot read properties");
    // The lockPoisoned copy mentions the lock name in the detail.
    expect(alert.textContent).toContain("career");
  });

  // AC4b-extra: error state must NOT show raw technical exception strings.
  it("error alert does not contain raw err.message on generic failure", async () => {
    vi.mocked(getSquadRoster).mockRejectedValue(
      new Error("Cannot read properties of undefined (reading 'invoke')"),
    );

    render(() => <Squad />);

    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });

    const alert = screen.getByRole("alert");
    expect(alert.textContent).not.toContain("Cannot read properties");
    expect(alert.textContent).not.toContain("invoke");
  });

  // AC5: club name placeholder shown in sub-header.
  it("shows club name placeholder in sub-header when roster loads", async () => {
    render(() => <Squad />);

    await waitFor(() => {
      expect(screen.getByText("Emeka Thorne")).toBeInTheDocument();
    });

    expect(screen.getByText("Ashvale United")).toBeInTheDocument();
    // The placeholder text pattern.
    expect(
      screen.getByText(/no club selected yet/i),
    ).toBeInTheDocument();
  });
});

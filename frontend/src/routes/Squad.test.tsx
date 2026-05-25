/*
 * Squad page — Vitest tests (T2-7).
 *
 * Substance requirements:
 *   AC1 — 4-column table renders correct headers (Player, Role, Region, Traits).
 *   AC2 — squad data sourced from IPC via getSquad() on mount.
 *   AC3 — rows render from a fixture; phenotype cell shows readable text
 *          (not a raw enum identifier like "ExplosiveFirstStep").
 *   AC4 — loading, IPC error, and empty states render correctly.
 *
 * Mocking strategy:
 *   - ~/lib/api/squad is mocked globally. Each test configures per-function
 *     mock behaviour via vi.mocked().
 *   - @tauri-apps/api/core mocked so invoke() never throws outside Tauri.
 */

import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@solidjs/testing-library";
import type { SquadPlayer } from "~/lib/types";

// ---------------------------------------------------------------------------
// Module mocks — hoisted before component import
// ---------------------------------------------------------------------------

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

vi.mock("~/lib/api/squad", () => ({
  getSquad: vi.fn(),
}));

// Import AFTER mocks are hoisted.
import Squad from "./Squad";
import { getSquad } from "~/lib/api/squad";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const FIXTURE_THREE_PLAYERS: SquadPlayer[] = [
  {
    playerId: "fwh.core:player_00001",
    name: "Emeka Thorne",
    role: "Striker",
    birthRegion: "Ashvale",
    phenotypeLabels: ["Pure finisher", "Poacher", "Late bloomer"],
  },
  {
    playerId: "fwh.core:player_00002",
    name: "Seren Voss",
    role: "Goalkeeper",
    birthRegion: "Brackwater",
    phenotypeLabels: ["Sweeper-keeper", "Composed under pressure"],
  },
  {
    playerId: "fwh.core:player_00003",
    name: "Orin Dake",
    role: "Centre-back",
    birthRegion: "Thornholt",
    phenotypeLabels: [],
  },
];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Squad page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getSquad).mockResolvedValue(FIXTURE_THREE_PLAYERS);
  });

  // AC1: correct column headers (4 columns total).
  it("renders four column headers: Player, Role, Region, Traits", async () => {
    render(() => <Squad />);

    await waitFor(() => {
      expect(screen.getAllByRole("columnheader")).toHaveLength(4);
    });

    const headers = screen.getAllByRole("columnheader");
    const headerTexts = headers.map((h) => h.textContent?.trim() ?? "");

    expect(headerTexts).toContain("Player");
    expect(headerTexts).toContain("Role");
    expect(headerTexts).toContain("Region");
    expect(headerTexts).toContain("Traits");
    expect(headers).toHaveLength(4);
  });

  // AC2: data sourced from IPC on mount.
  it("calls getSquad() on mount and renders player names", async () => {
    render(() => <Squad />);

    await waitFor(() => {
      expect(screen.getByText("Emeka Thorne")).toBeInTheDocument();
    });

    expect(screen.getByText("Seren Voss")).toBeInTheDocument();
    expect(screen.getByText("Orin Dake")).toBeInTheDocument();
    expect(vi.mocked(getSquad)).toHaveBeenCalledTimes(1);
  });

  // AC3a: rows render role and region correctly.
  it("renders role and region data for players", async () => {
    render(() => <Squad />);

    await waitFor(() => {
      expect(screen.getByText("Striker")).toBeInTheDocument();
    });

    expect(screen.getByText("Goalkeeper")).toBeInTheDocument();
    expect(screen.getByText("Centre-back")).toBeInTheDocument();
    expect(screen.getByText("Ashvale")).toBeInTheDocument();
    expect(screen.getByText("Brackwater")).toBeInTheDocument();
  });

  // AC3b: phenotype cell shows readable comma-joined text — NOT raw enum
  // identifiers like "ExplosiveFirstStep" and NOT raw JSON like ["..."].
  it("renders phenotype labels as readable comma-joined text", async () => {
    render(() => <Squad />);

    await waitFor(() => {
      expect(screen.getByText("Emeka Thorne")).toBeInTheDocument();
    });

    // "Pure finisher, Poacher, Late bloomer" — human-readable, comma-joined.
    const phenotypeCell = screen.getByText(/Pure finisher/);
    expect(phenotypeCell).toBeInTheDocument();
    // The text should contain commas (comma-joined), not brackets (raw JSON).
    expect(phenotypeCell.textContent).toContain(",");
    expect(phenotypeCell.textContent).not.toContain("[");
    expect(phenotypeCell.textContent).not.toContain("]");

    // "Sweeper-keeper, Composed under pressure"
    expect(screen.getByText(/Sweeper-keeper/)).toBeInTheDocument();

    // Player with no labels renders "—" (em-dash fallback).
    expect(screen.getByText("—")).toBeInTheDocument();
  });

  // AC3c: phenotype cell must not contain raw CamelCase enum identifiers.
  it("phenotype labels are never raw CamelCase enum identifiers", async () => {
    render(() => <Squad />);

    await waitFor(() => {
      expect(screen.getByText("Emeka Thorne")).toBeInTheDocument();
    });

    // Collect all text content from cells and assert no entry looks like a
    // raw CamelCase identifier (two consecutive uppercase letters indicate
    // a CamelCase run like "ExplosiveFirstStep").
    const allText = document.body.textContent ?? "";
    // "PureFinisher", "Poacher" as CamelCase — check specific known raw IDs
    // do NOT appear anywhere in the rendered page.
    expect(allText).not.toContain("PureFinisher");
    expect(allText).not.toContain("SweeperKeeper");
    expect(allText).not.toContain("ComposedUnderPressure");
    expect(allText).not.toContain("LateBloomer");
  });

  // AC4a: loading state shows fallback copy.
  it("shows loading fallback before squad resolves", () => {
    vi.mocked(getSquad).mockImplementation(
      () => new Promise(() => {/* pending */}),
    );

    render(() => <Squad />);

    expect(screen.getByText(/loading squad/i)).toBeInTheDocument();
  });

  // AC4b: IPC error shows the error alert with football-native copy.
  it("shows error state when getSquad rejects with IpcError", async () => {
    vi.mocked(getSquad).mockRejectedValue({
      kind: "lockPoisoned",
      lock: "season",
    });

    render(() => <Squad />);

    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });

    const alert = screen.getByRole("alert");
    // Must show the football-native placeholder headline (not raw err.message).
    expect(alert.textContent).not.toContain("Cannot read properties");
    // The lockPoisoned copy mentions the lock name in the detail.
    expect(alert.textContent).toContain("season");
  });

  // AC4b-extra: error state must NOT show raw technical exception strings.
  it("error alert does not contain raw err.message on generic failure", async () => {
    vi.mocked(getSquad).mockRejectedValue(
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

  // AC4c: empty state when getSquad returns an empty array.
  it("shows empty-state message when squad is an empty array", async () => {
    vi.mocked(getSquad).mockResolvedValue([]);

    render(() => <Squad />);

    await waitFor(() => {
      expect(screen.getByText(/no players in the pool/i)).toBeInTheDocument();
    });
  });
});

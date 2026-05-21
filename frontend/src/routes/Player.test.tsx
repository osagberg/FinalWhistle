/*
 * Player detail page — Vitest tests (T3-6).
 *
 * AC coverage:
 *   AC1 — Three blocks render: phenotype, career moments, contract.
 *   AC2 — getPlayerDetail() called on mount with the route param id.
 *   AC3 — Phenotype labels + memory callback strings render as text.
 *   AC4 — Loading state shows fallback copy.
 *   AC5 — IPC error state shows role="alert" with error text.
 *   AC6 — Empty memoryCallbacks shows "No notable career moments yet."
 *
 * Mocking strategy:
 *   - ~/lib/api/player is mocked globally. Tests configure per-mock behaviour
 *     via vi.mocked().
 *   - @solidjs/router's useParams is mocked to return a stable id.
 *   - @tauri-apps/api/core mocked so invoke() never throws outside Tauri.
 */

import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@solidjs/testing-library";
import type { PlayerDetail } from "~/lib/types";

// ---------------------------------------------------------------------------
// Module mocks — hoisted before component import
// ---------------------------------------------------------------------------

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(null),
}));

vi.mock("~/lib/api/player", () => ({
  getPlayerDetail: vi.fn(),
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Player detail page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getPlayerDetail).mockResolvedValue(FIXTURE_PLAYER_DETAIL);
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

  // AC3b: phenotype labels render as readable text (comma-joined, not raw enum IDs).
  it("renders scout traits as comma-joined human-readable text", async () => {
    render(() => <Player />);

    await waitFor(() => {
      expect(screen.getByText(/pure finisher/i)).toBeInTheDocument();
    });

    const traitsText = screen.getByText(/pure finisher/i).textContent ?? "";
    expect(traitsText).toContain(",");
    expect(traitsText).not.toContain("[");
    expect(traitsText).not.toContain("]");
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

  // AC5a: IPC error (lockPoisoned) shows role="alert".
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
    expect(alert.textContent).toContain("Failed to load player");
    expect(alert.textContent).toContain("memory_ledger");
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
});

/*
 * Home page — Vitest tests (B4 update).
 *
 * The Home route is the branded main-menu landing. Test coverage:
 *   AC1 — wordmark "FINAL WHISTLE" renders.
 *   AC2 — tagline renders.
 *   AC3 — "NEW CAREER" button renders and is ENABLED (was disabled, BK-FE-1).
 *   AC4 — "LOAD SAVE" button renders and is ENABLED (was disabled, BK-FE-2).
 *   AC5 — Settings link renders and points to /settings.
 *   AC6 — Backend status line shows "checking backend…" while pending, then
 *          the resolved label.
 *   AC7 — NEW CAREER click navigates to /new-career with a seedHex in state.
 *   AC8 — LOAD SAVE click calls loadCareer(); on success navigates to /squad.
 *   AC9 — LOAD SAVE shows an error line when loadCareer() rejects with
 *          IpcError::saveLoadFailed — football-native copy, no raw message.
 *
 * Mocking strategy:
 *   - ~/lib/tauri is mocked so each test controls isTauri() + getBackendHandshake().
 *   - ~/lib/api/new-career is mocked so loadCareer() is controllable.
 *   - @tauri-apps/api/core invoke is mocked so nothing throws outside Tauri.
 *   - @solidjs/router: real MemoryRouter for navigation assertions.
 */

import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@solidjs/testing-library";
import { MemoryRouter, Route } from "@solidjs/router";

// ---------------------------------------------------------------------------
// Module mocks — hoisted before component import
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

// Import AFTER mocks are hoisted.
import Home from "./Home";
import { isTauri, getBackendHandshake } from "~/lib/tauri";
import { loadCareer } from "~/lib/api/new-career";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function renderHome(): ReturnType<typeof render> {
  // Use MemoryRouter with stub routes for /new-career and /squad.
  return render(() => (
    <MemoryRouter>
      <Route path="/" component={Home} />
      <Route
        path="/new-career"
        component={() => <div>club selection</div>}
      />
      <Route
        path="/squad"
        component={() => <div>squad</div>}
      />
    </MemoryRouter>
  ));
}

beforeEach(() => {
  vi.mocked(isTauri).mockReset();
  vi.mocked(getBackendHandshake).mockReset();
  vi.mocked(loadCareer).mockReset();
  vi.mocked(isTauri).mockReturnValue(true);
  vi.mocked(getBackendHandshake).mockReturnValue(new Promise<never>(() => {}));
});

describe("Home — main menu (B4)", () => {
  it("renders the FINAL WHISTLE wordmark", () => {
    renderHome();
    expect(screen.getByText("FINAL WHISTLE")).toBeInTheDocument();
  });

  it("renders the tagline", () => {
    renderHome();
    expect(
      screen.getByText(/every career leaves a mark/i),
    ).toBeInTheDocument();
  });

  it("renders the NEW CAREER button and it is ENABLED (BK-FE-1)", () => {
    renderHome();
    const btn = screen.getByRole("button", { name: /new career/i });
    expect(btn).toBeInTheDocument();
    expect(btn).not.toBeDisabled();
  });

  it("renders the LOAD SAVE button and it is ENABLED (BK-FE-2)", () => {
    renderHome();
    const btn = screen.getByRole("button", { name: /load save/i });
    expect(btn).toBeInTheDocument();
    expect(btn).not.toBeDisabled();
  });

  it("renders the Settings link pointing to /settings", () => {
    renderHome();
    const link = screen.getByRole("link", { name: /settings/i });
    expect(link).toBeInTheDocument();
    expect(link).toHaveAttribute("href", "/settings");
  });

  it("shows the pending status line while the backend check is in flight", () => {
    renderHome();
    expect(screen.getByText(/checking backend/i)).toBeInTheDocument();
  });

  it("shows the resolved backend label on success (Tauri path)", async () => {
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
    expect(getBackendHandshake).toHaveBeenCalledTimes(1);
  });

  it("shows the browser-stub label when not running in Tauri", async () => {
    vi.mocked(isTauri).mockReturnValue(false);

    renderHome();

    await waitFor(() => {
      expect(
        screen.getByText(/browser preview — no Tauri backend/i),
      ).toBeInTheDocument();
    });
    expect(getBackendHandshake).not.toHaveBeenCalled();
  });

  it("NEW CAREER click navigates to /new-career (BK-FE-3)", async () => {
    renderHome();
    const btn = screen.getByRole("button", { name: /new career/i });
    fireEvent.click(btn);

    await waitFor(() => {
      expect(screen.getByText("club selection")).toBeInTheDocument();
    });
  });

  it("LOAD SAVE click calls loadCareer() and navigates to /squad on success", async () => {
    vi.mocked(loadCareer).mockResolvedValue(undefined);

    renderHome();
    const btn = screen.getByRole("button", { name: /load save/i });
    fireEvent.click(btn);

    await waitFor(() => {
      expect(vi.mocked(loadCareer)).toHaveBeenCalledTimes(1);
    });

    await waitFor(() => {
      expect(screen.getByText("squad")).toBeInTheDocument();
    });
  });

  it("LOAD SAVE shows a football-native error on saveLoadFailed", async () => {
    vi.mocked(loadCareer).mockRejectedValue({
      kind: "saveLoadFailed",
      reason: "disk full",
    });

    renderHome();
    const btn = screen.getByRole("button", { name: /load save/i });
    fireEvent.click(btn);

    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });

    const alert = screen.getByRole("alert");
    // Football-native copy from describeRouteError — no raw "disk full" leak.
    expect(alert.textContent).not.toContain("disk full");
    // The saveLoadFailed headline from route-errors.ts.
    expect(alert.textContent).toMatch(/save couldn't be read/i);
  });

  it("does not render old diagnostic panel content", () => {
    renderHome();
    expect(screen.queryByText("Backend handshake")).not.toBeInTheDocument();
    expect(screen.queryByText(/pinging backend/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/quick actions/i)).not.toBeInTheDocument();
  });
});

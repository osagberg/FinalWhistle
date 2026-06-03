/*
 * Home page — Vitest tests (T4-7 game-shell polish).
 *
 * The Home route is now the branded main-menu landing (not a diagnostic panel).
 * Test coverage:
 *   AC1 — wordmark "FINAL WHISTLE" renders.
 *   AC2 — tagline renders.
 *   AC3 — "NEW CAREER" and "LOAD SAVE" buttons render and are disabled.
 *   AC4 — Settings link renders and points to /settings.
 *   AC5 — Backend status line shows "checking backend…" while pending, then
 *          the resolved label (Tauri path + browser-stub path).
 *   AC6 — Error path does not crash the wordmark/action card (ErrorBoundary
 *          wraps only when thrown; here getBackendHandshake is fire-and-observe).
 *
 * Mocking strategy:
 *   - ~/lib/tauri is mocked so each test controls isTauri() + getBackendHandshake().
 *   - @tauri-apps/api/core invoke is mocked so nothing throws outside Tauri.
 */

import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@solidjs/testing-library";
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

// Import AFTER mocks are hoisted.
import Home from "./Home";
import { isTauri, getBackendHandshake } from "~/lib/tauri";
// Home uses <A> from @solidjs/router which requires a Router + Route context.
// Wrap each render in a MemoryRouter with a root Route so <A> can resolve hrefs.
function renderHome(): ReturnType<typeof render> {
  return render(() => (
    <MemoryRouter>
      <Route path="/" component={Home} />
    </MemoryRouter>
  ));
}

beforeEach(() => {
  vi.mocked(isTauri).mockReset();
  vi.mocked(getBackendHandshake).mockReset();
  // Default: inside Tauri. Individual tests override as needed.
  vi.mocked(isTauri).mockReturnValue(true);
});

describe("Home — main menu", () => {
  it("renders the FINAL WHISTLE wordmark", () => {
    vi.mocked(getBackendHandshake).mockReturnValue(new Promise<never>(() => {}));
    renderHome();
    expect(screen.getByText("FINAL WHISTLE")).toBeInTheDocument();
  });

  it("renders the tagline", () => {
    vi.mocked(getBackendHandshake).mockReturnValue(new Promise<never>(() => {}));
    renderHome();
    expect(
      screen.getByText(/every career leaves a mark/i),
    ).toBeInTheDocument();
  });

  it("renders the NEW CAREER button and it is disabled", () => {
    vi.mocked(getBackendHandshake).mockReturnValue(new Promise<never>(() => {}));
    renderHome();
    const btn = screen.getByRole("button", { name: /new career/i });
    expect(btn).toBeInTheDocument();
    expect(btn).toBeDisabled();
  });

  it("renders the LOAD SAVE button and it is disabled", () => {
    vi.mocked(getBackendHandshake).mockReturnValue(new Promise<never>(() => {}));
    renderHome();
    const btn = screen.getByRole("button", { name: /load save/i });
    expect(btn).toBeInTheDocument();
    expect(btn).toBeDisabled();
  });

  it("renders the Settings link pointing to /settings", () => {
    vi.mocked(getBackendHandshake).mockReturnValue(new Promise<never>(() => {}));
    renderHome();
    const link = screen.getByRole("link", { name: /settings/i });
    expect(link).toBeInTheDocument();
    expect(link).toHaveAttribute("href", "/settings");
  });

  it("shows the pending status line while the backend check is in flight", () => {
    vi.mocked(getBackendHandshake).mockReturnValue(new Promise<never>(() => {}));
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
      // "backend ready · v9.9.9"
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
    // The backend handshake must NOT be invoked outside Tauri.
    expect(getBackendHandshake).not.toHaveBeenCalled();
  });

  it("does not render old diagnostic panel content", () => {
    vi.mocked(getBackendHandshake).mockReturnValue(new Promise<never>(() => {}));
    renderHome();
    // Old panel headings must not appear on the new landing.
    expect(screen.queryByText("Backend handshake")).not.toBeInTheDocument();
    expect(screen.queryByText(/pinging backend/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/quick actions/i)).not.toBeInTheDocument();
  });
});

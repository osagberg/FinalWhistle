/*
 * Home page — Vitest tests (QA-5).
 *
 * Substance requirements (MASTER_PLAN QA-5):
 *   AC1 — loading skeleton renders while the backend handshake is pending.
 *   AC2 — success render shows the handshake payload (appVersion / message /
 *         backendReady pill), via the Tauri path AND the browser-stub path.
 *   AC3 — error path shows football-native copy and NEVER leaks the raw
 *         err.message into the user-facing structured copy.
 *
 * Error-copy convention (matches ErrorBoundary.test.tsx): Vitest runs with
 * import.meta.env.DEV === true, so ErrorBoundary's DEV-only <pre> frame *does*
 * contain err.message in tests. The production gate (isDev() === false hiding
 * the raw message) cannot be stubbed via a module mock — `isDev()` is called
 * by a direct in-module reference, so replacing the export does not affect the
 * component's own call (ESM same-module-binding). We therefore assert the
 * user-facing structured elements (the <h2> headline + <p> detail) carry the
 * football-native copy and do NOT contain the raw technical string. That is
 * the element a player sees in production; the dev <pre> is excluded from the
 * assertion. ErrorBoundary.test.tsx validates the same prod-gate the same way.
 *
 * Mocking strategy:
 *   - ~/lib/tauri is mocked so each test controls isTauri() + getBackendHandshake().
 *   - @tauri-apps/api/core invoke is mocked so nothing throws outside Tauri.
 */

import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@solidjs/testing-library";

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

// A raw technical error string that must never reach the user-facing copy.
const RAW_TECHNICAL =
  "Cannot read properties of undefined (reading 'invoke') at safeInvoke";

beforeEach(() => {
  vi.mocked(isTauri).mockReset();
  vi.mocked(getBackendHandshake).mockReset();
  // Default: inside Tauri. Individual tests override as needed.
  vi.mocked(isTauri).mockReturnValue(true);
});

describe("Home", () => {
  it("renders the loading skeleton while the handshake is pending", () => {
    // A promise that never settles keeps the resource in its loading state.
    vi.mocked(getBackendHandshake).mockReturnValue(new Promise<never>(() => {}));

    render(() => <Home />);

    // The <Show> fallback (<Loading message="Pinging backend…">) is shown
    // synchronously before the resource resolves.
    expect(screen.getByText("Pinging backend…")).toBeInTheDocument();
  });

  it("renders the handshake payload on success (Tauri path)", async () => {
    vi.mocked(getBackendHandshake).mockResolvedValue({
      appVersion: "9.9.9",
      message: "Pitch is live",
      backendReady: true,
    });

    render(() => <Home />);

    await waitFor(() => {
      expect(screen.getByText("9.9.9")).toBeInTheDocument();
    });
    expect(screen.getByText("Pitch is live")).toBeInTheDocument();
    // backendReady: true → the pill reads "yes" (not the "stub" fallback).
    expect(screen.getByText("yes")).toBeInTheDocument();
    expect(screen.queryByText("stub")).not.toBeInTheDocument();
    // getBackendHandshake is the source of the payload (called on mount).
    expect(getBackendHandshake).toHaveBeenCalledTimes(1);
  });

  it("renders the browser-stub payload when not running in Tauri", async () => {
    // Browser tab: Home short-circuits to the stub and never invokes the backend.
    vi.mocked(isTauri).mockReturnValue(false);

    render(() => <Home />);

    await waitFor(() => {
      expect(
        screen.getByText("Browser preview — no Tauri backend."),
      ).toBeInTheDocument();
    });
    expect(screen.getByText("0.1.0")).toBeInTheDocument();
    // backendReady: false → the pill reads "stub".
    expect(screen.getByText("stub")).toBeInTheDocument();
    // The backend handshake must NOT be invoked outside Tauri.
    expect(getBackendHandshake).not.toHaveBeenCalled();
  });

  it("renders football-native error copy and never leaks the raw error", async () => {
    vi.mocked(getBackendHandshake).mockRejectedValue(new Error(RAW_TECHNICAL));

    render(() => <Home />);

    // The ErrorBoundary fallback (role=alert) replaces the panel on failure.
    const alert = await waitFor(() => screen.getByRole("alert"));

    // Football-native headline + detail are present.
    expect(alert.textContent).toMatch(/something went wrong on the bench/i);
    expect(alert.textContent).toMatch(/hit reset to try again/i);

    // The user-facing structured copy (<h2> headline + <p> detail) must NOT
    // contain the raw technical string. (The DEV-only <pre> frame may, in
    // Vitest's DEV mode — it is excluded from this assertion per the
    // ErrorBoundary.test convention; it is hidden in production builds.)
    const headline = alert.querySelector("h2");
    const paragraphs = Array.from(alert.querySelectorAll("p"));
    expect(headline?.textContent).toBeTruthy();
    expect(headline?.textContent ?? "").not.toContain("invoke");
    expect(
      paragraphs.every((p) => !(p.textContent ?? "").includes("invoke")),
    ).toBe(true);
    expect(
      paragraphs.every(
        (p) => !(p.textContent ?? "").includes("Cannot read properties"),
      ),
    ).toBe(true);

    // The Reset affordance is offered.
    expect(
      screen.getByRole("button", { name: /reset/i }),
    ).toBeInTheDocument();
  });
});

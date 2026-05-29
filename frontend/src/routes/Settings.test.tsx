/**
 * Settings.test.tsx — T4-6a.
 *
 * Verifies the Settings route renders both controls and that toggling
 * each control persists via `setSettings`.
 *
 * Pattern mirrors Career.test.tsx / Squad.test.tsx — mock IPC before mounting,
 * assert DOM structure, simulate interactions, assert IPC calls.
 */

import { cleanup, fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Mock } from "vitest";

// Mock invoke BEFORE importing anything that calls it.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import Settings from "./Settings";
import type { AppSettingsDto } from "~/lib/types";

const mockInvoke = invoke as unknown as Mock;

function makeSettings(overrides?: Partial<AppSettingsDto>): AppSettingsDto {
  return { theme: "light", reduceMotion: false, ...overrides };
}

afterEach(() => {
  cleanup();
  mockInvoke.mockReset();
  // Reset document.documentElement classes between tests.
  document.documentElement.classList.remove("dark", "reduce-motion");
});

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

describe("Settings route", () => {
  beforeEach(() => {
    // Default: get_settings returns defaults; set_settings succeeds.
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_settings") return Promise.resolve(makeSettings());
      if (cmd === "set_settings") return Promise.resolve(null);
      return Promise.reject(new Error(`unexpected invoke: ${cmd}`));
    });
  });

  it("renders the Preferences heading", async () => {
    render(() => <Settings />);
    await waitFor(() => {
      expect(screen.getByRole("heading", { name: /preferences/i })).toBeTruthy();
    });
  });

  it("renders the theme toggle button", async () => {
    render(() => <Settings />);
    await waitFor(() => {
      // The button is labelled "Colour scheme" via its <label>.
      expect(screen.getByLabelText(/switch to/i)).toBeTruthy();
    });
  });

  it("renders the reduce-motion checkbox", async () => {
    render(() => <Settings />);
    await waitFor(() => {
      expect(screen.getByRole("checkbox", { name: /reduce motion/i })).toBeTruthy();
    });
  });

  it("loads initial values from getSettings on mount", async () => {
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "get_settings")
        return Promise.resolve(makeSettings({ theme: "dark", reduceMotion: true }));
      if (cmd === "set_settings") return Promise.resolve(null);
      return Promise.reject(new Error(`unexpected invoke: ${cmd}`));
    });

    render(() => <Settings />);

    await waitFor(() => {
      // Reduce-motion checkbox should be checked.
      const checkbox = screen.getByRole("checkbox", {
        name: /reduce motion/i,
      }) as HTMLInputElement;
      expect(checkbox.checked).toBe(true);
    });
  });

  // ---------------------------------------------------------------------------
  // Interactions
  // ---------------------------------------------------------------------------

  it("calls setSettings when theme toggle is clicked", async () => {
    render(() => <Settings />);

    await waitFor(() => {
      expect(screen.getByLabelText(/switch to/i)).toBeTruthy();
    });

    const toggleBtn = screen.getByLabelText(/switch to/i);
    fireEvent.click(toggleBtn);

    await waitFor(() => {
      const setCalls = mockInvoke.mock.calls.filter(
        (c: unknown[]) => c[0] === "set_settings",
      );
      expect(setCalls.length).toBeGreaterThan(0);
    });
  });

  it("calls setSettings when reduce-motion checkbox is toggled", async () => {
    render(() => <Settings />);

    await waitFor(() => {
      expect(screen.getByRole("checkbox", { name: /reduce motion/i })).toBeTruthy();
    });

    const checkbox = screen.getByRole("checkbox", { name: /reduce motion/i });
    fireEvent.click(checkbox);

    await waitFor(() => {
      const setCalls = mockInvoke.mock.calls.filter(
        (c: unknown[]) => c[0] === "set_settings",
      );
      expect(setCalls.length).toBeGreaterThan(0);
    });
  });
});

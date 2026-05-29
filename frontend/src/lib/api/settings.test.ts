/**
 * settings.test.ts — T4-6a.
 *
 * Round-trip tests for the two `api/settings.ts` wrappers.
 *
 * Pattern mirrors api/live_match.test.ts:
 *   - Mock `@tauri-apps/api/core` `invoke` BEFORE importing the SUT.
 *   - Assert the correct command-name string is passed.
 *   - Assert the payload shape matches the DTO the wrapper promises to return.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Mock } from "vitest";

// Mock BEFORE importing the SUT — vi.mock is hoisted by vitest.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { getSettings, setSettings } from "./settings";
import type { AppSettingsDto } from "~/lib/types";

const mockInvoke = invoke as unknown as Mock;

// ---------------------------------------------------------------------------
// Fixture builders
// ---------------------------------------------------------------------------

function makeSettings(overrides?: Partial<AppSettingsDto>): AppSettingsDto {
  return {
    theme: "light",
    reduceMotion: false,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// getSettings
// ---------------------------------------------------------------------------

describe("getSettings", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("invokes get_settings with no arguments", async () => {
    mockInvoke.mockResolvedValue(makeSettings());

    await getSettings();

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith("get_settings", {});
  });

  it("returns defaults (light, no reduce-motion) from backend", async () => {
    const defaults = makeSettings();
    mockInvoke.mockResolvedValue(defaults);

    const result = await getSettings();

    expect(result.theme).toBe("light");
    expect(result.reduceMotion).toBe(false);
  });

  it("returns dark + reduce_motion=true when backend returns them", async () => {
    mockInvoke.mockResolvedValue(makeSettings({ theme: "dark", reduceMotion: true }));

    const result = await getSettings();

    expect(result.theme).toBe("dark");
    expect(result.reduceMotion).toBe(true);
  });

  it("throws IpcShapeError when theme is an unknown string", async () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    mockInvoke.mockResolvedValue({ theme: "sepia", reduceMotion: false } as any);

    await expect(getSettings()).rejects.toThrow(/shape/i);
  });

  it("throws IpcShapeError when reduceMotion field is missing", async () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    mockInvoke.mockResolvedValue({ theme: "light" } as any);

    await expect(getSettings()).rejects.toThrow(/shape/i);
  });

  it("passes backend IpcError through unchanged on rejection", async () => {
    const ipcErr = { kind: "settingsLoadFailed", reason: "corrupt file" };
    mockInvoke.mockRejectedValue(ipcErr);

    await expect(getSettings()).rejects.toMatchObject(ipcErr);
  });
});

// ---------------------------------------------------------------------------
// setSettings
// ---------------------------------------------------------------------------

describe("setSettings", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("invokes set_settings with the settings payload", async () => {
    mockInvoke.mockResolvedValue(null);

    const settings = makeSettings({ theme: "dark", reduceMotion: true });
    await setSettings(settings);

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith("set_settings", { settings });
  });

  it("resolves to void when backend returns null", async () => {
    mockInvoke.mockResolvedValue(null);

    await expect(setSettings(makeSettings())).resolves.toBeUndefined();
  });

  it("resolves to void when backend returns undefined", async () => {
    mockInvoke.mockResolvedValue(undefined);

    await expect(setSettings(makeSettings({ theme: "dark" }))).resolves.toBeUndefined();
  });

  it("propagates SettingsLoadFailed IpcError on write failure", async () => {
    const ipcErr = { kind: "settingsLoadFailed", reason: "permission denied" };
    mockInvoke.mockRejectedValue(ipcErr);

    await expect(setSettings(makeSettings())).rejects.toMatchObject(ipcErr);
  });

  it("sends theme and reduceMotion with the correct camelCase keys", async () => {
    mockInvoke.mockResolvedValue(null);

    await setSettings({ theme: "dark", reduceMotion: true });

    const call = mockInvoke.mock.calls[0] as [string, { settings: AppSettingsDto }] | undefined;
    expect(call).toBeDefined();
    const settingsArg = call![1].settings;
    expect(settingsArg.theme).toBe("dark");
    expect(settingsArg.reduceMotion).toBe(true);
    // Confirm the camelCase key is present (not snake_case reduce_motion).
    expect("reduceMotion" in settingsArg).toBe(true);
    expect("reduce_motion" in settingsArg).toBe(false);
  });
});

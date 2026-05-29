/**
 * Settings IPC wrappers (T4-6a).
 *
 * Two `safeInvoke`-wrapped functions matching the T4-6a settings commands.
 * Each wrapper validates the backend payload before returning — backend DTO
 * drift throws `IpcShapeError` at the IPC seam rather than silently propagating.
 *
 * On first-run (no settings file), `getSettings` returns the defaults:
 * `{ theme: "light", reduceMotion: false }`. This is NOT an error.
 */

import { isAppSettings, safeInvoke } from "../runtime-validators";
import type { AppSettingsDto } from "../types";

/**
 * Fetch persisted app settings.
 *
 * Returns defaults when no settings file exists (first-run). Throws
 * `IpcError::SettingsLoadFailed` if the file is present but corrupt.
 *
 * Throws `IpcShapeError` if the backend returns an unexpected payload shape.
 */
export async function getSettings(): Promise<AppSettingsDto> {
  return safeInvoke("get_settings", {}, isAppSettings);
}

/**
 * Persist app settings.
 *
 * Encodes `settings` to the settings file in the Tauri app-config directory.
 * Returns `void` on success. Throws `IpcError::SettingsLoadFailed` on
 * encode or write failure.
 */
export async function setSettings(settings: AppSettingsDto): Promise<void> {
  await safeInvoke(
    "set_settings",
    { settings },
    (v): v is undefined => v === undefined || v === null,
  );
}

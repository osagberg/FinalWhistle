/*
 * Settings page — T4-6a.
 *
 * Exposes two app preferences that require zero additional design work:
 *   - Theme: Light / Dark toggle.
 *   - Reduce motion: disables CSS transitions + animations.
 *
 * Each control updates the local Solid signal (via setTheme / setReduceMotion
 * from ~/lib/state) AND persists via setSettings(). The resource loads the
 * current persisted values on mount so the controls reflect what was saved.
 *
 * IPC (via api/settings.ts wrappers):
 *   getSettings()   → AppSettingsDto    — loaded on mount
 *   setSettings(s)  → void             — onChange for each control
 *
 * Rules compliance:
 *   - No `any` (Frontend/RULES.md §6)
 *   - Solid signals / createResource, not React hooks (Frontend/RULES.md §1)
 *   - IpcError exhaustive switch + never discriminant (mirrors Squad.tsx pattern)
 *   - Dark-mode tokens on every color-bearing class (Frontend/RULES.md §2)
 *   - Keyboard-accessible controls with visible labels (Frontend/RULES.md §8)
 *   - Football-native / professional copy (Frontend/RULES.md §9)
 *   - UI never drives canonical state — settings are app-global, not per-save
 */

import { createMemo, createResource, createSignal, Show, type JSX } from "solid-js";
import ErrorBoundary from "~/components/ErrorBoundary";
import Loading from "~/components/Loading";
import { getSettings, setSettings } from "~/lib/api/settings";
import { IpcShapeError } from "~/lib/runtime-validators";
import { describeRouteError } from "~/lib/route-errors";
import { theme, setTheme, reduceMotion, setReduceMotion } from "~/lib/state";
import type { AppSettingsDto, IpcError } from "~/lib/types";

// ---------------------------------------------------------------------------
// IpcError type guard
//
// Self-contained per project convention — mirrors Squad.tsx / League.tsx.
// `satisfies` pins KNOWN_IPC_ERROR_KINDS to IpcError["kind"].
// ---------------------------------------------------------------------------

const KNOWN_IPC_ERROR_KINDS = new Set([
  "tooManyFrames",
  "invalidSeed",
  "matchInitFailed",
  "seasonComplete",
  "clubNotFound",
  "lockPoisoned",
  "playerNotFound",
  "seasonNotComplete",
  "liveMatchCommandUnimplemented",
  // T4-6a: settings variant.
  "settingsLoadFailed",
  // T4-F4: scouting variants.
  "notYetObserved",
  "leagueGenerationFailed",
] as const) satisfies ReadonlySet<IpcError["kind"]>;

function isIpcError(e: unknown): e is IpcError {
  if (typeof e !== "object" || e === null || !("kind" in e)) return false;
  const kind = (e as Record<string, unknown>).kind;
  return (
    typeof kind === "string" &&
    (KNOWN_IPC_ERROR_KINDS as ReadonlySet<string>).has(kind)
  );
}

function normaliseError(e: unknown): IpcError | Error {
  if (isIpcError(e)) return e;
  if (e instanceof Error) return e;
  return new Error(String(e));
}

// ---------------------------------------------------------------------------
// FetchErrorBanner — extracted to give TS a concrete prop type for `error`.
// ---------------------------------------------------------------------------

function FetchErrorBanner(props: { error: IpcError | Error | null }): JSX.Element {
  const copy = createMemo(() => describeRouteError(props.error, { what: "preferences" }));
  return (
    <div
      role="alert"
      class="rounded border border-flag-red/30 bg-flag-red/5 p-3 text-sm text-flag-red dark:border-flag-red/20 dark:bg-flag-red/10"
    >
      <p class="font-semibold">{copy().headline}</p>
      <p class="mt-1 text-ink-subtle dark:text-paper-subtle">{copy().detail}</p>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export default function Settings(): JSX.Element {
  return (
    <ErrorBoundary label="Settings">
      <SettingsInner />
    </ErrorBoundary>
  );
}

function SettingsInner(): JSX.Element {
  // Fetch error signal. MUST be a reactive signal (not a plain `let`) so the
  // error banner's <Show> re-evaluates after the async resource fetch rejects.
  const [fetchError, setFetchError] = createSignal<IpcError | Error | null>(null);

  // Load persisted settings on mount. On success, apply them to the global
  // state signals so the UI reflects what was saved.
  const [settings] = createResource<AppSettingsDto | null>(async () => {
    try {
      const s = await getSettings();
      // Apply persisted values to global state (first-run defaults are already
      // the Solid signal defaults, so this is a no-op on clean install).
      setTheme(s.theme);
      setReduceMotion(s.reduceMotion);
      return s;
    } catch (e: unknown) {
      if (e instanceof IpcShapeError) {
        // eslint-disable-next-line no-console
        console.error(
          "[Settings] get_settings DTO contract drift:",
          e.command,
          e.reason,
          e.payloadPreview,
        );
      } else {
        // eslint-disable-next-line no-console
        console.error("[Settings] getSettings failed:", e);
      }
      setFetchError(normaliseError(e));
      return null;
    }
  });

  /** Persist the current signal values. Best-effort: log on failure, don't crash. */
  async function persist(): Promise<void> {
    try {
      await setSettings({ theme: theme(), reduceMotion: reduceMotion() });
    } catch (e: unknown) {
      // eslint-disable-next-line no-console
      console.error("[Settings] setSettings failed:", e);
    }
  }

  const handleThemeToggle = async (): Promise<void> => {
    setTheme(theme() === "light" ? "dark" : "light");
    await persist();
  };

  const handleReduceMotionToggle = async (
    e: Event & { currentTarget: HTMLInputElement },
  ): Promise<void> => {
    setReduceMotion(e.currentTarget.checked);
    await persist();
  };

  return (
    <div class="space-y-6">
      {/* Header */}
      <header>
        <h1 class="font-display text-3xl text-pitch-600 dark:text-pitch-300">
          Preferences
        </h1>
        <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
          Adjust the display and accessibility options for Final Whistle.
        </p>
      </header>

      {/* Loading state */}
      <Show when={settings.loading}>
        <Loading />
      </Show>

      {/* Fetch error — non-fatal; show copy but still render controls with current signal values */}
      <Show when={!settings.loading && fetchError() !== null}>
        <FetchErrorBanner error={fetchError()} />
      </Show>

      {/* Controls — always rendered (use current signal values, not resource) */}
      <Show when={!settings.loading}>
        <section class="fw-panel space-y-5 p-4">
          {/* Theme */}
          <div class="flex items-center justify-between gap-4">
            <div>
              <label
                for="settings-theme-toggle"
                class="text-sm font-semibold text-ink dark:text-paper"
              >
                Colour scheme
              </label>
              <p class="mt-0.5 text-xs text-ink-subtle dark:text-paper-subtle">
                Switch between light and dark display modes.
              </p>
            </div>
            <button
              id="settings-theme-toggle"
              type="button"
              class="px-4 py-1.5 rounded text-sm font-mono bg-paper-subtle dark:bg-midnight-subtle border border-ink-mute/20 dark:border-midnight-line text-ink dark:text-paper hover:bg-pitch-50 dark:hover:bg-pitch-900/30 focus:outline-none focus:ring-2 focus:ring-pitch-400"
              onClick={() => void handleThemeToggle()}
              aria-pressed={theme() === "dark"}
              aria-label={`Switch to ${theme() === "light" ? "dark" : "light"} mode`}
            >
              {theme() === "light" ? "Light" : "Dark"}
            </button>
          </div>

          <hr class="border-ink-mute/10 dark:border-midnight-line" />

          {/* Reduce motion */}
          <div class="flex items-center justify-between gap-4">
            <div>
              <label
                for="settings-reduce-motion"
                class="text-sm font-semibold text-ink dark:text-paper"
              >
                Reduce motion
              </label>
              <p class="mt-0.5 text-xs text-ink-subtle dark:text-paper-subtle">
                Disables transitions and animations across the interface. Recommended for vestibular disorders.
              </p>
            </div>
            <input
              id="settings-reduce-motion"
              type="checkbox"
              checked={reduceMotion()}
              onChange={(e) => void handleReduceMotionToggle(e)}
              class="h-4 w-4 rounded border-ink-mute/40 text-pitch-500 focus:ring-pitch-400"
              aria-label="Reduce motion"
            />
          </div>
        </section>
      </Show>
    </div>
  );
}

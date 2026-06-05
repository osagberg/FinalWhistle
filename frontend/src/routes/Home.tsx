import { createResource, createSignal, Show, type JSX } from "solid-js";
import { A, useNavigate } from "@solidjs/router";
import { getBackendHandshake, isTauri } from "~/lib/tauri";
import { loadCareer } from "~/lib/api/new-career";
import { describeRouteError } from "~/lib/route-errors";

/**
 * Main-menu landing (T4-7 game-shell polish).
 *
 * Shows the FINAL WHISTLE wordmark, a one-line football-native tagline,
 * and three action surfaces:
 *   - New career   (disabled, coming at T4-8+)
 *   - Load save    (disabled, coming at T4-8+)
 *   - Settings     (real, routes to /settings)
 *
 * The backend liveness status is demoted to a small unobtrusive line at the
 * foot of the card — it's a dev diagnostic, not player-facing.
 */
/** Generate a random 64-bit world seed as a `0x…` hex string. */
function generateSeedHex(): string {
  const bytes = new Uint8Array(8);
  crypto.getRandomValues(bytes);
  return (
    "0x" +
    Array.from(bytes)
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("")
  );
}

export default function Home(): JSX.Element {
  const navigate = useNavigate();

  // Liveness check — kept as a small diagnostic line only.
  const [status] = createResource(async () => {
    if (!isTauri()) {
      return { ready: false, label: "browser preview — no Tauri backend" };
    }
    const hs = await getBackendHandshake();
    return {
      ready: hs.backendReady,
      label: hs.backendReady ? `backend ready · v${hs.appVersion}` : hs.message,
    };
  });

  // LOAD SAVE action state.
  const [loadPending, setLoadPending] = createSignal(false);
  const [loadError, setLoadError] = createSignal<string | null>(null);

  const handleNewCareer = (): void => {
    const seedHex = generateSeedHex();
    // Pass seed to the club-selection screen via router state.
    navigate("/new-career", { state: { seedHex } });
  };

  const handleLoadSave = async (): Promise<void> => {
    if (loadPending()) return;
    setLoadPending(true);
    setLoadError(null);
    try {
      await loadCareer();
      navigate("/squad");
    } catch (e: unknown) {
      const copy = describeRouteError(e, { what: "your save" });
      setLoadError(copy.headline);
    } finally {
      setLoadPending(false);
    }
  };

  return (
    <div class="min-h-[calc(100vh-7rem)] flex flex-col items-center justify-center gap-8 py-12">
      {/* Wordmark — the <h1> carries the accessible name; the wrapper is
          decorative structure (no aria-label, to avoid a double announcement). */}
      <div class="flex flex-col items-center gap-2 select-none">
        <h1
          class="font-display text-6xl tracking-wider text-pitch-600 dark:text-pitch-300"
          aria-label="Final Whistle"
        >
          FINAL WHISTLE
        </h1>
        <p class="text-sm text-ink-mute dark:text-paper-subtle font-body tracking-wide">
          Every career leaves a mark. Every world plays different.
        </p>
      </div>

      {/* Action card */}
      <div class="fw-panel w-full max-w-xs flex flex-col gap-3 p-6">
        {/* New career — generates a random seed then routes to /new-career */}
        <button
          type="button"
          class="w-full py-3 px-4 rounded text-sm font-display tracking-wider bg-pitch-500 text-paper hover:bg-pitch-600 focus:outline-none focus:ring-2 focus:ring-pitch-400 transition-colors"
          onClick={handleNewCareer}
        >
          NEW CAREER
        </button>

        {/* Load save */}
        <button
          type="button"
          disabled={loadPending()}
          class="w-full py-3 px-4 rounded text-sm font-display tracking-wider border border-ink-mute/30 dark:border-midnight-line text-ink-subtle dark:text-paper-subtle hover:text-ink dark:hover:text-paper hover:bg-paper-subtle dark:hover:bg-midnight-subtle focus:outline-none focus:ring-2 focus:ring-pitch-400 disabled:opacity-50 disabled:cursor-wait transition-colors"
          onClick={() => void handleLoadSave()}
        >
          {loadPending() ? "LOADING…" : "LOAD SAVE"}
        </button>

        {/* Load error — inline below the button */}
        <Show when={loadError()}>
          {(msg) => (
            <p
              class="text-xs text-rose-600 dark:text-rose-400"
              role="alert"
            >
              {msg()}
            </p>
          )}
        </Show>

        {/* Settings — real link */}
        <A
          href="/settings"
          class="w-full py-2.5 px-4 rounded text-sm font-body text-center text-ink-subtle dark:text-paper-subtle hover:text-ink dark:hover:text-paper hover:bg-paper-subtle dark:hover:bg-midnight-subtle transition-colors"
        >
          Settings
        </A>
      </div>

      {/* Backend status — dev diagnostic, visually minimal */}
      <Show
        when={status()}
        fallback={
          <p class="text-xs text-ink-mute dark:text-paper-subtle font-mono opacity-50">
            checking backend…
          </p>
        }
      >
        {(s) => (
          <p
            class="text-xs font-mono opacity-50"
            classList={{
              "text-pitch-600 dark:text-pitch-400": s().ready,
              "text-ink-mute dark:text-paper-subtle": !s().ready,
            }}
          >
            {s().label}
          </p>
        )}
      </Show>
    </div>
  );
}

import { createSignal, For, Show, type JSX } from "solid-js";
import TacticalBoard from "~/components/TacticalBoard";
import ErrorBoundary from "~/components/ErrorBoundary";
import { isTauri, playMatch } from "~/lib/tauri";
import type { MatchResult } from "~/lib/types";

export default function Match(): JSX.Element {
  const [result, setResult] = createSignal<MatchResult | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  // Deterministic seed for the placeholder. Real seed selection lands at T1-6.
  const PLACEHOLDER_SEED = 0xfeedbeefcafefaden;

  const onPlay = async () => {
    setError(null);
    setBusy(true);
    try {
      if (!isTauri()) {
        // Browser preview — fake a 0-0 result so the surface renders.
        setResult({
          matchId: "browser-preview",
          homeId: "home",
          awayId: "away",
          homeScore: 0,
          awayScore: 0,
          canonicalHash: "0x" + "0".repeat(64),
          events: [],
        });
      } else {
        setResult(await playMatch(PLACEHOLDER_SEED, "home", "away"));
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <ErrorBoundary>
      <div class="space-y-4">
        <header class="flex items-center justify-between">
          <div>
            <h1 class="font-display text-3xl text-pitch-600 dark:text-pitch-300">Match</h1>
            <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
              T0 placeholder — tactical board renders an empty pitch. Real sim lands at T1-5.
            </p>
          </div>
          <button
            type="button"
            class="fw-nav-link bg-pitch-500 text-white hover:bg-pitch-600 disabled:opacity-50"
            onClick={onPlay}
            disabled={busy()}
          >
            {busy() ? "Simulating…" : "Play match"}
          </button>
        </header>

        <div class="grid grid-cols-1 lg:grid-cols-[1fr_320px] gap-4">
          <section>
            <TacticalBoard height={520} />
          </section>
          <aside class="fw-panel p-3 space-y-2">
            <h2 class="font-display text-lg">Commentary</h2>
            <Show
              when={result()}
              fallback={
                <p class="text-sm text-ink-mute dark:text-paper-subtle">
                  Press <span class="font-mono">Play match</span> to run the stub command.
                </p>
              }
            >
              {(r) => (
                <div class="space-y-2 text-sm">
                  <p class="font-mono text-base">
                    {r().homeScore} – {r().awayScore}
                  </p>
                  <p class="text-xs text-ink-mute font-mono break-all">
                    {r().canonicalHash}
                  </p>
                  <ul class="space-y-1">
                    <For
                      each={r().events}
                      fallback={
                        <li class="text-xs text-ink-mute italic">
                          No events yet (placeholder result).
                        </li>
                      }
                    >
                      {(ev) => (
                        <li class="text-xs">
                          <span class="font-mono mr-1">{ev.minute}'</span>
                          {ev.description}
                        </li>
                      )}
                    </For>
                  </ul>
                </div>
              )}
            </Show>
            <Show when={error()}>
              <p class="text-xs text-flag-red font-mono">{error()}</p>
            </Show>
          </aside>
        </div>
      </div>
    </ErrorBoundary>
  );
}

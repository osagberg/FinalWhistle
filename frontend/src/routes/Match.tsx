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
  // 90 simulated minutes at ~1 tick/6s real time = 900 ticks. Enough for a
  // full match. Real tick-count configuration is T1-6 territory.
  const PLACEHOLDER_TICKS = 900;

  const onPlay = async () => {
    setError(null);
    setBusy(true);
    try {
      if (!isTauri()) {
        // Browser preview — fake a result so the surface renders.
        setResult({
          finalScore: { home: 0, away: 0 },
          canonicalHash: "blake3:" + "0".repeat(64),
          matchEvents: [],
          seedHex: "0xfeedbeefcafefade",
          tickCount: 0,
          commentaryPreview: [],
        });
      } else {
        setResult(await playMatch(PLACEHOLDER_SEED, PLACEHOLDER_TICKS));
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
              T1-5 IPC surface live. Press Play to run a full sim match.
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
                  Press <span class="font-mono">Play match</span> to run the sim.
                </p>
              }
            >
              {(r) => (
                <div class="space-y-2 text-sm">
                  <p class="font-mono text-base">
                    {r().finalScore.home} – {r().finalScore.away}
                  </p>
                  <p class="text-xs text-ink-mute font-mono break-all">
                    {r().canonicalHash}
                  </p>
                  <ul class="space-y-1">
                    <For
                      each={r().commentaryPreview}
                      fallback={
                        <li class="text-xs text-ink-mute italic">
                          No events recorded.
                        </li>
                      }
                    >
                      {(line) => (
                        <li class="text-xs">{line}</li>
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

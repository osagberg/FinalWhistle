import { createResource, Show, type JSX } from "solid-js";
import { getDummyState, isTauri } from "~/lib/tauri";
import Loading from "~/components/Loading";
import ErrorBoundary from "~/components/ErrorBoundary";

export default function Home(): JSX.Element {
  // Liveness ping. In a plain browser tab there's no Tauri runtime, so we
  // skip the invoke and render a stub message instead.
  const [state] = createResource(async () => {
    if (!isTauri()) {
      return {
        appVersion: "0.1.0",
        message: "Browser preview — no Tauri backend.",
        backendReady: false,
      };
    }
    return getDummyState();
  });

  return (
    <ErrorBoundary>
      <div class="space-y-4">
        <header>
          <h1 class="font-display text-3xl text-pitch-600 dark:text-pitch-300">
            Welcome to Final Whistle
          </h1>
          <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
            T0 placeholder — not yet implemented.
          </p>
        </header>
        <section class="fw-panel p-4">
          <h2 class="font-display text-lg">Backend handshake</h2>
          <Show when={state()} fallback={<Loading message="Pinging backend…" />}>
            {(s) => (
              <dl class="mt-2 grid grid-cols-2 gap-2 text-sm">
                <dt class="text-ink-mute">App version</dt>
                <dd class="font-mono">{s().appVersion}</dd>
                <dt class="text-ink-mute">Message</dt>
                <dd class="font-mono">{s().message}</dd>
                <dt class="text-ink-mute">Backend ready</dt>
                <dd class="font-mono">
                  <span
                    class="fw-pill"
                    classList={{
                      "bg-pitch-100 text-pitch-700": s().backendReady,
                      "bg-paper-bold text-ink-mute": !s().backendReady,
                    }}
                  >
                    {s().backendReady ? "yes" : "stub"}
                  </span>
                </dd>
              </dl>
            )}
          </Show>
        </section>
        <section class="fw-panel p-4">
          <h2 class="font-display text-lg">Quick actions</h2>
          <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
            New career, load save, and quickstart land at T2-5.
          </p>
        </section>
      </div>
    </ErrorBoundary>
  );
}

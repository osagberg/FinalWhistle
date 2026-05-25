import type { JSX } from "solid-js";
import ErrorBoundary from "~/components/ErrorBoundary";

export default function Tactics(): JSX.Element {
  return (
    <ErrorBoundary label="Tactics">
      <TacticsInner />
    </ErrorBoundary>
  );
}

function TacticsInner(): JSX.Element {
  return (
    <div class="space-y-4">
      <header>
        <h1 class="font-display text-3xl text-pitch-600 dark:text-pitch-300">Tactics</h1>
        <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
          T0 placeholder — not yet implemented. Formation + role assignment land at T1-6.
        </p>
      </header>
      <section class="fw-panel p-4">
        <h2 class="font-display text-lg">Formation</h2>
        <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
          Formation slots and starting XI controls will surface here.
        </p>
      </section>
      <section class="fw-panel p-4">
        <h2 class="font-display text-lg">Manager archetype</h2>
        <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
          20–30 archetypes ship at T2-1. Today: no selection.
        </p>
      </section>
    </div>
  );
}

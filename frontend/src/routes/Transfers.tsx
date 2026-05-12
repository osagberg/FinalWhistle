import type { JSX } from "solid-js";

export default function Transfers(): JSX.Element {
  return (
    <div class="space-y-4">
      <header>
        <h1 class="font-display text-3xl text-pitch-600 dark:text-pitch-300">Transfers</h1>
        <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
          T0 placeholder — not yet implemented. Transfer-window stub UI lands at T2-8.
        </p>
      </header>
      <section class="fw-panel p-4">
        <h2 class="font-display text-lg">Window state</h2>
        <span class="fw-pill bg-paper-bold text-ink-mute mt-2 inline-block">closed</span>
      </section>
    </div>
  );
}

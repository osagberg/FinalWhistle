import { A } from "@solidjs/router";
import { For, type JSX, type ParentProps, Show } from "solid-js";
import { theme, setTheme, selectedClubId } from "~/lib/state";

interface NavItem {
  to: string;
  label: string;
  /** Hotkey hint shown in the corner. Real key handling lands at T4-6. */
  hotkey?: string;
}

const NAV: readonly NavItem[] = [
  { to: "/", label: "Home", hotkey: "1" },
  { to: "/squad", label: "Squad", hotkey: "2" },
  { to: "/tactics", label: "Tactics", hotkey: "3" },
  { to: "/transfers", label: "Transfers", hotkey: "4" },
  { to: "/league", label: "League", hotkey: "5" },
  { to: "/career", label: "Career", hotkey: "6" },
  { to: "/stats", label: "Stats", hotkey: "7" },
  { to: "/match", label: "Match", hotkey: "M" },
] as const;

export default function Layout(props: ParentProps): JSX.Element {
  return (
    <div class="h-full flex flex-col">
      <TopBar />
      <div class="flex-1 flex min-h-0">
        <Sidebar />
        <main class="flex-1 min-w-0 overflow-auto bg-paper dark:bg-midnight">
          <div class="max-w-[1400px] mx-auto p-4">{props.children}</div>
        </main>
      </div>
      <StatusBar />
    </div>
  );
}

function TopBar(): JSX.Element {
  return (
    <header class="flex items-center justify-between border-b border-ink-mute/15 dark:border-midnight-line bg-white dark:bg-midnight-panel px-4 py-2">
      <div class="flex items-center gap-3">
        <span class="font-display text-xl tracking-wide text-pitch-600 dark:text-pitch-300">
          FINAL WHISTLE
        </span>
        <span class="fw-pill bg-paper-subtle dark:bg-midnight-subtle text-ink-mute dark:text-paper-subtle">
          T0 scaffold
        </span>
      </div>
      <nav class="flex items-center gap-1" aria-label="Primary">
        <For each={NAV}>
          {(item) => (
            <A
              href={item.to}
              class="fw-nav-link"
              activeClass="active"
              end={item.to === "/"}
            >
              <span class="font-body">{item.label}</span>
              <Show when={item.hotkey}>
                <span class="ml-1.5 text-[10px] text-ink-mute font-mono">
                  [{item.hotkey}]
                </span>
              </Show>
            </A>
          )}
        </For>
      </nav>
      <button
        type="button"
        class="fw-nav-link"
        onClick={() => setTheme(theme() === "light" ? "dark" : "light")}
        aria-label="Toggle theme"
      >
        <span class="font-mono text-xs">{theme() === "light" ? "DARK" : "LIGHT"}</span>
      </button>
    </header>
  );
}

function Sidebar(): JSX.Element {
  // Sidebar surfaces career-context info at MVP. Until a career is loaded it
  // shows an "Inactive" stub — DESIGN_DOC §9 says inactive states must be
  // honest, not decorative.
  return (
    <aside class="w-56 shrink-0 border-r border-ink-mute/15 dark:border-midnight-line bg-paper-subtle dark:bg-midnight-panel p-3 space-y-3">
      <div>
        <h2 class="text-xs uppercase tracking-wider text-ink-mute dark:text-paper-subtle">
          Career
        </h2>
        <Show
          when={selectedClubId()}
          fallback={
            <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
              No career active.
            </p>
          }
        >
          <p class="mt-1 text-sm font-mono">{selectedClubId()}</p>
        </Show>
      </div>
      <div>
        <h2 class="text-xs uppercase tracking-wider text-ink-mute dark:text-paper-subtle">
          Season
        </h2>
        <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">—</p>
      </div>
      <div>
        <h2 class="text-xs uppercase tracking-wider text-ink-mute dark:text-paper-subtle">
          Next fixture
        </h2>
        <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">—</p>
      </div>
    </aside>
  );
}

function StatusBar(): JSX.Element {
  return (
    <footer class="border-t border-ink-mute/15 dark:border-midnight-line bg-white dark:bg-midnight-panel px-4 py-1 flex items-center justify-between text-xs text-ink-mute dark:text-paper-subtle font-mono">
      <span>v0.1.0 · T0 scaffold</span>
      <span>determinism: pinned · build: dev</span>
    </footer>
  );
}

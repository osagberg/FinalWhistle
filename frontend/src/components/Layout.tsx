import { A } from "@solidjs/router";
import { For, type JSX, type ParentProps, Show } from "solid-js";
import {
  theme,
  setTheme,
  reduceMotion,
  isCareerActive,
  managedClubName,
  seasonNumber,
} from "~/lib/state";
import { setSettings } from "~/lib/api/settings";

interface NavItem {
  to: string;
  label: string;
}

// Primary navigation domains. Hotkey hints are intentionally absent —
// decorative [1]/[M] spans are noise until real key handling lands at T4-6b.
// When keys land, surface them via a discoverable shortcut overlay (press ?),
// not permanent label clutter.
const NAV: readonly NavItem[] = [
  { to: "/", label: "Home" },
  { to: "/squad", label: "Squad" },
  { to: "/tactics", label: "Tactics" },
  { to: "/transfers", label: "Transfers" },
  { to: "/league", label: "League" },
  { to: "/career", label: "Career" },
  { to: "/stats", label: "Stats" },
  { to: "/match", label: "Match" },
  { to: "/settings", label: "Settings" },
] as const;

export default function Layout(props: ParentProps): JSX.Element {
  return (
    <div class="h-full flex flex-col">
      <TopStrip />
      <div class="flex-1 flex min-h-0">
        <PrimarySidebar />
        <main class="flex-1 min-w-0 overflow-auto bg-paper dark:bg-midnight">
          <div class="max-w-[1400px] mx-auto p-4">{props.children}</div>
        </main>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// TopStrip — slim identity strip carrying only the wordmark, career context
// (club + season), and the theme toggle. Navigation lives in the sidebar.
// ---------------------------------------------------------------------------

function TopStrip(): JSX.Element {
  // Toggle the theme AND persist it. Without the persist, the nav toggle set
  // the signal only, and visiting /settings (whose onMount re-reads the saved
  // value) reset it to light. Mirrors the Settings route's persist path.
  const handleThemeToggle = (): void => {
    const next = theme() === "light" ? "dark" : "light";
    setTheme(next);
    void setSettings({ theme: next, reduceMotion: reduceMotion() }).catch(
      // eslint-disable-next-line no-console
      (e: unknown) => console.error("[Layout] theme persist failed:", e),
    );
  };

  return (
    <header
      class="flex items-center justify-between border-b border-ink-mute/15 dark:border-midnight-line bg-white dark:bg-midnight-panel px-4 py-2 shrink-0"
      aria-label="App header"
    >
      {/* Wordmark — Anton display font, pitch-green accent */}
      <span
        class="font-display text-xl tracking-wide text-pitch-600 dark:text-pitch-300 select-none"
        aria-hidden="true"
      >
        FINAL WHISTLE
      </span>

      {/* In-world career context — club + season. Rendered only when a career
          is active. Omitting when inactive is the honesty rule: no permanent
          placeholder copy when there is no live data. */}
      <Show when={isCareerActive()}>
        <div class="flex items-center gap-4 text-xs font-mono text-ink-mute dark:text-paper-subtle">
          <Show when={managedClubName()}>
            {(name) => (
              <span class="text-ink dark:text-paper font-medium">{name()}</span>
            )}
          </Show>
          <Show when={seasonNumber() !== null}>
            <span>Season {seasonNumber()}</span>
          </Show>
        </div>
      </Show>

      {/* Theme toggle */}
      <button
        type="button"
        class="fw-nav-link"
        onClick={handleThemeToggle}
        aria-label="Toggle theme"
      >
        <span class="font-mono text-xs">
          {theme() === "light" ? "DARK" : "LIGHT"}
        </span>
      </button>
    </header>
  );
}

// ---------------------------------------------------------------------------
// PrimarySidebar — persistent w-56 left rail. Always visible; never a
// hamburger, never hover-only. This is the FM-classic pattern the FM26
// backlash identified as the most destructive navigation regression.
// ---------------------------------------------------------------------------

function PrimarySidebar(): JSX.Element {
  return (
    <aside
      class="w-56 shrink-0 flex flex-col border-r border-ink-mute/15 dark:border-midnight-line bg-paper-subtle dark:bg-midnight-panel overflow-y-auto"
      aria-label="Primary navigation"
    >
      {/* Primary nav — keyboard-navigable vertical list */}
      <nav aria-label="Primary" class="flex flex-col pt-2 pb-1">
        <For each={NAV}>
          {(item) => (
            <A
              href={item.to}
              class="fw-nav-link fw-nav-link--vertical"
              activeClass="active"
              end={item.to === "/"}
            >
              {item.label}
            </A>
          )}
        </For>
      </nav>

      {/* Career-context card — rendered only when a career is active.
          Honesty rule: omit every slot that has no live data rather than
          showing permanent em-dash placeholders. */}
      <Show when={isCareerActive()}>
        <CareerContextCard />
      </Show>
    </aside>
  );
}

// ---------------------------------------------------------------------------
// CareerContextCard — below-nav career summary: managed club + season +
// next fixture (next fixture slot only rendered when data exists).
// ---------------------------------------------------------------------------

function CareerContextCard(): JSX.Element {
  return (
    <div class="mx-3 mb-3 mt-2 border border-ink-mute/15 dark:border-midnight-line rounded p-2.5 space-y-2">
      <Show when={managedClubName()}>
        {(name) => (
          <div>
            <p class="text-[10px] uppercase tracking-wider text-ink-mute dark:text-paper-subtle">
              Club
            </p>
            <p class="mt-0.5 text-sm text-ink dark:text-paper font-medium leading-snug">
              {name()}
            </p>
          </div>
        )}
      </Show>
      <Show when={seasonNumber() !== null}>
        <div>
          <p class="text-[10px] uppercase tracking-wider text-ink-mute dark:text-paper-subtle">
            Season
          </p>
          <p class="mt-0.5 text-sm font-mono text-ink dark:text-paper">
            {seasonNumber()}
          </p>
        </div>
      </Show>
      {/*
        Next fixture slot is intentionally absent until a fixture signal exists.
        Render it only when live data is available — never show "Next fixture: —".
        This slot will be uncommented when the fixture signal is wired.
      */}
    </div>
  );
}

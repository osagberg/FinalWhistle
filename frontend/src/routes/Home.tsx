import {
  createResource,
  createSignal,
  createMemo,
  For,
  Show,
  type JSX,
} from "solid-js";
import { A, useNavigate } from "@solidjs/router";
import { getBackendHandshake, isTauri } from "~/lib/tauri";
import { loadCareer } from "~/lib/api/new-career";
import { getStandings, getFixtures } from "~/lib/api/season";
import { getPressInbox } from "~/lib/api/career";
import { IpcShapeError } from "~/lib/runtime-validators";
import { describeRouteError } from "~/lib/route-errors";
import {
  isCareerActive,
  managedClubName,
  selectedClubId,
  seasonNumber,
} from "~/lib/state";
import type {
  StandingsRow,
  FixtureWithResult,
  PressItemDto,
  PressTopicDto,
} from "~/lib/types";

// ---------------------------------------------------------------------------
// Fixture → (homeClubId, awayClubId) derivation (M2b)
//
// getFixtures() returns fixtures RELATIVE to the managed club. For an unplayed
// fixture, we need to derive the absolute (home, away) club ids so that
// startLiveMatchForFixture can construct the correct MatchState.
//
// Rule:
//   isHome === true  → managed club is at home: home = managedClubId, away = opponentClubId
//   isHome === false → managed club is away:    home = opponentClubId, away = managedClubId
// ---------------------------------------------------------------------------

interface FixtureClubIds {
  homeClubId: number;
  awayClubId: number;
}

/**
 * Home route — two modes:
 *
 *   Pre-career (isCareerActive() === false):
 *     Full-screen main-menu landing — wordmark, tagline, New Career / Load Save
 *     / Settings. Unchanged from the original design.
 *
 *   Active career (isCareerActive() === true):
 *     Inbox-heartbeat hub — "Bite" header (league position + next fixture) +
 *     press-inbox feed + quick-link cards. The authoritative "what do I do now"
 *     anchor for every session.
 *
 * Rules compliance:
 *   - UI never drives canonical state — all data fetches are read-only projections.
 *   - Honesty: data slots rendered ONLY when real data exists (no em-dash placeholders).
 *   - No `any` — all resources typed against closed DTOs.
 *   - Tailwind v3, pitch/ink/paper/midnight tokens, dark-mode on every color class.
 *   - Football-native copy only; no banned terms.
 */

// ---------------------------------------------------------------------------
// Seed generator — pre-career only
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Press topic label — closed switch with exhaustiveness guard
// ---------------------------------------------------------------------------

function pressTopicLabel(topic: PressTopicDto): string {
  switch (topic) {
    case "playerMilestone":  return "Milestone";
    case "matchResult":      return "Result";
    case "contractTransfer": return "Transfer";
    case "relational":       return "Story";
    default:
      // Exhaustiveness check: new PressTopicDto variant forces a compile error here.
      return ((_: never) => "Press")(topic);
  }
}

// ---------------------------------------------------------------------------
// Ordinal helper for league position display ("1st", "2nd", "3rd", "4th"…)
// ---------------------------------------------------------------------------

function ordinal(n: number): string {
  const s: string[] = ["th", "st", "nd", "rd"];
  const v = n % 100;
  return n + (s[(v - 20) % 10] ?? s[v] ?? "th");
}

// ---------------------------------------------------------------------------
// Pre-career main menu (full screen, unchanged)
// ---------------------------------------------------------------------------

function PreCareerMenu(): JSX.Element {
  const navigate = useNavigate();

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

  const [loadPending, setLoadPending] = createSignal(false);
  const [loadError, setLoadError] = createSignal<string | null>(null);

  const handleNewCareer = (): void => {
    const seedHex = generateSeedHex();
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
      {/* Wordmark */}
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
        <button
          type="button"
          class="w-full py-3 px-4 rounded text-sm font-display tracking-wider bg-pitch-500 text-paper hover:bg-pitch-600 focus:outline-none focus:ring-2 focus:ring-pitch-400 transition-colors"
          onClick={handleNewCareer}
        >
          NEW CAREER
        </button>

        <button
          type="button"
          disabled={loadPending()}
          class="w-full py-3 px-4 rounded text-sm font-display tracking-wider border border-ink-mute/30 dark:border-midnight-line text-ink-subtle dark:text-paper-subtle hover:text-ink dark:hover:text-paper hover:bg-paper-subtle dark:hover:bg-midnight-subtle focus:outline-none focus:ring-2 focus:ring-pitch-400 disabled:opacity-50 disabled:cursor-wait transition-colors"
          onClick={() => void handleLoadSave()}
        >
          {loadPending() ? "LOADING…" : "LOAD SAVE"}
        </button>

        <Show when={loadError()}>
          {(msg) => (
            <p class="text-xs text-flag-red" role="alert">
              {msg()}
            </p>
          )}
        </Show>

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

// ---------------------------------------------------------------------------
// Active-career hub
// ---------------------------------------------------------------------------

/**
 * Hub rendered when a career is active.
 *
 * Three zones:
 *   1. Bite header — club name + league position + next fixture (slots omitted
 *      when data is absent — honesty rule, no em-dash placeholders).
 *   2. Press feed — getPressInbox() in reverse-salience order (backend orders by
 *      projected salience DESC; we render as-received). This is the "what's
 *      happened" surface. A clear seam comment marks where "must-respond / blocks
 *      time-advance" items will slot in once the gameplay system lands.
 *   3. Quick links — squad, league, fixtures, next match.
 *
 * Data fetches are Tauri-only (invoke). They are gated behind isCareerActive()
 * in the parent so the pre-career menu still renders cleanly in a plain browser.
 */
function ActiveCareerHub(): JSX.Element {
  const clubId = selectedClubId;
  const navigate = useNavigate();

  // ── standings ──────────────────────────────────────────────────────────────
  // Fetch standings once on mount to derive managed-club position.

  const [standings] = createResource<StandingsRow[] | null>(async () => {
    try {
      return await getStandings();
    } catch (e: unknown) {
      if (e instanceof IpcShapeError) {
        // eslint-disable-next-line no-console
        console.error("[Home/Hub] standings DTO drift:", e.command, e.reason);
      } else {
        // eslint-disable-next-line no-console
        console.error("[Home/Hub] getStandings failed:", e);
      }
      return null;
    }
  });

  // ── fixtures ───────────────────────────────────────────────────────────────
  // Fetch all fixtures for the managed club; derive next unplayed entry.

  const [fixtures] = createResource<FixtureWithResult[] | null>(async () => {
    const id = clubId();
    if (id === null) return null;
    const numericId = parseInt(id, 10);
    if (isNaN(numericId)) return null;
    try {
      return await getFixtures(numericId);
    } catch (e: unknown) {
      if (e instanceof IpcShapeError) {
        // eslint-disable-next-line no-console
        console.error("[Home/Hub] fixtures DTO drift:", e.command, e.reason);
      } else {
        // eslint-disable-next-line no-console
        console.error("[Home/Hub] getFixtures failed:", e);
      }
      return null;
    }
  });

  // ── press inbox ─────────────────────────────────────────────────────────────

  const [pressItems] = createResource<PressItemDto[] | null>(async () => {
    try {
      const inbox = await getPressInbox();
      return inbox.items;
    } catch (e: unknown) {
      if (e instanceof IpcShapeError) {
        // eslint-disable-next-line no-console
        console.error("[Home/Hub] press inbox DTO drift:", e.command, e.reason);
      } else {
        // eslint-disable-next-line no-console
        console.error("[Home/Hub] getPressInbox failed:", e);
      }
      return null;
    }
  });

  // ── derived memos ──────────────────────────────────────────────────────────

  /**
   * Find the managed club's standings row.
   * selectedClubId is stored as a string in state (it comes from ClubChoiceDto.clubId
   * converted via String(); we parse it back to a number for comparison).
   */
  const managedRow = createMemo<StandingsRow | null>(() => {
    const rows = standings();
    const id = clubId();
    if (!rows || id === null) return null;
    const numericId = parseInt(id, 10);
    if (isNaN(numericId)) return null;
    return rows.find((r) => r.clubId === numericId) ?? null;
  });

  /** Position ordinal derived from standings sort order (1-based index). */
  const leaguePosition = createMemo<{ pos: number; total: number } | null>(() => {
    const rows = standings();
    const id = clubId();
    if (!rows || id === null) return null;
    const numericId = parseInt(id, 10);
    if (isNaN(numericId)) return null;
    const idx = rows.findIndex((r) => r.clubId === numericId);
    if (idx === -1) return null;
    return { pos: idx + 1, total: rows.length };
  });

  /** Next unplayed fixture for the managed club. */
  const nextFixture = createMemo<FixtureWithResult | null>(() => {
    const all = fixtures();
    if (!all) return null;
    return all.find((f) => !f.played) ?? null;
  });

  /**
   * Derives the absolute (homeClubId, awayClubId) pair for the next unplayed
   * fixture so we can pass it directly to startLiveMatchForFixture.
   *
   * getFixtures() returns fixtures relative to the managed club:
   *   isHome === true  → home = managedClubId, away = opponentClubId
   *   isHome === false → home = opponentClubId, away = managedClubId
   *
   * Returns null when there is no unplayed fixture or the managed club id
   * cannot be parsed.
   */
  const watchFixtureIds = createMemo<FixtureClubIds | null>(() => {
    const nf = nextFixture();
    const id = clubId();
    if (!nf || id === null) return null;
    const managedClubId = parseInt(id, 10);
    if (isNaN(managedClubId)) return null;
    if (nf.isHome) {
      return { homeClubId: managedClubId, awayClubId: nf.opponentClubId };
    }
    return { homeClubId: nf.opponentClubId, awayClubId: managedClubId };
  });

  function handleWatchMatch(): void {
    const ids = watchFixtureIds();
    if (!ids) return;
    navigate("/live-match", { state: ids });
  }

  return (
    <div class="space-y-6">
      {/* ── Bite header ─────────────────────────────────────────────────────── */}
      <header aria-label="Career snapshot">
        {/* Club name — Anton display face, pitch-green */}
        <Show when={managedClubName()}>
          {(name) => (
            <h1 class="font-display text-3xl tracking-wider text-pitch-600 dark:text-pitch-300">
              {name()}
            </h1>
          )}
        </Show>

        {/* Season tag — rendered only when data exists */}
        <Show when={seasonNumber() !== null}>
          <p class="mt-0.5 text-xs font-mono text-ink-mute dark:text-paper-subtle uppercase tracking-wider">
            Season {seasonNumber()}
          </p>
        </Show>

        {/* Bite stats — league position + next fixture, each slot honesty-gated */}
        <Show when={!standings.loading && !fixtures.loading}>
          <div class="mt-3 flex flex-wrap gap-6">
            {/* League position — omitted until standings resolve */}
            <Show when={leaguePosition()}>
              {(lp) => (
                <div>
                  <p class="text-[10px] uppercase tracking-wider text-ink-mute dark:text-paper-subtle">
                    League position
                  </p>
                  <p class="mt-0.5 font-mono text-2xl text-ink dark:text-paper">
                    {ordinal(lp().pos)}
                    <span class="text-sm text-ink-mute dark:text-paper-subtle ml-1">
                      of {lp().total}
                    </span>
                  </p>
                  {/* Points — from the managed row */}
                  <Show when={managedRow()}>
                    {(row) => (
                      <p class="text-xs font-mono text-ink-mute dark:text-paper-subtle">
                        {row().points} pts
                        <span class="ml-2">
                          {row().wins}W {row().draws}D {row().losses}L
                        </span>
                      </p>
                    )}
                  </Show>
                </div>
              )}
            </Show>

            {/* Next fixture — omitted when all matches are played or data missing */}
            <Show when={nextFixture()}>
              {(nf) => (
                <div class="space-y-1.5">
                  <p class="text-[10px] uppercase tracking-wider text-ink-mute dark:text-paper-subtle">
                    Next fixture
                  </p>
                  <p class="mt-0.5 font-mono text-base text-ink dark:text-paper">
                    {nf().isHome ? "vs" : "@"}{" "}
                    <span class="text-pitch-600 dark:text-pitch-300">
                      {nf().opponentClubName}
                    </span>
                  </p>
                  <p class="text-xs font-mono text-ink-mute dark:text-paper-subtle">
                    Match day {nf().matchDay} · {nf().isHome ? "Home" : "Away"}
                  </p>
                  {/*
                    Watch entry-point (M2b): visible only when a managed club +
                    unplayed fixture both exist (watchFixtureIds() handles both
                    guards). Navigates to /live-match with { homeClubId, awayClubId }
                    in router state so LiveMatch calls startLiveMatchForFixture.

                    Read-only deterministic preview — advance_week is still the
                    authoritative play of the round (M3 will wire the authoritative
                    path here without changing this button).
                  */}
                  <Show when={watchFixtureIds()}>
                    <button
                      type="button"
                      class="px-3 py-1 rounded text-xs font-mono bg-pitch-500 text-paper hover:bg-pitch-600 focus:outline-none focus:ring-2 focus:ring-pitch-400 transition-colors"
                      onClick={handleWatchMatch}
                      aria-label={`Watch the match against ${nf().opponentClubName}`}
                    >
                      Watch this match
                    </button>
                  </Show>
                </div>
              )}
            </Show>
          </div>
        </Show>

        {/* Loading state for bite stats — minimal, non-blocking */}
        <Show when={standings.loading || fixtures.loading}>
          <p class="mt-3 text-xs font-mono text-ink-mute dark:text-paper-subtle">
            Loading…
          </p>
        </Show>
      </header>

      {/* ── Quick links ─────────────────────────────────────────────────────── */}
      <nav aria-label="Quick links">
        <p class="text-[10px] uppercase tracking-wider text-ink-mute dark:text-paper-subtle mb-2">
          Management
        </p>
        <div class="flex flex-wrap gap-2">
          <QuickLink href="/squad"   label="Squad"      />
          <QuickLink href="/league"  label="League"     />
          <QuickLink href="/fixtures" label="Fixtures"  />
          <QuickLink href="/match"   label="Next match" />
        </div>
      </nav>

      {/* ── Press feed ──────────────────────────────────────────────────────── */}
      <section aria-label="Press feed">
        <h2 class="font-display text-lg tracking-wide text-ink dark:text-paper mb-3">
          Press
        </h2>

        {/*
          FUTURE SEAM — must-respond items
          ─────────────────────────────────────────────────────────────────────
          When the gameplay system lands a "pending press response" or
          "time-advance blocker" surface (T4+ career-loop milestone), render
          a "Must respond" section here above the feed, with flag-red left
          borders on each blocking item. Those items need a new IPC command
          (e.g. `get_pending_manager_actions`) and a `PressResponseDto` DTO —
          do NOT fabricate them here. The press feed below is the "what's
          happened" read-only surface; the must-respond section is the
          "what must you do next" write surface. Keep them visually separate.
          ─────────────────────────────────────────────────────────────────────
        */}

        <Show when={pressItems.loading}>
          <p class="text-sm text-ink-mute dark:text-paper-subtle font-mono">
            Loading…
          </p>
        </Show>

        <Show when={!pressItems.loading}>
          <Show
            when={(pressItems() ?? []).length > 0}
            fallback={
              <p class="text-sm text-ink-mute dark:text-paper-subtle">
                Nothing in the press yet — results and milestones will surface here as the season unfolds.
              </p>
            }
          >
            <ol
              class="space-y-px"
              aria-label="Press items, most recent first"
            >
              <For each={pressItems() ?? []}>
                {(item) => <PressItem item={item} />}
              </For>
            </ol>
          </Show>
        </Show>

        {/* Silent error — press failure doesn't crash the hub; just omit the feed */}
        <Show when={!pressItems.loading && pressItems() === null}>
          <p class="text-sm text-ink-mute dark:text-paper-subtle">
            Press feed unavailable right now.
          </p>
        </Show>
      </section>
    </div>
  );
}

// ---------------------------------------------------------------------------
// PressItem — one row in the feed
// ---------------------------------------------------------------------------

function PressItem(props: { item: PressItemDto }): JSX.Element {
  return (
    <li class="border-l-2 border-ink-mute/20 dark:border-midnight-line pl-3 py-1.5">
      <div class="flex items-baseline gap-2">
        {/* Topic badge */}
        <span
          class="text-[10px] font-mono uppercase tracking-wider text-ink-mute dark:text-paper-subtle shrink-0"
          aria-label={`Topic: ${pressTopicLabel(props.item.topic)}`}
        >
          {pressTopicLabel(props.item.topic)}
        </span>
        {/* Season tag */}
        <span class="text-[10px] font-mono text-ink-mute/60 dark:text-paper-subtle/50 shrink-0">
          S{props.item.season}
        </span>
      </div>
      {/* Headline — football-native prose */}
      <p class="mt-0.5 text-sm text-ink dark:text-paper leading-snug">
        {props.item.headline}
      </p>
      {/* Manager quote — rendered only when present */}
      <Show when={props.item.managerQuote}>
        {(quote) => (
          <p class="mt-0.5 text-xs text-ink-mute dark:text-paper-subtle italic">
            "{quote()}"
          </p>
        )}
      </Show>
    </li>
  );
}

// ---------------------------------------------------------------------------
// QuickLink — a card-style nav link for the hub's quick-links row
// ---------------------------------------------------------------------------

function QuickLink(props: { href: string; label: string }): JSX.Element {
  return (
    <A
      href={props.href}
      class="px-4 py-2 rounded border border-ink-mute/20 dark:border-midnight-line text-sm font-body text-ink-subtle dark:text-paper-subtle hover:text-ink dark:hover:text-paper hover:bg-paper-subtle dark:hover:bg-midnight-subtle focus:outline-none focus:ring-2 focus:ring-pitch-400 transition-colors"
    >
      {props.label}
    </A>
  );
}

// ---------------------------------------------------------------------------
// Root export — toggles between pre-career menu and active-career hub
// ---------------------------------------------------------------------------

export default function Home(): JSX.Element {
  return (
    <Show when={isCareerActive()} fallback={<PreCareerMenu />}>
      <ActiveCareerHub />
    </Show>
  );
}

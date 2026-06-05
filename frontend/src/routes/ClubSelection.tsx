/*
 * Club-selection screen — B4.
 *
 * Flow:
 *   1. On mount: call `newCareer(seedHex)` (seed generated on the Home screen,
 *      passed via location state or query param), then `getClubs()`.
 *   2. Render 20 clubs as a keyboard-navigable list.
 *   3. On pick: `selectManagedClub(clubId)` → navigate to /squad.
 *
 * The seed is a 16-hex-char string like "0xfeedbeefcafefade", generated
 * client-side via `crypto.getRandomValues` on the Home screen and carried in
 * router location state as `{ seedHex: string }`.
 *
 * IPC calls at mount make this route Tauri-only — NOT browser-safe.
 * Visual proof comes from the desktop build, not the browser dev harness.
 *
 * Rules compliance:
 *   - SolidJS only — createSignal / createResource / createMemo (Frontend/RULES §1)
 *   - Tailwind v3 utility classes only (Frontend/RULES §2)
 *   - No `any` — typed via lib/types.ts (Frontend/RULES §6)
 *   - Always await IPC calls (Frontend/RULES §7)
 *   - Full keyboard nav + ARIA labels on the list (Frontend/RULES §8)
 *   - Football-native copy only — no banned terms (Frontend/RULES §9)
 */

import {
  createResource,
  createSignal,
  createEffect,
  For,
  Show,
  type JSX,
} from "solid-js";
import { useNavigate, useLocation } from "@solidjs/router";
import { newCareer, getClubs, selectManagedClub } from "~/lib/api/new-career";
import ErrorBoundary from "~/components/ErrorBoundary";
import Loading from "~/components/Loading";
import { IpcShapeError } from "~/lib/runtime-validators";
import { describeRouteError } from "~/lib/route-errors";
import {
  setCareerId,
  setSelectedClubId,
  setManagedClubName,
} from "~/lib/state";
import type { ClubChoiceDto, IpcError } from "~/lib/types";

// ---------------------------------------------------------------------------
// IpcError narrowing (self-contained per project convention)
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
  "settingsLoadFailed",
  "notYetObserved",
  "leagueGenerationFailed",
  "saveLoadFailed",
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
// Location state shape
// ---------------------------------------------------------------------------

interface ClubSelectionState {
  seedHex: string;
}

function isClubSelectionState(v: unknown): v is ClubSelectionState {
  return (
    typeof v === "object" &&
    v !== null &&
    typeof (v as Record<string, unknown>).seedHex === "string"
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export default function ClubSelection(): JSX.Element {
  return (
    <ErrorBoundary label="Club selection">
      <ClubSelectionInner />
    </ErrorBoundary>
  );
}

function ClubSelectionInner(): JSX.Element {
  const navigate = useNavigate();
  const location = useLocation();

  // Resolve the world seed ONCE for this screen instance: from router location
  // state (set by Home), or a fresh random seed if absent (direct URL hit in
  // dev). Captured as a const — NOT a re-callable function — so newCareer(),
  // the displayed seed, and the stored career id all use the SAME value. A
  // function would re-roll crypto.getRandomValues on every call on the fallback
  // path, desyncing the generated world from the stored career id.
  const seed: string = (() => {
    const state: unknown = location.state;
    if (isClubSelectionState(state)) return state.seedHex;
    const bytes = new Uint8Array(8);
    crypto.getRandomValues(bytes);
    return (
      "0x" +
      Array.from(bytes)
        .map((b) => b.toString(16).padStart(2, "0"))
        .join("")
    );
  })();

  const [setupError, setSetupError] = createSignal<IpcError | Error | null>(
    null,
  );
  const [pickPending, setPickPending] = createSignal(false);
  const [pickError, setPickError] = createSignal<IpcError | Error | null>(null);
  const [focusedIndex, setFocusedIndex] = createSignal(0);

  // Keep the keyboard-focused option scrolled into view for a long club list
  // (Frontend/RULES §8 — keyboard nav must move the visible highlight too).
  const liRefs: (HTMLLIElement | undefined)[] = [];
  createEffect(() => {
    liRefs[focusedIndex()]?.scrollIntoView({ block: "nearest" });
  });

  // On mount: newCareer → getClubs (sequential — clubs depend on a world existing).
  const [clubs] = createResource<ClubChoiceDto[]>(async () => {
    setSetupError(null);
    try {
      await newCareer(seed);
      return await getClubs();
    } catch (e: unknown) {
      if (e instanceof IpcShapeError) {
        // eslint-disable-next-line no-console
        console.error(
          "[ClubSelection] IPC shape drift:",
          e.command,
          e.reason,
          e.payloadPreview,
        );
      } else {
        // eslint-disable-next-line no-console
        console.error("[ClubSelection] setup failed:", e);
      }
      setSetupError(normaliseError(e));
      return [];
    }
  });

  const handlePickClub = async (club: ClubChoiceDto): Promise<void> => {
    if (pickPending()) return;
    setPickPending(true);
    setPickError(null);
    try {
      await selectManagedClub(club.clubId);
      // Update app-wide state so the sidebar reflects the chosen club.
      setCareerId(seed);
      setSelectedClubId(String(club.clubId));
      setManagedClubName(club.clubName);
      navigate("/squad");
    } catch (e: unknown) {
      // eslint-disable-next-line no-console
      console.error("[ClubSelection] selectManagedClub failed:", e);
      setPickError(normaliseError(e));
    } finally {
      setPickPending(false);
    }
  };

  // Keyboard nav handler for the club list.
  const handleListKeyDown = (
    e: KeyboardEvent,
    clubList: ClubChoiceDto[],
  ): void => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setFocusedIndex((i) => Math.min(i + 1, clubList.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setFocusedIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter" || e.key === " ") {
      const club = clubList[focusedIndex()];
      if (club) void handlePickClub(club);
    }
  };

  return (
    <div class="space-y-4">
      <header>
        <h1 class="font-display text-3xl text-pitch-600 dark:text-pitch-300">
          Choose your club
        </h1>
        <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
          Pick the side you'll take into your first season.
        </p>
      </header>

      {/* Seed display + re-roll */}
      <SeedRow seedHex={seed} />

      {/* Loading state */}
      <Show when={clubs.loading}>
        <Loading message="Generating league…" />
      </Show>

      {/* Error state */}
      <Show when={!clubs.loading && setupError()}>
        {(err) => {
          const copy = describeRouteError(err(), { what: "the club list" });
          return (
            <div
              class="fw-panel p-4 text-sm text-rose-600 dark:text-rose-400"
              role="alert"
            >
              <p class="font-semibold">{copy.headline}</p>
              <p class="mt-1 text-ink-subtle dark:text-paper-subtle">
                {copy.detail}
              </p>
            </div>
          );
        }}
      </Show>

      {/* Pick error (after list loads — club selection failed) */}
      <Show when={pickError()}>
        {(err) => {
          const copy = describeRouteError(err(), { what: "the club selection" });
          return (
            <div
              class="fw-panel p-4 text-sm text-rose-600 dark:text-rose-400"
              role="alert"
            >
              <p class="font-semibold">{copy.headline}</p>
              <p class="mt-1 text-ink-subtle dark:text-paper-subtle">
                {copy.detail}
              </p>
            </div>
          );
        }}
      </Show>

      {/* Club list */}
      <Show when={!clubs.loading && !setupError() && (clubs() ?? []).length > 0}>
        {/* Pending overlay copy */}
        <Show when={pickPending()}>
          <p class="text-sm text-ink-mute dark:text-paper-subtle" aria-live="polite">
            Confirming selection…
          </p>
        </Show>

        <ul
          role="listbox"
          aria-label="Available clubs"
          aria-activedescendant={`club-option-${focusedIndex()}`}
          tabIndex={0}
          class="fw-panel divide-y divide-ink-mute/10 dark:divide-midnight-line focus:outline-none focus:ring-2 focus:ring-pitch-400"
          onKeyDown={(e) => {
            const list = clubs() ?? [];
            handleListKeyDown(e, list);
          }}
        >
          <For each={clubs() ?? []}>
            {(club, idx) => (
              <li
                id={`club-option-${idx()}`}
                role="option"
                aria-selected={idx() === focusedIndex()}
                class="flex items-center justify-between px-4 py-3 cursor-pointer select-none transition-colors"
                classList={{
                  "bg-pitch-50 dark:bg-pitch-900/30 text-pitch-700 dark:text-pitch-300":
                    idx() === focusedIndex(),
                  "text-ink dark:text-paper hover:bg-paper-subtle dark:hover:bg-midnight-subtle":
                    idx() !== focusedIndex(),
                  "opacity-60 cursor-wait": pickPending(),
                }}
                onClick={() => void handlePickClub(club)}
                onMouseEnter={() => setFocusedIndex(idx())}
              >
                <span class="font-body text-sm">{club.clubName}</span>
                <span
                  class="text-xs text-ink-mute dark:text-paper-subtle font-mono"
                  aria-hidden="true"
                >
                  select →
                </span>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}

// ---------------------------------------------------------------------------
// SeedRow — displays the world seed + offers a re-roll link.
//
// Re-roll navigates back to Home — the seed is generated fresh there.
// ---------------------------------------------------------------------------

interface SeedRowProps {
  seedHex: string;
}

function SeedRow(props: SeedRowProps): JSX.Element {
  const navigate = useNavigate();
  return (
    <div class="flex items-center gap-3 text-xs text-ink-mute dark:text-paper-subtle font-mono">
      <span>World seed: {props.seedHex}</span>
      <button
        type="button"
        class="underline hover:text-ink dark:hover:text-paper focus:outline-none focus:ring-1 focus:ring-pitch-400 rounded"
        aria-label="Re-roll world seed and return to start"
        onClick={() => navigate("/")}
      >
        re-roll
      </button>
    </div>
  );
}

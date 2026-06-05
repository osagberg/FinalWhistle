/*
 * Squad page — T4-2.5h.
 *
 * Text-first per DESIGN_DOC.md §3. Roster-based player list (TanStack Table
 * v8) backed by `get_squad_roster` IPC command. Shows the DEFAULT club's
 * 22-player roster with season stats.
 *
 * Columns: Player, Role (derived from slot), Apps, Goals, Minutes.
 * Region and Traits are dropped — they live on PlayerBio, accessible via
 * the Player detail page.
 *
 * The default club is the lowest ClubId in career.roster — a deterministic
 * stand-in until career-start club selection is implemented.
 *
 * IPC (read-only via squad.ts wrapper):
 *   getSquadRoster() → SquadRosterDto    — loaded on mount
 *
 * Rules compliance:
 *   - No `any` (Frontend/RULES.md §6)
 *   - Solid signals / createResource, not React hooks (Frontend/RULES.md §1)
 *   - IpcError exhaustive switch + never discriminant (matches League.tsx pattern)
 *   - Dark-mode tokens on every color-bearing class (Frontend/RULES.md §2)
 *   - Column defs in ~/lib/columns/squad.columns.ts (Frontend/RULES.md §3)
 *   - UI never drives canonical state — read-only fetch only
 */

import { createResource, createSignal, Show, type JSX } from "solid-js";
import DataTable from "~/components/DataTable";
import ErrorBoundary from "~/components/ErrorBoundary";
import Loading from "~/components/Loading";
import { getSquadRoster } from "~/lib/api/squad";
import { rosterColumns } from "~/lib/columns/squad.columns";
import { IpcShapeError } from "~/lib/runtime-validators";
import { describeRouteError } from "~/lib/route-errors";
import type { IpcError, SquadRosterDto } from "~/lib/types";

// ---------------------------------------------------------------------------
// IpcError type guard + exhaustiveness helper
//
// Self-contained per project convention — shared helpers go in ~/lib only if
// needed by 3+ routes. Mirrors the pattern in routes/League.tsx.
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
  // T4-F4: scouting variants.
  "notYetObserved",
  "leagueGenerationFailed",
] as const) satisfies ReadonlySet<IpcError["kind"]>;

function isIpcError(e: unknown): e is IpcError {
  if (typeof e !== "object" || e === null || !("kind" in e)) return false;
  const kind = (e as Record<string, unknown>).kind;
  return (
    typeof kind === "string" &&
    (KNOWN_IPC_ERROR_KINDS as ReadonlySet<string>).has(kind)
  );
}

/** Normalise an arbitrary thrown value into the structured shape the error signal wants. */
function normaliseError(e: unknown): IpcError | Error {
  if (isIpcError(e)) return e;
  if (e instanceof Error) return e;
  return new Error(String(e));
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export default function Squad(): JSX.Element {
  return (
    <ErrorBoundary label="Squad">
      <SquadInner />
    </ErrorBoundary>
  );
}

function SquadInner(): JSX.Element {
  const [squadError, setSquadError] = createSignal<IpcError | Error | null>(
    null,
  );

  // Resource: drives getSquadRoster() on mount.
  // Returns null on failure (distinct from an empty roster which would be a
  // structural error — all careers start with 22 players).
  const [roster] = createResource<SquadRosterDto | null>(async () => {
    setSquadError(null);
    try {
      return await getSquadRoster();
    } catch (e: unknown) {
      if (e instanceof IpcShapeError) {
        // Backend DTO contract drift — log as its own greppable class so a
        // SquadRosterDto shape mismatch is distinguishable from a transient
        // runtime error during triage.
        // eslint-disable-next-line no-console
        console.error(
          "[Squad] get_squad_roster DTO contract drift:",
          e.command,
          e.reason,
          e.payloadPreview,
        );
      } else {
        // eslint-disable-next-line no-console
        console.error("[Squad] getSquadRoster failed:", e);
      }
      setSquadError(normaliseError(e));
      return null;
    }
  });

  return (
    <div class="space-y-4">
      <header>
        <h1 class="font-display text-3xl text-pitch-600 dark:text-pitch-300">
          Squad
        </h1>
        {/* Sub-header: show managed club name when selected; honest placeholder
            when showing the default lowest-ClubId stand-in.
            `!= null` (loose) so the undefined-while-loading resource value is
            also gated, not just an explicit null. */}
        <Show when={roster() != null && !roster.loading}>
          {roster()?.isManaged ? (
            <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
              <span class="font-medium text-ink dark:text-paper">
                {roster()?.clubName ?? ""}
              </span>
            </p>
          ) : (
            <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
              No club selected yet — showing{" "}
              <span class="font-medium text-ink dark:text-paper">
                {roster()?.clubName ?? ""}
              </span>
            </p>
          )}
        </Show>
      </header>

      {/* Loading state first — no white-flash before spinner. */}
      <Show when={roster.loading}>
        <Loading message="Loading squad…" />
      </Show>

      {/* Player list: error / data states (only when not loading). */}
      <Show when={!roster.loading}>
        <Show
          when={squadError()}
          fallback={
            <Show when={roster() != null}>
              <DataTable
                columns={rosterColumns}
                data={roster()?.players ?? []}
                emptyMessage="No players in the squad — career roster may be empty."
              />
            </Show>
          }
        >
          {(err) => {
            const copy = describeRouteError(err(), { what: "the squad" });
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
      </Show>
    </div>
  );
}

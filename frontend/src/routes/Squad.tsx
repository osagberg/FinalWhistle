/*
 * Squad page — T2-7.
 *
 * Text-first per DESIGN_DOC.md §3. Player list (TanStack Table v8) backed by
 * `get_squad` IPC command. Renders the 22-player pool from the content store.
 *
 * Columns: Player, Role, Region, Traits (phenotype labels).
 * Age and contract columns are absent by design — they are T4+ career-roster
 * state that PlayerBio does not carry.
 *
 * IPC (read-only via squad.ts wrapper):
 *   getSquad() → SquadPlayer[]    — loaded on mount
 *
 * Rules compliance:
 *   - No `any` (Frontend/RULES.md §6)
 *   - Solid signals / createResource, not React hooks (Frontend/RULES.md §1)
 *   - IpcError exhaustive switch + never discriminant (matches League.tsx pattern)
 *   - Dark-mode tokens on every color-bearing class (Frontend/RULES.md §2)
 *   - Column defs split into ~/lib/columns/squad.columns.ts (Frontend/RULES.md §3)
 *   - UI never drives canonical state — read-only fetch only
 */

import { createResource, createSignal, Show, type JSX } from "solid-js";
import DataTable from "~/components/DataTable";
import { getSquad } from "~/lib/api/squad";
import { squadColumns } from "~/lib/columns/squad.columns";
import { IpcShapeError } from "~/lib/runtime-validators";
import type { IpcError, SquadPlayer } from "~/lib/types";

// ---------------------------------------------------------------------------
// IpcError type guard + exhaustiveness helper
//
// Self-contained per project convention — shared helpers go in ~/lib only if
// needed by 3+ routes. Mirrors the pattern in routes/League.tsx.
//
// The `satisfies` annotation pins KNOWN_IPC_ERROR_KINDS to IpcError["kind"],
// so adding a new variant to the IpcError union in lib/types.ts produces a
// compile error here, forcing a coordinated update.
// ---------------------------------------------------------------------------

const KNOWN_IPC_ERROR_KINDS = new Set([
  "tooManyFrames",
  "invalidSeed",
  "matchInitFailed",
  "seasonComplete",
  "clubNotFound",
  "lockPoisoned",
  "playerNotFound",
] as const) satisfies ReadonlySet<IpcError["kind"]>;

function isIpcError(e: unknown): e is IpcError {
  if (typeof e !== "object" || e === null || !("kind" in e)) return false;
  const kind = (e as Record<string, unknown>).kind;
  return (
    typeof kind === "string" &&
    (KNOWN_IPC_ERROR_KINDS as ReadonlySet<string>).has(kind)
  );
}

/**
 * Format an IpcError into a human-readable string.
 *
 * Exhaustive switch — adding a new IpcError variant forces a compile error at
 * the `never` default arm unless the arm is handled AND the runtime guard
 * above is updated.
 */
function formatIpcError(err: IpcError): string {
  switch (err.kind) {
    case "tooManyFrames":
      return `Too many ticks requested (${err.requested}; max ${err.max}).`;
    case "invalidSeed":
      return `Invalid seed "${err.input}": ${err.reason}`;
    case "matchInitFailed":
      return `Match could not start: ${err.reason}`;
    case "seasonComplete":
      return "The season is already complete — no more match-days to play.";
    case "clubNotFound":
      return `Club id ${err.clubId} was not found in the current league.`;
    case "lockPoisoned":
      return `Internal state was corrupted by a prior error (lock: ${err.lock}). Please restart the app.`;
    case "playerNotFound":
      return `Player "${err.playerId}" was not found in the content store.`;
    default: {
      const _exhaustive: never = err;
      throw new Error(
        `formatIpcError: unhandled IpcError variant — KNOWN_IPC_ERROR_KINDS / formatIpcError drift. err=${JSON.stringify(_exhaustive)}`,
      );
    }
  }
}

/** Describe any thrown value as a display string. */
function describeError(e: unknown): string {
  if (isIpcError(e)) return formatIpcError(e);
  // IpcShapeError is the backend-contract-drift signal — surface it as its
  // own message class rather than flattening it into the generic Error path,
  // so a SquadPlayerDto ↔ SquadPlayer mismatch reads as a version skew, not
  // a transient runtime fault.
  if (e instanceof IpcShapeError) {
    return `The backend sent an unexpected response shape for '${e.command}'. This is a version mismatch between the app and the sim — please report it.`;
  }
  if (e instanceof Error) return e.message;
  return String(e);
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
  // Fetch error — typed signal so it's fully reactive in the jsdom test env.
  const [squadError, setSquadError] = createSignal<IpcError | Error | null>(
    null,
  );

  // Resource: drives getSquad() on mount.
  // Returns null on failure (distinct from [] which means "store is empty").
  const [squad] = createResource<SquadPlayer[] | null>(async () => {
    setSquadError(null);
    try {
      return await getSquad();
    } catch (e: unknown) {
      if (e instanceof IpcShapeError) {
        // Backend DTO contract drift — log as its own greppable class so a
        // SquadPlayerDto ↔ SquadPlayer mismatch is distinguishable from a
        // transient runtime error during triage.
        // eslint-disable-next-line no-console
        console.error(
          "[Squad] get_squad DTO contract drift:",
          e.command,
          e.reason,
          e.payloadPreview,
        );
      } else {
        // eslint-disable-next-line no-console
        console.error("[Squad] getSquad failed:", e);
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
        {/* Sub-header: short honest description of this page's current scope.
            Age and contract columns arrive with the T4+ career-roster layer. */}
        <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
          Player pool — role, region, and scouted traits. Career details arrive
          with the T4 roster layer.
        </p>
      </header>

      {/* Player list: loading / error / data states. */}
      <Show
        when={squadError()}
        fallback={
          <Show
            when={!squad.loading && squad() !== null}
            fallback={
              <div class="fw-panel p-4 text-sm text-ink-mute dark:text-paper-subtle">
                Loading squad…
              </div>
            }
          >
            <DataTable
              columns={squadColumns}
              data={squad() ?? []}
              emptyMessage="No players in the pool — content store may be empty."
            />
          </Show>
        }
      >
        {(err) => (
          <div
            class="fw-panel p-4 text-sm font-mono text-flag-red"
            role="alert"
          >
            Failed to load squad: {describeError(err())}
          </div>
        )}
      </Show>
    </div>
  );
}

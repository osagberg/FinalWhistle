/*
 * Player detail page — T3-6.
 *
 * Text-first per DESIGN_DOC.md §3. Three blocks:
 *   1. Phenotype — name, role, region, scout labels.
 *   2. Memory callbacks — rendered career moments (honest empty state when ledger is empty).
 *   3. Contract — deferred placeholder (T4+ career-roster layer).
 *
 * IPC (read-only via player.ts wrapper):
 *   getPlayerDetail(id) → PlayerDetail    — loaded on mount from route param
 *
 * Rules compliance:
 *   - No `any` (Frontend/RULES.md §6)
 *   - Solid signals / createResource + createSignal, not React hooks (Frontend/RULES.md §1)
 *   - IpcError exhaustive switch + never discriminant (mirrors Squad.tsx pattern)
 *   - Dark-mode tokens on every color-bearing class (Frontend/RULES.md §2)
 *   - Explicit try/catch in resource fetcher preserves original thrown value so
 *     isIpcError() can match the plain object Tauri throws — avoids Solid's
 *     createResource wrapping the rejection in a generic Error.
 *   - UI never drives canonical state — read-only fetch only
 */

import { createResource, createSignal, For, Show, type JSX } from "solid-js";
import { useParams } from "@solidjs/router";
import { getPlayerDetail } from "~/lib/api/player";
import { IpcShapeError } from "~/lib/runtime-validators";
import type { IpcError, PlayerDetail } from "~/lib/types";

// ---------------------------------------------------------------------------
// IpcError type guard + exhaustiveness helper
//
// Self-contained per project convention — mirrors Squad.tsx / League.tsx.
// `satisfies` pins KNOWN_IPC_ERROR_KINDS to IpcError["kind"] so adding a new
// variant to the IpcError union forces a compile error here.
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
  // own message class rather than flattening it into the generic Error path.
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

export default function Player(): JSX.Element {
  const params = useParams<{ id: string }>();

  // Fetch error — typed signal so it's fully reactive in the jsdom test env.
  // Mirrors Squad.tsx: explicit try/catch preserves the original thrown value
  // (IpcError plain object). Without this, Solid's createResource wraps the
  // rejection in a generic Error and the `kind` discriminant is lost, so
  // isIpcError() can't match and describeError() returns "Unknown error".
  const [playerError, setPlayerError] = createSignal<IpcError | Error | null>(
    null,
  );

  // Resource: drives getPlayerDetail() on mount (re-runs when params.id changes).
  // Returns null on failure (distinct from missing player, which the server
  // signals via IpcError::PlayerNotFound before the resource ever resolves).
  const [detail] = createResource<PlayerDetail | null, string>(
    () => params.id,
    async (id) => {
      setPlayerError(null);
      try {
        return await getPlayerDetail(id);
      } catch (e: unknown) {
        if (e instanceof IpcShapeError) {
          // Backend DTO contract drift — log as its own greppable class.
          // eslint-disable-next-line no-console
          console.error(
            "[Player] get_player_detail DTO contract drift:",
            e.command,
            e.reason,
            e.payloadPreview,
          );
        } else {
          // eslint-disable-next-line no-console
          console.error("[Player] getPlayerDetail failed:", e);
        }
        setPlayerError(normaliseError(e));
        return null;
      }
    },
  );

  return (
    <div class="space-y-6">
      {/* Error path — shown when the resource fetch threw (IpcError or drift). */}
      <Show
        when={playerError()}
        fallback={
          <Show
            when={!detail.loading && detail() !== null}
            fallback={
              <div class="fw-panel p-4 text-sm text-ink-mute dark:text-paper-subtle">
                Loading player…
              </div>
            }
          >
            <Show
              when={detail()}
              fallback={
                <div class="fw-panel p-4 text-sm text-ink-mute dark:text-paper-subtle">
                  Player not found.
                </div>
              }
            >
              {(d) => (
                <>
                  {/* Block 1: Phenotype */}
                  <section
                    aria-label="Player profile"
                    class="fw-panel space-y-2 p-4"
                  >
                    <h1 class="font-display text-3xl text-pitch-600 dark:text-pitch-300">
                      {d().phenotype.name}
                    </h1>
                    <dl class="grid grid-cols-2 gap-x-6 gap-y-1 text-sm sm:grid-cols-3">
                      <div>
                        <dt class="text-ink-mute dark:text-paper-subtle">
                          Role
                        </dt>
                        <dd class="font-medium text-ink dark:text-paper">
                          {d().phenotype.role}
                        </dd>
                      </div>
                      <div>
                        <dt class="text-ink-mute dark:text-paper-subtle">
                          Region
                        </dt>
                        <dd class="font-medium text-ink dark:text-paper">
                          {d().phenotype.birthRegion}
                        </dd>
                      </div>
                    </dl>
                    <Show when={d().phenotype.phenotypeLabels.length > 0}>
                      <div class="text-sm">
                        <span class="text-ink-mute dark:text-paper-subtle">
                          Scout traits:{" "}
                        </span>
                        <span class="text-ink dark:text-paper">
                          {d().phenotype.phenotypeLabels.join(", ")}
                        </span>
                      </div>
                    </Show>
                  </section>

                  {/* Block 2: Memory callbacks */}
                  <section
                    aria-label="Career moments"
                    class="fw-panel space-y-2 p-4"
                  >
                    <h2 class="text-lg font-semibold text-ink dark:text-paper">
                      Career moments
                    </h2>
                    <Show
                      when={d().memoryCallbacks.length > 0}
                      fallback={
                        <p class="text-sm text-ink-mute dark:text-paper-subtle">
                          No notable career moments yet.
                        </p>
                      }
                    >
                      <ul class="space-y-1" aria-label="Memory callbacks">
                        <For each={d().memoryCallbacks}>
                          {(cb) => (
                            <li class="text-sm text-ink dark:text-paper">{cb}</li>
                          )}
                        </For>
                      </ul>
                    </Show>
                  </section>

                  {/* Block 3: Contract (deferred to T4+ career-roster layer) */}
                  <section
                    aria-label="Contract details"
                    class="fw-panel space-y-2 p-4"
                  >
                    <h2 class="text-lg font-semibold text-ink dark:text-paper">
                      Contract
                    </h2>
                    <p class="text-sm text-ink-mute dark:text-paper-subtle">
                      {d().contractStatus ??
                        "Contract details arrive with the career-roster layer."}
                    </p>
                  </section>
                </>
              )}
            </Show>
          </Show>
        }
      >
        {(err) => (
          <div
            class="fw-panel p-4 text-sm font-mono text-flag-red"
            role="alert"
          >
            Failed to load player: {describeError(err())}
          </div>
        )}
      </Show>
    </div>
  );
}

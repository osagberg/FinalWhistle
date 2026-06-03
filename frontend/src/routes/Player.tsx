/*
 * Player detail page — T3-6, extended T4-F4.
 *
 * Text-first per DESIGN_DOC.md §3. Four blocks:
 *   1. Phenotype — name, role, region, profile labels (bio truth, renamed from
 *      "Scout traits" at T4-F4 to distinguish from Block 4's uncertain scout read).
 *   2. Memory callbacks — rendered career moments (honest empty state when ledger is empty).
 *   3. Contract — deferred placeholder (T4+ career-roster layer).
 *   4. Scouting report — scout's uncertain read (T4-F4); isolated from the rest
 *      of the page so a scout-resource failure never breaks the phenotype/career view.
 *
 * IPC (read-only via api wrappers):
 *   getPlayerDetail(id)   → PlayerDetail    — loaded on mount from route param
 *   getScoutReport(id)    → ScoutReportDto  — separate resource, isolated failure
 *
 * Rules compliance:
 *   - No `any` (Frontend/RULES.md §6)
 *   - Solid signals / createResource + createSignal, not React hooks (Frontend/RULES.md §1)
 *   - IpcError exhaustive switch + never discriminant (mirrors Squad.tsx pattern)
 *   - Dark-mode tokens on every color-bearing class (Frontend/RULES.md §2)
 *   - Explicit try/catch in resource fetchers preserves original thrown value so
 *     isIpcError() can match the plain object Tauri throws — avoids Solid's
 *     createResource wrapping the rejection in a generic Error.
 *   - UI never drives canonical state — read-only fetches only
 *   - Raw confidence / low / high numbers are NEVER shown to players; the band
 *     text is the primary surface (CLAUDE.md §7 invisible-floats rule).
 */

import { createResource, createSignal, For, Show, type JSX } from "solid-js";
import { useParams } from "@solidjs/router";
import ErrorBoundary from "~/components/ErrorBoundary";
import Loading from "~/components/Loading";
import { getPlayerDetail } from "~/lib/api/player";
import { getScoutReport } from "~/lib/api/scout";
import { IpcShapeError } from "~/lib/runtime-validators";
import { describeRouteError } from "~/lib/route-errors";
import type { IpcError, PlayerDetail, ScoutReportDto } from "~/lib/types";

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
  // T3-9: career-loop variant.
  "seasonNotComplete",
  // T4-5a: live-match command variant.
  "liveMatchCommandUnimplemented",
  // T4-6a: settings variant.
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
// Scout-report state discriminant
//
// Three outcomes the scout resource can produce — kept separate from the
// main player-error signal so scout failures don't break the phenotype block.
//   "ok"           — report loaded successfully.
//   "notObserved"  — player exists in roster but scouts haven't watched them yet.
//   "noSection"    — player is not in the career roster (content-bio only);
//                    omit the section silently.
//   "error"        — unexpected error; show a modest inline message.
// ---------------------------------------------------------------------------

type ScoutOutcome =
  | { kind: "ok"; report: ScoutReportDto }
  | { kind: "notObserved" }
  | { kind: "noSection" }
  | { kind: "error"; err: IpcError | Error };

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export default function Player(): JSX.Element {
  return (
    <ErrorBoundary label="Player">
      <PlayerInner />
    </ErrorBoundary>
  );
}

function PlayerInner(): JSX.Element {
  const params = useParams<{ id: string }>();

  // Fetch error — typed signal so it's fully reactive in the jsdom test env.
  // Mirrors Squad.tsx: explicit try/catch preserves the original thrown value
  // (IpcError plain object). Without this, Solid's createResource wraps the
  // rejection in a generic Error and the `kind` discriminant is lost, so
  // isIpcError() can't match and describeError() returns "Unknown error".
  const [playerError, setPlayerError] = createSignal<IpcError | Error | null>(
    null,
  );

  // Scout-report outcome — separate signal so this never blocks the player block.
  const [scoutOutcome, setScoutOutcome] = createSignal<ScoutOutcome | null>(
    null,
  );

  // Resource: drives getPlayerDetail() on mount (re-runs when params.id changes).
  // Returns null on failure (distinct from missing player, which the server
  // signals via IpcError::PlayerNotFound before the resource ever resolves).
  const [detail] = createResource<PlayerDetail | null, string>(
    // Squad links use encodeURIComponent (content-pack IDs contain ':'), so the
    // route param arrives percent-encoded (e.g. `fwh.core%3Aplayer_00001`).
    // Decode before it reaches the backend, which keys on the raw ':' form.
    () => decodeURIComponent(params.id),
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

  // Resource: drives getScoutReport() on mount (re-runs when params.id changes).
  // Produces a `ScoutOutcome` signal rather than throwing into the error boundary —
  // scout failures are lower-severity than the main player fetch and should not
  // crash the page.
  //
  // Note: getScoutReport takes the CONTENT-PACK-QUALIFIED id string (e.g.
  // "fwh.core:player_00042" — for a roster player, "fwh.core:player_<rosterId>"),
  // the same form get_player_detail takes. The backend parses the numeric suffix
  // and routes by range (suffix >= ROSTER_PLAYER_ID_BASE = roster). The decoded
  // route id here IS that content-pack id, so passing it directly is correct.
  // (A non-roster/content-bio id yields playerNotFound → the scout section omits.)
  const [_scoutResource] = createResource<null, string>(
    () => decodeURIComponent(params.id),
    async (id) => {
      setScoutOutcome(null);
      try {
        const report = await getScoutReport(id);
        setScoutOutcome({ kind: "ok", report });
      } catch (e: unknown) {
        if (isIpcError(e)) {
          if (e.kind === "notYetObserved") {
            setScoutOutcome({ kind: "notObserved" });
          } else if (e.kind === "playerNotFound") {
            // Player is a content-bio player not in the career roster —
            // scouting doesn't apply. Omit the section entirely.
            setScoutOutcome({ kind: "noSection" });
          } else {
            // Unexpected IpcError (e.g. lockPoisoned) — show modest inline note.
            setScoutOutcome({ kind: "error", err: e });
          }
        } else if (e instanceof IpcShapeError) {
          // eslint-disable-next-line no-console
          console.error(
            "[Player] get_scout_report DTO contract drift:",
            e.command,
            e.reason,
            e.payloadPreview,
          );
          setScoutOutcome({ kind: "error", err: e });
        } else {
          setScoutOutcome({ kind: "error", err: normaliseError(e) });
        }
      }
      return null;
    },
  );

  return (
    <div class="space-y-6">
      {/* Loading state first — no white-flash before spinner. */}
      <Show when={detail.loading}>
        <Loading message="Loading player…" />
      </Show>

      {/* Error / data paths (only when not loading). */}
      <Show when={!detail.loading}>
        <Show
          when={playerError()}
          fallback={
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
                  {/* Block 1: Phenotype
                      Relabelled "Profile" (was "Scout traits") at T4-F4 to
                      distinguish the bio truth from Block 4's uncertain scout read.
                  */}
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
                          Profile:{" "}
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

                  {/* Block 4: Scouting report (T4-F4)
                      Isolated resource — this block never breaks the blocks above.
                      Three sub-states:
                        ok          → render overall band + category bands + labels.
                        notObserved → graceful muted "no read yet" note.
                        noSection   → omit entirely (content-bio player; scouting N/A).
                        error       → modest inline note, not a full alert banner.
                        null        → still loading; show a subtle placeholder.
                  */}
                  <ScoutSection scoutOutcome={scoutOutcome()} />
                </>
              )}
            </Show>
          }
        >
          {(err) => {
            const copy = describeRouteError(err(), { what: "the player" });
            return (
              <div
                class="fw-panel p-4 text-sm text-rose-600 dark:text-rose-400"
                role="alert"
              >
                <p class="font-semibold">{copy.headline}</p>
                <p class="mt-1 text-ink-subtle dark:text-paper-subtle">{copy.detail}</p>
              </div>
            );
          }}
        </Show>
      </Show>
    </div>
  );
}

// ---------------------------------------------------------------------------
// ScoutSection — isolated block for the scouting report
//
// Receives a `ScoutOutcome | null` — null means the resource is still in
// flight. The section renders a loading placeholder for that case.
// Raw confidence / low / high numbers are never shown — band text only.
// ---------------------------------------------------------------------------

interface ScoutSectionProps {
  scoutOutcome: ScoutOutcome | null;
}

function ScoutSection(props: ScoutSectionProps): JSX.Element {
  return (
    <>
      {/* notObserved: graceful muted note — NOT a red error banner */}
      <Show when={props.scoutOutcome?.kind === "notObserved"}>
        <section
          aria-label="Scouting report"
          class="fw-panel space-y-2 p-4"
        >
          <h2 class="text-lg font-semibold text-ink dark:text-paper">
            Scouting report
          </h2>
          <p class="text-sm text-ink-mute dark:text-paper-subtle">
            No scouting read yet — your scouts need to watch this player
            before they can form a view.
          </p>
        </section>
      </Show>

      {/* noSection: content-bio only player — omit entirely */}
      {/* (Show with false condition renders nothing) */}

      {/* error: modest inline note, not a full page-level alert */}
      <Show when={props.scoutOutcome?.kind === "error"}>
        <section
          aria-label="Scouting report"
          class="fw-panel space-y-2 p-4"
        >
          <h2 class="text-lg font-semibold text-ink dark:text-paper">
            Scouting report
          </h2>
          <p class="text-sm text-ink-mute dark:text-paper-subtle">
            The scouting report couldn't be loaded right now.
          </p>
        </section>
      </Show>

      {/* ok: full report */}
      <Show
        when={
          props.scoutOutcome?.kind === "ok"
            ? (props.scoutOutcome as { kind: "ok"; report: ScoutReportDto })
                .report
            : null
        }
      >
        {(report) => (
          <section
            aria-label="Scouting report"
            class="fw-panel space-y-3 p-4"
          >
            <div class="flex items-baseline justify-between">
              <h2 class="text-lg font-semibold text-ink dark:text-paper">
                Scouting report
              </h2>
              <span class="text-xs text-ink-mute dark:text-paper-subtle">
                watched {report().observationCount}{" "}
                {report().observationCount === 1 ? "time" : "times"}
              </span>
            </div>

            {/* Overall read */}
            <p class="text-sm text-ink dark:text-paper">
              Overall:{" "}
              <span class="font-medium">{report().overallBand}</span>
            </p>

            {/* Category bands (Physical / Mental / Technical) */}
            <Show when={report().categories.length > 0}>
              <div
                class="grid grid-cols-3 gap-2"
                aria-label="Category estimates"
              >
                <For each={report().categories}>
                  {(cat) => (
                    <div class="rounded border border-ink-subtle/20 p-2 text-center dark:border-paper-subtle/20">
                      <p class="text-xs text-ink-mute dark:text-paper-subtle">
                        {cat.category}
                      </p>
                      <p class="mt-0.5 text-sm font-medium text-ink dark:text-paper">
                        {cat.band}
                      </p>
                    </div>
                  )}
                </For>
              </div>
            </Show>

            {/* Per-label estimates */}
            <Show when={report().labels.length > 0}>
              <ul
                class="space-y-0.5"
                aria-label="Label estimates"
              >
                <For each={report().labels}>
                  {(lbl) => (
                    <li class="flex items-center justify-between text-sm">
                      <span class="text-ink dark:text-paper">{lbl.label}</span>
                      <span class="ml-4 text-xs text-ink-mute dark:text-paper-subtle">
                        {lbl.band}
                      </span>
                    </li>
                  )}
                </For>
              </ul>
            </Show>
          </section>
        )}
      </Show>
    </>
  );
}

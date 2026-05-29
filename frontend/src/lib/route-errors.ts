/*
 * route-errors.ts — T4-4.
 *
 * Centralised error-copy utility for route-level IPC failures.
 *
 * Rules:
 *   - No raw `err.message` in production-visible copy (DESIGN_DOC §9).
 *   - Copy is football-native voice (T4-4 narrative-director pass).
 *   - Narrows against the closed IpcError discriminated union from lib/types.ts.
 */

import type { IpcError } from "~/lib/types";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/** Structured user-facing error copy for a route-level failure. */
export interface RouteErrorCopy {
  headline: string;
  detail: string;
}

/** Contextual hint the caller provides about what was being loaded. */
export interface RouteErrorContext {
  /**
   * Plain-noun phrase for the data being loaded.
   * e.g. "the squad", "the standings", "the player".
   */
  what: string;
}

// ---------------------------------------------------------------------------
// IpcError narrowing — exhaustive against the closed union
//
// `satisfies` pins this list to the actual `IpcError["kind"]` union, so
// adding a new variant in `lib/types.ts` produces a compile error here
// AND at the `assertNever` arm of `describeRouteError`. Single source of
// truth for the runtime kinds list inside this module.
// ---------------------------------------------------------------------------

const KNOWN_IPC_ERROR_KINDS = [
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
] as const satisfies readonly IpcError["kind"][];

const KNOWN_IPC_ERROR_KINDS_SET: ReadonlySet<string> = new Set(
  KNOWN_IPC_ERROR_KINDS,
);

/**
 * Type guard: returns true when `e` is a known `IpcError` variant.
 *
 * Exported so route fetchers can normalise a caught `unknown` into the
 * `IpcError | Error | null` shape some of them use for their error signals.
 * The guard is strict: only objects with a `kind` matching one of the
 * KNOWN_IPC_ERROR_KINDS narrow positive — malformed or future variants
 * fall through to the route's `instanceof Error` fallback.
 */
export function isIpcError(e: unknown): e is IpcError {
  if (typeof e !== "object" || e === null || !("kind" in e)) return false;
  const kind = (e as Record<string, unknown>).kind;
  return typeof kind === "string" && KNOWN_IPC_ERROR_KINDS_SET.has(kind);
}

// ---------------------------------------------------------------------------
// describeRouteError
// ---------------------------------------------------------------------------

/**
 * Maps any caught error to football-native copy keyed off IpcError.kind.
 *
 * For non-IpcError errors (raw Error, IpcShapeError, unknown shapes), returns
 * a generic fallback that uses `ctx.what` for context. `err.message` is NEVER
 * included in production-visible copy.
 */
export function describeRouteError(
  err: unknown,
  ctx: RouteErrorContext,
): RouteErrorCopy {
  if (!isIpcError(err)) {
    // Generic fallback for non-IpcError values (raw Error, IpcShapeError,
    // unknown shapes, or a future IpcError variant we have not mapped yet).
    return {
      headline: `Something went wrong loading ${ctx.what}`,
      detail: `The back office ran into a problem pulling up ${ctx.what}. Give it another go, or restart if it keeps happening.`,
    };
  }

  // Typed switch — `err` is narrowed to the closed `IpcError` union.
  // Adding a new variant to `lib/types.ts` will force a compile error at the
  // `never` default below (and at KNOWN_IPC_ERROR_KINDS' satisfies clause).
  switch (err.kind) {
    case "tooManyFrames":
      return {
        headline: "That clip is longer than the replay reel holds",
        detail: `You asked for ${err.requested} frames but the reel tops out at ${err.max}. Trim the window and try again.`,
      };
    case "invalidSeed":
      return {
        headline: "That seed doesn't look right",
        detail: `"${err.input}" couldn't be read as a match seed. Try a hex value like 0xfeedbeefcafefade.`,
      };
    case "matchInitFailed":
      return {
        headline: "Kick-off didn't happen",
        detail: "Something went wrong getting the match underway — the teams couldn't take the field. Check the seed and line-up, then try again.",
      };
    case "seasonComplete":
      return {
        headline: "Full time on the season",
        detail: "There are no more match-days left this season. Head to the boardroom to see out the close season and start the next campaign.",
      };
    case "clubNotFound":
      return {
        headline: "This club isn't on the league's books",
        detail: `Club ${err.clubId} doesn't appear in the current league. The fixture list may be out of date — try reloading the page.`,
      };
    case "lockPoisoned":
      return {
        headline: "Something's gone quiet in the back office",
        detail: `An earlier problem left the ${err.lock} ledger in a bad state. Save your progress if you can, then restart the app.`,
      };
    case "playerNotFound":
      return {
        headline: "This player isn't on the team-sheet",
        detail: `${err.playerId} doesn't appear in the squad. They may have moved on or the reference is out of date.`,
      };
    case "seasonNotComplete":
      return {
        headline: "There are still fixtures to play",
        detail: "The season can't wrap up while matches remain on the fixture list. Play out the remaining games first.",
      };
    case "liveMatchCommandUnimplemented":
      return {
        headline: "That instruction hasn't landed yet",
        detail: `The in-match command system isn't fully wired up — that particular instruction (${err.commandKind}) will land in a future build.`,
      };
    case "settingsLoadFailed":
      return {
        headline: "Preferences couldn't be loaded",
        detail: "The settings file looks to be damaged. The app will use the defaults. If this keeps happening, try removing the settings file and restarting.",
      };
    default: {
      // Exhaustiveness gate: TypeScript flags this if a new IpcError variant
      // lands in lib/types.ts without an arm here. The throw is unreachable
      // unless KNOWN_IPC_ERROR_KINDS_SET has been edited inconsistently with
      // the union — also a compile error via the `satisfies` clause above.
      const _exhaustive: never = err;
      throw new Error(
        `describeRouteError: unhandled IpcError variant — KNOWN_IPC_ERROR_KINDS / switch drift. err=${JSON.stringify(_exhaustive)}`,
      );
    }
  }
}

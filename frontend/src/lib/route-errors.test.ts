/*
 * route-errors.test.ts — T4-4.
 *
 * Verifies that `describeRouteError` returns variant-specific copy for each
 * IpcError kind, not the generic fallback. This is the non-vacuous
 * mutation-thinking guard: collapsing two branches makes the test fail.
 */

import { describe, expect, it } from "vitest";
import { describeRouteError } from "~/lib/route-errors";

const CTX = { what: "the squad" };

// Generic-fallback shape — anything that isn't a recognisable IpcError.
const GENERIC_SHAPE = { foo: "bar" };

// Helper: returns the headline for a given error shape.
function headline(err: unknown): string {
  return describeRouteError(err, CTX).headline;
}

// Helper: returns the detail for a given error shape.
function detail(err: unknown): string {
  return describeRouteError(err, CTX).detail;
}

describe("describeRouteError", () => {
  // ---------------------------------------------------------------------------
  // Generic fallback
  // ---------------------------------------------------------------------------

  it("returns the generic fallback for an unrecognised shape, threading ctx.what", () => {
    const result = describeRouteError(GENERIC_SHAPE, CTX);
    // Tightened mutation guard: a "truthy" assertion passed for any non-empty
    // literal including a wrong-variant return. The fallback's contract is
    // that it folds `ctx.what` into both headline and detail — assert that
    // explicitly so a future change that drops ctx.what surfaces here.
    expect(result.headline).toContain(CTX.what);
    expect(result.detail).toContain(CTX.what);
  });

  it("does not include err.message in the generic fallback detail", () => {
    // A plain Error — message must NOT bleed into production copy.
    const err = new Error("Cannot read properties of undefined (reading 'invoke')");
    const result = describeRouteError(err, CTX);
    expect(result.detail).not.toContain("Cannot read properties of undefined");
    expect(result.detail).not.toContain("invoke");
  });

  // ---------------------------------------------------------------------------
  // Variant-specific checks: each kind produces a headline distinct from the
  // generic fallback (mutation guard — collapsing two branches → failing test).
  // ---------------------------------------------------------------------------

  it("tooManyFrames: variant-specific headline, NOT the generic fallback", () => {
    const err = { kind: "tooManyFrames", requested: 9000, max: 7200 };
    expect(headline(err)).not.toBe(headline(GENERIC_SHAPE));
  });

  it("tooManyFrames: detail contains the requested and max values", () => {
    const err = { kind: "tooManyFrames", requested: 9000, max: 7200 };
    const d = detail(err);
    expect(d).toContain("9000");
    expect(d).toContain("7200");
  });

  it("invalidSeed: variant-specific headline, NOT the generic fallback", () => {
    const err = { kind: "invalidSeed", input: "0xGGGG", reason: "not hex" };
    expect(headline(err)).not.toBe(headline(GENERIC_SHAPE));
  });

  it("invalidSeed: detail contains the invalid input string", () => {
    const err = { kind: "invalidSeed", input: "0xGGGG", reason: "not hex" };
    expect(detail(err)).toContain("0xGGGG");
  });

  it("matchInitFailed: variant-specific headline, NOT the generic fallback", () => {
    const err = { kind: "matchInitFailed", reason: "lineup incomplete" };
    expect(headline(err)).not.toBe(headline(GENERIC_SHAPE));
  });

  it("seasonComplete: variant-specific headline, NOT the generic fallback", () => {
    const err = { kind: "seasonComplete" };
    expect(headline(err)).not.toBe(headline(GENERIC_SHAPE));
  });

  it("clubNotFound: variant-specific headline, NOT the generic fallback", () => {
    const err = { kind: "clubNotFound", clubId: 99999 };
    expect(headline(err)).not.toBe(headline(GENERIC_SHAPE));
  });

  it("clubNotFound: detail contains the club id", () => {
    const err = { kind: "clubNotFound", clubId: 99999 };
    expect(detail(err)).toContain("99999");
  });

  it("lockPoisoned: variant-specific headline, NOT the generic fallback", () => {
    const err = { kind: "lockPoisoned", lock: "season" };
    expect(headline(err)).not.toBe(headline(GENERIC_SHAPE));
  });

  it("lockPoisoned: detail contains the lock name", () => {
    const err = { kind: "lockPoisoned", lock: "season" };
    expect(detail(err)).toContain("season");
  });

  it("playerNotFound: variant-specific headline, NOT the generic fallback", () => {
    const err = { kind: "playerNotFound", playerId: "fwh.core:player_99999" };
    expect(headline(err)).not.toBe(headline(GENERIC_SHAPE));
  });

  it("playerNotFound: detail contains the player id", () => {
    const err = { kind: "playerNotFound", playerId: "fwh.core:player_99999" };
    expect(detail(err)).toContain("fwh.core:player_99999");
  });

  it("seasonNotComplete: variant-specific headline, NOT the generic fallback", () => {
    const err = { kind: "seasonNotComplete" };
    expect(headline(err)).not.toBe(headline(GENERIC_SHAPE));
  });

  it("liveMatchCommandUnimplemented: variant-specific headline, NOT the generic fallback", () => {
    const err = {
      kind: "liveMatchCommandUnimplemented",
      commandKind: "substitute",
    };
    expect(headline(err)).not.toBe(headline(GENERIC_SHAPE));
  });

  it("liveMatchCommandUnimplemented: detail contains the commandKind verbatim", () => {
    const err = {
      kind: "liveMatchCommandUnimplemented",
      commandKind: "changeFormation",
    };
    expect(detail(err)).toContain("changeFormation");
  });

  it("notYetObserved: variant-specific headline, NOT the generic fallback", () => {
    const err = { kind: "notYetObserved", playerId: "fwh.core:player_00042" };
    expect(headline(err)).not.toBe(headline(GENERIC_SHAPE));
  });

  it("notYetObserved: headline and detail are football-native copy", () => {
    const err = { kind: "notYetObserved", playerId: "fwh.core:player_00042" };
    // Must not be the generic fallback — must be specific to scouting.
    expect(headline(err)).not.toContain("went wrong");
    expect(detail(err)).toMatch(/scouts/i);
  });

  it("leagueGenerationFailed: variant-specific headline, NOT the generic fallback", () => {
    const err = { kind: "leagueGenerationFailed", reason: "missing culture" };
    expect(headline(err)).not.toBe(headline(GENERIC_SHAPE));
  });

  it("leagueGenerationFailed: detail does NOT expose the raw reason to the player", () => {
    const err = { kind: "leagueGenerationFailed", reason: "missing culture data in pack" };
    // The raw reason string must not bleed into production copy.
    expect(detail(err)).not.toContain("missing culture data in pack");
  });

  it("leagueGenerationFailed: detail is football-native copy, not a generic stub", () => {
    const err = { kind: "leagueGenerationFailed", reason: "missing culture" };
    // Must reference the game world (season / campaign / content pack), so a
    // collapsed branch returning generic text would fail here.
    expect(detail(err)).toMatch(/season|campaign|content pack/i);
  });

  // ---------------------------------------------------------------------------
  // Uniqueness: no two variants produce identical headlines (coarse catch-all
  // for collapsed-branch regressions not caught by individual assertions).
  // ---------------------------------------------------------------------------

  it("all 12 known variants produce distinct headlines from each other", () => {
    const variants = [
      { kind: "tooManyFrames", requested: 9000, max: 7200 },
      { kind: "invalidSeed", input: "0xGGGG", reason: "not hex" },
      { kind: "matchInitFailed", reason: "lineup incomplete" },
      { kind: "seasonComplete" },
      { kind: "clubNotFound", clubId: 99999 },
      { kind: "lockPoisoned", lock: "season" },
      { kind: "playerNotFound", playerId: "fwh.core:player_99999" },
      { kind: "seasonNotComplete" },
      { kind: "liveMatchCommandUnimplemented", commandKind: "substitute" },
      // T4-6a:
      { kind: "settingsLoadFailed", reason: "bad bincode" },
      // T4-F4:
      { kind: "notYetObserved", playerId: "fwh.core:player_00042" },
      { kind: "leagueGenerationFailed", reason: "missing culture" },
    ];
    const headlines = variants.map((v) => headline(v));
    const unique = new Set(headlines);
    expect(unique.size).toBe(variants.length);
  });
});

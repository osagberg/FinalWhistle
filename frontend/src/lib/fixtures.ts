/**
 * Fixture helpers shared across routes (Home hub + LiveMatch route).
 *
 * `getFixtures()` returns fixtures RELATIVE to the managed club. To start a
 * live match we need the ABSOLUTE (homeClubId, awayClubId) pair so
 * `startLiveMatchForFixture` can construct the correct MatchState. The
 * derivation rule lives here once so the Home hub's "Watch this match" button
 * and the sidebar-driven LiveMatch auto-start agree on exactly the same logic.
 *
 * Read-only: these helpers consume DTO projections only and never mutate
 * canonical state (CLAUDE.md §7).
 */

import type { FixtureWithResult } from "./types";

/** Absolute home/away club ids for a fixture, ready for startLiveMatchForFixture. */
export interface FixtureClubIds {
  homeClubId: number;
  awayClubId: number;
}

/** First unplayed fixture in a managed-club fixture list, or null when none remain. */
export function findNextUnplayedFixture(
  fixtures: readonly FixtureWithResult[] | null | undefined,
): FixtureWithResult | null {
  if (!fixtures) return null;
  return fixtures.find((f) => !f.played) ?? null;
}

/**
 * Derive the absolute (homeClubId, awayClubId) pair for a managed-club-relative
 * fixture.
 *
 * `getFixtures()` returns fixtures relative to the managed club:
 *   isHome === true  → home = managedClubId, away = opponentClubId
 *   isHome === false → home = opponentClubId, away = managedClubId
 */
export function deriveFixtureClubIds(
  fixture: FixtureWithResult,
  managedClubId: number,
): FixtureClubIds {
  if (fixture.isHome) {
    return { homeClubId: managedClubId, awayClubId: fixture.opponentClubId };
  }
  return { homeClubId: fixture.opponentClubId, awayClubId: managedClubId };
}

/**
 * Resolve the next-fixture club-id pair for a managed-club fixture list.
 *
 * Combines {@link findNextUnplayedFixture} and {@link deriveFixtureClubIds}.
 * Returns null when there is no managed club, the id cannot be parsed, or no
 * unplayed fixture remains. This is the single derivation the Home hub's
 * "Watch this match" button and the LiveMatch sidebar auto-start both reuse.
 */
export function deriveNextFixtureClubIds(
  fixtures: readonly FixtureWithResult[] | null | undefined,
  managedClubId: string | null,
): FixtureClubIds | null {
  if (managedClubId === null) return null;
  const numericId = parseInt(managedClubId, 10);
  if (isNaN(numericId)) return null;
  const next = findNextUnplayedFixture(fixtures);
  if (!next) return null;
  return deriveFixtureClubIds(next, numericId);
}

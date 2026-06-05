/**
 * match-events.ts — shared match-event utilities (S3b).
 *
 * Extracted from routes/Match.tsx so the key-moments type filter is defined
 * once and imported by both the batch-replay Match route and the live
 * LiveMatch route. The suppression predicate is TYPE-based, not salience-based
 * (salience is degenerate today; MatchEvent has no salience field).
 */

import type { MatchEventKind } from "~/lib/types";

/**
 * Returns true for event kinds that are suppressed by the key-moments filter.
 *
 * Suppressed: Pass, PassIncomplete — high-frequency ball-movement events.
 * Always shown: Goal, Shot, KickOff, HalfTime, FullTime, Card, Substitution,
 * SignatureFirstFired, Offside.
 *
 * When `keyMomentsOnly` is false this function is never called.
 */
export function isHighFrequencyKind(kind: MatchEventKind): boolean {
  return kind === "Pass" || kind === "PassIncomplete";
}

/**
 * Returns true for event kinds that are considered key moments.
 *
 * Inverse of `isHighFrequencyKind`. Used by the live-match step loop to decide
 * whether to auto-pause after a step (stop on goal/shot/card/etc., skip past
 * passes).
 */
export function isKeyMomentKind(kind: MatchEventKind): boolean {
  return !isHighFrequencyKind(kind);
}

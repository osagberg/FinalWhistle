/*
 * Transfer-window state derivation — T2-8.
 *
 * Pure function: same input (current match-day) → same output.
 * No IPC, no async, no side effects.
 *
 * The match-day → window-state mapping encodes the FM-style two-window
 * calendar (summer + mid-season winter) per football convention. Hardcoded
 * boundaries are deliberate at T2-8 — transfer mechanics (and any
 * configurable window dates) land at Phase T3.
 *
 * Boundary contract:
 *   match-day 0           → summer  (pre-season; no fixtures played yet)
 *   match-day 1..=18      → closed  (first half of season)
 *   match-day 19..=20     → winter  (mid-season window)
 *   match-day 21..=38     → closed  (second half of season)
 *   match-day > 38        → closed  (season complete / defensive)
 *
 * Each branch is unit-tested at its boundary values in
 * `transfer-window.test.ts`.
 */

/** A discriminated window-state with its display label. */
export type WindowState =
  | { kind: "summer"; label: "Summer window — open" }
  | { kind: "winter"; label: "Winter window — open" }
  | { kind: "closed"; label: "Closed" };

/**
 * Compute the transfer-window state for a given match-day.
 *
 * Pure function — no IPC, no clocks, no randomness. Same `currentMatchDay`
 * value always produces the same `WindowState` (frontend determinism contract
 * mirroring the project's sim-side discipline).
 *
 * Post-T2-8 silent-failure-hunter P1 fix: invalid inputs (negative,
 * non-integer, NaN, Infinity) THROW rather than silently mapping to "Closed".
 * The prior shape laundered wire-format bugs (a future `played` being `NaN`
 * from a serde-rename mishap) into a confidently-wrong "Closed" pill mid-
 * summer with no diagnostic. Throwing here surfaces via `createResource`'s
 * `.error` accessor + the consumer's error-arm renders a clear "unavailable"
 * message. Matches the project's sim-side discipline (Sim/RULES.md §11:
 * canonical invariants fail loudly, not silently saturate).
 */
export function computeTransferWindowState(currentMatchDay: number): WindowState {
  if (!Number.isInteger(currentMatchDay) || currentMatchDay < 0) {
    throw new RangeError(
      `computeTransferWindowState: invalid match-day ${currentMatchDay} (expected non-negative integer)`,
    );
  }
  if (currentMatchDay === 0) {
    return { kind: "summer", label: "Summer window — open" };
  }
  if (currentMatchDay >= 19 && currentMatchDay <= 20) {
    return { kind: "winter", label: "Winter window — open" };
  }
  return { kind: "closed", label: "Closed" };
}

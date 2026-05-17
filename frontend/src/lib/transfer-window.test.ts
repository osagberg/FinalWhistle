/*
 * Transfer-window pure-function tests — T2-8.
 *
 * Covers all 5 branches of `computeTransferWindowState` + boundary days.
 * Each test is non-vacuous: mutating the function's branch logic (e.g.
 * shifting the winter window from 19-20 to 20-21) would fail at least one
 * boundary assertion below.
 */

import { describe, expect, it } from "vitest";
import { computeTransferWindowState } from "./transfer-window";

describe("computeTransferWindowState", () => {
  // AC3a — pre-season → summer
  it("returns summer for match-day 0 (pre-season)", () => {
    const state = computeTransferWindowState(0);
    expect(state.kind).toBe("summer");
    expect(state.label).toBe("Summer window — open");
  });

  // AC3b — early-season → closed (boundary days 1 and 18)
  it("returns closed for match-day 1 (early-season boundary)", () => {
    const state = computeTransferWindowState(1);
    expect(state.kind).toBe("closed");
    expect(state.label).toBe("Closed");
  });

  it("returns closed for match-day 18 (last day before winter window)", () => {
    expect(computeTransferWindowState(18).kind).toBe("closed");
  });

  // AC3c — mid-season → winter (both boundary days)
  it("returns winter for match-day 19 (first day of winter window)", () => {
    const state = computeTransferWindowState(19);
    expect(state.kind).toBe("winter");
    expect(state.label).toBe("Winter window — open");
  });

  it("returns winter for match-day 20 (last day of winter window)", () => {
    expect(computeTransferWindowState(20).kind).toBe("winter");
  });

  // AC3d — late-season → closed (boundary days 21 and 38)
  it("returns closed for match-day 21 (first day after winter window)", () => {
    expect(computeTransferWindowState(21).kind).toBe("closed");
  });

  it("returns closed for match-day 38 (last match-day of season)", () => {
    expect(computeTransferWindowState(38).kind).toBe("closed");
  });

  // AC3e — out-of-bounds → closed (defensive)
  it("returns closed for match-day 39 (season complete, out of bounds)", () => {
    expect(computeTransferWindowState(39).kind).toBe("closed");
  });

  // Post-T2-8 silent-failure-hunter P1 fix: invalid inputs THROW.
  // Prior behavior silently returned `closed` which laundered wire-format
  // bugs (NaN/Infinity from a future serde mishap) into a confidently-wrong
  // "Closed" mid-summer pill. Throwing surfaces via createResource.error.
  it("throws RangeError for negative match-day", () => {
    expect(() => computeTransferWindowState(-1)).toThrow(RangeError);
  });

  it("throws RangeError for non-integer match-day", () => {
    expect(() => computeTransferWindowState(19.5)).toThrow(RangeError);
  });

  it("throws RangeError for NaN match-day (wire-format bug guard)", () => {
    expect(() => computeTransferWindowState(Number.NaN)).toThrow(RangeError);
  });

  it("throws RangeError for Infinity match-day (wire-format bug guard)", () => {
    expect(() => computeTransferWindowState(Number.POSITIVE_INFINITY)).toThrow(
      RangeError,
    );
  });
});

/**
 * live_match.test.ts — T4-5a.
 *
 * Round-trip tests for the five `api/live_match.ts` wrappers.
 *
 * Pattern mirrors FrameSource.test.ts / runtime-validators.test.ts:
 *   - Mock `@tauri-apps/api/core` `invoke` BEFORE importing the SUT.
 *   - Assert the correct command-name string is passed (single authoritative
 *     location: if the Rust-side rename changes, this test goes RED).
 *   - Assert the payload shape matches the DTO the wrapper promises to return
 *     (non-vacuous: a wrong field name in the args record is caught here, not
 *     silently swallowed by the backend).
 *   - For `applyMatchCommand`: assert the error path propagates correctly
 *     (always throws `IpcError::LiveMatchCommandUnimplemented`).
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Mock } from "vitest";

// Mock BEFORE importing the SUT — vi.mock is hoisted by vitest.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import {
  applyMatchCommand,
  finishLiveMatch,
  getMatchSnapshot,
  startLiveMatch,
  stepLiveMatch,
} from "./live_match";
import type {
  FinalMatchResult,
  MatchCommand,
  MatchHandle,
  MatchSnapshot,
  StepResult,
} from "~/lib/types";

const mockInvoke = invoke as unknown as Mock;

// ---------------------------------------------------------------------------
// Fixture builders
// ---------------------------------------------------------------------------

function makeHandle(): MatchHandle {
  return { id: 0, seedHex: "0xdeadbeefdeadbeef" };
}

/** Minimal valid StepResult (all required fields, delta = 0 events). */
function makeStepResult(handle: MatchHandle): StepResult {
  return {
    handle,
    newEvents: [],
    score: { home: 0, away: 0 },
    tick: 0,
    isFinished: false,
    frame: {
      seedHex: handle.seedHex,
      tick: 0,
      homeScore: 0,
      awayScore: 0,
      players: [],
      ball: { posX: 0, posY: 0, posZ: 0, velX: 0, velY: 0, velZ: 0 },
      possession: null,
    },
  };
}

/** Minimal valid MatchSnapshot at tick 0 (firstHalf, 0-0). */
function makeMatchSnapshot(handle: MatchHandle): MatchSnapshot {
  // 11 player-id slots (u32 values)
  const lineup = { players: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11] };
  return {
    handle,
    tick: 0,
    minute: 0,
    phase: "firstHalf",
    score: { home: 0, away: 0 },
    possessionPct: { homePct: 50, awayPct: 50 },
    ballZone: "center",
    homeLineup: lineup,
    awayLineup: { players: [12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22] },
    recentEvents: [],
    yellowCards: {},
    sentOff: [],
  };
}

/** Minimal valid FinalMatchResult. */
function makeFinalResult(handle: MatchHandle): FinalMatchResult {
  return {
    handle,
    finalScore: { home: 1, away: 0 },
    tick: 60,
    totalEvents: 5,
  };
}

// ---------------------------------------------------------------------------
// startLiveMatch
// ---------------------------------------------------------------------------

describe("startLiveMatch", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("invokes start_live_match with seedHex", async () => {
    const handle = makeHandle();
    mockInvoke.mockResolvedValue(handle);

    const result = await startLiveMatch("0xdeadbeefdeadbeef");

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith("start_live_match", {
      seedHex: "0xdeadbeefdeadbeef",
    });
    expect(result).toStrictEqual(handle);
  });

  it("returns a MatchHandle with the id from the backend", async () => {
    mockInvoke.mockResolvedValue({ id: 7, seedHex: "0xaabbccdd" });

    const result = await startLiveMatch("0xaabbccdd");

    expect(result.id).toBe(7);
    expect(result.seedHex).toBe("0xaabbccdd");
  });

  it("throws IpcShapeError when backend returns a malformed handle (missing seedHex)", async () => {
    // Backend returns { id: 0 } — missing required seedHex
    mockInvoke.mockResolvedValue({ id: 0 });

    await expect(startLiveMatch("0x1")).rejects.toThrow(/shape/i);
  });
});

// ---------------------------------------------------------------------------
// stepLiveMatch
// ---------------------------------------------------------------------------

describe("stepLiveMatch", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("invokes step_live_match with handle + ticks", async () => {
    const handle = makeHandle();
    mockInvoke.mockResolvedValue(makeStepResult(handle));

    await stepLiveMatch(handle, 60);

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith("step_live_match", {
      handle,
      ticks: 60,
    });
  });

  it("returns StepResult with isFinished=false before match ends", async () => {
    const handle = makeHandle();
    const payload = makeStepResult(handle);
    mockInvoke.mockResolvedValue(payload);

    const result = await stepLiveMatch(handle, 1);

    expect(result.isFinished).toBe(false);
    expect(result.tick).toBe(0);
    expect(result.newEvents).toHaveLength(0);
  });

  it("returns StepResult with isFinished=true at full time", async () => {
    const handle = makeHandle();
    mockInvoke.mockResolvedValue({
      ...makeStepResult(handle),
      isFinished: true,
      tick: 3600,
    });

    const result = await stepLiveMatch(handle, 1);

    expect(result.isFinished).toBe(true);
    expect(result.tick).toBe(3600);
  });

  it("throws IpcShapeError when backend omits isFinished field", async () => {
    const handle = makeHandle();
    const bad = { ...makeStepResult(handle) };
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    delete (bad as any).isFinished;
    mockInvoke.mockResolvedValue(bad);

    await expect(stepLiveMatch(handle, 1)).rejects.toThrow(/shape/i);
  });

  it("passes the backend IpcError through unchanged on rejection", async () => {
    const handle = makeHandle();
    const ipcErr = { kind: "tooManyFrames", requested: 9000, max: 7200 };
    mockInvoke.mockRejectedValue(ipcErr);

    await expect(stepLiveMatch(handle, 9000)).rejects.toMatchObject(ipcErr);
  });
});

// ---------------------------------------------------------------------------
// getMatchSnapshot
// ---------------------------------------------------------------------------

describe("getMatchSnapshot", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("invokes get_match_snapshot with the handle", async () => {
    const handle = makeHandle();
    mockInvoke.mockResolvedValue(makeMatchSnapshot(handle));

    await getMatchSnapshot(handle);

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith("get_match_snapshot", { handle });
  });

  it("returns the MatchSnapshot payload from the backend", async () => {
    const handle = makeHandle();
    const snapshot = makeMatchSnapshot(handle);
    mockInvoke.mockResolvedValue(snapshot);

    const result = await getMatchSnapshot(handle);

    expect(result.phase).toBe("firstHalf");
    expect(result.possessionPct).toStrictEqual({ homePct: 50, awayPct: 50 });
    expect(result.ballZone).toBe("center");
    expect(result.homeLineup.players).toHaveLength(11);
  });

  it("throws IpcShapeError when homeLineup has ≠11 players", async () => {
    const handle = makeHandle();
    const bad: MatchSnapshot = {
      ...makeMatchSnapshot(handle),
      homeLineup: { players: [1, 2, 3] }, // only 3 — should fail isLineupDto
    };
    mockInvoke.mockResolvedValue(bad);

    await expect(getMatchSnapshot(handle)).rejects.toThrow(/shape/i);
  });

  it("throws IpcShapeError when phase is an unknown string", async () => {
    const handle = makeHandle();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const bad = { ...makeMatchSnapshot(handle), phase: "overtime" as any };
    mockInvoke.mockResolvedValue(bad);

    await expect(getMatchSnapshot(handle)).rejects.toThrow(/shape/i);
  });
});

// ---------------------------------------------------------------------------
// finishLiveMatch
// ---------------------------------------------------------------------------

describe("finishLiveMatch", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("invokes finish_live_match with the handle", async () => {
    const handle = makeHandle();
    mockInvoke.mockResolvedValue(makeFinalResult(handle));

    await finishLiveMatch(handle);

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith("finish_live_match", { handle });
  });

  it("returns FinalMatchResult with correct totalEvents", async () => {
    const handle = makeHandle();
    mockInvoke.mockResolvedValue({ ...makeFinalResult(handle), totalEvents: 42 });

    const result = await finishLiveMatch(handle);

    expect(result.totalEvents).toBe(42);
    expect(result.finalScore).toStrictEqual({ home: 1, away: 0 });
  });

  it("throws IpcShapeError when totalEvents field is missing", async () => {
    const handle = makeHandle();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const bad = { ...makeFinalResult(handle) } as any;
    delete bad.totalEvents;
    mockInvoke.mockResolvedValue(bad);

    await expect(finishLiveMatch(handle)).rejects.toThrow(/shape/i);
  });

  it("propagates MatchInitFailed IpcError when handle is stale", async () => {
    const handle = makeHandle();
    const ipcErr = { kind: "matchInitFailed", reason: "no session for id 0" };
    mockInvoke.mockRejectedValue(ipcErr);

    await expect(finishLiveMatch(handle)).rejects.toMatchObject(ipcErr);
  });
});

// ---------------------------------------------------------------------------
// applyMatchCommand
// ---------------------------------------------------------------------------

describe("applyMatchCommand", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("invokes apply_match_command with handle + command", async () => {
    const handle = makeHandle();
    // Backend returns () → null over JSON bridge.
    mockInvoke.mockResolvedValue(null);

    const cmd: MatchCommand = { kind: "changeFormation", formation: "fwh.core:formation.4-4-2" };
    await applyMatchCommand(handle, cmd);

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith("apply_match_command", {
      handle,
      command: cmd,
    });
  });

  it("resolves to void when backend returns null", async () => {
    const handle = makeHandle();
    mockInvoke.mockResolvedValue(null);

    const cmd: MatchCommand = { kind: "changePressLevel", level: "high" };
    // Should not throw.
    await expect(applyMatchCommand(handle, cmd)).resolves.toBeUndefined();
  });

  it("resolves to void when backend returns undefined", async () => {
    const handle = makeHandle();
    mockInvoke.mockResolvedValue(undefined);

    const cmd: MatchCommand = { kind: "changeTempoBias", bias: "slow" };
    await expect(applyMatchCommand(handle, cmd)).resolves.toBeUndefined();
  });

  it("propagates LiveMatchCommandUnimplemented IpcError (all commands T4-5a)", async () => {
    const handle = makeHandle();
    const ipcErr = {
      kind: "liveMatchCommandUnimplemented",
      commandKind: "substitute",
    };
    mockInvoke.mockRejectedValue(ipcErr);

    const cmd: MatchCommand = { kind: "substitute", playerIn: 9, playerOut: 0 };
    await expect(applyMatchCommand(handle, cmd)).rejects.toMatchObject(ipcErr);
  });

  it("passes the commandKind verbatim in the IpcError for each command variant", async () => {
    const handle = makeHandle();
    // Spot-check 3 of the 9 variants to confirm the wire shape is stable.
    const cases: Array<[MatchCommand, string]> = [
      [{ kind: "changeFormation", formation: "fwh.core:formation.4-3-3" }, "changeFormation"],
      [{ kind: "teamTalk", messageId: "fwh.core:teamtalk_00001" }, "teamTalk"],
      [{ kind: "changePressLevel", level: "high" }, "changePressLevel"],
    ];

    for (const [cmd, expectedKind] of cases) {
      mockInvoke.mockRejectedValue({
        kind: "liveMatchCommandUnimplemented",
        commandKind: expectedKind,
      });

      await expect(applyMatchCommand(handle, cmd)).rejects.toMatchObject({
        kind: "liveMatchCommandUnimplemented",
        commandKind: expectedKind,
      });

      mockInvoke.mockReset();
    }
  });
});

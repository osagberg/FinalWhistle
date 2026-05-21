/**
 * T1-3.6: runtime-shape validators for IPC payloads (Codex audit P1 response).
 *
 * Each guard tested against:
 *   1. A valid payload (positive case)
 *   2. 2-3 malformed payloads with specific shape distortions (negative cases)
 *
 * Plus `safeInvoke` integration test confirming guard failure throws
 * `IpcShapeError` with the command name + payload preview.
 *
 * Non-vacuous per the iii-c lesson: each negative case asserts a SPECIFIC
 * field-level distortion (wrong type, missing field, wrong array length,
 * unknown discriminant) rather than just `expect(guard(garbage)).toBe(false)`.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Mock @tauri-apps/api/core BEFORE importing the SUT.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import {
  IpcShapeError,
  isBackendHandshake,
  isMatchEvent,
  isMatchFrameDTO,
  isMatchResult,
  isPlayerDetail,
  isScore,
  isSquadPlayer,
  isSquadPlayerArray,
  safeInvoke,
} from "./runtime-validators";

type Mock = ReturnType<typeof vi.fn>;
const mockInvoke = invoke as unknown as Mock;

// ---------------------------------------------------------------------------
// isScore
// ---------------------------------------------------------------------------

describe("isScore", () => {
  it("accepts valid Score", () => {
    expect(isScore({ home: 2, away: 1 })).toBe(true);
  });

  it("rejects when home is a string", () => {
    expect(isScore({ home: "2", away: 1 })).toBe(false);
  });

  it("rejects null and undefined", () => {
    expect(isScore(null)).toBe(false);
    expect(isScore(undefined)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// isMatchEvent
// ---------------------------------------------------------------------------

describe("isMatchEvent", () => {
  it("accepts valid Goal event", () => {
    expect(
      isMatchEvent({ tick: 540, minute: 9, kind: "Goal" }),
    ).toBe(true);
  });

  it("accepts event with optional description", () => {
    expect(
      isMatchEvent({
        tick: 0,
        minute: 0,
        kind: "KickOff",
        description: "Kick-off.",
      }),
    ).toBe(true);
  });

  it("rejects unknown kind (closed-union enforcement)", () => {
    expect(
      isMatchEvent({ tick: 540, minute: 9, kind: "PenaltyKick" }),
    ).toBe(false);
  });

  it("rejects NaN tick (non-finite number)", () => {
    expect(
      isMatchEvent({ tick: Number.NaN, minute: 0, kind: "Goal" }),
    ).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// isMatchResult
// ---------------------------------------------------------------------------

describe("isMatchResult", () => {
  const validResult = {
    finalScore: { home: 1, away: 0 },
    canonicalHash: "blake3:" + "a".repeat(64),
    matchEvents: [
      { tick: 0, minute: 0, kind: "KickOff" },
      { tick: 540, minute: 9, kind: "Goal" },
    ],
    seedHex: "0xdeadbeefdeadbeef",
    tickCount: 600,
    commentaryPreview: ["Kick-off.", "Goal!"],
  };

  it("accepts a fully-valid MatchResult", () => {
    expect(isMatchResult(validResult)).toBe(true);
  });

  it("rejects when canonicalHash lacks blake3: prefix (drift sentinel)", () => {
    const bad = { ...validResult, canonicalHash: "sha256:" + "0".repeat(64) };
    expect(isMatchResult(bad)).toBe(false);
  });

  it("rejects when matchEvents contains a malformed entry", () => {
    const bad = {
      ...validResult,
      matchEvents: [
        ...validResult.matchEvents,
        { tick: 999, minute: 16, kind: "BogusKind" },
      ],
    };
    expect(isMatchResult(bad)).toBe(false);
  });

  it("rejects when finalScore is missing", () => {
    const bad = { ...validResult } as Record<string, unknown>;
    delete bad.finalScore;
    expect(isMatchResult(bad)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// isMatchFrameDTO
// ---------------------------------------------------------------------------

describe("isMatchFrameDTO", () => {
  function makeFrame(overrides: Record<string, unknown> = {}) {
    return {
      seedHex: "0x1",
      tick: 0,
      homeScore: 0,
      awayScore: 0,
      players: Array.from({ length: 22 }, (_, slot) => ({
        slot,
        posX: 0,
        posY: 0,
        velX: 0,
        velY: 0,
      })),
      ball: { posX: 0, posY: 0, posZ: 0, velX: 0, velY: 0, velZ: 0 },
      possession: null,
      ...overrides,
    };
  }

  it("accepts valid frame with null possession", () => {
    expect(isMatchFrameDTO(makeFrame())).toBe(true);
  });

  it("accepts valid frame with numeric possession (T1-3.6)", () => {
    expect(isMatchFrameDTO(makeFrame({ possession: 9 }))).toBe(true);
  });

  it("rejects frame with wrong player count (≠22)", () => {
    const bad = makeFrame();
    bad.players = bad.players.slice(0, 21);
    expect(isMatchFrameDTO(bad)).toBe(false);
  });

  it("rejects frame with NaN coordinate in ball", () => {
    const bad = makeFrame();
    bad.ball.posX = Number.NaN;
    expect(isMatchFrameDTO(bad)).toBe(false);
  });

  it("rejects frame where possession is a string", () => {
    expect(isMatchFrameDTO(makeFrame({ possession: "9" }))).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// isBackendHandshake
// ---------------------------------------------------------------------------

describe("isBackendHandshake", () => {
  it("accepts valid handshake", () => {
    expect(
      isBackendHandshake({
        appVersion: "0.1.0",
        message: "Backend live.",
        backendReady: true,
      }),
    ).toBe(true);
  });

  it("rejects when backendReady is a string", () => {
    expect(
      isBackendHandshake({
        appVersion: "0.1.0",
        message: "Backend live.",
        backendReady: "true",
      }),
    ).toBe(false);
  });

  it("rejects when appVersion is missing", () => {
    expect(
      isBackendHandshake({
        message: "Backend live.",
        backendReady: true,
      }),
    ).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// isSquadPlayer / isSquadPlayerArray — T2-7
// ---------------------------------------------------------------------------

describe("isSquadPlayer", () => {
  const valid = {
    playerId: "fwh.core:player_00001",
    name: "Emeka Thorne",
    role: "Striker",
    birthRegion: "Ashvale",
    phenotypeLabels: ["Pure finisher", "Poacher"],
  };

  it("accepts a fully-valid SquadPlayer", () => {
    expect(isSquadPlayer(valid)).toBe(true);
  });

  it("accepts a SquadPlayer with empty phenotypeLabels array", () => {
    expect(isSquadPlayer({ ...valid, phenotypeLabels: [] })).toBe(true);
  });

  it("rejects when name is missing", () => {
    const bad = { ...valid } as Record<string, unknown>;
    delete bad.name;
    expect(isSquadPlayer(bad)).toBe(false);
  });

  it("rejects when playerId is a number", () => {
    expect(isSquadPlayer({ ...valid, playerId: 42 })).toBe(false);
  });

  it("rejects when birthRegion is missing", () => {
    const bad = { ...valid } as Record<string, unknown>;
    delete bad.birthRegion;
    expect(isSquadPlayer(bad)).toBe(false);
  });

  it("rejects when phenotypeLabels contains a non-string element", () => {
    expect(isSquadPlayer({ ...valid, phenotypeLabels: ["ok", 99] })).toBe(false);
  });

  it("rejects null", () => {
    expect(isSquadPlayer(null)).toBe(false);
  });
});

describe("isSquadPlayerArray", () => {
  const validPlayer = {
    playerId: "fwh.core:player_00001",
    name: "Emeka Thorne",
    role: "Striker",
    birthRegion: "Ashvale",
    phenotypeLabels: ["Pure finisher"],
  };

  it("accepts an empty array", () => {
    expect(isSquadPlayerArray([])).toBe(true);
  });

  it("accepts an array of valid SquadPlayers", () => {
    expect(isSquadPlayerArray([validPlayer, { ...validPlayer, playerId: "fwh.core:player_00002" }])).toBe(true);
  });

  it("rejects when one element has a missing field", () => {
    const bad = { ...validPlayer } as Record<string, unknown>;
    delete bad.role;
    expect(isSquadPlayerArray([validPlayer, bad])).toBe(false);
  });

  it("rejects a non-array", () => {
    expect(isSquadPlayerArray(validPlayer)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// isPlayerDetail — T3-6
// ---------------------------------------------------------------------------

describe("isPlayerDetail", () => {
  const validPhenotype = {
    playerId: "fwh.core:player_00001",
    name: "Emeka Thorne",
    role: "Striker",
    birthRegion: "Ashvale",
    phenotypeLabels: ["Pure finisher", "Poacher"],
  };

  const valid = {
    phenotype: validPhenotype,
    memoryCallbacks: ["Made his debut on a wet Tuesday.", "First senior goal."],
    contractStatus: null,
  };

  it("accepts a fully-valid PlayerDetail with null contractStatus", () => {
    expect(isPlayerDetail(valid)).toBe(true);
  });

  it("accepts PlayerDetail with a string contractStatus", () => {
    expect(isPlayerDetail({ ...valid, contractStatus: "2 years remaining" })).toBe(true);
  });

  it("accepts PlayerDetail with empty memoryCallbacks array", () => {
    expect(isPlayerDetail({ ...valid, memoryCallbacks: [] })).toBe(true);
  });

  it("accepts PlayerDetail with empty phenotypeLabels", () => {
    expect(
      isPlayerDetail({
        ...valid,
        phenotype: { ...validPhenotype, phenotypeLabels: [] },
      }),
    ).toBe(true);
  });

  it("rejects when phenotype is missing", () => {
    const bad = { ...valid } as Record<string, unknown>;
    delete bad.phenotype;
    expect(isPlayerDetail(bad)).toBe(false);
  });

  it("rejects when phenotype.playerId is a number", () => {
    expect(
      isPlayerDetail({
        ...valid,
        phenotype: { ...validPhenotype, playerId: 42 },
      }),
    ).toBe(false);
  });

  it("rejects when phenotype.name is missing", () => {
    const badPhenotype = { ...validPhenotype } as Record<string, unknown>;
    delete badPhenotype.name;
    expect(isPlayerDetail({ ...valid, phenotype: badPhenotype })).toBe(false);
  });

  it("rejects when memoryCallbacks contains a non-string element", () => {
    expect(isPlayerDetail({ ...valid, memoryCallbacks: ["ok", 99] })).toBe(false);
  });

  it("rejects when contractStatus is a number (not string | null)", () => {
    expect(isPlayerDetail({ ...valid, contractStatus: 42 })).toBe(false);
  });

  it("rejects null", () => {
    expect(isPlayerDetail(null)).toBe(false);
  });

  it("rejects a non-object primitive", () => {
    expect(isPlayerDetail("string")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// safeInvoke — integration
// ---------------------------------------------------------------------------

describe("safeInvoke", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  afterEach(() => {
    mockInvoke.mockReset();
  });

  it("returns the typed payload on a passing guard", async () => {
    mockInvoke.mockResolvedValue({
      appVersion: "0.1.0",
      message: "Backend live.",
      backendReady: true,
    });
    const result = await safeInvoke(
      "get_backend_handshake",
      {},
      isBackendHandshake,
    );
    expect(result.appVersion).toBe("0.1.0");
    expect(result.backendReady).toBe(true);
  });

  it("throws IpcShapeError with command name when guard fails", async () => {
    mockInvoke.mockResolvedValue({ wrong: "shape" });
    await expect(
      safeInvoke("get_backend_handshake", {}, isBackendHandshake),
    ).rejects.toBeInstanceOf(IpcShapeError);
    await expect(
      safeInvoke("get_backend_handshake", {}, isBackendHandshake),
    ).rejects.toThrow(/get_backend_handshake/);
  });

  it("IpcShapeError carries payload preview for debugging", async () => {
    mockInvoke.mockResolvedValue({ wrong: "shape", extra: 123 });
    try {
      await safeInvoke("get_backend_handshake", {}, isBackendHandshake);
      throw new Error("safeInvoke should have thrown");
    } catch (e) {
      expect(e).toBeInstanceOf(IpcShapeError);
      const err = e as IpcShapeError;
      expect(err.command).toBe("get_backend_handshake");
      expect(err.payloadPreview).toContain("wrong");
      expect(err.payloadPreview).toContain("shape");
    }
  });
});

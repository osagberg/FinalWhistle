/**
 * T1-13: tests for FrameSource module (5th + FINAL audit-triage P1 closure).
 *
 * Covers all 3 exports per Codex 2026-05-16 audit P1 acceptance:
 *   1. TauriFrameSource — invoke round-trip + MAX_FRAMES_PER_REQUEST guard
 *   2. HttpFrameSource — fetch happy path + fetch-failure + shape validation
 *   3. frameSourceFromUrlParams — every documented URL-param branch
 *
 * Non-vacuous per the iii-c lesson: each test asserts SPECIFIC error message
 * substrings or exact value shapes, not just `expect.toThrow()` / `toBeInstanceOf(Error)`.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Mock } from "vitest";

// Mock @tauri-apps/api/core BEFORE importing the SUT (vi.mock is hoisted).
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import {
  FrameSourceConfigError,
  HttpFrameSource,
  TauriFrameSource,
  frameSourceFromUrlParams,
} from "./FrameSource";
import { MAX_FRAMES_PER_REQUEST, type MatchFrameDTO } from "~/lib/types";

const mockInvoke = invoke as unknown as Mock;

// ---------------------------------------------------------------------------
// Helper: minimal valid MatchFrameDTO for shape-validation success paths.
// Mirrors what `dump_frames` actually emits + what HttpFrameSource accepts.
//
// Return type annotated as `MatchFrameDTO` per T1-13 type-design audit P2:
// if MatchFrameDTO gains a required field, this fixture goes RED at compile
// time + every shape-validation test fails loudly — instead of silently
// passing while the production validator (extended to check the new field)
// rejects the under-specified fixture at runtime.
// ---------------------------------------------------------------------------

function makeValidFrame(tick: number): MatchFrameDTO {
  const players = Array.from({ length: 22 }, (_, slot) => ({
    slot,
    posX: 0,
    posY: 0,
    velX: 0,
    velY: 0,
  }));
  return {
    seedHex: "0xdeadbeefdeadbeef",
    tick,
    homeScore: 0,
    awayScore: 0,
    players,
    ball: { posX: 0, posY: 0, posZ: 0, velX: 0, velY: 0, velZ: 0 },
    // T1-3.6: MatchFrameDTO now exposes possession (Option<u8> on Rust side).
    // Null here mirrors the loose-ball / pre-kickoff state.
    possession: null,
  };
}

// ---------------------------------------------------------------------------
// TauriFrameSource
// ---------------------------------------------------------------------------

describe("TauriFrameSource", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("invokes match_frames with seedHex + tickCount", async () => {
    const frames = [makeValidFrame(0), makeValidFrame(1)];
    mockInvoke.mockResolvedValue(frames);

    const src = new TauriFrameSource("0xfeedbeefcafefade", 60);
    const result = await src.loadFrames();

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith("match_frames", {
      seedHex: "0xfeedbeefcafefade",
      tickCount: 60,
    });
    expect(result).toHaveLength(2);
    expect(result[0]?.tick).toBe(0);
  });

  it("rejects tickCount > MAX_FRAMES_PER_REQUEST before any IPC call", async () => {
    const src = new TauriFrameSource("0x1", MAX_FRAMES_PER_REQUEST + 1);
    await expect(src.loadFrames()).rejects.toBeInstanceOf(
      FrameSourceConfigError,
    );
    await expect(src.loadFrames()).rejects.toThrow(
      /exceeds MAX_FRAMES_PER_REQUEST/,
    );
    // Critical: the IPC call MUST NOT fire when the guard rejects (avoid the
    // round-trip on a guaranteed failure).
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("accepts tickCount equal to MAX_FRAMES_PER_REQUEST (boundary)", async () => {
    mockInvoke.mockResolvedValue([]);
    const src = new TauriFrameSource("0x1", MAX_FRAMES_PER_REQUEST);
    await expect(src.loadFrames()).resolves.toEqual([]);
    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });
});

// ---------------------------------------------------------------------------
// HttpFrameSource
// ---------------------------------------------------------------------------

describe("HttpFrameSource", () => {
  const originalFetch = globalThis.fetch;

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it("returns parsed frames on 200 OK with valid shape", async () => {
    const frames = [makeValidFrame(0), makeValidFrame(1)];
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(frames), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    const src = new HttpFrameSource("/dev-fixtures/smoke.json");
    const result = await src.loadFrames();
    expect(result).toHaveLength(2);
    expect(result[0]?.players).toHaveLength(22);
  });

  it("throws with status + statusText on non-OK response", async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response("not found", { status: 404, statusText: "Not Found" }),
    );

    const src = new HttpFrameSource("/missing.json");
    await expect(src.loadFrames()).rejects.toThrow(/404/);
    await expect(src.loadFrames()).rejects.toThrow(/Not Found/);
  });

  it("rejects malformed JSON shape (missing fields)", async () => {
    const bad = [{ tick: 0, players: [{}], ball: {} }]; // missing scores, seed, etc.
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(bad), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    const src = new HttpFrameSource("/bad.json");
    await expect(src.loadFrames()).rejects.toThrow(/not MatchFrameDTO/);
  });

  it("rejects frames with wrong player count (≠22)", async () => {
    const frame = makeValidFrame(0);
    frame.players = frame.players.slice(0, 21); // drop one player
    const bad = [frame];
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(bad), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    const src = new HttpFrameSource("/bad-count.json");
    await expect(src.loadFrames()).rejects.toThrow(/not MatchFrameDTO/);
  });

  it("rejects frames with non-finite coordinates (NaN/Infinity)", async () => {
    const frame = makeValidFrame(0);
    frame.ball.posX = Number.NaN;
    const bad = [frame];
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify(bad), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    const src = new HttpFrameSource("/bad-nan.json");
    await expect(src.loadFrames()).rejects.toThrow(/not MatchFrameDTO/);
  });
});

// ---------------------------------------------------------------------------
// frameSourceFromUrlParams — every documented URL-param branch
// ---------------------------------------------------------------------------

describe("frameSourceFromUrlParams", () => {
  it("returns TauriFrameSource when ?source absent (default)", () => {
    const src = frameSourceFromUrlParams("");
    expect(src).toBeInstanceOf(TauriFrameSource);
  });

  it("returns TauriFrameSource when ?source=tauri (explicit)", () => {
    const src = frameSourceFromUrlParams("?source=tauri");
    expect(src).toBeInstanceOf(TauriFrameSource);
  });

  it("honors ?seed and ?ticks for TauriFrameSource", async () => {
    const src = frameSourceFromUrlParams("?seed=0xfeed&ticks=90");
    expect(src).toBeInstanceOf(TauriFrameSource);
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue([]);
    await src.loadFrames();
    expect(mockInvoke).toHaveBeenCalledWith("match_frames", {
      seedHex: "0xfeed",
      tickCount: 90,
    });
  });

  it("falls back to defaults when ?ticks is non-numeric or non-positive", async () => {
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue([]);
    const src = frameSourceFromUrlParams("?ticks=not-a-number");
    await src.loadFrames();
    expect(mockInvoke).toHaveBeenCalledWith(
      "match_frames",
      expect.objectContaining({ tickCount: 60 }),
    );
  });

  it("returns HttpFrameSource for ?source=fixture:/rooted/path", () => {
    const src = frameSourceFromUrlParams("?source=fixture:/dev-fixtures/smoke.json");
    expect(src).toBeInstanceOf(HttpFrameSource);
  });

  it("returns HttpFrameSource for ?source=fixture:./relative/path", () => {
    const src = frameSourceFromUrlParams("?source=fixture:./local.json");
    expect(src).toBeInstanceOf(HttpFrameSource);
  });

  it("returns HttpFrameSource for ?source=fixture:https://...", () => {
    const src = frameSourceFromUrlParams(
      "?source=fixture:https://example.com/frames.json",
    );
    expect(src).toBeInstanceOf(HttpFrameSource);
  });

  it("throws FrameSourceConfigError for unknown source value", () => {
    expect(() => frameSourceFromUrlParams("?source=bogus")).toThrow(
      FrameSourceConfigError,
    );
    expect(() => frameSourceFromUrlParams("?source=bogus")).toThrow(
      /unknown source value/,
    );
  });

  it("throws FrameSourceConfigError for empty fixture path", () => {
    expect(() => frameSourceFromUrlParams("?source=fixture:")).toThrow(
      FrameSourceConfigError,
    );
    expect(() => frameSourceFromUrlParams("?source=fixture:")).toThrow(
      /requires a path/,
    );
  });

  it("rejects non-http schemes in fixture path (security guard)", () => {
    // javascript: scheme would have bypassed an older regex-based guard;
    // the current factory rejects ANY non-http(s) scheme.
    expect(() =>
      frameSourceFromUrlParams("?source=fixture:javascript:alert(1)"),
    ).toThrow(FrameSourceConfigError);
    expect(() =>
      frameSourceFromUrlParams("?source=fixture:javascript:alert(1)"),
    ).toThrow(/scheme/);

    expect(() => frameSourceFromUrlParams("?source=fixture:file:///etc/passwd")).toThrow(
      /scheme/,
    );
    expect(() => frameSourceFromUrlParams("?source=fixture:data:text/html,<x>")).toThrow(
      /scheme/,
    );
  });

  it("case-sensitive: ?source=Tauri (capital T) fails fast", () => {
    // Pre-T1-2b audit caught this: silent fallback masked typos. Now fails loudly.
    expect(() => frameSourceFromUrlParams("?source=Tauri")).toThrow(
      FrameSourceConfigError,
    );
  });
});

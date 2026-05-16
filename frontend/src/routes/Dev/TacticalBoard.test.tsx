/**
 * T1-13: tests for Dev/TacticalBoard lifecycle + window.fwDev debug surface
 * (5th + FINAL audit-triage P1 closure).
 *
 * What's actually testable under jsdom:
 *   - Pixi v8 needs WebGL → mock `pixi.js` (same pattern as Match.test.tsx)
 *   - useSearchParams comes from @solidjs/router → mock it per-test so each test
 *     drives a specific URL-param shape without wiring a real Router
 *   - Bogus URL params surface as loadError BEFORE any Pixi init (T1-2a Codex fix)
 *   - window.fwDev exposes scrubTo / currentTick / frameCount with bounds-checked scrub
 *
 * Non-vacuous per the iii-c lesson: each test asserts SPECIFIC behavior — the
 * Pixi mock's call counts, the loadError string substring, the fwDev surface shape
 * + bounds-checked scrubTo semantics — not just "renders without crashing."
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Mock } from "vitest";
import { render, waitFor } from "@solidjs/testing-library";

// Mock @tauri-apps/api/core BEFORE importing the SUT.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// Mock @solidjs/router::useSearchParams. The SUT reconstructs an URLSearchParams
// from this dict via `"?" + new URLSearchParams(searchParams as ...).toString()`,
// so each test sets `mockSearchParams` BEFORE rendering to drive the URL-param branch.
let mockSearchParams: Record<string, string> = {};
vi.mock("@solidjs/router", () => ({
  useSearchParams: () => [mockSearchParams, vi.fn()],
}));

// Mock pixi.js — jsdom has no WebGL so real Application.init() throws.
// Capture instances so tests can assert lifecycle (single construction, destroy-on-cleanup).
const mockAppInstances: Array<{
  init: Mock;
  destroy: Mock;
  canvas: HTMLCanvasElement;
  stage: { addChild: Mock };
}> = [];

vi.mock("pixi.js", () => {
  return {
    Application: vi.fn().mockImplementation(() => {
      const instance = {
        init: vi.fn().mockResolvedValue(undefined),
        destroy: vi.fn(),
        canvas: document.createElement("canvas"),
        stage: { addChild: vi.fn() },
      };
      mockAppInstances.push(instance);
      return instance;
    }),
    Graphics: vi.fn().mockImplementation(() => ({
      fill: vi.fn().mockReturnThis(),
      circle: vi.fn().mockReturnThis(),
      stroke: vi.fn().mockReturnThis(),
      rect: vi.fn().mockReturnThis(),
      moveTo: vi.fn().mockReturnThis(),
      lineTo: vi.fn().mockReturnThis(),
      setStrokeStyle: vi.fn().mockReturnThis(),
      x: 0,
      y: 0,
    })),
  };
});

import { invoke } from "@tauri-apps/api/core";
import type { MatchFrameDTO } from "~/lib/types";
import TacticalBoard from "./TacticalBoard";

const mockInvoke = invoke as unknown as Mock;

// Minimal valid MatchFrameDTO array for mocked Tauri responses.
//
// Return type annotated as `MatchFrameDTO[]` per T1-13 type-design audit P2:
// if MatchFrameDTO gains a required field, this fixture fails compile +
// every test using it surfaces the drift loudly — instead of silently
// passing while the production validator (extended to require the new
// field) rejects the under-specified fixture at runtime.
function validFrames(count: number): MatchFrameDTO[] {
  return Array.from({ length: count }, (_, tick) => ({
    seedHex: "0xdeadbeefdeadbeef",
    tick,
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
    // T1-3.6: MatchFrameDTO now exposes possession (Option<u8> on Rust side).
    possession: null,
  }));
}

// ---------------------------------------------------------------------------
// Lifecycle invariants (Frontend/RULES.md §4)
// ---------------------------------------------------------------------------

describe("Dev/TacticalBoard lifecycle", () => {
  beforeEach(() => {
    mockAppInstances.length = 0;
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue(validFrames(3));
    mockSearchParams = {}; // default URL: TauriFrameSource
    delete window.fwDev;
  });

  afterEach(() => {
    delete window.fwDev;
  });

  it("initialises a single Pixi Application on mount", async () => {
    const { unmount } = render(() => <TacticalBoard />);
    await waitFor(() => expect(mockAppInstances.length).toBe(1));
    expect(mockAppInstances[0]?.init).toHaveBeenCalledTimes(1);
    unmount();
  });

  it("destroys the Application on unmount (no WebGL leak)", async () => {
    const { unmount } = render(() => <TacticalBoard />);
    await waitFor(() => expect(mockAppInstances.length).toBe(1));
    const instance = mockAppInstances[0];
    if (!instance) throw new Error("instance");

    // Wait for async init to resolve so destroy fires on a fully-initialised
    // Application (the early-out at TacticalBoard.tsx:151-154 covers fast HMR
    // teardown; here we want the post-init destroy path).
    await waitFor(() => expect(instance.init).toHaveBeenCalled());
    unmount();
    await waitFor(() =>
      expect(instance.destroy).toHaveBeenCalledWith(true, { children: true }),
    );
  });

  it("sets loadError WITHOUT Pixi init when URL param is bogus", async () => {
    // Pre-T1-2a Codex Tier-2 guard: bogus ?source=... surfaces as loadError
    // (frameSourceFromUrlParams throws synchronously inside onMount; the early
    // return at TacticalBoard.tsx:131-137 keeps Pixi init from firing).
    mockSearchParams = { source: "bogus" };
    const { container, unmount } = render(() => <TacticalBoard />);
    await waitFor(() => {
      expect(container.textContent ?? "").toMatch(/unknown source value/);
    });
    // CRITICAL: Pixi must NOT have been initialised on this path.
    expect(mockAppInstances.length).toBe(0);
    unmount();
  });
});

// ---------------------------------------------------------------------------
// window.fwDev debug surface
// ---------------------------------------------------------------------------

describe("Dev/TacticalBoard window.fwDev", () => {
  beforeEach(() => {
    mockAppInstances.length = 0;
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue(validFrames(5)); // 5 frames → ticks 0..4
    mockSearchParams = {};
    delete window.fwDev;
  });

  afterEach(() => {
    delete window.fwDev;
  });

  it("exposes scrubTo + currentTick + frameCount with bounds-checked scrub", async () => {
    const { unmount } = render(() => <TacticalBoard />);
    // Wait for async init + fwDev exposure. (Vitest defaults
    // `import.meta.env.DEV = true`, so the producer's gate at
    // TacticalBoard.tsx fires + window.fwDev gets populated.)
    await waitFor(() => {
      expect(window.fwDev).toBeDefined();
    });
    const fwDev = window.fwDev;
    if (!fwDev) throw new Error("fwDev not exposed");

    // Surface shape check.
    expect(typeof fwDev.scrubTo).toBe("function");
    expect(typeof fwDev.currentTick).toBe("function");
    expect(typeof fwDev.frameCount).toBe("function");

    // Wait for frames to load.
    await waitFor(() => expect(fwDev.frameCount()).toBe(5));

    // Default tick is 0.
    expect(fwDev.currentTick()).toBe(0);

    // scrubTo respects Math.max(0, Math.min(n, frames().length - 1)).
    fwDev.scrubTo(3);
    expect(fwDev.currentTick()).toBe(3);

    // Above-bounds clamps to last valid index.
    fwDev.scrubTo(999);
    expect(fwDev.currentTick()).toBe(4); // frames.length - 1 = 5 - 1

    // Below-bounds clamps to 0.
    fwDev.scrubTo(-50);
    expect(fwDev.currentTick()).toBe(0);

    unmount();
  });
});

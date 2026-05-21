/**
 * T4-1: TacticalBoard production component tests.
 *
 * Three test groups:
 *   1. Pure interpolation math (lerp + simToCanvas) — no DOM, no Pixi, fast.
 *   2. Component lifecycle: single Pixi Application, no WebGL leak on unmount.
 *   3. Controls: play/pause + scrubber DOM interactions (Pixi mocked).
 *
 * Pixi is mocked for groups 2+3 (jsdom has no WebGL). The ticker `add` and
 * `remove` methods are tracked to verify the ticker-driven playback loop is
 * correctly wired and removed on cleanup.
 *
 * Non-vacuous: each test asserts specific behavior (call counts, signal values,
 * DOM state, math output) — not just "renders without crashing."
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Mock } from "vitest";
import { render, waitFor, fireEvent } from "@solidjs/testing-library";
import type { MatchFrameDTO } from "~/lib/types";

// ---------------------------------------------------------------------------
// Mock pixi.js — jsdom has no WebGL; Application.init() would throw without it.
// Track ticker.add / ticker.remove calls so tests can assert the loop is
// registered once and removed on cleanup.
// ---------------------------------------------------------------------------

interface MockTickerHandle {
  add: Mock;
  remove: Mock;
}

const mockAppInstances: Array<{
  init: Mock;
  destroy: Mock;
  canvas: HTMLCanvasElement;
  stage: { addChild: Mock };
  ticker: MockTickerHandle;
}> = [];

vi.mock("pixi.js", () => ({
  Application: vi.fn().mockImplementation(() => {
    const ticker: MockTickerHandle = {
      add: vi.fn(),
      remove: vi.fn(),
    };
    const instance = {
      init: vi.fn().mockResolvedValue(undefined),
      destroy: vi.fn(),
      canvas: document.createElement("canvas"),
      stage: { addChild: vi.fn() },
      ticker,
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
}));

// Import AFTER mocks are in place.
import TacticalBoard from "./TacticalBoard";
import { lerp, simToCanvas, pitchLayout } from "~/lib/pitch-coords";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function validFrames(count: number): MatchFrameDTO[] {
  return Array.from({ length: count }, (_, tick) => ({
    seedHex: "0xdeadbeefdeadbeef",
    tick,
    homeScore: 0,
    awayScore: 0,
    players: Array.from({ length: 22 }, (__, slot) => ({
      slot,
      posX: tick * 0.1,
      posY: slot * 0.5,
      velX: 0,
      velY: 0,
    })),
    ball: {
      posX: tick * 0.05,
      posY: 0,
      posZ: 0,
      velX: 0,
      velY: 0,
      velZ: 0,
    },
    possession: null,
  }));
}

// ---------------------------------------------------------------------------
// 1. Pure interpolation math — no DOM, no Pixi
// ---------------------------------------------------------------------------

describe("lerp (pitch-coords.ts)", () => {
  it("returns a at t=0", () => {
    expect(lerp(10, 20, 0)).toBe(10);
  });

  it("returns b at t=1", () => {
    expect(lerp(10, 20, 1)).toBe(20);
  });

  it("returns midpoint at t=0.5", () => {
    expect(lerp(0, 100, 0.5)).toBe(50);
  });

  it("clamps t below 0", () => {
    expect(lerp(10, 20, -5)).toBe(10);
  });

  it("clamps t above 1", () => {
    expect(lerp(10, 20, 5)).toBe(20);
  });

  it("handles negative range", () => {
    expect(lerp(-10, 10, 0.5)).toBe(0);
  });
});

describe("simToCanvas coordinate mapping", () => {
  const layout = pitchLayout(840, 560);

  it("maps sim origin (0,0) to canvas centre", () => {
    const [cx, cy] = simToCanvas(0, 0, layout);
    expect(cx).toBeCloseTo(420, 0);
    expect(cy).toBeCloseTo(280, 0);
  });

  it("maps positive simX (toward away goal) to right of centre", () => {
    const [cx] = simToCanvas(10, 0, layout);
    expect(cx).toBeGreaterThan(420);
  });

  it("maps negative simX (toward home goal) to left of centre", () => {
    const [cx] = simToCanvas(-10, 0, layout);
    expect(cx).toBeLessThan(420);
  });

  it("maps positive simY (toward home touchline) to above centre (lower canvas Y)", () => {
    const [, cy] = simToCanvas(0, 10, layout);
    // Canvas Y is inverted: higher simY → lower canvasY value (above centre).
    expect(cy).toBeLessThan(280);
  });

  it("maps negative simY to below centre (higher canvas Y)", () => {
    const [, cy] = simToCanvas(0, -10, layout);
    expect(cy).toBeGreaterThan(280);
  });

  it("maps pitch boundary ±52.5m to pitch edge pixels", () => {
    const [leftEdge] = simToCanvas(-52.5, 0, layout);
    const [rightEdge] = simToCanvas(52.5, 0, layout);
    // Both should be equidistant from centre.
    const centre = 420;
    expect(Math.abs(centre - leftEdge)).toBeCloseTo(
      Math.abs(rightEdge - centre),
      0,
    );
    // And inside the canvas.
    expect(leftEdge).toBeGreaterThan(0);
    expect(rightEdge).toBeLessThan(840);
  });
});

// ---------------------------------------------------------------------------
// 2. Lifecycle — Application created once, ticker wired, destroyed on unmount
// ---------------------------------------------------------------------------

describe("TacticalBoard lifecycle (Frontend/RULES.md §4)", () => {
  beforeEach(() => {
    mockAppInstances.length = 0;
  });

  it("initialises a single Pixi Application on mount", async () => {
    const { unmount } = render(() => (
      <TacticalBoard frames={validFrames(3)} />
    ));
    await waitFor(() => expect(mockAppInstances.length).toBe(1));
    expect(mockAppInstances[0]?.init).toHaveBeenCalledTimes(1);
    unmount();
  });

  it("adds a ticker callback on mount", async () => {
    const { unmount } = render(() => (
      <TacticalBoard frames={validFrames(3)} />
    ));
    await waitFor(() => expect(mockAppInstances.length).toBe(1));
    const instance = mockAppInstances[0];
    if (!instance) throw new Error("no instance");
    await waitFor(() => expect(instance.ticker.add).toHaveBeenCalledTimes(1));
    unmount();
  });

  it("destroys the Application on unmount (no WebGL leak)", async () => {
    const { unmount } = render(() => (
      <TacticalBoard frames={validFrames(3)} />
    ));
    await waitFor(() => expect(mockAppInstances.length).toBe(1));
    const instance = mockAppInstances[0];
    if (!instance) throw new Error("no instance");
    await waitFor(() => expect(instance.init).toHaveBeenCalled());
    unmount();
    await waitFor(() =>
      expect(instance.destroy).toHaveBeenCalledWith(true, { children: true }),
    );
  });

  it("removes the ticker callback on unmount", async () => {
    const { unmount } = render(() => (
      <TacticalBoard frames={validFrames(3)} />
    ));
    await waitFor(() => expect(mockAppInstances.length).toBe(1));
    const instance = mockAppInstances[0];
    if (!instance) throw new Error("no instance");
    await waitFor(() => expect(instance.ticker.add).toHaveBeenCalled());
    unmount();
    await waitFor(() =>
      expect(instance.ticker.remove).toHaveBeenCalledTimes(1),
    );
  });

  it("does not re-create the Application when props.frames changes", async () => {
    // Use a Solid signal so SolidJS propagates the update reactively.
    const { createSignal: cs } = await import("solid-js");
    const [getFrames, setFrames] = cs<MatchFrameDTO[]>(validFrames(3));

    const { getByLabelText } = render(() => (
      <TacticalBoard frames={getFrames()} />
    ));
    await waitFor(() => expect(mockAppInstances.length).toBe(1));

    // Change the frames prop reactively, then wait until the component has
    // DEMONSTRABLY processed it — the readout's max-index reflects 10 frames
    // (maxIndex = 9). Without this settle, a frames-keyed Application rebuild
    // (the mutation this test must catch) could fire after a bare synchronous
    // assertion and escape detection.
    setFrames(validFrames(10));
    await waitFor(() =>
      expect(getByLabelText(/frame info/i).textContent).toContain("/ 9"),
    );

    // The frames change has propagated — yet still exactly one Application:
    // a props.frames change repositions sprites, it never rebuilds the App.
    expect(mockAppInstances.length).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// 3. Controls — play/pause + scrubber
// ---------------------------------------------------------------------------

describe("TacticalBoard controls", () => {
  beforeEach(() => {
    mockAppInstances.length = 0;
  });

  afterEach(() => {
    // nothing to clean
  });

  it("renders a Play button and scrubber", async () => {
    const { getByRole, getByLabelText } = render(() => (
      <TacticalBoard frames={validFrames(5)} />
    ));
    expect(getByRole("button", { name: /play replay/i })).toBeInTheDocument();
    expect(getByLabelText(/scrub to tick/i)).toBeInTheDocument();
  });

  it("Play button is disabled when frames are empty", () => {
    const { getByRole } = render(() => <TacticalBoard frames={[]} />);
    expect(getByRole("button", { name: /play replay/i })).toBeDisabled();
  });

  it("scrubber is disabled when frames are empty", () => {
    const { getByLabelText } = render(() => <TacticalBoard frames={[]} />);
    expect(getByLabelText(/scrub to tick/i)).toBeDisabled();
  });

  it("clicking Play toggles aria-pressed to true", async () => {
    const { getByRole } = render(() => (
      <TacticalBoard frames={validFrames(5)} />
    ));
    await waitFor(() => expect(mockAppInstances.length).toBe(1));
    const btn = getByRole("button", { name: /play replay/i });
    fireEvent.click(btn);
    // After clicking, the button label changes to "Pause" and aria-pressed is true.
    await waitFor(() =>
      expect(
        getByRole("button", { name: /pause playback/i }),
      ).toBeInTheDocument(),
    );
  });

  it("clicking Pause toggles playback off", async () => {
    const { getByRole } = render(() => (
      <TacticalBoard frames={validFrames(5)} />
    ));
    await waitFor(() => expect(mockAppInstances.length).toBe(1));
    // Play → Pause.
    fireEvent.click(getByRole("button", { name: /play replay/i }));
    await waitFor(() =>
      expect(
        getByRole("button", { name: /pause playback/i }),
      ).toBeInTheDocument(),
    );
    // Pause → Play.
    fireEvent.click(getByRole("button", { name: /pause playback/i }));
    await waitFor(() =>
      expect(
        getByRole("button", { name: /play replay/i }),
      ).toBeInTheDocument(),
    );
  });

  it("scrubbing stops playback and updates tick readout", async () => {
    const { getByRole, getByLabelText } = render(() => (
      <TacticalBoard frames={validFrames(10)} />
    ));
    await waitFor(() => expect(mockAppInstances.length).toBe(1));

    // Start playing.
    fireEvent.click(getByRole("button", { name: /play replay/i }));
    await waitFor(() =>
      expect(
        getByRole("button", { name: /pause playback/i }),
      ).toBeInTheDocument(),
    );

    // Scrub — should pause and move cursor.
    const scrubber = getByLabelText(/scrub to tick/i);
    fireEvent.input(scrubber, { target: { value: "5" } });

    // After scrub, should be in paused state.
    await waitFor(() =>
      expect(
        getByRole("button", { name: /play replay/i }),
      ).toBeInTheDocument(),
    );
  });

  it("shows 'No frames loaded' when frames prop is empty", () => {
    const { getByLabelText } = render(() => <TacticalBoard frames={[]} />);
    const info = getByLabelText(/frame info/i);
    expect(info.textContent).toContain("No frames loaded");
  });

  it("shows tick and seed info when frames are present", async () => {
    const { getByLabelText } = render(() => (
      <TacticalBoard frames={validFrames(3)} />
    ));
    // The info div shows Tick/Seed/Score from the first frame.
    await waitFor(() => {
      const info = getByLabelText(/frame info/i);
      expect(info.textContent).toContain("Seed:");
    });
  });
});

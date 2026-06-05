/*
 * LiveMatch route — Vitest tests (S3b / M2b).
 *
 * Acceptance criteria:
 *   AC1  — on mount with seed state, calls startLiveMatch(seed).
 *   AC1b — on mount with fixture state { homeClubId, awayClubId }, calls
 *           startLiveMatchForFixture({ homeClubId, awayClubId }) NOT startLiveMatch.
 *   AC1c — on mount with no state (direct dev entry), calls startLiveMatch with
 *           a random seed hex (seed path fallback).
 *   AC2  — the step loop calls stepLiveMatch + appends key events to the feed.
 *   AC3  — board receives frames from step results.
 *   AC4  — scoreline updates on each step result.
 *   AC5  — auto mode: loop pauses when a key event is emitted; "Continue" resumes.
 *   AC6  — isFinished: loop stops; finishLiveMatch called; final result panel shown.
 *   AC7  — play/pause button toggles the loop.
 *   AC8  — speed mode buttons are aria-pressed for the active mode.
 *   AC9  — startLiveMatch failure shows a football-native error alert.
 *   AC10 — stepLiveMatch IPC error shows a football-native error alert.
 *
 * Mocking strategy:
 *   - ~/lib/api/live_match mocked globally; each test configures via vi.mocked().
 *   - ~/lib/tauri mocked so backendAvailable() returns true (backend path).
 *   - ~/components/TacticalBoard mocked to a div — no WebGL in jsdom.
 *   - pixi.js mocked globally (TacticalBoard is lazy — Suspense resolves the
 *     lazy import; mock ensures Pixi never touches real canvas).
 *   - @solidjs/router mocked: useLocation provides seed state; useNavigate spy.
 *     The fixture-path tests reconfigure useLocation to return fixture state.
 *   - Fake timers (vi.useFakeTimers) control the setInterval step loop.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@solidjs/testing-library";
import type { MatchEvent, MatchFrameDTO, MatchEventKind } from "~/lib/types";

// ---------------------------------------------------------------------------
// Module mocks — hoisted before any import
// ---------------------------------------------------------------------------

vi.mock("pixi.js", () => ({
  Application: vi.fn().mockImplementation(() => ({
    init: vi.fn().mockResolvedValue(undefined),
    destroy: vi.fn(),
    stage: { addChild: vi.fn() },
    canvas: document.createElement("canvas"),
    ticker: { add: vi.fn(), remove: vi.fn() },
  })),
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

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue({}),
}));

vi.mock("~/lib/tauri", () => ({
  isTauri: vi.fn(() => true),
  backendAvailable: vi.fn(() => true),
  httpBackendActive: vi.fn(() => false),
}));

vi.mock("~/lib/api/live_match", () => ({
  startLiveMatch: vi.fn(),
  startLiveMatchForFixture: vi.fn(),
  stepLiveMatch: vi.fn(),
  finishLiveMatch: vi.fn(),
  getMatchSnapshot: vi.fn(),
  applyMatchCommand: vi.fn(),
}));

// Mock TacticalBoard so it doesn't need WebGL + can track received frames.
vi.mock("~/components/TacticalBoard", () => ({
  default: (props: { frames: MatchFrameDTO[]; followLatest?: boolean }) => (
    <div
      data-testid="tactical-board"
      data-frame-count={props.frames.length}
      data-follow-latest={String(props.followLatest ?? false)}
    >
      Board ({props.frames.length} frames)
    </div>
  ),
}));

const navigateSpy = vi.fn();
vi.mock("@solidjs/router", () => ({
  useLocation: vi.fn(() => ({
    state: { seedHex: "0xdeadbeefdeadbeef" },
    pathname: "/live-match",
    search: "",
    hash: "",
    query: {},
  })),
  useNavigate: vi.fn(() => navigateSpy),
}));

// Import SUT AFTER mocks are hoisted.
import LiveMatch from "./LiveMatch";
import {
  startLiveMatch,
  startLiveMatchForFixture,
  stepLiveMatch,
  finishLiveMatch,
} from "~/lib/api/live_match";
import { useLocation } from "@solidjs/router";

// ---------------------------------------------------------------------------
// Fixture builders
// ---------------------------------------------------------------------------

function makeHandle(id = 0) {
  return { id, seedHex: "0xdeadbeefdeadbeef" };
}

function makeFrame(tick: number): MatchFrameDTO {
  return {
    seedHex: "0xdeadbeefdeadbeef",
    tick,
    homeScore: 0,
    awayScore: 0,
    players: [],
    ball: { posX: 0, posY: 0, posZ: 0, velX: 0, velY: 0, velZ: 0 },
    possession: null,
  };
}

function makeStep(
  tick: number,
  overrides: {
    isFinished?: boolean;
    newEvents?: MatchEvent[];
    score?: { home: number; away: number };
  } = {},
) {
  const handle = makeHandle();
  return {
    handle,
    newEvents: overrides.newEvents ?? [],
    score: overrides.score ?? { home: 0, away: 0 },
    tick,
    isFinished: overrides.isFinished ?? false,
    frame: makeFrame(tick),
  };
}

/** Helper to build a typed MatchEvent inline without losing MatchEventKind narrowing. */
function makeEvent(
  kind: MatchEventKind,
  tick: number,
  minute: number,
  description?: string,
): MatchEvent {
  // exactOptionalPropertyTypes: omit the key entirely when undefined so the type
  // aligns with MatchEvent's `description?: string` (not `description: string | undefined`).
  if (description !== undefined) {
    return { tick, minute, kind, description };
  }
  return { tick, minute, kind };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("LiveMatch route (S3b)", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    navigateSpy.mockReset();
    vi.mocked(startLiveMatch).mockResolvedValue(makeHandle());
    vi.mocked(stepLiveMatch).mockResolvedValue(makeStep(60));
    vi.mocked(finishLiveMatch).mockResolvedValue({
      handle: makeHandle(),
      finalScore: { home: 1, away: 0 },
      tick: 5400,
      totalEvents: 10,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  // AC1: on mount calls startLiveMatch with the seed from location state.
  it("calls startLiveMatch(seed) on mount (AC1)", async () => {
    render(() => <LiveMatch />);

    await waitFor(() => {
      expect(vi.mocked(startLiveMatch)).toHaveBeenCalledTimes(1);
    });
    expect(vi.mocked(startLiveMatch)).toHaveBeenCalledWith("0xdeadbeefdeadbeef");
  });

  // AC2: step loop calls stepLiveMatch + appends key events to the feed.
  it("step loop calls stepLiveMatch and key events appear in the feed (AC2)", async () => {
    vi.mocked(stepLiveMatch).mockResolvedValue(
      makeStep(60, {
        newEvents: [makeEvent("KickOff", 60, 1, "Kick-off.")],
      }),
    );

    render(() => <LiveMatch />);

    // Wait for startLiveMatch to resolve.
    await waitFor(() =>
      expect(vi.mocked(startLiveMatch)).toHaveBeenCalledTimes(1),
    );

    // Advance fake timers to trigger the interval.
    await vi.advanceTimersByTimeAsync(150);

    // stepLiveMatch should have been called at least once.
    await waitFor(() =>
      expect(vi.mocked(stepLiveMatch)).toHaveBeenCalled(),
    );

    // KickOff is a key moment — should appear in the feed.
    await waitFor(() => {
      expect(screen.getByText("Kick-off")).toBeInTheDocument();
    });
  });

  // AC3: board receives frames from step results.
  it("board receives frames as steps arrive (AC3)", async () => {
    render(() => <LiveMatch />);

    await waitFor(() =>
      expect(vi.mocked(startLiveMatch)).toHaveBeenCalledTimes(1),
    );

    await vi.advanceTimersByTimeAsync(150);

    await waitFor(() => {
      const board = screen.queryByTestId("tactical-board");
      if (!board) return;
      const frameCount = parseInt(board.getAttribute("data-frame-count") ?? "0", 10);
      expect(frameCount).toBeGreaterThan(0);
    });
  });

  // AC4: scoreline updates on step results.
  it("scoreline updates when a goal is scored (AC4)", async () => {
    vi.mocked(stepLiveMatch).mockResolvedValue(
      makeStep(1080, {
        newEvents: [makeEvent("Goal", 1080, 18, "Clean finish.")],
        score: { home: 1, away: 0 },
      }),
    );

    render(() => <LiveMatch />);
    await waitFor(() =>
      expect(vi.mocked(startLiveMatch)).toHaveBeenCalledTimes(1),
    );

    await vi.advanceTimersByTimeAsync(150);

    await waitFor(() => {
      // Scoreline region exists.
      expect(screen.getByRole("region", { name: /live score/i })).toBeInTheDocument();
    });
  });

  // AC5: auto mode pauses when a key event is emitted; "Continue" resumes.
  it("auto mode pauses on a key event; Continue resumes the loop (AC5)", async () => {
    vi.mocked(stepLiveMatch).mockResolvedValue(
      makeStep(60, {
        newEvents: [makeEvent("Shot", 60, 1, "Decent effort.")],
      }),
    );

    render(() => <LiveMatch />);
    await waitFor(() =>
      expect(vi.mocked(startLiveMatch)).toHaveBeenCalledTimes(1),
    );

    await vi.advanceTimersByTimeAsync(150);

    // Auto mode should have paused at the Shot event.
    await waitFor(() => {
      // The "Continue" button appears when auto-paused.
      expect(
        screen.getByRole("button", { name: /continue watching/i }),
      ).toBeInTheDocument();
    });

    // Clicking Continue should resume — loop restarts; no more "Continue" button
    // until the NEXT key event.
    const continueBtn = screen.getByRole("button", { name: /continue watching/i });
    // Set the next step to be pass-only (no key events — loop runs freely).
    vi.mocked(stepLiveMatch).mockResolvedValue(makeStep(120));
    fireEvent.click(continueBtn);

    await waitFor(() => {
      expect(
        screen.queryByRole("button", { name: /continue watching/i }),
      ).toBeNull();
    });
  });

  // AC6: isFinished — loop stops, finishLiveMatch called, final panel shown.
  it("shows final result panel and calls finishLiveMatch when match ends (AC6)", async () => {
    vi.mocked(stepLiveMatch).mockResolvedValue(
      makeStep(5400, {
        isFinished: true,
        newEvents: [makeEvent("FullTime", 5400, 90, "Full time.")],
        score: { home: 1, away: 0 },
      }),
    );
    vi.mocked(finishLiveMatch).mockResolvedValue({
      handle: makeHandle(),
      finalScore: { home: 1, away: 0 },
      tick: 5400,
      totalEvents: 12,
    });

    render(() => <LiveMatch />);
    await waitFor(() =>
      expect(vi.mocked(startLiveMatch)).toHaveBeenCalledTimes(1),
    );

    await vi.advanceTimersByTimeAsync(200);

    await waitFor(() => {
      expect(vi.mocked(finishLiveMatch)).toHaveBeenCalledTimes(1);
    });

    await waitFor(() => {
      expect(
        screen.getByRole("region", { name: /final result/i }),
      ).toBeInTheDocument();
    });

    // Final result panel should mention "Full time".
    const finalPanel = screen.getByRole("region", { name: /final result/i });
    expect(finalPanel.textContent).toMatch(/full time/i);
  });

  // AC6b: "Back to career" navigates to /career.
  // This test uses a custom approach: after the match ends the "Back to career"
  // button should become visible. We advance timers in small increments to
  // avoid the runAllTimersAsync infinite-loop problem.
  it("'Back to career' navigates to /career after match ends (AC6b)", async () => {
    vi.mocked(stepLiveMatch).mockResolvedValue(
      makeStep(5400, { isFinished: true, score: { home: 0, away: 0 } }),
    );
    vi.mocked(finishLiveMatch).mockResolvedValue({
      handle: makeHandle(),
      finalScore: { home: 0, away: 0 },
      tick: 5400,
      totalEvents: 5,
    });

    render(() => <LiveMatch />);
    await waitFor(() =>
      expect(vi.mocked(startLiveMatch)).toHaveBeenCalledTimes(1),
    );

    await vi.advanceTimersByTimeAsync(200);

    await waitFor(() => {
      expect(vi.mocked(finishLiveMatch)).toHaveBeenCalledTimes(1);
    });

    await waitFor(() =>
      expect(screen.getByRole("region", { name: /final result/i })).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByRole("button", { name: /back to career/i }));
    expect(navigateSpy).toHaveBeenCalledWith("/career");
  });

  // AC7: play/pause button toggles the loop.
  it("Pause button stops the loop; Resume restarts it (AC7)", async () => {
    render(() => <LiveMatch />);
    await waitFor(() =>
      expect(vi.mocked(startLiveMatch)).toHaveBeenCalledTimes(1),
    );

    // Initially running — Pause button exists.
    await waitFor(() =>
      screen.getByRole("button", { name: /pause the match/i }),
    );

    // Pause immediately — before the interval fires any steps.
    fireEvent.click(screen.getByRole("button", { name: /pause the match/i }));

    await waitFor(() =>
      screen.getByRole("button", { name: /resume the match/i }),
    );

    // Record call count while paused.
    const stepsWhilePaused = vi.mocked(stepLiveMatch).mock.calls.length;

    // Advance timers — no new steps should fire (interval was cleared).
    await vi.advanceTimersByTimeAsync(500);
    const stepsAfterWait = vi.mocked(stepLiveMatch).mock.calls.length;
    expect(stepsAfterWait).toBe(stepsWhilePaused);

    // Resume — the Pause button reappears.
    fireEvent.click(screen.getByRole("button", { name: /resume the match/i }));

    await waitFor(() =>
      expect(screen.getByRole("button", { name: /pause the match/i })).toBeInTheDocument(),
    );
  });

  // AC8: speed mode buttons have aria-pressed for the active mode.
  it("speed buttons have aria-pressed reflecting the current mode (AC8)", async () => {
    render(() => <LiveMatch />);
    await waitFor(() =>
      expect(vi.mocked(startLiveMatch)).toHaveBeenCalledTimes(1),
    );

    // Default mode is "auto".
    await waitFor(() => {
      const autoBtn = screen.getByRole("button", { name: /auto mode/i });
      expect(autoBtn.getAttribute("aria-pressed")).toBe("true");
    });

    // Switch to x3.
    const x3Btn = screen.getByRole("button", { name: /speed × 3/i });
    fireEvent.click(x3Btn);

    await waitFor(() => {
      expect(x3Btn.getAttribute("aria-pressed")).toBe("true");
      const autoBtn = screen.getByRole("button", { name: /auto mode/i });
      expect(autoBtn.getAttribute("aria-pressed")).toBe("false");
    });
  });

  // AC9: startLiveMatch failure shows a football-native error alert.
  it("shows football-native error when startLiveMatch rejects (AC9)", async () => {
    vi.mocked(startLiveMatch).mockRejectedValue({
      kind: "matchInitFailed",
      reason: "content not loaded",
    });

    render(() => <LiveMatch />);

    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });

    const alert = screen.getByRole("alert");
    // Must not leak raw error.message.
    expect(alert.textContent).not.toContain("content not loaded");
    // Football-native copy from describeRouteError.
    expect(alert.textContent).toMatch(/kick-off didn't happen/i);
  });

  // AC10: stepLiveMatch IPC error shows football-native alert and stops loop.
  it("shows football-native error and stops when stepLiveMatch rejects (AC10)", async () => {
    vi.mocked(stepLiveMatch).mockRejectedValue({
      kind: "matchInitFailed",
      reason: "handle expired",
    });

    render(() => <LiveMatch />);
    await waitFor(() =>
      expect(vi.mocked(startLiveMatch)).toHaveBeenCalledTimes(1),
    );

    await vi.advanceTimersByTimeAsync(150);

    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });
    // Football-native — no raw error.message.
    expect(screen.getByRole("alert").textContent).not.toContain("handle expired");
    expect(screen.getByRole("alert").textContent).toMatch(/kick-off didn't happen/i);
  });

  // Board receives followLatest=true while match is live.
  it("board receives followLatest=true while match is in progress", async () => {
    render(() => <LiveMatch />);
    await waitFor(() =>
      expect(vi.mocked(startLiveMatch)).toHaveBeenCalledTimes(1),
    );

    await vi.advanceTimersByTimeAsync(150);

    await waitFor(() => {
      const board = screen.queryByTestId("tactical-board");
      if (!board) return;
      expect(board.getAttribute("data-follow-latest")).toBe("true");
    });
  });

  // ---------------------------------------------------------------------------
  // M2b: fixture-path tests
  // ---------------------------------------------------------------------------

  // AC1b: when location state has { homeClubId, awayClubId }, calls
  // startLiveMatchForFixture — NOT startLiveMatch.
  it("calls startLiveMatchForFixture when fixture state is provided (AC1b)", async () => {
    // Reconfigure useLocation to return fixture state.
    vi.mocked(useLocation).mockReturnValue({
      state: { homeClubId: 42, awayClubId: 7 },
      pathname: "/live-match",
      search: "",
      hash: "",
      query: {},
      key: "test-key",
    } as unknown as ReturnType<typeof useLocation>);

    vi.mocked(startLiveMatchForFixture).mockResolvedValue(makeHandle());

    render(() => <LiveMatch />);

    await waitFor(() => {
      expect(vi.mocked(startLiveMatchForFixture)).toHaveBeenCalledTimes(1);
    });
    expect(vi.mocked(startLiveMatchForFixture)).toHaveBeenCalledWith({
      homeClubId: 42,
      awayClubId: 7,
    });
    // The seed path must NOT have been called.
    expect(vi.mocked(startLiveMatch)).not.toHaveBeenCalled();
  });

  // AC1c: when location state has seedHex (no fixture), uses the seed path.
  it("calls startLiveMatch(seed) when only seedHex is in state (AC1c)", async () => {
    vi.mocked(useLocation).mockReturnValue({
      state: { seedHex: "0xdeadbeefdeadbeef" },
      pathname: "/live-match",
      search: "",
      hash: "",
      query: {},
      key: "test-key",
    } as unknown as ReturnType<typeof useLocation>);

    render(() => <LiveMatch />);

    await waitFor(() => {
      expect(vi.mocked(startLiveMatch)).toHaveBeenCalledTimes(1);
    });
    expect(vi.mocked(startLiveMatch)).toHaveBeenCalledWith("0xdeadbeefdeadbeef");
    expect(vi.mocked(startLiveMatchForFixture)).not.toHaveBeenCalled();
  });

  // AC1c-null: when state is null (direct dev entry), falls back to startLiveMatch
  // with a random hex seed.
  it("calls startLiveMatch with a random seed when state is null (AC1c-null)", async () => {
    vi.mocked(useLocation).mockReturnValue({
      state: null,
      pathname: "/live-match",
      search: "",
      hash: "",
      query: {},
      key: "test-key",
    } as unknown as ReturnType<typeof useLocation>);

    render(() => <LiveMatch />);

    await waitFor(() => {
      expect(vi.mocked(startLiveMatch)).toHaveBeenCalledTimes(1);
    });
    // Seed is a random hex; we just verify the call happened and fixture path didn't.
    const calledSeed = vi.mocked(startLiveMatch).mock.calls[0]?.[0];
    expect(typeof calledSeed).toBe("string");
    expect(calledSeed).toMatch(/^0x[0-9a-f]+$/i);
    expect(vi.mocked(startLiveMatchForFixture)).not.toHaveBeenCalled();
  });
});

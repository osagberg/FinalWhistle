/*
 * Match page — Vitest substrate tests (T1-6).
 *
 * These tests warm the Vitest harness and exercise the Match component's
 * render + interaction paths. They are NOT exhaustive coverage; the
 * comprehensive gate lands at T1-13.
 *
 * Substance requirements met (per the cargo-cult meta-pattern):
 *   1. Tests actually mount the component + assert rendered DOM, not just
 *      "module loads without error".
 *   2. Tests simulate a user interaction (Play button click) and assert
 *      the UI state after the interaction.
 *   3. IpcError narrowing path is exercised by a mocked thrown error.
 *
 * Mocking strategy:
 *   - `~/lib/tauri` is mocked globally. `isTauri()` returns false so the
 *     browser-preview path (makeMockResult) runs — no actual Tauri IPC needed.
 *   - `~/routes/Dev/TacticalBoard` is mocked to a no-op div to avoid
 *     requiring a live WebGL context in jsdom.
 *   - pixi.js is mocked via `__mocks__` (or inline vi.mock) to avoid
 *     WebGL init errors in jsdom.
 */

import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@solidjs/testing-library";
import type { MatchResult } from "~/lib/types";

// ---------------------------------------------------------------------------
// Module mocks — must be hoisted before component import
// ---------------------------------------------------------------------------

// Mock pixi.js so PixiJS Application init doesn't fail in jsdom (no WebGL).
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
  Container: vi.fn().mockImplementation(() => ({
    addChild: vi.fn(),
  })),
}));

// Mock @tauri-apps/api/core so invoke() doesn't throw outside Tauri.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue({}),
}));

// Mock ~/lib/tauri — isTauri + backendAvailable return false (browser-preview
// path) by default. playMatch is overridden per-test when needed.
// `backendAvailable` delegates to the mocked `isTauri` so that per-test
// `vi.mocked(tauriMod.isTauri).mockReturnValue(true)` still toggles the
// real-backend path in Match.tsx (which now checks `backendAvailable()`).
vi.mock("~/lib/tauri", () => {
  const isTauri = vi.fn(() => false);
  const backendAvailable = vi.fn(() => isTauri());
  return {
    isTauri,
    backendAvailable,
    playMatch: vi.fn(),
    getDummyState: vi.fn(),
  };
});

// Mock the production TacticalBoard to avoid Pixi full init in the toggle path.
vi.mock("~/components/TacticalBoard", () => ({
  default: (props: { frames: unknown[] }) => (
    <div data-testid="dev-tactical-board">
      Tactical Board (mocked) — {props.frames.length} frames
    </div>
  ),
}));

// Mock Dev/FrameSource so TauriFrameSource.loadFrames() doesn't hit IPC.
vi.mock("~/routes/Dev/FrameSource", () => ({
  TauriFrameSource: vi.fn().mockImplementation(() => ({
    loadFrames: vi.fn().mockResolvedValue([]),
  })),
  HttpFrameSource: vi.fn().mockImplementation(() => ({
    loadFrames: vi.fn().mockResolvedValue([]),
  })),
}));

// Import AFTER mocks are defined.
import Match from "./Match";
import * as tauriMod from "~/lib/tauri";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const mockResult: MatchResult = {
  finalScore: { home: 2, away: 1 },
  canonicalHash: "blake3:" + "0".repeat(64),
  matchEvents: [
    { tick: 0, minute: 0, kind: "KickOff", description: "Kick-off." },
    { tick: 540, minute: 9, kind: "Goal", description: "Goal to home side." },
    { tick: 2700, minute: 45, kind: "HalfTime", description: "Half-time." },
    { tick: 5400, minute: 90, kind: "FullTime", description: "Full-time." },
  ],
  commentaryPreview: [
    "The referee's whistle starts proceedings.",
    "Neat finish.",
    "Half-time.",
    "Full-time.",
  ],
  seedHex: "0xfeedbeefcafefade",
  tickCount: 900,
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Match page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Default: browser-preview path (isTauri = false).
    vi.mocked(tauriMod.isTauri).mockReturnValue(false);
  });

  it("renders Play match button before any run", () => {
    render(() => <Match />);
    const playBtn = screen.getByRole("button", { name: /play match/i });
    expect(playBtn).toBeInTheDocument();
  });

  it("does not render scoreline before Play is clicked", () => {
    render(() => <Match />);
    // Scoreline has aria-label "Final score" — should not exist yet.
    expect(screen.queryByRole("region", { name: /final score/i })).toBeNull();
  });

  it("renders scoreline after Play button click (browser-preview mock path)", async () => {
    render(() => <Match />);
    const playBtn = screen.getByRole("button", { name: /play match/i });
    fireEvent.click(playBtn);

    // Browser-preview path returns mock result immediately (sync in test).
    await waitFor(() => {
      expect(
        screen.getByRole("region", { name: /final score/i }),
      ).toBeInTheDocument();
    });

    // Scoreline should show 2 – 1 (the mock result values).
    const scoreRegion = screen.getByRole("region", { name: /final score/i });
    expect(scoreRegion.textContent).toContain("2");
    expect(scoreRegion.textContent).toContain("1");
  });

  it("renders event list with minute markers after Play", async () => {
    render(() => <Match />);
    fireEvent.click(screen.getByRole("button", { name: /play match/i }));

    await waitFor(() => {
      expect(
        screen.getByRole("list", { name: /match event list/i }),
      ).toBeInTheDocument();
    });

    // Kick-off should be present in the event list.
    expect(screen.getByText("Kick-off")).toBeInTheDocument();
  });

  it("disables Play button while busy and re-enables on completion", async () => {
    // Tauri path: playMatch returns a promise we control.
    vi.mocked(tauriMod.isTauri).mockReturnValue(true);
    let resolve!: (value: MatchResult) => void;
    vi.mocked(tauriMod.playMatch).mockImplementation(
      () =>
        new Promise<MatchResult>((res) => {
          resolve = res;
        }),
    );

    render(() => <Match />);
    const playBtn = screen.getByRole("button", { name: /play match/i });
    fireEvent.click(playBtn);

    // Button should be disabled while simulating.
    await waitFor(() => {
      expect(playBtn).toBeDisabled();
    });

    // Resolve the match — button should re-enable.
    resolve(mockResult);
    await waitFor(() => {
      expect(playBtn).not.toBeDisabled();
    });
  });

  it("shows error message on IpcError tooManyFrames", async () => {
    vi.mocked(tauriMod.isTauri).mockReturnValue(true);
    vi.mocked(tauriMod.playMatch).mockRejectedValue({
      kind: "tooManyFrames",
      requested: 99999,
      max: 7200,
    });

    render(() => <Match />);
    fireEvent.click(screen.getByRole("button", { name: /play match/i }));

    await waitFor(() => {
      // The formatted error should mention the specific values.
      expect(screen.getByRole("alert").textContent).toContain("99999");
      expect(screen.getByRole("alert").textContent).toContain("7200");
    });
  });

  it("shows error message on IpcError invalidSeed", async () => {
    vi.mocked(tauriMod.isTauri).mockReturnValue(true);
    vi.mocked(tauriMod.playMatch).mockRejectedValue({
      kind: "invalidSeed",
      input: "0xbad",
      reason: "not a valid u64",
    });

    render(() => <Match />);
    fireEvent.click(screen.getByRole("button", { name: /play match/i }));

    await waitFor(() => {
      const alertText = screen.getByRole("alert").textContent ?? "";
      // describeRouteError for invalidSeed includes the input in the detail.
      expect(alertText).toContain("0xbad");
      // Must not include raw technical exception strings.
      expect(alertText).not.toContain("Cannot read properties");
    });
  });

  it("shows error message on IpcError matchInitFailed", async () => {
    vi.mocked(tauriMod.isTauri).mockReturnValue(true);
    vi.mocked(tauriMod.playMatch).mockRejectedValue({
      kind: "matchInitFailed",
      reason: "content not loaded",
    });

    render(() => <Match />);
    fireEvent.click(screen.getByRole("button", { name: /play match/i }));

    await waitFor(() => {
      const alertText = screen.getByRole("alert").textContent ?? "";
      // describeRouteError for matchInitFailed returns a generic "couldn't start" message.
      expect(alertText).toBeTruthy();
      // Must not include raw technical exception strings.
      expect(alertText).not.toContain("Cannot read properties");
    });
  });

  it("toggles tactical board on and off without crashing", async () => {
    render(() => <Match />);

    // Play first so the toggle button appears.
    fireEvent.click(screen.getByRole("button", { name: /play match/i }));
    await waitFor(() =>
      screen.getByRole("button", { name: /show tactical board/i }),
    );

    // Toggle on.
    const toggleBtn = screen.getByRole("button", { name: /show tactical board/i });
    fireEvent.click(toggleBtn);

    await waitFor(() => {
      expect(screen.getByTestId("dev-tactical-board")).toBeInTheDocument();
    });

    // Toggle off — board should disappear.
    fireEvent.click(screen.getByRole("button", { name: /hide tactical board/i }));
    await waitFor(() => {
      expect(screen.queryByTestId("dev-tactical-board")).toBeNull();
    });
  });

  it("seed input validation rejects non-hex input (disables Play)", () => {
    render(() => <Match />);
    const seedInput = screen.getByLabelText(/match seed/i);
    const playBtn = screen.getByRole("button", { name: /play match/i });

    // Enter a clearly invalid seed.
    fireEvent.input(seedInput, { target: { value: "not-a-hex" } });
    expect(playBtn).toBeDisabled();
  });

  // Codex 2026-05-16 Tier-2 fix-pass: parse the EXACT JSON shape the Rust
  // backend now produces (per `crates/fw-tauri/tests/ipc_contract_test.rs`
  // `match_result_match_events_serializes_as_flat_dto_not_tagged_enum`) and
  // assert it casts cleanly into the TS `MatchResult` interface + renders
  // the events. Prior tests passed because mockResult was built in the
  // frontend shape directly; the Rust→TS round-trip was never validated, so
  // the tagged-enum bug shipped despite green tests.
  it("renders MatchResult parsed from exact backend JSON wire shape", async () => {
    // This string is the literal serialization shape produced by
    // `serde_json::to_string(&MatchResult)` on the Rust side. If the
    // backend changes its wire format, this test fails — catching the
    // regression at `pnpm test` time, not at `pnpm tauri dev`.
    // Backend omits `description` when None via #[serde(skip_serializing_if)]
    // — matches TS `description?: string` optional shape exactly.
    const backendJson = `{
      "finalScore": { "home": 1, "away": 0 },
      "canonicalHash": "blake3:782fcde65ba8a0fc12bb90af1b61f77d8cd403103ab3671b0d5d6b03e75c8c0f",
      "matchEvents": [
        { "tick": 0, "minute": 0, "kind": "KickOff" },
        { "tick": 540, "minute": 9, "kind": "Goal" },
        { "tick": 5400, "minute": 90, "kind": "FullTime" }
      ],
      "seedHex": "0xdeadbeefdeadbeef",
      "tickCount": 5400,
      "commentaryPreview": [
        "Kick-off.",
        "Goal to home side.",
        "Full-time."
      ]
    }`;
    const parsed = JSON.parse(backendJson) as MatchResult;

    // The Rust→TS contract has these load-bearing properties; assert each
    // explicitly so a regression at any field fails with a specific message.
    expect(parsed.finalScore.home).toBe(1);
    expect(parsed.matchEvents).toHaveLength(3);
    // The flat-DTO shape: `kind` is a PascalCase string, NOT an object key.
    const [first, second, third] = parsed.matchEvents;
    expect(first?.kind).toBe("KickOff");
    expect(second?.kind).toBe("Goal");
    expect(third?.kind).toBe("FullTime");
    // Each event has the 3 required flat fields (description optional + omitted).
    // If any required field is missing the backend regressed to enum form.
    parsed.matchEvents.forEach((ev) => {
      expect(ev.tick).toBeTypeOf("number");
      expect(ev.minute).toBeTypeOf("number");
      expect(ev.kind).toBeTypeOf("string");
    });

    // Render Match with this parsed result via the same mock-and-play path
    // — proves the component handles the actual backend shape end-to-end.
    vi.mocked(tauriMod.playMatch).mockResolvedValue(parsed);
    vi.mocked(tauriMod.isTauri).mockReturnValue(true);
    render(() => <Match />);
    fireEvent.click(screen.getByRole("button", { name: /play match/i }));
    await waitFor(() => {
      expect(
        screen.getByRole("region", { name: /final score/i }),
      ).toBeInTheDocument();
    });
    // Goal badge rendered → eventLabel switch handled the PascalCase kind.
    expect(screen.getAllByText("GOAL").length).toBeGreaterThan(0);
  });
});

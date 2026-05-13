# ADR-0008 — Browser-dev mode for the dev-tier 2D tactical board

**Status:** Proposed
**Date:** 2026-05-13
**Decider:** Claude (synthesis from user direction) + Codex (pending audit at next phase gate)

---

## Context

T1-2a delivers a dev-tier 2D tactical board (ADR-0007 Layer 2; `MASTER_PLAN.md` T1-2a row) that renders 22 dots + ball + tick scrubber on a top-down pitch. The component lives at `frontend/src/routes/Dev/TacticalBoard.tsx` and (per ADR-0004) consumes a `MatchFrameDTO` stream over Tauri events from the live sim. Always-on for the developer running `pnpm tauri dev`.

That design makes the board verifiable for a human watching the Tauri window. It is **not** verifiable for Claude as a development collaborator. Claude works through file Read/Edit, Bash, and MCP servers — no access to the Tauri runtime, no way to see the rendered output. The "is this football?" verification problem (the central reason Layer 2 exists, per `docs/design/dev-verification.md`) stays a human-eyeball-only loop.

The newly-added Claude Preview MCP (`mcp__Claude_Preview__preview_*` tools — `preview_start`, `preview_screenshot`, `preview_eval`, `preview_console_logs`, etc.) lets Claude drive a headless Chrome instance against any HTTP URL and capture screenshots. The catch: Tauri's `invoke()` IPC only resolves inside the Tauri runtime. The Vite dev server at `localhost:1420` serves the SolidJS UI fine, but any frame-source code path that calls `tauri::invoke()` fails in a plain browser.

To unlock Claude-eyeball verification of the 2D board, the component needs a second frame source that works without a Tauri runtime.

## Decision

`TacticalBoard.tsx` reads its frame stream through a `FrameSource` trait with two concrete impls, selected at mount time by URL parameter:

```
/dev/board                                  →  TauriFrameSource (default; production + Tauri dev)
/dev/board?source=fixture:/path/to.json     →  HttpFrameSource (browser-dev; static JSON fixture)
/dev/board?source=http://localhost:N/...    →  HttpFrameSource (browser-dev; live HTTP endpoint)
```

The TS surface (sketch):

```typescript
interface FrameSource {
  start(): Promise<void>;
  onFrame(cb: (frame: MatchFrameDTO) => void): void;
  seek(tick: number): Promise<void>;
  stop(): void;
}

class TauriFrameSource implements FrameSource { /* listen('match:frame', ...) */ }
class HttpFrameSource implements FrameSource { /* fetch JSON, index by tick */ }

function pickSource(): FrameSource {
  const url = new URLSearchParams(window.location.search);
  const source = url.get("source");
  if (!source) return new TauriFrameSource();
  return new HttpFrameSource(source);
}
```

The fixture format is a plain JSON array of `MatchFrameDTO` records — the same shape Tauri emits, deterministically dumped. Produced by a new Rust binary at `crates/fw-match-sim/src/bin/dump_frames.rs`:

```sh
cargo run --bin dump_frames -- --seed 0xdeadbeef --ticks 600 > /tmp/smoke-match.json
```

The binary loads the same `MatchState::initial(seed)` + `tick_match` pipeline the live sim uses, accumulates `MatchFrameDTO` per tick (via the same projection function fw-tauri uses), and writes the result to stdout as JSON. No new sim path — same code, different I/O.

The Claude-driven workflow:

```sh
# Terminal 1: start Vite dev server
pnpm --filter ./frontend dev

# Terminal 2: produce a fixture
cargo run --bin dump_frames -- --seed 0xdeadbeef --ticks 600 > /tmp/smoke.json

# In Claude (with Claude Preview MCP):
preview_start http://localhost:1420/dev/board?source=fixture:/tmp/smoke.json
preview_screenshot                        # tick 0 (kickoff layout)
preview_eval "window.fwDev.scrubTo(180)"  # T+3 minutes
preview_screenshot                        # tick 180
preview_eval "window.fwDev.scrubTo(540)"  # T+9 minutes
preview_screenshot                        # tick 540
```

The component exposes a small `window.fwDev` debug object in dev builds for `preview_eval` to drive the scrubber and read state. The object is gated behind `import.meta.env.DEV` so it never ships.

## Consequences

**Positive:**
- **Claude-runnable visual verification.** The "is this football?" gate (currently the most-cited risk in `docs/postmortems/phase-T0.md`) can be checked by Claude without human-in-the-loop. T1-2b iteration speed becomes AI-paced, not human-paced.
- **Faster frontend dev cycle.** No Tauri rebuild between frontend changes (Vite HMR < 100ms vs Tauri rebuild 5-30s).
- **Single rendering surface.** The same `TacticalBoard.tsx` handles live (Tauri), replay (Tauri replaying canonical state), and fixture (browser, deterministic). No parallel renderers.
- **Visual regression catchable.** Same fixture + same git ref + same Vite build → bit-identical screenshot. Sets up CI screenshot-diff as a future T4 polish row.
- **Unblocks Claude Preview for any frontend route, not just the board.** Once `pnpm dev` is the standard dev workflow, the rest of the UI is screenshot-verifiable too.

**Negative:**
- **Small dual-source code path.** ~30-80 LoC across the `FrameSource` trait + URL param parse + `HttpFrameSource` impl. Code review will catch divergence; both impls share the same `MatchFrameDTO` shape.
- **One new Rust binary** (`dump_frames`). Trivial code (~50 LoC); just a thin wrapper over the existing tick loop with JSON output.
- **`window.fwDev` debug surface.** Must be `import.meta.env.DEV`-gated. Hook will catch leaks via the banned-terms / boundary-check pass.
- **Bypasses Tauri's capability layer for frames in dev mode.** Not a security concern (no real data on disk in dev), but worth flagging — capabilities still apply for everything else (filesystem, network beyond localhost).

**Neutral:**
- **ADR-0007 (dev verification) Layer 2 gains a sub-mechanism**, not a replacement. Tauri-IPC live mode stays. The fixture-source mode is the Claude-accessible variant.
- **Future T4 polish (shipped match-day surface)** uses the same component with `TauriFrameSource`. Browser-dev mode does not affect the shipped game.

## Alternatives considered

- **`mcp__Claude_in_Chrome` MCP via the Chrome extension** — same end state, requires a one-time extension install. Claude Preview is lower-friction (no extension), picked on that basis.
- **Computer-use MCP driving the Tauri app directly** — works, but Mac-only, requires accessibility permissions, no headless mode, slow screenshot cadence. Browser route is 90% of the value at 10% of the friction.
- **Build a Playwright CI harness with screenshot-diff regression** — future T2 or T4 polish. Out of scope for T1; the manual Claude-Preview workflow is sufficient for iteration.
- **Skip Claude-visible verification entirely; trust commentary + proptest** — current ADR-0007 plan. Rejected: visual eyeball is the highest-signal "is this football?" test the project will ever have, and the marginal cost of making it Claude-runnable is small.
- **Auto-detect missing Tauri runtime at startup and silently fall back** — rejected because explicit URL-param mode-switching is debuggable; silent fallback hides which source produced a bug.

## References

- `docs/adr/0004-ipc-command-surface.md` — Tauri frame-streaming pattern (the production path this ADR adds a fallback to)
- `docs/adr/0007-dev-verification-surface.md` — Layer 2 (the 2D board this ADR extends)
- `docs/design/dev-verification.md` — the verification-surface design doc this ADR operationalizes
- `docs/MASTER_PLAN.md` T1-2a — row updated alongside this ADR
- `crates/fw-match-sim/src/bin/dump_frames.rs` (forthcoming, T1-2a deliverable)
- Claude Preview MCP: `mcp__Claude_Preview__preview_*` tool surface

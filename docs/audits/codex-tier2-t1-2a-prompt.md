# Codex Tier-2 audit prompt — T1-2a (dev-tier 2D tactical board)

**Tier-2 audit per ADR-0015 §"5 explicit criteria"** (criterion 4: API surface to UI — adds `match_frames` IPC command; new public type surface in fw-match-sim).

**Use this prompt verbatim** in a Codex CLI session. Read-only audit; Codex does not mutate code. Findings come back as a structured report; Claude applies P0/P1 fixes via `/next` cycles.

---

```
CODEX TIER-2 AUDIT — T1-2a (dev-tier 2D tactical board)

Repo: /Users/vibelogic/dev/football
Commit in scope: b4befc40 (HEAD).
Range to inspect: main..b4befc40 (so anything earlier is treated as
already-audited prior work and not re-reviewed).
v1 archive (read-only): /Users/vibelogic/dev/football-archive

Read-only audit. DO NOT mutate code or docs.

================================================================================
CONTEXT
================================================================================

Final Whistle is a procedural-fantasy football management sim in Rust +
Tauri + SolidJS. Phase T1 (First Match) active. T1-1 (fw-content schema
lock) shipped + audited; the full-project audit + pre-T1-2b re-audit
(passes #1, #2, #3) all cleared. T1-2a is the dev-tier 2D tactical
board — verification surface per ADR-0007 Layer 2 + ADR-0008 browser-
dev mode. It exists so we can SEE whether the sim is producing
football-shaped behavior (T1-2b-i lands ball physics next; T1-2b-iii
lands real BT-driven match data).

This Tier-2 audit fires because ADR-0015 §"5 explicit criteria"
criterion 4 ("API surface to UI — any IPC command added to fw-tauri")
is satisfied: T1-2a adds `match_frames`. Also bumps the public type
surface in fw-match-sim (`MatchFrameDto`/`PlayerFrameDto`/`BallFrameDto`
+ projection from `MatchState`).

What T1-2a shipped:

**Rust:**
- `crates/fw-match-sim/src/dto.rs` (NEW) — MatchFrameDto et al. with
  camelCase serde + Q32→f64 projection. Has
  `#![allow(clippy::float_arithmetic)]` (scoped to this module). The
  `scripts/determinism-audit.py` script has an explicit file
  exemption for this path.
- `crates/fw-match-sim/src/bin/dump_frames.rs` (NEW) — clap CLI
  binary. Bit-identical stdout across reruns verified.
- `crates/fw-tauri/src/commands.rs` — new `match_frames(seedHex,
  tickCount) -> Vec<MatchFrameDto>` IPC command.
- `crates/fw-tauri/src/lib.rs` — re-exports the DTO types from
  fw-match-sim. `MatchStateDto`/`PlayerDto`/`BallDto` gained
  `#[serde(rename_all = "camelCase")]` (Codex audit P0 fix on
  pre-existing rule violation).
- `crates/fw-match-sim/Cargo.toml` — added serde_json + clap deps.
- `src-tauri/src/main.rs` — registers `fw_tauri::commands::match_frames`.

**TypeScript:**
- `frontend/src/routes/Dev/FrameSource.ts` (NEW) — `FrameSource`
  interface + `TauriFrameSource` + `HttpFrameSource` + fail-loud
  `frameSourceFromUrlParams` factory + `isMatchFrameArray` runtime
  shape check.
- `frontend/src/routes/Dev/TacticalBoard.tsx` (NEW) — SolidJS +
  PixiJS component (pitch + 22 dots + ball + scrubber + info readout
  + DEV-only `window.fwDev` debug surface).
- `frontend/src/App.tsx` — adds `/dev/board` lazy route.
- `frontend/src/lib/types.ts` — TS mirror types.

**Other:**
- `scripts/determinism-audit.py` — file-level exemption for dto.rs.
- `.gitignore` — Rust binary-source carve-out (`!crates/*/src/bin/`).
- `frontend/public/dev-fixtures/.gitignore` — ephemeral fixtures.

Self-review pre-commit: silent-failure-hunter + type-design-analyzer +
feature-dev:code-reviewer all ran. 1 P0 (MatchStateDto camelCase rule
violation) + 4 P1 (HttpFrameSource shape validation, frameSource URL
guard, tick_count=0 contract pin, AbilityCeiling escape hatch) closed
in-place pre-commit. P2/P3 deferred per ADR-0015 / commit body.

================================================================================
AUDIT SCOPE — 5 FOCUSED LANES
================================================================================

LANE A — IPC contract correctness (`match_frames`)

A1. `match_frames(seed_hex: String, tick_count: u32) -> Vec<MatchFrameDto>`
    in fw-tauri/src/commands.rs. Read the signature + body. Length
    contract: `tick_count + 1`. Confirm:
    - Saturating add against `usize` overflow at extreme `tick_count`?
    - Deterministic across calls (same seed + ticks → same Vec)?
    - Error path for malformed `seedHex` (non-hex chars, length, etc.)
      surfaces a useful Tauri-side message?
    - Does the camelCase wire shape match the TS `MatchFrameDTO`
      interface in `frontend/src/lib/types.ts` exactly?
A2. The handler is `async fn` but has no `.await`. Tauri requires
    async fn for IPC commands; verify the handler body is genuinely
    sync (no hidden blocking call that should await).
A3. The 2 sync test wrappers (`match_frames_tick_count_zero_returns_one_frame`,
    `match_frames_returns_tick_count_plus_one_frames`) use
    `tauri::async_runtime::block_on`. Confirm this is the right
    primitive vs. `pollster` / hand-rolled noop-waker — and that
    Tauri's bundled runtime is available in test cfg.

LANE B — Float boundary in `fw-match-sim::dto`

B1. The DTO module has `#![allow(clippy::float_arithmetic)]` plus a
    file-level exemption in `scripts/determinism-audit.py`. Verify:
    - The `q32_to_f64` projection is the ONLY float arithmetic in
      the file.
    - No code path reads any f64 back into Q32 (one-way contract).
    - The `MatchState` reference is read-only.
    - The exemption in determinism-audit.py is correctly scoped to
      `crates/fw-match-sim/src/dto.rs` exactly (not the whole crate).
B2. The DTO lives in fw-match-sim BECAUSE the `dump_frames` binary
    can't depend on fw-tauri (would invert the dep graph). Is the
    re-export pattern in fw-tauri (`pub use fw_match_sim::...`)
    clean, or should `MatchFrameDto` move back to fw-tauri with a
    duplicated type for the binary? Read both files + audit the
    inverse direction.
B3. Two `q32_to_f64` functions exist now: one in
    `fw-match-sim::dto` (private) and one in `fw-tauri::lib.rs`
    (private). The code-reviewer flagged this as P2 deferred. Was
    that triage correct, or is there a hidden drift risk?

LANE C — Frontend SolidJS + PixiJS lifecycle

C1. `frontend/src/routes/Dev/TacticalBoard.tsx` creates a PixiJS
    `Application` in `onMount` + destroys in `onCleanup`. Per
    `Frontend/RULES.md §4`. Verify:
    - The Application is created ONCE (not in `createEffect` or any
      reactive context).
    - The `destroyed` race-condition guard handles the case where
      `onCleanup` fires before `onMount`'s `app.init()` resolves.
    - `createEffect` mutates dot/ball positions WITHOUT recreating
      sprites or destroying the Application.
    - No memory leaks under Solid's dev re-renders.
C2. The `FrameSource` interface vs the two impls. `TauriFrameSource`
    calls `invoke('match_frames', ...)`. `HttpFrameSource` calls
    `fetch(url)` + `isMatchFrameArray` validation. Verify error paths:
    - Fetch network failure → surfaces to UI?
    - 404 → surfaces?
    - Wrong-shape JSON → surfaces (the `isMatchFrameArray` check)?
    - Invoke failure (Tauri shell crashed mid-call) → surfaces?
C3. `frameSourceFromUrlParams` factory + `FrameSourceConfigError`
    custom error class. Verify it correctly throws on:
    - `?source=bogus` (unknown scheme)
    - `?source=fixture` (no colon, no path)
    - `?source=fixture:` (empty path)
    - `?source=fixture:javascript:...` (non-http(s)/non-relative)
    - `?source=Tauri` (capitalized) — does it fall through to
      TauriFrameSource correctly OR error?
C4. `window.fwDev` debug surface — DEV-only gate (`import.meta.env.DEV`)
    correct? Production builds genuinely don't expose it?

LANE D — Cross-cutting + project conventions

D1. Codex audit Lane E P2 (Frontend `@apply` clarification from
    earlier) — does the new TacticalBoard.tsx use any shared
    utility CSS that violates the rule? Tailwind utilities directly
    in className are correct; `@apply` in shared CSS is the
    anti-pattern.
D2. Codex audit Lane F P2 — does the new code introduce any
    `unwrap_or_default()` / `unwrap_or(...)` / `try/catch` patterns
    that silently swallow errors? (Beyond what the self-review
    already caught.)
D3. The `.gitignore` carve-out (`!crates/*/src/bin/`) — does it
    correctly preserve other crates' future `src/bin/` files while
    not over-broadening? Test pattern: would
    `crates/fw-tauri/src/bin/foo.rs` be tracked?
D4. The 2-line camelCase fix on `MatchStateDto`/`PlayerDto`/`BallDto`
    — does it actually flow through `play_match` + `get_dummy_state`
    correctly? Or does the existing `frontend/src/lib/types.ts`
    have manual field renames that now duplicate work?

LANE E — Determinism + canonical-state safety

E1. T1-2a does NOT modify `MatchState` itself — only adds a
    projection on top. Confirm by grepping for `MatchState` writes
    in the diff (`git diff main..b4befc40 -- crates/fw-match-sim/`).
    Should be zero outside `dto.rs` (read-only via `&MatchState`).
E2. Canonical hash UNCHANGED claim. Confirm by checking the pinned
    BLAKE3 + the canonical_hash test status. The hash is still
    `d6258107...d96b1a49` per STATUS.md.
E3. Tier-2 verdict on whether T1-2a is safe to leave in place
    BEFORE T1-2b-i begins. T1-2b-i (ball physics) WILL bump the
    canonical hash per ADR-0012 trigger #1. T1-2a should produce
    NO drift in the meantime — i.e. the dev-board is a pure
    overlay, not a sim modifier.

================================================================================
DELIVERABLE
================================================================================

Single Markdown report:

```markdown
# T1-2a Tier-2 Audit Report (Codex, 2026-MM-DD)

## Executive summary (≤200 words)
[Verdict: GREEN / YELLOW / RED. The 5 most important things.]

## Verdict per lane
[Lane A IPC: CLOSED / OPEN. Lane B float boundary: CLOSED / OPEN. Etc.]

## New findings (P0/P1 only)
[Full detail per finding: severity, file:line, description, root
cause, recommended fix shape.]

## Deferred P2/P3
[List with one-line each + recommended slot for fixing.]

## Tier-2 verdict
GO / NO-GO for T1-2b-i to start. State explicit pre-conditions.
```

Quality bar: brutal but specific. Every finding file:line-anchored
with recommended-fix shape on every P0/P1.

If T1-2a is genuinely clean, say so — don't manufacture findings.
This is a focused Tier-2 audit, not a full-project pass.

Begin.
```

---

## How to run

1. Open Codex CLI in `/Users/vibelogic/dev/football`.
2. Paste the prompt block above verbatim.
3. Codex returns the report.
4. Paste the report back to Claude. Claude triages → fixes P0/P1 via
   `/next` cycles or in-place follow-up commit → re-verifies → pushes.
5. T1-2b-i (ball physics) starts only after this re-audit clears OR
   any new findings are remediated.

Cost: ~$5–15 in Codex API spend; ~20 min of user time. Per ADR-0015.

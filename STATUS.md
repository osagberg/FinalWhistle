# STATUS — Final Whistle

**Last updated**: 2026-05-13

## Phase

**T1 — First Match** (active; T1-1 + T1-2a closed; next: T1-2b-i ball physics — first TDD-mandated row)

## Active task

(none — T1-2a closed. `/next` picks T1-2b-i.)

## Phase pointer

- **Just closed:** **T1-2a Dev-tier 2D tactical board.** ADR-0007 Layer 2 + ADR-0008 implemented. `MatchFrameDto` lives in `fw-match-sim::dto` (camelCase serde, exempted in determinism-audit). `match_frames` IPC command in fw-tauri. `dump_frames` CLI binary (bit-identical stdout). SolidJS `TacticalBoard.tsx` with PixiJS Application (lifecycle per Frontend/RULES.md §4). FrameSource trait + dual impls + fail-loud URL-param factory + JSON-shape validation. `window.fwDev` DEV-only debug surface for Claude Preview. E2E verified via Claude Preview MCP. 1 P0 + 4 P1 from self-review triple closed in-place.
- **Now:** Phase T1 critical path advances: T1-2a → **T1-2b-i (ball physics)** → T1-2b-ii (tactic FSM + cadence stagger) → T1-2b-iii (FSM-of-BTs + utility + PlayerSeparation) → T1-2b-iv (signature dispatcher). T1-2b-i is the **first row under the TDD mandate** (per `docs/DECISIONS.md` 2026-05-13 superpowers TDD entry — real behavior code in `fw-match-sim`).
- **Next:** `T1-2b-i` ball physics — semi-implicit Euler in Q32 (gravity, drag, Magnus, bounce, friction) per ADR-0001 §"7 layers" + v1 carry-forward design. Tier-2 Codex audit recommended pre-implementation per ADR-0015 §"5 explicit criteria" (criterion 1: schema lock — adds ball-state extensions to canonical `MatchState`; criterion 2: new canonical-state surface).

## Blockers

None. T1-2a left a clean board (figuratively and literally).

## Last green verify

2026-05-13 — `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo fmt --check` + frontend `pnpm typecheck`/`lint`/`build` + `scripts/fw verify` (banned-terms + determinism-audit + canonical-hash regression + verify-content) — all clean at T1-2a HEAD.

## Last canonical hash

`blake3:d6258107b2c90c84d2feeaa8633d1f5c159e10ccd2016623b52b41d3d96b1a49` (60-tick smoke seed; pinned T0-7; UNCHANGED through T1-2a — the dev board is a read-only projection of canonical state, doesn't mutate it). Re-baseline expected at T1-2b-ii (`decision_slots: [u8; 22]` + `interrupt_cooldown_until: [Tick; 22]` added to `MatchState` per ADR-0012 trigger #1).

## Recent commits

- `<this commit>` feat(ui,tauri,sim): T1-2a dev-tier 2D tactical board (ADR-0007 Layer 2 + ADR-0008)
- `e7807927` ci: drop --release from FW-VAL CI step; STATUS+MEMORY → re-audit green
- `5bb0939d` docs: close 3 P1s + 3 P2s from Codex re-audit pass #2
- `80c53f76` fix: re-audit pass #1 P1+P2 fixes (7 P1s closed)
- `af7df8fa` docs: close 7-tranche audit remediation + queue Codex re-audit

## Next up

`/next` will pick **T1-2b-i** — ball physics in `fw-match-sim::ball`. Semi-implicit Euler integration in Q32 (gravity, drag, Magnus, bounce, friction) ported from v1's `BallPhysics.cs` design (NOT code; Rust idioms only). The TDD mandate fires — RED-GREEN-REFACTOR per the `superpowers` plugin skill before each implementation chunk. Tier-2 Codex audit recommended before code lands (per ADR-0015) since this is the first canonical-state extension in T1.

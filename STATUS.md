# STATUS — Final Whistle

**Last updated**: 2026-05-13

## Phase

**T1 — First Match** (active; T1-1 closed)

## Active task

(none — awaiting next `/next`. Suggested: T1-2a dev-tier 2D tactical board.)

## Phase pointer

- **Just closed:** T1-1 — `fw-content` schema lock. 55-field player model (ADR-0002) in `fw-core::PlayerAttributes`; `AbilityCeiling` encapsulated with breakthrough-only PA mutator; `RoleId` newtype + `RoleAffinityTable` with collect-all validators; `TacticalArchetype.buildup_speed_factor` `f32 → u16 bps` (Codex Imp #3 from T0); `schema_version: 1` on new content types; first RON fixtures under `content/sources/players/` + `content/sources/role-affinities/`. Canonical hash UNCHANGED. Self-review triple twice → Accept.
- **Now:** Phase T1 continues. Critical path: T1-2a → T1-2b → T1-4 → T1-5 → T1-6 (T1-9 lands after T1-2b).
- **Next:** T1-2a — dev-tier 2D tactical board (per ADR-0007 + ADR-0008). Browser-dev mode + Claude Preview MCP wiring confirmed working.

## Blockers

None.

## Last green verify

2026-05-13 — `scripts/fw verify` green at HEAD: `cargo fmt --check` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo test --workspace` + `banned-terms` + `determinism-audit` + canonical-hash regression all clean.

## Last canonical hash

`blake3:d6258107b2c90c84d2feeaa8633d1f5c159e10ccd2016623b52b41d3d96b1a49` (60-tick smoke seed; pinned T0-7; UNCHANGED through T1-1 — `MatchState` does not yet reference `PlayerAttributes`).

## Recent commits

- `pending` feat(content): T1-1 fw-content schema lock — ADR-0002 55-field player model + Codex Imp #3
- `20314655` chore(docs): ADR-0008 browser-dev mode for the 2D tactical board + TDD mandate
- `7e64b477` fix(docs): Codex consistency-check fixes across ADRs + DESIGN_DOC + MASTER_PLAN + research notes
- `42637855` docs(adr): land 7 ADRs + DESIGN_DOC §11 architecture overview + DECISIONS.md log
- `bad1a400` chore(ci): cache-bin: false on Swatinem invocations (preempt flake)

## Next up

`/next` will pick **T1-2a** — dev-tier 2D tactical board. Frontend `TacticalBoard.tsx` consumes `MatchFrameDTO` via a `FrameSource` trait with `TauriFrameSource` (default) + `HttpFrameSource` (browser-dev) impls; `crates/fw-match-sim/src/bin/dump_frames.rs` produces deterministic fixture JSON; `window.fwDev` debug surface for Claude Preview `preview_eval`. Acceptance gate covers Tauri + browser-dev paths end-to-end. Per ADR-0007 Layer 2.

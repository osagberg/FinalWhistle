# Final Whistle — Working Memory

> Updated: 2026-05-13 | Phase: T0 Scaffold

## Project

Procedural fantasy football management sim. Rust + Tauri 2 + SolidJS. Solo dev + Claude.
Pivoted from Unity + C# v1 (preserved at git tag `v0-pre-pivot-2026-05-13` and sibling `/Users/vibelogic/dev/football-archive/`).

## Module status

| Module | State | Key file | Notes |
|---|---|---|---|
| `fw-core` | Scaffolded | `crates/fw-core/src/q32.rs` | Q32, Seed, Tick, IDs. Q32 unit tests pass locally per Wave-2 agent. NOT YET RUN on CI matrix. |
| `fw-match-sim` | Stub | `crates/fw-match-sim/src/lib.rs` | 22-player struct + tick() reducer (no-op tick). Float-deny clippy active. |
| `fw-content` | Schema-only | `crates/fw-content/src/lib.rs` | TeamTemplate, PlayerTemplate, ArchetypeTemplate types. No baked corpus yet. |
| `fw-content-baker` | CLI stub | `crates/fw-content-baker/src/main.rs` | clap CLI, prompt templates in src/prompts/, NOT yet wired to Claude API. |
| `fw-scouting` | Empty | `crates/fw-scouting/src/lib.rs` | Compiles, no types. |
| `fw-memory` | Empty | `crates/fw-memory/src/lib.rs` | Compiles, MemoryEvent enum stub. |
| `fw-replay` | Scaffolded | `crates/fw-replay/tests/canonical_hash.rs` | Phase-0 acceptance test. Pinned hash is placeholder; fills on first CI green pass. |
| `fw-save` | Empty | `crates/fw-save/src/lib.rs` | Compiles, SaveV1 enum stub. |
| `fw-tauri` | Stub | `crates/fw-tauri/src/lib.rs` | Five `#[tauri::command]` stubs returning placeholder data. |
| `src-tauri` | Scaffolded | `src-tauri/src/main.rs` | Tauri shell wires fw-tauri commands. |
| `frontend` | Scaffolded | `frontend/src/App.tsx` | SolidJS + Tailwind + 6 placeholder routes. |

## Recent work

- 2026-05-13: Pivot from Unity + C# to Rust + Tauri 2 + SolidJS. Single commit. 109 scaffold files.

## Current task

None active. Next via `/next` (suggested: `/done` to open the T0 phase-gate PR for Codex review).

## Recently completed

- 2026-05-13 — T0-12 Fix pre-existing scaffold build failures — fw-tauri commands moved to sibling module (known Tauri 2 `pub` + `#[tauri::command]` bug); fw-content-baker `#![allow(dead_code)]` on staging modules; src-tauri build.rs stubs frontend/dist for clean-clone `cargo build`; tauri icons generated (gitignored); ui-vocabulary.md meta-references wrapped in sentinels. `cargo test --workspace --release` 19 test-runs all green.
- 2026-05-13 — T0-7 Pin BLAKE3 canonical hash on dev box — `d6258107…` pinned; `cargo test -p fw-replay` 4/4 green; cross-OS matrix → T0-7b.

## Recently completed

- 2026-05-13 — T0-7 Pin BLAKE3 canonical hash on dev box — `d6258107…` pinned; `cargo test -p fw-replay` 4/4 green; cross-OS matrix → T0-7b.

## Active decisions

- Q32.32 (`fixed::FixedI64<U32>`) for all canonical-state quantities. `#[deny(clippy::float_arithmetic)]` on sim crates.
- BLAKE3 (not SHA-256) for canonical-state hashing.
- RON files for content sources + replay fixtures (human-diffable).
- Bincode 2 for save format.
- `ChaCha8Rng` for all sim randomness. `thread_rng` / `StdRng` banned in sim code.
- `BTreeMap` / `IndexMap` only in canonical-state-emitting code. `HashMap` banned in sim crates.
- Tauri 2 + SolidJS + Tailwind v3 + TanStack Table v8 + PixiJS v8 + ECharts.
- Single primary workflow command: `/next` (see `.claude/skills/next/SKILL.md`).
- Codex review at phase-gates via PR (not per-task).
- Per-task self-review via `pr-review-toolkit` subagents on ≥100 LoC code changes.

## Open questions

(See `docs/DESIGN_DOC.md` §12 for the gameplay design open questions.)

Technical open questions for T0 follow-up:

1. Pinned hash placeholder — fill on first CI green pass (T0-7).
2. Apple-codesign secrets — defer until T5-1 release pipeline.
3. Tailwind v3 vs v4 — picked v3 for May 2026 stability; revisit at T4.

## Next up (after T0 scaffold lands)

Per `docs/MASTER_PLAN.md`:
- T0-1 through T0-11 — all currently TODO. Critical path: T0-1 → T0-3 → T0-4 → T0-5 → T0-6 → T0-7.
- Acceptance gate for T0 phase: pinned canonical SHA on macOS/Win/Linux CI + Tauri opens.

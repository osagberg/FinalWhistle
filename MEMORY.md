# Final Whistle — Working Memory

> Updated: 2026-05-13 | Phase: T0 Scaffold

## Project

Procedural fantasy football management sim. Rust + Tauri 2 + SolidJS. Solo dev + Claude.
Pivoted from Unity + C# v1 (preserved at git tag `v0-pre-pivot-2026-05-13` and sibling `/Users/vibelogic/dev/football-archive/`).

## Module status (post-T0)

| Module | State | Key file | Notes |
|---|---|---|---|
| `fw-core` | Scaffolded + audited | `crates/fw-core/src/q32.rs` | Q32 with panic-on-overflow operators (Codex Q1). Durable u32 IDs (Codex Q2). Seed + Tick + cordic sqrt. Tested on CI matrix; deterministic across macOS-14 + Win + Linux. |
| `fw-match-sim` | Stub | `crates/fw-match-sim/src/lib.rs` | 22-player struct + no-op tick reducer. Hand-rolled little-endian canonical encoder (FWMS magic + version). `assert_eq!` slot-order invariant (Codex Imp #11). Float-deny clippy. T1-2 fills behavior. |
| `fw-content` | Schema-only | `crates/fw-content/src/lib.rs` | TeamTemplate + PlayerTemplate + ArchetypeTemplate types. `CultureWeights` u16 basis points (Codex Crit #3). T1-1 fills first RON fixtures. NOTE: `TacticalArchetype.buildup_speed_factor: f32` deferred Codex Imp #3 → T1-1. |
| `fw-content-baker` | CLI stub | `crates/fw-content-baker/src/main.rs` | clap CLI; prompt + schema + validator modules `#![allow(dead_code)]`-staged (T2-3+). Wires to Claude API at T2-3. |
| `fw-scouting` | Empty | `crates/fw-scouting/src/lib.rs` | Compiles, no types. T3-5 begins. |
| `fw-memory` | Empty | `crates/fw-memory/src/lib.rs` | Compiles, MemoryEvent enum stub. `stakes` + `salience` Q32 (Codex Crit #3 doc fix). T3-1 fills the ledger. |
| `fw-replay` | Acceptance test live | `crates/fw-replay/tests/canonical_hash.rs` | Phase-0 acceptance test ACTIVE. Pinned BLAKE3 `d6258107…` verified cross-OS. 4 tests run (1 still ignored — insta baseline pending T1). |
| `fw-save` | Empty | `crates/fw-save/src/lib.rs` | Compiles, SaveV1 enum stub. T2-9 begins. bincode 1 vs 2 alignment (Codex Imp #12) → T2-9. |
| `fw-tauri` | Stub | `crates/fw-tauri/src/lib.rs` + `commands.rs` | Two `#[tauri::command]` handlers in sibling module (Tauri 2 `pub` bug workaround). MatchStateDto projects Q32→f64 (read-only). T1-5 wires real surface. |
| `src-tauri` | Scaffolded | `src-tauri/src/main.rs` + `build.rs` | Tauri shell. build.rs stubs frontend/dist on clean clones. Tracked icon stubs. Local placeholder commands shadow fw-tauri (Codex Imp #10 → T1-5 consolidation). |
| `frontend` | Scaffolded + green | `frontend/src/main.tsx` | SolidJS + Tailwind v3 + 6 placeholder routes. typecheck + lint + build green across CI matrix. `<For />` over `.map()` (lint pass). |

## Recent work

- 2026-05-13: **Phase T0 closed.** Pivot (109 files) + blueprint reconciliation (51 files) + Codex audit (14 of 16 findings landed) + canonical hash pinned + CI matrix green + Codex APPROVE. 12 commits in session. Cross-OS BLAKE3 verified. See `docs/postmortems/phase-T0.md`.

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

## Next up — Phase T1 — First Match

Per `docs/MASTER_PLAN.md`:
- T1-1 `fw-content` schema + first RON fixtures (also: Codex Imp #3 buildup_speed_factor f32 → u16 bps conversion)
- T1-2 ball physics + 22-player BT runner (XL — biggest row; consider splitting at task-spec time)
- T1-3 signatures stub (types only)
- T1-4 MatchEvent enum + ledger output
- T1-5 fw-tauri play_match command (also: Codex Imp #10 — src-tauri command consolidation)
- T1-6 frontend Match route + text recap
- T1-7 procedural content stub (Markov names + 2 teams)
- T1-8 replay corpus fixture #1 (smoke seed, 600 ticks, two-archetype matchup)

Acceptance gate for T1: two procedural teams play one match end-to-end; text recap surfaces with goals + score + key events; ≥2 replay-corpus fixtures pin across CI matrix.

Critical path: T1-1 → T1-2 → T1-4 → T1-5 → T1-6.

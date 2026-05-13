# Changelog

Append-only human-readable ship log. One line per shipped MASTER_PLAN task. Phase-summary block on `/done`. Reverse-chronological within each phase section.

---

## Phase T0: Scaffold — APPROVED + MERGED 2026-05-13

**Codex verdict 2026-05-13:** APPROVE at `4721fee6` + hardening `bad1a400`. All workflow runs green on macOS-14 + windows-latest + ubuntu-22.04. T0-7b cross-OS canonical-hash agreement verified. Postmortem at `docs/postmortems/phase-T0.md`.

Additional commits post-`/done`:
- 2026-05-13 — **CI matrix unblock + Codex pre-merge audit** (`89479063`, `a0b2e084`, `a612e585`, `4721fee6`, `bad1a400`) — 5 commits closing the gap between dev-box green and CI matrix green. MSRV bump (fixed@1.31 → rustc 1.95); workflow path fixes; icon-tracking; Linux Tauri deps; SKILL.md atomic ordering; frontend strict-typecheck cleanup; Swatinem bin-cache hardening.

---

## Phase T0: Scaffold — closed 2026-05-13 (preliminary; superseded by APPROVED block above)

**Shipped:** an empty repo became a workspace-verifying Rust+Tauri+SolidJS scaffold with deterministic match-sim primitives, a pinned BLAKE3 canonical-state hash, a reconciled blueprint, a 9-step `/next` workflow, and Codex pre-T0 audit findings landed.

- **Crates (8 + Tauri shell)**: `fw-core` (Q32.32 panic-on-overflow + Seed + Tick + durable u32 IDs), `fw-match-sim` (22-player struct + canonical encoder + tick reducer), `fw-content` (schema + integer-bps weights), `fw-content-baker` (CLI scaffold with staged prompts/schemas/validators), `fw-scouting` (stub), `fw-memory` (MemoryEvent stub with Q32 stakes/salience), `fw-replay` (pinned-hash regression test + RON fixture + sanity-non-zero guard + corpus-agreement test), `fw-save` (SaveV1 enum stub), `fw-tauri` (DTOs + IPC handlers in sibling commands module), `src-tauri` (Tauri shell + frontend/dist stub + icon stubs).
- **Frontend**: Vite + SolidJS + Tailwind v3 + TanStack v8 + PixiJS v8 + ECharts scaffolded with 6 placeholder routes.
- **Workflow**: 7-agent roster, 6 slash commands, 5 path-scoped rule modules, 5 hooks, 3 context scopes, 8 design templates, 9 patterns.
- **Determinism gate**: pinned BLAKE3 = `d6258107b2c90c84d2feeaa8633d1f5c159e10ccd2016623b52b41d3d96b1a49` on the smoke seed at tick 60. Bit-identical across 100 fresh runs intra-process on macOS-14. Cross-OS matrix agreement (T0-7b) verified by the phase-gate PR's CI.
- **Codex pre-T0 audit**: 14 of 16 findings fixed in-tree; 2 intentionally deferred (src-tauri command consolidation → T1-5; bincode dev-dep alignment → T2-9). 7 of 7 open questions resolved.
- **Canonical-state hash:** `blake3:d6258107…` (60-tick smoke seed).
- **Tests:** 19 test-runs across 13 crates + integration; all green on macOS-14 dev box. Cross-OS verification pending T0-7b.
- **Decisions logged:** 0 in `docs/DECISIONS.md` this phase (architectural decisions were made via STATUS open-questions resolution + Codex audit follow-ups, all documented in the commit bodies). First DECISIONS entry expected at T1 (BT-runner contract).

### Commit chain in T0

- `81fdeff` — Rust pivot scaffold (109 files; clean-slate rewrite from Unity+C#)
- `bc5c683` — pivot cleanup (drop Unity AI session leftovers)
- `26f1ba0` — blueprint reconciliation (51 files; 14→7 agents, 35→6 commands, 14→5 hooks)
- `7dc510d` — Codex audit followup quick wins (9 of 16; STATUS bootstrap; sanity hash test)
- `9eb184e` — Codex Q1+Q2 (Q32 panic-on-overflow + durable u32 IDs)
- `239594e` — T0-7 pin BLAKE3 canonical hash on macOS-14 dev box
- `73c1da0` — T0-12 unblock workspace verify (Tauri lib.rs `pub` bug + 4 other scaffold gaps)

### Per-task summary

- 2026-05-13 — **T0-12** Fix pre-existing scaffold build failures — fw-tauri commands moved to sibling module (Tauri 2 `pub` + `#[command]` bug, ref tauri #4665); fw-content-baker dead-code suppressed at staging-module roots (T2-3+ wiring deferred); src-tauri build.rs stubs frontend/dist + icons generated; ui-vocabulary.md meta-references wrapped in sentinels. `cargo {build,clippy,test,fmt} --workspace` ALL green.
- 2026-05-13 — **T0-7** Pin BLAKE3 canonical hash on macOS-14 dev box — `PINNED_60_TICK = d6258107…` recorded; `#[ignore]` removed; `cargo test -p fw-replay` 4/4 green. Cross-OS matrix verification deferred to T0-7b via the phase-gate PR.
- 2026-05-13 — **Codex audit followup** (`9eb184e`) — Q32 operators panic-on-overflow; IDs are durable `u32` newtypes (slotmap dropped). 7 of 7 Q1–Q7 audit decisions resolved.
- 2026-05-13 — **Codex audit quick wins** (`7dc510d`) — STATUS.md created; hash-non-zero sanity test; DESIGN_DOC float fields → Q32 basis points; fw-content weights → integer bps; determinism-audit script (comment-aware); 9 of 16 Codex findings fixed.
- 2026-05-13 — **Blueprint reconciliation** (`26f1ba0`) — 51 files; 7 agents (down from 14), 6 commands (down from 35), 5 hooks (down from 14), 3 context scopes (down from 5), Rust-flavored path-scoped RULES, 8 design templates, 9 patterns.
- 2026-05-13 — **Rust pivot scaffold** (`81fdeff`) — Clean-slate rewrite from Unity+C# to Rust+Tauri+SolidJS. 109 scaffold files across 8 crates + frontend + Tauri shell. Pre-pivot state preserved at git tag `v0-pre-pivot-2026-05-13` + sibling `/Users/vibelogic/dev/football-archive/`.

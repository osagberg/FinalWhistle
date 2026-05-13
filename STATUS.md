# STATUS — Final Whistle

**Last updated**: 2026-05-13

## Phase

**T0 — Scaffold** (in progress)

## Active task

None — awaiting first `/next` invocation.

## Phase progress (T0)

| ID | Status | Notes |
|---|---|---|
| T0-1 Cargo workspace skeleton | DONE | 8 crates compile clean (commit 81fdeff). Crate names: `fw-core`, `fw-match-sim`, `fw-content`, `fw-content-baker`, `fw-memory`, `fw-replay`, `fw-save`, `fw-scouting`, `fw-tauri`. NOTE: MASTER_PLAN T0-1 row says "fw-cli" — actual name is `fw-content-baker`. |
| T0-2 Tauri 2 + SolidJS shell | DONE | Shell with 6 placeholder routes lands at 81fdeff. |
| T0-3 fw-core Q32/Seed/Tick/IDs | DONE | Q32 newtype with checked_* arithmetic; per-Codex audit, bare operators wrap in release mode — see deferred decision below. |
| T0-4 fw-match-sim stub | DONE | 22-player struct + tick reducer (no behavior). |
| T0-5 Canonical encoder + BLAKE3 hash | DONE | Hand-rolled little-endian encoder in `crates/fw-match-sim/src/canonical.rs`. Uses BLAKE3 (MASTER_PLAN earlier referenced SHA-256 — that's stale; doc-fixed). |
| T0-6 Pinned hash + insta snapshot | PARTIAL | Test exists but pinned hash is `[0u8; 32]` placeholder, gated by `#[ignore]`. Sanity test added (`smoke_seed_canonical_hash_is_nonzero`) so the placeholder can't silently pass on real state. Filling the real hash is T0-7. |
| T0-7 GitHub Actions matrix CI green | TODO | Critical-path next task. Workflow files exist; first green run will surface the pinned BLAKE3 hash. |
| T0-8 Justfile / scripts/fw | DONE | Justfile + bash front-door at 81fdeff. Reconciliation added `banned-terms` + `verify-content` + `determinism-audit` recipes. |
| T0-9 /next slash command + hooks | DONE | Full blueprint reconciled at 26f1ba0. 6 commands, 7 agents, 5 hooks, path-scoped rules. |
| T0-10 DECISIONS.md + protect hook | DONE | Hook live; append-only enforced. |
| T0-11 README + REFERENCES | DONE | Both committed at 81fdeff; REFERENCES.md doc-fixed at this commit (15→7 agents). |

## Blockers

None. T0-7 is the critical path before T1.

## Last green verify

Not yet run end-to-end on this branch — `scripts/fw verify` will be exercised on the first `/next` cycle. Local sub-checks confirmed during reconciliation: banned-terms lint clean; structural file presence verified.

## Last canonical hash

Placeholder `[0u8; 32]`. Real hash pinned at T0-7 (first CI green pass).

## Recent commits

- `26f1ba0` blueprint: slim + adapt the framework for Rust+Tauri+SolidJS
- `bc5c683` pivot: drop leftover Unity AI Assistant session files
- `81fdeff` pivot: Rust + Tauri 2 + SolidJS rewrite — clean-slate scaffold
- `1d3a58b` polish(sim): Round-3 #4 defensive line height — FINAL C# commit (tag `v0-pre-pivot-2026-05-13`)

## Open decisions queued for user

After Codex pre-T0 audit (see `docs/CODEX_AUDIT_2026-05-13.md` if landed, or inline review):

1. **Q32 operators**: bare `+ - * /` wrap silently in release. Two options — remove the operator impls from canonical crates (force `checked_*`), or add `#[deny(clippy::arithmetic_side_effects)]` to sim crates.
2. **PlayerId / ClubId / MatchId** are currently slotmap keys. Memory + save need durable IDs that survive serialization. Proposed split: `PlayerId(u32)` durable + `PlayerSlot` runtime slotmap handle.
3. **src-tauri/main.rs** has local placeholder commands shadowing `fw-tauri`'s real surface. T1-5 was the planned consolidation; pull forward to T0?
4. **fw-core dev tests use bincode 1**; the save format uses bincode 2. Align now or defer to T2-9 (save crate work).
5. **content/baked/** is gitignored. Track once baker stabilizes (T2-3) or leave gitignored?
6. **/next docs sync** is in the same commit as code (current state). Keep, or separate?
7. **Canonical-hash rebaseline** authorization: task-spec flag (current) or require a `/log-decision` entry?

## Next up

`/next` will pick **T0-7** (CI matrix green + canonical hash pinned).

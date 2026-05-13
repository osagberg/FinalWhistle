# STATUS — Final Whistle

**Last updated**: 2026-05-13

## Phase

**T0 — Scaffold** (in progress)

## Active task

None — awaiting next `/next` invocation. Suggested: `T0-12` (fix pre-existing fw-tauri + fw-content-baker scaffold build failures discovered during T0-7).

## Phase progress (T0)

| ID | Status | Notes |
|---|---|---|
| T0-1 Cargo workspace skeleton | DONE | 8 crates compile clean (commit 81fdeff). Crate names: `fw-core`, `fw-match-sim`, `fw-content`, `fw-content-baker`, `fw-memory`, `fw-replay`, `fw-save`, `fw-scouting`, `fw-tauri`. NOTE: MASTER_PLAN T0-1 row says "fw-cli" — actual name is `fw-content-baker`. |
| T0-2 Tauri 2 + SolidJS shell | DONE | Shell with 6 placeholder routes lands at 81fdeff. |
| T0-3 fw-core Q32/Seed/Tick/IDs | DONE | Q32 newtype with checked_* arithmetic; bare operators panic-on-overflow (post-audit Q1=B refinement). IDs are u32 newtypes (durable, save→load round-trip). PlayerSlot=u8 runtime handle lives in fw-match-sim. |
| T0-4 fw-match-sim stub | DONE | 22-player struct + tick reducer (no behavior). |
| T0-5 Canonical encoder + BLAKE3 hash | DONE | Hand-rolled little-endian encoder in `crates/fw-match-sim/src/canonical.rs`. Uses BLAKE3 (MASTER_PLAN earlier referenced SHA-256 — that's stale; doc-fixed). |
| T0-6 Canonical-hash regression test wiring | DONE | Test + fixture + 4-test surface live in `crates/fw-replay`. Sanity test (`smoke_seed_canonical_hash_is_nonzero`) prevents the all-zero footgun. Reframed as test-wiring-only per Codex audit; the actual pinning was T0-7. |
| T0-7 Pin BLAKE3 canonical hash on dev box | DONE | `PINNED_60_TICK = d6258107b2c90c84d2feeaa8633d1f5c159e10ccd2016623b52b41d3d96b1a49`. `cargo test --release -p fw-replay`: 4 passed / 1 ignored (insta baseline, separate work) / 0 failed. RON fixture + in-code constant agree. `#[ignore]` removed from `smoke_seed_60_tick_canonical_hash_pinned`. |
| T0-7b Cross-OS canonical-hash matrix agreement | TODO | Phase-gate task. Open the T0 review PR via `/done`; CI matrix `[macos-14, windows-latest, ubuntu-22.04]` must produce the same BLAKE3 digest. Drift on any platform = real determinism leak, debug before merge. |
| T0-12 Fix pre-existing fw-tauri + fw-content-baker scaffold build failures | TODO | Discovered during T0-7 verify. `fw-tauri` fails `cargo build` with 5 E0255 errors (duplicate `__cmd__*` items from `#[tauri::command]` expansion — likely missing Cargo feature flag). `fw-content-baker` fails clippy `-D warnings` with 14 dead-code errors (constants + validators authored ahead of T2-3 wiring). Cleanest fix: 1-line Cargo.toml feature + `#![allow(dead_code)]` at module roots. Unblocks `cargo test --workspace` so `/next` can use full-workspace verify from here on. |
| T0-8 Justfile / scripts/fw | DONE | Justfile + bash front-door at 81fdeff. Reconciliation added `banned-terms` + `verify-content` + `determinism-audit` recipes. |
| T0-9 /next slash command + hooks | DONE | Full blueprint reconciled at 26f1ba0. 6 commands, 7 agents, 5 hooks, path-scoped rules. |
| T0-10 DECISIONS.md + protect hook | DONE | Hook live; append-only enforced. |
| T0-11 README + REFERENCES | DONE | Both committed at 81fdeff; REFERENCES.md doc-fixed at this commit (15→7 agents). |

## Blockers

None for T0-7 (DONE). T0-12 is the critical path before T1 (workspace verify currently red on pre-existing scaffold debt).

## Last green verify

Not yet run end-to-end on this branch — `scripts/fw verify` will be exercised on the first `/next` cycle. Local sub-checks confirmed during reconciliation: banned-terms lint clean; structural file presence verified.

## Last canonical hash

`blake3:d6258107b2c90c84d2feeaa8633d1f5c159e10ccd2016623b52b41d3d96b1a49` (macOS-14 dev box, T0-7, 2026-05-13). Cross-OS matrix agreement pending T0-7b at phase-gate.

## Recent commits

- `26f1ba0` blueprint: slim + adapt the framework for Rust+Tauri+SolidJS
- `bc5c683` pivot: drop leftover Unity AI Assistant session files
- `81fdeff` pivot: Rust + Tauri 2 + SolidJS rewrite — clean-slate scaffold
- `1d3a58b` polish(sim): Round-3 #4 defensive line height — FINAL C# commit (tag `v0-pre-pivot-2026-05-13`)

## Codex audit followups — resolved

1. **Q32 operators** (Codex Crit #4): **RESOLVED** — bare operators now panic on overflow via `checked_*().expect()` (commit pending). Silent wrap unreachable in canonical paths.
2. **PlayerId / ClubId / MatchId** (Codex Imp #5 + Open Q A): **RESOLVED** — now `u32` newtypes (durable across save→load). Runtime `PlayerSlot = u8` already lived separately in `fw-match-sim`; the split is now real.
3. **src-tauri/main.rs** (Codex Imp #10): **DEFERRED** to T1-5 per the in-file comment. Documented as intentional.
4. **bincode 1 vs 2** in `fw-core` dev tests (Codex Imp #12): **DEFERRED** to T2-9. New bincode-2 round-trip test added alongside the existing one then.
5. **content/baked/** tracking (Open Q B): **DEFERRED** to T2-3. Stays gitignored until baker stabilizes.
6. **/next docs sync** (Open Q C): **KEPT** in the same commit as code (atomic; resolved in SKILL.md Step 7/8 ordering fix).
7. **Canonical-hash rebaseline** (Open Q D): **POLICY** — task-spec flag suffices; commit body cites the new BLAKE3 short. `/log-decision` reserved for canonical-encoding **format** bumps (a much rarer event).

## Next up

`/next` will pick **T0-12** (fw-tauri + fw-content-baker scaffold fixes). After T0-12 lands, run `/done` to open the T0 phase-gate PR (which exercises T0-7b cross-OS matrix verification).

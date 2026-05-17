# STATUS — Final Whistle

**Last updated**: 2026-05-18

## Phase

**T2 in progress. T2-5 (`fw-tauri` season-controller IPC commands) closed 2026-05-18.** Selected ahead of T2-4 (PlayerBio generation) because T2-4 references `design/player-generation.md` which doesn't exist — substantive content/balance design call (22-field gene model + phenotype-label catalog) requires user input via `/log-decision` or design-doc authoring. T2 progress: 7 of 10 MVP rows DONE (T2-1a/b/c/d-infra + T2-1-codex-fix + T2-2 + T2-3 + T2-5). T2-1d2 DEFERRED to end-of-T2; T2-4 BLOCKED on design-doc gap.

T2-5 ships 4 new Tauri IPC commands (`advance_week`, `play_fixtures`, `get_standings`, `get_fixtures(club_id)`) backed by NEW `SeasonState` + `MatchOutcome` + `Standings` types in `fw-content/league.rs`, NEW `fw-tauri::season::play_one_match` orchestration (wraps existing `tick_match` loop without touching sim crates), and AppState extended with `RwLock<SeasonState>` + `career_seed`. Per-fixture seed derivation via `SeedLayer::ContentBake` site=1. 41 new tests (22 fw-content unit + 18 fw-tauri integration + 1 ignored perf gate; full season 380 matches in 0.364s vs 30s budget). Self-review triple verdicts: silent-failure-hunter REVISE (1 P0 + 4 P1 + 1 P1-edge → all 5 fixed in-place + 1 doc-only deferred); type-design REVISE (1 P0 + 1 P1 + 4 P2 → P0+P1 fixed; P2 deferred); code-reviewer REVISE (1 P0 + 2 P1 → all 3 fixed including TS-union-expansion cascade into routes/Match.tsx). **Canonical hashes UNCHANGED on both pins** — bin/IPC-crate work only; fw-match-sim public surface untouched.

## Active task

(none — T2-5 closed at this commit; `scripts/fw verify` exit 0; **canonical hashes UNCHANGED on both pins**. Next `/next` picks **T2-6** (Frontend: League page — TanStack Table standings) per declared order + skip-DEFERRED rule + skip-blocked-on-missing-design-doc rule applied to T2-4. **Deferred/blocked**: T1-17, T1-25..T1-28, T2-1d2, T4-9 all DEFERRED; T2-4 BLOCKED awaiting `design/player-generation.md` authorship via `/log-decision`. 13 commits ahead of origin/main waiting to push.)

## Phase pointer

- **Just landed:** **T2-5** — `crates/fw-content/src/league.rs` (~+209 LoC: SeasonState/MatchOutcome/StandingsRow/Standings types + impls). `crates/fw-tauri/src/state.rs` (AppState gains `RwLock<SeasonState>` + `career_seed: Seed`). NEW `crates/fw-tauri/src/season.rs` (`play_one_match` orchestrator + `SEASON_MATCH_TICK_BUDGET = 600`). `crates/fw-tauri/src/commands.rs` (+~220 LoC for 4 commands + their `_inner` helpers + transactional rollback). 4 new DTOs in `crates/fw-tauri/src/lib.rs`. New `IpcError::LockPoisoned` variant + ClubNotFound restructured to named-field. `src-tauri/src/main.rs` registers 4 new commands. `frontend/src/lib/types.ts` adds 4 new DTO mirrors + 3 new IpcError union variants. NEW `frontend/src/lib/api/season.ts` invoke wrappers. NEW `crates/fw-content/tests/season_state_test.rs` (22 tests) + NEW `crates/fw-tauri/tests/season_commands_test.rs` (18 + 1 perf). Cascade fix in `frontend/src/routes/Match.tsx` exhaustive IpcError switch + KNOWN_IPC_ERROR_KINDS set (TS exhaustiveness check broke from in-scope union expansion — minimal cross-file scope expansion).
- **Next:** **T2-6** — Frontend: League page (standings table with TanStack Table v8; sortable columns P / W / D / L / GF / GA / GD / Pts; dark + light mode). Deps T2-5 (DONE). Likely a `ui-programmer` task class. **Deferred follow-ups (status `DEFERRED` — `/next` skips)**: T1-17, T1-25, T1-26, T1-27, T1-28, T2-1d2, T4-9. **Effectively-blocked (TODO but cannot proceed)**: T2-4 — awaits `design/player-generation.md` authorship via `/log-decision` (22-field gene model + phenotype labels + names→personality mapping is creative/balance judgment). **Carry-forward known follow-ups from T2-5 self-review (P2/P3)**: SeasonState API surface tightening (current_match_day += 1 from cross-crate); StandingsRow.goal_difference cached vs derived; MatchOutcome could use existing Score type; Standings(Vec) wrapper is currently pure ceremony; matches_played width inconsistency (u16 vs u32); season_read/write helpers on AppState to centralise LockPoisoned mapping; mid-loop fixture-count truth in play_fixtures (P1-2; doc-only fix landed, fuller structured-error variant deferred); end-to-end Tauri-test-harness coverage of `#[tauri::command]` wrappers (currently only `_inner` is tested); plus T2-3 carry-forwards still standing (BakeManifest.output_path: PathBuf; model_id as enum; MIN_NAME_BANK_SIZE const dedup; Culture invariant in validator vs type; workspace-hoist clap/tokio/reqwest/jsonschema).

## Blockers

- **T2-4 (`PlayerBio` generation) BLOCKED** on missing `design/player-generation.md` design doc. The 22-field gene model + phenotype-label catalog + names→personality mapping are substantive content/balance design calls. Resolve via `/log-decision` authoring an ADR + design-doc; OR by user authorship of `design/player-generation.md` outside of `/next`; OR by promoting T2-4 to TODO with a one-paragraph spec that names the 22 gene fields + phenotype labels explicitly.

## Last green verify

2026-05-18 (T2-5 close): `scripts/fw verify` exit 0 (cargo fmt + clippy + cargo test --workspace + pnpm test 56 frontend + pnpm typecheck + banned-terms + canonical-hash regression on both pins UNCHANGED + content-pack validate-structural + hash-pins atomicity test + cargo audit + cargo deny). 41 baker+tauri+content tests added.

## Last canonical hash

`blake3:eaf842ac3d19651d38dc7ce45d0763cc62b4d571ce2c2a5d56f1ee3c6ddead46` (60-tick smoke seed; UNCHANGED from T2-1b rebaseline — bin/IPC-crate work; sim crates untouched).

**Second corpus pin:** `blake3:aa7efe9b2a567d5e87d12c7da6a4ea928271429729884f38819baed85c3be5ae` (600-tick extended seed; UNCHANGED from T2-1-codex-fix rebaseline).

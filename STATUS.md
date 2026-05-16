# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 — First Match.** T1-5 shipped: Tauri IPC consolidation end-to-end. `fw-tauri` is now the only command surface (`src-tauri/src/commands.rs` deleted entirely; `src-tauri/main.rs` has ZERO local `#[tauri::command]` impls). `AppState` constructed once at app startup via `tauri::Builder::manage` (T1-11 stop-gap `load_content_for_command()` deleted). `play_match` returns a typed `MatchResult { final_score, canonical_hash, match_events, commentary_preview, seed_hex, tick_count }`. `match_frames` caps at `MAX_FRAMES_PER_REQUEST = 7200` with typed `IpcError::TooManyFrames`. The match-engine vertical is complete from sim through IPC: T1-3.5 ball actions → T1-4a MatchEvents → T1-4b Tracery commentary → T1-11 signatures wired → **T1-5 makes the full match playable through a typed IPC contract the frontend can consume**. Codex 2026-05-16 audit P1 closed on IPC split-brain + match_frames unbounded. Next: T1-6 (frontend Match page consuming the new MatchResult) per MASTER_PLAN order, OR T1-12 (content validation hardening) per audit-triage order.

## Active task

(none — T1-5 closed across 1 implementation pass + 1 main-thread fix-pass for 2 P1 + 4 P2/P3 self-review findings)

## Phase pointer

- **Just closed:** **T1-5** — `AppState` + typed `IpcError` discriminated union + `MAX_FRAMES_PER_REQUEST = 7200` cap + `MatchResult` with pre-rendered `commentary_preview` + deletion of `src-tauri/src/commands.rs` (4 T2-stub commands gone) + harmonised frontend `playMatch(seed, tickCount)` wrapper + `FrameSource.ts` pre-invoke MAX validation + 6 IPC contract tests (round-trip canonical-hash, error-shape, discriminated-union serde, src-tauri/commands.rs deletion assertion). Fix-pass closed code-reviewer's P1 (get_dummy_state `Result<_, String>` → `Result<_, IpcError>`) + silent-failure-hunter's P1 (commentary `unwrap_or_else` → aggregate counter + `log::warn!`) + 4 P2/P3 fold-ins (AppState `pub`→`pub(crate)` + accessors; Score u16→u8; `?` operator + dual `From<ContentLoadError>`/`From<ContentInitError>`; `saturating_add(1)` → `debug_assert!` + bare add).
- **Next:** **T1-6** per MASTER_PLAN order — Frontend Match page with Play button, text recap rendering (goals + minute markers), event-list view; reuses T1-2a board component via debug toggle. Consumes the new `MatchResult` shape directly. **OR T1-12** per audit-triage order — `fw-content` validation hardening (fail on duplicate IDs; replace unwired Ok(()) validators with real implementations OR ValidatorNotImplemented errors; post-parse serde validation on RoleId/SignatureId/SignatureCandidate newtypes).
- **Recommended /next order** (per audit triage): T1-6 → T1-12 (content validation hardening) → T1-10 (LUT bake) → T1-13 (frontend Vitest + cargo audit) → T1-7 (procgen) → T1-8 (replay corpus) → T1-9 (behavioral assertions). The audit-triage P1s left are content-validation (T1-12) + LUT-bake determinism risk (T1-10) + frontend test gate (T1-13). Two audit P1s (signatures unreachable + IPC split-brain + match_frames unbounded) are closed by T1-11 + T1-5; the remaining four queue cleanly after T1-6 ships the player-visible vertical slice.

## Blockers

None.

## Last green verify

2026-05-16 — `scripts/fw verify` clean post T1-5 + fix-pass: fmt + clippy + `cargo test --workspace` + release-mode canonical-hash regression on `782fcde6…8c0f` + banned-terms + determinism-audit + `fw-content-baker validate`. Frontend `pnpm typecheck` + `pnpm lint` clean.

## Last canonical hash

`blake3:782fcde65ba8a0fc12bb90af1b61f77d8cd403103ab3671b0d5d6b03e75c8c0f` (60-tick smoke seed; UNCHANGED since T1-3.5; T1-4a/T1-4b/T1-11/T1-5 all kept it stable — IPC + content side + new constructor paths don't reach the existing smoke seed which uses `MatchState::initial(seed)` with `&BTreeMap::new()` for sig_definitions).

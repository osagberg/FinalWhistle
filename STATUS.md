# STATUS — Final Whistle

**Last updated**: 2026-05-29

## Phase

**Phase T4 — Pillar Wiring + Polish — IN PROGRESS.** T4-1/2/3/4/5a/6a + T4-2.5a + T4-sim-halt + T4-2.5b + T4-2.5c (+T1-25/T1-26) + T4-2.5e + T4-2.5d + T4-2.5f + T4-F1 + T4-F2 + **T4-2.5g** DONE. Career-roster layer underway (blueprint `docs/design/career-roster-layer.md`). T4-2.5g (2026-06-03) shipped SaveV4 — career persistence via a mutable-subset `SavedPlayerInstance` + `save_career`/`load_career` IPC (regenerate-base-from-seed-then-overlay; user-approved Option A, DECISIONS 2026-06-03). The career layer is now **correct** (T4-F1/F2) **+ persistent** (T4-2.5g); the **visible** phase (UI) begins at T4-2.5h. Both match-state pins UNCHANGED. T4.5 (World Scale + Content Bake) is the EA-critical phase between T4 and T5 — see `docs/MASTER_PLAN.md ## Tier 4.5`.

**2026-06-02 external dual review (Claude + Codex; memo `docs/audits/2026-06-02-external-dual-review-claude-codex.md`)** re-prioritised the career layer to correct → persistent → visible. Correct (T4-F1/F2) + persistent (T4-2.5g) are DONE; the remaining review items: F4 (player-visible UI — folds into T4-2.5h + T4-F4), I1 (dev fixture shim), F5/F6 (match-engine, second), C1 (CI posture, at private-flip).

## Active task

**T4-2.5h next** — per-player match stats + Squad stats UI (the "visible" phase begins). `PlayerSeasonStats` accrues after each match (apps/goals/minutes across a season — today only `career_apps` increments; full accrual is this row); `get_squad`/roster DTO carry them; `Squad.tsx` renders them. Deps T4-2.5c + T4-2.5e (both DONE). `ui-programmer` (frontend + a preview screenshot per Step 5.1). Unblocks T4-2b, T4-5b, T4-F4. Then T4-F4 (full roster/scout/breakthrough UI + the TS `IpcError::notYetObserved` gap) + T4-I1 (dev fixture invoke shim). Parallel-eligible: QA-3. Match-goal-RATE realism stays T5-5b.

## Blockers

None live. Pre-task for T4-2.5b: log Decision 5 (forward-compat clause) via `/log-decision` before it starts. T4-sim-halt closed the desktop FullTime/goal SPAM (2026-06-02). OPEN: match-goal-RATE realism (a real 5400-tick match scores unrealistically many goals until the engine is calibrated — that is T5-5b, layered separately from the fantasy/dev systems).

Previously: T2-1d2 (xg_utility honesty / cross-band oscillation) re-anchored to Deferred section of MASTER_PLAN. Open EA-scope question resolved 2026-05-29 per DECISIONS.md — DESIGN_DOC `§MVP-scope` anchor added at §8 `### MVP scope`.

## Last green verify

2026-06-03 (T4-2.5g close): `scripts/fw verify` exit 0. SaveV4 career persistence — `SaveEnvelope::V4` (wire `0x04`) + mutable-subset `SavedPlayerInstance` + `migrate_v3_to_v4` + `save_career`/`load_career` IPC (regenerate-base-then-overlay); `PlayerSeasonStats` moved to fw-core; `breakthrough_eval_watermark` persisted as `u64`. 4 migration tests + a non-empty-roster frozen `v4_career_sample.fwsave` + AC3 round-trip-with-mutated-delta. Self-review triple → Accept + fixes (silent-overlay-skip→log::warn, usize→u64 wire, false-migration docstrings, club_id §11 assert). **Both match-state canonical pins UNCHANGED** (`85f45bf8…`/`206bddae…`) — save state is off the match-hash path. Prior: T4-F2 (scout disambiguation); T4-F1 (breakthrough RNG → global EventId); T4-2.5f (pillar-4 scouting). Clippy + fmt + determinism-audit + banned-terms + cargo audit/deny clean throughout.

## Last canonical hash

`blake3:85f45bf8ae8821182a45a82969ec36bc5b2d70ba2518b8271de24782fd8064fa` (60-tick, UNCHANGED) + `blake3:206bddaef4df4fec909b9456e2efb04f6c5120ef4104dbdf6aec9665b45b57a9` (600-tick). **600-tick REBASELINED at T4-2.5c (2026-06-02)** — authorized; pillar-5 signature candidates onto role-matched (MID) slots, was slot-7-only; SINGLE-pin (the 60-tick bare-`initial` path is untouched); 5-seed envelope re-verified (pinned 4 goals ∈ [2,5]). The 60-tick pin lives in 3 sites (`canonical_hash.rs` + `0xdeadbeefdeadbeef.ron` + `fw-content/tests/fixtures_load.rs`) — all unchanged this row. Prior: T4-sim-halt rebaselined both. Save wire bytes: SaveEnvelope V0–V4 (V4 = `0x04`, shipped T4-2.5g; current production schema); SettingsEnvelope V0. Next bump: SaveV5 at T4.5-H (world-gen descriptors) / T4.5-E1 (mutable attributes).

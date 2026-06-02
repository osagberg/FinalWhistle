# STATUS — Final Whistle

**Last updated**: 2026-05-29

## Phase

**Phase T4 — Pillar Wiring + Polish — IN PROGRESS.** T4-1/2/3/4/5a/6a + T4-2.5a + T4-sim-halt + T4-2.5b + T4-2.5c (+T1-25/T1-26) + T4-2.5e + T4-2.5d + **T4-2.5f** DONE. Career-roster layer underway (blueprint `docs/design/career-roster-layer.md`). T4-2.5f (2026-06-02) wired pillar 4 — `observe_match_participants` runs single-scout `observe_player` on each club's starting XI per match-day in `advance_week_inner`, caching the latest `ScoutReport` on `PlayerInstance.last_scout_report`; new `get_scout_report(playerId)` IPC projects Q32→f64 bands + `UncertaintyBand` labels. Source bio re-derived from the career-start round-robin index (gene-match asserted). Self-review triple → Accept + 4 fixes (shared q32 helper, fixture-derived AC4 test, PlayerId newtype param, doc-rot). NON-canonical (both pins UNCHANGED). **All 5 pillars now produce player-visible career output.** T4.5 (World Scale + Content Bake) is the EA-critical phase between T4 and T5 — see `docs/MASTER_PLAN.md ## Tier 4.5`.

## Active task

**T4-2.5g next** — SaveV4: persist `roster` (incl. `genes` + the new `last_scout_report` + `breakthrough_state` + `breakthrough_eval_watermark`); `migrate_v3_to_v4` (absorbs V3 `breakthrough_states`); `save_career`/`load_career` IPC (closes the no-production-load-path gap). Four migration tests (forward-migration + callback-preservation + forward-incompat-failure + round-trip-byte-identical); SaveEnvelope wire byte `0x04` pinned. Deps T4-2.5b (DONE) + T4-2.5d (DONE). `lead-programmer`. **T4-2.5g follow-up flagged from T4-2.5f self-review:** `ScoutReport` carries a redundant constant `scout_archetype_id` + a duplicate `player_id` string — consider strip-and-reconstruct on load rather than persisting verbatim per observed instance. Parallel-eligible: QA-3 (world-gen seed-diversity proptest), T4-2.5h (per-player stats + Squad UI; deps DONE). Match-goal-RATE realism stays T5-5b.

## Blockers

None live. Pre-task for T4-2.5b: log Decision 5 (forward-compat clause) via `/log-decision` before it starts. T4-sim-halt closed the desktop FullTime/goal SPAM (2026-06-02). OPEN: match-goal-RATE realism (a real 5400-tick match scores unrealistically many goals until the engine is calibrated — that is T5-5b, layered separately from the fantasy/dev systems).

Previously: T2-1d2 (xg_utility honesty / cross-band oscillation) re-anchored to Deferred section of MASTER_PLAN. Open EA-scope question resolved 2026-05-29 per DECISIONS.md — DESIGN_DOC `§MVP-scope` anchor added at §8 `### MVP scope`.

## Last green verify

2026-06-02 (T4-2.5f close): `scripts/fw verify` exit 0. Pillar-4 scouting wired into `advance_week_inner` — `observe_match_participants` runs single-scout `observe_player` on each club's starting XI, caching `ScoutReport` on `PlayerInstance.last_scout_report`; `get_scout_report(playerId)` IPC projects Q32→f64 bands + `UncertaintyBand` labels; source bio re-derived from the round-robin index (gene-match asserted). 7 new fw-tauri unit/dto tests + 4 integration green. **Both canonical pins UNCHANGED** (`85f45bf8…`/`206bddae…`) — non-canonical CareerState. Prior: T4-2.5d (pillar-3 breakthroughs); T4-2.5e (pillar-2 + PlayerId offset); T4-2.5c (600-tick pin `206bddae…`). Clippy + fmt + determinism-audit + banned-terms + cargo audit/deny clean throughout.

## Last canonical hash

`blake3:85f45bf8ae8821182a45a82969ec36bc5b2d70ba2518b8271de24782fd8064fa` (60-tick, UNCHANGED) + `blake3:206bddaef4df4fec909b9456e2efb04f6c5120ef4104dbdf6aec9665b45b57a9` (600-tick). **600-tick REBASELINED at T4-2.5c (2026-06-02)** — authorized; pillar-5 signature candidates onto role-matched (MID) slots, was slot-7-only; SINGLE-pin (the 60-tick bare-`initial` path is untouched); 5-seed envelope re-verified (pinned 4 goals ∈ [2,5]). The 60-tick pin lives in 3 sites (`canonical_hash.rs` + `0xdeadbeefdeadbeef.ron` + `fw-content/tests/fixtures_load.rs`) — all unchanged this row. Prior: T4-sim-halt rebaselined both. Save wire bytes: SaveEnvelope V0–V3; SettingsEnvelope V0. T4-2.5g adds SaveV4 (`0x04`).

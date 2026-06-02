# STATUS — Final Whistle

**Last updated**: 2026-05-29

## Phase

**Phase T4 — Pillar Wiring + Polish — IN PROGRESS.** T4-1/2/3/4/5a/6a + T4-2.5a + T4-sim-halt + T4-2.5b + T4-2.5c (+T1-25/T1-26) + T4-2.5e + **T4-2.5d** DONE. Career-roster layer underway (blueprint `docs/design/career-roster-layer.md`). T4-2.5d (2026-06-02) wired pillar 3 — `advance_season_inner` runs `evaluate()` per rostered player (genes round-robined onto the roster, incremental per-season eval via a `CareerState` ledger watermark, ceiling write via a new fw-core mutator), producing ≥1 `BreakthroughMoment` in a 5-season career. The self-review triple caught a P0 (breakthroughs re-firing from the same historical event every season) — fixed by the incremental watermark. NON-canonical (both pins UNCHANGED). **Pillars 2/3/5 + partial-4 now produce player-visible career output; only scouting (T4-2.5f) remains.** T4.5 (World Scale + Content Bake) is the EA-critical phase between T4 and T5 — see `docs/MASTER_PLAN.md ## Tier 4.5`.

## Active task

**T4-2.5f next** — pillar-4 scouting: `observe_player` per the player's-club match-day; cache the latest `ScoutReport` on `PlayerInstance` (`observation_count` already exists from T4-2.5b); new `get_scout_report(playerId)` IPC + `ScoutReportDto` (Q32→f64 bands + `UncertaintyBand` label) registered in `src-tauri/main.rs`. Deps T4-2.5b (DONE). Done-criterion: `get_scout_report` returns a banded estimate; a report after 5 observations differs from 0 observations; an IPC contract test round-trips the DTO. `lead-programmer`. Parallel-eligible: QA-3 (world-gen seed-diversity proptest). Then T4-2.5g (SaveV4 — must persist `roster.genes` + `breakthrough_state` + `breakthrough_eval_watermark`) + T4-2.5h (per-player stats + Squad UI). Match-goal-RATE realism stays T5-5b.

## Blockers

None live. Pre-task for T4-2.5b: log Decision 5 (forward-compat clause) via `/log-decision` before it starts. T4-sim-halt closed the desktop FullTime/goal SPAM (2026-06-02). OPEN: match-goal-RATE realism (a real 5400-tick match scores unrealistically many goals until the engine is calibrated — that is T5-5b, layered separately from the fantasy/dev systems).

Previously: T2-1d2 (xg_utility honesty / cross-band oscillation) re-anchored to Deferred section of MASTER_PLAN. Open EA-scope question resolved 2026-05-29 per DECISIONS.md — DESIGN_DOC `§MVP-scope` anchor added at §8 `### MVP scope`.

## Last green verify

2026-06-02 (T4-2.5d close): `scripts/fw verify` exit 0. Pillar-3 breakthroughs wired into `advance_season_inner` — genes on roster, incremental per-season `evaluate()` via a `CareerState` ledger watermark (the P0 re-firing fix), ceiling write via a new fw-core `apply_breakthrough_delta`; 9 breakthrough_wiring tests (incl. the no-re-fire invariant) + 10 fw-replay green. **Both canonical pins UNCHANGED** (`85f45bf8…`/`206bddae…`) — non-canonical CareerState. Prior: T4-2.5e (pillar-2 + PlayerId offset); T4-2.5c (600-tick pin `206bddae…`). Clippy + fmt + determinism-audit + banned-terms + cargo audit/deny clean throughout.

## Last canonical hash

`blake3:85f45bf8ae8821182a45a82969ec36bc5b2d70ba2518b8271de24782fd8064fa` (60-tick, UNCHANGED) + `blake3:206bddaef4df4fec909b9456e2efb04f6c5120ef4104dbdf6aec9665b45b57a9` (600-tick). **600-tick REBASELINED at T4-2.5c (2026-06-02)** — authorized; pillar-5 signature candidates onto role-matched (MID) slots, was slot-7-only; SINGLE-pin (the 60-tick bare-`initial` path is untouched); 5-seed envelope re-verified (pinned 4 goals ∈ [2,5]). The 60-tick pin lives in 3 sites (`canonical_hash.rs` + `0xdeadbeefdeadbeef.ron` + `fw-content/tests/fixtures_load.rs`) — all unchanged this row. Prior: T4-sim-halt rebaselined both. Save wire bytes: SaveEnvelope V0–V3; SettingsEnvelope V0. T4-2.5g adds SaveV4 (`0x04`).

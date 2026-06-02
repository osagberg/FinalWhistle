# STATUS — Final Whistle

**Last updated**: 2026-05-29

## Phase

**Phase T4 — Pillar Wiring + Polish — IN PROGRESS.** T4-1/2/3/4/5a/6a + T4-2.5a + T4-sim-halt + T4-2.5b + **T4-2.5c (+T1-25/T1-26)** DONE. Career-roster layer underway (blueprint `docs/design/career-roster-layer.md`). T4-2.5c (2026-06-02) wired pillar 5 — signature candidates now fire on ROLE-MATCHED slots (the 6 MID slots, was slot-7-only); an all-22-slot spray was rejected for collapsing scoring (DECISIONS 2026-06-02 T4-2.5c); 600-tick canonical pin rebaselined to `206bddae…` (60-tick untouched). T4.5 (World Scale + Content Bake) is the new EA-critical phase between T4 and T5 per re-baseline 2026-05-29 — see `docs/MASTER_PLAN.md ## Tier 4.5`.

## Active task

**T4-2.5d next** — pillar-3 breakthrough wiring: `advance_season_inner` builds a `BreakthroughContext` per rostered player via the T4-2.5a gene→family bridge, calls `fw_memory::breakthrough::evaluate()`, applies deltas to the canonical ceiling, appends `outcome.event` to the ledger. Deps T4-2.5a+b (DONE). Done-criterion: a 5-season integration test on seed `0xfeedbeefcafefade` asserts ≥1 `BreakthroughMoment` (the end-to-end proof the T3 gate lacked). `gameplay-programmer`. Parallel-eligible siblings: T4-2.5e (pillar-2 player-subject MemoryEvents), T4-2.5f (pillar-4 scouting + `get_scout_report`), QA-3. Match-goal-RATE realism stays T5-5b.

## Blockers

None live. Pre-task for T4-2.5b: log Decision 5 (forward-compat clause) via `/log-decision` before it starts. T4-sim-halt closed the desktop FullTime/goal SPAM (2026-06-02). OPEN: match-goal-RATE realism (a real 5400-tick match scores unrealistically many goals until the engine is calibrated — that is T5-5b, layered separately from the fantasy/dev systems).

Previously: T2-1d2 (xg_utility honesty / cross-band oscillation) re-anchored to Deferred section of MASTER_PLAN. Open EA-scope question resolved 2026-05-29 per DECISIONS.md — DESIGN_DOC `§MVP-scope` anchor added at §8 `### MVP scope`.

## Last green verify

2026-06-02 (T4-2.5c close): `scripts/fw verify` exit 0. Pillar-5 role-matched signature spread + `with_slot_signatures`/`with_possession` builders + season roster-build; T1-25/T1-26 dispatch-hardening + AC-5 roster-signature + role-formation cross-check tests. **600-tick pin REBASELINED `856a7fed…`→`206bddae…`** (authorized; 60-tick UNCHANGED) + envelope re-verified (pinned 4 goals ∈ [2,5]). Prior: T4-2.5b (roster model, pins unchanged); T4-sim-halt (both pins). Clippy + fmt + determinism-audit + banned-terms + cargo audit/deny clean throughout.

## Last canonical hash

`blake3:85f45bf8ae8821182a45a82969ec36bc5b2d70ba2518b8271de24782fd8064fa` (60-tick, UNCHANGED) + `blake3:206bddaef4df4fec909b9456e2efb04f6c5120ef4104dbdf6aec9665b45b57a9` (600-tick). **600-tick REBASELINED at T4-2.5c (2026-06-02)** — authorized; pillar-5 signature candidates onto role-matched (MID) slots, was slot-7-only; SINGLE-pin (the 60-tick bare-`initial` path is untouched); 5-seed envelope re-verified (pinned 4 goals ∈ [2,5]). The 60-tick pin lives in 3 sites (`canonical_hash.rs` + `0xdeadbeefdeadbeef.ron` + `fw-content/tests/fixtures_load.rs`) — all unchanged this row. Prior: T4-sim-halt rebaselined both. Save wire bytes: SaveEnvelope V0–V3; SettingsEnvelope V0. T4-2.5g adds SaveV4 (`0x04`).

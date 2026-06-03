# STATUS — Final Whistle

**Last updated**: 2026-05-29

## Phase

**Phase T4 — Pillar Wiring + Polish — IN PROGRESS.** T4-1/2/3/4/5a/6a + T4-2.5a + T4-sim-halt + T4-2.5b + T4-2.5c (+T1-25/T1-26) + T4-2.5e + T4-2.5d + T4-2.5f + T4-F1 + **T4-F2** DONE. Career-roster layer underway (blueprint `docs/design/career-roster-layer.md`). T4-F2 (2026-06-03) fixed scout-observation correctness: `observe_player` now keys its RNG `site` on the roster `PlayerId` (was hardcoded 0 → players sharing a bio got identical reports) and `ScoutReport.player_id` is the roster id (was the content-bio id). Prior T4-F1 fixed the breakthrough-RNG event-id keying. Both are the external-review correctness prerequisites for SaveV4; both pins UNCHANGED. All 5 pillars are backend-wired; an external review (below) notes they are not yet player-VISIBLE in the UI (T4-F4). T4.5 (World Scale + Content Bake) is the EA-critical phase between T4 and T5 — see `docs/MASTER_PLAN.md ## Tier 4.5`.

**2026-06-02 external dual review (Claude + Codex; memo `docs/audits/2026-06-02-external-dual-review-claude-codex.md`)** re-prioritised the career layer to correct → persistent → visible. Both CONFIRMED correctness prerequisites are now DONE: **T4-F1** (breakthrough RNG → global career `EventId`) + **T4-F2** (scout RNG `site` per-player + `ScoutReport.player_id` → roster id). SaveV4 can now persist correct data. See DECISIONS 2026-06-02 external-review entry.

## Active task

**T4-2.5g next** — SaveV4: persist `roster` (incl. `genes` + `last_scout_report` + `breakthrough_state` + `breakthrough_eval_watermark`); `migrate_v3_to_v4` (absorbs V3 `breakthrough_states`); `save_career`/`load_career` IPC (closes the no-production-load-path gap). External-review F3 additions: NON-EMPTY roster/scout/breakthrough migration fixtures + an explicit V3→V4 identity map (V3 keys are content-bio `PlayerId`; roster uses the `ROSTER_PLAYER_ID_BASE` range — old keys won't attach without a map). T4-2.5f follow-up: `ScoutReport` still carries a constant `scout_archetype_id` String — consider stripping on persist. Four migration tests (forward-migration + callback-preservation + forward-incompat-failure + round-trip-byte-identical); SaveEnvelope wire byte `0x04` pinned. Deps T4-2.5b + T4-2.5d + T4-F1 + T4-F2 (all DONE). `lead-programmer`. Then T4-F4 + T4-2.5h (player-visible UI) + T4-I1 (dev fixture shim). Match-goal-RATE realism stays T5-5b.

## Blockers

None live. Pre-task for T4-2.5b: log Decision 5 (forward-compat clause) via `/log-decision` before it starts. T4-sim-halt closed the desktop FullTime/goal SPAM (2026-06-02). OPEN: match-goal-RATE realism (a real 5400-tick match scores unrealistically many goals until the engine is calibrated — that is T5-5b, layered separately from the fantasy/dev systems).

Previously: T2-1d2 (xg_utility honesty / cross-band oscillation) re-anchored to Deferred section of MASTER_PLAN. Open EA-scope question resolved 2026-05-29 per DECISIONS.md — DESIGN_DOC `§MVP-scope` anchor added at §8 `### MVP scope`.

## Last green verify

2026-06-03 (T4-F2 close): `scripts/fw verify` exit 0. Scout-observation RNG `site` now keys on the roster `PlayerId` (was hardcoded 0); `ScoutReport.player_id: String → fw_core::PlayerId`; `ScoutReportDto::from_report` dropped its redundant id param. Tests: `different_subjects_same_bio_differ`, `observe_player_report_player_id_matches_subject`, the proptest now varies the subject; the fw-scouting `observe_proptest` insta snapshot regenerated (authorized readable-output change). **Both match-state canonical pins UNCHANGED** (`85f45bf8…`/`206bddae…`) — scouting runs post-match in CareerState. Prior: T4-F1 (breakthrough RNG → global EventId); T4-2.5f (pillar-4 scouting); T4-2.5d (pillar-3 breakthroughs). Clippy + fmt + determinism-audit + banned-terms + cargo audit/deny clean throughout.

## Last canonical hash

`blake3:85f45bf8ae8821182a45a82969ec36bc5b2d70ba2518b8271de24782fd8064fa` (60-tick, UNCHANGED) + `blake3:206bddaef4df4fec909b9456e2efb04f6c5120ef4104dbdf6aec9665b45b57a9` (600-tick). **600-tick REBASELINED at T4-2.5c (2026-06-02)** — authorized; pillar-5 signature candidates onto role-matched (MID) slots, was slot-7-only; SINGLE-pin (the 60-tick bare-`initial` path is untouched); 5-seed envelope re-verified (pinned 4 goals ∈ [2,5]). The 60-tick pin lives in 3 sites (`canonical_hash.rs` + `0xdeadbeefdeadbeef.ron` + `fw-content/tests/fixtures_load.rs`) — all unchanged this row. Prior: T4-sim-halt rebaselined both. Save wire bytes: SaveEnvelope V0–V3; SettingsEnvelope V0. T4-2.5g adds SaveV4 (`0x04`).

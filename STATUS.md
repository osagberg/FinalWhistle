# STATUS — Final Whistle

**Last updated**: 2026-05-29

## Phase

**Phase T4 — Pillar Wiring + Polish — IN PROGRESS.** T4-1/2/3/4/5a/6a + T4-2.5a + T4-sim-halt + T4-2.5b + T4-2.5c (+T1-25/T1-26) + T4-2.5e + T4-2.5d + **T4-2.5f** DONE. Career-roster layer underway (blueprint `docs/design/career-roster-layer.md`). T4-2.5f (2026-06-02) wired pillar 4 — `observe_match_participants` runs single-scout `observe_player` on each club's starting XI per match-day in `advance_week_inner`, caching the latest `ScoutReport` on `PlayerInstance.last_scout_report`; new `get_scout_report(playerId)` IPC projects Q32→f64 bands + `UncertaintyBand` labels. Source bio re-derived from the career-start round-robin index (gene-match asserted). Self-review triple → Accept + 4 fixes (shared q32 helper, fixture-derived AC4 test, PlayerId newtype param, doc-rot). NON-canonical (both pins UNCHANGED). All 5 pillars are backend-wired; an external review (below) notes they are not yet player-VISIBLE in the UI (T4-F4). T4.5 (World Scale + Content Bake) is the EA-critical phase between T4 and T5 — see `docs/MASTER_PLAN.md ## Tier 4.5`.

**2026-06-02 external dual review (Claude + Codex; memo `docs/audits/2026-06-02-external-dual-review-claude-codex.md`)** re-prioritised the career layer to correct → persistent → visible. Two CONFIRMED correctness bugs now gate SaveV4: F1 (breakthrough RNG keyed by renumbered event ids → magnitudes correlate across a career) and F2 (scout noise not disambiguated by player; `ScoutReport.player_id` wrong id space). See DECISIONS 2026-06-02 external-review entry.

## Active task

**T4-F1 next** — fix the breakthrough-delta RNG to key on the GLOBAL career `EventId`, not the within-batch index that `filter_new_events_for_player`'s `MemoryLedger::append` renumbers from 0 (`breakthrough.rs:984`/`:1049` read `event.event_id.0`). Preserve original `EventId`s through the per-player filter (e.g. a non-renumbering `MemoryLedger::from_events`); fix the false `season.rs:643-648` comment; cover both positive + regressive gates. Off the canonical-hash path → pins UNCHANGED. **Must land before T4-2.5g persists `BreakthroughState`.** `gameplay-programmer` (TDD). Then T4-F2 (scout `site` + roster id) → T4-2.5g (SaveV4, deps F1+F2) → T4-F4 + T4-2.5h (player-visible UI) + T4-I1 (dev fixture shim). Match-goal-RATE realism stays T5-5b.

## Blockers

None live. Pre-task for T4-2.5b: log Decision 5 (forward-compat clause) via `/log-decision` before it starts. T4-sim-halt closed the desktop FullTime/goal SPAM (2026-06-02). OPEN: match-goal-RATE realism (a real 5400-tick match scores unrealistically many goals until the engine is calibrated — that is T5-5b, layered separately from the fantasy/dev systems).

Previously: T2-1d2 (xg_utility honesty / cross-band oscillation) re-anchored to Deferred section of MASTER_PLAN. Open EA-scope question resolved 2026-05-29 per DECISIONS.md — DESIGN_DOC `§MVP-scope` anchor added at §8 `### MVP scope`.

## Last green verify

2026-06-02 (T4-2.5f close): `scripts/fw verify` exit 0. Pillar-4 scouting wired into `advance_week_inner` — `observe_match_participants` runs single-scout `observe_player` on each club's starting XI, caching `ScoutReport` on `PlayerInstance.last_scout_report`; `get_scout_report(playerId)` IPC projects Q32→f64 bands + `UncertaintyBand` labels; source bio re-derived from the round-robin index (gene-match asserted). 7 new fw-tauri unit/dto tests + 4 integration green. **Both canonical pins UNCHANGED** (`85f45bf8…`/`206bddae…`) — non-canonical CareerState. Prior: T4-2.5d (pillar-3 breakthroughs); T4-2.5e (pillar-2 + PlayerId offset); T4-2.5c (600-tick pin `206bddae…`). Clippy + fmt + determinism-audit + banned-terms + cargo audit/deny clean throughout.

## Last canonical hash

`blake3:85f45bf8ae8821182a45a82969ec36bc5b2d70ba2518b8271de24782fd8064fa` (60-tick, UNCHANGED) + `blake3:206bddaef4df4fec909b9456e2efb04f6c5120ef4104dbdf6aec9665b45b57a9` (600-tick). **600-tick REBASELINED at T4-2.5c (2026-06-02)** — authorized; pillar-5 signature candidates onto role-matched (MID) slots, was slot-7-only; SINGLE-pin (the 60-tick bare-`initial` path is untouched); 5-seed envelope re-verified (pinned 4 goals ∈ [2,5]). The 60-tick pin lives in 3 sites (`canonical_hash.rs` + `0xdeadbeefdeadbeef.ron` + `fw-content/tests/fixtures_load.rs`) — all unchanged this row. Prior: T4-sim-halt rebaselined both. Save wire bytes: SaveEnvelope V0–V3; SettingsEnvelope V0. T4-2.5g adds SaveV4 (`0x04`).

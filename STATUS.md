# STATUS — Final Whistle

**Last updated**: 2026-05-29

## Phase

**Phase T4 — Pillar Wiring + Polish — IN PROGRESS.** T4-1/2/3/4/5a/6a + T4-2.5a + T4-sim-halt + T4-2.5b + T4-2.5c (+T1-25/T1-26) + T4-2.5e + T4-2.5d + T4-2.5f + **T4-F1** DONE. Career-roster layer underway (blueprint `docs/design/career-roster-layer.md`). T4-F1 (2026-06-02) fixed the breakthrough-delta RNG to key on the global career `EventId` (it was the renumbered batch index → magnitudes correlated across seasons); new non-renumbering `MemoryLedger::from_events`, `get_by_id` hardened to resolve by id; self-review caught + fixed a §11 fail-open hazard. Both pins UNCHANGED. Prior T4-2.5f wired pillar 4 (scouting per match-day + `get_scout_report` IPC). All 5 pillars are backend-wired; an external review (below) notes they are not yet player-VISIBLE in the UI (T4-F4). T4.5 (World Scale + Content Bake) is the EA-critical phase between T4 and T5 — see `docs/MASTER_PLAN.md ## Tier 4.5`.

**2026-06-02 external dual review (Claude + Codex; memo `docs/audits/2026-06-02-external-dual-review-claude-codex.md`)** re-prioritised the career layer to correct → persistent → visible. **T4-F1 DONE (2026-06-02):** breakthrough-delta RNG now keys on the GLOBAL career `EventId` (new non-renumbering `MemoryLedger::from_events`; both filters rewritten; `get_by_id` hardened to resolve by id) — magnitudes no longer correlate across a career; both pins UNCHANGED. **T4-F2 remains** before SaveV4. See DECISIONS 2026-06-02 external-review entry.

## Active task

**T4-F2 next** — fix scout-observation correctness (pillar 4). `observe_player` (`fw-scouting/src/observe.rs:39`) hardcodes the RNG `site` to `0`, so two roster players sharing a round-robin bio + `observation_count` get byte-identical reports (defeats the uncertainty pillar); thread the roster player identity into the `site` (per ADR-0009 `site` is the per-layer disambiguator). Separately `ScoutReport.player_id` (`observe.rs:57`) is the content-bio id, not the roster `PlayerId` (the T4-2.5f DTO masks this via `target_pid`, but persisting verbatim freezes the wrong id) — set/translate it to the roster id (or drop it as redundant). Touches fw-scouting + fw-tauri (`observe_match_participants` must pass the roster id). **Must land before T4-2.5g persists scout reports.** `gameplay-programmer` (TDD); pins UNCHANGED. Then T4-2.5g (SaveV4, deps F1+F2) → T4-F4 + T4-2.5h (player-visible UI) + T4-I1 (dev fixture shim). Match-goal-RATE realism stays T5-5b.

## Blockers

None live. Pre-task for T4-2.5b: log Decision 5 (forward-compat clause) via `/log-decision` before it starts. T4-sim-halt closed the desktop FullTime/goal SPAM (2026-06-02). OPEN: match-goal-RATE realism (a real 5400-tick match scores unrealistically many goals until the engine is calibrated — that is T5-5b, layered separately from the fantasy/dev systems).

Previously: T2-1d2 (xg_utility honesty / cross-band oscillation) re-anchored to Deferred section of MASTER_PLAN. Open EA-scope question resolved 2026-05-29 per DECISIONS.md — DESIGN_DOC `§MVP-scope` anchor added at §8 `### MVP scope`.

## Last green verify

2026-06-02 (T4-F1 close): `scripts/fw verify` exit 0. Breakthrough-delta RNG fixed to key on the global career `EventId` — new non-renumbering `MemoryLedger::from_events` (§11 ascending-id assert), both per-player filters rewritten, `get_by_id` hardened to resolve by id (binary search). 4 new tests (from_events ids/next_id; delta-differs-by-global-id; production-filter id-preservation; AC3 id assertions). **Both canonical pins UNCHANGED** (`85f45bf8…`/`206bddae…`) — fw-memory is off the match-hash path. Prior: T4-2.5f (pillar-4 scouting); T4-2.5d (pillar-3 breakthroughs); T4-2.5e (pillar-2 + PlayerId offset); T4-2.5c (600-tick pin `206bddae…`). Clippy + fmt + determinism-audit + banned-terms + cargo audit/deny clean throughout.

## Last canonical hash

`blake3:85f45bf8ae8821182a45a82969ec36bc5b2d70ba2518b8271de24782fd8064fa` (60-tick, UNCHANGED) + `blake3:206bddaef4df4fec909b9456e2efb04f6c5120ef4104dbdf6aec9665b45b57a9` (600-tick). **600-tick REBASELINED at T4-2.5c (2026-06-02)** — authorized; pillar-5 signature candidates onto role-matched (MID) slots, was slot-7-only; SINGLE-pin (the 60-tick bare-`initial` path is untouched); 5-seed envelope re-verified (pinned 4 goals ∈ [2,5]). The 60-tick pin lives in 3 sites (`canonical_hash.rs` + `0xdeadbeefdeadbeef.ron` + `fw-content/tests/fixtures_load.rs`) — all unchanged this row. Prior: T4-sim-halt rebaselined both. Save wire bytes: SaveEnvelope V0–V3; SettingsEnvelope V0. T4-2.5g adds SaveV4 (`0x04`).

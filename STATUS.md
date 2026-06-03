# STATUS — Final Whistle

**Last updated**: 2026-05-29

## Phase

**Phase T4 — Pillar Wiring + Polish — IN PROGRESS.** T4-1/2/3/4/5a/6a + T4-2.5a + T4-sim-halt + T4-2.5b + T4-2.5c (+T1-25/T1-26) + T4-2.5e + T4-2.5d + T4-2.5f + T4-F1 + T4-F2 + T4-2.5g + **T4-2.5h** DONE. Career-roster layer underway (blueprint `docs/design/career-roster-layer.md`). T4-2.5h (2026-06-03) began the **visible** phase: per-player season-stats accrual (apps/goals/minutes; goals from LegacyGoal; minutes=90/appearance) + a stats-bearing Squad screen (`get_squad_roster` → default lowest-ClubId club, labelled placeholder). The career layer is now **correct (T4-F1/F2) + persistent (T4-2.5g) + visible-underway (T4-2.5h)**. Both match-state pins UNCHANGED. T4.5 (World Scale + Content Bake) is the EA-critical phase between T4 and T5 — see `docs/MASTER_PLAN.md ## Tier 4.5`.

**2026-06-02 external dual review** re-prioritised the career layer to correct → persistent → visible. Correct (F1/F2) + persistent (T4-2.5g) DONE; visible underway (T4-2.5h → T4-F4). Remaining: I1 (dev fixture shim), F5/F6 (match-engine, second), C1 (CI posture, at private-flip).

## Active task

**T4-F4 next** — make the pillars player-VISIBLE: surface scouting bands (`get_scout_report`) + breakthrough/career moments on the Player page, a scouting view, frontend `get_scout_report`/`get_roster_for_club` wrappers, and CLOSE the TS `IpcError` union gap (`notYetObserved` + the pre-existing `leagueGenerationFailed`, which `get_squad_roster` can now emit). Extends the T4-2.5h Squad stats UI. Deps T4-2.5h + T4-F2 (both DONE). `ui-programmer`. Parallel-eligible: T4-I1 (dev fixture `invoke` shim — also unblocks browser preview of these Tauri routes), QA-3.

## Blockers

None blocking the next task. **KNOWN (pre-existing, NOT T4-2.5h): a match-sim proptest flake** — `fw-match-sim` `team_width_when_in_possession_within_band` occasionally fails (in-possession outfield Y-width ~23.98m vs the [24,70]m band floor, ~1.8cm under) on a random proptest seed. Proven unrelated to the career-layer work (zero fw-match-sim code touched; both canonical pins byte-identical). FILED as its own match-engine task (F5/T5-5b territory) — it makes `scripts/fw verify` probabilistically red until the band/behaviour is calibrated. T4-sim-halt closed the desktop FullTime/goal SPAM (2026-06-02). OPEN: match-goal-RATE realism (T5-5b).

Previously: T2-1d2 (xg_utility honesty / cross-band oscillation) re-anchored to Deferred section of MASTER_PLAN. Open EA-scope question resolved 2026-05-29 per DECISIONS.md — DESIGN_DOC `§MVP-scope` anchor added at §8 `### MVP scope`.

## Last green verify

2026-06-03 (T4-2.5h close): T4-2.5h's own surface fully green — `cargo test -p fw-tauri` 91 (incl. the strengthened goals + career_apps tests), frontend 264 (incl. the bench "Sub" assertion), `tsc` + `pnpm lint` clean, clippy + banned-terms clean, **both match-state canonical pins UNCHANGED** (`85f45bf8…`/`206bddae…`). NOTE: the full `scripts/fw verify` is probabilistically red on a PRE-EXISTING, UNRELATED `fw-match-sim` proptest flake (`team_width_when_in_possession_within_band`, ~23.98m vs the 24m band floor on a random seed) — proven not a T4-2.5h regression (zero fw-match-sim code touched; pins byte-identical) and filed as its own match-engine task; T4-2.5h committed over it per user direction (its commit hook re-checks the canonical hash, which passes). Shipped: season-stats accrual (apps/goals/minutes) + reset; `get_squad_roster` IPC; stats-bearing Squad screen. Prior: T4-2.5g (SaveV4); T4-F2 (scout disambiguation); T4-F1 (breakthrough RNG). Clippy + fmt + determinism-audit + banned-terms + cargo audit/deny clean throughout the career-layer work.

## Last canonical hash

`blake3:85f45bf8ae8821182a45a82969ec36bc5b2d70ba2518b8271de24782fd8064fa` (60-tick, UNCHANGED) + `blake3:206bddaef4df4fec909b9456e2efb04f6c5120ef4104dbdf6aec9665b45b57a9` (600-tick). **600-tick REBASELINED at T4-2.5c (2026-06-02)** — authorized; pillar-5 signature candidates onto role-matched (MID) slots, was slot-7-only; SINGLE-pin (the 60-tick bare-`initial` path is untouched); 5-seed envelope re-verified (pinned 4 goals ∈ [2,5]). The 60-tick pin lives in 3 sites (`canonical_hash.rs` + `0xdeadbeefdeadbeef.ron` + `fw-content/tests/fixtures_load.rs`) — all unchanged this row. Prior: T4-sim-halt rebaselined both. Save wire bytes: SaveEnvelope V0–V4 (V4 = `0x04`, shipped T4-2.5g; current production schema); SettingsEnvelope V0. Next bump: SaveV5 at T4.5-H (world-gen descriptors) / T4.5-E1 (mutable attributes).

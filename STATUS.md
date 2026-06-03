# STATUS — Final Whistle

**Last updated**: 2026-05-29

## Phase

**Phase T4 — Pillar Wiring + Polish — IN PROGRESS.** T4-1/2/3/4/5a/6a + T4-2.5a + T4-sim-halt + T4-2.5b + T4-2.5c (+T1-25/T1-26) + T4-2.5e + T4-2.5d + T4-2.5f + T4-F1 + T4-F2 + T4-2.5g + T4-2.5h + **T4-F4** DONE. **Running AUTONOMOUS (user out): a self-paced /loop runs /next through Phase T4, making recommended calls on forks, stopping at the T4 phase boundary or a real blocker.** T4-F4 (2026-06-03) added the Player-page scout-report section (UncertaintyBand bands + labels; graceful notYetObserved/non-roster) + closed the TS IpcError union gap. The career layer is now **correct (F1/F2) + persistent (T4-2.5g) + visible (T4-2.5h Squad stats + T4-F4 Player scout)**. Both match-state pins UNCHANGED. T4.5 (World Scale + Content Bake) is the EA-critical phase between T4 and T5.

**2026-06-02 external dual review** re-prioritised the career layer to correct → persistent → visible — all three now landed. Remaining T4: I1 (dev fixture shim, NEXT), the pillar polish rows (T4-2.5i/j/k/L), T4-7, then the T4-8 phase gate. F5/F6 (match-engine) second; C1 at private-flip. **Filed follow-up:** roster Squad → Player navigation (roster DTO needs a navigable qualified-id) so the new scout section is reachable for roster players.

## Active task

**T4-I1 next** (autonomous loop) — dev-only, fail-loud `safeInvoke` fixture shim (`?backend=fixtures` / `VITE_FW_BROWSER_BACKEND=fixtures`) so the Tauri-`invoke` routes (Squad/Player/Career/Standings) render in plain Chrome, unblocking browser preview of the T4-2.5h + T4-F4 UI (currently preview-n/a). Deps T4-1 (DONE). `ui-programmer`. `[RELAYED]` external-review I1 — verify cited lines at pick time. Parallel-eligible: QA-3, QA-5.

## Blockers

None blocking the next task. **KNOWN (pre-existing, NOT T4-2.5h): a match-sim proptest flake** — `fw-match-sim` `team_width_when_in_possession_within_band` occasionally fails (in-possession outfield Y-width ~23.98m vs the [24,70]m band floor, ~1.8cm under) on a random proptest seed. Proven unrelated to the career-layer work (zero fw-match-sim code touched; both canonical pins byte-identical). FILED as its own match-engine task (F5/T5-5b territory) — it makes `scripts/fw verify` probabilistically red until the band/behaviour is calibrated. T4-sim-halt closed the desktop FullTime/goal SPAM (2026-06-02). OPEN: match-goal-RATE realism (T5-5b).

Previously: T2-1d2 (xg_utility honesty / cross-band oscillation) re-anchored to Deferred section of MASTER_PLAN. Open EA-scope question resolved 2026-05-29 per DECISIONS.md — DESIGN_DOC `§MVP-scope` anchor added at §8 `### MVP scope`.

## Last green verify

2026-06-03 (T4-F4 close): `scripts/fw verify` exit 0 (the unrelated `fw-match-sim` `team_width` proptest flake did not fire this run). Frontend-only T4-F4 — Player-page scout-report section + the TS `IpcError` union gap (`notYetObserved` + `leagueGenerationFailed`); 274 frontend tests + `tsc` + `pnpm lint` clean; zero Rust touched; **both match-state pins UNCHANGED** (`85f45bf8…`/`206bddae…`). Self-review → Revise + fixes (inverted-playerId-docstrings, queryByText→getByText, category closed-union, leagueGenerationFailed football-native test, AC9 isolation-ordering). KNOWN unrelated: the match-sim `team_width` proptest flake remains filed as its own match-engine task (probabilistically reddens the full verify; not a career-layer regression). Prior: T4-2.5h (stats+Squad); T4-2.5g (SaveV4); T4-F2; T4-F1. Clippy + fmt + determinism-audit + banned-terms + cargo audit/deny clean throughout.

## Last canonical hash

`blake3:85f45bf8ae8821182a45a82969ec36bc5b2d70ba2518b8271de24782fd8064fa` (60-tick, UNCHANGED) + `blake3:206bddaef4df4fec909b9456e2efb04f6c5120ef4104dbdf6aec9665b45b57a9` (600-tick). **600-tick REBASELINED at T4-2.5c (2026-06-02)** — authorized; pillar-5 signature candidates onto role-matched (MID) slots, was slot-7-only; SINGLE-pin (the 60-tick bare-`initial` path is untouched); 5-seed envelope re-verified (pinned 4 goals ∈ [2,5]). The 60-tick pin lives in 3 sites (`canonical_hash.rs` + `0xdeadbeefdeadbeef.ron` + `fw-content/tests/fixtures_load.rs`) — all unchanged this row. Prior: T4-sim-halt rebaselined both. Save wire bytes: SaveEnvelope V0–V4 (V4 = `0x04`, shipped T4-2.5g; current production schema); SettingsEnvelope V0. Next bump: SaveV5 at T4.5-H (world-gen descriptors) / T4.5-E1 (mutable attributes).

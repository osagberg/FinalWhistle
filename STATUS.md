# STATUS — Final Whistle

**Last updated**: 2026-05-29

## Phase

**Phase T4 — Pillar Wiring + Polish — IN PROGRESS.** T4-1/2/3/4/5a/6a + T4-2.5a + T4-sim-halt + T4-2.5b + T4-2.5c (+T1-25/T1-26) + T4-2.5e + T4-2.5d + T4-2.5f + T4-F1 + T4-F2 + T4-2.5g + T4-2.5h + T4-F4 + T4-I1 + QA-3 + QA-5 + T4-2.5i + T4-2.5j + **T4-2.5k** DONE. **The /loop continues through Phase T4, making recommended calls on forks, stopping at the T4 phase boundary or a real blocker.** Pillar 5 is broad (per-signature commentary + 8-of-24 catalogue); Pillar 2's PressReader is now IPC-callable + visible (Career press-inbox panel). T4-2.5k also fixed a pre-existing silent failure: memory callbacks (player-detail/career-overview/press) now render REAL prose instead of a generic fallback. The 600-tick canonical pin was rebaselined at T4-2.5j (user-authorized, envelope-verified); the 60-tick smoke pin is UNCHANGED. T4.5 (World Scale + Content Bake) is the EA-critical phase between T4 and T5.

**2026-06-02 external dual review** correct → persistent → visible all landed; QA rows + Pillar-5 commentary/catalogue + Pillar-2 PressReader IPC done. Remaining T4: **T4-2.5L** (cross-decade callback proof), T4-7, then the T4-8 phase gate. F5/F6 (match-engine) second; C1 at private-flip. **Filed follow-ups:** roster Squad → Player navigation; match-sim `team_width` flake; T4-I1 `import type` reorder; T4-2.5i loader error-distinction + `commentary_line_bank_id`; T4-2.5j inert `trigger: NoOpStub` + body_shield vacuous test + hash-pins fw-tauri pin; T4-2.5k PressTopic::as_dto_str + repo-wide season-display indexing.

## Active task

**T4-2.5L next** (autonomous loop) — Pillar 2: cross-decade callback proof + compaction corpus. In a ≥8-season career, a player-subject event from season 1 that survives the 5-season compaction boundary still renders as non-empty callback prose. Deps T4-2.5e (DONE). `gameplay-programmer`/`qa-lead`.

## Blockers

None blocking the next task. **KNOWN (pre-existing, NOT T4-2.5h): a match-sim proptest flake** — `fw-match-sim` `team_width_when_in_possession_within_band` occasionally fails (in-possession outfield Y-width ~23.98m vs the [24,70]m band floor, ~1.8cm under) on a random proptest seed. Proven unrelated to the career-layer work (zero fw-match-sim code touched; both canonical pins byte-identical). FILED as its own match-engine task (F5/T5-5b territory) — it makes `scripts/fw verify` probabilistically red until the band/behaviour is calibrated. T4-sim-halt closed the desktop FullTime/goal SPAM (2026-06-02). OPEN: match-goal-RATE realism (T5-5b).

Previously: T2-1d2 (xg_utility honesty / cross-band oscillation) re-anchored to Deferred section of MASTER_PLAN. Open EA-scope question resolved 2026-05-29 per DECISIONS.md — DESIGN_DOC `§MVP-scope` anchor added at §8 `### MVP scope`.

## Last green verify

2026-06-03 (T4-2.5k close): `scripts/fw verify` exit 0 — both canonical pins UNCHANGED (`85f45bf8…`/`12ce5ab7…`; PressReader is a pure read-projection). New `get_press_inbox` IPC (top-K-per-topic merge across 4 PressTopics, dedup, salience-ranked, cap 20, roster-name resolution, rendered via the existing render_memory_callback path) + `PressItemDto`/`PressInboxDto` + main.rs registration + `press_inbox_test.rs` (2-season → non-empty incl. a matchResult/TitleWon item). Frontend: TS mirrors + guards + `getPressInbox` + an isolated Career.tsx press-inbox `<section>` + dev fixture (browser-preview-verified, no new console errors). **Bonus bugfix:** `render_memory_callback` was silently falling back to a generic string for EVERY memory callback (empty Tracery slot rejected) — now renders real prose (+ whitespace-collapse, + a regression test). Self-review triple → Revise+fixed (stray-space P1, missing regression test P1, fabricated eventClass 31→9, stale ordering doc, seasonNumber→u16). fw-content 11 memory_callback tests + fw-tauri 3 press_inbox tests + 288 frontend tests green. Prior: T4-2.5j (signature catalogue, pin rebaselined); T4-2.5i (per-signature commentary); QA-5; QA-3. KNOWN unrelated: the match-sim `team_width` flake remains filed. Clippy + fmt + determinism-audit + banned-terms + cargo audit/deny clean throughout.

## Last canonical hash

`blake3:85f45bf8ae8821182a45a82969ec36bc5b2d70ba2518b8271de24782fd8064fa` (60-tick, UNCHANGED) + `blake3:206bddaef4df4fec909b9456e2efb04f6c5120ef4104dbdf6aec9665b45b57a9` (600-tick). **600-tick REBASELINED at T4-2.5c (2026-06-02)** — authorized; pillar-5 signature candidates onto role-matched (MID) slots, was slot-7-only; SINGLE-pin (the 60-tick bare-`initial` path is untouched); 5-seed envelope re-verified (pinned 4 goals ∈ [2,5]). The 60-tick pin lives in 3 sites (`canonical_hash.rs` + `0xdeadbeefdeadbeef.ron` + `fw-content/tests/fixtures_load.rs`) — all unchanged this row. Prior: T4-sim-halt rebaselined both. Save wire bytes: SaveEnvelope V0–V4 (V4 = `0x04`, shipped T4-2.5g; current production schema); SettingsEnvelope V0. Next bump: SaveV5 at T4.5-H (world-gen descriptors) / T4.5-E1 (mutable attributes).

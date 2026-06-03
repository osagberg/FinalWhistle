# STATUS — Final Whistle

**Last updated**: 2026-05-29

## Phase

**Phase T4 — Pillar Wiring + Polish — IN PROGRESS.** T4-1/2/3/4/5a/6a + T4-2.5a + T4-sim-halt + T4-2.5b + T4-2.5c (+T1-25/T1-26) + T4-2.5e + T4-2.5d + T4-2.5f + T4-F1 + T4-F2 + T4-2.5g + T4-2.5h + T4-F4 + T4-I1 + QA-3 + QA-5 + T4-2.5i + T4-2.5j + T4-2.5k + **T4-2.5L** DONE. **The /loop continues through Phase T4; the LAST selectable implementation row is T4-7 (game-shell polish), after which the loop HARD-STOPS at the T4-8 phase gate (no `/done`, no entering T4.5).** All five pillars are wired AND have a falsifiable proof: T4-2.5L proves Pillar 2 cross-decade (a season-1 event survives the 5-season compaction boundary + still renders in season 8), emits RegressiveCollapse at a terminal `CAREER_END_SEASON`, + a 100-ledger compaction-retention corpus. T4-2.5k also fixed a pre-existing silent failure: memory callbacks now render REAL prose instead of a generic fallback. The 600-tick canonical pin was rebaselined at T4-2.5j (user-authorized, envelope-verified); the 60-tick smoke pin is UNCHANGED. T4.5 (World Scale + Content Bake) is the EA-critical phase between T4 and T5.

**2026-06-02 external dual review** correct → persistent → visible all landed; all T4-2.5x pillar-polish + QA rows done. Remaining T4: **T4-7** (game-shell polish — the loop's terminus), then the T4-8 phase gate (Codex review — loop hard-stops here). DEFERRED: T4-2b. F5/F6 (match-engine) second; C1 at private-flip. **Filed follow-ups:** roster Squad → Player navigation; match-sim `team_width` flake; T4-I1 `import type` reorder; T4-2.5i loader error-distinction + `commentary_line_bank_id`; T4-2.5j inert `trigger: NoOpStub` + body_shield vacuous test + hash-pins fw-tauri pin; T4-2.5k PressTopic::as_dto_str + repo-wide season-display indexing.

## Active task

**T4-7 next** (autonomous loop) — Game-shell polish: window chrome (Tauri title-bar customization), main-menu, splash screen, per-OS app-icon. The LAST selectable T4 implementation row (deps T4-3 DONE). `ui-programmer`. After it the loop hard-stops at the T4-8 phase gate.

## Blockers

None blocking the next task. **KNOWN (pre-existing, NOT T4-2.5h): a match-sim proptest flake** — `fw-match-sim` `team_width_when_in_possession_within_band` occasionally fails (in-possession outfield Y-width ~23.98m vs the [24,70]m band floor, ~1.8cm under) on a random proptest seed. Proven unrelated to the career-layer work (zero fw-match-sim code touched; both canonical pins byte-identical). FILED as its own match-engine task (F5/T5-5b territory) — it makes `scripts/fw verify` probabilistically red until the band/behaviour is calibrated. T4-sim-halt closed the desktop FullTime/goal SPAM (2026-06-02). OPEN: match-goal-RATE realism (T5-5b).

Previously: T2-1d2 (xg_utility honesty / cross-band oscillation) re-anchored to Deferred section of MASTER_PLAN. Open EA-scope question resolved 2026-05-29 per DECISIONS.md — DESIGN_DOC `§MVP-scope` anchor added at §8 `### MVP scope`.

## Last green verify

2026-06-03 (T4-2.5L close): `scripts/fw verify` exit 0 — both canonical pins UNCHANGED (`85f45bf8…`/`12ce5ab7…`; career/ledger state, not match canonical). Cross-decade proof `cross_decade_callback_survives_compaction` (inject high-stakes season-1 DebutSenior → 8 seasons → compacted `tick==None` yet still renders non-fallback prose on `/player`) + `regressive_collapse_emitted_at_career_end` (10-season run → RegressiveCollapse in ledger) + QA-1 `compaction_retention_corpus_100_ledgers_10_seasons` (100 synthetic ledgers, 500/500 season-0 events survive in top-N, ≥95% floor). New `CAREER_END_SEASON`=10 terminal RegressiveCollapse emission (most-pressured player; PLACEHOLDER for post-EA retirement; fail-loud log on the unreachable empty-roster path). Self-review triple → Accept + 2 P2 fixes (None-branch fail-loud log; corrected boundary comment). fw-memory corpus + fw-tauri 2 proof tests + 288 frontend tests green. Prior: T4-2.5k (PressReader IPC + render bugfix); T4-2.5j (signature catalogue, pin rebaselined); T4-2.5i. KNOWN unrelated: the match-sim `team_width` flake remains filed. Clippy + fmt + determinism-audit + banned-terms + cargo audit/deny clean throughout.

## Last canonical hash

`blake3:85f45bf8ae8821182a45a82969ec36bc5b2d70ba2518b8271de24782fd8064fa` (60-tick, UNCHANGED) + `blake3:206bddaef4df4fec909b9456e2efb04f6c5120ef4104dbdf6aec9665b45b57a9` (600-tick). **600-tick REBASELINED at T4-2.5c (2026-06-02)** — authorized; pillar-5 signature candidates onto role-matched (MID) slots, was slot-7-only; SINGLE-pin (the 60-tick bare-`initial` path is untouched); 5-seed envelope re-verified (pinned 4 goals ∈ [2,5]). The 60-tick pin lives in 3 sites (`canonical_hash.rs` + `0xdeadbeefdeadbeef.ron` + `fw-content/tests/fixtures_load.rs`) — all unchanged this row. Prior: T4-sim-halt rebaselined both. Save wire bytes: SaveEnvelope V0–V4 (V4 = `0x04`, shipped T4-2.5g; current production schema); SettingsEnvelope V0. Next bump: SaveV5 at T4.5-H (world-gen descriptors) / T4.5-E1 (mutable attributes).

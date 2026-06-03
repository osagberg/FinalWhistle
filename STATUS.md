# STATUS — Final Whistle

**Last updated**: 2026-05-29

## Phase

**Phase T4 — Pillar Wiring + Polish — IN PROGRESS.** T4-1/2/3/4/5a/6a + T4-2.5a + T4-sim-halt + T4-2.5b + T4-2.5c (+T1-25/T1-26) + T4-2.5e + T4-2.5d + T4-2.5f + T4-F1 + T4-F2 + T4-2.5g + T4-2.5h + T4-F4 + T4-I1 + QA-3 + QA-5 + T4-2.5i + **T4-2.5j** DONE. **User is back; the /loop continues through Phase T4, making recommended calls on forks, stopping at the T4 phase boundary or a real blocker.** Pillar 5 is now broad: per-signature commentary (T4-2.5i) + an 8-of-24 signature catalogue, one implemented predicate per role family (T4-2.5j). The 600-tick canonical pin was rebaselined (user-authorized, envelope-verified); the 60-tick smoke pin is UNCHANGED. T4.5 (World Scale + Content Bake) is the EA-critical phase between T4 and T5.

**2026-06-02 external dual review** correct → persistent → visible all landed; QA rows + Pillar-5 commentary + signature catalogue done. Remaining T4: the pillar polish rows (T4-2.5k/L), T4-7, then the T4-8 phase gate. F5/F6 (match-engine) second; C1 at private-flip. **Filed follow-ups:** roster Squad → Player navigation; match-sim `team_width` flake; T4-I1 `import type` reorder; T4-2.5i loader error-distinction + `commentary_line_bank_id`; T4-2.5j inert `trigger: NoOpStub` schema field + pre-existing `body_shield` vacuous test.

## Active task

**T4-2.5k next** (autonomous loop) — Pillar 2: PressReader IPC registration (scope bounded to PressReader; Fan/Coach/Scout panel UIs are post-EA Deferred). Deps T4-2.5e (DONE). `lead-programmer`/`ui-programmer`.

## Blockers

None blocking the next task. **KNOWN (pre-existing, NOT T4-2.5h): a match-sim proptest flake** — `fw-match-sim` `team_width_when_in_possession_within_band` occasionally fails (in-possession outfield Y-width ~23.98m vs the [24,70]m band floor, ~1.8cm under) on a random proptest seed. Proven unrelated to the career-layer work (zero fw-match-sim code touched; both canonical pins byte-identical). FILED as its own match-engine task (F5/T5-5b territory) — it makes `scripts/fw verify` probabilistically red until the band/behaviour is calibrated. T4-sim-halt closed the desktop FullTime/goal SPAM (2026-06-02). OPEN: match-goal-RATE realism (T5-5b).

Previously: T2-1d2 (xg_utility honesty / cross-band oscillation) re-anchored to Deferred section of MASTER_PLAN. Open EA-scope question resolved 2026-05-29 per DECISIONS.md — DESIGN_DOC `§MVP-scope` anchor added at §8 `### MVP scope`.

## Last green verify

2026-06-03 (T4-2.5j close): `scripts/fw verify` exit 0 — 60-tick smoke pin UNCHANGED (`85f45bf8…`); **600-tick extended pin REBASELINED `206bddae…`→`12ce5ab7…`** (authorized; user present chose auto-rebaseline + Claude-verifies-envelope). 5 new pure-Q32 trigger predicates (GK/FullBack/DefMid/Winger/Striker — all 8 role families now ≥1) + 40 unit tests + 5 signature RON + 3 role player-templates wiring all 22 slots; `design/signatures.md` (24 entries: 8 live, 16 doc stubs). MAIN-THREAD independently verified the 5-seed envelope BEFORE re-pinning per the multi-pin rule: pinned `0xfeedbeefcafefade` = 4 goals (in [2,5]); all 5 seeds in [0,7]. Self-review triple → Revise: fixed stale fixture metadata + a doc-comment typo + the doc-lint that initially reddened lint; ADDED a role_family↔predicate-gate guard test (closes a class the slot-hardcoded unit tests can't); deferred 2 items (filed). banned-terms clean; `team_width` passed this run. Prior: T4-2.5i (per-signature commentary); QA-5 (Home.test); QA-3 (seed-diversity); T4-I1 (fixture shim). KNOWN unrelated: the match-sim `team_width` flake remains filed. Clippy + fmt + determinism-audit + cargo audit/deny clean throughout.

## Last canonical hash

`blake3:85f45bf8ae8821182a45a82969ec36bc5b2d70ba2518b8271de24782fd8064fa` (60-tick, UNCHANGED) + `blake3:206bddaef4df4fec909b9456e2efb04f6c5120ef4104dbdf6aec9665b45b57a9` (600-tick). **600-tick REBASELINED at T4-2.5c (2026-06-02)** — authorized; pillar-5 signature candidates onto role-matched (MID) slots, was slot-7-only; SINGLE-pin (the 60-tick bare-`initial` path is untouched); 5-seed envelope re-verified (pinned 4 goals ∈ [2,5]). The 60-tick pin lives in 3 sites (`canonical_hash.rs` + `0xdeadbeefdeadbeef.ron` + `fw-content/tests/fixtures_load.rs`) — all unchanged this row. Prior: T4-sim-halt rebaselined both. Save wire bytes: SaveEnvelope V0–V4 (V4 = `0x04`, shipped T4-2.5g; current production schema); SettingsEnvelope V0. Next bump: SaveV5 at T4.5-H (world-gen descriptors) / T4.5-E1 (mutable attributes).

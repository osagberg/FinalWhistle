# STATUS — Final Whistle

**Last updated**: 2026-05-29

## Phase

**Phase T4 — Pillar Wiring + Polish — IN PROGRESS.** T4-1/2/3/4/5a/6a + T4-2.5a + **T4-sim-halt** DONE. Career-roster layer underway (blueprint `docs/design/career-roster-layer.md`). T4-sim-halt (2026-06-02) made the sim self-halt at `match_end_tick` (default 60→5400 + freeze at FullTime), fixing the desktop FullTime/goal SPAM; both canonical pins rebaselined (authorized, event-only — goal counts unchanged). T4.5 (World Scale + Content Bake) is the new EA-critical phase between T4 and T5 per re-baseline 2026-05-29 — see `docs/MASTER_PLAN.md ## Tier 4.5`.

## Active task

**T4-2.5b next** — roster data model: `CareerState.roster: BTreeMap<ClubId, Vec<PlayerInstance>>` + career-start generation (22-`PlayerTemplate` pool across 20 clubs; distinct `PlayerId`s from `(career_seed, club_idx, slot)`) + `get_roster_for_club` IPC + `PlayerRosterDto`. **Pre-task: log Decision 5 (forward-compat — `BTreeMap<ClubId, Vec<PlayerInstance>>` + `PlayerId` scheme must NOT assume exactly 20 clubs) via `/log-decision` before implementation starts.** `lead-programmer`. Then T4-2.5c/d/e/f (pillar wiring); the calibration track (T5-5b) feeds match-goal-RATE realism (separate from T4-sim-halt's match-LENGTH fix).

## Blockers

None live. Pre-task for T4-2.5b: log Decision 5 (forward-compat clause) via `/log-decision` before it starts. T4-sim-halt closed the desktop FullTime/goal SPAM (2026-06-02). OPEN: match-goal-RATE realism (a real 5400-tick match scores unrealistically many goals until the engine is calibrated — that is T5-5b, layered separately from the fantasy/dev systems).

Previously: T2-1d2 (xg_utility honesty / cross-band oscillation) re-anchored to Deferred section of MASTER_PLAN. Open EA-scope question resolved 2026-05-29 per DECISIONS.md — DESIGN_DOC `§MVP-scope` anchor added at §8 `### MVP scope`.

## Last green verify

2026-06-02 (T4-sim-halt close): `scripts/fw verify` exit 0. fw-match-sim 382 tests + the new `match_halt.rs` (3 halt/freeze/default tests) green; fw-tauri + fw-replay green; both canonical pins rebaselined + verified across all 3 pin sites + 2 fixtures + insta snap. Prior: T4-P2-fixes (4 fixes, no canonical touch) + T4-2.5a (+19 tests). Clippy + fmt + determinism-audit + banned-terms + cargo audit/deny clean throughout.

## Last canonical hash

`blake3:85f45bf8ae8821182a45a82969ec36bc5b2d70ba2518b8271de24782fd8064fa` (60-tick) + `blake3:856a7fede1ab802b88f12a239bdb54e94381348bccfc60c09964d0dfd01dd3fa` (600-tick). **REBASELINED at T4-sim-halt (2026-06-02)** — authorized; `match_end_tick` default 60→5400 + sim self-halts at FullTime; gameplay byte-identical, only `match_events` loses the spurious FullTime; 5-seed envelope re-verified. Pinned in 3 sites: `canonical_hash.rs` ×2 + `fw-content/tests/fixtures_load.rs`. **T4-2.5c will REBASELINE both again** (signature candidates onto all 22 slots). Save wire bytes: SaveEnvelope V0–V3; SettingsEnvelope V0. T4-2.5g adds SaveV4 (`0x04`).

# STATUS — Final Whistle

**Last updated**: 2026-05-29

## Phase

**Phase T4 — Pillar Wiring + Polish — IN PROGRESS.** T4-1/2/3/4/5a/6a + T4-2.5a + **T4-sim-halt** + **T4-2.5b** DONE. Career-roster layer underway (blueprint `docs/design/career-roster-layer.md`). T4-2.5b (2026-06-02) shipped the chokepoint roster data model — `CareerState.roster: BTreeMap<ClubId, Vec<PlayerInstance>>` populated at career start (20×22=440 bijective PlayerIds), `get_roster_for_club` IPC, `generate_league_with_teams` returning per-club ProcGenTeam; Decision 5 (forward-compat, no hardcoded 20 clubs) logged 01c56de. T4.5 (World Scale + Content Bake) is the new EA-critical phase between T4 and T5 per re-baseline 2026-05-29 — see `docs/MASTER_PLAN.md ## Tier 4.5`.

## Active task

**T4-2.5c next** (+ T1-25 + T1-26) — pillar-5 signatures onto all 22 match slots: `play_one_match`/`initial_with_content` take a `slot_signatures: BTreeMap<PlayerSlot, Vec<SignatureCandidate>>` built from both clubs' rosters (replaces slot-7-only); promote T1-25/T1-26 dispatch-hardening tests. **Authorized canonical-hash rebaseline of BOTH pins** (signature candidates on all 22 slots is a behavior change — follow the multi-pin discipline: main-thread review + 5-seed envelope verify BEFORE rebaselining). `gameplay-programmer`. Parallel-eligible after T4-2.5b: T4-2.5d (pillar-3 breakthrough), T4-2.5e (pillar-2 memory events), T4-2.5f (pillar-4 scouting), QA-3. Match-goal-RATE realism stays T5-5b.

## Blockers

None live. Pre-task for T4-2.5b: log Decision 5 (forward-compat clause) via `/log-decision` before it starts. T4-sim-halt closed the desktop FullTime/goal SPAM (2026-06-02). OPEN: match-goal-RATE realism (a real 5400-tick match scores unrealistically many goals until the engine is calibrated — that is T5-5b, layered separately from the fantasy/dev systems).

Previously: T2-1d2 (xg_utility honesty / cross-band oscillation) re-anchored to Deferred section of MASTER_PLAN. Open EA-scope question resolved 2026-05-29 per DECISIONS.md — DESIGN_DOC `§MVP-scope` anchor added at §8 `### MVP scope`.

## Last green verify

2026-06-02 (T4-2.5b close): `scripts/fw verify` exit 0. NEW `fw-tauri` `roster.rs`/`roster_dto.rs` (PlayerInstance/PlayerSeasonStats/PlayerRosterDto + bijective-PlayerId/forward-compat/IPC tests) + `generate_league_with_teams` (fw-content); `pnpm test` 265 + `pnpm tsc` clean. **Canonical pins UNCHANGED** (roster is non-canonical fw-tauri state). Prior: T4-sim-halt (both pins rebaselined; +match_halt.rs); T4-P2-fixes; T4-2.5a. Clippy + fmt + determinism-audit + banned-terms + cargo audit/deny clean throughout.

## Last canonical hash

`blake3:85f45bf8ae8821182a45a82969ec36bc5b2d70ba2518b8271de24782fd8064fa` (60-tick) + `blake3:856a7fede1ab802b88f12a239bdb54e94381348bccfc60c09964d0dfd01dd3fa` (600-tick). **REBASELINED at T4-sim-halt (2026-06-02)** — authorized; `match_end_tick` default 60→5400 + sim self-halts at FullTime; gameplay byte-identical, only `match_events` loses the spurious FullTime; 5-seed envelope re-verified. Pinned in 3 sites: `canonical_hash.rs` ×2 + `fw-content/tests/fixtures_load.rs`. **T4-2.5c will REBASELINE both again** (signature candidates onto all 22 slots). Save wire bytes: SaveEnvelope V0–V3; SettingsEnvelope V0. T4-2.5g adds SaveV4 (`0x04`).

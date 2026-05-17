# STATUS — Final Whistle

**Last updated**: 2026-05-18

## Phase

**T2 in progress. T2-8 (Frontend Transfer-window stub) closed 2026-05-18.** T2 progress: 9 of 10 MVP rows DONE (T2-1a/b/c/d-infra + T2-1-codex-fix + T2-2 + T2-3 + T2-5 + T2-6 + T2-8). T2-1d2 DEFERRED to end-of-T2; T2-4 BLOCKED on `design/player-generation.md`; T2-7 transitively blocked on T2-4. **One MVP row remains**: T2-9 (`fw-save` bincode + version-migration enum chain).

T2-8 ships a UI shell stub: pure `computeTransferWindowState(matchDay) → WindowState` discriminated-union helper + rewritten Transfers route that derives window-open/closed state from T2-5's `getStandings()` IPC. FM-style two-window calendar (match-day 0 → Summer; 1-18 → Closed; 19-20 → Winter; 21-38 → Closed). Self-review triple verdicts: silent-failure-hunter REVISE (3 P1 silent-failure-laundering chain — try/catch swallowing → `[]` → "Summer" lie; all 3 fixed in-place via explicit error arm + `number | null` distinction + throw-on-invalid-input); type-design REVISE (1 P2 fixed inline via constant naming + 4 P2/P3 deferred); code-reviewer REVISE (1 P1 "Phase T3" internal-dev-label leaking into player-facing UI — fixed with football-vernacular copy). 15 new vitest tests; 87 frontend tests pass.

**Canonical hashes UNCHANGED on both pins** — frontend-only work.

## Active task

(none — T2-8 closed at this commit; `scripts/fw verify` exit 0; **canonical hashes UNCHANGED on both pins**. Next `/next` picks **T2-9** (`fw-save` bincode-based save format + version-migration enum chain — first schema version locked at 1; save → load → byte-identical state). Deps T2-5 (DONE). Likely a `lead-programmer` task class — new crate surface + serialization contracts. **T2-7 (Squad page) STILL BLOCKED** on T2-4 (BLOCKED on missing `design/player-generation.md` design doc). 15 commits ahead of origin/main waiting to push.)

## Phase pointer

- **Just landed:** **T2-8** — NEW `frontend/src/lib/transfer-window.ts` (~60 LoC: pure helper with discriminated-union return + throw-on-invalid) + NEW `transfer-window.test.ts` (~75 LoC, 10 unit tests). REWRITTEN `frontend/src/routes/Transfers.tsx` (~135 LoC) with createResource + explicit error/null/data arms + describeStandingsError that unwraps Solid's `castError`. NEW `frontend/src/routes/Transfers.test.tsx` (~135 LoC, 6 smoke tests).
- **Next:** **T2-9** — `fw-save`: bincode-based save format + version-migration enum chain. First schema version locked at `1`. Deps T2-5 (DONE). Likely involves authoring `crates/fw-save/src/v1.rs` schema struct + a `SaveError` enum + migration trait + 4 mandatory tests per `design/specs/save-migration-fixtures.md` (forward-migration, callback-preservation, forward-incompat-failure, round-trip-byte-identical). **Deferred follow-ups (`/next` skips)**: T1-17, T1-25..T1-28, T2-1d2, T4-9. **Effectively-blocked**: T2-4 awaits `design/player-generation.md` authorship; T2-7 transitively blocked. **Carry-forward known follow-ups from T2-8 self-review (P2/P3)**: drop `WindowState.label` literal-string field for i18n-readiness; introduce `MatchDay` newtype on the `currentMatchDay: number` parameter; extract `SUMMER_END_MATCHDAY` / `WINTER_WINDOW_START` / `WINTER_WINDOW_END` / `SEASON_LENGTH_MATCHDAYS` named constants to a season-calendar module; `WindowState.closed` over-collapses three semantically distinct cases (mid-half-1-closed / mid-half-2-closed / season-complete) — flag for `nextOpens: MatchDay` field at T3. Plus the still-standing T2-6 / T2-5 / T2-3 carry-forwards (extract `lib/ipc-error.ts` to dedupe Match/League/Transfers trio; SeasonState API surface tightening; StandingsRow.goal_difference cached vs derived; MatchOutcome could use Score; matches_played width inconsistency; season_read/write helpers; BakeManifest.output_path: PathBuf; model_id as enum; workspace-hoist Cargo deps).

## Blockers

- **T2-4 (`PlayerBio` generation) BLOCKED** on missing `design/player-generation.md`. Resolve via `/log-decision` ADR authoring OR external design-doc authoring OR explicit one-paragraph spec promoting T2-4 to TODO with the 22 gene fields named.
- **T2-7 (Squad page) transitively blocked** by T2-4.

## Last green verify

2026-05-18 (T2-8 close): `scripts/fw verify` exit 0 (cargo fmt + clippy + cargo test --workspace + pnpm test 87 frontend + pnpm typecheck + banned-terms + canonical-hash regression on both pins UNCHANGED + content-pack validate-structural + hash-pins atomicity test + cargo audit + cargo deny). 15 new frontend tests added.

## Last canonical hash

`blake3:eaf842ac3d19651d38dc7ce45d0763cc62b4d571ce2c2a5d56f1ee3c6ddead46` (60-tick smoke seed; UNCHANGED from T2-1b rebaseline — frontend-only work; sim crates untouched).

**Second corpus pin:** `blake3:aa7efe9b2a567d5e87d12c7da6a4ea928271429729884f38819baed85c3be5ae` (600-tick extended seed; UNCHANGED from T2-1-codex-fix rebaseline).

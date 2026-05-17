# STATUS — Final Whistle

**Last updated**: 2026-05-18

## Phase

**T2 in progress. T2-6 (Frontend League page) closed 2026-05-18.** Selected ahead of T2-4 (still BLOCKED on `design/player-generation.md`). T2 progress: 8 of 10 MVP rows DONE (T2-1a/b/c/d-infra + T2-1-codex-fix + T2-2 + T2-3 + T2-5 + T2-6). T2-1d2 DEFERRED to end-of-T2; T2-4 BLOCKED on design-doc gap.

T2-6 ships the full League page: 10-column TanStack v8 standings table (`#` / Club / P / W / D / L / GF / GA / GD / Pts), season action buttons (`Advance Week`, `Play Fixtures` fast-forward) consuming T2-5's IPC commands, typed `ActionOutcome` signal for success-OR-failure feedback, IpcError narrowing with exhaustive switch + `throw` on variant drift, position-column visual re-numbering via the canonical TanStack v8 `findIndex(r => r.id === row.id) + 1` pattern, dark+light mode tokens throughout. Self-review triple verdicts: silent-failure-hunter REVISE (1 P0 position-column + 4 P1 + 1 P1-edge → all 6 fixed in-place); type-design REVISE (4 P2/P3 deferred); code-reviewer REVISE (1 P1 `aria-label` anti-pattern → fixed). 13 vitest tests (4 added for sort coverage post-self-review). **Canonical hashes UNCHANGED on both pins** — frontend-only work.

## Active task

(none — T2-6 closed at this commit; `scripts/fw verify` exit 0; **canonical hashes UNCHANGED on both pins**. Next `/next` likely picks **T2-8** (Frontend Transfer-window stub — UI shell only; deps T2-6 DONE) OR **T2-9** (`fw-save` bincode + version migration chain; deps T2-5 DONE). T2-7 (Squad page) STILL BLOCKED on T2-4 (PlayerBio) which is BLOCKED on `design/player-generation.md` authorship. 14 commits ahead of origin/main waiting to push.)

## Phase pointer

- **Just landed:** **T2-6** — `frontend/src/routes/League.tsx` rewritten (~340 LoC) with full standings + action-buttons + outcome feedback. NEW `frontend/src/lib/columns/league.columns.ts` (~62 LoC) with 10 ColumnDef entries — position column uses `findIndex` by stable row.id for visual re-numbering. NEW `frontend/src/routes/League.test.tsx` (~430 LoC) with 13 vitest tests including 3 sort-coverage tests added during self-review fix-pass.
- **Next:** Most-likely **T2-8** (Frontend Transfer-window stub — UI shell only, no transfer mechanics, "window opens/closes" state visible). Deps T2-6 (DONE). Likely `ui-programmer` task. Alternative is **T2-9** (`fw-save` bincode-based save format + version-migration enum chain; deps T2-5 DONE) which would set up the Save pillar work earlier. **Deferred follow-ups (status `DEFERRED` — `/next` skips)**: T1-17, T1-25, T1-26, T1-27, T1-28, T2-1d2, T4-9. **Effectively-blocked**: T2-4 (PlayerBio) awaits `design/player-generation.md` design-doc authorship via `/log-decision`; T2-7 (Squad page) depends transitively on T2-4. **Carry-forward known follow-ups from T2-6 self-review (P2/P3)**: F1 extract `frontend/src/lib/ipc-error.ts` to dedupe the Match.tsx + League.tsx KNOWN_IPC_ERROR_KINDS / isIpcError / formatIpcError trio; F4 spawn a follow-up for `SeasonStateDto` IPC command to decouple "current match-day" from per-club `played` field (currently `matchDayHeader` derives match-day from first-row `played` — works but conflates league-level with row-level facts); F2-partial: `actionError` already converted to typed `ActionOutcome | null`, but `Match.tsx` still has the same `unknown` pattern (out of scope here; could be folded into F1's extraction). T2-5 carry-forwards still standing: SeasonState API surface tightening; StandingsRow.goal_difference cached vs derived; MatchOutcome could use Score; Standings(Vec) wrapper ceremony; matches_played u16/u32 inconsistency; season_read/write helpers; mid-loop fixture-count truth structured variant; #[tauri::command] wrapper end-to-end coverage. T2-3 carry-forwards still standing: BakeManifest.output_path: PathBuf; model_id as enum; MIN_NAME_BANK_SIZE const dedup; Culture invariant in validator vs type; workspace-hoist clap/tokio/reqwest/jsonschema.

## Blockers

- **T2-4 (`PlayerBio` generation) BLOCKED** on missing `design/player-generation.md` design doc. The 22-field gene model + phenotype-label catalog + names→personality mapping are substantive content/balance design calls. Resolve via `/log-decision` authoring an ADR + design-doc; OR by user authorship of `design/player-generation.md` outside of `/next`; OR by promoting T2-4 to TODO with a one-paragraph spec that names the 22 gene fields + phenotype labels explicitly.
- **T2-7 (Squad page) transitively blocked** by T2-4.

## Last green verify

2026-05-18 (T2-6 close): `scripts/fw verify` exit 0 (cargo fmt + clippy + cargo test --workspace + pnpm test 69 frontend + pnpm typecheck + banned-terms + canonical-hash regression on both pins UNCHANGED + content-pack validate-structural + hash-pins atomicity test + cargo audit + cargo deny). 13 new frontend tests added.

## Last canonical hash

`blake3:eaf842ac3d19651d38dc7ce45d0763cc62b4d571ce2c2a5d56f1ee3c6ddead46` (60-tick smoke seed; UNCHANGED from T2-1b rebaseline — frontend-only work; sim crates untouched).

**Second corpus pin:** `blake3:aa7efe9b2a567d5e87d12c7da6a4ea928271429729884f38819baed85c3be5ae` (600-tick extended seed; UNCHANGED from T2-1-codex-fix rebaseline).

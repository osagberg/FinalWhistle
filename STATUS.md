# STATUS — Final Whistle

**Last updated**: 2026-05-22

## Phase

**Phase T4 — Beautiful UI + Tactical Viewer — IN PROGRESS.** Phase T3 (Career + Memory) CLOSED 2026-05-21 (Codex phase-gate ACCEPT, tag `v0.3.0-career`); the post-T3 cleanup cluster T3-R-A..F all DONE. T4 began 2026-05-21 — **T4-1 (PixiJS tactical board) + T4-2 (ECharts per-team stat dashboard) + T4-3 (visual-identity lock) + T4-4 (loading / empty / error states) + T4-5a (ADR-0004 live-match IPC quintet) DONE.** T4 ships 8 MVP rows (T4-1..T4-8) + a Stretch T4-9; the T4 Exit Gate is locked in `docs/MASTER_PLAN.md`.

## Active task

(none — T4-5a (ADR-0004 live-match IPC quintet) closed 2026-05-22; new `fw-tauri::live_match` module + 5 IPC commands + `LiveMatchSession` handle store + closed 9-variant `MatchCommand` + `MatchSnapshot` DTO; AC4 determinism-equivalence holds (step×N == batched `play_match(N)`); `apply_match_command` fail-loud-stubbed. T4-5b (frontend live UI + salience jump) split off DEFERRED. Next `/next` picks **T4-6** — settings screen.)

## Blockers

- **T2-1d2** rolled through T3 as `DEFERRED-ROLLED-TO-T3`; not promoted. Carry into T4 — schedule alongside the end-of-T-phase rebalance pass per `personality-bias-weights.md §Re-tuning cadence`.

## Last green verify

2026-05-22 (T4-5a close): `scripts/fw verify` exit 0; full workspace + 60 fw-tauri tests (9 new live-match integration incl. AC4 determinism-equivalence) + 248 frontend tests; clippy + eslint + tsc + banned-terms + determinism-audit clean; canonical match-state hashes UNCHANGED on both pins.

## Last canonical hash

`blake3:eaf842ac3d19651d38dc7ce45d0763cc62b4d571ce2c2a5d56f1ee3c6ddead46` (60-tick smoke seed; T2-1b rebaseline; UNCHANGED through T3).

**Second corpus pin:** `blake3:aa7efe9b2a567d5e87d12c7da6a4ea928271429729884f38819baed85c3be5ae` (600-tick extended seed; T2-1-codex-fix rebaseline; UNCHANGED through T3).

**Save-format wire-byte pins** (T2-9 + T3-1 + T3-R-E): SaveEnvelope V0=0x00 / V1=0x01 / V2=0x02 / V3=0x03. Locked FOREVER. V3 is the current production schema (career-state persistence).

## Phase T4 notes

T4 carry-ins: T2-1d2 (see Blockers); wiring breakthrough + scout into a played career (needs the T4+ career-roster layer — the genuine T4-scale follow-up the post-T3 review surfaced). T3-R-F's `current_tick()` salience-decay path + the breakthrough/scout emitters remain unwired infrastructure until that career-roster layer lands.

**Audits:** `docs/audits/post-t3-ultimate-review-2026-05-21.md`, `docs/audits/post-t3-codex-gate-2026-05-21.md`.

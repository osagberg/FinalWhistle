# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 — First Match.** **The Codex 2026-05-16 audit-triage P1 sweep is COMPLETE — 5 of 5 P1s closed.** T1-13 shipped: 2 new frontend test files (`FrameSource.test.ts` 19 tests + `TacticalBoard.test.tsx` 4 tests) bringing total frontend coverage to 3 files / 34 tests / ~830ms (12x headroom on the <10s acceptance budget); new `deny.toml` with permissive+LGPL license allowlist + RUSTSEC vulnerability gate + bincode `RUSTSEC-2025-0141` documented ignore; `Justfile` extended with `frontend-test` + `audit` + `deny` recipes wired into `ci-local` (and therefore `scripts/fw verify`); `ci.yml` adds `pnpm test` step in build-test + 2 new parallel `cargo-audit` + `cargo-deny` jobs on ubuntu-22.04. Plus `Cargo.toml` workspace license bumped to SPDX-valid `LicenseRef-Proprietary`. Main-thread fix-pass closed 2 P2 type-design findings (shared `FwDevApi` declaration in new `frontend/src/lib/fw-dev.d.ts`; `MatchFrameDTO` return-type annotations on test fixtures for drift detection). **5 clean silent-failure-hunter verdicts in a row** (T1-11 + T1-12 + T1-10 + T1-13 silent-failure + T1-13 code-reviewer). T1 phase remaining: T1-7 procgen + T1-8 replay corpus + T1-9 behavioral assertions (all non-P1 polish).

## Active task

(none — T1-13 closed + full Codex audit-triage P1 sweep complete; T1-7 / T1-8 / T1-9 are the remaining T1 polish rows)

## Phase pointer

- **Just closed:** **T1-13** — Frontend test scaffolding broadening + CI gating + cargo audit/deny wiring. 5 chunks (FrameSource tests + TacticalBoard lifecycle/fwDev tests + deny.toml + Justfile + ci.yml). Fix-pass: shared `FwDevApi` type declaration + `MatchFrameDTO` fixture annotations. **5th + FINAL audit-triage P1 closure.**
- **Next:** **T1-7** per MASTER_PLAN order — Procedural content stub: 22 player names (Markov chain seeded by region prior) + 2 team names + 1 manager archetype RON port. First non-P1 row; substantive content-generation work. OR **T1-9** (behavioral assertions — proptest invariants for positional invariants + PlayerSeparation + knob-isolation tests + events_chronological per ADR-0007) if QA-shaped work is preferred over content-shaped.
- **Recommended /next order** post-audit-sweep: **T1-7** (procgen) → T1-9 (behavioral assertions) → T1-8 (replay corpus fixture). Three remaining T1 rows; all non-P1; pick based on what feels most valuable next. After T1-9 the T1 phase should be ready for `/done` + the Codex phase-boundary review.

## Blockers

None.

## Last green verify

2026-05-16 — `scripts/fw verify` clean post T1-13 + fix-pass: fmt + clippy + cargo test --workspace + `pnpm test` (34 tests / 3 files / ~830ms) + canonical-hash regression on `782fcde6…8c0f` + banned-terms + determinism-audit + fw-content-baker validate + `cargo audit` (0 vulnerabilities + 19 transitive GTK3 unmaintained warnings, acceptable risk) + `cargo deny check` (advisories ok + bans ok + licenses ok + sources ok).

## Last canonical hash

`blake3:782fcde65ba8a0fc12bb90af1b61f77d8cd403103ab3671b0d5d6b03e75c8c0f` (60-tick smoke seed; UNCHANGED across the entire audit-triage P1 sweep — T1-3.5/T1-4a/T1-4b/T1-11/T1-12/T1-5/T1-6/Codex-fix-pass/T1-10/T1-13 all kept it stable).

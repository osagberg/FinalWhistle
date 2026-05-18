# STATUS — Final Whistle

**Last updated**: 2026-05-18

## Phase

**Phase T2 (League + Season) FULLY CLOSED 2026-05-18.** Codex Tier-3 phase-boundary review posted to issue #1: REVISE-but-narrow → phase ACCEPTED, with ONE constraint on T3-1 (SaveV2 alongside V1, never mutate V1 in place). T2-10 marked DONE; tag `v0.2.0-season` finalized at `a0c03d5f`. **Phase T3 (Career + Memory) opens.** First task: T3-1 with the Codex SaveV2 constraint baked into the row body.

## Active task

(none — Phase T2 closed; T3 opens; next `/next` picks T3-1 OR the pre-T3 cleanup rows R1-R9 from `docs/audits/post-t2-ultimate-review-2026-05-18.md`)

## Blockers

- **T2-4 + T2-7** still BLOCKED (rolled to T3 as `DEFERRED-ROLLED-TO-T3`); awaits `design/player-generation.md` authorship via `/log-decision` ADR. T3-5 (scout-uncertainty) + T3-6 (Player detail with memory callbacks) transitively block.
- **T2-1d2** rolled to T3 as `DEFERRED-ROLLED-TO-T3`; promote at T3 midpoint after BT-runner matures per `personality-bias-weights.md` cadence.
- 9 pre-T3 cleanup rows (R1-R9) advised in the ultimate-review audit file; not phase-blockers but recommended before T3-1's first `/next`.

## Last green verify

2026-05-18 (post-Codex-ack): `scripts/fw verify` exit 0; full-season perf test 380 matches / 0.358s release; CI matrix green on all 5 jobs (ubuntu-22.04 + macos-14 + windows-latest + cargo-deny + cargo-audit); Determinism Gate green.

## Last canonical hash

`blake3:eaf842ac3d19651d38dc7ce45d0763cc62b4d571ce2c2a5d56f1ee3c6ddead46` (60-tick smoke seed; T2-1b rebaseline; UNCHANGED through phase close).

**Second corpus pin:** `blake3:aa7efe9b2a567d5e87d12c7da6a4ea928271429729884f38819baed85c3be5ae` (600-tick extended seed; T2-1-codex-fix rebaseline; UNCHANGED through phase close).

## Phase T3 pointer

**First task (T3-1):** `fw-memory` ledger storage + persistence — append-only `MemoryEvent` records keyed by `event_id`; port schema from `adr-0004-memory-event-schema.md`. **Carries the Codex Tier-3 phase-gate constraint: author `SaveV2` ALONGSIDE V1 (never mutate V1 in place) + migrate-on-load V1→V2 + wire-byte regression test pinning V2 at `0x02`.** Deps T2-9 (DONE).

**Pre-T3 cleanup backlog (9 rows; ordered by value):**
- **R7** (~2h) — Codex E + F follow-ups (SetPieceKind canonical-tag pinning; semantic content validator; determinism-audit per-rule exemption; F-1 rare-seed width fix; 3 missing proptests)
- **R1** (~1-2h) — tactic-fsm + personality-bias doc-honesty pass (B-1..B-4)
- **R2** (~1h) — vacuous-constant test pins + 3 misc test-quality fixes (A-1..A-6 + D-4 + D-7 + D-8)
- **R3** (~30 min) — personality_bias.rs `debug_assert!` → `assert!` (Sim/RULES.md §11 hardening)
- **R4** (~30 min) — fw-content-baker validate-structural empty-corpus false-positive
- **R8 / R9** (opportunistic + doc-only) — tactic_fsm_proptest generator narrowing + Content/RULES.md §6 future-tense fix
- **R5 / R6** (opportunistic) — sibling silent-failure cleanups + NotImplemented validator-test TODO markers

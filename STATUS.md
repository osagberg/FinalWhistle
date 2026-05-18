# STATUS — Final Whistle

**Last updated**: 2026-05-18

## Phase

**Phase T3 (Career + Memory) IN PROGRESS.** T3-1 closed 2026-05-18 (first task of T3): `fw-memory` ADR-0005 schema port + `fw-save` SaveV2 alongside V1 per Codex Tier-3 phase-gate constraint. Schema lock complete; ADR-0005 moved to Accepted. Pre-T3 cleanup backlog R-rows all closed prior to T3-1 (see commits `2c38fb9..1e980e3`).

## Active task

(none — T3-1 closed; next `/next` picks T3-2 — 5 readers (alumni-DB / rival-recall / promise-tracking / big-match-scars / press-fan-callbacks). Deps T3-1 DONE.)

## Blockers

- **T2-4 + T2-7 + T3-5** still `DEFERRED-ROLLED-TO-T3` per the /done roll-forward. PROMOTABLE at any /next; T2-4 unblock landed at 2026-05-18 via `design/player-generation.md` port.
- **T2-1d2** rolled to T3 as `DEFERRED-ROLLED-TO-T3`; promote at T3 midpoint after BT-runner matures per `personality-bias-weights.md` cadence.

## Last green verify

2026-05-18 (post-T3-1): `scripts/fw verify` exit 0; 1000-event ledger round-trip well under 100ms in release; 38 new tests across fw-memory + fw-save; canonical match-state hashes UNCHANGED on both pins (fw-memory + fw-save are non-canonical-state-pin path).

## Last canonical hash

`blake3:eaf842ac3d19651d38dc7ce45d0763cc62b4d571ce2c2a5d56f1ee3c6ddead46` (60-tick smoke seed; T2-1b rebaseline; UNCHANGED through T3-1).

**Second corpus pin:** `blake3:aa7efe9b2a567d5e87d12c7da6a4ea928271429729884f38819baed85c3be5ae` (600-tick extended seed; T2-1-codex-fix rebaseline; UNCHANGED through T3-1).

**Save-format wire-byte pins** (T2-9 + T3-1): SaveEnvelope V0=0x00 / V1=0x01 / V2=0x02. Locked FOREVER.

## Phase T3 pointer

**Next task (T3-2):** `fw-memory` 5 readers — `SalienceReader` / `PressReader` / `FanReader` / `ScoutReader` / `CoachReader`. Each reader has ≥3 unit tests + one integration test against a seeded multi-season ledger. Per ADR-0005 §"The five readers" — read-only projections; lazy-rebuilt BTreeMap indexes (`by_subject`/`by_club`/`by_class_season`) are already in place from T3-1.

**Remaining T3 rows (7):** T3-2 (5 readers), T3-3 (news headlines + manager-quote templates via Tracery), T3-4 (breakthrough events — signature awakening + latent-flag unlock + regressive collapse triggers), T3-5 (scout-uncertainty model — single-scout-report variant), T3-6 (Frontend Player detail page with memory callbacks), T3-7 (save migration 4-test discipline for V1→V2 — partially complete via T3-1; T3-7 acts as the formal verifier), T3-8 (phase-gate Codex review #2).

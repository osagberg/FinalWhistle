# STATUS — Final Whistle

**Last updated**: 2026-05-18

## Phase

**Phase T3 (Career + Memory) IN PROGRESS.** T3-1 (schema + SaveV2), T3-2 (5 ledger readers), T3-3 (news render) closed 2026-05-18; T3-4 (breakthrough mechanism — pillar 3) closed 2026-05-20; T3-7 (save-migration committed-fixture verifier) closed 2026-05-21. 5 of 8 T3 rows DONE. The 3 remaining: T3-5 + T3-6 (both dep-blocked on DEFERRED rows), T3-8 (phase-gate Codex review).

## Active task

(none — T3-7 closed. **Phase T3 has no cleanly-eligible `/next` row left:** T3-5 dep T2-4 + T3-6 dep T2-7 are both `DEFERRED-ROLLED-TO-T3`; T3-8 dep is T3-7 (now DONE) but T3-8 IS the phase-gate Codex review — a `/done`-class step, not a `/next` implementation row. Next move is a user decision — see Phase T3 pointer.)

## Blockers

- **T2-4 + T2-7 + T3-5** still `DEFERRED-ROLLED-TO-T3` per the /done roll-forward. PROMOTABLE at any /next; T2-4 unblock landed at 2026-05-18 via `design/player-generation.md` port.
- **T2-1d2** rolled to T3 as `DEFERRED-ROLLED-TO-T3`; promote at T3 midpoint after BT-runner matures per `personality-bias-weights.md` cadence.

## Last green verify

2026-05-21 (post-T3-7): `scripts/fw verify` exit 0; 29 fw-save tests (incl 5 committed-fixture migration verifiers); clippy clean; canonical match-state hashes UNCHANGED on both pins.

## Last canonical hash

`blake3:eaf842ac3d19651d38dc7ce45d0763cc62b4d571ce2c2a5d56f1ee3c6ddead46` (60-tick smoke seed; T2-1b rebaseline; UNCHANGED through T3-1).

**Second corpus pin:** `blake3:aa7efe9b2a567d5e87d12c7da6a4ea928271429729884f38819baed85c3be5ae` (600-tick extended seed; T2-1-codex-fix rebaseline; UNCHANGED through T3-1).

**Save-format wire-byte pins** (T2-9 + T3-1): SaveEnvelope V0=0x00 / V1=0x01 / V2=0x02. Locked FOREVER.

## Phase T3 pointer

**No cleanly-eligible `/next` row remains in T3 — user decision needed.** The 3 open rows:
- **T3-5** (`fw-scouting` scout-uncertainty model) — dep T2-4 is `DEFERRED-ROLLED-TO-T3`. T2-4's blocker (`design/player-generation.md`) was resolved 2026-05-18; T2-4 is promotable to TODO.
- **T3-6** (Frontend Player detail page) — dep T2-7 is `DEFERRED-ROLLED-TO-T3`. T2-7's blocker also resolved 2026-05-18; promotable alongside T2-4.
- **T3-8** (phase-gate Codex review #2) — dep T3-7 is now DONE, so T3-8 is technically eligible, but T3-8 IS the phase-close Codex review — a `/done`-class step, not a `/next` implementation row.

**Options for the user:** (a) promote T2-4 + T2-7 back to TODO (their blockers are resolved) so `/next` can ship the PlayerBio-generation + Squad-page + scouting + player-detail rows; (b) run `/done` to close Phase T3 now with T3-5/T3-6 carried forward as DEFERRED — but T3-5/T3-6 are MVP rows, so closing without them is a scope cut; (c) decide T3-5/T3-6 order. The honest read: T3 is not really complete — 2 MVP rows (scouting, player-detail UI) are unbuilt because their PlayerBio dependency was rolled forward. Promoting T2-4/T2-7 is the path to a genuinely-complete T3.

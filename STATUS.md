# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 — First Match.** T1-2b sub-phase + T1-2b-fix audit-remediation closed. T1-4a (MatchEvent emission, sim side) shipped. Codex 2026-05-16 whole-codebase audit landed REVISE — 1 P0 (ball mutation) + 8 P1s + 6 backlog; triaged into new row T1-3.5 + 4 hardening rows T1-10..T1-13. T1-4b reordered behind T1-3.5 so commentary renders real outcomes, not intentions.

## Active task

(none — Codex audit triage decision logged + MASTER_PLAN revised. `/next` will pick T1-3.5 (ball mutation P0) per the audit-recommended order.)

## Phase pointer

- **Just closed:** **Codex 2026-05-16 audit triage + MASTER_PLAN revision.** Two `docs/DECISIONS.md` entries appended (ADR-0009 amendment for `SeedLayer::Commentary 0x18` + audit-triage decision). MASTER_PLAN grew from 13 to 23 T1 rows: T1-3.5 (ball-mutation P0) inserted before T1-4b; T1-10/11/12/13 hardening rows appended; T1-5 scope amended to fold in IPC consolidation + match_frames cap; T0-7b flipped TODO → DONE (state-doc-drift fix).
- **Next:** **T1-3.5** — Ball mutation + possession state + goal detection. Closes Codex's P0 finding: `apply_intent` currently treats Shot/Pass/Cross/LayOff/Dribble as "move player toward target" with no ball.vel mutation, no possession transfer, no goal detection. Adds canonical `possession: Option<PlayerSlot>` + `last_touched_by: Option<PlayerSlot>` to MatchState; ball-physics mutation in apply_intent; goal detection at tick-end with `MatchEvent::Goal` emission + score bump + new KickOff. Canonical hash REBASELINE expected. After T1-3.5: T1-4b (Tracery commentary templates; was queued first but reordered behind ball-mutation per Codex's recommended order so commentary describes real outcomes).
- **Recommended /next order** (per audit triage decision): T1-3.5 → T1-4b → T1-11 (signature wiring into tick_match) → T1-5 (Tauri + IPC consolidation) → T1-12 (content validation) → T1-10 (LUT bake) → T1-13 (frontend tests + cargo audit) → T1-6 → T1-7 → T1-8 → T1-9.

## Blockers

None. T1-4b's `SeedLayer::Commentary` prereq logged at `docs/DECISIONS.md` 2026-05-16; T1-4b's dep upgraded to also include T1-3.5.

## Last green verify

2026-05-16 — `scripts/fw verify` clean post T1-4a + fix-pass: fmt + clippy + `cargo test --workspace` + release-mode canonical-hash regression on `02ab97d0…27e686` + banned-terms + determinism-audit + `fw-content-baker validate`.

## Last canonical hash

`blake3:02ab97d06e60f508f5076aa37cf371263c73d5fc104ab1448989cb5f5627e686` (60-tick smoke seed; pinned at T1-4a per ADR-0012 trigger #1 — encoder VERSION 6→7 schema bump for MatchState's new match_events + match_end_tick + signature_memory_events removal). T1-3.5 will rebaseline again when possession + ball-mutation fields land.

# STATUS — Final Whistle

**Last updated**: 2026-05-13

## Phase

**T1 — First Match** (active; T1-1 closed; Codex pre-T1-2b re-audit GREEN; T1-2a unblocked, awaiting CI green on HEAD)

## Active task

(none — re-audit cleared. After full CI on HEAD goes green, `/next` picks T1-2a.)

## Phase pointer

- **Just closed:** Codex pre-T1-2b re-audit pass #3 (GREEN, no new P0/P1). Pass #1 closed 6 of 7 prior P1s at `80c53f76`. Pass #2 closed the remaining 3 P1s + the xG P2 + state-pointer drift + ADR-0012 wording at `5bb0939d`. Pass #3 confirms: design + spec audit clean, determinism floor passes, ADR coherence pass, no new findings.
- **Now:** Final CI verification on HEAD. Determinism Gate green; full CI was cancelled on `5bb0939d` because the new fw-content-baker validate step's release build pushed Ubuntu over the 20m timeout. This commit drops `--release` from that step (validate is a static check; debug build is fast). After this push, CI re-runs.
- **Next:** Once CI is green on HEAD, `/next` picks **T1-2a** — dev-tier 2D tactical board per ADR-0007 + ADR-0008. Companion specs (Tranche 4) are all on disk: `tactic-fsm.md`, `bt-attribute-binding.md`, `decision-cadence-stagger.md`, `xg-coefficients.md`, `personality-bias-weights.md`. Implementation can begin.

## Blockers

None. Soft acceptance dependency: Claude Preview MCP install (queued in `MEMORY.md` "Queued user actions"). T1-2a starts without it; the e2e Claude-Preview path is part of T1-2a's done-criteria, not a start blocker.

## Last green verify

2026-05-13 — `scripts/fw verify` green at `5bb0939d` (the last code-touching commit was `80c53f76` and had full CI matrix green; `5bb0939d` is docs-only with Determinism Gate green).

## Last canonical hash

`blake3:d6258107b2c90c84d2feeaa8633d1f5c159e10ccd2016623b52b41d3d96b1a49` (60-tick smoke seed; pinned T0-7; UNCHANGED through every audit-remediation commit). The hash next re-baselines at T1-2b-ii (Tranche-5 split row that adds `decision_slots: [u8; 22]` + `interrupt_cooldown_until: [Tick; 22]` to canonical `MatchState`, per ADR-0012 trigger #1).

## Recent commits

- `<this commit>` ci: drop --release from content-pack validation step (Ubuntu timeout fix); STATUS + MEMORY → re-audit green
- `5bb0939d` docs: close 3 P1s + 3 P2s from Codex re-audit pass #2
- `80c53f76` fix: re-audit pass #1 P1+P2 fixes (7 P1s closed)
- `af7df8fa` docs: close 7-tranche audit remediation + queue Codex re-audit
- `27920de6` Tranche 7 — workflow + rules cleanup
- `e79adb07` Tranche 6 — real ContentStore loader + real FW-VAL

## Next up

`/next` picks **T1-2a** as soon as CI on the current HEAD is green. T1-2a scope (per `docs/MASTER_PLAN.md` T1-2a row): `frontend/src/routes/Dev/TacticalBoard.tsx` consuming `MatchFrameDTO` via a `FrameSource` trait (TauriFrameSource + HttpFrameSource impls); `crates/fw-match-sim/src/bin/dump_frames.rs` producing deterministic fixture JSON; `window.fwDev` debug surface for Claude Preview `preview_eval`. Acceptance gate covers Tauri + browser-dev paths end-to-end.

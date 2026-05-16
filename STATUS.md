# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 — First Match.** T1-2b sub-phase shipped + T1-2b-fix audit-remediation closed across 4 rounds. Match-engine inner loop complete. Next: T1-4 MatchEvent emission.

## Active task

(none — T1-2b-fix round 4 closes the last Codex P1; awaiting Codex re-audit confirmation before `/next` picks T1-4)

## Phase pointer

- **Just closed:** **T1-2b-fix round 4** — AC-2 rewritten to observe `signature_cooldowns[&key]` + `signature_firing[slot][cat].start_tick()` instead of the structurally-non-duplicating `signature_first_fired_seen` set (Codex round-3 P1-8 remaining gap). Hash unchanged (test-only behavioral fix).
- **T1-2b sub-phase:** all 9 rows shipped (i, ii, iii-a/b/c/d, T1-3, iv, fix). T1-2b-fix consolidated 8 Codex P1s + 6 P2s across 4 fix rounds; per-round meta-pattern (cargo-cult fix-shape-without-substance) captured in `docs/DECISIONS.md`.
- **Next:** **T1-4** — `MatchEvent` enum (Goal / Shot / Pass / KickOff / FullTime) + ledger output struct + diagnostic commentary templates rich enough to spot brain-dead behavior from text alone (ADR-0007 dev-verification §Layer 1). Reconciles `MemoryEvent::SignatureFirstFired` stub into the real `MatchEvent`. Canonical hash REBASELINE expected.

## Blockers

None. Pending Codex round-4 re-audit verdict on the AC-2 observable rewrite.

## Last green verify

2026-05-16 — `scripts/fw verify` clean post round-4: fmt + clippy + `cargo test --workspace` + release-mode canonical-hash regression on `d376ba26…fa93` + banned-terms + determinism-audit + `fw-content-baker validate`.

## Last canonical hash

`blake3:d376ba2624646f19e3061342f5854bc117ed0a35a2b99a13f51a143bc446fa93` (60-tick smoke seed; pinned at T1-2b-fix round 1 per ADR-0012 trigger #3 — bias-application restructured; subsequent T1-2b-fix rounds 2-4 left the hash unchanged because the new behavior — GK FSM attribute bindings, cross-category bias combine, AC test observables, wire-diagram doc — doesn't reach the smoke-seed dispatch path).

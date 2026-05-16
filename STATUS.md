# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 CLOSED 2026-05-16 at `v0.1.0-first-match`. T2 — League + Season ready to start.** Codex Tier-3 phase-boundary review verdict ACCEPT 2026-05-16. Post-Tier-3 user-requested ultimate review (4 Claude tracks + 2 Codex tracks, 6 tracks total) returned ACCEPT for T2 start: 9 P1 findings across 4 surfaces, none phase-blocking, 4 follow-up rows authored (T1-19/20/21/22). Codex Track F property explosion (PROPTEST_CASES=10000 on 13 invariants, 130k cases) found zero new failures — strongest positive signal of the audit. **7 workflow improvements landed in this commit** per Codex's parallel "anything to change in workflow/setup/blueprint" review: AC-to-test matrix in `/next` Step 2; mutation-thinking checklist in `/next` Step 6; `Sim/RULES.md` §11 (invariants fail in release, not just debug); multi-track ultimate-review codified as `/done` Step 5.5 phase-close lane; validate-naming honesty split in T1-20 spec; hash-pin registry script in T1-22 spec; subagent boilerplate already binding from prior Tier-3 fix.

## Active task

(none — T1 phase CLOSED at `v0.1.0-first-match`; both Tier-3 ACCEPT + ultimate-review ACCEPT in hand. T2-1 ready to start via `/next`; ultimate-review follow-ups T1-19/20/21/22 are recommended-before-T2-1/T2-3 but NOT strict blockers per audit verdict — user chooses to land them sequentially or fold into T2-1/T2-3 spec bodies.)

## Phase pointer

- **Just closed:** **T1 — First Match** at tag `v0.1.0-first-match`. 43 commits since T0 close `27920de6`; 151 files; +135,488 / -677 LoC. End-to-end match-engine vertical works: 22-player deterministic sim → BT-driven decisions → ball physics + goal detection → MatchEvent stream → Tracery commentary → frontend Match page → 2D dev tactical board. Cross-OS canonical-hash determinism gate holds on 2 corpus pins.
- **Next:** **T1-19 / T1-20 / T1-21 / T1-22** OR **T2-1** (user choice — audit verdict says either ordering works). Per ultimate-review recommendation: T1-19 (preempt_check tests + ADR-0006 amendment) is the strongest pre-T2-1 candidate since T2-1 will touch `dispatch.rs` extensively; T1-20 (content validation hardening + validate-naming split) is the strongest pre-T2-3 candidate; T1-21 (Tick policy + Sim/RULES.md §11) is opportunistic; T1-22 (hash-pin registry + env-driven determinism counts) is procedural cleanup. `/next` will pick T1-19 first per MASTER_PLAN declared order (skip-DEFERRED rule walks past T1-17). **Deferred follow-ups (status `DEFERRED` — `/next` skips)**: T1-17 (friction-test rewrite, test-quality only); T4-9 (Stretch 2D viewer, deps T3-3 + T4-1 + T4-5).

## Blockers

None for T2 start. Codex Tier-3 review of T1 is concurrent — its findings (if any) land as a separate /next cycle on `main` or as a follow-up PR.

## Last green verify

2026-05-16 (/done re-attempt post T1-16/T1-18 Codex Tier-2 closures): `scripts/fw verify` exit 0 (cargo fmt + clippy + cargo test --workspace --release + pnpm test 56 frontend + canonical-hash regression on both NEW pins + banned-terms + determinism-audit + fw-content-baker validate + cargo audit + cargo deny check).

## Last canonical hash

`blake3:fcccb840b5868a4ed55c019c353a1d5496259073e2d88bf7abd97d9bdca7a751` (60-tick smoke seed; REBASELINED at T1-16 per ADR-0012 trigger #3 — shoot utility clamp + `fw_core::GOAL_LINE_X` alignment).

**Second corpus pin (T1-16 rebaseline):** `blake3:9353bd257d4da92092407355e3c2b32cc6e91abc81664d0015336ebe812947eb` (600-tick extended seed `0xfeedbeefcafefade` with content-loaded init; final score 2-2).

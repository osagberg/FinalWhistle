# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 — First Match. IN PROGRESS (one blocker: T1-15).** First `/done` attempt on T1 paused 2026-05-16: T1 exit-gate Bullet 1 ("Play Match produces a sensible text recap — 2-5 goals total across 600 ticks, no NaN-tier weirdness") fails empirically. Diagnostic over 5 sampled smoke seeds = 0 goals every run; ball moves in 592/601 frames + 377 distinct positions but stays in [-1.72, 1.14]m Y × [0, 2.64]m X (centre-circle midfielders cycle possession indefinitely; build-up never reaches the attacking third → 0 shots → 0 goals). Real passes ARE firing (T1-3.5 ball mutation + T1-3.6 BT carrier routing land end-to-end) but the offensive progression model is too conservative. All OTHER T1 exit-gate bullets PASS (5 behavioral proptests, replay corpus ≥2 fixtures pin cross-OS, cargo test + clippy + fmt clean, no `unwrap()` in sim non-test code, commentary renders with minute markers). User picked: **add T1-15 to investigate + fix → reach 2-5 goals before `/done`**. T1-15 row added to MASTER_PLAN with hypothesis catalog + acceptance criteria + canonical-hash rebaseline authorization per ADR-0012 trigger #3 (sim behavior change). After T1-15 ships, re-run `/done`.

## Active task

(none — T1-15 is the next `/next` target; T1 phase remains IN-PROGRESS until Bullet 1 passes.)

## Phase pointer

- **Just attempted:** **`/done` on T1** — paused at Step 3 (acceptance-gate verification) when Bullet 1's 2-5-goals envelope was not met across 5 sampled seeds. No git tag created, no PR opened. Other gate bullets all passed.
- **Next:** **T1-15** — investigate + fix offensive build-up; reach 2-5 goals in smoke seed `0xfeedbeefcafefade`. Hypotheses catalogued in the MASTER_PLAN row (4 candidates: `nearest_teammate_near` recursive central loop / off-ball forwards don't drift upfield / shot-attempt utility gates on distance-to-goal / dominant possession slot wrongly bound to deep-lying playmaker subtree). Approach: instrumented dump_frames + commentary trace → identify root cause → MINIMAL fix (calibrate utility weights or fix the obvious bug; deeper tuning defers to T2-1 per its own scope). Canonical-hash REBASELINE authorized per ADR-0012 trigger #3 (sim behavior change with documented intent).
- **After T1-15:** re-attempt `/done`. With Bullet 1 green, the gate passes; tag `v0.1.0-first-match`; open the Codex Tier-3 PR per ADR-0015.

## Blockers

- **T1-15** — `/done` blocker; T1 exit gate Bullet 1 fails. See MASTER_PLAN T1-15 row.

## Last green verify

2026-05-16 — `scripts/fw verify` clean: cargo fmt + clippy + cargo test --workspace --release (8 fw-replay tests + 4 behavior_proptest invariants + all existing) + pnpm test (56 frontend tests) + canonical-hash regression on BOTH pins (60-tick `ddccaf88…000b3` + 600-tick `66585ca8…4625`) + banned-terms + determinism-audit + fw-content-baker validate + cargo audit + cargo deny check. The test/lint/canonical gates ARE green; T1-15 is a behavioral-envelope gate fail, not a regression.

## Last canonical hash

`blake3:ddccaf88c94f328274d484ed1e14ced8638d1ccf63bb922ad64a4f28664000b3` (60-tick smoke seed; unchanged since T1-3.6 rebaseline).

**Second corpus pin:** `blake3:66585ca8af67a5445f32a31f7661089c1a2a608a6dad283f22ac50efc6a34625` (600-tick extended seed `0xfeedbeefcafefade`).

**Note:** both pins WILL rebaseline at T1-15 per ADR-0012 trigger #3 (BT/utility tuning to fix offensive build-up changes per-tick decision outputs → canonical state shifts).

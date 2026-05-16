# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 — First Match. CLOSING (Codex Tier-2 REVISE follow-ups T1-16 + T1-18 closed).** Codex Tier-2 pre-/done audit (2026-05-16 against `575917bc..e79ac115`) said not to open the Tier-3 PR until T1-16 + T1-18 landed; T1-17 could defer. **T1-18 DONE**: split the possession-width proptest into outfield-carry [25, 70]m + GK-carry [5, 100]m bands and fixed the anti-vacuousness counter so tick 0 alone no longer satisfies it. **T1-16 DONE**: shoot proximity scoring now uses `fw_core::GOAL_LINE_X`; shoot utility is clamped back into `[0, 1]` after proximity + personality bias before it enters normal softmax; GK transition constants use `GOAL_LINE_X`; both canonical pins were intentionally rebaselined per ADR-0012 trigger #3. The 600-tick smoke seed `0xfeedbeefcafefade` still finishes 2-2, so T1 exit-gate Bullet 1 remains met. T1-17 (friction-test discrimination) is still deferred as test-quality debt, not a `/done` blocker per Codex.

## Active task

None. T1-16 and T1-18 are closed; T1-17 remains deferred.

## Phase pointer

- **Just closed:** **T1-16** — shoot utility softmax-domain fix + `GOAL_LINE_X` alignment. Canonical pins changed to 60-tick `fcccb840…a751` and 600-tick `9353bd25…47eb`; smoke seed still scores 2-2.
- **Deferred follow-up:** **T1-17** — friction-test rewrite at a 60-tick horizon + pitch-bounds proptest. This remains test-quality debt and is okay to defer past `/done`.
- **Next:** **`/done`** — re-attempt the T1 phase exit-gate verification and then open the Codex Tier-3 PR per ADR-0015.

## Blockers

None.

## Last green verify

2026-05-16 after T1-16/T1-18 follow-ups: `cargo fmt --check` and `scripts/fw verify` both green. The 600-tick smoke seed `0xfeedbeefcafefade` still finishes 2-2 after the T1-16 shoot-utility clamp + `GOAL_LINE_X` alignment rebaseline.

## Last canonical hash

`blake3:fcccb840...a751` (60-tick smoke seed; **REBASELINED 2026-05-16 at T1-16** per ADR-0012 trigger #3 — sim behavior change: shoot utility clamp + `GOAL_LINE_X` alignment).

**Second corpus pin (T1-16 rebaseline):** `blake3:9353bd257d4da92092407355e3c2b32cc6e91abc81664d0015336ebe812947eb` (600-tick extended seed `0xfeedbeefcafefade` with content-loaded init; final score remains 2-2).

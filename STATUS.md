# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 — First Match. COMPLETE (with 3 audit follow-ups deferred per user 2026-05-16).** T1-15 shipped at commit `0a0df5c3` (subagent-autonomous; main-thread post-hoc accepted): empirical T1 exit-gate Bullet 1 ("2-5 goals across 600 ticks") MET — smoke seed `0xfeedbeefcafefade` = 4 goals (2-2); 5-seed range [0, 6]; ball reaches goal line (52.6m abs X) on 4 of 5 seeds; `scripts/fw verify` exit 0. Subagent went substantively beyond the MEMORY-authorized 1-axis fix (short-pass forward target) and shipped a 5-axis behavioral retune (MAX_PLAYER_SPEED 5→8 m/s + nearest-2-chasers preempt + GK chase + ball physics friction tuning + shoot proximity multiplier + insta accept + canonical hash REBASELINED on both pins to `2f14a562…cb27` / `268984…e95ae`). Self-review triple ran post-hoc (3 P1 + 5 P2/MEDIUM): shoot utility unbounded beyond [0, 1] softmax domain; proptest GK-possession exception masks formation-collapse signal; friction tests lose discriminating power; `goal_x = 45` literal vs canonical `GOAL_LINE_X = 52.5`; nearest-2-chasers strict-`<` tie handling; doc-comment 3×/2× typo; invariant 4 docstring stale post speed-bump. User chose **defer all findings to follow-up MASTER_PLAN rows** — T1-16 / T1-17 / T1-18 added; none are `/done` blockers per the user decision. **The T1 phase is now ready for `/done`** with the 3 deferred follow-ups documented in MASTER_PLAN under "T1-15 audit follow-ups (deferred)" + Codex Tier-3 phase-boundary review per ADR-0015 will see them in the PR body.

## Active task

(none — T1-15 closed empirically; T1-16/17/18 deferred per user; T1 phase complete subject to `/done` formality.)

## Phase pointer

- **Just closed:** **T1-15** at commit `0a0df5c3` (subagent-autonomous; main-thread accepted post-hoc). Empirical: smoke seed = 4 goals; 5-seed range [0, 6]; verify green. Scope: 12 files / 460 ins / 113 del — substantively beyond the 1-axis fix authorized in the spec; user accepted the broader change via AskUserQuestion.
- **Deferred follow-ups (not `/done` blockers):** **T1-16** (shoot utility clamp + goal_x→GOAL_LINE_X alignment + comment typo); **T1-17** (friction-test discrimination rewrite + new pitch-bounds proptest); **T1-18** (proptest GK-possession exception investigation + restored team-width discrimination). All TODO in MASTER_PLAN T1-15 audit follow-ups section. May fold into T2-1 (xG calibration phase) or address opportunistically.
- **Next:** **`/done`** — re-attempt the T1 phase exit-gate verification. Bullet 1 now empirically MET (was the prior blocker); all other 7 bullets passed previously. The `/done` skill at `.claude/skills/done/SKILL.md` will (1) re-run `scripts/fw verify`; (2) verify the 8 exit-gate bullets including the now-passing Bullet 1; (3) append a phase-summary block to CHANGELOG.md; (4) rewrite STATUS.md to point at T2; (5) print the `gh pr create` command for Codex Tier-3 phase-boundary review per ADR-0015. **NOTE:** `/done` Step 1 ("every task row DONE") will see T1-16/17/18 as TODO — the user must decide whether to treat the "T1-15 audit follow-ups" subsection as out-of-scope for T1 gate (per the user's defer decision) or to address them first. Recommended: pass per the user's explicit "Defer ALL findings" call.

## Blockers

None (T1-16/17/18 explicitly deferred per user 2026-05-16; not `/done` blockers).

## Last green verify

2026-05-16 — `scripts/fw verify` clean post T1-15 + post-self-review (no in-place fixes applied per user decision): cargo fmt + clippy + cargo test --workspace --release (4 behavior proptests + 8 fw-replay canonical-hash tests + all existing) + pnpm test (56 frontend tests) + canonical-hash regression on BOTH NEW pins (60-tick `2f14a562…cb27` + 600-tick `268984…e95ae`) + banned-terms + determinism-audit + fw-content-baker validate + cargo audit + cargo deny check.

## Last canonical hash

`blake3:2f14a562...dcb27` (60-tick smoke seed; **REBASELINED 2026-05-16 at T1-15** per ADR-0012 trigger #3 — sim behavioral change: utility_pass_short forward target + MAX_PLAYER_SPEED 5→8 m/s + nearest-2-chasers preempt + GK chase + ball physics friction tuning + shoot proximity multiplier).

**Second corpus pin (T1-15 rebaseline):** `blake3:268984120f5eb3ecece932b845f367b0d6f45b94613b7e773ce187027b7e95ae` (600-tick extended seed `0xfeedbeefcafefade` with content-loaded init). Both pins held cross-OS via the existing T0-7b CI matrix.

# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 — First Match.** **T1-8 shipped** — adds the second entry in the cross-OS canonical-hash regression corpus: `0xfeedbeefcafefade.ron` at 600 ticks with content-loaded init (`MatchState::initial_with_content` + `&content.signature_definitions` passed to every `tick_match` call). Sits alongside the bedrock 60-tick smoke pin `0xdeadbeefdeadbeef.ron` (bare `initial` + empty sig defs). Broadens determinism coverage from "60 ticks, no signatures, no content-driven softmax" to "600 ticks, real signature dispatcher firings + cooldowns over many KickOff cycles + content-driven softmax + ball physics through possession transfers". 4 new tests: `extended_seed_600_tick_canonical_hash_pinned` (bedrock for new pin), `extended_seed_runs_10_times_produce_one_hash` (intra-process determinism; 10× × 600 ≈ 60× × 100 tick-evaluation-budget parity with the smoke variant), `extended_seed_corpus_fixture_matches_pinned_constant` (RON-vs-const drift detector mirroring the smoke version), plus extended `PINNED_HASHES` array from 1 row to 2. New pin `66585ca8…4625` was baked via the documented bootstrap protocol (placeholder zero hash → fail-with-actual → replace → green). `fw-content` added as `[dev-dependencies]` on fw-replay (production crate stays content-independent). Existing 60-tick pin `ddccaf88…000b3` UNCHANGED. Self-review triple all ACCEPT (8th clean silent-failure verdict in a row); 7 P3 follow-ups deferred to commit body + future corpus-expansion rows. **T1 phase status: 9 of 11 numbered rows DONE.**

## Active task

(none — T1-8 closed across one main-thread implementation pass; zero P0/P1 self-review findings.)

## Phase pointer

- **Just closed:** **T1-8** — Replay corpus fixture #1: extended-seed `0xfeedbeefcafefade` 600-tick canonical-hash pin with content-loaded init. ~187 LoC of test scaffold + RON fixture; 4 new tests; +1 dev-dep on fw-replay; new pin `66585ca8…4625`; existing pin `ddccaf88…000b3` unchanged.
- **Next:** **T1-9** — behavioral assertions per ADR-0007. New `crates/fw-match-sim/tests/behavior_proptest.rs` with the T1 subset of the invariant catalogue: (a) 4 positional invariants (GK within 30m of own goal 95%+ of ticks; team width 35-65m during in-possession; no sustained >12m/s sprint >4s; defender depth tracks tactical archetype within 8m); (b) PlayerSeparation invariants (Codex Lane D carry-forward from v1 — clumping resistance + opposing-player separation floor); (c) 3 pair-seed knob-isolation tests adopted from openfootmanager's `home_advantage_helps` pattern; (d) `events_chronological` proptest invariant. Acceptance: all positional invariants hold over 100 random seeds, PlayerSeparation over 50, pair-seed produce directional deltas matching the hypothesis. Subagent: `qa-lead` per CLAUDE.md §5 (QA / acceptance criteria / proptest invariants).
- **After T1-9:** `/done` + Codex Tier-3 phase-boundary review per ADR-0015.

## Blockers

None.

## Last green verify

2026-05-16 — `scripts/fw verify` clean post T1-8: cargo fmt + clippy + cargo test --workspace --release (fw-replay went 5 → 9 tests; +4 new) + pnpm test (56 frontend tests) + canonical-hash regression on BOTH pins (60-tick `ddccaf88…000b3` + 600-tick `66585ca8…4625`) + banned-terms + determinism-audit + fw-content-baker validate + cargo audit + cargo deny check.

## Last canonical hash

`blake3:ddccaf88c94f328274d484ed1e14ced8638d1ccf63bb922ad64a4f28664000b3` (60-tick smoke seed; UNCHANGED since T1-3.6 rebaseline).

**Second corpus pin (new, T1-8):** `blake3:66585ca8af67a5445f32a31f7661089c1a2a608a6dad283f22ac50efc6a34625` (600-tick extended seed `0xfeedbeefcafefade` with content-loaded init). Fresh corpus entry, not a rebaseline of existing state — broadens cross-OS gate coverage rather than shifting an existing pin.

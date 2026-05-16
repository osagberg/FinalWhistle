# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 — First Match. COMPLETE.** T1-9 (the last numbered T1 row) shipped — `crates/fw-match-sim/tests/behavior_proptest.rs` adds 4 NEW positional regression-guard proptest invariants per ADR-0007 §Layer 3: GK home/away stays within 30m of own goal line (≥95% of ticks), team width during in-possession stays in [25, 70]m band, no player sustains >12 m/s sprint for ≥4s (vacuously true today at MAX_PLAYER_SPEED=5 m/s — regression-guard for future cap-bump). 100+ line doc-comment header catalogs what lives where + what's deferred to T2-1 (defender-depth-tracks-archetype + 3 pair-seed knob-isolation tests, all blocked on per-team archetype loading) + what's deferred to T2 (5 stat-distribution assertions, need season-length aggregates). qa-lead subagent shipped the file; main-thread fix-pass closed 2 P2 self-review findings (dead-code `sq_sum` guard → `debug_assert!` + bare sqrt; doc-comment "Eight" → "Seven" PlayerSeparation invariants off-by-one). 9th clean silent-failure-hunter verdict in a row. Canonical hashes UNCHANGED on both pins. **T1 phase status: 10 of 11 numbered rows DONE — the phase is COMPLETE; run `/done` to verify the acceptance gate + print the `gh pr create` command for Codex Tier-3 phase-boundary review per ADR-0015.**

## Active task

(none — T1-9 closed; T1 phase complete.)

## Phase pointer

- **Just closed:** **T1-9** — Behavioral assertions per ADR-0007 §Layer 3. 1 new file (`crates/fw-match-sim/tests/behavior_proptest.rs`, ~474 LoC), 4 new proptest invariants, doc-comment catalog header. Self-review: all 3 ACCEPT (2 P2 FIXED in-place); 9th clean silent-failure verdict in a row.
- **Next:** **`/done`** — close the T1 phase. The skill at `.claude/skills/done/SKILL.md` will: (1) re-run `scripts/fw verify` end-to-end, (2) verify T1's acceptance gate (90-min match completes + replay round-trip byte-identical + proptest invariants hold over 10k matches), (3) append a phase-summary block to CHANGELOG.md, (4) rewrite STATUS.md to point at the T2 phase, (5) print the `gh pr create` command for Codex Tier-3 review per ADR-0015. After Codex review + merge → T2 phase begins.
- **T1 deliverables shipped this phase**: 16 numbered rows + audit-triage sub-rows + Codex fix-pass = end-to-end match-engine vertical (sim crates fw-core / fw-match-sim / fw-content / fw-replay all wired; Tauri IPC + frontend Match page + 2D dev tactical board; 600-tick smoke seed runs to completion with real ball motion + signature dispatcher firings + MatchEvent emission + Tracery commentary; canonical-hash determinism gate green on 2 corpus pins; behavioral proptest invariants land; 8 numbered self-review verdicts clean in a row; Codex pre-T0 + T1-2b mid-phase + post-T1-7 adversarial audits all closed).

## Blockers

None.

## Last green verify

2026-05-16 — `scripts/fw verify` clean post T1-9 + fix-pass: cargo fmt + clippy + cargo test --workspace --release (fw-match-sim went +4 new tests via behavior_proptest.rs) + pnpm test (56 frontend tests) + canonical-hash regression on BOTH pins (60-tick `ddccaf88…000b3` + 600-tick `66585ca8…4625`) + banned-terms + determinism-audit + fw-content-baker validate + cargo audit + cargo deny check.

## Last canonical hash

`blake3:ddccaf88c94f328274d484ed1e14ced8638d1ccf63bb922ad64a4f28664000b3` (60-tick smoke seed; UNCHANGED since T1-3.6 rebaseline).

**Second corpus pin (T1-8):** `blake3:66585ca8af67a5445f32a31f7661089c1a2a608a6dad283f22ac50efc6a34625` (600-tick extended seed `0xfeedbeefcafefade` with content-loaded init). Both pins held across T1-8 + T1-9 — no canonical-state schema changes in either row.

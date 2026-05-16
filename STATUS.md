# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 CLOSED 2026-05-16 at `v0.1.0-first-match`; first post-close follow-up T1-19 landed.** T2 ready to start. Codex Tier-3 verdict ACCEPT + post-T1-close ultimate-review verdict ACCEPT both in hand. T1-19 (preempt_check behavioral unit tests + ADR-0006 amendment) closed in this commit — first row to exercise the new `/next` skill hardening (AC-to-test matrix, mutation-thinking pre-check, skip-DEFERRED selector, sub-100-LoC main-thread carve-out) end-to-end. 5 tests green; canonical hashes UNCHANGED on both pins; `scripts/fw verify` exit 0.

## Active task

(none — T1-19 closed at this commit; `scripts/fw verify` green. Next `/next` picks from T1-20 / T1-21 / T1-22 (TODO follow-ups) or T2-1 (full BT runner with 20-30 manager archetypes). Per declared order T1-20 is next; user may also advance to T2-1 directly since the ultimate-review verdict said either ordering works.)

## Phase pointer

- **Just landed:** **T1-19** — `fw-match-sim::dispatch::preempt_check` behavioral unit tests + ADR-0006 amendment. 5 new mutation-discriminating unit tests pin the 3-policy substance shipped at T1-15 (possession gate / GK own-side chase / outfield nearest-2 cap) + the `continue;`-skips-tick_goalkeeper coexistence invariant. Test-only + doc-only change; canonical hashes UNCHANGED on both pins.
- **Next:** **T1-20** per MASTER_PLAN declared order (content-validation hardening + validate-naming split + signature-candidate dangling-reference check + sentinel-block escape close — pre-T2-3 recommended) OR **T1-21** (Tick arithmetic policy + Sim/RULES.md §11 — opportunistic) OR **T1-22** (hash-pin registry script + env-driven determinism counts — procedural cleanup) OR **T2-1** (full BT runner with 20-30 manager archetypes + xG/personality coefficient calibration). `/next` will pick T1-20 first per declared order (skip-DEFERRED rule walks past T1-17; remaining T1-* rows have status TODO). **Deferred follow-ups (status `DEFERRED` — `/next` skips)**: T1-17 (friction-test rewrite, test-quality only); T4-9 (Stretch 2D viewer).

## Blockers

None.

## Last green verify

2026-05-16 (T1-19 close): `scripts/fw verify` exit 0 (cargo fmt + clippy + cargo test --workspace --release including 5 new `preempt_check_*` unit tests + pnpm test 56 frontend + canonical-hash regression on both pins UNCHANGED + banned-terms + determinism-audit + fw-content-baker validate + cargo audit + cargo deny check).

## Last canonical hash

`blake3:fcccb840b5868a4ed55c019c353a1d5496259073e2d88bf7abd97d9bdca7a751` (60-tick smoke seed; UNCHANGED from T1-16 rebaseline).

**Second corpus pin:** `blake3:9353bd257d4da92092407355e3c2b32cc6e91abc81664d0015336ebe812947eb` (600-tick extended seed `0xfeedbeefcafefade`; UNCHANGED from T1-16 rebaseline).

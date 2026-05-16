# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 CLOSED 2026-05-16 at `v0.1.0-first-match`; all 4 post-T1-close ultimate-review follow-ups + first post-Codex-followup-review row (T1-23) landed.** T2 ready to start. T1-23 closed Codex Finding #1 (Tick policy bypassed in 4 production cooldown sites) + Finding #3 (stale dispatch.rs module header) per the post-followup-review REVISE. T2-1 now inherits a Tick-typed, panic-on-overflow cooldown substrate per Codex's pre-T2-1 framing. Self-review triple all ACCEPT with 1 P2 fix in-place. Canonical hashes UNCHANGED on both pins.

## Active task

(none — T1-23 closed at this commit; `scripts/fw verify` exit 0; canonical hashes UNCHANGED on both pins. Next `/next` picks T1-24 (`fw-hash-pins` genuine atomicity — Codex Finding #2; procedural; deferrable per Codex's "next hardening patch before next rebaseline" framing) OR T2-1 (full BT runner with 20-30 manager archetypes + xG/personality coefficient calibration — main T2 row). Per declared MASTER_PLAN order T1-24 is next.)

## Phase pointer

- **Just landed:** **T1-23** — `fw-core::Tick` gained `checked_elapsed_since(entry) -> u32` + `checked_add_ticks(n: u32) -> Tick` typed cooldown-math helpers (both panic-on-invariant-violation per Sim/RULES.md §11). 4 raw-arithmetic cooldown callsites refactored (tactic_fsm.rs PossessionLost + heartbeat_check; dispatch.rs signature cooldown_end_tick; signature/mod.rs::is_active). 2 cooldown constants `i64 → u32` for type alignment. `dispatch.rs` module header rewritten to reflect live 3-policy `preempt_check` (was stale "stubbed None" prose). 3 proptests got `prop_assume!` filters to keep the test domain on invariant-respecting inputs.
- **Next:** **T1-24** per declared MASTER_PLAN order — refactor `fw-hash-pins.py::update_mode` to genuine atomicity (preflight-all-replacements-in-memory + abort-with-no-writes on any failure + then all writes). Closes Codex post-followup-review Finding #2. Procedural; not blocking; Codex's framing was "next hardening patch before next rebaseline." Also eligible: **T2-1** (full BT runner with 20-30 manager archetypes + xG/personality calibration — main T2 row). `/next` will pick T1-24 first per declared order. **Deferred follow-ups (status `DEFERRED` — `/next` skips)**: T1-17 (friction-test rewrite); T4-9 (Stretch 2D viewer).

## Blockers

None.

## Last green verify

2026-05-16 (T1-23 close): `scripts/fw verify` exit 0 (cargo fmt + clippy + cargo test --workspace --release including 6 new Tick helper tests + 3 proptest filter additions + pnpm test 56 frontend + banned-terms + canonical-hash regression on both pins UNCHANGED + content-pack validate-structural + cargo audit + cargo deny check).

## Last canonical hash

`blake3:fcccb840b5868a4ed55c019c353a1d5496259073e2d88bf7abd97d9bdca7a751` (60-tick smoke seed; UNCHANGED from T1-16 rebaseline).

**Second corpus pin:** `blake3:9353bd257d4da92092407355e3c2b32cc6e91abc81664d0015336ebe812947eb` (600-tick extended seed `0xfeedbeefcafefade`; UNCHANGED from T1-16 rebaseline).

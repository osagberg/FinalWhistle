# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 CLOSED 2026-05-16 at `v0.1.0-first-match`; 3 of 4 post-T1-close follow-ups landed (T1-19 + T1-20 + T1-21).** T2 ready to start. Codex Tier-3 ACCEPT + post-T1-close ultimate-review ACCEPT both in hand. T1-21 (`fw-core::Tick` arithmetic policy alignment to Q32's panic-on-overflow + 2 §11-named `debug_assert!` → `assert!` conversions + formal `// SAFETY:` comment on `bump_decision_counter`) closed in this commit — third post-T1-close follow-up. Self-review triple cross-converged on a SAFETY-comment correction (silent-failure-hunter P2 + type-design-analyzer P1 + code-reviewer P2 all flagged the same factually-wrong claim about `local_decision_counter`'s consumers); fixed in-place. Canonical hashes UNCHANGED on both pins; release-mode + debug-mode panics verified.

## Active task

(none — T1-21 closed at this commit; `scripts/fw verify` exit 0; canonical hashes UNCHANGED on both pins. Next `/next` picks T1-22 (procedural cleanup — hash-pin registry script + env-driven determinism rerun counts) or T2-1 (full BT runner with 20-30 manager archetypes + xG / personality coefficient calibration). Per declared order T1-22 is next.)

## Phase pointer

- **Just landed:** **T1-21** — `Tick` operators (`+`, `-`, `+=`, `-=`) + `successor()` + `from_seconds()` now panic-on-overflow via `checked_*().expect()` per Sim/RULES.md §11. New `Tick::clamping_add` + `Tick::clamping_sub` opt-in saturation methods (with `// SAFETY:`-style doc-comments). 8 new Tick arithmetic tests (6 `#[should_panic]` + 1 non-panic negative-zone test + 1 clamping test). `ball_physics.rs` + `dispatch.rs` `debug_assert!` → `assert!` at the 2 §11-named load-bearing sites (canonical ball trajectory + match_events corruption surfaces). `player.rs::bump_decision_counter` gets a formal `// SAFETY:` inline comment justifying the saturating_add on u32 — REWRITTEN after self-review triple cross-converged on a factually-wrong claim about the field's consumers (counter writes BOTH into RNG site AND canonical-hash buffer; comment originally said RNG-only).
- **Next:** **T1-22** per MASTER_PLAN declared order — `scripts/fw hash-pins` registry subcommand + `FW_DETERMINISM_SMOKE_RUNS` / `FW_DETERMINISM_EXTENDED_RUNS` env vars. Procedural cleanup; closes Codex Track F caveat + Track D 5th-pin finding + Codex workflow improvement #6. Also eligible: **T2-1** (full BT runner with 20-30 manager archetypes + xG/personality coefficient calibration — main T2 row). `/next` will pick T1-22 first per declared order (skip-DEFERRED rule walks past T1-17). **Deferred follow-ups (status `DEFERRED` — `/next` skips)**: T1-17 (friction-test rewrite, test-quality only); T4-9 (Stretch 2D viewer).

## Blockers

None.

## Last green verify

2026-05-16 (T1-21 close): `scripts/fw verify` exit 0 (cargo fmt + clippy + cargo test --workspace --release including 8 new Tick arithmetic tests + pnpm test 56 frontend + banned-terms + canonical-hash regression on both pins UNCHANGED + content-pack validate-structural + cargo audit + cargo deny check). Tick panic tests verified green in BOTH debug + release builds.

## Last canonical hash

`blake3:fcccb840b5868a4ed55c019c353a1d5496259073e2d88bf7abd97d9bdca7a751` (60-tick smoke seed; UNCHANGED from T1-16 rebaseline).

**Second corpus pin:** `blake3:9353bd257d4da92092407355e3c2b32cc6e91abc81664d0015336ebe812947eb` (600-tick extended seed `0xfeedbeefcafefade`; UNCHANGED from T1-16 rebaseline).

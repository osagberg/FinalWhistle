# STATUS — Final Whistle

**Last updated**: 2026-05-17

## Phase

**T2 in progress. T2-1 split (a/b/c/d-infra) closed 2026-05-17 + Codex Tier-2 review + 2 P1 + 2 P2 fixes shipped at T2-1-codex-fix.** Codex Tier-2 audit verdict REVISE found a real goal-tick + dispatch race: when a goal fires on a tick where the kickoff taker's decision slot is ALSO active, the post-goal dispatch step could mutate possession again + emit_possession_transition_events would override the Goal arm's MidBlock reset. Fix: 3 if-guards in `lib.rs::tick_match` skip dispatch + pickup + emit_possession on `goal_fired_this_tick`. Also authored T2-1d2 MASTER_PLAN row (was implicit in T2-1d-infra closure docs); cleaned T2-1d done criteria; fixed canonical.rs:204 history-comment drift; strengthened the auto-exit AC5 test to assert post-event state (not just `!SetPiece`). **600-tick canonical pin rebaselined** per ADR-0012 trigger #3; 60-tick UNCHANGED (smoke seed doesn't score in 60 ticks).

## Active task

(none — T2-1-codex-fix closed at this commit; `scripts/fw verify` exit 0; **600-tick canonical pin rebaselined per ADR-0012 trigger #3** (`5716e868…19e3` → `aa7efe9b…5ae`); 60-tick UNCHANGED. Next `/next` picks **T2-2** (procedural clubs) per declared order + skip-DEFERRED rule. T2-1d2 row now authored in MASTER_PLAN; can be picked anytime ahead of T2-2 if user prefers the xG-model-wiring-first sequencing.)

## Phase pointer

- **Just landed:** **T2-1d-infra** — `crates/fw-match-sim/src/bin/calibrate.rs` (~700 LoC) with 3 subcommands (`run` / `fit-xg` / `fit-personality`); new `ShotTelemetryRecord` + `DribbleTelemetryRecord` types + `#[serde(skip)]` Vec fields on MatchState; 4 new feature-extraction helpers in `dispatch.rs`; apply_intent push sites for Shot + Dribble; `drain_shot_telemetry()` / `drain_dribble_telemetry()` pub accessors; new `calibrate_smoke_test.rs` with 2 integration tests; design-doc infrastructure blocks in both `xg-coefficients.md` + `personality-bias-weights.md`; determinism-audit FULLY_EXEMPT_FILES extended for the calibrate binary's off-canonical-path f64 math. 3-match smoke run captured 17 shots, mean xG/shot pre-fit = 0.194 (above [0.09, 0.12] target — confirms Phase-1 BETA is hand-tuned for "feel" not fit). Silent-failure-hunter ACCEPT-with-P2/P3.
- **Next:** **T2-2** — `fw-content`: 20 procedural clubs in a fantasy second-tier league (one-nation pyramid slice for season-loop testing). Deps T1-1, T1-7 — both DONE. T2-1d2 (utility_shoot rewire + coefficient apply) needs to be authored as a new row before /next can pick it; for now T2-2 is the next declared-order eligible TODO. **Deferred follow-ups (status `DEFERRED` — `/next` skips)**: T1-17, T1-25, T1-26, T1-27, T1-28, T4-9. **Carry-forward known follow-ups** from T2-1d-infra self-review: goal back-fill double-attribution (T2-1d2); ridge regularization off textbook (T2-1d2); `solve_k_for_ratio` NaN-resilience belt-and-braces.

## Blockers

None.

## Last green verify

2026-05-17 (T2-1d-infra close): `scripts/fw verify` exit 0 (cargo fmt + clippy + cargo test --workspace + pnpm test 56 frontend + banned-terms + canonical-hash regression on both pins UNCHANGED + content-pack validate-structural + hash-pins atomicity test + cargo audit + cargo deny).

## Last canonical hash

`blake3:eaf842ac3d19651d38dc7ce45d0763cc62b4d571ce2c2a5d56f1ee3c6ddead46` (60-tick smoke seed; UNCHANGED from T2-1b rebaseline — T2-1d-infra is a strict superset; `#[serde(skip)]` telemetry fields don't reach the canonical encoder).

**Second corpus pin:** `blake3:aa7efe9b2a567d5e87d12c7da6a4ea928271429729884f38819baed85c3be5ae` (600-tick extended seed; REBASELINED at T2-1-codex-fix from `5716e868…19e3` per ADR-0012 trigger #3 — Codex Tier-2 audit P1 #1 goal-tick early-return correctness fix: 3 if-guards in tick_match skip dispatch + pickup + emit_possession on goal_fired_this_tick so kickoff taker's same-tick decision can't override Goal arm's MidBlock reset).

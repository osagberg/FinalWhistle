# STATUS — Final Whistle

**Last updated**: 2026-05-17

## Phase

**T2 in progress. T2-1a + T2-1b + T2-1c + T2-1d-infra all closed 2026-05-17.** T2-1 split parent now 4/4 sub-rows DONE (per the SPLIT framing). T2-1d shipped as INFRASTRUCTURE-ONLY (calibrate binary + telemetry types + smoke tests + design-doc blocks) after implementation-discovery #2 revealed `utility_shoot` uses a hand-tuned stub instead of `xg_utility(ShotContext)` — applying fitted β/K constants now would be decorative. T2-1d2 follow-up row (to be authored) wires `utility_shoot` to the xG model + applies the fitted values atomically.

## Active task

(none — T2-1d-infra closed at this commit; `scripts/fw verify` exit 0; **canonical hashes UNCHANGED on both pins** — `#[serde(skip)]` telemetry-buffer fields don't reach the canonical encoder. Next `/next` picks **T2-2** (procedural clubs — `fw-content`: 20 procedural clubs in a fantasy second-tier league) per declared order + skip-DEFERRED rule. T2-1d2 (utility_shoot rewire + apply fitted coefficients) needs to be authored as a new MASTER_PLAN row before it can be picked.)

## Phase pointer

- **Just landed:** **T2-1d-infra** — `crates/fw-match-sim/src/bin/calibrate.rs` (~700 LoC) with 3 subcommands (`run` / `fit-xg` / `fit-personality`); new `ShotTelemetryRecord` + `DribbleTelemetryRecord` types + `#[serde(skip)]` Vec fields on MatchState; 4 new feature-extraction helpers in `dispatch.rs`; apply_intent push sites for Shot + Dribble; `drain_shot_telemetry()` / `drain_dribble_telemetry()` pub accessors; new `calibrate_smoke_test.rs` with 2 integration tests; design-doc infrastructure blocks in both `xg-coefficients.md` + `personality-bias-weights.md`; determinism-audit FULLY_EXEMPT_FILES extended for the calibrate binary's off-canonical-path f64 math. 3-match smoke run captured 17 shots, mean xG/shot pre-fit = 0.194 (above [0.09, 0.12] target — confirms Phase-1 BETA is hand-tuned for "feel" not fit). Silent-failure-hunter ACCEPT-with-P2/P3.
- **Next:** **T2-2** — `fw-content`: 20 procedural clubs in a fantasy second-tier league (one-nation pyramid slice for season-loop testing). Deps T1-1, T1-7 — both DONE. T2-1d2 (utility_shoot rewire + coefficient apply) needs to be authored as a new row before /next can pick it; for now T2-2 is the next declared-order eligible TODO. **Deferred follow-ups (status `DEFERRED` — `/next` skips)**: T1-17, T1-25, T1-26, T1-27, T1-28, T4-9. **Carry-forward known follow-ups** from T2-1d-infra self-review: goal back-fill double-attribution (T2-1d2); ridge regularization off textbook (T2-1d2); `solve_k_for_ratio` NaN-resilience belt-and-braces.

## Blockers

None.

## Last green verify

2026-05-17 (T2-1d-infra close): `scripts/fw verify` exit 0 (cargo fmt + clippy + cargo test --workspace + pnpm test 56 frontend + banned-terms + canonical-hash regression on both pins UNCHANGED + content-pack validate-structural + hash-pins atomicity test + cargo audit + cargo deny).

## Last canonical hash

`blake3:eaf842ac3d19651d38dc7ce45d0763cc62b4d571ce2c2a5d56f1ee3c6ddead46` (60-tick smoke seed; UNCHANGED from T2-1b rebaseline — T2-1d-infra is a strict superset; `#[serde(skip)]` telemetry fields don't reach the canonical encoder).

**Second corpus pin:** `blake3:5716e86877c2d9973a713be0a49ab400fa1d4d8356bfebe9985bf5758aa619e3` (600-tick extended seed; UNCHANGED from T2-1b rebaseline — same rationale).

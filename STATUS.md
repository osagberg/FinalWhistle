# STATUS — Final Whistle

**Last updated**: 2026-05-17

## Phase

**T2 in progress. T2-1a + T2-1b + T2-1c all closed 2026-05-17.** Total manager-archetype catalog reaches **16** (2 T1 hand-authored + 6 T2-1b football-canonical + 8 T2-1c mixed-category). `BallOutOfPlay` + `BallInPlay` `TacticEvent` emissions now wired in `tick_match`: BallOutOfPlay fires per-team in the OOB-clamp with reciprocal `SetPieceKind` (ThrowInFor/Against, CornerFor/Against, GoalKick/GoalKickOpponent); BallInPlay auto-exits SetPiece state in the next possession-transition event. Per-team archetype-driven behavioral divergence covers all 4 TacticEvent classes T2-1a CRITICAL-1 originally deferred (Goal, PossessionLost, BallRecovered, BallOutOfPlay+BallInPlay).

## Active task

(none — T2-1c closed at this commit; `scripts/fw verify` exit 0; **canonical hashes UNCHANGED on both pins** — wiring is a strict superset that doesn't fire in pinned-seed scenarios; 5 emission unit tests prove correctness. Next `/next` picks **T2-1d** (xG / personality coefficient re-fit per `docs/design/xg-coefficients.md` calibration cadence — CREATIVE JUDGMENT required for coefficient values; ambiguity-gate confirmation needed).)

## Phase pointer

- **Just landed:** **T2-1c** — 8 new mixed-category archetypes (lopsided-right-overload, lopsided-left-overload, anti-tiki-taka, anti-high-press, ultra-attacking-no-cb, ultra-defensive-10-back, false-9-system, inverted-fullback-3-2-5) + 8 matching managers + `setpiece_kind_for` helper + `auto_exit_setpiece` helper + BallOutOfPlay emission in OOB-clamp + auto-exit injection in `emit_possession_transition_events`. anti-high-press introduces the previously-empty `(High, High, MidBlock)` bridge bucket. Bridge bucket walk test extended to all 14 new archetypes; spread-check tightened from ≥2 buckets to ≥3. New file `tactic_event_emission_test.rs` with 5 integration tests proving per-team SetPieceKind correctness + auto-exit reliability. Silent-failure-hunter ACCEPT-with-P2 (no P0/P1; 3 P2 deferred to follow-up).
- **Next:** **T2-1d** — xG / personality coefficient re-fit per `docs/design/xg-coefficients.md` + `docs/design/personality-bias-weights.md` calibration cadence. Done criteria: `xG mean ≈ 0.10/shot across 100-match calibration corpus`. CREATIVE JUDGMENT required for coefficient values; will need user ambiguity-gate confirmation. Required subagent: `systems-designer`. **Deferred follow-ups (status `DEFERRED` — `/next` skips)**: T1-17 (friction-test rewrite); T1-25 (sig fit-score 2-candidate test); T1-26 (AC-4 via dispatch_tick path); T1-27 (BT attribute-binding table-walk proptest); T1-28 (separation EPSILON vs MIN_PLAYER_DISTANCE); T4-9 (Stretch 2D viewer). Plus T2-1b/T2-1c commit-body known follow-ups (carry forward): `compute_opponent_shape_broken` wrapping_add→checked_add §11 hardening; goal+dispatch same-tick semantic documentation; direct unit-test matrix for `emit_possession_transition_events`'s 4 transition classes; auto_exit_setpiece eager-fire timing revisit at restart-timing landing; `setpiece_kind_for` corner-flag priority unit test.

## Blockers

None.

## Last green verify

2026-05-17 (T2-1c close): `scripts/fw verify` exit 0 (cargo fmt + clippy + cargo test --workspace + pnpm test 56 frontend + banned-terms + canonical-hash regression on both pins UNCHANGED + content-pack validate-structural + new hash-pins atomicity test + cargo audit + cargo deny).

## Last canonical hash

`blake3:eaf842ac3d19651d38dc7ce45d0763cc62b4d571ce2c2a5d56f1ee3c6ddead46` (60-tick smoke seed; UNCHANGED from T2-1b rebaseline — T2-1c wiring is a strict superset that doesn't fire on this seed).

**Second corpus pin:** `blake3:5716e86877c2d9973a713be0a49ab400fa1d4d8356bfebe9985bf5758aa619e3` (600-tick extended seed `0xfeedbeefcafefade`; UNCHANGED from T2-1b rebaseline — same rationale).

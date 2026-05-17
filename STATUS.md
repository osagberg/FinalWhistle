# STATUS — Final Whistle

**Last updated**: 2026-05-17

## Phase

**T2 in progress. T2-1 split + Codex Tier-2 review fixes + T2-2 (procedural league + fixtures) all closed 2026-05-17.** Total T2 rows DONE: T2-1a/b/c/d-infra + T2-2 (5 of 10 MVP rows). T2-1d2 (utility_shoot rewire + coefficient apply) DEFERRED to end-of-T2 cadence per ambiguity-gate resolution + design-doc "wait for BT to mature" guidance. Codex Tier-2 audit verdict REVISE found a real goal-tick + dispatch race: when a goal fires on a tick where the kickoff taker's decision slot is ALSO active, the post-goal dispatch step could mutate possession again + emit_possession_transition_events would override the Goal arm's MidBlock reset. Fix: 3 if-guards in `lib.rs::tick_match` skip dispatch + pickup + emit_possession on `goal_fired_this_tick`. Also authored T2-1d2 MASTER_PLAN row (was implicit in T2-1d-infra closure docs); cleaned T2-1d done criteria; fixed canonical.rs:204 history-comment drift; strengthened the auto-exit AC5 test to assert post-event state (not just `!SetPiece`). **600-tick canonical pin rebaselined** per ADR-0012 trigger #3; 60-tick UNCHANGED (smoke seed doesn't score in 60 ticks).

## Active task

(none — T2-2 closed at this commit; `scripts/fw verify` exit 0; **canonical hashes UNCHANGED on both pins** — `fw-content` is non-canonical-path. Next `/next` picks **T2-3** (`fw-content-baker` bake-time content-baker binary stub) per declared order + skip-DEFERRED rule (T1-17, T1-25..T1-28, T2-1d2, T4-9 all DEFERRED). 11 commits ahead of origin/main waiting to push.)

## Phase pointer

- **Just landed:** **T2-2** — `crates/fw-content/src/league.rs` with `League` + `Fixture` types + `generate_fixtures(club_ids, _seed)` (circle method, 380 matches / 38 days / 10 per day) + `generate_league(seed, content)` (loops `generate_team` 20× with per-club seed-derivation + culture/archetype/manager round-robin from BTreeMap-ordered catalogs). Pub constants `CLUBS_PER_LEAGUE=20`, `MATCH_DAYS_PER_SEASON=38`, `MATCHES_PER_SEASON=380`. 4 integration tests in `tests/league_generation_test.rs` (count/structure; fixture pair-coverage + per-club 19h/19a + per-day 10 matches + each club exactly once per day; same-seed determinism via struct + serde JSON byte-equality; different-seed divergence). Canonical hashes UNCHANGED (non-canonical-path crate).
- **Next:** **T2-3** — `fw-content-baker` bake-time content-baker binary stub (`fw-cli bake`) — Claude API call → RON corpus → manifest with model-id + prompt-hash + seed. Adopts FW v1's "validator-as-one-class" pattern (carry-forward from `MatchSim/Content/IdentityPacketValidator.cs` per `REFERENCES.md` carry-forward table). One dedicated `<Kind>Validator` type per content kind, chained checks, structured error enum. Deps T1-1 (DONE). **Deferred follow-ups (status `DEFERRED` — `/next` skips)**: T1-17, T1-25, T1-26, T1-27, T1-28, T2-1d2, T4-9. **Carry-forward known follow-ups** from T2-1b/c/d-infra/codex-fix self-reviews: `compute_opponent_shape_broken` wrapping_add→checked_add §11 hardening; direct unit-test matrix for `emit_possession_transition_events`'s 4 transition classes; `auto_exit_setpiece` eager-fire timing revisit at restart-timing landing; `setpiece_kind_for` corner-flag priority unit test; calibrate goal back-fill nearest-unmatched + determinism-audit per-rule exemption (both for T2-1d2).

## Blockers

None.

## Last green verify

2026-05-17 (T2-2 close): `scripts/fw verify` exit 0 (cargo fmt + clippy + cargo test --workspace + pnpm test 56 frontend + banned-terms + canonical-hash regression on both pins UNCHANGED + content-pack validate-structural + hash-pins atomicity test + cargo audit + cargo deny). 4 new tests added: league_generation_test suite.

## Last canonical hash

`blake3:eaf842ac3d19651d38dc7ce45d0763cc62b4d571ce2c2a5d56f1ee3c6ddead46` (60-tick smoke seed; UNCHANGED from T2-1b rebaseline — T2-1d-infra is a strict superset; `#[serde(skip)]` telemetry fields don't reach the canonical encoder).

**Second corpus pin:** `blake3:aa7efe9b2a567d5e87d12c7da6a4ea928271429729884f38819baed85c3be5ae` (600-tick extended seed; REBASELINED at T2-1-codex-fix from `5716e868…19e3` per ADR-0012 trigger #3 — Codex Tier-2 audit P1 #1 goal-tick early-return correctness fix: 3 if-guards in tick_match skip dispatch + pickup + emit_possession on goal_fired_this_tick so kickoff taker's same-tick decision can't override Goal arm's MidBlock reset).

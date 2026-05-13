# openfootmanager — tests + verification

**Read on:** 2026-05-13

## Test inventory

Roughly **816 Rust `#[test]` functions** across four crates plus the Tauri shell, all collected by `cargo test --workspace`. Counts via `grep -rE '^\s*#\[(test|tokio::test)\]'`:

| Crate | In-src `#[cfg(test)] mod tests` | `tests/` integration | Total |
|---|---|---|---|
| `crates/engine` | 0 | 107 | 107 |
| `crates/ofm_core` | 134 | 379 | 513 |
| `crates/domain` | 5 | 0 | 5 |
| `crates/db` | 130 | 0 | 130 |
| `src-tauri/src` (commands) | 61 | 0 | 61 |

Frontend: **25 vitest tests** total (per `vitest-report.json`); 14 passed / 11 failed in the committed report (i18n breakage on a Spanish locale flow). Test surface is tiny relative to the Rust side.

Integration tests in `crates/engine/tests/`:
- `simulation_tests.rs` — 1028 lines, ~46 tests on the older one-shot `simulate_with_rng` API.
- `live_match_tests.rs` — 1744 lines, ~61 tests on the newer tick-based `LiveMatchState`.

## Test types breakdown

- **Unit + integration:** the entire 816. No distinction beyond file location.
- **Property-based:** zero. No `proptest`, no `quickcheck` (`grep` finds no occurrences anywhere in the workspace; no Cargo entries either).
- **Snapshot:** zero. No `insta`, no `__snapshots__` dirs, no golden files.
- **End-to-end:** zero — no Playwright, no webdriver, no Tauri smoke beyond `tauri build --debug --no-bundle` in CI.
- **Bench/perf:** zero `criterion` usage.

Tests are hand-rolled with builder helpers (`make_player`, `make_team`, `make_live_match`) and assert directly on report/snapshot fields. Style is "loop over N seeds and assert a count or a range."

## Sim-correctness tests (if any)

Yes, and this is the most interesting finding. The engine crate has explicit football-realism tests that go beyond structural checks:

- `simulation_tests.rs:400` `strong_team_wins_more_often` — 100 trials, asserts `strong_wins > weak_wins * 2`.
- `simulation_tests.rs:423` `equal_teams_roughly_even` — 200 trials, asserts win-diff `< trials/3`.
- `simulation_tests.rs:454` `home_advantage_helps` — paired-seed comparison with/without home advantage over 200 trials.
- `simulation_tests.rs:490` `possession_style_has_more_possession` — `PlayStyle::Possession` vs `Counter`, asserts avg possession `> 48%` (note: extremely loose).
- `simulation_tests.rs:693` `average_goals_realistic` — 500 trials, asserts `0.5 < avg_goals < 8.0`. Comment: *"Real football averages ~2.5 goals/game. Allow a wide range for a simulation."*
- `live_match_tests.rs:771` same test re-done against the live-match API, 30 trials, same wide band.
- `simulation_tests.rs:559` + `live_match_tests.rs:815` `events_are_chronological` — 10 seeds, asserts `window[1].minute >= window[0].minute`.
- `simulation_tests.rs:344` `possession_adds_up` — possession in `[0, 100]` and sum > 0.
- `simulation_tests.rs:539` `team_stats_shots_consistent` — `shots >= shots_on_target` across 10 seeds.

These are **soft behavioral envelopes**, not invariants. The goal-band is `0.5..8.0` (real football: 2.5–2.8), which catches a sim that emits zero or twenty-goal averages but not one that drifts to 4.5 or 1.2. No xG, no shot-conversion-rate, no per-position stat distribution checks.

## Determinism tests (if any)

Two: `simulation_tests.rs:265` `simulation_deterministic_with_same_seed` and `live_match_tests.rs:208` `deterministic_with_same_seed`. Both same shape — run twice with identical seed, assert `(home_goals, away_goals, events.len())` match. They only assert three integers; they do not compare the full event vector field-by-field, and they do not assert anything across OSes.

**No pinned-hash regression test exists.** No BLAKE3, no MD5, no canonical-state digest of any kind. `grep` for `canonical|blake3|sha|pinned` in the test tree returns only `save_manager`'s `canonicalize_game_starting_xi_ids` — which is data normalization for save-mirroring, not sim determinism.

**No cross-OS matrix on tests.** The PR-gating workflow `.github/workflows/build-check.yml` runs `cargo test --manifest-path src-tauri/Cargo.toml --workspace` on `ubuntu-latest` only. The nightly Tauri *build* matrix covers macOS / Windows / Ubuntu, but it does not run the test suite — only `tauri build`.

Crucially, the canonical sim state freely uses `f64` (`engine/src/types.rs:93` `overall() -> f64`, `MatchConfig::home_advantage: f64`) and `HashMap` (`engine/src/report.rs:88` `player_stats: HashMap<String, PlayerMatchStats>`, `live_match/mod.rs:143` `home_yellows: HashMap<String, u8>`). No determinism floor; the determinism tests pass because the OS is the same, not because the engine guarantees it.

## Stat regression / behavioral assertions

The `average_goals_realistic` test (0.5–8.0 band, repeated in two crates) is the closest thing. There is **no multi-season regression** — no test simulates a full league or season and checks that final goals/match, yellows/match, or pass-accuracy land in a real-world band. League-level tests (`ofm_core/tests/turn_tests.rs`, `end_of_season_tests.rs`) check structural progression (standings advance, contracts expire, news generated, AI hires happen) but not statistical realism.

## CI workflows

`.github/workflows/`:
- `build-check.yml` (PR gate to `develop`): three jobs on `ubuntu-latest`: frontend (`npm test` + `npm run build`), tauri-smoke (`tauri build --debug --no-bundle --ci`), backend (`cargo test --workspace` + `cargo check --workspace`). **No `cargo clippy`, no `cargo fmt --check`, no banned-terms lint, no determinism gate, no cross-OS test matrix.**
- `tauri-action.yml` + `nightly-tauri-action.yml`: production/nightly *build* matrix (macOS / Windows / Ubuntu arm64+x64). These only build artifacts; they do not re-run tests.
- `release-manifest.yml`, `nightly-release-manifest.yml`: release metadata pipeline.

`scripts/` has exactly one file: `audit-i18n.mjs` (translation-key sweep). No test orchestration scripts.

## insta / proptest usage

Zero. Confirmed via `grep -rEn 'insta::|assert_snapshot|proptest|quickcheck'` across the entire workspace — no matches and no Cargo entries.

## Comparison to Final Whistle

### Where we're stricter

- **Determinism floor:** we ban `f32` / `f64` in canonical state, ban `HashMap` in sim crates, ban `tokio` in sim crates, ban `Instant::now`. openfootmanager has none of these — `f64` and `HashMap` are everywhere in `engine` and `live_match`. Their "deterministic_with_same_seed" tests only assert score + event-count integers, never the full state.
- **Cross-OS hash regression:** we have it; they have nothing. Their CI tests run Ubuntu-only.
- **Property invariants (`proptest`):** we mandate them for canonical-state-emitting code; they have zero `proptest` tests.
- **Snapshot stability (`insta`):** we mandate it for new behavior; they have zero `insta` tests.
- **Lint gates:** we run `clippy -D warnings`, `fmt --check`, banned-terms lint, content validation. They run `cargo check` and tests only.

### Where they're stricter (if anywhere)

- **Raw count of behavioral assertions on the running sim.** 816 tests is a lot, and 100+ of them actually load a match, run it, and inspect the report — vastly more "did the sim do football?" coverage than we currently have shipped. Many of our T0/T1 tests are still hash + invariant scaffolding rather than "after 100 sims, the right thing happened on average."
- **Population-level realism tests live in code, not docs.** `strong_team_wins_more_often`, `possession_style_has_more_possession`, `home_advantage_helps` are committed `#[test]`s that run on every PR. Our `docs/design/dev-verification.md` describes a similar three-layer model but currently has fewer codified equivalents in `crates/fw-match-sim/tests/`.

### Techniques we should adopt

1. **Pair-seed behavioral comparisons.** `home_advantage_helps` runs the same N seeds twice with different config and compares — neat trick for isolating a single sim knob's effect. Cheap to write, hard to fake.
2. **Wide-band stat envelopes.** `0.5 < avg_goals < 8.0` is loose, but it's a real backstop and catches catastrophic drift. We should commit equivalents per pillar metric (goals/match, fouls/match, possession sum, sub-count, pass-accuracy band) on top of our proptest invariants.
3. **`events_are_chronological` invariant tested across many seeds.** Trivial but high-value; our equivalent should run as a `proptest` invariant over arbitrary tick budgets and any seed.
4. **Trait/style differentiation tests.** Their `hot_head_trait_increases_foul_likelihood` (`live_match_tests.rs:1354`) statistically asserts a trait actually changes outcomes. We should mirror this for our 24 signatures: "this signature, given N reps, fires above baseline."

### Techniques we should avoid

- **Their `0.5..8.0` goal band is too loose to mean anything.** A working sim should land 2.0–3.5 on default config. We should narrow bands aggressively, and band-tightening should be a phase gate, not optional.
- **`HashMap<String, _>` in canonical paths** with hand-rolled "deterministic" tests that only check score-tuples — this is exactly the false-confidence trap our `Sim/RULES.md` §2 forbids.
- **Ubuntu-only test CI.** Their nightly matrix builds three OSes but never runs `cargo test` on macOS or Windows. Our cross-OS matrix is mandatory and that's correct.
- **No clippy in CI.** Surprised. Don't follow.
- **Mixing `f64` with seed-driven RNG and calling it deterministic** — works on one machine, hides cross-OS LLVM fpmath drift forever.

## Open questions

- They have *two* match engines living side-by-side (`simulate_with_rng` vs `LiveMatchState`) with duplicated behavioral tests. Migration in flight? Worth checking `git log` if we ever want to learn from their tick-based refactor.
- Vitest report shows 11 of 25 frontend tests currently failing — does that block merges? `build-check.yml` runs `npm test`, so presumably the report on disk is stale. Worth confirming if we go deeper on their frontend test patterns.
- `db/src/save_manager.rs` does *save-order canonicalization* (sorting starting XI IDs before persisting) — a different determinism strategy than ours. They normalize at the save boundary instead of forbidding non-determinism in the sim. Worth a separate look for save-migration techniques in `04-openfootmanager-saves.md` if that ever happens.

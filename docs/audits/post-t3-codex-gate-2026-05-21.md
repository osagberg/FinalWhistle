# Post-T3 Codex Gate Review - 2026-05-21

## Verdict

ACCEPT.

I found no T3 phase-gate blocker. The three locked T3 exit criteria are met in substance: the 5-season career path runs, ledger compaction happens at the season-5 boundary, player/career screens can surface memory callbacks, and Save V1 -> V2 migration is locked by tests and fixtures.

There are two P1 carry-forward issues I would fix before T4 save/UI work gets built on top: SaveV2 does not persist the runtime career state, and structural content validation can accept a one-player squad pack. Neither invalidates T3 as closed, but both are real enough to deserve explicit rows before they become hidden foundation.

## Pre-flight

- `scripts/fw verify`: PASS on this workspace.
- `PROPTEST_CASES=10000 cargo test -p fw-scouting --release -- observe_player_structural_invariants`: PASS.
- `PROPTEST_CASES=10000 cargo test -p fw-memory --release -- append_count_matches_next_id event_ids_are_strictly_monotonic`: PASS.
- `PROPTEST_CASES=10000 cargo test -p fw-memory --release`: PASS.
- `PROPTEST_CASES=10000 cargo test -p fw-save --release -- v2_round_trip_byte_identical_across_payloads v1_round_trip_byte_identical_across_payloads`: PASS.
- `cargo test -p fw-save --release --test migration_fixtures_test`: PASS.
- `PROPTEST_CASES=10000 cargo test -p fw-replay --release -- encoder_invariants_hold_across_seeds`: PASS.
- `FW_DETERMINISM_EXTENDED_RUNS=100 cargo test -p fw-replay --release -- extended_seed_runs_10_times_produce_one_hash`: PASS.
- `FW_DETERMINISM_SMOKE_RUNS=1000 cargo test -p fw-replay --release -- smoke_seed_runs_100_times_produce_one_hash`: PASS.
- `PROPTEST_CASES=10000 cargo test -p fw-content --release -- fixture_schedule_pair_coverage_holds_across_seeds`: PASS.
- `cargo test -p fw-tauri --release --test season_commands_test -- five_season_career_integration_fast`: PASS.

## T3 Exit Gate

- 5-season career with compaction: MET. The fast integration test exercises five season advances and the ledger compaction path through the Tauri career state. Runtime compaction is in `MemoryLedger::compact`, and `advance_season_inner` calls it when the new season number reaches the five-season boundary.
- Cross-season callback on a screen: MET. `get_player_detail_inner` and `get_career_overview_inner` both read the ledger and render callback/news surfaces. There is one timing caveat below: player detail still passes `Tick::ZERO` into salience reads until a real career clock exists.
- Save V1 -> V2 bump: MET. Save discriminants V0/V1/V2 are explicit, V1 migrates into V2, V0 remains rejected, future versions remain rejected, and byte-identical encode/decode property tests passed at 10000 cases.

## Track E - Adversarial Red-Team

### E1 - Canonical hash bypass

Negative result. I did not find a T3 match-state canonical hash bypass.

The T3 work does not add canonical match-state fields, both pinned match hashes are unchanged, and replay determinism survived 1000 smoke reruns plus 100 extended reruns. The remaining f64 uses I found are renderer/prose/test surfaces, not canonical match-state mutation paths. I also did not find new `HashMap`/`HashSet`, clock, thread RNG, `rayon`, pointer-ordering, or `unsafe` use in the T3 canonical path.

Residual risk: the match encoder is still hand-rolled. That is acceptable today because the pins and field-order property test are doing useful work, but every future MatchState field still needs explicit encoder review.

### E2 - Content-pack semantic poisoning

P1 carry-forward. A content pack can pass structural validation with a semantically broken squad.

Repro I ran in a temp content copy:

```bash
tmp=$(mktemp -d /tmp/fw-t3-content-poison-count.XXXXXX)
cp -R content "$tmp/content"
find "$tmp/content/sources/player-bios" -type f ! -name 'player_00001.ron' -delete
cargo run -q -p fw-content-baker -- --workspace "$tmp" validate-structural
```

The command exited 0 and reported `structurally validated ... 1 player bios`. That passes because `validate_content_store` only requires non-empty categories in `crates/fw-content-baker/src/main.rs:356`, and player bios are validated one-by-one in `crates/fw-content-baker/src/main.rs:419`. The per-bio validator checks ID shape in `crates/fw-content-baker/src/validators.rs:443`, but there is no pack-level invariant saying the MVP squad must contain exactly 22 player bios, contiguous roster IDs, or a manifest-backed squad list. The Tauri squad command then returns whatever bios exist, despite its 22-player-pool contract in `crates/fw-tauri/src/commands.rs:255`.

Recommended fix: add a pack-level player-bio roster validator before T4 UI/content work builds on this. For the current MVP pack, require exactly 22 bios with expected IDs or a committed squad manifest that all pages and validators consume. This belongs at `ContentStore`/baker level, not inside each `PlayerBio`.

### E3 - Malicious mod overlay

Negative result for current code. Runtime mod overlays are not implemented yet.

`Content/RULES.md` describes mod overlay ordering and overrides, but `ContentStore::load_baked` still delegates to `load_sources`, and the baked/mod overlay path is still a TODO in `crates/fw-content/src/runtime.rs`. There is no live overlay merge path to exploit today.

Recommended future gate: when mod overlays land, validation must run after overlay merge, not only on base pack files. Add tests for banned-term override, dangling manager/archetype override, and ID collision with explicit `overrides`.

### E4 - Determinism leak

Negative result. I did not find a new T3 determinism leak.

The new memory/scouting/save/career surfaces use ordered collections or explicit vectors where determinism matters. `RwLock<CareerState>` serializes the mutable career state behind a write lock in `advance_season_inner`; I did not find concurrent mutation that can interleave into canonical output. The clock-like uses I found are tests/perf or not in canonical state.

### E5 - SaveV2 does not persist runtime career state

P1 carry-forward. SaveV2 locks a useful migration format, but it does not yet save enough to resume the T3 career loop honestly.

`SaveV2` stores only `career_seed`, `content_pack_version`, and `ledger` in `crates/fw-save/src/lib.rs:123`. Runtime career state also contains the active `SeasonState` and `season_number` in `crates/fw-tauri/src/state.rs:48`, and `advance_season_inner` mutates both in `crates/fw-tauri/src/commands.rs:752`. Today that means the save schema can preserve the ledger, but not the current season table, match-day position, or season number unless a caller reconstructs them out-of-band.

This does not block T3 because file-backed save/load UI is not the shipped user path here, and the T3 gate only required one deliberate schema bump. But it should block any real save/load UI or long-career UX. Otherwise T4/T5 can accidentally ship "save remembers the story but not where the career actually is."

Recommended fix: add SaveV3 before exposing save/load UI. Include `season_number` and either the current `SeasonState` or a deterministic replay cursor/results log that can reconstruct it. Add a committed V2 -> V3 fixture with a non-empty ledger and non-initial career state.

### E6 - Frozen V2 fixture is missing

P2. The save migration fixture set proves V0/V1 behavior, but it does not freeze a real V2 payload with a non-empty ledger.

The fixture README documents V1, V0, and V99 samples, while the current schema map says V2 is production. The regeneration test writes V1/V0/V99 samples, not a V2 non-empty-ledger sample. The property tests cover V2 encode/decode well, but they do not protect an old committed V2 byte stream across future code changes.

Recommended fix: add `v2_nonempty_ledger_sample.fwsave` before the next save-schema bump. Include at least two MemoryEvents and one Compaction event, then assert decode and byte-identical re-encode.

## Track F - Property Explosion

No property failures found.

The 10000-case sweeps passed for the T3-heavy invariants:

- fw-scouting observation structural invariants.
- fw-memory append monotonicity, append count, compaction, readers, and breakthrough tests.
- fw-save V1/V2 byte-identical round trips.
- fw-replay encoder field-order invariants.
- fw-content fixture schedule pair coverage.

The determinism stress reruns also passed:

- smoke seed: 1000 reruns -> one hash.
- extended seed: 100 reruns -> one hash.

## Additional Notes

### Salience decay is intentionally not live in player detail yet

P2. `get_player_detail_inner` still passes `Tick::ZERO` into salience ranking and documents that a real career clock is deferred to T4. This is acceptable for T3 gate closure, but it should be fixed before memory callback ranking becomes a player-facing balance surface.

Recommended fix: add a career clock / current tick abstraction and pass it into all salience readers. Avoid each UI command inventing its own timestamp.

### Memory compaction behavior is internally consistent

I checked the 5-season compaction boundary. `MemoryLedger::compact` repeatedly summarizes any event older than the 5-season window and appends a new Compaction event when the in-window count remains non-zero. The tests explicitly expect the cumulative shape. I am not flagging this as a bug, but the behavior is worth keeping documented because it is a design choice, not an obvious invariant.

## Codex Consolidation

Codex Track E/F adds two categories to the Claude-track picture: save/runtime-career persistence shape, and pack-level roster validation. These are not T3 gate blockers, but they are high-leverage pre-T4 cleanup items.

My recommendation: close T3 as accepted, then add explicit follow-up rows before T4 user-facing polish:

1. SaveV3 career-state persistence: include season number and active season reconstruction.
2. Pack-level squad/player-bio validation: exact roster invariant or manifest-backed roster.
3. Frozen V2 fixture with non-empty ledger.
4. Career clock for salience decay in player/career UI reads.

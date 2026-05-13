# Pattern: Unit testing in Final Whistle

`cargo test` + `insta` snapshots + `proptest` invariants + pinned canonical-hash regression. Cross-OS gate on every PR.

## The test pyramid

| Level | Tool | Purpose |
|---|---|---|
| Unit | `#[cfg(test)] mod tests` | Pure function correctness |
| Property | `proptest` | Invariants that should hold over input distributions |
| Snapshot | `insta` | Stable readable outputs (canonical state dumps, commentary, prose) |
| Integration | `tests/*.rs` per crate | Cross-module flows, IPC round-trips |
| Regression | `fw-replay::canonical_hash` | The pinned hash gate (the most important test) |
| Cross-OS | GH Actions matrix | macOS + Windows + Linux; drift on any blocks merge |

## Unit (basic)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q32_saturating_add_at_max_saturates() {
        let max = Q32::MAX;
        assert_eq!(max.saturating_add(Q32::from_int(1)), Q32::MAX);
    }
}
```

## Property (proptest)

```rust
proptest! {
    #[test]
    fn saturating_add_is_commutative(a: i64, b: i64) {
        let qa = Q32::from_bits(a);
        let qb = Q32::from_bits(b);
        prop_assert_eq!(qa.saturating_add(qb), qb.saturating_add(qa));
    }
}
```

Run with `cargo test --workspace`. Proptest seeds are persisted in `proptest-regressions/` — commit these to ensure regression catches reproducible.

## Snapshot (insta)

```rust
#[test]
fn canonical_state_at_tick_500_matches_snapshot() {
    let mut sim = Sim::new(Seed::from_u64(0xdeadbeef));
    for _ in 0..500 { sim.tick(); }
    let canonical = sim.canonical_state_string();
    insta::assert_snapshot!("canonical_at_tick_500", canonical);
}
```

First run: `cargo test` produces a `.snap` file. Review with `cargo insta review`. Commit the `.snap` file.

Updates: when behavior intentionally changes, delete the old snap + re-run. The diff in PR review is the change you're approving.

## Pinned canonical-hash regression

Lives at `crates/fw-replay/tests/canonical_hash.rs`. Runs a known scenario (seed, fixture, N ticks), BLAKE3-hashes the final canonical state, asserts == pinned.

Drift on any platform → REGRESSION. The `.claude/hooks/canonical-hash-guard.sh` PreCommit hook blocks commits that drift the hash unless the task spec explicitly authorized re-pinning.

Re-pinning procedure (when intentional):
1. Task spec includes "regenerate pinned canonical hash" as an acceptance criterion.
2. Run the test → it fails → record the new hash printed in the assertion error.
3. Update the fixture file with the new hash.
4. Commit body notes the re-pinning + reason.

## Integration tests

`crates/<crate>/tests/<name>.rs` files — they import the crate as an external user would. Use sparingly; prefer unit + property where possible.

## Tests that are MANDATORY for new behavior

Per `Sim/RULES.md` §8:
- New canonical-state-emitting behavior → 1 `insta` snapshot + 1 `proptest` invariant.
- New `MatchEvent` / `MemoryEvent` variant → canonical encoder snapshot.
- New IPC command → integration test in `fw-tauri` round-tripping the boundary.

Per save-load pattern:
- Schema bump → four tests (forward-migration, callback-preservation, forward-incompat-failure, round-trip-byte-identical).

## Determinism in tests

- Tests MUST use seeded `ChaCha8Rng`, never `thread_rng()`.
- Tests MUST NOT read clocks unless explicitly testing time-formatting code.
- Tests run single-threaded by default (`.cargo/config.toml` sets `test-threads = 1` for sim crates) to avoid order-dependent state leaks.

## Cross-references

- `docs/specs/determinism-gate.md` — pinned-hash contract
- `docs/specs/save-migration-fixtures.md` — save tests
- `Sim/RULES.md` — determinism rules
- `qa-lead` agent — test strategy owner

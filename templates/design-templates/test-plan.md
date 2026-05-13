# Test plan — {{system-name}}

**Author:** {{name}}
**Date:** {{YYYY-MM-DD}}
**System under test:** {{e.g., fw-match-sim ball physics integrator}}
**Owning agent:** `qa-lead` partnered with {{gameplay-programmer / ui-programmer / etc.}}

---

## Acceptance criteria (observable)

1. {{falsifiable criterion 1, with named test/snapshot}}
2. {{falsifiable criterion 2}}
3. {{falsifiable criterion 3}}

Every acceptance criterion has a named test that proves PASS or FAIL.

## Test levels

### Unit
- {{test name}} at `crates/{{crate}}/src/{{module}}.rs::tests::{{name}}` — covers {{behavior}}

### Snapshot (insta)
- `{{snapshot_name}}` at `crates/{{crate}}/tests/{{file}}.rs` — captures {{canonical surface}}

### Property (proptest)
- `{{invariant_name}}` at `crates/{{crate}}/tests/{{file}}.rs` — invariant: {{property held under all input distributions}}

### Integration
- `{{test_name}}` at `crates/{{crate}}/tests/{{file}}.rs` — exercises {{IPC boundary / cross-crate flow}}

### Canonical-hash regression
- Does this system's change drift the pinned canonical hash?
  - **No drift expected.** The pinned-hash test in `fw-replay` should pass unchanged.
  - **Drift expected and authorized.** Re-pin procedure: {{steps}}. New pinned hash recorded in commit body.

### Cross-OS gate
- Tests must pass on `[macos-14, windows-latest, ubuntu-22.04]`. Local dev validates macOS-14; CI matrix covers the other two.

## Test data

- Seeds: {{list of `(match_seed, tick, event_id)` triples used in fixtures}}
- Fixtures: {{paths to fixture files}}
- Mocks: {{any mocked dependency, why mocked, what real-world drift risk}}

## Manual verification steps

For UI / content changes that aren't fully testable:
1. {{step 1}}
2. {{step 2}}
3. Screenshot attached: {{path}}

## Regression risk

What existing tests might break? What's the blast radius?

## Cross-references

- `docs/specs/save-migration-fixtures.md` (if save-affecting)
- `docs/specs/content-pack-validation-contract.md` (if content-affecting)
- `docs/specs/determinism-gate.md` (if sim-affecting)
- `CLAUDE.md` §9 — verification matrix

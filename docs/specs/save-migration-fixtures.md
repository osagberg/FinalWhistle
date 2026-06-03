# Save-migration fixtures — the four-test-per-bump contract

Status: authoritative. This spec consolidates the save-migration discipline that
was previously referenced (under the never-created `design/specs/` path) by
ADR-0010, `determinism-gate.md`, `career-roster-layer.md`, and CLAUDE.md §9.
The byte-level fixture tables live alongside the binaries in
`fixtures/save-migration/<from>-to-<to>/README.md`; CLAUDE.md §9 is the onboarding
summary. This file is the spec they all point at.

## Why

The save format ships FOREVER. A save written by any past binary must remain
loadable by every future binary, migrated forward to the current schema. The
failure mode this guards against is a silent one: a migration that compiles, runs,
and produces a working-but-WRONG career with no diagnostic. The contract makes
that failure loud at test time.

## The four tests — every schema bump owes all four

For each `SaveV{N} → SaveV{N+1}` bump, the migration owes these tests (see
`crates/fw-save/tests/migration_fixtures_test.rs`):

1. **forward-migration** — a frozen `v{N}` fixture loads through `load_envelope`
   and lands at `SaveV{N+1}` with every preserved field intact. Fixtures use
   NON-EMPTY, non-default state (real roster / ledger / scout rows) so the test
   exercises the actual serde surface, not just defaults.
2. **callback-preservation** — fields the new schema carries forward survive the
   migration bit-exact (e.g. `career_seed` across all 64 bits).
3. **forward-incompat-failure** — a save authored at a FUTURE unknown version
   (e.g. `v99_future.fwsave`) loaded by an older binary FAILS LOUDLY
   (`SaveError::Decode` via bincode's `UnexpectedVariant`), never silently.
4. **round-trip-byte-identical** — re-encoding the migrated payload reproduces
   the committed golden bytes exactly, so a serde reorder / discriminant shift
   is caught against frozen bytes.

## Fixture-accumulation policy

- Frozen fixtures are **append-only history.** The `v{N}` fixture stays exactly
  as it was the day `v{N+1}` shipped — re-writing on-disk fixtures across versions
  is forbidden (it would mask a broken migration). Re-pin a golden ONLY after an
  intentional schema bump or an explicitly-authorized fixture change (e.g. the
  T4-8-CR1 watermark correction), documented in the commit body.
- Fixtures live in `fixtures/save-migration/<from>-to-<to>/`, each with a README
  carrying the per-field value table + the annotated `xxd` wire bytes.
- Migrations are pure functions `migrate_v{N}_to_v{N+1}` in `fw-save`; forward-only
  (no down-migration), per the `Content/RULES.md §3` pattern.

## Load-boundary invariants (T4-8-CR1)

Beyond the four tests, `load_envelope` enforces fail-loud integrity gates on the
decoded payload BEFORE it reaches the runtime (reject, never clamp):

- `breakthrough_eval_watermark <= ledger.len()` (`SaveError::WatermarkBeyondLedger`).
- `MemoryLedger` event-ids are contiguous-from-zero, `events[i].event_id.0 == i`
  (`SaveError::MalformedLedger` via `MemoryLedger::validate_for_load`).

A crafted or corrupt save that violates either fails loudly at load rather than
loading working-but-wrong. See `docs/audits/post-t4-codex-gate-2026-06-03.md`.

## Cross-references

- `docs/adr/0010-save-format.md` — the save-format ADR (wire framing, zstd, migration discipline).
- `fixtures/save-migration/v0001-to-v0002/README.md` — the byte-level fixture tables.
- `CLAUDE.md §9` — the onboarding summary of this contract.
- `crates/fw-save/tests/migration_fixtures_test.rs` — the tests themselves.

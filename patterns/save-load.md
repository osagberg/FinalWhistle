# Pattern: Save / load with version migration

Bincode 2 format. Forward migration only. Four mandatory tests per schema bump.

## Why

A football management career is a 20-year save. Schema WILL evolve. Players load saves from old builds into new builds; old saves must continue to work — at the cost of bumping the schema and writing a migration path.

## When to use

- Persisting full canonical state (in-game career, world)
- Settings / preferences (a separate `SettingsV1` etc.)
- Replay corpus snapshots (less migration burden, but same pattern)

## When NOT to use

- Temporary in-memory state (no need to version)
- IPC DTOs (those are read-only, not persisted)
- Content packs (use RON + content-pack-qualified IDs — different versioning model, see `Content/RULES.md`)

## Pattern

```rust
#[derive(serde::Serialize, serde::Deserialize)]
pub enum SaveFile {
    V1(SaveV1),
    V2(SaveV2),
    V3(SaveV3),
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SaveV3 {
    pub schema_version: u32,  // always 3 — sanity check
    pub world: World,
    pub careers: BTreeMap<CareerId, CareerLedger>,
    pub mod_load_fingerprint: blake3::Hash,
    // ...
}

pub fn load(bytes: &[u8]) -> Result<SaveV3, SaveError> {
    let raw: SaveFile = bincode::serde::decode_from_slice(bytes, bincode::config::standard())?.0;
    match raw {
        SaveFile::V1(v1) => Ok(migrate_v2_to_v3(migrate_v1_to_v2(v1))),
        SaveFile::V2(v2) => Ok(migrate_v2_to_v3(v2)),
        SaveFile::V3(v3) => Ok(v3),
    }
}

fn migrate_v1_to_v2(old: SaveV1) -> SaveV2 { /* explicit field mapping */ }
fn migrate_v2_to_v3(old: SaveV2) -> SaveV3 { /* explicit field mapping */ }
```

## Schema bump procedure

When a struct change requires a new schema version:

1. **Create `SaveV<N>`** as a new struct (don't modify the old `SaveV<N-1>`).
2. **Add the variant** to the `SaveFile` enum.
3. **Write the migration function** `migrate_v<N-1>_to_v<N>(old: SaveV<N-1>) -> SaveV<N>`.
4. **Update the `load` fn** to chain the new migration step.
5. **Author the four mandatory tests** per `docs/specs/save-migration-fixtures.md`:
   - Forward-migration: old fixture file → new schema → expected values
   - Callback-preservation: load → save → load → second-load == first-load
   - Forward-incompat-failure: V<N+1> bytes → load on V<N> code → typed error
   - Round-trip-byte-identical: serialize SaveV<N> → bytes; deserialize bytes → SaveV<N>; serialize again → identical bytes

The `qa-lead` agent owns these tests.

## Determinism considerations

- `BTreeMap` only — never `HashMap` (canonical-state serialization stability).
- Field order in struct definitions is STABLE — don't casually rearrange, Bincode encodes order.
- Q32 values serialize as their underlying `i64` (the fixed crate's serde impl handles this).

## `mod_load_fingerprint`

A BLAKE3 hash of `(sorted list of (mod-id, mod-version))` at save time. On load, compare against the current mod set:
- Identical → load silently.
- Different → warn the user; some content may shift.
- Mods load order is lexicographic by mod-id (per `Content/RULES.md` §6).

## Failure modes

- **Mutating SaveV1 in place:** breaks loading of old saves. Never do this. Always create a new SaveV<N+1>.
- **Forgetting a migration step:** SaveV1 → load via V3 code fails. Tests catch this.
- **Bincode config drift:** if `bincode::config::standard()` changes between Bincode 2.x versions, byte output drifts. Pin Bincode version + write integration test that hashes a known fixture's bytes.

## Cross-references

- `crates/fw-save/` (Phase T1+ target)
- `docs/specs/save-migration-fixtures.md` (the test contract)
- `qa-lead` agent — test design

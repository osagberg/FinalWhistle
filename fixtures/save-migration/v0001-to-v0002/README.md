# Save-migration fixtures — v0001 to v0002

Committed binary fixtures for the T3-7 four-test migration discipline
(`migration_fixtures_test.rs`). These files are FROZEN input — they represent
real save bytes that must remain loadable by any future binary. Do NOT
regenerate casually; re-pin only after an intentional schema bump + explicit
decision.

See `CLAUDE.md §9` for the authoritative four-test-per-bump discipline.

---

## Fixture definitions

### `v1_sample.fwsave` (12 bytes)

A `SaveEnvelope::V1` save. Wire tag: `0x01`.

| Field                | Value                        |
|----------------------|------------------------------|
| `career_seed`        | `0x5A5EF1C700010002` (u64)   |
| `content_pack_version` | `1` (u32)                  |
| `ledger`             | `MemoryLedger::new()` (empty)|

Encoded via `bincode 2` / `standard` config (`bincode::config::standard()`).
The full wire bytes (`xxd v1_sample.fwsave`):

```
01  fd 02 00 01 00 c7 f1 5e 5a  01  00
─┬─ ──────────────┬─────────── ─┬─ ─┬─
 │                │              │   └── ledger: 0 events (varint 0 = 0x00)
 │                │              └────── content_pack_version: 1 (varint 1 = 0x01)
 │                └───────────────────── career_seed: varint-253 (0xfd) prefix,
 │                                       then 8 bytes LE = 0x5A5EF1C700010002
 └──────────────────────────────────────  V1 enum tag (0x01)
```

bincode 2 standard config encodes u64 as a variable-length integer: values ≥ 251
use a multi-byte prefix. 0xfd (253) signals "8-byte u64 follows."

Used by tests:
- `fixture_v1_forward_migrates_to_v2` (AC2 — forward-migration)
- `fixture_v1_all_fields_preserved` (AC3 — callback-preservation)
- `fixture_v1_round_trip_byte_identical` (AC5 — round-trip-byte-identical)

---

### `v0_sample.fwsave` (10 bytes)

A `SaveEnvelope::V0` save. Wire tag: `0x00`.

| Field         | Value                        |
|---------------|------------------------------|
| `career_seed` | `0xA0B1C2D3E4F50001` (u64)   |

Encoded via `bincode 2` / `standard` config.

Used by tests:
- `fixture_v0_traverses_full_chain` (AC6 — V0→V1→V2 full chain)

---

### `v99_future.fwsave` (1 byte)

Hand-crafted bytes: `[0x63]`.

This is the bincode-2 varint for discriminant 99. Since 99 < 128, no
continuation bit is needed; the entire "save file" is the single byte `0x63`.
`load_envelope` must reject this with `SaveError::Decode` whose message
mentions both `"99"` and `"variant"` (bincode 2's serde adapter error shape:
`invalid value: integer \`99\`, expected variant index 0 <= i < 3`).

This fixture exercises the forward-incompat-failure path: an old binary must
loudly reject a save written by a future binary it doesn't understand.

Used by tests:
- `fixture_v99_fails_loudly` (AC4 — forward-incompat-failure)

---

### `v2_nonempty_ledger_sample.fwsave` (73 bytes)

A `SaveEnvelope::V2` save with a NON-EMPTY ledger. Wire tag: `0x02`.

| Field                  | Value                                            |
|------------------------|--------------------------------------------------|
| `career_seed`          | `0x7E57C0DE00020003` (u64)                       |
| `content_pack_version` | `1` (u32)                                        |
| `ledger`               | 2 plain `MemoryEvent`s + 1 `Compaction` (3 rows) |

The ledger is built by appending a season-0 `DebutSenior` event and a season-5
`LegacyGoal` event, then calling `compact(SeasonNumber(5))` — which nulls the
season-0 event's `tick` and appends one `Compaction` event.

This is the ONLY frozen fixture with a non-empty ledger. The `v0`/`v1` fixtures
carry empty ledgers, so their round-trip tests never exercise the `MemoryEvent`
serde surface. A backward-compat regression in `MemoryEvent` encoding (a field
reorder, an `EventClass` discriminant shift, a varint-encoding change) is caught
here against frozen bytes where the empty-ledger fixtures stay silent.

Used by tests:
- `fixture_v2_nonempty_ledger_decodes` (AC7 — decode + row / Compaction count)
- `fixture_v2_nonempty_round_trip_byte_identical` (AC7 — round-trip-byte-identical)
- `fixture_v2_nonempty_loads_and_restores_transient_state` (AC7 — load + transient-state restore)

---

## Regeneration

Run this once to bootstrap or re-pin after a schema bump:

```sh
cargo test -p fw-save --test migration_fixtures_test -- --ignored regenerate_fixtures
```

Then commit the four `.fwsave` files. After regeneration, verify all eight
committed-fixture verifier tests still pass:

```sh
cargo test -p fw-save --test migration_fixtures_test
```

**Do not re-pin without an explicit schema bump + a decision entry in
`docs/DECISIONS.md`.** The entire point of committed fixtures is that they
are frozen — if re-encoding them produces different bytes, that is a
regression to investigate, not a regen prompt.

---

## Schema version map

| File                              | `SaveEnvelope` variant | Wire tag |
|-----------------------------------|------------------------|----------|
| `v0_sample.fwsave`                | `V0(SaveV0) = 0`       | `0x00`   |
| `v1_sample.fwsave`                | `V1(SaveV1) = 1`       | `0x01`   |
| `v2_nonempty_ledger_sample.fwsave`| `V2(SaveV2) = 2`       | `0x02`   |
| `v99_future.fwsave`               | N/A (discriminant 99)  | `0x63`   |

The current production schema is `V2` (T3-1). `V1` is the locked first real
schema (T2-9). `V0` is the fictional pre-T2-9 stub kept forever to exercise
the migration chain.

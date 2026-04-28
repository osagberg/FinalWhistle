---
description: Save migration fixture policy specification. Per-schema-bump fixture + migration-test + callback-preservation + forward-incompat-failure. Prevents schema drift before first save surfaces land.
last_verified: 2026-04-24
status: Phase 2 spec — policy locked; first fixtures authored when first schema version ships (Phase 3 for MemoryEvent minimum, Phase 6 for save-schema v2)
---

# Save Migration Fixtures — policy specification

## Purpose

Every schema bump — `MemoryEvent`, `IdentityPacket`, `SignatureSO`, `ShotTypeSO`, content-pack manifest, save-file envelope — ships with a checked-in fixture from the **previous** version plus four tests proving the migration works. The fixtures accumulate forever. Never deleted. The Phase-6 schema-v1-to-v2 save migration proves itself against a save fixture generated in Phase 3.

Without this discipline, silent save-corruption becomes the failure mode nobody catches until Phase 8 when the first EA tester loses their 20-hour career.

## Why this spec exists (not an ADR)

Architectural decisions already locked:

- `design/event-sourced-memory.md` 2026-04-24 resolution §Q5 — **load-time forward migration** (not lazy-per-read at MVP); every `MemoryEvent` carries its own `schema_version`; per-version `Migrate(event, from, to)` chain; no downgrades.
- ADR-0003 Tier-A + Tier-D — fixtures run inside `fw verify` umbrella (fast smoke) and Tier-D full-matrix (every version pair).
- `design/player-generation.md` 2026-04-24 resolution §Q5 — content-pack additive-only delta strategy; ID stability enforced.

This spec defines the **fixture file shape + test discipline + growth policy** that implements the commitments above.

## Locked decisions

- **Four tests per schema bump, not three, not "the obvious one":**
  1. **Forward migration test** — previous-version fixture loads cleanly at the current version.
  2. **Callback-eligibility preservation test** — compacted memory-events' callback tags + expiry policy survive the migration (per `design/event-sourced-memory.md` three-tier compaction rule).
  3. **Forward-incompat failure test** — a newer-schema fixture fails cleanly (no silent corruption) in an older build's `max_supported_schema_version` header check.
  4. **Round-trip test** — load + re-save at current version produces byte-identical output (locked-order serialization per the corpus spec's pattern).
- **Fixtures accumulate forever** at `MatchSim.Tests/fixtures/saves/`. Never edited. Never deleted without a SPEC decisions-log entry citing why.
- **One fixture per schema version, not per bump event.** If `MemoryEvent` goes v1 → v2 → v3, we have three fixtures: `memory-event-v1.sav`, `v2.sav`, `v3.sav`. The v1→v3 migration is tested via the chain `v1.sav → v2.sav → v3.sav` at load time.
- **Synthetic, not captured:** fixtures are deterministically generated from a known seed + known input events. Human inspection of a fixture is via `fw save-inspect <fixture>` (Phase-6 tooling).

## Fixture file shape

### Path convention

```
MatchSim.Tests/
  fixtures/
    saves/
      memory-event-v1.json             # hot log, canonical JSON form
      memory-event-v1-compacted.json   # after compaction pass
      memory-event-v2.json
      memory-event-v2-compacted.json
      identity-packet-v1.json
      signature-so-v1.json
      shot-type-so-v1.json
      save-envelope-v1.json            # wraps all schemas for full-save test
      save-envelope-v2.json
      ...
```

Naming convention: `<schema-name>-v<version>[<-variant>].json` — `variant` reserved for edge cases (`-compacted`, `-pre-callback-change`, etc.).

### Fixture schema

Each fixture file is a JSON document matching the runtime save shape at the labelled schema version. Key invariants:

```jsonc
{
  "fixture_schema_version": 1,          // spec version, NOT the subject schema version
  "subject": "memory-event",            // which schema this fixture exercises
  "subject_version": 1,                 // the version of the subject schema captured here
  "generated_at": "2026-XX-XX",
  "generated_by": {
    "seed": "0x<16 hex>",
    "content_pack_version": "fwh.core@1.0.0",
    "tool": "scripts/fw save-fixture --generate",
    "matchsim_commit": "<sha>"
  },
  "description": "Minimal hot-log with 3 MemoryEvents spanning all three compaction tiers.",
  "payload": {
    // ... the actual schema-v<N> save data. Shape is schema-version-specific.
  }
}
```

Hash of `payload` at time of authoring is stored next to the fixture in `MatchSim.Tests/fixtures/saves/hashes.json` for round-trip verification.

## Test shape (per schema bump)

For every schema bump from `v<N>` to `v<N+1>`, the PR that introduces the bump MUST add:

### 1. Forward migration test

```csharp
[Fact]
public void MemoryEventV1_LoadsAtV2()
{
    var raw = FixtureLoader.Read("memory-event-v1.json");
    var migrated = MigrationChain.LoadAtCurrent(raw);
    Assert.Equal(2u, migrated.SchemaVersion);
    AssertCallbackEligibilityPreserved(raw, migrated);
}
```

### 2. Callback-eligibility preservation test

Explicitly checks that compacted events' callback tags + `min_band` + `expiry_policy` survive. Non-trivial because compaction itself may drop fields; the test ensures the migration doesn't drop MORE than compaction alone drops.

### 3. Forward-incompat failure test

```csharp
[Fact]
public void MemoryEventV2_RejectedByV1Build()
{
    // Simulate a v1 build by setting max_supported_schema_version = 1.
    var raw = FixtureLoader.Read("memory-event-v2.json");
    var ex = Assert.Throws<SaveIncompatibleException>(
        () => MigrationChain.LoadWithMaxVersion(raw, maxVersion: 1u));
    // Must fail explicitly with the missing-content-badge signal per memory doc,
    // not silently corrupt.
    Assert.Equal(SaveIncompatibleReason.SchemaTooNew, ex.Reason);
}
```

### 4. Round-trip test

```csharp
[Fact]
public void MemoryEventV2_RoundTripsByteIdentical()
{
    var raw = FixtureLoader.Read("memory-event-v2.json");
    var loaded = Serializer.Load(raw);
    var resaved = Serializer.Save(loaded);
    Assert.Equal(raw, resaved);   // byte-identical; serialization order is locked
}
```

## CI contract

### Tier A (Phase 3+)

One **smoke** fixture per active subject schema runs in `fw verify`. Covers the most-recent forward-migration + round-trip.

```
fw verify → fw save-migration-smoke
          → loads latest fixture per subject
          → runs forward-migration + round-trip
          → <10s total, Linux-only
```

### Tier D (RC, Phase 8)

Full migration matrix: every (v<N>, v<M>) pair where N < M for every subject schema. If `MemoryEvent` has versions v1 / v2 / v3, Tier D runs:
- v1 → v2, v1 → v3, v2 → v3
- Plus forward-incompat tests for (v2, max=1) and (v3, max=2) and (v3, max=1)

Expected runtime: seconds to low-minutes per subject schema; scales with schema-version count.

## Growth policy

### When a fixture gets added

- **Every schema-version bump** → one fixture captured at the PRIOR version before the bump merges. The bumping PR is NOT merged until the fixture + 4 tests land.
- **Every save-corruption bug found in the wild** → fixture captured from the corrupted save (or a synthetic reproduction) plus a regression test before the fix merges.
- **Every forward-incompat scenario encountered** → add the scenario as a test case against the existing fixture (no new fixture needed).

### When a fixture never gets deleted

- Fixtures are **append-only**. An older schema becoming "obsolete" does NOT justify deletion — the migration chain needs every historical fixture to prove no-gap coverage.
- A fixture that's become unusable (e.g., shipped with a schema field that was dropped entirely in a later version) gets flagged `"archived": true` in its JSON and its tests stop running in CI. The file stays.

### Fixture-count budget

- **Phase 3** (per SPEC 2026-04-28 enforcement-skeleton-rollout decisions-log entry): **placeholder-only** — directory + sentinel + `fw save-migration-test` CLI stub that exits 0 when no fixtures present. **No real fixtures yet.** First real fixture lands the moment any subject schema actually ships in Phase 4 (most likely candidate: `MemoryEvent` v1 when the first reader callback is implemented per the 2026-04-28 semantic-slice scope decision).
- Phase 4: 1 fixture per subject schema actually used in the slice (likely `MemoryEvent`, possibly `IdentityPacket` subset if its compiler-shaped JSON gets a schema bump). Fixture authoring proceeds per the 4-test discipline (forward migration + callback-preservation + forward-incompat + round-trip) starting from this point.
- Phase 6 save-schema v2 bump: ~10 fixtures total (the previous "Phase 3 ends with ~5 fixtures" target is superseded; the count grows as schemas actually bump).
- Phase 8 EA: ~15 fixtures.
- Ceiling signal: if fixture count climbs past 50 before Phase 8, fixtures have become too granular — evaluate consolidation (NOT deletion).

## Authoring a fixture (operator runbook)

1. Before merging a schema bump PR, run `scripts/fw save-fixture --subject <schema> --version <prior>` (Phase-6 tooling; synthesize at Phase 3 with a manual author script).
2. Verify the generated fixture loads with the PRIOR version's build (`fw save-fixture --verify-load <path>`).
3. Add the 4 required tests (forward migration + callback-preservation + forward-incompat + round-trip).
4. Commit fixture + tests in the same PR as the schema bump. **Schema bumps without fixtures don't merge.**
5. After merge, `fw verify` runs the Tier-A smoke subset on every subsequent push — any drift in the migration path trips CI immediately.

## MVP boundary

At Phase 3 (per SPEC 2026-04-28 enforcement-skeleton-rollout decisions-log entry): **placeholder skeleton only** — fixture directory + `fw save-migration-test` CLI stub. The first real fixture lands when the first subject schema actually ships in Phase 4 (most likely `MemoryEvent` v1 when the first reader callback is implemented). Earlier draft language *"At Phase 3 Week 4: 1 fixture per subject schema for schemas actually used in the slice"* is superseded — Phase-3 schemas are MatchSim-internal canonical-state encoding (locked, not save-bearing) + the IdentityPacket fixtures (compiler-shaped JSON, validator round-trip only); no save-envelope schema bump yet.

At Phase 6 save-schema v2: full migration matrix Tier-D operational. `fw verify` absorbs the Tier-A subset without busting the 5-minute budget.

At Phase 8 EA: release-gate artifact. Shipped build ships with a fixture set accompanying the RC tag; future hotfixes can be validated against the shipped fixtures before push.

## Deferred

- **Lazy per-read migration** — ruled out at MVP per `design/event-sourced-memory.md` Q5 resolution. If Phase-6 synthetic 20-year saves prove load-time too slow, lazy is a performance optimization — at which point, a new fixture type (per-event lazy-migration edge cases) gets added. Not anticipated pre-EA.
- **Cross-pack save migration** (save from pack A loaded with pack B) — ruled out at MVP per ADR-0003 cost-posture. Future Workshop content packs may surface this; spec extension at that time.
- **Save-compression fixture variants** — if Phase 6 save-size budget forces binary packing, fixtures get a `format: "binary" | "json"` field. Deferred until binary serialization is authored.

## Cross-refs

- `design/event-sourced-memory.md` §Q5 (2026-04-24) — load-time migration policy source.
- `design/player-generation.md` §Q5 (2026-04-24) — content-pack ID stability; feeds save-envelope version metadata.
- ADR-0003 — Tier-A / Tier-D CI contracts for fixture verification.
- ADR-0004 (when landed) — MemoryEvent migration framework implementation; first real user of this spec.
- `design/specs/golden-replay-corpus.md` — sibling spec; shares the "generator owns order, humans never hand-maintain" discipline.
- `scripts/fw` — `save-fixture` + `save-inspect` + `save-migration-smoke` subcommands (all phase-gated; stub at Phase 1).

## Open questions — dependencies tracked in SPEC, not this doc

1. **Binary vs JSON save serialization** — ruled out in favor of JSON at Phase 3 per `design/event-sourced-memory.md`; revisit at Phase 6 if save size requires it. When flipped, fixture format gains a variant field. Not blocking at this spec level.
2. **Synthetic fixture generator tooling** — Phase-6 `fw save-fixture` subcommand; at Phase 3 Month-3 slice, initial fixtures can be hand-authored against the schema.

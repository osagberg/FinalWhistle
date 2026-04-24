---
description: ADR-0004 — MemoryEvent schema, callback-tag enum, compaction tiers, and migration framework. Formalizes the event-sourced-memory resolution as the ledger's architecture commitment.
---

# ADR-0004: MemoryEvent schema + CallbackTag registry + compaction tiers + migration framework

## Status

**Accepted** — 2026-04-24. Tightened on five user-review findings: SalienceInputs + SalienceModelVersion persistence for Phase-6 audit; `FinalWhistle.Memory.Contracts` / impl split to decouple MatchSim from ledger persistence; quota rounding semantics formalized; event-class starter count corrected to ~40; float-salience added as rejected alternative.

## Date

2026-04-24

## Last Verified

2026-04-24

## Decision Makers

osagberg (project owner), GPT-5.5 (design partner on 2026-04-24 event-sourced-memory resolution), Claude (workhorse).

---

## Summary

Lock the Pillar-1 ledger architecture: append-only `MemoryEvent` records with a 5-input emission-time salience formula (weights are Phase-6 tuning seeds), a `CallbackTag` registry with consuming-reader metadata for validator discipline, a three-tier compaction policy with a per-season top-5% quota cap, and a load-time forward-migration chain with per-version `Migrate(event, from, to)` composition. Integrates directly with the save-migration fixture policy (ADR-consuming artifact) and feeds `key_event_hashes` into the golden replay corpus spec.

## Engine Compatibility

| Field | Value |
|---|---|
| Engine | Engine-agnostic at the schema layer — C# + JSON; Unity sees only the Ledger facade, not the event structs |
| Domain | Core / Persistence / Narrative Systems | <!-- ui-lint:allow term="domain" reason="ADR template canonical field name for engine-compat area" reviewer="osagberg" -->
| Knowledge Risk | LOW — `System.Text.Json`, dictionary lookups, value-type records are stable stdlib. No Unity API dependency; no post-cutoff concerns |
| References Consulted | `design/event-sourced-memory.md` 2026-04-24 resolution, ADR-0001 (ShotTypeSO — ChainConditionId registry pattern reused), ADR-0003 (Production pipeline — Tier-A save-migration smoke), `design/specs/save-migration-fixtures.md`, `design/specs/golden-replay-corpus.md` |
| Post-Cutoff APIs Used | None |
| Verification Required | Phase-3 Week-2 round-trip test: emit → serialize → deserialize → re-emit produces byte-identical output per the save-migration round-trip discipline |

## Dependencies

| Field | Value |
|---|---|
| Depends On | ADR-0003 (Production pipeline — Tier-A save-migration smoke path) |
| Enables | ADR-0005 (SignatureSO — emits SignatureAwakened + SignatureExecuted events), ADR-0006 (IdentityPacket — scout-report events + phenotype-label validator surface), ADR-0007 (Scout archetype — ScoutReportConfirmed emission; ScoutReportDisagreement gated on Month-4 feel-test) |
| Blocks | Phase-3 Week-4 Month-3-slice ledger reader (alumni-DB reader is 1 of 5; emits 1 callback end-to-end); Phase-5 full 3-reader ledger; Phase-6 save-schema-v2 compaction operational |

---

## Context

### Problem Statement

`design/event-sourced-memory.md` 2026-04-24 resolved all five open questions with specific shapes: salience formula with tuning-seed weights; callback-tag schema with consuming-reader metadata; PascalCase event-class enum (~40 starter entries, ceiling 60); three-tier compaction with top-5% per-season quota capped at `N_quota`; load-time forward migration. That resolution specified what to build. This ADR formalizes the contracts across schema + registry + compaction + migration so Phase-3 implementation has no interpretation questions and so the Phase-2 `IdentityPacket` + `SignatureSO` + `Scout archetype` ADRs can cite a specific `MemoryEvent` shape when they describe their emission patterns.

### Current State

No runtime implementation yet. Design doc has the full schema draft and all five resolutions. No fixtures authored. `save-migration-fixtures.md` spec names `MemoryEvent` as its first real user but has no fixtures to act on until this ADR is Accepted.

### Constraints

- **Append-only at runtime.** No ledger mutations after emission. Every compaction writes new records; the hot log shrinks only via the compaction pass, never via in-place edit.
- **Deterministic serialization.** Cross-platform replay parity requires byte-identical serialization of the same `MemoryEvent` values. This feeds `key_event_hashes` in the golden replay corpus.
- **Load-time migration, NOT lazy per-read** at MVP per 2026-04-24 resolution §Q5. Lazy-per-read is an optimization path if Phase-6 synthetic 20-year saves prove load-time too slow.
- **Storage budget target:** <100MB per 20-year save before compression; harness warns at 50MB.
- **PEGI-12 content floor** (see `SETUP.md §7`) — event-class enum + callback-tag registry may never introduce categories that reference real people, ethnicities, mental-health language, or other floor-violating content.

### Requirements

- **Functional:** ~40 starter event classes; 5 emission-time salience inputs; `CallbackTag` with `consuming_readers` metadata; three-tier compaction; per-season quota cap; load-time forward migration chain.
- **Cross-ref-stability:** `SignatureAwakened` + `SignatureExecuted` + `ScoutReportConfirmed` + `ScoutReportDisagreement` enum names must match exact strings used in `design/signatures.md`, `design/scout-disagreement.md`, and downstream ADRs 0005-0007. Rename = schema bump.
- **Fixture-compatible:** `MemoryEvent` serialization matches `design/specs/save-migration-fixtures.md` four-test discipline (forward migration + callback-preservation + forward-incompat + round-trip).

---

## Decision

### `MemoryEvent` schema (Accepted)

Locked per `design/event-sourced-memory.md` 2026-04-24. Summary (full shape lives in the design doc; this ADR commits to it):

```csharp
public readonly struct MemoryEvent
{
    public EventId Id;                              // deterministic: career_id + sequence OR match_id + tick + event_seq
    public MatchId? MatchId;                        // null for non-match events
    public ushort Season;
    public uint? Tick;                              // null for non-match events
    public CareerDate CareerDate;                   // struct { season, week, day } — save-world time, not wall-clock
    public EventEmitter Emitter;                    // { kind: "match"|"contract"|"press"|"board"|..., source_id }
    public ImmutableArray<Participant> Participants;// [{ role, entity_id }]
    public EventClass What;                         // enum — see "Event class catalog" below
    public Fixed Stakes;                            // Q32.32 [0,1]
    public Emotion Emotion;                         // enum: Triumph | Shame | Relief | Anger | Hope | ...
    public ImmutableArray<Consequence> Consequences;// [{ kind, delta }]
    public CallbackEligibility CallbackEligibility; // { recall_after_seasons, recall_tags, expires_after_seasons? }
    public Fixed Salience;                          // Q32.32 [0,1] — computed at emission, IMMUTABLE
    public SalienceInputs SalienceInputs;           // Q32.32 breakdown of the 5 inputs — persisted alongside Salience
    public ushort SalienceModelVersion;             // which weight table produced Salience; frozen at emission
    public ushort SchemaVersion;
}

public readonly struct SalienceInputs
{
    public Fixed Stakes;                            // Q32.32 [0,1] — same value as MemoryEvent.Stakes; duplicated here for a self-contained audit record
    public Fixed ParticipantProminenceAvg;          // Q32.32 [0,1]
    public Fixed EventClassBaseWeight;              // Q32.32 [0,1]
    public Fixed RivalryBoost;                      // Q32.32 [0,1]
    public Fixed RarityBoost;                       // Q32.32 [0,1]
}
```

**Why `SalienceInputs` + `SalienceModelVersion` are persisted:** preserving only the scalar `Salience` keeps behavior frozen (append-only correctness) but loses the ability to explain or compare why an event scored 0.82 vs 0.79 three seasons later. Storing the 5 inputs + the model version used to combine them means Phase-6 balance-harness analysis can retroactively audit, and an intentional re-scoring migration is possible if ever needed — by explicit schema-bumping migration, NEVER by recomputing on load. The immutability invariant applies to `Salience`; the inputs are the audit trail.

`Fixed` = Q32.32 per `design/match-engine.md`. `Salience` is stored as Q32.32 for determinism across platforms.

### Salience formula (structure Accepted; numeric weights are Phase-6 tuning seeds)

```
salience = clamp(
    w_stakes        * stakes
  + w_prominence    * participant_prominence_avg
  + w_event_class   * event_class_base_weight
  + w_rivalry       * rivalry_boost
  + w_rarity        * rarity_boost,
  0, 1
)
```

**Emission-time only.** `callback_age` and `player_attention` from the 2026-04-22 SPEC seed are **reader-side surfacing modifiers**, recomputed per surface opportunity — never stored in the event. This is the append-only invariant.

Phase-6 tuning seeds (live in `design/event-sourced-memory.md`, NOT this ADR): `w_stakes=0.4`, `w_prominence=0.2`, `w_event_class=0.2`, `w_rivalry=0.1`, `w_rarity=0.1`. Band cutoffs: 0.3 / 0.6 / 0.85.

### `CallbackTag` registry (Accepted)

```csharp
public sealed record CallbackTag(
    ContentPackQualifiedId Id,                      // stable; mod-safe; no pack-minor per ADR-0006
    ImmutableArray<ReaderId> ConsumingReaders,      // at least one — validator-enforced
    SalienceBand MinBand,                           // enum: Routine | Notable | SeasonDefining
    ExpiryPolicy Expiry                             // discriminated union: Never | Seasons(u8) | OnEvent(EventClass)
);
```

**Every tag must declare ≥1 consuming reader. Lint-enforced.** Tags without consumers accumulate; this validator closes that drift.

MVP starter tags: `alumni`, `rival-recall`, `promise`, `big-match-scar`, `press-fan`, `derby`, `cup-heartbreak`, `youth-sold`, `contract-drama`, `board-ultimatum`, `scoring-milestone`, `upset`. Content-pack-qualified IDs. Tag registry is Phase-2 authoring; tag IDs don't embed pack-minor.

### Event class catalog (Accepted — ~40-entry starter set, ceiling 60)

Versioned PascalCase enum with monotonic integer IDs for fast-path lookup. Full enumeration lives in `design/event-sourced-memory.md` §Event-class-catalog. Growth beyond 60 triggers a Phase-review.

**Cross-doc exact-match discipline:**
- `SignatureAwakened` + `SignatureExecuted` — must match `design/signatures.md` lifecycle terminology exactly. Validated by the Phase-6 content-pack validator.
- `ScoutReportDisagreement` — conditional on Month-4 feel-test pass. If Scout Disagreement is cut, this value is dropped at a schema-version bump. `ScoutReportConfirmed` stays regardless.

### Compaction — three tiers + per-season quota (Accepted)

At 5-season boundary, each `MemoryEvent` is routed by band:

| Band | Rule | Callback capability after |
|---|---|---|
| `salience ≥ 0.85` (**season-defining**) | **HARD PRESERVE.** Full record retained forever. | Any reader, any callback type |
| `0.6 ≤ salience < 0.85` (**notable**) | **COMPACT PRESERVE.** Retains participants, what, emotion, tags, career_date, season. Drops tick + consequence-delta. | Press / fan / rival-recall readers; tick-specific callbacks degrade |
| `salience < 0.6` | **AGGREGATED ONLY.** Rolled into `CompactedMemory.aggregates` (counts, sums, streaks). | Stats-flavored readers only |

**Plus a per-season quota:** top events by salience per season are **hard-preserved regardless of band**, capped at `N_quota`. Exact formula (deterministic; no floating-point ambiguity):

```
quota_count =
    0                                               when event_count == 0
    min(N_quota, max(1, ceil(event_count * 0.05)))  otherwise
```

- **`event_count > 0` guarantees at least 1 event preserved per season** — protects the "season you barely remember" from full aggregation even in a 10-event season.
- **`ceil(count * 0.05)`** is integer-ceiling (compute as `(count * 5 + 99) / 100` against integer `count`, or via `Math.Ceiling` on `count * 0.05` — same result since inputs are small ints).
- **Tie-break when >`quota_count` events share the cutoff salience:** ascending `EventId` ordering (stable, deterministic per ADR-0001 §deterministic-selection Id-tiebreak pattern).
- **`N_quota = 20`** is a Phase-6 tuning seed (lives in `design/event-sourced-memory.md`, not SPEC-locked).

**Compaction never runs during a match.** It executes at season-end as part of save finalization; the ledger is immutable within a match.

### Load-time forward migration (Accepted)

Per 2026-04-24 §Q5 resolution:

```csharp
public static class MigrationChain
{
    // Per-version forward-migration functions registered at static-init.
    private static readonly Dictionary<(ushort from, ushort to), Func<MemoryEvent, MemoryEvent>> _migrators;

    public static MemoryEvent Migrate(MemoryEvent event, ushort toVersion)
    {
        var current = event;
        while (current.SchemaVersion < toVersion)
        {
            var step = _migrators[(current.SchemaVersion, current.SchemaVersion + 1)];
            current = step(current);
        }
        return current;
    }
}
```

**Load-time, not lazy-per-read at MVP.** Save-file header advertises `max_supported_schema_version`; newer saves fail-loudly against older builds per `design/specs/save-migration-fixtures.md` test #3. CI enforces the 4-test-per-bump discipline.

### Architecture Sketch

```
                             ┌─────────────────────────────┐
                             │  MatchSim / Press / Board / │
                             │  Contract / etc.            │
                             │  (emission sources)         │
                             └─────────────┬───────────────┘
                                           │ MemoryEvent (Q32.32 Stakes + Salience)
                                           v
                             ┌─────────────────────────────┐
                             │       LedgerAppender        │
                             │  • validates schema         │
                             │  • computes salience        │
                             │  • routes by band to hot    │
                             │    log vs debug log         │
                             └─────────────┬───────────────┘
                                           │
          ┌────────────────────────────────┼─────────────────────────────┐
          v                                v                             v
┌──────────────────┐         ┌─────────────────────────┐     ┌──────────────────────┐
│  HotLog (5 seas) │         │  SeasonEndCompactor     │     │  ReaderSubscribers   │
│  append-only     │ ──────> │  runs at save finalize  │ ──> │  (5 readers)         │
│  JSON now,       │         │  → CompactedMemory      │     │  salience + tag      │
│  binary at P6    │         │  + aggregates           │     │  filter based lookup │
└──────────────────┘         └─────────────────────────┘     └──────────────────────┘
          │
          v
      Save-file envelope w/ schema_version header
      └─> MigrationChain.Migrate on load (if needed)
```

### Key Interfaces

```csharp
public interface ILedgerAppender
{
    void Emit(MemoryEvent event);              // validates, computes salience, appends to hot log
    IReadOnlyList<MemoryEvent> SinceSeason(ushort season);
}

public interface IMemoryReader
{
    ReaderId Id { get; }
    IEnumerable<CallbackCandidate> Query(ReaderQuery q);  // filters by tag + band + season
}

public readonly struct CallbackCandidate
{
    public MemoryEvent Source;
    public Fixed SurfacingSalience;              // includes reader-side callback_age + player_attention modifiers
    public CallbackTemplateId Template;
}
```

### Implementation Guidelines

- **Asmdef / project boundaries split for decoupling:**
  - `FinalWhistle.Memory.Contracts` — pure value-type schemas: `MemoryEvent`, `SalienceInputs`, `EventClass` enum, `CallbackTag`, `ReaderQuery`. Zero logic. No Unity deps. This is the dependency surface for any system that EMITS events.
  - `FinalWhistle.Memory` — persistence / salience computation / compaction / migration / reader infrastructure. Consumes `Contracts`; owns the ledger storage + the `MigrationChain`.
- **`MatchSim.csproj` depends on `FinalWhistle.Memory.Contracts` ONLY** (emission shape). MatchSim never touches the ledger, never touches compaction, never touches migration. This keeps canonical sim strictly decoupled from career persistence — a hard line that preserves the determinism architecture per `TECH_APPROACH.md §3`.
- **Viewer depends on reader interfaces only** (consumption); also via `FinalWhistle.Memory.Contracts` + a read-side interface package.
- **Serialization order is locked** per `MatchSim.Tests/SerializationContract.cs` (Phase-3 SPEC task). Field order matches struct declaration order; `ImmutableArray` fields serialize in index order; no reflection-driven ordering.
- **Salience computation is deterministic** — all inputs are Q32.32 or enum-backed constants; no floats, no wall-clock, no random sampling.
- **Compaction is replay-seeded** if it ever needs randomness (it shouldn't; top-5% quota is ordered selection on salience, Id-tiebreaker); quota-tiebreaker rule: when >N_quota events share the cutoff salience, tie-break by event `Id` ascending.
- **Reader `callback_age` modifier** is computed as `current_season - event.Season` capped at the tag's `expires_after_seasons` if set; reader code owns this, NOT the event record.

---

## Alternatives Considered

### Alternative 1: Salience stored as mutable, recomputed per load

- **Description** — store only the inputs (stakes, prominence, etc.); recompute salience on every load with the current weight table.
- **Pros** — weight tuning in Phase 6 automatically re-salience-scores historical events.
- **Cons** — breaks append-only invariant. A pre-weight-tune save has salience band X; post-tune it has band Y; reader queries become non-reproducible. Violates the "consequences stick" pillar — a decision's weight should be frozen at the moment it was made.
- **Rejected because** — append-only + pillar-1 correctness outweighs tuning convenience. If Phase-6 balance-harness requires historical re-scoring, do it as an explicit migration that creates a new event version citing the tuning change.

### Alternative 2: Single-tier "keep everything, just compress"

- **Description** — no three-tier compaction; binary-pack every event and keep forever.
- **Pros** — simplest policy; no information loss.
- **Cons** — storage explodes at 20-year saves (estimate: ~1-5MB per season uncompressed → 20-100MB for 20-year careers even with binary packing). Callback queries over 20 years of every-event data become slow. Three-tier compaction is the pragmatic compromise that preserves callback-essential fields while shedding bytes.
- **Rejected because** — storage target (<100MB per 20-year save) is load-bearing for save-file portability across Workshop content packs.

### Alternative 3: Lazy-per-read migration

- **Description** — don't migrate on load; migrate each `MemoryEvent` when a reader actually reads it.
- **Pros** — fast load for huge saves.
- **Cons** — spreads schema complexity into every reader; bugs become intermittent (only reproducible when a specific read path hits an unmigrated event); test surface explodes across reader × schema-version combinations.
- **Rejected because** — solo-dev complexity budget. Load-time migration catches schema issues loudly at load, not quietly in a rare reader code path. Revisit ONLY if Phase-6 synthetic 20-year saves prove load-time too slow; at that point, `lazy-per-read` becomes a performance optimization with explicit incident justification, not a default choice.

### Alternative 4: Float `Salience` (not Q32.32)

- **Description** — Store `Salience` as `float` (or `double`) instead of Q32.32. Recompute at emission time from `float` inputs.
- **Pros** — Smaller memory footprint; simpler arithmetic; natural interop with C# math libraries.
- **Cons** — Cross-platform IEEE-754 behavior differs in subtle corners (denormals, rounding-mode defaults, fused multiply-add availability). `MemoryEvent` serialization feeds `key_event_hashes` in the golden replay corpus per ADR-0003 + `design/specs/golden-replay-corpus.md`; a `float` Salience means the same event stream might hash differently on Win vs Mac vs Linux. Determinism across platforms is the whole point of the canonical sim — `Salience` sitting outside that discipline would be a slow-burn bug that only surfaces when replay parity is checked.
- **Rejected because** — `Salience` is part of the replay-hashed event record. Q32.32 matches the rest of the canonical sim's fixed-point posture per `design/match-engine.md`. The memory-footprint saving is irrelevant at event volumes (hundreds per match) compared to the determinism cost.

### Alternative 5: String-tag callback registry (no `CallbackTag` struct)

- **Description** — callbacks reference tags as strings; no struct, no consuming-reader validation.
- **Pros** — simpler serialization.
- **Cons** — typos silently become new tags; unused tags accumulate; there's no way to validate "this tag has a reader that consumes it." Exactly the drift mode the `CallbackTag` registry closes.
- **Rejected because** — the validator discipline is the point. Typo'd tags shipping in a content pack = silent callback failures that surface only in specific replay edge cases.

---

## Consequences

### Positive

- Append-only invariant preserved via stored immutable salience + reader-side modifier split.
- `CallbackTag` registry with `consuming_readers` metadata closes silent tag drift — lint validator rejects unconsumed tags at content-pack compile time.
- Three-tier compaction + top-5% quota hits the storage budget while preserving the memorable-events-per-season floor that Pillar-1 needs.
- Load-time migration matches the 4-test-per-bump save-migration-fixture discipline; CI catches schema issues loudly.
- Deterministic serialization feeds `key_event_hashes` in the golden replay corpus — cross-platform replay parity is testable without the viewer running.

### Negative (Accepted Tradeoffs)

- Schema-version bumps require 4 tests + a fixture per bump (save-migration fixture policy). Non-trivial process overhead — pays back through zero-silent-save-corruption.
- `CallbackTag.ConsumingReaders` requires a registration step per tag × per reader — some authoring friction. Mitigated by Phase-6 validator catching missing registrations at compile time.
- Reader-side `SurfacingSalience` recomputation means surface-time behavior varies as game state changes (e.g., `callback_age` grows). Intentional for Pillar-1 (events that haven't fired in 3 seasons surface stronger) but adds a subtle surface-time determinism concern — mitigated because surface-time salience is never persisted.

### Neutral

- JSON serialization at Phase 3 (vs binary at Phase 6) is a policy decision inside this ADR, reversible per Phase-6 save-size evaluation; binary flip is a schema-version bump + 4 fixture tests.
- ~40 starter event classes + ceiling 60 is a design-doc authoring decision, not an ADR-level commitment — ADR locks the mechanism (versioned enum, validator-checked), not the specific count.

---

## Performance Implications

| Metric | Target | Notes |
|---|---|---|
| Emission throughput | >1k events / sec on mid-range hardware | MatchSim peak is ~10-100 events / match × 380 matches / season |
| Compaction pass (per season) | <500ms on 20-year save at Phase 6 | Scales with hot-log event count; top-5% quota is O(n log n) sort |
| Load-time migration (20-year save) | <2 sec cold load | Scales with migration chain length × event count; single-step migrations are O(n) |
| Serialization size (per season) | <500KB compressed | Derived from the <100MB / 20-year budget |
| Reader query | <50ms for "all events tagged X in last 5 seasons" | Hot-log indexed by tag; cold path uses `CompactedMemory.hard_preserved` |

Actuals land at Phase 6; targets are authoring-time estimates.

---

## GDD Requirements Addressed

| GDD | System | Requirement | How This ADR Satisfies It |
|---|---|---|---|
| `design/event-sourced-memory.md` 2026-04-24 | Ledger architecture | Append-only, salience-scored, three-tier compaction, load-time migration | Schema + formula + compaction + migration all locked here |
| `design/event-sourced-memory.md` Q1 | Salience structure | Formula + band semantics locked; numeric weights are Phase-6 tuning | Emission-time inputs locked; reader-side modifiers separate |
| `design/event-sourced-memory.md` Q2 | Callback tags | Fixed MVP enum + consuming-reader metadata | `CallbackTag` record with registry discipline |
| `design/event-sourced-memory.md` Q3 | Event class catalog | Versioned PascalCase enum, ~40 starter | Enum registration pattern; growth gated past 60 |
| `design/event-sourced-memory.md` Q4 | Compaction | Three tiers + per-season quota | Compactor algorithm in this ADR |
| `design/event-sourced-memory.md` Q5 | Migration | Load-time forward, no downgrades | `MigrationChain.Migrate` interface |
| `design/specs/save-migration-fixtures.md` | Schema-bump discipline | 4 tests per bump | This ADR is the first real user |
| `design/specs/golden-replay-corpus.md` | Cross-platform hash parity | `key_event_hashes` field | Deterministic `MemoryEvent` serialization |

---

## Migration Plan

Not applicable — greenfield ledger. First real migration exercise happens when `MemoryEvent` v1 → v2 occurs (likely Phase-6 save-schema v2 bump). At that point, the save-migration-fixture policy is the operational discipline.

**Rollback:** if any of the three compaction tiers proves wrong in Phase-6 testing (e.g., callback readers need data we've dropped in the "notable" tier), supersede via a new ADR. Tier semantics are data-shape commitments, not throw-away implementation details — change = migration.

---

## Validation Criteria

- [ ] Phase 3 Week 2: `MemoryEvent` struct + `EventClass` enum + `CallbackTag` record authored; round-trip serialization test passes for the MVP starter event set.
- [ ] Phase 3 Week 3: one reader (Alumni DB) operational; Month-3 slice emits 1 callback end-to-end.
- [ ] Phase 3 Week 4: first save-migration fixture (`memory-event-v1.json`) checked in per `design/specs/save-migration-fixtures.md` discipline.
- [ ] Phase 5: 3 readers operational (Alumni DB + rival recall + big-match scars); salience thresholds tuned against playtest feedback.
- [ ] Phase 6: compaction runs on 20-year synthetic save; storage stays under 100MB; save/load cycle round-trips cleanly.
- [ ] Phase 6: first schema bump (v1 → v2) exercises the 4-test discipline end-to-end.
- [ ] All 5 readers operational by Phase 7 polish pass.
- [ ] `CallbackTag` validator catches unconsumed tags in a red-team content-pack test.
- [ ] Cross-platform `key_event_hashes` parity verified on Win/Mac/Linux Tier-A CI matrix for the golden replay corpus seeds.

---

## Related

- Depends on: ADR-0003 (Production pipeline — Tier-A save-migration smoke).
- Enables: ADR-0005 (SignatureSO — emits SignatureAwakened + SignatureExecuted), ADR-0006 (IdentityPacket — phenotype labels + content-pack ID rules shared with `CallbackTag.Id`), ADR-0007 (Scout archetype — `ScoutReport*` events).
- Cross-refs: `design/event-sourced-memory.md` 2026-04-24 resolution (source), `design/specs/save-migration-fixtures.md` (4-test discipline), `design/specs/golden-replay-corpus.md` (key-event hashes), `design/match-engine.md` (Q32.32 for Stakes + Salience), `design/signatures.md` + `design/scout-disagreement.md` (cross-doc exact-match enum names).
- Code (once implemented): `src/FinalWhistle.Memory/` (paths tentative, finalize Phase 3 bootstrap).

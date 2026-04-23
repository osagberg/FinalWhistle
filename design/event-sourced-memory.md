---
description: Career memory ledger architecture, readers, salience scoring, compaction. The load-bearing system behind "careers that remember."
last_verified: 2026-04-22
status: scaffolded; awaiting Phase 2 lock
---

# Event-Sourced Career Memory

## Purpose

Answer "how do consequences actually stick across 10-in-game-year careers, surfacing years later as callbacks that change decisions, without storage exploding or narrative becoming event-spam?"

## Locked decisions

See SPEC.md 2026-04-22. Summary:

- **Single ledger.** Every meaningful event emits one structured record. Five reader subsystems are readers, not separate stores.
- **Append-only at runtime.** No ledger mutations.
- **Salience-scored.** Not every event becomes player-facing; only high-salience surfaces.
- **Compaction at 5-season boundary.** Hot log for recent seasons; summarized state for older careers (preserves callback eligibility; drops tick-level granularity).
- **Narrative ceiling: 5-8 events per season.** Enforced by salience threshold, not authoring limit.

## Ledger record schema (draft — Phase 2 lock)

```
MemoryEvent {
    event_id: stable UUID (content-pack-qualified)
    match_id: nullable (only for in-match events)
    season: u16
    tick: u32 nullable
    timestamp: ISO8601
    emitter: { kind: "match" | "contract" | "press" | "board" | ..., source_id: string }
    participants: [
        { role: "scorer" | "assist" | "opponent" | "fan_base" | ..., entity_id: string }
    ]
    what: enum (GoalScored, SignatureAwakened, ContractBroken, DerbyHumiliation, CupFinalLoss, YouthSold, PromiseMade, ...)
    stakes: f32 [0, 1]
    emotion: enum (Triumph, Shame, Relief, Anger, Hope, ...)
    consequence: [
        { kind: "attribute_change" | "identity_packet_update" | "relationship_state_shift" | "club_finances", delta: { ... } }
    ]
    callback_eligibility: {
        recall_after_seasons: u8 (earliest a reader can surface this)
        recall_tags: [string]  // for reader filtering
        expires_after_seasons: u8 nullable
    }
    salience: f32 [0, 1]  // computed at emission
    schema_version: u16
}
```

Salience computation (draft):

```
salience = clamp(
    0.4 * stakes
  + 0.2 * participant_prominence_avg
  + 0.2 * event_class_base_weight
  + 0.1 * rivalry_boost
  + 0.1 * rarity_boost,
    0, 1
)
```

Thresholds (draft):
- `< 0.3`: logged only; never surfaced
- `0.3–0.6`: readers may surface in routine contexts
- `0.6–0.85`: eligible for press / fan callback
- `>= 0.85`: eligible for season-ending narrative beat / cutscene-adjacent cinematic moment

## Five readers

Readers query the ledger and produce surfacing decisions. They don't store state beyond ledger references.

### Reader 1 — Alumni DB
Query: "for opponent team, any player who ever played for user's club?" Returns alumni facts for rendering in `player-isolation` pre-match overlay + commentary callbacks.

### Reader 2 — Rival recall
Query: "for this fixture, any prior high-salience scoreline or event between these clubs?" Returns list of ledger events with recall text ("last time Hartfield visited, they scored in the 94th minute").

### Reader 3 — Promise tracking
Query: "at contract talks with player X, surface all PromiseMade events where X was participant." Returns promise ledger for display in contract UI; breaking promises emits new BrokenPromise events.

### Reader 4 — Big-match scars
Query: "is this a high-stakes match? surface the last 1-2 high-salience events matching stake-class." Used for pre-match press overlay + `aftermath-freeze` text.

### Reader 5 — Press / fan callbacks
Query: "for this press-conference / fan-sentiment surface, relevant ledger events in recent seasons?" Returns event list filtered by callback_tags for runtime template slot-filling.

## Compaction strategy

Default boundary: 5 seasons.

**Hot log (last 5 seasons):** full fidelity, tick-level, all events. Lives in JSON array in save file. Fast to query; grows O(events × seasons).

**Compacted state (older than 5 seasons):** summary records per entity, preserving callback-eligible facts but dropping sub-event granularity:

```
CompactedMemory {
    entity_id: string
    career_span: [u16, u16]  // first/last season
    salience_events: [MemoryEvent]  // only salience >= 0.6 preserved intact
    aggregates: { goals, finals_lost, promises_broken, derby_wins, ... }
}
```

Queries against compacted state use aggregates + the preserved-salience-events subset. Readers gracefully degrade: some callback types (tick-level specifics) drop out for ancient events; salience-preserved callbacks remain forever.

Storage estimate: 50K players × 10 seasons × ~20 salience-preserved events = 10M records in compacted storage per save. ~1-2KB per record → ~10-20GB per save. **Too much.** Compaction must be tighter:

- Only STORE events with salience >= 0.3 in hot log (drops the noise from the start)
- Compacted state: keep only salience >= 0.75 events + aggregates
- Realistic save size: ~50MB per 20-year save (feasible)

## MVP boundary

At Month 3 slice: ledger exists. 1 reader implemented (Alumni DB). 1 memory callback demonstrates surfacing works end-to-end.

At Month 5 vertical slice: 3 readers operational (Alumni DB, rival recall, big-match scars).

At Month 12 EA: all 5 readers. Salience tuning via balance harness. Compaction implemented + tested with synthetic 20-year career.

## Deferred

- Cross-save ledger (your club's events visible in other saves) — post-1.0
- Community ledger sharing / "Legend Exchange" — post-1.0
- LLM-generated ledger prose — NEVER at runtime; bake-time templates only

## Open questions (Phase 2 lock)

1. **Salience computation weights** — above is draft; tune via balance harness output in Phase 6.
2. **Callback tag taxonomy** — fixed set or extensible? Recommend fixed at MVP for tooling; extensible via content packs post-EA.
3. **Event class catalog** — how many distinct `what` values? Proposal: 40-60 at MVP (enumerated + versioned). Avoid runaway growth.
4. **Compaction semantic preservation** — what attributes define "this specific event is preserved intact beyond 5 seasons"? Proposal: top-5% salience per season, always; everything else compacts.
5. **Save-load ledger migration** — when schema_version bumps, how is ledger migrated? Every MemoryEvent carries its own schema_version; migration functions handle old events on load.

## Prototype gate

**Phase 3 Week 4:** ledger operational in Month-3 slice. 1 event emitted per meaningful MatchSim action; alumni-DB reader surfaces 1 callback in post-match screen.

**Phase 5 gate:** 3 readers + salience thresholds operational; balance harness sweeps confirm "right 5-8 events surface per season" holds across 10K simulated seasons.

**Phase 6 gate:** compaction validated on 20-year synthetic career; save size acceptable; query perf under target.

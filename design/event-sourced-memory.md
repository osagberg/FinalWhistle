---
description: Career memory ledger architecture, readers, salience scoring, compaction. The load-bearing system behind "careers that remember."
last_verified: 2026-04-24
status: Phase 0 open questions resolved; salience structure + callback-tag schema + event-class starter set + three-tier compaction + load-time migration locked. One Phase-2 ADR pre-seeded.
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
    event_id: deterministic runtime id (career_id + sequence or match_id + tick + event_seq)
    match_id: nullable (only for in-match events)
    season: u16
    tick: u32 nullable
    career_date: { season: u16, week: u8, day: u8 }  // save-world time, not wall-clock
    emitter: { kind: "match" | "contract" | "press" | "board" | ..., source_id: string }
    participants: [
        { role: "scorer" | "assist" | "opponent" | "fan_base" | ..., entity_id: string }
    ]
    what: EventClass enum (see "Event class catalog" below — versioned PascalCase, stable int IDs)
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

**Salience formula (structure locked; weights are Phase-6 tuning seeds):**

```
salience = clamp(
    w_stakes * stakes
  + w_prominence * participant_prominence_avg
  + w_event_class * event_class_base_weight
  + w_rivalry * rivalry_boost
  + w_rarity * rarity_boost,
    0, 1
)
```

**Reconciling with the 2026-04-22 SPEC seed** (`stakes x rarity x character involvement x rivalry x callback age x player attention`): the 5 inputs above are **emission-time** salience — computed when the event is written. `callback_age` and `player_attention` are **reader-side surfacing modifiers**, applied when a reader decides whether to surface an existing ledger event right now. Emission-time salience is an immutable property of the event record; surfacing salience is recomputed per reader per surface opportunity. This separation is intentional — storing a time-varying salience in the ledger would violate the append-only discipline.

**Semantic threshold bands (locked; numeric cutoffs are Phase-6 tuning seeds):**

| Band | Purpose |
|---|---|
| **debug-only** | below write-to-ledger cutoff — stays in match telemetry / debug logs, not written to career ledger |
| **routine** | written to hot log; readers may surface in routine contexts |
| **notable** | eligible for press / fan callback |
| **season-defining** | eligible for season-ending narrative beat / cinematic emphasis |

**Phase-6 tuning seeds (NOT SPEC-locked):**
- `w_stakes = 0.4`, `w_prominence = 0.2`, `w_event_class = 0.2`, `w_rivalry = 0.1`, `w_rarity = 0.1`
- band cutoffs: `0.3 / 0.6 / 0.85`

Expect all of these to re-tune during Phase-6 balance-harness sweeps. They are fixed-step / fixed-formula tuning constants at current salience semantics; if the formula changes, re-derive.

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

## Callback tag schema

Callback tags are a **fixed MVP enum with consuming-reader metadata**, extensible post-EA via schema / content-pack versioning.

```
CallbackTag {
  id: ContentPackQualifiedId            // stable; mod-safe
  consuming_readers: [ReaderId]         // which readers may filter on this tag
  min_band: enum { routine, notable, season-defining }   // minimum salience band for surfacing
  expiry_policy: enum { never, seasons(u8), on-event(EventClass) }
}
```

Each tag declares which readers can consume it. Tags without at least one consuming reader are invalid (lint-checked). This prevents useless tags accumulating and guarantees every tag has a consumer.

`MemoryEvent.callback_eligibility.recall_tags` references these CallbackTag ids. The tag's `min_band` + `expiry_policy` are the defaults; the event's eligibility block may override per-event for specific exceptions (e.g., a catastrophic ledger entry never expires regardless of its tag's default).

**MVP starter tag set (indicative; Phase-2 ADR finalizes):**
`alumni`, `rival-recall`, `promise`, `big-match-scar`, `press-fan`, `derby`, `cup-heartbreak`, `youth-sold`, `contract-drama`, `board-ultimatum`, `scoring-milestone`, `upset`.

## Event class catalog

Versioned PascalCase enum with stable integer IDs. **MVP target ~35-40**, ceiling ~60. Growth beyond ~60 requires a Phase-review — runaway growth dilutes reader efficacy and bloats the schema-migration surface.

**Starter set (Phase 0 lock):**

| Group | Events |
|---|---|
| Match outcomes | `GoalScored`, `GoalConceded`, `Assist`, `MatchWon`, `MatchLost`, `MatchDrawn` |
| Signature life | `SignatureAwakened`, `SignatureExecuted` |
| Season shape | `Promotion`, `Relegation`, `TitleWon`, `TitleLost`, `CupFinalWon`, `CupFinalLost`, `DerbyWon`, `DerbyLost`, `UpsetAchieved`, `UpsetSuffered` |
| Rivalries / scars | `RivalryIntensified`, `BigMatchScarCreated`, `BigMatchScarAvenged` |
| Contracts / promises | `PromiseMade`, `PromiseKept`, `PromiseBroken`, `ContractExtended`, `ContractBroken` |
| Transfers / youth | `PlayerSigned`, `PlayerSold`, `YouthPromoted`, `YouthSold` |
| Injuries | `Injury`, `InjuryRecovered` |
| Scouting | `ScoutReportDisagreement`, `ScoutReportConfirmed` |
| Press / fan / board | `PressConferenceWin`, `PressConferenceLoss`, `FanSentimentBoost`, `FanSentimentCollapse`, `BoardUltimatum`, `BoardConfidenceShift` |
| Coaching | `ManagerHired`, `ManagerFired` |

Count: ~38. Room for ~20 Phase-5/6 additions without hitting the soft ceiling.

**Conditional drops / cross-refs:**
- `SignatureAwakened` + `SignatureExecuted` must match exact strings used in `design/signatures.md`. Cross-doc consistency check required at Phase 2.
- `ScoutReportDisagreement` presupposes the Scout Disagreement system. If the Month-4 feel-prototype gate kills that system, this value is dropped at a schema-version bump; `ScoutReportConfirmed` stays as long as basic scouting exists.

## Compaction strategy

Default compaction boundary: **5 seasons.**

**Hot log (last 5 seasons):** full fidelity for career-relevant events only, with optional tick references into match replay data. Routine passes, touches, position samples, and low-salience match telemetry never enter the career ledger. Lives as structured save data; JSON is acceptable during development, with a compact binary form evaluated at Phase 6 if save size requires it.

### Three-tier preservation rule

| Band | Rule | Callback capability after compaction |
|---|---|---|
| `salience >= 0.85` (**season-defining**) | **HARD PRESERVE.** Full event record: participants, tags, emotion, consequence, source context. Tick-level telemetry NOT retained — tick detail lives in match replay data, not the career ledger. | Any reader, any callback type, forever |
| `0.6 <= salience < 0.85` (**notable**) | **COMPACT PRESERVE.** Preserved in the compacted record with callback-essential fields only: participants, what, emotion, tags, career_date, season. Consequence-delta + source detail dropped. | Press / fan / rival-recall readers. Tick-specific callbacks gracefully degrade. |
| `salience < 0.6` | **AGGREGATED ONLY.** Rolled into `CompactedMemory.aggregates` (counts, sums, streaks). No per-event record. | No per-event callback; aggregates feed stats-flavored readers only. |

### Per-season quota rule

Additionally, at compaction: **top 5% salience events per season are hard-preserved regardless of band, capped at `N_quota` events per season.**

- `N_quota` starting seed: **20** events (NOT SPEC-locked — Phase-6 tuning target).
- Rationale: the cap prevents a noisy season from bloating the save; the quota prevents a low-drama season from losing every event through compaction. Lower bound protects "the season you barely remember" from becoming aggregate-only; upper bound protects the save file.

### CompactedMemory record

```
CompactedMemory {
  scope_id: string  // player, club-pair, player-club, competition, promise-chain
  career_span: [u16, u16]  // first/last season (inclusive)
  hard_preserved: [MemoryEvent]   // season-defining + top-5% quota hits (full record)
  compact_preserved: [MemoryEventCompact]  // notable band; callback-essential fields only
  aggregates: { goals, finals_lost, promises_broken, derby_wins, streak_data, ... }
  compaction_schema_version: u16
}

MemoryEventCompact {
  // subset of MemoryEvent fields: event_id, season, career_date, emitter.kind,
  // participants, what, emotion, callback_eligibility.recall_tags, salience,
  // schema_version.
  // NOT carried: tick, source_id detail, consequence deltas.
}
```

Queries against compacted state use aggregates + `hard_preserved` + `compact_preserved`. Readers gracefully degrade by band — season-defining callbacks remain forever; notable callbacks lose tick-level specifics.

### Storage targets

Active EA universe is ~2,000-2,400 players, not 50K. Still, long saves can explode if routine match actions are logged. The ledger therefore stores only career-relevant facts:

- Only write events at or above the `routine` band to the hot career ledger.
- Keep low-level match telemetry in replay / debug files, NOT the career save.
- Compact older seasons per the three-tier + quota rules above.
- **Hard target:** <100MB per 20-year save before compression; harness warns if synthetic saves exceed 50MB.

## Save / load migration

**Load-time forward migration** (NOT lazy per-read).

- Every `MemoryEvent` carries its own `schema_version: u16` at emission.
- On save load: migrate the save envelope and ALL `MemoryEvent` records to the current runtime schema version before gameplay starts. Re-save on next save point at the current version.
- Per-version forward-migration: `Migrate(MemoryEvent, from_version, to_version)` applies a chain of `v{N} → v{N+1}` transforms.
- **No downgrades.** A save written at schema version `N+1` cannot be loaded in a build that supports only up to `N`. Game build advertises `max_supported_schema_version` on the save-file header for pre-load validation.
- Save file header includes `min_event_schema_version` + `max_event_schema_version` hints — fast-path skip when both equal current (no migration needed).
- **CI requirement (Phase-6 onward):** every schema bump requires a migration test — a synthetic save at the prior version loads cleanly at the current version and preserves callback eligibility for all preserved events.

**Why load-time, not lazy per-read (at MVP):** lazy migration spreads schema-complexity into every reader; bugs become intermittent (only reproducible when a read path hits an unmigrated event); test surface explodes. Load-time migration is simpler, catches migration bugs at load (loudly, once), and keeps readers version-naïve. If Phase-6 synthetic 20-year saves prove load-time migration too slow, introduce lazy / indexed migration as a **performance optimization**, not an architecture default.

## MVP boundary

At Month 3 slice: ledger exists. 1 reader implemented (Alumni DB). 1 memory callback demonstrates surfacing works end-to-end.

At Month 5 vertical slice: 3 readers operational (Alumni DB, rival recall, big-match scars).

At Month 12 EA: all 5 readers. Salience tuning via balance harness. Compaction implemented + tested with synthetic 20-year career.

## Deferred

- Cross-save ledger (your club's events visible in other saves) — post-1.0
- Community ledger sharing / "Legend Exchange" — post-1.0
- LLM-generated ledger prose — NEVER at runtime; bake-time templates only

## Resolved (2026-04-24)

See SPEC.md decisions log entry `2026-04-24 — Event-sourced memory open questions resolved`. All five questions resolved; one Phase-2 ADR pre-seeded.

1. **Salience structure locked; weights are Phase-6 tuning seeds.** See "Salience formula" section above. Reconciliation note: `callback_age` + `player_attention` from the 2026-04-22 SPEC seed are **reader-side surfacing modifiers**, not emission-time salience inputs — separation preserves ledger append-only discipline.
2. **Callback tags: fixed MVP enum with consuming-reader metadata.** See "Callback tag schema" section. `CallbackTag { id, consuming_readers, min_band, expiry_policy }`. Every tag must declare at least one consuming reader (lint-checked).
3. **Event class catalog: versioned PascalCase enum, ~38 starter entries, ceiling ~60.** See "Event class catalog" section. Conditional entries flagged for cross-doc sync (signatures, scouting).
4. **Three-tier compaction + per-season quota cap.** See "Compaction strategy" section. `season-defining` → hard preserve / `notable` → compact preserve / `routine-and-below` → aggregated. Top-5% quota with `N_quota = 20` Phase-6 tuning seed. "Hard preserve" = full participant / tag / emotion / consequence / source context, but NOT raw tick telemetry (that lives in match replay data).
5. **Load-time forward migration.** See "Save / load migration" section. Not lazy-per-read at MVP — lazy spreads complexity, bugs become intermittent. Optimize to lazy only if Phase-6 synthetic saves prove load-time too slow.

## Prototype gate

**Phase 3 Week 4:** ledger operational in Month-3 slice. A small whitelist of career-relevant MatchSim actions emits MemoryEvents; alumni-DB reader surfaces 1 callback in post-match screen.

**Phase 5 gate:** 3 readers + salience thresholds operational; balance harness sweeps confirm "right 5-8 events surface per season" holds across 10K simulated seasons.

**Phase 6 gate:** compaction validated on 20-year synthetic career; save size acceptable; query perf under target.

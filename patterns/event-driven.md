# Pattern: Event-sourced career memory

The append-only ledger behind pillar 2 — "careers that remember." Decisions land as events; readers project the ledger into different surfaces (salience, press, fan, scout, coach).

## Why

Players, clubs, managers carry personal history across decades. A transfer rejection in 2026 surfaces 4 seasons later as "still loyal" salience boost during contract renewal commentary. This only works if the events are immutable + queryable in tick order.

## When to use

- Cross-match, cross-season narrative state (career memory)
- Anything where "what HAPPENED" matters more than "what IS the current value"
- Audit-trail-style data the UI surfaces years later

## When NOT to use

- Current canonical match state — that's `fw-match-sim`, not `fw-memory`
- UI display state — that's frontend signals/stores
- Settings / preferences — that's `fw-save`'s settings struct

## Pattern

```rust
pub enum MemoryEvent {
    PromisedYouthMinutes { player: PlayerId, tick: Tick, manager: ManagerId },
    TransferRequested { player: PlayerId, tick: Tick, club_from: ClubId, club_to: ClubId },
    BreakthroughMoment { player: PlayerId, tick: Tick, signature: SignatureId, match_context: MatchId },
    RivalryFormed { player_a: PlayerId, player_b: PlayerId, tick: Tick, trigger: RivalryTrigger },
    LegacyGoal { player: PlayerId, tick: Tick, opponent: ClubId, scoreline: (u8, u8) },
    // ... ~20-30 variants total
}

pub struct CareerLedger {
    events: Vec<MemoryEvent>,  // append-only, tick-ordered
}

impl CareerLedger {
    pub fn append(&mut self, event: MemoryEvent) {
        debug_assert!(self.events.last().map_or(true, |last| last.tick() <= event.tick()));
        self.events.push(event);
    }
}
```

Storage: `BTreeMap<CareerId, CareerLedger>` (BTreeMap because deterministic iteration matters for canonical state). On disk via Bincode 2.

## Five readers

| Reader | What it projects |
|---|---|
| Salience | Numeric weight per (career, current_context) → drives commentary trigger probabilities |
| Press | Headline templates filtered by salience → narrative-director's Tracery grammars |
| Fan | Per-club fan-faction sentiment shifts |
| Scout | Scout report flavor text (bias-influenced reading of the ledger) |
| Coach | Manager-AI feedback on player development |

Each reader is a pure function: `Reader::project(&CareerLedger, &QueryContext, current_tick) -> ReaderOutput`.

## Determinism considerations

- Events appended in tick order. Append-only `Vec<MemoryEvent>` per career.
- No `HashMap` — ledger keyed by `BTreeMap<CareerId, ...>`.
- Reader projections are pure (no clock, no RNG outside seeded ChaCha8Rng).

## Worked example

2026: Player A rejects transfer request from Club X → `MemoryEvent::TransferRejected { player: A, tick: T1, club: X }`.

2029: Player A enters contract renewal at Club Y (Y == A's current club, not X). The Salience reader projects: "TransferRejected event 3 seasons ago, same club → +5 loyalty salience." Press reader picks template: "{{player}} sees out his contract — a rare loyalty in modern football."

## Failure modes

- **Event mutation:** the `events: Vec<MemoryEvent>` MUST be append-only at the type level. Don't expose `&mut Vec`; only `append(&mut self, event)`.
- **Reader leakage:** readers MUST NOT call `events.last_mut()` or any mutating method.
- **Tick-order violation:** new events with tick < last event's tick → debug_assert. In release, sort defensively on serialize.

## Cross-references

- `crates/fw-memory/` (Phase T1+ target)
- FW v1 reference (design intent ONLY): `/Users/vibelogic/dev/football-archive/MatchSim/Memory/*` — 5-reader pattern
- `docs/DESIGN_DOC.md` pillar 2
- `narrative-director` agent — owns reader prose-templates

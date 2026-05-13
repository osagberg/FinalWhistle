# Playtest report — {{YYYY-MM-DD}} — seed {{seed}}

**Tester:** {{solo-dev / external}}
**Build:** {{commit SHA / version}}
**Session length:** {{duration}}
**Seed:** {{match_seed for reproducibility}}

---

## What was tested

{{1-2 sentences on the slice tested}}

## Setup

- Phase: {{T<N>}}
- Active mods: {{list}}
- Save loaded: {{path or "fresh start"}}

## Timeline of observations

Roughly chronological. Tick numbers + wall-clock times.

- **Tick {{N}} ({{HH:MM:SS}}):** {{what happened, what was observed}}
- **Tick {{N}} ({{HH:MM:SS}}):** {{...}}

## What worked

- {{...}}
- {{...}}

## What didn't work

- {{...}} (severity: P0 | P1 | P2 | P3)
- {{...}}

## Reactions

What did playing this feel like? Was the player experience goal hit? Be honest, not optimistic.

## Numerical signals (if any)

- {{stat / hash / metric}}
- Reference: did the canonical hash drift? If yes, was it expected?

## Action items

- {{task}} — proposed MASTER_PLAN entry T{{phase}}-{{n}}
- {{bug}} — needs `/log-decision` or just a `/next` fix
- {{question}} — needs design clarification

## Cross-references

- Related design doc: {{path}}
- Related ADR: {{link}}
- Commit shipped this slice: {{SHA}}

---
paths:
  - "UnityProject/Assets/_Project/Scripts/Memory/**"
  - "UnityProject/Assets/_Project/Memory/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Memory — event-sourced career consequences

Career memory is a structured ledger with readers. It is not a prose archive.

## MUST

- Append MemoryEvents at runtime; never mutate existing ledger records.
- Store only career-relevant events in the career ledger. Routine match telemetry stays in replay/debug data.
- Use deterministic runtime event IDs derived from career or match sequence.
- Keep reader systems separate from storage; alumni, rival recall, promise tracking, big-match scars, and press/fan callbacks read the same ledger.
- Version every persisted schema and provide migrations from day one.

## SHOULD

- Preserve high-salience old events intact and compact lower-salience history into aggregates.
- Add tests for reader queries and migration behavior whenever schemas change.
- Keep salience formulas explicit and range-checked.
- Emit enough debug context to explain why an event surfaced or was suppressed.

## AVOID

- Free-form runtime LLM prose in saves or UI.
- One-off callback stores per feature.
- Writing every MatchSim event into career saves.
- Save formats that cannot be migrated by version.

## References

- [design/event-sourced-memory.md](../../../../../design/event-sourced-memory.md)
- [TECH_APPROACH.md](../../../../../TECH_APPROACH.md) §5 Event-sourced career memory

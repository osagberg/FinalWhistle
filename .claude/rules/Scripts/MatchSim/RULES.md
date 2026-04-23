---
paths:
  - "MatchSim/**"
  - "MatchSim.csproj"
  - "MatchSim.Tests.csproj"
  - "UnityProject/Assets/_Project/Scripts/MatchSim/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# MatchSim — deterministic canonical simulation

MatchSim is the source of truth for match outcomes. Unity renders it; Unity never drives it.

## MUST

- Keep `MatchSim.csproj` pure C# with zero `UnityEngine` references.
- Use Q32.32 fixed-point for canonical positions, velocities, forces, timers, and derived trajectory values.
- Use deterministic RNG streams derived from `match_seed`, tick, and event sequence; never use wall-clock time or platform RNG.
- Emit structured `MatchEvent` records for viewer, memory, and tests; do not let presentation code infer canonical outcomes.
- Run xUnit determinism tests after sim changes; canonical state hash must match across repeated runs.

## SHOULD

- Keep hot-path math allocation-free and LINQ-free.
- Put tactical constants in data records or YAML archetypes, not literals inside behavior code.
- Separate match telemetry from career memory; routine passes and position samples are replay/debug data, not ledger events.
- Hash canonical state after known tick counts in golden tests.

## AVOID

- Floats or doubles in canonical state.
- Unity PhysX, `Time.deltaTime`, coroutines, or scene objects in the canonical sim path.
- Hidden global state or singletons that affect replay.
- Random event IDs; runtime IDs must be deterministic from career/match sequence.

## References

- [TECH_APPROACH.md](../../../../../TECH_APPROACH.md) §3 MatchSim architecture
- [design/match-engine.md](../../../../../design/match-engine.md)
- [design/month-3-vertical-slice.md](../../../../../design/month-3-vertical-slice.md)

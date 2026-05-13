# Systems index

A table-of-contents for all in-game systems. One row per system. Status = Stubbed | InDev | Shipped.

---

| System | Owning crate | Status | Pillar | Key doc |
|---|---|---|---|---|
| MatchSim | fw-match-sim | Stubbed | 5 (signatures) | `docs/design/match-engine.md` |
| MemoryLedger | fw-memory | Stubbed | 2 (memory) | `docs/design/memory.md` |
| Scouting | fw-scouting | Stubbed | 4 (uncertainty) | `docs/design/scouting.md` |
| Breakthroughs | fw-match-sim + fw-memory | Stubbed | 3 (breakthroughs) | `docs/design/breakthrough-moments.md` |
| Signatures | fw-match-sim | Stubbed | 5 | `docs/design/signatures.md` |
| ContentBaker | fw-content-baker | Stubbed | 1 (procedural world) | `docs/CONTENT_PIPELINE.md` |
| SaveMigration | fw-save | Empty | — | `docs/specs/save-migration-fixtures.md` |
| TacticalBoard | frontend + fw-tauri | Empty | — (presentation) | TBD |
| Commentary | fw-tauri + frontend | Empty | 1, 2, 5 | `docs/design/ui-vocabulary.md` |
| ReplayCanonicalHash | fw-replay | Stubbed | — (regression floor) | `docs/specs/determinism-gate.md` |

---

## How to add a row

When introducing a new system:

1. Add a row in alphabetical order within its status block (Stubbed first, then InDev, then Shipped).
2. Owning crate = the primary crate. If cross-cutting, list the lead crate + cross-references in the key doc.
3. Status:
   - **Stubbed** = compiles, has the types, no real behavior
   - **InDev** = behavior implemented, tests partial, not feature-complete
   - **Shipped** = feature-complete, tests green on CI matrix, in the latest CHANGELOG entry
4. Pillar = primary pillar served. List multiple if cross-cutting, primary first.
5. Key doc = canonical design doc path.

## Cross-references

- `docs/DESIGN_DOC.md` §3 — the 5 pillars
- `docs/MASTER_PLAN.md` — phase-by-phase task list
- `.claude/agents/README.md` — which agent owns which task class

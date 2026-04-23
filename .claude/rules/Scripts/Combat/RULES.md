---
paths:
  - "Assets/_Project/Scripts/Combat/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Combat — combat rules + resolution

Turn-based vs real-time is a design decision recorded on `CombatConfigSO.Mode`. Code supports both.

## MUST

- `CombatConfigSO.Mode` flag (`TurnBased | RealTime`) gates timing paths. No duplicated systems.
- All moves defined as `CombatMoveSO`. Damage formulas read numbers from `StatDefSO` references.
- Zero balance numbers in `.cs` files. `damage = caster.ATK * move.Power - target.DEF * move.Mitigation`.
- Delta time on every real-time update (`Time.deltaTime`, `Time.fixedDeltaTime`). Never frame-count.
- Depends on `Core` + `Stats` + `Characters` + `Outfits`. No UI, no Dialog (combat emits events; UI listens).

## SHOULD

- Combat resolution is a pure function given `(attacker, defender, move, rng)` — testable without scene.
- Critical hits, status application, and resistance all consume SO-defined curves/tables.
- Use a command queue for turn-based flow — replayable, debuggable.
- Log every resolution to a ring buffer for the Debug asmdef to dump.

## AVOID

- `if (damage > 50) { ... }` — thresholds are SO fields, not literals.
- Coroutines for multi-turn sequences — async/await with UniTask.
- Combat side effects (animation triggers, VFX, SFX) inline in resolution — emit events.
- Referencing specific character `Id`s inside combat — dispatch by `CharacterSO` type/tag.

## RATIONALE

Combat is where balance changes land most often. Numbers in SOs means designer iteration without recompile. The Mode flag prevents the common trap of forking the codebase when real-time gets added to a turn-based system late.

## References

- [Stats/RULES.md](../Stats/RULES.md)
- [Characters/RULES.md](../Characters/RULES.md)
- [ScriptableObjects/RULES.md](../../ScriptableObjects/RULES.md)

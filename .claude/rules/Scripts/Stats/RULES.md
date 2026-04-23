---
paths:
  - "Assets/_Project/Scripts/Stats/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Stats — resources, attributes, progression

All tunable values live in ScriptableObjects. Code describes behavior; SOs describe numbers.

## MUST

- Zero hardcoded balance numbers in `.cs` files. HP, damage, regen, thresholds — all SO fields.
- Every stat type backed by a `StatDefSO` (name, clamp range, default, display).
- Runtime stat instances are `struct` — no per-entity GC churn.
- Depends only on `Core`. No references to `Characters`, `Combat`, or UI.
- MonoBehaviour usage confined to explicit bridge types (`StatComponent : MonoBehaviour`) — pure stat math stays plain C#.

## SHOULD

- Prefer events/signals for "stat changed" notifications over polling.
- Clamp at the boundary (setter), not at the reader.
- Serialize stats as `int` or `float`, not `double` — Unity inspector friction.
- Group related stats into a `StatSheetSO` when >4 travel together.

## AVOID

- `StatsManager.Instance.HP` — singletons couple the whole game to one stats table.
- Magic numbers in formulas. `damage * 1.5f` → `damage * critMultiplierSO.Value`.
- Deriving display strings inside stat logic — UI formats, Stats compute.
- `[SerializeField] float maxHP = 100f;` on a MonoBehaviour — put it on the SO.

## RATIONALE

Balance changes should be asset-only (no recompile, no code review). Keeping numbers in SOs lets the designer-you tune without the engineer-you rebuilding. Struct instances keep thousands of stat-bearing agents cheap.

## References

- [TECH_APPROACH.md](../../../../../TECH_APPROACH.md) §4 Data architecture
- [ScriptableObjects/RULES.md](../../ScriptableObjects/RULES.md)

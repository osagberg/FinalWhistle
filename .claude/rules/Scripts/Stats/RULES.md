---
paths:
  - "Assets/_Project/Scripts/Stats/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Stats — football attributes, form, and progression

All tunable values live in ScriptableObjects. Code describes behavior; SOs describe numbers.

## MUST

- Zero hardcoded balance numbers in `.cs` files. Attribute ranges, development rates, form curves, confidence effects, and thresholds are data fields.
- Every stat type backed by a `StatDefSO` (name, clamp range, default, display).
- Runtime stat instances are `struct` — no per-entity GC churn.
- Depends only on Core-style primitives. No UI references.
- MonoBehaviour usage confined to explicit bridge types (`StatComponent : MonoBehaviour`) — pure stat math stays plain C#.

## SHOULD

- Prefer events/signals for "stat changed" notifications over polling.
- Clamp at the boundary (setter), not at the reader.
- Serialize stats as `int` or `float`, not `double` — Unity inspector friction.
- Group related stats into a `StatSheetSO` when >4 travel together.

## AVOID

- `StatsManager.Instance` — singletons couple the whole game to one stats table.
- Magic numbers in formulas. `pace * 1.1f` → `pace * transitionBoost.Value`.
- Deriving display strings inside stat logic — UI formats, Stats compute.
- `[SerializeField] float maxPace = 100f;` on a MonoBehaviour — put it on data.

## RATIONALE

Balance changes should be data-only where possible. Keeping numbers in content data lets the designer-you tune without the engineer-you rebuilding, while struct instances keep thousands of player projections cheap.

## References

- [TECH_APPROACH.md](../../../../../TECH_APPROACH.md) §4 Data architecture
- [ScriptableObjects/RULES.md](../../ScriptableObjects/RULES.md)

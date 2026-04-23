---
paths:
  - "Assets/_Project/Scripts/Characters/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Characters — NPC + PC runtime

Character spawn + identity + state. Data flows from `CharacterSO`; runtime behavior is code.

## MUST

- All spawns via Addressables. `Addressables.InstantiateAsync(characterSO.PrefabRef)` — never `Resources.Load`, never direct prefab refs in scenes.
- Prefab references on SOs are `AssetReferenceGameObject`, not raw `GameObject` fields.
- Age gate: `CharacterSO` has `int Age` with `OnValidate` guard `Age >= 18`. Editor rejects below floor.
- Depends on `Core` + `Stats` only. No UI, no Dialog, no Combat references.
- Character identity keyed by stable string `Id` (e.g., `"npc_wren"`) — never by scene object name.

## SHOULD

- Pool spawned characters where the count exceeds ~10 live instances.
- Use `IDumpable` for state-dump participation (Debug asmdef reads via interface only).
- Keep runtime `Character.cs` under 300 lines — factor to sub-components.
- Load heavy data (portraits, voice banks) lazily via `AssetReference`, not on spawn.

## AVOID

- `[SerializeField] GameObject prefab;` — use `AssetReferenceGameObject`.
- Character state stored on the GameObject's name or tag.
- Scene-baked references to specific character instances — resolve by `Id` at runtime.
- Circular lookups between `Character` and `CharacterSO` — SO is read-only at runtime.

## RATIONALE

Addressables is the one build-safe loading path (Resources bloats the Player build; direct refs defeat streaming). Age gate at data layer is a content-safety invariant — enforcing it in code would be bypassable, in the SO it fails at author time.

## References

- [TECH_APPROACH.md](../../../../../TECH_APPROACH.md) §6 Addressables
- [Addressables/RULES.md](../../Addressables/RULES.md)
- [unity/addressables-groups-pattern.md](../../../../../unity/addressables-groups-pattern.md)

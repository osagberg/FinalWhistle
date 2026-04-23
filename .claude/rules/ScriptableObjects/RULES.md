---
paths:
  - "Assets/_Project/ScriptableObjects/**"
  - "Assets/_Project/Scripts/**/*SO.cs"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# ScriptableObjects — content data layer

Content = SOs. Code executes SOs; code does not embed SO values.

## MUST

- Every SO class carries `[CreateAssetMenu(menuName = "{{PROJECT_NAME}}/<Category>/<TypeName>", fileName = "<Category>_<Name>")]`.
- Asset naming: `<Category>_<Name>.asset` (e.g., `Character_Wren.asset`, `Outfit_Overalls_Default.asset`).
- One SO field = one purpose. No `float multiUseValue` that means different things in different contexts.
- No serialized references to scene GameObjects. SOs are asset-scoped; scene refs don't survive reload.
- Addressable prefab references as `AssetReferenceGameObject`, not `GameObject`.

## SHOULD

- Group asset files under `ScriptableObjects/<Category>/` mirroring the `Scripts/<System>/` folder name.
- Use `ReadOnly` attribute on fields computed at author time (so the inspector doesn't tempt hand-editing).
- Provide an `OnValidate()` that enforces invariants (age ≥ 18, required fields non-null, ranges clamped).
- Document the schema version as a constant (`const int SchemaVersion = 2;`) when migrating.

## AVOID

- Singleton SOs with global access. Inject via a bootstrap `RegistrySO` instead.
- `[System.Serializable] class Nested { }` fields that grow unbounded — factor into their own SO.
- SOs referencing other SOs by string name — use typed refs (`CharacterSO` field, not `string characterId`).
- Per-save-slot data on a shared SO. Save data lives in runtime state, not on the asset.

## RATIONALE

SOs are the editable layer. If a value is on an SO, it survives code refactors and is designer-editable. If it's in code, it needs a recompile to change. Naming conventions matter because Addressables + asset search both key off file names.

## References

- [unity/scripts-folder-structure.md](../../../unity/scripts-folder-structure.md) §`ScriptableObjects/` mirror
- [Scripts/Stats/RULES.md](../Scripts/Stats/RULES.md)
- [Scripts/Characters/RULES.md](../Scripts/Characters/RULES.md)

---
paths:
  - "Assets/_Project/Scripts/Outfits/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Outfits — gear, cosmetics, variant state

Layered outfit system. Morph/blend states live in SOs; the runtime composes components.

## MUST

- Outfit composition via components (one `OutfitLayer` per slot: base, top, bottom, accessory).
- Morph / blend-shape states declared on `OutfitSO`. Code reads them; never computes target values inline.
- Physical prefabs referenced via `AssetReferenceGameObject` on the SO.
- Depends on `Core` + `Characters` only. No UI, no Dialog.
- Outfit swap must survive save/load — serialize the `OutfitSO.Id`, not the runtime instance.

## SHOULD

- Cache the current outfit material set; avoid per-frame `GetComponent`.
- Use `MaterialPropertyBlock` for per-instance variations, not material duplication.
- Keep cloth-sim tuning in a dedicated `ClothProfileSO` referenced by the outfit, not baked into the prefab.
- Write an Editor validator asserting every `OutfitSO` has a matching prefab address resolvable.

## AVOID

- Swapping meshes by disabling GameObjects in a single giant prefab — fragments GPU batches and scales badly.
- String-based slot lookups (`"TopSlot"`) — use an `OutfitSlot` enum in Core.
- Per-outfit MonoBehaviour subclasses — prefer one `OutfitController` driven by SO data.
- Coupling outfit state to animation controllers — animation reads outfit, not vice versa.

## RATIONALE

Outfits multiply: characters × variants × morphs. A data-driven system scales linearly with content; a code-driven one scales linearly with engineer-hours. Layering via components keeps swap cheap and save-load trivial (one string Id per slot).

## References

- [Characters/RULES.md](../Characters/RULES.md)
- [ScriptableObjects/RULES.md](../../ScriptableObjects/RULES.md)

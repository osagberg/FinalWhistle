---
paths:
  - "Assets/_Project/Scripts/**/*.cs"
  - "Assets/_Project/Editor/**"
  - "Assets/AddressableAssetsData/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Addressables — asset streaming

All runtime-loaded content routes through Addressables. No exceptions in Player builds.

## MUST

- Zero `Resources.Load` / `Resources.LoadAsync` calls in runtime code. The `Resources/` folder is reserved for Unity-required edge cases (none, in practice, for this project).
- Seven canonical groups: `Characters`, `Outfits`, `Scenes`, `Audio`, `CGs`, `UI`, `Ink`. Create others only with a decision log entry.
- Addresses: `lowercase-kebab-case` (`character-player`, `scene-library-main`).
- Labels: `lowercase_underscore` (`character_npc`, `outfit_uniform_b`).
- Every `AssetReference*` field has a null-check at load site before use.

## SHOULD

- Release handles in `OnDisable` / `OnDestroy`. Never leak Addressables handles — memory bleeds fast.
- Preload groups for scene-scoped content at scene load; stream incidentally-needed assets lazily.
- Use `Addressables.InstantiateAsync` over manual `LoadAssetAsync` + `Instantiate` — lifecycle cleanup is built in.
- Profile group sizes with `Analyze` window; split groups exceeding 100 MB.

## AVOID

- Hardcoded address strings scattered across code. Centralize in `AddressableKeys` static class or per-system `KeysSO`.
- Mixing editor-time direct refs and runtime Addressables refs on the same SO — pick one per field.
- `Addressables.LoadAssetAsync<T>(key).WaitForCompletion()` in gameplay code — blocks; use async/await.
- Content updating groups outside the 7 canonical without explicit decision. Group sprawl makes builds fragile.

## RATIONALE

Resources folder bloats the Player build (all content loaded eagerly at startup). Addressables streams, supports content updates, and respects platform size limits. The 7-group canon exists because group count correlates with build-pipeline fragility; we pick the split that maps to content types and stop there.

## References

- [unity/addressables-groups-pattern.md](../../../unity/addressables-groups-pattern.md)
- [Scripts/Characters/RULES.md](../Scripts/Characters/RULES.md)

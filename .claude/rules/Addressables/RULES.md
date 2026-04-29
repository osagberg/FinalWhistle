---
paths:
  - "unity-project/Assets/Scripts/**/*.cs"
  - "unity-project/Assets/Editor/**"
  - "Assets/AddressableAssetsData/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Addressables — asset streaming

All runtime-loaded content routes through Addressables. No exceptions in Player builds.

## MUST

- Zero `Resources.Load` / `Resources.LoadAsync` calls in runtime code. The `Resources/` folder is reserved for Unity-required edge cases (none, in practice, for this project).
- Canonical groups: `Content/Clubs`, `Content/Players`, `Content/Signatures`, `UI/Screens`, `Viewer/SemanticCinema`, `Audio/Crowd`, `Audio/Music`, `Fonts`. Create others only with a decision log entry.
- Addresses: `lowercase-kebab-case` (`club-hartfield-town`, `ui-squad-screen`).
- Labels: `lowercase_underscore` (`club_content`, `player_portrait`, `semantic_cinema`).
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
- Content updating groups outside the canonical list without explicit decision. Group sprawl makes builds fragile.

## RATIONALE

Resources folder bloats the Player build. Addressables streams, supports content packs, and keeps UI/viewer/audio assets explicit. The group canon exists because group count correlates with build-pipeline fragility; we pick the split that maps to Final Whistle content types and stop there.

## References

- [TECH_APPROACH.md](../../../TECH_APPROACH.md) §7 Addressables group ontology
- [Scripts/Pipeline/RULES.md](../Scripts/Pipeline/RULES.md)

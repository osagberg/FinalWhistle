---
paths:
  - "Assets/_Project/Scripts/Debug/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Debug — state dump, playtest, cheats

Dev-only tooling. Must compile out of Release builds cleanly.

## MUST

- Every file guarded: `#if UNITY_EDITOR || DEVELOPMENT_BUILD` around the type declaration, OR the asmdef lists `defineConstraints: ["UNITY_EDITOR"]` / `["DEVELOPMENT_BUILD"]`.
- No runtime references from `Core`, `Stats`, `Players`, `Memory`, `MatchSim`, `Viewer`, `UI`, or `AI` back into `Debug`.
- Subscribers consume the `IDumpable` interface defined in Core; Debug never depends on concrete types outside its asmdef.
- Cheat commands are opt-in via a dev console, never hotkey-triggered in builds.
- Debug output routes through a single `DevLog` facade — easy to silence in one place.

## SHOULD

- State dumps produce deterministic, diffable text (sorted keys, stable newlines, no timestamps inline).
- Cheat flags live on a `DebugConfigSO` under `Assets/_Project/ScriptableObjects/Debug/` — gitignored or a separate asset.
- Replay / rewind tooling uses the event bus, not direct state mutation.
- Playtest capture writes to a gitignored `Captures/` folder, never into `_Project/`.

## AVOID

- Calling Debug APIs from non-Debug asmdefs. That's what interfaces in Core are for.
- `Debug.Log` strings that contain player-visible text templates — prefer structured fields.
- `Debug.Break()` in shipped code paths — makes the Player build hang on dev assertions.
- Leaving `#if UNITY_EDITOR` off a Debug type — it'll drag `UnityEditor` references into the Player build and fail the final build.

## RATIONALE

The Debug asmdef is where the build most often breaks before ship. Strict compile-time gating (define constraints + conditional compilation) means Player builds can't accidentally include editor code. The `IDumpable` pattern keeps the dependency arrow one-way: systems expose; Debug consumes.

## References

- [Assemblies/RULES.md](../../Assemblies/RULES.md)
- [Core/RULES.md](../Core/RULES.md) `IDumpable`

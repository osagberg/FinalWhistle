---
paths:
  - "Assets/_Project/Scripts/Editor/**"
  - "Assets/_Project/Editor/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Editor — editor-only tooling

Asset validators, MenuItem utilities, property drawers, Addressables bootstrap. Never shipped.

## MUST

- Asmdef has `includePlatforms: ["Editor"]` and `defineConstraints: []`. Nothing else.
- No runtime asmdefs reference the Editor asmdef. (Runtime → Editor is a circular dep + build break.)
- Editor-only code is free to `using UnityEditor;` — outside this folder, never.
- `MenuItem` paths follow `_Project/<Category>/<Action>` convention. No root-level menu items.
- Validators return typed `ValidationResult` objects — the `/audit` slash command consumes them.

## SHOULD

- One file per `MenuItem`. Long-running validators are their own class.
- Editor state that persists between domain reloads uses `SessionState` or `EditorPrefs`, not static fields.
- Wrap destructive operations (`AssetDatabase.DeleteAsset`) behind a confirmation dialog.
- Factor Addressables bootstrap into idempotent functions — re-runnable, no duplicate groups.

## AVOID

- `using UnityEditor` in any file outside `Editor/` — build breaks immediately on Player target.
- `EditorApplication.update` for anything non-trivial — it runs constantly, adds editor lag.
- `AssetDatabase.Refresh()` in hot paths — it's a full project rescan.
- MenuItem shortcuts (`_`, `%`, `#`, `&`) conflicting with Unity built-ins without checking `Edit > Shortcuts`.

## RATIONALE

The Editor / Runtime split is enforced by asmdef platform filters; getting it wrong breaks the Player build in a way that's mysterious (works in editor, fails at build). Keeping validators typed enables the `/audit` pipeline to aggregate results without string-parsing console output.

## References

- [Assemblies/RULES.md](../../Assemblies/RULES.md) asmdef rules
- [commands/audit.md](../../../commands/audit.md)

---
paths:
  - "Assets/_Project/**/*.asmdef"
  - "Assets/_Project/**/*.asmref"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Assemblies — asmdef layout

One asmdef per system folder. Name-based refs. Layered dependency graph, enforced at compile time.

## MUST

- One `.asmdef` per top-level `Scripts/<System>/` folder. No nested asmdefs unless an Editor sub-folder.
- `references:` uses assembly names (`"{{PROJECT_NAME}}.Core"`), NOT GUIDs. GUIDs are pain to review in diffs.
- `rootNamespace` set to the dotted path (`{{PROJECT_NAME}}.Core`, `{{PROJECT_NAME}}.Stats`, etc.).
- `autoReferenced: false` — explicit over implicit. Every dep declared.
- `precompiledReferences:` only for DLLs actually used. Empty array is correct when unused.

## SHOULD

- Dependency direction follows the stack: `Core ← Stats ← Characters ← Outfits ← CoreMechanic ← Combat ← Dialog, UI, AI, Debug`.
- Editor asmdefs under `<System>/Editor/` with `includePlatforms: ["Editor"]`.
- Test asmdefs under `<System>/Tests/` with `"optionalUnityReferences": ["TestAssemblies"]`.
- Keep `defineConstraints` minimal — prefer `#if` inside code over build-excluded assemblies.

## AVOID

- Circular references. If A needs B and B needs A, one of them owns an interface the other consumes.
- `"Unity.*"` references without version pinning in `Packages/manifest.json`.
- Blanket `allowUnsafeCode: true` — enable only on the specific asmdef that needs it.
- Hand-editing GUIDs in asmdef files. Let Unity generate them; reference by name.

## RATIONALE

Name-based refs survive GUID regeneration (happens on Library rebuild, re-imports); GUID refs don't. The layered graph isn't bureaucracy — it's what lets you swap out `Combat.dll` without rebuilding `Stats.dll`. Circular refs collapse the benefit entirely.

## References

- [unity/assembly-definitions-skeleton.md](../../../unity/assembly-definitions-skeleton.md)
- [unity/scripts-folder-structure.md](../../../unity/scripts-folder-structure.md)

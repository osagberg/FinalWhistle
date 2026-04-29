---
paths:
  - "unity-project/Assets/**/*.asmdef"
  - "unity-project/Assets/**/*.asmref"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Assemblies — asmdef layout

One asmdef per system folder. Name-based refs. Layered dependency graph, enforced at compile time.

## MUST

- One `.asmdef` per top-level `Scripts/<System>/` folder. No nested asmdefs unless an Editor sub-folder.
- `references:` uses assembly names (`"FinalWhistle.Core"`), NOT GUIDs. GUIDs are pain to review in diffs.
- `rootNamespace` set to the dotted path (`FinalWhistle.Core`, `FinalWhistle.Memory`, etc.).
- `autoReferenced: false` — explicit over implicit. Every dep declared.
- `precompiledReferences:` only for DLLs actually used. Empty array is correct when unused.

## SHOULD

- Dependency direction follows the stack: `Core ← Stats/Players/Memory ← MatchSim/AI/Signatures ← Viewer/Management/UI ← Debug/Editor`.
- Editor asmdefs under `<System>/Editor/` with `includePlatforms: ["Editor"]`.
- Test asmdefs under `<System>/Tests/` with `"optionalUnityReferences": ["TestAssemblies"]`.
- Keep `defineConstraints` minimal — prefer `#if` inside code over build-excluded assemblies.

## AVOID

- Circular references. If A needs B and B needs A, one of them owns an interface the other consumes.
- `"Unity.*"` references without version pinning in `Packages/manifest.json`.
- Blanket `allowUnsafeCode: true` — enable only on the specific asmdef that needs it.
- Hand-editing GUIDs in asmdef files. Let Unity generate them; reference by name.

## RATIONALE

Name-based refs survive GUID regeneration (happens on Library rebuilds and re-imports); GUID refs don't. The layered graph keeps MatchSim headless and testable while Unity-facing presentation remains replaceable. Circular refs collapse the benefit entirely.

## References

- [TECH_APPROACH.md](../../../TECH_APPROACH.md) §7 Assembly Definitions skeleton

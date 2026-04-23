---
description: Run all project validators — SO schemas, assembly defs, asset licensing, docs alignment
---

# /audit — validate project integrity

Runs every validator and reports green/red status.

## Procedure (adapt to current phase — some checks don't apply until later phases)

### Content-layer validators (from Phase 2 onward)

1. **Design-doc intent coverage**: every `ScriptableObject` type declared in `design/` has a matching SO asset authored by Phase 6.
2. **Scene metadata**: every scene file has a companion `meta.yaml` or equivalent (declaring tags, characters, CG refs).
3. **Content-tag vocabulary**: scene tags drawn from the declared vocabulary (if project has a tag list).

### Outfit / content-type validators (from Phase 4 onward — project-specific)

4. **Outfit / gear / item slots**: every outfit/item SO has a valid slot enum value.
5. **Morph / state integrity**: every SO with variant-states lists all declared states.

### Skeleton / rig standard (from Phase 4 onward)

6. **Skeleton standard enforcement**: every Character SO has the declared skeleton type (`vrm` / `mixamo` / `humanik` — per TECH_APPROACH).
7. **Rig bone parity**: every rigged character has the base bone set for the declared standard.

### Engine / structural (from Phase 3 onward)

8. **Assembly definitions**: every `Scripts/<System>/` folder has an `.asmdef`.
9. **Addressables policy**: no direct `Resources.Load` calls in runtime code (Addressables is the only loading path).
10. **SO data-driven**: no hardcoded stat deltas / balance numbers in executor code — must read from SO.
11. **Unity MCP handshake**: `claude mcp list` shows unity-mcp green.
12. **No prohibited DLL refs**: precompiled references in asmdefs match the approved set.

### Steam-release validators (from Phase 7 onward)

13. **Asset licensing tracker**: every 3rd-party asset in `Packages/` or `Assets/Plugins/` has a row in `asset-licensing-tracker.csv` with license terms + paid/free + attribution-required flag.
14. **Age rating consistency**: content-tag inventory matches the target rating declared in PROJECT_CONTEXT.md.
15. **Localization coverage**: if localization is live, every user-facing string in `StringTable` has entries in declared languages (or is flagged `[en-only]`).
16. **Accessibility checklist**: items in `design/accessibility.md` that are marked done have code verification.

### Workflow-state checks (always)

17. **Docs alignment**: SPEC.md active phase matches STATUS.md. CHANGELOG has entries for recent SPEC `[x]` marks.
18. **No archived references**: no references to retired iterations in active docs (per `/refresh-docs` check A).
19. **Git hygiene**: no uncommitted changes in `main`; feature branches exist only for active work.

## Output

One report with columns: `Check | Status (✅ / ❌ / ⚪ N/A-for-phase) | Detail if failed`.

If any high-severity failures: list proposed fixes but do NOT auto-apply. User reviews.

## Phase-appropriate scoping

Early phases: skip checks that reference artifacts not yet created. Report `⚪ N/A` for those.

## Project-specific extensions

Add project-specific checks at the bottom of this file as they become relevant (e.g., `age >= 18` for adult content, `min-fps >= 60` for competitive games, etc.).

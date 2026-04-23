---
paths:
  - "UnityProject/Assets/_Project/Scripts/Pipeline/**"
  - "content-packs/**"
  - "tools/content-compiler/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Pipeline — content packs, compiler, validation

Pipeline code turns authored specs and generated JSON into versioned game content. It is not runtime AI.

## MUST

- Use stable IDs qualified by content pack and schema version.
- Validate JSON/schema before import; malformed generated content fails the build.
- Lint names for duplicates, legal risk, banned UI vocabulary, age-rating issues, and regional-style drift.
- Keep prompt, seed, model version, schema version, and generated output provenance with every compiled pack.
- Add migrations whenever persisted schemas change.

## SHOULD

- Prefer deterministic seeded transforms after LLM draft generation.
- Generate deltas as new content-pack versions; never rewrite shipped IDs in place.
- Run sim-sanity checks on generated players/clubs before Unity import.
- Keep compiler outputs reviewable in text during early phases; optimize storage only when size becomes painful.

## AVOID

- Runtime LLM calls in Player builds.
- Unvalidated prose blobs that bypass content lint.
- Importers that silently coerce invalid values.
- Coupling compiler code to Unity Editor APIs unless the file is explicitly editor-only.

## References

- [TECH_APPROACH.md](../../../../../TECH_APPROACH.md) §4 Content pipeline architecture
- [design/player-generation.md](../../../../../design/player-generation.md)
- [design/worldbuilding.md](../../../../../design/worldbuilding.md)

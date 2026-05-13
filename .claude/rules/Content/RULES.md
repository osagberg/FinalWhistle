---
description: RON content authoring conventions, content-pack-qualified IDs, banned-terms enforcement, mod overlay layout.
applies_to:
  - content/**
  - crates/fw-content/**
  - crates/fw-content-baker/**
auto_load: when_editing_matching_path
---

# Content authoring rules

## §1. RON file structure

- File extension: `.ron`.
- Top-level structure: a single named `( ... )` with named fields. Example:
  ```ron
  Culture(
      id: "fwh.core:culture_00001",
      schema_version: 1,
      name: "Anglo-Northern",
      first_names: [ "Elliot", "Marcus", ... ],
      surnames: [ "Ashby", "Thornton", ... ],
      ...
  )
  ```
- Indentation: 4 spaces. No tabs.
- Trailing commas on multi-line lists / structs (RON allows them; consistency matters).

## §2. Content-pack-qualified IDs

- Format: `<pack-id>:<entity-type>_<5-digit>`.
- Pack-id is lowercase + dots: `fwh.core`, `fwh.fantasy.elvish`, `mod.community.somerset`.
- Entity-type is singular lowercase: `player`, `club`, `culture`, `archetype`, `competition`.
- 5-digit zero-padded: `_00042`, `_01337`, never `_42` or `_1337`.
- Examples:
  - `fwh.core:player_00042`
  - `fwh.fantasy.elvish:culture_00003`
  - `mod.community.somerset:club_00001`

## §3. Schema versioning

- Every RON file has `schema_version: <N>` as a top-level field.
- **Forward migration only.** Bumping the schema adds a migration path; the v1 fixtures are NEVER mutated.
- Migrations live in `crates/fw-content/src/migrations/<N>_to_<N+1>.rs`.

## §4. Tracery grammars

- File extension: `.tracery.json` (NOT `.ron`).
- Use only documented Tracery operations: substitution, modifiers (`.capitalize`, `.s`, `.a`), savesymbol.
- **NO** eval / arbitrary code / fetch.
- ≥3 variants per template slot to avoid prose loops (the player should not see the same phrasing twice in a session).

## §5. Banned terms

- Catalog in `docs/design/ui-vocabulary.md`. The lint script is `scripts/lint-banned-terms.py`.
- Football-native vocabulary only.
- **NO** capitalized mystical state-nouns ("The Hush", "Awakened", "Resonance Cascade").
- **NO** "+5 Finishing" or any visible-stats tooltips.
- Sentinel exemption (use sparingly, only for meta-references):
  ```ron
  // ui-lint:allow term="Hush" reason="quoting fictional player's autobiography title" reviewer="narrative-director"
  ```

## §6. Mod overlay layout

- Mods live in `content/mods/<mod-id>/` mirroring `content/sources/` structure.
- Load order: lexicographic (sorted by mod-id ASCII order).
- A mod can:
  - **Add** new entities (new pack-id namespace).
  - **Override** an existing entity by ID (with explicit `overrides: "fwh.core:player_00042"` field).
- A mod **cannot**:
  - Delete entities.
  - Modify schema_version of existing entities.
  - Introduce new schema fields without a corresponding `fw-content` migration.
- `mod_load_fingerprint` (BLAKE3 hash of the sorted mod-id + version list) is stamped into save files. Loading a save with different mods active shows a warning.

## §7. Bake-time pipeline

- `crates/fw-content-baker/` is a CLI that calls Claude API at bake-time, validates output against `crates/fw-content/` schema, and emits validated RON to `content/baked/`.
- `content/baked/` is gitignored — regenerate via `just bake-content` (needs `ANTHROPIC_API_KEY`).
- **NO** runtime LLM calls. All LLM output is committed RON.

## §8. FW-VAL checks

- Validation contract in `docs/specs/content-pack-validation-contract.md`.
- Run: `cargo run --bin fw-content-baker -- validate`.
- Checks: schema conformance, ID uniqueness, length bounds, banned terms, cross-references resolve.

## §9. Authorship voice

- `narrative-director` agent owns tone for player-facing copy. When in doubt, invoke them.
- Read every Tracery template aloud once before committing.
- No emojis in content unless explicitly required.

## Cross-references

- `CLAUDE.md` §1 (pillar 1: procedural fantasy world), §7 (banned-terms lint)
- `docs/CONTENT_PIPELINE.md` — bake-time + runtime spec
- `docs/design/ui-vocabulary.md` — banned-terms source of truth
- `docs/specs/content-pack-validation-contract.md` — FW-VAL contract

# Pattern: Bake-time LLM content pipeline

Pillar 1 (procedural fantasy world) is delivered via offline LLM corpus generation. `fw-content-baker` CLI calls Claude API → validates → emits RON to `content/baked/`. **Zero runtime LLM calls.**

## Why

Every save is a different world. That requires hundreds of cultures, thousands of names, archetypes, headline templates, manager quotes, club histories. Hand-authoring at this scale is infeasible. Runtime LLM calls would add latency, network dependency, cost, and non-determinism. So: bake the corpus offline; ship the validated bytes.

## When to use

- Procedural worldbuilding (names, cultures, place-names)
- Phrase banks (Tracery grammars seeded with LLM-authored variants)
- Archetype catalogues with flavor copy
- Headline / press / commentary template variants

## When NOT to use

- Anything per-match (commentary at match time uses pre-baked templates + ChaCha8Rng sampling)
- Anything per-tick (sim is sync + deterministic)
- Anything responsive to player input live (those are pre-baked phrase banks + Tracery)

## Pipeline

```
content/sources/<category>/*.ron       # hand-authored seeds (cultures, archetype scaffolds, prompts)
content/sources/grammars/*.tracery.json # Tracery grammar scaffolds
              ↓
   fw-content-baker CLI:
     1. Read prompt templates in crates/fw-content-baker/src/prompts/*.md
     2. Substitute seed values from sources/
     3. Call Claude API (rate-limited, cache-keyed by prompt-hash)
     4. Validate output against crates/fw-content schema
     5. Apply banned-terms lint
     6. Cross-reference resolve (IDs that point to other entities exist)
     7. Emit RON to content/baked/
              ↓
content/baked/                          # gitignored; regenerated via `just bake-content`
              ↓
   Runtime sampling:
     - ChaCha8Rng seeded by (world_seed, sampler_id)
     - Deterministic index into BTreeMap-keyed corpus
     - NO LLM calls
```

## Validation gates (FW-VAL contract)

Per `docs/specs/content-pack-validation-contract.md`:
- Schema conformance (every entity has `id`, `schema_version`, required fields)
- ID uniqueness (no two `fwh.core:player_00042`s)
- Length bounds (names 2-30 chars, descriptions ≤200 chars)
- Banned-terms catalog check (`scripts/lint-banned-terms.py`)
- Cross-references resolve (a player's `archetype_id` must exist in the archetype RON)
- Tracery grammar well-formedness (parseable, no orphan rules)

Run with `cargo run --bin fw-content-baker -- validate`.

## Cost-awareness

- Rate limiter on the API client (configurable; default 5 req/s).
- Cache by prompt hash — re-running with no prompt changes is free.
- Total bake budget tracked in `content/baked/.bake-stats.json` (gitignored — informational).
- Re-bake only what changed (Makefile-style dependency tracking).

## Determinism considerations

- LLM output is non-deterministic at bake time — that's WHY we bake and commit the result. Commit time is the determinism boundary.
- Once `content/baked/` is committed, runtime sampling from it is deterministic (ChaCha8Rng-driven).
- Mod overlays can swap baked entities; the `mod_load_fingerprint` hash captures the active set.

## Failure modes

- **LLM returns unparseable JSON:** validator rejects → retry with backoff. Persistent failure → flag to user, do not commit partial.
- **API key missing:** baker errors out clearly. `just bake-content` requires `ANTHROPIC_API_KEY`.
- **Banned-terms violation:** baker rejects + prints the offending term. Author re-seeds the prompt to nudge LLM away.
- **Cross-reference fails:** baker rejects. Probably means a prerequisite RON wasn't baked first; check the bake order.

## Cross-references

- `crates/fw-content-baker/` — the CLI
- `crates/fw-content/` — the schema being validated against
- `docs/CONTENT_PIPELINE.md` — full bake + runtime spec
- `docs/specs/content-pack-validation-contract.md` — FW-VAL rules
- `Content/RULES.md` — RON authoring rules
- `narrative-director` agent — owns voice + tone of bake-time prompts

# `content/` — procedural-content corpus

Final Whistle's procedural-fantasy world is compiled at **bake time** by
`fw-content-baker` (a dev-only CLI that calls the Claude API) into versioned
RON corpus files. The shipping runtime (`fw-content::runtime`) loads the
corpus once at startup and samples deterministically. **No HTTP calls.
No API keys. No runtime LLM.**

Full pipeline spec: [`../docs/CONTENT_PIPELINE.md`](../docs/CONTENT_PIPELINE.md).

---

## Layout

```
content/
  baked/        ← OUTPUT of fw-content-baker. Gitignored at T0 (baker hasn't
                  shipped yet); allowlisted once baked artifacts start landing
                  per MASTER_PLAN T2-3. The manifest.ron is always tracked
                  and pins corpus_version + model_id + prompt_hash + seed.
  sources/      ← INPUT to fw-content-baker. Hand-authored seeds the baker
                  reads + extends. Always tracked.
    cultures/   ← Per-culture naming priors (~20 cultures at MVP +
                  3 fantasy archetypes at Stretch).
    archetypes/ ← Hand-authored tactical archetypes (formation, press radius,
                  buildup speed). DELIBERATELY NOT LLM-BAKED — drives
                  canonical sim behavior; a one-coordinate drift would shift
                  the pinned canonical-state hash.
    grammars/   ← Hand-authored Tracery grammars (headlines, manager quotes,
                  fan reactions). Baker extends per event class.
  mods/         ← User-writable mod overlay root. Each subdirectory is one
                  mod, identified by `mod_id` in its manifest.ron. Loaded in
                  lexicographic order over the base corpus.
  historical/   ← Shipped historical corpus snapshots, used when a save
                  file references an older corpus_version than the current
                  build ships. Populated as new corpus versions ship.
```

---

## Mod loading order

1. **Base corpus** from `content/baked/**.ron` loads first.
2. **Mod packs** load in lexicographic order by `mod_id` (top to bottom).
   Deterministic by construction.
3. **Per-file precedence: last writer wins.** A mod that ships
   `names/anglo.ron` fully replaces the base file for that culture.
   No field-level merge — granular merging breaks determinism reasoning.
4. **Sealed paths:** mods cannot override `content/sources/archetypes/**`
   (sim-bearing tactical data). The runtime refuses to load any mod that
   touches that subtree.
5. **Determinism preserved per `mod_load_fingerprint`**: BLAKE3 over the
   sorted `(mod_id, mod_version, content_file_hash)` tuples is written to
   every save. Fingerprint match ⇒ identical content; mismatch ⇒ player
   warned; previously-materialized entities retain their stored names.

Mod manifest contract (`content/mods/<mod-id>/manifest.ron`):

```ron
ModManifest(
    mod_id:                  "aaa-bigger-names",  // ^[a-z0-9-]{3,40}$
    display_name:            "Bigger Name Banks",
    version:                 "0.2.0",
    author:                  "communitymember",
    description:             "Doubles every culture's first-name + last-name bank.",
    targets_corpus_version:  4,
)
```

Mods declaring a `targets_corpus_version` that mismatches the loaded base
trigger a compatibility warning. They still load; players choose whether
to continue.

---

## Determinism guarantees

| Layer | What's guaranteed | Where it lives |
|---|---|---|
| Sampling | `derive_seed(career_seed, entity_id, kind)` ⇒ same `u64` across OSes | `fw-content::runtime::derive_seed` |
| Iteration | All bank lookup via `BTreeMap` / `Vec`; no `HashMap` | `fw-content::ContentStore` |
| Cross-OS | BLAKE3 + ChaCha8Rng — no platform-specific paths | tested on `macos-14` + `windows-latest` + `ubuntu-22.04` CI |
| Mod-stable | `mod_load_fingerprint` + materialized-name caching in saves | `fw-save` (T2-9) + `fw-content` runtime |
| Corpus-stable | `corpus_version: u32` in every baked artifact + save reference | `content/baked/manifest.ron` |

---

## Adding a new culture

1. Author the seed at `content/sources/cultures/<id>.ron` with ~30+ entries
   per bank (the baker extends, but a viable seed needs hand-authored
   ground truth).
2. Run `fw-content-baker bake-names --culture <id>` (lands at T2-3).
3. Review the baker output for clichés, real-name overlap, and
   banned-term hits.
4. Commit the baked RON + the manifest.ron update; CI runs the cross-OS
   determinism gate.

---

## Adding a new mod

1. Create `content/mods/<mod-id>/manifest.ron` with the contract above.
2. Add RON files mirroring the `content/baked/` layout you want to override.
3. Launch the game; the mod surface shows it loaded; the
   `mod_load_fingerprint` updates in your next save.

Mods are plain RON files — user-editable in any text editor. The
`fw-content` runtime parses them at load; no compilation step required.

---

## Cross-refs

- [`../docs/CONTENT_PIPELINE.md`](../docs/CONTENT_PIPELINE.md) — full pipeline spec.
- [`../docs/design/ui-vocabulary.md`](../docs/design/ui-vocabulary.md) — banned-terms lint catalog (source of truth).
- [`../crates/fw-content/`](../crates/fw-content/) — runtime sampling layer.
- [`../crates/fw-content-baker/`](../crates/fw-content-baker/) — bake-time CLI.

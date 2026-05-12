---
description: Procedural content pipeline for Final Whistle — bake-time LLM generation + runtime deterministic sampling. Pairs with docs/DESIGN_DOC.md §6 and docs/MASTER_PLAN.md T2-3 + T3-3.
last_verified: 2026-05-13
status: Scaffolded T0 (structure + spec); implementation lands T2-3 (baker) + T3-3 (Tracery runtime).
---

# Content Pipeline — bake-time + runtime contract

> **The LLM USP runs at bake time, not runtime.** Every procedural-fantasy
> string the game ever surfaces — player names, club names, stadium names,
> biographies, scout-report phrases, news headlines, manager quotes, fan
> reactions, match commentary — is compiled to versioned RON corpus files
> before any build ships. Runtime loads the corpus once and samples
> deterministically.
>
> This doc defines the contract. Implementation lives across `fw-content` /
> `fw-content-baker` / `content/`.

---

## 1. Two-phase architecture

### Phase A — BAKE TIME (offline, dev-machine only)

```
        prompt template (md)           dev runs:
        + culture / archetype  ───►   $ fw-content-baker bake-names
            seed RON              ┐   $ fw-content-baker bake-bios
                                  │   $ fw-content-baker bake-headlines
                                  ▼   $ fw-content-baker bake-all
                             ┌──────────────┐
                             │ Claude API   │   (anthropic-sdk-rust)
                             └──────┬───────┘
                                    │ JSON fragments
                                    ▼
                             ┌──────────────┐
                             │ JSON Schema  │   reject malformed / off-spec
                             │ validator    │   reject banned terms (lint)
                             └──────┬───────┘
                                    │ validated fragments
                                    ▼
                             ┌──────────────┐
                             │ RON compiler │   stable-ID assignment
                             │              │   manifest emission
                             └──────┬───────┘
                                    │
                                    ▼
                             content/baked/<kind>/<culture>.ron
                             content/baked/manifest.ron
                             (the manifest pins model_id +
                              prompt_hash + seed +
                              corpus_version + generated_at)
```

The baker is a **dev-only CLI**. It calls the Claude API directly. Its output
is reviewed and committed as RON. **The committed RON is the source of truth
— not the prompt, not the model.** Regeneration produces a delta pack with a
bumped corpus version.

### Phase B — RUNTIME (in-product, no network)

```
                        startup:
                  ┌───────────────────────┐
                  │ ContentStore::load    │   reads content/baked/*.ron
                  │                       │   merges content/mods/<id>/*.ron
                  └──────────┬────────────┘   per mod load order
                             │
                             ▼
                  ┌───────────────────────┐
                  │ ContentStore (frozen) │
                  └──────────┬────────────┘
                             │
            sample call from sim / readers:
            sample_player_name(culture, seed)
                             │
                             ▼
                  ┌───────────────────────┐
                  │ ChaCha8Rng seeded by  │   (career_seed, entity_id, kind)
                  │ derive_seed(...)      │   → BTreeMap iteration
                  └──────────┬────────────┘   → deterministic index
                             │
                             ▼
                          "Henry Tabor"
```

**No HTTP calls. No API keys. No `tokio` in this path.** The runtime never
re-bakes; it samples a frozen committed corpus. Missing-pack fallback uses a
placeholder generator with a UI badge flagging the gap.

---

## 2. Content categories

| Category | Shape | Bake target | MVP / Stretch | Owner reader |
|---|---|---|---|---|
| **Player names** | Per-culture first-name + last-name banks (~20 cultures) + naming-pattern grammar + Markov-chain weights | RON per culture under `content/baked/names/` | **MVP (T2-3)** | `fw-content::sample_player_name` |
| **Club names** | Generator grammar (city + suffix, animal + suffix, founder-name) + curated bank per culture | `content/baked/clubs/<culture>.ron` | **MVP (T2-3)** | `fw-content::sample_club_name` |
| **Stadium names** | Same shape as clubs; tier-weighted (Tier 1 grander suffixes) | `content/baked/stadiums/<culture>.ron` | **MVP (T2-3)** | `fw-content::sample_stadium_name` |
| **Player biographies** | ~200 templates × 6 role archetypes × 20 cultures ≈ 24k starter snippets, sampled with slot-filling for name + age + birthplace | `content/baked/bios/<culture>/<archetype>.ron` | **MVP (T2-4)** | `fw-content::sample_bio` |
| **Scout-report phrases** | ~500 phrase templates with positive / neutral / negative variants, keyed by archetype + observed phenotype label | `content/baked/scout-phrases/<archetype>.ron` | **MVP (T3-5)** | `fw-scouting::sample_phrase` |
| **News headlines** | Tracery grammar with weighted alternatives per event class (breakthrough goal / sacking / derby / upset / contract drama) | `content/baked/headlines/<event-class>.tracery.json` | **MVP (T3-3)** | `fw-content::sample_headline` |
| **Manager quotes** | Tracery grammar keyed by archetype + outcome class | `content/baked/manager-quotes/<archetype>.tracery.json` | **MVP (T3-3)** | `fw-content::sample_manager_quote` |
| **Fan reactions** | Tracery grammar per fan-base mood + recent result | `content/baked/fan-reactions/<mood>.tracery.json` | Stretch (T3+) | `fw-content::sample_fan_reaction` |
| **Match commentary** | Per-event-type pool (goal / save / miss / foul / card / sub / kick-off / full-time) — ~50 templates per event type per ui-vocabulary.md MVP target ~140 total | `content/baked/commentary/<event-type>.ron` | **MVP (T1-6 stub; T3-3 full)** | `fw-match-sim::commentary::sample_line` |
| **Cultures** | First-name + last-name banks, naming patterns, weight knobs | `content/sources/cultures/*.ron` (hand-authored seed, baker EXTENDS but never replaces) | **MVP (T2-3)** | `fw-content::Culture` |
| **Tactical archetypes** | Formation + press-radius + buildup-speed + BT-archetype reference (port of FW v1 `direct-pressing.yaml`) | `content/sources/archetypes/*.ron` (hand-authored, NOT LLM-baked — too load-bearing) | **MVP (T1-1)** | `fw-match-sim::BehaviorArchetype` |

**Tactical archetypes are deliberately NOT baked.** They drive canonical sim
behavior; a one-character drift in the formation x/z coordinates would shift
canonical-hash output. Hand-authored RON, reviewed, committed. See
`design/match-sim.md` for the BT-runner contract once authored.

### Culture archetypes (20 total at MVP)

**Real-world-coded** (per `worldbuilding.md` cohort priors — names are
LLM-generated *novel* names with cultural priors, never licensed real names):

`anglo`, `germanic`, `slavic`, `latin`, `nordic`, `francophone`, `iberian`,
`italic`, `hellenic`, `yoruba`, `bantu`, `arabic`, `persian`, `turkic`,
`han`, `japanese`, `korean`, `south-asian`, `southeast-asian`, `andean`.

**Fantasy archetypes** (procedural-fantasy USP — credible naming priors with
no real-world referent): `fantasy-elvish`, `fantasy-dwarven`,
`fantasy-orcish`. These ship as Stretch in T3+; MVP starts with 20
real-world-coded cultures because the per-nation pyramid is the EA target.

---

## 3. Determinism guarantees

The pipeline must produce **byte-identical content for any entity given the
same `career_seed` + same corpus version**. This is load-bearing for the
pinned canonical-state hash regression discipline.

### 3.1 Sampling determinism

- All randomness goes through `ChaCha8Rng` seeded by a derivation function:
  ```rust
  pub fn derive_seed(career_seed: u64, entity_id: u64, kind: ContentKind) -> u64
  ```
  The derivation uses BLAKE3 over a fixed-order byte buffer
  `(career_seed | entity_id | kind_tag)`; output is `u64`.
- `ContentKind` is an enum with `#[repr(u8)]` and a locked discriminant
  range. **Adding a variant must be a corpus-version bump.** Reordering or
  removing variants is forbidden mid-version.
- All iteration over corpus banks goes through `BTreeMap` / `BTreeSet` /
  `Vec`. `HashMap` is banned (clippy-enforced via the workspace `[lints]`).
- No `Instant::now()`, `SystemTime::now()`, `thread_rng()` in `fw-content`
  runtime paths.

### 3.2 Corpus version pinning

- Every baked artifact carries a `corpus_version: u32` in its header.
- The manifest `content/baked/manifest.ron` pins the corpus version for the
  whole shipped pack + per-file `model_id`, `prompt_hash`, `seed` audit
  trail.
- Save files reference the corpus version they were created against. On
  load, version mismatch triggers a migration prompt:
  - **Same corpus_version** → load directly.
  - **Newer corpus_version (additive only)** → load; new content available
    for new entities, existing entities retain their original deterministic
    names.
  - **Newer corpus_version (breaking)** → migration prompt; cannot continue
    without a player choice.
  - **Older corpus_version than the shipped game** → load with the
    historical pack from `content/historical/<version>/` (if shipped) or
    refuse to load with a clear error.

### 3.3 ID assignment determinism

- Stable content-pack-qualified IDs per FW v1 ADR-0006:
  `^fwh\.core(?:\.v[0-9]+)?:<kind>_[a-z0-9_-]+$`.
- IDs are assigned by the baker at compile time, never at runtime. They
  appear in the RON; the runtime treats them as opaque keys.
- Pack-minor versions (`v1.1`) NEVER appear in entity IDs — only major-pack
  namespace (`.vN`) is permitted. Cross-ref `design/modding.md` §stable-IDs.

### 3.4 Test surface

- `fw-content::tests::names_deterministic` — sample 1000 names from the same
  `(culture, career_seed)`; assert byte-identical output across 100 reps.
- `fw-content::tests::corpus_version_round_trip` — load corpus, serialize,
  reload; assert byte-identical RON.
- Cross-OS CI matrix (`macos-14`, `windows-latest`, `ubuntu-22.04`) runs
  these tests on every commit. Drift on any platform fails merge.

---

## 4. Modding overlays

The pipeline is **mod-friendly from day one** (cross-ref
`design/modding.md` 12-constraint contract).

### 4.1 Filesystem layout

```
content/
  baked/                          # sealed — ships with the game build
    manifest.ron                  # corpus version + audit trail
    names/<culture>.ron
    bios/<culture>/<archetype>.ron
    headlines/<event-class>.tracery.json
    commentary/<event-type>.ron
    scout-phrases/<archetype>.ron
    ... (one subdir per category)
  sources/                        # hand-authored seeds + LLM prompt input
    cultures/<culture>.ron        # per-culture naming priors
    archetypes/<archetype>.ron    # tactical archetypes (NOT baked — sim-bearing)
    grammars/<grammar>.tracery.json  # hand-authored Tracery templates
  mods/                           # mod overlay root — user-writable
    <mod-id>/
      manifest.ron                # mod metadata + base corpus_version it targets
      names/<culture>.ron         # OVERRIDES the baked file at this path
      bios/<culture>/<archetype>.ron
      ...
  historical/                     # shipped historical corpus snapshots for save-load
    <corpus-version>/
      manifest.ron
      ...
```

### 4.2 Mod loading order

1. **Base corpus** loads first — `content/baked/**.ron`.
2. **Mod packs** load in **lexicographic order by mod-id**, top to bottom.
   `aaa-fantasy-extra` loads before `zzz-realism-tweaks`. Deterministic by
   construction.
3. Each mod's `manifest.ron` declares:
   - `mod_id: String` (must be unique; lint enforces `^[a-z0-9-]{3,40}$`)
   - `targets_corpus_version: u32` (rejected if mismatched; player sees a
     compatibility warning)
   - `display_name`, `version`, `author`, `description`
4. Per-file precedence: **last writer wins** within a content category.
   A mod that ships `names/anglo.ron` fully replaces the base file for that
   culture. There is no field-level merge — granular merging breaks
   determinism reasoning. (Mods that want to *add* a single name fork the
   whole culture file.)
5. **Mods cannot override `content/sources/archetypes/**`** — tactical
   archetypes are sim-bearing and would break the pinned-canonical-hash
   regression. The runtime refuses to load a mod that touches that path
   (lint hook + load-time guard).

### 4.3 Mod-determinism preservation

The naive question: "if mods can replace name banks, how does
`(career_seed, entity_id)` still produce byte-identical names?"

**Answer:** determinism is preserved **within a given mod-load-order
fingerprint**. The save file records:
- `corpus_version: u32`
- `mod_load_fingerprint: [u8; 32]` — BLAKE3 over the sorted list of
  `(mod_id, mod_version, content_file_hash)` tuples.

On load:
- **Fingerprint matches** → deterministic replay; names + bios identical.
- **Fingerprint mismatches** → player sees a warning. They may continue;
  names of entities generated AFTER the mod swap will use the new corpus,
  but pre-existing entities' names are **cached in the save file** (per
  ADR-0006-derived discipline: names are written to the save the first time
  they're materialized, not re-sampled on every load).

This means: **deterministic for a fixed mod set**, **stable for previously
materialized entities under any mod set**, **gracefully degraded for new
entities after a mod swap**. The pinned-hash regression corpus runs against
the base corpus only (no mods), so the sim's canonical-hash gate is
unaffected by user mod activity.

---

## 5. Banned-terms lint integration

The banned-terms lint (`scripts/lint-banned-terms.py`, ported verbatim from
FW v1) is the structural floor that stops procedurally generated content
from leaking real-world place names, mystical state nouns, or stigmatizing
phenotype framings into player-facing surfaces.

### 5.1 Where the lint runs

1. **Pre-bake** — on the prompt templates under
   `crates/fw-content-baker/src/prompts/**.md`. Catches authored prompt
   text that would itself encourage banned-term output.
2. **Post-bake, pre-commit** — the baker pipes every generated fragment
   through the lint before writing RON. A Category-A hit rejects the bake
   for that fragment; the dev reviews the prompt + reroll seed. Category-B
   hits log to the audit JSON for review-before-EA.
3. **CI on `content/baked/**.ron`** — `scripts/fw banned-terms` runs on
   every commit; any drift fails the gate.
4. **CI on rendered runtime UI** — once `pnpm tauri build` exists, a
   `--scope frontend/src/**` invocation lints the SolidJS surface for any
   hardcoded strings.

### 5.2 Catalog source-of-truth

`docs/design/ui-vocabulary.md` (stubbed in this scaffold). When the catalog
changes, the lint patterns in `scripts/lint-banned-terms.py` update too;
CI spot-grep verifies the doc and the lint stay in sync. The lint has its
own sentinel-comment exemption mechanism for meta-references in design docs.

### 5.3 Baker-specific lints (extends the FW v1 catalog)

The baker adds two extra validators on top of the FW v1 lint:

- **No-licensed-data** check: regex match against a curated list of real
  Premier-League / La-Liga / Bundesliga / Serie-A / etc. clubs + canonical
  player surnames. A hit rejects the fragment outright (Category A).
- **Cliché detector**: rejects bio sentences that match common LLM tells —
  "passionate about", "exceptional ability to", "rising star with bright
  future". Lint catalog in `crates/fw-content-baker/src/validators.rs`.

The cliché detector is opinionated — devs can override per-fragment with a
sentinel comment in the bake log, but the default is reject.

---

## 6. What ships when

| Milestone | Deliverable |
|---|---|
| **T0 (now)** | Scaffolded structure + this doc + stub `fw-content-baker` CLI + stub `fw-content::runtime` + example culture / archetype / grammar files + banned-terms lint port |
| **T1** | Hand-authored seed: 22 player names via Markov-chain over a single culture (T1-7); 2 team names; 1 manager archetype. No LLM bake yet. |
| **T2-3** | First real bake: `fw-content-baker bake-names` produces 100 player names + manifest; offline runtime reproduces identically. Names baker is MVP. |
| **T2-4** | `PlayerBio` generation with 22-field gene model; 500 players with phenotype labels. |
| **T3-3** | Tracery-grammar runtime + news headlines + manager quotes baked. Commentary phrase banks baked per event type. |
| **T3-5** | Scout-report prose templates baked + per-archetype variant pools. |
| **Stretch (T3+)** | Fantasy culture archetypes (Elvish / Dwarven / Orcish); fan-reaction grammars; commentary depth ceiling raised to 8-12 per common event per `DESIGN_DOC.md` §12 question 7. |
| **Stretch (T4+)** | Steam Workshop UX shell (data shape locked at T2; UX is T4+ polish). |
| **Stretch (post-EA)** | Per-locale baked corpus; translator handoff via the manifest audit trail. |

---

## 7. Cross-refs

- `docs/DESIGN_DOC.md` §6 — high-level content pipeline contract.
- `docs/MASTER_PLAN.md` T2-3 (baker), T2-4 (PlayerBio), T3-3 (Tracery), T3-5 (scouting).
- `docs/design/ui-vocabulary.md` — banned-terms lint catalog (source of truth).
- `design/modding.md` (Unity-era; carry-forward via MIGRATION_AUDIT.md §2.2) — 12-constraint mod-readiness contract.
- `design/player-generation.md` (Unity-era; carry-forward) — 22-field gene model + 46-label phenotype catalog.
- `design/scout-disagreement.md` (Unity-era; carry-forward) — scout archetype + ScoutReport schema.
- ADR-0006 (Unity-era) — IdentityPacket compiler + content-pack ID rules.

---

*Authored 2026-05-13. Revise at every phase transition; bump corpus_version on every breaking change to a baked-content schema.*

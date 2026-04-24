---
description: Data-architecture contract every system must respect so Final Whistle content packs stay mod-loadable from day one, even while the editor UX is deferred post-EA. Synthesizes cross-ADR constraints that have been accumulating since Phase 0; introduces no new architecture.
last_verified: 2026-04-24
status: Phase 2 authoring pass — consolidates 2026-04-22 "mod-ready data architecture, editor UX deferred" bootstrap decision into a single reference. Constraints below are all already locked via ADRs 0001/0004/0005/0006/0007 + TECH_APPROACH §4 + `design/specs/*`. This doc is synthesis, not new commitment.
---

# Modding — data-architecture contract

## Purpose

Answer "what does every system in Final Whistle have to respect so that modders can load third-party content packs cleanly at EA, and so that we don't paint ourselves into a corner when the editor UX ships post-EA?"

Framing: mod-readiness is an architectural posture, not a feature. The editor UX is deferred. The data contract is not. Every ADR authored in Phase 2 was built with mod-pack loadability in mind; this doc collects those constraints in one place so any new system added in Phase 3+ can consult a single authoritative reference instead of re-deriving from scratch.

## Locked decisions

See SPEC.md 2026-04-22 `Mod-ready data architecture from day one; editor UX deferred` and the full Phase-2 ADR run. Summary:

- **Data architecture is mod-ready from day one.** Stable IDs, schema versions, content packs, import validation, Workshop-ready ID conventions all baked in before any mod tooling exists.
- **Editor UX deferred post-EA.** No in-game mod editor, no authoring UI, no live-reload workflow ships at EA. Mods are authored externally against the content-pack schemas + validators shipped with the game.
- **Workshop readiness is an ID + manifest contract, not a code contract.** Runtime loads the same ScriptableObjects and content-pack manifests whether they come from the base pack, a delta pack, or a Steam Workshop mod pack — if the validator passes, the pack loads.

## The contract — twelve constraints every system respects

### 1. Content-pack-qualified stable IDs, no pack-minor leak

Every addressable game entity (player, club, signature, shot type, scout archetype, memory event class, callback tag, phenotype label, behavior-tree archetype, manager archetype) uses a `ContentPackQualifiedId` of the form:

```
fwh.core:<kind>.<slug>           — canonical form (e.g. fwh.core:signature.low-cutback-from-byline)
fwh.core.v1:<kind>.<slug>        — explicit major-version form, optional
fwh.core.v1:player_00042         — numeric sequential form for compiler-generated entities
```

Canonical validator regex for player-style numeric IDs: `^fwh\.core(?:\.v[0-9]+)?:player_[0-9]{5}$`.

**Pack-minor versions (`v1.1`, `v1.2`) NEVER appear in entity IDs.** Pack-minor lives in the manifest as `introduced_in_pack_version: "1.1.0"` per-entity. A delta pack adds new entities at fresh sequential IDs; it never renames or rewrites an existing ID. This is what makes mod references to base-pack entities safe across patches.

**Sources:** `TECH_APPROACH.md §4.1`, `design/player-generation.md §ID stability`, ADR-0006 `§Schema locks`, ADR-0001 `§Addressables grouping`, ADR-0005 `§Stable ID contract`, ADR-0007 `§ScoutArchetype schema`.

### 2. Schema versioning + load-time forward migration, no downgrades

Every serialized subject (save file envelope, MemoryEvent ledger, content-pack manifest, IdentityPacket, ScoutReport persistence) carries a `schema_version: int`. Loaders run forward-only migration chains (`MigrationChain.Migrate(event, toVersion)`) at load time. Downgrades are NOT supported — a save authored at v3 cannot be opened by a v2 binary. The save envelope carries `max_supported_schema_version` so the loader fails loud rather than silent when a save is ahead of the binary.

**Every schema bump requires four tests** per `design/specs/save-migration-fixtures.md`: forward-migration + callback-preservation + forward-incompat + round-trip byte-identical. Schema-bump PRs without these four tests are unmergeable.

**Sources:** ADR-0004 `§Migration framework`, ADR-0006 `§Schema evolution`, `design/specs/save-migration-fixtures.md`.

### 3. Per-content-pack Addressables grouping with stable labels

Each content pack owns its Addressables groups with labels that namespace the pack:

```
Group: shot-types-fwh.core           Labels: content-pack:fwh.core, kind:shot-type
Group: signatures-fwh.core           Labels: content-pack:fwh.core, kind:signature
Group: scout-archetypes-fwh.core     Labels: content-pack:fwh.core, kind:scout-archetype
```

Mod packs follow the same convention under their own pack ID (`shot-types-modname.pack`). Runtime queries by `kind:*` label cross all enabled packs; load order is deterministic per §4 below.

**Sources:** ADR-0001 `§Addressables grouping`, ADR-0005 `§Addressables grouping`, ADR-0007 `§Addressables grouping`.

### 4. Deterministic base-then-mod-pack precedence, lexicographic tiebreak

When multiple packs provide entities of the same kind:

1. **Base pack (`fwh.core`) resolves first.** Always. Base-pack entities cannot be overridden by mod packs at the ID level — they can only be extended by fresh IDs.
2. **Enabled mod packs follow in deterministic order, sorted by `pack_id` lexicographically.** Two mod packs sharing a `pack_id` is a validator failure — not an Addressables race.
3. **Within a pack, selection among candidates (e.g. which shot type fires this tick) is `priority DESC, Id ASC` tiebreak** — never Addressables load order, never dictionary iteration, never `DateTime.Now`, never `Random` without a seeded source.

Modders extend the game by adding entities at new IDs; they do not override base-pack content. This is the same discipline that lets save files reference `fwh.core:player_00042` across mod loads — the ID is stable because the base pack is the floor.

**Sources:** ADR-0001 `§Deterministic selection contract + §Forbidden nondeterminism sources`, ADR-0005 `§Stacking determinism`, ADR-0004 `§Id tiebreaker`.

### 5. Registry-backed IDs for anything content packs reference

Content-pack data files (YAML, JSON, ScriptableObjects authored externally) never carry scripted predicates, inline C# code, or free-form strings for system-referenced values. Instead they reference **registry-backed IDs** owned by code:

| Registry | Owner | Consumers |
|---|---|---|
| `ChainConditionId` | `FinalWhistle.Viewer.Cinema` | ShotTypeSO `chain_rules` |
| `SimBiasFieldId` | `MatchSim.Contracts` | SignatureSO `sim_bias_fields`, SimBiasSnapshot |
| `EventClass` | `Memory.Contracts` | MemoryEvent, SignatureSO `emits_on_*`, ScoutReport emitter |
| `CallbackTag` | `Memory.Contracts` | MemoryEvent, reader registrations (≥1 consuming reader per tag) |
| `PhenotypeLabelId` | `Content.Contracts` | IdentityPacket `scout_labels`, scout prose templates |
| `GeneCategory` | `Content.Contracts` | IdentityPacket `InternalGeneSnapshot`, ScoutArchetype biases |
| `ScoutArchetypeKind` | `Scouting.Contracts` | ScoutArchetype, ScoutReport |

These are **closed code-owned registries** for the currently shipped binary. Third-party EA mod packs may reference only values already present in the binary's Contracts registries; a content pack that references an unknown `ChainConditionId`, `PhenotypeLabelId`, `EventClass`, or equivalent registry value fails the validator at pack-import time. Registry expansion is a first-party schema + binary update with migration fixtures, not something an external Workshop pack can do by itself. Data-extensible catalogs — new signatures, shot types, scouts, prose templates, clubs, players, and IdentityPackets — extend through new `ContentPackQualifiedId`s that reference existing registry values.

**Sources:** ADR-0001 `§ChainConditionId registry`, ADR-0004 `§EventClass + CallbackTag`, ADR-0005 `§SimBiasFieldId ownership`, ADR-0006 `§PhenotypeLabelId`, ADR-0007 `§ScoutArchetypeKind`.

### 6. Contracts/impl asmdef split — MatchSim stays Unity-free

Every cross-boundary schema lives in a Unity-free `*.Contracts` asmdef that MatchSim can reference without pulling in UnityEngine. The asmdef surface:

```
MatchSim.Contracts          Fixed, Tick, Seed, SimBiasFieldId, SimBiasSnapshot
Memory.Contracts            MemoryEvent, EventClass, CallbackTag, SalienceInputs, ReaderQuery
Content.Contracts           IdentityPacket, InternalGeneSnapshot, PhenotypeLabelId, GeneCategory
Signatures.Contracts        (via Memory.Contracts + MatchSim.Contracts; no new package needed)
Scouting.Contracts          Scout, ScoutArchetype, ScoutReport, LabelEstimate, GeneCategoryEstimate
```

This is what makes mod packs authoring sane: mod tooling only has to target the Contracts surface, not the full Unity runtime. Future mod editor UX (post-EA) reads and validates against Contracts-defined schemas directly.

**Sources:** ADR-0004 `§Memory.Contracts split`, ADR-0005 `§Unity-free DTOs`, ADR-0006 `§4-project split`, ADR-0007 `§Scouting.Contracts`.

### 7. Canonical-JSON content-pack artifacts; LLM paths outside byte-identical regeneration

The checked-in JSON / ScriptableObject content-pack artifacts are canonical. LLM-assisted generation (name banks, prose templates, regional flavor seeding) produces reviewable deltas that become canonical only once the reviewed output is checked in. Compiler determinism means: given the same cohort spec + seed + checked-in name-bank artifact, the compiler produces byte-identical content-pack output. LLM output itself is not assumed bit-deterministic.

Mod packs follow the same rule: the pack artifact (JSON + SOs + manifest) is canonical; however the modder authored it is not the game's concern.

**Sources:** ADR-0006 `§Compiler pipeline + §LLM path outside byte-identical regeneration`, `TECH_APPROACH.md §4.2`.

### 8. Player-facing strings lint-scanned; internal enum IDs lint-exempt

Rendered strings across all content-pack fields (DisplayName, UiDescription, OverlayTextBank, phenotype-label prose, scout-archetype prose, commentary templates, press/fan copy) are scanned by the banned-terms lint per `design/ui-vocabulary.md`. Mod packs get the same scan at pack-import time. Category A is hard-ban, no exemption. Category B uses inline `ui-lint:allow term="..." reason="..." reviewer="..."` with audit reports reviewed before EA content lock + every RC.

Internal enum IDs (`EventClass.SignatureAwakened`, `SimBiasFieldId.early_cross_freq`, `GeneCategory.NarrativeFlag`) are lint-exempt — the scan targets the player-facing field set, not the symbol namespace.

**Sources:** ADR-0005 `§Lint-target separation`, ADR-0006 `§Banned-term lint on rendered strings`, `design/ui-vocabulary.md §Sentinel exemptions`.

### 9. Bake-time-only AI; no runtime LLMs, no runtime prose generation

Content packs ship as pre-baked structured data. All prose surfaced at runtime (match reports, press quotes, scout narration, fan sentiment, breakthrough-moment overlay text) is rendered from bake-time templates with runtime slot-filling from event-ledger state. Mods may ship their own templates, but templates are data — they cannot embed code, cannot call external services, cannot perform HTTP.

**Sources:** `CLAUDE.md §3 Tech stack`, `TOOLING.md §Anti-patterns`, ADR-0006 `§Alternative 5 — runtime LLM (rejected)`, ADR-0007 `§Alternative 5 — runtime LLM scout prose (rejected)`.

### 10. Determinism boundaries mods cannot cross

Mods cannot introduce:

- **Floats into the canonical MatchSim path.** Fixed-point Q32.32 is the canonical format. Mod-authored sim-bias values are declared as `Fixed`, never `float`.
- **`_Time` / `UnityEngine.Time.*` into sim code or gameplay-affecting shaders.** ADR-0002 + Phase-3 `fw shader-audit` enforce this for viewer shaders; mod-supplied visual packs will get the same scan.
- **`DateTime.Now`, unseeded `Random`, `Resources.Load`, platform-conditional behavior** into runtime code paths. Mods that ship C# assemblies (deferred post-EA in any case) face the same discipline.
- **Per-tick Unity-driven mutation of MatchSim state.** The `SimBiasSnapshot` pre-bake pattern (Unity-free DTO computed before sim execution) is the only supported way to inject signature-like effects. Mods bias the sim by authoring SignatureSOs that bake into snapshots — not by patching the tick loop.

**Sources:** `TECH_APPROACH.md §3.2 Determinism discipline`, ADR-0002 `§No _Time in shaders`, ADR-0005 `§Unity-free SimBiasSnapshot`, ADR-0001 `§Forbidden nondeterminism sources`.

### 11. Walled-off player internals; advanced tooltip default OFF

The 22-field `InternalGeneSnapshot` inside IdentityPacket is NEVER rendered directly to the player. It feeds the sim, feeds the scout-bias evaluator, and feeds the compiler's affinity rolls. Scout reports expose **category-level** `GeneCategoryEstimate` ranges (`LowerBound`, `UpperBound`) per the advanced tooltip contract, never raw gene values. The `NarrativeFlag` category is non-observable — every scout archetype has 0 weight against it (validator-enforced).

Mod packs inherit this wall. A mod-supplied ScoutArchetype cannot raise `NarrativeFlag` weight above 0; the validator rejects the pack. A mod-supplied phenotype label cannot carry raw gene values in its prose; the banned-term lint catches it.

**Sources:** ADR-0006 `§InternalGeneSnapshot wall-off + §Q3 advanced tooltip contract`, ADR-0007 `§NarrativeFlag zero-visibility validator`.

### 12. Content-pack validator surface mods must pass

Every content pack — base, delta, mod — passes the same validator before it loads. The import-safe Tier-A subset runs on every pack import; the full Tier-A set runs on every PR, including CI-only repository-discipline checks. The Tier-D full suite runs at RC. Per the Phase-6 content-pack-validator SPEC task:

**Tier A (blocking, fast):**
- ID format (canonical regex; no pack-minor leak in entity IDs)
- Duplicate-name detection within pack
- Invalid / unresolved `ContentPackQualifiedId` references
- Invalid / unresolved registry-backed IDs (unknown `ChainConditionId`, `EventClass`, `CallbackTag`, `PhenotypeLabelId`, `SimBiasFieldId`, `ScoutArchetypeKind`)
- Banned UI vocabulary in rendered strings (Category A hard-ban; Category B with inline exemption)
- `CallbackTag.ConsumingReaders ≥ 1` per tag
- `NarrativeFlag` scout-bias weight = 0 per archetype
- Missing `ShotTypeSO.reduce_motion_variant` for shots that use impact-flash or motion-line features (blocking by Phase-6 content-pack v1 / EA lock)

**Tier D (blocking, full matrix):**
- Legal-sensitive-name diff (detects real-world analogue leakage against `dev-config/compiler/region-analogues.json` and similar gitignored reference lists)
- Missing localizations against locked locale set
- Cross-doc event-class enum exact-match (`EventClass.SignatureAwakened` / `SignatureExecuted` / `ScoutReportConfirmed` / `ScoutReportDisagreement` match `design/signatures.md` + `design/scout-disagreement.md` + ADR-0005 / ADR-0007)
- AI-content disclosure manifest completeness (Steam's 2025 policy compliance)

**Sources:** `SPEC.md Phase-2 content-pack validation contract task`, ADR-0004 `§Cross-doc exact-match`, ADR-0006 `§Validator coverage`, ADR-0007 `§Validator invariants`.

## MVP boundary

**In at EA (mod-loadability):**
- Every system above respects the 12 constraints by Phase 3 code review.
- Workshop ID convention + manifest schema stable by Phase 6 (content pack v1 compile).
- Validator Tier-A subset live by Phase 3 (grows incrementally through Phase 6); Tier-D full suite live by Phase 8 RC.
- A third-party mod pack that passes Tier-A + Tier-D can load on the EA binary without code changes, provided it uses only the closed registry values already present in that binary.

**Out at EA (editor UX):**
- No in-game mod authoring editor.
- No mod-upload-to-Workshop flow from inside the game.
- No hot-reload workflow.
- No mod-compatibility matrix or dependency-resolver UI.
- No first-party mod marketplace outside Steam Workshop (Workshop scaffolding is Phase 6; the listing UI itself is post-EA).

## Deferred

Seeded now, surfaces post-EA contingent on audience signal:

- **Mod editor UX** — in-game authoring screens for ScriptableObjects, YAML behavior trees, commentary templates. Design draft waits for Phase 9+.
- **Dependency resolver** — multi-mod compatibility matrix, version pinning, conflict detection. Phase 9+.
- **Runtime hot-reload** — load/unload mod packs without restart. Phase 9+.
- **C# assembly mods** — currently not in scope at any phase; data-only mods are the EA + 1.0 surface. Revisiting requires a decisions-log entry.
- **Mod-authored shaders** — deferred indefinitely; shader surface requires the `fw shader-audit` discipline and mod-authored HLSL is a sandboxing risk.
- **Workshop paid mods** — out of scope; free-only if Workshop ships.

## Open questions

Deferred to Phase 3+ with the trigger condition named:

1. **Workshop manifest schema field list.** Steam Workshop integration lands at Phase 6; the exact manifest fields (dependency declarations, compatibility range, content-rating metadata) get locked at that point. Current assumption: the manifest is a superset of the content-pack manifest schema already defined by `Content.Contracts`.

2. **Mod-pack content-safety review.** Steam Workshop carries Valve's content rules; the game's own PEGI-12 posture adds requirements (no real-person likenesses, no explicit content). The review surface — automated lint vs manual reporting — gets spec'd alongside `design/content_policy.md`. Phase-2 task.

3. **Mod-to-save binding.** If a player's save references `modpack.foo:player_12345` and `modpack.foo` is disabled/uninstalled, what loads? Current default: the save loads with the mod-entity rendered as a "phantom" reference in the ledger, preserving callback eligibility but rendering UI as `[missing mod content]`. Alternative: fail-loud-on-load. Resolution gates at Phase 6 when the save migration matrix stabilizes.

4. **Determinism parity under mod load.** Golden replay corpus today pins `content_pack_version: fwh.core@1.0.0`. Replays generated against `fwh.core + modpack.foo@1.0.0` need their own golden set, or the mod must declare itself "replay-neutral" (no MatchSim-affecting changes). Policy locks at Phase 6 alongside the Tier-A smoke-seed rotation evaluation.

None of these block the Phase-2 gate. They're flagged here because `design/modding.md` is the contract owner; when they resolve, the resolutions land in the decisions log + update this doc's §Locked decisions, and any consequent constraint joins the twelve above.

## Prototype gate

No separate prototype gate for modding. The Phase-6 content-pack v1 compile itself IS the integration test:

- Content pack v1 authored against Contracts schemas ships ~96 clubs + ~2000-2400 players + 24 signatures + 7 shot types + scout archetypes.
- If content pack v1 loads clean, the 12 constraints above are demonstrably live.
- If a synthetic second pack (a "thin mod pack" — 1-2 new signatures, 1 new shot type, 5 new IdentityPackets using existing `PhenotypeLabelId` values, and 1 ScoutArchetype using an existing `ScoutArchetypeKind`) loads cleanly alongside `fwh.core` at Phase 6, the base-then-mod-pack precedence + validator + closed-registry reference discipline all pass.

Phase-6 SPEC task owed: author the synthetic thin mod pack as a CI fixture. Ships alongside `fwh.core@1.0.0` in the Phase-6 content-pack validator Tier-D run. Failure of the synthetic pack to load = content pack v1 is not actually mod-ready — fix before EA.

## Cross-references

- **ADRs (all Accepted 2026-04-24):** [0001 ShotTypeSO](adr/adr-0001-shot-type-so-schema.md) / [0002 Viewer rendering](adr/adr-0002-viewer-rendering-pipeline.md) / [0003 Production pipeline](adr/adr-0003-production-pipeline.md) / [0004 MemoryEvent](adr/adr-0004-memory-event-schema.md) / [0005 SignatureSO](adr/adr-0005-signature-so-schema.md) / [0006 IdentityPacket + AI Content Compiler](adr/adr-0006-identity-packet-compiler.md) / [0007 Scout archetype](adr/adr-0007-scout-archetype-schema.md)
- **Specs:** [golden-replay-corpus](specs/golden-replay-corpus.md) / [save-migration-fixtures](specs/save-migration-fixtures.md)
- **Engineering blueprint:** [TECH_APPROACH.md §3 MatchSim architecture + §4 Content pipeline](../TECH_APPROACH.md)
- **UI vocabulary:** [ui-vocabulary.md](ui-vocabulary.md)
- **Player generation:** [player-generation.md §ID stability + §RegionPriors](player-generation.md)
- **Production pipeline:** [production-pipeline.md §Tier-A + §Tier-D validator contracts](production-pipeline.md)

## Changelog within this doc

- **2026-04-24** — Authored as Phase-2 synthesis pass. Twelve constraints drawn from ADRs 0001-0007 + `TECH_APPROACH.md` + `design/specs/*`. Zero new architectural commitments; this doc is citation + consolidation. Open questions flagged for Phase 3+ resolution. No separate prototype gate — Phase-6 content-pack-v1 compile + synthetic-thin-mod-pack CI fixture IS the integration test.

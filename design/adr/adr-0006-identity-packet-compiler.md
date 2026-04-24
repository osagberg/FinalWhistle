---
description: ADR-0006 — IdentityPacket schema + AI Content Compiler pipeline. Phenotype enum governance, affinity-count rolls, content-pack ID rules, canonical-artifact discipline, scout visibility mapping.
---

# ADR-0006: IdentityPacket + AI Content Compiler

## Status

**Proposed** — pending user review before Accepted.

## Date

2026-04-24

## Last Verified

2026-04-24

## Decision Makers

osagberg (project owner), GPT-5.5 (design partner on 2026-04-24 player-generation resolution), Claude (workhorse).

---

## Summary

Lock the Pillar-2 player-authoring contract: `IdentityPacket` is the stable data shape for every generated player — internal 22-field gene model (never rendered) + player-facing phenotype labels (content-pack-qualified enum, banned-term-lint-enforced) + signature candidates with affinity weights + scout-visibility metadata. The AI Content Compiler produces canonical checked-in JSON artifacts (not prompt+seed+model) with manifest audit trails. Content-pack-qualified stable IDs never embed pack-minor versions. Scout-visibility operates at category level at MVP; narrative-flag category is never directly observable.

## Engine Compatibility

| Field | Value |
|---|---|
| Engine | Engine-agnostic schema — C# + JSON content packs; Unity-side only for Addressables grouping |
| Domain | Content Authoring / Player Generation | <!-- ui-lint:allow term="domain" reason="ADR template canonical field name for engine-compat area" reviewer="osagberg" -->
| Knowledge Risk | LOW — `System.Text.Json`, enum-backed IDs, Addressables-grouped content packs. Patterns already exercised by ADR-0001 (ShotTypeSO) + ADR-0005 (SignatureSO) |
| References Consulted | `design/player-generation.md` 2026-04-24 resolution, `design/signatures.md` §Affinity-distribution (authoritative cross-ref to this ADR), `design/worldbuilding.md` RegionPriors schema, ADR-0004 (MemoryEvent — ScoutReportConfirmed / ScoutReportDisagreement emission), ADR-0005 (SignatureSO — signature_candidates references SignatureSO.Id) |
| Post-Cutoff APIs Used | None |
| Verification Required | Phase-3 Week-2: author 22 hand-crafted `IdentityPacket` JSON fixtures for Month-3 slice; verify round-trip serialization + save-migration discipline per `design/specs/save-migration-fixtures.md` |

## Dependencies

| Field | Value |
|---|---|
| Depends On | ADR-0003 (Production pipeline — content-pack validator Tier), ADR-0004 (MemoryEvent — scout-event emission), ADR-0005 (SignatureSO — `signature_candidates[].signature_id` references) |
| Enables | ADR-0007 (Scout archetype — reads `internal_gene_snapshot` categories through bias filter), Phase-3 Week-2 22-player slice authoring, Phase-6 full ~96-club / ~2400-player content pack compilation |
| Blocks | Phase-3 Month-3 slice (can't author 22 players without packet schema); Phase-6 AI Content Compiler operational (can't compile without schema + validator contract) |

---

## Context

### Problem Statement

`design/player-generation.md` 2026-04-24 resolved all five open questions with: 22-field internal gene model locked, 46-label phenotype catalog (ceiling 50), default-off advanced tooltip, canonical-JSON-artifact reproducibility, ID-stability correction (no pack-minor in entity IDs). That resolution specified what to build. This ADR commits the schema + compiler contract so Phase-3 hand-authored packets can serve as the ground truth that Phase-6 compiler-generated packets must match byte-for-byte after round-trip.

### Current State

No Unity project. No compiler. No authored packets. Design doc has the 22-field model, the 46-label catalog, the affinity P(k) tables by cohort, and the compiler-pipeline outline.

### Constraints

- **Content-pack-qualified stable IDs; no pack-minor in entity IDs.** Hard rule per `design/player-generation.md` 2026-04-24 §Q5.
- **Internal gene values NEVER render in shipped UI** — player-facing surface is phenotype labels only. Advanced tooltip (opt-in) shows scout-estimated ranges, not internal truth.
- **Deterministic content compilation.** The **canonical artifact is the checked-in JSON**, not the prompt+seed+model combination. Regeneration with newer models produces new delta packs, never in-place mutations.
- **PEGI-12 content floor** — `design/ui-vocabulary.md` A.4 stigmatizing-phenotype bans + no real-people-name collisions in generated names.
- **Cross-doc exact-match discipline** inherits ADR-0004: `SignatureAwakened` / `ScoutReportConfirmed` enum names are consumed from `FinalWhistle.Memory.Contracts`, not duplicated.
- **Save-compat through schema bumps** — `IdentityPacket.schema_version` participates in the 4-test-per-bump discipline from `design/specs/save-migration-fixtures.md`.

### Requirements

- **Functional:** `IdentityPacket` schema with 22-field internal snapshot (non-rendered) + 46-label phenotype enum surface (lint-enforced) + signature-candidate list (references `SignatureSO.Id`) + scout-visibility metadata; AI Content Compiler pipeline with canonical-JSON-artifact discipline.
- **Performance:** content-pack load is O(N players) once at startup; packet lookup by `player_id` is O(1) via indexed catalog; Phase-6 compiler must produce ~2400 packets within a reasonable overnight run on solo-dev hardware.
- **Mod-pack compatibility:** content packs can ship additional `IdentityPacket` records (post-EA Workshop); existing packs' IDs never mutate.

---

## Decision

### `IdentityPacket` schema (Accepted)

Locked per `design/player-generation.md` 2026-04-24. Full shape lives in the design doc; this ADR commits to it:

```csharp
public sealed record IdentityPacket
{
    // --- Stable identity ---
    public ContentPackQualifiedId PlayerId;         // e.g. "fwh.core:player.00042" — never mutates
    public string DisplayNameFull;                  // banned-term-lint TARGET
    public string DisplayNameShort;                 // banned-term-lint TARGET
    public RoleFamily RoleFamily;

    // --- Playing instincts (authored, not derived) ---
    public PlayingInstincts Instincts;
    public PressureResponseCurve PressureResponse;

    // --- Development + signature surface ---
    public ImmutableArray<DevelopmentHook> DevelopmentHooks;
    public ImmutableArray<SignatureCandidate> SignatureCandidates;  // 0-3 entries; affinity lives here, NOT in SignatureSO

    // --- Scout-visible player-facing labels ---
    public ImmutableArray<PhenotypeLabelId> ScoutLabels;            // banned-term-lint TARGET (via enum → string rendering)
    public CommentaryHandles CommentaryHandles;

    // --- Narrative / rivalry / lineage metadata ---
    public ImmutableArray<RivalryCompatibility> RivalryCompatibility;
    public RegionId BirthRegion;
    public ImmutableArray<ContentPackQualifiedId> AlumniOf;         // club IDs for Coaching Lineage (data only; surfacing post-MVP)
    public ImmutableArray<TacticalDnaFragment> TacticalDnaFragments; // data only; Coaching Lineage surfacing post-EA

    // --- Internal gene snapshot (NEVER rendered in shipped UI) ---
    public InternalGeneSnapshot InternalGenes;                      // walled off; see below

    // --- Schema + provenance ---
    public ushort SchemaVersion;
    public ContentPackVersion SourcePackVersion;                    // manifest cross-ref; version metadata lives in pack manifest, NOT in PlayerId
}

public readonly struct SignatureCandidate
{
    public ContentPackQualifiedId SignatureId;      // references SignatureSO.Id — validator resolves at load
    public Fixed AffinityWeight;                    // Q32.32 [0,1] — likelihood of awakening
}

// Internal gene snapshot: 22 fields, walled off from any shipped UI surface.
// Scouts read this through a bias filter (ADR-0007); players never see raw values.
public sealed record InternalGeneSnapshot
{
    // Physical — 7 fields
    public Fixed HeightCeiling;
    public Fixed FrameDensity;
    public Fixed FastTwitchRatio;
    public Fixed StaminaRecovery;
    public Fixed GrowthCurve;           // signed: [-1, +1]
    public Fixed AgingCurve;
    public Fixed InjuryResilience;

    // Mental — 6 fields
    public Fixed PatternRecognition;
    public Fixed ComposureFloor;
    public Fixed DecisionVelocity;
    public Fixed LearningRate;
    public Fixed Ambition;
    public Fixed Mentality;             // signed: [-1, +1]

    // Technical affinities — 5 fields
    public Fixed LeftFoot;
    public Fixed Aerial;
    public Fixed DeadBall;
    public Fixed Striking;
    public Fixed FirstTouch;

    // Narrative trigger flags — 4 fields (rare; trigger-gated)
    public bool FlowAccess;             // ~1.5% carriers
    public bool PeakCeilingHigh;        // ~0.5%
    public LateBloomer LateBloomer;     // tagged enum: None | ByTier(tier) | ByAgeGate(age)
    public bool AwakeningDormant;       // ~0.2% very-late-career trigger
}
```

**Q32.32 for all gene numerics** — same cross-platform-determinism posture as `Stakes` / `Salience` in ADR-0004 and `Delta` in ADR-0005. Gene values feed scout-report generation; scout reports emit `ScoutReportConfirmed` / `ScoutReportDisagreement` `MemoryEvent`s whose `key_event_hashes` must match across Win/Mac/Linux.

### Phenotype label catalog (Accepted)

- **Enum-backed with content-pack-qualified IDs.** 46 labels at MVP, ceiling 50 per `design/player-generation.md` 2026-04-24 §Q2.
- **Banned-term lint scans ONLY the rendered-string form** of the label (e.g., `"Struggles Under Scrutiny"`). Internal enum identifier (`PhenotypeLabelId.StrugglesUnderScrutiny`) is lint-exempt — same Display/ID separation as ADR-0005.
- **Catalog growth past 50 triggers schema review**, matching the event-class-catalog ceiling discipline in ADR-0004.
- **PEGI-12 floor** is enforced by lint + manual review at authoring time. No labels referencing real-world ethnicity, religion, politics, or mental-health language. `design/ui-vocabulary.md` Category A.4 stigmatizing bans apply.

### Signature affinity — lives HERE, not in SignatureSO

Per ADR-0005's explicit handoff. `SignatureCandidate { SignatureId, AffinityWeight }` lives in `IdentityPacket`. `SignatureSO` is the *signature*; `IdentityPacket` is the *carrier*. Phase-6 content-pack validator verifies every `SignatureCandidate.SignatureId` resolves against the loaded `SignatureSO` catalog.

### Affinity-count distribution (locked policy; numeric seeds are Phase-6 tuning)

Per `design/player-generation.md` 2026-04-24 §Affinity-count-distribution + `design/signatures.md` cross-ref. Each generated player rolls `affinity_count ∈ {0, 1, 2, 3}` via a **cohort-weighted power-law tail**. The policy is locked; the numeric P(k) tables are Phase-6 tuning seeds that live in `design/player-generation.md`, not in this ADR or in SPEC.

**Roll procedure (locked, deterministic):**
1. Cohort assignment from club tier + squad-depth role + age bracket.
2. Roll `affinity_count` against cohort row using the seeded content-pack compiler RNG (never runtime nondeterminism; the compiler is a bake-time pure-C# process).
3. If `affinity_count > 0`: roll which specific signature candidates weighted by internal-gene-model alignment (`FastTwitchRatio > 0.7` biases toward winger/striker signatures, etc.).
4. Store in `IdentityPacket.SignatureCandidates` with `AffinityWeight ∈ [0, 1]`. Never more than 3.

### Scout visibility — category-level at MVP

Per `design/player-generation.md` 2026-04-24 §Gene-category-visibility-for-Scout-Disagreement:

- Scouts read `InternalGeneSnapshot` through a **category-level** bias filter (`physical` / `mental` / `technical` / `narrative_flag`). No per-field scout biases at MVP (deferred as tuning debt).
- **Narrative-flag category is NEVER directly observable by any scout.** Zero weight across all scout archetypes. Flags surface only through the events that trigger them (`LateBloomer` reveals via `SignatureAwakened` on the qualifying match; flag-discovery is retroactive).
- ADR-0007 (Scout archetype) locks the per-archetype category-weight tables; this ADR just commits the *mechanism* (category-level, narrative-flag-exempt).

### Advanced tooltip — default OFF, opt-in

Per `design/player-generation.md` 2026-04-24 §Q3:

- Default setting is OFF. Opt-in via settings toggle.
- When enabled, advanced tooltip exposes **scout-estimated ranges with uncertainty**, NEVER raw `InternalGeneSnapshot` values.
- Shipped builds NEVER expose raw internal values under any settings combination. Debug/dev builds may expose raw snapshots behind a build-time flag; shipped builds do not.

### Content-pack ID rules (locked; corrects the pre-2026-04-24 shape)

Per `design/player-generation.md` 2026-04-24 §Q5 resolution:

- Player IDs take the form `fwh.core:player.00042` or at most `fwh.core.v1:player.00042`. **Pack-minor versions (`v1.1`, `v1.2`) NEVER appear in entity IDs.**
- Pack-minor metadata lives in the **pack manifest** as `introduced_in_pack_version: "1.1.0"` per entity, NOT in the ID itself.
- **Every `ContentPackQualifiedId` is stable forever once shipped.** Renames = deprecation + new ID at next schema bump. Never mutate.

### AI Content Compiler pipeline (locked)

Canonical artifact is the checked-in JSON, not the prompt+seed+model. Pipeline:

```
1. Specify cohort  (human-authored cohort spec: region, club-tier distribution, name-pattern priors)
            │
            v
2. Seeded RNG rolls gene distributions via regional priors
   (pure-C# bake-time process; no Unity; no network; no wall-clock)
            │
            v
3. LLM generates names via seed prefix + FROZEN model version recorded in manifest
   (Claude API or similar; record the exact model version + prompt hash in the manifest)
            │
            v
4. Compile IdentityPackets from gene snapshots + names
            │
            v
5. Validate (Phase-6 content-pack validator):
   - schema correctness
   - no duplicate DisplayNameFull
   - no legal-sensitive-names collision (real-player-name database diff)
   - no banned phenotype-label leakage (Category A.4)
   - no real-world-place-name leakage in region-adjacent strings (Category A.5)
   - SignatureCandidate.SignatureId resolves against loaded SignatureSO catalog
   - IdentityPacket.schema_version matches current schema
   - ContentPackQualifiedId format: no pack-minor in ID
            │
            v
6. Sim sanity check: load packets into MatchSim; play 10-match harness; confirm no crashes / degenerate behavior
            │
            v
7. Commit to content pack with bumped pack_version on delta; manifest records
   generator={model, seed_prefix, prompt_hash, generated_at, matchsim_commit}
            │
            v
8. Phase-3: Unity Addressables import as SO assets (one SO per IdentityPacket, grouped per content pack)
```

### Architecture Sketch

```
Cohort Spec  ──►  SeededRngRolls  ──►  LLMNameGen  ──►  PacketCompiler  ──►  Validator
(human)          (deterministic,       (frozen-model       (pure-C#)           (Phase-6)
                  regional priors)     recorded)                                    │
                                                                                    v
                                                              ┌─────────────────────────────┐
                                                              │  Content Pack (JSON canon)  │
                                                              │  + manifest with generator  │
                                                              │    metadata + SHA-256 hash  │
                                                              └─────────────┬───────────────┘
                                                                            │
                                                    ┌───────────────────────┴────────────────┐
                                                    v                                        v
                                        ┌──────────────────────┐               ┌────────────────────────┐
                                        │ Unity Addressables   │               │ MatchSim balance harness│
                                        │ import (Phase 3+)    │               │ (Tier C local)          │
                                        │ - SO per IdentityPkt │               │ - 10K-match sweeps      │
                                        │ - Grouped per pack   │               │ - Sim sanity validation │
                                        └──────────────────────┘               └────────────────────────┘
```

### Implementation Guidelines

- **Asmdef / project boundaries:**
  - `FinalWhistle.Content.Contracts` — pure C#, no Unity. Defines `IdentityPacket`, `PhenotypeLabelId` enum, `SignatureCandidate`, `InternalGeneSnapshot`, `ContentPackQualifiedId`.
  - `FinalWhistle.Content.Compiler` — pure C# bake-time. Seeded RNG, regional prior consumer, LLM name-gen integration (via abstraction; swappable), packet compiler.
  - `FinalWhistle.Content.Validator` — pure C#. Phase-6 implementation. Consumed by Tier-A subset + Tier-D full.
  - `FinalWhistle.Content.UnityImport` — Unity-side. Addressables grouping per pack; generates SO assets from JSON at editor-time.
- **MatchSim.csproj depends on `FinalWhistle.Content.Contracts` ONLY** (packet shape for sim sanity checks). Same pattern as ADR-0004's `Memory.Contracts` split.
- **Canonical JSON serialization rules** match `design/specs/golden-replay-corpus.md` §stable-serialization: 2-space JSON, lowercase hex, no floats (Q32.32 integer representation), SHA-256 with `sha256:` prefix, structural-order keys, generator owns order.
- **Compiler determinism:** same cohort spec + same seed prefix + same frozen model version = byte-identical JSON output. If any input changes, it's a new pack version, not an in-place mutation.

---

## Alternatives Considered

### Alternative 1: Prompts + seeds as canonical artifact, regenerate at import

- **Description** — Store only the prompt + seed + frozen-model reference; regenerate the JSON on every project import.
- **Pros** — Smaller repo footprint; prompt = source of truth.
- **Cons** — LLM determinism is not bit-guaranteed across runs even with temperature=0 + fixed seed. The same prompt + seed + model can produce different tokenization choices in edge cases; accumulated drift breaks save compatibility for any save referencing the regenerated content. Also: the LLM provider becomes a hard dependency for anyone cloning the repo to run.
- **Rejected because** — canonical-JSON-artifact discipline is what makes the content pack reproducible-without-LLM-access and diffable. Prompts are recorded in the manifest for audit, not for regeneration.

### Alternative 2: Single flat `IdentityPacket` without `InternalGeneSnapshot` separation

- **Description** — Merge internal gene fields directly into the top-level `IdentityPacket` record.
- **Pros** — Simpler schema; one record per player.
- **Cons** — No structural wall between player-facing and developer-only fields. Any accidental UI surface that serializes "all fields" leaks raw gene values. Scout bias filtering also becomes awkward — there's no clean way to say "scout sees the player-facing fields + category-filtered gene snapshot, never both together."
- **Rejected because** — the wall is the point. `InternalGeneSnapshot` as a nested record makes "never render this" a structural property, not a code-review discipline.

<!-- ui-lint:ignore-start reason="alternative-rejection prose referencing the awakening affinity mechanic" -->
### Alternative 3: Affinity stored per-signature (not per-player)

- **Description** — Each `SignatureSO` lists which players can awaken it. Rejected in ADR-0005 already; restated here for cross-doc clarity.
- **Cons** — Architectural inversion. Affinity is a *player* property; signatures are *shareable definitions*. Per-signature affinity tables would force reverse-indexing at load time and couple player generation to every signature asset.
- **Rejected because** — same architectural inversion reason as ADR-0005 §Alternative 2. Affinity data lives on the carrier, not the thing being carried.
<!-- ui-lint:ignore-end -->

### Alternative 4: Per-field scout bias (not category-level)

- **Description** — Each scout has per-field weights across all 22 internal gene fields.
- **Pros** — Finest-grained scout differentiation.
- **Cons** — 22 fields × N scout archetypes = N×22 weights to tune. Balance-harness sweep becomes intractable. Marginal gain in scout-personality differentiation vs category-level, which already produces the "physical profiler misses decision-making" axis users actually care about.
- **Rejected because** — tuning debt. Deferred to post-EA if scout-disagreement Month-4 gate passes AND per-field differentiation becomes the top playtest feedback request (unlikely).

### Alternative 5: Runtime LLM content generation

- **Description** — Generate player names + biographies at save-creation time using the LLM.
- **Pros** — Infinite variety; no pre-baked content.
- **Cons** — Inference latency breaks match-day flow. Inference cost per save = recurring API spend. LLM provider becomes a runtime dependency. Offline play impossible. Content becomes un-reproducible (same save state produces different content on reload).
- **Rejected because** — runtime LLM is ruled out by `TOOLING.md §Anti-patterns`. Bake-time only is non-negotiable.

---

## Consequences

### Positive

- Canonical-JSON-artifact discipline makes content packs reproducible-without-LLM-access, diffable in PR review, and shippable without an API dependency.
- `InternalGeneSnapshot` as a nested record makes "never render this" structural — accidental UI leakage requires explicit serialization override, not default behavior.
- Signature affinity living in `IdentityPacket` matches the conceptual model (players carry affinities; signatures are portable definitions) and keeps ADR-0005 clean.
- Category-level scout bias at MVP ships with tractable tuning surface; per-field is a deferred optimization path.
- Content-pack-qualified IDs with no pack-minor leak match the Phase-8 Workshop-compat promise.
- Compiler is a pure-C# bake-time process — MatchSim never touches LLM artifacts; canonical sim path stays clean.

### Negative (Accepted Tradeoffs)

- Four-project split (`Contracts` / `Compiler` / `Validator` / `UnityImport`) is substantial scaffolding for Phase 3. Pays back through strict boundary discipline and Phase-6 validator cleanness.
- `InternalGeneSnapshot` walled-off design means scout-report generation needs explicit field access through the bias-filter abstraction — a small authoring cost on every new scout or report shape.
- Phase-6 compiler producing ~2400 packets is non-trivial (LLM cost, validator runtime, 10-match sim sanity per packet group). Solo-dev overnight-run feasible; not blocking.

### Neutral

- JSON content-pack format at MVP vs binary-packed content pack at Phase 6 is reversible via schema bump + migration fixture. JSON is the MVP floor for diffability.
- Compiler swappable for different LLM providers via abstraction — no lock-in on a specific vendor.

---

## Performance Implications

| Metric | Target | Notes |
|---|---|---|
| Content pack load (Phase 3 slice, 22 players) | <100ms | JSON parse + validator + Addressables import |
| Content pack load (Phase 8 EA, ~2400 players) | <5 seconds cold | Parallelizable across packs; cached Addressables |
| Compiler runtime (Phase 6, ~2400 packets) | Overnight on solo-dev hardware | LLM API rate-limit-bound; ~1-2 seconds per packet |
| Packet lookup by `PlayerId` | O(1) | Scene-load-indexed dictionary |
| Validator runtime (Tier-A subset) | <30 seconds | Fast checks: ID format, schema version, duplicate names |
| Validator runtime (Tier-D full) | 1-5 minutes | Full-pack: phenotype-label lint, real-player-name diff, SignatureCandidate resolution |

Phase-6 actuals supersede these targets.

---

## GDD Requirements Addressed

| GDD | System | Requirement | How This ADR Satisfies It |
|---|---|---|---|
| `design/player-generation.md` 2026-04-24 Q1 | 22-field internal gene model | Nested `InternalGeneSnapshot` with 7+6+5+4 fields | Schema locked |
| `design/player-generation.md` 2026-04-24 Q2 | 46-label phenotype catalog, ceiling 50 | `PhenotypeLabelId` enum with banned-term-lint on rendered strings | Label catalog via content-pack-qualified enum |
| `design/player-generation.md` 2026-04-24 Q3 | Advanced tooltip default OFF | Opt-in; shows scout-estimated ranges, never raw values | Non-commitment in schema; reader-side rule |
| `design/player-generation.md` 2026-04-24 Q4 | Canonical JSON artifact, not prompt+seed | Pipeline step 7 commits JSON; manifest records generator metadata | Compiler pipeline locked |
| `design/player-generation.md` 2026-04-24 Q5 | No pack-minor in entity IDs | `ContentPackQualifiedId` format; validator rejects pack-minor-in-ID | ID rules section |
| `design/signatures.md` §Affinity-distribution | Cohort-weighted power-law tail | `SignatureCandidate[]` storage on `IdentityPacket`; roll procedure locked | Signature affinity section |
| `design/scout-disagreement.md` §gene-category-visibility | Category-level bias filter | `InternalGeneSnapshot` read through bias filter; narrative-flag category zero-visibility | Scout visibility section |
| `design/worldbuilding.md` RegionPriors | Regional priors bias generation | Pipeline step 2 consumes RegionPriors additively | Compiler step 2 |
| ADR-0004 | ScoutReport* event emission | Scout-report generation emits via `Memory.Contracts` event-class constants | Cross-ref |
| ADR-0005 | Signature affinity lives here | `SignatureCandidate` record with `AffinityWeight` | Signature affinity section |

---

## Migration Plan

Not applicable — greenfield. First real migration exercise when `IdentityPacket.schema_version` v1 → v2 occurs (likely Phase-6 with save-schema v2 bump). `design/specs/save-migration-fixtures.md` 4-test discipline applies.

**Rollback:** if Phase-6 compiler proves unable to hit the ~2400-packet target within solo-dev budget, supersede with a hybrid approach (hand-authored core clubs + compiler-filled depth) via a new ADR. `IdentityPacket` schema itself is unlikely to need rollback; the compiler is where scalability risk lives.

---

## Validation Criteria

- [ ] Phase 3 Week 2: 22 hand-authored `IdentityPacket` JSON fixtures for Month-3 slice; schema-version 1; round-trip serialization clean.
- [ ] Phase 3 Week 4: Month-3 slice plays end-to-end with hand-authored packets; no raw `InternalGeneSnapshot` values leak to UI.
- [ ] Phase 6: AI Content Compiler produces 1 region's full club set (~10-15 clubs + ~250 players) that passes the validator and plays a 10-match sim-sanity sweep without degenerate behavior.
- [ ] Phase 6: Full ~96-club / ~2400-player content pack v1 compiled; validator green; storage budget met.
- [ ] Phase 6: Content-pack validator catches a red-team test where (a) an ID embeds a pack-minor version, (b) a banned Category-A.4 phenotype label slips into `PhenotypeLabelId` enum rendering, (c) a `SignatureCandidate.SignatureId` references an unknown signature.
- [ ] Phase 6: Round-trip regeneration — same cohort spec + same seed prefix + same frozen model version = byte-identical JSON.
- [ ] Phase 8: Content pack v1 shipped as canonical checked-in JSON; Workshop content packs post-EA can cleanly reference v1 IDs without mutation.
- [ ] Cross-platform parity: `InternalGeneSnapshot` Q32.32 gene values produce identical scout-report hashes on Win/Mac/Linux Tier-A CI matrix.

---

## Related

- Depends on: ADR-0003 (Production pipeline — validator Tier assignments), ADR-0004 (MemoryEvent — scout-event emission), ADR-0005 (SignatureSO — `signature_candidates[]` target).
- Enables: ADR-0007 (Scout archetype — category-level bias filter consumer).
- Cross-refs: `design/player-generation.md` 2026-04-24 resolution (source), `design/signatures.md` §Affinity-distribution (reciprocal reference), `design/worldbuilding.md` RegionPriors (compiler input), `design/ui-vocabulary.md` Categories A + B (phenotype-label lint), `design/specs/save-migration-fixtures.md` (schema-bump discipline), `design/specs/golden-replay-corpus.md` (canonical JSON serialization rules).
- Code (once implemented): `src/FinalWhistle.Content.*` + `unity-project/Assets/_Project/Content/` (paths tentative, finalize Phase-3 bootstrap).

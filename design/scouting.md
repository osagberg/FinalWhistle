---
description: Scout uncertainty model — biased observers, hidden gene truth, reports rendered as text bands not numbers. Path B (single-scout uncertainty) is the FW v2 MVP; Path A (3-archetype disagreement) is a conditional T4+ expansion.
last_verified: 2026-05-21
status: Ported + consolidated from two Unity-era archive sources — `design/scout-disagreement.md` (207 LoC) + `design/adr/adr-0007-scout-archetype-schema.md` (389 LoC), both locked 2026-04-24. C#/Unity references reconciled to Rust + `fw-scouting`; locked decisions preserved; phase/Month numbering aligned to MASTER_PLAN T-N. T3-5 unblocked by this port. See `docs/DECISIONS.md` 2026-05-21 T3-5 entry.
---

# Scouting — uncertainty as a gameplay surface

## Purpose

Answer "can scouting be a decision-generating system rather than an obscured-numbers
system?" FM uses fog over hidden numbers. Final Whistle makes the fog *itself* the
surface: a scout is a biased observer with a model, blind spots, and regional
familiarity. The player never sees the true `GeneSnapshot`; truth emerges from scout
reports, match-watching, and career outcomes over seasons (DESIGN_DOC §3 Pillar 4).

This is pillar 4. In a text-first sim the scouting screen *is* the main
player-evaluation surface, so the system is load-bearing.

## Locked decisions

Carry-forward from FW v1 SPEC.md 2026-04-22 + 2026-04-24 resolution + archive
ADR-0007; reconciled to FW v2 `docs/DECISIONS.md` conventions at this port.

- **No omniscient scouts.** Every scout is a biased observer with a model. The player
  never sees the canonical `GeneSnapshot` directly.
- **Two paths, one schema.** The `ScoutReport` data shape is **identical** whether the
  shipped system is Path A (disagreement) or Path B (uncertainty). Only the
  *population character* of the fields differs. This keeps the UI renderer and the
  signature-counterplay surface stable regardless of which path ships.
- **Structured data is canonical; prose is a rendered artifact.** `ScoutReport.labels`
  + `category_estimates` are what the UI and tests read. Prose is rendered
  deterministically from templates at generation time and stored for replay — never a
  runtime LLM (consistent with the bake-time content pipeline, ADR-0014).
- **Narrative-flag zero-visibility.** Scouts never observe the narrative-flag gene
  category. `GeneCategory` has no `NarrativeFlag` variant (compile-time exclusion);
  `CategoryBiases.narrative_flag` MUST be `0` (validator-enforced). Narrative flags
  surface retroactively through trigger events only (per `design/player-generation.md`
  + ADR-0002).
- **Category-level bias, not per-field.** Scouts bias at the gene-*category* level
  (physical / mental / technical) — not per-gene-field. 22 fields × N archetypes of
  tuning weight is intractable for the balance sweep; category-level is the
  MVP-appropriate granularity (archive ADR-0007 Alternative 4, inherited from
  ADR-0002's same call).
- **`ScoutReport` is event-class-free.** The report record holds no `EventClass`
  reference. The future emitter / career-loop layer selects which `MemoryEvent` class
  to emit per report. This keeps `fw-scouting` off an `fw-scouting → fw-memory`
  dependency edge and lets Path B drop a disagreement event class at a schema bump
  without touching the `ScoutReport` contract.
- **Determinism.** Identical `(scout, player_bio, career_seed, observation_id)` →
  byte-identical `ScoutReport`. All numerics `Q32`; RNG is `ChaCha8Rng` seeded via
  `seed_fn(.., SeedLayer::ScoutObservation, ..)` per ADR-0009. `fw-scouting` is a
  canonical-state crate (`float_arithmetic = "deny"`).
- **Track record is save-state, not archetype.** A scout's accumulating reliability
  lives in the save file, never in the immutable `Scout` archetype definition. (Track
  record itself is post-T3-5 — see Deferred.)

## The two paths

FW v1 framed the system as a **conditional MVP**: a Month-4 feel-prototype would
decide between Path A and Path B. FW v2 has no Month-4 calendar; the decision is
made structurally instead:

- **Path B — Scout Uncertainty — is the T3-5 MVP.** A single `BasicScoutUncertainty`
  scout produces one report per observed player: fog-over-numbers, the true genes
  hidden behind phenotype labels and text uncertainty bands. This delivers pillar 4's
  hidden-gene-model promise without the full disagreement mechanic.
- **Path A — Scout Disagreement — is a conditional T4+ expansion.** Three archetypes
  (`PhysicalProfiler` / `TechnicalPurist` / `RegionalExpert`) producing *disagreeing*
  reports on the same player. It ships only if a future feel-prototype shows
  disagreement creates interesting decisions rather than feeling like noise. The
  `ScoutArchetypeKind` enum carries the reserved archetype slots so the schema is
  stable across the future expansion; T3-5 constructs only `BasicScoutUncertainty`.

The feel-gate criterion, the 3-tester protocol, the fail-mode taxonomy
(RNG-fail / ignore-fail / overload-fail), and the one-remediation-pass ceiling are
preserved verbatim from `design/scout-disagreement.md` §Q1–Q4 for whoever runs the
Path-A prototype; they are out of scope for T3-5 and not reproduced here in full.

## Type contract

All types live in `fw-scouting`. `Q32` is `fw_core::Q32`. IDs are content-pack-qualified
`String`s per `Content/RULES.md §2`. `Serialize` + `Deserialize` on every type.

### `ScoutArchetypeKind`

`#[repr(u8)]` enum with explicit discriminants (canonical-state-crate enum discipline,
matching `EventClass` / `RoleFamily`). Reordering is a visible diff; discriminants are
locked.

```
0  BasicScoutUncertainty   // Path B — the T3-5 MVP archetype
1  PhysicalProfiler        // Path A (reserved) — overweights physical, misses mental
2  TechnicalPurist         // Path A (reserved) — overweights technical + mental
3  RegionalExpert          // Path A (reserved) — accurate in-region, noisy elsewhere
4  TempoReader             // T4+ expansion (reserved)
5  AcademySpotter          // T4+ expansion (reserved)
6  SetPieceSpecialist      // T4+ expansion (reserved)
```

### `CategoryBiases`

```
CategoryBiases { physical: Q32, mental: Q32, technical: Q32, narrative_flag: Q32 }
```
Category-level observation-bias weights. `narrative_flag` MUST be `0` — a
`validate()` / `try_new()` returns `Err` on any nonzero `narrative_flag`. For Path B
all four are `0` (neutral fog — Path B has no per-category bias; bias is Path-A
disagreement texture).

### `Scout`

The immutable archetype definition (track record lives in save-state, not here).

```
Scout {
    archetype_id: String,          // "fwh.core:scout.basic-uncertainty" — never mutates
    kind: ScoutArchetypeKind,
    display_name: String,          // player-facing — banned-terms-lint target
    ui_description: String,        // player-facing — banned-terms-lint target
    biases: CategoryBiases,        // all-zero for Path B
    familiar_regions: Vec<String>, // RegionalExpert (Path A) uses this; Path B leaves empty
    base_observation_noise: Q32,   // [0,1] — Path-B noise amplitude
    regional_noise_penalty: Q32,   // [0,1] — Path A only; Path B leaves 0
}
```

`fw-scouting` ships a `Scout::basic_uncertainty()` constructor returning the canonical
Path-B archetype with the tuning-seed values below.

### `GeneCategory`

```
enum GeneCategory { Physical, Mental, Technical }
```
Three variants only. There is deliberately **no `NarrativeFlag` variant** — narrative
flags are never scout-observable, and excluding the variant makes a narrative-flag
estimate compile-time-impossible.

### `GeneCategoryEstimate`

```
GeneCategoryEstimate { category: GeneCategory, low: Q32, high: Q32 }
```
The scout's estimated `[low, high]` band for a player's category-level score.
Invariant: `low <= high`, both clamped to `[0, 1]`. Consumed canonically by tests +
(default-OFF) the advanced numeric tooltip; the *default* surface renders its
`band()` text, never the raw numbers.

### `LabelEstimate`

```
LabelEstimate { label: PhenotypeLabelId, confidence: Q32 }
```
"This scout thinks this player carries this phenotype label, with this confidence."
`PhenotypeLabelId` is `fw_content`'s 46-variant enum. Confidence is `Q32 ∈ [0,1]`.

### `ScoutReport`

```
ScoutReport {
    scout_archetype_id: String,
    player_id: String,
    confidence: Q32,                          // [0,1] — overall confidence in this report
    label_estimates: Vec<LabelEstimate>,      // BTreeSet-order of the player's true labels
    category_estimates: Vec<GeneCategoryEstimate>,  // exactly 3 — Physical, Mental, Technical
    // prose + source_template_id are reserved for the deterministic-render layer
    // (a later narrative-director row); T3-5 ships the structured data only.
}
```

### `UncertaintyBand` — the "bands display as text, not numbers" deliverable

```
enum UncertaintyBand { Hunch, Tentative, Confident, Settled }
```
- `UncertaintyBand::from_confidence(c: Q32) -> UncertaintyBand` maps a confidence to a
  band (thresholds below).
- `UncertaintyBand::display_label(&self) -> &'static str` returns football-native
  scout text — exhaustive match, no wildcard arm (the `display_label` pattern from
  T2-7's `PhenotypeLabelId`):
  - `Hunch` → `"a hunch"`
  - `Tentative` → `"a tentative read"`
  - `Confident` → `"a confident read"`
  - `Settled` → `"a settled read"`
- `GeneCategoryEstimate::band(&self) -> UncertaintyBand` derives a band from the
  estimate's width: `width = high - low`; `effective_confidence = clamp(1 - width, 0, 1)`;
  `from_confidence(effective_confidence)`.

These labels are player-facing copy: football-native, banned-terms-clean, no
capitalised mystical state-nouns, no numbers.

## Path-B report generation

`observe_player(scout, player_bio, career_seed, observation_id) -> ScoutReport`,
where `scout.kind == BasicScoutUncertainty`. Deterministic + pure.

1. **Seed.** One `ChaCha8Rng` per call, seeded from
   `seed_fn(career_seed, observation_id, SeedLayer::ScoutObservation, 0)`. All draws
   for the report are pulled sequentially from this single stream (fixed draw order →
   deterministic).
2. **Category estimates** — for each `GeneCategory` C in declared order
   (Physical, Mental, Technical):
   - `true_mean(C)` = arithmetic mean of C's **quality gene fields** in
     `player_bio.internal_gene_snapshot`:
     - Physical = mean of the **6** quality genes (`height_ceiling`, `frame_density`,
       `fast_twitch_ratio`, `stamina_recovery`, `aging_curve`, `injury_resilience`).
       **Excludes `growth_curve`** — a signed `[-1, +1]` trajectory parameter that is
       dimensionally incoherent in a `[0, 1]` level estimate (a late-bloomer gene says
       nothing about current physical level).
     - Mental = mean of the **5** quality genes (`pattern_recognition`, `composure_floor`,
       `decision_velocity`, `learning_rate`, `ambition`).
       **Excludes `mentality`** — a signed `[-1, +1]` disposition parameter, likewise
       dimensionally incoherent in a `[0, 1]` level estimate.
     - Technical = mean of the **5** quality genes (`left_foot`, `aerial`, `dead_ball`,
       `striking`, `first_touch`). All fields are `[0, 1]`; no exclusion.
   - `noise` = a uniform draw in `[-base_observation_noise, +base_observation_noise]`.
   - `center` = `clamp(true_mean + noise, 0, 1)`.
   - `low` = `clamp(center - BAND_HALF_WIDTH, 0, 1)`,
     `high` = `clamp(center + BAND_HALF_WIDTH, 0, 1)`.
   - Emit `GeneCategoryEstimate { category: C, low, high }`.
3. **Label estimates** — for each true `PhenotypeLabelId` in
   `player_bio.scout_labels` (a `BTreeSet` — iterate in its sorted order):
   - `confidence` = a uniform draw in `[LABEL_CONFIDENCE_MIN, LABEL_CONFIDENCE_MAX]`.
   - Emit `LabelEstimate { label, confidence }`.
   Path-B MVP reports every *true* label (no false positives, no drops — those are
   Path-A disagreement texture). The uncertainty lives in the per-label confidence.
4. **Overall confidence** = arithmetic mean of all `label_estimates[].confidence`;
   if the player has no scout labels, fall back to `Q32` `0.5`.
5. Assemble + return the `ScoutReport`.

Path A's bias-filter (category visibility weighted by `scout.biases`) and regional
noise are not part of the Path-B generator and are not implemented at T3-5.

## T3 tuning seeds — NOT DECISIONS-locked

Per the archive's "`CategoryBiases` are Phase-3 tuning seeds, not SPEC-locked"
framing. These live as `Q32` constants in `fw-scouting`; revise freely during
balancing without a DECISIONS entry.

| Seed | Value | Meaning |
|---|---|---|
| `BASIC_SCOUT_OBSERVATION_NOISE` | `0.10` | Path-B category-estimate noise amplitude (± around the true mean). |
| `BASIC_SCOUT_BAND_HALF_WIDTH` | `0.12` | Half-width of a `GeneCategoryEstimate` `[low, high]` band. |
| `LABEL_CONFIDENCE_MIN` | `0.40` | Lower bound of the per-label confidence draw. |
| `LABEL_CONFIDENCE_MAX` | `0.95` | Upper bound of the per-label confidence draw. |
| `NO_LABEL_DEFAULT_CONFIDENCE` | `0.50` | Overall confidence fallback when a player has no scout labels. |
| `UNCERTAINTY_BAND_HUNCH_MAX` | `0.35` | `confidence < 0.35` → `Hunch`. |
| `UNCERTAINTY_BAND_TENTATIVE_MAX` | `0.60` | `[0.35, 0.60)` → `Tentative`. |
| `UNCERTAINTY_BAND_CONFIDENT_MAX` | `0.82` | `[0.60, 0.82)` → `Confident`; `>= 0.82` → `Settled`. |

Path-A archetype `CategoryBiases` (when Path A is built) are also tuning seeds; not
authored here.

## T3-5 MVP boundary

T3-5 ships, in `fw-scouting`:
- The full type contract above (Path-B-relevant + the reserved Path-A enum slots).
- `Scout::basic_uncertainty()` + the tuning-seed constants.
- The `observe_player` Path-B generator.
- `UncertaintyBand` + `from_confidence` + `display_label` + `GeneCategoryEstimate::band`.
- Tests: serde round-trip of the report shape; `observe_player` determinism (identical
  inputs → equal report; different `career_seed` → different report); `CategoryBiases`
  validator rejects nonzero `narrative_flag`; `UncertaintyBand` labels non-empty +
  unique + boundary cases.

## Deferred

- **Path A** — the 3-archetype disagreement model, the category bias-filter, regional
  noise. Conditional T4+ expansion.
- **Scout track record** — reliability accumulating from `ScoutReportConfirmed` /
  disagreement outcomes over seasons. Save-state; post-T3-5.
- **`MemoryEvent` emission** — the emitter that writes scout outcomes to the ledger.
  Career-loop / wiring-layer concern; `ScoutReport` is deliberately event-class-free.
- **Deterministic prose render** — `ScoutReport.prose` from Tracery templates. A later
  `narrative-director` row, following the T3-3 render-from-resolved-context pattern.
- **Scouting UI** — the report-display + scout-assignment screens. A later frontend row.
- **Advanced numeric tooltip** — power-user opt-in surfacing `GeneCategoryEstimate`
  bounds as numbers (ADR-0002 §Q3, default OFF). The bounds are canonical data now;
  no UI surfacing at T3-5.
- Regional scouting-network evolution, rival-club scout intelligence, scout
  hiring/firing — post-EA (archive `design/scout-disagreement.md` §Deferred).

---
description: Internal gene model + Identity Packet compiler. How bake-time generation produces specific, football-coherent players without surfacing a "gene" UI.
last_verified: 2026-05-18
status: Ported from Unity-era `/Users/vibelogic/dev/football-archive/design/player-generation.md` (315 LoC, locked 2026-04-24). T2-4 + T2-7 + T3-5 unblocked by this port. Unity references reconciled to Rust + Tauri + SolidJS; locked design decisions preserved verbatim; phase numbering aligned to MASTER_PLAN T-N.
---

# Player Generation — internal model + Identity Packet

## Purpose

Answer "how do we generate ~2000-2400 players at bake time that feel specifically authored rather than randomly rolled, while keeping the generation deterministic + version-migration-safe + modding-ready, AND never exposing a 'gene' UI to the player?"

## Locked decisions

Carry-forward from FW v1 SPEC.md 2026-04-22 + 2026-04-24 resolution; reconciled to FW v2 `docs/DECISIONS.md` + `docs/MASTER_PLAN.md` conventions at this port. Summary:

- **Internal gene model is invisible.** No "genetics" terminology in player-facing UI. Phenotype labels only ("Late Bloomer", "Composed Under Pressure", "Explosive First Step", "Set-Piece Natural"). Numeric details exposed only in advanced scout-report tooltips for power users (default OFF).
- **Identity Packet** is the stable output of generation: playing instincts + pressure response + development hooks + signature candidates + scout labels + commentary handles + rivalry compatibility.
- **Stable IDs content-pack-qualified.** Regenerating a content pack with refined prompts must NOT change existing player IDs; deltas ship as patch content packs. Per `Content/RULES.md §2` the canonical ID form is `fwh.core:player_00042` (5-digit zero-padded; default procedural form) — hand-authored IDs use the dotted form per the §2 carve-out.
- **Lineage data seeded at bootstrap; surfacing deferred post-MVP.** The Coaching Lineage system (tactical inheritance) stores data in alumni fields from T3+; exposes post-EA.

## Internal gene model (LOCKED — 22 fields, 4 categories)

NEVER surfaced as "genes." Internally used by the Identity Packet compiler + Scout Disagreement system. Growth past 22 requires a schema bump, not organic drift.

All numeric fields are `Q32` (`fixed::FixedI64<U32>`) per `Sim/RULES.md §1` — `f32`/`f64` banned in canonical state. Range notation `0.0-1.0` corresponds to `Q32::ZERO..=Q32::ONE`; `-1.0-+1.0` corresponds to `Q32::from_int(-1)..=Q32::ONE`.

### A. Physical profile (7 fields)

| Field | Range | Scout-visible signal |
|---|---|---|
| `height_ceiling` | 0.0–1.0 | observable directly |
| `frame_density` | 0.0–1.0 | observable (lean / athletic / strong) |
| `fast_twitch_ratio` | 0.0–1.0 | pace-vs-stamina tradeoff signal |
| `stamina_recovery` | 0.0–1.0 | late-match performance signal |
| `growth_curve` | -1.0–+1.0 | early-peak vs late-bloomer — scouts guess |
| `aging_curve` | 0.0–1.0 | longevity — scouts guess |
| `injury_resilience` | 0.0–1.0 | history-observable over seasons |

### B. Mental profile (6 fields)

| Field | Range | Scout-visible signal |
|---|---|---|
| `pattern_recognition` | 0.0–1.0 | reading-the-game observable |
| `composure_floor` | 0.0–1.0 | pressure-observed |
| `decision_velocity` | 0.0–1.0 | time-to-decision observable |
| `learning_rate` | 0.0–1.0 | improvement-over-time observable |
| `ambition` | 0.0–1.0 | training-willingness + loyalty tradeoff |
| `mentality` | -1.0–+1.0 | introvert-grinder vs extrovert-charisma |

### C. Technical affinities (5 fields)

| Field | Weight | Description |
|---|---|---|
| `left_foot` | 0.0–1.0 | preferred left-foot work |
| `aerial` | 0.0–1.0 | header / high-ball affinity |
| `dead_ball` | 0.0–1.0 | free-kick + penalty affinity |
| `striking` | 0.0–1.0 | power-shot + volley affinity |
| `first_touch` | 0.0–1.0 | control + trap affinity |

### D. Narrative trigger flags (4 fields — NEVER called "Soul genes")

| Field | Rarity | Unlock condition |
|---|---|---|
| `flow_access` | ~1.5% carriers | Sustained-readiness states in high-stakes matches, unlocked by specific qualifying match |
| `peak_ceiling_high` | ~0.5% | Cap on peak-state expression; raised by first qualifying high-stakes career event |
| `late_bloomer` | ~variable per cohort | Dormant until specific career event (relegation 6-pointer decisive goal, cup-run moment, etc.) |
| `awakening_dormant` | ~0.2% | Very-late-career explosive trait activation (post-25 specifically) |

These are flags, not "genes." Never surfaced as capitalized mystical terms. Post-match report might say "Something clicked for him today" — the flag framing is architectural, not lexical.

## Identity Packet (the stable output)

Every player has exactly one Identity Packet, stored as content-pack-qualified RON in `content/baked/players/<id>.ron`. Rust shape sketch (final shape lands at T2-4):

```rust
// crates/fw-content/src/player_bio.rs (new file at T2-4)
//
// `PlayerBio` (renamed from FW-v1 IdentityPacket per project naming
// convention — "Bio" reads more football-native than "Identity Packet" in
// dev surfaces; player-facing UI never sees either term, only phenotype
// labels per the visibility rules below).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerBio {
    pub player_id: String,                 // `fwh.core:player_00042` per Content/RULES.md §2
    pub schema_version: u16,                // load-time migration framework, per fw-save
    pub content_pack_version: String,       // `1.0.0`, `1.1.0`, etc.

    pub display_name_full: String,
    pub display_name_short: String,
    pub role_family: RoleFamily,            // existing enum in fw-content::signature
    pub birth_region: String,               // for commentary + lineage flavor

    pub playing_instincts: PlayingInstincts,
    pub pressure_response: PressureResponse,
    pub development_hooks: Vec<DevelopmentHook>,
    pub signature_candidates: Vec<SignatureCandidate>,  // existing type in fw-content
    pub scout_labels: BTreeSet<PhenotypeLabelId>,       // BTreeSet per Sim/RULES.md §2
    pub commentary_handles: CommentaryHandles,
    pub rivalry_compatibility: BTreeMap<String, Q32>,   // archetype_id → salience_multiplier
    pub alumni_of: Vec<ClubId>,             // for Coaching Lineage data seeding (T3+ surfacing)
    pub tactical_dna_fragments: Vec<TacticalDnaFragment>,  // data-only at T3; surfacing post-EA

    /// INTERNAL ONLY. Never serialized to any UI surface. Even the advanced
    /// scout-tooltip exposes scout-ESTIMATED ranges, not the true snapshot.
    /// Debug/dev builds may surface this behind a build-time flag; shipped
    /// builds never do.
    pub internal_gene_snapshot: GeneSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayingInstincts {
    pub defensive_shape_preference: DefensiveShape,    // enum: Compact / SpreadHigh / Aggressive / Sit-and-Counter
    pub attacking_run_preference: AttackingRun,        // enum: Channels / In-Behind / Drift-Wide / Drop-Deep
    pub pressing_trigger: PressingTrigger,             // enum: Aggressive / Reactive / Conservative
    pub risk_appetite: Q32,                            // 0.0–1.0
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PressureResponse {
    /// Stakes → performance curve points. Sampled at career match-stake events.
    pub stakes_to_performance_curve: Vec<CurvePoint>,
    pub composure_floor: Q32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DevelopmentHook {
    MinutesInRole { role: RoleFamily, threshold_minutes: u32, readiness_target_field: ReadinessField },
    EventCount { event_class: MemoryEventClass, threshold: u32, readiness_target_field: ReadinessField },
    NarrativeFlag { flag: NarrativeFlag, unlock_conditions: Vec<UnlockCondition> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentaryHandles {
    pub preferred_nouns: Vec<String>,       // "the striker", "the lad from <region>", ...
    pub preferred_verbs: Vec<String>,       // "drives", "arrives", "glides", ...
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneSnapshot {
    pub physical: PhysicalGenes,            // 7 Q32 fields per §A above
    pub mental: MentalGenes,                // 6 Q32 fields per §B above
    pub technical: TechnicalAffinities,     // 5 Q32 fields per §C above
    pub narrative_flags: BTreeSet<NarrativeFlag>,  // BTreeSet of activated flags from §D
}
```

(The above is a sketch. T2-4 ships the binding `PlayerBio` + supporting types; the exact field set above is the design contract — adding a 23rd gene field is a schema bump per §A.)

## Bake-time content compiler pipeline

Generation is reproducible by artifact, not by assuming the LLM is bit-deterministic. Optional prompt + seed + frozen model version produces a candidate name bank; once reviewed, that name bank is checked in as structured RON (per `Content/RULES.md §7` content/baked/ is gitignored — actually committed via `just bake-content`). The checked-in RON is the source of truth. If regeneration differs, the compiler treats it as a new delta pack, not an in-place mutation.

Pipeline (executed by `crates/fw-content-baker/`):

1. **Specify cohort** (e.g., "20 clubs in fantasy-second-tier region, squad size 22–26, 60% native, 30% cross-region, 10% foreign")
2. **Generate gene distributions** from regional / cultural priors via `ChaCha8Rng::seed_from_u64(seed_fn(career_seed, cohort_idx, SeedLayer::ContentBake, site))` per `ADR-0009`
3. **Optionally generate candidate names** via Claude API at bake time (`fw-content-baker bake-names` per T2-3) with seed prefix + frozen model version recorded; review + commit the name bank RON
4. **Compile PlayerBios** from gene snapshots + checked-in name bank into `content/baked/players/<id>.ron`
5. **Validate** (per T2-3's validator-as-one-class pattern):
   - schema correctness (`PlayerBioValidator`)
   - no duplicate names (lint — extends `scripts/lint-banned-terms.py`)
   - no licensed-data collisions (extends `check_licensed_data`)
   - style consistency — player-age matches career-history plausibility
6. **Sim sanity check** — load packet into MatchSim, play 10-match harness, confirm no crashes / degenerate behavior (existing harness in `crates/fw-match-sim/tests/`)
7. **Commit to content pack** with bumped `pack_version` if delta
8. **Manifest record** — `content-pack-manifest.json` per T2-3's `BakeManifest` shape, extended with the player-bio fields

See `docs/CONTENT_PIPELINE.md` for the wider bake-time architecture, `personality-bias-weights.md` for the personality-vector tuning, and `xg-coefficients.md` for the calibration cadence.

## UI surfacing rules

**Player card shows:**
- Name, age, position, kit number
- Phenotype scout labels (from `PlayerBio.scout_labels`)
- Active signatures (football-copy names, e.g., "Looks for early crosses")
- Contract + wage
- Development progress bar (opaque — no gene reveal)

**Scout report shows:**
- Per-scout phenotype labels with confidence
- Projected range (wide early; narrows over time)
- Narrative prose ("A late bloomer who reads the game well but struggles under physical duress")
- Advanced tooltip (power users, gated behind settings toggle): per-field gene estimates with uncertainty

**Never surfaces:**
- "Genes" / "genetics" / "chromosomes" / "bloodline" etc. (per `docs/design/ui-vocabulary.md` banned-terms catalog)
- Capitalized mystical trigger-flag names
- Raw numeric gene values in default UI

## MVP boundary

At T2-4: **hand-authored compiler-shaped `PlayerBio` fixtures** sufficient to exercise scout-prose templates + signature-affinity reads. 22 fixtures minimum — one per player on a representative roster. Round-trip through the validator + `scripts/fw verify` green. These are NOT compiler output; they're compiler-shaped RON the compiler will eventually produce.

At T4+ (full content compile): full pack v1 compiled (~2000–2400 players across the season's clubs) via the real bake-time pipeline; all pipeline lints green; sim sanity check on 10K-match sweep.

## Deferred

- Lineage surfacing (Coaching Lineage, "son of [player]" flags in scout reports) — post-MVP
- Player-authored custom players via Workshop content packs — post-EA
- Per-player generated portraits (e.g. GPT Image 2 at bake time) — Phase 6 or 7 polish
- 3D player model generation — ruled out in `docs/DESIGN_DOC.md` §3 (text-first presentation, no 3D)

## Resolved (carry-forward from 2026-04-24 SPEC.md resolution)

### Q1 — Internal gene model size

**Locked at 22 fields across 4 categories:** 7 physical + 6 mental + 5 technical + 4 narrative-flag. Growth requires a schema bump, not organic drift.

### Q2 — Phenotype label catalog (46 locked for MVP, ceiling 50)

Stable content-pack-qualified enum. Labels are filter keys for scouts + commentary + UI surfacing. **No stigmatizing framing; no systemic/mechanics vocabulary; PEGI 12 safe** per `docs/DESIGN_DOC.md §1`.

**Physical (7):**
`Explosive First Step`, `Relentless Engine`, `Aerial Presence`, `Agile Pivot`, `Slow Starter`, `Late-Career Peak`, `Quick Recovery`

**Mental (8):**
`Reads the Game`, `Composed Under Pressure`, `Decisive in the Box`, `Struggles Under Scrutiny`, `Slow to Adapt`, `Grows Into Games`, `Ambitious`, `Loyal`

**Technical (6):**
`Set-Piece Natural`, `Strong Left Foot`, `Pure Finisher`, `Silken First Touch`, `Powerful Ball Striker`, `Aerial Threat`

**Development (3):**
`Late Bloomer`, `Early Developer`, `Steady Progressor`

**Role-specific (22):**

| Role family | Labels |
|---|---|
| Goalkeeper | `Sweeper Keeper`, `Line Keeper`, `Cross Claimer` |
| Centre-back | `Ball-Playing Defender`, `Stopper`, `Cover Defender` |
| Full-back / wing-back | `Overlapping Full-Back`, `Inverted Full-Back`, `Wing-Back Runner` |
| Defensive midfielder | `Anchor Man`, `Ball-Winning Midfielder`, `Pressing Midfielder` |
| Central midfielder | `Tempo Setter`, `Box-to-Box` |
| Attacking midfielder / #10 | `Playmaker`, `Half-Space Creator` |
| Winger | `Inverted Winger`, `Traditional Winger` |
| Striker / centre-forward | `Poacher`, `Target Man`, `False 9`, `Link Forward` |

**Total: 46.** Headroom of 4 before the 50 ceiling.

<!-- ui-lint:ignore-start reason="phenotype-edits resolution naming the banned / renamed labels" -->
**Explicit exclusions (edits applied in the 2026-04-24 resolution):**
- `Fragile Under Scrutiny` → renamed **`Struggles Under Scrutiny`** (less stigmatizing, more football-observable).
- `Powerful Striker` → renamed **`Powerful Ball Striker`** (avoids confusion with striker-as-position).
- `Plateau Risk` **removed from the player-facing enum entirely.** Ceiling-visibility concepts surface through scout prose + projected-range narrowing, not a systemic label.
- `Injury-Prone` not in catalog; injury history surfaces through explicit record on player card (real events, not a prejudicial tag).
<!-- ui-lint:ignore-end -->

**No label may reference real-world ethnicity, religion, politics, or mental-health language.** Football-observable traits only. Growth past 50 labels triggers schema review.

### Q3 — Advanced tooltip numeric exposure

**Default OFF. Power-user opt-in via settings toggle** ("Show advanced scout-report details"). When enabled, the advanced tooltip surfaces **scout-estimated ranges with uncertainty bars** — never the true `internal_gene_snapshot` values. Uncertainty ranges narrow as the scout observes the player more over seasons.

This is advanced scouting detail, **not debug mode.** True internal values never appear in any shipped UI surface under any settings combination. Debug/dev builds may expose raw snapshots behind a build-time flag; shipped builds do not.

### Q4 — Compiler reproducibility across model versions

**Canonical artifact is the checked-in RON in the content pack**, NOT the prompt + seed + model combination. The compiler pipeline records the exact model version + seed used to generate the pack as `BakeManifest` metadata (auditability; T2-3 shape). Regenerating with a different model version produces a **new delta pack**, never an in-place mutation.

```json
content/baked/manifest.json
{
  "pack_id": "fwh.core",
  "pack_version": "1.0.0",
  "generated_at": "2026-XX-XX",
  "generator": {
    "model": "claude-opus-4.7-20XXXXXX",
    "seed_prefix": "fwh_core_v1_seed_0x...",
    "prompt_hash": "blake3:..."
  }
}
```

### Q5 — Content pack delta strategy (ID stability)

**Additive-only delta packs. Stable IDs never mutate. Pack version does NOT leak into entity IDs.**

Player IDs use the form `fwh.core:player_00042` per `Content/RULES.md §2` (or `fwh.core.v1:player_00042` if a major-version namespace is preserved for hypothetical future rebuilds). **`v1.1`, `v1.2`, etc. never appear in an entity ID.** Pack-minor-version lives in the manifest as `introduced_in_pack_version: "1.1.0"` per-entity, not in the ID itself.

**Rules:**
- Every `ContentPackQualifiedId` is stable forever once shipped.
- Delta pack `v1.1.0` introduces new players at fresh sequential IDs (`fwh.core:player_00043`, `fwh.core:player_00044`...). Each entity records `introduced_in_pack_version` for auditability.
- **Never rename an ID.** Renames require a deprecation flag + new ID at the next schema bump.
- Save-file resolver: reference by ID; if an ID is unresolved (save built against a newer pack than the user has installed), fall back to the base pack's placeholder-player generator with a "missing content" UI badge. Save is not blocked.
- `PlayerBio` schema bumps (vs pack version bumps) run through the load-time migration path in `fw-save` per T3-1's V0→V1→V2 chain.

---

## Affinity-count distribution (authoritative — cross-ref from `docs/design/personality-bias-weights.md`)

Each player's `signature_candidates` count (`k ∈ {0, 1, 2, 3}`) is cohort-weighted.

**T-phase tuning seeds (NOT DECISIONS.md-locked):**

| Cohort | P(0) | P(1) | P(2) | P(3) |
|---|---|---|---|---|
| Top-flight starter | 0.02 | 0.60 | 0.32 | 0.06 |
| Mid-tier starter | 0.08 | 0.62 | 0.25 | 0.05 |
| Lower-tier / depth / journeyman | 0.20 | 0.60 | 0.18 | 0.02 |

Overall population P(0) ≈ 0.10. 3-affinity players remain rare across every tier — these are the save-defining characters.

**Roll procedure:**
1. Cohort assignment from club tier + squad-depth role + age bracket (authored in the compiler's cohort-spec).
2. Roll `affinity_count` against the cohort row.
3. If `affinity_count > 0`: roll which specific signature candidates, weighted by internal-gene-model alignment (e.g., `fast_twitch_ratio > 0.7` biases toward winger/striker signatures).
4. Store in `PlayerBio.signature_candidates` with `affinity_weight ∈ [0,1]`. Never more than 3.

## Gene-category visibility for Scout Disagreement (T3-5)

Scouts' `biases: { gene_category: weight }` map to the 4 internal categories at **category level only** — no per-field biases at MVP. Per-field biases are tuning debt; the 3-archetype prototype is sufficient with category-level.

**Default archetype mappings (T3-5 tuning seeds, not DECISIONS.md-locked):**

| Archetype | physical | mental | technical | narrative_flag |
|---|---|---|---|---|
| `physical_profiler` | 1.0 | 0.3 | 0.4 | 0.0 |
| `technical_purist` | 0.3 | 0.8 | 1.0 | 0.0 |
| `regional_expert` | cohort-dependent (neutral in-region, noisy out-of-region) | | | 0.0 |

**Narrative-flag category (`flow_access`, `late_bloomer`, etc.) is never directly observable by any scout.** Zero weight across all archetypes. Narrative flags only ever surface via the events that trigger them — generating a retroactive `Late Bloomer` phenotype label once the unlock conditions have been met. Intentional: narrative flags are the "something you couldn't have known" layer.

## Regional priors integration

Step 2 of the bake-time pipeline ("generate gene distributions from regional / cultural priors") consumes `RegionPriors` objects from the worldbuilding layer (deferred; T4+ scope). Each region's `physical_priors` / `mental_priors` / `technical_priors` biases the roll **additively** — never replaces the base roll.

A Northern-region player can still be a technical wizard; just less likely. Cultural-flavor priors (`dominant_role_families`, `stylistic_tendencies`) additionally bias role-family assignment + signature-candidate selection.

## Prototype gate

- **T2-4 (this row):** Hand-authored `PlayerBio` fixtures — 22 minimum, one per slot on a representative roster. Round-trip through `PlayerBioValidator` (extends T2-3's validator-as-one-class pattern). Manual review confirms each reads as a specific coherent footballer, not generic.
- **T3-5:** Scout Disagreement feel prototype uses 10 varied `PlayerBio`s to demonstrate scout-bias divergence reads clearly.
- **T4+:** Bake-time compiler produces full cohort of ~2000–2400 players. Balance harness 10K-match sweep confirms no degenerate archetype dominates or bricks.

---

## Port notes (FW v1 → FW v2)

- **`IdentityPacket` → `PlayerBio`** — naming aligned with the project's "football-native vocabulary" discipline (`CLAUDE.md §7`). Dev surfaces only; player UI never sees either term.
- **ScriptableObject → RON file** — content/baked/players/<id>.ron per `Content/RULES.md §1`.
- **Phase 2/3/4 numbering → T2/T3/T4** — aligned with current MASTER_PLAN. T2-4 row was blocked on this doc's authorship; this port unblocks it + the transitive T2-7 Squad page + T3-5 Scout Disagreement.
- **f32 → Q32** — all gene fields use `fixed::FixedI64<U32>` per `Sim/RULES.md §1`. Bake-time gene rolls go through `ChaCha8Rng::seed_from_u64(seed_fn(career_seed, player_idx, SeedLayer::ContentBake, 0))` per `ADR-0009`.
- **HashMap → BTreeMap** — `rivalry_compatibility` + `tactical_dna_fragments` use `BTreeMap` per `Sim/RULES.md §2`.
- **SPEC.md / DECISIONS.md references** — Q1–Q5 above were originally in FW v1's SPEC.md decisions log; reconciled to "carry-forward from 2026-04-24 SPEC.md resolution" in this v2 port. If any Q1–Q5 decision needs to change, append a new `docs/DECISIONS.md` bullet citing this port verbatim per `design-docs/RULES.md §2`.

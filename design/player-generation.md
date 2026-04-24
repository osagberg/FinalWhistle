---
description: Internal gene model + Identity Packet compiler. How bake-time generation produces specific, football-coherent players without surfacing a "gene" UI.
last_verified: 2026-04-24
status: Phase 0 open questions resolved; 22-field model locked, 46-label phenotype catalog authored, affinity P(k) tables materialized, ID strategy corrected to not encode minor pack versions. One Phase-2 ADR pre-seeded.
---

# Player Generation — internal model + Identity Packet

## Purpose

Answer "how do we generate ~2000-2400 players at bake time that feel specifically authored rather than randomly rolled, while keeping the generation deterministic + version-migration-safe + modding-ready, AND never exposing a 'gene' UI to the player?"

## Locked decisions

See SPEC.md 2026-04-22. Summary:

- **Internal gene model is invisible.** No "genetics" terminology in player-facing UI. Phenotype labels only ("Late Bloomer", "Composed Under Pressure", "Explosive First Step", "Set-Piece Natural"). Numeric details exposed only in advanced scout-report tooltips for power users.
- **Identity Packet** is the stable output of generation: playing instincts + pressure response + development hooks + signature candidates + scout labels + commentary handles + rivalry compatibility.
- **Stable IDs content-pack-qualified.** Regenerating a content pack with refined prompts must NOT change existing player IDs; deltas ship as patch content packs.
- **Lineage data seeded at bootstrap; surfacing deferred post-MVP.** The Coaching Lineage system (tactical inheritance) stores data in alumni fields from Phase 3; exposes post-EA.

## Internal gene model (draft — Phase 2 lock)

Four categories, ~22 fields total. NEVER surfaced as "genes." Internally used by the Identity Packet compiler + Scout Disagreement system.

### A. Physical profile (7 fields)

| Field | Range | Scout-visible signal |
|---|---|---|
| `height_ceiling` | 0.0-1.0 | observable directly |
| `frame_density` | 0.0-1.0 | observable (lean / athletic / strong) |
| `fast_twitch_ratio` | 0.0-1.0 | pace-vs-stamina tradeoff signal |
| `stamina_recovery` | 0.0-1.0 | late-match performance signal |
| `growth_curve` | -1.0-+1.0 | early-peak vs late-bloomer — scouts guess |
| `aging_curve` | 0.0-1.0 | longevity — scouts guess |
| `injury_resilience` | 0.0-1.0 | history-observable over seasons |

### B. Mental profile (6 fields)

| Field | Range | Scout-visible signal |
|---|---|---|
| `pattern_recognition` | 0.0-1.0 | reading-the-game observable |
| `composure_floor` | 0.0-1.0 | pressure-observed |
| `decision_velocity` | 0.0-1.0 | time-to-decision observable |
| `learning_rate` | 0.0-1.0 | improvement-over-time observable |
| `ambition` | 0.0-1.0 | training-willingness + loyalty tradeoff |
| `mentality` | -1.0-+1.0 | introvert-grinder vs extrovert-charisma |

### C. Technical affinities (5 fields)

| Field | Weight | Description |
|---|---|---|
| `left_foot` | 0.0-1.0 | preferred left-foot work |
| `aerial` | 0.0-1.0 | header / high-ball affinity |
| `dead_ball` | 0.0-1.0 | free-kick + penalty affinity |
| `striking` | 0.0-1.0 | power-shot + volley affinity |
| `first_touch` | 0.0-1.0 | control + trap affinity |

### D. Narrative trigger flags (4 fields — NEVER called "Soul genes")

| Field | Rarity | Unlock condition |
|---|---|---|
| `flow_access` | ~1.5% carriers | Sustained-readiness states in high-stakes matches, unlocked by specific qualifying match |
| `peak_ceiling_high` | ~0.5% | Cap on peak-state expression; raised by first qualifying high-stakes career event |
| `late_bloomer` | ~variable per cohort | Dormant until specific career event (relegation 6-pointer decisive goal, cup-run moment, etc.) |
| `awakening_dormant` | ~0.2% | Very-late-career explosive trait activation (post-25 specifically) |

These are flags, not "genes." Never surfaced as capitalized mystical terms. Post-match report might say "Something clicked for him today" — the flag framing is architectural, not lexical.

## Identity Packet (the stable output)

Every player has exactly one Identity Packet, stored as content-pack-qualified ScriptableObject:

```
IdentityPacket {
    player_id: "fwh.core.v1:player_00042"
    display_name_full: "Marco Halsey"
    display_name_short: "Halsey"
    role_family: enum
    playing_instincts: {
        defensive_shape_preference: enum
        attacking_run_preference: enum
        pressing_trigger: enum
        risk_appetite: f32
    }
    pressure_response: {
        stakes_to_performance_curve: [curve points]
        composure_floor: f32
    }
    development_hooks: [
        { trigger_kind: "minutes_in_role", role, threshold, readiness_target_field }
        { trigger_kind: "event_count", event_class, threshold, readiness_target_field }
        { trigger_kind: "narrative_flag", flag_name, unlock_conditions }
    ]
    signature_candidates: [
        { signature_id, affinity_weight }
        // typically 1-3 per player
    ]
    scout_labels: [  // these are the phenotype labels that EVER appear in UI
        "Late Bloomer",
        "Explosive First Step",
        "Set-Piece Natural"
    ]
    commentary_handles: {
        preferred_nouns: [string]  // "the striker", "the lad from [region]", ...
        preferred_verbs: [string]  // "drives", "arrives", "glides", ...
    }
    rivalry_compatibility: [
        { archetype_id, salience_multiplier }
    ]
    birth_region: string  // for commentary + lineage flavor
    alumni_of: [club_id]  // for Coaching Lineage data seeding
    tactical_dna_fragments: [  // for Coaching Lineage (data only; surfacing deferred)
        { doctrine_id, fragment_weight }
    ]
    internal_gene_snapshot: {  // INTERNAL only; never rendered to UI
        physical: { ... }
        mental: { ... }
        technical: { ... }
        narrative_flags: [ ... ]
    }
    schema_version: u16
    content_pack_version: string
}
```

## AI Content Compiler pipeline (for bake-time generation)

Generation is reproducible by artifact, not by assuming the LLM is bit-deterministic. Prompt + seed + frozen model version produce the draft; the checked-in structured JSON/content pack is the source of truth. If regeneration differs, the compiler treats it as a new delta pack, not an in-place mutation. Pipeline:

```
1. Specify cohort (e.g., "96 clubs in East-Midlands-analog region, squad size 22-26, 60% native, 30% cross-region, 10% foreign")
2. Generate gene distributions from regional / cultural priors (seeded RNG)
3. Generate names via LLM with seed prefix + frozen model version recorded
4. Compile Identity Packets from gene snapshots
5. Validate:
   - schema correctness
   - no duplicate names (lint)
   - no legal-sensitive names (lint: real-player name database check)
   - style consistency (lint: player-age matches career-history plausibility)
6. Sim sanity check: load packet into MatchSim, play 10-match harness, confirm no crashes / degenerate behavior
7. Commit to content pack with bumped pack_version if delta
8. Import to Unity as SOs
```

See `design/worldbuilding.md` for cohort + cultural priors, and `TECH_APPROACH.md §4` for pipeline architecture.

## UI surfacing rules

**Player card shows:**
- Name, age, position, kit number
- Phenotype scout labels (from Identity Packet `scout_labels`)
- Active signatures (football-copy names, e.g., "Looks for early crosses")
- Contract + wage
- Development progress bar (opaque — no gene reveal)

**Scout report shows:**
- Per-scout phenotype labels with confidence
- Projected range (wide early; narrows over time)
- Narrative prose ("A late bloomer who reads the game well but struggles under physical duress")
- Advanced tooltip (power users, gated behind settings toggle): per-field gene estimates with uncertainty

**Never surfaces:**
- "Genes" / "genetics" / "chromosomes" / "bloodline" etc.
- Capitalized mystical trigger-flag names
- Raw numeric gene values in default UI

## MVP boundary

At Month 3 slice: Identity Packets for 22 players hand-compiled or compiled via AI Content Compiler with user-review step. Compiler pipeline demonstrated end-to-end on at least 22 players.

At Month 12 EA: full content pack v1 compiled (~2000-2400 players across 96 clubs); all pipeline lints green; sim sanity check on 10K-match sweep.

## Deferred

- Lineage surfacing (Coaching Lineage, "son of [player]" flags in scout reports) — post-MVP
- Player-authored custom players via Workshop content packs — post-EA
- Per-player generated portraits (GPT Image 2 at bake time) — Phase 6 or 7 polish
- 3D player models — deferred indefinitely

## Resolved (2026-04-24)

See SPEC.md decisions log entry `2026-04-24 — Player-generation open questions resolved`. One Phase-2 ADR pre-seeded.

### Q1 — Internal gene model size

**Locked at 22 fields across 4 categories:** 7 physical + 6 mental + 5 technical + 4 narrative-flag. Growth requires a schema bump, not organic drift.

### Q2 — Phenotype label catalog (46 locked for MVP, ceiling 50)

Stable content-pack-qualified enum. Labels are filter keys for scouts + commentary + UI surfacing. **No stigmatizing framing; no systemic/mechanics vocabulary; PEGI-safe.**

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

**Explicit exclusions (edits applied in this resolution):**
- `Fragile Under Scrutiny` → renamed **`Struggles Under Scrutiny`** (less stigmatizing, more football-observable).
- `Powerful Striker` → renamed **`Powerful Ball Striker`** (avoids confusion with striker-as-position).
- `Plateau Risk` **removed from the player-facing enum entirely.** Ceiling-visibility concepts surface through scout prose + projected-range narrowing, not a systemic label.
- `Injury-Prone` not in catalog; injury history surfaces through explicit record on player card (real events, not a prejudicial tag).

**No label may reference real-world ethnicity, religion, politics, or mental-health language.** Football-observable traits only. Growth past 50 labels triggers schema review — same ceiling discipline as event-class catalog.

### Q3 — Advanced tooltip numeric exposure

**Default OFF. Power-user opt-in via settings toggle** ("Show advanced scout-report details"). When enabled, the advanced tooltip surfaces **scout-estimated ranges with uncertainty bars** — never the true `internal_gene_snapshot` values. Uncertainty ranges narrow as the scout observes the player more over seasons.

This is advanced scouting detail, **not debug mode.** True internal values never appear in any shipped UI surface under any settings combination. Debug/dev-only builds may expose raw snapshots behind a build-time flag; shipped builds do not.

### Q4 — Compiler reproducibility across model versions

**Canonical artifact is the checked-in structured JSON in the content pack**, NOT the prompt + seed + model combination. The compiler pipeline records the exact model version + seed used to generate the pack as manifest metadata (auditability); regenerating with a different model version produces a **new delta pack**, never an in-place mutation.

```
content-pack-manifest.json
{
  "pack_id": "fwh.core",
  "pack_version": "1.0.0",
  "generated_at": "2026-XX-XX",
  "generator": {
    "model": "claude-opus-4.7-20XXXXXX",
    "seed_prefix": "fwh_core_v1_seed_0x...",
    "prompt_hash": "sha256:..."
  }
}
```

### Q5 — Content pack delta strategy (ID stability corrected)

**Additive-only delta packs. Stable IDs never mutate. Pack version does NOT leak into entity IDs.**

Player IDs use the form `fwh.core:player_00042` (or `fwh.core.v1:player_00042` if a major-version namespace is preserved for hypothetical future rebuilds). **`v1.1`, `v1.2`, etc. never appear in an entity ID.** Pack-minor-version lives in the **manifest** as `introduced_in_pack_version: "1.1.0"` per-entity, not in the ID itself. Otherwise every patch leaks into save references and mod compatibility.

**Rules:**
- Every `ContentPackQualifiedId` is stable forever once shipped.
- Delta pack `v1.1.0` introduces new players at fresh sequential IDs (`fwh.core:player_00043`, `fwh.core:player_00044`...). Each entity records `introduced_in_pack_version` in the manifest for auditability.
- **Never rename an ID.** Renames require a deprecation flag + new ID at the next schema bump.
- Save-file resolver: reference by ID; if an ID is unresolved (save built against a newer pack than the user has installed), fall back to the base pack's placeholder-player generator with a "missing content" UI badge. Save is not blocked.
- `IdentityPacket` schema bumps (vs. pack version bumps) run through the load-time migration path in `event-sourced-memory.md`.

---

## Affinity-count distribution (authoritative — cross-ref from `design/signatures.md`)

Each player's `signature_candidates` count (`k ∈ {0, 1, 2, 3}`) is cohort-weighted. Signatures.md surfaces a summary; this table is the source of truth.

**Phase-6 tuning seeds (NOT SPEC-locked):**

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
4. Store in `IdentityPacket.signature_candidates` with `affinity_weight ∈ [0,1]`. Never more than 3.

## Gene-category visibility for Scout Disagreement (cross-ref `design/scout-disagreement.md`)

Scouts' `biases: { gene_category: weight }` map to the 4 internal categories at **category level only** — no per-field biases at MVP. Per-field biases are tuning debt; the 3-archetype prototype is sufficient with category-level.

**Default archetype mappings (Phase-3 tuning seeds, not SPEC-locked):**

| Archetype | physical | mental | technical | narrative_flag |
|---|---|---|---|---|
| `physical_profiler` | 1.0 | 0.3 | 0.4 | 0.0 |
| `technical_purist` | 0.3 | 0.8 | 1.0 | 0.0 |
| `regional_expert` | cohort-dependent (neutral in-region, noisy out-of-region) | | | 0.0 |

**Narrative-flag category (`flow_access`, `late_bloomer`, etc.) is never directly observable by any scout.** Zero weight across all archetypes. Narrative flags only ever surface via the events that trigger them — generating a retroactive `Late Bloomer` phenotype label once the unlock conditions have been met. Intentional: narrative flags are the "something you couldn't have known" layer.

## Regional priors integration (cross-ref `design/worldbuilding.md`)

Step 2 of the AI Content Compiler pipeline ("generate gene distributions from regional / cultural priors") consumes `RegionPriors` objects from `worldbuilding.md`. Each region's `physical_priors` / `mental_priors` / `technical_priors` biases the roll **additively** — never replaces the base roll.

A Northern-region player can still be a technical wizard; just less likely. Cultural-flavor priors (`dominant_role_families`, `stylistic_tendencies`) additionally bias role-family assignment + signature-candidate selection.

## Prototype gate

**Phase 3:** AI Content Compiler produces 22 valid Identity Packets for Month-3 slice. Manual review confirms each reads as a specific coherent footballer, not generic.

**Phase 4:** Scout Disagreement feel prototype uses 10 varied Identity Packets to demonstrate scout-bias divergence reads clearly.

**Phase 6:** Full cohort of ~2400 players compiled. Balance harness 10K-match sweep confirms no degenerate archetype dominates or bricks.

---
description: Internal gene model + Identity Packet compiler. How bake-time generation produces specific, football-coherent players without surfacing a "gene" UI.
last_verified: 2026-04-22
status: scaffolded; awaiting Phase 2 internal-model lock
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

## Open questions (Phase 2 lock)

1. **Internal gene model size** — 22 fields sufficient? Overkill? Recommend lock at 22 for MVP.
2. **Phenotype label catalog** — how many distinct labels? Recommend 30-50 for variety without becoming dense.
3. **Advanced tooltip numeric exposure** — default off, power-user toggle on. Confirm.
4. **Compiler reproducibility across model versions** — Claude Opus 4.7 is current; if 4.8 ships, same prompt+seed may produce different text. Strategy: freeze model version per content pack version; checked-in JSON is canonical; don't regenerate on model updates unless pack_version bumps.
5. **Content pack delta strategy** — how do we add 500 new players in content pack v1.1 without breaking v1 save compat? Strategy: new IDs in delta pack; v1 IDs never mutate; savefile references by ID, resolver falls back to base pack for unresolved IDs.

## Prototype gate

**Phase 3:** AI Content Compiler produces 22 valid Identity Packets for Month-3 slice. Manual review confirms each reads as a specific coherent footballer, not generic.

**Phase 4:** Scout Disagreement feel prototype uses 10 varied Identity Packets to demonstrate scout-bias divergence reads clearly.

**Phase 6:** Full cohort of ~2400 players compiled. Balance harness 10K-match sweep confirms no degenerate archetype dominates or bricks.

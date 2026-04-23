---
description: Fictional nation, pyramid structure, cultural priors, naming grammar. The bake-time inputs that make the generated world feel credible.
last_verified: 2026-04-22
status: scaffolded; awaiting Phase 2 nation lock
---

# Worldbuilding — fictional nation + football structure

## Purpose

Answer "what's the fictional football world we're simulating, and what cultural priors feed the AI Content Compiler to make it feel credible rather than sterile?"

## Locked decisions

See SPEC.md 2026-04-22. Summary:

- **Fully fictional football world.** No real places, no real clubs, no alternate-history framing.
- **England-readable grammar.** Six-tier pyramid, promotion/relegation, cup competitions, home/away fixtures, Saturday 3pm cultural rhythm.
- **No "Albion" or other try-hard fantasy-nation framing** (per GPT-5.5 round 3). Nation name should read as a plausible country, not a fantasy.

## The nation (draft — Phase 2 lock)

### Concept

A single fictional nation. England-scale (population 50-60M-analog), coastal + inland, mixed urban-industrial + rural. Football is the dominant sport, culturally central. Modern-era setting (present-day-adjacent).

### Nation naming — user to lock

Proposals (no preferred pick; user choice):

1. **Anvara** — invented word, vaguely Northern European; does not map to real place
2. **Wellingsham** — invented but English-sounding compound
3. **The Reach** — neutral territorial name (no "kingdom" framing); England-readable
4. **Cresland** — invented compound; coastal-feeling
5. **[no nation name]** — game says "the league" in UI; nation's name only appears in lore flavor

Recommend #5 or #3. Avoids try-hard fantasy while keeping football grammar clean.

### Regional structure (draft)

~6-8 regions within the nation, each with cultural flavor priors for AI Content Compiler seeding:

| Region (fictional) | Analog | Flavor priors |
|---|---|---|
| The North | Manchester / Leeds analog | Direct football; industrial-city identity; working-class naming; long rivalries |
| The Capital | London analog | Cosmopolitan; cross-cultural player names; money-heavy clubs |
| The West | Cardiff / Bristol analog | Technical football; rugby-adjacent culture (in flavor, not mechanic) |
| The South Coast | Brighton / Southampton analog | Coastal; hip; young-squad flavor |
| The East | Norwich / Hull analog | Rural-hinterland clubs; local-talent emphasis |
| The Midlands | Birmingham / Nottingham analog | Mixed-industry; mid-table heartland |
| The Far North | Newcastle / Edinburgh analog | Fierce parochial support; older player cultures |
| Offshore islands | Jersey / Isle-of-Man analog | Footnote regions; "discovered unknown from the islands" flavor |

### Pyramid structure (locked)

Six tiers:

| Tier | Name (locked: football-native) | Size | Status |
|---|---|---|---|
| 1 | Top flight | 20 clubs | Full professional |
| 2 | Championship (or equivalent football-native name) | 24 clubs | Full professional |
| 3 | Tier 3 (name TBD) | Phase 2 lock | Semi-professional split or single league |
| 4 | Tier 4 | Phase 2 lock | Semi-pro / amateur mixed |
| 5 | Regional leagues | Phase 2 lock | Amateur-ish |
| 6 | Sub-regional leagues | Phase 2 lock | Non-league |

Promotion / relegation between tiers per typical English-football patterns. Cup competitions: one national cup, one league cup, one smaller-tier-club cup.

**Total clubs in EA content pack v1:** target ~96 fully simulated clubs. Exact tier distribution is not locked here because the earlier 20 + 24 + split-tier arithmetic overshoots the content target. Phase 2 must choose either smaller fictional tiers or lightweight lower-tier feeder pools, then update this table.

## Cultural priors (AI Content Compiler seeding)

Each region provides priors that bias the Compiler's generation:

```
RegionPriors {
    region_id: string
    name_patterns: {
        first_names: weighted list
        last_names: weighted list
        nickname_patterns: weighted list
    }
    physical_priors: {  // bias on gene-model physical fields
        height_ceiling_bias: f32
        frame_density_bias: f32
    }
    mental_priors: {
        composure_floor_bias: f32
        ambition_bias: f32
    }
    technical_priors: {
        aerial_bias: f32
        first_touch_bias: f32
    }
    cultural_flavor: {
        dominant_role_families: [enum]  // "this region produces wingers and CMs"
        stylistic_tendencies: [string]  // "direct football", "technical", "defensive"
    }
    naming_club_patterns: [string]  // "City of X", "X United", "X Town", "Royal X", ...
}
```

Example: **The North region** gets higher `frame_density` + `aerial` biases; club naming favors `Town / United / City`; common last names lean working-class Anglo-Saxon + post-industrial immigration waves.

## Cultural priors — non-human stuff

Other world-flavor facts for the Compiler:

- Kit patterns per region (stripes / solid / cuffs — determined at bake time)
- Stadium naming conventions (e.g., "Memorial Ground", "The Hollows", "Riverside Stadium")
- Fan culture flavor (e.g., chant style, ultra presence, corporate-owned vs community-owned ownership model)
- Historical seeded events (fictional — "the 1987 relegation of Northcote FC from top flight that fans still talk about")

## MVP boundary

At Month 3 slice: one home club + one opponent. Region-flavor for 2 regions hand-compiled. Nation name + flag placeholder.

At Month 12 EA: full ~96 clubs × 8 regions with cultural-prior seeding. Every club has a generated history (founding year, biggest trophies, rival clubs, current-era identity). Players have region-of-birth; names + style match region priors.

## Deferred

- Multi-nation expansion — post-1.0
- European continental competition — post-1.0
- International national-team management — post-1.0
- Deep historical seeding (multiple decades of fictional history) — gradual through EA+

## Open questions (Phase 2 lock)

1. **Nation name (above proposals)** — user picks or supplies alternative.
2. **Region count** — 6 vs 8 vs more? Recommend 8 as target for variety without content bloat.
3. **Pyramid tier-count exactness** — 6 is locked. Exact club counts per tier TBD during Phase 6 bake.
4. **Cup competition structure** — FA-Cup-analog all-tiers vs restricted? Recommend all-tiers (classic underdog narrative).
5. **Real-world parallels in flavor** — how explicit? Is "The North" openly an industrial-north analog, or more distant? Recommend loose — mention flavor only to the Compiler; user-facing world is not on-the-nose.

## Prototype gate

**Phase 2 lock:** nation name chosen; 8 regions finalized with prior JSON schemas.

**Phase 4:** AI Content Compiler generates 1 region's full club set (~10-15 clubs + ~250 players) with validated cultural-prior expression. Spot-check: player names and styles read as regionally coherent.

**Phase 6:** Full ~96-club world compiled. Playtesters confirm "the world feels lived-in" (subjective but critical signal).

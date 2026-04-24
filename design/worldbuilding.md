---
description: Fictional nation, pyramid structure, cultural priors, naming grammar. The bake-time inputs that make the generated world feel credible.
last_verified: 2026-04-24
status: Phase 0 open questions resolved; nation locked as Caldren (Cresland fallback), 8 regions, 96-club MVP slice with tier distribution 20/24/16/14/12/10, three-cup structure, compiler-only analogue strings with Phase-1 lint. No new ADR (RegionPriors covered by AI Content Compiler ADR).
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

### Nation name — Caldren (locked 2026-04-24; Cresland fallback)

**Locked: Caldren.** Reads as a grounded fictional football nation, supports clean league/cup naming, and avoids awkward demonym forms.

Football-sentence examples:
- *Caldren Premier Division*
- *Caldren National Cup*
- *Caldren Football Association*
- *"a lower-tier club from western Caldren"*
- *"Caldren clubs have always favoured direct wide play"*

Demonym: **Caldren** (uninflected — avoids awkward "Creslish" / "Anvaran" / "Wellinghamite" forms).

**Fallback:** Cresland — acceptable if formal trademark / Steam-name clearance against Caldren fails during Phase-8 launch prep. Cresland has known low-noise uses (Cresland Development Group, CRESLAND LTD, Cresland/Crescent Island portfolio) but is clear enough for in-game setting context.

**Rejected candidates (lightweight clearance-pass findings, 2026-04-24):**
- `Anvara` — active ad-marketplace brand; reads fantasy/startup-slick
- `Wellingsham` — reads as a town/city within a nation, not a nation itself
- `The Reach` — trademark noise + obvious Halo association + region/territorial framing rather than nation
- `Haldren` / `Keldren` — stronger sound but read more fantasy-RPG than football sim
- `Brisland` — plausible grammar, too soft and surname-adjacent
- `Northmere` — active software / licensing noise
- `Rivermark` — trademark noise
- `Valmere` — existing Steam publisher
- `[no nation name]` — ruled out by 2026-04-24 overview resolution requiring a **named** fictional nation

**Scope caveat:** Caldren is in-game setting context, not Steam-page branding. Formal legal clearance deferred to Phase-8 prep alongside the "Final Whistle" title clearance.

<!-- ui-lint:ignore-start reason="compiler-seeding region-analog table; strings ship dev-config-only, never runtime content" -->
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
<!-- ui-lint:ignore-end -->

### Pyramid structure (locked 2026-04-24)

Six tiers. **The 96-club total is the fully simulated slice, not the entire off-screen Caldren football ecosystem.** The broader lower pyramid exists abstractly — referenced in lore, in scout flavour ("he came up through the Tier-7 Southern Counties league"), and in occasional ledger events — but is not simulated match-by-match.

| Tier | Name (football-native; final naming at Phase 6 bake) | Clubs | Status | Structure |
|---|---|---|---|---|
| 1 | Caldren Premier Division | 20 | Full professional | Single league |
| 2 | Championship-equivalent (Caldren-native name TBD) | 24 | Full professional | Single league |
| 3 | Tier-3 name TBD | 16 | Semi-professional | Single league |
| 4 | Tier-4 name TBD | 14 | Semi-professional | Single league |
| 5 | Regional leagues | 12 | Amateur-ish | 2 × 6 regional splits |
| 6 | Sub-regional / feeder | 10 | Non-league | Feeder / reserve pool |

**Total: 96** fully simulated clubs for EA content pack v1.

**Promotion / relegation:**
- Tier 1 ↔ Tier 2: 3 up / 3 down (top-flight tempo)
- Tiers 2 ↔ 3, 3 ↔ 4, 4 ↔ 5, 5 ↔ 6: 2 up / 2 down (slower churn, identity stickiness for lower-tier clubs)
- Finalise at Phase 6 if playtesting demands adjustment

**Small-tier season format** — flagged as Phase-6 decision point:
- Tier 5's 2×6 regional split = 10 home/away matches per team per season at pure round-robin. That's too thin for a full-season rhythm.
- Options: **(a)** repeat fixtures (home/away × 3 → 30 matches), **(b)** add a lightweight cross-group phase (top 2 of each group play knockout after regular season), or **(c)** run shorter concurrent seasons for lower tiers that end mid-year.
- Not locked at Phase 0 — Phase 6 picks based on bake-test feel.

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

## Cup competitions (locked 2026-04-24)

Three cups. English-football-pattern.

| Cup | Eligibility | Narrative role |
|---|---|---|
| **National Cup** (FA-analog) | All 96 clubs, all 6 tiers | The underdog cup. Tier-6 giant-killing runs are a memory-pillar jackpot. |
| **League Cup** (EFL-Cup-analog) | Top 2 tiers only (44 clubs) | Mid-stakes silverware. Rotation-friendly. |
| **Trophy** (EFL-Trophy-analog) | Tiers 3-6 (52 clubs) | Lower-tier silverware. Gives small clubs a chance at their own Wembley moment. |

All-tier National Cup is essential: memory-pillar giant-killing runs from a Tier-6 club to a top-flight semi-final is one of the few ways a lower-tier save can produce an event that rivals a top-tier title. Restricting the main cup would flatten that.

Trophy is low engineering cost relative to the narrative value it provides lower-tier saves — kept in scope.

## Real-world-parallel flavour (locked: compiler-only; no user-facing leakage)

<!-- ui-lint:ignore-start reason="meta-reference describing the compiler-only analogue column" -->
The region analog column in the region-structure table (Manchester / Leeds / London / etc.) is **compiler-seeding context only** — never rendered to the user.
<!-- ui-lint:ignore-end -->

**Rules:**
- User-facing surfaces (scout reports, commentary, press, region names in-game) refer to Caldren regions by their **in-game fictional names** (finalised at Phase-6 bake).
- The `RegionPriors` `region_id` is the stable enum the runtime uses; the real-world analog string is a **compiler-config-only annotation** that never ships in runtime content packs.
- Ideal storage: analogue strings live in `dev-config/compiler/region-analogues.json` or equivalent, gitignored-from-runtime-build. Never in the shipped content pack payload.
<!-- ui-lint:ignore-start reason="Phase-1 lint rule spec naming the analogue strings it catches" -->
- **Phase-1 lint rule (required):** scan runtime content packs + all user-facing string tables for any of the analogue-column strings (`"Manchester"`, `"Leeds"`, `"London"`, `"Cardiff"`, `"Bristol"`, `"Brighton"`, etc.). Any match = build failure. Prevents Uncanny-Valley England leakage.
<!-- ui-lint:ignore-end -->

Flavour priors that DO ship: `physical_priors`, `mental_priors`, `technical_priors`, `dominant_role_families`, `stylistic_tendencies`, `naming_club_patterns`. None of these reference real places.

## Resolved (2026-04-24)

See SPEC.md decisions log entry `2026-04-24 — Worldbuilding open questions resolved`. No new ADR — `RegionPriors` schema governance is covered by the existing Phase-2 `IdentityPacket / AI Content Compiler` ADR.

1. **Nation name:** **Caldren** (locked; Cresland fallback if Phase-8 clearance fails). See "Nation name" section above.
2. **Region count:** **8 regions locked.** Internal analog table preserved as compiler-seeding context; user-facing names fictionalised at Phase-6 bake.
3. **Pyramid tier distribution:** **20 / 24 / 16 / 14 / 12 / 10 = 96** fully simulated clubs for EA v1. This is the simulated slice, not the entire Caldren football ecosystem — the broader lower pyramid exists abstractly off-screen. Small-tier season format (pure round-robin vs repeat fixtures vs cross-group phase) flagged as Phase-6 decision point.
4. **Cup structure:** **three cups locked.** All-tier National Cup + top-2-tier League Cup + Tiers-3-6 Trophy.
5. **Real-world parallel explicitness:** **loose; compiler-only.** Analogue strings are dev-config-only and never ship. Phase-1 lint rule blocks runtime leakage.

## Prototype gate

**Phase 2 lock:** nation name chosen; 8 regions finalized with prior JSON schemas.

**Phase 4:** AI Content Compiler generates 1 region's full club set (~10-15 clubs + ~250 players) with validated cultural-prior expression. Spot-check: player names and styles read as regionally coherent.

**Phase 6:** Full ~96-club world compiled. Playtesters confirm "the world feels lived-in" (subjective but critical signal).

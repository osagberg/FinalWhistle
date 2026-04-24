---
description: Banned-terms lint + approved football-native phrasing catalog. The discipline that stops the game from sounding like a fantasy-RPG skin over football.
last_verified: 2026-04-24
status: Phase 0 open questions resolved; lint scope + sentinel-exemption mechanism + template pool structure + tone register locked. Category-A bans expanded with 2026-04-24 additions from prior resolutions.
---

# UI Vocabulary — the anti-cringe contract

## Purpose

Answer "what words are allowed in player-facing text, and what words are banned, so the game never accidentally becomes a fantasy-RPG skin over football?"

GPT-5.5's framing: "The anime layer should be visual, not lexical. If a commentator can't say it without sounding like they lost a bet, don't capitalize it."

## Locked decisions

See SPEC.md 2026-04-22. Summary:

- **No capitalized state nouns in player-facing UI.** Banned as visible system names: "The Hush", "Weather", "Calling", "Canon", "Seven", "Kismet", "Soul", "The Author", "The Ledger" (as a UI noun).
- **Internal float names stay internal.** `momentum`, `rhythm`, `pressure`, `team_cohesion`, `signature_readiness` live in code + design docs, never in player text.
- **Football-native vocabulary** for all surfaced states. Lift directly from real commentary + supporter culture.

## The lint — banned terms in player-facing text

<!-- ui-lint:ignore-start reason="banned-term catalog" -->

### Category A — hard ban, no exemption path

Any text rendered in UI labels, button text, menu headers, in-match overlays, scout reports, press/fan copy, commentary text, achievement names, tutorial copy, or any shipped runtime string MUST NOT contain:

**A.1 — mystical / RPG / fantasy capitalized state nouns** (2026-04-22):

| Banned | Internal alternative | Player-facing replacement |
|---|---|---|
| The Hush | `signature_readiness` / `pressure` float | "He's locked in." / "The stadium's gone quiet." / "He's finding something." |
| Weather (as team state) | `team_cohesion` / `rhythm` | "Form: rising" / "They're starting to click" / "The side's found a tempo" |
| Calling (as player identity) | `role_family` / `playing_instincts` | "A natural winger" / "Plays like a #10" |
| Canon / Shelves / Reading Lists (as pyramid tiers) | `tier` | "Caldren Premier Division" / "Championship" / "Tier 3" |
| The Seven (as rival managers) | (system deferred; no UI needed) | — |
| Kismet / Soul / Flow (as gene flags) | internal `narrative_triggers` | "Something clicked today." / "A late bloomer, it seems." |
| The Author (as manager identity) | — | manager / head coach / gaffer |
| The Ledger (as UI noun) | internal `MemoryEvent` ledger | "Club history" / "Career" / "The archive" (lowercase) |

**A.2 — system / progression / menu-game vocabulary** (added 2026-04-24 per breakthrough-moments resolution):

| Banned | Internal alternative | Player-facing replacement |
|---|---|---|
| Signature unlocked | internal `SignatureAwakened` event | *"He's found something."* / *"He cuts inside again — and this time he goes through."* |
| Awakened (capitalized noun/verb) | internal `SignatureAwakened` event | *"Something clicked."* / *"That's new."* (lowercase "awakens" is Category B — soft ban) |
| XP gained / Level up / Skill point | — | no progression-mechanic surface exists; player development is narrative, not numeric |
| +5 finishing (stat-delta callouts) | internal `sim_bias` deltas | *"He's striking the ball cleaner."* / scout prose |
| Perk / Trait (as stat-label) | internal `trait` field | phenotype label from `design/player-generation.md` catalog |

**A.3 — genetics / bloodline vocabulary** (added 2026-04-24 per player-generation resolution):

| Banned | Internal alternative | Player-facing replacement |
|---|---|---|
| Genes / Genetics / Chromosomes | internal `gene_model` fields | phenotype labels + scout prose |
| Bloodline (as mechanic) | internal `tactical_dna_fragments` | Coaching-lineage surfacing deferred post-MVP; no UI noun |
| DNA (as player-facing stat) | internal `identity_packet` | phenotype labels + scout prose |

**A.4 — stigmatizing / systemic phenotype framings** (added 2026-04-24 per player-generation resolution):

| Banned | Canonical replacement |
|---|---|
| Fragile Under Scrutiny | **Struggles Under Scrutiny** (per `design/player-generation.md`) |
| Fragile When Tested | **Struggles Under Scrutiny** |
| Plateau Risk | (removed from enum; surface via scout prose + projected-range narrowing) |
| Injury-Prone | (not a label; injury history surfaces as explicit event record) |
| Powerful Striker (as phenotype) | **Powerful Ball Striker** — avoids confusion with striker-as-position |

**A.5 — real-world place-name analogues** (added 2026-04-24 per worldbuilding resolution):

Any occurrence in runtime content packs or user-facing strings: `Manchester`, `Liverpool`, `Leeds`, `London`, `Cardiff`, `Bristol`, `Brighton`, `Southampton`, `Newcastle`, `Edinburgh`, `Norwich`, `Hull`, `Birmingham`, `Nottingham`, `Jersey`, `Isle of Man`.

Replacement: Caldren-region fictional names (finalised at Phase-6 bake). Design-internal `RegionPriors` analogue strings live in `dev-config/compiler/region-analogues.json`, gitignored from runtime build.

<!-- ui-lint:ignore-end -->

<!-- ui-lint:ignore-start reason="banned-term catalog" -->

### Category B — soft ban, inline exemption allowed with audit

Avoid unless the specific surface genuinely needs them. Exemption mechanism:

```csharp
// ui-lint:allow term="weapon" reason="cup-final commentary, deliberate" reviewer="osagberg"
commentary.Push("He'll need his best weapon in this final.");
```

Rules:
- `term=` must match a Category-B banned term exactly.
- `reason=` must be non-empty and specific.
- `reviewer=` must be a handle.
- Exemptions without reviewer attribution are lint fails.
- CI emits an exemption report. Exemptions are reviewed before **EA content lock** and before **every release candidate** — not on a fixed calendar.

Category-B terms:

- "awakens", "awakened" (lowercase verb of gene unlock) — prefer "clicked", "found", "broke through"
- "Savant", "Genius" (as stat-label) — use phenotype labels from `design/player-generation.md`
- "Weapon" / "Weaponize" — use "signature", "technique", or role-specific term
- "Egoist", "The Ego" — use "manager", "gaffer", "boss"
- "Realm", "Domain", "Kingdom" — no royal/fantasy territory framing
- "Power level" — use football-native stakes language
- "Forge", "Forged" (as generator verb) — use "compiled", "generated", "built"

<!-- ui-lint:ignore-end -->

### Category C — over-quoted FM-specific vocabulary (context-use only)

Allowed but in their proper football context:

- "potential", "ability" — OK as scouting vocabulary, but tempered with "projected range" + phenotype labels
- "morale", "form" — OK, standard football-English
- "condition", "sharpness" — OK
- "legend" — OK sparingly; never auto-assigned

## Approved football-native vocabulary

### Match-state language (what commentary + UI says)

- "He's locked in." / "He's found the read." / "Something's clicked today."
- "The stadium's gone quiet." / "The home crowd's turned on them."
- "Tempo's shifted." / "The side's on top." / "They're chasing shadows."
- "He arrives late in the box." / "Looks for the early ball." / "Turns on the shoulder."
- "Body's square." / "Leaves his marker for dead." / "Reads it early."

### Team-state language

- "Form: rising / faltering / holding" (structured label)
- "Confidence: shaky / rising / serene" (structured label)
- "Tempo: controlled / bite-and-kick / open" (structured label)
- Commentary: "the side's clicking", "they've got a rhythm", "they can't string two passes"

### Player-identity language

**Phenotype labels:** authoritative catalog lives in [`design/player-generation.md`](player-generation.md) — 46 labels across Physical / Mental / Technical / Development / Role-specific. This doc does NOT duplicate the list. Label IDs are content-pack-qualified; all player-identity copy flows through that catalog.

Examples (not exhaustive — see player-generation.md for the full catalog):
- `Late Bloomer`, `Composed Under Pressure`, `Struggles Under Scrutiny`, `Set-Piece Natural`, `Reads the Game`, `Sweeper Keeper`, `Half-Space Creator`

**Signature display names:** authoritative catalog lives in [`design/signatures.md`](signatures.md) — 24 signatures, football-copy-only names. Cross-doc exact-match discipline (2026-04-24 signatures lock).

Examples:
- *"Looks for early crosses"* / *"Late arriving in the box"* / *"Underlap into cutback lane"*
- *"First-time diagonal switch"* / *"Cuts inside onto his stronger foot"* / *"Low cutback from the byline"*
- *"Fast long release"* / *"Commands his area"* / *"Front-foot interception"*

## MVP boundary

At Month 3 slice: 3-shot-type overlay text + 3 signatures' UI copy reviewed against this lint. No banned terms present.

At Month 12 EA: lint implemented as automated check:

```
# Runs on every PR touching UI text / ScriptableObject content
scripts/lint-banned-terms.py \
    --input design/ui-vocabulary.md:banned_terms \
    --scope "Assets/_Project/**/*.cs" "content/**/*.json" "design/**/*.md-rendered"
```

Lint fails on any hit; CI blocks merge until fixed.

## Deferred

- Localization vocabulary packs — Phase 7 content scaling
- Per-commentator style variants — post-EA polish
- Dynamic commentary generation via bake-time LLM — Phase 6 if template variety feels thin

## Commentary template pool structure (locked 2026-04-24)

**Flatter-per-shot-type pools, not per (shot × stake × memory) combinations.**

- **15-30 templates per shot type** (7 shot types → ~105-210 templates for match-flow overlay text).
- **MVP target: ~140 total match-flow overlay templates.**
- **Stake + memory modulation** selects eligible variants from the shot-type pool — not separate pools.
- **Slot variables** (`{scorer_name}`, `{last_goal_time}`, `{memory_callback_phrase}`, etc.) create variety compounding from data, not from template count.
- **Separate pools** (separately counted) for scout reports, press/fan copy, post-match reports.

Do NOT build per (shot_type × stake_band × memory_hit) pools at MVP — that's 42-class × 15-30 = 630-1260 templates, content bloat disguised as polish.

**Governance:** template IDs are content-pack-qualified; template rendering is bake-time (no runtime LLM). Template governance folds into the Phase-2 `IdentityPacket / AI Content Compiler` ADR (pre-seeded 2026-04-24) — no separate ADR.

## Tone register (locked 2026-04-24)

**Default English: British-football vernacular.**

Locale-specific: **native football idiom in target language.** Translation briefs for Phase-7 localisation use football-register equivalence, not literal translation. A British phrase that doesn't translate (*"chasing shadows"*) becomes the closest native football-register equivalent in the target locale.

Per-locale banned-terms lints — each locale may need its own stigmatizing-language list that doesn't exist in EN. Phase-7 deliverable.

## Resolved (2026-04-24)

See SPEC.md decisions log entry `2026-04-24 — UI vocabulary open questions resolved`. No new ADR — template governance folds into existing AI Content Compiler ADR.

1. **Lint enforcement surface:** UI code (`Assets/_Project/**/*.cs`) + runtime content (`content/**/*.json`) + rendered player-facing doc/content outputs. **Self-reference exemption uses sentinel comments only** — `<!-- ui-lint:ignore-start reason="..." --> ... <!-- ui-lint:ignore-end -->` — never whole-file whitelist. Only the banned-term catalog sections in this doc are ignored; everything else remains lintable.
2. **Category-A:** hard ban, **no exemption path.** Expanded with 2026-04-24 additions from prior resolutions: system/progression vocabulary, genetics/bloodline terms, stigmatizing phenotype framings, real-world place-name analogues (see Categories A.1-A.5 above).
3. **Category-B:** inline exemption allowed via `// ui-lint:allow term="..." reason="..." reviewer="..."` with audit discipline. CI emits an exemption report. Exemptions are reviewed **before EA content lock and before every release candidate** — not on a calendar cadence.
4. **Commentary templates:** flatter per-shot-type pools of 15-30 templates, ~140 MVP match-flow target, stake/memory filters + slot variables for variety. Separate pools for scout reports / press-fan / post-match. Governance folds into AI Content Compiler ADR.
5. **Tone register:** British-football vernacular default for EN; native football idiom per locale with per-locale banned-term lists.

## Prototype gate

**Phase 3 Week 4:** Month-3 slice passes lint — zero banned terms in any player-visible text.

**Phase 5:** lint runs on CI; enforcement green across full-season playthrough UI.

**Phase 7:** localization extraction picks up approved phrasing; translators brief is "football-native vernacular in target language", with banned-terms lint respected per-locale.

---
description: Banned-terms lint + approved football-native phrasing catalog. The discipline that stops the game from sounding like a fantasy-RPG skin over football.
last_verified: 2026-04-22
status: scaffolded; awaiting Phase 2 lint lock
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

### Category A — mystical / RPG / fantasy capitalized nouns (hard ban)

Any text rendered in UI labels, button text, menu headers, in-match overlays, scout reports, press/fan copy, commentary text, achievement names, or tutorial copy MUST NOT contain:

| Banned | Internal alternative (if needed) | Player-facing replacement |
|---|---|---|
| The Hush | `signature_readiness` / `pressure` float | "He's locked in." / "The stadium's gone quiet." / "He's finding something." |
| Weather (as team state) | `team_cohesion` / `rhythm` | "Form: rising" / "They're starting to click" / "The side's found a tempo" |
| Calling (as player identity) | `role_family` / `playing_instincts` | "A natural winger" / "Plays like a #10" |
| Canon / Shelves / Reading Lists (as pyramid tiers) | `tier` | "Top flight" / "Championship" / "Tier 3 South" |
| The Seven (as rival managers) | (system deferred anyway; no UI needed) | — |
| Kismet / Soul / Flow (as gene flags) | internal `narrative_triggers` | "Something clicked today." / "A late bloomer, it seems." |
| The Author (as manager identity) | — | manager / head coach / gaffer |
| The Ledger (as UI noun) | internal `MemoryEvent` ledger | "Club history" / "Career" / "The archive" (soft, non-capitalized) |

### Category B — fantasy-RPG grammar (soft ban — requires justification)

Avoid unless the specific surface genuinely needs them:

- "awakens", "awakened" (as verb of gene unlock) — prefer "clicked", "found", "broke through"
- "Savant", "Genius" (as stat-label) — use phenotype labels ("Set-Piece Natural", "Reads the Game")
- "Weapon" / "Weaponize" — use "signature", "technique", or role-specific term
- "Egoist", "The Ego" — use "manager", "gaffer", "boss"
- "Realm", "Domain", "Kingdom" — no royal/fantasy territory framing
- "Power level", "Tier" as internal-power-ranking — tier is OK as league-tier only
- "Forge", "Forged" (as generator verb) — use "compiled", "generated", "built"

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

Phenotype labels (from Identity Packet `scout_labels`):

- "Late Bloomer" / "Early Peak" / "Physically Raw"
- "Composed Under Pressure" / "Fragile When Tested"
- "Explosive First Step" / "Set-Piece Natural" / "Aerial Threat"
- "Reads the Game" / "Direct" / "Technical"
- "Hometown Kid" / "Cross-Border Signing" / "Academy Graduate"

Signature names (football-copy only):

- "Looks for early crosses" / "Arrives late in the box" / "Underlaps into cutback lane"
- "Plays first-time diagonal switches" / "Cuts inside on his weaker foot"
- "Fast long release" / "Commands his area" / "Front-foot interception"

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

## Open questions (Phase 2 lock)

1. **Lint enforcement surface** — code + content + rendered-design-docs, or just code + content? Recommend all three; rendered-design-doc check catches leaks into player-visible places.
2. **Category-B soft-ban exceptions** — some terms may have legitimate uses (e.g., "weapon" in a cup-final-commentary context). Recommend: allow with `// ui-lint:allow reason="..."` inline exemption comment in source.
3. **Commentary template pool size** — how many unique phrases per match-event class? Target: 15-30 per class for freshness without bloat.
4. **Tone register** — how "British football commentary" vs neutral? Recommend lean British-football for default English localization; allow neutral register for other locales where translation idiom differs.

## Prototype gate

**Phase 3 Week 4:** Month-3 slice passes lint — zero banned terms in any player-visible text.

**Phase 5:** lint runs on CI; enforcement green across full-season playthrough UI.

**Phase 7:** localization extraction picks up approved phrasing; translators brief is "football-native vernacular in target language", with banned-terms lint respected per-locale.

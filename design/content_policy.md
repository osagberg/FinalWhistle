---
description: PEGI 12 / ESRB T content boundaries + real-world content policy + AI-content disclosure + mod-pack content-safety review surface. Consolidates posture already encoded across PROJECT_CONTEXT, CLAUDE, SETUP, worldbuilding, player-generation, ui-vocabulary, and FW-VAL-D-005.
last_verified: 2026-04-24
status: Phase 2 authoring pass — rating target locked at PEGI 12 / ESRB T from bootstrap (2026-04-22); no-real-people + no-real-places + no-real-clubs posture locked via worldbuilding.md + player-generation.md; AI-content disclosure covered by FW-VAL-D-005 + SETUP.md §7. This doc consolidates + closes modding.md open question #2 (mod-pack content-safety review surface).
---

# Content Policy — PEGI 12 / ESRB T boundaries

## Purpose

Answer "what content is in-scope for a PEGI 12 / ESRB T football-management RPG, what's out, how does the game stay there under AI-compiled content + user mods, and how do press / fan / dressing-room narrative beats stay mature without breaching the rating?"

Framing: the rating is not the ceiling we stretch toward — it's the floor we design from. Football-management drama at PEGI 12 is rich: relegation anxiety, dressing-room factionalism, ageing-star contract standoffs, derby-day hostility, press narrative attacks, career setbacks that stack across seasons. None of that requires language, violence, or content that a 12-year-old can't encounter. Career-memory depth comes from systemic consequence, not from shock material.

## Locked decisions

**Rating target (bootstrap 2026-04-22):** PEGI 12 / ESRB T. Source: `PROJECT_CONTEXT.md §3`, `CLAUDE.md §1`.

**Cross-system posture already seeded:**

- **No real people / clubs / places / leagues / kits.** Fully fictional world. Source: `PROJECT_CONTEXT.md §9 Non-goals`, `design/worldbuilding.md` Caldren-is-fictional lock, `design/player-generation.md` compiler-generated-only discipline, 2026-04-22 bootstrap decision *"Fully fictional football world with England-readable grammar"*.
- **Compiler-only real-world analogue strings.** Region analogues live in `dev-config/compiler/region-analogues.json`, gitignored from runtime build; Phase-1 banned-terms lint catches leakage into runtime content (Category A.5 in `design/ui-vocabulary.md`). Source: worldbuilding.md 2026-04-24 resolution + FW-VAL-D-002 in `design/specs/content-pack-validation-contract.md`.
- **Banned-terms vocabulary discipline.** Category A hard-ban (no exemption) covers mystical / RPG / progression / genetics / stigmatizing-phenotype / real-world-place-analogue subsections. Category B audited exemption for soft-bans. Source: `design/ui-vocabulary.md` + `scripts/lint-banned-terms.py` + FW-VAL-A-015 / A-016.
- **AI-content disclosure at Steam level.** Ships with Steam's AI-content-disclosure metadata enabled per Valve's 2025 policy. Source: `SETUP.md §7 Privacy / security posture`, FW-VAL-D-005 AI-content disclosure manifest complete check.
- **No voiced commentary at MVP.** Text commentary only; audio-VO evaluation post-EA contingent on player demand. Source: `PROJECT_CONTEXT.md §5 Audio target`, `TOOLING.md §External services`.
- **Legal-sensitive names diff.** Tier-D FW-VAL-D-001 catches real-world-club / real-person / trademark matches against the gitignored reference list before RC build. Source: `design/specs/content-pack-validation-contract.md`.

## The content-policy boundary — what ships, what doesn't

### Mature elements that ARE in scope (PEGI 12-appropriate)

Football management is a sport-drama; rating-appropriate tension drives the game's retention hook. The following ARE shipping material, handled in language-safe football-native vocabulary per `design/ui-vocabulary.md`:

| Theme | Shipping form | Example surface |
|---|---|---|
| **Dressing-room tension** | Factional metadata on squad screens; scout prose; press reactions; morale-adjacent commentary | *"The senior players have stopped speaking to the captain."* / *"A training-ground rift, swept under the rug but not forgotten."* |
| **Ageing-star decline narratives** | Phenotype labels; memory-event callbacks; contract-talk trigger events | *"He knows the legs aren't what they were."* / *"One more season, probably. Maybe."* |
| **Relegation anxiety** | Fan-sentiment text; press-narrative templates; board-pressure ledger events | *"The mood at the ground has turned. Supporters want answers."* |
| **Derby / cup-final hostility** | Stakes-modulated semantic cinema; crowd-audio cues; memory-event emphasis | (visual: tighter paneling, desaturated bench shots; prose: *"This place has a memory. He'll hear it."*) |
| **Press narrative attacks on form** | Press-quote templates with football-specific criticism | *"Another anonymous performance. Questions are being asked."* |
| **Contract standoffs + betrayal** | Promise-tracking reader + rival-recall reader (per event-sourced memory); post-transfer callback events | *"He refused to shake your hand in the tunnel. Five years since you sold him."* |
| **Injury + absence** | Football-standard description of injury type + recovery window; never medical-graphic prose | *"Hamstring, likely 4-6 weeks."* / *"A knock picked up in training; he'll be assessed."* |
| **Career setback + redemption arcs** | Salience-preserved memory-event callbacks across multiple seasons | *"Two years since that night at Elmswood. He's back."* |
| **Manager confrontations (with board, players, press)** | Press-quote rendering + dressing-room event emissions; language-safe tone (firm, not profane) | *"He told the board, privately, that the budget wasn't enough. It leaked within the week."* |
| **Referee / officiating controversy (in football-native terms)** | Match-event annotation + commentary templates; no abuse language | *"A clear pull, no whistle. The stadium has gone quiet."* |
| **Transfer-market leverage / agent pressure** | Contract-talk event emissions; agent-demand narrative beats | *"His agent has been working the phones. The interest is real."* |

These are the retention hook. The game IS allowed to be about these things, at depth, at PEGI 12, in football-native language.

### Content that is NOT in scope (ruled out by PEGI 12 target + project posture)

<!-- ui-lint:ignore-start reason="content-policy exclusion list must enumerate disallowed content categories by name for prescriptive clarity" -->

- **Violence beyond football-standard physical challenge.** Tackles, fouls, cards, the occasional hard shoulder are match material. Fights, brawls, post-match assault narratives, crowd-violence depictions are NOT. Tunnel confrontations stay verbal-non-profane. No injury prose describing blood, bruising-gore, or acute medical-graphic detail.
- **Explicit sexual content.** None at any surface. No romantic-partner mechanics, no dressing-room intimacy prose, no personal-life-romantic press angles. Footballer-personal-life narratives stay at the level of *"his wife has been ill; he missed training this week"* — grounded, language-safe, sparse.
- **Substance use.** No alcohol-mechanic, no drug-mechanic, no match-day-hangover prose, no performance-enhancing-drug narrative. Post-match celebrations are described as *"celebrated late"* / *"enjoyed the result"* with no substance-specific material.
- **Gambling / betting mechanics.** Football's match-fixing history is well-documented; the game does not simulate betting lines, fix-the-match events, or bookmaker odds as a mechanic. The AI-compiler does not generate match-fixing narrative content.
- **Real-world political content.** No real-world politicians, no real-world political parties, no real-world conflict / war / protest narrative surfacing in press quotes or fan sentiment. The Caldren setting is deliberately apolitical at the national-narrative level; regional rivalries are cultural + footballing, not ethnonational or sectarian.
- **Real-world religion references.** No real-world denominational naming, no real-world religious conflict narrative. Stadium rituals / chants / traditions are fictional-cultural.
- **Hate speech / slurs of any kind.** Banned at every surface: commentary templates, press quotes, fan-sentiment text, scout prose, dressing-room events, mod-pack content. This is Category A enforced via banned-terms lint + locale-specific banned-term lists per `design/ui-vocabulary.md`.
- **Real-person likenesses / names / voices.** Every player / manager / staff member / board member is compiler-generated. No real footballer names in shipping content — caught by Tier-D FW-VAL-D-001 legal-sensitive-names diff before RC.
- **Real-club / real-league / real-venue names.** Same discipline. Caldren Premier Division, Caldren National Cup — never a real competition name.
- **Real-world-brand sponsor content.** Fictional sponsors only; kit-front logos generated by the AI Content Compiler against fictional-brand seeding.
- **Graphic crowd-violence depiction.** Supporter culture in visuals is energetic, hostile-in-voice-not-action, cheering + chanting; no depicted flares-to-pitch, no depicted pitch-invasion-assault, no depicted stand-violence. Crowd-reaction shot-type (`crowd-reaction` per `design/semantic-cinema.md`) stays stylized and choreographed.
- **Self-harm / suicide narrative.** Not in compiler templates, not in press-quote pools, not in mod-pack validator-allowed surface.
- **Child endangerment prose / imagery.** Academy-player narratives stay at age-appropriate tactical/developmental framing (*"a precocious 16-year-old"* / *"the academy's most promising teenager"*). No personal-life prose about youth players beyond footballing context.

<!-- ui-lint:ignore-end -->

### Edge cases — explicit rulings so the AI compiler doesn't drift

- **Post-derby hostility prose.** Allowed to describe *"hostile reception"*, *"the stadium has turned against him"*, *"not a friendly return"*. NOT allowed to describe crowd slurs, targeted personal attacks on protected characteristics, or supporters-attacking-supporters violence.
- **Managerial dismissals.** Ledger events describe dismissal, not graphic confrontation. *"Let go after the cup exit. Two years, four trophies, one very public row."* is in-scope. Naming the row's insult-content is not.
- **Youth-player career-path risk.** Players who don't progress get framings drawn from neutral / compassionate prose templates — never stigmatizing language like "failure" / "washout" / "wasted talent". Example template surfaces (illustrative, NOT locked `PhenotypeLabelId` values — phenotype catalog is locked separately in `design/player-generation.md` + 2026-04-24 player-generation resolution): *"Quietly gone"*, *"Did not kick on"*, *"Returned to semi-pro"*. Compiler generates neutral-or-compassionate narrative language around development shortfalls. If any of these examples needs to become an actual locked `PhenotypeLabelId`, route through normal catalog/schema discipline (Phase-4 IdentityPacket / AI Content Compiler ADR — currently ADR-0006).
- **Racism / discrimination in football history.** Real-world football has documented this; the game's fictional football DOES NOT reproduce it. No narrative beats about race / nationality-based abuse, stand-culture racism, or managerial-discrimination scandals. Caldren's football culture is not an alternate-history-cleansed England; it's a fictional country whose narrative scope intentionally excludes this theme. Modders cannot introduce it under §Mod-pack review surface below.
- **Injury severity + career-ending injuries.** Career-ending injuries ARE in scope as a narrative beat (*"his career ended at Elmswood, on a wet Tuesday in November"*) — football reality, handled with gravitas not shock. Graphic on-pitch injury prose (blood, contorted-limb imagery, severity-specific medical description) is NOT in scope. The `pass-shot-impact` shot type never renders an injury moment; injuries go to `aftermath-freeze` with text-overlay prose.
- **Fan-sentiment during relegation / crisis.** Allowed: *"frustrated"*, *"angry"*, *"the mood has soured"*, *"supporters vented at the board"*. Not allowed: specific slur content, threats against individuals, specific acts-of-violence-against-manager narrative.

## AI-content disclosure

### Steam 2025 compliance

Every shipping build ships with Valve's AI-content-disclosure metadata enabled. Disclosure declares:

- **Pre-baked AI-generated content:** player-generation (names, personalities, identity packets), worldbuilding (region names, club names, cultural flavor strings), commentary + press + fan-sentiment template pools, scout-prose templates, match-report templates.
- **No runtime AI content.** Steam's category for "live AI" is NOT enabled. All AI output is bake-time, human-reviewed, checked-in-to-content-pack-JSON, deterministic per `TECH_APPROACH.md §4.2`.
- **Human review gate.** No AI output ships without human review at content-pack compile time. Reviewed output is canonical per ADR-0006 §LLM path outside byte-identical regeneration.

### Validator enforcement

`FW-VAL-D-005` (per `design/specs/content-pack-validation-contract.md`) asserts the pack manifest carries an `ai_content_disclosure` block for any pack containing compiler-generated entities. Missing block = pack fails at RC. Base pack `fwh.core` is always populated; mod packs that include compiler-generated content MUST populate their own disclosure block.

### Content-pack manifest `ai_content_disclosure` block shape (Phase 6)

Locked separately at Phase-6 pack-manifest schema freeze; sketch:

```jsonc
{
  "ai_content_disclosure": {
    "uses_ai_generation": true,
    "generation_scope": ["player_names", "club_names", "commentary_templates", "press_templates"],
    "bake_time_only": true,
    "runtime_generation": false,
    "human_review_gate": true,
    "frozen_model_version": "<model-name-and-version>",
    "canonical_artifact_sha256": "<hash of the checked-in name-bank JSON>"
  }
}
```

## Commentary / prose / overlay content rules

These rules apply to every rendered string the player sees — commentary overlay text, press quotes, fan sentiment, scout prose, breakthrough-moment two-tier text, crowd-audio subtitles, match-report templates, post-match text-log entries.

1. **Banned-terms lint is prerequisite.** Category A hard-ban + Category B audited exemption per `design/ui-vocabulary.md`; FW-VAL-A-015 / A-016 enforce at validator level.
2. **British-football vernacular default.** Locked 2026-04-24 in `design/ui-vocabulary.md`; other locales use native football idiom per locale-specific banned-term lists.
3. **No capitalized mystical state nouns.** Internal floats stay invisible; surfaces via football-native commentary per `CLAUDE.md §6.1 + design/ui-vocabulary.md Category A.1`.
4. **Flatter template pools** (~140 templates MVP, per `design/ui-vocabulary.md` resolution): favours variation-by-slot-fill over deeper per-template branching, which reduces the surface area for content-policy drift.
5. **Prose reviewed at pack-compile time.** Per ADR-0006 LLM path outside byte-identical regeneration — every generated template is human-reviewed before it enters the canonical content-pack artifact. The validator doesn't re-review prose at pack-import time (Tier-A validator has no NLP layer); content-policy review is at authoring, enforced-at-output via banned-terms lint.

## Mod-pack content policy (Workshop surface — closes modding.md OQ#2)

Mod packs submitted to Steam Workshop — whenever Workshop integration ships (scaffolding Phase 6; UX post-EA) — pass the same Tier-A + Tier-D validator surface as the base pack. Per `design/modding.md §5` the registry catalogs are CLOSED code-owned; mods can only reference existing `ChainConditionId` / `EventClass` / `SimBiasFieldId` / `PhenotypeLabelId` / `CallbackTag` / `ScoutArchetypeKind` / `GeneCategory` values, so the validator surface is well-bounded.

### Content-safety review posture

**At EA (Workshop scaffolding live, UX deferred):**

- Automated validator checks: full Tier-A + Tier-D suite blocks any mod pack that fails ID format, unknown registry values, duplicate IDs, banned-terms Category-A, `NarrativeFlag` bias leak, legal-sensitive-names diff, or AI-content disclosure completeness. This catches the entire mechanical failure surface.
- **No automated content-safety review** for narrative prose / commentary templates / fan-sentiment text in mod packs beyond the banned-terms lint. Nuanced content-policy violations (e.g. a mod that technically passes banned-terms but pushes prose into discrimination territory) are caught by Valve's Steam Workshop review policy + user-report flow, not by Final Whistle's validator.
- **User-report flow** for mods that violate content policy relies on Steam Workshop's standard report-and-remove mechanism — Final Whistle does not duplicate this.

**Post-EA (Workshop UX live):**

- **Mod-pack content-safety review surface** is the open question flagged in `design/modding.md` OQ#2. The answer locked here: automated banned-terms lint + Steam Workshop report-flow covers the surface at launch; a dedicated in-game content-report flow is deferred post-EA, trigger-gated on mod-pack volume + observed content issues. If the Workshop-pack content-policy violation rate during EA is < 1% of active mods, the deferred status holds.
- **Category-B exemption audit applies to mod packs at Workshop-submit time.** A mod pack submitting with Category-B exemption count above the `FW-VAL-D-010` threshold (shared with base pack) is flagged for review before acceptance.

### What mod packs explicitly cannot do

- Introduce Category-A banned terms (hard-ban discipline; banned-terms lint catches).
- Reference registry values not present in the shipped binary (ClosedRegistry discipline; FW-VAL-A-007/008/009/010/012/013).
- Override base-pack entity IDs (base pack is floor per `design/modding.md §4`).
- Ship runtime code (C# assembly mods not in scope at any phase per modding.md §Deferred).
- Call external services / HTTP / LLM at runtime (data-only mods per modding.md §9).
- Populate `NarrativeFlag` scout-bias weights (FW-VAL-A-014 invariant).
- Misuse AI-content disclosure — a mod that generates content via LLM tooling but doesn't declare it in `ai_content_disclosure` fails FW-VAL-D-005.

## MVP boundary

**In at EA (ship-blocking content-policy enforcement):**
- PEGI 12 / ESRB T rating submission filled per this doc's scope-in + scope-out lists.
- All `fwh.core` content passes FW-VAL-A-015 (Category A) + FW-VAL-A-016 (Category B exemption audit) + FW-VAL-D-001 (legal-sensitive-names diff) + FW-VAL-D-005 (AI-content disclosure).
- AI-content disclosure block populated in `fwh.core` pack manifest.
- Banned-terms exemption report reviewed before RC + EA lock per `design/ui-vocabulary.md`.
- Workshop-pack validator applies the same discipline (scaffolding present; UX deferred).

**Out at EA (policy-adjacent features deferred):**
- In-game content-report flow for Workshop mods (Steam Workshop handles this at launch).
- Content-policy-violation training data for automated prose review — out of scope at EA, may never be in scope (Steam + banned-terms + community covers the surface).
- Locale-specific content-policy deltas beyond banned-term lists (Phase-7 localization pass + per-locale banned-term catalog covers this).
- Rating-submission in jurisdictions beyond PEGI + ESRB (other regional rating boards evaluated post-EA if distribution demand surfaces).

## Deferred

Seeded now; surfaces post-EA contingent on audience signal or regulatory trigger:

- **In-game content-report flow** for Workshop mods. Steam Workshop covers at EA.
- **Automated nuanced-prose review** (beyond banned-terms literal matching). Cost + false-positive risk + model dependency all point to "not at EA"; revisit if observed mod-content-policy-violation rate exceeds threshold.
- **Additional rating-board submissions** (USK, ClassInd, GRAC, etc.). Evaluated post-EA per distribution region.
- **Commentary voice-acting content-policy layer.** No voiced commentary at MVP; if VO ships post-EA per `PROJECT_CONTEXT.md §5`, a separate content-policy pass covers VO-specific material (fewer slur-adjacent edge cases because text → voice typically sanitizes; but inflection, tone, and delivery raise new questions).
- **Youth-player age raise / lower** — MVP treats youth at 15-16+ per Caldren setting; lowering youth-intake age requires a content-policy pass to ensure academy-player narrative stays age-appropriate.
- **Adult-rating variant (PEGI 16 / ESRB M) for post-1.0.** Not committed; evaluated if audience signal justifies grittier narrative (language-explicit press attacks, substance-use narrative in specific contexts, more graphic injury prose). Requires new SPEC decisions-log entry + separate content-pack compilation path.

## Open questions

Deferred to Phase 3+ with trigger conditions named:

1. **Locale-specific content-policy deltas.** Some mature-element thresholds differ by locale — e.g. German-market sensitivity around specific-era historical narrative, Japanese-market sensitivity around specific-character-depiction norms. Phase-7 localization pass surfaces these; per-locale content-policy annex lives alongside per-locale banned-term list. Not blocking for EN-GB EA launch.

2. **Fan-sentiment hostility ceiling calibration.** PEGI 12 permits *"frustrated"* / *"angry"* / *"supporters vented"* — how far into targeted-individual-name hostility can fan sentiment go before it drifts? Current posture: no named-individual targeting beyond *"the manager"* / *"the chairman"* / *"the senior pro"* — no specific player-name-in-fan-sentiment prose. Phase-6 balance pass + first-playtest observation will calibrate.

3. **Press-quote profanity substitution.** Real football press isn't clean. The game's press-quote template pool uses football-native minced-oaths (*"blasted"* / *"unhappy"* / *"let rip"* / *"rollocking"*) without explicit language. Does this read as authentic-enough or as sanitized? Phase-4 closed-itch retention signal + tester feedback will calibrate; if not-authentic-enough, the fix is richer minced-oath vocabulary, not profanity.

4. **AI-content disclosure granularity.** Steam's 2025 policy accepts a per-pack disclosure block; does the game also surface per-entity disclosure anywhere (e.g. *"this player's biography was AI-generated from a prompt"*)? Current default: no per-entity disclosure in UI. Pack-level disclosure in settings / credits screen is enough. Revisit if regulatory or community expectation shifts.

## Prototype gate

Two gates, tied to existing milestones:

**Phase 6 — content-pack v1 content-policy audit.** When content pack v1 compiles (~96 clubs, ~2000-2400 players, ~24 signatures, ~140 commentary templates), a focused content-policy audit pass runs before Phase-7 polish: (a) banned-terms exemption report review (FW-VAL-A-015 / A-016); (b) legal-sensitive-names diff clean (FW-VAL-D-001); (c) AI-content disclosure complete (FW-VAL-D-005); (d) spot-check of ~50 random press-quote + fan-sentiment templates against the scope-in / scope-out lists above; (e) edge-case coverage check — does at least one template in the pool surface each in-scope mature element (dressing-room / ageing-star / relegation / derby) without crossing into out-of-scope territory. Pass = signed-off content-policy checklist; fail = remediation pass before Phase 7.

**Phase 8 — rating-submission readiness.** PEGI 12 / ESRB T questionnaires filled citing this doc. Specific questions (violence / language / substance / sexual / gambling / discrimination / fear / user-generated-content) answered with pack-content evidence. Pass = rating submitted + accepted. Fail = remediation; worst-case slip rating to PEGI 16 / ESRB M requires new SPEC decisions-log entry + pack-compilation-path branching (adult-variant mentioned in §Deferred).

## Cross-references

- **PROJECT_CONTEXT.md §3 Target audience:** PEGI 12 / ESRB T target + mature-element framing
- **CLAUDE.md §1:** project-level rating target
- **SETUP.md §7 Privacy / security posture:** AI-content Steam disclosure metadata commitment
- **`design/worldbuilding.md`:** no-real-places + compiler-only analogues discipline (Caldren fictional nation lock)
- **`design/player-generation.md`:** compiler-generated-only discipline; no real-person likenesses
- **`design/ui-vocabulary.md`:** banned-terms catalog + Category A / B discipline + British-football vernacular
- **`design/modding.md §5 + §Mod-pack content policy`:** closed-registry posture + Workshop content-safety review surface
- **`design/specs/content-pack-validation-contract.md`:** FW-VAL-A-015 / A-016 (banned-terms), FW-VAL-D-001 (legal-sensitive-names diff), FW-VAL-D-002 (region-analogue leakage), FW-VAL-D-005 (AI-content disclosure), FW-VAL-D-010 (Category-B exemption audit)
- **ADR-0006 §LLM path outside byte-identical regeneration:** bake-time-only + human-review gate for AI content
- **ADR-0007 §NarrativeFlag zero-visibility validator:** internal narrative flags never surface in scout output (reinforces compassionate-development-language rule above)

## Changelog within this doc

- **2026-04-24** — Authored as Phase-2 consolidation pass. PEGI 12 / ESRB T rating target locked (from 2026-04-22 bootstrap). Scope-in enumerated (12 in-scope mature themes with shipping-form + example). Scope-out enumerated (13 out-of-scope content categories, sentinel-wrapped to cite by name). Edge-case rulings on 6 drift-prone subjects (derby hostility / managerial dismissal / youth-player setback / discrimination exclusion / injury severity / fan-sentiment hostility). AI-content disclosure locked at Steam-2025 compliance + FW-VAL-D-005 + pack-manifest `ai_content_disclosure` block sketch. Mod-pack content policy closes modding.md open question #2 — banned-terms lint + Steam Workshop report-flow cover EA surface; dedicated in-game report flow deferred post-EA. Two prototype gates at Phase 6 / Phase 8. Four open questions for Phase 3+.

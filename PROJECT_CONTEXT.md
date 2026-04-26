# PROJECT_CONTEXT.md — Final Whistle

> Project pitch, tone, audience, commercial intent, 4-bucket scope split. Read this after `CLAUDE.md` at session start.
>
> Authored 2026-04-22. Revise at each phase transition.

---

## 1. Pitch

**Elevator pitch (one sentence):**
A football management RPG where careers remember — your old decisions return years later as rivals, legends, regrets, and revenge.

**Longer pitch (one paragraph):**
Final Whistle is a football management simulation with RPG-depth progression, running on a deterministic pure-C# match sim, rendered through a renderer-agnostic semantic-cinema viewer (per ADR-0008 ShotPresentationContract; 7-shot-type camera grammar — Phase-3-onward dots-prototype adapter per ADR-0009 sprite-on-pitch viewer; cel-shaded 3D candidate shipping adapter pending Phase-5/6 production-feasibility spike per `design/3d-pipeline.md`). You manage a club in a fully fictional six-tier football pyramid; sign and develop players, shape tactics, and live through season-defining moments. Every consequential event — the youth you sold, the promise you broke, the humiliating derby loss, the cup final gamble — is stored in a persistent event ledger. Years later those events surface as returning rivals, mentoring callbacks, press narratives, and fan sentiment. Depth over breadth: one fictional pyramid modelled deeply, with memory, signatures, and breakthrough moments that make each save a genuinely personal history.

**Problem we're solving / itch we're scratching:**
FM is a spreadsheet-flat depth simulator. FIFA-adjacent managers don't simulate. Blue Lock-style anime-football is player-centric, not managerial. Nobody is building a football manager where the world has memory and players have specific, expressive identities. FM26's rough November launch (UI regressions, feature cuts, modding-tool gutting) created a window where some FM players are newly willing to try a different football-management fantasy — but we don't build reactive to FM; we build a fantasy FM structurally cannot become.

---

## 2. Comparable titles

| Title | What we learn from it | What we do differently |
|---|---|---|
| Football Manager 26 | Depth of sim + UI density expectations; licensed-world anchor | Fictional world, event-sourced memory, signature system, RPG progression ceiling |
| Out of the Park Baseball | Fictional-universe mode as first-class feature proves the audience exists | Football, stylized 2D viewer, anime-inflected signature moments |
| Motorsport Manager (Playsport) | Stylized-but-watchable race engine + tight loops = sustainable indie niche | Football, semantic-cinema viewer, memory/signature systems |
| Giant Killing (anime) | Tone anchor: grounded + mythic, tactical doctrine + dressing-room drama | Fictional pyramid; manager POV throughout; full manager control |
| Aoashi (anime) | Scout-view POV, tactical-x-ray, youth-development weight | Scout Disagreement system; internal gene/development model |
| Clutchtime: Basketball Deckbuilder | Solo dev hybrid proves indie sports-sim + RPG can ship on Steam | Simulation depth instead of deckbuilder loop |
| NUTMEG (upcoming) | Deckbuilder football manager proves the genre hybrid has room | Non-deckbuilder; depth + memory + 2D cinematic viewer |

---

## 3. Target audience

- **Primary audience:** FM-disillusioned players 25-45 looking for a different football-management fantasy; anime-sports-curious 18-30 comfortable with management UX
- **Secondary audience:** OOTP + Motorsport Manager crowd (stylized-sim-friendly), moddable-world enthusiasts
- **Not-for:** FIFA / eFootball arcade-football players (we're not that); licensed-club purists (we're fictional); casual streamers looking for 15-minute loops (match + context takes longer)

**Age rating target:** PEGI 12 / ESRB T. No violence, no explicit content, no substance use. Dressing-room tension and press drama are the mature elements, all language-safe.

---

## 4. Commercial intent

- **Platform:** Steam (PC only). Windows + Mac + Linux builds. Steam Deck Verified deferred post-launch (Linux build exists; cert work later).
- **Price point:** $20 Early Access → $30 1.0. Honest indie RPG-management hybrid pricing, not FM-institutional.
- **Monetization:** premium one-time purchase. Post-launch content as free patches within EA; paid DLC only if 1.0 proves sustainable.
- **Launch target:** EA Month 12 from Phase 1 kickoff. 1.0 target ~Month 18-24 contingent on EA reception.
- **Revenue goal:** break-even on solo-dev opportunity cost within 12 months of EA; one full-time year of runway from EA revenue.

---

## 5. Setting & mood

<!-- ui-lint:ignore-start reason="setting/tone section with explicit banned-term meta-references" -->
**One-paragraph setting summary:**
A fully fictional football world structured with England-readable grammar: a six-tier pyramid, identifiable regional culture, top flight + Championship-equivalent + lower tiers + cup competitions. No real places, no real clubs, no "Manchester-but-not-City" uncanny valley. Fictional nation(s) with credible footballing culture, regional rivalries, promotion/relegation, pyramid politics. All club names, places, player names generated via the AI content compiler at bake time with cultural flavor seeding.

**Tone / register:**
Grounded football first; heightened moments second. Giant Killing + Aoashi + occasional anime exaggeration. The league behaves like real football — injuries, transfers, contracts, relegation, fan sentiment. The game's rhythm is football rhythm. The anime DNA lives in presentation moments: a young striker discovering his move via motion-line-saturated panel cuts; a veteran's last season getting cinematic emphasis; a cup-final signature action captured in impact-frame stylization. Text never says "The Hush." Commentary says "the stadium has gone quiet." Anime is visual, not lexical.
<!-- ui-lint:ignore-end -->

**Visual target** (per 2026-04-26 visual-target supersession decisions-log entry):
Renderer-agnostic semantic-cinema viewer. The 7-shot camera vocabulary (`tactical-wide / diagonal-attack-lane / player-isolation / duel-panel / pass-shot-impact / crowd-reaction / aftermath-freeze`) is renderer-agnostic; stakes + memory-state modulate intensity, paneling, text, and timing through ADR-0008 `ShotPresentationContract`. **Phase-3-onward validation visual:** dots-prototype sprite-on-pitch adapter (ADR-0009) — held to a shippable polish bar (kit discrimination / identity overlays / camera rhythm / readable possession-pressure / signature presentation cues / commentary integration). **Candidate shipping visual:** cel-shaded stylized 3D adapter (ADR-0010 conditional, gated on Phase-5/6 production-feasibility spike per `design/3d-pipeline.md`). Phase-7/8 EA-launch visual locks per spike outcome — three outcomes: spike-green → 3D ships; spike-yellow/red → dots ships if polish bar met (no public 3D promise); dots-not-strong-enough → delay EA. UI aesthetic: football-native dense typography (Anton display / JetBrains Mono data / Rajdhani body); management screens prioritize information density and clarity over FM26's over-clicked navigation. Both adapters share the same UI-Toolkit overlay system + typography stack.

Visual references for the eventual 3D adapter (cel-shaded football aesthetic): Inazuma Eleven (sport expressiveness), Captain Tsubasa Rise of New Champions (anime impact moments), VA-11 Hall-A (typography discipline), Aoashi manga panels (diagonal compositions + motion-line emphasis), Giant Killing manga (tactical-diagram aesthetic). For the dots adapter: classic top-down football-management readability (FM2D as comparable baseline, with semantic-cinema camera-rhythm + identity overlays as differentiator).

**Audio target:**
FMOD-driven crowd layers (anthems, chants, cup-final roar, relegation silence) + contextual match-music stings. Bake-time AI music via Suno or Udio for in-game themes once vertical slice demands it (Phase 6 trigger). No commentary voice-acting at MVP; text commentary only. ElevenLabs evaluated only if post-EA analytics show commentary as top feature request.

---

## 6. Core loop

**What does the player DO in the first 30 seconds?**
Picks a club from the six-tier fictional pyramid (quickstart roster highlights notable archetypes: decaying-giant-tier-2 / rising-academy-tier-3 / backs-against-the-wall-tier-5 / etc.). Sees a stylized club crest + squad portrait + opening-fixture press question.

**What does the player DO in the first 10 minutes?**
First match, through a tutorial layer woven into the viewer itself (not a blocking wall of tooltips). Match ends, post-match report surfaces: a stand-out performance, a press quote, one event logged to the memory ledger.

**What does the player DO in hour 10?**
Managing a full season: tactical iteration, scouting disagreement resolution (if system proves out at Month 4), signature-action unlocks for breakthrough players, memory callbacks surfacing from early-season decisions, promotion/relegation pressure, youth-intake decisions for next generation.

<!-- ui-lint:ignore-start reason="retention-hook prose describing the awakening mechanic" -->
**What's the retention hook?**
The memory ledger + signature system together. Every save accumulates a specific history: the kid you cut who becomes a rival's captain, the derby you threw to save the league, the veteran who awakened a signature in the cup final. These are not generic storylets — they're your specific causality chain. Players come back because their save has a memory nothing else has.
<!-- ui-lint:ignore-end -->

---

## 7. Scope estimate — Product MVP (EA target, Month 12)

**Content volume:**
- 1 fictional six-tier football pyramid (~96 clubs total, squad size 20-25 → ~2000-2400 players in active universe + youth reserves)
- 24 pre-authored signatures (3 per role family × 8 role families)
- 20-30 manager archetypes (behavior-tree authored; rival-manager ecosystem)
- 5-8 salience-gated narrative events per season
- 7-shot-type semantic cinema vocabulary with stakes/memory modulation
- Full season calendar (league + cup + promotion/relegation)
- Save/load with schema versioning + content-pack-qualified IDs

**Hours of engagement (target):**
- 30-hour season that feels like a story
- 200-hour careers feasible but not required

**Localization:**
- English at EA launch
- JP / ES / PT / DE as Phase 7 targets

**Deferred (not in EA):**
<!-- ui-lint:ignore-start reason="scope-out list enumerating deferred / filed-indefinitely mechanics by name" -->
- 3D match engine — candidate shipping visual gated on Phase-5/6 production-feasibility spike per `design/3d-pipeline.md` (per 2026-04-26 visual-target supersession; supersedes original "post-audience-signal" deferral)
- Coaching Lineage surfacing (data seeded, exposure post-MVP)
- Counterfactual Development Lab
- Multi-nation pyramid
- Manager Archetype Forge (content-scaling tool)
- Physical Load as Narrative Debt polish
- Workshop / mod editor UX (data architecture ready; UX post-EA)
- Bloodline / lineage mechanics (filed indefinitely)
<!-- ui-lint:ignore-end -->

---

## 8. The 4-bucket scope split (MVP discipline)

Every proposed feature belongs in exactly one bucket. Features that don't fit get cut or deferred. This discipline is what stops the project from collapsing under good ideas.

### A. Product MVP — what the Steam page promises

1. Deterministic MatchSim (pure-C#, fixed-point canonical state)
2. Renderer-agnostic semantic-cinema viewer (per ADR-0008 ShotPresentationContract) with minimum Semantic Cinema (7 shot types). Phase-3-onward dots-prototype adapter (ADR-0009) is shipping-quality candidate; cel-shaded 3D adapter (ADR-0010, conditional) is candidate shipping visual gated on Phase-5/6 spike per `design/3d-pipeline.md`. EA visual locks per spike outcome.
3. Event-sourced Career Memory (single ledger)
4. Signature actions (24 pre-authored, 3 per role family, football-native UI copy)
5. Breakthrough moments (match-flow cinematic, not pause-QTE)
6. Scout Disagreement (conditional — Month 4 feel prototype gate decides MVP inclusion)
7. Clean, fast management UI

### B. Architecture from day one — invisible but load-bearing

- Stable IDs (content-pack-qualified)
- Schema versions
- Content packs
- Save migrations
- Replay seeds per match + highlight
- Memory event log + compaction strategy
- Mod-ready data constraints (editor UX deferred)
- Fixed-point canonical state in MatchSim; floats only in viewer interpolation

### C. Dev pipeline — not Steam-facing

- AI Content Compiler (prompt/spec → JSON → lint → sim sanity → content pack → import)
- Player Identity Compiler
- Validation + lint + sanity checks
- Balance harness (Claude-assisted, human-approved)

<!-- ui-lint:ignore-start reason="deferred-bucket list enumerating banned/deferred mechanics by name" -->
### D. Deferred — seeded now, exposed later

- Coaching Lineage surfacing
- Counterfactual Development Lab
- Manager Archetype Forge
- Physical Load as Narrative Debt
- Workshop / mod editor UX
- 3D match engine — candidate shipping visual via spike-gated path per `design/3d-pipeline.md`
- Multi-nation pyramid
- Named capitalized "state" vocabulary (never shipping)
- Bloodline / genetic-lineage mechanics
<!-- ui-lint:ignore-end -->

---

## 9. Non-goals (explicit scope boundaries)

- Licensed real clubs, players, leagues, kits
- Single-player story-driven campaign with scripted cutscenes
- Real-time action gameplay (player-controlled on-pitch moments)
- Social-media integration / live ops / server-side anything
- Mobile port (deferred indefinitely; revisit post-1.0 only)
<!-- ui-lint:ignore-start reason="Non-goals list explicitly naming banned vocabulary" -->
- Named mystical RPG-vocabulary UI ("The Hush", "Weather", "Calling", "Canon", "Seven" — all banned as visible system names)
<!-- ui-lint:ignore-end -->

---

## 10. Risk register (top risks at bootstrap)

| # | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| 1 | 2D match viewer fails to feel emotionally legible → Month-3 gate missed | Medium | Critical | Month-3 explicit gate; fall back to simpler shot grammar or extend Phase 3 by one cycle; do not proceed to later Phases until gate passes |
| 2 | Scout Disagreement prototype feels like spreadsheet noise at Month 4 | Medium | High | Gate criterion clear; drop to "scout uncertainty" simpler system if prototype fails |
| 3 | Deterministic cross-platform sim breaks via floating-point drift | Medium | High | Fixed-point canonical state; CI matrix tests Windows/Mac/Linux; no Unity physics in canonical path |
| 4 | Content-pack schema locks us out of modding infrastructure late | Low | High | Phase 0 Modding ADR constrains every system; editor UX deferred not blocked |
| 5 | Solo-dev burnout | High | Critical | Phase-gate discipline; month-by-month audience checks create external accountability; scope cuts aggressive |
| 6 | FM26 recovers via patches → community stops looking for alternatives | Medium | Medium | We don't build reactive. Our moats (fictional world + memory + signatures + stylized 2D) remain valid regardless |
| 7 | Steam algorithm doesn't surface stylized-football-manager niche | Medium | High | Pre-EA community building via devlog clips Month 2-3; closed itch beta Month 4-6; Next Fest gate Month 8 |
| 8 | AI content compiler lint catches too much / too little → shipping slop or hand-authoring creep | Medium | Medium | Compiler architecture explicit; lint rules reviewed at Phase 4 gate |

Revise at each phase transition.

---

## 11. Audience-signal gates (month-by-month)

- **Month 2-3:** devlog clips of 2D viewer prototype shared publicly. First external audience signal.
- **Month 3:** match-engine gate — *"Can a stranger watch a 2D match for three minutes and understand drama, momentum, and player identity without reading a design doc?"*
- **Month 4:** Scout Disagreement feel prototype gate — *"Does disagreement create interesting decisions, or does it just obscure truth?"*
- **Month 4-6:** closed itch build with trusted testers.
- **Month 6:** public demo — only if retention metrics from closed itch are sane.
- **Month 8:** Steam Next Fest — only if first 10 minutes are sharp.
- **Month 12:** EA launch.

---

*Authored 2026-04-22. Updated at each phase transition.*

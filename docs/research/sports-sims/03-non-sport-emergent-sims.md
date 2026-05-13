# Non-sport emergent-behavior sims — research notes

**Researched:** 2026-05-13
**For:** Final Whistle T1-2b (BT runner) + T3 (memory + breakthroughs)

## Sources

- [Q&A: Dissecting the development of Dwarf Fortress with Tarn Adams (Game Developer)](https://www.gamedeveloper.com/design/q-a-dissecting-the-development-of-i-dwarf-fortress-i-with-creator-tarn-adams) — Adams on "collaboration with the player to produce stories." [primary]
- [Emergent Narrative in Dwarf Fortress — Tarn Adams chapter (Procedural Storytelling in Game Design, CRC)](https://www.taylorfrancis.com/chapters/edit/10.1201/9780429488337-15/emergent-narrative-dwarf-fortress-tarn-adams) — Adams's own taxonomy of emergent narrative. [primary]
- [DF Wiki — Personality facet](https://dwarffortresswiki.org/index.php/Personality_facet) — 0–100 facets, species medians, effect on thought/need triggering. [secondary]
- [DF Wiki — Thoughts and preferences](https://dwarffortresswiki.org/index.php/DF2014:Thoughts_and_preferences) — memory-driven facet drift; needs lower focus until met. [secondary]
- [CK3 Wiki — Traits](https://ck3.paradoxwikis.com/Traits) — trait → (Boldness, Energy, Rationality, Greed, Honor, Sociability, Compassion, Vengefulness) deltas, capped ±100. [secondary]
- [CK3 Dev Diary #58 — Stress and traits (Paradox)](https://forum.paradoxplaza.com/forum/developer-diary/ck3-dev-diary-58-stre-ss-tching-the-traits.1472092/) — stress as the cost of acting against trait. [primary]
- [Tynan Sylvester — Designing Games (O'Reilly, 2013)](https://tynansylvester.com/book/) — the "engineering experiences" framing; mood/memory as story-engine inputs. [primary]
- [GDC Vault — RimWorld: Contrarian, Ridiculous, and Impossible Game Design Methods](https://www.gdcvault.com/play/1024232/-RimWorld-Contrarian-Ridiculous-and) — Sylvester on storyteller-first design. [primary]
- [RimWorld Wiki — AI Storytellers](https://rimworldwiki.com/wiki/AI_Storytellers) — Cassandra's cooldown/wealth curve. [secondary]
- [Mark Brown — The Genius AI Behind The Sims (GMTK)](https://gmtk.substack.com/p/the-genius-ai-behind-the-sims) — smart-object advertising, top-N random pick, trait weights. [secondary]
- [Bourse — AI in The Sims series (ENS report)](https://yo252yo.com/old/ens/sims-rapport.pdf) — academic walk-through of the utility loop. [secondary]
- [Tirrell — Dumb People, Smart Objects (Philosophy of Computer Games)](https://www.gamephilosophy.org/wp-content/uploads/confmanuscripts/pcg2012/Tirrell%202012%20-Dumb-People-Smart-Objects-The-Sims-and-the-Distributed-Self.pdf) — distributed-intelligence framing. [secondary]

## Per-game findings

### Dwarf Fortress
- Personality decomposed into **facets** (0–100 scalars, e.g. ambition, anger-propensity, altruism) + **values** (tradition, cooperation, sacrifice) + **goals**. Each is independent; combination yields ~10^N distinct profiles without enumerating them.
- Species set **medians**, then per-individual jitter. Dwarves median 55 greed; goblins median 25 altruism (capped 50). Same code, different priors per culture.
- **Thoughts** are timestamped memories from interactions; they decay but can re-fire facet drift. Needs (worship, craft, drink-with-friends) are derived from values, not hand-authored per dwarf — when unmet, focus drops, productivity drops, tantrum risk rises.
- Adams frames the game as a "writing prompt … collaboration with the player." Behavior is intentionally under-determined; the player narrates the why.

### Crusader Kings 2/3
- Personality traits are **stat-deltas on a small fixed AI-attribute vector** (Boldness, Energy, Compassion, Greed, Honor, Rationality, Sociability, Vengefulness), each capped ±100. Brave: +200 Boldness, −20 Rationality. Craven: the mirror.
- Decisions and schemes carry per-trait `ai_will_do` modifiers (Playful → +35 on Playdate). Composition is **additive**, not combinatorial — N traits × M decisions, not N! tables.
- **Stress** is the pressure-release valve: acting against your personality traits costs stress; enough stress triggers a mental break that can flip traits. Personalities can change, but only with story friction.
- Opposite-trait characters have lower mutual Opinion → diplomacy emerges from the same scalar field that drives decisions.

### RimWorld
- Pawns score actions by a **utility-like job giver** stack, gated by needs (food/rest/recreation/comfort/joy). Needs decay; below thresholds they outweigh chores.
- **Mood** is a unified scalar aggregating need satisfaction + per-event "thoughts" (timestamped, decaying memories: saw-corpse, ate-without-table, witnessed-death-of-friend). Below thresholds it triggers mental breaks (daze, berserk, insulting-spree).
- **Traits + backstories** bias mood deltas and unlock/lock work types ("bloodlust" → +mood from kills; "kind" → opposite). Combinatorial space is huge but each trait is a small additive patch.
- **AI Storyteller** (Cassandra/Phoebe/Randy) is a directed-difficulty layer above the sim: tracks wealth + colonist count + cooldowns and chooses *when* to fire raids/disease/quests. The sim is the substrate; the storyteller is a pacing meta-agent. Sylvester's GDC pitch: design the storyteller first, the colony second.

### The Sims
- **Smart-object advertising**: objects broadcast utility offers ("fridge: +5 hunger at distance d"); Sims do not enumerate what objects do. New object = new advertisement; no AI changes.
- Selection: each option scored as `advertised_delta × need_curve(current_level) × distance_falloff × personality_weight`, then pick from the **top N at random** to dodge robotic determinism.
- Need curves are non-linear (Maslow-inspired): low hunger spikes sharply; fun/social only matter once physiology is satisfied.
- Personality traits (playful, neat, outgoing…) are per-need multipliers — a playful Sim's `fun` weight is just higher.

## Cross-cutting patterns

1. **Personality = small scalar vector that biases a generic scorer.** Not a per-character behavior tree. All four games. Avoids combinatorial blow-up and makes traits composable.
2. **Top-N random pick** (Sims) and **cooldowns + medians** (DF/CK3) prevent the "every agent does the same thing" failure when scorers happen to agree.
3. **Memory as decaying timestamped thoughts** (DF, RimWorld) feeds back into scoring — yesterday's funeral is today's mood penalty is next month's facet drift.
4. **Architecture choice:** all four lean **utility AI** (or hybrid utility + reactive jobs), not pure BTs or HTN. BTs control *how* an action plays out; utility chooses *which* action. HTN appears only in heavily-scripted games (F.E.A.R., Horizon) — none of these.
5. **Meta-director layer** (RimWorld storyteller) sits above the sim and shapes pacing without touching pawn logic. A clean separation worth copying.
6. **Believability ≠ realism.** Adams + Sylvester both: the player fills in the why; the sim only has to be *consistent enough* to support the inference.

## Direct application to Final Whistle

**For T1-2b (22-player BT runner):**
- Use BTs for **how** (close-down, cover-shadow, third-man-run, press-trigger) but layer a **utility scorer at the role-frame entry** for **which** action — exactly the Sims pattern. The BT is the verb; utility picks the verb.
- Personality stays as a small scalar vector on each player — e.g. (Aggression, Composure, FlairBias, WorkRate, Selflessness) — feeding multiplicative weights on candidate actions. Avoid per-archetype BTs; bias the shared BT instead. Matches CK3's `ai_will_do` discipline.
- Top-N random pick (seeded by `(match_seed, tick, event_id)`) prevents 22 players converging on the same scored action. Determinism preserved; sameness avoided.
- Form / momentum / signature_readiness are mood-analogues — scalar accumulators that bias scoring, surfaced as commentary not numbers.

**For T3 (career memory + breakthroughs):**
- Model `MemoryEvent` as **timestamped thoughts** with salience that decays but can re-fire. RimWorld's "saw-friend-die" is structurally our "missed-penalty-vs-rivals." The ledger is append-only; salience is the read-side computation.
- **Breakthroughs as facet drift**: a player whose Composure was 60 and who scored a 92nd-minute winner gets a salience-weighted nudge. Don't promote XP — promote a small permanent shift in the bias vector, exactly DF's "memories can change facets" rule.
- A **director layer** like Cassandra is the natural home for "season narrative pacing" later (T6+): when the league has been quiet, fire a derby controversy; when chaotic, give breathing room. Defer to T6, but architect T1–T3 so director hooks exist (read-only ledger queries, no canonical-state writes from the director).
- Scout disagreement (pillar 4) maps cleanly onto **biased facet observation**: each scout sees `true_facets + scout_bias_vector + noise`. Truth emerges over seasons of repeated observations. Same primitive as DF/CK3 trait detection.

## Open questions

- **Top-N random pick vs. softmax**: random-from-top-3 is cheap and Sims-proven; softmax is smoother but costlier per tick at 60 Hz × 22 agents. Bench at T1-2b.
- **Where does the utility-scorer table live in `fw-content`?** Per-action weight tables suggest a `tactical_actions.ron` registry alongside archetypes. Decide before T1-2b implementation begins.
- **Cooldowns on player decisions** (CK3 Energy, Cassandra-style): do we need a per-player "consideration tick" rate, or is 60 Hz scoring fine? Cheaper sim says yes to cooldowns.
- **Mood-equivalent breakdowns**: RimWorld's mental breaks are dramatic. Football analogue is the visible meltdown / red-card lash-out / silent-treatment. Worth a dedicated breakdown table at T3, or fold into existing event surface?
- **Storyteller vs. fixture list**: real football has a fixed calendar; the director has less freedom than Cassandra. Probably director picks *which subplot* surfaces in commentary, not *which match*.

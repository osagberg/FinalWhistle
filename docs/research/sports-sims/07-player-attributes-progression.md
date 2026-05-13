# Player attributes + progression — research notes

**Researched:** 2026-05-13
**For:** T1-1 schema + T2-4 player generation + T3-4 breakthroughs

## Sources

- [FM24 Player Attributes Explained — sortitoutsi](https://sortitoutsi.net/content/67538/fm24-guide-players-attributes-explained) [primary]
- [FM Guide: Hidden Attributes — footballmanagerblog.org](https://www.footballmanagerblog.org/2024/09/football-manager-guide-hidden-attributes.html) [primary]
- [Current & Potential Ability — FMInside](https://fminside.net/guides/basic-guides/76-current-potential-ability) [primary]
- [CA/PA + Star Ratings — sortitoutsi](https://sortitoutsi.net/content/67526/current-ability-potential-ability-and-star-ratings-in-football-manager) [primary]
- [FM24 Age Curve — makefootballgreat](https://makefootballgreat.com/football-manager/recruitment-methods/fm24-age-curve/) [secondary]
- [When Do Football Players Peak — Macro-Football](https://macro-football.com/other/aging/) [secondary]
- [The Aging Curve in Elite Football — PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC12551122/) [primary, academic]
- [OOTP Current vs Potential Ratings — OOTP Manual](https://manuals.ootpdevelopments.com/index.php?man=ootp19&page=ratings_overview) [primary]
- [OOTP OVR + POT Ratings — OOTP Manual](https://manuals.ootpdevelopments.com/index.php?man=ootp19&page=overall_rating) [primary]
- [OOTP Player Development — OOTP Manual](https://manuals.ootpdevelopments.com/index.php?man=ootp19&page=player_development) [primary]
- [NBA 2K MyLeague Player Potential — Operation Sports forum](https://forums.operationsports.com/forums/nba-2k-basketball/976493-myleague-player-potential.html) [secondary]
- [NBA 2K Overall Rating — nba2k Fandom](https://nba2k.fandom.com/wiki/Overall_Rating) [secondary]
- [Front Office Football 2007 Player Guide — Solecismic](http://www.solecismic.com/support/FOF2007PlayerGuide.pdf) [primary]
- [Madden 26 Franchise Deep Dive — EA](https://www.ea.com/games/madden-nfl/madden-nfl-26/news/madden-26-gridiron-notes-franchise-deep-dive) [primary]
- [CK3 Traits — ck3 wiki](https://ck3.paradoxwikis.com/Traits) [primary]
- [FM21 Condition + Fatigue — realsport101](https://realsport101.com/football-manager/football-manager-21-condition-fatigue-sharpness-changes-tooltip-injury-risk/) [secondary]

## Per-game findings

### FM attribute system

36 visible attributes per outfield player on a 1–20 scale: **14 technical, 14 mental, 8 physical**. Goalkeepers swap technical for 13 GK-technical. Plus ~14 hidden attributes routinely cited in community work — six headliners (Consistency, Dirtiness, Important Matches, Injury Proneness, Loyalty, Pressure) plus less-discussed ones (Versatility, Adaptability, Ambition, Professionalism, Sportsmanship, Temperament, Controversy). All hidden attributes are 1–20 too; only the editor reveals them. The visible totals plus hidden hover near **~56** — that's the FM ceiling people quote.

**Current Ability (CA) and Potential Ability (PA)** are aggregate 0–200 scores. CA is a weighted sum of attributes (the weights depend on position — Pace costs more CA for a striker than a centre-back). PA is a fixed ceiling. CA can rise toward PA only under development conditions (training quality, manager attention, the player's own Determination/Ambition/Professionalism). Players under 21 get a **range PA** (e.g. `-7` means "150–180, rolled at runtime") so the value isn't fully determined at gen.

### FM aging curves

FM development effectively ends around 23–24; players then plateau briefly, peak around 27–29, decline noticeably from 30–32, fall off a cliff at 33–35. Physical attributes (Pace, Acceleration, Stamina, Strength) decline first and sharpest; mental attributes (Decisions, Positioning, Composure) can still tick up into the early 30s; technical attributes hold roughly flat through the peak then drift down.

Real-world studies broadly agree: physical peaks at ~25–26, peak overall around 27, with wingers peaking earliest (~26) and centre-backs latest (~28). FM's curves track that within tolerance.

### OOTP ratings + scouting-ratings split

OOTP keeps **true ratings** (the canonical hidden values) and **scouting ratings** (your scout's perceived values) as separate stores. The scout-perception layer is on by default; the UI shows scout values unless you disable it. Each rated skill carries **both** current and potential variants — Contact and Contact Potential, etc. Scouts are explicitly less accurate at potential for younger players. Scales are configurable (20–80 scout-grade, 1–100, or 1–250).

Aging is a multiplier-based curve with adjustable defaults: development ends ~25, decline starts ~30. Per-position decline rates apply; catchers age fast, pitchers degrade differently from hitters.

### NBA 2K / Madden franchise mode hidden potential

NBA 2K MyLeague: each player has a visible **Potential** rating, plus a **hidden development archetype** (Normal / Likely Boom / Likely Bust / Boom / Bust / Boom-to-Bust) that adjusts how aggressively they trend toward potential vs. away from it. The archetype is hard-coded at generation and largely invisible to the player.

Madden 25/26 layers **dev traits** (Normal/Star/Superstar/X-Factor) plus **breakout storylines** — performance triggers (sack totals, 100-yd rushing games) that unlock attribute boosts via cinematic moments. Madden 26 is also adding coach archetypes (Offensive Guru / Defensive Genius / Development Wizard) that modulate player growth.

### Front Office Football

FOF shows scouting-derived attribute **ranges** in colored blue bands rather than point values; better scouts deliver tighter bands. Scouts have per-position strengths (one is great at linebackers, another at running backs). Interview budget (60 players pre-draft) lets you gather "underrated/overrated" tags. The model is conceptually closest to what FW needs: explicit uncertainty surfaced as a range, not a point estimate.

### Crusader Kings hidden traits (transferable insights)

CK3 separates **active** traits (visible, mechanical effects) from **inactive** ones (treated as recessive genes — invisible, no effect, but inheritable). Trait inheritance rolls active-first, falls back to inactive. The lesson: hidden things still need rules to exist, propagate, and occasionally become visible. The cinematic "reveal" of a previously-hidden trait is a content moment, not a stat reveal.

## Cross-cutting patterns

- **Visible/hidden split:** ~3:1 visible to hidden across the board (FM 36:~14, OOTP doubled current/potential pairs, NBA 2K visible-overall + hidden archetype). Hidden attributes own the things you can't observe directly from match output: motivation, ceiling, injury luck, locker-room behavior.
- **Aging curve shape:** asymmetric. Long ramp-up to peak (5–8 years), short peak (2–3 years), gentle decline (4–6 years), cliff. The cliff is position-dependent.
- **Form/morale/fatigue:** layered as short-term multipliers on top of base attributes. FM separates **Condition** (per-match fitness, recoverable in days) from **Fatigue** (multi-week load, recoverable in weeks) from **Sharpness** (match-rust on returning players) from **Morale** (locker-room state). Each acts independently on different sim subsystems.
- **CA vs PA:** the universal pattern. Current is what you have; Potential is the ceiling. The interesting design question is *what shape Potential takes* — a fixed point (FM PA), a fixed point with a hidden archetype that modulates trajectory (NBA 2K), or a range that resolves at runtime (FM young-player range PA).

## Direct application to Final Whistle

- **T1-1 schema — start small, leave room.** ~24 visible attributes (8 technical, 10 mental, 6 physical) on a 1–20 scale is enough to differentiate. Plus 8 hidden (Determination, Ambition, Loyalty, Professionalism, Injury Proneness, Big-Match Temperament, Consistency, Versatility). All `Q32` because BT-runner formulas multiply them. **Visible/hidden ratio ~3:1, total ~32** — comfortably under FM's 56 without feeling thin. Add a separate `signature_readiness: Q32` for the breakthrough hook (T3-4).
- **T1-2b BT runner consumption — name the formula.** Shooting decision reads `finishing + composure + (technique * pressure_modifier) + form_bonus`. Each BT decision should cite 2–4 attributes by name plus exactly one mental/physical short-term modifier. Document the per-action attribute set in `docs/specs/bt-attribute-binding.md`.
- **T2-4 generation — region-conditioned, role-conditioned, normal-distributed.** Sample CA from a normal distribution conditioned on club tier (Premier League regen ~ N(140, 18), League Two ~ N(80, 12)). Distribute CA across attributes via role-weighted Dirichlet so a striker gets more in Finishing/Pace and a centre-back more in Jumping/Tackling. Hidden attributes sample independently — uniform 5–20 with cultural priors (e.g., region X biases Determination higher). Markov-name correlation is fine but secondary to role-weighting.
- **T3-4 breakthroughs — diverge from XP grind.** Shipped sims (Madden, NBA 2K, FM) all bolt cinematic breakouts onto a linear XP base. FW's pillar inverts that: **growth lives in the ledger first**. A breakthrough is a `MemoryEvent::Breakthrough { player, trigger_event, attribute_set, delta_ca }` that (a) redraws PA — not just CA — by sampling new values for 2–4 specific attributes, (b) only fires when ledger salience clears a threshold across recent events. No daily/weekly tick boost. Decline still ticks down with age, but the "rise" is event-driven and rare (~2–4 per career arc).

## Open questions

- Do we want a CA/PA equivalent at all, or just compute "rating" from attribute aggregation on read? PA as a fixed cap is design-debt — a breakthrough lifts the cap, but lifts to what? Either lift-by-delta (PA += delta) or remove PA entirely and trust the salience-gated breakthrough system to bound growth.
- Hidden attributes — surfaced by scout disagreement (T2-7), or only via ledger surfacing? Probably both, but the contract needs to be explicit before T2-4.
- Form/morale/fatigue: how many layers? FM ships four; OOTP fewer. FW's text-first presentation suggests two is enough (Condition for per-match, Form for multi-week) — but BT runner needs the contract before T1-2b.
- Does FW's region-conditioned generation need a separate `RegionPriors` table per content pack, or does the existing content-pack schema cover it? Likely the former — flag for T2-4 design.

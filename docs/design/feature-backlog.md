# Feature backlog — management depth (the breadth fun-axis)

> **Status: living backlog, NOT a commitment.** This catalogues candidate
> MANAGEMENT-DEPTH features for Final Whistle and ranks them by fun-leverage ×
> pillar-fit × effort. Rows here are research-sourced candidates feeding the
> roadmap conversation; nothing in this doc is scheduled until it lands as a
> `T*` row in `docs/MASTER_PLAN.md`. It respects the `docs/DESIGN_DOC.md` §8
> ruled-out list (no real licensed data, no 3D, no multiplayer, no mobile, no
> runtime LLM) and favours features that exploit FW's own moat — determinism,
> the procedural world, the append-only memory ledger, and scouting uncertainty
> — over generic manager-game clones.

---

## 1. Method + how to read this doc

### Two fun-axes

Final Whistle's fun lives on two orthogonal axes:

1. **Match-feel + season-arc (depth-of-moment).** Is a single match gripping?
   Does a season pull you forward? Does a callback land as earned? This axis is
   owned by the **Tier F** track (`FUN-0..5`, `FUN-H1`, `DX-2`) and its design
   docs: `docs/design/drama-model.md`, `docs/design/fun-evaluation-harness.md`,
   `docs/design/match-quality-inspection.md`. **This backlog does NOT cover that
   axis** — it cross-references it (see §5) and feeds `FUN-5` (decision
   satisfaction), which explicitly waits on this research.

2. **Management depth (breadth-of-decision).** How many meaningful, non-trivial
   decisions does the manager get to make off the pitch — tactics prep,
   recruitment, youth, transfers, squad management, competition strategy, club
   operations, career arc? **This is the axis this doc catalogues.** A deep
   match engine with a thin management layer is a tech demo; the breadth axis is
   what turns a watchable match into a career worth playing for ten seasons.

The two axes multiply. A great match means little if the squad you field, the
prospect you gambled on, and the rival you've never beaten don't carry weight.

### Scoring

Each candidate carries:

- **fun rationale** — the specific decision/story it unlocks, in one line.
- **pillar** — which of the five DESIGN_DOC pillars it serves (1 procedural
  world, 2 careers remember, 3 breakthrough-driven dev, 4 scouting uncertainty,
  5 signature identity; `new` = genre-baseline depth not tied to a pillar).
- **fwStatus** — `missing` (no scaffolding) / `partial` (some backend exists) /
  `planned` (a roadmap row owns it) / `done` (shipped).
- **effort** — S / M / L / XL, relative to the FW codebase as it stands.
- **leverage** — fun-leverage × pillar-fit, the headline ranking signal:
  `high` / `medium` / `low`.
- **EA/post-EA** — earliest sensible window. Most depth is post-EA; the EA floor
  is the procedural world (T4.5) + the five wired pillars + Tier F match-feel.

### How to act on it

Read **§2 (Top candidates)** first — that's the shortlist the owner acts on.
Then **§3 (Table-stakes gaps)** — the genre-baseline holes FW must fill before
any depth feature matters; these are the highest priority of all. The
**§4 per-dimension tables** are the full de-duplicated catalogue. **§5** maps
candidates that already have roadmap rows so they aren't double-counted.

A guiding bias throughout: prefer the feature that only FW can do. Anyone can
build a transfer market; only FW can have a scout whose 5-season report history
on a player you remember the day you first watched, surfaced as a press callback
when he scores against you. Lean into the ledger, the uncertainty, the
determinism, and the per-save procedural world.

---

## 2. Top candidates (the shortlist the owner acts on)

Ranked by fun-leverage × pillar-fit × reasonable effort, EA-first where it fits.
These are the features that buy the most career-depth per unit of work AND lean
hardest on FW's moat. Items already owned by a roadmap row are in §5, not here.

| # | Feature | Pillar | Effort | Leverage | Window | Why it's top |
|---|---|---|---|---|---|---|
| 1 | **Morale driver system + career-satisfaction score** | 2,3 | M | high | post-EA | The connective tissue for half this backlog. Morale = transparent weighted sum of form, results, playing-time, loyalty, unbroken-promises — all ledger-sourced, no magic. Gates transfer requests, breakthrough triggers, dressing-room knock-on. Nothing else unlocks as much downstream depth. |
| 2 | **Tactical familiarity + cohesion ramp** | 3 | S | high | **EA** | Cheapest high-leverage win in the whole doc. New formation/lineup starts at ~0.5 cohesion, ramps per shared start; multiplies decision-confidence + signature/set-piece success. Lightweight formula, outsized "the team finally gelled" narrative. Wires into the existing tactic-FSM + utility scoring. Plausible EA stretch. |
| 3 | **Cumulative match-load + fatigue decay** | new | M | high | post-EA | The data-layer foundation for rotation, fatigue, injury return-to-play, and fixture congestion. Without it, condition fields (`match_fitness`, `sharpness`) stay static and four other features can't exist. MVP is just the ticker + decay; UI follows. |
| 4 | **Injury progression + return-to-play gates** | 3 | L | high | post-EA | Most relatable drama in football. Injury → recovery ticker → return at reduced fitness → re-injury risk if rushed. A return-from-long-term-injury is a breakthrough gate. Builds on `InjuryLongTerm` event-class + the match-load tracker (#3). |
| 5 | **Board confidence dynamics** | 2 | M | high | post-EA | Persistent confidence state that decays on bad results, restores on signings/trophies; gates sacking, wage caps, transfer mandates. The strategic tension between board patience and a long rebuild. Ledger-tracked so it surfaces as callbacks ("the board never forgot that 6-0"). |
| 6 | **Opposition tactical analysis (scout report on shape/press/set-piece danger)** | 4,1 | L | high | post-EA | Turns the pre-match into a tactical puzzle: facing a high press is a different problem from a low block. Post-match computable from the ledger + match-stats; minimal new sim data. A genuinely FW-native scouting output (opponent patterns, not player attributes). |
| 7 | **Manager reputation gradient (0–100, ledger-derived)** | 2,5 | M | high | post-EA | The manager counterpart to player breakthrough arcs — "the weight of my own history." Read-only projection from event salience; decays slowly so failures linger. Feeds press tone, signing leverage, board patience. Foundation for the whole career-arc dimension. |
| 8 | **Mentoring system (senior→youth pairing)** | 2,3 | M | high | post-EA | The prospect-blooming-under-a-veteran story. Pairing lifts the youth's relevant readiness + morale; mentor earns Leadership readiness. `MentorTeammate` event already weighted in the family-relevance table; multi-generational arcs emerge. |
| 9 | **Shortlist / watch-list mechanics** | 4,2 | S | high | post-EA | Low effort given the existing scouting backend. Flag a prospect, scouts feed reports over seasons, the ledger remembers the 3-year journey ("remember when we first watched this lad?"). Pure leverage-per-effort. |
| 10 | **Team instructions (press height, tempo, defensive shape, counter speed)** | 1,5 | M | high | post-EA | The persistent tactical-identity foundation players execute within. Pre-match sliders seed `ArchetypeParams`; the tactic-FSM + per-player BT already consume these at match-time — this makes them a durable team setup, not just a mid-match toggle. |

**Honorable mentions** (high leverage, slightly further out or thinner): captain
role + dressing-room influence (5); transfer market search + contract negotiation
(the transfer dimension's table-stakes spine, §3); academy intake pipeline (1,2);
in-match individual instructions (5,1); manager-vs-manager rivalry callbacks (2,5,
near-S effort, EA-plausible as a pure read-projection over the existing ledger).

---

## 3. Table-stakes gaps (highest priority — genre baseline FW lacks)

These are not "nice depth" — they are the floor any football-manager game stands
on. Until they exist, the depth features in §2/§4 have nothing to attach to. They
are the highest priority in the doc. Several are partly owned by roadmap rows
(noted); the rest are gaps with no row yet.

### Tactics + match preparation

- **Team instruction system** (press-trigger height, passing tempo, defensive
  shape, buildup style) — partial: the tactic-FSM exists with 5 states and
  `ChangePressLevel`/`ChangeTempoBias` at match-time, but there is no persistent
  pre-match team-setup screen. See §2 #10.
- **Individual player instructions** (positioning, aggression, marking,
  pass-bias) — missing. `MatchCommand` has no per-player instruction variant.
- **Player role variants within a formation** (defensive vs attacking winger,
  etc.) — partial: role-affinity tables + formation-change exist; the selector is
  formation-only and a `role_variant` field + BT dispatch are needed.
- **Opposition analysis focused on tactical weaknesses** (not just player
  attributes) — missing. See §2 #6.
- **In-match tactical flexibility** beyond formation swaps — missing.
- **Training routines** (any kind) — missing. No training system exists at all.

### Scouting + recruitment

- **Multi-scout comparison + disagreement, weighted by track record** — planned
  (T4-2.5m/n/p, Deferred / conditional-EA). Single-scout uncertainty (T4-2.5f) is
  the shipped EA floor.
- **Scout specialization archetypes with visible skill profiles** — partial
  (archetype enum + personality bias partially wired; not surfaced).
- **Shortlist / watch-list management** — missing. See §2 #9.
- **Scout hiring / retirement / performance tracking** — missing.
- **Lower-league discovery** (scouts reporting from tiers 4–6) — missing; unlocked
  by the T4.5 6-tier pyramid.
- **Attribute-certainty tiers** (observation count → confidence band) — partial:
  bands are per-observation noise today; observation-count-driven narrowing is the
  gap.

### Youth + player development

- **Youth academy intake pipeline + procedural newgens** — missing (the procedural
  generator is T4.5-E1; the academy layer that consumes it is the gap).
- **Training system with coaching assignment** — missing.
- **Loan system for development** — missing (loans land T5+).
- **Mentoring / senior influence** — missing. See §2 #8.
- **Visible-potential vs hidden-ceiling age curves** — partial: the Identity
  Packet (T4.5-E0) encodes peak-age + aging curve; scouting projection exists; the
  age-narrowing visibility is the gap.
- **Academy reputation** tied to youth-product success — missing.

### Transfers + club economy

- **Transfer market UI + player search/filtering** — missing. The single biggest
  table-stakes hole; the spine the whole transfer dimension hangs on.
- **Contract system** (expiry, renewals, wage negotiation) — partial: basic
  renewal accept/reject scaffolding; no negotiation, wages, or expiry-driven free
  transfers.
- **Club budget constraints + financial sustainability** — missing.
- **Transfer windows with deadlines** — missing.
- **Board financial expectations + revenue mechanics** — missing (board confidence
  is §2 #5; the financial half is here).
- **Agent / negotiation mechanics** — missing.

### Squad dynamics + condition

- **Team cohesion / synergy** — missing (the cheapest high-value fix; §2 #2).
- **Injury management + recovery curves** — partial (event class exists; zero
  progression logic; §2 #4).
- **Rotation + rest management** — missing (depends on match-load; §2 #3).
- **Dressing-room hierarchy + captaincy** — missing (Leadership family exists; no
  hierarchy or captain mechanic).
- **Transfer requests + career satisfaction** — missing (morale field exists,
  never mutated; driver logic is §2 #1).
- **Form decay + match-sharpness** — missing (fields exist, never update
  post-match).
- **Player availability state** (suspension, return-to-play windows, congestion
  fatigue) — missing.

### Competitions + progression

- **Multi-tier pyramid with promotion/relegation** — planned (T4.5-B pyramid;
  T4.5-F promotion/relegation). EA-critical.
- **Cup draw + bracket structure** — partial (event scaffolding exists; the draw +
  bracket is T4.5-F). EA-plausible.
- **Fixture congestion from multi-competition play** — missing (depends on
  match-load #3).
- **Dynamic qualification races + tie-breakers** — missing (only final standings
  today).
- **Club/manager reputation affecting transfers + rival perception** — missing
  (manager reputation is §2 #7; club prestige is a separate gap).

### Club operations + off-pitch

- **Board interactions + expectations** — missing (§2 #5).
- **Staff hiring + specialization** (coaches, scouts, medical) — missing.
- **Club finances + budget constraints** — missing (overlaps transfers above).
- **Fan culture as a dynamic system** — partial (fan reader is structural; no
  sentiment mechanics).
- **Facilities + training infrastructure** — missing.
- **Media reputation + press relations** — partial (press conferences /
  PressReader exist; broader media ecosystem missing).

### Career arc + manager progression

- **Manager job-market** (contract end-game, club-switching) — missing (rosters
  tier-lock at career start).
- **Manager reputation as a career-long modifier** — missing (§2 #7).
- **Career-spanning manager legacy** (trophy count, youth-developer brand, etc.) —
  missing (player achievements are ledger-tracked; manager achievements surface
  nowhere).
- **Manager attributes that drift with experience** — missing (archetype is static
  hand-authored content).
- **Persistent manager-vs-manager rivalry state** — partial (callbacks possible via
  ledger; no persistent rivalry relationship).

---

## 4. Per-dimension catalogue (de-duplicated)

One table per research dimension. Where a feature appears in multiple dimensions,
it is listed once in its best-fit home and cross-referenced elsewhere. Features
that map to an existing roadmap row are listed in §5, not repeated here.

### 4.1 Tactics + match preparation

| Feature | Fun rationale | Pillar | fwStatus | Effort | Leverage | Window | Notes |
|---|---|---|---|---|---|---|---|
| Tactical familiarity + cohesion ramp | New shape/lineup takes 2-3 matches to gel, then dominates | 3 | missing | S | high | **EA** | §2 #2. Track `(formation, lineup_hash) → cohesion`; multiplies BT decision-confidence; `ShapeClick` breakthrough at threshold. Cross-ref squad-cohesion (4.5) — same mechanic, unify. |
| Team instructions (press height, tempo, shape, counter speed) | Architect the team identity players execute within | 1,5 | partial | M | high | post-EA | §2 #10. Pre-match sliders seed `ArchetypeParams`; tactic-FSM + BT already consume. |
| Player role variants within formation | "Inverted-pressing-RW" tells the system's story; scouting calibrates fit | 1,2,5 | partial | M | high | post-EA | Add `role_variant` to `FormationSlot` + selector UI + BT-site filtering; ~16-24 role classes. Signatures activate per variant. |
| Opposition tactical analysis (shape / press / set-piece danger) | Facing a high press is a different puzzle from a low block | 4,1 | missing | L | high | post-EA | §2 #6. Post-match computable from ledger + stats; add a `tactical_analyst` scout sub-type. Cross-ref tactical-familiarity scouting + predictive-lineup (below) — fold into one opposition-analysis surface. |
| In-match individual instructions (mark, tighten, pass-lane) | "Stop drifting inside" mid-match is peak micro-management | 5,1 | missing | M | high | post-EA | New `MatchCommand::InstructPlayer { player_id, kind }` → per-player BT context override. |
| Set-piece design + execution (corners/FKs with routines) | A far-post corner routine that works across 3 seasons is a manager signature | 5,1,2 | planned | XL | high | post-EA | DESIGN_DOC §8 lists set-pieces as scripted stubs in T1-2; full design is the deferred T4.5-K (no row yet). Routine RON + per-player assignment + replay. Cross-ref dead-ball coordinator (4.5). |
| Training routines — tactical chemistry (set-piece drills, press shape) | Drilling corners all season unlocks the 89th-minute decider | 3,2 | missing | M | medium | post-EA | No training system exists. Routine picker + daily-tick applier + outcome RNG + ledger event. Foundation for all training features below. |
| Position-specific training focus | Drill fullbacks on cover-shadow, then the tactic works | 3,2 | missing | M | medium | post-EA | Extends the training-routine system per role family; bumps sharpness + signature readiness. |
| Opposition-weakness presets ("press their 10", "overload left") | Tactical agency vs a world-class playmaker; preset saves cognitive load | 1 | missing | M | medium | post-EA | 8-12 preset templates mapping to instruction deltas; archetype gates which presets you see. |
| Dynamic pressing-intensity scaling (press-vs-rest trade-off) | Press 90 mins every match and you collapse in month 2 | 3,2 | partial | M | medium | post-EA | Make pressing intensity accumulate across a week → modulates fitness decay + injury/breakthrough risk. Depends on match-load (4.5). |
| Live tactical heatmaps (clustering / pressure overlays) | "We're too narrow on the left" in real time teaches football literacy | 1,5 | partial | M | medium | post-EA | Influence maps (32×24 grid, 8 Hz) exist in the tactic-FSM; serialize to `MatchFrameDto` + PixiJS gradient overlay + toggle. Overlaps the deferred enhanced viewer (T4-9, §5). |
| Predictive opponent lineup (likely formation + XI) | Pick your shape in response, not reactively at kickoff | 4,1 | missing | S | medium | post-EA | Heuristic over opponent recent lineups + archetype + form. Fold into the opposition-analysis surface (#6). |

### 4.2 Scouting + recruitment

| Feature | Fun rationale | Pillar | fwStatus | Effort | Leverage | Window | Notes |
|---|---|---|---|---|---|---|---|
| Shortlist + watch-list mechanics | Flag a prospect; scouts feed reports over seasons; recall the 3-year journey | 4,2 | missing | S | high | post-EA | §2 #9. `CareerState.shortlist` + `get_watch_list` IPC + `AddedToWatchlist` ledger event. |
| Scout network + hiring workflow | Build the network that finds YOUR hidden gems; scouts age, retire, get poached | 4,1 | missing | L | high | post-EA | Procedural scout-personality gen exists in the bake. Hire/fire/negotiate modal + salary cost + age tracking. Cross-ref staff hiring (4.6) + scout hiring (4.6). |
| Lower-league discovery (tiers 4–6) | An untested tier-5 gem becomes your talisman; the ledger remembers | 4,2 | missing | M | high | post-EA | Unlocked by the T4.5 pyramid. Filter scout reports by tier + discovery feed; `spotted-at-tier-5` callback tag. |
| Attribute-masking depth (genetic uncertainty bands) | Raw pace is hidden; scouts estimate [18-20] vs [14-16]; uncertainty is the tension | 4 | partial | M | medium | post-EA | Deepen: true genes stay hidden, observation count narrows band width (not absolute precision), breakthroughs shift truth within prior range. Reworks `observe_player`. |
| Scout prose reports (vs number-anchored bands) | "Quick but technically raw — a star on the wing, or a project" | 4,1 | missing | S | medium | post-EA | Tracery prose bank keyed by (archetype, confidence, salient bands). Quick win on the existing template system. Cross-ref scout-personality-bias (below). |
| Eye-test match observation | Watch a lower-league match yourself; form your own opinion vs your scouts | 4 | missing | M | medium | post-EA | Optional post-match "record impression" seeded per player-perf; advances confidence more than a scout report (you trust your eye). |
| Regional scout specialization | Your West-African scout reads the WAFCON pipeline; a mismatch yields worse reports | 4,1 | missing | S | medium | post-EA | Archetype already carries a regional-bias field. Procedural world region-codes players → band-width bonus/penalty. Quick wiring once T4.5 lands. |
| Scout-report evolution (same player, same scout, over time) | "[Pace 17-19]" → 2 seasons later "[15-17]" after an injury the scout noticed | 4,2 | missing | S | medium | post-EA | Report already timestamped; show history timeline + `ScoutObservationChanged` event when a band shifts >1 step. |
| Scout personality bias visible (optimism/pessimism vector) | Learn that Scout A overrates, Scout B underrates, and weight accordingly | 4,1 | partial | S | medium | post-EA | Bias vector partly wired (`CategoryBiases`). Surface it on the scout profile; pairing optimist + pessimist balances the read. |
| Youth-specific scout archetypes (Potential-Spotter vs Finisher) | One says "world-class in 4 years," the other "mediocre now"; triangulating teaches wisdom | 4 | planned | S | high | post-EA | Depends on T4-2.5m/n/p (multi-scout, §5). Pure data-layer once multi-scout ships. Cross-ref youth-developer scouting (4.3). |
| Youth academy scouting specialization | Spot 15-year-olds 5 years before the market; early breakthroughs feel like vision | 4,1 | missing | M | medium | post-EA | Needs the T4.5 youth-academy layer. `youth_specialist` archetype flag; lower certainty, earlier window; `FoundYouthTalent` event. |
| Scout training (accuracy improves over seasons) | Experienced scouts are worth more; rookies are cheaper but unreliable | 4,2 | missing | M | medium | post-EA | `years_experience` field; +~1%/accurate-report toward archetype cap; accuracy curve on the profile. |
| Loan-watch (scouts track loanees) | A loanee improves fast; scouts flag the upside; recall decision later | 4,2 | missing | S | low | post-EA | Depends on the loan system (T5+). `on_loan` flag on observations; revised estimates on return. |
| Rival scout reports leak to press | "Rival's top scout rates our new signing" — prestige + morale ripple | 4,2 | missing | S | low | post-EA | `ScoutReportLeaked` event → press headline; morale/prestige bump if the scout is respected. |
| Scout observation-load fairness (budget per cycle) | Over-scouting one player leaves others in fog; build the calendar strategically | 4 | partial | M | medium | post-EA | Weekly observation budget (e.g. 5 slots/cycle) + per-player narrowing cap. You can't know everyone — deep observation is a deliberate choice. |

### 4.3 Youth + player development

| Feature | Fun rationale | Pillar | fwStatus | Effort | Leverage | Window | Notes |
|---|---|---|---|---|---|---|---|
| Mentoring system (senior→youth pairs) | The prospect blooming under a veteran's wing; multi-generational arcs | 2,3 | missing | M | high | post-EA | §2 #8. `MentorTeammate` already in the family-relevance table. Cross-ref dressing-room mentorship (4.5) — unify into one mentor mechanic. |
| Academy intake pipeline + newgen regens | Each season auto-generates 6-12 save-unique prospects anchored to your nation's culture | 1,2 | missing | M | high | post-EA | Uses T4.5-E1 procedural gen + name Markov + Identity Packet. The feeder that keeps academies fresh. |
| Youth intake day (annual draft-style event) | Scout 12 prospects for 30 seconds each; pick 2-3; rejected ones join rivals | 1,4 | missing | M | high | post-EA | Annual modal + squad-capacity gating; rejected prospects seeded to rival academies. The pipeline that avoids tedious weekly scouting. |
| Hidden vs visible potential (age-curve trajectory) | A late-bloomer reads "moderate" until year 6, then your genius call pays off | 4,3 | partial | S | high | post-EA | Identity Packet (T4.5-E0) encodes peak-age + aging curve; scouting projection exists. Narrow the band as age advances + a "projected peak" tooltip. Mostly visibility work. |
| Development loans (recall + obligations) | Loan a 17-year-old to tier-4, he breaks through, you recall or let it run | 2,3 | missing | L | high | post-EA | `LoanStarted/Ended/GoalScored` events; early-recall on breakthrough is emergent drama; toxic environment can trigger regressive collapse. Cross-ref transfer loans (4.4) — same system. |
| Youth contract promises + broken expectations | Promise "30 matches," bench him, and a burned youth becomes a rival's grudge signing | 2,3 | partial | M | high | post-EA | `PromisedYouthMinutes` + `BrokenPromise` already in the event enum. UI promises + season-end match-count check. Cross-ref morale drivers (§2 #1). |
| International youth tournaments | A goal in the U20 final accelerates Composure/Leadership readiness; club prestige rises | 2,3 | missing | L | high | post-EA | Needs T4.5 national-team scaffolding + multi-nation (post-EA per DECISIONS 2026-05-29). Rolled-outcome sim at away-fixture cadence. |
| Coach attribute-focus system (training specialization) | "Finishing drills" this quarter; high-affinity players accrue readiness +25% | 3,2 | missing | M | medium | post-EA | A `+multiplier` on readiness accumulation, NOT XP. Needs a coach-role system + the academy first. Cross-ref assistant-coach specialization (4.6). |
| Academy reputation + youth-product revenue | Selling young players you developed builds a tier; unlocks better newgens + prices | 1,2 | missing | M | medium | post-EA | Needs the transfer market. Reputation feeds newgen quality + selling multiplier; surfaces in media + rival quotes. Cross-ref academy prestige (4.6). |
| Youth role pathways + position fluidity | A 16-year-old winger trends toward fullback; a position lock at 20 is permanent | 3,1 | partial | M | medium | post-EA | `role_affinity_weights` shipped (T1-1). Add a mutable `position_commitment` at age 20 + coach-focus multiplier; show role-probability bars not a hard lock. |
| Injury recovery curves + resilience gene | High resilience returns in 6 weeks; an ACL at 19 is a regressive-collapse with a rare reversal | 3,2 | partial | S | medium | post-EA | `InjuryLongTerm` weighted in the family-relevance table. `injury_resilience` gene (T4.5-E0) modifies recovery; rare resilience-breakthrough reverses collapse. Cross-ref injury progression (§2 #4) + medical staff (4.6). |
| Wonderkid milestones + media tracking | First senior goal at 16 → "teenage phenom emerges"; goals accumulate into elite-tracking | 2,1 | missing | S | medium | **EA** | `DebutSenior` already fires. Add `TeenageGoalMilestone`/`YoungProdigyMilestone`/`EliteTrajectory` salience-gated events. Low lift on the committed template-bank. EA-plausible. |
| Player personality stability + youth volatility | A hot-headed youth becomes composed after a leadership breakthrough; adults lock at 23 | 3,2 | missing | M | medium | post-EA | `PersonalityVector` in canonical state (ADR-0002). Apply ±0.1 to 1-3 fields per breakthrough, youth only, family-gated. Deterministic + ledger-compatible. |
| Multi-season academy cohorts + graduation | "The '26 Class" — track how many promoted, sold, became captains; club lore | 2,1 | missing | L | medium | post-EA | Needs multi-season tracking (T3 ships it). Birth-year grouping + graduation + cohort-milestone events. |

### 4.4 Transfers + club economy

| Feature | Fun rationale | Pillar | fwStatus | Effort | Leverage | Window | Notes |
|---|---|---|---|---|---|---|---|
| Transfer market search + filtering | Scout a rival's prospect, negotiate the fee — recruitment as a story, not a chore | 4 + new | missing | M | high | post-EA | Table-stakes spine (§3). Base table + filters; scout disagreement feeds the "true value." Must assume unknown club count (Decision 5 / pyramid scale). |
| Contract negotiation with asking prices | A player you can't afford becomes a free agent next summer — urgency vs budget | 2 + new | missing | M | high | post-EA | `(expiry, wage, asking_fee)` per player; board → offer → club → player chain; `TransferOffered/Rejected/Completed` events; rejections seed rivalry callbacks. |
| Free transfers at contract expiry | A player you trained walks free to a rival on a signing-on bonus | 2 + new | missing | S | high | post-EA | Auto-generate AI offers for expiring contracts; `PlayerFreed`/`PlayerSignedByRival` events surface in pre-season + rivalry prose years later. |
| Squad budget + wage-bill tracking | £3M to spend but wages already 92% of cap — sign means sell two or break board rules | new | missing | M | high | post-EA | `transfer_budget_remaining` + `max_wage_bill`. Pre-transfer math; breach → `BoardWarning` toward sacking. Grounds transfers in scarcity. Cross-ref board financial pressure (below) + wage parity (4.5). |
| Transfer-window deadlines + closing | 11:59pm deadline day, bid pending, the door about to shut | new | missing | S | high | post-EA | Frozen window state; offers auto-expire at close; `TransferWindowOpens/Closes`/`OfferExpired`. Calendar urgency off the existing season loop. |
| Resale value depreciation + aging | The 29-year-old worth £8M three years ago is worth £2M and nobody wants him | new | missing | S | high | post-EA | Annual recalc: base × age-curve × form × recent-perf. Couples progression to economic reality. |
| Academy graduates + young sell-offs | A 16-year-old you trained is sold at 18, then becomes a rival's key man in a derby | 2,3 + new | missing | L | high | post-EA | Needs the academy (post-EA). `youth_developed_at` tag; `AcademyProductSold`/`AcademyProductScored` callbacks ("their academy prospect just went through us"). Cross-ref academy reputation (4.3). |
| Release clauses (percentage triggers) | A secret buyout clause activates when a richer club triggers it — a dramatic forced exit | new | missing | M | medium | post-EA | Optional `release_clause_multiplier`; auto-accepts unless player rejects for morale/ambition. Depth without negotiation-UI bloat. |
| Loan moves with obligations + buy options | A loanee develops; recall, or let the obligation-to-buy trigger | 2 + new | missing | L | medium | post-EA | `(duration, obligation, buy_option_fee)`. Same system as development loans (4.3) — unify. |
| Sell-on clauses + percentage rights | You sold him with 20% sell-on; years later a mega-move gives you a windfall | 2 + new | missing | M | medium | post-EA | `(player_id → Vec<sell_on_stake>)` registry; payment ledger events. Long-tail financial stories. |
| Board expectations + financial pressure | "Spend £5M or face relegation" — confidence decays toward a sacking | new | missing | M | medium | post-EA | Season targets (spend, wage-growth, sale-minimum); shortfall → `BoardExpectationMissed`. Couples to board confidence (§2 #5). |
| Agent demands + intermediary fees | The agent wants 8%; negotiate to 5% or walk | new | missing | M | medium | post-EA | Deterministic `agent_demand_percentage` per player; `AgentFeeNegotiated`. Friction without heavy UI. |
| Promotion/relegation transfer cascades | Get promoted, rivals double the price; relegation forces fire-sales | new | missing | L | medium | post-EA | Tier-change → asking-price recalc (+40% promo, -30% releg); crisis fire-sale at 50%. Couples market to world mobility. Depends on T4.5-F. |
| Rival counter-bids + transfer battles | You bid, the rival matches, you must outbid — the escalating war for a signature | new | missing | L | medium | post-EA | AI clubs track targets per seed; `TransferBattleWon/Lost` → press ("you beat City to his signature"). |
| Injury-driven emergency transfers | Striker breaks his leg; 2 weeks to sign a replacement or play out of position | new | missing | M | medium | post-EA | Depends on injuries (§2 #4). Emergency mode: AI prices +50%, reduced leverage; `EmergencySigning*` events. |
| Multi-year contract terms + installments | Sell for £2M over 2 years; cash flow matters when buying a replacement | new | missing | M | low | post-EA | Installment schedule on transfer fees; budget accounts for deferred liabilities. Lower immediate fun; valuable late-career. |
| Summer splurge vs winter rebuild | £10M summer overhaul, then a constrained January defensive rebuild | new | missing | S | medium | post-EA | Winter window <60% of summer budget; ties to the existing schedule (T4.5-I). `SummerWindowClosed`/`WinterWindowOpened`. |

### 4.5 Squad dynamics + condition

| Feature | Fun rationale | Pillar | fwStatus | Effort | Leverage | Window | Notes |
|---|---|---|---|---|---|---|---|
| Morale driver system + satisfaction score | Transparent ledger-sourced morale that gates transfer requests + breakthroughs | 2,3 | missing | M | high | post-EA | §2 #1. The connective tissue for half this backlog. Cross-ref club-ops morale (4.6) + youth promises (4.3) — one morale model. |
| Cumulative match-load + fatigue decay | Field a star too often and he limps through the cup final | new | missing | M | high | post-EA | §2 #3. The data-layer foundation for rotation, congestion, injury return-to-play. |
| Injury progression + return-to-play gates | Return too early before full recovery and risk re-injury | 3 | partial | L | high | post-EA | §2 #4. Cross-ref injury recovery curves (4.3) + medical staff (4.6) — one injury state machine. |
| Squad cohesion + tactical-synergy multiplier | Five new signings drop set-piece success from 75% to 52% until they gel | 5 | missing | S | high | post-EA | Same mechanic as tactical familiarity (4.1, §2 #2) — UNIFY. Per-pair shared-starts → signature success bonus. |
| Transfer-request system + exit pressure | An unhappy player demands a move; accept, reject, or negotiate | 2 | missing | M | high | post-EA | Conditional on morale + unresolved driver. `TransferRequested` event; response options reset/append callbacks. Couples to morale (#1) + transfer market (4.4). |
| Captain role + dressing-room influence | A composed veteran captain broadcasts a mentality bonus; his unhappiness ripples | 5 | missing | M | high | post-EA | Captaincy at the tactical board; in-match +0.08 bias on relevant decisions; 1.5× morale-driver weight. Leadership breakthroughs are high-salience. |
| Mentorship + senior influence on youth | Pair a youth with a veteran; decision-velocity + vision accelerate; lose the mentor, growth flattens | 3 | partial | M | high | post-EA | Same as the youth mentoring system (4.3, §2 #8) — UNIFY into one `mentor_id` mechanic. |
| Fixture congestion + midweek fatigue | Three matches in 8 days drops pass-success + decision quality 8-12% | new | missing | S | high | post-EA | Depends on match-load (#3). Start-spacing penalty + cumulative re-injury tick. Mid-season congestion becomes a tactical bottleneck. |
| Form decay from results + inactivity | Six in three, then a missed penalty + a benching, and form tanks | new | missing | S | medium | post-EA | Weekly update from goals/assists/results/benchings; bounds [0,1]; feeds morale. Core to FM-player perception. |
| Player personality-conflict + chemistry | Two aggressive low-teamwork players battle for ball dominance; a mentor fits impressionable youth | 5 | missing | M | medium | post-EA | Pairwise compatibility table → pass/positioning modifier [0.9,1.1]; `PersonalityClash` events. Edge-case-heavy (most pairs compatible). |
| Player position versatility + role learning | Play a striker on the wing 8 times and muscle memory kicks in | 3 | missing | M | medium | post-EA | Per-player `role_affinity: BTreeMap<Role, Q32>`; off-role penalty then learning; rare off-role-success breakthrough. Orthogonal to the main loops. |
| Contract wage parity + morale spillover | A top-earner playing badly breeds dressing-room tension | new | missing | M | medium | post-EA | Wage-vs-performance-percentile morale driver + squad-wide spillover on >3× disparity. Economic texture without negotiation UI. |
| Retirement pressure + aging decline | A 33-year-old captain loses starts; retire him gracefully or force out a 7-season loyalist | 3 | missing | M | medium | post-EA | Aging-curve PA decay 31+, retirement hazard 34+, graceful-farewell option; `Retirement` event. Late-career long-tail. Cross-ref managerial aging (4.7). |
| Set-piece routine mastery + dead-ball coordinator | Designate a corner/FK/penalty taker; (taker, striker) reps build routine success | 5 | missing | S | medium | post-EA | Per-pair mastery bonus; taker swap resets. Ties to signature modulation. Cross-ref set-piece design (4.1) — the design system is the XL parent; this is the lightweight taker-assignment slice. |

### 4.6 Competitions + progression (non-roadmap rows)

> Pyramid (T4.5-B), promotion/relegation + one cup (T4.5-F), and the unbeaten-run
> event are in §5. The below are additional candidates.

| Feature | Fun rationale | Pillar | fwStatus | Effort | Leverage | Window | Notes |
|---|---|---|---|---|---|---|---|
| Qualification races + tiebreaker drama | Final day, neck-and-neck for the second promotion slot; a goal elsewhere swings it | 2,4 | missing | M | high | post-EA | Live tie-breaking (GD, head-to-head) + a "race-to-promotion" widget; `QualificationRaceDecided` event. Cross-ref head-to-head tiebreaker (below). Note: season-arc drama tuning is Tier F (`FUN-4`, §5) — this is the mechanic, not the tuning. |
| Club reputation modulation across tiers | Promote and you're suddenly outclassed; transfers dry up; prestige re-earns slowly | 4,2 | missing | L | medium | post-EA | Club-subject MemoryEvents → a `ReputationScore` modulating transfer-interest + scout confidence. The procedural moat: no inherited real-world brand prestige — reputation is earned per save. Cross-ref club operations §3. |
| Fixture congestion from cup midweek | A cup run means a star plays every 3 days; rotate or risk injury | 3,5 | missing | M | high | post-EA | Same as squad fixture-congestion (4.5) once a cup exists — unify. |
| Head-to-head tiebreaker drama | Level on points, you own the H2H 2-1, the final match decides the title | 2,4 | missing | S | medium | post-EA | Compute H2H at standings-finalize; `TitleDecidedViaHeadToHead` event + commentary. Fold into the qualification-race surface. |
| Seasonal awards (Player of the Year, etc.) | Your striker wins POTY year 2; it affects transfer appeal + next-season confidence | 2,3 | missing | S | medium | post-EA | Rank by a season-stat formula; `PlayerAwardWon`; season-end podium. Light addition to the post-season routine. |
| Goalkeeper save records + defensive milestones | 14 consecutive clean sheets; scouts rave; a defensive signature unlocks | 2,3,5 | missing | S | medium | post-EA | `ConsecutiveCleanSheets` tracker → `DefensiveMilestone`; keeper-specific signature. Light post-match harvest. |
| Reserve/youth league parallel track | Prospects play a weekly reserve match; breaking into the senior XI is a tracked milestone | 3,2 | missing | L | medium | post-EA | Auto-sim only; `ReserveGoal`/`ReserveAssist` development gating. Needs the academy. |
| Historical playoff bracket (final promotion slot) | Finish third, a 4-team playoff decides promotion; sudden-death atmosphere | 2,5 | missing | S | medium | post-EA | Light fixture-structure addition on T4.5-F; playoff `MatchEvent` class + commentary bank. |
| Continental cup (multi-nation tournament) | A domestic cup win sends you into a continental run vs other nations' clubs | 1,2 | missing | XL | high | post-EA | OUT OF EA — multi-nation is post-EA (DECISIONS 2026-05-29). Needs 2+ seeded nations + inter-nation seeding + a European calendar. High fun if it ships; deferred until world-gen matures. |
| Relegation cascade (double-demotion) | A catastrophic relegation chain becomes a scar the manager never forgets | 2,1 | missing | S | low | post-EA | Optional per-pyramid-config rule on consecutive bottom-2 finishes; nation seed determines cascade rules. Flavor. |
| Split-season format (spring/fall titles) | Some nations split the season; an autumn peak doesn't carry to spring | 1,3 | missing | M | low | post-EA | `SeasonHalf` enum + revised title logic; region-seeded. Shallow fun, high world-gen authenticity. |
| Conference seeding (split-season regrouping) | Mid-season regrouping into title-race vs playoff conferences shifts the pressure | 1,3 | missing | M | medium | post-EA | Pairs with split-season; conference-aware scheduling. Low priority unless split-season ships. |
| Expansion / new-club induction | A seeded new club enters a lower tier mid-career with different cultural priors | 1,2 | missing | M | low | post-EA | Mid-career club insertion + fixture rebalance. Advanced; out of EA. Minimal leverage unless paired with relegation-replacement. |

### 4.7 Club operations + off-pitch

| Feature | Fun rationale | Pillar | fwStatus | Effort | Leverage | Window | Notes |
|---|---|---|---|---|---|---|---|
| Board confidence dynamics | Board pressure vs a long rebuild; sacking threats; bold-signing demands | 2 | missing | M | high | post-EA | §2 #5. Confidence curve + decay; ledger-tracked callbacks. Cross-ref board financial pressure (4.4) — financial + confidence halves of one board model. |
| Assistant-coach specialization + training focus | A coach archetype modifies training output + tactical flexibility; a bad hire locks you in | 2,4 | missing | L | high | post-EA | Coach archetypes with hidden expertise genes × player learning-style matrix; dismissed coaches surface at rivals. Uses the archetype + name infrastructure. Cross-ref coach attribute-focus (4.3) — unify. |
| Club rivalry depth (mutual respect vs animosity) | Rivalry heat escalates over years; affects morale, press tone, fan sentiment | 1,2,5 | missing | M | high | post-EA | `RivalryFormed` exists one-way; extend to bidirectional heat with decay + incident escalation. The procedural-world edition: rivalries unique per save. |
| Player morale + team cohesion (club-ops view) | Unhappy players underperform; one toxic player poisons chemistry | 2,3 | partial | M | high | post-EA | Same morale model as §2 #1 + cohesion as the group aggregate — UNIFY, don't build twice. |
| Fan culture + ultras/supporter groups | Procedural ultras with chants + demands; upsetting them sours the atmosphere | 1,2,5 | missing | L | high | post-EA | Procedural groups per club; extend the fan-sentiment reader to group-specific demands; signature moments make fan legends. High flavor, low mechanical depth. |
| Youth intake + academy prestige | Better facilities attract better prospects; producing your own star beats buying | 1,2 | planned | L | medium | post-EA | Scaffolded in the save schema; surfacing deferred (DESIGN_DOC §8). Same academy system as 4.3 — UNIFY. Prestige is callback-eligible. |
| Media narrative + press reputation arc | Press tracks your methods; poor PR makes recruitment harder; a trophy redemption arc | 1,2 | missing | M | medium | post-EA | Procedural press outlets with archetype biases; tracked opinion state per outlet + team consensus. `PressReader` exists; extend with a "Media Reaction" topic. Cross-ref press-cycle pressure (4.8). |
| Rival manager ecosystem + tactical arms race | Rivals learn from playing you, shift tactics, get promoted/sacked, hire your ex-coaches | 1,2 | planned | M | medium | post-EA | Manager archetype roster generated at bake (T3-4 rival ecosystem). Extend with evolution + mobility + callbacks. Cross-ref the whole career-arc dimension (4.8). |
| Sponsorship + kit revenue | Secure sponsors for budget; a bad deal locks an ugly identity; a megastar attracts sponsors | 1 | missing | M | medium | post-EA | Procedural sponsor gen (no licensed brands); contract duration + prestige → budget ceiling; disputes hit fan sentiment. Per-save sponsor identity. |
| Stadium + facility upgrades | Upgrade training/youth/medical; better facilities attract stars + cut injuries | 1 | missing | M | medium | post-EA | Multi-year projects; facility state baked per club seed (asymmetric). Summary-level spend toggle, not a minigame. Modulates training + morale + recruitment. |
| Injury + medical staff impact | Better medical staff cut injury rates + speed recovery; a star injury hits morale | 3,2 | missing | M | medium | post-EA | Medical archetype modifies injury probability + recovery. Same injury state machine as §2 #4 — UNIFY. |
| Contract negotiation + wage-structure depth | Negotiate base + bonuses; broken bonus promises fuel transfers | 2 | partial | M | medium | post-EA | Basic renewal scaffolding exists. Wage negotiation UI + hidden expectation + bonus structure + broken-bonus callbacks. Same contract system as 4.4 — UNIFY. |
| Long-term club legacy + monument building | Retire and see your impact endure: a youth product becomes the club's face for a decade | 2,1 | missing | S | medium | post-EA | Late-career summary; legacy invoked in callbacks. Low mechanical depth, high emotional resonance. Cross-ref manager dynasty (4.8). |
| International duty + availability tracking | Stars leave for international duty mid-season; you lose them for critical weeks | 2 | partial | S | low | post-EA | `InternationalCallUp` event exists. Per-player duty flags + break-window availability + injury risk + morale. Low leverage (not FW-unique); valuable as a constraint. |

### 4.8 Career arc + manager progression

> Most of this dimension depends on a manager job-market, which depends on the
> T4.5 world being live. Several are pure ledger read-projections (low effort,
> high leverage) and are EA-plausible.

| Feature | Fun rationale | Pillar | fwStatus | Effort | Leverage | Window | Notes |
|---|---|---|---|---|---|---|---|
| Manager reputation gradient (0–100, ledger-derived) | The weight of your own history feeds press tone, signing leverage, board patience | 2,5 | missing | M | high | post-EA | §2 #7. Read-only projection from event salience; slow decay. The foundation for this whole dimension. |
| Manager-vs-manager rivalries (ledger + press callbacks) | The manager you beat in a cup final 5 seasons ago is now your title rival | 2,5 | partial | S | high | **EA** | Cup-final/title events already carry opponent context. Pure read-side projection — no new canonical state. EA-plausible high-leverage win. |
| Manager attributes drift with career events | Watch yourself grow as a tactician, or calcify into a rigid veteran after relegation | 2,3 | partial | M | high | post-EA | Add a mutable attribute layer to `ManagerArchetype` (tactical_innovation, composure, youth-eye) drifting via signature events; youth-eye boosts young-player readiness. |
| Promotion/relegation alters manager identity | Climb tier-5→tier-1 and board expectations flip from survival to titles | 1,2,4 | missing | S | high | post-EA | Needs job-market + promo/releg. `TierChanged` event; reset reputation/confidence to 50 on tier-up (you're new to this level). Pure event + projection. |
| Sacked-from-top-club comeback arc | Fired mid-season, forced to a tier-4 role, a 3-season climb back is a redemption arc | 2,5 | missing | S | high | post-EA | Needs job-market. `FiredFromClub` event; press flags "redemption" on lower-tier re-emergence. Pure event-narrative. |
| Youth-developer vs pragmatist career arcs | Sell youth at peak (cold pragmatist) or blood academy kids (loyalty arc) — a readable brand | 2,5 | partial | M | high | post-EA | `(youth_debuted - youth_sold)/seasons` = pragmatism score from event history. Press callbacks reference it. No canonical change. |
| Manager dynasty (25+ season arc) | Build one club to legend; when you retire, the club descends; future saves reference you | 1,2,5 | missing | S | high | post-EA | No new mechanics — needs robust 25-season play + the legends file (below). Press surfaces "longest tenure"/"most-trophies." Cross-ref club legacy (4.7). |
| Inter-save manager legacies + Hall of Legends | Your past-self haunts future saves as a rival archetype or legendary predecessor | 1,2,5 | missing | L | high | post-EA | Serialize a career summary to `legends.ron` indexed by (seed, era); new-save worldgen references them. Needs T4.5 world-gen wiring. |
| Manager job-market + contract negotiation | Contract expires: renew, drop a tier to a bigger club, or get sacked into a job drought | 1,2,5 | missing | XL | high | post-EA | Needs T4.5-level world gen: per-season postings from ~96 clubs filtered by reputation + tier-affinity; negotiation UI; job loss is a career event. The structural keystone for half this dimension. |
| Board relations (3-axis confidence gating tenure) | Attack/Defend/Ambition axes shift with results; low trust = interference, high = blank cheque | 2,4,5 | missing | M | medium | post-EA | Per-club `(attack, defend, ambition)` 0-100. Thresholds gate forced board meetings vs free spend. Read-only projection. Same board model as §2 #5 + 4.4 — UNIFY (3-axis is the richer variant). |
| Press-cycle pressure (media swings affect morale) | A bad month's hostile headlines cost morale; reputation gates how hard the press turns | 2,5 | partial | S | medium | post-EA | `PressReader` wired (T4-2.5k). Aggregate morale delta from press cycles, reputation-gated magnitude. No new canonical state. Cross-ref media narrative (4.7). |
| Manager signature win-type ("brand") | Known for possession, or clinical counters, or last-minute heroics; rivals tailor their approach | 5 | missing | M | medium | post-EA | Win-context distribution from event history → a soft `manager_brand` tag for press flavor. No canonical mutation. |
| Personal arch-rivals (never-beaten white-whale) | One rival systematically beats you; finally beating him is a massive release | 2,5 | partial | M | medium | post-EA | `arch_rival_id` + `h2h_records` BTreeMap; press detects "never beaten." Ledger already tracks matches. Cross-ref manager rivalries (above) — same H2H spine. |
| Managerial aging curve + career ceiling | At 68+ the press asks "when will he step aside?"; you know this career is finite | 2,3 | missing | S | medium | post-EA | `manager_age` increments; 65+ morale drift, 70+ retirement queries, 75+ auto-retire. UI-projected from career date. Cross-ref player retirement (4.5). |
| Contract standoff (wage/duration discord) | Club wants you at 80% wages; take the cut, force a move, or ride it out | 2,5 | missing | M | medium | post-EA | Needs contract + job-market. Multi-choice UI at contract-end; `ContractNegotiation`/`SackedWithPenalty` events; reputation gates leverage. |
| Unlockable manager archetypes (achievement-gated) | Win a treble, unlock a "Trophy Hunter" archetype for the next save with new tactics | 5 | missing | M | medium | post-EA | Needs the inter-save legends file. New-save character-select lists unlocked variants. Achievement flags in the legends file. |

---

## 5. Already planned (cross-reference — do NOT double-count)

These research candidates map to existing roadmap rows. They are tracked there;
this backlog only notes the mapping so they aren't re-scheduled as "new."

| Research candidate(s) | Roadmap row | Status / window |
|---|---|---|
| 6-tier pyramid, ~96 clubs, lower-league discovery substrate | **T4.5-B / T4.5-B0** | Planned, EA-critical |
| Promotion/relegation + one cup competition + player-club tier-mobility; cup draw/bracket | **T4.5-F** | Planned, EA-critical |
| Procedural ~2000-player compiler + newgen gene substrate; Identity Packet (peak-age, aging curve, resilience gene) | **T4.5-E0 / T4.5-E1** | Planned, EA-critical (academy LAYER on top is post-EA, §4.3/§4.7) |
| LLM-baked content corpus (name banks, Tracery grammars for scout-prose / commentary / headlines / manager quotes / fan reactions) | **T4.5-D** | Planned, EA-critical (scout-prose + media banks draw from here) |
| SaveV5 world-gen descriptor | **T4.5-H** | Planned, EA-critical |
| Multi-scout disagreement (3+ archetypes), scout track-record scoring, scouting-board UI, youth-specific scout archetypes | **T4-2.5m / n / p** | Deferred — conditional-EA behind the Month-4 feel-prototype gate. Single-scout (T4-2.5f) is the shipped EA floor |
| Set-piece DESIGN system (corners/FKs with routines) | **T4.5-K** (deferred, no row cut yet) | Post-EA; DESIGN_DOC §8 lists scripted stubs only at T1-2 |
| Live tactical heatmaps / influence overlays / intent arrows | **T4-9** (enhanced 2D viewer) | Deferred; promotion trigger = EA ships clean + owner wants it |
| Unbeaten-run-ended milestone | **T3-3** (`UnbeatenRunEnded` event exists) | Partial — tracking + emission at threshold is the remaining slice |
| Rival manager ecosystem (BT-driven selection) | **T3-4** | Shipped baseline; evolution + mobility extensions are §4.7/§4.8 |

### Tier F boundary (the OTHER fun-axis — explicitly not catalogued here)

The match-feel + season-arc axis is owned by Tier F and its design docs, NOT this
backlog. For clarity on the seam:

- **Match drama tuning, commentary quality, callback-landing, season-arc
  retention** → `FUN-1..4` + `drama-model.md` + `fun-evaluation-harness.md`.
  Where this backlog names a *mechanic* (qualification-race tie-breaking, fixture
  congestion, milestone events), Tier F owns the *tuning* of how dramatic those
  moments feel. The mechanic is here; the feel is there.
- **`FUN-5` (decision satisfaction)** explicitly depends on this feature-backlog —
  it asks whether a signing / tactic / youth bet feels like a meaningful gamble
  that pays off, which is exactly the breadth this doc catalogues. The §2 top
  candidates (scouting uncertainty resolving, youth bets, transfer gambles,
  morale-gated requests) are the decision surfaces `FUN-5` will probe.

---

*Authored 2026-06-04. Living backlog — revise as roadmap rows absorb candidates
and as the Tier F fun-evaluation harness reports which decision surfaces actually
land. Cross-references: `docs/MASTER_PLAN.md` (Tier F, T4.5-*, T4-2.5*),
`docs/design/drama-model.md`, `docs/design/fun-evaluation-harness.md`,
`docs/DESIGN_DOC.md` §8 (ruled-out list).*

---
title: Football-management genre gap analysis vs Final Whistle
date: 2026-06-06
status: DRAFT — workflow-generated, pending owner review
provenance: 15-domain dynamic workflow (46 agents) — genre web-research -> code-verified FW audit -> per-domain gap list -> opus synthesis. Audit agents read the real repo; treat priorities as a starting map, not verified ground truth. Verify any specific claim against code before acting.
---

# Final Whistle — Prioritized Gap Analysis (15-domain synthesis)

*Synthesized 2026-06-06 from per-domain genre-vs-FW analyses. Status reflects implementation reality (code-verified), not specs. Football-native vocabulary throughout. Effort: S/M/L/XL. Priority: P0 (blocks "is this a game"), P1 (core depth), P2 (rounding-out), P3 (long-tail/flavor).*

---

## 1. Executive summary

**Where Final Whistle is genuinely strong.** FW has built the hard part first — a deterministic, honest simulation core that most management games never attempt. The pieces that are real and wired end-to-end:

- **A watchable, paced live match** (`LiveMatch.tsx`) with five speed modes, auto-pause on key moments, a half-time interstitial, a 2D PixiJS tactical board following live play, and a text-first event feed. This is the spine of the text-first presentation pillar and it works today.
- **The breakthrough-driven development engine** (pillar 3) — `crates/fw-memory/src/breakthrough.rs` `evaluate()` runs for every rostered player on season advance, fills 10 per-family readiness meters from ledger events, redraws PA/CA, and fires `SignatureAwakening` / `RegressiveCollapse` moments into the append-only ledger. This is the bespoke alternative to FM's training grid and it is live.
- **A deep, compile-time-pinned attribute & personality substrate** — 55-field `PlayerAttributes`, a 14-axis hidden `PersonalityVector` wired into BT utility via 21 bias coefficients, `AbilityCeiling` with CA≤PA invariant enforcement, and a live signature dispatcher (pillar 5).
- **Single-scout uncertainty** (pillar 4 floor) — `observe_player` produces banded estimates surfaced as football-native confidence text ("a hunch" → "a settled read"), cached per player and persisted.
- **A playable single-tier career loop** — procedural 20-club league generated from culture-seeded Markov chains, 380-fixture schedule, live standings, season advance, a 440-instance roster (20×22), club selection, load/save.
- **A genuinely sound ledger + reader foundation** — 31 `EventClass` variants, five readers (Press/Fan/Coach/Scout/Salience), with `PressReader` wired to the Career inbox.

In short: the engine is honest and the spine is playable. FW is closer to "a believable simulation you can watch" than to "a complete manager game you can live inside."

**The biggest missing pillars/systems that stop it being a complete game.** Three structural holes dominate, and they cluster:

1. **No morale, and no condition/fatigue/sharpness.** `PlayerCondition` (morale, form, match_fitness, sharpness) is *defined* in `fw-core` (line 944) but **completely inert** — zero consumers in `fw-match-sim`, not persisted, not populated. A football management game where morale has no effect on the pitch and players never tire is, mechanically, not yet a manager game. This is the connective tissue every other interaction system (team talks, promises, press responses, board confidence) needs.

2. **No pre-match team selection, no formation choice, no working in-match levers.** The Tactics route is literally "Not yet wired." Lineups are auto-filled by `advance_week`; the sim is hardcoded 4-3-3 (`FORMATION_4_3_3_POSITIONS`); 8 of 9 `MatchCommand` variants return `LiveMatchCommandUnimplemented` (only `ChangePressLevel` does anything). The manager currently cannot pick who plays, in what shape, or change anything live beyond pressing intensity. This is the single largest cluster blocking "feels like a manager game."

3. **No transfer market, no contracts, no finances, no board.** `Transfers.tsx` is a window-state pill over a stub. There is no wage/contract/expiry field anywhere, `contract_status` is hardcoded `None`, no budget or finance fields exist in SaveV4, and there is no board-confidence state in `CareerState`. The schema is *ahead* of the code — 8+ `EventClass` discriminants (promises, transfers, contract renewals, board events) and `EmitterKind::BoardSystem` exist as locked stubs that **nothing emits**. Careers cannot yet span multiple seasons of squad-building, which is what makes pillars 1 and 2 (unique worlds, careers that remember) pay off.

A recurring pattern across all 15 domains: **FW has consistently built the data substrate (ledger event classes, condition fields, scout archetype enum slots, age_years field) before the mechanic that drives or reads it.** `age_years` exists but is always set to a hardcoded `24`. The six non-basic scout archetypes are reserved enum variants with no biases. This is disciplined scaffolding, but it means the *visible* game lags the *latent* one. The highest-leverage work is repeatedly "emit the event / read the field / wire the handler," not "design a new system."

**The honest verdict:** FW has the strongest foundation layer of any indie football manager but is missing the three table-stakes loops (manage the match, manage morale, manage the squad over time) that turn a simulation into a game. Closing those three — in the text-first, procedural-fantasy idiom rather than by importing FM's UI — is the path to a complete game.

---

## 2. Per-domain gap tables

> Status legend: **present** (wired end-to-end today), **partial** (substrate or one slice exists; the loop is incomplete), **missing** (no implementation), **out-of-scope** (rejected by design — see §4).

### Area 1 — Match day & in-match management

| Feature | Genre behavior | FW status | Priority | Effort | FW-idiom translation |
|---|---|---|---|---|---|
| Watchable paced match (speed modes, auto-pause, 2D board) | Real-time watch with speed/highlight controls | present | P0 | S | Text-first feed + commentary; PixiJS board; auto-pause = "key moments" mode. Done in `LiveMatch.tsx`. |
| Pre-match team selection / starting XI | Pick 11, set lineup | missing | P0 | L | Roster panel drags players into shape slots; committed lineup replaces attribute auto-fill. Largest single "feels like a manager" gap. |
| Formation selection & switching | Pick formation, switch live | missing | P0 | XL | Formation = content-pack ID; pre-match pick from authored shapes; live `ChangeFormation` shifts at next restart. Touches positional model + content + UI. |
| Working live substitutions | Swap players mid-match | missing | P0 | L | Slot-picker fires `MatchCommand::Substitute`; sim swaps slot, board updates, commentary frames it. IPC type ready; sim swap absent. |
| Live pressing control (touchline) | Press higher / drop deeper | present | P1 | S | "Press high / Sit deep" → `set_press_level`. The one working command. |
| Live tempo bias (touchline) | Faster / slower tempo | partial | P1 | M | `ChangeTempoBias` type + button exist; no sim hook. Add urgency-weight change. |
| Live xG surfaced as prose | Running xG tally in dugout | partial | P1 | M | xG computed (`xg_utility`) but not in `StepResult`. Surface as "chance quality" prose, not a number. |
| Possession % shown live | Home/away split | partial | P2 | S | Computed in snapshot but `LiveMatch` uses `StepResult`; add `possessionPct` to StepResult. |
| Player condition / fatigue monitoring | Color-coded condition bars | missing | P1 | L | No fatigue numbers; "Ashby is tiring" prose. Prereq for sub-timing. `PlayerState.scalars` is an empty stub. |
| Opposition scouting report (pre-match) | Opponent dossier | missing | P1 | M | Assistant prose briefing from fixture log + procedural scout notes; no real names. |
| Post-match review screen | xG story, ratings, physio, mood | missing | P1 | L | Prose summary: "How the goals fell", "Standout contributions", "Condition report", "Mood". Auto-navigated from full-time. |
| Team talks (pre/HT/FT) | Tone choice affects morale | missing | P1 | XL | HT interstitial → talk picker; tones as football verbs; reactions as opening-minutes commentary. Needs morale layer (absent). |
| Set-piece taker selection | Designate corner/FK/pen takers | partial | P2 | M | "Specialists" pickers fire `SetCornerTaker` etc.; FSM detects set pieces but no taker state. |
| Momentum / match-story strip | Period-by-period dominance | missing | P2 | M | Prose cadence from event-cluster windows, not a line chart. |
| In-match analysis (heat/pass maps, ratings) | Visual analytics | missing | P2 | XL | Prose zone summaries; "ball movement" panel; ratings as post-match star roll. |
| Touchline shouts | 8 quick encouragements | missing | P2 | L | 3 prose cues resolved vs character attribute. Needs morale/modifier layer. |
| Press conference (interactive Q&A) | Tone-choice journalist Q&A | missing | P3 | L | Extend read-only press inbox into deflect/praise/critique choices feeding narrative. |
| Concurrent other-match scores | Live ticker of other fixtures | missing | P3 | M | "Other results" strip from batch-sim outcomes as they complete. |
| Presentation modes (full/key/quick) | Watch all vs highlights vs sim | partial | P2 | S | Functional mapping exists in SpeedMode; relabel + add summary post-skip. |
| Camera/view toggle (board vs text-only) | Display modes | missing | P3 | S | Promote `Match.tsx` board-toggle to `LiveMatch`. |

### Area 2 — Tactics, roles, duties & set pieces

| Feature | Genre behavior | FW status | Priority | Effort | FW-idiom translation |
|---|---|---|---|---|---|
| Formation selection (player-facing) | Reshape positioning | missing | P0 | L | Pick one of 16 authored archetypes; arrays exist in RON but ignored. FUN-TI1 wires it. |
| Set-piece restart mechanics | Ball placement, possession award, timing gate | missing | P0 | M | `SetPieceKind` detected; restart mechanics absent. FUN-LAW1. |
| Tactical preset / archetype selection UI | Named presets bundle shape+instructions | partial | P0 | M | 16 archetypes sim-functional but unselectable; Tactics route is a stub. |
| Named player roles (Mezzala etc.) | ~45-70 named roles | missing | P1 | XL | Roles collapse into signature identity — authored signature set shapes how a slot is filled; commentary names behavior, not label. |
| Mentality / risk dial | 7-level team risk multiplier | missing | P1 | M | Prose match-intent ("Hold the result" / "Go for the win") adjusts FSM thresholds. |
| OOP team instructions (line, press, transition) | ~15 controls | partial | P1 | M | Press works; line is archetype-init-only. Bind to mid-match + add 2-3 levers. |
| Fouls, free kicks, cards, dismissals | Foul → restart, card escalation | missing | P1 | L | Q32 foul probability → `MatchEvent::Foul` → `SetPieceKind`; cards ledger-logged. FUN-LAW2/3. |
| Substitutions (squad picker) | Up to 5 subs | missing | P1 | M | `Substitute` deserializes but returns Unimplemented; no `MatchEvent::Substitution`. |
| Penalty shootouts | Alternating kicks | missing | P1 | M | Deterministic Q32 gates per kick; `MatchEvent::PenaltyKick` to ledger. FUN-LAW4. |
| Role duties (Attack/Support/Defend) | Duty tiers | missing | P1 | L | "Defensive contribution" axis baked into signature definitions; no slider. |
| IP team instructions (directness/width/build-up) | ~20 controls | missing | P2 | L | 3-4 prose dials over existing `buildup_speed_factor_bps` etc. |
| Individual player instructions | Per-player overlays | missing | P2 | L | 1-2 prose briefings per player → match-scoped BT node weight flags. |
| Positional familiarity | Natural→Makeshift ladder | missing | P2 | M | Role-affinity weights drive BT utility; surfaced as "uncomfortable at right back" prose. |
| Tactical familiarity / fluency | Built by training | missing | P2 | L | `team_cohesion` rises with repeated archetype play; unlocks combination commentary. |
| Team talks | Tone selection | missing | P2 | M | 4-5 prose stances shift `morale_modifier`; Tracery-driven. Needs morale. |
| Corner/FK routines | Configurable routines | missing | P2 | L | Two authored routines per team; taker from physical signature profile. Downstream of FUN-LAW1. |
| Tactical visualizer (zone shape) | Zone-grid shape preview | missing | P2 | L | Repurpose `TacticalBoard` to show centroids from last 3 matches + zone text labels. |
| Opposition instructions (per-opponent) | Man-mark / show-onto-foot | missing | P3 | M | Pre-match "danger player focus" biases marking; scout-recommended. |
| Touchline shouts | Emotional instructions | missing | P3 | S | One-click cue + brief commentary + short BT modifier. Needs morale layer. |
| Saved tactics slots + familiarity | 2-5 named tactics | missing | P3 | L | Two named slots; familiarity from ledger prose. Needs formation first. |
| IP/OOP dual formations | Two shapes (FM26) | missing | P3 | XL | FSM states could carry own centroids; sim parameterization, not a dual editor. |

### Area 3 — Training & player development

| Feature | Genre behavior | FW status | Priority | Effort | FW-idiom translation |
|---|---|---|---|---|---|
| Breakthrough-driven PA/CA redraw | (FW-native dev model) | present | P0 | S | 10 readiness meters fill from ledger; the match *is* the training session. Wired in `advance_season_inner`. |
| Signature activation via breakthrough | (FW-native) | present | P0 | S | `SignatureAwakening` fires from meter; surfaced as a readable moment. |
| Age-stage development curves | Training-vs-experience by age | partial | P1 | M | `age_years` field exists but is **always set to hardcoded 24** and never read in `evaluate()`. Wire into PA redraw ceilings + regressive-pressure multiplier. |
| Mentoring mechanic | Senior shifts junior personality | partial | P1 | M | `MentorTeammate` (class 15) weighted in readiness tables but **no emitter**. Detect co-appearances; optional nominate-pair UI. |
| Match sharpness | Built by minutes, drained by rest | missing | P1 | M | Q32 field; surfaced as "looks rusty" prose; decay/recover in season advance. Field is inert. |
| Condition / intensity management | Fatigue + workload safety | missing | P1 | L | Selection *is* the intensity control; match-load counter → injury-prob modifier. No schedule grid. |
| Directed individual training focus | Per-player attribute targeting | missing | P2 | M | One focus per player/season = per-family readiness multiplier (1.5×). Maps cleanly onto 10 meters. |
| Position/role retraining | Versatility-gated, 6-12 mo | missing | P2 | L | Per-player role-affinity delta accumulates from appearance-events; Versatility gene gates rate. |
| Coaching staff (9 areas, hire) | Staff modify training | missing | P2 | XL | Coaches as named ledger characters; specialist raises a family's fill-rate. Multiplier layer, not load-bearing. |
| Support staff (physio/scientist/HoYD) | Role-specific effects | missing | P2 | L | Collapse into club-level attributes (physio_quality, youth_infrastructure) for EA scope. |
| Development Centre hub | Youth/loan monitoring | missing | P2 | M | Dev screen shows family readiness + last breakthrough/collapse + sharpness as text. |
| Facilities infrastructure | Training quality tiers | missing | P3 | M | `training_infrastructure: Q32` on Club → fill-rate multiplier. |
| Weekly training schedule grid | Session-type composition | missing | P3 | XL | Architecturally misaligned with pillar 3. Optional M-effort seasonal-emphasis dial as the only honest variant. |
| Youth academy intake / newgen | Annual cohort generation | missing | P1 | XL | Runtime procgen cohort seeded from club region/infrastructure; core to multi-season life. Planned T4.5-E1. |

### Area 4 — Player attributes, personality & morale

| Feature | Genre behavior | FW status | Priority | Effort | FW-idiom translation |
|---|---|---|---|---|---|
| Multi-category visible attributes | 38-47 fields, 1-20 | present | P0 | S | 38 Q32 projected to 1-20 at DTO boundary; scouting shows ranges. Live. |
| Hidden CA/PA ceiling | 0-200 ceiling | present | P0 | S | `AbilityCeiling` with breakthrough-gated mutation. Live. |
| Hidden personality vector | 13-14 hidden axes | partial | P0 | S | 14-field vector wired into BT via 21 k-constants; no career-loop consumer (morale/dev rate) yet. |
| **Morale as a live performance multiplier** | 8-label live multiplier | **missing** | **P0** | L | Hidden Q32 biases BT utility per tick; read via scout/press prose. `PlayerCondition.morale` defined but **inert — zero `fw-match-sim` consumers, not persisted**. Biggest P0 in the project. |
| Match sharpness | Biggest perf lever | missing | P1 | M | Q32 ticks with appearances; low sharpness multiplies BT utility down. Inert field. |
| In-match fitness drain | Tire over 90 min | missing | P1 | M | `match_fitness` drains per tick by stamina; multiplies physical-action utility. No fatigue model. |
| Form tracking | Rolling performance avg | missing | P1 | M | Q32 decay accumulator updated each advance_week. Field defined, no computer. |
| Age curves (peak/decline) | Physical peak, mental growth | missing | P1 | L | Age tick → family-specific CA deltas. No age tick; CAREER_START_AGE hardcoded. |
| Role-affinity scoring (role fitness) | 117 role weight tables | partial | P1 | M | Tables exist for FW-VAL; not wired to live fitness display or selection. |
| Injury simulation | Proneness × intensity | missing | P1 | XL | `DurabilityProfile` exists; `InjuryLongTerm` (27) weighted but never emitted. Roll per week → ledger → regressive pressure. |
| Personality archetype labels | Named labels from hidden axes | missing | P2 | M | Prose archetypes from vector ("hard-nosed pro with a short fuse"); no enum shown. |
| Squad hierarchy / dressing-room | Tiers + contagion | missing | P2 | XL | `RivalryFormed`/`MentorTeammate` defined, not emitted; press surfaces factions. |
| On-pitch chemistry (co-play bond) | Pair-bond accumulation | missing | P2 | L | Q32 pair-bond from shared matches; multiplies pass/press utility. |
| Mentoring / tutoring | Group personality drift | missing | P2 | L | Mentor's vector nudges mentee; `MentorTeammate` ready. |
| Playing-time promises / status | 11 status tiers | missing | P2 | L | `PromisedYouthMinutes`/`BrokenPromise` defined+routed, no emitter/UI. |
| Manager-player interactions | 2000+ dialogue variants | missing | P2 | XL | Press inbox callbacks live; active interactions = choices emitting events; Tracery prose, no tree. |
| Player adaptation/settling | Foreign penalty arc | missing | P3 | M | `settling_arc` from cultural distance; `adaptability` axis exists, no consumer. |
| Wage/contract fairness | Peer comparison morale | missing | P3 | L | Hidden satisfaction from wage-vs-median biases loyalty. `contract_status` hardcoded None. |
| Position conversion | Versatility-gated | missing | P3 | M | `versatility` axis exists, no consumer; conversion_progress tick. |

### Area 5 — Squad dynamics & dressing room

| Feature | Genre behavior | FW status | Priority | Effort | FW-idiom translation |
|---|---|---|---|---|---|
| Morale as a live sim input | Continuous, spreads via groups | partial | P0 | L | Q32 morale on `PlayerCondition`; wire into BT alongside personality bias. Field inert. |
| Morale drivers (time/form/results/contract) | Subcategory states | missing | P0 | M | Season-tick delta from minutes-share, results, unsatisfied-promise events. |
| Form decay from results/inactivity | Form rises/decays | partial | P1 | S | Field exists; read last-N results, apply delta, feed BT confidence. |
| Match-fitness drain/ramp | Depletes/accumulates | partial | P1 | S | Field exists; per-tick drain → stamina BT multipliers; between-match ramp. |
| Squad hierarchy (4 tiers, asymmetric ripple) | Leader unhappiness ripples | missing | P1 | M | Tier from reputation/apps/personality; tier-scaled morale propagation. |
| Social groups / contagion | Tenure groups spread morale | missing | P1 | L | Derived groups weight morale-spread; surfaced as "a faction has formed". |
| Captain mechanic | Mediator from leaders | missing | P1 | M | Designate via IPC, validate vs personality, emits memory event; minor spread multiplier. |
| Player-to-player relationships | Friend→Dislike dyads | missing | P1 | L | `BTreeMap<(Pid,Pid),Score>` from co-appearance; partnership lines on board. |
| Tactical cohesion ramp | Familiarity accrues | missing | P1 | M | Q32 cohesion from same-XI tick runs; BT multiplier on coordination. "Cheapest high-leverage win." |
| Promise system | Make/track/break | partial | P1 | M | `PromisedYouthMinutes`/`BrokenPromise` (6/7) exist; need IPC + season-end eval. Pillar 2 direct expression. |
| Unhappiness escalation | Formal transfer-request states | missing | P1 | L | Morale thresholds → ledger state transitions → inbox. `TransferRequested`/`Refused` (10/11) exist. |
| Mentoring assignment | Personality drift | partial | P2 | L | Assign veteran→youth; compatibility-scaled vector nudge feeds breakthrough. |
| Team talks (pre/HT/FT) | 6 tones | missing | P2 | M | Tone set; per-player reaction from temperament/pressure axes; Tracery paragraph per phase. |
| One-on-one interactions | Conversation system | missing | P2 | XL | 4-6 action verbs → personality-weighted prose response → memory event. Minimal version is M. |
| Aggregate dressing-room panel | 5-indicator hub | missing | P2 | S | Prose summary paragraph on home view from morale/hierarchy/events. |
| Personality archetype display | Named labels (scout-reported) | missing | P2 | S | Threshold table + Tracery → "iron-willed", "mercurial"; pillar-4 framing. |
| Individual player targets | Linked to contract/promises | missing | P2 | M | Structured ledger conditions evaluated at season-end; multi-season is ledger-native. |
| Code of conduct / fines | Conduct rules + fines | missing | P3 | M | Worth it only if infractions shift temperament axis. |
| Squad/team meetings | Short-term morale | missing | P3 | S | Subset of team-talk mechanics. |
| Manager personality types | OOTP archetypes | missing | P3 | S | Ledger-derived identity from accumulated choices, not a preset. |

### Area 6 — Transfers, contracts & negotiation

| Feature | Genre behavior | FW status | Priority | Effort | FW-idiom translation |
|---|---|---|---|---|---|
| Transfer window indicator | Open/closed pill | present | P1 | S | `computeTransferWindowState`. Done. |
| Player transfer search/discovery | Filterable DB | missing | P0 | L | "Market" = scout-observed players filtered through uncertainty; unscouted = invisible. Discovery is a scouting investment. |
| Squad/wage budget tracking | Transfer + wage budgets | missing | P0 | M | Q32 fields on club; framed as prose ("modest funds"). Gate every bid. No finance fields exist. |
| Contract terms (wage/length/status/bonus) | Full term editor | missing | P0 | XL | Wages as internal Q32, surfaced as "best-paid in squad"; squad-status = promise mechanic. No contract fields. |
| Transfer bid structure (add-ons, sell-on, buyback) | Layered bids | missing | P0 | XL | Fees procedurally anchored; add-ons emit ledger consequences years later. |
| Contract negotiation (renewal/walkout) | Multi-stage | partial | P0 | L | `ContractRenewalRejected`/`Accepted` (8/9) reserved; text-choice dialogue, no sliders. |
| Playing-time promise tracking | Promise → consequence | partial | P0 | M | Most FW-native item; `PromisedYouthMinutes`/`BrokenPromise` reserved; broken promise = callback 5 seasons later. |
| Transfer request system | Request → list/reject/negotiate | partial | P1 | M | `TransferRequested`/`Refused` reserved; refusal = future press callback. Needs morale. |
| Loan mechanics | Standard/loan-to-buy/recall | missing | P1 | L | Temporary `club_id` reassign; loan-club scouts contribute observations (pillar 4); recall-on-breakthrough drama. |
| Free transfers / pre-contract | Bosman / tribunal | missing | P1 | M | Expiry-driven variant; tribunal fee scaled to breakthrough history. |
| Sold-under-protest / deadline-day | Chairman forced sale | partial | P2 | M | `SoldUnderProtest`/`BoughtOnDeadlineDay` (12/13) reserved; pure memory-pillar. Needs board confidence. |
| Agent / intermediary | Agent friction | missing | P2 | M | Procedural agent character; deterministic demand %; surfaces in callbacks. |
| Rival counter-bids | Bidding wars | missing | P2 | L | Seeded AI targets; rival bid = press event; `TransferBattleWon` callback. |
| Promotion/relegation pricing | Tier-change cascades | missing | P2 | M | Pure function of new tier; fire-sale flag reduces leverage. |
| Club finances (amortization, FFP) | P&L + governance | missing | P2 | L | FW invents its own governance per tier; embargo = ledger event. |
| Foreign quotas / home-grown | Caps + permits | missing | P2 | M | "Foreign" defined per-competition by world-gen; ties to culture system. |
| Multi-year installments | Deferred liabilities | missing | P3 | M | `(tick, amount)` list emitting consequences at scheduled ticks. |
| Manager job-market & contract | Job board + sacking | missing | P1 | XL | "Structural keystone for half this dimension"; reputation gates jobs; sacking = callback. |

### Area 7 — Scouting & recruitment

| Feature | Genre behavior | FW status | Priority | Effort | FW-idiom translation |
|---|---|---|---|---|---|
| Single-scout uncertainty (bands) | (FW EA floor) | present | P1 | S | Banded estimates as football-native confidence text. Live. |
| Scout bias / false positives / dropped labels | Differential accuracy | partial | P1 | M | Path-B reports every true label, no drops/false positives. Path-A bias-filter makes two scouts diverge — most important missing pillar-4 mechanic. |
| Multiple scouts / network | Hire/fire, staff attrs | partial | P1 | L | 1-3 named archetype scouts; disagreement IS the headline. 6 enum slots reserved, all biases zero. |
| Scout assignment system | Directed observation | missing | P1 | M | "Watch their striker 3 fixtures"; consumes weeks. Currently auto-fires on every starter. |
| Recruitment focus / search | Structured filters | missing | P1 | M | Prose focus → 2-3 banded candidates. No search exists; pool is 22 bios until T4.5+. |
| Band narrowing over observation | Knowledge accumulation | partial | P1 | S | `observation_count` seeds RNG but bands don't actually narrow. Make noise amplitude a function of count — makes "truth emerges" visible. |
| Scout prose synthesis | Prose report IS the surface | missing | P2 | M | Paragraph from label+category bands via Tracery; no raw numbers. Pillar-4's most visible missing output. |
| Regional knowledge | Per-region familiarity | partial | P2 | M | `familiar_regions`/`regional_noise_penalty` exist but zero. Tighter bands for known cultures. |
| Scout track record | Reliability over seasons | missing | P2 | M | Accurate calls → trusted scout → tighter future estimates. |
| Shortlists / watchlist | Named watchlists | missing | P2 | M | `Vec<PlayerId>` triggers auto-observation; band-progression view. Useful pre-transfers. |
| Opposition scouting (pre-match) | Team report | missing | P2 | M | Prose from opponent's recent sim data via Tracery. |
| Transfer market integration | Bids/negotiation | missing | P1 | XL | Largest missing domain; prereq for scouting payoff. Table-bid-accept EA floor. |
| Data scouting / analytics | Stats search | missing | P3 | L | "Numbers scout" vs field scout — the data-vs-bias tension is FW-specific. |
| Youth scouting / intake | Academy pipeline | missing | P3 | L | Youth tier gates intake quality; breakthrough is the payoff. |
| Trial system | Short eval contracts | missing | P3 | M | Two-week look accumulates observations fast. |
| Squad planner | Depth chart | missing | P3 | M | Text-first depth/gap prose summary. |
| Scouting budget | Tiered coverage | missing | P3 | S | Budget gates assignment slots + reach. |

### Area 8 — Youth academy & newgen pipeline

| Feature | Genre behavior | FW status | Priority | Effort | FW-idiom translation |
|---|---|---|---|---|---|
| Career-start procgen (names/league/roster) | (FW foundation) | present | P0 | — | `generate_team`/`generate_league_with_teams`; 440-instance roster. Live. |
| Annual youth intake (newgen) | Per-season cohort | missing | P1 | L | Season-end cohort seeded by region + academy prestige; core to world feeling alive. |
| Per-save PA variability (range bands) | Hidden ceiling revealed over time | missing | P1 | S | Draw `potential` from a range band at intake; scout bands are the only window. Small — AbilityCeiling exists. |
| Multiple scout archetypes | Diverging biases | partial | P1 | M | Author 3-5 live bias vectors; disagreement as prose. Infra present. |
| Age-stage arc logic | Physical→technical→mental | missing | P1 | M | Family→stage map; age multiplier on readiness fill. Engine already accumulates per-family. |
| Mentoring (senior→youth) | Trait contagion | missing | P2 | M | `MentorTeammate` ready; pairing + emission + Leadership readiness unlock. |
| Academy facility tiers | Board-gated upgrades | missing | P2 | M | Single prestige scalar (1-4) feeds intake draw. |
| Youth candidate preview | Pre-intake grades | missing | P2 | S | Prose positional breakdown of upcoming cohort. |
| Head of Youth Development | Biases intake | missing | P2 | M | Procedural staff; personality skews NarrativeFlag distribution of intake. |
| National youth quality | Per-nation modifiers | missing | P2 | S | `youth_talent_density` scalar on Culture RON. |
| Scout knowledge accumulation | Per-region narrowing | missing | P2 | M | `familiar_regions` empty; observation_count_by_culture confidence modifier. |
| Scout assignment workflow | Assign by region/player | missing | P2 | M | `ScoutAssignment` struct queued via IPC. |
| Loan system | Dev loans + guarantees | missing | P2 | L | Temp club reassign; appearance-count promises → `PromisedYouthMinutes`/`BrokenPromise`. |
| Youth contract promises | Playing-time promises | partial | P2 | S | Event schema complete (6/7); only emission + UI missing. High narrative payoff. |
| Development stagnation alerts | Flag no-progress youth | missing | P2 | S | Scan readiness for no-threshold-crossing; press-question prompt. |
| Training system | Session focus | missing | P1 | XL | Training intent = second readiness channel; capped by Consistency gene. |
| Position retraining | Versatility-gated | missing | P3 | M | Intent → seasonal roll vs versatility → `PositionRoleExpanded` event. |
| Affiliate/feeder clubs | 4 partnership types | missing | P3 | L | Procgen affiliate clubs; expands scout regions. |
| Home-grown registration | HGC/HGN rules | missing | P3 | M | Maps to culture-of-birth, not nationality. |
| Development Centre hub | Unified monitoring | missing | P2 | M | `/development` route; intake dossiers + uncertainty bands + breakthrough candidates. |
| Youth competitions / reserves | U18/U21 leagues | missing | P3 | XL | Parallel simplified sim; loan system approximates the minutes-as-dev effect. |
| Trait teaching | Targeted trait training | missing | P3 | M | `teach_narrative_flag` intent rolls vs age/coaching/professionalism. |

### Area 9 — Staff, backroom & board

| Feature | Genre behavior | FW status | Priority | Effort | FW-idiom translation |
|---|---|---|---|---|---|
| Pre-match lineup/formation selection | Set XI + shape | missing | P0 | L | `set_lineup` writes intent before advance; board renders shape. Without it, the manager has no pre-match decision. |
| In-match adjustments (subs/press/shape) | Dugout panel | partial | P0 | M | Most implementation-ready gap — `MatchCommand` enum + IPC exist; all 9 return Unimplemented; wire through intent queue. |
| Transfer market — search & offers | Bid/negotiate/sign | missing | P0 | XL | Turn-based: focus target → bid → multi-round at week-advance → inbox. Largest single missing system. |
| Contract system (wages/length/clauses) | Negotiated contracts | missing | P0 | L | Wage/expiry/clause struct on PlayerInstance; the budget constraint that makes squad-building meaningful. |
| Assistant Manager (advice + delegation) | Pre-match advice | missing | P1 | M | Ledger-projection prose in match inbox; no named entity needed in MVP. |
| Board confidence (5 axes) | Independent ratings → sacking | missing | P1 | M | 5 Q32 scalars updated season-end from ledger events; text verdicts. No board field in `CareerState`. |
| Injury & match-load tracking | Sports scientist | missing | P1 | L | Q32 fatigue → InjuryRisk inbox → `InjuryLongTerm` (27, never emitted). Prereq for rotation. |
| Morale & team dynamics | Individual + cohesion | missing | P1 | M | `morale` field never mutated; MoraleUpdate pass at week-advance. |
| Delegation system (~15 tasks) | Delegate operations | missing | P1 | L | `BTreeMap<Category, Option<StaffId>>`; staff archetype decides, prose summary, override. |
| Coaching staff taxonomy | 18+ roles | missing | P2 | XL | Content-pack entities; effect as training-summary modifier text. |
| Backroom advice channels | Proactive advice | missing | P2 | M | Tagged inbox items from ledger-reader projections. |
| Multi-scout network + regional | Coverage % | missing | P2 | L | Hireable scouts with region affinity; assignment as delegation. |
| Scout specialization | 5 archetypes | partial | P2 | M | Enum + biases exist; only Basic constructed. |
| Recruitment focuses/shortlists | Filters → candidates | missing | P1 | M | Filter struct; matching candidates as inbox items. |
| Opposition analysis | Pre-match report | missing | P2 | M | Ledger-reader projection over opponent MatchEvent history. |
| Youth intake + HoYD | Annual newgens | missing | P2 | L | Deterministic procgen pass biased by HoYD archetype. |
| Club Vision / 5-year plan | Tiered objectives | missing | P2 | M | RON entity; prose commitment at career start. |
| Board requests/ultimatums | Inbox decisions | missing | P2 | M | Inbox items with tiered response → ledger events. |
| Chairman hidden attributes | Patience/Interference | missing | P3 | S | Q32 scalars shaping request distributions. |
| Mentoring | Senior→junior | missing | P2 | M | `MentorTeammate` (15) ready; breakthrough evaluator consumes it. |
| Loan management | Playing-time tracking | missing | P3 | M | Loan contract variant + delegation policy. |
| Manager creator / identity | Background + personality | missing | P2 | M | New-career step seeds AI archetype + board baseline. Makes the career personal. |
| Coaching badges | Badge tiers | missing | P3 | S | Content enum surfaced as prose qualifier. |

### Area 10 — Finance, budgets & infrastructure

| Feature | Genre behavior | FW status | Priority | Effort | FW-idiom translation |
|---|---|---|---|---|---|
| Transfer/wage budget split | Manager-controlled envelope | missing | P1 | L | Board prose envelope; internal Q32 gates every transfer/contract. Prereq for the domain. No finance fields exist. |
| Wage negotiation / contract terms | Agent demands | partial | P1 | L | Prose expectations + structured offers; `ContractRenewal*` events exist, no fields/loop. |
| Transfer fee mechanics (installments/add-ons/clauses) | Structured fees | missing | P1 | XL | Named structural options; ledger records structure for later callbacks. |
| Loan mechanics (fee/wage split/option) | Loan variants | missing | P2 | M | Narrative-framed; tracks loan breakthrough on return. |
| Window enforcement / deadline | Locked outside windows | partial | P2 | S | Window computed; enforcement blocks commands once transfers exist. `BoughtOnDeadlineDay` (13) hook ready. |
| Matchday/gate revenue | Capacity × attendance | missing | P2 | M | Capacity attribute; attendances narrated; Q32 income, invisible. |
| Broadcasting / prize money | Merit + pool | missing | P2 | M | Prize as narrative event scaled to fantasy tier reputation. |
| Board confidence + Club Vision | 5-dim + 5-year plan | missing | P1 | XL | Board as ledger voice; `EmitterKind::BoardSystem` exists, nothing emits. Results → tone → stability loop is P1 for career depth. |
| Financial sustainability (FFP/PSR equiv) | Break-even sanctions | missing | P2 | L | Fantasy governing body issues prose warnings; breach → board ultimatum. |
| Training/youth facility levels | 10-level scales | missing | P2 | L | Tier integer modifies breakthrough probability + scouting radius; board-requested. |
| Commercial / sponsorship | Board deals | missing | P3 | M | Procedural sponsors; endorse/decline as relationship signal. |
| P&L / amortization | Accounting | missing | P3 | L | Internal only; never shown as a balance sheet. |
| Stadium capacity / expansion | Multi-season build | missing | P3 | L | Single integer; expansion as multi-season board decision. |
| Ownership / takeovers | Ambition + risk | missing | P3 | M | Procedural ownership attribute sets budget narrative + ambition. |
| Agent fees | Intermediary cut | missing | P3 | M | Named procedural agent; fee described narratively. |
| Parachute payments | Declining post-relegation | missing | P3 | S | Board announcement + named credit over seasons. High narrative leverage. |

### Area 11 — Competitions, leagues & international

| Feature | Genre behavior | FW status | Priority | Effort | FW-idiom translation |
|---|---|---|---|---|---|
| Single-tier 20-club league + standings | Round-robin + table | present | P0 | S | Procedural league, circle-method schedule, live standings. Fully playable. |
| Tactical lineup/formation + in-match commands | Set XI + adjust | partial | P0 | L | Managed club's players flow into sim via `build_slot_signatures`; all 9 `MatchCommand` return Unimplemented; no formation/sub UI. Core interactive surface. |
| Transfer market | Full transfer loop | missing | P0 | XL | "Single biggest table-stakes hole"; transfer events drive memory density. |
| Multi-tier pyramid (6 tiers) | Living pyramid | missing | P1 | L | 6 procedural tiers; unmanaged tiers advance from seed. T4.5-B. |
| Promotion/relegation | Tier movement + consequences | missing | P1 | M | `PromotionWon`/`RelegationSuffered` (events + callbacks exist); movement logic absent. |
| Cup competition | Bracket + rounds | missing | P1 | M | Procedurally named cup; `CupFinalWin/Loss` (18/19) exist; `CupRunDepth` breakthrough trigger ready. T4.5-F. |
| Window enforcement | Hard deadlines | partial | P1 | S | Window computed in UI; backend enforcement absent (market absent). |
| Club finances / prize / parachute | Budgets + payments | missing | P1 | L | Procedural club revenue tiers; prize as season-end news; parachute as board event. |
| Fixture calendar | Auto-generated skeleton | missing | P1 | M | Deterministic calendar; cup interleaving gated on bracket; pre-season as board choice. |
| Squad registration rules | Homegrown/foreign caps | missing | P2 | M | Procedural "local talent floor" + "loan cap" per world-seed. |
| Split-season / alt formats | Apertura/conference | missing | P2 | M | Per-nation league character determines format. |
| Youth/reserve competitions | U18/U21 leagues | missing | P2 | L | Breakthrough-factory; age-eligibility per competition. |
| Pre-season scheduling | Friendlies/tours | missing | P2 | S | Board-proposed tour text-choice; rapid-sim results. |

### Area 12 — Media, press & interaction

| Feature | Genre behavior | FW status | Priority | Effort | FW-idiom translation |
|---|---|---|---|---|---|
| Unified news/inbox hub | Career heartbeat | missing | P0 | M | Dedicated Hub route aggregating all reader outputs into a prioritized action feed. Most structurally impactful for making the loop alive. Career inbox is the embryo. |
| Morale feeding performance | Live multiplier | missing | P1 | L | Morale contributes BT utility modifier at the signature_readiness site; shifted by talks/promises/results. `PlayerCondition.morale` inert. |
| Interactive press conference | Q&A with effects | partial | P1 | L | Inbox ranked candidates ARE implied questions; add 2-4 response framings emitting events readers consume. PressReader wired. |
| Manager-player conversations + promises | Severity tiers, tracked | missing | P1 | L | `TransferRequested`/`PromisedYouthMinutes`/`BrokenPromise` exist; surface obligations + resolution framings. |
| Team talks (tone + targeting) | 5 tones | partial | P1 | M | `MatchCommand::TeamTalk` deserializes; needs sim handler → MoraleShift → per-player reaction DTO. |
| Real-time matchday commands | Touchline | partial | P1 | M | Sim-completion task; wire types done, handlers + downstream effects missing. |
| Board confidence + Club Vision | Tracked objectives | missing | P1 | L | Season mandate prose; `EmitterKind::BoardSystem` exists, nothing emits; surfaced as board statements. |
| Individual player targets | Training/perf | missing | P2 | M | `PlayerTarget` struct; resolver emits TargetMet/Missed; CoachReader surfaces. |
| Squad dynamics (pyramid/groups/mentoring) | Social sim | missing | P2 | XL | Ledger-derived influence ranking; mentoring maps to breakthrough; no separate pyramid UI. |
| Supporter confidence / fan sentiment | Segments → framing | partial | P2 | M | FanReader implemented + tested but **not IPC-wired**; add `get_fan_sentiment` + render. Small remaining work. |
| Code of conduct / discipline | Auto conversations | missing | P2 | M | CareerPolicy flags; violations → `DisciplinaryIncident` → CoachReader. Needs conversation system. |
| Journalist personas / question banks | Persona pool | missing | P3 | M | Procedural press archetypes in RON; PressReader ranking is implicit questions. |
| Social media feed | Separate screen | missing | P3 | S | One panel in the Hub, not a screen. FanReader data already produced. |

### Area 13 — Data, analytics & UI surfaces

| Feature | Genre behavior | FW status | Priority | Effort | FW-idiom translation |
|---|---|---|---|---|---|
| League table + fixtures + 3 charts | Standings + ECharts | present | P1 | — | Table, fixtures, ranked-bar/scatter/trend. Live (no xG). |
| **PlayerAssessmentDTO** (stars/chip/radar/verdicts) | Radar + role score + pros/cons | **missing** | **P0** | L | Squad-row stars + identity chip; radar solidifies from dots→polygon as scouting builds; tier-words replace 1-20. **Spec complete; `get_player_assessment` does not exist.** Largest user-visible data gap. |
| xG surfaced to player | Per-shot + running + story | missing | P1 | M | Post-match "chance quality favoured them" prose; running xG as fuzzy band. xG computed but discarded after goal draw. |
| Post-match HT/FT stat summary | Shots/possession/ratings | partial | P1 | M | HT interstitial + per-unit prose verdicts; possession available; shot counts client-accumulated. |
| Top scorers/assists leaderboard | Sortable table | missing | P2 | S | `PlayerSeasonStats` in save; `get_top_scorers` + TanStack table. |
| Directed scouting / recruitment focus | Region/age briefs | missing | P2 | XL | Scout brief → banded dossier; intersects career-roster + transfers. |
| Player comparison | Side-by-side radar | missing | P3 | M | Two assessment panels + natural-language diff. Depends on PlayerAssessmentDTO. |
| Heatmap / positional frequency | Zone overlay | missing | P2 | M | Ball-zone % bars; client-accumulate per-tick positions into a zone grid. |
| Momentum graph | Rolling curve | missing | P2 | S | 5-segment timeline bar from xG-weighted event density. |
| Per-player live ratings | 1-10 | missing | P2 | M | Verdict sentence per player; `average_rating_numerator` accumulates but unexposed. |
| Dev tracking graph | Attribute change over time | missing | P2 | L | Text career arc from breakthrough events; no attribute snapshot history. |
| Pass map | Volume/accuracy/direction | missing | P3 | L | Zone-to-zone matrix prose; needs passer/target coords (sim change). |
| PPDA / pressing metric | Press effectiveness | missing | P3 | S | "Pressed relentlessly" prose from pressing utility internals. |
| Scout archetype variety | JCA/JPA accuracy | partial | P2 | M | Archetype voice changes dossier; enum declared, only Basic used. |
| Squad gap detection / depth | Depth chart | missing | P2 | M | Prose gap report from slot-coverage; pure frontend on existing data. |
| Column customization | Configurable columns | missing | P3 | M | TanStack v8 visibility API; needs more columns in roster DTO. |
| Home/away splits | Per-club splits | missing | P3 | S | Expandable rows from fixture data. |
| Historical season tables | Past standings | missing | P2 | M | Store `StandingsSnapshot` per season (schema bump); season-advance currently discards. |
| Formation effectiveness | Win rate by shape | missing | P3 | L | Needs formation tracking; downstream of formation system. |
| Next-opponent dossier | Opposition analysis | missing | P2 | M | Form + danger man (M); tactical tendency (L). |
| Financial screens | Budgets/wages/FFP | missing | P2 | XL | Band language ("tight/comfortable"); raw numbers behind power-user toggle. No finance model. |
| Watchlist | Scouting watchlist | missing | P2 | M | `Vec<PlayerId>`; band progression view. Useful pre-transfers. |
| Age profile / squad balance | Age pyramid | missing | P3 | M | Prose spine summary; **age absent by design until career-roster layer.** |

### Area 14 — Manager career, reputation & history

| Feature | Genre behavior | FW status | Priority | Effort | FW-idiom translation |
|---|---|---|---|---|---|
| Career history (record/trophies/awards) | Per-club record + timeline | partial | P1 | M | Ledger records title/promotion/cup events; render as timeline; W/D/L from match events; awards as projection. `get_career_overview` returns only champion-per-season. |
| Manager identity (name/profile/style) | Builder + fingerprint | missing | P1 | M | Pick name + procedural philosophy from `ManagerArchetype` RON. Player never names own manager today. |
| Manager reputation | 0-10,000 gate | missing | P1 | M | 0-100 ledger-derived read-only projection; gates jobs/leverage/patience; surfaced as prose. |
| Board relations / objectives | 1-20 + Club Vision | missing | P1 | L | Narrative season mandate; `EmitterKind::BoardSystem` is a placeholder only. |
| Press conferences | Tone/gesture choices | partial | P2 | L | Curated post-match inbox items + tone choice → ledger → morale/headlines. PressReader empty until events fire. |
| Player/staff meetings + team talks | Targets + promises | missing | P2 | L | Tone choice → morale modifier; first-youth-appearance auto-emits promise; `BrokenPromise` callback. Stubs (6/7) never emitted. |
| Seasonal manager awards | MoM/MoY/HoF | missing | P2 | S | Rank by outcome formula → `ManagerAwardWon` event (schema bump) → timeline. |
| Tendencies fingerprint | Behavior-derived (FM26) | missing | P2 | M | Ledger-derived tags ("relies on academy graduates") from event patterns. Read-only projection. |
| Job market — vacancies/dismissal | Reputation-gated | missing | P2 | XL | Season-end vacancy notices filtered by reputation; dismissal → re-entry at penalty. `managed_club_id` is session-only, not persisted. |
| Interview/appointment flow | Choice responses | missing | P2 | M | 2-3 prompts anchor season-1 mandate. Depends on job market. |
| Contract negotiation (manager) | Wage/length/clauses | missing | P3 | M | No manager wages (no licensed finance); drama is commitment horizon + trust. `ContractRenewal*` (8/9) never emitted. |
| Coaching attributes (sliders) | Numeric attrs | missing | P3 | M | Philosophy label IS the identity; dev driven by breakthrough ledger, not coaching numbers. |
| Dual role (club + international) | Simultaneous | missing | P3 | XL | `InternationalCallUp` (28) has breakthrough weights; needs national-team entity first. Post-EA. |

### Area 15 — Modding, editor, database & customization

| Feature | Genre behavior | FW status | Priority | Effort | FW-idiom translation |
|---|---|---|---|---|---|
| Mod overlay loader (runtime walk) | Load mods at start | partial | P1 | M | Walk `content/mods/`, last-writer-wins per ID, BLAKE3 fingerprint into save. Loader is a stub (T2-3 TODO); data contract complete. |
| Community culture / name-bank packs | Alt name databases | missing | P1 | S | `content/mods/<id>/cultures/*.ron`; closest analogue to FM's DB ecosystem. Docs + loader + one example. |
| In-game mod browser | Enable/reorder mods | missing | P2 | M | Settings screen lists packs; toggle at next career; load-order = lexicographic key. |
| Pre-game world editor | Inspect/edit entities | missing | P2 | L | "World Workshop" edits name banks, archetype mix, tier structure as a user mod overlay. |
| Create-a-Club wizard | Custom club | missing | P2 | M | Name + palette + procedural badge + founding philosophy (archetype); SVG badge. |
| Community signature/archetype packs | New roles/systems | missing | P2 | M | RON packs validated by FW-VAL; must use existing trigger/effect vocabulary. |
| Commentary/template packs | Extend phrases | missing | P2 | S | `*.tracery.json` adds variants; banned-terms lint at load. |
| Custom competition editor | League structure | missing | P3 | L | Pre-career pyramid seed parameters feed the generator. |
| Steam Workshop integration | Subscribe/auto-update | missing | P3 | L | RON mods well under 200MB; `steamworks-rs` planned T5-2; in-game manager becomes Workshop UI. |
| UI skinning / themes | Community skins | missing | P3 | L | Tailwind token override (colour/font); layout skinning out of scope. |
| Save portability / sharing | Share saves | missing | P3 | S | Self-contained envelope; needs file-picker + mod-mismatch warning. "Share my world" value. |

---

## 3. Recommended build sequence

The ordering follows three principles: **(1) unblock the three table-stakes loops in dependency order; (2) prefer "wire the existing substrate" over "design a new system" — FW has paid the scaffolding cost already; (3) every cluster must produce a visible, playable improvement, not internal plumbing alone.** Within each tier, items are roughly sequenced by dependency.

### Near-term — make it a manager game you can play one match in

These close the P0 match-management and morale holes. Nothing here requires the transfer/finance layer, and each step makes the existing live-match loop more of a game.

1. **Wire the in-match command handlers (subs, press already done, tempo, formation-at-restart).** This is the most implementation-ready P0 in the project — the `MatchCommand` enum, IPC, and frontend buttons exist; only the sim handlers return `Unimplemented`. Start with `Substitute` and `ChangeTempoBias` (both have ready wire types). This turns the watch loop into a *managed* match. *(Areas 1, 2, 9, 11.)*
2. **Pre-match team selection + lineup commit.** A roster panel that writes a `set_lineup` intent consumed by the match instead of attribute auto-fill. This is the single largest "feels like a manager game" gap and it gates everything tactical. *(Areas 1, 9.)*
3. **Make `PlayerCondition` live: morale + match-fitness + form + sharpness wired into BT utility.** All four fields are *defined and inert*. Morale is the highest-leverage single wire in the codebase — it is the connective tissue for team talks, promises, press responses, and board confidence. Match-fitness/sharpness are the prerequisite for meaningful substitution and rotation decisions. Do morale and fitness first (they directly change on-pitch output), form and sharpness second. *(Areas 3, 4, 5, 12.)*
4. **Team talks (HT/FT) over the new morale layer.** `MatchCommand::TeamTalk` deserializes; add the sim handler → MoraleShift → per-player reaction prose. This is the first interactive man-management surface and it lands cheaply once morale exists. *(Areas 1, 2, 5, 12.)*
5. **Formation selection (pre-match) from the 16 authored archetypes.** The arrays exist in RON and are ignored; FUN-TI1 is the planned wire-up. Pair with the live `ChangeFormation` restart shift. XL because it touches the positional model, but it removes the hardcoded-4-3-3 ceiling that limits every tactical feature downstream. *(Area 2.)*
6. **Set-piece restart mechanics + fouls/cards (FUN-LAW1/2/3) and penalties/shootouts (FUN-LAW4).** These complete the laws of the game — without fouls and restarts the match is not a faithful football sim. Taker designation rides on top. *(Areas 1, 2.)*

### Mid-term — make a career you can live inside for several seasons

These close the squad-over-time and career-identity holes. They depend on the near-term morale/condition layer and on each other.

7. **The unified news/inbox Hub route.** Aggregate all five readers (Press/Fan/Coach/Scout/Salience) into one prioritized action feed — the "what do I do now" anchor. FanReader is implemented but not IPC-wired (cheap win); PressReader is wired. This is the highest structural-impact frontend piece for making the loop feel alive, and it makes every subsequent system (promises, board, transfers) surface in one place. *(Areas 12, 14.)*
8. **Contracts + budgets as the foundation of squad-building.** Add wage/expiry/squad-status to PlayerInstance and Q32 transfer/wage budgets to the club (`contract_status` is hardcoded `None`; no finance fields exist). Surfaced as prose ("best-paid in the squad", "modest funds"), never as currency. This is the prerequisite for the entire transfer dimension. *(Areas 6, 9, 10.)*
9. **The transfer market core (table-bid → accept/reject, scout-gated discovery).** The single biggest table-stakes hole and the payoff loop for scouting. Discovery = scout-observed players (pillar 4); bids gated by the new budget. The 8 reserved transfer/contract `EventClass` discriminants finally get emitters, which lights up pillar-2 callbacks. Keep the EA floor simple (no multi-round negotiation). *(Areas 6, 7, 9, 11.)*
10. **Promise system + transfer requests + unhappiness escalation.** `PromisedYouthMinutes`/`BrokenPromise`/`TransferRequested`/`Refused` are all reserved and routed but never emitted. With morale (near-term) and contracts (step 8) in place, these become the emotional core of "careers that remember" — a broken promise resurfacing as a press callback five seasons later. *(Areas 5, 6, 12.)*
11. **Board confidence + Club Vision.** Five Q32 axes updated from ledger events (promotion/relegation/title already emitted); `EmitterKind::BoardSystem` exists with no emitter. Surfaced as board statements in the Hub. This makes results and squad decisions consequential and creates manager-stability stakes. *(Areas 9, 10, 12, 14.)*
12. **Manager identity + reputation + career history.** Name your manager, pick a philosophy, see a real career timeline. Reputation is a read-only ledger projection that gates the (later) job market. `get_career_overview` already returns champion-per-season; extend to a full timeline with awards. *(Area 14.)*
13. **Multi-tier pyramid + promotion/relegation + a cup.** T4.5-B/F. Promotion/relegation events and the `CupRunDepth` breakthrough trigger already exist. This is what makes the procedural world (pillar 1) feel like a world to climb, and it multiplies the value of every career system above. *(Area 11.)*
14. **Multi-scout disagreement + scout prose + band-narrowing-over-time.** Path-A bias filters that make two scouts diverge is the most important *missing* pillar-4 mechanic; the six archetype enum slots are reserved with zero biases. Band narrowing (an S-effort change making noise a function of observation_count) makes "truth emerges over seasons" finally visible. Pairs naturally with directed scout assignments. *(Areas 4, 7, 8.)*
15. **Annual youth intake (newgen pipeline) + age curves.** Runtime procgen cohort (T4.5-E1) is core to a career feeling alive across seasons; age-stage arcs wire the inert `age_years` field (always `24` today) into the breakthrough engine so careers age honestly. *(Areas 3, 8.)*

### Long-term — depth, longevity, and the modding moat

16. **Injuries + fatigue-driven rotation.** `InjuryLongTerm` (27) is weighted in the breakthrough table but never emitted; `DurabilityProfile` exists. With match-load tracking (near-term fitness) in place, this adds rotation strategy and squad-depth stakes. *(Areas 3, 4, 9.)*
17. **Mentoring, squad hierarchy, chemistry, and the full conversation system.** `MentorTeammate`/`RivalryFormed` reserved; the breakthrough evaluator already consumes mentor events. These deepen the dressing room once the morale spine is mature. *(Areas 4, 5, 8.)*
18. **Loans, agents, financial governance, facilities, finances UI.** The economic depth layer — meaningful once contracts/budgets/board exist. Parachute payments and fire-sales are high-narrative-leverage, low-effort additions. *(Areas 6, 7, 8, 10, 13.)*
19. **Richer analytics surfaces (PlayerAssessmentDTO radar, xG prose, post-match review, historical tables, opposition dossiers).** PlayerAssessmentDTO is the largest user-visible data gap and its spec is complete — it can move earlier if squad-evaluation friction becomes the top complaint. xG-as-prose and the post-match review screen depend on exposing the already-computed xG in `StepResult`. *(Areas 1, 13.)*
20. **The modding moat: mod loader → culture packs → in-game mod browser → Steam Workshop.** FW's RON-pack architecture is a genuine advantage over FM's broken-on-Unity skinning and 200MB Workshop limit. The mod overlay loader is a stub with a complete data contract — finishing it plus a culture-pack starter kit is the cheapest way to seed a community-content flywheel. Workshop rides on the T5-2 `steamworks-rs` pull. *(Area 15.)*

**Why this order:** Steps 1-6 turn a watchable sim into a playable match (the most acute "is this a game" gap). Steps 7-15 turn one match into a multi-season career that builds squads and remembers them (the pillars 1/2/4 payoff). Steps 16-20 add the depth and the community engine that sustain longevity. Crucially, almost every near/mid-term step is *wiring an existing field, emitting an existing event, or selecting from existing content* — the project's scaffolding discipline means the highest-leverage work is unusually low-risk.

---

## 4. Deliberately NOT building

These are genre/FM features Final Whistle rejects by design. Their absence is a choice, not a gap, and they should never appear on the roadmap. (Ruled out in `CLAUDE.md` §3 and `docs/DESIGN_DOC.md` pillar 1.)

- **Real licensed players, clubs, leagues, kits, competitions.** Pillar 1 is a procedural-fantasy world — every entity is generated with content-pack-qualified IDs (`fwh.core:player_00042`). No Premier League, no Messi, ever. FM's community real-name-fix / daily-transfer-update database moat does not translate; FW builds a different community value around content packs.
- **3D match rendering / cinematic broadcast mode.** Text-first with a 2D overhead PixiJS tactical board is the visual ceiling by design. The board can show formation centroids and shot markers — that is the visual idiom, not a stepping stone to 3D. (Also removes the need for face/logo/stadium graphics packs.)
- **Multiplayer / co-manager / online leagues.** Single-player procedural-fantasy only.
- **Mobile.** Steam-first desktop (Mac/Windows/Linux + Steam Deck via Linux).
- **Runtime LLM calls anywhere.** All LLM output is bake-time RON (Tracery grammars, content packs) reviewed and committed. "AI scouting recommendations" and dynamic press questions are reframed as biased-scout disagreement and Tracery-templated responses — the deliberate alternative, not a missing feature.
- **ML / adaptive opponent AI.** Opponent behavior is deterministic BT + archetype FSM. No learned models.
- **In-game / live-save attribute editors and external memory editors (FMRTE-style).** Ruled out by the IPC contract ("UI never drives canonical state") and the determinism contract (pinned BLAKE3 hashes, Q32.32). Mid-save mutation would corrupt the canonical hash. Career integrity is non-negotiable — "what happened, happened" is the point of pillar 2.

**Genre features that are out-of-scope *until a prerequisite world-system exists*, not permanently rejected** (so the reader does not confuse "deferred" with "never"): international management and continental cups (need 2+ seeded nations — post-EA), work permits / foreign-EU quotas (no licensed nations; a fantasy cross-region-eligibility equivalent is a future design call), FFP/PSR (no licensing body; a fantasy governance equivalent is planned once finances land), and women's football (gender is not yet a modeled world dimension). Family/private-life mechanics (We Are Football style) are not a FW pillar and are out of scope for EA and likely 1.0.
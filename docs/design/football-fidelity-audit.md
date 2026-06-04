# Football-fidelity audit — what makes football behave like football, and what we're missing

**Status:** AUDIT (2026-06-04). A grounded gap analysis of the match engine against real football,
commissioned by the owner ("figure out if we are missing something… football behaves like it does
because of all the complex parts"). **Method:** 7 parallel agents, each auditing one football-fidelity
domain against the *actual code + roadmap* (not generic football), classifying every mechanism as HAVE /
PLANNED / MISSING with a believability-impact tag. **Owner:** systems-designer (steering) + the owner
(scope forks). This is a steering document — it does not change the contract; it informs the next
roadmap round.

---

## §1. The one-sentence verdict

**The attribute *model* is rich (a 55-field FM-comparable taxonomy) and genuinely wired to per-tick
*decisions* — but the *resolution* layer (do actions contest?) and the *dynamics* layer (does anything
change within or across a match?) are thin or absent. The match is essentially static-per-input: the
89th minute plays like the 1st.**

That single property — nothing tires, swings, varies, or reacts to the scoreline — is the largest
believability gap, and it recurs across five of the seven domains. The good news: most of the fields and
substrate already exist (`PlayerCondition`, `xT`, `pitch_control`, the offside event, `heading`/`jumping`,
the tactic FSM, 16 authored formations) — they're declared-but-unconsumed. A lot of this is wire-up, not
green-field.

**Three honest reframes of the owner's list:**
- **Already PLANNED (don't re-litigate):** offside, fouls/cards/free-kicks/penalties, restart timing,
  build-up/passing geometry, the gene→attribute compiler, differentiated rosters. These are on the
  roadmap (FUN-TS2/3/4, FUN-LAW1-4, T4.5-E0/E1, FUN-DR) — see §3-B.
- **Built-but-UNUSED (cheap wire-up wins):** `xT` + `pitch_control` (zero attacking call sites),
  `PlayerCondition` form/morale/match_fitness (inert in-match), the `Offside` event (defined, never
  emitted), `heading`/`jumping_reach` (dead attributes), `consistency` (dead), and the 16 authored
  formations (**every team plays a hardcoded 4-3-3**). See §3-C.
- **Genuinely MISSING + UNPLANNED (the real gaps):** the static-match cluster, contested actions,
  footedness, form/injuries, formation-wiring, live manager identity. See §3-A — this is where the
  owner's steering matters most.

---

## §2. The prioritized gap register

Ranked by believability impact across all domains. **Status:** `UNPLANNED` (no row/doc), `DEFERRED-COMMENT`
(named in a doc/comment, no row), `PLANNED` (has a row/doc), `BUILT-UNUSED` (code exists, not consumed).

### TIER A — the biggest holes, mostly UNPLANNED (this is the steering list)

| # | Gap | Impact | Status | Where it fits |
|---|---|---|---|---|
| A1 | **In-match fatigue / stamina depletion** — stamina is a static attribute; nothing drains, so no tired legs, no late-game opening up, no reason subs matter. `PlayerCondition.match_fitness` exists, inert. | HIGH | UNPLANNED (field exists) | New "living match" row; per-tick Q32 drain scaled by activity, multiplies physical/technical utility |
| A2 | **Score + clock reach the tactic FSM** — a side 3-0 down at 85' plays identically to 0-0 at 5'. This is the *causal* engine behind the M4/M5/M6/M7 drama the sweep only *measures*. | HIGH | DEFERRED-COMMENT (T1-2b-iii) | FUN-TS4 is the hook; add `score_lead`/`ticks_remaining` FSM arms |
| A3 | **In-match team momentum** — a goal/save/red should tilt the next ~10 min. DESIGN_DOC §6 UI *already advertises* a momentum number with no system behind it. | HIGH | UNPLANNED | `#[serde(skip)]` momentum sidecar (exactly how FUN-TS1 added `TeamShape`) |
| A4 | **Substitutions + fresh-legs** — no bench, no in-match subs, no manager lever. Depends on A1 to mean anything. | HIGH | UNPLANNED | New row; seeded manager-policy sub at fatigue/score/clock thresholds |
| A5 | **Passes can fail** — `T1_PASS_COMPLETED = true` is hardcoded; possession only changes via tackle/shot. No interceptions, no misplaced passes → midfield can't be a contest. | HIGH | UNPLANNED stub | FUN-TS3-adjacent; Q32 completion prob = passer × lane-openness (pitch_control) × receiver pressure |
| A6 | **Authored formations drive placement** — `archetype.formation` is read *nowhere* in the sim; **all 16 archetypes play as a hardcoded 4-3-3**. Formation families + most manager identity are invisible in-match today. | HIGH | UNPLANNED | Near-pure data-wiring at `initial_with_content`; multi-pin rebaseline |
| A7 | **Form / "bad days" / consistency variance** — every player performs identically every match given identical inputs; `consistency` (a top real-football differentiator) is dead. Directly serves FUN-AS anti-scripting. | HIGH | UNPLANNED (fields exist) | Pre-match seeded Q32 form draw, width set by `consistency`; one utility multiplier |
| A8 | **Injuries** — no in-match knock, match-ending, or lay-off ever fires. `InjuryLongTerm` ledger class exists with no emitter. Feeds Pillars 2/3. | HIGH | UNPLANNED | Seeded per-tick roll weighted by `injury_proneness` × contact × fatigue → MatchEvent + forced sub |
| A9 | **Dribble as a 1v1 contest** — `Dribble` just snaps the ball to feet + advances 8m; beating a man is invisible. Wingers/flair/the `dribbling` attribute mean almost nothing in resolved play. | HIGH | UNPLANNED | 1v1 roll when a dribbler enters a defender's radius (dribbling/agility/flair vs marking/positioning) |
| A10 | **Aerial duels + contested crosses + first-touch failure** — `heading`/`jumping_reach` are read by *nothing*; crosses teleport to the nearest body; first touch never fails. Three inert attributes, no contested-ball texture. | HIGH | UNPLANNED | Aerial contest on high balls (heading×jumping×strength); first-touch roll on receipt under pressure |
| A11 | **Marking assignment + runner-tracking + rest-defense** — when defending it's 100% zonal hold; `utility_mark_player` is dead code; nobody tracks a specific runner; on possession loss the whole team had zero retained structure (clean counters). | HIGH | UNPLANNED (FUN-TS adjacent) | marker→runner assignment sidecar (same pattern as the shipped `compute_press`); retain N holders in possession |
| A12 | **Live manager identity** — `risk_appetite`/`possession_preference` are stored in content but never loaded into the sim; manager character beyond 3 shape knobs is inert. | HIGH | UNPLANNED | Load `ManagerArchetype` into `MatchState`; feed risk into pass/shot-risk utility |
| A13 | **Scoreline/stakes-responsive composure** — composure is a static gate; high-stakes matches feel like friendlies; no big-game bottling. | HIGH | DESIGNED-NOT-BUILT | Pass `(score_diff, tick, stakes)` into the existing `personality_bias` composure tilt |
| A14 | **Footedness (L/R/both) + weak-foot** — the `left_foot` gene exists in content; the sim has *zero* foot references. No cut-in side, no weak-foot penalty. | MED-HIGH | UNPLANNED (gene exists) | `preferred_foot` + `weak_foot` Q32; penalise weak-side shot/pass/dribble; drives cut-in vs overlap |
| A15 | **Runtime role-fit penalty** — a CB at RW is no worse on the day; role-affinity is bake-time-only. | MED-HIGH | UNPLANNED (table exists) | Per-player positional-fit multiplier vs assigned slot from the existing role-affinity table |

### TIER B — already PLANNED (confirm coverage, don't re-litigate)

| Gap | Where |
|---|---|
| Offside *emission* (event defined, never pushed) | **FUN-TS2 (in flight)** — this is exactly its current task |
| Coordinated press as a line, cover-shadow, press triggers, decouple high-line/high-press | FUN-TS2 + match-realism-reference §3 |
| Build-up phases, support angles, width-in-possession, xT/pitch-control-driven pass selection | FUN-TS3 (tactical-shape.md §Slice 4) |
| Tactic FSM as shape driver (the hook A2 plugs into) | FUN-TS4 |
| Fouls → free kicks, cards/dismissals (emergent 10-man), penalties, restart timing, referee-strictness coefficient, advantage gate | FUN-LAW1-4 (laws-of-the-game.md) |
| Gene→attribute compiler (turns height/footedness/aging genes into the 55 attributes) | T4.5-E0 |
| Differentiated rosters (retire the flat-0.5 mirror substrate the engine is currently tuned on) | FUN-DR + T4.5-E1 |
| Age/decline curves, breakthrough-driven growth | progression.md / Pillar 3 (breakthroughs DONE; aging has no row) |
| Tactical cohesion ramp, morale system, captain influence, set-piece routines, partnerships/"telepathy", personality-conflict chemistry | **feature-backlog.md (post-EA, NO MASTER_PLAN rows)** |

### TIER C — built-but-UNUSED (cheap wire-ups, feed Tier A/B)

`xT` + `pitch_control` (zero attacking call sites → FUN-TS3) · `PlayerCondition` form/morale/match_fitness
(inert → A1/A7) · `heading`/`jumping_reach` (dead → A10) · `Offside` event (defined, never emitted →
FUN-TS2) · `consistency` (dead → A7) · 16 authored formations (all play 4-3-3 → A6) · `ManagerArchetype`
risk/possession (inert → A12) · role-affinity table (bake-time only → A15).

---

## §3. Per-domain one-liners (detail in the audit transcript)

- **Defensive phase:** held zonal block is REAL (FUN-TS1) + press roles in flight (FUN-TS2). Missing: offside *emission*, man-marking/runner-tracking (vestigial), rest-defense/transition shape, cover-shadow geometry, ball-side shuffle.
- **Attacking + passing:** 7 on-ball considerations are real, but pass targets are "kick 10m forward, nearest body receives," passes always complete, and `xT`/`pitch_control` are built-but-unused. FUN-TS3 is the planned fix; through-balls/switches/cut-backs/combination-play are not yet scoped even there.
- **On-ball skills:** the attribute MODEL is complete; the RESOLUTION is the gap. Only shooting is well-modelled. Dribble/first-touch/aerial/crossing/hold-up are decision-only stubs or absent; footedness is entirely missing from the sim.
- **Attributes + physical + condition:** 55-field model, genuinely wired to decisions — but the *dynamic* side (fatigue, injury, age, form, footedness, height-as-duel-input) is absent, and the live substrate flattens everyone to 0.5 (mirror teams) until T4.5-E0/E1 + FUN-DR.
- **Psychology + mentality:** the 21-multiplier `personality_bias` system is the strongest piece (8 of 14 axes wired). Missing: in-match momentum, scoreline-responsive composure, inert `PlayerCondition`, pair cohesion, big-game bottling, leadership.
- **Match-state dynamics:** the tactic FSM + team shape are the vehicle, but **scoreline and clock never reach the FSM**; stamina never drains; half-time is a stub; no subs; no home advantage. Referee variation is correctly PLANNED (FUN-LAW3).
- **Set-pieces + rules + teamwork:** rules are well-planned (FUN-LAW). Genuinely undocumented: **archetype formations not wired (all 4-3-3)**, runtime role-fit penalty, live manager identity. Set-piece *routines* + defensive organisation + specialist takers are post-EA backlog with no rows.

---

## §4. Recommended sequencing + the owner forks

**The believability-first thesis (already adopted) says: build the causes before tuning the output, and
before going wide.** This audit sharpens what "the causes" are. Recommended order after the in-flight
FUN-TS2/TS3/TS4 + FUN-LAW work:

1. **A "living match" cluster** (A1 fatigue + A2 score-state + A3 momentum + A4 subs + half-time) — the
   single highest-leverage believability gain, mostly wire-up of existing fields, and the *causal* source
   of the drama the sweep currently only measures. **Recommend pulling into the EA believability push.**
2. **A "contested ball" cluster** (A5 pass-can-fail + A9 dribble-1v1 + A10 aerial/first-touch) — converts
   the match from rolled outcomes to earned ones; the contested-ball texture that reads as football.
3. **A6 formation-wiring** — near-pure data-wiring, unlocks formation families + manager identity; cheap,
   high-value, do it early.
4. **A7 form + A8 injuries + A14 footedness** — partly gated on T4.5-E0 (gene compiler) for
   differentiation; fatigue/form/injury are match-engine and can lead.

**Owner forks (NOT decided here — these need your call):**
- **EA scope.** A lot of the above is currently post-EA backlog. The believability-first logic argues the
  "living match" + "contested ball" clusters belong *in* the EA match, not after it. How much do we pull
  forward vs hold the line at FUN-TS/FUN-LAW and ship a thinner-but-honest EA match? (Ties to the
  already-flagged EA management-depth fork.)
- **New track vs fold-in.** Do these become a new `FUN-MATCH` tier, or fold into FUN-TS4 (score-state) +
  FUN-TS3 (pass-fail) + FUN-LAW (injuries-as-events)?
- **The mirror-substrate dependency.** Form/footedness/role-fit only *show* once T4.5-E0 (gene compiler)
  + FUN-DR retire the flat-0.5 teams. The audit reinforces that those are believability-critical, not
  just world-scale plumbing — worth re-weighting their priority.

## Cross-references
- `docs/MASTER_PLAN.md` Tier F (FUN-TS/FUN-LAW/FUN-DR/FUN-FP) — the planned work this audit cross-checks.
- `docs/design/{tactical-shape,laws-of-the-game,match-realism-reference,feature-backlog,progression}.md` — the domain specs.
- `docs/DESIGN_DOC.md` Pillar 0 (believable football) — the standard this audits against.
- Full 7-domain transcript: the audit workflow output (file/line citations per gap).

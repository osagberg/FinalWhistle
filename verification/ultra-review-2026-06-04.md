# Independent ultra-review — Final Whistle (whole project) — 2026-06-04

**Reviewer:** independent adversarial pass (Opus 4.8), no stake in the existing plan.
**Method:** 8 dimension investigators read the live tree read-only, then every P0/P1 finding
was handed to a separate skeptic instructed to *refute* it against the cited code before it
survived. 25 agents, ~2.2M tokens. Findings are tagged `[CONFIRMED]` (a reviewer personally
read the cited lines) or `[HYPOTHESIS]`, with `file:line` / `doc §` evidence. Several
investigator P0s were *downgraded* by their verifiers — those corrections are reflected here.
**Tree state:** judged against the **uncommitted working tree** (the in-flight FUN-0b+c
"watchable-match Slice A" work is present and materially changes the picture — see below).
This is advisory; the human owns every decision. No working-tree files were edited.

---

## Read this first (the one paragraph)

Final Whistle is architecturally sound, genuinely deterministic, and — as of the uncommitted
working tree — has just broken open its worst problem: the bimodal 43-43 match is gone and full
5400-tick matches now produce believable **0–3 goal scorelines** (verified by running the
in-flight envelope test). The determinism bet is earning its keep and is **not** the scale risk.
But the project carries one structural blind spot that is the root cause of how a 43-43 shipped
through four phase gates undetected: **"believable football" is not a pillar, not a
non-negotiable, and not a standing CI gate** — and the lone regression guard that existed was run
on 600-tick fixtures and mirror-image teams of 22 attribute-identical players, inputs that
physically cannot exhibit the failure. Across a 25-agent adversarial sweep there are **zero P0
correctness defects** — the real risks are structural, of sequencing, and of focus. The single
highest-leverage move is to make believable football a first-class *standing* gate (a full-match
goal-distribution check over *differentiated* rosters) **before** T4.5 scales the world to ~96
clubs and ~2000 players — because every still-unsolved problem (the unbuilt match fast-path, the
un-tuned drama, the thin scouting, the near-absent management depth) gets 5–20× more expensive to
fix once the world is wide. The biggest threat to this game is not a bug; it is shipping a
beautiful, unique, *wide* world wrapped around a match you cannot yet fully trust and a management
layer with almost no decisions.

---

## Headline verdict — the 5 things that matter most

1. **No "pillar zero."** The five pillars (procedural world / memory / breakthrough / scouting /
   signature) and the ten non-negotiables protect *determinism and content policy* with clippy
   lints + a required CI check. They protect *believability* with nothing co-equal — only a few
   narrow per-property proptests and a runaway guard whose seed set was deliberately chosen to
   exclude the high-variance outliers. That is exactly why a mean-38.85-goals/match engine passed
   T0→T4. **`[CONFIRMED]`**

2. **The match is the long pole, and it is the least-finished thing the entire "beats FM"
   comparison rests on.** No influence-map module, no xT-into-selection, no real off-ball
   movement, no restart/offside/foul/card mechanics (all grep-confirmed absent). The in-flight
   work is good and broke open the goal-rate, but the possession-lock root cause is explicitly
   deferred and the realism floor still has depth gaps. **`[CONFIRMED]`**

3. **Scaling the world is the next big phase (T4.5) and is the wrong thing to do next.** Doing
   fun (FUN-1..4) and at least one management loop on the cheap 20-club substrate first is faster
   and de-risks everything. This is my strongest disagreement, argued below.

4. **The match fast-path that the whole scalability story depends on does not exist.**
   `advance_week_inner` full-sims every fixture; the designed two-tier (player-club real, AI
   cheap-procgen) path is unbuilt, and there is no perf benchmark in CI at all. Harmless at 20
   clubs, ~22 s/season-advance at pyramid scale without it. **`[CONFIRMED]`**

5. **Harness-gravity is real but self-aware.** 9 of the last 10 post-T4 commits are tooling/docs;
   exactly one touched the engine. The tools *did* find the bugs and the team wrote its own
   guardrail — the problem is ratio and *stopping*, not direction. **`[CONFIRMED]`**

> **On P0:** after adversarial verification there are **no P0 *correctness* defects**. The one
> item I rank P0 is *structural* (the missing believability gate) because it is the meta-cause of
> the headline product defect and it will re-bite at T4.5. I am transparent that it aggregates two
> verifier-confirmed P1s; I elevate it on priority, not on code-severity.

---

## P0 — Critical (structural; settle before T4.5 world-scale)

### P0-1 · Believable football is neither a pillar nor a standing gate — and the guard that exists was tuned around the very failure that shipped · `[CONFIRMED]`
**Evidence (aggregates two verified P1s):**
- `DESIGN_DOC.md §3` names five pillars; **none** is "the match behaves like recognisable
  football." `§2`'s ten non-negotiables are all determinism + content policy. `§11.1` asserts
  "fidelity competitive with FM's match engine" with **no acceptance criterion attached**. A grep
  for `believ|realis|watchable|plausib` finds the realism framing only in the TF track (added
  2026-06-04) and `drama-model.md`, which itself says "calibrate once the engine produces
  watchable football" — i.e. deferred.
- The Determinism Gate CI required-check is "Tier-A scope: smoke seed only (60-tick fixture)"
  (`determinism-gate.yml:9`); pinned hashes are 60-tick + 600-tick only (`canonical_hash.rs`
  PINNED table). At committed HEAD the only goal-envelope assertion ran over **600 ticks, which
  now scores 0 goals on every seed** — vacuous as a goal-rate check (`canonical_hash.rs:1029-1046`
  documents this). The 5-seed full-match runaway guard added in-flight uses a seed set chosen to
  top out at 3 goals, **excluding the high-variance outliers** — the exact mechanism by which the
  bimodal 43-43 (mean 38.85 vs 2.3-3.2, `MASTER_PLAN:402`) passed every phase gate T0–T4.

**Why it matters:** all five differentiators are emotionally inert if the match they sit inside
reads as nonsense — they are *parasitic* on believable football. The project defends its
determinism contract with lints and a blocking CI gate, and defends the thing that actually makes
the game good with an assumption. That is the single biggest mismatch between what the docs guard
and what ships a fun game.

**Fix:** (a) add "Pillar 0 — believable football" to `DESIGN_DOC §3`, stating the five pillars are
parasitic on it; (b) promote the watchable-match criteria (M1 goals/match ∈ [2.3,3.2], M2 timing
spread, zero glitch-flags over a 5-seed **full 5400-tick** sweep on **differentiated** rosters)
to a *standing* regression gate, not a one-time milestone; (c) wire a full-match
goal-*distribution* check (not just the collapse/runaway ≤8 guard) into CI so calibration cannot
silently regress. The drama_sweep `guards::M1_*` constants already exist — the remaining work is
to run them over fixed seeds in CI.

---

## P1 — High

### P1-1 · The designed two-tier match fast-path is not implemented; every fixture full-sims, and there is no perf benchmark in CI · `[CONFIRMED]`
`advance_week_inner` (`commands.rs:438-503`) calls `season::play_one_match` for **every** fixture
with no player-club branch; `play_one_match` (`season.rs:145-174`) is unconditionally a full
22-player tick loop. Grep for `poisson|dixon|simulate_cheap|fast_path` → zero. `career-roster-
layer.md §6` and `MASTER_PLAN` T4.5-I/T5-5 confirm the cheap-procgen reduction (Decision 3) is a
*future TODO*, not code. **No criterion bench exists** (zero `criterion` deps, zero `benches/`);
the only sim-cost gate is `full_season_perf_under_30s` (`season_commands_test.rs:615`) — `#[ignore]`d
(never runs in CI) and pointed at the 600-tick proxy, **9× shorter** than a real match.
**Why it matters:** harmless at 20 clubs; at the shipped ~96-club / ~1800-match pyramid with full
matches this is **~22 s per single season advance** on the synchronous IPC thread (estimate; the
12 ms/match figure could not be independently re-confirmed — the measurement file was a throwaway).
With the fast-path it drops to ~0.46 s/season. The whole scalability story rests on an unbuilt
path, and the absence of any bench already let a 4× cost-model error sit in a ratified design doc.
**Fix:** build `simulate_fixture_cheap(seed, home_rating, away_rating)` (seeded Poisson/Dixon-Coles
from squad ratings) as a **gating dependency of T4.5-G**, not a T5 optimization. Add a ~30-line
`benches/tick.rs` and un-ignore a budget test pointed at the real 5400-tick length.

### P1-2 · No anti-scripting / unpredictability guard — aggregate drama bands are satisfiable by a deterministic script · `[CONFIRMED]`
Every drama metric in `bin/drama_sweep/drama.rs` (M1-M8) is a per-match scalar reduced to a corpus
mean/rate; there is no conditional-outcome entropy, no who-trailed-vs-who-scored anti-correlation,
no home/away or seed-independence symmetry test. `inspect_frames.rs` covers only physical
coherence. **Why it matters:** a rate target is textbook Goodhart bait — a rule like "whoever
trails by 1 at 0.85×end always equalises" drives M5 late-winner and M7 nervy rates straight into
band while being completely fake football, and nothing flags it. `fun-evaluation-harness.md`
Component B defines Commentary/Callback/Match-motion rubrics but **no "gripping / was-it-scripted"
dimension**. FUN-1 (the literal next task) tunes M3-M7 toward these bands with no guard.
**Fix:** before FUN-1, add ≥1 patterning metric to `drama.rs` (conditional late-winner entropy
and/or home/away drama symmetry) and a "predictable/scripted = fail" dimension to the gripping
rubric.

### P1-3 · Drama-target ceilings are written but dead-coded — over-delivery of drama is invisible · `[CONFIRMED]`
The `targets` module (`drama_sweep/main.rs:75-99`) carries two-sided M3-M7 bands but is
`#[allow(dead_code)]`; `all_guards_pass` (line 571) is gated solely by the three realism guards
(M1/M2/M8); drama rates print as "informational — no pass/fail yet" (line 716). So a runaway 100%
nervy-finish rate trips nothing. Dual-bounding protects realism only when enforced; today the
ceiling exists only on paper. **Fix:** drop `#[allow(dead_code)]` and emit PASS/OVER/UNDER per
drama target (warn-only pre-FUN-1) so tuning cannot silently overshoot.

### P1-4 · Goal-rate calibration ran exclusively on mirror teams of 22 attribute-identical players — the realism floor has no team-quality or shot-quality depth · `[CONFIRMED]` (verifier-downgraded from P0)
The live drama_sweep corpus uses both archetypes = `DEFAULT_ARCHETYPE_ID` and **every** player at
`PlayerAttributes::mid_range_baseline()` (`main.rs:325-334`, `lib.rs initial_with_content`
709-770 never assigns attributes). So the best-achieved M1=3.15 was tuned with **zero team-quality
differential and zero per-player spread**; `shooter_quality` (BETA_6) is fed a constant 0.5; S2
(upset rate) and S3 (table variance) are unmeasurable. (Correction vs the raw finding: `calibrate.rs`
*does* sweep archetype pairs, but its fitted betas are explicitly not applied to live shooting, and
it too uses mid-baseline players.) **Why it matters:** realism is supposed to be the master
constraint, but the goal-rate was certified on a degenerate input with no transfer guarantee to
real differentiated rosters — better teams may not beat worse ones. **Fix:** before declaring
watchable, run drama_sweep across seed-varied attributes and non-identical archetype pairs; add a
guard that the higher-rated side wins more than chance (S2 ∈ 25-40%, not the ~50% a mirror forces).

### P1-5 · `last_scout_report` is dropped on save/load — scouted players read "NotYetObserved" after every reload (NEW correctness bug) · `[CONFIRMED]`
`SavedPlayerInstance` persists `observation_count` but not `last_scout_report` (`fw-save/src/lib.rs:
254-281`); `load_career_inner` (`commands.rs:2284-2290`) never restores it; `get_scout_report_inner`
(`commands.rs:943-946`) returns `IpcError::NotYetObserved` whenever it is `None`. Result: a reloaded
roster has `observation_count > 0` with `last_scout_report == None` — a state that cannot occur in
live play — and previously-scouted players show as never-observed. The `fw-save` doc-comment claims
the field is "re-derived at load from the career seed," but **no load-path code calls
`observe_player`**. Self-heals next match-day for the starting XI, but persists for bench/reserve
players. Untested. **Why it matters:** breaks the #1 differentiator (careers remember) and pillar 4
(scouting) across a save/load — the most basic player action. **Fix:** re-derive on load via
`observe_player(scout, bio, career_seed, observation_count - 1, player_id)` (the data is
deterministically recoverable from already-persisted state), or persist the field; add a round-trip
test that `observation_count > 0 ⇒ last_scout_report.is_some()`.

### P1-6 · Stale `last_shot_xg` can defeat the GK save gate on non-shot goals (NEW correctness bug, latent) · `[CONFIRMED]`
`last_shot_xg[slot]` is written only by `AttemptShot` (`dispatch.rs:888`) and cleared only on a save
(`lib.rs:2010`) — never on a non-shot ball touch (Pass/Cross/Dribble/GkDistribute) or after a goal
stands. The save gate keys `xg_score` on `last_shot_xg[last_touched_by]` (`lib.rs:2006`), and its own
comment says dribbled-in/deflected/scramble goals must **not** be save situations. So a player who
shot (xG cached), then later touches a ball that crosses the line via a dribble/deflection, makes the
save model fire on a non-shot goal — defeating the documented invariant. Reproducible (canonical
field), not random; current smoke seeds just don't hit the sequence. **Fix:** clear `last_shot_xg` in
non-shot ball-touch arms and after a goal stands, or stamp `last_shot_tick` and require recency.

### P1-7 · No off-pitch management depth is on the EA-critical path — EA bets entirely on match-feel + the 5 narrative pillars · `[CONFIRMED]`
`feature-backlog.md §3` lists as **missing with no roadmap row**: transfer market UI ("the single
biggest table-stakes hole"), training ("no training system exists at all"), finances, transfer
windows, morale driver ("field exists, never mutated" — confirmed: `morale` init to `ONE` and never
reassigned), form decay, board interactions, individual instructions. The only EA-path breadth rows
(T4.5/T5) are the pyramid, promo/relegation + one cup, the 2000-player compiler, and the content
bake. `DESIGN_DOC §8` lists "Transfer window + contracts (basic)" as Phase 1-2 *intent*, but **no
MASTER_PLAN row schedules it** — intent and plan have diverged. **Why it matters:** as scoped, EA
ships pick-XI + sim/watch + read scout bands + accept/reject renewals + see breakthroughs/callbacks.
Against a self-declared "match or exceed FM on depth" positioning, a buyer who finds no transfer
market, no training, no finances, and a single non-disagreeing scout will bounce regardless of how
good the match looks. **Fix:** make the bet explicit on the store page *or* pull one cheap
high-leverage loop (tactical-familiarity/cohesion ramp, or a minimal transfer-market + contract-
expiry spine) onto the EA path. At minimum schedule **one** table-stakes depth row into T4.5/T5.

### P1-8 · Season metrics (S1-S4) unimplemented, and the season sim runs 600 ticks vs the 5400-tick match calibration — two incompatible goal regimes · `[CONFIRMED]`
drama_sweep is match-only (S1-S4 deferred to FUN-4, TODO). The live season runs every match at
`SEASON_MATCH_TICK_BUDGET=600` (`season.rs:45`), which deliberately never reaches FullTime and scores
~0 goals, while M1 was calibrated at `FULL_MATCH_TICKS=5400`. `shot-model.md:718-726` itself notes the
600-tick rate is ~18× incompatible with the full-match rate. **S3 (table-spread variance) is classified
a REALISM GUARD yet does not run**, so a whole class of season-believability failure (every season
identical, or quality irrelevant) is unguarded — and today's AI league tables are produced by 0-goal
stub matches. **Fix:** at FUN-4, run season matches at the calibrated tick budget (or document the
mapping), implement S3 as a hard guard, and pin S2's squad-rating composite.

### P1-9 · Harness-gravity: 9 of 10 post-T4 commits are tooling/docs; ~4,557 tooling LoC + ~1,500 doc LoC built before the one engine fix · `[CONFIRMED]`
Of the 10 commits since T4 close, exactly one (FUN-0, ~468 LoC) touched the engine; the rest are an
HTTP bridge, a 7-detector frame analyzer, a contact-sheet renderer, a drama-sweep with A/B baseline,
and ~1,500 lines of design docs. `calibrate.rs` (716 LoC, built at T2-1d) is a *prior* stranded tool
whose fitted output was never wired in ("decorative") — evidence this is a recurring pattern, not a
moment. The team wrote the correct guardrail (`fun-evaluation-harness.md:124`: "build the next harness
piece ONLY when it unblocks actual fun-progress") but the commit log shows it hasn't been obeyed.
**Why it matters:** the inspection machine is growing faster than the thing inspected; the watchable
match the whole pivot exists to fix is still in-flight. **Fix:** freeze the harness at v1; block every
remaining DX/FUN-H tooling row and every harness-evolution rung (CI gates, archetype probe,
auto-tuner, director mode) on FUN-0 reaching DONE; run the next ~6 commits entirely inside
`fw-match-sim`. Make "no new TF tooling row until FUN-0 DONE" a hard rule at the top of Tier F.

### P1-10 · "Match/beat FM" framing is honest, but the shippability risk is the enormous combined scope with the proxy-stubbed engine as the long pole · `[CONFIRMED]`
The framing is grounded and layer-separated (`DECISIONS:107` calibrates HOW-football-is-played to
public aggregate bands; `CLAUDE.md:15` calls FM "a low watermark, not a ceiling") — *not* realism-axis
hubris, and the determinism architecture is a genuine enabler. The risk is the surface area: an
FM-class engine that is the least-finished part (still proxy selection in most paths; no influence
maps, no xT-into-selection, no off-ball movement, no restarts/fouls/offside — all grep-confirmed)
**plus** 5 novel pillars **plus** procedural world **plus** a 2000-player bake, under solo-dev+Claude.
(Staleness note: the in-flight tree already rewired `utility_shoot` to the real `xg_utility` logistic;
the bimodal possession-lock is still deferred per `shot-model.md:27`.) **Fix:** treat the engine as the
long pole; land possession-lock + the influence-map/xG-into-selection critical path **before** widening
content/world scope, and gate each calibration slice with the full-match envelope.

---

## P2 — Medium (worth doing; not urgent)

- **Scouting has no convergence over observations** `[CONFIRMED]` (verifier-downgraded P1→P2). Band
  width (0.12) and noise (0.10) are fixed constants, never functions of `observation_count`; each
  match-day **overwrites** `last_scout_report` with a fresh resample (`season.rs:757-764`). So
  scouting a player 10× gives 10 equally-foggy guesses with a jittering centre — "truth emerges over
  seasons" (`DESIGN_DOC §3 Pillar 4`) is not implemented. *Documented-deferred* (track records behind
  the Month-4 gate; single-scout is the EA floor), so the genuine residual is a **UX dissonance**: the
  UI shows a rising `observation_count` integer implying knowledge the model never delivers. Fix when
  track-records lands: `effective_hw = base_hw / sqrt(1 + observation_count)` + a running mean, with a
  proptest for monotonic narrowing.
- **Per-category scout band reads "a confident read" for nearly every player, and extremes read
  *firmer*** `[CONFIRMED]`. Constant 0.24 width → `Confident` for any mid-range category; edge-clamping
  makes clearly-elite/poor categories read *more* certain — backwards from the intended "uncertainty at
  the elite tail." The displayed fog carries almost no information.
- **Scout noise is flat, not tail-concentrated** `[CONFIRMED]`. Uniform ±0.10 across the ability range;
  adds the most confusion in the crowded middle and the least where the design wants drama
  (world-beater-or-merely-elite). Missed-design, not a correctness bug.
- **Decision-gate vs save-model xG use different pressure features** `[CONFIRMED]`. The shoot gate uses
  a composure proxy; the stored save xG uses a spatial proxy (`dispatch.rs:246-280`) — so the xG driving
  the save is not the xG that authorised the shot. A calibration trap while drama_sweep is active.
- **`XG_SHOOT_THRESHOLD` comment rot** `[CONFIRMED]`. The doc block argues 0.020 (raw bits 85,899,346)
  but the const is `from_raw(408_021_893) ≈ 0.095`. The single most-tuned lever has a lying comment.
- **`body_shield` test still vacuous** `[CONFIRMED]`. Asserts only `>0 && <1`, not the exact product —
  QA-T4H item 3 was rowed but **not applied** (siblings `long_range_strike`/`diagonal_switch` use the
  exact form). A constant-0.5 mutation survives.
- **Watermark comment contradicts code** `[CONFIRMED]`. `commands.rs:1403-1406` says appended
  BreakthroughMoment events are "PAST the watermark" but the code sets `watermark = ledger.len()` after
  appending. Behaviour is safe; the comment invites a "fix" that re-introduces the re-fire bug.
- **600-tick season budget produces ~0-goal AI standings** `[CONFIRMED, in-flight]`. Until T5-5b raises
  it, league tables are noise — and raising it to 5400 makes the fast-path (P1-1) *mandatory*, not
  optional. Sequence them together.
- **Per-tick `build_trigger_table` allocation** `[CONFIRMED]`. A fresh 9-entry BTreeMap is built every
  tick in `dispatch_tick` (`dispatch.rs:555`) — ~9.7M allocations/season at scale, pure waste on the
  hottest path. Hoist to a `OnceLock`/`match`.
- **Breakthrough loop is O(players × new_events) per season** `[CONFIRMED]`. Fine at 440 players;
  unmeasured at ~2000. Group `new_events` by subject once before the player loop.
- **Cross-OS bit-identical hashing on every commit has never caught a real divergence** `[CONFIRMED]`.
  All 17 rebaselines are triggers #1-3 (schema/encoder/behavior), never #4 (cross-OS). The primitives
  are portable-by-construction; the per-commit tri-OS cadence is the over-engineered tail (already
  forced a Windows-timeout workaround). Keep macOS canonical-hash per-commit; path-filter / nightly the
  tri-OS agreement (per external-review C1). Do **not** drop the determinism architecture.
- **Realism anchors lack provenance/firmness tags** `[CONFIRMED]`. "Upset 28-34%", "late-winner 8-14%"
  are bare assertions; label which are hard real-world regularities (xG ~0.10/shot) vs soft design
  targets a high-variance fantasy world may legitimately push, so tuning doesn't treat a taste call as a
  physics constant.
- **Two untracked `_measure_*.rs` throwaway test files** `[CONFIRMED]`. Self-labelled throwaway; one uses
  `Instant::now()` and would look like a clock leak to a future reviewer. Delete before committing.

---

## Top 5 highest-leverage changes

1. **Make believable football a standing gate (P0-1).** Add Pillar 0 + a full-match,
   differentiated-roster goal-distribution CI check using the existing `drama_sweep` M1 guards. This is
   the one change that would have caught the 43-43 and prevents the next regression.
2. **Build the AI-fixture fast-path before T4.5-G (P1-1).** A seeded Poisson/Dixon-Coles scoreline for
   non-player fixtures + a 30-line bench. Turns ~22 s/season into ~0.5 s and unblocks the whole world-
   scale phase honestly.
3. **Do FUN-1..4 (+ one management loop) on the 20-club world before scaling (my strongest
   disagreement, below).** Fun and depth iterate 5× faster on the small substrate and are
   substrate-independent.
4. **Add an anti-scripting metric + a "scripted/predictable" judge dimension before FUN-1 tuning
   (P1-2/P1-3).** Cheap insurance against tuning into metric-gamed, fake-feeling drama.
5. **Fix the two new correctness bugs (P1-5 scout-report-lost-on-load, P1-6 stale-xg) and freeze the
   harness at v1 (P1-9).** Small, concrete, and they protect the differentiators + redirect velocity to
   the match.

---

## My single strongest disagreement (argued hard)

**Do not let T4.5 "World Scale + Content Bake" be the next major phase. Finish *fun and depth* on the
small 20-club world first — a believable + dramatic match (FUN-0 through FUN-4) **and** at least one
real off-pitch management loop — and only then scale to 96 clubs and 2000 players.**

The roadmap, even after the (correct, praised) fun-pivot that pulled watchable-match forward, still
treats FUN-1..4 as a "parallel track alongside T4.5" with no committed cadence, and puts *all* off-pitch
management depth post-EA. That sequence inverts the actual dependency structure of this game.

Here is the argument. Everything that makes Final Whistle *Final Whistle* — the five pillars — is
parasitic on two things the game does not yet have: a match you can trust, and decisions worth making.
Scaling the world adds neither. It multiplies. A 96-club pyramid with an un-tuned drama model produces
96 clubs' worth of un-tuned drama. A 2000-player bake wrapped around a single non-disagreeing,
non-converging scout produces 2000 players you can't meaningfully scout. The unbuilt fast-path (P1-1)
becomes a hard blocker the moment fixture count goes 5× and match length goes 9×. And the validation
loop you depend on for fun gets *slower* exactly when you've made the world bigger — a 1000-seed
season sweep over a 6-tier pyramid is far costlier to iterate than over one 20-club league.

The counter-argument is that T4.5 is "EA-critical" — you can't ship the procedural-world USP without
the world. True. But "build the world" and "build it *now*, before the core is fun" are different
claims. The world is content-and-scale work: it is the most seductive kind of work because it always
*looks* like progress and rarely fails a test. That is precisely why it's the wrong thing to reach for
while the hard, taste-dependent, failure-prone work (is the match gripping? do the callbacks land? is
there a decision to make on a Tuesday?) is unfinished. Fun and depth are substrate-independent — drama
tuning, commentary quality, callback-landing, a transfer loop, a cohesion ramp all work identically on
20 clubs and on 96. So do them where the feedback loop is fastest and the blast radius smallest, prove
the core is a game people want to play for 100 hours, *then* pour it into the wide procedural world you
already know how to generate.

The failure mode I'm warning against is concrete and, on the current trajectory, likely: you ship EA
with a gorgeous, genuinely unique, fully-scaled fantasy pyramid — and a match that's believable but flat,
a scout that's fog-over-numbers (the exact FM behaviour the USP table claims to beat), and an off-pitch
layer that is "pick XI, hit continue." A beautiful empty stadium. The pillars don't save it, because the
pillars were never the foundation — believable, deep, fun football was, and it got deferred behind the
work that felt safest. Build the small thing until it's fun. Then make it big.

---

## Verified CORRECT — do not re-litigate

Each was independently re-read by a skeptic. These are sound; spend no effort here.

- **Determinism primitives are correct and honored.** `seed_fn` is portable-by-construction (BLAKE3
  over a fixed 17-byte LE buffer, pinned by a layout-regression vector); Q32/BTreeMap/cordic/no-clock/
  no-thread_rng/no-async all hold; the post-T4 4-track review found **zero P0 correctness defects**.
- **Fixed-point/single-thread/BTreeMap perf is a non-issue at product scale** — ~12.4 ms for a full
  5400-tick 22-player match in release. Do **not** introduce floats or rayon in sim crates to chase
  throughput; the scale lever is algorithmic (the fast-path), not micro.
- **The determinism A/B-causal loop is real and used**, not theoretical — `drama_sweep` derives seeds
  deterministically and warns loudly when seed sets differ ("deltas are NOT causal"). The
  determinism-measures-fun idea is the right bet for this engine.
- **The in-flight FUN-0b+c work is well-engineered and broke open the 43-43.** Full matches now produce
  0-3 goals/seed (verified by running the envelope test); the canonical schema bump (v9→v11) is
  append-only and correct; the hash rebaseline was authorized and envelope-verified; the rewritten
  proptests are stronger, not vacuous; `Tick::checked_add` panics rather than saturates (Sim/RULES §11).
- **QA-T4H genuinely landed** — career-end player-identity, watermark advancement/fire-once, the two
  silent-blank fail-loud logs, and `PressTopic::as_dto_str` exhaustiveness are all FIXED and
  non-vacuous (only the `body_shield` exact-product back-port slipped — see P2).
- **SaveV4 migration chain + 4-test contract is complete, forward-only, non-vacuous**;
  `validate_for_load` runs before `restore_transient_state`.
- **Harvest atomicity is genuinely all-or-nothing** (`commands.rs:438-503`): a mid-loop error rolls back
  `season.results` before any roster/ledger mutation; goal/appearance harvest is exactly-once across the
  split-call pattern; `ledger.compact()` never shrinks the ledger.
- **The scout estimator is sound on the *believability* axis** — zero-mean/unbiased, the band **always**
  brackets the true category mean, and a real 2-point/1-20 quality gap inverts only ~12% of the time. It
  is **not** a noise generator that misleads the player into wrong calls. (Its gap is the *strategic*
  convergence promise — P2, not believability.) F1 and F2 fixes are confirmed intact.
- **The fun-pivot diagnosis and sequencing correction are correct**, the career-roster foundation was
  properly sequenced and shipped, `feature-backlog.md` is an honest gap analysis, and the `drama-model.md`
  realism-guard-vs-drama-target design is rigorous. The harness tools earned their keep (they found the
  velocity bypass and characterized the bimodal failure), and the team wrote its own anti-gravity
  guardrail — the issue is obeying it, not authoring it.

---

## Viability verdict (blunt)

**Will it ship? Plausibly — but not on the current sequence without slipping.** The hard architectural
bets are paid and sound; there are no correctness landmines; the team's process catches what it's pointed
at. **Will it be fun? Unproven, and gated entirely on work that is only now starting.** The match just
became *believable* in the working tree; *gripping* (FUN-1..4) and *deep* (any management loop) are still
ahead, and the roadmap risks letting a large world-scale phase consume the runway they need. **Is the
determinism bet earning its cost? Yes** — it's cheap, sound, a genuine calibration superpower, and not the
scale risk; only the per-commit tri-OS *cadence* is over-engineered. **Is "beats FM" hubris?** Not on the
realism *method* (calibration-to-aggregate-bands via deterministic sweeps is exactly right) — but the
*scope* (FM-class engine + 5 novel pillars + procedural world + 2000-player bake, solo-dev+Claude) is the
real bet, and the match engine, the thing every FM comparison rests on, is the least-finished, hardest,
least-de-risked component. Get the small world fun and deep first. The big world is the victory lap, not
the race.

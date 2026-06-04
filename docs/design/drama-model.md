# Drama model — deterministic fun metrics for match and season quality

**Status:** SPEC (provisional tuning values — calibrate once the engine produces watchable
football; gated on DX-2 inspection tooling + a first-pass "watchable match" milestone).
**Owner:** `systems-designer`.
**Tuning values:** per `docs/.claude/rules/design-docs/RULES.md` §4, all thresholds and target
bands in this doc are Phase-N tuning values. They live here, not in SPEC. They will be
revised in-place (with a dated block) after each calibration pass.

---

## Why this doc exists

The engine is deterministic. That is the whole payoff: change one coefficient, re-run the
exact same seed, and MEASURE whether the outcome changed — no noise, no "maybe the RNG was
unlucky." But that loop only has teeth if "better" has a computable definition.

This doc defines that definition. The central distinction:

**Realism guards** are hard plausibility bands the simulation must not leave. A 12-0 scoreline
breaks a guard. A realistic 0-0 grind does not — but it also earns no drama credit.

**Drama targets** are what we maximize INSIDE the realism guards. Late winners, comebacks,
leads that change hands, title races decided on the final day — these are measurable properties
of the deterministic event stream, and we tune toward them explicitly.

Realism ≠ drama. The guards keep us in the possible; the targets keep us in the interesting.

---

## How to use these metrics

**A/B coefficient test (same seed):** change one tuning constant, re-run seed `S` for `N` ticks,
compare per-metric deltas. Same seed + same tick count = any change in output is causal, not
RNG variance.

**N-seed sweep (move a distribution):** run 100–1000 seeds, collect the full metric distribution
for each coefficient variant, compare means + percentile bands. A tweak that improves the 90th
percentile without moving the 10th is not obviously good — inspect the tails.

These metrics are INTERNAL dev-loop numerics. They are never shown to the player as numbers.
The player sees commentary, results, and emergent narrative — not a "drama score" tooltip.

All metrics are pure functions of two existing deterministic outputs:
- `Vec<MatchFrameDto>` — per-tick stream (positions, score, possession) from `dump_frames` /
  `match_frames` IPC.
- `Vec<MatchEvent>` — event stream from `MatchState::match_events()` (Goals with tick + score
  snapshot, Shots, Passes, SignatureFirstFired, KickOff, FullTime).

No sim change. No canonical-state touch. Both canonical hash pins stay byte-identical.

---

## Match-level metrics

### M1 — Goals per match (realism guard)

**Definition:** total goals in one match = `home_score + away_score` at `MatchEvent::FullTime`.

**Source fields:** `FullTime::home_score`, `FullTime::away_score`.

**Classification:** REALISM GUARD.

**What GOOD looks like:** mean 2.6–2.8 goals/match over a 100-seed sweep; standard deviation
0.9–1.4. Tails: P5 ≥ 0 goals (0-0 happens), P95 ≤ 6 goals (7+ scorelines are rare at the
top level). The current engine's 3-match smoke run produced a 17-shot mean-xG of 0.194 with
a 0.529 empirical goal rate — well above target. This guard will fire until `utility_shoot`
rewiring (T2-1d2) suppresses low-xG shots.

**Phase-1 provisional bands:**

| Stat | Guard value |
|---|---|
| Mean goals/match | 2.3 – 3.2 |
| Std dev | 0.8 – 1.6 |
| P95 goals in one match | ≤ 7 |
| P5 goals in one match | ≥ 0 (0-0 allowed) |

---

### M2 — Goal timing distribution (realism guard)

**Definition:** for each goal in the event stream, record `tick / match_end_tick` as a
fractional minute proxy. Compare distribution across the match thirds (first 33%, middle 34%,
final 33%) against the target ratio.

**Source fields:** `Goal::tick`, `MatchEvent::FullTime::tick` (match end tick).

**Classification:** REALISM GUARD.

**What GOOD looks like:** goals should be roughly spread across the match, not clustered.
Real football sees slightly more goals in the final third (fatigue, chasing the game). The
current engine clusters goals in the first 15% of ticks (the T1 issue documented in
`match-quality-inspection.md` and the xg-coefficients calibration notes). This guard must
catch that.

**Phase-1 provisional band (across 100-seed sweep):**

| Third | % of all goals | Guard |
|---|---|---|
| First (0–33%) | 25–40% | FAIL if > 55% (current known-broken state) |
| Middle (33–66%) | 25–40% | — |
| Final (66–100%) | 25–45% | — |

---

### M3 — Competitive margin (drama target)

**Definition:** `abs(home_score - away_score)` at full time. A 1-goal game is maximally
competitive; a 3+ goal game is a blowout.

**Source fields:** same as M1.

**Classification:** DRAMA TARGET.

**What GOOD looks like:** over 100-seed sweep, ≥ 45% of matches decided by 1 goal; ≤ 15% of
matches decided by 3+ goals. Do not maximize this at the expense of M1 — squeezing all matches
to 1-0 is not drama, it is monotony.

**Phase-1 provisional targets:**

| Margin | Target share |
|---|---|
| 0 (draw) | 22–28% |
| 1 goal | 38–48% |
| 2 goals | 16–24% |
| 3+ goals (blowout) | 6–14% |

---

### M4 — Lead changes + equalisers (drama target)

**Definition:** count `lead_changes` as the number of times the leading team switches (home
ahead → away ahead or vice versa), and `equalisers` as goals that restore parity from a
deficit (score becomes `n:n` where it was `n:m`, `m < n`).

**Source fields:** walk `Vec<MatchEvent>`, tracking running score via `Goal::score_home_after` +
`Goal::score_away_after` (or reconstruct from KickOff + Goal events in chronological order
using `Goal::tick` ordering).

**Classification:** DRAMA TARGET.

**What GOOD looks like:** in a 100-seed sweep, mean lead changes per match 0.6–1.4; ≥ 25% of
matches have at least one lead change or equaliser. A world with zero lead changes after 100
matches indicates the sim is producing runaway scorelines or defensive lockouts, not football.

**Phase-1 provisional targets:**

| Metric | Target |
|---|---|
| Mean lead changes/match | 0.5 – 1.5 |
| Matches with ≥ 1 lead change or equaliser | 22 – 40% |

---

### M5 — Late drama rate (drama target)

**Definition:** fraction of matches where either (a) a goal is scored in the final 15% of ticks
(`tick / match_end_tick > 0.85`), or (b) a goal changes the result (not just the margin) in
the final 15%. Call the latter a "late winner" — the subset where a goal either breaks a draw
or reclaims the lead.

**Source fields:** `Goal::tick`, `MatchEvent::FullTime::tick`, `Goal::score_home_after`,
`Goal::score_away_after`.

**Classification:** DRAMA TARGET.

**What GOOD looks like:** over 100-seed sweep, ≥ 30% of matches have a goal in the final 15%
of ticks; ≥ 10% have a late winner. The "late winner" rate in real top-flight football is
roughly 8–14% of matches — use that as the real-world anchor.

**Phase-1 provisional targets:**

| Metric | Target |
|---|---|
| Matches with any late goal (final 15%) | 28 – 45% |
| Matches with a late winner or late equaliser | 9 – 18% |

---

### M6 — Comeback magnitude (drama target)

**Definition:** for each match, the largest deficit overcome. Construct the score timeline
from the Goal event stream; find the peak negative margin for the eventual winner (or for
either team in a draw). A match where the loser goes 2-0 up and loses 2-3 has comeback
magnitude 2.

**Source fields:** chronological `Goal` events, running score, final `FullTime` scores.

**Classification:** DRAMA TARGET.

**What GOOD looks like:** mean comeback magnitude across 100 seeds > 0 (some comebacks happen);
≥ 8% of matches have a comeback from ≥ 2-goal deficit. The upper bound is M1-constrained —
you cannot have a 3-goal comeback in a 2-goal-total game.

**Phase-1 provisional targets:**

| Metric | Target |
|---|---|
| Matches with any comeback from deficit | 15 – 35% |
| Matches with 2+ goal comeback | 5 – 12% |

---

### M7 — Nervy finish rate (drama target)

**Definition:** fraction of matches where the result is in doubt in the final 10% of ticks.
"In doubt" means: the current margin is 0 (level) or 1 (anyone can equalise or score a late
winner). Measure at tick `t = 0.90 × match_end_tick`.

**Source fields:** reconstruct running score at tick `0.90 × match_end_tick` from the ordered
Goal events. Compare to `FullTime` score to classify whether the final result was already
decided.

**Classification:** DRAMA TARGET.

**What GOOD looks like:** over 100-seed sweep, ≥ 45% of matches are in-doubt in the closing
stretch. A nervy finish doesn't require a late goal (M5); it requires the score to be close.

**Phase-1 provisional target:** 40 – 58% of matches in doubt (margin ≤ 1) at the 90% tick mark.

---

### M8 — Key-moment density (realism guard + secondary drama target)

**Definition:** count salient match events per match. A salient event is any member of
`{Goal, Shot, SignatureFirstFired}`. Passes and KickOffs are not salient for this metric.
Report mean events/match and standard deviation over a 100-seed sweep.

**Source fields:** `MatchEvent` discriminant filter on the `Vec<MatchEvent>`.

**Classification:** REALISM GUARD (lower bound) + DRAMA TARGET (shape).

**What GOOD looks like:** a match with zero shots is broken. A match with 80 shots is noise.
The guard catches both ends. Within the valid range, higher Shot + SignatureFirstFired density
means more memorable moments per match — the drama target.

**Phase-1 provisional bands:**

| Metric | Guard / target |
|---|---|
| Mean shots/match | 9 – 18 (guard) |
| Mean signatures fired/match | 0.5 – 4.0 (guard; sparse early, denser once catalogue grows) |
| Shot-on-target rate | 30 – 55% of shots (guard; requires `Shot::on_target` field at T2+) |

Note: `Shot::on_target` is not yet in the `MatchEvent::Shot` variant (T1 state). Until T2+,
M8 counts all shots; the on-target sub-metric is a T2+ addition.

---

## Season-level metrics

Season metrics require a results table: N matches, each with `(home_goals, away_goals,
home_club_id, away_club_id)`. The `fw-tauri` `play_match` IPC can generate this for any
(seed, archetype-pair) batch. Season-level metrics run over a league of ≥ 8 clubs, ≥ 14
matchdays (home + away). The sweep is over league seeds, not individual match seeds.

---

### S1 — Title race tightness (drama target)

**Definition:** final points gap between the champion and the runner-up. Construct the
final standings using standard 3-1-0 points (W/D/L). Report mean gap and the "decided early"
rate — fraction of seasons where the champion is mathematically guaranteed before the final
matchday.

**Classification:** DRAMA TARGET.

**What GOOD looks like:** mean points gap 2–7 over a 30-season sweep; "decided early" rate
≤ 35%. A gap of 1–4 points is a genuine race; 15+ is a procession. Real top-flight leagues
average roughly 4–8 points between first and second.

**Phase-1 provisional targets:**

| Metric | Target |
|---|---|
| Mean champion–runner-up points gap | 2 – 8 |
| Seasons decided before final matchday | ≤ 40% |

---

### S2 — Upset frequency (drama target)

**Definition:** fraction of matches where a lower-rated club beats a higher-rated club.
"Rating" is the mean squad attribute composite (`finishing × 0.3 + physical × 0.3 +
positioning × 0.4`, or the simplest available composite from `PlayerAttributes` at the time
of measurement). Report mean upset rate over a 30-season sweep.

**Classification:** DRAMA TARGET.

**What GOOD looks like:** 28–38% upset rate. Lower means the sim rewards quality too
deterministically; higher means quality is irrelevant and the game is random. Real football
sits roughly at 28–34% upset rate in balanced competitions.

**Phase-1 provisional target:** 25 – 40% of matches won by the lower-rated team.

---

### S3 — Table-spread variance (realism guard)

**Definition:** standard deviation of final-season points across all clubs in the table.
Measure over 20 independent league seeds. If spread variance is near zero across seeds, all
seasons look identical — procedural world generation is not producing different competitive
landscapes.

**Classification:** REALISM GUARD.

**What GOOD looks like:** per-season points spread (std dev across clubs) ≥ 8 points; seed-to-seed
variance in that spread > 0. Two seasons with identical standings are a flag, not a feature.

**Phase-1 provisional guard:** per-season std dev ≥ 6 points; inter-seed std dev of final standings
> 1.5 points across 20 seeds.

---

### S4 — Underdog over-performance (drama target)

**Definition:** track clubs ranked in the bottom-third of initial ratings. Report the fraction
that finish in the top half of the table. This is the "shock season" metric — a low-rated club
punching well above their rating for a full campaign.

**Classification:** DRAMA TARGET.

**What GOOD looks like:** 12–22% of bottom-third clubs finish in the top half. Below 8% means the
sim is too deterministic; above 28% means ratings are noise.

**Phase-1 provisional target:** 10 – 25% of bottom-third clubs finishing in the top half.

---

## Composite drama index (optional, secondary)

A weighted blend of the primary drama targets for single-number reporting during a tuning sprint.
The individual metrics are primary — a composite can hide a regression in one term. Use the
composite as a summary, not a gate.

```
DI = 0.25 × normalise(M3_competitive_margin_score)
   + 0.20 × normalise(M5_late_drama_rate)
   + 0.15 × normalise(M4_lead_change_rate)
   + 0.15 × normalise(M6_comeback_magnitude_score)
   + 0.15 × normalise(S1_title_tightness_score)
   + 0.10 × normalise(S2_upset_frequency)
```

Where `normalise(x)` maps the metric's target band to `[0, 1]` (0 = at guard floor, 1 = at
drama peak). A DI of 0.50 means all individual metrics are at the midpoint of their target
bands — a solid baseline. A DI of 0.70+ across 100 seeds would be an excellent result.

Season metrics (S1, S2) contribute only 25% of the index so a single-match sweep still yields
a meaningful DI even before the season simulator is wired.

Weight justification: M3 and M5 are the most player-visible drama signals (was the game
competitive? was there a late moment?), so they dominate. M4 and M6 reward the more nuanced
forms of drama. Season metrics are real but less tractable for per-coefficient A/B tests.

---

## Worked example — 5-match seed sweep trace

Seed `0xfeedbeefcafefade` (the canonical T1 smoke seed), 600 ticks (one half):

| Metric | Value | Guard / target | Status |
|---|---|---|---|
| M1 goals/match | 4 (2-2) | 2.3–3.2 mean over sweep | 4 goals in 600 ticks is above target — expected, T2-1d2 pending |
| M2 first-third goal % | ~75% (3 of 4 goals in first 15% of ticks) | FAIL if > 55% | GUARD FAIL — known issue |
| M3 competitive margin | 0 (draw) | 0-goal draw is in the target band | PASS |
| M4 lead changes | 2 (2-0, then 2-1, then 2-2) | 0.5–1.5 mean | One match: 2 changes — on the high side, healthy |
| M5 late drama | 1 goal in final 15% | ≥ 28% of matches over sweep | 1 match cannot satisfy sweep target |
| M8 shots | ~8 shots (from calibration smoke run) | 9–18 guard | Just below guard floor |

This example shows the two known failure modes — goal-timing clustering (M2) and shot
suppression (M8) — that T2-1d2 is expected to fix. After that fix, re-run all five metrics
and compare the sweep distribution.

---

## Calibration cadence

These are provisional Phase-1 values. The right time to calibrate is after T2-1d2 (`utility_shoot`
rewiring, which will materially change shot + goal distributions) and after DX-2 ships (which
gives the frame-stream substrate to run sweeps at scale without manual inspection). Re-fit
procedure:

1. Run `calibrate run --matches 100` (the T2-1d calibration binary).
2. Compute each metric over the 100-match corpus.
3. If any guard fails: treat it as a regression, fix before proceeding.
4. If any drama target misses badly (>2x outside the band): file a tuning row in MASTER_PLAN.
5. Update this doc with a dated "Phase-N re-fit" block. Do NOT delete prior values.

---

## Relationship to `fun-evaluation-harness.md`

This doc covers the computable half of automated fun-evaluation: pure functions of the
deterministic event + frame stream that yield reproducible numbers.

The forthcoming `docs/design/fun-evaluation-harness.md` covers the un-computable half: the
`drama-sweep` tool that automates running these metrics across N seeds, plus an LLM-judge
protocol for the things metrics cannot capture — "does this commentary callback land?", "does
the signature move feel earned?", "is the prose readable?". Together, the two docs define
an automated fun-evaluation pipeline that frees a human from having to judge match-feel
manually every time a coefficient changes.

---

## Cross-references

- `docs/design/match-quality-inspection.md` — DX-2 glitch-detectors; the same `dump_frames`
  output feeds both glitch-detection and drama scoring.
- `docs/design/xg-coefficients.md` — xG calibration; M1 and M2 depend on shot + goal rates.
- `docs/MASTER_PLAN.md` — DX-2 row (frame-stream substrate); T2-1d2 row (`utility_shoot`
  rewiring, expected to move M1 + M2 + M8 materially).
- `crates/fw-match-sim/src/dto.rs` — `MatchFrameDto` (the per-tick stream M2 and M7 use).
- `crates/fw-content/src/event.rs` — `MatchEvent` (the event stream all match metrics use).

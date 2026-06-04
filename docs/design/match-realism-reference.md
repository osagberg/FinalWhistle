# Match-realism reference — aggregate football anchors for tuning

**Status:** REFERENCE (research-grounded; 2026-06-04). Not a spec — a **tuning anchor sheet**: the
public aggregate football regularities the match engine + drama metrics should be calibrated against,
so the SOFT bands in `drama-model.md` and `tactical-shape.md` are grounded in real football rather than
intuition. Addresses ultra-review P2 ("realism anchors lack provenance + firmness tags").
**Owner:** `systems-designer`. **Baseline:** top-flight men's football (Premier League primary — the
best-documented public aggregate), modern decade (2015-2025) where era matters.

**Procedural-fantasy guard:** these are AGGREGATE statistical regularities (rates, distributions,
typical metres/percentages) — NOT licensed club/player data. The project's own calibration decision
(DECISIONS 2026-05-29) explicitly tunes to "public aggregate football bands." No proprietary/paywalled
datasets were used; sourced from Opta/TheAnalyst public pieces, peer-reviewed tournament studies,
football-analytics literature, and widely-reported league summaries (≥2 independent sources per
headline number).

**Firmness legend:** **HARD** = robust regularity stable across seasons/leagues (pin tightly).
**MEDIUM** = stable order-of-magnitude, drifts with era. **SOFT** = context/era/style-dependent (model
as a range, never hard-code). Where a number is era-sensitive, the *ordering* is usually HARD even when
the *value* is SOFT.

---

## 1. Scoring & shots → drama-model M1/M8, shot model

| Anchor | Value | Firmness | Engine use |
|---|---|---|---|
| Goals / match (mean) | **2.6–2.7** (PL all-time 2.65; range 2.45–2.82) | HARD | M1 calibration centre. Many-sim mean outside ~2.4–2.9 = a miss. |
| Per-match goal **distribution** | **~Poisson(λ≈2.7), unimodal, peaks at 2 goals**; ~0:7% 1:18% 2:24% 3:22% 4:15% 5+:14%; **~53% of matches Over 2.5**; mildly heavy-tailed (neg-binomial fits tail) | HARD (shape) / SOFT (exact bins ±2-3pp) | **The M1 shape/unimodality guard.** Real football is unimodal-at-2, NOT flat/bimodal — this is the anchor the 43-43 bimodal failure violated. The current drama-sweep guard (≥80% of matches in [0,5] goals, hard ceiling >8) is **consistent** with reality (~93% of real matches fall in [0,5]; 5+ is the ~14% tail). |
| Shots / match | **~24–26 combined (~12–13/team)** | MEDIUM | Shot-generation rate per 90; too few starves the goal model. |
| **Shots on target %** | **~33–35%** of shots | HARD | **Calibration gap: the engine sits ~60% — roughly 2× too clean.** Target ~1/3 reaching the keeper. The FUN-TS2 coordinated press (rushed/blocked shots) is the believable mechanism to pull this down. |
| Goals / shot | **~10–11%** | HARD | Consistency check below. |
| Goals / shot-on-target | **~30–33%** | HARD | Keeper-beaten rate on a real shot. |
| xG / (open-play) shot | **~0.10–0.12** | HARD | xG model per-shot mean; converges with goals/shot by construction. Long-range ~0.03–0.05; tap-in ~0.5–0.7; pen ~0.76–0.79. |
| Penalties / match | **~0.25 awarded** (both teams), **~77% scored** | MEDIUM (award) / HARD (conversion) | Discrete high-xG, high-salience event. |
| Goal source split | open play **~70–75%**, set pieces **~20–22%**, pens **~8–10%** (set-piece share rising recently) | MEDIUM | Routing of goals through phases. |

**Consistency check (the load-bearing one):** ~25 shots/match × ~10.5% conversion ≈ **2.6 goals/match**.
The shots-per-match, on-target%, conversion, and goals/match anchors must close this loop — if the
engine hits goals/match by over-converting too few clean shots (the current ~60%-on-target path), it is
right for the wrong reasons and will not transfer to differentiated rosters.
*Sources: footballhistory.org (PL goals); bookdown honors thesis + arXiv 2105.09881 (Poisson goal model);
WinDrawWin (Over/Under 2.5 ≈53%); SofaScore + Sportmonks (on-target ~33%); Sportmonks + STATSCORE
(conversion 10-15%); American Soccer Analysis + SoccerEDU (xG/shot ~0.11); Opta Analyst (pens 0.78 xG).*

---

## 2. Possession & passing → FUN-TS3 build-up

| Anchor | Value | Firmness | Engine use |
|---|---|---|---|
| Match possession | mean **50/50**; winner tilt **53–55%**; dominant side **60–61%**; counter side **38–42%**; league spread **~20pp** | HARD (mean + spread) / SOFT (which end) | **Possession-lock guard:** cap a dominant side near **~65/35**, rare beyond, never the 80%+ self-sustaining lock FUN-0 had to break. |
| Pass completion (overall) | **~83–86%** (Euro24 86%, Copa24 82%, PL teams 75–89%) | HARD | Per-pass build-up resolution converges to low-to-mid 80s% — not a coin-flip, not ~100%. |
| Completion **ordering** | backward **~92–96%** > short **~85–90%** > forward **~70–78%** > long **~50–60%** | HARD (ordering) / SOFT (exact fwd%) | **Do NOT use one flat completion rate.** Safe recycling almost always lands; progressive/line-breaking passes are riskier — turnovers (and the counters they spawn) should cluster on failed FORWARD passes, not square balls. This ordering is what makes build-up read as football. |
| Passes / team / match | **~450–550** (combined ~870–1060) | HARD (order) / SOFT (band) | Tick-budget sanity bound: ~400–550/side over 90 min, not 50 (no midfield) or 2000 (tiki-taka lock). |
| Possession-sequence length | mean **~4 passes, right-skewed** (SD ≥ mean); **~90–100 possessions/team/match**; turnover ≈ every ~4 passes | HARD (shape) / SOFT (mean) | **The key FUN-TS3 shape constraint:** many short possessions + a fat tail of occasional long build-ups — never a fixed length (that feels mechanical). |
| Build-up vs direct | direct/short dominates; sustained 10+ pass→shot is the minority + style-dependent; 2024-26 trend more direct | SOFT | Tactical identity drives patient build-up vs vertical transition; sustained build-up goals are the satisfying exception, not the default. |

*Sources: Opta/TheAnalyst + FBref (possession spread); PMC peer-reviewed Euro/Copa 2024 study + StatMuse
(pass completion + ordering + sequence length); FotMob/FBref (passes/match); Opta (build-up vs direct).*

---

## 3. Defensive shape & pressing → FUN-TS1 / FUN-TS2 (the believability core)

Distances are **metres from the defending team's own goal line** (pitch ~105×68m; penalty area 0–16.5m;
halfway 52.5m).

| Anchor | Value | Firmness | Engine use |
|---|---|---|---|
| **Defensive line height** (last line x) | low/deep block **~22–28m**, mid block **~38–42m**, high line **~48–55m**; league avg **~44m** (risen over the decade) | SOFT (metres) / HARD (20–55m envelope + low<mid<high ordering) | **`line_x` per tactic state (FUN-TS1).** ⚠ **Refines the tactical-shape.md provisional seeds** (18/35/55m): real low/mid blocks sit a touch higher — seed **LowBlock ~25m, MidBlock ~40m, HighPress ~52m, CounterAttack ~45m** and tune from there. |
| Inter-line vertical gap | **10–15m** between adjacent lines | HARD | "No space between the lines" — the core compactness rule. |
| Vertical compactness (def→fwd, out of possession) | **~25–35m** total (3 lines × ~10–15m) | MEDIUM | `compactness_v` out of possession (provisional 25/32/40 is in-range; the 40m HighPress value is the loose end, which is correct — higher press = more stretched). |
| Horizontal width | out of possession **~30–44m** (narrows ball-side); in possession **~55–68m** (near full width) | HARD (the in/out swing) | `compactness_h` must contract defending + expand attacking — that swing is what reads as organised-defending vs open-attacking on the 2D board. |
| **PPDA** (pressing intensity) | extreme press **~4–8**, mid **~9–12**, low block **13+**; league spread ~5–18; league avg has *risen* (~9.7→13.1, i.e. less pressing) over the decade | HARD (4–18 envelope + ordering) / MEDIUM (absolute mean) | Pressing-intensity dial (low PPDA = aggressive). **Confound:** PPDA co-varies with possession/match-state — treat as a dial, not a clean knob. |
| **Press coordination** | **≥2 players** to be valid; a subset (**~3–5 of 11**: front line + nearest mid) engages in coordinated bursts off cues while **~6–8 hold the line**; press effectiveness decays with fatigue | HARD (line-not-swarm principle) / SOFT (exact counts) | **The literature backing for FUN-TS2's "press = line not swarm" rule** (1 primary + 2 cover is the floor; ~3–5 engaging is the real band). Never 11 individuals chasing the ball. |
| Offsides / match | **~1.5–2.0 per team, ~3–4 combined**; high-line teams skew to the 4–5 end | MEDIUM | FUN-TS2 offside calibration target. (Ignore obscure-league 8+/match figures — not a top-flight anchor.) |

**Cross-era tension to encode:** over the decade, lines went **UP** (more territory) *while* PPDA went
**UP too** (less intense pressing) — modern teams sit higher but press less frenetically than the
2014-era gegenpress peak. **The engine's tactic presets should let "high line" and "high press" vary
independently, not bolt them together.**
*Sources: The Football Analyst + StatsUltra (line height); Café Tactiques (decade line-height + PPDA
trends); Footballizer + Coaches' Voice + Explored Football (PPDA ranges + 1,826-match fatigue study);
OneFootball/Total Football Analysis (10–15m inter-line compactness); The Football Analyst (pressing
triggers / ≥2 players); Squawka + StatMuse (offsides per team).*

---

## 4. Drama & competitiveness → drama-model M3–M7, S1–S2

These keep "drama" the realistic tail of a real distribution — not a maximised or scripted artifact
(the FUN-AS anti-scripting concern).

| Anchor | Value | Firmness | Engine use |
|---|---|---|---|
| Outcome margin | draws **~26–27%**; **1-goal-margin ≈ 50% of decisive** games (1-0 alone ~18–20%, the modal scoreline); 3+ blowouts **~13–16%** | HARD | M3: a top-flight match is a draw or 1-goal game ~2/3 of the time; blowouts are the ~1-in-7 tail. |
| Comeback wins (recover deficit to win) | **~15–17%** of matches (rising: subs, longer stoppage) | SOFT (rising) | M4 small-deficit comeback target. |
| **2+ goal comeback win** | **~0.7%** of such games (~1 in 150) | HARD | M6 ceiling: a 2+ goal comeback must stay a rare spectacle — if the engine produces them often, drama is being manufactured. |
| Late goals | **~25% of goals in the final segment** (76–90+); final 10 min is the single most prolific block (~18–19%) | HARD | M5: concentrate goals/late winners in the closing phase — without overdoing scripted 90th-minute theatre. |
| Home / draw / away | long-run **~46/27/27**; modern decade **~44–45/26/29** (home edge declining, away rising) | HARD (home edge exists) / SOFT (magnitude) | S1 home-advantage baseline: home win clearly > away, without the 1990s 60%+ inflation. Calibrate to the modern decade. |
| Upset rate | favourite loses outright **~1 in 4**; draws absorb another ~1 in 4; football is **structurally high-upset** vs other team sports (low scoring → high variance) | HARD (high variance) / SOFT (exact %) | S2: the rating→result map must let the weaker side win often enough that the table isn't deterministic. (NB: NFL point-spread "underdog %" figures do NOT transfer — football has no spread + is lower-scoring.) |
| Title gap (38-game league) | champion **~87 pts**, runner-up **~80**, gap **~7 pts mean but BIMODAL** — frequent ≤1-pt nail-biters + occasional 12–19pt runaways; ~1 in 4 titles decided by ≤1 pt | HARD (point levels) / SOFT (gap) | S1 season-drama: champion usually ~5–10 clear, with photo-finishes AND runaways at realistic rates. |

*Sources: soccerstats + Opta + r-bloggers (margins/draws/1-goal); Opta + Sports&Betting (comebacks +
2-goal-lead drops); soccerstats + StatsUltra (goal timing); Sky Sports + soccerstats (home advantage);
ScienceDirect + OddsShark (upset/variance); Opta + BettingOffers (title points).*

---

## How this anchors the tunable bands

- **`drama-model.md` M1:** centre 2.7 goals/match; the shape guard (unimodal, ≥80% in [0,5], ceiling >8)
  is validated by the real Poisson-peak-at-2 / ~14%-tail distribution. M3/M4/M5/M6/M7 + S1/S2 bands map
  to §4 above. **Action:** add provenance + firmness tags to drama-model bands citing this doc.
- **`tactical-shape.md` FUN-TS1:** `line_x` seeds should move toward §3 (LowBlock ~25m / MidBlock ~40m /
  HighPress ~52m); `compactness_v` ~25–35m out of possession with the 10–15m inter-line rule;
  `compactness_h` must swing contract↔expand. **Action:** nudge the SOFT seeds + decouple line-height
  from press-intensity (the cross-era tension).
- **FUN-TS2:** press = ≥2 (real band ~3–5) engaging while ~6–8 hold shape; PPDA dial 4–18; offside ~3–4
  combined/match.
- **FUN-TS3:** possession cap ~65/35; completion ordering back>short>forward>long; sequence mean ~4
  right-skewed; ~450–550 passes/team.
- **Shot model:** the headline calibration gap is **on-target ~33% (engine ~60%)** — close it via FUN-TS2
  pressure, not by lowering conversion to compensate; verify the shots×conversion≈goals loop closes.

## Cross-references

- `docs/design/drama-model.md` — the metrics these anchors calibrate (M1–M8, S1–S4).
- `docs/design/tactical-shape.md` — FUN-TS1/TS2/TS3 tuning bands grounded here.
- `docs/design/shot-model.md` / `xg-coefficients.md` — the on-target + xG calibration.
- `docs/DECISIONS.md` 2026-05-29 — "calibrate to public aggregate football bands".
- `verification/ultra-review-2026-06-04.md` P2 — the provenance/firmness-tag gap this closes.

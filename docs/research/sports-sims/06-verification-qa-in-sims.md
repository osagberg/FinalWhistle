# Verification + QA in deep-sim sports games — research notes

**Researched:** 2026-05-13
**For:** Final Whistle T1-9 (behavioral assertions) + ongoing dev-verification strategy

## Sources

- [PC Gamer — Miles Jacobson on FM 2011 (page 2)](https://www.pcgamer.com/miles-jacobson-on-football-manager-2011/2/) — SI QA team sizing, year-long beta-team cycle. [primary, paraphrased journalism]
- [FourFourTwo — Miles Jacobson on how FM gets made](https://www.fourfourtwo.com/features/miles-jacobson-how-we-make-football-manager-future-and-where-you-come-it) — iteration plan (March → July), weekly dossier. [primary]
- [PCGamesN — FM2016 launch-day polish](https://www.pcgamesn.com/football-manager-2016/football-manager-2016-s-miles-jacobson-on-launch-day-polish-fantasy-draft-and-twitter-trolls) — Sega QA layered on top of in-house. [primary]
- [Footballmanagerblog.org — FM26 beta engagement](https://www.footballmanagerblog.org/2025/10/fm26-bug-fixes-hotfix-patch-notes.html) — 200k daily managers during public beta; bug-tracker scale. [secondary]
- [OOTP Manual — Historical Settings + Stats and AI](https://manuals.ootpdevelopments.com/index.php?man=ootp22&page=ootp9-help_game_create_new_game_page.strategy) — `League Totals modifiers` auto-adjust each season to historical year totals; modifier-default-1.000 lever for HR/BB/K rates. [primary]
- [OOTP wiki — Statistics + Stats and AI screen](https://wiki.ootpdevelopments.com/index.php?title=OOTP_Baseball%3AScreens_and_Menus%2FLeague_Menu%2FLeague_Settings%2FStats_and_AI) — runtime knobs the engine exposes for stat calibration. [primary]
- [Vice — interview with Markus Heinsohn](https://www.vice.com/en/article/yp3pqx/out-of-the-park-baseball-17-markus-heinsohn) — OOTP origin "Hardball 4's stats engine was horrible"; DIPS adopted in v6. [secondary]
- [EHM The Blue Line — Quick Sim vs Enhanced Sim](https://www.ehmtheblueline.com/forums/viewtopic.php?t=5490) — community-confirmed: Quick Sim stats track real-life closer than Enhanced Sim because abstract slicing is easier to calibrate than a tick-accurate simulator. [secondary]
- [EHM The Blue Line — sim/detail settings](https://www.ehmtheblueline.com/site/2020/04/21/ehm-simulation-detail-settings-explained/) — public beta opt-in flow. [secondary]
- [ACM Queue — Automated QA Testing at EA, Driven by Events](https://queue.acm.org/detail.cfm?id=2627372) — EA's event-driven test automation framework starting FIFA 10; replaced 20–22 manual testers per session. [primary]
- [GDC Vault — Behavior is Brittle: Testing Game AI](https://gdcvault.com/play/1024564/Behavior-is-Brittle-Testing-Game) — industry panel on emergent-AI test brittleness; advocates property-style assertions over scripted scenarios. [primary]
- [Wikipedia — Characterization test / Golden Master](https://en.wikipedia.org/wiki/Characterization_test) — formal description of pinning + drift-detection regression style FW already uses. [secondary]

## Per-game / per-technique findings

### Football Manager testing process
- **Year-round beta team** dedicated to the ME (separate from main QA). Iteration plan kicks off March, hard-stops July → bug-fix lockdown.
- **9 lead QAs in-house** + ~10 full-time + 30–40 contractors annually + Sega's QA layer on top.
- **Public beta** the dominant integration test. FM26 saw ~200k daily managers during beta — community is effectively SI's "long-tail stat-distribution test harness." Forum-based bug tracker triages issues like "defenders scoring at unusual rates" — exactly the kind of emergent regression internal QA misses.
- **Miles' weekly dossier** is a curated narrative of "things that feel off" surfaced from playtest. Subjective, not automated.

### OOTP regression testing
- The most explicit stat-calibration approach in the genre. `League Totals modifiers` auto-adjust after every simulated season toward the *real* historical year's totals (HR, BB, K, AVG, etc.) — runtime closed-loop stat-distribution control, not a test gate per se.
- Designer-exposed modifiers default to 1.000; player-level tuning lever for HR rate, BB rate, etc. This is the productized form of "expose calibration knobs so balancing doesn't churn code."
- No public evidence of automated KS / chi-square tests; the validation is "run a season, eyeball league leaderboards against real life."

### Front Office Football
- Solecismic is a one-person shop. Testing is forum-driven (FOFC). Calibration done by uploading season exports and letting the community audit stat distributions.
- The FOF/OOTP merge (2018–2020) and split (2020) suggest the engine code was treated as a guarded algorithmic asset — Gindin retained rights, implying the engine itself is small enough for one author to hold in his head + verify by eyeball.

### Others
- **EA Sports FC / FIFA**: ACM Queue paper documents an *event-driven automated stability test* framework (replays mocked gameplay sessions, asserts on crash + state-validity events). Focus is stability + technical correctness, not behavioral realism. Behavioral realism is still humans + closed playtest cohorts ("Technical Test" cycles).
- **EHM**: confirms the tradeoff Final Whistle already chose to fight: a higher-fidelity sim is harder to calibrate to real-world stats than an abstract slice-sim. EHM uses two engines partly to escape this — Quick Sim for calibration, Enhanced Sim for fidelity. Community-noted divergence between the two.
- **NBA 2K MyLeague**: no public engineering writeups. Community concern is QA gaps (force-sim wins, age-curve regressions). Stat-distribution sliders are the user-facing knob, mirroring OOTP's pattern.

## Cross-cutting techniques

- **Stat-distribution regression** — universal. OOTP's auto-adjust closed loop is the strongest form; everyone else does eyeball-on-leaderboard.
- **Golden-replay / canonical-state pins** — rare in shipped sports sims. Most are non-deterministic by design (thread RNG, hash-randomized maps). FW's pinned BLAKE3 hash on a fixed-seed scenario is genre-leading discipline; the closest analog is EA's event-replay framework but that's a stability harness, not state-hash equality.
- **Behavioral assertions / proptest invariants** — virtually nobody in this genre encodes these as code. GDC "Behavior is Brittle" advocates this style but it remains aspirational for shipped sports sims.
- **Human eyeball / season-sim QA** — the dominant validation everywhere. "Sim 10 seasons, look at top scorers, does it look right?" — explicit at OOTP, FOF, FM.
- **Community / open-beta cycles** — the genre's load-bearing emergent-bug catcher. FM's 200k-daily-tester beta is the most extreme case; EHM, OOTP, FOF all run smaller versions of the same loop.

## What Final Whistle is already doing right

- **Pinned canonical-state hash + cross-OS matrix** exceeds anything publicly documented from FM/OOTP/EHM/FOF. Determinism-by-construction is a structural advantage no shipped sim has.
- **Diagnostic commentary + tactical board (dev-tier)** is the in-house equivalent of FM's "weekly dossier" but mechanized and reproducible per seed — i.e. a human-readable telemetry layer that pairs with the hash gate.
- **Behavioral proptest invariants** as a first-class verification layer is rare-to-unique in the genre.
- **Banned-terms lint + content-pack FW-VAL** corresponds to FM/OOTP forum-driven copy/content QA but moved to CI.

## What we might be missing

- **OOTP-style closed-loop stat-distribution auto-calibration** — even if not runtime-adjusting, a CI gate that runs an N-season simulation and asserts that match-level distributions (goals/match, shots/match, completion %, possession %) sit inside a real-world envelope (KS-test against a pinned reference distribution). Stronger than scalar invariants.
- **Two-engine cross-check (EHM-style)** — a "quick sim" path (Poisson + Dixon-Coles style) we tune against real-world totals, then assert that the full ME's aggregate output matches the quick-sim's distributions over many seasons. Catches systemic drift the hash gate misses (the hash can be stable yet the sim quietly drifts away from real football).
- **Community-style emergent-bug catcher** — solo-dev can't get 200k testers, but we can: stash anonymized canonical event ledgers from playtest runs and run a "leaderboard plausibility" check (top scorers, golden boot totals, clean sheets, red-card counts) every CI.
- **EA-style event-driven replay harness** — replay a recorded human session against the current engine, assert no panic / no banned event-type / no Q32 overflow. Complements the seeded canonical-hash test by covering input shapes the seeded scenario doesn't.

## Direct application

Concrete additions to T1-9 behavioral assertions list:

1. **`assert_seasonal_goal_rate_in_band`** — over a 38-match league sim at a fixed seed, total goals / matches ∈ [2.4, 3.1]. (Real EPL: 2.6–2.9.)
2. **`assert_shots_per_match_in_band`** — shots/match ∈ [22, 28] aggregate (real ~25).
3. **`assert_pass_completion_distribution`** — team completion % falls in [70, 90] for ≥95% of matches; mean ∈ [78, 84].
4. **`assert_top_scorer_concentration`** — Gini-style: top scorer's goals ≤ 35% of team total; top-5 scorers ≥ 55%. Catches "one striker scores everything" regressions.
5. **`assert_card_distribution`** — yellows/match ∈ [3, 5], reds/match ∈ [0.05, 0.20].
6. **`assert_home_advantage_present_but_bounded`** — home win % ∈ [40, 50], away win % ∈ [25, 35] over a full season. Catches "home advantage broken" + "home advantage extreme" symmetrically.
7. **`assert_signature_move_diversity`** — over 38 matches, ≥18 of the 24 signature moves fire at least once. Catches "only the same 3 signatures ever trigger."
8. **`assert_breakthrough_trigger_rate`** — breakthroughs per player-season ∈ [0.3, 1.5]. Catches "nobody ever grows" + "everyone breaks through every match."
9. **`assert_scout_disagreement_bounded`** — std-dev across N scouts on the same player's true gene falls in a designed band. Catches "all scouts converge" (scouting flat) + "all scouts diverge wildly" (scouting noise).

Pair (1–6) with a **KS-test against a pinned real-world distribution file** (`docs/design/reference-distributions.ron`) for stronger detection than scalar bounds. (7–9) are FW-specific and have no shipped analog.

## Open questions

- Does OOTP publish its `League Totals modifier` calibration code path anywhere? Worth a deeper forum dig if we want to mimic the auto-adjust loop.
- Are there any GDC talks specifically on *baseball-management* engine testing that go past the 2018 Sims talk?
- The ACM Queue EA paper is from FIFA 10 era — has EA published an update on FC 26's automated test surface? (Search came up dry but worth tracking.)
- Could we get a single ex-SI employee statement on whether FM has *any* code-level stat-distribution assertion, or whether it's 100% human-eyeball + beta-community? Worth a targeted LinkedIn/X search before T1-9 lands.

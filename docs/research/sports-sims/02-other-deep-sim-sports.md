# Other deep-sim sports games — research notes

**Researched:** 2026-05-13
**For:** Final Whistle T1-2b (22-player BT match engine)

## Sources

- [OOTP Pitching Ratings manual (OOTP 16+)](https://manuals.ootpdevelopments.com/index.php?man=ootp16&page=pitching_ratings) — Stuff / Control / Stamina ratings feed pitch outcomes. [primary]
- [OOTP wiki: Pitching Ratings & Player Model](https://wiki.ootpdevelopments.com/index.php?title=OOTP_Baseball:Important_Game_Concepts/The_Player_Model/Baseball_Ratings/Pitching_Ratings) — DIPS-theory baked into engine. [primary]
- [OOTP 15 manual: game controls / one-pitch vs pitch-by-pitch modes](https://manuals.ootpdevelopments.com/index.php?man=ootp15&page=game_controls) — engine simulates every pitch; UI choice is whether to display them. [primary]
- [MojoTech blog: extracting OOTP game logs](https://www.mojotech.com/blog/opening-day/) — confirms engine emits per-pitch text events as the canonical stream. [secondary]
- [Vice interview, Markus Heinsohn](https://www.vice.com/en/article/yp3pqx/out-of-the-park-baseball-17-markus-heinsohn) — sole engine author for ~25 years; multi-threading only recently added. [secondary]
- [Solecismic — Front Office Football site](http://www.solecismic.com/solecismic.php) — depth-charts + game-plan scripts drive sim. [primary]
- [Solecismic FOF9 docs/FAQ](http://www.solecismic.com/documentation/dokuwiki/doku.php?id=faq) — 48 player skills; rebuilt code base for sim speed in FOF9. [primary]
- [EHM Wikipedia](https://en.wikipedia.org/wiki/Eastside_Hockey_Manager_\(video_game\)) — SI Games-built top-down 2D engine with physics for deflections/shot-blocking. [secondary]
- [EHM The Blue Line — match engine forum thread](https://www.ehmtheblueline.com/forums/viewtopic.php?t=5490) — community reports the engine pre-computes scoreline then animates to it; re-sims on tactical change. [secondary]
- [WeLoveCycling interview with Cyanide PCM designer](https://www.welovecycling.com/wide/2021/11/05/how-is-the-legendary-pro-cycling-manager-developed-an-interview-with-a-game-designer/) — race AI development context. [secondary]
- [PCM26 "Detailed Simulation" feature](https://cyanide-studio.com/en/cyanide-games/pro-cycling-manager-26/) — full 3D engine run headless, no visuals, "in seconds". [primary]
- [Tennis Elbow 2013 PDF manual](https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/346470/manuals/www_managames_com_tennis_doc_En_Documentation.pdf) — point-by-point / game-by-game / skip-result simulation tiers. [primary]
- [DraftKings Engineering: Building a Tennis Simulation](https://medium.com/draftkings-engineering/building-a-tennis-simulation-d6afdaa97d19) — Monte-Carlo per-point with Bayesian momentum updates. [secondary]
- [F1 Manager 2023 race simulation page (Frontier)](https://www.f1manager.com/2023/features/focus/race-simulation) — Unreal Engine 5; per-driver line/tyre/fuel AI. [primary]
- [F1 Manager Wikipedia](https://en.wikipedia.org/wiki/F1_Manager_2022) — UE4→UE5 migration. [secondary]
- [Wikipedia: Discrete-event simulation](https://en.wikipedia.org/wiki/Discrete-event_simulation) — general DES theory; clock hops to next event vs fixed-Δt. [secondary]

## Per-game findings

### OOTP Baseball
- **Per-pitch discrete-event engine** is canonical even in "fast" mode: in One-Pitch mode the engine still simulates every pitch internally, only the last one is rendered to UI. Pitch-by-pitch mode just displays each event.
- **Ratings → outcome roll**: every pitch consults Stuff (strikeout propensity, derived from per-pitch-type ratings + velocity), Control (walk rate), Stamina (degradation over pitch count), Hold Runners, plus hitter ratings. Each pitch outcome is a stochastic resolution against this composite.
- **DIPS theory baked in**: pitchers don't get credit/blame for batted-ball-in-play outcome — fielder skill + park factors + RNG resolve that branch. This is a deliberate "what attribute matters here?" architectural choice.
- **Scale**: ~150k at-bats/season × thousands of seasons in long careers. Single-author engine (Heinsohn) for ~25 years; multi-threading only added recently. The engine optimises for serialised throughput, not per-tick fidelity.

### Front Office Football
- **Play-by-play discrete events** keyed on (down, distance, field position, clock). Game-plan is a scripted decision table — the player authors plays-to-call rules; the sim executes them. No real-time agent ticking.
- **48 player skills** + 16 defensive coverage shells + double-team/spy/blitz flags. The sim picks one play per side per snap, resolves blocking and the resulting yardage probabilistically.
- **Calibration via mass replay**: charts are "based on simulation of tens of millions of plays" — Solecismic's correctness story is statistical aggregate, not per-snap fidelity. FOF9 rebuilt the code base "for simulation speed, growth and flexibility".

### Eastside Hockey Manager (SI Games)
- **Top-down 2D engine** with physics for puck deflections and shot-blocking, similar in lineage to Football Manager's match engine.
- **Pre-computed scoreline pattern reported by community**: the engine appears to compute the final scoreline first, then play out animation to hit it. Mid-match tactical changes trigger a re-sim of the remaining time. (Unconfirmed by SI; community-observed.) This is a hybrid: deterministic per-tick visuals on top of a statistical outcome backbone.
- **Multiple detail levels**: lower-detail leagues use abstracted event sampling; only the user's match runs the full engine.

### Pro Cycling Manager (Cyanide)
- **Full 3D physics engine** runs at race-render frame rate during live view; PCM26 introduced a "Detailed Simulation" mode running the same engine **headless** and producing a result "in seconds" — strong evidence the engine is internally a fixed-Δt continuous-physics sim that can be decoupled from rendering.
- **Multi-hour race horizon (Grand Tours)**: behaviour layered as long-horizon strategy (attack here / defend here) plus short-horizon mechanical state (gradient, fatigue, draft). Riders are agents with rules like "follow attacks ≤ threshold, react to leader, conserve for sprint".
- **Player-centric AI bias**: community reports the field collectively reacts to the human player. Suggests cooperative/team AI is gated on "is this the player's team?" rather than emergent peloton dynamics — a fragility worth avoiding.

### Tennis Elbow Manager (ManuTOO / Mana Games)
- **Three tiers of sim depth**: Point-by-Point, Game-by-Game, Skip-to-Result — outcomes can diverge between tiers, suggesting they are not just rendering variants of one engine but actually different stochastic models.
- **Underlying model is per-point stochastic** with player style as bias (serve-and-volley vs baseliner shifts conditional probabilities). DraftKings' parallel approach uses per-player serve win % + Bayesian momentum updating — same shape, different parameters.
- **Match Markov chain**: tennis's natural structure (point → game → set → match) makes a hierarchical Markov chain almost free. No 22-agent equivalent: 2-agent + ball.

### F1 / MotoGP Manager (Frontier)
- **Unreal Engine 5 ECS-style tick** (UE4 in F1M22, UE5 from F1M23 onward) renders the race at 60Hz. Driver AI runs alongside as agents holding state: tyre wear, fuel, pace target, attack/defend orders from pit wall.
- **Per-driver racing-line variation**: the same corner is taken differently depending on order ("push harder" → more kerb, "back off" → safer line). Pit-stop strategist is its own agent reacting to track conditions. Distinct from rider/cycling: F1 is short horizon (race < 2 hours) with high mechanical state churn.
- **No public commitment to determinism** (it's Unreal; floats abound). Save-mid-race replays differ — not the model FW should copy.

## Cross-cutting patterns

- **Discrete-event vs continuous physics splits along the sport's natural unit.** Sports with crisp atomic events (baseball pitch, tennis point, NFL down) lean DES: clock hops between events, no fixed-Δt loop. Sports with continuous play (hockey, cycling, F1) lean fixed-tick continuous sim. Football sits in the continuous camp.
- **All of the continuous-play sims keep a "skip to result" path**, and at least two (EHM, PCM26) suggest the engine can run headless at much higher speed than render rate. The same physics, no rendering, batched output.
- **"What attribute matters here?" is solved by per-event branching, not per-attribute weights.** OOTP picks a roll table based on (pitch type, count, runner state) — only the relevant ratings enter. FOF picks a play-resolution branch per (down, formation, coverage). This is closer to a behaviour-tree leaf than a multivariate regression.
- **Single-author engines are common** (Heinsohn / OOTP, Solecismic). They tend to evolve via per-version recalibration of stochastic tables rather than architectural rewrites — survival of the engine depends on the stochastic model being legible to its author years later. Bias toward clarity over cleverness.
- **None of these sims are publicly committed to cross-OS canonical-hash determinism.** OOTP's reproducibility is "good enough for statistical aggregates"; F1M is Unreal float soup. FW's determinism floor is a stronger guarantee than any of the commercial peers.

## Direct application to Final Whistle T1-2b

- **Keep football as continuous-tick (60Hz Q32.32).** Of the six sports here, only baseball/American football/tennis are natural DES; football is closer to hockey + cycling, both of which run continuous engines.
- **Steal OOTP's "engine emits a canonical event stream the UI consumes" pattern.** Per-pitch text events are OOTP's true sim output; the visual render is downstream. FW's `MatchEvent` ledger should be the same: 60Hz canonical-state hash + an event stream that the 2D tactical board, commentary, and replay all consume.
- **Steal Tennis Elbow's tiered sim-depth ladder for non-user matches.** Detail-level setting per league (full 22-agent BT for user matches; coarser sampled-outcome path for background fixtures) is the standard solve for season-throughput. Document the abstraction boundary explicitly so background-sim drift can be measured against the full sim.
- **Avoid EHM's pre-computed-scoreline-then-animate pattern.** That model breaks "careers that remember" — if the canonical state is the scoreline, then a tick-resolved breakthrough moment can't be load-bearing. FW's pillar requires the per-tick events to be the source of truth, with the scoreline derived.
- **Avoid PCM's player-centric AI bias.** Final Whistle's 22 agents must run the same BT regardless of which side the user manages; team identity is not an input to the agent's decision policy.

## Open questions

- Does OOTP's One-Pitch mode actually skip the inner pitches, or just hide the play-by-play strings? (Affects whether tiered detail is "different model" vs "same model, fewer log writes".)
- How does EHM's mid-match tactical-change re-sim handle the time already played — is it canonical, or do the earlier events get rewritten? (Determinism contract question for FW's substitution / tactical-shift handling.)
- Is there a published Sports Interactive (FM) engine post-mortem we could read? Their hockey/football lineage is the closest comparable, and §5 of CLAUDE.md's `feature-dev:code-explorer` should sweep `docs/archive/` for FM-engine writeups already in our REFERENCES.
- Is Tennis Elbow's "different model per tier" intentional, or a bug? Either answer informs FW's tiered-sim contract.

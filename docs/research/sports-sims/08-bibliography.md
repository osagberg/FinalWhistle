# Sports-sim + game-AI bibliography

**Compiled:** 2026-05-13
**Purpose:** Starting point for future Final Whistle research sessions. High-signal only.

A grep-friendly index of public technical resources on deep-simulation sports games and game AI architecture. Quality > quantity — every link earns its place. Notes are terse on purpose; click through to read.

---

## 1. GDC talks

- [Three States and a Plan: The A.I. of F.E.A.R.](https://www.gamedevs.org/uploads/three-states-plan-ai-of-fear.pdf) — Jeff Orkin — GDC 2006 — the canonical GOAP paper; STRIPS-style planner for shooter AI. PDF mirror; also on GDC Vault.
- [Goal-Oriented Action Planning: Ten Years Old and No Fear!](https://www.gdcvault.com/play/1022019/Goal-Oriented-Action-Planning-Ten) — Chris Conway — GDC 2015 — GOAP retrospective, lessons learned across a decade of shipped titles.
- [Behavior Trees: Three Ways of Cultivating Strong AI](https://www.gdcvault.com/play/1012416/Behavior-Trees-Three-Ways-of) — GDC AI Summit — how shipped games use BTs in practice; designer-friendly authoring patterns.
- [Improving AI Decision Modeling Through Utility Theory](https://www.gdcvault.com/play/1012410/Improving-AI-Decision-Modeling-Through) — Dave Mark — response curves, population distributions, weighted-random selection. Core utility-AI reading.
- [Architecture Tricks: Managing Behaviors in Time, Space, and Depth](https://www.gdcvault.com/play/1018040/Architecture-Tricks-Managing-Behaviors-in) — Dave Mark — "infinite axis" utility system; modular reasoners. Directly relevant to a BT-with-utility-scored-selector design.
- [Deciding on an AI Architecture: Which Tool for the Job?](https://www.gdcvault.com/play/1012411/) — comparison of BT vs. utility vs. GOAP vs. HTN with trade-offs.
- [AI Behavior Editing and Debugging in The Division](https://gdcvault.com/play/1023382/AI-Behavior-Editing-and-Debugging) — Ubisoft Massive — BT structure + tooling for a real shipped game.
- [RimWorld: Contrarian, Ridiculous, and Impossible Game Design Methods](https://www.gdcvault.com/play/1024232/-RimWorld-Contrarian-Ridiculous-and) — Tynan Sylvester — GDC 2017 — story-generator framing; designing systems to surface narrative. [YouTube mirror](https://www.youtube.com/watch?v=VdqhHKjepiE).

## 2. Devblogs from shipped sports sims

- [Out of the Park Developments — blog](https://blog.ootpdevelopments.com/) — OOTP / FHM / WWB release notes + design posts; closest analogue to FW's solo-dev sim-depth ethos.
- [Solecismic Software](http://www.solecismic.com/solecismic.php) — Jim Gindin's Front Office Football homepage; version-by-version design notes for a 25-year solo-built NFL sim.
- [Sports Interactive blog (FM newsroom)](https://www.footballmanager.com/news) — official FM patch + feature blogs. Animation/match-engine writeups land here.
- [Football Manager 26 + Unity transition interview](https://store.epicgames.com/en-US/news/football-manager-26-interview) — Epic Games Store interview; SI on Hawk-Eye skeletal data integration into the animation system.
- [FOF9 scrapped — algorithmic code sale](https://gmgames.org/2021/02/20/fof9-scrapped-jim-gindin-sells-algorithmic-code-and-pursues-different-format-of-game/) — gmgames.org post on Gindin's pivot; rare candor on shipping economics of a solo sim.

## 3. Interviews with sim-game devs

- [Behind Football Manager 26's delayed release — Miles Jacobson](https://store.epicgames.com/en-US/news/football-manager-26-interview) — on the FM25 cancellation, scope creep, "diamond that needs polishing."
- [Operation Sports — Miles Jacobson opens up about cancelling FM25](https://www.operationsports.com/miles-jacobson-opens-up-about-cancelling-football-manager-25/) — frank post-mortem of a publicly visible failure mode.
- [An interview with OOTP creator Markus Heinsohn](https://blog.ootpdevelopments.com/an-interview-with-ootp-creator-markus-heinsohn/) — origin story; soccer-management games inspired his approach to baseball.
- [Dwarf Fortress' creator on how he's 42% towards simulating existence](https://www.pcgamer.com/dwarf-fortress-creator-on-how-hes-42-towards-simulating-existence/) — Tarn Adams in PC Gamer; deep-sim philosophy at its purest.
- [The Design of Dwarf Fortress — Tarn Adams (Perceptive Podcast)](https://www.youtube.com/watch?v=IDgjVRqy5IA) — long-form video interview on procedural worldbuilding.
- [Q&A: Dissecting the development of Dwarf Fortress](https://www.gamedeveloper.com/design/q-a-dissecting-the-development-of-i-dwarf-fortress-i-with-creator-tarn-adams) — Game Developer magazine; codebase + tooling angle.
- [Generating Human Stories with RimWorld Creator (AIAS Game Maker's Notebook)](https://open.spotify.com/episode/1T43NtHpjpY1BSwnapcACI) — Tynan Sylvester podcast; how the storyteller-driven AI is structured.
- [Sports Gaming Network — Jim Gindin interview](http://www.sports-gaming.com/football/f_office/interview1.shtml) — early FOF design retrospective.
- [List of Dwarf Fortress developer interviews](https://dwarffortresswiki.org/index.php/List_of_Dwarf_Fortress_developer_interviews) — community-maintained index; one-stop archive.

## 4. Academic papers — sports analytics + game AI

- [Valuing On-the-Ball Actions in Soccer: A Critical Comparison of xT and VAEP](https://tomdecroos.github.io/reports/xt_vs_vaep.pdf) — Van Roy, Robberechts, Yang, Davis, Decroos — AAAI-20 workshop. Pick-one reading for action-valuation models.
- [Interpretable Prediction of Goals in Soccer](https://ai-teamsports.weebly.com/uploads/1/2/7/0/127046800/paper12.pdf) — Decroos — xG model with interpretability constraints; closer to what a sim engine can mimic.
- [Karun Singh — Introducing Expected Threat (xT)](https://karun.in/blog/expected-threat.html) — the public-facing xT introduction; Markov-chain pitch-zone valuation. Foundational.
- [Predicting goal probabilities with improved xG models using event sequences](https://pmc.ncbi.nlm.nih.gov/articles/PMC11524524/) — PMC open-access; sequence-aware xG, useful for chance-quality modeling.
- [Soccermatics: Explaining Expected Threat](https://soccermatics.medium.com/explaining-expected-threat-cbc775d97935) — David Sumpter — readable derivation of xT math.
- [Soccermatics documentation — Expected Threat (Position-based)](https://soccermatics.readthedocs.io/en/latest/lesson4/xTPos.html) — runnable course material; the most accessible on-ramp.

## 5. Books

- *AI for Games* — Ian Millington & John Funge, 3rd ed. 2019 — the field reference. Behavior trees, decision systems, pathfinding, steering. Owns a shelf in every game-AI office.
- *Game AI Pro 1 / 2 / 3* — ed. Steve Rabin, 2013/2015/2017 — free PDFs at [gameaipro.com](https://www.gameaipro.com/). Volume 3 is the most modern. Cherry-pick chapters by topic.
- *Game AI Pro 360: Guide to Architecture* — Rabin — focused volume distilling the architecture chapters across the series.
- *Designing Games: A Guide to Engineering Experiences* — Tynan Sylvester, O'Reilly 2013 — design-philosophy book; "elegance" and emergent narrative chapters are essential. [Author page](https://tynansylvester.com/book/).
- *Football Hackers: The Science and Art of a Data Revolution* — Christoph Biermann, 2019 — the European analytics scene from a journalist embedded in it.
- *The Numbers Game: Why Everything You Know About Soccer Is Wrong* — Chris Anderson & David Sally, 2013 — the foundational popular-analytics book; weak-link vs. strong-link debate.
- *Inverting the Pyramid: The History of Football Tactics* — Jonathan Wilson, 2008 (rev. ed. 2018) — tactical history; what BT runners + signature moves should evoke.
- *Zonal Marking: The Making of Modern European Football* — Michael Cox, 2019 — national-tradition lens on tactical evolution. Useful for content-pack flavor.
- *Soccermatics: Mathematical Adventures in the Beautiful Game* — David Sumpter, 2016 — bridges football to math/stats; pairs with the Soccermatics course.

## 6. Open-source sports-sim engines + data

- [statsbomb/open-data](https://github.com/statsbomb/open-data) — free event-stream data for selected competitions (Messi La Liga, WC, Euros). JSON; well-documented schema in `doc/`.
- [statsbomb/statsbombpy](https://github.com/statsbomb/statsbombpy) — Python loader. R counterpart: [StatsBombR](https://github.com/statsbomb/StatsBombR).
- [openfootmanager/openfootmanager](https://github.com/openfootmanager/openfootmanager) — GPLv3 FM-alike using Rust (match engine) + Tauri + React. **Closest architectural sibling to Final Whistle.** Worth a deep read.
- [ZOXEXIVO/open-football](https://github.com/ZOXEXIVO/open-football) — pure-Rust FM-style sim engine with editor-oriented workflow.
- [google-research/football](https://github.com/google-research/football) — RL environment built on Gameplay Football. Reference for 22-player physics scope and observation-space design.
- [atas76/openengine](https://github.com/atas76/openengine) — small "pluggable" football match engine; readable.
- [socceraction docs — Loading StatsBomb data](https://socceraction.readthedocs.io/en/latest/documentation/data/statsbomb.html) — SPADL action-format docs + VAEP reference implementation.

## 7. Football tactics literature (non-academic)

- [Spielverlagerung.com](https://spielverlagerung.com/) — and English mirror [spielverlagerung.com/category/english/](https://spielverlagerung.com/category/english/) — German collective; the deepest free tactical analysis on the web. Pressing-trigger taxonomies, build-up structures, etc.
- [Michael Cox — Zonal Marking](https://web.archive.org/web/2020*/zonalmarking.net) (defunct) — archived original blog. Most of Cox's current writing lives at [The Athletic](https://theathletic.com/) and his books.
- [The New Statesman — Intelligent football: Michael Cox and the rise of tactical analysis](https://www.newstatesman.com/long-reads/2020/10/intelligent-football-michael-cox-and-rise-tactical-analysis) — meta-piece on the analyst class itself.
- [Jonathan Wilson at The Guardian](https://www.theguardian.com/profile/jonathanwilson) — tactical columns; The Question column archive.
- [The Coaches' Voice — tactics-explained library](https://www.coachesvoice.com/cv/analysis/) — coach-authored breakdowns of formations, transitions, set pieces.

## 8. Modding docs for shipped sims

- [FM Scout — Football Manager tutorials](https://www.fmscout.com/c-help.html) — community-maintained editor, skinning, and modding guides. Skin XML internals leak engine internals.
- [Football Manager 26 In-Game Editor guide](https://deltiasgaming.com/football-manager-26-in-game-editor-guide/) — current editor surface; what attributes/relationships are mutable tells you what the engine reads.
- [FMRTE](https://www.fmrte.com/) — third-party real-time editor; the field map across editions effectively documents the FM data model.
- [OOTP Developments forums — modding](https://forums.ootpdevelopments.com/) — official forum, mod subforums for face packs, logos, and the OOTP database editor.
- [Football Manager 22+ Steam Workshop discussions](https://steamcommunity.com/app/1569040/discussions/) — practical modding Q&A; supplements official docs.

---

## Top 10 — if you only read these

A focused two-hour reading list, prioritized across categories:

1. **[Three States and a Plan: The A.I. of F.E.A.R.](https://www.gamedevs.org/uploads/three-states-plan-ai-of-fear.pdf)** — Orkin, GDC 2006. The shortest single PDF that will change how you think about agent decision-making.
2. **[Karun Singh — Introducing Expected Threat](https://karun.in/blog/expected-threat.html)** — the analytics concept FW's "salience" naming should never collide with. Read before designing any value-of-possession heuristic.
3. **[openfootmanager](https://github.com/openfootmanager/openfootmanager) source tour** — read the match-engine crate; it's Rust + Tauri + a frontend. Direct architectural prior art.
4. **[Improving AI Decision Modeling Through Utility Theory](https://www.gdcvault.com/play/1012410/Improving-AI-Decision-Modeling-Through)** — Dave Mark. Watch once; reread the response-curve section three times.
5. **[Architecture Tricks: Managing Behaviors in Time, Space, and Depth](https://www.gdcvault.com/play/1018040/Architecture-Tricks-Managing-Behaviors-in)** — Mark again; "infinite axis utility system." This is what a 22-player BT runner with per-tick action scoring looks like at scale.
6. **[Valuing On-the-Ball Actions in Soccer (xT vs. VAEP)](https://tomdecroos.github.io/reports/xt_vs_vaep.pdf)** — single PDF; lays out the two leading possession-value frameworks. Required for any sim that wants to feel "modern."
7. **[Designing Games (book) — Tynan Sylvester](https://tynansylvester.com/book/)** — chapters on elegance and story generation. The mental model for "careers that remember."
8. **[RimWorld: Contrarian Game Design Methods](https://www.youtube.com/watch?v=VdqhHKjepiE)** — Sylvester GDC. The "we built a story generator" framing is a direct cousin to FW's pillar 2.
9. **[Game AI Pro vol. 3 — free PDFs](https://www.gameaipro.com/)** — skim TOC, pick 3 architecture chapters. (HTN intro + utility-AI deep dive + animation-driven movement are the high-leverage picks.)
10. **[statsbomb/open-data](https://github.com/statsbomb/open-data)** — the data your match-engine output should be comparable to. Inspect a few JSON event streams to see what "ground truth" looks like.

---

*Maintenance note:* this file is a starting bibliography, not exhaustive. When future research uncovers a high-signal source, append in the matching category. Don't grow the Top 10 beyond 10 — replace, don't add. Verify URL reachability on each pass.

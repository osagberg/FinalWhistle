---
description: Football rules matrix for MatchSim. Tracks real-rule intent, Phase-3 simplifications, canonical-state impact, and promotion triggers for football-law surfaces.
last_verified: 2026-04-30
status: Phase-3 guardrail spec — matrix introduced before further MatchRules / Viewer.EventBridge work; implementation changes happen through separate SPEC tasks.
---

# Football Rules Matrix — MatchSim guardrail spec

## Purpose

This spec is **not an IFAB lawbook**. It is the project's local contract for which football rules MatchSim models, which it intentionally simplifies, which it defers, and what each simplification owes in tests / promotion triggers / explicit acknowledgement.

The contract exists because rule-law simplifications were accumulating in scattered code comments inside `MatchSim/Sim/MatchRules.cs` (and the surrounding system docs) — making it difficult for any reviewer (Claude, Codex, the user, future-Claude) to answer the question "what football do we model TODAY?" without reading the whole codebase. This matrix consolidates that answer.

## Scope

- **Covers** deterministic MatchSim rule surfaces and viewer-visible restart semantics.
- **Does not implement rules by itself.** This is documentation; implementation lands through separate SPEC tasks.
- **Does not require perfect football law at Month 3.** The Month-3 gate prioritizes legible continuous play; offside / fouls / cards / subs / stoppage stay out of scope unless a new SPEC decision supersedes that.
- **Does require every deviation to be explicit before it becomes player-visible.** A simplification that nobody can see is fine. A simplification that the dots viewer or commentary surfaces is not — it needs either fix or acknowledgement.

## Locked decisions

- **Matrix before expansion.** Any new MatchRules / PitchRules rule surface MUST add or update a row in the table below before its implementation is marked done in SPEC.md. No new rule code without a matrix row.
- **Phase-3 simplifications are allowed but named.** Month-3 gate prioritizes legible continuous play; offside / fouls / cards / substitutions / stoppage stay out of scope unless a new SPEC decision supersedes this.
- **Player-visible restarts need a contract.** If the viewer can show a restart, the matrix MUST say whether gameplay authority matches the displayed restart, or whether the displayed side is a placeholder ahead of last-touched tracking.
- **Canonical impact is explicit.** Rows mark whether a rule changes `MatchCanonicalState`, `KeyEvent`, replay hashes, or is presentation-only.
- **Football lines are treated deliberately.** The matrix must state whether a rule uses whole-ball-over-line, ball-center approximation, or some other deterministic simplification.
- **Promotion triggers are binding.** When a trigger lands, the simplification cannot remain hidden in code comments — it becomes a Phase task or ADR/spec update.

## Matrix

Legend for **Canonical impact** column:
- **Score** — mutates `MatchSimulationState.HomeScore` / `AwayScore`.
- **Restart-state** — mutates `MatchSimulationState.OutOfPlay` (transient per-tick flag).
- **KeyEvent** — appends a row to `MatchSimulationState.KeyEvents`.
- **Ball-position** — respawns the ball at a deterministic restart spot.
- **Replay-hash** — anything mutating canonical state is in the deterministic 60-tick / corpus replay hash by definition.
- **Presentation-only** — viewer-visible but not in canonical state.
- **None** — currently unmodeled.

| Rule surface | Real-football intent | Phase-3 behavior | Simplification / deviation | Player-visible risk | Canonical impact | Tests owed | Promotion trigger |
|---|---|---|---|---|---|---|---|
| **Goal detection** | Goal awarded only when whole ball crosses goal line between posts and under crossbar. | Q32.32 line crossing on goal-plane; ball-center approximation (no ball radius); checks `Y < 2.44` (crossbar) and `|Z| < 3.66` (post half-width); strict `<` on Z so on-the-post is OUT. | No whole-ball-radius geometry; on-the-line / on-the-post defaults to OUT; no goal-line technology drama. | Medium — visible on close calls in dots viewer. | Score, KeyEvent.Goal, Ball-position (respawn at center), Replay-hash. | Goals positive/negative side ✅; above crossbar → GoalKick ✅; outside posts → GoalKick ✅; ON the post / ON the crossbar boundary case (currently OUT by strict `<`) — pin policy. | Before viewer ships close-up goal-line shots; before public EA. |
| **Touchline out** | Goal awarded only when whole ball crosses touchline. | Q32.32 line crossing on Z-plane; ball-center approximation; strict `<` on Z so on-the-line is OUT. | No ball-radius geometry; exact-line policy is "center past line." | Medium — viewer shows ball near line. | Restart-state (`OutOfPlay.ThrowIn`), KeyEvent.ThrowInRestart, Ball-position, Replay-hash. | Both touchlines ✅; diagonal earliest-crossing ✅; sub-ULP earliest-crossing both corners ✅; exact-on-line policy pin. | Before dots viewer shows line-close restarts as gameplay-meaningful. |
| **Goal-line out (goal kick vs corner)** | Last touch decides goal kick vs corner. | All non-goal goal-line crossings classify as `OutOfPlay.GoalKick` + `KeyEventKind.GoalKickRestart`. Restart side = defending team (`post.X > 0 ? Away : Home`). | No last-touched tracking → no corner kicks. | High — if viewer ever labels these as corners, the label is wrong. Currently no viewer commentary distinguishes. | Restart-state, KeyEvent.GoalKickRestart, Ball-position, Replay-hash. | Above crossbar ✅; wide of post ✅; both ends ✅; diagonal touchline-first → ThrowIn ✅; sub-ULP corner → goal-line tie-break ✅. Future: last-touch attacker → CornerKick. | When possession / last-touched tracking lands (Phase 4); before any Phase-4 set-piece signature relies on corner context. |
| **Corner kick** | Attacking restart from corner when defender last touched ball over own goal line. | `KeyEventKind.CornerKickRestart` enum value reserved (pinned byte=4) but never emitted. `OutOfPlay.CornerKick` enum reserved similarly. | Deferred — no active corner classification; no taker behavior; no curved trajectory authoring. | Medium / high once matches produce obvious deflections that should clearly be corners. | None until activated; future KeyEvent.CornerKickRestart + Restart-state. | None active; future last-touch + corner classification tests. | Phase 4 fouls + set pieces / signatures requiring set-piece context. |
| **Throw-in restart side** | Opposite team from last toucher restarts. | **Hardcoded `restartSide = TeamSide.Home`** in `HandleTouchlineCrossing` (`MatchRules.cs:547`). KeyEvent records `Side: Home` regardless of which team's possession it actually was. | Placeholder — no last-touched tracking. The recorded side is **informational only** per `MatchSimulationRunner` doc-lock; the runner does NOT consume it on the next tick. | High if overlay / commentary ever says "Home throw" — it would be wrong half the time. Mitigated currently because Phase-3 has no commentary surface for restart-side. | KeyEvent.Side (=Home), Restart-state, Ball-position, Replay-hash. | Current placeholder pinned via tests asserting `Side == Home`; future last-touch-driven side tests. | Before viewer displays restart side, OR when possession tracking lands (Phase 4). |
| **Restart gameplay authority** | Restart controls possession; play resumes from a legal dead-ball restart taken by a player. | **Event-only placeholders.** `MatchRules.Step` emits the restart KeyEvent and respawns the ball at zero velocity at the canonical restart spot. `MatchSimulationRunner` does NOT consume any restart-control state on the next tick — both BTs run normally; nearest-player heuristic decides who picks the ball up. | No restart taker, no dead-ball state, no possession handoff, no BT suppression, no "restart-walk-to-spot" choreography. | High if dots viewer shows a restart and the player expects authoritative possession. Phase-3 viewer is not yet authoring restart presentation, so risk is latent. | Ball-position (respawn), Restart-state (per-tick flag), KeyEvent. **No** possession-lock state. | Document the event-only contract via doc-comments ✅; future possession-authority tests when Phase 4 lands. | Before dots viewer ships restart presentation, OR before any Phase-4 set-piece signature relies on a deterministic restart taker. |
| **Kickoff / post-goal restart** | Kickoff from center by team that conceded; restart authority sits with the conceding side. | Immediate center respawn at zero velocity (`MatchRules.cs:497`); no `KickOff` enum state in `OutOfPlay`; `OutOfPlay` stays `InPlay` after a goal. | No dead-ball / restart taker / possession flip. The 1-tick transition between goal-detected and ball-back-at-center is implicit. | Medium — Month-3 observers may flag absence of a celebration / kickoff rhythm. | Score (mutated by goal), Ball-position (respawn at center), KeyEvent.Goal. | Goal resets ball center + score ✅; goal at both ends ✅. | Before match viewer shows post-goal sequence; if Month-3 observers report "feels like the ball just teleports back to center, weird," add a `KickOff` enum value via new SPEC entry. |
| **Offside** | Offside position + offence on active involvement (IFAB Law 11). | **Not modeled.** | Explicit omission per 2026-04-24 match-engine resolution + 2026-04-28 PitchRules decisions-log entry. | Medium / high for through-ball-heavy tactics — direct-running archetypes will be artificially over-effective. | None. | None until introduced. | Before direct-running tactics become public balance surface; OR Phase 4+ tactical exploit testing flags through-balls as broken. |
| **Fouls** | Illegal contact / handling / dangerous play (IFAB Law 12). | **Not modeled.** | Explicit omission per 2026-04-24 match-engine resolution + 2026-04-28 PitchRules decisions-log entry. | Medium — tactics can't model "kick the playmaker" or "tactical foul" archetypes. | None. | None until introduced. | Phase 4 per match-engine introduction order. |
| **Cards** | Yellow / red disciplinary state, suspensions across matches. | **Not modeled.** | Explicit omission. | Low at Month-3 (3-minute slice); higher for rivalry / pressure / suspension-arc memory stories Phase 4+. | None. | None until introduced. | After fouls are introduced. |
| **Substitutions** | Manager swaps players within competition rules; in-match tactical lever. | **Not modeled.** All 22 starters stay on pitch for the entire match. | Explicit omission. | Low for 3-minute Month-3 slice. | None. | None until introduced; future roster / lineup mutation tests. | Phase 4 / 5 manager-decision ledger work. |
| **Injuries** | Player availability / forced sub / stoppage. | **Not modeled.** | Explicit omission. | Low at Month-3. Higher post-EA when career memory wants "the keeper cried at full-time after his cup-final injury" beats. | None. | None until introduced; future injury event + save-persistence tests. | After substitution framework. |
| **Stoppage time** | Added time for delays. | **Not modeled.** Match length is fixed. | Explicit omission. | Low at Month-3. | Match length fixed; no `Tick`-extending behavior. | None until introduced; future clock / stoppage tests. | After fouls / cards / subs / injuries. |
| **Advantage** | Referee allows play after a foul when beneficial to the fouled side. | **Not modeled** (because fouls aren't modeled). | Explicit omission. | Low until fouls exist. | None. | None until fouls. | After basic foul system. |
| **Penalties / direct & indirect free kicks** | Dead-ball restarts for offences. | **Not modeled.** | Explicit omission. | Low until fouls exist. | None. | None until fouls. | Phase 4 set-piece / signature work. |
| **Goalkeeper handling** | Goalkeeper may handle the ball within own penalty area, with restrictions (back-pass, 6-second rule, etc.). | **Not modeled.** Phase-3 has no GK-specific role differentiation in `PlayerActuator`; all 22 players obey identical kinematics. | Explicit omission / placeholder. | Medium once viewer shows keeper actions or shot-stops. | None until introduced; future GK possession + set-piece tests. | Before keeper-specific signatures (`#7` / `#9` etc. catalog entries that involve the keeper) or viewer keeper close-ups. |

## Current Phase-3 watchlist

Items currently within MatchRules / PitchRules scope where reviewer attention should converge during Phase-3 implementation:

- **Exact-line boundary policy must be pinned in code AND tests.** `IsInField` strict-`<` is documented but the on-the-line test case is not pinned in `MatchRulesTests`. Add an explicit fixture asserting "ball at exactly `|X| == GoalLineX` is treated as OUT under Phase-3 strict-inequality policy."
- **Earliest-crossing must compare true crossing order, not Q32.32-rounded fractions.** Closed in commit `a6506dd` via `BigInteger` cross-multiplication + sub-ULP regression test (both `+X/+Z` and `-X/+Z` corners). Future addition: `+X/-Z` and `-X/-Z` mirrors if any reviewer doubts symmetry.
- **Throw-in restart side must not be presented as authoritative.** Currently hardcoded `Home`; doc-lock prevents misuse. Any viewer overlay that surfaces `KeyEvent.Side` for `ThrowInRestart` events MUST show a Phase-3 placeholder marker, not a confident "Home throws in."
- **Restart events must not imply gameplay authority.** Documented in `MatchRules` + `MatchSimulationRunner` doc-comments as event-only placeholders. Any new code that consumes restart-state on the next tick triggers a matrix update + new SPEC task.
- **Corner enum / event reservations must not imply active corner behavior.** `KeyEventKind.CornerKickRestart` byte=4 is reserved; do NOT emit it in Phase 3. The matrix row above is the binding contract.

## Test policy

- **Every active Phase-3 row with canonical impact must have at least one xUnit test in `MatchSim.Tests/Sim/MatchRulesTests.cs`** OR carry an explicit Phase-4+ deferral note in the matrix.
- **Future rule promotions require regression tests before SPEC.md / STATUS.md can mark the promotion done.** New rule + new test, in the same commit.
- **Golden replay fixtures** (`MatchSim.Tests/fixtures/replay-corpus/<seed>.json` per `golden-replay-corpus.md`) update only when canonical behavior intentionally changes. The pinned 60-tick smoke hash re-baselined v0 → v1 when the PitchRules layer landed; that is the model — re-baseline is intentional, traceable in the test file's doc-comment, and acknowledged in CHANGELOG.

## Relationship to existing docs

- [`design/match-engine.md`](../match-engine.md) — high-level MatchSim architecture; this matrix is the rule-law sub-contract.
- [`design/month-3-vertical-slice.md`](../month-3-vertical-slice.md) — Month-3 gate locked the "legible continuous play" priority that justifies the deferrals here.
- [`design/specs/golden-replay-corpus.md`](golden-replay-corpus.md) — corpus fixtures are how this matrix's "canonical impact" claims get verified across Win/Mac/Linux.
- [`SPEC.md` decisions log](../../SPEC.md) — the 2026-04-28 PitchRules entry + 2026-04-30 round-4 hardening entry + the 2026-04-30 entry that introduces this matrix are the primary source of authority. If any matrix row contradicts a later SPEC decision, the SPEC decision wins and this matrix gets an update commit.
- `MatchSim/Sim/MatchRules.cs` — implementation, NOT source of truth. The doc-comments in that file are sub-contracts; this matrix is the umbrella.

## Changelog

- **2026-04-30** — Initial matrix introduced after Codex round-4 review identified that MatchRules simplifications had accumulated in scattered code comments rather than a single football-law contract. Seeds 16 rule-surface rows covering goals, touchlines, goal kicks, corners, throw-ins, restart authority, kickoffs, offside, fouls, cards, substitutions, injuries, stoppage, advantage, penalties / free kicks, and goalkeeper handling.

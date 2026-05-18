# Post-T2 Ultimate Review — 2026-05-18

> Multi-track adversarial review at the Phase T2 → T3 boundary. Per the /done
> skill Step 5.5: dispatched when a phase ships ≥5 commits. T2 shipped 10.
>
> **Phase scope reviewed:** T2 commits `b2d41f9b..26540dcd` (10 commits;
> +9844 / -276 LoC across 77 files; 90+ new tests). All 10 T2 MVP rows
> landed; 3 rolled to T3 (T2-4 PlayerBio + T2-7 Squad blocked on missing
> design doc; T2-1d2 utility_shoot deferred per "wait for BT to mature").
>
> **Track split rationale** (per `/done` Step 5.5): Claude is strong on
> implementation-drift / test-quality / systemic-pattern detection; Codex is
> strong on adversarial red-team + property explosion. Combined ≈ 2× the
> findings of either alone (per the 2026-05-16 audit convergence patterns).
>
> **Subagent discipline**: every Claude track is read-only. No file edits
> outside this shared review file. No git commits. Findings land here +
> the main thread consolidates into a verdict section at the end.

---

## Track A — Mutation-test analysis (Claude `feature-dev:code-explorer`)

*Lens: "If I mutated the constants / flipped the team / removed the guard / always returned default, would a test fail?" Identify MUTATION SURVIVORS across the T2 diff.*

**Owner:** Claude `feature-dev:code-explorer`
**Status:** Complete — 2026-05-18

### Summary

Tests reviewed: ~65 test functions across 9 new T2 test files + inline `#[cfg(test)]` modules in `fw-save`, `fw-content-baker/validators`, `fw-tauri/commands`. Mutation survivors found: **6**.

**Worst pattern**: vacuous named-constant assertions — the constant is read on BOTH sides of the assert at test time, so mutating the constant in production keeps both sides equal and the test passes silently. Found across `league_generation_test.rs`, `season_state_test.rs`, `season_commands_test.rs`. The `total_fixture_count_in_league_is_three_hundred_eighty` test pins `MATCHES_PER_SEASON == 380` as a hard literal, but equivalent pins are MISSING for 10/day + 38/club.

**Verdict: concerning in T2-2 / T2-5 test suites; clean in sim, save, and frontend.** Sim tests (tactic_event_emission, calibrate_smoke, goal-tick regression) are among the strongest in the codebase. Vacuous-constant pattern is T2-2/T2-5 specific and warrants a pre-T3 cleanup row.

### Findings

- **A-1** (severity: pre-T3) — `crates/fw-content/tests/league_generation_test.rs:37–48` `generate_league_produces_20_clubs_and_380_fixtures` — mutation: `CLUBS_PER_LEAGUE` 20→N, both sides shift together. Fix: `assert_eq!(CLUBS_PER_LEAGUE, 20, "sanity pin");` + pin `MATCH_DAYS_PER_SEASON` to 38.
- **A-2** (severity: pre-T3) — `crates/fw-content/tests/season_state_test.rs:102–113` `fixtures_for_match_day_returns_ten_per_day` — mutation: `CLUBS_PER_LEAGUE` 20→16 → 8/day; assert checks 8==8. Fix: `assert_eq!(CLUBS_PER_LEAGUE / 2, 10, "sanity pin");`
- **A-3** (severity: pre-T3) — `crates/fw-tauri/tests/season_commands_test.rs:258–263` `get_fixtures_returns_38_for_valid_club` — same shift-together pattern. Fix: replace `(CLUBS_PER_LEAGUE-1)*2` with literal `38`. (The hard-coded `19` mirror asserts at lines 338–339 ARE non-vacuous.)
- **A-4** (severity: pre-T3) — `crates/fw-tauri/tests/season_commands_test.rs:67–80` `advance_week_returns_season_complete_after_final_day` — `MATCH_DAYS_PER_SEASON` shift-together across loop bound + assert + `is_complete()`. Fix: direct literal `assert_eq!(MATCH_DAYS_PER_SEASON, 38, "sanity pin");`
- **A-5** (severity: opportunistic) — `crates/fw-tauri/tests/season_commands_test.rs:210–215` `get_standings_points_tally_after_full_season` — range `[760, 1140]` too permissive; off-by-one scoring (win=2) survives. Unit tests in `season_state_test.rs` (win_awards_three_points / draw_awards_one_point_each) are the strong discriminators.
- **A-6** (severity: opportunistic) — `crates/fw-match-sim/tests/calibrate_smoke_test.rs:98–103` — `assert total_shots >= 1` trivially weak for 5×600-tick run. Fix: bump to `>= 10`.

### Tests that ARE genuinely non-vacuous (notable strengths)

- `ball_past_sideline_with_home_last_touched_emits_throw_in_for_away` + mirror — full team-side discriminator coverage.
- `goal_tick_skips_dispatch_so_kickoff_taker_decisions_dont_override_midblock` — targets 3 specific if-guards with named test doc.
- `v0_envelope_wire_first_byte_is_locked_at_0x00` + `v1_..._0x01` — exact wire bytes pinned.
- `transfer-window.test.ts` boundary tests — days 0/1/18/19/20/21/38/39 + 4 RangeError guards.
- `standings_sort_order_is_points_desc_then_gd_desc_then_gf_desc_then_club_id_asc` — full tiebreaker-chain exercise.
- `forward-incompat-failure` — pattern-matches BOTH "99" AND "variant" (correctly tight).

---

## Track B — Architectural drift (Claude `pr-review-toolkit:code-reviewer`)

*Lens: docs vs code, both directions. Where does code drift from the design doc that authored it? Where does a design doc lie about what the code actually does? Where does a comment claim an invariant the code doesn't enforce?*

**Owner:** Claude `pr-review-toolkit:code-reviewer`
**Status:** complete

### Summary

8 drifts found across the T2 commit range, broken down by class:

| Class | Count | Worst |
|---|---|---|
| doc-vs-code (doc lies about what code does) | 4 | B-1, B-2 |
| code-vs-doc (code extended past spec without doc update) | 3 | B-4 |
| cross-rule (CLAUDE.md / RULES.md inconsistency) | 1 | B-5 |
| adr-non-honor | 0 | — |

**Worst pattern:** the `docs/specs/tactic-fsm.md` transition table promises three event-driven transitions (`PressTimeoutExpired`, `CounterWindowClosed`, `HalfTime`) that **no production call site ever emits**. The implementation (`tactic_fsm.rs`) carries internal-only TODO comments acknowledging this, but the spec presents them as live transitions with concrete time-budgets ("5s after entry", "4s after entry") that don't fire. Anyone reading the spec to understand what tactic-FSM behaviour to expect will be misled. T2-1b shipped `PossessionLost` / `BallRecovered` / `BallOutOfPlay` / `BallInPlay` emissions; the three timer-derived events stayed deferred without the spec being updated to call that out.

**Verdict: concerning.** No gate-blockers — every drift is recoverable by an honesty patch to the doc OR a code wire-up. The pattern is "design docs promise more than the implementation delivers"; left un-addressed, this erodes the spec-as-contract discipline that makes ADR + design-doc review meaningful at phase boundaries. Three findings (B-1, B-2, B-4) are pre-T3 priority because they touch surfaces the next phase will extend; the remaining five are opportunistic.

### Findings

- **B-1** (severity: pre-T3) — drift type: doc-vs-code
  - Where: `docs/specs/tactic-fsm.md:81-84` + `crates/fw-match-sim/src/lib.rs` (production tick_match — no emission sites)
  - What the code does: `TacticEvent::PressTimeoutExpired` and `TacticEvent::CounterWindowClosed` are only referenced inside `crates/fw-match-sim/src/tactic_fsm.rs::tests` (lines 758-784). No production code in `lib.rs`, `dispatch.rs`, or elsewhere emits these events. The `apply_event` arms exist (lines 450-465) but are dead-code in production.
  - What the doc says: The transition table in `tactic-fsm.md:81-82` lists `PressTimeoutExpired` as the canonical exit from HighPress after `5s` and `CounterWindowClosed` as the canonical exit from CounterAttack after `4s OR shot taken`. The spec presents these as live behavior the FSM exhibits.
  - Severity rationale: The HighPress state is real (T2-1b wired `PossessionLost` → HighPress transitions), but it can ONLY exit via the heartbeat-timeout drift check at >600 ticks (10 seconds), NOT the spec-promised 5s. Anyone reading the spec to tune HighPress duration will tune the wrong knob. The CounterAttack state is reachable via `BallRecovered`, never exits until the next `BallOutOfPlay` / `Goal` / `HalfTime` (which also never fires). The deferral is documented inside `tactic_fsm.rs:294-300` as a "T1-4 reconciliation TODO" but the spec was never updated to mark these transitions deferred.
  - Suggested fix: add a "Deferred to T3" caveat block in `tactic-fsm.md` ahead of the transition table, listing the three deferred event classes + linking the `tactic_fsm.rs:294-300` TODO. Same fix or implementation row gets the timer events wired.

- **B-2** (severity: pre-T3) — drift type: doc-vs-code
  - Where: `docs/specs/tactic-fsm.md:85` + `crates/fw-match-sim/src/lib.rs` (no HalfTime emission)
  - What the code does: `MatchEvent::HalfTime` does not exist in `crates/fw-content/src/event.rs` (no enum variant). `TacticEvent::HalfTime` exists in `tactic_fsm.rs:335` and the `apply_event` arm at line 471 resets to MidBlock — but nothing ever emits it in production. Match-time is not even tracked relative to half-time (sim runs N ticks regardless of half boundary).
  - What the doc says: Spec line 85 lists `MatchEvent::HalfTime` → `MidBlock` (always) as a top-level transition. Spec also says "state resets at the break" as if break-time has semantics.
  - Severity rationale: There is no half-time semantic in the sim at all. The spec wires half-time-reset into the tactic FSM, but the upstream MatchEvent doesn't exist. T3+ work on match-time clock will need this; today the spec promises behavior the codebase has no representation for.
  - Suggested fix: same as B-1 — mark "Deferred to T3" in tactic-fsm.md, AND note that `MatchEvent::HalfTime` itself doesn't exist yet so this is a two-level deferral. Cross-link the deferred MatchEvent → tactic event chain.

- **B-3** (severity: pre-T3) — drift type: doc-vs-code
  - Where: `docs/specs/tactic-fsm.md:96-131` (heartbeat drift rules) + `crates/fw-match-sim/src/tactic_fsm.rs:497-509` (heartbeat_check)
  - What the code does: `heartbeat_check` implements ONLY the "HighPress > 600 ticks → MidBlock" rule. Other rules in the spec (MidBlock + scoreline-lead-2 + own_mean_x < 30 → LowBlock; archetype-conditioned drift rules) are absent.
  - What the doc says: Lines 109-129 present TWO concrete drift rules (HighPress timeout + MidBlock-deep-with-lead → LowBlock), plus "... archetype-conditioned drift rules, authored in `docs/design/tactic-fsm-heartbeat-rules.md` (Phase 1 tuning doc)".
  - Severity rationale: The referenced `docs/design/tactic-fsm-heartbeat-rules.md` does not exist in the repo. The spatial drift rule (`own_mean_x < 30`) is in the spec as if implemented. The implementation explicitly says "T1-2b-ii implements only the HighPress-timeout-10s rule. Spatial drift rules ... defer to T1-2b-iii when spatial state is available" — but spatial state exists now (T1-2b-iii-a landed `PlayerState.pos_x` reads everywhere) and the rule still isn't wired.
  - Suggested fix: Either author `docs/design/tactic-fsm-heartbeat-rules.md` + implement the rules, OR update the spec to say "spatial drift rules deferred indefinitely; only HighPress timeout fires today." The undocumented gap is the drift.

- **B-4** (severity: pre-T3) — drift type: code-vs-doc
  - Where: `crates/fw-match-sim/src/bt/personality_bias.rs:104-152` (K_1..K_21 constants) + `docs/design/personality-bias-weights.md:32-47` (7×8 mapping table)
  - What the code does: Defines K_1 through K_21 (21 multiplicative coefficient constants) across the 7 considerations. K_15..K_21 are real match-tick bias factors used by `apply_cross_bias`, `apply_lay_off_bias`, `apply_mark_bias`, `apply_run_off_ball_bias`, `apply_hold_formation_bias`, and the audacious-shot `K_18` for Shoot.
  - What the doc says: Lists "k₁..k₁₄" only in the heading + section §3 ("7 × 8 mapping table"). Only the 2026-05-17 T2-1d-infra block mentions K_18 in passing as one of "5 most-load-bearing shoot+dribble-bias K constants". K_15, K_16, K_17, K_19, K_20, K_21 are not listed anywhere in the design doc, despite being live coefficients in the sim.
  - Severity rationale: Anyone tuning personality biases from this doc will miss 6 of the 21 actual coefficient sites. T1-2b-fix added K_15..K_21 to fix bt-attribute-binding drift but the design-doc tuning surface didn't update — so the doc undersells the actual coefficient surface by ~30%. The T2-1d calibration tooling proposes fits for K_18 (in scope) but the other 5 unlisted constants stay invisible to the design-doc-driven re-fit cadence.
  - Suggested fix: Extend the §3 mapping table to a full 7 × 11+ shape covering K_15..K_21, or add a §3a "Per-site P1-5 helpers" subsection mirroring the inline doc comment at `personality_bias.rs:90-97`. Make the doc the source-of-truth surface a re-fit author would consult.

- **B-5** (severity: opportunistic) — drift type: cross-rule
  - Where: `crates/fw-tauri/src/commands.rs:134` + `crates/fw-tauri/src/commands.rs:325` (`debug_assert!` usage) + `.claude/rules/Sim/RULES.md §11` (debug_assert ban for invariants)
  - What the code does: Two `debug_assert!` calls in `commands.rs` — one pinning `tick_count <= MAX_FRAMES_PER_REQUEST` after the loud guard (line 134), one pinning `current >= 1` after the loop (line 325). Both have inline comments justifying their use as "sanity checks after the real guard already fired."
  - What the rule says: Sim/RULES.md §11 explicitly bans `debug_assert!` for canonical-state OR gameplay-truth invariants. These two sites are in `fw-tauri` (NOT a sim crate per the rule's path list), and the comments are honest about the rationale ("guarantee" already enforced by the explicit guard above). Strictly the rule doesn't bind here.
  - Severity rationale: This is opportunistic only — the use is legitimate per the letter of the rule. But the precedent is risky: silent-failure-hunter sweeps will likely flag this, and the next reviewer who applies the rule strictly across the workspace (rather than respecting the path scope) could get confused. A one-line comment per site noting "fw-tauri is outside Sim/RULES.md §11's path scope; debug_assert OK here because the guard above provides the load-bearing check" would close the loophole.
  - Suggested fix: Either add the explicit "outside Sim/RULES.md scope" justification to both debug_assert call sites, or promote them to `assert!` for consistency across the workspace.

- **B-6** (severity: opportunistic) — drift type: doc-vs-code
  - Where: `crates/fw-tauri/src/season.rs:22-28` (`SEASON_MATCH_TICK_BUDGET = 600`) + `docs/DESIGN_DOC.md §5` (match engine integration)
  - What the code does: Each season match runs `tick_match` for exactly 600 ticks. At the 60 Hz integration rate (per `docs/specs/decision-cadence-stagger.md:19`), 600 ticks = 10 seconds of in-match time. The code's inline comment is honest: "Real 90-minute match realism is deferred to later work."
  - What the doc says: Nothing in `docs/DESIGN_DOC.md` or `docs/MASTER_PLAN.md` mentions that season matches are 10-second proxies for full matches. The T2-5 row in MASTER_PLAN says "season-controller IPC + SeasonState + fast-forward perf" — implying full match simulation. Standings tables, fixture lists, and the "Advance Week" UI surface in T2-6 imply players are watching a full season simulate.
  - Severity rationale: The implementation comment is candid, but the architectural surface (Tauri commands, DTOs, frontend League page) presents the result as a full simulated season. A reviewer reading MASTER_PLAN T2-5 + the league.tsx UI without diving into `season.rs:28` will believe more is happening than is. The 10-second-per-match shortcut is a load-bearing simplification that earns a top-level note in DESIGN_DOC §5 OR MASTER_PLAN T2-5 "Intentionally deferred" block.
  - Suggested fix: One-line note in `docs/MASTER_PLAN.md` T2-5 description: "Per-match tick budget is 600 (10s @ 60 Hz); full 90-minute match-time scaling deferred to T3+." Same note as a known-shortcut callout in DESIGN_DOC §5's match-engine paragraph.

- **B-7** (severity: opportunistic) — drift type: doc-vs-code
  - Where: `crates/fw-content/src/league.rs:128-222` (`generate_fixtures`) + `docs/specs/determinism-gate.md` (canonical iteration discipline)
  - What the code does: `generate_fixtures` takes a `_seed: Seed` parameter that is unused (prefixed with underscore). Comment says "reserved for future fixture-order randomization (e.g. shuffling match-day order while preserving the pair-coverage invariant); for T2-2 MVP the schedule is fixed by the circle-method algorithm alone."
  - What the doc says: No design doc covers the fixture generation algorithm. ADR-0009 SeedLayer enumeration includes `ContentBake` as the layer for any content-generation operation; the unused seed in `generate_fixtures` is correctly reserved against that layer per `state.rs:148-155`.
  - Severity rationale: Minor — an unused parameter with a clear comment explaining its reservation isn't a drift per se, but it's the kind of "TODO baked into the API" that earns either a tracking task or an explicit "yes this stays unused indefinitely" note. If the season schedule never shuffles (e.g. real football is fixed by FA fixture computer; only EPL kickoff times shuffle), the seed should be removed; if it'll be wired in T3, this becomes documented. Today it's neither.
  - Suggested fix: Either remove the `_seed` parameter (callers already supply it and removal is a 3-line change) OR add a tracked row to MASTER_PLAN's T3 section for "Wire fixture-shuffle to ContentBake seed (lite anti-replay defense)".

- **B-8** (severity: opportunistic) — drift type: doc-vs-code
  - Where: `crates/fw-content/src/league.rs:308` (`format!("fwh.core:club_{:05}", ...)`) + `.claude/rules/Content/RULES.md §2` (ID format rules)
  - What the code does: Procedurally-generated club IDs use the underscore-with-5-digit pattern (`fwh.core:club_00001`). Per the Content/RULES.md §2 carve-out, this is the correct "default form for procedural / generated entities" — distinct from the dotted-slug form (`fwh.core:culture.anglo`) used for hand-authored entities.
  - What the doc says: Content/RULES.md §2 lists `player`, `club`, `culture`, `archetype`, `competition` as the valid entity-type prefixes. The 5-digit zero-padded form is explicitly described as the default. The procedural club generation correctly follows this.
  - Severity rationale: Not actually a drift — this finding is a NULL result for the lens. Including for completeness. The club ID format complies with Content/RULES.md §2; the dotted form used for hand-authored archetypes also complies via the explicit carve-out at the bottom of §2. Both forms coexist correctly.
  - Suggested fix: None — this is honesty noise. Removed from the headline drift count above.

### Negative results (lenses checked, no drift found)

- **DTO camelCase compliance (Tauri/RULES.md §3):** all 7 T2-5 / T2-6 / T2-8 DTOs in `crates/fw-tauri/src/lib.rs` carry `#[serde(rename_all = "camelCase")]`. IpcError uses `#[serde(tag = "kind", rename_all = "camelCase")]`. Frontend `types.ts` mirrors with camelCase fields. No drift.
- **Sim/RULES.md §1 (no f32/f64 in canonical state):** zero hits for `f32` / `f64` in `crates/fw-content/src/league.rs`, `crates/fw-content-baker/src/bake.rs`, `crates/fw-match-sim/src/tactic_fsm.rs`. The calibrate binary (`crates/fw-match-sim/src/bin/calibrate.rs`) uses f64 for the Newton-Raphson fit — explicitly allowed via the bin-target opt-out, with the rationale documented at line 138.
- **Sim/RULES.md §2 (no HashMap):** zero hits for `HashMap` / `HashSet` in T2-touched sim/content code. `SeasonState.results` is `BTreeMap<(ClubId, ClubId), MatchOutcome>` (deterministic iteration).
- **ADR-0009 (SeedLayer):** `fixture_seed` (`state.rs:148-155`) uses `SeedLayer::ContentBake` with `site = 1` to distinguish from other ContentBake uses. Disambiguator naming is undocumented (no `rng-seed-sites.md` entry for `site = 1`), but the discriminant is non-overlapping per ADR-0009's "non-overlapping layers" contract. Compliant.
- **ADR-0012 (rebaseline triggers):** the 4 T2 rebaselines (T2-1a schema, T2-1b behavior, T2-1-codex-fix behavior, T2-9 save schema not a canonical hash) all carry explicit trigger-N citations in the canonical_hash.rs comment block AND the commit body markers. Rebaseline discipline holds.
- **Tauri/RULES.md §2 (UI never drives canonical state):** the `advance_week_inner` + `play_fixtures_inner` mutate `SeasonState.results` + `current_match_day`, NOT canonical match state. Each match generates a fresh `MatchState` via `MatchState::initial_with_content`. The `RwLock<SeasonState>` is on the IPC side of the boundary; canonical sim state stays immutable per tick. Compliant.

---

## Track C — Whole-codebase silent-failure sweep (Claude `pr-review-toolkit:silent-failure-hunter`)

*Lens: NOT commit-diff-scoped — walk the whole codebase looking for silent-failure surfaces, including ones that existed pre-T2 but weren't caught. Especially: `unwrap_or_default`, `if let Err(_) {}`, `Result` swallowing, `saturating_*` on sim-bearing fields without justification, `debug_assert!` for canonical invariants.*

**Owner:** Claude `pr-review-toolkit:silent-failure-hunter`
**Status:** complete

### Summary

**Total findings: 8.** Severity breakdown: 2 gate-blocker, 3 pre-T3-recommended, 3 opportunistic.

**Top 3 most concerning patterns:**

1. **Match.tsx still ships the runtime-garbage `return _exhaustive` pattern at three sites** — League.tsx fixed its single occurrence at T2-6 (throws on a future variant drift); Match.tsx's three sister sites still evaluate the `never`-typed binding at runtime and return it from `formatIpcError` / `eventLabel` / `badgeClass`. The compile-time guard is meaningful but the runtime fallback is literally the silent-failure pattern this entire `_exhaustive: never` ritual was designed to prevent. T2-6's deferred P1 has now hardened into a recurring debt; it must land before T3.
2. **All four T2-5 season IPC commands bypass `safeInvoke` / runtime shape validation** — `advanceWeek`, `playFixtures`, `getStandings`, `getFixtures` in `frontend/src/lib/api/season.ts` call raw `invoke<T>()`, casting the response to the TS DTO type without checking. This is the exact regression the T1-3.6 `runtime-validators.ts` landed to prevent: backend wire-shape drift on any of these four DTOs will silently land in TS and NPE deep in League.tsx / Transfers.tsx. The implicit contract from T1-3.6 was "every new DTO requires a guard update"; the four T2-5 DTOs have no guards at all.
3. **`debug_assert!` for gameplay-bearing invariants in `bt/personality_bias.rs`** — 13 sites assert `raw ∈ [0,1]` / `pressure ∈ [0,1]` preconditions on functions that multiply raw utility by personality factors and feed the result into utility-scored BT leaves (canonical-decision-bearing). Release builds skip the assert; out-of-range raw passes through silently as a polluted Q32 utility. Sibling `signature/bias_apply.rs:88` uses `assert!` (release-active) for the same invariant class — proving the project knows the pattern; personality_bias.rs is the outlier. Exactly the Sim/RULES.md §11 P2 pattern from the T1 ultimate-review.

**Verdict: concerning.** The Match.tsx P1 deferral has now hardened into a recurring gap (League.tsx fixed it, Match.tsx didn't). The season-IPC runtime-validation skip undoes the T1-3.6 audit response one phase later. The personality_bias.rs `debug_assert!` issue mirrors the exact pattern that drove Sim/RULES.md §11's authoring six weeks ago. Three independent surfaces that "the project already knew" — each one a silent-failure surface in production-load-bearing code. Track this as a process-discipline signal, not just three bugs.

### Findings

- **C-1** (severity: gate-blocker) — silent-failure class: runtime-garbage-return-on-exhaustive-default
  - File: `frontend/src/routes/Match.tsx:113-114`, `161-163`, `183-185`
  - Pattern: `default: { const _exhaustive: never = err; return _exhaustive; }`
  - Why silent: TypeScript's `never` is a compile-time-only erasure. At runtime, `_exhaustive` IS the original `err` / `kind` value. If a future IpcError variant lands and the runtime guard's `KNOWN_IPC_ERROR_KINDS` is updated but the switch isn't (the compile guard fires), one is caught. BUT in `eventLabel` / `badgeClass` the input is `MatchEventKind` from a typed payload — if the backend sends an unknown kind that runtime-validators is updated to accept but the switch isn't, the function returns the raw kind string from `eventLabel` (rendered as a JSX child — works but skips the friendly label) and returns the raw kind string from `badgeClass` (used as a CSS class — silently breaks the badge styling). For `formatIpcError`, `describeError` then concatenates the returned object into a template literal yielding `[object Object]` in the user-facing `errorMsg` signal. League.tsx already fixed this exact pattern at T2-6 (`frontend/src/routes/League.tsx:104-115`): it throws with a structured diagnostic message. Match.tsx's three sister sites carry the same defect Match.tsx self-review flagged as "T2-6 silent-failure-hunter P1 deferred"; the deferral has now persisted across a phase boundary.
  - Suggested fix: Replace each `return _exhaustive` with `throw new Error(\`<fn>: unhandled <Type> variant — drift in KNOWN_*_KINDS / <fn>. value=${JSON.stringify(_exhaustive)}\`);` — mirror League.tsx:111-114 verbatim. Three sites, one PR.

- **C-2** (severity: gate-blocker / pre-T3) — silent-failure class: missing runtime shape validation on IPC boundary
  - File: `frontend/src/lib/api/season.ts:28-68` (all four functions: `advanceWeek`, `playFixtures`, `getStandings`, `getFixtures`)
  - Pattern: `return invoke<DTOType>("cmd_name", args);` — direct cast of `invoke`'s `unknown` result to `DTOType` with no runtime check.
  - Why silent: The T1-3.6 audit response (`frontend/src/lib/runtime-validators.ts:4-12` doc comment) explicitly notes that `invoke<T>()` casts without runtime validation, and that backend wire-shape drift silently lands in TS + propagates as a `T`-shaped `any` until it NPEs deep in the render path. The fix at T1-3.6 was `safeInvoke(...)` + per-DTO guards. `playMatch` / `getBackendHandshake` / `matchFrames` are wrapped; the four season commands added at T2-5 are NOT (confirmed via grep on `invoke<` across `frontend/src/`). A Rust-side rename of `StandingsRowDto::club_name` → `clubName` (or a `Vec<>` → `BTreeMap<>` swap, or the addition of a `seasonComplete` discriminator that the frontend doesn't expect) would land silently in `getStandings()` and then NPE inside League.tsx's row renderer with no indication of the IPC boundary as the failure site.
  - Suggested fix: Add `isStandingsRow`, `isFixtureWithResult`, `isAdvanceWeekSummary`, `isPlayFixturesSummary` guards in `runtime-validators.ts`; wrap all four season IPC calls in `safeInvoke`. Mirror the existing pattern at `runtime-validators.ts:213-219` (BackendHandshake). One PR; the test scaffolding in `runtime-validators.test.ts` already covers the pattern.

- **C-3** (severity: pre-T3-recommended) — silent-failure class: debug_assert-on-canonical-bearing-invariant
  - File: `crates/fw-match-sim/src/bt/personality_bias.rs:173, 174, 194, 209, 222, 236, 252, 267, 268, 284, 296, 311, 323, 335` (13 sites)
  - Pattern: `debug_assert!(raw >= Q32::ZERO && raw <= Q32::ONE, "raw must be in [0,1]");` at the top of every `apply_*_bias` function.
  - Why silent: These functions multiply a raw utility Q32 by personality factors (`factor1 * factor2 * ...`) and the result feeds back into utility-scored BT leaves — i.e. they are canonical-decision-bearing. Release builds skip `debug_assert!` per Sim/RULES.md §11. If `raw` is outside [0,1] (caller bug: e.g. an unclamped utility somewhere upstream in `bt/on_ball.rs`), the function still multiplies happily and produces a polluted Q32 that propagates into softmax selection, changes the decision, changes canonical state, and changes the canonical hash. The proof this is a known pattern: sibling `crates/fw-match-sim/src/signature/bias_apply.rs:88-91` uses `assert!` (release-active) for the same "biased_utility >= ZERO" invariant class, with the inline note "sim invariant violated". `personality_bias.rs` adopted the weaker `debug_assert!` form despite the same load-bearing semantic. Additionally: `bt/on_ball.rs:225` notes "[0, 1] contract that `apply_shoot_bias::debug_assert!` expects" — confirming the [0,1] range IS treated as an invariant, not advisory.
  - Suggested fix: Either (a) promote all 13 sites to `assert!` per `bias_apply.rs:88` if the [0,1] range is a real invariant; OR (b) saturate-then-validate inside each helper rather than asserting. The current state asserts in debug + silently corrupts in release — the worst of both worlds. Comment-evidence at on_ball.rs:225 supports (a) as the intended semantic.

- **C-4** (severity: pre-T3-recommended) — silent-failure class: validate-structural false-positive on empty corpus
  - File: `crates/fw-content-baker/src/main.rs:316-385` (`run_validate_structural`)
  - Pattern: After `ContentStore::load_sources`, the four validator loops iterate the (possibly-empty) BTreeMaps; an empty corpus produces zero iterations + zero errors + prints `"validated 0 cultures, 0 archetypes, 0 role-affinity tables, 0 player templates, 0 signatures, 0 managers"` followed by `"STRUCTURAL validation passed"`.
  - Why silent: `ContentStore::load_sources` (`crates/fw-content/src/runtime.rs:546-745`) treats missing directories as silently-skipped (`if cultures_dir.is_dir() { ... }`). An operator running `validate-structural` against a freshly-cloned repo where `content/sources/cultures/` is missing OR against a malformed content pack with no archetypes will see a passing structural validation result. The downstream `AppState::new` would then fail at `generate_league` (state.rs:83-86 panics) — but that failure is far from the validation site. The validator's job is to fail-loud at corpus authoring time; the empty-corpus pass-through is the false-positive failure mode. Same class as T2-9's load_sources comment-vs-code-drift finding.
  - Suggested fix: Add a top-of-function check `if store.cultures.is_empty() || store.tactical_archetypes.is_empty() || store.player_templates.is_empty() || store.managers.is_empty() { anyhow::bail!("content corpus is empty in one or more required categories — content/sources/{{cultures,archetypes,players,managers}}/ must each contain ≥1 entity"); }` before the validator loops. Or stronger: per-category minimum counts (T2-5's `generate_league` already implicitly requires CLUBS_PER_LEAGUE=20 club-worthy entities; encode those gates in the validator).

- **C-5** (severity: pre-T3-recommended) — silent-failure class: unwrap_or-on-arbitrary-input-with-misleading-comment
  - File: `crates/fw-content/src/runtime.rs:87-108` (`load_commentary_grammars` inner loop)
  - Pattern: `let stem = path.file_name().and_then(|n| n.to_str()).map(|n| n.trim_end_matches(".tracery.json")).unwrap_or("");` followed by `match stem { "kickoff" => ..., other => { let _ = other; continue; } }` with the comment "Unknown filename — log and skip."
  - Why silent: TWO silent surfaces here. (a) `unwrap_or("")` masks non-UTF8 file names or `file_name() == None` as empty-stem, which falls into the `other` arm and silently `continue`s. (b) The `let _ = other; continue;` literally discards the unknown stem with NO log — the comment claims "log and skip" but the code never logs. A typo like `kickoff.tracery.json.bak` from an editor backup silently makes the file invisible to the loader; the downstream missing-discriminant check at line 125-128 will catch the MISSING required grammar but won't tell the operator that the .bak file was the cause.
  - Suggested fix: (a) Promote `unwrap_or("")` to explicit handling — `let stem = match path.file_name().and_then(|n| n.to_str()) { Some(s) => s.trim_end_matches(".tracery.json"), None => return Err(ContentLoadError::Io { path: path.clone(), source: io::Error::other("non-UTF8 filename in commentary/ directory") }), };`. (b) Either delete the misleading comment + the `let _ = other` dead code, OR add a real `tracing::warn!("skipping unknown commentary grammar filename: {}", path.display());` so the operator can actually see what got skipped.

- **C-6** (severity: opportunistic) — silent-failure class: standings-aggregation-silent-drop-on-bogus-club-id
  - File: `crates/fw-content/src/league.rs:538-552, 554-568`
  - Pattern: `if let Some(row) = rows.get_mut(home) { ... }` (and same for away) inside `Season::standings()`. The accumulator is pre-populated from `league.clubs` at lines 511-532; any result whose `(home, away)` references a ClubId NOT in `league.clubs` is silently dropped from the standings.
  - Why silent: The current call path (`Season::apply_result` at line 499 is the only mutator) is invoked from `advance_week_inner` (`crates/fw-tauri/src/commands.rs:247`) with fixtures that came from `season.fixtures_for_match_day`, which itself iterates `self.league.fixtures` — so today this is structurally unreachable. BUT `Season::apply_result` accepts arbitrary `ClubId` parameters with NO validation against `league.clubs`. If a future caller (test fixture, mod overlay, save-load path) ever calls `apply_result` with a stale or fabricated ClubId, those goals silently vanish from standings — an invisible scoreline bug.
  - Suggested fix: Add a guard at the top of `Season::apply_result`: `assert!(self.league.clubs.iter().any(|c| c.id == home), "apply_result: home ClubId {:?} not in league.clubs", home);` (same for away). Or change `apply_result` to return `Result<(), ApplyResultError>` with a `ClubNotInLeague` variant. Cheap; eliminates the invisible-scoreline class entirely.

- **C-7** (severity: opportunistic) — silent-failure class: lossy-string-conversion-on-author-controlled-path
  - File: `crates/fw-content-baker/src/bake.rs:196-199`
  - Pattern: `let output_filename = ron_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| format!("names_{slug}.ron"));`
  - Why silent: (a) `to_string_lossy()` silently substitutes U+FFFD for non-UTF8 bytes — for an author-controlled output path this should be a programmer error, not a silent substitution. (b) The `unwrap_or_else` reconstructs the filename from `slug` if `file_name()` returns None (impossible for a freshly built `output_dir.join(format!("names_{slug}.ron"))` but the fallback hides that impossibility). The manifest then records the reconstructed filename, which would differ from the actual file on disk in any future case where `ron_path` construction changes — silent manifest drift in the audit trail. The whole point of the T2-3 manifest is reproducibility; this fallback masks a corrupted manifest as a successful bake.
  - Suggested fix: `ron_path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()).expect("ron_path was just constructed from output_dir.join(format!(\"names_{slug}.ron\")); file_name + UTF8 must succeed")`. Defensive but loud — the manifest's reproducibility-audit guarantee depends on this not silently substituting.

- **C-8** (severity: opportunistic) — silent-failure class: lint-script swallows file-read errors
  - File: `scripts/lint-banned-terms.py:295-299` (`lint_file`) + `scripts/determinism-audit.py:200-203` (`audit_file`)
  - Pattern: `try: text = path.read_text(...) except OSError: return` — silently treats unreadable files as having zero violations.
  - Why silent: Both lint scripts walk the workspace. If a file errors on read (permission denied, partial-write race, deleted-mid-walk), the lint silently passes for that file — a banned term in a file that briefly errored would slip through CI. The lint scripts are the LAST line of defense for the banned-terms rule per Sim/RULES.md + Content/RULES.md §5; silent file-skip undermines that role. The risk is bounded (the walked files are in-tree + author-controlled), but the right semantic is "fail loud on any unreadable file" — a partial-walk lint is a false-pass.
  - Suggested fix: Replace `except OSError: return` (and `except (UnicodeDecodeError, OSError): return []`) with `except OSError as e: print(f"ERROR: lint cannot read {path}: {e}", file=sys.stderr); sys.exit(1)`. Loud-fail is the right semantic for a CI lint.

### Notes for consolidation

- **C-1 / C-2 / C-3 are the convergence with prior audit findings.** All three are "the project already knew this pattern" surfaces. C-1 = T2-6 P1 deferred. C-2 = T1-3.6 contract violated by T2-5. C-3 = Sim/RULES.md §11 (post-T1-16) authored to prevent exactly this; personality_bias.rs is the violation. Process discipline signal, not just three bugs.
- **C-4 mirrors T2-9's load_sources false-positive class** — same "validation silently passes on empty/missing" pattern, this time in the baker entrypoint vs the loader.
- Workspace was scanned for `decode_from_slice` siblings (per the prompt's checklist): `fw-save` is the only consumer per the grep at sweep time, so T2-9's fix has no siblings. Confirmed clean.
- `MemoryLedger` (fw-memory) is a T3 stub with no production callers — no silent-failure surface to flag yet.
- `Tick` arithmetic + `Q32` arithmetic + `should_decide` cadence (the canonical-state hot paths) audit clean per T1-21 + T1-23 — `Tick`'s panic-on-overflow + typed `checked_elapsed_since` / `checked_add_ticks` helpers are working as designed.
- `fw-tauri/commands.rs:134, 325` use `debug_assert!` but for genuine "Allowed" cases per Sim/RULES.md §11 (the actual safety check is the explicit `if` guard preceding the debug_assert, or a structural invariant already validated by the loop). Not flagged.
- All four `unwrap_or_else(|| panic!())` patterns in `tactic_fsm.rs:1134`, `subtree_library.rs:260, 344`, `signature/dispatcher.rs:103`, `lib.rs:1664` are correct invariant assertions with descriptive panic messages. Not flagged.
- `player.rs:231 saturating_add` has a thorough `// SAFETY:` comment per §11; not flagged.

---

## Track D — Test-the-tests (Claude `qa-lead`)

*Lens: vacuous-test patterns, redundancy across the suite, coverage holes that test NAMES imply but the test bodies don't actually cover, insta snapshot quality. Especially: tests that read named constants at assert-time (where mutating the constant doesn't fail the test), tests that assert default-shape only, integration tests that re-implement the production code instead of asserting against it.*

**Owner:** Claude `qa-lead`
**Status:** COMPLETE — 2026-05-18

### Summary

Tests reviewed: ~851 Rust `#[test]` items + 88 frontend Vitest `it()` blocks across 32 test files. Findings by severity: 1 gate-blocker, 4 pre-T3, 3 opportunistic. Top 3 most-concerning: (1) vacuous `smoke` test in `fw-save` — `assert_eq!(2+2, 4)` as the opening test in the migration-discipline crate; (2) `standings_sort_order_is_points_desc_then_gd_desc_then_gf_desc_then_club_id_asc` never exercises GD or GF tie-break sorting despite the name promising it does; (3) `play_fixtures_is_deterministic_same_seed` tests only the top-1 standings slot out of 20.

Verdict: **concerning**. No test is broken. The canonical-hash regression layer and proptest invariants are solid. But three coverage-holes in test names vs bodies, one actively vacuous test, and a systematic partial-determinism pattern in the IPC layer reduce mutation-detection confidence in the exact scenarios most likely to drift: save migration correctness, full-table sort order, and season-level determinism.

### Findings

- **D-1** (severity: gate-blocker) — class: vacuous
  - File: `crates/fw-save/src/lib.rs:207-210`, function `smoke()`
  - Issue: `assert_eq!(2 + 2, 4)` is the entire test body. The test lives in `mod smoke` inside `fw-save` — the crate that owns the four-test save-migration discipline. Mutating any production code in this file (`encode`, `decode`, `migrate_v0_to_v1`, `load_envelope`) does not fail this test. It contributes zero mutation-detection coverage to the most critical regression surface in the crate, and its presence as the FIRST named test in `mod smoke` misleads readers into expecting it guards something.
  - Suggested fix: Delete the `smoke()` test body. The adjacent `encode_decode_round_trip` test (immediately following at line 213) covers the same implied intent. Deletion also resolves D-7's redundancy finding against that test.

- **D-2** (severity: pre-T3) — class: name-vs-body-mismatch
  - File: `crates/fw-content/tests/season_state_test.rs:327-365`, function `standings_sort_order_is_points_desc_then_gd_desc_then_gf_desc_then_club_id_asc`
  - Issue: The test name claims four-key sort coverage (points DESC, goal_difference DESC, goals_for DESC, club_id ASC). The body injects uniform 1-0 wins for all day-1 fixtures, giving every winning club identical GD (+1) and identical GF (1). The test then falls through immediately to the club_id tiebreak — GD and GF sort arms are never exercised. Mutating `b.goal_difference.cmp(&a.goal_difference)` to `a.goal_difference.cmp(&b.goal_difference)` (reversing the GD sort direction) in `crates/fw-content/src/league.rs:577` would not fail this test. The sort implementation at lines 574-580 is correct; the test just doesn't discriminate it.
  - Suggested fix: Add two targeted tests: (a) two clubs with equal points but different GD — assert the higher-GD club ranks first; (b) three clubs with equal points and equal GD but different GF — assert higher-GF ranks first. Rename the existing test to `standings_sort_by_points_then_club_id_tiebreak` to match what it actually exercises.

- **D-3** (severity: pre-T3) — class: name-vs-body-mismatch (partial determinism)
  - File: `crates/fw-tauri/tests/season_commands_test.rs:151-172`, function `play_fixtures_is_deterministic_same_seed`
  - Issue: The test name claims same-seed determinism across a full 380-match season. The body asserts only `standings_a[0].points == standings_b[0].points` and `standings_a[0].club_id == standings_b[0].club_id` — two fields from the league leader. A bug that non-deterministically scrambled positions 2-20 while leaving position 1 stable would pass. The comment says "avoids fragility on field additions" — but `state_a.season().results` (the raw `BTreeMap<(ClubId,ClubId), MatchOutcome>`) is field-order-stable and already compared directly in `advance_week_is_deterministic_same_seed` in the same file (lines 97-112). The full-results comparison is the right target for the full-season determinism claim.
  - Suggested fix: Replace the two top-standings assertions with `assert_eq!(state_a.season().read().unwrap().results, state_b.season().read().unwrap().results, "same career seed must produce identical full-season results BTreeMap")`. This is already the pattern used by `advance_week_is_deterministic_same_seed` — apply it to the full-season test.

- **D-4** (severity: pre-T3) — class: coverage-hole (snapshot dormant)
  - File: `crates/fw-replay/tests/canonical_hash.rs:550-563`, function `smoke_seed_final_state_snapshot`
  - Issue: The insta snapshot for human-diffable state-change detection is `#[ignore = "snapshot baseline created alongside first CI green hash"]`. The pinned constant has gone through 18+ re-baselines since T0-7 and the snapshot was never activated. There is no committed `.snap` baseline (only one exists: `crates/fw-match-sim/tests/snapshots/match_event_snapshot__smoke_seed_60_tick_match_events.snap` for the event stream). A behavioral regression that keeps the canonical hash intact but changes player positions or possession state would produce no human-readable diff signal — only hex-string mismatch or silence. The meta-guard comment explicitly allows this test to stay ignored, but that permission has been abused for 18 re-baselines.
  - Suggested fix: Remove the `#[ignore]` attribute. Run `cargo test -p fw-replay -- smoke_seed_final_state_snapshot --review` to accept the current state as the insta snapshot baseline. Commit the resulting `.snap` file alongside this audit. The snapshot is not a correctness gate — it is a human-diff surface. It should have been activated at T0-7 alongside the first real hash.

- **D-5** (severity: pre-T3) — class: coverage-hole (stub tests counted as production coverage)
  - File: `crates/fw-content-baker/src/validators.rs:437-507`, four functions named `check_*_returns_not_implemented_*`
  - Issue: These four tests verify that stub functions (`check_banned_terms`, `check_licensed_data`, `check_cliche`, `validate_fragment`) return `Err(ValidationError::NotImplemented { ... })`. They count toward the "22 T2-3 baker tests" total in the CHANGELOG but they exercise only the stub contract, not production validation logic. When T2-4 ships real implementations, these four tests become invalid test artifacts that must be removed or rewritten. More critically, there is zero test coverage for what happens when content DOES contain a banned term or licensed data — the structural validators (CultureValidator, etc.) each have both happy-path AND rejection tests; the semantic validators have only the "returns NotImplemented" test.
  - Suggested fix: Add a `// TODO(T2-4): remove or replace this test when real implementation lands` comment on each of the four tests to make their lifecycle explicit. This is a documentation fix, not a structural one — the tests are correct for stubs. The coverage gap for real semantic validation is a T2-4 delivery concern, not a T2-phase defect.

- **D-6** (severity: opportunistic) — class: redundant
  - File: `crates/fw-save/src/lib.rs:213-222`, function `encode_decode_round_trip`
  - Issue: This test encodes `SaveEnvelope::V1` and decodes it, asserting equality. The subsequent `v0_and_v1_variants_construct_and_round_trip` test (lines 229-251) does the same V1 encode+decode+equality-check AND adds V0 + the first-byte-divergence guard. `encode_decode_round_trip`'s V1 assertion is entirely subsumed. The four migration-discipline tests (`forward_v0_to_v1`, `callback_preservation`, `forward-incompat`, `round-trip-byte-identical`) are all meaningfully orthogonal and are NOT flagged.
  - Suggested fix: Delete `encode_decode_round_trip`. Combined with D-1 (delete `smoke()`), the net effect is reducing fw-save tests from 11 to 9 with zero loss of mutation-detection coverage.

- **D-7** (severity: opportunistic) — class: coverage-hole (GD accumulation under overwrite)
  - File: `crates/fw-content/tests/season_state_test.rs:278-323`, function `apply_result_overwrites_prior_result`
  - Issue: The overwrite test records a home win (2-0) then overwrites it with an away win (0-1). It asserts on points and played — correctly verifying the overwrite semantics. It does NOT assert on `goal_difference`. A bug where `apply_result` subtracted the new result's goals from GD WITHOUT first rolling back the previous result's GD contribution would corrupt goal_difference silently — the overwrite would double-count negative GD from the first result's perspective. The `goal_difference_is_goals_for_minus_goals_against` test (lines 236-263) exercises GD computation on a single first-application, not on the overwrite path.
  - Suggested fix: Add `assert_eq!(home_row.goal_difference, -1, "overwritten result: home GD should be -1 (lost 0-1)"); assert_eq!(away_row.goal_difference, 1, "overwritten result: away GD should be +1 (won 1-0)");` at the end of `apply_result_overwrites_prior_result`.

- **D-8** (severity: opportunistic) — class: coverage-hole (IPC canonical hash scope overpromise)
  - File: `crates/fw-tauri/tests/ipc_contract_test.rs:39-84`, function `play_match_round_trip_canonical_hash_matches`
  - Issue: The test proves path-equivalence (IPC path ↔ direct path both call the same `encode_canonical` + `blake3::hash`). The inline comment honestly documents the limitation: "It does NOT prove the hash matches an external auditor's BLAKE3." The function NAME however is `play_match_round_trip_canonical_hash_matches` — implying a stronger external validation. This is a name-vs-scope mismatch, not a vacuousness finding (the test IS meaningful for catching IPC-path divergence from direct-sim-path). The canonical regression in `fw-replay/tests/canonical_hash.rs` is the authoritative external pin; the IPC test's naming confuses the two roles.
  - Suggested fix: Rename to `play_match_ipc_path_matches_direct_sim_call` and add a one-line comment referencing `fw-replay/tests/canonical_hash.rs` as the external hash pin. No structural change needed.

---

## Track E — Adversarial red-team (Codex CLI; user-driven)

*Lens: 4 attack goals.*
1. **Break canonical hash silently** — find a code change that produces drift bytes the existing pinned-hash test doesn't catch (e.g. fixture seed not in the corpus; per-OS divergence not in the matrix).
2. **Make a content pack pass validation while semantically invalid** — find a RON fixture that passes `fw-content-baker validate-structural` but produces wrong behavior at sim time.
3. **Malicious mod overlay** — author a mod-pack RON that exploits the load order / overlay rules per `Content/RULES.md §6` to inject banned terms / licensed data / dangling refs.
4. **Find a determinism leak** — identify a Rust pattern in the T2 diff that introduces non-determinism (hidden HashMap iteration, `Instant::now()`, OS-time read, thread-RNG, etc.) that the existing clippy bans don't catch.

**Owner:** Codex CLI
**Status:** Complete — 2026-05-18

### Codex CLI prompt (copy-paste verbatim into Codex)

```
Adversarial red-team review at Phase T2 close, Final Whistle (Rust + Tauri 2 + SolidJS football-management sim).
Commit range: b2d41f9b..26540dcd on main (10 commits, +9844 / -276 LoC, 77 files).

You are CODEX. Your job is to BREAK the project's invariants. Be specific + reproducible.

4 attack goals (separate findings per goal):

1. CANONICAL-HASH BYPASS. Find a code change in the T2 diff (or a code change you can author) that produces canonical-state drift bytes the existing pinned-hash test does NOT catch. The pin is `crates/fw-replay/tests/canonical_hash.rs` with two seeds (60-tick `blake3:eaf842ac…ad46` + 600-tick `blake3:aa7efe9b…5ae`). Per-OS divergence is checked via CI matrix [macos-14, windows-latest, ubuntu-22.04]. Look for: fixture seeds not in the corpus that would show drift; per-OS f64 calls that snuck back in; SeedLayer collisions; encoder field-order drift; #[serde(skip)] on a field that SHOULD be in canonical state.

2. CONTENT-PACK SEMANTIC POISONING. Author a RON fixture that passes `cargo run -p fw-content-baker -- validate-structural` (the T2-3 validator-as-one-class refactor) BUT produces wrong behavior at sim time. E.g. a TacticalArchetype with formation slots that pass the structural check (11 slots, no duplicate roster_slots, buildup_speed_factor in range) but breaks `tick_match`. Or a Culture whose name banks pass the ≥20-entry check but produces banned-term output via Markov chain composition.

3. MALICIOUS MOD OVERLAY. Per `Content/RULES.md §6`, mod overlays load lex-sorted by mod-id + can override existing IDs via `overrides: "fwh.core:..."`. Author a mod-pack RON that:
   (a) injects banned terms via override (silently substituting a clean fixture's display_name with a banned one — does the load-time banned-terms check catch the override?)
   (b) creates a dangling cross-ref via override (modify a ManagerArchetype's `tactical_archetype_id` to point at a removed archetype — does the DanglingReference check catch the post-override state?)
   (c) bypasses the sentinel-block restriction (per T1-20 the `ui-lint:ignore-start/end` is restricted to `docs/` / `crates/` / `scripts/` + `.md` paths — find a path that the restriction misses)

4. DETERMINISM LEAK. Walk the T2 diff (especially T2-5 `fw-tauri::season` + T2-3 `fw-content-baker/bake.rs` + T2-2 `fw-content::league.rs`) for any pattern that introduces non-determinism without tripping the existing clippy bans. Look for: hidden HashMap iteration via library trait impls (e.g. `std::collections::HashMap::iter` invoked indirectly); `tokio::time::Instant` in fw-tauri command handlers that bleeds back into a canonical-state DTO; `std::collections::hash_map::DefaultHasher` invoked by serde; OS-time bleeding through `cargo audit`'s downloads timestamp into a build script.

For each goal:
- Describe the exploit in 3-5 sentences (reproducible — no hand-waving)
- Provide the EXACT file path + line numbers + minimal change to demonstrate
- State the SEVERITY of the silent-failure / break (gate-blocker vs pre-next-phase recommended vs doc-only)
- If you can't find an exploit for a goal, say so explicitly (negative result is useful)

Write your findings into `docs/audits/post-t2-ultimate-review-2026-05-18.md` under the "Track E — Adversarial red-team" section. Do NOT commit. Hand back to main thread when done.
```

### Findings

### Summary

4 attack goals exercised. Findings: **0 gate-blockers**, **3 pre-T3**, **1 doc-only negative result**.

The core sim determinism path looks clean: no active `HashMap` / `HashSet` /
wall-clock / system-RNG / rayon leak was found in the T2 production path. The
red-team value is narrower: the canonical corpus still misses one newly-live
T2 state family (`SetPieceKind`), structural validation can be semantically
poisoned by composed banned terms, and the determinism audit has an overly broad
file-level exemption on the calibration binary.

- **E-1** (severity: pre-T3) — attack goal: canonical-hash bypass
  - File: `crates/fw-match-sim/src/canonical.rs:775-788`,
    `crates/fw-match-sim/src/canonical.rs:904-924`,
    `crates/fw-replay/tests/canonical_hash.rs:680-726`
  - Exploit: swap two `SetPieceKind` tags in `set_piece_kind_tag` — for example
    `CornerFor => 3` and `ThrowInFor => 7`. The existing two pinned canonical
    seeds do not enter a `SetPiece` state, so the pinned BLAKE3 tests still pass.
    The local unit test `setpiece_encoding_includes_kind_tag` only asserts that
    two different set-piece variants encode differently; a tag swap preserves
    that property and still changes replay wire semantics for any future OOB
    fixture.
  - Minimal change to demonstrate:
    ```rust
    // crates/fw-match-sim/src/canonical.rs
    SetPieceKind::CornerFor => 7,
    SetPieceKind::ThrowInFor => 3,
    ```
  - Suggested fix: add exact discriminant tests for all 11 `SetPieceKind`
    variants OR add a third canonical fixture that intentionally reaches
    `BallOutOfPlay -> SetPiece` and pins its hash. Exact tag tests are cheaper;
    the third fixture gives better end-to-end protection.

- **E-2** (severity: pre-T3) — attack goal: content-pack semantic poisoning
  - File: `crates/fw-content-baker/src/validators.rs:238-242`,
    `crates/fw-content/src/runtime.rs:767-773`
  - Exploit: a `Culture` with 20 first names all `"Man"`, 20 last names all
    `"chester"`, and `naming_pattern: "{first}{last}"` passes
    `fw-content-baker validate-structural` but deterministically generates the
    banned place-name `"Manchester"` at runtime/bake time. I verified this using
    a temp content copy at `/tmp/fw-poison.*`: `validate-structural` exited 0 and
    reported 3 cultures validated. The structural validator checks only first
    and last bank lengths; it does not sample or lint composed outputs.
  - Minimal fixture:
    ```ron
    Culture(
        id: "fwh.test:culture.manchester-composed",
        name: "Composed Poison",
        first_name_bank: ["Man", "Man", "Man", "Man", "Man", "Man", "Man", "Man", "Man", "Man", "Man", "Man", "Man", "Man", "Man", "Man", "Man", "Man", "Man", "Man"],
        last_name_bank: ["chester", "chester", "chester", "chester", "chester", "chester", "chester", "chester", "chester", "chester", "chester", "chester", "chester", "chester", "chester", "chester", "chester", "chester", "chester", "chester"],
        team_name_bank: [],
        naming_pattern: "{first}{last}",
        weights: (first_alpha_diversity_bps: 0, compound_last_chance_bps: 0),
    )
    ```
  - Suggested fix: add a semantic content validator that samples a deterministic
    corpus from every `Culture` and runs `scripts/lint-banned-terms.py` against
    the generated strings. Until then, `validate-structural` should continue to
    say clearly that it is not a semantic content-pack validator.

- **E-3** (severity: doc-only negative result) — attack goal: malicious mod overlay
  - File: `.claude/rules/Content/RULES.md:66-77`,
    `crates/fw-content/src/runtime.rs:546-755`,
    `scripts/lint-banned-terms.py:230-268`
  - Result: no current exploit was found because mod overlays are not implemented.
    `ContentStore::load_sources` reads only `content/sources/**`; `load_baked`
    still delegates to `load_sources`; no current RON type has an `overrides`
    field. A malicious `content/mods/<id>/...` pack is ignored rather than
    merged, so override-based banned-term injection and dangling-reference
    attacks cannot execute today.
  - Sentinel side-channel: the sentinel restriction appears correctly anchored
    at root-relative prefixes. Paths like `frontend/src/docs/legacy/x.ts` and
    `content/sources/scripts/x.ron` are covered by regression tests and do not
    honor sentinel blocks.
  - Suggested fix: change the wording in Content/RULES.md §6 from "Mods live..."
    to "Future mod overlays will live..." until the loader exists, or add a
    MASTER_PLAN row for the real mod-overlay loader + post-merge validation.

- **E-4** (severity: pre-T3) — attack goal: determinism leak
  - File: `scripts/determinism-audit.py:169-192`,
    `crates/fw-match-sim/src/bin/calibrate.rs:1-10`
  - Exploit surface: `crates/fw-match-sim/src/bin/calibrate.rs` is in
    `FULLY_EXEMPT_FILES`, which suppresses every determinism-audit rule, not
    just float arithmetic. A future edit could add `SystemTime::now()`,
    `HashMap`, or `thread_rng()` to the corpus collection path and
    `scripts/determinism-audit.py` would still report clean. I did not find an
    active leak in the current file; the issue is that the guardrail is blind on
    the one T2 tool whose output is intended to become source constants later.
  - Minimal change that would bypass the script:
    ```rust
    // crates/fw-match-sim/src/bin/calibrate.rs
    let stamp = std::time::SystemTime::now();
    ```
  - Suggested fix: replace the full-file exemption with a per-rule float
    exemption, or split the f64 fit routines into a separate exempt module while
    keeping the corpus runner covered by the normal HashMap/time/RNG bans.

---

## Track F — Property explosion (Codex CLI; user-driven)

*Lens: bump PROPTEST_CASES from default 256 → 10,000 on key invariants; bump intra-process determinism count; run for ~15-20 min wall-clock.*

**Owner:** Codex CLI
**Status:** Complete — 2026-05-18

### Codex CLI prompt (copy-paste verbatim into Codex)

```
Property-explosion run at Phase T2 close, Final Whistle.

You are CODEX. Your job is to find proptest failures that the default 256-case shrink misses. Use a higher case count + a different seed.

Pre-flight:
- `cd /Users/vibelogic/dev/football`
- `git status` should show clean tree on main + 11 commits ahead of origin
- Run `scripts/fw verify` first; must be green

Then:

1. Identify every proptest in the workspace: `grep -rn "#\[proptest\|proptest!" crates/ | head -30`
2. For each proptest, run with `PROPTEST_CASES=10000` (bump from default 256 -> 10000):
   `PROPTEST_CASES=10000 cargo test -p <crate> --release -- <test_name>`
   Use --release so 10k cases finish in reasonable wall-clock.
3. Also bump intra-process determinism: where a test runs N matches and compares hashes, bump N from 10 to 100 if the test allows.
4. Specifically target these high-value invariants:
   - fw-match-sim proptest: canonical-state determinism across seeds + ticks
   - fw-content proptest: League fixture pair-coverage symmetry (each (home, away) pair exactly once + per-club 19h/19a)
   - fw-replay proptest: encoder field-order stability + roundtrip
   - fw-save proptest: encode -> decode -> re-encode byte identity (if a proptest exists; if only the unit-test version, author a quick proptest with random SaveV1 fixtures + assert round-trip)
5. Time-box: ~15-20 min wall-clock. If a test hangs or panics, kill it + report the seed that triggered.

For each finding:
- Test name + crate
- PROPTEST_CASES that surfaced the failure
- Minimal-shrunk test case (proptest's shrink output)
- Expected vs actual behavior
- Severity classification (gate-blocker / pre-T3-recommended / opportunistic)

Write findings into `docs/audits/post-t2-ultimate-review-2026-05-18.md` under "Track F — Property explosion". Do NOT commit. Hand back to main thread when done.
```

### Findings

### Summary

Commands run:
- `scripts/fw verify` — green.
- `PROPTEST_CASES=10000` on the existing `fw-match-sim` proptest targets:
  `dispatch_proptest`, `behavior_proptest`, `decision_cadence_proptest`,
  `tactic_fsm_proptest`, `utility_proptest`, `separation_proptest`,
  `ball_physics_proptest`, `ball_mutation_proptest`, `bt_runner_proptest`,
  `signature_dispatcher_proptest`, `match_event_proptest`.
- `FW_DETERMINISM_SMOKE_RUNS=100 FW_DETERMINISM_EXTENDED_RUNS=100 cargo test -p fw-replay --release --test canonical_hash -- smoke_seed_runs_100_times_produce_one_hash extended_seed_runs_10_times_produce_one_hash` — green.
- `cargo test -p fw-content --release --test league_generation_test fixture_schedule_covers_all_pairs_home_and_away` — green.
- `cargo test -p fw-save --release -- smoke::v1_encode_decode_reencode_produces_identical_bytes migration::callback_preservation_v0_seed_survives_migration_bit_exact` — green.

Findings: **0 gate-blockers**, **2 pre-T3**, **1 opportunistic**. The notable
result is that the 10k sweep found one real rare-seed behavioral invariant
failure and one proptest generator that cannot survive 10k cases because it
rejects too much.

- **F-1** (severity: pre-T3) — `fw-match-sim` / `behavior_proptest::team_width_when_in_possession_within_band`
  - Cases: surfaced at `PROPTEST_CASES=10000`.
  - Minimal shrunk case: `seed_u64 = 7611787884383819691`
    (`0x69a280c07a51d7ab`). Proptest generated regression key:
    `cc 73c326c21cd6974a51aca6d3f0e08414ead7709c41734933b09606fec23e644a`.
  - Expected vs actual: at tick 252, outfield-carry width was
    `Q32(24.9463570719)` with carrier slot 20; the invariant requires
    `[Q32(25), Q32(70)]`. This is barely below the lower band, but it is a real
    failure of the football-shape width invariant at deeper case count.
  - File: `crates/fw-match-sim/tests/behavior_proptest.rs:455-463`
  - Suggested fix: decide whether the lower band should include a small Q32
    tolerance (for example 24.75m) or whether the sim needs a small formation
    width correction for this seed. If the behavior is accepted, check in the
    regression seed with the updated bound so the edge case stays visible.

- **F-2** (severity: opportunistic) — `fw-match-sim` / `tactic_fsm_proptest`
  - Cases: surfaced at `PROPTEST_CASES=10000`.
  - Minimal case: none; the run aborts because three tests exceed proptest's
    global reject cap, not because a semantic counterexample shrinks.
  - Expected vs actual: `transition_is_deterministic`, `apply_event_is_pure`,
    and `heartbeat_check_is_pure` all use
    `prop_assume!(now_tick >= current.entry_tick())`. At 10k cases they abort
    after roughly 2.7k successes and 1024 global rejects.
  - File: `crates/fw-match-sim/tests/tactic_fsm_proptest.rs:122-164`
  - Suggested fix: generate `now_tick` from `entry_tick..entry_tick + N`
    instead of drawing an independent `0..10000` and rejecting invalid pairs.
    The invariant is likely fine; the generator is just too wasteful for
    audit-time property explosion.

- **F-3** (severity: pre-T3) — requested high-value proptests do not exist in three crates
  - `fw-content`: no proptest exists for fixture pair coverage / 19h+19a; only
    unit/integration tests exercise the shape.
  - `fw-replay`: no proptest exists for encoder field-order stability or
    roundtrip; pinned-hash and exact unit tests carry that burden.
  - `fw-save`: no random `SaveV1` encode → decode → re-encode proptest exists;
    the byte-identical check is a fixed unit test.
  - Scope note: I did not author a new test because this audit was read-only
    except this shared file.
  - Suggested fix: add one focused proptest per crate before T3-1: fixture
    schedule coverage over generated seed corpus, canonical encoder mutation /
    field-order probe, and random `SaveV1` byte-identical roundtrip. This is
    test-substrate hardening, not a phase blocker by itself.

### Negative results

At `PROPTEST_CASES=10000`, these existing `fw-match-sim` targets passed:
`dispatch_proptest`, `decision_cadence_proptest`, `utility_proptest`,
`separation_proptest`, `ball_physics_proptest`, `ball_mutation_proptest`,
`bt_runner_proptest`, `signature_dispatcher_proptest`, and
`match_event_proptest`. The replay intra-process hash rerun also stayed stable
with both smoke and extended run counts set to 100.

---

## Consolidated verdict

**Status:** Complete — 2026-05-18 (all 6 of 6 tracks landed)

### Headline

**Verdict: ACCEPT-WITH-FIXES.** 3 gate-blockers found across Tracks C + D, **all 3 fixed in-place pre-PR** at commit `d573161`. Codex Tracks E + F added 0 new gate-blockers. **Phase totals: 3 gate-blockers (all fixed), 17 pre-T3 findings, 9 opportunistic.** No findings warrant phase REJECT.

### Track-by-track summary

| Track | Owner | Findings | Gate-blockers | Pre-T3 | Opportunistic | Verdict |
|---|---|---|---|---|---|---|
| A — mutation analysis | Claude code-explorer | 6 | 0 | 4 | 2 | concerning (T2-2/T2-5 only) |
| B — arch drift | Claude code-reviewer | 8 | 0 | 3 | 5 | concerning |
| C — silent-failure sweep | Claude silent-failure-hunter | 8 | 2 (fixed) | 3 | 3 | concerning |
| D — test-the-tests | Claude qa-lead | 8 | 1 (fixed) | 4 | 3 | concerning |
| E — adversarial red-team | Codex CLI | 4 | 0 | 3 | 0 (1 doc-only negative) | accept (narrow value) |
| F — property explosion | Codex CLI | 3 | 0 | 2 | 1 | accept (1 real rare-seed find) |

### Gate-blockers (all 3 fixed in-place at commit `d573161`)

1. **C-1 — Match.tsx `return _exhaustive` runtime garbage at 3 sites** (`formatIpcError` + `eventLabel` + `badgeClass`). Sister of the post-T2-6 silent-failure-hunter P1 fix landed on League.tsx; the deferred Match.tsx P1 had hardened into recurring debt across a phase boundary. **FIX**: throw with payload preview, matching the League.tsx pattern.
2. **C-2 — TS `season.ts` IPC wrappers skip runtime shape validation** (all 4 T2-5 commands). Undoes the T1-3.6 audit response one phase later; backend DTO drift would silently NPE deep in League.tsx / Transfers.tsx. **FIX**: route all 4 through `safeInvoke` + 4 new runtime shape guards (`isAdvanceWeekSummary`, `isPlayFixturesSummary`, `isStandingsRowArray`, `isFixtureWithResultArray`) in `runtime-validators.ts`.
3. **D-1 — Vacuous `smoke()` test in fw-save** asserting `2 + 2 == 4`; mutation-survives every fw-save production change. **FIX**: deleted. A vacuous test in the save-migration crate is exactly the false-confidence pattern the four-test discipline exists to prevent.

### Cross-track convergence patterns (highest-value signal)

- **"Docs promise more than code delivers"** — Tracks B + D both surface this. Track B finds 4 doc-vs-code drifts in `tactic-fsm.md` (3 deferred timer events + missing `tactic-fsm-heartbeat-rules.md`) + `personality-bias-weights.md` (K_15..K_21 missing). Track D-3 finds `play_fixtures_is_deterministic_same_seed` test NAME claims full-season determinism but only asserts on row 0. Pattern: design docs + test names overpromise their substance. **Recommended row:** "docs-and-test-name honesty pass" before T3-1.
- **"Same-constant-on-both-sides vacuity"** — Track A finds 4 instances (CLUBS_PER_LEAGUE / MATCH_DAYS_PER_SEASON shifts together across loop bounds + asserts). Track D-7 finds GD coverage hole in `apply_result_overwrites_prior_result`. Pattern: tests that derive their expected values from the same constants the production uses. **Recommended row:** "literal-pin sanity tests for league constants" (cheap; ~30 LoC).
- **"Sister silent-failure pattern across files"** — Track C-1 (Match.tsx ↔ League.tsx) is the headline case. Track C also surfaces 13 `debug_assert!` sites in `personality_bias.rs` echoing the Sim/RULES.md §11 violation pattern. Pattern: a fix lands in one file; the same shape persists in sibling files until a future audit catches it. **Recommended row:** workspace-wide `debug_assert!` audit for canonical-invariant misuse (per Sim/RULES.md §11).

### Recommended new MASTER_PLAN rows (pre-T3, ordered by value)

**Pre-T3 row R1 — Tactic-FSM + personality-bias doc-honesty pass** (1-2h): tactic-fsm.md transition table needs explicit "Deferred to T3" caveat for `PressTimeoutExpired` / `CounterWindowClosed` / `HalfTime` + reference to the deferred `tactic-fsm-heartbeat-rules.md`. personality-bias-weights.md mapping table needs K_15..K_21 added (currently 15 of 21 live coefficients are doc-visible; 6 are not). Addresses B-1, B-2, B-3, B-4.

**Pre-T3 row R2 — Test-quality pass for T2-2 + T2-5 vacuous constants** (1h): add 4 literal-pin sanity tests (`CLUBS_PER_LEAGUE == 20`, `MATCH_DAYS_PER_SEASON == 38`, `CLUBS_PER_LEAGUE / 2 == 10`, `(CLUBS_PER_LEAGUE - 1) * 2 == 38`); replace 3 vacuous derivations with hard literals; tighten `calibrate_smoke` lower bound 1 → 10. Addresses A-1 through A-6. Also: activate the `#[ignore]`'d `smoke_seed_final_state_snapshot` (D-4); fix `apply_result_overwrites_prior_result` GD coverage hole (D-7); rename `play_match_round_trip_canonical_hash_matches` → `play_match_ipc_path_matches_direct_sim_call` (D-8).

**Pre-T3 row R3 — Personality-bias `debug_assert!` migration to `assert!`** (~30 min): 13 sites in `personality_bias.rs` use `debug_assert!` for `[0,1]` invariants — Sim/RULES.md §11 bans this for canonical invariants. Sibling `signature/bias_apply.rs:88` uses `assert!` for the same class, proving the pattern is known. Addresses C-3.

**Pre-T3 row R4 — `fw-content-baker validate-structural` empty-corpus false-positive** (~30 min): prints "validated 0 cultures, 0 archetypes" + exits OK against an empty content corpus. Add a non-zero check or a `--allow-empty` opt-in flag. Addresses C-4.

**Opportunistic row R5 — sibling silent-failure cleanups** (~1h): commentary loader / standings aggregator / bake manifest / Python lint scripts. Addresses C-5 through C-8. Fold into other touching commits if convenient; not standalone urgent.

**Doc-only row R6 — Track D-5 NotImplemented validator tests**: mark with `TODO(T2-4)` comments so the next phase that touches `validators.rs` knows to delete them.

**Pre-T3 row R7 — Codex Track E + F follow-ups** (~2h total): (a) `SetPieceKind` canonical-tag pinning — add exact discriminant tests for all 11 variants OR a third canonical fixture that reaches `BallOutOfPlay -> SetPiece` and pins its hash (E-1). (b) Semantic content validator authored that samples each `Culture`'s composed name output + runs `scripts/lint-banned-terms.py` against the generated strings; until then `validate-structural` docs/CLI must explicitly say "structural only, NOT semantic" (E-2). (c) Replace the full-file `FULLY_EXEMPT_FILES` entry for `calibrate.rs` in `scripts/determinism-audit.py` with per-rule float-only exemption — the corpus runner must stay covered by HashMap/time/RNG bans (E-4). (d) Investigate + resolve the F-1 rare-seed behavioral failure (`team_width_when_in_possession_within_band` at seed `0x69a280c07a51d7ab`, tick 252, width `Q32(24.9463)` vs band `[Q32(25), Q32(70)]`) — either widen tolerance OR fix the formation-width drift; check in the regression seed either way. (e) Author 3 missing high-value proptests (fw-content fixture pair-coverage; fw-replay encoder field-order; fw-save random `SaveV1` roundtrip byte-identity) per F-3.

**Opportunistic row R8 — `tactic_fsm_proptest` generator narrowing** (~30 min): the 3 affected tests draw `now_tick` from `0..10000` then `prop_assume!(now_tick >= entry_tick)` — at PROPTEST_CASES=10000 the global reject cap fires at ~2.7k successes. Generate `now_tick` directly from `entry_tick..entry_tick + N` instead. Addresses F-2.

**Doc-only row R9 — Content/RULES.md §6 mod-overlay future-tense fix**: change "Mods live in `content/mods/<mod-id>/`..." to "Future mod overlays will live in..." until the loader exists. Per E-3 the malicious-overlay attack goal returned a negative result because the overlay loader isn't implemented; the spec presently reads as if it is. Honesty fix.

### Deferred to T3 phase (rolled-rows context)

T2-4 (PlayerBio) + T2-7 (Squad) + T2-1d2 (utility_shoot rewire) were all rolled to T3 per user direction (`docs/MASTER_PLAN.md` rows now carry `DEFERRED-ROLLED-TO-T3` status). T2-4 + T2-7 promote upon `design/player-generation.md` authorship; T2-1d2 promotes after T3 BT-runner maturation per `personality-bias-weights.md §Re-tuning cadence`.

### Tracks E + F resolution

Codex CLI ran both tracks in a user-driven session post-`/done` invocation. Both completed with **0 new gate-blockers** + 5 pre-T3 findings + 1 opportunistic. E-3 returned a doc-only NEGATIVE RESULT (the mod-overlay loader isn't implemented yet — Content/RULES.md §6 reads as if it is; fix is doc-honesty-only, captured as R9). All 5 actionable findings are folded into rows R7 + R8 + R9 above for landing before T3-1.

### Codex consolidation

Codex Tracks E + F add **0 new gate-blockers**, but they strengthen two existing
Claude-track patterns and add one new category. Convergence: E-2 supports
Track C's "structural validation is not semantic validation" concern; F-3
supports Tracks A/D on missing high-value property coverage; E-4 is the same
guardrail-blind-spot class as earlier determinism-audit exemption findings.
New category: F-1 found a rare-seed football-shape invariant failure that the
default 256-case run misses. Recommended before T3-1: add a small
`T2-codex-followup` cleanup row covering E-1/E-2/E-4/F-1/F-2/F-3, or fold those
items into the existing pre-T3 rows if the main thread wants fewer plan rows.

### Verdict

**ACCEPT-WITH-FIXES.** Phase T2 ships. The 3 gate-blockers were fixable in <100 LoC total; all 6 tracks (4 Claude + 2 Codex) confirm the deeper architectural + correctness invariants are intact. The 17 pre-T3 findings warrant 6 small cleanup rows (R1-R4 + R7 + R9 substantive; R5 + R6 + R8 opportunistic) before T3-1 dispatches. Tag `v0.2.0-season` per T2 exit gate Bullet 5.

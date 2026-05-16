# Post-T1 Ultimate Review — 2026-05-16

Multi-agent adversarial review post-Codex-Tier-3 ACCEPT verdict. User explicitly requested "hardcore review beyond what we've done before" with 4 Claude subagents + 2-3 Codex tracks running in parallel.

## Why this audit exists

Codex Tier-3 phase-boundary review on `v0.1.0-first-match` returned ACCEPT on 2026-05-16. The prior 10 audit passes this phase covered: Codex Tier-2 mid-phase on T1-2b, Codex post-T1-7 adversarial multi-agent (6 agents), Codex Tier-2 pre-/done, Codex Tier-3 phase-boundary, plus per-task self-review triple on every ≥100 LoC commit.

The user wants an 11th pass with a deliberately different lens — not per-commit-diff-scoped (we've done that ~20×), not Codex-conventional-review-scoped (4×), but **whole-codebase + adversarial + tactics-we-haven't-used**:

- Mutation-test analysis (does the test suite catch hypothetical regressions?)
- Architectural drift (do CLAUDE.md / ADRs / docs/specs match the code?)
- Whole-codebase silent-failure sweep (not commit-scoped — sweep every fw-match-sim/fw-core/fw-content path)
- Test-the-tests (which tests would silently pass if disabled?)
- Adversarial red-team (Codex side: try to BREAK the canonical hash; try to make a content pack pass validation while semantically invalid)
- Property explosion (Codex side: bump proptest CASES 256 → 10,000+ on key invariants)

## Audit scope

- **Commit range:** full T1 phase (`27920de6..v0.1.0-first-match` = `3a7fce97`), but lens is whole-codebase not commit-by-commit
- **Files in scope (everything in workspace except generated artifacts):** all `crates/**/src/**/*.rs`, `crates/**/tests/**/*.rs`, `content/sources/**/*.ron`, `frontend/src/**/*.{ts,tsx}`, `docs/**/*.md`, `.claude/**/*.md`, `Cargo.toml`, `CLAUDE.md`, `STATUS.md`, `MEMORY.md`
- **Workflow discipline (binding — per /next skill 2026-05-16 hardening):** all subagents READ-ONLY. No code edits. No commits. Append findings to THIS file only. Main thread consolidates verdict at end.
- **Don't repeat:** any finding already addressed in T1-2b-fix / T1-3.6 / T1-15+T1-16+T1-18 / Codex Tier-2 pre-/done is OUT OF SCOPE. New findings only.

## Tracks (parallel — agents append to their own sections)

---

### Track A — Claude mutation-test analysis

**Agent:** `feature-dev:code-explorer`
**Scope:** `crates/fw-match-sim/src/bt/**`, `crates/fw-match-sim/src/dispatch.rs`, `crates/fw-match-sim/src/ball_physics.rs`, `crates/fw-match-sim/src/canonical.rs`
**Lens:** For each load-bearing operator / constant / branch in the named files, propose a hypothetical mutation (flip `<` to `<=`, swap `+` to `-`, change a constant by ±10%, invert a boolean branch). Predict which test SHOULD catch the mutation. Identify mutations where NO test would catch the regression (= test-coverage hole). Don't actually mutate the code — produce a mutation-coverage map.

[agent appends findings below this line — main thread appended below on behalf of `feature-dev:code-explorer` agent which lacked Write/Edit tools in its sandbox]

#### Track A findings — 2026-05-16

**Headline:** 5 RED coverage holes + 3 YELLOW (covered but weakly) + 4 systemic patterns. Test suite is strong on canonical determinism, utility ranges, attribute bindings, and ball physics endpoints. Holes concentrated in `preempt_check` (zero behavioral unit tests), cross-team pass-target correctness, GK own-side chase logic, pressure-gating inside shoot bias, and role-state transition exit paths.

##### Mutation entries (selected highlights — 30 entries audited)

**`on_ball.rs:250` — `dist_to_goal < Q32::from_int(30)` (4× proximity threshold)**
- M1: flip `<` to `<=`. **Coverage:** YELLOW (boundary never tested directly; 4× branch test uses dist=22.5m well inside).
- M2: swap 30m and 40m thresholds. **Coverage:** RED (both still ∈ [0,1] post-clamp; no test asserts which specific multiplier fires).

**`on_ball.rs:237-241` — `roster_slot < 11` (home/away discriminator for goal_x)**
- M1: `<` → `<=`. **Coverage:** YELLOW (`shoot_target_direction_is_correct` covers slots 6 + 16; boundary slot 11 not directly tested).

**`on_ball.rs:477` — `fy - Q32::from_int(7)` (lay-off target_y)**
- M1: change `-` to `+`. **Coverage:** RED (no test asserts lay_off target direction).

**`on_ball.rs:416-420` — `utility_dribble` advance `Q32::from_int(8)`**
- M1: `8` → `80` (off-pitch). **Coverage:** RED (no pitch-bounds assertion on dribble targets).

**`off_ball.rs:174` — `press_target_slot = if roster_slot < 11 { 11 } else { 0 }`**
- M1: invert direction (home presses own GK). **Coverage:** RED (no team-correctness test for Press target).

**`off_ball.rs:200` — `mark_slot = if roster_slot < 11 { 12 } else { 1 }`**
- M1: always slot 12. **Coverage:** RED (no test for MarkPlayer cross-team semantics).

**`personality_bias.rs:182-184` — `apply_shoot_bias` `factor2 = Q32::ONE + K_2_SHOOT_COMPOSURE * composure * pressure.0`**
- M1: drop `pressure.0`. **Coverage:** RED (no test compares apply_shoot_bias output at pressure=0 vs pressure=1 with matching attrs).

**`personality_bias.rs:357` — `read_defender_pressure` `denom = Q32::ONE + PT_DIVISOR_COEFF * pt`**
- M1: `+` → `-`. **Coverage:** GREEN (`read_defender_pressure_pt_one_is_approx_raw_div_1_75` catches: with `-`, denom=0.25 → 4×raw vs expected 0.5714).
- M2: `PT_DIVISOR_COEFF = 0.75` → `0.25`. **Coverage:** GREEN (same test catches at endpoint).

**`dispatch.rs:79` — `MAX_PLAYER_SPEED = 8 m/s`**
- M1: `8` → `80`. **Coverage:** RED (`apply_intent_clamps_to_max_speed` asserts `vel_x == MAX_PLAYER_SPEED` by reading the const at assert time; constant mutation invisible to constant-referencing test).
- M2: `clamp_speed` uses `>` instead of `>=`. **Coverage:** YELLOW (boundary delta = MAX_PLAYER_SPEED untested).

**`dispatch.rs:797` — `team_end = team_start + 11`**
- M1: `+ 11` → `+ 10` (misses last slot per team). **Coverage:** YELLOW (debug_assert catches in debug; no release-mode test on slot 10 or 21 as passer).

**`dispatch.rs:797` — `passer_team = if passer_slot_idx < 11 { 0 } else { 1 }`**
- M1: `<` → `<=` (cross-team pass from slot 11). **Coverage:** RED (no test verifies from_slot/to_slot same team).

**`dispatch.rs:940` — `closer_count >= 2` (nearest-2 chaser limit)**
- M1: `>= 2` → `>= 1`. **Coverage:** RED (chaser-count policy has no direct test; team-width proptest might NOT fail with fewer chasers since formation may improve).

**`dispatch.rs:888-889` — `THRESHOLD_BITS = 42 m` (GK chase threshold)**
- M1: `42` → `52` (GK chases only within 0.5m of goal line). **Coverage:** RED (no test exercises GK loose-ball chase at intermediate distance).

**`dispatch.rs:895` — `home_gk_side = bx_bits < 0` (GK own-side check)**
- M1: flip `<` to `>`. **Coverage:** RED (no test asserts home GK does NOT chase ball near away goal line).

**`ball_physics.rs:213` — `started_on_ground = state.pos_z <= Q32::ZERO`**
- M1: `<=` → `<`. **Coverage:** GREEN (`ball_at_rest_on_ground_stays_at_rest` catches via gravity-induced pos_z < 0 → bounce fires under mutation).

**`ball_physics.rs:263` — `vz = -(coeffs.bounce_retention * vz)` (bounce reverses)**
- M1: drop negation. **Coverage:** GREEN (`ground_bounce_reverses_and_dampens_vertical_velocity` catches).

**`canonical.rs:500` — attribute write order (55 fields)**
- M1: swap any two fields. **Coverage:** GREEN (canonical_hash regression test catches every byte reorder).

**`canonical.rs:339-342` — `encode_option_slot(state.possession)` before `encode_option_slot(state.last_touched_by)`**
- M1: swap. **Coverage:** GREEN via hash pin only; YELLOW for semantic-level test (no encode-decode roundtrip on these two fields independently).

**`canonical.rs:446-451` — `PassKind` discriminant mapping (Short=0, Long=1)**
- M1: swap. **Coverage:** YELLOW (only hash pin catches; logic-level tests reading `MatchEvent::Pass.kind` would miss the discriminant tag swap).

**`role_states.rs:207-209` — `InPossession` exit defaults (Defender → Supporting; Midfielder → Supporting; Forward → RunningOffBall)**
- M1: change `Supporting` to `Defending`. **Coverage:** RED (no test sets possession=Some(slot_5), then None, then checks slot_5's role state).
- M2: invert `is_carrier` check. **Coverage:** YELLOW (team_width proptest might fail emergently; no direct unit test).

**`goalkeeper_fsm.rs:107` — `approaching_goal = bvx < Q32::ZERO` (home)**
- M1: `<` → `<=`. **Coverage:** RED (no test with bvx=0 in penalty area).

**`goalkeeper_fsm.rs:119` — `dist_from_line < DISTRIBUTION_THRESHOLD` (3m)**
- M1: remove the `bx > HOME_GOAL_X` guard. **Coverage:** RED (no test with ball behind goal line, bx < -52.5).

##### Top 5 RED coverage holes

1. **`apply_shoot_bias` pressure-gating** (`personality_bias.rs:182`): dropping `pressure.0` from factor2 invisible to suite.
2. **`preempt_check` GK own-side selection** (`dispatch.rs:895`): no negative test for "home GK does NOT chase ball at away goal line".
3. **`nearest_teammate_near` cross-team pass** (`dispatch.rs:797`): `<` → `<=` would route slot-11 search into home team; no test verifies same-team pass.
4. **`evaluate_transitions` InPossession exit states** (`role_states.rs:207-209`): possession-transfer exit-state transitions have no unit test.
5. **Press/Mark target team-correctness** (`off_ball.rs:174, 200`): neither `utility_press` nor `utility_mark_player` has a test asserting target is on opponent's team.

##### Top 3 YELLOW (covered but weakly)

1. Proximity zone boundary at 30m (on_ball.rs:250) — exact boundary untested.
2. `nearest_teammate_near` team_end (dispatch.rs:797) — slot 10 / slot 21 never considered as pass recipient in release tests.
3. PassKind discriminant order (canonical.rs:446-451) — only hash pin catches; logic-level tests would miss.

##### Systemic patterns

1. **Target-direction uncovered for 3 of 5 off-ball intents**: Press, MarkPlayer, RunOffBall lack target/team-correctness assertions; TrackBack + HoldFormation do have them.
2. **Constant-value mutations escape constant-referencing tests**: `apply_intent_clamps_to_max_speed` reads `MAX_PLAYER_SPEED` at assert time — constant mutation invisible.
3. **`preempt_check` has zero dedicated behavioral unit tests**: most complex unconditionally-executed hot-path function, tested only emergently via GK FSM state-machine tests.
4. **No cross-function relative-ordering tests**: no test asserts `utility_shoot > utility_hold_ball` for a forward in the attack third. Systematic bias miscalibration would be invisible.

##### Recommended new tests (5; for consolidation phase — not authored here)

1. `apply_shoot_bias_pressure_gates_composure_factor` — varies pressure with fixed attrs; closes RED #1.
2. `press_intent_targets_opponent_half` — asserts press target on opponent's pitch side; closes RED #5 part 1.
3. `nearest_teammate_near_includes_last_slot` — passer = slot 9, target near slot 10; closes YELLOW #2.
4. `evaluate_transitions_carrier_exit_to_correct_default` — possession=Some(slot_5)→None roundtrip; closes RED #4.
5. `preempt_check_home_gk_does_not_chase_away_ball` — ball at x=+45, slot 0 GK; closes RED #2.

---

### Track B — Claude architectural-drift audit

**Agent:** `pr-review-toolkit:code-reviewer`
**Scope:** `CLAUDE.md`, `docs/DESIGN_DOC.md`, `docs/adr/*.md`, `docs/specs/*.md`, `docs/MASTER_PLAN.md` — cross-referenced against the actual Rust code in `crates/**/src/**/*.rs`
**Lens:** Every CLAIM made in those docs about what the code does — verify it against the code. Examples to check: ADR-0006 BT-FSM dispatch contract vs `dispatch.rs` reality; ADR-0009 SeedLayer discriminants vs `fw-core/src/seed.rs` reality; ADR-0011 signature stacking policy vs `signature/dispatcher.rs` reality; CLAUDE.md §3 "tech stack — LOCKED" claims vs actual crate Cargo.toml deps; CLAUDE.md §7 banned-primitives claims (`HashMap`, `f32`/`f64`, `Instant::now()`) vs actual `git grep` in sim crates; docs/specs/determinism-gate.md §9 rebaseline protocol vs the actual `canonical_hash.rs` test. Catch drift in either direction: code that does what docs DON'T claim it does + docs that claim things the code DOESN'T do.

[agent appends findings below this line]

---

#### Track B findings — 2026-05-16

**Headline:** drift surface is mostly clean on the load-bearing axes (no determinism contract violations; no banned-primitive leaks into sim/memory/content; rebaseline history is faithfully traced through the 3 pin locations). The drift that exists is dominated by stale **doc→code** counts (Commentary discriminant + crate count never propagated into CLAUDE.md / lib.rs / ADR-0009 enum body) and a missing **code→doc** entry for the T1-15 `preempt_check` GK-chase + nearest-2-chaser logic (the implementation grew a real loose-ball-routing policy that ADR-0006 still describes only as a stubbed-but-planned "single-chaser claim" hook). No P0/P1 architectural bugs found.

##### Drift — `SeedLayer discriminant count: CLAUDE.md §3 and fw-match-sim/src/lib.rs say "8" but code has 9`
- **Severity:** P2
- **Direction:** doc→code (docs claim 8; code has 9)
- **Doc citation:**
  - `CLAUDE.md:58` — "`SeedLayer` enumerates 8 non-overlapping discriminants: `Decision` / `UtilityTieBreak` / `ReactiveInterrupt` / `BallPhysics` / `SignatureTrigger` / `MemoryEvent` / `ScoutObservation` / `ContentBake`"
  - `crates/fw-match-sim/src/lib.rs:20` — "The 8 `SeedLayer` discriminants ensure non-…"
  - `docs/adr/0009-rng-seed-derivation.md:42-65` — enum body still lists only 8 variants; the ADR was supposed to be amended per the 2026-05-16 DECISIONS entry but the source ADR file was never updated to add `Commentary = 0x18`
- **Code reality:** `crates/fw-core/src/seed.rs:103-126` defines 9 `SeedLayer` variants including `Commentary = 0x18`; the docstring at `seed.rs:83` correctly says "9 non-overlapping seed discriminants", and `DECISIONS.md` 2026-05-16 entry records the amendment.
- **Fix shape:** doc-is-stale across three locations. Single-line edit each: (a) CLAUDE.md §3 — change "8" to "9" and add `Commentary` to the list; (b) `fw-match-sim/src/lib.rs:20` — change "8" to "9"; (c) ADR-0009 enum body — add the `Commentary = 0x18` variant with a docstring matching the DECISIONS log, and bump the ADR header to "amended 2026-05-16".
- **Pre-T2 blocker?:** no. Determinism is intact (the canonical-hash regression test would have caught any code-side breakage; the test at `seed.rs:259-269` pins all 9 discriminants).

##### Drift — `Workspace crate count: CLAUDE.md says "~8 crates" but workspace has 9`
- **Severity:** P3
- **Direction:** doc→code
- **Doc citation:** `CLAUDE.md:56` — "Cargo workspace, ~8 crates: `fw-core`, `fw-match-sim`, `fw-content`, `fw-scouting`, `fw-memory`, `fw-replay`, `fw-save`, `fw-tauri`."
- **Code reality:** `ls crates/` returns 9 directories — the listed 8 plus `fw-content-baker` (referenced separately in §3 line "AI: bake-time only" and in Content/RULES.md §7 but absent from the workspace bullet).
- **Fix shape:** update doc to reflect reality. Add `fw-content-baker` to the listed crates and change "~8" to "9".
- **Pre-T2 blocker?:** no. Cosmetic.

##### Drift — `ADR-0006 pre-emption hooks: documented as "stubbed" / "deferred"; code now wires real GK-chase + nearest-2-chaser policy in T1-15`
- **Severity:** P2
- **Direction:** code→doc (code does substantial behavior the ADR doesn't describe)
- **Doc citation:**
  - `docs/adr/0006-bt-vs-fsm-decision-layer.md:27,39` — pre-emption hooks listed only as: "single-chaser claim, foul reaction, set-piece switchover"; framed as "live at the dispatcher, not duplicated per state."
  - `crates/fw-match-sim/src/dispatch.rs:42-46,849-866` — the in-source docstring acknowledges the new scope: "Pre-emption hooks: stubbed to None in -iii-a … wires loose-ball chase for T1-15 … Full pre-emption hook (foul reaction, set-piece switchover, etc.) defers to T2+ per ADR-0006. Loose-ball chase is the only live hook in T1."
- **Code reality:** `dispatch.rs::preempt_check` at lines 867-948 implements three policy decisions never reflected in ADR-0006: (a) loose-ball routing (`possession.is_some()` early-return), (b) GK conditional chase based on `bx_abs >= 42 << 32` + own-side check, (c) nearest-2-outfielder-per-team cap via `closer_count >= 2`. T1-15's MASTER_PLAN row body documents the change; ADR-0006 does not.
- **Fix shape:** update ADR-0006 to reflect reality. Add a "Pre-emption hooks shipped in T1" subsection enumerating loose-ball chase (with the GK-near-own-goal carve-out + nearest-2-chaser cap) and explicitly call out single-chaser/foul/set-piece as T2+. Anything T2-1 author touches in `preempt_check` should re-read the ADR; today the ADR is silent on what they would be modifying.
- **Pre-T2 blocker?:** soft yes. T2-1 is named as the next picker; the loose-ball chase policy is exactly the kind of cross-cutting behavior the ADR is supposed to govern. Folding the doc update into T2-1's first commit is the right shape — not a blocker by itself.

##### Drift — `ADR-0011 mentions cancellation predicates / counterplay; code has no cancellation surface`
- **Severity:** P2
- **Direction:** doc→code
- **Doc citation:** `docs/adr/0011-signature-system.md:93-97` — "The 24-signature catalogue includes defensive signatures with cancellation predicates (e.g. `BodyShieldPressure` cancels nearby `LowCutbackByline` triggers if the defender's positioning predicate fires within 2m). Cancellation = the dispatcher's softmax skips the cancelled signature when re-evaluating."
- **Code reality:** `crates/fw-match-sim/src/signature/dispatcher.rs` (the only signature dispatcher in the tree) has no cancellation/cancellation-predicate code path. `evaluate_signatures` filters by stacking-category lane and softmax-picks; nothing reads a cancellation predicate from neighboring players. `signature/triggers.rs` carries triggers, not cancellers.
- **Fix shape:** depends on T2 plan. Either (a) explicitly note in ADR-0011 Status that cancellation is a Tranche-4 addition (matches today's reality), or (b) keep the ADR aspirational and add a `T2-N — signature cancellation predicates` row to MASTER_PLAN so the gap is tracked. Today neither is true and someone reading ADR-0011 cold would assume cancellation ships.
- **Pre-T2 blocker?:** no.

##### Drift — `Content/RULES.md §2 ID format: dotted-slug carve-out documented; some hand-authored content files predate the carve-out`
- **Severity:** P3
- **Direction:** documented-as-known (this is more a "Codex P2 carve-out the rules already acknowledged"), but worth re-flagging because the carve-out only covers cultures/archetypes/role-affinities/named-signatures — and `fwh.core:role-affinities.default` is the kind of ID a T2 schema add could trivially break if FW-VAL coverage isn't aware.
- **Doc citation:** `.claude/rules/Content/RULES.md` §2 "Hand-authored ID exception (Codex audit P2 carve-out, 2026-05-13)"
- **Code reality:** acceptable; the rules text already captures it. Flagging only because the divergence between procedural (`_00042`) and hand-authored (`.anglo`) ID conventions doubles the FW-VAL coverage surface — worth re-verifying that FW-VAL's duplicate-ID test exercises BOTH naming conventions before T2 mod-overlay work lands.
- **Fix shape:** no doc change needed; QA hook: confirm `fw-content/tests/duplicate_id_test.rs` exercises both conventions, otherwise add a fixture. Out of scope for this audit to verify.
- **Pre-T2 blocker?:** no.

##### Drift — `ADR-0012 §"Three-layer guard" claims the bedrock test pins history through commit eb0b952e; the rebaseline-history docstrings in canonical_hash.rs cite only T1-15 + T1-16 (not the earlier T1-2b-* chain)`
- **Severity:** P3
- **Direction:** code→doc (asymmetric — the RON fixture at `crates/fw-replay/fixtures/0xdeadbeefdeadbeef.ron` carries the full 9-entry rebaseline history with ADR-0012 trigger citations; the in-code `canonical_hash.rs::PINNED_60_TICK` docstring at lines 218-227 only documents the most-recent T1-16 rebaseline). This is the kind of asymmetry ADR-0012 §"Re-baselining workflow" step 3 was meant to prevent ("Both in the same commit").
- **Doc citation:** ADR-0012 §"Re-baselining workflow" step 3, `docs/specs/determinism-gate.md` §9
- **Code reality:** `crates/fw-replay/tests/canonical_hash.rs:218-235` keeps a thin history (only the T1-16 entry visible above the `const PINNED_60_TICK`); `crates/fw-replay/fixtures/0xdeadbeefdeadbeef.ron:1-73` carries the full 9-entry rebaseline log. The two sources are not the same.
- **Fix shape:** update `canonical_hash.rs` docstring to cite the full rebaseline chain like the RON fixture does (or, cleaner, point the docstring at the RON fixture as the single source of truth). Today a reader reading only the .rs file sees one rebaseline; reading only the .ron file sees nine.
- **Pre-T2 blocker?:** no. Forensics gap, not a determinism gap.

##### Drift — `CLAUDE.md §7 banned-primitives claim: HOLDS as documented`
- **Severity:** clean (no drift)
- **Direction:** verification
- **Code reality:** confirmed via grep — zero `HashMap`/`HashSet` in `crates/fw-match-sim/src`, `fw-memory/src`, `fw-replay/src`, `fw-save/src`, `fw-content/src`. Zero `tokio`/`async fn`/`.await` in `fw-match-sim/src` or `fw-memory/src`. Zero `Instant::now`/`SystemTime::now`/`thread_rng()` in sim/memory/content production code. The `f32`/`f64` occurrences in `fw-match-sim/src/dto.rs` are the Tauri/RULES.md §3 sanctioned Q32→f64 DTO boundary; the occurrences in `bt/personality_bias.rs:440,451,525`, `utility/softmax.rs:117-119`, `utility/pitch_control.rs:302-311`, `utility/pressing.rs:67-72` are all inside `#[cfg(test)]` modules (test fixture builders converting f64 literals to Q32 raw bits); the occurrence in `fw-content/src/commentary.rs:425-427` is gated by `#[allow(clippy::float_arithmetic)]` with a doc-comment justification matching the sanctioned Tauri/RULES.md §3 DTO pattern.
- **Pre-T2 blocker?:** no — this is the clean axis.

##### Drift — `ADR-0011 §"Stacking policy" claims signature counterplay is a "real reactive force"; today it is purely stacking-category gating`
- **Severity:** P3
- **Direction:** doc→code (counterplay claim overpromises vs implementation)
- **Doc citation:** `docs/adr/0011-signature-system.md:97` — "Cancellation is a real reactive force, not a stat-line counter — a defender with high `marking + positioning` will materially suppress opposing attacking signatures around them."
- **Code reality:** `dispatcher.rs::evaluate_signatures` only respects `StackingPolicy::Exclusive { category }` (same-category co-fire prevention). No defender attribute reads, no neighbor-radius predicate, no cancellation softmax skip. This is closely related to the cancellation-predicates drift above but distinct — that drift is "doc mentions cancellation predicates exist"; this drift is "doc characterises counterplay's strength as already-shipped".
- **Fix shape:** update ADR-0011 §"Counterplay" to flag the "real reactive force" framing as future-state (T2-N), OR add a row to MASTER_PLAN for the defensive-signature cancellation work.
- **Pre-T2 blocker?:** no.

##### Top 5 drift cases overall
1. CLAUDE.md §3 + fw-match-sim/src/lib.rs + ADR-0009 enum body all still say "8 SeedLayer discriminants" — code has 9 (P2; doc-is-stale, three-location fix).
2. ADR-0006 pre-emption hooks: described as stubbed in the ADR; code wires a substantive 3-policy loose-ball chase in T1-15 (P2; code-grew-past-doc; T2-1 author needs it).
3. ADR-0011 cancellation predicates + "real reactive force" counterplay framing: doc promises a defender-attribute-driven cancellation surface that isn't in the dispatcher (P2/P3 split; doc-overpromise vs ship-state).
4. Workspace crate count: "~8 crates" listed; actual is 9 (`fw-content-baker` absent from the §3 list) (P3; cosmetic).
5. Rebaseline-history asymmetry: `canonical_hash.rs` docstring carries 1 entry, RON fixture carries 9 — both are supposed to update lockstep per ADR-0012 §"Re-baselining workflow" (P3; forensics gap, not a determinism gap).

##### Top 1 architectural drift that's a real bug
**None at P0/P1.** The load-bearing determinism axes (HashMap ban, float ban, clock ban, async ban, RNG-via-seed_fn) all hold. The pinned canonical hashes cross-OS-replicate. The signature dispatcher honors its documented stacking-category contract. The drift is concentrated in **stale documentation** (counts not propagated; ADR-0006 not updated alongside T1-15 implementation; ADR-0011 written aspirationally about counterplay that hasn't shipped). For an 11th-pass audit specifically scoped to architectural drift, the lack of P0/P1 findings is itself the result — the code-to-contract fidelity is strong; only the contract texts need refresh.

---

### Track C — Claude whole-codebase silent-failure sweep

**Agent:** `pr-review-toolkit:silent-failure-hunter`
**Scope:** `crates/fw-match-sim/src/**/*.rs`, `crates/fw-core/src/**/*.rs`, `crates/fw-content/src/**/*.rs` — every file, not commit-scoped
**Lens:** Per-commit silent-failure hunts have happened ~20× this phase, all scoped to the diff. This pass is whole-file-by-whole-file: find silent-failure patterns in code paths NO commit ever touched directly (= they've never been audited). Patterns: `unwrap_or_default()` that swallows real failure modes; `if cond { ... } else { /* nothing */ }` where else should be an error; `.ok()` discarding Result; per-tick mutations with hidden order dependencies; `match` arms that fall through silently; `Option::map` that loses the None case.

#### Track C findings — 2026-05-16

**Headline:** 8 distinct silent-failure patterns found in whole-file scope across `fw-core`, `fw-match-sim`, and `fw-content` (the per-commit hunts have hit none of these). The most load-bearing one is a project-wide policy split: `Q32` operators panic on overflow (Codex Q1 — strictly enforced), but `Tick` operators silently saturate. Two P1s reachable today (Tick saturation + `v_max == 0` fallback in pitch_control); six P2/P3s latent until a specific future scenario (T2-1 tuning RON, long-form sims, mixed-case content packs). No new P0s — the per-commit lens already caught those.

##### `crates/fw-core/src/tick.rs:89-117` — `Tick` arithmetic silently saturates (divergent from `Q32` panic-on-overflow policy)
- **Severity:** P1
- **Pattern:** #8 Q32-adjacent overflow pattern; project-wide discipline drift
- **Code excerpt:**
  ```rust
  impl Add for Tick {
      fn add(self, rhs: Tick) -> Tick { Tick(self.0.saturating_add(rhs.0)) }
  }
  impl Sub for Tick { fn sub(self, rhs: Tick) -> Tick { Tick(self.0.saturating_sub(rhs.0)) } }
  ```
- **Why silent:** `Q32` operators explicitly panic on overflow per Codex Q1 (q32.rs:274-348). `Tick` does the opposite — saturates without any signal. `now_tick - entry_tick` in `tactic_fsm::apply_event:313-314` and `heartbeat_check:406` would silently produce `Tick::ZERO` if `entry_tick > now_tick` (which should be impossible — but the cooldown guard `(now_tick - entry_tick) > 600` would then fail-open, allowing immediate HighPress re-entry on any state where the invariant is violated).
- **Recommended fix:** Switch to `checked_add` + `expect` with a context message that names the invariant, matching the Q32 operator policy. If saturating semantics are genuinely wanted for some call site, expose them as `saturating_add(rhs)` methods like `Q32::checked_add` so the choice is explicit.
- **Reachable today?:** YES — `tactic_fsm` cooldown math runs every event tick; an entry-tick bug elsewhere would silently allow HighPress oscillation.

##### `crates/fw-match-sim/src/utility/pitch_control.rs:193-197` — `v_max == 0` silently produces `Q32::MAX` travel time
- **Severity:** P1
- **Pattern:** #2 Fallback-on-condition that produces a wrong-but-not-erroring result
- **Code excerpt:**
  ```rust
  let travel = if snap.v_max > Q32::ZERO {
      dist / snap.v_max
  } else {
      Q32::MAX
  };
  ```
- **Why silent:** A player with `v_max == Q32::ZERO` is treated as "infinitely slow" rather than an error. Player snapshots flow from `PlayerState` / `PlayerAttributes`; a zero-stamina player or a corrupted attribute load could produce this. Pitch control would then attribute every zone to opponents with no diagnostic, masking the bad attribute. The `Q32::MAX` value also poisons downstream arithmetic — `mean_tau` collapses into a huge value, sigmoid saturates to `0`, and all teammates of that player get arrival-probability `~0`.
- **Recommended fix:** Either (a) panic with a clear message ("PlayerSnapshot has v_max == 0 — invariant: all sprint speeds positive"), or (b) return a typed `PitchControlError::InvalidSpeed { slot }` and propagate. Bare fallback is the worst choice — neither loud nor a safe default.
- **Reachable today?:** UNCLEAR — depends on whether any code path can produce `PlayerSnapshot { v_max: Q32::ZERO, .. }`. The seed paths route through PlayerAttributes which should clamp; no audit has confirmed the clamp covers `to_player_snapshot`.

##### `crates/fw-match-sim/src/utility/softmax.rs:82-84` — `weight_sum <= Q32::ZERO` silently returns top[0] without context
- **Severity:** P2
- **Pattern:** #2 Fallback-on-error masks impossible-state assertion
- **Code excerpt:**
  ```rust
  if weight_sum <= Q32::ZERO {
      return Some(top[0].0);
  }
  ```
- **Why silent:** `exp_q32` is always positive (sigmoid LUT entries ≥ 0; arithmetic above prevents zero). The only way `weight_sum <= 0` is if all weights overflowed or the LUT was somehow corrupted — either an invariant violation worth a panic. Silently returning argmax means a bug in `exp_q32` (e.g. a hypothetical LUT regeneration bug producing negative entries) hides as "always picks the best option" instead of failing visibly.
- **Recommended fix:** `debug_assert!(weight_sum > Q32::ZERO, "softmax weight_sum non-positive — exp_q32 invariant broken")` or return a typed error.
- **Reachable today?:** NO under correct invariants; YES if `exp_q32` LUT drifts. The committed `math_luts.rs` is const, so the only way to trip this is bake-time drift. Catching here would give a second line of defense.

##### `crates/fw-match-sim/src/utility/softmax.rs:48-49` — `sort_by_key` ties resolved by std stability (not by `UtilityTieBreak` seed layer)
- **Severity:** P3
- **Pattern:** #6 Per-tick ordering with hidden std-stability dependency
- **Code excerpt:**
  ```rust
  sorted.sort_by_key(|item| std::cmp::Reverse(item.1));
  ```
- **Why silent:** `sort_by_key` IS stable in std today (TimSort), but the doc-contract only promises stability for `sort` / `sort_by`. If two candidates have identical Q32 utility (possible with tied scores from LUT-truncated math), the tie-break would silently change if std ever swapped algorithms. The `UtilityTieBreak` seed layer exists for exactly this purpose but is not consulted here.
- **Recommended fix:** Either explicitly `sort_by` with `action_id` as the tiebreaker, OR draw from the `UtilityTieBreak` seed layer when ties exist. Document the tie-break policy in the function header.
- **Reachable today?:** YES on the smoke seed but only when two candidates have bit-identical utility, which is rare. Cross-platform std stability is the more theoretical concern.

##### `crates/fw-content/src/runtime.rs:88-107` — `.unwrap_or("")` + `continue` silently skips unrecognized commentary files
- **Severity:** P2
- **Pattern:** #1 Error suppression + #4 match catch-all fallthrough
- **Code excerpt:**
  ```rust
  let stem = path.file_name().and_then(|n| n.to_str())
      .map(|n| n.trim_end_matches(".tracery.json"))
      .unwrap_or("");
  let disc = match stem {
      "kickoff" => MatchEventDiscriminant::KickOff,
      // ...
      other => { let _ = other; continue; }
  };
  ```
- **Why silent:** Two failure modes silently swallowed:
  (1) A file with a non-UTF-8 name produces empty `stem`, matches no arm, gets skipped — `load_commentary_grammars` then errors with `MissingCommentaryGrammar { event_class: KickOff }` (whichever discriminant is checked first), giving a misleading "you're missing kickoff" message when the real problem was bad filename encoding.
  (2) A typo'd filename like `goals.tracery.json` (plural) silently skips and the load fails with "missing goal grammar" — the content author has to figure out the typo without help.
- **Recommended fix:** Replace the `continue` with a `tracing::warn!` naming the path, OR return `ContentLoadError::UnknownCommentaryFile { path }` so the author sees exactly which file is unrecognized. The doc comment says "Unknown filename — log and skip" but `let _ = other;` is not a log.
- **Reachable today?:** YES on any content authoring typo.

##### `crates/fw-content/src/markov.rs:75-86, 86, 191` — doc/code drift: docs claim lowercase normalization that the code never performs
- **Severity:** P3
- **Pattern:** #4 (semantic) — implicit contract violation hiding behind narrative comments
- **Code excerpt:**
  ```rust
  // Line 86:
  let word = word.trim();   // No .to_lowercase()
  // Line 75 doc:
  /// 1. Lowercase each entry (normalizes training data; sampled output is
  ///    then title-cased by the caller or left lowercase as needed).
  // Line 191 doc:
  // (lowercase, since train normalizes to lowercase).
  ```
- **Why silent:** The training loop NEVER calls `.to_lowercase()`. If the corpus contains mixed-case names (e.g. `"McDonald"`, `"O'Brien"`), the bigram table will have separate transition lanes for `M-c` and `m-c`, doubling the state space and biasing sampling toward whichever case dominates. The output then has `to_uppercase` applied to the first char only, leaving any uppercase interior chars unchanged — `MarvinSon` instead of `Marvinson`. Test corpora happen to be lowercase, so the bug ships green.
- **Recommended fix:** Either (a) add `.to_lowercase()` at line 86 to match the documented contract, or (b) update the docs to match the code (and re-audit the title-case logic at line 191 to handle mixed-case interior chars).
- **Reachable today?:** YES if any culture's name banks contain mixed-case entries. The committed `content/sources/cultures/*.ron` should be audited; if they're all-lowercase today, this is latent until a content pack with capitalized names lands.

##### `crates/fw-content/src/commentary.rs:330-343` + `crates/fw-match-sim/src/signature/dispatcher.rs:150` — `tick.to_raw() as u32` silently wraps negative or overlong ticks
- **Severity:** P2
- **Pattern:** #8 silent integer truncation in canonical-RNG site construction
- **Code excerpt:**
  ```rust
  MatchEvent::KickOff { tick, .. } => (tick.to_raw() as u32, SLOT_SENTINEL),
  // ...same pattern for all 6 variants
  // signature/dispatcher.rs:150:
  let tick_u32 = state.tick.to_raw() as u32;
  ```
- **Why silent:** `Tick::to_raw()` returns `i64`. A negative tick (impossible by invariant per ADR-0009) wraps to a huge `u32`; a tick > `u32::MAX` (impossible for a 90-min match — 5.4M ticks; FW could later run season-long sims) silently truncates. Both produce a `site` value that diverges from what the encoder would derive, breaking the canonical RNG-derivation contract silently.
- **Recommended fix:** Either tighten `Tick` to `u32` internally (it's monotonic non-negative and the 5.4M-tick budget fits 23 bits), or use `u32::try_from(tick.to_raw()).expect("Tick fits u32 — ADR-0009 invariant")` to panic loudly on violation.
- **Reachable today?:** NO for the 60-tick smoke seed. YES if a future test or scenario produces a Tick > 2³¹.

##### `crates/fw-match-sim/src/ball_physics.rs:190-193` — `debug_assert!` for `is_well_formed` disables in release builds
- **Severity:** P2
- **Pattern:** #2 fallback-via-disabled-assertion — release-mode silent acceptance
- **Code excerpt:**
  ```rust
  debug_assert!(
      coeffs.is_well_formed(),
      "ball_step called with malformed coefficients: {coeffs:?}"
  );
  ```
- **Why silent:** The comment above explicitly calls out the silent-failure risk: out-of-range coefficients "would silently produce velocity-reversing drag or super-ball bounce and look like a determinism regression". Yet the guard is `debug_assert!`, which compiles out of release. A future caller passing tunable coefficients from RON (T2-1 tuning rebake) would in release silently produce a ball that wraps velocity each tick — and the canonical-hash regression catches it cross-platform but NOT cross-build-profile. If the canonical-hash test happens to run debug and the release shipping binary runs release, drift could escape.
- **Recommended fix:** Promote to `assert!` — `is_well_formed` is a one-shot O(1) check per tick, negligible cost. Matches the Q32 operator policy (panic on invariant violation, both build modes).
- **Reachable today?:** NO under the committed `phase1_seeds` (all values are in-range constants); YES at T2-1 if RON tuning ships malformed coefficients.

#### Top 5 P0/P1 findings (highest ship-impact)

1. **`Tick` saturating arithmetic** (tick.rs:89-117, P1) — silently fails-open on impossible-state arithmetic, easy invariant escape via tactic-FSM cooldown bug.
2. **`v_max == 0` → `Q32::MAX`** (pitch_control.rs:193-197, P1) — corrupts pitch-control outputs without diagnostic.

(No new P0 patterns found in the whole-file sweep beyond what the per-commit audits already caught. The other 6 findings are P2/P3 — real but lower-impact.)

#### Top 3 P2/P3 findings worth opportunistic fix

3. **`tick.to_raw() as u32`** (commentary.rs:330-343 + dispatcher.rs:150, P2) — silent truncation breaks RNG-derivation invariant on long-running sims.
4. **`debug_assert!` on coefficient invariants** (ball_physics.rs:190-193, P2) — guard disabled in release, exactly the silent-failure mode the comment warns about.
5. **Markov doc/code drift on lowercase normalization** (markov.rs:75/86/191, P3) — latent bug until any content pack ships mixed-case names.

#### Patterns observed

- **Saturating-vs-panic policy split across `fw-core` primitives.** `Q32` rigorously panics on overflow (Codex Q1 enforced); `Tick` quietly saturates. This is a CLAUDE.md §10 violation in spirit if not letter — the silent-failure-hunter agent should treat the divergence as a project-wide pattern, not a one-off. Recommendation: align `Tick` with `Q32`'s checked-then-panic policy unless there's a deliberate design reason for saturation (and document it where the impls live).
- **`debug_assert!` for "this would silently corrupt determinism if violated".** ball_physics.rs:190 explicitly states the silent-failure risk in its comment, then guards with debug_assert. tick.rs:79 follows the same pattern. Project rule should be: if the comment says "silent" the guard is `assert!`, not `debug_assert!`. The cost is one branch per tick; the safety is universal.
- **`unwrap_or("")` + `continue` fallthrough in content loaders** (runtime.rs:91-106) — the doc says "log and skip" but no log call exists. Per-commit audits have flagged similar patterns; this whole-file pass surfaces the inherited version of the same anti-pattern.
- **Bare integer casts (`as u32`, `as u8`) in canonical-RNG site construction** — commentary.rs and dispatcher.rs both cast `tick.to_raw() as u32` to pack into the site word. The cast is correct for current tick budgets but is the kind of structural-vs-numeric mismatch that breaks canonical hashes silently when budgets grow. A typed `Tick::to_site_u32() -> Result<u32, TickOverflow>` would surface the contract.

These 8 findings are net-new beyond the per-commit silent-failure hunter passes (which were diff-scoped). The 2 P1s are reachable production paths today; the 6 P2/P3s are latent until specific future scenarios (T2-1 tuning RON, long-form sims, mixed-case content packs).


---

### Track D — Claude test-the-tests dead-test detection

**Agent:** `qa-lead`
**Scope:** `crates/fw-match-sim/tests/**/*.rs`, `crates/fw-match-sim/src/**/*.rs` `#[cfg(test)]` modules, `crates/fw-content/tests/**/*.rs`, `crates/fw-replay/tests/**/*.rs`
**Lens:** For each test, ask: "if I disabled this test, would any OTHER test fail on the same regression?" Find tests that are redundant (same assertion in 2+ places) AND find regression classes that NO test would catch (= coverage hole). Plus: scan for tests that pass for the wrong reason (vacuous patterns: `prop_assume!` that always rejects, `assert!(true)` deadcode, asserts on attribute setters that don't actually exercise behavior). This is the "11th silent-failure verdict" framing but specifically about TEST QUALITY not production-code silent failure.

[agent appends findings below this line]
---

#### Track D findings — 2026-05-16

**Scope covered:** 15 integration test files in `crates/fw-match-sim/tests/`, 6 in `crates/fw-content/tests/`, 1 in `crates/fw-replay/tests/`, 1 in `crates/fw-tauri/tests/`, plus embedded `#[cfg(test)]` modules in `fw-core/src/`. One insta snapshot file.

**Overall verdict:** The test suite is in good health. The T1-9 and T1-18 anti-vacuousness work landed well; every test that matters has a deliberate guard. No catastrophic dead-tests found. Findings cluster in: one permanently ignored snapshot (P1), two stale documentation claims (P2), one rebaseline-procedure blind spot creating an undocumented third update obligation (P2), one semantic mismatch in the argmax reference (P2), and one confirmed-subsumed test (P3).

---

##### `crates/fw-replay/tests/canonical_hash.rs:479-491` — `smoke_seed_final_state_snapshot` — permanently ignored since T0-7; zero coverage

- **Severity:** P1
- **Angle:** Vacuous (test never runs; no `.snap` file exists)
- **Test name + claim:** human-diffable insta snapshot of the 60-tick final state, complementing the BLAKE3 hash pin.
- **Reality:** `#[ignore = "snapshot baseline created alongside first CI green hash"]` has been on this test since T0-7. The canonical hash has been re-baselined 14 times since then. No `.snap` file exists in `crates/fw-replay/tests/snapshots/`. The test produces zero regression coverage.
- **Recommended fix:** remove `#[ignore]`, run `cargo insta review`, commit the `.snap` file. The human-diffable snapshot is genuinely valuable — it makes canonical-state drift readable (positions, score, events) rather than opaque hex bytes. Alternatively, delete the test body entirely. Leaving it `#[ignore]`d permanently is the worst option: it implies coverage that does not exist and the docstring will mislead future authors.
- **What regression class WOULD slip past:** drift in human-readable MatchState fields that the BLAKE3 hash catches as opaque bytes but that a human reviewer would understand far more clearly from a Debug snapshot.

---

##### `crates/fw-match-sim/tests/behavior_proptest.rs:12-14` — `gk_home_stays_near_own_goal_95pct_of_ticks` — module docstring claims "ball must move" but test checks tick counter

- **Severity:** P2
- **Angle:** Coverage-hole (docstring implies a stronger anti-vacuousness guarantee than the test provides)
- **Test name + claim:** module-level docstring line 12: "Anti-vacuous: ball must move between tick 0 and tick 60, proving tick_match was not a no-op."
- **Reality:** the actual assertion at lines 208-213 is `state.tick == Tick::from_raw(60)`. Tick advances unconditionally as the first operation of `tick_match` — an implementation that increments the counter and does nothing else satisfies this guard. Ball position is never compared. The docstring was not updated when the T1-15 rewrite switched from ball-position delta to tick-counter (the in-code comment at line 201 explains why the switch was made; the module-level docstring was missed).
- **Recommended fix:** update the module-level docstring lines 11-15 to say "Anti-vacuous: tick counter must advance to exactly 60" and remove the stale "ball must move" claim. The test body is correct; only the top-level doc is wrong.
- **What regression class WOULD slip past:** a tick_match that advances the counter but integrates no physics still passes both GK invariants (GK at initial position = within 30m = 61/61 ticks). The canonical hash test provides the system-level safety net; this is a defense-in-depth gap.

---

##### `crates/fw-match-sim/tests/utility_proptest.rs:173-191` — `softmax_argmax_at_zero_temperature` — reference `max_by_key` semantics diverge from production `sort_by_key(Reverse)` on tied inputs

- **Severity:** P2
- **Angle:** Coverage-hole (tie-breaking behavior untested; "avoids ties" comment not enforced by the strategy)
- **Test name + claim:** near-zero temperature must return the argmax for all valid utility inputs. Comment on line 174 says "avoids ties."
- **Reality:** the strategy generates `u1, u2, u3` independently from `1i64..=(1i64 << 30)` — values CAN be equal. When u1 == u2 > u3: `max_by_key` (the test's reference for `best`) returns the LAST equal element by iterator semantics; `pick_top_n_softmax` uses `sort_by_key(Reverse(utility))` stable sort so `top[0]` is the FIRST equal element. Reference returns id=2; function returns id=1. A correct implementation false-fails the test on any tied-top-two case. P(u1==u2) per proptest case is ~1/2^30, negligible in practice, but the semantic mismatch is real and the comment "avoids ties" is aspirational, not enforced.
- **Recommended fix:** add `prop_assume!(u1 != u2 && u2 != u3 && u1 != u3)` to actually enforce the assumption, OR rewrite the `best` reference computation to match stable-sort-descending semantics. Add a separate deterministic test that explicitly pins the tie-breaking contract.
- **What regression class WOULD slip past:** a change to softmax tie-breaking (e.g., switching the production path to `max_by_key`) would make the test consistently pass for tied inputs but fail for non-tied ones — a confusing debugging signal.

---

##### `crates/fw-content/tests/fixtures_load.rs:235-296` — `signature_load_does_not_drift_canonical_hash` — redundant hash pin creates undocumented third rebaseline obligation

- **Severity:** P2
- **Angle:** Redundant (same BLAKE3 hash pinned in two separate test files; documented rebaseline procedure covers only two of three locations)
- **Test name + claim:** verifies loading signature RON files does not affect the canonical match-state hash.
- **Reality:** the `EXPECTED` constant at lines 263-267 is `[0xfc, 0xcc, 0xb8, 0x40, ...]` — byte-identical to `PINNED_60_TICK` in `canonical_hash.rs` (both contain `fcccb840...a751`). The rebaseline procedure in `docs/specs/determinism-gate.md §9` documents two mandatory update locations: `PINNED_60_TICK` and `0xdeadbeefdeadbeef.ron`. A rebaseline author following documented procedure updates two locations and leaves this constant stale, causing the test to fail with "unexpected drift" when the drift was intentional and authorized.
- **Recommended fix:** replace the hardcoded byte array with a reference to the RON fixture (load and parse `crates/fw-replay/fixtures/0xdeadbeefdeadbeef.ron`, extract `expected_hash`). The RON fixture is always updated per the documented procedure and is already the third location in the three-location rebaseline chain. This collapses to two locations. Update `docs/specs/determinism-gate.md §9` to reflect the change.
- **What regression class WOULD slip past:** none in normal operation. The false-alarm failure on an authorized rebaseline is the problem, not a missed real regression.

---

##### `crates/fw-match-sim/tests/bt_runner_proptest.rs:147-171` — `outfield_players_move_to_formation_after_decision` — strictly subsumed by stronger test in same file

- **Severity:** P3
- **Angle:** Redundant (assertion fully covered by `decision_counter_increments_monotonically`)
- **Test name + claim:** at least one outfield player (non-GK) has `decision_counter() > 0` after 30 ticks.
- **Reality:** `decision_counter_increments_monotonically` (line 109, same file) runs 60 ticks and asserts ALL 22 players have `decision_counter() >= 3`. That strictly implies this test. Any regression caught here is also caught there.
- **Recommended fix:** delete this test. No coverage is lost. If the intent is to separately guard that non-GK players receive decisions, add a named sub-assertion inside `decision_counter_increments_monotonically` with explicit outfield-slot ranges.
- **What regression class WOULD slip past:** none beyond what `decision_counter_increments_monotonically` already catches.

---

##### `crates/fw-match-sim/tests/behavior_proptest.rs:504-579` — `no_player_sustained_sprint_over_threshold_for_4_seconds` — documented vacuous; acceptable as future regression-guard

- **Severity:** P2 (documented; acceptable; needs a companion test plan)
- **Angle:** Vacuous (explicitly documented VACUOUSLY TRUE while MAX_PLAYER_SPEED = 8 m/s < 12 m/s threshold)
- **Test name + claim:** no player sustains >12 m/s for 4 full seconds.
- **Reality:** docstring is honest about the vacuousness. The anti-vacuousness guard at lines 547-554 asserts "at least one player has non-zero speed" — proves tick_match is alive but not that the 12 m/s sprint boundary is near-reachable. With total_ticks = window_size = 240, there is exactly one window; the guard proves "sim runs" but not "sprint property is exercised."
- **Recommended fix:** add a docstring note that when `MAX_PLAYER_SPEED` is bumped above 12 m/s, a companion unit test should be added that constructs a player with sustained velocity >12 m/s for 240 ticks and confirms the invariant fires. No change to the current test body required.
- **What regression class WOULD slip past:** MAX_PLAYER_SPEED bump to >12 m/s without a sustained-sprint throttle. Until that bump, the test catches nothing a simpler speed-bound check wouldn't catch.

---

##### Top 5 vacuous-test findings

1. **`canonical_hash.rs::smoke_seed_final_state_snapshot`** (P1) — permanently `#[ignore]`d since T0-7; zero coverage; no `.snap` file. Activate or delete.
2. **`behavior_proptest.rs` GK docstring** (P2) — module docstring claims "ball must move"; test checks tick counter. Stale documentation of the anti-vacuousness guarantee.
3. **`utility_proptest.rs::softmax_argmax_at_zero_temperature`** (P2) — "avoids ties" not enforced; reference `max_by_key` diverges from production stable-sort semantics on tied inputs.
4. **`behavior_proptest.rs::no_player_sustained_sprint_over_threshold_for_4_seconds`** (P2) — documented vacuous; anti-vacuousness guard proves sim is alive but not that the sprint boundary is exercised. Acceptable state; needs a companion test plan for the MAX_PLAYER_SPEED bump.
5. **`bt_runner_proptest.rs::outfield_players_move_to_formation_after_decision`** (P3) — strictly subsumed by `decision_counter_increments_monotonically` in the same file.

##### Top 3 coverage-hole findings

1. **GK docstring/reality mismatch** (P2) — module docstring says "ball must move"; test checks tick counter only. A tick_match that advances the counter but does no physics passes both GK invariants.
2. **Softmax tie-breaking behavior** (P2) — no test pins the contract when two utilities are equal at near-zero temperature. Production path has well-defined semantics (first element after Reverse stable sort); test reference diverges for ties.
3. **`fixtures_load.rs` rebaseline gap** (P2) — third hash-pin location undocumented in the rebaseline procedure; future authorized rebaseline produces a false-alarm failure in this test.

##### Top 3 redundant tests

1. **`fixtures_load.rs::signature_load_does_not_drift_canonical_hash`** — pins the same BLAKE3 hash as `canonical_hash.rs::PINNED_60_TICK`. Third update obligation not listed in `docs/specs/determinism-gate.md §9`. Collapse to two locations.
2. **`dispatch_proptest.rs::decision_counter_never_decrements`** — monotonicity property covered by `bt_runner_proptest.rs::decision_counter_increments_monotonically` (60-tick, all-player, with liveness floor). Borderline defense-in-depth; keep but document the relationship.
3. **`bt_runner_proptest.rs::outfield_players_move_to_formation_after_decision`** — strictly subsumed by `decision_counter_increments_monotonically`. Delete without coverage loss.

##### Insta snapshot health verdict

**One live snapshot:** `crates/fw-match-sim/tests/snapshots/match_event_snapshot__smoke_seed_60_tick_match_events.snap` — HEALTHY. Pins 6 events (KickOff, 3 Passes, 1 Shot, FullTime) for the smoke seed at 60 ticks. The test has a correct anti-vacuousness pre-check (non-empty + KickOff-first + FullTime-last) before `insta::assert_debug_snapshot!`. A behavioral regression that reorders or inserts events produces a human-readable diff. The absence of a Goal event in 60 ticks is correct per current ball physics timing (shot at tick 26, ~37 ticks travel time, goal would appear around tick 63). The snapshot goes stale only if `SMOKE_TICK_COUNT` changes without a `cargo insta review` pass — no enforcement exists for that coupling, but that is inherent to snapshot tests.

**One permanently ignored snapshot:** `canonical_hash.rs::smoke_seed_final_state_snapshot` — no `.snap` file, zero coverage (P1 finding above). The `match_event_snapshot.rs` snapshot covers the event stream; the full-state Debug snapshot has never been activated.


---

### Track E — Codex adversarial red-team

**Owner:** Codex CLI (separate session)
**Lens** (red-team mindset, different from "find bugs"):

1. **Break the canonical hash silently.** Find a way to mutate canonical state (`MatchState` field reorder, encoder field-order change, new field that bypasses the unique-attr test) such that the canonical hash changes but `cargo test` doesn't catch it. Or: find a way to make two semantically-different states hash to the same value (encoder injectivity bug).
2. **Make a content pack pass validation while being semantically invalid.** Try `cargo run --bin fw-content-baker -- validate` on hand-crafted RON: duplicate IDs that bypass `ContentLoadError::DuplicateId`; malformed `ManagerArchetypeId` that slips the serde post-parse `try_new`; `DanglingReference` that escapes the cross-ref validator; banned-terms catalog entries that escape the lint via sentinel abuse.
3. **Make a malicious mod overlay take over the base pack** silently — content load order, ID collision, or `overrides:` field abuse that doesn't fail loudly.
4. **Find a determinism leak** that escaped all prior audits — anything in `fw-match-sim` / `fw-core` / `fw-content` that could platform-vary (libm float path, OS-dependent collection iteration, time-dependent state) without triggering the cross-OS canonical-hash regression test.

Append findings to Track E section in `docs/audits/post-t1-ultimate-review-2026-05-16.md`.

[Codex appends findings below this line]

#### Track E findings — 2026-05-16

**Codex verdict:** no new canonical-hash or deterministic-runtime P0 found. `python3 scripts/determinism-audit.py` is clean, and I did not find a current `HashMap` / clock / thread RNG / rayon / `unsafe` leak in the canonical sim path. The strongest red-team issues are content-validation gaps: I can make bad content pass the current validation story more easily than I can break the match hash.

1. **P1 — `fw-content-baker validate` still gives a false "FW-VAL passed" signal for licensed/banned/generated text.**  
   `run_validate` says it runs every available validator, but it only loads `ContentStore` and checks role-affinity sums plus player attribute ranges: `crates/fw-content-baker/src/main.rs:201-230`, `crates/fw-content-baker/src/main.rs:241-270`. The actual banned-term / licensed-data / cliche validators all return `NotImplemented`: `crates/fw-content-baker/src/validators.rs:61-120`. This becomes a red-team bypass because `scripts/lint-banned-terms.py` excludes `/content/baked/` on the assumption that baker-side validation runs separately: `scripts/lint-banned-terms.py:131-146`. A generated baked pack with real-club names or LLM slop can therefore sit outside the lint path and outside `fw-content-baker validate`.
   **Fix shape:** either make `validate` fail while those three validators are unimplemented, or rename the current command to `validate-structural` and reserve `validate` for full FW-VAL. When T2-3 lands, wire `validate_fragment` into the bake path and add a fixture under `content/baked/` proving a licensed name fails.

2. **P1 — Several stable content IDs are exact strings, despite docs claiming load-time format validation.**  
   `Culture.id` documents a regex and says it is "Validated at load": `crates/fw-content/src/runtime.rs:168-172`. The loader inserts it directly as a `String`: `crates/fw-content/src/runtime.rs:553-561`. Same pattern for `TacticalArchetype.id`, `RoleAffinityTable.id`, and `PlayerTemplate.qualified_id`: `crates/fw-content/src/runtime.rs:248-258`, `crates/fw-content/src/role_affinity.rs:211-218`, `crates/fw-content/src/player.rs:52-65`. Duplicate detection catches only byte-identical keys: `crates/fw-content/src/runtime.rs:431-450`. That leaves uppercase, whitespace, Unicode confusables, wrong entity kind, and schema-version drift as semantically-invalid-but-loadable content.
   **Fix shape:** add newtypes for every durable content ID class, mirroring `SignatureId` / `ManagerArchetypeId`. Deserialize through `try_new`, normalize nothing implicitly, and reject anything outside one ASCII canonical form. Add tests for uppercase, trailing space, Unicode lookalike, wrong kind prefix, and exact duplicate.

3. **P1 — Player signature candidates can reference missing definitions and silently degrade to no signature behavior.**  
   `ContentStore::load_sources` validates manager → tactical-archetype references only: `crates/fw-content/src/runtime.rs:681-707`. It does not validate `PlayerTemplate.signature_candidates` against `store.signature_definitions`. At runtime the dispatcher treats a missing definition as "skip this candidate" and keeps going: `crates/fw-match-sim/src/signature/dispatcher.rs:82-89`. A broken content pack can therefore remove a player's signature identity without failing load, verify, or dispatch.
   **Fix shape:** after loading players and signatures, collect every `signature_candidate.signature_id` and return `ContentLoadError::DanglingReference` when no definition exists. Keep the runtime `continue` only for future mod-compat if an explicit "unknown mod signature" policy exists.

4. **P1 — Category-A banned terms can be hidden inside scanned runtime files with sentinel blocks.**  
   The linter permits `<!-- ui-lint:ignore-start ... -->` blocks as the only Category-A escape path: `scripts/lint-banned-terms.py:23-32`. It scans `.json` and `.ron`: `scripts/lint-banned-terms.py:148`, then strips every sentinel block before matching: `scripts/lint-banned-terms.py:193-215`. That means a content author can put the sentinel markers in a valid JSON string or RON comment and hide hard-banned shipped text. This is acceptable for banned-term catalog docs; it is too permissive for runtime content.
   **Fix shape:** disallow sentinel blocks under `content/sources/**`, `content/baked/**`, and `frontend/src/**`. If runtime content needs a one-off exception, require the stricter same-line Category-B style with reviewer metadata and block Category-A entirely.

5. **P2 — Mod-overlay docs promise two different override semantics, while runtime support is absent.**  
   `Content/RULES.md` says mods override by explicit `overrides:` field: `.claude/rules/Content/RULES.md:66-77`. `content/README.md` says per-file last-writer-wins replacement: `content/README.md:41-55`. Runtime does neither today; `load_baked` delegates to `load_sources` and leaves mod load order as TODO: `crates/fw-content/src/runtime.rs:710-718`. `SaveV1` has `content_pack_version` but no `mod_load_fingerprint`: `crates/fw-save/src/lib.rs:37-45`. There is no active takeover path yet because mods are not loaded, but the written contract is already split before implementation.
   **Fix shape:** before T2 mod work, pick one override model in an ADR. Until then, fail loudly if `content/mods/` exists rather than silently ignoring it. When implemented, include mod ID/version/file-hash in save fingerprints and forbid sim-bearing overrides unless explicitly whitelisted.

6. **P2 — Tracery grammar validation is shallow enough for runaway or low-variety generated content.**  
   `CommentaryGrammarBank::try_from_map` only proves every loaded event discriminant has a non-empty `origin`: `crates/fw-content/src/commentary.rs:93-115`. Rendering then hands the merged rules straight to `tracery::Grammar::flatten`: `crates/fw-content/src/commentary.rs:448-479`. There is no cycle check, max expansion depth, max output length, or minimum-origin-variant check. The seed grammar files outside the loaded commentary bank already use single-entry origins: `content/sources/grammars/headlines.tracery.json:4`, `content/sources/grammars/manager-quotes.tracery.json:4`.
   **Fix shape:** add a grammar validator before accepting baked/LLM grammar packs: reference resolution, DFS cycle detection, max depth, max rendered length, and minimum variant count for player-facing origin rules.

7. **P2 — Canonical encoding is well-covered now, but still lacks an automatic "new `MatchState` field must be encoded" tripwire.**  
   I found targeted tests for recent canonical additions: decision slots, interrupt cooldowns, tactic state, signatures, match events, and possession are all covered around `crates/fw-match-sim/src/canonical.rs:813-1012`. Current state looks encoded. The remaining attack path is future drift: `MatchState` owns many canonical fields in `crates/fw-match-sim/src/lib.rs:163-315`, while `encode_match_state` is a hand-written field list in `crates/fw-match-sim/src/canonical.rs:224-342`. A new field can be added to `MatchState` and omitted from the encoder; pinned hashes still pass because the omitted field never enters the bytes.
   **Fix shape:** add a MatchState-level mutation harness that clones an initialized state, mutates every canonical field through a table of mutators, and asserts the canonical bytes change. Treat adding a `MatchState` field without adding a mutator as a compile/test failure.

**Friendly scoreboard seed:** Codex Track E adds 4 P1s + 3 P2s. Property Track F below found no new shrunk failures. Claude Tracks B/C currently own the scary core-sim P1s; Codex is winning the content-red-team lane, not the sim-math lane.

---

### Track F — Codex property explosion (optional; can fold into E)

**Owner:** Codex CLI (separate session, can run sequentially with E or in parallel)
**Lens:**

1. **Bump proptest CASES from default 256 → 10,000** (via `PROPTEST_CASES=10000 cargo test --release`) on the key invariants:
   - `crates/fw-match-sim/tests/behavior_proptest.rs` (4 positional invariants from T1-9)
   - `crates/fw-match-sim/tests/separation_proptest.rs` (7 PlayerSeparation invariants from T1-2b-iii-d)
   - `crates/fw-match-sim/tests/ball_mutation_proptest.rs` (4 ball mutation invariants from T1-3.5)
   - `crates/fw-match-sim/tests/match_event_proptest.rs::events_chronological` (T1-4a)
2. **Report any new shrunk failures.** With 40× the proptest sampling, any 1-in-thousands rare seed will surface. The current `behavior_proptest.proptest-regressions` file has 6 saved seeds (3 from T1-18 threshold investigation + others). Anything new is a real regression-class signal we haven't seen.
3. **Bump intra-process determinism counts:** `smoke_seed_runs_100_times_produce_one_hash` → 10,000 runs; `extended_seed_runs_10_times_produce_one_hash` → 1,000 runs. Confirm "same seed → one hash" holds at the 10⁴ scale not just 10²-10³.

Append findings to Track F section.

[Codex appends findings below this line]

#### Track F findings — 2026-05-16

**Result:** no new proptest failures at 10,000 cases on the requested integration targets. I ran with `PROPTEST_DISABLE_FAILURE_PERSISTENCE=1` so any failure would report without writing new regression files.

Commands run from `/Users/vibelogic/dev/football`:

```sh
PROPTEST_CASES=10000 PROPTEST_DISABLE_FAILURE_PERSISTENCE=1 cargo test -p fw-match-sim --release --test behavior_proptest
PROPTEST_CASES=10000 PROPTEST_DISABLE_FAILURE_PERSISTENCE=1 cargo test -p fw-match-sim --release --test separation_proptest
PROPTEST_CASES=10000 PROPTEST_DISABLE_FAILURE_PERSISTENCE=1 cargo test -p fw-match-sim --release --test ball_mutation_proptest
PROPTEST_CASES=10000 PROPTEST_DISABLE_FAILURE_PERSISTENCE=1 cargo test -p fw-match-sim --release --test match_event_proptest events_chronological
PROPTEST_CASES=10000 PROPTEST_DISABLE_FAILURE_PERSISTENCE=1 cargo test -p fw-match-sim --release --test match_event_proptest
cargo test -p fw-replay --release --test canonical_hash
python3 scripts/determinism-audit.py
```

Observed results:

- `behavior_proptest`: 4/4 passed, 10,000 cases, 20.39s.
- `separation_proptest`: 7/7 passed, 10,000 cases, 28.62s.
- `ball_mutation_proptest`: 3/3 passed, 10,000 cases, 0.08s.
- `match_event_proptest events_chronological`: 1/1 passed, 10,000 cases, 1.14s.
- Full `match_event_proptest`: 2/2 passed, 10,000 cases, 3.28s.
- `canonical_hash`: 8 passed, 1 ignored. The ignored test is still the documented snapshot baseline test; the pinned hash and no-ignore guard passed.
- `scripts/determinism-audit.py`: clean.

**Determinism-count escalation caveat:** the requested 10,000 smoke reruns and 1,000 extended reruns cannot be parameterized read-only. The loop counts are hardcoded in `crates/fw-replay/tests/canonical_hash.rs:325-353` and `crates/fw-replay/tests/canonical_hash.rs:593-615`. Cargo test filters can select the tests, but cannot change `100` to `10_000` or `10` to `1_000` without editing source. I did not edit source for this audit.

**Recommended follow-up:** make those counts env-driven, defaulting to the current cheap values in CI, e.g. `FW_DETERMINISM_SMOKE_RUNS` and `FW_DETERMINISM_EXTENDED_RUNS`. Then Track F can run the true 10k/1k in-process stress without patching test code.

---

## Consolidated verdict

### Overall: ACCEPT for T2 start. Real findings, none phase-blocking.

All 6 tracks returned with substance. No P0. 9 P1 across 4 distinct concern surfaces. **Codex Track F is the most reassuring single finding of the whole audit: 10,000-case proptest sweeps on 13 invariants — zero new failures.** Combined with the per-commit silent-failure streak (9 clean before T1-15 broke it via scope-creep, not real defects), the sim's core determinism + property surface is well-tested at depth, not just breadth.

Findings break into 5 distinct concern surfaces. Severity-sorted:

#### Surface 1 — Content-validation gap (highest exploitability; Codex E owns this)

**`fw-content-baker validate` is a misleading green signal.** Codex E #1 + E #3 + E #4 converge: `validate` reports success while banned-term / licensed-data / cliche validators return `NotImplemented`; `scripts/lint-banned-terms.py` excludes `content/baked/` on the assumption baker-side validation runs; signature_candidates can reference missing definitions and silently degrade. Net result: a hypothetical T2 LLM bake could produce real-club names or licensed phrases that pass every gate the project ships today. Two distinct fixes needed: rename `validate` → `validate-structural` (or fail it while validators are stubbed) AND drop the `content/baked/` lint exclusion. Plus stop the sentinel-block abuse path in runtime content paths.

**Affects:** T2-3 content baker pipeline. Not phase-blocking at T1 because no baker runs yet. Must land before T2-3 closes OR T2-3's row-body owes a forward note about the validation gap.

#### Surface 2 — `preempt_check` is the single hottest under-tested area (cross-track convergence: A + B + D + E)

Three independent agents found related symptoms of the same root cause — T1-15 grew `preempt_check` from "stubbed `None`" to a real 3-policy loose-ball-chase / GK-conditional-chase / nearest-2-cap implementation, but:

- **Track A**: zero behavioral unit tests for `preempt_check` (`dispatch.rs:867-948`). GK own-side flip undetectable; chaser-count policy undetectable; GK chase threshold undetectable. 4 of A's 5 RED findings live here.
- **Track B**: ADR-0006 still describes pre-emption as "stubbed" / "deferred"; T2-1 author will land on stale docs.
- **Track E**: Codex didn't flag this directly but didn't find a sim-side P0 either — the red-team came up cold on canonical-hash silent breaks, which is itself a positive signal about the encoder.

**Affects:** T2-1 will touch `dispatch.rs` extensively (full BT runner with 20-30 archetypes); landing T2-1 on top of an untested `preempt_check` substrate is the highest carry-forward risk into T2.

**Fix shape:** one row before T2-1 (call it `T1-19` or fold into T2-1's spec) adding 4-5 behavioral unit tests on `preempt_check` per Track A's "Recommended new tests" list (`preempt_check_home_gk_does_not_chase_away_ball` + the 4 others). Plus an ADR-0006 amendment via `/log-decision` documenting the loose-ball-chase scope expansion that T1-15 already shipped. Both small.

#### Surface 3 — `Tick` arithmetic policy violation (Track C P1)

`Tick` newtype saturates on overflow (`saturating_add` / `saturating_sub`); `Q32` panics on overflow per project policy. Tactic-FSM cooldown math (`tactic_fsm.rs:271-289`) can silently fail-open on `entry_tick > now_tick` invariant violations, allowing HighPress oscillation. Net divergence from the Q32 panic-on-overflow discipline; ~50 lines of code, single newtype, fix is to align `Tick` operators with `Q32`'s checked-then-panic discipline.

**Affects:** any future code that consumes `Tick` arithmetic. Currently no PRODUCTION exploit reachable; the failure mode is a hypothetical future cooldown bug that wouldn't surface as a regression — it'd surface as "HighPress never expires" which would show up in T2-1 calibration as a tactical-FSM bug we couldn't trace.

**Fix shape:** one focused row (~30 LoC + a unit test). Worth landing before T2-1's tactic-FSM expansion touches `Tick` arithmetic more heavily.

#### Surface 4 — Test-suite-vs-spec drift (cross-track: A + C + D)

Three different "test claims to guard X; doesn't actually guard X" patterns surfaced independently:

- **Track A**: `MAX_PLAYER_SPEED` constant mutation invisible to its own clamp test (test reads the const at assert time)
- **Track C**: `softmax_argmax_at_zero_temperature` reference impl uses `max_by_key` while production uses `sort_by_key(Reverse)` — diverge on tied inputs, the "avoids ties" comment doesn't enforce the precondition
- **Track D**: `smoke_seed_final_state_snapshot` `#[ignore]`d since T0-7 with no `.snap` file ever (implied coverage never existed across 14 rebaselines)
- **Track D**: third hash-pin location in `fixtures_load.rs` not in `docs/specs/determinism-gate.md §9` 4-location protocol (actually 5 locations — protocol undercounts)

**Affects:** test-quality maintenance debt. Each individual issue is small but the PATTERN suggests a class of "tests written to look thorough but not actually exercising what they claim." Worth flagging as a discipline note in `/next` skill (subagent prompts could include "if your test reads a named constant at assert time AND that constant is what you're testing, you've written a vacuous test").

**Fix shape:** mostly inline cleanup during T2 rows; the only one urgent enough to row-ify is the snapshot ignore (delete OR activate) + fix the documented-vs-actual hash pin count in the determinism-gate spec (5 not 4).

#### Surface 5 — Documentation drift (Track B P2s only)

3 doc-stale items: SeedLayer count 8→9 in CLAUDE.md + lib.rs + ADR-0009 enum body; ADR-0006 vs T1-15 preempt_check scope (covered by Surface 2 above); ADR-0011 reactive surface gap.

**Fix shape:** single docs-only commit; trivial.

### Property explosion (Codex F) — strongest positive signal of the audit

13 invariants × 10,000 proptest cases each = 130,000 sampled cases on top of the per-commit 256-case defaults that ran during the phase. **Zero new shrunk failures.** The proptest harness at default CASES=256 is well-tuned; bumping to 10K doesn't surface anything new. Strong evidence the sim's property surface holds at depth, not just breadth.

Codex F's one procedural finding (10K/1K determinism rerun counts are hardcoded in canonical_hash.rs:325 + :593 — can't be parameterized read-only) is a worthwhile quality-of-life improvement: env-driven counts (`FW_DETERMINISM_SMOKE_RUNS` / `FW_DETERMINISM_EXTENDED_RUNS`) defaulting to current cheap values in CI. Doesn't change empirical confidence; lets future audits push harder without source edits.

### Recommended new MASTER_PLAN rows (3 — none are T2-blockers)

1. **T1-19** — `preempt_check` behavioral tests + ADR-0006 amendment (Surface 2). 4-5 unit tests + ADR amendment via `/log-decision`. Worth landing BEFORE T2-1 dispatches a sim-Rust subagent, given T2-1 will touch `dispatch.rs` extensively.
2. **T1-20** (or T2-3-blocker) — fail `fw-content-baker validate` while stubbed validators return `NotImplemented` + drop `content/baked/` exclusion from `scripts/lint-banned-terms.py` + add signature_candidates dangling-reference check at load time + close sentinel-block escape in runtime paths (Surface 1). Must land before T2-3 closes; could land NOW as a content-validation hardening pass.
3. **T1-21** (or T2-1-precondition) — `Tick` arithmetic align-to-Q32-panic-on-overflow policy + companion unit test (Surface 3). Small; could fold into T2-1's spec body as a precondition.

Plus 2 small cleanups (not row-worthy on their own; fold opportunistically):
- 1 docs-only commit closing Surface 5's doc-staleness items
- Env-driven determinism rerun counts per Codex F caveat

### Workflow-incident discipline check — first real test of the post-T1-15 hardening

All 4 Claude subagents were dispatched with the mandatory boilerplate from the post-T1-15 `/next` skill hardening. **Discipline held**: all 4 returned with read-only audits, no commits, no file edits outside the shared review file. Track A's subagent specifically reported it lacked Write/Edit tools and asked main thread to append on its behalf — exactly the "scope-expansion-needed" escalation pattern the boilerplate prescribed. The hardening works as designed.

### Cross-tool consolidation experiment — Claude + Codex parallel against shared file

This was the first audit where Claude main-thread orchestrated subagents AND Codex ran in parallel writing to the same shared file. The split worked well:
- Claude tracks (A-D) found systemic patterns + breadth coverage
- Codex tracks (E-F) found the highest-impact single bug (content-validation gap) + the strongest positive signal (10K-case proptest sweeps clean)
- Cross-track convergences emerged organically — `preempt_check` was flagged by A + B from independent angles; constant-mutation-invisible was flagged by A + C + D as a class

Format worked. Worth using again for future cross-phase audits.

### Verdict

**ACCEPT.** T2 unblocked. The 3 recommended new rows are real follow-ups but none gate T2-1 dispatch. T1-19 (preempt_check tests) is the strongest pre-T2-1 candidate; T1-20 is the strongest pre-T2-3 candidate; T1-21 (Tick policy) is opportunistic.

The codebase is in better shape than the audit-finding count suggests. The 9 P1s span 4 surfaces and 3 of them are "do this before phase X starts" rather than "this is broken today." The single most exploitable finding (Codex E #1 fw-content-baker validate misleading green) is a Phase T2-3 problem, not a T1-close problem.

---

## Workflow incident discipline (this audit is the first real test of the post-T1-15 hardening)

The 4 Claude subagents below are dispatched with the new mandatory boilerplate from `.claude/skills/next/SKILL.md` "Subagent discipline" section (post-T1-15 hardening). All 4 are READ-ONLY (no source code mutation, no commits) — the only file they edit is this review file, appending to their named Track section. Main thread will verify on return that none of them mutated other files.

This is also the first audit where Claude + Codex run in parallel against the same shared review file — a deliberate experiment in cross-tool consolidation.

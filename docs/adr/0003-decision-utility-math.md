# ADR-0003 — Decision-utility math primitives

**Status:** Proposed

**Date:** 2026-05-13

**Decider:** osagberg (+ Codex review at the T1 phase gate)

---

## Context

T1-2b lands the 22-player behavior-tree runner. On-ball events (shot / pass / dribble / hold) are scored by a utility selector; off-ball positioning consumes influence maps derived from pitch-control kinematics. The math has to lock before BT code lands because:

1. The shapes of `xg_utility`, `xt_delta`, `pitch_control`, `pressing_intensity` constrain every BT leaf API.
2. Every formula must be Q32.32-tractable. A late discovery that we need `f64::erf` is a sim-wide refactor.
3. The pinned canonical-state hash (`crates/fw-replay/tests/canonical_hash.rs`) goes load-bearing the moment T1-2b lands. Changing math post-T1-2b means re-pinning hashes — the churn the determinism gate is designed to forbid.

Two scope reframes change the menu vs. the original synthesis (per `docs/DESIGN_DOC.md` §1 "Scope ambition" 2026-05-13, and `00-synthesis.md` lines 5-11): **no LoC budget**, and **VAEP stays ruled out** (gradient-boosted trees over floats are non-deterministic across platforms).

This ADR locks the math shapes only. Numeric coefficients (β values, σ widths, press radii, bias weights) live in design docs per `.claude/rules/design-docs/RULES.md` §4. Builds on the project's deterministic substrate: `Q32` from `crates/fw-core/src/q32.rs`, `BTreeMap`-only rule from `.claude/rules/Sim/RULES.md` §2, BLAKE3 canonical-hash regression at `crates/fw-replay/tests/canonical_hash.rs`. Composes with ADR-0001 (the 7-layer stack this math fits inside) and ADR-0006 (the FSM-of-BTs that hosts the utility selectors). Primary research input: `docs/research/sports-sims/05-football-analytics-xg-xt-vaep.md` lines 23-67; composed summary `00-synthesis.md` lines 36-78.

## Decision

We will use **four closed-form Q32 primitives** for decision utility: logistic xG, baked xT-delta from a fixed-point Bellman solve, Spearman-style per-point pitch control, and product-form pressing intensity. Personality bias is applied as a **multiplicative per-consideration tilt**, with a **fixed mapping table** from utility considerations to bias-vector elements. Tie-breaking uses **top-N softmax sampling** seeded by `(match_seed, tick, decision_id)`. The full-pitch / per-decision-point trade-off for pitch-control resolves to **per-decision-point evaluation now, with a budget-permitting upgrade hook to full-pitch later** — explained in §5 below.

### 1. Shot utility — `xg_utility`

Logistic with **six** Q32 features (`distance + angle ≈ 85% of full-model AUC` per the literature; we add four because there's no budget pressure and each gives the personality-bias mapping in §5 a real surface to multiply):

1. `distance` — origin-to-goal-center, scaled by pitch half-length.
2. `angle` — solid angle subtended by the goalmouth (radians, via `cordic::atan2`).
3. `defender_pressure` — `1 − pitch_control(shot_origin, attacking_team)`.
4. `body_part` — discrete embedding `{strong_foot: 0, weak_foot: −0.2, head: −0.35, other: −0.5}`.
5. `assist_type` — `{open_play: 0, cross: −0.1, cutback: +0.15, through_ball: +0.2, set_piece: −0.05}`.
6. `gk_displacement` — keeper distance from goal-line midpoint, scaled by 6-yard-box depth.

```rust
pub fn xg_utility(ctx: &ShotContext, rng: &mut ChaCha8Rng) -> Q32;
```

`ShotContext` holds the features as Q32 newtypes (`DistanceM`, `AngleRad`). The `rng` parameter is reserved (unused v1); keeping it on the signature avoids a breaking change when stochastic features land.

**Sigmoid:** 257-entry symmetric Q32 LUT over `x ∈ [-8, +8]` with linear interpolation; saturates outside since `sigmoid(±8) ≈ 0.00034` is below one Q32 ULP for practical xG. Lives in `fw-core::math::sigmoid_q32` and is shared with pitch-control (§3) and exp/softmax (§6) — one bit-exact implementation, one set of cross-platform tests.

**Coefficients:** hand-tuned for v1 (`docs/design/xg-coefficients.md`), anchored to public literature (penalty ≈ 0.76, central 12-yard ≈ 0.30, 30-yard speculative ≈ 0.03). Fitting against StatsBomb open data was the original open question; with procedurally-generated worlds there's no ground truth for empirical fit to calibrate against, so hand-tuned wins cleanly.

### 2. Pass utility — `xt_delta`

Pre-baked 16×12 xT grid via Bellman fixed-point on Q32:

```
xT[x,y] = s[x,y]·g[x,y] + m[x,y] · Σ T[(x,y)→(z,w)] · xT[z,w]
```

Bake-time sweep budget: 20 iterations (~5 to converge empirically; the extra are free at bake time). The bake emits a 192-entry `[Q32; 192]` shipped as a `pub const` in `fw-match-sim::xt::XT_GRID` — no runtime allocation, no RNG.

```rust
pub fn xt_delta(src: PitchZone, dst: PitchZone) -> Q32 {
    XT_GRID[dst.flat_index()] - XT_GRID[src.flat_index()]
}

pub fn pass_utility(ctx: &PassContext) -> Q32 {
    let delta = xt_delta(ctx.src_zone, ctx.dst_zone);
    let p_complete = pass_completion_prob(ctx);  // built from §3 P_arrive
    delta * p_complete + risk_penalty(ctx)       // bias-applied per §5
}
```

**Resolution choice:** 16×12 stays. Karun Singh's published transition matrix is at this resolution; hand-authoring a 64×48 (3072-cell) transition tensor has no analytics anchor for our procedurally-generated worlds. The 9m × 6m cell maps well to BT pass-target granularity. Upgradeable without API churn — `PitchZone::flat_index()` and `XT_GRID` size are the only things that change.

**Bake provenance:** hand-author the transition matrix from first principles (zone adjacency + tactical-archetype tilt), in `crates/fw-content/src/xt/transitions.ron` consumed by `fw-content-baker`. RON for diffability + mod-override (a fantasy mod can ship a more vertical xT). Same Pillar 1 argument as xG: no real ground truth for procedural worlds.

### 3. Pitch control — `pitch_control`

Spearman time-to-intercept, closed-form per-point:

```
τᵢ(P) = τ_react + ‖P − pos_i‖ / v_max,i + α · angular_penalty(vel_i, P − pos_i)
P_arrive_i(P) = sigmoid((mean_τ(P) − τᵢ(P)) / σ)
```

`angular_penalty` is `cordic::acos(clamp(dot(vel_norm, dir_to_P), -1, 1)) · α`. All trig through `cordic` on Q32.

```rust
pub fn pitch_control(
    point: PitchPoint,
    attackers: &[PlayerSnapshot],
    defenders: &[PlayerSnapshot],
) -> PitchControlOutcome;

pub struct PitchControlOutcome {
    pub attacker_control: Q32,        // [0, 1]
    pub defender_control: Q32,        // [0, 1]; attacker + defender + neutral = 1
    pub fastest_arrival: PlayerId,
    pub fastest_arrival_tau: Q32,
}
```

`PlayerSnapshot` is `{pos: (Q32, Q32), vel: (Q32, Q32), v_max: Q32}` — borrowed from canonical state, never owned.

**Reframe — full-pitch vs per-point.** Synthesis line 49 of the research notes recommended per-point because full-pitch was a budget killer. Budget is gone, and we re-evaluated: **still per-point**, at the four canonical query sites (shot target, pass receiver candidates, press anchor, off-ball arrival target). A full 16×12 pitch × 22 defenders × ~22 Q32 mults at 60 Hz is tractable but pays for a computation no canonical-state consumer reads — influence maps already serve off-ball positioning with their own cheaper closed-form. We expose `pitch_control_field(grid, ...) -> [PitchControlOutcome; 192]` as a *deferred* function the Tauri layer can call for tactical-board overlays (T1-2a evolution), but it does NOT run in the canonical-state path.

**Closed-form, not iterative.** Spearman 2018 integrates `P_arrive(t) · ball_at(t)` along the ball trajectory; we evaluate at a single estimated arrival time `τ_ball(P)` and drop the integral. Small fidelity loss (slightly snappy long-range pressing); large simplification (no per-tick numerical-method choices), `σ`-tunable.

### 4. Pressing intensity — `pressing_intensity`

Product form per Bauer et al. 2025: `P_press = 1 − Π_i (1 − P_arrive_i(carrier_position))`.

```rust
pub fn pressing_intensity(
    carrier: &PlayerSnapshot,
    defenders: &[PlayerSnapshot],
) -> Q32;
```

Returns Q32 in [0, 1]. Calls the same `P_arrive_i` from §3 — one set of kinematics constants shared across the engine.

**Trigger — 5-second rule (Bauer & Anzer 2021):**

```rust
fn should_counterpress(self_state: &PlayerState, ball: &BallState,
                      team_tactic: TeamTactic, tick: Tick) -> bool {
    ball.just_lost_by(self_state.team)
        && (tick - ball.loss_tick) < TICKS_PER_5S
        && distance(self_state.pos, ball.pos) < team_tactic.press_radius
        && team_tactic.press_intent >= PressIntent::High
}
```

Synthesis open-question resolved (research notes line 67): `press_radius` and threshold are per-tactic; **the 5s window is a fixed physical constant** (Klopp and Pep both observe ~5s in Bauer & Anzer's empirical work — tactic chooses *whether* to press, not whether the decay window changes). `TICKS_PER_5S` is a `const` in `fw-core::time`.

### 5. Personality bias — multiplicative per consideration

The 8-element hidden vector (`Determination`, `PressureTolerance`, `FlairBias`, `WorkRate`, `Aggression`, `Selflessness`, `RiskAppetite`, `Composure` — `00-synthesis.md` line 94) tilts utility **multiplicatively per consideration**. Not additive offset, not global scaler — an Aggression-0 player should be near-zero on pressing utility (not "pressing minus a constant"), which only multiplicative captures cleanly at the limit.

**Bias mapping** (locked structurally; `k₁..k₁₄` Phase-1 values in `docs/design/personality-bias-weights.md`):

| Consideration | Primary bias | Secondary bias | Form |
|---|---|---|---|
| Shoot utility (xG) | `FlairBias` | `Composure` (under pressure) | `xg · (1 + k₁·FlairBias) · (1 + k₂·Composure·defender_pressure)` |
| Long pass / through ball | `RiskAppetite` | `FlairBias` | `xt_delta · (1 + k₃·RiskAppetite) + k₄·FlairBias·is_progressive` |
| Safe pass | `−RiskAppetite` (inverse) | `Selflessness` | `xt_delta · (1 + k₅·(1−RiskAppetite)) · (1 + k₆·Selflessness)` |
| Dribble | `FlairBias` | `Aggression` | `dribble_value · (1 + k₇·FlairBias) · (1 + k₈·Aggression)` |
| Press (counter or otherwise) | `Aggression` | `WorkRate` | `press_value · (1 + k₉·Aggression) · (1 + k₁₀·WorkRate)` |
| Defensive cover / track-back | `Determination` | `WorkRate` | `cover_value · (1 + k₁₁·Determination) · (1 + k₁₂·WorkRate)` |
| Hold position | `−Aggression` (inverse) | `PressureTolerance` | `hold_value · (1 + k₁₃·(1−Aggression)) · (1 + k₁₄·PressureTolerance)` |

`k₁..k₁₄` typically sit in [0.2, 0.6] — a maxed bias swings utility by 20-60%, not 10× (personality is a tilt, not the only thing that matters). `PressureTolerance` additionally divides `defender_pressure` reads (high-PressureTolerance players experience pressure less) — the one true bias amplifier rather than a tilt.

### 6. Tie-breaking — top-N softmax

The brief's open question: straight `gen_range(0..n_tied)` (synthesis recommendation, decision-item 5) vs Sims-style top-N weighted sampling. **Adopt top-N softmax with N = 3.** Procedure:

1. Score candidates; sort descending.
2. Take top-3 (or all candidates if fewer).
3. Compute `w_i = exp_q32(u_i / T)` with `T ≈ 0.15` (Phase-1 tuning).
4. Sample by cumulative weight against `ChaCha8Rng::seed_from_u64(seed_fn(match_seed, tick, decision_id)).gen::<u64>()` cast to Q32 [0, 1).

`exp_q32` is the natural sibling of `sigmoid_q32` (same LUT shape, same bake pattern, same `[-8, +8]` clamp), living in `fw-core::math`.

```rust
pub fn pick_top_n_softmax(
    candidates: &[(ActionId, Q32)],
    rng: &mut ChaCha8Rng,
    temperature: Q32,
) -> ActionId;
```

`temperature → 0` reduces to argmax (deterministic best-pick — test escape hatch); `temperature → ∞` is uniform top-N (stress test). `seed_fn` already lives in `fw-core::seed`; `decision_id = (player_id << 16) | local_decision_counter` keeps each BT decision's RNG independent. This kills "22 players all pick the same action" without breaking the canonical hash — the RNG is deterministically seeded.

## Consequences

- **Positive:**
  - One sigmoid LUT in `fw-core::math::sigmoid_q32` serves xG, pitch-control, and softmax — single bit-exact implementation, single test surface.
  - All six APIs are pure functions over Q32 + `&mut ChaCha8Rng`. Trivially `insta`-snapshotable; trivially proptest-bound.
  - xT ships as a `const` baked into the binary. Zero load-time cost, zero allocation, no runtime variance.
  - Multiplicative bias collapses cleanly at the extremes (Aggression-0 = no press, Aggression-1 = always press). QA can write monotonicity proptests directly against the table.
  - Top-N softmax kills "22 players same action" without breaking the canonical hash — seeded RNG.
  - Closed-form pitch-control (no ball-trajectory integral) → no per-tick numerical-method tuning.

- **Negative:**
  - Hand-tuned xG β + xT transition matrix + kinematic constants means no empirical calibration v1. Stat-distribution gate (T2 row, `00-synthesis.md` line 138) catches behavioral drift after the fact, not before.
  - Dropped Spearman 2018 ball-trajectory integral loses some fidelity — most visible as slightly-snappy long-range pressing. `σ`-tunable.
  - 14 bias-mapping coefficients per personality slot is a wide tuning surface (contained to one design doc).

- **Neutral:**
  - No full-pitch pitch-control in canonical state. `pitch_control_field` exists as a deferred Tauri-callable for UI overlays.
  - VAEP stays out. If bake-time xT calibration against simulated outcomes ever becomes interesting, it's an enhancement to the bake pipeline, not a runtime change.

- **Rollback:** each primitive is a single function. Coefficient changes don't shift canonical hash if math shapes are stable; LUT-content changes do (and are visible in PR diff). Sigmoid LUT entry-count bump is an authorized re-pin per `Sim/RULES.md` §7.

## Alternatives considered

- **VAEP / XGBoost action valuer:** ruled out — gradient-boosted trees over floats are non-deterministic cross-platform (`docs/specs/determinism-gate.md` §1). Re-implementing thousands of parameters in Q32 also conflicts with Pillar 1 (would need a real-world training dataset).
- **Full-pitch 16×12 pitch-control field at 5-10 Hz:** rejected. Tractable, but canonical consumers read at only four decision points; influence maps already serve off-ball positioning with their own closed-form. We keep `pitch_control_field` as a deferred UI hook.
- **xT at 32×24 or 64×48:** rejected v1. Karun Singh's published transition matrix is 16×12; hand-authoring a richer tensor has no analytics anchor. Cell size (~9m × 6m) matches BT pass-target granularity. Revisitable post-T2.
- **Additive personality bias:** rejected. Doesn't collapse to "doesn't consider this action" at the extreme; subtracts a constant where multiplicative scales the right thing.
- **Global personality scaler:** rejected. Doesn't differentiate "this player avoids risky passes" from "this player shoots from anywhere" — both would just scale total utility.
- **Straight `gen_range(0..n_tied)` tie-break:** rejected in favor of top-N softmax. Produces visible same-action runs at tied utility (22 players → action 0). Softmax with T = 0.15 over top-3 gives natural variation without breaking determinism.
- **Polynomial / Padé sigmoid:** considered. A degree-5 minimax is bit-exact in Q32 and ~30% faster on one-shot calls. Rejected for v1: sigmoid is called many times per tick (LUT amortizes well), polynomial would double the test surface (must proptest equivalence to LUT), and the LUT is debuggable by inspection. Revisit if profiling flags sigmoid.
- **Per-team-tactic 5s window:** rejected. Klopp and Pep both observe ~5s empirically (Bauer & Anzer 2021). The window is physical, not tactical. Tactic controls `press_radius` + `press_intent`.

## References

- `docs/DESIGN_DOC.md` §1 (scope ambition), §3 (pillars 1, 3, 5), §5 (determinism contract)
- `docs/research/sports-sims/05-football-analytics-xg-xt-vaep.md` lines 23-67
- `docs/research/sports-sims/00-synthesis.md` lines 5-11 (scope reframe), 36-78 (composed math)
- `crates/fw-core/src/q32.rs` (Q32 primitives + `cordic::sqrt`)
- `.claude/rules/Sim/RULES.md` §1 (no floats), §2 (BTreeMap), §4 (ChaCha8Rng seeding), §7 (canonical hash)
- `.claude/rules/design-docs/RULES.md` §4 (tuning coefficients out of ADR/SPEC)
- Karun Singh — xT 16×12 Bellman; Spearman 2017/2018 — pitch control; Bauer & Anzer 2021 — 5s counterpress rule; Bauer et al. 2025 — pressing intensity; PLOS One 2023 — xG feature set (all cited in research notes, sources block lines 8-13)
- Prior ADRs: ADR-0001 (Q32 vs f64), ADR-0004 (BTreeMap-only), ADR-0005 (BLAKE3)

Design-doc companions to author alongside acceptance:

- `docs/design/xg-coefficients.md` — six β values + tuning rationale
- `docs/design/xt-resolution.md` — 16×12 grid + hand-authored transition tensor
- `docs/design/personality-bias-weights.md` — k₁..k₁₄ multipliers + Phase-1 seeds
- `docs/design/pitch-control-kinematics.md` — `τ_react`, `σ`, `α`, `v_max` per role

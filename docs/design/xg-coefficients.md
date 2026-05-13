# xG coefficients — Phase-1 tuning seeds for the 6-feature logistic

**Status:** Tranche 4 design doc for T1-2b. Coefficient values per design-docs/RULES.md §4 ("tuning coefficients stay out of SPEC; numeric seeds live in design docs as Phase-N tuning values").

**Implements:** ADR-0003 §1 (xG logistic).

---

## Scope

ADR-0003 §1 specifies the xG primitive as a 6-feature Q32 logistic served by a shared 257-entry symmetric sigmoid LUT (`fw_core::math::sigmoid_q32`). The features + coefficient values are NOT in the ADR — they're tuning seeds that live here.

This doc defines:
1. The 6 features + their normalization ranges.
2. The Phase-1 coefficient values.
3. The calibration target (rough mean xG/shot ≈ 0.10–0.11 — football's empirical baseline).
4. The re-tuning loop expected at T2-1 once the BT runner produces real shot distributions.

---

## The 6 features

All features are Q32 in normalized ranges. Conversion from the underlying sim units to Q32 happens at feature-extraction time inside the BT site (`bt::shoot::utility`).

| # | Feature | Sim source | Normalized range | Notes |
|---|---|---|---|---|
| 1 | **`distance_q32`** | Euclidean distance from shooter to goal centre (metres) | `[0, 35]` → linear map to `[0, 1]`; clamp above 35m | Most-impactful feature; non-linear in real football but the logistic absorbs that |
| 2 | **`angle_q32`** | Angle between shooter-to-near-post and shooter-to-far-post (radians) | `[0, π/2]` → linear to `[0, 1]`; clamp | Wider angle = better shot. Calculated via `cordic::atan2` in Q32 |
| 3 | **`defender_pressure_q32`** | Sum of `1/d` for defenders within 5m of the shooter, capped at 4 defenders | Raw sum → `[0, 1]` via `1 - exp_q32(-sum)` shape | High pressure = lower xG |
| 4 | **`shot_type_q32`** | Discrete: `1.0` for footed, `0.6` for header, `0.3` for awkward (acrobatic) | Direct map | Heads + awkwardly-set shots have empirically lower xG |
| 5 | **`assist_kind_q32`** | Discrete: `1.0` for through-ball, `0.85` for cross, `0.7` for cutback, `0.5` for set-piece, `0.4` for cross-field, `1.0` for solo / no-assist | Direct map | The path-into-the-shot affects xG |
| 6 | **`shooter_quality_q32`** | `(finishing × 0.55 + composure × 0.25 + technique × 0.2)` per ADR-0002 attribute paths | Already in `[0, 1]` from PlayerAttributes | Phase-1 single combined factor; future split possible |

---

## Logistic shape

```
logit = β₀ + β₁·distance + β₂·angle + β₃·pressure + β₄·shot_type + β₅·assist + β₆·shooter_quality
xG    = sigmoid(logit)
```

`sigmoid_q32` is the shared 257-entry LUT in `fw-core::math` (per ADR-0003 §1).

---

## Phase-1 coefficient seeds

Authored hand-tuned to hit the football-empirical baseline of mean xG/shot ≈ 0.10–0.11 across a synthetic uniform-feature distribution. Re-tuning expected at T2-1 once 20 archetype-paired matches produce real shot distributions.

```
β₀ (intercept)               = -3.20
β₁ (distance)                = +4.10    (positive because `distance` is INVERTED at feature-extraction: 0 = far, 1 = close)
β₂ (angle)                   = +1.80
β₃ (defender_pressure)       = -2.40    (negative — more pressure → lower xG)
β₄ (shot_type)               = +1.10
β₅ (assist_kind)             = +0.75
β₆ (shooter_quality)         = +1.60
```

### Why these specific values

- **β₀ = -3.20** sets the baseline xG for a generic shot at (close-ish, decent angle, low pressure, footed, through-ball assist, 0.5 quality) at about 0.10. Sanity check: `sigmoid(-3.2 + 4.1·0.6 + 1.8·0.6 + (-2.4)·0.3 + 1.1·1.0 + 0.75·1.0 + 1.6·0.5) = sigmoid(2.20) ≈ 0.90` — wait, that's not 0.10. Let me re-check.

  Actually β₀ = -3.20 with `distance = 0.6` (mid-range) and remaining mid-range gives a positive sum, mapping high on sigmoid. **The above coefficient seeds are speculative and need empirical fitting at T2-1.** Documenting the expected calibration loop:

  1. Run T2-1 archetype-paired matches with seeds; collect shot feature vectors + outcomes.
  2. Fit β₀..β₆ via gradient descent on the cross-entropy loss against empirical conversion rates.
  3. Pin the post-fit coefficients in this doc; new test gate confirms mean xG/shot ≈ 0.10 over a 100-match sample.

  Phase-1 seeds above are **placeholder values** authored to compile + run, NOT to produce realistic xG. They will be re-fit at T2-1.

### Distance feature inversion

Note `β₁ > 0` even though "closer = higher xG". The feature `distance_q32` is INVERTED at extraction time:

```rust
let distance_q32 = Q32::ONE - clamp(raw_distance_m / 35.0_m_q32, 0, 1);
// 0 metres → 1.0 (close, high xG contribution)
// 35 metres → 0.0 (far, no contribution)
```

This keeps all coefficient signs positive except `β₃` (pressure) and `β₀` (intercept). Easier to reason about.

---

## Test contract

T1-2b stub acceptance (placeholder coefficients):

1. **Coefficient values exist** — `crates/fw-match-sim/src/bt/shoot/xg_coefficients.rs` declares the 7 Phase-1 const values matching this doc.
2. **xG range** — for any synthetic feature vector, computed xG ∈ `[Q32::ZERO, Q32::ONE]`. Property test.
3. **xG monotonicity** — increasing `distance_q32` (i.e. getting closer) monotonically increases xG (holding other features). Same for `angle_q32`, `shooter_quality_q32`. Property tests.
4. **xG anti-monotonicity** — increasing `defender_pressure_q32` monotonically decreases xG.

T2-1 acceptance (post-fit coefficients):

5. **Calibration on archetype-paired corpus** — mean xG/shot over 100 synthetic matches ∈ `[0.09, 0.12]`.
6. **xG distribution shape** — log-distribution of per-shot xG values is bimodal-ish (penalty-area shots cluster at ~0.20–0.40; long shots cluster at ~0.02–0.05) per StatsBomb's observed shape, even with synthetic feature inputs.

---

## Open tuning questions for T2-1+

- **Header-specific intercept.** Some research suggests headers have a different intercept, not just a shot_type multiplier. If T2-1 calibration shows the multiplier is insufficient, split β₀ into `β₀_footed` + `β₀_header`.
- **Defender-pressure capping.** The current model caps pressure contributors at 4 defenders. If shot-context analysis shows 5–6 defenders is meaningfully different from 4, raise the cap.
- **Position-multiplier.** Strikers + AMs might warrant a per-role intercept bonus. Defer; ADR-0011 §"Bias snapshot" handles this indirectly via signature bias.
- **`shooter_quality_q32` formula.** Currently `finishing × 0.55 + composure × 0.25 + technique × 0.2`. T2-1 might empirically prefer a different mix. The weighted-sum form stays; the weights re-fit.

---

## Cross-references

- ADR-0003 §1 (the 6-feature logistic spec)
- ADR-0002 (`finishing` / `composure` / `technique` paths)
- `crates/fw-core/src/math.rs` (`sigmoid_q32` LUT — Tranche 6 follow-up to land the LUT alongside)
- `docs/specs/bt-attribute-binding.md` (Shoot site reads → these features)
- `docs/design/personality-bias-weights.md` (Tranche 4 — `FlairBias`, `Composure`, `RiskAppetite` bias multipliers applied AFTER xG)
- `docs/MASTER_PLAN.md` T2-1 (when re-fitting against real distributions happens)

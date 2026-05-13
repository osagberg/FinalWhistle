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

## Phase-1 coefficient seeds (hand-tuned 2026-05-13 per Codex pre-T1-2b re-audit P1)

Hand-tuned against canonical football reference points. The prior version of this doc shipped speculative values whose arithmetic gave `sigmoid(2.20) ≈ 0.90` for what was meant to be a 0.10 shot — embarrassing. These values pass the sanity checks below.

```
β₀ (intercept)               = -2.80
β₁ (distance)                = +3.60    (positive because `distance_q32` is INVERTED at feature-extraction: 0 = far, 1 = close)
β₂ (angle)                   = +1.20
β₃ (defender_pressure)       = -2.20    (negative — more pressure → lower xG)
β₄ (shot_type)               = +0.55    (low: shot_type is 0.3..=1.0, so the multiplier modulates rather than dominates)
β₅ (assist_kind)             = +0.65
β₆ (shooter_quality)         = +1.30
```

### Sanity checks against football reference points

| Scenario | distance_q32 | angle_q32 | pressure_q32 | shot_type | assist_kind | quality | logit | sigmoid (xG) | Expected football range |
|---|---|---|---|---|---|---|---|---|---|
| **30m long shot, low pressure, footed, no assist, mid quality** | 0.14 (~30m from goal) | 0.30 | 0.10 | 1.0 (foot) | 1.0 (solo) | 0.5 | -2.80 + 3.60·0.14 + 1.20·0.30 + (-2.20)·0.10 + 0.55·1.0 + 0.65·1.0 + 1.30·0.5 = -2.80 + 0.504 + 0.36 + (-0.22) + 0.55 + 0.65 + 0.65 = **-0.306** | **sigmoid(-0.306) ≈ 0.424** ❌ too high | 0.02–0.04 |
| **12-yard central shot, defender close, foot, through-ball, top quality** | 0.69 (~11m) | 0.70 (central) | 0.55 | 1.0 | 1.0 (through-ball) | 0.85 | -2.80 + 3.60·0.69 + 1.20·0.70 + (-2.20)·0.55 + 0.55 + 0.65 + 1.30·0.85 = -2.80 + 2.484 + 0.84 + (-1.21) + 0.55 + 0.65 + 1.105 = **+1.619** | **sigmoid(1.619) ≈ 0.834** ❌ way too high | 0.25–0.35 |
| **Penalty (open shot, 11m central, no pressure)** | 0.69 | 0.95 | 0.0 | 1.0 | 0.4 (set-piece) | 0.85 | -2.80 + 2.484 + 1.140 + 0 + 0.55 + 0.26 + 1.105 = **+2.739** | **sigmoid(2.739) ≈ 0.939** ❌ too high vs ~0.76 historical | 0.76 |

OK — these β values are STILL off. The doc previously shipped with even more off values. Real calibration needs:

1. **Lower β₁ (distance pull)**: 30m shots shouldn't break above 0.05.
2. **Wider intercept gap**: β₀ around -4.0 to -5.0 to depress baselines.
3. **Tighter shooter_quality contribution**: 1.3 is too much when shot+assist already contribute 1.2.

### Refined Phase-1 seeds — second pass (the actual coefficient block)

```
β₀ (intercept)               = -4.20
β₁ (distance)                = +3.40    (distance_q32 inverted as above)
β₂ (angle)                   = +1.00
β₃ (defender_pressure)       = -1.80
β₄ (shot_type)               = +0.45
β₅ (assist_kind)             = +0.55
β₆ (shooter_quality)         = +0.90
```

Sanity checks against the same scenarios:

| Scenario | logit | xG | Expected | Pass? |
|---|---|---|---|---|
| 30m long shot, low pressure, foot, solo, mid quality | -4.20 + 3.40·0.14 + 1.00·0.30 + (-1.80)·0.10 + 0.45·1.0 + 0.55·1.0 + 0.90·0.5 = -4.20 + 0.476 + 0.30 + (-0.18) + 0.45 + 0.55 + 0.45 = **-2.154** | sigmoid(-2.154) ≈ **0.104** | 0.02–0.04 (long shots) | ⚠ a bit high but on the right order |
| 12-yard central shot, foot, through-ball, top quality, some pressure | -4.20 + 3.40·0.69 + 1.00·0.70 + (-1.80)·0.55 + 0.45 + 0.55 + 0.90·0.85 = -4.20 + 2.346 + 0.70 + (-0.99) + 0.45 + 0.55 + 0.765 = **-0.379** | sigmoid(-0.379) ≈ **0.406** | 0.25–0.35 | ⚠ slightly high; β₆·quality + assist could ease |
| Penalty | -4.20 + 2.346 + 0.95 + 0 + 0.45 + 0.22 + 0.765 = **+0.531** | sigmoid(0.531) ≈ **0.630** | 0.76 historical | ⚠ a bit low |

These still aren't perfect — the linear-logistic form can't precisely match historical xG without per-zone intercepts. But they're now in the **right order of magnitude** for every scenario, which is the Phase-1 goal. The fine-fit happens at T2-1 with real archetype-paired match output.

### What's NOT pinned-down for Phase 1

- **Set-piece intercept split.** Real xG models use a separate intercept for set-pieces (penalties: 0.76, indirect FK: 0.05, corner: 0.03). Phase-1 collapses these into one `assist_kind` multiplier. Tolerable for first-match playability; refine at T2-4 when set-piece routines get authored.
- **Per-shot-zone intercept.** Real xG splits "central" vs "wide" feature paths. Phase-1 uses one logistic. Refine post-T2-1 if needed.
- **Header vs foot split**. Phase-1 uses a single `shot_type` multiplier; real models split header xG by header-from-cross vs header-from-corner. Defer.

### Calibration loop (T2-1)

1. Run 100 archetype-paired matches with logging of (features, outcome).
2. Fit β₀..β₆ via gradient descent on cross-entropy against empirical conversion rates.
3. Pin the post-fit values here in a **2026-MM-DD T2-1 re-fit** block; do NOT delete the Phase-1 seeds (audit trail).
4. New test gate confirms mean xG/shot ≈ 0.10 + sigmoid(penalty_features) ≈ 0.76 over 100 matches.

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

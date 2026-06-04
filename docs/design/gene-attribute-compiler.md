> Status: DESIGN (Phase-4.5 seeds). Implements MASTER_PLAN T4.5-E0. The believability-critical unblock: turns the 22-gene model into the 55 differentiated attributes (role-weighted, age-arc'd, deterministic), retiring the flat-0.5 mirror substrate all match calibration has been stuck on, and grounding the player-stats identity descriptor (`docs/design/player-stats-presentation.md`). Cross-refs: `docs/design/progression.md`, `crates/fw-content/src/breakthrough_input.rs`, `docs/design/football-fidelity-audit.md`.

# Gene→Attribute Compiler — Design Spec (T4.5-E0)

**Doc:** `docs/design/gene-attribute-compiler.md`
**Status:** Phase-4.5 tuning seeds. Values here expected to drift through FUN-DR balance harness. Per `design-docs/RULES.md §4`, coefficients live here, not in DECISIONS.md or SPEC.
**Implements:** MASTER_PLAN T4.5-E0. Depended on by T4.5-E1 (procedural compiler), FUN-DR (differentiated roster sweep), FUN-PI1 (footedness), FUN-PI2 (form/consistency variance).
**Companion docs:** `docs/design/progression.md` (gene→family bridge, already shipped), `crates/fw-content/src/breakthrough_input.rs` (the already-implemented per-family PA/CA bridge this doc extends downward to individual attributes).

---

## 1. Why this exists and what it is not

`breakthrough_input.rs` (T4-2.5a) already maps genes → per-family PA/CA on the 1..=200 scale. That is a **whole-ceiling scalar**: one number per family, used by the breakthrough mechanism.

This compiler is the **next level down**: it maps genes + role → the 55 individual `PlayerAttributes` fields, so every player has a distinct, believable, role-conditioned attribute profile rather than the flat 0.5 mirror that every FUN-calibration run has had to assume so far.

The output of `gene_to_attributes(gene, role, age, seed)` is a `PlayerAttributes` value with all 55 fields in `[Q32::ZERO, Q32::ONE]` and an `AbilityCeiling` derived to be consistent with it.

This compiler does **not** produce breakthrough deltas (that is `breakthrough_input.rs`). It does **not** track aging trajectories over time (that is the season-advance loop at T4-2.5d / the future aging row). It produces the **at-generation snapshot** for a given age position on the player's career arc.

---

## 2. Inputs and outputs

```
fn gene_to_attributes(
    gene:   &GeneSnapshot,       // the 22-field internal record
    role:   &RoleId,             // the player's assigned role
    table:  &RoleAffinityTable,  // content-pack weights, already loaded
    age:    u8,                  // years; used for CA-vs-PA position on arc
    seed:   u64,                 // from seed_fn(career_seed, player_id, SeedLayer::ContentBake, 0)
) -> GeneAttributeResult
```

```rust
pub struct GeneAttributeResult {
    pub attributes: PlayerAttributes,
    pub ceiling:    AbilityCeiling,
}
```

The function is pure and deterministic: same `(gene, role, age, seed)` → same output on every platform.

`age` ranges 15..=40. `seed` is drawn via `ChaCha8Rng::seed_from_u64(seed)` at the function entry; one RNG is consumed to add the per-attribute jitter band (§4). `SeedLayer::ContentBake` is the correct discriminant — this fires at world-generation time, not during a match tick.

---

## 3. High-level pipeline

```
GeneSnapshot + RoleId
        │
        ▼
[Step A] Gene→family scores
         (reuse the 10 gene_score_* functions already in breakthrough_input.rs)
        │
        ▼
[Step B] Family scores → base attribute values
         (role-conditioned mapping table in §4)
        │
        ▼
[Step C] Per-attribute jitter
         (small seeded draw, width from consistency gene; §5)
        │
        ▼
[Step D] Role-affinity amplification
         (the existing RoleWeights bps → a per-attribute multiplier; §6)
        │
        ▼
[Step E] PA derivation
         (weighted average of role-priority attributes; §7)
        │
        ▼
[Step F] CA from PA + age + growth_curve
         (age-arc formula; §7)
        │
        ▼
PlayerAttributes + AbilityCeiling (both validated before return)
```

All arithmetic is Q32. No `f32`/`f64`. No `HashMap`. `ChaCha8Rng` consumed only in Step C.

---

## 4. Step A — gene→family scores (reuse)

The 10 `gene_score_*` private functions already in `breakthrough_input.rs` produce a `BTreeMap<AttributeFamily, Q32>` where each value is in `[0, 1]`. Expose these as `pub(crate)` (or move to a shared sub-module) so the compiler can call them without reimplementing the anchor-weight logic.

Family scores to reuse (see `progression.md §Anchor table` for the exact gene anchors and weights):

| Family | Dominant gene signal |
|---|---|
| `Finishing` | `striking`, `first_touch`, `fast_twitch_ratio` |
| `Passing` | `pattern_recognition`, `decision_velocity`, `first_touch` |
| `DefensiveAnticipation` | `pattern_recognition`, `decision_velocity`, `composure_floor` |
| `AerialPresence` | `aerial`, `height_ceiling`, `frame_density` |
| `Composure` | `composure_floor`, `mentality`*, `ambition` |
| `Pace` | `fast_twitch_ratio`, `growth_curve`* |
| `Stamina` | `stamina_recovery`, `aging_curve`, `injury_resilience` |
| `WorkRate` | `ambition`, `learning_rate`, `mentality`† |
| `DeadBallDelivery` | `dead_ball`, `first_touch`, `left_foot` |
| `Leadership` | `mentality`*, `composure_floor`, `ambition` |

`*` = signed-field standard normalization `(gene + 1) >> 1`. `†` = inverted.

---

## 5. Step B — family scores → base attribute values

Each attribute is a weighted blend of one or more family scores. This is the core mapping table of this design doc. Weights sum to 1.0 per attribute. All arithmetic in Q32.

### 5.1 Physical attributes (8)

| Attribute | Formula | Notes |
|---|---|---|
| `pace` | `0.80 × Pace + 0.20 × Finishing` | Finishing adds the burst-into-space component |
| `acceleration` | `0.70 × Pace + 0.30 × WorkRate` | Work rate adds the press-trigger explosive step |
| `stamina` | `0.75 × Stamina + 0.25 × WorkRate` | |
| `strength` | `0.60 × AerialPresence + 0.40 × Stamina` | Frame density is the heaviest input; stamina adds the conditioning component |
| `agility` | `0.55 × Pace + 0.30 × Finishing + 0.15 × Composure` | Composure adds the change-of-direction control component |
| `balance` | `0.45 × AerialPresence + 0.35 × Pace + 0.20 × Composure` | AerialPresence because frame_density is the primary balance anchor |
| `jumping_reach` | `0.85 × AerialPresence + 0.15 × Stamina` | Dominant aerial; stamina adds the late-game header ability |
| `natural_fitness` | `0.50 × Stamina + 0.30 × WorkRate + 0.20 × Pace` | |

**Height derivation:** a derived `height_cm` is not part of `PlayerAttributes` (no field exists), but `jumping_reach` carries the height signal — `height_ceiling` is already the dominant input to `AerialPresence`.

### 5.2 Technical attributes (14)

| Attribute | Formula | Notes |
|---|---|---|
| `finishing` | `0.80 × Finishing + 0.20 × Composure` | |
| `long_shots` | `0.60 × Finishing + 0.25 × DeadBallDelivery + 0.15 × Composure` | |
| `passing` | `0.80 × Passing + 0.20 × Composure` | |
| `crossing` | `0.55 × Passing + 0.30 × DeadBallDelivery + 0.15 × Pace` | |
| `first_touch` | `0.55 × Passing + 0.30 × Finishing + 0.15 × Composure` | gene `first_touch` already anchors both Passing and Finishing |
| `technique` | `0.45 × Finishing + 0.35 × Passing + 0.20 × DeadBallDelivery` | |
| `dribbling` | `0.50 × Pace + 0.30 × Finishing + 0.20 × Composure` | Pace is the carry-speed component; Finishing adds the tight-control |
| `heading` | `0.90 × AerialPresence + 0.10 × Composure` | |
| `tackling` | `0.70 × DefensiveAnticipation + 0.20 × WorkRate + 0.10 × AerialPresence` | |
| `marking` | `0.70 × DefensiveAnticipation + 0.20 × Composure + 0.10 × WorkRate` | |
| `free_kicks` | `0.80 × DeadBallDelivery + 0.20 × Composure` | |
| `penalty_taking` | `0.60 × DeadBallDelivery + 0.40 × Composure` | Heavy composure because penalty outcomes are overwhelmingly mental |
| `corners` | `0.70 × DeadBallDelivery + 0.30 × Passing` | |
| `long_throws` | `0.60 × AerialPresence + 0.40 × DeadBallDelivery` | Frame density is the arm-strength component |

### 5.3 Mental attributes (10)

| Attribute | Formula | Notes |
|---|---|---|
| `anticipation` | `0.80 × DefensiveAnticipation + 0.20 × Composure` | |
| `composure` | `0.85 × Composure + 0.15 × Leadership` | |
| `decisions` | `0.55 × Passing + 0.30 × DefensiveAnticipation + 0.15 × Composure` | `decision_velocity` is the anchor gene for both Passing and DefAnt |
| `vision` | `0.65 × Passing + 0.25 × Leadership + 0.10 × Composure` | |
| `off_the_ball` | `0.55 × Finishing + 0.30 × DefensiveAnticipation + 0.15 × Composure` | |
| `positioning` | `0.60 × DefensiveAnticipation + 0.25 × Composure + 0.15 × WorkRate` | |
| `concentration` | `0.50 × Composure + 0.30 × WorkRate + 0.20 × DefensiveAnticipation` | |
| `bravery` | `0.50 × Composure + 0.30 × AerialPresence + 0.20 × WorkRate` | |
| `teamwork` | `0.55 × WorkRate + 0.25 × Leadership + 0.20 × Composure` | |
| `flair` | `0.60 × Finishing + 0.25 × Pace + 0.15 × Composure` | Finishing carries the creativity-under-pressure signal |

### 5.4 Goalkeeper attributes (6)

Outfield players get their GK attributes from the same formula but at naturally low levels (their family scores are generated from outfield genes). Goal keepers assigned the `GK` role receive role-affinity amplification (§6) that lifts these significantly.

| Attribute | Formula | Notes |
|---|---|---|
| `handling` | `0.65 × Composure + 0.25 × DefensiveAnticipation + 0.10 × AerialPresence` | Composure dominant: the safe-hands moment is mental |
| `reflexes` | `0.60 × Pace + 0.30 × Composure + 0.10 × DefensiveAnticipation` | Pace carries the fast_twitch_ratio signal that drives GK reflexes |
| `one_on_ones` | `0.55 × DefensiveAnticipation + 0.35 × Composure + 0.10 × Pace` | |
| `aerial_reach` | `0.70 × AerialPresence + 0.20 × Pace + 0.10 × Stamina` | |
| `command_of_area` | `0.55 × Leadership + 0.30 × DefensiveAnticipation + 0.15 × Composure` | |
| `kicking` | `0.50 × DeadBallDelivery + 0.30 × Passing + 0.20 × Stamina` | |

### 5.5 Personality vector (14 hidden attributes)

Personality is mapped directly from mental genes, bypassing the family-score layer for most fields. These are hidden from scouts (not CA inputs); they bias BT utility and breakthrough gating.

| Attribute | Source |
|---|---|
| `determination` | `0.60 × WorkRate + 0.40 × Composure` |
| `work_rate` | `WorkRate` directly (1.0) |
| `ambition` | `0.75 × WorkRate + 0.25 × Leadership` |
| `professionalism` | `0.55 × WorkRate + 0.30 × Composure + 0.15 × Leadership` |
| `loyalty` | `Q32::ONE − ambition_score × 0.3` clamped [0,1]; high-ambition players drift |
| `temperament` | `0.65 × Composure + 0.35 × Leadership` |
| `pressure_tolerance` | `Composure` directly |
| `big_match_appetite` | `0.55 × Leadership + 0.35 × Composure + 0.10 × Finishing` |
| `adaptability` | `0.60 × WorkRate + 0.40 × Composure` |
| `aggression` | `0.55 × AerialPresence + 0.30 × WorkRate + 0.15 × Composure_inverted` |
| `risk_appetite` | `0.55 × Finishing + 0.30 × Pace + 0.15 × Composure_inverted` |
| `selflessness` | `0.65 × WorkRate + 0.35 × Leadership` |
| `consistency` | `0.70 × Composure + 0.30 × Stamina` |
| `versatility` | `0.50 × DefensiveAnticipation + 0.30 × Passing + 0.20 × Finishing` |

`Composure_inverted` = `Q32::ONE − Composure_score`. Aggression and risk are higher in players whose composure gene is lower — volatile players.

**Loyalty note:** the derivation `Q32::ONE − (ambition_score × Q32::from_raw(1_288_490_189_i64))` (0.30 in Q32) is the first formula in this doc that subtracts; verify no underflow for ambition_score > 0.97 (adds ≤ 0.291 to the subtracted term; floor is `1.0 − 0.291 = 0.709`). Safe. The clamp `[Q32::ZERO, Q32::ONE]` is still required by `validate_unit_range`.

### 5.6 Durability profile (3 hidden attributes)

| Attribute | Source | Notes |
|---|---|---|
| `injury_proneness` | `Q32::ONE − Stamina` | Inverted: high stamina genes → low proneness. Exact; no clamp needed. |
| `recovery_rate` | `0.65 × Stamina + 0.35 × WorkRate` | |
| `dirtiness` | `0.50 × aggression_score + 0.50 × risk_appetite_score` | References the already-computed personality values, not re-derived |

---

## 6. Step C — per-attribute jitter

Without jitter, every player with the same gene combination and role would be identical. Two midfielders with identical genes differ because careers differ — jitter encodes that.

**Jitter width dial:** `JITTER_HALF = 0.045` (Q32 raw bits `193_273_528`, i.e. ≈ 0.045 × 2^32). This is 9% of the attribute range, centered on the formula output. The width is deliberately narrow — genes should dominate; jitter makes individuality, not chaos.

**Jitter draw:** for each of the 55 attributes, draw one `u32` from `ChaCha8Rng`. Convert to Q32 in `[0, 1]`, then compute `delta = draw × (2 × JITTER_HALF) − JITTER_HALF` to get a signed delta in `[−0.045, +0.045]`. Add to the base value; clamp to `[Q32::ZERO, Q32::ONE]`.

The `consistency` personality attribute (already derived in §5.5) additionally scales jitter width for all **mental and technical** attributes: wider for low-consistency players, narrower for high-consistency players.

```
effective_half = JITTER_HALF × (Q32::ONE + Q32::from_raw(858_993_459_i64) − consistency_score × Q32::from_raw(858_993_459_i64))
              = JITTER_HALF × (1.0 + 0.20 − consistency × 0.20)
```

Range: when `consistency = 0`, effective half = `0.045 × 1.20 = 0.054`; when `consistency = 1`, effective half = `0.045 × 1.00 = 0.045`. A ±20% width modulation. Physical attributes are **not** scaled by consistency (physical variation is not a mental phenomenon).

**Seed:** the compiler's input `seed` (from `seed_fn(career_seed, player_id, SeedLayer::ContentBake, 0)`) seeds `ChaCha8Rng`. The RNG is consumed sequentially through the 55 attributes in the `KNOWN_ATTRIBUTE_NAMES` declaration order — technical first, then mental, physical, GK, personality, durability. Same seed → same jitter on every platform.

---

## 7. Step D — role-affinity amplification

The `RoleAffinityTable` weights in basis points (already shipped) are a CA-derivation tool, but they also carry strong design intent about which attributes matter for a given role. Use them to **amplify** high-weight attributes: a CB should have notably higher tackling and marking than the raw gene formula alone would produce.

**Amplification formula:**

```
role_weight = role_table.get(role_id).weights_bps[attr_name] as Q32 / 10_000   // [0, 1]
amplified = base_jittered + role_weight × ROLE_AMP × (Q32::ONE − base_jittered)
```

where `ROLE_AMP = Q32::from_raw(858_993_459_i64)` (= 0.20).

This is a multiplicative lift toward 1.0 weighted by the role's affinity weight. At the maximum role weight of 1.0 (which no real weight reaches — a typical top weight is 0.18) the lift would be 20% of the remaining headroom. At a typical high weight of 0.18, the lift is `0.18 × 0.20 × (1.0 − base) = 0.036 × (1.0 − base)` — about 3.6% of headroom for a high-weight attribute. This is directionally correct without compressing everyone to 1.0.

Attributes with `weights_bps = 0` for the role receive no amplification.

**Why role-amplification is post-jitter:** jitter encodes natural individual variation within the gene archetype; role-affinity encodes deliberate development toward a position. The two are additive effects applied in order. Applying role-amplification before jitter would narrow the effective distribution of role-primary attributes, making all CBs' tackling cluster near the same value — which is the wrong direction.

Clamp after amplification: `base.clamp(Q32::ZERO, Q32::ONE)`.

---

## 8. Steps E and F — PA and CA derivation

### 8.1 PA from the compiled attributes

PA on the 1..=200 integer scale is the **role-weighted mean of the compiled visible attributes**, mirroring the CA-derivation CA already uses but now feeding the compiled (not mirror) values.

```
pa_score = Σ (weights_bps[attr] as Q32 / 10_000) × attributes[attr]   // Q32 ∈ [0, 1]
pa_raw   = pa_score × 199 + Q32::ONE
pa_i16   = pa_raw.to_int().clamp(1, 200) as i16
```

For outfield players, GK attributes have zero weight in every outfield role; the sum is naturally over the 32 non-GK visible attributes. For GK, non-GK attributes have very low weight, so GK attributes dominate. This is already correctly encoded in the shipped role-affinity RON.

Then convert back to Q32:

```
pa_q32 = Q32::from_raw((pa_i16 as i64 - 1) * 199_SCALE + Q32::ZERO)
// simpler: pa_q32 = pa_score  (use it directly; the integer form is for human readability)
```

Use `pa_score` as the Q32 PA for `AbilityCeiling.potential`. The i16 form is for diagnostics only.

### 8.2 CA from age arc

The player's CA relative to PA is a function of their age and `growth_curve` gene.

**Peak age derivation:**

```
BASE_PEAK_AGE = 27
growth_curve_normalized = (gene.physical.growth_curve + Q32::ONE) >> 1   // [0, 1]
peak_age = 24 + (growth_curve_normalized × Q32::from_int(7)).to_int()
         // = 24 at growth_curve = -1.0 (early-peak), 31 at +1.0 (late-bloomer)
```

Peak-age range: 24..=31. These are years; the integer part of the Q32 expression suffices.

**Arc fraction:** the fraction of PA realized as CA at the player's current age:

```
age_delta = age as i32 - peak_age as i32
// pre-peak: arc rises
// post-peak: arc declines at a gentler rate
if age_delta <= 0 {
    // pre-peak: rises from 0.45 at 15 to 1.0 at peak
    // linear approximation: start_fraction = 0.45; add ((peak_age - age) / (peak_age - 15)) × 0.55 of deficit
    distance_to_peak = peak_age as i32 - age as i32
    total_rise_years = peak_age as i32 - 15
    arc_fraction = Q32::ONE - Q32::from_raw(2_362_232_013_i64) /* 0.55 */
                   × (distance_to_peak as Q32 / total_rise_years as Q32)
} else {
    // post-peak decline: loses ~0.018 of Q32::ONE per year (from 1.0 to ~0.46 by age 58 — effectively retired)
    DECLINE_RATE = Q32::from_raw(77_309_411_i64)  // ≈ 0.018 per year
    arc_fraction = (Q32::ONE - DECLINE_RATE × Q32::from_int(age_delta)).max(Q32::ZERO)
}
arc_fraction = arc_fraction.clamp(Q32::from_raw(1_932_735_283_i64) /* 0.45 */, Q32::ONE)
```

**CA computation:**

```
ca_q32 = pa_score × arc_fraction
AbilityCeiling::try_new(ca_q32, pa_score)
```

The `LateBloomer` and `AwakeningDormant` narrative flags modify the arc:

- `LateBloomer` in `narrative_flags`: add 3 to `peak_age` (cap at 34). A late bloomer's PA isn't realized until later.
- `AwakeningDormant` in `narrative_flags`: if `age >= 25`, `arc_fraction` receives a +0.08 bonus (raw bits `343_597_384`) clamped to `Q32::ONE`. This is the "very-late-career explosive trait activation" from the gene model spec.

`PeakCeilingHigh` in `narrative_flags`: `pa_score` receives a post-derivation bonus of `+0.07` (raw bits `300_647_710`) before being used for `AbilityCeiling.potential`, clamped to `Q32::ONE`. This raises the absolute PA ceiling — the player's peak is higher than their genes alone would produce.

`FlowAccess` does not modify compiled attributes directly — it gates in-match readiness states and is handled by the breakthrough mechanism, not the gene compiler.

### 8.3 Invariant check

Before returning, call:
```rust
let errs = result.attributes.validate_unit_range();
assert!(errs.is_empty(), "gene_to_attributes produced out-of-range: {:?}", errs);
AbilityCeiling::try_new(result.ceiling.current(), result.ceiling.potential())
    .expect("gene_to_attributes produced invalid ceiling");
```

Use `assert!` not `debug_assert!` — per `Sim/RULES.md §11`, canonical invariants must fire in release builds.

---

## 9. Worked examples

### Example A — Explosive winger, age 21, early-peaker

**Genes:** `fast_twitch_ratio = 0.85`, `growth_curve = −0.70`, `stamina_recovery = 0.45`, `pattern_recognition = 0.40`, `composure_floor = 0.35`, `mentality = +0.20`, `ambition = 0.55`, `learning_rate = 0.40`, `aerial = 0.25`, `height_ceiling = 0.50`, `frame_density = 0.40`, `striking = 0.55`, `first_touch = 0.60`, `dead_ball = 0.30`, `left_foot = 0.15`, `injury_resilience = 0.55`, `aging_curve = 0.50`, `decision_velocity = 0.55`. Role: `RW`.

**Family scores:**
- Pace: `0.60 × 0.85 + 0.40 × norm(−0.70)` = `0.510 + 0.40 × 0.15` = `0.510 + 0.060` = **0.570**
- Finishing: `0.45 × 0.55 + 0.35 × 0.60 + 0.20 × 0.85` = `0.248 + 0.210 + 0.170` = **0.628**
- Composure: `0.45 × 0.35 + 0.35 × norm(+0.20) + 0.20 × 0.55` = `0.158 + 0.35 × 0.60 + 0.110` = `0.158 + 0.210 + 0.110` = **0.478**
- Stamina: `0.45 × 0.45 + 0.35 × 0.50 + 0.20 × 0.55` = `0.203 + 0.175 + 0.110` = **0.488**

**Selected compiled attributes (before jitter):**
- `pace`: `0.80 × 0.570 + 0.20 × 0.628` = `0.456 + 0.126` = **0.582**
- `dribbling`: `0.50 × 0.570 + 0.30 × 0.628 + 0.20 × 0.478` = `0.285 + 0.188 + 0.096` = **0.569**
- `tackling`: `0.70 × DefAnt + …` — DefAnt ≈ 0.47 (moderate `pattern_recognition`, `decision_velocity`) → tackling ≈ **0.35** (winger, low naturally)

**Role-affinity amplification (RW weights — pace/dribbling/crossing high; tackling near zero):**
Assume RW has `pace` weight 0.14, `dribbling` 0.15, `tackling` 0.01:
- `pace` lift: `0.14 × 0.20 × (1.0 − 0.582)` = `0.028 × 0.418` = +0.012 → **0.594**
- `tackling` lift: `0.01 × 0.20 × 0.65` ≈ +0.001 → **0.351** (negligible)

**PA/CA arc:**
- `growth_curve = −0.70`; normalized = `(−0.70 + 1) / 2` = 0.15; `peak_age = 24 + (0.15 × 7) = 25`
- Age 21, distance to peak = 4 years; `arc_fraction = 1 − 0.55 × (4 / 10) = 1 − 0.220 = 0.780`
- `pa_score` ≈ role-weighted average of compiled attributes; for an explosive winger ≈ **0.52** (solid Championship-level)
- `ca_q32 = 0.52 × 0.780 ≈ 0.406`

**Result:** pace 0.594, dribbling ≈ 0.57, tackling ≈ 0.35 — reads as a quick winger with limited defensive contribution. CA (≈0.41) below PA (≈0.52) with 4 years to peak. Correct shape.

---

### Example B — Dominant aerial CB, age 28, standard peak

**Genes:** `height_ceiling = 0.88`, `frame_density = 0.78`, `aerial = 0.72`, `fast_twitch_ratio = 0.30`, `stamina_recovery = 0.60`, `composure_floor = 0.65`, `pattern_recognition = 0.70`, `decision_velocity = 0.60`, `growth_curve = 0.00`, `aging_curve = 0.55`. Role: `CB`.

**Key family scores:**
- AerialPresence: `0.50 × 0.72 + 0.30 × 0.88 + 0.20 × 0.78` = `0.360 + 0.264 + 0.156` = **0.780**
- DefAnt: `0.50 × 0.70 + 0.30 × 0.60 + 0.20 × 0.65` = `0.350 + 0.180 + 0.130` = **0.660**
- Pace: `0.60 × 0.30 + 0.40 × norm(0.00)` = `0.180 + 0.200` = **0.380**

**Selected attributes (before jitter):**
- `heading`: `0.90 × 0.780 + 0.10 × Composure` = `0.702 + 0.065` ≈ **0.767**
- `jumping_reach`: `0.85 × 0.780 + 0.15 × 0.488` = `0.663 + 0.073` ≈ **0.736**
- `strength`: `0.60 × 0.780 + 0.40 × 0.488` = `0.468 + 0.195` ≈ **0.663**
- `pace`: `0.80 × 0.380 + 0.20 × Finishing` ≈ **0.330**
- `tackling`: `0.70 × 0.660 + 0.20 × WorkRate + 0.10 × 0.780` ≈ **0.570**

**Role-affinity CB amplification** (tackling/marking/heading/strength/positioning high-weight):
- `heading` at CB weight ≈ 0.12: lift = `0.12 × 0.20 × (1 − 0.767)` = +0.006 → **0.773**
- `pace` at CB weight ≈ 0.03: minimal lift

**Age arc:** `peak_age = 27` (growth_curve = 0.0); age 28, delta = +1; decline = `1 − 0.018 × 1 = 0.982`; CA ≈ `0.58 × 0.982 ≈ 0.570`.

**Result:** heading 0.773, jumping_reach 0.736, strength 0.663, pace 0.330. Reads as a dominant aerial CB near prime with limited pace. CA ≈ PA at 28 — correct.

---

### Example C — Late-bloomer deep midfielder, age 23, `LateBloomer` flag

**Genes:** `pattern_recognition = 0.75`, `decision_velocity = 0.72`, `composure_floor = 0.68`, `ambition = 0.70`, `learning_rate = 0.75`, `growth_curve = +0.80`, `mentality = −0.30`, `fast_twitch_ratio = 0.35`. Role: `DM`. Flag: `LateBloomer`.

**Key family scores:**
- Passing: `0.40 × 0.75 + 0.35 × 0.72 + 0.25 × first_touch` ≈ **0.672**
- DefAnt: `0.50 × 0.75 + 0.30 × 0.72 + 0.20 × 0.68` = **0.727**
- WorkRate: `0.40 × 0.70 + 0.35 × 0.75 + 0.25 × norm_inv(−0.30)` = `0.280 + 0.263 + 0.25 × 0.65` = **0.706**

**Narrative flag effect:**
- `LateBloomer`: add 3 to peak_age. `peak_age = 24 + (0.40 × 7/2 + 0.5 × 7) = ~31 + 3 = 34` (capped at 34).
- Distance to peak at age 23 = 11 years. `arc_fraction = 1 − 0.55 × (11/18) ≈ 0.664`

**Result:** compiled `decisions ≈ 0.70`, `positioning ≈ 0.65`, `stamina ≈ 0.62`, but `pa_score ≈ 0.63`, `ca_q32 ≈ 0.63 × 0.664 ≈ 0.418`. The player reads correctly as technically gifted (high passing/DefAnt) but showing only 42% of potential at 23. The late-bloomer flag means they will not peak until 34 — which is the career story.

---

## 10. Determinism and invariants

### 10.1 Seed hygiene

```rust
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn gene_to_attributes(gene, role, table, age, seed) -> GeneAttributeResult {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    // 55 sequential draws, one per attribute in KNOWN_ATTRIBUTE_NAMES order
}
```

The `seed` must be derived by the caller as:
```
seed = seed_fn(career_seed, player_id_as_u32 as u64, SeedLayer::ContentBake, 0)
```

This is the only RNG use in the function. All other arithmetic is deterministic Q32.

### 10.2 Proptest invariants

The implementer must add the following `proptest` invariants in `crates/fw-content/tests/gene_attribute_compiler.rs`:

**Invariant 1 — range:** for any `(gene, role, age, seed)`, all 55 compiled attributes are in `[Q32::ZERO, Q32::ONE]` and `ceiling.current() <= ceiling.potential()`.

**Invariant 2 — same-input determinism:** for any `(gene, role, age, seed)`, calling the compiler twice returns identical results.

**Invariant 3 — physique monotonicity:** for any gene snapshot where only `height_ceiling` is varied, `jumping_reach` is non-decreasing. Similarly, `pace` is non-decreasing in `fast_twitch_ratio`.

**Invariant 4 — PA ≥ CA:** for any input, `AbilityCeiling::try_new(ca, pa)` is `Ok`.

**Invariant 5 — late bloomer lower CA at 23:** for any gene snapshot with `LateBloomer` vs the same without, the `LateBloomer` player's CA is lower at age 23 and equal-or-higher at age 30.

**Invariant 6 — role differentiation:** a GK-role compilation produces a higher `handling + reflexes + one_on_ones` sum than the same genes compiled for `ST`.

**Invariant 7 — 1000-gene balance sweep:** draw 1000 random gene snapshots from the arbitrary proptest strategy; for each, compile for `CB` and `ST`; assert no attribute outside `[Q32::ZERO, Q32::ONE]` and no `ceiling.current > ceiling.potential`. This is the T4.5-E0 acceptance gate.

### 10.3 Insta snapshot

Seed one specific `GeneSnapshot` (e.g. the all-mid-range `valid_gene_snapshot()` from `gene.rs` tests) with role `CB` and age 27, seed `0xdeadbeef_cafe_f00d`. Run the compiler. Snapshot the output with `insta`. This pins the baseline against which any formula change is immediately visible.

---

## 11. How this retires the mirror substrate (FUN-DR dependency)

Today `drama_sweep/main.rs:325-334` creates all players via `PlayerAttributes::mid_range_baseline()`. Every attribute is exactly 0.5; `shooter_quality` is fed 0.5 constant; the quality differential between the two sides is zero.

FUN-DR replaces this with:
1. **T4.5-E0 (this compiler):** given a `GeneSnapshot` (random-but-seeded), produce a real `PlayerAttributes`.
2. **FUN-DR sweep harness:** generate two squads with different average gene distributions (e.g. one squad's PA centered at 120, the other at 90). Run `drama_sweep` over them. Assert `S2 ∈ 25-40%` (higher-rated side wins more than chance but doesn't dominate) and that `M1` stays in the 2.3-3.2 goals/match band. Both squads use differentiated archetypes from the role-affinity table.

The mirror substrate is formally retired when the FUN-DR gate passes. From that point, `mid_range_baseline()` is a test-only utility (it stays in the code — it's still useful for isolated attribute-scorer tests where you want a known neutral input).

---

## 12. Stats-panel identity descriptor

The stats panel for a player surfaces an identity descriptor ("technical artist / physical specimen / late bloomer") that reads the compiled attribute profile. With the mirror substrate, every player's profile is identical and the descriptor is noise. With this compiler, the descriptor becomes grounded.

The descriptor reads the compiled `PlayerAttributes` after T4.5-E0 runs. It should be derived as:

- **Technical artist:** `(finishing + passing + technique + first_touch + dribbling) / 5 > 0.62`
- **Physical specimen:** `(pace + strength + jumping_reach + stamina) / 4 > 0.65`
- **Defensive anchor:** `(tackling + marking + anticipation + positioning) / 4 > 0.60`
- **Engine:** `(stamina + work_rate_personality + concentration) / 3 > 0.60`
- **Late bloomer:** `LateBloomer` in `narrative_flags` AND age < 26

Multiple tags can apply. Priority order for the primary label (if exactly one must be chosen): Physical > Technical > Defensive > Engine > Late Bloomer. This is a UI concern passed to `narrative-director` for the final wording; this spec establishes the numeric thresholds.

Threshold rationale: 0.62 for technical is the 62nd percentile of a normally-distributed population centered on 0.50. A "technical artist" should be meaningfully above average but not rare — roughly 1 in 3 forwards should qualify. 0.65 for physical is higher because elite athleticism is genuinely less common than above-average technique. These are soft tuning dials — adjust after the FUN-DR balance sweep shows the distribution.

---

## 13. Phase-4.5 tuning bands

| Dial | Current seed | Expected range | Notes |
|---|---|---|---|
| `JITTER_HALF` | 0.045 | 0.030–0.060 | Tighter if generated squads feel too varied game-to-game; wider if players feel homogeneous within a role |
| `ROLE_AMP` | 0.20 | 0.12–0.28 | Lower if role-primary attributes are already high from genes; higher if role differentiation is too subtle |
| Arc fraction floor | 0.45 | 0.38–0.52 | The minimum CA/PA ratio for the youngest players (age 15); lower means larger youth development headroom |
| Arc decline rate | 0.018/year | 0.014–0.022 | Rate of decline post-peak; lower for longer "peak window" careers |
| Narrative flag PA bonus (`PeakCeilingHigh`) | +0.07 | +0.05–0.09 | Rare carriers should feel meaningfully higher ceiling without being obviously broken |
| Narrative flag CA bonus (`AwakeningDormant`, post-25) | +0.08 | +0.06–0.12 | The "found another gear" late-career story; should be perceptible but not make the player a youth-level prodigy |
| Technical artist threshold | 0.62 | 0.58–0.66 | |
| Physical specimen threshold | 0.65 | 0.60–0.70 | |

All of these live in this document as Phase-4.5 seeds. When the T4.5-E1 balance harness runs over ~2000 generated players and FUN-DR verifies differentiated-roster drama, append a dated re-fit block under the heading `## YYYY-MM-DD T4.5 balance-harness re-fit`. Do not delete the seeds above — audit trail.

---

## 14. Open questions for the implementer

1. **Crate location:** the compiler needs `GeneSnapshot` (fw-content) and `RoleAffinityTable` (fw-content) and `PlayerAttributes` (fw-core). The natural home is `fw-content` as a new module `gene_attribute_compiler.rs`, exposing `pub fn gene_to_attributes`. Verify no crate-cycle is introduced (fw-core has no fw-content dep; this stays fine).

2. **Sharing gene_score functions:** the 10 `gene_score_*` functions in `breakthrough_input.rs` are currently private. Promote them to `pub(crate)` inside a `gene_scores` sub-module so this compiler can call them without duplication. One proptest over the gene_score functions then covers both callers.

3. **RoleWeights for GK personality attributes:** the personality and durability fields have no role weights in the current affinity table (because they're hidden, not CA inputs). Step D's amplification only applies to the 38 visible attributes. Confirm this at implementation — the personality mapping in §5.5 is pure-gene, not role-amplified.

4. **Age as a `u8` vs a `Tick`:** the caller likely has a player's birth year and the current season year. Deriving `age = current_year - birth_year` is the simplest path; no tick-level resolution needed for the arc formula.

5. **`AwakeningDormant` age threshold:** the gene spec says "post-25." The arc formula uses exact age. Confirm the flag bonus should apply continuously for all ages >= 25 (not just at one moment), which is what the formula above assumes.

---

## 15. Cross-references

- `crates/fw-content/src/gene.rs` — `GeneSnapshot`, 22 fields, 4 categories
- `crates/fw-core/src/player_attributes.rs` — `PlayerAttributes` (55 fields), `AbilityCeiling`, `validate_unit_range`
- `crates/fw-content/src/role_affinity.rs` — `RoleAffinityTable`, `RoleWeights`, basis-point schema
- `crates/fw-content/src/breakthrough_input.rs` — gene→family bridge; `gene_score_*` functions to reuse
- `docs/design/progression.md` §"Gene→family PA/CA bridge (T4-2.5a)" — the anchor-weight table this compiler calls into
- `docs/MASTER_PLAN.md` rows T4.5-E0, T4.5-E1, FUN-DR, FUN-PI1, FUN-PI2
- `Sim/RULES.md §1, §2, §4, §11` — no floats, BTreeMap, ChaCha8Rng, assert not debug_assert
- ADR-0002 — PlayerAttributes schema and AbilityCeiling encapsulation contract
- ADR-0009 — `SeedLayer` enum; `ContentBake` is the correct discriminant here
# Personality bias weights — k₁..k₁₄ multiplicative coefficients

**Status:** Tranche 4 design doc for T1-2b. Coefficient values per design-docs/RULES.md §4.

**Implements:** ADR-0003 §5 (personality bias — multiplicative per consideration).

---

## Scope

ADR-0003 §5 specifies the personality bias as a multiplicative per-consideration tilt with 14 coefficients (k₁..k₁₄), 7 considerations × 2 biases per (primary + secondary) — though some considerations have 3 inputs. This doc:

1. Maps each k coefficient to its consideration + bias slot.
2. Defines Phase-1 seed values.
3. Specifies the full 7-consideration × 14-bias matrix (the table ADR-0003 §5 promised).

---

## Bias surface recap (from ADR-0003 §5)

Match-tick bias inputs read from two ADR-0002 surfaces:

**Hidden (`PersonalityVector`, 14 fields):** Determination, WorkRate, Ambition, Professionalism, Loyalty, Temperament, PressureTolerance, BigMatchAppetite, Adaptability, Aggression, RiskAppetite, Selflessness, Consistency, Versatility.

**Visible (`MentalAttributes`, 2 fields used as bias-like inputs):** Flair (referenced as `FlairBias` in the table below), Composure (referenced as `Composure`).

Eight named biases drive the 7-consideration mapping: Determination, PressureTolerance, FlairBias (mental.flair), WorkRate, Aggression, Selflessness, RiskAppetite, Composure (mental.composure).

The remaining 11 PersonalityVector elements carry over into longer-tail systems (transfer / dressing-room / press / big-match heuristics) and DO NOT directly tilt match-tick utility scores. They're consumed elsewhere — see `bt-attribute-binding.md` "Long-tail consumers" notes.

---

## The 12-consideration mapping table (Phase-1 seeds + T2-R1 long-tail visibility)

**T2-R1 docs-honesty pass (2026-05-18)**: this table previously listed only the 7 primary considerations (Shoot / Long pass / Safe pass / Dribble / Press / Cover / Hold position) with coefficients k₁..k₁₄. The live `personality_bias.rs` source has shipped **21 K constants** since the T1-2b-fix spec-drift corrections — k₁₅..k₂₁ cover 5 additional per-site considerations (Cross / Lay-off / Mark / Run off ball / Hold formation) + k₁₈ adds the RiskAppetite tertiary tilt on Shoot. All 21 are now table-visible so the design-doc-driven re-tuning cadence + `calibrate fit-personality` audit-trail can land them coherently. The old "7 × 8" framing was the gap Track B-4 of the post-T2 ultimate-review flagged.

Each consideration carries a primary + (often) a secondary bias; Shoot now carries a third (`RiskAppetite`-driven audacious-shot tilt). Coefficients k₁..k₂₁ apply multiplicatively as `utility · (1 + k_i · bias_value)`.

Bias values are Q32 in `[0, 1]` (or `[-1, 0]` via the "1 − bias" inverse form on RiskAppetite for safe-pass / hold considerations).

| Consideration | Primary bias | k_primary | Secondary bias | k_secondary | Tertiary bias | k_tertiary | Form |
|---|---|---|---|---|---|---|---|
| **Shoot (xG)** | FlairBias | k₁ = 0.30 | Composure (under pressure) | k₂ = 0.40 | RiskAppetite (audacious-shot tilt) | k₁₈ = 0.40 | `xg · (1 + 0.30·FlairBias) · (1 + 0.40·Composure·defender_pressure) · (1 + 0.40·RiskAppetite)` |
| **Long pass / through ball** | RiskAppetite | k₃ = 0.45 | FlairBias | k₄ = 0.25 | — | — | `xt_delta · (1 + 0.45·RiskAppetite) + 0.25·FlairBias·is_progressive` |
| **Safe pass** | (1 − RiskAppetite) (inverse) | k₅ = 0.35 | Selflessness | k₆ = 0.30 | — | — | `xt_delta · (1 + 0.35·(1−RiskAppetite)) · (1 + 0.30·Selflessness)` |
| **Dribble** | FlairBias | k₇ = 0.40 | Aggression | k₈ = 0.25 | — | — | `dribble_value · (1 + 0.40·FlairBias) · (1 + 0.25·Aggression)` |
| **Press (counter or sustained)** | Aggression | k₉ = 0.45 | WorkRate | k₁₀ = 0.35 | — | — | `press_value · (1 + 0.45·Aggression) · (1 + 0.35·WorkRate)` |
| **Defensive cover / track-back** | Determination | k₁₁ = 0.40 | WorkRate | k₁₂ = 0.35 | — | — | `cover_value · (1 + 0.40·Determination) · (1 + 0.35·WorkRate)` |
| **Hold position** | (1 − Aggression) (inverse) | k₁₃ = 0.35 | PressureTolerance | k₁₄ = 0.30 | — | — | `hold_value · (1 + 0.35·(1−Aggression)) · (1 + 0.30·PressureTolerance)` |
| **Cross** | WorkRate | k₁₅ = 0.35 | FlairBias | k₁₆ = 0.30 | — | — | `cross_value · (1 + 0.35·WorkRate) · (1 + 0.30·FlairBias)` |
| **Lay-off** | Selflessness | k₁₇ = 0.35 | — | — | — | — | `lay_off_value · (1 + 0.35·Selflessness)` |
| **Mark player** | Determination | k₁₉ = 0.40 | — | — | — | — | `mark_value · (1 + 0.40·Determination)` |
| **Run off ball** | RiskAppetite | k₂₀ = 0.35 | — | — | — | — | `run_off_value · (1 + 0.35·RiskAppetite)` |
| **Hold formation** | Professionalism | k₂₁ = 0.35 | — | — | — | — | `hold_form_value · (1 + 0.35·Professionalism)` |

### Why values in [0.25, 0.45]

A coefficient of 0.40 means a max-personality bias (1.0) swings the utility by 40%. Two biases stacking multiplicatively on a shot: `(1 + 0.30·1.0) · (1 + 0.40·1.0) = 1.30 · 1.40 = 1.82` — an 82% utility boost for a perfectly-disposed player. Three-quartile coefficients keep personality a TILT, not the dominant factor. Larger values would make personality dominate the math primitives (xG / xT / pitch-control), which the synthesis (`docs/research/sports-sims/00-synthesis.md` line 94) explicitly warns against.

### PressureTolerance — the one bias amplifier

Per ADR-0003 §5, `PressureTolerance` additionally divides `defender_pressure` reads everywhere it appears as a feature input. Not a tilt — a true amplifier. Implementation:

```rust
fn read_defender_pressure(player: &Player, raw_pressure: Q32) -> Q32 {
    let pt = player.attributes.personality.pressure_tolerance;
    // High PressureTolerance → effective pressure ↓.
    raw_pressure / (Q32::ONE + Q32::from_fraction(75, 100) * pt)
    // PT=0 → raw; PT=1 → raw / 1.75 (≈57% of raw)
}
```

The 0.75 coefficient there is the "PT divisor" tuning value; lives in this doc alongside the k₁..k₁₄ multipliers.

---

## Long-tail bias readers (NOT match-tick — for cross-reference only)

| Bias | Read by (system, phase) | Function |
|---|---|---|
| `Ambition` | Transfer market (T2-5+) | High ambition → more likely to push for transfer to bigger clubs |
| `Professionalism` | Training (T2+), Dressing room (T3+) | Higher = consistent attendance + form floor |
| `Loyalty` | Contract renewal (T2-5+) | Higher = more likely to accept renewals at home club |
| `Temperament` | Discipline events, press conferences (T3+) | Inverse → controversy, red cards |
| `BigMatchAppetite` | Match-tick MODULATOR at high-stakes matches (T3+) | Multiplier on all other biases during knockout / final fixtures |
| `Adaptability` | Transfer integration (T2-5+) | Higher = faster CA recovery post-transfer |
| `Consistency` | Form modelling (T3+) | Inverse → larger week-to-week form variance |
| `Versatility` | Role-switching, position-flexibility (T2+) | Higher = smaller penalty in non-preferred roles |
| `Durability.injury_proneness` | Injury sim (T2+) | Higher = more injuries |
| `Durability.recovery_rate` | Injury return (T2+) | Higher = faster comeback |
| `Durability.dirtiness` | Foul propensity, card propensity (T2+) | Higher = more fouls + cards |

---

## Signature-bias snapshot integration

When a signature fires (ADR-0011), the signature's `SimBiasSnapshot` applies AS WELL AS the personality biases above, multiplied through. Stacking example:

For a `LongRangeStrike` signature firing on a player at the moment of a Shoot decision:
- Personality contributes `(1 + 0.30·FlairBias) · (1 + 0.40·Composure·pressure)`.
- Signature contributes `xg_multiplier = 1.4` per the signature's `SimBiasSnapshot.xg_mul`.
- Combined utility = `xg · personality_factor · signature_factor`.

The personality + signature factors are computed independently and multiplied at the consideration level. Same shape across all 7 considerations.

---

## Re-tuning cadence

Phase-1 seeds (above) are hand-tuned for "feel" — they MUST be re-fit during T2-1 against archetype-paired match output. The fitting procedure:

1. Run 100 archetype-paired matches with logging of (decision, personality, signature, outcome).
2. Compute, per consideration, the empirical "personality-impact-on-decision-frequency" curve.
3. Adjust k_i so the curve matches the design intent (e.g. high-FlairBias players take 50% more shots than low-FlairBias in the same position).
4. Pin the re-fit values here; commit alongside a `corpus_version` bump.

The re-fit happens at the END of T2 (after the BT runner has matured), NOT continuously.

### 2026-05-17 T2-1d-infra block — calibration tooling exists; K_i NOT YET re-fit

T2-1d shipped the calibration INFRASTRUCTURE (not the K_i re-fit itself).
The new `crates/fw-match-sim/src/bin/calibrate.rs` binary provides the
`fit-personality` subcommand that reads the `target/calibration-corpus.json`
dumped by `calibrate run --matches N` and outputs PROPOSED Q32 raw-bits
for the 5 most-load-bearing shoot+dribble-bias K constants (K_1
SHOOT_FLAIR, K_2 SHOOT_COMPOSURE, K_7 DRIBBLE_FLAIR, K_8 DRIBBLE_AGG,
K_18 SHOOT_RISK). The other 16 K constants remain Phase-1 seeds per
this doc's explicit "wait for BT to mature" caveat — they'll be re-fit
at T2-1d3 / end-of-T2 in a dedicated row once T2-2/T2-3/T2-4/T2-5
mature the BT runner + content corpus.

**K_i NOT YET applied to source.** T2-1d-infra defers the const updates
to T2-1d2 follow-up row for the same reason xG β values are deferred:
`bt/on_ball.rs::utility_shoot` currently uses a hand-tuned stub instead
of `xg_utility(ShotContext)`, so the personality-bias multipliers also
feed into a stub that doesn't yet use the calibrated bias semantics.
Once T2-1d2 rewires `utility_shoot` to use the xG model, the K_i fit
becomes meaningful + can be applied atomically.

Empirical fit method (Rust-only; no sklearn dep): for each of the 5
in-scope K constants, the `fit-personality` subcommand computes top-
quartile vs bottom-quartile attribute means from the corpus +
analytically solves for K such that the action-frequency ratio
(top/bottom) meets the design-doc target of ≥1.40. If the corpus
sample size is insufficient (<8 records) OR the denominator is
degenerate (top_mean ≤ target_ratio × bot_mean), the subcommand
holds the current Phase-1 K value + flags this in the
`target/k-fit-result.json` provenance record.

The Phase-1 K values remain canonical:

```rust
pub const K_1_SHOOT_FLAIR:      Q32 = Q32::from_raw(1_288_490_188); // ≈ 0.30
pub const K_2_SHOOT_COMPOSURE:  Q32 = Q32::from_raw(1_717_986_918); // ≈ 0.40
pub const K_7_DRIBBLE_FLAIR:    Q32 = Q32::from_raw(1_717_986_918); // ≈ 0.40
pub const K_8_DRIBBLE_AGG:      Q32 = Q32::from_raw(1_073_741_824); // ≈ 0.25
pub const K_18_SHOOT_RISK:      Q32 = Q32::from_raw(1_717_986_918); // ≈ 0.40
```

T2-1d2 will re-run `calibrate fit-personality` AFTER `utility_shoot`
rewiring + commit the post-fit K block under a "**2026-MM-DD T2-1d2
re-fit**" header per this section's step-4 audit-trail discipline.

---

## Test contract

T1-2b stub acceptance:

1. **Coefficient values exist** — `crates/fw-match-sim/src/bt/personality_bias.rs` declares `const K_1..K_14: Q32` matching this doc.
2. **Multiplicative composition** — `proptest`: for any personality + raw utility, the biased utility is in a reasonable range (`raw / 2 ≤ biased ≤ 2·raw` for the max-coefficient case).
3. **PressureTolerance divisor** — `read_defender_pressure` returns raw for PT=0 and approximately raw/1.75 for PT=1.

T2-1 re-fit acceptance (later):

4. **Personality-impact curve fit** — across 100 archetype-paired matches, high-FlairBias players (top quartile) take ≥40% more shots than low-FlairBias players (bottom quartile) in equivalent positions.
5. **No-domination invariant** — even with maxed bias, the math primitive (xG / xT / pitch-control) remains the dominant input. Test: scaling raw xG by 2× should still produce ≥1.5× output, even when personality is maxed.

---

## Cross-references

- ADR-0003 §5 (the mapping table this doc tunes)
- ADR-0011 §"Bias snapshot" (signature biases stack with personality biases)
- ADR-0002 §"Choices" item 2 (the 14-element PersonalityVector)
- `docs/specs/bt-attribute-binding.md` (where each consideration's BT site reads personality)
- `docs/design/xg-coefficients.md` (Phase-1 xG seeds — composes with k₁ + k₂ for Shoot)
- `docs/research/sports-sims/00-synthesis.md` line 94 (the "personality = small scalar vector, MULTIPLICATIVE" research recommendation)

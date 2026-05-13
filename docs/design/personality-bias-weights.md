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

## The 7 × 8 mapping table (Phase-1 seeds)

Each consideration carries a primary + secondary bias. Some carry a third (RiskAppetite inverse form). Coefficients k₁..k₁₄ apply multiplicatively as `utility · (1 + k_i · bias_value)`.

Bias values are Q32 in `[0, 1]` (or `[-1, 0]` via the "1 − bias" inverse form on RiskAppetite for safe-pass / hold considerations).

| Consideration | Primary bias | k_primary | Secondary bias | k_secondary | Tertiary bias | k_tertiary | Form |
|---|---|---|---|---|---|---|---|
| **Shoot (xG)** | FlairBias | k₁ = 0.30 | Composure (under pressure) | k₂ = 0.40 | — | — | `xg · (1 + 0.30·FlairBias) · (1 + 0.40·Composure·defender_pressure)` |
| **Long pass / through ball** | RiskAppetite | k₃ = 0.45 | FlairBias | k₄ = 0.25 | — | — | `xt_delta · (1 + 0.45·RiskAppetite) + 0.25·FlairBias·is_progressive` |
| **Safe pass** | (1 − RiskAppetite) (inverse) | k₅ = 0.35 | Selflessness | k₆ = 0.30 | — | — | `xt_delta · (1 + 0.35·(1−RiskAppetite)) · (1 + 0.30·Selflessness)` |
| **Dribble** | FlairBias | k₇ = 0.40 | Aggression | k₈ = 0.25 | — | — | `dribble_value · (1 + 0.40·FlairBias) · (1 + 0.25·Aggression)` |
| **Press (counter or sustained)** | Aggression | k₉ = 0.45 | WorkRate | k₁₀ = 0.35 | — | — | `press_value · (1 + 0.45·Aggression) · (1 + 0.35·WorkRate)` |
| **Defensive cover / track-back** | Determination | k₁₁ = 0.40 | WorkRate | k₁₂ = 0.35 | — | — | `cover_value · (1 + 0.40·Determination) · (1 + 0.35·WorkRate)` |
| **Hold position** | (1 − Aggression) (inverse) | k₁₃ = 0.35 | PressureTolerance | k₁₄ = 0.30 | — | — | `hold_value · (1 + 0.35·(1−Aggression)) · (1 + 0.30·PressureTolerance)` |

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

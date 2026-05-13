# BT attribute binding — which `PlayerAttributes` fields each BT decision site reads

**Status:** Tranche 4 spec for T1-2b. The critical companion spec — without it, the 55-attribute model has no consumption contract.

**Implements:** ADR-0001 layer 2 (per-player decision runner) + ADR-0006 (FSM-of-BTs).

**Consumes:** ADR-0002 (the 55-field player attribute model), ADR-0003 (decision-utility math primitives), ADR-0011 (signatures bias BT scores).

---

## Scope

Every BT decision site (leaf + utility-selector) reads a small, named subset of `fw_core::PlayerAttributes` fields. This spec is the **single authoritative table** that names every binding. It exists so:

1. New BT sites can't sprawl into reading 20 attributes per decision. Tight bindings → readable behavior + manageable tuning surface.
2. Removing or renaming an attribute is auditable — `grep` this file to find consumers before changing the schema.
3. Tuning per-attribute is bounded — coefficient files (`docs/design/personality-bias-weights.md` et al.) align with the bindings here.

---

## Binding format

```
[BT Site] = (primary attrs) + (secondary attrs) + (bias from PersonalityVector/MentalAttributes)
```

- **Primary attrs** are read at full weight in the utility/predicate.
- **Secondary attrs** are tie-breakers or context modifiers.
- **Bias inputs** are the personality / mental fields per ADR-0003 §5.

Each binding lists ≤ 5 primary + ≤ 5 secondary + the relevant bias slots. If a site needs more, the design needs revisiting — that's the discipline.

---

## On-ball decision sites (utility-selector — ADR-0003 §1–§5)

### Shoot

Reads:
- **Primary:** `technical.finishing`, `technical.long_shots` (long-distance shots only), `mental.composure`, `mental.decisions`
- **Secondary:** `mental.vision` (assess angles), `physical.balance` (off-balance penalty)
- **Bias (ADR-0003 §5):** `FlairBias` (mental.flair), `Composure` (mental.composure), `RiskAppetite` (personality.risk_appetite)

Math: xG sigmoid (`docs/design/xg-coefficients.md`) × bias multiplicatives.

### Pass — short

Reads:
- **Primary:** `technical.passing`, `technical.first_touch`, `technical.technique`, `mental.vision`
- **Secondary:** `mental.composure` (pressure effects), `mental.decisions`
- **Bias:** `Selflessness` (personality.selflessness), `RiskAppetite` (inverse — high RiskAppetite ↓ safe-pass utility)

### Pass — long / through-ball

Reads:
- **Primary:** `technical.passing`, `mental.vision`, `mental.decisions`, `technical.long_shots` (proxy for ball-power)
- **Secondary:** `mental.composure` (under pressure), `mental.anticipation` (off-ball receiver timing)
- **Bias:** `FlairBias`, `RiskAppetite` (high RiskAppetite ↑ utility)

### Cross

Reads:
- **Primary:** `technical.crossing`, `mental.vision`, `physical.pace` (carry-and-cross)
- **Secondary:** `technical.first_touch`, `mental.anticipation`
- **Bias:** `WorkRate` (personality.work_rate), `FlairBias`

### Dribble — beat marker

Reads:
- **Primary:** `technical.dribbling`, `technical.technique`, `physical.agility`, `physical.acceleration`
- **Secondary:** `physical.balance`, `mental.flair`
- **Bias:** `FlairBias`, `Aggression` (personality.aggression)

### Hold position / shield ball

Reads:
- **Primary:** `physical.strength`, `mental.composure`, `physical.balance`
- **Secondary:** `mental.decisions`
- **Bias:** `−Aggression` (inverse), `PressureTolerance` (personality.pressure_tolerance)

### Lay-off / one-touch return

Reads:
- **Primary:** `technical.first_touch`, `technical.passing`, `mental.vision`
- **Secondary:** `mental.teamwork`, `mental.composure`
- **Bias:** `Selflessness`

---

## Off-ball positioning sites (4 Hz BT decision runner)

### Defensive cover — track-back

Reads:
- **Primary:** `mental.positioning`, `mental.anticipation`, `physical.pace`, `physical.stamina`
- **Secondary:** `mental.concentration`, `mental.teamwork`
- **Bias:** `Determination` (personality.determination), `WorkRate`

### Press — initiate

Reads:
- **Primary:** `mental.anticipation`, `physical.acceleration`, `physical.stamina`, `mental.work_rate` *(NOTE: work_rate lives on PersonalityVector, not MentalAttributes — read via the bias path, not as a primary)*
- **Secondary:** `mental.positioning`, `physical.pace`
- **Bias:** `Aggression`, `WorkRate`

### Mark — close marker

Reads:
- **Primary:** `mental.marking` *(NOTE: `marking` lives on TechnicalAttributes, not MentalAttributes — verify field path at T1-2b implementation; the binding here is to `technical.marking`)*, `mental.anticipation`, `physical.pace`, `mental.concentration`
- **Secondary:** `physical.strength`, `physical.balance`
- **Bias:** `Determination`

### Running off-ball — make a run

Reads:
- **Primary:** `mental.off_the_ball`, `physical.pace`, `physical.acceleration`, `mental.anticipation`
- **Secondary:** `mental.flair`, `physical.stamina`
- **Bias:** `WorkRate`, `RiskAppetite`

### Hold formation slot

Reads:
- **Primary:** `mental.positioning`, `mental.teamwork`, `mental.concentration`
- **Secondary:** `mental.decisions`
- **Bias:** `Professionalism` (personality.professionalism), `Determination`

---

## Reactive interrupt predicates (60 Hz — ADR-0001 layer 6)

These fire cheap per-tick predicates that preempt the 4 Hz decision. They read a TIGHT subset:

### Ball reached defensive third (own goal threatened)

Reads: `mental.positioning`, `mental.bravery`, `mental.anticipation`
Bias: `Determination`

### Shot incoming (defending goalkeeper attention)

Reads (GK only): `goalkeeper.reflexes`, `goalkeeper.positioning`, `goalkeeper.handling`
Bias: `Composure` (mental.composure)

### Marker arrived (under pressure, off-ball player)

Reads: `mental.composure`, `physical.balance`, `mental.anticipation`
Bias: `PressureTolerance`

### Through-ball intercept

Reads: `mental.anticipation`, `mental.positioning`, `physical.pace`
Bias: `Aggression`

---

## Goalkeeper-specific decision sites (ADR-0006 — GK is pure FSM, not FSM-of-BTs)

### Shot stopping

Reads: `goalkeeper.reflexes`, `goalkeeper.handling`, `goalkeeper.one_on_ones`, `mental.composure`
Bias: `Composure` (mental.composure)

### Cross collection

Reads: `goalkeeper.aerial_reach`, `goalkeeper.command_of_area`, `goalkeeper.handling`, `physical.jumping_reach`
Bias: `Composure`

### Sweeper rush

Reads: `goalkeeper.one_on_ones`, `goalkeeper.command_of_area`, `physical.pace`, `mental.decisions`
Bias: `Aggression`

### Distribution — short

Reads: `goalkeeper.kicking`, `technical.passing` (proxy), `mental.vision`, `mental.composure`
Bias: `Selflessness`, `RiskAppetite` (inverse)

### Distribution — long

Reads: `goalkeeper.kicking`, `mental.vision`, `mental.decisions`
Bias: `RiskAppetite`

---

## Coverage analysis

Across the bindings above, the attribute consumption count (`# of sites where this attribute appears as primary or secondary`):

**High-use (≥4 sites):** `mental.composure`, `mental.anticipation`, `mental.positioning`, `mental.vision`, `mental.decisions`, `physical.pace`, `physical.acceleration`, `physical.stamina`, `technical.passing`, `technical.first_touch`.

**Mid-use (2–3 sites):** `mental.teamwork`, `mental.concentration`, `mental.bravery`, `mental.flair`, `physical.balance`, `physical.strength`, `physical.agility`, `technical.finishing`, `technical.technique`, `technical.dribbling`, `technical.crossing`, `technical.marking`, `goalkeeper.handling`, `goalkeeper.kicking`, `goalkeeper.command_of_area`, `goalkeeper.one_on_ones`, `goalkeeper.aerial_reach`, `goalkeeper.reflexes`, `goalkeeper.positioning` *(NOTE: there is no `goalkeeper.positioning` field — see implementation note below)*.

**Low-use (1 site):** `technical.long_shots`, `technical.heading`, `technical.tackling`, `technical.free_kicks`, `technical.penalty_taking`, `technical.corners`, `technical.long_throws`, `physical.jumping_reach`, `physical.natural_fitness`, `mental.off_the_ball`.

**Zero-use at on-ball + off-ball + reactive (consumed elsewhere, e.g. set-pieces, signatures, scouts):** `technical.free_kicks` (set-piece sites), `technical.penalty_taking` (penalty), `technical.corners` (corners), `technical.long_throws` (throw-ins), `technical.heading` (set-piece + aerial duels), `technical.tackling` (tackling reactive site — add at T1-2b polish pass).

Hidden attribute usage (via the bias path, NOT directly):
- **Bias-vector consumers (every personality attribute reads through ADR-0003 §5's multiplicative bias):** `Determination`, `WorkRate`, `Ambition`, `Professionalism`, `Loyalty`, `Temperament`, `PressureTolerance`, `BigMatchAppetite`, `Adaptability`, `Aggression`, `RiskAppetite`, `Selflessness`, `Consistency`, `Versatility`. All 14.
- **Match-tick consumers (read directly as bias-like inputs):** `mental.flair` (FlairBias), `mental.composure` (Composure) — these are visible MentalAttributes that ADR-0003 §5 reads alongside the hidden vector.
- **Long-tail consumers (NOT match-tick; consumed by other systems):** `personality.ambition` (transfer market), `personality.loyalty` (contract renewal), `personality.consistency` (form modelling), `personality.adaptability` (new-club integration), `personality.versatility` (role-switching), `personality.big_match_appetite` (high-stakes match modulator), `durability.injury_proneness` (injury sim — T2+), `durability.recovery_rate` (injury return), `durability.dirtiness` (foul propensity).

Every one of the 55 fields has a consumer or a deferred-consumer. None is dead weight.

---

## Implementation notes (caveats surfaced during spec authoring)

These caveats are flagged for the T1-2b implementation pass — they're either field-path discrepancies between this spec and the ADR-0002 struct shape OR potential ADR amendments needed:

1. **`mental.work_rate` mistake** in the Press site above — `work_rate` lives on `PersonalityVector`, not `MentalAttributes`. The Press site reads it via the bias path, NOT as a primary. Corrected.
2. **`mental.marking` mistake** in the Mark site above — `marking` lives on `TechnicalAttributes`, not `MentalAttributes`. Path is `technical.marking`. Corrected.
3. **`goalkeeper.positioning` doesn't exist** — `positioning` is in `MentalAttributes`. GK shot-stopping reads `mental.positioning` (the keeper's positioning, which is a mental attribute even though the keeper is the keeper). Likewise GK distribution reads `mental.vision` etc.
4. **Reading `personality.flair` vs `mental.flair`** — `flair` is on MentalAttributes (visible), not PersonalityVector. Same for `composure`. Per ADR-0003 §5 amendment, these visible-attributes are read as bias-like inputs even though they're not hidden.

These caveats are NOT ADR amendments — they're path corrections within the spec. The 55-field schema (ADR-0002) stands.

---

## Test contract

T1-2b acceptance:

1. **Coverage assertion** — a `proptest` that walks every BT decision site and confirms it reads ONLY the attributes named in its binding above. Failures: "Site X reads attr Y not declared in binding."
2. **Field-existence test** — every attribute path named in the bindings above resolves to a real `PlayerAttributes` field (the caveats above are corrected before this test goes green).
3. **No off-binding reads** — clippy-style lint (custom) that flags any `player.attributes.<field>` read inside `fw-match-sim::bt::*` that doesn't appear in this spec's binding tables. Manual review until the lint is authored.

---

## Cross-references

- ADR-0002 §"Concrete shape" (the 55-field schema being consumed)
- ADR-0003 §1–§5 (the math that the on-ball bindings power)
- ADR-0006 §"Decision" (FSM-of-BTs structure — bindings apply per-state)
- ADR-0011 §"Bias snapshot" (signatures tilt these bindings during firings)
- `docs/specs/tactic-fsm.md` (Tranche 4 — team-level parameters that gate these per-player BT sites)
- `docs/specs/decision-cadence-stagger.md` (Tranche 4 — 22-player stagger at 4 Hz)
- `docs/design/personality-bias-weights.md` (Tranche 4 — the bias multiplier coefficients k₁..k₁₄)
- `docs/design/xg-coefficients.md` (Tranche 4 — the 6-feature logistic for Shoot site)

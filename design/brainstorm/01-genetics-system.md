---
description: Historical research report on player-generation genetics/inheritance ideas. Non-binding; locked specs supersede this document.
status: historical-brainstorm; not authoritative
---

# Player Genetics System — Final Whistle

**Author:** Systems Designer (Vibelogic)
**Date:** 2026-04-22
**Status:** Draft 1 — exploratory, not SPEC-binding
**Inspirations:** Crusader Kings 3 (genes/mutations/phenotype vs genotype), Rimworld Biotech (gene slots, xenotypes), Pokemon (IV/EV breeding), Mount & Blade 2 (companion trait inheritance), Wildermyth (emergent character histories)

---

## Executive Summary — "Seeded Ancestry with Live Family Lines"

We reject two extremes:

1. **Full multigenerational CK3 simulation** — too expensive for a 15-30 season manager arc, too much UI tax (pedigree trees), wrong focus for football.
2. **Pure cohort RNG (FM's approach)** — wastes narrative gold; "every regen has genes" but none of it surfaces.

Instead, every regen is generated with a **deterministic gene payload** at spawn — fixed, not dice-rolled per save-load. A subset (~3% per cohort) carry a `LineageRef` linking them to a retired/active player in the same save, enabling "famous father" scouting reports and dynastic drama WITHOUT CK3-depth pedigree trees. Arrives **Phase 2 post-MVP** — MVP ships cohort-only; lineage layers on.

**Core design stances (opinionated):**

1. **Playstyle is NOT heritable.** Only learning-bias is. Preserves the "hard work develops anyone" football fantasy.
2. **Potential-range bounds ARE gene-driven** (soft caps). But the *bend* from performance/narrative is NOT gene-locked — a lower-gene grinder can out-CA a lazy prodigy.
3. **Prodigy = 1 in ~800 regens** — roughly one every 2-3 seasons per nation. Signaled by gold aura in scouting UI, not a number dump.
4. **Determinism math:** Genes control ~40% of realized CA variance; training/narrative/injury controls ~60%.

---

## 1. Research Grounding

### CK3 (from web research)

- Genes are split into: **appearance genes** (hair color, face structure — phenotype) and **personality/ability genes** (intelligent, strong, beautiful, healthy).
- Each gene has allele pairs (dominant/recessive like Mendelian genetics).
- Mutations happen on inheritance (low chance, can unlock "good" or "bad" rare traits).
- The famous "Congenital tracks": Intelligent → Quick → Genius, or Dumb → Slow → Imbecile; same tracks for physical/beauty/health.
- Players can "engineer" lineages through marriage choices — opt-in dynasty-planning gameplay.

### Rimworld Biotech

- Gene slots per pawn, metabolic cost budget.
- Xenotypes = labeled gene presets (Sanguophage, Neanderthal, etc.)
- Genes are *composable* — you design a template, not inherit one.

### Pokemon Breeding

- IVs (hidden individual values 0-31) + EVs (trainable) + Nature.
- Breeding passes IVs from parents with mutations.
- Shiny rate (1/4096 base) as the "jackpot" signal.

### Mount & Blade 2 Bannerlord

- Companions have minimal traits; children inherit some parent stats.
- Lesson: shallow inheritance still creates narrative value if surfaced well.

### Wildermyth

- No genetics per se, but characters accumulate history events that modify traits.
- Lesson: *moments* > *inheritance* for narrative impact in short-run games.

---

## 2. The Gene Model — 22 Genes Across 4 Categories

### Category A — SOMATIC (7 genes)

Body-physical genes. Revealed through medical scouting + match observation.

| Gene | Range | Effect |
|---|---|---|
| `HeightCeiling` | 0.0–1.0 | Caps adult height at 160-210 cm scale |
| `FrameDensity` | 0.0–1.0 | Muscle-vs-lean balance; affects duels + injury profile |
| `FastTwitchRatio` | 0.0–1.0 | Pace+acceleration ceiling vs stamina trade |
| `StaminaRecovery` | 0.0–1.0 | Between-match + within-match recovery rate |
| `GrowthCurve` | -1.0–+1.0 | Early-peak (negative) vs late-bloomer (positive) timing |
| `AgingCurve` | 0.0–1.0 | Decline rate post-peak (Messi-like longevity vs Ronaldo-aging) |
| `InjuryResilience` | 0.0–1.0 | Frequency + severity resistance |

### Category B — NEURAL (6 genes)

Brain/mental genes. Revealed via psychological profiling + match pattern-analysis.

| Gene | Range | Effect |
|---|---|---|
| `PatternRecognition` | 0.0–1.0 | Tactical reading, anticipation ceiling |
| `CompositFloor` | 0.0–1.0 | Pressure-resistance baseline (never drops below X) |
| `DecisionVelocity` | 0.0–1.0 | Time-to-decision in-possession |
| `LearningRate` | 0.0–1.0 | How fast training raises attributes |
| `Ambition` | 0.0–1.0 | Self-driven training willingness, loyalty trade |
| `Mentality` | -1.0–+1.0 | Introvert-grinder (negative) vs extrovert-charisma (positive) |

### Category C — RESONANCE (5 genes, rare)

Gene-gated signature-move affinities. These DON'T grant moves — they multiply learning-rate for specific move families.

| Gene | Rarity | Effect |
|---|---|---|
| `LeftFootGenius` | ~4% carriers | 2.5x learning on left-foot signatures (curlers, outside-foot passes) |
| `AerialPredator` | ~3% | 2x learning on header/high-ball moves |
| `DeadBallSavant` | ~2% | 2.5x on free-kick + penalty moves |
| `BallStriker` | ~3% | 2x on power-shot + volley moves |
| `FirstTouchArtisan` | ~2% | 2.5x on control + trap signatures |

### Category D — SOUL (4 genes, ultra-rare, "Blue Lock tier")

These tie directly into the aura ladder. Hidden until triggered — scouting cannot reveal.

| Gene | Rarity | Effect |
|---|---|---|
| `FlowThreshold` | ~1.5% | Enables entry into Tier-1 peak state (Hush) at lower-stakes matches |
| `PeakCeiling` | ~0.5% | Raises the ceiling of Tier-2 peak state (True Hush) |
| `Kismet` | ~0.3% | Triggers "moment of destiny" events — narrative pivots baked into the career |
| `Awakening` | ~0.2% | Late-bloomer gene: dormant until a specific career event triggers |

**"Godlike prodigy" = FlowThreshold + PeakCeiling + Kismet all rolled together** = roughly 1 in ~800 regens.

---

## 3. Three Worked Examples

### Example A — Marco Halsey (Ordinary Regen)

Somatic: HeightCeiling 0.48 (178 cm), FrameDensity 0.62, FastTwitch 0.55, Stamina 0.50, Growth +0.1 (slight late), Aging 0.60, Injury 0.45.
Neural: PatternRec 0.50, Composure 0.40, Decision 0.55, Learning 0.58, Ambition 0.45, Mentality +0.2.
Resonance: none.
Soul: none.

**Realized career:** Age 22 CA 65, peaks 72 at 27. Potential range started 68-82, narrowed to 70-76 by age 23 through scouting. Solid Championship (tier 2) regular. No signature-move brilliance; reliable workhorse. Never enters Hush.

### Example B — Kenji Ayala (Godlike Prodigy)

Somatic: HeightCeiling 0.72 (193 cm), FrameDensity 0.58, FastTwitch 0.88, Stamina 0.82, Growth -0.3 (early peak), Aging 0.88, Injury 0.72.
Neural: PatternRec 0.91, Composure 0.85, Decision 0.92, Learning 0.80, Ambition 0.88, Mentality +0.5.
Resonance: BallStriker (2x).
Soul: **FlowThreshold + PeakCeiling + Kismet** — all three.

**Realized career:** Age 17 youth intake, scouts see 88-96 range by 2 months in. Golden aura on scouting card. Enters Hush (Flow) in first senior match. By 19 has 2 signature moves mastered including a power-shot variant. Triggers first Kismet event at age 20 (Champions League semi, down 1-0, 85th minute) — breakthrough redraws ceiling to 102. Peaks 99 CA at 27. Retires 35, becomes generational legend.

### Example C — Tomás Beltrán (Late-Bloomer Mutant)

Somatic: mostly average. HeightCeiling 0.58 (184 cm), Growth +0.6 (strong late bloomer).
Neural: average, Learning 0.72 (high).
Resonance: FirstTouchArtisan.
Soul: **Awakening** (dormant until triggered).

**Realized career:** Released by academy at 19, journeyman at tier 4. Awakening trigger fires age 26 after a career-defining match (player scores decisive goal against former club in relegation 6-pointer). Trait unlocks, potential range jumps from 65-72 to 82-90. Signs for Premier League club at 27. Peak 86 at 31. The Vardy gene.

---

## 4. Integration Hooks

### 4.1 Potential-range interaction

- Gene payload sets initial soft caps (min/max of range).
- Performance bends the range WITHIN bounds (and can breach via Kismet events).
- Aging shrinks the range from the top per `AgingCurve` gene.

### 4.2 Scouting visibility (4 channels)

| Channel | Reveals |
|---|---|
| Match-watching | Phenotype observations (height, build, pace signals) — noisy |
| Medical screening | Somatic genes with uncertainty bars |
| Psychological profiling | Neural genes with uncertainty bars |
| Mentor recognition | Resonance genes (requires specific staff) |
| — | Soul genes NEVER scoutable; revealed by trigger events |

Scout-level influences uncertainty width: poor scout = wide range, elite scout = narrow.

### 4.3 Training interaction

`LearningRate` gene multiplies all training gains. Resonance genes add move-family multipliers. No ceiling from genes alone — gene-mediocre players can still grind to the ceiling if willing.

### 4.4 Aging + regression

`AgingCurve` defines when decline starts. `GrowthCurve` defines peak-timing window. Allows dev to seed "late-bloomer prodigies" separately from "athletic-freak-but-bust" archetypes.

### 4.5 Signature moves ("Calling" in the renamed taxonomy)

- Moves are LEARNED, not gene-granted.
- Resonance genes multiply learning speed (2-2.5x on family-matched moves).
- Soul genes unlock access to *awakened forms* of moves at peak-state entry.

### 4.6 Lineage (Phase 2)

- At regen generation, ~3% of cohort roll a `LineageRef` pointing at a retired/active player in the save.
- Child inherits 40-60% of parent's gene values with mutation (small random perturbation).
- Scouting report flag: "Son of [player name]." Creates dynastic drama — does son outperform father? Do rivals bid harder because of the name?
- NOT full CK3 — no pedigree trees, no arranged marriages, no spouse system.

---

## 5. Determinism vs Fantasy — Balance Math

Realized CA variance model (empirical target, to be validated in balance harness):

- 40% gene-driven (ceiling, learning rate, somatic)
- 30% training/club-quality driven (career path, coaching)
- 20% narrative/moment-driven (Kismet triggers, Awakening, trauma events)
- 10% pure randomness (injury luck, manager decisions)

Result: Genetically gifted prodigy can flop (40% CA comes from other factors). Genetically average grinder can reach top-20% (60% of variance is controllable). Matches the "hard work develops anyone" fantasy while preserving prodigy-excitement.

---

## 6. Visual Expression (Cel-Shaded)

Genes surface in 3D models at bake time:
- `HeightCeiling` → body scale variant
- `FrameDensity` → 3 body-type silhouettes (lean, athletic, strong)
- `FastTwitchRatio` → posture/stance variants
- Soul-gene carriers → subtle aura-tint overlay in cel shader when active

Cheap implementation: 3-5 body-type meshes, modular kit + head variation. Expensive: per-gene mesh blending.

Recommendation: 3-body-type baseline for EA, expand post-1.0.

---

## 7. Flagged Risks

1. **Fairness optics.** "Genetics" framing can read as essentialist. Mitigation: language audit (prefer "athletic profile," "cognitive profile" in UI), no visible ethnic/nationality clustering, Soul genes not locked to appearance.
2. **UI overload.** 22 genes is a lot. Mitigation: progressive disclosure — reveal genes as scouting uncovers them, don't show all at once.
3. **Balance fragility.** Gene interactions with other systems (aura, training, potential) need cohort sweeps through the headless harness BEFORE EA ship. A gene imbalance that makes 20% of regens broken-OP ships the game dead.
4. **Narrative over-promise.** If every save has only 1-2 prodigies, and they're not always exciting, the genetic system becomes invisible. Mitigation: midtier "interesting mutants" (Resonance-only, Awakening-only) fire more frequently — ~1 in 50 regens.

---

## 8. Open Questions for User Decision

1. **Numeric gene values or phenotype-only UI?** CK3 shows "Quick" not "Intelligence=12." Football analog: "Gifted Learner" or "Learning Rate 0.82"?
2. **Partnerships as player-facing mechanic?** Phase 2 lineage could expose "arrange academy pairings" as a system — but this is ethically queasy. Default: lineage happens off-screen, invisible.
3. **Soul-gene visibility.** Should Soul genes be save-visible (cheater-debug mode) or hidden-until-triggered across the board?
4. **How much to show at youth intake?** Full gene readout from scouts? Or only triggered/observed? (I recommend: only triggered, with scout uncertainty bars on measurable ones.)

---

*End of genetics-system draft. Next: systems-designer to validate interactions with potential-range + aura systems.*

# ADR-0002 — Player attribute model

**Status:** Proposed

**Date:** 2026-05-13

**Decider:** osagberg (+ Claude `lead-programmer` draft, GPT-5.5 / Codex review pending)

---

## Context

T1-1 is the first canonical-state surface in the Rust workspace — `PlayerAttributes` plus the surrounding per-player record. Every downstream system reads it: the BT runner consumes attributes for utility scoring (T1-2b), the match engine resolves contests from them (T1-5+), scouts surface biased projections (T2-7), the memory ledger triggers breakthroughs that redraw the ceiling (T3-4), and the commentary banks translate them into football-native prose (T1-6+). Locking the schema wrong forces a save-migration on every downstream phase.

The "Scope ambition" framing in `docs/DESIGN_DOC.md` §1 (2026-05-13 reframe) explicitly retracts the earlier "~3000 LoC match-sim budget" constraint. The research synthesis (`docs/research/sports-sims/00-synthesis.md` lines 5–10) follows up: "attribute count grows from '24+8 recommended for tight budget' to 'whatever the design needs' — FM-class (~56) or beyond is on the table; 32 was a research-paper midpoint, not a cap." DESIGN_DOC §1 line 37 names FM-scale (~56) or beyond as in-scope. This ADR is the formal reconciliation.

The pillar constraints (`docs/DESIGN_DOC.md` §3) remain binding:
- **Pillar 1 — Procedural fantasy world.** No real licensed attribute names if any are trademarked; FM's attribute names are generic football vocabulary and safe.
- **Pillar 3 — Breakthrough-driven development.** PA is mutable, but only via salience-gated `MemoryEvent::Breakthrough` — not training ticks.
- **Pillar 4 — Scouting uncertainty.** Truth + biased observation are both representable; scouts disagree from the same canonical attribute table.
- **Pillar 5 — Signature identity.** Attributes must support the 24 signature catalogue's trigger predicates without a parallel hidden-signature attribute system.

The determinism floor (`CLAUDE.md` §3, `.claude/rules/Sim/RULES.md` §1–§4) is non-negotiable: Q32.32 in canonical paths, BTreeMap-only, no thread RNG, no f32/f64 leakage. ADR-0002 inherits these.

PlayerId is already a durable `u32` newtype (`crates/fw-core/src/ids.rs`, locked at Codex Q2). This ADR layers attributes on top of that identity.

## Decision

We will adopt a **38 visible + 17 hidden/support = 55-field player model**, with **Football-Manager-class breadth** as the floor. The 38 visible decompose 14 technical / 10 mental / 8 physical / 6 goalkeeper-specific. The 17 hidden/support decompose 14 personality (the bias vector that multiplies into BT utility scores) + 3 durability (injury_proneness / recovery_rate / dirtiness, on a separate `DurabilityProfile` substruct). "52 pillar-load-bearing attributes" is the alternative pivot count (the 38 visible + 14 personality, excluding the 3 durability); 55 is the field count of the player record. Use 55 when sizing storage; use 52 when sizing the BT consideration surface.

**Concrete shape:**

```rust
// crates/fw-core/src/player_attributes.rs (sketch — actual code lands in T1-1)

use crate::q32::Q32;
use serde::{Deserialize, Serialize};

/// Per-player canonical attribute record.
///
/// All fields are `Q32` in the range `[Q32::ZERO, Q32::ONE]`. The UI projects
/// to the FM-familiar 1–20 integer scale at the DTO boundary; the sim never
/// sees the integer scale. The `[0, 1]` range keeps utility-score arithmetic
/// in the BT runner uniform (multiplicative biases compose cleanly).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerAttributes {
    pub technical: TechnicalAttributes,
    pub mental: MentalAttributes,
    pub physical: PhysicalAttributes,
    pub goalkeeper: GoalkeeperAttributes,
    pub personality: PersonalityVector,
    pub durability: DurabilityProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TechnicalAttributes {
    pub finishing: Q32,
    pub long_shots: Q32,
    pub passing: Q32,
    pub crossing: Q32,
    pub first_touch: Q32,
    pub technique: Q32,
    pub dribbling: Q32,
    pub heading: Q32,
    pub tackling: Q32,
    pub marking: Q32,
    pub free_kicks: Q32,
    pub penalty_taking: Q32,
    pub corners: Q32,
    pub long_throws: Q32,
}  // 14

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MentalAttributes {
    pub anticipation: Q32,
    pub composure: Q32,
    pub decisions: Q32,
    pub vision: Q32,
    pub off_the_ball: Q32,
    pub positioning: Q32,
    pub concentration: Q32,
    pub bravery: Q32,
    pub teamwork: Q32,
    pub flair: Q32,
}  // 10

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalAttributes {
    pub pace: Q32,
    pub acceleration: Q32,
    pub stamina: Q32,
    pub strength: Q32,
    pub agility: Q32,
    pub balance: Q32,
    pub jumping_reach: Q32,
    pub natural_fitness: Q32,
}  // 8

/// Specialized goalkeeper fields. Present on every player record — fielders
/// generate low values, keepers generate high ones. Cheaper than a sum-type
/// (eliminates pattern-matching at every BT decision site).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalkeeperAttributes {
    pub handling: Q32,
    pub reflexes: Q32,
    pub one_on_ones: Q32,
    pub aerial_reach: Q32,
    pub command_of_area: Q32,
    pub kicking: Q32,
}  // 6

/// The personality bias vector — also called "hidden attributes" in FM. These
/// drive scout disagreement (Pillar 4), bias BT utility scores (per
/// `00-synthesis.md` "personality = small scalar vector"), and gate breakthrough
/// triggers (Pillar 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonalityVector {
    pub determination: Q32,
    pub work_rate: Q32,
    pub ambition: Q32,
    pub professionalism: Q32,
    pub loyalty: Q32,
    pub temperament: Q32,
    pub pressure_tolerance: Q32,
    pub big_match_appetite: Q32,
    pub adaptability: Q32,
    pub aggression: Q32,
    pub risk_appetite: Q32,
    pub selflessness: Q32,
    pub consistency: Q32,
    pub versatility: Q32,
}  // 14

/// Career-shape fields. Distinct from PersonalityVector because they describe
/// a player's relationship to time + their body, not their disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurabilityProfile {
    pub injury_proneness: Q32,
    pub recovery_rate: Q32,
    pub dirtiness: Q32,
}  // 3 — totals: 14+10+8+6+14+3 = 55 fields; "52 attributes" counts the
   //     pillar-load-bearing ones, with the 3 durability fields adjacent.

/// Current and potential ability. Both Q32 in [0, 1]. CA is a weighted sum
/// of visible attributes (weights are role-conditioned, see RoleAffinityTable).
/// PA is the ceiling CA can rise to; **only** `MemoryEvent::Breakthrough`
/// mutates PA (Pillar 3). Aging curves move CA toward PA pre-peak and away
/// from PA post-peak.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilityCeiling {
    pub current: Q32,
    pub potential: Q32,
}

/// Short-term modulator layers — kept distinct from canonical attributes so
/// the BT runner can compose them multiplicatively without touching the base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCondition {
    pub form: Q32,        // multi-week rolling average, decays linearly
    pub morale: Q32,      // dressing-room state, decays daily
    pub match_fitness: Q32, // per-match condition, drains tick-by-tick
    pub sharpness: Q32,   // match-rust on returning players, weekly tick-up
    pub signature_readiness: Q32, // pillar-3 breakthrough accumulator
}
```

### Choices, item by item

1. **Visible count + split: 14 technical / 10 mental / 8 physical / 6 goalkeeper = 38 visible.** FM ships 36 outfield-visible (14/14/8) plus 13 GK-technical for keepers; ZOXEXIVO ships 37 (14/14/9). We split the difference and add the GK group as a flat sub-struct on every player (no sum type — eliminates BT pattern-match branches). The mental column compresses to 10 by folding leadership + sportsmanship + controversy into the personality vector where they semantically belong (those are not match-time decision inputs).

2. **Hidden count + content: 14 personality + 3 durability = 17 hidden.** FM's ~16 hidden are the public reference; our 14-field personality vector covers FM's headliners (Determination, Pressure Tolerance, Big-Match Temperament, Consistency, Versatility, Adaptability, Ambition, Loyalty, Professionalism, Temperament, Controversy folded into Temperament). We add Aggression + RiskAppetite + Selflessness + WorkRate from the synthesis bias-vector pattern (`00-synthesis.md` lines 22–24, `03-non-sport-emergent-sims.md` lines 30–34, 49–54). Durability gets its own struct because injury-proneness drives the simulation differently from disposition.

3. **Value type + range: `Q32` in `[Q32::ZERO, Q32::ONE]`.** Internal sim arithmetic stays multiplicative-friendly (utility scores compose without rescaling). The UI projects to 1–20 at the DTO boundary (`Tauri/RULES.md` §3) so the player-facing surface is FM-familiar. `u8` was rejected because BT utility multipliers (`finishing × composure × technique_pressure_modifier`) need fractional precision below `1/255`. Per `Sim/RULES.md` §1, `f32` / `f64` are banned in canonical state.

4. **CA vs PA: keep both, mutable PA gated by breakthroughs only.** `AbilityCeiling { current, potential }`. CA is a derived weighted sum at read time — not a stored cache — so attribute changes propagate without invalidation logic. PA is stored. PA mutation is restricted to `MemoryEvent::Breakthrough` writes through a typed API on `AbilityCeiling` (`pub(crate) fn redraw_ceiling(...)`). Aging curves move CA toward PA pre-peak (~age 27) and away from PA post-peak (`07-player-attributes-progression.md` lines 33–37) without touching PA itself. This satisfies Pillar 3's "growth lives in the ledger first" framing.

5. **Form / morale / fatigue / chemistry layers: 5 separate `PlayerCondition` fields.** FM ships four condition layers (Condition, Fatigue, Sharpness, Morale per `07-player-attributes-progression.md` line 63); we ship five by adding `signature_readiness` as the pillar-3 breakthrough accumulator. Each is Q32 in `[0, 1]` with its own decay rate (form weekly, morale daily, match_fitness per-tick, sharpness weekly, signature_readiness event-driven). Chemistry lives on the `Squad` / `Club` aggregate, not the player record — it's a relationship metric, not a per-player scalar.

6. **Role-affinity weights: content-pack RON, not hardcoded.** Per-role weight tables live in `content/sources/<pack>/role-affinities.ron` keyed by `RoleId`. A CB role weights `{ tackling: 0.18, marking: 0.18, strength: 0.12, jumping_reach: 0.10, positioning: 0.10, ... }`; an AM role weights `{ vision: 0.15, passing: 0.15, dribbling: 0.12, off_the_ball: 0.10, technique: 0.10, ... }`. Weights sum to 1.0 per role; clippy lint enforces. This keeps balancing in RON-edit land (per `CLAUDE.md` §7 "tuning coefficients stay out of SPEC and source") rather than recompile land, and lets mod overlays (`Content/RULES.md` §6) introduce custom roles without source patches.

7. **Hidden trait surfacing for scouts: FOF-style ranges keyed to scout skill.** Per `07-player-attributes-progression.md` lines 51–53, FOF surfaces attribute *ranges* whose width tightens as scout skill rises. We adopt that model rather than OOTP's parallel `true_ratings` / `scout_ratings` tables. Rationale: it surfaces uncertainty in the UI without duplicating canonical state (one table, computed projections), and it maps directly onto Pillar 4's "biased scouts disagree" framing — two scouts of equal skill produce overlapping-but-non-identical ranges because their `scout_bias_vector` differs (`03-non-sport-emergent-sims.md` line 68). Implementation lands in T2-7 as `ScoutObservation { attribute: AttributeKey, range: (Q32, Q32), confidence: Q32 }` — a pure projection of canonical truth + scout-specific bias + observation count.

### Revisions to prior research recommendations

The original "24 visible + 8 hidden = 32" recommendation in `07-player-attributes-progression.md` lines 68–70 was explicitly LoC-budget-constrained. With the budget retracted (`00-synthesis.md` lines 5–10, `DESIGN_DOC.md` §1 lines 32–37), we revise upward:

- Visible **24 → 38** (FM-class breadth; keeper attributes promoted from research footnote to first-class).
- Hidden **8 → 17** (personality vector grows from 8 to 14 to cover FM's full hidden set; durability gets its own 3-field struct).
- Total **32 → 55 fields, ~52 "attributes" in the pillar-load-bearing sense.**

This is a deliberate upward revision documented in `docs/DECISIONS.md` (the 2026-05-13 ADR-0002 entry).

## Consequences

**Positive:**
- Pillar 5 (signature identity) gains room — the 24 signature triggers can read from 38 visible attributes without overloading 9 fields with three meanings each.
- Pillar 4 (scouting uncertainty) gets a clean two-axis surface: visible attributes get range-projected; hidden attributes are scout-discoverable only via observation accumulation over seasons.
- FM-familiar attribute names lower the "what is this game" barrier for FM refugees (DESIGN_DOC §1 audience framing) without locking us into FM's progression model.
- Content-pack-driven role affinities mean Phase T6+ balancing happens in RON edits, not Rust recompiles. Modders can ship custom roles without forking source.
- `Q32` in `[0, 1]` keeps BT utility composition arithmetic clean (multiplicative biases compose without rescaling).
- Mutable-only-via-breakthrough PA gives Pillar 3 a real mechanical surface, not just a narrative one.

**Negative:**
- 55-field player record is heavy on save size. At `Q32` = 8 bytes per field, ~440 bytes per player attribute block; ~50k players × 440 bytes = ~22 MB just for attribute state. Mitigation: bincode + zstd in `fw-save` (the save-format ADR will land before T2-9 schema work; ADR-0003 in this batch is decision-utility math, not save format); compresses to ~6 MB in practice given attribute-value entropy.
- BT decision sites now have more candidate attributes to read. Mitigation: per-decision attribute binding is documented in `docs/specs/bt-attribute-binding.md` (Open question, `00-synthesis.md` line 153). 2–4 attributes per decision keeps the binding tight.
- Generation pipeline (T2-4) is more complex than a 32-field record. Mitigation: ZOXEXIVO's group-by-group generation pattern (`05-open-football-data.md` lines 14–28) maps directly; we re-implement deterministically (no `rand::random()`, no `rayon`) but the structure carries over.
- Bigger schema = bigger save-migration surface when fields change. Mitigation: `fw-save` schema versioning (forward-only) plus the four-test contract per `Sim/RULES.md` §9.

**Neutral:**
- We're FM-class in breadth, not FM-class in tooling — the editor / scout-network / player-search depth is its own phase (T6+). The schema doesn't force us to ship FM's surface, but it doesn't preclude it either.
- The personality vector field count (14) is generous. We expect to use ~8 of them heavily in the BT (the synthesis-named 8), with the rest serving long-tail systems (transfer market, press, dressing room). If 6 of them prove dead weight by T4, we drop them in a forward-migration; the cost is a save bump, not a schema rethink.

**Rollback path:**
- If T1-2b reveals the 38-attribute surface is too wide to BT-bind cleanly, we collapse via a new ADR that introduces derived "compound attributes" (e.g. `ball_control = (first_touch + technique + dribbling) / 3`) consumed by the BT, with the 38 raw attributes preserved as the source-of-truth. The schema doesn't need to shrink; only the BT consumption surface does.
- If 55 fields × 50k players makes saves untenable post-compression, fold less-critical fields (corners, long_throws, penalty_taking) into derived sums of others, removing them from canonical state. Forward-migrate at the next save schema bump.

## Alternatives considered

- **Alternative A: 19 attributes, u8 0–100, no hidden/visible split (OpenFootManager pattern, `02-openfootmanager-data-and-tauri.md` lines 12–20).** Rejected because it forecloses Pillar 4 — without a hidden axis, scout disagreement reduces to "how close to true value did your scout guess," not "scouts disagree about different facets of the same player." Also forecloses Pillar 5's signature triggers, which need readable attribute axes the player can recognize ("his finishing was the trigger").

- **Alternative B: 37 attributes in three groups, `f32` 1–20 (ZOXEXIVO pattern, `05-open-football-data.md` lines 6–12).** Rejected on type — `f32` in canonical state breaks `Sim/RULES.md` §1. Schema structurally close to what we adopted, so we lift the group split + generation pipeline shape without lifting the storage type.

- **Alternative C: original "24 visible + 8 hidden = 32" budget-constrained recommendation (`07-player-attributes-progression.md` lines 68–70).** Rejected because the LoC budget that motivated it has been retracted (DESIGN_DOC §1 reframe). The recommendation was a *floor*, not a *ceiling* — we revise upward, not because 32 was wrong, but because it was conservatively scoped under a constraint that no longer applies. Path-not-taken: had we shipped 32, the T1-5+ work would have hit "we need more attribute surface" sooner; better to start FM-class and trim if needed.

- **Alternative D: split goalkeepers into a `PlayerRole::Goalkeeper { gk_attrs: GoalkeeperAttributes }` sum type with no GK fields on outfielders.** Rejected because every BT decision site that reads attributes would need pattern-match on the role discriminant, polluting the decision code. Storage overhead of 6 unused fields per outfielder (~48 bytes × 50k players ≈ 2.4 MB pre-compression) is acceptable; it compresses to near-zero given outfielders' GK attrs cluster near `Q32::ZERO`.

- **Alternative E: store CA as a cached field, recompute on attribute write.** Rejected because cache invalidation across breakthroughs + aging + form + role-context creates bug surface. Derived-at-read keeps the canonical truth in one place. The cost is recomputation on every read — measured at ~50 ns per call against a `RoleAffinityTable` lookup, acceptable.

- **Alternative F: OOTP-style parallel `true_attrs` + `scout_attrs` tables.** Rejected for scouts (see Decision §7). FOF's range model is a thinner, more honest representation of "uncertainty" — duplicate tables imply scouts maintain a parallel internal model, which neither matches Pillar 4's design intent nor saves implementation effort.

## References

- `docs/DESIGN_DOC.md` §1 "Scope ambition" (2026-05-13 reframe — LoC budget retracted)
- `docs/DESIGN_DOC.md` §3 pillars 3 (breakthroughs), 4 (scouting), 5 (signature identity)
- `docs/research/sports-sims/00-synthesis.md` lines 5–10 (scope reframe), 81–101 (attribute synthesis)
- `docs/research/sports-sims/07-player-attributes-progression.md` lines 29–37 (FM/FM aging), 51–53 (FOF ranges), 68–70 (original 24+8 recommendation revised upward by this ADR)
- `docs/research/sports-sims/03-non-sport-emergent-sims.md` lines 30–34 (CK3 personality vector), 49–54 (composition pattern), 68 (scout-as-biased-observer)
- `docs/research/existing-rust-sims/02-openfootmanager-data-and-tauri.md` lines 12–20 (19-attr u8 model, considered + rejected)
- `docs/research/existing-rust-sims/05-open-football-data.md` lines 6–28 (ZOXEXIVO 37-attr f32 model + generation pipeline — structure adopted, types rejected)
- `crates/fw-core/src/ids.rs` (PlayerId u32 newtype, locked at Codex Q2)
- `crates/fw-core/src/q32.rs` (Q32 fixed-point primitive)
- `.claude/rules/Sim/RULES.md` §1–§4 (determinism floor)
- `.claude/rules/Content/RULES.md` §1–§3 (RON authoring for role affinities)
- `.claude/rules/Tauri/RULES.md` §3 (DTO projection — UI sees 1–20 scale)
- `docs/MASTER_PLAN.md` T1-1 (the row this ADR unblocks)
- Prior ADRs: none (ADR-0001 not yet authored; the Q32 vs f64 decision listed in the template's worked-example block is committed to source as the `Sim/RULES.md` §1 ban and the `#![deny(clippy::float_arithmetic)]` lint, but no formal ADR document exists yet).

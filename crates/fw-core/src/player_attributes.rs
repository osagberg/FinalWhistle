//! Player attributes — the canonical per-player attribute record.
//!
//! Implements ADR-0002 (`docs/adr/0002-player-attribute-model.md`) verbatim.
//! 55 fields total: 14 technical / 10 mental / 8 physical / 6 goalkeeper /
//! 14 personality / 3 durability. All fields `Q32` in `[Q32::ZERO, Q32::ONE]`.
//!
//! The "52 attributes" framing referenced in design docs counts the 38
//! visible + 14 personality (pillar-load-bearing); the 3 durability fields
//! sit adjacent in `DurabilityProfile`. Use 55 when sizing storage, 52
//! when sizing the BT consideration surface (ADR-0002 §"Decision").
//!
//! ## Why these structs live in `fw-core`
//!
//! Every downstream crate reads `PlayerAttributes`: the BT runner consumes
//! it for utility scoring (`fw-match-sim`, T1-2b), scouts surface biased
//! projections (`fw-scouting`, T2-7), and breakthroughs mutate the
//! `AbilityCeiling` (`fw-memory`, T3-4). Placing the types in `fw-core`
//! keeps the dependency graph acyclic — every other crate depends on
//! `fw-core`, never the reverse.
//!
//! `fw-content::PlayerTemplate` wraps these types for content-pack RON
//! authoring; the canonical state structures in `fw-match-sim` will
//! reference them by composition.
//!
//! ## Determinism floor
//!
//! All fields are `Q32`. No `f32` / `f64` in this module. The UI projects
//! `Q32` → 1..=20 integer at the DTO boundary in `fw-tauri`, never here.
//! `#[derive(PartialEq, Eq)]` is sound because `Q32` itself derives `Eq`
//! (no NaN footgun — that's the point of fixed-point).
//!
//! ## Field-count invariants
//!
//! The compile-time constants `VISIBLE_ATTR_COUNT = 38` and
//! `HIDDEN_ATTR_COUNT = 17` are part of the contract — they pin the schema
//! shape so a future refactor that adds/drops a field also has to update
//! the count. Static asserts at the bottom of this module enforce that the
//! actual `size_of::<...>()` matches `<count> * size_of::<Q32>()`.

use crate::q32::Q32;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Count contract
// ---------------------------------------------------------------------------

/// Number of visible attributes (technical + mental + physical + goalkeeper).
/// Surfaced to scouts as range-projected values; surfaced to UI as the
/// FM-familiar 1..=20 integer scale.
pub const VISIBLE_ATTR_COUNT: usize = 14 + 10 + 8 + 6;

/// Number of hidden/support fields (personality vector + durability profile).
/// Personality biases BT utility scores; durability drives the injury sim.
/// Not directly surfaced to the player; scouts discover via observation
/// accumulation (T2-7+).
pub const HIDDEN_ATTR_COUNT: usize = 14 + 3;

const _: () = assert!(VISIBLE_ATTR_COUNT == 38);
const _: () = assert!(HIDDEN_ATTR_COUNT == 17);

/// Stable enumeration of every attribute name in `PlayerAttributes`,
/// ordered by struct + field declaration order (matches ADR-0002
/// §"Concrete shape" verbatim).
///
/// Used by `fw-content::RoleAffinityTable::unknown_attribute_keys` to
/// catch misspellings in weight tables. If a future ADR adds, drops, or
/// renames an attribute, this list `MUST` move in lockstep — the
/// `known_attribute_names_count_matches_total_field_count` test pins the
/// length, and the `role_affinity::tests::sample_role_weights_all_keys_known`
/// test catches drift via the sample fixtures.
pub const KNOWN_ATTRIBUTE_NAMES: &[&str] = &[
    // Technical (14)
    "finishing",
    "long_shots",
    "passing",
    "crossing",
    "first_touch",
    "technique",
    "dribbling",
    "heading",
    "tackling",
    "marking",
    "free_kicks",
    "penalty_taking",
    "corners",
    "long_throws",
    // Mental (10)
    "anticipation",
    "composure",
    "decisions",
    "vision",
    "off_the_ball",
    "positioning",
    "concentration",
    "bravery",
    "teamwork",
    "flair",
    // Physical (8)
    "pace",
    "acceleration",
    "stamina",
    "strength",
    "agility",
    "balance",
    "jumping_reach",
    "natural_fitness",
    // Goalkeeper (6)
    "handling",
    "reflexes",
    "one_on_ones",
    "aerial_reach",
    "command_of_area",
    "kicking",
    // Personality (14)
    "determination",
    "work_rate",
    "ambition",
    "professionalism",
    "loyalty",
    "temperament",
    "pressure_tolerance",
    "big_match_appetite",
    "adaptability",
    "aggression",
    "risk_appetite",
    "selflessness",
    "consistency",
    "versatility",
    // Durability (3)
    "injury_proneness",
    "recovery_rate",
    "dirtiness",
];

const _: () = assert!(
    KNOWN_ATTRIBUTE_NAMES.len() == VISIBLE_ATTR_COUNT + HIDDEN_ATTR_COUNT,
    "KNOWN_ATTRIBUTE_NAMES length must match field count (38 + 17 = 55)"
);

// ---------------------------------------------------------------------------
// Visible attribute groups
// ---------------------------------------------------------------------------

/// Technical attributes — ball-skill axes.
///
/// 14 fields, all `Q32` in `[Q32::ZERO, Q32::ONE]`. Order matches
/// ADR-0002 §"Concrete shape" verbatim; the serde representation depends
/// on the field declaration order, so do not reorder.
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
}

/// Mental attributes — decision + awareness axes.
///
/// 10 fields. Note: leadership / sportsmanship / controversy fold into
/// `PersonalityVector` (those aren't match-time decision inputs) — see
/// ADR-0002 §"Choices" item 1.
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
}

/// Physical attributes — body-state axes.
///
/// 8 fields. Stamina is the per-match physical baseline; per-match drain
/// is tracked separately in `PlayerCondition::match_fitness`.
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
}

/// Goalkeeper-specific attributes.
///
/// 6 fields. Present on every player record — outfielders carry low
/// values, keepers carry high ones. Flat struct (not a `PlayerRole` sum
/// type) eliminates pattern-match branches at every BT decision site that
/// reads attributes. Storage overhead per outfielder is ~48 bytes, which
/// compresses to near-zero in the save format (clustered near `Q32::ZERO`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalkeeperAttributes {
    pub handling: Q32,
    pub reflexes: Q32,
    pub one_on_ones: Q32,
    pub aerial_reach: Q32,
    pub command_of_area: Q32,
    pub kicking: Q32,
}

// ---------------------------------------------------------------------------
// Hidden / support groups
// ---------------------------------------------------------------------------

/// Personality bias vector — the 14 hidden axes that drive scout
/// disagreement (Pillar 4), bias BT utility scores (per `00-synthesis.md`
/// "personality = small scalar vector"), and gate breakthrough triggers
/// (Pillar 3).
///
/// FM names: Determination, Pressure Tolerance, Big-Match Temperament,
/// Consistency, Versatility, Adaptability, Ambition, Loyalty,
/// Professionalism, Temperament (Controversy folded in). Additions from
/// the synthesis bias-vector pattern: Aggression, Risk Appetite,
/// Selflessness, Work Rate.
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
}

/// Durability profile — career-shape fields distinct from
/// `PersonalityVector`. Describes the player's relationship to time and
/// their body, not their disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurabilityProfile {
    pub injury_proneness: Q32,
    pub recovery_rate: Q32,
    pub dirtiness: Q32,
}

// ---------------------------------------------------------------------------
// Composite record
// ---------------------------------------------------------------------------

/// Per-player canonical attribute record. The full 55-field surface.
///
/// Field order is load-bearing: the bincode + serde representation depends
/// on declaration order, and the canonical-hash regression test will catch
/// any reorder as a corpus drift. Do not reorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerAttributes {
    pub technical: TechnicalAttributes,
    pub mental: MentalAttributes,
    pub physical: PhysicalAttributes,
    pub goalkeeper: GoalkeeperAttributes,
    pub personality: PersonalityVector,
    pub durability: DurabilityProfile,
}

/// Current Ability + Potential Ability ceiling.
///
/// Both `Q32` in `[Q32::ZERO, Q32::ONE]`. CA is a weighted sum of visible
/// attributes (role-conditioned via `fw-content::RoleAffinityTable`); the
/// stored value is the BT-runner's projection from the last CA-derivation
/// pass. PA is the long-term ceiling that CA can rise toward; only
/// `MemoryEvent::Breakthrough` (via the breakthrough-gated mutator in
/// `fw-memory`, which routes through `fw-core`) writes PA (Pillar 3).
///
/// **Encapsulation contract (ADR-0002 §"Choices" item 4):** fields are
/// `pub(crate)` so only `fw-core` can mutate them directly. External
/// crates (`fw-match-sim`, `fw-memory`, `fw-tauri`) read via
/// `current()` / `potential()` accessors and mutate PA only via the
/// `redraw_ceiling` breakthrough API. Direct field assignment from
/// outside `fw-core` would silently bypass Pillar 3 ("growth lives in
/// the ledger").
///
/// `fw-content::PlayerTemplate` constructs `AbilityCeiling` via RON
/// deserialization; the derive-generated `Deserialize` impl lives in
/// `fw-core`, so the `pub(crate)` visibility is fine for serde.
///
/// Aging curves move CA toward PA pre-peak (~age 27) and away from PA
/// post-peak without touching PA itself
/// (`docs/research/sports-sims/07-player-attributes-progression.md`
/// lines 33–37).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilityCeiling {
    pub(crate) current: Q32,
    pub(crate) potential: Q32,
}

impl AbilityCeiling {
    /// Construct an `AbilityCeiling` with the given current + potential
    /// values. Both `MUST` be `Q32` in `[Q32::ZERO, Q32::ONE]`; the FW-VAL
    /// gauntlet (T2-3+) catches out-of-range fixtures at load time.
    #[must_use]
    pub const fn new(current: Q32, potential: Q32) -> Self {
        Self { current, potential }
    }

    /// Current Ability — the player's present derived ceiling. Read-only
    /// outside `fw-core`; downstream consumers either trust the stored
    /// value or recompute via `fw-content::RoleAffinityTable`.
    #[must_use]
    pub const fn current(self) -> Q32 {
        self.current
    }

    /// Potential Ability — the long-term ceiling CA can rise to. Mutable
    /// only via `redraw_ceiling` (`pub(crate)`); external callers must
    /// route breakthrough writes through a `fw-core`-resident helper.
    #[must_use]
    pub const fn potential(self) -> Q32 {
        self.potential
    }

    /// Breakthrough-only PA mutator. Restricted to `pub(crate)` per
    /// ADR-0002 §"Choices" item 4 — only `fw-core` code (and in the
    /// future, `fw-memory` via a delegating helper resident in `fw-core`)
    /// may move PA upward.
    #[allow(dead_code)] // consumed by fw-memory breakthrough writer at T3-4
    pub(crate) fn redraw_ceiling(&mut self, new_potential: Q32) {
        self.potential = new_potential;
    }
}

/// Short-term modulator layers — kept distinct from `PlayerAttributes` so
/// the BT runner can compose them multiplicatively without touching the
/// base canonical attributes.
///
/// Five fields, each with its own decay cadence:
/// - `form` — multi-week rolling average, decays linearly
/// - `morale` — dressing-room state, decays daily
/// - `match_fitness` — per-match condition, drains tick-by-tick
/// - `sharpness` — match-rust on returning players, weekly tick-up
/// - `signature_readiness` — pillar-3 breakthrough accumulator, event-driven
///
/// Chemistry is NOT here — it's a relationship metric on the squad
/// aggregate, not a per-player scalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCondition {
    pub form: Q32,
    pub morale: Q32,
    pub match_fitness: Q32,
    pub sharpness: Q32,
    pub signature_readiness: Q32,
}

// ---------------------------------------------------------------------------
// Size contract — static asserts pin the field counts at compile time.
// ---------------------------------------------------------------------------

const _Q32_SIZE: usize = std::mem::size_of::<Q32>();
const _: () = assert!(std::mem::size_of::<TechnicalAttributes>() == 14 * _Q32_SIZE);
const _: () = assert!(std::mem::size_of::<MentalAttributes>() == 10 * _Q32_SIZE);
const _: () = assert!(std::mem::size_of::<PhysicalAttributes>() == 8 * _Q32_SIZE);
const _: () = assert!(std::mem::size_of::<GoalkeeperAttributes>() == 6 * _Q32_SIZE);
const _: () = assert!(std::mem::size_of::<PersonalityVector>() == 14 * _Q32_SIZE);
const _: () = assert!(std::mem::size_of::<DurabilityProfile>() == 3 * _Q32_SIZE);
const _: () = assert!(
    std::mem::size_of::<PlayerAttributes>() == (VISIBLE_ATTR_COUNT + HIDDEN_ATTR_COUNT) * _Q32_SIZE
);

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_attrs() -> PlayerAttributes {
        let half = Q32::from_raw(1i64 << 31); // ~0.5
        PlayerAttributes {
            technical: TechnicalAttributes {
                finishing: half,
                long_shots: half,
                passing: half,
                crossing: half,
                first_touch: half,
                technique: half,
                dribbling: half,
                heading: half,
                tackling: half,
                marking: half,
                free_kicks: half,
                penalty_taking: half,
                corners: half,
                long_throws: half,
            },
            mental: MentalAttributes {
                anticipation: half,
                composure: half,
                decisions: half,
                vision: half,
                off_the_ball: half,
                positioning: half,
                concentration: half,
                bravery: half,
                teamwork: half,
                flair: half,
            },
            physical: PhysicalAttributes {
                pace: half,
                acceleration: half,
                stamina: half,
                strength: half,
                agility: half,
                balance: half,
                jumping_reach: half,
                natural_fitness: half,
            },
            goalkeeper: GoalkeeperAttributes {
                handling: Q32::ZERO,
                reflexes: Q32::ZERO,
                one_on_ones: Q32::ZERO,
                aerial_reach: Q32::ZERO,
                command_of_area: Q32::ZERO,
                kicking: Q32::ZERO,
            },
            personality: PersonalityVector {
                determination: half,
                work_rate: half,
                ambition: half,
                professionalism: half,
                loyalty: half,
                temperament: half,
                pressure_tolerance: half,
                big_match_appetite: half,
                adaptability: half,
                aggression: half,
                risk_appetite: half,
                selflessness: half,
                consistency: half,
                versatility: half,
            },
            durability: DurabilityProfile {
                injury_proneness: half,
                recovery_rate: half,
                dirtiness: Q32::ZERO,
            },
        }
    }

    #[test]
    fn visible_count_pins_at_38() {
        assert_eq!(VISIBLE_ATTR_COUNT, 38);
    }

    #[test]
    fn known_attribute_names_count_matches_total_field_count() {
        // 38 visible + 17 hidden = 55 fields; KNOWN_ATTRIBUTE_NAMES must
        // enumerate all of them in struct + field declaration order.
        assert_eq!(
            KNOWN_ATTRIBUTE_NAMES.len(),
            VISIBLE_ATTR_COUNT + HIDDEN_ATTR_COUNT
        );
    }

    #[test]
    fn known_attribute_names_are_unique() {
        // No duplicate keys — a duplicate would silently shadow a weight
        // in BTreeMap-keyed RoleWeights.
        let mut sorted: Vec<&&str> = KNOWN_ATTRIBUTE_NAMES.iter().collect();
        sorted.sort();
        let unique = sorted.iter().fold(Vec::new(), |mut acc: Vec<&&str>, &x| {
            if acc.last() != Some(&x) {
                acc.push(x);
            }
            acc
        });
        assert_eq!(unique.len(), KNOWN_ATTRIBUTE_NAMES.len());
    }

    #[test]
    fn ability_ceiling_constructor_and_accessors() {
        let c = AbilityCeiling::new(Q32::from_raw(1i64 << 30), Q32::from_raw(3i64 << 30));
        assert_eq!(c.current(), Q32::from_raw(1i64 << 30));
        assert_eq!(c.potential(), Q32::from_raw(3i64 << 30));
    }

    #[test]
    fn ability_ceiling_breakthrough_writer() {
        let mut c = AbilityCeiling::new(Q32::ZERO, Q32::from_raw(1i64 << 30)); // PA ~0.25
        c.redraw_ceiling(Q32::from_raw(3i64 << 30)); // PA ~0.75
        assert_eq!(c.potential(), Q32::from_raw(3i64 << 30));
        // CA untouched.
        assert_eq!(c.current(), Q32::ZERO);
    }

    #[test]
    fn hidden_count_pins_at_17() {
        assert_eq!(HIDDEN_ATTR_COUNT, 17);
    }

    #[test]
    fn visible_count_matches_struct_field_counts() {
        // 14 + 10 + 8 + 6 = 38.
        let technical = std::mem::size_of::<TechnicalAttributes>() / std::mem::size_of::<Q32>();
        let mental = std::mem::size_of::<MentalAttributes>() / std::mem::size_of::<Q32>();
        let physical = std::mem::size_of::<PhysicalAttributes>() / std::mem::size_of::<Q32>();
        let goalkeeper = std::mem::size_of::<GoalkeeperAttributes>() / std::mem::size_of::<Q32>();
        assert_eq!(
            technical + mental + physical + goalkeeper,
            VISIBLE_ATTR_COUNT
        );
        assert_eq!(technical, 14);
        assert_eq!(mental, 10);
        assert_eq!(physical, 8);
        assert_eq!(goalkeeper, 6);
    }

    #[test]
    fn hidden_count_matches_struct_field_counts() {
        // 14 + 3 = 17.
        let personality = std::mem::size_of::<PersonalityVector>() / std::mem::size_of::<Q32>();
        let durability = std::mem::size_of::<DurabilityProfile>() / std::mem::size_of::<Q32>();
        assert_eq!(personality + durability, HIDDEN_ATTR_COUNT);
        assert_eq!(personality, 14);
        assert_eq!(durability, 3);
    }

    #[test]
    fn player_attributes_serde_round_trip() {
        // bincode 1.x in fw-core dev-deps (matches q32.rs convention).
        let attrs = sample_attrs();
        let bytes = bincode::serialize(&attrs).expect("bincode serialize");
        let decoded: PlayerAttributes = bincode::deserialize(&bytes).expect("bincode deserialize");
        assert_eq!(decoded, attrs);
    }

    #[test]
    fn ability_ceiling_round_trip() {
        let ceiling = AbilityCeiling {
            current: Q32::from_raw(1i64 << 30),   // ~0.25
            potential: Q32::from_raw(3i64 << 30), // ~0.75
        };
        let bytes = bincode::serialize(&ceiling).expect("bincode serialize");
        let decoded: AbilityCeiling = bincode::deserialize(&bytes).expect("bincode deserialize");
        assert_eq!(decoded, ceiling);
    }

    #[test]
    fn player_condition_round_trip() {
        let cond = PlayerCondition {
            form: Q32::ONE,
            morale: Q32::ONE,
            match_fitness: Q32::ONE,
            sharpness: Q32::ONE,
            signature_readiness: Q32::ZERO,
        };
        let bytes = bincode::serialize(&cond).expect("bincode serialize");
        let decoded: PlayerCondition = bincode::deserialize(&bytes).expect("bincode deserialize");
        assert_eq!(decoded, cond);
    }

    #[test]
    fn ron_round_trip() {
        // Ensures content-pack RON authoring will work (used in T1-1
        // fixtures).
        let attrs = sample_attrs();
        let s = ron::ser::to_string(&attrs).expect("ron encode");
        let decoded: PlayerAttributes = ron::de::from_str(&s).expect("ron decode");
        assert_eq!(decoded, attrs);
    }
}

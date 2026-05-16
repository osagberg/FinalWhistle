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

/// Predicate: is `q` in the closed `[Q32::ZERO, Q32::ONE]` range?
///
/// Every visible/hidden attribute field, `AbilityCeiling` field, and
/// `PlayerCondition` field is contract-bound to this range. Used by
/// `AbilityCeiling::try_new` + `PlayerAttributes::validate_unit_range`
/// + downstream validators.
#[must_use]
pub fn is_in_unit_range(q: Q32) -> bool {
    q >= Q32::ZERO && q <= Q32::ONE
}

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

/// Stable enumeration of every **visible** attribute name in `PlayerAttributes`
/// (technical + mental + physical + goalkeeper = 38 fields), ordered by
/// struct + field declaration order (ADR-0002 §"Concrete shape" verbatim).
///
/// **This is the canonical key set for CA derivation weights.** Per ADR-0002
/// §"Choices" item 6, role-affinity tables weight ONLY visible attributes
/// toward Current Ability. Hidden fields (personality / durability) drive
/// scout disagreement (Pillar 4) + BT utility biasing (Pillar 5) + injury
/// modeling — they do NOT contribute to the CA-derivation weighted sum.
///
/// `fw-content::RoleAffinityTable::unknown_attribute_keys` validates against
/// this list (per Codex audit P1 fix, 2026-05-13).
pub const VISIBLE_ATTRIBUTE_NAMES: &[&str] = &[
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
];

/// Stable enumeration of every **hidden/support** field name (personality
/// vector + durability profile = 17 fields), ordered by struct + field
/// declaration order.
///
/// Hidden fields drive scout disagreement, BT utility biasing, and injury
/// modeling. They are NOT valid keys in role-affinity weight tables.
/// `fw-content::RoleAffinityTable::unknown_attribute_keys` rejects them
/// as keys per Codex audit P1 fix (2026-05-13).
pub const HIDDEN_ATTRIBUTE_NAMES: &[&str] = &[
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

/// Stable enumeration of every attribute name (visible + hidden = 55).
///
/// Use `VISIBLE_ATTRIBUTE_NAMES` for CA-weight key validation (the right
/// answer per ADR-0002); use `KNOWN_ATTRIBUTE_NAMES` only when you
/// genuinely need the full 55-name set (e.g. attribute-binding documents
/// that catalogue every possible BT input).
pub const KNOWN_ATTRIBUTE_NAMES: &[&str] = &[
    // Visible (38)
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
    "pace",
    "acceleration",
    "stamina",
    "strength",
    "agility",
    "balance",
    "jumping_reach",
    "natural_fitness",
    "handling",
    "reflexes",
    "one_on_ones",
    "aerial_reach",
    "command_of_area",
    "kicking",
    // Hidden (17)
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
    "injury_proneness",
    "recovery_rate",
    "dirtiness",
];

const _: () = assert!(
    VISIBLE_ATTRIBUTE_NAMES.len() == VISIBLE_ATTR_COUNT,
    "VISIBLE_ATTRIBUTE_NAMES length must match visible field count (38)"
);
const _: () = assert!(
    HIDDEN_ATTRIBUTE_NAMES.len() == HIDDEN_ATTR_COUNT,
    "HIDDEN_ATTRIBUTE_NAMES length must match hidden field count (17)"
);
const _: () = assert!(
    KNOWN_ATTRIBUTE_NAMES.len() == VISIBLE_ATTR_COUNT + HIDDEN_ATTR_COUNT,
    "KNOWN_ATTRIBUTE_NAMES length must match total field count (38 + 17 = 55)"
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

/// Error returned by `PlayerAttributes::validate_unit_range`.
///
/// Codex audit P1 (2026-05-13): there was no single validation entry-point
/// for the "every attribute is in [0, 1]" invariant — the doc claim was
/// carried by convention only. `validate_unit_range` is the single point
/// of enforcement; called by FW-VAL at load time + by any runtime helper
/// that derives a new `PlayerAttributes` from arithmetic.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("PlayerAttributes field `{field}` outside [0, 1]: bits = {bits}")]
pub struct AttributeRangeError {
    /// Dotted field path (`technical.finishing`, `personality.work_rate`).
    pub field: &'static str,
    /// Raw `Q32` bits of the offending value (for debug; convert via
    /// `Q32::from_raw(bits).to_f64()` for a human-readable form).
    pub bits: i64,
}

impl PlayerAttributes {
    /// Construct a mid-range baseline with all 55 fields set to exactly 0.5
    /// (`Q32::from_raw(1i64 << 31)`).
    ///
    /// Used as the canonical default for newly-generated players before an
    /// ability-derivation pass, and as the stable test fixture for BT utility
    /// scorer unit tests (`T1-2b-iii-b`).
    #[must_use]
    pub fn mid_range_baseline() -> Self {
        let half = Q32::from_raw(1i64 << 31);
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
                handling: half,
                reflexes: half,
                one_on_ones: half,
                aerial_reach: half,
                command_of_area: half,
                kicking: half,
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
                dirtiness: half,
            },
        }
    }

    /// All-zero `PlayerAttributes` (every field = `Q32::ZERO`).
    ///
    /// Test-only utility for checking base-speed formulas: at zero attrs the
    /// attribute-scaling bonus is zero, so `compute_ball_speed_for_shot` must
    /// return exactly `SHOT_BASE_SPEED_MPS`. Not a realistic player state.
    #[must_use]
    pub fn default_zero() -> Self {
        let z = Q32::ZERO;
        PlayerAttributes {
            technical: TechnicalAttributes {
                finishing: z,
                long_shots: z,
                passing: z,
                crossing: z,
                first_touch: z,
                technique: z,
                dribbling: z,
                heading: z,
                tackling: z,
                marking: z,
                free_kicks: z,
                penalty_taking: z,
                corners: z,
                long_throws: z,
            },
            mental: MentalAttributes {
                anticipation: z,
                composure: z,
                decisions: z,
                vision: z,
                off_the_ball: z,
                positioning: z,
                concentration: z,
                bravery: z,
                teamwork: z,
                flair: z,
            },
            physical: PhysicalAttributes {
                pace: z,
                acceleration: z,
                stamina: z,
                strength: z,
                agility: z,
                balance: z,
                jumping_reach: z,
                natural_fitness: z,
            },
            goalkeeper: GoalkeeperAttributes {
                handling: z,
                reflexes: z,
                one_on_ones: z,
                aerial_reach: z,
                command_of_area: z,
                kicking: z,
            },
            personality: PersonalityVector {
                determination: z,
                work_rate: z,
                ambition: z,
                professionalism: z,
                loyalty: z,
                temperament: z,
                pressure_tolerance: z,
                big_match_appetite: z,
                adaptability: z,
                aggression: z,
                risk_appetite: z,
                selflessness: z,
                consistency: z,
                versatility: z,
            },
            durability: DurabilityProfile {
                injury_proneness: z,
                recovery_rate: z,
                dirtiness: z,
            },
        }
    }

    /// All-max `PlayerAttributes` (every field = `Q32::ONE`).
    ///
    /// Test-only utility for checking peak-speed formulas: at max attrs the
    /// attribute-scaling bonus is fully applied, so `compute_ball_speed_for_shot`
    /// must return `SHOT_BASE_SPEED_MPS + SHOT_PEAK_BONUS_MPS`.
    #[must_use]
    pub fn max_baseline() -> Self {
        let one = Q32::ONE;
        PlayerAttributes {
            technical: TechnicalAttributes {
                finishing: one,
                long_shots: one,
                passing: one,
                crossing: one,
                first_touch: one,
                technique: one,
                dribbling: one,
                heading: one,
                tackling: one,
                marking: one,
                free_kicks: one,
                penalty_taking: one,
                corners: one,
                long_throws: one,
            },
            mental: MentalAttributes {
                anticipation: one,
                composure: one,
                decisions: one,
                vision: one,
                off_the_ball: one,
                positioning: one,
                concentration: one,
                bravery: one,
                teamwork: one,
                flair: one,
            },
            physical: PhysicalAttributes {
                pace: one,
                acceleration: one,
                stamina: one,
                strength: one,
                agility: one,
                balance: one,
                jumping_reach: one,
                natural_fitness: one,
            },
            goalkeeper: GoalkeeperAttributes {
                handling: one,
                reflexes: one,
                one_on_ones: one,
                aerial_reach: one,
                command_of_area: one,
                kicking: one,
            },
            personality: PersonalityVector {
                determination: one,
                work_rate: one,
                ambition: one,
                professionalism: one,
                loyalty: one,
                temperament: one,
                pressure_tolerance: one,
                big_match_appetite: one,
                adaptability: one,
                aggression: one,
                risk_appetite: one,
                selflessness: one,
                consistency: one,
                versatility: one,
            },
            durability: DurabilityProfile {
                injury_proneness: one,
                recovery_rate: one,
                dirtiness: one,
            },
        }
    }

    /// Verify every field is in `[Q32::ZERO, Q32::ONE]`. Returns
    /// `Vec<AttributeRangeError>` listing every violation (collect-all,
    /// not first-only — matches the `RoleAffinityTable::invalid_roles`
    /// pattern; FW-VAL surfaces all errors in one pass).
    #[must_use]
    pub fn validate_unit_range(&self) -> Vec<AttributeRangeError> {
        let mut errors = Vec::new();
        macro_rules! check {
            ($group:ident . $field:ident) => {
                if !is_in_unit_range(self.$group.$field) {
                    errors.push(AttributeRangeError {
                        field: concat!(stringify!($group), ".", stringify!($field)),
                        bits: self.$group.$field.to_bits(),
                    });
                }
            };
        }
        // Technical (14)
        check!(technical.finishing);
        check!(technical.long_shots);
        check!(technical.passing);
        check!(technical.crossing);
        check!(technical.first_touch);
        check!(technical.technique);
        check!(technical.dribbling);
        check!(technical.heading);
        check!(technical.tackling);
        check!(technical.marking);
        check!(technical.free_kicks);
        check!(technical.penalty_taking);
        check!(technical.corners);
        check!(technical.long_throws);
        // Mental (10)
        check!(mental.anticipation);
        check!(mental.composure);
        check!(mental.decisions);
        check!(mental.vision);
        check!(mental.off_the_ball);
        check!(mental.positioning);
        check!(mental.concentration);
        check!(mental.bravery);
        check!(mental.teamwork);
        check!(mental.flair);
        // Physical (8)
        check!(physical.pace);
        check!(physical.acceleration);
        check!(physical.stamina);
        check!(physical.strength);
        check!(physical.agility);
        check!(physical.balance);
        check!(physical.jumping_reach);
        check!(physical.natural_fitness);
        // Goalkeeper (6)
        check!(goalkeeper.handling);
        check!(goalkeeper.reflexes);
        check!(goalkeeper.one_on_ones);
        check!(goalkeeper.aerial_reach);
        check!(goalkeeper.command_of_area);
        check!(goalkeeper.kicking);
        // Personality (14)
        check!(personality.determination);
        check!(personality.work_rate);
        check!(personality.ambition);
        check!(personality.professionalism);
        check!(personality.loyalty);
        check!(personality.temperament);
        check!(personality.pressure_tolerance);
        check!(personality.big_match_appetite);
        check!(personality.adaptability);
        check!(personality.aggression);
        check!(personality.risk_appetite);
        check!(personality.selflessness);
        check!(personality.consistency);
        check!(personality.versatility);
        // Durability (3)
        check!(durability.injury_proneness);
        check!(durability.recovery_rate);
        check!(durability.dirtiness);
        errors
    }
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

/// Errors returned when constructing or validating an `AbilityCeiling`.
///
/// Codex audit P1 (2026-05-13): the original `AbilityCeiling::new`
/// constructor was unchecked, so doc claims of "Q32 in [0, 1]" + "CA ≤ PA"
/// were carried by convention only. This enum + the `try_new` constructor
/// make the invariants enforceable. Pre-T1-2b re-audit P1 follow-up:
/// the unchecked path was renamed to `pub(crate) new_unchecked`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AbilityCeilingError {
    /// `current` is outside `[Q32::ZERO, Q32::ONE]`.
    #[error("AbilityCeiling.current must be in [0, 1]; got bits = {bits}")]
    CurrentOutOfRange { bits: i64 },
    /// `potential` is outside `[Q32::ZERO, Q32::ONE]`.
    #[error("AbilityCeiling.potential must be in [0, 1]; got bits = {bits}")]
    PotentialOutOfRange { bits: i64 },
    /// `current` exceeds `potential`. Semantically: a player cannot have a
    /// CA higher than their PA ceiling. Aging curves move CA *toward* PA,
    /// never past it.
    #[error("AbilityCeiling.current ({current_bits}) > potential ({potential_bits})")]
    CurrentExceedsPotential {
        current_bits: i64,
        potential_bits: i64,
    },
}

impl AbilityCeiling {
    /// Unchecked construction. Codex pre-T1-2b re-audit P1 (2026-05-13):
    /// the prior `pub const fn new` was a bypass route around `try_new`'s
    /// range + monotonicity invariants. Renamed + visibility tightened to
    /// `pub(crate) const fn new_unchecked`. Now usable only from within
    /// `fw-core` (in-crate tests construct directly; serde's derived
    /// `Deserialize` reaches the `pub(crate)` fields directly without
    /// needing a constructor). All external callers route through
    /// `try_new`. `#[allow(dead_code)]` because non-test builds have no
    /// caller — `try_new` is the only path used in production code.
    #[allow(dead_code)]
    #[must_use]
    pub(crate) const fn new_unchecked(current: Q32, potential: Q32) -> Self {
        Self { current, potential }
    }

    /// Range + monotonicity check on an already-constructed `AbilityCeiling`.
    /// Returns `Err(AbilityCeilingError::*)` on violation. Used by
    /// `fw-content-baker validate` to gate content-pack-loaded ceilings
    /// (Codex pre-T1-2b re-audit P1: FW-VAL validated `template.attributes`
    /// but not `template.ceiling`). Equivalent to running the
    /// `try_new(self.current, self.potential)` check.
    pub fn validate(&self) -> Result<(), AbilityCeilingError> {
        Self::try_new(self.current, self.potential).map(|_| ())
    }

    /// Construct an `AbilityCeiling`, validating the three invariants:
    /// `current ∈ [0, 1]`, `potential ∈ [0, 1]`, `current ≤ potential`.
    ///
    /// Returns `Err(AbilityCeilingError::*)` on violation. The FW-VAL
    /// gauntlet (T2-3+) will route all content-pack-loaded ceilings
    /// through this constructor; runtime code that constructs ceilings
    /// from derived values should also prefer this path.
    pub fn try_new(current: Q32, potential: Q32) -> Result<Self, AbilityCeilingError> {
        if !is_in_unit_range(current) {
            return Err(AbilityCeilingError::CurrentOutOfRange {
                bits: current.to_bits(),
            });
        }
        if !is_in_unit_range(potential) {
            return Err(AbilityCeilingError::PotentialOutOfRange {
                bits: potential.to_bits(),
            });
        }
        if current > potential {
            return Err(AbilityCeilingError::CurrentExceedsPotential {
                current_bits: current.to_bits(),
                potential_bits: potential.to_bits(),
            });
        }
        Ok(Self { current, potential })
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
        let c = AbilityCeiling::new_unchecked(Q32::from_raw(1i64 << 30), Q32::from_raw(3i64 << 30));
        assert_eq!(c.current(), Q32::from_raw(1i64 << 30));
        assert_eq!(c.potential(), Q32::from_raw(3i64 << 30));
    }

    #[test]
    fn ability_ceiling_breakthrough_writer() {
        let mut c = AbilityCeiling::new_unchecked(Q32::ZERO, Q32::from_raw(1i64 << 30)); // PA ~0.25
        c.redraw_ceiling(Q32::from_raw(3i64 << 30)); // PA ~0.75
        assert_eq!(c.potential(), Q32::from_raw(3i64 << 30));
        // CA untouched.
        assert_eq!(c.current(), Q32::ZERO);
    }

    #[test]
    fn ability_ceiling_try_new_accepts_valid() {
        let half = Q32::from_raw(1i64 << 31);
        let three_quarters = Q32::from_raw(3i64 << 30);
        assert!(AbilityCeiling::try_new(half, three_quarters).is_ok());
        assert!(AbilityCeiling::try_new(Q32::ZERO, Q32::ZERO).is_ok());
        assert!(AbilityCeiling::try_new(Q32::ONE, Q32::ONE).is_ok());
    }

    #[test]
    fn ability_ceiling_try_new_rejects_current_out_of_range() {
        let above_one = Q32::from_int(2);
        let result = AbilityCeiling::try_new(above_one, Q32::ONE);
        assert!(matches!(
            result,
            Err(AbilityCeilingError::CurrentOutOfRange { .. })
        ));
    }

    #[test]
    fn ability_ceiling_try_new_rejects_potential_out_of_range() {
        let above_one = Q32::from_int(2);
        let result = AbilityCeiling::try_new(Q32::ZERO, above_one);
        assert!(matches!(
            result,
            Err(AbilityCeilingError::PotentialOutOfRange { .. })
        ));
    }

    #[test]
    fn ability_ceiling_try_new_rejects_negative() {
        let neg = Q32::from_raw(-1);
        assert!(matches!(
            AbilityCeiling::try_new(neg, Q32::ONE),
            Err(AbilityCeilingError::CurrentOutOfRange { .. })
        ));
        assert!(matches!(
            AbilityCeiling::try_new(Q32::ZERO, neg),
            Err(AbilityCeilingError::PotentialOutOfRange { .. })
        ));
    }

    #[test]
    fn ability_ceiling_try_new_rejects_current_exceeds_potential() {
        let high = Q32::from_raw(3i64 << 30);
        let low = Q32::from_raw(1i64 << 30);
        let result = AbilityCeiling::try_new(high, low);
        assert!(matches!(
            result,
            Err(AbilityCeilingError::CurrentExceedsPotential { .. })
        ));
    }

    #[test]
    fn ability_ceiling_validate_method_catches_malformed_post_construction() {
        // A ceiling constructed via the in-crate unchecked path with bad
        // values returns Err on validate(). This is the path FW-VAL uses
        // to catch malformed content-pack ceilings post-load.
        let bad = AbilityCeiling::new_unchecked(Q32::from_int(2), Q32::ONE);
        assert!(matches!(
            bad.validate(),
            Err(AbilityCeilingError::CurrentOutOfRange { .. })
        ));
        let inverted =
            AbilityCeiling::new_unchecked(Q32::from_raw(3i64 << 30), Q32::from_raw(1i64 << 30));
        assert!(matches!(
            inverted.validate(),
            Err(AbilityCeilingError::CurrentExceedsPotential { .. })
        ));
        let good = AbilityCeiling::new_unchecked(Q32::ZERO, Q32::ONE);
        assert!(good.validate().is_ok());
    }

    #[test]
    fn validate_unit_range_accepts_well_formed_attrs() {
        let attrs = sample_attrs();
        assert!(attrs.validate_unit_range().is_empty());
    }

    #[test]
    fn validate_unit_range_catches_out_of_range_fields() {
        let mut attrs = sample_attrs();
        attrs.technical.finishing = Q32::from_int(2); // > 1.0
        attrs.physical.pace = Q32::from_raw(-1); // < 0
        attrs.personality.consistency = Q32::from_int(5); // > 1.0
        let errors = attrs.validate_unit_range();
        assert_eq!(errors.len(), 3);
        let fields: Vec<&str> = errors.iter().map(|e| e.field).collect();
        assert!(fields.contains(&"technical.finishing"));
        assert!(fields.contains(&"physical.pace"));
        assert!(fields.contains(&"personality.consistency"));
    }

    #[test]
    fn visible_and_hidden_attribute_name_lists_are_disjoint() {
        for &v in VISIBLE_ATTRIBUTE_NAMES {
            assert!(
                !HIDDEN_ATTRIBUTE_NAMES.contains(&v),
                "name {v} is in BOTH visible + hidden sets"
            );
        }
    }

    #[test]
    fn visible_plus_hidden_equals_known() {
        // Sanity: VISIBLE + HIDDEN should equal KNOWN as a set.
        let mut combined: Vec<&str> = VISIBLE_ATTRIBUTE_NAMES
            .iter()
            .chain(HIDDEN_ATTRIBUTE_NAMES.iter())
            .copied()
            .collect();
        combined.sort();
        let mut known: Vec<&str> = KNOWN_ATTRIBUTE_NAMES.to_vec();
        known.sort();
        assert_eq!(combined, known);
    }

    #[test]
    fn is_in_unit_range_helper() {
        assert!(is_in_unit_range(Q32::ZERO));
        assert!(is_in_unit_range(Q32::ONE));
        assert!(is_in_unit_range(Q32::from_raw(1i64 << 31)));
        assert!(!is_in_unit_range(Q32::from_raw(-1)));
        assert!(!is_in_unit_range(Q32::from_int(2)));
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

    // P2-3 (T1-2b-iii-b self-review): assert that mid_range_baseline() produces
    // a valid PlayerAttributes. This cheap test keeps mid_range_baseline() and
    // validate_unit_range() in sync — if a future field is added to one but
    // not the other, this test will fail.
    #[test]
    fn mid_range_baseline_is_in_unit_range() {
        let attrs = PlayerAttributes::mid_range_baseline();
        let errors = attrs.validate_unit_range();
        assert!(
            errors.is_empty(),
            "mid_range_baseline() produced out-of-range fields: {errors:?}"
        );
    }
}

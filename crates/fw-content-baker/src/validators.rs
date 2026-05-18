//! Post-LLM validation lints + structural entity validators.
//!
//! Two surfaces live here:
//!
//! **1. Structural Validators (T2-3)** — one `*Validator` struct per
//! content kind. Each exposes `pub fn validate(&self, entity) -> Result<(),
//! ValidationError>` that runs chained internal checks and returns the FIRST
//! violation. `run_validate_structural` in `main.rs` consumes these.
//!
//! **2. Semantic free-functions (T1-12, deferred)** — `check_banned_terms`,
//! `check_licensed_data`, `check_cliche`, `validate_fragment`. These still
//! return `ValidationError::NotImplemented`; they are wired to bake
//! subcommands that land at T2-4+. Do NOT remove or change them — the
//! T1-12 honesty contract pins their behaviour.

// Suppress dead_code for the NotImplemented free-fns until a bake subcommand
// consumer arrives.
#![allow(dead_code)]

use std::path::Path;

use fw_content::{
    BUILDUP_SPEED_MAX_BPS, BUILDUP_SPEED_MIN_BPS, Culture, PlayerTemplate, RoleAffinityTable,
    TacticalArchetype,
};

// ---------------------------------------------------------------------------
// ValidationError — extended with structured variants per T2-3
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    // --- Semantic / post-LLM variants (T1-12, deferred) -------------------
    #[error("banned-term violation (Category A): {0}")]
    BannedTerm(String),
    #[error("licensed-data hit: {0}")]
    LicensedData(String),
    #[error("cliché detected: {0}")]
    Cliche(String),
    #[error("validator I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Returned by any validator that has not yet been implemented.
    ///
    /// Replaces the prior `Ok(())` stub so callers that mistakenly invoke
    /// unimplemented validators fail loudly rather than silently passing
    /// (T1-12 audit-triage hardening).
    #[error(
        "validator '{validator}' is not yet implemented \
         (deferred to {defer_to}); \
         fail-closed per T1-12 audit-triage hardening"
    )]
    NotImplemented {
        /// The function name of the unimplemented validator.
        validator: &'static str,
        /// The MASTER_PLAN milestone string where real implementation lands.
        defer_to: &'static str,
    },

    // --- Structural variants (T2-3) ----------------------------------------
    #[error(
        "role-affinity table {table_id:?}: role {role:?} weights sum to \
         {sum_bps} bps (expected 10_000)"
    )]
    RoleAffinityWeightSumMismatch {
        table_id: String,
        role: String,
        sum_bps: u32,
    },

    #[error(
        "role-affinity table {table_id:?}: role {role:?} contains unknown / \
         hidden attribute key {key:?}"
    )]
    RoleAffinityUnknownAttributeKey {
        table_id: String,
        role: String,
        key: String,
    },

    #[error("player template {player_id:?}: attribute out of [0, 1] range — {detail}")]
    PlayerAttributeOutOfRange { player_id: String, detail: String },

    #[error("player template {player_id:?}: ability ceiling invalid — {detail}")]
    PlayerCeilingInvalid { player_id: String, detail: String },

    #[error("culture {culture_id:?}: first_name_bank has {len} entries (minimum 20)")]
    CultureFirstNameBankTooSmall { culture_id: String, len: usize },

    #[error("culture {culture_id:?}: last_name_bank has {len} entries (minimum 20)")]
    CultureLastNameBankTooSmall { culture_id: String, len: usize },

    #[error("tactical archetype {archetype_id:?}: formation has {len} slots (expected 11)")]
    TacticalArchetypeFormationWrongSize { archetype_id: String, len: usize },

    #[error(
        "tactical archetype {archetype_id:?}: buildup_speed_factor_bps {bps} \
         outside [{min}, {max}]"
    )]
    TacticalArchetypeBuildupSpeedOutOfRange {
        archetype_id: String,
        bps: u16,
        min: u16,
        max: u16,
    },

    #[error(
        "tactical archetype {archetype_id:?}: formation roster_slot values \
         are not a permutation of 1..=11 — {detail}"
    )]
    TacticalArchetypeFormationSlotsInvalid {
        archetype_id: String,
        detail: String,
    },
}

// ---------------------------------------------------------------------------
// RoleAffinityTableValidator
// ---------------------------------------------------------------------------

/// Structural validator for `RoleAffinityTable` fixtures.
///
/// Chained checks (in order):
///  1. `invalid_roles()` — every role's weights must sum to 10_000 bps.
///  2. `unknown_attribute_keys()` — every weight key must resolve to
///     `fw_core::VISIBLE_ATTRIBUTE_NAMES`.
#[derive(Debug, Default)]
pub struct RoleAffinityTableValidator;

impl RoleAffinityTableValidator {
    pub fn new() -> Self {
        Self
    }

    /// Run all structural checks on `entity`. Returns the FIRST violation.
    /// The check ordering is part of the contract — unit tests assert on
    /// specific variants at specific positions in the chain.
    pub fn validate(&self, entity: &RoleAffinityTable) -> Result<(), ValidationError> {
        self.check_weight_sums(entity)?;
        self.check_attribute_keys(entity)?;
        Ok(())
    }

    fn check_weight_sums(&self, t: &RoleAffinityTable) -> Result<(), ValidationError> {
        if let Some((role, sum_bps)) = t.invalid_roles().into_iter().next() {
            return Err(ValidationError::RoleAffinityWeightSumMismatch {
                table_id: t.id.clone(),
                role: role.to_string(),
                sum_bps,
            });
        }
        Ok(())
    }

    fn check_attribute_keys(&self, t: &RoleAffinityTable) -> Result<(), ValidationError> {
        if let Some((role, key)) = t.unknown_attribute_keys().into_iter().next() {
            return Err(ValidationError::RoleAffinityUnknownAttributeKey {
                table_id: t.id.clone(),
                role: role.to_string(),
                key: key.to_string(),
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PlayerTemplateValidator
// ---------------------------------------------------------------------------

/// Structural validator for `PlayerTemplate` fixtures.
///
/// Chained checks (in order):
///  1. `attributes.validate_unit_range()` — every Q32 attribute field in [0, 1].
///  2. `ceiling.validate()` — current <= potential, both in [0, 1].
#[derive(Debug, Default)]
pub struct PlayerTemplateValidator;

impl PlayerTemplateValidator {
    pub fn new() -> Self {
        Self
    }

    /// Run all structural checks on `entity`. Returns the FIRST violation.
    pub fn validate(&self, entity: &PlayerTemplate) -> Result<(), ValidationError> {
        self.check_attribute_unit_range(entity)?;
        self.check_ability_ceiling(entity)?;
        Ok(())
    }

    fn check_attribute_unit_range(&self, t: &PlayerTemplate) -> Result<(), ValidationError> {
        let errs = t.attributes.validate_unit_range();
        if let Some(first_err) = errs.into_iter().next() {
            return Err(ValidationError::PlayerAttributeOutOfRange {
                player_id: t.qualified_id.clone(),
                detail: first_err.to_string(),
            });
        }
        Ok(())
    }

    fn check_ability_ceiling(&self, t: &PlayerTemplate) -> Result<(), ValidationError> {
        if let Err(e) = t.ceiling.validate() {
            return Err(ValidationError::PlayerCeilingInvalid {
                player_id: t.qualified_id.clone(),
                detail: e.to_string(),
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// CultureValidator
// ---------------------------------------------------------------------------

/// Structural validator for `Culture` fixtures.
///
/// Chained checks (in order):
///  1. `first_name_bank.len() >= 20` — doc-comment minimum.
///  2. `last_name_bank.len() >= 20` — doc-comment minimum.
///
/// **NOT checked (deferred to T2-4):** `team_name_bank` size + content.
/// T2-3 only consumes `first_name_bank` × `last_name_bank` via
/// `BakeNamesOffline`; the first `team_name_bank` consumer lands at T2-4
/// alongside `BakeTeamNames` (or similar), at which point this validator
/// gains a `check_team_name_bank` chained check. Documenting the gap here so
/// reviewers reading "Structural validator for `Culture` fixtures" don't
/// reasonably assume all banks are checked.
/// Post-T2-3 silent-failure-hunter P1-edge honesty fix.
///
/// **STRUCTURAL ONLY — NOT semantic** (T2-R7(b) honesty, post-T2 Codex Track
/// E-2). The chained checks above verify the bank sizes meet the
/// doc-declared minimum. They do NOT sample composed name output and lint
/// it for banned terms / licensed-data collisions / cliché overlap.
/// Specifically: a `Culture` with 20 first_names all "Man" + 20 last_names
/// all "chester" + `naming_pattern: "{first}{last}"` passes this validator
/// AND deterministically generates the banned place-name "Manchester" at
/// bake time. The semantic validator that samples composed output + runs
/// `scripts/lint-banned-terms.py` against the generated strings lands at
/// T2-4 alongside `PlayerBioValidator` (which has the same shape: chained
/// structural checks here; composed-output sampling at T2-4 when the real
/// bake pipeline ships). Until then, treat `CultureValidator::validate`
/// output as a NECESSARY but NOT SUFFICIENT check before publishing a
/// content pack.
#[derive(Debug, Default)]
pub struct CultureValidator;

impl CultureValidator {
    pub fn new() -> Self {
        Self
    }

    /// Run all structural checks on `entity`. Returns the FIRST violation.
    pub fn validate(&self, entity: &Culture) -> Result<(), ValidationError> {
        self.check_first_name_bank(entity)?;
        self.check_last_name_bank(entity)?;
        Ok(())
    }

    fn check_first_name_bank(&self, c: &Culture) -> Result<(), ValidationError> {
        const MIN: usize = 20;
        if c.first_name_bank.len() < MIN {
            return Err(ValidationError::CultureFirstNameBankTooSmall {
                culture_id: c.id.clone(),
                len: c.first_name_bank.len(),
            });
        }
        Ok(())
    }

    fn check_last_name_bank(&self, c: &Culture) -> Result<(), ValidationError> {
        const MIN: usize = 20;
        if c.last_name_bank.len() < MIN {
            return Err(ValidationError::CultureLastNameBankTooSmall {
                culture_id: c.id.clone(),
                len: c.last_name_bank.len(),
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// TacticalArchetypeValidator
// ---------------------------------------------------------------------------

/// Structural validator for `TacticalArchetype` fixtures.
///
/// Chained checks (in order):
///  1. `formation.len() == 11` — exactly 11 outfield slots.
///  2. `buildup_speed_factor_bps in [BUILDUP_SPEED_MIN_BPS, BUILDUP_SPEED_MAX_BPS]`.
///  3. `formation` roster_slot values are a permutation of 1..=11 (no duplicates,
///     no gaps, covers all 11 positions).
#[derive(Debug, Default)]
pub struct TacticalArchetypeValidator;

impl TacticalArchetypeValidator {
    pub fn new() -> Self {
        Self
    }

    /// Run all structural checks on `entity`. Returns the FIRST violation.
    pub fn validate(&self, entity: &TacticalArchetype) -> Result<(), ValidationError> {
        self.check_formation_size(entity)?;
        self.check_buildup_speed(entity)?;
        self.check_formation_slots(entity)?;
        Ok(())
    }

    fn check_formation_size(&self, t: &TacticalArchetype) -> Result<(), ValidationError> {
        if t.formation.len() != 11 {
            return Err(ValidationError::TacticalArchetypeFormationWrongSize {
                archetype_id: t.id.clone(),
                len: t.formation.len(),
            });
        }
        Ok(())
    }

    fn check_buildup_speed(&self, t: &TacticalArchetype) -> Result<(), ValidationError> {
        let bps = t.buildup_speed_factor_bps;
        if !(BUILDUP_SPEED_MIN_BPS..=BUILDUP_SPEED_MAX_BPS).contains(&bps) {
            return Err(ValidationError::TacticalArchetypeBuildupSpeedOutOfRange {
                archetype_id: t.id.clone(),
                bps,
                min: BUILDUP_SPEED_MIN_BPS,
                max: BUILDUP_SPEED_MAX_BPS,
            });
        }
        Ok(())
    }

    fn check_formation_slots(&self, t: &TacticalArchetype) -> Result<(), ValidationError> {
        // Collect roster_slot values and verify they are exactly the set 1..=11.
        // Use a sorted Vec (not BTreeSet) to produce a deterministic error message.
        let mut slots: Vec<u8> = t.formation.iter().map(|s| s.roster_slot).collect();
        slots.sort_unstable();

        let expected: Vec<u8> = (1u8..=11).collect();
        if slots != expected {
            return Err(ValidationError::TacticalArchetypeFormationSlotsInvalid {
                archetype_id: t.id.clone(),
                detail: format!(
                    "found roster_slots {:?}; expected a permutation of {:?}",
                    slots, expected
                ),
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Semantic free-functions (T1-12 deferred — DO NOT CHANGE)
// ---------------------------------------------------------------------------

/// Run the banned-terms lint against a generated content fragment.
///
/// Real implementation (T2-3): spawns
/// `scripts/lint-banned-terms.py <tmpfile>` and parses exit code + stderr.
///
/// Until T2-3 wires `bake-names` as the first consumer, this returns
/// `ValidationError::NotImplemented` so any premature caller fails loudly.
pub fn check_banned_terms(_fragment_path: &Path) -> Result<(), ValidationError> {
    Err(ValidationError::NotImplemented {
        validator: "check_banned_terms",
        defer_to: "T2-3",
    })
}

/// Reject any text containing a real-world licensed club / surname.
///
/// Curated list lives in `data/licensed-blocklist.txt` (gitignored from
/// shipped build; lives alongside this crate for dev-only use).
///
/// Until T2-3 wires `bake-names` as the first consumer, this returns
/// `ValidationError::NotImplemented` so any premature caller fails loudly.
pub fn check_licensed_data(_text: &str) -> Result<(), ValidationError> {
    Err(ValidationError::NotImplemented {
        validator: "check_licensed_data",
        defer_to: "T2-3",
    })
}

/// Cliché detector — reject obvious LLM tells.
///
/// Default patterns (override per-fragment via sentinel):
/// - "passionate about"
/// - "exceptional ability to"
/// - "rising star with bright future"
/// - "the world of football"
/// - "wears his heart on his sleeve" (acceptable as football vernacular but
///   over-used by LLMs; soft-rejected and devs decide)
///
/// Until T2-3 wires `bake-names` as the first consumer, this returns
/// `ValidationError::NotImplemented` so any premature caller fails loudly.
pub fn check_cliche(_text: &str) -> Result<(), ValidationError> {
    Err(ValidationError::NotImplemented {
        validator: "check_cliche",
        defer_to: "T2-3",
    })
}

/// Run all three semantic validators in order. Returns the first error encountered.
///
/// Post-T2-3 silent-failure-hunter P1 fix: the prior implementation chained the
/// three `NotImplemented` free-fns via `?` and the test asserted on the
/// specific "first" validator's `validator` field. That test was a tripwire —
/// the moment ONE of the three landed (say `check_banned_terms` returns
/// `Ok(())`), the assertion that `validate_fragment` returns
/// `validator: "check_banned_terms"` would flip to failing for a reason
/// unrelated to the test's intent ("did the chain order change?" when actually
/// "the first validator just became implemented").
///
/// Today `validate_fragment` ITSELF fails-loud as `NotImplemented` with its
/// own `validator: "validate_fragment"` identifier. When the three inner
/// validators land one at a time at T2-4+, this function's body chains them
/// in declared order — and the "first error wins" contract becomes meaningful
/// because the three errors will be distinct variants
/// (`BannedTerm` / `LicensedData` / `Cliche`), not three indistinguishable
/// `NotImplemented` shapes.
pub fn validate_fragment(_text: &str, _fragment_path: &Path) -> Result<(), ValidationError> {
    Err(ValidationError::NotImplemented {
        validator: "validate_fragment",
        defer_to: "T2-4",
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::Path;

    use fw_content::{
        BUILDUP_SPEED_BASELINE_BPS, BUILDUP_SPEED_MAX_BPS, BUILDUP_SPEED_MIN_BPS, Culture,
        CultureWeights, FormationSlot, PlayerTemplate, ROLE_AFFINITY_SCHEMA_VERSION,
        RoleAffinityTable, TacticalArchetype,
    };
    use fw_core::{
        AbilityCeiling, DurabilityProfile, GoalkeeperAttributes, MentalAttributes,
        PersonalityVector, PhysicalAttributes, PlayerAttributes, PlayerId, Q32,
        TechnicalAttributes,
    };

    // --- T1-12 NotImplemented tests (DO NOT CHANGE) -------------------------
    //
    // T2-R-D5 (post-T2 ultimate-review Track D-5 honesty): the FOUR
    // tests below verify that stub functions return
    // `Err(ValidationError::NotImplemented { .. })`. They exercise the
    // STUB contract only, not production validation logic. When T2-4 /
    // T3-1 ships the real implementations:
    //   - `check_banned_terms`     → wired against `scripts/lint-banned-terms.py`
    //   - `check_licensed_data`    → wired against the licensed-data corpus
    //   - `check_cliche`           → wired against the cliché-detection pass
    //   - `validate_fragment`      → composes the three above + structural checks
    // these four tests MUST be REMOVED OR REWRITTEN into happy-path +
    // rejection-path pairs (mirror the CultureValidator / PlayerTemplateValidator
    // test shape above). Until then, treat their PASSING status as
    // "the stub contract holds" — NOT as "semantic validation is covered."

    #[test]
    fn check_banned_terms_returns_not_implemented_with_correct_fields() {
        let err = check_banned_terms(Path::new("/fake/fragment.ron"))
            .expect_err("check_banned_terms must return NotImplemented");
        match err {
            ValidationError::NotImplemented {
                validator,
                defer_to,
            } => {
                assert_eq!(validator, "check_banned_terms");
                assert_eq!(defer_to, "T2-3");
            }
            other => panic!("expected NotImplemented, got {other:?}"),
        }
    }

    #[test]
    fn check_licensed_data_returns_not_implemented_with_correct_fields() {
        let err = check_licensed_data("some generated text")
            .expect_err("check_licensed_data must return NotImplemented");
        match err {
            ValidationError::NotImplemented {
                validator,
                defer_to,
            } => {
                assert_eq!(validator, "check_licensed_data");
                assert_eq!(defer_to, "T2-3");
            }
            other => panic!("expected NotImplemented, got {other:?}"),
        }
    }

    #[test]
    fn check_cliche_returns_not_implemented_with_correct_fields() {
        let err = check_cliche("some generated text")
            .expect_err("check_cliche must return NotImplemented");
        match err {
            ValidationError::NotImplemented {
                validator,
                defer_to,
            } => {
                assert_eq!(validator, "check_cliche");
                assert_eq!(defer_to, "T2-3");
            }
            other => panic!("expected NotImplemented, got {other:?}"),
        }
    }

    #[test]
    fn validate_fragment_returns_its_own_not_implemented_identifier() {
        // Post-T2-3 silent-failure-hunter P1 fix: `validate_fragment` itself
        // returns NotImplemented (no longer chains the three inner
        // free-fns). The test now asserts on `validator: "validate_fragment"`
        // and `defer_to: "T2-4"` — the SEMANTIC contract of "the chain
        // semantic-validator surface is not yet wired" — rather than on the
        // fragile "which inner free-fn fires first" ordering. When the inner
        // validators land at T2-4+, this test gets reshaped to assert the
        // first REAL error wins.
        let err = validate_fragment("text", Path::new("/fake/x.ron"))
            .expect_err("validate_fragment must return NotImplemented");
        match err {
            ValidationError::NotImplemented {
                validator,
                defer_to,
            } => {
                assert_eq!(validator, "validate_fragment");
                assert_eq!(defer_to, "T2-4");
            }
            other => panic!("expected NotImplemented from validate_fragment, got {other:?}"),
        }
    }

    // --- T2-3: well-formed entity smoke tests (AC1 surface) ----------------

    /// Constructs a known-good `RoleAffinityTable` fixture in code.
    fn well_formed_role_affinity_table() -> RoleAffinityTable {
        // Single role "GK" whose weights sum exactly to 10_000 bps.
        // Uses the public API surface from fw-content::role_affinity.
        use fw_content::RoleWeights;
        let mut weights = BTreeMap::new();
        weights.insert("handling".to_string(), 5_000u16);
        weights.insert("reflexes".to_string(), 5_000u16);
        let role_weights = RoleWeights {
            weights_bps: weights,
        };
        let mut roles = BTreeMap::new();
        roles.insert(fw_content::RoleId::new("GK"), role_weights);
        RoleAffinityTable {
            schema_version: ROLE_AFFINITY_SCHEMA_VERSION,
            id: "fwh.core:role-affinities.test".to_string(),
            roles,
        }
    }

    fn well_formed_player_template() -> PlayerTemplate {
        let half = Q32::from_raw(1i64 << 31); // ~0.5
        PlayerTemplate {
            schema_version: fw_content::PLAYER_TEMPLATE_SCHEMA_VERSION,
            id: PlayerId::new(1),
            qualified_id: "fwh.core:player_00001".to_string(),
            display_name: "Test Player".to_string(),
            attributes: PlayerAttributes {
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
            },
            ceiling: AbilityCeiling::try_new(half, Q32::from_raw(3i64 << 30))
                .expect("valid ceiling"),
            preferred_role: fw_content::RoleId::new("GK"),
            signature_candidates: vec![],
        }
    }

    fn well_formed_culture() -> Culture {
        Culture {
            id: "fwh.core:culture.test".to_string(),
            name: "Test".to_string(),
            first_name_bank: (0..20).map(|i| format!("First{i}")).collect(),
            last_name_bank: (0..20).map(|i| format!("Last{i}")).collect(),
            team_name_bank: vec![],
            naming_pattern: "{first} {last}".to_string(),
            weights: CultureWeights::default(),
        }
    }

    fn well_formed_tactical_archetype() -> TacticalArchetype {
        TacticalArchetype {
            id: "fwh.core:archetype.test".to_string(),
            formation: (1u8..=11)
                .map(|i| FormationSlot {
                    roster_slot: i,
                    role: "XX".to_string(),
                    x: 0,
                    z: 0,
                })
                .collect(),
            press_radius_metres: 20,
            buildup_speed_factor_bps: BUILDUP_SPEED_BASELINE_BPS,
        }
    }

    #[test]
    fn four_validators_exist_and_expose_validate_method() {
        // AC1: each Validator::new() accepts a well-formed entity.
        assert!(
            RoleAffinityTableValidator::new()
                .validate(&well_formed_role_affinity_table())
                .is_ok(),
            "RoleAffinityTableValidator must accept a well-formed fixture"
        );
        assert!(
            PlayerTemplateValidator::new()
                .validate(&well_formed_player_template())
                .is_ok(),
            "PlayerTemplateValidator must accept a well-formed fixture"
        );
        assert!(
            CultureValidator::new()
                .validate(&well_formed_culture())
                .is_ok(),
            "CultureValidator must accept a well-formed fixture"
        );
        assert!(
            TacticalArchetypeValidator::new()
                .validate(&well_formed_tactical_archetype())
                .is_ok(),
            "TacticalArchetypeValidator must accept a well-formed fixture"
        );
    }

    // --- T2-3: malformed-fixture rejection tests (AC2) ----------------------

    #[test]
    fn role_affinity_validator_rejects_malformed_fixture_with_structured_error() {
        // Construct a RoleAffinityTable whose role "ST" sums to 9_999 instead
        // of 10_000. Only the weight-sum check fires; the attribute-key check
        // passes because "passing" is a valid visible attribute name.
        use fw_content::RoleWeights;
        let mut weights = BTreeMap::new();
        weights.insert("passing".to_string(), 9_999u16); // sum = 9_999, not 10_000
        let role_weights = RoleWeights {
            weights_bps: weights,
        };
        let mut roles = BTreeMap::new();
        roles.insert(fw_content::RoleId::new("ST"), role_weights);
        let bad_table = RoleAffinityTable {
            schema_version: ROLE_AFFINITY_SCHEMA_VERSION,
            id: "fwh.core:role-affinities.bad".to_string(),
            roles,
        };

        let err = RoleAffinityTableValidator::new()
            .validate(&bad_table)
            .expect_err("must reject weight-sum mismatch");

        match err {
            ValidationError::RoleAffinityWeightSumMismatch {
                table_id,
                role,
                sum_bps,
            } => {
                assert_eq!(table_id, "fwh.core:role-affinities.bad");
                assert_eq!(role, "ST");
                assert_eq!(sum_bps, 9_999);
            }
            other => panic!("expected RoleAffinityWeightSumMismatch, got {other:?}"),
        }
    }

    #[test]
    fn player_template_validator_rejects_malformed_fixture_with_structured_error() {
        // Build a PlayerTemplate with one attribute out of [0, 1] range:
        // set finishing to Q32::ONE + 1 raw unit (just above 1.0).
        let mut t = well_formed_player_template();
        // Q32::ONE is the fixed-point representation of 1.0. Adding 1 raw
        // unit pushes it to 1.0 + 2^-32, which is outside [0, 1].
        t.attributes.technical.finishing = Q32::from_raw(Q32::ONE.to_bits() + 1);

        let err = PlayerTemplateValidator::new()
            .validate(&t)
            .expect_err("must reject attribute out of range");

        match err {
            ValidationError::PlayerAttributeOutOfRange { player_id, detail } => {
                assert_eq!(player_id, "fwh.core:player_00001");
                assert!(!detail.is_empty(), "detail must name the offending field");
            }
            other => panic!("expected PlayerAttributeOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn culture_validator_rejects_malformed_fixture_with_structured_error() {
        // Culture with only 19 first names (one below the 20-entry minimum).
        // The second check (last_name_bank) would also pass — the first check
        // fires first and returns, which is what we're testing.
        let mut c = well_formed_culture();
        c.first_name_bank.truncate(19);

        let err = CultureValidator::new()
            .validate(&c)
            .expect_err("must reject first_name_bank with < 20 entries");

        match err {
            ValidationError::CultureFirstNameBankTooSmall { culture_id, len } => {
                assert_eq!(culture_id, "fwh.core:culture.test");
                assert_eq!(len, 19);
            }
            other => panic!("expected CultureFirstNameBankTooSmall, got {other:?}"),
        }
    }

    #[test]
    fn tactical_archetype_validator_rejects_malformed_fixture_with_structured_error() {
        // Formation with only 10 slots (one missing). The formation-size check
        // fires first, before the buildup-speed or slot-permutation checks.
        let mut a = well_formed_tactical_archetype();
        a.formation.pop(); // now 10 slots

        let err = TacticalArchetypeValidator::new()
            .validate(&a)
            .expect_err("must reject formation with != 11 slots");

        match err {
            ValidationError::TacticalArchetypeFormationWrongSize { archetype_id, len } => {
                assert_eq!(archetype_id, "fwh.core:archetype.test");
                assert_eq!(len, 10);
            }
            other => panic!("expected TacticalArchetypeFormationWrongSize, got {other:?}"),
        }
    }

    // --- T2-3: additional chained-check ordering tests ----------------------

    #[test]
    fn role_affinity_validator_rejects_unknown_attribute_key_after_sum_passes() {
        // Role with valid sum (10_000) but an unknown key ("stamina" is a
        // physical attribute, not a visible attribute in the CA derivation set).
        // Wait — stamina IS in VISIBLE_ATTRIBUTE_NAMES; use "injury_proneness"
        // which is a hidden field banned from CA derivation.
        use fw_content::RoleWeights;
        let mut weights = BTreeMap::new();
        weights.insert("injury_proneness".to_string(), 10_000u16); // unknown key
        let role_weights = RoleWeights {
            weights_bps: weights,
        };
        let mut roles = BTreeMap::new();
        roles.insert(fw_content::RoleId::new("AM"), role_weights);
        let bad_table = RoleAffinityTable {
            schema_version: ROLE_AFFINITY_SCHEMA_VERSION,
            id: "fwh.core:role-affinities.badkey".to_string(),
            roles,
        };

        let err = RoleAffinityTableValidator::new()
            .validate(&bad_table)
            .expect_err("must reject unknown attribute key");

        match err {
            ValidationError::RoleAffinityUnknownAttributeKey {
                table_id,
                role,
                key,
            } => {
                assert_eq!(table_id, "fwh.core:role-affinities.badkey");
                assert_eq!(role, "AM");
                assert_eq!(key, "injury_proneness");
            }
            other => panic!("expected RoleAffinityUnknownAttributeKey, got {other:?}"),
        }
    }

    #[test]
    fn tactical_archetype_validator_rejects_out_of_range_buildup_speed() {
        // Formation with 11 slots (passes size check) but buildup speed
        // of 4_999, below BUILDUP_SPEED_MIN_BPS (5_000).
        let mut a = well_formed_tactical_archetype();
        a.buildup_speed_factor_bps = BUILDUP_SPEED_MIN_BPS - 1;

        let err = TacticalArchetypeValidator::new()
            .validate(&a)
            .expect_err("must reject out-of-range buildup speed");

        match err {
            ValidationError::TacticalArchetypeBuildupSpeedOutOfRange {
                archetype_id,
                bps,
                min,
                max,
            } => {
                assert_eq!(archetype_id, "fwh.core:archetype.test");
                assert_eq!(bps, BUILDUP_SPEED_MIN_BPS - 1);
                assert_eq!(min, BUILDUP_SPEED_MIN_BPS);
                assert_eq!(max, BUILDUP_SPEED_MAX_BPS);
            }
            other => panic!("expected TacticalArchetypeBuildupSpeedOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn tactical_archetype_validator_rejects_duplicate_roster_slots() {
        // Formation with 11 slots, valid speed, but slot 1 appears twice and
        // slot 11 is missing. Tests that the slot-permutation check fires after
        // size + speed both pass.
        let mut a = well_formed_tactical_archetype();
        // Replace last slot (roster_slot=11) with a duplicate of slot 1.
        a.formation.last_mut().unwrap().roster_slot = 1;

        let err = TacticalArchetypeValidator::new()
            .validate(&a)
            .expect_err("must reject duplicate roster slots");

        match err {
            ValidationError::TacticalArchetypeFormationSlotsInvalid {
                archetype_id,
                detail,
            } => {
                assert_eq!(archetype_id, "fwh.core:archetype.test");
                assert!(
                    detail.contains("roster_slots"),
                    "detail must mention roster_slots"
                );
            }
            other => panic!("expected TacticalArchetypeFormationSlotsInvalid, got {other:?}"),
        }
    }

    #[test]
    fn culture_validator_rejects_small_last_name_bank_after_first_passes() {
        // First bank has 20 entries (passes), last bank has 19 (fails).
        // Tests that the second chained check fires correctly.
        let mut c = well_formed_culture();
        c.last_name_bank.truncate(19);

        let err = CultureValidator::new()
            .validate(&c)
            .expect_err("must reject last_name_bank with < 20 entries");

        match err {
            ValidationError::CultureLastNameBankTooSmall { culture_id, len } => {
                assert_eq!(culture_id, "fwh.core:culture.test");
                assert_eq!(len, 19);
            }
            other => panic!("expected CultureLastNameBankTooSmall, got {other:?}"),
        }
    }
}

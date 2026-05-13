//! Player templates — content-pack-authored player records.
//!
//! `PlayerTemplate` wraps the canonical attribute types from `fw-core` for
//! content-pack RON authoring. Loaded once by `ContentStore::load_baked`;
//! immutable after load (Pillar 3 mutation lives in `fw-memory` ledger
//! writes, never on the template).
//!
//! Implements ADR-0002 (`docs/adr/0002-player-attribute-model.md`):
//! the 55-field player model (38 visible + 17 hidden) lives in
//! `fw-core::player_attributes`; this module composes those types into
//! the content-pack-facing record.
//!
//! **What's NOT here (deliberately):** `fw-core::PlayerCondition` (form /
//! morale / match_fitness / sharpness / signature_readiness). That struct
//! is runtime modulator state — the BT-runner mutates it tick-by-tick.
//! Embedding it in the content-pack template would couple save migrations
//! to fixture files (any condition-field bump would require migrating
//! every committed RON). Condition is initialized at template →
//! canonical-player projection time by `fw-match-sim` (T1-2b+), not
//! authored in the content pack.

use fw_core::{AbilityCeiling, PlayerAttributes, PlayerId};
use serde::{Deserialize, Serialize};

use crate::role_affinity::RoleId;

/// Current schema version for `PlayerTemplate`. Bumped at every
/// breaking-shape change; forward-migration only (per
/// `Content/RULES.md` §3).
///
/// **Marker-only at T1-1.** Codex audit P2 (2026-05-13): the field
/// exists on `PlayerTemplate` + the load-test asserts it equals 1, but
/// no `ContentLoader` gates loading on `schema_version <= MAX_SUPPORTED`
/// yet. The gate lands at **Tranche 6** as part of the real
/// `ContentStore::load_baked` implementation (the current `load_baked`
/// is a stub returning `Ok(Self::default())`). Until then, treat this
/// constant as a forward-compatibility marker — content packs commit
/// `schema_version: 1` so the future loader knows what to do.
pub const PLAYER_TEMPLATE_SCHEMA_VERSION: u32 = 1;

/// A player-template entry from a content pack.
///
/// Carries the durable `PlayerId` (`u32` newtype, locked at Codex Q2),
/// the content-pack-qualified textual ID for cross-pack references, the
/// display name, the full ADR-0002 attribute bundle, the ability ceiling
/// (`current` derived at read time, `potential` mutable only via
/// `MemoryEvent::Breakthrough`), and the preferred-role used by
/// `RoleAffinityTable` to weight CA derivation.
///
/// Field order is load-bearing for serde determinism — do not reorder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerTemplate {
    /// Schema version of this record. Forward-migrated by the content
    /// loader (T2-3+). Bumped on every breaking-shape change.
    pub schema_version: u32,

    /// Stable durable handle. Allocated by the content compiler at
    /// pack-bake time; persists across save → load.
    pub id: PlayerId,

    /// Content-pack-qualified textual ID (`fwh.core:player_00042`). The
    /// canonical form for cross-pack references; validated by the
    /// content-pack-validate gauntlet (FW-VAL, T2-3+).
    pub qualified_id: String,

    /// Display name. Procedurally generated in T2-4 from the player's
    /// culture; hand-authored for fixtures.
    pub display_name: String,

    /// The 55-field attribute bundle from ADR-0002 (`fw-core::PlayerAttributes`).
    pub attributes: PlayerAttributes,

    /// Current Ability + Potential Ability. `current` is recomputed at read
    /// time from a role-conditioned weighted sum of visible attributes;
    /// `potential` is stored and mutable only via breakthroughs (through
    /// the `pub(crate) redraw_ceiling` API in `fw-core`).
    pub ceiling: AbilityCeiling,

    /// Preferred role (e.g. `"GK"`, `"CB"`, `"AM"`). Resolves at
    /// CA-derivation time via `RoleAffinityTable`. Typed as `RoleId`
    /// rather than `String` so a misspelling at the call site is a
    /// compile-time error, not a silent `RoleAffinityTable::get` miss.
    /// An unrecognized role at lookup time surfaces as a load-time
    /// FW-VAL error.
    pub preferred_role: RoleId,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_core::{
        DurabilityProfile, GoalkeeperAttributes, MentalAttributes, PersonalityVector,
        PhysicalAttributes, Q32, TechnicalAttributes,
    };

    /// Build a sample `PlayerTemplate` with mid-range attributes. Shared
    /// helper for tests below.
    pub(super) fn sample_template() -> PlayerTemplate {
        let half = Q32::from_raw(1i64 << 31); // ~0.5
        PlayerTemplate {
            schema_version: PLAYER_TEMPLATE_SCHEMA_VERSION,
            id: PlayerId::new(42),
            qualified_id: "fwh.core:player_00042".to_string(),
            display_name: "James Tabor".to_string(),
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
                .expect("sample ceiling is valid"), // ~0.5 / ~0.75
            preferred_role: RoleId::new("AM"),
        }
    }

    #[test]
    fn ron_round_trip() {
        let template = sample_template();
        let ron_text = ron::ser::to_string(&template).expect("ron encode");
        let decoded: PlayerTemplate = ron::de::from_str(&ron_text).expect("ron decode");
        assert_eq!(decoded, template);
    }

    #[test]
    fn ron_pretty_round_trip() {
        // Pretty-printed RON is what content authors edit; round-trip must
        // also work via the pretty serializer.
        let template = sample_template();
        let ron_text = ron::ser::to_string_pretty(&template, ron::ser::PrettyConfig::default())
            .expect("ron encode pretty");
        let decoded: PlayerTemplate = ron::de::from_str(&ron_text).expect("ron decode");
        assert_eq!(decoded, template);
    }

    #[test]
    fn schema_version_is_one_at_t1_1() {
        let template = sample_template();
        assert_eq!(template.schema_version, 1);
        assert_eq!(template.schema_version, PLAYER_TEMPLATE_SCHEMA_VERSION);
    }
}

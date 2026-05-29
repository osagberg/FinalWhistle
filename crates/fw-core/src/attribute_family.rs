//! Attribute-family enum — the 10 coarse family groupings used by the
//! breakthrough meter.
//!
//! Relocated from `fw-memory::breakthrough` to `fw-core` at T4-2.5a so
//! `fw-content::breakthrough_input` can consume it without a dependency on
//! `fw-memory` (which would create a cycle: fw-content → fw-memory →
//! fw-core; fw-content already depends on fw-core).
//!
//! ## Tag-stability (LOAD-BEARING FOREVER)
//!
//! `#[repr(u32)]` + explicit discriminants 0..9 pin the canonical career-state
//! encoding. Re-ordering variants or changing discriminants is a schema-breaking
//! change. The wire discriminants are further pinned by
//! `attribute_family_discriminants_locked` + `attribute_family_serde_roundtrip`
//! tests.
//!
//! Mod content packs do NOT add families — the family set is closed at T3-4.

use serde::{Deserialize, Serialize};

/// Coarse attribute-family grouping used by the breakthrough meter.
///
/// The 10 families from `docs/design/progression.md` §"Attribute-family list".
///
/// ## Tag-stability (LOAD-BEARING FOREVER)
///
/// `#[repr(u32)]` + explicit discriminants 0..9 pin the canonical career-state
/// encoding. Re-ordering variants or changing discriminants is a schema-breaking
/// change. The wire discriminants are further pinned by
/// `attribute_family_discriminants_locked` test.
///
/// Mod content packs do NOT add families — the family set is closed at T3-4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u32)]
pub enum AttributeFamily {
    /// Conversion, composure in the box. Strikers, AMs.
    Finishing = 0,
    /// Range, vision, precision. Midfielders, full-backs.
    Passing = 1,
    /// Reading the game, positioning. Centre-backs, DMs.
    DefensiveAnticipation = 2,
    /// Heading, physical duels. Centre-backs, target men.
    AerialPresence = 3,
    /// Pressure response, decision quality under duress. All positions.
    Composure = 4,
    /// Explosive speed, acceleration. Wingers, strikers, attacking full-backs.
    Pace = 5,
    /// Late-match intensity, injury-load resilience. All positions.
    Stamina = 6,
    /// Pressing, tracking-back, shuttle runs. Pressing mids, box-to-box.
    WorkRate = 7,
    /// Set-pieces, free kicks, penalties. Specialist DM / AM / full-back.
    DeadBallDelivery = 8,
    /// Dressing-room influence, mentoring yield. Captains, senior figures.
    Leadership = 9,
}

impl AttributeFamily {
    /// The `#[repr(u32)]` discriminant for this variant.
    ///
    /// Pinned by `attribute_family_discriminants_locked` test.
    #[must_use]
    pub fn discriminant(self) -> u32 {
        self as u32
    }

    /// All 10 families in discriminant order. Canonical for iteration.
    pub const ALL: [AttributeFamily; 10] = [
        AttributeFamily::Finishing,
        AttributeFamily::Passing,
        AttributeFamily::DefensiveAnticipation,
        AttributeFamily::AerialPresence,
        AttributeFamily::Composure,
        AttributeFamily::Pace,
        AttributeFamily::Stamina,
        AttributeFamily::WorkRate,
        AttributeFamily::DeadBallDelivery,
        AttributeFamily::Leadership,
    ];
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the discriminants. Any re-ordering of variants breaks this test,
    /// which is exactly the intent — discriminants are load-bearing for the
    /// canonical career-state encoding.
    #[test]
    fn attribute_family_discriminants_locked() {
        assert_eq!(AttributeFamily::Finishing.discriminant(), 0);
        assert_eq!(AttributeFamily::Passing.discriminant(), 1);
        assert_eq!(AttributeFamily::DefensiveAnticipation.discriminant(), 2);
        assert_eq!(AttributeFamily::AerialPresence.discriminant(), 3);
        assert_eq!(AttributeFamily::Composure.discriminant(), 4);
        assert_eq!(AttributeFamily::Pace.discriminant(), 5);
        assert_eq!(AttributeFamily::Stamina.discriminant(), 6);
        assert_eq!(AttributeFamily::WorkRate.discriminant(), 7);
        assert_eq!(AttributeFamily::DeadBallDelivery.discriminant(), 8);
        assert_eq!(AttributeFamily::Leadership.discriminant(), 9);
    }

    #[test]
    fn attribute_family_all_has_ten_entries_in_discriminant_order() {
        assert_eq!(AttributeFamily::ALL.len(), 10);
        for (i, family) in AttributeFamily::ALL.iter().enumerate() {
            assert_eq!(
                family.discriminant(),
                i as u32,
                "ALL[{i}] must have discriminant {i}"
            );
        }
    }

    /// RON name roundtrip — verifies that variant names survive RON encode/decode
    /// before and after relocation from fw-memory to fw-core. RON uses names,
    /// not numeric discriminants, so this guards against a rename breaking
    /// the RON fixture format. For the actual numeric wire-discriminant pin,
    /// see `attribute_family_bincode_roundtrip`.
    #[test]
    fn attribute_family_serde_roundtrip() {
        for family in AttributeFamily::ALL {
            let encoded = ron::ser::to_string(&family).expect("ron encode");
            let decoded: AttributeFamily = ron::de::from_str(&encoded).expect("ron decode");
            assert_eq!(decoded, family, "serde roundtrip failed for {family:?}");
        }
    }

    /// Bincode roundtrip — the save-file format. Each family variant must
    /// survive bincode encode/decode byte-for-byte.
    #[test]
    fn attribute_family_bincode_roundtrip() {
        for family in AttributeFamily::ALL {
            let bytes = bincode::serialize(&family).expect("bincode encode");
            let decoded: AttributeFamily = bincode::deserialize(&bytes).expect("bincode decode");
            assert_eq!(decoded, family, "bincode roundtrip failed for {family:?}");
        }
    }
}

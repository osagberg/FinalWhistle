//! `ManagerArchetype` — hand-authored manager personality + tactic reference.
//!
//! Each archetype encodes a manager's tactical preference (via a reference to
//! a `TacticalArchetype` ID) and two Q32 personality scalars that the sim
//! reads at T2-2+ when wiring tactical AI. The fields are stored but not yet
//! consumed by the sim (wired at T2-2 / T2-3 season controller).
//!
//! Hand-authored; NOT LLM-baked. IDs use the `fwh.core:manager.<slug>`
//! dotted form per `Content/RULES.md §2` hand-authored carve-out.
//!
//! `schema_version: 1` from day one per `Content/RULES.md §3` for new types.
//!
//! # T1-7 self-review fix-pass (type-design audit P1)
//!
//! Per T1-12's `SignatureCandidate::try_new` + `RawSignatureCandidate` bridge
//! pattern: this module ships full `try_new` validators + a `RawManagerArchetype`
//! intermediate type + manual `Deserialize` so serde-parsed RON gets validated
//! at load time (rather than silently accepting out-of-range `risk_appetite`
//! values and pushing the failure to first sim consumption). Also ships a
//! `ManagerArchetypeId` newtype with format validator (mirrors `SignatureId`).

use fw_core::Q32;
use serde::{Deserialize, Serialize};

/// Current schema version for `ManagerArchetype`. Bumped on every
/// breaking-shape change; forward-migration only (per `Content/RULES.md §3`).
pub const MANAGER_ARCHETYPE_SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// ManagerArchetypeId — newtype with format validator
// ---------------------------------------------------------------------------

/// Error returned by [`ManagerArchetypeId::try_new`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManagerArchetypeIdError {
    #[error(
        "ManagerArchetypeId {0:?} malformed; expected `<pack-id>:manager.<slug>` \
         where pack-id is dotted-lowercase (≥2 segments, [a-z0-9]+) and \
         slug matches [a-z0-9-]+"
    )]
    Malformed(String),
}

/// Content-pack-qualified manager-archetype ID.
///
/// Format: `<pack-id>:manager.<slug>` matching the same dotted-lowercase
/// shape `SignatureId` uses per `Content/RULES.md §2` hand-authored carve-out.
/// Examples: `fwh.core:manager.pragmatic-defender`, `mod.community.somerset:manager.tinkerer`.
///
/// Construct via [`ManagerArchetypeId::try_new`]; `Deserialize` runs `try_new`
/// post-parse so malformed IDs in RON fixtures fail at load time, not later.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ManagerArchetypeId(String);

impl ManagerArchetypeId {
    /// Validate and wrap `s` as a `ManagerArchetypeId`.
    pub fn try_new(s: impl Into<String>) -> Result<Self, ManagerArchetypeIdError> {
        let s = s.into();
        if Self::is_valid(&s) {
            Ok(Self(s))
        } else {
            Err(ManagerArchetypeIdError::Malformed(s))
        }
    }

    /// Borrow the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn is_valid(s: &str) -> bool {
        let Some((prefix, rest)) = s.split_once(':') else {
            return false;
        };
        // Prefix: dotted-lowercase pack-id (≥2 segments, [a-z0-9]+).
        if prefix.is_empty() {
            return false;
        }
        let mut segment_count = 0usize;
        for segment in prefix.split('.') {
            if segment.is_empty() {
                return false;
            }
            if !segment.chars().all(|c| matches!(c, 'a'..='z' | '0'..='9')) {
                return false;
            }
            segment_count += 1;
        }
        if segment_count < 2 {
            return false;
        }
        // rest: `manager.<slug>`
        let Some(slug) = rest.strip_prefix("manager.") else {
            return false;
        };
        !slug.is_empty()
            && slug
                .chars()
                .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '-'))
    }
}

// Manual Deserialize — runs try_new post-parse, mirroring SignatureId pattern.
impl<'de> Deserialize<'de> for ManagerArchetypeId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ManagerArchetypeId::try_new(s).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for ManagerArchetypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// ManagerArchetype — the main type
// ---------------------------------------------------------------------------

/// Error returned by [`ManagerArchetype::try_new`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManagerArchetypeError {
    #[error("risk_appetite {value:?} out of [Q32::ZERO, Q32::ONE]")]
    RiskAppetiteOutOfRange { value: Q32 },
    #[error("possession_preference {value:?} out of [Q32::ZERO, Q32::ONE]")]
    PossessionPreferenceOutOfRange { value: Q32 },
}

/// A manager archetype — links a tactical system to personality scalars.
///
/// `risk_appetite` and `possession_preference` are stored as `Q32` (no
/// `f32`/`f64`) per `Sim/RULES.md §1`. Both are unit-range [Q32::ZERO, Q32::ONE]
/// scalars (0 = conservative/no-ball, 1 = aggressive/full-possession) and the
/// range is ENFORCED by [`ManagerArchetype::try_new`] (T1-7 self-review fix-pass
/// per type-design audit P1 — pattern mirrors `SignatureCandidate::try_new`).
///
/// Field order is load-bearing for serde determinism — do not reorder.
///
/// Construct via [`ManagerArchetype::try_new`]; `Deserialize` runs `try_new`
/// post-parse via the `RawManagerArchetype` bridge so malformed RON fixtures
/// fail at load time, not at first sim consumption.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagerArchetype {
    /// Schema version. Forward-migrated by the content loader (T2-3+).
    pub schema_version: u32,

    /// Stable content-pack-qualified ID. Format `fwh.core:manager.<slug>`
    /// per `Content/RULES.md §2` hand-authored dotted-form carve-out.
    /// Validated via [`ManagerArchetypeId::try_new`] at deserialize.
    pub id: ManagerArchetypeId,

    /// Display name (used in dev UIs + commentary templates; player-facing).
    pub display_name: String,

    /// Stable ID of the `TacticalArchetype` this manager prefers.
    /// **Format-validated at deserialize via standard String parsing** (no
    /// newtype yet — TacticalArchetype.id is also bare String today; a
    /// dedicated `TacticalArchetypeId` newtype refactor is a future row that
    /// touches the existing TacticalArchetype struct + fixtures + tests).
    /// **Cross-reference validation** (the ID resolves in
    /// `ContentStore::tactical_archetypes`) happens via
    /// `ContentStore::validate_cross_references` at load time + at
    /// `generate_team` call time as a defence-in-depth (T1-7 fix-pass per
    /// silent-failure F4 — prior doc claimed load-time validation but loader
    /// didn't enforce; now it does).
    pub tactical_archetype_id: String,

    /// Risk appetite scalar. `Q32::ZERO` = never risks possession;
    /// `Q32::ONE` = pure risk-taking. Range [Q32::ZERO, Q32::ONE] ENFORCED
    /// by try_new. `0.3 × 2^32 ≈ 1_288_490_189` raw bits.
    pub risk_appetite: Q32,

    /// Possession preference scalar. `Q32::ZERO` = route-one / direct;
    /// `Q32::ONE` = full-possession game. Range [Q32::ZERO, Q32::ONE]
    /// ENFORCED by try_new. `0.4 × 2^32 ≈ 1_717_986_918` raw bits.
    pub possession_preference: Q32,
}

impl ManagerArchetype {
    /// Validate and construct a `ManagerArchetype`.
    ///
    /// Returns `Err(RiskAppetiteOutOfRange)` if `risk_appetite` is outside
    /// `[Q32::ZERO, Q32::ONE]`; same for `possession_preference`. ID format
    /// validation happens at `ManagerArchetypeId::try_new` (separate call).
    pub fn try_new(
        schema_version: u32,
        id: ManagerArchetypeId,
        display_name: String,
        tactical_archetype_id: String,
        risk_appetite: Q32,
        possession_preference: Q32,
    ) -> Result<Self, ManagerArchetypeError> {
        if risk_appetite < Q32::ZERO || risk_appetite > Q32::ONE {
            return Err(ManagerArchetypeError::RiskAppetiteOutOfRange {
                value: risk_appetite,
            });
        }
        if possession_preference < Q32::ZERO || possession_preference > Q32::ONE {
            return Err(ManagerArchetypeError::PossessionPreferenceOutOfRange {
                value: possession_preference,
            });
        }
        Ok(Self {
            schema_version,
            id,
            display_name,
            tactical_archetype_id,
            risk_appetite,
            possession_preference,
        })
    }
}

// `RawManagerArchetype` bridge for serde — derives the trivial structural
// Deserialize, then `TryFrom` runs `try_new` to enforce invariants. Mirrors
// `RawSignatureCandidate` in signature.rs.
//
// `#[serde(rename = "ManagerArchetype")]` so RON's strict struct-name check
// accepts the public type name in fixture files (they're authored as
// `ManagerArchetype(...)`, not `RawManagerArchetype(...)`).
#[derive(Deserialize)]
#[serde(rename = "ManagerArchetype")]
struct RawManagerArchetype {
    schema_version: u32,
    id: ManagerArchetypeId,
    display_name: String,
    tactical_archetype_id: String,
    risk_appetite: Q32,
    possession_preference: Q32,
}

impl TryFrom<RawManagerArchetype> for ManagerArchetype {
    type Error = ManagerArchetypeError;

    fn try_from(raw: RawManagerArchetype) -> Result<Self, Self::Error> {
        ManagerArchetype::try_new(
            raw.schema_version,
            raw.id,
            raw.display_name,
            raw.tactical_archetype_id,
            raw.risk_appetite,
            raw.possession_preference,
        )
    }
}

impl<'de> Deserialize<'de> for ManagerArchetype {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawManagerArchetype::deserialize(deserializer)?;
        ManagerArchetype::try_from(raw).map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_id() -> ManagerArchetypeId {
        ManagerArchetypeId::try_new("fwh.core:manager.pragmatic-defender").expect("valid id")
    }

    fn valid_archetype() -> ManagerArchetype {
        ManagerArchetype::try_new(
            MANAGER_ARCHETYPE_SCHEMA_VERSION,
            valid_id(),
            "Pragmatic Defender".to_string(),
            "fwh.core:archetype.low-block-counter".to_string(),
            Q32::from_raw(1_288_490_189),
            Q32::from_raw(1_717_986_918),
        )
        .expect("valid archetype")
    }

    #[test]
    fn schema_version_is_one() {
        assert_eq!(MANAGER_ARCHETYPE_SCHEMA_VERSION, 1);
    }

    #[test]
    fn try_new_accepts_valid_archetype() {
        let a = valid_archetype();
        assert_eq!(a.display_name, "Pragmatic Defender");
    }

    #[test]
    fn try_new_rejects_risk_appetite_above_one() {
        let err = ManagerArchetype::try_new(
            MANAGER_ARCHETYPE_SCHEMA_VERSION,
            valid_id(),
            "Bad".to_string(),
            "fwh.core:archetype.low-block-counter".to_string(),
            Q32::from_int(5), // 5.0 > 1.0
            Q32::ZERO,
        )
        .expect_err("should reject risk_appetite > 1");
        assert!(matches!(
            err,
            ManagerArchetypeError::RiskAppetiteOutOfRange { .. }
        ));
    }

    #[test]
    fn try_new_rejects_negative_risk_appetite() {
        let err = ManagerArchetype::try_new(
            MANAGER_ARCHETYPE_SCHEMA_VERSION,
            valid_id(),
            "Bad".to_string(),
            "fwh.core:archetype.low-block-counter".to_string(),
            Q32::from_raw(-(1i64 << 32)), // -1.0
            Q32::ZERO,
        )
        .expect_err("should reject risk_appetite < 0");
        assert!(matches!(
            err,
            ManagerArchetypeError::RiskAppetiteOutOfRange { .. }
        ));
    }

    #[test]
    fn try_new_rejects_possession_preference_out_of_range() {
        let err = ManagerArchetype::try_new(
            MANAGER_ARCHETYPE_SCHEMA_VERSION,
            valid_id(),
            "Bad".to_string(),
            "fwh.core:archetype.low-block-counter".to_string(),
            Q32::ZERO,
            Q32::from_int(10), // 10.0 > 1.0
        )
        .expect_err("should reject possession_preference > 1");
        assert!(matches!(
            err,
            ManagerArchetypeError::PossessionPreferenceOutOfRange { .. }
        ));
    }

    // ---- ManagerArchetypeId validator ----

    #[test]
    fn id_try_new_accepts_canonical_form() {
        let id = ManagerArchetypeId::try_new("fwh.core:manager.pragmatic-defender")
            .expect("canonical id");
        assert_eq!(id.as_str(), "fwh.core:manager.pragmatic-defender");
    }

    #[test]
    fn id_try_new_accepts_mod_pack_namespace() {
        // Mod packs use multi-segment dotted prefixes.
        let id =
            ManagerArchetypeId::try_new("mod.community.somerset:manager.tinkerer").expect("mod id");
        assert_eq!(id.as_str(), "mod.community.somerset:manager.tinkerer");
    }

    #[test]
    fn id_try_new_rejects_uppercase() {
        assert!(
            ManagerArchetypeId::try_new("Fwh.core:manager.x").is_err(),
            "should reject uppercase prefix"
        );
    }

    #[test]
    fn id_try_new_rejects_single_segment_prefix() {
        assert!(
            ManagerArchetypeId::try_new("fwh:manager.x").is_err(),
            "should reject single-segment prefix"
        );
    }

    #[test]
    fn id_try_new_rejects_missing_manager_prefix() {
        assert!(
            ManagerArchetypeId::try_new("fwh.core:archetype.low-block-counter").is_err(),
            "should reject non-manager entity-type"
        );
    }

    #[test]
    fn id_try_new_rejects_empty_slug() {
        assert!(
            ManagerArchetypeId::try_new("fwh.core:manager.").is_err(),
            "should reject empty slug"
        );
    }

    // ---- serde round-trip ----

    #[test]
    fn ron_round_trip_valid() {
        let archetype = valid_archetype();
        let ron_text = ron::ser::to_string(&archetype).expect("ron encode");
        let decoded: ManagerArchetype = ron::de::from_str(&ron_text).expect("ron decode");
        assert_eq!(decoded, archetype);
    }

    #[test]
    fn ron_deserialize_rejects_malformed_id() {
        let ron_text = r#"(
            schema_version: 1,
            id: "INVALID_NO_COLON_ID",
            display_name: "Bad",
            tactical_archetype_id: "fwh.core:archetype.low-block-counter",
            risk_appetite: (bits: 1288490189),
            possession_preference: (bits: 1717986918),
        )"#;
        let result: Result<ManagerArchetype, _> = ron::de::from_str(ron_text);
        assert!(result.is_err(), "should reject malformed id at deserialize");
    }

    #[test]
    fn ron_deserialize_rejects_risk_appetite_out_of_range() {
        let ron_text = r#"(
            schema_version: 1,
            id: "fwh.core:manager.bad",
            display_name: "Bad",
            tactical_archetype_id: "fwh.core:archetype.low-block-counter",
            risk_appetite: (bits: 99999999999),
            possession_preference: (bits: 1717986918),
        )"#;
        let result: Result<ManagerArchetype, _> = ron::de::from_str(ron_text);
        assert!(
            result.is_err(),
            "should reject out-of-range risk_appetite at deserialize"
        );
    }

    #[test]
    fn risk_appetite_raw_bits_approx_0_3() {
        // 0.3 * 2^32 = 1_288_490_188.8 → rounded to 1_288_490_189
        let v = Q32::from_raw(1_288_490_189);
        // Convert via f64 for the assertion (bake-time only path — safe here).
        let as_f64 = v.to_bits() as f64 / (1u64 << 32) as f64;
        assert!(
            (as_f64 - 0.3f64).abs() < 1e-9,
            "risk_appetite raw bits should be ≈ 0.3, got {as_f64}"
        );
    }

    #[test]
    fn possession_preference_raw_bits_approx_0_4() {
        let v = Q32::from_raw(1_717_986_918);
        let as_f64 = v.to_bits() as f64 / (1u64 << 32) as f64;
        assert!(
            (as_f64 - 0.4f64).abs() < 1e-9,
            "possession_preference raw bits should be ≈ 0.4, got {as_f64}"
        );
    }
}

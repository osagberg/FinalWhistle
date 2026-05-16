//! Signature schema — type system only (T1-3).
//!
//! A signature is the tuple `(SignatureId, TriggerPredicate, SimBiasSnapshot,
//! PresentationRecipe, CooldownPolicy)` per ADR-0011 §"Mechanical shape".
//!
//! The trigger predicate FUNCTION lives in `fw-match-sim` and is bound to
//! `SignatureId` by the dispatcher at T1-2b-iv. This module defines the
//! DATA shapes only — no predicate functions, no dispatcher, no cooldown
//! state on `MatchState`.
//!
//! ## What lives here
//! - `SignatureId` — newtype with content-pack-qualified format validation.
//! - `RoleFamily` — 8-variant enum; 3 signatures per family = 24 catalogue.
//! - `BiasCategory` — 4-variant enum for stacking policy enforcement.
//! - `SimBiasSnapshot` — 5-field multiplicative bias: shoot/pass/dribble/
//!   press/cover. Collapses ADR-0003 §5's 7 personality-bias surfaces into 5
//!   by merging long_pass + safe_pass → `pass_mul` and dropping `hold_mul`
//!   (hold-position is a fallback that signatures don't typically modify).
//!   T2-1 re-tuning may expand if the collapse proves limiting.
//! - `CooldownPolicy` — `EveryTicks(u32)` default 600 + `PerMatchCount(u8)`.
//! - `StackingPolicy` — `Exclusive { category }` enforces single-active-per-
//!   category; cross-category concurrent firings are allowed.
//! - `SignatureTrigger` — stub enum; only `NoOpStub` for T1-3; T1-2b-iv
//!   expands with real predicate-parameter variants.
//! - `SignaturePresentationRecipe` — string placeholders; T2 fills the
//!   commentary line bank IDs and camera framing enum.
//! - `SignatureDefinition` — the full record written to RON.
//! - `SignatureCandidate` — per-player affinity carry-forward from v1's
//!   `IdentityPacket.SignatureCandidates`.

use fw_core::Q32;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// SignatureId — content-pack-qualified newtype
// ---------------------------------------------------------------------------

/// Error returned when a string is rejected as a `SignatureId`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SignatureIdError {
    #[error(
        "SignatureId {0:?} malformed; expected `<pack-id>:signature.<slug>` \
         where pack-id is dotted-lowercase (≥2 segments, [a-z0-9]+) and \
         slug matches [a-z0-9-]+"
    )]
    Malformed(String),
}

/// Content-pack-qualified signature ID.
///
/// Format: `<pack-id>:signature.<slug>` where:
/// - `<pack-id>` is a dotted-lowercase pack identifier with at least two
///   dot-separated segments, each matching `[a-z0-9]+`. Examples:
///   `fwh.core`, `fwh.core.v2`, `fwh.fantasy.elvish`,
///   `mod.community.somerset`. Uppercase, underscores, hyphens, leading /
///   trailing / doubled dots are all rejected.
/// - `<slug>` is non-empty and matches `[a-z0-9-]+`.
///
/// Per `Content/RULES.md` §2 (hand-authored dotted-form carve-out). Mod packs
/// may define their own signature IDs using their pack-id namespace.
///
/// Construct via [`SignatureId::try_new`]. The manual `Deserialize` impl calls
/// `try_new` post-parse so malformed IDs are rejected at load time.
///
/// RON wire form: `SignatureId("fwh.core:signature.slug")` (newtype struct).
/// The manual impl preserves this form via `deserialize_newtype_struct`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct SignatureId(String);

impl<'de> serde::Deserialize<'de> for SignatureId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SignatureIdVisitor;

        impl<'de> serde::de::Visitor<'de> for SignatureIdVisitor {
            type Value = SignatureId;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(
                    "a content-pack-qualified signature ID string in the form \
                     `<pack-id>:signature.<slug>`",
                )
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<SignatureId, E> {
                SignatureId::try_new(v).map_err(E::custom)
            }

            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<SignatureId, E> {
                SignatureId::try_new(v).map_err(E::custom)
            }

            // RON dispatches newtype structs (`SignatureId("...")`) via
            // `visit_newtype_struct`. Deserialize the inner value as a `String`
            // then validate via `try_new`.
            fn visit_newtype_struct<A>(self, deserializer: A) -> Result<SignatureId, A::Error>
            where
                A: serde::Deserializer<'de>,
            {
                let s = String::deserialize(deserializer)?;
                SignatureId::try_new(s).map_err(serde::de::Error::custom)
            }
        }

        // RON delivers `SignatureId("...")` as a newtype struct dispatch;
        // other formats (JSON, Bincode) deliver the inner string directly.
        deserializer.deserialize_newtype_struct("SignatureId", SignatureIdVisitor)
    }
}

impl SignatureId {
    /// Validate and wrap `s` as a `SignatureId`.
    ///
    /// Accepts `<pack-id>:signature.<slug>` where `<pack-id>` is a dotted-
    /// lowercase identifier with at least two segments (each `[a-z0-9]+`)
    /// and `<slug>` is `[a-z0-9-]+`. See the type doc for full rules.
    /// Returns `Err(SignatureIdError::Malformed)` for anything else.
    pub fn try_new(s: impl Into<String>) -> Result<Self, SignatureIdError> {
        let s = s.into();
        if Self::is_valid(&s) {
            Ok(Self(s))
        } else {
            Err(SignatureIdError::Malformed(s))
        }
    }

    /// Borrow the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn is_valid(s: &str) -> bool {
        let Some((prefix, rest)) = s.split_once(':') else {
            return false;
        };
        // Prefix: dotted-lowercase pack-id per Content/RULES.md §2.
        //   Valid:   `fwh.core`, `fwh.core.v2`, `fwh.fantasy.elvish`,
        //            `mod.community.somerset`
        //   Invalid: uppercase, underscores, hyphens, whitespace, leading /
        //            trailing / doubled dots, single-segment (e.g. `fwh`).
        if prefix.is_empty() {
            return false;
        }
        let mut segment_count = 0usize;
        for segment in prefix.split('.') {
            if segment.is_empty() {
                // Leading, trailing, or consecutive dots.
                return false;
            }
            if !segment.chars().all(|c| matches!(c, 'a'..='z' | '0'..='9')) {
                return false;
            }
            segment_count += 1;
        }
        if segment_count < 2 {
            // Pack-IDs are at minimum two segments: `vendor.pack`.
            return false;
        }
        // rest: `signature.<slug>`
        let Some(slug) = rest.strip_prefix("signature.") else {
            return false;
        };
        // slug: non-empty, only [a-z0-9-]
        !slug.is_empty()
            && slug
                .chars()
                .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '-'))
    }
}

// ---------------------------------------------------------------------------
// RoleFamily — 8 role families per ADR-0011 §"Catalogue size"
// ---------------------------------------------------------------------------

/// Eight role families; 3 signatures per family = 24-signature catalogue.
///
/// Variant discriminants are stable for canonical encoding at T1-2b-iv when
/// signatures land in `MatchState`. Do NOT reorder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum RoleFamily {
    Goalkeeper = 0,
    CentreBack = 1,
    FullBack = 2,
    DefensiveMidfielder = 3,
    CentralMidfielder = 4,
    AttackingMidfielder = 5,
    Winger = 6,
    Striker = 7,
}

// ---------------------------------------------------------------------------
// BiasCategory — 4 categories per ADR-0011 §"Stacking policy"
// ---------------------------------------------------------------------------

/// Stacking-policy category. Same-category signatures cannot be in flight
/// simultaneously; cross-category concurrent firings are allowed because they
/// bias non-overlapping utility-scoring lanes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum BiasCategory {
    Attacking = 0,
    Defensive = 1,
    BuildUp = 2,
    SetPiece = 3,
}

// ---------------------------------------------------------------------------
// SimBiasSnapshot — multiplicative utility biases
// ---------------------------------------------------------------------------

/// Multiplicative bias bumps applied to BT utility scoring while the
/// signature is in flight. Composes multiplicatively with ADR-0003 §5
/// personality biases.
///
/// Q32 multipliers: > 1.0 boosts that decision, < 1.0 suppresses it,
/// 1.0 is no-op. All fields default to `Q32::ONE` in `SimBiasSnapshot::NO_OP`.
///
/// ## Field count decision (vs ADR-0003 §5's 7 surfaces)
///
/// ADR-0003 §5 tracks 7 personality-bias surfaces:
/// shoot / long_pass / safe_pass / dribble / press / cover / hold.
/// This struct uses 5 by merging long_pass + safe_pass → `pass_mul`
/// and dropping `hold_mul` (hold-position is a fallback action that
/// signatures don't typically modify — it's the "do nothing well" action
/// and boosting it competes with positive decisions). If T2-1 tuning finds
/// the merge ambiguous, expand to 7 fields without a canonical-hash change
/// (T1-3 snapshots don't encode `SimBiasSnapshot` into `MatchState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimBiasSnapshot {
    /// Multiplier on shoot utility. > 1.0 = more likely to shoot.
    pub shoot_mul: Q32,
    /// Multiplier on pass utility (merges long_pass + safe_pass).
    pub pass_mul: Q32,
    /// Multiplier on dribble utility.
    pub dribble_mul: Q32,
    /// Multiplier on press utility.
    pub press_mul: Q32,
    /// Multiplier on cover/marking utility.
    pub cover_mul: Q32,
}

impl SimBiasSnapshot {
    /// No-op snapshot — all multipliers are 1.0. Used by the no-op stub.
    pub const NO_OP: Self = Self {
        shoot_mul: Q32::ONE,
        pass_mul: Q32::ONE,
        dribble_mul: Q32::ONE,
        press_mul: Q32::ONE,
        cover_mul: Q32::ONE,
    };
}

// ---------------------------------------------------------------------------
// CooldownPolicy — per ADR-0011 §"Cooldown"
// ---------------------------------------------------------------------------

/// Cooldown policy per ADR-0011 §"Cooldown".
///
/// Cooldown state lives in `MatchState.signature_cooldowns` (T1-2b-iv schema
/// bump). Default: 600-tick cooldown = 10 s at 60 Hz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CooldownPolicy {
    /// Per-player-per-signature cooldown: fires no more than once per
    /// `ticks` game ticks. Default 600 (10 s at 60 Hz).
    EveryTicks(u32),
    /// Match-rare signature: fires AT MOST `n` times per match per player.
    PerMatchCount(u8),
}

impl Default for CooldownPolicy {
    fn default() -> Self {
        Self::EveryTicks(600)
    }
}

// ---------------------------------------------------------------------------
// StackingPolicy — per ADR-0011 §"Stacking policy"
// ---------------------------------------------------------------------------

/// Stacking policy: two signatures CAN co-fire if they belong to DIFFERENT
/// `BiasCategory` values. Same-category concurrent firings are forbidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StackingPolicy {
    /// This signature is exclusive with other signatures in the same
    /// `BiasCategory`. Cross-category concurrent firings are allowed.
    Exclusive { category: BiasCategory },
}

// ---------------------------------------------------------------------------
// SignatureTrigger — stub enum; T1-2b-iv expands
// ---------------------------------------------------------------------------

/// Trigger parameters per ADR-0011 §"Mechanical shape".
///
/// The actual predicate FUNCTION over `MatchState + player + ball` lives in
/// `fw-match-sim` and is bound to `SignatureId` by the dispatcher at T1-2b-iv.
/// This enum carries ONLY the parameters the predicate function consumes.
///
/// T1-3 ships only `NoOpStub`. Real trigger variants (e.g.
/// `LongRangeStrike { min_distance_m, max_pressure }`) land at T1-2b-iv when
/// the matching Rust predicate functions are written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureTrigger {
    /// Predicate always returns false. Used by the no-op fixture.
    NoOpStub,
}

// ---------------------------------------------------------------------------
// SignaturePresentationRecipe — placeholder; T2 fills the commentary bank
// ---------------------------------------------------------------------------

/// Stub presentation recipe per ADR-0011 §"Mod compatibility".
///
/// T2-4 replaces these string placeholders with:
/// - `commentary_line_bank_id`: a content-pack-qualified ID into the
///   commentary-line-bank fixture.
/// - `camera_framing_hint`: a `CameraFraming` enum variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignaturePresentationRecipe {
    /// Commentary line bank ID (T2 placeholder; currently a free string).
    pub commentary_line_bank_id: String,
    /// Camera framing hint (T2 placeholder; currently a free string).
    pub camera_framing_hint: String,
    /// Schema version — bumped when the real T2 fields replace the stubs.
    pub schema_version: u32,
}

// ---------------------------------------------------------------------------
// SignatureDefinition — the full record per ADR-0011 §"Mechanical shape"
// ---------------------------------------------------------------------------

/// Full signature definition. Written to `content/sources/signatures/<slug>.ron`.
///
/// Field order is stable for serde determinism — do not reorder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureDefinition {
    /// Schema version for forward-migration. `1` at T1-3.
    pub schema_version: u32,
    /// Content-pack-qualified ID. Format: `fwh.core:signature.<slug>`.
    pub id: SignatureId,
    /// Human-readable name (dev UI + commentary template key).
    pub display_name: String,
    /// Role family this signature belongs to (1 of 8 per ADR-0011).
    pub role_family: RoleFamily,
    /// Trigger parameter set. Real predicate bound at T1-2b-iv.
    pub trigger: SignatureTrigger,
    /// Utility multipliers applied while this signature is in flight.
    pub bias_snapshot: SimBiasSnapshot,
    /// Commentary + camera presentation (stub at T1-3; T2 fills).
    pub presentation: SignaturePresentationRecipe,
    /// Cooldown policy. Default: `EveryTicks(600)`.
    pub cooldown: CooldownPolicy,
    /// Stacking exclusivity category.
    pub stacking: StackingPolicy,
}

// ---------------------------------------------------------------------------
// SignatureCandidate — per-player affinity, carry-forward from v1
// ---------------------------------------------------------------------------

/// Error returned when `SignatureCandidate::try_new` rejects its arguments.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SignatureCandidateError {
    #[error("affinity {affinity:?} out of [0, 1]")]
    AffinityOutOfRange { affinity: Q32 },
}

/// Per-player signature affinity. Carry-forward from FW v1's
/// `IdentityPacket.SignatureCandidates`.
///
/// Lives on `PlayerTemplate.signature_candidates: Vec<SignatureCandidate>`.
/// A player typically has 0–3 candidates. Affinity is a hand-authored
/// gene-style attribute (procedurally derived at T2-4); it is NOT in the
/// 55-field `PlayerAttributes` model from ADR-0002.
///
/// The manual `Deserialize` impl uses a private `RawSignatureCandidate` bridge
/// type + `TryFrom` to run `try_new` post-parse. This is cleaner than a
/// `Visitor` impl for a two-field struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SignatureCandidate {
    /// Stable content-pack-qualified signature ID.
    pub signature_id: SignatureId,
    /// Affinity in `[0, 1]` Q32. Used as the softmax input when multiple
    /// candidates are eligible at the same tick (T1-2b-iv dispatcher).
    pub affinity: Q32,
}

/// Private bridge type for `SignatureCandidate` deserialization.
///
/// `RawSignatureCandidate` derives `Deserialize` with no validation logic.
/// Once deserialized, `TryFrom<RawSignatureCandidate> for SignatureCandidate`
/// runs `try_new` to enforce the `[0, 1]` affinity constraint.
#[derive(serde::Deserialize)]
struct RawSignatureCandidate {
    signature_id: SignatureId,
    affinity: Q32,
}

impl TryFrom<RawSignatureCandidate> for SignatureCandidate {
    type Error = SignatureCandidateError;

    fn try_from(raw: RawSignatureCandidate) -> Result<Self, Self::Error> {
        SignatureCandidate::try_new(raw.signature_id, raw.affinity)
    }
}

impl<'de> serde::Deserialize<'de> for SignatureCandidate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawSignatureCandidate::deserialize(deserializer)?;
        SignatureCandidate::try_from(raw).map_err(serde::de::Error::custom)
    }
}

impl SignatureCandidate {
    /// Validate and construct a `SignatureCandidate`.
    ///
    /// Returns `Err(SignatureCandidateError::AffinityOutOfRange)` if `affinity`
    /// is outside `[0, 1]` (i.e. `< Q32::ZERO` or `> Q32::ONE`).
    pub fn try_new(
        signature_id: SignatureId,
        affinity: Q32,
    ) -> Result<Self, SignatureCandidateError> {
        if affinity < Q32::ZERO || affinity > Q32::ONE {
            return Err(SignatureCandidateError::AffinityOutOfRange { affinity });
        }
        Ok(Self {
            signature_id,
            affinity,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_core::Q32;

    // ---- SignatureId validation (P1-1 tightened validator) ----

    #[test]
    fn valid_signature_ids_accepted() {
        // Canonical fwh.core forms
        assert!(SignatureId::try_new("fwh.core:signature.no-op-stub").is_ok());
        assert!(SignatureId::try_new("fwh.core:signature.long-range-strike").is_ok());
        // Version suffix
        assert!(SignatureId::try_new("fwh.core.v2:signature.body-shield").is_ok());
        // Multi-segment mod-pack prefix (Content/RULES.md §2 mod carve-out)
        assert!(SignatureId::try_new("fwh.fantasy.elvish:signature.ghost-dribble").is_ok());
        assert!(SignatureId::try_new("mod.community.somerset:signature.direct-run").is_ok());
        // Numeric-only segment is fine (e.g. versioned packs)
        assert!(SignatureId::try_new("fwh.core.2:signature.test").is_ok());
    }

    #[test]
    fn invalid_signature_ids_prefix_rejected() {
        // Single-segment prefix — need at least vendor.pack
        assert!(SignatureId::try_new("fwh:signature.foo").is_err());
        // Uppercase in prefix segment
        assert!(SignatureId::try_new("Fwh.Core:signature.x").is_err());
        // Underscore in prefix segment
        assert!(SignatureId::try_new("fwh_core.pack:signature.x").is_err());
        // Hyphen in prefix segment (slugs allow hyphens; pack segments do not)
        assert!(SignatureId::try_new("fwh-core.pack:signature.x").is_err());
        // Leading dot → empty first segment
        assert!(SignatureId::try_new(".fwh.core:signature.x").is_err());
        // Double dot → empty middle segment
        assert!(SignatureId::try_new("fwh..core:signature.x").is_err());
        // Trailing dot → empty last segment
        assert!(SignatureId::try_new("fwh.core.:signature.x").is_err());
        // Missing colon entirely
        assert!(SignatureId::try_new("fwh.core.signature.foo").is_err());
    }

    #[test]
    fn invalid_signature_ids_namespace_and_slug_rejected() {
        // Wrong namespace (not "signature")
        assert!(SignatureId::try_new("fwh.core:player.foo").is_err());
        // Uppercase in namespace
        assert!(SignatureId::try_new("fwh.core:Signature.x").is_err());
        // Uppercase in slug
        assert!(SignatureId::try_new("fwh.core:signature.FooBar").is_err());
        assert!(SignatureId::try_new("fwh.core:signature.Hello").is_err());
        // Empty slug
        assert!(SignatureId::try_new("fwh.core:signature.").is_err());
        // Whitespace in slug
        assert!(SignatureId::try_new("fwh.core:signature.foo bar").is_err());
        // Underscore in slug
        assert!(SignatureId::try_new("fwh.core:signature.foo_bar").is_err());
    }

    #[test]
    fn signature_id_as_str() {
        let id = SignatureId::try_new("fwh.core:signature.no-op-stub").unwrap();
        assert_eq!(id.as_str(), "fwh.core:signature.no-op-stub");
    }

    // ---- SimBiasSnapshot::NO_OP ----

    #[test]
    fn no_op_snapshot_all_ones() {
        let snap = SimBiasSnapshot::NO_OP;
        assert_eq!(snap.shoot_mul, Q32::ONE);
        assert_eq!(snap.pass_mul, Q32::ONE);
        assert_eq!(snap.dribble_mul, Q32::ONE);
        assert_eq!(snap.press_mul, Q32::ONE);
        assert_eq!(snap.cover_mul, Q32::ONE);
    }

    // ---- CooldownPolicy default ----

    #[test]
    fn cooldown_default_is_600_ticks() {
        assert_eq!(CooldownPolicy::default(), CooldownPolicy::EveryTicks(600));
    }

    // ---- SignatureDefinition RON round-trip ----

    fn sample_definition() -> SignatureDefinition {
        SignatureDefinition {
            schema_version: 1,
            id: SignatureId::try_new("fwh.core:signature.no-op-stub").unwrap(),
            display_name: "No-Op Stub Signature".to_string(),
            role_family: RoleFamily::CentralMidfielder,
            trigger: SignatureTrigger::NoOpStub,
            bias_snapshot: SimBiasSnapshot::NO_OP,
            presentation: SignaturePresentationRecipe {
                commentary_line_bank_id: "placeholder".to_string(),
                camera_framing_hint: "default".to_string(),
                schema_version: 1,
            },
            cooldown: CooldownPolicy::EveryTicks(600),
            stacking: StackingPolicy::Exclusive {
                category: BiasCategory::BuildUp,
            },
        }
    }

    #[test]
    fn signature_definition_ron_round_trip() {
        let def = sample_definition();
        let ron_text = ron::ser::to_string(&def).expect("ron encode");
        let decoded: SignatureDefinition = ron::de::from_str(&ron_text).expect("ron decode");
        assert_eq!(decoded, def);
    }

    #[test]
    fn signature_definition_ron_pretty_round_trip() {
        let def = sample_definition();
        let ron_text = ron::ser::to_string_pretty(&def, ron::ser::PrettyConfig::default())
            .expect("ron pretty");
        let decoded: SignatureDefinition = ron::de::from_str(&ron_text).expect("ron decode");
        assert_eq!(decoded, def);
    }

    // ---- SignatureCandidate RON round-trip ----

    #[test]
    fn signature_candidate_ron_round_trip() {
        let cand = SignatureCandidate {
            signature_id: SignatureId::try_new("fwh.core:signature.no-op-stub").unwrap(),
            affinity: Q32::from_raw(3_006_477_107), // ~0.70
        };
        let ron_text = ron::ser::to_string(&cand).expect("ron encode");
        let decoded: SignatureCandidate = ron::de::from_str(&ron_text).expect("ron decode");
        assert_eq!(decoded, cand);
    }

    // ---- RoleFamily all variants present ----

    #[test]
    fn role_family_8_variants() {
        let variants = [
            RoleFamily::Goalkeeper,
            RoleFamily::CentreBack,
            RoleFamily::FullBack,
            RoleFamily::DefensiveMidfielder,
            RoleFamily::CentralMidfielder,
            RoleFamily::AttackingMidfielder,
            RoleFamily::Winger,
            RoleFamily::Striker,
        ];
        assert_eq!(variants.len(), 8);
        // Ensure discriminants are as expected (stable for canonical encoding at T1-2b-iv).
        for (i, v) in variants.iter().enumerate() {
            assert_eq!(*v as u8, i as u8, "RoleFamily::{v:?} discriminant wrong");
        }
    }

    // ---- BiasCategory all variants present ----

    #[test]
    fn bias_category_4_variants() {
        let variants = [
            BiasCategory::Attacking,
            BiasCategory::Defensive,
            BiasCategory::BuildUp,
            BiasCategory::SetPiece,
        ];
        assert_eq!(variants.len(), 4);
        for (i, v) in variants.iter().enumerate() {
            assert_eq!(*v as u8, i as u8, "BiasCategory::{v:?} discriminant wrong");
        }
    }

    // ---- CooldownPolicy both variants round-trip ----

    #[test]
    fn cooldown_every_ticks_round_trip() {
        let p = CooldownPolicy::EveryTicks(600);
        let ron = ron::ser::to_string(&p).unwrap();
        let d: CooldownPolicy = ron::de::from_str(&ron).unwrap();
        assert_eq!(d, p);
    }

    #[test]
    fn cooldown_per_match_count_round_trip() {
        let p = CooldownPolicy::PerMatchCount(2);
        let ron = ron::ser::to_string(&p).unwrap();
        let d: CooldownPolicy = ron::de::from_str(&ron).unwrap();
        assert_eq!(d, p);
    }

    // ---- StackingPolicy round-trip ----

    #[test]
    fn stacking_policy_exclusive_round_trip() {
        let sp = StackingPolicy::Exclusive {
            category: BiasCategory::Attacking,
        };
        let ron = ron::ser::to_string(&sp).unwrap();
        let d: StackingPolicy = ron::de::from_str(&ron).unwrap();
        assert_eq!(d, sp);
    }

    // ---- SignatureCandidate::try_new (P1-2) ----

    fn stub_id() -> SignatureId {
        SignatureId::try_new("fwh.core:signature.no-op-stub").unwrap()
    }

    #[test]
    fn try_new_affinity_zero_succeeds() {
        assert!(SignatureCandidate::try_new(stub_id(), Q32::ZERO).is_ok());
    }

    #[test]
    fn try_new_affinity_half_succeeds() {
        // 0.5 in Q32 = 2^31 = 2_147_483_648 raw bits
        let half = Q32::from_raw(2_147_483_648);
        assert!(SignatureCandidate::try_new(stub_id(), half).is_ok());
    }

    #[test]
    fn try_new_affinity_one_succeeds() {
        assert!(SignatureCandidate::try_new(stub_id(), Q32::ONE).is_ok());
    }

    #[test]
    fn try_new_affinity_negative_fails() {
        // -0.01 in Q32: -0.01 * 2^32 ≈ -42_949_673 raw bits
        let neg = Q32::from_raw(-42_949_673_i64);
        let result = SignatureCandidate::try_new(stub_id(), neg);
        assert!(
            matches!(
                result,
                Err(SignatureCandidateError::AffinityOutOfRange { .. })
            ),
            "expected AffinityOutOfRange, got {result:?}"
        );
    }

    #[test]
    fn try_new_affinity_above_one_fails() {
        // 1.01 in Q32: 1.01 * 2^32 ≈ 4_337_916_969 raw bits (> i64::MAX/2 is fine,
        // but Q32 is FixedI64<U32> so we need a signed i64 — 1.01 > 1.0 is positive).
        // Q32::ONE bits = 4_294_967_296; add ~42_949_673 for 0.01.
        let above_one = Q32::from_raw(4_337_916_969_i64);
        let result = SignatureCandidate::try_new(stub_id(), above_one);
        assert!(
            matches!(
                result,
                Err(SignatureCandidateError::AffinityOutOfRange { .. })
            ),
            "expected AffinityOutOfRange, got {result:?}"
        );
    }
}

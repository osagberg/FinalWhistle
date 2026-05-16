//! Role-affinity tables — per-role weight catalogues for CA derivation.
//!
//! Per ADR-0002 §"Choices" item 6: role-affinity weights live in
//! content-pack RON, not hardcoded. A CB role weights tackling, marking,
//! strength, jumping_reach, and positioning heavily; an AM role weights
//! vision, passing, dribbling, off_the_ball, and technique heavily.
//! Weights sum to 1.0 per role; the schema enforces this via the
//! basis-point sum invariant (`sum_bps == 10_000`).
//!
//! Each weight is stored as a `u16` in basis points (`0..=10_000` maps
//! `0.0..=1.0`) for the same reason `TacticalArchetype.buildup_speed_factor_bps`
//! is integer-only (Codex Imp #3): the CA-derivation path eventually feeds
//! canonical state, and the content layer stays float-free.
//!
//! `RoleId` is a `#[serde(transparent)]` newtype over `String` so a
//! misspelling at the call site is a compile-time error rather than a
//! silent `BTreeMap::get` miss. The role-id catalogue is open — mods may
//! add custom roles via their own affinity tables — but the type itself
//! is non-fungible with bare `String`.
//!
//! `RoleWeights::weights_bps` keys are validated against
//! `fw_core::VISIBLE_ATTRIBUTE_NAMES` so misspellings like `"finsihing"`
//! AND incorrect uses of hidden fields like `"injury_proneness"` (which
//! the BT runner reads but does NOT include in CA derivation per ADR-0002
//! §"Choices" item 6) both surface as load-time errors in
//! `unknown_attribute_keys`. Codex audit P1 (2026-05-13): the prior
//! `KNOWN_ATTRIBUTE_NAMES` set silently accepted hidden fields,
//! contradicting the ADR.

use fw_core::VISIBLE_ATTRIBUTE_NAMES;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Current schema version for `RoleAffinityTable`. Bumped at every
/// breaking-shape change; forward-migration only.
///
/// **Marker-only at T1-1.** Same status as
/// `PLAYER_TEMPLATE_SCHEMA_VERSION` — content packs commit
/// `schema_version: 1` so the future `ContentStore::load_baked` loader
/// (Tranche 6) can gate on it; no enforcement at the type level yet.
pub const ROLE_AFFINITY_SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// RoleId — typed handle for a role-string
// ---------------------------------------------------------------------------

/// A typed role identifier (e.g. `"GK"`, `"CB"`, `"AM"`, `"RWB"`).
///
/// RON sees a bare string (e.g. `"GK"`); the manual `Deserialize` impl
/// calls `try_new` post-parse so whitespace-only or empty strings are
/// rejected at load time rather than silently stored.
///
/// Serialize remains transparent — the wire form is a bare string in both
/// RON and JSON.
///
/// The role-id catalogue is open — mods may ship custom roles via their
/// own `RoleAffinityTable`. Validation lives at lookup time
/// (`RoleAffinityTable::get`); an unrecognized role surfaces as a
/// load-time FW-VAL error, not a silent zero-fallback.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct RoleId(String);

impl<'de> serde::Deserialize<'de> for RoleId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        RoleId::try_new(s).map_err(serde::de::Error::custom)
    }
}

/// Error returned by `RoleId::try_new`. Codex audit P2 (2026-05-13):
/// `debug_assert!` on empty input meant release builds silently accepted
/// `RoleId("")` — invalid in any role table lookup but allowed at
/// construction time. `try_new` enforces non-emptiness in all build
/// modes; `new` panics with a clear message (test/internal use only).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RoleIdError {
    /// Empty string is never a valid role identifier.
    #[error("RoleId cannot be empty")]
    Empty,
    /// Whitespace-only or whitespace-bracketed (`" GK "`) string —
    /// silent typo class in hand-authored RON. Reject explicitly.
    #[error("RoleId contains leading/trailing whitespace: {input:?}")]
    Whitespace { input: String },
}

impl RoleId {
    /// Construct a `RoleId`. Panics on invalid input. Use `try_new` for
    /// content-load-time validation; `new` is the convenience form for
    /// in-source construction (tests, sample fixtures).
    ///
    /// Codex audit P2 (2026-05-13): previously this used `debug_assert!`,
    /// which made release builds silently accept empty strings. Promoted
    /// to a real panic + parallel `try_new` for non-panicking callers.
    #[track_caller]
    pub fn new(s: impl Into<String>) -> Self {
        match Self::try_new(s) {
            Ok(id) => id,
            Err(e) => panic!("invalid RoleId: {e}"),
        }
    }

    /// Fallible constructor — preferred at content-load time. Returns
    /// `Err(RoleIdError::*)` on validation failure.
    pub fn try_new(s: impl Into<String>) -> Result<Self, RoleIdError> {
        let s = s.into();
        if s.is_empty() {
            return Err(RoleIdError::Empty);
        }
        if s.trim() != s {
            return Err(RoleIdError::Whitespace { input: s });
        }
        Ok(Self(s))
    }

    /// Borrow the underlying string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RoleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// T1-12 fix-pass per type-design audit P2-1: `From<&str>` + `From<String>`
// previously delegated to `RoleId::new` which panics on invalid input. That
// gave non-test callers (e.g. future Tauri DTO code) an infallible-looking
// surface that silently skipped the validation the new manual `Deserialize`
// impl was added to enforce. Removed — callers MUST use `RoleId::new`
// (panicking, for in-source fixture construction) or `RoleId::try_new`
// (fallible, for content-load-time validation). No production caller relied
// on these `From` impls (verified by grep over the workspace).

// ---------------------------------------------------------------------------
// RoleWeights — per-role attribute weight bag
// ---------------------------------------------------------------------------

/// Per-role weight assignment in basis points.
///
/// Every attribute that contributes to this role's CA-derivation
/// weighted sum is keyed by a stable attribute name (matching the field
/// name in `fw-core::player_attributes` — `"finishing"`, `"passing"`,
/// `"tackling"`, etc.). Unweighted attributes are omitted. The sum of
/// all values `MUST` equal `10_000` (1.0 in basis points). Misspelled
/// keys (`"finsihing"`) `MUST` be caught by
/// `RoleWeights::unknown_attribute_keys` against
/// `fw_core::KNOWN_ATTRIBUTE_NAMES`.
///
/// `BTreeMap` for deterministic iteration order (per `Sim/RULES.md` §2 —
/// `HashMap` banned in canonical-state-emitting paths; CA derivation
/// feeds canonical state).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleWeights {
    /// Attribute-name → weight in basis points. Must sum to `10_000`.
    /// Keys must be members of `fw_core::KNOWN_ATTRIBUTE_NAMES`.
    pub weights_bps: BTreeMap<String, u16>,
}

impl RoleWeights {
    /// Total of all weights, in basis points. Validation invariant:
    /// `sum_bps() == 10_000` for any well-formed role.
    #[must_use]
    pub fn sum_bps(&self) -> u32 {
        self.weights_bps.values().map(|&w| u32::from(w)).sum()
    }

    /// Check the sum-to-10_000 invariant.
    #[must_use]
    pub fn is_normalized(&self) -> bool {
        self.sum_bps() == 10_000
    }

    /// Return every weight-key in this role that is NOT a member of
    /// `fw_core::VISIBLE_ATTRIBUTE_NAMES`. Empty iff every key is a real
    /// visible attribute; non-empty iff there's a typo (`"finsihing"`),
    /// a stale rename, OR an incorrect use of a hidden field
    /// (`"injury_proneness"` is real but hidden-only — not a valid
    /// CA-weight key per ADR-0002 §"Choices" item 6).
    ///
    /// Collect-all rather than first-only so FW-VAL can surface every
    /// misspelling + every hidden-field misuse in one pass.
    #[must_use]
    pub fn unknown_attribute_keys(&self) -> Vec<&str> {
        self.weights_bps
            .keys()
            .map(String::as_str)
            .filter(|k| !VISIBLE_ATTRIBUTE_NAMES.contains(k))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// RoleAffinityTable — the catalogue
// ---------------------------------------------------------------------------

/// The catalogue of role weights — one entry per role-id.
///
/// Loaded from `content/sources/role-affinities/<pack>.ron`. Mods may
/// overlay or extend; load order is lexicographic per
/// `Content/RULES.md` §6.
///
/// `BTreeMap` keyed by `RoleId` ensures deterministic CA derivation when
/// the BT runner iterates roles to find affinity matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleAffinityTable {
    /// Schema version of this record. Forward-migrated by the content
    /// loader (T2-3+).
    pub schema_version: u32,
    /// Stable content-pack-qualified ID (`fwh.core:role-affinities.default`).
    pub id: String,
    /// Role-id → weights. Role-ids match `PlayerTemplate::preferred_role`.
    pub roles: BTreeMap<RoleId, RoleWeights>,
}

impl RoleAffinityTable {
    /// Return every role whose weights fail the sum-to-10_000 invariant.
    /// Empty iff the table is well-formed. Collect-all so FW-VAL can
    /// surface every bad entry in one pass.
    #[must_use]
    pub fn invalid_roles(&self) -> Vec<(&RoleId, u32)> {
        self.roles
            .iter()
            .filter_map(|(id, w)| {
                let sum = w.sum_bps();
                if sum == 10_000 { None } else { Some((id, sum)) }
            })
            .collect()
    }

    /// Return every (role-id, unknown-attribute-key) pair across the
    /// table. Empty iff every role weights only real `PlayerAttributes`
    /// fields. Misspellings + stale renames surface here.
    #[must_use]
    pub fn unknown_attribute_keys(&self) -> Vec<(&RoleId, &str)> {
        let mut out = Vec::new();
        for (id, weights) in &self.roles {
            for bad in weights.unknown_attribute_keys() {
                out.push((id, bad));
            }
        }
        out
    }

    /// Look up weights for a role-id. `None` means the role is absent —
    /// FW-VAL should surface that as a `UnknownRole` error rather than
    /// silently zeroing the affinity.
    #[must_use]
    pub fn get(&self, role_id: &RoleId) -> Option<&RoleWeights> {
        self.roles.get(role_id)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_gk_weights() -> RoleWeights {
        let mut weights_bps = BTreeMap::new();
        weights_bps.insert("handling".to_string(), 1_800);
        weights_bps.insert("reflexes".to_string(), 1_800);
        weights_bps.insert("one_on_ones".to_string(), 1_200);
        weights_bps.insert("aerial_reach".to_string(), 1_000);
        weights_bps.insert("command_of_area".to_string(), 1_000);
        weights_bps.insert("kicking".to_string(), 800);
        weights_bps.insert("positioning".to_string(), 800);
        weights_bps.insert("concentration".to_string(), 600);
        weights_bps.insert("agility".to_string(), 600);
        weights_bps.insert("jumping_reach".to_string(), 400);
        RoleWeights { weights_bps }
    }

    fn sample_am_weights() -> RoleWeights {
        let mut weights_bps = BTreeMap::new();
        weights_bps.insert("vision".to_string(), 1_500);
        weights_bps.insert("passing".to_string(), 1_500);
        weights_bps.insert("dribbling".to_string(), 1_200);
        weights_bps.insert("off_the_ball".to_string(), 1_000);
        weights_bps.insert("technique".to_string(), 1_000);
        weights_bps.insert("first_touch".to_string(), 800);
        weights_bps.insert("flair".to_string(), 800);
        weights_bps.insert("decisions".to_string(), 800);
        weights_bps.insert("composure".to_string(), 700);
        weights_bps.insert("anticipation".to_string(), 700);
        RoleWeights { weights_bps }
    }

    #[test]
    fn gk_weights_sum_to_10_000() {
        assert_eq!(sample_gk_weights().sum_bps(), 10_000);
        assert!(sample_gk_weights().is_normalized());
    }

    #[test]
    fn am_weights_sum_to_10_000() {
        assert_eq!(sample_am_weights().sum_bps(), 10_000);
        assert!(sample_am_weights().is_normalized());
    }

    #[test]
    fn sample_role_weights_all_keys_known() {
        // Sanity: every key in our sample weights must resolve to a real
        // attribute name. If KNOWN_ATTRIBUTE_NAMES drifts away from
        // PlayerAttributes field names, this catches it.
        assert!(sample_gk_weights().unknown_attribute_keys().is_empty());
        assert!(sample_am_weights().unknown_attribute_keys().is_empty());
    }

    #[test]
    fn unknown_attribute_keys_caught() {
        let mut weights_bps = BTreeMap::new();
        weights_bps.insert("finsihing".to_string(), 5_000); // typo
        weights_bps.insert("passing".to_string(), 5_000);
        let w = RoleWeights { weights_bps };
        let unknown = w.unknown_attribute_keys();
        assert_eq!(unknown, vec!["finsihing"]);
    }

    #[test]
    fn hidden_field_used_as_visible_weight_is_rejected() {
        // Codex audit P1 (2026-05-13): hidden fields like
        // injury_proneness / professionalism are NOT valid CA-weight keys
        // per ADR-0002 §"Choices" item 6. The validator must catch this
        // even though injury_proneness IS a real PlayerAttributes field.
        let mut weights_bps = BTreeMap::new();
        weights_bps.insert("injury_proneness".to_string(), 5_000); // real field, but hidden
        weights_bps.insert("finishing".to_string(), 5_000);
        let w = RoleWeights { weights_bps };
        let unknown = w.unknown_attribute_keys();
        assert_eq!(unknown, vec!["injury_proneness"]);
    }

    #[test]
    fn role_id_try_new_rejects_empty() {
        assert!(matches!(RoleId::try_new(""), Err(RoleIdError::Empty)));
    }

    #[test]
    fn role_id_try_new_rejects_whitespace() {
        assert!(matches!(
            RoleId::try_new(" GK"),
            Err(RoleIdError::Whitespace { .. })
        ));
        assert!(matches!(
            RoleId::try_new("GK "),
            Err(RoleIdError::Whitespace { .. })
        ));
        assert!(matches!(
            RoleId::try_new("   "),
            Err(RoleIdError::Whitespace { .. })
        ));
    }

    #[test]
    fn role_id_try_new_accepts_valid() {
        assert!(RoleId::try_new("GK").is_ok());
        assert!(RoleId::try_new("RWB").is_ok());
        assert!(RoleId::try_new("attacking-midfielder").is_ok());
    }

    #[test]
    #[should_panic(expected = "invalid RoleId")]
    fn role_id_new_panics_on_empty() {
        let _ = RoleId::new("");
    }

    #[test]
    fn unnormalized_weights_caught() {
        let mut weights_bps = BTreeMap::new();
        weights_bps.insert("finishing".to_string(), 5_000);
        weights_bps.insert("passing".to_string(), 4_000); // sum = 9_000
        let w = RoleWeights { weights_bps };
        assert!(!w.is_normalized());
        assert_eq!(w.sum_bps(), 9_000);
    }

    #[test]
    fn invalid_roles_collects_all() {
        let mut roles = BTreeMap::new();
        roles.insert(RoleId::new("GK"), sample_gk_weights());
        // Two deliberately broken entries — confirms collect-all behavior.
        let mut bad1 = BTreeMap::new();
        bad1.insert("finishing".to_string(), 9_999);
        roles.insert(RoleId::new("ST"), RoleWeights { weights_bps: bad1 });
        let mut bad2 = BTreeMap::new();
        bad2.insert("passing".to_string(), 5_000);
        bad2.insert("dribbling".to_string(), 4_500); // sum = 9_500
        roles.insert(RoleId::new("WB"), RoleWeights { weights_bps: bad2 });
        let table = RoleAffinityTable {
            schema_version: ROLE_AFFINITY_SCHEMA_VERSION,
            id: "fwh.core:role-affinities.test".to_string(),
            roles,
        };
        let invalid = table.invalid_roles();
        assert_eq!(invalid.len(), 2);
        assert!(
            invalid
                .iter()
                .any(|(id, sum)| id.as_str() == "ST" && *sum == 9_999)
        );
        assert!(
            invalid
                .iter()
                .any(|(id, sum)| id.as_str() == "WB" && *sum == 9_500)
        );
    }

    #[test]
    fn well_formed_table_passes_validation() {
        let mut roles = BTreeMap::new();
        roles.insert(RoleId::new("GK"), sample_gk_weights());
        roles.insert(RoleId::new("AM"), sample_am_weights());
        let table = RoleAffinityTable {
            schema_version: ROLE_AFFINITY_SCHEMA_VERSION,
            id: "fwh.core:role-affinities.default".to_string(),
            roles,
        };
        assert!(table.invalid_roles().is_empty());
        assert!(table.unknown_attribute_keys().is_empty());
        assert!(table.get(&RoleId::new("GK")).is_some());
        assert!(table.get(&RoleId::new("ZZ")).is_none());
    }

    #[test]
    fn ron_round_trip() {
        let mut roles = BTreeMap::new();
        roles.insert(RoleId::new("GK"), sample_gk_weights());
        roles.insert(RoleId::new("AM"), sample_am_weights());
        let table = RoleAffinityTable {
            schema_version: ROLE_AFFINITY_SCHEMA_VERSION,
            id: "fwh.core:role-affinities.default".to_string(),
            roles,
        };
        let ron_text = ron::ser::to_string(&table).expect("ron encode");
        let decoded: RoleAffinityTable = ron::de::from_str(&ron_text).expect("ron decode");
        assert_eq!(decoded, table);
        assert!(decoded.invalid_roles().is_empty());
    }

    #[test]
    fn role_id_serde_transparent() {
        // RoleId must serialize as a bare string for RON authoring.
        let id = RoleId::new("CB");
        let s = ron::ser::to_string(&id).expect("encode");
        assert_eq!(s, "\"CB\"");
        let decoded: RoleId = ron::de::from_str("\"CB\"").expect("decode");
        assert_eq!(decoded, id);
    }
}

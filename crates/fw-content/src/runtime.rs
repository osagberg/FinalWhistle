//! Runtime content store + deterministic samplers.
//!
//! The store is loaded once at startup, frozen, and shared (Arc) across
//! the rest of the runtime. Sampling functions take a derived seed; the
//! caller is responsible for threading the right `(career_seed, entity_id,
//! kind)` tuple into `derive_seed`.
//!
//! This is the T0 scaffold — enough to compile and demonstrate the pattern.
//! Real loaders for bios / headlines / commentary / scout-phrases land in
//! T2-3 → T3-5 per `docs/CONTENT_PIPELINE.md` §6.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Per-culture naming corpus
// ---------------------------------------------------------------------------

/// A culture archetype — defines the naming priors for procedurally
/// generated players from this cultural region.
///
/// Loaded from `content/sources/cultures/<id>.ron` (hand-authored seed) or
/// `content/baked/cultures/<id>.ron` (after a `fw-content-baker bake-names`
/// pass extends the bank).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Culture {
    /// Stable content-pack-qualified culture ID. Format
    /// `^fwh\.core(?:\.v[0-9]+)?:culture\.[a-z0-9-]+$`. Validated at load.
    pub id: String,
    /// Display name (used in dev UIs only; never surfaced to players).
    pub name: String,
    /// First-name bank. Order is load-bearing for determinism — preserved
    /// from the baked RON. Minimum 20 entries.
    pub first_name_bank: Vec<String>,
    /// Last-name bank. Same constraints as first_name_bank.
    pub last_name_bank: Vec<String>,
    /// Naming pattern grammar — `{first}`, `{last}`, optional `{patronymic}`.
    /// Default `"{first} {last}"`.
    #[serde(default = "default_naming_pattern")]
    pub naming_pattern: String,
    /// Tuning knobs — first-letter distribution diversity, compound-last-name
    /// probability, etc. Defaults are conservative; bakers may extend.
    #[serde(default)]
    pub weights: CultureWeights,
}

fn default_naming_pattern() -> String {
    "{first} {last}".to_string()
}

/// Sampling weights expressed as **basis points** (0..=10_000 maps to 0.0..=1.0).
///
/// `f32` was rejected per Codex pre-T0 audit: even though
/// `ContentStore::sample_player_name` runs at career-init not match-tick,
/// the sampled name lands in canonical state. Keeping the whole sampling
/// path integer-only removes a class of cross-platform rand-version drift.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CultureWeights {
    /// 0..=10_000 basis points. Higher values force a more uniform first-letter
    /// distribution across sampled names within a club roster.
    #[serde(default)]
    pub first_alpha_diversity_bps: u16,
    /// 0..=10_000 basis points. Probability a sampled last-name is rendered
    /// as a compound ("Smith-Jones") via two-pull from the last_name_bank.
    #[serde(default)]
    pub compound_last_chance_bps: u16,
}

// ---------------------------------------------------------------------------
// Tactical archetype — hand-authored, NOT LLM-baked
// ---------------------------------------------------------------------------

/// A tactical archetype — formation + press radius + buildup-speed factor +
/// BT-archetype reference. Ported from FW v1 `direct-pressing.yaml`.
///
/// **NOT baked by fw-content-baker.** Archetypes drive canonical sim
/// behavior; even a one-coordinate drift would shift the pinned canonical
/// hash. Hand-authored, reviewed, committed under
/// `content/sources/archetypes/`. See `design/match-sim.md` (once authored)
/// for the BT-runner contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalArchetype {
    pub id: String,
    pub formation: Vec<FormationSlot>,
    pub press_radius_metres: u32,
    /// Q32.32-equivalent at the source level; stored as `f32` here because
    /// archetype files are loaded as content (NOT canonical sim state) and
    /// the BT-runner converts to Q32 at load time.
    pub buildup_speed_factor: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationSlot {
    pub roster_slot: u8, // 1..=11
    pub role: String,    // "GK", "RB", "RCB", ...
    pub x: i16,          // metres, home-orientation (own goal at X = -52)
    pub z: i16,
}

// ---------------------------------------------------------------------------
// ContentKind — locked discriminant for derive_seed
// ---------------------------------------------------------------------------

/// Stable enum of every content category the runtime samples from.
///
/// **The discriminant values are LOAD-BEARING for determinism.** Reordering,
/// removing, or re-numbering variants is a corpus_version bump. Adding a
/// new variant at the end is forward-compatible.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContentKind {
    PlayerName = 0x01,
    ClubName = 0x02,
    StadiumName = 0x03,
    PlayerBio = 0x04,
    ScoutPhrase = 0x05,
    NewsHeadline = 0x06,
    ManagerQuote = 0x07,
    FanReaction = 0x08,
    CommentaryLine = 0x09,
}

/// Derive a u64 seed from a `(career_seed, entity_id, kind)` triple.
///
/// BLAKE3 over a fixed-order 17-byte buffer; truncate to u64 LE. Identical
/// inputs ⇒ identical output across OSes (BLAKE3 is deterministic + has no
/// platform-specific paths).
pub fn derive_seed(career_seed: u64, entity_id: u64, kind: ContentKind) -> u64 {
    let mut buf = [0u8; 17];
    buf[0..8].copy_from_slice(&career_seed.to_le_bytes());
    buf[8..16].copy_from_slice(&entity_id.to_le_bytes());
    buf[16] = kind as u8;
    let hash = blake3::hash(&buf);
    let bytes = hash.as_bytes();
    u64::from_le_bytes(bytes[0..8].try_into().expect("hash >= 8 bytes"))
}

// ---------------------------------------------------------------------------
// ContentStore — the loaded corpus
// ---------------------------------------------------------------------------

/// Loaded content corpus. Constructed once at startup; never mutated.
///
/// BTreeMap keyed by stable culture/archetype IDs — never HashMap (would
/// inject iteration non-determinism into any sampler that scans the table).
#[derive(Debug, Clone, Default)]
pub struct ContentStore {
    pub corpus_version: u32,
    pub cultures: BTreeMap<String, Culture>,
    pub tactical_archetypes: BTreeMap<String, TacticalArchetype>,
    // TODO(T2-3): bios, scout phrases, headlines, manager quotes, fan
    // reactions, commentary — wired in as each baker subcommand lands.
}

#[derive(Debug, thiserror::Error)]
pub enum ContentLoadError {
    #[error("I/O reading content path {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("RON parse error in {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: ron::error::SpannedError,
    },
    #[error("missing required content directory: {0}")]
    MissingDir(PathBuf),
    #[error("corpus_version mismatch: shipped={shipped} on-disk={found}")]
    VersionMismatch { shipped: u32, found: u32 },
}

impl ContentStore {
    /// Load the baked corpus from a directory tree.
    ///
    /// Expected layout under `path`:
    /// ```text
    /// content/
    ///   baked/
    ///     manifest.ron
    ///     cultures/<id>.ron     ← TODO T2-3 once the baker writes here
    ///   sources/
    ///     cultures/<id>.ron
    ///     archetypes/<id>.ron
    /// ```
    ///
    /// At T0 this stub only loads `content/sources/cultures/*.ron` and
    /// `content/sources/archetypes/*.ron` so the runtime has *something* to
    /// sample from before the baker exists.
    pub fn load_baked(_path: &Path) -> Result<Self, ContentLoadError> {
        // TODO(T2-3): walk content/baked/**.ron, parse, populate; honor mod
        // load order; verify corpus_version against the manifest.
        Ok(Self::default())
    }

    /// Deterministically sample a player display name from a given culture.
    ///
    /// `seed` should be `derive_seed(career_seed, player_entity_id,
    /// ContentKind::PlayerName)`.
    pub fn sample_player_name(&self, culture_id: &str, seed: u64) -> Option<String> {
        let culture = self.cultures.get(culture_id)?;
        if culture.first_name_bank.is_empty() || culture.last_name_bank.is_empty() {
            return None;
        }
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let first_idx = rng.gen_range(0..culture.first_name_bank.len());
        let last_idx = rng.gen_range(0..culture.last_name_bank.len());
        let mut name = culture
            .naming_pattern
            .replace("{first}", &culture.first_name_bank[first_idx])
            .replace("{last}", &culture.last_name_bank[last_idx]);
        // Optional compound last name (per culture weights). Integer-only
        // roll: 0..=9_999 < bps_weight has the same semantics as
        // `f32_roll < f32_weight` but is platform-portable without any
        // dependency on rand's f32 distribution stability.
        if culture.weights.compound_last_chance_bps > 0 {
            let roll: u16 = rng.gen_range(0..10_000) as u16;
            if roll < culture.weights.compound_last_chance_bps {
                let second_last_idx = rng.gen_range(0..culture.last_name_bank.len());
                let compound = format!(
                    "{}-{}",
                    culture.last_name_bank[last_idx], culture.last_name_bank[second_last_idx]
                );
                name = name.replace(&culture.last_name_bank[last_idx], &compound);
            }
        }
        Some(name)
    }

    /// Look up a hand-authored tactical archetype by stable ID.
    pub fn tactical_archetype(&self, id: &str) -> Option<&TacticalArchetype> {
        self.tactical_archetypes.get(id)
    }
}

// ---------------------------------------------------------------------------
// Tests — determinism floor
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_seed_is_deterministic() {
        let a = derive_seed(0xfeed_beef, 42, ContentKind::PlayerName);
        let b = derive_seed(0xfeed_beef, 42, ContentKind::PlayerName);
        assert_eq!(a, b);
    }

    #[test]
    fn derive_seed_distinguishes_kind() {
        let name = derive_seed(0xfeed_beef, 42, ContentKind::PlayerName);
        let bio = derive_seed(0xfeed_beef, 42, ContentKind::PlayerBio);
        assert_ne!(name, bio);
    }

    #[test]
    fn empty_store_returns_none() {
        let store = ContentStore::default();
        assert!(
            store
                .sample_player_name("fwh.core:culture.anglo", 1)
                .is_none()
        );
    }

    #[test]
    fn sample_player_name_is_deterministic() {
        let mut store = ContentStore::default();
        store.cultures.insert(
            "fwh.core:culture.anglo".to_string(),
            Culture {
                id: "fwh.core:culture.anglo".to_string(),
                name: "Anglo".to_string(),
                first_name_bank: vec!["James".into(), "William".into(), "Henry".into()],
                last_name_bank: vec!["Smith".into(), "Jones".into(), "Brown".into()],
                naming_pattern: "{first} {last}".to_string(),
                weights: CultureWeights::default(),
            },
        );
        let seed = derive_seed(0xfeed_beef, 42, ContentKind::PlayerName);
        let a = store.sample_player_name("fwh.core:culture.anglo", seed);
        let b = store.sample_player_name("fwh.core:culture.anglo", seed);
        assert_eq!(a, b);
        assert!(a.is_some());
    }
}

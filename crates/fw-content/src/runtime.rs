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
use std::fs;
use std::path::{Path, PathBuf};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// Walk `dir` and return every `*.ron` file path, sorted alphabetically.
///
/// **Sorting is load-bearing for determinism** (`.claude/rules/Sim/RULES.md`
/// §2). `fs::read_dir` iteration order is filesystem-dependent; sorting
/// guarantees the same load order on every platform + every run.
fn walk_ron_files(dir: &Path) -> Result<Vec<PathBuf>, ContentLoadError> {
    let mut entries: Vec<PathBuf> = Vec::new();
    let read = fs::read_dir(dir).map_err(|source| ContentLoadError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in read {
        let entry = entry.map_err(|source| ContentLoadError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("ron") {
            entries.push(path);
        }
    }
    entries.sort();
    Ok(entries)
}

/// Parse a single `*.ron` file into a typed `T`. Errors surface with the
/// file path attached for diagnostic visibility.
fn parse_ron_file<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, ContentLoadError> {
    let raw = fs::read_to_string(path).map_err(|source| ContentLoadError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    ron::de::from_str(&raw).map_err(|source| ContentLoadError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

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
///
/// `buildup_speed_factor_bps` is stored as `u16` basis points
/// (Codex Imp #3 from T0; integer-only sampling, no `f32` in the content
/// layer). Folded in at T1-1: the prior `f32` field would have leaked a
/// float into a path that ultimately writes canonical state, risking
/// cross-platform drift.
///
/// **Semantics — multiplier, not [0, 1]:** the field is a multiplier
/// against the engine's baseline buildup tempo, with
/// `BUILDUP_SPEED_BASELINE_BPS` (10_000) representing the neutral 1.0
/// reference. Values below baseline represent patient buildup (e.g.
/// `9_000` = 0.9, attacking-fullback overlapping pattern); values above
/// baseline represent route-one counter-attacking (e.g. `11_500` = 1.15,
/// low-block-counter pattern). The BT-runner reads this as
/// `Q32::from_int(bps as i32) / BUILDUP_SPEED_BASELINE_BPS` without
/// clipping to 1.0 — clipping would drop the counter-attacking boost
/// silently.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalArchetype {
    pub id: String,
    pub formation: Vec<FormationSlot>,
    pub press_radius_metres: u32,
    /// Buildup-speed multiplier in basis points; see the type-level doc.
    /// `BUILDUP_SPEED_BASELINE_BPS` (10_000) is neutral 1.0; valid range
    /// is `BUILDUP_SPEED_MIN_BPS..=BUILDUP_SPEED_MAX_BPS`. The BT-runner
    /// MUST NOT clip to 10_000 — values above baseline carry the
    /// counter-attacking semantics.
    pub buildup_speed_factor_bps: u16,
}

/// Baseline buildup-speed factor in basis points. `10_000` corresponds to
/// a neutral `Q32::ONE` after the BT-runner's divide. Lives on
/// `TacticalArchetype`'s contract — content authors and the BT-runner
/// both read this constant rather than hard-coding `10_000`.
pub const BUILDUP_SPEED_BASELINE_BPS: u16 = 10_000;

/// Minimum sensible buildup-speed factor (0.5 = very patient).
/// Below this, formation timing breaks down at the BT-runner's tick rate.
pub const BUILDUP_SPEED_MIN_BPS: u16 = 5_000;

/// Maximum sensible buildup-speed factor (2.0 = pure transition football).
/// Above this, the BT-runner can't produce coherent build-up phases.
pub const BUILDUP_SPEED_MAX_BPS: u16 = 20_000;

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
    pub player_templates: BTreeMap<String, crate::PlayerTemplate>,
    pub role_affinity_tables: BTreeMap<String, crate::RoleAffinityTable>,
    /// Signature definitions — keyed by `SignatureId.as_str()` for O(log n)
    /// look-up at T1-2b-iv dispatch time. Loaded from
    /// `content/sources/signatures/*.ron`. BTreeMap for deterministic iteration.
    pub signature_definitions: BTreeMap<String, crate::SignatureDefinition>,
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
    /// Load the on-disk content corpus from a directory tree.
    ///
    /// Codex audit P1 (2026-05-13): this was previously
    /// `pub fn load_baked(_) -> Ok(Self::default())` — a stub that
    /// returned an empty store while the docs claimed it loaded
    /// content. The current implementation walks `content/sources/*`
    /// under `path` and populates every BTreeMap on the store from
    /// the on-disk RON.
    ///
    /// Expected layout under `path`:
    /// ```text
    /// content/
    ///   sources/
    ///     cultures/<id>.ron         → Culture
    ///     archetypes/<id>.ron       → TacticalArchetype
    ///     role-affinities/<id>.ron  → RoleAffinityTable
    ///     players/<id>.ron          → PlayerTemplate
    ///   baked/                      ← TODO T2-3: bake-pipeline output
    ///     manifest.ron
    ///     cultures/<id>.ron, ...
    /// ```
    ///
    /// **T1-2b scope:** loads `content/sources/*` only. The `content/baked/`
    /// path + manifest validation + mod-overlay load order all land at T2-3
    /// alongside the real baker pipeline (per `docs/MASTER_PLAN.md` T2-3).
    /// Until then, `load_sources` is the only loader path and is what
    /// `load_baked` delegates to.
    ///
    /// Errors propagate as `ContentLoadError`; the loader is fail-closed.
    /// No silent defaults; no `unwrap_or_default` fallbacks.
    pub fn load_sources(content_root: &Path) -> Result<Self, ContentLoadError> {
        let sources_dir = content_root.join("sources");
        if !sources_dir.is_dir() {
            return Err(ContentLoadError::MissingDir(sources_dir));
        }
        let mut store = Self::default();

        // Cultures
        let cultures_dir = sources_dir.join("cultures");
        if cultures_dir.is_dir() {
            for entry in walk_ron_files(&cultures_dir)? {
                let parsed: Culture = parse_ron_file(&entry)?;
                store.cultures.insert(parsed.id.clone(), parsed);
            }
        }

        // Tactical archetypes
        let archetypes_dir = sources_dir.join("archetypes");
        if archetypes_dir.is_dir() {
            for entry in walk_ron_files(&archetypes_dir)? {
                let parsed: TacticalArchetype = parse_ron_file(&entry)?;
                store.tactical_archetypes.insert(parsed.id.clone(), parsed);
            }
        }

        // Role-affinity tables
        let role_aff_dir = sources_dir.join("role-affinities");
        if role_aff_dir.is_dir() {
            for entry in walk_ron_files(&role_aff_dir)? {
                let parsed: crate::RoleAffinityTable = parse_ron_file(&entry)?;
                store.role_affinity_tables.insert(parsed.id.clone(), parsed);
            }
        }

        // Player templates
        let players_dir = sources_dir.join("players");
        if players_dir.is_dir() {
            for entry in walk_ron_files(&players_dir)? {
                let parsed: crate::PlayerTemplate = parse_ron_file(&entry)?;
                store
                    .player_templates
                    .insert(parsed.qualified_id.clone(), parsed);
            }
        }

        // Signature definitions (T1-3). Optional — dir may not exist yet in
        // older content packs; silently skip if absent (same guard pattern as
        // cultures/archetypes/players above).
        let signatures_dir = sources_dir.join("signatures");
        if signatures_dir.is_dir() {
            for entry in walk_ron_files(&signatures_dir)? {
                let parsed: crate::SignatureDefinition = parse_ron_file(&entry)?;
                store
                    .signature_definitions
                    .insert(parsed.id.as_str().to_owned(), parsed);
            }
        }

        Ok(store)
    }

    /// Delegate to `load_sources` while the baked-corpus pipeline is being
    /// implemented (T2-3). The signature is preserved for forward-compat —
    /// callers will not need to change paths when T2-3 lands.
    pub fn load_baked(content_root: &Path) -> Result<Self, ContentLoadError> {
        // TODO(T2-3): walk content/baked/**.ron, parse manifest, honour mod
        // load order, verify corpus_version. For now, delegate to
        // load_sources — every existing caller treats the result the same
        // way regardless of source/baked origin.
        Self::load_sources(content_root)
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
    fn load_sources_walks_content_directory() {
        // Locate the workspace-root content/ directory the same way
        // crates/fw-content/tests/fixtures_load.rs does. CARGO_MANIFEST_DIR
        // = crates/fw-content; workspace root = ../..
        let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("content");
        let store = ContentStore::load_sources(&content_root)
            .expect("load_sources should succeed against the committed fixtures");
        // Sample assertions — fixtures must contain at least one of each.
        assert!(
            !store.cultures.is_empty(),
            "expected at least one culture loaded from content/sources/cultures/"
        );
        assert!(
            !store.tactical_archetypes.is_empty(),
            "expected at least one tactical archetype loaded"
        );
        assert!(
            !store.role_affinity_tables.is_empty(),
            "expected at least one role-affinity table loaded"
        );
        assert!(
            !store.player_templates.is_empty(),
            "expected at least one player template loaded"
        );
    }

    #[test]
    fn load_sources_is_deterministic() {
        // Codex audit P1 lineage: confirm load order is sorted so the
        // result is reproducible across platforms.
        let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("content");
        let a = ContentStore::load_sources(&content_root).expect("load 1");
        let b = ContentStore::load_sources(&content_root).expect("load 2");
        // BTreeMap iteration is sorted by key; same keys + same values
        // = same iteration; if the file walk had non-deterministic order
        // a parse error would also surface in one but not the other.
        assert_eq!(
            a.cultures.keys().collect::<Vec<_>>(),
            b.cultures.keys().collect::<Vec<_>>()
        );
        assert_eq!(
            a.tactical_archetypes.keys().collect::<Vec<_>>(),
            b.tactical_archetypes.keys().collect::<Vec<_>>()
        );
        assert_eq!(
            a.role_affinity_tables.keys().collect::<Vec<_>>(),
            b.role_affinity_tables.keys().collect::<Vec<_>>()
        );
        assert_eq!(
            a.player_templates.keys().collect::<Vec<_>>(),
            b.player_templates.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn load_sources_missing_root_errors() {
        // Fail-closed: a missing content directory surfaces as an error,
        // not a silent empty store.
        let bogus = PathBuf::from("/tmp/this/path/does/not/exist-fwh-test");
        let result = ContentStore::load_sources(&bogus);
        assert!(matches!(result, Err(ContentLoadError::MissingDir(_))));
    }

    #[test]
    fn load_baked_delegates_to_load_sources() {
        // Until T2-3 lands the real baked-corpus path, load_baked
        // delegates. This test pins the contract — if anyone changes
        // load_baked without updating load_sources or vice-versa, this
        // breaks loudly.
        let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("content");
        let sources = ContentStore::load_sources(&content_root).expect("load_sources");
        let baked = ContentStore::load_baked(&content_root).expect("load_baked");
        assert_eq!(
            sources.cultures.keys().collect::<Vec<_>>(),
            baked.cultures.keys().collect::<Vec<_>>()
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

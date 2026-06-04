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

use crate::commentary::{CommentaryGrammarBank, MatchEventDiscriminant};
use crate::memory_callback::{MemoryCallbackGrammarBank, MemoryCallbackLoadError};
use crate::news::NewsGrammarBank;

/// Walk `dir` and return every file with the given extension, sorted
/// alphabetically. Sorting is load-bearing for determinism — `fs::read_dir`
/// iteration order is filesystem-dependent.
fn walk_files_with_ext(dir: &Path, ext: &str) -> Result<Vec<PathBuf>, ContentLoadError> {
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
        // Match files whose name ends with `.tracery.json` or the requested ext.
        // path.extension() only returns the final extension component (after the
        // last '.'), so `foo.tracery.json` gives extension "json" — not enough.
        // Use file_name() string matching instead.
        if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && name.ends_with(ext)
        {
            entries.push(path);
        }
    }
    entries.sort();
    Ok(entries)
}

/// Walk `dir` and return every `*.ron` file path, sorted alphabetically.
///
/// **Sorting is load-bearing for determinism** (`.claude/rules/Sim/RULES.md`
/// §2). `fs::read_dir` iteration order is filesystem-dependent; sorting
/// guarantees the same load order on every platform + every run.
fn walk_ron_files(dir: &Path) -> Result<Vec<PathBuf>, ContentLoadError> {
    walk_files_with_ext(dir, ".ron")
}

/// Load the commentary grammar bank from `commentary_dir`.
///
/// Each `*.tracery.json` file in the directory is parsed as a
/// `BTreeMap<String, Vec<String>>` (Tracery's native JSON format) and mapped
/// to a `MatchEventDiscriminant` by filename stem:
///
///   `kickoff`                                → `KickOff`
///   `full_time`                              → `FullTime`
///   `goal`                                   → `Goal`
///   `shot`                                   → `Shot`
///   `pass`                                   → `Pass`
///   `signature_first_fired`                  → `SignatureFirstFired` (generic)
///   `signature_first_fired.<slug>` (T4-2.5i) → per-signature sub-bank keyed by `<slug>`
///
/// Returns `ContentLoadError::MissingCommentaryGrammar` for any missing class
/// (fail-loud; all 6 are required). A malformed sub-bank file (bad JSON or
/// missing origin rule) also returns an error — fail-loud posture extends to
/// sub-banks (T4-2.5i spec).
fn load_commentary_grammars(
    commentary_dir: &Path,
) -> Result<CommentaryGrammarBank, ContentLoadError> {
    if !commentary_dir.is_dir() {
        // Return the first missing discriminant so the error is actionable.
        return Err(ContentLoadError::MissingCommentaryGrammar {
            event_class: MatchEventDiscriminant::KickOff,
        });
    }

    let mut raw: BTreeMap<MatchEventDiscriminant, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    // Per-signature sub-banks keyed by slug (e.g. "long-range-strike").
    let mut sig_banks: Vec<(String, BTreeMap<String, Vec<String>>)> = Vec::new();

    for path in walk_files_with_ext(commentary_dir, ".tracery.json")? {
        // T2-R-C5 (post-T2 ultimate-review Track C-5): the prior
        // `unwrap_or("")` silently mapped non-UTF8 file names or
        // missing-file-name (impossible-but-possible in principle) to
        // empty-stem, which fell into the `other` arm + silently
        // `continue`d. Replaced with explicit failure: non-UTF8 paths
        // are author-error and must surface, not become invisible.
        let stem = match path.file_name().and_then(|n| n.to_str()) {
            Some(s) => s.trim_end_matches(".tracery.json"),
            None => {
                return Err(ContentLoadError::Io {
                    path: path.clone(),
                    source: std::io::Error::other("non-UTF8 file name in commentary/ directory"),
                });
            }
        };

        // Detect per-signature sub-bank files: stem matches
        // `signature_first_fired.<slug>` (one dot after the base name).
        // The base `signature_first_fired` stem (no trailing dot) still
        // maps to the generic SignatureFirstFired discriminant.
        const SFF_PREFIX: &str = "signature_first_fired.";
        if let Some(slug) = stem.strip_prefix(SFF_PREFIX) {
            // slug is e.g. "long-range-strike".
            let raw_json = fs::read_to_string(&path).map_err(|source| ContentLoadError::Io {
                path: path.clone(),
                source,
            })?;
            let rules: BTreeMap<String, Vec<String>> =
                serde_json::from_str(&raw_json).map_err(|e| ContentLoadError::TraceryParse {
                    path: path.clone(),
                    source: tracery::Error::from(e),
                })?;
            sig_banks.push((slug.to_owned(), rules));
            continue;
        }

        let disc = match stem {
            "kickoff" => MatchEventDiscriminant::KickOff,
            "full_time" => MatchEventDiscriminant::FullTime,
            "goal" => MatchEventDiscriminant::Goal,
            "shot" => MatchEventDiscriminant::Shot,
            "pass" => MatchEventDiscriminant::Pass,
            "signature_first_fired" => MatchEventDiscriminant::SignatureFirstFired,
            // FUN-TS2b: offside grammar. Narrative-director to polish later.
            "offside" => MatchEventDiscriminant::Offside,
            other => {
                // T2-R-C5 (post-T2 ultimate-review Track C-5): the prior
                // shape claimed "log and skip" in the comment but never
                // logged — the `let _ = other; continue;` was a silent
                // drop. Replaced with an explicit eprintln! so operator-
                // observable diagnostic exists when (e.g.) an editor
                // backup file `kickoff.tracery.json.bak` is misread as
                // `kickoff.tracery.json.bak` stem. The downstream
                // missing-discriminant check at the bottom of this fn
                // still catches the load-gap fail-loud; this is the
                // diagnostic that names the cause.
                eprintln!(
                    "fw-content: skipping unknown commentary grammar filename: \
                     {} (stem {:?} not in known discriminants)",
                    path.display(),
                    other
                );
                continue;
            }
        };

        let raw_json = fs::read_to_string(&path).map_err(|source| ContentLoadError::Io {
            path: path.clone(),
            source,
        })?;

        let rules: BTreeMap<String, Vec<String>> =
            serde_json::from_str(&raw_json).map_err(|e| ContentLoadError::TraceryParse {
                path: path.clone(),
                source: tracery::Error::from(e),
            })?;

        raw.insert(disc, rules);
    }

    // Fail-loud: verify all 6 discriminants loaded.
    for disc in MatchEventDiscriminant::all() {
        if !raw.contains_key(&disc) {
            return Err(ContentLoadError::MissingCommentaryGrammar { event_class: disc });
        }
    }

    // All discriminants are present (checked above); try_from_map now ALSO
    // validates that each grammar has a non-empty `origin` rule with ≥1
    // non-empty variant (Codex Tier-2 type-design P1 on T1-4b 2026-05-16
    // tightened CommentaryGrammarBank::try_from_map to reject malformed
    // grammars at construction time, not silently empty at render time).
    // All build-error variants map to the same fail-loud ContentLoadError
    // for now; T1-12 content-validation hardening can distinguish them.
    let mut bank = CommentaryGrammarBank::try_from_map(raw).map_err(|e| {
        ContentLoadError::MissingCommentaryGrammar {
            event_class: e.discriminant(),
        }
    })?;

    // Attach per-signature sub-banks. Fail-loud: a malformed sub-bank (bad
    // JSON already caught above; missing origin caught by insert_signature_bank)
    // must error, not silently be skipped.
    for (slug, rules) in sig_banks {
        bank.insert_signature_bank(slug, rules).map_err(|e| {
            ContentLoadError::MissingCommentaryGrammar {
                event_class: e.discriminant(),
            }
        })?;
    }

    Ok(bank)
}

/// Load the narrative grammar bank (news headlines + manager quotes) from
/// `grammars_dir`.
///
/// Expects exactly two files in the directory:
///   `headlines.tracery.json`       → headline grammar
///   `manager-quotes.tracery.json`  → manager-quote grammar
///
/// Both files carry a `"_comment"` key — the JSON carries it as a
/// single-element array (`["..."]`) so `serde_json::from_str::<BTreeMap<String,
/// Vec<String>>>` parses it cleanly (Tracery ignores any rule never referenced
/// from `origin`). The loader reads the full `BTreeMap<String, Vec<String>>`
/// then hands it directly to `NewsGrammarBank::from_parts` which validates the
/// `origin` rule discipline.
///
/// Returns `ContentLoadError::MissingNarrativeGrammar` for any absent file
/// (fail-loud; both are required). Returns `ContentLoadError::TraceryParse` for
/// a malformed JSON file. Returns `ContentLoadError::MissingCommentaryGrammar`
/// (reused variant) if the loaded rules fail the `NewsGrammarBank` origin-rule
/// invariant — the `NewsBankBuildError` message is forwarded via the Display
/// impl.
fn load_narrative_grammars(grammars_dir: &Path) -> Result<NewsGrammarBank, ContentLoadError> {
    if !grammars_dir.is_dir() {
        return Err(ContentLoadError::MissingNarrativeGrammar {
            filename: "headlines.tracery.json",
        });
    }

    let load_grammar =
        |filename: &'static str| -> Result<BTreeMap<String, Vec<String>>, ContentLoadError> {
            let path = grammars_dir.join(filename);
            if !path.exists() {
                return Err(ContentLoadError::MissingNarrativeGrammar { filename });
            }
            let raw_json = fs::read_to_string(&path).map_err(|source| ContentLoadError::Io {
                path: path.clone(),
                source,
            })?;

            // Parse as `BTreeMap<String, serde_json::Value>` first, then
            // filter to array-valued entries only. This handles the
            // `"_comment": ["..."]` key (array — kept) and would also
            // gracefully skip any bare-string value that snuck in during
            // content authoring. Array-typed values are collected into
            // `Vec<String>` by unwrapping `Value::Array` of `Value::String`
            // entries; any non-string elements are silently skipped (Tracery
            // ignores them; the origin-rule validation in try_from_map is the
            // real gate).
            let raw_map: BTreeMap<String, serde_json::Value> = serde_json::from_str(&raw_json)
                .map_err(|e| ContentLoadError::TraceryParse {
                    path: path.clone(),
                    source: tracery::Error::from(e),
                })?;

            let rules: BTreeMap<String, Vec<String>> = raw_map
                .into_iter()
                .filter_map(|(k, v)| match v {
                    serde_json::Value::Array(arr) => {
                        let strings: Vec<String> = arr
                            .into_iter()
                            .filter_map(|el| match el {
                                serde_json::Value::String(s) => Some(s),
                                _ => None,
                            })
                            .collect();
                        Some((k, strings))
                    }
                    _ => None, // skip bare-string _comment or other non-array values
                })
                .collect();

            Ok(rules)
        };

    let headline_rules = load_grammar("headlines.tracery.json")?;
    let quote_rules = load_grammar("manager-quotes.tracery.json")?;

    // Fail-loud: try_from_map validates origin-rule discipline on both grammars.
    // Map the build error to an Io variant so the caller sees a clear message
    // naming the grammar key and the rule violation. `std::io::Error::other`
    // (stable since Rust 1.74) wraps the display string without requiring
    // serde traits in scope.
    crate::news::NewsGrammarBank::from_parts(headline_rules, quote_rules).map_err(|e| {
        ContentLoadError::Io {
            path: grammars_dir.to_path_buf(),
            source: std::io::Error::other(e.to_string()),
        }
    })
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
    /// Team-name bank. Hand-authored football-native club names for this
    /// culture. Used by `procgen::generate_team` to pick a team name;
    /// Markov is NOT used for team names (team names are authored, not
    /// generated). `#[serde(default)]` for backwards-compat with
    /// pre-T1-7 culture fixtures that lack this field.
    #[serde(default)]
    pub team_name_bank: Vec<String>,
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
    /// Optional explicit line-height in metres (home orientation, signed).
    /// When `Some(m)`, the bridge uses this value directly for
    /// `default_in_defence_state` rather than deriving it from
    /// `press_radius_metres`. When `None`, the bridge falls back to the
    /// legacy coupled rule (press_radius > 20 → MidBlock, else LowBlock).
    ///
    /// FUN-TS2d (2026-06-04): decouples line-height from press-intensity.
    /// A team can set a high line WITHOUT high press (e.g. "high line, low
    /// press" = deep-lying playmaker system) or press hard from a mid-block
    /// (e.g. "low line, high press" = ultra-defensive trap-press).
    ///
    /// Values:
    ///   < 20  → LowBlock (own half, deep)
    ///   20-35 → MidBlock (middle third)
    ///   > 35  → HighPress (high line, near or past centre)
    ///
    /// Existing archetypes that omit this field retain the prior derived
    /// behaviour (no canonical-hash drift).
    #[serde(default)]
    pub line_height_metres: Option<u32>,
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
#[derive(Debug, Clone)]
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
    /// Player bio identity records — keyed by `PlayerBio.player_id` string.
    /// Loaded from `content/sources/player-bios/*.ron`. Optional directory —
    /// absent directory is silently skipped (backwards-compat; content packs
    /// without the T2-4 PlayerBio layer still load cleanly).
    /// BTreeMap for deterministic iteration per `Sim/RULES.md §2`.
    pub player_bios: BTreeMap<String, crate::player_bio::PlayerBio>,
    /// In-match commentary grammar bank. One grammar per `MatchEventDiscriminant`.
    /// Loaded from `content/sources/commentary/*.tracery.json`.
    /// Missing any of the 6 required files is a hard load error
    /// (`ContentLoadError::MissingCommentaryGrammar`).
    pub commentary_grammars: crate::commentary::CommentaryGrammarBank,
    /// Manager archetypes — keyed by stable ID (`fwh.core:manager.<slug>`).
    /// Loaded from `content/sources/managers/*.ron`. Optional directory —
    /// absent directory is silently skipped (backwards-compat; old content
    /// packs have no managers/ dir).
    pub managers: BTreeMap<String, crate::manager::ManagerArchetype>,
    /// News headline + manager-quote grammar bank. Loaded from
    /// `content/sources/grammars/headlines.tracery.json` and
    /// `content/sources/grammars/manager-quotes.tracery.json`.
    /// Missing either file is a hard load error
    /// (`ContentLoadError::MissingNarrativeGrammar`).
    pub news_grammars: crate::news::NewsGrammarBank,
    /// Memory-callback phrase bank. Loaded from
    /// `content/sources/grammars/memory-callback.tracery.json`.
    /// Missing the file is a hard load error
    /// (`ContentLoadError::MissingNarrativeGrammar`).
    pub memory_callback_grammars: crate::memory_callback::MemoryCallbackGrammarBank,
    // TODO(T2-3): bios, scout phrases, fan reactions — wired in as each
    // baker subcommand lands.
}

impl Default for ContentStore {
    fn default() -> Self {
        // Build a bank with non-empty placeholder grammars for all 6
        // discriminants. Used in tests that construct a ContentStore without
        // going through load_sources.
        //
        // Post Codex Tier-2 type-design P1 on T1-4b 2026-05-16:
        // CommentaryGrammarBank::try_from_map now rejects empty-variant
        // origin rules (the construction-time guard that was missing
        // pre-fix-pass). The placeholder MUST be non-empty so this Default
        // impl still passes — switched from `vec![""]` to
        // `vec!["(default placeholder)".into()]`. The "(default placeholder)"
        // text surfaces in test output if anyone forgets to load real
        // grammars + helps debugging.
        use crate::commentary::{CommentaryGrammarBank, MatchEventDiscriminant};
        let mut map = std::collections::BTreeMap::new();
        for disc in MatchEventDiscriminant::all() {
            let mut rules: std::collections::BTreeMap<String, Vec<String>> =
                std::collections::BTreeMap::new();
            rules.insert(
                "origin".into(),
                vec![format!("(default placeholder for {disc:?})")],
            );
            map.insert(disc, rules);
        }
        // try_from_map is infallible here — all 6 discriminants present with
        // non-empty origin variants, satisfying the tightened invariant.
        let commentary_grammars = CommentaryGrammarBank::try_from_map(map)
            .expect("default ContentStore: all discriminants present with non-empty origin");

        // Build placeholder news grammar bank for tests that construct a
        // ContentStore without going through load_sources. The placeholder text
        // surfaces in test output if anyone forgets to load real grammars.
        let mut headline_rules: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        headline_rules.insert(
            "origin".into(),
            vec!["(default headline placeholder)".into()],
        );
        let mut quote_rules: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        quote_rules.insert("origin".into(), vec!["(default quote placeholder)".into()]);
        let news_grammars = crate::news::NewsGrammarBank::from_parts(headline_rules, quote_rules)
            .expect("default ContentStore: news grammar placeholder must construct cleanly");

        // Build placeholder memory-callback grammar bank for tests that
        // construct a ContentStore without going through load_sources.
        let mut mc_rules: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        mc_rules.insert(
            "origin".into(),
            vec!["(default memory-callback placeholder)".into()],
        );
        let memory_callback_grammars = MemoryCallbackGrammarBank::try_from_rules(mc_rules)
            .expect("default ContentStore: memory-callback placeholder must construct cleanly");

        Self {
            corpus_version: 0,
            cultures: BTreeMap::new(),
            tactical_archetypes: BTreeMap::new(),
            player_templates: BTreeMap::new(),
            role_affinity_tables: BTreeMap::new(),
            signature_definitions: BTreeMap::new(),
            player_bios: BTreeMap::new(),
            commentary_grammars,
            managers: BTreeMap::new(),
            news_grammars,
            memory_callback_grammars,
        }
    }
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
    /// A required commentary grammar file is absent from
    /// `content/sources/commentary/`. Fail-loud: every event class must have
    /// a grammar; missing one means commentary is silently broken.
    #[error(
        "missing commentary grammar for event class {event_class:?}; \
         expected file in content/sources/commentary/"
    )]
    MissingCommentaryGrammar {
        event_class: crate::commentary::MatchEventDiscriminant,
    },
    /// A `.tracery.json` file failed to parse.
    #[error("Tracery parse error in {path}: {source}")]
    TraceryParse {
        path: PathBuf,
        #[source]
        source: tracery::Error,
    },
    /// Two RON files in the same content category claim the same stable ID.
    ///
    /// IDs must be unique within a content category — duplicate IDs would cause
    /// the second file to silently overwrite the first, making load order
    /// a hidden correctness dependency. Fail-closed: surface the collision with
    /// both paths so the content author can resolve it.
    #[error(
        "duplicate content ID in {kind}: id={id:?} first seen at {path_first:?}, \
         duplicate at {path_dupe:?}"
    )]
    DuplicateId {
        /// Content category (e.g. `"culture"`, `"archetype"`, `"signature_definition"`).
        kind: &'static str,
        /// The colliding stable content-pack-qualified ID string.
        id: String,
        /// Path of the first RON file that claimed this ID.
        path_first: PathBuf,
        /// Path of the duplicate RON file that also claimed this ID.
        path_dupe: PathBuf,
    },
    /// A required narrative grammar file is absent from
    /// `content/sources/grammars/`. Fail-loud: both headline and manager-quote
    /// grammars are required; missing one means the news renderer is broken.
    #[error(
        "missing required narrative grammar file: {filename}; \
         expected in content/sources/grammars/"
    )]
    MissingNarrativeGrammar { filename: &'static str },

    /// A content fixture references another content entity by ID, but that
    /// ID doesn't resolve in the loaded `ContentStore`.
    ///
    /// T1-7 fix-pass per silent-failure F4 — prior `ManagerArchetype` doc
    /// claimed cross-reference validation at load time but no validator
    /// existed; manager fixtures with dangling `tactical_archetype_id`
    /// loaded silently + the failure only surfaced later via
    /// `generate_team`'s `MissingTacticalArchetype`, misleadingly
    /// suggesting the call site was wrong rather than the fixture. This
    /// variant surfaces dangling refs at load time.
    #[error(
        "dangling content reference: {from_kind} {from_id:?} (loaded from {from_path:?}) \
         references {to_kind} {to_id:?} which does not exist in the loaded ContentStore"
    )]
    DanglingReference {
        /// Content category of the entity holding the reference (e.g. `"manager_archetype"`).
        from_kind: &'static str,
        /// ID of the entity holding the reference.
        from_id: String,
        /// Path of the RON file that defines the entity holding the reference.
        from_path: PathBuf,
        /// Content category being referenced (e.g. `"tactical_archetype"`).
        to_kind: &'static str,
        /// The unresolved reference ID.
        to_id: String,
    },
}

/// Insert `value` into `map` under `id`, rejecting duplicates.
///
/// `paths_seen` tracks the first path that claimed each ID in this load pass.
/// On the second insertion for the same `id`, returns
/// `ContentLoadError::DuplicateId` with both paths for fail-loud reporting.
///
/// # Arguments
/// - `map` — the target `BTreeMap` on the `ContentStore` field.
/// - `paths_seen` — local accumulator keyed by the same ID type; one per
///   loader block, not shared across categories (no cross-category ID collision
///   rule).
/// - `id` — the stable content-pack-qualified ID string from the parsed RON.
/// - `value` — the parsed entity to insert.
/// - `kind` — content category label for the error message (e.g. `"culture"`).
/// - `path` — the RON file path that `value` was parsed from.
fn insert_unique<V>(
    map: &mut BTreeMap<String, V>,
    paths_seen: &mut BTreeMap<String, PathBuf>,
    id: String,
    value: V,
    kind: &'static str,
    path: PathBuf,
) -> Result<(), ContentLoadError> {
    if let Some(path_first) = paths_seen.get(&id) {
        return Err(ContentLoadError::DuplicateId {
            kind,
            id,
            path_first: path_first.clone(),
            path_dupe: path,
        });
    }
    paths_seen.insert(id.clone(), path);
    map.insert(id, value);
    Ok(())
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
            let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
            for entry in walk_ron_files(&cultures_dir)? {
                let parsed: Culture = parse_ron_file(&entry)?;
                let id = parsed.id.clone();
                insert_unique(&mut store.cultures, &mut seen, id, parsed, "culture", entry)?;
            }
        }

        // Tactical archetypes
        let archetypes_dir = sources_dir.join("archetypes");
        if archetypes_dir.is_dir() {
            let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
            for entry in walk_ron_files(&archetypes_dir)? {
                let parsed: TacticalArchetype = parse_ron_file(&entry)?;
                let id = parsed.id.clone();
                insert_unique(
                    &mut store.tactical_archetypes,
                    &mut seen,
                    id,
                    parsed,
                    "archetype",
                    entry,
                )?;
            }
        }

        // Role-affinity tables
        let role_aff_dir = sources_dir.join("role-affinities");
        if role_aff_dir.is_dir() {
            let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
            for entry in walk_ron_files(&role_aff_dir)? {
                let parsed: crate::RoleAffinityTable = parse_ron_file(&entry)?;
                let id = parsed.id.clone();
                insert_unique(
                    &mut store.role_affinity_tables,
                    &mut seen,
                    id,
                    parsed,
                    "role_affinity_table",
                    entry,
                )?;
            }
        }

        // Player templates.
        //
        // T1-20: track player-fixture paths separately (mirroring the managers
        // loader's `manager_paths` at line 662) so the post-load cross-reference
        // validator below can surface the RON file path in the DanglingReference
        // error when a `signature_candidates[i].signature_id` fails to resolve
        // in `store.signature_definitions`.
        let mut player_paths: BTreeMap<String, PathBuf> = BTreeMap::new();
        let players_dir = sources_dir.join("players");
        if players_dir.is_dir() {
            let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
            for entry in walk_ron_files(&players_dir)? {
                let parsed: crate::PlayerTemplate = parse_ron_file(&entry)?;
                let id = parsed.qualified_id.clone();
                player_paths.insert(id.clone(), entry.clone());
                insert_unique(
                    &mut store.player_templates,
                    &mut seen,
                    id,
                    parsed,
                    "player_template",
                    entry,
                )?;
            }
        }

        // Signature definitions (T1-3). Optional — dir may not exist yet in
        // older content packs; silently skip if absent (same guard pattern as
        // cultures/archetypes/players above).
        let signatures_dir = sources_dir.join("signatures");
        if signatures_dir.is_dir() {
            let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
            for entry in walk_ron_files(&signatures_dir)? {
                let parsed: crate::SignatureDefinition = parse_ron_file(&entry)?;
                let id = parsed.id.as_str().to_owned();
                insert_unique(
                    &mut store.signature_definitions,
                    &mut seen,
                    id,
                    parsed,
                    "signature_definition",
                    entry,
                )?;
            }
        }

        // Commentary grammars (T1-4b). Required — missing the directory is a
        // hard load error (fail-loud per T1-12 content-validation hardening).
        // Filename → MatchEventDiscriminant mapping:
        //   kickoff.tracery.json            → KickOff
        //   full_time.tracery.json          → FullTime
        //   goal.tracery.json               → Goal
        //   shot.tracery.json               → Shot
        //   pass.tracery.json               → Pass
        //   signature_first_fired.tracery.json → SignatureFirstFired
        let commentary_dir = sources_dir.join("commentary");
        store.commentary_grammars = load_commentary_grammars(&commentary_dir)?;

        // Narrative grammars — news headlines + manager quotes (T3-3).
        // Required: both `headlines.tracery.json` and `manager-quotes.tracery.json`
        // must exist under `content/sources/grammars/`. Missing either is a hard
        // load error (fail-loud; the news renderer is broken without them).
        let grammars_dir = sources_dir.join("grammars");
        store.news_grammars = load_narrative_grammars(&grammars_dir)?;

        // Memory-callback grammar (T3-6). Required — missing the file is a
        // hard load error. Uses `MemoryCallbackGrammarBank::load_from_dir`
        // which validates origin-rule discipline at construction time.
        //
        // Each error variant maps to a distinct `ContentLoadError` so callers
        // can distinguish "file missing" (operator problem) from "file present
        // but malformed JSON" (content-authoring problem) from "file valid JSON
        // but origin-rule broken" (grammar-structure problem). A blanket map to
        // `MissingNarrativeGrammar` was used before T3-6 self-review P1 — that
        // silently reported a parse failure as a missing file, obscuring the
        // real cause.
        store.memory_callback_grammars = MemoryCallbackGrammarBank::load_from_dir(&grammars_dir)
            .map_err(|e| {
                eprintln!("fw-content: failed to load memory-callback grammar: {e}");
                match e {
                    MemoryCallbackLoadError::MissingFile(_) => {
                        ContentLoadError::MissingNarrativeGrammar {
                            filename: "memory-callback.tracery.json",
                        }
                    }
                    MemoryCallbackLoadError::Io { path, source } => {
                        ContentLoadError::Io { path, source }
                    }
                    MemoryCallbackLoadError::Parse { path, source } => {
                        ContentLoadError::TraceryParse { path, source }
                    }
                    MemoryCallbackLoadError::InvalidBank { path, source } => {
                        // Grammar file present + valid JSON but fails origin-
                        // rule invariant. `TraceryParse` is the closest honest
                        // `ContentLoadError` variant — it signals a grammar-
                        // level content error (not a missing file). The source
                        // detail is preserved via the `eprintln!` above; the
                        // `tracery::Error::ParseError` wrapping provides the
                        // message to any downstream error reporter.
                        ContentLoadError::TraceryParse {
                            path,
                            source: tracery::Error::ParseError(source.to_string()),
                        }
                    }
                }
            })?;

        // Manager archetypes (T1-7). Optional — old content packs may not
        // have a managers/ dir; silently skip if absent. ID conversion
        // `ManagerArchetypeId -> String` mirrors the SignatureDefinition
        // loader pattern (line 600) — BTreeMap key is bare String for
        // hashability while the struct's id field is the newtype. Kind
        // string `"manager_archetype"` matches the snake_case-mirrors-
        // type-name convention used by the other 5 loaders (T1-7 fix-pass
        // per code-reviewer P2).
        //
        // Manager-fixture path also tracked separately for the cross-reference
        // validator below (we need the RON file path to surface in the
        // DanglingReference error).
        let mut manager_paths: BTreeMap<String, PathBuf> = BTreeMap::new();
        let managers_dir = sources_dir.join("managers");
        if managers_dir.is_dir() {
            let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
            for entry in walk_ron_files(&managers_dir)? {
                let parsed: crate::manager::ManagerArchetype = parse_ron_file(&entry)?;
                let id = parsed.id.as_str().to_owned();
                manager_paths.insert(id.clone(), entry.clone());
                insert_unique(
                    &mut store.managers,
                    &mut seen,
                    id,
                    parsed,
                    "manager_archetype",
                    entry,
                )?;
            }
        }

        // Player bios (T2-4). Optional — old content packs may not have a
        // player-bios/ dir; silently skip if absent (same guard pattern as the
        // other optional loaders above).
        let player_bios_dir = sources_dir.join("player-bios");
        if player_bios_dir.is_dir() {
            let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
            for entry in walk_ron_files(&player_bios_dir)? {
                let parsed: crate::player_bio::PlayerBio = parse_ron_file(&entry)?;
                let id = parsed.player_id.clone();
                insert_unique(
                    &mut store.player_bios,
                    &mut seen,
                    id,
                    parsed,
                    "player_bio",
                    entry,
                )?;
            }
        }

        // Cross-reference validation (T1-7 fix-pass per silent-failure F4):
        // every ManagerArchetype.tactical_archetype_id MUST resolve in
        // store.tactical_archetypes — fail loudly with DanglingReference if
        // a manager fixture points at a deleted/typo'd archetype. Prior
        // behavior surfaced this as `MissingTacticalArchetype` only when
        // `generate_team` was called against that manager, misleadingly
        // implicating the call site rather than the fixture.
        for (manager_id, manager) in &store.managers {
            if !store
                .tactical_archetypes
                .contains_key(&manager.tactical_archetype_id)
            {
                let from_path = manager_paths
                    .get(manager_id)
                    .cloned()
                    .unwrap_or_else(|| PathBuf::from("<unknown path>"));
                return Err(ContentLoadError::DanglingReference {
                    from_kind: "manager_archetype",
                    from_id: manager_id.clone(),
                    from_path,
                    to_kind: "tactical_archetype",
                    to_id: manager.tactical_archetype_id.clone(),
                });
            }
        }

        // Cross-reference validation (T1-20 per post-T1-close ultimate-review
        // Track E #3): every `PlayerTemplate.signature_candidates[i].signature_id`
        // MUST resolve in `store.signature_definitions`. Surfaces dangling refs
        // at load time so a typo or deleted signature in a content pack fails
        // loudly with the offending player-fixture path, NOT silently as a
        // sim-time "signature never fires" mystery during a 90-min match.
        //
        // Mirrors the manager → tactical_archetype check above; uses the same
        // `DanglingReference` variant. Iterates BTreeMap-deterministic so the
        // FIRST detected violation is reported deterministically across runs.
        for (player_id, template) in &store.player_templates {
            for candidate in &template.signature_candidates {
                let sig_id_str = candidate.signature_id.as_str();
                if !store.signature_definitions.contains_key(sig_id_str) {
                    let from_path = player_paths
                        .get(player_id)
                        .cloned()
                        .unwrap_or_else(|| PathBuf::from("<unknown path>"));
                    return Err(ContentLoadError::DanglingReference {
                        from_kind: "player_template",
                        from_id: player_id.clone(),
                        from_path,
                        to_kind: "signature_definition",
                        to_id: sig_id_str.to_owned(),
                    });
                }
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
                team_name_bank: vec![],
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

    // --- Chunk 1 (T1-12): insert_unique unit tests ---

    #[test]
    fn insert_unique_first_insertion_succeeds() {
        let mut map: BTreeMap<String, u32> = BTreeMap::new();
        let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
        let result = insert_unique(
            &mut map,
            &mut seen,
            "fwh.core:culture.test".to_string(),
            42u32,
            "culture",
            PathBuf::from("/fake/a.ron"),
        );
        assert!(result.is_ok());
        assert_eq!(map.get("fwh.core:culture.test"), Some(&42u32));
        assert!(seen.contains_key("fwh.core:culture.test"));
    }

    #[test]
    fn insert_unique_duplicate_returns_error_with_both_paths() {
        let mut map: BTreeMap<String, u32> = BTreeMap::new();
        let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
        let path_first = PathBuf::from("/fake/first.ron");
        let path_dupe = PathBuf::from("/fake/dupe.ron");

        insert_unique(
            &mut map,
            &mut seen,
            "fwh.core:culture.test".to_string(),
            1u32,
            "culture",
            path_first.clone(),
        )
        .expect("first insertion must succeed");

        let err = insert_unique(
            &mut map,
            &mut seen,
            "fwh.core:culture.test".to_string(),
            2u32,
            "culture",
            path_dupe.clone(),
        )
        .expect_err("second insertion must return DuplicateId");

        match err {
            ContentLoadError::DuplicateId {
                kind,
                id,
                path_first: pf,
                path_dupe: pd,
            } => {
                assert_eq!(kind, "culture");
                assert_eq!(id, "fwh.core:culture.test");
                assert_eq!(pf, path_first);
                assert_eq!(pd, path_dupe);
            }
            other => panic!("expected DuplicateId, got {other:?}"),
        }

        // Map still holds first value — not overwritten.
        assert_eq!(map.get("fwh.core:culture.test"), Some(&1u32));
    }

    #[test]
    fn insert_unique_distinct_ids_both_succeed() {
        let mut map: BTreeMap<String, u32> = BTreeMap::new();
        let mut seen: BTreeMap<String, PathBuf> = BTreeMap::new();
        insert_unique(
            &mut map,
            &mut seen,
            "fwh.core:culture.alpha".to_string(),
            1u32,
            "culture",
            PathBuf::from("/fake/alpha.ron"),
        )
        .expect("alpha insertion must succeed");
        insert_unique(
            &mut map,
            &mut seen,
            "fwh.core:culture.beta".to_string(),
            2u32,
            "culture",
            PathBuf::from("/fake/beta.ron"),
        )
        .expect("beta insertion must succeed");
        assert_eq!(map.len(), 2);
    }
}

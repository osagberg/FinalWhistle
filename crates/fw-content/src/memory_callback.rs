//! Memory-callback prose renderer — T3-6 part 1.
//!
//! Turns a career memory event into one sentence of football-grade callback
//! prose — the player-facing "remembered moment" shown in press conferences,
//! fan callbacks, and scout reports.
//!
//! ## Design
//!
//! Mirrors `news.rs` discipline exactly:
//!
//! - Caller resolves IDs → display strings; `fw-content` has NO dependency on
//!   `fw-memory`. All slots arrive as already-resolved `String` fields in
//!   `MemoryCallbackContext`.
//! - `MemoryCallbackGrammarBank` stores raw `BTreeMap<String, Vec<String>>`
//!   rule maps (not compiled `tracery::Grammar`) so per-render variable
//!   injection is clean.
//! - `render_memory_callback` seeds `ChaCha8Rng` from
//!   `seed_fn(career_id, event_id, SeedLayer::Commentary, SITE_MEMORY_CALLBACK)`
//!   where `SITE_MEMORY_CALLBACK = 2` (0 = headline, 1 = quote — per `news.rs`).
//! - Returns `Result<String, NewsRenderError>` — NEVER `unwrap_or_default`.
//!
//! ## Grammar family mapping
//!
//! One unified grammar file (`memory-callback.tracery.json`) with per-class
//! rule families keyed by EventClass name. The `u32` discriminant is mapped to
//! a `&'static str` family key at render time via `discriminant_to_family_key`.
//! Discriminants 0–29 map to class-specific families; discriminant 30
//! (`UnknownEventClass`) is NOT registered — callers should treat it as
//! unrenderable and fall back to a generic phrase if needed.
//!
//! ## Seed derivation
//!
//! `site = SITE_MEMORY_CALLBACK` (2). Same `(career_id, event_id)` produces
//! three independent RNG streams: headline (site 0), manager quote (site 1),
//! memory callback (site 2). The streams do not lock-step.
//!
//! ## Context fields
//!
//! See `MemoryCallbackContext` for the full field contract and what each field
//! is expected to contain. The `ui-programmer` dispatch resolves a `MemoryEvent`
//! into this struct; `fw-content` never touches `MemoryEvent` directly.
//!
//! ## Vocabulary
//!
//! Football-native only. No capitalised state-nouns. No "+5 Finishing" tooltips.
//! All copy must pass `scripts/fw banned-terms`.

use std::collections::BTreeMap;
use std::path::Path;

use fw_core::seed::{SeedLayer, seed_fn};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::news::NewsRenderError;

// ---------------------------------------------------------------------------
// Site constant
// ---------------------------------------------------------------------------

/// `site` discriminator for `seed_fn` — memory-callback renders.
///
/// 0 = headline (news.rs), 1 = manager quote (news.rs), 2 = this.
pub const SITE_MEMORY_CALLBACK: u32 = 2;

// ---------------------------------------------------------------------------
// Grammar-bank key
// ---------------------------------------------------------------------------

/// Single grammar file that holds all 30 per-class rule families.
const GRAMMAR_FILENAME: &str = "memory-callback.tracery.json";

// ---------------------------------------------------------------------------
// Discriminant → family-key table (0–29)
// ---------------------------------------------------------------------------

/// Map a `MemoryEvent::event_class.discriminant()` value to the Tracery rule
/// family key in the grammar.
///
/// Returns `None` for discriminant 30 (`UnknownEventClass`) — that variant
/// has no dedicated family; callers may fall back to a generic phrase.
///
/// **This table is the pinnable source of truth.** Adding a new core
/// `EventClass` variant at discriminant N requires a matching entry here AND
/// a matching rule family in `memory-callback.tracery.json`, otherwise the
/// `has_family_for(N)` AC4 pin test fails loudly.
#[must_use]
pub fn discriminant_to_family_key(discriminant: u32) -> Option<&'static str> {
    match discriminant {
        // Performance moments
        0 => Some("breakthrough_moment"),
        1 => Some("signature_first_fired"),
        2 => Some("legacy_goal"),
        3 => Some("hat_trick_scored"),
        4 => Some("big_match_scar"),
        5 => Some("regressive_collapse"),
        // Contract / transfer arc
        6 => Some("promised_youth_minutes"),
        7 => Some("broken_promise"),
        8 => Some("contract_renewal_rejected"),
        9 => Some("contract_renewal_accepted"),
        10 => Some("transfer_requested"),
        11 => Some("transfer_refused"),
        12 => Some("sold_under_protest"),
        13 => Some("bought_on_deadline_day"),
        // Relational
        14 => Some("rivalry_formed"),
        15 => Some("mentor_teammate"),
        16 => Some("derby_controversy"),
        17 => Some("former_club_reunion"),
        // Competition arc
        18 => Some("cup_final_win"),
        19 => Some("cup_final_loss"),
        20 => Some("promotion_won"),
        21 => Some("relegation_suffered"),
        22 => Some("title_won"),
        23 => Some("unbeaten_run_ended"),
        // Career-shape
        24 => Some("debut_senior"),
        25 => Some("debut_club"),
        26 => Some("retirement"),
        27 => Some("injury_long_term"),
        28 => Some("international_call_up"),
        // System
        29 => Some("compaction"),
        // 30 = UnknownEventClass — no dedicated family
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// MemoryCallbackContext
// ---------------------------------------------------------------------------

/// Caller-resolved substitution variables for a memory-callback sentence.
///
/// All fields are already-resolved display strings. ID → name resolution is a
/// caller concern (the `ui-programmer` dispatch); `fw-content` has NO
/// dependency on `fw-memory`.
///
/// Field names map directly to Tracery substitution variables in
/// `memory-callback.tracery.json`:
///
/// | Field            | Tracery var          | Expected content                                              |
/// |------------------|----------------------|---------------------------------------------------------------|
/// | `player_name`    | `#player_name#`      | Full display name of the event's primary subject.             |
/// | `club_name`      | `#club_name#`        | Club the player was at when the event happened.               |
/// | `opponent_name`  | `#opponent_name#`    | Opposing club or person (empty string if none relevant).      |
/// | `competition_name` | `#competition_name#` | Cup or competition name (empty string if none relevant).    |
/// | `season_label`   | `#season_label#`     | Short human-readable season label, e.g. "Season 4".           |
/// | `score_line`     | `#score_line#`       | Final score, e.g. "2-1" (empty string if not a match event). |
/// | `outcome_phrase` | `#outcome_phrase#`   | Short outcome description, e.g. "a late winner".              |
/// | `role_label`     | `#role_label#`       | Player's position/role label, e.g. "striker".                 |
/// | `detail_phrase`  | `#detail_phrase#`    | A free-text detail phrase, e.g. "after being written off".    |
///
/// The `ui-programmer` dispatch must populate ALL fields. Empty strings are
/// acceptable for contextually irrelevant slots (Tracery will substitute an
/// empty string — the grammar is authored to remain coherent when a slot is
/// empty, e.g. opponent_name is empty for transfer events).
#[derive(Debug, Clone)]
pub struct MemoryCallbackContext {
    /// Full display name of the event's primary subject player.
    pub player_name: String,
    /// Club the player was at when the event happened.
    pub club_name: String,
    /// Opposing club or person; empty string if not relevant to the event class.
    pub opponent_name: String,
    /// Cup or competition name; empty string if not relevant.
    pub competition_name: String,
    /// Short human-readable season label, e.g. "Season 4".
    pub season_label: String,
    /// Final score string, e.g. "2-1"; empty if not a match event.
    pub score_line: String,
    /// Short outcome description, e.g. "a late winner", "comfortable win".
    pub outcome_phrase: String,
    /// Player's position/role label, e.g. "striker", "central midfielder".
    pub role_label: String,
    /// Free-text detail phrase for narrative texture, e.g. "after being written off".
    pub detail_phrase: String,
}

// ---------------------------------------------------------------------------
// MemoryCallbackGrammarBank
// ---------------------------------------------------------------------------

/// Holds the raw Tracery rule map for the single memory-callback grammar.
///
/// Mirrors `NewsGrammarBank` discipline:
/// - Stores `BTreeMap<String, Vec<String>>` — raw rules, not a compiled Grammar.
///   Per-render Grammar is rebuilt (clean external-API path; Grammar is not
///   `Serialize` and the only mutation API is `pub(crate)`).
/// - Construction-time guard via `try_from_rules`: rejects a missing or
///   all-empty-string `origin` rule at build time, not silently at render time.
///
/// **Invariant:** `rules` has a non-empty `origin` rule with ≥1 non-empty
/// variant. Every discriminant 0–29 has a corresponding family rule key in
/// `rules`. Enforced by `try_from_rules` + the AC4 pin test.
///
/// `BTreeMap` for deterministic iteration per `Sim/RULES.md §2`.
#[derive(Debug, Clone)]
pub struct MemoryCallbackGrammarBank {
    rules: BTreeMap<String, Vec<String>>,
}

/// Error returned when a `MemoryCallbackGrammarBank` cannot be constructed.
#[derive(Debug)]
pub enum MemoryCallbackBankBuildError {
    /// The rule map has no `origin` entry.
    MissingOriginRule,
    /// The `origin` entry is an empty `Vec`.
    EmptyOriginRule,
    /// All `origin` variants are empty strings.
    AllEmptyOriginVariants,
}

impl std::fmt::Display for MemoryCallbackBankBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingOriginRule => write!(
                f,
                "memory-callback grammar has no `origin` rule (required by renderer)"
            ),
            Self::EmptyOriginRule => write!(
                f,
                "memory-callback grammar has an empty `origin` rule (Vec::new())"
            ),
            Self::AllEmptyOriginVariants => write!(
                f,
                "memory-callback grammar has only empty-string `origin` variants"
            ),
        }
    }
}

impl std::error::Error for MemoryCallbackBankBuildError {}

impl MemoryCallbackGrammarBank {
    /// Construct from a raw Tracery rule map.
    ///
    /// Rejects missing/empty `origin` rule at construction time (fail-loud,
    /// mirrors `NewsGrammarBank::try_from_map`).
    #[must_use = "discarding the Result silently drops a bank-build failure"]
    pub fn try_from_rules(
        rules: BTreeMap<String, Vec<String>>,
    ) -> Result<Self, MemoryCallbackBankBuildError> {
        let Some(origin) = rules.get("origin") else {
            return Err(MemoryCallbackBankBuildError::MissingOriginRule);
        };
        if origin.is_empty() {
            return Err(MemoryCallbackBankBuildError::EmptyOriginRule);
        }
        if origin.iter().all(|v| v.is_empty()) {
            return Err(MemoryCallbackBankBuildError::AllEmptyOriginVariants);
        }
        Ok(Self { rules })
    }

    /// Load from the `memory-callback.tracery.json` file in `grammars_dir`.
    ///
    /// Returns `Err` if the file is absent, not valid JSON, or fails the
    /// origin-rule invariant check.
    #[must_use = "discarding the Result silently drops a load failure"]
    pub fn load_from_dir(grammars_dir: &Path) -> Result<Self, MemoryCallbackLoadError> {
        let path = grammars_dir.join(GRAMMAR_FILENAME);
        if !path.exists() {
            return Err(MemoryCallbackLoadError::MissingFile(path.to_path_buf()));
        }
        let raw_json =
            std::fs::read_to_string(&path).map_err(|source| MemoryCallbackLoadError::Io {
                path: path.clone(),
                source,
            })?;

        // Parse as `BTreeMap<String, serde_json::Value>` first (same pattern
        // as `load_narrative_grammars` in runtime.rs) so `"_comment"` array
        // entries are handled correctly. Array-valued entries → `Vec<String>`;
        // non-array entries (bare strings, objects) are skipped.
        let raw_map: BTreeMap<String, serde_json::Value> = serde_json::from_str(&raw_json)
            .map_err(|e| MemoryCallbackLoadError::Parse {
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
                _ => None,
            })
            .collect();

        // Construction-validation failure is NOT an I/O error — the file was
        // readable and valid JSON; the grammar content is semantically invalid
        // (e.g. missing origin rule). Map to the honest `InvalidBank` variant.
        Self::try_from_rules(rules)
            .map_err(|source| MemoryCallbackLoadError::InvalidBank { path, source })
    }

    /// Returns `true` if discriminant `d` maps to a family key that exists in
    /// the loaded rules.
    ///
    /// Used by the AC4 pin test: every discriminant 0–29 must return `true`,
    /// and discriminant 30 must return `false`.
    #[must_use]
    pub fn has_family_for(&self, discriminant: u32) -> bool {
        match discriminant_to_family_key(discriminant) {
            Some(key) => self.rules.contains_key(key),
            None => false,
        }
    }
}

/// Error loading a `MemoryCallbackGrammarBank`.
#[derive(Debug)]
pub enum MemoryCallbackLoadError {
    MissingFile(std::path::PathBuf),
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: std::path::PathBuf,
        source: tracery::Error,
    },
    /// The grammar file was present and valid JSON, but failed the
    /// `MemoryCallbackGrammarBank` construction-time origin-rule invariant
    /// (e.g. missing `origin` key, empty origin, all-empty-string variants).
    /// Distinct from `Io` (file readable) and `Parse` (valid JSON) — the
    /// issue is the semantic content of the grammar, not the I/O layer.
    InvalidBank {
        path: std::path::PathBuf,
        source: MemoryCallbackBankBuildError,
    },
}

impl std::fmt::Display for MemoryCallbackLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingFile(p) => write!(f, "missing memory-callback grammar: {}", p.display()),
            Self::Io { path, source } => {
                write!(f, "I/O error reading {}: {}", path.display(), source)
            }
            Self::Parse { path, source } => {
                write!(f, "parse error in {}: {}", path.display(), source)
            }
            Self::InvalidBank { path, source } => write!(
                f,
                "memory-callback grammar at {} failed origin-rule check: {}",
                path.display(),
                source
            ),
        }
    }
}

impl std::error::Error for MemoryCallbackLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidBank { source, .. } => Some(source),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// render_memory_callback
// ---------------------------------------------------------------------------

/// Render a memory-callback sentence for a career event.
///
/// `event_class_discriminant` is the `fw_memory::EventClass::discriminant()`
/// value (a `u32`). `fw-content` maps it to a grammar family key internally,
/// keeping `fw-content` free of any `fw-memory` dependency.
///
/// ## Determinism
///
/// Same `(career_id, event_id, event_class_discriminant, ctx)` → byte-identical
/// output on every platform. `ChaCha8Rng` is seeded from
/// `seed_fn(career_id, event_id, SeedLayer::Commentary, SITE_MEMORY_CALLBACK)`.
/// No `thread_rng`, no `OsRng`.
///
/// ## Errors
///
/// Returns `NewsRenderError::Tracery` if Tracery raises an error (content-
/// authoring typo: missing rule referenced from origin). Returns
/// `NewsRenderError::EmptyOutput` if the output string is empty. Never
/// `unwrap_or_default` — callers decide how to degrade.
///
/// Returns `NewsRenderError::Tracery` with a descriptive message if
/// `event_class_discriminant` has no registered family (discriminant 30 /
/// `UnknownEventClass`); callers are expected to guard against this before
/// calling.
#[must_use = "discarding the Result silently drops a render failure"]
pub fn render_memory_callback(
    career_id: u64,
    event_id: u32,
    event_class_discriminant: u32,
    ctx: &MemoryCallbackContext,
    bank: &MemoryCallbackGrammarBank,
) -> Result<String, NewsRenderError> {
    let family_key = discriminant_to_family_key(event_class_discriminant).ok_or_else(|| {
        // No family registered for this discriminant (e.g. UnknownEventClass = 30).
        NewsRenderError::Tracery {
            grammar_key: "memory_callback",
            source: tracery::Error::ParseError(format!(
                "no memory-callback family for discriminant {event_class_discriminant} \
                 (UnknownEventClass has no dedicated callback family)"
            )),
        }
    })?;

    let derived = seed_fn(
        career_id,
        event_id,
        SeedLayer::Commentary,
        SITE_MEMORY_CALLBACK,
    );
    let mut rng = ChaCha8Rng::seed_from_u64(derived);

    let vars = callback_vars(ctx);

    // Build a merged rule map: start from the full bank, override the `origin`
    // rule to point at the family key for this discriminant, then inject all
    // context vars as single-entry rules.
    //
    // Strategy: replace `origin` with a single-variant rule pointing to the
    // family key's rule. This ensures the grammar only generates text from
    // the discriminant's family, not from the generic `callback` dispatch.
    let single_origin: Vec<String> = vec![format!("#{family_key}#")];

    let var_names: BTreeMap<&str, ()> = vars.iter().map(|(k, _)| (k.as_str(), ())).collect();

    let mut merged: Vec<(String, Vec<String>)> = bank
        .rules
        .iter()
        .filter(|(k, _)| k.as_str() != "origin" && !var_names.contains_key(k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    // Inject the discriminant-specific origin.
    merged.push(("origin".to_string(), single_origin));

    // Inject context vars as single-entry rules.
    // Tracery's `from_map` rejects empty rule strings via its PEG parser
    // ("expected rule" on `""`). Substitute a single space for any empty
    // context slot so the grammar can expand — an empty slot in the context
    // means the event has no value for that field (e.g. no player name on a
    // club-subject event like TitleWon). The resulting phrase will contain a
    // space in place of the slot, which is correct: phrases that omit the
    // slot (e.g. "club won the title") won't use it at all.
    for (k, v) in &vars {
        let safe_v = if v.is_empty() {
            " ".to_string()
        } else {
            v.clone()
        };
        merged.push((k.clone(), vec![safe_v]));
    }

    let grammar =
        tracery::Grammar::from_map(merged).map_err(|source| NewsRenderError::Tracery {
            grammar_key: "memory_callback",
            source,
        })?;

    let output = grammar
        .flatten(&mut rng)
        .map_err(|source| NewsRenderError::Tracery {
            grammar_key: "memory_callback",
            source,
        })?;

    // Collapse whitespace runs + trim. The empty-slot `" "` substitution above
    // (the workaround for Tracery rejecting empty rules) leaves a stray space
    // wherever a grammar variant references a slot the event has no value for
    // (e.g. `#player_name#` on a club-subject TitleWon) — which would otherwise
    // surface as a visible "…and   was part of it" gap in a player-facing
    // headline. Normalising to single-spaced + trimmed keeps the prose clean;
    // the empty-output guard below still fires if a phrase collapses to nothing.
    let output = output.split_whitespace().collect::<Vec<_>>().join(" ");

    if output.is_empty() {
        return Err(NewsRenderError::EmptyOutput {
            grammar_key: "memory_callback",
        });
    }

    Ok(output)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build the substitution variable list from a `MemoryCallbackContext`.
fn callback_vars(ctx: &MemoryCallbackContext) -> Vec<(String, String)> {
    vec![
        ("player_name".into(), ctx.player_name.clone()),
        ("club_name".into(), ctx.club_name.clone()),
        ("opponent_name".into(), ctx.opponent_name.clone()),
        ("competition_name".into(), ctx.competition_name.clone()),
        ("season_label".into(), ctx.season_label.clone()),
        ("score_line".into(), ctx.score_line.clone()),
        ("outcome_phrase".into(), ctx.outcome_phrase.clone()),
        ("role_label".into(), ctx.role_label.clone()),
        ("detail_phrase".into(), ctx.detail_phrase.clone()),
    ]
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn minimal_rules() -> BTreeMap<String, Vec<String>> {
        let mut rules: BTreeMap<String, Vec<String>> = BTreeMap::new();
        // origin must point to callback; callback dispatches all families.
        rules.insert("origin".into(), vec!["#callback#".into()]);
        rules.insert("callback".into(), vec!["#breakthrough_moment#".into()]);
        // One representative family with 3 variants.
        rules.insert(
            "breakthrough_moment".into(),
            vec![
                "#player_name# clicked at #club_name# variant-A".into(),
                "#player_name# found another gear at #club_name# variant-B".into(),
                "#player_name# stopped doubting at #club_name# variant-C".into(),
            ],
        );
        rules
    }

    fn sample_ctx() -> MemoryCallbackContext {
        MemoryCallbackContext {
            player_name: "Vale".into(),
            club_name: "Hartside".into(),
            opponent_name: "Breck City".into(),
            competition_name: "Northern Cup".into(),
            season_label: "Season 2".into(),
            score_line: "1-0".into(),
            outcome_phrase: "a late winner".into(),
            role_label: "striker".into(),
            detail_phrase: "under pressure".into(),
        }
    }

    // ---- construction-time guard ----

    #[test]
    fn try_from_rules_rejects_missing_origin() {
        let rules: BTreeMap<String, Vec<String>> = BTreeMap::new();
        assert!(
            MemoryCallbackGrammarBank::try_from_rules(rules).is_err(),
            "must reject a rule map with no origin"
        );
    }

    #[test]
    fn try_from_rules_rejects_empty_origin_vec() {
        let mut rules: BTreeMap<String, Vec<String>> = BTreeMap::new();
        rules.insert("origin".into(), vec![]);
        assert!(
            MemoryCallbackGrammarBank::try_from_rules(rules).is_err(),
            "must reject empty origin Vec"
        );
    }

    #[test]
    fn try_from_rules_rejects_all_empty_variants() {
        let mut rules: BTreeMap<String, Vec<String>> = BTreeMap::new();
        rules.insert("origin".into(), vec!["".into()]);
        assert!(
            MemoryCallbackGrammarBank::try_from_rules(rules).is_err(),
            "must reject all-empty-string origin variants"
        );
    }

    // ---- render path with minimal bank ----

    #[test]
    fn render_non_empty_for_discriminant_0() {
        let bank = MemoryCallbackGrammarBank::try_from_rules(minimal_rules()).expect("build bank");
        let ctx = sample_ctx();
        let result = render_memory_callback(0xCAFE, 1, 0, &ctx, &bank)
            .expect("render must succeed for discriminant 0");
        assert!(!result.is_empty(), "output was empty");
    }

    #[test]
    fn render_deterministic_for_discriminant_0() {
        let bank = MemoryCallbackGrammarBank::try_from_rules(minimal_rules()).expect("build bank");
        let ctx = sample_ctx();
        let a = render_memory_callback(0xCAFE, 1, 0, &ctx, &bank).expect("first render");
        let b = render_memory_callback(0xCAFE, 1, 0, &ctx, &bank).expect("second render");
        assert_eq!(a, b, "non-deterministic output for same inputs");
    }

    #[test]
    fn render_substitutes_player_name() {
        let bank = MemoryCallbackGrammarBank::try_from_rules(minimal_rules()).expect("build bank");
        let ctx = sample_ctx();
        let output = render_memory_callback(0, 0, 0, &ctx, &bank).expect("render");
        assert!(
            output.contains("Vale"),
            "player_name 'Vale' not found in output: {output:?}"
        );
    }

    #[test]
    fn render_with_empty_slots_succeeds_and_has_no_stray_whitespace() {
        // Regression (T4-2.5k): empty context slots used to make Tracery's
        // `from_map` reject the rule ("expected rule" on `""`), and every caller
        // silently swallowed the Err into the generic "a notable moment in the
        // career" fallback — so get_player_detail / get_career_overview / the
        // press inbox NEVER rendered real prose (they always pass several empty
        // slots). The `" "`-for-empty workaround unblocks rendering; the
        // whitespace-collapse keeps a referenced-but-empty slot (e.g.
        // `#player_name#` on a club-subject event) from leaving a visible "  "
        // gap in a player-facing headline. This test would FAIL before the fix
        // (Err → no render) AND would catch a regression of the stray-space.
        let bank = MemoryCallbackGrammarBank::try_from_rules(minimal_rules()).expect("build bank");
        let mut ctx = sample_ctx();
        // The minimal grammar's variants all LEAD with `#player_name#`, so an
        // empty player_name is exactly the stray-leading-space case.
        ctx.player_name = String::new();
        ctx.opponent_name = String::new();

        let output = render_memory_callback(0xCAFE, 1, 0, &ctx, &bank)
            .expect("render must SUCCEED with empty slots (no silent fallback)");

        assert!(!output.is_empty(), "output was empty");
        assert!(
            !output.contains("  "),
            "stray double-space (empty-slot artifact) in output: {output:?}"
        );
        assert_eq!(
            output,
            output.trim(),
            "leading/trailing whitespace (empty-slot artifact) in output: {output:?}"
        );
    }

    #[test]
    fn render_variant_spread_across_seeds() {
        let bank = MemoryCallbackGrammarBank::try_from_rules(minimal_rules()).expect("build bank");
        let ctx = sample_ctx();
        let outputs: Vec<String> = (0u64..30)
            .map(|seed| {
                render_memory_callback(seed, 0, 0, &ctx, &bank).expect("render must succeed")
            })
            .collect();
        let unique: BTreeSet<&str> = outputs.iter().map(String::as_str).collect();
        assert!(
            unique.len() >= 3,
            "fewer than 3 variants across 30 seeds; got: {unique:?}"
        );
    }

    #[test]
    fn render_unknown_discriminant_errors() {
        let bank = MemoryCallbackGrammarBank::try_from_rules(minimal_rules()).expect("build bank");
        let ctx = sample_ctx();
        // Discriminant 30 = UnknownEventClass — no family registered.
        let result = render_memory_callback(0, 0, 30, &ctx, &bank);
        assert!(
            result.is_err(),
            "discriminant 30 must return Err (no dedicated family)"
        );
    }

    // ---- discriminant_to_family_key coverage ----

    #[test]
    fn all_core_discriminants_have_a_family_key() {
        for d in 0u32..=29 {
            assert!(
                discriminant_to_family_key(d).is_some(),
                "discriminant {d} has no family key — table is incomplete"
            );
        }
    }

    #[test]
    fn discriminant_30_has_no_family_key() {
        assert!(
            discriminant_to_family_key(30).is_none(),
            "discriminant 30 (UnknownEventClass) must not have a family key"
        );
    }
}

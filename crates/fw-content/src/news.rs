//! News headline + manager-quote renderer — T3-3.
//!
//! Deterministic Tracery-backed render of player-facing news headlines and
//! press-conference manager quotes. Mirrors the structural discipline of
//! `crates/fw-content/src/commentary.rs` (T1-4b):
//!
//! - Raw rule maps (`BTreeMap<String, Vec<String>>`) stored, not compiled grammars.
//! - `NewsGrammarBank::try_from_map` fails loud on missing / empty `origin` rules.
//! - `render_headline` / `render_manager_quote` seed `ChaCha8Rng` from
//!   `seed_fn(career_id, event_id, SeedLayer::Commentary, site)` — no `thread_rng`.
//! - Both functions return `Result<String, NewsRenderError>` — NEVER `unwrap_or_default`.
//!
//! ## Grammar keys
//!
//! The bank holds two fixed keys: `"headlines"` (origin rule for headlines) and
//! `"manager_quotes"` (origin rule for manager quotes). Both load from
//! `content/sources/grammars/` — see `runtime::load_narrative_grammars`.
//!
//! ## Seed derivation
//!
//! `site = 0` → headline render.
//! `site = 1` → manager-quote render.
//!
//! Same `(career_id, event_id)` therefore produces two independent RNG streams
//! (distinct `site` in `seed_fn`), so the headline and quote for the same event
//! do not lock-step in their variant picks.
//!
//! ## Substitution variables
//!
//! | Render fn              | Caller-supplied vars                                          |
//! |------------------------|---------------------------------------------------------------|
//! | `render_headline`      | `team`, `opponent`, `player`, `scorer`, `score_line`, `minute`, `manager` |
//! | `render_manager_quote` | `manager`, `team`, `opponent`, `player`, `scorer`            |
//!
//! Variables in `HeadlineContext` / `QuoteContext` are already-resolved display
//! strings (player names, club names). ID → name resolution is a caller concern,
//! deferred to the wiring task (T3-6). `fw-content` has NO dependency on `fw-memory`.

use std::collections::BTreeMap;

use fw_core::seed::{SeedLayer, seed_fn};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

// ---------------------------------------------------------------------------
// Grammar-bank key constants
// ---------------------------------------------------------------------------

/// Key under which the headline grammar rules are stored in `NewsGrammarBank`.
const HEADLINE_KEY: &str = "headlines";

/// Key under which the manager-quote grammar rules are stored.
const QUOTE_KEY: &str = "manager_quotes";

/// `site` discriminator passed to `seed_fn` for headline renders.
const SITE_HEADLINE: u32 = 0;

/// `site` discriminator passed to `seed_fn` for manager-quote renders.
const SITE_QUOTE: u32 = 1;

// ---------------------------------------------------------------------------
// NewsGrammarBank
// ---------------------------------------------------------------------------

/// Holds the raw Tracery rule maps for the two news grammars (headlines +
/// manager quotes).
///
/// Mirrors `CommentaryGrammarBank` in `commentary.rs`:
/// - Stores `BTreeMap<String, Vec<String>>` — raw rules, not compiled grammars.
///   Compiled `tracery::Grammar` is not `Serialize`; per-render we inject
///   substitution variables as additional rules, requiring a Grammar rebuild
///   from the merged rule set.
/// - `try_from_map` is fail-loud: absent or malformed `origin` rules are
///   rejected at construction time.
///
/// **Invariant:** both `HEADLINE_KEY` and `QUOTE_KEY` entries are present and
/// each has a non-empty `origin` rule with ≥1 non-empty variant. Enforced by
/// `try_from_map`.
///
/// `BTreeMap` for deterministic iteration (`.claude/rules/Sim/RULES.md` §2).
#[derive(Debug, Clone)]
pub struct NewsGrammarBank {
    rules: BTreeMap<&'static str, BTreeMap<String, Vec<String>>>,
}

impl NewsGrammarBank {
    /// Construct from a map of grammar-key → raw rule map.
    ///
    /// Expects exactly two keys: `HEADLINE_KEY` (`"headlines"`) and
    /// `QUOTE_KEY` (`"manager_quotes"`). Returns `Err` if either is absent
    /// or its `origin` rule is missing / empty / all-empty-variants.
    ///
    /// Fail-loud per the `commentary.rs` construction-time guard discipline.
    #[must_use = "discarding the Result silently drops a bank-build failure"]
    pub fn try_from_map(
        map: BTreeMap<&'static str, BTreeMap<String, Vec<String>>>,
    ) -> Result<Self, NewsBankBuildError> {
        for key in [HEADLINE_KEY, QUOTE_KEY] {
            let Some(rules) = map.get(key) else {
                return Err(NewsBankBuildError::MissingGrammar(key));
            };
            let Some(origin_variants) = rules.get("origin") else {
                return Err(NewsBankBuildError::MissingOriginRule(key));
            };
            if origin_variants.is_empty() {
                return Err(NewsBankBuildError::EmptyOriginRule(key));
            }
            if origin_variants.iter().all(|v| v.is_empty()) {
                return Err(NewsBankBuildError::AllEmptyOriginVariants(key));
            }
        }
        Ok(Self { rules: map })
    }

    /// Build a `NewsGrammarBank` from two separate rule maps.
    ///
    /// Convenience constructor used by the loader: pass the headline rules
    /// and quote rules individually rather than pre-building the outer map.
    #[must_use = "discarding the Result silently drops a bank-build failure"]
    pub fn from_parts(
        headline_rules: BTreeMap<String, Vec<String>>,
        quote_rules: BTreeMap<String, Vec<String>>,
    ) -> Result<Self, NewsBankBuildError> {
        let mut map = BTreeMap::new();
        map.insert(HEADLINE_KEY, headline_rules);
        map.insert(QUOTE_KEY, quote_rules);
        Self::try_from_map(map)
    }

    /// Look up the raw rules for `key`.
    ///
    /// Panics if the key is missing — the `try_from_map` invariant guarantees
    /// both keys are present after valid construction. This is unreachable in
    /// correctly-constructed instances.
    fn get_rules(&self, key: &'static str) -> &BTreeMap<String, Vec<String>> {
        self.rules
            .get(key)
            .expect("NewsGrammarBank invariant: headline and quote rules always present")
    }
}

// ---------------------------------------------------------------------------
// NewsBankBuildError
// ---------------------------------------------------------------------------

/// Error returned by `NewsGrammarBank::try_from_map` when the bank cannot be
/// constructed cleanly.
///
/// Mirrors `CommentaryBankBuildError` — same four failure modes.
#[derive(Debug)]
pub enum NewsBankBuildError {
    /// Map does not contain an entry for this grammar key.
    MissingGrammar(&'static str),
    /// Grammar exists but has no `origin` rule.
    MissingOriginRule(&'static str),
    /// Grammar has an `origin` rule but it is an empty `Vec`.
    EmptyOriginRule(&'static str),
    /// Grammar has `origin` variants but all of them are empty strings.
    AllEmptyOriginVariants(&'static str),
}

impl std::fmt::Display for NewsBankBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingGrammar(k) => write!(f, "missing news grammar for key {k:?}"),
            Self::MissingOriginRule(k) => write!(
                f,
                "news grammar {k:?} has no `origin` rule (required by the renderer)"
            ),
            Self::EmptyOriginRule(k) => {
                write!(
                    f,
                    "news grammar {k:?} has an empty `origin` rule (Vec::new())"
                )
            }
            Self::AllEmptyOriginVariants(k) => write!(
                f,
                "news grammar {k:?} has only empty-string `origin` variants \
                 (Tracery would render to empty)"
            ),
        }
    }
}

impl std::error::Error for NewsBankBuildError {}

// ---------------------------------------------------------------------------
// NewsRenderError
// ---------------------------------------------------------------------------

/// Render-time failure for `render_headline` / `render_manager_quote`.
///
/// Mirrors `CommentaryRenderError`. NEVER swallowed by `unwrap_or_default` —
/// callers decide how to degrade: log + fall back to a generic line, OR
/// surface "(news unavailable)" in the UI.
#[derive(Debug)]
pub enum NewsRenderError {
    /// Tracery raised an error during render. Most common cause: template
    /// references a variable not injected by the caller (content-authoring
    /// typo in the `.tracery.json` file).
    Tracery {
        grammar_key: &'static str,
        source: tracery::Error,
    },
    /// Tracery succeeded but the output is an empty string. Most common
    /// cause: a grammar's `origin` rule has an empty-string variant or the
    /// variant-pick landed on one.
    EmptyOutput { grammar_key: &'static str },
}

impl std::fmt::Display for NewsRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tracery {
                grammar_key,
                source,
            } => {
                write!(
                    f,
                    "news render failed for grammar {grammar_key:?}: {source}"
                )
            }
            Self::EmptyOutput { grammar_key } => write!(
                f,
                "news render produced empty string for grammar {grammar_key:?} \
                 (content-authoring bug: check `origin` rule has non-empty variants)"
            ),
        }
    }
}

impl std::error::Error for NewsRenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Tracery { source, .. } => Some(source),
            Self::EmptyOutput { .. } => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Context structs
// ---------------------------------------------------------------------------

/// Caller-resolved substitution variables for a news headline.
///
/// All fields are already-resolved display strings (player names, club names).
/// ID → name resolution is a caller concern (T3-6 / career loop). `fw-content`
/// has NO dependency on `fw-memory`.
///
/// Field names map directly to Tracery substitution vars in the headline
/// grammar: `#team#`, `#opponent#`, `#player#`, `#scorer#`, `#score_line#`,
/// `#minute#`, `#manager#`.
#[derive(Debug, Clone)]
pub struct HeadlineContext {
    pub team: String,
    pub opponent: String,
    pub player: String,
    pub scorer: String,
    pub score_line: String,
    pub minute: String,
    pub manager: String,
}

/// Caller-resolved substitution variables for a manager press-conference quote.
///
/// Field names map to Tracery substitution vars: `#manager#`, `#team#`,
/// `#opponent#`, `#player#`, `#scorer#`.
#[derive(Debug, Clone)]
pub struct QuoteContext {
    pub manager: String,
    pub team: String,
    pub opponent: String,
    pub player: String,
    pub scorer: String,
}

// ---------------------------------------------------------------------------
// render_headline
// ---------------------------------------------------------------------------

/// Render a news headline for a career event.
///
/// Determinism: same `(ctx, career_id, event_id)` → byte-identical output,
/// every platform. `ChaCha8Rng` is seeded from
/// `seed_fn(career_id, event_id, SeedLayer::Commentary, SITE_HEADLINE)` —
/// no `thread_rng`, no `OsRng`.
///
/// **Returns `Result<String, NewsRenderError>`** — callers handle typed errors
/// explicitly; no silent empty-string fallback.
///
/// **`career_id` maps to the `match_seed` slot** in `seed_fn`; `event_id`
/// maps to the `tick` slot. This is consistent with the site-discriminator
/// contract: same `(career_id, event_id)` but `site = SITE_HEADLINE` (0) vs
/// `site = SITE_QUOTE` (1) → independent RNG streams.
#[must_use = "discarding the Result silently drops a render failure"]
pub fn render_headline(
    ctx: &HeadlineContext,
    career_id: u64,
    event_id: u32,
    bank: &NewsGrammarBank,
) -> Result<String, NewsRenderError> {
    let derived = seed_fn(career_id, event_id, SeedLayer::Commentary, SITE_HEADLINE);
    let mut rng = ChaCha8Rng::seed_from_u64(derived);

    let base_rules = bank.get_rules(HEADLINE_KEY);
    let vars = headline_vars(ctx);

    let output = render_with_vars(base_rules, &vars, &mut rng).map_err(|source| {
        NewsRenderError::Tracery {
            grammar_key: HEADLINE_KEY,
            source,
        }
    })?;

    if output.is_empty() {
        return Err(NewsRenderError::EmptyOutput {
            grammar_key: HEADLINE_KEY,
        });
    }

    Ok(output)
}

// ---------------------------------------------------------------------------
// render_manager_quote
// ---------------------------------------------------------------------------

/// Render a manager press-conference quote for a career event.
///
/// Same determinism contract as `render_headline` but uses `site = SITE_QUOTE`
/// (1), so the same `(career_id, event_id)` pair yields an independent RNG
/// stream. Headline and quote for the same event do not lock-step.
#[must_use = "discarding the Result silently drops a render failure"]
pub fn render_manager_quote(
    ctx: &QuoteContext,
    career_id: u64,
    event_id: u32,
    bank: &NewsGrammarBank,
) -> Result<String, NewsRenderError> {
    let derived = seed_fn(career_id, event_id, SeedLayer::Commentary, SITE_QUOTE);
    let mut rng = ChaCha8Rng::seed_from_u64(derived);

    let base_rules = bank.get_rules(QUOTE_KEY);
    let vars = quote_vars(ctx);

    let output = render_with_vars(base_rules, &vars, &mut rng).map_err(|source| {
        NewsRenderError::Tracery {
            grammar_key: QUOTE_KEY,
            source,
        }
    })?;

    if output.is_empty() {
        return Err(NewsRenderError::EmptyOutput {
            grammar_key: QUOTE_KEY,
        });
    }

    Ok(output)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build the substitution variable list for a headline render.
fn headline_vars(ctx: &HeadlineContext) -> Vec<(String, String)> {
    vec![
        ("team".into(), ctx.team.clone()),
        ("opponent".into(), ctx.opponent.clone()),
        ("player".into(), ctx.player.clone()),
        ("scorer".into(), ctx.scorer.clone()),
        ("score_line".into(), ctx.score_line.clone()),
        ("minute".into(), ctx.minute.clone()),
        ("manager".into(), ctx.manager.clone()),
    ]
}

/// Build the substitution variable list for a manager-quote render.
fn quote_vars(ctx: &QuoteContext) -> Vec<(String, String)> {
    vec![
        ("manager".into(), ctx.manager.clone()),
        ("team".into(), ctx.team.clone()),
        ("opponent".into(), ctx.opponent.clone()),
        ("player".into(), ctx.player.clone()),
        ("scorer".into(), ctx.scorer.clone()),
    ]
}

/// Merge `base_rules` with variable overrides, build a `Grammar`, and flatten.
///
/// Variables are injected as single-entry rules; they shadow any same-named
/// key in the template (pre-filtering base rules that collide with a var name,
/// following the `render_with_vars` discipline in `commentary.rs`).
///
/// Per-render Grammar construction is intentional — `Grammar` is `Clone` but
/// the only mutation API is `pub(crate)`. Storing raw rules and rebuilding is
/// the clean external-API path.
fn render_with_vars(
    base_rules: &BTreeMap<String, Vec<String>>,
    vars: &[(String, String)],
    rng: &mut ChaCha8Rng,
) -> Result<String, tracery::Error> {
    // Pre-filter base rules to remove any key that's about to be overridden
    // by a var. Mirrors `commentary.rs::render_with_vars` — removes the
    // ordering dependency on `Grammar::from_map`'s duplicate-key handling.
    let var_names: BTreeMap<&str, ()> = vars.iter().map(|(k, _)| (k.as_str(), ())).collect();
    let mut merged: Vec<(String, Vec<String>)> = base_rules
        .iter()
        .filter(|(k, _)| !var_names.contains_key(k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    for (k, v) in vars {
        merged.push((k.clone(), vec![v.clone()]));
    }

    let grammar = tracery::Grammar::from_map(merged)?;
    grammar.flatten(rng)
}

// ---------------------------------------------------------------------------
// Unit tests (TDD — written alongside the module)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    // ---- helpers ----

    fn two_variant_bank() -> NewsGrammarBank {
        let mut headline_rules: BTreeMap<String, Vec<String>> = BTreeMap::new();
        headline_rules.insert(
            "origin".into(),
            vec![
                "#team# beat #opponent# variant-A".into(),
                "#team# beat #opponent# variant-B".into(),
            ],
        );

        let mut quote_rules: BTreeMap<String, Vec<String>> = BTreeMap::new();
        quote_rules.insert(
            "origin".into(),
            vec![
                "#manager# quote variant-A".into(),
                "#manager# quote variant-B".into(),
            ],
        );

        NewsGrammarBank::from_parts(headline_rules, quote_rules)
            .expect("two_variant_bank must construct cleanly")
    }

    fn sample_headline_ctx() -> HeadlineContext {
        HeadlineContext {
            team: "Hartside".into(),
            opponent: "Breck City".into(),
            player: "Vale".into(),
            scorer: "Vale".into(),
            score_line: "2-1".into(),
            minute: "87".into(),
            manager: "Prescott".into(),
        }
    }

    fn sample_quote_ctx() -> QuoteContext {
        QuoteContext {
            manager: "Prescott".into(),
            team: "Hartside".into(),
            opponent: "Breck City".into(),
            player: "Vale".into(),
            scorer: "Vale".into(),
        }
    }

    // ---- AC2: render_headline non-empty ----

    #[test]
    fn render_headline_non_empty() {
        let bank = two_variant_bank();
        let ctx = sample_headline_ctx();
        let result = render_headline(&ctx, 0xC0FFEE, 7, &bank)
            .expect("render_headline must succeed for test bank");
        assert!(!result.is_empty(), "render_headline returned empty string");
    }

    // ---- AC2: render_headline deterministic ----

    #[test]
    fn render_headline_deterministic() {
        let bank = two_variant_bank();
        let ctx = sample_headline_ctx();
        let a = render_headline(&ctx, 0xC0FFEE, 7, &bank).expect("first render must succeed");
        let b = render_headline(&ctx, 0xC0FFEE, 7, &bank).expect("second render must succeed");
        assert_eq!(
            a, b,
            "render_headline produced different outputs for same inputs"
        );
    }

    // ---- AC2: render_headline substitutes team ----

    #[test]
    fn render_headline_substitutes_team() {
        let bank = two_variant_bank();
        let ctx = sample_headline_ctx();
        let result = render_headline(&ctx, 0xC0FFEE, 7, &bank).expect("render must succeed");
        assert!(
            result.contains("Hartside"),
            "expected ctx.team 'Hartside' to appear in headline; got: {result:?}"
        );
    }

    // ---- AC3: render_manager_quote non-empty ----

    #[test]
    fn render_manager_quote_non_empty() {
        let bank = two_variant_bank();
        let ctx = sample_quote_ctx();
        let result = render_manager_quote(&ctx, 0xC0FFEE, 7, &bank)
            .expect("render_manager_quote must succeed for test bank");
        assert!(
            !result.is_empty(),
            "render_manager_quote returned empty string"
        );
    }

    // ---- AC3: render_manager_quote deterministic ----

    #[test]
    fn render_manager_quote_deterministic() {
        let bank = two_variant_bank();
        let ctx = sample_quote_ctx();
        let a = render_manager_quote(&ctx, 0xC0FFEE, 7, &bank).expect("first render must succeed");
        let b = render_manager_quote(&ctx, 0xC0FFEE, 7, &bank).expect("second render must succeed");
        assert_eq!(
            a, b,
            "render_manager_quote produced different outputs for same inputs"
        );
    }

    // ---- AC3: render_manager_quote substitutes manager ----

    #[test]
    fn render_manager_quote_substitutes_manager() {
        let bank = two_variant_bank();
        let ctx = sample_quote_ctx();
        let result = render_manager_quote(&ctx, 0xC0FFEE, 7, &bank).expect("render must succeed");
        assert!(
            result.contains("Prescott"),
            "expected ctx.manager 'Prescott' to appear in quote; got: {result:?}"
        );
    }

    // ---- AC4: variant spread across seeds ----

    #[test]
    fn headline_variant_spread_across_seeds() {
        let bank = two_variant_bank();
        let ctx = sample_headline_ctx();
        let results: Vec<String> = (0u64..20)
            .map(|seed| {
                render_headline(&ctx, seed, 7, &bank).expect("render must succeed for test bank")
            })
            .collect();
        let unique: BTreeSet<&str> = results.iter().map(String::as_str).collect();
        assert!(
            unique.len() >= 2,
            "render_headline produced only 1 unique variant across 20 seeds — \
             RNG path may not be exercised. Got: {unique:?}"
        );
    }

    #[test]
    fn manager_quote_variant_spread_across_seeds() {
        let bank = two_variant_bank();
        let ctx = sample_quote_ctx();
        let results: Vec<String> = (0u64..20)
            .map(|seed| {
                render_manager_quote(&ctx, seed, 7, &bank)
                    .expect("render must succeed for test bank")
            })
            .collect();
        let unique: BTreeSet<&str> = results.iter().map(String::as_str).collect();
        assert!(
            unique.len() >= 2,
            "render_manager_quote produced only 1 unique variant across 20 seeds — \
             RNG path may not be exercised. Got: {unique:?}"
        );
    }

    // ---- AC5: site discriminator — headline and quote use independent RNG streams ----

    #[test]
    fn headline_and_quote_independent_rng_streams() {
        // With the same (career_id, event_id) but different site values (0 vs 1),
        // headline and quote RNG streams are independent. Test: across 20 seeds,
        // both renderers show variance (not locked in identical pick patterns).
        let bank = two_variant_bank();
        let headline_ctx = sample_headline_ctx();
        let quote_ctx = sample_quote_ctx();

        let headlines: Vec<String> = (0u64..20)
            .map(|seed| {
                render_headline(&headline_ctx, seed, 0, &bank)
                    .expect("headline render must succeed")
            })
            .collect();
        let quotes: Vec<String> = (0u64..20)
            .map(|seed| {
                render_manager_quote(&quote_ctx, seed, 0, &bank).expect("quote render must succeed")
            })
            .collect();

        let unique_headlines: BTreeSet<&str> = headlines.iter().map(String::as_str).collect();
        let unique_quotes: BTreeSet<&str> = quotes.iter().map(String::as_str).collect();

        // Both renderers must show variant spread — proves each uses its own RNG stream.
        assert!(
            unique_headlines.len() >= 2,
            "headlines did not vary across 20 seeds: {unique_headlines:?}"
        );
        assert!(
            unique_quotes.len() >= 2,
            "quotes did not vary across 20 seeds: {unique_quotes:?}"
        );

        // Also assert the two sequence patterns are NOT identical (they pick from
        // different rule sets so coincidental equality is implausible, but the
        // real insurance is the seed_fn site split).
        let headline_pattern: Vec<usize> = headlines
            .iter()
            .map(|h| if h.contains("variant-A") { 0 } else { 1 })
            .collect();
        let quote_pattern: Vec<usize> = quotes
            .iter()
            .map(|q| if q.contains("variant-A") { 0 } else { 1 })
            .collect();
        assert_ne!(
            headline_pattern, quote_pattern,
            "headline and quote pick patterns are identical across 20 seeds — \
             suggests site discriminator is not working (both using site=0)"
        );
    }

    // ---- Construction-time guard tests (mirrors CommentaryGrammarBank tests) ----

    #[test]
    fn try_from_map_rejects_missing_headline_grammar() {
        let mut map: BTreeMap<&'static str, BTreeMap<String, Vec<String>>> = BTreeMap::new();
        // Only quote key — headline missing.
        let mut quote_rules: BTreeMap<String, Vec<String>> = BTreeMap::new();
        quote_rules.insert("origin".into(), vec!["a quote".into()]);
        map.insert(QUOTE_KEY, quote_rules);

        let result = NewsGrammarBank::try_from_map(map);
        assert!(
            result.is_err(),
            "try_from_map must reject a bank missing the headline grammar"
        );
        matches!(
            result.unwrap_err(),
            NewsBankBuildError::MissingGrammar(HEADLINE_KEY)
        );
    }

    #[test]
    fn try_from_map_rejects_empty_origin_rule() {
        let mut headline_rules: BTreeMap<String, Vec<String>> = BTreeMap::new();
        headline_rules.insert("origin".into(), vec![]); // empty Vec
        let mut quote_rules: BTreeMap<String, Vec<String>> = BTreeMap::new();
        quote_rules.insert("origin".into(), vec!["a quote".into()]);

        let result = NewsGrammarBank::from_parts(headline_rules, quote_rules);
        assert!(
            result.is_err(),
            "try_from_map must reject a grammar with an empty origin Vec"
        );
        matches!(
            result.unwrap_err(),
            NewsBankBuildError::EmptyOriginRule(HEADLINE_KEY)
        );
    }

    #[test]
    fn try_from_map_rejects_all_empty_origin_variants() {
        let mut headline_rules: BTreeMap<String, Vec<String>> = BTreeMap::new();
        headline_rules.insert("origin".into(), vec!["".into()]); // all empty-string
        let mut quote_rules: BTreeMap<String, Vec<String>> = BTreeMap::new();
        quote_rules.insert("origin".into(), vec!["a quote".into()]);

        let result = NewsGrammarBank::from_parts(headline_rules, quote_rules);
        assert!(
            result.is_err(),
            "try_from_map must reject a grammar where all origin variants are empty strings"
        );
        matches!(
            result.unwrap_err(),
            NewsBankBuildError::AllEmptyOriginVariants(HEADLINE_KEY)
        );
    }
}

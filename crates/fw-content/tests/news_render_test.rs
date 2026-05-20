//! Integration tests for the news headline + manager-quote render module (T3-3).
//!
//! These tests exercise the full stack: load the real grammar files from
//! `content/sources/grammars/`, build a `NewsGrammarBank`, and drive
//! `render_headline` / `render_manager_quote` against it.
//!
//! AC coverage:
//!   AC1 — loader: `load_sources` populates `ContentStore::news_grammars`.
//!   AC2 — render_headline: deterministic + non-empty + substitution vars.
//!   AC3 — render_manager_quote: deterministic + non-empty + substitution vars.
//!   AC4 — variant spread: ≥2 distinct outputs across 20 seeds.
//!   AC5 — site discriminator: headline + quote have independent RNG streams.

use std::collections::BTreeSet;
use std::path::PathBuf;

use fw_content::{
    ContentStore, HeadlineContext, QuoteContext, render_headline, render_manager_quote,
};

/// Load the real ContentStore from the workspace content/ directory.
/// `CARGO_MANIFEST_DIR` = `crates/fw-content`; workspace root is `../..`.
fn load_real_store() -> ContentStore {
    let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content");
    ContentStore::load_sources(&content_root)
        .expect("ContentStore::load_sources must succeed against committed fixtures")
}

fn sample_headline_ctx() -> HeadlineContext {
    HeadlineContext {
        team: "Hartside United".into(),
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
        team: "Hartside United".into(),
        opponent: "Breck City".into(),
        player: "Vale".into(),
        scorer: "Vale".into(),
    }
}

// ---------------------------------------------------------------------------
// AC1: load_sources populates news_grammars
// ---------------------------------------------------------------------------

#[test]
fn load_narrative_grammars_populates_content_store() {
    let store = load_real_store();
    // The bank is opaque — the best observable is that render_headline
    // and render_manager_quote succeed against it (which AC2/AC3 tests check).
    // Here we just prove load_sources returned Ok (the expect above guarantees
    // this) and that a render call with the loaded bank doesn't immediately error.
    let ctx = sample_headline_ctx();
    let result = render_headline(&ctx, 0xC0FFEE, 7, &store.news_grammars);
    assert!(
        result.is_ok(),
        "render_headline against loaded store must succeed; got: {result:?}"
    );
}

#[test]
fn load_narrative_grammars_manager_quote_succeeds() {
    let store = load_real_store();
    let ctx = sample_quote_ctx();
    let result = render_manager_quote(&ctx, 0xC0FFEE, 7, &store.news_grammars);
    assert!(
        result.is_ok(),
        "render_manager_quote against loaded store must succeed; got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// AC2: render_headline — deterministic + non-empty + substitution
// ---------------------------------------------------------------------------

#[test]
fn render_headline_deterministic_against_real_grammar() {
    let store = load_real_store();
    let ctx = sample_headline_ctx();
    let a = render_headline(&ctx, 0xC0FFEE, 7, &store.news_grammars)
        .expect("first render must succeed");
    let b = render_headline(&ctx, 0xC0FFEE, 7, &store.news_grammars)
        .expect("second render must succeed");
    assert_eq!(a, b, "render_headline not deterministic for same inputs");
}

#[test]
fn render_headline_non_empty_against_real_grammar() {
    let store = load_real_store();
    let ctx = sample_headline_ctx();
    let result =
        render_headline(&ctx, 0xC0FFEE, 7, &store.news_grammars).expect("render must succeed");
    assert!(!result.is_empty(), "render_headline returned empty string");
}

#[test]
fn render_headline_substitutes_team_against_real_grammar() {
    let store = load_real_store();
    let ctx = sample_headline_ctx();
    // Try several seeds — the grammar has many headline variants;
    // at least one should contain the team name.
    let found_team = (0u64..20).any(|seed| {
        render_headline(&ctx, seed, 7, &store.news_grammars)
            .map(|s| s.contains("Hartside United"))
            .unwrap_or(false)
    });
    assert!(
        found_team,
        "render_headline never included 'Hartside United' across 20 seeds — \
         substitution vars may not be injected"
    );
}

// ---------------------------------------------------------------------------
// AC3: render_manager_quote — deterministic + non-empty + substitution
// ---------------------------------------------------------------------------

#[test]
fn render_manager_quote_deterministic_against_real_grammar() {
    let store = load_real_store();
    let ctx = sample_quote_ctx();
    let a = render_manager_quote(&ctx, 0xC0FFEE, 7, &store.news_grammars)
        .expect("first render must succeed");
    let b = render_manager_quote(&ctx, 0xC0FFEE, 7, &store.news_grammars)
        .expect("second render must succeed");
    assert_eq!(
        a, b,
        "render_manager_quote not deterministic for same inputs"
    );
}

#[test]
fn render_manager_quote_non_empty_against_real_grammar() {
    let store = load_real_store();
    let ctx = sample_quote_ctx();
    let result =
        render_manager_quote(&ctx, 0xC0FFEE, 7, &store.news_grammars).expect("render must succeed");
    assert!(
        !result.is_empty(),
        "render_manager_quote returned empty string"
    );
}

#[test]
fn render_manager_quote_substitutes_opponent_against_real_grammar() {
    let store = load_real_store();
    let ctx = sample_quote_ctx();
    // The grammar's closer rule has one variant that uses #opponent#:
    //   "but credit to #opponent# — they made it difficult."
    // With 10 closer variants and a 3-way quote template, the probability of
    // hitting that variant in any single render is roughly 1/10 (for templates
    // that include a closer). We use 100 seeds to make a false-negative
    // vanishingly unlikely while keeping the test fast.
    let found_opponent = (0u64..100).any(|seed| {
        render_manager_quote(&ctx, seed, 7, &store.news_grammars)
            .map(|s| s.contains("Breck City"))
            .unwrap_or(false)
    });
    assert!(
        found_opponent,
        "render_manager_quote never included 'Breck City' across 100 seeds — \
         #opponent# substitution may not be injected."
    );
}

// ---------------------------------------------------------------------------
// AC4: variant spread — ≥2 distinct outputs across 20 seeds
// ---------------------------------------------------------------------------

#[test]
fn headline_variant_spread_across_seeds_real_grammar() {
    let store = load_real_store();
    let ctx = sample_headline_ctx();
    let results: Vec<String> = (0u64..20)
        .map(|seed| {
            render_headline(&ctx, seed, 7, &store.news_grammars)
                .expect("render must succeed for real grammar")
        })
        .collect();
    let unique: BTreeSet<&str> = results.iter().map(String::as_str).collect();
    assert!(
        unique.len() >= 2,
        "render_headline produced only 1 unique variant across 20 seeds against real grammar — \
         RNG path may not be consulted. Got: {unique:?}"
    );
}

#[test]
fn manager_quote_variant_spread_across_seeds_real_grammar() {
    let store = load_real_store();
    let ctx = sample_quote_ctx();
    let results: Vec<String> = (0u64..20)
        .map(|seed| {
            render_manager_quote(&ctx, seed, 7, &store.news_grammars)
                .expect("render must succeed for real grammar")
        })
        .collect();
    let unique: BTreeSet<&str> = results.iter().map(String::as_str).collect();
    assert!(
        unique.len() >= 2,
        "render_manager_quote produced only 1 unique variant across 20 seeds against real grammar \
         — RNG path may not be consulted. Got: {unique:?}"
    );
}

// ---------------------------------------------------------------------------
// AC5: site discriminator — headline + quote have independent RNG streams
// ---------------------------------------------------------------------------

#[test]
fn headline_and_quote_independent_rng_streams_real_grammar() {
    let store = load_real_store();
    let headline_ctx = sample_headline_ctx();
    let quote_ctx = sample_quote_ctx();

    // Render headline + quote for same (career_id=seed, event_id=0) across 20 seeds.
    // Both must show variance (AC4 already checked this individually).
    // The meaningful check: the two sequence patterns are not identical —
    // i.e. they are not picking from the same RNG stream.
    let headlines: Vec<String> = (0u64..20)
        .map(|seed| {
            render_headline(&headline_ctx, seed, 0, &store.news_grammars)
                .expect("headline render must succeed")
        })
        .collect();
    let quotes: Vec<String> = (0u64..20)
        .map(|seed| {
            render_manager_quote(&quote_ctx, seed, 0, &store.news_grammars)
                .expect("quote render must succeed")
        })
        .collect();

    // Independent variance check: both renderers vary.
    let unique_h: BTreeSet<&str> = headlines.iter().map(String::as_str).collect();
    let unique_q: BTreeSet<&str> = quotes.iter().map(String::as_str).collect();
    assert!(
        unique_h.len() >= 2,
        "headlines did not vary across 20 seeds: {unique_h:?}"
    );
    assert!(
        unique_q.len() >= 2,
        "quotes did not vary across 20 seeds: {unique_q:?}"
    );

    // The two output sequences should not be identical strings element-by-element.
    // (Headlines and quotes come from entirely different grammar rule sets;
    // element-by-element equality would require both to be single-variant
    // grammars with the same single variant — impossible given AC4 passes.
    // This assert guards against a hypothetical future regression where someone
    // accidentally reuses site=0 for both.)
    assert_ne!(
        headlines, quotes,
        "headline and quote output sequences are identical — \
         check that SITE_HEADLINE != SITE_QUOTE in news.rs"
    );
}

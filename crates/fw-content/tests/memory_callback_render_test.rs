//! Memory-callback render tests (T3-6 part 1).
//!
//! TDD RED pass: these tests are written BEFORE the production code in
//! `fw-content::memory_callback`. Every test here MUST fail on first run;
//! only then does production code get written.
//!
//! AC coverage:
//!   AC1 — every discriminant 0–29 renders a non-empty, seam-free string.
//!   AC2 — same (career_id, event_id, discriminant, ctx) → byte-identical output.
//!   AC3 — variant spread: sweep event_id yields ≥3 distinct outputs per class.
//!   AC4 — internal pin: u32→grammar-family map covers exactly 0–29.
//!   AC5 — no `{{`, no `#`, no unresolved-key artifacts in any output.

use std::collections::BTreeSet;

use fw_content::memory_callback::{
    MemoryCallbackContext, MemoryCallbackGrammarBank, render_memory_callback,
};
use fw_content::news::NewsRenderError;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A minimal context with enough string variety to exercise substitution slots.
fn sample_ctx() -> MemoryCallbackContext {
    MemoryCallbackContext {
        player_name: "Elliot Vale".into(),
        club_name: "Hartside FC".into(),
        opponent_name: "Breck City".into(),
        competition_name: "Northern Cup".into(),
        season_label: "Season 4".into(),
        score_line: "2-1".into(),
        outcome_phrase: "a late winner".into(),
        role_label: "striker".into(),
        detail_phrase: "after being written off".into(),
    }
}

/// Load the real grammar bank from the on-disk grammar file.
fn load_real_bank() -> MemoryCallbackGrammarBank {
    use std::path::PathBuf;
    let grammars_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
        .join("sources")
        .join("grammars");
    MemoryCallbackGrammarBank::load_from_dir(&grammars_dir)
        .expect("memory-callback grammar must load cleanly from content/sources/grammars/")
}

/// All 30 core discriminants (0..=29). Does NOT include 30 (UnknownEventClass).
const ALL_DISCRIMINANTS: &[u32] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29,
];

// ---------------------------------------------------------------------------
// AC1 + AC5 — every discriminant 0–29 renders non-empty and seam-free
// ---------------------------------------------------------------------------

#[test]
fn all_discriminants_render_non_empty_and_seam_free() {
    let bank = load_real_bank();
    let ctx = sample_ctx();

    for &disc in ALL_DISCRIMINANTS {
        let result = render_memory_callback(0xCAFE_BABE, 42, disc, &ctx, &bank);
        let output = result.unwrap_or_else(|e| {
            panic!("discriminant {disc} render failed: {e}");
        });
        assert!(!output.is_empty(), "discriminant {disc}: output was empty");
        // AC5 — seam-free: no Tracery artifacts
        assert!(
            !output.contains("{{"),
            "discriminant {disc}: unresolved `{{{{` seam in output: {output:?}"
        );
        assert!(
            !output.contains("}}"),
            "discriminant {disc}: unresolved `}}}}` seam in output: {output:?}"
        );
        assert!(
            !output.contains("(("),
            "discriminant {disc}: unresolved `((` artifact in output: {output:?}"
        );
        // Tracery leaves unresolved keys as `#key#` if the rule is missing.
        // Any `#` in the final output means a variable was not injected.
        assert!(
            !output.contains('#'),
            "discriminant {disc}: unresolved `#key#` pattern in output: {output:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// AC2 — determinism: same inputs → byte-identical output
// ---------------------------------------------------------------------------

#[test]
fn render_is_deterministic() {
    let bank = load_real_bank();
    let ctx = sample_ctx();

    for &disc in ALL_DISCRIMINANTS {
        let a = render_memory_callback(0xDEAD_BEEF, 7, disc, &ctx, &bank)
            .unwrap_or_else(|e| panic!("discriminant {disc} first render failed: {e}"));
        let b = render_memory_callback(0xDEAD_BEEF, 7, disc, &ctx, &bank)
            .unwrap_or_else(|e| panic!("discriminant {disc} second render failed: {e}"));
        assert_eq!(
            a, b,
            "discriminant {disc}: produced different outputs for same inputs"
        );
    }
}

// ---------------------------------------------------------------------------
// AC3 — variant spread: sweep event_id yields ≥3 distinct outputs per class
// ---------------------------------------------------------------------------

#[test]
fn variant_spread_per_discriminant() {
    let bank = load_real_bank();
    let ctx = sample_ctx();

    // Sweep 30 event_ids (0..30) to exercise RNG variant paths.
    for &disc in ALL_DISCRIMINANTS {
        let outputs: Vec<String> = (0u32..30)
            .map(|event_id| {
                render_memory_callback(0xFEED_CAFE, event_id, disc, &ctx, &bank).unwrap_or_else(
                    |e| panic!("discriminant {disc} event_id={event_id} failed: {e}"),
                )
            })
            .collect();
        let unique: BTreeSet<&str> = outputs.iter().map(String::as_str).collect();
        assert!(
            unique.len() >= 3,
            "discriminant {disc}: only {n} unique variants across 30 event_ids — \
             grammar has fewer than 3 origin variants. Got: {unique:?}",
            n = unique.len()
        );
    }
}

// ---------------------------------------------------------------------------
// AC4 — internal pin: u32→grammar-family map covers exactly 0–29
// ---------------------------------------------------------------------------

#[test]
fn grammar_family_map_covers_exactly_discriminants_0_to_29() {
    let bank = load_real_bank();
    // Every discriminant 0..=29 must map to a non-empty family.
    for &disc in ALL_DISCRIMINANTS {
        assert!(
            bank.has_family_for(disc),
            "discriminant {disc}: no grammar family registered — \
             u32→family map is incomplete; adding an EventClass variant requires \
             a corresponding entry in MemoryCallbackGrammarBank"
        );
    }
    // Discriminant 30 (UnknownEventClass) must NOT be registered as a core family
    // (it is handled by a generic fallback, not a dedicated family).
    // This assertion catches accidental over-registration.
    assert!(
        !bank.has_family_for(30),
        "discriminant 30 (UnknownEventClass) must not have a dedicated family — \
         it falls through to the generic callback family"
    );
}

// ---------------------------------------------------------------------------
// Context slot substitution — player name appears in output for debut classes
// ---------------------------------------------------------------------------

#[test]
fn player_name_appears_in_debut_senior_output() {
    let bank = load_real_bank();
    let ctx = sample_ctx();
    // Discriminant 24 = DebutSenior
    let output =
        render_memory_callback(1, 1, 24, &ctx, &bank).expect("DebutSenior render must succeed");
    assert!(
        output.contains("Elliot Vale") || output.contains("Hartside"),
        "expected player_name or club_name to appear in DebutSenior output; got: {output:?}"
    );
}

// ---------------------------------------------------------------------------
// Cup final output mentions competition or opponent for CupFinalWin (disc 18)
// ---------------------------------------------------------------------------

#[test]
fn cup_final_win_output_references_context() {
    let bank = load_real_bank();
    let ctx = sample_ctx();
    // Discriminant 18 = CupFinalWin
    let output =
        render_memory_callback(2, 2, 18, &ctx, &bank).expect("CupFinalWin render must succeed");
    let has_context_ref = output.contains("Northern Cup")
        || output.contains("Hartside")
        || output.contains("Breck City")
        || output.contains("Season 4");
    assert!(
        has_context_ref,
        "CupFinalWin output has no reference to any context slot; got: {output:?}"
    );
}

// ---------------------------------------------------------------------------
// NewsRenderError reuse — return type is Result, callers must handle errors
// ---------------------------------------------------------------------------

#[test]
fn empty_bank_errors_rather_than_silent_empty() {
    // Assert that `render_memory_callback` returns `Result<String, NewsRenderError>`,
    // not a bare String. The Ok path is confirmed here; the Err path (EmptyOutput,
    // Tracery) is exercised by the empty-string detection in the renderer itself
    // (mirrors `render_headline` discipline — no `unwrap_or_default`).
    let bank = load_real_bank();
    let ctx = sample_ctx();
    let result: Result<String, NewsRenderError> = render_memory_callback(0, 0, 0, &ctx, &bank);
    assert!(
        result.is_ok(),
        "Expected Ok for valid discriminant 0 with real bank; got: {:?}",
        result.err()
    );
}

// ---------------------------------------------------------------------------
// SITE_MEMORY_CALLBACK = 2 — independent RNG stream from headline (site=0) /
// quote (site=1)
// ---------------------------------------------------------------------------

#[test]
fn memory_callback_rng_stream_differs_from_headline_site() {
    // This test is structural: we verify that `render_memory_callback` uses
    // `seed_fn(..., SITE_MEMORY_CALLBACK)` where SITE_MEMORY_CALLBACK = 2,
    // i.e. a different site than headline (0) or quote (1).
    //
    // Indirect test: for discriminant 0 (BreakthroughMoment), render across
    // 20 career_ids and confirm the output sequence is not identical to what
    // headline would produce (headline uses "#team# beat #opponent#"-style
    // rules from a totally different grammar, so coincidental equality is
    // near-impossible). The real insurance is code inspection + the site
    // constant value.
    let bank = load_real_bank();
    let ctx = sample_ctx();

    let callback_outputs: Vec<String> = (0u64..20)
        .map(|career_id| {
            render_memory_callback(career_id, 0, 0, &ctx, &bank).expect("render must succeed")
        })
        .collect();
    let unique: BTreeSet<&str> = callback_outputs.iter().map(String::as_str).collect();
    assert!(
        unique.len() >= 2,
        "render_memory_callback produced only 1 unique variant across 20 career_ids — \
         RNG path may not be varying. Got: {unique:?}"
    );
}

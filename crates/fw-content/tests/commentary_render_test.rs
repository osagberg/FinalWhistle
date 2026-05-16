//! Integration test for the T1-4b commentary renderer.
//!
//! Loads `content/sources/` (which includes `commentary/` real fixtures —
//! 5 variants per event class authored by narrative-director in chunks 4-5),
//! exercises `render_event` for all 6 MatchEvent variants, and asserts:
//!
//! 1. Every render is non-empty (no silent failures).
//! 2. Same (match_seed, event) → byte-identical output (determinism).
//! 3. Different seeds + same event → different output ≥1 time in 20 trials
//!    (RNG is exercised — anti-vacuousness guard).
//!
//! ## Fixture state (post-T1-4b chunks 4-5)
//!
//! `content/sources/commentary/*.tracery.json` files each have ≥3 origin
//! variants (5 in practice — narrative-director shipped 5 per file in
//! chunk 4). The disk-loaded diversity-check test (`diversity_check_*`)
//! runs assertion 3 against the loaded fixtures + would catch a fixture
//! regression where someone deletes 4 of 5 variants or substitutes them
//! with identical strings. The prior in-memory 2-variant bank still ships
//! as a focused-test for the renderer's RNG path independent of fixture
//! content. (Stale doc-comment that claimed "placeholder fixtures with 1
//! variant each" corrected post Codex Tier-2 silent-failure P1 on T1-4b.)

use std::path::PathBuf;

use fw_content::{
    commentary::{MatchEventDiscriminant, render_event},
    event::{MatchEvent, PassKind},
    runtime::ContentStore,
};
use fw_core::{Q32, Tick};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn content_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

fn kickoff_event() -> MatchEvent {
    MatchEvent::KickOff {
        tick: Tick::from_raw(0),
        is_second_half: false,
    }
}

fn fulltime_event() -> MatchEvent {
    MatchEvent::FullTime {
        tick: Tick::from_raw(60),
        home_score: 1,
        away_score: 0,
    }
}

fn goal_event() -> MatchEvent {
    MatchEvent::Goal {
        scorer_slot: 9,
        tick: Tick::from_raw(30),
        score_home_after: 1,
        score_away_after: 0,
    }
}

fn shot_event() -> MatchEvent {
    MatchEvent::Shot {
        shooter_slot: 9,
        tick: Tick::from_raw(25),
        target_x: Q32::from_int(52),
        target_y: Q32::ZERO,
        on_target: true,
    }
}

fn pass_event() -> MatchEvent {
    MatchEvent::Pass {
        from_slot: 5,
        to_slot: 7,
        tick: Tick::from_raw(10),
        kind: PassKind::Short,
        completed: true,
    }
}

fn sig_event() -> MatchEvent {
    use fw_content::SignatureId;
    let id = SignatureId::try_new("fwh.core:signature.long-range-strike").unwrap();
    MatchEvent::SignatureFirstFired {
        player_slot: 9,
        signature_id: id,
        tick: Tick::from_raw(50),
    }
}

fn all_events() -> Vec<MatchEvent> {
    vec![
        kickoff_event(),
        fulltime_event(),
        goal_event(),
        shot_event(),
        pass_event(),
        sig_event(),
    ]
}

// ---------------------------------------------------------------------------
// Test 1: ContentStore loads all 6 event-class grammars from disk
// ---------------------------------------------------------------------------

#[test]
fn load_sources_loads_all_commentary_grammars() {
    let store = ContentStore::load_sources(&content_root())
        .expect("load_sources should succeed against committed fixtures");

    // Verify all 6 discriminants are present in the bank.
    for disc in MatchEventDiscriminant::all() {
        // Access via render_event — if the bank is missing a discriminant
        // it will panic (CommentaryGrammarBank invariant violated at construction,
        // so this path is unreachable after a valid load; the real guard is
        // try_from_map returning Err at load time).
        //
        // The simplest observable behavior: render_event on a placeholder event
        // must not return empty string or panic for any of the 6 classes.
        let ev = disc_to_event(disc);
        let result = render_event(&ev, 0xDEAD_BEEF_u64, &store.commentary_grammars)
            .expect("render_event must succeed for committed fixtures");
        assert!(
            !result.is_empty(),
            "render_event returned empty string for {disc:?} — \
             grammar may be missing or malformed in content/sources/commentary/"
        );
    }
}

fn disc_to_event(disc: MatchEventDiscriminant) -> MatchEvent {
    match disc {
        MatchEventDiscriminant::KickOff => kickoff_event(),
        MatchEventDiscriminant::FullTime => fulltime_event(),
        MatchEventDiscriminant::Goal => goal_event(),
        MatchEventDiscriminant::Shot => shot_event(),
        MatchEventDiscriminant::Pass => pass_event(),
        MatchEventDiscriminant::SignatureFirstFired => sig_event(),
    }
}

// ---------------------------------------------------------------------------
// Test 2: Determinism — same (match_seed, event) → byte-identical output
// ---------------------------------------------------------------------------

#[test]
fn render_event_is_deterministic_across_100_seeds() {
    let store = ContentStore::load_sources(&content_root()).expect("load_sources must succeed");

    let bank = &store.commentary_grammars;

    for event in all_events() {
        for seed in 0u64..100 {
            let a = render_event(&event, seed, bank)
                .expect("render_event must succeed for committed fixtures (a)");
            let b = render_event(&event, seed, bank)
                .expect("render_event must succeed for committed fixtures (b)");
            assert_eq!(
                a, b,
                "render_event is not deterministic for {event:?} with seed {seed}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Test 3: Anti-vacuousness — different seeds produce different variants
//
// NOTE: This test uses an in-memory 2-variant bank, NOT the on-disk fixtures
// (which start at 1 variant each — placeholder for narrative-director).
// The in-memory bank is the same setup as commentary::tests::two_variant_bank().
// This guards against regressions where the renderer stops consulting the RNG.
// ---------------------------------------------------------------------------

#[test]
fn render_event_different_seeds_produce_variety_with_two_variant_grammar() {
    use fw_content::commentary::CommentaryGrammarBank;
    use std::collections::BTreeMap;

    let mut map = BTreeMap::new();
    for disc in MatchEventDiscriminant::all() {
        let key = format!("{disc:?}");
        let mut rules: BTreeMap<String, Vec<String>> = BTreeMap::new();
        rules.insert(
            "origin".into(),
            vec![
                format!("{key} variant-A tick #tick#"),
                format!("{key} variant-B tick #tick#"),
            ],
        );
        map.insert(disc, rules);
    }
    let bank = CommentaryGrammarBank::try_from_map(map).expect("all discriminants present");

    // For each event class, 20 different seeds must produce ≥2 distinct outputs.
    for event in all_events() {
        let results: Vec<String> = (0u64..20)
            .map(|seed| {
                render_event(&event, seed, &bank)
                    .expect("render_event must succeed for valid 2-variant bank")
            })
            .collect();
        let unique: std::collections::BTreeSet<&str> = results.iter().map(String::as_str).collect();
        assert!(
            unique.len() >= 2,
            "render_event({event:?}) produced only 1 unique variant across 20 seeds — \
             RNG may not be exercised. Got: {unique:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 4: Missing commentary directory → MissingCommentaryGrammar error
//
// This test exercises the fail-loud path: load_sources with a content root
// that has no commentary/ dir must return an error, not a silent empty bank.
// ---------------------------------------------------------------------------

#[test]
fn load_sources_fails_loud_when_commentary_dir_missing() {
    use fw_content::runtime::ContentLoadError;
    use std::fs;
    use std::process;
    use std::thread;

    // Build a minimal content tree with no commentary/ directory.
    //
    // Codex Tier-2 code-reviewer P3 on T1-4b 2026-05-16: prior path used a
    // shared `/tmp/fwh-test-no-commentary` literal — race hazard if two test
    // processes ran concurrently. Suffixing with PID + thread-id is the
    // standard "no `tempfile` dep, no clock" pattern (Sim/RULES.md §3 bans
    // `SystemTime::now()` / `Instant::now()` across all sim-adjacent crates).
    // PID handles cross-process collisions; thread-id handles intra-process
    // parallel cargo-test invocations of the same integration test (rare but
    // possible). Neither is a clock — both are process-state.
    let thread_id = format!("{:?}", thread::current().id());
    let tmp = std::env::temp_dir().join(format!(
        "fwh-test-no-commentary-{}-{}",
        process::id(),
        thread_id.replace(['(', ')', ' '], "_")
    ));
    let sources = tmp.join("sources");
    fs::create_dir_all(&sources).unwrap();

    let result = ContentStore::load_sources(&tmp);
    assert!(
        matches!(
            result,
            Err(ContentLoadError::MissingCommentaryGrammar { .. })
        ),
        "expected MissingCommentaryGrammar error; got: {result:?}"
    );

    // Cleanup (best-effort).
    let _ = fs::remove_dir_all(&tmp);
}

// ---------------------------------------------------------------------------
// Test 5: Diversity check against disk-loaded fixtures
//
// Codex Tier-2 silent-failure P1 on T1-4b 2026-05-16: prior test suite only
// ran the diversity check (assertion 3) against an in-memory 2-variant bank.
// With narrative-director's chunk-4 fixtures (5 variants per file), the
// diversity check CAN + SHOULD run against the loaded grammars too — catches
// a fixture regression where someone deletes 4 of 5 variants OR substitutes
// them with strings that hash identically. Without this test, a future PR
// that ships single-variant fixtures would pass the existing assertion-3
// test (which uses its own in-memory bank) but produce repetitive prose.
// ---------------------------------------------------------------------------

#[test]
fn disk_loaded_fixtures_produce_variant_diversity_across_seeds() {
    let store = ContentStore::load_sources(&content_root())
        .expect("load_sources should succeed against committed fixtures");
    let bank = &store.commentary_grammars;

    // For each event class, 30 different seeds must produce ≥2 distinct
    // outputs. (≥2 is a soft floor — Content/RULES.md §4 mandates ≥3
    // variants per slot; if narrative-director ships 3+ variants per file
    // the actual unique count will be 3-5. ≥2 floor is the regression
    // tripwire that flags single-variant fixture regressions without
    // brittleness against legitimate variant-pick distribution patterns.)
    for event in all_events() {
        let results: Vec<String> = (0u64..30)
            .map(|seed| {
                render_event(&event, seed, bank)
                    .expect("render_event must succeed for committed fixtures")
            })
            .collect();
        let unique: std::collections::BTreeSet<&str> = results.iter().map(String::as_str).collect();
        assert!(
            unique.len() >= 2,
            "disk-loaded {event:?} produced only {} unique variant(s) across 30 seeds — \
             content/sources/commentary/ fixtures may have regressed to single-variant. \
             Got: {unique:?}",
            unique.len()
        );
    }
}

// ---------------------------------------------------------------------------
// Tests 6 + 7: CommentaryRenderError variant exercise
//
// Codex 2026-05-16 re-audit P1: render_event now returns
// `Result<String, CommentaryRenderError>` with Tracery + EmptyOutput
// variants (fix-pass closure for the prior silent-failure P0 where
// `unwrap_or_default()` converted every error into an empty string). The
// existing integration tests cover the happy-path render only; without
// these two tests the typed error surface is structurally untested — a
// future refactor that quietly collapses the Err arm back into Ok("")
// would pass the existing suite. These tests construct bad-grammar bank
// inputs that bypass `try_from_map`'s construction-time guard via the
// `CommentaryGrammarBank::try_from_map` Err path itself, OR via
// hand-built valid-by-construction-but-render-broken grammars that
// reference an undefined substitution variable in their origin.
// ---------------------------------------------------------------------------

#[test]
fn render_event_returns_tracery_error_on_undefined_substitution_variable() {
    use fw_content::commentary::{CommentaryGrammarBank, CommentaryRenderError};
    use std::collections::BTreeMap;

    // Construct a bank where every grammar's origin references a variable
    // the renderer does NOT inject (`#nonExistentField#`). The grammar
    // passes `try_from_map`'s structural checks (non-empty origin variant)
    // but Tracery raises MissingKeyError at flatten time.
    let mut map = BTreeMap::new();
    for disc in MatchEventDiscriminant::all() {
        let mut rules: BTreeMap<String, Vec<String>> = BTreeMap::new();
        rules.insert(
            "origin".into(),
            vec!["Variable not injected: #nonExistentField#".into()],
        );
        map.insert(disc, rules);
    }
    let bank = CommentaryGrammarBank::try_from_map(map)
        .expect("bank construction succeeds — origin rule is non-empty");

    for event in all_events() {
        let result = render_event(&event, 0xDEAD_BEEF_u64, &bank);
        assert!(
            matches!(result, Err(CommentaryRenderError::Tracery { .. })),
            "render_event({event:?}) should have returned CommentaryRenderError::Tracery \
             (MissingKeyError on #nonExistentField#); got: {result:?}"
        );
    }
}

#[test]
fn empty_output_variant_is_publicly_constructible_and_displays_correctly() {
    use fw_content::commentary::CommentaryRenderError;

    // EmptyOutput is the defensive tripwire in render_event: if Tracery
    // ever returns `Ok("")` (which it doesn't today — tracery 0.2.1 errors
    // out on empty rule bodies + try_from_map blocks empty origin variants
    // at construction time, so EmptyOutput is structurally unreachable via
    // legitimate input in T1-4b). The check exists because tracery's API
    // doesn't forbid an empty Ok return; a future tracery version that
    // changed parser behavior could make it reachable.
    //
    // This test confirms the variant is publicly constructible (callers
    // can pattern-match on it without compile error) + the Display impl
    // names the variant clearly + the variant is structurally distinct
    // from Tracery. It's a type-design test, not a render-path test.
    //
    // If a future change moves Tracery toward "return Ok(empty)" behavior,
    // the unreachable defensive check becomes the reachable defense; this
    // test ensures the API surface is ready for that day.
    for disc in MatchEventDiscriminant::all() {
        let err = CommentaryRenderError::EmptyOutput { event_class: disc };
        let msg = format!("{err}");
        assert!(
            msg.contains(&format!("{disc:?}")),
            "Display impl for EmptyOutput must include discriminant name; got: {msg:?}"
        );
        assert!(
            msg.contains("content-authoring bug"),
            "Display impl for EmptyOutput must hint at the underlying cause \
             (content-authoring bug); got: {msg:?}"
        );
        // Variant is distinct from Tracery (caller pattern-match works).
        match err {
            CommentaryRenderError::EmptyOutput { event_class } => {
                assert_eq!(event_class, disc, "discriminant must round-trip");
            }
            CommentaryRenderError::Tracery { .. } => {
                panic!("EmptyOutput must not pattern-match as Tracery");
            }
        }
    }
}

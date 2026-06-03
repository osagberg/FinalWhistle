//! QA-3 — world-gen seed-diversity proptest.
//!
//! Pillar #1 ("every save is a different world") requires that distinct
//! career seeds produce structurally distinct leagues. This proptest locks
//! that invariant: across a 50-pair seed sweep, two DIFFERENT seeds must
//! yield leagues whose full generated identity differs.
//!
//! ## What "structurally distinct" means here
//!
//! The only world-gen output that varies per seed is generated IDENTITY:
//! club names, manager names, and the 22 player names per club (each drawn
//! from a per-club ChaCha8 stream seeded via `seed_fn(top_seed, club_idx,
//! SeedLayer::ContentBake, 0)`). Club COUNT (always 20) and the fixture
//! schedule (circle method against the fixed club ordering) are
//! seed-INDEPENDENT by design — see `fixture_schedule_proptest.rs` — so they
//! are deliberately NOT part of the diversity observable. The fingerprint
//! below therefore captures the full generated identity (20 club names +
//! 20 manager names + 440 player names).
//!
//! ## Why the companion guard exists
//!
//! `same_seed_is_identity_stable` pins same-seed → identical fingerprint.
//! Without it, the diversity test could pass for the WRONG reason: if the
//! fingerprint were nondeterministic noise, two calls would differ
//! regardless of the seed. Locking same-seed stability proves the
//! fingerprint is a genuine function of the seed, so a diversity failure
//! means "generation ignored the seed" — exactly the regression we want to
//! catch (e.g. a refactor that drops the per-club seed derivation and makes
//! the world seed-blind).
//!
//! ## Entropy / flakiness
//!
//! The fingerprint concatenates ~480 Markov-drawn names over the culture
//! name banks. A genuine collision between two distinct seeds is
//! astronomically unlikely (each club's stream is keyed by the BLAKE3-based
//! `seed_fn`), so this test is deterministic-pass for a correct
//! implementation and deterministic-fail only if generation collapses to
//! seed-independence. It is NOT a probabilistic flake risk in practice.

use fw_content::{CLUBS_PER_LEAGUE, ContentStore, ProcGenTeam, generate_league_with_teams};
use fw_core::Seed;
use proptest::prelude::*;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Load the committed content tree ONCE per test-run. Without the cache,
/// each proptest iteration would re-parse every RON fixture under
/// `content/sources/`, dominating the suite wall-clock. Mirrors the cache
/// in `fixture_schedule_proptest.rs`.
fn content() -> &'static ContentStore {
    static CONTENT: OnceLock<ContentStore> = OnceLock::new();
    CONTENT.get_or_init(|| {
        let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("content");
        ContentStore::load_sources(&content_root).expect("ContentStore::load_sources failed")
    })
}

/// Deterministic structural fingerprint of a league's full generated
/// identity: every club name, every manager name, and every player name, in
/// club-then-slot order, separated by ASCII Unit Separator (U+001F) — a
/// control character that never appears inside a generated name, so the
/// join is unambiguous. Two leagues share a fingerprint iff they share their
/// entire generated identity.
fn league_identity_fingerprint(teams: &[ProcGenTeam]) -> String {
    let mut parts: Vec<String> = Vec::with_capacity(teams.len() * 24);
    for team in teams {
        parts.push(format!("T:{}", team.team_name));
        parts.push(format!("M:{}", team.manager.display()));
        for (slot, player) in team.players.iter().enumerate() {
            parts.push(format!("P{slot:02}:{}", player.display()));
        }
    }
    parts.join("\u{1f}")
}

/// Generate a league for `seed` and return its identity fingerprint.
fn fingerprint_for(seed: u64) -> String {
    let (_league, teams) = generate_league_with_teams(Seed::from_u64(seed), content())
        .expect("generate_league_with_teams must succeed on committed content tree");
    assert_eq!(
        teams.len(),
        CLUBS_PER_LEAGUE,
        "generated team count must equal CLUBS_PER_LEAGUE"
    );
    league_identity_fingerprint(&teams)
}

proptest! {
    #![proptest_config(ProptestConfig {
        // 50 pairs per the QA-3 acceptance criterion. Each case generates two
        // leagues (~ms-scale) so the suite stays fast.
        cases: 50,
        ..ProptestConfig::default()
    })]

    /// Two DIFFERENT seeds produce structurally distinct leagues.
    #[test]
    fn distinct_seeds_produce_distinct_leagues(s1 in any::<u64>(), s2 in any::<u64>()) {
        prop_assume!(s1 != s2);
        let f1 = fingerprint_for(s1);
        let f2 = fingerprint_for(s2);
        prop_assert_ne!(
            f1,
            f2,
            "distinct seeds {:#x} and {:#x} produced identical league identity — \
             world generation appears to ignore the seed (pillar #1 violation)",
            s1,
            s2
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 50,
        ..ProptestConfig::default()
    })]

    /// Vacuity guard: the SAME seed always produces the SAME identity, so the
    /// diversity test above cannot pass on fingerprint noise.
    #[test]
    fn same_seed_is_identity_stable(seed in any::<u64>()) {
        let a = fingerprint_for(seed);
        let b = fingerprint_for(seed);
        prop_assert_eq!(
            a,
            b,
            "seed {:#x} produced two different identities across calls — \
             world generation is nondeterministic",
            seed
        );
    }
}

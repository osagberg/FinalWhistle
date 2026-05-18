//! T2-R7(e) — fixture-schedule pair-coverage symmetry across seed-space.
//!
//! Post-T2 ultimate-review Track D-Survey-3 caught that the AC4 test
//! `fixture_schedule_covers_all_pairs_home_and_away` only exercised ONE
//! seed (`0xC0FFEE`). The circle-method invariants ("every ordered home/
//! away pair appears exactly once" and "each club plays 19 home + 19
//! away") are properties of the algorithm, not of any single seed; a
//! single-seed test is structurally vacuous against the invariant it
//! claims to enforce. This proptest hits the invariant across a 256-seed
//! sample so a future edit to the circle-method that accidentally breaks
//! pair-coverage on some seeds would surface here.
//!
//! Note: the circle-method itself is seed-independent today (the seed
//! drives club-identity content, not the schedule), so in steady-state
//! this proptest is redundant with the AC4 single-seed test. Its value
//! is REGRESSION-ONLY: it locks the invariant against future refactors
//! that might thread the seed into the pairing step (e.g. weighted
//! seedings, derby-spreading heuristics, broadcast-window adjustments).
//! If/when that refactor lands, this proptest stops being trivially
//! satisfied and starts paying for itself.
//!
//! Cross-reference: post-T2 ultimate-review doc at
//! `docs/audits/post-t2-ultimate-review-2026-05-18.md` Track D-Survey-3.

use fw_content::{CLUBS_PER_LEAGUE, ContentStore, MATCHES_PER_SEASON, generate_league};
use fw_core::{ClubId, Seed};
use proptest::prelude::*;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Load the committed content tree ONCE per test-run. Without the cache,
/// each proptest iteration would re-parse every RON fixture under
/// `content/sources/`, which dominates the test wall-clock and pushes
/// individual cases past proptest's default per-case timeout.
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

proptest! {
    #![proptest_config(ProptestConfig {
        // 256 cases covers the seed-derivation surface without making the
        // suite noticeably slower. Each case re-runs generate_league which
        // is ~ms-scale.
        cases: 256,
        ..ProptestConfig::default()
    })]

    /// Every seed yields a fixture list where each ordered (home, away)
    /// pair appears EXACTLY ONCE and each club plays EXACTLY 19 home + 19
    /// away fixtures.
    #[test]
    fn fixture_schedule_pair_coverage_holds_across_seeds(raw_seed in any::<u64>()) {
        let league = generate_league(Seed::from_u64(raw_seed), content())
            .expect("generate_league must succeed on committed content tree");

        prop_assert_eq!(
            league.fixtures.len(),
            MATCHES_PER_SEASON,
            "fixture count must equal MATCHES_PER_SEASON"
        );
        prop_assert_eq!(
            league.clubs.len(),
            CLUBS_PER_LEAGUE,
            "club count must equal CLUBS_PER_LEAGUE"
        );

        // Each ordered (home, away) pair must appear exactly ONCE.
        let mut pair_count: std::collections::BTreeMap<(ClubId, ClubId), u32> =
            std::collections::BTreeMap::new();
        for f in &league.fixtures {
            prop_assert_ne!(
                f.home, f.away,
                "fixture has same home + away club"
            );
            *pair_count.entry((f.home, f.away)).or_insert(0) += 1;
        }
        // 20 clubs × 19 distinct away opponents = 380 unique ordered pairs.
        prop_assert_eq!(
            pair_count.len(),
            MATCHES_PER_SEASON,
            "must have MATCHES_PER_SEASON distinct ordered (home,away) pairs"
        );
        for (pair, count) in &pair_count {
            prop_assert_eq!(*count, 1, "pair {:?} appeared {} times (must be 1)", pair, count);
        }

        // Per-club: exactly 19 home + 19 away.
        let mut home_count: std::collections::BTreeMap<ClubId, u32> =
            std::collections::BTreeMap::new();
        let mut away_count: std::collections::BTreeMap<ClubId, u32> =
            std::collections::BTreeMap::new();
        for f in &league.fixtures {
            *home_count.entry(f.home).or_insert(0) += 1;
            *away_count.entry(f.away).or_insert(0) += 1;
        }
        for club in &league.clubs {
            let h = home_count.get(&club.id).copied().unwrap_or(0);
            let a = away_count.get(&club.id).copied().unwrap_or(0);
            prop_assert_eq!(h, 19, "club {:?} home count must be 19, got {}", club.id, h);
            prop_assert_eq!(a, 19, "club {:?} away count must be 19, got {}", club.id, a);
        }
    }
}

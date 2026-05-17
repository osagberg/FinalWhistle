//! T2-2: Procedural league + double-round-robin fixture generator.
//!
//! Wraps the existing `procgen::generate_team` (T1-7) with a per-club
//! seed-derivation + culture/archetype/manager pairing strategy, producing
//! a `League` of 20 procedural clubs + a deterministic 380-fixture season
//! schedule via the circle method.
//!
//! Per the T2-2 MEMORY task-spec user-resolved ambiguity gate:
//!   - Clubs are RUNTIME-GENERATED from a single seed (no source RON files
//!     under `content/sources/clubs/`). Procedural-FIRST per pillar #1
//!     ("every save is a different world"). Each career seed produces a
//!     unique 20-club catalog.
//!   - Fixture algorithm is SIMPLE DOUBLE ROUND-ROBIN via circle method
//!     (each club pair plays home + away exactly once each; 20 × 19 = 380
//!     matches across 38 match-days × 10 matches/day; deterministic order).
//!
//! ## Determinism contract
//!
//! - `generate_league(seed, content)` is a pure function of (seed, content).
//!   Same inputs → byte-identical League across runs + platforms.
//! - Per-club draws derive their seeds via `seed_fn(career_seed, club_idx,
//!   SeedLayer::ContentBake, 0)` so each club is independently reproducible.
//! - Fixture ordering uses the circle method against a fixed club-ordering
//!   (`league.clubs[0..20]` index order); no RNG involvement in scheduling
//!   itself (the seed only influences club identity, not fixture order
//!   beyond that).
//!
//! ## Downstream consumers
//!
//! T2-5 (season controller) consumes `League.fixtures` as the source-of-
//! truth for "what plays this week." T2-5's `play_fixtures(match_day) ->
//! [MatchResult; 10]` boundary maps to `League.fixtures.iter().filter(|f|
//! f.match_day == target_day)`.

use fw_core::{ClubId, Seed};
use serde::{Deserialize, Serialize};

use crate::procgen::{ProcGenError, ProcGenInputs, generate_team};
use crate::runtime::ContentStore;
use crate::team::TeamTemplate;

/// Number of clubs per league (T2-2 single-tier slice). Fixed at 20 per
/// the design-doc T2-2 row + the user-resolved ambiguity-gate scope.
pub const CLUBS_PER_LEAGUE: usize = 20;

/// Number of match-days per season for a double round-robin with
/// `CLUBS_PER_LEAGUE` teams: each round has `CLUBS_PER_LEAGUE / 2`
/// matches; first leg = `CLUBS_PER_LEAGUE - 1` rounds; reverse leg
/// mirrors the first = `(CLUBS_PER_LEAGUE - 1) × 2` rounds. For 20 teams
/// = 38 match-days.
pub const MATCH_DAYS_PER_SEASON: u16 = (CLUBS_PER_LEAGUE as u16 - 1) * 2;

/// Total fixture count per season for a double round-robin: each club pair
/// plays home + away once each. `(CLUBS_PER_LEAGUE × (CLUBS_PER_LEAGUE - 1))`
/// per the standard formula. For 20 teams = 380.
pub const MATCHES_PER_SEASON: usize = CLUBS_PER_LEAGUE * (CLUBS_PER_LEAGUE - 1);

/// A single league match between two clubs.
///
/// `home` + `away` are fixed at fixture-generation time (the circle method
/// determines which side each club takes per pair-occurrence). `match_day`
/// is the 1-indexed day-of-season slot (1..=`MATCH_DAYS_PER_SEASON`); each
/// match-day groups exactly `CLUBS_PER_LEAGUE / 2 == 10` simultaneous
/// fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Fixture {
    /// Home club for this match. Always != `away`.
    pub home: ClubId,
    /// Away club for this match. Always != `home`.
    pub away: ClubId,
    /// 1-indexed match-day slot (1..=38 for `CLUBS_PER_LEAGUE = 20`).
    /// All fixtures sharing the same `match_day` are played
    /// simultaneously per real-football match-week semantics.
    pub match_day: u16,
}

/// A procedural league: 20 clubs + 380 fixtures + a display name.
///
/// Runtime-generated from a single `Seed` via `generate_league`; no
/// committed source RON files (per pillar #1 procedural-fantasy world).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct League {
    /// Procgen'd league name (e.g. "Northshire Second Division" via the
    /// existing culture markov chain — though T2-2's MVP shape uses a
    /// fixed-template name; T3+ may procgen this via a dedicated
    /// `league_name` markov chain per culture).
    pub name: String,

    /// The 20 procedural clubs. Length is exactly `CLUBS_PER_LEAGUE`;
    /// invariant enforced by `generate_league`. Ordered by `clubs[i].id`
    /// for stable iteration / fixture lookups.
    pub clubs: Vec<TeamTemplate>,

    /// The 380 fixtures for the season. Length is exactly
    /// `MATCHES_PER_SEASON`; invariant enforced by `generate_fixtures`.
    /// Ordered by `(match_day, home_id, away_id)` for stable
    /// deterministic iteration.
    pub fixtures: Vec<Fixture>,
}

/// Generate a deterministic double-round-robin schedule for `CLUBS_PER_LEAGUE`
/// clubs via the circle method.
///
/// **Circle method** (standard sports-league algorithm): pin one club
/// (club at index 0) + rotate the remaining `CLUBS_PER_LEAGUE - 1` clubs
/// around a circle. For each round, pair the pinned club with the rotating
/// club at the opposite position + pair the other clubs across the circle.
/// Produces `CLUBS_PER_LEAGUE - 1` rounds covering every pair exactly once
/// for the first leg; the reverse leg flips home/away for each pair on
/// the corresponding mirror round.
///
/// **Determinism**: pure function of `club_ids` order. Same input → same
/// output Vec byte-for-byte. The `_seed` parameter is reserved for future
/// fixture-order randomization (e.g. shuffling match-day order while
/// preserving the pair-coverage invariant); for T2-2 MVP the schedule is
/// fixed by the circle-method algorithm alone.
///
/// **Home/away assignment per pair-occurrence**: in the first leg, the
/// club at the lower circle-position takes home; in the reverse leg,
/// home/away swap. This produces a balanced 19 home + 19 away per club.
///
/// # Panics
///
/// Panics if `club_ids.len() != CLUBS_PER_LEAGUE`. Callers should ensure
/// the slice is exactly 20 long; `generate_league` enforces this.
#[must_use]
pub fn generate_fixtures(club_ids: &[ClubId; CLUBS_PER_LEAGUE], _seed: Seed) -> Vec<Fixture> {
    let n = CLUBS_PER_LEAGUE;
    let rounds_per_leg = n - 1; // 19 for n=20
    let half = n / 2; // 10 matches per round

    let mut fixtures: Vec<Fixture> = Vec::with_capacity(MATCHES_PER_SEASON);

    // Circle method working array: club_ids[0] is pinned; club_ids[1..n]
    // rotate. We track the rotating array as indices into the original
    // `club_ids` slice for clarity.
    //
    // Initial layout (n=20 example):
    //   pinned: 0
    //   rotating: [1, 2, 3, ..., 19]
    //
    // Round r (0-indexed) pairings:
    //   (pinned, rotating[n-2])              — pinned vs last rotating slot
    //   for i in 0..(half - 1):
    //     (rotating[i], rotating[n - 3 - i]) — symmetric pairs from outer-to-inner
    //
    // After producing round r's pairings, rotate `rotating` right by 1:
    //   new[0] = old[n-2]; new[1..] = old[0..n-2]
    let mut rotating: Vec<usize> = (1..n).collect();

    // First leg: rounds 0..rounds_per_leg.
    for round in 0..rounds_per_leg {
        let match_day = (round as u16) + 1; // 1-indexed
        // Pinned (index 0) vs the last rotating slot.
        let pinned_opponent_idx = rotating[n - 2];
        // First leg home/away convention: pinned takes home in even rounds,
        // away in odd rounds. Produces balanced 9 home + 10 away (or vice
        // versa) for the pinned club across the first leg's 19 rounds.
        // (Exact 19/2 split rounds to 10 home + 9 away for the pinned club
        // across the first leg; reverse leg flips → final tally 19 each.)
        let (home, away) = if round.is_multiple_of(2) {
            (club_ids[0], club_ids[pinned_opponent_idx])
        } else {
            (club_ids[pinned_opponent_idx], club_ids[0])
        };
        fixtures.push(Fixture {
            home,
            away,
            match_day,
        });

        // The other (half - 1) pairings: rotating[i] vs rotating[n - 3 - i]
        // for i in 0..(half - 1). Home/away alternates similarly per i +
        // round parity.
        for i in 0..(half - 1) {
            let a_idx = rotating[i];
            let b_idx = rotating[n - 3 - i];
            // Symmetric home/away derivation: use (round + i) parity so
            // each club gets a balanced home/away count across the leg.
            let (home, away) = if (round + i).is_multiple_of(2) {
                (club_ids[a_idx], club_ids[b_idx])
            } else {
                (club_ids[b_idx], club_ids[a_idx])
            };
            fixtures.push(Fixture {
                home,
                away,
                match_day,
            });
        }

        // Rotate `rotating` right by 1: last element moves to front.
        let last = rotating[n - 2];
        for i in (1..n - 1).rev() {
            rotating[i] = rotating[i - 1];
        }
        rotating[0] = last;
    }

    // Reverse leg: rounds rounds_per_leg..(rounds_per_leg * 2). Each
    // first-leg fixture has a mirror in the reverse leg with home/away
    // swapped + match_day shifted by `rounds_per_leg`. Iterate the first
    // leg's fixtures directly + emit the mirrors.
    let first_leg_count = fixtures.len();
    for i in 0..first_leg_count {
        let original = fixtures[i];
        fixtures.push(Fixture {
            home: original.away,
            away: original.home,
            match_day: original.match_day + rounds_per_leg as u16,
        });
    }

    // Sort by (match_day, home.raw(), away.raw()) for stable
    // deterministic iteration (downstream T2-5 consumers can rely on
    // ordering for "next match" lookups without re-sorting).
    fixtures.sort_by_key(|f| (f.match_day, f.home.raw(), f.away.raw()));

    fixtures
}

/// Generate a deterministic procedural league of 20 clubs + 380 fixtures
/// from a single seed.
///
/// Per-club draws use `Seed::from_u64(seed_fn(seed.to_u64(), club_idx as
/// u32, SeedLayer::ContentBake, 0))` so each club is independently
/// reproducible — changing one club's input (or adding a new culture)
/// doesn't perturb the other 19 clubs' identities.
///
/// **Culture / archetype / manager pairing strategy** (T2-2 MVP):
///   - Cultures: round-robin across `content.cultures` (2 cultures
///     today → 10 anglo clubs + 10 fantasy-elvish; future culture
///     additions automatically rebalance).
///   - Tactical archetypes: round-robin across `content.tactical_archetypes`
///     (16 archetypes today → each club gets archetype `i % 16`).
///   - Manager archetypes: round-robin across `content.managers` (15
///     managers today → club `i` gets manager `i % 15`; some duplicate
///     assignments inevitable when CLUBS_PER_LEAGUE > manager-count).
///
/// **Determinism**: pure function of `(seed, content)`. Same inputs →
/// byte-identical League. ContentStore iteration is BTreeMap-ordered so
/// the round-robin assignments are stable across builds + platforms.
///
/// # Errors
///
/// Returns `ProcGenError::MissingCulture` / `MissingTacticalArchetype` /
/// `MissingManagerArchetype` if the content store is empty in any of
/// those catalogs (T2-2 requires ≥1 culture + ≥1 archetype + ≥1 manager
/// at minimum; today's content tree has 2/16/15 respectively so this
/// only fires on a malformed empty content store).
pub fn generate_league(seed: Seed, content: &ContentStore) -> Result<League, ProcGenError> {
    if content.cultures.is_empty() {
        return Err(ProcGenError::MissingCulture(
            "(content.cultures empty)".into(),
        ));
    }
    if content.tactical_archetypes.is_empty() {
        return Err(ProcGenError::MissingTacticalArchetype(
            "(content.tactical_archetypes empty)".into(),
        ));
    }
    if content.managers.is_empty() {
        return Err(ProcGenError::MissingManagerArchetype(
            "(content.managers empty)".into(),
        ));
    }

    // Snapshot the ordered catalog keys for round-robin assignment. BTreeMap
    // iteration is key-ordered so this is platform-stable.
    let culture_ids: Vec<&str> = content.cultures.keys().map(String::as_str).collect();
    let archetype_ids: Vec<&str> = content
        .tactical_archetypes
        .keys()
        .map(String::as_str)
        .collect();
    let manager_ids: Vec<&str> = content.managers.keys().map(String::as_str).collect();

    let mut clubs: Vec<TeamTemplate> = Vec::with_capacity(CLUBS_PER_LEAGUE);
    for club_idx in 0..CLUBS_PER_LEAGUE {
        // Per-club seed derivation: each club gets an independent ChaCha8
        // stream so adding/removing a culture (or changing the round-robin
        // strategy) doesn't perturb the 19 other clubs' identities.
        // Using SeedLayer::ContentBake matches the existing generate_team
        // convention so cross-row seed-space stays partitioned per ADR-0009.
        let club_seed = Seed::from_u64(fw_core::seed_fn(
            seed.to_u64(),
            club_idx as u32,
            fw_core::SeedLayer::ContentBake,
            0,
        ));

        let inputs = ProcGenInputs {
            culture_id: culture_ids[club_idx % culture_ids.len()],
            tactical_archetype_id: archetype_ids[club_idx % archetype_ids.len()],
            manager_archetype_id: manager_ids[club_idx % manager_ids.len()],
            seed: club_seed,
        };

        let procgen_team = generate_team(content, inputs)?;

        // ClubId allocation: deterministic 1-indexed sequence per league.
        // Future career-mode multi-league worlds need a global ClubId
        // allocator (likely a save-state counter); T2-2 single-league
        // scope is simple 1..=20.
        let club_id = ClubId::new((club_idx as u32) + 1);
        let qualified_id = format!("fwh.core:club_{:05}", club_id.raw());

        clubs.push(TeamTemplate {
            id: club_id,
            qualified_id,
            display_name: procgen_team.team_name,
        });
    }

    // Snapshot club IDs in order for fixture generation. The collect into
    // `[ClubId; CLUBS_PER_LEAGUE]` via try_into is safe because the loop
    // above pushed exactly CLUBS_PER_LEAGUE items + the panic message
    // names the binding invariant for any future regression.
    let club_id_array: [ClubId; CLUBS_PER_LEAGUE] = clubs
        .iter()
        .map(|c| c.id)
        .collect::<Vec<_>>()
        .try_into()
        .expect("clubs.len() == CLUBS_PER_LEAGUE invariant violated post-loop");

    let fixtures = generate_fixtures(&club_id_array, seed);

    // League name: MVP shape is a fixed-template-with-seed-suffix string.
    // T3+ may procgen this via a dedicated `league_name` markov chain
    // per culture; out of scope for T2-2 per the MEMORY task-spec
    // "Intentionally NOT done" list.
    let name = format!("Procedural League ({:#018x})", seed.to_u64());

    Ok(League {
        name,
        clubs,
        fixtures,
    })
}

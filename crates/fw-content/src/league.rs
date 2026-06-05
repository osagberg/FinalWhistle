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

use std::collections::{BTreeMap, BTreeSet};

use fw_core::{ClubId, Seed};
use serde::{Deserialize, Serialize};

use crate::procgen::{ProcGenError, ProcGenInputs, ProcGenTeam, generate_team};
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
/// output Vec byte-for-byte.
///
/// **Home/away assignment per pair-occurrence**: in the first leg, the
/// club at the lower circle-position takes home; in the reverse leg,
/// home/away swap. This produces a balanced 19 home + 19 away per club.
///
/// T2-R-B7 (post-T2 ultimate-review Track B-7): the prior signature
/// took an unused `_seed: Seed` parameter "reserved for future fixture-
/// order randomization (weighted seedings, derby-spreading, broadcast-
/// window adjustments)." Removed — the API parameter was undocumented
/// reserve-against-vapor. If/when seed-driven fixture shuffling lands
/// (potentially never — real-football schedules are computer-fixed by
/// the FA, only kickoff times shuffle), the signature gains the param
/// back AT THAT TIME, with the consumer test that authorises it.
///
/// # Panics
///
/// Panics if `club_ids.len() != CLUBS_PER_LEAGUE`. Callers should ensure
/// the slice is exactly 20 long; `generate_league` enforces this.
/// Generate a deterministic double-round-robin schedule for exactly
/// `CLUBS_PER_LEAGUE` clubs via the circle method.
///
/// For arbitrary club counts use [`generate_fixtures_from_slice`].
///
/// # Panics
///
/// Panics if `club_ids.len() != CLUBS_PER_LEAGUE`. Callers should ensure
/// the slice is exactly 20 long; `generate_league` enforces this.
#[must_use]
pub fn generate_fixtures(club_ids: &[ClubId; CLUBS_PER_LEAGUE]) -> Vec<Fixture> {
    generate_fixtures_from_slice(club_ids.as_slice())
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
    generate_league_with_teams(seed, content).map(|(league, _teams)| league)
}

/// Generate a deterministic procedural league, returning both the `League`
/// and the per-club `ProcGenTeam` (22 player names + manager name + team name).
///
/// The `ProcGenTeam` at index `i` corresponds to `league.clubs[i]`: names are
/// already attached to `TeamTemplate.display_name` via `procgen_team.team_name`;
/// the `players` array carries the 22 slot-ordered `PlayerName`s consumed by
/// the career-roster layer at T4-2.5b so they are NOT recomputed there.
///
/// See [`generate_league`] for the full determinism contract and error docs.
pub fn generate_league_with_teams(
    seed: Seed,
    content: &ContentStore,
) -> Result<(League, Vec<ProcGenTeam>), ProcGenError> {
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

    // S9 dedup: track used team names per culture so clubs sharing a culture
    // don't collide.  BTreeMap<culture_id, BTreeSet<team_name>>.
    // BTreeMap for deterministic ordering (Sim/RULES.md §2).
    let mut used_names_by_culture: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();

    let mut clubs: Vec<TeamTemplate> = Vec::with_capacity(CLUBS_PER_LEAGUE);
    let mut procgen_teams: Vec<ProcGenTeam> = Vec::with_capacity(CLUBS_PER_LEAGUE);
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

        let culture_id = culture_ids[club_idx % culture_ids.len()];

        // Snapshot the used-names set for this culture as a local value so
        // the borrow on `used_names_by_culture` ends before we need to
        // mutably insert the result below.  Cloning is O(used_count) per
        // club, and `used_count` is at most `CLUBS_PER_LEAGUE / culture_count`
        // ≈ 10 for 2 cultures — negligible at career-init frequency.
        let used_snapshot: BTreeSet<String> = used_names_by_culture
            .get(culture_id)
            .cloned()
            .unwrap_or_default();

        let inputs = ProcGenInputs {
            culture_id,
            tactical_archetype_id: archetype_ids[club_idx % archetype_ids.len()],
            manager_archetype_id: manager_ids[club_idx % manager_ids.len()],
            seed: club_seed,
            used_team_names: Some(&used_snapshot),
        };

        let procgen_team = generate_team(content, inputs)?;

        // Record the chosen team name as used for this culture so the next
        // club assigned to the same culture won't collide.
        used_names_by_culture
            .entry(culture_id)
            .or_default()
            .insert(procgen_team.team_name.clone());

        // ClubId allocation: deterministic 1-indexed sequence per league.
        // Future career-mode multi-league worlds need a global ClubId
        // allocator (likely a save-state counter); T2-2 single-league
        // scope is simple 1..=20.
        let club_id = ClubId::new((club_idx as u32) + 1);
        let qualified_id = format!("fwh.core:club_{:05}", club_id.raw());

        clubs.push(TeamTemplate {
            id: club_id,
            qualified_id,
            display_name: procgen_team.team_name.clone(),
        });
        procgen_teams.push(procgen_team);
    }

    // Snapshot club IDs in order for fixture generation. The collect into
    // a Vec and try_into a fixed-size array is safe because the loop above
    // pushed exactly CLUBS_PER_LEAGUE items.
    let club_id_vec: Vec<ClubId> = clubs.iter().map(|c| c.id).collect();
    let fixtures = generate_fixtures_from_slice(&club_id_vec);

    // League name: MVP shape is a fixed-template-with-seed-suffix string.
    // T3+ may procgen this via a dedicated `league_name` markov chain
    // per culture; out of scope for T2-2 per the MEMORY task-spec
    // "Intentionally NOT done" list.
    let name = format!("Procedural League ({:#018x})", seed.to_u64());

    Ok((
        League {
            name,
            clubs,
            fixtures,
        },
        procgen_teams,
    ))
}

/// Generate a deterministic double-round-robin schedule for an arbitrary club
/// slice. Generalises `generate_fixtures` to any non-zero even club count —
/// the N×22 roster builder iterates the actual league rather than assuming
/// `CLUBS_PER_LEAGUE`.
///
/// # Panics
///
/// Panics if `club_ids` is empty or has an odd length (a round-robin schedule
/// requires at least 2 clubs and an even count for full pairing).
pub fn generate_fixtures_from_slice(club_ids: &[ClubId]) -> Vec<Fixture> {
    let n = club_ids.len();
    assert!(
        n >= 2 && n.is_multiple_of(2),
        "generate_fixtures_from_slice requires an even non-zero club count, got {n}"
    );
    let rounds_per_leg = n - 1;
    let half = n / 2;

    // Upper bound: each pair plays twice; n*(n-1) total fixtures.
    let mut fixtures: Vec<Fixture> = Vec::with_capacity(n * (n - 1));

    let mut rotating: Vec<usize> = (1..n).collect();

    for round in 0..rounds_per_leg {
        let match_day = (round as u16) + 1;
        let pinned_opponent_idx = rotating[n - 2];
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

        for i in 0..(half - 1) {
            let a_idx = rotating[i];
            let b_idx = rotating[n - 3 - i];
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

        let last = rotating[n - 2];
        for i in (1..n - 1).rev() {
            rotating[i] = rotating[i - 1];
        }
        rotating[0] = last;
    }

    let first_leg_count = fixtures.len();
    for i in 0..first_leg_count {
        let original = fixtures[i];
        fixtures.push(Fixture {
            home: original.away,
            away: original.home,
            match_day: original.match_day + rounds_per_leg as u16,
        });
    }

    fixtures.sort_by_key(|f| (f.match_day, f.home.raw(), f.away.raw()));
    fixtures
}

// ---------------------------------------------------------------------------
// SeasonState — mutable season-progress wrapper consumed by fw-tauri T2-5
// ---------------------------------------------------------------------------

/// Final score for a played fixture.
///
/// Stored by `SeasonState::apply_result`; read by `standings()` +
/// `fixtures_for_club()`. Scores are `u8` matching `MatchState::home_score` /
/// `MatchState::away_score` exactly (no widening at the data layer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchOutcome {
    pub home_score: u8,
    pub away_score: u8,
}

/// One row in the league standings table.
///
/// `goal_difference` is `i32` — can be negative (goals_for - goals_against).
/// All other counts fit in `u16` (max 38 played × max 99 goals = 3762 goals,
/// comfortably within 65535).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandingsRow {
    pub club_id: ClubId,
    pub club_name: String,
    pub played: u16,
    pub wins: u16,
    pub draws: u16,
    pub losses: u16,
    pub goals_for: u16,
    pub goals_against: u16,
    pub goal_difference: i32,
    pub points: u16,
}

/// Full league table, sorted canonical order:
/// `(points DESC, goal_difference DESC, goals_for DESC, club_id ASC)`.
///
/// Length is always `CLUBS_PER_LEAGUE`. Each `Standings::rows` entry
/// corresponds to one club; the position is the 0-indexed rank.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Standings {
    pub rows: Vec<StandingsRow>,
}

/// Active season progress: which match-days have been played and what the
/// final scores were.
///
/// ## Invariants
///
/// - `league.clubs.len() == CLUBS_PER_LEAGUE` (enforced by `generate_league`).
/// - `current_match_day` starts at `1` and increments by 1 per
///   `advance_week` call. `is_complete()` returns true when
///   `current_match_day > MATCH_DAYS_PER_SEASON`.
/// - `results` contains exactly the fixtures already played; unplayed
///   fixtures are absent.
/// - `tactical_archetype_ids` has one entry per `ClubId` in `league.clubs`.
///
/// ## Field visibility
///
/// All fields are `pub` because `fw-tauri` (a different crate) must read
/// them to orchestrate IPC commands. However, the "correct" mutation path is
/// `apply_result()` + bumping `current_match_day` — direct field mutation
/// bypasses the invariants above. Use the accessors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeasonState {
    pub league: League,
    /// Next match-day to be played (1-indexed). Starts at 1; advances to
    /// `MATCH_DAYS_PER_SEASON + 1` when the season is complete.
    pub current_match_day: u16,
    /// Results keyed by `(home, away)` `ClubId` pair — the same pair as the
    /// `Fixture`. `BTreeMap` for deterministic iteration.
    pub results: BTreeMap<(ClubId, ClubId), MatchOutcome>,
    /// Per-club tactical archetype ID, derived at construction via the same
    /// round-robin used by `generate_league`. Stored here because
    /// `TeamTemplate` doesn't carry `tactical_archetype_id` (out of T2-5 scope
    /// to change `team.rs`).
    pub tactical_archetype_ids: BTreeMap<ClubId, String>,
}

impl SeasonState {
    /// Construct a fresh season state for the given league.
    ///
    /// `tactical_archetype_ids` is assigned via the same round-robin as
    /// `generate_league`: club at index `i` gets
    /// `archetype_ids[i % archetype_ids.len()]`.
    ///
    /// Starts at `current_match_day = 1`; `results` is empty (no fixtures
    /// played yet).
    pub fn new(league: League, content: &ContentStore) -> Self {
        // Mirror the archetype round-robin from generate_league. BTreeMap
        // iteration order is key-sorted — same as generate_league — so the
        // assignment is identical to what the procgen loop did.
        let archetype_ids: Vec<&str> = content
            .tactical_archetypes
            .keys()
            .map(String::as_str)
            .collect();
        // Invariant: `archetype_ids` is non-empty here. Reachability proof:
        // `SeasonState::new` is only called by `AppState::new_with_career_seed`
        // AFTER `generate_league(career_seed, content)` returned `Ok(league)`.
        // `generate_league` itself FAILS with `ProcGenError::MissingTacticalArchetype`
        // if `content.tactical_archetypes.is_empty()` (see league.rs:259-263).
        // Therefore, by the time we reach here, the archetypes BTreeMap is
        // guaranteed non-empty.
        //
        // Post-T2-5 silent-failure-hunter P0 fix: the prior code held a silent
        // string-literal fallback (`"fwh.core:archetype.attacking-fullback"`)
        // that duplicated `fw_match_sim::DEFAULT_ARCHETYPE_ID` with only a
        // comment as sync mechanism — a latent drift footgun if DEFAULT_ARCHETYPE_ID
        // were renamed, AND a misleading comment that claimed
        // `ContentStore::load_sources` validates non-emptiness (it does NOT;
        // `generate_league` is the actual gatekeeper). Both removed.
        assert!(
            !archetype_ids.is_empty(),
            "SeasonState::new precondition violated: tactical_archetypes empty after \
             successful generate_league(). This is a defect — generate_league must \
             have returned Err(MissingTacticalArchetype) instead of Ok(_)."
        );
        let tactical_archetype_ids: BTreeMap<ClubId, String> = league
            .clubs
            .iter()
            .enumerate()
            .map(|(i, club)| {
                let arch_id = archetype_ids[i % archetype_ids.len()].to_owned();
                (club.id, arch_id)
            })
            .collect();

        SeasonState {
            league,
            current_match_day: 1,
            results: BTreeMap::new(),
            tactical_archetype_ids,
        }
    }

    /// Returns `true` when all `MATCH_DAYS_PER_SEASON` match-days have been
    /// played (i.e., `current_match_day > MATCH_DAYS_PER_SEASON`).
    pub fn is_complete(&self) -> bool {
        self.current_match_day > MATCH_DAYS_PER_SEASON
    }

    /// Return all fixtures scheduled for `match_day`. Empty `Vec` if the
    /// match-day is out of range.
    pub fn fixtures_for_match_day(&self, match_day: u16) -> Vec<&Fixture> {
        self.league
            .fixtures
            .iter()
            .filter(|f| f.match_day == match_day)
            .collect()
    }

    /// Record the outcome of a played fixture.
    ///
    /// Overwrites a prior result for the same `(home, away)` pair — this
    /// permits re-running a match-day in tests without special-casing.
    ///
    /// # Panics
    ///
    /// T2-R-C6 (post-T2 ultimate-review Track C-6): panics if `home`
    /// or `away` is not a member of `self.league.clubs`. The current
    /// call path (`advance_week_inner` / `play_fixtures_inner` in
    /// fw-tauri) always passes clubs sourced from
    /// `self.fixtures_for_match_day`, so this is structurally
    /// unreachable today. But `apply_result` accepts arbitrary
    /// `ClubId`s, and `Season::standings`'s aggregator silently drops
    /// any result whose ids are not in `league.clubs` (the
    /// `rows.get_mut(...)` returns None and the goals vanish from the
    /// standings). Guarding here at write time so a future caller
    /// (test fixture / mod overlay / save-load round-trip) that
    /// fabricates a stale ClubId fails loudly at the write site
    /// rather than producing invisible-scoreline corruption.
    pub fn apply_result(&mut self, home: ClubId, away: ClubId, outcome: MatchOutcome) {
        assert!(
            self.league.clubs.iter().any(|c| c.id == home),
            "Season::apply_result: home ClubId {home:?} not in league.clubs (would silently \
             drop the result from standings — see T2-R-C6)"
        );
        assert!(
            self.league.clubs.iter().any(|c| c.id == away),
            "Season::apply_result: away ClubId {away:?} not in league.clubs (would silently \
             drop the result from standings — see T2-R-C6)"
        );
        self.results.insert((home, away), outcome);
    }

    /// Compute league standings from the recorded results.
    ///
    /// Sort order (canonical): `(points DESC, goal_difference DESC,
    /// goals_for DESC, club_id ASC)`.
    pub fn standings(&self) -> Standings {
        // Build per-club accumulators. BTreeMap so the iteration below is
        // deterministic (required by Sim/RULES.md §2, which applies to any
        // computation touching canonical-adjacent state).
        let mut rows: BTreeMap<ClubId, StandingsRow> = self
            .league
            .clubs
            .iter()
            .map(|club| {
                (
                    club.id,
                    StandingsRow {
                        club_id: club.id,
                        club_name: club.display_name.clone(),
                        played: 0,
                        wins: 0,
                        draws: 0,
                        losses: 0,
                        goals_for: 0,
                        goals_against: 0,
                        goal_difference: 0,
                        points: 0,
                    },
                )
            })
            .collect();

        for ((home, away), outcome) in &self.results {
            let hg = outcome.home_score as u16;
            let ag = outcome.away_score as u16;

            if let Some(row) = rows.get_mut(home) {
                row.played += 1;
                row.goals_for += hg;
                row.goals_against += ag;
                row.goal_difference = row.goals_for as i32 - row.goals_against as i32;
                if hg > ag {
                    row.wins += 1;
                    row.points += 3;
                } else if hg == ag {
                    row.draws += 1;
                    row.points += 1;
                } else {
                    row.losses += 1;
                }
            }

            if let Some(row) = rows.get_mut(away) {
                row.played += 1;
                row.goals_for += ag;
                row.goals_against += hg;
                row.goal_difference = row.goals_for as i32 - row.goals_against as i32;
                if ag > hg {
                    row.wins += 1;
                    row.points += 3;
                } else if ag == hg {
                    row.draws += 1;
                    row.points += 1;
                } else {
                    row.losses += 1;
                }
            }
        }

        let mut sorted: Vec<StandingsRow> = rows.into_values().collect();
        // Canonical sort: points DESC, then goal_difference DESC, then
        // goals_for DESC, then club_id ASC (tie-break determinism).
        sorted.sort_by(|a, b| {
            b.points
                .cmp(&a.points)
                .then(b.goal_difference.cmp(&a.goal_difference))
                .then(b.goals_for.cmp(&a.goals_for))
                .then(a.club_id.cmp(&b.club_id))
        });

        Standings { rows: sorted }
    }

    /// Return all fixtures involving `club_id` (19 home + 19 away = 38 total),
    /// in match-day order, each paired with its `Option<MatchOutcome>`.
    ///
    /// Returns an empty `Vec` if `club_id` is not in the league.
    pub fn fixtures_for_club(&self, club_id: ClubId) -> Vec<(Fixture, Option<MatchOutcome>)> {
        self.league
            .fixtures
            .iter()
            .filter(|f| f.home == club_id || f.away == club_id)
            .map(|f| {
                let outcome = self.results.get(&(f.home, f.away)).copied();
                (*f, outcome)
            })
            .collect()
    }
}

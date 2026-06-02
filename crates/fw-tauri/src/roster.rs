//! Career-roster types and career-start generation.
//!
//! `PlayerInstance` is the mutable per-player career record owned by
//! `CareerState::roster`. It is distinct from `PlayerBio` (stable identity,
//! lives in `fw-content`) and from `PlayerTemplate` (content-authored
//! attributes, lives in `fw-content`). The instance holds the mutable career
//! state: current attributes, ceiling, breakthrough state, and season stats.
//!
//! ## Determinism contract
//!
//! - `PlayerId` derivation: `ROSTER_PLAYER_ID_BASE + club_index * SLOTS_PER_CLUB + slot`.
//!   Unique BY CONSTRUCTION — distinct `(club_index, slot)` pairs map to
//!   distinct values; no birthday collisions possible. The base offset (1_000_000)
//!   ensures no overlap with content-bio suffix ids (1..=99_999). Max id for a
//!   96-club pyramid: 1_002_111. Does not encode `career_seed` — per-career
//!   variation flows through names/attributes/template selection, not id values.
//! - `BTreeMap<ClubId, Vec<PlayerInstance>>` — `ClubId` iteration is
//!   deterministic (Sim/RULES.md §2); inner `Vec` is slot-ordered (GK=0).
//! - No `HashMap`/`HashSet` anywhere in this module (Sim/RULES.md §2).
//! - No floats in canonical types (Sim/RULES.md §1). `PlayerSeasonStats`
//!   stores `average_rating` as a Q32 numerator + sample count; division
//!   to f64 happens at DTO projection time only.
//! - No clocks or thread_rng (Sim/RULES.md §3/§4).
//!
//! ## Forward-compatibility (Decision 5, 2026-06-02)
//!
//! The generation function iterates `league.clubs` directly — it never
//! assumes 20 clubs. The bijective `PlayerId` scheme is collision-free at any
//! club count: max id at 96-club pyramid scale is 1_002_111, well within `u32`.
//! `SaveV4` (T4-2.5g) will serialise `BTreeMap<ClubId, Vec<PlayerInstance>>`
//! unchanged; `ROSTER_PLAYER_ID_BASE` is frozen from that point.

use std::collections::BTreeMap;

use fw_content::{ProcGenTeam, SignatureCandidate, generate_league_with_teams};
use fw_core::{AbilityCeiling, ClubId, PlayerAttributes, PlayerId, Seed};
use fw_memory::BreakthroughState;

/// Slots per club squad. Drives the bijective `PlayerId` scheme:
/// `PlayerId = ROSTER_PLAYER_ID_BASE + club_index * SLOTS_PER_CLUB as u32 + slot as u32`.
///
/// Matches `fw-content-baker::validators::MVP_ROSTER_SIZE = 22` (two teams
/// of eleven). That constant lives in the baker crate (validation); this one
/// lives in the generation path. Keep both in sync if the squad size ever
/// changes (a schema bump would accompany such a change anyway).
pub const SLOTS_PER_CLUB: u8 = 22;

/// Base offset for roster `PlayerId` values.
///
/// Roster ids are `ROSTER_PLAYER_ID_BASE + club_index * SLOTS_PER_CLUB + slot`.
/// The base is chosen to be strictly above any plausible content-bio suffix
/// (content bios use `_00001`–`_99999` → raw suffixes 1–99_999). Setting the
/// base to 1_000_000 guarantees the roster id space (1_000_000..~1_002_111 for
/// a 96-club pyramid) never overlaps the content-bio id space, preventing the
/// chimera: `get_player_detail("fwh.core:player_00022")` querying the ledger
/// for `PlayerId(22)` which belongs to a DIFFERENT roster player.
///
/// This offset is non-canonical (roster ids live only in `CareerState` and the
/// ledger's `by_subject` index, not in `MatchState` canonical encoding) → no
/// canonical-hash impact. SaveV4 (T4-2.5g) will freeze this value; changing it
/// after T4-2.5g requires a save-migration.
pub const ROSTER_PLAYER_ID_BASE: u32 = 1_000_000;

// ---------------------------------------------------------------------------
// PlayerSeasonStats — float-free per-season accumulator
// ---------------------------------------------------------------------------

/// Per-season performance statistics for one player instance.
///
/// `average_rating_numerator` accumulates the sum of per-match Q32 ratings;
/// divide by `rating_sample_count` at DTO projection time to avoid float
/// in canonical state (Sim/RULES.md §1).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub struct PlayerSeasonStats {
    pub appearances: u16,
    pub goals: u16,
    pub assists: u16,
    /// Total minutes played across all appearances this season.
    pub minutes_played: u32,
    /// Running sum of per-match Q32 ratings (numerator; divide by
    /// `rating_sample_count` to obtain the mean at DTO time).
    pub average_rating_numerator: i64,
    /// Number of match ratings recorded in `average_rating_numerator`.
    pub rating_sample_count: u16,
}

// ---------------------------------------------------------------------------
// PlayerInstance — mutable per-player career record
// ---------------------------------------------------------------------------

/// A single player in the career roster.
///
/// Identity (name, bio labels, gene snapshot) lives in `PlayerBio`/`PlayerTemplate`
/// in the content store, keyed by the same `player_id`. This struct holds only
/// the mutable career state so save/load doesn't duplicate immutable content.
///
/// Field order is stable for serde determinism — do not reorder.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PlayerInstance {
    /// Durable career-unique player handle.
    ///
    /// Derived bijectively from `(club_index, slot)`:
    /// `ROSTER_PLAYER_ID_BASE + club_index * SLOTS_PER_CLUB as u32 + slot as u32`.
    /// Unique by construction — no two distinct `(club_index, slot)` pairs
    /// produce the same id. The base offset (1_000_000) ensures no overlap with
    /// content-bio suffix ids (1..=99_999). Max value at 96-club pyramid: 1_002_111.
    pub player_id: PlayerId,

    /// Current club affiliation.
    pub club_id: ClubId,

    /// Slot within the club's squad (0 = GK; positional meaning for slots
    /// 1–21 lands at T4.5-E1 when the gene→attribute compiler wires roles).
    pub slot: u8,

    /// Display name — sourced from the per-club `ProcGenTeam` at career start.
    /// Stored here so the roster DTO doesn't need to re-derive names.
    pub display_name: String,

    /// Current 55-field attribute bundle; mutable via breakthroughs (T4-2.5d).
    pub attributes: PlayerAttributes,

    /// Current ability + potential ability ceiling.
    pub ceiling: AbilityCeiling,

    /// Signature affinities. Sourced from the content pool at career start;
    /// mutable via breakthrough unlocks (T4-2.5c/d).
    pub signature_candidates: Vec<SignatureCandidate>,

    /// Breakthrough readiness / regressive pressure state.
    pub breakthrough_state: BreakthroughState,

    /// Current-season performance statistics. Reset at `advance_season`.
    pub season_stats: PlayerSeasonStats,

    /// Total career appearances across all seasons.
    pub career_apps: u32,

    /// Number of times an observer has run `observe_player` on this instance.
    /// Drives pillar-4 scouting uncertainty (T4-2.5f).
    pub observation_count: u32,
}

// ---------------------------------------------------------------------------
// Career-start generation
// ---------------------------------------------------------------------------

/// Errors that can occur during roster generation.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RosterGenError {
    /// League generation failed — typically an empty content store.
    #[error("league generation failed: {0}")]
    LeagueGen(#[from] fw_content::ProcGenError),

    /// The content store has no `PlayerTemplate` records.
    ///
    /// At T4-2.5b there is 1 template; T4.5-E1 adds the full compiler.
    /// An empty template pool would produce placeholder-only instances.
    #[error("content store has no PlayerTemplate records; cannot generate roster")]
    NoPlayerTemplates,
}

/// Generate the career-start roster from an already-generated league.
///
/// Accepts `(league, procgen_teams)` produced by `generate_league_with_teams`
/// so the caller (always `AppState::new_with_settings_path`) can build both
/// the `SeasonState` and the roster from one procgen pass. This avoids the
/// double-call that a `career_seed`-only entry point would require.
///
/// For each club, `SLOTS_PER_CLUB` (= 22) `PlayerInstance` records are built:
///
/// - `player_id`: bijective `club_index * SLOTS_PER_CLUB as u32 + slot as u32`
///   — unique by construction, no hash/birthday risk.
/// - `display_name` from `procgen_teams[i].players[slot]`.
/// - `attributes`, `ceiling`, `signature_candidates` from the `PlayerTemplate`
///   pool indexed `slot % pool_len`. With 1 template, all players share the
///   same baseline; T4.5-E1 replaces this with the gene→attribute compiler.
///
/// Returns `BTreeMap<ClubId, Vec<PlayerInstance>>` keyed by `ClubId`;
/// inner `Vec` has exactly `SLOTS_PER_CLUB` entries, slot-ordered (GK=0).
pub fn build_roster_from_league(
    league: &fw_content::League,
    procgen_teams: &[ProcGenTeam],
    content: &fw_content::ContentStore,
) -> Result<BTreeMap<ClubId, Vec<PlayerInstance>>, RosterGenError> {
    // Template pool: BTreeMap values() is key-ordered (Sim/RULES.md §2),
    // so pool[i] is deterministic across platforms.
    let templates: Vec<&fw_content::PlayerTemplate> = content.player_templates.values().collect();
    if templates.is_empty() {
        return Err(RosterGenError::NoPlayerTemplates);
    }

    let mut roster: BTreeMap<ClubId, Vec<PlayerInstance>> = BTreeMap::new();

    for (club_idx, (club, procgen_team)) in
        league.clubs.iter().zip(procgen_teams.iter()).enumerate()
    {
        let mut instances: Vec<PlayerInstance> = Vec::with_capacity(SLOTS_PER_CLUB as usize);
        for slot in 0u8..SLOTS_PER_CLUB {
            // Bijective PlayerId with base offset to avoid overlap with content-bio
            // suffix ids. Max at 96-club pyramid: 1_000_000 + 95*22 + 21 = 1_002_111.
            let player_id = PlayerId::new(
                ROSTER_PLAYER_ID_BASE + (club_idx as u32) * (SLOTS_PER_CLUB as u32) + (slot as u32),
            );

            // Template round-robin: deterministic, N-template-safe.
            let template = templates[slot as usize % templates.len()];

            let display_name = procgen_team.players[slot as usize].display();

            instances.push(PlayerInstance {
                player_id,
                club_id: club.id,
                slot,
                display_name,
                attributes: template.attributes,
                ceiling: template.ceiling,
                signature_candidates: template.signature_candidates.clone(),
                breakthrough_state: BreakthroughState::new(),
                season_stats: PlayerSeasonStats::default(),
                career_apps: 0,
                observation_count: 0,
            });
        }
        roster.insert(club.id, instances);
    }

    Ok(roster)
}

/// Convenience wrapper: generate the league, then build the roster.
///
/// Exists for tests that only need the roster and don't want to manage the
/// `(League, Vec<ProcGenTeam>)` pair. Production code uses
/// [`build_roster_from_league`] directly (via `AppState::new_with_settings_path`)
/// to avoid a second procgen pass.
pub fn generate_career_roster(
    career_seed: Seed,
    content: &fw_content::ContentStore,
) -> Result<BTreeMap<ClubId, Vec<PlayerInstance>>, RosterGenError> {
    let (league, procgen_teams) = generate_league_with_teams(career_seed, content)?;
    build_roster_from_league(&league, &procgen_teams, content)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn workspace_content_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("content")
    }

    fn load_content() -> fw_content::ContentStore {
        fw_content::ContentStore::load_sources(&workspace_content_path())
            .expect("ContentStore::load_sources failed in test")
    }

    /// AC1/AC5 — each club owns exactly 22 instances; all PlayerIds distinct.
    #[test]
    fn roster_generation_gives_22_per_club_distinct_ids() {
        let content = load_content();
        let seed = Seed::from_u64(0xDEAD_BEEF_CAFE_BABE);
        let roster = generate_career_roster(seed, &content).expect("generate_career_roster failed");

        // Every club has exactly 22 instances.
        for (club_id, instances) in &roster {
            assert_eq!(
                instances.len(),
                22,
                "club {:?} has {} instances, expected 22",
                club_id,
                instances.len()
            );
        }

        // All PlayerIds across the entire roster are distinct.
        let all_ids: BTreeSet<PlayerId> = roster
            .values()
            .flat_map(|instances| instances.iter().map(|p| p.player_id))
            .collect();
        let total = roster.values().map(|v| v.len()).sum::<usize>();
        assert_eq!(
            all_ids.len(),
            total,
            "PlayerIds are not all distinct: {} unique out of {} total",
            all_ids.len(),
            total
        );
    }

    /// AC5 — same career seed → identical roster (determinism).
    #[test]
    fn roster_is_deterministic_for_same_seed() {
        let content = load_content();
        let seed = Seed::from_u64(0xFEED_BEEF_CAFE_FADE);

        let roster_a = generate_career_roster(seed, &content).expect("roster_a");
        let roster_b = generate_career_roster(seed, &content).expect("roster_b");

        assert_eq!(
            roster_a, roster_b,
            "same career seed must produce identical roster"
        );
    }

    /// AC2 — default 20-club league → roster has 440 instances total.
    #[test]
    fn default_league_roster_has_440_instances() {
        let content = load_content();
        let seed = Seed::from_u64(0xCAFE_BABE);
        let roster = generate_career_roster(seed, &content).expect("generate_career_roster failed");

        let total: usize = roster.values().map(|v| v.len()).sum();
        assert_eq!(total, 440, "20 clubs × 22 players = 440 instances");
    }

    /// Slot ordering — GK (slot 0) is the first instance in each club's Vec.
    #[test]
    fn slot_ordering_gk_first() {
        let content = load_content();
        let seed = Seed::from_u64(1);
        let roster = generate_career_roster(seed, &content).expect("generate_career_roster failed");
        for (club_id, instances) in &roster {
            assert_eq!(
                instances[0].slot, 0,
                "first instance for club {:?} must be GK (slot 0)",
                club_id
            );
        }
    }

    /// AC5 (Decision 5 forward-compat) — roster club count equals `league.clubs.len()`.
    ///
    /// Verifies the generation loop iterates `league.clubs` directly and
    /// does not hard-code 20 (Decision 5, 2026-06-02). Asserts:
    /// - `roster.len() == league.clubs.len()` (not pinned at 20)
    /// - total instances == clubs × SLOTS_PER_CLUB
    /// - all PlayerIds distinct
    #[test]
    fn roster_generation_not_hardcoded_to_20_clubs() {
        let content = load_content();
        let seed = Seed::from_u64(0xABCD_1234);

        let roster = generate_career_roster(seed, &content).expect("generate_career_roster");

        // Derive the club count from the actual league — not the literal 20.
        let (league, _) = fw_content::generate_league_with_teams(seed, &content).expect("league");

        assert_eq!(
            roster.len(),
            league.clubs.len(),
            "roster club count must equal league.clubs.len() — not a hardcoded 20"
        );

        let total: usize = roster.values().map(|v| v.len()).sum();
        assert_eq!(
            total,
            league.clubs.len() * SLOTS_PER_CLUB as usize,
            "total instances must be clubs × SLOTS_PER_CLUB"
        );

        // All PlayerIds must be distinct.
        let all_ids: BTreeSet<PlayerId> = roster
            .values()
            .flat_map(|v| v.iter().map(|p| p.player_id))
            .collect();
        assert_eq!(all_ids.len(), total, "all PlayerIds must be distinct");
    }

    /// PlayerId bijection holds at pyramid scale (96 clubs × 22 slots).
    ///
    /// Exercises the bijective formula
    /// `ROSTER_PLAYER_ID_BASE + club_idx * SLOTS_PER_CLUB + slot`
    /// across the full EA pyramid population without needing a 96-club
    /// ContentStore. Proves:
    /// (a) all 2112 ids are distinct in a BTreeSet,
    /// (b) min id == ROSTER_PLAYER_ID_BASE, strictly above the content-bio
    ///     suffix range (> 100_000) so the two id spaces never collide,
    /// (c) max id == ROSTER_PLAYER_ID_BASE + 95*22 + 21 = 1_002_111.
    #[test]
    fn player_id_scheme_is_bijective_at_pyramid_scale() {
        const PYRAMID_CLUBS: u32 = 96;
        let ids: BTreeSet<PlayerId> = (0..PYRAMID_CLUBS)
            .flat_map(|club_idx| {
                (0u8..SLOTS_PER_CLUB).map(move |slot| {
                    PlayerId::new(
                        ROSTER_PLAYER_ID_BASE + club_idx * (SLOTS_PER_CLUB as u32) + (slot as u32),
                    )
                })
            })
            .collect();

        let expected_count = (PYRAMID_CLUBS * SLOTS_PER_CLUB as u32) as usize;
        assert_eq!(
            ids.len(),
            expected_count,
            "bijective scheme must produce {expected_count} distinct ids for \
             {PYRAMID_CLUBS} clubs × {SLOTS_PER_CLUB} slots"
        );

        // Min id is the base — GK of club 0.
        let min_id = ids.iter().next().unwrap().raw();
        assert_eq!(
            min_id, ROSTER_PLAYER_ID_BASE,
            "min roster PlayerId must equal ROSTER_PLAYER_ID_BASE"
        );

        // The base must be strictly greater than the content-bio suffix range.
        // Content bios use suffixes 1..=99_999 → PlayerId(1)..=PlayerId(99_999).
        // This is a compile-time guarantee: the `const _` line below makes the
        // check part of the crate's type-system invariants rather than a
        // runtime assert (which clippy correctly flags as "constant value").
        const _: () = assert!(
            ROSTER_PLAYER_ID_BASE > 100_000,
            "ROSTER_PLAYER_ID_BASE must be > 100_000 to clear the content-bio suffix range"
        );

        // Max id at pyramid scale.
        let max_id = ROSTER_PLAYER_ID_BASE
            + (PYRAMID_CLUBS - 1) * SLOTS_PER_CLUB as u32
            + (SLOTS_PER_CLUB as u32 - 1);
        assert_eq!(
            max_id, 1_002_111,
            "max PlayerId at 96-club pyramid scale must be 1_002_111"
        );
        assert_eq!(*ids.iter().next_back().unwrap(), PlayerId::new(max_id));
    }
}

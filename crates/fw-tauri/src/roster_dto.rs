//! Roster DTOs — read-only projections of `PlayerInstance` for the frontend.
//!
//! `PlayerRosterDto` is a one-way projection (Tauri/RULES.md §3):
//! - All numeric Q32 values are converted to `f64` at the boundary.
//! - The DTO is NEVER serialized back into canonical state.
//! - `#[serde(rename_all = "camelCase")]` so TypeScript receives `playerId`,
//!   not `player_id`.
//!
//! DTOs defined here (T4-2.5b scope):
//!   - `PlayerRosterDto` — identity + role + zeroed season stats.
//!
//! Deferred to later sub-rows (NOT here):
//!   - `ScoutReportDto` / `CategoryEstimateDto` / `LabelEstimateDto` — T4-2.5f.
//!   - Per-match stat accumulation — T4-2.5h.

use crate::roster::PlayerInstance;

/// One player row returned by `get_roster_for_club`.
///
/// Contains identity (player id, name, club id, preferred role) and
/// season statistics (all zero at career start). No overall/rating number —
/// that would violate the banned-terms rule against visible stat-number tooltips
/// (CLAUDE.md §7 + `Content/RULES.md §5`).
///
/// The `preferredRole` string is the display label from the content store
/// at DTO-projection time.  At T4-2.5b, it is derived from the
/// `PlayerTemplate.preferred_role` field; a richer "preferred position per
/// club-tactics" lookup is T4-2.5f scope.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerRosterDto {
    /// Durable career-unique handle. Wire form: u32 (serde-transparent on PlayerId).
    pub player_id: u32,
    /// Display name for this player.
    pub name: String,
    /// Raw u32 of the club this player belongs to.
    pub club_id: u32,
    /// Slot within the squad (0 = GK, 1–21 = outfield).
    pub slot: u8,
    /// Appearances this season.
    pub appearances: u16,
    /// Goals this season.
    pub goals: u16,
    /// Assists this season.
    pub assists: u16,
    /// Minutes played this season.
    pub minutes_played: u32,
}

impl PlayerRosterDto {
    /// Project a `PlayerInstance` into the DTO.
    pub fn from_instance(instance: &PlayerInstance) -> Self {
        PlayerRosterDto {
            player_id: instance.player_id.raw(),
            name: instance.display_name.clone(),
            club_id: instance.club_id.raw(),
            slot: instance.slot,
            appearances: instance.season_stats.appearances,
            goals: instance.season_stats.goals,
            assists: instance.season_stats.assists,
            minutes_played: instance.season_stats.minutes_played,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roster::default_gene_snapshot;
    use crate::roster::{PlayerInstance, PlayerSeasonStats};
    use fw_core::{AbilityCeiling, ClubId, PlayerAttributes, PlayerId, Q32};
    use fw_core::{
        DurabilityProfile, GoalkeeperAttributes, MentalAttributes, PersonalityVector,
        PhysicalAttributes, TechnicalAttributes,
    };
    use fw_memory::BreakthroughState;

    fn zero_attributes() -> PlayerAttributes {
        let z = Q32::ZERO;
        PlayerAttributes {
            technical: TechnicalAttributes {
                finishing: z,
                long_shots: z,
                passing: z,
                crossing: z,
                first_touch: z,
                technique: z,
                dribbling: z,
                heading: z,
                tackling: z,
                marking: z,
                free_kicks: z,
                penalty_taking: z,
                corners: z,
                long_throws: z,
            },
            mental: MentalAttributes {
                anticipation: z,
                composure: z,
                decisions: z,
                vision: z,
                off_the_ball: z,
                positioning: z,
                concentration: z,
                bravery: z,
                teamwork: z,
                flair: z,
            },
            physical: PhysicalAttributes {
                pace: z,
                acceleration: z,
                stamina: z,
                strength: z,
                agility: z,
                balance: z,
                jumping_reach: z,
                natural_fitness: z,
            },
            goalkeeper: GoalkeeperAttributes {
                handling: z,
                reflexes: z,
                one_on_ones: z,
                aerial_reach: z,
                command_of_area: z,
                kicking: z,
            },
            personality: PersonalityVector {
                determination: z,
                work_rate: z,
                ambition: z,
                professionalism: z,
                loyalty: z,
                temperament: z,
                pressure_tolerance: z,
                big_match_appetite: z,
                adaptability: z,
                aggression: z,
                risk_appetite: z,
                selflessness: z,
                consistency: z,
                versatility: z,
            },
            durability: DurabilityProfile {
                injury_proneness: z,
                recovery_rate: z,
                dirtiness: z,
            },
        }
    }

    fn sample_instance(player_id: u32, club_id: u32, slot: u8, name: &str) -> PlayerInstance {
        let half = Q32::from_raw(1i64 << 31);
        PlayerInstance {
            player_id: PlayerId::new(player_id),
            club_id: ClubId::new(club_id),
            slot,
            display_name: name.to_string(),
            attributes: zero_attributes(),
            ceiling: AbilityCeiling::try_new(half, Q32::from_raw(3i64 << 30)).expect("ceiling"),
            signature_candidates: vec![],
            breakthrough_state: BreakthroughState::new(),
            season_stats: PlayerSeasonStats {
                appearances: 5,
                goals: 3,
                assists: 1,
                minutes_played: 450,
                average_rating_numerator: 0,
                rating_sample_count: 0,
            },
            career_apps: 5,
            observation_count: 0,
            genes: default_gene_snapshot(),
        }
    }

    #[test]
    fn player_roster_dto_from_instance_projects_correctly() {
        let instance = sample_instance(42, 7, 9, "Elliot Ashby");
        let dto = PlayerRosterDto::from_instance(&instance);

        assert_eq!(dto.player_id, 42);
        assert_eq!(dto.club_id, 7);
        assert_eq!(dto.slot, 9);
        assert_eq!(dto.name, "Elliot Ashby");
        assert_eq!(dto.appearances, 5);
        assert_eq!(dto.goals, 3);
        assert_eq!(dto.assists, 1);
        assert_eq!(dto.minutes_played, 450);
    }

    #[test]
    fn player_roster_dto_serializes_camel_case() {
        let instance = sample_instance(1, 2, 0, "Test Player");
        let dto = PlayerRosterDto::from_instance(&instance);
        let json = serde_json::to_string(&dto).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");

        // Fields must be camelCase on the wire.
        assert!(v.get("playerId").is_some(), "playerId missing");
        assert!(v.get("clubId").is_some(), "clubId missing");
        assert!(v.get("minutesPlayed").is_some(), "minutesPlayed missing");
        // snake_case must NOT appear.
        assert!(v.get("player_id").is_none(), "player_id must not appear");
        assert!(v.get("club_id").is_none(), "club_id must not appear");
    }
}

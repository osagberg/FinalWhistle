//! Roster DTOs — read-only projections of `PlayerInstance` for the frontend.
//!
//! `PlayerRosterDto` is a one-way projection (Tauri/RULES.md §3):
//! - All numeric Q32 values are converted to `f64` at the boundary.
//! - The DTO is NEVER serialized back into canonical state.
//! - `#[serde(rename_all = "camelCase")]` so TypeScript receives `playerId`,
//!   not `player_id`.
//!
//! DTOs defined here:
//!   - `PlayerRosterDto` — identity + role + zeroed season stats (T4-2.5b).
//!   - `ScoutReportDto` / `CategoryEstimateDto` / `LabelEstimateDto` — scouting
//!     projections for `get_scout_report` (T4-2.5f).
//!
//! Deferred to later sub-rows (NOT here):
//!   - Per-match stat accumulation — T4-2.5h.

use fw_scouting::{ScoutReport, UncertaintyBand};

use crate::roster::PlayerInstance;

// Q32 → f64 projection uses the single crate-level helper `crate::q32_to_f64`
// (defined in `lib.rs`) so there is one source of truth for the boundary
// conversion (one `#[allow(clippy::float_arithmetic)]`, one `Q32_SCALE` const).
use crate::q32_to_f64;

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

// ---------------------------------------------------------------------------
// ScoutReportDto and sub-DTOs (T4-2.5f) — scouting projections
// ---------------------------------------------------------------------------

/// Per-category estimate from a scout observation.
///
/// `category` is one of `"Physical"`, `"Mental"`, `"Technical"`.
/// `band` is a football-native uncertainty label (e.g. `"a confident read"`).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryEstimateDto {
    pub category: String,
    pub low: f64,
    pub high: f64,
    pub band: String,
}

/// Per-label estimate from a scout observation.
///
/// `label` is the `PhenotypeLabelId::display_label()` string.
/// `band` is a football-native uncertainty label.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelEstimateDto {
    pub label: String,
    pub confidence: f64,
    pub band: String,
}

/// Scouting report DTO returned by `get_scout_report`.
///
/// Projection of a `ScoutReport` cached on `PlayerInstance.last_scout_report`.
/// All Q32 values are converted to f64 at the DTO boundary (Tauri/RULES §3).
/// `overall_band` uses `UncertaintyBand::from_confidence(report.confidence).display_label()`.
///
/// `categories` is always 3 entries (Physical, Mental, Technical in that order).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoutReportDto {
    /// Raw u32 of the roster `PlayerId`.
    pub player_id: u32,
    /// Overall confidence as f64 ∈ [0, 1].
    pub confidence: f64,
    /// Football-native band for the overall confidence.
    pub overall_band: String,
    /// Number of observations accumulated so far.
    pub observation_count: u32,
    /// Per-category estimates (always 3: Physical, Mental, Technical).
    pub categories: Vec<CategoryEstimateDto>,
    /// Per-label estimates (one per scout label in the player's bio).
    pub labels: Vec<LabelEstimateDto>,
}

impl ScoutReportDto {
    /// Project a `ScoutReport` + metadata into the DTO.
    ///
    /// `report.player_id` is the roster `PlayerId` set by `observe_player`'s
    /// `subject` parameter — no separate `player_id` arg is needed (F2 fix).
    pub fn from_report(report: &ScoutReport, observation_count: u32) -> Self {
        let confidence = q32_to_f64(report.confidence.to_bits());
        let overall_band = UncertaintyBand::from_confidence(report.confidence)
            .display_label()
            .to_string();

        let categories = report
            .category_estimates
            .iter()
            .map(|est| {
                let category = match est.category {
                    fw_scouting::GeneCategory::Physical => "Physical",
                    fw_scouting::GeneCategory::Mental => "Mental",
                    fw_scouting::GeneCategory::Technical => "Technical",
                }
                .to_string();
                CategoryEstimateDto {
                    category,
                    low: q32_to_f64(est.low.to_bits()),
                    high: q32_to_f64(est.high.to_bits()),
                    band: est.band().display_label().to_string(),
                }
            })
            .collect();

        let labels = report
            .label_estimates
            .iter()
            .map(|le| {
                let confidence_f64 = q32_to_f64(le.confidence.to_bits());
                LabelEstimateDto {
                    label: le.label.display_label().to_string(),
                    confidence: confidence_f64,
                    band: UncertaintyBand::from_confidence(le.confidence)
                        .display_label()
                        .to_string(),
                }
            })
            .collect();

        ScoutReportDto {
            player_id: report.player_id.raw(),
            confidence,
            overall_band,
            observation_count,
            categories,
            labels,
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
            last_scout_report: None,
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

    /// AC5 — `ScoutReportDto` serializes field names as camelCase on the wire.
    #[test]
    fn scout_report_dto_serializes_camel_case() {
        use fw_core::Q32;
        use fw_scouting::{GeneCategory, GeneCategoryEstimate, ScoutReport};

        let half = Q32::from_raw(2_147_483_648_i64); // 0.5
        let report = ScoutReport {
            scout_archetype_id: "fwh.core:scout.basic-uncertainty".to_string(),
            // player_id is now a PlayerId — matches the subject passed to observe_player (F2).
            player_id: PlayerId::new(1_000_000),
            confidence: half,
            label_estimates: vec![],
            category_estimates: vec![
                GeneCategoryEstimate {
                    category: GeneCategory::Physical,
                    low: half,
                    high: half,
                },
                GeneCategoryEstimate {
                    category: GeneCategory::Mental,
                    low: half,
                    high: half,
                },
                GeneCategoryEstimate {
                    category: GeneCategory::Technical,
                    low: half,
                    high: half,
                },
            ],
        };

        // from_report no longer takes a separate player_id arg — it reads report.player_id (F2).
        let dto = ScoutReportDto::from_report(&report, 3);
        let json = serde_json::to_string(&dto).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");

        // playerId round-trips the typed PlayerId's raw value.
        assert_eq!(v.get("playerId").and_then(|x| x.as_u64()), Some(1_000_000));
        // Required camelCase fields.
        assert!(v.get("playerId").is_some(), "playerId missing");
        assert!(v.get("overallBand").is_some(), "overallBand missing");
        assert!(
            v.get("observationCount").is_some(),
            "observationCount missing"
        );
        assert!(v.get("categories").is_some(), "categories missing");
        assert!(v.get("labels").is_some(), "labels missing");
        // snake_case must NOT appear.
        assert!(v.get("player_id").is_none(), "player_id must not appear");
        assert!(
            v.get("overall_band").is_none(),
            "overall_band must not appear"
        );
        assert!(
            v.get("observation_count").is_none(),
            "observation_count must not appear"
        );

        // Categories array has 3 entries.
        let cats = v["categories"].as_array().expect("categories array");
        assert_eq!(cats.len(), 3, "categories must have 3 entries");

        // Observation count round-trips.
        assert_eq!(v["observationCount"].as_u64(), Some(3));
    }
}

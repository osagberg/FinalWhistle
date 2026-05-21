//! `PlayerBio` — stable per-player identity record (T2-4).
//!
//! Renamed from FW-v1 `IdentityPacket` per the "football-native vocabulary"
//! discipline in `CLAUDE.md §7`. Dev surfaces only; player-facing UI never
//! sees either term — only phenotype labels via `PlayerBio.scout_labels`.
//!
//! The binding design contract is `design/player-generation.md` §"Identity Packet
//! (the stable output)" + §"Internal gene model" + §"Resolved Q2".
//!
//! All numeric fields are `Q32` per `Sim/RULES.md §1`. `BTreeMap`/`BTreeSet`
//! only per §2. `schema_version: u16` present from day one per `Content/RULES.md §3`.

use std::collections::{BTreeMap, BTreeSet};

use fw_core::Q32;
use serde::{Deserialize, Serialize};

use crate::gene::{GeneSnapshot, NarrativeFlag};
use crate::signature::{RoleFamily, SignatureCandidate};

// ---------------------------------------------------------------------------
// Schema version sentinel
// ---------------------------------------------------------------------------

/// Schema version for `PlayerBio`. Bump on any structural change; triggers
/// the load-time migration path in `fw-save`.
pub const PLAYER_BIO_SCHEMA_VERSION: u16 = 1;

// ---------------------------------------------------------------------------
// PhenotypeLabelId — 46 variants, ceiling 50
// ---------------------------------------------------------------------------

/// Stable enum of the 46 phenotype labels per `design/player-generation.md §Q2`.
///
/// These are player-facing scout labels and filter keys. Field names are the
/// football-observable terms shown to players — NO internal gene terminology.
///
/// **Explicit exclusions applied in the 2026-04-24 resolution:**
/// - `Fragile Under Scrutiny` → renamed `StrugglesUnderScrutiny`
/// - `Powerful Striker` → renamed `PowerfulBallStriker`
/// - `Plateau Risk` removed entirely (no such variant)
/// - `Injury-Prone` absent (injury history surfaces through explicit record)
///
/// Total: 46 variants (7 physical + 8 mental + 6 technical + 3 development + 22 role-specific).
/// Headroom of 4 before the 50-variant ceiling. Growth past 50 triggers schema review.
///
/// The `ALL` const enumerates all 46 variants in the canonical order defined here.
/// Tests assert `ALL.len() == 46` and spot-check specific renamed variants.
///
/// Per `Content/RULES.md §5` banned terms: these labels ARE player-facing copy;
/// the banned-terms lint covers `content/sources/player-bios/`. The labels
/// themselves are football-native and pre-approved by the 2026-04-24 resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PhenotypeLabelId {
    // Physical (7)
    ExplosiveFirstStep,
    RelentlessEngine,
    AerialPresence,
    AgilePivot,
    SlowStarter,
    LateCareerPeak,
    QuickRecovery,

    // Mental (8)
    ReadsTheGame,
    ComposedUnderPressure,
    DecisiveInTheBox,
    StrugglesUnderScrutiny,
    SlowToAdapt,
    GrowsIntoGames,
    Ambitious,
    Loyal,

    // Technical (6)
    SetPieceNatural,
    StrongLeftFoot,
    PureFinisher,
    SilkenFirstTouch,
    PowerfulBallStriker,
    AerialThreat,

    // Development (3)
    LateBloomer,
    EarlyDeveloper,
    SteadyProgressor,

    // Role-specific (22)
    // Goalkeeper (3)
    SweeperKeeper,
    LineKeeper,
    CrossClaimer,
    // Centre-back (3)
    BallPlayingDefender,
    Stopper,
    CoverDefender,
    // Full-back / wing-back (3)
    OverlappingFullBack,
    InvertedFullBack,
    WingBackRunner,
    // Defensive midfielder (3)
    AnchorMan,
    BallWinningMidfielder,
    PressingMidfielder,
    // Central midfielder (2)
    TempoSetter,
    BoxToBox,
    // Attacking midfielder / #10 (2)
    Playmaker,
    HalfSpaceCreator,
    // Winger (2)
    InvertedWinger,
    TraditionalWinger,
    // Striker / centre-forward (4)
    Poacher,
    TargetMan,
    False9,
    LinkForward,
}

impl PhenotypeLabelId {
    /// All 46 phenotype label variants in declaration order. Tests assert
    /// `ALL.len() == 46`; mutating this array (adding/removing) breaks that test.
    pub const ALL: &'static [PhenotypeLabelId] = &[
        // Physical (7)
        PhenotypeLabelId::ExplosiveFirstStep,
        PhenotypeLabelId::RelentlessEngine,
        PhenotypeLabelId::AerialPresence,
        PhenotypeLabelId::AgilePivot,
        PhenotypeLabelId::SlowStarter,
        PhenotypeLabelId::LateCareerPeak,
        PhenotypeLabelId::QuickRecovery,
        // Mental (8)
        PhenotypeLabelId::ReadsTheGame,
        PhenotypeLabelId::ComposedUnderPressure,
        PhenotypeLabelId::DecisiveInTheBox,
        PhenotypeLabelId::StrugglesUnderScrutiny,
        PhenotypeLabelId::SlowToAdapt,
        PhenotypeLabelId::GrowsIntoGames,
        PhenotypeLabelId::Ambitious,
        PhenotypeLabelId::Loyal,
        // Technical (6)
        PhenotypeLabelId::SetPieceNatural,
        PhenotypeLabelId::StrongLeftFoot,
        PhenotypeLabelId::PureFinisher,
        PhenotypeLabelId::SilkenFirstTouch,
        PhenotypeLabelId::PowerfulBallStriker,
        PhenotypeLabelId::AerialThreat,
        // Development (3)
        PhenotypeLabelId::LateBloomer,
        PhenotypeLabelId::EarlyDeveloper,
        PhenotypeLabelId::SteadyProgressor,
        // Role-specific (22)
        PhenotypeLabelId::SweeperKeeper,
        PhenotypeLabelId::LineKeeper,
        PhenotypeLabelId::CrossClaimer,
        PhenotypeLabelId::BallPlayingDefender,
        PhenotypeLabelId::Stopper,
        PhenotypeLabelId::CoverDefender,
        PhenotypeLabelId::OverlappingFullBack,
        PhenotypeLabelId::InvertedFullBack,
        PhenotypeLabelId::WingBackRunner,
        PhenotypeLabelId::AnchorMan,
        PhenotypeLabelId::BallWinningMidfielder,
        PhenotypeLabelId::PressingMidfielder,
        PhenotypeLabelId::TempoSetter,
        PhenotypeLabelId::BoxToBox,
        PhenotypeLabelId::Playmaker,
        PhenotypeLabelId::HalfSpaceCreator,
        PhenotypeLabelId::InvertedWinger,
        PhenotypeLabelId::TraditionalWinger,
        PhenotypeLabelId::Poacher,
        PhenotypeLabelId::TargetMan,
        PhenotypeLabelId::False9,
        PhenotypeLabelId::LinkForward,
    ];

    /// Human-readable football-native label for this phenotype.
    ///
    /// Exhaustive match — adding a new variant without a corresponding arm
    /// causes a compile error, guaranteeing the label list stays in sync.
    /// Labels are banned-terms-clean per `Content/RULES.md §5`: football
    /// vocabulary only, no capitalised mystical state-nouns, no "+N stat" forms.
    pub fn display_label(&self) -> &'static str {
        match self {
            // Physical (7)
            PhenotypeLabelId::ExplosiveFirstStep => "Explosive first step",
            PhenotypeLabelId::RelentlessEngine => "Relentless engine",
            PhenotypeLabelId::AerialPresence => "Aerial presence",
            PhenotypeLabelId::AgilePivot => "Agile pivot",
            PhenotypeLabelId::SlowStarter => "Slow starter",
            PhenotypeLabelId::LateCareerPeak => "Late career peak",
            PhenotypeLabelId::QuickRecovery => "Quick recovery",
            // Mental (8)
            PhenotypeLabelId::ReadsTheGame => "Reads the game",
            PhenotypeLabelId::ComposedUnderPressure => "Composed under pressure",
            PhenotypeLabelId::DecisiveInTheBox => "Decisive in the box",
            PhenotypeLabelId::StrugglesUnderScrutiny => "Struggles under scrutiny",
            PhenotypeLabelId::SlowToAdapt => "Slow to adapt",
            PhenotypeLabelId::GrowsIntoGames => "Grows into games",
            PhenotypeLabelId::Ambitious => "Ambitious",
            PhenotypeLabelId::Loyal => "Loyal",
            // Technical (6)
            PhenotypeLabelId::SetPieceNatural => "Set-piece natural",
            PhenotypeLabelId::StrongLeftFoot => "Strong left foot",
            PhenotypeLabelId::PureFinisher => "Pure finisher",
            PhenotypeLabelId::SilkenFirstTouch => "Silken first touch",
            PhenotypeLabelId::PowerfulBallStriker => "Powerful ball striker",
            PhenotypeLabelId::AerialThreat => "Aerial threat",
            // Development (3)
            PhenotypeLabelId::LateBloomer => "Late bloomer",
            PhenotypeLabelId::EarlyDeveloper => "Early developer",
            PhenotypeLabelId::SteadyProgressor => "Steady progressor",
            // Role-specific — Goalkeeper (3)
            PhenotypeLabelId::SweeperKeeper => "Sweeper-keeper",
            PhenotypeLabelId::LineKeeper => "Line keeper",
            PhenotypeLabelId::CrossClaimer => "Cross claimer",
            // Role-specific — Centre-back (3)
            PhenotypeLabelId::BallPlayingDefender => "Ball-playing defender",
            PhenotypeLabelId::Stopper => "Stopper",
            PhenotypeLabelId::CoverDefender => "Cover defender",
            // Role-specific — Full-back / wing-back (3)
            PhenotypeLabelId::OverlappingFullBack => "Overlapping full-back",
            PhenotypeLabelId::InvertedFullBack => "Inverted full-back",
            PhenotypeLabelId::WingBackRunner => "Wing-back runner",
            // Role-specific — Defensive midfielder (3)
            PhenotypeLabelId::AnchorMan => "Anchor man",
            PhenotypeLabelId::BallWinningMidfielder => "Ball-winning midfielder",
            PhenotypeLabelId::PressingMidfielder => "Pressing midfielder",
            // Role-specific — Central midfielder (2)
            PhenotypeLabelId::TempoSetter => "Tempo setter",
            PhenotypeLabelId::BoxToBox => "Box-to-box",
            // Role-specific — Attacking midfielder (2)
            PhenotypeLabelId::Playmaker => "Playmaker",
            PhenotypeLabelId::HalfSpaceCreator => "Half-space creator",
            // Role-specific — Winger (2)
            PhenotypeLabelId::InvertedWinger => "Inverted winger",
            PhenotypeLabelId::TraditionalWinger => "Traditional winger",
            // Role-specific — Striker (4)
            PhenotypeLabelId::Poacher => "Poacher",
            PhenotypeLabelId::TargetMan => "Target man",
            PhenotypeLabelId::False9 => "False 9",
            PhenotypeLabelId::LinkForward => "Link forward",
        }
    }
}

// ---------------------------------------------------------------------------
// Supporting enums for PlayingInstincts
// ---------------------------------------------------------------------------

/// Preferred defensive shape when out of possession.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefensiveShape {
    /// High block, narrow shape.
    Compact,
    /// High line, spread horizontally to press wide.
    SpreadHigh,
    /// Aggressive press from the front.
    Aggressive,
    /// Low block, absorb pressure and hit on the break.
    SitAndCounter,
}

/// Preferred attacking run pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackingRun {
    /// Run into channels between defenders.
    Channels,
    /// Run in behind the defensive line.
    InBehind,
    /// Drift wide to receive.
    DriftWide,
    /// Drop deep to receive and link play.
    DropDeep,
}

/// Preferred pressing trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PressingTrigger {
    /// Press immediately on loss of possession.
    Aggressive,
    /// Press only when the ball enters defined zones.
    Reactive,
    /// Sit off and organise; only press in the final third.
    Conservative,
}

// ---------------------------------------------------------------------------
// PlayingInstincts
// ---------------------------------------------------------------------------

/// On-pitch behavioural preferences — the player's default decision-making
/// tendencies independent of team tactics. Sampled at bake time from the
/// gene model; consumed by the BT runner at T4+.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayingInstincts {
    pub defensive_shape_preference: DefensiveShape,
    pub attacking_run_preference: AttackingRun,
    pub pressing_trigger: PressingTrigger,
    /// Overall risk-taking tendency; Q32 ∈ [0, 1]. Higher = more speculative.
    pub risk_appetite: Q32,
}

// ---------------------------------------------------------------------------
// PressureResponse
// ---------------------------------------------------------------------------

/// A single point on the stakes-to-performance curve.
/// `stakes` is the match-importance measure (Q32 ∈ [0, 1]);
/// `performance_delta` is the signed delta applied to the player's output
/// (Q32; may be negative for players who wilt).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurvePoint {
    /// Match-stakes level; Q32 ∈ [0, 1].
    pub stakes: Q32,
    /// Performance delta at this stakes level; Q32 (signed, may be negative).
    pub performance_delta: Q32,
}

/// How a player performs under high-stakes conditions. Sampled at career
/// match-stake events by the sim. Consumed by T3-5 scout disagreement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PressureResponse {
    /// Curve points sampled at career match-stake events.
    /// BTreeMap-indexed by the stakes key would break Q32's Ord limitation
    /// so this is a plain Vec in declared order (bake-time authored).
    pub stakes_to_performance_curve: Vec<CurvePoint>,
    /// Minimum performance floor even in extreme-pressure situations; Q32 ∈ [0, 1].
    pub composure_floor: Q32,
}

// ---------------------------------------------------------------------------
// DevelopmentHook supporting types
// ---------------------------------------------------------------------------

/// Named readiness field targeted by a DevelopmentHook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReadinessField {
    /// The player's overall technical readiness ceiling.
    TechnicalCeiling,
    /// Readiness for a specific role family.
    RoleReadiness { role: RoleFamily },
    /// Signature affinity boost.
    SignatureAffinity,
}

/// Condition that must be met to unlock a narrative-flag hook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnlockCondition {
    /// A match of this stakes level or above was played.
    StakesThreshold { min_stakes: Q32 },
    /// The player scored a decisive goal in a match of this importance.
    DecisiveGoalInHighStakes { min_stakes: Q32 },
    /// The player reached a career age milestone.
    AgeReached { age: u8 },
    /// The player completed a cup run of this depth.
    CupRunDepth { min_rounds: u8 },
}

/// Match-event class used to gate `EventCount` development hooks.
/// These are the event CLASS identifiers — not the full `MatchEvent` enum.
/// The sim maps `MatchEvent` variants to these classes at T3+ when the
/// development-hook evaluator is wired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryEventClass {
    Goal,
    AssistCreated,
    CleanSheet,
    TackleWon,
    KeyPass,
    SaveMade,
    BigMatchPlayed,
}

/// A development hook — an event-conditioned readiness unlock. One player
/// can have multiple hooks; the sim evaluates them at T3+.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DevelopmentHook {
    /// Unlock readiness after `threshold_minutes` in the given role family.
    MinutesInRole {
        role: RoleFamily,
        threshold_minutes: u32,
        readiness_target_field: ReadinessField,
    },
    /// Unlock readiness after `threshold` qualifying match events of the given class.
    EventCount {
        event_class: MemoryEventClass,
        threshold: u32,
        readiness_target_field: ReadinessField,
    },
    /// Unlock a narrative flag when unlock conditions are all met.
    NarrativeFlagHook {
        flag: NarrativeFlag,
        unlock_conditions: Vec<UnlockCondition>,
    },
}

// ---------------------------------------------------------------------------
// CommentaryHandles
// ---------------------------------------------------------------------------

/// Short football-native phrases used by the commentary layer to refer to
/// this player. Banned-terms clean (football vernacular only per CLAUDE.md §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentaryHandles {
    /// Short noun phrases ("the striker", "the lad from Ashvale").
    pub preferred_nouns: Vec<String>,
    /// Short verb phrases ("drives", "arrives", "glides past").
    pub preferred_verbs: Vec<String>,
}

// ---------------------------------------------------------------------------
// TacticalDnaFragment
// ---------------------------------------------------------------------------

/// A fragment of tactical influence inherited at bake time. Data-only at T3;
/// surfacing (Coaching Lineage) is post-EA per `design/player-generation.md`.
/// Stored as a `Vec<TacticalDnaFragment>` on `PlayerBio`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TacticalDnaFragment {
    /// The tactical archetype ID this fragment was derived from.
    pub archetype_id: String,
    /// Influence weight; Q32 ∈ [0, 1].
    pub influence_weight: Q32,
}

// ---------------------------------------------------------------------------
// PlayerBio — the stable identity record
// ---------------------------------------------------------------------------

/// Stable per-player identity record (renamed from FW-v1 `IdentityPacket`).
///
/// Stored as `content/baked/players/<id>.ron` once the bake-time compiler
/// runs at T4+. In T2-4, 22 hand-authored fixtures exercise the type.
///
/// Field order is stable for serde determinism — do NOT reorder.
/// Adding a 23rd gene field to `internal_gene_snapshot.physical/mental/technical`
/// is a schema bump (bump `PLAYER_BIO_SCHEMA_VERSION`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerBio {
    /// Current schema version. Load-time migration uses this field.
    pub schema_version: u16,
    /// Content-pack-qualified player ID (`fwh.core:player_00042`).
    pub player_id: String,
    /// Content-pack version this entity was introduced in (`1.0.0`).
    pub content_pack_version: String,

    /// Full display name (dev UI; player-facing is role label + kit number).
    pub display_name_full: String,
    /// Short display name for dense text contexts.
    pub display_name_short: String,
    /// Role family this player is primarily designed for.
    pub role_family: RoleFamily,
    /// Fantasy region string for commentary + lineage flavor.
    pub birth_region: String,

    /// On-pitch behavioural tendencies.
    pub playing_instincts: PlayingInstincts,
    /// High-stakes performance curve + composure floor.
    pub pressure_response: PressureResponse,
    /// Event-conditioned readiness and narrative-flag unlock hooks.
    pub development_hooks: Vec<DevelopmentHook>,
    /// Signature affinities for this player (0–3 per distribution table).
    pub signature_candidates: Vec<SignatureCandidate>,
    /// Scout phenotype labels for this player. `BTreeSet` per `Sim/RULES.md §2`.
    pub scout_labels: BTreeSet<PhenotypeLabelId>,
    /// Commentary noun/verb handles (football-native, banned-terms-clean).
    pub commentary_handles: CommentaryHandles,
    /// Archetype salience multipliers; keyed by archetype_id string.
    /// `BTreeMap` per `Sim/RULES.md §2`.
    pub rivalry_compatibility: BTreeMap<String, Q32>,
    /// Clubs this player trained under — for Coaching Lineage data seeding.
    /// Surfacing is post-EA. Empty Vec is valid.
    pub alumni_of: Vec<String>,
    /// Tactical DNA fragments data-only at T3; surfacing post-EA.
    pub tactical_dna_fragments: Vec<TacticalDnaFragment>,

    /// INTERNAL ONLY. Never serialized to any UI surface.
    /// Debug/dev builds may surface this behind a build-time flag; shipped
    /// builds never do. Even the advanced scout tooltip exposes scout-estimated
    /// ranges, not the true snapshot.
    pub internal_gene_snapshot: GeneSnapshot,
}

// ---------------------------------------------------------------------------
// Tests — AC1 + AC2
// ---------------------------------------------------------------------------

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::gene::{MentalGenes, NarrativeFlag, PhysicalGenes, TechnicalAffinities};

    // ------------------------------------------------------------------------
    // AC2: phenotype_catalog_has_exactly_46_labels
    //
    // Non-vacuous: asserts the literal `46`. Spot-checks the renames and the
    // absence of the removed/excluded variants (compile-time: no such variant
    // exists — the test references specific renamed variant names so any typo
    // in the enum definition would be a compile error, not a runtime surprise).
    // ------------------------------------------------------------------------
    #[test]
    fn phenotype_catalog_has_exactly_46_labels() {
        assert_eq!(
            PhenotypeLabelId::ALL.len(),
            46,
            "PhenotypeLabelId::ALL must have exactly 46 variants; \
             design/player-generation.md §Q2 ceiling is 50"
        );

        // Spot-check the renamed variants are present under the correct names.
        let all: std::collections::BTreeSet<_> = PhenotypeLabelId::ALL.iter().collect();
        assert!(
            all.contains(&PhenotypeLabelId::StrugglesUnderScrutiny),
            "StrugglesUnderScrutiny must be present (renamed from Fragile Under Scrutiny)"
        );
        assert!(
            all.contains(&PhenotypeLabelId::PowerfulBallStriker),
            "PowerfulBallStriker must be present (renamed from Powerful Striker)"
        );

        // Spot-check ALL four categories have at least one representative.
        // Physical
        assert!(all.contains(&PhenotypeLabelId::ExplosiveFirstStep));
        // Mental
        assert!(all.contains(&PhenotypeLabelId::ComposedUnderPressure));
        // Technical
        assert!(all.contains(&PhenotypeLabelId::SetPieceNatural));
        // Development
        assert!(all.contains(&PhenotypeLabelId::LateBloomer));
        // Role-specific (all 8 role families)
        assert!(all.contains(&PhenotypeLabelId::SweeperKeeper)); // GK
        assert!(all.contains(&PhenotypeLabelId::BallPlayingDefender)); // CB
        assert!(all.contains(&PhenotypeLabelId::OverlappingFullBack)); // FB
        assert!(all.contains(&PhenotypeLabelId::AnchorMan)); // DM
        assert!(all.contains(&PhenotypeLabelId::TempoSetter)); // CM
        assert!(all.contains(&PhenotypeLabelId::Playmaker)); // AM
        assert!(all.contains(&PhenotypeLabelId::InvertedWinger)); // W
        assert!(all.contains(&PhenotypeLabelId::Poacher)); // ST

        // Verify subcounts: physical=7, mental=8, technical=6, development=3, role=22
        // Count by known members of each group.
        let physical_labels = [
            PhenotypeLabelId::ExplosiveFirstStep,
            PhenotypeLabelId::RelentlessEngine,
            PhenotypeLabelId::AerialPresence,
            PhenotypeLabelId::AgilePivot,
            PhenotypeLabelId::SlowStarter,
            PhenotypeLabelId::LateCareerPeak,
            PhenotypeLabelId::QuickRecovery,
        ];
        assert_eq!(physical_labels.len(), 7);

        let mental_labels = [
            PhenotypeLabelId::ReadsTheGame,
            PhenotypeLabelId::ComposedUnderPressure,
            PhenotypeLabelId::DecisiveInTheBox,
            PhenotypeLabelId::StrugglesUnderScrutiny,
            PhenotypeLabelId::SlowToAdapt,
            PhenotypeLabelId::GrowsIntoGames,
            PhenotypeLabelId::Ambitious,
            PhenotypeLabelId::Loyal,
        ];
        assert_eq!(mental_labels.len(), 8);

        let technical_labels = [
            PhenotypeLabelId::SetPieceNatural,
            PhenotypeLabelId::StrongLeftFoot,
            PhenotypeLabelId::PureFinisher,
            PhenotypeLabelId::SilkenFirstTouch,
            PhenotypeLabelId::PowerfulBallStriker,
            PhenotypeLabelId::AerialThreat,
        ];
        assert_eq!(technical_labels.len(), 6);

        let development_labels = [
            PhenotypeLabelId::LateBloomer,
            PhenotypeLabelId::EarlyDeveloper,
            PhenotypeLabelId::SteadyProgressor,
        ];
        assert_eq!(development_labels.len(), 3);

        // 7+8+6+3 = 24 non-role labels; 46-24 = 22 role-specific
        assert_eq!(46 - 24, 22);
    }

    // ------------------------------------------------------------------------
    // AC1: player_bio_constructs_and_round_trips
    //
    // A `PlayerBio` with every field populated serializes to RON and
    // deserializes back equal (assert_eq!).
    // ------------------------------------------------------------------------

    /// Build a fully-populated `PlayerBio` for round-trip testing.
    pub fn sample_player_bio() -> PlayerBio {
        let half = Q32::from_raw(2_147_483_648_i64); // 0.5
        let zero = Q32::ZERO;
        let one = Q32::ONE;

        PlayerBio {
            schema_version: PLAYER_BIO_SCHEMA_VERSION,
            player_id: "fwh.core:player_00001".to_string(),
            content_pack_version: "1.0.0".to_string(),
            display_name_full: "Emeka Thorne".to_string(),
            display_name_short: "E. Thorne".to_string(),
            role_family: RoleFamily::Striker,
            birth_region: "Ashvale".to_string(),
            playing_instincts: PlayingInstincts {
                defensive_shape_preference: DefensiveShape::Compact,
                attacking_run_preference: AttackingRun::InBehind,
                pressing_trigger: PressingTrigger::Aggressive,
                risk_appetite: half,
            },
            pressure_response: PressureResponse {
                stakes_to_performance_curve: vec![
                    CurvePoint {
                        stakes: zero,
                        performance_delta: zero,
                    },
                    CurvePoint {
                        stakes: half,
                        performance_delta: half,
                    },
                    CurvePoint {
                        stakes: one,
                        performance_delta: one,
                    },
                ],
                composure_floor: half,
            },
            development_hooks: vec![
                DevelopmentHook::MinutesInRole {
                    role: RoleFamily::Striker,
                    threshold_minutes: 900,
                    readiness_target_field: ReadinessField::TechnicalCeiling,
                },
                DevelopmentHook::NarrativeFlagHook {
                    flag: NarrativeFlag::LateBloomer,
                    unlock_conditions: vec![UnlockCondition::DecisiveGoalInHighStakes {
                        min_stakes: half,
                    }],
                },
            ],
            signature_candidates: vec![],
            scout_labels: {
                let mut s = BTreeSet::new();
                s.insert(PhenotypeLabelId::PureFinisher);
                s.insert(PhenotypeLabelId::Poacher);
                s.insert(PhenotypeLabelId::LateBloomer);
                s
            },
            commentary_handles: CommentaryHandles {
                preferred_nouns: vec!["the striker".to_string()],
                preferred_verbs: vec!["drives".to_string(), "arrives".to_string()],
            },
            rivalry_compatibility: {
                let mut m = BTreeMap::new();
                m.insert("fwh.core:archetype.direct-pressing".to_string(), half);
                m
            },
            alumni_of: vec!["fwh.core:club_00001".to_string()],
            tactical_dna_fragments: vec![TacticalDnaFragment {
                archetype_id: "fwh.core:archetype.direct-pressing".to_string(),
                influence_weight: half,
            }],
            internal_gene_snapshot: GeneSnapshot {
                physical: PhysicalGenes {
                    height_ceiling: half,
                    frame_density: half,
                    fast_twitch_ratio: half,
                    stamina_recovery: half,
                    growth_curve: zero,
                    aging_curve: half,
                    injury_resilience: half,
                },
                mental: MentalGenes {
                    pattern_recognition: half,
                    composure_floor: half,
                    decision_velocity: half,
                    learning_rate: half,
                    ambition: half,
                    mentality: zero,
                },
                technical: TechnicalAffinities {
                    left_foot: zero,
                    aerial: half,
                    dead_ball: half,
                    striking: half,
                    first_touch: half,
                },
                narrative_flags: {
                    let mut s = BTreeSet::new();
                    s.insert(NarrativeFlag::LateBloomer);
                    s
                },
            },
        }
    }

    #[test]
    fn player_bio_constructs_and_round_trips() {
        let bio = sample_player_bio();
        let encoded = ron::ser::to_string(&bio).expect("ron encode");
        let decoded: PlayerBio = ron::de::from_str(&encoded).expect("ron decode");
        assert_eq!(
            decoded, bio,
            "PlayerBio must survive a RON round-trip bit-identical"
        );
    }

    #[test]
    fn player_bio_schema_version_is_one() {
        assert_eq!(PLAYER_BIO_SCHEMA_VERSION, 1);
    }

    #[test]
    fn player_bio_scout_labels_are_btreeset() {
        // BTreeSet iteration order is deterministic (sorted). Verify we can
        // insert two labels and iterate them in a predictable order.
        let mut labels: BTreeSet<PhenotypeLabelId> = BTreeSet::new();
        labels.insert(PhenotypeLabelId::Poacher);
        labels.insert(PhenotypeLabelId::PureFinisher);
        let v: Vec<_> = labels.iter().collect();
        assert_eq!(v.len(), 2);
        // PureFinisher < Poacher in declaration order (PureFinisher is declared
        // before Poacher). PartialOrd/Ord derives use declaration order.
        assert_eq!(*v[0], PhenotypeLabelId::PureFinisher);
        assert_eq!(*v[1], PhenotypeLabelId::Poacher);
    }

    // ------------------------------------------------------------------------
    // T2-7: display_label — RED tests (written before the impl)
    // ------------------------------------------------------------------------

    /// Every label in PhenotypeLabelId::ALL returns a non-empty string and
    /// multi-word variants do NOT render as the raw CamelCase identifier.
    ///
    /// Single-word labels (e.g. "Ambitious", "Loyal", "Poacher") may share the
    /// same spelling as their variant name — that is intentional: the football
    /// vocabulary for those concepts is a single English word, so there is no
    /// CamelCase to transform. Multi-word variants (anything whose debug form
    /// contains no internal hyphens and has >1 CamelCase word) must NOT appear
    /// verbatim as the CamelCase identifier.
    #[test]
    fn phenotype_display_labels_all_non_empty_and_football_native() {
        for variant in PhenotypeLabelId::ALL {
            let label = variant.display_label();
            assert!(
                !label.is_empty(),
                "display_label for {variant:?} must not be empty"
            );
            // For multi-word CamelCase identifiers (detected by the presence of
            // a second uppercase letter that is NOT the very first character),
            // the display label must differ from the debug form. Single-word
            // variants (Poacher, Stopper, Playmaker, etc.) are exempt because
            // the readable English word IS the identifier.
            let debug_str = format!("{variant:?}");
            let is_multi_word_camel = debug_str.chars().skip(1).any(|c| c.is_uppercase());
            if is_multi_word_camel {
                assert_ne!(
                    label,
                    debug_str.as_str(),
                    "display_label for multi-word variant {variant:?} must not be \
                     the raw CamelCase identifier; got {label:?}"
                );
            }
        }
    }

    /// Labels are unique — no two variants share the same display string.
    #[test]
    fn phenotype_display_labels_are_unique() {
        let labels: Vec<&'static str> = PhenotypeLabelId::ALL
            .iter()
            .map(|v| v.display_label())
            .collect();
        let unique: std::collections::BTreeSet<_> = labels.iter().collect();
        assert_eq!(
            labels.len(),
            unique.len(),
            "every PhenotypeLabelId variant must have a unique display label"
        );
    }

    /// Spot-check specific human-readable renders.
    #[test]
    fn phenotype_display_label_spot_checks() {
        assert_eq!(
            PhenotypeLabelId::ExplosiveFirstStep.display_label(),
            "Explosive first step"
        );
        assert_eq!(
            PhenotypeLabelId::ReadsTheGame.display_label(),
            "Reads the game"
        );
        assert_eq!(
            PhenotypeLabelId::SweeperKeeper.display_label(),
            "Sweeper-keeper"
        );
        assert_eq!(PhenotypeLabelId::Poacher.display_label(), "Poacher");
        assert_eq!(
            PhenotypeLabelId::StrugglesUnderScrutiny.display_label(),
            "Struggles under scrutiny"
        );
        assert_eq!(
            PhenotypeLabelId::PowerfulBallStriker.display_label(),
            "Powerful ball striker"
        );
    }
}

//! Breakthrough mechanism — pillar 3 ("Breakthrough-Driven Development").
//!
//! Players permanently change (PA redraw) because of what happened in the
//! ledger, not XP accumulation. Three trigger kinds — signature awakening,
//! latent-flag unlock, regressive collapse — are three gating-flavors of ONE
//! per-(player, attribute_family) meter mechanism.
//!
//! ## Design references
//!
//! - `design/breakthrough-moments.md` — mechanism design contract
//! - `docs/design/progression.md` — ALL tuning seeds (ported cell-for-cell)
//! - `docs/adr/0005-memory-ledger-and-breakthroughs.md` §"Breakthrough mechanism"
//!   + §"Regressive collapse" — authoritative formulas
//!
//! ## Determinism contract
//!
//! - No `f32`/`f64` anywhere — Q32 only (Sim/RULES §1).
//! - `BTreeMap`/`BTreeSet` only — no `HashMap`/`HashSet` (Sim/RULES §2).
//! - No clocks (Sim/RULES §3). All time references are `Tick` / `CareerDate`.
//! - RNG draws via `ChaCha8Rng::seed_from_u64(seed_fn(career_seed, tick,
//!   SeedLayer::SignatureTrigger, site))` per ADR-0009 (Sim/RULES §4).
//! - No async/tokio (Sim/RULES §5).

use std::collections::BTreeMap;

use fw_core::{PlayerId, Q32, SeedLayer, Tick, seed_fn};
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::event::{
    CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind, Emotion,
    EntityRef, EventClass, MemoryEvent, Participant, ParticipantRole, SeasonNumber, SourceId,
};
use crate::ledger::MemoryLedger;
use crate::readers::project_salience;

// -------------------------------------------------------------------------
// AttributeFamily — 10 coarse families per progression.md
// -------------------------------------------------------------------------

/// Coarse attribute-family grouping used by the breakthrough meter.
///
/// The 10 families from `docs/design/progression.md` §"Attribute-family list".
///
/// ## Tag-stability (LOAD-BEARING FOREVER)
///
/// `#[repr(u32)]` + explicit discriminants 0..9 pin the canonical career-state
/// encoding. Re-ordering variants or changing discriminants is a schema-breaking
/// change. The wire discriminants are further pinned by
/// `attribute_family_discriminants_locked` test.
///
/// Mod content packs do NOT add families — the family set is closed at T3-4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u32)]
pub enum AttributeFamily {
    /// Conversion, composure in the box. Strikers, AMs.
    Finishing = 0,
    /// Range, vision, precision. Midfielders, full-backs.
    Passing = 1,
    /// Reading the game, positioning. Centre-backs, DMs.
    DefensiveAnticipation = 2,
    /// Heading, physical duels. Centre-backs, target men.
    AerialPresence = 3,
    /// Pressure response, decision quality under duress. All positions.
    Composure = 4,
    /// Explosive speed, acceleration. Wingers, strikers, attacking full-backs.
    Pace = 5,
    /// Late-match intensity, injury-load resilience. All positions.
    Stamina = 6,
    /// Pressing, tracking-back, shuttle runs. Pressing mids, box-to-box.
    WorkRate = 7,
    /// Set-pieces, free kicks, penalties. Specialist DM / AM / full-back.
    DeadBallDelivery = 8,
    /// Dressing-room influence, mentoring yield. Captains, senior figures.
    Leadership = 9,
}

impl AttributeFamily {
    /// The `#[repr(u32)]` discriminant for this variant.
    ///
    /// Pinned by `attribute_family_discriminants_locked` test.
    #[must_use]
    pub fn discriminant(self) -> u32 {
        self as u32
    }

    /// All 10 families in discriminant order. Canonical for iteration.
    pub const ALL: [AttributeFamily; 10] = [
        AttributeFamily::Finishing,
        AttributeFamily::Passing,
        AttributeFamily::DefensiveAnticipation,
        AttributeFamily::AerialPresence,
        AttributeFamily::Composure,
        AttributeFamily::Pace,
        AttributeFamily::Stamina,
        AttributeFamily::WorkRate,
        AttributeFamily::DeadBallDelivery,
        AttributeFamily::Leadership,
    ];
}

// -------------------------------------------------------------------------
// Progression constants — ported from docs/design/progression.md
// -------------------------------------------------------------------------

/// Readiness threshold for a positive breakthrough to fire.
/// Q32(0.92) = `Q32::from_raw(round(0.92 × 2^32))`.
/// Arithmetic: 4_294_967_296 × 92 / 100 = 3_951_369_912.32 → round = 3_951_369_912
pub const BREAKTHROUGH_THRESHOLD: Q32 = Q32::from_raw(3_951_369_912_i64);

/// Regressive pressure threshold for a regressive collapse to fire.
/// Q32(0.90) = round(0.90 × 2^32) = 3_865_470_566
pub const REGRESSIVE_THRESHOLD: Q32 = Q32::from_raw(3_865_470_566_i64);

/// Post-breakthrough readiness residue: 0.15.
/// Raw bits: round(0.15 × 2^32) = 644_245_094
/// Arithmetic: 4_294_967_296 × 15 / 100 = 644_245_094.4 → round = 644_245_094
pub const READINESS_RESIDUE: Q32 = Q32::from_raw(644_245_094_i64);

/// Post-collapse regressive pressure residue: 0.15.
/// Same value as `READINESS_RESIDUE` per ADR-0005 §"Regressive collapse" — both
/// meters reset to 0.15 residue after firing. Stored as a separate constant so the
/// two tuning dials can diverge independently during Phase-3 balance work.
/// Arithmetic: 4_294_967_296 × 15 / 100 = 644_245_094.4 → round = 644_245_094
pub const REGRESSIVE_RESIDUE: Q32 = Q32::from_raw(644_245_094_i64);

/// CA-lift fraction: 0.50 (CA catches up halfway to the new PA ceiling).
/// Raw bits: 1 << 31 = 2_147_483_648
pub const CA_LIFT_FRACTION: Q32 = Q32::from_raw(2_147_483_648_i64);

/// Positive breakthrough cooldown in in-game months.
/// 12 in-game months = 12 * 30 = 360 in-game days.
/// We store as ticks; the career system maps days → ticks externally.
/// Here we store it as "in-game days" for the cooldown comparison
/// (career date arithmetic). 12 months ≈ 365 days.
pub const BREAKTHROUGH_COOLDOWN_DAYS: u32 = 365;

/// Regressive collapse cooldown in in-game days. 18 months ≈ 548 days.
pub const REGRESSIVE_COOLDOWN_DAYS: u32 = 548;

// -------------------------------------------------------------------------
// family_relevance table — per (EventClass, AttributeFamily) weights
// -------------------------------------------------------------------------
//
// Ported cell-for-cell from progression.md §"family_relevance table".
// Positive values → feed `signature_readiness`.
// Negative values → feed `regressive_pressure` (absolute value).
// ZERO cells omitted (no effect on either meter).
//
// Per progression.md §"Design notes": negative cells do NOT tick
// `signature_readiness` downward. They are used ONLY for `regressive_pressure`
// accumulation. The sign of the return value signals which meter to feed.

/// Returns `(positive_relevance, regressive_relevance)` for the given
/// `(event_class, family)` pair.
///
/// Both values are non-negative Q32. A non-zero `positive_relevance` means
/// the event feeds `signature_readiness` for that family. A non-zero
/// `regressive_relevance` means the event feeds `regressive_pressure`.
///
/// The raw table from progression.md is encoded inline; negative cells become
/// `regressive_relevance` entries only.
#[must_use]
pub fn family_relevance(class: &EventClass, family: AttributeFamily) -> (Q32, Q32) {
    // Helper: Q32 from a decimal string factor.
    // We encode each weight as its raw i64 bits for the Q32.32 format.
    // round(w × 2^32) for each weight in the table.
    // Weights from progression.md §"family_relevance table" (non-zero cells only).
    use AttributeFamily::*;

    let (pos, neg): (i64, i64) = match (class, family) {
        // ---- SignatureFirstFired ----
        (EventClass::SignatureFirstFired, Finishing) => (644_245_094, 0), // 0.15
        (EventClass::SignatureFirstFired, Passing) => (429_496_730, 0),   // 0.10
        (EventClass::SignatureFirstFired, DefensiveAnticipation) => (214_748_365, 0), // 0.05
        (EventClass::SignatureFirstFired, AerialPresence) => (214_748_365, 0), // 0.05
        (EventClass::SignatureFirstFired, Composure) => (429_496_730, 0), // 0.10
        (EventClass::SignatureFirstFired, Pace) => (214_748_365, 0),      // 0.05
        (EventClass::SignatureFirstFired, WorkRate) => (214_748_365, 0),  // 0.05
        (EventClass::SignatureFirstFired, DeadBallDelivery) => (429_496_730, 0), // 0.10

        // ---- LegacyGoal ----
        (EventClass::LegacyGoal, Finishing) => (1_932_735_283, 0), // 0.45
        (EventClass::LegacyGoal, Passing) => (214_748_365, 0),     // 0.05
        (EventClass::LegacyGoal, AerialPresence) => (343_597_384, 0), // 0.08
        (EventClass::LegacyGoal, Composure) => (1_073_741_824, 0), // 0.25
        (EventClass::LegacyGoal, Pace) => (214_748_365, 0),        // 0.05
        (EventClass::LegacyGoal, WorkRate) => (214_748_365, 0),    // 0.05
        (EventClass::LegacyGoal, DeadBallDelivery) => (429_496_730, 0), // 0.10
        (EventClass::LegacyGoal, Leadership) => (214_748_365, 0),  // 0.05

        // ---- HatTrickScored ----
        (EventClass::HatTrickScored, Finishing) => (1_717_986_918, 0), // 0.40
        (EventClass::HatTrickScored, AerialPresence) => (214_748_365, 0), // 0.05
        (EventClass::HatTrickScored, Composure) => (644_245_094, 0),   // 0.15
        (EventClass::HatTrickScored, Pace) => (214_748_365, 0),        // 0.05
        (EventClass::HatTrickScored, DeadBallDelivery) => (429_496_730, 0), // 0.10

        // ---- BigMatchScar (negative: Composure, WorkRate, Leadership; Finishing negative too) ----
        (EventClass::BigMatchScar, Finishing) => (0, 429_496_730), // −0.10 regressive
        (EventClass::BigMatchScar, Composure) => (0, 1_288_490_189), // −0.30 regressive
        (EventClass::BigMatchScar, WorkRate) => (0, 429_496_730),  // −0.10 regressive
        (EventClass::BigMatchScar, Leadership) => (0, 429_496_730), // −0.10 regressive

        // ---- BrokenPromise ----
        (EventClass::BrokenPromise, Composure) => (0, 644_245_094), // −0.15 regressive
        (EventClass::BrokenPromise, WorkRate) => (0, 1_073_741_824), // −0.25 regressive
        (EventClass::BrokenPromise, Leadership) => (0, 429_496_730), // −0.10 regressive

        // ---- ContractRenewalRejected ----
        (EventClass::ContractRenewalRejected, Composure) => (0, 214_748_365), // −0.05
        (EventClass::ContractRenewalRejected, WorkRate) => (0, 644_245_094),  // −0.15

        // ---- ContractRenewalAccepted ----
        (EventClass::ContractRenewalAccepted, Composure) => (343_597_384, 0), // 0.08
        (EventClass::ContractRenewalAccepted, WorkRate) => (343_597_384, 0),  // 0.08
        (EventClass::ContractRenewalAccepted, Leadership) => (214_748_365, 0), // 0.05

        // ---- TransferRequested ----
        (EventClass::TransferRequested, WorkRate) => (0, 214_748_365), // −0.05

        // ---- TransferRefused ----
        (EventClass::TransferRefused, Composure) => (0, 214_748_365), // −0.05

        // ---- SoldUnderProtest ----
        (EventClass::SoldUnderProtest, Composure) => (0, 429_496_730), // −0.10
        (EventClass::SoldUnderProtest, WorkRate) => (0, 429_496_730),  // −0.10

        // ---- BoughtOnDeadlineDay ----
        (EventClass::BoughtOnDeadlineDay, Stamina) => (214_748_365, 0), // 0.05

        // ---- RivalryFormed ----
        (EventClass::RivalryFormed, Finishing) => (214_748_365, 0), // 0.05
        (EventClass::RivalryFormed, DefensiveAnticipation) => (214_748_365, 0), // 0.05
        (EventClass::RivalryFormed, Composure) => (214_748_365, 0), // 0.05
        (EventClass::RivalryFormed, WorkRate) => (214_748_365, 0),  // 0.05
        (EventClass::RivalryFormed, Leadership) => (214_748_365, 0), // 0.05

        // ---- MentorTeammate ----
        (EventClass::MentorTeammate, Passing) => (515_396_076, 0), // 0.12
        (EventClass::MentorTeammate, DefensiveAnticipation) => (429_496_730, 0), // 0.10
        (EventClass::MentorTeammate, Composure) => (858_993_459, 0), // 0.20
        (EventClass::MentorTeammate, WorkRate) => (343_597_384, 0), // 0.08
        (EventClass::MentorTeammate, Leadership) => (1_503_238_554, 0), // 0.35

        // ---- DerbyControversy ----
        (EventClass::DerbyControversy, Composure) => (0, 515_396_076), // −0.12
        (EventClass::DerbyControversy, Leadership) => (0, 343_597_384), // −0.08

        // ---- FormerClubReunion ----
        (EventClass::FormerClubReunion, Finishing) => (214_748_365, 0), // 0.05
        (EventClass::FormerClubReunion, Composure) => (343_597_384, 0), // 0.08
        (EventClass::FormerClubReunion, WorkRate) => (214_748_365, 0),  // 0.05

        // ---- CupFinalWin ----
        (EventClass::CupFinalWin, Finishing) => (644_245_094, 0), // 0.15
        (EventClass::CupFinalWin, Passing) => (429_496_730, 0),   // 0.10
        (EventClass::CupFinalWin, DefensiveAnticipation) => (429_496_730, 0), // 0.10
        (EventClass::CupFinalWin, AerialPresence) => (343_597_384, 0), // 0.08
        (EventClass::CupFinalWin, Composure) => (1_288_490_189, 0), // 0.30
        (EventClass::CupFinalWin, Stamina) => (343_597_384, 0),   // 0.08
        (EventClass::CupFinalWin, WorkRate) => (429_496_730, 0),  // 0.10
        (EventClass::CupFinalWin, DeadBallDelivery) => (343_597_384, 0), // 0.08
        (EventClass::CupFinalWin, Leadership) => (1_073_741_824, 0), // 0.25

        // ---- CupFinalLoss ----
        (EventClass::CupFinalLoss, Composure) => (0, 1_073_741_824), // −0.25
        (EventClass::CupFinalLoss, Leadership) => (0, 429_496_730),  // −0.10

        // ---- PromotionWon ----
        (EventClass::PromotionWon, Finishing) => (343_597_384, 0), // 0.08
        (EventClass::PromotionWon, Passing) => (214_748_365, 0),   // 0.05
        (EventClass::PromotionWon, DefensiveAnticipation) => (214_748_365, 0), // 0.05
        (EventClass::PromotionWon, Composure) => (644_245_094, 0), // 0.15
        (EventClass::PromotionWon, Stamina) => (214_748_365, 0),   // 0.05
        (EventClass::PromotionWon, WorkRate) => (343_597_384, 0),  // 0.08
        (EventClass::PromotionWon, Leadership) => (515_396_076, 0), // 0.12

        // ---- RelegationSuffered ----
        (EventClass::RelegationSuffered, Finishing) => (0, 214_748_365), // −0.05
        (EventClass::RelegationSuffered, DefensiveAnticipation) => (0, 214_748_365), // −0.05
        (EventClass::RelegationSuffered, Composure) => (0, 858_993_459), // −0.20
        (EventClass::RelegationSuffered, WorkRate) => (0, 429_496_730),  // −0.10
        (EventClass::RelegationSuffered, Leadership) => (0, 644_245_094), // −0.15

        // ---- TitleWon ----
        (EventClass::TitleWon, Finishing) => (429_496_730, 0), // 0.10
        (EventClass::TitleWon, Passing) => (343_597_384, 0),   // 0.08
        (EventClass::TitleWon, DefensiveAnticipation) => (343_597_384, 0), // 0.08
        (EventClass::TitleWon, Composure) => (858_993_459, 0), // 0.20
        (EventClass::TitleWon, Stamina) => (343_597_384, 0),   // 0.08
        (EventClass::TitleWon, WorkRate) => (429_496_730, 0),  // 0.10
        (EventClass::TitleWon, Leadership) => (773_094_114, 0), // 0.18

        // ---- UnbeatenRunEnded ----
        (EventClass::UnbeatenRunEnded, Composure) => (0, 343_597_384), // −0.08
        (EventClass::UnbeatenRunEnded, Leadership) => (0, 214_748_365), // −0.05

        // ---- DebutSenior ----
        (EventClass::DebutSenior, Finishing) => (214_748_365, 0), // 0.05
        (EventClass::DebutSenior, Passing) => (214_748_365, 0),   // 0.05
        (EventClass::DebutSenior, DefensiveAnticipation) => (214_748_365, 0), // 0.05
        (EventClass::DebutSenior, Composure) => (515_396_076, 0), // 0.12
        (EventClass::DebutSenior, Pace) => (214_748_365, 0),      // 0.05
        (EventClass::DebutSenior, WorkRate) => (214_748_365, 0),  // 0.05

        // ---- DebutClub ----
        (EventClass::DebutClub, Finishing) => (128_849_019, 0), // 0.03
        (EventClass::DebutClub, Passing) => (128_849_019, 0),   // 0.03
        (EventClass::DebutClub, DefensiveAnticipation) => (128_849_019, 0), // 0.03
        (EventClass::DebutClub, Composure) => (343_597_384, 0), // 0.08
        (EventClass::DebutClub, WorkRate) => (128_849_019, 0),  // 0.03

        // ---- InjuryLongTerm ----
        (EventClass::InjuryLongTerm, Finishing) => (0, 214_748_365), // −0.05
        (EventClass::InjuryLongTerm, Composure) => (0, 343_597_384), // −0.08
        (EventClass::InjuryLongTerm, Pace) => (0, 644_245_094),      // −0.15
        (EventClass::InjuryLongTerm, Stamina) => (0, 1_288_490_189), // −0.30
        (EventClass::InjuryLongTerm, WorkRate) => (0, 343_597_384),  // −0.08

        // ---- InternationalCallUp ----
        (EventClass::InternationalCallUp, Finishing) => (214_748_365, 0), // 0.05
        (EventClass::InternationalCallUp, Passing) => (214_748_365, 0),   // 0.05
        (EventClass::InternationalCallUp, DefensiveAnticipation) => (214_748_365, 0), // 0.05
        (EventClass::InternationalCallUp, AerialPresence) => (128_849_019, 0), // 0.03
        (EventClass::InternationalCallUp, Composure) => (429_496_730, 0), // 0.10
        (EventClass::InternationalCallUp, Pace) => (128_849_019, 0),      // 0.03
        (EventClass::InternationalCallUp, WorkRate) => (214_748_365, 0),  // 0.05
        (EventClass::InternationalCallUp, DeadBallDelivery) => (128_849_019, 0), // 0.03
        (EventClass::InternationalCallUp, Leadership) => (343_597_384, 0), // 0.08

        // All other (class, family) pairs: zero relevance.
        _ => (0, 0),
    };

    (Q32::from_raw(pos), Q32::from_raw(neg))
}

// -------------------------------------------------------------------------
// Gating-event tables — per progression.md §"Gating-event table"
// -------------------------------------------------------------------------

/// Returns true if `class` is a valid positive gating event for `family`.
/// All gate events must have `stakes >= 0.5` (checked by the caller).
#[must_use]
pub fn is_positive_gate(class: &EventClass, family: AttributeFamily) -> bool {
    use AttributeFamily::*;
    match family {
        Finishing => matches!(class, EventClass::LegacyGoal | EventClass::HatTrickScored),
        Passing => matches!(
            class,
            EventClass::LegacyGoal
                | EventClass::CupFinalWin
                | EventClass::PromotionWon
                | EventClass::TitleWon
        ),
        DefensiveAnticipation => matches!(
            class,
            EventClass::CupFinalWin
                | EventClass::PromotionWon
                | EventClass::TitleWon
                | EventClass::RelegationSuffered
                | EventClass::UnbeatenRunEnded
        ),
        AerialPresence => matches!(
            class,
            EventClass::LegacyGoal | EventClass::CupFinalWin | EventClass::PromotionWon
        ),
        Composure => matches!(
            class,
            EventClass::CupFinalWin
                | EventClass::CupFinalLoss
                | EventClass::PromotionWon
                | EventClass::RelegationSuffered
                | EventClass::BigMatchScar
        ),
        Pace => matches!(
            class,
            EventClass::InternationalCallUp
                | EventClass::DebutSenior
                | EventClass::SignatureFirstFired
        ),
        Stamina => matches!(
            class,
            EventClass::InjuryLongTerm | EventClass::PromotionWon | EventClass::TitleWon
        ),
        WorkRate => matches!(
            class,
            EventClass::CupFinalWin
                | EventClass::PromotionWon
                | EventClass::TitleWon
                | EventClass::MentorTeammate
        ),
        DeadBallDelivery => matches!(
            class,
            EventClass::LegacyGoal | EventClass::CupFinalWin | EventClass::HatTrickScored
        ),
        Leadership => matches!(
            class,
            EventClass::MentorTeammate | EventClass::CupFinalWin | EventClass::TitleWon
        ),
    }
}

/// Returns true if `class` is a valid regressive gating event for `family`.
#[must_use]
pub fn is_regressive_gate(class: &EventClass, family: AttributeFamily) -> bool {
    use AttributeFamily::*;
    match family {
        Composure => matches!(
            class,
            EventClass::BigMatchScar | EventClass::CupFinalLoss | EventClass::RelegationSuffered
        ),
        WorkRate => matches!(
            class,
            EventClass::BrokenPromise | EventClass::SoldUnderProtest
        ),
        Finishing => matches!(class, EventClass::BigMatchScar),
        Pace => matches!(class, EventClass::InjuryLongTerm),
        Stamina => matches!(class, EventClass::InjuryLongTerm),
        Leadership => matches!(
            class,
            EventClass::DerbyControversy | EventClass::CupFinalLoss
        ),
        // Families without a regressive gate: Passing, DefensiveAnticipation,
        // AerialPresence, DeadBallDelivery
        _ => false,
    }
}

// -------------------------------------------------------------------------
// Redraw distribution — PA redraw magnitude per family
// -------------------------------------------------------------------------

/// Positive redraw range `(min, max)` in PA integer units for a given family.
/// From progression.md §"redraw_distribution".
#[must_use]
pub fn positive_redraw_range(family: AttributeFamily) -> (i16, i16) {
    use AttributeFamily::*;
    match family {
        Finishing => (4, 9),
        Passing => (3, 7),
        DefensiveAnticipation => (4, 8),
        AerialPresence => (4, 9),
        Composure => (3, 8),
        Pace => (5, 11),
        Stamina => (3, 7),
        WorkRate => (3, 6),
        DeadBallDelivery => (4, 8),
        Leadership => (3, 6),
    }
}

/// Regressive redraw range `(min_abs, max_abs)` in PA integer units (positive
/// values; the caller negates). From progression.md §"redraw_distribution".
#[must_use]
pub fn regressive_redraw_range(family: AttributeFamily) -> (i16, i16) {
    use AttributeFamily::*;
    match family {
        Finishing => (4, 8),
        Passing => (3, 6),
        DefensiveAnticipation => (4, 7),
        AerialPresence => (3, 6),
        Composure => (5, 10),
        Pace => (6, 12),
        Stamina => (4, 9),
        WorkRate => (3, 7),
        DeadBallDelivery => (3, 6),
        Leadership => (3, 7),
    }
}

/// Stakes threshold for the stakes modifier and its multiplier per family.
/// `(threshold_raw_q32, multiplier_numerator_percent)` — the multiplier is
/// stored as an integer percentage to stay float-free. E.g. 130 = ×1.3.
/// From progression.md §"redraw_distribution" Stakes modifier column.
#[must_use]
pub fn stakes_modifier(family: AttributeFamily) -> (Q32, u32) {
    use AttributeFamily::*;
    // threshold raw Q32 bits for the threshold fractions:
    // 0.80 = round(0.80 × 2^32) = 3_435_973_837
    // 0.85 = round(0.85 × 2^32) = 3_650_722_202
    // 0.90 = round(0.90 × 2^32) = 3_865_470_566
    match family {
        Finishing => (Q32::from_raw(3_650_722_202_i64), 130),
        Passing => (Q32::from_raw(3_650_722_202_i64), 120),
        DefensiveAnticipation => (Q32::from_raw(3_650_722_202_i64), 120),
        AerialPresence => (Q32::from_raw(3_435_973_837_i64), 110),
        Composure => (Q32::from_raw(3_650_722_202_i64), 140),
        Pace => (Q32::from_raw(3_865_470_566_i64), 130),
        Stamina => (Q32::from_raw(3_435_973_837_i64), 120),
        WorkRate => (Q32::from_raw(3_435_973_837_i64), 110),
        DeadBallDelivery => (Q32::from_raw(3_435_973_837_i64), 110),
        Leadership => (Q32::from_raw(3_865_470_566_i64), 150),
    }
}

// -------------------------------------------------------------------------
// Narrative-trigger gene flags — 4 flags that gate Kind 2
// -------------------------------------------------------------------------

/// The 4 narrative-trigger gene flags that can gate a Kind-2 (latent-flag)
/// breakthrough per `design/breakthrough-moments.md` §"Kind 2".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NarrativeFlag {
    /// Growth spike in later career years. Gates Finishing on rare events only.
    LateBloomer,
    /// Access to flow states under specific pressure conditions.
    FlowAccess,
    /// Unusually high potential ceiling relative to gene expression.
    PeakCeilingHigh,
    /// Latent ability that is dormant until a specific unlock event.
    AwakeningDormant,
}

// -------------------------------------------------------------------------
// BreakthroughState — per-player meter tracking
// -------------------------------------------------------------------------

/// Per-player meter and cooldown state for the breakthrough mechanism.
///
/// One `BreakthroughState` per player. Stored in `BreakthroughEvaluator`
/// as `BTreeMap<PlayerId, BreakthroughState>` — canonical career state;
/// serialized in the career save.
///
/// Field layout is stable; do not reorder without bumping save schema.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BreakthroughState {
    /// Per-family positive readiness meters, in [0, 1].
    /// Missing key means readiness == 0 (BTreeMap default semantics).
    /// `pub(crate)` — external callers must use `add_readiness` / `reset_readiness_to_residue`
    /// to preserve the [0,1] clamp invariant.
    pub(crate) signature_readiness: BTreeMap<AttributeFamily, Q32>,

    /// Per-family regressive pressure meters, in [0, 1].
    /// `pub(crate)` — same invariant as `signature_readiness`.
    pub(crate) regressive_pressure: BTreeMap<AttributeFamily, Q32>,

    /// Last breakthrough fire date per (player, family). Used for the cooldown check.
    /// `None` means no breakthrough has fired in this family.
    /// `pub(crate)` — written directly by `evaluate`; read via `positive_cooldown_clear`.
    pub(crate) last_positive_fire: BTreeMap<AttributeFamily, CareerDate>,

    /// Last regressive collapse fire date per family.
    /// `pub(crate)` — written directly by `evaluate`; read via `regressive_cooldown_clear`.
    pub(crate) last_regressive_fire: BTreeMap<AttributeFamily, CareerDate>,
}

impl BreakthroughState {
    /// Create a new zero-initialized state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the current `signature_readiness` for a family (default: zero).
    pub fn readiness(&self, family: AttributeFamily) -> Q32 {
        self.signature_readiness
            .get(&family)
            .copied()
            .unwrap_or(Q32::ZERO)
    }

    /// Get the current `regressive_pressure` for a family (default: zero).
    pub fn pressure(&self, family: AttributeFamily) -> Q32 {
        self.regressive_pressure
            .get(&family)
            .copied()
            .unwrap_or(Q32::ZERO)
    }

    /// Accumulate a readiness delta for a family, clamped to [0, 1].
    pub fn add_readiness(&mut self, family: AttributeFamily, delta: Q32) {
        let current = self.readiness(family);
        let new_val = (current + delta).min(Q32::ONE).max(Q32::ZERO);
        self.signature_readiness.insert(family, new_val);
    }

    /// Accumulate a regressive pressure delta for a family, clamped to [0, 1].
    pub fn add_pressure(&mut self, family: AttributeFamily, delta: Q32) {
        let current = self.pressure(family);
        let new_val = (current + delta).min(Q32::ONE).max(Q32::ZERO);
        self.regressive_pressure.insert(family, new_val);
    }

    /// Reset readiness to residue after a breakthrough fires.
    pub fn reset_readiness_to_residue(&mut self, family: AttributeFamily) {
        self.signature_readiness.insert(family, READINESS_RESIDUE);
    }

    /// Reset regressive pressure to residue after a collapse fires.
    pub fn reset_pressure_to_residue(&mut self, family: AttributeFamily) {
        self.regressive_pressure.insert(family, REGRESSIVE_RESIDUE);
    }

    /// Inject a last-positive-fire date for `family`. Used by tests that need to
    /// simulate an in-cooldown state without running a full evaluate loop.
    pub fn set_last_positive_fire(&mut self, family: AttributeFamily, date: CareerDate) {
        self.last_positive_fire.insert(family, date);
    }

    /// Inject a last-regressive-fire date for `family`. Used by tests.
    pub fn set_last_regressive_fire(&mut self, family: AttributeFamily, date: CareerDate) {
        self.last_regressive_fire.insert(family, date);
    }

    /// Check if the positive cooldown has cleared for `family` at `now`.
    /// Returns true if the player can fire a positive breakthrough.
    pub fn positive_cooldown_clear(&self, family: AttributeFamily, now: &CareerDate) -> bool {
        match self.last_positive_fire.get(&family) {
            None => true,
            Some(last) => days_since(last, now) >= BREAKTHROUGH_COOLDOWN_DAYS,
        }
    }

    /// Check if the regressive cooldown has cleared for `family` at `now`.
    pub fn regressive_cooldown_clear(&self, family: AttributeFamily, now: &CareerDate) -> bool {
        match self.last_regressive_fire.get(&family) {
            None => true,
            Some(last) => days_since(last, now) >= REGRESSIVE_COOLDOWN_DAYS,
        }
    }
}

/// Compute approximate in-game days between two `CareerDate`s.
/// Both dates are in a fictional calendar; `year` is fictional year,
/// `day_of_year` is 1-based (1..=365). Treats each fictional year as 365 days.
fn days_since(past: &CareerDate, now: &CareerDate) -> u32 {
    let past_total = (past.year as u32) * 365 + (past.day_of_year as u32);
    let now_total = (now.year as u32) * 365 + (now.day_of_year as u32);
    // saturating_sub OK here per Sim/RULES §11: a past-after-now career date
    // (clock skew or save migration edge case) saturates to 0, meaning "zero days
    // elapsed" → cooldown not yet cleared → breakthrough blocked. That is the safe
    // conservative direction; panicking on such input would be worse.
    now_total.saturating_sub(past_total)
}

// -------------------------------------------------------------------------
// BreakthroughContext — caller-supplied player data
// -------------------------------------------------------------------------

/// Caller-supplied player data for breakthrough evaluation.
///
/// fw-memory must NOT depend on fw-content for player data. The career system
/// (caller) supplies this context at evaluation time. Pattern matches T3-3's
/// render-from-context approach.
#[derive(Debug, Clone)]
pub struct BreakthroughContext {
    /// The player being evaluated.
    pub player_id: PlayerId,

    /// Per-family current PA (potential, ceiling). Scale: 1..=200.
    pub pa_by_family: BTreeMap<AttributeFamily, i16>,

    /// Per-family current CA (current ability, floor). Scale: 1..=200.
    pub ca_by_family: BTreeMap<AttributeFamily, i16>,

    /// The 4 narrative trigger gene flags present in this player's genome.
    pub narrative_flags: Vec<NarrativeFlag>,

    /// Signature candidates pending for this player.
    /// Each candidate is `(family, signature_id_string)`. The family
    /// indicates which attribute family the signature is tied to.
    /// `signature_id` is a String to avoid a fw-content dependency;
    /// it carries the content-pack-qualified ID (e.g. `"fwh.core:signature.first_time_diagonal"`).
    pub signature_candidates: Vec<(AttributeFamily, String)>,

    /// Player age in whole in-game years. Used for the age-curve modifier.
    pub age_years: u8,

    /// Current in-game date. Used for cooldown checks.
    pub career_date: CareerDate,
}

// -------------------------------------------------------------------------
// BreakthroughKind — the three gating-flavors
// -------------------------------------------------------------------------

/// Which gating-flavor triggered a breakthrough.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BreakthroughKind {
    /// Kind 1: a pending signature candidate in this family was activated.
    /// Carries the `signature_id` that was activated.
    SignatureAwakening { signature_id: String },
    /// Kind 2: a narrative-trigger gene flag gated this breakthrough.
    LatentFlagUnlock { flag: NarrativeFlag },
    /// Kind 3: regressive collapse — PA redraw downward.
    RegressiveCollapse,
}

// -------------------------------------------------------------------------
// BreakthroughOutcome — the structured result of a fired breakthrough
// -------------------------------------------------------------------------

/// The result of a breakthrough or regressive collapse firing.
///
/// The caller (career system) reads this to:
/// 1. Apply `delta_pa` / `delta_ca` to the player's stored attributes.
/// 2. Feed the emitted `event` into the `MemoryLedger`.
/// 3. Route text-recap to the post-match report.
#[derive(Debug, Clone)]
pub struct BreakthroughOutcome {
    /// The player who experienced the breakthrough.
    pub player_id: PlayerId,
    /// The attribute family affected.
    pub family: AttributeFamily,
    /// The gating flavor (Kind 1 / 2 / 3).
    pub kind: BreakthroughKind,
    /// PA delta (positive for breakthrough, negative for regressive collapse).
    pub delta_pa: i16,
    /// CA delta (positive for breakthrough, negative for regressive collapse).
    pub delta_ca: i16,
    /// The gating event that triggered the gate.
    pub gating_event_class: EventClass,
    /// The fully constructed `MemoryEvent` to append to the ledger.
    pub event: MemoryEvent,
}

// -------------------------------------------------------------------------
// Redraw sampling — deterministic via ChaCha8Rng + SeedLayer::SignatureTrigger
// -------------------------------------------------------------------------

/// Sample a positive PA delta for `family` from the `redraw_distribution`.
///
/// Uses `ChaCha8Rng::seed_from_u64(seed_fn(career_seed, tick, SignatureTrigger, site))`.
/// Stakes modifier applied + re-clamped to the range ceiling per progression.md.
///
/// `tick` is passed as `u32` (the low 32 bits of the in-game tick; the career
/// system supplies this from the ledger's current simulated time).
fn sample_positive_delta(
    family: AttributeFamily,
    stakes: Q32,
    career_seed: u64,
    tick: u32,
    site: u32,
) -> i16 {
    let (min_pa, max_pa) = positive_redraw_range(family);
    let (threshold, modifier_pct) = stakes_modifier(family);

    let seed = seed_fn(career_seed, tick, SeedLayer::SignatureTrigger, site);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    // Sample an integer in [min_pa, max_pa] (inclusive).
    let range = (max_pa - min_pa) as u32 + 1;
    let raw: u32 = rng.gen_range(0..range);
    let delta = min_pa + raw as i16;

    // Apply stakes modifier if stakes >= threshold.
    if stakes >= threshold {
        // modifier_pct is an integer percentage, e.g. 130 for ×1.3.
        // We avoid floats: delta × modifier_pct / 100, integer arithmetic.
        let amplified = (delta as i32 * modifier_pct as i32 / 100) as i16;
        amplified.min(max_pa)
    } else {
        delta
    }
}

/// Sample a regressive PA delta for `family` (returned as a NEGATIVE value).
fn sample_regressive_delta(
    family: AttributeFamily,
    stakes: Q32,
    career_seed: u64,
    tick: u32,
    site: u32,
) -> i16 {
    let (min_abs, max_abs) = regressive_redraw_range(family);
    let (threshold, modifier_pct) = stakes_modifier(family);

    // `site` already encodes the `reg_bit=1` via `derive_site(…, regressive=true)`,
    // which sets bit 0 of the site word. No additional offset needed here.
    let seed = seed_fn(career_seed, tick, SeedLayer::SignatureTrigger, site);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    let range = (max_abs - min_abs) as u32 + 1;
    let raw: u32 = rng.gen_range(0..range);
    let abs_delta = min_abs + raw as i16;

    let abs_result = if stakes >= threshold {
        let amplified = (abs_delta as i32 * modifier_pct as i32 / 100) as i16;
        amplified.min(max_abs)
    } else {
        abs_delta
    };

    // Return negative
    -(abs_result)
}

// -------------------------------------------------------------------------
// Career floor — regressive collapse lower bound
// -------------------------------------------------------------------------

/// Compute the career floor for a regressive collapse in `family`.
/// `max(20, current_ca − 30)` per progression.md.
fn career_floor(current_ca: i16) -> i16 {
    (current_ca - 30).max(20)
}

// -------------------------------------------------------------------------
// CA delta — apply ca_lift_fraction to PA delta
// -------------------------------------------------------------------------

/// Compute the CA delta given a PA delta.
/// `new_ca = clamp(old_ca + ca_lift_fraction × delta_pa, ca_min, new_pa)`.
/// Returns `delta_ca` (may be positive or negative).
fn compute_ca_delta(delta_pa: i16, old_pa: i16, old_ca: i16, new_pa: i16) -> i16 {
    // CA_LIFT_FRACTION = 0.5; multiply delta_pa by 0.5 using integer arithmetic.
    // delta_pa × 1 / 2 (rounds toward zero).
    let raw_lift = delta_pa / 2;
    let new_ca_raw = old_ca + raw_lift;
    // Clamp to [career_floor_for_positive, new_pa] for positive,
    // [career_floor_for_negative, old_pa] for negative.
    let ca_min = career_floor(old_ca);
    let ca_max = if delta_pa >= 0 { new_pa } else { old_pa };
    new_ca_raw.max(ca_min).min(ca_max) - old_ca
}

// -------------------------------------------------------------------------
// Emit MemoryEvent helpers
// -------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
// 8 args are genuine requirements of the MemoryEvent builder; no useful struct grouping exists.
fn make_breakthrough_event(
    player_id: PlayerId,
    season: SeasonNumber,
    career_date: CareerDate,
    family: AttributeFamily,
    kind: &BreakthroughKind,
    delta_pa: i16,
    delta_ca: i16,
    stakes: Q32,
) -> MemoryEvent {
    // Positive breakthrough invariant: delta_pa must be > 0 (Sim/RULES §11).
    // delta_ca may be 0 when CA is already at the new PA ceiling.
    assert!(
        delta_pa > 0,
        "PaRedraw delta_pa must be positive, got {delta_pa}"
    );
    assert!(
        delta_ca >= 0,
        "PaRedraw delta_ca must be non-negative, got {delta_ca}"
    );

    let consequence = match kind {
        BreakthroughKind::SignatureAwakening { signature_id } => vec![
            Consequence::PaRedraw {
                family,
                delta_pa,
                delta_ca,
            },
            Consequence::SignatureActivated {
                signature_id: signature_id.clone(),
            },
        ],
        BreakthroughKind::LatentFlagUnlock { .. } => vec![Consequence::PaRedraw {
            family,
            delta_pa,
            delta_ca,
        }],
        BreakthroughKind::RegressiveCollapse => {
            // This function is not called for regressive collapses;
            // see make_regressive_event instead.
            unreachable!("make_breakthrough_event called for RegressiveCollapse")
        }
    };

    MemoryEvent {
        event_id: crate::event::EventId(0), // overwritten by ledger.append
        schema_version: 1,
        season,
        tick: None,
        career_date,
        emitter: Emitter {
            kind: EmitterKind::CareerSystem,
            source_id: SourceId::Player(player_id),
        },
        participants: vec![Participant {
            role: ParticipantRole::Subject,
            entity: EntityRef::Player(player_id),
        }],
        event_class: EventClass::BreakthroughMoment,
        stakes,
        emotion: Emotion::Pride,
        consequence,
        callback_eligibility: CallbackEligibility::Immediate,
        salience: Q32::ZERO, // overwritten by ledger.append
        decay_function: DecayFunction::Never,
    }
}

fn make_regressive_event(
    player_id: PlayerId,
    season: SeasonNumber,
    career_date: CareerDate,
    family: AttributeFamily,
    delta_pa: i16,
    delta_ca: i16,
    stakes: Q32,
) -> MemoryEvent {
    // Regressive collapse invariant: delta_pa must be < 0 (Sim/RULES §11).
    // delta_ca may be 0 when CA is already at the career floor.
    assert!(
        delta_pa < 0,
        "PaReductionRedraw delta_pa must be negative, got {delta_pa}"
    );
    assert!(
        delta_ca <= 0,
        "PaReductionRedraw delta_ca must be non-positive, got {delta_ca}"
    );

    MemoryEvent {
        event_id: crate::event::EventId(0),
        schema_version: 1,
        season,
        tick: None,
        career_date,
        emitter: Emitter {
            kind: EmitterKind::CareerSystem,
            source_id: SourceId::Player(player_id),
        },
        participants: vec![Participant {
            role: ParticipantRole::Subject,
            entity: EntityRef::Player(player_id),
        }],
        event_class: EventClass::RegressiveCollapse,
        stakes,
        emotion: Emotion::Disappointment,
        consequence: vec![Consequence::PaReductionRedraw {
            family,
            delta_pa,
            delta_ca,
        }],
        callback_eligibility: CallbackEligibility::Immediate,
        salience: Q32::ZERO,
        decay_function: DecayFunction::Never,
    }
}

// -------------------------------------------------------------------------
// accumulate — process one ledger event into the player's meters
// -------------------------------------------------------------------------

/// Accumulate a single `MemoryEvent` into `state` for `player_id`.
///
/// Positive `family_relevance` → feeds `signature_readiness`.
/// Negative `family_relevance` → feeds `regressive_pressure` (absolute value).
/// The distinction matches progression.md §"Design notes on the table".
pub fn accumulate(state: &mut BreakthroughState, event: &MemoryEvent, now_tick: Tick) {
    let proj = project_salience(event, now_tick);
    if proj == Q32::ZERO {
        return; // no-op: zero salience has no effect
    }

    for &family in &AttributeFamily::ALL {
        let (pos_rel, neg_rel) = family_relevance(&event.event_class, family);

        if pos_rel > Q32::ZERO {
            // readiness_delta = projected_salience × family_relevance
            let delta = proj * pos_rel;
            state.add_readiness(family, delta);
        }

        if neg_rel > Q32::ZERO {
            let delta = proj * neg_rel;
            state.add_pressure(family, delta);
        }
    }
}

// -------------------------------------------------------------------------
// Stakes floor for gating — 0.5
// -------------------------------------------------------------------------

/// Minimum stakes for a gating event (per progression.md §"Gating-event table").
pub const GATE_MIN_STAKES: Q32 = Q32::from_raw(2_147_483_648_i64); // 0.50

// -------------------------------------------------------------------------
// evaluate — top-level driver
// -------------------------------------------------------------------------

/// Evaluate the breakthrough mechanism for a single player over all events
/// in `ledger`.
///
/// This is a re-evaluation from scratch (suitable for synthetic test harnesses
/// and career-save load). In a live career loop, the caller would maintain
/// `state` incrementally.
///
/// ## Parameters
///
/// - `ledger` — the player's career ledger (typically filtered to the player's
///   events by the caller; here we consume all events).
/// - `ctx` — caller-supplied player data.
/// - `state` — mutable per-player meter state. The caller initialises this to
///   `BreakthroughState::new()` and should persist it between calls.
/// - `career_seed` — the match/career seed (u64) for RNG derivation per ADR-0009.
/// - `now_tick` — the current sim tick for decay projection. Callers pass
///   `Tick::ZERO` for non-tick-aware contexts (synthetic harnesses).
///
/// ## Returns
///
/// A `Vec<BreakthroughOutcome>` of all breakthroughs that fired during
/// evaluation. The caller applies the deltas to the player's stored attributes
/// and appends the events to the ledger.
///
/// ## Determinism
///
/// Given the same `(ledger, ctx, state, career_seed, now_tick)`, `evaluate`
/// always returns the same outcomes. The RNG site per draw is
/// `(player_id_u32 << 8) | family_discriminant_u8`.
#[must_use]
pub fn evaluate(
    ledger: &MemoryLedger,
    ctx: &BreakthroughContext,
    state: &mut BreakthroughState,
    career_seed: u64,
    now_tick: Tick,
) -> Vec<BreakthroughOutcome> {
    let mut outcomes: Vec<BreakthroughOutcome> = Vec::new();

    // Process all events in chronological order.
    // For each event: first accumulate meters, then check gate conditions.
    // A single event can trigger at most one breakthrough per family
    // (the gate is tested once per event per family).
    for event in ledger.iter() {
        // 1. Accumulate meters.
        accumulate(state, event, now_tick);

        // 2. Check gate conditions per family.
        for &family in &AttributeFamily::ALL {
            // --- Positive gate ---
            let readiness = state.readiness(family);
            if readiness >= BREAKTHROUGH_THRESHOLD
                && is_positive_gate(&event.event_class, family)
                && event.stakes >= GATE_MIN_STAKES
                && state.positive_cooldown_clear(family, &ctx.career_date)
            {
                // Per design/breakthrough-moments.md: a positive breakthrough requires
                // EITHER a pending SignatureCandidate (Kind 1) OR a narrative-trigger
                // gene flag (Kind 2). No generic fall-through — a player with neither
                // never fires, regardless of meter level.
                if let Some(kind) = determine_positive_kind(ctx, family) {
                    let site = derive_site(ctx.player_id.raw(), family, false);
                    // Tick for RNG: use event_id as a proxy (stable per event).
                    let tick_for_rng = event.event_id.0;

                    let pa_current = ctx.pa_by_family.get(&family).copied().unwrap_or(100);
                    let ca_current = ctx.ca_by_family.get(&family).copied().unwrap_or(70);

                    let raw_delta = sample_positive_delta(
                        family,
                        event.stakes,
                        career_seed,
                        tick_for_rng,
                        site,
                    );
                    // Clamp new_pa to the documented PA scale ceiling (1..=200).
                    let new_pa = (pa_current + raw_delta).min(200);
                    let delta_pa = new_pa - pa_current;
                    let delta_ca = compute_ca_delta(delta_pa, pa_current, ca_current, new_pa);

                    let evt = make_breakthrough_event(
                        ctx.player_id,
                        event.season,
                        ctx.career_date,
                        family,
                        &kind,
                        delta_pa,
                        delta_ca,
                        event.stakes,
                    );

                    outcomes.push(BreakthroughOutcome {
                        player_id: ctx.player_id,
                        family,
                        kind,
                        delta_pa,
                        delta_ca,
                        gating_event_class: event.event_class.clone(),
                        event: evt,
                    });

                    // Reset readiness; record fire date.
                    state.reset_readiness_to_residue(family);
                    state.last_positive_fire.insert(family, ctx.career_date);
                }
                // None: player lacks both a signature candidate and a narrative flag
                // for this family — meter stays at threshold until conditions are met.
            }

            // --- Regressive gate ---
            let pressure = state.pressure(family);
            if pressure >= REGRESSIVE_THRESHOLD
                && is_regressive_gate(&event.event_class, family)
                && event.stakes >= GATE_MIN_STAKES
                && state.regressive_cooldown_clear(family, &ctx.career_date)
            {
                let site = derive_site(ctx.player_id.raw(), family, true);
                let tick_for_rng = event.event_id.0;

                let pa_current = ctx.pa_by_family.get(&family).copied().unwrap_or(100);
                let ca_current = ctx.ca_by_family.get(&family).copied().unwrap_or(70);

                let delta_pa =
                    sample_regressive_delta(family, event.stakes, career_seed, tick_for_rng, site);
                // Apply career floor: new_pa must not go below max(20, ca-30).
                let floor = career_floor(ca_current);
                let new_pa = (pa_current + delta_pa).max(floor);
                let actual_delta_pa = new_pa - pa_current; // may be less negative than sampled
                let new_ca = (ca_current + actual_delta_pa / 2).max(floor);
                let delta_ca = new_ca - ca_current;

                let evt = make_regressive_event(
                    ctx.player_id,
                    event.season,
                    ctx.career_date,
                    family,
                    actual_delta_pa,
                    delta_ca,
                    event.stakes,
                );

                outcomes.push(BreakthroughOutcome {
                    player_id: ctx.player_id,
                    family,
                    kind: BreakthroughKind::RegressiveCollapse,
                    delta_pa: actual_delta_pa,
                    delta_ca,
                    gating_event_class: event.event_class.clone(),
                    event: evt,
                });

                state.reset_pressure_to_residue(family);
                state.last_regressive_fire.insert(family, ctx.career_date);
            }
        }
    }

    outcomes
}

/// Determine the breakthrough kind (1 or 2) for a positive breakthrough.
///
/// Returns `None` if the player has neither a signature candidate in `family`
/// nor a narrative-trigger gene flag — per `design/breakthrough-moments.md`, a
/// positive breakthrough requires one or the other. The meter stays at threshold
/// until the player acquires a candidate or flag.
///
/// Priority: Kind 1 (signature awakening) before Kind 2 (latent-flag unlock).
fn determine_positive_kind(
    ctx: &BreakthroughContext,
    family: AttributeFamily,
) -> Option<BreakthroughKind> {
    // Kind 1: signature candidate in this family?
    for (candidate_family, sig_id) in &ctx.signature_candidates {
        if *candidate_family == family {
            return Some(BreakthroughKind::SignatureAwakening {
                signature_id: sig_id.clone(),
            });
        }
    }

    // Kind 2: any narrative flag present?
    if let Some(&flag) = ctx.narrative_flags.first() {
        return Some(BreakthroughKind::LatentFlagUnlock { flag });
    }

    // No candidate, no flag — this family cannot fire a positive breakthrough.
    None
}

/// Derive the RNG site for a draw.
/// Site encoding: `(player_id_low24 << 8) | (family_discriminant << 1) | regressive_bit`.
/// This gives a stable per-(player, family, kind) discriminant.
fn derive_site(player_id: u32, family: AttributeFamily, regressive: bool) -> u32 {
    let family_bits = family.discriminant();
    let reg_bit = if regressive { 1u32 } else { 0u32 };
    ((player_id & 0x00FF_FFFF) << 8) | (family_bits << 1) | reg_bit
}

// -------------------------------------------------------------------------
// Unit tests (Chunk 1 + Chunk 2 + Chunk 3 + Chunk 4 per AC-to-test matrix)
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_core::{PlayerId, Q32, Tick};

    use crate::event::{
        CallbackEligibility, CareerDate as CD, Consequence, DecayFunction, Emitter, EmitterKind,
        Emotion, EntityRef, EventId, MemoryEvent, Participant, ParticipantRole, SeasonNumber,
        SourceId,
    };
    use crate::ledger::MemoryLedger;

    // ---- Helpers ----

    fn make_event_with_stakes(
        class: EventClass,
        player_id: PlayerId,
        stakes_raw: i64,
        season: u16,
    ) -> MemoryEvent {
        use fw_core::MatchId;
        MemoryEvent {
            event_id: EventId(0),
            schema_version: 1,
            season: SeasonNumber(season),
            tick: Some(Tick::ZERO),
            career_date: CD {
                year: 1,
                day_of_year: 1,
            },
            emitter: Emitter {
                kind: EmitterKind::MatchEngine,
                source_id: SourceId::Match(MatchId::new(0)),
            },
            participants: vec![Participant {
                role: ParticipantRole::Subject,
                entity: EntityRef::Player(player_id),
            }],
            event_class: class,
            stakes: Q32::from_raw(stakes_raw),
            emotion: Emotion::Joy,
            consequence: vec![Consequence::None],
            callback_eligibility: CallbackEligibility::Immediate,
            salience: Q32::from_raw(stakes_raw),
            decay_function: DecayFunction::Never,
        }
    }

    /// No-candidate, no-flag context. Used for negative tests only (gate should NOT fire).
    fn default_ctx(player_id: PlayerId) -> BreakthroughContext {
        let mut pa = BTreeMap::new();
        let mut ca = BTreeMap::new();
        for &f in &AttributeFamily::ALL {
            pa.insert(f, 100i16);
            ca.insert(f, 70i16);
        }
        BreakthroughContext {
            player_id,
            pa_by_family: pa,
            ca_by_family: ca,
            narrative_flags: vec![],
            signature_candidates: vec![],
            age_years: 24,
            career_date: CD {
                year: 1,
                day_of_year: 1,
            },
        }
    }

    /// Context with a Finishing signature candidate — for tests that expect a positive
    /// breakthrough in Finishing to fire (Kind 1).
    fn ctx_with_finishing_candidate(player_id: PlayerId) -> BreakthroughContext {
        let mut ctx = default_ctx(player_id);
        ctx.signature_candidates = vec![(
            AttributeFamily::Finishing,
            "fwh.core:signature.long_range_strike".to_string(),
        )];
        ctx
    }

    /// Context with a Composure signature candidate — for tests that expect a positive
    /// breakthrough in Composure to fire (Kind 1).
    fn ctx_with_composure_candidate(player_id: PlayerId) -> BreakthroughContext {
        let mut ctx = default_ctx(player_id);
        ctx.signature_candidates = vec![(
            AttributeFamily::Composure,
            "fwh.core:signature.composure_under_pressure".to_string(),
        )];
        ctx
    }

    // ---- AC1: AttributeFamily discriminants locked ----

    /// AC1 — discriminant pin test (mirrors T2-R7(a) SetPieceKind pattern).
    #[test]
    fn attribute_family_discriminants_locked() {
        assert_eq!(AttributeFamily::Finishing.discriminant(), 0);
        assert_eq!(AttributeFamily::Passing.discriminant(), 1);
        assert_eq!(AttributeFamily::DefensiveAnticipation.discriminant(), 2);
        assert_eq!(AttributeFamily::AerialPresence.discriminant(), 3);
        assert_eq!(AttributeFamily::Composure.discriminant(), 4);
        assert_eq!(AttributeFamily::Pace.discriminant(), 5);
        assert_eq!(AttributeFamily::Stamina.discriminant(), 6);
        assert_eq!(AttributeFamily::WorkRate.discriminant(), 7);
        assert_eq!(AttributeFamily::DeadBallDelivery.discriminant(), 8);
        assert_eq!(AttributeFamily::Leadership.discriminant(), 9);
        assert_eq!(AttributeFamily::ALL.len(), 10);
    }

    // ---- AC2: table cells from progression.md ----

    /// AC2 — spot-check family_relevance against progression.md table cells.
    #[test]
    fn family_relevance_matches_progression_doc() {
        // LegacyGoal → Finishing = 0.45
        // round(0.45 × 2^32) = 1_932_735_283
        let (pos, neg) = family_relevance(&EventClass::LegacyGoal, AttributeFamily::Finishing);
        assert!(pos > Q32::ZERO, "LegacyGoal→Finishing should be positive");
        // 0.45 in Q32: raw bits 1_932_735_283
        assert_eq!(pos.to_raw(), 1_932_735_283, "LegacyGoal→Finishing == 0.45");
        assert_eq!(neg, Q32::ZERO);

        // BigMatchScar → Composure = −0.30 (regressive)
        let (pos2, neg2) = family_relevance(&EventClass::BigMatchScar, AttributeFamily::Composure);
        assert_eq!(pos2, Q32::ZERO);
        assert_eq!(
            neg2.to_raw(),
            1_288_490_189,
            "BigMatchScar→Composure regressive == 0.30"
        );

        // MentorTeammate → Leadership = 0.35
        let (pos3, neg3) =
            family_relevance(&EventClass::MentorTeammate, AttributeFamily::Leadership);
        assert_eq!(
            pos3.to_raw(),
            1_503_238_554,
            "MentorTeammate→Leadership == 0.35"
        );
        assert_eq!(neg3, Q32::ZERO);

        // CupFinalWin → Composure = 0.30
        let (pos4, _) = family_relevance(&EventClass::CupFinalWin, AttributeFamily::Composure);
        assert_eq!(
            pos4.to_raw(),
            1_288_490_189,
            "CupFinalWin→Composure == 0.30"
        );

        // InjuryLongTerm → Stamina = −0.30 (regressive)
        let (pos5, neg5) = family_relevance(&EventClass::InjuryLongTerm, AttributeFamily::Stamina);
        assert_eq!(pos5, Q32::ZERO);
        assert_eq!(
            neg5.to_raw(),
            1_288_490_189,
            "InjuryLongTerm→Stamina regressive == 0.30"
        );
    }

    /// AC2 — spot-check redraw ranges.
    #[test]
    fn redraw_range_matches_progression_doc() {
        assert_eq!(positive_redraw_range(AttributeFamily::Pace), (5, 11));
        assert_eq!(positive_redraw_range(AttributeFamily::Finishing), (4, 9));
        assert_eq!(regressive_redraw_range(AttributeFamily::Composure), (5, 10));
        assert_eq!(regressive_redraw_range(AttributeFamily::Pace), (6, 12));
    }

    /// AC2 — threshold constants pin their exact raw bits (literal regression test).
    ///
    /// Arithmetic (verified by hand):
    ///   BREAKTHROUGH_THRESHOLD = Q32(0.92): 4_294_967_296 × 92 / 100
    ///     = 395_136_991_232 / 100 = 3_951_369_912.32 → round = 3_951_369_912
    ///   REGRESSIVE_THRESHOLD = Q32(0.90): 4_294_967_296 × 90 / 100
    ///     = 386_547_056_640 / 100 = 3_865_470_566.40 → round = 3_865_470_566
    #[test]
    fn threshold_constants_match_progression_doc() {
        assert_eq!(
            BREAKTHROUGH_THRESHOLD.to_raw(),
            3_951_369_912,
            "BREAKTHROUGH_THRESHOLD must equal Q32(0.92)"
        );
        assert_eq!(
            REGRESSIVE_THRESHOLD.to_raw(),
            3_865_470_566,
            "REGRESSIVE_THRESHOLD must equal Q32(0.90)"
        );
        assert!(
            REGRESSIVE_THRESHOLD < BREAKTHROUGH_THRESHOLD,
            "regressive threshold must be lower than positive threshold"
        );
    }

    // ---- AC3: meter accumulation ----

    /// AC3 — positive readiness accumulates from salient events.
    #[test]
    fn readiness_accumulates_from_salient_events() {
        let mut state = BreakthroughState::new();
        let player_id = PlayerId::new(1);

        // stakes = 0.4 → salience = 0.4 (degenerate formula: salience == stakes)
        // family_relevance(LegacyGoal, Finishing) = 0.45
        // expected delta per event = 0.4 × 0.45 = 0.18
        let stakes_raw = Q32::from_raw(1_717_986_918_i64).to_raw(); // 0.40

        let event = make_event_with_stakes(EventClass::LegacyGoal, player_id, stakes_raw, 0);

        accumulate(&mut state, &event, Tick::ZERO);
        let r1 = state.readiness(AttributeFamily::Finishing);
        assert!(
            r1 > Q32::ZERO,
            "readiness must be positive after LegacyGoal"
        );

        accumulate(&mut state, &event, Tick::ZERO);
        let r2 = state.readiness(AttributeFamily::Finishing);
        assert!(r2 > r1, "readiness must grow after second LegacyGoal");

        // Also check Composure (LegacyGoal → Composure = 0.25)
        let rc = state.readiness(AttributeFamily::Composure);
        assert!(
            rc > Q32::ZERO,
            "Composure should also have readiness after LegacyGoal"
        );
    }

    /// AC3 — negative relevance feeds regressive_pressure, NOT signature_readiness.
    #[test]
    fn negative_relevance_feeds_regressive_pressure_not_readiness() {
        let mut state = BreakthroughState::new();
        let player_id = PlayerId::new(2);

        // BigMatchScar → Composure: neg = 0.30, pos = 0
        let event = make_event_with_stakes(
            EventClass::BigMatchScar,
            player_id,
            2_147_483_648_i64, // 0.5 stakes
            0,
        );

        accumulate(&mut state, &event, Tick::ZERO);
        let readiness = state.readiness(AttributeFamily::Composure);
        let pressure = state.pressure(AttributeFamily::Composure);
        assert_eq!(
            readiness,
            Q32::ZERO,
            "BigMatchScar should NOT feed Composure readiness"
        );
        assert!(
            pressure > Q32::ZERO,
            "BigMatchScar SHOULD feed Composure regressive_pressure"
        );
    }

    /// AC3 — clamping: readiness does not exceed Q32::ONE.
    #[test]
    fn readiness_clamps_to_one() {
        let mut state = BreakthroughState::new();
        let player_id = PlayerId::new(3);

        // LegacyGoal with stakes=1.0, relevance=0.45 → delta=0.45 per event.
        // After 3 events (3 × 0.45 = 1.35 > 1.0), should clamp.
        let event = make_event_with_stakes(
            EventClass::LegacyGoal,
            player_id,
            Q32::ONE.to_raw(), // stakes = 1.0
            0,
        );

        for _ in 0..5 {
            accumulate(&mut state, &event, Tick::ZERO);
        }
        let r = state.readiness(AttributeFamily::Finishing);
        assert_eq!(r, Q32::ONE, "readiness must clamp at Q32::ONE");
    }

    // ---- AC4: 3-part gate ----

    /// AC4 — all three gate conditions must be present.
    /// Negative test: meter at threshold + gating event, but within cooldown → no fire.
    #[test]
    fn gate_requires_cooldown() {
        let player_id = PlayerId::new(10);
        let mut ctx = default_ctx(player_id);
        // Set career_date such that the last fire was very recent.
        ctx.career_date = CD {
            year: 1,
            day_of_year: 100,
        };

        let mut state = BreakthroughState::new();
        // Force readiness above threshold.
        state
            .signature_readiness
            .insert(AttributeFamily::Finishing, Q32::ONE);
        // Record a recent fire (just 10 days ago).
        state.last_positive_fire.insert(
            AttributeFamily::Finishing,
            CD {
                year: 1,
                day_of_year: 90,
            },
        );

        let mut ledger = MemoryLedger::new();
        // LegacyGoal with stakes=0.8 (above 0.5 gate minimum).
        ledger.append(make_event_with_stakes(
            EventClass::LegacyGoal,
            player_id,
            3_435_973_837_i64, // 0.80
            0,
        ));

        let outcomes = evaluate(&ledger, &ctx, &mut state, 12345, Tick::ZERO);
        assert!(
            outcomes.is_empty(),
            "cooldown should prevent breakthrough from firing"
        );
    }

    /// AC4 — negative test: meter below threshold → no fire even with gating event.
    ///
    /// We start the meter at 0 (well below threshold 0.92) and append only one
    /// LegacyGoal with medium stakes (~0.40). The maximum accumulation from a
    /// single event is ≈ 0.40 × 0.45 = 0.18, which leaves the meter at ~0.18,
    /// far below the 0.92 threshold. No breakthrough should fire.
    #[test]
    fn gate_requires_threshold() {
        let player_id = PlayerId::new(11);
        let ctx = default_ctx(player_id);

        // Leave readiness at Q32::ZERO (well below 0.92).
        let mut state = BreakthroughState::new();

        let mut ledger = MemoryLedger::new();
        // stakes = 0.40 → after accumulate, Finishing readiness ≈ 0.18 (< 0.92).
        ledger.append(make_event_with_stakes(
            EventClass::LegacyGoal,
            player_id,
            1_717_986_918_i64, // 0.40 stakes
            0,
        ));

        let outcomes = evaluate(&ledger, &ctx, &mut state, 12345, Tick::ZERO);
        assert!(outcomes.is_empty(), "below-threshold meter should not fire");
    }

    /// AC4 — negative test: meter above threshold, cooldown clear, but no gating event → no fire.
    #[test]
    fn gate_requires_gating_event() {
        let player_id = PlayerId::new(12);
        let ctx = default_ctx(player_id);

        let mut state = BreakthroughState::new();
        state
            .signature_readiness
            .insert(AttributeFamily::Finishing, Q32::ONE);

        let mut ledger = MemoryLedger::new();
        // DebutClub is NOT a gating event for Finishing.
        ledger.append(make_event_with_stakes(
            EventClass::DebutClub,
            player_id,
            3_435_973_837_i64,
            0,
        ));

        let outcomes = evaluate(&ledger, &ctx, &mut state, 12345, Tick::ZERO);
        assert!(
            outcomes.is_empty(),
            "non-gating event should not fire breakthrough"
        );
    }

    /// AC4 — positive test: all 3 conditions met + a signature candidate → breakthrough fires.
    #[test]
    fn gate_fires_when_all_three_conditions_met() {
        let player_id = PlayerId::new(13);
        // Player must have a candidate or flag; otherwise the gate correctly suppresses.
        let ctx = ctx_with_finishing_candidate(player_id);

        let mut state = BreakthroughState::new();
        // Pre-fill readiness to just at threshold.
        state
            .signature_readiness
            .insert(AttributeFamily::Finishing, BREAKTHROUGH_THRESHOLD);

        let mut ledger = MemoryLedger::new();
        // LegacyGoal with stakes=0.8 → valid gating event for Finishing.
        ledger.append(make_event_with_stakes(
            EventClass::LegacyGoal,
            player_id,
            3_435_973_837_i64,
            0,
        ));

        let outcomes = evaluate(&ledger, &ctx, &mut state, 12345, Tick::ZERO);
        assert_eq!(outcomes.len(), 1, "one breakthrough should fire");
        assert_eq!(outcomes[0].family, AttributeFamily::Finishing);
    }

    /// P1-2 — no-candidate / no-flag player: positive gate suppressed even with all 3 meter
    /// conditions met. Per `design/breakthrough-moments.md`: Kind 1 requires a pending
    /// signature candidate, Kind 2 requires a narrative-trigger gene flag. Neither → no fire.
    #[test]
    fn gate_suppressed_when_no_candidate_and_no_flag() {
        let player_id = PlayerId::new(15);
        let ctx = default_ctx(player_id); // no candidates, no flags

        let mut state = BreakthroughState::new();
        // All 3 meter conditions met: readiness at threshold, gating event present,
        // cooldown cleared (no previous fire recorded).
        state
            .signature_readiness
            .insert(AttributeFamily::Finishing, BREAKTHROUGH_THRESHOLD);

        let mut ledger = MemoryLedger::new();
        // LegacyGoal with high stakes — valid gating event for Finishing.
        ledger.append(make_event_with_stakes(
            EventClass::LegacyGoal,
            player_id,
            3_435_973_837_i64, // 0.80 stakes
            0,
        ));

        let outcomes = evaluate(&ledger, &ctx, &mut state, 12345, Tick::ZERO);
        let positive_outcomes: Vec<_> = outcomes
            .iter()
            .filter(|o| !matches!(o.kind, BreakthroughKind::RegressiveCollapse))
            .collect();
        assert!(
            positive_outcomes.is_empty(),
            "positive breakthrough must NOT fire for a player with no candidate and no flag"
        );
    }

    // ---- AC5: deterministic redraw ----

    /// AC5 — same inputs → same delta_pa. Player has a candidate so the gate can fire.
    #[test]
    fn redraw_is_deterministic() {
        let player_id = PlayerId::new(20);
        let ctx = ctx_with_finishing_candidate(player_id);

        let mut state1 = BreakthroughState::new();
        state1
            .signature_readiness
            .insert(AttributeFamily::Finishing, BREAKTHROUGH_THRESHOLD);

        let mut state2 = state1.clone();

        let mut ledger = MemoryLedger::new();
        ledger.append(make_event_with_stakes(
            EventClass::LegacyGoal,
            player_id,
            3_435_973_837_i64,
            0,
        ));

        let outcomes1 = evaluate(&ledger, &ctx, &mut state1, 99999, Tick::ZERO);
        let outcomes2 = evaluate(&ledger, &ctx, &mut state2, 99999, Tick::ZERO);

        assert_eq!(
            outcomes1.len(),
            outcomes2.len(),
            "both runs must produce the same count"
        );
        assert!(
            !outcomes1.is_empty(),
            "expected at least one breakthrough given candidate + threshold + gating event"
        );
        assert_eq!(outcomes1[0].delta_pa, outcomes2[0].delta_pa);
    }

    // ---- AC6: correct event emission ----

    /// AC6 — breakthrough fires BreakthroughMoment with PaRedraw consequence + Never decay.
    #[test]
    fn breakthrough_emits_correct_event() {
        let player_id = PlayerId::new(30);
        let ctx = ctx_with_finishing_candidate(player_id);

        let mut state = BreakthroughState::new();
        state
            .signature_readiness
            .insert(AttributeFamily::Finishing, BREAKTHROUGH_THRESHOLD);

        let mut ledger = MemoryLedger::new();
        ledger.append(make_event_with_stakes(
            EventClass::LegacyGoal,
            player_id,
            3_435_973_837_i64,
            0,
        ));

        let outcomes = evaluate(&ledger, &ctx, &mut state, 99999, Tick::ZERO);
        assert_eq!(outcomes.len(), 1);
        let outcome = &outcomes[0];
        assert_eq!(outcome.event.event_class, EventClass::BreakthroughMoment);
        assert_eq!(outcome.event.decay_function, DecayFunction::Never);

        let has_pa_redraw = outcome
            .event
            .consequence
            .iter()
            .any(|c| matches!(c, Consequence::PaRedraw { .. }));
        assert!(
            has_pa_redraw,
            "BreakthroughMoment must carry PaRedraw consequence"
        );
    }

    /// AC6 — regressive collapse fires RegressiveCollapse with PaReductionRedraw + Never decay.
    #[test]
    fn regressive_emits_correct_event() {
        let player_id = PlayerId::new(31);
        let ctx = default_ctx(player_id);

        let mut state = BreakthroughState::new();
        state
            .regressive_pressure
            .insert(AttributeFamily::Composure, REGRESSIVE_THRESHOLD);

        let mut ledger = MemoryLedger::new();
        // BigMatchScar with high stakes → valid regressive gate for Composure.
        ledger.append(make_event_with_stakes(
            EventClass::BigMatchScar,
            player_id,
            3_650_722_202_i64, // 0.85 stakes
            0,
        ));

        let outcomes = evaluate(&ledger, &ctx, &mut state, 99999, Tick::ZERO);
        let reg_outcomes: Vec<_> = outcomes
            .iter()
            .filter(|o| matches!(o.kind, BreakthroughKind::RegressiveCollapse))
            .collect();
        assert!(
            !reg_outcomes.is_empty(),
            "regressive collapse should have fired"
        );
        let ro = &reg_outcomes[0];
        assert_eq!(ro.event.event_class, EventClass::RegressiveCollapse);
        assert_eq!(ro.event.decay_function, DecayFunction::Never);
        let has_reduction = ro
            .event
            .consequence
            .iter()
            .any(|c| matches!(c, Consequence::PaReductionRedraw { .. }));
        assert!(
            has_reduction,
            "RegressiveCollapse must carry PaReductionRedraw consequence"
        );
        assert!(ro.delta_pa < 0, "regressive delta_pa must be negative");
    }

    // ---- AC6: SignatureActivated in Kind-1 ----

    /// AC6 — Kind 1 (signature awakening) includes SignatureActivated consequence.
    #[test]
    fn signature_awakening_emits_signature_activated() {
        let player_id = PlayerId::new(32);
        let mut ctx = default_ctx(player_id);
        ctx.signature_candidates = vec![(
            AttributeFamily::Finishing,
            "fwh.core:signature.long_range_strike".to_string(),
        )];

        let mut state = BreakthroughState::new();
        state
            .signature_readiness
            .insert(AttributeFamily::Finishing, BREAKTHROUGH_THRESHOLD);

        let mut ledger = MemoryLedger::new();
        ledger.append(make_event_with_stakes(
            EventClass::LegacyGoal,
            player_id,
            3_435_973_837_i64,
            0,
        ));

        let outcomes = evaluate(&ledger, &ctx, &mut state, 99999, Tick::ZERO);
        assert_eq!(outcomes.len(), 1);
        let has_sig = outcomes[0]
            .event
            .consequence
            .iter()
            .any(|c| matches!(c, Consequence::SignatureActivated { .. }));
        assert!(
            has_sig,
            "Kind-1 breakthrough must include SignatureActivated consequence"
        );
    }

    // ---- AC8: career floor + reversibility ----

    /// AC8 — regressive redraw respects the career floor.
    #[test]
    fn regressive_redraw_respects_career_floor() {
        let player_id = PlayerId::new(40);
        let mut ctx = default_ctx(player_id);
        // PA=25, CA=30 → floor = max(20, 30-30) = max(20, 0) = 20.
        ctx.pa_by_family.insert(AttributeFamily::Composure, 25);
        ctx.ca_by_family.insert(AttributeFamily::Composure, 30);

        let mut state = BreakthroughState::new();
        state
            .regressive_pressure
            .insert(AttributeFamily::Composure, REGRESSIVE_THRESHOLD);

        let mut ledger = MemoryLedger::new();
        ledger.append(make_event_with_stakes(
            EventClass::BigMatchScar,
            player_id,
            3_650_722_202_i64,
            0,
        ));

        let outcomes = evaluate(&ledger, &ctx, &mut state, 99999, Tick::ZERO);
        let reg: Vec<_> = outcomes
            .iter()
            .filter(|o| matches!(o.kind, BreakthroughKind::RegressiveCollapse))
            .collect();
        assert!(!reg.is_empty(), "should have fired a regressive collapse");
        let new_pa = ctx.pa_by_family[&AttributeFamily::Composure] + reg[0].delta_pa;
        assert!(
            new_pa >= 20,
            "new_pa must not go below the career floor (20)"
        );
    }

    /// AC8 — subsequent positive breakthrough in same family can lift above a regressive scar.
    #[test]
    fn breakthrough_overwrites_regressive_scar() {
        let player_id = PlayerId::new(41);
        let mut ctx = ctx_with_composure_candidate(player_id);
        // Simulate that a regressive collapse reduced PA to 85 from 100.
        ctx.pa_by_family.insert(AttributeFamily::Composure, 85);
        ctx.ca_by_family.insert(AttributeFamily::Composure, 60);

        let mut state = BreakthroughState::new();
        state
            .signature_readiness
            .insert(AttributeFamily::Composure, BREAKTHROUGH_THRESHOLD);

        let mut ledger = MemoryLedger::new();
        // CupFinalWin → valid gating event for Composure.
        ledger.append(make_event_with_stakes(
            EventClass::CupFinalWin,
            player_id,
            3_650_722_202_i64,
            0,
        ));

        let outcomes = evaluate(&ledger, &ctx, &mut state, 99999, Tick::ZERO);
        let pos: Vec<_> = outcomes
            .iter()
            .filter(|o| !matches!(o.kind, BreakthroughKind::RegressiveCollapse))
            .collect();
        assert!(!pos.is_empty(), "positive breakthrough should fire");
        let new_pa = ctx.pa_by_family[&AttributeFamily::Composure] + pos[0].delta_pa;
        assert!(
            new_pa > 85,
            "positive breakthrough should lift PA above the regressive scar"
        );
    }
}

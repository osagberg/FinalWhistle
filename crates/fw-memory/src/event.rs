//! `MemoryEvent` schema — the canonical record for one career-ledger entry.
//!
//! Port of ADR-0005 §"MemoryEvent schema". Accepted 2026-05-18 by T3-1.
//!
//! ## Determinism contract
//!
//! - All canonical numerics (`stakes`, `salience`) are `Q32`. No `f32`/`f64`.
//! - `BTreeMap`-only in the ledger. No `HashMap`/`HashSet`.
//! - `EventClass` carries `#[repr(u32)]` + explicit discriminants 0..30.
//!   Same tag-stability discipline as `SaveEnvelope` (T2-9).
//!   Re-ordering variants is a compile error (Rust forbids duplicate explicit
//!   discriminants). The wire bytes per discriminant are pinned by the
//!   `event_class_discriminants_locked_forever` test.

use fw_core::{ClubId, MatchId, PlayerId, Q32, Tick};
use serde::{Deserialize, Serialize};

// -------------------------------------------------------------------------
// Top-level record
// -------------------------------------------------------------------------

/// A single appended row in the career memory ledger. Immutable after
/// emission. Schema-versioned for forward migration.
///
/// Field order must not change casually — it affects `Serialize` output and
/// therefore the canonical BLAKE3 hash. Any reorder requires a schema-version
/// bump + migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryEvent {
    /// Stable identity. Allocated monotonically by the ledger at append.
    pub event_id: EventId,

    /// Schema version. Bumped only by forward-migration in
    /// `fw-content::migrations`; older fixtures stay byte-identical.
    /// Always `1` at T3-1 emission.
    pub schema_version: u16,

    /// In-game season number (0-indexed from save start).
    pub season: SeasonNumber,

    /// Sim tick at emission. Compaction may set this to `None` past the
    /// 5-season boundary; readers must tolerate `None` and fall back to
    /// `season`.
    pub tick: Option<Tick>,

    /// Calendar date the event "happened" in the fictional world.
    /// Surfaces in commentary and press readers.
    pub career_date: CareerDate,

    /// Who emitted the event. Distinguishes "match engine emitted a
    /// breakthrough" from "career system emitted a transfer" from "press
    /// reader emitted a quote that itself becomes referenceable".
    pub emitter: Emitter,

    /// Who/what is implicated. Ordered for stable hashing. Empty on system
    /// events (e.g. `Compaction`).
    pub participants: Vec<Participant>,

    /// The structural kind of event.
    pub event_class: EventClass,

    /// How weighty the event was *as it happened*, in [0, 1]. Stable over
    /// time — does not decay. The decay model lives on `decay_function`.
    /// Stored as Q32 (Sim/RULES §1 — no f32/f64 in canonical state).
    pub stakes: Q32,

    /// The dominant affective register at emission. Drives press tone, fan
    /// callback, scout-report flavor.
    ///
    /// NOT used to bias the sim — emotion is a read-side projection input
    /// only.
    pub emotion: Emotion,

    /// Ordered list of downstream effects encoded at emission.
    /// Consumed by readers; never mutated.
    pub consequence: Vec<Consequence>,

    /// When and under what conditions this event becomes recall-eligible.
    pub callback_eligibility: CallbackEligibility,

    /// Computed salience at emission, in [0, 1]. Cached on the event so
    /// readers don't recompute every query. The emission-time value is
    /// canonical; reader-side modifiers (callback age, player attention) are
    /// applied on top at read time without mutating this field.
    ///
    /// At T3-1 the placeholder formula always returns `Q32::ZERO`; the real
    /// 5-term blend lands at T3-2.
    pub salience: Q32,

    /// The decay shape applied by readers projecting current salience.
    /// `Never` for legacy events (cup wins, breakthroughs);
    /// `Exponential` for routine emotional events;
    /// `Linear` for mid-band events.
    pub decay_function: DecayFunction,
}

// -------------------------------------------------------------------------
// Identity + time
// -------------------------------------------------------------------------

/// Stable, monotonically-allocated event identity.
///
/// Allocated by `MemoryLedger::append`; the `u32` value is the 0-based
/// insertion index, so `EventId(0)` is always the first event in the ledger.
/// The `PartialOrd`/`Ord` derivation gives chronological ordering for free.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(pub u32);

/// In-game season number. 0-indexed from save start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SeasonNumber(pub u16);

/// Calendar date in the fictional world. Both fields are in the fictional
/// calendar — `year` is fictional years elapsed since the save epoch;
/// `day_of_year` is 1-based (1..=365).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CareerDate {
    /// Fictional year (e.g. 1 for the first season).
    pub year: u16,
    /// Day of the fictional year. 1-based, 1..=365.
    pub day_of_year: u16,
}

// -------------------------------------------------------------------------
// Emitter
// -------------------------------------------------------------------------

/// The subsystem that emitted an event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Emitter {
    /// The kind of subsystem.
    pub kind: EmitterKind,
    /// The specific source within the subsystem.
    pub source_id: SourceId,
}

/// Which subsystem emitted an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmitterKind {
    /// The match simulation engine.
    MatchEngine,
    /// The career management system (transfers, contracts, promises).
    CareerSystem,
    /// The press reader emitting a quote that itself becomes referenceable.
    PressReader,
    /// The scout reader logging a scouted observation.
    ScoutReader,
    /// The board relations system.
    BoardSystem,
    /// A mod extension (Phase 6+).
    ModExtension,
}

/// The specific entity within the emitting subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceId {
    /// A specific match.
    Match(MatchId),
    /// A specific club.
    Club(ClubId),
    /// A specific player.
    Player(PlayerId),
    /// A mod entity, identified by an opaque u32.
    Mod(u32),
    /// No specific source (e.g. career-system administrative events).
    None,
}

// -------------------------------------------------------------------------
// Participants
// -------------------------------------------------------------------------

/// A participant in an event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Participant {
    /// The role this entity played in the event.
    pub role: ParticipantRole,
    /// The entity itself.
    pub entity: EntityRef,
}

/// The role a participant plays in an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipantRole {
    /// The primary subject (the player the event is about).
    Subject,
    /// The counterparty (e.g. the club the player transferred to/from).
    Counterparty,
    /// An observing entity with no direct agency.
    Witness,
    /// An entity that benefits from the event.
    Beneficiary,
    /// An entity that suffers from the event.
    Victim,
    /// An authority making a decision (e.g. the manager, the board).
    Authority,
}

/// A reference to a durable entity in the career world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityRef {
    /// A player by their durable `PlayerId`.
    Player(PlayerId),
    /// A club by their durable `ClubId`.
    Club(ClubId),
}

// -------------------------------------------------------------------------
// Event class catalogue — 30 core variants + UnknownEventClass extension
// -------------------------------------------------------------------------

/// The structural kind of a `MemoryEvent`. Locked at schema_version = 1.
///
/// ## Tag-stability (LOAD-BEARING FOREVER)
///
/// `#[repr(u32)]` + explicit discriminants pin the wire encoding of each
/// variant. Re-ordering variants or changing discriminants is a compile error
/// via Rust's "duplicate discriminant" check. The wire bytes per discriminant
/// are further pinned by `event_class_discriminants_locked_forever` test.
///
/// Mod content packs add new classes via `UnknownEventClass { tag, payload }`
/// (discriminant 30). Promoting a mod class to core requires a schema-version
/// bump + migration.
///
/// ## Thematic groups
///
/// - Performance moments (6): 0..=5
/// - Contract / transfer arc (8): 6..=13
/// - Relational (4): 14..=17
/// - Competition arc (6): 18..=23
/// - Career-shape (5): 24..=28
/// - System (1): 29
/// - Mod extension (1): 30
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum EventClass {
    // --- Performance moments (6) ---
    /// Pillar 3 trigger — the redraw event itself. `DecayFunction::Never`.
    BreakthroughMoment = 0,
    /// A player's signature move executed cleanly for the first time.
    SignatureFirstFired = 1,
    /// A goal that survives compaction (cup final winner, derby winner).
    LegacyGoal = 2,
    /// Three or more goals in a match — surfaces in alumni recall.
    HatTrickScored = 3,
    /// Sub-par performance in a high-stakes context.
    BigMatchScar = 4,
    /// Pillar 3 inverse trigger — PA-down redraw. `DecayFunction::Never`.
    RegressiveCollapse = 5,

    // --- Contract / transfer arc (8) ---
    /// Manager promise event; emits a `BrokenPromise` if expired.
    PromisedYouthMinutes = 6,
    /// A previously-emitted promise expired without delivery.
    BrokenPromise = 7,
    /// Player turned down an offered renewal.
    ContractRenewalRejected = 8,
    /// Signed; the symmetric positive callback.
    ContractRenewalAccepted = 9,
    /// A player requested a move.
    TransferRequested = 10,
    /// The manager refused a transfer request.
    TransferRefused = 11,
    /// Sold against the player's wishes.
    SoldUnderProtest = 12,
    /// Arrived at the buzzer; surfaces as commentary flavor.
    BoughtOnDeadlineDay = 13,

    // --- Relational (4) ---
    /// Two players, or player-and-club, escalated to rival status.
    RivalryFormed = 14,
    /// An older player mentored a younger one across a season.
    MentorTeammate = 15,
    /// Red-card lash-out, after-match incident, manager touchline drama.
    DerbyControversy = 16,
    /// Player faces a club they previously played for.
    FormerClubReunion = 17,

    // --- Competition arc (6) ---
    /// A cup-final victory; near-permanent recall eligibility.
    CupFinalWin = 18,
    /// Lost a cup final.
    CupFinalLoss = 19,
    /// Promotion won.
    PromotionWon = 20,
    /// Relegation suffered.
    RelegationSuffered = 21,
    /// League title won.
    TitleWon = 22,
    /// A notable unbeaten streak ended.
    UnbeatenRunEnded = 23,

    // --- Career-shape (5) ---
    /// First senior appearance.
    DebutSenior = 24,
    /// First appearance for a new club.
    DebutClub = 25,
    /// The player retired; the ledger closes for them.
    Retirement = 26,
    /// Absence of three or more in-game months; surfaces in scout reports.
    InjuryLongTerm = 27,
    /// First national-team call (procedural-fantasy nation only).
    InternationalCallUp = 28,

    // --- System (1) ---
    /// Emitted by the 5-season compactor; records what was dropped +
    /// summarised. `DecayFunction::Never`.
    Compaction = 29,

    // --- Mod extension (1) ---
    /// Mod-defined event class the host engine does not recognise.
    /// Round-trips losslessly through serde + canonical hash. Core readers
    /// ignore it. A future schema migration may promote a specific `tag`
    /// into a first-class variant.
    UnknownEventClass {
        /// The mod's tag for this class. A content-pack-qualified string.
        tag: ModEventTag,
        /// Opaque bincode bytes from the mod's emitter. Not introspected by
        /// core.
        payload: Vec<u8>,
    } = 30,
}

/// Tag for a mod-defined event class. Content-pack-qualified string.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ModEventTag(pub String);

impl EventClass {
    /// The `#[repr(u32)]` discriminant for this variant.
    ///
    /// This is the canonical discriminant value locked by
    /// `event_class_discriminants_locked_forever`. It matches the explicit
    /// `= N` values in the enum definition. Changing ANY mapping here or in
    /// the enum definition is a schema-breaking change that requires a
    /// `schema_version` bump + migration.
    ///
    /// This method is the single canonical source for the discriminant →
    /// number mapping so that `ledger.rs`'s index keying and the pin tests
    /// both agree by construction.
    #[must_use]
    pub fn discriminant(&self) -> u32 {
        match self {
            EventClass::BreakthroughMoment => 0,
            EventClass::SignatureFirstFired => 1,
            EventClass::LegacyGoal => 2,
            EventClass::HatTrickScored => 3,
            EventClass::BigMatchScar => 4,
            EventClass::RegressiveCollapse => 5,
            EventClass::PromisedYouthMinutes => 6,
            EventClass::BrokenPromise => 7,
            EventClass::ContractRenewalRejected => 8,
            EventClass::ContractRenewalAccepted => 9,
            EventClass::TransferRequested => 10,
            EventClass::TransferRefused => 11,
            EventClass::SoldUnderProtest => 12,
            EventClass::BoughtOnDeadlineDay => 13,
            EventClass::RivalryFormed => 14,
            EventClass::MentorTeammate => 15,
            EventClass::DerbyControversy => 16,
            EventClass::FormerClubReunion => 17,
            EventClass::CupFinalWin => 18,
            EventClass::CupFinalLoss => 19,
            EventClass::PromotionWon => 20,
            EventClass::RelegationSuffered => 21,
            EventClass::TitleWon => 22,
            EventClass::UnbeatenRunEnded => 23,
            EventClass::DebutSenior => 24,
            EventClass::DebutClub => 25,
            EventClass::Retirement => 26,
            EventClass::InjuryLongTerm => 27,
            EventClass::InternationalCallUp => 28,
            EventClass::Compaction => 29,
            EventClass::UnknownEventClass { .. } => 30,
        }
    }
}

// -------------------------------------------------------------------------
// Salience + decay
// -------------------------------------------------------------------------

/// The decay shape applied by readers projecting current salience over time.
///
/// Decay is a **read-time projection** — the canonical `salience` field on
/// `MemoryEvent` is the emission value forever. Readers project current
/// salience as `emission_salience × decay_factor(decay_function, elapsed)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecayFunction {
    /// Salience stays at emission value forever. Used for cup finals,
    /// breakthroughs, retirement, broken-promise resolutions.
    Never,

    /// Salience(t) = emission_salience × max(0, 1 − elapsed / lifetime).
    /// Mid-band events. Reaches zero at `lifetime_ticks` and becomes a
    /// compaction candidate.
    Linear {
        /// In-sim ticks until salience reaches zero.
        lifetime_ticks: u32,
    },

    /// Salience(t) = emission_salience × exp(−elapsed / half_life).
    /// Routine emotional events (fan annoyance after a draw, training-ground
    /// frustration). Approximated with a Q32-friendly lookup table — no
    /// `exp` call in the hot path.
    Exponential {
        /// In-sim ticks for salience to halve.
        half_life_ticks: u32,
    },
}

// -------------------------------------------------------------------------
// Emotion, Consequence, CallbackEligibility — minimal-but-extensible starters
// -------------------------------------------------------------------------

/// The dominant affective register of a `MemoryEvent` at emission.
///
/// Drives press tone, fan callback, and scout-report flavor. NOT used to
/// bias the sim — emotion is a read-side projection input only.
///
/// ## T3-1 minimal starter
///
/// Six variants cover the primary emotional registers required by the initial
/// reader set (PressReader / FanReader / CoachReader). Additional registers
/// (e.g. `Shame`, `Gratitude`, `Awe`, `Frustration`) are additive — adding a
/// variant requires a schema_version bump only if existing readers would
/// interpret the new variant incorrectly. Until T3-2 lands the readers, new
/// variants are purely additive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Emotion {
    /// No dominant affect (default for system events and compact summaries).
    Neutral,
    /// Positive high-arousal (goals, wins, promotions).
    Joy,
    /// Negative high-arousal (sacking, controversy, relegation).
    Anger,
    /// Positive self-referential (breakthrough, debut, landmark achievement).
    Pride,
    /// Negative low-arousal (missed opportunity, failure in a big moment).
    Disappointment,
    /// Forward-looking positive (promotion push, comeback arc underway).
    Hope,
}

/// A downstream effect encoded at event emission.
///
/// Consumed by readers to phrase the event specifically. Stored on the event
/// so commentary / scout / press readers don't re-derive the effect from raw
/// fields.
///
/// ## T3-1 minimal starter
///
/// Three core consequence types cover the T3-1 scope. The `PaRedraw`,
/// `PaReductionRedraw`, and `SignatureActivated` variants were reserved for
/// T3-4 and are now appended below (additive — old saves still decode).
/// Mod content packs may supply additional consequence variants via opaque
/// bytes analogous to `UnknownEventClass`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Consequence {
    /// No downstream effect (most events).
    None,
    /// A previously-emitted promise event has now been broken. Carries the
    /// `EventId` of the original `PromisedYouthMinutes` event.
    PromiseBroken { original_event_id: EventId },
    /// Records how many events were in the 5-season compaction window at this
    /// boundary crossing. This includes events whose `tick` was already `None`
    /// from a prior compaction — `in_window_count` is the total window size, not
    /// the count of events newly tick-nulled this pass.
    CompactionDrop { in_window_count: u32 },

    // ---- T3-4 additions (appended additive — old saves decode None for these) ----
    /// A positive PA redraw from a `BreakthroughMoment` event.
    ///
    /// `delta_pa` is always positive. `delta_ca` is also positive (the floor
    /// catches up by `ca_lift_fraction`). Carried on the `BreakthroughMoment`
    /// event so commentary / scout / press readers can phrase the lift.
    PaRedraw {
        /// The attribute family that was redrawn upward.
        family: crate::breakthrough::AttributeFamily,
        /// PA delta (positive integer, PA-scale units 1..=200).
        delta_pa: i16,
        /// CA delta (positive integer, PA-scale units 1..=200).
        delta_ca: i16,
    },

    /// A negative PA redraw from a `RegressiveCollapse` event.
    ///
    /// `delta_pa` is always negative (bounded by the career floor).
    /// `delta_ca` is also negative. Carried on the `RegressiveCollapse` event.
    PaReductionRedraw {
        /// The attribute family that was redrawn downward.
        family: crate::breakthrough::AttributeFamily,
        /// PA delta (negative integer, bounded by `max(20, ca − 30)`).
        delta_pa: i16,
        /// CA delta (negative integer, partial catch-down).
        delta_ca: i16,
    },

    /// A signature candidate has been activated (Kind 1 breakthrough).
    ///
    /// Carried on the `BreakthroughMoment` event alongside `PaRedraw`.
    /// The signature transitions from candidate to active on the player's card.
    /// `signature_id` is the content-pack-qualified signature ID string
    /// (e.g. `"fwh.core:signature.first_time_diagonal"`). Using `String`
    /// avoids a `fw-memory → fw-content` dependency edge; the career system
    /// (caller) resolves the ID via its own content store.
    SignatureActivated {
        /// Content-pack-qualified signature ID.
        signature_id: String,
    },
}

/// When and under what conditions a `MemoryEvent` becomes recall-eligible for
/// reader callbacks (press conferences, fan chants, scout reports, coaching
/// decisions).
///
/// ## T3-1 minimal starter
///
/// Three eligibility modes cover the T3-1 scope. `AfterSeasons` and
/// `AfterMatchesForSubject` are T3-2 additions once the PressReader /
/// CoachReader cadence is wired.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallbackEligibility {
    /// Eligible immediately after emission.
    Immediate,
    /// Eligible after the event's tick-level data has been compacted (i.e.
    /// once `tick` becomes `None`). Used for long-arc summary events.
    AfterCompaction,
    /// Never eligible for reader callbacks. Used for system / housekeeping
    /// events (e.g. `Compaction` itself).
    Never,
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_core::{ClubId, MatchId, PlayerId, Q32, Tick};

    fn minimal_event() -> MemoryEvent {
        MemoryEvent {
            event_id: EventId(0),
            schema_version: 1,
            season: SeasonNumber(0),
            tick: Some(Tick::ZERO),
            career_date: CareerDate {
                year: 1,
                day_of_year: 1,
            },
            emitter: Emitter {
                kind: EmitterKind::MatchEngine,
                source_id: SourceId::Match(MatchId::new(0)),
            },
            participants: vec![Participant {
                role: ParticipantRole::Subject,
                entity: EntityRef::Player(PlayerId::new(1)),
            }],
            event_class: EventClass::DebutSenior,
            stakes: Q32::ZERO,
            emotion: Emotion::Pride,
            consequence: vec![Consequence::None],
            callback_eligibility: CallbackEligibility::Immediate,
            salience: Q32::ZERO,
            decay_function: DecayFunction::Never,
        }
    }

    /// AC1 — every supporting type is constructible + the MemoryEvent struct
    /// compiles with all 14 field types populated.
    #[test]
    fn all_types_construct() {
        let ev = minimal_event();
        assert_eq!(ev.schema_version, 1);
        assert_eq!(ev.event_class, EventClass::DebutSenior);
        assert_eq!(ev.salience, Q32::ZERO);
        assert_eq!(ev.stakes, Q32::ZERO);
    }

    /// AC1 extension — round-trip through serde_json (structural serde
    /// correctness; bincode round-trip is exercised in fw-save tests).
    #[test]
    fn all_types_serde_round_trip() {
        let ev = minimal_event();
        let json = serde_json::to_string(&ev).expect("serialize");
        let decoded: MemoryEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ev, decoded);
    }

    /// AC1 — every `EventClass` variant is constructible.
    #[test]
    fn all_event_class_variants_construct() {
        let classes = [
            EventClass::BreakthroughMoment,
            EventClass::SignatureFirstFired,
            EventClass::LegacyGoal,
            EventClass::HatTrickScored,
            EventClass::BigMatchScar,
            EventClass::RegressiveCollapse,
            EventClass::PromisedYouthMinutes,
            EventClass::BrokenPromise,
            EventClass::ContractRenewalRejected,
            EventClass::ContractRenewalAccepted,
            EventClass::TransferRequested,
            EventClass::TransferRefused,
            EventClass::SoldUnderProtest,
            EventClass::BoughtOnDeadlineDay,
            EventClass::RivalryFormed,
            EventClass::MentorTeammate,
            EventClass::DerbyControversy,
            EventClass::FormerClubReunion,
            EventClass::CupFinalWin,
            EventClass::CupFinalLoss,
            EventClass::PromotionWon,
            EventClass::RelegationSuffered,
            EventClass::TitleWon,
            EventClass::UnbeatenRunEnded,
            EventClass::DebutSenior,
            EventClass::DebutClub,
            EventClass::Retirement,
            EventClass::InjuryLongTerm,
            EventClass::InternationalCallUp,
            EventClass::Compaction,
            EventClass::UnknownEventClass {
                tag: ModEventTag("mod.test:event.custom".to_string()),
                payload: vec![0xDE, 0xAD],
            },
        ];
        assert_eq!(classes.len(), 31, "30 core + 1 UnknownEventClass");
    }

    /// AC2 — EventClass discriminants are pinned at their compile-time values.
    /// Re-ordering any variant shifts its discriminant and fails this test.
    ///
    /// Uses `EventClass::discriminant()` — the canonical mapping function
    /// (single source of truth, shared with `ledger.rs` index keying). An
    /// `as u32` cast would only work on unit-only enums; `discriminant()` is
    /// the idiomatic alternative for enums with data variants.
    ///
    /// Mirrors T2-R7(a) SetPieceKind pin pattern.
    #[test]
    fn event_class_discriminants_locked_forever() {
        // Performance moments
        assert_eq!(EventClass::BreakthroughMoment.discriminant(), 0);
        assert_eq!(EventClass::SignatureFirstFired.discriminant(), 1);
        assert_eq!(EventClass::LegacyGoal.discriminant(), 2);
        assert_eq!(EventClass::HatTrickScored.discriminant(), 3);
        assert_eq!(EventClass::BigMatchScar.discriminant(), 4);
        assert_eq!(EventClass::RegressiveCollapse.discriminant(), 5);

        // Contract / transfer arc
        assert_eq!(EventClass::PromisedYouthMinutes.discriminant(), 6);
        assert_eq!(EventClass::BrokenPromise.discriminant(), 7);
        assert_eq!(EventClass::ContractRenewalRejected.discriminant(), 8);
        assert_eq!(EventClass::ContractRenewalAccepted.discriminant(), 9);
        assert_eq!(EventClass::TransferRequested.discriminant(), 10);
        assert_eq!(EventClass::TransferRefused.discriminant(), 11);
        assert_eq!(EventClass::SoldUnderProtest.discriminant(), 12);
        assert_eq!(EventClass::BoughtOnDeadlineDay.discriminant(), 13);

        // Relational
        assert_eq!(EventClass::RivalryFormed.discriminant(), 14);
        assert_eq!(EventClass::MentorTeammate.discriminant(), 15);
        assert_eq!(EventClass::DerbyControversy.discriminant(), 16);
        assert_eq!(EventClass::FormerClubReunion.discriminant(), 17);

        // Competition arc
        assert_eq!(EventClass::CupFinalWin.discriminant(), 18);
        assert_eq!(EventClass::CupFinalLoss.discriminant(), 19);
        assert_eq!(EventClass::PromotionWon.discriminant(), 20);
        assert_eq!(EventClass::RelegationSuffered.discriminant(), 21);
        assert_eq!(EventClass::TitleWon.discriminant(), 22);
        assert_eq!(EventClass::UnbeatenRunEnded.discriminant(), 23);

        // Career-shape
        assert_eq!(EventClass::DebutSenior.discriminant(), 24);
        assert_eq!(EventClass::DebutClub.discriminant(), 25);
        assert_eq!(EventClass::Retirement.discriminant(), 26);
        assert_eq!(EventClass::InjuryLongTerm.discriminant(), 27);
        assert_eq!(EventClass::InternationalCallUp.discriminant(), 28);

        // System
        assert_eq!(EventClass::Compaction.discriminant(), 29);

        // Mod extension
        assert_eq!(
            EventClass::UnknownEventClass {
                tag: ModEventTag(String::new()),
                payload: vec![],
            }
            .discriminant(),
            30
        );
    }

    /// AC1 + unknown-class round-trip: UnknownEventClass payload bytes survive
    /// serde round-trip without modification.
    #[test]
    fn unknown_event_class_payload_round_trips() {
        let payload = vec![0xCA, 0xFE, 0xBA, 0xBE];
        let tag = ModEventTag("mod.community.somerset:event.mystery".to_string());
        let class = EventClass::UnknownEventClass {
            tag: tag.clone(),
            payload: payload.clone(),
        };
        let json = serde_json::to_string(&class).expect("serialize");
        let decoded: EventClass = serde_json::from_str(&json).expect("deserialize");
        match decoded {
            EventClass::UnknownEventClass { tag: t, payload: p } => {
                assert_eq!(t, tag);
                assert_eq!(p, payload);
            }
            _ => panic!("expected UnknownEventClass after round-trip"),
        }
    }

    /// AC1 — ClubId-based SourceId + Participant with Club EntityRef construct
    /// cleanly (exercise the non-Player paths).
    #[test]
    fn emitter_and_participant_club_paths_construct() {
        let emitter = Emitter {
            kind: EmitterKind::CareerSystem,
            source_id: SourceId::Club(ClubId::new(42)),
        };
        let participant = Participant {
            role: ParticipantRole::Counterparty,
            entity: EntityRef::Club(ClubId::new(42)),
        };
        assert_eq!(emitter.kind, EmitterKind::CareerSystem);
        assert_eq!(participant.role, ParticipantRole::Counterparty);
    }

    /// AC1 — DecayFunction variants construct + carry their params.
    #[test]
    fn decay_function_variants_construct() {
        let never = DecayFunction::Never;
        let linear = DecayFunction::Linear {
            lifetime_ticks: 1800,
        };
        let exp = DecayFunction::Exponential {
            half_life_ticks: 300,
        };

        assert_eq!(never, DecayFunction::Never);
        match linear {
            DecayFunction::Linear { lifetime_ticks } => assert_eq!(lifetime_ticks, 1800),
            _ => panic!("wrong variant"),
        }
        match exp {
            DecayFunction::Exponential { half_life_ticks } => assert_eq!(half_life_ticks, 300),
            _ => panic!("wrong variant"),
        }
    }
}

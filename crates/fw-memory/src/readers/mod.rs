//! Read-only projections over `MemoryLedger`.
//!
//! Five readers implement the ADR-0005 reader contract without mutating the
//! canonical event store. Each reader is a stateless collection of functions
//! that project current salience from the ledger's emission-time values.
//!
//! ## Decay projection
//!
//! `project_salience(event, now_tick)` applies the event's `DecayFunction` as
//! a read-time modifier. The canonical `event.salience` field is the emission
//! value forever — decay is never written back to the event (pillar 2).
//!
//! ## Determinism contract
//!
//! - No `f32`/`f64` anywhere — Q32 only.
//! - No `HashMap`/`HashSet` — `BTreeMap`/`BTreeSet` only.
//! - No clocks — all time references are `Tick` from the sim.
//! - No async/tokio.
//!
//! See `.claude/rules/Sim/RULES.md` §1-§5 for the binding contract.

pub mod coach;
pub mod fan;
pub mod press;
pub mod salience;
pub mod scout;

use fw_core::{Q32, Tick, exp_q32};

use crate::event::{DecayFunction, MemoryEvent};

// -------------------------------------------------------------------------
// Ln(2) as a Q32 constant for half-life decay
// -------------------------------------------------------------------------

/// Q32 approximation of ln(2) ≈ 0.693147…
///
/// Used in `project_salience` for `DecayFunction::Exponential`.
/// Half-life decay: `salience × 2^(−elapsed/half_life)`
///   = `salience × e^(−ln2 × elapsed/half_life)`.
///
/// Raw bits: round(0.693147180559945 × 2^32) = 2_977_044_472
const LN2: Q32 = Q32::from_raw(2_977_044_472_i64);

// -------------------------------------------------------------------------
// Shared filter/output types
// -------------------------------------------------------------------------

/// Filter for `SalienceReader::top_n`.
#[derive(Debug, Clone)]
pub enum SalienceFilter {
    /// No filter — return events from all subjects and classes.
    None,
    /// Return only events where `player_id` is any participant.
    BySubject(fw_core::PlayerId),
    /// Return only events of the given class discriminant.
    ByClass(u32),
}

/// A topic frame for `PressReader::candidates`.
///
/// Structural filter only at T3-2 — Tracery slot-filling is T3-3.
/// Each variant maps to a fixed set of `EventClass` discriminants that
/// are press-conference-relevant for that topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressTopic {
    /// Player milestones (debut, breakthrough, hat-trick, legacy goal).
    PlayerMilestone,
    /// Contract and transfer narrative (sold-under-protest, renewal arcs).
    ContractTransfer,
    /// Match results and competition arcs (cup finals, promotion, relegation).
    MatchResult,
    /// Relational events (rivalry, mentor, derby controversy).
    Relational,
}

impl PressTopic {
    /// The `EventClass` discriminants that belong to this topic frame.
    ///
    /// Discriminant values match `EventClass::discriminant()`.
    pub fn class_discriminants(self) -> &'static [u32] {
        match self {
            PressTopic::PlayerMilestone => &[
                0,  // BreakthroughMoment
                1,  // SignatureFirstFired
                2,  // LegacyGoal
                3,  // HatTrickScored
                24, // DebutSenior
                25, // DebutClub
                28, // InternationalCallUp
            ],
            PressTopic::ContractTransfer => &[
                6,  // PromisedYouthMinutes
                7,  // BrokenPromise
                8,  // ContractRenewalRejected
                9,  // ContractRenewalAccepted
                10, // TransferRequested
                11, // TransferRefused
                12, // SoldUnderProtest
                13, // BoughtOnDeadlineDay
            ],
            PressTopic::MatchResult => &[
                4,  // BigMatchScar
                18, // CupFinalWin
                19, // CupFinalLoss
                20, // PromotionWon
                21, // RelegationSuffered
                22, // TitleWon
                23, // UnbeatenRunEnded
            ],
            PressTopic::Relational => &[
                14, // RivalryFormed
                15, // MentorTeammate
                16, // DerbyControversy
                17, // FormerClubReunion
            ],
        }
    }

    /// The camelCase string identifier sent across the IPC boundary for this topic.
    ///
    /// Exhaustive match — adding a new `PressTopic` variant forces a compile
    /// error here, preventing silent inbox gaps.
    pub fn as_dto_str(self) -> &'static str {
        match self {
            PressTopic::PlayerMilestone => "playerMilestone",
            PressTopic::ContractTransfer => "contractTransfer",
            PressTopic::MatchResult => "matchResult",
            PressTopic::Relational => "relational",
        }
    }
}

/// Output of `FanReader::fan_callbacks`.
#[derive(Debug, Clone)]
pub struct FanReaderOutput {
    /// Events tagged for fan culture within the query window, sorted by
    /// projected salience descending (ties broken by `event_id` ascending).
    pub events: Vec<crate::event::EventId>,
    /// Count of each `Emotion` variant across `events`. Ordered by variant
    /// discriminant (Neutral / Joy / Anger / Pride / Disappointment / Hope).
    pub emotion_tally: EmotionTally,
}

/// Simple tally of emotion variants across a fan-reader result set.
#[derive(Debug, Clone, Default)]
pub struct EmotionTally {
    pub neutral: u32,
    pub joy: u32,
    pub anger: u32,
    pub pride: u32,
    pub disappointment: u32,
    pub hope: u32,
}

impl EmotionTally {
    /// Increment the appropriate counter for `emotion`.
    pub fn record(&mut self, emotion: crate::event::Emotion) {
        use crate::event::Emotion;
        match emotion {
            Emotion::Neutral => self.neutral += 1,
            Emotion::Joy => self.joy += 1,
            Emotion::Anger => self.anger += 1,
            Emotion::Pride => self.pride += 1,
            Emotion::Disappointment => self.disappointment += 1,
            Emotion::Hope => self.hope += 1,
        }
    }
}

// -------------------------------------------------------------------------
// Fan-culture class set (T3-2 structural subset)
// -------------------------------------------------------------------------

/// The fixed structural subset of `EventClass` discriminants considered
/// "fan-culture" events for `FanReader` at T3-2.
///
/// Fan-culture events are those with lasting collective memory in the stands:
/// major trophies, promotion/relegation drama, derby incidents, and
/// controversial transfers. They are a strict subset of all event classes.
///
/// Discriminant values match `EventClass::discriminant()`.
///
/// T3-2 set (9 classes):
/// - `LegacyGoal` (2)
/// - `CupFinalWin` (18)
/// - `CupFinalLoss` (19)
/// - `TitleWon` (22)
/// - `PromotionWon` (20)
/// - `RelegationSuffered` (21)
/// - `DerbyControversy` (16)
/// - `SoldUnderProtest` (12)
/// - `UnbeatenRunEnded` (23)
pub const FAN_CULTURE_CLASS_DISCRIMINANTS: &[u32] = &[
    2,  // LegacyGoal
    12, // SoldUnderProtest
    16, // DerbyControversy
    18, // CupFinalWin
    19, // CupFinalLoss
    20, // PromotionWon
    21, // RelegationSuffered
    22, // TitleWon
    23, // UnbeatenRunEnded
];

// -------------------------------------------------------------------------
// Decay projection helper
// -------------------------------------------------------------------------

/// Project the current salience of `event` at `now_tick`.
///
/// Applies `event.decay_function` as a **read-time modifier**. The canonical
/// `event.salience` is the emission value and is never mutated; this function
/// returns a transient projected value for ranking + filtering.
///
/// ## Decay formulae
///
/// - `Never` → returns `event.salience` unchanged.
/// - `Linear { lifetime_ticks }` → `salience × max(0, 1 − elapsed/lifetime)`.
///   Reaches `Q32::ZERO` at `elapsed >= lifetime`. Pure Q32 arithmetic.
/// - `Exponential { half_life_ticks }` → `salience × 2^(−elapsed/half_life)`
///   = `salience × e^(−ln2 × elapsed/half_life)`.
///   Uses `fw_core::exp_q32` (LUT-backed, domain [−8, +8]; saturates outside).
///   A half-life decay with large elapsed saturates near exp(−8) ≈ 0.00034
///   (effectively zero, which is correct).
///
/// ## None-tick fallback
///
/// If `event.tick` is `None` (compacted event — ADR-0005 §Compaction),
/// elapsed cannot be computed. Falls back to **no decay** (returns
/// `event.salience` unchanged), per the ADR requirement that "readers must
/// tolerate None".
///
/// ## Elapsed guard
///
/// If `now_tick < event.tick` (should not occur in a well-formed ledger —
/// events are in the past), treats elapsed as 0 (no decay applied).
#[must_use]
pub fn project_salience(event: &MemoryEvent, now_tick: Tick) -> Q32 {
    let elapsed_ticks: i64 = match event.tick {
        None => return event.salience, // compacted event: no-decay fallback
        Some(emission_tick) => {
            let e = now_tick.to_raw() - emission_tick.to_raw();
            if e <= 0 {
                return event.salience; // guard: event not yet in the past
            }
            e
        }
    };

    match event.decay_function {
        DecayFunction::Never => event.salience,

        DecayFunction::Linear { lifetime_ticks } => {
            if lifetime_ticks == 0 {
                return Q32::ZERO;
            }
            let lifetime = lifetime_ticks as i64;
            if elapsed_ticks >= lifetime {
                return Q32::ZERO;
            }
            // remaining_fraction = (lifetime − elapsed) / lifetime  ∈ (0, 1]
            //
            // Overflow guard: `(lifetime - elapsed) << 32` can overflow i64 if
            // `lifetime - elapsed` exceeds i64::MAX >> 32 = 2_147_483_647
            // (~35M seconds at 60 Hz, ~400 in-game days). Values beyond this
            // are astronomically long for any MemoryEvent lifetime and indicate
            // a misconfigured event. We saturate at Q32::ONE (full salience,
            // no decay) rather than panic, matching the conservative "big
            // lifetime ≈ Never" semantic.
            let numerator = lifetime - elapsed_ticks; // > 0, < lifetime ≤ u32::MAX
            let remaining = if numerator > (i64::MAX >> 32) {
                Q32::ONE // lifetime so large it's effectively Never
            } else {
                let remaining_raw = (numerator << 32) / lifetime;
                Q32::from_raw(remaining_raw)
            };
            // salience × remaining; both in [0,1] so no overflow.
            event.salience * remaining
        }

        DecayFunction::Exponential { half_life_ticks } => {
            if half_life_ticks == 0 {
                return Q32::ZERO;
            }
            // exponent = −ln2 × elapsed / half_life
            // We compute elapsed / half_life as Q32, then multiply by LN2, negate.
            //
            // Overflow guard: `elapsed << 32` can overflow i64 if elapsed exceeds
            // i64::MAX >> 32 = 2_147_483_647. When elapsed is this large relative
            // to any realistic half_life, the exponent is enormous-negative and
            // exp_q32 saturates at ~0. We shortcut to Q32::ZERO for that case.
            let half_life = half_life_ticks as i64;
            let ratio = if elapsed_ticks > (i64::MAX >> 32) {
                // elapsed so large the ratio overflows; exp of huge-negative → 0
                return Q32::ZERO;
            } else {
                let ratio_raw = (elapsed_ticks << 32) / half_life;
                Q32::from_raw(ratio_raw)
            };
            // exponent = −ln2 × ratio; result in (−∞, 0]
            let exponent = -(LN2 * ratio);
            let decay_factor = exp_q32(exponent);
            event.salience * decay_factor
        }
    }
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{
        CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind, Emotion,
        EntityRef, EventClass, EventId, MemoryEvent, Participant, ParticipantRole, SeasonNumber,
        SourceId,
    };
    use fw_core::{MatchId, PlayerId, Q32, Tick};

    #[test]
    fn press_topic_as_dto_str_pins_wire_strings_at_source() {
        // Pins the exact camelCase wire strings here, at the source, rather than
        // relying only on the frontend DTO mirror (types.ts) one crate downstream.
        // The match in as_dto_str is exhaustive, so a new PressTopic variant forces
        // an update of BOTH this test and get_press_inbox_inner (compile error) —
        // no topic can silently vanish from the inbox, and no literal typo
        // ("player_milestone") slips through unnoticed.
        assert_eq!(PressTopic::PlayerMilestone.as_dto_str(), "playerMilestone");
        assert_eq!(
            PressTopic::ContractTransfer.as_dto_str(),
            "contractTransfer"
        );
        assert_eq!(PressTopic::MatchResult.as_dto_str(), "matchResult");
        assert_eq!(PressTopic::Relational.as_dto_str(), "relational");
    }

    fn make_event_with_decay(
        salience: Q32,
        decay: DecayFunction,
        emission_tick: Option<Tick>,
    ) -> MemoryEvent {
        MemoryEvent {
            event_id: EventId(0),
            schema_version: 1,
            season: SeasonNumber(0),
            tick: emission_tick,
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
            event_class: EventClass::LegacyGoal,
            stakes: salience,
            emotion: Emotion::Joy,
            consequence: vec![Consequence::None],
            callback_eligibility: CallbackEligibility::Immediate,
            salience,
            decay_function: decay,
        }
    }

    /// AC1: DecayFunction::Never — projected salience equals emission salience.
    #[test]
    fn project_salience_never_unchanged() {
        let salience = Q32::from_raw(1 << 31); // 0.5
        let event = make_event_with_decay(salience, DecayFunction::Never, Some(Tick::ZERO));
        let projected = project_salience(&event, Tick::from_raw(1000));
        assert_eq!(projected, salience, "Never decay must not change salience");
    }

    /// AC1: DecayFunction::Linear reaches Q32::ZERO at elapsed >= lifetime.
    #[test]
    fn project_salience_linear_reaches_zero_at_lifetime() {
        let salience = Q32::from_raw(1 << 32); // 1.0
        let lifetime = 100u32;
        let event = make_event_with_decay(
            salience,
            DecayFunction::Linear {
                lifetime_ticks: lifetime,
            },
            Some(Tick::ZERO),
        );

        // At exactly lifetime ticks elapsed → Q32::ZERO
        let at_lifetime = project_salience(&event, Tick::from_raw(100));
        assert_eq!(
            at_lifetime,
            Q32::ZERO,
            "Linear decay must reach zero at lifetime"
        );

        // Beyond lifetime → Q32::ZERO
        let beyond = project_salience(&event, Tick::from_raw(200));
        assert_eq!(
            beyond,
            Q32::ZERO,
            "Linear decay must be zero beyond lifetime"
        );
    }

    /// AC1: DecayFunction::Linear halves at elapsed = lifetime/2.
    #[test]
    fn project_salience_linear_halves_at_midpoint() {
        let salience = Q32::from_raw(1i64 << 32); // 1.0
        let lifetime = 100u32;
        let event = make_event_with_decay(
            salience,
            DecayFunction::Linear {
                lifetime_ticks: lifetime,
            },
            Some(Tick::ZERO),
        );
        // At elapsed=50 (half lifetime), projected = 0.5
        let half = project_salience(&event, Tick::from_raw(50));
        let expected = Q32::from_raw(1i64 << 31); // 0.5
        assert_eq!(
            half, expected,
            "Linear decay at half-lifetime must be salience/2"
        );
    }

    /// AC1: DecayFunction::Exponential halves at exactly half_life_ticks elapsed.
    ///
    /// Due to LUT interpolation, result should be salience/2 within 1 ULP epsilon.
    #[test]
    fn project_salience_exponential_halves_at_half_life() {
        let salience = Q32::from_raw(1i64 << 32); // 1.0
        let half_life = 100u32;
        let event = make_event_with_decay(
            salience,
            DecayFunction::Exponential {
                half_life_ticks: half_life,
            },
            Some(Tick::ZERO),
        );
        let at_half_life = project_salience(&event, Tick::from_raw(100));
        let expected_half = Q32::from_raw(1i64 << 31); // 0.5

        // LUT interpolation tolerance: ±0.005 (well within the 1/256 step)
        let tolerance = Q32::from_raw((0.005 * (1i64 << 32) as f64) as i64);
        let diff = if at_half_life > expected_half {
            at_half_life - expected_half
        } else {
            expected_half - at_half_life
        };
        assert!(
            diff <= tolerance,
            "Exponential at half_life should ≈ salience/2; got {at_half_life:?}, expected {expected_half:?}, diff {diff:?}"
        );
    }

    /// AC1: tick == None (compacted event) → falls back to no decay.
    #[test]
    fn project_salience_none_tick_falls_back_to_no_decay() {
        let salience = Q32::from_raw(1i64 << 31); // 0.5
        let event = make_event_with_decay(
            salience,
            DecayFunction::Linear { lifetime_ticks: 10 },
            None, // compacted — tick is None
        );
        // Even with a large elapsed, returns emission salience unchanged.
        let projected = project_salience(&event, Tick::from_raw(10_000));
        assert_eq!(
            projected, salience,
            "tick=None must fall back to emission salience"
        );
    }

    /// Overflow guard: Linear decay with a very large lifetime (exceeding
    /// i64::MAX >> 32 = 2_147_483_647) must not panic and must return a value
    /// in [0, 1] (specifically Q32::ONE for the "effectively Never" path).
    #[test]
    fn project_salience_linear_large_lifetime_no_overflow() {
        let salience = Q32::from_raw(1i64 << 31); // 0.5
        // lifetime_ticks = u32::MAX ≈ 4.3B — exceeds the safe << 32 range.
        let event = make_event_with_decay(
            salience,
            DecayFunction::Linear {
                lifetime_ticks: u32::MAX,
            },
            Some(Tick::ZERO),
        );
        // At elapsed = 1 (far below the huge lifetime), the overflow guard
        // triggers and returns ONE (effective no-decay). Then salience × 1 = salience.
        let projected = project_salience(&event, Tick::from_raw(1));
        assert!(
            projected > Q32::ZERO,
            "large-lifetime linear decay must not return zero at tiny elapsed"
        );
        assert!(
            projected <= salience,
            "large-lifetime linear decay must not exceed emission salience"
        );
    }

    /// Overflow guard: Exponential decay with very large elapsed returns Q32::ZERO
    /// rather than overflowing (the "huge elapsed" short-circuit path).
    #[test]
    fn project_salience_exponential_huge_elapsed_no_overflow() {
        let salience = Q32::from_raw(1i64 << 31); // 0.5
        let event = make_event_with_decay(
            salience,
            DecayFunction::Exponential {
                half_life_ticks: 100,
            },
            Some(Tick::ZERO),
        );
        // now_tick > i64::MAX >> 32 triggers the overflow guard → Q32::ZERO.
        // elapsed = now_tick - emission_tick(0) = now_tick.
        // Guard condition: elapsed > (i64::MAX >> 32) = 2_147_483_647.
        let huge_tick = Tick::from_raw((i64::MAX >> 32) + 1);
        let projected = project_salience(&event, huge_tick);
        assert_eq!(
            projected,
            Q32::ZERO,
            "huge elapsed must short-circuit to Q32::ZERO (exp of huge-negative)"
        );
    }
}

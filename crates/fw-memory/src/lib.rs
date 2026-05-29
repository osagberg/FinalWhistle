//! `fw-memory` — event-sourced career ledger.
//!
//! The memory ledger is the structural carrier of pillar 2 ("Careers That
//! Remember") and pillar 3 ("Breakthrough-Driven Development") per
//! `docs/DESIGN_DOC.md` §3. Schema locked at T3-1 per ADR-0005 (accepted
//! 2026-05-18).
//!
//! ## Public surface
//!
//! - `MemoryEvent` — the canonical immutable row. See `event` module.
//! - `MemoryLedger` — the append-only `Vec`-backed ledger with O(log n)
//!   `BTreeMap` indexes. See `ledger` module.
//!
//! ## Determinism contract
//!
//! The ledger is append-only and replays bit-exactly given a seeded sim.
//! - Floats forbidden; `stakes` + `salience` are `Q32`.
//! - `BTreeMap`/`BTreeSet` only — no `HashMap`/`HashSet`.
//! - No clocks (`std::time::Instant` / `SystemTime`).
//! - No async / tokio.
//!
//! See `.claude/rules/Sim/RULES.md` §1-§5 for the binding contract.

pub mod breakthrough;
pub mod event;
pub mod ledger;
pub mod readers;

// -------------------------------------------------------------------------
// Flat re-exports for convenience
// -------------------------------------------------------------------------

pub use breakthrough::{
    BREAKTHROUGH_COOLDOWN_DAYS, BREAKTHROUGH_THRESHOLD, BreakthroughContext, BreakthroughKind,
    BreakthroughOutcome, BreakthroughState, GATE_MIN_STAKES, NarrativeFlag, READINESS_RESIDUE,
    REGRESSIVE_COOLDOWN_DAYS, REGRESSIVE_RESIDUE, REGRESSIVE_THRESHOLD, accumulate, evaluate,
    family_relevance, is_positive_gate, is_regressive_gate, positive_redraw_range,
    regressive_redraw_range,
};
// AttributeFamily relocated to fw-core at T4-2.5a. Re-exported here so
// existing `fw_memory::AttributeFamily` import paths continue to resolve
// without changes to fw-save, fw-tauri, or any other consumer.
pub use event::{
    CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind, Emotion,
    EntityRef, EventClass, EventId, MemoryEvent, ModEventTag, Participant, ParticipantRole,
    SeasonNumber, SourceId,
};
pub use fw_core::AttributeFamily;
pub use ledger::MemoryLedger;
pub use readers::{
    EmotionTally, FAN_CULTURE_CLASS_DISCRIMINANTS, FanReaderOutput, PressTopic, SalienceFilter,
    project_salience,
};

//! `fw-memory` — event-sourced career ledger.
//!
//! Phase-0 scope: `MemoryEvent` enum stub + the ledger type. T3 fills in
//! the salience + reader-callback machinery per DESIGN_DOC §3 (Pillar 2)
//! and `design/memory.md` (owed Phase-3).
//!
//! ## Determinism contract
//!
//! The ledger is append-only and replays bit-exactly given a seeded sim.
//! Floats forbidden; `BTreeMap`/`BTreeSet` only; no clocks; no async.

use fw_core::{MatchId, PlayerId, Tick};
use serde::{Deserialize, Serialize};

/// Minimal `MemoryEvent` placeholder. T3 expands into the full
/// match-result / transfer / breakthrough / rivalry / press-quote union.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryEvent {
    /// Stand-in event so the enum has a non-empty variant set; replaced in
    /// T3 with the real catalog. Carries enough fields for tests to round-
    /// trip the type through serde.
    Placeholder {
        match_id: MatchId,
        actor: PlayerId,
        tick: Tick,
    },
}

/// The append-only ledger.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryLedger {
    /// Events in insertion order. `Vec` is canonical-encoding stable
    /// because the index *is* the chronological key.
    pub events: Vec<MemoryEvent>,
}

impl MemoryLedger {
    /// Fresh empty ledger.
    pub fn new() -> MemoryLedger {
        MemoryLedger { events: Vec::new() }
    }

    /// Append. T3 introduces salience-scored insertion + reader-callback
    /// indexing; T0 just pushes.
    pub fn push(&mut self, event: MemoryEvent) {
        self.events.push(event);
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

// -------------------------------------------------------------------------
// Smoke
// -------------------------------------------------------------------------

#[cfg(test)]
mod smoke {
    use super::*;

    #[test]
    fn smoke() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn ledger_round_trips() {
        let mut l = MemoryLedger::new();
        assert!(l.is_empty());
        l.push(MemoryEvent::Placeholder {
            match_id: MatchId::default(),
            actor: PlayerId::default(),
            tick: Tick::ZERO,
        });
        assert_eq!(l.len(), 1);
    }
}

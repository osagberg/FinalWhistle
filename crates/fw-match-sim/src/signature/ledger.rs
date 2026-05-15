//! Local `MemoryEvent` stub — T1-2b-iv pattern (mirrors `TacticEvent`).
//!
//! T1-4 reconciles this with the full `fw-memory::MemoryEvent` enum when
//! that crate gains the real ledger surface. For now this stub carries only
//! the `SignatureFirstFired` event emitted by the signature dispatcher.
//!
//! The event is appended to `MatchState.signature_memory_events: Vec<MemoryEvent>`
//! (transient — T3-1 wires real ledger persistence). Events order deterministically
//! because the dispatcher iterates slots 0..22 in fixed order at each tick;
//! emissions within a tick are appended in slot order.

use fw_content::SignatureId;
use fw_core::Tick;
use serde::{Deserialize, Serialize};

use crate::PlayerSlot;

/// Local stub — T1-4 reconciles with `fw-memory::MemoryEvent`.
///
/// Emission: when a signature fires for the first time in a match for a given
/// `(player_slot, signature_id)` pair, the dispatcher appends this event.
/// "First time" means the `signature_first_fired_seen` BTreeSet did NOT already
/// contain the pair.
///
/// T1-4 additions (anticipated): `Goal { scorer_slot, assisted_by, tick }`,
/// `Shot { shooter_slot, on_target, tick }`, `KickOff { tick }`,
/// `FullTime { home_score, away_score }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryEvent {
    /// A signature fired for the first time this match for this player.
    ///
    /// Determinism note: `SignatureId` implements `Ord` (it's a newtype over
    /// `String`); the BTreeSet key is `(PlayerSlot, SignatureId)` ensuring
    /// the "first-fired" check is canonical across platforms.
    SignatureFirstFired {
        player_slot: PlayerSlot,
        signature_id: SignatureId,
        tick: Tick,
    },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_content::SignatureId;
    use fw_core::Tick;

    #[test]
    fn signature_first_fired_constructs() {
        let id = SignatureId::try_new("fwh.core:signature.no-op-stub").unwrap();
        let ev = MemoryEvent::SignatureFirstFired {
            player_slot: 7,
            signature_id: id.clone(),
            tick: Tick::from_raw(100),
        };
        // Round-trip via Debug (basic structural check).
        let dbg = format!("{ev:?}");
        assert!(dbg.contains("SignatureFirstFired"));
        assert!(dbg.contains("no-op-stub"));
    }

    #[test]
    fn two_different_signatures_produce_distinct_events() {
        let id_a = SignatureId::try_new("fwh.core:signature.long-range-strike").unwrap();
        let id_b = SignatureId::try_new("fwh.core:signature.body-shield-pressure").unwrap();
        let ev_a = MemoryEvent::SignatureFirstFired {
            player_slot: 5,
            signature_id: id_a,
            tick: Tick::from_raw(42),
        };
        let ev_b = MemoryEvent::SignatureFirstFired {
            player_slot: 5,
            signature_id: id_b,
            tick: Tick::from_raw(42),
        };
        assert_ne!(ev_a, ev_b);
    }
}

//! Entity-ID newtypes — `PlayerId`, `ClubId`, `MatchId`.
//!
//! Backed by `slotmap` keys. `slotmap`'s generation-counted IDs let the sim
//! retire and recycle entity slots without ABA bugs: a dropped `PlayerId`
//! does not silently alias a future newly-allocated player even if the
//! underlying slot is reused. This matters more in `fw-memory` (where old
//! `MemoryEvent`s reference long-gone players) than in match-sim, but the
//! single ID vocabulary keeps the surface uniform.
//!
//! Equality + hash + Ord are derived from the slotmap-key wire layout, so
//! `BTreeMap<PlayerId, _>` iteration order is deterministic — a hard
//! requirement of the determinism gate.

use slotmap::new_key_type;

new_key_type! {
    /// A player's stable identity for the duration of a save. Allocated by
    /// the content compiler (procedural players) and the career system
    /// (in-career newgens). Survives serialization round-trip via
    /// slotmap's `serde` feature.
    pub struct PlayerId;

    /// A club's stable identity for the duration of a save.
    pub struct ClubId;

    /// A match's stable identity. One per fixture played. Used as the
    /// `match_seed`'s display companion (the seed is the determinism input;
    /// the `MatchId` is the bookkeeping handle).
    pub struct MatchId;
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    #[test]
    fn slotmap_round_trip_player_id() {
        // Sanity-check that the new_key_type! macro produced a working key.
        let mut sm: SlotMap<PlayerId, &'static str> = SlotMap::with_key();
        let k = sm.insert("Striker McStrikerface");
        assert_eq!(sm.get(k), Some(&"Striker McStrikerface"));
    }

    #[test]
    fn distinct_id_types_do_not_alias() {
        // The type system must prevent ClubId from being used where PlayerId
        // is expected. This is a compile-time test masquerading as a
        // runtime test — the assertion is "this file compiles."
        fn _accepts_player(_: PlayerId) {}
        fn _accepts_club(_: ClubId) {}
        // Uncomment to verify the negative case:
        // _accepts_player(ClubId::default()); // expected: type mismatch
    }
}

//! Entity-ID newtypes — durable `u32`-backed IDs for save / memory ledger / content.
//!
//! These IDs survive serialization. They're allocated:
//! - At content-bake time (procedural players in the baked corpus)
//! - At career-init time (newgens added during a save)
//! - At match-init time (`MatchId` is per-fixture)
//!
//! The runtime per-match slot (0..=21 within a single live match) is
//! `fw_match_sim::PlayerSlot`, a separate `u8`-backed type. The split is
//! deliberate per Codex pre-T0 audit (open question A):
//!
//! - `PlayerId` is **durable** — content-pack-qualified at the content
//!   layer, raw `u32` in canonical state, survives save→load round-trip.
//! - `PlayerSlot` is **ephemeral** — 0..=21 per live match, regenerated
//!   each match, never serialized as a player's identity.
//!
//! `BTreeMap<PlayerId, _>` iteration order is deterministic — a
//! requirement of the determinism gate (`Sim/RULES.md` §2).
//!
//! ## History
//!
//! Earlier scaffold used `slotmap::new_key_type!` for these. That gave
//! generation-counted IDs but tied them to a runtime SlotMap instance
//! and produced non-stable serialization across save/load. The Codex
//! audit caught the conflict with the durable-ID requirement in
//! `docs/CONTENT_PIPELINE.md` §6. Switched to `u32` newtypes here.

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! id_type {
    ($Name:ident, $stringified:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Debug,
            Clone,
            Copy,
            Default,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            Deserialize,
        )]
        #[serde(transparent)]
        pub struct $Name(pub u32);

        impl $Name {
            /// Construct from a raw `u32`. Use the content pipeline's
            /// allocator (`fw-content` / career system) rather than minting
            /// IDs by hand — collisions break referential integrity.
            #[inline]
            pub const fn new(raw: u32) -> Self {
                Self(raw)
            }

            /// Underlying raw `u32`. Stable for canonical state, fixture
            /// authoring, and content-pack ID renderers.
            #[inline]
            pub const fn raw(self) -> u32 {
                self.0
            }
        }

        impl fmt::Display for $Name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!($stringified, "#{}"), self.0)
            }
        }
    };
}

id_type!(
    PlayerId,
    "PlayerId",
    "A player's durable identity for the duration of a save. Allocated by the content compiler (procedural players in the baked corpus) or the career system (newgens). Survives save→load round-trip as a raw `u32`."
);
id_type!(
    ClubId,
    "ClubId",
    "A club's durable identity for the duration of a save. Allocated by the content compiler. Survives save→load round-trip as a raw `u32`."
);
id_type!(
    MatchId,
    "MatchId",
    "A match's durable identity. One per fixture played. Used as a bookkeeping handle (separate from `Seed`, which is the determinism input). Survives save→load round-trip as a raw `u32`."
);

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_construction_round_trips_raw() {
        let p = PlayerId::new(42);
        assert_eq!(p.raw(), 42);
        assert_eq!(p, PlayerId(42));
    }

    #[test]
    fn distinct_id_types_do_not_alias() {
        // The type system must prevent ClubId from being used where
        // PlayerId is expected. Compile-time test masquerading as runtime.
        fn _accepts_player(_: PlayerId) {}
        fn _accepts_club(_: ClubId) {}
        // Uncomment to verify the negative case:
        // _accepts_player(ClubId::default()); // expected: type mismatch
    }

    #[test]
    fn id_default_is_zero() {
        // Default is 0 — a sentinel "not yet allocated" value. Real IDs
        // start at 1 from the allocator.
        assert_eq!(PlayerId::default(), PlayerId(0));
        assert_eq!(ClubId::default(), ClubId(0));
        assert_eq!(MatchId::default(), MatchId(0));
    }

    #[test]
    fn id_ordering_is_stable() {
        let a = PlayerId::new(1);
        let b = PlayerId::new(2);
        let c = PlayerId::new(3);
        let mut v = vec![c, a, b];
        v.sort();
        assert_eq!(v, vec![a, b, c]);
    }

    #[test]
    fn id_display_renders_typed_form() {
        assert_eq!(PlayerId::new(42).to_string(), "PlayerId#42");
        assert_eq!(ClubId::new(7).to_string(), "ClubId#7");
        assert_eq!(MatchId::new(2026).to_string(), "MatchId#2026");
    }

    #[test]
    fn id_serializes_as_bare_u32() {
        // serde(transparent) means the wire form is the inner u32 — both
        // for forward-compat with content-pack-qualified IDs (where the
        // pack prefix lives in a separate field) and so Bincode encodes
        // exactly 4 bytes per ID.
        let p = PlayerId::new(0xDEAD_BEEF);
        let json = serde_json::to_string(&p).expect("PlayerId serializes");
        assert_eq!(json, "3735928559"); // 0xDEAD_BEEF as decimal
    }
}

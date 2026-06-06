//! Signature system module — T1-2b-iv.
//!
//! ## Module layout (directory module, sibling to `bt/`)
//!
//! - `mod.rs` (this file): `SignatureFiring` type + public re-exports.
//! - `triggers.rs`: 3 trigger predicate functions + binding table.
//! - `dispatcher.rs`: `evaluate_signatures` — cooldown check + softmax sample.
//! - `bias_apply.rs`: `apply_signature_bias` — composes `SimBiasSnapshot` into utility.
//!
//! Note: `ledger.rs` was deleted at T1-4a. The local `MemoryEvent::SignatureFirstFired`
//! stub it contained has been replaced by `fw_content::MatchEvent::SignatureFirstFired`,
//! which is pushed to `MatchState.match_events` (canonical persistent event stream)
//! rather than the removed `signature_memory_events` transient scratch buffer.
//!
//! ## Design choices (documented per task-spec §"Design choices to make")
//!
//! **Module organization choice:** A (directory module `signature/`) was chosen
//! over B (single flat file). The 4 distinct concerns (triggers / dispatcher /
//! bias_apply / ledger) have separate testing surfaces; a directory keeps each
//! testable in isolation without a 600-LoC monolith.
//!
//! **`SignatureFiring` shape:** `{ id: SignatureId, start_tick: Tick,
//! duration_ticks: u32 }`. Fields are `pub(crate)`. Duration default: 60 ticks
//! (= 1 second at 60 Hz). Documents as skeleton tier; T2-4 reads duration from
//! the `SignatureDefinition` parameter block.
//!
//! **`signature_firing` storage:** fixed array `[Option<SignatureFiring>; 22]`
//! (mirrors `interrupt_cooldown_until: [Tick; 22]` pattern). Encodes deterministically
//! in slot order without sparse-collection overhead.
//!
//! **Bias composition order:** personality-bias THEN signature-bias. The
//! on_ball/off_ball utility functions call personality bias first, then pass the
//! result to `apply_signature_bias`. Order is documented in each function body.
//!
//! **`evaluate_signatures` return type:** `Option<(SignatureId, &SignatureDefinition)>`
//! — the caller stashes `SignatureId` in `signature_firing` AND reads
//! `SimBiasSnapshot` from the `SignatureDefinition` reference in the same call,
//! avoiding a second map lookup.
//!
//! ## Determinism
//!
//! No floats, no HashMap, no async, no clocks. `signature_cooldowns` is a
//! `BTreeMap` keyed by `(PlayerSlot, SignatureId)` — both are `Ord` via derive.
//! `signature_firing` is a fixed-length array (slot-indexed, deterministic).
//! `signature_first_fired_seen` is a `BTreeSet` — set membership is deterministic.

pub mod bias_apply;
pub mod dispatcher;
pub mod triggers;

use fw_content::SignatureId;
use fw_core::Tick;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// SignatureFiring — per-player active-signature record
// ---------------------------------------------------------------------------

/// Records that a signature is currently "in flight" for a player.
///
/// Stored in `MatchState.signature_firing[slot]` as `Option<SignatureFiring>`.
/// While `Some`, the bias snapshot for this signature is applied to the player's
/// utility scoring each decision tick.
///
/// `duration_ticks` default: 60 ticks (1 second at 60 Hz). Skeleton tier uses
/// this constant; T2-4 reads it from the `SignatureDefinition` parameter block.
///
/// Field order is stable for serde — do NOT reorder (affects save forward-compat).
/// Fields are `pub(crate)` to prevent external code from bypassing the dispatch
/// path. External callers use `id()`, `start_tick()`, `duration_ticks()` accessors
/// and the `SignatureFiring::new` constructor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureFiring {
    /// The signature currently in flight.
    pub(crate) id: SignatureId,
    /// Tick at which the signature fired.
    pub(crate) start_tick: Tick,
    /// How many ticks the bias window lasts. Default 60.
    pub(crate) duration_ticks: u32,
}

/// Default duration for a signature firing window (1 second at 60 Hz).
pub const DEFAULT_FIRING_DURATION_TICKS: u32 = 60;

impl SignatureFiring {
    /// Construct a `SignatureFiring`. The only public constructor — enforces
    /// that external callers cannot bypass the dispatch path.
    #[must_use]
    pub fn new(id: SignatureId, start_tick: Tick, duration_ticks: u32) -> SignatureFiring {
        SignatureFiring {
            id,
            start_tick,
            duration_ticks,
        }
    }

    /// The signature currently in flight.
    #[must_use]
    pub fn id(&self) -> &SignatureId {
        &self.id
    }

    /// Tick at which the signature fired.
    #[must_use]
    pub fn start_tick(&self) -> Tick {
        self.start_tick
    }

    /// How many ticks the bias window lasts.
    #[must_use]
    pub fn duration_ticks(&self) -> u32 {
        self.duration_ticks
    }

    /// Is this firing still active at `current_tick`?
    ///
    /// Returns `false` if `current_tick < start_tick` — retroactive firings
    /// (where start_tick is in the future) are considered inactive.
    #[must_use]
    pub fn is_active(&self, current_tick: Tick) -> bool {
        // Guard against retroactive firings: a firing can only be active
        // once the simulation has reached its start tick.
        if current_tick < self.start_tick {
            return false;
        }
        // T1-23 (post-Codex Finding #1): `Tick::checked_add_ticks` replaces
        // raw `self.start_tick.to_raw() + self.duration_ticks as i64`. Same
        // arithmetic at realistic tick range; the helper funnels overflow
        // through the §11 panic-on-overflow policy. Returning Tick (not raw
        // i64) lets the comparison stay typed.
        let end_tick = self.start_tick.checked_add_ticks(self.duration_ticks);
        current_tick < end_tick
    }
}

// ---------------------------------------------------------------------------
// Re-exports for convenience at the crate root
// ---------------------------------------------------------------------------

pub use bias_apply::{BiasConsideration, apply_signature_bias};
pub use dispatcher::evaluate_signatures;
pub use triggers::{
    SIGNATURE_SETTLE_TICKS, TriggerFn, build_trigger_table, signature_executes_now,
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_content::SignatureId;
    use fw_core::Tick;

    #[test]
    fn signature_firing_is_active_within_window() {
        let id = SignatureId::try_new("fwh.core:signature.no-op-stub").unwrap();
        let firing = SignatureFiring::new(id, Tick::from_raw(100), 60);
        assert!(firing.is_active(Tick::from_raw(100)));
        assert!(firing.is_active(Tick::from_raw(159)));
        assert!(!firing.is_active(Tick::from_raw(160)));
        assert!(!firing.is_active(Tick::from_raw(200)));
    }

    #[test]
    fn signature_firing_not_active_before_start() {
        let id = SignatureId::try_new("fwh.core:signature.no-op-stub").unwrap();
        let firing = SignatureFiring::new(id, Tick::from_raw(100), 60);
        // P1-5: current_tick < start_tick must return false (retroactive guard).
        assert!(
            !firing.is_active(Tick::from_raw(50)),
            "tick before start_tick should be inactive (retroactive guard)"
        );
        assert!(
            !firing.is_active(Tick::from_raw(99)),
            "tick=99 < start_tick=100 should be inactive"
        );
        assert!(
            firing.is_active(Tick::from_raw(100)),
            "at window start: active"
        );
        assert!(
            !firing.is_active(Tick::from_raw(161)),
            "after window end: not active"
        );
    }

    #[test]
    fn default_duration_is_60() {
        assert_eq!(DEFAULT_FIRING_DURATION_TICKS, 60);
    }
}

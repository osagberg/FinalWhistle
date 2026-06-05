//! In-match event stream — T1-4a.
//!
//! `MatchEvent` is the Phase-1 in-match event stream enum. It carries the
//! six event classes emitted during a match: `KickOff`, `FullTime`, `Goal`,
//! `Shot`, `Pass`, and `SignatureFirstFired`.
//!
//! ## Design choices
//!
//! **Lives in `fw-content`, not `fw-match-sim`:** `fw-match-sim` depends on
//! `fw-content` (for `SignatureId`, `SignatureDefinition`, etc.). If
//! `MatchEvent` lived in `fw-match-sim`, `fw-content`'s commentary renderer
//! (T1-4b) would need to depend on `fw-match-sim` — a cycle. Moving it here
//! breaks the cycle while keeping both the emission site (`fw-match-sim`) and
//! the renderer (`fw-content::commentary`) able to import `MatchEvent`.
//!
//! **No `Ord` derive:** the Vec preserves insertion order (which is
//! chronological by construction — events are pushed at the tick they fire).
//! Sorting is not needed and would mask ordering bugs.
//!
//! **`PlayerSlot` from `fw-core`:** moved at T1-4a chunk 1 specifically to
//! allow this crate to use it without creating a dep on `fw-match-sim`.
//!
//! **`PassKind` sub-enum:** lumps all pass-class intents (`AttemptPassShort`,
//! `AttemptPassLong`, `Cross`, `LayOff`) into a single `MatchEvent::Pass`
//! with a `kind` field. T1-4b commentary templates can branch on `kind`.
//!
//! **Canonical encoding:** `MatchEvent` is in `match_events: Vec<MatchEvent>`
//! which IS in canonical state (encoder VERSION bumped 6→7 at chunk 3).
//! Discriminant assignments are stable — do NOT reorder variants.
//!
//! ## Discriminant table (stable; do not reorder)
//!
//! | Discriminant | Variant |
//! |---|---|
//! | 0 | `KickOff` |
//! | 1 | `FullTime` |
//! | 2 | `Goal` |
//! | 3 | `Shot` |
//! | 4 | `Pass` |
//! | 5 | `SignatureFirstFired` |
//! | 6 | `Offside` |
//! | 7 | `PassIncomplete` |
//!
//! ## `PassKind` discriminants (stable; do not reorder)
//!
//! | Discriminant | Variant |
//! |---|---|
//! | 0 | `Short` |
//! | 1 | `Long` |
//! | 2 | `Cross` |
//! | 3 | `LayOff` |

use crate::SignatureId;
use fw_core::{PlayerSlot, Q32, Tick};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// PassKind sub-enum
// ---------------------------------------------------------------------------

/// The class of a pass event. Maps from `PlayerIntent` pass variants.
///
/// Discriminant ordering is stable — do NOT reorder (encoder bumps VERSION
/// on any discriminant change). See module-level table.
///
/// `#[repr(u8)]` pins the discriminant layout and powers `canonical_tag()`.
/// The canonical encoder uses `kind.canonical_tag()` as the single source of
/// truth for the wire-format byte — no hand-rolled match needed at call sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PassKind {
    /// Short ground pass (≤ ~15m target distance, in the channel).
    Short = 0,
    /// Long aerial or driven pass (> ~25m).
    Long = 1,
    /// Wide cross into the box.
    Cross = 2,
    /// One-touch lay-off to a near-runner.
    LayOff = 3,
}

impl PassKind {
    /// Return the stable canonical encoding tag for this pass kind.
    ///
    /// This is the **single source of truth** for the `kind u8` byte the
    /// canonical encoder writes for both `Pass` and `PassIncomplete` events.
    /// Because `PassKind` is `#[repr(u8)]` with explicit discriminants, casting
    /// `self as u8` is sound and byte-identical to the prior hand-rolled `match`
    /// in `canonical.rs`. The cross-crate test in
    /// `fw-content/tests/event_discriminant_test.rs` pins all four values.
    ///
    /// Values are stable forever — changing them is an ADR-0012 trigger #1
    /// canonical-hash-invalidating event.
    #[must_use]
    pub fn canonical_tag(self) -> u8 {
        self as u8
    }
}

// ---------------------------------------------------------------------------
// MatchEvent enum
// ---------------------------------------------------------------------------

/// The in-match event stream. Emitted to `MatchState.match_events: Vec<MatchEvent>`
/// as the sim advances. Entries are in chronological (tick-ascending) order
/// by construction — events are pushed at the tick they fire; the Vec is
/// never sorted post-construction.
///
/// ## Determinism
///
/// No floats. Positional fields (`target_x`, `target_y`) use `Q32`.
/// `MatchEvent` must be `Serialize + Deserialize` because it lives in
/// canonical state.
///
/// Do NOT derive `Ord` — ordering is insertion-order (chronological), not
/// lexicographic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchEvent {
    /// The match kicked off.
    ///
    /// Emitted at `tick = 0` (from `MatchState::initial`) for the first
    /// half kick-off, or at the half-time restart tick for the second half.
    /// T1 only models one half; second-half support is T2+ territory.
    KickOff {
        /// The tick at which the kick-off took place.
        tick: Tick,
        /// `true` if this is the second-half kick-off.
        is_second_half: bool,
    },

    /// The match ended. Emitted at the last tick of the match.
    FullTime {
        /// The tick at which the match ended.
        tick: Tick,
        /// Home team's final score.
        home_score: u16,
        /// Away team's final score.
        away_score: u16,
    },

    /// A goal was scored.
    ///
    /// Emitted at the tick a `TacticEvent::Goal` transition fires. The score
    /// fields reflect the scoreline AFTER the goal (consistent with how
    /// commentary renders it: "Jones scores to make it 1-0").
    Goal {
        /// Which player scored (slot index).
        scorer_slot: PlayerSlot,
        /// The tick at which the goal was scored.
        tick: Tick,
        /// Home team score after this goal.
        score_home_after: u16,
        /// Away team score after this goal.
        score_away_after: u16,
    },

    /// A shot was attempted.
    ///
    /// Emitted in `dispatch_tick` when `PlayerIntent::AttemptShot` is
    /// selected. `target_x` / `target_y` are the aimed-at point on the
    /// goal-line, in Q32 pitch coordinates (−52.5..52.5 m, −34..34 m).
    Shot {
        /// Which player attempted the shot (slot index).
        shooter_slot: PlayerSlot,
        /// The tick at which the shot was attempted.
        tick: Tick,
        /// Horizontal aim point (goal-line X coordinate, Q32 pitch coords).
        target_x: Q32,
        /// Vertical aim point (goal-line Y coordinate, Q32 pitch coords).
        target_y: Q32,
        /// Whether the shot was on target (heading between the posts).
        ///
        /// T1: derived from `target_y` being within ±`GOAL_HALF_WIDTH_M`.
        /// T2+: real contest physics.
        on_target: bool,
    },

    /// A pass was attempted.
    ///
    /// Emitted for `AttemptPassShort`, `AttemptPassLong`, `Cross`, and
    /// `LayOff` intents. The `kind` field distinguishes the sub-class.
    Pass {
        /// Which player made the pass (slot index).
        from_slot: PlayerSlot,
        /// Intended recipient (slot index). T1: the nearest teammate in the
        /// direction of `target_x/target_y` — approximate, corrected at T2.
        to_slot: PlayerSlot,
        /// The tick at which the pass was attempted.
        tick: Tick,
        /// Sub-class of pass.
        kind: PassKind,
        /// Whether the pass was completed. T1: always `true` (no contest
        /// model yet; flagged for T1-9 / T2 territory).
        completed: bool,
    },

    /// A player's signature move fired for the first time this match.
    ///
    /// Emitted at most once per `(player_slot, signature_id)` pair per match.
    /// Replaces the `signature::ledger::MemoryEvent::SignatureFirstFired`
    /// local stub that existed in T1-2b-iv (chunk 5 deletes that stub).
    SignatureFirstFired {
        /// Which player fired the signature.
        player_slot: PlayerSlot,
        /// Which signature fired.
        signature_id: SignatureId,
        /// The tick at which the signature fired.
        tick: Tick,
    },

    /// An offside infringement was detected at pass-launch (FUN-TS2b).
    ///
    /// Checked in `apply_intent` at the moment a forward pass is played:
    /// if the intended receiver is beyond both the ball position AND the
    /// defending team's second-rearmost defender (incl. GK) at the instant
    /// the pass is made, an offside flag is emitted.
    ///
    /// Per Laws of the Game (IFAB 2025/26, Law 11):
    /// - Equal line = ONSIDE (not offside if level with last defender).
    /// - Backward/square passes (toward own goal) never offside.
    /// - No offside from throw-ins, corners, or goal-kicks.
    ///
    /// Discriminant 6 (append-only — do NOT reorder).
    Offside {
        /// The offending player (receiver who was in an offside position).
        offending_slot: PlayerSlot,
        /// The tick at which the pass was launched (and the offside detected).
        tick: Tick,
    },

    /// A pass attempt that failed — the ball was lost or intercepted (FUN-CB1).
    ///
    /// Emitted immediately after `Pass { completed: false }` in `apply_intent`
    /// when the completion draw fails. Spawns a loose ball dropped 40% of the
    /// way from passer to receiver (forward pass) or 20% (backward/lateral).
    ///
    /// `possession` is set to `None` and `last_touched_by` to `from_slot` so
    /// the nearest-2 preempt policy can pick it up on the next tick.
    ///
    /// Discriminant 7 (append-only — do NOT reorder).
    PassIncomplete {
        /// Which player attempted (and failed) the pass.
        from_slot: PlayerSlot,
        /// Intended recipient (the receiver the passer was targeting).
        to_slot: PlayerSlot,
        /// The tick at which the pass was launched.
        tick: Tick,
        /// The class of pass that failed (Short / Long / Cross / LayOff).
        kind: PassKind,
    },
}

impl MatchEvent {
    /// Return the stable canonical discriminant byte for this event variant.
    ///
    /// This is the **single source of truth** for the discriminant byte the
    /// canonical encoder writes to the wire format. Both
    /// `MatchEventDiscriminant::from_event` (in `fw-content::commentary`) AND
    /// `encode_match_event` (in `fw-match-sim::canonical`) MUST agree with
    /// these values. The cross-crate test in
    /// `crates/fw-content/tests/event_discriminant_test.rs` pins all 6 values
    /// against a hardcoded table, catching any reordering regression.
    ///
    /// ## Discriminant table (stable; do NOT reorder `MatchEvent` variants)
    ///
    /// | Byte | Variant              |
    /// |------|----------------------|
    /// | 0    | `KickOff`            |
    /// | 1    | `FullTime`           |
    /// | 2    | `Goal`               |
    /// | 3    | `Shot`               |
    /// | 4    | `Pass`               |
    /// | 5    | `SignatureFirstFired` |
    ///
    /// Changing these values is a canonical-hash-invalidating event that
    /// requires an authorized ADR-0012 rebaseline.
    ///
    /// **Return type** (Codex T1-11 type-design P1 fix-pass): returns the
    /// typed [`MatchEventDiscriminant`] enum rather than a raw `u8`. The
    /// canonical encoder casts via `event.discriminant() as u8` at point
    /// of use (sound because of `#[repr(u8)]`); the commentary renderer's
    /// `MatchEventDiscriminant::from_event` becomes a trivial passthrough
    /// (`event.discriminant()`). Prior `u8` return forced a byte→enum→byte
    /// round-trip with `unreachable!()` panic landmine on unknown bytes;
    /// the typed return removes both.
    #[must_use]
    pub fn discriminant(&self) -> MatchEventDiscriminant {
        match self {
            MatchEvent::KickOff { .. } => MatchEventDiscriminant::KickOff,
            MatchEvent::FullTime { .. } => MatchEventDiscriminant::FullTime,
            MatchEvent::Goal { .. } => MatchEventDiscriminant::Goal,
            MatchEvent::Shot { .. } => MatchEventDiscriminant::Shot,
            MatchEvent::Pass { .. } => MatchEventDiscriminant::Pass,
            MatchEvent::SignatureFirstFired { .. } => MatchEventDiscriminant::SignatureFirstFired,
            MatchEvent::Offside { .. } => MatchEventDiscriminant::Offside,
            MatchEvent::PassIncomplete { .. } => MatchEventDiscriminant::PassIncomplete,
        }
    }
}

// ---------------------------------------------------------------------------
// MatchEventDiscriminant
// ---------------------------------------------------------------------------

/// Stable discriminant for each `MatchEvent` variant. Values mirror the
/// canonical encoder table in `fw-match-sim::canonical` — do NOT reorder.
///
/// | Discriminant | Variant              |
/// |---|---|
/// | 0            | `KickOff`            |
/// | 1            | `FullTime`           |
/// | 2            | `Goal`               |
/// | 3            | `Shot`               |
/// | 4            | `Pass`               |
/// | 5            | `SignatureFirstFired` |
/// | 6            | `Offside`            |
/// | 7            | `PassIncomplete`     |
///
/// `#[repr(u8)]` per Codex Tier-2 type-design P1 on T1-4b: pins the
/// discriminant layout for any future `transmute` / FFI / serde-repr
/// need. The encoder casts `event.discriminant() as u8`.
///
/// **Located in `event.rs` post Codex T1-11 type-design P1 fix-pass**:
/// the enum was originally in `commentary.rs` (a downstream consumer)
/// but `MatchEvent::discriminant()` returns this type, so it MUST live
/// with `MatchEvent` (cyclic-import otherwise). `commentary.rs`
/// re-exports for backwards compat with existing consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum MatchEventDiscriminant {
    KickOff = 0,
    FullTime = 1,
    Goal = 2,
    Shot = 3,
    Pass = 4,
    SignatureFirstFired = 5,
    /// FUN-TS2b: offside detection at pass-launch.
    Offside = 6,
    /// FUN-CB1: failed pass — loose ball spawned.
    PassIncomplete = 7,
}

impl MatchEventDiscriminant {
    /// Derive the discriminant from a live `MatchEvent`.
    ///
    /// Trivial passthrough to [`MatchEvent::discriminant()`] post the
    /// Codex T1-11 type-design P1 fix-pass — both functions share the
    /// same source of truth via the typed enum return; no byte→enum
    /// round-trip, no `unreachable!()` panic landmine on unknown bytes.
    ///
    /// The cross-crate test `fw-content/tests/event_discriminant_test.rs`
    /// still pins this function AND the encoder's byte output against the
    /// same hardcoded table, catching any future drift.
    pub fn from_event(event: &MatchEvent) -> Self {
        event.discriminant()
    }

    /// All discriminants in canonical order — used by the `ContentStore` loader
    /// to validate that every event class has a grammar loaded.
    pub fn all() -> [MatchEventDiscriminant; 8] {
        [
            Self::KickOff,
            Self::FullTime,
            Self::Goal,
            Self::Shot,
            Self::Pass,
            Self::SignatureFirstFired,
            Self::Offside,
            Self::PassIncomplete,
        ]
    }
}

/// Half-width of the goal in pitch coordinates (m). Used to determine
/// `on_target` for shot events.
///
/// Standard goal is 7.32 m wide (4.88 m posts-from-centre = 3.66 m half-width
/// to each post). Using 3.66 m here; T2 will refine with actual keeper position.
///
/// **Single source of truth.** `is_shot_on_target` MUST read from this constant;
/// any inline literal disagrees by silent drift (caught by Codex Tier-2 P1 audit
/// on T1-4a 2026-05-16 — prior value was 15_633_680_957 ≈ 3.640 m via a wrong
/// bit-shift construction).
///
/// Raw bits: `3.66 * 2^32 = 15_720_299_520` exactly (modulo float-bake rounding).
pub const GOAL_HALF_WIDTH_M: Q32 = Q32::from_raw(15_720_299_520_i64);

/// On-target test: returns `true` if a shot at `target_y` is heading between
/// the posts (i.e. `|target_y| <= GOAL_HALF_WIDTH_M`).
///
/// T1 approximation — no keeper position, no dive model.
///
/// **Panic safety:** uses `i64::unsigned_abs()` to avoid the `i64::MIN.abs()`
/// undefined-behavior path (`-i64::MIN` overflows). A future caller passing a
/// pathological `Q32::from_raw(i64::MIN)` returns false (the comparison fits
/// in `u64` and `u64::MIN` is the goal half-width as `u64` would always be smaller).
pub fn is_shot_on_target(target_y: Q32) -> bool {
    // `unsigned_abs()` returns u64 — `i64::MIN.unsigned_abs()` is
    // `1 << 63` which is correctly representable; no overflow path.
    let abs_y_u64: u64 = target_y.to_bits().unsigned_abs();
    let half_width_u64: u64 = GOAL_HALF_WIDTH_M.to_bits() as u64;
    // GOAL_HALF_WIDTH_M is positive, so its raw bits as u64 are the same
    // magnitude. Compare in u64 space (abs_y is always non-negative).
    abs_y_u64 <= half_width_u64
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_core::{Q32, Tick};

    // --- RED tests (written before impl; these compile once the types exist) ---

    #[test]
    fn kick_off_event_round_trips_serde() {
        let ev = MatchEvent::KickOff {
            tick: Tick::ZERO,
            is_second_half: false,
        };
        let json = serde_json::to_string(&ev).expect("serialize KickOff");
        let ev2: MatchEvent = serde_json::from_str(&json).expect("deserialize KickOff");
        assert_eq!(ev, ev2);
    }

    #[test]
    fn full_time_event_round_trips_serde() {
        let ev = MatchEvent::FullTime {
            tick: Tick::from_raw(60),
            home_score: 2,
            away_score: 1,
        };
        let json = serde_json::to_string(&ev).expect("serialize FullTime");
        let ev2: MatchEvent = serde_json::from_str(&json).expect("deserialize FullTime");
        assert_eq!(ev, ev2);
    }

    #[test]
    fn goal_event_round_trips_serde() {
        let ev = MatchEvent::Goal {
            scorer_slot: 8,
            tick: Tick::from_raw(30),
            score_home_after: 1,
            score_away_after: 0,
        };
        let json = serde_json::to_string(&ev).expect("serialize Goal");
        let ev2: MatchEvent = serde_json::from_str(&json).expect("deserialize Goal");
        assert_eq!(ev, ev2);
    }

    #[test]
    fn shot_event_round_trips_serde() {
        let ev = MatchEvent::Shot {
            shooter_slot: 9,
            tick: Tick::from_raw(45),
            target_x: Q32::from_int(52),
            target_y: Q32::ZERO,
            on_target: true,
        };
        let json = serde_json::to_string(&ev).expect("serialize Shot");
        let ev2: MatchEvent = serde_json::from_str(&json).expect("deserialize Shot");
        assert_eq!(ev, ev2);
    }

    #[test]
    fn pass_event_round_trips_serde() {
        let ev = MatchEvent::Pass {
            from_slot: 5,
            to_slot: 7,
            tick: Tick::from_raw(20),
            kind: PassKind::Short,
            completed: true,
        };
        let json = serde_json::to_string(&ev).expect("serialize Pass");
        let ev2: MatchEvent = serde_json::from_str(&json).expect("deserialize Pass");
        assert_eq!(ev, ev2);
    }

    #[test]
    fn signature_first_fired_event_round_trips_serde() {
        let id = SignatureId::try_new("fwh.core:signature.long-range-strike").unwrap();
        let ev = MatchEvent::SignatureFirstFired {
            player_slot: 9,
            signature_id: id,
            tick: Tick::from_raw(50),
        };
        let json = serde_json::to_string(&ev).expect("serialize SignatureFirstFired");
        let ev2: MatchEvent = serde_json::from_str(&json).expect("deserialize SignatureFirstFired");
        assert_eq!(ev, ev2);
    }

    #[test]
    fn all_pass_kind_variants_are_distinct() {
        assert_ne!(PassKind::Short, PassKind::Long);
        assert_ne!(PassKind::Short, PassKind::Cross);
        assert_ne!(PassKind::Short, PassKind::LayOff);
        assert_ne!(PassKind::Long, PassKind::Cross);
        assert_ne!(PassKind::Long, PassKind::LayOff);
        assert_ne!(PassKind::Cross, PassKind::LayOff);
    }

    #[test]
    fn is_shot_on_target_within_posts() {
        // y = 0 (centre of goal) → on target
        assert!(is_shot_on_target(Q32::ZERO));
        // y = 3.0 m → on target (within 3.66 m half-width)
        assert!(is_shot_on_target(Q32::from_int(3)));
        // y = -3.0 m → on target
        assert!(is_shot_on_target(Q32::from_raw(
            -Q32::from_int(3).to_bits()
        )));
    }

    #[test]
    fn is_shot_on_target_outside_posts() {
        // y = 5 m → outside posts (half-width 3.66 m)
        assert!(!is_shot_on_target(Q32::from_int(5)));
        // y = -5 m → outside posts
        assert!(!is_shot_on_target(Q32::from_raw(
            -Q32::from_int(5).to_bits()
        )));
    }

    #[test]
    fn match_event_size_is_documented() {
        // Size probe — documents the enum's memory footprint. Helps catch
        // accidental bloat if a new variant or field adds unexpectedly.
        // Current expected max: SignatureFirstFired with SignatureId (String = 24 bytes on 64-bit)
        // + PlayerSlot (u8) + Tick (i64) = ~33 bytes; enum wrapper adds discriminant.
        // Exact size is platform-dependent but should be < 64 bytes.
        let sz = std::mem::size_of::<MatchEvent>();
        assert!(
            sz <= 64,
            "MatchEvent is unexpectedly large: {} bytes; \
             review variant fields for bloat (String + Q32 fields)",
            sz
        );
    }
}

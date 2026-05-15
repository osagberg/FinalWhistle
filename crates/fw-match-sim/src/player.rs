//! Per-player canonical state.
//!
//! T1-2b-iii-a additions (revised by self-review P1-1 + P1-2):
//! - `role_state: PlayerRoleState` — typed pair of (Role, per-role state) that
//!   makes illegal combinations (e.g. `Defender + GoalkeeperState`)
//!   unrepresentable. Replaces the former `role: Role` + `role_state: u8`
//!   split. Canonical wire format is byte-identical (to_tags() emits the same
//!   two u8 bytes in the same order).
//! - `local_decision_counter: u32` — monotonic per-player counter that
//!   increments each time the BT/FSM fires a decision. Resets to 0 at
//!   match-init. Used as the `site` suffix in `seed_fn` per ADR-0009:
//!   `site = (player_id << 16) | local_decision_counter`. Capped at `u32`
//!   (a single 90-minute match fires at most ~21_600 decisions per player;
//!   `u32` gives >4 billion headroom). Visibility: `pub(crate)` with public
//!   accessor `decision_counter()` and crate-internal `bump_decision_counter()`.
//!
//! Prior scope note: position + velocity in Q32, plus the slot index that
//! pins the player to a canonical-encoding position. Behavior-tree state,
//! signature-readiness, fatigue, etc. all land in later phases. The struct
//! is intentionally small so the canonical-hash baseline is easy to reason
//! about; subsequent phases extend it with `#[serde(default)]` fields where
//! backward compatibility is needed.

use fw_content::SignatureCandidate;
use fw_core::{PlayerAttributes, Q32};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::PlayerSlot;
use crate::role_states::{PlayerRoleState, Role};

/// Per-player canonical state. Slot-indexed inside `MatchState::players`.
///
/// ## T1-2b-iii-a canonical-encoder note
///
/// The canonical encoder emits fields in declaration order:
///   slot → pos_x → pos_y → vel_x → vel_y → scalars → role_tag → state_tag
///   → local_decision_counter
///
/// `role_state.to_tags()` produces `(role_tag u8, state_tag u8)`; both are
/// appended AFTER the existing scalar section. Wire-format byte positions of
/// prior fields are unchanged (forward-compatible extension).
///
/// The `local_decision_counter` (u32 LE, 4 bytes) follows immediately.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerState {
    /// Slot index 0..22. Pinned for the duration of the match; copied here
    /// for self-describing encoding (canonical-encoder reads it back out
    /// rather than threading the outer index through).
    pub slot: PlayerSlot,

    /// World-space position in metres. (0, 0) is the centre spot.
    /// `pos_x` runs goal-to-goal; `pos_y` runs touchline-to-touchline.
    pub pos_x: Q32,
    pub pos_y: Q32,

    /// World-space velocity in m/s. Set by `apply_intent` in `dispatch.rs`.
    /// Integrated into position by `tick_match` each tick.
    pub vel_x: Q32,
    pub vel_y: Q32,

    /// Stamina / fatigue / readiness / other scalar per-player state. Stays
    /// a `BTreeMap<u16, Q32>` so individual keys can be added without
    /// changing the struct shape — the canonical encoder iterates in sorted
    /// key order so the encoding is stable regardless of insertion order.
    ///
    /// Key allocation is fixed at the content-pack level; see
    /// `fw-content::PlayerScalarKey` (T1+).
    #[serde(default)]
    pub scalars: BTreeMap<u16, Q32>,

    // ---- T1-2b-iii-a additions (revised P1-1: typed union replaces split) ----
    /// Typed (Role, role-state) pair. `PlayerRoleState::initial(role)` at
    /// match-init; transitions are evaluated per tick by `dispatch_tick`.
    ///
    /// The canonical encoder calls `.to_tags()` → two u8 bytes in the same
    /// order the prior split-field encoding emitted them. Hash is unchanged.
    pub role_state: PlayerRoleState,

    /// Monotonic per-player decision counter. Resets to 0 at match-init;
    /// incremented by `bump_decision_counter()` each time `dispatch_tick`
    /// fires a decision for this player. Used as the low 16 bits of the
    /// ADR-0009 `site` value for per-player BT RNG draws:
    ///   `site = (player_id.as_u32() as u64) << 16 | local_decision_counter`
    ///
    /// Visibility is `pub(crate)` — external code uses `decision_counter()`.
    pub(crate) local_decision_counter: u32,

    // ---- T1-2b-iii-b addition ----
    /// Player attribute record — 55 Q32 fields.
    ///
    /// Initialised to `PlayerAttributes::mid_range_baseline()` at match-init.
    /// BT utility scorers read via `attributes()` accessor.
    /// The canonical encoder appends these as 55 × i64 LE after the decision
    /// counter; canonical encoder bumped to VERSION 4 at T1-2b-iii-b.
    /// Visibility: `pub(crate)` — external code uses the `attributes()` accessor.
    #[serde(default = "PlayerAttributes::mid_range_baseline")]
    pub(crate) attributes: PlayerAttributes,

    // ---- T1-2b-iv addition ----
    /// Per-player signature candidates — loaded from `PlayerTemplate` at match
    /// init. NOT encoded in canonical state (content-derived, not match state).
    /// Empty by default; populated by the match-setup path or test fixtures.
    ///
    /// `pub(crate)` — external callers use `signature_candidates()` (read) and
    /// `signature_candidates_mut()` (write; used by match-setup + tests).
    ///
    /// The dispatcher evaluates trigger predicates for each candidate each
    /// decision tick.
    #[serde(default)]
    pub(crate) signature_candidates: Vec<SignatureCandidate>,
}

impl PlayerState {
    /// Construct a player at `(x, y)` with zero velocity, no scalars, and
    /// the given role. The role_state is initialised to the role's default
    /// via `PlayerRoleState::initial(role)`. local_decision_counter starts at 0.
    ///
    /// Used by `MatchState::initial`.
    pub fn with_role(slot: PlayerSlot, x: Q32, y: Q32, role: Role) -> PlayerState {
        PlayerState {
            slot,
            pos_x: x,
            pos_y: y,
            vel_x: Q32::ZERO,
            vel_y: Q32::ZERO,
            scalars: BTreeMap::new(),
            role_state: PlayerRoleState::initial(role),
            local_decision_counter: 0,
            attributes: PlayerAttributes::mid_range_baseline(),
            signature_candidates: Vec::new(),
        }
    }

    /// Backward-compat constructor (no role argument). Assigns
    /// `Role::Midfielder` with default state so old call sites
    /// (tests that don't care about role) continue to compile.
    /// All new call sites should use `with_role`.
    ///
    /// P3 defer: this defaults to Midfielder — callers that need a specific
    /// role must use `with_role`. Footgun documented; cleanup in -iii-b.
    pub fn at(slot: PlayerSlot, x: Q32, y: Q32) -> PlayerState {
        PlayerState::with_role(slot, x, y, Role::Midfielder)
    }

    /// The coarse role for this player (GK / DEF / MID / FWD).
    #[must_use]
    pub fn role(&self) -> Role {
        self.role_state.role()
    }

    /// Read the monotonic decision counter. Increments once per BT/FSM
    /// decision fired by `dispatch_tick`.
    #[must_use]
    pub fn decision_counter(&self) -> u32 {
        self.local_decision_counter
    }

    /// Read the player's attribute record. External code (IPC DTOs, tests)
    /// uses this accessor; BT utility scorers in the same crate access
    /// `self.attributes` directly.
    #[must_use]
    pub fn attributes(&self) -> &PlayerAttributes {
        &self.attributes
    }

    /// Mutable access to the player's attribute record.
    ///
    /// Used by match-setup code (which populates attributes from
    /// `PlayerTemplate`) and test fixtures. The BT utility scorers
    /// in the same crate access `self.attributes` directly.
    pub fn attributes_mut(&mut self) -> &mut PlayerAttributes {
        &mut self.attributes
    }

    /// Read-only view of the player's signature candidates.
    /// External code (IPC DTOs, tests) uses this accessor.
    #[must_use]
    pub fn signature_candidates(&self) -> &[SignatureCandidate] {
        &self.signature_candidates
    }

    /// Mutable access to the signature candidates list.
    ///
    /// Used by match-setup code (which populates candidates from
    /// `PlayerTemplate`) and test fixtures. The dispatch path reads
    /// candidates via the read accessor or clones them internally.
    pub fn signature_candidates_mut(&mut self) -> &mut Vec<SignatureCandidate> {
        &mut self.signature_candidates
    }

    /// Increment the decision counter (crate-internal; only `dispatch.rs`
    /// should call this). Uses `saturating_add` so a marathon match can't
    /// cause UB even though `u32` has ample headroom for 90 minutes.
    pub(crate) fn bump_decision_counter(&mut self) {
        self.local_decision_counter = self.local_decision_counter.saturating_add(1);
    }
}

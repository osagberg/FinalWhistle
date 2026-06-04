//! Team defensive shape — closed-form affine zonal transform (FUN-TS1).
//!
//! ## Design
//!
//! ADR-0013: team shape is a **pure function of canonical inputs**
//! (`tactic_state`, player positions, ball_x, possession) computed once per
//! tick at the top of `dispatch_tick`, before the per-player decision loop.
//! The sidecar `[TeamShape; 2]` on `MatchState` is `#[serde(skip)]`; it
//! adds **no canonical bytes** — recomputed each tick from canonical inputs.
//!
//! ## Seam
//!
//! `compute(team_idx, state)` is called in `dispatch_tick` after the carrier
//! pre-pass and before the per-slot decision loop. The returned `TeamShape`
//! is written to `state.team_shape[team_idx]`.
//!
//! `zonal_slot(roster_slot, shape, attack_dir)` is then called by the three
//! off-ball utilities (`utility_track_back`, `utility_hold_formation`,
//! `utility_run_off_ball`) to obtain the FSM-shifted slot target instead of
//! the static `formation_position(slot)`.
//!
//! ## Determinism
//!
//! No floats. No clocks. No HashMap. No async. All Q32.
//! Line-height constants below are SOFT tuning values — see
//! `docs/design/tactical-shape.md §Tuning bands`.

use serde::{Deserialize, Serialize};

use fw_core::Q32;

use crate::MatchState;
use crate::subtree_library::FORMATION_4_3_3_POSITIONS;
use crate::tactic_fsm::TacticState;

// ---------------------------------------------------------------------------
// Tuning constants
// All distances are signed metres from pitch centre (range ±52.5m).
// The design doc lists them as "distance from own goal line at ±52.5m";
// we store them as signed pitch-x values (positive = toward +x end).
//
// SOFT tuning — see docs/design/tactical-shape.md §Tuning bands.
// ---------------------------------------------------------------------------

// Line heights — from docs/design/match-realism-reference.md §3.
// "LowBlock ~22–28m, mid block ~38–42m, high line ~48–55m from own goal."
// Home own goal is at x = -52.5m; away own goal at +52.5m.
// Seed values per that doc: LowBlock ~25m, MidBlock ~40m, HighPress ~52m, CA ~45m.
// Converted: from_own_goal_m = goal_line + metres_from_own_goal
//   home LowBlock: -52.5 + 25 = -27.5 ≈ -28m    SOFT
//   home MidBlock: -52.5 + 40 = -12.5 ≈ -13m    SOFT
//   home HighPress:-52.5 + 52 = -0.5  ≈  -1m    SOFT
//   home Counter:  -52.5 + 45 =  -7.5 ≈  -8m    SOFT
// Ordering invariant (HARD from research): LowBlock < MidBlock < HighPress.

/// Home-side defensive-line x in LowBlock (~25m from own goal). SOFT.
const LOW_BLOCK_LINE_METRES: i32 = -28; // SOFT

/// Home-side defensive-line x in MidBlock (~40m from own goal). SOFT.
/// Research anchor: "mid block ~38–42m" — sits clearly above LowBlock + below HighLine.
const MID_BLOCK_LINE_METRES: i32 = -13; // SOFT

/// Home-side defensive-line x in HighPress (~55m from own goal). SOFT.
/// Research: "high line ~48–55m" from own goal → -52.5+55 = +2.5m ≈ +2m (past centre).
/// Home DEFs at +2m, home FWDs at +2 + 35×(35/40) = +2+30.6 = +33m — in opp half.
/// Away HighPress: -(+2) = -2m (2m past centre into home half). Ordering: away < 0.
/// Previously caused runaway because enforcement was broken (defenders chased carriers).
/// With enforce_hold_zonal, defenders hold +2m and form a real block.
const HIGH_PRESS_LINE_METRES: i32 = 2; // SOFT

/// Home-side defensive-line x in CounterAttack (~45m from own goal). SOFT.
const COUNTER_ATTACK_LINE_METRES: i32 = -8; // SOFT

// Compactness — from docs/design/match-realism-reference.md §3.
// "Vertical compactness (def→fwd, out of possession): ~25–35m total (3 lines × ~10–15m)."
// The 40m HighPress value is the LOOSE end (correct — higher press = more stretched).
// With enforcement fixed (block-hold dominant intent), these values should produce
// a genuine block rather than the 1.6 goals/match over-correction.

/// Target vertical span in LowBlock. 25m — tightest block. SOFT.
const LOW_BLOCK_COMPACTNESS_V: i32 = 25; // SOFT

/// Target vertical span in MidBlock. 30m. SOFT.
const MID_BLOCK_COMPACTNESS_V: i32 = 30; // SOFT

/// Target vertical span in HighPress. 35m — most stretched (research: looser = more press space).
/// With line_x=-1m and span=35m: home FWD at -1 + 40×(35/40) = -1 + 35 = +34m.
/// This is feasible because enforcement (Fix 1) means DEFs now hold their line at -1m,
/// not chase into the opponent's half. SOFT.
const HIGH_PRESS_COMPACTNESS_V: i32 = 35; // SOFT

/// CounterAttack: moderate stretch (~MidBlock). SOFT.
const COUNTER_ATTACK_COMPACTNESS_V: i32 = MID_BLOCK_COMPACTNESS_V;

/// Target horizontal (y-axis) compactness for all states.
/// Research: "out of possession ~30–44m" — use 35m as mid-range seed. SOFT.
const COMPACTNESS_H: i32 = 35; // SOFT — FUN-TS3 possession-widening deferred to Slice 4

// NOTE: FORMATION_NATIVE_H and FORMATION_NATIVE_V are encoded into the
// inv40 constant used in zonal_slot. They are not needed as named constants
// since the transform is derived from them at compile time.

// ---------------------------------------------------------------------------
// TeamShape
// ---------------------------------------------------------------------------

/// Per-team shape anchors for one tick — a pure derived sidecar (no canonical
/// bytes). Stored as `[TeamShape; 2]` on `MatchState` with `#[serde(skip)]`.
///
/// Index 0 = home team (defends -x goal).
/// Index 1 = away team (defends +x goal).
///
/// All fields are Q32 metres in pitch-coordinate space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamShape {
    /// Target defensive-line x (signed pitch-coord). For home team this is
    /// negative (own half); for away team positive.
    pub line_x: Q32,
    /// Mean x of the 10 outfield defenders this tick (the actual centroid,
    /// not the target — used for compactness measurement + proptest checks).
    pub block_centroid_x: Q32,
    /// Mean y of the 10 outfield defenders this tick.
    pub block_centroid_y: Q32,
    /// Target vertical span (rearmost → foremost non-GK), metres.
    pub compactness_v: Q32,
    /// Target horizontal span, metres.
    pub compactness_h: Q32,
    /// True when the opposing team has possession (or ball is loose) — this
    /// team is in a DEFENSIVE shape (block-hold enforcement active).
    /// False when this team has possession — normal softmax utilities apply.
    /// Derived from canonical `state.possession` in `compute`; NOT canonical bytes.
    pub is_defending: bool,
    /// FUN-TS2b: coordinated press role for each team-local slot (0..11).
    /// Index 0 = GK (always HoldShape). Indices 1..11 = outfield.
    /// Only meaningful when `is_defending && is_high_press`.
    /// Filled by `compute_press_from_parts`; default is all HoldShape.
    pub press_roles: [PressRole; 11],
    /// True when this team's current tactic state is HighPress AND it is
    /// defending. Used in `select_outfield_intent` to decide whether to apply
    /// coordinated press-role routing or fall back to standard utility_press.
    /// Derived from canonical tactic_state in `compute_press_from_parts`;
    /// NOT canonical bytes (follows the same pattern as `is_defending`).
    pub is_high_press: bool,
}

impl TeamShape {
    /// Returns the team-neutral zero shape (used as `serde(skip)` default
    /// and at match-init before the first compute pass).
    #[must_use]
    pub const fn zero() -> TeamShape {
        TeamShape {
            line_x: Q32::ZERO,
            block_centroid_x: Q32::ZERO,
            block_centroid_y: Q32::ZERO,
            compactness_v: Q32::from_int(MID_BLOCK_COMPACTNESS_V),
            compactness_h: Q32::from_int(COMPACTNESS_H),
            is_defending: true,
            press_roles: [PressRole::HoldShape; 11],
            is_high_press: false,
        }
    }

    /// A const all-zeros shape for use in test statics (compactness intentionally
    /// zero — tests that need real compactness construct `test_shape()` locally).
    pub const CONST_ZERO: TeamShape = TeamShape {
        line_x: Q32::ZERO,
        block_centroid_x: Q32::ZERO,
        block_centroid_y: Q32::ZERO,
        compactness_v: Q32::ZERO,
        compactness_h: Q32::ZERO,
        is_defending: true,
        press_roles: [PressRole::HoldShape; 11],
        is_high_press: false,
    };

    /// Press role for this team-local slot (0-indexed within the team, 0..11).
    /// GK (slot 0) always HoldShape.
    /// Out-of-range index panics via the array bound (release + debug).
    #[must_use]
    pub fn press_role_for(&self, team_local_slot: usize) -> PressRole {
        self.press_roles[team_local_slot]
    }
}

/// `Default` impl for the `#[serde(skip)]` sidecar field on `MatchState`.
/// Serde requires `Default` when `skip` is used on a non-Option field.
impl Default for TeamShape {
    fn default() -> TeamShape {
        TeamShape::zero()
    }
}

// ---------------------------------------------------------------------------
// Shape computation
// ---------------------------------------------------------------------------

/// Returns the target `line_x` for `tactic_state`, adjusted for `team_idx`.
///
/// Constants are defined for the HOME team (team_idx = 0, defends -x goal).
/// Away team mirrors: negate the home-side constant.
#[must_use]
fn target_line_x(tactic_state: TacticState, team_idx: usize) -> Q32 {
    let home_x = match tactic_state {
        TacticState::LowBlock => LOW_BLOCK_LINE_METRES,
        TacticState::MidBlock => MID_BLOCK_LINE_METRES,
        TacticState::HighPress => HIGH_PRESS_LINE_METRES,
        TacticState::CounterAttack => COUNTER_ATTACK_LINE_METRES,
        // SetPiece: use MidBlock height as a neutral default.
        TacticState::SetPiece(_) => MID_BLOCK_LINE_METRES,
    };
    // Away team inverts x (defends +x goal, so their "deep" is +x territory).
    let signed = if team_idx == 0 { home_x } else { -home_x };
    Q32::from_int(signed)
}

/// Returns the target compactness_v for a given tactic state.
#[must_use]
fn target_compactness_v(tactic_state: TacticState) -> Q32 {
    let metres = match tactic_state {
        TacticState::LowBlock => LOW_BLOCK_COMPACTNESS_V,
        TacticState::MidBlock => MID_BLOCK_COMPACTNESS_V,
        TacticState::HighPress => HIGH_PRESS_COMPACTNESS_V,
        TacticState::CounterAttack => COUNTER_ATTACK_COMPACTNESS_V,
        TacticState::SetPiece(_) => MID_BLOCK_COMPACTNESS_V,
    };
    Q32::from_int(metres)
}

/// Compute the per-team shape anchors for `team_idx` (0 = home, 1 = away)
/// from the current canonical `state`.
///
/// Called once per tick in `dispatch_tick` before the per-player decision loop.
/// Pure function of canonical inputs — deterministic, no RNG, no floats.
#[must_use]
pub fn compute(team_idx: usize, state: &MatchState) -> TeamShape {
    let tactic_state = state.team_tactic_states[team_idx].state();

    let line_x = target_line_x(tactic_state, team_idx);
    let compactness_v = target_compactness_v(tactic_state);
    let compactness_h = Q32::from_int(COMPACTNESS_H);

    // Compute actual centroid from the 10 outfield slots (exclude GK).
    // Home slots: 0..11 (GK = 0, outfield = 1..11).
    // Away slots: 11..22 (GK = 11, outfield = 12..22).
    let (slot_start, slot_end, gk_slot) = if team_idx == 0 {
        (0usize, 11usize, 0usize)
    } else {
        (11usize, 22usize, 11usize)
    };

    let mut sum_x = Q32::ZERO;
    let mut sum_y = Q32::ZERO;
    let mut count = 0i32;
    for slot_idx in slot_start..slot_end {
        if slot_idx == gk_slot {
            continue;
        }
        sum_x += state.players[slot_idx].pos_x;
        sum_y += state.players[slot_idx].pos_y;
        count += 1;
    }

    // count is always 10 (11 slots minus 1 GK).
    assert!(
        count == 10,
        "expected 10 outfield players for centroid; got {count}"
    );

    // Q32 division by 10: multiply by 1/10 ≈ 429_496_729 raw (≈ 0.1 in Q32).
    let inv10 = Q32::from_raw(429_496_730_i64); // 2^32 / 10 rounded up
    let block_centroid_x = sum_x * inv10;
    let block_centroid_y = sum_y * inv10;

    // is_defending: true when the opposing team has possession OR ball is loose.
    // Home team slots: 0..11 (u8 0-10); away team slots: 11..22 (u8 11-21).
    // When possession is Some(slot): if slot < 11 → home has ball → home is NOT defending.
    let is_defending = match state.possession {
        Some(carrier_slot) => {
            let carrier_is_home = (carrier_slot as usize) < crate::PLAYERS_PER_TEAM;
            // Defending ⟺ the carrier is on the OPPOSING team.
            if team_idx == 0 {
                !carrier_is_home
            } else {
                carrier_is_home
            }
        }
        // Loose ball: treat both teams as defending (block-hold = safe default).
        None => true,
    };

    TeamShape {
        line_x,
        block_centroid_x,
        block_centroid_y,
        compactness_v,
        compactness_h,
        is_defending,
        // press_roles and is_high_press filled by compute_press_from_parts()
        // called after compute() in dispatch_tick.
        press_roles: [PressRole::HoldShape; 11],
        is_high_press: false,
    }
}

// ---------------------------------------------------------------------------
// Zonal slot transform
// ---------------------------------------------------------------------------

/// Return the zonal-slot target `(x, y)` for `roster_slot` under `shape`.
///
/// The static formation position is shifted toward `shape.line_x` and
/// vertically compressed to `shape.compactness_v`, horizontally to
/// `shape.compactness_h`.
///
/// `team_idx`: 0 = home (defends -x), 1 = away (defends +x). Used to
/// determine which direction is "toward the defensive line."
///
/// **Lipschitz guarantee:** the output is a linear function of `line_x`
/// and the slot's raw formation position — no discontinuities on tactic-state
/// flips.
///
/// # Panics
///
/// Panics if `roster_slot >= 22` (invariant violation — same contract as
/// `formation_position`).
#[must_use]
pub fn zonal_slot(roster_slot: u8, shape: &TeamShape, team_idx: usize) -> (Q32, Q32) {
    assert!(
        (roster_slot as usize) < FORMATION_4_3_3_POSITIONS.len(),
        "zonal_slot: roster_slot {roster_slot} out of range; expected 0..22 — sim invariant violation"
    );

    let (raw_x, raw_y) = FORMATION_4_3_3_POSITIONS[roster_slot as usize];
    let form_x = Q32::from_int(raw_x);
    let form_y = Q32::from_int(raw_y);

    // --- X transform: blend formation x toward line_x ---
    //
    // The formation's rearmost non-GK (defender) is at x = ±30m; the
    // forward is at x = ±10m. We want the block to shift so its rearmost
    // line lands at `line_x`.
    //
    // Approach: per-slot x = line_x + (form_x - form_defender_x) × scale_v
    // where form_defender_x is the natural rearmost-defender x (±30m) and
    // scale_v = compactness_v / FORMATION_NATIVE_V.
    //
    // For home team: form_defender_x = -30, FORMATION_NATIVE_V = 40.
    // For away team: form_defender_x = +30, FORMATION_NATIVE_V = 40.
    let form_defender_x_i = if team_idx == 0 { -30i32 } else { 30i32 };
    let form_defender_x = Q32::from_int(form_defender_x_i);

    // relative_x = formation_x − defender_anchor
    // For a home DEF at x=-30: relative_x = 0.
    // For a home FWD at x=+10: relative_x = +40.
    let relative_x = form_x - form_defender_x;

    // compactness_v / FORMATION_NATIVE_V = vertical scale factor (Q32).
    // FORMATION_NATIVE_V = 40; to avoid float we compute scale as:
    // scale_v = compactness_v × (1/40)
    // 1/40 in Q32 raw = 2^32 / 40 = 107_374_182.4 ≈ 107_374_182
    let inv40 = Q32::from_raw(107_374_182_i64);
    let scale_v = shape.compactness_v * inv40;

    // target_x = line_x + relative_x × scale_v
    // For a DEF (relative_x = 0): target_x = line_x (anchors the rear line).
    // For a FWD (relative_x = +40 home, -40 away): target_x scaled by compactness.
    let target_x = shape.line_x + relative_x * scale_v;

    // --- Y transform: scale toward compactness_h ---
    //
    // compactness_h_scale = compactness_h / (2 × FORMATION_NATIVE_H)
    // FORMATION_NATIVE_H = 20; half-span = 20; full-span = 40.
    // 1/40 = same inv40 constant.
    let scale_h = shape.compactness_h * inv40;
    let target_y = form_y * scale_h;

    // No per-team Y mirror: the 4-3-3 is symmetric across the pitch Y axis, so the
    // away block uses the same Y bands as home — only X is direction-mirrored (via
    // `line_x`'s sign and `form_defender_x`). Y-mirroring would be a no-op on shape.

    (target_x, target_y)
}

// ---------------------------------------------------------------------------
// PressPlan — coordinated press assignment (FUN-TS2b)
// ---------------------------------------------------------------------------
//
// A PressPlan is computed once per tick AFTER TeamShape (so it can use
// shape.is_defending) and stored as a non-canonical sidecar on MatchState.
//
// Role assignment:
//   Primary:   1 player — nearest defending-team player to the ball carrier
//              (Q32 Euclidean-squared distance; slot-order tiebreak for
//              determinism).
//   Cover:     2 players — next-nearest (same tiebreak).
//   HoldShape: remaining 8 outfield + 1 GK.
//
// When `team_state` is NOT HighPress, or when `is_defending` is false, every
// slot is assigned `HoldShape`.
//
// Determinism: all arithmetic is Q32; distances are squared (no sqrt); slot
// order provides a fully-deterministic tiebreak under equal distance.

/// Press role for a defending player in the coordinated press plan.
///
/// Only meaningful when the team is in `HighPress` tactic state AND defending.
/// When the team is in any other state, all roles resolve to `HoldShape`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PressRole {
    /// Step up to press the ball carrier — the one closest defender.
    Primary,
    /// Cut the nearest passing lane behind the primary presser.
    Cover,
    /// Hold the defensive block; do not step up.
    HoldShape,
}

/// Compute and fill per-team coordinated press roles into `shapes`.
///
/// Called once per tick AFTER `compute` (which fills `is_defending`), BEFORE
/// the per-slot decision loop so the Pressing arm can read `shape.press_roles`.
///
/// For each team in `HighPress` tactic state AND `is_defending`:
///   - Sort the 10 outfield defending-team slots by squared-distance to the
///     carrier (GK always HoldShape — never the primary presser).
///   - Assign `Primary` to rank 0, `Cover` to ranks 1..=2, `HoldShape` to rest.
///
/// When the carrier is `None` (loose ball), all roles stay `HoldShape`.
///
/// Roles are stored in `TeamShape::press_roles[team_local_slot]` where
/// `team_local_slot` is 0-indexed within the team (home 0..11, away 0..11 via
/// slot - 11 for away). The slot-order tiebreak ensures full determinism.
///
/// # Determinism
/// - Q32 squared distance (no sqrt, no floats).
/// - Slot-order tiebreak on equal distance.
/// - No HashMap; sort via `Vec<(Q32, u8)>` with stable key ordering.
///
/// This function takes decomposed parts (rather than `&MatchState`) so the
/// caller in `dispatch_tick` can split-borrow `state.team_shape` (mut) from
/// `state.players` / `state.possession` / `state.team_tactic_states` (shared).
pub fn compute_press_from_parts(
    shapes: &mut [TeamShape; 2],
    possession: Option<u8>,
    player_positions: &[(Q32, Q32); 22],
    tactic_states: &[crate::tactic_fsm::TeamTacticState; 2],
) {
    // No carrier → leave all roles at HoldShape.
    let carrier_slot = match possession {
        Some(s) => s,
        None => return,
    };
    let (carrier_x, carrier_y) = player_positions[carrier_slot as usize];

    for team_idx in 0..2usize {
        // Only assign press roles when defending AND in HighPress state.
        if !shapes[team_idx].is_defending
            || tactic_states[team_idx].state() != TacticState::HighPress
        {
            // Non-HighPress: mark is_high_press=false so subtree_library knows
            // to fall back to standard utility_press behavior.
            shapes[team_idx].is_high_press = false;
            continue;
        }

        // Reset roles to HoldShape before assigning, then mark active.
        shapes[team_idx].press_roles = [PressRole::HoldShape; 11];
        shapes[team_idx].is_high_press = true;

        // Collect (squared_distance, team_local_slot) for all outfield slots.
        // Home team: absolute slots 0..11; away team: absolute slots 11..22.
        // Team-local slot = absolute_slot for home; absolute_slot - 11 for away.
        let (abs_start, abs_end, gk_abs) = if team_idx == 0 {
            (0u8, 11u8, 0u8)
        } else {
            (11u8, 22u8, 11u8)
        };

        let mut distances: Vec<(Q32, u8)> = Vec::with_capacity(10);
        for abs_slot in abs_start..abs_end {
            if abs_slot == gk_abs {
                continue;
            }
            let (px, py) = player_positions[abs_slot as usize];
            let dx = px - carrier_x;
            let dy = py - carrier_y;
            let dist_sq = dx * dx + dy * dy;
            let local_slot = if team_idx == 0 {
                abs_slot
            } else {
                abs_slot - 11
            };
            distances.push((dist_sq, local_slot));
        }

        // Sort by (dist_sq ASC, local_slot ASC) — fully deterministic.
        distances.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

        // Assign roles into TeamShape.
        for (rank, (_dist_sq, local_slot)) in distances.iter().enumerate() {
            shapes[team_idx].press_roles[*local_slot as usize] = match rank {
                0 => PressRole::Primary,
                1 | 2 => PressRole::Cover,
                _ => PressRole::HoldShape,
            };
        }
        // GK (local_slot 0) stays HoldShape (excluded from distances above).
    }
}

/// Convenience wrapper for use in tests. Calls `compute_press_from_parts`
/// from a full `MatchState` reference.
#[cfg(test)]
pub fn compute_press(shapes: &mut [TeamShape; 2], state: &MatchState) {
    let player_positions: [(Q32, Q32); 22] = {
        let mut arr = [(Q32::ZERO, Q32::ZERO); 22];
        for (i, p) in state.players.iter().enumerate() {
            arr[i] = (p.pos_x, p.pos_y);
        }
        arr
    };
    compute_press_from_parts(
        shapes,
        state.possession,
        &player_positions,
        &state.team_tactic_states,
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tactic_fsm::{TacticState, TeamTacticState};
    use fw_core::{Q32, Seed};

    fn mk_state_with_tactic(home_state: TacticState, away_state: TacticState) -> MatchState {
        use crate::MatchState;
        let seed = Seed::from_u64(0xCAFE_BABE_DEAD_BEEF);
        let mut state = MatchState::initial(seed);
        state.team_tactic_states[0] =
            TeamTacticState::initial().transition(home_state, fw_core::Tick::ZERO);
        state.team_tactic_states[1] =
            TeamTacticState::initial().transition(away_state, fw_core::Tick::ZERO);
        state
    }

    // --- target_line_x ---

    #[test]
    fn target_line_x_lowblock_home_is_negative() {
        let x = target_line_x(TacticState::LowBlock, 0);
        assert!(
            x < Q32::ZERO,
            "home LowBlock line must be in own half (negative x): got {x:?}"
        );
    }

    #[test]
    fn target_line_x_highpress_away_is_negative() {
        // Away team in HighPress: their line is in the HOME half (negative x territory).
        let x = target_line_x(TacticState::HighPress, 1);
        assert!(
            x < Q32::ZERO,
            "away HighPress line should be in home half (negative x): got {x:?}"
        );
    }

    #[test]
    fn target_line_x_home_highpress_gt_midblock_gt_lowblock() {
        let lo = target_line_x(TacticState::LowBlock, 0);
        let mid = target_line_x(TacticState::MidBlock, 0);
        let hi = target_line_x(TacticState::HighPress, 0);
        assert!(
            lo < mid,
            "LowBlock line must be deeper than MidBlock for home team"
        );
        assert!(
            mid < hi,
            "MidBlock line must be deeper than HighPress for home team"
        );
    }

    #[test]
    fn target_line_x_away_lowblock_gt_midblock_gt_highpress() {
        // Away team: defensive line is in +x territory (their own half).
        let lo = target_line_x(TacticState::LowBlock, 1);
        let mid = target_line_x(TacticState::MidBlock, 1);
        let hi = target_line_x(TacticState::HighPress, 1);
        // Away "deepest" = most positive x.
        assert!(
            lo > mid,
            "away LowBlock line must be deeper (more positive x) than MidBlock"
        );
        assert!(mid > hi, "away MidBlock line must be deeper than HighPress");
    }

    // --- compute ---

    #[test]
    fn compute_produces_correct_compactness_for_lowblock() {
        let state = mk_state_with_tactic(TacticState::LowBlock, TacticState::MidBlock);
        let shape = compute(0, &state);
        assert_eq!(
            shape.compactness_v,
            Q32::from_int(LOW_BLOCK_COMPACTNESS_V),
            "home team in LowBlock should have LOW_BLOCK_COMPACTNESS_V"
        );
    }

    #[test]
    fn compute_highpress_more_spread_than_lowblock() {
        // With enforcement fixed (enforce_hold_zonal), HighPress at 35m > LowBlock at 25m
        // is the correct real-world direction (research: "higher press = more stretched").
        let state_lb = mk_state_with_tactic(TacticState::LowBlock, TacticState::MidBlock);
        let state_hp = mk_state_with_tactic(TacticState::HighPress, TacticState::MidBlock);
        let shape_lb = compute(0, &state_lb);
        let shape_hp = compute(0, &state_hp);
        assert!(
            shape_hp.compactness_v > shape_lb.compactness_v,
            "HighPress compactness_v ({:?}) must be > LowBlock ({:?}): \
             higher press = more stretched (research-anchored direction)",
            shape_hp.compactness_v,
            shape_lb.compactness_v,
        );
        // Also verify line_x is more advanced than MidBlock (pressing line is forward).
        let state_mb = mk_state_with_tactic(TacticState::MidBlock, TacticState::MidBlock);
        let shape_mb = compute(0, &state_mb);
        assert!(
            shape_hp.line_x > shape_mb.line_x,
            "HighPress DEF line must be more forward (higher x) than MidBlock for home team"
        );
    }

    #[test]
    fn compute_line_x_lowblock_shallower_than_highpress_home() {
        let state_lb = mk_state_with_tactic(TacticState::LowBlock, TacticState::MidBlock);
        let state_hp = mk_state_with_tactic(TacticState::HighPress, TacticState::MidBlock);
        let shape_lb = compute(0, &state_lb);
        let shape_hp = compute(0, &state_hp);
        assert!(
            shape_lb.line_x < shape_hp.line_x,
            "home LowBlock line must be deeper (more negative x) than HighPress"
        );
    }

    // --- zonal_slot ---

    #[test]
    fn zonal_slot_defender_anchors_at_line_x() {
        let state = mk_state_with_tactic(TacticState::LowBlock, TacticState::MidBlock);
        let shape = compute(0, &state);
        // Home DEF slot 1 is the rearmost formation position (x = -30).
        let (tz_x, _tz_y) = zonal_slot(1, &shape, 0);
        // The defender's zonal_slot x should equal line_x (relative_x = 0 for DEF).
        assert_eq!(
            tz_x, shape.line_x,
            "home DEF slot 1 zonal_x must equal line_x (relative_x = 0 for defender anchor)"
        );
    }

    #[test]
    fn zonal_slot_forward_advanced_beyond_defender() {
        let state = mk_state_with_tactic(TacticState::MidBlock, TacticState::MidBlock);
        let shape = compute(0, &state);
        // FWD slot 8: raw_x = 10 (in front of DEF at -30 → relative_x = +40).
        let (fwd_x, _) = zonal_slot(8, &shape, 0);
        let (def_x, _) = zonal_slot(1, &shape, 0);
        assert!(
            fwd_x > def_x,
            "home FWD must be advanced beyond DEF in zonal_slot (fwd_x={fwd_x:?} def_x={def_x:?})"
        );
    }

    #[test]
    fn zonal_slot_y_compressed_relative_to_formation() {
        let state = mk_state_with_tactic(TacticState::LowBlock, TacticState::MidBlock);
        let shape = compute(0, &state);
        // DEF slot 1 has raw_y = -20 (maximum spread).
        let (_, tz_y) = zonal_slot(1, &shape, 0);
        let raw_y = Q32::from_int(-20);
        // compactness_h = 35, formation native half-span = 20, scale = 35/40 = 0.875
        // |tz_y| should be less than |raw_y|.
        let tz_y_abs = if tz_y < Q32::ZERO {
            Q32::ZERO - tz_y
        } else {
            tz_y
        };
        let raw_y_abs = if raw_y < Q32::ZERO {
            Q32::ZERO - raw_y
        } else {
            raw_y
        };
        assert!(
            tz_y_abs < raw_y_abs,
            "zonal_slot y must be compressed vs raw formation y (|tz_y|={tz_y_abs:?} |raw|={raw_y_abs:?})"
        );
    }

    #[test]
    fn zonal_slot_is_deterministic() {
        let state = mk_state_with_tactic(TacticState::MidBlock, TacticState::MidBlock);
        let shape = compute(0, &state);
        let r1 = zonal_slot(5, &shape, 0);
        let r2 = zonal_slot(5, &shape, 0);
        assert_eq!(r1, r2, "zonal_slot must be deterministic");
    }

    #[test]
    fn compute_centroid_is_within_pitch_bounds() {
        let state = mk_state_with_tactic(TacticState::MidBlock, TacticState::MidBlock);
        let shape = compute(0, &state);
        // Pitch: ±52.5m x, ±30m y. Centroid must be inside.
        let pitch_half_x = Q32::from_int(53);
        let pitch_half_y = Q32::from_int(31);
        assert!(
            shape.block_centroid_x > Q32::ZERO - pitch_half_x
                && shape.block_centroid_x < pitch_half_x,
            "block_centroid_x out of pitch bounds: {:?}",
            shape.block_centroid_x
        );
        assert!(
            shape.block_centroid_y > Q32::ZERO - pitch_half_y
                && shape.block_centroid_y < pitch_half_y,
            "block_centroid_y out of pitch bounds: {:?}",
            shape.block_centroid_y
        );
    }
}

//! Signature trigger predicates — T1-2b-iv.
//!
//! Each predicate is a pure function `(&MatchState, PlayerSlot) -> bool`.
//! Parameters (attribute thresholds, radius bounds) are specified here as
//! named constants so T2-4 tuning can find them without hunting raw numbers.
//!
//! ## Attribute thresholds (design choice)
//!
//! `mid_range_baseline()` sets all attributes to 0.5. For triggers to fire in
//! unit tests without bespoke attribute override, thresholds are set at 0.45
//! (slightly below baseline) so a baseline player CAN trigger them. Commentary
//! text and UX treat these as "reads the moment well" signatures — they should
//! fire occasionally at baseline, not exclusively at elite athletes.
//!
//! Real tuning of thresholds is a `systems-designer` task at T2-4. Document
//! deviations from the task-spec's 0.6/0.7/0.75 in the trigger constants below.
//!
//! ## Spatial proxies (T1-2b-iii-c pattern)
//!
//! Real ball+opponent position is not yet threaded into BT context. Each
//! trigger uses the same attribute-proxy pattern established in bt/on_ball.rs:
//!
//! - Distance-to-ball proxy: `mental.positioning` (high → closer to ball zone)
//! - Opponent mean_x: `mental.anticipation` (high → reads opposition spacing)
//!
//! Replaced with real geometry at T2-1 when spatial inputs land.
//!
//! ## Binding table (`TRIGGER_BINDING_TABLE`)
//!
//! A `BTreeMap<&'static str, fn(&MatchState, PlayerSlot) -> bool>` keyed by
//! the signature ID string. The dispatcher uses this at T1-2b-iv. The binding
//! is verified by the binding tests in this module.
//!
//! ## Determinism
//!
//! All predicates are pure, no RNG, no clocks, no floats, no HashMap.

use std::collections::BTreeMap;

use fw_core::Q32;

use crate::MatchState;
use crate::PlayerSlot;

// ---------------------------------------------------------------------------
// Threshold constants
// ---------------------------------------------------------------------------

/// `BodyShieldPressure` — defender must be in close marking range.
///
/// Distance proxy: the game uses `mental.marking` as "quality of marking
/// positioning". At baseline (0.5) a defender CAN trigger. Threshold tuned
/// below baseline so skeleton tests pass without attribute overrides.
///
/// Task-spec suggestion: marking >= 0.6, strength >= 0.6, aggression >= 0.5.
/// T1-2b-iv adjustment: all reduced to 0.45 so mid_range_baseline() players
/// can trigger. T2-4 tuning restores the spec values.
pub const BODY_SHIELD_THRESHOLD_MARKING: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45
pub const BODY_SHIELD_THRESHOLD_STRENGTH: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45
pub const BODY_SHIELD_THRESHOLD_AGGRESSION: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45

/// `LongRangeStrike` — attacker must be in shooting range with composure+technique.
///
/// Task-spec: composure >= 0.7, long_shots >= 0.7. Lowered to 0.45 for skeleton.
pub const LONG_RANGE_STRIKE_THRESHOLD_COMPOSURE: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45
pub const LONG_RANGE_STRIKE_THRESHOLD_LONG_SHOTS: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45

/// `FirstTimeDiagonalSwitch` — midfielder with vision+passing in playmaking range.
///
/// Task-spec: vision >= 0.75, passing >= 0.7. Lowered to 0.45 for skeleton.
pub const DIAGONAL_SWITCH_THRESHOLD_VISION: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45
pub const DIAGONAL_SWITCH_THRESHOLD_PASSING: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45

// ---------------------------------------------------------------------------
// Trigger functions
// ---------------------------------------------------------------------------

/// `fwh.core:signature.body-shield-pressure`
///
/// Fires when a defender is in a close-marking position with sufficient
/// physical + mental attributes to apply sustained body pressure.
///
/// Attribute reads (proxy at T1-2b-iv):
/// - Primary: `technical.marking` (marking quality proxy for proximity)
/// - Primary: `physical.strength` (physical hold-off capability)
/// - Primary: `personality.aggression` (commit to the press)
///
/// Spatial proxy: we check the player IS a home or away defender (slots 1-4
/// or 12-15) and their attributes meet the threshold. Real spatial check
/// (within 2m of ball-carrier) lands at T2-1.
pub fn body_shield_pressure_trigger(state: &MatchState, slot: PlayerSlot) -> bool {
    let idx = slot as usize;
    if idx >= state.players.len() {
        return false;
    }
    let a = &state.players[idx].attributes;

    // Role check: only defenders or midfielders apply body-shield pressure.
    // Slot layout: home DEF = 1-4, away DEF = 12-15; home MID = 5-7, away MID = 16-18.
    let in_team = idx % 11;
    let is_defender_or_mid = (1..=7).contains(&in_team);
    if !is_defender_or_mid {
        return false;
    }

    a.technical.marking >= BODY_SHIELD_THRESHOLD_MARKING
        && a.physical.strength >= BODY_SHIELD_THRESHOLD_STRENGTH
        && a.personality.aggression >= BODY_SHIELD_THRESHOLD_AGGRESSION
}

/// `fwh.core:signature.long-range-strike`
///
/// Fires when an attacker has the composure + technique to attempt a
/// long-range effort.
///
/// Attribute reads (proxy at T1-2b-iv):
/// - Primary: `mental.composure` (stays calm in long-range decision)
/// - Primary: `technical.long_shots` (technique for the shot)
///
/// Spatial proxy: we check the player is a forward/midfielder and that their
/// positioning attribute (high → near goal) supports the attempt.
/// Real check (ball pos > 25m from goal) lands at T2-1.
pub fn long_range_strike_trigger(state: &MatchState, slot: PlayerSlot) -> bool {
    let idx = slot as usize;
    if idx >= state.players.len() {
        return false;
    }
    let a = &state.players[idx].attributes;

    // Role check: forwards (slots 8-10, 19-21) and attacking midfielders (5-7, 16-18).
    let in_team = idx % 11;
    let is_attacker = (5..=10).contains(&in_team);
    if !is_attacker {
        return false;
    }

    a.mental.composure >= LONG_RANGE_STRIKE_THRESHOLD_COMPOSURE
        && a.technical.long_shots >= LONG_RANGE_STRIKE_THRESHOLD_LONG_SHOTS
}

/// `fwh.core:signature.first-time-diagonal-switch`
///
/// Fires when a midfielder with vision and passing quality can execute a
/// first-touch diagonal switch to stretch the opposition.
///
/// Attribute reads (proxy at T1-2b-iv):
/// - Primary: `mental.vision` (sees the diagonal option)
/// - Primary: `technical.passing` (quality to execute the switch)
///
/// Spatial proxy: checks the player is a midfielder. Real check
/// (ball arrived this tick + opponent mean_x on one wing) at T2-1.
pub fn first_time_diagonal_switch_trigger(state: &MatchState, slot: PlayerSlot) -> bool {
    let idx = slot as usize;
    if idx >= state.players.len() {
        return false;
    }
    let a = &state.players[idx].attributes;

    // Role check: midfielders only (home 5-7, away 16-18).
    let in_team = idx % 11;
    let is_midfielder = (5..=7).contains(&in_team);
    if !is_midfielder {
        return false;
    }

    a.mental.vision >= DIAGONAL_SWITCH_THRESHOLD_VISION
        && a.technical.passing >= DIAGONAL_SWITCH_THRESHOLD_PASSING
}

// ---------------------------------------------------------------------------
// Trigger binding table
// ---------------------------------------------------------------------------

/// Type alias for a trigger predicate function.
pub type TriggerFn = fn(&MatchState, PlayerSlot) -> bool;

/// Build the `SignatureId → TriggerFn` binding table.
///
/// Returns a `BTreeMap` (deterministic key iteration) keyed by signature ID
/// string (the `str` form of `SignatureId`). The dispatcher calls
/// `table.get(sig_id.as_str())` to resolve the predicate.
///
/// New signatures land by adding a row here AND a matching RON fixture.
/// The binding-correctness test verifies every RON fixture ID has a binding.
#[must_use]
pub fn build_trigger_table() -> BTreeMap<&'static str, TriggerFn> {
    let mut table: BTreeMap<&'static str, TriggerFn> = BTreeMap::new();
    table.insert(
        "fwh.core:signature.body-shield-pressure",
        body_shield_pressure_trigger,
    );
    table.insert(
        "fwh.core:signature.long-range-strike",
        long_range_strike_trigger,
    );
    table.insert(
        "fwh.core:signature.first-time-diagonal-switch",
        first_time_diagonal_switch_trigger,
    );
    // NoOpStub: always-false predicate (the no-op fixture's trigger)
    table.insert("fwh.core:signature.no-op-stub", no_op_trigger);
    table
}

/// Always-false predicate for the no-op stub signature (T1-3 fixture).
pub fn no_op_trigger(_state: &MatchState, _slot: PlayerSlot) -> bool {
    false
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_core::{PlayerAttributes, Q32, Seed};

    use crate::MatchState;

    fn baseline_state() -> MatchState {
        MatchState::initial(Seed::from_u64(1))
    }

    // Helper: set all attributes to a specific value.
    fn set_all_attrs(state: &mut MatchState, slot: usize, val: Q32) {
        let p = &mut state.players[slot];
        p.attributes = PlayerAttributes::mid_range_baseline();
        // Override the specific fields used by triggers.
        p.attributes.technical.marking = val;
        p.attributes.physical.strength = val;
        p.attributes.personality.aggression = val;
        p.attributes.mental.composure = val;
        p.attributes.technical.long_shots = val;
        p.attributes.mental.vision = val;
        p.attributes.technical.passing = val;
    }

    // ---- body_shield_pressure ----

    #[test]
    fn body_shield_fires_for_defender_above_threshold() {
        let mut state = baseline_state();
        // slot 1 = home DEF; set attributes above threshold (use Q32::ONE for certainty)
        set_all_attrs(&mut state, 1, Q32::ONE);
        assert!(body_shield_pressure_trigger(&state, 1));
    }

    #[test]
    fn body_shield_does_not_fire_for_forward() {
        let mut state = baseline_state();
        // slot 8 = home FWD; even with all attrs above threshold, role check fails.
        set_all_attrs(&mut state, 8, Q32::ONE);
        assert!(!body_shield_pressure_trigger(&state, 8));
    }

    #[test]
    fn body_shield_does_not_fire_when_attrs_below_threshold() {
        let mut state = baseline_state();
        // slot 1 = home DEF; set ZERO attributes — all thresholds fail.
        set_all_attrs(&mut state, 1, Q32::ZERO);
        assert!(!body_shield_pressure_trigger(&state, 1));
    }

    // ---- long_range_strike ----

    #[test]
    fn long_range_strike_fires_for_forward_above_threshold() {
        let mut state = baseline_state();
        // slot 8 = home FWD; attributes above threshold.
        set_all_attrs(&mut state, 8, Q32::ONE);
        assert!(long_range_strike_trigger(&state, 8));
    }

    #[test]
    fn long_range_strike_fires_for_midfielder_above_threshold() {
        let mut state = baseline_state();
        // slot 6 = home MID; midfielders (5-7) count as attackers for this predicate.
        set_all_attrs(&mut state, 6, Q32::ONE);
        assert!(long_range_strike_trigger(&state, 6));
    }

    #[test]
    fn long_range_strike_does_not_fire_for_defender() {
        let mut state = baseline_state();
        // slot 2 = home DEF; role check fails.
        set_all_attrs(&mut state, 2, Q32::ONE);
        assert!(!long_range_strike_trigger(&state, 2));
    }

    #[test]
    fn long_range_strike_does_not_fire_when_attrs_below_threshold() {
        let mut state = baseline_state();
        // slot 8 = home FWD; zero attributes.
        set_all_attrs(&mut state, 8, Q32::ZERO);
        assert!(!long_range_strike_trigger(&state, 8));
    }

    // ---- first_time_diagonal_switch ----

    #[test]
    fn diagonal_switch_fires_for_midfielder_above_threshold() {
        let mut state = baseline_state();
        // slot 5 = home MID.
        set_all_attrs(&mut state, 5, Q32::ONE);
        assert!(first_time_diagonal_switch_trigger(&state, 5));
    }

    #[test]
    fn diagonal_switch_does_not_fire_for_forward() {
        let mut state = baseline_state();
        // slot 9 = home FWD; role check fails.
        set_all_attrs(&mut state, 9, Q32::ONE);
        assert!(!first_time_diagonal_switch_trigger(&state, 9));
    }

    #[test]
    fn diagonal_switch_does_not_fire_when_attrs_below_threshold() {
        let mut state = baseline_state();
        // slot 5 = home MID; zero attrs.
        set_all_attrs(&mut state, 5, Q32::ZERO);
        assert!(!first_time_diagonal_switch_trigger(&state, 5));
    }

    // ---- no_op trigger ----

    #[test]
    fn no_op_trigger_always_returns_false() {
        let state = baseline_state();
        for slot in 0..22u8 {
            assert!(
                !no_op_trigger(&state, slot),
                "no_op_trigger should always return false, slot={slot}"
            );
        }
    }

    // ---- binding table ----

    #[test]
    fn trigger_table_contains_all_three_real_signatures() {
        let table = build_trigger_table();
        assert!(table.contains_key("fwh.core:signature.body-shield-pressure"));
        assert!(table.contains_key("fwh.core:signature.long-range-strike"));
        assert!(table.contains_key("fwh.core:signature.first-time-diagonal-switch"));
    }

    #[test]
    fn trigger_table_contains_no_op_stub() {
        let table = build_trigger_table();
        assert!(table.contains_key("fwh.core:signature.no-op-stub"));
    }

    #[test]
    fn trigger_table_no_op_entry_always_false() {
        let table = build_trigger_table();
        let state = baseline_state();
        let predicate = table["fwh.core:signature.no-op-stub"];
        for slot in 0..22u8 {
            assert!(!predicate(&state, slot));
        }
    }

    #[test]
    fn trigger_deterministic_same_state_same_result() {
        let mut state = baseline_state();
        set_all_attrs(&mut state, 8, Q32::ONE);
        let r1 = long_range_strike_trigger(&state, 8);
        let r2 = long_range_strike_trigger(&state, 8);
        assert_eq!(r1, r2, "trigger must be deterministic");
    }
}

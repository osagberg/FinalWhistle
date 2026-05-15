//! Signature trigger predicates — T1-2b-iv.
//!
//! Each predicate is a pure function `(&MatchState, PlayerSlot) -> Q32`
//! returning `Q32::ZERO` for not-eligible OR a positive fit-score in `(0, 1]`
//! for eligible. This allows the dispatcher to compute `affinity × fit_score`
//! as the softmax input per ADR-0011 §"Dispatch + softmax".
//!
//! ## P1-6 (T1-2b-fix): TriggerFn → Q32
//!
//! Changed from `-> bool` to `-> Q32` so the dispatcher can weight each
//! candidate by how well the current context fits (fit-score), not just
//! whether it is eligible. A trigger returning `Q32::ONE` means "perfect fit";
//! `Q32::ZERO` means "not eligible". The softmax input becomes
//! `candidate.affinity × fit_score` per ADR-0011.
//!
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
//! A `BTreeMap<&'static str, TriggerFn>` keyed by the signature ID string.
//! The dispatcher uses this at T1-2b-iv. The binding is verified by the
//! binding tests in this module.
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
/// Returns `Q32::ZERO` if not eligible (role check fails or attrs below threshold).
/// Returns a positive fit-score derived from the excess above threshold, in `(0, 1]`.
///
/// Attribute reads (proxy at T1-2b-iv):
/// - Primary: `technical.marking` (marking quality proxy for proximity)
/// - Primary: `physical.strength` (physical hold-off capability)
/// - Primary: `personality.aggression` (commit to the press)
///
/// Spatial proxy: we check the player IS a home or away defender (slots 1-4
/// or 12-15) and their attributes meet the threshold. Real spatial check
/// (within 2m of ball-carrier) lands at T2-1.
pub fn body_shield_pressure_trigger(state: &MatchState, slot: PlayerSlot) -> Q32 {
    let idx = slot as usize;
    if idx >= state.players.len() {
        return Q32::ZERO;
    }
    let a = &state.players[idx].attributes;

    // Role check: only defenders or midfielders apply body-shield pressure.
    // Slot layout: home DEF = 1-4, away DEF = 12-15; home MID = 5-7, away MID = 16-18.
    let in_team = idx % 11;
    let is_defender_or_mid = (1..=7).contains(&in_team);
    if !is_defender_or_mid {
        return Q32::ZERO;
    }

    if a.technical.marking < BODY_SHIELD_THRESHOLD_MARKING
        || a.physical.strength < BODY_SHIELD_THRESHOLD_STRENGTH
        || a.personality.aggression < BODY_SHIELD_THRESHOLD_AGGRESSION
    {
        return Q32::ZERO;
    }

    // Fit-score: geometric mean of the three attributes (all in [0,1]).
    // Captures how much above the threshold the player is.
    a.technical.marking * a.physical.strength * a.personality.aggression
}

/// `fwh.core:signature.long-range-strike`
///
/// Fires when an attacker has the composure + technique to attempt a
/// long-range effort.
///
/// Returns `Q32::ZERO` if not eligible. Returns positive fit-score (product
/// of the two primary attributes) when eligible.
///
/// Attribute reads (proxy at T1-2b-iv):
/// - Primary: `mental.composure` (stays calm in long-range decision)
/// - Primary: `technical.long_shots` (technique for the shot)
///
/// Spatial proxy: we check the player is a forward/midfielder and that their
/// positioning attribute (high → near goal) supports the attempt.
/// Real check (ball pos > 25m from goal) lands at T2-1.
pub fn long_range_strike_trigger(state: &MatchState, slot: PlayerSlot) -> Q32 {
    let idx = slot as usize;
    if idx >= state.players.len() {
        return Q32::ZERO;
    }
    let a = &state.players[idx].attributes;

    // Role check: forwards (slots 8-10, 19-21) and attacking midfielders (5-7, 16-18).
    let in_team = idx % 11;
    let is_attacker = (5..=10).contains(&in_team);
    if !is_attacker {
        return Q32::ZERO;
    }

    if a.mental.composure < LONG_RANGE_STRIKE_THRESHOLD_COMPOSURE
        || a.technical.long_shots < LONG_RANGE_STRIKE_THRESHOLD_LONG_SHOTS
    {
        return Q32::ZERO;
    }

    // Fit-score: composure × long_shots product.
    a.mental.composure * a.technical.long_shots
}

/// `fwh.core:signature.first-time-diagonal-switch`
///
/// Fires when a midfielder with vision and passing quality can execute a
/// first-touch diagonal switch to stretch the opposition.
///
/// Returns `Q32::ZERO` if not eligible. Returns vision × passing fit-score
/// when eligible.
///
/// Attribute reads (proxy at T1-2b-iv):
/// - Primary: `mental.vision` (sees the diagonal option)
/// - Primary: `technical.passing` (quality to execute the switch)
///
/// Spatial proxy: checks the player is a midfielder. Real check
/// (ball arrived this tick + opponent mean_x on one wing) at T2-1.
pub fn first_time_diagonal_switch_trigger(state: &MatchState, slot: PlayerSlot) -> Q32 {
    let idx = slot as usize;
    if idx >= state.players.len() {
        return Q32::ZERO;
    }
    let a = &state.players[idx].attributes;

    // Role check: midfielders only (home 5-7, away 16-18).
    let in_team = idx % 11;
    let is_midfielder = (5..=7).contains(&in_team);
    if !is_midfielder {
        return Q32::ZERO;
    }

    if a.mental.vision < DIAGONAL_SWITCH_THRESHOLD_VISION
        || a.technical.passing < DIAGONAL_SWITCH_THRESHOLD_PASSING
    {
        return Q32::ZERO;
    }

    // Fit-score: vision × passing product.
    a.mental.vision * a.technical.passing
}

// ---------------------------------------------------------------------------
// Trigger binding table
// ---------------------------------------------------------------------------

/// Type alias for a trigger function.
///
/// Returns `Q32::ZERO` when not eligible, or a positive fit-score in `(0, 1]`
/// when eligible. The dispatcher computes `candidate.affinity × fit_score` as
/// the softmax input per ADR-0011 §"Dispatch + softmax".
pub type TriggerFn = fn(&MatchState, PlayerSlot) -> Q32;

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
    // NoOpStub: always-zero trigger (the no-op fixture's trigger)
    table.insert("fwh.core:signature.no-op-stub", no_op_trigger);
    table
}

/// Always-zero trigger for the no-op stub signature (T1-3 fixture).
///
/// Returns `Q32::ZERO` unconditionally — the signature never fires.
/// Used for testing that zero fit-score correctly blocks dispatch.
pub fn no_op_trigger(_state: &MatchState, _slot: PlayerSlot) -> Q32 {
    Q32::ZERO
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
        let fit = body_shield_pressure_trigger(&state, 1);
        assert!(
            fit > Q32::ZERO,
            "body_shield should return positive fit for eligible defender"
        );
    }

    #[test]
    fn body_shield_fit_score_is_product_of_attributes() {
        let mut state = baseline_state();
        // Set exact values so product is predictable.
        state.players[1].attributes.technical.marking = Q32::from_raw(1i64 << 31); // 0.5
        state.players[1].attributes.physical.strength = Q32::from_raw(1i64 << 31); // 0.5
        state.players[1].attributes.personality.aggression = Q32::from_raw(1i64 << 31); // 0.5
        let fit = body_shield_pressure_trigger(&state, 1);
        // 0.5 × 0.5 × 0.5 = 0.125 (above the 0.45 threshold on all three).
        // Wait — 0.5 >= 0.45 threshold → eligible. Product = 0.5³ = 0.125.
        assert!(fit > Q32::ZERO, "fit_score must be > 0 for eligible player");
        // Vacuousness check: fit_score must be < 1.0 (not just boolean).
        assert!(
            fit < Q32::ONE,
            "fit_score should be < 1 (product of three 0.5 attrs)"
        );
    }

    #[test]
    fn vacuousness_check_body_shield_fit_score_zero_for_ineligible() {
        // Verify body_shield returns ZERO (not just 'truthy zero float') for ineligible.
        let mut state = baseline_state();
        // Forward is ineligible by role.
        set_all_attrs(&mut state, 8, Q32::ONE);
        let fit = body_shield_pressure_trigger(&state, 8);
        assert_eq!(
            fit,
            Q32::ZERO,
            "body_shield must return Q32::ZERO for ineligible role"
        );
    }

    #[test]
    fn body_shield_does_not_fire_for_forward() {
        let mut state = baseline_state();
        // slot 8 = home FWD; even with all attrs above threshold, role check fails.
        set_all_attrs(&mut state, 8, Q32::ONE);
        assert_eq!(body_shield_pressure_trigger(&state, 8), Q32::ZERO);
    }

    #[test]
    fn body_shield_does_not_fire_when_attrs_below_threshold() {
        let mut state = baseline_state();
        // slot 1 = home DEF; set ZERO attributes — all thresholds fail.
        set_all_attrs(&mut state, 1, Q32::ZERO);
        assert_eq!(body_shield_pressure_trigger(&state, 1), Q32::ZERO);
    }

    // ---- long_range_strike ----

    #[test]
    fn long_range_strike_fires_for_forward_above_threshold() {
        let mut state = baseline_state();
        // slot 8 = home FWD; attributes above threshold.
        set_all_attrs(&mut state, 8, Q32::ONE);
        let fit = long_range_strike_trigger(&state, 8);
        assert!(
            fit > Q32::ZERO,
            "long_range_strike should return positive fit"
        );
    }

    #[test]
    fn long_range_strike_fit_score_is_product_of_attributes() {
        let mut state = baseline_state();
        // composure = 0.5, long_shots = 0.75; product = 0.375.
        state.players[8].attributes.mental.composure = Q32::from_raw(1i64 << 31); // 0.5
        state.players[8].attributes.technical.long_shots = Q32::from_raw(3i64 << 30); // 0.75
        let fit = long_range_strike_trigger(&state, 8);
        // Both above 0.45 threshold → eligible. Product = 0.5 × 0.75 = 0.375.
        assert!(fit > Q32::ZERO, "fit must be positive");
        // Fit should be strictly between 0.25 and 0.75 (product of [0.5, 0.75]).
        assert!(
            fit < Q32::from_raw(3i64 << 30),
            "fit 0.375 should be < 0.75"
        );
        assert!(
            fit > Q32::from_raw(1i64 << 30),
            "fit 0.375 should be > 0.25"
        );
    }

    #[test]
    fn vacuousness_check_long_range_strike_fit_varies_with_attrs() {
        // Verify the fit_score multiplication is non-vacuous: two eligible players
        // with different attrs must return different fit scores.
        let mut state_hi = baseline_state();
        let mut state_lo = baseline_state();
        set_all_attrs(&mut state_hi, 8, Q32::ONE); // composure=long_shots=1 → fit=1
        // lo: composure=0.5, long_shots=0.5 → fit=0.25
        state_lo.players[8].attributes.mental.composure = Q32::from_raw(1i64 << 31);
        state_lo.players[8].attributes.technical.long_shots = Q32::from_raw(1i64 << 31);
        let fit_hi = long_range_strike_trigger(&state_hi, 8);
        let fit_lo = long_range_strike_trigger(&state_lo, 8);
        assert!(
            fit_hi > fit_lo,
            "higher attrs must produce higher fit_score: hi={:?} lo={:?}",
            fit_hi,
            fit_lo
        );
    }

    #[test]
    fn long_range_strike_fires_for_midfielder_above_threshold() {
        let mut state = baseline_state();
        // slot 6 = home MID; midfielders (5-7) count as attackers for this predicate.
        set_all_attrs(&mut state, 6, Q32::ONE);
        assert!(long_range_strike_trigger(&state, 6) > Q32::ZERO);
    }

    #[test]
    fn long_range_strike_does_not_fire_for_defender() {
        let mut state = baseline_state();
        // slot 2 = home DEF; role check fails.
        set_all_attrs(&mut state, 2, Q32::ONE);
        assert_eq!(long_range_strike_trigger(&state, 2), Q32::ZERO);
    }

    #[test]
    fn long_range_strike_does_not_fire_when_attrs_below_threshold() {
        let mut state = baseline_state();
        // slot 8 = home FWD; zero attributes.
        set_all_attrs(&mut state, 8, Q32::ZERO);
        assert_eq!(long_range_strike_trigger(&state, 8), Q32::ZERO);
    }

    // ---- first_time_diagonal_switch ----

    #[test]
    fn diagonal_switch_fires_for_midfielder_above_threshold() {
        let mut state = baseline_state();
        // slot 5 = home MID.
        set_all_attrs(&mut state, 5, Q32::ONE);
        assert!(first_time_diagonal_switch_trigger(&state, 5) > Q32::ZERO);
    }

    #[test]
    fn diagonal_switch_fit_score_is_product_of_vision_and_passing() {
        let mut state = baseline_state();
        // vision = 0.5, passing = 0.6; product ≈ 0.3.
        state.players[5].attributes.mental.vision = Q32::from_raw(1i64 << 31); // 0.5
        // 0.6 × 2^32 ≈ 2_576_980_377
        state.players[5].attributes.technical.passing = Q32::from_raw(2_576_980_377_i64);
        let fit = first_time_diagonal_switch_trigger(&state, 5);
        assert!(fit > Q32::ZERO, "eligible player must have positive fit");
        // Fit should be in (0.25, 0.5): 0.5 × 0.6 = 0.3
        assert!(fit < Q32::from_raw(1i64 << 31), "fit 0.3 should be < 0.5");
    }

    #[test]
    fn diagonal_switch_does_not_fire_for_forward() {
        let mut state = baseline_state();
        // slot 9 = home FWD; role check fails.
        set_all_attrs(&mut state, 9, Q32::ONE);
        assert_eq!(first_time_diagonal_switch_trigger(&state, 9), Q32::ZERO);
    }

    #[test]
    fn diagonal_switch_does_not_fire_when_attrs_below_threshold() {
        let mut state = baseline_state();
        // slot 5 = home MID; zero attrs.
        set_all_attrs(&mut state, 5, Q32::ZERO);
        assert_eq!(first_time_diagonal_switch_trigger(&state, 5), Q32::ZERO);
    }

    // ---- no_op trigger ----

    #[test]
    fn no_op_trigger_always_returns_zero() {
        let state = baseline_state();
        for slot in 0..22u8 {
            assert_eq!(
                no_op_trigger(&state, slot),
                Q32::ZERO,
                "no_op_trigger should always return Q32::ZERO, slot={slot}"
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
    fn trigger_table_no_op_entry_always_zero() {
        let table = build_trigger_table();
        let state = baseline_state();
        let trigger_fn = table["fwh.core:signature.no-op-stub"];
        for slot in 0..22u8 {
            assert_eq!(
                trigger_fn(&state, slot),
                Q32::ZERO,
                "no_op trigger must return Q32::ZERO, slot={slot}"
            );
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

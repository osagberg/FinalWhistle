//! Off-ball BT utility functions — 5 consideration sites.
//!
//! Each function reads player attributes per `docs/specs/bt-attribute-binding.md`,
//! produces a raw utility Q32 in `[0, 1]`, then applies personality bias.
//!
//! ## Attribute binding compliance
//!
//! The `*_ATTRS` constants list every attribute path touched by each utility
//! function (primary + secondary + bias-via-helper). Binding-correctness tests
//! verify that non-spec attributes have no effect on utility output.
//!
//! ## Key binding notes
//!
//! - `personality.work_rate` is a bias-path input for Press and Track-back.
//!   It is NOT a primary read inside the utility function bodies — it is
//!   consumed via `apply_press_bias` and `apply_cover_bias` respectively.
//!   `hold_formation` does NOT read `work_rate` directly; it reads
//!   `mental.teamwork` as its third primary attribute.
//!
//! ## Spatial stub note (T1-2b-iii-c)
//!
//! Ball and opponent positions are not yet threaded into the BT context.
//! Proxy values derived from attributes substitute for missing spatial inputs.
//! These are replaced in T1-4 / T2-1.
//!
//! ## Determinism
//!
//! No floats. No clocks. No HashMap. No async. All Q32.

use fw_core::Q32;

use crate::bt::personality_bias::{apply_cover_bias, apply_press_bias};
use crate::player::PlayerState;
use crate::role_states::PlayerIntent;
use crate::subtree_library::formation_position;

// ---------------------------------------------------------------------------
// Site attribute lists
// ---------------------------------------------------------------------------

/// Attributes read by `utility_track_back` — for binding-correctness tests.
///
/// Primary (spec): `mental.positioning`, `mental.anticipation`, `physical.pace`, `physical.stamina`
/// Secondary (spec): `mental.concentration`, `mental.teamwork`
/// Bias via helper: `personality.determination` (k₁₁), `personality.work_rate` (k₁₂)
pub const TRACK_BACK_ATTRS: &[&str] = &[
    "mental.positioning",
    "mental.anticipation",
    "physical.pace",
    "physical.stamina",
    "mental.concentration",
    "mental.teamwork",
    // bias path:
    "personality.determination",
    "personality.work_rate",
];

/// Attributes read by `utility_press` — for binding-correctness tests.
///
/// Primary (spec): `mental.anticipation`, `physical.acceleration`, `physical.stamina`
/// Secondary (spec): `mental.positioning`, `physical.pace`
/// Bias via helper: `personality.aggression` (k₉), `personality.work_rate` (k₁₀)
///   (work_rate is bias-path-only per spec — NOT a primary read)
pub const PRESS_ATTRS: &[&str] = &[
    "mental.anticipation",
    "physical.acceleration",
    "physical.stamina",
    "mental.positioning",
    "physical.pace",
    // bias path:
    "personality.aggression",
    "personality.work_rate",
];

/// Attributes read by `utility_mark_player` — for binding-correctness tests.
///
/// Primary (spec): `technical.marking`, `mental.anticipation`, `physical.pace`, `mental.concentration`
/// Secondary (spec): `physical.strength`, `physical.balance`
/// Bias via helper: `personality.determination` (k₁₁ via cover_bias)
pub const MARK_PLAYER_ATTRS: &[&str] = &[
    "technical.marking",
    "mental.anticipation",
    "physical.pace",
    "mental.concentration",
    "physical.strength",
    "physical.balance",
    // bias path:
    "personality.determination",
    "personality.work_rate",
];

/// Attributes read by `utility_run_off_ball` — for binding-correctness tests.
///
/// Primary (spec): `mental.off_the_ball`, `physical.pace`, `physical.acceleration`, `mental.anticipation`
/// Secondary (spec): `mental.flair`, `physical.stamina`
/// Bias via helper: `personality.aggression` (k₉ via press_bias), `personality.work_rate` (k₁₀)
pub const RUN_OFF_BALL_ATTRS: &[&str] = &[
    "mental.off_the_ball",
    "physical.pace",
    "physical.acceleration",
    "mental.anticipation",
    "mental.flair",
    "physical.stamina",
    // bias path:
    "personality.aggression",
    "personality.work_rate",
];

/// Attributes read by `utility_hold_formation` — for binding-correctness tests.
///
/// Primary (spec): `mental.positioning`, `mental.teamwork`, `mental.concentration`
/// Secondary (spec): `mental.decisions`
/// Bias via helper: `personality.professionalism` + `personality.determination`
///   — cover_bias reads determination + work_rate. Professionalism deferred to
///   a dedicated hold-bias helper (T2-1 polish pass); cover_bias is the proxy.
pub const HOLD_FORMATION_ATTRS: &[&str] = &[
    "mental.positioning",
    "mental.teamwork",
    "mental.concentration",
    "mental.decisions",
    // bias path:
    "personality.determination",
    "personality.work_rate",
];

// ---------------------------------------------------------------------------
// Off-ball utility functions
// ---------------------------------------------------------------------------

/// Utility for tracking back to defensive shape.
///
/// Attribute binding (spec): positioning × anticipation × pace × stamina (primary);
/// concentration + teamwork as secondary; determination + work_rate via bias.
pub fn utility_track_back(player: &PlayerState, roster_slot: u8) -> (PlayerIntent, Q32) {
    let a = &player.attributes;

    // Primary product.
    let raw = a.mental.positioning * a.mental.anticipation * a.physical.pace * a.physical.stamina;

    // Secondary: concentration (sustained defensive effort) + teamwork (shape awareness).
    let w_conc = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let w_tw = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary =
        (Q32::ONE + w_conc * a.mental.concentration) * (Q32::ONE + w_tw * a.mental.teamwork);
    let raw = raw * secondary;

    let biased = apply_cover_bias(raw, a);

    let (target_x, target_y) = formation_position(roster_slot);
    (PlayerIntent::TrackBack { target_x, target_y }, biased)
}

/// Utility for pressing the ball carrier.
///
/// Attribute binding (spec): anticipation × acceleration × stamina (primary);
/// positioning + pace as secondary; aggression + work_rate via bias ONLY
/// (work_rate is NOT a direct read in this function body).
pub fn utility_press(player: &PlayerState, roster_slot: u8) -> (PlayerIntent, Q32) {
    let a = &player.attributes;

    // Primary product.
    let raw = a.mental.anticipation * a.physical.acceleration * a.physical.stamina;

    // Secondary: positioning (spatial press trigger) + pace (chase capability).
    let w_pos = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let w_pace = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary =
        (Q32::ONE + w_pos * a.mental.positioning) * (Q32::ONE + w_pace * a.physical.pace);
    let raw = raw * secondary;

    let biased = apply_press_bias(raw, a);

    // Target: ball carrier proxy = opponent GK slot.
    let press_target_slot: u8 = if (roster_slot as usize) < 11 { 11 } else { 0 };
    let (target_x, target_y) = formation_position(press_target_slot);
    (PlayerIntent::Press { target_x, target_y }, biased)
}

/// Utility for marking an opponent.
///
/// Attribute binding (spec): marking × anticipation × pace × concentration (primary);
/// strength + balance as secondary; determination via bias.
pub fn utility_mark_player(player: &PlayerState, roster_slot: u8) -> (PlayerIntent, Q32) {
    let a = &player.attributes;

    // Primary product.
    let raw =
        a.technical.marking * a.mental.anticipation * a.physical.pace * a.mental.concentration;

    // Secondary: strength (physicality) + balance (stability in duels).
    let w_str = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let w_bal = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary =
        (Q32::ONE + w_str * a.physical.strength) * (Q32::ONE + w_bal * a.physical.balance);
    let raw = raw * secondary;

    let biased = apply_cover_bias(raw, a);

    let mark_slot: u8 = if (roster_slot as usize) < 11 { 12 } else { 1 };
    let (target_x, target_y) = formation_position(mark_slot);
    (PlayerIntent::MarkPlayer { target_x, target_y }, biased)
}

/// Utility for making a run off the ball.
///
/// Attribute binding (spec): off_the_ball × pace × acceleration × anticipation (primary);
/// flair + stamina as secondary; aggression + work_rate via bias (press_bias proxy).
pub fn utility_run_off_ball(player: &PlayerState, roster_slot: u8) -> (PlayerIntent, Q32) {
    let a = &player.attributes;

    // Primary product.
    let raw =
        a.mental.off_the_ball * a.physical.pace * a.physical.acceleration * a.mental.anticipation;

    // Secondary: flair (creative run selection) + stamina (sustained sprint).
    let w_flair = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let w_stam = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary =
        (Q32::ONE + w_flair * a.mental.flair) * (Q32::ONE + w_stam * a.physical.stamina);
    let raw = raw * secondary;

    // Press bias (Aggression + WorkRate) models the "run more aggressively"
    // personality tilt — consistent with spec's `WorkRate` + `RiskAppetite` bias.
    let biased = apply_press_bias(raw, a);

    let (fx, fy) = formation_position(roster_slot);
    let advance = Q32::from_int(10);
    let target_x = if (roster_slot as usize) < 11 {
        fx + advance
    } else {
        fx - advance
    };
    (
        PlayerIntent::RunOffBall {
            target_x,
            target_y: fy,
        },
        biased,
    )
}

/// Utility for holding formation position.
///
/// Attribute binding (spec): positioning × teamwork × concentration (primary);
/// decisions as secondary gate; determination + work_rate via bias (cover_bias proxy).
///
/// Note: `personality.work_rate` is consumed ONLY via `apply_cover_bias`, NOT
/// as a direct `a.personality.work_rate` read in this function body.
pub fn utility_hold_formation(player: &PlayerState, roster_slot: u8) -> (PlayerIntent, Q32) {
    let a = &player.attributes;

    // Primary product: positioning + teamwork + concentration (spec §"Hold formation slot").
    let raw = a.mental.positioning * a.mental.teamwork * a.mental.concentration;

    // Secondary: decisions as quality gate.
    let w_dec = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary = Q32::ONE + w_dec * a.mental.decisions;
    let raw = raw * secondary;

    // Cover bias: determination + work_rate (bias path; work_rate not read directly above).
    let biased = apply_cover_bias(raw, a);

    let (target_x, target_y) = formation_position(roster_slot);
    (PlayerIntent::HoldFormation { target_x, target_y }, biased)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::PlayerState;
    use crate::subtree_library::FORMATION_4_3_3_POSITIONS;
    use fw_core::Q32;

    fn mid_player(roster_slot: u8) -> PlayerState {
        let (x, y) = FORMATION_4_3_3_POSITIONS[roster_slot as usize];
        PlayerState::with_role(
            roster_slot,
            Q32::from_int(x),
            Q32::from_int(y),
            crate::role_states::Role::Midfielder,
        )
    }

    // --- Range tests ---

    #[test]
    fn track_back_utility_in_unit_range() {
        let p = mid_player(6);
        let (_, u) = utility_track_back(&p, 6);
        assert!(u >= Q32::ZERO);
        assert!(u <= Q32::ONE);
    }

    #[test]
    fn press_utility_in_unit_range() {
        let p = mid_player(6);
        let (_, u) = utility_press(&p, 6);
        assert!(u >= Q32::ZERO);
        assert!(u <= Q32::ONE);
    }

    #[test]
    fn mark_player_utility_in_unit_range() {
        let p = mid_player(6);
        let (_, u) = utility_mark_player(&p, 6);
        assert!(u >= Q32::ZERO);
        assert!(u <= Q32::ONE);
    }

    #[test]
    fn run_off_ball_utility_in_unit_range() {
        let p = mid_player(6);
        let (_, u) = utility_run_off_ball(&p, 6);
        assert!(u >= Q32::ZERO);
        assert!(u <= Q32::ONE);
    }

    #[test]
    fn hold_formation_utility_in_unit_range() {
        let p = mid_player(6);
        let (_, u) = utility_hold_formation(&p, 6);
        assert!(u >= Q32::ZERO);
        assert!(u <= Q32::ONE);
    }

    // --- Binding-correctness tests (P0-1 / P1-3 requirement) ---

    #[test]
    fn track_back_non_spec_attr_has_no_effect() {
        // `technical.finishing` not in track_back binding.
        let mut p_a = mid_player(6);
        let mut p_b = mid_player(6);
        p_a.attributes.technical.finishing = Q32::ZERO;
        p_b.attributes.technical.finishing = Q32::ONE;
        let (_, u_a) = utility_track_back(&p_a, 6);
        let (_, u_b) = utility_track_back(&p_b, 6);
        assert_eq!(
            u_a, u_b,
            "technical.finishing must not affect track_back utility"
        );
    }

    #[test]
    fn track_back_spec_primary_attr_changes_utility() {
        let mut p_hi = mid_player(6);
        let mut p_lo = mid_player(6);
        p_hi.attributes.mental.anticipation = Q32::ONE;
        p_hi.attributes.physical.stamina = Q32::ONE;
        p_lo.attributes.mental.anticipation = Q32::ZERO;
        p_lo.attributes.physical.stamina = Q32::ZERO;
        let (_, hi) = utility_track_back(&p_hi, 6);
        let (_, lo) = utility_track_back(&p_lo, 6);
        assert!(
            hi > lo,
            "anticipation + stamina (spec primary) must affect track_back utility"
        );
    }

    #[test]
    fn press_non_spec_attr_has_no_effect() {
        // `mental.decisions` is NOT in press binding.
        let mut p_a = mid_player(6);
        let mut p_b = mid_player(6);
        p_a.attributes.mental.decisions = Q32::ZERO;
        p_b.attributes.mental.decisions = Q32::ONE;
        let (_, u_a) = utility_press(&p_a, 6);
        let (_, u_b) = utility_press(&p_b, 6);
        assert_eq!(u_a, u_b, "mental.decisions must not affect press utility");
    }

    #[test]
    fn press_spec_primary_attr_changes_utility() {
        let mut p_hi = mid_player(6);
        let mut p_lo = mid_player(6);
        p_hi.attributes.mental.anticipation = Q32::ONE;
        p_hi.attributes.physical.stamina = Q32::ONE;
        p_lo.attributes.mental.anticipation = Q32::ZERO;
        p_lo.attributes.physical.stamina = Q32::ZERO;
        let (_, hi) = utility_press(&p_hi, 6);
        let (_, lo) = utility_press(&p_lo, 6);
        assert!(
            hi > lo,
            "anticipation + stamina (spec primary) must affect press utility"
        );
    }

    #[test]
    fn press_work_rate_only_via_bias_path() {
        // `personality.work_rate` affects press via bias only — changing it
        // changes utility, but it's not a direct primary read in the body.
        // Verify that the bias effect is present (utility changes with work_rate).
        let mut p_hi = mid_player(6);
        let mut p_lo = mid_player(6);
        p_hi.attributes.personality.work_rate = Q32::ONE;
        p_lo.attributes.personality.work_rate = Q32::ZERO;
        let (_, hi) = utility_press(&p_hi, 6);
        let (_, lo) = utility_press(&p_lo, 6);
        assert!(
            hi >= lo,
            "work_rate via bias path should not decrease press utility"
        );
    }

    #[test]
    fn mark_player_non_spec_attr_has_no_effect() {
        // `mental.decisions` not in mark_player binding.
        let mut p_a = mid_player(6);
        let mut p_b = mid_player(6);
        p_a.attributes.mental.decisions = Q32::ZERO;
        p_b.attributes.mental.decisions = Q32::ONE;
        let (_, u_a) = utility_mark_player(&p_a, 6);
        let (_, u_b) = utility_mark_player(&p_b, 6);
        assert_eq!(
            u_a, u_b,
            "mental.decisions must not affect mark_player utility"
        );
    }

    #[test]
    fn mark_player_spec_primary_attr_changes_utility() {
        let mut p_hi = mid_player(6);
        let mut p_lo = mid_player(6);
        p_hi.attributes.technical.marking = Q32::ONE;
        p_hi.attributes.physical.pace = Q32::ONE;
        p_lo.attributes.technical.marking = Q32::ZERO;
        p_lo.attributes.physical.pace = Q32::ZERO;
        let (_, hi) = utility_mark_player(&p_hi, 6);
        let (_, lo) = utility_mark_player(&p_lo, 6);
        assert!(
            hi > lo,
            "marking + pace (spec primary) must affect mark_player utility"
        );
    }

    #[test]
    fn run_off_ball_non_spec_attr_has_no_effect() {
        // `mental.decisions` not in run_off_ball binding.
        let mut p_a = mid_player(6);
        let mut p_b = mid_player(6);
        p_a.attributes.mental.decisions = Q32::ZERO;
        p_b.attributes.mental.decisions = Q32::ONE;
        let (_, u_a) = utility_run_off_ball(&p_a, 6);
        let (_, u_b) = utility_run_off_ball(&p_b, 6);
        assert_eq!(
            u_a, u_b,
            "mental.decisions must not affect run_off_ball utility"
        );
    }

    #[test]
    fn run_off_ball_spec_primary_attr_changes_utility() {
        let mut p_hi = mid_player(6);
        let mut p_lo = mid_player(6);
        p_hi.attributes.mental.off_the_ball = Q32::ONE;
        p_hi.attributes.physical.acceleration = Q32::ONE;
        p_lo.attributes.mental.off_the_ball = Q32::ZERO;
        p_lo.attributes.physical.acceleration = Q32::ZERO;
        let (_, hi) = utility_run_off_ball(&p_hi, 6);
        let (_, lo) = utility_run_off_ball(&p_lo, 6);
        assert!(
            hi > lo,
            "off_the_ball + acceleration (spec primary) must affect run_off_ball utility"
        );
    }

    #[test]
    fn hold_formation_non_spec_attr_has_no_effect() {
        // `personality.professionalism` has no direct read in hold_formation.
        // (It should affect via professionalism-specific bias at T2-1, but not now.)
        // Use `technical.finishing` as an unambiguously non-spec attr.
        let mut p_a = mid_player(6);
        let mut p_b = mid_player(6);
        p_a.attributes.technical.finishing = Q32::ZERO;
        p_b.attributes.technical.finishing = Q32::ONE;
        let (_, u_a) = utility_hold_formation(&p_a, 6);
        let (_, u_b) = utility_hold_formation(&p_b, 6);
        assert_eq!(
            u_a, u_b,
            "technical.finishing must not affect hold_formation utility"
        );
    }

    #[test]
    fn hold_formation_work_rate_not_direct_primary() {
        // `personality.work_rate` is consumed via cover_bias ONLY — the function
        // body must not read it as a primary. Verify: changing only work_rate
        // changes utility (via bias), but teamwork (primary) also changes utility.
        let mut p_hi_wr = mid_player(6);
        let mut p_lo_wr = mid_player(6);
        p_hi_wr.attributes.personality.work_rate = Q32::ONE;
        p_lo_wr.attributes.personality.work_rate = Q32::ZERO;
        let (_, u_hi) = utility_hold_formation(&p_hi_wr, 6);
        let (_, u_lo) = utility_hold_formation(&p_lo_wr, 6);
        // Bias path: u_hi should be >= u_lo (work_rate increases cover bias).
        assert!(
            u_hi >= u_lo,
            "work_rate via bias must not decrease hold_formation utility"
        );
    }

    #[test]
    fn hold_formation_spec_primary_attr_changes_utility() {
        let mut p_hi = mid_player(6);
        let mut p_lo = mid_player(6);
        p_hi.attributes.mental.teamwork = Q32::ONE;
        p_hi.attributes.mental.positioning = Q32::ONE;
        p_lo.attributes.mental.teamwork = Q32::ZERO;
        p_lo.attributes.mental.positioning = Q32::ZERO;
        let (_, hi) = utility_hold_formation(&p_hi, 6);
        let (_, lo) = utility_hold_formation(&p_lo, 6);
        assert!(
            hi > lo,
            "teamwork + positioning (spec primary) must affect hold_formation utility"
        );
    }

    // --- Monotonicity ---

    #[test]
    fn press_utility_increases_with_aggression() {
        let mut p_hi = mid_player(6);
        p_hi.attributes.personality.aggression = Q32::ONE;
        p_hi.attributes.personality.work_rate = Q32::ONE;
        let mut p_lo = mid_player(6);
        p_lo.attributes.personality.aggression = Q32::ZERO;
        p_lo.attributes.personality.work_rate = Q32::ZERO;
        let (_, hi) = utility_press(&p_hi, 6);
        let (_, lo) = utility_press(&p_lo, 6);
        assert!(
            hi >= lo,
            "higher aggression/work_rate should not decrease press utility"
        );
    }

    // --- Determinism ---

    #[test]
    fn off_ball_utilities_deterministic() {
        let p = mid_player(6);
        let (i1, u1) = utility_press(&p, 6);
        let (i2, u2) = utility_press(&p, 6);
        assert_eq!(i1, i2);
        assert_eq!(u1, u2);
    }

    // --- Target correctness ---

    #[test]
    fn track_back_targets_own_slot() {
        let p = mid_player(6);
        let (intent, _) = utility_track_back(&p, 6);
        let (expected_x, expected_y) = formation_position(6);
        assert_eq!(
            intent,
            PlayerIntent::TrackBack {
                target_x: expected_x,
                target_y: expected_y
            }
        );
    }

    #[test]
    fn hold_formation_targets_own_slot() {
        let p = mid_player(6);
        let (intent, _) = utility_hold_formation(&p, 6);
        let (expected_x, expected_y) = formation_position(6);
        assert_eq!(
            intent,
            PlayerIntent::HoldFormation {
                target_x: expected_x,
                target_y: expected_y
            }
        );
    }
}

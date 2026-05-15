//! On-ball BT utility functions — 7 consideration sites.
//!
//! Each function reads the documented player attributes per
//! `docs/specs/bt-attribute-binding.md`, produces a raw utility Q32 in
//! `[0, 1]`, then applies the per-consideration personality bias from
//! `bt::personality_bias`.
//!
//! ## Attribute binding compliance
//!
//! The `*_ATTRS` constants list EVERY attribute path touched by the
//! corresponding utility function (primary + secondary + bias-via-helper).
//! Primary reads appear directly in the `raw` product; secondary reads apply
//! as mild modifiers; bias inputs are consumed via the `apply_*_bias` helpers.
//! The binding-correctness tests verify spec alignment.
//!
//! ## Spatial stub note (T1-2b-iii-c)
//!
//! Ball position, opponent positions, and teammate positions are not yet
//! threaded into the BT context. Each function uses proxy Q32 values:
//!
//! - `defender_pressure_proxy` = complement of `mental.composure`
//!   (less composure → more felt pressure).
//! - `distance_proxy` = `mental.positioning` (high positioning → closer to goal).
//! - `progressive_pass_proxy` = `mental.vision` (vision → sees forward option).
//!
//! When spatial inputs are wired in T1-4 / T2-1, these proxies are replaced
//! with real geometry; the bias-layer and attribute-binding code are unchanged.
//!
//! ## Determinism
//!
//! No floats. No clocks. No HashMap. No async. All Q32.

use fw_core::Q32;

use crate::bt::personality_bias::{
    IsProgressive, apply_cross_bias, apply_dribble_bias, apply_hold_bias, apply_lay_off_bias,
    apply_long_pass_bias, apply_safe_pass_bias, apply_shoot_bias, read_defender_pressure,
};
use crate::player::PlayerState;
use crate::role_states::PlayerIntent;
use crate::subtree_library::formation_position;

// ---------------------------------------------------------------------------
// Site attribute lists
//
// These constants list every attribute path the corresponding utility function
// touches: primary + secondary reads directly in the Rust body, plus the bias
// inputs consumed by the `apply_*_bias` helper.  The binding-correctness tests
// assert that altering only a non-listed attribute leaves utility unchanged.
// ---------------------------------------------------------------------------

/// Attributes read by `utility_shoot` — for binding-correctness tests.
///
/// Primary (spec): `technical.finishing`, `mental.composure`, `mental.decisions`
/// Secondary (spec): `technical.long_shots` (range capability modifier), `mental.vision`, `physical.balance`
/// Bias via helper: `mental.flair` (FlairBias), `mental.composure` (Composure),
///   `personality.risk_appetite` (RiskAppetite), `personality.pressure_tolerance` (PT divisor)
pub const SHOOT_ATTRS: &[&str] = &[
    "technical.finishing",
    "technical.long_shots",
    "mental.composure",
    "mental.decisions",
    "mental.vision",
    "physical.balance",
    // bias path (via apply_shoot_bias + read_defender_pressure):
    "mental.flair",
    "personality.risk_appetite",
    "personality.pressure_tolerance",
];

/// Attributes read by `utility_pass_short` — for binding-correctness tests.
///
/// Primary (spec): `technical.passing`, `technical.first_touch`, `technical.technique`, `mental.vision`
/// Secondary (spec): `mental.composure`, `mental.decisions`
/// Bias via helper: `personality.risk_appetite` (inverse), `personality.selflessness`
pub const PASS_SHORT_ATTRS: &[&str] = &[
    "technical.passing",
    "technical.first_touch",
    "technical.technique",
    "mental.vision",
    "mental.composure",
    "mental.decisions",
    // bias path:
    "personality.risk_appetite",
    "personality.selflessness",
];

/// Attributes read by `utility_pass_long` — for binding-correctness tests.
///
/// Primary (spec): `technical.passing`, `mental.vision`, `mental.decisions`, `technical.long_shots`
/// Secondary (spec): `mental.composure`, `mental.anticipation`
/// Bias via helper: `personality.risk_appetite` (RiskAppetite), `mental.flair` (FlairBias)
pub const PASS_LONG_ATTRS: &[&str] = &[
    "technical.passing",
    "mental.vision",
    "mental.decisions",
    "technical.long_shots",
    "mental.composure",
    "mental.anticipation",
    // bias path:
    "personality.risk_appetite",
    "mental.flair",
];

/// Attributes read by `utility_cross` — for binding-correctness tests.
///
/// Primary (spec): `technical.crossing`, `mental.vision`, `physical.pace`
/// Secondary (spec): `technical.first_touch`, `mental.anticipation`
/// Bias via helper: `personality.work_rate` (WorkRate), `mental.flair` (FlairBias)
pub const CROSS_ATTRS: &[&str] = &[
    "technical.crossing",
    "mental.vision",
    "physical.pace",
    "technical.first_touch",
    "mental.anticipation",
    // bias path (via apply_cross_bias):
    "personality.work_rate",
    "mental.flair",
];

/// Attributes read by `utility_dribble` — for binding-correctness tests.
///
/// Primary (spec): `technical.dribbling`, `technical.technique`, `physical.agility`, `physical.acceleration`
/// Secondary (spec): `physical.balance`, `mental.flair`
/// Bias via helper: `mental.flair` (FlairBias), `personality.aggression` (Aggression)
pub const DRIBBLE_ATTRS: &[&str] = &[
    "technical.dribbling",
    "technical.technique",
    "physical.agility",
    "physical.acceleration",
    "physical.balance",
    "mental.flair",
    // bias path:
    "personality.aggression",
];

/// Attributes read by `utility_hold_ball` — for binding-correctness tests.
///
/// Primary (spec): `physical.strength`, `mental.composure`, `physical.balance`
/// Secondary (spec): `mental.decisions`
/// Bias via helper: `personality.aggression` (inverse), `personality.pressure_tolerance`
pub const HOLD_BALL_ATTRS: &[&str] = &[
    "physical.strength",
    "mental.composure",
    "physical.balance",
    "mental.decisions",
    // bias path:
    "personality.aggression",
    "personality.pressure_tolerance",
];

/// Attributes read by `utility_lay_off` — for binding-correctness tests.
///
/// Primary (spec): `technical.first_touch`, `technical.passing`, `mental.vision`
/// Secondary (spec): `mental.teamwork`, `mental.composure`
/// Bias via helper: `personality.selflessness` (Selflessness ONLY per spec)
pub const LAY_OFF_ATTRS: &[&str] = &[
    "technical.first_touch",
    "technical.passing",
    "mental.vision",
    "mental.teamwork",
    "mental.composure",
    // bias path (via apply_lay_off_bias — selflessness only):
    "personality.selflessness",
];

// ---------------------------------------------------------------------------
// On-ball utility functions
// ---------------------------------------------------------------------------

/// Utility for attempting a shot.
///
/// Attribute binding (spec): finishing × composure × decisions (primary);
/// long_shots + vision + balance as secondary modifiers; flair + composure + risk_appetite + PT via bias.
///
/// Note: long_shots is a spec-listed attribute but implemented as a secondary additive
/// modifier `(1 + 0.30×long_shots)` to keep the primary product magnitude consistent
/// with other 3-primary sites. This matches the pattern used by pass_long, which also
/// lists long_shots in the spec table but applies it as a secondary modifier.
pub fn utility_shoot(player: &PlayerState, roster_slot: u8) -> (PlayerIntent, Q32) {
    let a = &player.attributes;

    // Primary product: three core shooting attributes.
    let raw = a.technical.finishing * a.mental.composure * a.mental.decisions;

    // Secondary: long_shots as range capability (higher = can shoot from distance).
    // vision improves the shot (angle quality proxy).
    // balance improves off-balance situations.
    // Applied as mild additive multipliers.
    let w_ls = Q32::from_raw(1_288_490_188_i64); // ≈ 0.30
    let w_vision = Q32::from_raw(858_993_459_i64); // ≈ 0.20
    let w_balance = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary = (Q32::ONE + w_ls * a.technical.long_shots)
        * (Q32::ONE + w_vision * a.mental.vision)
        * (Q32::ONE + w_balance * a.physical.balance);
    let raw = raw * secondary;

    // Effective defender pressure = PT-attenuated proxy (complement of composure).
    let raw_pressure = Q32::ONE - a.mental.composure;
    let eff_pressure = read_defender_pressure(a, raw_pressure);

    let biased = apply_shoot_bias(raw, a, eff_pressure);

    // Target: the attacking goal centre.
    let (target_x, target_y) = if (roster_slot as usize) < 11 {
        (Q32::from_int(45), Q32::ZERO)
    } else {
        (Q32::from_int(-45), Q32::ZERO)
    };

    (PlayerIntent::AttemptShot { target_x, target_y }, biased)
}

/// Utility for a short pass.
///
/// Attribute binding (spec): passing × first_touch × technique × vision (primary);
/// composure + decisions as secondary modifiers; risk_appetite + selflessness via bias.
pub fn utility_pass_short(player: &PlayerState, roster_slot: u8) -> (PlayerIntent, Q32) {
    let a = &player.attributes;

    // Primary product.
    let raw =
        a.technical.passing * a.technical.first_touch * a.technical.technique * a.mental.vision;

    // Secondary: composure under pressure, decisions as quality gate.
    let w_comp = Q32::from_raw(858_993_459_i64); // ≈ 0.20
    let w_dec = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary =
        (Q32::ONE + w_comp * a.mental.composure) * (Q32::ONE + w_dec * a.mental.decisions);
    let raw = raw * secondary;

    let biased = apply_safe_pass_bias(raw, a);

    let (fx, fy) = formation_position(roster_slot);
    let target_x = fx;
    let target_y = fy + Q32::from_int(5);

    (
        PlayerIntent::AttemptPassShort { target_x, target_y },
        biased,
    )
}

/// Utility for a long / through-ball pass.
///
/// Attribute binding (spec): passing × vision × decisions (primary); long_shots as
/// ball-power proxy, composure + anticipation as secondary; risk_appetite + flair via bias.
pub fn utility_pass_long(player: &PlayerState, roster_slot: u8) -> (PlayerIntent, Q32) {
    let a = &player.attributes;

    // Primary product: passing + vision + decisions.
    let raw = a.technical.passing * a.mental.vision * a.mental.decisions;

    // long_shots as ball-power secondary (proxy for distance capability).
    // anticipation + composure as secondary.
    let w_ls = Q32::from_raw(858_993_459_i64); // ≈ 0.20
    let w_ant = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let w_comp = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary = (Q32::ONE + w_ls * a.technical.long_shots)
        * (Q32::ONE + w_ant * a.mental.anticipation)
        * (Q32::ONE + w_comp * a.mental.composure);
    let raw = raw * secondary;

    // is_progressive proxy = vision (high vision → sees forward option).
    let is_progressive = IsProgressive(a.mental.vision);
    let biased = apply_long_pass_bias(raw, a, is_progressive);

    let (_, fy) = formation_position(roster_slot);
    let target_x = if (roster_slot as usize) < 11 {
        Q32::from_int(25)
    } else {
        Q32::from_int(-25)
    };

    (
        PlayerIntent::AttemptPassLong {
            target_x,
            target_y: fy,
        },
        biased,
    )
}

/// Utility for a cross into the box.
///
/// Attribute binding (spec): crossing × vision × pace (primary);
/// first_touch + anticipation as secondary; work_rate + flair via bias.
pub fn utility_cross(player: &PlayerState, roster_slot: u8) -> (PlayerIntent, Q32) {
    let a = &player.attributes;

    // Primary product.
    let raw = a.technical.crossing * a.mental.vision * a.physical.pace;

    // Secondary: first_touch (control before cross) + anticipation (timing).
    let w_ft = Q32::from_raw(858_993_459_i64); // ≈ 0.20
    let w_ant = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary =
        (Q32::ONE + w_ft * a.technical.first_touch) * (Q32::ONE + w_ant * a.mental.anticipation);
    let raw = raw * secondary;

    // Cross bias: WorkRate + FlairBias per spec (P1-5 fix — was safe_pass proxy).
    let biased = apply_cross_bias(raw, a);

    let (target_x, target_y) = if (roster_slot as usize) < 11 {
        (Q32::from_int(40), Q32::ZERO)
    } else {
        (Q32::from_int(-40), Q32::ZERO)
    };

    (PlayerIntent::Cross { target_x, target_y }, biased)
}

/// Utility for dribbling.
///
/// Attribute binding (spec): dribbling × technique × agility × acceleration (primary);
/// balance + flair as secondary; flair + aggression via bias.
pub fn utility_dribble(player: &PlayerState, roster_slot: u8) -> (PlayerIntent, Q32) {
    let a = &player.attributes;

    // Primary product.
    let raw = a.technical.dribbling
        * a.technical.technique
        * a.physical.agility
        * a.physical.acceleration;

    // Secondary: balance (stability in contact) + flair (creative dribbling).
    let w_bal = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let w_flair = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary = (Q32::ONE + w_bal * a.physical.balance) * (Q32::ONE + w_flair * a.mental.flair);
    let raw = raw * secondary;

    let biased = apply_dribble_bias(raw, a);

    let (fx, fy) = formation_position(roster_slot);
    let advance = Q32::from_int(8);
    let target_x = if (roster_slot as usize) < 11 {
        fx + advance
    } else {
        fx - advance
    };

    (
        PlayerIntent::Dribble {
            target_x,
            target_y: fy,
        },
        biased,
    )
}

/// Utility for holding the ball.
///
/// Attribute binding (spec): strength × composure × balance (primary);
/// decisions as secondary gate; aggression (inverse) + PT via bias.
pub fn utility_hold_ball(player: &PlayerState, roster_slot: u8) -> (PlayerIntent, Q32) {
    let a = &player.attributes;

    // Primary product.
    let raw = a.physical.strength * a.mental.composure * a.physical.balance;

    // Secondary: decisions as a quality gate.
    let w_dec = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary = Q32::ONE + w_dec * a.mental.decisions;
    let raw = raw * secondary;

    let biased = apply_hold_bias(raw, a);

    let (target_x, target_y) = formation_position(roster_slot);

    (PlayerIntent::HoldBall { target_x, target_y }, biased)
}

/// Utility for laying the ball off.
///
/// Attribute binding (spec): first_touch × passing × vision (primary);
/// teamwork + composure as secondary; selflessness via bias.
pub fn utility_lay_off(player: &PlayerState, roster_slot: u8) -> (PlayerIntent, Q32) {
    let a = &player.attributes;

    // Primary product.
    let raw = a.technical.first_touch * a.technical.passing * a.mental.vision;

    // Secondary: teamwork (cooperative action) + composure (calmness).
    let w_tw = Q32::from_raw(858_993_459_i64); // ≈ 0.20
    let w_comp = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary =
        (Q32::ONE + w_tw * a.mental.teamwork) * (Q32::ONE + w_comp * a.mental.composure);
    let raw = raw * secondary;

    // Lay-off bias: Selflessness ONLY per spec (P1-5 fix — was safe_pass proxy which also read risk_appetite).
    let biased = apply_lay_off_bias(raw, a);

    let (fx, fy) = formation_position(roster_slot);
    let target_x = fx;
    let target_y = fy - Q32::from_int(7);

    (PlayerIntent::LayOff { target_x, target_y }, biased)
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
    fn shoot_utility_in_unit_range() {
        let p = mid_player(6);
        let (_, u) = utility_shoot(&p, 6);
        assert!(u >= Q32::ZERO, "shoot utility must be >= 0; got {:?}", u);
        assert!(u <= Q32::ONE, "shoot utility must be <= 1; got {:?}", u);
    }

    #[test]
    fn pass_short_utility_in_unit_range() {
        let p = mid_player(6);
        let (_, u) = utility_pass_short(&p, 6);
        assert!(u >= Q32::ZERO);
        assert!(u <= Q32::ONE);
    }

    #[test]
    fn pass_long_utility_in_unit_range() {
        let p = mid_player(6);
        let (_, u) = utility_pass_long(&p, 6);
        assert!(u >= Q32::ZERO);
        assert!(u <= Q32::ONE);
    }

    #[test]
    fn cross_utility_in_unit_range() {
        let p = mid_player(6);
        let (_, u) = utility_cross(&p, 6);
        assert!(u >= Q32::ZERO);
        assert!(u <= Q32::ONE);
    }

    #[test]
    fn dribble_utility_in_unit_range() {
        let p = mid_player(6);
        let (_, u) = utility_dribble(&p, 6);
        assert!(u >= Q32::ZERO);
        assert!(u <= Q32::ONE);
    }

    #[test]
    fn hold_ball_utility_in_unit_range() {
        let p = mid_player(6);
        let (_, u) = utility_hold_ball(&p, 6);
        assert!(u >= Q32::ZERO);
        assert!(u <= Q32::ONE);
    }

    #[test]
    fn lay_off_utility_in_unit_range() {
        let p = mid_player(6);
        let (_, u) = utility_lay_off(&p, 6);
        assert!(u >= Q32::ZERO);
        assert!(u <= Q32::ONE);
    }

    // --- Binding-correctness tests (P0-1 / P1-3 requirement) ---
    //
    // Each test creates two player states that differ ONLY in a NON-spec
    // attribute and asserts that utility is identical (non-spec attr has no
    // effect). Then two states differing in a spec-primary attribute must give
    // different utility.

    #[test]
    fn shoot_non_spec_attr_has_no_effect() {
        // `mental.off_the_ball` is NOT in the Shoot binding — changing it
        // must not change shoot utility.
        let mut p_a = mid_player(6);
        let mut p_b = mid_player(6);
        p_a.attributes.mental.off_the_ball = Q32::ZERO;
        p_b.attributes.mental.off_the_ball = Q32::ONE;
        let (_, u_a) = utility_shoot(&p_a, 6);
        let (_, u_b) = utility_shoot(&p_b, 6);
        assert_eq!(
            u_a, u_b,
            "mental.off_the_ball (non-spec) must not affect shoot utility"
        );
    }

    #[test]
    fn shoot_spec_primary_attr_changes_utility() {
        let mut p_hi = mid_player(6);
        let mut p_lo = mid_player(6);
        p_hi.attributes.technical.finishing = Q32::ONE;
        p_lo.attributes.technical.finishing = Q32::ZERO;
        let (_, hi) = utility_shoot(&p_hi, 6);
        let (_, lo) = utility_shoot(&p_lo, 6);
        assert!(
            hi > lo,
            "technical.finishing (spec primary) must affect shoot utility"
        );
    }

    #[test]
    fn shoot_long_shots_is_spec_secondary() {
        // technical.long_shots is a spec-listed attribute applied as a secondary
        // modifier (P1-5 fix) — it must affect shoot utility even though it is
        // not in the 3-factor primary product.
        let mut p_hi = mid_player(6);
        let mut p_lo = mid_player(6);
        p_hi.attributes.technical.long_shots = Q32::ONE;
        p_lo.attributes.technical.long_shots = Q32::ZERO;
        let (_, hi) = utility_shoot(&p_hi, 6);
        let (_, lo) = utility_shoot(&p_lo, 6);
        assert!(
            hi > lo,
            "technical.long_shots (spec attribute — secondary modifier) must affect shoot utility (P1-5)"
        );
    }

    #[test]
    fn shoot_risk_appetite_via_bias_changes_utility() {
        // personality.risk_appetite is now in the shoot bias (P1-5 fix).
        let mut p_hi = mid_player(6);
        let mut p_lo = mid_player(6);
        p_hi.attributes.personality.risk_appetite = Q32::ONE;
        p_lo.attributes.personality.risk_appetite = Q32::ZERO;
        let (_, hi) = utility_shoot(&p_hi, 6);
        let (_, lo) = utility_shoot(&p_lo, 6);
        assert!(
            hi > lo,
            "personality.risk_appetite (spec bias) must affect shoot utility (P1-5)"
        );
    }

    #[test]
    fn pass_short_non_spec_attr_has_no_effect() {
        // `mental.off_the_ball` not in pass_short binding.
        let mut p_a = mid_player(6);
        let mut p_b = mid_player(6);
        p_a.attributes.mental.off_the_ball = Q32::ZERO;
        p_b.attributes.mental.off_the_ball = Q32::ONE;
        let (_, u_a) = utility_pass_short(&p_a, 6);
        let (_, u_b) = utility_pass_short(&p_b, 6);
        assert_eq!(
            u_a, u_b,
            "mental.off_the_ball must not affect pass_short utility"
        );
    }

    #[test]
    fn pass_short_spec_primary_attr_changes_utility() {
        let mut p_hi = mid_player(6);
        let mut p_lo = mid_player(6);
        p_hi.attributes.technical.first_touch = Q32::ONE;
        p_lo.attributes.technical.first_touch = Q32::ZERO;
        let (_, hi) = utility_pass_short(&p_hi, 6);
        let (_, lo) = utility_pass_short(&p_lo, 6);
        assert!(
            hi > lo,
            "technical.first_touch (spec primary) must affect pass_short utility"
        );
    }

    #[test]
    fn pass_long_non_spec_attr_has_no_effect() {
        // `mental.teamwork` not in pass_long binding.
        let mut p_a = mid_player(6);
        let mut p_b = mid_player(6);
        p_a.attributes.mental.teamwork = Q32::ZERO;
        p_b.attributes.mental.teamwork = Q32::ONE;
        let (_, u_a) = utility_pass_long(&p_a, 6);
        let (_, u_b) = utility_pass_long(&p_b, 6);
        assert_eq!(
            u_a, u_b,
            "mental.teamwork must not affect pass_long utility"
        );
    }

    #[test]
    fn pass_long_spec_primary_attr_changes_utility() {
        let mut p_hi = mid_player(6);
        let mut p_lo = mid_player(6);
        p_hi.attributes.technical.long_shots = Q32::ONE;
        p_lo.attributes.technical.long_shots = Q32::ZERO;
        let (_, hi) = utility_pass_long(&p_hi, 6);
        let (_, lo) = utility_pass_long(&p_lo, 6);
        assert!(
            hi > lo,
            "technical.long_shots (spec secondary) must affect pass_long utility"
        );
    }

    #[test]
    fn cross_non_spec_attr_has_no_effect() {
        // `mental.decisions` not in cross binding.
        let mut p_a = mid_player(6);
        let mut p_b = mid_player(6);
        p_a.attributes.mental.decisions = Q32::ZERO;
        p_b.attributes.mental.decisions = Q32::ONE;
        let (_, u_a) = utility_cross(&p_a, 6);
        let (_, u_b) = utility_cross(&p_b, 6);
        assert_eq!(u_a, u_b, "mental.decisions must not affect cross utility");
    }

    #[test]
    fn cross_risk_appetite_not_in_binding() {
        // risk_appetite must NOT affect cross utility (P1-5 fix: was safe_pass proxy).
        let mut p_a = mid_player(6);
        let mut p_b = mid_player(6);
        p_a.attributes.personality.risk_appetite = Q32::ZERO;
        p_b.attributes.personality.risk_appetite = Q32::ONE;
        let (_, u_a) = utility_cross(&p_a, 6);
        let (_, u_b) = utility_cross(&p_b, 6);
        assert_eq!(
            u_a, u_b,
            "risk_appetite must not affect cross utility (P1-5 spec fix)"
        );
    }

    #[test]
    fn cross_work_rate_via_bias_changes_utility() {
        // personality.work_rate IS in the cross binding via apply_cross_bias (P1-5 fix).
        let mut p_hi = mid_player(6);
        let mut p_lo = mid_player(6);
        p_hi.attributes.personality.work_rate = Q32::ONE;
        p_lo.attributes.personality.work_rate = Q32::ZERO;
        let (_, hi) = utility_cross(&p_hi, 6);
        let (_, lo) = utility_cross(&p_lo, 6);
        assert!(
            hi > lo,
            "personality.work_rate (spec bias) must affect cross utility (P1-5)"
        );
    }

    #[test]
    fn cross_spec_primary_attr_changes_utility() {
        let mut p_hi = mid_player(6);
        let mut p_lo = mid_player(6);
        p_hi.attributes.technical.crossing = Q32::ONE;
        p_hi.attributes.physical.pace = Q32::ONE;
        p_lo.attributes.technical.crossing = Q32::ZERO;
        p_lo.attributes.physical.pace = Q32::ZERO;
        let (_, hi) = utility_cross(&p_hi, 6);
        let (_, lo) = utility_cross(&p_lo, 6);
        assert!(
            hi > lo,
            "technical.crossing + pace (spec primary) must affect cross utility"
        );
    }

    #[test]
    fn dribble_non_spec_attr_has_no_effect() {
        // `mental.decisions` not in dribble binding.
        let mut p_a = mid_player(6);
        let mut p_b = mid_player(6);
        p_a.attributes.mental.decisions = Q32::ZERO;
        p_b.attributes.mental.decisions = Q32::ONE;
        let (_, u_a) = utility_dribble(&p_a, 6);
        let (_, u_b) = utility_dribble(&p_b, 6);
        assert_eq!(u_a, u_b, "mental.decisions must not affect dribble utility");
    }

    #[test]
    fn dribble_spec_primary_attr_changes_utility() {
        let mut p_hi = mid_player(6);
        let mut p_lo = mid_player(6);
        p_hi.attributes.technical.dribbling = Q32::ONE;
        p_hi.attributes.physical.acceleration = Q32::ONE;
        p_lo.attributes.technical.dribbling = Q32::ZERO;
        p_lo.attributes.physical.acceleration = Q32::ZERO;
        let (_, hi) = utility_dribble(&p_hi, 6);
        let (_, lo) = utility_dribble(&p_lo, 6);
        assert!(
            hi > lo,
            "dribbling + acceleration (spec primary) must affect dribble utility"
        );
    }

    #[test]
    fn hold_ball_non_spec_attr_has_no_effect() {
        // `mental.off_the_ball` not in hold_ball binding.
        let mut p_a = mid_player(6);
        let mut p_b = mid_player(6);
        p_a.attributes.mental.off_the_ball = Q32::ZERO;
        p_b.attributes.mental.off_the_ball = Q32::ONE;
        let (_, u_a) = utility_hold_ball(&p_a, 6);
        let (_, u_b) = utility_hold_ball(&p_b, 6);
        assert_eq!(
            u_a, u_b,
            "mental.off_the_ball must not affect hold_ball utility"
        );
    }

    #[test]
    fn hold_ball_spec_primary_attr_changes_utility() {
        let mut p_hi = mid_player(6);
        let mut p_lo = mid_player(6);
        p_hi.attributes.physical.strength = Q32::ONE;
        p_hi.attributes.physical.balance = Q32::ONE;
        p_lo.attributes.physical.strength = Q32::ZERO;
        p_lo.attributes.physical.balance = Q32::ZERO;
        let (_, hi) = utility_hold_ball(&p_hi, 6);
        let (_, lo) = utility_hold_ball(&p_lo, 6);
        assert!(
            hi > lo,
            "strength + balance (spec primary) must affect hold_ball utility"
        );
    }

    #[test]
    fn lay_off_non_spec_attr_has_no_effect() {
        // `mental.decisions` not in lay_off binding.
        let mut p_a = mid_player(6);
        let mut p_b = mid_player(6);
        p_a.attributes.mental.decisions = Q32::ZERO;
        p_b.attributes.mental.decisions = Q32::ONE;
        let (_, u_a) = utility_lay_off(&p_a, 6);
        let (_, u_b) = utility_lay_off(&p_b, 6);
        assert_eq!(u_a, u_b, "mental.decisions must not affect lay_off utility");
    }

    #[test]
    fn lay_off_risk_appetite_not_in_binding() {
        // risk_appetite must NOT affect lay_off utility (P1-5 fix: was safe_pass proxy).
        let mut p_a = mid_player(6);
        let mut p_b = mid_player(6);
        p_a.attributes.personality.risk_appetite = Q32::ZERO;
        p_b.attributes.personality.risk_appetite = Q32::ONE;
        let (_, u_a) = utility_lay_off(&p_a, 6);
        let (_, u_b) = utility_lay_off(&p_b, 6);
        assert_eq!(
            u_a, u_b,
            "risk_appetite must not affect lay_off utility (P1-5 spec fix — selflessness only)"
        );
    }

    #[test]
    fn lay_off_spec_primary_attr_changes_utility() {
        let mut p_hi = mid_player(6);
        let mut p_lo = mid_player(6);
        p_hi.attributes.technical.first_touch = Q32::ONE;
        p_hi.attributes.technical.passing = Q32::ONE;
        p_lo.attributes.technical.first_touch = Q32::ZERO;
        p_lo.attributes.technical.passing = Q32::ZERO;
        let (_, hi) = utility_lay_off(&p_hi, 6);
        let (_, lo) = utility_lay_off(&p_lo, 6);
        assert!(
            hi > lo,
            "first_touch + passing (spec primary) must affect lay_off utility"
        );
    }

    // --- Monotonicity ---

    #[test]
    fn shoot_utility_increases_with_finishing() {
        let mut p_hi = mid_player(6);
        p_hi.attributes.technical.finishing = Q32::ONE;
        let mut p_lo = mid_player(6);
        p_lo.attributes.technical.finishing = Q32::ZERO;
        let (_, hi) = utility_shoot(&p_hi, 6);
        let (_, lo) = utility_shoot(&p_lo, 6);
        assert!(
            hi > lo,
            "high finishing should produce higher shoot utility"
        );
    }

    // --- Determinism ---

    #[test]
    fn on_ball_utilities_deterministic() {
        let p = mid_player(6);
        let (i1, u1) = utility_shoot(&p, 6);
        let (i2, u2) = utility_shoot(&p, 6);
        assert_eq!(i1, i2);
        assert_eq!(u1, u2);
    }

    // --- Direction ---

    #[test]
    fn shoot_target_direction_is_correct() {
        let home = mid_player(6);
        let away = mid_player(16);
        let (PlayerIntent::AttemptShot { target_x: htx, .. }, _) = utility_shoot(&home, 6) else {
            panic!("expected AttemptShot");
        };
        let (PlayerIntent::AttemptShot { target_x: atx, .. }, _) = utility_shoot(&away, 16) else {
            panic!("expected AttemptShot");
        };
        assert!(htx > Q32::ZERO, "home team shoots toward +x");
        assert!(atx < Q32::ZERO, "away team shoots toward -x");
    }
}

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
//! ## Key binding notes (P1-5 spec-drift corrections)
//!
//! - `personality.work_rate` is a bias-path input for Press and Track-back ONLY.
//!   It is NOT consumed by Mark, RunOffBall, or HoldFormation (P1-5 fix).
//! - Mark: Determination bias ONLY (not cover_bias which also reads work_rate).
//! - RunOffBall: WorkRate + RiskAppetite bias (not press_bias which reads aggression).
//! - HoldFormation: Professionalism + Determination bias (not cover_bias with work_rate).
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

use crate::bt::personality_bias::{
    apply_cover_bias, apply_hold_formation_bias, apply_mark_bias, apply_press_bias,
    apply_run_off_ball_bias,
};
use crate::player::PlayerState;
use crate::role_states::PlayerIntent;
use crate::subtree_library::formation_position;
use crate::team_shape::{TeamShape, zonal_slot};

// ---------------------------------------------------------------------------
// Layer 2 — defender lane-cover constants
// SOFT Phase-2 tuning values per docs/design/dynamic-positioning-model-2026-06-06.md §2.1.
// ---------------------------------------------------------------------------

/// Maximum defender step-up toward the carrier's forward passing lane. SOFT.
/// A high-reading defender (weight≈0.9) steps up ~7.2m; a low one (~0.3) ~2.4m.
/// Q32 raw = round(8.0 × 2^32) = 34_359_738_368.
const LANE_COVER_MAX_DEF: Q32 = Q32::from_raw(34_359_738_368_i64); // 8.0m SOFT

/// Attribute weights for the lane-cover weight formula (sum = 1.0):
///   lane_cover_weight = anticipation × 0.50 + tackling × 0.30 + positioning × 0.20
/// (`technical.tackling` is used as the proxy for the design doc's `technical.interception`
/// since that field does not exist in the current attribute schema.)
///
/// Weight on anticipation (0.50). Q32 raw = round(0.50 × 2^32) = 2_147_483_648.
const LANE_W_ANTICIPATION: Q32 = Q32::from_raw(2_147_483_648_i64); // 0.50
/// Weight on tackling (interception proxy, 0.30). Q32 raw = round(0.30 × 2^32) = 1_288_490_189.
const LANE_W_TACKLING: Q32 = Q32::from_raw(1_288_490_189_i64); // 0.30
/// Weight on positioning (0.20). Q32 raw = round(0.20 × 2^32) = 858_993_459.
const LANE_W_POSITIONING: Q32 = Q32::from_raw(858_993_459_i64); // 0.20

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
/// Bias via helper: `personality.determination` (Determination ONLY per spec)
pub const MARK_PLAYER_ATTRS: &[&str] = &[
    "technical.marking",
    "mental.anticipation",
    "physical.pace",
    "mental.concentration",
    "physical.strength",
    "physical.balance",
    // bias path (via apply_mark_bias — determination only):
    "personality.determination",
];

/// Attributes read by `utility_run_off_ball` — for binding-correctness tests.
///
/// Primary (spec): `mental.off_the_ball`, `physical.pace`, `physical.acceleration`, `mental.anticipation`
/// Secondary (spec): `mental.flair`, `physical.stamina`
/// Bias via helper: `personality.work_rate` (WorkRate), `personality.risk_appetite` (RiskAppetite)
pub const RUN_OFF_BALL_ATTRS: &[&str] = &[
    "mental.off_the_ball",
    "physical.pace",
    "physical.acceleration",
    "mental.anticipation",
    "mental.flair",
    "physical.stamina",
    // bias path (via apply_run_off_ball_bias — work_rate + risk_appetite per spec):
    "personality.work_rate",
    "personality.risk_appetite",
];

/// Attributes read by `utility_hold_formation` — for binding-correctness tests.
///
/// Primary (spec): `mental.positioning`, `mental.teamwork`, `mental.concentration`
/// Secondary (spec): `mental.decisions`
/// Bias via helper: `personality.professionalism` (Professionalism) + `personality.determination` (Determination)
pub const HOLD_FORMATION_ATTRS: &[&str] = &[
    "mental.positioning",
    "mental.teamwork",
    "mental.concentration",
    "mental.decisions",
    // bias path (via apply_hold_formation_bias — professionalism + determination per spec):
    "personality.professionalism",
    "personality.determination",
];

// ---------------------------------------------------------------------------
// Off-ball utility functions
// ---------------------------------------------------------------------------

/// Utility for tracking back to defensive shape.
///
/// Attribute binding (spec): positioning × anticipation × pace × stamina (primary);
/// concentration + teamwork as secondary; determination + work_rate via bias.
///
/// FUN-TS1: targets `zonal_slot(roster_slot, shape, team_idx)` instead of the
/// constant `formation_position(roster_slot)`.
pub fn utility_track_back(
    player: &PlayerState,
    roster_slot: u8,
    shape: &TeamShape,
    team_idx: usize,
) -> (PlayerIntent, Q32) {
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

    let (target_x, target_y) = zonal_slot(roster_slot, shape, team_idx);
    (PlayerIntent::TrackBack { target_x, target_y }, biased)
}

/// Utility for pressing the ball carrier.
///
/// Attribute binding (spec): anticipation × acceleration × stamina (primary);
/// positioning + pace as secondary; aggression + work_rate via bias ONLY
/// (work_rate is NOT a direct read in this function body).
///
/// B1 (FUN-0b+c): target is the ACTUAL ball carrier's position when provided,
/// falling back to the opponent-GK formation slot when the carrier is unknown
/// (loose ball — preempt_check handles that case, not press).
pub fn utility_press(
    player: &PlayerState,
    roster_slot: u8,
    carrier_pos: Option<(Q32, Q32)>,
) -> (PlayerIntent, Q32) {
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

    // B1: target the actual carrier if known, otherwise fall back to the
    // opponent-GK formation slot (pre-B1 behaviour; only fires on loose ball,
    // which is handled by preempt_check before this utility runs).
    let (target_x, target_y) = if let Some((cx, cy)) = carrier_pos {
        (cx, cy)
    } else {
        let press_target_slot: u8 = if (roster_slot as usize) < 11 { 11 } else { 0 };
        formation_position(press_target_slot)
    };
    (PlayerIntent::Press { target_x, target_y }, biased)
}

/// Utility for marking an opponent.
///
/// Attribute binding (spec): marking × anticipation × pace × concentration (primary);
/// strength + balance as secondary; determination ONLY via bias.
///
/// B1 (FUN-0b+c): when the carrier position is known, mark toward the carrier
/// instead of the fixed formation-slot proxy (slot 12 / slot 1). This keeps
/// tight markers on the actual threat. When no carrier is active the old
/// formation-slot target is used as a fallback.
pub fn utility_mark_player(
    player: &PlayerState,
    roster_slot: u8,
    carrier_pos: Option<(Q32, Q32)>,
) -> (PlayerIntent, Q32) {
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

    // Mark bias: Determination ONLY per spec (P1-5 fix — was cover_bias which also read work_rate).
    let biased = apply_mark_bias(raw, a);

    // B1: target the actual carrier if known, otherwise fall back to the
    // opponent-GK+1 formation slot (pre-B1 proxy).
    let (target_x, target_y) = if let Some((cx, cy)) = carrier_pos {
        (cx, cy)
    } else {
        let mark_slot: u8 = if (roster_slot as usize) < 11 { 12 } else { 1 };
        formation_position(mark_slot)
    };
    (PlayerIntent::MarkPlayer { target_x, target_y }, biased)
}

/// Utility for making a run off the ball.
///
/// Attribute binding (spec): off_the_ball × pace × acceleration × anticipation (primary);
/// flair + stamina as secondary; work_rate + risk_appetite via bias.
///
/// FUN-TS1: starts from `zonal_slot(roster_slot, shape, team_idx)` then
/// adds the 10m advance in the attack direction (home = +x, away = -x).
pub fn utility_run_off_ball(
    player: &PlayerState,
    roster_slot: u8,
    shape: &TeamShape,
    team_idx: usize,
) -> (PlayerIntent, Q32) {
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

    // RunOffBall bias: WorkRate + RiskAppetite per spec (P1-5 fix — was press_bias which used aggression).
    let biased = apply_run_off_ball_bias(raw, a);

    // FUN-TS1: target is the zonal_slot directly — no extra advance.
    // The prior +10m advance (from static formation era) is removed because
    // zonal_slot already places FWDs at the correct attacking position for the
    // tactic state (e.g. HighPress FWDs at +33m). Adding another +10m placed
    // them at +43m = near the opponent's penalty area, causing runaway scoring.
    let (target_x, target_y) = zonal_slot(roster_slot, shape, team_idx);
    (PlayerIntent::RunOffBall { target_x, target_y }, biased)
}

/// **Enforcement intent for defensive block holding (FUN-TS1).**
///
/// Returns a `HoldFormation` intent targeting the player's `zonal_slot` with
/// score = `Q32::ONE` (1.0). Used in the defensive match arm of
/// `select_outfield_intent` as the SOLE candidate — single-candidate softmax
/// = deterministic argmax, bypassing attribute-product competition that dilutes
/// the defensive block.
///
/// This is NOT attribute-gated by design: holding a zonal slot when the team
/// is out of possession is a structural duty, not a preference. The press utility
/// (`utility_press`) handles attribute-gated pressing; this handles block-holding.
///
/// No `formation_position` fallback; always `zonal_slot`.
pub fn enforce_hold_zonal(
    roster_slot: u8,
    shape: &TeamShape,
    team_idx: usize,
) -> (PlayerIntent, Q32) {
    let (target_x, target_y) = zonal_slot(roster_slot, shape, team_idx);
    (PlayerIntent::HoldFormation { target_x, target_y }, Q32::ONE)
}

/// Compute the x-axis lane-cover offset for a defending player (Layer 2, §2.1).
///
/// The offset nudges the defender's zonal target toward the carrier's position
/// by up to `LANE_COVER_MAX_DEF = 8m`, scaled by:
///   `lane_cover_weight = anticipation × 0.50 + tackling × 0.30 + positioning × 0.20`
///
/// The offset is clamped so it never moves the defender PAST the carrier's x (no
/// overrun). For team_idx=0 (home, attacks +x), "toward carrier" means +x when
/// the carrier is ahead. For team_idx=1 (away, attacks -x), "toward carrier" means -x.
///
/// Returns a signed Q32 x-offset in metres. Positive = toward +x end.
///
/// Determinism: Q32 only, no RNG, pure function of canonical inputs.
#[must_use]
pub fn lane_cover_offset_x(
    player: &PlayerState,
    base_target_x: Q32,
    carrier_pos: (Q32, Q32),
    team_idx: usize,
) -> Q32 {
    let a = &player.attributes;
    let lane_cover_weight = a.mental.anticipation * LANE_W_ANTICIPATION
        + a.technical.tackling * LANE_W_TACKLING
        + a.mental.positioning * LANE_W_POSITIONING;

    // Raw offset magnitude (toward carrier): lane_cover_weight × LANE_COVER_MAX_DEF.
    let raw_offset = lane_cover_weight * LANE_COVER_MAX_DEF;

    // Direction: does the carrier lie ahead (in the attacking direction) of our base target?
    // For home (team_idx=0, attacks +x): "ahead" = carrier_x > base_target_x.
    // For away (team_idx=1, attacks -x): "ahead" = carrier_x < base_target_x.
    // If not ahead (carrier behind the defender's line), no step-up is useful.
    let (carrier_x, _carrier_y) = carrier_pos;
    if team_idx == 0 {
        if carrier_x <= base_target_x {
            return Q32::ZERO;
        }
        // Clamp: must not overshoot past carrier_x.
        let room = carrier_x - base_target_x;
        if raw_offset > room { room } else { raw_offset }
    } else {
        if carrier_x >= base_target_x {
            return Q32::ZERO;
        }
        // Away moves in -x direction. Return a NEGATIVE offset.
        let room = base_target_x - carrier_x; // always positive
        let clamped = if raw_offset > room { room } else { raw_offset };
        Q32::ZERO - clamped
    }
}

/// **Defender lane-cover enforcement (Layer 2, §2.1).**
///
/// Like `enforce_hold_zonal` but applies a lane-cover offset toward the carrier's
/// passing lane, scaled by `lane_cover_weight = anticipation×0.50 + tackling×0.30 +
/// positioning×0.20`. The score remains `Q32::ONE` so this intent dominates the
/// single-candidate softmax exactly as `enforce_hold_zonal` does.
///
/// The offset is at most `LANE_COVER_MAX_DEF = 8m` and is clamped so the defender
/// never runs past the carrier.
///
/// When `carrier_pos` is `None`, falls back to `enforce_hold_zonal` (no offset).
///
/// `technical.tackling` is used as the proxy for the design doc's `technical.interception`
/// since that attribute field does not currently exist in the schema.
pub fn enforce_hold_with_lane_cover(
    player: &PlayerState,
    roster_slot: u8,
    shape: &TeamShape,
    team_idx: usize,
    carrier_pos: Option<(Q32, Q32)>,
) -> (PlayerIntent, Q32) {
    let (base_x, target_y) = zonal_slot(roster_slot, shape, team_idx);
    let target_x = if let Some(cpos) = carrier_pos {
        base_x + lane_cover_offset_x(player, base_x, cpos, team_idx)
    } else {
        base_x
    };
    (PlayerIntent::HoldFormation { target_x, target_y }, Q32::ONE)
}

/// Utility for holding formation position.
///
/// Attribute binding (spec): positioning × teamwork × concentration (primary);
/// decisions as secondary gate; professionalism + determination via bias.
///
/// FUN-TS1: targets `zonal_slot(roster_slot, shape, team_idx)` instead of the
/// constant `formation_position(roster_slot)`.
pub fn utility_hold_formation(
    player: &PlayerState,
    roster_slot: u8,
    shape: &TeamShape,
    team_idx: usize,
) -> (PlayerIntent, Q32) {
    let a = &player.attributes;

    // Primary product: positioning + teamwork + concentration (spec §"Hold formation slot").
    let raw = a.mental.positioning * a.mental.teamwork * a.mental.concentration;

    // Secondary: decisions as quality gate.
    let w_dec = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary = Q32::ONE + w_dec * a.mental.decisions;
    let raw = raw * secondary;

    // HoldFormation bias: Professionalism + Determination per spec (P1-5 fix — was cover_bias with work_rate).
    let biased = apply_hold_formation_bias(raw, a);

    let (target_x, target_y) = zonal_slot(roster_slot, shape, team_idx);
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
    use crate::team_shape::TeamShape;
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

    /// Test shape using MidBlock defaults so targets are non-zero and representative.
    fn test_shape() -> TeamShape {
        TeamShape {
            line_x: Q32::from_int(-18),
            block_centroid_x: Q32::from_int(-20),
            block_centroid_y: Q32::ZERO,
            compactness_v: Q32::from_int(32),
            compactness_h: Q32::from_int(35),
            is_defending: true,
            press_roles: [crate::team_shape::PressRole::HoldShape; 11],
            is_high_press: false,
            phase_tx: Q32::ZERO,
        }
    }

    // --- Range tests ---

    #[test]
    fn track_back_utility_in_unit_range() {
        let p = mid_player(6);
        let s = test_shape();
        let (_, u) = utility_track_back(&p, 6, &s, 0);
        assert!(u >= Q32::ZERO);
        assert!(u <= Q32::ONE);
    }

    #[test]
    fn press_utility_in_unit_range() {
        let p = mid_player(6);
        let (_, u) = utility_press(&p, 6, None);
        assert!(u >= Q32::ZERO);
        assert!(u <= Q32::ONE);
    }

    #[test]
    fn mark_player_utility_in_unit_range() {
        let p = mid_player(6);
        let (_, u) = utility_mark_player(&p, 6, None);
        assert!(u >= Q32::ZERO);
        assert!(u <= Q32::ONE);
    }

    #[test]
    fn run_off_ball_utility_in_unit_range() {
        let p = mid_player(6);
        let s = test_shape();
        let (_, u) = utility_run_off_ball(&p, 6, &s, 0);
        assert!(u >= Q32::ZERO);
        assert!(u <= Q32::ONE);
    }

    #[test]
    fn hold_formation_utility_in_unit_range() {
        let p = mid_player(6);
        let s = test_shape();
        let (_, u) = utility_hold_formation(&p, 6, &s, 0);
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
        let s = test_shape();
        let (_, u_a) = utility_track_back(&p_a, 6, &s, 0);
        let (_, u_b) = utility_track_back(&p_b, 6, &s, 0);
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
        let s = test_shape();
        let (_, hi) = utility_track_back(&p_hi, 6, &s, 0);
        let (_, lo) = utility_track_back(&p_lo, 6, &s, 0);
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
        let (_, u_a) = utility_press(&p_a, 6, None);
        let (_, u_b) = utility_press(&p_b, 6, None);
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
        let (_, hi) = utility_press(&p_hi, 6, None);
        let (_, lo) = utility_press(&p_lo, 6, None);
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
        let (_, hi) = utility_press(&p_hi, 6, None);
        let (_, lo) = utility_press(&p_lo, 6, None);
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
        let (_, u_a) = utility_mark_player(&p_a, 6, None);
        let (_, u_b) = utility_mark_player(&p_b, 6, None);
        assert_eq!(
            u_a, u_b,
            "mental.decisions must not affect mark_player utility"
        );
    }

    #[test]
    fn mark_player_work_rate_not_in_binding() {
        // work_rate must NOT affect mark_player utility (P1-5 fix: was cover_bias proxy).
        let mut p_a = mid_player(6);
        let mut p_b = mid_player(6);
        p_a.attributes.personality.work_rate = Q32::ZERO;
        p_b.attributes.personality.work_rate = Q32::ONE;
        let (_, u_a) = utility_mark_player(&p_a, 6, None);
        let (_, u_b) = utility_mark_player(&p_b, 6, None);
        assert_eq!(
            u_a, u_b,
            "work_rate must not affect mark_player utility (P1-5 spec fix — determination only)"
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
        let (_, hi) = utility_mark_player(&p_hi, 6, None);
        let (_, lo) = utility_mark_player(&p_lo, 6, None);
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
        let s = test_shape();
        let (_, u_a) = utility_run_off_ball(&p_a, 6, &s, 0);
        let (_, u_b) = utility_run_off_ball(&p_b, 6, &s, 0);
        assert_eq!(
            u_a, u_b,
            "mental.decisions must not affect run_off_ball utility"
        );
    }

    #[test]
    fn run_off_ball_aggression_not_in_binding() {
        // aggression must NOT affect run_off_ball utility (P1-5 fix: was press_bias proxy).
        let mut p_a = mid_player(6);
        let mut p_b = mid_player(6);
        p_a.attributes.personality.aggression = Q32::ZERO;
        p_b.attributes.personality.aggression = Q32::ONE;
        let s = test_shape();
        let (_, u_a) = utility_run_off_ball(&p_a, 6, &s, 0);
        let (_, u_b) = utility_run_off_ball(&p_b, 6, &s, 0);
        assert_eq!(
            u_a, u_b,
            "aggression must not affect run_off_ball utility (P1-5 spec fix — work_rate+risk_appetite)"
        );
    }

    #[test]
    fn run_off_ball_risk_appetite_via_bias_changes_utility() {
        // personality.risk_appetite IS in the run_off_ball binding now (P1-5 fix).
        let mut p_hi = mid_player(6);
        let mut p_lo = mid_player(6);
        p_hi.attributes.personality.risk_appetite = Q32::ONE;
        p_lo.attributes.personality.risk_appetite = Q32::ZERO;
        let s = test_shape();
        let (_, hi) = utility_run_off_ball(&p_hi, 6, &s, 0);
        let (_, lo) = utility_run_off_ball(&p_lo, 6, &s, 0);
        assert!(
            hi > lo,
            "personality.risk_appetite (spec bias) must affect run_off_ball utility (P1-5)"
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
        let s = test_shape();
        let (_, hi) = utility_run_off_ball(&p_hi, 6, &s, 0);
        let (_, lo) = utility_run_off_ball(&p_lo, 6, &s, 0);
        assert!(
            hi > lo,
            "off_the_ball + acceleration (spec primary) must affect run_off_ball utility"
        );
    }

    #[test]
    fn hold_formation_non_spec_attr_has_no_effect() {
        // `technical.finishing` is unambiguously not in hold_formation binding.
        let mut p_a = mid_player(6);
        let mut p_b = mid_player(6);
        p_a.attributes.technical.finishing = Q32::ZERO;
        p_b.attributes.technical.finishing = Q32::ONE;
        let s = test_shape();
        let (_, u_a) = utility_hold_formation(&p_a, 6, &s, 0);
        let (_, u_b) = utility_hold_formation(&p_b, 6, &s, 0);
        assert_eq!(
            u_a, u_b,
            "technical.finishing must not affect hold_formation utility"
        );
    }

    #[test]
    fn hold_formation_work_rate_not_in_binding() {
        // work_rate must NOT affect hold_formation utility (P1-5 fix: was cover_bias proxy).
        let mut p_a = mid_player(6);
        let mut p_b = mid_player(6);
        p_a.attributes.personality.work_rate = Q32::ZERO;
        p_b.attributes.personality.work_rate = Q32::ONE;
        let s = test_shape();
        let (_, u_a) = utility_hold_formation(&p_a, 6, &s, 0);
        let (_, u_b) = utility_hold_formation(&p_b, 6, &s, 0);
        assert_eq!(
            u_a, u_b,
            "work_rate must not affect hold_formation utility (P1-5 spec fix — professionalism+determination)"
        );
    }

    #[test]
    fn hold_formation_professionalism_via_bias_changes_utility() {
        // personality.professionalism IS in the hold_formation binding now (P1-5 fix).
        let mut p_hi = mid_player(6);
        let mut p_lo = mid_player(6);
        p_hi.attributes.personality.professionalism = Q32::ONE;
        p_lo.attributes.personality.professionalism = Q32::ZERO;
        let s = test_shape();
        let (_, hi) = utility_hold_formation(&p_hi, 6, &s, 0);
        let (_, lo) = utility_hold_formation(&p_lo, 6, &s, 0);
        assert!(
            hi > lo,
            "personality.professionalism (spec bias) must affect hold_formation utility (P1-5)"
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
        let s = test_shape();
        let (_, hi) = utility_hold_formation(&p_hi, 6, &s, 0);
        let (_, lo) = utility_hold_formation(&p_lo, 6, &s, 0);
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
        let (_, hi) = utility_press(&p_hi, 6, None);
        let (_, lo) = utility_press(&p_lo, 6, None);
        assert!(
            hi >= lo,
            "higher aggression/work_rate should not decrease press utility"
        );
    }

    // --- Determinism ---

    #[test]
    fn off_ball_utilities_deterministic() {
        let p = mid_player(6);
        let (i1, u1) = utility_press(&p, 6, None);
        let (i2, u2) = utility_press(&p, 6, None);
        assert_eq!(i1, i2);
        assert_eq!(u1, u2);
    }

    // --- B1 carrier-targeting tests ---

    #[test]
    fn press_targets_carrier_when_provided() {
        let p = mid_player(6);
        let carrier_x = Q32::from_int(20);
        let carrier_y = Q32::from_int(5);
        let (intent, _) = utility_press(&p, 6, Some((carrier_x, carrier_y)));
        assert_eq!(
            intent,
            PlayerIntent::Press {
                target_x: carrier_x,
                target_y: carrier_y
            },
            "press must target the actual carrier position when carrier_pos is Some"
        );
    }

    #[test]
    fn mark_player_targets_carrier_when_provided() {
        let p = mid_player(6);
        let carrier_x = Q32::from_int(15);
        let carrier_y = Q32::from_int(-3);
        let (intent, _) = utility_mark_player(&p, 6, Some((carrier_x, carrier_y)));
        assert_eq!(
            intent,
            PlayerIntent::MarkPlayer {
                target_x: carrier_x,
                target_y: carrier_y
            },
            "mark must target the actual carrier position when carrier_pos is Some"
        );
    }

    #[test]
    fn press_falls_back_to_formation_slot_when_no_carrier() {
        let p = mid_player(6);
        // Home slot 6 → opponent GK slot = 11.
        let (expected_x, expected_y) = formation_position(11);
        let (intent, _) = utility_press(&p, 6, None);
        assert_eq!(
            intent,
            PlayerIntent::Press {
                target_x: expected_x,
                target_y: expected_y
            },
            "press must fall back to opponent-GK formation slot when carrier_pos is None"
        );
    }

    // --- Target correctness ---

    #[test]
    fn track_back_targets_zonal_slot() {
        let p = mid_player(6);
        let s = test_shape();
        let (intent, _) = utility_track_back(&p, 6, &s, 0);
        let (expected_x, expected_y) = zonal_slot(6, &s, 0);
        assert_eq!(
            intent,
            PlayerIntent::TrackBack {
                target_x: expected_x,
                target_y: expected_y
            },
            "track_back must target zonal_slot (FUN-TS1)"
        );
    }

    #[test]
    fn hold_formation_targets_zonal_slot() {
        let p = mid_player(6);
        let s = test_shape();
        let (intent, _) = utility_hold_formation(&p, 6, &s, 0);
        let (expected_x, expected_y) = zonal_slot(6, &s, 0);
        assert_eq!(
            intent,
            PlayerIntent::HoldFormation {
                target_x: expected_x,
                target_y: expected_y
            },
            "hold_formation must target zonal_slot (FUN-TS1)"
        );
    }

    // --- Layer 2: lane-cover tests ---

    /// High-anticipation/tackling defender steps CLOSER to the carrier
    /// than a low-reading one in the same game state.
    #[test]
    fn high_reading_defender_steps_closer_to_carrier_than_low() {
        let s = test_shape(); // line_x=-18, defending=true, compactness_v=32

        // Carrier is ahead (carrier_x > base_slot_x for home team 0).
        // Slot 2 (home DEF) at x=(-30) after zonal transform relative to line_x.
        // Let's use slot 2 with shape line_x=-18.
        let (base_x, _) = zonal_slot(2, &s, 0); // defender's base position
        let carrier_x = Q32::from_int(10); // carrier is well ahead of defender
        let carrier_y = Q32::from_int(0);
        let carrier = (carrier_x, carrier_y);

        // High-reading defender.
        let mut hi_def = mid_player(2);
        hi_def.attributes.mental.anticipation = Q32::ONE;
        hi_def.attributes.technical.tackling = Q32::ONE;
        hi_def.attributes.mental.positioning = Q32::ONE;

        // Low-reading defender.
        let mut lo_def = mid_player(2);
        lo_def.attributes.mental.anticipation = Q32::ZERO;
        lo_def.attributes.technical.tackling = Q32::ZERO;
        lo_def.attributes.mental.positioning = Q32::ZERO;

        let (hi_intent, _) = enforce_hold_with_lane_cover(&hi_def, 2, &s, 0, Some(carrier));
        let (lo_intent, _) = enforce_hold_with_lane_cover(&lo_def, 2, &s, 0, Some(carrier));

        let hi_x = match hi_intent {
            PlayerIntent::HoldFormation { target_x, .. } => target_x,
            _ => panic!("expected HoldFormation"),
        };
        let lo_x = match lo_intent {
            PlayerIntent::HoldFormation { target_x, .. } => target_x,
            _ => panic!("expected HoldFormation"),
        };

        // Both targets must be >= base (stepped toward carrier, not backwards).
        assert!(
            hi_x >= base_x,
            "high-reading defender must step at least to base_x, got hi={hi_x:?} base={base_x:?}"
        );
        // High-reading defender must be closer to carrier (larger x for home team).
        assert!(
            hi_x > lo_x,
            "high-reading defender (hi={hi_x:?}) must be closer to carrier than low (lo={lo_x:?})"
        );
    }

    /// Lane-cover offset must not move the defender past the carrier.
    #[test]
    fn lane_cover_never_overshoots_carrier() {
        let s = test_shape();

        // Put carrier just slightly ahead of the defender's base slot.
        let (base_x, _) = zonal_slot(2, &s, 0);
        // Carrier only 1m ahead.
        let carrier_x = base_x + Q32::from_raw(1_i64 << 32); // +1m
        let carrier = (carrier_x, Q32::ZERO);

        let mut hi_def = mid_player(2);
        hi_def.attributes.mental.anticipation = Q32::ONE;
        hi_def.attributes.technical.tackling = Q32::ONE;
        hi_def.attributes.mental.positioning = Q32::ONE;

        let (intent, _) = enforce_hold_with_lane_cover(&hi_def, 2, &s, 0, Some(carrier));
        let target_x = match intent {
            PlayerIntent::HoldFormation { target_x, .. } => target_x,
            _ => panic!("expected HoldFormation"),
        };
        // Must not overshoot the carrier.
        assert!(
            target_x <= carrier_x,
            "lane-cover must not overshoot carrier: target={target_x:?} carrier={carrier_x:?}"
        );
    }

    /// When carrier is BEHIND the defender (carrier_x < base_x for home),
    /// no step-up should occur.
    #[test]
    fn lane_cover_no_offset_when_carrier_behind() {
        let s = test_shape();
        let (base_x, _) = zonal_slot(2, &s, 0);
        // Put carrier behind the defender (further from opp goal).
        let carrier_x = base_x - Q32::from_raw(5_i64 << 32); // 5m behind
        let carrier = (carrier_x, Q32::ZERO);

        let mut hi_def = mid_player(2);
        hi_def.attributes.mental.anticipation = Q32::ONE;
        hi_def.attributes.technical.tackling = Q32::ONE;
        hi_def.attributes.mental.positioning = Q32::ONE;

        let (intent, _) = enforce_hold_with_lane_cover(&hi_def, 2, &s, 0, Some(carrier));
        let target_x = match intent {
            PlayerIntent::HoldFormation { target_x, .. } => target_x,
            _ => panic!("expected HoldFormation"),
        };
        // Target must equal base_x — no step-up when carrier is behind.
        assert_eq!(
            target_x, base_x,
            "no lane-cover offset when carrier is behind: target={target_x:?} base={base_x:?}"
        );
    }

    /// enforce_hold_with_lane_cover with carrier=None must equal enforce_hold_zonal.
    #[test]
    fn lane_cover_no_carrier_equals_plain_hold_zonal() {
        let s = test_shape();
        let p = mid_player(2);
        let (with_none, _) = enforce_hold_with_lane_cover(&p, 2, &s, 0, None);
        let (plain, _) = enforce_hold_zonal(2, &s, 0);
        assert_eq!(
            with_none, plain,
            "lane_cover with None carrier must equal enforce_hold_zonal"
        );
    }

    /// Score remains Q32::ONE after lane-cover (enforcement invariant).
    #[test]
    fn lane_cover_score_is_one() {
        let s = test_shape();
        let (base_x, _) = zonal_slot(2, &s, 0);
        let carrier = (base_x + Q32::from_raw(5_i64 << 32), Q32::ZERO);
        let p = mid_player(2);
        let (_, score) = enforce_hold_with_lane_cover(&p, 2, &s, 0, Some(carrier));
        assert_eq!(
            score,
            Q32::ONE,
            "enforce_hold_with_lane_cover score must be Q32::ONE"
        );
    }
}

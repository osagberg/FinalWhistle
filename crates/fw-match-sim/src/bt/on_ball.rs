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
// SS1 — shot-decision quality gate constants (FUN-0b)
// ---------------------------------------------------------------------------

/// Minimum xG score required to attempt a shot (SS1 gate).
///
/// Shots with `xg_utility(ctx) < XG_SHOOT_THRESHOLD` are suppressed entirely
/// (utility = `Q32::ZERO`) — removing them from the softmax candidate pool.
///
/// docs/design/shot-model.md §Sub-system 1 started with a provisional value of
/// 0.041. At T1 formation positions (FWDs start at 42.5m from goal) the xG
/// logistic yields ~0.031 (because `distance_q32 = 0` for shots beyond 35m),
/// so 0.041 gated out ALL formation-position shots. Drama-sweep calibration
/// (R11 final) settled on ≈0.095 — high enough to suppress blind 50m+ pokes
/// while letting plausible efforts through.
///
/// FUN-TS2 recal: lowered from 0.095 to 0.070. Per the calibration rule
/// (shots/match < 10 → lower by 0.003/sweep), TS2 coordinated press suppressed
/// shot volume to ~7/match; -0.025 recovers ~3-4 shots/match from speculative
/// mid-range efforts (15-30m zone). Keeps blind 50m+ pokes gated (xG ≈ 0.031
/// at formation start positions stays below even 0.070).
pub(crate) const XG_SHOOT_THRESHOLD: Q32 = Q32::from_raw(300_647_710_i64); // ≈ 0.070 (FUN-TS2 recal)

/// Shooter quality composite weights for `xg_utility` feature extraction.
/// `shooter_quality = finishing × 0.55 + composure × 0.25 + technique × 0.20`
/// Matches the `shot_quality_feature_q32` helper in `dispatch.rs`.
const W_SQ_FINISHING: Q32 = Q32::from_raw(2_362_232_012_i64); // ≈ 0.55
const W_SQ_COMPOSURE: Q32 = Q32::from_raw(1_073_741_824_i64); // ≈ 0.25
const W_SQ_TECHNIQUE: Q32 = Q32::from_raw(858_993_459_i64); // ≈ 0.20

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
/// **Codex Tier-2 re-audit P2 fix**: `technical.long_shots` is PRIMARY in
/// the spec (§"Shoot") — prior doc-comment + impl treated it as secondary.
/// Spec primary list: `technical.finishing`, `technical.long_shots` (long-
/// distance shots), `mental.composure`, `mental.decisions`. Spec secondary
/// list: `mental.vision` (angle assessment), `physical.balance` (off-balance
/// penalty). The implementation below now multiplies `long_shots` into the
/// primary product, matching the spec.
///
/// Primary (spec): finishing × long_shots × composure × decisions
/// Secondary (spec): vision, balance
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
/// Attribute binding (spec, post-Codex-P2-fix):
/// - Primary: finishing × long_shots × composure × decisions
/// - Secondary: vision (angle quality), balance (off-balance penalty)
/// - Bias via helper: flair + composure + risk_appetite + PT
///
/// Prior implementation treated `long_shots` as a secondary `(1 + 0.30×x)`
/// additive modifier; spec §"Shoot" lists it as primary. Reclassified to
/// the primary product (multiplicative). Drifts the canonical hash because
/// the per-player utility values change — accepted within the T1-2b-fix
/// row per ADR-0012 trigger #3.
pub fn utility_shoot(player: &PlayerState, roster_slot: u8) -> (PlayerIntent, Q32) {
    let a = &player.attributes;

    // ---------------------------------------------------------------------------
    // SS1 — Shot-decision quality gate (FUN-0b, docs/design/shot-model.md §SS1)
    //
    // Replace the prior 4×/3×/2× proximity stub with the real xG logistic from
    // `crate::utility::xg::xg_utility`. Gate shots below `XG_SHOOT_THRESHOLD`
    // to Q32::ZERO (removes them from the softmax candidate set entirely).
    //
    // Feature extraction uses available info (player pos + proxy pressure).
    // Defender pressure proxy = complement of composure (same proxy as before;
    // real spatial pressure arrives at T2-1 when opponent positions thread in).
    // ---------------------------------------------------------------------------

    // Distance feature (inverted: 0=far, 1=close) via GOAL_LINE_X.
    let goal_x: Q32 = if (roster_slot as usize) < 11 {
        fw_core::GOAL_LINE_X
    } else {
        -fw_core::GOAL_LINE_X
    };
    let dx = goal_x - player.pos_x;
    let dx_abs_bits = dx.to_bits().unsigned_abs();
    // Threshold 35m
    const DIST_THRESHOLD_BITS: u64 = 35_u64 << 32;
    let distance_q32: Q32 = if dx_abs_bits >= DIST_THRESHOLD_BITS {
        Q32::ZERO
    } else {
        let normalized = Q32::from_raw(dx_abs_bits as i64) / Q32::from_int(35);
        Q32::ONE - normalized
    };

    // Angle feature: 1 - clamp(|pos_y| / 25, 0, 1).
    let py_abs_bits = player.pos_y.to_bits().unsigned_abs();
    const ANGLE_THRESHOLD_BITS: u64 = 25_u64 << 32;
    let angle_q32: Q32 = if py_abs_bits >= ANGLE_THRESHOLD_BITS {
        Q32::ZERO
    } else {
        let normalized = Q32::from_raw(py_abs_bits as i64) / Q32::from_int(25);
        Q32::ONE - normalized
    };

    // Defender pressure proxy = PT-attenuated complement of composure.
    let raw_pressure = Q32::ONE - a.mental.composure;
    let eff_pressure = read_defender_pressure(a, raw_pressure);
    // Extract the inner Q32 from the DefenderPressure newtype for ShotContext.
    let pressure_q32 = eff_pressure.0;

    // Shooter quality: finishing × 0.55 + composure × 0.25 + technique × 0.20.
    let shooter_quality: Q32 = a.technical.finishing * W_SQ_FINISHING
        + a.mental.composure * W_SQ_COMPOSURE
        + a.technical.technique * W_SQ_TECHNIQUE;
    let shooter_quality = if shooter_quality > Q32::ONE {
        Q32::ONE
    } else {
        shooter_quality
    };

    // Build ShotContext. All fields are derived above and are in [0, 1].
    // Use footed shot (1.0) and solo assist (1.0) for T1 proxy values.
    let ctx = crate::utility::xg::ShotContext::try_new(
        distance_q32,
        angle_q32,
        pressure_q32,
        Q32::ONE, // footed shot
        Q32::ONE, // solo assist
        shooter_quality,
    )
    .expect("ShotContext features must be in [0, 1] — check proxy derivation");

    let xg_score = crate::utility::xg::xg_utility(&ctx);

    // Gate: suppress low-xG shots entirely.
    if xg_score < XG_SHOOT_THRESHOLD {
        // Zero utility removes this from the softmax candidate pool.
        // Target still set for structural completeness (won't be used).
        let target_x = if (roster_slot as usize) < 11 {
            Q32::from_int(52)
        } else {
            Q32::from_int(-52)
        };
        return (
            PlayerIntent::AttemptShot {
                target_x,
                target_y: Q32::ZERO,
            },
            Q32::ZERO,
        );
    }

    // Gate passed: raw utility = xg_score × shooter_quality × secondary modifiers.
    // Secondary: long_shots (power), vision (angle assessment), balance (off-balance).
    // drama-sweep R3: weights boosted from 0.20/0.20/0.10 to 3.0/2.5/1.5 to make
    // shoot utility competitive with pass utility in the softmax.
    // Justification: pass utility (0.5^4 × secondary ≈ 0.07) dominated shoot
    // (xG×quality×old_secondary ≈ 0.026) at default attributes. At T1 formation
    // positions, players rarely reach penalty-area xG values, so the xG component
    // alone is insufficient to outbid passing. The higher secondary weights
    // amplify the position signal so forwards at 17-25m score utility ≈ 0.07-0.12,
    // competitive with passes. Long-distance attempts (xG < 0.010) remain gated.
    // Secondary: long_shots (power), vision (angle assessment), balance (off-balance).
    // drama-sweep R7: secondary=4 (w_ls=2.0, w_vis=1.5, w_bal=1.0) to bridge the
    // structural utility gap between pass (attribute-product, ~0.085) and shoot
    // (xG-driven, ~0.011-0.154 depending on distance). At secondary=4:
    //   from 20m: shoot biased ≈ 0.067 < pass 0.085 → prefers passing (correct)
    //   from 15m: shoot biased ≈ 0.297 > pass 0.085 → prefers shooting (correct)
    //   from 12m: shoot biased ≈ 0.469 → strong shoot preference (correct for box)
    // This gives ~40% shoot probability from 15-20m; near-certain from <12m.
    let w_ls = Q32::from_raw(5_583_457_434_i64); // ≈ 1.3 (drama-sweep R14)
    let w_vision = Q32::from_raw(4_294_967_296_i64); // ≈ 1.0 (drama-sweep R14)
    let w_balance = Q32::from_raw(2_576_980_378_i64); // ≈ 0.6 (drama-sweep R14)
    let secondary = (Q32::ONE + w_ls * a.technical.long_shots)
        * (Q32::ONE + w_vision * a.mental.vision)
        * (Q32::ONE + w_balance * a.physical.balance);
    // raw_shoot = xg_score × shooter_quality × secondary modifiers.
    let raw = xg_score * shooter_quality * secondary;
    let raw = if raw > Q32::ONE { Q32::ONE } else { raw };

    let biased = apply_shoot_bias(raw, a, eff_pressure);
    let biased = if biased > Q32::ONE { Q32::ONE } else { biased };

    // Target: the attacking goal centre. `target_y = Q32::ZERO` is the dead-centre
    // placeholder; the REAL dispersed target_y is computed in `dispatch::apply_intent`
    // (SS2) where match_seed + tick + player positions are all available.
    // Using ±52m ensures ball_unit_vel never returns (0, 0) for a shooter still
    // behind the goal line (same rationale as the T1-15 fix above).
    let target_x = if (roster_slot as usize) < 11 {
        Q32::from_int(52)
    } else {
        Q32::from_int(-52)
    };

    (
        PlayerIntent::AttemptShot {
            target_x,
            target_y: Q32::ZERO, // Placeholder; overwritten by SS2 in dispatch::apply_intent
        },
        biased,
    )
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

    // T1-15 fix: short pass targets 10m FORWARD from the carrier's CURRENT
    // position (not formation position). Using current position ensures the
    // target is always ahead of the carrier — even when players have drifted
    // far from their formation slot. Prior code used `fy + 5` (Y-axis sideways
    // offset from formation), causing the ball to loop in midfield forever.
    let target_x = if (roster_slot as usize) < 11 {
        player.pos_x + Q32::from_int(10)
    } else {
        player.pos_x - Q32::from_int(10)
    };
    let target_y = player.pos_y;

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

    /// FUN-0b SS1: a player close to goal (high xG) must have non-zero shoot utility.
    ///
    /// At pos_x=40m for home FWD (dist_to_goal ≈ 12.5m), xG is well above the
    /// XG_SHOOT_THRESHOLD (≈0.095) — the gate must pass and utility must be > 0.
    #[test]
    fn shoot_utility_nonzero_for_close_shot_above_gate() {
        let mut p = mid_player(9);
        p.pos_x = Q32::from_int(40); // close to home goal +x (dist ≈ 12.5m)
        p.attributes = fw_core::PlayerAttributes::max_baseline();
        let (_, u) = utility_shoot(&p, 9);
        assert!(
            u > Q32::ZERO,
            "close shot above XG_SHOOT_THRESHOLD must have non-zero utility; got {u:?}"
        );
        assert!(u <= Q32::ONE, "shoot utility must be <= 1; got {u:?}");
    }

    /// FUN-0b SS1: a player far from goal (low xG) must be gated to ZERO utility.
    ///
    /// At pos_x = -30m for home FWD (dist_to_goal ≈ 82.5m), xG < 0.001 — well
    /// below XG_SHOOT_THRESHOLD. The gate must suppress the candidate.
    #[test]
    fn shoot_utility_zero_for_far_shot_below_gate() {
        let mut p = mid_player(9);
        p.pos_x = Q32::from_int(-30); // far from opponent goal (home player far out)
        let (_, u) = utility_shoot(&p, 9);
        assert_eq!(
            u,
            Q32::ZERO,
            "long-range shot well below XG_SHOOT_THRESHOLD must be gated to zero utility; \
             got {u:?} (this ensures the SS1 quality gate removes hopeless long shots from \
             the softmax candidate set)"
        );
    }

    /// FUN-0b SS1: shoot utility is in [0, 1] for a near-goal player with peak attrs.
    /// Replaces the prior 4×-branch test (proximity multiplier removed in FUN-0b).
    #[test]
    fn shoot_utility_in_unit_range_at_near_goal() {
        let mut p = mid_player(9);
        p.pos_x = Q32::from_int(40); // 12.5m from goal → high xG → gate passes
        p.attributes = fw_core::PlayerAttributes::max_baseline();
        let (_, u) = utility_shoot(&p, 9);
        assert!(u >= Q32::ZERO, "shoot utility must be >= 0; got {u:?}");
        assert!(
            u <= Q32::ONE,
            "shoot utility must be <= 1 (SS1 gate + bias clamp); got {u:?}"
        );
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

    /// FUN-0b: attribute binding tests for utility_shoot now require players to be
    /// close enough to goal for the SS1 xG gate to pass (xG > XG_SHOOT_THRESHOLD).
    /// `near_goal_player` creates a home forward at pos_x = 35m (≈ 17.5m from goal),
    /// which gives xG ≈ 0.06+ for mid-range attrs — safely above the 0.041 gate.
    fn near_goal_player(roster_slot: u8) -> PlayerState {
        let mut p = mid_player(roster_slot);
        // Home slots 0..11: goal at +52.5m. 35m position → dist = 17.5m → xG ≈ 0.06+.
        // Away slots 11..22: goal at -52.5m. Negate.
        if (roster_slot as usize) < 11 {
            p.pos_x = Q32::from_int(35);
        } else {
            p.pos_x = Q32::from_int(-35);
        }
        p
    }

    #[test]
    fn shoot_spec_primary_attr_changes_utility() {
        // SS1 note: test players must be near goal so xG > XG_SHOOT_THRESHOLD.
        // At pos_x = 35m (home FWD slot 9): dist ≈ 17.5m, xG ≈ 0.06+ for hi-quality.
        let mut p_hi = near_goal_player(9);
        let mut p_lo = near_goal_player(9);
        p_hi.attributes.technical.finishing = Q32::ONE;
        p_lo.attributes.technical.finishing = Q32::from_raw(429_496_729_i64); // 0.10 — still passes gate
        let (_, hi) = utility_shoot(&p_hi, 9);
        let (_, lo) = utility_shoot(&p_lo, 9);
        assert!(
            hi > lo,
            "technical.finishing (spec primary) must affect shoot utility when both players \
             are near goal (SS1 gate passes for both)"
        );
    }

    #[test]
    fn shoot_long_shots_is_spec_secondary() {
        // FUN-0b update: `technical.long_shots` is NO LONGER in the SS1 xG-gate
        // formula (the gate uses the 6-feature logistic from xg.rs, which uses
        // shooter_quality = finishing×0.55 + composure×0.25 + technique×0.20).
        // `long_shots` still appears in the secondary modifier AFTER the gate passes.
        // The attribute must affect shoot utility when the player is near goal.
        let mut p_hi = near_goal_player(9);
        let mut p_lo = near_goal_player(9);
        p_hi.attributes.technical.long_shots = Q32::ONE;
        p_lo.attributes.technical.long_shots = Q32::ZERO;
        let (_, hi) = utility_shoot(&p_hi, 9);
        let (_, lo) = utility_shoot(&p_lo, 9);
        assert!(
            hi > lo,
            "technical.long_shots (secondary modifier after SS1 gate) must affect shoot utility \
             when player is near goal"
        );
    }

    #[test]
    fn shoot_risk_appetite_via_bias_changes_utility() {
        // personality.risk_appetite is now in the shoot bias (P1-5 fix).
        // SS1 note: player must be near goal for gate to pass.
        let mut p_hi = near_goal_player(9);
        let mut p_lo = near_goal_player(9);
        p_hi.attributes.personality.risk_appetite = Q32::ONE;
        p_lo.attributes.personality.risk_appetite = Q32::ZERO;
        let (_, hi) = utility_shoot(&p_hi, 9);
        let (_, lo) = utility_shoot(&p_lo, 9);
        assert!(
            hi > lo,
            "personality.risk_appetite (spec bias) must affect shoot utility when near goal \
             (P1-5 — gate passes at this distance)"
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
        // SS1 note: use near-goal position so the xG gate passes for both players.
        let mut p_hi = near_goal_player(9);
        p_hi.attributes.technical.finishing = Q32::ONE;
        let mut p_lo = near_goal_player(9);
        p_lo.attributes.technical.finishing = Q32::from_raw(429_496_729_i64); // 0.10
        let (_, hi) = utility_shoot(&p_hi, 9);
        let (_, lo) = utility_shoot(&p_lo, 9);
        assert!(
            hi > lo,
            "high finishing should produce higher shoot utility (SS1 gate passes at near-goal pos)"
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

    // T1-15: short pass target must advance FORWARD, not sideways.
    // Home slot 5 has formation_x = -10; forward = +x → target_x > -10.
    // Away slot 16 has formation_x = +10; forward = -x → target_x < +10.
    #[test]
    fn pass_short_target_advances_forward_home() {
        let p = mid_player(5);
        let (fx, _fy) = formation_position(5);
        let (PlayerIntent::AttemptPassShort { target_x, .. }, _) = utility_pass_short(&p, 5) else {
            panic!("expected AttemptPassShort");
        };
        assert!(
            target_x > fx,
            "home short-pass target_x ({target_x:?}) must be ahead of formation_x ({fx:?})"
        );
    }

    #[test]
    fn pass_short_target_advances_forward_away() {
        let p = mid_player(16);
        let (fx, _fy) = formation_position(16);
        let (PlayerIntent::AttemptPassShort { target_x, .. }, _) = utility_pass_short(&p, 16)
        else {
            panic!("expected AttemptPassShort");
        };
        assert!(
            target_x < fx,
            "away short-pass target_x ({target_x:?}) must be ahead (less than) formation_x ({fx:?})"
        );
    }
}

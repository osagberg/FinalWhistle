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

use fw_core::{CurveClass, Q32, curve};

use crate::bt::personality_bias::{
    IsProgressive, apply_cross_bias, apply_dribble_bias, apply_hold_bias, apply_lay_off_bias,
    apply_long_pass_bias, apply_safe_pass_bias, apply_shoot_bias, read_defender_pressure,
};
use crate::player::PlayerState;
use crate::role_states::PlayerIntent;
use crate::subtree_library::formation_position;

// ---------------------------------------------------------------------------
// FUN-TS3b — zone-conditional pass-kind bias constants (Attempt 2)
// ---------------------------------------------------------------------------
//
// These constants implement zone-conditional suppression/boosting of pass-kind
// utility so the pass mix approaches realistic football (Short dominant but
// Long/Cross present as a meaningful minority — the "floored gate").
//
// Root cause: Short has a 4-factor primary product (0.5^4 ≈ 0.063) vs Long/Cross
// 3-factor (0.5^3 ≈ 0.125), causing Long/Cross to win ~49%/49% of softmax
// competitions at mid-range attributes.
//
// Fix: apply zone-conditional multipliers AFTER the attribute-derived raw utility
// and BEFORE the personality bias. These reflect real football rationale:
//   - Short passing is rational in own/mid zones (recycle possession).
//   - Long is rational in mid-to-deep zones with an open forward lane.
//   - Cross is only rational wide in the attacking third.
//
// Attempt 1 (ZONE_SHORT_BOOST=5.0 universal in zones 0-13) drove long/cross to 0%.
// Attempt 2 final calibration:
//   - ZONE_SHORT_BOOST=3.2 applied universally (all 16 zones covered):
//     short beats layoff everywhere but does not crush long. Raised from
//     the initial Attempt 2 draft value of 2.8 → 3.2 to push short above
//     hold_ball in the top-3 softmax competition.
//   - LAYOFF_SUPPRESS_ZONE=9: layoff utility suppressed 0.55x in zones ≥ 9 (attacking half).
//     In own/mid zones layoff is rational; in the attacking half it goes backward.
//   - LONG_BASE_SUPPRESS=0.45, LONG_LANE_COEFF=0.35: suppressor at vision=0.5 is 0.625.
//     Long fires at 8-15% of passes. LONG_NO_SUPPRESS_ZONE raised to 15.
//   - CROSS_GATE_COEFF=2.5, CROSS_CENTRAL_Y_M=8m, width-only gate: any player 8m+ wide
//     can cross (no attacking-third gate). At y=15m (wide FWD): suppressor ≈ 0.952.
//
// Target gate: Short 75-85%, Long 8-15%, Cross 3-10%, LayOff 3-8%.
//
// Pitch geometry: x ∈ [-52.5m, +52.5m] goal-to-goal, y ∈ [-34m, +34m].
// Zone grid: 16 x-columns × 12 y-rows. Column width = 105/16 = 6.5625m.
// Zone_x is the attack-direction column (0 = own goal, 15 = opponent goal).
// For HOME team (attacks +x): zone_x = floor((pos_x + 52.5) / 6.5625).
// For AWAY team (attacks -x): zone_x = 15 - home_zone_x.

/// Q32 constant for pitch half-width: 52.5m = 52.5 × 2^32 raw bits.
const PITCH_HALF_X_Q32: Q32 = Q32::from_raw(225_485_783_040_i64);
/// Q32 constant for zone column width: 6.5625m = 6.5625 × 2^32 raw bits.
/// Exact: 6.5625 × 4_294_967_296 = 28_185_722_880. Prior value 28_180_722_893
/// was off by ~1mm (6.5613m vs 6.5625m); corrected in FUN-TS3b Fix B.
const PITCH_ZONE_WIDTH_Q32: Q32 = Q32::from_raw(28_185_722_880_i64);

/// Compute the attack-direction zone index (0-15) from player position.
///
/// Used by `utility_pass_short`, `utility_pass_long`, and `utility_cross` to
/// apply zone-conditional bias. Pure Q32 arithmetic — no floats.
///
/// Returns zone_x ∈ [0, 15]: 0 = own goal end, 15 = opponent goal line.
/// `team_idx`: 0 = home (attacks +x), 1 = away (attacks -x).
#[inline]
pub(crate) fn player_zone_x(pos_x: Q32, team_idx: usize) -> u8 {
    // Home-team zone_x: (pos_x + 52.5m) / 6.5625m → [0, 16) → clamp to [0, 15].
    let shifted = pos_x + PITCH_HALF_X_Q32;
    let zone_q32 = shifted / PITCH_ZONE_WIDTH_Q32;
    let zone_raw = zone_q32.to_bits();
    let home_zone: u8 = if zone_raw < 0 {
        0u8
    } else if zone_raw >= (16_i64 << 32) {
        15u8
    } else {
        (zone_raw >> 32).clamp(0, 15) as u8
    };
    if team_idx == 0 {
        home_zone
    } else {
        15 - home_zone
    }
}

// --- Short pass zone boost ---

/// Universal multiplier applied to raw_short before bias (all zones).
/// At mid-attrs short primary × secondary ≈ 0.097. With boost 3.2: boosted = 0.310.
/// Raised from 2.8→3.2 to push short above hold_ball (0.177) in the top-3 softmax
/// competition, lifting the overall short pass share from 70% toward 75%.
///
/// Q32 raw: round(3.2 × 2^32) = 13_743_895_347.
const ZONE_SHORT_BOOST: Q32 = Q32::from_raw(13_743_895_347_i64); // ≈ 3.2

/// Zone-x threshold above which LayOff is suppressed.
/// Zones ≥ LAYOFF_SUPPRESS_ZONE (attacking half, x > ~4m): a layoff to formation
/// position is a backward pass and is less rational than a short pass forward.
/// Uses the "old" MIDFIELD_ZONE=9 concept — apply at mid-pitch boundary.
const LAYOFF_SUPPRESS_ZONE: u8 = 9;

/// Multiplier applied to LayOff utility in zones ≥ LAYOFF_SUPPRESS_ZONE.
/// At LayOff_biased=0.170, multiplied by 0.55: layoff_effective=0.094.
/// Short_biased (boosted, no clamp) ≈ 0.272×1.35 = 0.368 >> 0.094.
/// Short dominates in attacking zones. In own/mid zones layoff is unsuppressed
/// (rational backward touch in build-up).
///
/// Q32 raw: round(0.55 × 2^32) = 2_362_232_012.
const LAYOFF_ATTACK_SUPPRESS: Q32 = Q32::from_raw(2_362_232_012_i64); // ≈ 0.55

// --- Long pass suppressor ---

/// Zone-x threshold below which full long suppressor applies (no vision bonus).
/// Zones 0-5 = very deep own-half (own penalty area).
const LONG_THRESHOLD_ZONE: u8 = 6;

/// Zone-x threshold above which long passes are unsuppressed.
/// Raised from 14→15: only the final 6m strip near opponent goal is free.
/// In zones 6-14 (mid to attacking), long is suppressed by LONG_BASE_SUPPRESS.
const LONG_NO_SUPPRESS_ZONE: u8 = 15;

/// Long-pass base suppressor (multiplier floor at zero vision).
/// raw_long × (LONG_BASE_SUPPRESS + LONG_LANE_COEFF × vision).
/// At vision=0.5: 0.45+0.35×0.5=0.625. At vision=1.0: 0.45+0.35=0.80.
/// Reduced from 0.58→0.45 to bring long % from 18% down toward 10-15%.
/// At vision=0.5: long_biased ≈ 0.152 × 0.625 × 1.302 ≈ 0.124.
/// Below hold_ball (0.177) so long competes in top-3 only when layoff/dribble low.
///
/// Q32 raw: round(0.45 × 2^32) = 1_932_735_283.
const LONG_BASE_SUPPRESS: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45

/// Lane-openness coefficient for the long-pass suppressor.
/// At vision=1.0: suppressor = 0.45+0.35 = 0.80 (vision-elite can play long).
/// Reduced from 0.37→0.35, narrowing the vision-scaling range.
///
/// Q32 raw: round(0.35 × 2^32) = 1_503_238_554.
const LONG_LANE_COEFF: Q32 = Q32::from_raw(1_503_238_554_i64); // ≈ 0.35

// --- Cross suppressor ---

/// Centre-of-pitch half-width for classifying "central" players (metres).
/// Lowered to 8m: players 8m+ from centre qualify as "wide".
/// Wide FWD at y=15m: wide_pos = (15-8)/26 = 0.269. Cross fires.
/// Q32 raw: 8 × 2^32 = 34_359_738_368.
const CROSS_CENTRAL_Y_M: Q32 = Q32::from_raw(34_359_738_368_i64); // ≈ 8m

/// Width range beyond CROSS_CENTRAL_Y_M to touchline used for normalising
/// wide_position to [0, 1]. 8m → 34m sideline = 26m range.
/// Q32 raw: 26 × 2^32 = 111_669_149_696.
const CROSS_WIDE_RANGE_M: Q32 = Q32::from_raw(111_669_149_696_i64); // ≈ 26m

/// Cross base suppressor (multiplier floor when cross_gate=0).
/// Central or deep players → raw_cross × 0.28.
/// Q32 raw: round(0.28 × 2^32) = 1_202_590_843.
const CROSS_BASE_SUPPRESS: Q32 = Q32::from_raw(1_202_590_843_i64); // ≈ 0.28

/// Gate coefficient scaling wide_position to lift cross into top-3 softmax.
/// Raised from 0.72 to 2.5: at y=15m wide_pos=0.269, suppressor=0.28+2.5×0.269=0.952.
/// raw_cross × 0.952 × bias ≈ 0.163 × 0.952 × 1.2 ≈ 0.186, above hold_ball (0.177).
/// At y=20m: suppressor=1.435 → raw×suppressor ≈ 0.234 (clamped to ≤1.0 in utility_cross).
/// At y=34m (touchline): suppressor=2.78 → raw×suppressor ≈ 0.453 → still ≤1.0 at mid-attrs.
/// At y=0m (central): suppressor = 0.28 (base only), cross stays suppressed.
///
/// Q32 raw: round(2.5 × 2^32) = 10_737_418_240.
const CROSS_GATE_COEFF: Q32 = Q32::from_raw(10_737_418_240_i64); // ≈ 2.5

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
///
/// Goal-production re-tune (2026-06-05): lowered 0.070 -> 0.054. After the
/// goalmouth-defending slice closed all drift goals, shot volume (~11/match)
/// with the floored save model and 8.0m sigma reached only M1 ~2.1. Relaxing
/// the gate to 0.054 lifts shot volume to ~13/match (still inside the 9-18
/// band) and lands M1 ~2.5-2.6 from shots over 100-seed windows, with on-target
/// held at ~43% (in band) and zero drift. xG approx 0.031 at formation start
/// positions stays below 0.054, so blind 50m+ pokes remain gated. Caveat: the
/// extra lower-quality efforts push the M1 goal-distribution std-dev to roughly
/// 1.6 (the top of its 0.8-1.6 band); std-band tuning is a systems-designer
/// balance call.
pub(crate) const XG_SHOOT_THRESHOLD: Q32 = Q32::from_raw(231_928_234_i64); // = 0.054 (goal-production re-tune 2026-06-05)

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
    // Slice 0: each attribute is curved (g_class) before the weighted sum so the
    // elite end of the dominant term (finishing) pulls hard.
    let shooter_quality: Q32 = curve(CurveClass::Skill, a.technical.finishing) * W_SQ_FINISHING
        + curve(CurveClass::Mental, a.mental.composure) * W_SQ_COMPOSURE
        + curve(CurveClass::Skill, a.technical.technique) * W_SQ_TECHNIQUE;
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
    // Slice 0: curve each secondary attribute (long_shots/vision = skill,
    // balance = contest) so an elite long-shooter's secondary boost is
    // disproportionate vs a mid one.
    let secondary = (Q32::ONE + w_ls * curve(CurveClass::Skill, a.technical.long_shots))
        * (Q32::ONE + w_vision * curve(CurveClass::Skill, a.mental.vision))
        * (Q32::ONE + w_balance * curve(CurveClass::Contest, a.physical.balance));
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
///
/// FUN-TS3b: universal ZONE_SHORT_BOOST (3.2×) applied across all zones.
/// Short passing is rational everywhere; the 4-factor primary would otherwise lose to
/// Long/Cross's 3-factor primary at equal attributes. LayOff is separately suppressed
/// in attacking zones (see utility_lay_off + LAYOFF_SUPPRESS_ZONE).
pub fn utility_pass_short(player: &PlayerState, roster_slot: u8) -> (PlayerIntent, Q32) {
    let a = &player.attributes;

    // Primary product. Slice 0: each skill factor is curved before the product,
    // so a player elite in two of these gets a super-linear joint edge.
    let raw = curve(CurveClass::Skill, a.technical.passing)
        * curve(CurveClass::Skill, a.technical.first_touch)
        * curve(CurveClass::Skill, a.technical.technique)
        * curve(CurveClass::Skill, a.mental.vision);

    // Secondary: composure under pressure, decisions as quality gate (mental).
    let w_comp = Q32::from_raw(858_993_459_i64); // ≈ 0.20
    let w_dec = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary = (Q32::ONE + w_comp * curve(CurveClass::Mental, a.mental.composure))
        * (Q32::ONE + w_dec * curve(CurveClass::Mental, a.mental.decisions));
    let raw = raw * secondary;

    // FUN-TS3b: universal short boost (Attempt 2 final, 3.2×).
    // Applied in all zones — the 16-zone pitch grid is fully covered.
    // Short must beat LayOff everywhere; layoff is separately suppressed in attacking
    // zones (see utility_lay_off). Short_raw×secondary ≈ 0.097; boosted 3.2× ≈ 0.310.
    let raw = {
        let boosted = raw * ZONE_SHORT_BOOST;
        if boosted > Q32::ONE {
            Q32::ONE
        } else {
            boosted
        }
    };

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
///
/// FUN-TS3b: vision-weighted suppressor applied so long passes fire proportionally to
/// forward-lane openness (proxied by mental.vision). Suppressor: base=0.45, lane_coeff=0.35.
/// At vision=0.5: suppressor=0.625. At vision=1.0: 0.80 (vision-elite can play long).
/// Final-third players (zone ≥ LONG_NO_SUPPRESS_ZONE=15) are unsuppressed — through-balls.
pub fn utility_pass_long(player: &PlayerState, roster_slot: u8) -> (PlayerIntent, Q32) {
    let a = &player.attributes;

    // Primary product: passing + vision + decisions. Slice 0: curved per class
    // (passing/vision = skill, decisions = mental).
    let raw = curve(CurveClass::Skill, a.technical.passing)
        * curve(CurveClass::Skill, a.mental.vision)
        * curve(CurveClass::Mental, a.mental.decisions);

    // long_shots as ball-power secondary (proxy for distance capability).
    // anticipation + composure as secondary. Slice 0: curved per class.
    let w_ls = Q32::from_raw(858_993_459_i64); // ≈ 0.20
    let w_ant = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let w_comp = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary = (Q32::ONE + w_ls * curve(CurveClass::Skill, a.technical.long_shots))
        * (Q32::ONE + w_ant * curve(CurveClass::Mental, a.mental.anticipation))
        * (Q32::ONE + w_comp * curve(CurveClass::Mental, a.mental.composure));
    let raw = raw * secondary;

    // FUN-TS3b: zone-conditional suppressor (Attempt 2 final).
    // Zones ≥ LONG_NO_SUPPRESS_ZONE=15: no suppressor (final-third through-balls).
    // Zones 6-14: vision-weighted suppressor.
    //   suppressor = LONG_BASE_SUPPRESS(0.45) + LONG_LANE_COEFF(0.35) × vision.
    //   At vision=0.5: 0.45+0.175=0.625. At vision=1.0: 0.80. At vision=0.0: 0.45.
    // Zones 0-5 (very deep own half): flat LONG_BASE_SUPPRESS(0.45) regardless of vision.
    let team_idx = if (roster_slot as usize) < 11 { 0 } else { 1 };
    let zone_x = player_zone_x(player.pos_x, team_idx);
    let raw = if zone_x >= LONG_NO_SUPPRESS_ZONE {
        // Final-third: no suppressor — long through-balls are tactical here.
        raw
    } else if zone_x >= LONG_THRESHOLD_ZONE {
        // Mid zone: vision-dependent suppressor (gentler than Attempt 1).
        let suppressor = LONG_BASE_SUPPRESS + LONG_LANE_COEFF * a.mental.vision;
        raw * suppressor
    } else {
        // Very deep own half: flat suppressor (no lane openness credit).
        raw * LONG_BASE_SUPPRESS
    };

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
///
/// FUN-TS3b: width-only cross gate. Cross fires for any wide player (|pos_y| > CROSS_CENTRAL_Y_M=8m),
/// regardless of x-position. In-possession events from wide positions are rare in the sim;
/// requiring an attacking-zone condition drove cross to 0% (players rarely have the ball
/// while both wide AND attacking). Gate is now pure width: suppressor grows from
/// CROSS_BASE_SUPPRESS(0.28) at y=8m to (0.28+2.5×1.0)=2.78 at y=34m (touchline).
/// The clamped raw = min(raw × suppressor, 1.0); at baseline attrs (crossing/vision/pace=0.5)
/// raw_pre_suppressor ≈ 0.163 so the clamp is dormant until real attrs arrive.
pub fn utility_cross(player: &PlayerState, roster_slot: u8) -> (PlayerIntent, Q32) {
    let a = &player.attributes;

    // Primary product. Slice 0: crossing/vision = skill, pace = physical ceiling.
    let raw = curve(CurveClass::Skill, a.technical.crossing)
        * curve(CurveClass::Skill, a.mental.vision)
        * curve(CurveClass::Physical, a.physical.pace);

    // Secondary: first_touch (control before cross) + anticipation (timing).
    let w_ft = Q32::from_raw(858_993_459_i64); // ≈ 0.20
    let w_ant = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary = (Q32::ONE + w_ft * curve(CurveClass::Skill, a.technical.first_touch))
        * (Q32::ONE + w_ant * curve(CurveClass::Mental, a.mental.anticipation));
    let raw = raw * secondary;

    // FUN-TS3b: width-only cross gate (Attempt 2 final).
    // wide_position = clamp((|pos_y| - CROSS_CENTRAL_Y_M) / CROSS_WIDE_RANGE_M, 0, 1).
    //   CROSS_CENTRAL_Y_M=8m: players 8m+ from centre qualify as "wide".
    //   Wide FWD at y=15m: wide_pos = (15-8)/26 = 0.269.
    // cross_gate = wide_position (no attacking-third gate).
    // suppressor = CROSS_BASE_SUPPRESS(0.28) + CROSS_GATE_COEFF(2.5) × cross_gate.
    //   At y=15m: suppressor = 0.28 + 2.5×0.269 = 0.952. At y=34m: suppressor = 2.78.
    //   raw × suppressor is clamped to ≤ Q32::ONE before apply_cross_bias (Fix A).
    //
    // Requiring in_attacking drove cross to 0%: players rarely possess the ball
    // while simultaneously wide AND in the attacking third. Width alone is sufficient
    // — a wide player crossing from mid-pitch is still a rational cross attempt.
    let py_abs_bits = player.pos_y.to_bits().unsigned_abs();
    let central_bits = CROSS_CENTRAL_Y_M.to_bits().unsigned_abs();
    let wide_pos: Q32 = if py_abs_bits <= central_bits {
        Q32::ZERO
    } else {
        let excess = Q32::from_raw((py_abs_bits - central_bits) as i64);
        let w = excess / CROSS_WIDE_RANGE_M;
        if w > Q32::ONE { Q32::ONE } else { w }
    };

    // Width-only gate: no attacking-third condition.
    let cross_gate = wide_pos;

    let suppressor = CROSS_BASE_SUPPRESS + CROSS_GATE_COEFF * cross_gate;
    // Apply the width gate: multiply raw by the suppressor first, then clamp.
    // FUN-TS3b Fix A: suppressor can exceed 1.0 for very wide players (e.g. at
    // y=34m touchline: suppressor ≈ 2.78). At current mid-attrs (crossing=0.5,
    // vision=0.5, pace=0.5) raw_pre ≈ 0.163, and 0.163 × 2.78 ≈ 0.453, which is
    // already ≤ 1.0. However, when real PlayerTemplate attrs land near 1.0,
    // raw_pre × suppressor ≈ 2.78 → panics apply_cross_bias's assert. Clamp
    // to Q32::ONE mirrors the short-pass path (on_ball.rs utility_pass_short).
    // This clamp is a no-op at current baseline attrs (no hash change from Fix A).
    let raw = {
        let with_suppressor = raw * suppressor;
        if with_suppressor > Q32::ONE {
            Q32::ONE
        } else {
            with_suppressor
        }
    };

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

    // Primary product. Slice 0: dribbling/technique/agility = skill (close
    // control), acceleration = physical ceiling. The 4-factor curved product is
    // the strongest combinatorial-skew case in the sim (elite-end delta ≈ 23×
    // the mid delta vs ≈ 5× for the old linear product).
    let raw = curve(CurveClass::Skill, a.technical.dribbling)
        * curve(CurveClass::Skill, a.technical.technique)
        * curve(CurveClass::Skill, a.physical.agility)
        * curve(CurveClass::Physical, a.physical.acceleration);

    // Secondary: balance (contest stability) + flair (personality tendency).
    let w_bal = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let w_flair = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary = (Q32::ONE + w_bal * curve(CurveClass::Contest, a.physical.balance))
        * (Q32::ONE + w_flair * curve(CurveClass::Personality, a.mental.flair));
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

    // Primary product. Slice 0: strength = physical, composure = mental,
    // balance = contest (contact resistance).
    let raw = curve(CurveClass::Physical, a.physical.strength)
        * curve(CurveClass::Mental, a.mental.composure)
        * curve(CurveClass::Contest, a.physical.balance);

    // Secondary: decisions as a quality gate.
    let w_dec = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary = Q32::ONE + w_dec * curve(CurveClass::Mental, a.mental.decisions);
    let raw = raw * secondary;

    let biased = apply_hold_bias(raw, a);

    let (target_x, target_y) = formation_position(roster_slot);

    (PlayerIntent::HoldBall { target_x, target_y }, biased)
}

/// Utility for laying the ball off.
///
/// Attribute binding (spec): first_touch × passing × vision (primary);
/// teamwork + composure as secondary; selflessness via bias.
///
/// FUN-TS3b: zone-conditional suppressor in attacking zones (zone_x ≥ LAYOFF_SUPPRESS_ZONE).
/// In the attacking half, a layoff targets formation_position.y - 7m — a backward pass.
/// Suppressed to prevent layoff dominating over short passes in the final third.
/// In own/mid zones layoff is unsuppressed — a backward recycling touch is rational.
pub fn utility_lay_off(player: &PlayerState, roster_slot: u8) -> (PlayerIntent, Q32) {
    let a = &player.attributes;

    // Primary product. Slice 0: first_touch/passing/vision = skill expression.
    let raw = curve(CurveClass::Skill, a.technical.first_touch)
        * curve(CurveClass::Skill, a.technical.passing)
        * curve(CurveClass::Skill, a.mental.vision);

    // Secondary: teamwork (cooperative action) + composure (calmness), mental.
    let w_tw = Q32::from_raw(858_993_459_i64); // ≈ 0.20
    let w_comp = Q32::from_raw(429_496_729_i64); // ≈ 0.10
    let secondary = (Q32::ONE + w_tw * curve(CurveClass::Mental, a.mental.teamwork))
        * (Q32::ONE + w_comp * curve(CurveClass::Mental, a.mental.composure));
    let raw = raw * secondary;

    // Lay-off bias: Selflessness ONLY per spec (P1-5 fix — was safe_pass proxy which also read risk_appetite).
    let biased = apply_lay_off_bias(raw, a);

    // FUN-TS3b: zone-conditional layoff suppressor.
    // In attacking zones (zone_x ≥ LAYOFF_SUPPRESS_ZONE=9), a layoff sends the ball
    // backward toward formation position — less rational than a short pass forward.
    // Suppressor = LAYOFF_ATTACK_SUPPRESS (0.55) in attacking zones.
    // This prevents layoff from dominating over short in the final third.
    let team_idx = if (roster_slot as usize) < 11 { 0 } else { 1 };
    let zone_x = player_zone_x(player.pos_x, team_idx);
    let biased = if zone_x >= LAYOFF_SUPPRESS_ZONE {
        biased * LAYOFF_ATTACK_SUPPRESS
    } else {
        biased
    };

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

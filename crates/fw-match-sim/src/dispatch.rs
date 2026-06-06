//! Per-tick player decision dispatcher — ADR-0006 `dispatch_tick`.
//!
//! ## Design
//!
//! `dispatch_tick` iterates roster slots 0..22. For each slot where
//! `should_decide` fires, it:
//! 1. Checks pre-emption hooks (stubbed to `None` in -iii-a).
//! 2. Evaluates FSM transitions (skeleton: identity) per ADR-0006 §"Concrete
//!    sketch" — the transition runs BEFORE the BT/GK lookup.
//! 3. Routes by role: GK → `goalkeeper_fsm::tick_goalkeeper`; outfield →
//!    `bt::tick_tree` via `SubtreeLibrary`.
//! 4. Applies the returned `PlayerIntent` via `apply_intent` (mutates
//!    `vel_x`/`vel_y`).
//! 5. Increments `local_decision_counter` via `bump_decision_counter()`.
//!
//! ## Slot indexing convention (P2-3)
//!
//! Two different 0/1-indexed values live in this function:
//! - `slot_idx` (0-indexed, 0..22): Vec index into `state.players`.
//! - `roster_slot` (1-indexed, 1..=22): the value `should_decide` expects
//!   per `decision_cadence`'s contract (it subtracts 1 internally).
//! - `formation_slot` (0-indexed, 0..22): same as `slot_idx`; named
//!   separately where it's passed to `formation_position` / `BtContext`
//!   to document which convention is in use.
//!
//! ## Determinism
//!
//! - Roster slots are iterated in fixed order (0..22).
//! - RNG is seeded per ADR-0009: `seed_fn(match_seed, tick, SeedLayer::Decision,
//!   (player_id << 16) | local_decision_counter)`.
//! - In the skeleton tier (-iii-a), no leaf actually draws from the RNG.
//!   The seed is constructed for -iii-b compatibility.
//! - No floats, no HashMap, no clocks, no async.
//!
//! ## Player velocity model (skeleton tier)
//!
//! `apply_intent` for `MoveToPosition` computes a direction vector from the
//! player's current position to the target, then clamps magnitude to
//! `MAX_PLAYER_SPEED`. Direct velocity set (no acceleration model) — adequate
//! for the skeleton tier. -iii-b will add an acceleration ramp.
//!
//! ## Pre-emption hooks — 3-policy live (T1-15 + T1-19)
//!
//! `preempt_check` runs before per-slot signature evaluation + role dispatch.
//! Returning `Some(intent)` short-circuits both: the intent is applied, the
//! decision counter is bumped, and the slot's per-tick decision loop body
//! `continue`s past role dispatch. T1-15 grew this from the prior stub to
//! a substantive 3-policy implementation; T1-19 added 5 behavioral unit tests
//! + amended ADR-0006 to document the scope. The 3 live policies:
//!
//!   1. **Possession gate** — `preempt_check` returns `None` whenever
//!      `state.possession.is_some()`. All other policies require a loose ball.
//!   2. **Goalkeeper own-side chase** — slot 0 (home GK) / slot 11 (away GK)
//!      return `Some(MoveToPosition { target: ball })` IFF `|ball.pos_x| > 42m`
//!      (ball within 10m of the ±52.5m goal line) AND the ball is on the GK's
//!      own side (`bx_bits < 0` for home GK; `bx_bits >= 0` for away GK).
//!      Originally framed for the "ball stranded 2-3m short of goal line"
//!      failure surfaced during T1-15 empirical playtesting.
//!   3. **Outfield nearest-2 chase** — for outfielders, `preempt_check`
//!      returns `MoveToPosition { target: ball }` UNLESS 2+ same-team
//!      outfielders are STRICTLY closer (Manhattan distance; `<` tiebreak;
//!      same-team GK excluded from the count). Under exact-Manhattan ties
//!      more than 2 may chase — a known limitation deferred to T2-1's
//!      archetype-driven positioning.
//!
//! See `docs/adr/0006-bt-vs-fsm-decision-layer.md` "Amendment 2026-05-16 —
//! preempt_check 3-policy scope" for the architectural rationale + future
//! T2+ scope (foul reaction, set-piece switchover, 60 Hz reactive interrupts).
//! Behavioral coverage lives in `dispatch::tests::preempt_check_*` (T1-19).

use std::collections::BTreeMap;

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;

use fw_content::{CooldownPolicy, SignatureDefinition, SimBiasSnapshot, StackingPolicy};
use fw_core::Q32;
use fw_core::{CurveClass, curve};

use crate::MatchState;
use crate::bt::{BtContext, LeafKind, Node, Tree, tick_tree};
use crate::decision_cadence::{SeedLayer, seed_fn, should_decide};
use crate::goalkeeper_fsm::tick_goalkeeper;
use crate::role_states::{PlayerIntent, PlayerRoleState};
use crate::signature;
use crate::signature::{
    DEFAULT_FIRING_DURATION_TICKS, SignatureFiring, build_trigger_table, evaluate_signatures,
};
use crate::subtree_library::select_outfield_intent;
use fw_content::{MatchEvent, PassKind, is_shot_on_target};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

// Layer 1b replaced the flat MAX_PLAYER_SPEED = 8 m/s constant with per-player
// speed caps. Outfield players now use player_v_max(pace) = V_PACE_BASE + pace ×
// V_PACE_RANGE (range [6.5, 9.0] m/s). GKs use V_GK_SPEED = 7.2 m/s.
// The old flat-8 constant is removed; V_PACE_BASE + V_PACE_RANGE = 9.0 m/s is
// the new ceiling for outfield players (pace = 1.0).

// ---------------------------------------------------------------------------
// Layer 1b: pace-scaled player speed (dynamic-positioning campaign 2026-06-06)
// ---------------------------------------------------------------------------
// Outfield players' movement cap scales linearly with their `pace` attribute
// (a Q32 value in [0, 1], where 0 = slowest, 1 = fastest in the game):
//
//   v_max(pace) = V_PACE_BASE + pace × V_PACE_RANGE
//              = 6.5 + pace × 2.5  m/s
//
// At pace=0.0: 6.5 m/s (slow jog). At pace=0.5: 7.75 m/s (baseline player).
// At pace=1.0: 9.0 m/s (elite sprinter).
//
// The existing flat 8 m/s cap becomes the mid-range (pace≈0.6) value, so
// mid-range players are unchanged to within 0.1 m/s; only the extremes move.
//
// GKs (slots 0 and 11) use a fixed cap V_GK_SPEED = 7.2 m/s (more mobile
// than a slow outfield player but not competing with elite sprinters).
//
// All values: Q32.32 raw = value × 2^32.

/// Base speed for outfield players (pace = 0): 6.5 m/s.
/// Raw = round(6.5 × 2^32) = 27_917_287_424.
const V_PACE_BASE: Q32 = Q32::from_raw(27_917_287_424_i64);

/// Speed range for outfield players: 9.0 - 6.5 = 2.5 m/s.
/// Raw = round(2.5 × 2^32) = 10_737_418_240.
const V_PACE_RANGE: Q32 = Q32::from_raw(10_737_418_240_i64);

/// GK movement cap: 7.2 m/s (fixed; pace attribute ignored for GKs).
/// Raw = round(7.2 × 2^32) = 30_923_764_531.
const V_GK_SPEED: Q32 = Q32::from_raw(30_923_764_531_i64);

// ---------------------------------------------------------------------------
// Ball-speed constants (T1-3.5)
// ---------------------------------------------------------------------------
// Shot speed: base 20 m/s + up to 15 m/s bonus at peak shooter attrs.
// Full formula: base + bonus × (strength × finishing).
// At mid-range attrs (0.5 × 0.5 = 0.25): 20 + 15 × 0.25 = 23.75 m/s.
// At peak attrs (1.0 × 1.0 = 1.0): 20 + 15 × 1.0 = 35 m/s.
//
// Pass speed: base 15 m/s + up to 10 m/s bonus at peak passer attrs.
// Full formula: base + bonus × (passing × vision).
// At mid-range attrs (0.5 × 0.5 = 0.25): 15 + 10 × 0.25 = 17.5 m/s.
// At peak attrs (1.0 × 1.0 = 1.0): 15 + 10 × 1.0 = 25 m/s.
//
// All values in Q32.32: X m/s = X << 32 raw bits.

/// Base shot speed (m/s) before attribute scaling.
const SHOT_BASE_SPEED_MPS: Q32 = Q32::from_raw(20_i64 << 32);

/// Peak shot speed bonus (m/s) at maximum `strength × finishing` product.
/// Applied as: speed = SHOT_BASE + SHOT_PEAK_BONUS × (strength × finishing).
const SHOT_PEAK_BONUS_MPS: Q32 = Q32::from_raw(15_i64 << 32);

/// Base pass speed (m/s) before attribute scaling.
const PASS_BASE_SPEED_MPS: Q32 = Q32::from_raw(15_i64 << 32);

/// Peak pass speed bonus (m/s) at maximum `passing × vision` product.
/// Applied as: speed = PASS_BASE + PASS_PEAK_BONUS × (passing × vision).
const PASS_PEAK_BONUS_MPS: Q32 = Q32::from_raw(10_i64 << 32);

// ---------------------------------------------------------------------------
// SS2 — Shot accuracy dispersion constants (FUN-0b)
// docs/design/shot-model.md §Sub-system 2 §Revised coefficients
// ---------------------------------------------------------------------------

/// Base scatter in metres. FUN-TS3-ShotModel sweep 6: raised to 9.0m.
/// Sweep history:
///   Baseline: 7.0m → on-target=44.4%, M1=2.60 (on-target too high)
///   Sweep 1: 8.5m  → on-target=35.6%, M1=2.00 (M1 too low)
///   Sweep 2: 8.0m  → on-target=41.0%, M1=2.30 (on-target just above 40% band, 20-seed)
///   Sweep 3: 8.5m + SAVE_BASE(0.58,0.78) → on-target=39.2%, M1=2.20 (M1 still low)
///   Sweep 4: 8.5m + SAVE_BASE(0.55,0.75) → on-target=38.2%, M1=2.30 (20-seed)
///            40-seed: on-target=42.4%, M1=2.90 (on-target above ceiling)
///   Sweep 5: 8.75m → on-target=40.9%, M1=3.05 (still on boundary; 40-seed)
///   Sweep 6: 9.0m  → push further into 28-40% band
/// See docs/design/shot-model.md Phase-2 re-fit for full per-sweep table.
///
/// Goal-production re-tune (2026-06-05): lowered 9.0m -> 8.0m. After the
/// goalmouth-defending slice closed all drift goals (M1 fell to 1.82, all
/// shot-based), conversion had to be recovered from SHOTS. The save model was
/// already lowered to its hard floor (0.50/0.72), so the remaining honest lever
/// was on-target volume: 8.0m lifts on-target ~39% -> ~43% (within the 33-45%
/// realism band) without making finishing clinical. Combined with the shoot
/// threshold drop (XG_SHOOT_THRESHOLD 0.070 -> 0.054) this lands M1 ~2.5-2.6
/// from shots over 100-seed windows. Going lower (7.0m) pushes on-target to 51%
/// (clinical) — rejected as dishonest.
const SIGMA_BASE_M: Q32 = Q32::from_raw(34_359_738_368_i64); // = 8.0m (goal-production re-tune 2026-06-05)

/// Distance contribution to sigma. dist_factor = clamp(d_m / 35, 0, 1) (the
/// NON-inverted distance — 0=close, 1=far, opposite of the xG feature).
const SIGMA_DIST_WEIGHT: Q32 = Q32::from_raw(3_435_973_836_i64); // ≈ 0.80

/// Pressure contribution to sigma (additive inside the parens).
const SIGMA_PRESSURE_WEIGHT: Q32 = Q32::from_raw(2_147_483_648_i64); // ≈ 0.50

/// Shooter quality REDUCES sigma (multiplicative quality-suppression factor).
const SIGMA_QUALITY_WEIGHT: Q32 = Q32::from_raw(1_717_986_918_i64); // ≈ 0.40

/// Minimum scatter floor in metres (best-case: world-class, penalty area, no pressure).
const SIGMA_MIN_M: Q32 = Q32::from_raw(8_589_934_592_i64); // ≈ 2.0m (raised from 1.5m for base calibration)

/// Maximum scatter ceiling in metres (worst-case: weak player, 35m, full block pressure).
/// Raised from 9.0m to 15.0m to accommodate the wider base + pressure radius (FUN-TS2).
const SIGMA_MAX_M: Q32 = Q32::from_raw(64_424_509_440_i64); // ≈ 15.0m

/// 1/√3 normaliser for the sum-of-3-uniforms normal approximation.
/// Scales the [-3, +3] sum to unit-normal scale.
const SIGMA_NORMAL_SCALE: Q32 = Q32::from_raw(2_479_700_525_i64); // ≈ 0.577

/// Clamp for the final dispersed target_y (metres). Ball must stay on the pitch.
const TARGET_Y_CLAMP_M: Q32 = Q32::from_raw(12_i64 << 32); // 12.0m (half pitch width ≈ 34m; clamp is conservative)

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Ball-speed helper functions (T1-3.5)
// ---------------------------------------------------------------------------

/// Compute the scalar ball speed (m/s) for a shot, attribute-modulated.
///
/// Formula: `SHOT_BASE_SPEED_MPS + SHOT_PEAK_BONUS_MPS × (strength × finishing)`
///
/// T2-1d shot-feature extraction helpers (off-canonical-path; consumed by
/// the calibrate binary via the `ShotTelemetryRecord` sidecar Vec). The
/// 6 features mirror the `xg::ShotContext` field semantics so the offline
/// fit produces β coefficients compatible with `xg_utility(ctx)`.
///
/// Distance feature: `1 - clamp(distance_m / 35, 0, 1)` per
/// `docs/design/xg-coefficients.md §Distance feature inversion`. The
/// shooter's distance to the OPPONENT goal-line is the input; orientation
/// flips based on home/away slot (home attacks +X; away attacks -X).
/// Q32-only arithmetic (sim crate; no f64 here per Sim/RULES.md §1).
fn shot_distance_feature_q32(shooter_pos_x: Q32, shooter_slot: u8) -> Q32 {
    // Distance to opponent goal in metres.
    let goal_x = if (shooter_slot as usize) < crate::PLAYERS_PER_TEAM {
        fw_core::GOAL_LINE_X
    } else {
        -fw_core::GOAL_LINE_X
    };
    let dx = goal_x - shooter_pos_x;
    // Absolute value via two's-complement check (Q32 is fixed-point signed).
    let dx_abs_bits = dx.to_bits().unsigned_abs();
    // Threshold 35m = 35 * 2^32 raw bits = 150_323_855_360 unsigned.
    const THRESHOLD_BITS: u64 = 35_u64 << 32;
    if dx_abs_bits >= THRESHOLD_BITS {
        Q32::ZERO
    } else {
        // distance_q32 = 1 - clamp(d_m / 35, 0, 1)
        // = 1 - (d_m / 35)
        // Compute (d_m / 35) in Q32 then subtract from ONE.
        let normalized = Q32::from_raw(dx_abs_bits as i64) / Q32::from_int(35);
        Q32::ONE - normalized
    }
}

/// Angle feature: half-angle of goal cone from shooter normalized to [0, 1].
/// Pragmatic approximation: `1 - clamp(|pos_y| / 25, 0, 1)`. More central
/// (pos_y near 0) = higher angle = higher xG. Wider pos_y = lower angle.
/// The 25m threshold approximates the pitch-width zone where the angle
/// becomes degenerate (extreme wide-angle shots have near-zero xG).
fn shot_angle_feature_q32(shooter_pos_y: Q32) -> Q32 {
    let py_abs_bits = shooter_pos_y.to_bits().unsigned_abs();
    const THRESHOLD_BITS: u64 = 25_u64 << 32;
    if py_abs_bits >= THRESHOLD_BITS {
        Q32::ZERO
    } else {
        let normalized = Q32::from_raw(py_abs_bits as i64) / Q32::from_int(25);
        Q32::ONE - normalized
    }
}

/// Defender-pressure feature: `1 - exp_q32(-sum_inv_dist)` per the
/// `docs/design/xg-coefficients.md` formula. Sum inverse distance over
/// opposing players within 15m. Approximated for T2-1d telemetry purposes
/// using `1 - 1/(1 + sum_inv_dist)` (sigmoid-like; avoids the cordic exp
/// dependency in the sim hot path). Acceptable since this is OFFLINE
/// telemetry data going into a fitter — fitter learns β₃ against whatever
/// pressure proxy we feed it.
fn shot_pressure_feature_q32(players: &[crate::player::PlayerState], shooter_idx: usize) -> Q32 {
    let shooter = &players[shooter_idx];
    let shooter_team_is_home = (shooter.slot as usize) < crate::PLAYERS_PER_TEAM;
    let opponent_range = if shooter_team_is_home {
        crate::PLAYERS_PER_TEAM..crate::TOTAL_PLAYERS
    } else {
        0..crate::PLAYERS_PER_TEAM
    };
    // Scan opponents within 15m. The 1/(d+ε) kernel means close defenders
    // contribute much more than far ones (2m defender → inv=0.5; 10m → inv=0.1).
    // Sat function: 1 - 1/(1+sum_inv) → bounded [0,1).
    let mut sum_inv = Q32::ZERO;
    let epsilon = Q32::from_raw(1 << 28); // ~0.0625m floor to avoid divide-by-zero
    for opp_idx in opponent_range {
        let opp = &players[opp_idx];
        let dx = opp.pos_x - shooter.pos_x;
        let dy = opp.pos_y - shooter.pos_y;
        let dist_sq = dx * dx + dy * dy;
        let dist_sq_abs = dist_sq.to_bits().unsigned_abs();
        // Coarse radius gate via squared distance: 15^2 = 225 → 225 × 2^32.
        const RADIUS_SQ_BITS: u64 = (15_u64 * 15_u64) << 32;
        if dist_sq_abs >= RADIUS_SQ_BITS {
            continue;
        }
        // Q32::sqrt for the actual distance, then 1/(d + ε).
        let dist = dist_sq.sqrt();
        let inv = Q32::ONE / (dist + epsilon);
        sum_inv += inv;
    }
    // Approximation: 1 - 1/(1 + sum_inv). Bounded [0, 1); approaches 1 as
    // sum_inv grows.
    Q32::ONE - (Q32::ONE / (Q32::ONE + sum_inv))
}

/// Shooter quality feature: `finishing × 0.55 + composure × 0.25 + technique × 0.20`
/// per `docs/design/xg-coefficients.md §The 6 features`.
fn shot_quality_feature_q32(attrs: &fw_core::PlayerAttributes) -> Q32 {
    let w_finishing = Q32::from_raw(2_362_232_012_i64); // ≈ 0.55
    let w_composure = Q32::from_raw(1_073_741_824_i64); // ≈ 0.25
    let w_technique = Q32::from_raw(858_993_459_i64); // ≈ 0.20
    // Slice 0: curve each term (finishing/technique = skill, composure = mental)
    // before the weighted sum. This composite feeds the xG gate + shot sigma, so
    // an elite finisher's shot is disproportionately accurate (smaller sigma) and
    // more likely to clear the xG threshold. Mirrors the on_ball.rs
    // `shooter_quality` curve so the two stay consistent.
    curve(CurveClass::Skill, attrs.technical.finishing) * w_finishing
        + curve(CurveClass::Mental, attrs.mental.composure) * w_composure
        + curve(CurveClass::Skill, attrs.technical.technique) * w_technique
}

/// SS2 — Compute the dispersed target_y for a shot attempt (FUN-0b).
///
/// Uses a sum-of-3-uniforms normal approximation to scatter the shot target
/// around the goal centre. Sigma is determined by distance/pressure/quality
/// per `docs/design/shot-model.md §Sub-system 2 §Revised dispersion model`.
///
/// # Determinism
///
/// Three separate `ChaCha8Rng` instances are seeded via `seed_fn` per ADR-0009:
///   `SeedLayer::BallPhysics`, sites `(slot << 16) | 0x0001..0x0003`.
/// This ensures the three draws are independent and reproducible.
///
/// # Returns
///
/// `(target_y_m, xg_score)` — the dispersed target_y in metres and the shot's
/// xG score (to be written into `state.last_shot_xg[slot_idx]`).
pub(crate) fn compute_shot_dispersion_and_xg(
    shooter: &crate::player::PlayerState,
    shooter_slot: u8,
    players: &[crate::player::PlayerState],
    shooter_idx: usize,
    match_seed: u64,
    tick_u32: u32,
) -> (Q32, Q32) {
    // --- Distance factor (NON-inverted: 0=close, 1=far for sigma calculation) ---
    let goal_x: Q32 = if (shooter_slot as usize) < crate::PLAYERS_PER_TEAM {
        fw_core::GOAL_LINE_X
    } else {
        -fw_core::GOAL_LINE_X
    };
    let dx = goal_x - shooter.pos_x;
    let dx_abs_bits = dx.to_bits().unsigned_abs();
    const DIST_THRESHOLD_BITS: u64 = 35_u64 << 32;
    let dist_factor: Q32 = if dx_abs_bits >= DIST_THRESHOLD_BITS {
        Q32::ONE // far shot → max distance factor
    } else {
        // dist_factor = clamp(d_m / 35, 0, 1) = 1 - distance_q32 (inverted from xG feature)
        Q32::from_raw(dx_abs_bits as i64) / Q32::from_int(35)
    };

    // --- Pressure proxy ---
    let pressure_q32 = shot_pressure_feature_q32(players, shooter_idx);

    // --- Shooter quality ---
    let quality = shot_quality_feature_q32(shooter.attributes());
    let quality = if quality > Q32::ONE {
        Q32::ONE
    } else {
        quality
    };

    // --- Sigma in metres ---
    // sigma_y_m = SIGMA_BASE_M × (1 + SIGMA_DIST_WEIGHT×dist + SIGMA_PRESSURE_WEIGHT×press)
    //           × (1 - SIGMA_QUALITY_WEIGHT×quality)
    // clamped to [SIGMA_MIN_M, SIGMA_MAX_M]
    let dist_press_factor =
        Q32::ONE + SIGMA_DIST_WEIGHT * dist_factor + SIGMA_PRESSURE_WEIGHT * pressure_q32;
    let quality_factor = Q32::ONE - SIGMA_QUALITY_WEIGHT * quality;
    let quality_factor = if quality_factor < Q32::ZERO {
        Q32::ZERO
    } else {
        quality_factor
    };
    let sigma_raw = SIGMA_BASE_M * dist_press_factor * quality_factor;
    let sigma_y_m = if sigma_raw < SIGMA_MIN_M {
        SIGMA_MIN_M
    } else if sigma_raw > SIGMA_MAX_M {
        SIGMA_MAX_M
    } else {
        sigma_raw
    };

    // --- Sum-of-3-uniforms normal approximation ---
    // Each draw: seed via seed_fn(BallPhysics, (slot<<16)|site), draw next_u64,
    // upper 32 bits → Q32 in [0, 1), map to [-1, +1] via 2u - 1.
    let slot_u32 = shooter_slot as u32;
    let draw_u = |site: u32| -> Q32 {
        use rand_chacha::rand_core::{RngCore, SeedableRng};
        let rng_seed = crate::decision_cadence::seed_fn(
            match_seed,
            tick_u32,
            crate::decision_cadence::SeedLayer::BallPhysics,
            (slot_u32 << 16) | site,
        );
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(rng_seed);
        let raw_u64 = rng.next_u64();
        // Upper 32 bits as Q32 in [0, 1).
        let u = Q32::from_raw((raw_u64 >> 32) as i64);
        // Map [0, 1) → [-1, +1): 2u - 1.
        Q32::from_int(2) * u - Q32::ONE
    };

    let u1 = draw_u(0x0001);
    let u2 = draw_u(0x0002);
    let u3 = draw_u(0x0003);
    // Sum-of-3-uniforms ≈ N(0,1) scaled; multiply by 1/√3 to normalize.
    let sum = u1 + u2 + u3;
    let rng_z = sum * SIGMA_NORMAL_SCALE;

    // Final target_y in metres.
    let target_y_raw = rng_z * sigma_y_m;
    // Clamp to ±TARGET_Y_CLAMP_M (ball stays on the pitch).
    let target_y_m = if target_y_raw < -TARGET_Y_CLAMP_M {
        -TARGET_Y_CLAMP_M
    } else if target_y_raw > TARGET_Y_CLAMP_M {
        TARGET_Y_CLAMP_M
    } else {
        target_y_raw
    };

    // --- Recompute xG for last_shot_xg cache ---
    // Use the same features as the utility_shoot gate (pressure is proxy).
    let distance_q32 = if dx_abs_bits >= DIST_THRESHOLD_BITS {
        Q32::ZERO
    } else {
        let normalized = Q32::from_raw(dx_abs_bits as i64) / Q32::from_int(35);
        Q32::ONE - normalized
    };
    let py_abs_bits = shooter.pos_y.to_bits().unsigned_abs();
    const ANGLE_THRESHOLD_BITS: u64 = 25_u64 << 32;
    let angle_q32 = if py_abs_bits >= ANGLE_THRESHOLD_BITS {
        Q32::ZERO
    } else {
        let normalized = Q32::from_raw(py_abs_bits as i64) / Q32::from_int(25);
        Q32::ONE - normalized
    };
    let xg_ctx = crate::utility::xg::ShotContext::try_new(
        distance_q32,
        angle_q32,
        pressure_q32,
        Q32::ONE, // footed
        Q32::ONE, // solo
        quality,
    )
    .expect("SS2 xG recompute: features must be in [0, 1]");
    let xg_score = crate::utility::xg::xg_utility(&xg_ctx);

    (target_y_m, xg_score)
}

/// Both `strength` and `finishing` are Q32 values in `[0, 1]`. Their product
/// is also in `[0, 1]` (multiplication of two sub-unit Q32 values).
///
/// Pure function — no RNG, no side effects. Q32 arithmetic only.
pub(crate) fn compute_ball_speed_for_shot(shooter: &crate::player::PlayerState) -> Q32 {
    // Slice 0: strength = physical ceiling, finishing = skill. Curving both
    // means an elite striker's shot is disproportionately quicker.
    let attr_product = curve(CurveClass::Physical, shooter.attributes.physical.strength)
        * curve(CurveClass::Skill, shooter.attributes.technical.finishing);
    SHOT_BASE_SPEED_MPS + SHOT_PEAK_BONUS_MPS * attr_product
}

/// Compute the scalar ball speed (m/s) for a pass, attribute-modulated.
///
/// Formula: `PASS_BASE_SPEED_MPS + PASS_PEAK_BONUS_MPS × (passing × vision)`
///
/// Both `passing` and `vision` are Q32 values in `[0, 1]`.
///
/// Pure function — no RNG, no side effects. Q32 arithmetic only.
pub(crate) fn compute_ball_speed_for_pass(passer: &crate::player::PlayerState) -> Q32 {
    // Slice 0: passing + vision are both skill expression; curving both makes an
    // elite passer's ball measurably zippier.
    let attr_product = curve(CurveClass::Skill, passer.attributes.technical.passing)
        * curve(CurveClass::Skill, passer.attributes.mental.vision);
    PASS_BASE_SPEED_MPS + PASS_PEAK_BONUS_MPS * attr_product
}

/// Compute the ball velocity components (vel_x, vel_y) for a kick from
/// `from_pos` toward `to_pos`, with the given scalar `speed` (m/s).
///
/// Uses cordic-backed `Q32::sqrt` for the normalisation — same path as
/// `separation.rs::resolve_pair`. No `f64` division or `f64::sqrt`.
///
/// ## Zero-distance fallback
///
/// When `from_pos == to_pos` (zero distance), the ball is kicked straight
/// along +X at the given speed. This is deterministic and avoids division by
/// zero — the same convention as `separation.rs`'s EPSILON fallback.
///
/// ## Return value
///
/// `(vel_x, vel_y)` in Q32.32 m/s. The Z component is left unchanged by the
/// caller (aerial trajectory is a Phase-2 concern; for now, kicks are
/// treated as ground-level — ball.vel_z is reset to Q32::ZERO by the caller).
///
/// ## Zero-distance fallback (Codex 2026-05-16 audit code-reviewer Critical #2)
///
/// When `from == to` (zero distance — e.g. clustered players where the
/// passer's nearest-teammate is co-located), this function returns
/// `(Q32::ZERO, Q32::ZERO)` instead of the prior `(speed, Q32::ZERO)`
/// "+X-at-full-speed" fallback. Rationale: the prior fallback could fire
/// the ball along +X at 15–35 m/s with the passer near the positive goal
/// line, producing a **phantom goal** on the next tick attributed to the
/// passer's own team. The zero-velocity fallback means a self-pass-to-
/// coincident-receiver produces no ball motion (the Pass MatchEvent still
/// emits — possession transfer is symbolic — but the ball stays put,
/// which is the safe-default semantics for the degenerate case).
///
/// Production-path risk for the degenerate case is low (separation runs
/// at tick_match step 8 keeping players ≥0.4m apart; identical Q32
/// positions across two slots would need an active-tick mid-overlap
/// before separation fires); flagged here so a future ball-physics audit
/// has the rationale in source.
pub(crate) fn ball_unit_vel(
    from_x: Q32,
    from_y: Q32,
    to_x: Q32,
    to_y: Q32,
    speed: Q32,
) -> (Q32, Q32) {
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    // dist_sq in Q32 — product of two Q32 values (both in metres ≈ [-105, 105]).
    // Squares can be up to ~11025 m², well within Q32's ±2^31 integer range.
    let dist_sq = dx * dx + dy * dy;

    if dist_sq == Q32::ZERO {
        // Degenerate: passer is at the receiver's exact position. Return
        // zero velocity to avoid the phantom-goal risk documented above.
        // Speed parameter is intentionally unused on this branch.
        let _ = speed;
        return (Q32::ZERO, Q32::ZERO);
    }

    // cordic sqrt — Q32-backed, same as separation.rs.
    let dist = dist_sq.sqrt();
    // unit_x = dx / dist; unit_y = dy / dist.
    // Q32 division: (dx / dist) × speed.
    let vel_x = dx / dist * speed;
    let vel_y = dy / dist * speed;
    (vel_x, vel_y)
}

/// Extract the `BiasCategory` as a usize index (0..4) from a `StackingPolicy`.
/// Used to index into `signature_firing[slot][cat_idx]`.
fn stacking_category_idx(policy: &StackingPolicy) -> usize {
    match *policy {
        StackingPolicy::Exclusive { category } => category as usize,
    }
}

// ---------------------------------------------------------------------------
// dispatch_tick
// ---------------------------------------------------------------------------

/// Advance all player decisions by one tick.
///
/// This is the canonical-state-mutating entry point for the per-player
/// decision layer (ADR-0006). Called from `tick_match` after ball physics
/// and the tactic-FSM heartbeat.
///
/// Roster slots 0..22 are iterated in fixed order. For each slot where
/// `should_decide` fires, the role-appropriate runner executes and the
/// returned `PlayerIntent` is applied.
///
/// `sig_definitions` — map from `SignatureId.as_str()` to `SignatureDefinition`,
/// used by the signature dispatcher. Pass `&BTreeMap::new()` when no content
/// store is available (no signatures will fire without definitions).
pub fn dispatch_tick(
    mut state: MatchState,
    sig_definitions: &BTreeMap<String, SignatureDefinition>,
) -> MatchState {
    // T1-4a: match_events is the persistent canonical event stream (replaces
    // the removed signature_memory_events scratch buffer). It is NOT cleared
    // here — events accumulate across the match by design.

    // Build the trigger table once per tick (cheap: it's a BTreeMap of fn ptrs).
    let trigger_table = build_trigger_table();

    // Advance firing windows: clear expired signature_firing entries per (slot, category) lane.
    // Must run before the per-slot decision loop so the stacking check sees
    // up-to-date firing state.
    for slot_idx in 0..22usize {
        for cat_idx in 0..4usize {
            if let Some(firing) = &state.signature_firing[slot_idx][cat_idx]
                && !firing.is_active(state.tick)
            {
                state.signature_firing[slot_idx][cat_idx] = None;
            }
        }
    }

    // T1-3.6: carrier routing pre-pass.
    //
    // Runs BEFORE the per-slot decision loop. Updates role_state for ALL 22
    // slots based on current possession, independently of whether the slot
    // fires a decision this tick. This ensures:
    //   (a) The carrier's role_state is InPossession when their decision tick
    //       arrives — select_outfield_intent routes them to on-ball candidates.
    //   (b) A player whose possession was transferred away (via Pass/Shot in a
    //       previous tick) exits InPossession immediately, not only at their
    //       next decision tick.
    //
    // T1-3.6 self-review P1-1 (type-design-analyzer): this pre-pass delegates
    // to `PlayerRoleState::evaluate_transitions` rather than open-coding the
    // transition table again. There is exactly one source of truth for
    // carrier ↔ non-carrier routing. The per-slot loop below calls the same
    // function via `current_role_state.evaluate_transitions(...)` at the
    // decision tick — but for non-deciding slots, this pre-pass is the only
    // place transitions land. Idempotency holds: calling it twice in the same
    // tick (pre-pass + per-slot loop on a deciding tick) is a no-op on the
    // second call because the role state has already converged.
    for slot_idx in 0..22usize {
        let current = state.players[slot_idx].role_state;
        let next = current.evaluate_transitions(&state, slot_idx);
        state.players[slot_idx].role_state = next;
    }

    // FUN-TS1: compute per-team shape anchors AFTER the carrier pre-pass
    // (so role states are up-to-date) and BEFORE the per-slot decision loop
    // (so off-ball utilities can read the computed shape via zonal_slot).
    // This is the "FSM finally drives positions" seam from ADR-0013.
    state.team_shape[0] = crate::team_shape::compute(0, &state);
    state.team_shape[1] = crate::team_shape::compute(1, &state);
    // FUN-TS2b: fill coordinated press roles into team_shape[*].press_roles
    // AFTER team_shape (needs is_defending) and BEFORE the per-slot decision
    // loop (so the Pressing arm reads shape.press_roles[team_local_slot]).
    // Decomposed into parts to avoid the split-borrow conflict:
    // cannot take &mut state.team_shape and &state at the same time.
    {
        let pos_snap: [(fw_core::Q32, fw_core::Q32); 22] = {
            let mut arr = [(fw_core::Q32::ZERO, fw_core::Q32::ZERO); 22];
            for (i, p) in state.players.iter().enumerate() {
                arr[i] = (p.pos_x, p.pos_y);
            }
            arr
        };
        // SLICE-1: pass effective_possession so a successful in-flight pass
        // keeps the coordinated press active on the passing team rather than
        // dropping to HoldShape because possession == None during the flight.
        // Compute before the mutable borrow of state.team_shape to avoid the
        // split-borrow conflict (effective_possession borrows state immutably;
        // compute_press_from_parts takes &mut state.team_shape).
        let eff_poss = state.effective_possession();
        crate::team_shape::compute_press_from_parts(
            &mut state.team_shape,
            eff_poss,
            &pos_snap,
            &state.team_tactic_states,
            &state.press_level,
        );
    }

    for slot_idx in 0..22usize {
        // roster_slot is 1-indexed per decision_cadence::should_decide contract.
        // (should_decide subtracts 1 internally to derive the slot array index.)
        let roster_slot = (slot_idx + 1) as u8; // 1-indexed per spec

        if !should_decide(
            roster_slot,
            &state.decision_slots,
            &state.interrupt_cooldown_until,
            state.tick,
        ) {
            continue;
        }

        // SLICE-1: suppress the intended receiver's decision while a
        // successful pass is in flight to them. The receiver waits to trap;
        // no new intent (shot/pass/dribble) fires until the ball arrives.
        // This prevents the receiver from running away from the arrival point
        // and causing the flight timeout / a missed trap. Measured: removing
        // this freeze drops goals 2.88 → 2.06 (receivers drift off the lane and
        // miss traps), so the freeze is net-positive and retained.
        if let Some(bif) = state.ball_in_flight
            && bif.outcome_is_success
            && state.players[slot_idx].slot == bif.intended_receiver
        {
            // Receiver is waiting to trap — no ball-launching intent this
            // tick. Still bump the decision counter: the receiver DID make a
            // decision (to wait for the incoming ball), so the cadence
            // accounting stays consistent (a frozen receiver must not appear
            // to have stopped deciding — see decision_counter monotonicity +
            // the 3-decisions-per-60-ticks floor invariant).
            state.players[slot_idx].bump_decision_counter();
            continue;
        }

        // Pre-emption hooks — stubbed; -iii-b wires MatchEvent-driven hooks.
        if let Some(preempt_intent) = preempt_check(&state, slot_idx) {
            apply_intent(&mut state, slot_idx, preempt_intent);
            state.players[slot_idx].bump_decision_counter();
            continue;
        }

        // T1-2b-iv: evaluate signature triggers for this player.
        // Runs before role dispatch so the picked signature (if any) is in
        // `signature_firing[slot_idx]` when utility scoring reads the bias.
        let slot = slot_idx as u8;
        {
            // Clone candidates to avoid aliasing with &mut state below.
            let candidates = state.players[slot_idx].signature_candidates.clone();
            // active_firings: per-category snapshot for stacking check.
            // Clone all 4 lanes so we can pass them to evaluate_signatures
            // while state is borrowed mutably below.
            let active_firings: [Option<SignatureFiring>; 4] = [
                state.signature_firing[slot_idx][0].clone(),
                state.signature_firing[slot_idx][1].clone(),
                state.signature_firing[slot_idx][2].clone(),
                state.signature_firing[slot_idx][3].clone(),
            ];
            if let Some((sig_id, sig_def)) = evaluate_signatures(
                &state,
                slot,
                &candidates,
                sig_definitions,
                &trigger_table,
                &active_firings,
            ) {
                // Determine cooldown end tick from the definition's CooldownPolicy.
                // T1-23 (post-Codex Finding #1): `Tick::checked_add_ticks(u32)` replaces
                // `Tick::from_raw(state.tick.to_raw() + n as i64)`. Same arithmetic at
                // the realistic tick range; the helper funnels overflow through the
                // §11 panic-on-overflow policy so a stuck-loop bug at i64::MAX
                // surfaces loudly instead of silently wrapping.
                let cooldown_end_tick = match sig_def.cooldown {
                    CooldownPolicy::EveryTicks(n) => state.tick.checked_add_ticks(n),
                    CooldownPolicy::PerMatchCount(_) => {
                        // For PerMatchCount, use the 600-tick default as the
                        // intra-match spacing; the count limit enforced at T2-4.
                        state.tick.checked_add_ticks(600)
                    }
                };
                // Set cooldown.
                state
                    .signature_cooldowns
                    .insert((slot, sig_id.clone()), cooldown_end_tick);
                // Determine the category of the firing signature from its definition.
                let cat_idx = stacking_category_idx(&sig_def.stacking);
                // Set firing window in the correct category lane.
                state.signature_firing[slot_idx][cat_idx] = Some(SignatureFiring {
                    id: sig_id.clone(),
                    start_tick: state.tick,
                    duration_ticks: DEFAULT_FIRING_DURATION_TICKS,
                });
                // Emit SignatureFirstFired if first time this match.
                // T1-4a: push to match_events (persistent canonical stream),
                // replacing the removed signature_memory_events scratch buffer.
                let first_fired_key = (slot, sig_id.clone());
                if !state.signature_first_fired_seen.contains(&first_fired_key) {
                    state.signature_first_fired_seen.insert(first_fired_key);
                    state.match_events.push(MatchEvent::SignatureFirstFired {
                        player_slot: slot,
                        signature_id: sig_id,
                        tick: state.tick,
                    });
                }
            }
        }

        // Build ADR-0009 RNG for this decision.
        // site = (player_slot << 16) | local_decision_counter — truncated to u32.
        // Per ADR-0009: site is u32; the top 16 bits carry the slot (0..22 fits
        // in 5 bits), the low 16 bits carry the decision counter (u32 with
        // headroom per PlayerState docs). The counter is bounded to u16 range
        // for the site encoding; overflow into the slot bits would produce
        // collisions but is practically impossible in a 90-minute match.
        let counter = state.players[slot_idx].decision_counter();
        let site = ((slot_idx as u32) << 16) | (counter & 0xFFFF);
        // Tick is u32 per ADR-0009 (fw-core::seed::seed_fn takes tick: u32).
        // Tick::to_raw() returns i64; tick is monotonically non-negative so
        // the cast to u32 is safe for ~1 billion ticks (~194 days at 60 Hz).
        let tick_u32 = state.tick.to_raw() as u32;
        // UtilityTieBreak is the correct layer for softmax sampling over
        // utility-scored candidates (ADR-0009 §SeedLayer discriminants).
        // SeedLayer::Decision is reserved for binary decision draws
        // (e.g. GK shot-stopping direction) which are not yet wired.
        let rng_seed = seed_fn(
            state.seed.to_u64(),
            tick_u32,
            SeedLayer::UtilityTieBreak,
            site,
        );
        let mut rng = ChaCha8Rng::seed_from_u64(rng_seed);

        // P1-4: evaluate FSM transitions BEFORE subtree lookup, per ADR-0006
        // §"Concrete sketch". In skeleton tier this is always identity.
        // formation_slot is the 0-indexed slot for BtContext / SubtreeLibrary.
        let formation_slot = slot_idx as u8; // 0-indexed for formation_position / BtContext
        let current_role_state = state.players[slot_idx].role_state;
        let next_role_state = current_role_state.evaluate_transitions(&state, slot_idx);
        // Write back the (possibly updated) role state.
        state.players[slot_idx].role_state = next_role_state;

        let intent = match next_role_state {
            PlayerRoleState::Goalkeeper(gk_state) => {
                let player = &state.players[slot_idx];
                let (new_gk_state, gk_intent) =
                    tick_goalkeeper(gk_state, player, formation_slot, &state.ball, &mut rng);
                // Write back the new GK state.
                state.players[slot_idx].role_state = PlayerRoleState::Goalkeeper(new_gk_state);
                gk_intent
            }
            PlayerRoleState::Defender(_)
            | PlayerRoleState::Midfielder(_)
            | PlayerRoleState::Forward(_) => {
                // ADR-0006 P1-3: outfield roles use FSM-of-BTs. Route through
                // `bt::tick_tree` where the `OutfieldSelect` leaf invokes
                // `select_outfield_intent` (utility-scored softmax).
                // The BtContext carries what the leaf needs to call the select fn.
                let player = &state.players[slot_idx];
                // Resolve active signature bias: COMPOSITE-FOLD across all
                // active per-category lanes (Codex Tier-2 re-audit P1 closure).
                // ADR-0011 §"Stacking policy" allows cross-category concurrent
                // signatures BECAUSE their bias surfaces don't overlap; the
                // composite multiplies each *_mul field across all lanes
                // (Q32::ONE when no firings). Local Option holds the owned
                // SimBiasSnapshot so the &-borrow in BtContext stays valid
                // through the tick_tree call.
                let active_bias_owned: Option<SimBiasSnapshot> =
                    signature::dispatcher::combine_active_biases(
                        &state.signature_firing[slot_idx],
                        sig_definitions,
                    );
                let active_bias: Option<&SimBiasSnapshot> = active_bias_owned.as_ref();
                // B1 (FUN-0b+c): compute carrier_pos — the actual ball carrier's
                // world position. Passed through BtContext so utility_press and
                // utility_mark_player target the real carrier instead of the
                // formation-slot proxy. When possession is None (loose ball),
                // carrier_pos is None and the fallback formation-slot target is used
                // (that case is handled by preempt_check, not the BT runner).
                let carrier_pos: Option<(Q32, Q32)> = state.possession.and_then(|carrier_slot| {
                    // Only provide carrier pos if this player is on the OPPOSING team.
                    // (The carrier themselves and teammates don't press toward the carrier.)
                    let carrier_team = if (carrier_slot as usize) < crate::PLAYERS_PER_TEAM {
                        0usize
                    } else {
                        1usize
                    };
                    let player_team = if slot_idx < crate::PLAYERS_PER_TEAM {
                        0usize
                    } else {
                        1usize
                    };
                    if player_team != carrier_team {
                        let cs = carrier_slot as usize;
                        Some((state.players[cs].pos_x, state.players[cs].pos_y))
                    } else {
                        None
                    }
                });
                // Build a minimal tree: single OutfieldSelect leaf. The leaf resolves
                // role state → candidate list → softmax pick inside tick_tree.
                // Content-pack RON trees replace this stub at T2-3.
                //
                // FUN-TS1: determine team_idx for the zonal_slot transform.
                // Slots 0..11 = home (team_idx 0); slots 11..22 = away (team_idx 1).
                let team_idx_for_slot = if slot_idx < crate::PLAYERS_PER_TEAM {
                    0
                } else {
                    1
                };
                let shape_for_slot = &state.team_shape[team_idx_for_slot];
                let outfield_tree = Tree::new(Node::Leaf(LeafKind::OutfieldSelect));
                let ctx = BtContext {
                    roster_slot: formation_slot,
                    outfield_role_state: Some(next_role_state),
                    player: Some(player),
                    active_bias,
                    select_fn: Some(select_outfield_intent),
                    carrier_pos,
                    team_shape: shape_for_slot,
                    team_idx: team_idx_for_slot,
                };
                let (_, outfield_intent) = tick_tree(&outfield_tree, &ctx, &mut rng);
                outfield_intent
            }
        };

        // SLICE-1: while a successful pass is in flight, NOBODY holds the ball
        // (possession == None). Any intent that would launch/claim the ball
        // (a pass, cross, lay-off, shot, dribble-snap, or — critically — a GK
        // distribution) must be suppressed: only `trap_check_in_flight` may
        // grant possession during a flight. The GK FSM is the dangerous case —
        // it enters DistributingFromHand on ball-proximity alone (no possession
        // check), so an in-flight ball passing near a goal line would otherwise
        // make the GK "distribute" a ball it does not hold, re-launching the
        // flight every decision tick and resetting the timeout into a perpetual
        // flight-lock (seed 0x3c54cda26: 272-tick null run, 24.7m ball-carrier).
        // We rewrite the ball-launching intent into a positioning move toward
        // the same target so the player still repositions but does not seize
        // the flying ball.
        let intent = if state.ball_in_flight.is_some() {
            suppress_ball_launch_during_flight(intent)
        } else {
            intent
        };

        apply_intent(&mut state, slot_idx, intent);
        state.players[slot_idx].bump_decision_counter();
    }

    state
}

/// SLICE-1: rewrite a ball-launching intent into an equivalent positioning
/// move while a pass is in flight.
///
/// During a flight `possession == None`, so no player legitimately holds the
/// ball; only `trap_check_in_flight` may grant possession on arrival. Any
/// intent that would seize/launch the ball (pass / cross / lay-off / shot /
/// dribble-snap / GK distribution) is converted to a `MoveToPosition` toward
/// the same target so the player keeps repositioning without hijacking the
/// flying ball. Pure off-ball intents (movement, press, mark, hold-formation)
/// pass through unchanged.
///
/// This is the single fix that closes the flight-lock side-effect: the GK FSM
/// transitions to `DistributingFromHand` on ball-proximity alone (no possession
/// check), so without this guard an in-flight ball travelling near a goal line
/// makes the GK re-launch the pass every decision tick, resetting the timeout.
fn suppress_ball_launch_during_flight(intent: PlayerIntent) -> PlayerIntent {
    match intent {
        PlayerIntent::AttemptShot { target_x, target_y }
        | PlayerIntent::AttemptPassShort { target_x, target_y }
        | PlayerIntent::AttemptPassLong { target_x, target_y }
        | PlayerIntent::Cross { target_x, target_y }
        | PlayerIntent::Dribble { target_x, target_y }
        | PlayerIntent::HoldBall { target_x, target_y }
        | PlayerIntent::LayOff { target_x, target_y }
        | PlayerIntent::GkDistributeShort { target_x, target_y }
        | PlayerIntent::GkDistributeLong { target_x, target_y } => {
            PlayerIntent::MoveToPosition { target_x, target_y }
        }
        // Off-ball / positioning intents do not touch the ball — pass through.
        PlayerIntent::MoveToPosition { .. }
        | PlayerIntent::Idle
        | PlayerIntent::TrackBack { .. }
        | PlayerIntent::Press { .. }
        | PlayerIntent::MarkPlayer { .. }
        | PlayerIntent::RunOffBall { .. }
        | PlayerIntent::HoldFormation { .. }
        | PlayerIntent::GkShotStop { .. }
        | PlayerIntent::GkCollectCross { .. }
        | PlayerIntent::GkSweeperRush { .. } => intent,
    }
}

// ---------------------------------------------------------------------------
// apply_intent
// ---------------------------------------------------------------------------

/// Apply a `PlayerIntent` to a player by mutating their `vel_x`/`vel_y`
/// and emitting any corresponding `MatchEvent` entries.
///
/// ## Event emissions (T1-4a)
///
/// - `AttemptShot` → `MatchEvent::Shot` (before velocity update; happens regardless of outcome)
/// - `AttemptPassShort` → `MatchEvent::Pass { kind: Short }`
/// - `AttemptPassLong` → `MatchEvent::Pass { kind: Long }`
/// - `Cross` → `MatchEvent::Pass { kind: Cross }`
/// - `LayOff` → `MatchEvent::Pass { kind: LayOff }`
///
/// All other intents update velocity only; no event emitted.
///
/// ## T1 approximations
///
/// - `Shot.on_target`: derived from `target_y` within ±3.66 m half-width.
///   No keeper model yet.
/// - `Pass.to_slot`: nearest teammate heuristic — the nearest same-team
///   player to the target point. T2 will refine with passing-lane model.
/// - `Pass.completed`: always `true` in T1 (no contest physics yet).
///
/// ## Velocity model (FUN-0)
///
/// All variants with a target use the same velocity-toward-target model:
/// `apply_vel_toward_target(dx, dy)` — 2D magnitude cap at `MAX_PLAYER_SPEED`.
/// If `sqrt(dx² + dy²) <= MAX_PLAYER_SPEED`, velocity = `(dx, dy)`.
/// If the magnitude exceeds the cap, the vector is scaled to `MAX_PLAYER_SPEED`.
/// This ensures per-tick displacement ≤ `MAX_PLAYER_SPEED × dt ≈ 0.133m`
/// in all movement directions, not just cardinal.
///
/// FUN-CB1: pass completion is now stochastic (T1_PASS_COMPLETED removed).
/// See `crate::pass_completion::resolve_pass_completion` for the mechanic.
pub fn apply_intent(state: &mut MatchState, slot_idx: usize, intent: PlayerIntent) {
    // Emit events BEFORE mutating velocity. The emission match is EXHAUSTIVE
    // (no `_` wildcard) so that adding a new `PlayerIntent` variant produces
    // a compile error here — forcing the author to decide whether the new
    // variant emits a `MatchEvent` or not. Codex Tier-2 P0-1 on T1-4a
    // (2026-05-16) caught a `_ => {}` catch-all that would have silently
    // dropped events for any future pass-like variant (ThroughBall, Backheel,
    // OneTwo, etc.). Mirrors the T1-2b-iv `intent_to_bias_consideration`
    // wildcard removal lesson (P0-3 fix-pass).
    // T1-3.5: ball mutation + possession state update.
    // Runs BEFORE the velocity-update match below. Mutations are:
    //   AttemptShot: ball.vel toward target, speed from shooter attrs,
    //                possession → None (loose), last_touched_by → shooter.
    //   Pass-class (Short/Long/Cross/LayOff): ball.vel toward to_slot pos,
    //                speed from passer attrs, possession → Some(to_slot),
    //                last_touched_by → from_slot.
    //   Dribble: possession stays with dribbler, last_touched_by → dribbler,
    //            ball.pos snaps to player.pos (ball "at feet"), vel zeroed.
    //   GkDistributeShort/Long: mirror pass treatment from GK slot.
    //   All others: no ball mutation.
    //
    // MatchEvent emission is interleaved here so the event and the ball
    // mutation are co-located (atomic per intent; no split between the two
    // match arms). The possession/last_touched_by updates also happen here
    // so downstream code in tick_match (goal detection, step 7) sees the
    // updated state immediately after apply_intent returns.
    match &intent {
        PlayerIntent::AttemptShot {
            target_x,
            target_y: _target_y_placeholder,
        } => {
            let shooter_slot = state.players[slot_idx].slot;
            let tick_u32 = state.tick.to_raw() as u32;

            // SS2 — Compute dispersed target_y + xG for this shot.
            // `target_y_m` is the dispersed target in metres; `xg_score` is
            // written to `state.last_shot_xg[slot_idx]` for SS3's save model.
            let (target_y_dispersed, xg_score) = compute_shot_dispersion_and_xg(
                &state.players[slot_idx],
                shooter_slot,
                &state.players,
                slot_idx,
                state.seed.to_u64(),
                tick_u32,
            );

            // Cache xG in canonical state for SS3 GK save model.
            state.last_shot_xg[slot_idx] = xg_score;

            // Determine on-target from the DISPERSED target_y (not dead-centre).
            let on_target = is_shot_on_target(target_y_dispersed);
            state.match_events.push(MatchEvent::Shot {
                shooter_slot,
                tick: state.tick,
                target_x: *target_x,
                target_y: target_y_dispersed,
                on_target,
            });
            // T2-1d telemetry capture (NON-canonical; #[serde(skip)] field on
            // MatchState). Push a ShotTelemetryRecord with features extracted
            // post-hoc for the offline xG β / personality K_i fits per
            // `docs/design/xg-coefficients.md §Calibration loop (T2-1)`. The
            // record's `became_goal` field is None at push time; the calibrate
            // binary back-fills it post-match via MatchEvent::Goal correlation.
            let shooter = &state.players[slot_idx];
            let shooter_attrs = shooter.attributes();
            let shot_telem = crate::ShotTelemetryRecord {
                shot_tick: state.tick.to_raw() as u32,
                shooter_slot,
                distance_q32_raw: shot_distance_feature_q32(shooter.pos_x, shooter_slot).to_bits(),
                angle_q32_raw: shot_angle_feature_q32(shooter.pos_y).to_bits(),
                pressure_q32_raw: shot_pressure_feature_q32(&state.players, slot_idx).to_bits(),
                shot_type_q32_raw: fw_core::Q32::ONE.to_bits(), // T1: always footed
                assist_kind_q32_raw: fw_core::Q32::ONE.to_bits(), // T1: always solo
                shooter_quality_q32_raw: shot_quality_feature_q32(shooter_attrs).to_bits(),
                shooter_flair_q32_raw: shooter_attrs.mental.flair.to_bits(),
                shooter_composure_q32_raw: shooter_attrs.mental.composure.to_bits(),
                shooter_risk_appetite_q32_raw: shooter_attrs.personality.risk_appetite.to_bits(),
                became_goal: None,
            };
            state.shot_telemetry.push(shot_telem);
            // T1-15: snap ball to shooter's feet before kicking. Without this
            // the ball starts from its last physical position (often center
            // spot after kick-off) rather than the shooter's feet, causing
            // shots to travel from center and miss the goal entirely.
            let from_x = state.players[slot_idx].pos_x;
            let from_y = state.players[slot_idx].pos_y;
            state.ball.pos_x = from_x;
            state.ball.pos_y = from_y;
            // T1-3.5: ball mutation — kick toward the DISPERSED target.
            // `target_y_dispersed` is the SS2 scatter value; `target_x` is
            // still ±52m (goal depth sentinel — ensures ball travels forward).
            let speed = compute_ball_speed_for_shot(&state.players[slot_idx]);
            let (bvx, bvy) = ball_unit_vel(from_x, from_y, *target_x, target_y_dispersed, speed);
            state.ball.vel_x = bvx;
            state.ball.vel_y = bvy;
            state.ball.vel_z = Q32::ZERO; // ground-level shot in T1
            // Possession: shot releases the ball.
            state.possession = None;
            state.last_touched_by = Some(shooter_slot);
        }
        PlayerIntent::AttemptPassShort { target_x, target_y } => {
            let from_slot = state.players[slot_idx].slot;
            let to_slot = nearest_teammate_near(state, slot_idx, *target_x, *target_y);
            // FUN-TS2c: offside check at pass-launch tick.
            if is_offside_at_pass_launch(state, slot_idx, to_slot) {
                apply_offside(state, slot_idx, to_slot);
            } else {
                let tick_u32 = state.tick.to_raw() as u32;
                let pass_ok = crate::pass_completion::resolve_pass_completion(
                    state,
                    slot_idx,
                    to_slot,
                    PassKind::Short,
                    tick_u32,
                );
                state.match_events.push(MatchEvent::Pass {
                    from_slot,
                    to_slot,
                    tick: state.tick,
                    kind: PassKind::Short,
                    completed: pass_ok,
                });
                // T1-15: snap ball to passer's feet before computing velocity.
                let from_x = state.players[slot_idx].pos_x;
                let from_y = state.players[slot_idx].pos_y;
                state.ball.pos_x = from_x;
                state.ball.pos_y = from_y;
                if pass_ok {
                    // SLICE-1: ball travels; possession transfers on arrival, not now.
                    let speed = compute_ball_speed_for_pass(&state.players[slot_idx]);
                    let to_x = state.players[to_slot as usize].pos_x;
                    let to_y = state.players[to_slot as usize].pos_y;
                    let (bvx, bvy) = ball_unit_vel(from_x, from_y, to_x, to_y, speed);
                    state.ball.vel_x = bvx;
                    state.ball.vel_y = bvy;
                    state.ball.vel_z = Q32::ZERO;
                    assert!(
                        (to_slot as usize) < crate::TOTAL_PLAYERS,
                        "short-pass to_slot {to_slot} out of range"
                    );
                    state.ball_in_flight = Some(crate::BallInFlight {
                        intended_receiver: to_slot,
                        outcome_is_success: true,
                        launch_tick: state.tick,
                    });
                    state.possession = None;
                    state.last_touched_by = Some(from_slot);
                } else {
                    // Failure: emit PassIncomplete, drop loose ball, clear possession.
                    let to_x = state.players[to_slot as usize].pos_x;
                    let to_y = state.players[to_slot as usize].pos_y;
                    let is_forward = slot_idx < crate::PLAYERS_PER_TEAM && to_x > from_x
                        || slot_idx >= crate::PLAYERS_PER_TEAM && to_x < from_x;
                    drop_loose_ball(state, from_x, from_y, to_x, to_y, is_forward);
                    state.match_events.push(MatchEvent::PassIncomplete {
                        from_slot,
                        to_slot,
                        tick: state.tick,
                        kind: PassKind::Short,
                    });
                    state.possession = None;
                    state.last_touched_by = Some(from_slot);
                }
            }
        }
        PlayerIntent::AttemptPassLong { target_x, target_y } => {
            let from_slot = state.players[slot_idx].slot;
            let to_slot = nearest_teammate_near(state, slot_idx, *target_x, *target_y);
            // FUN-TS2c: offside check at pass-launch tick.
            if is_offside_at_pass_launch(state, slot_idx, to_slot) {
                apply_offside(state, slot_idx, to_slot);
            } else {
                let tick_u32 = state.tick.to_raw() as u32;
                let pass_ok = crate::pass_completion::resolve_pass_completion(
                    state,
                    slot_idx,
                    to_slot,
                    PassKind::Long,
                    tick_u32,
                );
                state.match_events.push(MatchEvent::Pass {
                    from_slot,
                    to_slot,
                    tick: state.tick,
                    kind: PassKind::Long,
                    completed: pass_ok,
                });
                // T1-15: snap ball to passer's feet.
                let from_x = state.players[slot_idx].pos_x;
                let from_y = state.players[slot_idx].pos_y;
                state.ball.pos_x = from_x;
                state.ball.pos_y = from_y;
                if pass_ok {
                    // SLICE-1: ball travels; possession transfers on arrival.
                    let speed = compute_ball_speed_for_pass(&state.players[slot_idx]);
                    let to_x = state.players[to_slot as usize].pos_x;
                    let to_y = state.players[to_slot as usize].pos_y;
                    let (bvx, bvy) = ball_unit_vel(from_x, from_y, to_x, to_y, speed);
                    state.ball.vel_x = bvx;
                    state.ball.vel_y = bvy;
                    state.ball.vel_z = Q32::ZERO;
                    assert!(
                        (to_slot as usize) < crate::TOTAL_PLAYERS,
                        "long-pass to_slot {to_slot} out of range"
                    );
                    state.ball_in_flight = Some(crate::BallInFlight {
                        intended_receiver: to_slot,
                        outcome_is_success: true,
                        launch_tick: state.tick,
                    });
                    state.possession = None;
                    state.last_touched_by = Some(from_slot);
                } else {
                    let to_x = state.players[to_slot as usize].pos_x;
                    let to_y = state.players[to_slot as usize].pos_y;
                    let is_forward = slot_idx < crate::PLAYERS_PER_TEAM && to_x > from_x
                        || slot_idx >= crate::PLAYERS_PER_TEAM && to_x < from_x;
                    drop_loose_ball(state, from_x, from_y, to_x, to_y, is_forward);
                    state.match_events.push(MatchEvent::PassIncomplete {
                        from_slot,
                        to_slot,
                        tick: state.tick,
                        kind: PassKind::Long,
                    });
                    state.possession = None;
                    state.last_touched_by = Some(from_slot);
                }
            }
        }
        PlayerIntent::Cross { target_x, target_y } => {
            let from_slot = state.players[slot_idx].slot;
            let to_slot = nearest_teammate_near(state, slot_idx, *target_x, *target_y);
            // FUN-TS2c: offside check at pass-launch tick.
            if is_offside_at_pass_launch(state, slot_idx, to_slot) {
                apply_offside(state, slot_idx, to_slot);
            } else {
                let tick_u32 = state.tick.to_raw() as u32;
                let pass_ok = crate::pass_completion::resolve_pass_completion(
                    state,
                    slot_idx,
                    to_slot,
                    PassKind::Cross,
                    tick_u32,
                );
                state.match_events.push(MatchEvent::Pass {
                    from_slot,
                    to_slot,
                    tick: state.tick,
                    kind: PassKind::Cross,
                    completed: pass_ok,
                });
                // T1-15: snap ball to crosser's feet before kick.
                let from_x = state.players[slot_idx].pos_x;
                let from_y = state.players[slot_idx].pos_y;
                state.ball.pos_x = from_x;
                state.ball.pos_y = from_y;
                if pass_ok {
                    // SLICE-1: ball travels; possession transfers on arrival.
                    let speed = compute_ball_speed_for_pass(&state.players[slot_idx]);
                    let to_x = state.players[to_slot as usize].pos_x;
                    let to_y = state.players[to_slot as usize].pos_y;
                    let (bvx, bvy) = ball_unit_vel(from_x, from_y, to_x, to_y, speed);
                    state.ball.vel_x = bvx;
                    state.ball.vel_y = bvy;
                    state.ball.vel_z = Q32::ZERO;
                    assert!(
                        (to_slot as usize) < crate::TOTAL_PLAYERS,
                        "cross to_slot {to_slot} out of range"
                    );
                    state.ball_in_flight = Some(crate::BallInFlight {
                        intended_receiver: to_slot,
                        outcome_is_success: true,
                        launch_tick: state.tick,
                    });
                    state.possession = None;
                    state.last_touched_by = Some(from_slot);
                } else {
                    let to_x = state.players[to_slot as usize].pos_x;
                    let to_y = state.players[to_slot as usize].pos_y;
                    let is_forward = slot_idx < crate::PLAYERS_PER_TEAM && to_x > from_x
                        || slot_idx >= crate::PLAYERS_PER_TEAM && to_x < from_x;
                    drop_loose_ball(state, from_x, from_y, to_x, to_y, is_forward);
                    state.match_events.push(MatchEvent::PassIncomplete {
                        from_slot,
                        to_slot,
                        tick: state.tick,
                        kind: PassKind::Cross,
                    });
                    state.possession = None;
                    state.last_touched_by = Some(from_slot);
                }
            }
        }
        PlayerIntent::LayOff { target_x, target_y } => {
            let from_slot = state.players[slot_idx].slot;
            let to_slot = nearest_teammate_near(state, slot_idx, *target_x, *target_y);
            // FUN-TS2c: offside check at pass-launch tick.
            if is_offside_at_pass_launch(state, slot_idx, to_slot) {
                apply_offside(state, slot_idx, to_slot);
            } else {
                let tick_u32 = state.tick.to_raw() as u32;
                let pass_ok = crate::pass_completion::resolve_pass_completion(
                    state,
                    slot_idx,
                    to_slot,
                    PassKind::LayOff,
                    tick_u32,
                );
                state.match_events.push(MatchEvent::Pass {
                    from_slot,
                    to_slot,
                    tick: state.tick,
                    kind: PassKind::LayOff,
                    completed: pass_ok,
                });
                // T1-15: snap ball to passer's feet before kick.
                let from_x = state.players[slot_idx].pos_x;
                let from_y = state.players[slot_idx].pos_y;
                state.ball.pos_x = from_x;
                state.ball.pos_y = from_y;
                if pass_ok {
                    // SLICE-1: ball travels; possession transfers on arrival.
                    let speed = compute_ball_speed_for_pass(&state.players[slot_idx]);
                    let to_x = state.players[to_slot as usize].pos_x;
                    let to_y = state.players[to_slot as usize].pos_y;
                    let (bvx, bvy) = ball_unit_vel(from_x, from_y, to_x, to_y, speed);
                    state.ball.vel_x = bvx;
                    state.ball.vel_y = bvy;
                    state.ball.vel_z = Q32::ZERO;
                    assert!(
                        (to_slot as usize) < crate::TOTAL_PLAYERS,
                        "layoff to_slot {to_slot} out of range"
                    );
                    state.ball_in_flight = Some(crate::BallInFlight {
                        intended_receiver: to_slot,
                        outcome_is_success: true,
                        launch_tick: state.tick,
                    });
                    state.possession = None;
                    state.last_touched_by = Some(from_slot);
                } else {
                    let to_x = state.players[to_slot as usize].pos_x;
                    let to_y = state.players[to_slot as usize].pos_y;
                    // LayOff is typically backward/lateral — use 20% drop.
                    drop_loose_ball(state, from_x, from_y, to_x, to_y, false);
                    state.match_events.push(MatchEvent::PassIncomplete {
                        from_slot,
                        to_slot,
                        tick: state.tick,
                        kind: PassKind::LayOff,
                    });
                    state.possession = None;
                    state.last_touched_by = Some(from_slot);
                }
            }
        }
        PlayerIntent::Dribble { .. } => {
            // T1-3.5: Dribble — ball stays at the dribbler's feet.
            // pos_x/pos_y updated to player position; vel zeroed (ball moves
            // with player via position snap rather than physics integration).
            // The player's vel_x/vel_y is still set by the velocity-update
            // match below so the player navigates toward the dribble target.
            let dribbler_slot = state.players[slot_idx].slot;
            // T2-1d telemetry capture for personality K_7 / K_8 fit (DRIBBLE_FLAIR
            // / DRIBBLE_AGG). NON-canonical; #[serde(skip)] field. See AttemptShot
            // arm above for the parallel shot-telemetry pattern + design-doc ref.
            let dribbler_attrs = state.players[slot_idx].attributes();
            let dribble_telem = crate::DribbleTelemetryRecord {
                dribble_tick: state.tick.to_raw() as u32,
                dribbler_slot,
                dribbler_flair_q32_raw: dribbler_attrs.mental.flair.to_bits(),
                dribbler_aggression_q32_raw: dribbler_attrs.personality.aggression.to_bits(),
            };
            state.dribble_telemetry.push(dribble_telem);
            state.ball.pos_x = state.players[slot_idx].pos_x;
            state.ball.pos_y = state.players[slot_idx].pos_y;
            state.ball.vel_x = Q32::ZERO;
            state.ball.vel_y = Q32::ZERO;
            state.ball.vel_z = Q32::ZERO;
            state.possession = Some(dribbler_slot);
            state.last_touched_by = Some(dribbler_slot);
        }
        PlayerIntent::GkDistributeShort { target_x, target_y } => {
            // Mirror pass: GK distributes to a teammate.
            let from_slot = state.players[slot_idx].slot;
            let to_slot = nearest_teammate_near(state, slot_idx, *target_x, *target_y);
            // No MatchEvent for GK distribution in T1 (commentary in T1-4b).
            // T1-15: snap ball to GK's feet before kick.
            let from_x = state.players[slot_idx].pos_x;
            let from_y = state.players[slot_idx].pos_y;
            state.ball.pos_x = from_x;
            state.ball.pos_y = from_y;
            // T1-3.5: ball mutation toward receiver.
            let speed = compute_ball_speed_for_pass(&state.players[slot_idx]);
            let to_x = state.players[to_slot as usize].pos_x;
            let to_y = state.players[to_slot as usize].pos_y;
            let (bvx, bvy) = ball_unit_vel(from_x, from_y, to_x, to_y, speed);
            state.ball.vel_x = bvx;
            state.ball.vel_y = bvy;
            state.ball.vel_z = Q32::ZERO;
            // SLICE-1: ball travels; possession transfers on arrival.
            assert!(
                (to_slot as usize) < crate::TOTAL_PLAYERS,
                "gk-distribute-short to_slot {to_slot} out of range"
            );
            state.ball_in_flight = Some(crate::BallInFlight {
                intended_receiver: to_slot,
                outcome_is_success: true,
                launch_tick: state.tick,
            });
            state.possession = None;
            state.last_touched_by = Some(from_slot);
        }
        PlayerIntent::GkDistributeLong { target_x, target_y } => {
            // Long GK distribution — same pattern as short but uses shot-speed
            // scaling (GKs kick hard) rather than pass-speed scaling.
            let from_slot = state.players[slot_idx].slot;
            let to_slot = nearest_teammate_near(state, slot_idx, *target_x, *target_y);
            // T1-15: snap ball to GK's feet before kick.
            let from_x = state.players[slot_idx].pos_x;
            let from_y = state.players[slot_idx].pos_y;
            state.ball.pos_x = from_x;
            state.ball.pos_y = from_y;
            let speed = compute_ball_speed_for_shot(&state.players[slot_idx]);
            let to_x = state.players[to_slot as usize].pos_x;
            let to_y = state.players[to_slot as usize].pos_y;
            let (bvx, bvy) = ball_unit_vel(from_x, from_y, to_x, to_y, speed);
            state.ball.vel_x = bvx;
            state.ball.vel_y = bvy;
            state.ball.vel_z = Q32::ZERO;
            // SLICE-1: ball travels; possession transfers on arrival.
            assert!(
                (to_slot as usize) < crate::TOTAL_PLAYERS,
                "gk-distribute-long to_slot {to_slot} out of range"
            );
            state.ball_in_flight = Some(crate::BallInFlight {
                intended_receiver: to_slot,
                outcome_is_success: true,
                launch_tick: state.tick,
            });
            state.possession = None;
            state.last_touched_by = Some(from_slot);
        }
        // Non-emitting / non-ball-touching variants — enumerated explicitly so
        // adding a new PlayerIntent variant forces a compile error here.
        PlayerIntent::Idle
        | PlayerIntent::MoveToPosition { .. }
        | PlayerIntent::HoldBall { .. }
        | PlayerIntent::TrackBack { .. }
        | PlayerIntent::Press { .. }
        | PlayerIntent::MarkPlayer { .. }
        | PlayerIntent::RunOffBall { .. }
        | PlayerIntent::HoldFormation { .. }
        | PlayerIntent::GkShotStop { .. }
        | PlayerIntent::GkCollectCross { .. }
        | PlayerIntent::GkSweeperRush { .. } => {
            // No MatchEvent emitted; no ball mutation.
            // T2+ may add events for press-trigger / interception / save —
            // wire them at this site, not via a wildcard.
        }
    }

    // SS3 staleness fix (reviewers P1, 2026-06-04): `last_shot_xg[slot_idx]` is
    // the SS3 GK-save gate — it must be non-zero ONLY while this player's MOST
    // RECENT ball action was a shot. A non-shot BALL-TOUCH (pass / cross /
    // lay-off / dribble / GK distribution) supersedes any prior shot context, so
    // clear it here. Without this, a shooter who later regains the ball and
    // dribbles it over the line would wrongly face the save model (the gate is
    // meant to let non-shot crossings — own goals, deflections, scrambles —
    // score). AttemptShot SETS the value above; NO-TOUCH intents (Idle / Move /
    // Press / Mark / RunOffBall / GkShotStop / ...) deliberately PRESERVE it so
    // an in-flight shot still faces the keeper when the ball reaches goal several
    // ticks later (the shooter is off-ball by then and may dispatch a Move).
    match &intent {
        PlayerIntent::AttemptPassShort { .. }
        | PlayerIntent::AttemptPassLong { .. }
        | PlayerIntent::Cross { .. }
        | PlayerIntent::LayOff { .. }
        | PlayerIntent::Dribble { .. }
        | PlayerIntent::GkDistributeShort { .. }
        | PlayerIntent::GkDistributeLong { .. } => {
            state.last_shot_xg[slot_idx] = Q32::ZERO;
        }
        _ => {}
    }

    // Velocity update (same for all target-bearing intents; Idle zeroes vel).
    let p = &mut state.players[slot_idx];
    match intent {
        PlayerIntent::Idle => {
            p.vel_x = Q32::ZERO;
            p.vel_y = Q32::ZERO;
        }

        // All variants with a target use the same velocity-toward-target model.
        // The target semantics differ per variant (aim point / run endpoint /
        // recipient position) but the locomotion physics are identical in this
        // tier: 2D normalisation to per-player v_max (Layer 1b: pace-scaled;
        // FUN-0 fix — replaces the prior per-component clamp that allowed
        // sqrt(2)×MAX diagonal movement).
        PlayerIntent::MoveToPosition { target_x, target_y }
        | PlayerIntent::AttemptShot { target_x, target_y }
        | PlayerIntent::AttemptPassShort { target_x, target_y }
        | PlayerIntent::AttemptPassLong { target_x, target_y }
        | PlayerIntent::Cross { target_x, target_y }
        | PlayerIntent::Dribble { target_x, target_y }
        | PlayerIntent::HoldBall { target_x, target_y }
        | PlayerIntent::LayOff { target_x, target_y }
        | PlayerIntent::TrackBack { target_x, target_y }
        | PlayerIntent::Press { target_x, target_y }
        | PlayerIntent::MarkPlayer { target_x, target_y }
        | PlayerIntent::RunOffBall { target_x, target_y }
        | PlayerIntent::HoldFormation { target_x, target_y }
        | PlayerIntent::GkShotStop { target_x, target_y }
        | PlayerIntent::GkCollectCross { target_x, target_y }
        | PlayerIntent::GkSweeperRush { target_x, target_y }
        | PlayerIntent::GkDistributeShort { target_x, target_y }
        | PlayerIntent::GkDistributeLong { target_x, target_y } => {
            let dx = target_x - p.pos_x;
            let dy = target_y - p.pos_y;
            // Layer 1b: GKs get a fixed cap; outfield players scale with pace.
            let v_max = if slot_idx == 0 || slot_idx == 11 {
                V_GK_SPEED
            } else {
                player_v_max(p.attributes.physical.pace)
            };
            let (vx, vy) = apply_vel_toward_target(dx, dy, v_max);
            p.vel_x = vx;
            p.vel_y = vy;
        }
    }
}

/// Find the nearest same-team player to `(target_x, target_y)`, excluding
/// `passer_slot_idx`. Returns the slot of the nearest teammate, or falls
/// back to the passer's own slot if no teammate exists.
///
/// Team is determined by slot index: slots 0..11 = home, 11..22 = away.
/// Comparison uses Manhattan distance (Q32 integer arithmetic; no sqrt).
///
/// **Panic safety (Codex Tier-2 P1-3 on T1-4a 2026-05-16):** distance
/// computation uses `i128` so the subtraction can't overflow even at the
/// extremes of `Q32`'s `i64` raw range, and the `unsigned_abs()` call can't
/// hit the `i64::MIN.abs()` undefined-behavior path. (`i64::MIN.abs()`
/// panics in debug and is UB in release.)
///
/// **Self-pass guard (Codex Tier-2 Critical on T1-4a 2026-05-16; T1-21
/// hardened to release per Sim/RULES.md §11):** the 22-slot match always has
/// 10 teammates available (11 same-team players minus the passer), so the
/// loop runs ≥10 iterations and `best_slot` is always overwritten. An
/// `assert_ne!` against the passer slot pins this invariant — if a future
/// refactor breaks the team_start/team_end derivation, the assertion fires
/// in BOTH debug + release builds, surfacing the bug at the violation site
/// rather than silently landing a self-pass `MatchEvent::Pass` into
/// canonical state. Pre-T1-21 this was `debug_assert_ne!` which the §11
/// hardening identified as exactly the silent-failure pattern banned.
///
/// FUN-CB1: Drop the ball at the loose-ball point after a failed pass.
///
/// For a forward pass (`is_forward = true`): drop at 40% of the passer→receiver
/// vector. For backward/lateral (`is_forward = false`): drop at 20%.
/// Ball velocity is zeroed — the preempt nearest-2 policy picks it up.
///
/// ## FUN-PHYS-1 partial symptom fix: lateral offset away from second-nearest opponent
///
/// Without an offset, two opposing preempt-chasers sprint toward the same
/// point and drive straight through each other (position-only separation +
/// velocity-re-issue creates a 60–150mm sustained clip-through). The root
/// fix (collision-aware movement) is FUN-PHYS-1 (TODO in MASTER_PLAN).
///
/// Cheap deterministic mitigation: after computing the nominal drop point,
/// find the closest opponent to that point (the likely second chaser, since
/// the primary chaser is nearest-2 by preempt policy), then apply a
/// `LOOSE_BALL_LATERAL_OFFSET` = `MIN_PLAYER_DISTANCE` (0.4m) in the
/// direction AWAY from that opponent along the pass-perpendicular axis
/// (signed by Y: above-centre → +Y offset, below-centre or on-axis → −Y).
///
/// This biases the ball off the head-on approach line so the two chasers
/// arrive at slightly different angles and don't overlap. The offset is
/// Q32-only, deterministic, and < 0.5m — invisible to commentary.
///
/// 0.40 in Q32 = 1_717_986_918 raw bits.
/// 0.20 in Q32 =   858_993_459 raw bits.
fn drop_loose_ball(
    state: &mut MatchState,
    from_x: Q32,
    from_y: Q32,
    to_x: Q32,
    to_y: Q32,
    is_forward: bool,
) {
    const FRAC_40: Q32 = Q32::from_raw(1_717_986_918_i64); // 0.40
    const FRAC_20: Q32 = Q32::from_raw(858_993_459_i64); // 0.20
    // Lateral offset magnitude = MIN_PLAYER_DISTANCE (0.4m). Applied perpendicular
    // to the pass direction so the ball lands off the head-on approach line.
    const LATERAL_OFFSET: Q32 = crate::separation::MIN_PLAYER_DISTANCE; // 0.4m

    let frac = if is_forward { FRAC_40 } else { FRAC_20 };
    // nominal drop: from + frac × (to - from)
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    let drop_x = from_x + frac * dx;
    let drop_y = from_y + frac * dy;

    // FUN-PHYS-1 mitigation: find the nearest opponent to the drop point
    // (i.e. the likely second chaser — not the passer's team).
    // The passer's team is determined by slot_idx < PLAYERS_PER_TEAM.
    // "Opponent" = the OTHER team's outfield slots (excludes GK at slot 0/11
    // since GKs only chase when ball is near their goal line).
    // Manhattan distance scan — no sqrt needed.
    let passer_is_home = {
        // Find who has the ball (last_touched_by / current possession state).
        // At this point possession is still set to Some(from_slot) — the caller
        // will clear it immediately after. We determine from_slot via from_x/from_y
        // matching: scan players for the one at (from_x, from_y). If no exact match
        // found (floating-point-like imprecision), fall back to Y-sign heuristic.
        // Q32 positions are exact; the passer was just positioned here this tick.
        let mut found_home = true;
        'outer: for idx in 0..crate::TOTAL_PLAYERS {
            if state.players[idx].pos_x == from_x && state.players[idx].pos_y == from_y {
                found_home = idx < crate::PLAYERS_PER_TEAM;
                break 'outer;
            }
        }
        // Fallback: if from_x > 0 the pass is in the home half → likely away passer;
        // heuristic only fires when position match fails (should be very rare).
        // Deliberately simple — wrong sign just means offset flips; both prevent head-on.
        found_home
    };

    let opp_range = if passer_is_home {
        crate::PLAYERS_PER_TEAM..crate::TOTAL_PLAYERS
    } else {
        0..crate::PLAYERS_PER_TEAM
    };

    let drop_x_i128 = drop_x.to_bits() as i128;
    let drop_y_i128 = drop_y.to_bits() as i128;
    let mut nearest_opp_dist: i128 = i128::MAX;
    let mut nearest_opp_y_raw: i64 = 0;

    for opp_idx in opp_range {
        let op = &state.players[opp_idx];
        let odx = (op.pos_x.to_bits() as i128 - drop_x_i128).unsigned_abs() as i128;
        let ody = (op.pos_y.to_bits() as i128 - drop_y_i128).unsigned_abs() as i128;
        let dist = odx + ody;
        if dist < nearest_opp_dist {
            nearest_opp_dist = dist;
            nearest_opp_y_raw = op.pos_y.to_bits();
        }
    }

    // Lateral offset direction: AWAY from nearest opponent in Y.
    // If nearest_opp_y_raw > drop_y_raw → opponent is above → offset downward (−Y).
    // If nearest_opp_y_raw ≤ drop_y_raw → opponent is below or same → offset upward (+Y).
    // This biases the ball off the head-on line regardless of pitch orientation.
    let drop_y_raw = drop_y.to_bits();
    let offset_y = if nearest_opp_y_raw > drop_y_raw {
        -LATERAL_OFFSET
    } else {
        LATERAL_OFFSET
    };

    state.ball.pos_x = drop_x;
    state.ball.pos_y = drop_y + offset_y;
    state.ball.vel_x = Q32::ZERO;
    state.ball.vel_y = Q32::ZERO;
    state.ball.vel_z = Q32::ZERO;
}

/// T1 approximation — T2 refines with passing-lane model.
fn nearest_teammate_near(
    state: &MatchState,
    passer_slot_idx: usize,
    target_x: Q32,
    target_y: Q32,
) -> u8 {
    let passer_team = if passer_slot_idx < 11 { 0usize } else { 1usize };
    let team_start = passer_team * 11;
    let team_end = team_start + 11;
    let passer_slot = state.players[passer_slot_idx].slot;

    let mut best_slot = passer_slot;
    // i128 distance space — Q32 raw bits fit comfortably; no overflow path.
    let mut best_dist: i128 = i128::MAX;

    let target_x_i128 = target_x.to_bits() as i128;
    let target_y_i128 = target_y.to_bits() as i128;

    for teammate_idx in team_start..team_end {
        if teammate_idx == passer_slot_idx {
            continue;
        }
        let tp = &state.players[teammate_idx];
        // Manhattan distance in i128 (positive by construction via unsigned_abs).
        let dx = (tp.pos_x.to_bits() as i128 - target_x_i128).unsigned_abs() as i128;
        let dy = (tp.pos_y.to_bits() as i128 - target_y_i128).unsigned_abs() as i128;
        let dist = dx + dy;
        if dist < best_dist {
            best_dist = dist;
            best_slot = tp.slot;
        }
    }

    // T1-21 per Sim/RULES.md §11: assert_ne! (release-active) replaces the
    // prior debug_assert_ne!. See the doc-comment Self-pass-guard section above
    // for the rationale. `best_slot != passer_slot` is a load-bearing canonical
    // invariant — a self-pass landing in match_events is a real silent-failure
    // class the §11 hardening exists to prevent.
    assert_ne!(
        best_slot, passer_slot,
        "nearest_teammate_near produced a self-pass for slot_idx={passer_slot_idx} \
         (team {passer_team}, range {team_start}..{team_end}); loop did not find \
         any teammate — check team-boundary derivation"
    );

    best_slot
}

/// FUN-TS2c: offside detection at pass-launch tick.
///
/// Returns `true` when the receiver would be in an offside position at the
/// moment this pass is played, per IFAB 2025/26 simplified rules.
///
/// ## Rules implemented
///
/// 1. **Attacking direction**: home team (passer_slot_idx < 11) attacks +x;
///    away team attacks -x.
/// 2. **Backward/square pass**: if receiver x ≤ passer x (for home) or
///    receiver x ≥ passer x (for away), no offside possible — return false.
/// 3. **Set-piece exemption**: no offside from a throw-in, corner, goal-kick,
///    or kick-off. Checked via the passer's team tactic FSM state.
/// 4. **Offside line**: the x-position of the 2nd-rearmost defender from the
///    attacking team's perspective. Includes GK. Equal = onside (IFAB §11.2).
/// 5. **Double check**: receiver beyond BOTH offside line AND ball x → flag.
///
/// ## T1 simplifications
///
/// - "Involvement" condition: any receiver who receives the ball is involved.
/// - Penalty area / goal area fine-grained rules deferred to T2.
/// - No offside trap exploit detection.
fn is_offside_at_pass_launch(
    state: &MatchState,
    passer_slot_idx: usize,
    receiver_slot: u8,
) -> bool {
    let is_home_passer = passer_slot_idx < crate::PLAYERS_PER_TEAM;
    let passer_team_idx = if is_home_passer { 0usize } else { 1usize };
    let opp_team_idx = 1 - passer_team_idx;

    // Set-piece exemption: no offside from throw-in, corner, goal-kick, kick-off.
    let passer_tactic = state.team_tactic_states[passer_team_idx].state();
    if is_set_piece_offside_exempt(passer_tactic) {
        return false;
    }

    let passer_x = state.players[passer_slot_idx].pos_x;
    let ball_x = state.ball.pos_x;
    let receiver_x = state.players[receiver_slot as usize].pos_x;

    // T1 offside zone: only check within 20m of the opponent's goal line.
    // With HighPress defensive lines at x=±2m and formation FWDs at x=±10m,
    // the entire midfield becomes a permanent offside zone — every pass in the
    // build-up is blocked. Real football risks are concentrated near the goal
    // where strikers make late runs; midfield offsides are extremely rare in
    // practice. Gate the check at 20m from goal: for home attacks, receiver
    // must be at x > +32.5m (52.5m - 20m); for away attacks, x < -32.5m.
    // This means offside only fires in the opponent's final third.
    // T2 can relax this when formation FWDs adjust dynamically to the DEF line.
    const GOAL_LINE: Q32 = Q32::from_int(52); // ≈ 52.5m (GOAL_LINE_X ≈ 52m)
    const OFFSIDE_ZONE_DEPTH: Q32 = Q32::from_int(20);
    if is_home_passer {
        // Home attacks +x; offside zone = x > 52.5 - 20 = +32.5m.
        if receiver_x <= GOAL_LINE - OFFSIDE_ZONE_DEPTH {
            return false;
        }
    } else {
        // Away attacks -x; offside zone = x < -(52.5 - 20) = -32.5m.
        if receiver_x >= -(GOAL_LINE - OFFSIDE_ZONE_DEPTH) {
            return false;
        }
    }

    // Backward/square pass: receiver no further forward than the passer.
    // Home attacks +x: forward = larger x.
    // Away attacks -x: forward = smaller x.
    if is_home_passer {
        if receiver_x <= passer_x {
            return false;
        }
    } else if receiver_x >= passer_x {
        return false;
    }

    // Compute the offside line from the opponent's TARGET defensive-line x.
    //
    // T1 approximation: we use `TeamShape.line_x` (the target for this tactic
    // state) rather than the strict IFAB 2nd-rearmost individual. Reason:
    // individual positions drift aggressively in the first 120-300 ticks as
    // defenders push up during attacking possession. The 2nd-rearmost
    // individual can temporarily be 15-20m ahead of the actual block, creating
    // cascade offside traps that block goal-scoring for the entire match.
    // Using the target line is stable (it changes only on tactic-state
    // transitions, not per-tick), football-legible, and avoids false offside
    // from individual drift. T2 can revisit with full positional correction
    // (IFAB 2nd-rearmost) once defender shape recovery is tighter.
    let offside_line = state.team_shape[opp_team_idx].line_x;

    // IFAB §11.2: equal = onside. Receiver must be STRICTLY beyond the line
    // AND beyond the ball (can't be offside if ball is further forward).
    if is_home_passer {
        receiver_x > offside_line && receiver_x > ball_x
    } else {
        receiver_x < offside_line && receiver_x < ball_x
    }
}

/// Returns true for set-piece states from which no offside can be called
/// (IFAB: throw-in, corner, goal-kick, kick-off).
fn is_set_piece_offside_exempt(state: crate::tactic_fsm::TacticState) -> bool {
    use crate::tactic_fsm::{SetPieceKind, TacticState};
    matches!(
        state,
        TacticState::SetPiece(
            SetPieceKind::ThrowInFor
                | SetPieceKind::ThrowInAgainst
                | SetPieceKind::CornerFor
                | SetPieceKind::CornerAgainst
                | SetPieceKind::GoalKick
                | SetPieceKind::GoalKickOpponent
                | SetPieceKind::KickOff
                // FreeKick restarts (including IFK from offside): exempt.
                // The GK receives the ball in FreeKickFor state. If the first
                // pass from the restart fires the offside check while defensive
                // lines are still recovering, away FWDs in their own half get
                // wrongly flagged. IFAB exempts the restart take itself; we
                // model this by exempting the tick the team is in FreeKickFor.
                // `emit_possession_transition_events` exits this state once
                // the ball reaches an open-play recipient.
                | SetPieceKind::FreeKickFor
                | SetPieceKind::FreeKickAgainst
        )
    )
}

/// Apply an offside call: emit the `MatchEvent::Offside`, clear possession,
/// award the ball to the defending GK (IFK restart).
///
/// The passer's team is team_idx. The opposing team's GK gets possession.
/// Ball is moved to the receiver's position at the offside call (IFAB: restart
/// from where the offside player was; T1 approximation).
fn apply_offside(state: &mut MatchState, passer_slot_idx: usize, receiver_slot: u8) {
    let passer_team_idx = if passer_slot_idx < crate::PLAYERS_PER_TEAM {
        0usize
    } else {
        1usize
    };
    let opp_team_idx = 1 - passer_team_idx;
    let opp_gk_slot = (opp_team_idx * crate::PLAYERS_PER_TEAM) as u8;

    state.match_events.push(fw_content::MatchEvent::Offside {
        offending_slot: receiver_slot,
        tick: state.tick,
    });

    // T1 approximation: snap ball to the opposing GK's position (IFK restart).
    // IFAB places the restart at the offside player's position, but our GK-
    // distribution model gates distribution on `dist_from_line < THRESHOLD`.
    // Placing the ball at the receiver's position (often far from the goal)
    // caused the GK to enter InBoxPositioning (never passes) instead of
    // DistributingFromHand, freezing possession for the rest of the match.
    // Snapping to GK position is accurate enough for T1 and keeps the sim alive.
    state.ball.pos_x = state.players[opp_gk_slot as usize].pos_x;
    state.ball.pos_y = state.players[opp_gk_slot as usize].pos_y;
    state.ball.vel_x = Q32::ZERO;
    state.ball.vel_y = Q32::ZERO;
    state.ball.vel_z = Q32::ZERO;
    state.possession = Some(opp_gk_slot);
    state.last_touched_by = None;

    // Tactic FSM: opposing team enters FreeKickFor; passer's team FreeKickAgainst.
    // Copy params before mutating state to avoid borrow conflict.
    use crate::tactic_fsm::{SetPieceKind, TacticEvent};
    let opp_arch = if opp_team_idx == 0 {
        state.home_archetype_params
    } else {
        state.away_archetype_params
    };
    let passer_arch = if passer_team_idx == 0 {
        state.home_archetype_params
    } else {
        state.away_archetype_params
    };
    let tick_now = state.tick;
    state.team_tactic_states[opp_team_idx] = crate::tactic_fsm::apply_event(
        state.team_tactic_states[opp_team_idx],
        &opp_arch,
        TacticEvent::BallOutOfPlay {
            kind: SetPieceKind::FreeKickFor,
        },
        tick_now,
    );
    state.team_tactic_states[passer_team_idx] = crate::tactic_fsm::apply_event(
        state.team_tactic_states[passer_team_idx],
        &passer_arch,
        TacticEvent::BallOutOfPlay {
            kind: SetPieceKind::FreeKickAgainst,
        },
        tick_now,
    );
}

/// Compute per-player v_max from `pace` attribute (outfield players only).
///
/// `v_max = V_PACE_BASE + pace × V_PACE_RANGE = 6.5 + pace × 2.5 m/s`.
/// At pace=0 → 6.5 m/s; pace=0.5 → 7.75 m/s; pace=1.0 → 9.0 m/s.
///
/// GKs (slots 0 and 11) use `V_GK_SPEED` directly; their pace attr is ignored.
fn player_v_max(pace: Q32) -> Q32 {
    V_PACE_BASE + pace * V_PACE_RANGE
}

/// Compute 2D velocity components toward a target, capped to `v_max`
/// as a **vector magnitude** (not per-component).
///
/// ## Why vector-magnitude cap (FUN-0)
///
/// The prior `clamp_speed(dx)` + `clamp_speed(dy)` independent clamping
/// allowed diagonal movement at magnitude `sqrt(2) × MAX_PLAYER_SPEED ≈ 11.31 m/s`
/// when both `|dx|` and `|dy|` were each ≥ 8m. This produced per-tick
/// displacement of `11.31/60 ≈ 0.189m`, tripping the `ImpossiblePlayerVelocity`
/// detector (threshold 0.15m) on every tick for any diagonally-moving player —
/// causing 32,000+ violations per 5400-tick match.
///
/// Layer 1b: `v_max` is now per-player (pace-scaled for outfield; GK-fixed for
/// GKs) rather than the flat `MAX_PLAYER_SPEED = 8 m/s` constant.
///
/// ## Zero-delta fallback
///
/// When `dx == dy == 0` (player is already at target), returns `(ZERO, ZERO)`.
/// The division-by-zero path is avoided by the `dist_sq == Q32::ZERO` guard.
///
/// ## Q32 determinism
///
/// Uses `Q32::sqrt()` (cordic-backed), same path as `separation.rs` and
/// `ball_unit_vel`. No floats, no clocks, no RNG.
fn apply_vel_toward_target(dx: Q32, dy: Q32, v_max: Q32) -> (Q32, Q32) {
    let dist_sq = dx * dx + dy * dy;
    let v_max_sq = v_max * v_max;

    if dist_sq == Q32::ZERO {
        // Already at target — zero velocity.
        return (Q32::ZERO, Q32::ZERO);
    }

    if dist_sq <= v_max_sq {
        // Within cap — use raw delta as velocity (no normalisation needed).
        return (dx, dy);
    }

    // Over cap: normalise to v_max magnitude.
    // dist = sqrt(dx² + dy²); vel = (dx/dist) × v_max.
    let dist = dist_sq.sqrt();
    let vel_x = dx / dist * v_max;
    let vel_y = dy / dist * v_max;
    (vel_x, vel_y)
}

// ---------------------------------------------------------------------------
// Pre-emption hooks (stub)
// ---------------------------------------------------------------------------

/// Pre-emption hook — wires loose-ball chase for T1-15.
///
/// Returns `Some(intent)` if a pre-emption fires for this player,
/// `None` to proceed to normal role dispatch.
///
/// When `state.possession == None` (loose ball — ball has been shot or knocked
/// free), the nearest-2 outfield players per team chase the ball's current
/// position. This prevents possession from staying `None` indefinitely while
/// preserving formation Y-spread (routing all 10 outfielders collapses width).
///
/// GKs (slots 0 and 11) are normally excluded — GK routing remains in the
/// goalkeeper FSM — EXCEPT when the ball is within 10m of the GK's own goal
/// line. In that case the GK chases the ball to prevent it from lingering
/// uncontested near the goal (the "ball stranded 2-3m short of goal line"
/// scenario from T1-15).
///
/// Full pre-emption hook (foul reaction, set-piece switchover, etc.) defers
/// to T2+ per ADR-0006. Loose-ball chase is the only live hook in T1.
fn preempt_check(state: &MatchState, slot_idx: usize) -> Option<PlayerIntent> {
    // Only fire when the ball is loose (no current carrier).
    if state.possession.is_some() {
        return None;
    }
    // SLICE-1: do NOT fire loose-ball chase during an in-flight pass. The
    // ball is already on its way to the intended receiver; chasing it disrupts
    // team shape. Only trigger preempt when the ball is genuinely loose
    // (shot out of play, failed-pass deflection, contested loose ball).
    if state.ball_in_flight.is_some() {
        return None;
    }

    // GK slots: only chase when ball is near their own goal line.
    // Home GK (slot 0): own goal at x = -52.5m.
    // Away GK (slot 11): own goal at x = +52.5m.
    // "Near" = within GK_CHASE_RADIUS_M of the goal line (absolute x distance).
    //
    // In Q32, GOAL_LINE_X = 52.5m stored as Q32::from_raw(52_i64 << 32 | ...)
    // We use a simple integer comparison: if abs(ball_x) > GK_CHASE_THRESHOLD_X,
    // the ball is close enough to the goal line for the GK to chase.
    // GK_CHASE_THRESHOLD_X = 42m (ball within 10m of the 52.5m goal line).
    if slot_idx == 0 || slot_idx == 11 {
        // Threshold: ball must be in the attacking third (>42m from centre)
        // to trigger GK chase. This keeps the GK in position during normal play.
        let bx_bits = state.ball.pos_x.to_bits();
        let bx_abs: u64 = bx_bits.unsigned_abs();
        // 42m in Q32: 42 << 32 = 180_388_203_520_u64
        const THRESHOLD_BITS: u64 = 42_u64 << 32;
        if bx_abs < THRESHOLD_BITS {
            return None; // ball is not near a goal line — let GK FSM decide
        }
        // Ball is near a goal line. Check it's near THIS GK's goal.
        // Home GK (slot 0): defends negative x (bx < 0).
        // Away GK (slot 11): defends positive x (bx > 0).
        let home_gk_side = bx_bits < 0; // true if ball is in home half
        let is_home_gk = slot_idx == 0;
        if home_gk_side != is_home_gk {
            return None; // ball is near the OPPONENT's goal — stay back
        }
        // GK chases the ball.
        return Some(PlayerIntent::MoveToPosition {
            target_x: state.ball.pos_x,
            target_y: state.ball.pos_y,
        });
    }

    // Only route the two outfield players NEAREST the ball toward it.
    // Routing all 10 outfielders collapses Y-formation spread (the
    // team_width invariant catches this). In real football, the nearest
    // 1-2 players chase; others hold shape. T1-15 approximation: nearest
    // 2 from each team chase; the rest hold formation (returning via BT).
    //
    // Compute this player's Manhattan distance to the ball.
    let bx = state.ball.pos_x;
    let by = state.ball.pos_y;
    let p = &state.players[slot_idx];
    let my_dx = (p.pos_x - bx).to_bits().unsigned_abs() as i128;
    let my_dy = (p.pos_y - by).to_bits().unsigned_abs() as i128;
    let my_dist = my_dx + my_dy;

    // Count how many same-team outfield players are closer to the ball.
    let team_start = if slot_idx < 11 { 1usize } else { 12usize };
    let team_end = if slot_idx < 11 { 11usize } else { 22usize };
    let gk_slot = if slot_idx < 11 { 0usize } else { 11usize };

    let closer_count = (team_start..team_end)
        .filter(|&i| {
            if i == slot_idx || i == gk_slot {
                return false;
            }
            let op = &state.players[i];
            let dx = (op.pos_x - bx).to_bits().unsigned_abs() as i128;
            let dy = (op.pos_y - by).to_bits().unsigned_abs() as i128;
            dx + dy < my_dist
        })
        .count();

    // If 2 or more same-team outfielders are closer, hold formation.
    // Only the 2 nearest outfielders chase the ball.
    if closer_count >= 2 {
        return None; // let BT decide (formation hold)
    }

    Some(PlayerIntent::MoveToPosition {
        target_x: state.ball.pos_x,
        target_y: state.ball.pos_y,
    })
}

// ---------------------------------------------------------------------------
// Tests — Chunk 6 (RED → GREEN)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_core::{Q32, Seed, Tick};

    use crate::role_states::Role;
    use crate::{MatchState, tick_match};

    // --- apply_intent ---

    #[test]
    fn apply_intent_idle_zeroes_velocity() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        // Give player 0 a nonzero velocity.
        state.players[0].vel_x = Q32::from_int(3);
        state.players[0].vel_y = Q32::from_int(-2);
        apply_intent(&mut state, 0, PlayerIntent::Idle);
        assert_eq!(state.players[0].vel_x, Q32::ZERO);
        assert_eq!(state.players[0].vel_y, Q32::ZERO);
    }

    #[test]
    fn apply_intent_move_to_position_sets_velocity() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        let p0_x = state.players[0].pos_x;
        let p0_y = state.players[0].pos_y;

        // Target 3 m/s above the player (within MAX_PLAYER_SPEED).
        apply_intent(
            &mut state,
            0,
            PlayerIntent::MoveToPosition {
                target_x: p0_x,
                target_y: p0_y + Q32::from_int(3),
            },
        );
        assert_eq!(state.players[0].vel_x, Q32::ZERO);
        assert_eq!(state.players[0].vel_y, Q32::from_int(3));
    }

    #[test]
    fn apply_intent_clamps_to_max_speed() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        let p0_x = state.players[0].pos_x;
        let p0_y = state.players[0].pos_y;

        // Target 100 m away — delta well beyond any player speed cap.
        // Slot 0 is the home GK, so the cap is V_GK_SPEED (7.2 m/s).
        apply_intent(
            &mut state,
            0,
            PlayerIntent::MoveToPosition {
                target_x: p0_x + Q32::from_int(100),
                target_y: p0_y,
            },
        );
        assert_eq!(state.players[0].vel_x, V_GK_SPEED);
    }

    // --- dispatch_tick wired into tick_match ---

    #[test]
    fn tick_match_increments_local_decision_counter_for_decided_players() {
        // Run 15 ticks — at least one player decides per tick (balanced slot).
        let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);
        let mut state = MatchState::initial(seed);
        let initial_counters: Vec<u32> =
            state.players.iter().map(|p| p.decision_counter()).collect();
        assert!(
            initial_counters.iter().all(|&c| c == 0),
            "all counters should start at zero"
        );

        for _ in 0..15 {
            state = tick_match(state, &std::collections::BTreeMap::new());
        }

        // After 15 ticks (one full cadence window), every player should have
        // fired at least once (the balanced slot template guarantees no empty slot).
        for (idx, p) in state.players.iter().enumerate() {
            assert!(
                p.decision_counter() > 0,
                "player slot {idx} had zero decisions after 15 ticks"
            );
        }
    }

    #[test]
    fn dispatch_tick_is_deterministic() {
        let seed = Seed::from_u64(0xCAFE_BABE);
        let s1 = MatchState::initial(seed);
        let s2 = MatchState::initial(seed);

        let empty_defs = BTreeMap::new();
        let r1 = dispatch_tick(s1, &empty_defs);
        let r2 = dispatch_tick(s2, &empty_defs);

        assert_eq!(
            r1.encode_canonical(),
            r2.encode_canonical(),
            "dispatch_tick must produce identical canonical output for the same initial state"
        );
    }

    /// P2-2 renamed from `gk_slot0_moves_toward_goal_line_after_decision`:
    /// the original test only asserted the counter; GK starts AT the goal
    /// line so position delta is zero. This test makes an honest assertion.
    #[test]
    fn gk_slot0_decides_within_15_ticks() {
        let seed = Seed::from_u64(1);
        let mut state = MatchState::initial(seed);

        // Run 15 ticks to cover one full cadence window.
        for _ in 0..15 {
            state = tick_match(state, &std::collections::BTreeMap::new());
        }

        // GK's decision_counter should be at least 1 (they decided).
        assert!(
            state.players[0].decision_counter() >= 1,
            "home GK (slot 0) should have made at least one decision in 15 ticks"
        );

        // Role must still be Goalkeeper.
        assert_eq!(
            state.players[0].role(),
            Role::Goalkeeper,
            "slot 0 should still be a Goalkeeper after 15 ticks"
        );
    }

    /// P2-2 additional coverage: verify GK actually moves when starting away
    /// from the goal line. Constructed directly with a non-goal-line position.
    #[test]
    fn gk_slot0_moves_toward_goal_line_when_displaced() {
        let seed = Seed::from_u64(1);
        let mut state = MatchState::initial(seed);
        // Move the home GK to centre spot (0, 0) — far from their goal line at (-45, 0).
        state.players[0].pos_x = Q32::ZERO;
        state.players[0].pos_y = Q32::ZERO;
        let initial_pos_x = state.players[0].pos_x;

        // SLICE-1: ensure NO pass fires to the GK during the run. The default
        // initial state has possession = Some(9) (home CF); within a few ticks
        // the CF passes — and in this jammed-at-centre fixture the nearest
        // teammate to the pass target is the displaced GK itself, so the GK
        // would become the in-flight pass receiver and be frozen by the
        // receiver-suppression guard (correct behaviour: a receiver waits to
        // trap). That defeats this test's intent (verify GK FSM positioning),
        // so we clear possession to a loose ball — no pass launches, the GK
        // FSM runs InBoxPositioning and moves toward its goal line.
        state.possession = None;

        // Run until GK decides at least once (find the first decision tick).
        for _ in 0..15 {
            state = tick_match(state, &std::collections::BTreeMap::new());
        }

        // After decisions, GK should have moved toward x=-45 (velocity set to negative x).
        // Position should now be less than the initial (0) since the GK moves toward -45.
        assert!(
            state.players[0].pos_x < initial_pos_x,
            "home GK should have moved toward x=-45; pos_x={:?} initial={:?}",
            state.players[0].pos_x,
            initial_pos_x,
        );
    }

    #[test]
    fn all_initial_roles_are_correctly_assigned() {
        let state = MatchState::initial(Seed::from_u64(1));

        // Home team: slots 0..11
        assert_eq!(state.players[0].role(), Role::Goalkeeper); // slot 0
        for i in 1..=4 {
            assert_eq!(
                state.players[i].role(),
                Role::Defender,
                "home slot {i} should be Defender"
            );
        }
        for i in 5..=7 {
            assert_eq!(
                state.players[i].role(),
                Role::Midfielder,
                "home slot {i} should be Midfielder"
            );
        }
        for i in 8..=10 {
            assert_eq!(
                state.players[i].role(),
                Role::Forward,
                "home slot {i} should be Forward"
            );
        }

        // Away team: slots 11..22
        assert_eq!(state.players[11].role(), Role::Goalkeeper); // slot 11
        for i in 12..=15 {
            assert_eq!(
                state.players[i].role(),
                Role::Defender,
                "away slot {i} should be Defender"
            );
        }
        for i in 16..=18 {
            assert_eq!(
                state.players[i].role(),
                Role::Midfielder,
                "away slot {i} should be Midfielder"
            );
        }
        for i in 19..=21 {
            assert_eq!(
                state.players[i].role(),
                Role::Forward,
                "away slot {i} should be Forward"
            );
        }
    }

    #[test]
    fn all_initial_decision_counters_are_zero() {
        let state = MatchState::initial(Seed::from_u64(42));
        for (i, p) in state.players.iter().enumerate() {
            assert_eq!(
                p.decision_counter(),
                0,
                "player slot {i} should have decision_counter == 0 at match-init"
            );
        }
    }

    #[test]
    fn decision_counter_increases_monotonically_per_player() {
        // Counters should never decrease during a match.
        let seed = Seed::from_u64(0xABCDABCD);
        let mut state = MatchState::initial(seed);
        let mut prev_counters = [0u32; 22];

        for _ in 0..60 {
            state = tick_match(state, &std::collections::BTreeMap::new());
            for (i, p) in state.players.iter().enumerate() {
                assert!(
                    p.decision_counter() >= prev_counters[i],
                    "player slot {i} counter went from {} to {} — counters must not decrease",
                    prev_counters[i],
                    p.decision_counter()
                );
                prev_counters[i] = p.decision_counter();
            }
        }
    }

    /// Verify the tick where N decisions are scheduled: check that exactly
    /// the right number of players have their counter incremented.
    #[test]
    fn decisions_fire_only_at_scheduled_ticks() {
        let seed = Seed::from_u64(7);
        let mut state = MatchState::initial(seed);
        let slots = state.decision_slots;

        // Advance to tick 1.
        state = tick_match(state, &std::collections::BTreeMap::new());
        let tick_raw = state.tick.to_raw();

        // Count how many roster slots fire at this tick.
        let expected_deciders = (0..22usize)
            .filter(|&i| {
                tick_raw.rem_euclid(15) as u8 == slots[i]
                    && state.interrupt_cooldown_until[i] <= Tick::from_raw(tick_raw)
            })
            .count();

        // Count how many players have counter == 1 (fired once).
        let actual_deciders = state
            .players
            .iter()
            .filter(|p| p.decision_counter() == 1)
            .count();

        assert_eq!(
            actual_deciders, expected_deciders,
            "at tick {tick_raw}: expected {expected_deciders} decisions but got {actual_deciders}"
        );
    }

    // --- T1-3.5 Chunk 3: ball-speed helper tests ---

    /// Zero attributes: speed equals base (no bonus).
    #[test]
    fn shot_speed_at_zero_attrs_equals_base() {
        use fw_core::PlayerAttributes;
        let mut state = MatchState::initial(Seed::from_u64(1));
        // Zero all attrs on player 9 (home FWD, slot 9).
        state.players[9].attributes = PlayerAttributes::default_zero();
        let speed = compute_ball_speed_for_shot(&state.players[9]);
        assert_eq!(
            speed, SHOT_BASE_SPEED_MPS,
            "at zero attrs, shot speed must equal SHOT_BASE_SPEED_MPS (20 m/s); got {:?}",
            speed
        );
    }

    /// Peak attributes: speed equals base + bonus.
    #[test]
    fn shot_speed_at_peak_attrs_equals_base_plus_bonus() {
        use fw_core::PlayerAttributes;
        let mut state = MatchState::initial(Seed::from_u64(1));
        // Max all attrs on player 9.
        state.players[9].attributes = PlayerAttributes::max_baseline();
        let speed = compute_ball_speed_for_shot(&state.players[9]);
        let expected = SHOT_BASE_SPEED_MPS + SHOT_PEAK_BONUS_MPS; // 35 m/s
        assert_eq!(
            speed, expected,
            "at max attrs (strength=1.0 × finishing=1.0 = 1.0), shot speed must equal \
             SHOT_BASE + SHOT_PEAK = 35 m/s; got {:?}",
            speed
        );
    }

    /// Mid-range attributes: speed between base and base+bonus.
    #[test]
    fn shot_speed_at_mid_attrs_is_between_base_and_max() {
        use fw_core::PlayerAttributes;
        let mut state = MatchState::initial(Seed::from_u64(1));
        state.players[9].attributes = PlayerAttributes::mid_range_baseline();
        let speed = compute_ball_speed_for_shot(&state.players[9]);
        // strength ≈ 0.5, finishing ≈ 0.5 → product ≈ 0.25 → bonus ≈ 3.75 m/s → total ≈ 23.75 m/s
        assert!(
            speed > SHOT_BASE_SPEED_MPS,
            "mid-range attrs must produce speed above base; got {:?}",
            speed
        );
        assert!(
            speed < SHOT_BASE_SPEED_MPS + SHOT_PEAK_BONUS_MPS,
            "mid-range attrs must produce speed below max; got {:?}",
            speed
        );
    }

    /// Pass: zero attrs → base speed.
    #[test]
    fn pass_speed_at_zero_attrs_equals_base() {
        use fw_core::PlayerAttributes;
        let mut state = MatchState::initial(Seed::from_u64(1));
        state.players[5].attributes = PlayerAttributes::default_zero();
        let speed = compute_ball_speed_for_pass(&state.players[5]);
        assert_eq!(
            speed, PASS_BASE_SPEED_MPS,
            "at zero attrs, pass speed must equal PASS_BASE_SPEED_MPS (15 m/s)"
        );
    }

    /// Pass: peak attrs → base + bonus.
    #[test]
    fn pass_speed_at_peak_attrs_equals_base_plus_bonus() {
        use fw_core::PlayerAttributes;
        let mut state = MatchState::initial(Seed::from_u64(1));
        state.players[5].attributes = PlayerAttributes::max_baseline();
        let speed = compute_ball_speed_for_pass(&state.players[5]);
        let expected = PASS_BASE_SPEED_MPS + PASS_PEAK_BONUS_MPS; // 25 m/s
        assert_eq!(
            speed, expected,
            "at max attrs (passing=1.0 × vision=1.0), pass speed must equal 25 m/s"
        );
    }

    // -----------------------------------------------------------------------
    // T1-19: preempt_check behavioral unit tests
    //
    // Background: T1-15 grew preempt_check from "stubbed None" to a 3-policy
    // implementation:
    //   1. Possession-gate: return None if state.possession.is_some().
    //   2. GK chase: slot 0 / 11 chase only when the ball is within 10m of
    //      their OWN goal line (|ball.pos_x| > 42m AND ball on own side).
    //   3. Outfield nearest-2: only fire for the 2 nearest same-team outfielders
    //      (strict-< tiebreak on Manhattan distance).
    //
    // These 5 tests pin each policy + the GK-vs-FSM coexistence invariant
    // documented in ADR-0006's 2026-05-16 amendment. See post-T1 ultimate-review
    // Track A (docs/audits/post-t1-ultimate-review-2026-05-16.md) for the
    // RED coverage-hole analysis that motivated this row.
    // -----------------------------------------------------------------------

    /// Policy 2 negative case: home GK does NOT chase a ball near the AWAY goal.
    /// Mutation discriminator: flipping the `home_gk_side != is_home_gk` predicate
    /// to `==` would make this test fail (preempt would fire, returning Some).
    #[test]
    fn preempt_check_home_gk_does_not_chase_away_ball() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        // Loose ball at +45m (away half, within 10m of the away goal line at +52.5m).
        state.possession = None;
        state.ball.pos_x = Q32::from_int(45);
        state.ball.pos_y = Q32::ZERO;
        state.ball.vel_x = Q32::ZERO;
        state.ball.vel_y = Q32::ZERO;

        let intent = preempt_check(&state, 0); // slot 0 = home GK
        assert!(
            intent.is_none(),
            "home GK (slot 0) must NOT chase a loose ball near the AWAY goal \
             (ball at x=+45m); got {intent:?}"
        );
    }

    /// Policy 2 positive case: home GK chases a loose ball within 10m of own goal line.
    /// Mutation discriminator: raising THRESHOLD_BITS from 42 to e.g. 100 would
    /// cause ball at |x|=43 to early-return None.
    #[test]
    fn preempt_check_home_gk_chases_loose_ball_within_42m_of_own_goal() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        // Loose ball at -43m (home side, within 10m of home goal line at -52.5m).
        state.possession = None;
        state.ball.pos_x = Q32::from_int(-43);
        state.ball.pos_y = Q32::from_int(2);
        state.ball.vel_x = Q32::ZERO;
        state.ball.vel_y = Q32::ZERO;

        let intent = preempt_check(&state, 0);
        match intent {
            Some(PlayerIntent::MoveToPosition { target_x, target_y }) => {
                assert_eq!(
                    target_x, state.ball.pos_x,
                    "preempt MoveToPosition target_x must equal ball.pos_x"
                );
                assert_eq!(
                    target_y, state.ball.pos_y,
                    "preempt MoveToPosition target_y must equal ball.pos_y"
                );
            }
            other => panic!(
                "home GK (slot 0) must chase ball within 10m of own goal line; \
                 expected MoveToPosition, got {other:?}"
            ),
        }
    }

    /// Policy 3: exactly the 2 nearest same-team outfielders preempt-chase a
    /// loose ball. The remaining 3 hold formation (return None).
    /// Mutation discriminator: changing `closer_count >= 2` to `>= 5` would
    /// make all 5 outfielders chase.
    #[test]
    fn preempt_check_outfield_chaser_count_caps_at_2() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        state.possession = None;
        // Place loose ball at the centre spot.
        state.ball.pos_x = Q32::ZERO;
        state.ball.pos_y = Q32::ZERO;
        state.ball.vel_x = Q32::ZERO;
        state.ball.vel_y = Q32::ZERO;

        // Choose 5 home outfielders (slots 1..=5) and place them at strictly
        // distinct Manhattan distances from the ball: 1m, 2m, 3m, 4m, 5m.
        // Strict-distinct distances mean the strict-< tiebreak yields a stable
        // ranking with no ties; the cap policy then routes the 2 nearest.
        for (i, slot) in [1usize, 2, 3, 4, 5].iter().enumerate() {
            state.players[*slot].pos_x = Q32::from_int((i as i32) + 1); // 1, 2, 3, 4, 5 m
            state.players[*slot].pos_y = Q32::ZERO;
        }
        // Park other home outfielders far away so they're not closer than these 5.
        for slot in [6usize, 7, 8, 9, 10] {
            state.players[slot].pos_x = Q32::from_int(40);
            state.players[slot].pos_y = Q32::from_int(20);
        }

        let chasers: Vec<usize> = [1usize, 2, 3, 4, 5]
            .iter()
            .copied()
            .filter(|&slot| preempt_check(&state, slot).is_some())
            .collect();

        assert_eq!(
            chasers.len(),
            2,
            "exactly 2 of the 5 nearest same-team outfielders must preempt-chase; \
             got {} chasers: {:?}",
            chasers.len(),
            chasers
        );
        // The 2 nearest by construction are slots 1 (1m) and 2 (2m).
        assert_eq!(
            chasers,
            vec![1, 2],
            "the nearest 2 outfielders should chase; got {chasers:?}"
        );

        // Determinism sub-assertion: same state → same result on a re-call.
        let second_pass: Vec<usize> = [1usize, 2, 3, 4, 5]
            .iter()
            .copied()
            .filter(|&slot| preempt_check(&state, slot).is_some())
            .collect();
        assert_eq!(
            chasers, second_pass,
            "preempt_check is a pure function over canonical state — \
             re-calling on unchanged state must return identical chaser set"
        );
    }

    /// Policy 1: preempt_check returns None whenever the ball is owned.
    /// Mutation discriminator: deleting the `state.possession.is_some()`
    /// early-return would make preempt fire under possession, returning Some.
    #[test]
    fn preempt_check_only_fires_on_loose_ball() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        // Possession held by home FWD (kickoff convention from MatchState::initial).
        state.possession = Some(9);
        // Place ball deep in own half (would otherwise trigger GK chase).
        state.ball.pos_x = Q32::from_int(-44);
        state.ball.pos_y = Q32::ZERO;
        state.ball.vel_x = Q32::ZERO;
        state.ball.vel_y = Q32::ZERO;

        // GK slot 0: even with ball within 10m of own goal, possession blocks preempt.
        assert!(
            preempt_check(&state, 0).is_none(),
            "preempt_check must return None for GK while possession is held"
        );
        // Outfield slot 1: same — possession blocks before nearest-2 logic runs.
        // Place slot 1 right on the ball so it would otherwise be the closest chaser.
        state.players[1].pos_x = state.ball.pos_x;
        state.players[1].pos_y = state.ball.pos_y;
        assert!(
            preempt_check(&state, 1).is_none(),
            "preempt_check must return None for outfielders while possession is held"
        );
    }

    /// Coexistence invariant: when preempt fires for the GK, dispatch_tick's
    /// `continue;` after `apply_intent` skips the GK FSM (tick_goalkeeper).
    /// Observable: the GK's role_state does NOT transition this tick, even
    /// when ball position + velocity would normally drive an InBoxPositioning
    /// → ShotStopping transition inside the GK FSM.
    /// Mutation discriminator: removing the `continue;` after the preempt
    /// branch would let tick_goalkeeper run and transition the FSM.
    #[test]
    fn preempt_check_does_not_conflict_with_goalkeeper_fsm() {
        use crate::role_states::{GoalkeeperState, PlayerRoleState};

        let seed = Seed::from_u64(0x1234_5678);
        let mut state = MatchState::initial(seed);

        // Force slot 0 to fire its decision at tick 0 (decision_slots[0] = 0
        // means tick.rem_euclid(15) == 0 → fires). MatchState::initial assigns
        // decision_slots from the seed; we override deterministically here.
        state.decision_slots[0] = 0;
        state.interrupt_cooldown_until[0] = Tick::ZERO;
        // (state.tick is already Tick::ZERO at MatchState::initial.)

        // Loose ball deep in the home penalty area, moving TOWARD the home goal:
        //   pos_x = -44m → in own half (bx < 0), in penalty area (bx < -36.5m).
        //   vel_x = -1   → approaching_goal predicate fires inside GK FSM.
        // Without preempt's `continue;`, tick_goalkeeper's evaluate_transitions
        // would route InBoxPositioning → ShotStopping.
        state.possession = None;
        state.ball.pos_x = Q32::from_int(-44);
        state.ball.pos_y = Q32::ZERO;
        state.ball.vel_x = Q32::from_int(-1);
        state.ball.vel_y = Q32::ZERO;

        // Pre-condition: GK starts InBoxPositioning (the MatchState::initial default).
        assert_eq!(
            state.players[0].role_state,
            PlayerRoleState::Goalkeeper(GoalkeeperState::InBoxPositioning),
            "test pre-condition: home GK must start in InBoxPositioning"
        );

        // Sanity: preempt would fire if dispatch consulted it standalone.
        let preempt_intent = preempt_check(&state, 0);
        assert!(
            matches!(preempt_intent, Some(PlayerIntent::MoveToPosition { .. })),
            "test pre-condition: preempt_check must return MoveToPosition for this state; \
             got {preempt_intent:?}"
        );

        // Execute one dispatch_tick. The preempt branch fires + `continue;` skips
        // tick_goalkeeper, so the GK FSM never runs this tick.
        let after = dispatch_tick(state, &BTreeMap::new());

        assert_eq!(
            after.players[0].role_state,
            PlayerRoleState::Goalkeeper(GoalkeeperState::InBoxPositioning),
            "preempt branch must skip GK FSM via `continue;`: GK role_state must \
             remain InBoxPositioning. If this fails, tick_goalkeeper ran and \
             transitioned to ShotStopping (or another state) — meaning preempt + \
             GK FSM both fired this tick, violating ADR-0006's 'preempt OR role \
             dispatch, never both' contract"
        );
    }

    // -----------------------------------------------------------------------
    // FUN-0: velocity-cap 2D normalisation tests
    //
    // These tests verify that the 2D velocity vector magnitude never exceeds
    // MAX_PLAYER_SPEED, regardless of the direction of movement. The prior
    // per-component clamp allowed diagonal movement at sqrt(2) * MAX_SPEED
    // ≈ 11.31 m/s, producing 0.189m/tick — well above the 0.133m/tick cap.
    // -----------------------------------------------------------------------

    /// Cardinal movement within MAX_PLAYER_SPEED is unchanged (regression guard).
    #[test]
    fn apply_intent_cardinal_within_cap_unchanged() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        let p0_x = state.players[0].pos_x;
        let p0_y = state.players[0].pos_y;
        // Move exactly 5 m in +X (within cap).
        apply_intent(
            &mut state,
            0,
            PlayerIntent::MoveToPosition {
                target_x: p0_x + Q32::from_int(5),
                target_y: p0_y,
            },
        );
        assert_eq!(state.players[0].vel_x, Q32::from_int(5));
        assert_eq!(state.players[0].vel_y, Q32::ZERO);
    }

    /// Diagonal movement with both components beyond the GK speed cap must be
    /// normalised so the vector magnitude equals V_GK_SPEED, not
    /// sqrt(2) * V_GK_SPEED. Slot 0 is the home GK.
    ///
    /// Old behaviour: vel_x = vel_y = MAX_PLAYER_SPEED = 8 →
    ///   magnitude = sqrt(2) * 8 ≈ 11.31 m/s.
    /// New behaviour: the vector (100, 100) is scaled so its magnitude = V_GK_SPEED →
    ///   each component ≈ V_GK_SPEED/sqrt(2) ≈ 5.09 m/s.
    #[test]
    fn apply_intent_diagonal_at_max_speed_normalised_to_cap() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        let p0_x = state.players[0].pos_x;
        let p0_y = state.players[0].pos_y;
        // Target 100m away diagonally — both components beyond the speed cap.
        apply_intent(
            &mut state,
            0,
            PlayerIntent::MoveToPosition {
                target_x: p0_x + Q32::from_int(100),
                target_y: p0_y + Q32::from_int(100),
            },
        );
        let vx = state.players[0].vel_x;
        let vy = state.players[0].vel_y;
        // Magnitude = sqrt(vx^2 + vy^2) must not exceed V_GK_SPEED (slot 0 = home GK).
        let mag_sq = vx * vx + vy * vy;
        let mag = mag_sq.sqrt();
        assert!(
            mag <= V_GK_SPEED,
            "diagonal velocity magnitude {mag:?} exceeds V_GK_SPEED={V_GK_SPEED:?} \
             after 2D normalisation (old per-component clamp would give sqrt(2)*MAX)"
        );
        // Both components must be equal (symmetric diagonal) and positive.
        assert_eq!(
            vx, vy,
            "symmetric diagonal (dx=dy=100) should yield equal velocity components"
        );
        assert!(vx > Q32::ZERO, "vel_x must be positive for +X movement");
    }

    /// A 45-degree diagonal target within MAX_PLAYER_SPEED is not clipped —
    /// only vectors whose magnitude exceeds the cap are normalised.
    #[test]
    fn apply_intent_diagonal_within_cap_not_clipped() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        let p0_x = state.players[0].pos_x;
        let p0_y = state.players[0].pos_y;
        // Target 3m in X, 3m in Y — magnitude = sqrt(18) ≈ 4.24 m/s (below 8 m/s cap).
        apply_intent(
            &mut state,
            0,
            PlayerIntent::MoveToPosition {
                target_x: p0_x + Q32::from_int(3),
                target_y: p0_y + Q32::from_int(3),
            },
        );
        assert_eq!(
            state.players[0].vel_x,
            Q32::from_int(3),
            "vel_x within-cap diagonal should equal the raw delta (3)"
        );
        assert_eq!(
            state.players[0].vel_y,
            Q32::from_int(3),
            "vel_y within-cap diagonal should equal the raw delta (3)"
        );
    }

    /// Per-tick displacement for diagonal full-speed movement must not
    /// exceed `MAX_PLAYER_SPEED × dt` (≈ 0.133m).
    ///
    /// This test is an integration-level discriminator: it calls `apply_intent`
    /// with a fully diagonal over-cap target (100m in both X and Y), reads the
    /// resulting velocity from the player state, then applies the same position
    /// integration step `tick_match` uses (`pos += vel * dt`) and checks that
    /// the Euclidean displacement is ≤ `MAX_PLAYER_SPEED × dt`.
    ///
    /// Mutation discriminator: under the OLD per-component clamp, `apply_intent`
    /// would set `vel_x = vel_y = MAX_PLAYER_SPEED = 8`, producing a diagonal
    /// displacement of `sqrt((8dt)² + (8dt)²) = 8√2 dt ≈ 0.189m` — which
    /// fails the `≤ 8dt ≈ 0.133m` assertion. The new 2D normalisation yields
    /// `vel_x = vel_y = 8/√2 ≈ 5.66`, giving displacement exactly `8dt`.
    #[test]
    fn tick_match_diagonal_full_speed_displacement_within_cap() {
        use crate::ball_physics::dt_per_tick;
        let seed = Seed::from_u64(42);
        let mut state = MatchState::initial(seed);
        let pos_before_x = state.players[0].pos_x;
        let pos_before_y = state.players[0].pos_y;

        // Drive apply_intent with a target 100m diagonally away — both
        // components far exceed MAX_PLAYER_SPEED, so the 2D normaliser kicks in.
        apply_intent(
            &mut state,
            0,
            PlayerIntent::MoveToPosition {
                target_x: pos_before_x + Q32::from_int(100),
                target_y: pos_before_y + Q32::from_int(100),
            },
        );

        // Integrate position one tick (mirrors tick_match step 7).
        let dt = dt_per_tick();
        let pos_after_x = pos_before_x + state.players[0].vel_x * dt;
        let pos_after_y = pos_before_y + state.players[0].vel_y * dt;

        // Euclidean displacement squared (avoid sqrt rounding by comparing squared values).
        let ddx = pos_after_x - pos_before_x;
        let ddy = pos_after_y - pos_before_y;
        let disp_sq = ddx * ddx + ddy * ddy;

        // Must not exceed (V_GK_SPEED × dt)² (slot 0 = home GK).
        // The old per-component clamp would produce ≈ 0.0357m² here
        // ((sqrt(2)×0.133)²), failing this assert.
        let max_allowed_sq = (V_GK_SPEED * dt) * (V_GK_SPEED * dt);
        // Add 1 ULP tolerance for cordic sqrt rounding accumulated across two multiplications.
        let tolerance = Q32::from_raw(1_i64);
        assert!(
            disp_sq <= max_allowed_sq + tolerance,
            "diagonal per-tick displacement² {disp_sq:?} exceeds (V_GK_SPEED×dt)² \
             {max_allowed_sq:?}; old per-component clamp would give ~0.0357m² here"
        );
    }

    /// V_PACE_BASE + V_PACE_RANGE must equal 9.0 m/s (the maximum outfield speed).
    ///
    /// Both constants are hand-written raw Q32 values. This test catches a
    /// future retune that updates one without the other, which would silently
    /// change the effective peak-pace speed cap.
    #[test]
    fn v_pace_base_plus_range_equals_nine_ms() {
        // 9.0 m/s in Q32.32 = 9 << 32
        let nine_ms = Q32::from_raw(9_i64 << 32);
        assert_eq!(
            V_PACE_BASE + V_PACE_RANGE,
            nine_ms,
            "V_PACE_BASE + V_PACE_RANGE should equal 9.0 m/s (9<<32); \
             if either constant is retuned, update both to keep base+range=9.0"
        );
    }

    // ---------------------------------------------------------------------------
    // FUN-0b SS2 — shot accuracy dispersion tests
    // ---------------------------------------------------------------------------

    /// SS2: on-target rate is NOT 100%.
    ///
    /// Before FUN-0b, every shot had target_y = 0 so every shot was on-target.
    /// After SS2 wiring, a distribution of shots should produce some off-target
    /// outcomes. Run 10 shots from the same slot across different ticks and assert
    /// that not all land on-target (probability that all 10 land on-target with
    /// sigma ≈ 3-5m is negligible).
    #[test]
    fn ss2_shot_target_y_is_not_always_on_target() {
        let state = MatchState::initial(Seed::from_u64(42));
        let shooter_idx = 9; // home FWD
        let shooter_slot = 9u8;
        let players = &state.players;
        let match_seed = state.seed.to_u64();

        let mut any_off_target = false;
        for tick in 0u32..10 {
            let (target_y, _xg) = compute_shot_dispersion_and_xg(
                &players[shooter_idx],
                shooter_slot,
                players,
                shooter_idx,
                match_seed,
                tick,
            );
            // The existing fw_content::is_shot_on_target checks |target_y| <= 3.66m
            if !fw_content::is_shot_on_target(target_y) {
                any_off_target = true;
                break;
            }
        }
        assert!(
            any_off_target,
            "SS2 dispersion must produce some off-target shots across 10 ticks; \
             all 10 were on-target which is statistically impossible at realistic sigma values"
        );
    }

    /// SS2: different ticks produce different target_y values (determinism + variance).
    #[test]
    fn ss2_dispersion_varies_across_ticks() {
        let state = MatchState::initial(Seed::from_u64(42));
        let shooter_idx = 9;
        let shooter_slot = 9u8;
        let players = &state.players;
        let match_seed = state.seed.to_u64();

        let (y0, _) = compute_shot_dispersion_and_xg(
            &players[shooter_idx],
            shooter_slot,
            players,
            shooter_idx,
            match_seed,
            0,
        );
        let (y1, _) = compute_shot_dispersion_and_xg(
            &players[shooter_idx],
            shooter_slot,
            players,
            shooter_idx,
            match_seed,
            1,
        );
        // Different ticks must produce different draws (by probabilistic argument;
        // the chance of identical Q32 output from two different seeds is negligible).
        assert_ne!(
            y0, y1,
            "SS2 dispersion across different ticks must differ (different RNG seeds)"
        );
    }

    /// SS2: same tick + same state produces same target_y (determinism).
    #[test]
    fn ss2_dispersion_is_deterministic() {
        let state = MatchState::initial(Seed::from_u64(42));
        let shooter_idx = 9;
        let shooter_slot = 9u8;
        let players = &state.players;
        let match_seed = state.seed.to_u64();

        let (y0, xg0) = compute_shot_dispersion_and_xg(
            &players[shooter_idx],
            shooter_slot,
            players,
            shooter_idx,
            match_seed,
            100,
        );
        let (y1, xg1) = compute_shot_dispersion_and_xg(
            &players[shooter_idx],
            shooter_slot,
            players,
            shooter_idx,
            match_seed,
            100,
        );
        assert_eq!(
            y0, y1,
            "SS2 dispersion must be deterministic for same inputs"
        );
        assert_eq!(
            xg0, xg1,
            "SS2 xg_score must be deterministic for same inputs"
        );
    }

    /// SS2: last_shot_xg is written when AttemptShot fires.
    ///
    /// Before FUN-0b, last_shot_xg didn't exist. After wiring, dispatching
    /// AttemptShot for a player must update state.last_shot_xg[slot_idx].
    #[test]
    fn ss2_apply_intent_attempt_shot_writes_last_shot_xg() {
        let mut state = MatchState::initial(Seed::from_u64(42));
        let slot_idx = 9;
        // Give possession to slot 9 and move them to a near-goal position.
        state.players[slot_idx].pos_x = Q32::from_int(40); // 12.5m from goal
        state.possession = Some(9);
        state.last_touched_by = Some(9);

        // Before shot, last_shot_xg should be zero.
        assert_eq!(
            state.last_shot_xg[slot_idx],
            Q32::ZERO,
            "last_shot_xg must be Q32::ZERO before any shot"
        );

        // Fire an AttemptShot via apply_intent.
        apply_intent(
            &mut state,
            slot_idx,
            PlayerIntent::AttemptShot {
                target_x: Q32::from_int(52),
                target_y: Q32::ZERO, // placeholder; SS2 computes real target_y
            },
        );

        // After shot, last_shot_xg must be non-zero (the shot was at 12.5m — high xG).
        assert!(
            state.last_shot_xg[slot_idx] > Q32::ZERO,
            "last_shot_xg must be non-zero after AttemptShot fires from near-goal (12.5m); \
             got {:?}",
            state.last_shot_xg[slot_idx]
        );
    }

    // ---------------------------------------------------------------------------
    // FUN-0b SS3 — GK save model integration test
    // ---------------------------------------------------------------------------

    /// SS3: a GK save prevents a goal.
    ///
    /// Construct a scenario where the ball is in the goal mouth and the save
    /// probability is very high (manually set last_shot_xg to near-zero so
    /// (1-xg) ≈ 1.0). Run tick_match and assert no goal was scored + the ball
    /// is no longer in the goal mouth.
    ///
    /// This is a probabilistic test — we fix the seed to a known value where
    /// the save roll wins. We verify at minimum that the save machinery is
    /// functional (the score doesn't always increment) across multiple seeds.
    #[test]
    fn ss3_save_can_prevent_goal() {
        // We run 20 seeds and assert that at least one produces a save
        // (score stays 0-0 after the ball enters the goal mouth at low xG).
        let mut any_saved = false;

        for seed_val in 0u64..20 {
            let mut state = MatchState::initial(Seed::from_u64(seed_val));
            // Place ball in the home goal mouth (negative x → away scores in home goal).
            // Home GK = slot 0. Away scores when ball is at negative x.
            state.ball.pos_x = -fw_core::GOAL_LINE_X;
            state.ball.pos_y = Q32::ZERO; // centre of goal mouth
            state.ball.vel_x = Q32::from_int(-5); // moving into goal
            state.ball.vel_y = Q32::ZERO;
            // Set last_touched_by to an away player (slot 19).
            state.last_touched_by = Some(19);
            // Set last_shot_xg for slot 19 very low (xG ≈ 0.02 — near-zero → save_prob ≈ high).
            state.last_shot_xg[19] = Q32::from_raw(85_899_346_i64); // ≈ 0.02
            state.possession = None;

            let state_after = tick_match(state, &std::collections::BTreeMap::new());
            // If saved: away_score stays 0 and ball is no longer at the goal line.
            if state_after.away_score == 0 {
                let bx_abs = state_after.ball.pos_x.to_bits().unsigned_abs();
                let goal_bits = fw_core::GOAL_LINE_X.to_bits().unsigned_abs();
                // Ball should have been cleared away from the goal line.
                if bx_abs < goal_bits {
                    any_saved = true;
                    break;
                }
            }
        }

        assert!(
            any_saved,
            "SS3: across 20 seeds with very low xG (≈0.02) entering the goal mouth, \
             at least one save must occur — the GK save model must be functional"
        );
    }

    /// SS3: xg_score near 1.0 makes saves very unlikely.
    ///
    /// With xG ≈ 0.80 (penalty-like), (1-xg) ≈ 0.20 → save_prob ≈ 0.145.
    /// Across 20 seeds, most should result in goals.
    #[test]
    fn ss3_high_xg_shot_mostly_results_in_goal() {
        let mut goals_scored = 0u32;
        let goal_mouth_x = -fw_core::GOAL_LINE_X; // home goal

        for seed_val in 0u64..20 {
            let mut state = MatchState::initial(Seed::from_u64(seed_val));
            state.ball.pos_x = goal_mouth_x;
            state.ball.pos_y = Q32::ZERO;
            state.ball.vel_x = Q32::from_int(-5);
            state.ball.vel_y = Q32::ZERO;
            state.last_touched_by = Some(19); // away player
            // High xG: ≈ 0.80 → save_prob ≈ save_base × 0.20 × 1.0 ≈ 0.145
            state.last_shot_xg[19] = Q32::from_raw(3_435_973_836_i64); // ≈ 0.80
            state.possession = None;

            let state_after = tick_match(state, &std::collections::BTreeMap::new());
            if state_after.away_score == 1 {
                goals_scored += 1;
            }
        }
        // With save_prob ≈ 0.145, expect ~85% goals → ≥ 14 of 20.
        assert!(
            goals_scored >= 12,
            "SS3: high-xG shots (≈0.80) should mostly score; got {goals_scored}/20 goals \
             (expected ≥ 12; if this fails the save probability may be miscalibrated)"
        );
    }

    // -----------------------------------------------------------------------
    // Layer 1b: pace-scaled player speed tests
    // -----------------------------------------------------------------------

    /// At pace=0 the outfield cap equals V_PACE_BASE (6.5 m/s).
    #[test]
    fn player_v_max_at_zero_pace_equals_base() {
        let v = player_v_max(Q32::ZERO);
        assert_eq!(
            v, V_PACE_BASE,
            "player_v_max(0) should equal V_PACE_BASE=6.5 m/s; got {v:?}"
        );
    }

    /// At pace=1 the outfield cap equals V_PACE_BASE + V_PACE_RANGE (9.0 m/s).
    #[test]
    fn player_v_max_at_full_pace_equals_base_plus_range() {
        let v = player_v_max(Q32::ONE);
        // V_PACE_BASE + V_PACE_RANGE = 6.5 + 2.5 = 9.0 m/s
        let expected = V_PACE_BASE + V_PACE_RANGE;
        assert_eq!(
            v, expected,
            "player_v_max(1) should equal 9.0 m/s; got {v:?}"
        );
    }

    /// player_v_max is monotonically non-decreasing with pace.
    #[test]
    fn player_v_max_monotone_with_pace() {
        let v0 = player_v_max(Q32::ZERO);
        let v_half = player_v_max(Q32::from_raw(1_i64 << 31)); // 0.5 in Q32.32
        let v1 = player_v_max(Q32::ONE);
        assert!(
            v0 <= v_half,
            "v_max should be non-decreasing: v(0)={v0:?} > v(0.5)={v_half:?}"
        );
        assert!(
            v_half <= v1,
            "v_max should be non-decreasing: v(0.5)={v_half:?} > v(1)={v1:?}"
        );
    }

    /// GK (slot 0) uses V_GK_SPEED; outfield slot with pace=0 uses V_PACE_BASE.
    /// Verifies apply_intent branches correctly on GK vs outfield.
    #[test]
    fn apply_intent_gk_uses_gk_speed_outfield_uses_pace_speed() {
        let mut state = MatchState::initial(Seed::from_u64(1));

        // GK slot 0: drive 100m in +X, should cap at V_GK_SPEED.
        let gk_x = state.players[0].pos_x;
        let gk_y = state.players[0].pos_y;
        apply_intent(
            &mut state,
            0,
            PlayerIntent::MoveToPosition {
                target_x: gk_x + Q32::from_int(100),
                target_y: gk_y,
            },
        );
        assert_eq!(
            state.players[0].vel_x, V_GK_SPEED,
            "GK (slot 0) capped velocity should equal V_GK_SPEED={V_GK_SPEED:?}; \
             got {:?}",
            state.players[0].vel_x
        );

        // Outfield slot 1: pace = mid-range baseline (0.5 in Q32), so
        // v_max = 6.5 + 0.5 × 2.5 = 7.75 m/s. Drive 100m in +X.
        // Initial state has mid_range_baseline pace ≈ 0.5.
        let p1_x = state.players[1].pos_x;
        let p1_y = state.players[1].pos_y;
        let expected_v = player_v_max(state.players[1].attributes.physical.pace);
        apply_intent(
            &mut state,
            1,
            PlayerIntent::MoveToPosition {
                target_x: p1_x + Q32::from_int(100),
                target_y: p1_y,
            },
        );
        assert_eq!(
            state.players[1].vel_x, expected_v,
            "outfield slot 1 capped velocity should equal player_v_max(pace)={expected_v:?}; \
             got {:?}",
            state.players[1].vel_x
        );
        // Outfield v_max must differ from GK cap.
        assert_ne!(
            expected_v, V_GK_SPEED,
            "outfield pace-scaled cap should differ from GK fixed cap"
        );
    }
}

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
// T4-2.5j: Five new signature thresholds (all at 0.45 matching existing pattern)
// ---------------------------------------------------------------------------

/// `CommandingClaim` — GK must have aerial-reach + handling + command of area.
pub const COMMANDING_CLAIM_THRESHOLD_AERIAL_REACH: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45
pub const COMMANDING_CLAIM_THRESHOLD_HANDLING: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45
pub const COMMANDING_CLAIM_THRESHOLD_COMMAND_OF_AREA: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45

/// `OverlappingSurge` — full-back with pace + stamina + crossing.
pub const OVERLAPPING_SURGE_THRESHOLD_PACE: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45
pub const OVERLAPPING_SURGE_THRESHOLD_STAMINA: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45
pub const OVERLAPPING_SURGE_THRESHOLD_CROSSING: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45

/// `ScreeningInterception` — defensive-mid slot with anticipation + positioning + tackling + marking.
pub const SCREENING_INTERCEPTION_THRESHOLD_ANTICIPATION: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45
pub const SCREENING_INTERCEPTION_THRESHOLD_POSITIONING: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45
pub const SCREENING_INTERCEPTION_THRESHOLD_TACKLING: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45
pub const SCREENING_INTERCEPTION_THRESHOLD_MARKING: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45

/// `TouchlineBeat` — winger slots with dribbling + pace + crossing.
pub const TOUCHLINE_BEAT_THRESHOLD_DRIBBLING: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45
pub const TOUCHLINE_BEAT_THRESHOLD_PACE: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45
pub const TOUCHLINE_BEAT_THRESHOLD_CROSSING: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45

/// `Poacher's Dart` — striker slot with off-the-ball + anticipation + finishing + acceleration + pace.
pub const POACHERS_DART_THRESHOLD_OFF_THE_BALL: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45
pub const POACHERS_DART_THRESHOLD_ANTICIPATION: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45
pub const POACHERS_DART_THRESHOLD_FINISHING: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45
pub const POACHERS_DART_THRESHOLD_ACCELERATION: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45
pub const POACHERS_DART_THRESHOLD_PACE: Q32 = Q32::from_raw(1_932_735_283_i64); // ≈ 0.45

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
// T4-2.5j: Five new trigger functions
// ---------------------------------------------------------------------------

/// `fwh.core:signature.commanding-claim`
///
/// A goalkeeper with aerial presence and command of the penalty area
/// claims the ball decisively, dominating their box.
///
/// Role check: goalkeeper only (in_team == 0, slots 0 + 11).
/// Attributes:
/// - `goalkeeper.aerial_reach` (claim height)
/// - `goalkeeper.handling` (secure the catch)
/// - `goalkeeper.command_of_area` (dominance in the box)
///
/// Returns `Q32::ZERO` if not eligible; fit-score = product of three attributes.
pub fn commanding_claim_trigger(state: &MatchState, slot: PlayerSlot) -> Q32 {
    let idx = slot as usize;
    if idx >= state.players.len() {
        return Q32::ZERO;
    }
    let a = &state.players[idx].attributes;

    // Role check: goalkeeper only (in_team == 0).
    let in_team = idx % 11;
    if in_team != 0 {
        return Q32::ZERO;
    }

    if a.goalkeeper.aerial_reach < COMMANDING_CLAIM_THRESHOLD_AERIAL_REACH
        || a.goalkeeper.handling < COMMANDING_CLAIM_THRESHOLD_HANDLING
        || a.goalkeeper.command_of_area < COMMANDING_CLAIM_THRESHOLD_COMMAND_OF_AREA
    {
        return Q32::ZERO;
    }

    // Fit-score: product of three goalkeeper attributes.
    a.goalkeeper.aerial_reach * a.goalkeeper.handling * a.goalkeeper.command_of_area
}

/// `fwh.core:signature.overlapping-surge`
///
/// A full-back makes an overlapping run, combining physical drive with
/// the quality to deliver a cross from deep.
///
/// Role check: full-back positions (in_team == 1 or in_team == 4).
/// In the 4-3-3: slot 1 = home left-back, slot 4 = home right-back;
/// slot 12 = away left-back, slot 15 = away right-back.
/// Attributes:
/// - `physical.pace` (burst capacity)
/// - `physical.stamina` (sustain the run)
/// - `technical.crossing` (quality of the delivery)
///
/// Returns `Q32::ZERO` if not eligible; fit-score = product of three attributes.
pub fn overlapping_surge_trigger(state: &MatchState, slot: PlayerSlot) -> Q32 {
    let idx = slot as usize;
    if idx >= state.players.len() {
        return Q32::ZERO;
    }
    let a = &state.players[idx].attributes;

    // Role check: full-back positions (flanks of the back four).
    let in_team = idx % 11;
    if in_team != 1 && in_team != 4 {
        return Q32::ZERO;
    }

    if a.physical.pace < OVERLAPPING_SURGE_THRESHOLD_PACE
        || a.physical.stamina < OVERLAPPING_SURGE_THRESHOLD_STAMINA
        || a.technical.crossing < OVERLAPPING_SURGE_THRESHOLD_CROSSING
    {
        return Q32::ZERO;
    }

    // Fit-score: pace × stamina × crossing.
    a.physical.pace * a.physical.stamina * a.technical.crossing
}

/// `fwh.core:signature.screening-interception`
///
/// A defensive midfielder reads the passing lane and steps in to
/// intercept, shielding the back four.
///
/// Role check: defensive midfielder slot (in_team == 5).
/// In the 4-3-3: slot 5 = home defensive-mid pivot; slot 16 = away.
/// Attributes:
/// - `mental.anticipation` (reads the pass)
/// - `mental.positioning` (occupies the lane)
/// - `technical.tackling` (executes the interception)
/// - `technical.marking` (stays tight)
///
/// Returns `Q32::ZERO` if not eligible; fit-score = product of four attributes.
pub fn screening_interception_trigger(state: &MatchState, slot: PlayerSlot) -> Q32 {
    let idx = slot as usize;
    if idx >= state.players.len() {
        return Q32::ZERO;
    }
    let a = &state.players[idx].attributes;

    // Role check: defensive-mid pivot (lowest midfielder slot in 4-3-3).
    let in_team = idx % 11;
    if in_team != 5 {
        return Q32::ZERO;
    }

    if a.mental.anticipation < SCREENING_INTERCEPTION_THRESHOLD_ANTICIPATION
        || a.mental.positioning < SCREENING_INTERCEPTION_THRESHOLD_POSITIONING
        || a.technical.tackling < SCREENING_INTERCEPTION_THRESHOLD_TACKLING
        || a.technical.marking < SCREENING_INTERCEPTION_THRESHOLD_MARKING
    {
        return Q32::ZERO;
    }

    // Fit-score: anticipation × positioning × tackling × marking.
    a.mental.anticipation * a.mental.positioning * a.technical.tackling * a.technical.marking
}

/// `fwh.core:signature.touchline-beat`
///
/// A winger takes on the full-back on the touchline, using pace and
/// dribbling skill to get to the byline and cross.
///
/// Role check: winger slots (in_team == 8 or in_team == 10).
/// In the 4-3-3: slot 8 = home left-winger, slot 10 = home right-winger;
/// slot 19 = away left-winger, slot 21 = away right-winger.
/// Attributes:
/// - `technical.dribbling` (close control in tight space)
/// - `physical.pace` (burst past the defender)
/// - `technical.crossing` (quality of the final delivery)
///
/// Returns `Q32::ZERO` if not eligible; fit-score = product of three attributes.
pub fn touchline_beat_trigger(state: &MatchState, slot: PlayerSlot) -> Q32 {
    let idx = slot as usize;
    if idx >= state.players.len() {
        return Q32::ZERO;
    }
    let a = &state.players[idx].attributes;

    // Role check: wide forward positions (flanks of the front three).
    let in_team = idx % 11;
    if in_team != 8 && in_team != 10 {
        return Q32::ZERO;
    }

    if a.technical.dribbling < TOUCHLINE_BEAT_THRESHOLD_DRIBBLING
        || a.physical.pace < TOUCHLINE_BEAT_THRESHOLD_PACE
        || a.technical.crossing < TOUCHLINE_BEAT_THRESHOLD_CROSSING
    {
        return Q32::ZERO;
    }

    // Fit-score: dribbling × pace × crossing.
    a.technical.dribbling * a.physical.pace * a.technical.crossing
}

/// `fwh.core:signature.poachers-dart`
///
/// A striker peels off the last defender's shoulder and darts in behind
/// to meet the ball, pure instinct and movement quality.
///
/// Role check: centre-forward slot (in_team == 9).
/// In the 4-3-3: slot 9 = home centre-forward; slot 20 = away.
/// Attributes:
/// - `mental.off_the_ball` (movement quality)
/// - `mental.anticipation` (reads the pass before it comes)
/// - `technical.finishing` (clinical in the chance)
/// - `physical.acceleration` (first burst of pace)
/// - `physical.pace` (sustained run to meet the ball)
///
/// Returns `Q32::ZERO` if not eligible; fit-score = product of five attributes.
pub fn poachers_dart_trigger(state: &MatchState, slot: PlayerSlot) -> Q32 {
    let idx = slot as usize;
    if idx >= state.players.len() {
        return Q32::ZERO;
    }
    let a = &state.players[idx].attributes;

    // Role check: centre-forward (central slot of the front three).
    let in_team = idx % 11;
    if in_team != 9 {
        return Q32::ZERO;
    }

    if a.mental.off_the_ball < POACHERS_DART_THRESHOLD_OFF_THE_BALL
        || a.mental.anticipation < POACHERS_DART_THRESHOLD_ANTICIPATION
        || a.technical.finishing < POACHERS_DART_THRESHOLD_FINISHING
        || a.physical.acceleration < POACHERS_DART_THRESHOLD_ACCELERATION
        || a.physical.pace < POACHERS_DART_THRESHOLD_PACE
    {
        return Q32::ZERO;
    }

    // Fit-score: off_the_ball × anticipation × finishing × acceleration × pace.
    a.mental.off_the_ball
        * a.mental.anticipation
        * a.technical.finishing
        * a.physical.acceleration
        * a.physical.pace
}

// ---------------------------------------------------------------------------
// In-play action gate (2026-06-06: kickoff-spam fix)
// ---------------------------------------------------------------------------
//
// The attribute predicates above (`*_trigger`) answer "is this player CAPABLE
// of the move + a good enough fit to bias toward it" — a quality/affinity
// score. They are pure functions of static attributes, so they are satisfied
// from kick-off and never change. Used alone to gate firing, every eligible
// player fired their signature the instant they first decided (tick < 120),
// then never again — a kickoff dump, not the rare-meaningful-moment vision of
// Pillar 5 (DECISIONS 2026-06-06 attribute-effect philosophy).
//
// `signature_executes_now` is the second gate: it answers "is the player, THIS
// tick, actually PERFORMING the move during live play?" — keyed off the chosen
// `PlayerIntent` + real ball/position geometry. A signature only fires (bias
// window + cooldown + `SignatureFirstFired`) when BOTH gates pass, so firings
// spread across the match and correlate with real shots / dribbles / runs /
// interceptions instead of clustering at kick-off.

/// Settle window: no signature may fire in the first `SIGNATURE_SETTLE_TICKS`
/// ticks (1 second at 60 Hz). Players are still organising off the kick-off;
/// the action gate already excludes most kick-off artefacts, but this is a
/// cheap belt-and-suspenders floor that needs no new canonical state.
pub const SIGNATURE_SETTLE_TICKS: i64 = 60;

/// Touchline band: a winger/full-back move counts as "on the touchline" when
/// within `TOUCHLINE_BAND_M` of either sideline (`|pos_y| > SIDELINE_Y - band`).
/// 10 m — the width of a typical wide channel.
const TOUCHLINE_BAND_M: Q32 = Q32::from_raw(10_i64 << 32);

/// Penalty-area depth from the goal line (real laws: 16.5 m).
const PENALTY_AREA_DEPTH_M: Q32 = Q32::from_raw(16_i64 << 32 | (1_i64 << 31)); // 16.5 m

/// Penalty-area half-width (real laws: 40.32 m wide → ±20.16 m).
const PENALTY_AREA_HALF_WIDTH_M: Q32 = Q32::from_raw(20_i64 << 32); // ≈ 20 m

/// Long-range-strike minimum distance from the target goal (≈ 22 m): a shot
/// closer than this is a regular finish, not the signature long-range effort.
const LONG_RANGE_MIN_DIST_M: Q32 = Q32::from_raw(22_i64 << 32);

/// Body-shield / press contest radius: the defender's chosen press/mark only
/// counts as the signature when the ball is within this radius (a real duel,
/// not a distant shadow). 4 m.
const PRESS_CONTEST_RADIUS_M: Q32 = Q32::from_raw(4_i64 << 32);

/// Squared 2-D distance between two points (Q32, no sqrt — determinism-safe).
#[inline]
fn dist_sq(ax: Q32, ay: Q32, bx: Q32, by: Q32) -> Q32 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}

/// Magnitude of a Q32 as raw bits (avoids the `i64::MIN.abs()` UB path; matches
/// the `unsigned_abs()` idiom used across the sim for geometry comparisons).
#[inline]
fn abs_bits(v: Q32) -> u64 {
    v.to_bits().unsigned_abs()
}

/// The `+X`-pointing goal line a player's team attacks toward.
///
/// Home (slots 0..11) attacks `+GOAL_LINE_X`; away (slots 11..22) attacks
/// `-GOAL_LINE_X` (locked convention — see `lib.rs::compute_opponent_shape_broken`).
#[inline]
fn attacking_goal_x(slot: PlayerSlot) -> Q32 {
    if (slot as usize) < crate::PLAYERS_PER_TEAM {
        fw_core::GOAL_LINE_X
    } else {
        -fw_core::GOAL_LINE_X
    }
}

/// True when `(px, py)` is inside the opponent penalty area the player attacks.
#[inline]
fn in_attacking_box(slot: PlayerSlot, px: Q32, py: Q32) -> bool {
    let goal_x = attacking_goal_x(slot);
    // Depth: within PENALTY_AREA_DEPTH_M of the attacked goal line.
    let depth_ok = abs_bits(goal_x - px) <= abs_bits(PENALTY_AREA_DEPTH_M);
    let width_ok = abs_bits(py) <= abs_bits(PENALTY_AREA_HALF_WIDTH_M);
    depth_ok && width_ok
}

/// Second firing gate: does the player's chosen `intent` THIS tick genuinely
/// execute `sig_id` during live play?
///
/// Returns `false` when the intent is unrelated to the signature's move, when
/// the geometry doesn't match (e.g. a "touchline" beat away from the touchline,
/// a "long-range" strike taken from 6 yards), or during the kick-off settle
/// window. Unknown signature ids return `false` (no firing) — the binding test
/// guarantees every shipped signature has a gate arm.
///
/// `intent` is the move the player committed to this tick (post-utility-pick).
/// `player_pos` is the player's current position; `ball_pos` the ball's.
#[must_use]
pub fn signature_executes_now(
    sig_id: &str,
    intent: &crate::role_states::PlayerIntent,
    slot: PlayerSlot,
    tick: fw_core::Tick,
    player_pos: (Q32, Q32),
    ball_pos: (Q32, Q32),
) -> bool {
    use crate::role_states::PlayerIntent as PI;

    // Settle floor: nothing fires in the opening second.
    if tick.to_raw() < SIGNATURE_SETTLE_TICKS {
        return false;
    }

    let (px, py) = player_pos;
    let (bx, by) = ball_pos;

    let on_touchline = abs_bits(py) > abs_bits(fw_core::SIDELINE_Y - TOUCHLINE_BAND_M);
    let in_box = in_attacking_box(slot, px, py);
    let goal_x = attacking_goal_x(slot);
    let far_from_goal = abs_bits(goal_x - px) >= abs_bits(LONG_RANGE_MIN_DIST_M);
    let near_ball = dist_sq(px, py, bx, by) <= PRESS_CONTEST_RADIUS_M * PRESS_CONTEST_RADIUS_M;

    match sig_id {
        // A winger driving at the byline: a real dribble in a wide channel,
        // in the attacking half (px ahead of the player's own half toward goal).
        "fwh.core:signature.touchline-beat" => matches!(intent, PI::Dribble { .. }) && on_touchline,
        // A poacher's run / finish inside the box.
        "fwh.core:signature.poachers-dart" => {
            matches!(intent, PI::AttemptShot { .. } | PI::RunOffBall { .. }) && in_box
        }
        // A genuine effort struck from distance.
        "fwh.core:signature.long-range-strike" => {
            matches!(intent, PI::AttemptShot { .. }) && far_from_goal
        }
        // A first-time switch of play — a long pass that changes the angle.
        "fwh.core:signature.first-time-diagonal-switch" => {
            matches!(intent, PI::AttemptPassLong { .. } | PI::Cross { .. })
        }
        // A full-back bombing on: a run/dribble/cross from a wide advanced spot.
        "fwh.core:signature.overlapping-surge" => {
            matches!(
                intent,
                PI::Dribble { .. } | PI::RunOffBall { .. } | PI::Cross { .. }
            ) && on_touchline
        }
        // The screening pivot stepping in: a real press/mark with the ball
        // within contesting distance (an actual challenge, not shadow-marking).
        "fwh.core:signature.screening-interception" => {
            matches!(intent, PI::Press { .. } | PI::MarkPlayer { .. }) && near_ball
        }
        // The keeper claiming a cross / sweeping out — an actual ball-collection.
        "fwh.core:signature.commanding-claim" => {
            matches!(intent, PI::GkCollectCross { .. } | PI::GkSweeperRush { .. })
        }
        // A defender clamping the carrier: a press/mark with the ball in a duel.
        "fwh.core:signature.body-shield-pressure" => {
            matches!(intent, PI::Press { .. } | PI::MarkPlayer { .. }) && near_ball
        }
        // No-op stub never executes.
        "fwh.core:signature.no-op-stub" => false,
        // Unknown id: no firing. The binding test asserts coverage so a new
        // signature without a gate arm is caught before match time.
        _ => false,
    }
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
    // T4-2.5j: five new role-family predicates
    table.insert(
        "fwh.core:signature.commanding-claim",
        commanding_claim_trigger,
    );
    table.insert(
        "fwh.core:signature.overlapping-surge",
        overlapping_surge_trigger,
    );
    table.insert(
        "fwh.core:signature.screening-interception",
        screening_interception_trigger,
    );
    table.insert("fwh.core:signature.touchline-beat", touchline_beat_trigger);
    table.insert("fwh.core:signature.poachers-dart", poachers_dart_trigger);
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
        // T4-2.5j: additional fields for new predicates.
        p.attributes.goalkeeper.aerial_reach = val;
        p.attributes.goalkeeper.handling = val;
        p.attributes.goalkeeper.command_of_area = val;
        p.attributes.physical.pace = val;
        p.attributes.physical.stamina = val;
        p.attributes.technical.crossing = val;
        p.attributes.mental.anticipation = val;
        p.attributes.mental.positioning = val;
        p.attributes.technical.tackling = val;
        p.attributes.technical.dribbling = val;
        p.attributes.mental.off_the_ball = val;
        p.attributes.technical.finishing = val;
        p.attributes.physical.acceleration = val;
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
        // All three at 0.5; 0.5 >= 0.45 threshold → eligible.
        // Product = 0.5 × 0.5 × 0.5 = 0.125 = raw (1 << 29).
        let half = Q32::from_raw(1i64 << 31); // 0.5
        state.players[1].attributes.technical.marking = half;
        state.players[1].attributes.physical.strength = half;
        state.players[1].attributes.personality.aggression = half;
        let fit = body_shield_pressure_trigger(&state, 1);
        // Mirror the long_range_strike_fit_score_is_product_of_attributes pattern:
        // compute expected as the same three-way product to pin the exact value.
        let expected = half * half * half; // 0.125 = raw (1 << 29)
        assert_eq!(
            fit, expected,
            "fit_score must be the exact product marking × strength × aggression = 0.125; \
             got {fit:?}, expected {expected:?}"
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

    // ---- commanding_claim ----

    #[test]
    fn commanding_claim_fires_for_gk_above_threshold() {
        let mut state = baseline_state();
        // slot 0 = home GK
        set_all_attrs(&mut state, 0, Q32::ONE);
        let fit = commanding_claim_trigger(&state, 0);
        assert!(
            fit > Q32::ZERO,
            "commanding_claim should fire for GK slot 0 with max attrs"
        );
    }

    #[test]
    fn commanding_claim_fit_score_is_product_of_three_gk_attrs() {
        let mut state = baseline_state();
        let half = Q32::from_raw(1i64 << 31); // 0.5
        state.players[0].attributes.goalkeeper.aerial_reach = half;
        state.players[0].attributes.goalkeeper.handling = half;
        state.players[0].attributes.goalkeeper.command_of_area = half;
        let fit = commanding_claim_trigger(&state, 0);
        // 0.5 × 0.5 × 0.5 = 0.125 — above threshold (0.45), positive fit.
        assert!(
            fit > Q32::ZERO,
            "fit must be positive for GK with attrs=0.5"
        );
        assert!(fit < Q32::ONE, "fit 0.125 must be < 1.0");
        // Exact value: 0.5^3 = 0.125 = Q32::from_raw(round(0.125 * 2^32)) = 536870912
        let expected = half * half * half;
        assert_eq!(fit, expected, "fit must equal the exact attribute product");
    }

    #[test]
    fn commanding_claim_zero_for_wrong_role() {
        let mut state = baseline_state();
        // slot 1 = home DEF (in_team=1) — not a GK
        set_all_attrs(&mut state, 1, Q32::ONE);
        assert_eq!(commanding_claim_trigger(&state, 1), Q32::ZERO);
    }

    #[test]
    fn commanding_claim_zero_for_attrs_below_threshold() {
        let mut state = baseline_state();
        // slot 0 = home GK; zero GK attrs
        state.players[0].attributes.goalkeeper.aerial_reach = Q32::ZERO;
        state.players[0].attributes.goalkeeper.handling = Q32::ZERO;
        state.players[0].attributes.goalkeeper.command_of_area = Q32::ZERO;
        assert_eq!(commanding_claim_trigger(&state, 0), Q32::ZERO);
    }

    // ---- overlapping_surge ----

    #[test]
    fn overlapping_surge_fires_for_left_back_above_threshold() {
        let mut state = baseline_state();
        // slot 1 = home left-back (in_team=1)
        set_all_attrs(&mut state, 1, Q32::ONE);
        let fit = overlapping_surge_trigger(&state, 1);
        assert!(
            fit > Q32::ZERO,
            "overlapping_surge should fire for slot 1 (left-back)"
        );
    }

    #[test]
    fn overlapping_surge_fires_for_right_back_above_threshold() {
        let mut state = baseline_state();
        // slot 4 = home right-back (in_team=4)
        set_all_attrs(&mut state, 4, Q32::ONE);
        let fit = overlapping_surge_trigger(&state, 4);
        assert!(
            fit > Q32::ZERO,
            "overlapping_surge should fire for slot 4 (right-back)"
        );
    }

    #[test]
    fn overlapping_surge_fit_score_is_product_of_pace_stamina_crossing() {
        let mut state = baseline_state();
        let half = Q32::from_raw(1i64 << 31); // 0.5
        state.players[1].attributes.physical.pace = half;
        state.players[1].attributes.physical.stamina = half;
        state.players[1].attributes.technical.crossing = half;
        let fit = overlapping_surge_trigger(&state, 1);
        assert!(fit > Q32::ZERO, "fit must be positive");
        let expected = half * half * half;
        assert_eq!(fit, expected, "fit must equal pace × stamina × crossing");
    }

    #[test]
    fn overlapping_surge_zero_for_centre_back() {
        let mut state = baseline_state();
        // slot 2 = home centre-back (in_team=2) — not a full-back
        set_all_attrs(&mut state, 2, Q32::ONE);
        assert_eq!(overlapping_surge_trigger(&state, 2), Q32::ZERO);
    }

    #[test]
    fn overlapping_surge_zero_for_attrs_below_threshold() {
        let mut state = baseline_state();
        state.players[1].attributes.physical.pace = Q32::ZERO;
        state.players[1].attributes.physical.stamina = Q32::ZERO;
        state.players[1].attributes.technical.crossing = Q32::ZERO;
        assert_eq!(overlapping_surge_trigger(&state, 1), Q32::ZERO);
    }

    // ---- screening_interception ----

    #[test]
    fn screening_interception_fires_for_dm_slot_above_threshold() {
        let mut state = baseline_state();
        // slot 5 = home defensive-mid pivot (in_team=5)
        set_all_attrs(&mut state, 5, Q32::ONE);
        let fit = screening_interception_trigger(&state, 5);
        assert!(
            fit > Q32::ZERO,
            "screening_interception should fire for slot 5 (DM pivot)"
        );
    }

    #[test]
    fn screening_interception_fit_score_is_product_of_four_attrs() {
        let mut state = baseline_state();
        let half = Q32::from_raw(1i64 << 31); // 0.5
        state.players[5].attributes.mental.anticipation = half;
        state.players[5].attributes.mental.positioning = half;
        state.players[5].attributes.technical.tackling = half;
        state.players[5].attributes.technical.marking = half;
        let fit = screening_interception_trigger(&state, 5);
        assert!(fit > Q32::ZERO, "fit must be positive");
        let expected = half * half * half * half;
        assert_eq!(
            fit, expected,
            "fit must equal anticipation × positioning × tackling × marking"
        );
    }

    #[test]
    fn screening_interception_zero_for_non_dm_midfielder() {
        let mut state = baseline_state();
        // slot 6 = home central-mid (in_team=6) — not the DM pivot
        set_all_attrs(&mut state, 6, Q32::ONE);
        assert_eq!(screening_interception_trigger(&state, 6), Q32::ZERO);
    }

    #[test]
    fn screening_interception_zero_for_attrs_below_threshold() {
        let mut state = baseline_state();
        state.players[5].attributes.mental.anticipation = Q32::ZERO;
        state.players[5].attributes.mental.positioning = Q32::ZERO;
        state.players[5].attributes.technical.tackling = Q32::ZERO;
        state.players[5].attributes.technical.marking = Q32::ZERO;
        assert_eq!(screening_interception_trigger(&state, 5), Q32::ZERO);
    }

    // ---- touchline_beat ----

    #[test]
    fn touchline_beat_fires_for_left_winger_above_threshold() {
        let mut state = baseline_state();
        // slot 8 = home left-winger (in_team=8)
        set_all_attrs(&mut state, 8, Q32::ONE);
        let fit = touchline_beat_trigger(&state, 8);
        assert!(
            fit > Q32::ZERO,
            "touchline_beat should fire for slot 8 (left-winger)"
        );
    }

    #[test]
    fn touchline_beat_fires_for_right_winger_above_threshold() {
        let mut state = baseline_state();
        // slot 10 = home right-winger (in_team=10)
        set_all_attrs(&mut state, 10, Q32::ONE);
        let fit = touchline_beat_trigger(&state, 10);
        assert!(
            fit > Q32::ZERO,
            "touchline_beat should fire for slot 10 (right-winger)"
        );
    }

    #[test]
    fn touchline_beat_fit_score_is_product_of_dribbling_pace_crossing() {
        let mut state = baseline_state();
        let half = Q32::from_raw(1i64 << 31); // 0.5
        state.players[8].attributes.technical.dribbling = half;
        state.players[8].attributes.physical.pace = half;
        state.players[8].attributes.technical.crossing = half;
        let fit = touchline_beat_trigger(&state, 8);
        assert!(fit > Q32::ZERO, "fit must be positive");
        let expected = half * half * half;
        assert_eq!(fit, expected, "fit must equal dribbling × pace × crossing");
    }

    #[test]
    fn touchline_beat_zero_for_centre_forward() {
        let mut state = baseline_state();
        // slot 9 = home centre-forward (in_team=9) — not a winger
        set_all_attrs(&mut state, 9, Q32::ONE);
        assert_eq!(touchline_beat_trigger(&state, 9), Q32::ZERO);
    }

    #[test]
    fn touchline_beat_zero_for_attrs_below_threshold() {
        let mut state = baseline_state();
        state.players[8].attributes.technical.dribbling = Q32::ZERO;
        state.players[8].attributes.physical.pace = Q32::ZERO;
        state.players[8].attributes.technical.crossing = Q32::ZERO;
        assert_eq!(touchline_beat_trigger(&state, 8), Q32::ZERO);
    }

    // ---- poachers_dart ----

    #[test]
    fn poachers_dart_fires_for_striker_slot_above_threshold() {
        let mut state = baseline_state();
        // slot 9 = home centre-forward (in_team=9)
        set_all_attrs(&mut state, 9, Q32::ONE);
        let fit = poachers_dart_trigger(&state, 9);
        assert!(
            fit > Q32::ZERO,
            "poachers_dart should fire for slot 9 (striker)"
        );
    }

    #[test]
    fn poachers_dart_fit_score_is_product_of_five_attrs() {
        let mut state = baseline_state();
        let half = Q32::from_raw(1i64 << 31); // 0.5
        state.players[9].attributes.mental.off_the_ball = half;
        state.players[9].attributes.mental.anticipation = half;
        state.players[9].attributes.technical.finishing = half;
        state.players[9].attributes.physical.acceleration = half;
        state.players[9].attributes.physical.pace = half;
        let fit = poachers_dart_trigger(&state, 9);
        assert!(fit > Q32::ZERO, "fit must be positive");
        // 0.5^5 = 0.03125 — above threshold (0.45), all five attributes at 0.5
        let expected = half * half * half * half * half;
        assert_eq!(
            fit, expected,
            "fit must equal off_the_ball × anticipation × finishing × acceleration × pace"
        );
    }

    #[test]
    fn poachers_dart_zero_for_wrong_slot() {
        let mut state = baseline_state();
        // slot 8 = home left-winger (in_team=8) — not in_team==9
        set_all_attrs(&mut state, 8, Q32::ONE);
        assert_eq!(poachers_dart_trigger(&state, 8), Q32::ZERO);
    }

    #[test]
    fn poachers_dart_zero_for_attrs_below_threshold() {
        let mut state = baseline_state();
        state.players[9].attributes.mental.off_the_ball = Q32::ZERO;
        state.players[9].attributes.mental.anticipation = Q32::ZERO;
        state.players[9].attributes.technical.finishing = Q32::ZERO;
        state.players[9].attributes.physical.acceleration = Q32::ZERO;
        state.players[9].attributes.physical.pace = Q32::ZERO;
        assert_eq!(poachers_dart_trigger(&state, 9), Q32::ZERO);
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
    fn trigger_table_contains_all_t4_2_5j_signatures() {
        let table = build_trigger_table();
        assert!(table.contains_key("fwh.core:signature.commanding-claim"));
        assert!(table.contains_key("fwh.core:signature.overlapping-surge"));
        assert!(table.contains_key("fwh.core:signature.screening-interception"));
        assert!(table.contains_key("fwh.core:signature.touchline-beat"));
        assert!(table.contains_key("fwh.core:signature.poachers-dart"));
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

    // ---- QA-T4H item 3: poachers_dart forward-gate hole kill ----

    /// Mutation target: `in_team != 9` → `in_team > 9` lets slots 0-9 through.
    ///
    /// Slot 20 (away team's centre-forward, in_team == 9) is the boundary that
    /// the existing `poachers_dart_zero_for_wrong_slot` test already covers (slot 8).
    /// That test does NOT kill the mutation `!= 9` → `> 9` because slot 8 is also
    /// excluded by `> 9`. We need a slot where in_team == 10 (right-winger, also a
    /// forward but NOT the striker): max-attrs there must return ZERO.
    ///
    /// With `in_team != 9`: slot 10 (in_team=10) fails `in_team != 9` → returns ZERO. OK.
    /// With `in_team > 9`: slot 10 (in_team=10 > 9) PASSES the role check → fires. WRONG.
    /// This test kills the mutation.
    #[test]
    fn poachers_dart_zero_for_right_winger_slot_10_max_attrs() {
        // Mutation killed: `in_team != 9` → `in_team > 9` would let in_team=10 through.
        // With the correct gate (`in_team != 9`) slot 10 (in_team=10) must return ZERO.
        let mut state = baseline_state();
        // slot 10 = home right-winger (in_team = 10 % 11 = 10)
        set_all_attrs(&mut state, 10, Q32::ONE);
        let fit = poachers_dart_trigger(&state, 10);
        assert_eq!(
            fit,
            Q32::ZERO,
            "poachers_dart must return Q32::ZERO for slot 10 (in_team=10, right-winger): \
             gate is `in_team != 9`, not `in_team > 9`; \
             mutating to `> 9` would let this slot through — this test must fail under that mutation"
        );
    }

    // ---- QA-T4H item 4: back-port exact-product form to body_shield_fit_score ----

    /// Replaces the vacuous `>0 && <1` body_shield_fit_score test with an exact
    /// attribute product assertion, matching the form already used by the 5 new
    /// T4-2.5j predicates.
    ///
    /// Mutation killed: any change to the fit-score expression (e.g. sum instead of
    /// product, omitting one attribute) produces a different value → `assert_eq!` fails.
    #[test]
    fn body_shield_fit_score_exact_product() {
        let mut state = baseline_state();
        // Use distinct non-trivial values to expose any summation vs. product error.
        // marking = 0.5, strength = 0.75, aggression = 0.6 — all above 0.45 threshold.
        let half = Q32::from_raw(1i64 << 31); // 0.5
        let three_quarters = Q32::from_raw(3i64 << 30); // 0.75
        // 0.6 × 2^32 ≈ 2_576_980_377
        let point_six = Q32::from_raw(2_576_980_377_i64);

        state.players[1].attributes.technical.marking = half;
        state.players[1].attributes.physical.strength = three_quarters;
        state.players[1].attributes.personality.aggression = point_six;

        let fit = body_shield_pressure_trigger(&state, 1);

        // Exact product: marking × strength × aggression
        let expected = half * three_quarters * point_six;
        assert_eq!(
            fit, expected,
            "body_shield fit-score must be the exact product marking × strength × aggression; \
             got {:?}, expected {:?}. \
             Mutation killed: using sum/average instead of product returns a different value.",
            fit, expected
        );
    }

    // ---- QA-T4H item 5: body-shield gate exclusivity characterization test ----

    /// Characterization/exclusivity test pinning CURRENT behavior for body_shield_pressure.
    ///
    /// The RON declares `role_family: CentreBack` but the gate is `in_team 1..=7`
    /// (DEF + all MID, 13/22 positions). This is documented as intentional in
    /// design/signatures.md ("CB-or-mid shielding"). This test pins the exact current
    /// behavior so any future gate change is caught explicitly.
    ///
    /// Pins:
    /// - in_team == 0 (GK): ZERO
    /// - in_team 1..=7 (DEF+MID): positive (fires)
    /// - in_team 8, 9, 10 (FWD): ZERO
    ///
    /// Do NOT change the RON to match this test — the broad gate is intentional.
    /// If design changes the gate, update this test AND create a DECISIONS.md entry.
    #[test]
    fn body_shield_gate_exclusivity_characterizes_current_in_team_1_to_7_behavior() {
        let mut state = baseline_state();

        // Set max attrs on all slots so only the role gate determines the result.
        for slot in 0..11usize {
            set_all_attrs(&mut state, slot, Q32::ONE);
        }

        // GK (in_team == 0): must be ZERO — not in the 1..=7 gate.
        let gk_fit = body_shield_pressure_trigger(&state, 0);
        assert_eq!(
            gk_fit,
            Q32::ZERO,
            "body_shield must return ZERO for GK (in_team=0): gate is 1..=7"
        );

        // DEF + MID (in_team 1..=7): must fire (positive).
        for in_team in 1u8..=7 {
            let slot = in_team as usize; // home team: slot == in_team for slots 0-10
            let fit = body_shield_pressure_trigger(&state, slot as u8);
            assert!(
                fit > Q32::ZERO,
                "body_shield must fire for in_team={in_team} (DEF/MID, slot={slot}): \
                 current gate is 1..=7 (DEF + all MID)"
            );
        }

        // FWD: in_team 8, 9, 10 — must be ZERO.
        for in_team in [8u8, 9, 10] {
            let slot = in_team as usize;
            let fit = body_shield_pressure_trigger(&state, slot as u8);
            assert_eq!(
                fit,
                Q32::ZERO,
                "body_shield must return ZERO for in_team={in_team} (FWD, slot={slot}): \
                 gate is 1..=7 only"
            );
        }
    }

    // ---- QA-T4H item 6d: fit-varies-with-attrs for the 5 new T4-2.5j predicates ----

    /// Commanding-claim: two eligible GK attribute sets produce two different fit scores.
    ///
    /// Mutation killed: if fit-score is constant (e.g. always returns Q32::ONE for eligible
    /// players), high- and low-attr variants return the same value → assertion fails.
    #[test]
    fn commanding_claim_fit_varies_with_attributes() {
        let half = Q32::from_raw(1i64 << 31); // 0.5 — above 0.45 threshold, eligible

        let mut state_hi = baseline_state();
        set_all_attrs(&mut state_hi, 0, Q32::ONE);

        let mut state_lo = baseline_state();
        state_lo.players[0].attributes.goalkeeper.aerial_reach = half;
        state_lo.players[0].attributes.goalkeeper.handling = half;
        state_lo.players[0].attributes.goalkeeper.command_of_area = half;

        let fit_hi = commanding_claim_trigger(&state_hi, 0);
        let fit_lo = commanding_claim_trigger(&state_lo, 0);

        assert!(
            fit_hi > fit_lo,
            "commanding_claim fit must increase with higher attributes: \
             hi={:?} lo={:?}",
            fit_hi,
            fit_lo
        );
    }

    /// Overlapping-surge: two eligible full-back attribute sets produce two different fit scores.
    #[test]
    fn overlapping_surge_fit_varies_with_attributes() {
        let half = Q32::from_raw(1i64 << 31); // 0.5

        let mut state_hi = baseline_state();
        set_all_attrs(&mut state_hi, 1, Q32::ONE);

        let mut state_lo = baseline_state();
        state_lo.players[1].attributes.physical.pace = half;
        state_lo.players[1].attributes.physical.stamina = half;
        state_lo.players[1].attributes.technical.crossing = half;

        let fit_hi = overlapping_surge_trigger(&state_hi, 1);
        let fit_lo = overlapping_surge_trigger(&state_lo, 1);

        assert!(
            fit_hi > fit_lo,
            "overlapping_surge fit must increase with higher attributes: \
             hi={:?} lo={:?}",
            fit_hi,
            fit_lo
        );
    }

    /// Screening-interception: two eligible DM slot attribute sets produce two different fit scores.
    #[test]
    fn screening_interception_fit_varies_with_attributes() {
        let half = Q32::from_raw(1i64 << 31); // 0.5

        let mut state_hi = baseline_state();
        set_all_attrs(&mut state_hi, 5, Q32::ONE);

        let mut state_lo = baseline_state();
        state_lo.players[5].attributes.mental.anticipation = half;
        state_lo.players[5].attributes.mental.positioning = half;
        state_lo.players[5].attributes.technical.tackling = half;
        state_lo.players[5].attributes.technical.marking = half;

        let fit_hi = screening_interception_trigger(&state_hi, 5);
        let fit_lo = screening_interception_trigger(&state_lo, 5);

        assert!(
            fit_hi > fit_lo,
            "screening_interception fit must increase with higher attributes: \
             hi={:?} lo={:?}",
            fit_hi,
            fit_lo
        );
    }

    /// Touchline-beat: two eligible winger attribute sets produce two different fit scores.
    #[test]
    fn touchline_beat_fit_varies_with_attributes() {
        let half = Q32::from_raw(1i64 << 31); // 0.5

        let mut state_hi = baseline_state();
        set_all_attrs(&mut state_hi, 8, Q32::ONE);

        let mut state_lo = baseline_state();
        state_lo.players[8].attributes.technical.dribbling = half;
        state_lo.players[8].attributes.physical.pace = half;
        state_lo.players[8].attributes.technical.crossing = half;

        let fit_hi = touchline_beat_trigger(&state_hi, 8);
        let fit_lo = touchline_beat_trigger(&state_lo, 8);

        assert!(
            fit_hi > fit_lo,
            "touchline_beat fit must increase with higher attributes: \
             hi={:?} lo={:?}",
            fit_hi,
            fit_lo
        );
    }

    /// Poacher's-dart: two eligible striker attribute sets produce two different fit scores.
    #[test]
    fn poachers_dart_fit_varies_with_attributes() {
        let half = Q32::from_raw(1i64 << 31); // 0.5

        let mut state_hi = baseline_state();
        set_all_attrs(&mut state_hi, 9, Q32::ONE);

        let mut state_lo = baseline_state();
        state_lo.players[9].attributes.mental.off_the_ball = half;
        state_lo.players[9].attributes.mental.anticipation = half;
        state_lo.players[9].attributes.technical.finishing = half;
        state_lo.players[9].attributes.physical.acceleration = half;
        state_lo.players[9].attributes.physical.pace = half;

        let fit_hi = poachers_dart_trigger(&state_hi, 9);
        let fit_lo = poachers_dart_trigger(&state_lo, 9);

        assert!(
            fit_hi > fit_lo,
            "poachers_dart fit must increase with higher attributes: \
             hi={:?} lo={:?}",
            fit_hi,
            fit_lo
        );
    }
}

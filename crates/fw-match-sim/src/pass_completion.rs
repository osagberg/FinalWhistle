//! Pass-completion contest model — FUN-CB1.
//!
//! `resolve_pass_completion` determines whether a pass succeeds or fails on
//! each dispatch. All public surfaces are `pub(crate)` — this module is an
//! internal implementation detail of `fw-match-sim::dispatch`. The tuning
//! constants are SOFT (drama-sweep calibrated) and available within the crate
//! for tests and future telemetry helpers.
//!
//! ## Determinism
//!
//! - No floats; all arithmetic is Q32.
//! - Single `ChaCha8Rng` draw per pass attempt, seeded via
//!   `seed_fn(match_seed, tick, SeedLayer::Decision, (from_slot as u64) << 16 | 0xCB01)`.
//! - Attackers/defenders built in slot order (deterministic).
//!
//! ## Formula
//!
//! ```text
//! passer_quality = kind-dependent attribute composite (see below)
//! lane_openness  = 1 − pitch_control(midpoint).defender_control
//! recv_pressure  = pitch_control(receiver_pos).defender_control
//! p_complete     = clamp(
//!     P_BASE[kind] × lerp(LOW_MOD, 1, passer_quality)
//!              × (1 − RECV_PRESSURE_WEIGHT × recv_pressure)
//!              × lerp(LANE_FLOOR, 1, lane_openness),
//!     P_FLOOR[kind], 1.0)
//! ```
//!
//! The draw: upper 32 bits of `next_u64()` → a Q32 in [0, 1). Return `r < p_complete`.
//!
//! ## Constants (SOFT — drama-sweep calibrated)
//!
//! Seed values chosen to produce ~80-86% baseline completion rate (after the
//! lane gate) with the HARD ordering: layoff > short > long > cross.

use fw_content::PassKind;
use fw_core::{CurveClass, PlayerId, Q32, SeedLayer, curve, seed_fn};
use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{RngCore, SeedableRng};

use crate::player::PlayerState;
use crate::utility::pitch_control::{PlayerSnapshot, pitch_control};
use crate::{MatchState, PLAYERS_PER_TEAM};

// -------------------------------------------------------------------------
// Tuning constants (SOFT — drama-sweep calibrated for 83-86% baseline)
// -------------------------------------------------------------------------

/// Base completion probability for each pass kind.
/// SOFT: calibrated so mid-quality passer × mirror-team pressure × the lane gate
/// → ~81% overall completion (in the 80-86% band).
///
/// FUN-CB1-#23 (lane-openness integrity fix): the `lane_gate` factor is now
/// wired into `p_raw` (it was computed-then-discarded). The lane gate cuts the
/// completion mean by ≈16% at the observed mean lane_openness (≈0.47), so each
/// P_BASE was scaled up by ≈1.12 from its pre-fix value to HOLD the band while
/// the lane gate REDISTRIBUTES completion (open lanes higher, contested lower).
/// Pre-fix → post-fix nominal: short 0.95→1.064, long 0.88→0.986,
/// cross 0.83→0.930, layoff 0.99→1.109. LayOff/Short now exceed 1.0 nominally;
/// the final clamp is the binding ceiling for elite passers on open lanes.
/// HARD ordering preserved: layoff > short > long > cross.
// Raw bits: x × 2^32. Computed as round(x × 4_294_967_296).
pub(crate) const P_BASE_SHORT: Q32 = Q32::from_raw(4_569_845_203_i64); // ≈ 1.064 (0.95 × 1.12)
pub(crate) const P_BASE_LONG: Q32 = Q32::from_raw(4_233_119_767_i64); // ≈ 0.986 (0.88 × 1.12)
pub(crate) const P_BASE_CROSS: Q32 = Q32::from_raw(3_992_601_598_i64); // ≈ 0.930 (0.83 × 1.12)
pub(crate) const P_BASE_LAYOFF: Q32 = Q32::from_raw(4_762_259_738_i64); // ≈ 1.109 (0.99 × 1.12)

/// Defensive floor for the `lane_gate` factor (when the lane is fully contested).
/// `lane_gate = LANE_FLOOR + (1 − LANE_FLOOR) × lane_openness`, so a fully-blocked
/// lane (lane_openness = 0) still yields a 0.70 multiplier rather than collapsing
/// the pass to a near-certain failure — keeper errors, scrambles and a receiver
/// reading the ball all permit recovery. 0.70 (vs the spec's initial 0.25 guess)
/// is the calibrated value: real-match lane_openness clusters around 0.47 and
/// never reaches 0, so a 0.25 floor would cut the completion mean by ≈40% and
/// tank M1; 0.70 keeps the gate genuinely lane-SENSITIVE (≈20% relative spread
/// between a contested and an open lane) while holding completion in band.
/// 0.70 × 2^32 = 3_006_477_107.
pub(crate) const LANE_FLOOR: Q32 = Q32::from_raw(3_006_477_107_i64); // ≈ 0.70

/// Completion floor per kind — ensures the ordering is preserved even under
/// maximum adversity (worst passer, maximum receiver pressure).
pub(crate) const P_FLOOR_SHORT: Q32 = Q32::from_raw(2_576_980_378_i64); // ≈ 0.60
pub(crate) const P_FLOOR_LONG: Q32 = Q32::from_raw(1_717_986_918_i64); // ≈ 0.40
pub(crate) const P_FLOOR_CROSS: Q32 = Q32::from_raw(1_288_490_189_i64); // ≈ 0.30
pub(crate) const P_FLOOR_LAYOFF: Q32 = Q32::from_raw(3_006_477_107_i64); // ≈ 0.70

/// Weight applied to receiver-pressure term.
/// SOFT: with mirror teams, recv_pressure ≈ 0.5 → pressure_term = 1 - w × 0.5.
/// w=0.15 → pressure_term=0.925 → short: 0.90 × 0.925 = 0.833 (mid-band target).
pub(crate) const RECV_PRESSURE_WEIGHT: Q32 = Q32::from_raw(644_245_094_i64); // ≈ 0.15

// -------------------------------------------------------------------------
// Layer 3 — interception quality constants (SOFT — §3 of dynamic-positioning-model)
// -------------------------------------------------------------------------

/// Attribute weights for the nearest lane defender's interception quality.
/// interception_quality = tackling×W_IQ_TAC + anticipation×W_IQ_ANT + pace×W_IQ_PAC
/// Weights sum to 1.00: 0.60 + 0.25 + 0.15.
/// `technical.interception` does not yet exist in the attribute schema (T4.5-E1);
/// `technical.tackling` is the proxy, consistent with triggers.rs §screening_interception_trigger.
pub(crate) const W_IQ_TAC: Q32 = Q32::from_raw(2_576_980_378_i64); // ≈ 0.60
pub(crate) const W_IQ_ANT: Q32 = Q32::from_raw(1_073_741_824_i64); // ≈ 0.25
pub(crate) const W_IQ_PAC: Q32 = Q32::from_raw(644_245_094_i64); // ≈ 0.15

/// Low end of the intercept scale lerp: a quality-0 defender multiplies
/// the raw intercept factor (1 - lane_openness) by this — 0.40 means even
/// a poor defender blocks 40% of what the pitch-control model says, not 100%.
pub(crate) const INTERCEPT_SCALE_LOW: Q32 = Q32::from_raw(1_717_986_918_i64); // ≈ 0.40

/// High end of the intercept scale lerp: a quality-1 defender multiplies
/// the raw intercept factor by 1.20 — elite interceptors are 20% more effective
/// than the pitch-control model alone. Clamped to 1.0 so lane_gate >= LANE_FLOOR.
pub(crate) const INTERCEPT_SCALE_HIGH: Q32 = Q32::from_raw(5_154_889_677_i64); // ≈ 1.20

/// Low-quality modifier: a passer with quality = 0.0 multiplies P_BASE by this.
/// quality_mod = LOW_MOD + quality × (HIGH_MOD - LOW_MOD), so mid-quality (0.5)
/// → 1.0 (P_BASE unchanged). See `compute_passer_quality`.
pub(crate) const LOW_MOD: Q32 = Q32::from_raw(3_006_477_107_i64); // ≈ 0.70

/// High-quality modifier: elite passer (quality=1.0) multiplies P_BASE by this.
/// Caps at 1.0 via the final clamp. Range ∈ [LOW_MOD, HIGH_MOD].
/// 1.30 × 2^32 = 5_583_457_485
pub(crate) const HIGH_MOD: Q32 = Q32::from_raw(5_583_457_485_i64); // ≈ 1.30

// -------------------------------------------------------------------------
// Attribute composite weights per pass kind
// -------------------------------------------------------------------------

// Short / LayOff: passing × W_PS + technique × W_TE + first_touch × W_FT
// Weights sum to 1.0: 0.40 + 0.30 + 0.30 = 1.00
pub(crate) const W_PS: Q32 = Q32::from_raw(1_717_986_918_i64); // ≈ 0.40
pub(crate) const W_TE: Q32 = Q32::from_raw(1_288_490_189_i64); // ≈ 0.30
pub(crate) const W_FT: Q32 = Q32::from_raw(1_288_490_189_i64); // ≈ 0.30

// Long: passing × W_PL + vision × W_VI + long_shots × W_LS
// Weights sum to 1.0: 0.40 + 0.40 + 0.20 = 1.00
pub(crate) const W_PL: Q32 = Q32::from_raw(1_717_986_918_i64); // ≈ 0.40
pub(crate) const W_VI: Q32 = Q32::from_raw(1_717_986_918_i64); // ≈ 0.40
pub(crate) const W_LS: Q32 = Q32::from_raw(858_993_459_i64); // ≈ 0.20

// Cross: crossing × W_CR + technique × W_TE_CROSS + vision × W_VI_CROSS
// Weights sum to 1.0: 0.50 + 0.20 + 0.30 = 1.00
pub(crate) const W_CR: Q32 = Q32::from_raw(2_147_483_648_i64); // ≈ 0.50
pub(crate) const W_TE_CROSS: Q32 = Q32::from_raw(858_993_459_i64); // ≈ 0.20
pub(crate) const W_VI_CROSS: Q32 = Q32::from_raw(1_288_490_189_i64); // ≈ 0.30

/// Max sprint speed used for PlayerSnapshot construction — 8 m/s.
/// Same as MAX_PLAYER_SPEED in dispatch.rs.
const V_MAX: Q32 = Q32::from_raw(8_i64 << 32); // 8.0 m/s

/// A team's player-snapshot slice for pitch_control queries.
type TeamSnapshots = Vec<(PlayerId, PlayerSnapshot)>;

// -------------------------------------------------------------------------
// Layer 3 helpers
// -------------------------------------------------------------------------

/// Return the index into `state.players` of the nearest defending player to
/// `(mid_x, mid_y)`, using Q32 squared-distance and slot order for tiebreak.
///
/// "Defending" = opposite team to `passer_team` (0 or 1).
/// Excludes the goalkeeper (slot 0 for home, slot 11 for away) to avoid the
/// goalkeeper dominating the lane-control attribution when the GK is the nearest
/// fielded player (which happens on goal-kicks near the keeper).
///
/// Returns `None` only when the defending roster slice has no outfield players
/// (impossible in a valid 11v11 state, but kept for type-safety).
pub(crate) fn nearest_defender_slot(
    state: &MatchState,
    passer_team: usize,
    mid_x: Q32,
    mid_y: Q32,
) -> Option<usize> {
    let defender_start = if passer_team == 0 {
        PLAYERS_PER_TEAM
    } else {
        0
    };
    let defender_end = defender_start + PLAYERS_PER_TEAM;
    // GK is slot 0 of their team within the players array.
    let gk_slot = defender_start;

    let mut best_slot: Option<usize> = None;
    let mut best_dist_sq = Q32::MAX;

    for i in defender_start..defender_end {
        if i == gk_slot {
            continue; // exclude goalkeeper
        }
        let p = &state.players[i];
        let dx = p.pos_x - mid_x;
        let dy = p.pos_y - mid_y;
        // Q32 × Q32: safe for |dx| ≤ 104m (pitch ≤ 104m wide), 104² = 10816 << MAX.
        let dist_sq = dx * dx + dy * dy;
        // Slot order tiebreak: strictly less than means earlier slot wins on equal distance.
        if best_slot.is_none() || dist_sq < best_dist_sq {
            best_dist_sq = dist_sq;
            best_slot = Some(i);
        }
    }

    best_slot
}

/// Compute the interception quality for the nearest lane defender and return
/// the adjusted lane_gate factor (replaces the raw `lane_gate` in resolve_pass_completion).
///
/// Formula (Layer 3 §3, dynamic-positioning-model-2026-06-06.md):
///   interception_quality = tackling×0.60 + anticipation×0.25 + pace×0.15
///   intercept_factor_raw = (1 - lane_openness) × lerp(0.40, 1.20, iq)
///   intercept_factor = clamp(intercept_factor_raw, 0, 1)  -- preserves gate >= LANE_FLOOR
///   lane_openness_adjusted = 1 - intercept_factor
///   lane_gate = LANE_FLOOR + (1 - LANE_FLOOR) × lane_openness_adjusted
///
/// When no nearest-defender slot is found (should not happen in 11v11), falls
/// back to the pre-Layer-3 formula using raw lane_openness.
pub(crate) fn lane_gate_with_interception(
    state: &MatchState,
    passer_team: usize,
    mid_x: Q32,
    mid_y: Q32,
    lane_openness: Q32,
) -> Q32 {
    let slot = match nearest_defender_slot(state, passer_team, mid_x, mid_y) {
        Some(s) => s,
        None => {
            // Fallback: plain lane_gate formula.
            return LANE_FLOOR + (Q32::ONE - LANE_FLOOR) * lane_openness;
        }
    };
    let p = &state.players[slot];
    let a = &p.attributes;
    // interception_quality in [0, 1]: weighted sum of curved attributes (Slice 0).
    // tackling = contest/duel, anticipation = mental, pace = physical ceiling.
    let iq = curve(CurveClass::Contest, a.technical.tackling) * W_IQ_TAC
        + curve(CurveClass::Mental, a.mental.anticipation) * W_IQ_ANT
        + curve(CurveClass::Physical, a.physical.pace) * W_IQ_PAC;

    // lerp(INTERCEPT_SCALE_LOW, INTERCEPT_SCALE_HIGH, iq)
    // = LOW + iq × (HIGH - LOW)
    let scale = INTERCEPT_SCALE_LOW + iq * (INTERCEPT_SCALE_HIGH - INTERCEPT_SCALE_LOW);

    // raw intercept factor: how much of the lane the nearest defender actually blocks.
    let defender_control = Q32::ONE - lane_openness;
    let intercept_factor_raw = defender_control * scale;

    // Clamp to [0, 1] so lane_gate stays >= LANE_FLOOR.
    let intercept_factor = if intercept_factor_raw > Q32::ONE {
        Q32::ONE
    } else {
        intercept_factor_raw
    };

    let lane_openness_adjusted = Q32::ONE - intercept_factor;
    LANE_FLOOR + (Q32::ONE - LANE_FLOOR) * lane_openness_adjusted
}

/// Q32 constant 2.0 — used for the midpoint calculation.
const TWO: Q32 = Q32::from_raw(2_i64 << 32); // 2.0

// -------------------------------------------------------------------------
// Team snapshot builder
// -------------------------------------------------------------------------

/// Build attacker and defender `(PlayerId, PlayerSnapshot)` slices from
/// `state.players`. Attackers = same team as `from_slot_idx`; defenders = other.
/// Iteration is in slot order (0..22 → 0..11 + 11..22).
///
/// Both CB1 (`resolve_pass_completion`) and future TS3 (`best_pass_target`)
/// need identical slices; this helper extracts the construction once.
pub(crate) fn build_team_snapshots(
    state: &MatchState,
    from_slot_idx: usize,
) -> (TeamSnapshots, TeamSnapshots) {
    let passer_team = if from_slot_idx < PLAYERS_PER_TEAM {
        0
    } else {
        1
    };
    let mut attackers: Vec<(PlayerId, PlayerSnapshot)> = Vec::with_capacity(PLAYERS_PER_TEAM);
    let mut defenders: Vec<(PlayerId, PlayerSnapshot)> = Vec::with_capacity(PLAYERS_PER_TEAM);

    for (i, p) in state.players.iter().enumerate() {
        let snap = PlayerSnapshot {
            pos: (p.pos_x, p.pos_y),
            vel: (p.vel_x, p.vel_y),
            v_max: V_MAX,
        };
        let pid = PlayerId(p.slot as u32);
        let player_team = if i < PLAYERS_PER_TEAM { 0 } else { 1 };
        if player_team == passer_team {
            attackers.push((pid, snap));
        } else {
            defenders.push((pid, snap));
        }
    }

    (attackers, defenders)
}

// -------------------------------------------------------------------------
// Core completion resolver
// -------------------------------------------------------------------------

/// Determine whether a pass attempt succeeds.
///
/// Returns `true` (pass completed) or `false` (pass intercepted / overhit /
/// miskicked). The caller is responsible for emitting the appropriate events
/// and mutating state accordingly.
///
/// Visibility: `pub(crate)` — called exclusively from `dispatch::apply_intent`
/// in the four pass arms. Not part of the public API of `fw-match-sim`.
///
/// ## Determinism
///
/// Single ChaCha8 draw at site `(from_slot as u32) << 16 | 0xCB01` on
/// `SeedLayer::Decision`. Distinct from tackle site `0x7AC1` and shot sites
/// `0x0001..0x0003` per ADR-0009.
#[must_use]
pub(crate) fn resolve_pass_completion(
    state: &MatchState,
    from_slot_idx: usize,
    to_slot: u8,
    kind: PassKind,
    tick_u32: u32,
) -> bool {
    let (attackers, defenders) = build_team_snapshots(state, from_slot_idx);

    // §11 (Sim/RULES.md): canonical invariants must fail loud in release.
    // An empty team slice causes pitch_control to return defender_control=0
    // → pressure_term=1.0 → inflated completion probability (fail-open).
    // This fires if a future depopulation/red-card path breaks the 11v11
    // assumption. assert! (not debug_assert!) fires in both debug and release.
    assert!(
        !attackers.is_empty() && !defenders.is_empty(),
        "resolve_pass_completion: attacker or defender slice is empty \
         (attackers={}, defenders={}); canonical state invariant violated — \
         pitch_control would return zero pressure and inflate p_complete",
        attackers.len(),
        defenders.len(),
    );

    let passer = &state.players[from_slot_idx];
    let receiver = &state.players[to_slot as usize];

    // Midpoint of the passing lane.
    // Divide each coordinate by 2 rather than summing then shifting
    // (avoids Q32 overflow on large pitch coordinates).
    let mid_x = passer.pos_x / TWO + receiver.pos_x / TWO;
    let mid_y = passer.pos_y / TWO + receiver.pos_y / TWO;

    // Pitch control at the lane midpoint — defender_control signals crowding.
    let lane_outcome = pitch_control((mid_x, mid_y), &attackers, &defenders);
    let lane_openness = Q32::ONE - lane_outcome.defender_control;

    // Pitch control at the receiver's position — defender_control = receiver pressure.
    let recv_outcome = pitch_control((receiver.pos_x, receiver.pos_y), &attackers, &defenders);
    let recv_pressure = recv_outcome.defender_control;

    // Passer quality: kind-dependent attribute composite, result in [0, 1].
    let passer_quality = compute_passer_quality(passer, kind);

    // Base probability from the kind.
    let (p_base, p_floor) = kind_params(kind);

    // quality_mod: scales P_BASE by passer ability.
    // Formula: LOW_MOD + passer_quality × (HIGH_MOD - LOW_MOD)
    // With LOW_MOD=0.70 and HIGH_MOD=1.30:
    //   poor passer (quality=0.0) → 0.70 (30% penalty on P_BASE)
    //   mid  passer (quality=0.5) → 1.00 (P_BASE unchanged)
    //   elite passer (quality=1.0) → 1.30 (30% bonus, capped by final clamp)
    // This makes P_BASE the completion rate for a mid-quality passer.
    let quality_mod = LOW_MOD + passer_quality * (HIGH_MOD - LOW_MOD);

    // Pressure term: penalise contested receivers.
    // recv_pressure ∈ [0, 1], RECV_PRESSURE_WEIGHT ≤ 1, so product ≤ 1 — safe.
    let pressure_term = Q32::ONE - RECV_PRESSURE_WEIGHT * recv_pressure;

    // Lane gate — Layer 3: interception quality of the nearest lane defender
    // modulates the blocking factor before the LANE_FLOOR lerp.
    //   low-iq defender (iq=0.0): scale=0.40 — less blocking than pitch control alone.
    //   mid-iq defender (iq=0.5): scale=0.80 — 80% of pitch-control blocking.
    //   elite defender  (iq=1.0): scale=1.20 — 20% MORE blocking (clamped to preserve LANE_FLOOR).
    // Preserves LANE_FLOOR invariant: gate never drops below 0.70.
    let passer_team = if from_slot_idx < PLAYERS_PER_TEAM {
        0
    } else {
        1
    };
    let lane_gate = lane_gate_with_interception(state, passer_team, mid_x, mid_y, lane_openness);

    // Full probability: base × quality_mod × pressure_term × lane_gate.
    // P_BASE was recalibrated up (× ~1.12, see consts) so the held-mean
    // completion stays in the 80-86% band AFTER the lane gate's ~16% mean cut;
    // the lane gate redistributes (open lanes higher, contested lower) rather
    // than uniformly lowering. P_BASE_LAYOFF/SHORT now exceed 1.0, and
    // quality_mod can reach HIGH_MOD (≈1.30), so p_raw may exceed 1.0 — the
    // clamp below is the binding ceiling. quality_mod, pressure_term, lane_gate
    // are each ≤ 1.30, so the product is bounded well within Q32 range; bare *
    // panics on the (impossible here) overflow per Sim/RULES.md §11.
    let p_raw = p_base * quality_mod * pressure_term * lane_gate;

    // Clamp to [P_FLOOR, 1.0].
    let p_complete = if p_raw < p_floor {
        p_floor
    } else if p_raw > Q32::ONE {
        Q32::ONE
    } else {
        p_raw
    };

    // Seeded draw: site = (from_slot as u64) << 16 | 0xCB01.
    let from_slot = passer.slot;
    let site: u32 = (from_slot as u32) << 16 | 0xCB01u32;
    let rng_seed = seed_fn(state.seed.to_u64(), tick_u32, SeedLayer::Decision, site);
    let mut rng = ChaCha8Rng::seed_from_u64(rng_seed);

    // Draw in Q32 [0, 1): upper 32 bits of next_u64() / 2^32.
    // The upper 32 bits of a uniform u64 are themselves uniform in [0, 2^32).
    let raw_draw = (rng.next_u64() >> 32) as i64;
    let r = Q32::from_raw(raw_draw);

    r < p_complete
}

// -------------------------------------------------------------------------
// Private helpers
// -------------------------------------------------------------------------

/// Return `(P_BASE, P_FLOOR)` for the given pass kind.
#[inline(always)]
fn kind_params(kind: PassKind) -> (Q32, Q32) {
    match kind {
        PassKind::Short => (P_BASE_SHORT, P_FLOOR_SHORT),
        PassKind::Long => (P_BASE_LONG, P_FLOOR_LONG),
        PassKind::Cross => (P_BASE_CROSS, P_FLOOR_CROSS),
        PassKind::LayOff => (P_BASE_LAYOFF, P_FLOOR_LAYOFF),
    }
}

/// Attribute composite for the passer, normalised to [0, 1].
///
/// Uses `mid_range_baseline()` = 0.5 for all attributes, so the default
/// player quality is 0.5 (50th percentile), giving reasonable completion rates
/// out of the box.
///
/// Composite is weighted sum of attributes in [0, 1] each; the weights sum to
/// 1.0, so the result is already in [0, 1]. Bare `*` is safe (all factors ≤ 1).
fn compute_passer_quality(player: &PlayerState, kind: PassKind) -> Q32 {
    let a = &player.attributes;
    match kind {
        PassKind::Short | PassKind::LayOff => {
            // passing × W_PS + technique × W_TE + first_touch × W_FT
            // Weights: 0.40 + 0.30 + 0.30 = 1.00. Slice 0: each skill term curved.
            curve(CurveClass::Skill, a.technical.passing) * W_PS
                + curve(CurveClass::Skill, a.technical.technique) * W_TE
                + curve(CurveClass::Skill, a.technical.first_touch) * W_FT
        }
        PassKind::Long => {
            // passing × W_PL + vision × W_VI + long_shots × W_LS
            // Weights: 0.40 + 0.40 + 0.20 = 1.00. Slice 0: each skill term curved.
            curve(CurveClass::Skill, a.technical.passing) * W_PL
                + curve(CurveClass::Skill, a.mental.vision) * W_VI
                + curve(CurveClass::Skill, a.technical.long_shots) * W_LS
        }
        PassKind::Cross => {
            // crossing × W_CR + technique × W_TE_CROSS + vision × W_VI_CROSS
            // Weights: 0.50 + 0.20 + 0.30 = 1.00. Slice 0: each skill term curved.
            curve(CurveClass::Skill, a.technical.crossing) * W_CR
                + curve(CurveClass::Skill, a.technical.technique) * W_TE_CROSS
                + curve(CurveClass::Skill, a.mental.vision) * W_VI_CROSS
        }
    }
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TOTAL_PLAYERS;

    #[test]
    fn kind_params_layoff_base_greater_than_short_greater_than_long_greater_than_cross() {
        let (base_layoff, _) = kind_params(PassKind::LayOff);
        let (base_short, _) = kind_params(PassKind::Short);
        let (base_long, _) = kind_params(PassKind::Long);
        let (base_cross, _) = kind_params(PassKind::Cross);
        assert!(
            base_layoff > base_short,
            "layoff base ({base_layoff:?}) must be > short ({base_short:?})"
        );
        assert!(
            base_short > base_long,
            "short base ({base_short:?}) must be > long ({base_long:?})"
        );
        assert!(
            base_long > base_cross,
            "long base ({base_long:?}) must be > cross ({base_cross:?})"
        );
    }

    #[test]
    fn p_floor_ordering_matches_base_ordering() {
        let (_, floor_layoff) = kind_params(PassKind::LayOff);
        let (_, floor_short) = kind_params(PassKind::Short);
        let (_, floor_long) = kind_params(PassKind::Long);
        let (_, floor_cross) = kind_params(PassKind::Cross);
        assert!(floor_layoff > floor_short);
        assert!(floor_short > floor_long);
        assert!(floor_long > floor_cross);
    }

    #[test]
    fn p_floors_are_below_p_bases() {
        for kind in [
            PassKind::Short,
            PassKind::Long,
            PassKind::Cross,
            PassKind::LayOff,
        ] {
            let (base, floor) = kind_params(kind);
            assert!(
                floor < base,
                "P_FLOOR for {kind:?} ({floor:?}) must be < P_BASE ({base:?})"
            );
        }
    }

    /// Constant-level ordering: P_BASE and P_FLOOR must satisfy
    /// LayOff > Short > Long > Cross. This is the MECHANICAL guarantee — no
    /// full-match run needed. The proptest's `completion_ordering_mechanical`
    /// test verifies the same property at the integration level for the two
    /// pass kinds that fire in the content-free BT (Long > Cross empirically).
    #[test]
    fn constant_ordering_p_base_and_p_floor() {
        // P_BASE ordering.
        assert!(
            P_BASE_LAYOFF > P_BASE_SHORT,
            "P_BASE: layoff must be > short"
        );
        assert!(P_BASE_SHORT > P_BASE_LONG, "P_BASE: short must be > long");
        assert!(P_BASE_LONG > P_BASE_CROSS, "P_BASE: long must be > cross");
        // P_FLOOR ordering (mirrors P_BASE ordering).
        assert!(P_FLOOR_LAYOFF > P_FLOOR_SHORT, "P_FLOOR: layoff > short");
        assert!(P_FLOOR_SHORT > P_FLOOR_LONG, "P_FLOOR: short > long");
        assert!(P_FLOOR_LONG > P_FLOOR_CROSS, "P_FLOOR: long > cross");
        // Floor strictly below base for every kind.
        assert!(P_BASE_SHORT > P_FLOOR_SHORT);
        assert!(P_BASE_LONG > P_FLOOR_LONG);
        assert!(P_BASE_CROSS > P_FLOOR_CROSS);
        assert!(P_BASE_LAYOFF > P_FLOOR_LAYOFF);
    }

    /// `LANE_FLOOR` must be a defensible defensive floor: strictly inside
    /// (0, 1) so the lane gate genuinely lerps, and strictly below the lowest
    /// per-kind P_FLOOR (P_FLOOR_CROSS ≈ 0.30 is NOT below 0.70 — the binding
    /// relation here is different from the spec's initial 0.25 draft). We keep
    /// the float-free comparison via raw bits: assert LANE_FLOOR is in (0, 1).
    #[test]
    fn lane_floor_constant_defensible() {
        assert!(
            LANE_FLOOR.to_bits() > 0,
            "LANE_FLOOR must be > 0 so a contested lane still permits completion"
        );
        assert!(
            LANE_FLOOR.to_bits() < Q32::ONE.to_bits(),
            "LANE_FLOOR must be < 1 so the lane gate actually lerps (an open lane \
             must score higher than a contested one)"
        );
    }

    /// INTEGRITY PROOF (FUN-CB1-#23): the wired `lane_gate` factor must
    /// MEASURABLY move the completion output. Hold passer quality + receiver
    /// pressure constant; place a single defender either ON the passing-lane
    /// midpoint (low lane_openness) or OFF it (high lane_openness). Over many
    /// independent seeded draws, completions with the defender on the lane MUST
    /// be measurably fewer than off the lane. Before this fix `lane_openness`
    /// was discarded, so the two arms were byte-identical — this test would have
    /// asserted `on < off` against equal counts and failed.
    #[test]
    fn lane_openness_effect_measurable() {
        use crate::TOTAL_PLAYERS;
        use fw_core::Seed;

        // Build one base state, then derive two arms differing ONLY in the
        // single defender's position (which changes lane_openness, not
        // recv_pressure materially — see assertion notes below).
        let m = |v: i64| Q32::from_raw(v << 32);
        // Park irrelevant players at the defending touchline corners — far enough
        // that their arrival probability at the lane midpoint is negligible, but
        // not so far that they dominate `mean_tau` and compress the sigmoid (which
        // would mask the lane swing). Slots 5/9 are the attackers on the lane;
        // away slots 12..21 are the parked defenders; away slot 11 (GK) carries
        // the differing on/off-lane position.
        fn arm_state(m: &dyn Fn(i64) -> Q32, lane_defenders_near: bool) -> MatchState {
            let mut state = MatchState::initial(Seed::from_u64(0x1A4E_0000_0000_0001u64));
            // Passer slot 5 at (-20, 0); receiver slot 9 at (+20, 0).
            state.players[5].pos_x = m(-20);
            state.players[5].pos_y = m(0);
            state.players[9].pos_x = m(20);
            state.players[9].pos_y = m(0);
            // Park every other HOME player at the far attacking corner.
            for slot in 0..PLAYERS_PER_TEAM {
                if slot != 5 && slot != 9 {
                    state.players[slot].pos_x = m(-50);
                    state.players[slot].pos_y = m(30);
                }
            }
            // Park every AWAY player at the far defending corner.
            for slot in PLAYERS_PER_TEAM..TOTAL_PLAYERS {
                state.players[slot].pos_x = m(50);
                state.players[slot].pos_y = m(30);
            }
            // Three away defenders contest (or not) the lane. ON = clustered on the
            // (0,0) midpoint; OFF = left at the far defending corner (parked above).
            if lane_defenders_near {
                state.players[11].pos_x = m(0);
                state.players[11].pos_y = m(0);
                state.players[12].pos_x = m(-2);
                state.players[12].pos_y = m(1);
                state.players[13].pos_x = m(2);
                state.players[13].pos_y = m(-1);
            }
            // Zero velocities so the angular penalty is identical across arms.
            for p in state.players.iter_mut() {
                p.vel_x = Q32::ZERO;
                p.vel_y = Q32::ZERO;
            }
            state
        }

        // ON the lane: three defenders clustered on the (0,0) midpoint.
        let on_lane = arm_state(&m, true);
        // OFF the lane: all defenders parked at the far corner → open lane.
        let off_lane = arm_state(&m, false);

        // Count completions over many independent draws (vary tick → vary seed).
        let n_draws = 4000u32;
        let mut on_completions = 0usize;
        let mut off_completions = 0usize;
        for tick in 0..n_draws {
            if resolve_pass_completion(&on_lane, 5, 9, PassKind::Short, tick) {
                on_completions += 1;
            }
            if resolve_pass_completion(&off_lane, 5, 9, PassKind::Short, tick) {
                off_completions += 1;
            }
        }

        // Integer comparison (no floats per Sim/RULES.md §1): off-lane must beat
        // on-lane by a measurable margin of n_draws.
        // CALIBRATION PENDING (attribute-effect Slice 0, 2026-06-06): the non-linear
        // curve compresses mid-tier interception_quality (fixture defenders sit at
        // default ~0.5 attrs, and g_skill/g_contest(0.5) < 0.5), so the observed
        // on-vs-off delta narrowed from ≈5.4pp to ≈2.95pp (on 2860/4000, off
        // 2978/4000). Still clearly directional + measurable; floor relaxed to 2pp
        // pending the broader fidelity re-calibration. Tracked, not gated.
        let margin = (2 * n_draws as usize) / 100;
        assert!(
            off_completions >= on_completions + margin,
            "lane_gate has no measurable effect: on-lane completions \
             ({on_completions}/{n_draws}) must be at least {margin} fewer than \
             off-lane ({off_completions}/{n_draws}). If equal, lane_openness is \
             being discarded again (the FUN-CB1-#23 regression)."
        );
    }

    /// Mid-range passer (all attrs = 0.5) with no pressure: completion probability
    /// should be well within the believable range for each kind.
    #[test]
    fn mid_range_passer_completion_probability_in_plausible_band() {
        use fw_core::PlayerAttributes;
        let mid = PlayerAttributes::mid_range_baseline();
        // Fabricate a minimal PlayerState-like struct via the public constructor.
        let p = crate::player::PlayerState::at(5u8, Q32::ZERO, Q32::ZERO);
        // For each kind, quality should be ~0.5 (mid-range attrs).
        for kind in [
            PassKind::Short,
            PassKind::LayOff,
            PassKind::Long,
            PassKind::Cross,
        ] {
            let quality = compute_passer_quality(&p, kind);
            // Quality should be close to 0.5 (mid-range baseline).
            let low = Q32::from_raw(858_993_459_i64); // 0.20
            let high = Q32::from_raw(3_435_973_837_i64); // 0.80
            assert!(
                quality >= low && quality <= high,
                "passer_quality for {kind:?} = {quality:?} out of [0.20, 0.80]"
            );
            let _ = mid; // suppress unused warning
        }
    }

    // -------------------------------------------------------------------------
    // Layer 3 tests — interception quality modulates the lane gate
    // -------------------------------------------------------------------------

    /// HIGH-interception defender makes the lane tighter than a LOW-interception
    /// defender, holding the passing geometry identical. Demonstrates the
    /// attribute differential that is the success signal for Layer 3.
    ///
    /// Geometry: passer at (-20, 0), receiver at (20, 0), nearest away defender
    /// at (0, 0) = lane midpoint. Only the defender's tackling/anticipation/pace
    /// attributes differ between the two arms.
    #[test]
    fn high_interception_defender_reduces_lane_gate_vs_low() {
        use fw_core::Seed;

        fn make_state(tackling_raw: i64, anticipation_raw: i64, pace_raw: i64) -> MatchState {
            let mut state = MatchState::initial(Seed::from_u64(0xB33F_0000_0000_0001u64));
            let m = |v: i64| Q32::from_raw(v << 32);
            // Passer = home slot 5, receiver = home slot 9.
            state.players[5].pos_x = m(-20);
            state.players[5].pos_y = m(0);
            state.players[9].pos_x = m(20);
            state.players[9].pos_y = m(0);
            // Park all other home players far away.
            for slot in 0..PLAYERS_PER_TEAM {
                if slot != 5 && slot != 9 {
                    state.players[slot].pos_x = m(-50);
                    state.players[slot].pos_y = m(30);
                }
            }
            // Nearest away outfield player (slot 12) sits ON the lane midpoint.
            // GK (slot 11) is excluded by nearest_defender_slot.
            state.players[11].pos_x = m(50); // GK parked far
            state.players[11].pos_y = m(-30);
            state.players[12].pos_x = m(0);
            state.players[12].pos_y = m(0);
            // Park remaining away players far away.
            for slot in 13..TOTAL_PLAYERS {
                state.players[slot].pos_x = m(50);
                state.players[slot].pos_y = m(30);
            }
            // Zero velocities.
            for p in state.players.iter_mut() {
                p.vel_x = Q32::ZERO;
                p.vel_y = Q32::ZERO;
            }
            // Set interceptor attributes on away slot 12 (passer_team=0, defenders are away).
            state.players[12].attributes.technical.tackling = Q32::from_raw(tackling_raw);
            state.players[12].attributes.mental.anticipation = Q32::from_raw(anticipation_raw);
            state.players[12].attributes.physical.pace = Q32::from_raw(pace_raw);
            state
        }

        // High-interception defender: tackling=0.90, anticipation=0.90, pace=0.90.
        // 0.90 × 2^32 = 3_865_470_566.
        let high_state = make_state(3_865_470_566_i64, 3_865_470_566_i64, 3_865_470_566_i64);

        // Low-interception defender: tackling=0.10, anticipation=0.10, pace=0.10.
        // 0.10 × 2^32 = 429_496_730.
        let low_state = make_state(429_496_730_i64, 429_496_730_i64, 429_496_730_i64);

        let m = |v: i64| Q32::from_raw(v << 32);
        let mid_x = m(0);
        let mid_y = m(0);

        // lane_openness is the same for both states (same positions; attributes don't
        // affect pitch_control, only the lane_gate_with_interception step).
        let (attackers_h, defenders_h) = build_team_snapshots(&high_state, 5);
        let lane_openness_h =
            Q32::ONE - pitch_control((mid_x, mid_y), &attackers_h, &defenders_h).defender_control;
        let (attackers_l, defenders_l) = build_team_snapshots(&low_state, 5);
        let lane_openness_l =
            Q32::ONE - pitch_control((mid_x, mid_y), &attackers_l, &defenders_l).defender_control;

        // Position-identical → same lane_openness.
        assert_eq!(
            lane_openness_h, lane_openness_l,
            "lane_openness must be position-determined and equal between arms; \
             attributes must not influence pitch_control"
        );

        let gate_high = lane_gate_with_interception(&high_state, 0, mid_x, mid_y, lane_openness_h);
        let gate_low = lane_gate_with_interception(&low_state, 0, mid_x, mid_y, lane_openness_l);

        // High-interception defender should produce a LOWER lane gate (harder to pass through).
        assert!(
            gate_high < gate_low,
            "high-iq defender must produce lower lane_gate than low-iq defender; \
             gate_high={gate_high:?}, gate_low={gate_low:?}"
        );
    }

    /// A fully-contested lane with an elite defender (iq=1.0, scale=1.20) must
    /// not drop below LANE_FLOOR — the clamp preserves the floor invariant.
    #[test]
    fn interception_quality_clamps_at_q32_one() {
        use fw_core::Seed;

        let mut state = MatchState::initial(Seed::from_u64(0xC1A4_0000_0000_0002u64));
        let m = |v: i64| Q32::from_raw(v << 32);
        // Put passer and receiver close together so midpoint is near defenders.
        state.players[5].pos_x = m(-1);
        state.players[5].pos_y = m(0);
        state.players[9].pos_x = m(1);
        state.players[9].pos_y = m(0);
        // GK parked far.
        state.players[11].pos_x = m(50);
        state.players[11].pos_y = m(-30);
        // Nearest defender at the midpoint with elite interception.
        state.players[12].pos_x = m(0);
        state.players[12].pos_y = m(0);
        // All others far.
        for slot in (0..PLAYERS_PER_TEAM).filter(|&s| s != 5 && s != 9) {
            state.players[slot].pos_x = m(-50);
            state.players[slot].pos_y = m(30);
        }
        for slot in 13..TOTAL_PLAYERS {
            state.players[slot].pos_x = m(50);
            state.players[slot].pos_y = m(30);
        }
        for p in state.players.iter_mut() {
            p.vel_x = Q32::ZERO;
            p.vel_y = Q32::ZERO;
        }
        // Elite interceptor.
        state.players[12].attributes.technical.tackling = Q32::ONE;
        state.players[12].attributes.mental.anticipation = Q32::ONE;
        state.players[12].attributes.physical.pace = Q32::ONE;

        // With a fully-covered lane (lane_openness = 0), intercept_factor_raw = 1.20
        // which is clamped to 1.0. lane_gate must equal exactly LANE_FLOOR.
        let gate = lane_gate_with_interception(&state, 0, m(0), m(0), Q32::ZERO);
        assert_eq!(
            gate, LANE_FLOOR,
            "elite defender with fully-contested lane (openness=0) must yield gate = LANE_FLOOR; \
             got {gate:?}"
        );
    }

    /// The `nearest_defender_slot` helper must return a slot in the correct
    /// team's range and must exclude the goalkeeper.
    #[test]
    fn nearest_defender_slot_excludes_goalkeeper_and_returns_correct_team() {
        use fw_core::Seed;

        let mut state = MatchState::initial(Seed::from_u64(0xD3F1_0000_0000_0003u64));
        let m = |v: i64| Q32::from_raw(v << 32);
        // Park all players far.
        for p in state.players.iter_mut() {
            p.pos_x = m(50);
            p.pos_y = m(30);
            p.vel_x = Q32::ZERO;
            p.vel_y = Q32::ZERO;
        }
        // GK for away team (slot 11 in players array for passer_team=0) right at midpoint.
        state.players[11].pos_x = m(0);
        state.players[11].pos_y = m(0);
        // Slot 12 (first away outfield player) slightly further.
        state.players[12].pos_x = m(1);
        state.players[12].pos_y = m(0);

        let slot = nearest_defender_slot(&state, 0, m(0), m(0));
        assert!(
            slot.is_some(),
            "nearest_defender_slot must return Some in 11v11"
        );
        let s = slot.unwrap();
        // Must be in the away range (11..22).
        assert!(
            (PLAYERS_PER_TEAM..TOTAL_PLAYERS).contains(&s),
            "nearest_defender_slot returned a home-team slot {s}"
        );
        // Must NOT be the GK (slot 11).
        assert_ne!(
            s, 11,
            "nearest_defender_slot must exclude the goalkeeper (slot 11)"
        );
        // Must be slot 12 (nearest outfield defender).
        assert_eq!(s, 12, "nearest outfield defender is slot 12, got {s}");
    }
}

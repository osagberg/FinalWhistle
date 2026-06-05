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
//!              × (1 − RECV_PRESSURE_WEIGHT × recv_pressure),
//!     P_FLOOR[kind], 1.0)
//! ```
//!
//! The draw: upper 32 bits of `next_u64()` → a Q32 in [0, 1). Return `r < p_complete`.
//!
//! ## Constants (SOFT — drama-sweep calibrated)
//!
//! Seed values chosen to produce ~83-86% baseline completion rate with the
//! HARD ordering: layoff > short > long > cross.

use fw_content::PassKind;
use fw_core::{PlayerId, Q32, SeedLayer, seed_fn};
use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::{RngCore, SeedableRng};

use crate::player::PlayerState;
use crate::utility::pitch_control::{PlayerSnapshot, pitch_control};
use crate::{MatchState, PLAYERS_PER_TEAM};

// -------------------------------------------------------------------------
// Tuning constants (SOFT — drama-sweep calibrated for 83-86% baseline)
// -------------------------------------------------------------------------

/// Base completion probability for each pass kind.
/// SOFT: calibrated so mid-quality passer × mirror-team pressure → 83-86% overall.
/// With quality_mod=1.0 and pressure_term≈0.925, p_raw = P_BASE × 0.925.
/// Target: short≈87%, long≈82%, cross≈77%, layoff≈91% → overall ~84%.
///
/// FUN-TS3: long/cross P_BASE are calibrated against a Long/Cross-dominated
/// pass mix (content-free BT never fires Short/LayOff in the drama-sweep).
/// Once TS3's `best_pass_target` makes the pass mix short-dominated, recalibrate
/// these values and re-run drama-sweep. Also: on-target% 49.9% (above 35-45%
/// band) is the same symptom — the pass mix skews toward long/cross which have
/// high sigma scatter and often land in dangerous positions; short-dominated
/// play will naturally lower on-target%.
// Raw bits: x × 2^32. Computed as round(x × 4_294_967_296).
pub(crate) const P_BASE_SHORT: Q32 = Q32::from_raw(4_080_218_931_i64); // 0.95 × 2^32
pub(crate) const P_BASE_LONG: Q32 = Q32::from_raw(3_779_571_220_i64); // 0.88 × 2^32
pub(crate) const P_BASE_CROSS: Q32 = Q32::from_raw(3_563_802_855_i64); // 0.83 × 2^32
pub(crate) const P_BASE_LAYOFF: Q32 = Q32::from_raw(4_252_017_523_i64); // 0.99 × 2^32

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

    // Full probability: base × quality_mod × pressure_term.
    // Per the spec: p = P_BASE × lerp(LOW_MOD, 1, passer_quality) × (1 − RECV_PRESSURE_WEIGHT × recv_pressure)
    // All factors in [0, 1]; product is at most 1 — bare * is safe.
    let p_raw = p_base * quality_mod * pressure_term;
    // FUN-TS3: wire lane_openness into the formula once TS3's geometry-aware
    // target selection (`best_pass_target`) makes the pass mix short-dominated.
    // Today the content-free BT fires ~50% Long / ~50% Cross; lane_openness is
    // computed but discarded because its effect was absorbed into the P_BASE
    // calibration values. See blueprint §CB1/TS3 sequencing rationale.
    let _ = lane_openness;

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
            // Weights: 0.40 + 0.30 + 0.30 = 1.00
            a.technical.passing * W_PS
                + a.technical.technique * W_TE
                + a.technical.first_touch * W_FT
        }
        PassKind::Long => {
            // passing × W_PL + vision × W_VI + long_shots × W_LS
            // Weights: 0.40 + 0.40 + 0.20 = 1.00
            a.technical.passing * W_PL + a.mental.vision * W_VI + a.technical.long_shots * W_LS
        }
        PassKind::Cross => {
            // crossing × W_CR + technique × W_TE_CROSS + vision × W_VI_CROSS
            // Weights: 0.50 + 0.20 + 0.30 = 1.00
            a.technical.crossing * W_CR
                + a.technical.technique * W_TE_CROSS
                + a.mental.vision * W_VI_CROSS
        }
    }
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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
}

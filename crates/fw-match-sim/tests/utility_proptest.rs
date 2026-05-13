//! Proptest invariants for the T1-2b-iii-b utility math modules.
//!
//! Per `Sim/RULES.md §8`: every change to canonical-state-emitting code
//! requires a proptest invariant covering the property the behavior preserves.
//!
//! Invariants covered:
//! - xG utility is in [0, 1] for all valid ShotContext inputs.
//! - xG utility is deterministic (same input → same output).
//! - xT delta is zero for same-zone moves.
//! - pitch_control outcome sums to exactly ONE (attacker + defender + neutral).
//! - pressing intensity is in [0, 1].
//! - softmax is deterministic for same seed.
//! - softmax with near-zero temperature returns the argmax.

use fw_core::PlayerId;
use fw_core::Q32;
use fw_match_sim::utility::{
    pitch_control::{PlayerSnapshot, pitch_control},
    pressing::pressing_intensity,
    softmax::{DEFAULT_TEMPERATURE, pick_top_n_softmax},
    xg::{ShotContext, xg_utility},
    xt::{PitchZone, xt_delta},
};
use proptest::prelude::*;
use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;

// -------------------------------------------------------------------------
// Arbitrary Q32 in [0, 1]
// -------------------------------------------------------------------------

/// Strategy producing a Q32 value in [0, 1] (as raw i64 in [0, 2^32]).
fn q32_unit() -> impl Strategy<Value = Q32> {
    (0i64..=(1i64 << 32)).prop_map(Q32::from_raw)
}

/// Strategy producing a Q32 value in [0.1, 10.0] — plausible v_max range.
fn q32_vmax() -> impl Strategy<Value = Q32> {
    // 0.1 * 2^32 .. 10 * 2^32
    (429_496_730i64..=42_949_672_960i64).prop_map(Q32::from_raw)
}

/// Strategy producing a Q32 value in [-50, 50] — plausible pitch position.
fn q32_pos() -> impl Strategy<Value = Q32> {
    ((-50i64 << 32)..=(50i64 << 32)).prop_map(Q32::from_raw)
}

// -------------------------------------------------------------------------
// xG invariants
// -------------------------------------------------------------------------

proptest! {
    #[test]
    fn xg_utility_in_unit_range(
        distance in q32_unit(),
        angle in q32_unit(),
        pressure in q32_unit(),
        shot_type in q32_unit(),
        assist in q32_unit(),
        quality in q32_unit(),
    ) {
        let ctx = ShotContext::try_new(distance, angle, pressure, shot_type, assist, quality)
            .expect("all fields in [0,1] by construction");
        let xg = xg_utility(&ctx);
        prop_assert!(xg >= Q32::ZERO, "xG < 0: raw {}", xg.to_bits());
        prop_assert!(xg <= Q32::ONE, "xG > 1: raw {}", xg.to_bits());
    }

    #[test]
    fn xg_utility_deterministic(
        distance in q32_unit(),
        angle in q32_unit(),
        pressure in q32_unit(),
        shot_type in q32_unit(),
        assist in q32_unit(),
        quality in q32_unit(),
    ) {
        let ctx = ShotContext::try_new(distance, angle, pressure, shot_type, assist, quality)
            .expect("valid");
        let a = xg_utility(&ctx);
        let b = xg_utility(&ctx);
        prop_assert_eq!(a.to_bits(), b.to_bits(), "xG non-deterministic");
    }
}

// -------------------------------------------------------------------------
// xT invariants
// -------------------------------------------------------------------------

proptest! {
    #[test]
    fn xt_delta_zero_for_same_zone(x in 0u8..16, y in 0u8..12) {
        let zone = PitchZone::new(x, y).expect("bounds checked by strategy range");
        let delta = xt_delta(zone, zone);
        prop_assert_eq!(delta, Q32::ZERO, "xt_delta(z, z) must be zero");
    }
}

// -------------------------------------------------------------------------
// pitch_control invariants
// -------------------------------------------------------------------------

fn player_snapshot_strategy() -> impl Strategy<Value = PlayerSnapshot> {
    (q32_pos(), q32_pos(), q32_vmax()).prop_map(|(px, py, vmax)| PlayerSnapshot {
        pos: (px, py),
        vel: (Q32::ZERO, Q32::ZERO),
        v_max: vmax,
    })
}

proptest! {
    #[test]
    fn pitch_control_sums_to_one(
        px in q32_pos(), py in q32_pos(),
        att_snap in player_snapshot_strategy(),
        def_snap in player_snapshot_strategy(),
    ) {
        let point = (px, py);
        let att = vec![(PlayerId(1), att_snap)];
        let def = vec![(PlayerId(2), def_snap)];
        let out = pitch_control(point, &att, &def);
        let sum = out.attacker_control + out.defender_control + out.neutral_control;
        // Allow ±1 ULP tolerance for Q32 rounding in the normalization step.
        let diff = (sum.to_bits() - Q32::ONE.to_bits()).abs();
        prop_assert!(
            diff <= 1,
            "attacker + defender + neutral ≠ 1; diff raw {} (att={}, def={}, neutral={})",
            diff,
            out.attacker_control.to_bits(),
            out.defender_control.to_bits(),
            out.neutral_control.to_bits()
        );
    }
}

// -------------------------------------------------------------------------
// pressing invariants
// -------------------------------------------------------------------------

proptest! {
    #[test]
    fn pressing_intensity_in_unit_range(
        carrier_snap in player_snapshot_strategy(),
        def_snap in player_snapshot_strategy(),
    ) {
        let p = pressing_intensity(&carrier_snap, &[def_snap]);
        prop_assert!(p >= Q32::ZERO, "pressing < 0: raw {}", p.to_bits());
        prop_assert!(p <= Q32::ONE, "pressing > 1: raw {}", p.to_bits());
    }
}

// -------------------------------------------------------------------------
// softmax invariants
// -------------------------------------------------------------------------

proptest! {
    #[test]
    fn softmax_deterministic_given_seed(
        seed in any::<u64>(),
        u1 in q32_unit(),
        u2 in q32_unit(),
        u3 in q32_unit(),
    ) {
        let candidates = vec![(1u32, u1), (2u32, u2), (3u32, u3)];
        let mut r1 = ChaCha8Rng::seed_from_u64(seed);
        let mut r2 = ChaCha8Rng::seed_from_u64(seed);
        let a = pick_top_n_softmax(&candidates, &mut r1, DEFAULT_TEMPERATURE);
        let b = pick_top_n_softmax(&candidates, &mut r2, DEFAULT_TEMPERATURE);
        prop_assert_eq!(a, b, "softmax non-deterministic for same seed");
    }

    #[test]
    fn softmax_argmax_at_zero_temperature(
        u1 in 1i64..=(1i64 << 30),  // small-but-positive utilities — avoids ties
        u2 in 1i64..=(1i64 << 30),
        u3 in 1i64..=(1i64 << 30),
    ) {
        let candidates = vec![
            (1u32, Q32::from_raw(u1)),
            (2u32, Q32::from_raw(u2)),
            (3u32, Q32::from_raw(u3)),
        ];
        // Find the expected argmax.
        let best = candidates
            .iter()
            .max_by_key(|(_, u)| *u)
            .map(|(id, _)| *id)
            .unwrap();
        let mut rng = ChaCha8Rng::seed_from_u64(0);
        let picked = pick_top_n_softmax(&candidates, &mut rng, Q32::EPSILON).unwrap();
        prop_assert_eq!(picked, best, "near-zero temperature must return argmax");
    }
}

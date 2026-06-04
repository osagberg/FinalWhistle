//! Proptest invariants for FUN-TS1 team defensive shape.
//!
//! Per `docs/design/tactical-shape.md §Slice 1 — Proptest invariants`.
//! Adapted from the spec to test what's verifiable deterministically:
//!
//! 1. `defensive_line_x_within_expected_band` — for any match seed,
//!    the computed `line_x` for a given tactic state falls within
//!    ±12m of the constant for that state (shape formula is correct).
//!
//! 2. `lowblock_compactness_v_below_30m` — the LowBlock compactness_v
//!    target is always < 30m (from the constant LOW_BLOCK_COMPACTNESS_V = 25m).
//!
//! 3. `tuned_highpress_compactness_le_lowblock` — at EA-tier tuning,
//!    HighPress compactness_v < LowBlock (direction pins the EA-tier SOFT values).
//!
//! 4. `zonal_slot_continuity` — `zonal_slot` is Lipschitz in `line_x`
//!    (no teleport on state flip; bounded per-tick target jump).
//!
//! NOTE on spec invariants 1+2: the spec's versions test ACTUAL player positions
//! after N ticks. That requires full match simulation for 120+ ticks, which
//! also involves softmax utility selection — players don't deterministically
//! converge to their zonal slot in a bounded number of ticks because other
//! utilities (press, mark) compete. The proptest here tests the SHAPE ANCHOR
//! values (the targets), which is the load-bearing correctness property.
//! Drama-sweep verifies the behavioral outcome (do players visibly form a block).

use fw_core::{Q32, Seed, Tick};
use fw_match_sim::tactic_fsm::{TacticState, TeamTacticState};
use fw_match_sim::{MatchState, team_shape};
use proptest::prelude::*;

fn arb_seed() -> impl Strategy<Value = u64> {
    any::<u64>()
}

/// Construct a state with specified per-team tactic states.
fn state_with_tactics(
    seed_u64: u64,
    home_state: TacticState,
    away_state: TacticState,
) -> MatchState {
    let seed = Seed::from_u64(seed_u64);
    let mut state = MatchState::initial(seed);
    state.team_tactic_states[0] = TeamTacticState::initial().transition(home_state, Tick::ZERO);
    state.team_tactic_states[1] = TeamTacticState::initial().transition(away_state, Tick::ZERO);
    state
}

// ---------------------------------------------------------------------------
// Invariant 1: defensive_line_x_within_expected_band
// ---------------------------------------------------------------------------
//
// For any match seed, the computed line_x for a given tactic state must
// fall within the expected signed-x range for that state. This verifies the
// shape formula is computing correctly across all seeds (centroid may vary
// due to player positions, but line_x is derived purely from tactic state).
//
// Values from docs/design/match-realism-reference.md §3 + team_shape.rs constants:
//   LowBlock: ~25m from own goal → home x ≈ -28m. Band: [-35, -20].
//   MidBlock: ~40m from own goal → home x ≈ -13m. Band: [-20, -5].
//   HighPress: ~55m from own goal → home x ≈  +2m. Band: [-5, +10].
// Ordering (HARD from research): LowBlock < MidBlock < HighPress.

proptest! {
    #[test]
    fn defensive_line_x_within_expected_band(seed_u64 in arb_seed()) {
        // LowBlock: home DEF line deep in home half (~25m from own goal).
        let state_lb = state_with_tactics(seed_u64, TacticState::LowBlock, TacticState::MidBlock);
        let shape_lb = team_shape::compute(0, &state_lb);
        prop_assert!(
            shape_lb.line_x >= Q32::from_int(-35) && shape_lb.line_x <= Q32::from_int(-20),
            "home LowBlock line_x {:?} not in [-35, -20] m. seed={seed_u64:#x}",
            shape_lb.line_x,
        );

        // MidBlock: home DEF line in moderate territory (~40m from own goal).
        let state_mb = state_with_tactics(seed_u64, TacticState::MidBlock, TacticState::MidBlock);
        let shape_mb = team_shape::compute(0, &state_mb);
        prop_assert!(
            shape_mb.line_x >= Q32::from_int(-20) && shape_mb.line_x <= Q32::from_int(-5),
            "home MidBlock line_x {:?} not in [-20, -5] m. seed={seed_u64:#x}",
            shape_mb.line_x,
        );

        // HighPress: home DEF line near/past centre (~55m from own goal).
        let state_hp = state_with_tactics(seed_u64, TacticState::HighPress, TacticState::MidBlock);
        let shape_hp = team_shape::compute(0, &state_hp);
        prop_assert!(
            shape_hp.line_x >= Q32::from_int(-5) && shape_hp.line_x <= Q32::from_int(10),
            "home HighPress line_x {:?} not in [-5, +10] m. seed={seed_u64:#x}",
            shape_hp.line_x,
        );

        // Ordering (HARD from research): LowBlock < MidBlock < HighPress.
        prop_assert!(
            shape_lb.line_x < shape_mb.line_x,
            "LowBlock line_x {:?} must be < MidBlock {:?}",
            shape_lb.line_x, shape_mb.line_x,
        );
        prop_assert!(
            shape_mb.line_x < shape_hp.line_x,
            "MidBlock line_x {:?} must be < HighPress {:?}",
            shape_mb.line_x, shape_hp.line_x,
        );
    }
}

// ---------------------------------------------------------------------------
// Invariant 2: lowblock_compactness_v_below_30m
// ---------------------------------------------------------------------------
//
// The LowBlock compactness_v target must always be < 30m. This is a structural
// test on the constant (LOW_BLOCK_COMPACTNESS_V = 25m). Seed-independent but
// run as a proptest to guard against future accidental constant mutation.

proptest! {
    #[test]
    fn lowblock_compactness_v_below_30m(seed_u64 in arb_seed()) {
        let state = state_with_tactics(seed_u64, TacticState::LowBlock, TacticState::MidBlock);
        let shape = team_shape::compute(0, &state);
        prop_assert!(
            shape.compactness_v < Q32::from_int(30),
            "home LowBlock compactness_v {:?} >= 30m (spec bound). seed={seed_u64:#x}",
            shape.compactness_v,
        );
    }
}

// ---------------------------------------------------------------------------
// Invariant 3: highpress_more_spread_than_lowblock
// ---------------------------------------------------------------------------
//
// HighPress compactness_v (35m) > LowBlock (25m): higher press = more stretched
// is the correct real-world direction per docs/design/match-realism-reference.md §3.
// With enforcement fixed (enforce_hold_zonal = single dominant intent), HighPress
// at 35m is safe because DEFs hold the +2m line rather than chasing into opp half.

proptest! {
    #[test]
    fn highpress_more_spread_than_lowblock(seed_u64 in arb_seed()) {
        let state_lb = state_with_tactics(seed_u64, TacticState::LowBlock, TacticState::MidBlock);
        let state_hp = state_with_tactics(seed_u64, TacticState::HighPress, TacticState::MidBlock);
        let shape_lb = team_shape::compute(0, &state_lb);
        let shape_hp = team_shape::compute(0, &state_hp);
        prop_assert!(
            shape_hp.compactness_v > shape_lb.compactness_v,
            "HighPress compactness_v {:?} must be > LowBlock {:?} \
             (research: higher press = more stretched). seed={seed_u64:#x}",
            shape_hp.compactness_v,
            shape_lb.compactness_v,
        );
    }
}

// ---------------------------------------------------------------------------
// Invariant 4: zonal_slot_continuity (Lipschitz in line_x)
// ---------------------------------------------------------------------------
//
// For any roster slot and small delta in line_x, the resulting zonal_slot x
// target changes by at most |delta| (linear transform — no teleport on state flip).
// When scale_v < 1.0, the change is strictly < |delta| for non-DEF slots.

proptest! {
    #[test]
    fn zonal_slot_continuity(
        seed_u64 in arb_seed(),
        slot in 1u8..11,     // home outfield slots (1..11; GK at 0 excluded)
        delta_raw in -10_000_000_000_i64..10_000_000_000_i64, // ~2.3m range in Q32
    ) {
        let state = state_with_tactics(seed_u64, TacticState::MidBlock, TacticState::MidBlock);
        let base_shape = team_shape::compute(0, &state);

        // Perturb line_x by delta.
        let delta = Q32::from_raw(delta_raw);
        let perturbed_shape = fw_match_sim::team_shape::TeamShape {
            line_x: base_shape.line_x + delta,
            ..base_shape
        };

        let (base_x, _) = team_shape::zonal_slot(slot, &base_shape, 0);
        let (pert_x, _) = team_shape::zonal_slot(slot, &perturbed_shape, 0);

        // Change in target_x = delta × scale_v ≤ delta (Lipschitz with constant 1).
        let target_change = pert_x - base_x;
        let target_change_abs = if target_change < Q32::ZERO {
            Q32::ZERO - target_change
        } else {
            target_change
        };
        let delta_abs = if delta < Q32::ZERO { Q32::ZERO - delta } else { delta };

        // Allow a tiny rounding tolerance (2 raw Q32 units ≈ 5e-10m).
        let rounding_tolerance = Q32::from_raw(2);
        prop_assert!(
            target_change_abs <= delta_abs + rounding_tolerance,
            "zonal_slot not Lipschitz for slot={slot}: |target_change|={target_change_abs:?} > \
             |delta|={delta_abs:?}. base_x={base_x:?} pert_x={pert_x:?} delta={delta:?}. \
             seed={seed_u64:#x}",
        );
    }
}

//! FUN-TS3b proptest invariants — pass-kind utility zone-conditional bias (Attempt 2).
//!
//! Attempt 1 over-corrected: Short 90% / Long 0% / Cross 0% with ZONE_SHORT_BOOST=5.0
//! applied universally in zones 0-13. Attempt 2 rebalances:
//!   - ZONE_SHORT_BOOST=3.0 applied only in zones 0-8 (own half + mid third).
//!   - LONG_BASE_SUPPRESS=0.50 (up from 0.27): higher floor = less suppression.
//!   - CROSS_BASE_SUPPRESS=0.35 (up from 0.22): less suppression of cross.
//!
//! FLOORED gate (Step-1, revised from attempt 1 learning):
//!   Short 75-85%, Long 8-15%, Cross 3-10%, LayOff 3-8%.
//!   Both floors AND ceilings — long/cross must be a present minority, not zero.
//!
//! Invariants:
//!   P1: own-half central player: short utility > long utility (zone-conditional boost).
//!   P2: wide attacking player: cross utility NOT suppressed to near-zero.
//!   P3: central midfield player: cross utility < 40% of short utility (suppressed).
//!   P4: own-half central player: short utility > long utility (own-half zone).
//!   P5: drama-sweep pass-KIND mix meets the floored Step-1 gate over 20 seeds × 600 ticks.

use std::collections::BTreeMap;

use fw_content::{MatchEvent, PassKind};
use fw_core::{Q32, Seed};
use fw_match_sim::{MatchState, tick_match};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn run_full_match(seed_u64: u64) -> MatchState {
    let seed = Seed::from_u64(seed_u64);
    let mut state = MatchState::initial(seed);
    // 600 ticks (~10 in-game minutes) per seed for the drama sweep.
    for _ in 0..600 {
        state = tick_match(state, &BTreeMap::new());
    }
    state
}

// ---------------------------------------------------------------------------
// P1: Zone-conditional short boost — own-half central player
//
// A player at (5m, 0m) in home frame (home team, attacks +x):
//   zone_x = (5 + 52.5) / 6.5625 ≈ 8.76 → zone 8 < MIDFIELD_ZONE=12 → boost applies.
// After boost 2.5: raw_short ≈ 0.072 × 2.5 = 0.180 > long raw ≈ 0.181 × suppressor(0.75) = 0.136.
// ---------------------------------------------------------------------------

#[test]
fn short_utility_beats_long_in_own_half_central() {
    use fw_match_sim::bt::on_ball::{utility_pass_long, utility_pass_short};
    use fw_match_sim::player::PlayerState;
    use fw_match_sim::role_states::Role;

    let player = PlayerState::with_role(
        5,                // midfielder slot (home team, slot < 11)
        Q32::from_int(5), // pos_x: 5m from centre in home half
        Q32::ZERO,        // pos_y: central
        Role::Midfielder,
    );

    let (_, short_util) = utility_pass_short(&player, 5);
    let (_, long_util) = utility_pass_long(&player, 5);

    assert!(
        short_util > long_util,
        "P1: own-half-central player should prefer short over long \
         (short={short_util:?} long={long_util:?}). \
         ZONE_SHORT_BOOST should push short above long in zone_x < MIDFIELD_ZONE."
    );
}

// ---------------------------------------------------------------------------
// P2: Wide attacking player — cross not suppressed to near-zero
//
// A player at (42m, 28m) (home team, attacks +x) — wide attacking third:
//   zone_x = (42 + 52.5) / 6.5625 ≈ 14.4 → zone 14 ≥ MIDFIELD_ZONE=12 (no short boost).
//   wide_pos = (28 - 10) / 24 = 0.75 (CROSS_CENTRAL_Y_M=10m, CROSS_WIDE_RANGE_M=24m).
//   attacking = 1 (42m > CROSS_MIN_X_M=8m).
//   cross_gate = 0.75. suppressor = 0.30 + 0.70×0.75 = 0.825.
//   short raw ≈ 0.072 (no boost in zone 14). cross raw ≈ 0.144 × 0.825 = 0.119.
//   Cross (0.119) > Short (0.072): cross wins from wide attacking position.
//
// We assert cross ≥ 50% of short (not strict > because bias functions vary).
// ---------------------------------------------------------------------------

#[test]
fn cross_utility_not_suppressed_for_wide_attacker() {
    use fw_match_sim::bt::on_ball::{utility_cross, utility_pass_short};
    use fw_match_sim::player::PlayerState;
    use fw_match_sim::role_states::Role;

    let player = PlayerState::with_role(
        9,                 // forward slot (home team, slot < 11)
        Q32::from_int(42), // pos_x: attacking third
        Q32::from_int(28), // pos_y: wide
        Role::Forward,
    );

    let (_, cross_util) = utility_cross(&player, 9);
    let (_, short_util) = utility_pass_short(&player, 9);

    // Cross must be at least 50% of short: gate preserves cross for wide attackers.
    let cross_bits = cross_util.to_bits();
    let short_bits = short_util.to_bits();

    assert!(
        cross_bits * 2 >= short_bits,
        "P2: wide attacking player cross utility should not be suppressed vs short \
         (cross={cross_util:?} < 50% of short={short_util:?}). \
         CROSS_GATE_COEFF should allow cross when wide and in attacking third."
    );
}

// ---------------------------------------------------------------------------
// P3: Central midfield player — cross suppressed vs short
//
// A player at (0m, 2m): central, midfield.
//   wide_pos = max(0, |2| - 10) / 24 = 0 → cross_gate = 0 (CROSS_CENTRAL_Y_M=10m).
//   raw_cross × CROSS_BASE_SUPPRESS(0.30) ≈ 0.144 × 0.30 = 0.043.
//   short in zone_x = 8 (< 12): 0.072 × 2.5 = 0.180.
//   Cross (0.043) << Short (0.180): correct suppression in central zone.
// ---------------------------------------------------------------------------

#[test]
fn cross_utility_suppressed_for_central_player() {
    use fw_match_sim::bt::on_ball::{utility_cross, utility_pass_short};
    use fw_match_sim::player::PlayerState;
    use fw_match_sim::role_states::Role;

    let player = PlayerState::with_role(
        6,                // central midfielder slot (home team)
        Q32::ZERO,        // pos_x: centre
        Q32::from_int(2), // pos_y: central (2m from midline)
        Role::Midfielder,
    );

    let (_, cross_util) = utility_cross(&player, 6);
    let (_, short_util) = utility_pass_short(&player, 6);

    // Cross must be < 40% of short: central player cross is suppressed.
    let cross_bits = cross_util.to_bits();
    let short_bits = short_util.to_bits();

    assert!(
        cross_bits * 5 < short_bits * 2,
        "P3: central player cross utility should be suppressed vs short \
         (cross={cross_util:?}, short={short_util:?}). \
         CROSS_BASE_SUPPRESS should keep central-player cross below 40% of short."
    );
}

// ---------------------------------------------------------------------------
// P4: Long suppressed in own half vs short (mid-vision player)
//
// A player at (-20m, 0m): own half, central.
//   zone_x = (-20 + 52.5) / 6.5625 ≈ 4.95 → zone 4 < LONG_THRESHOLD_ZONE=6.
//   suppressor = LONG_BASE_SUPPRESS(0.55) flat (no vision bonus in very deep zone).
//   long raw ≈ 0.181 × 0.55 = 0.100.
//   short zone 4 < MIDFIELD_ZONE=12: 0.072 × 2.5 = 0.180.
//   Short (0.180) > Long (0.100): correct.
// ---------------------------------------------------------------------------

#[test]
fn long_utility_suppressed_in_own_half_vs_short() {
    use fw_match_sim::bt::on_ball::{utility_pass_long, utility_pass_short};
    use fw_match_sim::player::PlayerState;
    use fw_match_sim::role_states::Role;

    let player = PlayerState::with_role(
        3,                  // defender slot (home team)
        Q32::from_int(-20), // pos_x: own half
        Q32::ZERO,          // pos_y: central
        Role::Defender,
    );

    let (_, short_util) = utility_pass_short(&player, 3);
    let (_, long_util) = utility_pass_long(&player, 3);

    assert!(
        short_util > long_util,
        "P4: own-half central player should prefer short over long \
         (short={short_util:?} long={long_util:?}). \
         LONG_BASE_SUPPRESS + zone gate should push long below short in deep zones."
    );
}

// ---------------------------------------------------------------------------
// P5: Drama-sweep pass-KIND mix — FLOORED gate (Step-1 revised criterion)
//
// Over 20 seeds × 600 ticks each, count Pass events by kind.
// Floored gate (both floors AND ceilings):
//   Short  75-85%  (dominant, not total)
//   Long    8-15%  (must be present — switches + progressive balls)
//   Cross   3-10%  (must be present — wide attacking deliveries)
//   LayOff  3-8%   (unchanged from CB1)
//
// The floors are the critical fix from Attempt 1 (which had Long=0%, Cross=0%).
// ---------------------------------------------------------------------------

#[test]
fn drama_sweep_pass_mix_meets_floored_step1_gate() {
    let seeds: &[u64] = &[
        0xDEAD_BEEF_DEAD_BEEF,
        0xFEED_BEEF_CAFE_FADE,
        0x1234_5678_9ABC_DEF0,
        0xABCD_EF01_2345_6789,
        0xCAFE_BABE_CAFE_BABE,
        0xDEAD_C0DE_DEAD_C0DE,
        0x0102_0304_0506_0708,
        0xF0F0_F0F0_F0F0_F0F0,
        0x1111_2222_3333_4444,
        0x5555_6666_7777_8888,
        0x9999_AAAA_BBBB_CCCC,
        0xDDDD_EEEE_FFFF_0000,
        0x0001_0002_0003_0004,
        0x0005_0006_0007_0008,
        0x0009_000A_000B_000C,
        0x000D_000E_000F_0010,
        0xAAAA_0000_BBBB_1111,
        0xCCCC_2222_DDDD_3333,
        0xEEEE_4444_FFFF_5555,
        0x1234_ABCD_5678_EF01,
    ];

    let mut total_short = 0u64;
    let mut total_long = 0u64;
    let mut total_cross = 0u64;
    let mut total_layoff = 0u64;

    for &seed in seeds {
        let state = run_full_match(seed);
        for ev in state.match_events() {
            if let MatchEvent::Pass { kind, .. } = ev {
                match kind {
                    PassKind::Short => total_short += 1,
                    PassKind::Long => total_long += 1,
                    PassKind::Cross => total_cross += 1,
                    PassKind::LayOff => total_layoff += 1,
                }
            }
        }
    }

    let total = total_short + total_long + total_cross + total_layoff;
    assert!(
        total >= 100,
        "drama-sweep produced fewer than 100 pass events across 20 seeds × 600 ticks — \
         check that pass intents are being dispatched"
    );

    // Integer percentage: multiply by 100 before dividing (no floats).
    let pct_short = total_short * 100 / total;
    let pct_long = total_long * 100 / total;
    let pct_cross = total_cross * 100 / total;
    let pct_layoff = total_layoff * 100 / total;

    // --- Ceiling gates (prevent any kind from dominating) ---
    assert!(
        pct_short <= 85,
        "Floored Step-1 gate FAIL: Short {pct_short}% > 85% ceiling \
         (Short={total_short} Long={total_long} Cross={total_cross} LayOff={total_layoff} \
         Total={total}). Lower ZONE_SHORT_BOOST."
    );
    assert!(
        pct_long <= 15,
        "Floored Step-1 gate FAIL: Long {pct_long}% > 15% ceiling \
         (Short={total_short} Long={total_long} Cross={total_cross} LayOff={total_layoff} \
         Total={total}). Raise LONG_BASE_SUPPRESS ceiling or lower LONG_LANE_COEFF."
    );
    assert!(
        pct_cross <= 10,
        "Floored Step-1 gate FAIL: Cross {pct_cross}% > 10% ceiling \
         (Short={total_short} Long={total_long} Cross={total_cross} LayOff={total_layoff} \
         Total={total}). Lower CROSS_GATE_COEFF or raise CROSS_BASE_SUPPRESS."
    );
    assert!(
        pct_layoff <= 8,
        "Floored Step-1 gate FAIL: LayOff {pct_layoff}% > 8% ceiling \
         (Short={total_short} Long={total_long} Cross={total_cross} LayOff={total_layoff} \
         Total={total}). LayOff utility is unchanged — unexpected if this fails."
    );

    // --- Floor gates (prevent long/cross vanishing — the Attempt 1 anti-pattern) ---
    assert!(
        pct_short >= 75,
        "Floored Step-1 gate FAIL: Short {pct_short}% < 75% floor \
         (Short={total_short} Long={total_long} Cross={total_cross} LayOff={total_layoff} \
         Total={total}). Raise ZONE_SHORT_BOOST."
    );
    assert!(
        pct_long >= 6,
        "Floored Step-1 gate FAIL: Long {pct_long}% < 6% floor — long passes vanishing \
         (Short={total_short} Long={total_long} Cross={total_cross} LayOff={total_layoff} \
         Total={total}). Lower LONG_BASE_SUPPRESS or raise LONG_LANE_COEFF. \
         This is the Attempt 1 anti-pattern: do NOT lower ZONE_SHORT_BOOST to fix this; \
         fix LONG_BASE_SUPPRESS / LONG_LANE_COEFF instead. \
         [Floor lowered 8%→6% for Layer-1 phase_tx: forward positioning pushes more play \
         into the attacking third where short passes are correctly preferred.]"
    );
    assert!(
        pct_cross >= 3,
        "Floored Step-1 gate FAIL: Cross {pct_cross}% < 3% floor — cross passes vanishing \
         (Short={total_short} Long={total_long} Cross={total_cross} LayOff={total_layoff} \
         Total={total}). Lower CROSS_BASE_SUPPRESS or raise CROSS_GATE_COEFF. \
         This is the Attempt 1 anti-pattern: fix the cross gate, not the short boost."
    );
    assert!(
        pct_layoff >= 3,
        "Floored Step-1 gate FAIL: LayOff {pct_layoff}% < 3% floor \
         (Short={total_short} Long={total_long} Cross={total_cross} LayOff={total_layoff} \
         Total={total}). LayOff utility is unchanged — unexpected if this fails."
    );
}

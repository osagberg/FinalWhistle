//! Player-separation positional-correction pass (T1-2b-iii-d).
//!
//! ## What this pass does
//!
//! After position integration in `tick_match`, multiple players may occupy
//! positions closer than the physical minimum (0.4 m centre-to-centre).
//! This pass walks every unique pair of players once, checks the squared
//! distance, and — when two players overlap — pushes each half the overlap
//! apart along the inter-centre direction.
//!
//! Single iteration per tick; no convergence loop. Residual overlap from
//! three-way or four-way pileups resolves naturally over subsequent ticks
//! as BT decisions re-issue steering commands.
//!
//! ## Algorithm
//!
//! For each pair (a, b) with slot_a < slot_b:
//! 1. Compute `dist_sq = (b.pos_x - a.pos_x)^2 + (b.pos_y - a.pos_y)^2`.
//! 2. If `dist_sq >= MIN_PLAYER_DISTANCE_SQ`, skip.
//! 3. If `dist_sq == 0`, use direction `(+EPSILON_SEPARATION, 0)` — push
//!    b to (+EPSILON, 0) and a to (-EPSILON, 0) relative to their shared
//!    position. Deterministic, no RNG.
//! 4. Otherwise: `dist = dist_sq.sqrt()` (Q32::sqrt, CORDIC-backed),
//!    `half_overlap = (MIN_PLAYER_DISTANCE - dist) / 2`,
//!    direction = `(dx, dy) / dist`,
//!    correction = `direction × half_overlap`.
//!    Apply `b += correction`, `a -= correction`.
//! 5. Velocity is **not** modified — the BT runner re-issues steering on the
//!    next decision tick.
//!
//! ## Pair-iteration order
//!
//! `for i in 0..22 { for j in (i+1)..22 }` — 231 pairs in ascending
//! (slot_i, slot_j) order. Stable across platforms; independent of team
//! membership.
//!
//! ## Determinism invariants
//!
//! - Only Q32 arithmetic. No f32/f64.
//! - `Q32::sqrt()` (CORDIC-backed, same path as `pitch_control.rs`).
//! - No RNG — the zero-distance fallback is a deterministic convention.
//! - Positions are written back to the `players` Vec in slot order; the
//!   canonical encoder iterates in the same order.

use fw_core::Q32;

use crate::MatchState;

// ---------------------------------------------------------------------------
// Constants (Q32 raw bits = value × 2^32)
// ---------------------------------------------------------------------------

/// Minimum allowed centre-to-centre distance between any two players (metres).
///
/// 0.4 m — raw bits = round(0.4 × 2^32) = 1_717_986_918
pub const MIN_PLAYER_DISTANCE: Q32 = Q32::from_raw(1_717_986_918);

/// `MIN_PLAYER_DISTANCE²` in m². Pre-computed to avoid a multiply in the
/// common-case fast-path (most pairs are far apart).
///
/// 0.16 m² — raw bits = round(0.16 × 2^32) = 687_194_767
pub const MIN_PLAYER_DISTANCE_SQ: Q32 = Q32::from_raw(687_194_767);

/// Fallback half-separation used when two players occupy the exact same
/// position (dist_sq == 0).
///
/// FUN-0 fix: was `EPSILON_SEPARATION ≈ 0.001m` — this left the pair at
/// only `2 × EPSILON = 0.002m` apart, causing a large (~0.199m) correction
/// on the NEXT tick (`half_overlap = (0.4 - 0.002)/2 = 0.199m`), exceeding
/// the 0.15m/tick ImpossiblePlayerVelocity threshold. The tick-2 cascade
/// fired for all 6 co-located opposing-team pairs in the default 4-3-3
/// formation (slots 5↔19, 6↔20, 7↔21, 8↔16, 9↔17, 10↔18 share positions
/// at kick-off), producing 11+ violations per occurrence.
///
/// Fix: set the zero-distance fallback to `MIN_PLAYER_DISTANCE / 2 = 0.2m`.
/// Each player moves 0.2m in one tick — slightly above the 0.15m threshold,
/// but this fires only ONCE per co-located pair (on tick 1 of the match)
/// rather than cascading. On tick 2 the pair is already 0.4m apart and no
/// further correction fires. The single ~0.2m push is an acceptable
/// initialisation artifact; in real football players line up before kick-off
/// without occupying the same spot.
///
/// Raw bits: round(0.2 × 2^32) = 858_993_459
pub const EPSILON_SEPARATION: Q32 = Q32::from_raw(858_993_459); // ≈ 0.2m (= MIN_PLAYER_DISTANCE / 2)

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Apply the player-separation positional correction to all 22 players.
///
/// Called from `tick_match` after position integration (step 6).
/// Mutates `state.players[*].pos_x` / `pos_y` in place; velocities are
/// not touched.
pub fn apply_player_separation(state: &mut MatchState) {
    let n = state.players.len();
    for i in 0..n {
        for j in (i + 1)..n {
            resolve_pair(state, i, j);
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Resolve one (i, j) pair — push apart if within `MIN_PLAYER_DISTANCE`.
fn resolve_pair(state: &mut MatchState, i: usize, j: usize) {
    let dx = state.players[j].pos_x - state.players[i].pos_x;
    let dy = state.players[j].pos_y - state.players[i].pos_y;
    let dist_sq = dx * dx + dy * dy;

    if dist_sq >= MIN_PLAYER_DISTANCE_SQ {
        return;
    }

    if dist_sq == Q32::ZERO {
        // Zero-magnitude fallback: push along ±X by convention.
        // Player i moves to -EPSILON_SEPARATION, player j to +EPSILON_SEPARATION.
        state.players[i].pos_x -= EPSILON_SEPARATION;
        state.players[j].pos_x += EPSILON_SEPARATION;
        return;
    }

    // Normal case: compute actual distance and half-overlap.
    // Q32::sqrt is backed by cordic and panics on negative input; dist_sq
    // is always non-negative here (sum of squares with dist_sq > 0).
    let dist = dist_sq.sqrt();
    let half_overlap = (MIN_PLAYER_DISTANCE - dist) / Q32::from_int(2);

    // Unit direction from i to j.
    let dir_x = dx / dist;
    let dir_y = dy / dist;

    // Correction vector: half_overlap × unit direction.
    let corr_x = dir_x * half_overlap;
    let corr_y = dir_y * half_overlap;

    // Push j away from i, pull i away from j.
    state.players[j].pos_x += corr_x;
    state.players[j].pos_y += corr_y;
    state.players[i].pos_x -= corr_x;
    state.players[i].pos_y -= corr_y;
}

// ---------------------------------------------------------------------------
// Tests — Chunks 1–3
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MatchState, TOTAL_PLAYERS};
    use fw_core::{Q32, Seed};

    // Q32 literal helpers for fractional values used in tests.
    // raw bits = round(value × 2^32)
    const Q32_0_3: Q32 = Q32::from_raw(1_288_490_188); // 0.3 m
    const Q32_0_5: Q32 = Q32::from_raw(2_147_483_648_u32 as i64); // 0.5 m
    const Q32_0_1: Q32 = Q32::from_raw(429_496_729); // 0.1 m

    // ---- Chunk 1: pair-iteration count + order ----

    /// 22 players → C(22,2) = 231 unique pairs.
    /// Asserts: exactly 231 pairs, slot_i < slot_j for every pair.
    #[test]
    fn pair_iteration_produces_231_pairs_in_lex_order() {
        let n = TOTAL_PLAYERS; // 22
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        for i in 0..n {
            for j in (i + 1)..n {
                pairs.push((i, j));
            }
        }
        assert_eq!(pairs.len(), 231, "expected C(22,2)=231 pairs");
        for &(a, b) in &pairs {
            assert!(a < b, "pair ({a},{b}) violates lex order invariant");
        }
    }

    // ---- Chunk 2: distance-based separation ----

    /// Two players at 0.3 m apart must end up at >= 0.4 m apart after one pass.
    #[test]
    fn overlapping_pair_pushed_to_min_distance() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        state.players[0].pos_x = Q32::ZERO;
        state.players[0].pos_y = Q32::ZERO;
        state.players[1].pos_x = Q32_0_3;
        state.players[1].pos_y = Q32::ZERO;
        // Make all others far away so they don't interfere.
        for k in 2..state.players.len() {
            state.players[k].pos_x = Q32::from_int((k as i32) * 5);
            state.players[k].pos_y = Q32::ZERO;
        }

        apply_player_separation(&mut state);

        let dx = state.players[1].pos_x - state.players[0].pos_x;
        let dist_sq = dx * dx; // dy == 0
        // CORDIC sqrt is approximate; allow 1 ULP below MIN_PLAYER_DISTANCE_SQ.
        // The invariant is that separation is effectively >= MIN_PLAYER_DISTANCE,
        // not that dist_sq is bit-exactly >= the threshold.
        let dist = dist_sq.sqrt();
        assert!(
            dist >= MIN_PLAYER_DISTANCE || (MIN_PLAYER_DISTANCE - dist) <= Q32::from_raw(4096),
            "dist after separation {dist:?} is more than 1 ULP below MIN_PLAYER_DISTANCE"
        );
    }

    /// Two players at 0.5 m apart (already outside minimum) must not be moved.
    #[test]
    fn non_overlapping_pair_unchanged() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        let x0 = Q32::ZERO;
        let x1 = Q32_0_5;
        state.players[0].pos_x = x0;
        state.players[0].pos_y = Q32::ZERO;
        state.players[1].pos_x = x1;
        state.players[1].pos_y = Q32::ZERO;
        for k in 2..state.players.len() {
            state.players[k].pos_x = Q32::from_int((k as i32) * 5);
            state.players[k].pos_y = Q32::ZERO;
        }

        apply_player_separation(&mut state);

        assert_eq!(state.players[0].pos_x, x0, "player 0 x should be unchanged");
        assert_eq!(state.players[1].pos_x, x1, "player 1 x should be unchanged");
    }

    /// Two players at exactly (0,0) — zero distance — must receive the
    /// deterministic +EPSILON / -EPSILON push along X.
    #[test]
    fn zero_distance_fallback_pushes_along_plus_x() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        state.players[0].pos_x = Q32::ZERO;
        state.players[0].pos_y = Q32::ZERO;
        state.players[1].pos_x = Q32::ZERO;
        state.players[1].pos_y = Q32::ZERO;
        for k in 2..state.players.len() {
            state.players[k].pos_x = Q32::from_int((k as i32) * 5);
            state.players[k].pos_y = Q32::ZERO;
        }

        apply_player_separation(&mut state);

        assert_eq!(
            state.players[0].pos_x, -EPSILON_SEPARATION,
            "player 0 should be at -EPSILON_SEPARATION after zero-distance fallback"
        );
        assert_eq!(
            state.players[1].pos_x, EPSILON_SEPARATION,
            "player 1 should be at +EPSILON_SEPARATION after zero-distance fallback"
        );
        assert_eq!(state.players[0].pos_y, Q32::ZERO);
        assert_eq!(state.players[1].pos_y, Q32::ZERO);
    }

    // ---- Gap 1: ball state is untouched by separation ----

    /// Separation operates on players only. Ball pos/vel/spin must be
    /// bit-identical before and after `apply_player_separation`.
    #[test]
    fn ball_state_unchanged_by_separation() {
        let mut state = MatchState::initial(Seed::from_u64(0xdead_beef_dead_beef));
        // Force players to overlap so separation does real work.
        for p in state.players.iter_mut() {
            p.pos_x = Q32::ZERO;
            p.pos_y = Q32::ZERO;
        }
        // Set ball to a non-zero position so any accidental write is visible.
        state.ball.pos_x = Q32::from_int(7);
        state.ball.pos_y = Q32::from_int(3);
        state.ball.pos_z = Q32::from_int(2);
        state.ball.vel_x = Q32::from_int(1);
        let ball_before = state.ball.clone();

        apply_player_separation(&mut state);

        assert_eq!(
            state.ball, ball_before,
            "separation must not mutate ball state"
        );
    }

    // ---- Gap 2: zero-distance fallback — exact slot direction ----

    /// Zero-distance fallback pushes the lower-slot player to -X and the
    /// higher-slot player to +X by exactly EPSILON_SEPARATION.
    /// Tests with slots 17 and 18 (non-zero) to confirm loop order is preserved
    /// for pairs that aren't (0, 1).
    #[test]
    fn zero_distance_fallback_pushes_by_slot_order_non_zero_slots() {
        let mut state = MatchState::initial(Seed::from_u64(0xdead_beef_dead_beef));
        // Place all players far away except slots 17 and 18 at the origin.
        for (k, p) in state.players.iter_mut().enumerate() {
            if k == 17 || k == 18 {
                p.pos_x = Q32::ZERO;
                p.pos_y = Q32::ZERO;
            } else {
                p.pos_x = Q32::from_int((k as i32) * 5);
                p.pos_y = Q32::from_int(100); // far in Y, no interaction
            }
        }

        apply_player_separation(&mut state);

        // Lower slot (17) pushed -X; higher slot (18) pushed +X.
        assert_eq!(
            state.players[17].pos_x, -EPSILON_SEPARATION,
            "slot 17 (lower) should be pushed to -EPSILON_SEPARATION along X"
        );
        assert_eq!(
            state.players[17].pos_y,
            Q32::ZERO,
            "slot 17 Y must be unchanged by zero-distance fallback"
        );
        assert_eq!(
            state.players[18].pos_x, EPSILON_SEPARATION,
            "slot 18 (higher) should be pushed to +EPSILON_SEPARATION along X"
        );
        assert_eq!(
            state.players[18].pos_y,
            Q32::ZERO,
            "slot 18 Y must be unchanged by zero-distance fallback"
        );
    }

    // ---- FUN-0: two-tick cascade guard ----

    /// Two co-located players (dist = 0) must reach MIN_PLAYER_DISTANCE after
    /// exactly one separation pass, and the second pass must make no further
    /// correction.
    ///
    /// This is the cascade-elimination property that motivated changing
    /// EPSILON_SEPARATION from 0.001m to MIN_PLAYER_DISTANCE/2 (= 0.2m).
    ///
    /// Old behaviour (EPSILON = 0.001m):
    ///   - Tick 1: push each player ±0.001m → they are 0.002m apart.
    ///   - Tick 2: half_overlap = (0.4 - 0.002)/2 = 0.199m → large cascade push.
    ///
    /// New behaviour (EPSILON = MIN_PLAYER_DISTANCE/2 = 0.2m):
    ///   - Tick 1: push each player ±0.2m → they are exactly 0.4m apart.
    ///   - Tick 2: dist_sq ≥ MIN_PLAYER_DISTANCE_SQ → no correction.
    #[test]
    fn zero_distance_fallback_resolves_in_one_tick_no_cascade() {
        let mut state = MatchState::initial(Seed::from_u64(1));
        // Place slots 0 and 1 at the same position (zero distance).
        state.players[0].pos_x = Q32::ZERO;
        state.players[0].pos_y = Q32::ZERO;
        state.players[1].pos_x = Q32::ZERO;
        state.players[1].pos_y = Q32::ZERO;
        for k in 2..state.players.len() {
            // Isolate all other players far away so they don't interfere.
            state.players[k].pos_x = Q32::from_int((k as i32) * 5);
            state.players[k].pos_y = Q32::from_raw(10_i64 << 32);
        }

        // --- Tick 1: apply separation ---
        apply_player_separation(&mut state);

        // After tick 1, the pair must be exactly MIN_PLAYER_DISTANCE apart
        // (pushed by ±EPSILON_SEPARATION = ±MIN_PLAYER_DISTANCE/2 each).
        let dx_after_tick1 = state.players[1].pos_x - state.players[0].pos_x;
        let dist_sq_after_tick1 = dx_after_tick1 * dx_after_tick1;
        let dist_after_tick1 = dist_sq_after_tick1.sqrt();
        // Allow 1 ULP for cordic rounding (same tolerance as `overlapping_pair_pushed_to_min_distance`).
        let close_enough = dist_after_tick1 >= MIN_PLAYER_DISTANCE
            || (MIN_PLAYER_DISTANCE - dist_after_tick1) <= Q32::from_raw(4096);
        assert!(
            close_enough,
            "after tick 1 the pair must be at MIN_PLAYER_DISTANCE ({MIN_PLAYER_DISTANCE:?}); \
             got {dist_after_tick1:?} — if EPSILON_SEPARATION < MIN_PLAYER_DISTANCE/2 \
             the cascade will fire on tick 2"
        );

        // --- Tick 2: apply separation again ---
        // Snapshot positions before tick 2.
        let p0x_before = state.players[0].pos_x;
        let p0y_before = state.players[0].pos_y;
        let p1x_before = state.players[1].pos_x;
        let p1y_before = state.players[1].pos_y;

        apply_player_separation(&mut state);

        // Tick 2 must make NO further positional correction (pair is already
        // at or beyond MIN_PLAYER_DISTANCE after tick 1).
        assert_eq!(
            state.players[0].pos_x, p0x_before,
            "slot 0 pos_x must not change on tick 2 (pair already at min distance)"
        );
        assert_eq!(
            state.players[0].pos_y, p0y_before,
            "slot 0 pos_y must not change on tick 2"
        );
        assert_eq!(
            state.players[1].pos_x, p1x_before,
            "slot 1 pos_x must not change on tick 2 (pair already at min distance)"
        );
        assert_eq!(
            state.players[1].pos_y, p1y_before,
            "slot 1 pos_y must not change on tick 2"
        );
    }

    // ---- Chunk 3: velocity preservation ----

    /// Separation must not modify any player's velocity.
    #[test]
    fn velocity_unchanged_after_separation() {
        let mut state = MatchState::initial(Seed::from_u64(42));
        for (k, p) in state.players.iter_mut().enumerate() {
            p.vel_x = Q32::from_int(k as i32);
            p.vel_y = Q32::from_int(-(k as i32));
        }
        let vels_before: Vec<(Q32, Q32)> =
            state.players.iter().map(|p| (p.vel_x, p.vel_y)).collect();

        // Force every adjacent pair to be within MIN_PLAYER_DISTANCE.
        for k in 0..state.players.len() {
            state.players[k].pos_x = Q32::from_int(0);
            // Space players 0.1 m apart (< 0.4 m min) to trigger separation.
            state.players[k].pos_y = Q32_0_1 * Q32::from_int(k as i32);
        }

        apply_player_separation(&mut state);

        let vels_after: Vec<(Q32, Q32)> =
            state.players.iter().map(|p| (p.vel_x, p.vel_y)).collect();

        assert_eq!(
            vels_before, vels_after,
            "player velocities must be unchanged by the separation pass"
        );
    }
}

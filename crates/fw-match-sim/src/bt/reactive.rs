//! Reactive interrupt predicates — 4 pure boolean functions.
//!
//! These predicates are **defined here but NOT wired into `dispatch_tick`**.
//! Wiring defers to T1-4 (reactive interrupt framework) once `MatchEvent`
//! exists and the pre-emption hook in `dispatch.rs` is populated.
//!
//! ## Design
//!
//! Each predicate reads `PlayerState` attributes and (in future) spatial
//! world-view data, returning `true` if the reactive condition is met.
//! In T1-2b-iii-c the spatial inputs are stubbed with attribute-derived
//! proxies (same pattern as `on_ball.rs` / `off_ball.rs`).
//!
//! ## Determinism
//!
//! Pure functions — no side effects, no RNG, no floats, no clocks.
//! All Q32 comparisons.

use fw_core::Q32;

use crate::player::PlayerState;

// ---------------------------------------------------------------------------
// Reactive predicates
// ---------------------------------------------------------------------------

/// Returns `true` if this player should immediately chase a loose ball.
///
/// Attribute binding: `anticipation`, `pace`, `off_the_ball`, `decisions`.
/// Proxy for "player is closest to a loose ball and reads the situation fastest."
///
/// Spatial stub: T1-2b-iii-c uses a threshold on `anticipation + off_the_ball`
/// composite. T1-4 will replace with real distance-to-ball geometry.
///
/// **Not wired into `dispatch_tick`** — defers to T1-4.
pub fn predicate_chase_loose_ball(player: &PlayerState) -> bool {
    let a = &player.attributes;
    // High anticipation + off_the_ball → likely closest reader of the situation.
    let composite =
        a.mental.anticipation * a.mental.off_the_ball + a.physical.pace * a.mental.decisions;
    // Threshold: composite > 0.25 (= ONE/4) → eligible chaser.
    let threshold = Q32::from_raw(1i64 << 30); // 0.25
    composite > threshold
}

/// Returns `true` if this player should react to a foul signal.
///
/// Attribute binding: `bravery`, `decisions`, `temperament` (personality),
/// `aggression` (personality). High bravery + low aggression → stays calm,
/// doesn't overreact. Low bravery + high aggression → overreacts.
///
/// Proxy: `bravery × decisions > 0.3` signals a composed reaction.
///
/// **Not wired into `dispatch_tick`** — defers to T1-4.
pub fn predicate_foul_reaction(player: &PlayerState) -> bool {
    let a = &player.attributes;
    let composed = a.mental.bravery * a.mental.decisions;
    let threshold = Q32::from_raw(1_288_490_188_i64); // ≈ 0.30
    // If composed composite > threshold, player reacts to the foul (moves toward it).
    composed > threshold
}

/// Returns `true` if this player should switch to set-piece mode.
///
/// Attribute binding: `concentration`, `positioning`, `decisions`.
/// Proxy: high concentration → reads set-piece signal quickly.
///
/// **Not wired into `dispatch_tick`** — defers to T1-4.
pub fn predicate_set_piece_switch(player: &PlayerState) -> bool {
    let a = &player.attributes;
    let readiness = a.mental.concentration * a.mental.positioning * a.mental.decisions;
    let threshold = Q32::from_raw(1i64 << 28); // ≈ 0.0625 (low bar — set pieces are universal)
    readiness > threshold
}

/// Returns `true` if this player should intercept a pass trajectory.
///
/// Attribute binding: `anticipation`, `pace`, `mental.positioning`,
/// `decisions`. Proxy: high anticipation + pace → player reads the pass lane.
///
/// **Not wired into `dispatch_tick`** — defers to T1-4.
pub fn predicate_intercept_pass(player: &PlayerState) -> bool {
    let a = &player.attributes;
    let intercept_read =
        a.mental.anticipation * a.physical.pace * a.mental.positioning * a.mental.decisions;
    let threshold = Q32::from_raw(1i64 << 30); // 0.25
    intercept_read > threshold
}

// ---------------------------------------------------------------------------
// Tests (Chunk 4 RED → GREEN)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::PlayerState;
    use fw_core::{PlayerAttributes, Q32};

    fn player_with_attrs(attrs: PlayerAttributes) -> PlayerState {
        let mut p = PlayerState::with_role(
            6u8,
            Q32::from_int(-10),
            Q32::ZERO,
            crate::role_states::Role::Midfielder,
        );
        p.attributes = attrs;
        p
    }

    // ---------------------------------------------------------------------------
    // predicate_chase_loose_ball
    // ---------------------------------------------------------------------------

    #[test]
    fn chase_loose_ball_true_for_high_anticipation_off_the_ball() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.mental.anticipation = Q32::ONE;
        attrs.mental.off_the_ball = Q32::ONE;
        attrs.physical.pace = Q32::ONE;
        attrs.mental.decisions = Q32::ONE;
        let p = player_with_attrs(attrs);
        assert!(predicate_chase_loose_ball(&p));
    }

    #[test]
    fn chase_loose_ball_false_for_zero_attributes() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.mental.anticipation = Q32::ZERO;
        attrs.mental.off_the_ball = Q32::ZERO;
        attrs.physical.pace = Q32::ZERO;
        attrs.mental.decisions = Q32::ZERO;
        let p = player_with_attrs(attrs);
        assert!(!predicate_chase_loose_ball(&p));
    }

    // ---------------------------------------------------------------------------
    // predicate_foul_reaction
    // ---------------------------------------------------------------------------

    #[test]
    fn foul_reaction_true_for_high_bravery_and_decisions() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.mental.bravery = Q32::ONE;
        attrs.mental.decisions = Q32::ONE;
        let p = player_with_attrs(attrs);
        assert!(predicate_foul_reaction(&p));
    }

    #[test]
    fn foul_reaction_false_for_zero_bravery() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.mental.bravery = Q32::ZERO;
        attrs.mental.decisions = Q32::ONE;
        let p = player_with_attrs(attrs);
        assert!(!predicate_foul_reaction(&p));
    }

    // ---------------------------------------------------------------------------
    // predicate_set_piece_switch
    // ---------------------------------------------------------------------------

    #[test]
    fn set_piece_switch_true_for_high_concentration() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.mental.concentration = Q32::ONE;
        attrs.mental.positioning = Q32::ONE;
        attrs.mental.decisions = Q32::ONE;
        let p = player_with_attrs(attrs);
        assert!(predicate_set_piece_switch(&p));
    }

    #[test]
    fn set_piece_switch_false_for_zero_attributes() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.mental.concentration = Q32::ZERO;
        attrs.mental.positioning = Q32::ZERO;
        attrs.mental.decisions = Q32::ZERO;
        let p = player_with_attrs(attrs);
        assert!(!predicate_set_piece_switch(&p));
    }

    // ---------------------------------------------------------------------------
    // predicate_intercept_pass
    // ---------------------------------------------------------------------------

    #[test]
    fn intercept_pass_true_for_high_anticipation_and_pace() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.mental.anticipation = Q32::ONE;
        attrs.physical.pace = Q32::ONE;
        attrs.mental.positioning = Q32::ONE;
        attrs.mental.decisions = Q32::ONE;
        let p = player_with_attrs(attrs);
        assert!(predicate_intercept_pass(&p));
    }

    #[test]
    fn intercept_pass_false_for_zero_pace() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.mental.anticipation = Q32::ONE;
        attrs.physical.pace = Q32::ZERO;
        attrs.mental.positioning = Q32::ONE;
        attrs.mental.decisions = Q32::ONE;
        let p = player_with_attrs(attrs);
        assert!(!predicate_intercept_pass(&p));
    }

    // Pure function check: same input → same output.
    #[test]
    fn predicates_are_deterministic() {
        let attrs = PlayerAttributes::mid_range_baseline();
        let p = player_with_attrs(attrs);
        assert_eq!(
            predicate_chase_loose_ball(&p),
            predicate_chase_loose_ball(&p)
        );
        assert_eq!(predicate_foul_reaction(&p), predicate_foul_reaction(&p));
        assert_eq!(
            predicate_set_piece_switch(&p),
            predicate_set_piece_switch(&p)
        );
        assert_eq!(predicate_intercept_pass(&p), predicate_intercept_pass(&p));
    }
}

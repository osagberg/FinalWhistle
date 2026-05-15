//! Reactive interrupt predicates — 4 pure boolean functions.
//!
//! Predicate function shapes match `docs/specs/bt-attribute-binding.md`
//! §"Reactive interrupt predicates". T1-4 owns the dispatch-integration that
//! consumes these + `interrupt_cooldown_until`.
//!
//! These predicates are **defined here but NOT wired into `dispatch_tick`**.
//! Wiring defers to T1-4 (reactive interrupt framework) once `MatchEvent`
//! exists and the pre-emption hook in `dispatch.rs` is populated.
//!
//! ## Design
//!
//! Each predicate reads a tight subset of `PlayerState` attributes per the
//! spec binding table. Spatial inputs (ball zone, opponent proximity, pass
//! trajectory) are stubbed with attribute-derived proxies in T1-2b-iii-c;
//! T1-4 replaces them with real geometry.
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

/// Ball reached defensive third — own goal threatened.
///
/// Attribute binding (spec §"Ball reached defensive third"):
/// - Reads: `mental.positioning`, `mental.bravery`, `mental.anticipation`
/// - Bias: `Determination` (personality.determination) — high determination
///   makes the player react more decisively.
///
/// Spatial stub: T1-2b uses `positioning × bravery × anticipation` composite
/// as a proxy for "player is disciplined enough to track the ball into their
/// own third". T1-4 will replace with real ball-zone detection.
///
/// **Not wired into `dispatch_tick`** — defers to T1-4.
pub fn predicate_ball_reached_defensive_third(player: &PlayerState) -> bool {
    let a = &player.attributes;
    // Primary composite: positioning × bravery × anticipation.
    // High all three → player reads defensive danger + is brave enough to track.
    let composite = a.mental.positioning * a.mental.bravery * a.mental.anticipation;
    // Bias: determination amplifies the threshold sensitivity.
    // Effective threshold: 0.125 / (1 + 0.40 × determination) — more determined
    // players react at lower composites (they track back more eagerly).
    let threshold_base = Q32::from_raw(1i64 << 29); // ≈ 0.125
    let det_factor = Q32::ONE + Q32::from_raw(1_717_986_918) * a.personality.determination; // 0.40 × det
    let threshold = threshold_base / det_factor;
    composite > threshold
}

/// Shot incoming — defending goalkeeper attention.
///
/// Attribute binding (spec §"Shot incoming") — GK-only predicate:
/// - Reads: `goalkeeper.reflexes`, `mental.positioning`, `goalkeeper.handling`
/// - Bias: `Composure` (mental.composure) — calm GKs hold their position.
///
/// Spatial stub: T1-2b uses `reflexes × positioning × handling` composite
/// as a proxy for "GK reads the incoming shot". T1-4 will replace with real
/// ball-trajectory-aimed-at-goal detection.
///
/// **Not wired into `dispatch_tick`** — defers to T1-4.
pub fn predicate_shot_incoming(player: &PlayerState) -> bool {
    let a = &player.attributes;
    // Primary composite: reflexes × positioning × handling.
    let composite = a.goalkeeper.reflexes * a.mental.positioning * a.goalkeeper.handling;
    // Bias: composure modulates threshold — more composed GKs react at lower
    // composite values (they read the game better).
    let threshold_base = Q32::from_raw(1i64 << 29); // ≈ 0.125
    let comp_factor = Q32::ONE + Q32::from_raw(1_288_490_188) * a.mental.composure; // 0.30 × composure
    let threshold = threshold_base / comp_factor;
    composite > threshold
}

/// Marker arrived — under pressure, off-ball player.
///
/// Attribute binding (spec §"Marker arrived"):
/// - Reads: `mental.composure`, `physical.balance`, `mental.anticipation`
/// - Bias: `PressureTolerance` (personality.pressure_tolerance) — tolerant
///   players respond to marker arrival without panic.
///
/// Spatial stub: T1-2b uses `composure × balance × anticipation` composite
/// as a proxy for "player reads that an opponent has closed them down".
/// T1-4 will replace with real opponent-distance-to-player detection.
///
/// **Not wired into `dispatch_tick`** — defers to T1-4.
pub fn predicate_marker_arrived(player: &PlayerState) -> bool {
    let a = &player.attributes;
    // Primary composite: composure × balance × anticipation.
    // High all three → player is calm under marking + reads the arrival.
    let composite = a.mental.composure * a.physical.balance * a.mental.anticipation;
    // Bias: pressure_tolerance lowers the effective threshold — high PT players
    // respond to markers even with moderate composites.
    let threshold_base = Q32::from_raw(1i64 << 29); // ≈ 0.125
    let pt_factor = Q32::ONE + Q32::from_raw(1_288_490_188) * a.personality.pressure_tolerance; // 0.30 × PT
    let threshold = threshold_base / pt_factor;
    composite > threshold
}

/// Through-ball intercept — opposition through-ball in flight.
///
/// Attribute binding (spec §"Through-ball intercept"):
/// - Reads: `mental.anticipation`, `mental.positioning`, `physical.pace`
/// - Bias: `Aggression` (personality.aggression) — aggressive players commit
///   to interception more readily.
///
/// Spatial stub: T1-2b uses `anticipation × positioning × pace` composite
/// as a proxy for "player is positioned to intercept a through-ball".
/// T1-4 will replace with real ball-trajectory + player-position geometry.
///
/// **Not wired into `dispatch_tick`** — defers to T1-4.
pub fn predicate_through_ball_intercept(player: &PlayerState) -> bool {
    let a = &player.attributes;
    // Primary composite: anticipation × positioning × pace.
    let composite = a.mental.anticipation * a.mental.positioning * a.physical.pace;
    // Bias: aggression lowers threshold — aggressive players commit to the run.
    let threshold_base = Q32::from_raw(1i64 << 29); // ≈ 0.125
    let agg_factor = Q32::ONE + Q32::from_raw(1_717_986_918) * a.personality.aggression; // 0.40 × aggression
    let threshold = threshold_base / agg_factor;
    composite > threshold
}

// ---------------------------------------------------------------------------
// Tests (P2-9 spec-aligned predicates)
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
    // predicate_ball_reached_defensive_third
    // ---------------------------------------------------------------------------

    #[test]
    fn ball_reached_defensive_third_true_for_high_spec_attrs() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.mental.positioning = Q32::ONE;
        attrs.mental.bravery = Q32::ONE;
        attrs.mental.anticipation = Q32::ONE;
        let p = player_with_attrs(attrs);
        assert!(predicate_ball_reached_defensive_third(&p));
    }

    #[test]
    fn ball_reached_defensive_third_false_for_zero_attrs() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.mental.positioning = Q32::ZERO;
        attrs.mental.bravery = Q32::ZERO;
        attrs.mental.anticipation = Q32::ZERO;
        let p = player_with_attrs(attrs);
        assert!(!predicate_ball_reached_defensive_third(&p));
    }

    #[test]
    fn ball_reached_defensive_third_non_spec_attr_has_no_effect() {
        // `technical.finishing` is not in this binding.
        let mut p_a = player_with_attrs(PlayerAttributes::mid_range_baseline());
        let mut p_b = player_with_attrs(PlayerAttributes::mid_range_baseline());
        p_a.attributes.technical.finishing = Q32::ZERO;
        p_b.attributes.technical.finishing = Q32::ONE;
        assert_eq!(
            predicate_ball_reached_defensive_third(&p_a),
            predicate_ball_reached_defensive_third(&p_b),
            "technical.finishing must not affect ball_reached_defensive_third"
        );
    }

    // ---------------------------------------------------------------------------
    // predicate_shot_incoming
    // ---------------------------------------------------------------------------

    #[test]
    fn shot_incoming_true_for_high_spec_attrs() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.goalkeeper.reflexes = Q32::ONE;
        attrs.mental.positioning = Q32::ONE;
        attrs.goalkeeper.handling = Q32::ONE;
        let p = player_with_attrs(attrs);
        assert!(predicate_shot_incoming(&p));
    }

    #[test]
    fn shot_incoming_false_for_zero_attrs() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.goalkeeper.reflexes = Q32::ZERO;
        attrs.mental.positioning = Q32::ZERO;
        attrs.goalkeeper.handling = Q32::ZERO;
        let p = player_with_attrs(attrs);
        assert!(!predicate_shot_incoming(&p));
    }

    #[test]
    fn shot_incoming_non_spec_attr_has_no_effect() {
        // `personality.aggression` not in this binding.
        let mut p_a = player_with_attrs(PlayerAttributes::mid_range_baseline());
        let mut p_b = player_with_attrs(PlayerAttributes::mid_range_baseline());
        p_a.attributes.personality.aggression = Q32::ZERO;
        p_b.attributes.personality.aggression = Q32::ONE;
        assert_eq!(
            predicate_shot_incoming(&p_a),
            predicate_shot_incoming(&p_b),
            "personality.aggression must not affect shot_incoming"
        );
    }

    // ---------------------------------------------------------------------------
    // predicate_marker_arrived
    // ---------------------------------------------------------------------------

    #[test]
    fn marker_arrived_true_for_high_spec_attrs() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.mental.composure = Q32::ONE;
        attrs.physical.balance = Q32::ONE;
        attrs.mental.anticipation = Q32::ONE;
        let p = player_with_attrs(attrs);
        assert!(predicate_marker_arrived(&p));
    }

    #[test]
    fn marker_arrived_false_for_zero_attrs() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.mental.composure = Q32::ZERO;
        attrs.physical.balance = Q32::ZERO;
        attrs.mental.anticipation = Q32::ZERO;
        let p = player_with_attrs(attrs);
        assert!(!predicate_marker_arrived(&p));
    }

    #[test]
    fn marker_arrived_non_spec_attr_has_no_effect() {
        // `personality.aggression` not in this binding.
        let mut p_a = player_with_attrs(PlayerAttributes::mid_range_baseline());
        let mut p_b = player_with_attrs(PlayerAttributes::mid_range_baseline());
        p_a.attributes.personality.aggression = Q32::ZERO;
        p_b.attributes.personality.aggression = Q32::ONE;
        assert_eq!(
            predicate_marker_arrived(&p_a),
            predicate_marker_arrived(&p_b),
            "personality.aggression must not affect marker_arrived"
        );
    }

    // ---------------------------------------------------------------------------
    // predicate_through_ball_intercept
    // ---------------------------------------------------------------------------

    #[test]
    fn through_ball_intercept_true_for_high_spec_attrs() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.mental.anticipation = Q32::ONE;
        attrs.mental.positioning = Q32::ONE;
        attrs.physical.pace = Q32::ONE;
        let p = player_with_attrs(attrs);
        assert!(predicate_through_ball_intercept(&p));
    }

    #[test]
    fn through_ball_intercept_false_for_zero_pace() {
        let mut attrs = PlayerAttributes::mid_range_baseline();
        attrs.mental.anticipation = Q32::ONE;
        attrs.mental.positioning = Q32::ONE;
        attrs.physical.pace = Q32::ZERO;
        let p = player_with_attrs(attrs);
        assert!(!predicate_through_ball_intercept(&p));
    }

    #[test]
    fn through_ball_intercept_non_spec_attr_has_no_effect() {
        // `mental.decisions` not in this binding.
        let mut p_a = player_with_attrs(PlayerAttributes::mid_range_baseline());
        let mut p_b = player_with_attrs(PlayerAttributes::mid_range_baseline());
        p_a.attributes.mental.decisions = Q32::ZERO;
        p_b.attributes.mental.decisions = Q32::ONE;
        assert_eq!(
            predicate_through_ball_intercept(&p_a),
            predicate_through_ball_intercept(&p_b),
            "mental.decisions must not affect through_ball_intercept"
        );
    }

    // Pure function check: same input → same output.
    #[test]
    fn predicates_are_deterministic() {
        let attrs = PlayerAttributes::mid_range_baseline();
        let p = player_with_attrs(attrs);
        assert_eq!(
            predicate_ball_reached_defensive_third(&p),
            predicate_ball_reached_defensive_third(&p)
        );
        assert_eq!(predicate_shot_incoming(&p), predicate_shot_incoming(&p));
        assert_eq!(predicate_marker_arrived(&p), predicate_marker_arrived(&p));
        assert_eq!(
            predicate_through_ball_intercept(&p),
            predicate_through_ball_intercept(&p)
        );
    }
}

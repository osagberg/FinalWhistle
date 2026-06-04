//! Cross-crate discriminant alignment test — T1-11 chunk 1.
//!
//! This test is the single source of truth for the `MatchEvent::discriminant()`
//! byte values. It pins them against a hardcoded table (not derived from
//! `MatchEventDiscriminant::all()` — that would be the same source).
//!
//! ## Why this test catches the next contributor who reorders variants
//!
//! The canonical encoder's `encode_match_event` writes discriminant bytes
//! directly to the wire format. If a future contributor reorders `MatchEvent`
//! variants (which changes the compiler-assigned discriminants in a
//! `#[repr(...)]`-free enum) without updating the encoder, the hash drifts.
//! This test explicitly pins the byte value for each variant:
//! - If you rename a variant → test fails to compile.
//! - If you reorder variants → the `#[must_use]` method returns the stable
//!   hand-assigned byte, not the position byte, so the encoder stays correct
//!   AND this test confirms the values haven't changed.
//!
//! ## Cross-crate scope
//!
//! This test lives in `fw-content/tests/` (integration test directory) so
//! it can see the public API of `fw-content` without `pub(crate)` relaxation.
//! It does NOT depend on `fw-match-sim` — the discriminant method lives in
//! `fw-content::event` and the encoder consumes it from there.
//!
//! ## Anti-vacuousness discipline
//!
//! Per the T1-4b/T1-3.5 lessons: assert explicit byte values, not just
//! "it returns something non-zero". Each variant maps to a specific u8
//! that is also the value the canonical encoder writes. If discriminant()
//! returns the wrong byte, the canonical hash would silently change.

use fw_content::SignatureId;
use fw_content::event::{MatchEvent, PassKind};
use fw_core::{Q32, Tick};

// ---------------------------------------------------------------------------
// Discriminant table (HARDCODED — NOT derived from MatchEventDiscriminant::all())
// ---------------------------------------------------------------------------

/// The canonical discriminant byte for each `MatchEvent` variant.
///
/// These values MUST match what the canonical encoder writes in
/// `fw-match-sim::canonical::encode_match_event`. If they drift, the
/// canonical hash changes silently.
///
/// Do NOT change these values without:
/// 1. A task spec authorizing a hash rebaseline (ADR-0012 trigger #1).
/// 2. Updating the canonical encoder literals (or verifying the encoder
///    uses `event.discriminant()` — in which case it automatically stays aligned).
/// 3. Updating the `PINNED_60_TICK` hash and RON fixture.
const KICKOFF_DISC: u8 = 0;
const FULLTIME_DISC: u8 = 1;
const GOAL_DISC: u8 = 2;
const SHOT_DISC: u8 = 3;
const PASS_DISC: u8 = 4;
const SIGNATURE_FIRST_FIRED_DISC: u8 = 5;
/// FUN-TS2b: Offside detection (discriminant 6).
const OFFSIDE_DISC: u8 = 6;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn kickoff() -> MatchEvent {
    MatchEvent::KickOff {
        tick: Tick::ZERO,
        is_second_half: false,
    }
}

fn fulltime() -> MatchEvent {
    MatchEvent::FullTime {
        tick: Tick::from_raw(60),
        home_score: 1,
        away_score: 0,
    }
}

fn goal() -> MatchEvent {
    MatchEvent::Goal {
        scorer_slot: 9,
        tick: Tick::from_raw(30),
        score_home_after: 1,
        score_away_after: 0,
    }
}

fn shot() -> MatchEvent {
    MatchEvent::Shot {
        shooter_slot: 9,
        tick: Tick::from_raw(25),
        target_x: Q32::from_int(52),
        target_y: Q32::ZERO,
        on_target: true,
    }
}

fn pass_ev() -> MatchEvent {
    MatchEvent::Pass {
        from_slot: 5,
        to_slot: 7,
        tick: Tick::from_raw(20),
        kind: PassKind::Short,
        completed: true,
    }
}

fn signature_first_fired() -> MatchEvent {
    let id = SignatureId::try_new("fwh.core:signature.long-range-strike").unwrap();
    MatchEvent::SignatureFirstFired {
        player_slot: 9,
        signature_id: id,
        tick: Tick::from_raw(50),
    }
}

// ---------------------------------------------------------------------------
// RED tests: discriminant() method returns hardcoded table values
// ---------------------------------------------------------------------------

#[test]
fn kickoff_discriminant_is_0() {
    assert_eq!(
        kickoff().discriminant() as u8,
        KICKOFF_DISC,
        "KickOff discriminant must be {} (canonical encoder writes this byte)",
        KICKOFF_DISC
    );
}

#[test]
fn fulltime_discriminant_is_1() {
    assert_eq!(
        fulltime().discriminant() as u8,
        FULLTIME_DISC,
        "FullTime discriminant must be {} (canonical encoder writes this byte)",
        FULLTIME_DISC
    );
}

#[test]
fn goal_discriminant_is_2() {
    assert_eq!(
        goal().discriminant() as u8,
        GOAL_DISC,
        "Goal discriminant must be {} (canonical encoder writes this byte)",
        GOAL_DISC
    );
}

#[test]
fn shot_discriminant_is_3() {
    assert_eq!(
        shot().discriminant() as u8,
        SHOT_DISC,
        "Shot discriminant must be {} (canonical encoder writes this byte)",
        SHOT_DISC
    );
}

#[test]
fn pass_discriminant_is_4() {
    assert_eq!(
        pass_ev().discriminant() as u8,
        PASS_DISC,
        "Pass discriminant must be {} (canonical encoder writes this byte)",
        PASS_DISC
    );
}

#[test]
fn signature_first_fired_discriminant_is_5() {
    assert_eq!(
        signature_first_fired().discriminant() as u8,
        SIGNATURE_FIRST_FIRED_DISC,
        "SignatureFirstFired discriminant must be {} (canonical encoder writes this byte)",
        SIGNATURE_FIRST_FIRED_DISC
    );
}

fn offside_ev() -> MatchEvent {
    MatchEvent::Offside {
        offending_slot: 8,
        tick: Tick::from_raw(55),
    }
}

#[test]
fn offside_discriminant_is_6() {
    assert_eq!(
        offside_ev().discriminant() as u8,
        OFFSIDE_DISC,
        "Offside discriminant must be {} (FUN-TS2b canonical encoder)",
        OFFSIDE_DISC
    );
}

/// Anti-vacuousness: all 7 discriminants are DISTINCT.
///
/// This test catches a naive implementation that returns a constant for all
/// variants (which would pass each individual test above if the constant
/// were 0..6, but would fail this one).
#[test]
fn all_discriminants_are_distinct() {
    let all = [
        kickoff().discriminant() as u8,
        fulltime().discriminant() as u8,
        goal().discriminant() as u8,
        shot().discriminant() as u8,
        pass_ev().discriminant() as u8,
        signature_first_fired().discriminant() as u8,
        offside_ev().discriminant() as u8,
    ];
    // All must be distinct: collect into a BTreeSet and check the size.
    use std::collections::BTreeSet;
    let distinct: BTreeSet<u8> = all.iter().copied().collect();
    assert_eq!(
        distinct.len(),
        all.len(),
        "discriminant() returned duplicate values across variants: {:?}",
        all
    );
}

/// Cross-crate alignment: `MatchEvent::discriminant()` agrees with
/// `MatchEventDiscriminant::from_event()` on the byte value.
///
/// This ensures the refactored `from_event` consumes `discriminant()`
/// (or at least stays aligned). The canonical encoder also uses
/// `discriminant()`, so both must agree.
#[test]
fn discriminant_agrees_with_match_event_discriminant_from_event() {
    use fw_content::commentary::MatchEventDiscriminant;

    let pairs: &[(MatchEvent, MatchEventDiscriminant)] = &[
        (kickoff(), MatchEventDiscriminant::KickOff),
        (fulltime(), MatchEventDiscriminant::FullTime),
        (goal(), MatchEventDiscriminant::Goal),
        (shot(), MatchEventDiscriminant::Shot),
        (pass_ev(), MatchEventDiscriminant::Pass),
        (
            signature_first_fired(),
            MatchEventDiscriminant::SignatureFirstFired,
        ),
        (offside_ev(), MatchEventDiscriminant::Offside),
    ];

    for (event, expected_disc) in pairs {
        let disc_byte = event.discriminant() as u8;
        let disc_from_event = MatchEventDiscriminant::from_event(event);
        assert_eq!(
            disc_from_event, *expected_disc,
            "from_event({:?}) returned wrong MatchEventDiscriminant",
            event
        );
        assert_eq!(
            disc_byte, *expected_disc as u8,
            "discriminant() byte for {:?} ({}) != MatchEventDiscriminant as u8 ({})",
            event, disc_byte, *expected_disc as u8
        );
    }
}

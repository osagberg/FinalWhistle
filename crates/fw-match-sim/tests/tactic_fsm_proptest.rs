//! Proptest invariants for the T1-2b-ii tactic FSM.
//!
//! Per `docs/specs/tactic-fsm.md` T1-2b acceptance:
//! 1. **Transition determinism** — same `(from_state, archetype, event)` →
//!    same `to_state` over random inputs.
//! 2. **No RNG in tactic FSM** — `apply_event` and `heartbeat_check` are
//!    pure functions; multiple calls with same inputs produce same output.
//! 3. **Heartbeat-drift** — HighPress for >600 ticks → MidBlock.
//! 4. **BallOutOfPlay always produces SetPiece** — structural invariant.
//! 5. **Goal / HalfTime always reset to MidBlock** — structural invariant.

use fw_core::Tick;
use fw_match_sim::tactic_fsm::{
    ArchetypeParams, CounterIntent, PressIntensity, SetPieceKind, TacticEvent, TacticState,
    TeamTacticState, apply_event, heartbeat_check,
};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

fn arb_tactic_state() -> impl Strategy<Value = TacticState> {
    prop_oneof![
        Just(TacticState::HighPress),
        Just(TacticState::MidBlock),
        Just(TacticState::LowBlock),
        Just(TacticState::CounterAttack),
        arb_set_piece_kind().prop_map(TacticState::SetPiece),
    ]
}

fn arb_set_piece_kind() -> impl Strategy<Value = SetPieceKind> {
    prop_oneof![
        Just(SetPieceKind::KickOff),
        Just(SetPieceKind::GoalKick),
        Just(SetPieceKind::GoalKickOpponent),
        Just(SetPieceKind::CornerFor),
        Just(SetPieceKind::CornerAgainst),
        Just(SetPieceKind::FreeKickFor),
        Just(SetPieceKind::FreeKickAgainst),
        Just(SetPieceKind::ThrowInFor),
        Just(SetPieceKind::ThrowInAgainst),
        Just(SetPieceKind::PenaltyFor),
        Just(SetPieceKind::PenaltyAgainst),
    ]
}

fn arb_tactic_event() -> impl Strategy<Value = TacticEvent> {
    prop_oneof![
        arb_set_piece_kind().prop_map(|kind| TacticEvent::BallOutOfPlay { kind }),
        Just(TacticEvent::BallInPlay),
        any::<bool>().prop_map(|r| TacticEvent::PossessionLost { recovery_likely: r }),
        any::<bool>().prop_map(|b| TacticEvent::BallRecovered {
            opponent_shape_broken: b
        }),
        Just(TacticEvent::PressTimeoutExpired),
        Just(TacticEvent::CounterWindowClosed),
        Just(TacticEvent::Goal),
        Just(TacticEvent::HalfTime),
    ]
}

fn arb_press_intensity() -> impl Strategy<Value = PressIntensity> {
    prop_oneof![
        Just(PressIntensity::None),
        Just(PressIntensity::Low),
        Just(PressIntensity::Default),
        Just(PressIntensity::High),
    ]
}

fn arb_counter_intent() -> impl Strategy<Value = CounterIntent> {
    prop_oneof![
        Just(CounterIntent::None),
        Just(CounterIntent::Default),
        Just(CounterIntent::High),
    ]
}

fn arb_archetype_params() -> impl Strategy<Value = ArchetypeParams> {
    (
        arb_tactic_state(),
        arb_press_intensity(),
        arb_counter_intent(),
    )
        .prop_map(
            |(default_in_defence_state, press_intensity, counter_intent)| ArchetypeParams {
                default_in_defence_state,
                press_intensity,
                counter_intent,
            },
        )
}

fn arb_team_tactic_state() -> impl Strategy<Value = TeamTacticState> {
    // Construct via initial().transition(...) — Codex P1 type-design: the
    // TeamTacticState fields are pub(crate); external test code uses the
    // public constructor path instead of struct-literal syntax.
    (arb_tactic_state(), 0i64..5400).prop_map(|(state, tick_raw)| {
        TeamTacticState::initial().transition(state, Tick::from_raw(tick_raw))
    })
}

/// T2-R-F2 (post-T2 ultimate-review Track F-2): joint strategy
/// generating `(current, now_tick)` where `now_tick >= current.entry_tick()`
/// is invariant by construction. The prior shape independently drew
/// `now_tick in 0..10000` then used `prop_assume!` to reject pairs that
/// violate the invariant — at `PROPTEST_CASES=10000` that wasted ~1024
/// global rejects (proptest aborts the run when global reject budget
/// is exhausted, around 2.7k successful cases). Composing the strategies
/// eliminates rejection waste, so audit-time property explosion can
/// scale to 10k cases cleanly.
fn arb_team_tactic_state_and_now_tick() -> impl Strategy<Value = (TeamTacticState, Tick)> {
    arb_team_tactic_state().prop_flat_map(|current| {
        let entry_raw = current.entry_tick().to_raw();
        // 10000-tick window of "now" past entry — covers the same
        // semantic range the prior 0..10000 generator targeted (~166s
        // of real time at 60 Hz) without rejection.
        (Just(current), (entry_raw..entry_raw.saturating_add(10_000)))
            .prop_map(|(c, now_raw)| (c, Tick::from_raw(now_raw)))
    })
}

// ---------------------------------------------------------------------------
// Invariants
// ---------------------------------------------------------------------------

proptest! {
    /// Invariant 1: transition determinism.
    ///
    /// `apply_event` is a pure function: same inputs → same result state.
    ///
    /// T1-23 (post-Codex Finding #1): `prop_assume!` filters out inputs that
    /// violate the cooldown-math invariant `now_tick >= entry_tick`. The
    /// pre-T1-23 saturating arithmetic silently saturated to 0 on the
    /// invariant-violating input space; the new `Tick::checked_elapsed_since`
    /// panics per §11. The purity property is verified for the legal input
    /// space; the panic-on-illegal-input contract is verified separately by
    /// `fw_core::tick::tests::checked_elapsed_since_panics_when_entry_in_future`.
    #[test]
    fn transition_is_deterministic(
        // T2-R-F2: joint strategy guarantees now_tick >= entry_tick by
        // construction (no prop_assume! rejection waste). See
        // arb_team_tactic_state_and_now_tick rationale.
        (current, now_tick) in arb_team_tactic_state_and_now_tick(),
        archetype in arb_archetype_params(),
        event in arb_tactic_event(),
    ) {
        let a = apply_event(current, &archetype, event, now_tick);
        let b = apply_event(current, &archetype, event, now_tick);
        prop_assert_eq!(a.state(), b.state());
        prop_assert_eq!(a.entry_tick(), b.entry_tick());
    }

    /// Invariant 2: `apply_event` is pure (no-RNG witness).
    ///
    /// Three identical calls → identical results.
    #[test]
    fn apply_event_is_pure(
        // T2-R-F2: joint strategy — see above.
        (current, now_tick) in arb_team_tactic_state_and_now_tick(),
        archetype in arb_archetype_params(),
        event in arb_tactic_event(),
    ) {
        let a = apply_event(current, &archetype, event, now_tick);
        let b = apply_event(current, &archetype, event, now_tick);
        let c = apply_event(current, &archetype, event, now_tick);
        prop_assert_eq!(a.state(), b.state());
        prop_assert_eq!(b.state(), c.state());
    }

    /// Invariant 2b: `heartbeat_check` is pure.
    #[test]
    fn heartbeat_check_is_pure(
        // T2-R-F2: joint strategy — see above.
        (current, now_tick) in arb_team_tactic_state_and_now_tick(),
    ) {
        let a = heartbeat_check(&current, now_tick);
        let b = heartbeat_check(&current, now_tick);
        let c = heartbeat_check(&current, now_tick);
        prop_assert_eq!(a, b);
        prop_assert_eq!(b, c);
    }

    /// Invariant 3: BallOutOfPlay always produces SetPiece(kind).
    ///
    /// For ANY (from_state, archetype, set_piece_kind), BallOutOfPlay
    /// always transitions to SetPiece(kind). No guards.
    #[test]
    fn ball_out_of_play_always_produces_setpiece(
        current in arb_team_tactic_state(),
        archetype in arb_archetype_params(),
        kind in arb_set_piece_kind(),
        now_tick_raw in 0i64..10000,
    ) {
        let now_tick = Tick::from_raw(now_tick_raw);
        let result = apply_event(
            current,
            &archetype,
            TacticEvent::BallOutOfPlay { kind },
            now_tick,
        );
        prop_assert_eq!(result.state(), TacticState::SetPiece(kind));
    }

    /// Invariant 4: Goal always resets to MidBlock.
    #[test]
    fn goal_always_resets_to_midblock(
        current in arb_team_tactic_state(),
        archetype in arb_archetype_params(),
        now_tick_raw in 0i64..10000,
    ) {
        let now_tick = Tick::from_raw(now_tick_raw);
        let result = apply_event(current, &archetype, TacticEvent::Goal, now_tick);
        prop_assert_eq!(result.state(), TacticState::MidBlock);
    }

    /// Invariant 5: HalfTime always resets to MidBlock.
    #[test]
    fn halftime_always_resets_to_midblock(
        current in arb_team_tactic_state(),
        archetype in arb_archetype_params(),
        now_tick_raw in 0i64..10000,
    ) {
        let now_tick = Tick::from_raw(now_tick_raw);
        let result = apply_event(current, &archetype, TacticEvent::HalfTime, now_tick);
        prop_assert_eq!(result.state(), TacticState::MidBlock);
    }

    /// Invariant 6: HighPress heartbeat fires after >600 ticks in state.
    /// Codex P2 (self-review): the heartbeat now returns the FULL new
    /// TeamTacticState (state + entry_tick atomically advanced to now_tick),
    /// not just the new TacticState. Tests verify both fields.
    #[test]
    fn highpress_heartbeat_fires_after_600_ticks(
        entry_tick_raw in 0i64..5000,
        extra_ticks in 1i64..1000,
    ) {
        let entry_tick = Tick::from_raw(entry_tick_raw);
        let current = TeamTacticState::initial().transition(TacticState::HighPress, entry_tick);
        let now_raw = entry_tick_raw.saturating_add(600).saturating_add(extra_ticks);
        let now_tick = Tick::from_raw(now_raw);
        let result = heartbeat_check(&current, now_tick);
        let new_tts = result.expect("heartbeat must fire after >600 ticks");
        prop_assert_eq!(new_tts.state(), TacticState::MidBlock);
        prop_assert_eq!(
            new_tts.entry_tick(),
            now_tick,
            "entry_tick must advance to the heartbeat tick (not stay at the prior state's entry)"
        );
    }

    /// Invariant 7: HighPress heartbeat does NOT fire at or below 600 ticks.
    #[test]
    fn highpress_heartbeat_does_not_fire_at_or_below_600_ticks(
        entry_tick_raw in 0i64..5000,
        ticks_in_state in 0i64..=600,
    ) {
        let entry_tick = Tick::from_raw(entry_tick_raw);
        let current = TeamTacticState::initial().transition(TacticState::HighPress, entry_tick);
        let now_raw = entry_tick_raw.saturating_add(ticks_in_state);
        let now_tick = Tick::from_raw(now_raw);
        let result = heartbeat_check(&current, now_tick);
        prop_assert_eq!(result, None);
    }

    /// Invariant 8: non-HighPress states never trigger heartbeat.
    ///
    /// T1-2b-ii only implements the HighPress-timeout rule. MidBlock,
    /// LowBlock, CounterAttack heartbeat_check always returns None.
    #[test]
    fn non_highpress_heartbeat_never_fires(
        tactic_state in prop_oneof![
            Just(TacticState::MidBlock),
            Just(TacticState::LowBlock),
            Just(TacticState::CounterAttack),
        ],
        entry_tick_raw in 0i64..5000,
        now_tick_raw in 0i64..10000,
    ) {
        // T1-23 note: no prop_assume!(now >= entry) filter needed here.
        // `heartbeat_check` only calls `checked_elapsed_since` inside the
        // `if current.state == TacticState::HighPress` branch (see
        // `tactic_fsm.rs::heartbeat_check`); this invariant deliberately
        // excludes HighPress via the `prop_oneof![MidBlock, LowBlock,
        // CounterAttack]` strategy above, so the panic branch is unreachable
        // for this test's input space. Documented here so a future
        // maintainer reading this alongside the 3 filtered Invariants
        // (1, 2, 2b) doesn't add a spurious filter — or worse, copy this
        // unfiltered pattern to a HighPress-scoped test and get false
        // confidence.
        let current = TeamTacticState::initial()
            .transition(tactic_state, Tick::from_raw(entry_tick_raw));
        let result = heartbeat_check(&current, Tick::from_raw(now_tick_raw));
        prop_assert_eq!(result, None);
    }
}

//! T2-1c integration tests for `BallOutOfPlay` emission in the OOB-clamp
//! block + `BallInPlay` auto-exit in `emit_possession_transition_events`.
//!
//! These tests integrate the full `tick_match` path rather than calling the
//! `tactic_fsm::apply_event` arms directly — they verify the WIRING in
//! `lib.rs::tick_match` actually fires the new TacticEvent emissions, not
//! just that the arms work in isolation (which `tactic_fsm.rs`'s own test
//! module already covers).
//!
//! Test design per the T2-1c MEMORY spec AC4 + AC5 + the post-2026-05-16
//! mutation-thinking pre-check discipline: each assertion is mutation-
//! discriminating — flipping the per-team SetPieceKind logic, dropping the
//! auto-exit pattern, or skipping the BallOutOfPlay emission entirely would
//! surface here as a clear failure.

use fw_core::{Q32, Seed, Tick};
use fw_match_sim::{BallState, MatchState, TeamTacticState, dto::PlayerFrameDto, tick_match};
use std::collections::BTreeMap;

/// Borrowed constants for clarity (mirror `fw_core::{GOAL_LINE_X, SIDELINE_Y,
/// GOAL_HALF_WIDTH_M}` semantics without re-importing them — the tests
/// drive the ball via raw Q32 values rather than through the pitch
/// constants to keep the inputs surface-clean).
const SMOKE_SEED: u64 = 0xDEAD_BEEF_DEAD_BEEF;

// ---------------------------------------------------------------------------
// AC4: BallOutOfPlay emission per-team with correct SetPieceKind
// ---------------------------------------------------------------------------

/// Driving the ball past the sideline with home (slot 8) as last_touched_by
/// fires `TacticEvent::BallOutOfPlay { kind: ThrowInAgainst }` for the home
/// team + `TacticEvent::BallOutOfPlay { kind: ThrowInFor }` for the away
/// team. Both teams' tactic states transition to `SetPiece(kind)`.
///
/// Mutation discriminator: if the OOB-clamp doesn't emit OR per-team kind
/// logic is inverted, the assertion fails. The reciprocal nature (one
/// team's For matches the other's Against) is the strongest signal —
/// flipping team_of(last_touched_by) would invert both kinds in lockstep.
#[test]
fn ball_past_sideline_with_home_last_touched_emits_throw_in_for_away() {
    use fw_match_sim::{SetPieceKind, TacticState};

    // Construct a fresh state + force ball past the +Y sideline (positive Z
    // side in the home-orientation coordinate convention). Home (slot 8 =
    // home CAM) is the last_touched_by, so away gets the throw-in.
    let mut state = MatchState::initial(Seed::from_u64(SMOKE_SEED));
    // SIDELINE_Y = 34m approx (per fw_core). Drive ball to 35m, vel_y = 2m/s
    // so it's clearly past + still moving outward. vel_x = 0 to ensure it's
    // a pure sideline crossing (not a corner-flag edge case).
    state.ball.pos_x = Q32::from_int(0);
    state.ball.pos_y = Q32::from_int(35);
    state.ball.vel_x = Q32::ZERO;
    state.ball.vel_y = Q32::from_int(2);
    state.ball.pos_z = Q32::ZERO;
    state.ball.vel_z = Q32::ZERO;
    state = state.with_last_touched_by(8); // home CAM

    // Pre-condition: both teams in initial MidBlock state.
    assert_eq!(state.team_tactic_states[0].state(), TacticState::MidBlock);
    assert_eq!(state.team_tactic_states[1].state(), TacticState::MidBlock);

    state = tick_match(state, &BTreeMap::new());

    // Post-condition: OOB-clamp fired + BallOutOfPlay emitted per team.
    // Home (last touched the ball) gets ThrowInAgainst; away gets ThrowInFor.
    assert_eq!(
        state.team_tactic_states[0].state(),
        TacticState::SetPiece(SetPieceKind::ThrowInAgainst),
        "home team should transition to SetPiece(ThrowInAgainst) — home last touched"
    );
    assert_eq!(
        state.team_tactic_states[1].state(),
        TacticState::SetPiece(SetPieceKind::ThrowInFor),
        "away team should transition to SetPiece(ThrowInFor) — away takes the throw"
    );
}

/// Mirror of the home-last-touched case: away (slot 19 = away CAM) last
/// touched, so home gets the throw-in. Pinning the symmetry catches any
/// hardcoded "home always throws" bug that would have escaped the prior
/// test (where the team_of returns 0 for slot 8).
#[test]
fn ball_past_sideline_with_away_last_touched_emits_throw_in_for_home() {
    use fw_match_sim::{SetPieceKind, TacticState};

    let mut state = MatchState::initial(Seed::from_u64(SMOKE_SEED));
    state.ball.pos_x = Q32::from_int(0);
    state.ball.pos_y = Q32::from_int(35);
    state.ball.vel_x = Q32::ZERO;
    state.ball.vel_y = Q32::from_int(2);
    state.ball.pos_z = Q32::ZERO;
    state.ball.vel_z = Q32::ZERO;
    state = state.with_last_touched_by(19); // away CAM (slot 11 + 8)

    state = tick_match(state, &BTreeMap::new());

    assert_eq!(
        state.team_tactic_states[0].state(),
        TacticState::SetPiece(SetPieceKind::ThrowInFor),
        "home team should transition to SetPiece(ThrowInFor) — home takes the throw"
    );
    assert_eq!(
        state.team_tactic_states[1].state(),
        TacticState::SetPiece(SetPieceKind::ThrowInAgainst),
        "away team should transition to SetPiece(ThrowInAgainst) — away last touched"
    );
}

/// Ball crossing the AWAY-side goal-line (positive X) with home (slot 8)
/// as last_touched_by → home attacking + home touched last → home gets
/// CornerFor, away gets CornerAgainst.
#[test]
fn ball_past_away_goal_line_with_home_last_touched_emits_corner_for_home() {
    use fw_match_sim::{SetPieceKind, TacticState};

    let mut state = MatchState::initial(Seed::from_u64(SMOKE_SEED));
    // GOAL_LINE_X = 52.5m. GOAL_HALF_WIDTH_M = 3.66m. Place ball at
    // x=53, y=10 (past goal-line + outside goal-half-width = corner zone,
    // not goal zone). vel_x=2 so it's clearly past + moving outward.
    state.ball.pos_x = Q32::from_int(53);
    state.ball.pos_y = Q32::from_int(10);
    state.ball.vel_x = Q32::from_int(2);
    state.ball.vel_y = Q32::ZERO;
    state.ball.pos_z = Q32::ZERO;
    state.ball.vel_z = Q32::ZERO;
    state = state.with_last_touched_by(8); // home

    state = tick_match(state, &BTreeMap::new());

    assert_eq!(
        state.team_tactic_states[0].state(),
        TacticState::SetPiece(SetPieceKind::CornerFor),
        "home attacking + home last touched → home CornerFor"
    );
    assert_eq!(
        state.team_tactic_states[1].state(),
        TacticState::SetPiece(SetPieceKind::CornerAgainst),
        "home attacking + home last touched → away CornerAgainst"
    );
}

/// Ball crossing the AWAY-side goal-line with AWAY last_touched_by →
/// away kicks off the goal-kick + home gets the opponent variant.
#[test]
fn ball_past_away_goal_line_with_away_last_touched_emits_goal_kick_for_away() {
    use fw_match_sim::{SetPieceKind, TacticState};

    let mut state = MatchState::initial(Seed::from_u64(SMOKE_SEED));
    state.ball.pos_x = Q32::from_int(53);
    state.ball.pos_y = Q32::from_int(10);
    state.ball.vel_x = Q32::from_int(2);
    state.ball.vel_y = Q32::ZERO;
    state.ball.pos_z = Q32::ZERO;
    state.ball.vel_z = Q32::ZERO;
    state = state.with_last_touched_by(19); // away

    state = tick_match(state, &BTreeMap::new());

    assert_eq!(
        state.team_tactic_states[0].state(),
        TacticState::SetPiece(SetPieceKind::GoalKickOpponent),
        "home attacking + away last touched → home GoalKickOpponent"
    );
    assert_eq!(
        state.team_tactic_states[1].state(),
        TacticState::SetPiece(SetPieceKind::GoalKick),
        "home attacking + away last touched → away GoalKick"
    );
}

// ---------------------------------------------------------------------------
// AC5: BallInPlay auto-exits SetPiece state at next possession transition
// ---------------------------------------------------------------------------

/// After a team is in `SetPiece(_)` state (e.g. post-OOB), the next
/// possession transition should fire `apply_event(BallInPlay)` FIRST
/// to transition the team out of SetPiece + then fire the regular
/// `PossessionLost` / `BallRecovered` event.
///
/// Construction: pre-set team_tactic_states[0] to SetPiece(ThrowInFor)
/// at tick 1. Set up possession=Some(8) (home AM in possession). Run
/// one tick_match call that causes possession to transition (force a
/// shot intent via state manipulation). Assert that after the tick:
///   1. team_tactic_states[0].state is NOT SetPiece — auto-exit fired.
///   2. team_tactic_states[0].state matches the archetype-default
///      OR the post-PossessionLost state (LowBlock from MidBlock
///      fallback per the recovery_likely=false arm).
///
/// Mutation discriminator: if auto_exit_setpiece is skipped, the team
/// stays in SetPiece + the post-state assertion fails. If auto-exit
/// fires but PossessionLost is silently skipped, the team transitions
/// only to archetype default (MidBlock) — verifiable.
#[test]
fn setpiece_state_auto_exits_on_possession_loss_to_none() {
    use fw_match_sim::TacticState;

    // Build a state where home (slot 8) has possession + state.team_tactic_states[0]
    // is artificially set to SetPiece(KickOff). Then via direct possession
    // mutation: set state.possession = None to simulate a ball release at
    // the end of the tick BEFORE emit_possession_transition_events runs.
    //
    // Since emit_possession_transition_events is private to fw-match-sim,
    // we test the behavior end-to-end via tick_match: construct a state
    // that the dispatch step will turn into a Some→None possession via
    // a shot intent. Easier: use the SMOKE seed + run enough ticks for a
    // natural shot to fire.

    let mut state = MatchState::initial(Seed::from_u64(SMOKE_SEED));

    // Drive the ball past the sideline FIRST to put both teams in SetPiece.
    state.ball.pos_x = Q32::from_int(0);
    state.ball.pos_y = Q32::from_int(35);
    state.ball.vel_x = Q32::ZERO;
    state.ball.vel_y = Q32::from_int(2);
    state.ball.pos_z = Q32::ZERO;
    state.ball.vel_z = Q32::ZERO;
    state = state.with_last_touched_by(8);

    state = tick_match(state, &BTreeMap::new());

    // Confirm both teams are in SetPiece post-OOB.
    assert!(
        matches!(
            state.team_tactic_states[0].state(),
            TacticState::SetPiece(_)
        ),
        "AC4 pre-condition: home should be in SetPiece after sideline OOB"
    );
    assert!(
        matches!(
            state.team_tactic_states[1].state(),
            TacticState::SetPiece(_)
        ),
        "AC4 pre-condition: away should be in SetPiece after sideline OOB"
    );

    // Continue ticking — the smoke seed naturally produces a shot event
    // within a few hundred ticks (per T1-15 / T1-16 sim behavior).
    // When the shot fires, possession transitions Some→None +
    // emit_possession_transition_events fires PossessionLost(recovery_likely
    // =false) which would normally be a no-op while in SetPiece. The
    // auto_exit_setpiece helper fires BallInPlay FIRST, transitioning the
    // shooter's team out of SetPiece BEFORE the PossessionLost.

    // FUN-TS1 (2026-06-04): extended from 600 → 1200 ticks. With zonal
    // compactness active, players converge to their block more slowly (they're
    // moving toward new targets) before generating a shot that exits SetPiece.
    let mut exit_observed: Option<(usize, TacticState)> = None;
    for _ in 0..1200 {
        let prev_home_state = state.team_tactic_states[0].state();
        let prev_away_state = state.team_tactic_states[1].state();
        state = tick_match(state, &BTreeMap::new());
        // Look for an exit-SetPiece transition on either team. Capture
        // BOTH the team index + the post-exit state for the strengthened
        // assertion below (Codex Tier-2 audit 2026-05-17 P2 #3).
        let home_was_setpiece = matches!(prev_home_state, TacticState::SetPiece(_));
        let home_now_not = !matches!(
            state.team_tactic_states[0].state(),
            TacticState::SetPiece(_)
        );
        let away_was_setpiece = matches!(prev_away_state, TacticState::SetPiece(_));
        let away_now_not = !matches!(
            state.team_tactic_states[1].state(),
            TacticState::SetPiece(_)
        );
        if home_was_setpiece && home_now_not {
            exit_observed = Some((0, state.team_tactic_states[0].state()));
            break;
        }
        if away_was_setpiece && away_now_not {
            exit_observed = Some((1, state.team_tactic_states[1].state()));
            break;
        }
    }

    let (exited_team, post_exit_state) = exit_observed.unwrap_or_else(|| {
        panic!(
            "T2-1c auto-exit broken: no team transitioned out of SetPiece across 600 ticks. \
             Either auto_exit_setpiece never fired (BallInPlay not called) OR the SetPiece \
             state is sticky beyond what the smoke seed's possession transitions can drive. \
             Final home state: {:?}; final away state: {:?}",
            state.team_tactic_states[0].state(),
            state.team_tactic_states[1].state(),
        );
    });

    // FUN-0b+c: this is now a WIRING check — "the smoke seed enters AND exits
    // SetPiece via the real tick_match path, landing in a valid in-play state."
    // The silent-failure guard (Codex Tier-2 P2 #3 — auto_exit fires BallInPlay
    // but the subsequent PossessionLost/BallRecovered is dropped) moved to the
    // DETERMINISTIC controlled unit test
    // `lib.rs::setpiece_autoexit_tests::auto_exit_setpiece_then_possession_lost_lands_in_lowblock_not_midblock`.
    //
    // Why this assertion was weakened from "must be HighPress/LowBlock/CounterAttack"
    // to "must be any in-play state (incl. MidBlock)": the prior assertion rejected
    // MidBlock as proof-of-dropped-event, but MidBlock is a LEGITIMATE post-exit
    // landing. When the exit is driven by a cross-team transition, the losing team
    // gets PossessionLost{recovery_likely:true}, which from MidBlock returns MidBlock
    // unchanged while HighPress is on its re-entry cooldown (tactic_fsm.rs:419-421);
    // BallRecovered{opponent_shape_broken:false} likewise stays MidBlock. The Slice-B
    // tackle step shifted the smoke seed's single 600-tick exit (tick 548) into
    // exactly such a cross-team-under-cooldown case, so the old assertion failed on a
    // legitimate outcome. The final state alone cannot distinguish "event fired but
    // stayed MidBlock" from "event dropped" — hence the controlled unit test owns
    // that discrimination now, and this integration test just proves the public
    // wiring reaches SetPiece entry/exit at all.
    let post_exit_is_valid_in_play = matches!(
        post_exit_state,
        TacticState::MidBlock
            | TacticState::HighPress
            | TacticState::LowBlock
            | TacticState::CounterAttack
    );
    assert!(
        post_exit_is_valid_in_play,
        "team {exited_team} exited SetPiece but landed in {post_exit_state:?} — \
         expected a valid in-play state (MidBlock / HighPress / LowBlock / \
         CounterAttack). Landing anywhere else (e.g. still SetPiece, or an \
         unreachable state) means the auto-exit + possession-event wiring in \
         tick_match is broken. The silent-failure guard (subsequent event \
         dropped) is owned by the controlled unit test \
         setpiece_autoexit_tests::auto_exit_setpiece_then_possession_lost_lands_in_lowblock_not_midblock."
    );
}

// ---------------------------------------------------------------------------
// Codex Tier-2 audit 2026-05-17 P1 regression test: goal-tick early-return
// ---------------------------------------------------------------------------

/// Codex Tier-2 audit T2-1 review 2026-05-17 P1: when a goal fires on a
/// tick where the kickoff taker's decision slot is ALSO active, the
/// post-goal dispatch step would (pre-fix) mutate possession again + the
/// downstream `emit_possession_transition_events` would fire PossessionLost
/// or BallRecovered, overriding the Goal arm's MidBlock reset on both
/// teams.
///
/// Fix shape per Codex: skip dispatch + pickup + emit_possession_
/// transition_events when `goal_fired_this_tick` is true. The Goal arm of
/// `apply_event` becomes the single source of truth for goal-tick
/// tactic-FSM transitions; subsequent ticks resume normal flow.
///
/// Test construction: set `state.tick` to one tick before the kickoff
/// taker's `decision_slots[20]` value (slot 20 = away CF, the kickoff
/// taker when home scores) — then `tick_match`'s successor() advances
/// tick to exactly that decide value, AND the goal fires on that same
/// tick. Without the fix, dispatch would pick an intent for slot 20
/// (the kickoff taker has possession + an active decision slot), which
/// would mutate possession + the downstream emit would fire PossessionLost
/// transitioning `team_tactic_states[1]` (away team) from MidBlock to
/// LowBlock (the recovery_likely=false fallback arm at
/// tactic_fsm.rs:411-417). With the fix, both teams stay at MidBlock.
///
/// Mutation discriminator: if any of the 3 if-guards in tick_match
/// (`if !goal_fired_this_tick { dispatch... }`, `if !goal_fired_this_tick
/// { pickup... }`, `if !goal_fired_this_tick { emit_possession... }`)
/// is removed, the kickoff taker's dispatch picks AttemptShot or Dribble
/// or Pass on the goal-tick → possession mutates → emit fires
/// PossessionLost → away team transitions away from MidBlock → assertion
/// fails.
#[test]
fn goal_tick_skips_dispatch_so_kickoff_taker_decisions_dont_override_midblock() {
    use fw_match_sim::TacticState;

    // Construct fresh state + look up kickoff taker's decision slot value.
    // home-scored kickoff taker is slot 20 (away CF; slot 11+9 offset).
    let mut state = MatchState::initial(Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF));
    let kickoff_slot: usize = 20;
    let decide_value = state.decision_slots[kickoff_slot] as i64;

    // Advance tick to (decide_value - 1) so tick_match's successor() lands
    // on exactly decide_value, where slot 20 is an active decision slot.
    // (decision_cadence::should_decide uses tick % HEARTBEAT_INTERVAL_TICKS
    // i.e. tick % 30 == decision_slots[i]; setting tick to decide_value
    // means tick_match starts that tick by advancing to decide_value, and
    // the dispatch step would normally fire for slot 20.)
    state.tick = Tick::from_raw(decide_value - 1);

    // Set up goal-imminent ball state: ball just past +X goal-line, centered
    // y, zero velocity (the goal-detection step checks pos BEFORE physics
    // so vel doesn't matter for the check itself). last_touched_by = some
    // home slot (e.g. 8, home CAM) so the goal is attributed to home →
    // home_scored=true → kickoff_taker = slot 20 (away CF).
    state.ball.pos_x = Q32::from_int(53); // past +52.5 goal line
    state.ball.pos_y = Q32::ZERO; // centered (within goal mouth)
    state.ball.pos_z = Q32::ZERO;
    state.ball.vel_x = Q32::ZERO;
    state.ball.vel_y = Q32::ZERO;
    state.ball.vel_z = Q32::ZERO;
    state = state.with_last_touched_by(8); // home CAM
    // possession is pub(crate); set via the builder pattern. The test
    // doesn't need to pre-set possession because last_touched_by is what
    // the Goal arm reads for scorer attribution; possession before the
    // goal can be anything (the Goal arm overrides it to the kickoff
    // taker afterward).

    // Pre-condition: both teams in initial MidBlock.
    assert_eq!(state.team_tactic_states[0].state(), TacticState::MidBlock);
    assert_eq!(state.team_tactic_states[1].state(), TacticState::MidBlock);

    let state_after = tick_match(state, &BTreeMap::new());

    // Goal must have fired (verify via MatchEvent::Goal in the event stream).
    let goal_fired = state_after
        .match_events()
        .iter()
        .any(|e| matches!(e, fw_content::MatchEvent::Goal { .. }));
    assert!(
        goal_fired,
        "test construction failed: goal didn't fire on the constructed tick. \
         The early-return regression test only works if a goal fires; verify \
         ball.pos_x={:?} (must be past GOAL_LINE_X=52.5) + ball.pos_y={:?} \
         (must be within GOAL_HALF_WIDTH_M) at goal-detection time.",
        Q32::from_int(53),
        Q32::ZERO,
    );

    // Possession must be the kickoff taker (slot 20 = away CF since home scored).
    assert_eq!(
        state_after.possession(),
        Some(20),
        "Goal arm should set possession to away CF (slot 20) for home-scored kickoff"
    );

    // **The load-bearing assertion**: both teams MUST stay at MidBlock —
    // the Goal arm of apply_event hardcoded this; the early-return guards
    // prevent dispatch from picking a kickoff-taker intent that would
    // re-trigger PossessionLost/BallRecovered transitions.
    //
    // If any of the 3 if-guards is removed AND slot 20's BT picks an
    // intent that mutates possession (AttemptShot → Some→None; Pass to
    // home player → Some→Some cross-team; etc), the emit_possession_
    // transition_events fires PossessionLost or BallRecovered →
    // team_tactic_states[1] transitions from MidBlock to a different
    // TacticState (LowBlock under recovery_likely=false fallback, OR
    // CounterAttack under BallRecovered with opponent_shape_broken).
    assert_eq!(
        state_after.team_tactic_states[0].state(),
        TacticState::MidBlock,
        "P1 regression: home team's tactic state changed away from MidBlock \
         on the goal-tick. The Goal arm of apply_event sets BOTH teams to \
         MidBlock; the 3 if-guards (dispatch + pickup + emit_possession) \
         in tick_match must prevent the kickoff taker's same-tick decision \
         from triggering subsequent PossessionLost/BallRecovered transitions. \
         decide_value for slot 20 was {decide_value}; final tick was {:?}.",
        state_after.tick,
    );
    assert_eq!(
        state_after.team_tactic_states[1].state(),
        TacticState::MidBlock,
        "P1 regression: away team's tactic state changed away from MidBlock \
         on the goal-tick. See home-team assertion above for the discriminator \
         logic. away team is the kickoff-taker's team (slot 20 = away CF) so \
         if the early-return guards are broken, this is the assertion that \
         fires first (slot 20's BT decision drives PossessionLost on the away \
         team's PossessionLost arm at tactic_fsm.rs:411-417)."
    );
}

// ---------------------------------------------------------------------------
// Cross-tests: surface-area sanity (catch rename / signature drift on the
// consumed APIs at compile time).
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn _surface_area_witness() {
    let _: Q32 = Q32::zero();
    let _: Tick = Tick::ZERO;
    let _: Seed = Seed::from_u64(0);
    let _: MatchState = MatchState::initial(Seed::from_u64(0));
    let _: BallState = BallState::centre_spot();
    let _: TeamTacticState = TeamTacticState::initial();
    let _: PlayerFrameDto;
}

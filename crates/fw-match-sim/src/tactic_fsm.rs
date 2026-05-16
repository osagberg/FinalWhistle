//! Team-tactic finite-state machine (FSM) — layer 1 of the ADR-0001 sim stack.
//!
//! ## Design
//!
//! One FSM per team (indexed by `0 = home, 1 = away`). The FSM is
//! **deterministic-by-predicate**: no `seed_fn` calls, no RNG. Every
//! transition is a pure function of canonical state + archetype parameters.
//!
//! Implements `docs/specs/tactic-fsm.md` (Tranche 4 spec). The 2 Hz heartbeat
//! fires every 30 integration ticks (60 Hz / 2 Hz = 30) in `tick_match`.
//!
//! ## T1-2b-ii scope notes
//!
//! - `TacticEvent` is a **local stub** — only the events needed by the
//!   transition table are listed. T1-4 reconciles this with the full
//!   `MatchEvent` enum when that lands.
//! - `PressIntensity` and `CounterIntent` are **local enums**. T1-2b-iii
//!   moves them to `fw-content::TacticalArchetype` when the BT runner
//!   consumes them.
//! - `default_in_defence_state` / `line_height_metres_per_state` /
//!   `counter_intent` / `press_intensity` defer to T1-2b-iii RON param
//!   extensions. T1-2b-ii uses hardcoded defaults in `ArchetypeParams`.
//! - The heartbeat only implements the HighPress-timeout rule (the only
//!   rule with sufficient inputs at T1-2b-ii). Spatial rules (`own_mean_x`,
//!   `score_lead`) defer to T1-2b-iii when spatial state exists.

use fw_core::Tick;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Core state enums
// ---------------------------------------------------------------------------

/// The five team-tactic states. Commentary-legible, FSM-stable.
/// One per team; two in `MatchState.team_tactic_states`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TacticState {
    /// Aggressive press; line high; closing distance to ball-carrier.
    /// Active when opponent has the ball in their own half, press intensity
    /// ≥ High, and we haven't been pressing too long (Bauer-and-Anzer 5s
    /// window).
    HighPress,
    /// Mid-block compact shape; line at ~halfway; force play wide.
    /// Default in-defence state when neither HighPress nor LowBlock
    /// conditions hold.
    MidBlock,
    /// Deep block; line on or near own box; absorb pressure.
    /// Trigger: leading + late-game OR archetype-driven default.
    LowBlock,
    /// Active counter-attack window; verticality maximised.
    /// Trigger: just regained ball, opponent shape broken within 4s of
    /// recovery.
    CounterAttack,
    /// Active set-piece resolution; positions structured per archetype.
    /// Trigger: ball dead AND set-piece type is decided.
    SetPiece(SetPieceKind),
}

/// Sub-discriminant for `TacticState::SetPiece`. Eleven variants covering
/// all standard set-piece classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SetPieceKind {
    KickOff,
    GoalKick,
    GoalKickOpponent,
    CornerFor,
    CornerAgainst,
    FreeKickFor,
    FreeKickAgainst,
    ThrowInFor,
    ThrowInAgainst,
    PenaltyFor,
    PenaltyAgainst,
}

// ---------------------------------------------------------------------------
// TeamTacticState — canonical carrier
// ---------------------------------------------------------------------------

/// Canonical tactic state for one team. Stored as `[TeamTacticState; 2]` in
/// `MatchState` (index 0 = home, index 1 = away).
///
/// The `entry_tick` is the tick at which the FSM entered `state`. Used by the
/// heartbeat to measure time-in-state without storing a separate counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamTacticState {
    /// Current tactic state. `pub(crate)` per Codex P1 type-design finding:
    /// external code must NOT be able to write `state` without atomically
    /// updating `entry_tick`. The only valid mutation paths are `initial()`
    /// and `transition()`. Field is `pub(crate)` rather than fully private so
    /// the canonical encoder (same crate) can read it without an accessor.
    pub(crate) state: TacticState,
    /// Tick at which `state` was entered. Starts at `Tick::ZERO` for the
    /// default initial state. See `state` field comment for the same
    /// rationale on visibility.
    pub(crate) entry_tick: Tick,
}

impl TeamTacticState {
    /// Default initial state: MidBlock entered at Tick::ZERO.
    /// Both teams start in the neutral mid-block before kick-off.
    #[must_use]
    pub fn initial() -> TeamTacticState {
        TeamTacticState {
            state: TacticState::MidBlock,
            entry_tick: Tick::ZERO,
        }
    }

    /// Current tactic state.
    #[must_use]
    pub fn state(&self) -> TacticState {
        self.state
    }

    /// Tick at which the current state was entered.
    #[must_use]
    pub fn entry_tick(&self) -> Tick {
        self.entry_tick
    }

    /// Transition to `new_state` at `now_tick`. Returns a new `TeamTacticState`
    /// (the old one is consumed — canonical immutability preserved). The only
    /// way to set both fields atomically; bypassing this method via direct
    /// field assignment is prevented by the `pub(crate)` visibility.
    #[must_use]
    pub fn transition(self, new_state: TacticState, now_tick: Tick) -> TeamTacticState {
        TeamTacticState {
            state: new_state,
            entry_tick: now_tick,
        }
    }
}

// ---------------------------------------------------------------------------
// Local archetype parameter stubs (T1-2b-ii; promoted to fw-content at T1-2b-iii)
// ---------------------------------------------------------------------------

/// Press intensity levels. T1-2b-ii local stub; moves to `TacticalArchetype`
/// at T1-2b-iii when the BT runner consumes it from RON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PressIntensity {
    None,
    Low,
    Default,
    High,
}

/// Counter-attack intent levels. T1-2b-ii local stub; promoted to
/// `TacticalArchetype` at T1-2b-iii.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CounterIntent {
    None,
    Default,
    High,
}

/// Archetype parameter bag used by the transition function. T1-2b-ii uses
/// hardcoded defaults; T1-2b-iii reads these from `fw-content::TacticalArchetype`.
#[derive(Debug, Clone, Copy)]
pub struct ArchetypeParams {
    /// Default defensive shape when no explicit trigger applies.
    pub default_in_defence_state: TacticState,
    /// Press intensity threshold for HighPress transitions.
    pub press_intensity: PressIntensity,
    /// Counter-attack aggressiveness.
    pub counter_intent: CounterIntent,
}

impl ArchetypeParams {
    /// Direct-pressing defaults: MidBlock default, High press, Default counter.
    #[must_use]
    pub fn direct_pressing() -> ArchetypeParams {
        ArchetypeParams {
            default_in_defence_state: TacticState::MidBlock,
            press_intensity: PressIntensity::High,
            counter_intent: CounterIntent::Default,
        }
    }

    /// Low-block-counter defaults: LowBlock default, None press, High counter.
    #[must_use]
    pub fn low_block_counter() -> ArchetypeParams {
        ArchetypeParams {
            default_in_defence_state: TacticState::LowBlock,
            press_intensity: PressIntensity::None,
            counter_intent: CounterIntent::High,
        }
    }
}

// ---------------------------------------------------------------------------
// Local TacticEvent stub (T1-2b-ii; reconciled with MatchEvent at T1-4)
// ---------------------------------------------------------------------------

/// Event variants that the tactic-FSM transition table consumes. This is a
/// **local stub** — T1-4's `MatchEvent` enum supersedes it. At T1-4, every
/// place that emits `TacticEvent` will emit a `MatchEvent` variant instead;
/// this enum will be removed or aliased.
///
/// Only the events needed by the transition table are listed (8 variants).
///
/// ## T1-4 reconciliation TODO (Codex P3 — self-review)
///
/// When `MatchEvent` lands, each variant here maps as follows. If a mapping
/// is missed, the corresponding transition silently never fires:
/// - `BallOutOfPlay { kind }` ← `MatchEvent::BallOutOfPlay { kind }` (direct rename).
/// - `BallInPlay` ← `MatchEvent::BallInPlay` (direct rename).
/// - `PossessionLost { recovery_likely }` ← `MatchEvent::PossessionLost { ... }`
///   (the `recovery_likely` boolean is derived from contest-radius proximity
///   at the loss tick; T1-2b-iii's spatial layer computes this).
/// - `BallRecovered { opponent_shape_broken }` ← `MatchEvent::BallRecovered { ... }`
///   (`opponent_shape_broken` derives from `mean_opponent_x > halfway`).
/// - `PressTimeoutExpired` — **TIMER-DERIVED**; no `MatchEvent` counterpart
///   today. T1-4 must either add a timer-tick event class OR move the timeout
///   check into the heartbeat path (it would fire at the next 2 Hz boundary
///   instead of at the 5s mark — tighten before signing off).
/// - `CounterWindowClosed` — same TIMER-DERIVED concern as `PressTimeoutExpired`.
/// - `Goal` ← `MatchEvent::Goal { scorer_team, ... }` — **N.B.** current
///   variant is field-less; resets BOTH teams identically per spec. The
///   `scorer_team` field on `MatchEvent::Goal` will let the reconciliation
///   make this distinction explicit (the conceding team has different counter
///   needs than the scoring team). Re-audit the transition arm at T1-4.
/// - `HalfTime` ← `MatchEvent::HalfTime` (direct rename).
///
/// T1-4a reconciliation note: `TacticEvent` stays sim-internal (no serde
/// needed) — it drives FSM transitions only, not the canonical event stream.
/// The canonical event stream uses `MatchEvent` (in `fw_content::event`).
/// `TacticEvent::Goal` does NOT yet have a corresponding `MatchEvent::Goal`
/// emission — that wiring waits on T1-9/T2 ball-in-net detection. The
/// `MatchEvent::Goal` variant + canonical encoder + serde round-trip + a
/// direct `encode_match_event(Goal)` unit test all ship in T1-4a as
/// forward-compat scaffolding; the call site that pushes the event is the
/// next-phase delivery. See `fw_match_sim::lib`'s deletion comment for
/// `apply_tactic_event_with_emission` for the call-site protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TacticEvent {
    /// Ball went out of play; set-piece type decided.
    BallOutOfPlay { kind: SetPieceKind },
    /// Ball returned to play from a set-piece restart.
    BallInPlay,
    /// Possession lost; `recovery_likely` indicates whether the ball is
    /// contested nearby (true) vs. clearly conceded (false).
    PossessionLost { recovery_likely: bool },
    /// Ball regained from opponent; `opponent_shape_broken` indicates
    /// whether the opponent's mean-x is beyond halfway (Bauer-and-Anzer).
    BallRecovered { opponent_shape_broken: bool },
    /// 5s (300 ticks) elapsed since HighPress entry without ball recovery.
    PressTimeoutExpired,
    /// 4s (240 ticks) elapsed since CounterAttack entry, or shot taken.
    CounterWindowClosed,
    /// A goal was scored.
    Goal,
    /// Half-time; states reset.
    HalfTime,
}

// ---------------------------------------------------------------------------
// Transition function
// ---------------------------------------------------------------------------

/// Constants from the spec.
///
/// HighPress timeout: 5s at 60 Hz = 300 ticks (Bauer-and-Anzer empirical
/// constant). Archetype-overrideable in T1-2b-iii; fixed for T1.
pub const PRESS_TIMEOUT_TICKS: i64 = 300;

/// Anti-thrash guard: HighPress re-entry requires >600 ticks since prior
/// HighPress entry (prevents oscillation from rapid PossessionLost events).
pub const HIGH_PRESS_REENTRY_COOLDOWN_TICKS: i64 = 600;

/// HighPress 10s drift limit for the heartbeat: 600 ticks at 60 Hz.
pub const HIGH_PRESS_HEARTBEAT_TIMEOUT_TICKS: i64 = 600;

/// 30-tick heartbeat interval (2 Hz at 60 Hz integration rate).
pub const HEARTBEAT_INTERVAL_TICKS: i64 = 30;

/// Apply one event-driven transition. Pure function — no RNG, no side
/// effects. Returns the new `TeamTacticState` (may be identical to `current`
/// if no transition fires).
///
/// Per `docs/specs/tactic-fsm.md` transition table.
#[must_use]
pub fn apply_event(
    current: TeamTacticState,
    archetype: &ArchetypeParams,
    event: TacticEvent,
    now_tick: Tick,
) -> TeamTacticState {
    match event {
        // any → SetPiece (always fires on BallOutOfPlay)
        TacticEvent::BallOutOfPlay { kind } => {
            current.transition(TacticState::SetPiece(kind), now_tick)
        }

        // SetPiece → archetype default (always on BallInPlay)
        TacticEvent::BallInPlay => {
            if matches!(current.state, TacticState::SetPiece(_)) {
                current.transition(archetype.default_in_defence_state, now_tick)
            } else {
                current
            }
        }

        // MidBlock or LowBlock → HighPress
        // Guard: recovery_likely AND press_intensity ≥ High AND
        //        ticks-since-last-entry > 600 (anti-thrash)
        TacticEvent::PossessionLost { recovery_likely } => {
            match current.state {
                TacticState::MidBlock | TacticState::LowBlock => {
                    if recovery_likely
                        && archetype.press_intensity >= PressIntensity::High
                        && (now_tick.to_raw() - current.entry_tick.to_raw())
                            > HIGH_PRESS_REENTRY_COOLDOWN_TICKS
                    {
                        current.transition(TacticState::HighPress, now_tick)
                    } else if !recovery_likely {
                        // PossessionLost with recovery_likely=false AND in MidBlock
                        // → conservative fallback to LowBlock
                        if current.state == TacticState::MidBlock {
                            current.transition(TacticState::LowBlock, now_tick)
                        } else {
                            current
                        }
                    } else {
                        current
                    }
                }
                _ => current,
            }
        }

        // any → CounterAttack (when opponent shape broken)
        // HighPress: always (just recovered against a broken shape)
        // MidBlock / LowBlock: only if counter_intent ≥ Default
        TacticEvent::BallRecovered {
            opponent_shape_broken,
        } => {
            if !opponent_shape_broken {
                return current;
            }
            match current.state {
                TacticState::HighPress => current.transition(TacticState::CounterAttack, now_tick),
                TacticState::MidBlock | TacticState::LowBlock => {
                    if archetype.counter_intent >= CounterIntent::Default {
                        current.transition(TacticState::CounterAttack, now_tick)
                    } else {
                        current
                    }
                }
                _ => current,
            }
        }

        // HighPress → MidBlock on press timeout (always)
        TacticEvent::PressTimeoutExpired => {
            if current.state == TacticState::HighPress {
                current.transition(TacticState::MidBlock, now_tick)
            } else {
                current
            }
        }

        // CounterAttack → archetype default on window close (always)
        TacticEvent::CounterWindowClosed => {
            if current.state == TacticState::CounterAttack {
                current.transition(archetype.default_in_defence_state, now_tick)
            } else {
                current
            }
        }

        // any → MidBlock on Goal (reset on goal; kick-off restart is structural)
        TacticEvent::Goal => current.transition(TacticState::MidBlock, now_tick),

        // any → MidBlock on HalfTime (state resets at the break)
        TacticEvent::HalfTime => current.transition(TacticState::MidBlock, now_tick),
    }
}

// ---------------------------------------------------------------------------
// Heartbeat
// ---------------------------------------------------------------------------

/// Run the 2 Hz heartbeat drift check for one team. Returns the **complete
/// new `TeamTacticState`** (state + entry_tick atomically updated) if a
/// drift-triggered transition fires, `None` otherwise.
///
/// Codex P2 from self-review: the prior signature returned `Option<TacticState>`
/// and required the caller to remember to invoke `transition()` to bump
/// `entry_tick`. A caller that did `state.team_tactic_states[0].state = new`
/// directly would leave `entry_tick` stale and the heartbeat would re-fire
/// every 30 ticks forever. Returning the full `TeamTacticState` here moves
/// the entry_tick discipline INTO the function instead of relying on caller
/// memory.
///
/// Called every 30 ticks from `tick_match`. Pure predicate — no RNG.
///
/// T1-2b-ii implements only the HighPress-timeout-10s rule (the only rule
/// with sufficient inputs at this task row). Spatial drift rules
/// (`own_mean_x < 30`, scoreline lead) defer to T1-2b-iii when spatial
/// state is available.
#[must_use]
pub fn heartbeat_check(current: &TeamTacticState, now_tick: Tick) -> Option<TeamTacticState> {
    if current.state == TacticState::HighPress {
        let ticks_in_state = now_tick.to_raw() - current.entry_tick.to_raw();
        if ticks_in_state > HIGH_PRESS_HEARTBEAT_TIMEOUT_TICKS {
            return Some(current.transition(TacticState::MidBlock, now_tick));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests — Chunk 1 + 2 + 3 (types, transitions, heartbeat)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_core::Tick;

    // ------- Chunk 1 RED → GREEN: type round-trips -------

    #[test]
    fn default_initial_state_is_midblock_at_tick_zero() {
        let s = TeamTacticState::initial();
        assert_eq!(s.state, TacticState::MidBlock);
        assert_eq!(s.entry_tick, Tick::ZERO);
    }

    #[test]
    fn transition_updates_state_and_entry_tick() {
        let s = TeamTacticState::initial();
        let now = Tick::from_raw(100);
        let s2 = s.transition(TacticState::HighPress, now);
        assert_eq!(s2.state, TacticState::HighPress);
        assert_eq!(s2.entry_tick, now);
        // original unchanged (returned by value)
        assert_eq!(s.state, TacticState::MidBlock);
    }

    #[test]
    fn setpiece_kind_equality() {
        assert_eq!(SetPieceKind::KickOff, SetPieceKind::KickOff);
        assert_ne!(SetPieceKind::CornerFor, SetPieceKind::CornerAgainst);
    }

    #[test]
    fn tacticstate_setpiece_equality_on_same_kind() {
        let a = TacticState::SetPiece(SetPieceKind::PenaltyFor);
        let b = TacticState::SetPiece(SetPieceKind::PenaltyFor);
        assert_eq!(a, b);
    }

    #[test]
    fn tacticstate_setpiece_inequality_on_different_kind() {
        let a = TacticState::SetPiece(SetPieceKind::PenaltyFor);
        let b = TacticState::SetPiece(SetPieceKind::PenaltyAgainst);
        assert_ne!(a, b);
    }

    // ------- Chunk 2 RED → GREEN: transition table rows -------

    fn at(state: TacticState, tick_raw: i64) -> TeamTacticState {
        TeamTacticState {
            state,
            entry_tick: Tick::from_raw(tick_raw),
        }
    }

    #[test]
    fn ball_out_of_play_always_transitions_to_setpiece() {
        let current = at(TacticState::MidBlock, 0);
        let archetype = ArchetypeParams::direct_pressing();
        let result = apply_event(
            current,
            &archetype,
            TacticEvent::BallOutOfPlay {
                kind: SetPieceKind::CornerFor,
            },
            Tick::from_raw(120),
        );
        assert_eq!(result.state, TacticState::SetPiece(SetPieceKind::CornerFor));
    }

    #[test]
    fn ball_out_of_play_from_highpress_still_transitions_to_setpiece() {
        let current = at(TacticState::HighPress, 0);
        let archetype = ArchetypeParams::direct_pressing();
        let result = apply_event(
            current,
            &archetype,
            TacticEvent::BallOutOfPlay {
                kind: SetPieceKind::GoalKick,
            },
            Tick::from_raw(200),
        );
        assert_eq!(result.state, TacticState::SetPiece(SetPieceKind::GoalKick));
    }

    #[test]
    fn ball_in_play_from_setpiece_transitions_to_archetype_default() {
        let current = at(TacticState::SetPiece(SetPieceKind::FreeKickFor), 100);
        let archetype = ArchetypeParams::direct_pressing();
        let result = apply_event(
            current,
            &archetype,
            TacticEvent::BallInPlay,
            Tick::from_raw(200),
        );
        assert_eq!(result.state, archetype.default_in_defence_state);
    }

    #[test]
    fn ball_in_play_from_non_setpiece_is_noop() {
        let current = at(TacticState::HighPress, 50);
        let archetype = ArchetypeParams::direct_pressing();
        let result = apply_event(
            current,
            &archetype,
            TacticEvent::BallInPlay,
            Tick::from_raw(200),
        );
        // BallInPlay only fires the transition when coming FROM SetPiece
        assert_eq!(result.state, TacticState::HighPress);
    }

    #[test]
    fn possession_lost_recovery_likely_high_press_fires_after_cooldown() {
        // entry_tick=0, now_tick=700 → ticks_since_entry=700 > 600 cooldown
        let current = at(TacticState::MidBlock, 0);
        let archetype = ArchetypeParams::direct_pressing(); // High press
        let result = apply_event(
            current,
            &archetype,
            TacticEvent::PossessionLost {
                recovery_likely: true,
            },
            Tick::from_raw(700),
        );
        assert_eq!(result.state, TacticState::HighPress);
    }

    #[test]
    fn possession_lost_recovery_likely_high_press_blocked_by_cooldown() {
        // entry_tick=0, now_tick=300 → 300 < 600 cooldown → no HighPress
        let current = at(TacticState::MidBlock, 0);
        let archetype = ArchetypeParams::direct_pressing();
        let result = apply_event(
            current,
            &archetype,
            TacticEvent::PossessionLost {
                recovery_likely: true,
            },
            Tick::from_raw(300),
        );
        // Guard failed; stays MidBlock
        assert_eq!(result.state, TacticState::MidBlock);
    }

    #[test]
    fn possession_lost_recovery_false_from_midblock_drops_to_lowblock() {
        let current = at(TacticState::MidBlock, 0);
        let archetype = ArchetypeParams::direct_pressing();
        let result = apply_event(
            current,
            &archetype,
            TacticEvent::PossessionLost {
                recovery_likely: false,
            },
            Tick::from_raw(100),
        );
        assert_eq!(result.state, TacticState::LowBlock);
    }

    #[test]
    fn possession_lost_recovery_false_from_lowblock_is_noop() {
        let current = at(TacticState::LowBlock, 0);
        let archetype = ArchetypeParams::direct_pressing();
        let result = apply_event(
            current,
            &archetype,
            TacticEvent::PossessionLost {
                recovery_likely: false,
            },
            Tick::from_raw(100),
        );
        // Already in LowBlock; no further drop
        assert_eq!(result.state, TacticState::LowBlock);
    }

    #[test]
    fn possession_lost_low_press_archetype_does_not_transition_to_highpress() {
        let current = at(TacticState::MidBlock, 0);
        let archetype = ArchetypeParams::low_block_counter(); // press = None
        let result = apply_event(
            current,
            &archetype,
            TacticEvent::PossessionLost {
                recovery_likely: true,
            },
            Tick::from_raw(700),
        );
        // press_intensity = None < High → guard fails
        assert_ne!(result.state, TacticState::HighPress);
    }

    #[test]
    fn ball_recovered_from_highpress_with_broken_shape_transitions_to_counter() {
        let current = at(TacticState::HighPress, 0);
        let archetype = ArchetypeParams::direct_pressing();
        let result = apply_event(
            current,
            &archetype,
            TacticEvent::BallRecovered {
                opponent_shape_broken: true,
            },
            Tick::from_raw(150),
        );
        assert_eq!(result.state, TacticState::CounterAttack);
    }

    #[test]
    fn ball_recovered_with_intact_shape_is_noop() {
        let current = at(TacticState::HighPress, 0);
        let archetype = ArchetypeParams::direct_pressing();
        let result = apply_event(
            current,
            &archetype,
            TacticEvent::BallRecovered {
                opponent_shape_broken: false,
            },
            Tick::from_raw(150),
        );
        assert_eq!(result.state, TacticState::HighPress);
    }

    #[test]
    fn ball_recovered_from_midblock_with_counter_intent_transitions() {
        let current = at(TacticState::MidBlock, 0);
        let archetype = ArchetypeParams::direct_pressing(); // counter_intent = Default ≥ Default
        let result = apply_event(
            current,
            &archetype,
            TacticEvent::BallRecovered {
                opponent_shape_broken: true,
            },
            Tick::from_raw(100),
        );
        assert_eq!(result.state, TacticState::CounterAttack);
    }

    #[test]
    fn press_timeout_from_highpress_drops_to_midblock() {
        let current = at(TacticState::HighPress, 0);
        let archetype = ArchetypeParams::direct_pressing();
        let result = apply_event(
            current,
            &archetype,
            TacticEvent::PressTimeoutExpired,
            Tick::from_raw(300),
        );
        assert_eq!(result.state, TacticState::MidBlock);
    }

    #[test]
    fn press_timeout_from_midblock_is_noop() {
        let current = at(TacticState::MidBlock, 0);
        let archetype = ArchetypeParams::direct_pressing();
        let result = apply_event(
            current,
            &archetype,
            TacticEvent::PressTimeoutExpired,
            Tick::from_raw(300),
        );
        assert_eq!(result.state, TacticState::MidBlock);
    }

    #[test]
    fn counter_window_closed_from_counter_transitions_to_archetype_default() {
        let current = at(TacticState::CounterAttack, 0);
        let archetype = ArchetypeParams::low_block_counter(); // default = LowBlock
        let result = apply_event(
            current,
            &archetype,
            TacticEvent::CounterWindowClosed,
            Tick::from_raw(240),
        );
        assert_eq!(result.state, TacticState::LowBlock);
    }

    #[test]
    fn goal_resets_any_state_to_midblock() {
        for state in [
            TacticState::HighPress,
            TacticState::LowBlock,
            TacticState::CounterAttack,
            TacticState::SetPiece(SetPieceKind::PenaltyFor),
        ] {
            let current = at(state, 0);
            let archetype = ArchetypeParams::direct_pressing();
            let result = apply_event(current, &archetype, TacticEvent::Goal, Tick::from_raw(500));
            assert_eq!(
                result.state,
                TacticState::MidBlock,
                "Goal from {state:?} didn't reset to MidBlock"
            );
        }
    }

    #[test]
    fn halftime_resets_any_state_to_midblock() {
        for state in [
            TacticState::HighPress,
            TacticState::LowBlock,
            TacticState::CounterAttack,
        ] {
            let current = at(state, 0);
            let archetype = ArchetypeParams::direct_pressing();
            let result = apply_event(
                current,
                &archetype,
                TacticEvent::HalfTime,
                Tick::from_raw(2700),
            );
            assert_eq!(
                result.state,
                TacticState::MidBlock,
                "HalfTime from {state:?} didn't reset to MidBlock"
            );
        }
    }

    // ------- Chunk 3 RED → GREEN: heartbeat_check -------

    #[test]
    fn heartbeat_highpress_over_600_ticks_returns_midblock() {
        let current = TeamTacticState {
            state: TacticState::HighPress,
            entry_tick: Tick::ZERO,
        };
        // 601 ticks > 600 threshold. heartbeat_check now returns the full
        // post-transition TeamTacticState (Codex P2 self-review refactor).
        let now = Tick::from_raw(601);
        let result = heartbeat_check(&current, now).expect("heartbeat must fire at 601 ticks");
        assert_eq!(result.state(), TacticState::MidBlock);
        assert_eq!(
            result.entry_tick(),
            now,
            "entry_tick must advance to now_tick on transition"
        );
    }

    #[test]
    fn heartbeat_highpress_at_600_ticks_returns_none() {
        let current = TeamTacticState {
            state: TacticState::HighPress,
            entry_tick: Tick::ZERO,
        };
        // 600 ticks is NOT > 600 (strictly greater required)
        let result = heartbeat_check(&current, Tick::from_raw(600));
        assert_eq!(result, None);
    }

    #[test]
    fn heartbeat_highpress_at_599_ticks_returns_none() {
        let current = TeamTacticState {
            state: TacticState::HighPress,
            entry_tick: Tick::ZERO,
        };
        let result = heartbeat_check(&current, Tick::from_raw(599));
        assert_eq!(result, None);
    }

    #[test]
    fn heartbeat_midblock_never_fires_at_any_tick() {
        let current = TeamTacticState {
            state: TacticState::MidBlock,
            entry_tick: Tick::ZERO,
        };
        // Even after a very long run, MidBlock heartbeat doesn't fire (T1-2b-ii scope)
        for tick_raw in [30, 300, 600, 1800, 5400] {
            let result = heartbeat_check(&current, Tick::from_raw(tick_raw));
            assert_eq!(
                result, None,
                "heartbeat should not fire for MidBlock at tick {tick_raw}"
            );
        }
    }

    #[test]
    fn heartbeat_is_rng_free() {
        // The heartbeat is a pure predicate. We verify this structurally:
        // same inputs produce same output across multiple calls (no internal
        // state mutation that would signal RNG use).
        let current = TeamTacticState {
            state: TacticState::HighPress,
            entry_tick: Tick::ZERO,
        };
        let tick = Tick::from_raw(700);
        let a = heartbeat_check(&current, tick);
        let b = heartbeat_check(&current, tick);
        let c = heartbeat_check(&current, tick);
        assert_eq!(a, b);
        assert_eq!(b, c);
    }

    #[test]
    fn apply_event_is_rng_free() {
        // Same inputs → same output (pure function check, no RNG divergence)
        let current = at(TacticState::MidBlock, 0);
        let archetype = ArchetypeParams::direct_pressing();
        let tick = Tick::from_raw(700);
        let a = apply_event(
            current,
            &archetype,
            TacticEvent::PossessionLost {
                recovery_likely: true,
            },
            tick,
        );
        let b = apply_event(
            current,
            &archetype,
            TacticEvent::PossessionLost {
                recovery_likely: true,
            },
            tick,
        );
        assert_eq!(a.state, b.state);
        assert_eq!(a.entry_tick, b.entry_tick);
    }
}

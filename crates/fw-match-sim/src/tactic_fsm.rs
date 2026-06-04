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
/// hardcoded defaults; T2-1a reads these from `fw-content::TacticalArchetype`
/// via `archetype_params_for` bridge + caches on `MatchState`.
///
/// T2-1a added `Serialize` / `Deserialize` / `PartialEq` / `Eq` derives so
/// `MatchState` (which now caches per-team `ArchetypeParams` as sidecar
/// fields) still satisfies its own serde + equality derives. The values are
/// recomputed from the canonical `home_archetype_id` / `away_archetype_id`
/// strings at MatchState construction time; serde on this type is for
/// derive-completeness, NOT for round-trip stability (the canonical encoder
/// in `canonical.rs` encodes only the IDs, not the resolved params).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
// T2-1a: TacticalArchetype → ArchetypeParams bridge
//
// `TacticalArchetype` (content-pack type, in `fw_content`) holds the
// content-author surface fields: `press_radius_metres` (u32) +
// `buildup_speed_factor_bps` (u16) + `formation` (Vec<FormationSlot>).
// `ArchetypeParams` (this module) holds the sim-runtime enums consumed by
// `apply_event`. The bridge function below maps the former to the latter.
//
// Bridge thresholds are deliberately **preserve-current-behavior** wide
// 2-bucket buckets: the 2 existing RON archetypes (attacking-fullback +
// low-block-counter) round-trip through the bridge to their existing
// hardcoded `ArchetypeParams::direct_pressing()` / `low_block_counter()`
// values exactly. This minimizes canonical-hash drift on T2-1a's
// per-team-archetype foundation row: smoke seed drift is SCHEMA-ONLY
// (the 2 new encoded fields appended to MatchState), not behavior-driven.
//
// T2-1d (xG / personality coefficient re-fit) is the row that earns the
// right to refine these into 4-bucket thresholds calibrated against a
// 100-match corpus. Today's thresholds preserve the headroom + document
// the intent.
// ---------------------------------------------------------------------------

/// Map a content-pack `TacticalArchetype` to its sim-runtime `ArchetypeParams`.
///
/// **Preserve-current-behavior bridge**: the 2 existing archetypes
/// (`attacking-fullback` press_radius=30 buildup=9000 + `low-block-counter`
/// press_radius=15 buildup=11500) round-trip through this function to the
/// existing hardcoded `ArchetypeParams::direct_pressing()` /
/// `low_block_counter()` values. Verified by `tactic_fsm::tests::
/// archetype_params_for_*` unit tests.
///
/// Thresholds:
///
/// | TacticalArchetype field                | ArchetypeParams field           | Mapping       |
/// |----------------------------------------|----------------------------------|---------------|
/// | `press_radius_metres ≤ 20`             | `press_intensity`                | `None`        |
/// | `press_radius_metres > 20`             | `press_intensity`                | `High`        |
/// | `buildup_speed_factor_bps ≥ 11000`     | `counter_intent`                 | `High`        |
/// | `buildup_speed_factor_bps < 11000`     | `counter_intent`                 | `Default`     |
/// | `press_radius_metres ≤ 20`             | `default_in_defence_state`       | `LowBlock`    |
/// | `press_radius_metres > 20`             | `default_in_defence_state`       | `MidBlock`    |
///
/// T2-1d will refine these into 4-bucket thresholds (None / Low / Default / High
/// for press_intensity, etc.) calibrated against a 100-match xG corpus per
/// `docs/design/xg-coefficients.md`. Today's wide 2-bucket thresholds are the
/// foundation that T2-1d builds on.
#[must_use]
pub fn archetype_params_for(arch: &fw_content::TacticalArchetype) -> ArchetypeParams {
    let press_intensity = if arch.press_radius_metres > 20 {
        PressIntensity::High
    } else {
        PressIntensity::None
    };
    let counter_intent = if arch.buildup_speed_factor_bps >= 11_000 {
        CounterIntent::High
    } else {
        CounterIntent::Default
    };
    // FUN-TS2d: decouple line-height from press-intensity.
    // When `line_height_metres` is set by the content author, use it directly
    // for `default_in_defence_state`; otherwise fall back to the legacy coupled
    // rule (`press_radius_metres <= 20 → LowBlock`, else `MidBlock`).
    let default_in_defence_state = match arch.line_height_metres {
        Some(h) if h < 20 => TacticState::LowBlock,
        Some(h) if h > 35 => TacticState::HighPress,
        Some(_) => TacticState::MidBlock,
        None => {
            // Legacy coupled rule: preserves existing archetype behaviour.
            if arch.press_radius_metres <= 20 {
                TacticState::LowBlock
            } else {
                TacticState::MidBlock
            }
        }
    };
    ArchetypeParams {
        default_in_defence_state,
        press_intensity,
        counter_intent,
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
///
/// T1-23: `u32` (was `i64`) to align with `Tick::checked_elapsed_since`'s
/// return type — durations are non-negative by definition, so u32 is the
/// honest signature + lets the comparison site type-check without an
/// `as i64` cast on the elapsed-tick count.
pub const HIGH_PRESS_REENTRY_COOLDOWN_TICKS: u32 = 600;

/// HighPress 10s drift limit for the heartbeat: 600 ticks at 60 Hz.
///
/// T1-23: `u32` (was `i64`) — same rationale as `HIGH_PRESS_REENTRY_COOLDOWN_TICKS`.
pub const HIGH_PRESS_HEARTBEAT_TIMEOUT_TICKS: u32 = 600;

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
                        // T1-23 (post-Codex Finding #1): `Tick::checked_elapsed_since`
                        // funnels the previously-raw `now.to_raw() - entry.to_raw()`
                        // subtraction through the §11 panic-on-underflow policy.
                        // If entry_tick somehow lives in the future (a real
                        // cooldown-math invariant violation), this now fails loudly
                        // at the violation site instead of silently producing
                        // garbage from negative-i64-then-cast arithmetic.
                        && now_tick.checked_elapsed_since(current.entry_tick)
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
        // T1-23 (post-Codex Finding #1): `Tick::checked_elapsed_since` replaces
        // the previously-raw `now.to_raw() - entry.to_raw()` subtraction. Same
        // §11 panic-on-underflow rationale as the PossessionLost branch above.
        let ticks_in_state = now_tick.checked_elapsed_since(current.entry_tick);
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

    // ------------------------------------------------------------------
    // T2-1a: archetype_params_for bridge tests (preserve-current-behavior
    // verification)
    // ------------------------------------------------------------------

    fn attacking_fullback_fixture() -> fw_content::TacticalArchetype {
        fw_content::TacticalArchetype {
            id: "fwh.core:archetype.attacking-fullback".into(),
            formation: vec![],
            press_radius_metres: 30,
            line_height_metres: None, // derive from press_radius (legacy coupled rule)
            buildup_speed_factor_bps: 9_000,
        }
    }

    fn low_block_counter_fixture() -> fw_content::TacticalArchetype {
        fw_content::TacticalArchetype {
            id: "fwh.core:archetype.low-block-counter".into(),
            formation: vec![],
            press_radius_metres: 15,
            line_height_metres: None,
            buildup_speed_factor_bps: 11_500,
        }
    }

    /// LOAD-BEARING: `attacking-fullback` (the FW v1 `direct-pressing.yaml`
    /// rename) MUST bridge to the EXACT same ArchetypeParams as the existing
    /// hardcoded `ArchetypeParams::direct_pressing()`. Mismatch = T2-1a's
    /// "smoke seed canonical hash drift is schema-only" claim is broken +
    /// the rebaseline should be flagged for re-investigation.
    #[test]
    fn archetype_params_for_attacking_fullback_matches_direct_pressing_hardcoded() {
        let archetype = attacking_fullback_fixture();
        let bridged = archetype_params_for(&archetype);
        let hardcoded = ArchetypeParams::direct_pressing();
        assert_eq!(
            bridged.default_in_defence_state,
            hardcoded.default_in_defence_state
        );
        assert_eq!(bridged.press_intensity, hardcoded.press_intensity);
        assert_eq!(bridged.counter_intent, hardcoded.counter_intent);
    }

    /// LOAD-BEARING: `low-block-counter` MUST bridge to the EXACT same
    /// ArchetypeParams as `ArchetypeParams::low_block_counter()`. Same
    /// rationale as the attacking-fullback test above.
    #[test]
    fn archetype_params_for_low_block_counter_matches_hardcoded() {
        let archetype = low_block_counter_fixture();
        let bridged = archetype_params_for(&archetype);
        let hardcoded = ArchetypeParams::low_block_counter();
        assert_eq!(
            bridged.default_in_defence_state,
            hardcoded.default_in_defence_state
        );
        assert_eq!(bridged.press_intensity, hardcoded.press_intensity);
        assert_eq!(bridged.counter_intent, hardcoded.counter_intent);
    }

    /// Threshold-boundary test: press_radius_metres == 20 exactly is in the
    /// "None press / LowBlock default" bucket. Off-by-one in the bridge
    /// (using `<` instead of `≤` for the None check) would trip this.
    #[test]
    fn archetype_params_for_at_press_radius_20_yields_none_lowblock() {
        let archetype = fw_content::TacticalArchetype {
            id: "fwh.test:archetype.threshold-20".into(),
            formation: vec![],
            press_radius_metres: 20,
            line_height_metres: None,
            buildup_speed_factor_bps: 10_000,
        };
        let p = archetype_params_for(&archetype);
        assert_eq!(p.press_intensity, PressIntensity::None);
        assert_eq!(p.default_in_defence_state, TacticState::LowBlock);
        assert_eq!(p.counter_intent, CounterIntent::Default);
    }

    /// Threshold-boundary test: press_radius_metres == 21 is just over the
    /// boundary; press goes High + default goes MidBlock.
    #[test]
    fn archetype_params_for_at_press_radius_21_yields_high_midblock() {
        let archetype = fw_content::TacticalArchetype {
            id: "fwh.test:archetype.threshold-21".into(),
            formation: vec![],
            press_radius_metres: 21,
            line_height_metres: None,
            buildup_speed_factor_bps: 10_000,
        };
        let p = archetype_params_for(&archetype);
        assert_eq!(p.press_intensity, PressIntensity::High);
        assert_eq!(p.default_in_defence_state, TacticState::MidBlock);
    }

    /// T2-1b: each of the 6 football-canonical archetypes authored at
    /// T2-1b must bridge to the expected `(press_intensity,
    /// counter_intent, default_in_defence_state)` tuple per the tactical-
    /// shape parameter design table in the T2-1b MEMORY spec. Table-driven
    /// so a single tuple flip surfaces as a single row failure (not a
    /// whole-test failure).
    ///
    /// Mutation discriminator: if the bridge thresholds drifted (e.g.
    /// `press_radius_metres > 20` → `>= 20`), at least one row would
    /// flip its bucket + the assertion would fail.
    #[test]
    fn archetype_params_for_t2_1b_and_t2_1c_archetypes_yield_expected_buckets() {
        use fw_content::ContentStore;
        use std::path::PathBuf;

        let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("content");
        let store = ContentStore::load_sources(&content_root).expect("ContentStore load failed");

        // (archetype_id, expected_press, expected_counter, expected_default_state)
        let expectations: &[(&str, PressIntensity, CounterIntent, TacticState)] = &[
            (
                "fwh.core:archetype.high-press-possession",
                PressIntensity::High,
                CounterIntent::Default,
                TacticState::MidBlock,
            ),
            (
                "fwh.core:archetype.wing-overload",
                PressIntensity::High,
                CounterIntent::Default,
                TacticState::MidBlock,
            ),
            (
                "fwh.core:archetype.gegen-press",
                PressIntensity::High,
                CounterIntent::Default,
                TacticState::MidBlock,
            ),
            (
                "fwh.core:archetype.park-the-bus",
                PressIntensity::None,
                CounterIntent::High,
                TacticState::LowBlock,
            ),
            (
                "fwh.core:archetype.tiki-taka",
                PressIntensity::High,
                CounterIntent::Default,
                TacticState::MidBlock,
            ),
            (
                "fwh.core:archetype.route-one",
                PressIntensity::None,
                CounterIntent::High,
                TacticState::LowBlock,
            ),
            // T2-1c rows below (8 mixed-category archetypes added 2026-05-17):
            (
                "fwh.core:archetype.lopsided-right-overload",
                PressIntensity::High,
                CounterIntent::Default,
                TacticState::MidBlock,
            ),
            (
                "fwh.core:archetype.lopsided-left-overload",
                PressIntensity::High,
                CounterIntent::Default,
                TacticState::MidBlock,
            ),
            (
                "fwh.core:archetype.anti-tiki-taka",
                PressIntensity::High,
                CounterIntent::Default,
                TacticState::MidBlock,
            ),
            // anti-high-press introduces the NEW (High, High, MidBlock)
            // bucket combination — first archetype to populate it.
            (
                "fwh.core:archetype.anti-high-press",
                PressIntensity::High,
                CounterIntent::High,
                TacticState::MidBlock,
            ),
            (
                "fwh.core:archetype.ultra-attacking-no-cb",
                PressIntensity::High,
                CounterIntent::Default,
                TacticState::MidBlock,
            ),
            (
                "fwh.core:archetype.ultra-defensive-10-back",
                PressIntensity::None,
                CounterIntent::High,
                TacticState::LowBlock,
            ),
            (
                "fwh.core:archetype.false-9-system",
                PressIntensity::High,
                CounterIntent::Default,
                TacticState::MidBlock,
            ),
            (
                "fwh.core:archetype.inverted-fullback-3-2-5",
                PressIntensity::High,
                CounterIntent::Default,
                TacticState::MidBlock,
            ),
        ];

        for &(id, expected_press, expected_counter, expected_state) in expectations {
            let arch = store.tactical_archetypes.get(id).unwrap_or_else(|| {
                panic!("T2-1b/T2-1c archetype {id:?} missing from loaded content store")
            });
            let params = archetype_params_for(arch);
            assert_eq!(
                params.press_intensity, expected_press,
                "{id}: press_intensity bucket changed"
            );
            assert_eq!(
                params.counter_intent, expected_counter,
                "{id}: counter_intent bucket changed"
            );
            assert_eq!(
                params.default_in_defence_state, expected_state,
                "{id}: default_in_defence_state bucket changed"
            );
        }

        // Spread check: across the 16 archetypes total (2 existing + 6 T2-1b
        // + 8 T2-1c) we expect at least 3 distinct (press, counter,
        // default_state) buckets. T2-1c's `anti-high-press` archetype
        // introduces the NEW `(High, High, MidBlock)` combination not present
        // in the prior 8 archetypes — verifying this asserts the catalog
        // exercises the previously-empty corner of the bridge surface.
        //
        // `PressIntensity` / `CounterIntent` / `TacticState` don't derive Ord
        // (used for BTreeSet) so dedup via linear-scan Vec — small N +
        // sim-crate BTreeMap-only discipline.
        let mut buckets: Vec<(PressIntensity, CounterIntent, TacticState)> = Vec::new();
        for arch in store.tactical_archetypes.values() {
            let p = archetype_params_for(arch);
            let key = (
                p.press_intensity,
                p.counter_intent,
                p.default_in_defence_state,
            );
            if !buckets.contains(&key) {
                buckets.push(key);
            }
        }
        assert!(
            buckets.len() >= 3,
            "Bridge buckets collapsed: {} distinct (press, counter, default_state) tuples \
             across {} archetypes — T2-1c's anti-high-press archetype was \
             supposed to introduce a third bucket (High/High/MidBlock); \
             verify the archetype RON loads + the bridge thresholds haven't \
             drifted",
            buckets.len(),
            store.tactical_archetypes.len()
        );
    }

    /// Threshold-boundary test: buildup_speed_factor_bps == 11000 exactly
    /// yields High counter (the ≥ threshold). 10999 yields Default.
    #[test]
    fn archetype_params_for_buildup_speed_threshold_at_11000() {
        let at_threshold = fw_content::TacticalArchetype {
            id: "fwh.test:archetype.buildup-11000".into(),
            formation: vec![],
            press_radius_metres: 25,
            line_height_metres: None,
            buildup_speed_factor_bps: 11_000,
        };
        assert_eq!(
            archetype_params_for(&at_threshold).counter_intent,
            CounterIntent::High
        );

        let just_below = fw_content::TacticalArchetype {
            id: "fwh.test:archetype.buildup-10999".into(),
            formation: vec![],
            press_radius_metres: 25,
            line_height_metres: None,
            buildup_speed_factor_bps: 10_999,
        };
        assert_eq!(
            archetype_params_for(&just_below).counter_intent,
            CounterIntent::Default
        );
    }
}

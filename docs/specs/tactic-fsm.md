# Tactic FSM spec — team-level tactical state machine

**Status:** Tranche 4 spec for T1-2b. Lands the layer-1 detail referenced by ADR-0001.

**Implements:** ADR-0001 layer 1 ("Team tactic state machine, event-driven + 2 Hz heartbeat").

**Consumes:** ADR-0001 (architecture overview), ADR-0009 (RNG seed derivation), ADR-0011 (signature system — signatures bias the tactic state).

---

## Scope

The team-tactic FSM is the coarse intent layer. It runs per team (one FSM per side), parameterises every layer below it (decisions / utility / influence maps), and provides the legible "what is this team doing" handle for commentary + signature dispatch.

This spec covers:
1. The five states + their transitions.
2. The 2 Hz heartbeat that catches drift between event triggers.
3. The set-piece interrupt path.
4. The interaction with team-tactic parameters (press intensity, build-up tempo, line height).

What's NOT in scope: per-player decision-runner states (those are ADR-0006's FSM-of-BTs; see `decision-layer-state-catalogue.md` Tranche-4 follow-up). The team tactic FSM only parameterises the per-player layer.

---

## The five states

```rust
// crates/fw-match-sim/src/tactic_fsm.rs (lands at T1-2b)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TacticState {
    /// Aggressive press; line high; closing distance to ball-carrier.
    /// Trigger condition: opponent has the ball in their own half, press
    /// intensity ≥ High, recent loss < 5s (Bauer-and-Anzer counter window).
    HighPress,
    /// Mid-block compact shape; line at ~halfway; force play wide.
    /// Default in-defence state when neither HighPress conditions nor
    /// LowBlock conditions hold.
    MidBlock,
    /// Deep block; line on or near own box; absorb pressure.
    /// Trigger condition: leading + late-game OR archetype-driven default.
    LowBlock,
    /// Active counter-attack window; verticality maximised.
    /// Trigger condition: just regained ball, opponent shape broken
    /// (mean opponent x > halfway), within 4s of recovery.
    CounterAttack,
    /// Active set-piece resolution; positions structured per archetype.
    /// Trigger condition: ball is dead AND set-piece type is decided.
    SetPiece(SetPieceKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SetPieceKind {
    KickOff,
    GoalKick,
    GoalKick_Opponent,
    Corner_For,
    Corner_Against,
    FreeKick_For,
    FreeKick_Against,
    ThrowIn_For,
    ThrowIn_Against,
    Penalty_For,
    Penalty_Against,
}
```

Five states (with `SetPiece` carrying a sub-discriminant). The named states are the legible commentary handles; the set-piece sub-discriminant is the structural handle.

---

## Transition table

Event-driven transitions fire on the listed event class. Each transition is `(from_state, event, to_state, guard_predicate?)`.

| From | Event | To | Guard |
|---|---|---|---|
| any → | `MatchEvent::BallOutOfPlay { kind }` | `SetPiece(set_piece_for_kind(kind))` | always |
| `SetPiece(_)` | `MatchEvent::BallInPlay` | (resume per archetype default) | always |
| `MidBlock`, `LowBlock` | `MatchEvent::PossessionLost { recovery_likely: bool }` | `HighPress` | `recovery_likely == true AND press_intensity ≥ High AND tick_now - state_entry_tick > 600` (prevent thrashing) |
| `MidBlock` | `MatchEvent::PossessionLost { recovery_likely: false }` OR scoreline-trailing in-second-half | `LowBlock` | conservative trigger; archetype param |
| `HighPress` | `MatchEvent::PressTimeoutExpired` (`5s` after entry — Bauer-and-Anzer) | `MidBlock` | always (press windows close) |
| `HighPress` | `MatchEvent::BallRecovered { mean_opponent_x > halfway }` | `CounterAttack` | always |
| `MidBlock`, `LowBlock` | `MatchEvent::BallRecovered { mean_opponent_x > halfway }` | `CounterAttack` | tactic param `counter_intent ≥ Default` |
| `CounterAttack` | `MatchEvent::CounterWindowClosed` (`4s` after entry OR shot taken) | (resume archetype default) | always |
| any | `MatchEvent::Goal { ... }` | `MidBlock` | reset on goal (kick-off restart is structural) |
| any | `MatchEvent::HalfTime` | `MidBlock` | always (state resets at the break) |

**Default state per archetype.** Each `TacticalArchetype` declares its `default_in_defence_state: TacticState` (e.g. `direct-pressing` defaults to `MidBlock`; `low-block-counter` defaults to `LowBlock`). When no transition matches, the FSM idles in the archetype's default.

---

## 2 Hz heartbeat

Every 30 integration ticks (at 60 Hz, that's 2 Hz exact — 60 / 2 = 30, clean), the heartbeat fires per team and runs a drift check:

```rust
fn heartbeat_check(
    state: &TacticState,
    match_state: &MatchState,
    archetype: &TacticalArchetype,
) -> Option<TacticState> {
    // Drift detection: if the spatial picture has slipped meaningfully
    // away from the state's expected shape, transition. Catches cases
    // where no discrete event fired but the team has drifted (e.g.
    // gradual territorial shift).

    // Example: in HighPress for >10s without any ball recovery → drop to
    // MidBlock. The press has materially failed.
    if let TacticState::HighPress = state {
        let ticks_in_state = match_state.tick - state_entry_tick;
        if ticks_in_state > TICKS_PER_10S {
            return Some(TacticState::MidBlock);
        }
    }

    // Example: in MidBlock with own mean_x < 30 (deep) for >5s while
    // leading by 2+ → transition to LowBlock (we've slumped to defending
    // the lead).
    if let TacticState::MidBlock = state {
        let scoreline_lead = score_lead_for_team(match_state, ...);
        let own_mean_x = mean_x(match_state, ...);
        if scoreline_lead >= 2 && own_mean_x < q32_from_int(30) {
            return Some(TacticState::LowBlock);
        }
    }

    // ... archetype-conditioned drift rules, authored in
    // `docs/design/tactic-fsm-heartbeat-rules.md` (Phase 1 tuning doc).

    None
}
```

The heartbeat is at 2 Hz (30 ticks) because tactic state is meant to be **coarse**. Faster heartbeats would thrash; the per-player decision layer (4 Hz, ADR-0001 amended) reads the slow tactic state to bias its own decisions.

Heartbeat draws no RNG. It's a pure predicate over canonical state.

---

## Parameters

Each `TacticalArchetype` carries the tactic-state parameters in its RON:

```ron
TacticalArchetype(
    id: "fwh.core:archetype.direct-pressing",
    ...
    press_radius_metres: 30,
    buildup_speed_factor_bps: 9000,
    // T1-2b additions:
    default_in_defence_state: HighPress,
    line_height_metres_per_state: {
        HighPress: 55,
        MidBlock: 45,
        LowBlock: 25,
        CounterAttack: 55,
        SetPiece: 40,  // overridden per SetPieceKind in T2
    },
    counter_intent: Default,  // None | Default | High
    press_intensity: High,    // None | Low | Default | High
)
```

The archetype types in `fw-content::runtime` extend in T1-2b to absorb these fields. Backwards-compatible — existing fixtures get defaults via `#[serde(default)]`.

---

## RNG usage

The tactic FSM is **deterministic-by-predicate**. No `seed_fn` calls. Every transition is a pure function of canonical state + the archetype's parameters.

The downstream layers DO use RNG (per ADR-0009 layer assignments), but the tactic FSM itself is not a stochastic layer.

---

## Test contract

T1-2b acceptance for this spec:

1. **State enumeration** — every `TacticState` variant can be reached from some plausible match-state input.
2. **Transition determinism** — `proptest` invariant: same `(match_state, archetype, event)` produces the same `to_state`.
3. **Heartbeat drift detection** — a fixture where the smoke seed produces a 30-second HighPress without a recovery transitions to MidBlock at the heartbeat boundary.
4. **No RNG draws** — `cargo test` instruments any `seed_fn` calls inside `tactic_fsm::*`; expected count is zero.
5. **Set-piece interrupt path** — every `BallOutOfPlay` event class produces a corresponding `SetPiece(_)` state; `BallInPlay` restores the prior or archetype-default state.

---

## Open questions deferred to T1-2b implementation

- The exact `press_timeout_5s_ticks` constant — 300 ticks at 60 Hz, but should it be archetype-tunable? Default: no (Bauer-and-Anzer's 5s is an empirical constant from football). Open for archetype override if tuning shows otherwise.
- The state-entry-tick storage — does each `TacticState` carry its entry-tick directly, or does the team-level `TeamTacticState` struct carry `(state, entry_tick)` separately? Default: the latter (state stays a simple enum, entry-tick is a sibling field).
- Does `MidBlock` need sub-states for "compact" vs "stretched" shape? Default: no — those are influence-map outputs, not tactic states.

---

## Cross-references

- ADR-0001 §"Concrete shape" (the 7-layer table; this spec implements layer 1)
- ADR-0009 (RNG seed derivation — tactic FSM uses none)
- ADR-0011 (signature system — bias snapshots can fire during specific tactic states)
- `crates/fw-content/src/runtime.rs::TacticalArchetype` (the parameter carrier)
- `docs/specs/bt-attribute-binding.md` (Tranche 4 — what attributes each per-player BT state reads; the per-player layer this spec parameterises)
- `docs/specs/decision-cadence-stagger.md` (Tranche 4 — the 4 Hz per-player stagger across 22 players, given the chosen cadence)

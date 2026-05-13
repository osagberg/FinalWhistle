# Pattern: Finite state machines via Rust enums

Match-phase, player-state, ball-state. Rust enums + `match` give exhaustive-checking + free pattern docs at the type level.

## Why

State machines naturally appear in the sim: a match transitions Kickoff → InPlay → SetPiece → HalfTime; a player transitions Defending → Marking → Tackling; a ball transitions Loose → Held → InFlight → Settled.

Rust enums encode the state set; `match` forces exhaustive handling; the compiler catches "you forgot to handle the InFlight case."

## When to use

- Discrete state set (≤8 variants typically)
- Transitions are constrained (not every-to-every)
- The current state affects what operations are valid

## When NOT to use

- Continuous state (player position) — that's just a struct field
- Independent-bit state (is-tired AND is-tackling AND has-yellow-card) — that's flags, not a state machine

## Pattern: basic enum + transition function

```rust
pub enum BallState {
    Loose { position: Q32Position, velocity: Q32Velocity },
    Held { holder: PlayerId, control_quality: Q32 },
    InFlight { trajectory: Trajectory, last_touched: PlayerId },
    Settled { position: Q32Position },
}

impl BallState {
    pub fn tick(self, world: &WorldState, rng: &mut ChaCha8Rng) -> BallState {
        match self {
            BallState::Loose { position, velocity } => {
                if let Some(holder) = nearest_player_in_control_radius(world, &position) {
                    BallState::Held { holder, control_quality: initial_control(rng) }
                } else if velocity.magnitude() < Q32::from_int(1) {
                    BallState::Settled { position }
                } else {
                    BallState::Loose { position: integrate(position, velocity), velocity: apply_drag(velocity) }
                }
            }
            BallState::Held { holder, control_quality } => { /* ... */ }
            BallState::InFlight { trajectory, last_touched } => { /* ... */ }
            BallState::Settled { position } => { /* ... */ }
        }
    }
}
```

## Pattern: typed-state (compile-time enforcement)

For cases where you want "Held ball can be passed" but "Loose ball cannot":

```rust
pub struct Ball<S> { state: S, position: Q32Position }
pub struct Loose;
pub struct Held { holder: PlayerId }
// etc.

impl Ball<Held> {
    pub fn pass(self, target: PlayerId) -> Ball<InFlight> { /* ... */ }
}
```

Use sparingly — typed-state has cost (more types, harder to store homogeneous collections). Justify when invariants matter at compile time.

## Determinism considerations

- All transition functions take `&mut ChaCha8Rng` (deterministic).
- No `Instant::now()` — tick supplied externally.
- `match` is exhaustive — compiler enforces.

## Cross-references

- `Sim/RULES.md` — determinism
- `behavior-trees.md` — used together for player decision-making

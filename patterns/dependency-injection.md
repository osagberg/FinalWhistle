# Pattern: Dependency injection in Rust

Traits + generics over `Box<dyn Trait>`. Compile-time wiring. Minimal cost.

## Why

We want testability (swap a real `MatchClock` for a `TestClock`) and clean module boundaries. But we want zero runtime cost in the sim hot path. Rust's monomorphization gives both.

## When to use

- A module needs an abstract dependency that has multiple implementations
- Tests want to substitute a mock without changing production code
- Cross-crate boundaries with multiple impls

## When NOT to use

- Single impl that's unlikely to grow more — just use the concrete type
- Per-tick allocations of trait objects (`Box<dyn Trait>` per tick → GC pressure equivalent in Rust)

## Pattern: trait + generic struct

```rust
pub trait MatchClock {
    fn current_tick(&self) -> Tick;
    fn advance(&mut self);
}

pub struct SystemClock { tick: Tick }
impl MatchClock for SystemClock {
    fn current_tick(&self) -> Tick { self.tick }
    fn advance(&mut self) { self.tick = Tick(self.tick.0 + 1); }
}

pub struct TestClock { tick: Tick }
impl MatchClock for TestClock {
    fn current_tick(&self) -> Tick { self.tick }
    fn advance(&mut self) { self.tick = Tick(self.tick.0 + 1); }
}

pub struct Sim<C: MatchClock> {
    clock: C,
    world: World,
}

impl<C: MatchClock> Sim<C> {
    pub fn tick(&mut self) {
        self.world.advance(self.clock.current_tick());
        self.clock.advance();
    }
}
```

`Sim<SystemClock>` and `Sim<TestClock>` are different concrete types. Both monomorphize to inlined direct calls. Zero virtual dispatch.

## Pattern: trait object at boundaries

OK when you need heterogeneity at runtime — e.g., a list of `Box<dyn Plugin>` where plugins are loaded from dynamic libraries.

```rust
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}
```

Acceptable cost: plugin invocation is rare (not per-tick).

## Anti-patterns

- **`Box<dyn Trait>` in per-tick code paths.** Each call is a virtual dispatch + a heap allocation if the box churns. Don't.
- **Trait objects in canonical state.** Can't be `Serialize` (trait objects don't have stable type info). Don't.
- **DI frameworks (like Dagger / Spring).** Rust doesn't have them, and you don't need one. Compile-time generics + a `new()` constructor at the entry point is enough.

## When in doubt

- Have one impl + likely one impl forever → concrete struct.
- Two impls now or two impls foreseen within 6 months → trait + generic struct (this pattern).
- Many impls at runtime, all need to coexist → trait object collection.

## Cross-references

- `Sim/RULES.md` — performance constraints
- `Rust/RULES.md` — no speculative abstractions; three similar lines beats a premature trait

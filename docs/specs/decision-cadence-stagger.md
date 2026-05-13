# Decision cadence + stagger — 22 players at 4 Hz on a 60 Hz integration loop

**Status:** Tranche 4 spec for T1-2b. Resolves Codex audit P1 ("8Hz cadence math is wrong").

**Implements:** ADR-0001 layer 2 (per-player decision runner @ 4 Hz, amended 2026-05-13).

---

## Scope

The per-player decision runner fires 4 times per second per player. With 22 players on the pitch and a 60 Hz integration tick (15 ticks per second), the stagger needs to distribute 88 decisions per second across the 60-tick window cleanly. This spec defines the exact stagger pattern, the slot-assignment determinism, and the test cases.

The original ADR-0001 picked 8 Hz, which produced `60/8 = 7.5` ticks per decision per player — not a clean integer window. The amended 4 Hz cadence gives `60/4 = 15` ticks per decision per player, which IS clean. This spec records how the stagger then works.

---

## Cadence math

- **Integration tick:** 60 Hz. Every player position advances every 1/60 s ≈ 16.67 ms.
- **Per-player decision cadence:** 4 Hz. Each player re-decides every 1/4 s = 250 ms = 15 integration ticks.
- **22 players × 4 decisions/sec/player = 88 decisions/sec total.**
- **88 decisions over 60 integration ticks ≈ 1.47 decisions per integration tick.**

The stagger must distribute the 22 players across the 15-tick window so that ~1.47 decisions fire per tick on average — some ticks see 1 decision, some see 2.

---

## Stagger pattern

### Slot assignment

Each of the 22 players gets a `decision_slot: u8` in `0..15` at match-init time. Multiple players share a slot (22 / 15 ≈ 1.47, so 7 slots have 2 players and 8 slots have 1 player). Slot assignment is deterministic from the match seed:

```rust
// Pseudo-code; lands in fw-match-sim at T1-2b
fn assign_decision_slot(player_id: PlayerId, match_seed: Seed) -> u8 {
    // BLAKE3(match_seed || player_id) mod 15.
    // Use the canonical seed_fn for cross-platform determinism.
    let raw = seed_fn(
        match_seed.as_u64(),
        0,
        SeedLayer::Decision,
        player_id.as_u32(),
    );
    (raw % 15) as u8
}
```

Important properties:
- Same `(match_seed, player_id)` → same slot. Replay reproduces the stagger byte-identically.
- Slots are assigned ONCE at match-init, NOT per tick. The slot field lives on `MatchState.player_decision_slots: [u8; 22]` (canonical state).
- Distribution is approximately uniform across `0..15` — BLAKE3 is uniform, modulo bias is negligible at 22 samples.

### Firing rule

For player `p` with slot `s`, decision fires at integration tick `t` iff `(t mod 15) == s`:

```rust
fn should_decide(player: &PlayerState, tick: Tick) -> bool {
    (tick.raw() % 15) == u32::from(player.decision_slot)
}
```

That's it. Per-tick, the integration loop iterates 22 players, calls `should_decide`, and runs the decision runner on the matches.

### Per-tick decision count

In any given integration tick `t`:
- Count = number of players whose slot equals `t mod 15`
- Range: 1 or 2 (with the 22/15 distribution; theoretically 0 or 3 possible but BLAKE3 distribution rules this out under normal seeds)

Worst-case 3-in-a-tick is a smell — the slot-assignment fixture test (below) flags any seed where some slot gets ≥3 assignments.

---

## Determinism contract

The stagger pattern is canonical-state. Drift in slot assignment = drift in the integration loop's iteration order over decisions = drift in canonical hash. This means:

1. `MatchState` carries `decision_slots: [u8; 22]` in its canonical layout.
2. The `CanonicalEncoder` (`fw-match-sim/src/canonical.rs`) emits these bytes after the player list, before the ball state. Wire layout pinned at T1-2b.
3. The pinned BLAKE3 hash captures the stagger; any change to `assign_decision_slot` re-baselines per ADR-0012.

### Why not just iterate by player_id and decide every 4th frame for that player?

Because that's the same thing as the stagger above, but expressed implicitly through iteration order. The explicit `decision_slot` field makes:
- The canonical layout visible.
- The stagger inspectable in the replay viewer (which player decides at which tick).
- Re-baselines auditable.

Implicit iteration order is a recipe for "we accidentally changed iteration order in a refactor and the hash drifted with no design intent" — the worst kind of drift to debug.

---

## RNG slot for decision-time draws

When a player's decision fires at integration tick `t` (i.e. `t mod 15 == slot`), the BT runner may draw randomness for tie-breaking, probabilistic decorators, etc. Per ADR-0009:

```rust
let rng_seed = seed_fn(
    match_state.match_seed,
    tick.raw(),
    SeedLayer::Decision,
    (player_id.as_u32() << 16) | local_decision_counter,
);
let mut rng = ChaCha8Rng::seed_from_u64(rng_seed);
```

The `local_decision_counter` is a per-player monotonic counter that resets at match-init and increments per BT draw within a single decision invocation (most decisions need 0–2 draws; some signature-firing decisions need more). The counter is part of canonical state.

---

## Reactive interrupt path (60 Hz)

The 60 Hz reactive-interrupt layer (ADR-0001 layer 6) is **independent** of the 4 Hz stagger. Any player can be interrupted at any integration tick if a reactive predicate fires. When an interrupt fires:

1. The player's current BT execution is preempted.
2. The interrupt's `InterruptResolution` BT subtree runs immediately.
3. The player's decision-slot is RESET to `(tick + 15) mod 15` — i.e. their next scheduled decision is 15 ticks (250 ms) later. This prevents thrashing if an interrupt fires close to a scheduled decision.

Reactive interrupts use `SeedLayer::ReactiveInterrupt` with `site = player_id.as_u32()` per ADR-0009.

---

## Test contract

T1-2b acceptance:

1. **Slot assignment determinism** — `proptest` over 100 random `match_seed`s: same seed produces same `[u8; 22]` slot array.
2. **Slot distribution uniformity** — for the canonical smoke seed `0xDEAD_BEEF_DEAD_BEEF`, every slot in `0..15` is assigned at least once (i.e. no slot is empty). Sanity check.
3. **Worst-case-3 detection** — across 1000 random seeds, no slot gets ≥3 player assignments. (Theoretical bound: birthday-problem expectation < 1 occurrence per million seeds.)
4. **Tick-decision-count invariant** — for any `match_seed`, the per-tick decision count over a 600-tick run is in `{1, 2}`. Never 0, never 3+.
5. **Canonical-hash regression** — adding `decision_slots: [u8; 22]` to `MatchState` re-baselines the pinned hash (per ADR-0012 trigger #1, canonical schema bump). Both the pinned constant + the RON fixture's `expected_hash` update in the same T1-2b commit.
6. **Reactive interrupt reset** — when an interrupt fires at tick `t`, the player's `decision_slot` (or next-scheduled-decision-tick) is `(t + 15) mod 15`. Tested via a fixture that injects a reactive predicate hit + asserts the next scheduled decision tick.

---

## Cross-references

- ADR-0001 §"Cadence rationale" (4 Hz amended; this spec implements the 4 Hz stagger)
- ADR-0009 (RNG seed derivation — `SeedLayer::Decision` + `SeedLayer::ReactiveInterrupt`)
- ADR-0012 (hash rebaseline policy — adding `decision_slots` to canonical state fires trigger #1)
- `docs/specs/tactic-fsm.md` (Tranche 4 — the layer 1 tactic FSM parameters that gate per-player BT decisions)
- `docs/specs/bt-attribute-binding.md` (Tranche 4 — what each BT site reads from `PlayerAttributes`)
- `docs/specs/determinism-gate.md` (cross-OS canonical-hash contract; the stagger is canonical state)

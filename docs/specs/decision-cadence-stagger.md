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

### Slot assignment — balanced deterministic, NOT random modulo

Each of the 22 players gets a `decision_slot: u8` in `0..15` at match-init time. The distribution is **balanced by construction**: exactly 7 slots get 2 players each, exactly 8 slots get 1 player each (7×2 + 8×1 = 22). Every tick in the 15-tick window sees either 1 or 2 decisions — no thundering herd, no empty slot.

**Codex pre-T1-2b re-audit P1 (2026-05-13):** the prior algorithm was `seed_fn(...) % 15`. That's the birthday problem — random modulo over 22 items into 15 bins regularly produces collisions (3+ in a slot) AND empty slots (0 in some slots). BLAKE3 uniformity doesn't fix it. The correct fix is to assign from a **balanced multiset** + shuffle:

```rust
// Pseudo-code; lands in fw-match-sim at T1-2b-ii.
//
// SLOT_TEMPLATE is a 22-element multiset: slot 0 appears twice, 1 appears
// twice, ..., slot 6 appears twice (7 slots × 2 = 14), then slots 7..14
// appear once (8 slots × 1 = 8). Total 22.
//
// Match-init shuffles SLOT_TEMPLATE deterministically (Fisher-Yates seeded
// by the match seed), then assigns the i-th shuffled element to player
// roster_slot i+1 (player_id is durable; we use roster_slot because the
// stagger should be by ROSTER position so substitutions don't reshuffle).
//
// Resulting [u8; 22] is the decision_slots field on MatchState.
const SLOT_TEMPLATE: [u8; 22] = [
    0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6,   // 7 doubled slots = 14 entries
    7, 8, 9, 10, 11, 12, 13, 14,                 //  8 single slots =  8 entries
];

fn assign_decision_slots(match_seed: Seed) -> [u8; 22] {
    let mut slots = SLOT_TEMPLATE;
    let seed = seed_fn(
        match_seed.as_u64(),
        0,
        SeedLayer::Decision,
        0, // site=0 reserved for the stagger-shuffle draw
    );
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    // Fisher-Yates shuffle in-place. Iteration order is stable per Rust
    // semantics; ChaCha8Rng output is bit-identical across platforms.
    for i in (1..slots.len()).rev() {
        let j = (rng.gen::<u32>() as usize) % (i + 1);
        slots.swap(i, j);
    }
    slots
}
```

**Why "doubled slots" are 0..6 instead of an arbitrary 7-slot subset:** any deterministic choice works as long as the template is fixed. Picking the first 7 slots keeps the spec readable (and equivalent under shuffling).

Important properties:
- **Balanced by construction**: count(slot==0) ≤ count(slot==14) for all slots in `0..15`; every slot has ≥ 1 player; no slot has > 2 players. Static.
- **Deterministic**: same `match_seed` → same `[u8; 22]`. Cross-platform identical via ChaCha8Rng + Fisher-Yates from a fixed template.
- **Per-roster-slot, not per-PlayerId**: stagger is keyed by roster_slot (1..=11 per side, 22 total). Substitutions inherit the off-going player's slot — no reshuffle mid-match.
- **Stored in canonical state**: `MatchState.decision_slots: [u8; 22]`. The canonical-hash regression test pins the new layout per ADR-0012 trigger #1 (canonical schema bump) at T1-2b-ii.

### Firing rule

For roster_slot `r` with assigned `decision_slot s`, decision fires at integration tick `t` iff `(t mod 15) == s`:

```rust
fn should_decide(roster_slot: u8, decision_slots: &[u8; 22], tick: Tick) -> bool {
    let idx = (roster_slot as usize) - 1; // roster_slot is 1..=22; index is 0..=21
    (tick.raw() % 15) as u8 == decision_slots[idx]
}
```

Per-tick: the integration loop iterates 22 roster_slots, calls `should_decide`, runs the decision runner for matches.

### Per-tick decision count — guaranteed {1, 2}

By the balanced template + Fisher-Yates, any given tick `t`'s decision count is `1` for slots `7..15` and `2` for slots `0..7`. Never `0`. Never `3+`. **The invariant is structural**, not statistical — Fisher-Yates is a permutation, not a sample-with-replacement, so the slot distribution is unchanged after shuffling.

This resolves the Codex P1 finding that the prior spec promised `{1, 2}` but used an algorithm (random modulo) that couldn't guarantee it.

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

The 60 Hz reactive-interrupt layer (ADR-0001 layer 6) is **independent** of the 4 Hz stagger. Any player can be interrupted at any integration tick if a reactive predicate fires.

**Codex pre-T1-2b re-audit P1 reconciliation (2026-05-13):** the prior version of this section said interrupts RESET the player's `decision_slot` to `(tick + 15) mod 15`. That mutation would have **broken the balanced-multiset invariant** — runtime slot reassignments can produce empty slots OR 3+-in-a-slot at any moment, contradicting the per-tick `{1, 2}` invariant the balanced template guarantees. **`decision_slots: [u8; 22]` is now immutable canonical-init state**; interrupts use a parallel cooldown field that doesn't touch the slot assignment.

When an interrupt fires:

1. The player's current BT execution is preempted.
2. The interrupt's `InterruptResolution` BT subtree runs immediately.
3. **A parallel `interrupt_cooldown_until: [Tick; 22]` field** (also canonical state) is updated for the affected roster slot: `interrupt_cooldown_until[roster_idx] = tick + 15`. The `decision_slots` field is **not mutated** — the balanced multiset invariant holds for the full match.
4. The firing rule below is amended to combine both fields.

**Amended firing rule:**

```rust
fn should_decide(
    roster_slot: u8,
    decision_slots: &[u8; 22],
    interrupt_cooldown_until: &[Tick; 22],
    tick: Tick,
) -> bool {
    let idx = (roster_slot as usize) - 1;
    // Skip the scheduled decision if we're still inside an interrupt
    // cooldown window. The cooldown does NOT reshuffle the stagger —
    // it just suppresses ONE scheduled firing for the affected player,
    // after which the regular cadence resumes.
    if tick < interrupt_cooldown_until[idx] {
        return false;
    }
    (tick.raw() % 15) as u8 == decision_slots[idx]
}
```

Effect: if a reactive interrupt fires at tick `t` for a player whose next scheduled decision is at `t + 5`, the cooldown extends to `t + 15`, suppressing the `t + 5` firing. After tick `t + 15`, the player's next decision comes at the NEXT tick where `tick % 15 == decision_slots[idx]` — same slot, just delayed by one cadence cycle.

The cooldown field IS canonical state and IS counted in the canonical-hash regression. It starts at `Tick::ZERO` for every roster_slot at match-init, so a fresh match's pinned hash isn't sensitive to the cooldown until the first interrupt actually fires.

Reactive interrupts use `SeedLayer::ReactiveInterrupt` with `site = (roster_slot as u32)` per ADR-0009 for any stochastic draws WITHIN the interrupt resolution. The cooldown update itself is deterministic (no RNG).

---

## Test contract

T1-2b-ii acceptance:

1. **Slot assignment determinism** — `proptest` over 100 random `match_seed`s: same seed produces same `[u8; 22]` slot array.
2. **Balanced-multiset invariant** — for ANY `match_seed`, exactly 7 slots in `0..15` are assigned twice; exactly 8 are assigned once; no slot is empty; no slot is assigned ≥3 times. This is structural, NOT statistical — Fisher-Yates is a permutation of SLOT_TEMPLATE, so the multiset is invariant by construction.
3. **`decision_slots` immutability** — `MatchState.decision_slots: [u8; 22]` is canonical-init state + never mutated for the duration of the match. Reactive interrupts use the parallel `interrupt_cooldown_until: [Tick; 22]` field; the slot array stays frozen. Tested via a fixture that fires 100 random interrupts across a 600-tick run + asserts `decision_slots` is byte-identical at start + end.
4. **Tick-decision-count invariant** — for any `match_seed` AND any interrupt-firing pattern, the per-tick scheduled decision count (i.e. ignoring cooldown suppressions) is in `{1, 2}`. Suppressions just remove individual firings — they don't change the underlying stagger. Follows directly from invariant 2 + invariant 3.
5. **Canonical-hash regression** — adding `decision_slots: [u8; 22]` AND `interrupt_cooldown_until: [Tick; 22]` to `MatchState` re-baselines the pinned hash (per ADR-0012 trigger #1, canonical schema bump). Both the pinned constant + the RON fixture's `expected_hash` update in the same T1-2b-ii commit.
6. **Reactive interrupt cooldown** — when an interrupt fires at tick `t` for roster_slot `r`, `interrupt_cooldown_until[r-1]` becomes `t + 15`. The scheduled `decision_slots[r-1]`-aligned firing within `[t, t+15)` is suppressed (returns `false` from `should_decide`); the next firing at-or-after `t + 15` resumes the regular cadence. Tested via a fixture that injects an interrupt + asserts the next decision tick.
7. **Cross-OS determinism** — the Fisher-Yates draws use `ChaCha8Rng::seed_from_u64(seed_fn(match_seed, 0, SeedLayer::Decision, 0))` (site=0 reserved for the stagger draw). CI matrix on macOS / Windows / Linux must produce byte-identical `decision_slots` arrays for the smoke seed.

---

## Cross-references

- ADR-0001 §"Cadence rationale" (4 Hz amended; this spec implements the 4 Hz stagger)
- ADR-0009 (RNG seed derivation — `SeedLayer::Decision` + `SeedLayer::ReactiveInterrupt`)
- ADR-0012 (hash rebaseline policy — adding `decision_slots` to canonical state fires trigger #1)
- `docs/specs/tactic-fsm.md` (Tranche 4 — the layer 1 tactic FSM parameters that gate per-player BT decisions)
- `docs/specs/bt-attribute-binding.md` (Tranche 4 — what each BT site reads from `PlayerAttributes`)
- `docs/specs/determinism-gate.md` (cross-OS canonical-hash contract; the stagger is canonical state)

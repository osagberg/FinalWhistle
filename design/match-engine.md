---
description: MatchSim architecture, determinism discipline, ball physics spec. Canonical source for "how does a match simulate?"
last_verified: 2026-04-22
status: scaffolded; awaiting Phase 2 lock
---

# Match Engine — MatchSim + Ball Physics

## Purpose

Answer "how is a football match simulated deterministically, testably, cross-platform, at headless-10000x speed for balance harness use, while remaining viewer-friendly for semantic-cinema rendering?"

## Locked decisions

See SPEC.md 2026-04-22 entries. Summary:

- **MatchSim.csproj is pure C#, zero UnityEngine references.** Enables xUnit tests + headless balance harness + future mobile port.
- **Fixed-point canonical state.** Floats FORBIDDEN inside MatchSim except for non-canonical viewer interpolation. Format (Q16.16 vs Q24.8) to be locked at Phase 3 Week 1 based on test sweep.
- **Custom deterministic ball physics.** NOT Unity PhysX. Lockstep with MatchSim. Magnus force + air drag in fixed-point.
- **Fixed timestep.** 60Hz logical tick. Viewer interpolates at framerate; never drives sim.
- **Replay seeds.** Every match carries `match_seed: u64`; every in-match stochastic event derives from `(match_seed, tick, event_id)`.
- **CI matrix.** GitHub Actions runs MatchSim.Tests on Windows + Mac + Linux; failure on any platform blocks merge.

## MVP boundary

At Month 3:
- 22 players, 11 vs 11, single match
- Basic positional behavior (BT-driven role defaults)
- Ball physics: ground rolling + air kick + bounce + friction + spin (Magnus)
- 3 signatures authored end-to-end
- Deterministic replay verified by hash

At Month 12 EA:
- Full season-length stability (no drift across 380 matches per save)
- ~20-30 BT archetypes playing recognizably different football
- All 24 signatures authored
- Event emission to memory ledger
- Deterministic replay at seed-level granularity

## Deferred

- Unity Jobs / ECS port — only if Phase 6 perf demands
- GPU-side sim — no
- Networked sim — no (single-player game)
- Advanced injury modeling (Physical Load as Narrative Debt) — Phase 7+ polish

## Open questions (Phase 2 lock)

### Q1: Fixed-point format

**Q16.16** (32-bit): ±32,768 integer range, ~1.5e-5 precision. Sufficient for pitch coordinates (m) and velocity. Risk: tight on position × velocity multiplication overflow.

**Q24.8** (32-bit): ±8,388,608 integer range, ~4e-3 precision. Ample range; coarser precision. Risk: sub-cm positioning may feel snap-grid.

**Q32.32** (64-bit): both wide range + fine precision. Cost: 64-bit math perf on all platforms (fine on 64-bit ARM + x86).

**Recommend Q32.32** as default; revisit only if balance harness perf shows it's the bottleneck. Cross-platform determinism > tiny perf wins.

### Q2: Ball physics model complexity

**Minimum viable:** position + velocity + spin vector; update per tick with gravity + air drag + Magnus force when spin > 0; collision with ground (bounce energy coefficient 0.4-0.6), players (possession check), goal (score detection), touchlines.

**Stretch:** bounce coefficient varies with pitch state (dry/wet); stadium altitude affects air density; ball-specific models (different ball SOs for cup final vs league vs weather-affected).

**Recommend minimum viable at Phase 3.** Stretch behaviors added only when balance harness or player feedback demonstrate the simple model feels wrong.

### Q3: Player movement primitive

**Option A:** continuous force integration (accelerations + max-speed caps) — physics-feeling, may be noisy, fits 60Hz tick.

**Option B:** steering-target model (player has desired position + speed; actuator model moves toward it; BT controls desired) — more readable, easier to tune, may feel arcade.

**Recommend Option B** for Month-3 slice; evaluate against Option A during Phase 3 prototyping if feel is too mechanical.

### Q4: Substitution + in-match events

At Month 3 slice: no substitutions, no injuries mid-match, no fouls system. These surface in Phase 4-5. Lock that simplification in the slice spec.

## Prototype gate

**Phase 3 Week 2:** deterministic replay test passes.

```csharp
// Test: same seed → same canonical state hash on Win/Mac/Linux
[Fact]
public void Match_With_Same_Seed_Produces_Identical_State_Hash()
{
    var seed = 0xDEADBEEFUL;
    var home = BehaviorTreeArchetypes.Load("direct_pressing");
    var away = BehaviorTreeArchetypes.Load("low_block_counter");
    var sim = new MatchSim(home, away, seed);

    for (int tick = 0; tick < 60 * 90; tick++) sim.Tick();

    var hash = sim.CanonicalStateHash();
    Assert.Equal(EXPECTED_HASH_GOLDEN, hash);
}
```

CI matrix runs this on Windows + Mac + Linux; all three must produce the identical hash. Any drift = fix the sim before scaling.

**Phase 3 Week 4:** Month-3 slice's 22 players play a visibly legible match per the gate in `month-3-vertical-slice.md`.

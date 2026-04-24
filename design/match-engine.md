---
description: MatchSim architecture, determinism discipline, ball physics spec. Canonical source for "how does a match simulate?"
last_verified: 2026-04-24
status: Phase 0 open questions resolved; ball-physics structure, movement primitive, in-match event scope locked. Numeric coefficients live in this doc as Phase-3 tuning seeds, not SPEC commitments.
---

# Match Engine — MatchSim + Ball Physics

## Purpose

Answer "how is a football match simulated deterministically, testably, cross-platform, at headless-10000x speed for balance harness use, while remaining viewer-friendly for semantic-cinema rendering?"

## Locked decisions

See SPEC.md 2026-04-22 entries. Summary:

- **MatchSim.csproj is pure C#, zero UnityEngine references.** Enables xUnit tests + headless balance harness + future mobile port.
- **Fixed-point canonical state.** Floats FORBIDDEN inside MatchSim except for non-canonical viewer interpolation. Q32.32 is the default canonical format unless Phase 3 profiling proves it is the bottleneck.
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

## Resolved (2026-04-24)

See SPEC.md decisions log entry `2026-04-24 — Match-engine open questions resolved`.

### Q1 — Fixed-point format

**Q32.32 remains canonical** per SPEC 2026-04-23. No new decision; this section preserved for context.

- Q16.16 rejected: position × velocity multiplication overflow risk.
- Q24.8 rejected: ~4e-3 precision too coarse for ball/trajectory work.
- Q32.32 accepted: wide range + fine precision; 64-bit math performant on all target 64-bit platforms.

Revisit only if Phase 3 profiling proves fixed-point math is the bottleneck after algorithmic cleanup.

### Q2 — Ball physics: structure locked, coefficients are Phase-3 tuning seeds

**Structure (locked):**

- Ball state: `position` (Q32.32 vec3), `velocity` (Q32.32 vec3), `spin` (Q32.32 vec3, zero unless a signature / action imparts spin).
- Integrator: **semi-implicit Euler at fixed 60Hz step.**
- Forces applied each step:
  - **Gravity** — `F_g = (0, -g, 0)`.
  - **Linear air drag** — `F_d = -C_d · v`.
  - **Magnus (optional)** — `F_m = C_m · (spin × v)`, applied only when `|spin| > 0`.
- Collisions:
  - **Ground** — bounce energy coefficient `e` on vertical velocity component; rolling friction reduces horizontal velocity when `position.y ≈ 0` and `|v| > 0`.
  - **Player** — radius-based possession check only. No rebound energy model at Month 3.
  - **Goal** — line-cross vs goal-plane. No post-hit stochastics at Month 3.
  - **Touchline** — state transition to throw-in / corner / goal-kick; no crowd interaction.

**Magnus stub policy:** the Magnus term is **structurally present from Month 3**. If the Month-3 gate observers describe curve-driven moments as noisy / unreadable, the Magnus coefficient may be run at zero for the gate build. Phase 4 re-enables and tunes it for curve-dependent signatures. The structure stays; only the coefficient may be zeroed for the gate.

**Coefficients (Phase-3 starting tuning seeds — NOT locked design truth):**

> These are fixed-step tuning constants assuming the 60Hz simulation step. They are **not physical SI-unit coefficients** and will be re-derived if the tick rate ever changes. Expect to re-tune these in Phase 3 Week 1 once the first match is watchable.

| Symbol | Initial seed | Meaning |
|---|---|---|
| `g` | 9.81 m/s² (Q32.32) | gravitational acceleration |
| `C_d` | 0.02 / step | linear drag coefficient applied each 60Hz step |
| `C_m` | 0.0004 / step | Magnus coupling applied each 60Hz step |
| `e` | 0.55 | vertical bounce energy retention (0-1) |
| `μ_step` | 0.25 / step | rolling friction: `v_horizontal *= (1 - μ_step)` each step while rolling |

**Deferred stretch (Phase 4+):** pitch-state-modulated bounce (wet/dry), altitude-affected air density, ball-specific ScriptableObjects (cup-final vs league vs weather), post-hit stochastic deflection, collision rebound-energy model.

### Q3 — Player movement: steering-target actuator

**Locked:** BT outputs `desired_position` and `desired_speed` per tick. A deterministic fixed-point actuator applies **acceleration, deceleration, turn-rate, and max-speed caps** toward the target.

The actuator is a small physical model — this decision does **not** forbid internal force-integration inside the actuator. What it forbids is **dual-authoritative movement** (e.g., BT force-integration running alongside actuator cap logic). One authority over a player's movement per tick.

**Exit clause:** switching to continuous force integration (Option A — BT directly outputs forces, no steering-target layer) requires a **new append-only SPEC decision or ADR citing this one.** No silent flip during Phase 3 prototyping.

### Q4 — In-match event scope

**Month-3 slice:** NO substitutions, NO injuries, NO fouls, NO cards, NO stoppage time, NO VAR. Full continuous 90 in-game minutes; all 22 starters remain on pitch start-to-finish.

**Phase 4 introduction order (locked):**

1. **Fouls + basic set pieces** — several signatures depend on set-piece context for counterplay.
2. **Cards** — tightly coupled to fouls, but separate task / separate decision point.
3. **Substitutions** — surfaces manager-decision ledger events; required for Phase-5 transfer-market prototype compatibility.
4. **Basic injuries** — seeds the Physical Load as Narrative Debt data shape; full surfacing system is Phase 7+ polish.
5. **Stoppage time** — ordered last because it needs real event stoppages (fouls, cards, subs, injuries) to count; high-salience late-goal ledger emission.

**VAR deferred indefinitely.** Fictional league; "review" flavor may be added post-EA if audience signal requests. Not on the MVP path.

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

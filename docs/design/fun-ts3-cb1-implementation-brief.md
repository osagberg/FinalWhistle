# FUN-TS3 + FUN-CB1 — implementation blueprint (the in-possession slice)

> Status: BRIEF (2026-06-04). Implementation-ready blueprint for the next engine slice after FUN-TS2 — CB1 (passes-can-fail) FIRST, then TS3 (build-up geometry via the built-but-unused xT + pitch_control). Grounded in tactical-shape.md §Slice4 + believability-specs.md (contested-ball) + match-realism-reference.md §2. Hand this to the gameplay-programmer when FUN-TS2 commits. Tuning constants are SOFT (drama-sweep calibrated).

# FUN-CB1 + FUN-TS3 Implementation Blueprint

**For the gameplay-programmer. Effective task brief for the next engine lane slice following FUN-TS2.**

---

## Patterns and Conventions Found

The codebase has clean, established patterns that both slices must follow exactly:

**Pass dispatch seam** (`/crates/fw-match-sim/src/dispatch.rs:983-1106`): four pass-class arms — `AttemptPassShort`, `AttemptPassLong`, `Cross`, `LayOff` — all share the identical structure: resolve `to_slot` via `nearest_teammate_near`, run the offside gate, emit `MatchEvent::Pass { completed: T1_PASS_COMPLETED }`, then mutate ball state. The `T1_PASS_COMPLETED: bool = true` constant at line 880 is the named stub. `nearest_teammate_near` at line 1283 is the "kick 10m forward, nearest body receives" proxy.

**Utility function signatures** (`/crates/fw-match-sim/src/bt/on_ball.rs:366-554`): all utility functions are `fn(player: &PlayerState, roster_slot: u8) -> (PlayerIntent, Q32)`. They contain the target geometry inline. For TS3, the geometry changes but the function signature does not — the target `(x, y)` embedded in the returned `PlayerIntent` is what changes.

**BtContext threading** (`/crates/fw-match-sim/src/bt/mod.rs:168-201`, `SelectFn` type alias at line 152): adding context to `BtContext` requires updating the type alias for `SelectFn`, the struct, and the `tick_leaf` dispatch site. This has been done before for `carrier_pos` (FUN-0b+c) and `team_shape`/`team_idx` (FUN-TS1) — the pattern is established.

**Pitch control API** (`/crates/fw-match-sim/src/utility/pitch_control.rs:88`): `pitch_control(point: (Q32, Q32), attackers: &[(PlayerId, PlayerSnapshot)], defenders: &[(PlayerId, PlayerSnapshot)]) -> PitchControlOutcome`. Zero call sites today. `PlayerSnapshot { pos, vel, v_max }` is constructed inline from `state.players`. The function is Q32-clean, pure, zero-allocation at the canonical level.

**xT API** (`/crates/fw-match-sim/src/utility/xt.rs:283-288`): `xt_delta(src: PitchZone, dst: PitchZone) -> Q32`. `PitchZone::new(x: u8, y: u8) -> Option<PitchZone>` converts from 16×12 grid coordinates. Pitch coordinates (±52.5m × ±34m) must be mapped to grid indices.

**MatchEvent discriminants** (`/crates/fw-content/src/event.rs:31-50`): discriminants 0-6 are pinned. Next available is **7**. The table header comment, `MatchEventDiscriminant` enum, and `encode_match_event` in `canonical.rs` all require synchronised updates — this was done for `Offside` (discriminant 6) in FUN-TS2b.

**Seed sites** (`/crates/fw-match-sim/src/dispatch.rs:1380`): tackle site is `0x7AC1`; shot sites are `0x0001..0x0003`. The CB1 spec calls out site `(from_slot as u32) << 16 | 0xCB01` on `SeedLayer::Decision` — distinct from all existing sites.

**Rebaseline class**: CB1 adds a new `MatchEvent` variant (schema) and changes pass outcomes (behavioural) — ADR-0012 trigger #3 + schema, multi-pin authorized per slice. TS3 changes target geometry (behavioural only) — single-pin rebaseline.

---

## Architecture Decision: Slice Split — CB1 First

**Implement FUN-CB1 before FUN-TS3.** The rationale is load-bearing, not cosmetic:

CB1 retires `T1_PASS_COMPLETED` and introduces the completion draw. TS3 replaces `nearest_teammate_near` with the geometry-aware target selector and widens `compactness_h` in possession. These touch different code sites at the same logical boundary (`apply_intent` vs `on_ball.rs`), so they can be sequenced rather than merged. More importantly, CB1's drama-sweep calibration (the 83-86% completion rate, the ordering check) must complete before TS3 is added — because TS3 changes the pass target geometry, which feeds back into the completion probability through `pitch_control()`. Calibrating both simultaneously means chasing two interacting variables. CB1 alone has a stable calibration surface.

Each slice gets its own authorized rebaseline commit. CB1 is the smaller change (~150-200 LoC net) with the higher believability leverage per line. TS3 builds on CB1's stable completion model.

---

## Component Design

### FUN-CB1: Passes Can Fail

**Files modified:**

`/crates/fw-match-sim/src/dispatch.rs`

Remove `T1_PASS_COMPLETED` constant at line 880. Add a new module-private function:

```rust
fn resolve_pass_completion(
    state: &MatchState,
    from_slot_idx: usize,
    to_slot: u8,
    kind: PassKind,
) -> bool
```

This function is the entire CB1 mechanic. It:
1. Builds attacker/defender `PlayerSnapshot` slices from `state.players`. Attackers = same team as `from_slot_idx` (11 players); defenders = opposing team (11 players). Iterate in slot-order `(state.players[i].id, PlayerSnapshot { pos: (pos_x, pos_y), vel: (vel_x, vel_y), v_max: MAX_PLAYER_SPEED })`. No allocation beyond two `Vec` of capacity 11.
2. Calls `pitch_control(midpoint, &attackers, &defenders)` where `midpoint = ((passer.pos_x + receiver.pos_x) / 2, (passer.pos_y + receiver.pos_y) / 2)`. Reads `defender_control` as the lane-crowding signal: `lane_openness = Q32::ONE - outcome.defender_control`.
3. Calls `pitch_control(receiver_pos, &attackers, &defenders)` where `receiver_pos = (state.players[to_slot as usize].pos_x, state.players[to_slot as usize].pos_y)`. Reads `defender_control` as receiver pressure.
4. Computes `passer_quality` from a kind-dependent attribute composite on `state.players[from_slot_idx].attributes`:
   - `Short`/`LayOff`: `passing * W_PS + technique * W_TE + first_touch * W_FT` (weights from spec above)
   - `Long`: `passing * W_PL + vision * W_VI + long_shots * W_LS`
   - `Cross`: `crossing * W_CR + technique * W_TE + vision * W_VI`
5. Applies the revised formula (SOFT, drama-sweep-calibrated): `p_complete = clamp(P_BASE[kind] * lerp(LOW_MOD, Q32::ONE, passer_quality) * (Q32::ONE - RECV_PRESSURE_WEIGHT * receiver_pressure), P_FLOOR[kind], Q32::ONE)`. All P_BASE, LOW_MOD, RECV_PRESSURE_WEIGHT, P_FLOOR values are named `pub(crate) const` in a new file.
6. Draws one `ChaCha8Rng` seeded via `seed_fn(state.seed.to_u64(), tick_u32, SeedLayer::Decision, (from_slot as u32) << 16 | 0xCB01u32)`. Upper 32 bits of `next_u64()` → Q32 draw `r`. Returns `r < p_complete`.

In the four pass arms, replace:
```rust
completed: T1_PASS_COMPLETED,
```
with:
```rust
let pass_completed = resolve_pass_completion(state, slot_idx, to_slot, PassKind::Short); // or Long/Cross/LayOff
```

Then branch on `pass_completed`:
- **Success path**: existing ball velocity and possession mutation unchanged. Emit `MatchEvent::Pass { ..., completed: true }`.
- **Failure path**: emit `MatchEvent::Pass { ..., completed: false }`, then emit `MatchEvent::PassIncomplete { from_slot, to_slot, tick: state.tick, kind }` (new discriminant 7 — see below), then set `state.possession = None`, `state.last_touched_by = Some(from_slot)`, and drop the ball at the loose-ball point. Loose-ball calculation: for a forward pass (home: `to_slot.pos_x > passer.pos_x`; away: `to_slot.pos_x < passer.pos_x`), drop at 40% of the way from passer to receiver. For backward/lateral (LayOff, or non-forward direction), drop at 20%.

The `is_offside_at_pass_launch` check already runs before the pass arm body executes — leave it in place. A failed pass that is also offside: let the offside check win (it runs first at line 986-988 and returns early).

**New constants file:** `/crates/fw-match-sim/src/pass_completion.rs`

All tuning constants as `pub(crate) const Q32` values. Named and tagged SOFT per the design rules. The constants table from the believability spec goes here verbatim. Add `mod pass_completion;` to `lib.rs`.

`/crates/fw-content/src/event.rs`

Append `PassIncomplete` variant:
```rust
/// A pass that was attempted but lost control — spawns a loose ball.
/// Discriminant 7.
PassIncomplete {
    from_slot: PlayerSlot,
    to_slot: PlayerSlot,
    tick: Tick,
    kind: PassKind,
},
```

Update the discriminant table in the module doc comment. Update `MatchEventDiscriminant` enum in the same crate. Update `encode_match_event` in `/crates/fw-match-sim/src/canonical.rs` — add the discriminant 7 arm. Update `/crates/fw-content/tests/event_discriminant_test.rs` to pin the new variant count.

**No new canonical state fields.** The `Pass { completed: bool }` field already exists on the struct. `PassIncomplete` is a new event that lands in `match_events` — the only canonical surface touched is the event vec itself. This is why CB1 is a schema + behavioural rebaseline but not a field-addition rebaseline — the schema bump comes from the new discriminant, same as Offside was.

### FUN-TS3: Build-Up Structure

This slice has two independent sub-changes that can be implemented in sequence within the same commit:

**Sub-change A: Replace `nearest_teammate_near` with geometry-aware target selection**

Add a new function to `dispatch.rs`:

```rust
fn best_pass_target(
    state: &MatchState,
    from_slot_idx: usize,
    kind: PassKind,
) -> u8
```

This replaces `nearest_teammate_near` at the four call sites in the pass arms. It does NOT replace `nearest_teammate_near` for GK distribution (GKs use simpler distribution logic).

The algorithm:
1. Build attacker/defender snapshot slices identically to `resolve_pass_completion`.
2. For each same-team outfielder `candidate_slot` (exclude passer, exclude GK on short passes since GK is a poor short-pass target in normal play):
   - Compute `attacker_control_at_candidate` = `pitch_control(candidate_pos, &attackers, &defenders).attacker_control`. This is the "can the candidate receive cleanly" signal.
   - Compute `xt_gain` = `xt_delta(passer_zone, candidate_zone)`. Convert player positions to `PitchZone` via `pos_to_zone(pos_x, pos_y, team_idx)` — a new helper that maps pitch coords to the 16×12 grid. For home team: `grid_x = clamp(((pos_x + 52.5) / 105.0 * 16.0) as u8, 0, 15)`, `grid_y = clamp(((pos_y + 34.0) / 68.0 * 12.0) as u8, 0, 11)`. Away team: mirror x (`15 - grid_x`). All in Q32 arithmetic — no floats in canonical paths. Pre-compute the constant factors as `Q32` literals.
   - Score: `candidate_score = W_PC * attacker_control_at_candidate + W_XT * clamp(xt_gain + XT_NEUTRAL, Q32::ZERO, Q32::ONE)`. `XT_NEUTRAL` shifts the range so negative xT gains (backward passes) are penalised but not zeroed entirely — a back-pass to retain possession still has positive score from the `attacker_control` term.
3. Track `(best_score, best_slot)` in a simple loop in slot order — slot-order tiebreak for determinism (same discipline as `nearest_teammate_near`).
4. Add a `kind`-specific filter: `Long` passes skip candidates within 15m (no short targets for long passes). `Short` passes skip candidates beyond 20m. `LayOff` passes skip forward candidates (only consider candidates who are at/behind the passer's x). `Cross` passes skip non-box-area targets (only consider candidates in `pos_x > 35m` for home, `pos_x < -35m` for away).
5. Return `best_slot`. Add the same `assert_ne!` self-pass guard as `nearest_teammate_near` at line 1321.

Weights `W_PC` and `W_XT` are SOFT constants. Seed: `W_PC = 0.60` (attacker control dominates), `W_XT = 0.40`. The ordering constraint is that a player with high attacker control in a forward zone beats a player with high attacker control in a backward zone — the xT term enforces this when both have similar control values.

**Why this produces right-skewed sequences:** the LayOff and backward-pass filters mean a passer under pressure can find a safe back-pass target, but the xT scoring biases toward progressive targets when they're open. Sequences lengthen when possession is recycled safely through defenders; they shorten when a progressive pass opens up and the carrier attacks. This is the mechanism for the 4-pass mean with heavy right skew — it emerges from the geometry rather than being scripted.

**Sub-change B: Widen `compactness_h` in possession**

In `/crates/fw-match-sim/src/team_shape.rs`, `compute()` at line 235, the `compactness_h` is currently a fixed `Q32::from_int(COMPACTNESS_H)` where `COMPACTNESS_H = 35`. 

Add possession-aware widening:
```rust
let compactness_h = if is_defending {
    Q32::from_int(COMPACTNESS_H_DEFENDING) // 35m — current value
} else {
    Q32::from_int(COMPACTNESS_H_ATTACKING) // 55m — near full width
};
```

Add the two constants:
```rust
const COMPACTNESS_H_DEFENDING: i32 = 35; // SOFT — research: "out of possession ~30-44m"
const COMPACTNESS_H_ATTACKING: i32 = 55; // SOFT — research: "in possession ~55-68m"
```

The `is_defending` flag is already computed in `compute()` at line 277. This single-line change closes the possession-width swing the realism reference identifies as HARD.

**No BtContext changes for TS3.** The target geometry moves earlier (into `best_pass_target`) and is embedded in the intent returned by `utility_pass_short` etc. as `target_x/y`. But wait — the current design has `utility_pass_short` in `on_ball.rs` setting `target_x = player.pos_x + 10m` inline (line 387-394). For TS3, the target geometry must come from `best_pass_target` which needs `&MatchState`. This means the geometry cannot be computed in `on_ball.rs` (which only receives `&PlayerState`).

The solution is to move target resolution to `apply_intent`. `utility_pass_short` continues to return `AttemptPassShort { target_x, target_y }` with the current 10m-forward proxy target. In `apply_intent`, `best_pass_target` is called with the proxy target as a fallback discriminator only for the `kind` filter — the actual `to_slot` comes from `best_pass_target` replacing `nearest_teammate_near`. This is already the architecture: `utility_pass_short` chooses the intent and the rough direction; `apply_intent` resolves the actual receiver. Sub-change A slots in cleanly by replacing `nearest_teammate_near` with `best_pass_target` at the four call sites.

---

## Data Flow

**CB1 completion draw (per tick, per pass-class intent):**

```
tick_match → dispatch_tick → apply_intent (pass arm)
  → nearest_teammate_near [REPLACED by best_pass_target in TS3]
  → is_offside_at_pass_launch [existing gate, unchanged]
  → resolve_pass_completion(state, from_slot, to_slot, kind)
      → build attacker/defender snapshots (from state.players, slot order)
      → pitch_control(midpoint) → lane_openness
      → pitch_control(receiver_pos) → receiver_pressure
      → passer_quality from attributes
      → p_complete = formula
      → ChaCha8Rng::seed_fn(seed, tick, Decision, CB01_site) → draw r
      → r < p_complete → bool
  → if true: existing ball/possession mutation
  → if false: emit PassIncomplete(7), loose ball drop, possession = None
```

**TS3 target selection (per tick, per pass-class intent):**

```
apply_intent (pass arm)
  → best_pass_target(state, from_slot_idx, kind)
      → build attacker/defender snapshots (same as CB1 — can share a helper)
      → for each candidate in slot order (with kind-specific filter):
          → pos_to_zone(candidate.pos) → PitchZone
          → pitch_control(candidate_pos) → attacker_control
          → xt_delta(passer_zone, candidate_zone) → xt_gain
          → score = W_PC * control + W_XT * adjusted_xt_gain
      → best_slot (slot-order tiebreak)
  → to_slot = best_slot
  → resolve_pass_completion(state, from_slot, to_slot, kind) [CB1]
```

**TS3 width swing (per tick, per team_shape compute):**

```
dispatch_tick
  → team_shape::compute(team_idx, state)
      → is_defending = (carrier is on opposing team OR ball loose)
      → compactness_h = if is_defending { 35m } else { 55m }  [TS3 change]
  → store in state.team_shape[team_idx]
  → zonal_slot(slot, shape, team_idx) uses updated compactness_h
  → off-ball players widen in possession, narrow when defending
```

---

## Implementation Map

### FUN-CB1 Files

`/crates/fw-match-sim/src/pass_completion.rs` — **CREATE**
All `P_BASE`, `LOW_MOD`, `RECV_PRESSURE_WEIGHT`, `P_FLOOR` constants. The `resolve_pass_completion` function (or call it from dispatch.rs directly — either placement works; putting it here keeps `dispatch.rs` from growing further). If placed here, add a helper `build_team_snapshots(state: &MatchState, from_slot_idx: usize) -> (Vec<(PlayerId, PlayerSnapshot)>, Vec<(PlayerId, PlayerSnapshot)>)` since both CB1 and TS3 need identical slices — extract once, use twice.

`/crates/fw-match-sim/src/dispatch.rs` — **MODIFY**
- Remove `T1_PASS_COMPLETED` const at line 880.
- In all four pass arms (lines 983-1106): replace `completed: T1_PASS_COMPLETED` with the completion draw. Add the failure branch with loose-ball drop and `PassIncomplete` emission.
- Add `use crate::pass_completion::{resolve_pass_completion, build_team_snapshots};` at the top.

`/crates/fw-content/src/event.rs` — **MODIFY**
- Append `PassIncomplete` variant after `Offside` (discriminant 7).
- Update discriminant table in the module doc comment.

`/crates/fw-match-sim/src/canonical.rs` — **MODIFY**
- In `encode_match_event`, add the `PassIncomplete` arm with discriminant 7.
- Bump `VERSION` by 1 (same pattern as Offside in FUN-TS2b).

`/crates/fw-content/tests/event_discriminant_test.rs` — **MODIFY**
- Update the variant count assertion from 7 to 8.

`/crates/fw-replay/tests/canonical_hash.rs` and `canonical_hash__smoke_seed_final_state_snapshot.snap` — **REBASELINE** (authorized, multi-pin per CB1 task spec).

### FUN-TS3 Files

`/crates/fw-match-sim/src/dispatch.rs` — **MODIFY**
- Replace `nearest_teammate_near(state, slot_idx, *target_x, *target_y)` at lines 985, 1020, 1050, 1080 with `best_pass_target(state, slot_idx, PassKind::Short)` etc.
- Add `best_pass_target` function.
- Add `pos_to_zone` helper (pure Q32 arithmetic, no floats).

`/crates/fw-match-sim/src/team_shape.rs` — **MODIFY**
- Split `COMPACTNESS_H` into `COMPACTNESS_H_DEFENDING = 35` and `COMPACTNESS_H_ATTACKING = 55`.
- In `compute()` at line 240: branch on `is_defending` (already computed one line earlier at 277 — reorder so `is_defending` is computed before the `compactness_h` assignment, or inline the possession check at the compactness_h line).
- Update `TeamShape::zero()` default: use `Q32::from_int(35)` (defending default, unchanged).
- Update the existing `compute_produces_correct_compactness_for_lowblock` test to reflect the defending-state assumption; add a symmetric test for the attacking state.

`/crates/fw-match-sim/src/pass_completion.rs` — **MODIFY** (if the shared snapshot helper lives here)
- Add `build_team_snapshots` (shared by CB1 and TS3 code in dispatch.rs).

`/crates/fw-replay/tests/canonical_hash.rs` and `.snap` — **REBASELINE** (authorized, single-pin per TS3 task spec, separate from CB1's rebaseline commit).

---

## Proptest Invariants

### CB1 (`/crates/fw-match-sim/tests/pass_completion_proptest.rs` — CREATE)

```
completion_ordering_mechanical:
  N=200 seeded full matches → mean(layoff_completion) > mean(short_completion)
  > mean(long_completion) > mean(cross_completion) — assert all four inequalities
  with margin ≥ 0.01 (ordering is mechanical, so this should hold with probability
  approaching 1.0 across any seed set).

failed_pass_spawns_loose_ball:
  For every MatchEvent::PassIncomplete in any match run:
    - immediately after the tick it fires, state.possession == None
    - state.last_touched_by == Some(from_slot)
    - ball.pos_x/y is between passer and receiver positions (within 45% of the
      passer→receiver vector for forward, 25% for backward/lateral)

overall_completion_in_band:
  N=100 seeded matches → mean(all-pass completion rate) ∈ [0.78, 0.91]
  (the HARD 83-86% target at mirror-team baseline; the window is wider to
  account for drama-sweep calibration not yet having run)

p_floor_respected:
  For any (state, from_slot, to_slot, kind) input, resolve_pass_completion
  draws from [P_FLOOR[kind], 1.0] range — verify by constructing the
  maximally-adversarial case: worst passer attributes, maximum receiver
  pressure (pitch_control defender_control = 1.0 at receiver) → p_complete
  clamps to P_FLOOR, not below.
```

### TS3 (`/crates/fw-match-sim/tests/ts3_proptest.rs` — CREATE)

```
best_pass_target_no_self_pass:
  For any seeded state with possession, best_pass_target always returns
  a slot ≠ from_slot_idx — mirrors the assert_ne! invariant in a proptest.

pass_prefers_higher_attacker_control:
  Construct a state where one teammate has attacker_control ≈ 0.9 and
  another has attacker_control ≈ 0.1 at their respective positions;
  best_pass_target selects the higher-control candidate (all else equal,
  same zone → xT term cancels).

build_up_progresses_ball_upfield:
  N=50 seeded matches; for each uninterrupted possession spell of ≥6 passes
  with no PassIncomplete, the ball's mean_x at the end of the spell is
  forward of the mean_x at the start (home team: end_x > start_x; away: end_x < start_x).
  This is the "possession advances, not cycles" guard.

width_increases_in_possession:
  For any tick where home team has possession, home team horizontal span
  (max(pos_y) - min(pos_y) across slots 1-10) ≥ horizontal span when
  away team has possession — across 100 seeded ticks from a mid-match state.
  The 55m vs 35m compactness_h swing should produce this cleanly.

layoff_targets_not_forward:
  For every LayOff intent executed, to_slot.pos_x ≤ from_slot.pos_x (home)
  or to_slot.pos_x ≥ from_slot.pos_x (away) — the kind filter enforces backward targets.
```

---

## Drama-Sweep Acceptance Bar

Both slices require a drama-sweep run before the commit is accepted. The `drama_sweep` binary (or equivalent harness) runs N=200 seeded full matches and checks:

**CB1 acceptance:**
- `mean(pass_completion_rate) ∈ [0.78, 0.90]` at mirror-team baseline. If outside this band, adjust `P_BASE[Short]` up or down by 0.03 and re-run. One tuning pass per drama-sweep run.
- Ordering check: `layoff > short > long > cross` — must hold at N=200 with all four inequalities satisfied. This is a hard gate, not a p-value test.
- Contact-sheet at tick 3000+: a minimum of one `PassIncomplete` event visible per 200-match aggregate (confirming turnovers exist); the event's position in the event log is before the subsequent loose-ball chase.
- Passes per team per match: rough check that the total `Pass` event count lands in the [450, 550] band from match-realism-reference §2 (HARD ordering). If passes/match drops below 300, `P_BASE[Short]` is too low; if above 700, something is wrong with the completion formula.

**TS3 acceptance:**
- Contact-sheet (board-shots): at ticks where home team has possession, the horizontal spread of home outfielders is visibly wider (the compactness_h swing from 35m to 55m). A visual scan of 5 frames at different possession states suffices.
- Possession-spell length distribution: over 200 matches, compute mean and standard deviation of spell lengths (spell = consecutive passes without PassIncomplete ending possession). Target: mean ~4, SD > mean (right-skewed). This is the HARD shape constraint from match-realism-reference §2. If mean > 7, the pass selection is too accurate (lower W_XT); if mean < 2, too many turnovers (raise P_BASE or lower W_PC).
- `M_possession_cap` guard: no single team holds possession above 65% in any match (the 65/35 lock-prevention guard). Count possession ticks from `state.possession`.

---

## Critical Details

**Snapshot construction performance.** Both `resolve_pass_completion` and `best_pass_target` build two `Vec<(PlayerId, PlayerSnapshot)>` of capacity 11. Each is called once per pass-class intent dispatch. A typical match has ~500 passes/team, so ~1000 `pitch_control` calls per match for CB1 plus ~1000 for TS3 target selection. Each `pitch_control` call does 22 player tau computations with CORDIC `acos`. This is the hot path — measure it. If it's too slow, the snapshot construction can be hoisted to once per tick (before the per-slot decision loop) as a `[PlayerSnapshot; 22]` array on a stack-allocated struct passed through `dispatch_tick`. This is a performance optimisation, not a determinism concern.

**Half-position for loose-ball drop.** The 40%/20% interpolation: for a forward failed pass, `loose_x = passer.pos_x + Q32::from_raw(1_717_986_918) * (receiver.pos_x - passer.pos_x)` (0.40 in Q32). For backward, `loose_x = passer.pos_x + Q32::from_raw(858_993_459) * (receiver.pos_x - passer.pos_x)` (0.20). The y coordinate: `loose_y = passer.pos_y + 0.40 * (receiver.pos_y - passer.pos_y)`. Set `state.ball.pos_x = loose_x`, `state.ball.pos_y = loose_y`, `state.ball.vel_x = Q32::ZERO`, `state.ball.vel_y = Q32::ZERO`, `state.ball.vel_z = Q32::ZERO`. Ball stops dead — nearest body chases via `preempt_check`'s outfield-nearest-2 policy (already live, no change needed).

**`pos_to_zone` helper — Q32 arithmetic, no floats.** Pitch range: x ∈ [-52.5, +52.5]m, y ∈ [-34, +34]m. Grid: 16 bins in x (each 6.5625m wide), 12 bins in y (each 5.667m wide).

For home team (attacks +x, own goal at -52.5m): `zone_x = clamp((pos_x + Q32::from_raw(225_485_783_142_i64)) / Q32::from_raw(28_180_722_893_i64), 0, 15)` where the addend is `52.5 × 2^32` and the divisor is `(105.0/16.0) × 2^32`. Working entirely in Q32, the division produces a value in [0, 16); clamp to [0, 15]. For the y axis: `zone_y = clamp((pos_y + Q32::from_raw(146_028_888_064_i64)) / Q32::from_raw(24_338_148_011_i64), 0, 11)` where the addend is `34 × 2^32` and the divisor is `(68.0/12.0) × 2^32`. For away team: `zone_x = 15 - home_zone_x` (mirror).

Pre-compute these constants as `const Q32` values named `PITCH_HALF_X_Q32`, `PITCH_ZONE_WIDTH_Q32`, `PITCH_HALF_Y_Q32`, `PITCH_ZONE_HEIGHT_Q32`.

**`SeedLayer::Decision` for CB1.** The existing `SeedLayer` enum discriminant for `Decision` is already defined in `decision_cadence.rs`. CB1 uses it at site `(from_slot as u32) << 16 | 0xCB01`. This site is distinct from the existing tackle site `0x7AC1` and shot sites `0x0001-0x0003`. No new SeedLayer discriminant is needed.

**No through-ball variant.** The believability spec does not require a new `PassKind::ThroughBall` discriminant for this slice. A through-ball is a forward pass into space rather than to a player — the `best_pass_target` xT-driven selection already handles this organically (the highest-xT candidate in open space wins the selection). A named discriminant would be a speculative abstraction at this stage. Defer until the contact-sheet shows through-ball patterns deserve their own commentary treatment.

**Possession cap guard.** The 65/35 possession cap is not enforced mechanically — it emerges from the completion draw (failed passes return possession), the press (TS2), and the right-skewed spell distribution (TS3). If drama-sweep shows a team locking at >65%, the most likely cause is `P_BASE[LayOff]` too high (very safe back-passes let possession cycle indefinitely). Lower `P_BASE[LayOff]` by 0.03 per drama-sweep run until the cap holds.

**Offside interaction.** The offside check at lines 986-988 already runs before the pass arm body. A pass that would be offside does not reach `resolve_pass_completion` — `apply_offside` returns early. CB1 does not change this. Do not add a second offside check inside `resolve_pass_completion`.

**Rebaseline discipline.** CB1 commit: multi-pin rebaseline, commit body documents "FUN-CB1: passes can fail — new PassIncomplete discriminant (7) + behavioural change on every pass tick." TS3 commit (separate): single-pin rebaseline, commit body documents "FUN-TS3: build-up geometry — xT+pitch_control target selection + possession-width swing."

---

## Build Sequence

- [ ] **CB1-1:** Create `/crates/fw-match-sim/src/pass_completion.rs` with all constants, `build_team_snapshots` helper, and `resolve_pass_completion`. Write unit tests for `resolve_pass_completion` (mid-baseline short pass lands in [0.60, 0.90]; layoff > short > long > cross ordering at identical pressure).
- [ ] **CB1-2:** Append `PassIncomplete` (discriminant 7) to `event.rs`. Update `encode_match_event` in `canonical.rs`. Update discriminant pin test in `event_discriminant_test.rs`.
- [ ] **CB1-3:** Modify `dispatch.rs` — remove `T1_PASS_COMPLETED`, replace four pass arms with completion draw + failure branch. Verify `cargo build` clean.
- [ ] **CB1-4:** Create `pass_completion_proptest.rs`. Run all four invariants. Expect the completion-band invariant to fail on first run with wrong tuning — this is the drama-sweep calibration signal.
- [ ] **CB1-5:** Run drama-sweep N=200. Tune `P_BASE` constants until completion band [0.78, 0.90] holds and ordering is satisfied. Update constants in `pass_completion.rs`.
- [ ] **CB1-6:** `scripts/fw verify` green. Rebaseline `canonical_hash.rs` snapshot. Commit.
- [ ] **TS3-1:** Add `pos_to_zone` helper and `best_pass_target` to `dispatch.rs`. Replace `nearest_teammate_near` at four call sites. Write `pos_to_zone` unit tests (corners of the pitch map to expected grid cells).
- [ ] **TS3-2:** Modify `team_shape.rs` — split `COMPACTNESS_H` into defending/attacking constants. Branch `compute()` on `is_defending`. Update existing unit tests.
- [ ] **TS3-3:** Create `ts3_proptest.rs`. Run all five invariants. Expect the `build_up_progresses_ball_upfield` test to catch a regression if `best_pass_target` is selecting backward targets due to a `pos_to_zone` sign error.
- [ ] **TS3-4:** Run drama-sweep N=200. Check possession-spell mean ~4, right-skewed, possession cap ≤65%. Tune `W_PC`/`W_XT` if needed.
- [ ] **TS3-5:** Run board-shots at 5 representative ticks, read PNGs for the width swing. `scripts/fw verify` green. Rebaseline `canonical_hash.rs` snapshot. Commit.

---

## File Reference Summary

Read-only reference files grounding this blueprint:
- `/Users/vibelogic/dev/football/docs/design/tactical-shape.md` (Slice 4 spec)
- `/Users/vibelogic/dev/football/docs/design/believability-specs.md` (FUN-CB1 through FUN-CB3)
- `/Users/vibelogic/dev/football/docs/design/match-realism-reference.md` §2 and §3

Files the gameplay-programmer will modify:
- `/Users/vibelogic/dev/football/crates/fw-match-sim/src/dispatch.rs`
- `/Users/vibelogic/dev/football/crates/fw-match-sim/src/team_shape.rs`
- `/Users/vibelogic/dev/football/crates/fw-match-sim/src/bt/on_ball.rs` (if constant cleanup needed)
- `/Users/vibelogic/dev/football/crates/fw-content/src/event.rs`
- `/Users/vibelogic/dev/football/crates/fw-match-sim/src/canonical.rs`
- `/Users/vibelogic/dev/football/crates/fw-content/tests/event_discriminant_test.rs`
- `/Users/vibelogic/dev/football/crates/fw-replay/tests/canonical_hash.rs`
- `/Users/vibelogic/dev/football/crates/fw-replay/tests/snapshots/canonical_hash__smoke_seed_final_state_snapshot.snap`

Files to create:
- `/Users/vibelogic/dev/football/crates/fw-match-sim/src/pass_completion.rs`
- `/Users/vibelogic/dev/football/crates/fw-match-sim/tests/pass_completion_proptest.rs`
- `/Users/vibelogic/dev/football/crates/fw-match-sim/tests/ts3_proptest.rs`
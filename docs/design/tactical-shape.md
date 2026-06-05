# Team Tactical Shape — design spec (the believability core)

**Status:** SPEC (implementation-ready; the next match-engine work per DECISIONS 2026-06-04
"Believability-first sequencing + Pillar 0"). Pairs with `docs/adr/0013-team-tactical-shape.md`
(the architecture lock) and `docs/design/drama-model.md` (the metrics that certify it).
**Owner:** `systems-designer` (numbers/curves) + `gameplay-programmer` (the Rust).
**Tuning values:** per `.claude/rules/design-docs/RULES.md` §4, every metre/tick band below is a
Phase-N tuning value living here, NOT in code literals. Each band is tagged **FIRM** (a real-world
regularity) or **SOFT** (a taste call to iterate on the contact-sheet via the drama-sweep loop) per
the ultra-review P2 "anchors need firmness tags" finding.
**MASTER_PLAN rows:** this spec implements Tier-F **FUN-TS1** (defensive line + compactness),
**FUN-TS2** (coordinated press + offside), **FUN-TS3** (midfield build-up), **FUN-TS4** (integration
+ tactic-FSM-as-shape-driver), plus the mandatory companion **FUN-DR** (differentiated-roster sweep).

---

## Why this doc exists — Pillar 0 has no engine behind it yet

FUN-0 made the match *look* like football in a contact-sheet (players spread, ball moves, a 1-0;
drama-sweep M1 = 3.15 goals/match in band). But an independent ultra-review (P0-1) and the
implementing session converged on the same gap: **the engine has no team tactical SHAPE.** This doc
is the design that gives Pillar 0 (`DESIGN_DOC §3`) a real engine.

### What the engine actually has today (verified, file:line)

- The "tactic FSM" is real but **touches no positions.** `tactic_fsm.rs` produces a per-team
  `TacticState` (HighPress / MidBlock / LowBlock / CounterAttack / SetPiece) on
  `MatchState.team_tactic_states` (`lib.rs:262`), consumed only by signature/commentary *mood* — it
  is a label, not a shape system.
- Off-ball targets come from a **static formation lookup.** `utility_track_back` / `utility_hold_formation`
  / `utility_run_off_ball` (`bt/off_ball.rs:149,255,289`) target `formation_position(slot)`
  (`subtree_library.rs:123`) = a **constant** `FORMATION_4_3_3_POSITIONS[slot]` (`:88`), never shifted
  by ball or team centroid.
- Defending = individual `Press`/`Mark` toward `carrier_pos` (`off_ball.rs:184,225`) + the FUN-0b+c
  `resolve_tackles` (`lib.rs:1393`). It is a swarm, not a block.
- Passing options are scored from **attribute proxies** (`mental.positioning`, `mental.vision`;
  `on_ball.rs:21-24`) with **zero teammate geometry** — build-up is random forward pokes.
- There is **no** defensive line, **no** compactness, **no** offside, **no** coordinated press, **no**
  support structure. The ADR-0001 32×24 influence map (layer 5) was **never built**; `pitch_control.rs`
  is a per-point closed-form, not a grid.
- `initial_with_content` (`lib.rs:709`) **never assigns attributes** — every player is
  `mid_range_baseline()` (`player.rs:130`). This is the mirror-team calibration trap (ultra-review P1-4):
  every "better shape → better result" claim below is untestable until FUN-DR lands.

The residual FUN-0 leaks (goal-variance p95 reaching 12-17; on-target ~60% vs the 35-45% target) are a
**team-defensive-shape gap, not a coefficient gap** — no amount of xG/save/tackle tuning fixes a match
with no block to break up attacks.

---

## Core principle — closed-form affine zonal shape (NOT an influence map)

Players hold a **zonal slot = the static formation slot, transformed by a per-team affine shift**
derived from `(tactic_state, team_centroid, ball_x)`. The transform is a closed-form Q32 function:
no grid, no influence map, no float, no clock, no HashMap. We **explicitly do not build** the ADR-0001
32×24 influence map — it is overkill for shape and a determinism-surface + perf liability; the affine
transform is deterministic-by-construction, ~10× cheaper, and debuggable by reading it. ADR-0013 retires
that layer-5 plan.

Per-team shape anchors are recomputed each tick as a **pure function of canonical inputs** (positions,
ball, possession, tactic state), so the `TeamShape`/`PressPlan` sidecars are `#[serde(skip)]` — they add
**no canonical bytes**. The canonical-hash rebaseline each slice triggers comes from the **behaviour
change** (players move to new targets → positions drift), which is ADR-0012 trigger #3 (documented
sim-behaviour change), authorized per the slice, envelope-verified before re-pin (multi-pin discipline).

**The load-bearing seam:** between the tactic heartbeat (`tick_match` step 5, `lib.rs:2217`) and
`dispatch_tick` (step 6, `:2254`) — compute the team-shape anchors once per tick, then have the off-ball
utilities target zonal slots instead of static formation slots. This is where the FSM finally drives
positions.

---

## Slice 1 — Defensive line height + block compactness · FUN-TS1 (highest leverage)

The single change that kills the free-cycling attack chain (`shot-model.md:712-716`): today a CB
receives at x=30m and a forward dribbles unopposed to 17m because **the block never moves as a unit**.

**New state** (`src/team_shape.rs`, `#[serde(skip)]` sidecar on `MatchState`, `[TeamShape; 2]`):
```rust
pub(crate) struct TeamShape {
    line_x: Q32,           // defensive-line x (signed by attack direction)
    block_centroid_x: Q32, // mean x of the 10 outfield defenders this tick
    block_centroid_y: Q32,
    compactness_v: Q32,    // target vertical span (line → highest non-GK), metres
    compactness_h: Q32,    // target horizontal span, metres
}
```

**Transform** (`team_shape::compute(team, state)`, closed-form):
- `line_x` by `tactic_state` (measured from own goal line at ±52.5m): LowBlock ~18m, MidBlock ~35m,
  HighPress ~55m, CounterAttack ~45m (push up fast).
- `compactness_v`: LowBlock 25m, MidBlock 32m, HighPress 40m (higher press = more stretched — football
  reality).
- New helper `zonal_slot(roster_slot, shape, attack_dir) -> (Q32, Q32)`: takes the static
  `formation_position`, shifts x toward `line_x`, compresses inter-line spacing to `compactness_v`, and
  scales the slot's y by `compactness_h / formation_native_h`.

**Tick seam:** `team_shape::compute` at the top of `dispatch_tick` (`dispatch.rs:546`, after the carrier
pre-pass `:590-594`), storing on `state.team_shape`. `select_outfield_intent` (`subtree_library.rs:169`)
gains a `shape: &TeamShape` param; the three off-ball utilities call `zonal_slot(...)` instead of
`formation_position(...)`. **This wiring is the FSM-finally-drives-positions change.**

**Proptest invariants** (`tests/team_shape_proptest.rs`):
1. `defensive_line_within_band_of_state` — when defending, `|block_centroid_x - target_line_x(state)| < 12m` (allows transit lag).
2. `vertical_compactness_bounded_in_lowblock` — in LowBlock ≥120 consecutive ticks, rearmost-defender → foremost-non-GK span `< 30m` (today it is ~52m static).
3. `lowblock_more_compact_than_highpress` — same seed, LowBlock vs HighPress → strictly smaller mean vertical span (pins the qualitative direction).
4. `zonal_slot_continuity` — `zonal_slot` is Lipschitz in `line_x` (no teleport on state flip; bounds the per-tick target jump so step-8 separation doesn't thrash).

**Verification:** (a) `board-shots.mjs` / `render_contact_sheet` over a full match — defenders visibly
rise/drop as a unit on transitions; add a `dump_frames --overlay-shape` flag emitting `line_x`/`compactness_v`
so the contact-sheet can draw the line. (b) `drama_sweep` goal-variance (M1 std/p95) should fall from
~4.5/~17 toward 0.8-1.6/≤7 as the free-cycling chain is blocked — the primary statistical signal.

**Rebaseline:** YES, behavioural (trigger #3). **ADR:** 0013.

---

## Slice 2 — Coordinated pressing structure · FUN-TS2a (replaces the swarm)

On `PossessionLost` the FSM may flip to HighPress (`tactic_fsm.rs:395`), but **every defender independently
runs at `carrier_pos`** (`off_ball.rs:184`) — a swarm. ADR-0003 references the Bauer-Anzer 5s rule;
`PRESS_TIMEOUT_TICKS=300` (`tactic_fsm.rs:346`) governs only FSM timeout, not coordination.

**Design** (`team_shape::compute_press(team, state) -> PressPlan`, `#[serde(skip)]` sidecar):
- **Trigger:** press is "live" only when `tactic_state == HighPress` AND the ball is in the pressing
  team's press zone AND possession changed within the last N ticks (recovery window) — reuses
  `possession` / `last_touched_by` + the FSM entry tick.
- **Assignment (deterministic):** exactly **one Primary presser** (nearest opponent to carrier by Q32
  distance, slot-order tiebreak — same pattern as `resolve_tackles`) + **two Cover players** (next-nearest;
  shift to cut the carrier's two nearest passing lanes). All others **hold the Slice-1 block** (do NOT
  chase). A `PressRole { Primary, Cover, HoldShape }` enum selects the off-ball utility under HighPress.

**Tick seam:** computed in `dispatch_tick` after `team_shape::compute`; consumed by
`select_outfield_intent`'s Pressing arm (`subtree_library.rs:198-205`).

**Proptest invariants:**
1. `at_most_one_primary_presser_per_team` — `count(Primary per team) <= 1` across all ticks (press, not swarm).
2. `non_pressers_hold_shape_under_highpress` — non-Primary/Cover players stay within the Slice-1 compactness band.
3. `press_only_fires_in_press_state` — no `Primary` role when `tactic_state != HighPress`.
4. `press_recovery_window_bounded` — a designated press dissolves within `PRESS_TIMEOUT_TICKS` (the 5s Bauer-Anzer constant).

**Verification:** contact-sheet — under HighPress, 3 players converge + 7 hold a line. `drama_sweep`
on-target% falls toward 35-45% (coordinated pressing forces rushed/blocked shots). **Rebaseline:** YES,
behavioural. **ADR:** extends 0013 (press is shape-dependent).

---

## Slice 3 — Offside line + flag · FUN-TS2b (cleanest, highest believability-per-LoC)

No offside today (grep-confirmed absent). Self-contained.

**New canonical:** `MatchEvent::Offside { offending_slot: PlayerSlot, tick: Tick }` (`event.rs`, append after
`SignatureFirstFired` → discriminant **6**; update `MatchEventDiscriminant`, the cross-crate
`event_discriminant_test.rs` pin, and `encode_match_event` in `canonical.rs:464`).

**Mechanism (deterministic, at pass-launch):** offside is decided when a forward pass is *played*, not
received. In `apply_intent`'s pass-class arms (`dispatch.rs:867+`):
1. Compute the defending team's **offside line** = x of the second-rearmost defender (incl. GK), via a Q32
   min-pass over the 11 defending players (Vec/BTreeMap sorted — Sim/RULES §6).
2. Resolve the intended receiver (`to_slot`, already computed for the `Pass` event `:830`).
3. If the receiver's x is **beyond both** the offside line and the ball at launch → push `MatchEvent::Offside`,
   set `possession = None`, award restart to the defending GK (free-kick proxy; real set-piece is T2-4),
   firing the existing SetPiece `BallOutOfPlay`/`BallInPlay` machinery. Runs **before** the
   possession/`last_touched_by` mutation so a flagged pass doesn't transfer possession.

**Proptest invariants** (`tests/offside_proptest.rs`):
1. `offside_flagged_when_receiver_beyond_last_defender_at_pass_moment` — the literal invariant.
2. `onside_when_level_or_behind` — receiver at/behind the line → no flag (pin the equal case as onside, per law).
3. `backward_pass_never_offside` — a pass toward own goal never flags.
4. `offside_only_on_pass_launch_tick` — no `Offside` on dribble/shot/loose-ball ticks.

**Verification:** add an informational `M9_offside_per_match` to `drama.rs` (real-world ~1.5-3.5/match);
through-ball spam should drop. **Rebaseline:** YES — schema (new discriminant) **and** behavioural
(possession reverts). **ADR:** a `/log-decision` entry + this section suffice; fold into 0013 if the
GK-restart proxy proves load-bearing.

---

## Slice 4 — Midfield possession / build-up structure · FUN-TS3

Lowest believability-now leverage, but the natural consumer of Slices 1-2 and essential for "deep" later.
Today passing is attribute-proxy only — no teammate geometry.

**Design:** replace the on-ball pass proxies with **real support geometry** using the existing
`pitch_control` (`pitch_control.rs:88`, already built + Q32-clean — this is where that closed-form earns
its keep) + the existing xT table (`utility/xt.rs`):
- Each pass candidate's utility = `f(support_quality, xT_gain, lane_openness)` where support = attacker
  pitch-control at the receiver, progressivity = xT-delta toward goal — replacing the `vision_proxy`.
- The attacking team's off-ball runners (Slice-1 zonal slots) provide **width + support angles**: in
  possession, `compactness_h` widens to create passing options. This closes the loop with Slice 1.

**Tick seam:** `on_ball.rs` utilities gain teammate/opponent position slices threaded through `BtContext`
(like `carrier_pos` in FUN-0b+c, `dispatch.rs:777-784`). No new canonical state.

**Proptest invariants:**
1. `pass_prefers_higher_pitch_control_teammate` — higher attacker pitch-control → higher pass utility (all else equal).
2. `build_up_progresses_ball_upfield` — over a no-dispossession possession spell, mean ball x advances toward the opponent goal (not random).
3. `width_increases_in_possession` — team horizontal span larger in-possession than out.

**Verification:** `drama_sweep` possession-spell length + pass-completion look like football (3-6 pass
sequences); contact-sheet shows triangular support, not isolated dribbling. **Rebaseline:** YES,
behavioural. **ADR:** none new (utility refinement under ADR-0006).

---

## FUN-TS4 — Integration + tactic-FSM promotion to shape driver

The final pass: wire Slices 1-3 so the tactic-FSM states actually SELECT shape parameters (block height,
press trigger, build-up risk) rather than only nudging individual decision weights — make the FSM a shape
system. Re-run the FUN-0 watchable bar on the integrated engine. **Acceptance:** an archetype-distinctness
probe shows MidBlock vs HighPress vs LowBlock produce statistically distinguishable match signatures
(press distance, line height, transition speed); FUN-0 M1/M2 guards still in band on **differentiated**
rosters.

---

## Mandatory companion — FUN-DR (do it WITH FUN-TS1, not after)

Every "better shape → better outcome" invariant and the whole S2-upset story is **untestable on mirror
teams** (P1-4: `initial_with_content` leaves all players `mid_range_baseline`). Before FUN-TS1's drama-sweep
can certify anything, the harness must run **seed-varied per-player attributes + non-identical archetype
pairs** and an `S2_upset_rate` guard (target 25-40%, not the ~50% a mirror forces). This is a `drama_sweep`
bin change (off the canonical-hash path; floats allowed; **no rebaseline**), so it does not bloat any
slice's rebaseline. (Being delivered alongside the drama-sweep believability hardening.)

---

## Tuning bands (firmness-tagged)

| Band | Value (seed) | Firmness | Note |
|---|---|---|---|
| LowBlock line height | 18m from own goal | SOFT | Taste call — sets how "deep" a low block reads. Iterate on contact-sheet. |
| MidBlock line height | 35m | SOFT | — |
| HighPress line height | 55m (into opp half) | SOFT | — |
| CounterAttack line | 45m, push up fast | SOFT | Transition speed is feel-driven. |
| LowBlock vertical compactness | 25m | SOFT | The "how tight is the block" call. |
| MidBlock / HighPress compactness | 32m / 40m | SOFT | Higher press = looser is FIRM (direction); the exact metres are SOFT. |
| "Press = line not swarm" (≤1 Primary) | structural | FIRM | Not a number — an invariant. |
| Offside law (beyond 2nd-rearmost at launch; equal = onside) | structural | FIRM | Real-world rule. |
| Offside per match | ~1.5-3.5 | FIRM | Real-world regularity (informational metric). |
| S2 upset rate | 25-40% | FIRM-ish | Real football ~28-34%; a high-variance fantasy world may legitimately run hotter (revisit after FUN-DR). |
| On-target rate | 35-45% | FIRM | Real-world regularity; the press should drive ~60% → here. |

---

## Owner-feel forks (NOT coefficient tuning — need the owner's eye / a product call)

- **Line-height + compactness metres** are taste calls dressed as physics; they set how the whole match
  reads. Iterate on the contact-sheet via the drama-sweep loop, not by fiat.
- **Offside restart semantics:** awarding possession to the GK is a proxy for an indirect free-kick. Does
  the EA floor need a real free-kick set-piece (T2-4), or is the GK-proxy acceptable for now? A scope fork,
  flagged for the owner.
- **Press aggressiveness vs gaps:** a tighter press concedes space behind; the believability sweet spot is
  feel-driven and only findable on differentiated rosters with the S2 guard live.
- **Transition lag** (how fast the line travels between states) interacts with the 4 Hz cadence + step-8
  separation: snap = robotic, crawl = broken. Tune the `zonal_slot` Lipschitz bound (TS1 invariant 4) by eye.

---

## FUN-PHYS-1: Collision-aware player movement (known open limitation)

The separation pass (`separation.rs`) is position-only: it nudges overlapping players apart by
half the overlap, but does not touch velocities. `apply_vel_toward_target` in `dispatch.rs`
re-issues the full convergence velocity every decision tick (every 15 ticks / 250ms), so two
players chasing the same loose-ball point drive through each other across multiple ticks.

FUN-CB1 (passes-can-fail) exposed this concretely: seed 7834583133621575731 showed pair (6,20)
converging on a dropped loose ball with a 150mm / 62-tick clip-through. A partial mitigation is
in `drop_loose_ball` (dispatch.rs): the dropped ball is laterally offset 0.4m away from the
nearest opponent, breaking the head-on approach geometry. Measured result: that seed now shows
only CORDIC ringing (≤12 raw bits ≈ 0.000003mm), same as clean seeds.

The root cause remains: no steering/avoidance in the locomotion model. The correct fix requires
either a velocity-level avoidance force (add a repulsion term before the speed cap) or a
waypoint-routing approach (route around the nearest blocker rather than directly toward the
target). A naive global velocity-damp was tested and rejected: at any tested threshold it
suppresses approach velocity in attacking plays, dropping M1 from 2.35 to 2.15 (below the
[2.3, 3.2] acceptance band).

**Tracked:** `FUN-PHYS-1` in `docs/MASTER_PLAN.md`. `gameplay-programmer`.

---

## Cross-references

- `docs/DESIGN_DOC.md §3` — Pillar 0 (Believable Football), the foundation this serves.
- `docs/DECISIONS.md` 2026-06-04 — believability-first sequencing + Pillar 0.
- `docs/adr/0013-team-tactical-shape.md` — the architecture lock (retires ADR-0001's 32×24 influence map).
- `docs/adr/0001-match-engine-architecture.md` §layer-5 — the influence-map plan this supersedes.
- `docs/design/drama-model.md` — M1 (goal-variance), M8 (on-target), S2 (upset) — the metrics that certify each slice.
- `docs/design/match-realism-reference.md` §3 — the **research-grounded** line-height / compactness / PPDA / offside anchors. NB: it refines the provisional `line_x` seeds in this doc UPWARD (real low/mid blocks sit ~25m / ~40m, not 18m / 35m) and says high-line vs high-press should vary independently.
- `docs/design/shot-model.md` §"what resisted" — the deferred zonal-compactness gap this closes.
- `verification/ultra-review-2026-06-04.md` P0-1 / P1-4 — the believability + mirror-calibration findings.
- `crates/fw-match-sim/src/{tactic_fsm.rs, bt/off_ball.rs, bt/on_ball.rs, dispatch.rs, subtree_library.rs, lib.rs, utility/pitch_control.rs}` — the seams.

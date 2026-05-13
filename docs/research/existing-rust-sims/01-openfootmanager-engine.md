# openfootmanager match engine — read-through notes

**Read on:** 2026-05-13
**Lines of Rust:** ~3,500 (engine crate src + tests)
**Key file:** `src-tauri/crates/engine/src/live_match/simulation.rs` (line count: 259)

OFM ships two parallel engines: a one-shot batch simulator in `engine/mod.rs` (`simulate_with_rng`, 208 LoC) and an interactive minute-stepper in `live_match/` (`LiveMatchState::step_minute`, 385 LoC in mod.rs + 259 in simulation.rs). They duplicate most resolution logic — the live one is the load-bearing path. Engine deps: `rand 0.10`, `serde 1`, `log 0.4`. That's it. No `tokio`, no `fixed`, no `blake3` (Cargo.toml, lines 6–13).

## Tick structure

**Coarse minute-by-minute, NOT continuous physics.** No 60Hz tick. No positions. No vectors. No physics integration. `LiveMatchState::play_minute` (simulation.rs:155–201) is the inner loop:

1. `current_minute += 1` (simulation.rs:156)
2. Possession ticks++ for the in-possession side (simulation.rs:160–163)
3. `deplete_stamina_tick()` — drains all players' condition (helpers.rs:13–30)
4. Sample `actions = rng.random_range(1..=3u8)` — 1, 2, or 3 actions per minute (simulation.rs:170)
5. Each action: `resolve_action(minute, rng)` (zone_resolution.rs:14–28) — dispatch by ball zone
6. Possession contest via midfield-rating ratio (simulation.rs:177–185)
7. Check phase end (simulation.rs:203–245)

Match length is `90 + rng(0..=stoppage_time_max)` minutes per half, ~90–98 total. Extra-time and penalty shootout are full sub-phases (mod.rs:18–33, MatchPhase enum has 11 variants).

## Decision cycle / AI architecture

**No BT. No utility scoring. No state machines. Players don't decide anything — they're sampled.**

`snap_player(side, preferred_position, rng)` (helpers.rs:48 / engine/mod.rs:152) picks a random player from the right position bucket each action. That snapshot's attributes feed the probability formula. There is no per-player AI loop.

The only "AI" is the **manager AI** (ai.rs:33–72, called from the test harness — `ai_decide` is NOT called inside `step_minute`; it's a separate function the host must invoke). It returns `Vec<MatchCommand>` for substitutions + play-style swaps and runs as hardcoded `if` ladders on `(minute, goal_diff, experience_factor)` thresholds (ai.rs:108–203, 269–315). Example: "losing by 2+ goals after 70' with chance `0.02 * exp_factor * 3.0` → switch to Attacking" (ai.rs:269–278).

Action selection is a hard cascade in `resolve_action` based purely on `ball_zone` (zone_resolution.rs:19–27): box → shot; attacking third → dribble; midfield → midfield duel; defensive third → buildup pass.

## Per-player state

`PlayerData` (types.rs:33–81) — **f64 everywhere via attribute math, raw `u8` storage**, `id: String`. 22 attributes (pace, stamina, strength, agility, passing, shooting, tackling, dribbling, defending, positioning, vision, decisions, composure, aggression, teamwork, leadership, handling, reflexes, aerial, plus condition/fitness/traits). All `0..=100` `u8`. Reproduce-worthy: no `Vec2`, no `velocity`, no `facing`, no `on_ball_target`, no `team_role` beyond `Position::{Goalkeeper, Defender, Midfielder, Forward}` (types.rs:8–13) — just **4 buckets**.

Per-match mutable state is parked in two places: `LiveMatchState::player_conditions: HashMap<String, f64>` (mod.rs:220) for stamina, and `LiveMatchState::yellows: HashMap<String, u8>` / `sent_off: HashSet<String>` (mod.rs:193–194) for cards.

`PlayerSnap` (shared.rs:9–31) is a clone-then-release pattern: copy attributes into a snap to avoid borrow-checker conflicts when both teams are read mid-action. Same as our planned `Q32` snapshot pattern in spirit, just non-determinism-aware.

## Determinism analysis

**Partial. Same-process, same-OS, same-build only.** Evidence:

- `simulate_with_rng` accepts `R: Rng` — they test with `StdRng::seed_from_u64` (simulation_tests.rs:62, 270–276). `simulation_deterministic_with_same_seed` asserts equal `home_goals`, `away_goals`, `events.len()` — three integers, not a hash.
- `LiveMatchState` uses **`std::collections::HashMap` and `HashSet` heavily** (mod.rs:10, 193, 194, 220 + AI's `sent_off: &HashSet<String>` in ai.rs:209). Hash randomization → iteration order varies across runs on default-hash setups. The fact their det test passes means either DefaultHasher with `StdRng` happens to be stable enough in-process, or they never iterate these maps in a path that affects RNG-consumption order. Risky regardless.
- `f64` arithmetic pervasively (e.g., zone_resolution.rs:41–50, resolution.rs:45–54). No cross-OS guarantee for chained `f64` divisions and `(x).clamp(...)`.
- **No pinned canonical hash.** No `insta` snapshots. No `proptest`. The "deterministic" test (live_match_tests.rs:208–222) compares triplets across two runs in the same process — they explicitly DO NOT claim cross-OS reproducibility.
- `rand::rng()` (engine/mod.rs:17) — thread-RNG default path exists for callers who don't supply a seed.

For our purposes: OFM provides **no useful determinism playbook** — they punted on the hard problem we already committed to solving.

## Tactics + formations

`PlayStyle` is a 6-variant enum (types.rs:20–27): `Balanced, Attacking, Defensive, Possession, Counter, HighPress`. Encoded as **multiplicative modifiers per phase** in `play_style_modifier` (shared.rs:175–196):

```
(Attacking, Attack)    → 1.12   (Attacking, Defense)  → 0.93
(Possession, Midfield) → 1.15   (Counter, Attack)     → 1.18
(HighPress, Press)     → 1.20   (HighPress, Defense)  → 0.95
```

A team's `play_style` swaps in real-time via `ChangePlayStyle` command (mod.rs:50, 305–308). High press is identical to Possession structurally — it's just a different multiplier in the press phase.

Formation is a **`String` like "4-4-2"** (types.rs:122), parsed into defender/midfielder/forward counts at substitution time (live_match/substitution.rs reads it; tests at live_match_tests.rs:1043–1082 prove `"4-2-3-1"` collapses to 4 def / 5 mid / 1 fwd). **Roles below position bucket don't exist.** No CB/LB/CDM/CAM/RW distinction.

Home advantage: `home_mod(side, config) → 1.08 or 1.0` (shared.rs:202–207), applied as another multiplier on the same chain.

## Tests + verification

`tests/simulation_tests.rs` (1029 LoC) + `tests/live_match_tests.rs` (1745 LoC) — together ~2700 LoC of tests against ~1900 LoC of engine. Test style is **statistical / structural smoke**, not canonical:

- Structural: every match emits `KickOff`, `HalfTime`, `FullTime`, `SecondHalfStart` events (simulation_tests.rs:241–262).
- Identity over RNG: two runs with same seed produce same `(home_goals, away_goals, events.len())` triple (simulation_tests.rs:265–276). No hash.
- Distributional: 50–500 trial Monte-Carlo bounds — e.g. `strong_wins > weak_wins * 2` over 100 trials (simulation_tests.rs:399–419), `0.5 < avg_goals_per_game < 8.0` over 500 trials (simulation_tests.rs:693–710), possession-style teams average >48% possession across 100 sims (simulation_tests.rs:489–509).
- Property-like: events are chronological (simulation_tests.rs:559–579), shots ≥ shots_on_target (simulation_tests.rs:538–557), goal-events count == score (simulation_tests.rs:669–686).
- Invariant-style: bench size, sub guards, second-yellow → sent-off (live_match_tests.rs:1437–1472), can't sub a red-carded player (live_match_tests.rs:1522–1551).

**No insta. No proptest. No BLAKE3. No CI matrix.** Distributional tests use loops over seeds — slow + flaky in principle, fine in practice because attribute sums are stable enough.

## What's worth adopting (concrete, 5)

1. **Zone enum for ball location is genuinely useful even on top of continuous coordinates** (types.rs:251–308). Even if our 60Hz sim tracks `Q32`-vector positions, classifying the ball's current zone into 5 buckets (`AwayBox / AwayDefense / Midfield / HomeDefense / HomeBox`) gives commentary, salience tagging, and AI macros an O(1) symbolic handle. Cheap; high readability ROI.
2. **`MatchCommand` enum as the IPC sim ↔ UI boundary** (mod.rs:39–75): one Rust enum covers Substitute/ChangeFormation/ChangePlayStyle/SetFreeKickTaker/SetCornerTaker/SetPenaltyTaker/SetCaptain/PreMatchSwap. Maps cleanly to our "Tauri commands enqueue intents, sim consumes next tick" pattern (`Tauri/RULES.md` §2). Steal this shape verbatim.
3. **Per-half stoppage pre-computed at half-start** (simulation.rs:18, 42, 70) — eliminates a per-tick branch and makes total minutes a single integer state member, not a derived computation.
4. **Multiplicative tactic modifiers per phase** (shared.rs:175–196). Cleaner than additive role bonuses; cheap to compose. For our 24 signature-moves architecture, a similar `style_modifier × salience_weight × personality_bias` chain is well-trodden.
5. **`PlayerSnap` clone-before-resolve pattern** (shared.rs:9–63). For our `Q32` world we'll likely take a `Cow`-style snapshot at decision-cycle boundary anyway; OFM's design is the spiritual ancestor and works fine in tests.

## What's worth avoiding (concrete, 5)

1. **`HashMap<String, f64>` for player_conditions** (mod.rs:220). Direct violation of our `Sim/RULES.md` §1, §2. Their det test passes only because they don't iterate this map in an RNG-affecting path; that's a brittle accidental contract. We use `BTreeMap<PlayerId, Q32>`.
2. **`rng.random_range(1..=3u8)` for actions-per-minute** (simulation.rs:170). Abstract minutes-with-1-3-actions can't surface 24 signature moves at 60Hz cadence. This is the architectural choice that defines OFM's ceiling. Our `00-synthesis.md` 60Hz steering + 4Hz BT cadence is a strictly different sim — confirm, don't adopt.
3. **String IDs everywhere** (`player_id: String` in PlayerData, MatchEvent, HashMap keys, MatchCommand, etc.). Heap allocations on every action emit (`with_player(&id)` clones a String — event.rs:77–80). Use our `PlayerId(u32)` newtype + `&str` for content-pack-qualified refs at boundaries only.
4. **Two parallel engines** (`engine/mod.rs` + `live_match/`). Resolution logic is duplicated between resolution.rs:15–212 and zone_resolution.rs:14–282 with subtle drift (e.g., live includes condition-adjusted skill; one-shot doesn't). Single-source-of-truth: write one stepper, derive a "fast-forward" wrapper that loops it.
5. **Manager AI as hardcoded `if minute >= 70 && goal_diff <= -2 { ... } else if minute >= 75 && goal_diff == -1 { ... }` chains** (ai.rs:268–315). Untunable by content. Our personality-bias-vector + utility-scored decision design (synthesis 00) supersedes this; don't backslide.

## Open questions

- **How does cross-action serialization order work without canonical iteration?** OFM emits events in the order resolve_action calls `ctx.emit(...)`, which is RNG-driven within the action. Since they never iterate HashMaps to make state-affecting decisions, the per-action stream is RNG-deterministic in-process. Their HashMaps are end-of-match aggregate dumps in the report. Worth confirming by reading `report.rs` (345 LoC, unread in this pass) — if `MatchReport` iterates `yellows: HashMap<String, u8>` to build summary stats, that output is non-canonical even within their model.
- **Does the `live_match` AI ever consume formation strings beyond `"X-Y-Z"` and `"X-Y-Z-W"`?** Test at live_match_tests.rs:1085–1106 says invalid strings fall back to 4-4-2; the parser is in `substitution.rs` (unread, 201 LoC). If they support "4-4-1-1" or "3-4-3" the count math must be checked.
- **Penalty shootout determinism**: `live_match/penalty.rs` (157 LoC, unread) — does it exhaustively consume RNG in a fixed order? Their test only checks "shootout produces a winner" (live_match_tests.rs:351–380), not deterministic outcome.
- **Trait coverage**: shared.rs lists 18 traits across 7 contexts (`Shooting / Dribbling / Passing / Tackling / Goalkeeping / Foul / Midfield`). For our 24 signature moves we should map their trait taxonomy → our signature space to see if FW v1 carry-forward intent (REFERENCES.md) aligns or diverges.

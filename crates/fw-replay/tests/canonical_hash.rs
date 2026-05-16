//! The Phase-0 determinism gate. If this test fails on any platform in CI
//! (macos-14, windows-latest, ubuntu-22.04), NO further work proceeds
//! until it passes.
//!
//! See `docs/specs/determinism-gate.md` for the full contract. This file
//! is the load-bearing acceptance test for Phase 0 / T0 (Scaffold).
//!
//! Structure:
//!
//! 1. `PINNED_HASHES` is a `const &[(seed_hex, tick_count, [u8; 32])]`
//!    table. Each row is a seed + tick-budget + expected BLAKE3 digest of
//!    the canonical-encoded `MatchState` after that many ticks.
//!
//! 2. `smoke_seed_60_tick_canonical_hash_pinned` is the bedrock: run the
//!    smoke seed for 60 ticks and assert the hash equals `PINNED_60_TICK`.
//!
//! 3. `smoke_seed_runs_100_times_produce_one_hash` proves intra-process
//!    determinism — 100 fresh runs converge on a single hash.
//!
//! 4. `smoke_seed_corpus_fixture_matches_pinned_constant` asserts the RON
//!    fixture file's `expected_hash` agrees with the in-code pinned
//!    constant.
//!
//! 5. `insta` snapshot of the final-state `Debug` form for human-diffable
//!    change detection.
//!
//! ## Pinned-hash bootstrap protocol
//!
//! On first introduction, `PINNED_60_TICK` is a placeholder
//! (all-zeros). The first CI run produces the real hash; the developer
//! commits it into both this constant AND the RON fixture's
//! `expected_hash` field in the same commit. The
//! `#[ignore = "placeholder hash — fill on first CI green pass"]`
//! attribute is removed at the same time.
//!
//! Drift on any subsequent commit either signals a real regression OR an
//! intentional re-baseline. Re-baselines require a reviewer-approved
//! diff per `docs/specs/determinism-gate.md` §9 + the FW
//! `golden-replay-corpus.md` discipline.

// BTreeSet not BTreeSet — sim crates including their tests are bound by
// Sim/RULES.md §2 (no hash-randomized collections). Semantics identical
// for "count distinct hashes".
use std::collections::BTreeSet;
use std::path::PathBuf;

use blake3::Hasher;
use hex_literal::hex;
use serde::Deserialize;

use fw_core::{Q32, Seed, Tick};
use fw_match_sim::{MatchState, tick_match};

// T1-8: Replay corpus fixture #1 (`extended_seed_600_tick_*` tests) loads
// the real content tree via `ContentStore::load_sources`. The 60-tick
// smoke seed above runs `MatchState::initial(seed)` with no content; the
// 600-tick extended seed runs `initial_with_content(seed, &content)` so
// the bake exercises signature dispatcher + content-driven softmax paths.
// Imported here so the surface-area witness at the bottom of the file
// catches accidental rename / signature drift on the consumed APIs.
use fw_content::ContentStore;

// -------------------------------------------------------------------------
// Pinned-hash table (compile-time-enforced)
// -------------------------------------------------------------------------

/// Tier-A smoke seed. Same seed value as the FW C# reference at
/// `MatchSim.Tests/fixtures/replay-corpus/0xdeadbeefdeadbeef.json` so the
/// pre-pivot lineage is preserved (even though the underlying sim is a
/// fresh Rust impl and the hash will be brand-new).
const SMOKE_SEED: u64 = 0xDEAD_BEEF_DEAD_BEEF;
const SMOKE_TICK_COUNT: u32 = 60;

/// Pinned BLAKE3 of the 60-tick smoke seed's canonical state.
///
/// **Re-baseline history:**
/// - 2026-05-13 (T0-7) — initial pin `d6258107…d96b1a49` on the
///   stationary T0 fixture (22 players + ball at centre spot, no
///   integration). Cross-OS matrix agreement verified by T0-7b.
/// - 2026-05-13 (T1-2b-i) — **re-baselined to `0ddf91ef…c5722090`** per
///   ADR-0012 trigger #1 (canonical schema bump): `BallState` gained
///   `spin_{x,y,z}: Q32` (24 new bytes per ball encoding) AND
///   `tick_match` now advances ball physics each tick (the centre-spot
///   ball with zero velocity stays at zero, BUT the new spin fields
///   change the encoded layout regardless of values). Authorized by
///   the T1-2b-i task-spec in MEMORY.md.
/// - 2026-05-13 (T1-2b-ii) — **re-baselined to `5aea582b…cf5c544`** per
///   ADR-0012 trigger #1 (canonical schema bump): `MatchState` gained
///   `decision_slots: [u8; 22]` (22 bytes), `interrupt_cooldown_until:
///   [Tick; 22]` (176 bytes), `team_tactic_states: [TeamTacticState; 2]`
///   (~18 bytes). Wire-format VERSION bumped from 1 → 2. Tactic-FSM
///   heartbeat now runs every 30 ticks in `tick_match`. Authorized by
///   the T1-2b-ii task-spec in MEMORY.md.
/// - 2026-05-13 (T1-2b-iii-a) — **re-baselined to `c0b5e395…c1430ff`** per
///   ADR-0012 trigger #1 (canonical schema bump): `PlayerState` gained
///   `role: Role` (u8, 1 byte) + `role_state: u8` (1 byte) +
///   `local_decision_counter: u32` (4 bytes LE). Net: +6 bytes × 22 players
///   = +132 bytes per match-state. Wire-format VERSION bumped 2 → 3.
///   `MatchState::initial` now places players at 4-3-3 formation positions
///   (replacing the T0 placeholder grid). `tick_match` now dispatches
///   per-player BT / GK-FSM decisions via `dispatch_tick`. Authorized by
///   the T1-2b-iii-a task-spec in MEMORY.md.
/// - 2026-05-13 (T1-2b-iii-b) — **re-baselined to `b3b0e64f…d4da1169`** per
///   ADR-0012 trigger #1 (canonical schema bump): `PlayerState` gained
///   `attributes: PlayerAttributes` (55 × i64 LE = 440 bytes per player).
///   Net: +440 bytes × 22 players = +9680 bytes per match-state. Wire-format
///   VERSION bumped 3 → 4. `PlayerAttributes::mid_range_baseline()` added
///   to `fw-core`. All 5 utility primitives added to `fw-match-sim::utility`:
///   xG logistic, xT 16×12 grid, Spearman pitch-control, product-form
///   pressing, top-N softmax. `fw-core::math` gained `sigmoid_q32` +
///   `exp_q32` 257-entry LUTs. Authorized by the T1-2b-iii-b task-spec in
///   MEMORY.md.
/// - 2026-05-13 (T1-2b-iii-c) — **re-baselined to `c392bac5…14c7f7d2`** per
///   ADR-0012 trigger #3 (sim behavior change with documented intent): outfield
///   dispatch now uses utility-scored softmax selection (`select_outfield_intent`)
///   instead of the stub `MoveToFormationPosition` leaf. Player velocities per tick
///   differ → canonical state changes. No schema change (no new fields);
///   the change is purely in the velocity values written per player per tick.
///   `bt/personality_bias.rs` (k₁..k₁₄ + 7 bias helpers + PT divisor),
///   `bt/on_ball.rs` (7 utility sites), `bt/off_ball.rs` (5 utility sites),
///   `bt/reactive.rs` (4 predicates, not wired), goalkeeper_fsm.rs GK
///   variants, `PlayerIntent` expansion (17 new variants). Authorized by the
///   T1-2b-iii-c task-spec in MEMORY.md.
/// - 2026-05-13 (T1-2b-iii-c P0+P1 fix pass) — **re-baselined to `235f6c5e…181288d`**
///   per ADR-0012 trigger #3 (sim behavior change, authorized by T1-2b-iii-c fix-pass
///   spec). Fixes applied:
///   P0-1: all 12 utility sites corrected to read EXACTLY the attrs from
///   bt-attribute-binding.md (was reading ~10 non-spec attrs). Binding-correctness
///   tests added (12 pairs). P0-2: `evaluate_transitions` in goalkeeper_fsm.rs
///   now uses real ball-position predicates (ShotStopping / SweeperKeeperRush /
///   DistributingFromHand / InBoxPositioning) — was always InBoxPositioning.
///   P0-3: position integration in lib.rs uses bare operators not checked_mul/add.
///   P0-4: `xt_delta` uses direct subtraction, returning negative for backpasses.
///   P1-1: debug_assert on bias helper raw inputs. P1-2: unreachable!() fallback
///   in select_outfield_intent. P1-4: SeedLayer::UtilityTieBreak for softmax RNG.
///   P2-2: DefenderPressure + IsProgressive newtypes for arg-safety.
/// - 2026-05-14 (T1-2b-iii-d PlayerSeparation) — **re-baselined to `1db6020c…59c798`**
///   per ADR-0012 trigger #3 (sim behavior change, authorized by T1-2b-iii-d task-spec
///   in MEMORY.md). The player-separation positional-correction pass (step 6 in
///   `tick_match`) adjusts player positions after velocity integration, so canonical
///   state (which encodes pos_x/pos_y) changes. No schema bump; only position values
///   change. Separation is purely deterministic (Q32 arithmetic, no RNG).
/// - 2026-05-15 (T1-2b-iv signature dispatcher) — **re-baselined to `18f1776c…a5d048`**
///   per ADR-0012 trigger #1 (canonical schema bump): `MatchState` gained three new
///   canonical fields: `signature_cooldowns: BTreeMap<(PlayerSlot, SignatureId), Tick>`,
///   `signature_firing: [Option<SignatureFiring>; 22]`,
///   `signature_first_fired_seen: BTreeSet<(PlayerSlot, SignatureId)>`.
///   Canonical encoder VERSION bumped 4 → 5; three new encoding sections appended
///   after ball. Authorized by the T1-2b-iv task-spec in MEMORY.md.
/// - 2026-05-15 (T1-2b-fix post-Codex-audit) — **re-baselined to `dbe4f49b…85f2`**
///   per ADR-0012 trigger #1 (canonical schema bump): P1-2 fix: per-player
///   `signature_candidates` now encoded in canonical state (len + per-entry id_len
///   + id_bytes + affinity i64). Canonical encoder VERSION bumped 5 → 6.
///   - `signature_firing` changed from `[Option<SignatureFiring>; 22]`
///     to `[[Option<SignatureFiring>; 4]; 22]` (stacking categories per lane).
///   - `SeedLayer` + `seed_fn` moved to `fw-core::seed` per ADR-0009.
///   - `TriggerFn` signature changed `→ bool` to `→ Q32` (fit-score) per P1-6.
///   - `tick_goalkeeper` gained `player: &PlayerState` param per P1-3/P1-4.
///   - Authorized by the T1-2b-fix task-spec in MEMORY.md.
/// - 2026-05-15 (T1-2b-fix P1-5 + P2-9 attribute-binding corrections) —
///   **re-baselined to `d376ba26…fa93`** per ADR-0012 trigger #3 (sim behavior
///   change with documented intent): six BT decision sites corrected to match
///   `bt-attribute-binding.md` spec exactly. Bias helper multiplicative forms
///   changed:
///   - `apply_lay_off_bias`: 2 → 1 factor (selflessness only; was safe_pass proxy
///     with risk_appetite inverse + selflessness).
///   - `apply_mark_bias`: 2 → 1 factor (determination only; was cover proxy with
///     determination + work_rate).
///   - `apply_run_off_ball_bias`: k₉ (0.45) → k₁₀ (0.35) first factor coefficient
///     (aggression → work_rate for first factor).
///   - `apply_cross_bias`, `apply_hold_formation_bias`: coefficient-equivalent
///     swap (no numeric change at uniform attrs, but different attrs read).
///   - `utility_shoot`: `long_shots` demoted from 4th primary factor to secondary
///     modifier `(1 + 0.30×long_shots)` to keep primary product magnitude consistent.
///   - 4 reactive predicates rewritten to spec (P2-9; not wired into dispatch, so
///     no simulation output change — predicates unused until T1-4).
///   - Authorized by the T1-2b-fix P1-5/P2-9 task-spec in MEMORY.md.
/// - 2026-05-16 (T1-4a MatchEvent emission) — **re-baselined to `02ab97d0...27e686`**
///   per ADR-0012 trigger #1 (canonical schema bump): `MatchState` gained
///   `match_events: Vec<MatchEvent>` (persistent canonical event stream) and
///   `match_end_tick: Tick`. `signature_memory_events` field REMOVED (was transient
///   scratch buffer; subsumed by `match_events`). Encoder VERSION bumped 6->7.
///   Authorized by T1-4a task-spec in MEMORY.md.
/// - 2026-05-16 (T1-3.5 ball mutation + possession + goal detection) —
///   **re-baselined to `782fcde6...8c0f`** per ADR-0012 trigger #1 (canonical
///   schema bump + behavioral change): (1) ball physics coordinate convention
///   corrected — pos_z is now the altitude axis (gravity/bounce on -vel_z);
///   pos_y is the lateral pitch axis (no gravity); rolling friction acts on
///   vx + vy (not vx + vz). (2) `MatchState` gained two new fields:
///   `possession: Option<PlayerSlot>` and `last_touched_by: Option<PlayerSlot>`.
///   (3) Encoder VERSION bumped 7→8; two new sections appended after match_events.
///   (4) `tick_match` step ordering changed: goal detection + OOB clamp now run
///   BEFORE ball physics (steps 2+3 before step 4).
///   `apply_intent` now mutates ball state per Shot/Pass/Dribble/GK intents.
///   Authorized by T1-3.5 task-spec in MEMORY.md.
/// - 2026-05-16 (T1-3.6 BT carrier routing fix) — **re-baselined to
///   `ddccaf88...00b3`** per ADR-0012 trigger #1 (authorized behavioral change):
///   `PlayerRoleState::evaluate_transitions` now routes the possession holder
///   into `InPossession` state (was always returning `self` — the bug that
///   caused ball to never move). A carrier-routing pre-pass was added to
///   `dispatch_tick` that runs BEFORE the per-slot decision loop, updating
///   ALL 22 players' role states based on current possession every tick.
///   This means the carrier fires on-ball BT candidates (Pass/Shot/Dribble),
///   producing actual ball motion. The prior hash `782fcde6...8c0f` was the
///   hash of a BROKEN match (ball never moved); this new hash is the hash of
///   football actually happening. `MatchFrameDto` gained `pub possession:
///   Option<u8>` projected from `state.possession()`. Insta snapshot
///   updated to reflect Pass events starting at tick 5.
///   Authorized by T1-3.6 task-spec in MEMORY.md.
/// - 2026-05-16 (T1-15 goal scoring) — **re-baselined to `2f14a562...cb27`**
///   per ADR-0012 trigger #3 (sim behavior change with documented intent):
///   Ball physics tuned so shots reach the goal line (reduced rolling_friction
///   to 0.002 + linear_drag to 0.005 per-tick). GK loose-ball chase added
///   (preempt_check routes GK toward ball when within 10m of own goal line).
///   Loose-ball pickup extended to include GK when ball is near their goal.
///   MAX_PLAYER_SPEED raised from 5 to 8 m/s (brisk run) for faster convergence.
///   Preempt_check limited to nearest-2-chasers per team (preserves formation
///   Y-spread). Smoke seed now produces 4 goals (2-2) in 600 ticks.
///   Authorized by T1-15 task-spec in MEMORY.md.
/// - 2026-05-16 (T1-16 shoot utility contract) — **re-baselined to
///   `fcccb840...a751`** per ADR-0012 trigger #3 (sim behavior change with
///   documented intent): shoot proximity scoring now uses `fw_core::GOAL_LINE_X`
///   instead of the stale ±45m literal, and shoot utility is clamped back into
///   the `[0, 1]` softmax domain after the proximity and personality-bias
///   multipliers. GK transition goal-line constants also now use
///   `GOAL_LINE_X`. Authorized by the Codex Tier-2 pre-/done audit response.
///
/// Re-baselining requires: task-spec authorization + simultaneous update
/// of this constant + the RON fixture's `expected_hash` field + commit
/// body noting the new short BLAKE3 + the reason. Drift not authorized
/// by the task spec is a real determinism regression — investigate before
/// re-pinning. See `docs/specs/determinism-gate.md` §9 for the full
/// re-baselining procedure.
const PINNED_60_TICK: [u8; 32] =
    hex!("fcccb840b5868a4ed55c019c353a1d5496259073e2d88bf7abd97d9bdca7a751");

/// The corpus table. New seeds append here as the corpus grows. Each row:
/// `(seed_hex_string, tick_count, expected_blake3_digest)`.
///
/// Currently only the Tier-A smoke seed is pinned. Tier-D (RC gate)
/// expands this list per `docs/specs/determinism-gate.md` §11.
#[allow(dead_code)] // referenced by future corpus-iteration tests
const PINNED_HASHES: &[(&str, u32, [u8; 32])] = &[
    ("0xdeadbeefdeadbeef", SMOKE_TICK_COUNT, PINNED_60_TICK),
    // T1-8: corpus fixture #1 — content-loaded 600-tick run.
    ("0xfeedbeefcafefade", EXTENDED_TICK_COUNT, PINNED_600_TICK),
];

// -------------------------------------------------------------------------
// The Phase-0 acceptance test
// -------------------------------------------------------------------------

#[test]
fn smoke_seed_60_tick_canonical_hash_pinned() {
    let seed = Seed::from_u64(SMOKE_SEED);
    let mut state = MatchState::initial(seed);
    for _ in 0..SMOKE_TICK_COUNT {
        state = tick_match(state, &std::collections::BTreeMap::new());
    }

    let bytes = state.encode_canonical();
    let hash: [u8; 32] = blake3::hash(&bytes).into();

    assert_eq!(
        hash,
        PINNED_60_TICK,
        "\nCanonical-state hash drift on the Phase-0 smoke seed.\n\
         Seed:        0x{SMOKE_SEED:016x}\n\
         Ticks:       {SMOKE_TICK_COUNT}\n\
         Expected:    {}\n\
         Actual:      {}\n\
         \n\
         If this is a real regression, find and fix the determinism leak.\n\
         If this is an intentional re-baseline, update both PINNED_60_TICK\n\
         here AND the `expected_hash` field of\n\
         crates/fw-replay/fixtures/0xdeadbeefdeadbeef.ron in the same commit,\n\
         and call out the re-baseline in the commit body per\n\
         docs/specs/determinism-gate.md §9.\n",
        hex_string(&PINNED_60_TICK),
        hex_string(&hash),
    );
}

// -------------------------------------------------------------------------
// Placeholder-footgun guard — the pinned constant is all-zeros until the
// first CI green run fills it (see file-header bootstrap protocol). That
// creates a footgun: if the sim coincidentally produced an all-zero
// canonical state, the pinned test would falsely pass.
//
// This non-ignored test asserts the actual encoded buffer is non-empty
// AND the hash is non-zero on the smoke seed's initial state. It will
// stay live forever — once the real hash is pinned and the
// `#[ignore]` is removed from `smoke_seed_60_tick_canonical_hash_pinned`,
// this guard remains as cheap defence-in-depth.
// -------------------------------------------------------------------------

#[test]
fn smoke_seed_canonical_hash_is_nonzero() {
    let seed = Seed::from_u64(SMOKE_SEED);
    let state = MatchState::initial(seed);
    let bytes = state.encode_canonical();

    assert!(
        !bytes.is_empty(),
        "canonical encoder produced an empty buffer — encoder is broken"
    );
    assert!(
        bytes.iter().any(|&b| b != 0),
        "canonical encoder produced an all-zero buffer — encoder is broken"
    );

    let hash: [u8; 32] = blake3::hash(&bytes).into();
    assert_ne!(
        hash, [0u8; 32],
        "BLAKE3 of a non-empty buffer produced all zeros — \
         hashing layer is broken (cosmic coincidence is preferred over \
         a real bug here; investigate)"
    );
}

// -------------------------------------------------------------------------
// Intra-process determinism — 100 runs, one hash
// -------------------------------------------------------------------------

#[test]
fn smoke_seed_runs_100_times_produce_one_hash() {
    // 100 fresh identical runs. Single distinct hash means no hidden
    // non-determinism (HashMap iteration / thread_rng / SystemTime /
    // pointer-address-based ordering).
    //
    // This test runs cheaply (60 ticks × 100 runs ≈ 6k tick evaluations
    // on a tiny state) and catches the most common determinism leaks
    // BEFORE the cross-platform CI matrix has to disagree to surface them.
    let mut distinct: BTreeSet<[u8; 32]> = BTreeSet::new();
    for _ in 0..100 {
        let seed = Seed::from_u64(SMOKE_SEED);
        let mut state = MatchState::initial(seed);
        for _ in 0..SMOKE_TICK_COUNT {
            state = tick_match(state, &std::collections::BTreeMap::new());
        }
        let bytes = state.encode_canonical();
        let hash: [u8; 32] = blake3::hash(&bytes).into();
        distinct.insert(hash);
    }
    assert_eq!(
        distinct.len(),
        1,
        "100 runs of the same seed produced {} distinct hashes — \
         hidden non-determinism. Hashes: {:?}",
        distinct.len(),
        distinct.iter().map(|h| hex_string(h)).collect::<Vec<_>>(),
    );
}

// -------------------------------------------------------------------------
// Fixture / in-code agreement — corpus drift detector
// -------------------------------------------------------------------------

/// Minimal subset of the RON fixture schema (full schema in
/// `docs/specs/determinism-gate.md` §8). We only need the fields this
/// test asserts on; `#[serde(default)]` is forward-compatible with
/// future field additions.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct ReplayCorpusEntry {
    schema_version: u32,
    seed: String,
    tick_count: u32,
    expected_hash: String,
}

#[test]
fn smoke_seed_corpus_fixture_matches_pinned_constant() {
    // The fixture file (consumed by `scripts/fw replay --compare-corpus`)
    // and the in-code constant (consumed by `cargo test` in CI) MUST
    // agree. This test prevents silent drift between the two.
    let fixture_path = locate_fixture("0xdeadbeefdeadbeef.ron");
    let raw = std::fs::read_to_string(&fixture_path).unwrap_or_else(|err| {
        panic!(
            "corpus fixture missing at {}: {err}\n\
             Phase-0 / T0 acceptance gate requires \
             crates/fw-replay/fixtures/0xdeadbeefdeadbeef.ron per \
             docs/specs/determinism-gate.md §8.",
            fixture_path.display()
        )
    });
    let entry: ReplayCorpusEntry =
        ron::from_str(&raw).expect("failed to parse corpus fixture as RON");

    assert_eq!(
        entry.schema_version, 1,
        "fixture schema_version drift — review the spec before bumping"
    );
    assert_eq!(entry.seed, "0xdeadbeefdeadbeef");
    assert_eq!(entry.tick_count, SMOKE_TICK_COUNT);

    let fixture_hash =
        parse_blake3_hex(&entry.expected_hash).expect("fixture expected_hash field is malformed");
    assert_eq!(
        fixture_hash,
        PINNED_60_TICK,
        "Drift between RON fixture and in-code PINNED_60_TICK.\n\
         Fixture:    {}\n\
         In-code:    {}\n\
         These must be updated together; updating only one is forbidden \
         per docs/specs/determinism-gate.md §9.",
        entry.expected_hash,
        format_args!("blake3:{}", hex_string(&PINNED_60_TICK)),
    );
}

// -------------------------------------------------------------------------
// Meta-guard — bedrock pinned-hash test cannot be silently `#[ignore]`d
//
// Codex full-project audit P0 (2026-05-13): a contributor (or future
// Claude) could add `#[ignore]` to the bedrock pinned-hash test, and
// `cargo test` would return 0 (ignored tests count as passes) — the
// Phase-0 determinism gate would be silently disabled in CI.
//
// This test reads its own source file at compile-time via `include_str!`
// and asserts the bedrock function's attribute block does NOT contain
// `#[ignore]`. It fails loudly the moment someone tries to disable the
// bedrock. The CI workflow `determinism-gate.yml` adds a separate
// belt-and-braces grep that catches the same condition even if this
// meta-test is removed; the commit-time hook `canonical-hash-guard.sh`
// adds a third layer at commit time.
//
// Other tests in this file (e.g. `smoke_seed_final_state_snapshot`) are
// allowed to be `#[ignore]`d; only the bedrock is locked.
// -------------------------------------------------------------------------

#[test]
fn bedrock_pinned_test_is_not_ignored() {
    const SRC: &str = include_str!("canonical_hash.rs");
    const BEDROCK_FN_NAME: &str = "smoke_seed_60_tick_canonical_hash_pinned";

    // Locate the bedrock function.
    let fn_marker = format!("fn {BEDROCK_FN_NAME}");
    let fn_pos = SRC.find(&fn_marker).unwrap_or_else(|| {
        panic!(
            "could not find `{fn_marker}` in canonical_hash.rs — \
             has the bedrock function been renamed? Update \
             bedrock_pinned_test_is_not_ignored::BEDROCK_FN_NAME to match \
             OR restore the bedrock test name."
        )
    });

    // Find the most recent #[test] attribute before fn_pos. Everything
    // between that attribute and `fn <name>` is the attribute block.
    let before = &SRC[..fn_pos];
    let test_attr_pos = before.rfind("#[test]").unwrap_or_else(|| {
        panic!(
            "no #[test] attribute precedes `fn {BEDROCK_FN_NAME}` — \
             file structure changed unexpectedly. The meta-guard cannot \
             verify the bedrock is live; investigate."
        )
    });

    let attr_block = &SRC[test_attr_pos..fn_pos];
    assert!(
        !attr_block.contains("#[ignore"),
        "BLOCKED: the bedrock pinned-hash test `{BEDROCK_FN_NAME}` is marked #[ignore].\n\
         \n\
         The Phase-0 determinism gate is silently disabled. Remove the\n\
         #[ignore] attribute. If a re-baseline is genuinely required,\n\
         follow docs/specs/determinism-gate.md §9 — do NOT bypass via\n\
         #[ignore].\n\
         \n\
         Codex audit P0 (2026-05-13): this meta-guard catches the disable\n\
         at `cargo test` time. The CI workflow + commit hook add\n\
         additional layers."
    );
}

// -------------------------------------------------------------------------
// Snapshot — human-diffable change detection
// -------------------------------------------------------------------------

#[test]
#[ignore = "snapshot baseline created alongside first CI green hash"]
fn smoke_seed_final_state_snapshot() {
    // `insta` produces a textual snapshot of the final state. Drift
    // surfaces in PR as a readable diff (positions changed, score
    // changed, tick advanced) rather than just a hex-string mismatch.
    // The .snap file lives next to this test and is committed.
    let seed = Seed::from_u64(SMOKE_SEED);
    let mut state = MatchState::initial(seed);
    for _ in 0..SMOKE_TICK_COUNT {
        state = tick_match(state, &std::collections::BTreeMap::new());
    }
    insta::assert_debug_snapshot!(state);
}

// =========================================================================
// T1-8: Replay corpus fixture #1 — extended-seed 600-tick canonical hash
// =========================================================================
//
// Broadens cross-OS determinism coverage from "60 ticks, bare init,
// empty signature definitions" (the SMOKE pin above) to "600 ticks,
// content-loaded init, real signature definitions". Catches a different
// class of determinism leaks — softmax under the content-driven utility
// stack, signature firings + cooldowns over a long horizon, ball physics
// + possession state machine through many KickOff cycles.
//
// The frontend already uses `0xfeedbeefcafefade` as the default Match-page
// seed example; this row consecrates it as the long-horizon canonical pin.
//
// Re-baseline cadence: same as PINNED_60_TICK — any canonical-schema bump
// (new field on MatchState / BallState / PlayerState / MatchEvent) drifts
// this hash; re-pin alongside the existing 60-tick pin via the rebase
// procedure at `docs/specs/determinism-gate.md` §9.
// -------------------------------------------------------------------------

const EXTENDED_SEED: u64 = 0xFEED_BEEF_CAFE_FADE;
const EXTENDED_TICK_COUNT: u32 = 600;
const EXTENDED_FIXTURE_NAME: &str = "0xfeedbeefcafefade.ron";

/// Pinned BLAKE3 of the extended seed's canonical state after 600 ticks
/// with content-loaded init (signature definitions wired, slot 7's
/// signature_candidates populated from the AM template).
///
/// **Re-baseline history:**
/// - 2026-05-16 (T1-8) — initial pin (filled by chunk 1 bake). The
///   extended seed runs `MatchState::initial_with_content` against the
///   committed `content/sources/` tree + passes `content.signature_definitions`
///   to every `tick_match` call.
/// - 2026-05-16 (T1-15 goal scoring) — **re-baselined to `268984...e95ae`**
///   per ADR-0012 trigger #3 (sim behavior change with documented intent):
///   Same changes as PINNED_60_TICK above (GK chase, 2-chaser preempt,
///   MAX_PLAYER_SPEED 5→8, rolling_friction 0.002, goal line target 52m).
///   600-tick run now produces 4 goals (2-2) on this seed.
///   Authorized by T1-15 task-spec in MEMORY.md.
/// - 2026-05-16 (T1-16 shoot utility contract) — **re-baselined to
///   `9353bd25...47eb`** per ADR-0012 trigger #3. Same changes as
///   PINNED_60_TICK above: shoot utility clamp + `GOAL_LINE_X` alignment.
///   Authorized by the Codex Tier-2 pre-/done audit response.
///
/// Re-baselining: update this constant AND the `expected_hash` field of
/// `crates/fw-replay/fixtures/0xfeedbeefcafefade.ron` in the same commit,
/// per `docs/specs/determinism-gate.md` §9 — the same protocol that
/// governs PINNED_60_TICK above.
const PINNED_600_TICK: [u8; 32] =
    hex!("9353bd257d4da92092407355e3c2b32cc6e91abc81664d0015336ebe812947eb");

#[test]
fn extended_seed_600_tick_canonical_hash_pinned() {
    let content_root = workspace_content_root();
    let content = ContentStore::load_sources(&content_root).expect(
        "content/sources should load — fw-content/tests/fixtures_load.rs covers \
         the same path with the same expectation",
    );

    let seed = Seed::from_u64(EXTENDED_SEED);
    let mut state = MatchState::initial_with_content(seed, &content)
        .expect("initial_with_content should succeed against the committed corpus");
    for _ in 0..EXTENDED_TICK_COUNT {
        state = tick_match(state, &content.signature_definitions);
    }

    let bytes = state.encode_canonical();
    let hash: [u8; 32] = blake3::hash(&bytes).into();

    assert_eq!(
        hash,
        PINNED_600_TICK,
        "\nCanonical-state hash drift on the extended seed (T1-8 corpus fixture #1).\n\
         Seed:        0x{EXTENDED_SEED:016x}\n\
         Ticks:       {EXTENDED_TICK_COUNT}\n\
         Expected:    {}\n\
         Actual:      {}\n\
         \n\
         If this is a real regression, find and fix the determinism leak.\n\
         If this is an intentional re-baseline, update both PINNED_600_TICK\n\
         here AND the `expected_hash` field of\n\
         crates/fw-replay/fixtures/{EXTENDED_FIXTURE_NAME} in the same commit,\n\
         and call out the re-baseline in the commit body per\n\
         docs/specs/determinism-gate.md §9.\n",
        hex_string(&PINNED_600_TICK),
        hex_string(&hash),
    );
}

/// Intra-process determinism — 10 fresh runs converge on a single hash.
///
/// 10× (vs the 60-tick smoke's 100×) keeps the total cost ≈ 6k tick-
/// evaluations — same budget as the 60-tick × 100-runs smoke determinism
/// test. The extended seed runs significantly more sim code per tick
/// (signature dispatcher, content-driven softmax, ball physics through
/// possession transfers) so each tick costs more wall-clock — 10 runs is
/// enough to catch the determinism leak classes (HashMap iteration /
/// thread_rng / SystemTime / pointer-address-based ordering) that would
/// surface as multiple distinct hashes.
#[test]
fn extended_seed_runs_10_times_produce_one_hash() {
    let content_root = workspace_content_root();
    let content = ContentStore::load_sources(&content_root).expect("content/sources should load");

    let mut distinct: BTreeSet<[u8; 32]> = BTreeSet::new();
    for _ in 0..10 {
        let seed = Seed::from_u64(EXTENDED_SEED);
        let mut state = MatchState::initial_with_content(seed, &content)
            .expect("initial_with_content should succeed");
        for _ in 0..EXTENDED_TICK_COUNT {
            state = tick_match(state, &content.signature_definitions);
        }
        let bytes = state.encode_canonical();
        let hash: [u8; 32] = blake3::hash(&bytes).into();
        distinct.insert(hash);
    }
    assert_eq!(
        distinct.len(),
        1,
        "10 runs of the extended seed produced {} distinct hashes — \
         hidden non-determinism. Hashes: {:?}",
        distinct.len(),
        distinct.iter().map(|h| hex_string(h)).collect::<Vec<_>>(),
    );
}

#[test]
fn extended_seed_corpus_fixture_matches_pinned_constant() {
    // Mirrors `smoke_seed_corpus_fixture_matches_pinned_constant` above —
    // the on-disk RON + the in-code const MUST agree; updating only one
    // is forbidden per docs/specs/determinism-gate.md §9.
    let fixture_path = locate_fixture(EXTENDED_FIXTURE_NAME);
    let raw = std::fs::read_to_string(&fixture_path).unwrap_or_else(|err| {
        panic!(
            "corpus fixture missing at {}: {err}\n\
             T1-8 acceptance gate requires \
             crates/fw-replay/fixtures/{EXTENDED_FIXTURE_NAME} per \
             docs/MASTER_PLAN.md T1-8.",
            fixture_path.display()
        )
    });
    let entry: ReplayCorpusEntry =
        ron::from_str(&raw).expect("failed to parse extended corpus fixture as RON");

    assert_eq!(
        entry.schema_version, 1,
        "extended fixture schema_version drift — review the spec before bumping"
    );
    assert_eq!(entry.seed, "0xfeedbeefcafefade");
    assert_eq!(entry.tick_count, EXTENDED_TICK_COUNT);

    let fixture_hash = parse_blake3_hex(&entry.expected_hash)
        .expect("extended fixture expected_hash field is malformed");
    assert_eq!(
        fixture_hash,
        PINNED_600_TICK,
        "Drift between RON fixture and in-code PINNED_600_TICK.\n\
         Fixture:    {}\n\
         In-code:    {}\n\
         These must be updated together; updating only one is forbidden \
         per docs/specs/determinism-gate.md §9.",
        entry.expected_hash,
        format_args!("blake3:{}", hex_string(&PINNED_600_TICK)),
    );
}

// -------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------

/// Locate the RON fixture relative to the crate root. `cargo test` runs
/// from the workspace target dir; the fixture lives at
/// `crates/fw-replay/fixtures/<seed>.ron`.
fn locate_fixture(filename: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("fixtures").join(filename)
}

/// Locate the workspace-root `content/` directory the same way
/// `crates/fw-content/tests/fixtures_load.rs` does. `CARGO_MANIFEST_DIR`
/// = `crates/fw-replay`; workspace root = `../..`. Used by the T1-8
/// extended-seed tests to load the on-disk content corpus.
fn workspace_content_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

/// Parse a `"blake3:<64-hex-chars>"` string into a 32-byte digest.
fn parse_blake3_hex(s: &str) -> Result<[u8; 32], String> {
    let stripped = s
        .strip_prefix("blake3:")
        .ok_or_else(|| format!("missing 'blake3:' prefix in {s:?}"))?;
    if stripped.len() != 64 {
        return Err(format!(
            "expected 64 hex chars after 'blake3:'; got {} in {s:?}",
            stripped.len()
        ));
    }
    let mut out = [0u8; 32];
    for (i, chunk) in stripped.as_bytes().chunks(2).enumerate() {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

fn hex_nibble(b: u8) -> Result<u8, String> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(format!("invalid hex nibble: {b:#x}")),
    }
}

/// Render a 32-byte digest as lowercase hex (no prefix).
fn hex_string(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

// -------------------------------------------------------------------------
// Compile-time sanity: the imports below force us to fail-fast if the
// surface area drifts. If `Q32`, `Seed`, `Tick`, `MatchState`, or
// `tick_match` are renamed without updating this test, compilation breaks
// loudly. (Better than a runtime error in CI.)
// -------------------------------------------------------------------------

#[allow(dead_code)]
fn _surface_area_witness() {
    let _: Q32 = Q32::zero();
    let _: Tick = Tick::ZERO;
    let _: Seed = Seed::from_u64(0);
    let _: MatchState = MatchState::initial(Seed::from_u64(0));
    let _hasher: Hasher = Hasher::new();
}

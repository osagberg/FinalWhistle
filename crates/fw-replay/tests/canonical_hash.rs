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
///
/// Re-baselining requires: task-spec authorization + simultaneous update
/// of this constant + the RON fixture's `expected_hash` field + commit
/// body noting the new short BLAKE3 + the reason. Drift not authorized
/// by the task spec is a real determinism regression — investigate before
/// re-pinning. See `docs/specs/determinism-gate.md` §9 for the full
/// re-baselining procedure.
const PINNED_60_TICK: [u8; 32] =
    hex!("c0b5e3955662ccd3e56b75072d4dad71366f2e58f806ff89013aaf7eac1430ff");

/// The corpus table. New seeds append here as the corpus grows. Each row:
/// `(seed_hex_string, tick_count, expected_blake3_digest)`.
///
/// Currently only the Tier-A smoke seed is pinned. Tier-D (RC gate)
/// expands this list per `docs/specs/determinism-gate.md` §11.
#[allow(dead_code)] // referenced by future corpus-iteration tests
const PINNED_HASHES: &[(&str, u32, [u8; 32])] =
    &[("0xdeadbeefdeadbeef", SMOKE_TICK_COUNT, PINNED_60_TICK)];

// -------------------------------------------------------------------------
// The Phase-0 acceptance test
// -------------------------------------------------------------------------

#[test]
fn smoke_seed_60_tick_canonical_hash_pinned() {
    let seed = Seed::from_u64(SMOKE_SEED);
    let mut state = MatchState::initial(seed);
    for _ in 0..SMOKE_TICK_COUNT {
        state = tick_match(state);
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
            state = tick_match(state);
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
        state = tick_match(state);
    }
    insta::assert_debug_snapshot!(state);
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

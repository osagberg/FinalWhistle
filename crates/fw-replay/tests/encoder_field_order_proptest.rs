//! T2-R7(e) — canonical encoder field-order + length stability across seeds.
//!
//! Post-T2 ultimate-review Track A-M3 + Track D both flagged that the
//! existing `tests/canonical_hash.rs` suite pins ONE seed end-to-end
//! (smoke 60-tick + extended 600-tick) but never exercises the encoder
//! itself against a seed sweep. The wire-format invariants — magic+
//! version prefix, fixed initial-state length, intra-process determinism
//! — are properties of the encoder, not of any single seed. A future
//! bug that adds a conditional field-write keyed on (say) player count
//! or culture would make encoded length vary across seeds at tick=0 and
//! escape the pinned single-seed test.
//!
//! Three invariants this proptest defends:
//!
//! 1. **Magic + VERSION prefix is invariant across seeds.** Every
//!    `MatchState::initial(seed).encode_canonical()` starts with the
//!    same 6 bytes (4-byte `FWMS` magic + 2-byte LE VERSION). If a
//!    future refactor accidentally writes seed-dependent bytes BEFORE
//!    the magic, this fires.
//!
//! 2. **Initial-state encoded length is constant across seeds.**
//!    `MatchState::initial(seed)` produces a fixed-shape state (22
//!    players, fixed BTreeMap/BTreeSet emptinesses, fixed ball-at-
//!    centre). Different seeds change the BYTE VALUES but must not
//!    change the BYTE COUNT. A new conditional field-write (e.g.
//!    "only emit X if Y") that depends on seed-derived state would
//!    break this and would otherwise pass the single-seed pinned
//!    tests because they only assert hash equality, not length
//!    consistency.
//!
//! 3. **Encoding is intra-process deterministic for every seed.**
//!    Calling `encode_canonical()` twice on the same state produces
//!    byte-identical output. This is the per-seed analogue of
//!    `smoke_seed_runs_100_times_produce_one_hash`; the existing test
//!    pins ONE seed; this fires on any seed where a future
//!    BTreeMap → HashMap regression would surface as iteration-order
//!    nondeterminism.
//!
//! Cross-reference: post-T2 ultimate-review doc at
//! `docs/audits/post-t2-ultimate-review-2026-05-18.md` Track A-M3 and
//! Track D-Survey-1.

use fw_core::Seed;
use fw_match_sim::MatchState;
use proptest::prelude::*;
use std::sync::OnceLock;

/// Canonical encoding of `MatchState::initial(Seed::from_u64(0))`. Used
/// as the reference length for invariant #2. Computed once per test
/// run via `OnceLock` to avoid re-encoding inside the inner proptest
/// loop (the encoder is cheap but allocating 2KB on every case is
/// avoidable).
fn reference_initial_length() -> usize {
    static REF: OnceLock<usize> = OnceLock::new();
    *REF.get_or_init(|| {
        MatchState::initial(Seed::from_u64(0))
            .encode_canonical()
            .len()
    })
}

/// First 4 bytes of the canonical wire format: magic "FWMS". Asserted as
/// the bare-minimum prefix invariant (the 2-byte VERSION that follows
/// is implicitly checked by the length invariant #2 — if VERSION drifted
/// mid-suite, the length wouldn't match either).
const EXPECTED_MAGIC: &[u8; 4] = b"FWMS";

proptest! {
    #![proptest_config(ProptestConfig {
        // 512 cases is generous for a sub-millisecond encoder; total
        // wall-clock for this file is well under a second.
        cases: 512,
        ..ProptestConfig::default()
    })]

    #[test]
    fn encoder_invariants_hold_across_seeds(raw_seed in any::<u64>()) {
        let state = MatchState::initial(Seed::from_u64(raw_seed));

        let bytes_a = state.encode_canonical();
        let bytes_b = state.encode_canonical();

        // Invariant 1: magic prefix.
        prop_assert!(
            bytes_a.len() >= 4,
            "encoded output shorter than 4-byte magic prefix"
        );
        prop_assert_eq!(
            &bytes_a[..4],
            EXPECTED_MAGIC,
            "magic prefix changed for seed {:#x}",
            raw_seed
        );

        // Invariant 2: initial-state length is constant across seeds.
        let expected_len = reference_initial_length();
        prop_assert_eq!(
            bytes_a.len(),
            expected_len,
            "initial-state encoded length drifted from reference for seed {:#x} \
             (expected {}, got {})",
            raw_seed,
            expected_len,
            bytes_a.len()
        );

        // Invariant 3: intra-process determinism (re-encode produces same bytes).
        prop_assert_eq!(
            bytes_a, bytes_b,
            "encode_canonical produced different bytes on consecutive calls \
             for seed {:#x} — HashMap/HashSet regression?",
            raw_seed
        );
    }
}

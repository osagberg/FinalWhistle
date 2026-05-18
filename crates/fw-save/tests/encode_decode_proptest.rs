//! T2-R7(e) — SaveV1 encode → decode → re-encode byte identity across
//! seed-space + ledger-shape variation.
//!
//! Post-T2 ultimate-review Track D-Survey-3 flagged the gap: the in-
//! crate AC4d test `v1_encode_decode_reencode_produces_identical_bytes`
//! pins ONE payload (career_seed = 0x12345678_9ABCDEF0, empty ledger,
//! content_pack_version = 1). The round-trip-byte-identical invariant
//! is a property of the codec, NOT of any single payload — so a single-
//! payload test cannot catch a regression that only surfaces on
//! certain seed values or non-empty ledger shapes.
//!
//! Two invariants this proptest defends:
//!
//! 1. **Round-trip byte identity.** `encode(decode(encode(env)))` ==
//!    `encode(env)` for every `(career_seed, content_pack_version,
//!    ledger)` triple the strategy generates. A future serde-codec
//!    change that introduces non-determinism (e.g. swapping bincode
//!    for a serializer that emits maps in hash order) would fail this
//!    on at least one case even if the existing AC4d single-payload
//!    test happened to hit a payload that survived.
//!
//! 2. **Decode-equality.** `decode(encode(env)) == env` for every
//!    case. This catches a missing-field bug where serde silently
//!    drops a new field's bytes during decode — the AC4d test pins a
//!    single payload shape that may not exercise every codec path
//!    (e.g. an empty `MemoryLedger::events` Vec is a special-case
//!    bincode varint).
//!
//! Strategy notes:
//!   - `career_seed` and `content_pack_version` are raw u64/u32 via
//!     `any::<u64>()` / `any::<u32>()`.
//!   - `MemoryLedger` is built from a length-0..=8 Vec of
//!     `MemoryEvent::Placeholder` (the only variant today). This
//!     exercises the empty/short Vec codec path AND the populated
//!     path; both are reachable in production saves.
//!   - 256 cases at <1ms per case keeps the test suite cheap.
//!
//! When T3-1 lands `MemoryEvent::Placeholder → MemoryEvent::<real
//! variants>`, the placeholder Vec strategy becomes the real
//! event-variant strategy mirror; the test shape stays.
//!
//! Cross-reference: post-T2 ultimate-review doc at
//! `docs/audits/post-t2-ultimate-review-2026-05-18.md` Track D-Survey-3.

use fw_core::{MatchId, PlayerId, Seed, Tick};
use fw_memory::{MemoryEvent, MemoryLedger};
use fw_save::{SaveEnvelope, SaveV1, decode, encode};
use proptest::prelude::*;

/// Build a `MemoryLedger` from a length-N Vec of `(match_id, actor,
/// tick_raw)` triples. Each triple becomes one `MemoryEvent::Placeholder`.
fn ledger_from(events: Vec<(u32, u32, i64)>) -> MemoryLedger {
    let mut l = MemoryLedger::new();
    for (m, a, t) in events {
        l.push(MemoryEvent::Placeholder {
            match_id: MatchId::new(m),
            actor: PlayerId::new(a),
            tick: Tick::from_raw(t),
        });
    }
    l
}

/// Strategy generating an arbitrary `SaveV1` payload across the full
/// (seed, content_pack_version, ledger-shape) surface.
fn arb_save_v1() -> impl Strategy<Value = SaveV1> {
    (
        any::<u64>(),
        any::<u32>(),
        // 0..=8 events keeps the case-by-case wall-clock low while
        // covering both the empty-Vec and populated-Vec codec paths.
        prop::collection::vec((any::<u32>(), any::<u32>(), any::<i64>()), 0..=8),
    )
        .prop_map(|(seed_raw, pack_ver, events)| SaveV1 {
            career_seed: Seed::from_u64(seed_raw),
            content_pack_version: pack_ver,
            ledger: ledger_from(events),
        })
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        ..ProptestConfig::default()
    })]

    #[test]
    fn v1_round_trip_byte_identical_across_payloads(v1 in arb_save_v1()) {
        let env = SaveEnvelope::V1(v1.clone());

        // First encode.
        let bytes_1 = encode(&env).expect("encode #1 must succeed");

        // Decode → equality.
        let restored = decode(&bytes_1).expect("decode must succeed on freshly-encoded bytes");
        prop_assert_eq!(
            restored.clone(),
            env.clone(),
            "decode(encode(env)) must equal env (codec is not info-preserving?)"
        );

        // Re-encode → byte identity.
        let bytes_2 = encode(&restored).expect("encode #2 must succeed");
        prop_assert_eq!(
            bytes_1,
            bytes_2,
            "encode(decode(encode(env))) must produce byte-identical output \
             (codec non-determinism detected)"
        );
    }
}

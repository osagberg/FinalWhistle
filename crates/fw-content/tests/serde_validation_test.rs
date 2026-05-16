//! T1-12 acceptance criteria: post-parse serde validation for ID newtypes.
//!
//! Chunks 5 + 6: manual `Deserialize` impls for `RoleId`, `SignatureId`, and
//! `SignatureCandidate` now call `try_new` post-parse so malformed values in
//! RON fixtures are rejected at load time rather than silently stored.
//!
//! Each test verifies:
//!   - A well-formed value deserializes successfully (round-trip).
//!   - A malformed value that would previously have deserialized now returns
//!     a `serde` error whose message contains the validation failure.
//!
//! The tests use `ron::de::from_str` directly (the same path `ContentStore::
//! load_sources` takes) so the validation fires in the real load path.

use fw_content::{RoleId, SignatureCandidate, SignatureId};

// ---------------------------------------------------------------------------
// RoleId (Chunk 5)
// ---------------------------------------------------------------------------

#[test]
fn role_id_valid_string_deserializes() {
    let id: RoleId = ron::de::from_str(r#""GK""#).expect("valid RoleId must deserialize");
    assert_eq!(id.as_str(), "GK");
}

#[test]
fn role_id_empty_string_is_rejected_at_parse() {
    // Pre-T1-12: derived Deserialize + serde(transparent) accepted "" silently.
    // Post-T1-12: manual Deserialize calls try_new → RoleIdError::Empty → serde error.
    let result = ron::de::from_str::<RoleId>(r#""""#);
    assert!(
        result.is_err(),
        "empty RoleId string must be rejected at deserialize time"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("empty") || msg.contains("Empty"),
        "error message must mention emptiness; got: {msg}"
    );
}

#[test]
fn role_id_whitespace_only_is_rejected_at_parse() {
    let result = ron::de::from_str::<RoleId>(r#""  ""#);
    assert!(
        result.is_err(),
        "whitespace-only RoleId must be rejected at deserialize time"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("whitespace") || msg.contains("Whitespace"),
        "error message must mention whitespace; got: {msg}"
    );
}

#[test]
fn role_id_leading_whitespace_is_rejected_at_parse() {
    let result = ron::de::from_str::<RoleId>(r#"" GK""#);
    assert!(
        result.is_err(),
        "RoleId with leading space must be rejected at deserialize time"
    );
}

// ---------------------------------------------------------------------------
// SignatureId (Chunk 6)
// ---------------------------------------------------------------------------

#[test]
fn signature_id_valid_deserializes() {
    // SignatureId is serialized as `SignatureId("fwh.core:signature.foo")` in
    // RON — the newtype wrapper form (not transparent). The Deserialize impl
    // must handle the newtype envelope.
    let id: SignatureId = ron::de::from_str(r#"SignatureId("fwh.core:signature.no-op-stub")"#)
        .expect("valid SignatureId must deserialize");
    assert_eq!(id.as_str(), "fwh.core:signature.no-op-stub");
}

#[test]
fn signature_id_malformed_is_rejected_at_parse() {
    // "not-a-valid-id" lacks the required `<pack-id>:signature.<slug>` form.
    let result = ron::de::from_str::<SignatureId>(r#"SignatureId("not-a-valid-id")"#);
    assert!(
        result.is_err(),
        "malformed SignatureId must be rejected at deserialize time"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("malformed") || msg.contains("Malformed"),
        "error message must mention malformed; got: {msg}"
    );
}

#[test]
fn signature_id_missing_signature_segment_is_rejected() {
    // `fwh.core:role.foo` — valid pack-id but wrong entity type prefix.
    let result = ron::de::from_str::<SignatureId>(r#"SignatureId("fwh.core:role.foo")"#);
    assert!(
        result.is_err(),
        "SignatureId with wrong entity type must be rejected"
    );
}

#[test]
fn signature_id_single_segment_pack_id_is_rejected() {
    // `fwh:signature.foo` — single-segment pack-id (requires at least 2).
    let result = ron::de::from_str::<SignatureId>(r#"SignatureId("fwh:signature.foo")"#);
    assert!(
        result.is_err(),
        "SignatureId with single-segment pack-id must be rejected"
    );
}

// ---------------------------------------------------------------------------
// SignatureCandidate (Chunk 6)
// ---------------------------------------------------------------------------

#[test]
fn signature_candidate_valid_deserializes() {
    // Q32::ZERO = (bits: 0); Q32::ONE = (bits: 4294967296).
    // affinity: (bits: 2147483648) = 0.5 — well within [0, 1].
    let candidate: SignatureCandidate = ron::de::from_str(
        r#"(
            signature_id: SignatureId("fwh.core:signature.no-op-stub"),
            affinity: (bits: 2147483648),
        )"#,
    )
    .expect("valid SignatureCandidate must deserialize");
    assert_eq!(
        candidate.signature_id.as_str(),
        "fwh.core:signature.no-op-stub"
    );
}

#[test]
fn signature_candidate_malformed_signature_id_is_rejected() {
    // The SignatureCandidate.signature_id field must also be validated —
    // even when the outer struct deserialization otherwise succeeds.
    let result = ron::de::from_str::<SignatureCandidate>(
        r#"(
            signature_id: SignatureId("not-valid"),
            affinity: (bits: 2147483648),
        )"#,
    );
    assert!(
        result.is_err(),
        "SignatureCandidate with invalid signature_id must be rejected"
    );
}

#[test]
fn signature_candidate_affinity_above_one_is_rejected() {
    // Q32 bits: 4294967296 = 1.0 (exactly Q32::ONE).
    // Q32 bits: 4294967297 = slightly above 1.0 → try_new must reject it.
    let result = ron::de::from_str::<SignatureCandidate>(
        r#"(
            signature_id: SignatureId("fwh.core:signature.no-op-stub"),
            affinity: (bits: 4294967297),
        )"#,
    );
    assert!(
        result.is_err(),
        "SignatureCandidate with affinity > 1.0 must be rejected"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("out of range")
            || msg.contains("AffinityOutOfRange")
            || msg.contains("affinity"),
        "error message must mention affinity range; got: {msg}"
    );
}

#[test]
fn signature_candidate_affinity_negative_is_rejected() {
    // Q32 negative bits: -1 as i64 = a very small negative Q32.
    // (bits: -1) in RON is i64::MAX - ... depends on RON's i64 handling.
    // Use (bits: -4294967296) = -1.0 — clearly negative.
    let result = ron::de::from_str::<SignatureCandidate>(
        r#"(
            signature_id: SignatureId("fwh.core:signature.no-op-stub"),
            affinity: (bits: -4294967296),
        )"#,
    );
    assert!(
        result.is_err(),
        "SignatureCandidate with negative affinity must be rejected"
    );
}

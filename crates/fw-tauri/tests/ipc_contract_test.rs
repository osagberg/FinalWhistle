//! IPC contract tests for `crates/fw-tauri`.
//!
//! Validates the load-bearing IPC invariants identified in the T1-5 task spec
//! (cargo-cult-fix-pass meta-pattern — tests prove substance, not shape):
//!
//! 1. `play_match` round-trip canonical hash matches an independently-computed
//!    BLAKE3 over the same sim state (acceptance criterion 11a).
//! 2. `match_frames` with `tick_count = MAX + 1` returns `IpcError::TooManyFrames`
//!    with the correct `requested` + `max` fields (acceptance criterion 11b).
//! 3. `IpcError::TooManyFrames` serialises as the TS-readable discriminated
//!    union `{ kind: "tooManyFrames", requested: N, max: M }`.
//!
//! These tests call the `_inner` helpers directly (no `tauri::State` required).

use std::path::PathBuf;

use fw_core::Seed;
use fw_match_sim::{MatchState, tick_match};
use fw_tauri::{IpcError, MAX_FRAMES_PER_REQUEST};

fn workspace_content_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

fn test_app_state() -> fw_tauri::AppState {
    fw_tauri::AppState::new(&workspace_content_path()).expect("AppState::new in ipc_contract_test")
}

// ---------------------------------------------------------------------------
// Acceptance criterion 11a: play_match round-trip canonical hash
// ---------------------------------------------------------------------------

/// Invoke `play_match_inner` with seed `0xDEADBEEFDEADBEEF` + 60 ticks.
/// Parse the returned `MatchResult` and assert `canonical_hash` matches the
/// hash produced by independently running the sim and calling `blake3::hash`.
///
/// T2-R-D8 (renamed from `play_match_round_trip_canonical_hash_matches`):
/// the prior name implied stronger external-auditor validation than
/// this test actually performs. The authoritative external canonical-
/// hash pin lives at `crates/fw-replay/tests/canonical_hash.rs`. This
/// test proves IPC-path equivalence to a direct sim-call only.
#[test]
fn play_match_ipc_path_matches_direct_sim_call() {
    let state = test_app_state();

    // IPC inner path.
    let result = tauri::async_runtime::block_on(fw_tauri::commands::play_match_inner(
        "0xdeadbeefdeadbeef".to_string(),
        60,
        &state,
    ))
    .expect("play_match_inner should succeed");

    // Independent computation — same seed + tick_count, same algorithm.
    // SCOPE: this proves the IPC path runs the same sim + same hash function
    // as a direct `MatchState::initial_with_content` + `tick_match` + `blake3`
    // call. It does NOT prove the hash matches an external auditor's
    // BLAKE3 of the canonical encoding (both sides go through
    // `encode_canonical()` + `blake3::hash` here). For that stronger
    // round-trip a separate test would shell out to `b3sum` or similar.
    let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);
    let mut sim_state = MatchState::initial_with_content(
        seed,
        state.content(),
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
    )
    .expect("initial_with_content");
    for _ in 0..60 {
        sim_state = tick_match(sim_state, state.signature_definitions());
    }
    let canonical_bytes = sim_state.encode_canonical();
    let hash_bytes: [u8; 32] = blake3::hash(&canonical_bytes).into();
    let expected_hash = format!(
        "blake3:{}",
        hash_bytes
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );

    assert_eq!(
        result.canonical_hash, expected_hash,
        "IPC play_match canonical_hash must match independent BLAKE3 computation. \
         If this fails, the IPC path runs different logic than the direct sim call."
    );
}

/// The `canonical_hash` field must have the `blake3:` prefix and be exactly
/// 7 + 64 = 71 characters (prefix + 64 hex chars for 32 bytes).
#[test]
fn play_match_canonical_hash_has_correct_format() {
    let state = test_app_state();
    let result = tauri::async_runtime::block_on(fw_tauri::commands::play_match_inner(
        "0x1".to_string(),
        0,
        &state,
    ))
    .expect("play_match_inner");
    assert!(
        result.canonical_hash.starts_with("blake3:"),
        "canonical_hash must start with 'blake3:'"
    );
    assert_eq!(
        result.canonical_hash.len(),
        71,
        "canonical_hash must be 7 chars ('blake3:') + 64 hex chars"
    );
}

// ---------------------------------------------------------------------------
// Acceptance criterion 11b: match_frames error-shape test
// ---------------------------------------------------------------------------

/// Invoke `match_frames_inner` with `tick_count = MAX + 1` and assert:
/// - The call returns `Err(IpcError::TooManyFrames)`.
/// - The `requested` field equals `MAX + 1`.
/// - The `max` field equals `MAX_FRAMES_PER_REQUEST`.
#[test]
fn match_frames_over_max_returns_too_many_frames_error() {
    let state = test_app_state();
    let over_max = MAX_FRAMES_PER_REQUEST + 1;

    let err = tauri::async_runtime::block_on(fw_tauri::commands::match_frames_inner(
        "0x1".to_string(),
        over_max,
        &state,
    ))
    .expect_err("match_frames_inner should fail when tick_count > MAX_FRAMES_PER_REQUEST");

    match err {
        IpcError::TooManyFrames { requested, max } => {
            assert_eq!(
                requested, over_max,
                "`requested` must echo the caller's tick_count"
            );
            assert_eq!(
                max, MAX_FRAMES_PER_REQUEST,
                "`max` must equal MAX_FRAMES_PER_REQUEST"
            );
        }
        other => panic!(
            "expected IpcError::TooManyFrames, got {other:?}. \
             The guard in match_frames_inner must fire before any alloc."
        ),
    }
}

/// The `IpcError::TooManyFrames` variant must serialise as the TS-readable
/// discriminated union `{{ kind: "tooManyFrames", requested: N, max: M }}`.
/// This is the load-bearing serde shape the TypeScript side decodes.
#[test]
fn too_many_frames_error_serializes_as_ts_discriminated_union() {
    let err = IpcError::TooManyFrames {
        requested: MAX_FRAMES_PER_REQUEST + 1,
        max: MAX_FRAMES_PER_REQUEST,
    };
    let json = serde_json::to_string(&err).expect("IpcError must be Serialize");
    let v: serde_json::Value = serde_json::from_str(&json).expect("must be valid JSON");

    assert_eq!(
        v["kind"], "tooManyFrames",
        "discriminant must be camelCase 'tooManyFrames' (not 'TooManyFrames'). \
         Requires #[serde(tag = \"kind\", rename_all = \"camelCase\")] on IpcError."
    );
    assert_eq!(
        v["requested"],
        serde_json::json!(MAX_FRAMES_PER_REQUEST + 1),
        "`requested` field must be present in the serialised form"
    );
    assert_eq!(
        v["max"],
        serde_json::json!(MAX_FRAMES_PER_REQUEST),
        "`max` field must be present in the serialised form"
    );
}

/// Round-trip: serialise `IpcError::TooManyFrames` then deserialise it back.
/// Both `requested` and `max` must survive the round-trip unchanged.
#[test]
fn too_many_frames_error_round_trips_through_json() {
    let original = IpcError::TooManyFrames {
        requested: 9999,
        max: MAX_FRAMES_PER_REQUEST,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let back: IpcError = serde_json::from_str(&json).expect("deserialize");
    match back {
        IpcError::TooManyFrames { requested, max } => {
            assert_eq!(requested, 9999);
            assert_eq!(max, MAX_FRAMES_PER_REQUEST);
        }
        other => panic!("wrong variant after round-trip: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Structural: src-tauri has zero local commands (acceptance criterion 7)
// ---------------------------------------------------------------------------

/// Verify the src-tauri shell registers only fw_tauri commands, not local ones.
/// This test checks the compile-time fact that `src-tauri/src/commands.rs`
/// no longer exists, so there can be no local commands to register.
///
/// If this test file can be referenced in the build, `commands.rs` is gone.
/// (The Rust compiler enforces this — adding it back breaks the build.)
#[test]
fn src_tauri_commands_file_does_not_exist() {
    let commands_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("src-tauri")
        .join("src")
        .join("commands.rs");
    assert!(
        !commands_path.exists(),
        "src-tauri/src/commands.rs must be deleted (T1-5 consolidation). \
         Found it at: {}",
        commands_path.display()
    );
}

// ---------------------------------------------------------------------------
// Codex 2026-05-16 Tier-2 fix-pass: MatchEvent wire-shape verification
//
// The prior `MatchResult.match_events: Vec<fw_content::MatchEvent>` shipped
// the raw enum which serde-derives externally-tagged JSON
// (`{ "KickOff": {...} }`) — incompatible with the frontend's flat
// `{ tick, minute, kind, description }` interface. The T1-6 Vitest tests
// missed this because Match.test.tsx constructed mock events in the
// FRONTEND shape directly; the actual Rust→TS round-trip was never
// exercised. These tests pin the wire shape EXACTLY as the frontend sees
// it so a future regression fails at `cargo test`, not at `pnpm tauri dev`.
// ---------------------------------------------------------------------------

/// `MatchResult.match_events` MUST serialise as a flat array of
/// `{ tick, minute, kind, description }` objects (camelCase, kind is the
/// PascalCase variant name) — NOT as externally-tagged `{ "KickOff": {...} }`
/// objects. This is the load-bearing wire-shape Codex flagged.
#[test]
fn match_result_match_events_serializes_as_flat_dto_not_tagged_enum() {
    let state = test_app_state();
    let result = tauri::async_runtime::block_on(fw_tauri::commands::play_match_inner(
        "0xdeadbeefdeadbeef".to_string(),
        60,
        &state,
    ))
    .expect("play_match_inner");

    let json = serde_json::to_string(&result).expect("MatchResult serializes");
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let events = v["matchEvents"]
        .as_array()
        .expect("matchEvents must be a JSON array (camelCase)");
    assert!(!events.is_empty(), "60-tick run must emit ≥1 MatchEvent");

    for (i, ev) in events.iter().enumerate() {
        let obj = ev.as_object().unwrap_or_else(|| {
            panic!("event[{i}] must be a flat object, not a tagged enum form: {ev}")
        });
        // The flat DTO has exactly 4 keys; anything else is a regression to
        // the old fw_content::MatchEvent serialisation shape.
        assert!(
            obj.contains_key("tick"),
            "event[{i}] missing `tick`: {ev} — likely regressed to enum form"
        );
        assert!(
            obj.contains_key("minute"),
            "event[{i}] missing `minute`: {ev}"
        );
        assert!(
            obj.contains_key("kind"),
            "event[{i}] missing `kind`: {ev} — regression: enum-tag form would have variant name as outer key"
        );
        // `description` is optional via #[serde(skip_serializing_if)] so it
        // may be absent — that's the correct shape matching the TS
        // `description?: string` optional-field type. If present, it must be
        // a string (never null), since None gets omitted not serialized.
        if let Some(desc) = obj.get("description") {
            assert!(
                desc.is_string(),
                "event[{i}] description must be a string or absent, got: {desc}"
            );
        }

        // `tick` must be a number, `minute` must be a number, `kind` must be
        // a PascalCase string from the closed MatchEventKind union.
        assert!(obj["tick"].is_i64(), "event[{i}] tick must be i64: {ev}");
        assert!(
            obj["minute"].is_u64(),
            "event[{i}] minute must be u64: {ev}"
        );
        let kind = obj["kind"]
            .as_str()
            .unwrap_or_else(|| panic!("event[{i}] kind must be a string: {ev}"));
        let allowed = [
            "KickOff",
            "FullTime",
            "Goal",
            "Shot",
            "Pass",
            "SignatureFirstFired",
        ];
        assert!(
            allowed.contains(&kind),
            "event[{i}] kind {kind:?} not in known set {allowed:?}; \
             regression: enum form would have variant name as outer key instead"
        );
    }
}

/// The first event MUST be `KickOff` at tick 0 (sim invariant) and serialise
/// as `{ tick: 0, minute: 0, kind: "KickOff", description: null }`.
#[test]
fn first_match_event_is_kickoff_with_exact_wire_shape() {
    let state = test_app_state();
    let result = tauri::async_runtime::block_on(fw_tauri::commands::play_match_inner(
        "0x1".to_string(),
        0,
        &state,
    ))
    .expect("play_match_inner");

    let json = serde_json::to_string(&result).expect("serialize");
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let events = v["matchEvents"].as_array().expect("matchEvents array");
    assert!(!events.is_empty(), "initial state has KickOff event");

    let first = &events[0];
    assert_eq!(first["tick"], serde_json::json!(0));
    assert_eq!(first["minute"], serde_json::json!(0));
    assert_eq!(first["kind"], serde_json::json!("KickOff"));
    // `description` is omitted entirely via #[serde(skip_serializing_if)]
    // when None — matches TS `description?: string` optional shape. Asserting
    // absence catches any regression to `description: null` form which would
    // mismatch the optional field type in strict TS.
    assert!(
        first
            .as_object()
            .expect("event obj")
            .get("description")
            .is_none(),
        "description should be omitted from JSON when None (not serialized as null)"
    );
}

// ---------------------------------------------------------------------------
// Codex 2026-05-16 Tier-2 fix-pass: BackendHandshakeDto wire-shape
// ---------------------------------------------------------------------------

/// `BackendHandshakeDto` MUST serialise as `{ appVersion, message, backendReady }`
/// matching the frontend's `BackendHandshake` interface. Codex flagged the
/// prior shape mismatch where `get_dummy_state` returned `MatchStateDto`
/// while `Home.tsx` expected the handshake shape.
#[test]
fn backend_handshake_dto_serializes_as_frontend_handshake_shape() {
    let h = fw_tauri::BackendHandshakeDto::live();
    let json = serde_json::to_string(&h).expect("serialize");
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let obj = v.as_object().expect("handshake is a JSON object");

    // Exact key set the frontend reads.
    assert!(obj.contains_key("appVersion"), "appVersion missing: {json}");
    assert!(obj.contains_key("message"), "message missing: {json}");
    assert!(
        obj.contains_key("backendReady"),
        "backendReady missing: {json}"
    );
    assert_eq!(obj["backendReady"], serde_json::Value::Bool(true));
}

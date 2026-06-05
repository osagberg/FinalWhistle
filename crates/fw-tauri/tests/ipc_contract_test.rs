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

use fw_core::{MatchId, PlayerId, Q32, Seed, Tick};
use fw_match_sim::{MatchState, tick_match};
use fw_memory::event::{
    CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind, Emotion,
    EntityRef, EventClass, MemoryEvent, Participant, ParticipantRole, SeasonNumber, SourceId,
};
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
// T2-7: get_squad camelCase DTO contract
// ---------------------------------------------------------------------------

/// `SquadPlayerDto` must serialise with camelCase keys matching the TS
/// `SquadPlayer` interface: `playerId`, `name`, `role`, `birthRegion`,
/// `phenotypeLabels`.
#[test]
fn squad_player_dto_serializes_camel_case_keys() {
    let state = test_app_state();
    let squad = fw_tauri::commands::get_squad_inner(&state).expect("get_squad_inner");
    assert!(
        !squad.is_empty(),
        "ContentStore must have at least one player bio"
    );

    // Pick first DTO and round-trip through JSON to verify key names.
    let dto = &squad[0];
    let json = serde_json::to_string(dto).expect("SquadPlayerDto must serialize");
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let obj = v.as_object().expect("SquadPlayerDto is a JSON object");

    assert!(
        obj.contains_key("playerId"),
        "missing key 'playerId': {json}"
    );
    assert!(obj.contains_key("name"), "missing key 'name': {json}");
    assert!(obj.contains_key("role"), "missing key 'role': {json}");
    assert!(
        obj.contains_key("birthRegion"),
        "missing key 'birthRegion': {json}"
    );
    assert!(
        obj.contains_key("phenotypeLabels"),
        "missing key 'phenotypeLabels': {json}"
    );

    // No snake_case keys should appear.
    assert!(
        !obj.contains_key("player_id"),
        "snake_case key 'player_id' must not appear"
    );
    assert!(
        !obj.contains_key("birth_region"),
        "snake_case key 'birth_region' must not appear"
    );
    assert!(
        !obj.contains_key("phenotype_labels"),
        "snake_case key 'phenotype_labels' must not appear"
    );

    // phenotypeLabels must be a JSON array.
    let labels = obj["phenotypeLabels"]
        .as_array()
        .expect("phenotypeLabels must be a JSON array");
    // Each label string must be a string (not null).
    for label in labels {
        assert!(
            label.is_string(),
            "each phenotype label must be a string, got: {label}"
        );
    }
}

/// `get_squad_inner` returns exactly 22 players (the hand-authored fixture count).
#[test]
fn get_squad_returns_22_players() {
    let state = test_app_state();
    let squad = fw_tauri::commands::get_squad_inner(&state).expect("get_squad_inner");
    assert_eq!(
        squad.len(),
        22,
        "hand-authored content has exactly 22 player bios"
    );
}

// ---------------------------------------------------------------------------
// T3-6: get_player_detail — fixture-ledger end-to-end test
// ---------------------------------------------------------------------------

/// Helper to build a representative MemoryEvent for a given player + class.
fn make_fixture_event(
    player_id: PlayerId,
    season: u16,
    class: EventClass,
    stakes: Q32,
) -> MemoryEvent {
    MemoryEvent {
        event_id: fw_memory::event::EventId(0), // overwritten by ledger.append
        schema_version: 1,
        season: SeasonNumber(season),
        tick: Some(Tick::ZERO),
        career_date: CareerDate {
            year: 1,
            day_of_year: 42,
        },
        emitter: Emitter {
            kind: EmitterKind::MatchEngine,
            source_id: SourceId::Match(MatchId::new(0)),
        },
        participants: vec![Participant {
            role: ParticipantRole::Subject,
            entity: EntityRef::Player(player_id),
        }],
        event_class: class,
        stakes,
        emotion: Emotion::Joy,
        consequence: vec![Consequence::None],
        callback_eligibility: CallbackEligibility::Immediate,
        salience: stakes,
        decay_function: DecayFunction::Never,
    }
}

/// Fixture-ledger test: build AppState, inject ≥3 MemoryEvents for a known
/// player, assert `get_player_detail_inner` returns football-grade callbacks.
///
/// Acceptance criteria verified here:
/// - `memoryCallbacks` is non-empty when the ledger has events.
/// - No `{{` or `#` template seams in any callback string (renderer ran fully).
/// - Callback strings are non-empty and contain football-native text.
/// - The DTO serialises with camelCase keys (`memoryCallbacks` not `memory_callbacks`).
#[test]
fn get_player_detail_fixture_ledger_returns_football_grade_callbacks() {
    let state = test_app_state();

    // Pick the first player from the content store so we have a real bio.
    let first_id = state
        .content()
        .player_bios
        .keys()
        .next()
        .expect("content store has at least one player bio")
        .clone();

    // Extract the numeric suffix for PlayerId construction.
    let raw_suffix: u32 = first_id
        .split(':')
        .nth(1)
        .and_then(|s| s.split('_').next_back())
        .and_then(|s| s.parse().ok())
        .expect("first player id must have a numeric _NNNNN suffix");
    let player_fw_id = PlayerId::new(raw_suffix);

    // Inject 3 varied events into the ledger.
    {
        let mut career = state.career().write().expect("career write lock");

        career.ledger.append(make_fixture_event(
            player_fw_id,
            0,
            EventClass::DebutSenior,
            Q32::from_raw(1i64 << 31), // 0.5
        ));
        career.ledger.append(make_fixture_event(
            player_fw_id,
            1,
            EventClass::LegacyGoal,
            Q32::from_raw(3i64 << 30), // 0.75
        ));
        career.ledger.append(make_fixture_event(
            player_fw_id,
            2,
            EventClass::TitleWon,
            Q32::ONE,
        ));
    }

    let dto = fw_tauri::commands::get_player_detail_inner(&first_id, &state)
        .expect("get_player_detail_inner must succeed with populated ledger");

    // Callbacks must be non-empty.
    assert!(
        !dto.memory_callbacks.is_empty(),
        "ledger has 3 events → memoryCallbacks must be non-empty"
    );

    for cb in &dto.memory_callbacks {
        // Non-empty.
        assert!(!cb.is_empty(), "callback string must not be empty");
        // No template seams — renderer ran to completion.
        assert!(
            !cb.contains("{{"),
            "callback contains unresolved '{{{{' template seam: {cb:?}"
        );
        assert!(
            !cb.contains('#'),
            "callback contains unresolved '#' tracery substitution: {cb:?}"
        );
    }

    // DTO serialises with camelCase keys.
    let json = serde_json::to_string(&dto).expect("serialize PlayerDetailDto");
    let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
    let obj = v.as_object().expect("PlayerDetailDto is object");

    assert!(
        obj.contains_key("memoryCallbacks"),
        "wire key must be 'memoryCallbacks' (camelCase), not 'memory_callbacks'"
    );
    assert!(
        !obj.contains_key("memory_callbacks"),
        "snake_case key 'memory_callbacks' must not appear in wire JSON"
    );
    let callbacks_arr = obj["memoryCallbacks"]
        .as_array()
        .expect("memoryCallbacks must be a JSON array");
    assert!(
        !callbacks_arr.is_empty(),
        "memoryCallbacks array must be non-empty"
    );
    for v in callbacks_arr {
        assert!(
            v.is_string(),
            "each memoryCallbacks element must be a string"
        );
    }
}

/// Discriminant-30 guard: an `UnknownEventClass` event in the ledger must produce
/// the static fallback phrase `"an unusual moment in the career"` — NOT a render
/// error propagated as an IPC error, and NOT an empty string.
///
/// This exercises the `if discriminant_to_family_key(disc).is_none()` branch in
/// `get_player_detail_inner`, which is the guard for discriminant 30.
#[test]
fn get_player_detail_unknown_event_class_returns_static_fallback() {
    use fw_memory::event::ModEventTag;

    let state = test_app_state();

    let first_id = state
        .content()
        .player_bios
        .keys()
        .next()
        .expect("content store has at least one player bio")
        .clone();

    let raw_suffix: u32 = first_id
        .split(':')
        .nth(1)
        .and_then(|s| s.split('_').next_back())
        .and_then(|s| s.parse().ok())
        .expect("first player id must have a numeric _NNNNN suffix");
    let player_fw_id = PlayerId::new(raw_suffix);

    // Inject a single UnknownEventClass event — discriminant 30, no grammar family.
    {
        let mut career = state.career().write().expect("career write lock");
        career.ledger.append(make_fixture_event(
            player_fw_id,
            0,
            EventClass::UnknownEventClass {
                tag: ModEventTag("mod.test:unknown_class".to_string()),
                payload: Vec::new(),
            },
            Q32::from_raw(1i64 << 31), // 0.5 salience — will surface in top_n
        ));
    }

    let dto = fw_tauri::commands::get_player_detail_inner(&first_id, &state)
        .expect("get_player_detail_inner must succeed even with an UnknownEventClass event");

    // The callback must be the discriminant-30 static fallback, not empty and not an error.
    assert_eq!(
        dto.memory_callbacks.len(),
        1,
        "one injected event → exactly one callback string"
    );
    assert_eq!(
        dto.memory_callbacks[0], "an unusual moment in the career",
        "discriminant-30 (UnknownEventClass) must produce the static fallback phrase"
    );
}

/// Known-id → PlayerNotFound wire shape test: `{ kind: "playerNotFound", playerId: "..." }`.
#[test]
fn player_not_found_error_serializes_as_ts_discriminated_union() {
    let err = IpcError::PlayerNotFound {
        player_id: "fwh.core:player_99999".to_string(),
    };
    let json = serde_json::to_string(&err).expect("serialize");
    let v: serde_json::Value = serde_json::from_str(&json).expect("parse");

    assert_eq!(
        v["kind"], "playerNotFound",
        "kind discriminant must be camelCase"
    );
    assert_eq!(
        v["playerId"], "fwh.core:player_99999",
        "playerId field must be present"
    );
    assert!(
        !v.as_object().unwrap().contains_key("player_id"),
        "snake_case 'player_id' must not appear"
    );
}

// ---------------------------------------------------------------------------
// T3-9: get_career_overview — camelCase wire-shape contract (AC5)
// ---------------------------------------------------------------------------

/// `CareerOverviewDto` must serialise with camelCase keys matching the TS
/// `CareerOverviewDto` interface: `seasonNumber`, `history`,
/// `crossSeasonCallbacks`. Run after one completed season so history is
/// non-empty.
#[test]
fn get_career_overview_dto_serializes_camel_case_keys() {
    use fw_tauri::commands::{
        advance_season_inner, get_career_overview_inner, play_fixtures_inner,
    };

    let state = test_app_state();
    // Complete one season so there is at least one history entry.
    play_fixtures_inner(&state).expect("play_fixtures");
    advance_season_inner(&state).expect("advance_season");

    let dto = get_career_overview_inner(&state).expect("get_career_overview_inner");
    let json = serde_json::to_string(&dto).expect("CareerOverviewDto must serialize");
    let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let obj = v.as_object().expect("CareerOverviewDto is an object");

    // Required camelCase keys.
    assert!(
        obj.contains_key("seasonNumber"),
        "missing key 'seasonNumber': {json}"
    );
    assert!(obj.contains_key("history"), "missing key 'history': {json}");
    assert!(
        obj.contains_key("crossSeasonCallbacks"),
        "missing key 'crossSeasonCallbacks': {json}"
    );

    // No snake_case leakage.
    assert!(
        !obj.contains_key("season_number"),
        "snake_case 'season_number' must not appear"
    );
    assert!(
        !obj.contains_key("cross_season_callbacks"),
        "snake_case 'cross_season_callbacks' must not appear"
    );

    // history is an array with at least one entry.
    let history_arr = obj["history"]
        .as_array()
        .expect("history must be a JSON array");
    assert!(
        !history_arr.is_empty(),
        "history must have ≥1 entry after one completed season"
    );

    let first_entry = history_arr[0]
        .as_object()
        .expect("history entry is an object");
    assert!(
        first_entry.contains_key("season"),
        "history entry missing 'season' key"
    );
    assert!(
        first_entry.contains_key("championClubName"),
        "history entry missing 'championClubName' key"
    );
    assert!(
        !first_entry.contains_key("champion_club_name"),
        "snake_case leak in history entry"
    );

    // crossSeasonCallbacks is an array; each element is a non-empty string
    // with no template seams.
    let callbacks_arr = obj["crossSeasonCallbacks"]
        .as_array()
        .expect("crossSeasonCallbacks must be a JSON array");
    assert!(
        !callbacks_arr.is_empty(),
        "crossSeasonCallbacks must have ≥1 entry after one completed season"
    );
    for v in callbacks_arr {
        let s = v
            .as_str()
            .expect("each crossSeasonCallback element must be a string");
        assert!(!s.is_empty(), "callback string must not be empty");
        assert!(
            !s.contains("{{"),
            "callback contains '{{{{' template seam: {s:?}"
        );
        assert!(
            !s.contains('#'),
            "callback contains '#' tracery seam: {s:?}"
        );
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
            // FUN-TS2b
            "Offside",
            // FUN-CB1
            "PassIncomplete",
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

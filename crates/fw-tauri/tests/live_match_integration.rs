//! Integration tests for the T4-5a live-match IPC layer.
//!
//! All tests call the `_inner` helpers directly (`tauri::State` is not
//! constructable in tests). The load-bearing test is
//! `live_match_step_equivalent_to_play_match`, which proves AC4:
//! 600 × `step_live_match(handle, 1)` must produce the same canonical hash
//! as `play_match(seed, 600)`.

use std::path::PathBuf;

use fw_tauri::live_match::types::{KNOWN_MATCH_COMMAND_KINDS, MatchCommand, PressLevel, TempoBias};
use fw_tauri::{
    IpcError, MAX_FRAMES_PER_REQUEST,
    commands::{
        apply_match_command_inner, finish_live_match_inner, get_match_snapshot_inner,
        play_match_inner, start_live_match_inner, step_live_match_inner,
    },
};

fn workspace_content_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

fn test_app_state() -> fw_tauri::AppState {
    fw_tauri::AppState::new(&workspace_content_path())
        .expect("AppState::new in live_match_integration test")
}

// ---------------------------------------------------------------------------
// AC4: determinism-equivalence — the load-bearing test
// ---------------------------------------------------------------------------

/// Running 600 × `step_live_match(handle, 1)` must produce the same canonical
/// BLAKE3 hash as `play_match(seed, 600)` for the same seed.
///
/// This is the AC4 acceptance criterion from the T4-5a task spec. If this test
/// fails, the live-match step path deviates from the authoritative `play_match`
/// path — a determinism bug.
#[test]
fn live_match_step_equivalent_to_play_match() {
    const TICK_COUNT: u32 = 600;
    const SEED_HEX: &str = "0xdeadbeefcafebabe";

    let state = test_app_state();

    // ---- Reference path: play_match(seed, 600) ----
    let play_result =
        tauri::async_runtime::block_on(play_match_inner(SEED_HEX.to_string(), TICK_COUNT, &state))
            .expect("play_match_inner must succeed");
    let hash_a = play_result.canonical_hash;

    // ---- Live path: start + 600 × step(1) ----
    let handle = start_live_match_inner(SEED_HEX.to_string(), &state)
        .expect("start_live_match_inner must succeed");

    for _ in 0..TICK_COUNT {
        step_live_match_inner(handle.clone(), 1, &state)
            .expect("step_live_match_inner must succeed each tick");
    }

    // Hash the live session's final MatchState via the same encode_canonical path.
    let hash_b = {
        let live = state.live_matches().read().expect("live_matches read lock");
        let session = live.get(&handle.id).expect("session still present");
        let bytes = session.state.encode_canonical();
        let hash_bytes: [u8; 32] = blake3::hash(&bytes).into();
        format!(
            "blake3:{}",
            hash_bytes
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        )
    };

    assert_eq!(
        hash_a, hash_b,
        "AC4 FAILED: 600 × step_live_match(1) must produce the same canonical hash \
         as play_match(600). If this fails, the live-match tick path deviates from \
         the authoritative play_match path."
    );
}

// ---------------------------------------------------------------------------
// Concurrent live matches — no cross-contamination
// ---------------------------------------------------------------------------

/// Start two live matches with different seeds, step them interleaved, and
/// assert their canonical hashes diverge per seed (no cross-contamination)
/// and their handles are distinct.
#[test]
fn concurrent_live_matches_no_cross_contamination() {
    let state = test_app_state();

    let handle_a =
        start_live_match_inner("0x1111111111111111".to_string(), &state).expect("start A");
    let handle_b =
        start_live_match_inner("0x2222222222222222".to_string(), &state).expect("start B");

    assert_ne!(handle_a.id, handle_b.id, "handles must be distinct");

    // Step them interleaved.
    for _ in 0..10 {
        step_live_match_inner(handle_a.clone(), 1, &state).expect("step A");
        step_live_match_inner(handle_b.clone(), 1, &state).expect("step B");
    }

    let (hash_a, hash_b) = {
        let live = state.live_matches().read().expect("read lock");
        let bytes_a = live
            .get(&handle_a.id)
            .expect("session A")
            .state
            .encode_canonical();
        let bytes_b = live
            .get(&handle_b.id)
            .expect("session B")
            .state
            .encode_canonical();
        let ha: [u8; 32] = blake3::hash(&bytes_a).into();
        let hb: [u8; 32] = blake3::hash(&bytes_b).into();
        (ha, hb)
    };

    assert_ne!(
        hash_a, hash_b,
        "different seeds must produce different canonical states after 10 ticks"
    );
}

// ---------------------------------------------------------------------------
// Handle lifecycle
// ---------------------------------------------------------------------------

/// start → finish → re-fetching the handle returns MatchInitFailed.
#[test]
fn handle_lifecycle_after_finish_returns_error() {
    let state = test_app_state();

    let handle = start_live_match_inner("0xabcdef0123456789".to_string(), &state).expect("start");

    // Snapshot is OK while live.
    get_match_snapshot_inner(handle.clone(), &state).expect("snapshot while live");

    // Finish removes the session.
    finish_live_match_inner(handle.clone(), &state).expect("finish");

    // Any subsequent call with the stale handle must return MatchInitFailed.
    let err =
        get_match_snapshot_inner(handle.clone(), &state).expect_err("must error after finish");
    match err {
        IpcError::MatchInitFailed { reason } => {
            assert!(
                reason.contains(&handle.id.to_string()),
                "error reason must mention the handle id: {reason}"
            );
        }
        other => panic!("expected MatchInitFailed, got {other:?}"),
    }

    let err2 =
        step_live_match_inner(handle.clone(), 1, &state).expect_err("step must error after finish");
    assert!(
        matches!(err2, IpcError::MatchInitFailed { .. }),
        "step after finish must return MatchInitFailed"
    );
}

// ---------------------------------------------------------------------------
// apply_match_command — all 9 variants return LiveMatchCommandUnimplemented
// ---------------------------------------------------------------------------

/// For each of the 9 `MatchCommand` variants, assert that `apply_match_command`
/// returns `Err(IpcError::LiveMatchCommandUnimplemented { command_kind: <expected> })`.
#[test]
fn apply_each_variant_returns_unimplemented() {
    let state = test_app_state();
    let handle = start_live_match_inner("0xfeedbabe00000001".to_string(), &state).expect("start");

    let samples: Vec<(MatchCommand, &str)> = vec![
        (
            MatchCommand::Substitute {
                player_in: fw_core::PlayerId::new(1),
                player_out: fw_core::PlayerId::new(2),
            },
            "substitute",
        ),
        (
            MatchCommand::ChangeFormation {
                formation: "fwh.core:formation.4-3-3".to_string(),
            },
            "changeFormation",
        ),
        (
            MatchCommand::ChangePressLevel {
                level: PressLevel::High,
            },
            "changePressLevel",
        ),
        (
            MatchCommand::ChangeTempoBias {
                bias: TempoBias::Fast,
            },
            "changeTempoBias",
        ),
        (
            MatchCommand::SetCornerTaker {
                player: fw_core::PlayerId::new(3),
            },
            "setCornerTaker",
        ),
        (
            MatchCommand::SetFreeKickTaker {
                player: fw_core::PlayerId::new(4),
            },
            "setFreeKickTaker",
        ),
        (
            MatchCommand::SetPenaltyTaker {
                player: fw_core::PlayerId::new(5),
            },
            "setPenaltyTaker",
        ),
        (
            MatchCommand::SetCaptain {
                player: fw_core::PlayerId::new(6),
            },
            "setCaptain",
        ),
        (
            MatchCommand::TeamTalk {
                message_id: "fwh.core:teamtalk_00001".to_string(),
            },
            "teamTalk",
        ),
    ];

    assert_eq!(samples.len(), 9, "must cover all 9 MatchCommand variants");
    assert_eq!(
        KNOWN_MATCH_COMMAND_KINDS.len(),
        9,
        "KNOWN_MATCH_COMMAND_KINDS must have 9 entries"
    );

    for (cmd, expected_kind) in samples {
        let err = apply_match_command_inner(handle.clone(), cmd, &state)
            .expect_err("apply_match_command must always return Err at T4-5a");
        match err {
            IpcError::LiveMatchCommandUnimplemented { command_kind } => {
                assert_eq!(
                    command_kind, expected_kind,
                    "command_kind mismatch for variant {expected_kind:?}"
                );
            }
            other => panic!(
                "expected LiveMatchCommandUnimplemented for {expected_kind:?}, got {other:?}"
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// step_live_match caps at MAX_FRAMES_PER_REQUEST
// ---------------------------------------------------------------------------

#[test]
fn step_caps_at_max_frames() {
    let state = test_app_state();
    let handle = start_live_match_inner("0x0000000000000001".to_string(), &state).expect("start");

    let err = step_live_match_inner(handle, MAX_FRAMES_PER_REQUEST + 1, &state)
        .expect_err("must reject over-max ticks");
    match err {
        IpcError::TooManyFrames { requested, max } => {
            assert_eq!(requested, MAX_FRAMES_PER_REQUEST + 1);
            assert_eq!(max, MAX_FRAMES_PER_REQUEST);
        }
        other => panic!("expected TooManyFrames, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// MatchSnapshot default fields at kick-off
// ---------------------------------------------------------------------------

/// A brand-new live match produces a snapshot with:
/// - `yellow_cards.is_empty()` (no card system at T1)
/// - `sent_off.is_empty()`
/// - `home_lineup.players.len() == 11`
/// - `away_lineup.players.len() == 11`
/// - `phase == MatchPhase::FirstHalf` (no FullTime emitted yet)
/// - `possession_pct.home_pct + away_pct == 100` (or both 50 at tick 0)
#[test]
fn match_snapshot_default_fields_at_kickoff() {
    use fw_tauri::live_match::types::MatchPhase;

    let state = test_app_state();
    let handle = start_live_match_inner("0x0badcafedeadbeef".to_string(), &state).expect("start");

    let snap = get_match_snapshot_inner(handle, &state).expect("snapshot");

    assert!(
        snap.yellow_cards.is_empty(),
        "no card system at T1 — yellow_cards must be empty"
    );
    assert!(
        snap.sent_off.is_empty(),
        "no card system at T1 — sent_off must be empty"
    );
    assert_eq!(
        snap.home_lineup.players.len(),
        11,
        "home_lineup must have exactly 11 slots"
    );
    assert_eq!(
        snap.away_lineup.players.len(),
        11,
        "away_lineup must have exactly 11 slots"
    );
    assert_eq!(
        snap.phase,
        MatchPhase::FirstHalf,
        "brand-new match is in FirstHalf"
    );

    let home_pct = snap.possession_pct.home_pct as u16;
    let away_pct = snap.possession_pct.away_pct as u16;
    // At tick 0 both should be 50/50 (no ticks elapsed yet; 0/0 → 50/50).
    assert_eq!(home_pct + away_pct, 100, "possession_pct must sum to 100");
    // At kick-off both teams share 50/50 (initial state has no possession ticks).
    assert_eq!(
        home_pct, 50,
        "at kick-off home_pct must be 50 (no ticks elapsed)"
    );
}

// ---------------------------------------------------------------------------
// MatchSnapshot after FullTime
// ---------------------------------------------------------------------------

/// Step a match to FullTime and assert phase transitions to `FullTime`.
#[test]
fn match_snapshot_phase_transitions_to_full_time() {
    use fw_tauri::live_match::types::MatchPhase;

    let state = test_app_state();
    let handle = start_live_match_inner("0xdeadbeef00000001".to_string(), &state).expect("start");

    // The T1 match sim ends at 60 ticks by default. Step 61 to ensure FullTime fires.
    let step = step_live_match_inner(handle.clone(), 61, &state).expect("step to FullTime");
    assert!(step.is_finished, "is_finished must be true after FullTime");

    let snap = get_match_snapshot_inner(handle, &state).expect("snapshot after FullTime");
    assert_eq!(
        snap.phase,
        MatchPhase::FullTime,
        "phase must be FullTime after FullTime event emitted"
    );
}

// ---------------------------------------------------------------------------
// StepResult new_events delta is correct
// ---------------------------------------------------------------------------

/// The first `step_live_match(handle, 1)` call must return only the events
/// emitted on that one tick — not the KickOff event already in the initial
/// state, and not duplicates.
#[test]
fn step_result_new_events_is_delta_only() {
    let state = test_app_state();
    let handle = start_live_match_inner("0x1234abcd5678ef01".to_string(), &state).expect("start");

    let step1 = step_live_match_inner(handle.clone(), 1, &state).expect("step 1");
    let step2 = step_live_match_inner(handle.clone(), 1, &state).expect("step 2");

    // The events in step1 + step2 should not overlap.
    let ticks_step1: Vec<i64> = step1.new_events.iter().map(|e| e.tick).collect();
    let ticks_step2: Vec<i64> = step2.new_events.iter().map(|e| e.tick).collect();

    // No tick from step2 should appear in step1.
    for t in &ticks_step2 {
        assert!(
            !ticks_step1.contains(t),
            "tick {t} appears in both step1 and step2 — delta logic is broken"
        );
    }
}

// ---------------------------------------------------------------------------
// IpcError::LiveMatchCommandUnimplemented serde wire shape
// ---------------------------------------------------------------------------

/// The `LiveMatchCommandUnimplemented` variant must serialise as:
/// `{ kind: "liveMatchCommandUnimplemented", commandKind: "substitute" }`
/// This is the wire shape the TS frontend expects.
#[test]
fn live_match_command_unimplemented_serde_wire_shape() {
    let err = IpcError::LiveMatchCommandUnimplemented {
        command_kind: "substitute".to_string(),
    };
    let json = serde_json::to_string(&err).expect("serialize");
    let v: serde_json::Value = serde_json::from_str(&json).expect("parse");

    assert_eq!(v["kind"], "liveMatchCommandUnimplemented");
    assert_eq!(v["commandKind"], "substitute");
    // No snake_case `command_kind` leakage.
    assert!(
        v.as_object().unwrap().get("command_kind").is_none(),
        "snake_case 'command_kind' must not appear in wire JSON"
    );
}

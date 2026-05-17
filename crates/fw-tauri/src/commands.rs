//! Tauri command handlers — the IPC entry points the frontend invokes.
//!
//! Lives in a separate module from `lib.rs` because of a known Tauri 2
//! limitation: `#[tauri::command]` on a `pub` function inside `lib.rs`
//! produces `E0255 __cmd__<name> defined multiple times` (the macro
//! generates `pub use __cmd__<name>` AND uses the name locally, which
//! clashes inside the crate root). Moving the commands one level down
//! into `mod commands` sidesteps the clash entirely.
//!
//! Reference: <https://github.com/tauri-apps/tauri/discussions/4665>
//!
//! ## Testing strategy
//!
//! `tauri::State<'_, T>` is not directly constructable in unit tests —
//! only `tauri::Manager::state()` on a live App produces one. To keep tests
//! fast and framework-free, each command delegates to a corresponding
//! `_inner` free function that takes `&AppState` directly. Tests call the
//! `_inner` variant; the `#[tauri::command]` wrappers are thin forwarding
//! shells validated by the integration test in `crates/fw-tauri/tests/`.

use fw_content::Fixture;
use fw_core::Seed;
use fw_match_sim::{MatchState, tick_match};

use crate::state::{fixture_seed, league_fixture_index};
use crate::{
    AdvanceWeekSummaryDto, AppState, BackendHandshakeDto, FixtureWithResultDto, IpcError,
    MAX_FRAMES_PER_REQUEST, MatchFrameDto, MatchResult, PlayFixturesSummaryDto, StandingsRowDto,
    season,
};

// ---------------------------------------------------------------------------
// Public `#[tauri::command]` wrappers (thin shells; logic in `_inner` fns)
// ---------------------------------------------------------------------------

/// `play_match(seed_hex, tick_count)` — run a match end-to-end and return
/// the final `MatchResult` DTO including the canonical hash and per-event
/// commentary.
///
/// `seed_hex` accepts `"0x..."` or bare hex. Returns `IpcError::InvalidSeed`
/// if malformed; `IpcError::MatchInitFailed` if the sim cannot start.
#[tauri::command]
pub async fn play_match(
    seed_hex: String,
    tick_count: u32,
    state: tauri::State<'_, AppState>,
) -> Result<MatchResult, IpcError> {
    play_match_inner(seed_hex, tick_count, &state).await
}

/// `get_backend_handshake()` — return a `BackendHandshakeDto` proving the
/// Rust sim is alive. Codex 2026-05-16 Tier-2 fix-pass replaces
/// `get_dummy_state` (which after T1-5's IPC consolidation returned
/// `MatchStateDto` — the WRONG shape for `Home.tsx`'s liveness check, which
/// renders `appVersion` / `message` / `backendReady` fields).
///
/// No AppState injection needed. The `IpcError` return type matches the
/// other commands per Tauri/RULES.md §4 (uniform typed-error surface) even
/// though this handshake cannot fail today — reaching the handler means
/// Tauri delivered the IPC, so `backend_ready = true` is hard-coded.
#[tauri::command]
pub async fn get_backend_handshake() -> Result<BackendHandshakeDto, IpcError> {
    Ok(BackendHandshakeDto::live())
}

/// `match_frames(seed_hex, tick_count)` — produce a sequence of per-tick
/// frames for the dev-tier 2D tactical board.
///
/// Returns `Vec<MatchFrameDto>` of length `tick_count + 1` (one entry per
/// tick from `0` through `tick_count` inclusive).
///
/// Returns `IpcError::TooManyFrames` when `tick_count > MAX_FRAMES_PER_REQUEST`
/// **before** any allocation or sim work is done.
#[tauri::command]
pub async fn match_frames(
    seed_hex: String,
    tick_count: u32,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<MatchFrameDto>, IpcError> {
    match_frames_inner(seed_hex, tick_count, &state).await
}

// ---------------------------------------------------------------------------
// Inner logic (takes `&AppState`; testable without `tauri::State`)
// ---------------------------------------------------------------------------

pub async fn play_match_inner(
    seed_hex: String,
    tick_count: u32,
    state: &AppState,
) -> Result<MatchResult, IpcError> {
    let seed = parse_seed_hex(&seed_hex)?;
    // `?` exercises `From<ContentLoadError> for IpcError` (see error.rs).
    let mut sim_state = MatchState::initial_with_content(
        seed,
        state.content(),
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
    )?;
    for _ in 0..tick_count {
        sim_state = tick_match(sim_state, state.signature_definitions());
    }
    MatchResult::from_state(&sim_state, seed_hex, tick_count, state.content())
}

pub async fn match_frames_inner(
    seed_hex: String,
    tick_count: u32,
    state: &AppState,
) -> Result<Vec<MatchFrameDto>, IpcError> {
    // Guard fires BEFORE any allocation or sim work (acceptance criterion 4).
    if tick_count > MAX_FRAMES_PER_REQUEST {
        return Err(IpcError::TooManyFrames {
            requested: tick_count,
            max: MAX_FRAMES_PER_REQUEST,
        });
    }

    let seed = parse_seed_hex(&seed_hex)?;
    // `?` exercises `From<ContentLoadError> for IpcError` (see error.rs).
    let mut sim_state = MatchState::initial_with_content(
        seed,
        state.content(),
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
    )?;

    // tick_count + 1 frames: index 0 = initial state, index tick_count = final.
    // The guard above caps tick_count at MAX_FRAMES_PER_REQUEST (= 7200), so
    // `(tick_count as usize) + 1` cannot overflow on any platform with
    // `usize >= u32` (which the workspace requires). The debug_assert pins
    // the invariant at the call site so a future relaxation of the guard
    // fails loudly here rather than silently allocating usize::MAX.
    debug_assert!(tick_count <= MAX_FRAMES_PER_REQUEST);
    let total = (tick_count as usize) + 1;
    let mut frames = Vec::with_capacity(total);
    frames.push(MatchFrameDto::from_state(&sim_state));
    for _ in 0..tick_count {
        sim_state = tick_match(sim_state, state.signature_definitions());
        frames.push(MatchFrameDto::from_state(&sim_state));
    }
    Ok(frames)
}

// ---------------------------------------------------------------------------
// Season IPC commands (T2-5)
// ---------------------------------------------------------------------------

/// `advance_week()` — play the current match-day and advance the season by 1.
///
/// Plays all 10 fixtures on `season.current_match_day` deterministically,
/// records their results, and bumps `current_match_day` by 1. Returns a
/// summary DTO. Errors with `IpcError::SeasonComplete` if called after the
/// final match-day.
#[tauri::command]
pub async fn advance_week(
    state: tauri::State<'_, AppState>,
) -> Result<AdvanceWeekSummaryDto, IpcError> {
    advance_week_inner(&state)
}

/// `play_fixtures()` — fast-forward all remaining fixtures in one call.
///
/// Calls `advance_week_inner` in a loop until the season is complete.
/// Returns a summary with total matches played + the final match-day reached.
#[tauri::command]
pub async fn play_fixtures(
    state: tauri::State<'_, AppState>,
) -> Result<PlayFixturesSummaryDto, IpcError> {
    play_fixtures_inner(&state)
}

/// `get_standings()` — return the current league table (20 rows).
///
/// Sort order: `(points DESC, goal_difference DESC, goals_for DESC, club_id ASC)`.
#[tauri::command]
pub async fn get_standings(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<StandingsRowDto>, IpcError> {
    get_standings_inner(&state)
}

/// `get_fixtures(club_id)` — return all 38 fixtures for the given club.
///
/// Each entry includes the match result if already played (`homeScore` /
/// `awayScore` as `Option<u8>`). Returns `IpcError::ClubNotFound` if the
/// club is not in the current league.
#[tauri::command]
pub async fn get_fixtures(
    club_id: u32,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<FixtureWithResultDto>, IpcError> {
    get_fixtures_inner(club_id, &state)
}

// ---------------------------------------------------------------------------
// Season inner helpers (sync, take &AppState, testable without tauri::State)
// ---------------------------------------------------------------------------

pub fn advance_week_inner(state: &AppState) -> Result<AdvanceWeekSummaryDto, IpcError> {
    let mut season = state.season().write().map_err(|_| IpcError::LockPoisoned {
        lock: "season".to_string(),
    })?;

    if season.is_complete() {
        return Err(IpcError::SeasonComplete);
    }

    let match_day = season.current_match_day;

    // Collect the fixtures for this match-day. Clone because we need to drop
    // the borrow on `season` before calling `apply_result` / accessing fields.
    let fixtures: Vec<Fixture> = season
        .fixtures_for_match_day(match_day)
        .into_iter()
        .copied()
        .collect();

    let career_seed = state.career_seed();

    // Post-T2-5 silent-failure-hunter P1-1 fix: transactional pre-loop snapshot.
    // Prior code would commit a PARTIAL match-day on mid-loop failure — e.g. if
    // play_one_match errors on fixture 7/10, fixtures 1-6 stayed in
    // season.results while `current_match_day` did NOT advance. A retry would
    // replay 1-6 (silently overwriting via apply_result), then fail again on
    // fixture 7. Without telemetry the partial commit is invisible.
    //
    // The fix: snapshot results pre-loop, restore on `?` early-return so the
    // mutation is atomic per advance_week() call. BTreeMap clone is sub-
    // microsecond; cost is negligible vs the perf benefit was overstated.
    let results_snapshot = season.results.clone();

    for fixture in &fixtures {
        let idx = league_fixture_index(&season.league.fixtures, fixture) as u32;
        let seed = fixture_seed(career_seed, idx);
        let home_arch = season.tactical_archetype_ids[&fixture.home].clone();
        let away_arch = season.tactical_archetype_ids[&fixture.away].clone();
        match season::play_one_match(
            seed,
            state.content(),
            state.signature_definitions(),
            &home_arch,
            &away_arch,
            season::SEASON_MATCH_TICK_BUDGET,
        ) {
            Ok(outcome) => {
                season.apply_result(fixture.home, fixture.away, outcome);
            }
            Err(e) => {
                // Atomic rollback: restore results to the pre-loop snapshot
                // so the user-visible state matches the error semantic ("this
                // match-day did NOT advance"). current_match_day was never
                // bumped (the bump is after the loop), so no rollback needed
                // there.
                season.results = results_snapshot;
                return Err(e);
            }
        }
    }

    season.current_match_day += 1;

    Ok(AdvanceWeekSummaryDto {
        match_day_played: match_day,
        matches_played: fixtures.len() as u16,
        season_complete: season.is_complete(),
    })
}

pub fn play_fixtures_inner(state: &AppState) -> Result<PlayFixturesSummaryDto, IpcError> {
    let mut total_matches: u32 = 0;
    loop {
        // Post-T2-5 code-reviewer fix: prior code took a separate read lock
        // to check `is_complete()` before calling `advance_week_inner` (which
        // takes its own write lock) — a TOCTOU window between the lock
        // releases. Instead, drive the loop by `advance_week_inner`'s
        // own SeasonComplete error: if the season is already done at entry,
        // advance_week_inner returns SeasonComplete and we treat that as the
        // CLEAN loop-exit signal rather than propagating it as a play_fixtures
        // failure. Eliminates the TOCTOU + the redundant lock acquisition.
        //
        // Mid-loop failure semantic (post-T2-5 silent-failure-hunter P1-2):
        // `advance_week_inner` is transactional per match-day (see its
        // P1-1 rollback). If a fixture errors mid-week, that ENTIRE
        // match-day is rolled back, the error propagates here via the
        // explicit `Err(other)` arm, and any prior fully-played match-days
        // (with `total_matches` tracking them) remain committed. The user
        // retries `play_fixtures` and resumes from the same match-day.
        // Determinism guarantees identical outcomes on retry.
        match advance_week_inner(state) {
            Ok(summary) => total_matches += summary.matches_played as u32,
            Err(IpcError::SeasonComplete) => break,
            Err(other) => return Err(other),
        }
    }
    // Post-T2-5 type-design P0 fix: prior code used `current_match_day.saturating_sub(1)`
    // which silently returns 0 when the loop body never ran (season already complete
    // at entry). Sim/RULES.md §11 bans saturating arithmetic on gameplay-bearing
    // fields without explicit justification. Be explicit instead: distinguish
    // "loop ran" from "loop didn't run" via `total_matches` and return 0 in the
    // didn't-run case rather than silently saturating.
    let final_match_day = if total_matches == 0 {
        // Season was already complete at entry; no advancement happened.
        // Report current_match_day - 1 if it's > 0, else 0 (representing
        // "no fixtures played" or "all fixtures already played at entry").
        let current = state
            .season()
            .read()
            .map_err(|_| IpcError::LockPoisoned {
                lock: "season".to_string(),
            })?
            .current_match_day;
        if current == 0 { 0 } else { current - 1 }
    } else {
        let current = state
            .season()
            .read()
            .map_err(|_| IpcError::LockPoisoned {
                lock: "season".to_string(),
            })?
            .current_match_day;
        // `current` is the NEXT match-day to play; the last successfully played
        // is `current - 1`. The loop guarantees `current >= 1` because each
        // iteration advanced current_match_day at least once before exit.
        debug_assert!(current >= 1, "advance_week_inner advanced current >= 1");
        current - 1
    };

    Ok(PlayFixturesSummaryDto {
        matches_played: total_matches,
        final_match_day,
    })
}

pub fn get_standings_inner(state: &AppState) -> Result<Vec<StandingsRowDto>, IpcError> {
    let season = state.season().read().map_err(|_| IpcError::LockPoisoned {
        lock: "season".to_string(),
    })?;
    let standings = season.standings();

    // club_name is already on StandingsRow from standings(); no separate
    // BTreeMap lookup needed (post-T2-5 silent-failure-hunter cleanup —
    // the prior `let _ = club_names.get(...)` was dead code per clippy
    // path-of-noticing).
    let rows: Vec<StandingsRowDto> = standings
        .rows
        .into_iter()
        .map(|r| StandingsRowDto {
            club_id: r.club_id.raw(),
            club_name: r.club_name,
            played: r.played,
            wins: r.wins,
            draws: r.draws,
            losses: r.losses,
            goals_for: r.goals_for,
            goals_against: r.goals_against,
            goal_difference: r.goal_difference,
            points: r.points,
        })
        .collect();

    Ok(rows)
}

pub fn get_fixtures_inner(
    club_id_raw: u32,
    state: &AppState,
) -> Result<Vec<FixtureWithResultDto>, IpcError> {
    let club_id = fw_core::ClubId::new(club_id_raw);
    let season = state.season().read().map_err(|_| IpcError::LockPoisoned {
        lock: "season".to_string(),
    })?;

    // Validate that the club is in the league before doing any work.
    let club_exists = season.league.clubs.iter().any(|c| c.id == club_id);
    if !club_exists {
        return Err(IpcError::ClubNotFound {
            club_id: club_id_raw,
        });
    }

    // Build a ClubId → display_name map for the opponent name lookup.
    let club_names: std::collections::BTreeMap<fw_core::ClubId, &str> = season
        .league
        .clubs
        .iter()
        .map(|c| (c.id, c.display_name.as_str()))
        .collect();

    let fixtures = season.fixtures_for_club(club_id);

    let dtos: Vec<FixtureWithResultDto> = fixtures
        .into_iter()
        .map(|(fixture, outcome)| {
            let is_home = fixture.home == club_id;
            let opponent_id = if is_home { fixture.away } else { fixture.home };
            // Post-T2-5 silent-failure-hunter P1-4 fix: prior code used
            // `.unwrap_or_default()` which silently rendered an empty string
            // when the opponent ID wasn't in `club_names`. The fixture comes
            // FROM `season.fixtures_for_club(club_id)` whose entries are
            // built FROM `season.league.fixtures` — which `generate_league`
            // guarantees only references clubs present in `season.league.clubs`.
            // The `unwrap_or_default` therefore masked an impossible-state bug
            // as silent UI corruption. Promote to `expect` so the violation
            // surfaces at the moment of breakage rather than as a blank cell.
            let opponent_name = club_names.get(&opponent_id).map(|s| s.to_string()).expect(
                "fixture opponent must be in league.clubs \
                     (generate_league invariant)",
            );
            FixtureWithResultDto {
                match_day: fixture.match_day,
                opponent_club_id: opponent_id.raw(),
                opponent_club_name: opponent_name,
                is_home,
                played: outcome.is_some(),
                home_score: outcome.map(|o| o.home_score),
                away_score: outcome.map(|o| o.away_score),
            }
        })
        .collect();

    Ok(dtos)
}

// ---------------------------------------------------------------------------
// Shared helper
// ---------------------------------------------------------------------------

fn parse_seed_hex(seed_hex: &str) -> Result<Seed, IpcError> {
    let trimmed = seed_hex.trim_start_matches("0x");
    let raw = u64::from_str_radix(trimmed, 16).map_err(|e| IpcError::InvalidSeed {
        input: seed_hex.to_string(),
        reason: e.to_string(),
    })?;
    Ok(Seed::from_u64(raw))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use fw_core::Seed;
    use fw_match_sim::{MatchState, tick_match};

    use super::*;
    use crate::state::AppState;

    fn workspace_content_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("content")
    }

    fn test_app_state() -> AppState {
        AppState::new(&workspace_content_path()).expect("AppState::new in test")
    }

    // ---- parse_seed_hex ----

    #[test]
    fn parse_seed_hex_accepts_0x_prefix() {
        assert_eq!(
            parse_seed_hex("0xdeadbeef").expect("parse").to_u64(),
            0xDEAD_BEEF
        );
    }

    #[test]
    fn parse_seed_hex_accepts_bare_hex() {
        assert_eq!(parse_seed_hex("1").expect("parse bare").to_u64(), 1);
    }

    #[test]
    fn parse_seed_hex_rejects_invalid() {
        let err = parse_seed_hex("0xggg").expect_err("should fail");
        match err {
            IpcError::InvalidSeed { input, .. } => assert!(input.contains("ggg")),
            other => panic!("wrong variant: {other:?}"),
        }
    }

    // ---- MAX_FRAMES guard (chunk 3 acceptance criterion) ----

    #[test]
    fn match_frames_over_max_returns_too_many_frames_before_alloc() {
        let state = test_app_state();
        let err = tauri::async_runtime::block_on(match_frames_inner(
            "0x1".to_string(),
            MAX_FRAMES_PER_REQUEST + 1,
            &state,
        ))
        .expect_err("should reject over-max tick_count");
        match err {
            IpcError::TooManyFrames { requested, max } => {
                assert_eq!(requested, MAX_FRAMES_PER_REQUEST + 1);
                assert_eq!(max, MAX_FRAMES_PER_REQUEST);
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn match_frames_tick_count_zero_returns_one_frame() {
        let state = test_app_state();
        let frames =
            tauri::async_runtime::block_on(match_frames_inner("0x1".to_string(), 0, &state))
                .expect("zero ticks");
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].tick, 0);
    }

    #[test]
    fn match_frames_returns_tick_count_plus_one() {
        let state = test_app_state();
        let frames =
            tauri::async_runtime::block_on(match_frames_inner("0xdeadbeef".to_string(), 5, &state))
                .expect("5 ticks");
        assert_eq!(frames.len(), 6);
        assert_eq!(frames[0].tick, 0);
        assert_eq!(frames[5].tick, 5);
    }

    // ---- play_match ----

    #[test]
    fn play_match_returns_match_result_with_blake3_hash() {
        let state = test_app_state();
        let result = tauri::async_runtime::block_on(play_match_inner(
            "0xdeadbeefdeadbeef".to_string(),
            60,
            &state,
        ))
        .expect("play_match");
        assert!(result.canonical_hash.starts_with("blake3:"));
        assert_eq!(result.canonical_hash.len(), 7 + 64);
        assert_eq!(result.tick_count, 60);
        assert_eq!(result.seed_hex, "0xdeadbeefdeadbeef");
    }

    #[test]
    fn play_match_canonical_hash_matches_independent_computation() {
        let state = test_app_state();

        // Independent BLAKE3 computation.
        let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);
        let mut sim_state = MatchState::initial_with_content(
            seed,
            state.content(),
            fw_match_sim::DEFAULT_ARCHETYPE_ID,
            fw_match_sim::DEFAULT_ARCHETYPE_ID,
        )
        .expect("init");
        for _ in 0..60 {
            sim_state = tick_match(sim_state, state.signature_definitions());
        }
        let bytes = sim_state.encode_canonical();
        let hash_bytes: [u8; 32] = blake3::hash(&bytes).into();
        let expected = format!(
            "blake3:{}",
            hash_bytes
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );

        // IPC inner path.
        let result = tauri::async_runtime::block_on(play_match_inner(
            "0xdeadbeefdeadbeef".to_string(),
            60,
            &state,
        ))
        .expect("play_match");

        assert_eq!(
            result.canonical_hash, expected,
            "IPC canonical_hash must match independent BLAKE3 computation"
        );
    }

    #[test]
    fn play_match_commentary_preview_length_matches_events() {
        let state = test_app_state();
        let result = tauri::async_runtime::block_on(play_match_inner(
            "0xdeadbeefdeadbeef".to_string(),
            60,
            &state,
        ))
        .expect("play_match");
        assert_eq!(result.commentary_preview.len(), result.match_events.len());
    }

    // ---- IpcError serialization (acceptance criterion 3) ----

    #[test]
    fn too_many_frames_error_serializes_as_discriminated_union() {
        let err = IpcError::TooManyFrames {
            requested: MAX_FRAMES_PER_REQUEST + 1,
            max: MAX_FRAMES_PER_REQUEST,
        };
        let json = serde_json::to_string(&err).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(v["kind"], "tooManyFrames");
        assert_eq!(v["requested"], MAX_FRAMES_PER_REQUEST + 1);
        assert_eq!(v["max"], MAX_FRAMES_PER_REQUEST);
    }
}

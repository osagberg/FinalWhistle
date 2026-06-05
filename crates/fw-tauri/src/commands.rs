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

use std::collections::BTreeMap;

use fw_content::{
    Fixture, MemoryCallbackContext, SeasonState, discriminant_to_family_key, gene_family_pa_ca,
    generate_league, generate_league_with_teams, render_memory_callback,
};
use fw_core::{ClubId, PlayerId, Seed, Tick};
use fw_match_sim::{MatchState, PLAYERS_PER_TEAM, tick_match};
use fw_memory::event::{EventClass, SeasonNumber};
use fw_memory::readers::press::PressReader;
use fw_memory::readers::{PressTopic, SalienceFilter, project_salience, salience::SalienceReader};
use fw_memory::{BreakthroughContext, BreakthroughOutcome, evaluate};
use fw_save::{self, SaveEnvelope, SaveV4, SavedPlayerInstance};

use crate::live_match::session::LiveMatchSession;
use crate::live_match::snapshot::{project_final, project_snapshot};
use crate::live_match::types::{
    FinalMatchResult, MatchCommand, MatchHandle, MatchSnapshot, StepResult,
};
use crate::roster_dto::{PlayerRosterDto, ScoutReportDto};
use crate::state::{fixture_seed, league_fixture_index};
use crate::{
    AdvanceSeasonSummaryDto, AdvanceWeekSummaryDto, AppState, BackendHandshakeDto,
    CareerOverviewDto, ChampionHistoryEntryDto, FixtureWithResultDto, IpcError,
    MAX_FRAMES_PER_REQUEST, MatchFrameDto, MatchResult, PlayFixturesSummaryDto, PlayerDetailDto,
    PlayerPhenotypeDto, PressInboxDto, PressItemDto, SquadPlayerDto, StandingsRowDto, season,
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

/// `get_player_detail(player_id)` — return the player detail DTO for one player.
///
/// Returns:
/// - `phenotype`: bio data from the content store (name, role, region, labels).
/// - `memoryCallbacks`: rendered career moment strings from the memory ledger
///   (empty when the runtime ledger is empty — honest, not fabricated).
/// - `contractStatus`: `None` until the T4 career-roster layer lands.
///
/// Returns `IpcError::PlayerNotFound` if `player_id` is not in the content store.
#[tauri::command]
pub async fn get_player_detail(
    player_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<PlayerDetailDto, IpcError> {
    get_player_detail_inner(&player_id, &state)
}

/// `advance_season()` — complete the current season and advance to the next one.
///
/// Requires the current season to be complete (`IpcError::SeasonNotComplete`
/// if not). Emits the season-end `TitleWon` event into the ledger, runs
/// compaction if at the 5-season boundary, regenerates a fresh `SeasonState`,
/// and increments `season_number`.
#[tauri::command]
pub async fn advance_season(
    state: tauri::State<'_, AppState>,
) -> Result<AdvanceSeasonSummaryDto, IpcError> {
    advance_season_inner(&state)
}

/// `get_career_overview()` — return the career overview DTO.
///
/// Returns the current season number, per-season champion history, and
/// rendered cross-season memory callbacks from `TitleWon` events in past
/// seasons.
#[tauri::command]
pub async fn get_career_overview(
    state: tauri::State<'_, AppState>,
) -> Result<CareerOverviewDto, IpcError> {
    get_career_overview_inner(&state)
}

/// `get_press_inbox()` — return ranked press-conference items from the career ledger.
///
/// For each of the 4 `PressTopic` variants, calls `PressReader::candidates` to
/// fetch callback-eligible events. Merges the four lists, deduplicates by
/// `event_id` (keeping the first topic's assignment in declaration order:
/// `PlayerMilestone` → `ContractTransfer` → `MatchResult` → `Relational`),
/// re-sorts by projected salience descending with `event_id` ascending as a
/// tiebreak, and caps at 20 items. Each item's `headline` is rendered via
/// `render_memory_callback`.
///
/// Returns `IpcError::LockPoisoned` if the career write lock cannot be acquired.
#[tauri::command]
pub async fn get_press_inbox(state: tauri::State<'_, AppState>) -> Result<PressInboxDto, IpcError> {
    get_press_inbox_inner(&state)
}

// ---------------------------------------------------------------------------
// Live-match command quintet (T4-5a) — ADR-0004 §1
// ---------------------------------------------------------------------------

/// `start_live_match(seedHex)` — allocate a new live-match session.
///
/// Parses the seed, initialises a fresh `MatchState`, inserts a
/// `LiveMatchSession` into `AppState::live_matches`, and returns a
/// `MatchHandle { id, seedHex }`.
#[tauri::command]
pub async fn start_live_match(
    seed_hex: String,
    state: tauri::State<'_, AppState>,
) -> Result<MatchHandle, IpcError> {
    start_live_match_inner(seed_hex, &state)
}

/// `step_live_match(handle, ticks)` — advance the live match by `ticks` ticks.
///
/// Returns a `StepResult` containing the events emitted during this call
/// (delta since the previous `step_live_match`), current score, tick, and
/// a `isFinished` flag.
///
/// Returns `IpcError::TooManyFrames` when `ticks > MAX_FRAMES_PER_REQUEST`.
/// Returns `IpcError::MatchInitFailed` when `handle.id` is not found.
#[tauri::command]
pub async fn step_live_match(
    handle: MatchHandle,
    ticks: u32,
    state: tauri::State<'_, AppState>,
) -> Result<StepResult, IpcError> {
    step_live_match_inner(handle, ticks, &state)
}

/// `get_match_snapshot(handle)` — read the current match state as a fat DTO.
///
/// Powers scoreboard, lineup, and event-feed panels. Non-mutating.
///
/// Returns `IpcError::MatchInitFailed` when `handle.id` is not found.
#[tauri::command]
pub async fn get_match_snapshot(
    handle: MatchHandle,
    state: tauri::State<'_, AppState>,
) -> Result<MatchSnapshot, IpcError> {
    get_match_snapshot_inner(handle, &state)
}

/// `finish_live_match(handle)` — remove the live-match session and return the
/// final result.
///
/// After this call the handle is invalid. Returns `IpcError::MatchInitFailed`
/// when `handle.id` is not found.
#[tauri::command]
pub async fn finish_live_match(
    handle: MatchHandle,
    state: tauri::State<'_, AppState>,
) -> Result<FinalMatchResult, IpcError> {
    finish_live_match_inner(handle, &state)
}

/// `apply_match_command(handle, command)` — enqueue a manager intent.
///
/// All 9 `MatchCommand` variants currently return
/// `IpcError::LiveMatchCommandUnimplemented`. The command is deserialized and
/// recorded in the session's `pending_commands` audit trail before the error
/// is returned. Returns `IpcError::MatchInitFailed` when `handle.id` is not found.
#[tauri::command]
pub async fn apply_match_command(
    handle: MatchHandle,
    command: MatchCommand,
    state: tauri::State<'_, AppState>,
) -> Result<(), IpcError> {
    apply_match_command_inner(handle, command, &state)
}

/// `start_live_match_for_fixture(homeClubId, awayClubId)` — start a live
/// session for the user's real fixture, construction-equivalent to
/// `advance_week`'s AI-sim path.
///
/// The session is registered in `AppState::live_matches` and the returned
/// `MatchHandle` can be passed directly to `step_live_match`,
/// `get_match_snapshot`, `apply_match_command`, and `finish_live_match`.
///
/// When no in-match decisions are made, stepping to completion produces the
/// same final score and canonical state as `advance_week` would for the same
/// fixture.
#[tauri::command]
pub async fn start_live_match_for_fixture(
    home_club_id: u32,
    away_club_id: u32,
    state: tauri::State<'_, AppState>,
) -> Result<MatchHandle, IpcError> {
    start_live_match_for_fixture_inner(home_club_id, away_club_id, &state)
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
    // No roster wired for the dev play_match path yet — pass empty slot_names;
    // render_event will fall back to positional labels ("a forward" etc.).
    MatchResult::from_state(
        &sim_state,
        seed_hex,
        tick_count,
        state.content(),
        &std::collections::BTreeMap::new(),
    )
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
    //
    // T2-R-B5 (post-T2 ultimate-review Track B-5): debug_assert is
    // intentional here. fw-tauri is OUTSIDE Sim/RULES.md §11's path
    // scope (the rule covers fw-match-sim / fw-memory / fw-replay /
    // fw-save / fw-core / fw-content / fw-scouting only). The
    // load-bearing safety check is the explicit `if tick_count >
    // MAX_FRAMES_PER_REQUEST { bail!(...) }` immediately above this
    // assert; debug_assert is the redundant invariant pin for the
    // "future relaxation of the guard" case. Promotion to assert!
    // is unnecessary because release builds already have the explicit
    // guard providing the load-bearing check.
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

/// `get_scout_report(player_id)` — return the latest scouting report for a roster player.
///
/// Returns:
/// - `Ok(ScoutReportDto)` when the player has featured in at least one match-day.
/// - `IpcError::NotYetObserved` when the player has not yet appeared (no report cached).
/// - `IpcError::PlayerNotFound` when the player_id is not a valid roster id.
///
/// Only roster players (id ≥ `ROSTER_PLAYER_ID_BASE`) have scouting reports.
/// Content-bio ids (`< ROSTER_PLAYER_ID_BASE`) route to `get_player_detail` instead.
#[tauri::command]
pub async fn get_scout_report(
    player_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<ScoutReportDto, IpcError> {
    get_scout_report_inner(&player_id, &state)
}

/// `get_squad()` — return all player bios from the content store as a flat list.
///
/// Returns the 22-player pool from `ContentStore.player_bios` in BTreeMap
/// key order (deterministic, content-pack-qualified ID order). Columns
/// available: name, role, birth region, phenotype labels (human-readable).
/// Age and contract are deliberately absent — they are T4+ career-roster state.
#[tauri::command]
pub async fn get_squad(state: tauri::State<'_, AppState>) -> Result<Vec<SquadPlayerDto>, IpcError> {
    get_squad_inner(&state)
}

// ---------------------------------------------------------------------------
// Season inner helpers (sync, take &AppState, testable without tauri::State)
// ---------------------------------------------------------------------------

pub fn advance_week_inner(state: &AppState) -> Result<AdvanceWeekSummaryDto, IpcError> {
    let mut career = state.career().write().map_err(|_| IpcError::LockPoisoned {
        lock: "career".to_string(),
    })?;

    if career.season.is_complete() {
        return Err(IpcError::SeasonComplete);
    }

    let match_day = career.season.current_match_day;

    // Collect the fixtures for this match-day. Clone because we need to drop
    // the borrow on `season` before calling `apply_result` / accessing fields.
    let fixtures: Vec<Fixture> = career
        .season
        .fixtures_for_match_day(match_day)
        .into_iter()
        .copied()
        .collect();

    let career_seed = state.career_seed();

    // Atomicity contract (T4-2.5e): the harvest (career_apps increments +
    // ledger appends) must be all-or-nothing for the match-day. Doing the
    // harvest inside the play loop would commit partial roster/ledger mutations
    // if a later fixture errors — on retry career_apps would double-count and
    // the ledger would accumulate duplicate events (the ledger has no dedup;
    // the append-only contract makes mutation-reversal impossible).
    //
    // Pattern: collect (fixture, MatchState) for every successfully played
    // fixture during the loop; apply ALL mutations only after the loop exits
    // cleanly. A mid-loop `play_one_match` Err triggers the season.results
    // rollback (existing T2-5 fix) and returns before the apply phase, leaving
    // career.roster and career.ledger untouched.
    let results_snapshot = career.season.results.clone();

    // Accumulate played states. Capacity = number of fixtures on this match-day
    // (always 10 for a 20-club season; Vec avoids premature abstraction).
    let mut played: Vec<(fw_content::Fixture, fw_match_sim::MatchState)> =
        Vec::with_capacity(fixtures.len());

    for fixture in &fixtures {
        let idx =
            league_fixture_index(&career.season.league.fixtures, fixture).ok_or_else(|| {
                IpcError::LeagueGenerationFailed {
                    reason: "fixture not present in league.fixtures (generate_league invariant)"
                        .to_string(),
                }
            })? as u32;
        let seed = fixture_seed(career_seed, idx);
        let home_arch = career
            .season
            .tactical_archetype_ids
            .get(&fixture.home)
            .cloned()
            .ok_or_else(|| IpcError::LeagueGenerationFailed {
                reason: format!(
                    "no tactical archetype for club {} (generate_league invariant)",
                    fixture.home.raw()
                ),
            })?;
        let away_arch = career
            .season
            .tactical_archetype_ids
            .get(&fixture.away)
            .cloned()
            .ok_or_else(|| IpcError::LeagueGenerationFailed {
                reason: format!(
                    "no tactical archetype for club {} (generate_league invariant)",
                    fixture.away.raw()
                ),
            })?;
        let slot_signatures = {
            let home_instances = career.roster.get(&fixture.home);
            let away_instances = career.roster.get(&fixture.away);
            match (home_instances, away_instances) {
                (Some(home), Some(away)) => Some(season::build_slot_signatures(
                    home.as_slice(),
                    away.as_slice(),
                )),
                _ => None,
            }
        };

        match season::play_one_match(
            seed,
            state.content(),
            state.signature_definitions(),
            &home_arch,
            &away_arch,
            season::SEASON_MATCH_TICK_BUDGET,
            slot_signatures,
        ) {
            Ok((outcome, match_state)) => {
                career
                    .season
                    .apply_result(fixture.home, fixture.away, outcome);
                played.push((*fixture, match_state));
            }
            Err(e) => {
                // Rollback season.results to the pre-loop snapshot. No harvest
                // has run yet so career.roster and career.ledger are clean.
                career.season.results = results_snapshot;
                return Err(e);
            }
        }
    }

    // All fixtures succeeded. Apply the harvest atomically in two steps per
    // fixture so that career.roster and career.ledger borrows do not overlap:
    //
    // Step A: mutate career.roster (career_apps increment + scout observe) and
    //         collect the MemoryEvents to emit. Each half-call returns a
    //         Vec<MemoryEvent>; the roster borrow is dropped before step B.
    //
    // Step B: append the collected events to career.ledger (no roster borrow).
    //
    // This two-step pattern avoids the E0499 dual-mutable-borrow: the borrow
    // checker cannot prove that `career.roster` and `career.ledger` are
    // disjoint fields through a RwLockWriteGuard reference, so they must be
    // borrowed in non-overlapping scopes.
    //
    // Pillar-4 (T4-2.5f): materialise the bio pool + scout ONCE before the
    // fixture loop — not per fixture. `state.content()` is an immutable borrow
    // on `AppState`, disjoint from `career` (a separate field), so both borrows
    // coexist fine here.
    let bios: Vec<&fw_content::PlayerBio> = state.content().player_bios.values().collect();
    let scout = fw_scouting::Scout::basic_uncertainty();
    let career_seed_u64 = career_seed.to_u64();

    let season_num = career.season_number;
    for (fixture, match_state) in &played {
        // Step A — home half. Borrow ends before step B.
        let home_events = if let Some(home_vec) = career.roster.get_mut(&fixture.home) {
            // Observe home starting XI (T4-2.5f pillar-4).
            season::observe_match_participants(
                home_vec.as_mut_slice(),
                &bios,
                &scout,
                career_seed_u64,
            );
            season::harvest_match_memory_events(
                match_state,
                home_vec.as_mut_slice(),
                &mut [],
                season_num,
            )
        } else {
            Vec::new()
        };

        // Step A — away half. Separate borrow scope.
        let away_events = if let Some(away_vec) = career.roster.get_mut(&fixture.away) {
            // Observe away starting XI (T4-2.5f pillar-4).
            season::observe_match_participants(
                away_vec.as_mut_slice(),
                &bios,
                &scout,
                career_seed_u64,
            );
            season::harvest_match_memory_events(
                match_state,
                &mut [],
                away_vec.as_mut_slice(),
                season_num,
            )
        } else {
            Vec::new()
        };

        // Step B — append both event vecs to the ledger. No roster borrow active.
        for event in home_events.into_iter().chain(away_events) {
            career.ledger.append(event);
        }
    }

    career.season.current_match_day += 1;

    Ok(AdvanceWeekSummaryDto {
        match_day_played: match_day,
        matches_played: fixtures.len() as u16,
        season_complete: career.season.is_complete(),
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
            .career()
            .read()
            .map_err(|_| IpcError::LockPoisoned {
                lock: "career".to_string(),
            })?
            .season
            .current_match_day;
        if current == 0 { 0 } else { current - 1 }
    } else {
        let current = state
            .career()
            .read()
            .map_err(|_| IpcError::LockPoisoned {
                lock: "career".to_string(),
            })?
            .season
            .current_match_day;
        // `current` is the NEXT match-day to play; the last successfully played
        // is `current - 1`. The loop guarantees `current >= 1` because each
        // iteration advanced current_match_day at least once before exit.
        //
        // T2-R-B5 (post-T2 ultimate-review Track B-5): debug_assert
        // intentional here. fw-tauri is OUTSIDE Sim/RULES.md §11's
        // path scope. The load-bearing safety check is the structural
        // invariant guaranteed by the loop body above (each iteration
        // advances current_match_day at least once before reaching this
        // branch); debug_assert is the redundant invariant pin for
        // documentation. Promotion to assert! is unnecessary.
        debug_assert!(current >= 1, "advance_week_inner advanced current >= 1");
        current - 1
    };

    Ok(PlayFixturesSummaryDto {
        matches_played: total_matches,
        final_match_day,
    })
}

pub fn get_standings_inner(state: &AppState) -> Result<Vec<StandingsRowDto>, IpcError> {
    let career = state.career().read().map_err(|_| IpcError::LockPoisoned {
        lock: "career".to_string(),
    })?;
    let standings = career.season.standings();

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
    let career = state.career().read().map_err(|_| IpcError::LockPoisoned {
        lock: "career".to_string(),
    })?;

    // Validate that the club is in the league before doing any work.
    let club_exists = career.season.league.clubs.iter().any(|c| c.id == club_id);
    if !club_exists {
        return Err(IpcError::ClubNotFound {
            club_id: club_id_raw,
        });
    }

    // Build a ClubId → display_name map for the opponent name lookup.
    let club_names: std::collections::BTreeMap<fw_core::ClubId, &str> = career
        .season
        .league
        .clubs
        .iter()
        .map(|c| (c.id, c.display_name.as_str()))
        .collect();

    let fixtures = career.season.fixtures_for_club(club_id);

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
            // as silent UI corruption. A missing opponent is a league-integrity
            // violation — map it to a structured `IpcError` rather than
            // panicking in a handler (Tauri/RULES.md §4); the prior `.expect()`
            // would crash the command and poison the lock.
            let opponent_name = club_names
                .get(&opponent_id)
                .map(|s| s.to_string())
                .ok_or_else(|| IpcError::LeagueGenerationFailed {
                    reason: format!(
                        "fixture opponent {} missing from league.clubs (generate_league invariant)",
                        opponent_id.raw()
                    ),
                })?;
            Ok(FixtureWithResultDto {
                match_day: fixture.match_day,
                opponent_club_id: opponent_id.raw(),
                opponent_club_name: opponent_name,
                is_home,
                played: outcome.is_some(),
                home_score: outcome.map(|o| o.home_score),
                away_score: outcome.map(|o| o.away_score),
            })
        })
        .collect::<Result<Vec<_>, IpcError>>()?;

    Ok(dtos)
}

pub fn get_squad_inner(state: &AppState) -> Result<Vec<SquadPlayerDto>, IpcError> {
    let dtos: Vec<SquadPlayerDto> = state
        .content()
        .player_bios
        .values()
        // BTreeMap::values() iterates in key order — deterministic by contract.
        .map(|bio| {
            let phenotype_labels: Vec<String> = bio
                .scout_labels
                .iter()
                // BTreeSet::iter() is sorted — deterministic order.
                .map(|label| label.display_label().to_string())
                .collect();
            SquadPlayerDto {
                player_id: bio.player_id.clone(),
                name: bio.display_name_full.clone(),
                role: bio.role_family.display_label().to_string(),
                birth_region: bio.birth_region.clone(),
                phenotype_labels,
            }
        })
        .collect();
    Ok(dtos)
}

/// `get_roster_for_club(club_id)` — return the 22 slot-ordered players for a club.
///
/// Returns:
/// - `Vec<PlayerRosterDto>` (22 entries, slot 0 = GK) for a valid club id.
/// - `IpcError::ClubNotFound` for an unknown club id.
///
/// `club_id` is the raw u32 value (matching `ClubId.raw()`).
#[tauri::command]
pub async fn get_roster_for_club(
    club_id: u32,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<PlayerRosterDto>, IpcError> {
    get_roster_for_club_inner(club_id, &state)
}

pub fn get_roster_for_club_inner(
    club_id_raw: u32,
    state: &AppState,
) -> Result<Vec<PlayerRosterDto>, IpcError> {
    let career = state.career().read().map_err(|_| IpcError::LockPoisoned {
        lock: "career".to_string(),
    })?;

    let club_id = fw_core::ClubId::new(club_id_raw);
    let instances = career.roster.get(&club_id).ok_or(IpcError::ClubNotFound {
        club_id: club_id_raw,
    })?;

    // Slot-ordered: the Vec was built in slot 0..21 order at generation time.
    // `assert!` here — not `IpcError` — because a wrong instance count indicates
    // a programming error in `generate_career_roster` (corrupted career state),
    // not a recoverable user-facing condition. Same precedent as `state.rs`'s
    // `expect()` on `generate_league_with_teams`. Tauri/RULES §4 governs
    // user-input validation failures; structural binary corruption is a panic
    // domain per Sim/RULES §11 ("canonical and gameplay invariants MUST fail in
    // release, not silently degrade").
    assert!(
        instances.len() == 22,
        "roster invariant violated: club {:?} has {} instances, expected 22",
        club_id,
        instances.len()
    );

    let dtos: Vec<PlayerRosterDto> = instances
        .iter()
        .map(PlayerRosterDto::from_instance)
        .collect();

    Ok(dtos)
}

/// `get_squad_roster()` — return the default club's squad roster for the Squad screen.
///
/// The "default club" is the lowest `ClubId` in `career.roster` (BTreeMap
/// key order is deterministic; lowest key = club 0 in league order). This is
/// a placeholder until career-start club selection is implemented.
///
/// Returns `IpcError::LeagueGenerationFailed` if the roster is empty (should
/// not occur in a well-formed career; defensive only).
#[tauri::command]
pub async fn get_squad_roster(
    state: tauri::State<'_, AppState>,
) -> Result<crate::roster_dto::SquadRosterDto, IpcError> {
    get_squad_roster_inner(&state)
}

pub fn get_squad_roster_inner(
    state: &AppState,
) -> Result<crate::roster_dto::SquadRosterDto, IpcError> {
    use crate::roster_dto::{PlayerRosterDto, SquadRosterDto};

    let career = state.career().read().map_err(|_| IpcError::LockPoisoned {
        lock: "career".to_string(),
    })?;

    // Anchor on the managed club if one is set + present in the roster; else
    // fall back to the lowest-ClubId placeholder (BTreeMap order). The
    // managed_club_id lock is separate from `career` (read+read, no deadlock).
    let managed = *state
        .managed_club_id()
        .read()
        .map_err(|_| IpcError::LockPoisoned {
            lock: "managed_club_id".to_string(),
        })?;
    let empty_roster = || IpcError::LeagueGenerationFailed {
        reason: "career roster is empty — no club available".to_string(),
    };
    let (club_id, instances, is_managed) = match managed {
        Some(cid) => match career.roster.get_key_value(&cid) {
            Some((id, inst)) => (id, inst, true),
            None => {
                // Managed id absent (e.g. loaded a different-seed save after
                // selecting): fall back to placeholder, surfaced as a
                // greppable warn rather than a silent wrong-club.
                log::warn!(
                    "get_squad_roster: managed club {cid:?} not in the current roster — \
                     falling back to the lowest-ClubId placeholder"
                );
                let (id, inst) = career.roster.iter().next().ok_or_else(empty_roster)?;
                (id, inst, false)
            }
        },
        None => {
            let (id, inst) = career.roster.iter().next().ok_or_else(empty_roster)?;
            (id, inst, false)
        }
    };

    // Structural invariant: each club must have exactly 22 instances.
    assert!(
        instances.len() == 22,
        "get_squad_roster_inner: default club {:?} has {} instances, expected 22 \
         (programming error in generate_career_roster — Sim/RULES §11)",
        club_id,
        instances.len()
    );

    // Resolve the club's display name from the current season's league.
    let club_name = career
        .season
        .league
        .clubs
        .iter()
        .find(|c| c.id == *club_id)
        .map(|c| c.display_name.clone())
        .ok_or_else(|| IpcError::LeagueGenerationFailed {
            reason: format!(
                "default club {} is in roster but not in league.clubs — \
                 league/roster sync invariant violated",
                club_id.raw()
            ),
        })?;

    let players: Vec<PlayerRosterDto> = instances
        .iter()
        .map(PlayerRosterDto::from_instance)
        .collect();

    Ok(SquadRosterDto {
        club_id: club_id.raw(),
        club_name,
        players,
        is_managed,
    })
}

/// Return the cached scouting report for one roster player.
///
/// Routing:
/// - Non-roster ids (suffix < `ROSTER_PLAYER_ID_BASE` or no numeric suffix)
///   → `IpcError::PlayerNotFound`. Scouting is a roster-player feature;
///   content-bio details go through `get_player_detail`.
/// - Roster id not found in `career.roster` → `IpcError::PlayerNotFound`.
/// - Roster id found but `last_scout_report` is `None` (player not yet observed)
///   → `IpcError::NotYetObserved`.
/// - Otherwise → `Ok(ScoutReportDto)`.
pub fn get_scout_report_inner(
    player_id: &str,
    state: &AppState,
) -> Result<ScoutReportDto, IpcError> {
    use crate::roster::ROSTER_PLAYER_ID_BASE;

    let numeric = parse_player_id_suffix(player_id);
    let is_roster_id = numeric.is_some_and(|n| n >= ROSTER_PLAYER_ID_BASE);

    if !is_roster_id {
        return Err(IpcError::PlayerNotFound {
            player_id: player_id.to_string(),
        });
    }

    let raw_id = numeric.expect("is_roster_id true implies numeric is Some");
    let target_pid = PlayerId::new(raw_id);

    let career = state.career().read().map_err(|_| IpcError::LockPoisoned {
        lock: "career".to_string(),
    })?;

    let instance = career
        .roster
        .values()
        .flat_map(|instances| instances.iter())
        .find(|inst| inst.player_id == target_pid)
        .ok_or_else(|| IpcError::PlayerNotFound {
            player_id: player_id.to_string(),
        })?;

    match &instance.last_scout_report {
        None => Err(IpcError::NotYetObserved {
            player_id: player_id.to_string(),
        }),
        Some(report) => {
            // report.player_id == target_pid by construction: observe_match_participants
            // calls observe_player with instance.player_id as subject (F2 fix).
            Ok(ScoutReportDto::from_report(
                report,
                instance.observation_count,
            ))
        }
    }
}

pub fn get_player_detail_inner(
    player_id: &str,
    state: &AppState,
) -> Result<PlayerDetailDto, IpcError> {
    use crate::roster::ROSTER_PLAYER_ID_BASE;

    // 1. Derive the numeric suffix from the content-pack-qualified id string.
    let numeric_player_id: Option<u32> = parse_player_id_suffix(player_id);

    // 2. Route by id range.
    //
    //    Content-bio ids use suffixes 1..=99_999 (< ROSTER_PLAYER_ID_BASE).
    //    Roster ids use ROSTER_PLAYER_ID_BASE + club_idx*22 + slot (≥ BASE).
    //    The two spaces are disjoint by construction (see `roster.rs`).
    //
    //    Bio path  : suffix < ROSTER_PLAYER_ID_BASE (or no suffix).
    //                Use the content-store bio exclusively — its OWN display name,
    //                labels, birth region. No roster override.
    //    Roster path: suffix ≥ ROSTER_PLAYER_ID_BASE.
    //                 Skip the content store; scan the roster for matching PlayerId.
    let is_roster_id = numeric_player_id.is_some_and(|n| n >= ROSTER_PLAYER_ID_BASE);

    // ---- Bio path ----
    if !is_roster_id {
        let bio =
            state
                .content()
                .player_bios
                .get(player_id)
                .ok_or_else(|| IpcError::PlayerNotFound {
                    player_id: player_id.to_string(),
                })?;

        let phenotype_labels: Vec<String> = bio
            .scout_labels
            .iter()
            .map(|label| label.display_label().to_string())
            .collect();
        let phenotype = PlayerPhenotypeDto {
            player_id: bio.player_id.clone(),
            name: bio.display_name_full.clone(),
            role: bio.role_family.display_label().to_string(),
            birth_region: bio.birth_region.clone(),
            phenotype_labels,
        };
        let player_display_name = bio.display_name_full.clone();
        let role_label_for_ctx = phenotype.role.clone();

        return build_player_detail_dto(
            phenotype,
            player_display_name,
            role_label_for_ctx,
            numeric_player_id,
            state,
        );
    }

    // ---- Roster path ----
    // Scan the roster for a PlayerInstance whose player_id.raw() == suffix.
    // The scan is O(clubs × 22) — acceptable at on-demand query cadence.
    let raw_id = numeric_player_id.expect("is_roster_id true implies numeric_player_id is Some");
    let target_pid = PlayerId::new(raw_id);

    let roster_info: Option<(String, String)> = {
        let career = state.career().read().map_err(|_| IpcError::LockPoisoned {
            lock: "career".to_string(),
        })?;
        career
            .roster
            .values()
            .flat_map(|instances| instances.iter())
            .find(|inst| inst.player_id == target_pid)
            .map(|inst| {
                // Role label derived from slot position in the 4-3-3 formation.
                // slot % 11 gives in-team index (GK=0, DEF=1-4, MID=5-7, FWD=8-10).
                let in_team = (inst.slot as usize) % 11;
                let role_label = match in_team {
                    0 => "goalkeeper",
                    1..=4 => "defender",
                    5..=7 => "midfielder",
                    _ => "forward",
                };
                (inst.display_name.clone(), role_label.to_string())
            })
    };

    let (display_name, role_label) = roster_info.ok_or_else(|| IpcError::PlayerNotFound {
        player_id: player_id.to_string(),
    })?;

    let phenotype = PlayerPhenotypeDto {
        player_id: player_id.to_string(),
        name: display_name.clone(),
        role: role_label.clone(),
        birth_region: String::new(),
        phenotype_labels: Vec::new(),
    };

    build_player_detail_dto(phenotype, display_name, role_label, Some(raw_id), state)
}

/// Shared memory-callback rendering logic for both bio and roster player paths.
///
/// Extracted to avoid duplication between the two routing branches of
/// `get_player_detail_inner`. Takes pre-resolved phenotype, display name, and
/// role label; queries the ledger and renders callbacks.
fn build_player_detail_dto(
    phenotype: PlayerPhenotypeDto,
    player_display_name: String,
    role_label_for_ctx: String,
    numeric_player_id: Option<u32>,
    state: &AppState,
) -> Result<PlayerDetailDto, IpcError> {
    // Memory callbacks path.

    let memory_callbacks: Vec<String> = if let Some(raw_id) = numeric_player_id {
        let player_fw_id = PlayerId::new(raw_id);
        let bank = &state.content().memory_callback_grammars;
        let career_seed = state.career_seed().to_u64();

        // SalienceReader::top_n takes `&mut MemoryLedger` for lazy index
        // rebuilds, so we need the write lock on CareerState. We collect the
        // top events into owned values before the render loop so the write
        // guard is held for the shortest possible time — we release it after
        // collecting `top_events` and `club_names`, then render without holding
        // the lock.
        let (top_events, club_names) = {
            let mut career = state.career().write().map_err(|_| IpcError::LockPoisoned {
                lock: "career".to_string(),
            })?;

            // Career clock (T3-R-F): salience decay is projected to the current
            // career tick. Computed before the `&mut career.ledger` borrow below
            // so the shared `&career` borrow `current_tick()` needs does not
            // conflict with it.
            let now_tick = career.current_tick();
            let top_events: Vec<fw_memory::event::MemoryEvent> = SalienceReader::top_n(
                &mut career.ledger,
                5,
                SalienceFilter::BySubject(player_fw_id),
                now_tick,
            )
            .into_iter()
            .cloned()
            .collect();

            // Build ClubId → display_name map for participant resolution.
            // Keyed by ClubId (not by name string) to avoid silent dedup when two
            // procedurally-generated clubs happen to share a display_name.
            let club_names: std::collections::BTreeMap<fw_core::ClubId, String> = career
                .season
                .league
                .clubs
                .iter()
                .map(|c| (c.id, c.display_name.clone()))
                .collect();

            (top_events, club_names)
        }; // career write guard dropped here

        top_events
            .iter()
            .map(|event| {
                let disc = event.event_class.discriminant();

                // Guard: UnknownEventClass (discriminant 30) — no grammar family.
                // Fall back to a static phrase rather than propagating a render error.
                if discriminant_to_family_key(disc).is_none() {
                    return "an unusual moment in the career".to_string();
                }

                // Resolve the first Club participant by ClubId (not by name string).
                // An unresolved ClubId produces an empty slot — this is legitimate
                // for multi-season career references where a club no longer appears
                // in the current top-flight league snapshot; the grammar tolerates
                // empty context slots.
                let first_club_cid: Option<fw_core::ClubId> =
                    event.participants.iter().find_map(|p| {
                        if let fw_memory::event::EntityRef::Club(cid) = p.entity {
                            Some(cid)
                        } else {
                            None
                        }
                    });
                let club_name = first_club_cid
                    .and_then(|cid| club_names.get(&cid).map(|s| s.to_string()))
                    .unwrap_or_default();

                // Resolve opponent: the second Club participant with a different ClubId
                // than `first_club_cid` (dedup by ID, not by display string).
                let opponent_name = event
                    .participants
                    .iter()
                    .filter_map(|p| {
                        if let fw_memory::event::EntityRef::Club(cid) = p.entity {
                            if Some(cid) == first_club_cid {
                                return None;
                            }
                            club_names.get(&cid).map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .next()
                    .unwrap_or_default();

                let season_label = format!("Season {}", event.season.0 + 1);

                let ctx = MemoryCallbackContext {
                    // T4-2.5e: use the resolved display name (roster name when
                    // available) so the callback text matches the player's actual
                    // career identity, not a content-pool archetype stub.
                    player_name: player_display_name.clone(),
                    club_name,
                    opponent_name,
                    competition_name: String::new(),
                    season_label,
                    score_line: String::new(),
                    outcome_phrase: String::new(),
                    role_label: role_label_for_ctx.clone(),
                    detail_phrase: String::new(),
                };

                match render_memory_callback(career_seed, event.event_id.0, disc, &ctx, bank) {
                    Ok(s) => s,
                    // Render failure past the discriminant-30 guard is a genuine
                    // grammar-authoring bug (missing rule referenced from the family).
                    // Keep the static fallback so the UI degrades gracefully, but log
                    // the error so content authors see it immediately.
                    Err(e) => {
                        log::error!(
                            "memory-callback render failed for event {} disc {}: {}",
                            event.event_id.0,
                            disc,
                            e
                        );
                        "a notable moment in the career".to_string()
                    }
                }
            })
            .collect()
    } else {
        // numeric_player_id is None when parse_player_id_suffix found no suffix.
        // Both routing branches in get_player_detail_inner resolve a numeric id
        // before calling this helper, so None here is unreachable in practice.
        Vec::new()
    };

    Ok(PlayerDetailDto {
        phenotype,
        memory_callbacks,
        contract_status: None,
    })
}

/// `advance_season_inner` — complete the current season and advance to the next.
///
/// Atomically under one `career().write()` guard:
/// 1. Check `season.is_complete()` → early `Err(SeasonNotComplete)` before any mutation.
/// 2. Run the FALLIBLE step `generate_league(...)` FIRST, before any mutation, so a
///    failure leaves the career state unchanged. Maps failure to `IpcError::LeagueGenerationFailed`.
/// 3. Infallible mutations: emit season-end events, run per-player breakthrough
///    evaluation (T4-2.5d — pillar 3), compact if at the 5-season boundary,
///    swap the season, increment season_number. All under the same write guard.
pub fn advance_season_inner(state: &AppState) -> Result<AdvanceSeasonSummaryDto, IpcError> {
    // Run generate_league BEFORE acquiring the write lock — it's pure + fallible,
    // and we must not hold the career write lock across a potentially-expensive
    // call. If it fails we return an error without touching career state.
    let new_league = generate_league(state.career_seed(), state.content()).map_err(|e| {
        IpcError::LeagueGenerationFailed {
            reason: e.to_string(),
        }
    })?;

    let mut career = state.career().write().map_err(|_| IpcError::LockPoisoned {
        lock: "career".to_string(),
    })?;

    // Guard: require season complete before any mutation.
    if !career.season.is_complete() {
        return Err(IpcError::SeasonNotComplete);
    }

    // Collect champion data from standings (read-only; borrow ends before ledger write).
    let (champion_club_name, champion_club_id) = {
        let standings = career.season.standings();
        let first = standings.rows.first();
        let name = first.map(|r| r.club_name.clone()).unwrap_or_default();
        let id = first.map(|r| r.club_id);
        (name, id)
    };

    let current_season_num = career.season_number;

    // Emit season-end events into the ledger. Because `career.season` and
    // `career.ledger` are separate fields on the same struct, the borrow
    // checker requires we finish borrowing `career.season` before writing
    // to `career.ledger` via a function call (Rust cannot prove the function
    // touches disjoint fields). We pass only the derived champion_club_id
    // instead of `&career.season` to side-step the two-field borrow.
    match champion_club_id {
        Some(cid) => {
            season::emit_title_won_event(cid, current_season_num, &mut career.ledger);
        }
        None => {
            // Structurally unreachable: well-formed 20-club seasons always have a
            // champion (rows[0] is the title winner). If standings are ever empty
            // the TitleWon event silently vanishes from the ledger — that is a
            // load-bearing Pillar-2 event that must never fail silently.
            log::error!(
                "advance_season: standings empty for season {} — TitleWon event NOT emitted; \
                 investigate league setup or standings computation",
                current_season_num.0,
            );
        }
    }

    // ---- Pillar-3 (T4-2.5d): per-player breakthrough evaluation (incremental) ----
    //
    // INCREMENTAL DESIGN (P0 fix):
    //   Only the events appended SINCE the last evaluation pass
    //   (`career.breakthrough_eval_watermark..career.ledger.len()`) are fed to
    //   `evaluate()`. Historical events' meter contributions are already captured
    //   in each player's persisted `BreakthroughState`; re-accumulating them every
    //   season would cause the same gating event to re-fire a breakthrough in
    //   every subsequent season.
    //
    //   The watermark is advanced to `ledger.len()` AFTER the evaluation loop and
    //   AFTER breakthrough events are appended to the ledger — so the appended
    //   BreakthroughMoment events are included in the NEXT season's window (they
    //   are already processed from the perspective of the current season's state
    //   write-back, but they need to be visible as historical context for future
    //   seasons' cooldown checks in evaluate()).
    //
    // BORROW STRATEGY:
    //   We cannot borrow `career.ledger` and `career.roster` mutably at the same
    //   time through the `RwLockWriteGuard`. Two-phase approach:
    //   Phase 1: materialise `new_events` (owned Vec — no borrow held); iterate
    //            roster (shared borrow) to collect pending outcomes.
    //   Phase 2a: iterate `pending` to write back BreakthroughState + apply
    //             ceiling deltas (mutable roster borrow).
    //   Phase 2b: append breakthrough events to ledger (mutable ledger borrow;
    //             roster borrow is already released).

    let career_seed = state.career_seed().to_u64();
    let now_tick = Tick::ZERO; // career-system context; see CareerState::current_tick()

    // Materialise the new-event slice as an owned Vec so the shared borrow on
    // career.ledger is released before phase 2 (which needs &mut career.roster).
    let watermark = career.breakthrough_eval_watermark;
    let new_events: Vec<fw_memory::event::MemoryEvent> =
        career.ledger.iter().skip(watermark).cloned().collect();

    // Phase 1: for each rostered player, build a per-player view of the new
    // events and call evaluate() with the player's persisted BreakthroughState.
    // We always carry the mutated BreakthroughState back — even when no outcomes
    // fired — so meter accumulation is preserved for future seasons.
    //
    // NOTE: only PLAYER-SUBJECT events (those with ParticipantRole::Subject = Player)
    // are included in the per-player ledger. Club-level events such as TitleWon
    // (ParticipantRole::Beneficiary = Club) are NOT attributed to any individual
    // player and therefore do NOT accumulate breakthrough meters. Club-event gating
    // (associating TitleWon with all squad members as a shared career moment) is
    // a follow-up and is explicitly out of scope for T4-2.5d.
    let sig_defs = state.signature_definitions();
    let mut pending: Vec<(
        fw_core::ClubId,
        usize,
        fw_memory::BreakthroughState,
        Vec<BreakthroughOutcome>,
    )> = Vec::new();

    for (club_id, instances) in &career.roster {
        for (idx, inst) in instances.iter().enumerate() {
            // Filter new_events to those where this player is the Subject.
            // O(new_events) per player — much cheaper than a full-ledger clone
            // when the new-event slice is small (one season's worth of events).
            let player_new_ledger =
                season::filter_new_events_for_player(&new_events, inst.player_id);

            // Build BreakthroughContext from persisted genes + ceiling.
            let family_pa_ca = gene_family_pa_ca(&inst.genes, inst.ceiling);
            let narrative_flags: Vec<fw_memory::NarrativeFlag> = inst
                .genes
                .narrative_flags
                .iter()
                .map(|&f| season::content_flag_to_memory_flag(f))
                .collect();
            let sig_candidates =
                season::signature_candidates_to_ctx(&inst.signature_candidates, sig_defs);
            let career_date = fw_memory::event::CareerDate {
                year: current_season_num.0 + 1,
                day_of_year: 365,
            };

            let ctx = BreakthroughContext {
                player_id: inst.player_id,
                pa_by_family: family_pa_ca.pa,
                ca_by_family: family_pa_ca.ca,
                narrative_flags,
                signature_candidates: sig_candidates,
                age_years: season::CAREER_START_AGE_YEARS,
                career_date,
            };

            // Clone state; evaluate; always carry the mutated copy back so
            // readiness resets, pressure resets, and cooldown fire dates are
            // persisted across seasons — regardless of whether a breakthrough fired.
            let mut state_copy = inst.breakthrough_state.clone();
            let outcomes = evaluate(
                &player_new_ledger,
                &ctx,
                &mut state_copy,
                career_seed,
                now_tick,
            );

            pending.push((*club_id, idx, state_copy, outcomes));
        }
    }

    // Phase 2a: write back BreakthroughState and apply ceiling deltas.
    // (mutable borrow on career.roster; career.ledger not borrowed here)
    let mut breakthrough_events: Vec<fw_memory::event::MemoryEvent> = Vec::new();

    for (club_id, slot_idx, new_state, outcomes) in pending {
        let Some(instances) = career.roster.get_mut(&club_id) else {
            continue; // defensive; club must be present
        };
        let Some(inst) = instances.get_mut(slot_idx) else {
            continue; // defensive; slot must be present
        };
        inst.breakthrough_state = new_state;
        for outcome in outcomes {
            inst.ceiling
                .apply_breakthrough_delta(outcome.delta_pa, outcome.delta_ca);
            breakthrough_events.push(outcome.event);
        }
    }

    // Phase 2b: append breakthrough events to ledger; advance watermark.
    // career.roster is no longer borrowed (get_mut guards dropped at end of loop).
    for event in breakthrough_events {
        career.ledger.append(event);
    }
    // Advance the watermark to the current ledger length. This places the
    // BreakthroughMoment events we just appended BEHIND the new watermark, so the
    // next season's `evaluate()` does NOT re-process them as "new" events — which is
    // exactly the fire-exactly-once contract that stops the breakthrough meter
    // re-accumulating / the moment re-firing across season advances (pinned by the
    // QA-T4H watermark-advancement test). NOTE: an earlier comment here claimed the
    // opposite (that the appended events stay PAST the watermark) — that was wrong
    // and would invite a "fix" re-introducing the re-fire bug (ultra-review P2).
    career.breakthrough_eval_watermark = career.ledger.len();
    // ---- End pillar-3 breakthrough evaluation ----

    let new_season_num = SeasonNumber(current_season_num.0 + 1);
    // At or past the 5-season boundary: compact with the NEW season number
    // so the boundary condition (event.season + 5 <= current_season)
    // correctly identifies events from 5+ seasons ago.
    let compaction_fired = if new_season_num.0 >= 5 {
        career.ledger.compact(new_season_num) > 0
    } else {
        false
    };

    // ---- Pillar-2 (T4-2.5L D1): career-end RegressiveCollapse emission ----
    //
    // When the career reaches CAREER_END_SEASON, emit one RegressiveCollapse
    // for the most regressive-pressured roster player. This is a PLACEHOLDER
    // for the post-EA player-retirement / career-arc system (DECISIONS 2026-06-03
    // T4-2.5L D1); it makes the Pillar-2 lifecycle (debut → memory → decline)
    // provable end-to-end in tests without requiring a real retirement model.
    //
    // Two-phase borrow pattern (mirrors the breakthrough-eval borrow strategy
    // above): select the player id under an immutable roster borrow (phase 1),
    // then emit the event under a mutable ledger borrow (phase 2). The Rust
    // borrow checker cannot prove `career.roster` and `career.ledger` are
    // disjoint fields through the `RwLockWriteGuard` reference, so the two
    // borrows must be temporally sequential.
    if new_season_num.0 == season::CAREER_END_SEASON {
        // Phase 1: resolve player id (immutable borrow on career.roster).
        let collapse_player = season::select_career_end_collapse_player(&career.roster);
        // Phase 2: emit event (mutable borrow on career.ledger; roster borrow dropped).
        match collapse_player {
            Some(pid) => {
                season::emit_career_end_regressive_event(pid, new_season_num, &mut career.ledger);
            }
            // Unreachable in a well-formed career (roster generation always
            // populates every club). Fail LOUD in logs rather than silently
            // dropping the load-bearing career-end RegressiveCollapse — but do
            // NOT panic (Tauri/RULES §4: a command handler must not panic).
            None => {
                log::error!(
                    "career-end RegressiveCollapse skipped at season {}: roster is empty \
                     (malformed career — should be unreachable per roster generation)",
                    new_season_num.0
                );
            }
        }
    }
    // ---- End career-end emission ----

    // Per-season stats reset: season_stats is per-season (reset here);
    // career_apps is career-long (never reset).
    for instances in career.roster.values_mut() {
        for inst in instances.iter_mut() {
            inst.season_stats = fw_core::PlayerSeasonStats::default();
        }
    }

    // Swap in the fresh season and increment season_number — both under the
    // same write guard so the transition is atomic.
    career.season = SeasonState::new(new_league, state.content());
    career.season_number = new_season_num;

    Ok(AdvanceSeasonSummaryDto {
        completed_season: current_season_num.0,
        champion_club_name,
        new_season_number: new_season_num.0,
        compaction_fired,
    })
}

/// `get_career_overview_inner` — career overview DTO.
///
/// Reads the ledger's `TitleWon` events from past seasons (season <
/// current_season_number), resolves champion names from the current league's
/// club list, renders cross-season callbacks via `render_memory_callback`.
pub fn get_career_overview_inner(state: &AppState) -> Result<CareerOverviewDto, IpcError> {
    // Collect all career state under one read lock, then release before rendering.
    let (current_season_num, club_names, past_title_events, career_seed) = {
        let career = state.career().read().map_err(|_| IpcError::LockPoisoned {
            lock: "career".to_string(),
        })?;

        let current_season_num = career.season_number;

        let club_names: std::collections::BTreeMap<fw_core::ClubId, String> = career
            .season
            .league
            .clubs
            .iter()
            .map(|c| (c.id, c.display_name.clone()))
            .collect();

        let title_won_disc = EventClass::TitleWon.discriminant();
        let past_title_events: Vec<fw_memory::event::MemoryEvent> = career
            .ledger
            .iter()
            .filter(|e| {
                e.event_class.discriminant() == title_won_disc && e.season.0 < current_season_num.0
            })
            .cloned()
            .collect();

        // Read the seed UNDER the career guard so it pairs with the snapshot
        // above. new_career / load_career re-seed only while holding the career
        // WRITE lock, so a re-seed cannot interleave with this read block —
        // this is the invariant set_career_seed's doc relies on.
        let career_seed = state.career_seed().to_u64();

        (
            current_season_num,
            club_names,
            past_title_events,
            career_seed,
        )
    }; // career read guard dropped

    // Build champion history: one entry per season (ordered by season ASC).
    let mut history: Vec<ChampionHistoryEntryDto> = past_title_events
        .iter()
        .map(|event| {
            let champion_club_name = event
                .participants
                .iter()
                .find_map(|p| {
                    if let fw_memory::event::EntityRef::Club(cid) = p.entity {
                        club_names.get(&cid).cloned()
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    log::warn!(
                        "career overview: champion club for event {} (season {}) not found \
                         in current league snapshot — history entry will have blank name",
                        event.event_id.0,
                        event.season.0,
                    );
                    String::new()
                });
            ChampionHistoryEntryDto {
                season: event.season.0,
                champion_club_name,
            }
        })
        .collect();
    history.sort_by_key(|e| e.season);

    // Render cross-season callbacks via the T3-6 render_memory_callback path.
    // `career_seed` was captured under the career guard above (paired snapshot).
    let bank = &state.content().memory_callback_grammars;

    let cross_season_callbacks: Vec<String> = past_title_events
        .iter()
        .map(|event| {
            let disc = event.event_class.discriminant();
            if discriminant_to_family_key(disc).is_none() {
                log::error!(
                    "career overview: no grammar family for event {} disc {} — \
                     using static fallback",
                    event.event_id.0,
                    disc
                );
                return "an unusual moment in the career".to_string();
            }
            let first_club_cid: Option<fw_core::ClubId> = event.participants.iter().find_map(|p| {
                if let fw_memory::event::EntityRef::Club(cid) = p.entity {
                    Some(cid)
                } else {
                    None
                }
            });
            // Resolve the champion club's display name. For club-subject TitleWon
            // events the grammar variants reference #player_name# as the
            // subject (e.g. "#player_name# was part of it") — inject the club
            // name there so it reads "Northwood United was part of it".
            // If the club isn't in the current league snapshot, warn and fall
            // back to "the squad" so the sentence remains grammatical.
            let resolved_club_name: Option<String> =
                first_club_cid.and_then(|cid| club_names.get(&cid).cloned());
            let club_subject_name: String = match resolved_club_name {
                Some(ref name) => name.clone(),
                None => {
                    log::warn!(
                        "career overview: champion club for TitleWon event {} (season {}) \
                         not found in current league snapshot — using fallback subject",
                        event.event_id.0,
                        event.season.0,
                    );
                    "the squad".to_string()
                }
            };
            let club_name = resolved_club_name.unwrap_or_default();

            let season_label = format!("Season {}", event.season.0 + 1);
            let ctx = MemoryCallbackContext {
                // For club-subject TitleWon events, #player_name# is the club
                // (or "the squad" if unresolved) so grammar variants like
                // "#player_name# was part of it" produce a grammatical sentence.
                player_name: club_subject_name,
                club_name,
                opponent_name: String::new(),
                competition_name: String::new(),
                season_label,
                score_line: String::new(),
                outcome_phrase: String::new(),
                role_label: String::new(),
                detail_phrase: String::new(),
            };
            match render_memory_callback(career_seed, event.event_id.0, disc, &ctx, bank) {
                Ok(s) => s,
                Err(e) => {
                    log::error!(
                        "career overview: memory-callback render failed for event {} disc {}: {}",
                        event.event_id.0,
                        disc,
                        e
                    );
                    "a notable moment in the career".to_string()
                }
            }
        })
        .collect();

    Ok(CareerOverviewDto {
        season_number: current_season_num.0,
        history,
        cross_season_callbacks,
    })
}

/// `get_press_inbox_inner` — sync inner logic for `get_press_inbox`.
///
/// Lock acquisition strategy: `PressReader::candidates` takes `&mut MemoryLedger`
/// for lazy index rebuilds, so we need the WRITE lock. We collect all candidate
/// data as owned values and drop the write guard before the render loop, following
/// the same pattern as `get_player_detail_inner` / `get_career_overview_inner`.
///
/// ## Composition: top-K-per-topic merge
///
/// For each of the 4 `PressTopic` variants, take the top `PRESS_K_PER_TOPIC`
/// candidates (already ranked by projected salience desc inside each topic).
/// Merge across topics, dedup by `event_id` (first topic wins in declaration
/// order), re-sort the merged set by projected salience desc / event_id asc,
/// and cap at 20. This guarantees every non-empty topic is represented rather
/// than flooding the inbox with e.g. 440 `DebutSenior` events.
///
/// ## Name resolution
///
/// For each event the Subject `PlayerId` participant is resolved against
/// `career.roster` (flat scan) to populate `MemoryCallbackContext.player_name`.
/// Falls back to an empty string for club-subject events (e.g. `TitleWon`) —
/// the same behaviour as `get_career_overview_inner`.
///
/// ## Topic-string mapping (camelCase, matching TS union)
/// - `PressTopic::PlayerMilestone`  → `"playerMilestone"`
/// - `PressTopic::ContractTransfer` → `"contractTransfer"`
/// - `PressTopic::MatchResult`      → `"matchResult"`
/// - `PressTopic::Relational`       → `"relational"`
pub fn get_press_inbox_inner(state: &AppState) -> Result<PressInboxDto, IpcError> {
    /// Maximum candidates taken per topic before the cross-topic merge.
    /// 4 topics × 6 = 24 candidates before dedup + final cap of 20.
    const PRESS_K_PER_TOPIC: usize = 6;

    let all_topics: &[PressTopic] = &[
        PressTopic::PlayerMilestone,
        PressTopic::ContractTransfer,
        PressTopic::MatchResult,
        PressTopic::Relational,
    ];

    // Intermediate owned representation collected under the write lock so we
    // can drop the guard before the render loop.
    struct RawItem {
        event_id: u32,
        season: u16,
        event_class_disc: u32,
        topic: &'static str,
        // Salience at collection time (used for cross-topic merge sort).
        projected_salience: fw_core::Q32,
        // Context slots for render_memory_callback — mirroring get_player_detail_inner.
        player_name: String,
        club_name: String,
        opponent_name: String,
        season_label: String,
    }

    let (raw_items, season_number) = {
        let mut career = state.career().write().map_err(|_| IpcError::LockPoisoned {
            lock: "career".to_string(),
        })?;

        let now_tick = career.current_tick();
        let season_number = career.season_number;

        // Build ClubId → display_name for club/opponent name resolution.
        // Same pattern as get_player_detail_inner and get_career_overview_inner.
        let club_names: std::collections::BTreeMap<fw_core::ClubId, String> = career
            .season
            .league
            .clubs
            .iter()
            .map(|c| (c.id, c.display_name.clone()))
            .collect();

        // Build PlayerId → display_name for Subject participant resolution.
        // Flat scan of all roster instances (O(clubs × 22)); done once, not per event.
        // Keyed by PlayerId so we can look up the Subject of any player-subject event.
        let roster_names: std::collections::BTreeMap<PlayerId, String> = career
            .roster
            .values()
            .flat_map(|instances| instances.iter())
            .map(|inst| (inst.player_id, inst.display_name.clone()))
            .collect();

        // Top-K-per-topic merge: collect up to PRESS_K_PER_TOPIC candidates from
        // each topic, dedup by event_id keeping the FIRST topic assignment (topic
        // discriminant sets are disjoint, so dedup here only fires when the same
        // event_id is somehow indexed under two topics — a safety net, not a primary
        // path). BTreeMap for deterministic insertion order and O(log n) dedup.
        let mut seen: std::collections::BTreeMap<u32, RawItem> = std::collections::BTreeMap::new();

        for topic in all_topics {
            let topic_str = topic.as_dto_str();
            let candidates = PressReader::candidates(&mut career.ledger, *topic, now_tick);
            // Take only the top-K from this topic (PressReader already sorts salience desc).
            for event in candidates.into_iter().take(PRESS_K_PER_TOPIC) {
                let eid = event.event_id.0;
                if seen.contains_key(&eid) {
                    continue; // dedup: keep first topic's assignment
                }

                let projected_salience = project_salience(event, now_tick);

                // Resolve Subject participant's display name from the roster.
                // Mirrors the get_player_detail_inner / build_player_detail_dto pattern:
                // look up by PlayerId in career.roster; fall back to empty string for
                // club-subject events (TitleWon) where there is no Player Subject.
                let player_name: String = event
                    .participants
                    .iter()
                    .find_map(|p| {
                        if p.role == fw_memory::event::ParticipantRole::Subject
                            && let fw_memory::event::EntityRef::Player(pid) = p.entity
                        {
                            return roster_names.get(&pid).cloned();
                        }
                        None
                    })
                    .unwrap_or_default();

                // Resolve club/opponent names — same pattern as get_career_overview_inner.
                let first_club_cid: Option<fw_core::ClubId> =
                    event.participants.iter().find_map(|p| {
                        if let fw_memory::event::EntityRef::Club(cid) = p.entity {
                            Some(cid)
                        } else {
                            None
                        }
                    });
                let club_name = first_club_cid
                    .and_then(|cid| club_names.get(&cid).map(|s| s.to_string()))
                    .unwrap_or_default();

                let opponent_name = event
                    .participants
                    .iter()
                    .filter_map(|p| {
                        if let fw_memory::event::EntityRef::Club(cid) = p.entity {
                            if Some(cid) == first_club_cid {
                                return None;
                            }
                            club_names.get(&cid).map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .next()
                    .unwrap_or_default();

                let season_label = format!("Season {}", event.season.0 + 1);

                seen.insert(
                    eid,
                    RawItem {
                        event_id: eid,
                        season: event.season.0,
                        event_class_disc: event.event_class.discriminant(),
                        topic: topic_str,
                        projected_salience,
                        player_name,
                        club_name,
                        opponent_name,
                        season_label,
                    },
                );
            }
        }

        (seen, season_number)
    }; // career write guard dropped here

    // Merge, re-sort by projected_salience desc + event_id asc, cap at 20.
    // The per-topic pre-sort guarantees each topic's best items entered `seen`
    // first; this final sort produces the cross-topic ranking.
    let mut sorted: Vec<RawItem> = raw_items.into_values().collect();
    sorted.sort_by(|a, b| {
        b.projected_salience
            .cmp(&a.projected_salience)
            .then_with(|| a.event_id.cmp(&b.event_id))
    });
    sorted.truncate(20);

    // Render each headline via render_memory_callback (no career lock held).
    let bank = &state.content().memory_callback_grammars;
    let career_seed = state.career_seed().to_u64();

    let mut items: Vec<PressItemDto> = Vec::with_capacity(sorted.len());
    for raw in sorted {
        let disc = raw.event_class_disc;

        let headline = if discriminant_to_family_key(disc).is_none() {
            // No grammar family for this discriminant — static fallback so the
            // UI degrades gracefully without surfacing a raw discriminant number.
            log::error!(
                "press inbox: no grammar family for event {} disc {} — using static fallback",
                raw.event_id,
                disc,
            );
            "a notable moment in the career".to_string()
        } else {
            let ctx = MemoryCallbackContext {
                // player_name resolved from roster above; empty for club-subject
                // events (TitleWon) — same fallback as get_career_overview_inner.
                player_name: raw.player_name.clone(),
                club_name: raw.club_name.clone(),
                opponent_name: raw.opponent_name.clone(),
                competition_name: String::new(),
                season_label: raw.season_label.clone(),
                score_line: String::new(),
                outcome_phrase: String::new(),
                role_label: String::new(),
                detail_phrase: String::new(),
            };
            match render_memory_callback(career_seed, raw.event_id, disc, &ctx, bank) {
                Ok(s) => s,
                Err(e) => {
                    log::error!(
                        "press inbox: memory-callback render failed for event {} disc {}: {}",
                        raw.event_id,
                        disc,
                        e
                    );
                    "a notable moment in the career".to_string()
                }
            }
        };

        items.push(PressItemDto {
            event_id: raw.event_id,
            season: raw.season,
            event_class: disc,
            topic: raw.topic.to_string(),
            headline,
            manager_quote: None,
        });
    }

    Ok(PressInboxDto {
        items,
        season_number: season_number.0,
    })
}

// ---------------------------------------------------------------------------
// Live-match inner helpers (T4-5a)
// ---------------------------------------------------------------------------

pub fn start_live_match_inner(seed_hex: String, state: &AppState) -> Result<MatchHandle, IpcError> {
    let seed = parse_seed_hex(&seed_hex)?;
    let sim_state = MatchState::initial_with_content(
        seed,
        state.content(),
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
    )?;

    let id = state.alloc_live_match_id();
    let handle = MatchHandle {
        id,
        seed_hex: seed_hex.clone(),
    };
    let session = LiveMatchSession::new(id, seed.to_u64(), seed_hex, sim_state);

    state
        .live_matches()
        .write()
        .map_err(|_| IpcError::LockPoisoned {
            lock: "live_matches".to_string(),
        })?
        .insert(id, session);

    Ok(handle)
}

/// Start a live-match session for a specific real fixture, constructing the
/// `MatchState` identically to how `advance_week_inner` constructs it for the
/// AI-sim path.
///
/// The construction uses the same three inputs that `advance_week_inner` uses:
///
/// 1. `fixture_seed(career_seed, fixture_index)` — the per-fixture seed derived
///    from the career seed and the fixture's position in `league.fixtures`.
/// 2. Per-club tactical archetype IDs from `career.season.tactical_archetype_ids`.
/// 3. Per-club slot-signature overrides from `career.roster` (via
///    `season::build_slot_signatures`), applied via `MatchState::with_slot_signatures`.
///
/// When no in-match decisions are made, stepping this session to completion MUST
/// yield the same final score and canonical `MatchState` as `advance_week_inner`
/// would for that fixture. The determinism-equivalence test in this module
/// (`live_fixture_determinism_matches_ai_sim`) verifies this invariant with a
/// full-season sim + watched-replay comparison.
///
/// The returned `MatchHandle` is accepted by all five live-match commands
/// (`step_live_match`, `get_match_snapshot`, `apply_match_command`,
/// `finish_live_match`). The session's `seed_hex` echoes the fixture seed in
/// `"0x…"` form for replay traceability.
///
/// ## Errors
///
/// - `IpcError::LockPoisoned { lock: "career" }` if the career read lock is poisoned.
/// - `IpcError::ClubNotFound { club_id }` if either `home_club_id` or `away_club_id`
///   is not present in `league.clubs`.
/// - `IpcError::LeagueGenerationFailed` if the fixture formed by `(home, away)` is not
///   found in `league.fixtures` (indicates a caller/logic bug — only valid current-
///   season fixtures should be passed).
/// - `IpcError::MatchInitFailed` if `MatchState::initial_with_content` rejects the
///   archetype IDs (content-pack integrity violation).
/// - `IpcError::LockPoisoned { lock: "live_matches" }` if the live-match write lock
///   is poisoned.
pub fn start_live_match_for_fixture_inner(
    home_club_id: u32,
    away_club_id: u32,
    state: &AppState,
) -> Result<MatchHandle, IpcError> {
    let home = ClubId::new(home_club_id);
    let away = ClubId::new(away_club_id);

    // Acquire career read lock — we only read fixture metadata and roster, no mutation.
    let career = state.career().read().map_err(|_| IpcError::LockPoisoned {
        lock: "career".to_string(),
    })?;

    // Validate both clubs exist in the current league.
    let _ = career
        .season
        .league
        .clubs
        .iter()
        .find(|c| c.id == home)
        .ok_or(IpcError::ClubNotFound {
            club_id: home_club_id,
        })?;
    let _ = career
        .season
        .league
        .clubs
        .iter()
        .find(|c| c.id == away)
        .ok_or(IpcError::ClubNotFound {
            club_id: away_club_id,
        })?;

    // Build a synthetic Fixture key to look up the index in league.fixtures.
    // `league.fixtures` is sorted by (match_day, home_id, away_id) — the sort
    // order means any (home, away) pair appears at most once per match_day, and
    // at most once total across the season. We search for any fixture where
    // home==home and away==away, regardless of match_day, because the caller
    // supplies the clubs directly (not the match_day).
    let fixture_idx = career
        .season
        .league
        .fixtures
        .iter()
        .position(|f| f.home == home && f.away == away)
        .ok_or_else(|| IpcError::LeagueGenerationFailed {
            reason: format!(
                "no fixture found for home={} away={} in league.fixtures \
                 (caller supplied clubs not in the current fixture list)",
                home_club_id, away_club_id,
            ),
        })? as u32;

    let career_seed = state.career_seed();
    let seed = fixture_seed(career_seed, fixture_idx);

    // Resolve per-club archetype IDs — same look-up as advance_week_inner.
    let home_arch = career
        .season
        .tactical_archetype_ids
        .get(&home)
        .cloned()
        .ok_or_else(|| IpcError::LeagueGenerationFailed {
            reason: format!(
                "no tactical archetype for home club {} (generate_league invariant)",
                home_club_id
            ),
        })?;
    let away_arch = career
        .season
        .tactical_archetype_ids
        .get(&away)
        .cloned()
        .ok_or_else(|| IpcError::LeagueGenerationFailed {
            reason: format!(
                "no tactical archetype for away club {} (generate_league invariant)",
                away_club_id
            ),
        })?;

    // Build slot-signature overrides — same path as advance_week_inner.
    let slot_signatures = {
        let home_instances = career.roster.get(&home);
        let away_instances = career.roster.get(&away);
        match (home_instances, away_instances) {
            (Some(home_vec), Some(away_vec)) => Some(season::build_slot_signatures(
                home_vec.as_slice(),
                away_vec.as_slice(),
            )),
            _ => None,
        }
    };

    // Construct MatchState identically to `season::play_one_match`.
    let base_state =
        MatchState::initial_with_content(seed, state.content(), &home_arch, &away_arch).map_err(
            |e| IpcError::MatchInitFailed {
                reason: e.to_string(),
            },
        )?;

    let sim_state = if let Some(overrides) = slot_signatures {
        base_state.with_slot_signatures(overrides)
    } else {
        base_state
    };

    // Drop the career read lock before taking the live_matches write lock.
    drop(career);

    let seed_hex = format!("0x{:016x}", seed.to_u64());
    let id = state.alloc_live_match_id();
    let handle = MatchHandle {
        id,
        seed_hex: seed_hex.clone(),
    };
    let session = LiveMatchSession::new(id, seed.to_u64(), seed_hex, sim_state);

    state
        .live_matches()
        .write()
        .map_err(|_| IpcError::LockPoisoned {
            lock: "live_matches".to_string(),
        })?
        .insert(id, session);

    Ok(handle)
}

pub fn step_live_match_inner(
    handle: MatchHandle,
    ticks: u32,
    state: &AppState,
) -> Result<StepResult, IpcError> {
    if ticks > MAX_FRAMES_PER_REQUEST {
        return Err(IpcError::TooManyFrames {
            requested: ticks,
            max: MAX_FRAMES_PER_REQUEST,
        });
    }

    let mut live = state
        .live_matches()
        .write()
        .map_err(|_| IpcError::LockPoisoned {
            lock: "live_matches".to_string(),
        })?;

    let session = live
        .get_mut(&handle.id)
        .ok_or_else(|| IpcError::MatchInitFailed {
            reason: format!("unknown live-match handle id={}", handle.id),
        })?;

    // `state.match_events()` is the single append-only source (canonical state,
    // `fw-match-sim` encoder VERSION 7); record its length before the loop to
    // slice the per-step delta after. No separate session-side event mirror.
    //
    // Note: the sim now self-halts at FullTime (T4-sim-halt): `tick_match`
    // returns state unchanged once FullTime is the tail event. Extra ticks
    // beyond `match_end_tick` are no-ops. The `.any(... FullTime ...)` check
    // in `session.is_finished()` is still correct — FullTime is now always
    // the tail, so the scan finds it there.
    let events_before = session.state.match_events().len();

    for _ in 0..ticks {
        // Snapshot possession before tick for the tally update.
        let possession_before = session.state.possession();

        session.state = tick_match(session.state.clone(), state.signature_definitions());

        // Update possession tally.
        let possession_after = session.state.possession();
        if let Some(slot) = possession_after {
            // Tally for the slot that HAD possession at end of tick.
            // slot < PLAYERS_PER_TEAM = home; else away.
            let team = if (slot as usize) < PLAYERS_PER_TEAM {
                0
            } else {
                1
            };
            session.possession_ticks[team] = session.possession_ticks[team].saturating_add(1);
        } else if let Some(slot) = possession_before {
            // Ball was released this tick — credit the team that had it at start.
            let team = if (slot as usize) < PLAYERS_PER_TEAM {
                0
            } else {
                1
            };
            session.possession_ticks[team] = session.possession_ticks[team].saturating_add(1);
        }
    }

    let new_events: Vec<crate::result::MatchEventDto> = session.state.match_events()
        [events_before..]
        .iter()
        .map(crate::result::MatchEventDto::from_match_event)
        .collect();

    let result = StepResult {
        handle,
        new_events,
        score: crate::live_match::types::ScoreDto {
            home: session.state.home_score,
            away: session.state.away_score,
        },
        tick: session.state.tick.to_raw().max(0) as u32,
        is_finished: session.is_finished(),
        // Project the live session's current MatchState into a position frame
        // so the frontend 2D board can render from the live session without
        // re-simming independently. Reuses the same MatchFrameDto::from_state
        // projection as match_frames (Tauri/RULES §3: one-way read projection).
        frame: MatchFrameDto::from_state(&session.state),
    };

    Ok(result)
}

pub fn get_match_snapshot_inner(
    handle: MatchHandle,
    state: &AppState,
) -> Result<MatchSnapshot, IpcError> {
    let live = state
        .live_matches()
        .read()
        .map_err(|_| IpcError::LockPoisoned {
            lock: "live_matches".to_string(),
        })?;

    let session = live
        .get(&handle.id)
        .ok_or_else(|| IpcError::MatchInitFailed {
            reason: format!("unknown live-match handle id={}", handle.id),
        })?;

    let echo_handle = MatchHandle {
        id: handle.id,
        seed_hex: session.seed_hex.clone(),
    };
    Ok(project_snapshot(session, echo_handle))
}

pub fn finish_live_match_inner(
    handle: MatchHandle,
    state: &AppState,
) -> Result<FinalMatchResult, IpcError> {
    let mut live = state
        .live_matches()
        .write()
        .map_err(|_| IpcError::LockPoisoned {
            lock: "live_matches".to_string(),
        })?;

    let session = live
        .remove(&handle.id)
        .ok_or_else(|| IpcError::MatchInitFailed {
            reason: format!("unknown live-match handle id={}", handle.id),
        })?;

    let echo_handle = MatchHandle {
        id: handle.id,
        seed_hex: session.seed_hex.clone(),
    };
    Ok(project_final(&session, echo_handle))
}

pub fn apply_match_command_inner(
    handle: MatchHandle,
    command: MatchCommand,
    state: &AppState,
) -> Result<(), IpcError> {
    let mut live = state
        .live_matches()
        .write()
        .map_err(|_| IpcError::LockPoisoned {
            lock: "live_matches".to_string(),
        })?;

    let session = live
        .get_mut(&handle.id)
        .ok_or_else(|| IpcError::MatchInitFailed {
            reason: format!("unknown live-match handle id={}", handle.id),
        })?;

    let kind = command.kind_str().to_string();
    // Record for audit trail before returning the error.
    session.pending_commands.push(command);

    Err(IpcError::LiveMatchCommandUnimplemented { command_kind: kind })
}

// ---------------------------------------------------------------------------
// Settings commands (T4-6a)
// ---------------------------------------------------------------------------

/// `get_settings()` — read persisted app settings.
///
/// Returns `AppSettingsDto` with the current settings. If the settings file
/// is **absent** (first-run), returns `SettingsV0::default()` projected to the
/// DTO — NOT an error. A missing file is normal; a corrupt file IS an error.
#[tauri::command]
pub async fn get_settings(
    state: tauri::State<'_, AppState>,
) -> Result<crate::AppSettingsDto, IpcError> {
    get_settings_inner(&state)
}

/// `set_settings(settings)` — persist app settings.
///
/// Validates the DTO, encodes to `SettingsEnvelope::V0`, and writes to the
/// settings file. The write is non-atomic at T4-6a (a plain overwrite); a
/// write-temp-rename pattern can be added at T4-6b if needed.
#[tauri::command]
pub async fn set_settings(
    settings: crate::AppSettingsDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), IpcError> {
    set_settings_inner(settings, &state)
}

pub fn get_settings_inner(state: &AppState) -> Result<crate::AppSettingsDto, IpcError> {
    let path = state.settings_path();

    // First-run: missing file → return defaults, NOT an error.
    // Drop the TOCTOU exists()+read pattern: attempt fs::read directly and
    // treat NotFound as the first-run case. Any other I/O error is a real failure.
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(crate::AppSettingsDto::from_settings_v0(
                fw_save::SettingsV0::default(),
            ));
        }
        Err(e) => {
            return Err(IpcError::SettingsLoadFailed {
                reason: e.to_string(),
            });
        }
    };

    let v0 = fw_save::load_settings_envelope(&bytes).map_err(|e| IpcError::SettingsLoadFailed {
        reason: e.to_string(),
    })?;

    Ok(crate::AppSettingsDto::from_settings_v0(v0))
}

pub fn set_settings_inner(
    settings: crate::AppSettingsDto,
    state: &AppState,
) -> Result<(), IpcError> {
    let v0 = settings.to_settings_v0();
    let envelope = fw_save::SettingsEnvelope::V0(v0);
    let bytes = fw_save::encode_settings(&envelope).map_err(|e| IpcError::SettingsLoadFailed {
        reason: e.to_string(),
    })?;

    let target = state.settings_path();

    // Ensure parent directory exists (first write on a fresh install).
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| IpcError::SettingsLoadFailed {
            reason: "settings path has no parent directory".to_string(),
        })?;
    std::fs::create_dir_all(parent).map_err(|e| IpcError::SettingsLoadFailed {
        reason: format!("could not create settings directory: {e}"),
    })?;

    // Atomic write: write to a temp file in the same directory, then rename
    // over the target. std::fs::rename on the same filesystem is atomic, so a
    // crash mid-write cannot truncate the existing settings file.
    let tmp_path = parent.join(".settings_tmp.fwcfg");
    std::fs::write(&tmp_path, &bytes).map_err(|e| IpcError::SettingsLoadFailed {
        reason: format!("could not write settings temp file: {e}"),
    })?;
    std::fs::rename(&tmp_path, target).map_err(|e| IpcError::SettingsLoadFailed {
        reason: format!("could not rename settings temp file to target: {e}"),
    })?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Career save / load commands (T4-2.5g)
// ---------------------------------------------------------------------------

/// `save_career()` — persist the current career state to disk as a `SaveV4`.
///
/// Projects each `PlayerInstance` in `CareerState.roster` to a
/// `SavedPlayerInstance` (mutable-delta subset only: ceiling,
/// breakthrough_state, season_stats, career_apps, observation_count), builds a
/// `SaveEnvelope::V4`, and writes it to `AppState.career_save_path`.
///
/// The write is non-atomic (plain overwrite) at T4-2.5g — a write-temp-rename
/// pattern can be added at a later task if needed.
#[tauri::command]
pub async fn save_career(state: tauri::State<'_, AppState>) -> Result<(), IpcError> {
    save_career_inner(&state)
}

/// `load_career()` — load the career save from disk and reconstruct the full
/// `CareerState`.
///
/// 1. Reads `career_save_path` → `load_envelope` → `SaveV4`.
/// 2. Regenerates the full base roster deterministically from `career_seed`
///    via `generate_league_with_teams` + `build_roster_from_league`.
/// 3. If `save.roster` is non-empty, overlays each `SavedPlayerInstance` onto
///    the matching base instance by `player_id` — applies ceiling,
///    breakthrough_state, season_stats, career_apps, observation_count.
/// 4. Reconstructs and writes a fresh `CareerState` into `AppState.career`.
///    If `save.season` is `None` (migrated from <V4), a fresh `SeasonState`
///    is regenerated from the career seed (same as `AppState::new_with_settings_path`).
#[tauri::command]
pub async fn load_career(state: tauri::State<'_, AppState>) -> Result<(), IpcError> {
    load_career_inner(&state)
}

pub fn save_career_inner(state: &AppState) -> Result<(), IpcError> {
    let career = state.career().read().map_err(|_| IpcError::LockPoisoned {
        lock: "career".to_string(),
    })?;

    // Project the roster: PlayerInstance → SavedPlayerInstance (delta fields only).
    let mut roster: BTreeMap<ClubId, Vec<SavedPlayerInstance>> = BTreeMap::new();
    for (club_id, instances) in &career.roster {
        let saved: Vec<SavedPlayerInstance> = instances
            .iter()
            .map(|pi| SavedPlayerInstance {
                player_id: pi.player_id,
                club_id: pi.club_id,
                slot: pi.slot,
                ceiling: pi.ceiling,
                breakthrough_state: pi.breakthrough_state.clone(),
                season_stats: pi.season_stats.clone(),
                career_apps: pi.career_apps,
                observation_count: pi.observation_count,
            })
            .collect();
        roster.insert(*club_id, saved);
    }

    let save = SaveV4 {
        career_seed: state.career_seed(),
        content_pack_version: 1,
        ledger: career.ledger.clone(),
        season_number: career.season_number,
        season: Some(career.season.clone()),
        roster,
        // in-memory usize → fixed-width u64 wire field (lossless: a ledger length).
        breakthrough_eval_watermark: career.breakthrough_eval_watermark as u64,
    };

    let envelope = SaveEnvelope::V4(save);
    let bytes = fw_save::encode(&envelope).map_err(|e| IpcError::SaveLoadFailed {
        reason: e.to_string(),
    })?;

    let target = state.career_save_path();

    // Ensure parent directory exists (first write on a fresh install).
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| IpcError::SaveLoadFailed {
            reason: "career save path has no parent directory".to_string(),
        })?;
    std::fs::create_dir_all(parent).map_err(|e| IpcError::SaveLoadFailed {
        reason: format!("could not create career save directory: {e}"),
    })?;

    // Atomic write: write to a temp file in the same directory, then rename
    // over the target. std::fs::rename on the same filesystem is atomic, so a
    // crash mid-write cannot truncate the existing save file.
    let tmp_path = parent.join(".career_tmp.fwsave");
    std::fs::write(&tmp_path, &bytes).map_err(|e| IpcError::SaveLoadFailed {
        reason: format!("could not write career temp file: {e}"),
    })?;
    std::fs::rename(&tmp_path, target).map_err(|e| IpcError::SaveLoadFailed {
        reason: format!("could not rename career temp file to target: {e}"),
    })?;

    Ok(())
}

pub fn load_career_inner(state: &AppState) -> Result<(), IpcError> {
    let bytes = std::fs::read(state.career_save_path()).map_err(|e| IpcError::SaveLoadFailed {
        reason: e.to_string(),
    })?;

    let mut save = fw_save::load_envelope(&bytes).map_err(|e| IpcError::SaveLoadFailed {
        reason: e.to_string(),
    })?;

    // restore_transient_state is already called by load_envelope; the ledger is ready.

    // Step 1: Regenerate the full base roster deterministically from career_seed.
    let (league, procgen_teams) = generate_league_with_teams(save.career_seed, state.content())
        .map_err(|e| IpcError::LeagueGenerationFailed {
            reason: e.to_string(),
        })?;

    let mut base_roster =
        crate::roster::build_roster_from_league(&league, &procgen_teams, state.content()).map_err(
            |e| IpcError::LeagueGenerationFailed {
                reason: e.to_string(),
            },
        )?;

    // Step 2: Overlay saved deltas onto the base roster (if any).
    // For each SavedPlayerInstance: find the matching base instance by player_id
    // and overwrite the mutable-delta fields. Unmatched clubs/players (e.g. the
    // content pack changed between sessions, shifting the regenerated base) are
    // skipped GRACEFULLY — a hard error would block the whole save from loading
    // — but NOT silently: we count them and log::warn! so a vanished-progression
    // report is one greppable line, not a multi-hour mystery. (A proper
    // content-pack mismatch guard is the deferred mod_load_fingerprint check,
    // Content/RULES §6.) Self-review P1 (T4-2.5g silent-failure + type-design).
    let mut unmatched_clubs = 0usize;
    let mut unmatched_players = 0usize;
    for (club_id, saved_instances) in &save.roster {
        let Some(base_instances) = base_roster.get_mut(club_id) else {
            unmatched_clubs += 1;
            unmatched_players += saved_instances.len();
            continue;
        };
        for saved in saved_instances {
            // The saved row's club_id must agree with its BTreeMap key — a
            // mismatch means a corrupted/hand-edited save. This is save-bytes-derived
            // data, so we return a graceful error rather than panicking
            // (Tauri/RULES.md §4: never panic in a handler).
            if saved.club_id != *club_id {
                return Err(IpcError::SaveLoadFailed {
                    reason: format!(
                        "SavedPlayerInstance.club_id {:?} disagrees with its roster map key {:?} \
                         — save file may be corrupted or hand-edited",
                        saved.club_id, club_id
                    ),
                });
            }
            // Linear scan: 22 slots per club — negligible.
            match base_instances
                .iter_mut()
                .find(|pi| pi.player_id == saved.player_id)
            {
                Some(base) => {
                    base.ceiling = saved.ceiling;
                    base.breakthrough_state = saved.breakthrough_state.clone();
                    base.season_stats = saved.season_stats.clone();
                    base.career_apps = saved.career_apps;
                    base.observation_count = saved.observation_count;
                }
                None => unmatched_players += 1,
            }
        }
    }
    if unmatched_players > 0 {
        log::warn!(
            "load_career: {unmatched_players} saved player delta(s) across {unmatched_clubs} \
             unmatched club(s) could not be applied — the base roster regenerated from career \
             seed {:?} does not contain these ids (content pack changed since save?). Affected \
             players were reset to base progression.",
            save.career_seed,
        );
    }

    // Step 2b: Re-derive last_scout_report for any reloaded player observed in a
    // prior session. SaveV4 persists observation_count but NOT the report itself
    // (it is a deterministic projection of the career seed — see SavedPlayerInstance).
    // The overlay above restored observation_count but left last_scout_report = None
    // (build_roster_from_league's default), so without this pass get_scout_report
    // returns NotYetObserved for every reloaded scouted player (ultra-review P1-5).
    // observe_player is deterministic, so replaying the LAST observation
    // (id = observation_count - 1 — live play uses the pre-increment id then bumps,
    // season.rs:757-764) reproduces the exact report cached live. The bio pool +
    // scout must match the live observe_match_participants derivation byte-for-byte.
    {
        use crate::roster::ROSTER_PLAYER_ID_BASE;
        let bios: Vec<&fw_content::PlayerBio> = state.content().player_bios.values().collect();
        if !bios.is_empty() {
            let scout = fw_scouting::Scout::basic_uncertainty();
            let career_seed_u64 = save.career_seed.to_u64();
            for instances in base_roster.values_mut() {
                for inst in instances.iter_mut() {
                    if inst.observation_count == 0 {
                        continue;
                    }
                    let global_idx = (inst.player_id.raw() - ROSTER_PLAYER_ID_BASE) as usize;
                    let bio = bios[global_idx % bios.len()];
                    // Same fail-loud gene-snapshot invariant the live derivation
                    // enforces (season.rs) — fires if the round-robin formula drifts.
                    assert!(
                        bio.internal_gene_snapshot == inst.genes,
                        "load_career re-derive: gene snapshot mismatch for player {:?} \
                         (global_idx={global_idx}) — the round-robin bio formula drifted \
                         from build_roster_from_league; this is a programming error.",
                        inst.player_id,
                    );
                    // Pre-increment id of the last live observation.
                    let last_obs_id = inst.observation_count - 1;
                    inst.last_scout_report = Some(fw_scouting::observe_player(
                        &scout,
                        bio,
                        career_seed_u64,
                        last_obs_id,
                        inst.player_id,
                    ));
                }
            }
        } else {
            // Empty bio pool: a consistent empty-pool career never increments
            // observation_count (observe_match_participants early-returns before the
            // bump), so nothing is lost here normally. But a save authored against a
            // pack WITH bios, reloaded against one WITHOUT them (mod removal / pack
            // downgrade), would keep observation_count>0 with no bio to re-derive from
            // — convert that silent inconsistency into a diagnosable warning rather
            // than leaving a reloaded player reading NotYetObserved with no clue why.
            let observed = base_roster
                .values()
                .flatten()
                .filter(|i| i.observation_count > 0)
                .count();
            if observed > 0 {
                log::warn!(
                    "load_career: {observed} reloaded player(s) have observation_count>0 but the \
                     active content pack ships no player bios — last_scout_report could not be \
                     re-derived (content-pack/mod downgrade since save?). These players read \
                     NotYetObserved until the next match-day re-observes them."
                );
            }
        }
    }

    // Step 3: Reconstruct the season. If None (migrated from V2/V3), regenerate.
    let season = match save.season.take() {
        Some(s) => s,
        None => {
            let (fresh_league, _) = generate_league_with_teams(save.career_seed, state.content())
                .map_err(|e| IpcError::LeagueGenerationFailed {
                reason: e.to_string(),
            })?;
            SeasonState::new(fresh_league, state.content())
        }
    };

    // Step 4: Write the reconstructed CareerState.
    let mut career = state.career().write().map_err(|_| IpcError::LockPoisoned {
        lock: "career".to_string(),
    })?;

    career.season = season;
    career.ledger = save.ledger;
    career.season_number = save.season_number;
    career.roster = base_roster;
    // Wire field is u64 (pointer-width-independent); the in-memory watermark is
    // usize. The value is a ledger length, never near u32::MAX, so the cast is lossless.
    career.breakthrough_eval_watermark = save.breakthrough_eval_watermark as usize;

    // Re-seed AppState to the loaded save's seed (while still holding the
    // career write lock so the seed + season stay paired). Pre-new_career this
    // was a no-op because every career used DEFAULT_CAREER_SEED; once
    // new_career lets the user choose a seed, a save from a different-seed
    // career MUST update the seed or fixture derivation (fixture_seed) would
    // use the constructor's seed and desync from the loaded world.
    state.set_career_seed(save.career_seed);

    // Loading a (possibly different-world) save invalidates any session-managed
    // club from the prior world. Clear it under the career write lock (same
    // atomic-reset reasoning as new_career) so get_squad_roster starts
    // unanchored rather than silently anchoring on a stale, positional ClubId
    // that happens to exist in the loaded world.
    *state
        .managed_club_id()
        .write()
        .map_err(|_| IpcError::LockPoisoned {
            lock: "managed_club_id".to_string(),
        })? = None;

    Ok(())
}

// -------------------------------------------------------------------------
// Career lifecycle: new_career / get_clubs / select_managed_club (B1-B3)
//
// These make the career loop *startable* and *anchored to a club*. The world
// seed was previously hardcoded (DEFAULT_CAREER_SEED) and the Squad screen
// fell back to the lowest-ClubId placeholder. `new_career` re-seeds the
// AppState career in place (the Tauri-managed state is shared immutably, so
// the career is reset under the existing `career` write lock + the seed
// atomic). `managed_club_id` is session-only (NOT persisted by SaveV4) —
// cross-save persistence is a flagged SaveV5 owner decision.
// -------------------------------------------------------------------------

/// `new_career(seed_hex)` — start a fresh career world from a chosen seed.
///
/// Regenerates the league + roster + season deterministically from `seed_hex`
/// and replaces the in-memory career. Clears any previously-selected managed
/// club (the player picks one via `select_managed_club`). Does NOT touch the
/// save file — call `save_career` to persist.
#[tauri::command]
pub async fn new_career(
    seed_hex: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), IpcError> {
    new_career_inner(&seed_hex, &state)
}

pub fn new_career_inner(seed_hex: &str, state: &AppState) -> Result<(), IpcError> {
    let seed = parse_seed_hex(seed_hex)?;

    // Regenerate the base world from the new seed (mirrors load_career_inner
    // step 1, minus the saved-delta overlay — a fresh career has no deltas).
    let (league, procgen_teams) =
        generate_league_with_teams(seed, state.content()).map_err(|e| {
            IpcError::LeagueGenerationFailed {
                reason: e.to_string(),
            }
        })?;
    let roster = crate::roster::build_roster_from_league(&league, &procgen_teams, state.content())
        .map_err(|e| IpcError::LeagueGenerationFailed {
            reason: e.to_string(),
        })?;
    let season = SeasonState::new(league, state.content());

    // Swap the career under the write lock, then re-seed while still holding
    // it: this keeps a concurrent reader from pairing the new season with the
    // old seed. new_career is a rare, user-initiated, effectively-exclusive
    // operation, so the race window is theoretical — this just minimises it.
    {
        let mut career = state.career().write().map_err(|_| IpcError::LockPoisoned {
            lock: "career".to_string(),
        })?;
        career.season = season;
        career.ledger = fw_memory::ledger::MemoryLedger::new();
        career.season_number = SeasonNumber(0);
        career.roster = roster;
        career.breakthrough_eval_watermark = 0;
        state.set_career_seed(seed);
        // Clear the managed club WHILE holding the career write lock so the
        // reset is atomic w.r.t. get_squad_roster (which reads career then
        // managed_club_id — same career-outer/managed-inner lock order, no
        // deadlock). Clearing it after releasing the lock would let a
        // concurrent read pair the NEW roster with the OLD managed club; since
        // ClubIds are positional (not seed-derived), that can silently anchor
        // a same-id-but-different club.
        *state
            .managed_club_id()
            .write()
            .map_err(|_| IpcError::LockPoisoned {
                lock: "managed_club_id".to_string(),
            })? = None;
    }

    Ok(())
}

/// One club in the club-selection list returned by `get_clubs`.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClubChoiceDto {
    /// `ClubId.raw()` — raw u32 wire form; TS receives as `number`.
    pub club_id: u32,
    /// Club display name from the current season's league.
    pub club_name: String,
}

/// `get_clubs()` — enumerate the clubs in the current league for the
/// club-selection screen. Ordered as `league.clubs` (deterministic).
#[tauri::command]
pub async fn get_clubs(state: tauri::State<'_, AppState>) -> Result<Vec<ClubChoiceDto>, IpcError> {
    get_clubs_inner(&state)
}

pub fn get_clubs_inner(state: &AppState) -> Result<Vec<ClubChoiceDto>, IpcError> {
    let career = state.career().read().map_err(|_| IpcError::LockPoisoned {
        lock: "career".to_string(),
    })?;
    let clubs = career
        .season
        .league
        .clubs
        .iter()
        .map(|c| ClubChoiceDto {
            club_id: c.id.raw(),
            club_name: c.display_name.clone(),
        })
        .collect();
    Ok(clubs)
}

/// `select_managed_club(club_id)` — set the club the player manages this
/// session. Validates the id against the current league; the Squad screen then
/// anchors on this club. Session-only (see `AppState::managed_club_id`).
#[tauri::command]
pub async fn select_managed_club(
    club_id: u32,
    state: tauri::State<'_, AppState>,
) -> Result<(), IpcError> {
    select_managed_club_inner(club_id, &state)
}

pub fn select_managed_club_inner(club_id: u32, state: &AppState) -> Result<(), IpcError> {
    let cid = ClubId::new(club_id);
    {
        let career = state.career().read().map_err(|_| IpcError::LockPoisoned {
            lock: "career".to_string(),
        })?;
        if !career.season.league.clubs.iter().any(|c| c.id == cid) {
            return Err(IpcError::ClubNotFound { club_id });
        }
    }
    *state
        .managed_club_id()
        .write()
        .map_err(|_| IpcError::LockPoisoned {
            lock: "managed_club_id".to_string(),
        })? = Some(cid);
    Ok(())
}

/// Extract the numeric suffix from a content-pack-qualified player ID.
///
/// `"fwh.core:player_00042"` → `Some(42)`.
/// Returns `None` if the suffix is absent, non-numeric, or the ID has no
/// `_` after the `:` segment (best-effort — the caller handles `None` by
/// skipping the salience query).
fn parse_player_id_suffix(player_id: &str) -> Option<u32> {
    let after_colon = player_id.split(':').nth(1)?;
    let suffix = after_colon.split('_').next_back()?;
    suffix.parse::<u32>().ok()
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

    // ---- B1-B3: new_career / get_clubs / select_managed_club ----

    #[test]
    fn get_clubs_inner_returns_full_league_with_unique_named_clubs() {
        let state = test_app_state();
        let clubs = get_clubs_inner(&state).expect("get_clubs");
        assert_eq!(clubs.len(), 20, "the procgen division has 20 clubs");
        let mut ids: Vec<u32> = clubs.iter().map(|c| c.club_id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 20, "club ids must be unique");
        assert!(
            clubs.iter().all(|c| !c.club_name.is_empty()),
            "every club must carry a display name"
        );
    }

    #[test]
    fn new_career_reseeds_and_regenerates_a_different_world() {
        let state = test_app_state();
        let before: Vec<String> = get_clubs_inner(&state)
            .expect("clubs before")
            .into_iter()
            .map(|c| c.club_name)
            .collect();

        new_career_inner("0x0badc0de", &state).expect("new_career");

        assert_eq!(
            state.career_seed().to_u64(),
            0x0bad_c0de,
            "new_career must re-seed AppState"
        );
        let after: Vec<String> = get_clubs_inner(&state)
            .expect("clubs after")
            .into_iter()
            .map(|c| c.club_name)
            .collect();
        assert_ne!(
            before, after,
            "a different seed must regenerate a different league (procgen names are seed-derived)"
        );
    }

    #[test]
    fn select_managed_club_anchors_the_squad_screen() {
        let state = test_app_state();
        let clubs = get_clubs_inner(&state).expect("clubs");

        let placeholder = get_squad_roster_inner(&state).expect("placeholder squad");
        assert!(
            !placeholder.is_managed,
            "with no club chosen the squad is the placeholder"
        );

        // Pick a club distinct from the lowest-id placeholder so the anchor is observable.
        let target = clubs
            .iter()
            .find(|c| c.club_id != placeholder.club_id)
            .expect("a second club exists");
        select_managed_club_inner(target.club_id, &state).expect("select");

        let squad = get_squad_roster_inner(&state).expect("managed squad");
        assert!(squad.is_managed, "squad must report the managed club");
        assert_eq!(squad.club_id, target.club_id);
        assert_eq!(squad.club_name, target.club_name);
        assert_eq!(squad.players.len(), 22);
    }

    #[test]
    fn select_managed_club_rejects_unknown_id() {
        let state = test_app_state();
        let err = select_managed_club_inner(99_999, &state).expect_err("unknown club");
        assert!(
            matches!(err, IpcError::ClubNotFound { club_id: 99_999 }),
            "expected ClubNotFound, got {err:?}"
        );
    }

    #[test]
    fn new_career_clears_the_managed_club() {
        let state = test_app_state();
        let clubs = get_clubs_inner(&state).expect("clubs");
        select_managed_club_inner(clubs[1].club_id, &state).expect("select");
        assert!(
            get_squad_roster_inner(&state).expect("squad").is_managed,
            "club is managed after selection"
        );

        new_career_inner("0x1234", &state).expect("new_career");
        assert!(
            !get_squad_roster_inner(&state).expect("squad2").is_managed,
            "new_career must clear the managed club"
        );
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

    // ---- get_squad_inner ----

    #[test]
    fn get_squad_inner_returns_all_22_bios() {
        let state = test_app_state();
        let squad = get_squad_inner(&state).expect("get_squad_inner");
        assert_eq!(
            squad.len(),
            22,
            "ContentStore holds exactly 22 hand-authored player bios"
        );
        let first = &squad[0];
        assert!(!first.name.is_empty(), "name must be non-empty");
        assert!(!first.role.is_empty(), "role must be non-empty");
        // phenotype_labels must be human-readable text, NOT raw enum identifiers.
        // A rendered label is sentence-case — uppercase only at position 0
        // ("Explosive first step"). A raw multi-word CamelCase identifier
        // ("ExplosiveFirstStep") has uppercase letters PAST position 0, so
        // `chars().skip(1).any(is_uppercase)` flags exactly that — and this
        // assertion WOULD fail if get_squad_inner returned `format!("{:?}")`
        // instead of `display_label()`. (An adjacent-uppercase check does NOT
        // work: CamelCase never has two uppercase letters in a row.) Checked
        // across every player, not just squad[0].
        for player in &squad {
            for label in &player.phenotype_labels {
                let looks_like_raw_identifier = label.chars().skip(1).any(|c| c.is_uppercase());
                assert!(
                    !looks_like_raw_identifier,
                    "phenotype label {label:?} (player {}) looks like a raw \
                     CamelCase identifier — display_label() must render \
                     sentence-case text",
                    player.player_id
                );
            }
        }
        // At least one label across all players must contain a space (proving
        // multi-word labels are rendered correctly, not as raw identifiers).
        let any_with_space = squad
            .iter()
            .flat_map(|p| &p.phenotype_labels)
            .any(|l| l.contains(' '));
        assert!(
            any_with_space,
            "at least one phenotype label must contain a space (multi-word)"
        );
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

    // ---- get_player_detail_inner ----

    #[test]
    fn get_player_detail_inner_known_id_returns_phenotype() {
        let state = test_app_state();
        // Pick the first player from the content store.
        let first_id = state
            .content()
            .player_bios
            .keys()
            .next()
            .expect("content store has at least one player bio")
            .clone();

        let dto = get_player_detail_inner(&first_id, &state)
            .expect("get_player_detail_inner must succeed for a known player_id");

        assert_eq!(dto.phenotype.player_id, first_id);
        assert!(!dto.phenotype.name.is_empty(), "name must be non-empty");
        assert!(!dto.phenotype.role.is_empty(), "role must be non-empty");
        // Runtime ledger is empty → callbacks is empty (honest, not fabricated).
        assert!(
            dto.memory_callbacks.is_empty(),
            "empty runtime ledger must yield no callbacks"
        );
        // Contract deferred until T4.
        assert!(
            dto.contract_status.is_none(),
            "contract_status must be None at T3"
        );
    }

    #[test]
    fn get_player_detail_inner_unknown_id_returns_player_not_found() {
        let state = test_app_state();
        let err = get_player_detail_inner("fwh.core:player_99999", &state)
            .expect_err("unknown id must return Err");
        match err {
            IpcError::PlayerNotFound { player_id } => {
                assert_eq!(player_id, "fwh.core:player_99999");
            }
            other => panic!("expected PlayerNotFound, got {other:?}"),
        }
    }

    #[test]
    fn player_detail_dto_serializes_camel_case_keys() {
        let state = test_app_state();
        let first_id = state
            .content()
            .player_bios
            .keys()
            .next()
            .expect("at least one bio")
            .clone();
        let dto = get_player_detail_inner(&first_id, &state).expect("detail");
        let json = serde_json::to_string(&dto).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        let obj = v.as_object().expect("object");

        assert!(obj.contains_key("phenotype"), "missing key 'phenotype'");
        assert!(
            obj.contains_key("memoryCallbacks"),
            "missing key 'memoryCallbacks'"
        );
        assert!(
            obj.contains_key("contractStatus"),
            "missing key 'contractStatus'"
        );

        let pheno = obj["phenotype"].as_object().expect("phenotype is object");
        assert!(pheno.contains_key("playerId"), "missing key 'playerId'");
        assert!(pheno.contains_key("name"), "missing key 'name'");
        assert!(pheno.contains_key("role"), "missing key 'role'");
        assert!(
            pheno.contains_key("birthRegion"),
            "missing key 'birthRegion'"
        );
        assert!(
            pheno.contains_key("phenotypeLabels"),
            "missing key 'phenotypeLabels'"
        );
        // No snake_case leakage.
        assert!(
            !pheno.contains_key("player_id"),
            "snake_case leak: player_id"
        );
        assert!(
            !pheno.contains_key("birth_region"),
            "snake_case leak: birth_region"
        );
    }

    #[test]
    fn parse_player_id_suffix_extracts_numeric_suffix() {
        assert_eq!(parse_player_id_suffix("fwh.core:player_00042"), Some(42));
        assert_eq!(parse_player_id_suffix("fwh.core:player_00001"), Some(1));
        assert_eq!(parse_player_id_suffix("fwh.core:player_99999"), Some(99999));
    }

    #[test]
    fn parse_player_id_suffix_returns_none_for_malformed() {
        // No colon → None.
        assert_eq!(parse_player_id_suffix("nodots"), None);
        // Non-numeric suffix → None.
        assert_eq!(parse_player_id_suffix("fwh.core:player_abc"), None);
        // Hand-authored dotted-form IDs (no underscore in the local part after colon).
        // "fwh.core:culture.anglo" → last '_' split finds "culture.anglo", no parse → None.
        // This is fine — hand-authored IDs don't go through this path.
    }

    // ---- advance_season_inner tests (AC3) -----------------------------------

    /// AC3: incomplete season → SeasonNotComplete error.
    #[test]
    fn advance_season_inner_rejects_incomplete_season() {
        let state = test_app_state();
        // Fresh state: season not yet complete.
        let err = advance_season_inner(&state).expect_err("should fail on incomplete season");
        match err {
            IpcError::SeasonNotComplete => {}
            other => panic!("expected SeasonNotComplete, got {other:?}"),
        }
    }

    /// AC3: complete season → Ok, season_number incremented, new fresh season.
    #[test]
    fn advance_season_inner_complete_season_increments_season_number() {
        let state = test_app_state();
        // Complete the current season first.
        play_fixtures_inner(&state).expect("play_fixtures");
        assert!(
            state.career().read().expect("lock").season.is_complete(),
            "precondition: season must be complete"
        );

        let summary = advance_season_inner(&state).expect("advance_season_inner should succeed");
        assert_eq!(summary.completed_season, 0, "first completed season is 0");
        assert_eq!(
            summary.new_season_number, 1,
            "season_number must be 1 after first advance"
        );
        assert!(
            !summary.champion_club_name.is_empty(),
            "champion_club_name must be non-empty"
        );

        let career = state.career().read().expect("career lock");
        // Season_number on AppState must be 1.
        assert_eq!(
            career.season_number.0, 1,
            "AppState.season_number must be 1 after advance"
        );
        // The new season must not be complete.
        assert!(
            !career.season.is_complete(),
            "new season must not be complete immediately after advance_season"
        );
        assert_eq!(
            career.season.current_match_day, 1,
            "new season starts at match-day 1"
        );
    }

    /// AC3: ledger receives exactly one TitleWon event after advance_season.
    #[test]
    fn advance_season_inner_emits_title_won_event() {
        let state = test_app_state();
        play_fixtures_inner(&state).expect("play_fixtures");
        advance_season_inner(&state).expect("advance_season");

        let career = state.career().read().expect("career lock");
        let title_won_count = career
            .ledger
            .iter()
            .filter(|e| matches!(e.event_class, EventClass::TitleWon))
            .count();
        assert_eq!(
            title_won_count, 1,
            "exactly one TitleWon event after one season"
        );
    }

    // ---- get_career_overview_inner tests (AC5) ------------------------------

    /// AC5: fresh state (no seasons completed) → empty history + callbacks.
    #[test]
    fn get_career_overview_inner_fresh_state_returns_empty_history() {
        let state = test_app_state();
        let dto = get_career_overview_inner(&state).expect("get_career_overview");
        assert_eq!(dto.season_number, 0, "fresh career starts at season 0");
        assert!(
            dto.history.is_empty(),
            "no history before any season completes"
        );
        assert!(
            dto.cross_season_callbacks.is_empty(),
            "no callbacks before any season completes"
        );
    }

    /// AC5: after one completed season, history has one entry and callbacks are present.
    #[test]
    fn get_career_overview_inner_after_one_season_has_history() {
        let state = test_app_state();
        play_fixtures_inner(&state).expect("play_fixtures");
        advance_season_inner(&state).expect("advance_season");

        let dto = get_career_overview_inner(&state).expect("get_career_overview after season 1");
        assert_eq!(dto.season_number, 1, "season_number is 1 after one advance");
        assert_eq!(dto.history.len(), 1, "one history entry for season 0");
        assert_eq!(dto.history[0].season, 0, "history entry is for season 0");
        assert!(
            !dto.history[0].champion_club_name.is_empty(),
            "champion_club_name must be non-empty"
        );
        // Callbacks should be non-empty (TitleWon events are in past seasons).
        assert!(
            !dto.cross_season_callbacks.is_empty(),
            "cross_season_callbacks must be non-empty after one completed season"
        );
        // No template seams in callback strings.
        for cb in &dto.cross_season_callbacks {
            assert!(!cb.is_empty(), "callback must not be empty");
            assert!(
                !cb.contains("{{"),
                "callback contains '{{{{' template seam: {cb:?}"
            );
            assert!(
                !cb.contains('#'),
                "callback contains '#' tracery seam: {cb:?}"
            );
        }
    }

    // ---- T3-R-F: career clock drives salience decay ----

    /// Integration test: a fixture ledger of tick-bearing events, ranked via
    /// `SalienceReader::top_n` at the real `CareerState::current_tick()`, applies
    /// salience decay — an outcome the old `Tick::ZERO` placeholder could not
    /// produce.
    ///
    /// Two events for the same player, identical raw salience: one with a short
    /// `Linear` decay, one that `Never` decays. At the career clock (`now_tick`
    /// well past the decayer's lifetime) the decayer's projected salience is
    /// zero, so the `Never` event ranks first. At `Tick::ZERO` the decay guard
    /// (`elapsed <= 0`) leaves both at full salience — they tie, and the tie
    /// breaks to the lower `event_id`, so the decayer ranks first. The order
    /// flip proves the career clock genuinely feeds decay.
    #[test]
    fn salience_decay_applied_through_career_clock() {
        use fw_core::{MatchId, Q32, Tick};
        use fw_memory::event::{
            CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind,
            Emotion, EntityRef, EventId, MemoryEvent, Participant, ParticipantRole, SourceId,
        };

        let player = PlayerId::new(7);

        // A tick-0 event for `player`, identical in every field except its
        // decay function — so the two events have the same computed raw
        // salience and differ only in how they decay.
        fn fixture_event(player: PlayerId, decay: DecayFunction) -> MemoryEvent {
            MemoryEvent {
                event_id: EventId(0), // overwritten by ledger.append
                schema_version: 1,
                season: SeasonNumber(0),
                tick: Some(Tick::from_raw(0)),
                career_date: CareerDate {
                    year: 1,
                    day_of_year: 1,
                },
                emitter: Emitter {
                    kind: EmitterKind::MatchEngine,
                    source_id: SourceId::Match(MatchId::new(0)),
                },
                participants: vec![Participant {
                    role: ParticipantRole::Subject,
                    entity: EntityRef::Player(player),
                }],
                event_class: EventClass::LegacyGoal,
                stakes: Q32::ONE,
                emotion: Emotion::Joy,
                consequence: vec![Consequence::None],
                callback_eligibility: CallbackEligibility::Immediate,
                salience: Q32::ZERO, // overwritten by ledger.append (compute_salience)
                decay_function: decay,
            }
        }

        let state = test_app_state();
        let mut career = state.career().write().expect("career lock");

        // event_id 0: the fast Linear-decay event. event_id 1: the Never event.
        let decayer = fixture_event(
            player,
            DecayFunction::Linear {
                lifetime_ticks: 100,
            },
        );
        let stable = fixture_event(player, DecayFunction::Never);
        career.ledger.append(decayer);
        career.ledger.append(stable);

        // The career clock for a fresh career (season 0, match-day 1) is well
        // past the decayer's 100-tick lifetime.
        let now_tick = career.current_tick();
        assert!(
            now_tick.to_raw() > 100,
            "the fresh-career clock must exceed the decayer's lifetime for this test"
        );

        let at_career_clock = SalienceReader::top_n(
            &mut career.ledger,
            5,
            SalienceFilter::BySubject(player),
            now_tick,
        );
        assert_eq!(
            at_career_clock[0].event_id,
            EventId(1),
            "at the career clock the decayer (id 0) is fully decayed — the Never \
             event (id 1) must rank first",
        );

        // Control: at Tick::ZERO the decay guard fires; the order reverses.
        let at_zero = SalienceReader::top_n(
            &mut career.ledger,
            5,
            SalienceFilter::BySubject(player),
            Tick::ZERO,
        );
        assert_eq!(
            at_zero[0].event_id,
            EventId(0),
            "at Tick::ZERO both events hold full salience — the tie breaks to \
             the lower event_id, so the decayer (id 0) ranks first",
        );
    }

    // -------------------------------------------------------------------------
    // T4-2.5b: get_roster_for_club IPC tests
    // -------------------------------------------------------------------------

    /// AC3: valid club id → Vec<PlayerRosterDto> of length 22, slot-ordered.
    #[test]
    fn get_roster_for_club_inner_valid_id_returns_22_slot_ordered() {
        let state = test_app_state();
        // Use the first club id from the generated league.
        let career = state.career().read().expect("career lock");
        let first_club_id = career.season.league.clubs[0].id.raw();
        drop(career);

        let dtos = get_roster_for_club_inner(first_club_id, &state)
            .expect("get_roster_for_club_inner valid club");

        assert_eq!(dtos.len(), 22, "must return 22 players per club");
        assert_eq!(dtos[0].slot, 0, "first entry must be GK (slot 0)");
        for (i, dto) in dtos.iter().enumerate() {
            assert_eq!(
                dto.slot as usize, i,
                "slot at index {i} must be {i}, got {}",
                dto.slot
            );
        }
    }

    /// AC3: unknown club id → IpcError::ClubNotFound.
    #[test]
    fn get_roster_for_club_inner_unknown_id_returns_club_not_found() {
        let state = test_app_state();
        let result = get_roster_for_club_inner(999_999_u32, &state);
        assert!(
            matches!(result, Err(IpcError::ClubNotFound { club_id: 999_999 })),
            "unknown club id must return ClubNotFound, got: {:?}",
            result
        );
    }

    /// AC2: career state has 440 instances for the default 20-club league.
    #[test]
    fn career_state_roster_has_440_instances_at_career_start() {
        let state = test_app_state();
        let career = state.career().read().expect("career lock");
        let total: usize = career.roster.values().map(|v| v.len()).sum();
        assert_eq!(
            total, 440,
            "default 20-club league must have 20×22=440 roster instances"
        );
        let league_club_ids: std::collections::BTreeSet<fw_core::ClubId> =
            career.season.league.clubs.iter().map(|c| c.id).collect();
        let roster_club_ids: std::collections::BTreeSet<fw_core::ClubId> =
            career.roster.keys().copied().collect();
        assert_eq!(
            league_club_ids, roster_club_ids,
            "roster club ids must match league club ids"
        );
    }

    // -------------------------------------------------------------------------
    // T4-2.5h: stat-accrual + get_squad_roster tests
    // -------------------------------------------------------------------------

    /// AC-stat-1: after one advance_week, all starting-XI players (slots 0-10)
    /// have appearances == 1, minutes_played == 90.
    ///
    /// Only the starting XI (slots 0-10) appear in each match; slots 11-21
    /// are bench/subs who do not feature until a sub system is modeled (T5-5b).
    /// Each club's starting 11 map to match slots as either home (0-10) or
    /// away (11-21 in the match, but drawn from instances[0..11]).
    #[test]
    fn t4_2_5h_appearances_and_minutes_accrue_after_one_matchday() {
        let state = test_app_state();
        advance_week_inner(&state).expect("advance_week");

        let career = state.career().read().expect("career lock");

        // Only slots 0-10 (starting XI) receive appearances per match-day.
        // Slots 11-21 are bench — they should remain at 0.
        for (club_id, instances) in &career.roster {
            for inst in &instances[..11] {
                assert_eq!(
                    inst.season_stats.appearances, 1,
                    "club {:?} player {:?} (slot {}) must have appearances == 1 \
                     after match-day 1; got {}",
                    club_id, inst.player_id, inst.slot, inst.season_stats.appearances
                );
                assert_eq!(
                    inst.season_stats.minutes_played, 90,
                    "club {:?} player {:?} (slot {}) must have minutes_played == 90 \
                     after match-day 1; got {}",
                    club_id, inst.player_id, inst.slot, inst.season_stats.minutes_played
                );
            }
            // Bench (slots 11-21) must not have been credited.
            for inst in &instances[11..] {
                assert_eq!(
                    inst.season_stats.appearances, 0,
                    "bench player {:?} (slot {}) must have appearances == 0; got {}",
                    inst.player_id, inst.slot, inst.season_stats.appearances
                );
            }
        }
    }

    /// AC-stat-2: goals — at least one player has goals > 0 after playing a full season.
    ///
    /// Goals depend on match simulation output. Running a full season (play_fixtures)
    /// guarantees enough matches that at least some goals must have occurred (the sim
    /// produces goals in 600-tick matches). Proves the goal-accrual path is wired.
    #[test]
    fn t4_2_5h_goals_accrue_after_full_season() {
        let state = test_app_state();
        play_fixtures_inner(&state).expect("play_fixtures");

        let career = state.career().read().expect("career lock");

        // Count LegacyGoal events per scorer (Subject player) from the ledger —
        // the independent source of truth for who scored.
        let mut ledger_goals: std::collections::BTreeMap<PlayerId, u32> =
            std::collections::BTreeMap::new();
        for event in career.ledger.iter() {
            if !matches!(event.event_class, EventClass::LegacyGoal) {
                continue;
            }
            let subject = event.participants.iter().find_map(|p| {
                if matches!(p.role, fw_memory::event::ParticipantRole::Subject)
                    && let fw_memory::event::EntityRef::Player(pid) = p.entity
                {
                    Some(pid)
                } else {
                    None
                }
            });
            if let Some(pid) = subject {
                *ledger_goals.entry(pid).or_insert(0) += 1;
            }
        }

        let total_ledger_goals: u32 = ledger_goals.values().sum();
        assert!(
            total_ledger_goals > 0,
            "a full season must emit at least one LegacyGoal — else the per-player check below is vacuous"
        );

        // Each player's season_stats.goals must EXACTLY equal its LegacyGoal count
        // in the ledger. The bare `total > 0` this replaced was vacuous: it passed
        // even if goals were credited to the wrong player or double-counted under
        // the split-call harvest. This catches wrong-player attribution AND
        // double-count (T4-2.5h code-review P1).
        let mut total_season_goals: u32 = 0;
        for inst in career.roster.values().flat_map(|v| v.iter()) {
            let expected = ledger_goals.get(&inst.player_id).copied().unwrap_or(0);
            assert_eq!(
                inst.season_stats.goals as u32, expected,
                "player {:?} season_stats.goals ({}) must equal its LegacyGoal count in the ledger ({})",
                inst.player_id, inst.season_stats.goals, expected
            );
            total_season_goals += inst.season_stats.goals as u32;
        }

        // Conservation: sum of per-player season goals == total LegacyGoal events
        // (no goal counted twice, none dropped).
        assert_eq!(
            total_season_goals, total_ledger_goals,
            "sum of season_stats.goals must equal total LegacyGoal events (no double-count / no drop)"
        );
    }

    /// AC-stat-3: career_apps persists across season rollover; season_stats resets.
    ///
    /// - After season 1: career_apps >= 1 for each player, season_stats.appearances >= 1.
    /// - After advance_season: career_apps unchanged; season_stats.appearances == 0.
    /// - After advance_week in season 2: season_stats.appearances == 1 again.
    #[test]
    fn t4_2_5h_season_stats_reset_career_apps_persist_across_season() {
        let state = test_app_state();

        // Play a full season to accrue stats.
        play_fixtures_inner(&state).expect("play_fixtures season 1");

        // Snapshot career_apps before season rollover.
        let career_apps_before: Vec<u32> = {
            let career = state.career().read().expect("career lock");
            career
                .roster
                .values()
                .flat_map(|v| v.iter())
                .map(|inst| inst.career_apps)
                .collect()
        };

        // Guard against a vacuous "persistence" check: if the accrual path were
        // broken and career_apps stayed 0 for everyone, `before == after` would
        // pass trivially (all-zeros). Prove career_apps was actually accrued
        // before the rollover (a full season → every starting-XI player ≥ 1 app).
        assert!(
            career_apps_before.iter().any(|&c| c > 0),
            "career_apps must be > 0 for at least one player before advance_season \
             (else the persistence assertion is vacuous)"
        );

        // Roll over to season 2.
        advance_season_inner(&state).expect("advance_season");

        {
            let career = state
                .career()
                .read()
                .expect("career lock after advance_season");

            let career_apps_after: Vec<u32> = career
                .roster
                .values()
                .flat_map(|v| v.iter())
                .map(|inst| inst.career_apps)
                .collect();

            // career_apps must be identical before and after advance_season.
            assert_eq!(
                career_apps_before, career_apps_after,
                "career_apps must not be reset by advance_season"
            );

            // season_stats must all be zeroed.
            for inst in career.roster.values().flat_map(|v| v.iter()) {
                assert_eq!(
                    inst.season_stats.appearances, 0,
                    "season_stats.appearances must be 0 after advance_season; player {:?} has {}",
                    inst.player_id, inst.season_stats.appearances
                );
                assert_eq!(
                    inst.season_stats.goals, 0,
                    "season_stats.goals must be 0 after advance_season; player {:?} has {}",
                    inst.player_id, inst.season_stats.goals
                );
                assert_eq!(
                    inst.season_stats.minutes_played, 0,
                    "season_stats.minutes_played must be 0 after advance_season; player {:?} has {}",
                    inst.player_id, inst.season_stats.minutes_played
                );
            }
        }

        // Play one match-day in season 2 — starting XI appearances should be 1 again.
        advance_week_inner(&state).expect("advance_week season 2");

        let career = state.career().read().expect("career lock season 2");
        for instances in career.roster.values() {
            // Starting XI (slots 0-10) must have one appearance in season 2.
            for inst in &instances[..11] {
                assert_eq!(
                    inst.season_stats.appearances, 1,
                    "season_stats.appearances must be 1 after first match-day of season 2; \
                     player {:?} (slot {}) has {}",
                    inst.player_id, inst.slot, inst.season_stats.appearances
                );
            }
        }
    }

    /// AC-squad-roster-1: get_squad_roster_inner returns 22 players + non-empty club name.
    #[test]
    fn t4_2_5h_get_squad_roster_returns_22_players_and_club_name() {
        let state = test_app_state();
        let dto = get_squad_roster_inner(&state).expect("get_squad_roster_inner");

        assert_eq!(
            dto.players.len(),
            22,
            "squad roster must have 22 players; got {}",
            dto.players.len()
        );
        assert!(
            !dto.club_name.is_empty(),
            "squad roster club_name must not be empty"
        );
    }

    /// AC-squad-roster-2: get_squad_roster_inner resolves the lowest ClubId.
    #[test]
    fn t4_2_5h_get_squad_roster_is_lowest_club_id() {
        let state = test_app_state();

        // Derive the expected lowest club id from the career directly.
        let expected_club_id = {
            let career = state.career().read().expect("career lock");
            career
                .roster
                .keys()
                .next()
                .expect("at least one club")
                .raw()
        };

        let dto = get_squad_roster_inner(&state).expect("get_squad_roster_inner");
        assert_eq!(
            dto.club_id, expected_club_id,
            "get_squad_roster_inner must return the lowest ClubId's squad"
        );
    }

    /// AC-squad-roster-3: stats appear in the DTO after playing a match-day.
    ///
    /// The first club's starting XI (slots 0-10) should have appearances == 1
    /// and minutes_played == 90. Bench slots (11-21) remain at 0.
    #[test]
    fn t4_2_5h_get_squad_roster_reflects_accrued_stats() {
        let state = test_app_state();
        advance_week_inner(&state).expect("advance_week");

        let dto = get_squad_roster_inner(&state).expect("get_squad_roster_inner");

        // Partition players by starting XI (slots 0-10) vs bench (slots 11-21).
        for player in &dto.players {
            if player.slot <= 10 {
                // Starting XI must have appearances == 1, minutes == 90.
                assert_eq!(
                    player.appearances, 1,
                    "starting XI slot {} appearances must be 1 after match-day 1; got {}",
                    player.slot, player.appearances
                );
                assert_eq!(
                    player.minutes_played, 90,
                    "starting XI slot {} minutes_played must be 90 after match-day 1; got {}",
                    player.slot, player.minutes_played
                );
            } else {
                // Bench must have appearances == 0 (no sub system until T5-5b).
                assert_eq!(
                    player.appearances, 0,
                    "bench slot {} appearances must be 0 (no sub system); got {}",
                    player.slot, player.appearances
                );
            }
        }
    }

    // -------------------------------------------------------------------------
    // T4-2.5e: player-subject MemoryEvent emission + blank-name render fix
    // -------------------------------------------------------------------------

    /// Helper: run a multi-match-day career and return the AppState.
    /// Uses the default career seed for determinism.
    fn run_career_multi_season(match_days: u16) -> AppState {
        let state = test_app_state();
        // Play `match_days` worth of fixtures. Each advance_week plays one match-day.
        for _ in 0..match_days {
            match advance_week_inner(&state) {
                Ok(_) => {}
                Err(IpcError::SeasonComplete) => break,
                Err(e) => panic!("advance_week_inner failed: {e:?}"),
            }
        }
        state
    }

    /// AC1 — at least one DebutSenior (or DebutClub) event emitted after
    /// playing ≥1 match-day.
    ///
    /// Every player has `career_apps == 0` at career start, so the FIRST
    /// match-day MUST trigger debut events for all 22 appearing slots.
    #[test]
    fn t4_2_5e_debut_senior_emitted_after_first_match_day() {
        let state = run_career_multi_season(1);
        let career = state.career().read().expect("career lock");

        let debut_count = career
            .ledger
            .iter()
            .filter(|e| {
                matches!(
                    e.event_class,
                    EventClass::DebutSenior | EventClass::DebutClub
                )
            })
            .count();

        // Each match on match-day 1 has 22 appearing slots → 10 fixtures ×
        // 22 = 220 debut events for the first match-day.
        assert!(
            debut_count >= 1,
            "at least one DebutSenior event must be emitted after match-day 1; got 0"
        );

        // Every DebutSenior/DebutClub event must have a Subject player participant.
        for event in career.ledger.iter().filter(|e| {
            matches!(
                e.event_class,
                EventClass::DebutSenior | EventClass::DebutClub
            )
        }) {
            let has_subject = event.participants.iter().any(|p| {
                matches!(p.role, fw_memory::event::ParticipantRole::Subject)
                    && matches!(p.entity, fw_memory::event::EntityRef::Player(_))
            });
            assert!(
                has_subject,
                "DebutSenior/DebutClub event must have a Subject player participant; event {:?}",
                event.event_id
            );
        }
    }

    /// AC1 (career_apps increment) — career_apps is non-zero on roster
    /// instances after ≥1 match-day; it must not still be zero for any
    /// player that was in a starting XI.
    #[test]
    fn t4_2_5e_career_apps_incremented_after_match_day() {
        let state = run_career_multi_season(1);
        let career = state.career().read().expect("career lock");

        // At least one instance must have career_apps > 0.
        let any_appeared = career
            .roster
            .values()
            .flat_map(|v| v.iter())
            .any(|inst| inst.career_apps > 0);
        assert!(
            any_appeared,
            "at least one PlayerInstance must have career_apps > 0 after one match-day"
        );
    }

    /// AC2 — at least one LegacyGoal event emitted with correct subject
    /// attribution, including home/away correctness.
    ///
    /// Strengthened assertion (FIX 3): for each LegacyGoal, verify that the
    /// Subject player's `PlayerId` is in the roster AND that the `Counterparty
    /// Club` participant on the event matches the `club_id` field of the
    /// `PlayerInstance` found by that `PlayerId`. A home/away inversion in the
    /// slot→roster mapping (e.g. `scorer_slot < 11` but indexing
    /// `away_instances`) would produce a mismatch: the roster player found by
    /// `PlayerId` would have a different `club_id` than the Club participant on
    /// the event.
    ///
    /// With 600-tick matches, goals may or may not occur. Run enough match-days
    /// that at least one goal is virtually certain (38 match-days = full season).
    #[test]
    fn t4_2_5e_legacy_goal_attributed_to_roster_player() {
        // Play a full season to maximise probability of at least one goal.
        let state = test_app_state();
        play_fixtures_inner(&state).expect("play_fixtures full season");

        let career = state.career().read().expect("career lock");

        // Build PlayerId → (club_id) map from the full roster for cross-check.
        let roster_by_id: std::collections::BTreeMap<PlayerId, fw_core::ClubId> = career
            .roster
            .values()
            .flat_map(|v| v.iter().map(|inst| (inst.player_id, inst.club_id)))
            .collect();

        let goal_events: Vec<_> = career
            .ledger
            .iter()
            .filter(|e| matches!(e.event_class, EventClass::LegacyGoal))
            .collect();

        assert!(
            !goal_events.is_empty(),
            "at least one LegacyGoal event must be emitted across a full season"
        );

        for event in &goal_events {
            // Extract Subject player id.
            let subject_pid = event.participants.iter().find_map(|p| {
                if matches!(p.role, fw_memory::event::ParticipantRole::Subject)
                    && let fw_memory::event::EntityRef::Player(pid) = p.entity
                {
                    return Some(pid);
                }
                None
            });
            let subject_pid =
                subject_pid.expect("LegacyGoal must have a Subject player participant");

            // Must be in the roster.
            let roster_club_id = roster_by_id.get(&subject_pid).copied().unwrap_or_else(|| {
                panic!(
                    "LegacyGoal subject {:?} not found in roster (scorer_slot→roster \
                     attribution is broken)",
                    subject_pid
                )
            });

            // Extract Counterparty Club id from the event.
            let event_club_id = event.participants.iter().find_map(|p| {
                if matches!(p.role, fw_memory::event::ParticipantRole::Counterparty)
                    && let fw_memory::event::EntityRef::Club(cid) = p.entity
                {
                    return Some(cid);
                }
                None
            });
            let event_club_id = event_club_id.expect(
                "LegacyGoal must have a Counterparty Club participant (club_name resolution)",
            );

            // The event's club must match the roster player's actual club.
            // A mismatch means home/away was inverted in the slot mapping.
            assert_eq!(
                event_club_id, roster_club_id,
                "LegacyGoal event club {:?} must match the roster player's club {:?} \
                 (home/away attribution is inverted in scorer_slot→roster mapping)",
                event_club_id, roster_club_id
            );
        }
    }

    /// AC3 — `get_player_detail_inner` returns non-empty memoryCallbacks for
    /// an appeared rostered player after ≥1 match-day.
    ///
    /// Uses a roster-range id (`fwh.core:player_01000000`, suffix = 1_000_000 =
    /// ROSTER_PLAYER_ID_BASE, which is club 0 slot 0 = GK). This suffix is ≥
    /// ROSTER_PLAYER_ID_BASE so `get_player_detail_inner` routes to the roster
    /// path (not the content-bio path). The GK always appears in match-day 1,
    /// so `memoryCallbacks` must be non-empty.
    #[test]
    fn t4_2_5e_get_player_detail_returns_callbacks_for_appeared_player() {
        use crate::roster::ROSTER_PLAYER_ID_BASE;

        let state = run_career_multi_season(1);

        // Format as zero-padded 8-digit suffix: ROSTER_PLAYER_ID_BASE = 1_000_000.
        let player_id_str = format!("fwh.core:player_{ROSTER_PLAYER_ID_BASE:08}");
        let dto = get_player_detail_inner(&player_id_str, &state)
            .expect("get_player_detail_inner must succeed for a roster player");

        assert!(
            !dto.memory_callbacks.is_empty(),
            "appeared rostered player must have ≥1 memory callback; \
             got empty for {player_id_str}"
        );
    }

    /// AC4 — no orphaned ` — ` (blank-name fragments) in rendered callbacks.
    ///
    /// Runs after a full season to accumulate a rich ledger, then checks every
    /// rendered callback string. The blank-club pattern was: a DebutSenior
    /// event with no Club participant produced `"First senior appearance for  — name"`.
    /// Adding the Club counterparty participant (T4-2.5e fix) eliminates this.
    #[test]
    fn t4_2_5e_no_orphaned_em_dash_in_rendered_callbacks() {
        let state = test_app_state();
        play_fixtures_inner(&state).expect("play_fixtures full season");

        // Check callbacks for the first 5 content-store players (bios exist).
        let bio_ids: Vec<String> = state
            .content()
            .player_bios
            .keys()
            .take(5)
            .cloned()
            .collect();

        for pid_str in &bio_ids {
            let dto = get_player_detail_inner(pid_str, &state)
                .expect("get_player_detail_inner must succeed");
            for cb in &dto.memory_callbacks {
                // Check for the blank-name em-dash pattern: "for  — " or " —  " etc.
                // The orphaned form has a space immediately before or after " — ".
                // Normal usage: "scored against Arsenal — a goal people still talk about"
                // (the em-dash is mid-sentence, not between two blanks). We check
                // for " —  " (space em-dash double-space) and " — " at the START
                // of the string (em-dash after leading space = blank token before it).
                assert!(
                    !cb.contains("  — ") && !cb.contains(" —  "),
                    "callback for {pid_str:?} has orphaned em-dash (blank name/club): {cb:?}"
                );
            }
        }

        // Also check the roster-path player (club 0 slot 0 GK, no bio).
        use crate::roster::ROSTER_PLAYER_ID_BASE;
        let roster_id_str = format!("fwh.core:player_{ROSTER_PLAYER_ID_BASE:08}");
        let dto =
            get_player_detail_inner(&roster_id_str, &state).expect("roster player must be found");
        for cb in &dto.memory_callbacks {
            assert!(
                !cb.contains("  — ") && !cb.contains(" —  "),
                "roster-path callback has orphaned em-dash: {cb:?}"
            );
        }
    }

    /// AC5 — canonical pins UNCHANGED: explicitly re-run the fw-replay pin
    /// scenarios and verify the hashes match the pinned values. This test
    /// mirrors what `cargo test -p fw-replay` does, executed here as an
    /// explicit assertion so the report can include the pin verification.
    ///
    /// Seeds and tick counts mirror `crates/fw-replay/tests/canonical_hash.rs`:
    /// - 60-tick: seed `0xDEAD_BEEF_DEAD_BEEF`, plain `MatchState::initial`
    ///   → `d1170bfc…`
    /// - 600-tick: seed `0xfeed_beef_cafe_fade`, `initial_with_content`
    ///   → `f139c76a…` (FUN-TS2 shot-quality tuning: SIGMA_BASE 7m, threshold
    ///   0.070, SAVE_BASE 0.62/0.82) — then further updated at FUN-TS2 press-role
    ///   fix (off-by-one corrected; hash moves again — see re-baseline history
    ///   in `crates/fw-replay/tests/canonical_hash.rs`).
    ///
    /// This hand-synced pin must move in lockstep with fw-replay's pins.
    ///
    /// The harvest path only writes to `career.ledger` and `career.roster`
    /// (career_apps increments) — it does NOT touch `MatchState` canonical
    /// fields, `MatchEvent` encoding, or `fw-replay`. This test is the formal
    /// AC5 confirmation.
    #[test]
    fn t4_2_5e_canonical_pins_unchanged() {
        use fw_match_sim::MatchState;

        let state = test_app_state();

        // 60-tick pin — seed `0xDEAD_BEEF_DEAD_BEEF`, 60 ticks.
        // Matches `smoke_seed_60_tick_canonical_hash_pinned` in fw-replay.
        // NOTE: the 60-tick pin uses `MatchState::initial(seed)` (no content),
        // matching the fw-replay test exactly.
        let seed_60 = fw_core::Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);
        let mut sim = MatchState::initial(seed_60);
        for _ in 0..60 {
            sim = fw_match_sim::tick_match(sim, state.signature_definitions());
        }
        let hash_60 = {
            let bytes = sim.encode_canonical();
            let h: [u8; 32] = blake3::hash(&bytes).into();
            format!(
                "blake3:{}",
                h.iter().map(|b| format!("{b:02x}")).collect::<String>()
            )
        };
        assert!(
            hash_60.starts_with("blake3:110158b9"),
            "60-tick canonical hash must start with 110158b9 (FUN-TS3b Attempt 2: pass-kind utility reweighting; keep in lockstep with fw-replay's PINNED_60_TICK); got {hash_60}"
        );

        // 600-tick pin — seed `0xfeed_beef_cafe_fade`, 600 ticks.
        // Matches `extended_seed_600_tick_canonical_hash_pinned` in fw-replay.
        // NOTE: the fw-replay test uses home=DEFAULT_ARCHETYPE_ID,
        // away="fwh.core:archetype.low-block-counter" — must match exactly.
        let seed_600 = fw_core::Seed::from_u64(0xfeed_beef_cafe_fade);
        let mut sim600 = MatchState::initial_with_content(
            seed_600,
            state.content(),
            fw_match_sim::DEFAULT_ARCHETYPE_ID,
            "fwh.core:archetype.low-block-counter",
        )
        .expect("init 600-tick");
        for _ in 0..600 {
            sim600 = fw_match_sim::tick_match(sim600, state.signature_definitions());
        }
        let hash_600 = {
            let bytes = sim600.encode_canonical();
            let h: [u8; 32] = blake3::hash(&bytes).into();
            format!(
                "blake3:{}",
                h.iter().map(|b| format!("{b:02x}")).collect::<String>()
            )
        };
        assert!(
            hash_600.starts_with("blake3:a5dd8dfa"),
            "600-tick canonical hash must start with a5dd8dfa (FUN-CB1-#23: lane_openness wired into pass completion; keep in lockstep with fw-replay's PINNED_600_TICK); got {hash_600}"
        );
    }

    // ---------------------------------------------------------------------------
    // get_scout_report_inner tests (T4-2.5f)
    // ---------------------------------------------------------------------------

    /// Fresh career (no match played) → `NotYetObserved`.
    #[test]
    fn get_scout_report_inner_not_yet_observed_errors() {
        use crate::roster::ROSTER_PLAYER_ID_BASE;

        let state = test_app_state();
        // Slot 0 of club 0: PlayerId(ROSTER_PLAYER_ID_BASE + 0)
        let player_id_str = format!("fwh.core:player_{ROSTER_PLAYER_ID_BASE:08}");

        let result = get_scout_report_inner(&player_id_str, &state);
        match result {
            Err(IpcError::NotYetObserved { player_id }) => {
                assert!(
                    player_id.contains(&format!("{ROSTER_PLAYER_ID_BASE:08}")),
                    "NotYetObserved player_id should match requested id"
                );
            }
            other => panic!("expected NotYetObserved, got {other:?}"),
        }
    }

    /// Non-roster id → `PlayerNotFound`.
    #[test]
    fn get_scout_report_inner_non_roster_id_errors() {
        let state = test_app_state();
        let result = get_scout_report_inner("fwh.core:player_00001", &state);
        match result {
            Err(IpcError::PlayerNotFound { .. }) => {}
            other => panic!("expected PlayerNotFound for content-bio id, got {other:?}"),
        }
    }

    /// After one `advance_week`, the starting XI for each playing club has a
    /// scouting report cached (observation_count == 1, categories.len() == 3).
    #[test]
    fn get_scout_report_inner_returns_banded_dto_after_advance_week() {
        use crate::roster::ROSTER_PLAYER_ID_BASE;

        let state = test_app_state();
        advance_week_inner(&state).expect("advance_week");

        // Slot 0 (GK) of club index 0 → PlayerId(ROSTER_PLAYER_ID_BASE + 0).
        // That player is in the starting XI (index 0 of their club's Vec).
        let player_id_str = format!("fwh.core:player_{ROSTER_PLAYER_ID_BASE:08}");

        let dto = get_scout_report_inner(&player_id_str, &state)
            .expect("starting XI player must have a scout report after advance_week");

        assert!(
            !dto.overall_band.is_empty(),
            "overall_band must not be empty"
        );
        assert_eq!(
            dto.categories.len(),
            3,
            "categories must have 3 entries (Physical/Mental/Technical)"
        );
        assert_eq!(
            dto.observation_count, 1,
            "observation_count must be 1 after one advance_week"
        );
    }

    // ---- M2a: start_live_match_for_fixture determinism equivalence ----

    /// The watched result of a real fixture MUST be byte-identical to the
    /// AI-sim result for that fixture.
    ///
    /// Construction path verified:
    /// 1. `season::play_one_match(fixture_seed, …, SEASON_MATCH_TICK_BUDGET, slot_sigs)` —
    ///    the AI-sim reference path (same function called by `advance_week_inner`).
    /// 2. `start_live_match_for_fixture_inner(home, away, state)` — the live path.
    ///    Step to completion (SEASON_MATCH_TICK_BUDGET ticks) without any
    ///    in-match decisions.
    ///
    /// If this test fails, the two construction paths have diverged and the live
    /// session is NOT equivalent to the AI-sim — fix the construction, do NOT
    /// weaken the assertion.
    #[test]
    fn live_fixture_determinism_matches_ai_sim() {
        use crate::season::{SEASON_MATCH_TICK_BUDGET, build_slot_signatures};
        use crate::state::fixture_seed;

        let state = test_app_state();

        // Pick the first fixture in the league (deterministic, seed-derived).
        let career = state.career().read().expect("career lock");
        let first_fixture = career
            .season
            .league
            .fixtures
            .first()
            .copied()
            .expect("league must have at least one fixture");
        let home = first_fixture.home;
        let away = first_fixture.away;

        let fixture_idx = career
            .season
            .league
            .fixtures
            .iter()
            .position(|f| f.home == home && f.away == away)
            .expect("first fixture must exist in league.fixtures") as u32;

        let career_seed = state.career_seed();
        let seed = fixture_seed(career_seed, fixture_idx);

        let home_arch = career
            .season
            .tactical_archetype_ids
            .get(&home)
            .cloned()
            .expect("home club must have a tactical archetype");
        let away_arch = career
            .season
            .tactical_archetype_ids
            .get(&away)
            .cloned()
            .expect("away club must have a tactical archetype");

        let slot_sigs = {
            let home_vec = career.roster.get(&home);
            let away_vec = career.roster.get(&away);
            match (home_vec, away_vec) {
                (Some(h), Some(a)) => Some(build_slot_signatures(h.as_slice(), a.as_slice())),
                _ => None,
            }
        };

        drop(career);

        // --- AI-sim reference path (same as advance_week_inner) ---
        let (ai_outcome, ai_state) = season::play_one_match(
            seed,
            state.content(),
            state.signature_definitions(),
            &home_arch,
            &away_arch,
            SEASON_MATCH_TICK_BUDGET,
            slot_sigs,
        )
        .expect("play_one_match must succeed for a valid career fixture");

        // --- Live-match path ---
        let handle = start_live_match_for_fixture_inner(home.raw(), away.raw(), &state)
            .expect("start_live_match_for_fixture_inner must succeed");

        // Step to completion using exactly SEASON_MATCH_TICK_BUDGET ticks
        // (same budget as play_one_match). No decisions applied.
        let step = step_live_match_inner(handle.clone(), SEASON_MATCH_TICK_BUDGET, &state)
            .expect("step_live_match_inner must succeed");

        // Score equivalence.
        assert_eq!(
            step.score.home, ai_outcome.home_score,
            "live home_score must equal AI-sim home_score for the same fixture"
        );
        assert_eq!(
            step.score.away, ai_outcome.away_score,
            "live away_score must equal AI-sim away_score for the same fixture"
        );

        // Canonical-state byte equivalence: the watched match state must be
        // identical to the AI-simmed match state at the same tick count.
        let live_matches = state.live_matches().read().expect("live_matches lock");
        let session = live_matches
            .get(&handle.id)
            .expect("session must still be present");
        let live_canonical = session.state.encode_canonical();
        let ai_canonical = ai_state.encode_canonical();
        assert_eq!(
            live_canonical, ai_canonical,
            "canonical MatchState bytes must be identical between the live path \
             and the AI-sim path — if they differ, start_live_match_for_fixture \
             diverged from advance_week's construction"
        );
    }

    /// `start_live_match_for_fixture_inner` returns `ClubNotFound` when the
    /// home club ID is not in the current league.
    #[test]
    fn live_match_for_fixture_unknown_club_returns_club_not_found() {
        let state = test_app_state();
        let err = start_live_match_for_fixture_inner(0xFFFF_FFFF, 0xFFFF_FFFE, &state)
            .expect_err("must fail with unknown clubs");
        match err {
            IpcError::ClubNotFound { club_id } => {
                assert_eq!(
                    club_id, 0xFFFF_FFFF,
                    "should report the home club as not found"
                );
            }
            other => panic!("expected ClubNotFound, got {other:?}"),
        }
    }

    /// `start_live_match_for_fixture_inner` returns `LeagueGenerationFailed`
    /// when valid club IDs are supplied but no fixture exists between them
    /// (e.g. a team vs itself — valid clubs, absent fixture).
    #[test]
    fn live_match_for_fixture_no_fixture_between_clubs_returns_error() {
        let state = test_app_state();
        let career = state.career().read().expect("career lock");
        let first_club_id = career.season.league.clubs[0].id.raw();
        drop(career);

        // A club vs itself has no fixture in the schedule.
        let err = start_live_match_for_fixture_inner(first_club_id, first_club_id, &state)
            .expect_err("a team vs itself has no fixture");
        match err {
            IpcError::LeagueGenerationFailed { .. } => {}
            other => panic!("expected LeagueGenerationFailed, got {other:?}"),
        }
    }
}

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

use fw_content::{
    Fixture, MemoryCallbackContext, SeasonState, discriminant_to_family_key, generate_league,
    render_memory_callback,
};
use fw_core::{PlayerId, Seed, Tick};
use fw_match_sim::{MatchState, tick_match};
use fw_memory::event::{EventClass, SeasonNumber};
use fw_memory::readers::{SalienceFilter, salience::SalienceReader};

use crate::state::{fixture_seed, league_fixture_index};
use crate::{
    AdvanceSeasonSummaryDto, AdvanceWeekSummaryDto, AppState, BackendHandshakeDto,
    CareerOverviewDto, ChampionHistoryEntryDto, FixtureWithResultDto, IpcError,
    MAX_FRAMES_PER_REQUEST, MatchFrameDto, MatchResult, PlayFixturesSummaryDto, PlayerDetailDto,
    PlayerPhenotypeDto, SquadPlayerDto, StandingsRowDto, season,
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
    let results_snapshot = career.season.results.clone();

    for fixture in &fixtures {
        let idx = league_fixture_index(&career.season.league.fixtures, fixture) as u32;
        let seed = fixture_seed(career_seed, idx);
        let home_arch = career.season.tactical_archetype_ids[&fixture.home].clone();
        let away_arch = career.season.tactical_archetype_ids[&fixture.away].clone();
        match season::play_one_match(
            seed,
            state.content(),
            state.signature_definitions(),
            &home_arch,
            &away_arch,
            season::SEASON_MATCH_TICK_BUDGET,
        ) {
            Ok(outcome) => {
                career
                    .season
                    .apply_result(fixture.home, fixture.away, outcome);
            }
            Err(e) => {
                // Atomic rollback: restore results to the pre-loop snapshot
                // so the user-visible state matches the error semantic ("this
                // match-day did NOT advance"). current_match_day was never
                // bumped (the bump is after the loop), so no rollback needed
                // there.
                career.season.results = results_snapshot;
                return Err(e);
            }
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

pub fn get_player_detail_inner(
    player_id: &str,
    state: &AppState,
) -> Result<PlayerDetailDto, IpcError> {
    // 1. Look up the PlayerBio in the content store.
    let bio =
        state
            .content()
            .player_bios
            .get(player_id)
            .ok_or_else(|| IpcError::PlayerNotFound {
                player_id: player_id.to_string(),
            })?;

    // 2. Build the phenotype block.
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

    // 3. Memory callbacks — derive PlayerId from the `_NNNNN` suffix.
    //    Best-effort: the runtime ledger is empty so this matters only in
    //    fixture-ledger tests that control both sides. Unknown suffix → skip
    //    the salience query and return an empty callbacks vec.
    let numeric_player_id: Option<u32> = parse_player_id_suffix(player_id);

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

            // TODO(T4): wire the career clock — Tick::ZERO disables salience decay.
            let top_events: Vec<fw_memory::event::MemoryEvent> = SalienceReader::top_n(
                &mut career.ledger,
                5,
                SalienceFilter::BySubject(player_fw_id),
                Tick::ZERO,
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
                    player_name: bio.display_name_full.clone(),
                    club_name,
                    opponent_name,
                    competition_name: String::new(),
                    season_label,
                    score_line: String::new(),
                    outcome_phrase: String::new(),
                    role_label: bio.role_family.display_label().to_string(),
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
        log::warn!(
            "get_player_detail: could not derive numeric PlayerId from {:?}; \
             returning empty memoryCallbacks",
            player_id
        );
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
/// 3. Infallible mutations: emit season-end events, compact if at the 5-season
///    boundary, swap the season, increment season_number. All under the same write guard.
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
    if let Some(cid) = champion_club_id {
        season::emit_title_won_event(cid, current_season_num, &mut career.ledger);
    }

    let new_season_num = SeasonNumber(current_season_num.0 + 1);
    // At or past the 5-season boundary: compact with the NEW season number
    // so the boundary condition (event.season + 5 <= current_season)
    // correctly identifies events from 5+ seasons ago.
    let compaction_fired = if new_season_num.0 >= 5 {
        career.ledger.compact(new_season_num) > 0
    } else {
        false
    };

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
    let (current_season_num, club_names, past_title_events) = {
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

        (current_season_num, club_names, past_title_events)
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
                .unwrap_or_default();
            ChampionHistoryEntryDto {
                season: event.season.0,
                champion_club_name,
            }
        })
        .collect();
    history.sort_by_key(|e| e.season);

    // Render cross-season callbacks via the T3-6 render_memory_callback path.
    let bank = &state.content().memory_callback_grammars;
    let career_seed = state.career_seed().to_u64();

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
            let club_name = first_club_cid
                .and_then(|cid| club_names.get(&cid).cloned())
                .unwrap_or_default();

            let season_label = format!("Season {}", event.season.0 + 1);
            let ctx = MemoryCallbackContext {
                player_name: String::new(),
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
}

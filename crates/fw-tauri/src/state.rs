//! `AppState` — Tauri-managed resource constructed once at app startup.
//!
//! T1-5 closes the T1-11 fix-pass deferral: instead of reloading
//! `ContentStore` on every IPC command (~10ms per call), the store is
//! loaded once at boot and injected via `tauri::Builder::manage(AppState)`.
//! Command handlers receive `tauri::State<'_, AppState>` and read the
//! pre-loaded store without touching the filesystem.
//!
//! T2-5 adds `career_seed: Seed` + `season: RwLock<SeasonState>`. The
//! `RwLock` allows concurrent read-only commands (`get_standings`,
//! `get_fixtures`) without blocking each other, while single-writer
//! mutations (`advance_week`, `play_fixtures`) take the write lock.
//!
//! T3-9 fix-pass: Three separate `RwLock`s (`season`, `memory_ledger`,
//! `season_number`) are collapsed into one `RwLock<CareerState>`. This
//! makes `advance_season` atomic — the entire N→N+1 transition happens
//! under one write guard so there is no torn-read window between the
//! three fields.
//!
//! The `Arc<BTreeMap<...>>` for signature_definitions is extracted at
//! construction time — Arc-clone per command avoids re-borrowing the whole
//! ContentStore across the async command boundary.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use fw_content::{
    ContentLoadError, ContentStore, MATCH_DAYS_PER_SEASON, SeasonState, SignatureDefinition,
    generate_league_with_teams,
};
use fw_core::{ClubId, Seed, SeedLayer, Tick, seed_fn};
use fw_memory::event::SeasonNumber;
use fw_memory::ledger::MemoryLedger;

use crate::roster::{PlayerInstance, build_roster_from_league};

use crate::live_match::LiveMatchSession;
use crate::season::SEASON_MATCH_TICK_BUDGET;

/// The default career seed used when no explicit seed is provided.
///
/// `0xfeed_beef_cafe_fade` is arbitrary but memorable; production saves will
/// supply a random seed generated at career-creation time.
const DEFAULT_CAREER_SEED: u64 = 0xfeed_beef_cafe_fade;

/// All mutable career state. Held behind one `RwLock` in `AppState` so that
/// `advance_season` can mutate all three fields atomically under a single
/// write guard — no torn-read window between season, ledger, and season number.
///
/// Read commands (`get_standings`, `get_fixtures`) acquire a read lock.
/// Write commands (`advance_week`, `play_fixtures`, `advance_season`)
/// acquire the write lock.
pub struct CareerState {
    /// The current season's match schedule and results.
    pub season: SeasonState,
    /// Append-only event ledger. Written during `advance_season`; read by
    /// `get_player_detail` and `get_career_overview`.
    pub ledger: MemoryLedger,
    /// Ordinal of the current season. Starts at `SeasonNumber(0)`.
    /// Incremented by `advance_season`.
    pub season_number: SeasonNumber,
    /// Per-club player instances, keyed by `ClubId`.
    ///
    /// Populated at career start from the league's per-club `ProcGenTeam` +
    /// `PlayerTemplate` pool. Inner `Vec` is slot-ordered (GK=0, 22 entries
    /// per club). Mutable via breakthroughs (T4-2.5d), scouting (T4-2.5f),
    /// and season advance.
    ///
    /// `BTreeMap` for deterministic iteration (Sim/RULES.md §2).
    pub roster: BTreeMap<ClubId, Vec<PlayerInstance>>,

    /// Incremental evaluation watermark for breakthrough meter accumulation.
    ///
    /// Stores the `ledger.len()` value at the end of the previous
    /// `advance_season_inner` call. On the next call, only events in
    /// `ledger.events[watermark..]` are fed to `evaluate()` — events before
    /// the watermark have already been processed and their meter contributions
    /// are captured in each player's persisted `BreakthroughState`.
    ///
    /// This prevents historical events from re-accumulating meters every season,
    /// which would cause the same gating event to re-fire a breakthrough in every
    /// subsequent season (P0 fix, T4-2.5d self-review).
    ///
    /// Non-canonical: only used in the career system, not in `MatchState`.
    /// SaveV4 (T4-2.5g) will persist this alongside `roster` and `ledger`.
    /// On a career loaded from a SaveV3 save (before T4-2.5d), this defaults
    /// to 0 — the first `advance_season` after migration re-evaluates the full
    /// historical ledger once, then advances the watermark. That one-time
    /// re-evaluation is correct: no breakthroughs have fired yet in that save
    /// (the column was never wired), so the meters start from zero and the
    /// cooldown state is also zero. The results from that first evaluation are
    /// deterministic and valid.
    pub breakthrough_eval_watermark: usize,
}

impl CareerState {
    /// The current career clock — a monotonic `Tick` derived from how far the
    /// career has progressed: `season_number` complete seasons plus the current
    /// season's `current_match_day`, measured in the per-match `Tick` unit a
    /// played match uses (`SEASON_MATCH_TICK_BUDGET` ticks per match-day).
    ///
    /// Used as the `now_tick` argument to salience-decay projection
    /// (`SalienceReader::top_n`). Derived rather than stored so it cannot
    /// desync from `season_number` / `current_match_day`; it is monotonic
    /// non-decreasing and continuous across a season rollover (at
    /// season-N-complete `current_match_day == MATCH_DAYS_PER_SEASON + 1`,
    /// whose value equals season-(N+1)-day-1).
    pub fn current_tick(&self) -> Tick {
        let career_match_days = i64::from(self.season_number.0) * i64::from(MATCH_DAYS_PER_SEASON)
            + i64::from(self.season.current_match_day);
        Tick::from_raw(career_match_days * i64::from(SEASON_MATCH_TICK_BUDGET))
    }
}

/// Application-level state managed by Tauri.
///
/// T3-9 shape: content store (read-only after construction) + career seed +
/// all mutable career state behind a single `RwLock<CareerState>`.
///
/// T4-5a adds `live_matches` (active live-match sessions) and
/// `next_live_match_id` (lock-free handle allocation).
///
/// Fields are `pub(crate)` — external consumers go through the accessor
/// methods so the invariant "season mutations go through `SeasonState` API" is
/// doc-enforced, not just convention.
pub struct AppState {
    pub(crate) content: ContentStore,
    /// Arc-clone of `content.signature_definitions` for cheap per-command
    /// access without re-borrowing `content` across the async boundary.
    pub(crate) signature_definitions: Arc<BTreeMap<String, SignatureDefinition>>,
    /// The career seed used to generate the current league + fixture seeds.
    ///
    /// `AtomicU64` (not a bare `Seed`) so `new_career` / `load_career` can
    /// re-seed the career under `&AppState` — the Tauri-managed state is shared
    /// immutably, so interior mutability is the only seam. Lock-free (rather
    /// than moving the seed under the `career` lock) avoids the re-entrancy
    /// hazard at the call sites that read `career_seed()` and then take the
    /// `career` lock in the same handler.
    pub(crate) career_seed: AtomicU64,
    /// All mutable career state: season, ledger, season_number.
    ///
    /// One lock means `advance_season` is atomic — all three fields move
    /// together under one write guard, eliminating the torn-read window that
    /// the prior three-lock design had.
    ///
    /// Poison-error discipline: IPC handlers must map a poisoned lock to
    /// `IpcError::LockPoisoned { lock: "career".to_string() }` rather than
    /// `.expect()` (Tauri/RULES.md §4 forbids panics in handlers).
    pub(crate) career: RwLock<CareerState>,

    /// The club the player manages this session, if chosen.
    ///
    /// Set by `new_career` (cleared to `None`) and `select_managed_club`; read
    /// by `get_squad_roster` to anchor the Squad screen on the managed club
    /// rather than the lowest-`ClubId` placeholder. Session-only — NOT
    /// persisted by SaveV4; cross-save persistence is a flagged SaveV5 owner
    /// decision (a §8 schema bump). `None` until the player picks a club at
    /// career start.
    pub(crate) managed_club_id: RwLock<Option<ClubId>>,

    // ---- T4-6a: settings persistence ----
    /// Path to the settings file (`settings.fwcfg` in the Tauri app-config dir).
    ///
    /// Resolved at construction time. Production code passes the Tauri
    /// app-config dir; integration tests inject a temp-dir path so no live
    /// Tauri runtime is required.
    pub(crate) settings_path: PathBuf,

    // ---- T4-2.5g: career save persistence ----
    /// Path to the career save file (`career.fwsave` in the Tauri app-config dir).
    ///
    /// Resolved at construction time. Default: `./career.fwsave` relative to
    /// CWD. Production code (`main.rs`) overrides this with the Tauri app-config
    /// dir after construction — see `set_career_save_path`.
    pub(crate) career_save_path: PathBuf,

    // ---- T4-5a: live-match session store ----
    /// All currently active live-match sessions, keyed by handle ID.
    ///
    /// `BTreeMap` for deterministic iteration order (Sim/RULES.md §2 —
    /// although this map is non-canonical, keeping one rule avoids the
    /// "which DTO was canonical-feeding again?" confusion).
    ///
    /// Handlers that read (get_match_snapshot) take a read lock.
    /// Handlers that mutate (step, finish, apply_command) take a write lock.
    ///
    /// Poison-error discipline: same as `career` — map to `IpcError::LockPoisoned`.
    pub(crate) live_matches: RwLock<BTreeMap<u32, LiveMatchSession>>,

    /// Monotonically increasing counter for allocating live-match handle IDs.
    ///
    /// `AtomicU32` with `Ordering::Relaxed` is sufficient here: handle IDs
    /// only need to be unique (not sequentially ordered across threads), and
    /// the map insertion under the write lock provides the sequencing guarantee
    /// that prevents two concurrent `start_live_match` calls from overwriting
    /// each other.
    pub(crate) next_live_match_id: AtomicU32,
}

impl AppState {
    /// Construct `AppState` from a content root directory, using the default
    /// career seed.
    ///
    /// `generate_league` is called eagerly at construction time. It only fails
    /// on a malformed empty `ContentStore` (missing cultures/archetypes/managers)
    /// which `load_sources` already validated. A failure here means the
    /// content directory is corrupted post-load, so `expect` is the right
    /// escalation — not a fallible `Result` variant.
    ///
    /// `settings_path` defaults to `./settings.fwcfg` relative to the working
    /// directory. Production code (`main.rs`) overrides this with the Tauri
    /// app-config dir after construction — see `set_settings_path`.
    pub fn new(content_root: &Path) -> Result<Self, ContentLoadError> {
        Self::new_with_career_seed(content_root, Seed::from_u64(DEFAULT_CAREER_SEED))
    }

    /// Construct `AppState` with an explicit career seed.
    ///
    /// `settings_path` defaults to `./settings.fwcfg`. Integration tests that
    /// need a custom path should call `new_with_settings_path` instead.
    pub fn new_with_career_seed(
        content_root: &Path,
        career_seed: Seed,
    ) -> Result<Self, ContentLoadError> {
        Self::new_with_settings_path(content_root, career_seed, PathBuf::from("settings.fwcfg"))
    }

    /// Construct `AppState` with the default career seed and an explicit
    /// settings file path.
    ///
    /// This is the PRODUCTION constructor — `src-tauri/main.rs` resolves the
    /// Tauri app-config dir inside `.setup()` (where the `AppHandle` exists)
    /// and passes `<app-config-dir>/settings.fwcfg` here, so settings persist
    /// to the OS-correct location rather than the process working directory.
    pub fn new_with_settings_file(
        content_root: &Path,
        settings_path: PathBuf,
    ) -> Result<Self, ContentLoadError> {
        Self::new_with_settings_path(
            content_root,
            Seed::from_u64(DEFAULT_CAREER_SEED),
            settings_path,
        )
    }

    /// Construct `AppState` with an explicit career seed AND an explicit
    /// settings file path.
    ///
    /// This is the primary constructor used by integration tests that need to
    /// inject a temp-dir path so no live Tauri runtime is required.
    pub fn new_with_settings_path(
        content_root: &Path,
        career_seed: Seed,
        settings_path: PathBuf,
    ) -> Result<Self, ContentLoadError> {
        let content = ContentStore::load_sources(content_root)?;
        let signature_definitions = Arc::new(content.signature_definitions.clone());

        // generate_league_with_teams is a pure function of (seed, content).
        // A failure here means missing cultures/archetypes/managers —
        // load_sources already validated these are present, so expect is the
        // right escalation (true post-load content corruption, not user error).
        //
        // One procgen pass produces both the League (for SeasonState) and the
        // Vec<ProcGenTeam> (for build_roster_from_league), avoiding the double
        // call that a career_seed-only roster entry point would require.
        let (league, procgen_teams) = generate_league_with_teams(career_seed, &content).expect(
            "generate_league_with_teams must succeed on a valid ContentStore; \
             failure here means the content directory was corrupted post-load",
        );
        let roster = build_roster_from_league(&league, &procgen_teams, &content).expect(
            "build_roster_from_league must succeed on a valid ContentStore; \
             failure here means the content directory was corrupted post-load",
        );
        let season = SeasonState::new(league, &content);

        Ok(AppState {
            content,
            signature_definitions,
            career_seed: AtomicU64::new(career_seed.to_u64()),
            managed_club_id: RwLock::new(None),
            career: RwLock::new(CareerState {
                season,
                ledger: MemoryLedger::new(),
                season_number: SeasonNumber(0),
                roster,
                breakthrough_eval_watermark: 0,
            }),
            settings_path,
            career_save_path: PathBuf::from("career.fwsave"),
            live_matches: RwLock::new(BTreeMap::new()),
            next_live_match_id: AtomicU32::new(0),
        })
    }

    /// Override the settings file path after construction.
    ///
    /// Called by `main.rs` after the Tauri app-config directory is resolved —
    /// the Tauri API is not available until the builder runs, so we cannot
    /// resolve it during `AppState::new`.
    pub fn set_settings_path(&mut self, path: PathBuf) {
        self.settings_path = path;
    }

    /// Read-only reference to the settings file path.
    pub fn settings_path(&self) -> &PathBuf {
        &self.settings_path
    }

    /// Override the career save file path after construction.
    ///
    /// Called by `main.rs` after the Tauri app-config directory is resolved,
    /// alongside `set_settings_path`. See `AppState::set_settings_path` for
    /// the rationale.
    pub fn set_career_save_path(&mut self, path: PathBuf) {
        self.career_save_path = path;
    }

    /// Read-only reference to the career save file path.
    pub fn career_save_path(&self) -> &PathBuf {
        &self.career_save_path
    }

    /// Read-only access to the loaded [`ContentStore`].
    pub fn content(&self) -> &ContentStore {
        &self.content
    }

    /// Read-only access to the perf-cached signature definitions table.
    ///
    /// This is an `Arc`-clone of `content.signature_definitions` captured at
    /// construction time; it stays consistent because `AppState` is
    /// immutable after construction. Phase-2 modding (hot-reload) will need
    /// to replace the whole `AppState` rather than mutate it in place.
    pub fn signature_definitions(&self) -> &Arc<BTreeMap<String, SignatureDefinition>> {
        &self.signature_definitions
    }

    /// The career seed (T2-5). Used to derive per-fixture seeds so the same
    /// career seed always produces the same season results.
    pub fn career_seed(&self) -> Seed {
        Seed::from_u64(self.career_seed.load(Ordering::Relaxed))
    }

    /// Replace the career seed (interior mutability — see the field doc).
    ///
    /// Called by `new_career` (a fresh world) and `load_career` (the loaded
    /// save's seed, which may differ from the constructor's seed now that
    /// `new_career` exists). Both writers store the seed WHILE HOLDING the
    /// `career` write lock, so the seed and the season are re-seeded together.
    ///
    /// PAIRING INVARIANT: a reader that needs the seed to match the live
    /// `career` (e.g. deriving fixture seeds for the current world) MUST read
    /// `career_seed()` while holding a `career` guard — that excludes a
    /// concurrent re-seed for the read's duration. Readers that only need *a*
    /// seed (not paired with a specific season snapshot) may read it lock-free.
    /// `Relaxed` is sufficient: the `career` lock provides the happens-before
    /// edge for the paired case; the bare atomic carries no ordering of its own.
    pub fn set_career_seed(&self, seed: Seed) {
        self.career_seed.store(seed.to_u64(), Ordering::Relaxed);
    }

    /// The session's managed-club slot. Read/written via the lock, with the
    /// same poison discipline as [`AppState::career`] — handlers map a poisoned
    /// guard to `IpcError::LockPoisoned { lock: "managed_club_id" }`.
    pub fn managed_club_id(&self) -> &RwLock<Option<ClubId>> {
        &self.managed_club_id
    }

    /// Access to all mutable career state (season + ledger + season_number).
    ///
    /// Callers acquire read or write locks as appropriate.
    ///
    /// Per Tauri/RULES.md §4 ("Never panic in a handler"), do NOT `.expect()`
    /// on a poisoned lock from inside an IPC handler. Map the poison error to
    /// [`IpcError::LockPoisoned`] so the frontend can surface a structured
    /// "internal state corrupted — restart" message:
    ///
    /// ```ignore
    /// let career = state
    ///     .career()
    ///     .read()
    ///     .map_err(|_| IpcError::LockPoisoned { lock: "career".to_string() })?;
    /// ```
    pub fn career(&self) -> &RwLock<CareerState> {
        &self.career
    }

    /// Access to the live-match session store (T4-5a).
    ///
    /// Handlers that only read (get_match_snapshot) acquire a read lock.
    /// Handlers that mutate (step_live_match, finish_live_match,
    /// apply_match_command) acquire a write lock.
    ///
    /// Poison discipline: same as `career()` — map to
    /// `IpcError::LockPoisoned { lock: "live_matches".to_string() }`.
    pub fn live_matches(&self) -> &RwLock<BTreeMap<u32, LiveMatchSession>> {
        &self.live_matches
    }

    /// Allocate a new live-match handle ID (T4-5a).
    ///
    /// `Ordering::Relaxed` is sufficient — uniqueness is guaranteed by the
    /// fetch_add atomic increment; ordering relative to the subsequent map
    /// insertion is guaranteed by the write lock on `live_matches`.
    ///
    /// OVERFLOW: `fetch_add` wraps at `u32::MAX`. Reaching it requires ~4.3
    /// billion `start_live_match` calls within a single app process — a
    /// live match is a foregrounded, user-initiated session, so this is
    /// unreachable in practice. A wrap would only alias an id if a session
    /// allocated 2^32 starts ago were somehow still open; finished matches
    /// are removed from the map, so the realistic blast radius is nil. Wrap
    /// (vs panic) is the accepted behaviour here.
    pub fn alloc_live_match_id(&self) -> u32 {
        self.next_live_match_id.fetch_add(1, Ordering::Relaxed)
    }
}

/// Derive a per-fixture seed from the career seed + fixture index.
///
/// `fixture_index` is the 0-based position of the fixture in
/// `league.fixtures` (which is sorted deterministically by
/// `(match_day, home_id, away_id)`). Same `(career_seed, fixture_index)`
/// → same seed → same match outcome on every run.
///
/// Uses `SeedLayer::ContentBake` as the layer discriminant — this is a
/// content-generation operation (the league schedule is baked at career-init),
/// not a sim-tick decision. ADR-0009 §8 discriminants.
pub fn fixture_seed(career_seed: Seed, fixture_index: u32) -> Seed {
    Seed::from_u64(seed_fn(
        career_seed.to_u64(),
        fixture_index,
        SeedLayer::ContentBake,
        1, // site = 1 distinguishes fixture seeds from other ContentBake uses
    ))
}

/// Find the index of a fixture in `league.fixtures`.
///
/// `league.fixtures` is sorted by `(match_day, home_id, away_id)` (per
/// `generate_fixtures`'s final sort). Linear scan is acceptable — 380 items,
/// called at most 380 times per `play_fixtures` = 144_400 comparisons total,
/// negligible vs. 380 × 600 = 228_000 sim ticks.
///
/// Panics if the fixture is not found — callers supply fixtures from
/// `SeasonState::fixtures_for_match_day` which itself reads from
/// `league.fixtures`, so a not-found here means a logic bug in the caller.
pub fn league_fixture_index(
    fixtures: &[fw_content::Fixture],
    fixture: &fw_content::Fixture,
) -> Option<usize> {
    // `None` = a league-integrity violation (caller supplied a fixture not from
    // `fixtures_for_match_day`). Handler-context callers map it to an
    // `IpcError` rather than panicking (Tauri/RULES.md §4).
    fixtures.iter().position(|f| f == fixture)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn workspace_content_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("content")
    }

    #[test]
    fn new_loads_content_store_successfully() {
        let state = AppState::new(&workspace_content_path())
            .expect("AppState::new should succeed with workspace content/");
        // sig_definitions Arc shares the same entries as the store itself.
        assert_eq!(
            state.signature_definitions().len(),
            state.content().signature_definitions.len(),
        );
    }

    #[test]
    fn arc_clone_does_not_double_the_signatures() {
        let state = AppState::new(&workspace_content_path()).expect("AppState::new");
        let cloned = Arc::clone(state.signature_definitions());
        assert_eq!(cloned.len(), state.signature_definitions().len());
    }

    #[test]
    fn new_initialises_season_state_at_match_day_one() {
        let state = AppState::new(&workspace_content_path()).expect("AppState::new");
        let career = state.career().read().expect("career lock");
        assert_eq!(
            career.season.current_match_day, 1,
            "fresh season starts at match-day 1"
        );
        assert!(
            !career.season.is_complete(),
            "fresh season should not be complete"
        );
    }

    #[test]
    fn new_with_career_seed_is_deterministic() {
        let content_root = workspace_content_path();
        let seed = Seed::from_u64(0xDEAD_BEEF_CAFE_BABE);
        let state_a = AppState::new_with_career_seed(&content_root, seed).expect("state_a");
        let state_b = AppState::new_with_career_seed(&content_root, seed).expect("state_b");

        let career_a = state_a.career().read().expect("lock a");
        let career_b = state_b.career().read().expect("lock b");

        // Same seed → same first club name in the generated league.
        assert_eq!(
            career_a.season.league.clubs[0].display_name,
            career_b.season.league.clubs[0].display_name,
            "same career seed must produce the same league"
        );
    }

    #[test]
    fn fixture_seed_is_deterministic() {
        let career_seed = Seed::from_u64(0xCAFEBABE);
        let s0 = fixture_seed(career_seed, 0);
        let s0b = fixture_seed(career_seed, 0);
        assert_eq!(s0, s0b, "fixture_seed must be deterministic");
    }

    #[test]
    fn fixture_seed_differs_by_index() {
        let career_seed = Seed::from_u64(0xCAFEBABE);
        let s0 = fixture_seed(career_seed, 0);
        let s1 = fixture_seed(career_seed, 1);
        assert_ne!(
            s0, s1,
            "different fixture indices must produce different seeds"
        );
    }

    #[test]
    fn league_fixture_index_finds_first_fixture() {
        let content_root = workspace_content_path();
        let state = AppState::new(&content_root).expect("AppState::new");
        let career = state.career().read().expect("career lock");
        let first = &career.season.league.fixtures[0];
        let idx = league_fixture_index(&career.season.league.fixtures, first)
            .expect("first fixture must be present");
        assert_eq!(idx, 0);
    }

    // ---- T3-R-F: CareerState::current_tick() career clock ----

    /// A fresh career (season 0, match-day 1) has a non-zero career clock —
    /// the salience-decay `now_tick` is real, not the `Tick::ZERO` placeholder.
    #[test]
    fn current_tick_is_nonzero_for_fresh_career() {
        let state = AppState::new(&workspace_content_path()).expect("AppState::new");
        let career = state.career().read().expect("career lock");
        assert!(
            career.current_tick() > Tick::ZERO,
            "a fresh career (season 0, match-day 1) must have a non-zero career clock"
        );
    }

    /// The career clock advances by exactly `SEASON_MATCH_TICK_BUDGET` for each
    /// match-day, and strictly increases — it is monotonic.
    #[test]
    fn current_tick_advances_one_match_day_at_a_time() {
        let state = AppState::new(&workspace_content_path()).expect("AppState::new");
        let mut career = state.career().write().expect("career lock");

        let before = career.current_tick();
        career.season.current_match_day += 1;
        let after = career.current_tick();

        assert!(after > before, "career clock must strictly increase");
        assert_eq!(
            after.to_raw() - before.to_raw(),
            i64::from(SEASON_MATCH_TICK_BUDGET),
            "one match-day must advance the career clock by SEASON_MATCH_TICK_BUDGET",
        );
    }

    /// The career clock is continuous across a season rollover: a completed
    /// season at `current_match_day == MATCH_DAYS_PER_SEASON + 1` has the same
    /// career tick as the next season at match-day 1. No jump, no regression.
    #[test]
    fn current_tick_is_continuous_across_season_rollover() {
        let state = AppState::new(&workspace_content_path()).expect("AppState::new");
        let mut career = state.career().write().expect("career lock");

        // Season N complete: match-day advanced past the last played day.
        career.season_number = SeasonNumber(3);
        career.season.current_match_day = MATCH_DAYS_PER_SEASON + 1;
        let end_of_season = career.current_tick();

        // Season N+1, match-day 1 (the post-rollover state).
        career.season_number = SeasonNumber(4);
        career.season.current_match_day = 1;
        let start_of_next = career.current_tick();

        assert_eq!(
            end_of_season, start_of_next,
            "the career clock must be continuous across a season rollover"
        );
    }
}

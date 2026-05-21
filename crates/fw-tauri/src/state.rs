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
use std::path::Path;
use std::sync::{Arc, RwLock};

use fw_content::{
    ContentLoadError, ContentStore, SeasonState, SignatureDefinition, generate_league,
};
use fw_core::{Seed, SeedLayer, seed_fn};
use fw_memory::event::SeasonNumber;
use fw_memory::ledger::MemoryLedger;

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
}

/// Application-level state managed by Tauri.
///
/// T3-9 shape: content store (read-only after construction) + career seed +
/// all mutable career state behind a single `RwLock<CareerState>`.
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
    pub(crate) career_seed: Seed,
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
    pub fn new(content_root: &Path) -> Result<Self, ContentLoadError> {
        Self::new_with_career_seed(content_root, Seed::from_u64(DEFAULT_CAREER_SEED))
    }

    /// Construct `AppState` with an explicit career seed.
    ///
    /// Used by integration tests to supply a known seed for determinism checks.
    pub fn new_with_career_seed(
        content_root: &Path,
        career_seed: Seed,
    ) -> Result<Self, ContentLoadError> {
        let content = ContentStore::load_sources(content_root)?;
        let signature_definitions = Arc::new(content.signature_definitions.clone());

        // generate_league is a pure function of (seed, content). A failure
        // here means missing cultures/archetypes/managers — load_sources
        // already validated these are present, so this panic is a true
        // post-load content corruption, not a user-visible error path.
        let league = generate_league(career_seed, &content).expect(
            "generate_league must succeed on a valid ContentStore; \
             failure here means the content directory was corrupted post-load",
        );
        let season = SeasonState::new(league, &content);

        Ok(AppState {
            content,
            signature_definitions,
            career_seed,
            career: RwLock::new(CareerState {
                season,
                ledger: MemoryLedger::new(),
                season_number: SeasonNumber(0),
            }),
        })
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
        self.career_seed
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
) -> usize {
    fixtures
        .iter()
        .position(|f| f == fixture)
        .expect("fixture must exist in league.fixtures; caller supplied a fixture from fixtures_for_match_day")
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
        let idx = league_fixture_index(&career.season.league.fixtures, first);
        assert_eq!(idx, 0);
    }
}

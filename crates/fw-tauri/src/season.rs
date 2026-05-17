//! Season-play orchestration glue.
//!
//! `play_one_match` wraps the existing `tick_match` loop for a single fixture,
//! translating the sim's canonical types to `MatchOutcome` for `SeasonState`.
//! This is a sync function — it sits inside async Tauri handlers but runs
//! entirely on the calling thread (the sim is sync per `Sim/RULES.md §5`).
//!
//! `std::time::Instant` is only used in the integration-test perf gate (in
//! `crates/fw-tauri/tests/season_commands_test.rs`), not here — this module
//! is on the correct side of the clock-ban boundary (`fw-tauri` is allowed
//! clocks per `Sim/RULES.md §3`; this file is in `fw-tauri`).

use std::collections::BTreeMap;
use std::sync::Arc;

use fw_content::{ContentStore, MatchOutcome, SignatureDefinition};
use fw_core::Seed;
use fw_match_sim::{MatchState, tick_match};

use crate::IpcError;

/// Per-match tick budget for season simulation.
///
/// 600 ticks matches the existing 600-tick canonical hash pin (the
/// "extended" pinned scenario in `crates/fw-replay/tests/canonical_hash.rs`).
/// This keeps the season sim consistent with the already-verified canonical
/// state. Real 90-minute match realism is deferred to later work.
pub const SEASON_MATCH_TICK_BUDGET: u32 = 600;

/// Run one full match and return the final `MatchOutcome`.
///
/// Sync — the calling async handler is responsible for not blocking the Tauri
/// runtime; `play_fixtures` fast-forwards via a plain loop rather than
/// spawning threads (the sim is deterministic + single-threaded per design).
///
/// `sig_defs` is `Arc<BTreeMap<...>>` to allow cheap cloning per call without
/// re-borrowing the full `ContentStore` across the async boundary.
pub fn play_one_match(
    seed: Seed,
    content: &ContentStore,
    sig_defs: &Arc<BTreeMap<String, SignatureDefinition>>,
    home_archetype_id: &str,
    away_archetype_id: &str,
    tick_budget: u32,
) -> Result<MatchOutcome, IpcError> {
    let mut sim_state =
        MatchState::initial_with_content(seed, content, home_archetype_id, away_archetype_id)
            .map_err(|e| IpcError::MatchInitFailed {
                reason: e.to_string(),
            })?;
    for _ in 0..tick_budget {
        sim_state = tick_match(sim_state, sig_defs);
    }
    Ok(MatchOutcome {
        home_score: sim_state.home_score,
        away_score: sim_state.away_score,
    })
}

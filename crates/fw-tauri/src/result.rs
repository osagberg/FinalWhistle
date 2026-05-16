//! `MatchResult` + `Score` — the IPC return type for `play_match`.
//!
//! This is a one-way projection (DTO) from `MatchState`. It is NEVER
//! serialized back into canonical state (Tauri/RULES.md §3).

use fw_content::{CommentaryRenderError, ContentStore, render_event};
use fw_match_sim::MatchState;
use serde::{Deserialize, Serialize};

use crate::error::IpcError;

/// Per-event commentary line included in the `MatchResult` for immediate
/// rendering at T1-6 without a second IPC round-trip.
///
/// One entry per `MatchEvent` in `match_events`. On render failure the
/// entry is the fallback string `"(commentary unavailable)"` — never absent.
pub type CommentaryLine = String;

/// Home/away score pair.
///
/// `u8` mirrors `MatchState::home_score` / `MatchState::away_score` exactly —
/// no widening at the IPC boundary (type-design F6). Realistic football
/// scorelines stay well below 255 per side.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Score {
    pub home: u8,
    pub away: u8,
}

/// Full match result returned by `play_match`.
///
/// Shape mirrors what the TypeScript `MatchResult` type in `lib/types.ts`
/// expects. Serde camelCase on both sides (Tauri/RULES.md §3).
///
/// `commentary_preview` is pre-rendered prose for every event so T1-6's
/// Match page can display a text recap without an extra round-trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchResult {
    pub final_score: Score,
    /// `"blake3:<64-hex-chars>"` — BLAKE3 digest of `encode_canonical()`.
    pub canonical_hash: String,
    pub match_events: Vec<fw_content::MatchEvent>,
    /// Echo of the caller-supplied seed_hex for round-trip verification.
    pub seed_hex: String,
    pub tick_count: u32,
    /// One rendered prose line per entry in `match_events`. Length == `match_events.len()`.
    pub commentary_preview: Vec<CommentaryLine>,
}

impl MatchResult {
    /// Project `MatchState` into a `MatchResult` DTO.
    ///
    /// `canonical_hash` is computed via BLAKE3 over `state.encode_canonical()`.
    /// `commentary_preview` renders each event via `render_event`; on
    /// `CommentaryRenderError` the slot contains `"(commentary unavailable)"`.
    pub fn from_state(
        state: &MatchState,
        seed_hex: String,
        tick_count: u32,
        content: &ContentStore,
    ) -> Result<Self, IpcError> {
        let canonical_bytes = state.encode_canonical();
        let hash_bytes: [u8; 32] = blake3::hash(&canonical_bytes).into();
        let canonical_hash = format!("blake3:{}", hex_string(&hash_bytes));

        let events: Vec<fw_content::MatchEvent> = state.match_events().to_vec();

        let match_seed = state.seed.to_u64();
        // Render each event's commentary. On failure, fall back to the
        // "(commentary unavailable)" sentinel string AND record the failure
        // for aggregate logging — silent swallow would mask systematic
        // template-bank drift (T1-5 silent-failure audit P1).
        let mut render_failures: u32 = 0;
        let mut first_render_error: Option<CommentaryRenderError> = None;
        let commentary_preview: Vec<CommentaryLine> = events
            .iter()
            .map(
                |ev| match render_event(ev, match_seed, &content.commentary_grammars) {
                    Ok(line) => line,
                    Err(e) => {
                        render_failures += 1;
                        if first_render_error.is_none() {
                            first_render_error = Some(e);
                        }
                        "(commentary unavailable)".to_string()
                    }
                },
            )
            .collect();
        if render_failures > 0 {
            log::warn!(
                "MatchResult::from_state: {render_failures}/{total} commentary render failures \
                 (first error: {first:?})",
                total = events.len(),
                first = first_render_error,
            );
        }

        Ok(MatchResult {
            final_score: Score {
                home: state.home_score,
                away: state.away_score,
            },
            canonical_hash,
            match_events: events,
            seed_hex,
            tick_count,
            commentary_preview,
        })
    }
}

/// Format a 32-byte slice as a lowercase 64-hex-char string.
fn hex_string(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use fw_core::Seed;
    use fw_match_sim::{MatchState, tick_match};

    use super::*;

    fn workspace_content_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("content")
    }

    fn load_content() -> ContentStore {
        ContentStore::load_sources(&workspace_content_path()).expect("content load")
    }

    #[test]
    fn hex_string_produces_64_chars() {
        let bytes = [0u8; 32];
        assert_eq!(hex_string(&bytes).len(), 64);
    }

    #[test]
    fn canonical_hash_has_blake3_prefix() {
        let content = load_content();
        let seed = Seed::from_u64(1);
        let state = MatchState::initial(seed);
        let result =
            MatchResult::from_state(&state, "0x1".to_string(), 0, &content).expect("from_state");
        assert!(
            result.canonical_hash.starts_with("blake3:"),
            "canonical_hash must start with 'blake3:'"
        );
        assert_eq!(
            result.canonical_hash.len(),
            7 + 64,
            "canonical_hash length must be 7 (prefix) + 64 (hex)"
        );
    }

    #[test]
    fn canonical_hash_matches_independent_computation() {
        let content = load_content();
        let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);
        let mut state =
            MatchState::initial_with_content(seed, &content).expect("initial_with_content");
        for _ in 0..60 {
            state = tick_match(state, &content.signature_definitions);
        }

        let result =
            MatchResult::from_state(&state, "0xdeadbeefdeadbeef".to_string(), 60, &content)
                .expect("from_state");

        // Re-compute independently.
        let bytes = state.encode_canonical();
        let hash_bytes: [u8; 32] = blake3::hash(&bytes).into();
        let expected = format!(
            "blake3:{}",
            hash_bytes
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );

        assert_eq!(result.canonical_hash, expected);
    }

    #[test]
    fn commentary_preview_length_matches_events_length() {
        let content = load_content();
        let seed = Seed::from_u64(0xDEAD_BEEF_DEAD_BEEF);
        let mut state =
            MatchState::initial_with_content(seed, &content).expect("initial_with_content");
        for _ in 0..60 {
            state = tick_match(state, &content.signature_definitions);
        }
        let result =
            MatchResult::from_state(&state, "0xdeadbeefdeadbeef".to_string(), 60, &content)
                .expect("from_state");
        assert_eq!(
            result.commentary_preview.len(),
            result.match_events.len(),
            "commentary_preview must have one entry per match event"
        );
    }

    #[test]
    fn seed_hex_and_tick_count_are_echoed() {
        let content = load_content();
        let seed = Seed::from_u64(42);
        let state = MatchState::initial(seed);
        let result =
            MatchResult::from_state(&state, "0x2a".to_string(), 0, &content).expect("from_state");
        assert_eq!(result.seed_hex, "0x2a");
        assert_eq!(result.tick_count, 0);
    }
}

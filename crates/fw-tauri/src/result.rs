//! `MatchResult` + `Score` — the IPC return type for `play_match`.
//!
//! This is a one-way projection (DTO) from `MatchState`. It is NEVER
//! serialized back into canonical state (Tauri/RULES.md §3).

use fw_content::{CommentaryRenderError, ContentStore, MatchEvent, render_event};
use fw_match_sim::MatchState;
use serde::{Deserialize, Serialize};

use crate::error::IpcError;

/// Provisional tick-to-game-minute scale.
///
/// Codex 2026-05-16 Tier-2 fix-pass: the prior `MatchResult.match_events`
/// shipped `Vec<fw_content::MatchEvent>` which serde-derived externally as
/// `{ "KickOff": {...} }` — incompatible with the frontend's
/// `{ tick, minute, kind, description }` interface, breaking the production
/// render path despite green tests (tests mocked the frontend shape).
///
/// `minute = tick / TICKS_PER_GAME_MINUTE`. The current value (60) matches
/// the T1-6 mock convention (5400 ticks → minute 90) used for visible demo
/// content. The real sim runs at 60Hz (`fw_core::tick::TICKS_PER_SECOND`),
/// which would mean 90-min match = 324_000 ticks; the demo collapses that
/// to 5_400 ticks for fast iteration. T1-9 calibration pins the real
/// conversion + reconciles the two scales.
const TICKS_PER_GAME_MINUTE: i64 = 60;

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

/// Flattened per-event DTO matching the TypeScript `MatchEvent` interface
/// in `frontend/src/lib/types.ts`.
///
/// Codex 2026-05-16 Tier-2 fix-pass: the prior shape shipped raw
/// `fw_content::MatchEvent` (an externally-tagged enum that serde encodes
/// as `{ "KickOff": {...} }`) — incompatible with the frontend's flat
/// `{ tick, minute, kind, description }` interface. This DTO projects
/// every variant into a uniform shape the frontend can render directly.
///
/// `kind` is the PascalCase variant name (`"KickOff"` / `"Goal"` / `"Shot"`
/// / `"Pass"` / `"FullTime"` / `"SignatureFirstFired"`) matching the
/// frontend's closed `MatchEventKind` union AS-IS.
///
/// `description` is `None` for now (T1-6 renders commentary separately via
/// `commentary_preview`); T2 can attach short per-event labels here if a
/// second prose surface emerges.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchEventDto {
    /// Sim tick at which the event fired (60-tick-per-second canonical clock).
    pub tick: i64,
    /// Game-minute marker for UI display (`tick / TICKS_PER_GAME_MINUTE`).
    pub minute: u32,
    /// PascalCase event-kind discriminant name. Matches the frontend's
    /// closed `MatchEventKind` union exactly so a Rust-side variant
    /// addition that's missing on the TS side fails at TS compile time
    /// (frontend `eventLabel` / `badgeClass` switches with `never` defaults).
    pub kind: String,
    /// Optional short per-event label. `None` today; T2 may populate.
    ///
    /// `#[serde(skip_serializing_if)]` so absent descriptions are OMITTED
    /// from the JSON (not serialized as `null`) — matches the frontend's
    /// `description?: string` optional-field shape exactly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl MatchEventDto {
    /// Project a `fw_content::MatchEvent` into the flat IPC shape.
    ///
    /// Uses the existing `MatchEvent::discriminant()` enum (already pinned
    /// stable for canonical encoding) for the kind name — single source of
    /// truth for variant naming across the IPC boundary.
    pub fn from_match_event(event: &MatchEvent) -> Self {
        let tick = match event {
            MatchEvent::KickOff { tick, .. }
            | MatchEvent::FullTime { tick, .. }
            | MatchEvent::Goal { tick, .. }
            | MatchEvent::Shot { tick, .. }
            | MatchEvent::Pass { tick, .. }
            | MatchEvent::SignatureFirstFired { tick, .. } => tick.to_raw(),
        };
        let minute = tick.div_euclid(TICKS_PER_GAME_MINUTE).max(0) as u32;
        MatchEventDto {
            tick,
            minute,
            kind: format!("{:?}", event.discriminant()),
            description: None,
        }
    }
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
    /// Flat per-event DTOs. Codex 2026-05-16 Tier-2 fix-pass: was
    /// `Vec<fw_content::MatchEvent>` which serialised as externally-tagged
    /// `{ "KickOff": {...} }` and broke the frontend renderer; now
    /// `Vec<MatchEventDto>` matching the frontend's flat interface.
    pub match_events: Vec<MatchEventDto>,
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

        let raw_events: &[MatchEvent] = state.match_events();
        let events: Vec<MatchEventDto> = raw_events
            .iter()
            .map(MatchEventDto::from_match_event)
            .collect();

        let match_seed = state.seed.to_u64();
        // Render each event's commentary. On failure, fall back to the
        // "(commentary unavailable)" sentinel string AND record the failure
        // for aggregate logging — silent swallow would mask systematic
        // template-bank drift (T1-5 silent-failure audit P1).
        let mut render_failures: u32 = 0;
        let mut first_render_error: Option<CommentaryRenderError> = None;
        let commentary_preview: Vec<CommentaryLine> = raw_events
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
                total = raw_events.len(),
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
        let mut state = MatchState::initial_with_content(
            seed,
            &content,
            fw_match_sim::DEFAULT_ARCHETYPE_ID,
            fw_match_sim::DEFAULT_ARCHETYPE_ID,
        )
        .expect("initial_with_content");
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
        let mut state = MatchState::initial_with_content(
            seed,
            &content,
            fw_match_sim::DEFAULT_ARCHETYPE_ID,
            fw_match_sim::DEFAULT_ARCHETYPE_ID,
        )
        .expect("initial_with_content");
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

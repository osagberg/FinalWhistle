//! `IpcError` — typed error returned by all fw-tauri command handlers.
//!
//! Serialized as a discriminated union so the TypeScript frontend can
//! pattern-match on the `kind` field:
//!
//! ```json
//! { "kind": "tooManyFrames", "requested": 7201, "max": 7200 }
//! { "kind": "invalidSeed", "input": "0xggg", "reason": "invalid digit" }
//! { "kind": "matchInitFailed", "reason": "ContentStore load failure: ..." }
//! ```

use fw_content::ContentLoadError;
use fw_match_sim::ContentInitError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Typed IPC error. All command handlers return `Result<T, IpcError>`.
///
/// `#[serde(tag = "kind", rename_all = "camelCase")]` produces the
/// discriminated union shape the TypeScript side reads as:
/// `IpcError = { kind: "tooManyFrames"; requested: number; max: number } | ...`
#[derive(Debug, Clone, Serialize, Deserialize, Error)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum IpcError {
    /// Requested more frames than the per-request cap allows.
    ///
    /// Frontend: validate before invoking to avoid this variant; see
    /// `MAX_FRAMES_PER_REQUEST` in `lib.rs`.
    #[error("requested {requested} frames but max per request is {max}")]
    TooManyFrames { requested: u32, max: u32 },

    /// `seed_hex` argument could not be parsed as a u64 hex value.
    #[error("invalid seed_hex {input:?}: {reason}")]
    InvalidSeed { input: String, reason: String },

    /// The sim failed to initialise — typically a ContentStore problem.
    #[error("match init failed: {reason}")]
    MatchInitFailed { reason: String },

    /// `advance_week` was called after the season was already complete.
    #[error("season is already complete; cannot advance further")]
    SeasonComplete,

    /// `advance_season` was called before the current season is finished.
    ///
    /// The caller must complete all match-days (via `advance_week` or
    /// `play_fixtures`) before advancing to the next season.
    #[error("season is not yet complete; play all match-days before advancing")]
    SeasonNotComplete,

    /// `generate_league` failed during `advance_season`.
    ///
    /// This should be unreachable after a valid `AppState` construction —
    /// `generate_league` only fails when `ContentStore` is missing required
    /// cultures, archetypes, or managers, all of which `load_sources` validated.
    /// Surfaced here rather than `.expect()`-ing because Tauri/RULES.md §4
    /// forbids panics in handlers.
    #[error("league generation failed: {reason}")]
    LeagueGenerationFailed { reason: String },

    /// `get_fixtures` was called with a club ID that does not exist in the
    /// current league.
    ///
    /// Post-T2-5 code-reviewer P0 fix: named-field variant (was tuple
    /// `ClubNotFound(u32)`) so the wire shape is the clean
    /// `{ kind: "clubNotFound", clubId: N }` rather than the ugly
    /// `{ kind: "clubNotFound", "0": N }` that serde produces for tuple
    /// variants under `#[serde(tag = "kind")]`. The TS `IpcError` union
    /// can pattern-match on `clubId` cleanly.
    #[error("club id {club_id} not found in current league")]
    ClubNotFound { club_id: u32 },

    /// A player ID was not found in the content store's `player_bios` map.
    ///
    /// Named-field variant so the wire shape is:
    /// `{ kind: "playerNotFound", playerId: "fwh.core:player_00042" }` — clean
    /// TS pattern-match on `playerId` (string, content-pack-qualified).
    #[error("player id {player_id:?} not found in content store")]
    PlayerNotFound {
        #[serde(rename = "playerId")]
        player_id: String,
    },

    /// No scouting report is available yet for this player.
    ///
    /// Returned by `get_scout_report` when `PlayerInstance.last_scout_report`
    /// is `None` — the player has not yet featured in a match-day since career
    /// start. Scouts form their read once a player takes the field.
    ///
    /// Wire shape: `{ kind: "notYetObserved", playerId: "..." }`.
    #[error(
        "no scouting report yet for {player_id} — the player must feature in a match \
         before scouts can form a read"
    )]
    NotYetObserved {
        #[serde(rename = "playerId")]
        player_id: String,
    },

    /// A `MatchCommand` was received and recorded, but its implementation has
    /// not yet been wired up in the sim layer.
    ///
    /// All 9 `MatchCommand` variants return this error at T4-5a — the command
    /// is deserialized + stored in the session's audit trail, then rejected
    /// here. The `command_kind` field carries the camelCase discriminant string
    /// (e.g. `"substitute"`) so the frontend can display a targeted message.
    ///
    /// Wire shape: `{ kind: "liveMatchCommandUnimplemented", commandKind: "substitute" }`.
    /// `commandKind` is used (not a second `kind`) to avoid the field-name
    /// collision that `#[serde(tag = "kind")]` would produce if both the outer
    /// tag and the inner field were named `kind`.
    #[error("live-match command not yet implemented (commandKind: {command_kind})")]
    LiveMatchCommandUnimplemented {
        #[serde(rename = "commandKind")]
        command_kind: String,
    },

    /// Settings file exists but could not be decoded — likely corrupted.
    ///
    /// A missing settings file is NOT this variant — missing → first-run
    /// defaults (see `get_settings_inner`). This variant fires only when the
    /// file is present but malformed (bad bincode, unknown future version, or
    /// a splice/truncation).
    ///
    /// Wire shape: `{ kind: "settingsLoadFailed", reason: "..." }`.
    #[error("settings load failed: {reason}")]
    SettingsLoadFailed { reason: String },

    /// Career save file could not be written or read/decoded.
    ///
    /// Covers both I/O errors (disk full, permission denied) and decode errors
    /// (corrupted save file, unknown future discriminant). Distinct from
    /// `SettingsLoadFailed` so the frontend can surface "your save data could
    /// not be read" vs "your settings could not be read" with targeted messages.
    ///
    /// Wire shape: `{ kind: "saveLoadFailed", reason: "..." }`.
    ///
    /// NOTE: The TypeScript `IpcError` union mirror does NOT yet include this
    /// variant (out of scope for T4-2.5g per spec). Add `saveLoadFailed` to
    /// `frontend/src/lib/types.ts` when the first frontend surface calls
    /// `save_career` / `load_career`.
    #[error("career save failed: {reason}")]
    SaveLoadFailed { reason: String },

    /// An internal `RwLock` / `Mutex` was poisoned by a prior writer panic.
    ///
    /// Post-T2-5 silent-failure-hunter P1-3 fix: prior code used
    /// `.expect("season RwLock poisoned")` everywhere, which converted a
    /// poisoned-lock state into a handler PANIC. Per `Tauri/RULES.md §4`
    /// ("Never panic in a handler. Map all errors to `IpcError` variants."),
    /// poisoning must become a structured error the frontend can surface.
    /// `lock` names the field (e.g. `"season"`).
    ///
    /// `lock` is `String` not `&'static str` so `IpcError` continues to
    /// implement `Deserialize<'de>` for any `'de` lifetime — needed by the
    /// `round_trip_deserialize_preserves_fields` test which roundtrips
    /// through a runtime-allocated JSON string. Callers construct with
    /// `lock: "season".to_string()` from a static literal.
    #[error("internal lock {lock:?} poisoned by prior panic — app restart required")]
    LockPoisoned { lock: String },
}

impl From<ContentLoadError> for IpcError {
    fn from(e: ContentLoadError) -> Self {
        IpcError::MatchInitFailed {
            reason: e.to_string(),
        }
    }
}

impl From<ContentInitError> for IpcError {
    fn from(e: ContentInitError) -> Self {
        IpcError::MatchInitFailed {
            reason: e.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The discriminated-union wire shape must decode on the TS side as
    /// `{ kind: "tooManyFrames", requested: 7201, max: 7200 }`.
    #[test]
    fn too_many_frames_serializes_as_discriminated_union() {
        let err = IpcError::TooManyFrames {
            requested: 7201,
            max: 7200,
        };
        let json = serde_json::to_string(&err).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(v["kind"], "tooManyFrames", "kind discriminant wrong");
        assert_eq!(v["requested"], 7201_u32);
        assert_eq!(v["max"], 7200_u32);
    }

    #[test]
    fn invalid_seed_serializes_as_discriminated_union() {
        let err = IpcError::InvalidSeed {
            input: "0xggg".to_string(),
            reason: "invalid digit".to_string(),
        };
        let json = serde_json::to_string(&err).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(v["kind"], "invalidSeed");
        assert_eq!(v["input"], "0xggg");
    }

    #[test]
    fn round_trip_deserialize_preserves_fields() {
        let err = IpcError::TooManyFrames {
            requested: 9999,
            max: 7200,
        };
        let json = serde_json::to_string(&err).expect("serialize");
        let back: IpcError = serde_json::from_str(&json).expect("deserialize");
        match back {
            IpcError::TooManyFrames { requested, max } => {
                assert_eq!(requested, 9999);
                assert_eq!(max, 7200);
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn display_impl_is_human_readable() {
        let err = IpcError::TooManyFrames {
            requested: 7201,
            max: 7200,
        };
        let s = err.to_string();
        assert!(s.contains("7201"), "display should mention requested count");
        assert!(s.contains("7200"), "display should mention max");
    }
}

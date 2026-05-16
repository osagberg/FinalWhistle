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

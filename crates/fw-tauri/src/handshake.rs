//! Backend handshake — the smallest possible "is the Rust sim alive?" IPC
//! surface for the frontend Home page's liveness check.
//!
//! Codex 2026-05-16 Tier-2 fix-pass: T1-5 deleted `src-tauri/src/commands.rs`
//! (which had a local `get_dummy_state` returning `{ appVersion, message,
//! backendReady }`) AND consolidated `get_dummy_state` into fw-tauri where it
//! returns `MatchStateDto`. The frontend `Home.tsx` still expects the
//! handshake shape, so its `tauri.ts` wrapper became out-of-sync with the
//! actual command return. This module restores a typed handshake surface
//! that Home.tsx can consume directly; the old `get_dummy_state` command
//! and its wrapper are deleted.

use serde::{Deserialize, Serialize};

/// Payload returned by `get_backend_handshake`. Mirrors the frontend's
/// `BackendHandshake` interface in `frontend/src/lib/types.ts`.
///
/// `#[serde(rename_all = "camelCase")]` so TS sees `appVersion` /
/// `backendReady` (Tauri/RULES.md §3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendHandshakeDto {
    /// Backend crate version (read from `CARGO_PKG_VERSION` at build time).
    pub app_version: String,
    /// Human-readable status message.
    pub message: String,
    /// `true` if the backend is fully wired (returned from a live IPC call);
    /// `false` is the frontend's stub-mode sentinel and never produced here.
    pub backend_ready: bool,
}

impl BackendHandshakeDto {
    /// Build a live handshake payload. `backend_ready` is hard-coded `true`
    /// because reaching this code means a Tauri IPC round-trip succeeded.
    #[must_use]
    pub fn live() -> Self {
        BackendHandshakeDto {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            message: "Backend live.".to_string(),
            backend_ready: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_returns_backend_ready_true() {
        let h = BackendHandshakeDto::live();
        assert!(h.backend_ready);
        assert!(!h.app_version.is_empty());
        assert!(!h.message.is_empty());
    }

    #[test]
    fn serializes_as_camel_case() {
        let h = BackendHandshakeDto::live();
        let json = serde_json::to_string(&h).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        // The exact wire shape the TS frontend reads.
        assert!(v.get("appVersion").is_some(), "appVersion missing: {json}");
        assert!(v.get("message").is_some(), "message missing: {json}");
        assert!(
            v.get("backendReady").is_some(),
            "backendReady missing: {json}"
        );
        assert_eq!(v["backendReady"], serde_json::Value::Bool(true));
    }
}

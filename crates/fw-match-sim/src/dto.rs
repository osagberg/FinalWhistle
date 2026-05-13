//! Frontend-facing DTO for per-tick match state.
//!
//! The dev-tier 2D tactical board (T1-2a per ADR-0007 Layer 2 +
//! ADR-0008) consumes a stream of `MatchFrameDto` records — one per
//! integration tick — and renders 22 dots + ball + scrubber.
//!
//! Two producers feed this shape:
//!
//! 1. `fw_tauri::commands::match_frames` — the Tauri IPC path; returns
//!    `Vec<MatchFrameDto>` for a `(seed, tick_count)` request.
//! 2. `crates/fw-match-sim/src/bin/dump_frames.rs` — the browser-dev
//!    path (per ADR-0008); writes the same shape to stdout as JSON
//!    for the `HttpFrameSource` to fetch.
//!
//! ## Why this lives in `fw-match-sim`, not `fw-tauri`
//!
//! The `dump_frames` binary needs the projection without pulling in
//! `fw-tauri` (which would create a circular dep — fw-tauri already
//! depends on fw-match-sim). The DTO is pure data; the projection is
//! the only float arithmetic. We isolate that arithmetic in this
//! module with a local `#![allow(clippy::float_arithmetic)]` instead
//! of relaxing the crate-wide lint.
//!
//! ## Float boundary
//!
//! `MatchFrameDto` is a VIEWER-SIDE type. The canonical `MatchState`
//! stays Q32-fixed-point; the projection here is one-way (Q32 → f64).
//! Nothing in `fw-match-sim`'s sim path reads these floats back. The
//! determinism contract (`docs/specs/determinism-gate.md`) is
//! preserved: canonical state is what gets hashed; the DTO is for the
//! renderer only.

// The Q32 → f64 projection is the only float arithmetic in this
// module. The crate-wide deny is appropriate for sim code; the DTO is
// not sim code. Codex audit (2026-05-13) cross-checked that this
// allow is scoped to the DTO module and doesn't leak.
#![allow(clippy::float_arithmetic)]

use serde::{Deserialize, Serialize};

use crate::MatchState;

/// Per-tick snapshot of canonical match state, projected to f64 for
/// the frontend. The 22-player + ball shape is what the PixiJS dot
/// renderer needs; `tick` keys frames in a `Vec<MatchFrameDto>`
/// sequence.
///
/// `#[serde(rename_all = "camelCase")]` makes the JSON consumable
/// directly by the TypeScript `MatchFrameDTO` interface without
/// per-field renaming. Matches the convention documented in
/// `frontend/src/lib/types.ts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchFrameDto {
    /// Seed in hex, prefixed `0x` (e.g. `"0xdeadbeefdeadbeef"`).
    pub seed_hex: String,
    /// Tick index since `MatchState::initial`. 0 = initial state.
    pub tick: i64,
    pub home_score: u8,
    pub away_score: u8,
    pub players: Vec<PlayerFrameDto>,
    pub ball: BallFrameDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerFrameDto {
    pub slot: u8,
    pub pos_x: f64,
    pub pos_y: f64,
    pub vel_x: f64,
    pub vel_y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BallFrameDto {
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    pub vel_x: f64,
    pub vel_y: f64,
    pub vel_z: f64,
}

impl MatchFrameDto {
    /// Project the canonical `MatchState` into a frame DTO. One-way
    /// (Q32 → f64); the result is renderer-only and never read back
    /// into the sim.
    #[must_use]
    pub fn from_state(state: &MatchState) -> MatchFrameDto {
        MatchFrameDto {
            seed_hex: format!("0x{:016x}", state.seed.to_u64()),
            tick: state.tick.to_raw(),
            home_score: state.home_score,
            away_score: state.away_score,
            players: state
                .players
                .iter()
                .map(|p| PlayerFrameDto {
                    slot: p.slot,
                    pos_x: q32_to_f64(p.pos_x.to_bits()),
                    pos_y: q32_to_f64(p.pos_y.to_bits()),
                    vel_x: q32_to_f64(p.vel_x.to_bits()),
                    vel_y: q32_to_f64(p.vel_y.to_bits()),
                })
                .collect(),
            ball: BallFrameDto {
                pos_x: q32_to_f64(state.ball.pos_x.to_bits()),
                pos_y: q32_to_f64(state.ball.pos_y.to_bits()),
                pos_z: q32_to_f64(state.ball.pos_z.to_bits()),
                vel_x: q32_to_f64(state.ball.vel_x.to_bits()),
                vel_y: q32_to_f64(state.ball.vel_y.to_bits()),
                vel_z: q32_to_f64(state.ball.vel_z.to_bits()),
            },
        }
    }
}

/// Q32-raw-bits to f64. Renderer-side only — NEVER call this on a
/// canonical state value the sim will read back.
fn q32_to_f64(raw_bits: i64) -> f64 {
    const Q32_SCALE: f64 = 4_294_967_296.0; // 2^32
    raw_bits as f64 / Q32_SCALE
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_core::Seed;

    #[test]
    fn from_state_produces_22_player_frame() {
        let s = MatchState::initial(Seed::from_u64(1));
        let frame = MatchFrameDto::from_state(&s);
        assert_eq!(frame.players.len(), crate::TOTAL_PLAYERS);
        assert_eq!(frame.tick, 0);
        assert_eq!(frame.seed_hex, "0x0000000000000001");
    }

    #[test]
    fn json_round_trip_preserves_shape() {
        let s = MatchState::initial(Seed::from_u64(0xdead_beef));
        let frame = MatchFrameDto::from_state(&s);
        let json = serde_json::to_string(&frame).expect("encode");
        let parsed: MatchFrameDto = serde_json::from_str(&json).expect("decode");
        assert_eq!(parsed.seed_hex, frame.seed_hex);
        assert_eq!(parsed.tick, frame.tick);
        assert_eq!(parsed.players.len(), frame.players.len());
    }

    #[test]
    fn camel_case_serialization_at_wire() {
        let s = MatchState::initial(Seed::from_u64(1));
        let frame = MatchFrameDto::from_state(&s);
        let json = serde_json::to_string(&frame).expect("encode");
        // Frontend types.ts expects camelCase — verify the boundary
        // wire is what TS reads.
        assert!(json.contains("\"seedHex\""), "missing camelCase seedHex");
        assert!(
            json.contains("\"homeScore\""),
            "missing camelCase homeScore"
        );
        assert!(json.contains("\"posX\""), "missing camelCase posX");
        assert!(json.contains("\"velY\""), "missing camelCase velY");
        // Negative check: no snake_case leaked.
        assert!(!json.contains("\"seed_hex\""), "snake_case seed_hex leaked");
        assert!(!json.contains("\"pos_x\""), "snake_case pos_x leaked");
    }
}

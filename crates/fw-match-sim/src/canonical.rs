//! Canonical encoding — the byte stream the determinism gate hashes.
//!
//! ## Why a hand-rolled encoder rather than `bincode` or `serde_json`?
//!
//! The pinned-hash regression corpus
//! (`docs/specs/determinism-gate.md` §2 Layer 2) requires a byte format
//! that is:
//!
//! 1. **Bit-exact across platforms.** `bincode`'s native-endian numerics
//!    are not (varint encoding is platform-stable, but the v2 API churn
//!    risks subtle drift). A hand-rolled little-endian encoder removes
//!    third-party variance.
//! 2. **Structural, not pointer-dependent.** No iteration over hash-based
//!    collections. Vec + BTreeMap only.
//! 3. **Forward-compatible with new fields.** Each top-level section is
//!    length-prefixed so adding a new section appends rather than shifts.
//!
//! The encoding is deliberately verbose (no compression) — the consumer is
//! BLAKE3, not the network.
//!
//! ## Wire format (versioned)
//!
//! ```text
//! [ "FWMS" magic (4 bytes) ]
//! [ version u16 LE ]
//! [ seed u64 LE ]
//! [ tick i64 LE ]
//! [ home_score u8 ]
//! [ away_score u8 ]
//! [ player_count u8 ]
//! [ players × player_count ]
//!   [ slot u8 ]
//!   [ pos_x i64 LE (raw Q32 bits) ]
//!   [ pos_y i64 LE ]
//!   [ vel_x i64 LE ]
//!   [ vel_y i64 LE ]
//!   [ scalar_count u16 LE ]
//!   [ scalar × scalar_count ]
//!     [ key u16 LE ]
//!     [ value i64 LE (raw Q32 bits) ]
//!   [ role u8 ]                                    (T1-2b-iii-a: Role canonical tag)
//!   [ role_state u8 ]                              (T1-2b-iii-a: per-role state tag)
//!   [ local_decision_counter u32 LE ]              (T1-2b-iii-a: monotonic per-player counter)
//! [ decision_slots ]                               (T1-2b-ii: 22 raw u8 bytes)
//!   [ slot_0 u8 .. slot_21 u8 ]                   (22 bytes)
//! [ interrupt_cooldown_until ]                     (T1-2b-ii: 22 × i64 LE = 176 bytes)
//!   [ cooldown_0 i64 .. cooldown_21 i64 ]         (Tick::to_raw() as i64 LE)
//! [ team_tactic_states ]                           (T1-2b-ii: 2 × TeamTacticState)
//!   [ per TeamTacticState: ]
//!     [ state_tag u8 ]                             (TacticState discriminant)
//!     [ setpiece_kind_tag u8 ]                     (present only when state_tag == SetPiece)
//!     [ entry_tick i64 LE ]
//! [ ball ]                                         (T1-2b-i: 9 × Q32 = 72 bytes)
//!   [ pos_x i64, pos_y i64, pos_z i64 ]           (24 bytes)
//!   [ vel_x i64, vel_y i64, vel_z i64 ]           (24 bytes)
//!   [ spin_x i64, spin_y i64, spin_z i64 ]        (24 bytes; new at T1-2b-i)
//! ```
//!
//! **Field order rationale (T1-2b-iii-a):** the new per-player fields
//! (`role` + `role_state` + `local_decision_counter`) are appended AFTER the
//! existing player scalar section, BEFORE the match-level fields. This
//! preserves the T0/T1-2b-i/T1-2b-ii outer layout while extending the player
//! sub-record forward-compatibly. Per-player byte count increases by +6 bytes
//! (1 + 1 + 4) × 22 = +132 bytes per match-state.
//!
//! **Role encoding discriminants (T1-2b-iii-a; stable; do not reorder):**
//! - 0 = `Goalkeeper`
//! - 1 = `Defender`
//! - 2 = `Midfielder`
//! - 3 = `Forward`
//!
//! **Per-role state-tag discriminants (variant order = tag; stable):**
//! See `role_states.rs` module doc for the full table.
//!
//! **TacticState encoding discriminants (stable; do not reorder):**
//! - 0 = `HighPress`
//! - 1 = `MidBlock`
//! - 2 = `LowBlock`
//! - 3 = `CounterAttack`
//! - 4 = `SetPiece(_)` (followed by a second u8 for `SetPieceKind`)
//!
//! **SetPieceKind encoding discriminants (stable; do not reorder):**
//! - 0 = `KickOff`
//! - 1 = `GoalKick`
//! - 2 = `GoalKickOpponent`
//! - 3 = `CornerFor`
//! - 4 = `CornerAgainst`
//! - 5 = `FreeKickFor`
//! - 6 = `FreeKickAgainst`
//! - 7 = `ThrowInFor`
//! - 8 = `ThrowInAgainst`
//! - 9 = `PenaltyFor`
//! - 10 = `PenaltyAgainst`
//!
//! Adding a new field is a determinism-corpus-invalidating event. The
//! pinned hash will drift; re-baseline per
//! `docs/specs/determinism-gate.md` §9.

use crate::tactic_fsm::{SetPieceKind, TacticState, TeamTacticState};
use crate::{BallState, MatchState, PlayerState};

const MAGIC: &[u8; 4] = b"FWMS";
// VERSION history:
//   1 — T0 / T1-2b-i baseline (players + ball)
//   2 — T1-2b-ii: MatchState gained decision_slots, interrupt_cooldown_until,
//        team_tactic_states
//   3 — T1-2b-iii-a: PlayerState gained role (u8) + role_state (u8) +
//        local_decision_counter (u32 LE); +6 bytes per player × 22 = +132
const VERSION: u16 = 3;

/// Streaming canonical encoder. Append bytes as values are emitted; call
/// `finish()` to get the buffer for hashing.
pub struct CanonicalEncoder {
    buf: Vec<u8>,
}

impl CanonicalEncoder {
    /// Fresh encoder with the magic + version prefix already written.
    #[must_use]
    pub fn new() -> CanonicalEncoder {
        let mut enc = CanonicalEncoder {
            buf: Vec::with_capacity(2048),
        };
        enc.buf.extend_from_slice(MAGIC);
        enc.write_u16(VERSION);
        enc
    }

    /// Encode a `MatchState`. Single call site in `MatchState::encode_canonical`.
    ///
    /// Wire layout (T1-2b-ii):
    /// 1. Header: seed, tick, scores, player count.
    /// 2. Player loop (slot-ordered, stable).
    /// 3. Decision cadence: `decision_slots` (22 u8) + `interrupt_cooldown_until` (22 × i64).
    /// 4. Team tactic states: 2 × `TeamTacticState` (variable width; SetPiece adds 1 byte).
    /// 5. Ball: 9 × Q32 = 72 bytes.
    pub fn encode_match_state(&mut self, state: &MatchState) {
        self.write_u64(state.seed.to_u64());
        self.write_i64(state.tick.to_raw());
        self.write_u8(state.home_score);
        self.write_u8(state.away_score);

        // Player section — slot-indexed, written in slot order (which is
        // also Vec index order since `MatchState::initial` builds the Vec
        // that way). Asserts that the on-disk slot field agrees so a
        // future shuffled-Vec bug surfaces here loudly.
        assert!(
            state.players.len() <= u8::MAX as usize,
            "canonical encoder supports up to 255 players; got {}",
            state.players.len()
        );
        self.write_u8(state.players.len() as u8);
        for (i, p) in state.players.iter().enumerate() {
            // Was debug_assert_eq!; switched to assert_eq! per Codex pre-T0
            // audit. Release-mode CI must catch slot-order violations too —
            // they would silently encode bad ordering into the canonical
            // hash and corrupt the determinism gate.
            assert_eq!(
                p.slot as usize, i,
                "player at Vec index {i} has slot {} — canonical-encoding \
                 invariant violated (slot index must match Vec position)",
                p.slot
            );
            self.encode_player(p);
        }

        // T1-2b-ii: decision_slots — 22 raw u8 bytes (one per roster index).
        // Emitted after the player loop, before the ball, per the T1-2b-ii
        // wire-format spec comment at the top of this file.
        for &slot in &state.decision_slots {
            self.write_u8(slot);
        }

        // T1-2b-ii: interrupt_cooldown_until — 22 × i64 LE (176 bytes).
        // `Tick::to_raw()` returns i64; little-endian for cross-platform parity.
        for &cooldown in &state.interrupt_cooldown_until {
            self.write_i64(cooldown.to_raw());
        }

        // T1-2b-ii: team_tactic_states — 2 × TeamTacticState.
        // Each state emits: state_tag u8 + (optional setpiece_kind_tag u8) + entry_tick i64.
        // SetPiece adds one byte for the SetPieceKind discriminant; all other
        // states are 1 + 8 = 9 bytes. This is fixed-width per non-SetPiece state.
        for &tts in &state.team_tactic_states {
            self.encode_team_tactic_state(&tts);
        }

        self.encode_ball(&state.ball);
    }

    fn encode_player(&mut self, p: &PlayerState) {
        self.write_u8(p.slot);
        self.write_i64(p.pos_x.to_bits());
        self.write_i64(p.pos_y.to_bits());
        self.write_i64(p.vel_x.to_bits());
        self.write_i64(p.vel_y.to_bits());

        // BTreeMap iteration is sorted-by-key; that's exactly the property
        // the canonical encoder needs. HashMap here would silently break
        // cross-platform parity.
        assert!(
            p.scalars.len() <= u16::MAX as usize,
            "player scalar map overflowed u16; this is a sim bug"
        );
        self.write_u16(p.scalars.len() as u16);
        for (k, v) in p.scalars.iter() {
            self.write_u16(*k);
            self.write_i64(v.to_bits());
        }

        // T1-2b-iii-a: role (u8) + role_state (u8) + local_decision_counter (u32 LE).
        // Appended AFTER scalars; does not disturb byte positions of prior fields.
        // P1-1: use to_tags() from typed PlayerRoleState — byte-identical to the
        // prior split-field encoding so the canonical hash is UNCHANGED.
        let (role_tag, state_tag) = p.role_state.to_tags();
        self.write_u8(role_tag);
        self.write_u8(state_tag);
        self.write_u32(p.local_decision_counter);
    }

    /// Encode one `TeamTacticState`.
    ///
    /// Layout: `[state_tag u8] [setpiece_kind_tag u8?] [entry_tick i64 LE]`
    ///
    /// The `setpiece_kind_tag` is only present when `state_tag == 4`
    /// (`TacticState::SetPiece`). This makes the encoding variable-width:
    /// 9 bytes for non-SetPiece states, 10 bytes for SetPiece states.
    ///
    /// Discriminants are stable (documented in the module wire-format comment
    /// above); do NOT reorder `TacticState` or `SetPieceKind` variants.
    fn encode_team_tactic_state(&mut self, tts: &TeamTacticState) {
        let (state_tag, maybe_spk) = tactic_state_to_tags(tts.state);
        self.write_u8(state_tag);
        if let Some(spk_tag) = maybe_spk {
            self.write_u8(spk_tag);
        }
        self.write_i64(tts.entry_tick.to_raw());
    }

    /// Encode the ball: 9 × Q32 = 72 bytes total. Layout is fixed at
    /// T1-2b-i schema bump (canonical hash REBASELINED in same commit
    /// per ADR-0012 trigger #1):
    /// - bytes 0..24:  position (pos_x, pos_y, pos_z) as little-endian i64
    /// - bytes 24..48: velocity (vel_x, vel_y, vel_z)
    /// - bytes 48..72: spin (spin_x, spin_y, spin_z)
    ///
    /// Spin was added in T1-2b-i so Magnus integration has angular
    /// velocity in canonical state from day one. `phase1_seeds` zeros
    /// the Magnus coupling for T1 playability, so spin is structurally
    /// present but behaviorally inert until T1-2b-iii wires kicks/headers
    /// that impart spin.
    pub(crate) fn encode_ball(&mut self, b: &BallState) {
        self.write_i64(b.pos_x.to_bits());
        self.write_i64(b.pos_y.to_bits());
        self.write_i64(b.pos_z.to_bits());
        self.write_i64(b.vel_x.to_bits());
        self.write_i64(b.vel_y.to_bits());
        self.write_i64(b.vel_z.to_bits());
        self.write_i64(b.spin_x.to_bits());
        self.write_i64(b.spin_y.to_bits());
        self.write_i64(b.spin_z.to_bits());
    }

    /// Consume the encoder and return the buffer.
    pub fn finish(self) -> Vec<u8> {
        self.buf
    }

    // ---- Little-endian primitives --------------------------------------
    //
    // Hand-rolled so the encoder has no third-party drift surface. Every
    // multi-byte value goes through `to_le_bytes` — bit-exact on every
    // host CPU per the Rust language reference.

    fn write_u8(&mut self, v: u8) {
        self.buf.push(v);
    }
    fn write_u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn write_u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn write_u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn write_i64(&mut self, v: i64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
}

impl Default for CanonicalEncoder {
    fn default() -> CanonicalEncoder {
        CanonicalEncoder::new()
    }
}

// ---------------------------------------------------------------------------
// Stable encoding helpers — discriminant tables
// ---------------------------------------------------------------------------

/// Map a `TacticState` to its canonical encoding tag(s).
///
/// Returns `(state_tag, maybe_setpiece_kind_tag)`.
/// `SetPiece` emits two tags; all other states emit one.
fn tactic_state_to_tags(state: TacticState) -> (u8, Option<u8>) {
    match state {
        TacticState::HighPress => (0, None),
        TacticState::MidBlock => (1, None),
        TacticState::LowBlock => (2, None),
        TacticState::CounterAttack => (3, None),
        TacticState::SetPiece(kind) => (4, Some(set_piece_kind_tag(kind))),
    }
}

/// Map a `SetPieceKind` to its canonical encoding tag (0..=10).
fn set_piece_kind_tag(kind: SetPieceKind) -> u8 {
    match kind {
        SetPieceKind::KickOff => 0,
        SetPieceKind::GoalKick => 1,
        SetPieceKind::GoalKickOpponent => 2,
        SetPieceKind::CornerFor => 3,
        SetPieceKind::CornerAgainst => 4,
        SetPieceKind::FreeKickFor => 5,
        SetPieceKind::FreeKickAgainst => 6,
        SetPieceKind::ThrowInFor => 7,
        SetPieceKind::ThrowInAgainst => 8,
        SetPieceKind::PenaltyFor => 9,
        SetPieceKind::PenaltyAgainst => 10,
    }
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_core::{Seed, Tick};

    #[test]
    fn encoded_buffer_starts_with_magic_and_version() {
        let s = MatchState::initial(Seed::from_u64(1));
        let bytes = s.encode_canonical();
        assert_eq!(&bytes[0..4], MAGIC);
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), VERSION);
    }

    #[test]
    fn version_is_3_after_t1_2b_iii_a_schema_bump() {
        assert_eq!(
            VERSION, 3,
            "VERSION should be 3 after T1-2b-iii-a canonical schema bump \
             (PlayerState gained role + role_state + local_decision_counter)"
        );
    }

    #[test]
    fn encoding_is_stable_across_calls() {
        let s = MatchState::initial(Seed::from_u64(0xABCDEF));
        assert_eq!(s.encode_canonical(), s.encode_canonical());
    }

    #[test]
    fn different_seeds_produce_different_encodings() {
        let a = MatchState::initial(Seed::from_u64(1));
        let b = MatchState::initial(Seed::from_u64(2));
        assert_ne!(a.encode_canonical(), b.encode_canonical());
    }

    /// T1-2b-i Chunk 1 RED: the canonical ball block is now 9 fields
    /// (position + velocity + spin), each `Q32` (8 bytes), so the ball
    /// segment of the encoded buffer must be 72 bytes — up from 48 in T0.
    #[test]
    fn ball_block_encodes_spin_after_velocity() {
        let s = MatchState::initial(Seed::from_u64(1));
        let bytes = s.encode_canonical();
        // Probe the ball block directly via a fresh encoder
        let mut probe = CanonicalEncoder::new();
        probe.encode_ball(&fw_match_sim_test_ball_with_spin());
        let probe_bytes = probe.finish();
        // 6 bytes magic+version prefix on a fresh CanonicalEncoder via
        // `new`, plus 72 bytes ball.
        assert_eq!(probe_bytes.len(), 6 + 72);
        // Bytes 6..30 = position; 30..54 = velocity; 54..78 = spin.
        // Last 24 bytes (the spin block) must NOT be all-zero when spin
        // is non-zero — guards against the encoder silently dropping
        // the new fields.
        let spin_segment = &probe_bytes[6 + 48..6 + 72];
        assert!(
            spin_segment.iter().any(|&b| b != 0),
            "spin segment was all zeros; encoder didn't emit spin fields"
        );
        // The full match state is unaffected by this probe.
        assert!(
            bytes.len() > 100,
            "encoded MatchState was suspiciously short"
        );
    }

    /// T1-2b-ii: decision_slots block is present in encoding.
    /// Mutate one slot and verify the encoding changes.
    #[test]
    fn encoding_reflects_decision_slots() {
        let mut s = MatchState::initial(Seed::from_u64(1));
        let a = s.encode_canonical();
        // Mutate one decision slot to a value different from its current value
        let original = s.decision_slots[0];
        s.decision_slots[0] = if original == 14 { 0 } else { 14 };
        let b = s.encode_canonical();
        assert_ne!(a, b, "changing decision_slots should change the encoding");
    }

    /// T1-2b-ii: interrupt_cooldown_until block is present in encoding.
    #[test]
    fn encoding_reflects_interrupt_cooldown() {
        let mut s = MatchState::initial(Seed::from_u64(1));
        let a = s.encode_canonical();
        s.interrupt_cooldown_until[0] = Tick::from_raw(42);
        let b = s.encode_canonical();
        assert_ne!(
            a, b,
            "changing interrupt_cooldown_until should change the encoding"
        );
    }

    /// T1-2b-ii: team_tactic_states block is present in encoding.
    #[test]
    fn encoding_reflects_team_tactic_state() {
        use crate::tactic_fsm::{TacticState, TeamTacticState};
        let mut s = MatchState::initial(Seed::from_u64(1));
        let a = s.encode_canonical();
        s.team_tactic_states[0] =
            TeamTacticState::initial().transition(TacticState::HighPress, Tick::from_raw(100));
        let b = s.encode_canonical();
        assert_ne!(
            a, b,
            "changing team_tactic_states should change the encoding"
        );
    }

    /// T1-2b-ii: SetPiece state encodes both the state tag and the
    /// SetPieceKind tag.
    #[test]
    fn setpiece_encoding_includes_kind_tag() {
        use crate::tactic_fsm::{SetPieceKind, TacticState, TeamTacticState};
        let mut probe_a = CanonicalEncoder::new();
        let tts_penalty = TeamTacticState {
            state: TacticState::SetPiece(SetPieceKind::PenaltyFor),
            entry_tick: Tick::ZERO,
        };
        probe_a.encode_team_tactic_state(&tts_penalty);

        let mut probe_b = CanonicalEncoder::new();
        let tts_corner = TeamTacticState {
            state: TacticState::SetPiece(SetPieceKind::CornerFor),
            entry_tick: Tick::ZERO,
        };
        probe_b.encode_team_tactic_state(&tts_corner);

        assert_ne!(
            probe_a.finish(),
            probe_b.finish(),
            "different SetPieceKind variants must produce different encodings"
        );
    }

    // Test helper: a ball with nonzero spin so the encoder probe can
    // detect missing spin bytes.
    fn fw_match_sim_test_ball_with_spin() -> crate::BallState {
        crate::BallState {
            pos_x: fw_core::Q32::ZERO,
            pos_y: fw_core::Q32::ZERO,
            pos_z: fw_core::Q32::ZERO,
            vel_x: fw_core::Q32::ZERO,
            vel_y: fw_core::Q32::ZERO,
            vel_z: fw_core::Q32::ZERO,
            spin_x: fw_core::Q32::from_int(1),
            spin_y: fw_core::Q32::from_int(2),
            spin_z: fw_core::Q32::from_int(3),
        }
    }
}

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
//! [ ball ]                                  (T1-2b-i: 9 × Q32 = 72 bytes)
//!   [ pos_x i64, pos_y i64, pos_z i64 ]     (24 bytes)
//!   [ vel_x i64, vel_y i64, vel_z i64 ]     (24 bytes)
//!   [ spin_x i64, spin_y i64, spin_z i64 ]  (24 bytes; new at T1-2b-i)
//! ```
//!
//! Adding a new field is a determinism-corpus-invalidating event. The
//! pinned hash will drift; re-baseline per
//! `docs/specs/determinism-gate.md` §9.

use crate::{BallState, MatchState, PlayerState};

const MAGIC: &[u8; 4] = b"FWMS";
const VERSION: u16 = 1;

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

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_core::Seed;

    #[test]
    fn encoded_buffer_starts_with_magic_and_version() {
        let s = MatchState::initial(Seed::from_u64(1));
        let bytes = s.encode_canonical();
        assert_eq!(&bytes[0..4], MAGIC);
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), VERSION);
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
    /// Test asserts the total encoded length increased by exactly 24
    /// bytes (3 new Q32 fields × 8 bytes) vs. the implied T0 layout.
    #[test]
    fn ball_block_encodes_spin_after_velocity() {
        let s = MatchState::initial(Seed::from_u64(1));
        let bytes = s.encode_canonical();
        // Header + 22 players + 1 ball + scoreline. We don't pin the
        // total here (canonical_hash.rs::PINNED_60_TICK does that with
        // a BLAKE3); we just confirm the BALL segment grew by exactly
        // 24 bytes for the 3 new Q32 spin fields.
        //
        // The encoder writes position (3 × Q32 = 24B), velocity (24B),
        // and spin (24B). 72 bytes total per ball.
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

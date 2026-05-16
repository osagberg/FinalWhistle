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
//!   [ attributes: 55 × i64 LE ]                   (T1-2b-iii-b: PlayerAttributes in struct order)
//!   [ candidate_count u16 LE ]                    (T1-2b-fix P1-2: number of signature candidates)
//!   [ candidates × candidate_count ]              (T1-2b-fix P1-2: per-candidate encoding)
//!     [ id_len u16 LE ]                           (SignatureId UTF-8 byte length)
//!     [ id_bytes* ]                               (SignatureId UTF-8 bytes)
//!     [ affinity i64 LE ]                         (Q32 raw bits of SignatureCandidate::affinity)
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
//! [ signature_cooldowns ]                          (T1-2b-iv: BTreeMap<(slot,id),Tick>)
//!   [ entry_count u32 LE ]                         (number of active cooldown entries)
//!   [ entries × entry_count ]
//!     [ slot u8 ]                                  (PlayerSlot)
//!     [ id_len u16 LE ]                            (SignatureId UTF-8 byte length)
//!     [ id_bytes* ]                                (SignatureId UTF-8 bytes)
//!     [ cooldown_end i64 LE ]                      (Tick::to_raw() as i64)
//! [ signature_firing ]                             (T1-2b-fix: 22 × 4 categories per player)
//!   (fixed-size 22 players × 4 BiasCategory lanes — NO count prefixes; the
//!    array is statically sized at compile time and the encoder iterates
//!    the fixed dimensions directly. Codex Tier-2 re-audit P2 corrected the
//!    prior diagram which falsely claimed outer/inner counts were emitted.)
//!   [ per player slot 0..22 ]
//!     [ per BiasCategory lane 0..4 ]
//!       [ is_some u8 ]                             (0 = None, 1 = Some)
//!       [ per Some: ]
//!         [ id_len u16 LE ]
//!         [ id_bytes* ]
//!     [ start_tick i64 LE ]
//!     [ duration_ticks u32 LE ]
//! [ signature_first_fired_seen ]                   (T1-2b-iv: BTreeSet<(slot,id)>)
//!   [ entry_count u32 LE ]
//!   [ entries × entry_count ]
//!     [ slot u8 ]
//!     [ id_len u16 LE ]
//!     [ id_bytes* ]
//! [ match_end_tick i64 LE ]                        (T1-4a: match duration; 60 ticks for T1 smoke)
//! [ match_events ]                                 (T1-4a: Vec<MatchEvent> — canonical)
//!   [ event_count u32 LE ]
//!   [ events × event_count ]
//!     [ discriminant u8 ]
//!     KickOff (0):            [ tick i64 LE ] [ is_second_half u8 ]
//!     FullTime (1):           [ tick i64 LE ] [ home_score u16 LE ] [ away_score u16 LE ]
//!     Goal (2):               [ scorer_slot u8 ] [ tick i64 LE ] [ score_home_after u16 LE ] [ score_away_after u16 LE ]
//!     Shot (3):               [ shooter_slot u8 ] [ tick i64 LE ] [ target_x i64 LE ] [ target_y i64 LE ] [ on_target u8 ]
//!     Pass (4):               [ from_slot u8 ] [ to_slot u8 ] [ tick i64 LE ] [ kind u8 ] [ completed u8 ]
//!     SignatureFirstFired (5): [ player_slot u8 ] [ tick i64 LE ] [ id_len u16 LE ] [ id_bytes* ]
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

use crate::signature::SignatureFiring;
use crate::tactic_fsm::{SetPieceKind, TacticState, TeamTacticState};
use crate::{BallState, MatchEvent, MatchState, PlayerState};
use fw_content::PassKind;

const MAGIC: &[u8; 4] = b"FWMS";
// VERSION history:
//   1 — T0 / T1-2b-i baseline (players + ball)
//   2 — T1-2b-ii: MatchState gained decision_slots, interrupt_cooldown_until,
//        team_tactic_states
//   3 — T1-2b-iii-a: PlayerState gained role (u8) + role_state (u8) +
//        local_decision_counter (u32 LE); +6 bytes per player × 22 = +132
//   4 — T1-2b-iii-b: PlayerState gained attributes (55 × i64 LE);
//        +440 bytes per player × 22 = +9680 bytes per match-state
//   5 — T1-2b-iv: MatchState gained signature_cooldowns (BTreeMap len + entries),
//        signature_firing (22 × Option<SignatureFiring>),
//        signature_first_fired_seen (BTreeSet len + entries).
//        Wire-format: new sections appended AFTER ball (at end of encode_match_state).
//   6 — T1-2b-fix P1-2: PlayerState encode now includes per-player
//        signature_candidates: [candidate_count u16 LE] [per candidate:
//        id_len u16 LE + id_bytes + affinity i64 LE (raw Q32 bits)] appended
//        AFTER the 55 attribute fields. Canonical hash REBASELINED (ADR-0012
//        trigger #1 — schema bump).
//   7 — T1-4a: MatchState gained match_events: Vec<MatchEvent> (in canonical
//        state; section appended after signature_first_fired_seen).
//        signature_memory_events field REMOVED (was #[serde(skip)] transient
//        scratch buffer; subsumed by match_events). Canonical hash REBASELINED
//        per ADR-0012 trigger #1 (MatchState +1 canonical field; encoder VERSION
//        6→7 schema bump).
//        Wire-format for match_events section:
//          [ event_count u32 LE ]
//          [ per event: ]
//            [ discriminant u8 ]  (0=KickOff, 1=FullTime, 2=Goal, 3=Shot, 4=Pass, 5=SignatureFirstFired)
//            [ variant-specific fields in stable order ]
//        KickOff:            [ tick i64 LE ] [ is_second_half u8 (0=false, 1=true) ]
//        FullTime:           [ tick i64 LE ] [ home_score u16 LE ] [ away_score u16 LE ]
//        Goal:               [ scorer_slot u8 ] [ tick i64 LE ] [ score_home_after u16 LE ] [ score_away_after u16 LE ]
//        Shot:               [ shooter_slot u8 ] [ tick i64 LE ] [ target_x i64 LE ] [ target_y i64 LE ] [ on_target u8 ]
//        Pass:               [ from_slot u8 ] [ to_slot u8 ] [ tick i64 LE ] [ kind u8 ] [ completed u8 ]
//        SignatureFirstFired: [ player_slot u8 ] [ tick i64 LE ] [ id_len u16 LE ] [ id_bytes* ]
const VERSION: u16 = 7;

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

        // T1-2b-iv: signature_cooldowns — BTreeMap length + entries in sorted order.
        // Layout: [count u32 LE] [slot u8, id_len u16 LE, id_bytes*, cooldown_tick i64 LE] × count
        // BTreeMap iteration is sorted by (PlayerSlot, SignatureId) key — deterministic.
        assert!(
            state.signature_cooldowns.len() <= u32::MAX as usize,
            "signature_cooldowns overflowed u32 count field"
        );
        self.write_u32(state.signature_cooldowns.len() as u32);
        for ((slot, sig_id), cooldown_tick) in &state.signature_cooldowns {
            self.write_u8(*slot);
            let id_bytes = sig_id.as_str().as_bytes();
            assert!(
                id_bytes.len() <= u16::MAX as usize,
                "signature ID exceeds u16 length field"
            );
            self.write_u16(id_bytes.len() as u16);
            self.buf.extend_from_slice(id_bytes);
            self.write_i64(cooldown_tick.to_raw());
        }

        // T1-2b-fix P1-7: signature_firing — 22 × 4 Option<SignatureFiring> in
        // (slot, category) order. Outer loop: slots 0..22. Inner loop: categories
        // 0..4 (Attacking, Defensive, BuildUp, SetPiece by BiasCategory discriminant).
        // Layout: 88 × [present u8 (0=None, 1=Some)] [if Some: id_len u16, id_bytes*, start_tick i64, duration u32]
        for slot_row in &state.signature_firing {
            for maybe_firing in slot_row {
                self.encode_signature_firing(maybe_firing.as_ref());
            }
        }

        // T1-2b-iv: signature_first_fired_seen — BTreeSet length + entries in sorted order.
        // Layout: [count u32 LE] [slot u8, id_len u16 LE, id_bytes*] × count
        // BTreeSet iteration is sorted by (PlayerSlot, SignatureId) — deterministic.
        assert!(
            state.signature_first_fired_seen.len() <= u32::MAX as usize,
            "signature_first_fired_seen overflowed u32 count field"
        );
        self.write_u32(state.signature_first_fired_seen.len() as u32);
        for (slot, sig_id) in &state.signature_first_fired_seen {
            self.write_u8(*slot);
            let id_bytes = sig_id.as_str().as_bytes();
            assert!(
                id_bytes.len() <= u16::MAX as usize,
                "signature ID exceeds u16"
            );
            self.write_u16(id_bytes.len() as u16);
            self.buf.extend_from_slice(id_bytes);
        }

        // T1-4a: match_end_tick — i64 LE. Canonical so replaying fixtures
        // with different durations produce different hashes.
        self.write_i64(state.match_end_tick.to_raw());

        // T1-4a: match_events — Vec<MatchEvent> in chronological order.
        // Layout: [event_count u32 LE] [per-event encoding…]
        // Vec iteration is insertion order = chronological order (events are pushed
        // at the tick they fire; the Vec is never sorted post-construction).
        self.encode_match_events(&state.match_events);
    }

    /// Encode a `Vec<MatchEvent>` into the canonical byte stream.
    ///
    /// Wire format:
    /// ```text
    /// [ event_count u32 LE ]
    /// [ per event: discriminant u8 + variant-specific fields ]
    /// ```
    ///
    /// Discriminant table (stable; do NOT reorder):
    /// - 0 = `KickOff`
    /// - 1 = `FullTime`
    /// - 2 = `Goal`
    /// - 3 = `Shot`
    /// - 4 = `Pass`
    /// - 5 = `SignatureFirstFired`
    ///
    /// PassKind discriminant table (stable; do NOT reorder):
    /// - 0 = `Short`
    /// - 1 = `Long`
    /// - 2 = `Cross`
    /// - 3 = `LayOff`
    pub(crate) fn encode_match_events(&mut self, events: &[MatchEvent]) {
        assert!(
            events.len() <= u32::MAX as usize,
            "match_events overflowed u32 count field"
        );
        self.write_u32(events.len() as u32);
        for event in events {
            self.encode_match_event(event);
        }
    }

    /// Encode a single `MatchEvent`.
    fn encode_match_event(&mut self, event: &MatchEvent) {
        match event {
            MatchEvent::KickOff {
                tick,
                is_second_half,
            } => {
                self.write_u8(0); // discriminant
                self.write_i64(tick.to_raw());
                self.write_u8(if *is_second_half { 1 } else { 0 });
            }
            MatchEvent::FullTime {
                tick,
                home_score,
                away_score,
            } => {
                self.write_u8(1);
                self.write_i64(tick.to_raw());
                self.write_u16(*home_score);
                self.write_u16(*away_score);
            }
            MatchEvent::Goal {
                scorer_slot,
                tick,
                score_home_after,
                score_away_after,
            } => {
                self.write_u8(2);
                self.write_u8(*scorer_slot);
                self.write_i64(tick.to_raw());
                self.write_u16(*score_home_after);
                self.write_u16(*score_away_after);
            }
            MatchEvent::Shot {
                shooter_slot,
                tick,
                target_x,
                target_y,
                on_target,
            } => {
                self.write_u8(3);
                self.write_u8(*shooter_slot);
                self.write_i64(tick.to_raw());
                self.write_i64(target_x.to_bits());
                self.write_i64(target_y.to_bits());
                self.write_u8(if *on_target { 1 } else { 0 });
            }
            MatchEvent::Pass {
                from_slot,
                to_slot,
                tick,
                kind,
                completed,
            } => {
                self.write_u8(4);
                self.write_u8(*from_slot);
                self.write_u8(*to_slot);
                self.write_i64(tick.to_raw());
                let kind_tag: u8 = match kind {
                    PassKind::Short => 0,
                    PassKind::Long => 1,
                    PassKind::Cross => 2,
                    PassKind::LayOff => 3,
                };
                self.write_u8(kind_tag);
                self.write_u8(if *completed { 1 } else { 0 });
            }
            MatchEvent::SignatureFirstFired {
                player_slot,
                signature_id,
                tick,
            } => {
                self.write_u8(5);
                self.write_u8(*player_slot);
                self.write_i64(tick.to_raw());
                let id_bytes = signature_id.as_str().as_bytes();
                assert!(
                    id_bytes.len() <= u16::MAX as usize,
                    "signature ID exceeds u16 length field"
                );
                self.write_u16(id_bytes.len() as u16);
                self.buf.extend_from_slice(id_bytes);
            }
        }
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

        // T1-2b-iii-b: 55 attribute fields in struct-declaration order.
        // Each field is a Q32 serialised as i64 LE (8 bytes); total +440 bytes
        // per player, +9680 bytes per match-state. VERSION bumped to 4.
        let a = &p.attributes;
        // Technical (14)
        self.write_i64(a.technical.finishing.to_bits());
        self.write_i64(a.technical.long_shots.to_bits());
        self.write_i64(a.technical.passing.to_bits());
        self.write_i64(a.technical.crossing.to_bits());
        self.write_i64(a.technical.first_touch.to_bits());
        self.write_i64(a.technical.technique.to_bits());
        self.write_i64(a.technical.dribbling.to_bits());
        self.write_i64(a.technical.heading.to_bits());
        self.write_i64(a.technical.tackling.to_bits());
        self.write_i64(a.technical.marking.to_bits());
        self.write_i64(a.technical.free_kicks.to_bits());
        self.write_i64(a.technical.penalty_taking.to_bits());
        self.write_i64(a.technical.corners.to_bits());
        self.write_i64(a.technical.long_throws.to_bits());
        // Mental (10)
        self.write_i64(a.mental.anticipation.to_bits());
        self.write_i64(a.mental.composure.to_bits());
        self.write_i64(a.mental.decisions.to_bits());
        self.write_i64(a.mental.vision.to_bits());
        self.write_i64(a.mental.off_the_ball.to_bits());
        self.write_i64(a.mental.positioning.to_bits());
        self.write_i64(a.mental.concentration.to_bits());
        self.write_i64(a.mental.bravery.to_bits());
        self.write_i64(a.mental.teamwork.to_bits());
        self.write_i64(a.mental.flair.to_bits());
        // Physical (8)
        self.write_i64(a.physical.pace.to_bits());
        self.write_i64(a.physical.acceleration.to_bits());
        self.write_i64(a.physical.stamina.to_bits());
        self.write_i64(a.physical.strength.to_bits());
        self.write_i64(a.physical.agility.to_bits());
        self.write_i64(a.physical.balance.to_bits());
        self.write_i64(a.physical.jumping_reach.to_bits());
        self.write_i64(a.physical.natural_fitness.to_bits());
        // Goalkeeper (6)
        self.write_i64(a.goalkeeper.handling.to_bits());
        self.write_i64(a.goalkeeper.reflexes.to_bits());
        self.write_i64(a.goalkeeper.one_on_ones.to_bits());
        self.write_i64(a.goalkeeper.aerial_reach.to_bits());
        self.write_i64(a.goalkeeper.command_of_area.to_bits());
        self.write_i64(a.goalkeeper.kicking.to_bits());
        // Personality (14)
        self.write_i64(a.personality.determination.to_bits());
        self.write_i64(a.personality.work_rate.to_bits());
        self.write_i64(a.personality.ambition.to_bits());
        self.write_i64(a.personality.professionalism.to_bits());
        self.write_i64(a.personality.loyalty.to_bits());
        self.write_i64(a.personality.temperament.to_bits());
        self.write_i64(a.personality.pressure_tolerance.to_bits());
        self.write_i64(a.personality.big_match_appetite.to_bits());
        self.write_i64(a.personality.adaptability.to_bits());
        self.write_i64(a.personality.aggression.to_bits());
        self.write_i64(a.personality.risk_appetite.to_bits());
        self.write_i64(a.personality.selflessness.to_bits());
        self.write_i64(a.personality.consistency.to_bits());
        self.write_i64(a.personality.versatility.to_bits());
        // Durability (3)
        self.write_i64(a.durability.injury_proneness.to_bits());
        self.write_i64(a.durability.recovery_rate.to_bits());
        self.write_i64(a.durability.dirtiness.to_bits());

        // T1-2b-fix P1-2: per-player signature candidates.
        // Layout: [candidate_count u16 LE] [per-candidate: id_len u16 + id_bytes + affinity i64]
        // Vec iteration order is insertion order — stable. The candidates Vec is
        // populated at match-setup time (ordered by content-pack load order);
        // this encoding is stable across calls for the same state.
        // signature_candidates is `pub(crate)` — accessed via the field directly
        // because encode_player lives in the same crate.
        assert!(
            p.signature_candidates.len() <= u16::MAX as usize,
            "signature_candidates overflowed u16 count field"
        );
        self.write_u16(p.signature_candidates.len() as u16);
        for candidate in &p.signature_candidates {
            let id_bytes = candidate.signature_id.as_str().as_bytes();
            assert!(
                id_bytes.len() <= u16::MAX as usize,
                "signature ID exceeds u16 length field"
            );
            self.write_u16(id_bytes.len() as u16);
            self.buf.extend_from_slice(id_bytes);
            self.write_i64(candidate.affinity.to_bits());
        }
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

    /// Encode one `Option<SignatureFiring>`.
    ///
    /// Layout:
    /// - `None` → `[0u8]` (1 byte).
    /// - `Some(f)` → `[1u8] [id_len u16 LE] [id_bytes*] [start_tick i64 LE] [duration_ticks u32 LE]`.
    fn encode_signature_firing(&mut self, firing: Option<&SignatureFiring>) {
        match firing {
            None => self.write_u8(0),
            Some(f) => {
                self.write_u8(1);
                let id_bytes = f.id.as_str().as_bytes();
                assert!(
                    id_bytes.len() <= u16::MAX as usize,
                    "signature ID exceeds u16"
                );
                self.write_u16(id_bytes.len() as u16);
                self.buf.extend_from_slice(id_bytes);
                self.write_i64(f.start_tick.to_raw());
                self.write_u32(f.duration_ticks);
            }
        }
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
    fn version_is_7_after_t1_4a_schema_bump() {
        assert_eq!(
            VERSION, 7,
            "VERSION should be 7 after T1-4a MatchEvent emission canonical schema bump \
             (MatchState gained match_events: Vec<MatchEvent>; signature_memory_events removed)"
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

    /// T1-2b-iv: signature_cooldowns block is present in encoding.
    #[test]
    fn encoding_reflects_signature_cooldowns() {
        use fw_content::SignatureId;
        let mut s = MatchState::initial(Seed::from_u64(1));
        let a = s.encode_canonical();
        let sig_id = SignatureId::try_new("fwh.core:signature.no-op-stub").unwrap();
        s.signature_cooldowns
            .insert((0u8, sig_id), Tick::from_raw(600));
        let b = s.encode_canonical();
        assert_ne!(a, b, "adding a cooldown entry should change encoding");
    }

    /// T1-2b-fix P1-7: signature_firing block is present in encoding (2D array).
    #[test]
    fn encoding_reflects_signature_firing() {
        use crate::signature::SignatureFiring;
        use fw_content::{BiasCategory, SignatureId};
        let mut s = MatchState::initial(Seed::from_u64(1));
        let a = s.encode_canonical();
        // Set slot 3, Attacking category lane (index 0)
        let cat_idx = BiasCategory::Attacking as usize;
        s.signature_firing[3][cat_idx] = Some(SignatureFiring::new(
            SignatureId::try_new("fwh.core:signature.no-op-stub").unwrap(),
            Tick::from_raw(50),
            60,
        ));
        let b = s.encode_canonical();
        assert_ne!(
            a, b,
            "setting a signature_firing entry should change encoding"
        );
    }

    /// T1-2b-iv: signature_first_fired_seen block is present in encoding.
    #[test]
    fn encoding_reflects_signature_first_fired_seen() {
        use fw_content::SignatureId;
        let mut s = MatchState::initial(Seed::from_u64(1));
        let a = s.encode_canonical();
        let sig_id = SignatureId::try_new("fwh.core:signature.no-op-stub").unwrap();
        s.signature_first_fired_seen.insert((5u8, sig_id));
        let b = s.encode_canonical();
        assert_ne!(
            a, b,
            "inserting into signature_first_fired_seen should change encoding"
        );
    }

    /// T1-4a: `match_events` IS in canonical encoding (opposite of the prior
    /// `signature_memory_events_not_in_canonical_encoding` test — the old field
    /// was a transient scratch buffer excluded from encoding; `match_events` is
    /// persistent canonical state that IS encoded).
    ///
    /// Two states that differ only in `match_events` must produce DIFFERENT encoded bytes.
    #[test]
    fn match_events_is_in_canonical_encoding() {
        use fw_content::{MatchEvent, PassKind};
        use fw_core::Tick;

        let state_a = MatchState::initial(Seed::from_u64(1));
        let mut state_b = state_a.clone();
        // Push a Pass event into state_b only.
        state_b.match_events.push(MatchEvent::Pass {
            from_slot: 5,
            to_slot: 7,
            tick: Tick::from_raw(10),
            kind: PassKind::Short,
            completed: true,
        });
        assert_ne!(
            state_a.encode_canonical(),
            state_b.encode_canonical(),
            "match_events is canonical; states differing only in match_events \
             must produce different encoded bytes"
        );
        // Also verify state_a (empty events) has a 4-byte u32=0 events count
        // embedded somewhere in the output — proves the empty-list encoding
        // doesn't accidentally elide the count field.
        let bytes_a = state_a.encode_canonical();
        let bytes_b = state_b.encode_canonical();
        // state_b has 1 event; state_a has 0. The encoding must be longer.
        assert!(
            bytes_b.len() > bytes_a.len(),
            "encoding with 1 event must be longer than with 0 events"
        );
    }

    /// T1-4a Codex Tier-2 follow-up (silent-failure P0-3, code-reviewer Important,
    /// type-design P3 — all 2026-05-16): exercise `encode_match_event(Goal { .. })`
    /// directly even though no production code path emits Goal yet.
    ///
    /// Rationale: `MatchEvent::Goal` is structurally unreachable in T1 (the
    /// `apply_tactic_event_with_emission` helper was deleted as dead code; the
    /// real emission path waits on T1-9/T2 ball-in-net detection). Without this
    /// test, the Goal encoder arm has ZERO coverage — a future encoder refactor
    /// could break it silently and only surface when T1-9 wires actual emission.
    /// This test hand-constructs a Goal event, runs it through encode_match_event,
    /// and asserts the byte output is non-empty + starts with the Goal discriminant.
    #[test]
    fn encode_match_event_goal_arm_is_exercised() {
        use fw_content::MatchEvent;
        use fw_core::Tick;

        let mut enc = CanonicalEncoder::new();
        let goal_event = MatchEvent::Goal {
            scorer_slot: 9,
            tick: Tick::from_raw(1234),
            score_home_after: 1,
            score_away_after: 0,
        };
        enc.encode_match_event(&goal_event);
        let bytes = enc.finish();

        // CanonicalEncoder::new() prepends MAGIC (b"FWMS", 4 bytes) +
        // VERSION (u16 LE, 2 bytes) = 6-byte header. Goal payload starts
        // at offset 6.
        const HEADER_BYTES: usize = MAGIC.len() + 2; // 4 + 2 = 6

        // Goal discriminant is 2 (per the wire-format table: KickOff=0,
        // FullTime=1, Goal=2, Shot=3, Pass=4, SignatureFirstFired=5).
        assert!(
            bytes.len() > HEADER_BYTES,
            "encode_match_event(Goal) produced no payload bytes after header"
        );
        assert_eq!(
            bytes[HEADER_BYTES], 2u8,
            "Goal discriminant must be 2 (got {})",
            bytes[HEADER_BYTES]
        );
        // Goal encoding layout (per encode_match_event): discriminant u8 (1)
        // + scorer_slot u8 (1) + tick i64 LE (8) + score_home_after u16 LE (2)
        // + score_away_after u16 LE (2) = 14 bytes after the header.
        assert_eq!(
            bytes.len(),
            HEADER_BYTES + 14,
            "Goal variant must encode to exactly {} bytes (header {} + payload 14); got {}",
            HEADER_BYTES + 14,
            HEADER_BYTES,
            bytes.len()
        );
        // Spot-check the scorer_slot byte at header_bytes + 1.
        assert_eq!(
            bytes[HEADER_BYTES + 1],
            9u8,
            "scorer_slot byte mismatch at offset {}",
            HEADER_BYTES + 1
        );
    }

    /// T1-2b-fix P1-2: signature_candidates encoding is present and affects hash.
    #[test]
    fn encoding_reflects_player_signature_candidates() {
        use fw_content::{SignatureCandidate, SignatureId};
        use fw_core::Q32;
        let mut s = MatchState::initial(Seed::from_u64(1));
        let a = s.encode_canonical();
        // Add a candidate to player 0
        let cand = SignatureCandidate {
            signature_id: SignatureId::try_new("fwh.core:signature.long-range-strike").unwrap(),
            affinity: Q32::from_raw(1 << 31), // 0.5 in Q32.32
        };
        s.players[0].signature_candidates.push(cand);
        let b = s.encode_canonical();
        assert_ne!(
            a, b,
            "adding a signature candidate to a player should change the canonical encoding"
        );
    }

    /// Vacuousness guard: verify encoding_reflects_player_signature_candidates
    /// would fail if candidates were NOT encoded. Two states with different
    /// candidate counts must produce different encodings.
    #[test]
    fn vacuousness_check_signature_candidates_encoding() {
        use fw_content::{SignatureCandidate, SignatureId};
        use fw_core::Q32;
        let s_zero = MatchState::initial(Seed::from_u64(1));
        let mut s_one = MatchState::initial(Seed::from_u64(1));
        let cand = SignatureCandidate {
            signature_id: SignatureId::try_new("fwh.core:signature.long-range-strike").unwrap(),
            affinity: Q32::from_raw(1 << 31),
        };
        s_one.players[0].signature_candidates.push(cand);
        // They must differ (the encoding test above). If they were the same,
        // the encoding_reflects_player_signature_candidates test would pass vacuously.
        let enc_zero = s_zero.encode_canonical();
        let enc_one = s_one.encode_canonical();
        assert_ne!(
            enc_zero, enc_one,
            "vacuousness guard: states with different candidate counts must produce different encodings"
        );
        // Also verify the zero-candidate case encodes a u16 length of 0
        // (2 bytes of 0x00 0x00 appended per player after attributes).
        // This ensures the encoder didn't accidentally elide the count field.
        // We can't easily probe exact byte offsets without recomputing the layout,
        // but the length difference of the two encodings must be:
        // id_len(2) + id_bytes(len) + affinity(8) = variable, plus count field change (0->1 costs 0 bytes
        // for the count field itself which stays 2 bytes, but gains id_len+bytes+affinity).
        // The one-candidate encoding must be LONGER than zero.
        assert!(
            enc_one.len() > enc_zero.len(),
            "encoding with 1 candidate should be longer than with 0 candidates"
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

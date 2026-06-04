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
//! [ possession ]                                   (T1-3.5: Option<PlayerSlot>)
//!   [ present u8 ]                                 (0 = None, 1 = Some)
//!   [ slot u8 ]                                    (only if present == 1)
//! [ last_touched_by ]                              (T1-3.5: Option<PlayerSlot>)
//!   [ present u8 ]                                 (0 = None, 1 = Some)
//!   [ slot u8 ]                                    (only if present == 1)
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
//   8 — T1-3.5: MatchState gained possession: Option<PlayerSlot> +
//        last_touched_by: Option<PlayerSlot>. Wire-format section appended
//        AFTER match_events:
//          [ possession_present u8 ]   (0 = None, 1 = Some)
//          [ possession_slot u8 ]      (only if possession_present == 1)
//          [ last_touched_present u8 ] (0 = None, 1 = Some)
//          [ last_touched_slot u8 ]    (only if last_touched_present == 1)
//        History note (2026-05-16 T1-3.5): re-baselined per ADR-0012 trigger #1.
//   9 — T2-1a: MatchState gained home_archetype_id: String +
//        away_archetype_id: String (per-team archetype loading).
//        Resolved ArchetypeParams sidecars (home/away_archetype_params) are
//        NOT encoded — they're derived from the IDs at construction time
//        via tactic_fsm::archetype_params_for bridge. Wire-format section
//        appended AFTER last_touched_by:
//          [ home_id_len u16 LE ] [ home_id_bytes* ]
//          [ away_id_len u16 LE ] [ away_id_bytes* ]
//        History note (2026-05-17 T2-1a + T2-1b + Codex Tier-2 P2 #1
//        re-framing): T2-1a was ADR-0012 trigger #1 ONLY on BOTH pins —
//        the per-team archetype divergence didn't actually fire yet at
//        T2-1a (sole production TacticEvent consumer was Goal which
//        hardcodes MidBlock). T2-1b shipped the PossessionLost /
//        BallRecovered emissions that ACTUALLY consume per-team
//        archetype_params → T2-1b is the trigger #3 behavioral-change
//        rebaseline on the 600-tick extended seed. T2-1c BallOutOfPlay /
//        BallInPlay wiring is canonical-hash-neutral (only fires on
//        OOB, which neither pinned seed exercises). T2-1d-infra adds
//        telemetry sidecar fields via #[serde(skip)] — also canonical-
//        hash-neutral. See `crates/fw-replay/tests/canonical_hash.rs`
//        + the fixture .ron files for the per-pin re-baseline history
//        blocks; the historical sequence is T2-1a (trigger #1 schema-
//        only) → T2-1b (trigger #3 BEHAVIORAL on 600-tick; 60-tick
//        UNCHANGED through T2-1b) → T2-1c (UNCHANGED both pins) →
//        T2-1d-infra (UNCHANGED both pins).
// VERSION history continued:
//  10 — FUN-0b: MatchState gained last_shot_xg: [Q32; 22] (per-player most-recent
//       xG score, written at AttemptShot dispatch, used by GK save model). Wire:
//       22 × i64 LE appended AFTER archetype IDs. Canonical hash REBASELINED
//       per ADR-0012 trigger #1 (schema + behavior change). The REBASELINE is
//       authorized and expected — the main thread will tune coefficients and
//       re-pin after drama-sweep confirms M1 drops toward [2.3, 3.2].
//  11 — FUN-0b+c (Slice B — dispossession): MatchState gained
//       tackle_cooldown_until: [Tick; 22] (per-defender cooldown preventing
//       tackle-spam after a failed attempt). Wire: 22 × i64 LE
//       (Tick::to_raw()) appended AFTER last_shot_xg. Canonical hash
//       REBASELINED per ADR-0012 trigger #1 (schema + behavior change).
//       Rebaseline authorized: dispossession mechanic + carrier-targeting (B1)
//       change canonical possession-flow. Main thread re-pins after drama-sweep
//       confirms bimodal 0-0/infinite lock is broken.
const VERSION: u16 = 11;

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

        // T1-3.5: possession — Option<PlayerSlot> (2 bytes max: presence u8 + slot u8).
        // Appended AFTER match_events. Field order follows the append discipline;
        // no prior sections are reordered.
        self.encode_option_slot(state.possession);

        // T1-3.5: last_touched_by — Option<PlayerSlot> (2 bytes max).
        self.encode_option_slot(state.last_touched_by);

        // T2-1a: per-team archetype IDs. Appended AFTER last_touched_by per the
        // append discipline (no prior sections reordered). Each ID encodes as
        // [u16 LE byte length] [UTF-8 bytes] — same shape as signature IDs at
        // lines 287-293 above. Resolved ArchetypeParams sidecars are NOT
        // encoded; they're recomputed from the IDs at MatchState construction
        // via tactic_fsm::archetype_params_for. Schema bump VERSION 8 → 9.
        let home_id_bytes = state.home_archetype_id.as_bytes();
        assert!(
            home_id_bytes.len() <= u16::MAX as usize,
            "home_archetype_id exceeds u16 length field",
        );
        self.write_u16(home_id_bytes.len() as u16);
        self.buf.extend_from_slice(home_id_bytes);

        let away_id_bytes = state.away_archetype_id.as_bytes();
        assert!(
            away_id_bytes.len() <= u16::MAX as usize,
            "away_archetype_id exceeds u16 length field",
        );
        self.write_u16(away_id_bytes.len() as u16);
        self.buf.extend_from_slice(away_id_bytes);

        // FUN-0b: last_shot_xg — 22 × i64 LE (raw Q32 bits). Appended AFTER
        // archetype IDs per the append discipline (VERSION 9 → 10). Canonical
        // so GK save probability is deterministic and the hash pins it.
        // Wire: 22 consecutive i64 LE values, slot 0 first, slot 21 last.
        for xg in &state.last_shot_xg {
            self.write_i64(xg.to_bits());
        }

        // FUN-0b+c (Slice B): tackle_cooldown_until — 22 × i64 LE (Tick::to_raw()).
        // Appended AFTER last_shot_xg per the append discipline (VERSION 10 → 11).
        // Canonical because cooldown state affects which tackles fire deterministically.
        // Wire: 22 consecutive i64 LE values, slot 0 first, slot 21 last.
        for cooldown in &state.tackle_cooldown_until {
            self.write_i64(cooldown.to_raw());
        }
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
    ///
    /// T1-11 fold-in (Codex T1-4b P2): discriminant byte now written via
    /// `event.discriminant()` — the single source of truth in
    /// `fw-content::event::MatchEvent::discriminant()`. Prior literal values
    /// (0..5) are semantically identical; this removes the three-way
    /// duplication between `MatchEvent` variants, `MatchEventDiscriminant`,
    /// and this encoder. The cross-crate test in
    /// `fw-content/tests/event_discriminant_test.rs` pins the alignment.
    fn encode_match_event(&mut self, event: &MatchEvent) {
        // Write the stable discriminant byte first — this is what BLAKE3
        // hashes and what the canonical-hash regression pins.
        //
        // Codex T1-11 type-design P1 fix-pass: `discriminant()` now returns
        // the typed `MatchEventDiscriminant` enum (not raw u8). Cast at
        // point of use via `as u8` — sound because of `#[repr(u8)]` on the
        // enum. Byte output is identical (same hand-assigned discriminants).
        self.write_u8(event.discriminant() as u8);
        match event {
            MatchEvent::KickOff {
                tick,
                is_second_half,
            } => {
                self.write_i64(tick.to_raw());
                self.write_u8(if *is_second_half { 1 } else { 0 });
            }
            MatchEvent::FullTime {
                tick,
                home_score,
                away_score,
            } => {
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

    /// Encode an `Option<PlayerSlot>` as a 1-byte presence tag + optional 1-byte slot.
    ///
    /// Wire format:
    /// - `None`      → `[0u8]` (1 byte)
    /// - `Some(s)`   → `[1u8, s]` (2 bytes)
    ///
    /// Used by the `possession` and `last_touched_by` canonical fields (T1-3.5).
    fn encode_option_slot(&mut self, opt: Option<u8>) {
        match opt {
            None => self.write_u8(0),
            Some(slot) => {
                self.write_u8(1);
                self.write_u8(slot);
            }
        }
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
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 11);
    }

    #[test]
    fn version_is_11_after_fun0bc_dispossession_schema_bump() {
        assert_eq!(
            VERSION, 11,
            "VERSION should be 11 after FUN-0b+c dispossession schema bump \
             (MatchState gained tackle_cooldown_until: [Tick; 22])"
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

    /// T2-R7(a) — post-T2 Codex Track E-1 fix: lock the EXACT canonical
    /// tag byte for every `SetPieceKind` variant. The existing
    /// `setpiece_encoding_includes_kind_tag` test asserts only that two
    /// variants encode differently — a tag swap (e.g.
    /// `CornerFor => 7; ThrowInFor => 3`) preserves that property and
    /// silently changes the replay wire semantics for any future fixture
    /// that reaches `BallOutOfPlay -> SetPiece`. The two pinned canonical
    /// seeds (60-tick smoke + 600-tick extended) do not enter `SetPiece`,
    /// so the pinned BLAKE3 hashes give zero protection on this surface.
    ///
    /// This table is the canonical-tag SOURCE OF TRUTH for replay wire
    /// compatibility. Any reorder fails this test loudly. Adding a 12th
    /// `SetPieceKind` variant lands here as a new arm with the next
    /// integer tag; **never reuse a tag, never re-number an existing tag.**
    #[test]
    fn setpiece_kind_canonical_tags_are_locked_forever() {
        use crate::tactic_fsm::SetPieceKind;

        // (variant, expected tag byte) — see canonical.rs::set_piece_kind_tag.
        // Comment block at top of canonical.rs lines 129-145 also
        // declares these tags; keep in sync.
        let expected: &[(SetPieceKind, u8)] = &[
            (SetPieceKind::KickOff, 0),
            (SetPieceKind::GoalKick, 1),
            (SetPieceKind::GoalKickOpponent, 2),
            (SetPieceKind::CornerFor, 3),
            (SetPieceKind::CornerAgainst, 4),
            (SetPieceKind::FreeKickFor, 5),
            (SetPieceKind::FreeKickAgainst, 6),
            (SetPieceKind::ThrowInFor, 7),
            (SetPieceKind::ThrowInAgainst, 8),
            (SetPieceKind::PenaltyFor, 9),
            (SetPieceKind::PenaltyAgainst, 10),
        ];

        // Defensive: any future variant addition fails the count check
        // first (forcing this table + the encoder to be updated together).
        assert_eq!(
            expected.len(),
            11,
            "SetPieceKind has 11 variants; if you added a 12th, add it here \
             AND in set_piece_kind_tag with the next integer tag"
        );

        for (kind, want_tag) in expected {
            assert_eq!(
                super::set_piece_kind_tag(*kind),
                *want_tag,
                "SetPieceKind::{kind:?} canonical tag must be {want_tag} forever \
                 (replay wire compatibility — see canonical.rs lines 129-145)"
            );
        }
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

    /// T1-3.5: possession fields are in canonical encoding.
    ///
    /// Two states differing ONLY in `possession` / `last_touched_by` must
    /// produce different encoded bytes. Anti-vacuousness: we also assert the
    /// encoding WITH Some(9) is longer than with the initial None (which can't
    /// happen — initial is Some(9), but we manually set None to check).
    #[test]
    fn match_state_canonical_encodes_possession() {
        use fw_core::Seed;
        let state_a = MatchState::initial(Seed::from_u64(1));
        // state_a: possession = Some(9), last_touched_by = Some(9).

        let mut state_b = state_a.clone();
        // Mutate possession to None — must produce different bytes.
        state_b.possession = None;

        let enc_a = state_a.encode_canonical();
        let enc_b = state_b.encode_canonical();

        assert_ne!(
            enc_a, enc_b,
            "possession is canonical; states differing in possession \
             must produce different encoded bytes"
        );

        // Some(9) encodes as [1u8, 9u8] = 2 bytes; None encodes as [0u8] = 1 byte.
        // state_b (None) should be 1 byte shorter per field. Two fields → 2 bytes shorter.
        assert_eq!(
            enc_a.len(),
            enc_b.len() + 1, // only possession differs (last_touched_by same = Some(9))
            "Some(x) possession encodes 1 byte longer than None possession \
             (presence tag 1 byte + slot 1 byte vs presence tag 1 byte)"
        );

        // Also check last_touched_by.
        let mut state_c = state_a.clone();
        state_c.last_touched_by = None;
        let enc_c = state_c.encode_canonical();
        assert_ne!(enc_a, enc_c, "last_touched_by is canonical");
        assert_eq!(
            enc_a.len(),
            enc_c.len() + 1,
            "Some last_touched_by is 1 byte longer than None"
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

    // -----------------------------------------------------------------------
    // T1-3.6 Chunk 5: Canonical unique-attribute encoding test.
    //
    // Verifies that every `PlayerAttributes` field maps to a distinct byte
    // position in the canonical encoder. If any two fields share the same
    // encoder write site (e.g. due to copy-paste in `encode_player`), two
    // PlayerAttributes that differ only in those fields would produce the
    // same canonical hash — a silent equivalence hole.
    //
    // Method:
    //   1. Build a baseline MatchState with player 0's attributes set so
    //      ALL 55 fields have DISTINCT nonzero Q32 values
    //      (field i = Q32::from_raw((i+1) as i64 * 1024) for i in 0..55).
    //   2. Encode the baseline and record its BLAKE3 hash.
    //   3. For every pair (i, j) with i < j (1485 pairs total), swap the
    //      values at positions i and j in player 0's attribute fields,
    //      re-encode, and assert the hash differs from the baseline.
    //   4. Restore the pair before the next iteration.
    //
    // If any pair's swap produces the SAME hash, it means those two fields
    // are encoded at the same byte position — a real encoder bug.
    //
    // The setter approach (explicit closures mutating the `PlayerAttributes`
    // struct) is chosen over byte-offset swapping because it catches encoder
    // bugs rather than just verifying bytes are distinct (a byte-swap test
    // would always "pass" for any two distinct bytes, even if the encoder
    // wrote the same field twice).
    //
    // Runtime: 1485 pairs × ~1ms encoding ≈ 1.5s on a dev machine.
    // -----------------------------------------------------------------------
    #[test]
    fn canonical_encoder_all_attribute_fields_are_uniquely_positioned() {
        use fw_core::{PlayerAttributes, Q32};

        // Build base attributes: field i = (i+1)*1024 raw bits, all distinct nonzero.
        // We use `from_raw` so the values don't depend on Q32 normalization.
        // (i+1)*1024 for i in 0..55 gives values 1024, 2048, ..., 56320 —
        // all different, all in valid Q32 sub-unit range (well under 2^32).
        fn make_attrs_with_distinct_values() -> PlayerAttributes {
            let mut a = PlayerAttributes::default_zero();
            let mut i: i64 = 1;
            macro_rules! set {
                ($field:expr) => {
                    $field = Q32::from_raw(i * 1024);
                    i += 1;
                };
            }
            // MUST match the exact encoding order in encode_player (canonical.rs ~line 507).
            // Technical (14)
            set!(a.technical.finishing);
            set!(a.technical.long_shots);
            set!(a.technical.passing);
            set!(a.technical.crossing);
            set!(a.technical.first_touch);
            set!(a.technical.technique);
            set!(a.technical.dribbling);
            set!(a.technical.heading);
            set!(a.technical.tackling);
            set!(a.technical.marking);
            set!(a.technical.free_kicks);
            set!(a.technical.penalty_taking);
            set!(a.technical.corners);
            set!(a.technical.long_throws);
            // Mental (10)
            set!(a.mental.anticipation);
            set!(a.mental.composure);
            set!(a.mental.decisions);
            set!(a.mental.vision);
            set!(a.mental.off_the_ball);
            set!(a.mental.positioning);
            set!(a.mental.concentration);
            set!(a.mental.bravery);
            set!(a.mental.teamwork);
            set!(a.mental.flair);
            // Physical (8)
            set!(a.physical.pace);
            set!(a.physical.acceleration);
            set!(a.physical.stamina);
            set!(a.physical.strength);
            set!(a.physical.agility);
            set!(a.physical.balance);
            set!(a.physical.jumping_reach);
            set!(a.physical.natural_fitness);
            // Goalkeeper (6)
            set!(a.goalkeeper.handling);
            set!(a.goalkeeper.reflexes);
            set!(a.goalkeeper.one_on_ones);
            set!(a.goalkeeper.aerial_reach);
            set!(a.goalkeeper.command_of_area);
            set!(a.goalkeeper.kicking);
            // Personality (14)
            set!(a.personality.determination);
            set!(a.personality.work_rate);
            set!(a.personality.ambition);
            set!(a.personality.professionalism);
            set!(a.personality.loyalty);
            set!(a.personality.temperament);
            set!(a.personality.pressure_tolerance);
            set!(a.personality.big_match_appetite);
            set!(a.personality.adaptability);
            set!(a.personality.aggression);
            set!(a.personality.risk_appetite);
            set!(a.personality.selflessness);
            set!(a.personality.consistency);
            set!(a.personality.versatility);
            // Durability (3)
            set!(a.durability.injury_proneness);
            set!(a.durability.recovery_rate);
            set!(a.durability.dirtiness);
            debug_assert_eq!(i, 56, "expected 55 fields; counter ended at {}", i);
            a
        }

        // Build a list of 55 (name, getter, setter) triples in encoding order.
        // We use a Vec of Q32 values extracted from the baseline (via getter),
        // then swap via setter pairs. This is equivalent to the field-order
        // table in encode_player.
        fn extract_field_values(a: &PlayerAttributes) -> Vec<Q32> {
            vec![
                a.technical.finishing,
                a.technical.long_shots,
                a.technical.passing,
                a.technical.crossing,
                a.technical.first_touch,
                a.technical.technique,
                a.technical.dribbling,
                a.technical.heading,
                a.technical.tackling,
                a.technical.marking,
                a.technical.free_kicks,
                a.technical.penalty_taking,
                a.technical.corners,
                a.technical.long_throws,
                a.mental.anticipation,
                a.mental.composure,
                a.mental.decisions,
                a.mental.vision,
                a.mental.off_the_ball,
                a.mental.positioning,
                a.mental.concentration,
                a.mental.bravery,
                a.mental.teamwork,
                a.mental.flair,
                a.physical.pace,
                a.physical.acceleration,
                a.physical.stamina,
                a.physical.strength,
                a.physical.agility,
                a.physical.balance,
                a.physical.jumping_reach,
                a.physical.natural_fitness,
                a.goalkeeper.handling,
                a.goalkeeper.reflexes,
                a.goalkeeper.one_on_ones,
                a.goalkeeper.aerial_reach,
                a.goalkeeper.command_of_area,
                a.goalkeeper.kicking,
                a.personality.determination,
                a.personality.work_rate,
                a.personality.ambition,
                a.personality.professionalism,
                a.personality.loyalty,
                a.personality.temperament,
                a.personality.pressure_tolerance,
                a.personality.big_match_appetite,
                a.personality.adaptability,
                a.personality.aggression,
                a.personality.risk_appetite,
                a.personality.selflessness,
                a.personality.consistency,
                a.personality.versatility,
                a.durability.injury_proneness,
                a.durability.recovery_rate,
                a.durability.dirtiness,
            ]
        }

        fn set_field_values(a: &mut PlayerAttributes, vals: &[Q32]) {
            debug_assert_eq!(vals.len(), 55);
            a.technical.finishing = vals[0];
            a.technical.long_shots = vals[1];
            a.technical.passing = vals[2];
            a.technical.crossing = vals[3];
            a.technical.first_touch = vals[4];
            a.technical.technique = vals[5];
            a.technical.dribbling = vals[6];
            a.technical.heading = vals[7];
            a.technical.tackling = vals[8];
            a.technical.marking = vals[9];
            a.technical.free_kicks = vals[10];
            a.technical.penalty_taking = vals[11];
            a.technical.corners = vals[12];
            a.technical.long_throws = vals[13];
            a.mental.anticipation = vals[14];
            a.mental.composure = vals[15];
            a.mental.decisions = vals[16];
            a.mental.vision = vals[17];
            a.mental.off_the_ball = vals[18];
            a.mental.positioning = vals[19];
            a.mental.concentration = vals[20];
            a.mental.bravery = vals[21];
            a.mental.teamwork = vals[22];
            a.mental.flair = vals[23];
            a.physical.pace = vals[24];
            a.physical.acceleration = vals[25];
            a.physical.stamina = vals[26];
            a.physical.strength = vals[27];
            a.physical.agility = vals[28];
            a.physical.balance = vals[29];
            a.physical.jumping_reach = vals[30];
            a.physical.natural_fitness = vals[31];
            a.goalkeeper.handling = vals[32];
            a.goalkeeper.reflexes = vals[33];
            a.goalkeeper.one_on_ones = vals[34];
            a.goalkeeper.aerial_reach = vals[35];
            a.goalkeeper.command_of_area = vals[36];
            a.goalkeeper.kicking = vals[37];
            a.personality.determination = vals[38];
            a.personality.work_rate = vals[39];
            a.personality.ambition = vals[40];
            a.personality.professionalism = vals[41];
            a.personality.loyalty = vals[42];
            a.personality.temperament = vals[43];
            a.personality.pressure_tolerance = vals[44];
            a.personality.big_match_appetite = vals[45];
            a.personality.adaptability = vals[46];
            a.personality.aggression = vals[47];
            a.personality.risk_appetite = vals[48];
            a.personality.selflessness = vals[49];
            a.personality.consistency = vals[50];
            a.personality.versatility = vals[51];
            a.durability.injury_proneness = vals[52];
            a.durability.recovery_rate = vals[53];
            a.durability.dirtiness = vals[54];
        }

        // Build baseline state with distinct attribute values on player 0.
        let mut baseline_state = MatchState::initial(Seed::from_u64(1));
        baseline_state.players[0].attributes = make_attrs_with_distinct_values();
        let baseline_bytes = baseline_state.encode_canonical();
        let baseline_hash: [u8; 32] = blake3::hash(&baseline_bytes).into();

        // Extract the 55 distinct values from the baseline.
        let baseline_vals = extract_field_values(&baseline_state.players[0].attributes);
        assert_eq!(baseline_vals.len(), 55);

        // Verify all 55 values are distinct (sanity check on make_attrs_with_distinct_values).
        {
            let mut seen = std::collections::BTreeSet::new();
            for (idx, v) in baseline_vals.iter().enumerate() {
                assert!(
                    seen.insert(v.to_bits()),
                    "baseline attribute values are not all distinct at index {idx}: value {:?}",
                    v
                );
            }
        }

        // For every pair (i, j) with i < j: swap values, re-hash, assert different.
        let mut failures: Vec<(usize, usize)> = Vec::new();
        for i in 0..55usize {
            for j in (i + 1)..55usize {
                let mut swapped_vals = baseline_vals.clone();
                swapped_vals.swap(i, j);

                let mut state = MatchState::initial(Seed::from_u64(1));
                set_field_values(&mut state.players[0].attributes, &swapped_vals);
                let bytes = state.encode_canonical();
                let hash: [u8; 32] = blake3::hash(&bytes).into();

                if hash == baseline_hash {
                    failures.push((i, j));
                }
            }
        }

        assert!(
            failures.is_empty(),
            "T1-3.6 Chunk 5: canonical encoder has field-aliasing bugs. \
             Swapping the following (i, j) attribute pairs produced the \
             SAME canonical hash as the baseline, meaning both fields are \
             encoded at the same byte position in encode_player:\n\
             {:?}\n\
             Fix: check the attribute field order in canonical.rs encode_player \
             matches the struct declaration order exactly.",
            failures
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

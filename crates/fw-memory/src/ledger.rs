//! `MemoryLedger` — the append-only career-event store.
//!
//! The ledger is the source-of-truth for the career memory system. It is:
//! - **Append-only**: no mutation or removal API is exposed. Historical rows
//!   are immutable after emission (pillar 2 invariant).
//! - **Deterministically iterable**: backed by `Vec<MemoryEvent>` with
//!   `BTreeMap` indexes. No `HashMap`. Iteration order is stable.
//! - **Schema-compatible with save V2**: `MemoryLedger` serialises to the
//!   `SaveV2.ledger` field via serde. Index maps are NOT serialised — they
//!   are `#[serde(skip)]` fields rebuilt lazily on first read after an
//!   append.
//!
//! ## Salience formula
//!
//! `append` calls `compute_salience(event)` before pushing. At T3-2 this
//! returns `event.stakes` — the 1-term degenerate formula so readers have a
//! meaningful cached salience to rank on. The full 5-term blend
//! (`w_stakes·stakes + w_prominence·...`) lands when the career system
//! supplies `participant_prominence_avg` (Phase 4) and `docs/design/memory.md`
//! supplies `event_class_base_weight` + `rarity_boost`.

use std::collections::BTreeMap;

use fw_core::{ClubId, PlayerId, Q32};
use serde::{Deserialize, Serialize};

use crate::event::{EventId, MemoryEvent, SeasonNumber};

// -------------------------------------------------------------------------
// Salience (T3-2 degenerate formula; full 5-term blend deferred to Phase 4)
// -------------------------------------------------------------------------

/// Compute salience for an event at emission time.
///
/// **T3-2 degenerate formula:** returns `event.stakes` directly.
///
/// The full 5-term linear blend
/// (`w_stakes · stakes + w_prominence · participant_prominence_avg +
/// w_class · event_class_base_weight + w_rivalry · rivalry_boost +
/// w_rarity · rarity_boost`) is deferred until the career system supplies
/// `participant_prominence_avg` (Phase 4) and `docs/design/memory.md`
/// supplies `event_class_base_weight` + `rarity_boost`.
///
/// At T3-2, `stakes` is the sole emission-time signal available:
/// the emitter sets it as "how weighty this event was as it happened."
/// The cached `event.salience` field therefore equals `event.stakes` after
/// this task; readers can read either uniformly.
///
/// The result is always in [0, 1] because `stakes` is guaranteed in [0, 1]
/// by the emitter contract (enforced by `MemoryEvent`'s doc comment).
fn compute_salience(event: &MemoryEvent) -> Q32 {
    event.stakes
}

// -------------------------------------------------------------------------
// MemoryLedger
// -------------------------------------------------------------------------

/// The append-only career-event ledger.
///
/// ## Append-only invariant
///
/// No public method returns `&mut MemoryEvent` or allows removal /
/// replacement of existing rows. The ONLY write surface is `append`.
///
/// ## BTreeMap indexes (NOT canonical state)
///
/// Three lazy-rebuilt `BTreeMap` indexes allow O(log n) lookup without a
/// linear scan:
/// - `by_subject`: `PlayerId → Vec<EventId>` (the player who is `Subject`).
/// - `by_club`: `ClubId → Vec<EventId>` (any club participant).
/// - `by_class_season`: `(class_discriminant: u32, SeasonNumber) →
///   Vec<EventId>`.
///
/// Indexes are skipped in serde output (`#[serde(skip)]`). They are rebuilt
/// from `events` on the first read after an `append` (lazy via the `dirty`
/// flag).
///
/// The canonical-state source-of-truth is `events: Vec<MemoryEvent>`. The
/// BLAKE3 canonical hash is derived from `events` alone.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryLedger {
    /// Canonical event store. Insertion order IS chronological order.
    ///
    /// `pub(crate)` (T3-1 self-review type-design fix): external crates
    /// must NOT call `.events.push(...)` / `.clear()` directly — those
    /// paths bypass the `append()` machinery that stamps `event_id` +
    /// computes `salience`. Read access goes through `iter()` /
    /// `get_by_id()` / `len()` / `is_empty()`. Serde still serializes
    /// this field correctly regardless of visibility.
    pub(crate) events: Vec<MemoryEvent>,

    /// The next `EventId` to allocate. Always equal to `events.len()` after
    /// each `append`. Not in serde output — reconstructed on load.
    #[serde(skip)]
    next_id: u32,

    // ---- Lazy-rebuilt indexes (NOT canonical state) ----------------------
    /// Dirty flag: true when `events` has grown since the last index rebuild.
    /// Set on `append`; cleared on rebuild.
    #[serde(skip)]
    dirty: bool,

    /// `PlayerId` → ordered list of `EventId`s where the player is the
    /// `Subject` participant.
    #[serde(skip)]
    by_subject: BTreeMap<PlayerId, Vec<EventId>>,

    /// `ClubId` → ordered list of `EventId`s where the club participates in
    /// any role.
    #[serde(skip)]
    by_club: BTreeMap<ClubId, Vec<EventId>>,

    /// `(EventClass discriminant, SeasonNumber)` → ordered `EventId` list.
    /// Used by `SalienceReader` for rarity-band queries.
    #[serde(skip)]
    by_class_season: BTreeMap<(u32, SeasonNumber), Vec<EventId>>,
}

// Manual PartialEq: ignore the derived-index fields; equality is on the
// canonical `events` vector only.
impl PartialEq for MemoryLedger {
    fn eq(&self, other: &Self) -> bool {
        self.events == other.events
    }
}

impl Eq for MemoryLedger {}

impl MemoryLedger {
    /// Create a fresh empty ledger.
    pub fn new() -> Self {
        MemoryLedger {
            events: Vec::new(),
            next_id: 0,
            dirty: false,
            by_subject: BTreeMap::new(),
            by_club: BTreeMap::new(),
            by_class_season: BTreeMap::new(),
        }
    }

    /// Reconstruct transient state after deserialisation.
    ///
    /// `serde` skips `next_id`, `dirty`, and the index maps. This method
    /// restores `next_id` from `events.len()` and marks `dirty = true` so
    /// the indexes rebuild on first read.
    ///
    /// Call this immediately after deserialising a `MemoryLedger` from save
    /// bytes. `fw_save::load_envelope` calls this on the V2 ledger after
    /// decoding (see `fw-save/src/lib.rs` `load_envelope` body).
    pub fn restore_transient_state(&mut self) {
        self.next_id = self.events.len() as u32;
        self.dirty = true;
    }

    // ---- Write surface (append only) ------------------------------------

    /// Append a partially-constructed event. The ledger stamps `event_id`
    /// monotonically and computes + stores `salience` via `compute_salience`
    /// (T3-2: returns `event.stakes`; full 5-term blend at Phase 4).
    ///
    /// Returns the allocated `EventId`.
    ///
    /// ## Complexity
    ///
    /// O(1) — the Vec push is amortised O(1). Index maps are NOT updated
    /// here; they rebuild lazily on the next read.
    pub fn append(&mut self, mut event: MemoryEvent) -> EventId {
        let id = EventId(self.next_id);
        event.event_id = id;
        event.salience = compute_salience(&event);
        self.events.push(event);
        self.next_id += 1;
        self.dirty = true;
        id
    }

    // ---- Read surface ---------------------------------------------------

    /// Number of events in the ledger.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// True when the ledger has no events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Iterate over events in chronological order.
    pub fn iter(&self) -> std::slice::Iter<'_, MemoryEvent> {
        self.events.iter()
    }

    /// Retrieve an event by its `EventId`. O(1) — events are stored by
    /// insertion index and `EventId(n)` maps to `events[n]`.
    ///
    /// Returns `None` if the id is out of bounds (defensive; well-formed
    /// ledgers always allocate ids sequentially).
    pub fn get_by_id(&self, id: EventId) -> Option<&MemoryEvent> {
        self.events.get(id.0 as usize)
    }

    /// All `EventId`s for events where `player_id` is the `Subject`
    /// participant.
    ///
    /// Triggers a lazy index rebuild if the ledger is dirty. After rebuild
    /// subsequent calls for the same or any other subject are O(log n).
    pub fn by_subject(&mut self, player_id: PlayerId) -> &[EventId] {
        self.ensure_indexes_fresh();
        self.by_subject
            .get(&player_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// All `EventId`s for events where `club_id` participates in any role.
    pub fn by_club(&mut self, club_id: ClubId) -> &[EventId] {
        self.ensure_indexes_fresh();
        self.by_club.get(&club_id).map(Vec::as_slice).unwrap_or(&[])
    }

    /// All `EventId`s for events of a given `(class_discriminant, season)`.
    pub fn by_class_season(&mut self, class_discriminant: u32, season: SeasonNumber) -> &[EventId] {
        self.ensure_indexes_fresh();
        self.by_class_season
            .get(&(class_discriminant, season))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    // ---- Compaction ---------------------------------------------------------

    /// Compact the ledger at the 5-season boundary.
    ///
    /// For every non-`Compaction` event whose season satisfies
    /// `event.season.0 + 5 <= current_season.0` (i.e. ≥5 seasons old),
    /// sets `tick` to `None` if it is still `Some`. All other fields are
    /// untouched — `DecayFunction::Never` events survive intact; only
    /// tick-level granularity is dropped.
    ///
    /// After processing, appends exactly ONE `EventClass::Compaction` event
    /// carrying `Consequence::CompactionDrop { in_window_count }` — the total
    /// count of events in the 5-season window at this boundary, **including**
    /// events whose `tick` was already `None` from a prior compaction pass.
    /// This count is honest about the window size, not just the newly-nulled
    /// subset.
    ///
    /// **No-op** when no events fall in the compaction window — returns 0 and
    /// does NOT append a `Compaction` event.
    ///
    /// Determinism contract: `Vec`/`BTreeMap` only; no floats; no clocks;
    /// no `saturating_*` on `SeasonNumber` (plain `+` panics on impossible
    /// overflow per Sim/RULES.md §11).
    pub fn compact(&mut self, current_season: SeasonNumber) -> u32 {
        use crate::event::{
            CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind,
            EventClass, SourceId,
        };
        use fw_core::Q32;

        // Count events in the compaction window (≥5 seasons old). Null the tick
        // of those that still have one (Some → None). The `in_window_count` drives
        // the Compaction event even when all in-window events already had tick: None
        // (e.g. season-end TitleWon events emitted with tick: None by design).
        //
        // Plain `+ 5` — `SeasonNumber.0` is u16 and a 65530+ season career is
        // unreachable; panicking on overflow matches §11 checked-then-panic discipline.
        let mut in_window_count: u32 = 0;
        for event in self.events.iter_mut() {
            if !matches!(event.event_class, EventClass::Compaction)
                && event.season.0 + 5 <= current_season.0
            {
                in_window_count += 1;
                if event.tick.is_some() {
                    event.tick = None;
                }
            }
        }

        if in_window_count == 0 {
            return 0;
        }

        // Append the Compaction summary event.
        let compaction_event = crate::event::MemoryEvent {
            event_id: crate::event::EventId(0), // overwritten by append
            schema_version: 1,
            season: current_season,
            tick: None,
            career_date: CareerDate {
                year: 0,
                day_of_year: 1,
            },
            emitter: Emitter {
                kind: EmitterKind::CareerSystem,
                source_id: SourceId::None,
            },
            participants: Vec::new(),
            event_class: EventClass::Compaction,
            stakes: Q32::ZERO,
            emotion: crate::event::Emotion::Neutral,
            consequence: vec![Consequence::CompactionDrop { in_window_count }],
            callback_eligibility: CallbackEligibility::Never,
            salience: Q32::ZERO,
            decay_function: DecayFunction::Never,
        };
        self.append(compaction_event);
        in_window_count
    }

    // ---- Index internals ------------------------------------------------

    /// Rebuild all three BTreeMap indexes from scratch if the ledger is dirty.
    fn ensure_indexes_fresh(&mut self) {
        if !self.dirty {
            return;
        }
        self.rebuild_indexes();
        self.dirty = false;
    }

    fn rebuild_indexes(&mut self) {
        self.by_subject.clear();
        self.by_club.clear();
        self.by_class_season.clear();

        for event in &self.events {
            let eid = event.event_id;

            // Subject index: find the Subject participant, if any.
            for p in &event.participants {
                use crate::event::{EntityRef, ParticipantRole};
                if p.role == ParticipantRole::Subject
                    && let EntityRef::Player(pid) = p.entity
                {
                    self.by_subject.entry(pid).or_default().push(eid);
                }
                // Club index: collect ALL club participants regardless of role.
                if let EntityRef::Club(cid) = p.entity {
                    self.by_club.entry(cid).or_default().push(eid);
                }
            }

            // Class-season index. Use `EventClass::discriminant()` — the
            // single canonical mapping shared with the pin test.
            let discriminant = event.event_class.discriminant();
            self.by_class_season
                .entry((discriminant, event.season))
                .or_default()
                .push(eid);
        }
    }
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{
        CallbackEligibility, CareerDate, Consequence, DecayFunction, Emitter, EmitterKind, Emotion,
        EntityRef, EventClass, MemoryEvent, Participant, ParticipantRole, SeasonNumber, SourceId,
    };
    use fw_core::{ClubId, MatchId, PlayerId, Q32, Tick};

    fn make_event(player_id: PlayerId, season: u16, class: EventClass) -> MemoryEvent {
        MemoryEvent {
            event_id: EventId(0), // overwritten by `append`
            schema_version: 1,
            season: SeasonNumber(season),
            tick: Some(Tick::ZERO),
            career_date: CareerDate {
                year: 1,
                day_of_year: 1,
            },
            emitter: Emitter {
                kind: EmitterKind::MatchEngine,
                source_id: SourceId::Match(MatchId::new(0)),
            },
            participants: vec![Participant {
                role: ParticipantRole::Subject,
                entity: EntityRef::Player(player_id),
            }],
            event_class: class,
            stakes: Q32::ZERO,
            emotion: Emotion::Neutral,
            consequence: vec![Consequence::None],
            callback_eligibility: CallbackEligibility::Immediate,
            salience: Q32::ZERO, // overwritten by `append`
            decay_function: DecayFunction::Never,
        }
    }

    fn make_event_with_club(club_id: ClubId, season: u16) -> MemoryEvent {
        MemoryEvent {
            event_id: EventId(0),
            schema_version: 1,
            season: SeasonNumber(season),
            tick: Some(Tick::ZERO),
            career_date: CareerDate {
                year: 1,
                day_of_year: 1,
            },
            emitter: Emitter {
                kind: EmitterKind::CareerSystem,
                source_id: SourceId::Club(club_id),
            },
            participants: vec![Participant {
                role: ParticipantRole::Subject,
                entity: EntityRef::Club(club_id),
            }],
            event_class: EventClass::CupFinalWin,
            stakes: Q32::ZERO,
            emotion: Emotion::Joy,
            consequence: vec![Consequence::None],
            callback_eligibility: CallbackEligibility::Immediate,
            salience: Q32::ZERO,
            decay_function: DecayFunction::Never,
        }
    }

    /// AC3 — `append` allocates monotonically-increasing EventIds.
    #[test]
    fn append_allocates_monotonic_event_id() {
        let mut ledger = MemoryLedger::new();
        let p = PlayerId::new(1);

        // 100 sequential appends — each id must be exactly one more than the
        // previous.
        let mut last_id = None::<EventId>;
        for _ in 0..100 {
            let id = ledger.append(make_event(p, 0, EventClass::DebutSenior));
            if let Some(prev) = last_id {
                assert_eq!(
                    id.0,
                    prev.0 + 1,
                    "EventId must increment by 1 on every append"
                );
            }
            last_id = Some(id);
        }
        assert_eq!(ledger.len(), 100);
        assert_eq!(last_id.unwrap().0, 99);
    }

    /// AC8 — `append` stamps `salience == stakes` (T3-2 degenerate formula).
    ///
    /// The T3-1 placeholder always stored `Q32::ZERO`. At T3-2, `compute_salience`
    /// returns `event.stakes` so readers have a meaningful salience to rank on.
    #[test]
    fn compute_salience_returns_stakes() {
        let mut ledger = MemoryLedger::new();
        let mut ev = make_event(PlayerId::new(1), 0, EventClass::LegacyGoal);
        // Set a non-zero stakes value before appending.
        ev.stakes = Q32::from_raw(3_006_477_107_i64); // ≈ 0.7 in Q32
        let id = ledger.append(ev);
        let stored = ledger.get_by_id(id).unwrap();
        assert_eq!(
            stored.salience, stored.stakes,
            "compute_salience must return stakes (T3-2 degenerate formula)"
        );
        assert_ne!(
            stored.salience,
            Q32::ZERO,
            "salience must not be zero when stakes > 0"
        );
    }

    /// AC4 — verify the public surface of `MemoryLedger` contains ONLY the
    /// allowed read methods. This is a compile-time + runtime structural check:
    /// - `append`, `len`, `is_empty`, `iter`, `get_by_id` are the write/read
    ///   surface.
    /// - No `events_mut`, `remove`, `clear`, `replace_at` method exists.
    ///   (The absence is enforced by the compiler — adding one is a visible
    ///   diff that code review catches.)
    ///
    /// We verify the allowed surface works and that `get_by_id` returns
    /// `None` for an out-of-bounds id.
    #[test]
    fn no_mutation_api_exposed() {
        let mut ledger = MemoryLedger::new();
        assert!(ledger.is_empty());

        let id = ledger.append(make_event(PlayerId::new(42), 0, EventClass::DebutSenior));
        assert_eq!(ledger.len(), 1);
        assert!(!ledger.is_empty());
        assert!(ledger.get_by_id(id).is_some());
        assert!(ledger.iter().count() == 1);

        // Out-of-bounds id returns None, not a panic.
        assert!(ledger.get_by_id(EventId(999)).is_none());

        // No `events_mut` / `remove` / `clear` — compiler enforces this.
        // The test is proof-by-compilation: this test file cannot call those
        // methods because they don't exist.
    }

    /// AC5 — BTreeMap indexes rebuild lazily after `append` and return
    /// correct event lists per subject.
    #[test]
    fn indexes_rebuild_lazily_after_append() {
        let mut ledger = MemoryLedger::new();

        let p1 = PlayerId::new(1);
        let p2 = PlayerId::new(2);
        let p3 = PlayerId::new(3);

        // Subject p1: 3 events
        let id_a = ledger.append(make_event(p1, 0, EventClass::DebutSenior));
        let id_b = ledger.append(make_event(p1, 0, EventClass::LegacyGoal));
        let id_c = ledger.append(make_event(p1, 1, EventClass::HatTrickScored));

        // Subject p2: 1 event
        let id_d = ledger.append(make_event(p2, 0, EventClass::DebutClub));

        // Subject p3: 0 events
        let _ = p3;

        // First read triggers the lazy rebuild.
        let p1_events = ledger.by_subject(p1);
        assert_eq!(
            p1_events,
            &[id_a, id_b, id_c],
            "p1 subject index must contain all 3 of p1's events"
        );

        // Second read is served from the index (no rebuild needed — not dirty).
        let p1_events_again = ledger.by_subject(p1);
        assert_eq!(p1_events_again, &[id_a, id_b, id_c], "cache hit must match");

        let p2_events = ledger.by_subject(p2);
        assert_eq!(p2_events, &[id_d], "p2 subject index must contain 1 event");

        let p3_events = ledger.by_subject(p3);
        assert!(
            p3_events.is_empty(),
            "p3 has no events — index returns empty slice"
        );

        // Append a new event for p1 → marks dirty again.
        let id_e = ledger.append(make_event(p1, 2, EventClass::CupFinalWin));
        let p1_after = ledger.by_subject(p1);
        assert_eq!(
            p1_after.len(),
            4,
            "index must include the new event after rebuild"
        );
        assert_eq!(p1_after[3], id_e);
    }

    /// AC5 — by_club index correctly collects ClubId participants.
    #[test]
    fn by_club_index_collects_club_participants() {
        let mut ledger = MemoryLedger::new();
        let c1 = ClubId::new(10);
        let c2 = ClubId::new(20);

        let id_a = ledger.append(make_event_with_club(c1, 0));
        let id_b = ledger.append(make_event_with_club(c1, 1));
        let _ = ledger.append(make_event_with_club(c2, 0));

        let c1_events = ledger.by_club(c1);
        assert_eq!(c1_events, &[id_a, id_b]);

        let c2_events = ledger.by_club(c2);
        assert_eq!(c2_events.len(), 1);
    }

    /// AC5 — by_class_season index keys on (discriminant, season).
    #[test]
    fn by_class_season_index_keys_correctly() {
        let mut ledger = MemoryLedger::new();
        let p = PlayerId::new(7);

        let id_a = ledger.append(make_event(p, 0, EventClass::DebutSenior));
        let id_b = ledger.append(make_event(p, 0, EventClass::DebutSenior));
        let _id_c = ledger.append(make_event(p, 1, EventClass::DebutSenior));

        let debut_s0 =
            ledger.by_class_season(24 /* DebutSenior discriminant */, SeasonNumber(0));
        assert_eq!(debut_s0, &[id_a, id_b]);

        let debut_s1 = ledger.by_class_season(24, SeasonNumber(1));
        assert_eq!(debut_s1.len(), 1);
    }

    /// AC3 + AC4 — serde round-trip of MemoryLedger preserves the canonical
    /// `events` vector exactly and reconstructs transient state via
    /// `restore_transient_state`.
    #[test]
    fn serde_round_trip_restores_events_and_transient_state() {
        let mut ledger = MemoryLedger::new();
        let p = PlayerId::new(99);
        ledger.append(make_event(p, 0, EventClass::LegacyGoal));
        ledger.append(make_event(p, 1, EventClass::TitleWon));

        let json = serde_json::to_string(&ledger).expect("serialize");
        let mut restored: MemoryLedger = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ledger, restored, "events must match after round-trip");

        // next_id is skipped — must restore manually.
        assert_eq!(
            restored.next_id, 0,
            "next_id is skipped by serde; caller must call restore_transient_state"
        );
        restored.restore_transient_state();
        assert_eq!(
            restored.next_id, 2,
            "after restore, next_id equals events.len()"
        );
        assert!(
            restored.dirty,
            "dirty is true after restore so indexes rebuild on first read"
        );

        // After restore, append must pick up from id 2.
        let new_id = restored.append(make_event(p, 2, EventClass::Retirement));
        assert_eq!(new_id.0, 2);
    }

    /// Proptest: append-only invariant holds across random sequences of appends.
    /// The `events` length must equal `next_id` after every append.
    #[cfg(test)]
    mod proptest_invariants {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn append_count_matches_next_id(count in 1_u32..=200) {
                let mut ledger = MemoryLedger::new();
                let p = PlayerId::new(1);
                for _ in 0..count {
                    ledger.append(make_event(p, 0, EventClass::DebutSenior));
                }
                prop_assert_eq!(ledger.events.len() as u32, ledger.next_id);
                prop_assert_eq!(ledger.len() as u32, ledger.next_id);
            }
        }

        proptest! {
            #[test]
            fn event_ids_are_strictly_monotonic(count in 2_u32..=100) {
                let mut ledger = MemoryLedger::new();
                let p = PlayerId::new(1);
                let mut ids: Vec<EventId> = Vec::new();
                for _ in 0..count {
                    ids.push(ledger.append(make_event(p, 0, EventClass::DebutSenior)));
                }
                for w in ids.windows(2) {
                    prop_assert!(w[0] < w[1], "EventIds must be strictly increasing");
                }
            }
        }
    }

    // ---- compact() tests --------------------------------------------------

    /// AC1 (pre-boundary): compact() with no events older than 5 seasons is a
    /// no-op — ledger length unchanged, no Compaction event, all ticks intact.
    #[test]
    fn compact_pre_boundary_is_no_op() {
        use crate::event::EventClass;

        let mut ledger = MemoryLedger::new();
        let p = PlayerId::new(1);
        // Two events at season 0; current_season = 4 (0 + 5 > 4 → not compacted).
        ledger.append(make_event(p, 0, EventClass::DebutSenior));
        ledger.append(make_event(p, 1, EventClass::LegacyGoal));
        let len_before = ledger.len();

        let result = ledger.compact(SeasonNumber(4));

        assert_eq!(
            result, 0,
            "compact should return 0 when nothing is compacted"
        );
        assert_eq!(
            ledger.len(),
            len_before,
            "pre-boundary compact must not change ledger length"
        );
        // All ticks must still be Some.
        for ev in ledger.iter() {
            if !matches!(ev.event_class, crate::event::EventClass::Compaction) {
                assert!(
                    ev.tick.is_some(),
                    "pre-boundary compact must not null any ticks"
                );
            }
        }
        // No Compaction event appended.
        let compaction_count = ledger
            .iter()
            .filter(|e| matches!(e.event_class, crate::event::EventClass::Compaction))
            .count();
        assert_eq!(
            compaction_count, 0,
            "no Compaction event should be appended before the boundary"
        );
    }

    /// AC1 (post-boundary): compact() with events ≥5 seasons old nulls their
    /// ticks and appends exactly one Compaction event recording the count.
    #[test]
    fn compact_post_boundary_nulls_old_ticks_and_appends_compaction_event() {
        use crate::event::{Consequence, EventClass};

        let mut ledger = MemoryLedger::new();
        let p = PlayerId::new(1);
        // Season 0 events (5 seasons before current_season 5) → should be compacted.
        ledger.append(make_event(p, 0, EventClass::DebutSenior));
        ledger.append(make_event(p, 0, EventClass::LegacyGoal));
        // Season 3 event (not yet 5 seasons before 5) → NOT compacted.
        ledger.append(make_event(p, 3, EventClass::HatTrickScored));

        let len_before = ledger.len();
        let result = ledger.compact(SeasonNumber(5));

        assert_eq!(result, 2, "two season-0 events should be compacted");
        assert_eq!(
            ledger.len(),
            len_before + 1,
            "one Compaction event should be appended"
        );

        // The two season-0 events must have tick == None.
        let season_0_events: Vec<_> = ledger
            .iter()
            .filter(|e| e.season == SeasonNumber(0))
            .collect();
        assert_eq!(season_0_events.len(), 2);
        for ev in &season_0_events {
            assert!(
                ev.tick.is_none(),
                "season-0 events must have tick == None after compact()"
            );
        }

        // The season-3 event must still have tick == Some.
        let season_3_event = ledger.iter().find(|e| e.season == SeasonNumber(3)).unwrap();
        assert!(
            season_3_event.tick.is_some(),
            "season-3 event must not be compacted"
        );

        // Exactly one Compaction event.
        let compaction_events: Vec<_> = ledger
            .iter()
            .filter(|e| matches!(e.event_class, EventClass::Compaction))
            .collect();
        assert_eq!(compaction_events.len(), 1, "exactly one Compaction event");

        let compaction_ev = compaction_events[0];
        assert_eq!(compaction_ev.season, SeasonNumber(5));
        assert!(
            compaction_ev.tick.is_none(),
            "Compaction event tick must be None"
        );
        assert!(
            matches!(
                compaction_ev.callback_eligibility,
                crate::event::CallbackEligibility::Never
            ),
            "Compaction event must have CallbackEligibility::Never"
        );

        // The CompactionDrop consequence must carry the right count.
        let drop_consequence = compaction_ev
            .consequence
            .iter()
            .find(|c| matches!(c, Consequence::CompactionDrop { .. }));
        assert!(
            drop_consequence.is_some(),
            "Compaction event must carry CompactionDrop consequence"
        );
        match drop_consequence.unwrap() {
            Consequence::CompactionDrop { in_window_count } => {
                assert_eq!(*in_window_count, 2, "in_window_count must equal 2");
            }
            _ => unreachable!(),
        }
    }

    /// AC1 (indexes): after compact(), the lazy indexes rebuild correctly —
    /// by_class_season for Compaction and original event classes still works.
    #[test]
    fn compact_indexes_rebuild_correctly_after_compaction() {
        use crate::event::EventClass;

        let mut ledger = MemoryLedger::new();
        let p = PlayerId::new(1);
        ledger.append(make_event(p, 0, EventClass::TitleWon));
        ledger.append(make_event(p, 0, EventClass::TitleWon));
        ledger.append(make_event(p, 3, EventClass::DebutSenior));

        ledger.compact(SeasonNumber(5));

        // The by_class_season for Compaction (discriminant 29) at season 5
        // should have exactly one entry.
        let compaction_ids = ledger.by_class_season(29, SeasonNumber(5));
        assert_eq!(
            compaction_ids.len(),
            1,
            "one Compaction event in season 5 index"
        );

        // TitleWon (discriminant 22) at season 0 should still index correctly.
        let title_won_s0 = ledger.by_class_season(22, SeasonNumber(0));
        assert_eq!(
            title_won_s0.len(),
            2,
            "two TitleWon events in season 0 index"
        );

        // DebutSenior (discriminant 24) at season 3 should still be present.
        let debut_s3 = ledger.by_class_season(24, SeasonNumber(3));
        assert_eq!(debut_s3.len(), 1, "one DebutSenior event in season 3 index");
    }

    /// AC1 advancing boundary: a second compact at a HIGHER season boundary
    /// correctly compacts new in-window events and appends another Compaction
    /// event. This mirrors the real career-loop pattern where compact() is
    /// called once per `advance_season` with a strictly increasing season number.
    #[test]
    fn compact_at_advancing_boundary_compacts_new_events() {
        use crate::event::EventClass;

        let mut ledger = MemoryLedger::new();
        let p = PlayerId::new(1);
        // Season 0 and season 1 events.
        ledger.append(make_event(p, 0, EventClass::DebutSenior));
        ledger.append(make_event(p, 1, EventClass::LegacyGoal));

        // First compact at season 5 → compacts season-0 event.
        let r1 = ledger.compact(SeasonNumber(5));
        assert_eq!(r1, 1, "first compact should report 1 in-window event");
        let len_after_first = ledger.len();

        // Second compact at season 6 → season-1 event is now in window (1+5<=6).
        let r2 = ledger.compact(SeasonNumber(6));
        assert_eq!(
            r2, 2,
            "second compact should report 2 in-window events (season 0 + season 1)"
        );
        assert_eq!(
            ledger.len(),
            len_after_first + 1,
            "one additional Compaction event for the new boundary"
        );
    }

    /// T3-R-C mutation coverage — the `event.season.0 + 5 <= current_season.0`
    /// boundary is pinned at EXACTLY 5. For a fixed season-0 event, `compact()`
    /// is a no-op for `current_season` 1..=4 and fires at exactly 5. This single
    /// test kills the `+ 5` ±1 mutation (a `+ 4` would fire at season 4; a `+ 6`
    /// would not fire at season 5) AND the `<=` → `<` mutation (which would not
    /// fire at season 5).
    #[test]
    fn compact_boundary_fires_only_at_exactly_five_seasons() {
        use crate::event::EventClass;

        // Below the boundary: a season-0 event must NOT compact at current 1..=4.
        for current in 1u16..=4 {
            let mut ledger = MemoryLedger::new();
            ledger.append(make_event(PlayerId::new(1), 0, EventClass::DebutSenior));

            let fired = ledger.compact(SeasonNumber(current));

            assert_eq!(
                fired, 0,
                "compact(season {current}) must not compact a season-0 event (0 + 5 > {current})"
            );
            assert!(
                !ledger
                    .iter()
                    .any(|e| matches!(e.event_class, EventClass::Compaction)),
                "no Compaction event before the 5-season boundary (current_season = {current})"
            );
        }

        // At exactly the boundary: a season-0 event IS compacted at current 5.
        let mut ledger = MemoryLedger::new();
        ledger.append(make_event(PlayerId::new(1), 0, EventClass::DebutSenior));

        let fired = ledger.compact(SeasonNumber(5));

        assert_eq!(
            fired, 1,
            "compact(season 5) must compact the season-0 event (0 + 5 <= 5)"
        );
        assert_eq!(
            ledger
                .iter()
                .filter(|e| matches!(e.event_class, EventClass::Compaction))
                .count(),
            1,
            "exactly one Compaction event at the 5-season boundary"
        );
    }
}

# ADR-0005 — Memory ledger event model + breakthrough triggers

**Status:** Proposed

**Date:** 2026-05-13

**Decider:** Claude (synthesis from research wave 2026-05-13 + DESIGN_DOC §7 review) + Codex (pending pre-T3 audit)

---

## Context

Phase T3 lands the event-sourced career-memory ledger and the breakthrough mechanism. These are the structural carriers of pillar 2 ("Careers That Remember") and pillar 3 ("Breakthrough-Driven Development") in `docs/DESIGN_DOC.md` §3. Both pillars were confirmed by the 2026-05-13 research wave as **genuinely differentiated** in the shipped industry — `docs/research/sports-sims/00-synthesis.md` lines 127-128 records the finding that no shipped sports sim ships "the memory ledger IS the development system." Madden, NBA 2K, FM, FOF, OOTP all bolt cinematic breakouts onto a linear XP / training base; FW's pillar 3 inverts that, and this ADR has to make the inversion structural rather than rhetorical.

DESIGN_DOC §7 sketches the schema, the salience formula, and the five readers, but it pre-dates the Codex pre-T0 audit's correction that `stakes` and `salience` cannot be `f32`/`f64` (Sim/RULES §1). The audit note at §7 caveats this in prose; this ADR locks the canonical type as `Q32` and revises §7's struct sketch in-place when T3 begins. Several other shapes in §7 are stable enough to lock here: append-only ledger, schema-versioned record, content-pack-qualified IDs, salience computed at emission, callback eligibility per-event.

Constraints (priority order):

1. **Pillar fidelity.** Pillar 2 requires that years-old events surface specifically — not via a templated "former player returns" generic, but with the actual ledger row slotted into commentary. Pillar 3 requires that progression be *event-caused*, not XP-accumulated — a Breakthrough redraws PA, fires rarely (~1-3 per career), is permanent, and reads as a moment in football-native commentary. Pillar 4 (scouting uncertainty) consumes the same ledger as a read-side signal — biased scouts observe the ground-truth events through their archetype filters.
2. **Determinism non-negotiables.** Append-only ledger, hook-enforced for historical mutation (`.claude/hooks/protect-decisions.sh` is the model). All canonical-state numerics in Q32.32 (Sim/RULES §1). `BTreeMap`-only in `fw-memory` (§2). No clocks (§3). No system RNG (§4). No `tokio` / async (§5). `MemoryEvent` is a stable canonical type — schema-versioned, forward-migration only.
3. **Mod-friendliness from day one.** Content packs add new event classes; the ledger must round-trip an event whose class the host engine doesn't recognise without losing it and without crashing the canonical hash (DESIGN_DOC §6, Content/RULES §6).
4. **Compaction headroom.** A career runs 10-30 in-game seasons; the ledger grows monotonically until the 5-season compaction boundary. The schema must let compaction drop tick-level granularity while preserving callback eligibility (DESIGN_DOC §7 compaction paragraph).

This ADR locks the `MemoryEvent` schema, the initial event-class catalogue, the salience scoring shape (with numeric weights deferred to `docs/design/memory.md` per `MEMORY.md` "tuning coefficients stay out of SPEC"), the decay-function set, the five readers' contracts, the breakthrough mechanism, the regressive-collapse counterpart, and the unknown-event-class load contract for mods. Per-attribute-family redraw distributions and exact threshold values are tuning seeds for `docs/design/memory.md` and `docs/design/progression.md`, not ADR content.

## Decision

We will model career memory as an append-only `Vec<MemoryEvent>` ledger inside `fw-memory`, with `MemoryEvent` as a canonical `serde`-backed record carrying salience + decay + callback eligibility at emission time; five read-only projections (`SalienceReader`, `PressReader`, `FanReader`, `ScoutReader`, `CoachReader`) query the ledger without mutating it; a per-(player, attribute-family) `signature_readiness: Q32` ticker accumulates from salient events and gates a narrative-anchored `BreakthroughTrigger` that redraws PA upward in the relevant attribute family; the symmetric `RegressiveCollapse` trigger redraws PA downward and is reversible only through subsequent positive ledger events that re-cross the trigger threshold; unknown event classes from mod content packs round-trip via an `UnknownEventClass { tag, payload }` opaque variant that participates in canonical hashing but contributes nothing to readers.

### MemoryEvent schema

The canonical record. Declared in `crates/fw-memory/src/event.rs`, re-exported at the crate root:

```rust
/// A single appended row in the career memory ledger. Immutable after
/// emission. Schema-versioned for forward migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryEvent {
    /// Stable identity. Allocated monotonically by the ledger at append.
    pub event_id: EventId,
    /// Schema version. Bumped only by forward-migration in
    /// `fw-content::migrations`; older fixtures stay byte-identical.
    pub schema_version: u16,
    /// In-game season number (0-indexed from save start).
    pub season: SeasonNumber,
    /// Sim tick at emission. Compaction may zero this past the 5-season
    /// boundary; readers must tolerate `None` and fall back to `season`.
    pub tick: Option<Tick>,
    /// Calendar date the event "happened" in the fictional world.
    /// Surfaces in commentary and press readers.
    pub career_date: CareerDate,
    /// Who emitted the event. Distinguishes "match engine emitted a
    /// breakthrough" from "career system emitted a transfer" from "press
    /// reader emitted a quote that itself becomes referenceable".
    pub emitter: Emitter,
    /// Who/what is implicated. Ordered for stable hashing.
    pub participants: Vec<Participant>,
    /// The structural kind of event.
    pub event_class: EventClass,
    /// In [0, 1]. How weighty the event was *as it happened*. Stable
    /// over time — does not decay. The decay model lives on
    /// `decay_function`. Stored as Q32 (Sim/RULES §1).
    pub stakes: Q32,
    /// The dominant affective register. Drives press tone, fan callback,
    /// scout-report flavor. NOT used to bias the sim — emotion is a
    /// read-side projection input only.
    pub emotion: Emotion,
    /// Ordered list of downstream effects this event encoded at emission.
    /// Consumed by readers; never mutated.
    pub consequence: Vec<Consequence>,
    /// When and under what conditions this event becomes recall-eligible.
    pub callback_eligibility: CallbackEligibility,
    /// In [0, 1]. Computed at emission via the salience formula below.
    /// Cached on the event so readers don't recompute every query. The
    /// emission-time value is canonical; reader-side modifiers (callback
    /// age, player attention) are applied on top at read time without
    /// mutating this field.
    pub salience: Q32,
    /// The decay shape. `Never` for legacy events (cup wins, breakthroughs);
    /// `Exponential` for routine emotional events (fan annoyance after
    /// a draw); `Linear` for mid-band events (rivalry slights).
    pub decay_function: DecayFunction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SeasonNumber(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CareerDate {
    pub year: u16,
    pub day_of_year: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Emitter {
    pub kind: EmitterKind,
    pub source_id: SourceId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmitterKind {
    MatchEngine,
    CareerSystem,
    PressReader,
    ScoutReader,
    BoardSystem,
    ModExtension,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceId {
    Match(MatchId),
    Club(ClubId),
    Player(PlayerId),
    Mod(u32),
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Participant {
    pub role: ParticipantRole,
    pub entity: EntityRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipantRole {
    Subject,
    Counterparty,
    Witness,
    Beneficiary,
    Victim,
    Authority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityRef {
    Player(PlayerId),
    Club(ClubId),
}
```

`MemoryLedger` (already stubbed in `crates/fw-memory/src/lib.rs`) gains an append API that allocates the next `EventId` and stamps salience via the scoring function before pushing. `BTreeMap<EventId, usize>` indexes from id to position when a reader needs O(log n) lookup; the source-of-truth is still the `Vec` for canonical iteration order.

### Event class catalogue (initial set, 29 entries)

A non-exhaustive `EventClass` enum, locked at schema_version = 1. Mod content packs add classes via `UnknownEventClass` (see "Mod-overlay compatibility" below); upgrading a mod class into core requires a schema migration.

```rust
pub enum EventClass {
    // Performance moments
    BreakthroughMoment,         // pillar 3 trigger — the redraw event itself
    SignatureFirstFired,        // a player's signature move executed cleanly for the first time
    LegacyGoal,                 // a goal that survives compaction (cup final winner, derby winner)
    HatTrickScored,             // three or more in a match — surfaces in alumni recall
    BigMatchScar,               // sub-par performance in a high-stakes context
    RegressiveCollapse,         // pillar 3 inverse trigger — PA-down redraw

    // Contract / transfer arc
    PromisedYouthMinutes,       // manager promise event; emits a BrokenPromise if expired
    BrokenPromise,              // a previously-emitted promise expired without delivery
    ContractRenewalRejected,    // player turned down an offered renewal
    ContractRenewalAccepted,    // signed; the symmetric positive callback
    TransferRequested,          // a player requested a move
    TransferRefused,            // the manager refused a transfer request
    SoldUnderProtest,           // sold against the player's wishes
    BoughtOnDeadlineDay,        // arrived at the buzzer; surfaces as commentary flavor

    // Relational
    RivalryFormed,              // two players, or player-and-club, escalated to rival status
    MentorTeammate,             // an older player mentored a younger one across a season
    DerbyControversy,           // red-card lash-out, after-match incident, manager touchline drama
    FormerClubReunion,          // player faces a club they previously played for

    // Competition arc
    CupFinalWin,                // a cup-final victory; near-permanent recall eligibility
    CupFinalLoss,
    PromotionWon,
    RelegationSuffered,
    TitleWon,
    UnbeatenRunEnded,           // a notable streak broke

    // Career-shape
    DebutSenior,                // first senior appearance
    DebutClub,                  // first appearance for a new club
    Retirement,                 // the player retired; the ledger closes for them
    InjuryLongTerm,             // ≥ 3-month absence; surfaces in scout reports + commentary
    InternationalCallUp,        // first national-team call (procedural-fantasy nation only)
}
```

Each class carries a `base_weight: Q32` in `docs/design/memory.md` — that's the `event_class_base_weight` term in the salience formula. Weights are tuning seeds, not schema; bumping a weight does not bump `schema_version`.

### Salience scoring

Locked as a five-term linear blend with clamping to [0, 1], matching DESIGN_DOC §7 line 196-203:

```
salience = clamp(
    w_stakes        · stakes
  + w_prominence    · participant_prominence_avg
  + w_class         · event_class_base_weight
  + w_rivalry       · rivalry_boost
  + w_rarity        · rarity_boost
  , Q32::ZERO, Q32::ONE)
```

**Numeric weights** (`w_stakes`, `w_prominence`, `w_class`, `w_rivalry`, `w_rarity`) are tuning seeds in `docs/design/memory.md` — per `MEMORY.md` "tuning coefficients stay out of SPEC", and the same rule applies to ADRs. The current seeds (0.4 / 0.2 / 0.2 / 0.1 / 0.1) are reasonable starting values but expected to drift through Phase 3 user testing.

**Inputs:**
- `participant_prominence_avg`: mean of each participant's `prominence: Q32` field — a derived value from match history + standing + media share. Computed by the career system at emission time; not stored on participants persistently.
- `event_class_base_weight`: lookup in the event-class tuning table.
- `rivalry_boost`: nonzero only when at least two participants share an active rivalry edge in the rivalry graph (Phase 4).
- `rarity_boost`: nonzero only when the event class has fired < N times in the past M ticks for this subject; rarity decays back toward zero as the class fires.

**Implementation note:** the five-term blend is computed in Q32 with checked arithmetic. A failed clamp invariant (output > 1 + epsilon) is a panic in debug, a saturate in release, and a `proptest` invariant in tests. The clamp is the contract; the blend's specific weights are not.

### Decay model

Three decay functions are supported at schema_version = 1:

```rust
pub enum DecayFunction {
    /// Salience stays at emission value forever. Cup finals, breakthroughs,
    /// retirement, broken-promise resolutions.
    Never,
    /// Salience(t) = emission_salience · max(0, 1 - elapsed / lifetime).
    /// Mid-band events. Reaches zero at `lifetime` and is then a candidate
    /// for compaction.
    Linear { lifetime_ticks: u32 },
    /// Salience(t) = emission_salience · exp(-elapsed / half_life).
    /// Routine emotional events (fan annoyance after a draw, training-ground
    /// frustration). Approximated with a Q32-friendly lookup table — no
    /// `exp` call in the hot path.
    Exponential { half_life_ticks: u32 },
}
```

Decay is a **read-time projection**, not a mutation: the canonical `salience` field on the event is the emission value forever. Readers project current salience as `emission_salience × decay_factor(decay_function, now - emission_tick)`. This preserves the append-only invariant — historical events never change.

Decay interacts with salience via the readers, not the ledger. A `Never` event always reads at emission salience. An `Exponential` event with half_life = 30 in-game days reads at half emission salience after 30 days, regardless of intervening events. Re-firing salience via fresh related events emits a new event row rather than updating the old one — pillar 2's compounding ("the kid you cut becomes a captain") is a chain of rows, not a mutation.

### The five readers

Each is a read-only projection over `&MemoryLedger`. Lives in `crates/fw-memory/src/readers/`. None mutate. None hold state across calls.

| Reader | Reads | Computes | Cadence |
|---|---|---|---|
| `SalienceReader` | full ledger | top-N events by projected current salience, optionally filtered by subject / class / participant | on demand (UI surfaces, pre-match overlays) |
| `PressReader` | callback-eligible events whose tags match a press-conference topic frame | a ranked candidate list for Tracery slot-filling against the press-quote phrase bank | per press conference (~weekly) |
| `FanReader` | events tagged for fan culture (legacy goals, derbies, sell-under-protest), filtered by club and recency window | aggregate fan sentiment vector + recent-event callback list for the fan-mood surface | once per match-day cycle |
| `ScoutReader` | events about a target player, observed through the scout's archetype bias | a per-scout perceived event subset that may omit or distort events based on the scout's regional / archetype blind spots | per scout report (on demand) |
| `CoachReader` | events about own-club players, weighted by recency + match-stakes context | per-player breakthrough-readiness + regressive-risk signals for the coach AI's selection and training decisions | once per training-week cycle |

All readers iterate over `BTreeMap` indexes (`BTreeMap<PlayerId, Vec<EventId>>` for per-subject lookups, `BTreeMap<ClubId, Vec<EventId>>` for per-club, `BTreeMap<(EventClass, SeasonNumber), Vec<EventId>>` for rarity-band queries). Indexes are rebuilt lazily on first read after an append; the ledger's append path stays O(1).

### Breakthrough mechanism

The pillar 3 differentiator. Per (player, attribute_family) pair, the career system maintains a `signature_readiness: Q32` in [0, 1]. Attribute families are coarse-grained groups (`Finishing`, `Passing`, `DefensiveAnticipation`, `AerialPresence`, `Composure`, `Pace`, `WorkRate`, ...) — the exact list is a tuning concern for `docs/design/progression.md`.

`signature_readiness` ticks upward when a `MemoryEvent` with non-zero salience and an attribute-family-relevant `EventClass` is appended:

```
readiness_delta = projected_salience(event) × family_relevance(event.class, family)
new_readiness   = clamp(old_readiness + readiness_delta, Q32::ZERO, Q32::ONE)
```

`family_relevance` is a per-(event_class, attribute_family) lookup in `docs/design/progression.md`. A `LegacyGoal` adds heavily to `Finishing` and modestly to `Composure`. A `MentorTeammate` adds to `Composure` and `Vision` but nothing to `Pace`.

**The Breakthrough fires only on a narrative gate.** A meter at 1.0 is necessary but not sufficient. The actual trigger requires:

1. `signature_readiness[player, family] >= breakthrough_threshold` (tuning seed; current seed 0.92).
2. A **gating event** is emitted in the same tick — a meaningful `EventClass` that thematically matches the family. Late-bloomer Finishing breakthroughs gate on `LegacyGoal` or `HatTrickScored`. DefensiveAnticipation breakthroughs gate on a high-stakes clean sheet or a goal-line clearance moment. The gating-event table is in `docs/design/progression.md`.
3. The narrative-anchor check passes: the player is over the cooldown floor since their last breakthrough or regressive collapse (cooldown seed: 12 in-game months).

When all three pass, the career system emits a `MemoryEvent { event_class: BreakthroughMoment, decay_function: Never, ... }` and applies the redraw:

```
delta_pa[attribute_family] = redraw_distribution(family, stakes, age_curve_position)
new_pa  = clamp(old_pa + delta_pa, attribute_pa_min, attribute_pa_max)
new_ca  = clamp(old_ca + ca_lift_fraction · delta_pa, attribute_ca_min, new_pa)
```

The redraw is in PA first (the ceiling lifts), CA partially (the floor catches up by `ca_lift_fraction` — seed: 0.5), and the BreakthroughMoment event carries the deltas in its `consequence` so readers (commentary, scout, press) can phrase the lift specifically. `redraw_distribution` is a per-family seeded distribution in `docs/design/progression.md` — magnitude is tuning, not schema.

After firing, `signature_readiness[player, family]` resets to a seed value (current seed: 0.15 — a small residue, not full reset, so the curve feels continuous).

This is "the ledger told us this happened, so the player grew" — explicitly NOT XP, training-tick accumulation, or cinematic-only flavor on top of XP. The Madden / NBA 2K / FM pattern is rejected (`07-player-attributes-progression.md` lines 71-72).

### Regressive collapse

The dark counterpart. Bad events accumulate a parallel `regressive_pressure: Q32` per (player, attribute_family) that ticks up on negative-class events:

- `BigMatchScar` → +pressure on `Composure`
- `InjuryLongTerm` → +pressure on `Pace`, `Acceleration`, `Stamina`
- `BrokenPromise` (this player as Victim) → +pressure on `WorkRate`, `Determination`
- `ContractRenewalRejected` (manager refused player's demand) → +pressure on `Teamwork`
- `RelegationSuffered` (high stakes) → +pressure on club-wide `Composure` band

`regressive_pressure` follows the same threshold + gating + cooldown pattern as breakthrough — at threshold (seed: 0.9), with a narrative-anchored gating event (the missed penalty in the relegation playoff), the career system emits `MemoryEvent { event_class: RegressiveCollapse, decay_function: Never, ... }` and redraws PA *downward* in the affected family. The redraw is bounded — never below the attribute's career-floor (seed: max(20, current_ca - 30)) — so a collapse degrades but does not erase a player.

**Reversibility:** a regressive collapse does not auto-recover. The only path back is a subsequent BreakthroughMoment in the same family — clearing the readiness threshold again, with a positive gating event, after the cooldown. The narrative reading: a confidence-fractured striker can rebuild. The mechanical reading: collapses lift the regressive_pressure back to its seed and clear `signature_readiness` to seed, so the next arc starts on the meters from scratch. The PA delta is permanent unless a subsequent breakthrough in the same family adds positive PA.

Regressive collapse and breakthrough are symmetric in the schema (same `BreakthroughMoment` / `RegressiveCollapse` shape in `EventClass`, same Never decay, same redraw via `consequence`) and asymmetric in cadence (collapses cooldown is longer — seed: 18 in-game months — and the narrative gate is harder to satisfy, because pillar 3's intent is that growth-down moments are *rarer and heavier* than growth-up, not the inverse).

### Mod-overlay compatibility

A mod content pack adds new event classes via the `UnknownEventClass` variant:

```rust
pub enum EventClass {
    // ... 28 core variants above ...

    /// Mod-defined event class the host engine does not recognise.
    /// Round-trips losslessly through serde + canonical hash. Readers
    /// ignore it. A future schema migration may promote a specific
    /// `tag` into a first-class variant.
    UnknownEventClass {
        tag: ModEventTag,
        payload: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModEventTag(pub String);
```

Constraints:
- `payload` is opaque bytes (Bincode 2 from the mod's emitter). Core readers do not introspect it.
- The event still participates in canonical hashing (the payload's bytes are part of the BLAKE3 sample), so two saves with the same mod set produce identical hashes; two with different mods do not. `mod_load_fingerprint` (Content/RULES §6) is stamped into the save header.
- A mod-defined event still contributes salience via the standard formula — `event_class_base_weight` comes from the mod's tuning declaration, not core. The mod's content pack supplies a `mod_event_weight_table.ron` mapping `ModEventTag → Q32`.
- Readers ignore `UnknownEventClass` for callback projection — a press conference does not surface mod events through core press templates. The mod must ship its own readers (Phase 6 mod-overlay work; out of scope for this ADR's schema lock).

The schema_version on a `MemoryEvent` with `UnknownEventClass` is still the core schema version. If the mod bumps the payload's internal layout, that's the mod's problem; core does not migrate mod payloads.

## Consequences

### Positive

- **Pillar 2 ships structurally.** The ledger + readers separation gives every player-facing surface (press, fan, scout, coach, alumni recall) one path to the same canonical history. The "kid you cut as captain in eight seasons" callback is a `BTreeMap<PlayerId, Vec<EventId>>` lookup, not an ad-hoc surface-specific store.
- **Pillar 3 is genuinely event-caused.** `signature_readiness` is the only path to a Breakthrough; the meter cannot fill on training ticks; the redraw fires only with a narrative-anchored gating event. The "growth lives in the ledger" claim is now mechanical, not rhetorical.
- **Determinism contract holds end-to-end.** `Q32` salience + stakes + readiness + pressure; `BTreeMap` indexes; `Vec` source-of-truth; no clocks (every time reference is `Tick` or `CareerDate` — both sim-time, not wall-time); no async; no system RNG. The pinned canonical-hash regression in `crates/fw-replay/tests/canonical_hash.rs` extends to cover a fixed-seed ledger scenario.
- **Compaction is well-defined.** At the 5-season boundary, the compactor drops `tick: Some(_)` to `None` and collapses Linear-decay events that have decayed below the compaction floor into summarised aggregates per `(subject, class, season)`. Never-decay events survive intact. Compaction is itself an append-only operation that emits a `Compaction` event recording what was dropped; the BLAKE3 hash captures the compaction action.
- **Mod-friendly without coupling.** `UnknownEventClass` round-trips through serde + hash without core knowing the mod's semantics. A mod author can ship a new event class without recompiling core.
- **Symmetric breakthrough / collapse.** Same schema, same redraw path, same cooldown discipline. No special-case code for "growth-up vs growth-down" — the narrative gate differs, the mechanism is one.
- **Numeric weights stay in design docs.** The salience formula, threshold seeds, redraw distributions, family-relevance table, decay half-lives are all in `docs/design/memory.md` + `docs/design/progression.md`. Tuning loops do not require an ADR revision — they require a tuning-doc revision.

### Negative

- **A lot of schema to lock at schema_version = 1.** 28 event classes plus 6 emitter kinds plus 6 participant roles is a wide surface. We accept this because forward migration is supported and the alternative (start with 3 event classes and grow) means breaking ledger compatibility every time we add a class. We pre-lock a generous catalogue and forward-migrate only on structural changes.
- **`UnknownEventClass` participates in canonical hashing.** This means a mod's internal payload representation is canonical-state. If a mod changes its payload encoding without bumping its mod-version, the BLAKE3 hash shifts and existing saves fail the determinism check. We mitigate by making mod authors stamp a payload-version inside the payload bytes; the mod-fingerprint already covers external version drift.
- **Readers need indexes the ledger doesn't maintain on the hot path.** Per-subject and per-class indexes are rebuilt lazily on first read after each append, which means the first read after a batch of appends pays the rebuild cost. Acceptable for a per-press-conference (~weekly) cadence; we measure at T3 implementation.
- **The narrative gate adds gameplay variability.** A breakthrough may sit at readiness=0.98 for a season because no gating event fires. This is by design (pillar 3 explicitly: "rare, earned, memorable") but means the meter is not a promise. We surface the meter only obliquely in commentary ("seems to be finding his stride") — never as a numeric tooltip (Sim/RULES §1, banned visible-stats UI).
- **PA redraws as the canonical mechanism conflict with the "PA as fixed cap" mental model in some shipped sims.** This is intentional. `07-player-attributes-progression.md` line 75 flags the open question; we resolve it by saying PA is mutable but only through ledger events, never through training. Tooling that surfaces "potential" to the player surfaces it as a scout-perceived range, never as a single number (pillar 4).

### Neutral

- **Five readers may grow.** A `BoardReader` for board-confidence summaries and an `OppositionScoutReader` (the opposing manager's view of our players) are candidates for Phase 4-5. The reader pattern is extensible; this ADR locks the initial five.
- **Compaction algorithm is a separate spec.** `docs/specs/compaction-strategy.md` (Phase 3, after T3 lands the ledger primitives) details which fields drop, which aggregate, and how callback eligibility survives. This ADR commits to the boundary (5 seasons) and the append-only Compaction event; the algorithm itself is design-doc-level tuning.
- **Mod readers are Phase 6.** Core readers ignore `UnknownEventClass`; a mod that wants its events to surface in commentary ships its own reader-side templates. The host engine reserves the right to add core readers for mod event tags in a future schema migration (which would promote `UnknownEventClass` to a first-class variant).

## Alternatives considered

- **Salience computed on the fly at every read instead of cached at emission.** Rejected: the formula's inputs (`participant_prominence_avg`, `rarity_boost`) change as the ledger grows, so a per-read recompute would shift historical salience in subtle ways — breaking the "what felt big at the time" semantic and inflating reader cost from O(1) to O(n) on the ledger. The current model caches emission-time salience as canonical and applies decay as a pure read-side projection over `elapsed`.
- **Mutable salience (events get re-scored as the world changes around them).** Rejected: violates append-only. Pillar 2's "the kid you cut as captain in eight seasons" works specifically because the original event's emission salience is preserved verbatim — the callback is "look how big this seemed then; look what it means now," and that requires the then-salience to be a stable historical fact. New related events get new rows; old rows do not mutate.
- **One reader, parametrised by surface kind.** Rejected: the five readers each have genuinely different cost profiles (PressReader rebuilds per-conference; CoachReader rebuilds per-training-week; SalienceReader is on-demand UI). Parametrising over kind would either force one big union of all rebuild paths (expensive) or push the cadence parameter to the call site (leaky). Five small readers each with a sharply-scoped contract is clearer.
- **Breakthrough mechanism = pure threshold (no narrative gate).** Rejected: this is what shipped sims do (Madden's "100-yard game unlocks dev trait" — a threshold + a cinematic, but no gating-event requirement). The meter-only model produces breakthroughs at predictable intervals once the player learns the trigger; pillar 3's intent is rare-and-earned. The narrative gate is the load-bearing differentiator from the shipped industry.
- **Breakthrough mechanism = pure event (no readiness meter).** Rejected: a single `LegacyGoal` should not trigger a breakthrough on its own — pillar 3 explicitly says growth is *accumulated* over many salient events. The meter is what aggregates the accumulation; the gating event is what gives the moment its specificity. Both are required.
- **Regressive collapse fires automatically when negative_pressure clears the threshold (no gate).** Rejected: same as positive — the narrative gate is what makes the moment readable. A pure-threshold collapse would feel arbitrary ("why did my striker suddenly lose Finishing?"); a gated collapse reads as "the missed penalty in the relegation playoff is what broke him."
- **Regressive collapse is non-reversible.** Considered, rejected: pillar 3 specifically says "Equal gravity for regressive triggers" but also "rare and memorable." If collapses cannot be reversed, the design pressure on the player is to play conservatively to avoid them — which violates the design intent that the player takes risks because the world rewards memorable arcs. Reversibility through subsequent breakthrough preserves the arc-friendly design pressure. The PA delta is permanent unless a subsequent positive event redraws upward; the regressive_pressure resets, so the next arc starts fresh.
- **`UnknownEventClass` skipped during canonical hashing to make mods stack cleanly.** Rejected: skipping mod events from the hash would mean two saves with different mod sets could produce identical BLAKE3 hashes, which silently breaks the cross-OS regression guarantee for mod users. Better to include the payload bytes and stamp the mod fingerprint into the save header so divergences are explicit.
- **Stakes and salience as `u16` basis points (0..=10000) directly, bypassing Q32.** Considered, rejected: the salience formula multiplies Q32 inputs together; converting in and out of basis points at every multiplication is friction for no determinism benefit (Q32 is already bit-exact cross-platform). The DESIGN_DOC §7 audit note proposes "Q32 (or equivalently `u16` basis points)" — we pick Q32 as the canonical wire type and let `from_basis_points` / `to_basis_points` exist as accessors for the UI projection layer.

## Revisions to DESIGN_DOC §7

This ADR's acceptance lands the following corrections to DESIGN_DOC §7:

1. **`stakes` type:** `f32 in [0, 1]` → `Q32 in [0, 1]` (canonical). The audit note at §7 line 192 already caveats this in prose; the §7 struct sketch is updated in-place when T3 begins to drop the `f32` annotation.
2. **`salience` type:** same correction, same reason.
3. **`tick`:** clarified as `Option<Tick>` to support compaction's tick-dropping behavior. §7's current `tick?` notation is ambiguous about whether the option survives compaction; this ADR makes it explicit.
4. **Reader catalogue:** §7 lists five readers (Alumni DB, Rival recall, Promise tracking, Big-match scars, Press/fan callbacks). This ADR consolidates them as `SalienceReader / PressReader / FanReader / ScoutReader / CoachReader` with the §7 functions absorbed: Alumni DB and Rival recall are queries on `SalienceReader`; Promise tracking is a `CoachReader` projection; Big-match scars is a `CoachReader` + `SalienceReader` query. The five DESIGN_DOC names map to the five ADR names without losing any function.
5. **Compaction:** §7 says "5-season boundary, summarised state, preserves callback eligibility, drops tick granularity." This ADR adds: compaction itself emits a `MemoryEvent::Compaction` (append-only invariant preserved), and `tick: Option<Tick>` is the field that goes to `None` for compacted events.

DESIGN_DOC §7 prose is updated on this ADR's acceptance; no separate `/log-decision` entry is required (§7 is design-doc surface, not append-only decisions). The ADR + the §7 edit are the audit trail.

## References

- `docs/DESIGN_DOC.md` §3 (pillars 2, 3, 4) and §7 (event-sourced ledger sketch)
- `docs/MASTER_PLAN.md` Phase T3 (memory + breakthroughs delivery slot)
- `CLAUDE.md` §1 (pillar text), §3 (tech stack + determinism), §7 (code style)
- `.claude/rules/Sim/RULES.md` §1-§5 (the binding determinism contract for fw-memory)
- `.claude/rules/Content/RULES.md` §6 (mod overlay rules, mod_load_fingerprint)
- `docs/research/sports-sims/00-synthesis.md` lines 103-118 (memory + breakthroughs synthesis), lines 122-130 (genuinely-differentiated finding)
- `docs/research/sports-sims/03-non-sport-emergent-sims.md` lines 25-46 (DF facets, CK3 trait deltas, RimWorld thoughts, Sims utility) and lines 64-69 (direct application to FW T3)
- `docs/research/sports-sims/07-player-attributes-progression.md` lines 60-65 (visible/hidden split, CA/PA), lines 66-72 (FW application: PA-redraw breakthrough), lines 73-77 (open questions on CA/PA + hidden surfacing)
- `crates/fw-memory/src/lib.rs` (Phase-0 stub — extended at T3 to the schema above)
- `crates/fw-core/src/q32.rs` (canonical Q32 primitive)
- `crates/fw-core/src/ids.rs` (PlayerId, ClubId, MatchId — used as participant references)
- Prior ADRs: ADR-0001 (match engine architecture — the emitter of most match-class events)
- Pending companions: `docs/design/memory.md` (salience weights, decay half-lives, family-relevance table, threshold seeds, cooldown seeds), `docs/design/progression.md` (attribute-family list, redraw distributions, gating-event table, ca_lift_fraction, attribute floors)
- Pending decision (`/log-decision` on T3 entry): whether to ship Phase 6 mod-reader hooks in core or punt to a `fw-mod-bridge` crate
- ADR-0006 (forthcoming, pre-T1-2b): per-player decision representation (BT vs FSM). Independent of this ADR; the memory ledger consumes match events regardless of which one wins.

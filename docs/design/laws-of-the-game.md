# Laws of the Game — match-rules track (FUN-LAW)

**Status:** DESIGN (research-grounded; 2026-06-04). The spec for adding a believable subset of the
real football rulebook to the match engine. **Owner:** `systems-designer` (mechanic + calibration) +
`gameplay-programmer` (engine) + `narrative-director` (commentary grammars per new event).
**Grounded in:** IFAB Laws of the Game 2025/26; FM-class rule-modelling patterns; public aggregate
top-flight event rates (see `match-realism-reference.md` for the broader anchor set). Closes the
ultra-review / owner-flagged gap: *the engine has almost no actual football rules*.

**Procedural-fantasy guard:** rules are modelled as deterministic mechanics keyed off player genes +
tactics + a seeded referee coefficient — calibrated to AGGREGATE rates only. No licensed referee,
player, or club data.

---

## §1. The gap, and the key finding

**Today** the engine emits only `KickOff / FullTime / Goal / Shot / Pass / SignatureFirstFired` (6
`MatchEvent` discriminants). **Offside** lands next as discriminant 6 inside **FUN-TS2** (it ships with
the coordinated press — a held line needs offside to have teeth; specced in `tactical-shape.md`).
Everything else in the rulebook — fouls, free kicks, cards, corners, throw-ins, goal kicks, penalties
as real awarded events — is absent. A match has no turnovers-from-stoppage, no set-piece sequences, no
personnel consequences. That is the FM-recognisable spine this track adds.

**The key architectural finding (verified in code):** the engine *already anticipates this entire
track*. `tactic_fsm::SetPieceKind` already enumerates **all 11 restart classes** (`KickOff, GoalKick,
GoalKickOpponent, CornerFor/Against, FreeKickFor/Against, ThrowInFor/Against, PenaltyFor/Against`);
`setpiece_kind_for` (`lib.rs`) already maps ball-out-of-bounds geometry + last-touch → the correct
reciprocal corner/throw-in/goal-kick; and `apply_event(BallOutOfPlay{kind})` / `BallInPlay` already
drive both teams' FSM into and out of `SetPiece(kind)`. **So fouls/free-kicks/penalties are the missing
*producers* feeding kinds the FSM already understands, and cards are pure bookkeeping.** This is reuse,
not reinvention. The one genuinely new subsystem is restart *timing* (§5.3) — and the deferral comment
already in `auto_exit_setpiece` names exactly that as the future work.

---

## §2. Tiering — MUST-HAVE vs LATER

**MUST-HAVE (the believable spine):**
- Restart attribution + emission: throw-in / goal-kick / corner from last-touch + which line was crossed (geometry already computed; only the `MatchEvent` emission + restart *timing* are missing).
- Contact foul → direct free kick (DFK); DFK-class foul inside the box → **penalty** awarded.
- The card model: yellow / second-yellow→red / direct red, with a persistent **10-man** state for the rest of the match. The DOGSO (deny obvious goal-scoring opportunity)→red and SPA (stop promising attack)→yellow decisions.
- Deliberate-handball as a foul-class event.
- **Advantage** as a play-on-vs-stop-play decision gate (without it, every foul stops play and the match feels broken).
- Penalty as a real awarded event (reusing the shot/GK-save pipeline at a forced high base-xG).
- Offside-IFK (lands with FUN-TS2).

**LATER (fidelity, same event-model shape):** IFK taxonomy (dangerous play, impeding, GK back-pass,
GK 8-second→corner, dissent); dropped ball; penalty retakes + illegal-feint; non-deliberate-handball-PK
card leniency; handball-creates-goal disallow; advantage *revocation*; injuries/stoppages; suspension
accumulation across a season (a ledger concern, not a match-engine one).

Everything in LATER adds triggers without changing the event-model shape (same discriminants + same
SetPiece FSM).

---

## §3. Calibration rates — what a believable match emits

Per match (both teams) unless noted. Tune the engine's emergent rates to these.

| Event | Per-match target | Firmness | Note |
|---|---|---|---|
| Fouls | **~21.5** (~10.75/team) | HARD (~20-24 across eras) | The root event — cards/FKs/pens derive from it. |
| Offsides | **~3.8** (~1.9/team) | HARD | Cross-checks `match-realism-reference.md` §3 (~3-4 combined). |
| Corners | **~10** (~5/team) | HARD (~9.5-10.2) | |
| Throw-ins | **~40** (~20/team) | HARD-ish (wide ~35-50) | |
| Free kicks | **~25-35** | SOFT (≈ fouls minus advantage-played) | Derive from fouls, don't tune independently. |
| Yellow cards | **~4.2** (2023-24 record high) → **neutral baseline ~3.5** | SOFT (era-dependent) | Scale up under aggressive tactics / strict ref. |
| Red cards | **~0.23** → **neutral baseline ~0.15** | SOFT (~0.10-0.23/season) | |
| Penalties | **~0.28 awarded, ~0.26 scored** | SOFT (~0.14-0.33; VAR-era outliers) | A small subset of in-box fouls. |
| Goals from set pieces | **~20% of all goals** (non-pen) + pens on top | SOFT (rising trend) | Routes a fifth of goals through the restart machinery. |

**Internal consistency to ENFORCE** (the modelling discipline that keeps all four rates coherent):
model **fouls as the root event**, then derive cards (~1 per ~5 fouls league-wide), free kicks (≈ fouls
minus advantage), and penalties (in-box foul subset) as *conditional branches* — never as independent
dice. 2023-24 was a record-high card season; the neutral baseline above (~3.5 yellow / ~0.15 red) gives
a timeless feel that scales up under aggressive tactics and strict refs rather than baking the 2020s
litigiousness in. *Sources: StatMuse / MyFootballFacts / Opta Analyst / soccerstats / cornervalue /
Sheffield Hallam throw-in study / Barça Innovation Hub injury epidemiology — all public aggregates;
verify against FBref/Opta for a shipped calibration.*

---

## §4. Modelling philosophy — emergent, FM-style, deterministic

FM models discipline as an **emergent output of player + tactic + referee**, not scripted events. We
adopt the same pattern, mapped onto our determinism contract:

- **Per-player aggression/discipline gene** (hidden numeric) drives foul/card propensity — surfaced only as commentary + scout prose ("gets stuck in", "a competitive streak"), never as a visible stat. Fits Pillar 1 + the no-visible-stats rule.
- **Tactical aggression amplifies it.** Foul/card rate = f(player aggression × team tactical aggression × pressing intensity). A high press / aggressive tackling tactic measurably raises the booking rate; a contain/stay-on-feet tactic lowers it.
- **Referee strictness is a per-match coefficient drawn ONCE, seeded** (deterministic — a dedicated `site` under an appropriate `SeedLayer`, drawn at kickoff, not per-tick). This reproduces FM's match-to-match card-density swings *without* per-tick randomness fighting the canonical-hash floor — the single highest-leverage knob for believable variation. It scales the league-baseline foul→card conversion.
- **Fouls are the root event; cards / free kicks / penalties are conditional branches** off the foul roll + its severity — keeping the four rates mutually coherent under the single ref-strictness × tactical-aggression multiplier.

---

## §5. Engine integration

### §5.1 New MatchEvent variants (append-only discriminants 7-12)

Discriminants are append-only and do-not-reorder (the cross-crate discriminant pin + `encode_match_event`
agree on bytes). Offside takes 6 (FUN-TS2). This track appends:

| Disc | Variant | Payload sketch |
|---|---|---|
| 7 | `Foul` | offender_slot, victim_slot, tick, restart: SetPieceKind (FreeKick*/Penalty*), severity: FoulSeverity |
| 8 | `Card` | player_slot, tick, colour: CardColour (Yellow/Red), reason: CardReason |
| 9 | `FreeKick` | taker_slot, tick, kind: FreeKickKind (Direct/Indirect), is_penalty: bool |
| 10 | `Corner` | taker_slot, tick, side |
| 11 | `ThrowIn` | taker_slot, tick |
| 12 | `GoalKick` | taker_slot, tick |

Sub-enums mirror the existing `PassKind` pattern (`#[repr(u8)]`, hand-assigned discriminants,
do-not-reorder, encoder writes the tag byte explicitly): `FoulSeverity {Careless, Reckless,
ExcessiveForce}`, `CardColour {Yellow, Red}`, `CardReason {Foul, Dissent, SecondYellow, …}`,
`FreeKickKind {Direct, Indirect}`. Each new discriminant must grow the `MatchEventDiscriminant::all()`
array **and** ship a Tracery commentary grammar (the `ContentStore` load-validation gate rejects a
discriminant with no grammar — narrative-director owes a bank per variant).

### §5.2 Foul = a failed tackle that makes contact

Rides directly on `resolve_tackles`. Today a failed tackle just sets a cooldown. Insert a second roll on
the **failure branch** (under `SeedLayer::ReactiveInterrupt`, a fresh reserved `site` to avoid stream
collision with the existing save/tackle/dispersion sites): `p_foul = FOUL_BASE × aggression ×
contact_proximity`. On a foul: box-test the carrier position (Q32) → `PenaltyFor/Against` if inside the
defending penalty area, else `FreeKickFor/Against`; clear possession; push `MatchEvent::Foul`; fire the
**existing** `apply_event(BallOutOfPlay{kind})` path on both teams so the FSM enters `SetPiece`. At most
one foul per tick (a `break`, like the tackle-success path). This is the biggest reuse win — fouls are
just an additional caller of the OOB→SetPiece path that corners/throw-ins already use.

`FoulSeverity` comes from the same roll, banded (most → Careless → no card; tail → Reckless → Yellow;
far tail → ExcessiveForce → Red), scaled by the per-match ref coefficient.

### §5.3 Restart timing — the one genuinely new subsystem

`auto_exit_setpiece` is currently a hand-wave ("first dribble/pass off the boundary"); its own comment
defers "true set-piece restart timing (countdown, ball reposition to taker, possession to taker)" — that
deferral is this track's centrepiece. Add a canonical `restart_countdown: Option<(SetPieceKind,
PlayerSlot, Tick)>` on `MatchState`: on entering a SetPiece, reposition the ball to the restart spot,
give possession to the taker, and gate dispatch until `tick >= restart_tick`; on expiry emit the taker's
`FreeKick/Corner/ThrowIn/GoalKick` event + `BallInPlay`. A new tick step, shared by *every* set-piece —
so it retroactively makes the existing corner/throw/goal-kick believable too.

### §5.4 Cards, dismissals, penalties

New canonical fields `yellow_cards: [u8; 22]` + `sent_off: [bool; 22]` on `MatchState`; second yellow
auto-emits a red; sent-off slots are skipped in the `dispatch_tick` per-slot loop + excluded from
tackle/pickup/separation. **10-man behaviour is then emergent from the shape system** — no special-casing
beyond the skip (this is where FUN-TS pays off: a believable block with a man missing degrades
believably). Penalty: a `PenaltyFor` restart repositions to the spot; on `BallInPlay` the taker fires a
forced high-base-xG shot through the **existing** SS3 GK-save pipeline — penalties reuse the whole
shot/save path.

---

## §6. Phased slices

| Slice | Scope | New canonical | proptest invariants | Rebaseline |
|---|---|---|---|---|
| **FUN-TS2 (offside)** | disc 6; flag at pass-launch; possession→IFK/GK restart (specced in `tactical-shape.md`) | event only | receiver-beyond-2nd-rearmost-at-launch flagged; equal=onside; backward-pass never flags; only at pass-launch tick | schema + behavioural |
| **FUN-LAW1 — restart timing** | `restart_countdown` field; reposition + taker-possession + N-tick gate; emit Corner/ThrowIn/GoalKick at the already-firing OOB seam | field bump + disc 10/11/12 | every BallOutOfPlay yields exactly one restart event; ball at spot during countdown; dispatch gated until expiry; taker on correct team | schema + behavioural |
| **FUN-LAW2 — fouls + free kicks** | foul roll on tackle-failure branch; box-test → Penalty vs FreeKick; `Foul` (7) + `FreeKick` (9); FSM via existing OOB path | none beyond LAW1 | foul only on failed-tackle-with-contact; in-box ⇒ PenaltyFor; outside ⇒ FreeKick; possession reverts to fouled team; fouls/match in ~18-28 band (drama-sweep, informational) | behavioural |
| **FUN-LAW3 — cards + dismissals** | `yellow_cards`/`sent_off` fields; severity banding; 2nd-yellow auto-red; sent-off skip | field bump + `Card` (8) | 2nd yellow ⇒ red; red ⇒ never decides/tackles again; cards/match ≈ real band; team plays 10 | schema + behavioural |
| **FUN-LAW4 — penalty as a real event** | forced high-base-xG shot through SS3 on PenaltyFor; `is_penalty` wiring | none | penalty fires exactly one shot vs GK; conversion ~0.70-0.82 over N seeds; only from in-box foul | behavioural |

Every slice touches `match_events` content (and most add fields), so **every slice is a pinned-hash
rebaseline** — follow the multi-pin discipline (60-tick smoke + 600-tick extended): authorize per slice,
envelope-verify the drama-sweep rates fall in band **before** re-pinning, document ADR-0012 trigger #1
(schema) vs #3 (behaviour) in the commit body.

---

## §7. Sequencing relative to team-shape (FUN-TS)

- **Offside is part of FUN-TS2 by design** — don't separate it; it depends on FUN-TS1's defensive line existing and ships with the press.
- **FUN-LAW1 (restart timing) is shape-independent** and improves the *existing* corner/throw/goal-kick believability — a reasonable FUN-TS3/4-adjacent task.
- **FUN-LAW3 card-accumulation bookkeeping is pure ledger math** — genuinely independent, can land any time.
- **FUN-LAW2 / FUN-LAW4 (fouls, penalties) hard-depend on a believable block** — a foul model running against the current swarm would cluster unrealistically. Land them **after FUN-TS4 (integration)** so fouls happen in real shape.

---

## §8. Deferred

IFK taxonomy (dangerous play, impeding, GK back-pass, GK 8-sec→corner, dissent); dropped ball; penalty
retakes + illegal feint; non-deliberate-handball-PK card leniency; handball-creates-goal disallow;
advantage *revocation* (the play-on gate itself is MUST-HAVE); injuries/stoppages; cross-season
suspension accumulation (append-only ledger, not match-engine).

## Cross-references

- `docs/design/tactical-shape.md` — FUN-TS1..4 + offside (FUN-TS2); the believable base these rules attach to.
- `docs/design/match-realism-reference.md` — the broader aggregate-rate anchors (offsides cross-check; set-piece goal share).
- `docs/design/drama-model.md` — the drama-sweep harness that will measure fouls/cards/restart rates fall in band.
- `docs/adr/0012-canonical-rebaseline-policy.md` — the per-slice rebaseline discipline.
- `crates/fw-content/src/event.rs` (discriminant table + `all()`), `crates/fw-match-sim/src/canonical.rs` (encoder VERSION), `crates/fw-match-sim/src/tactic_fsm.rs` (`SetPieceKind`), `crates/fw-match-sim/src/lib.rs` (`setpiece_kind_for` / `auto_exit_setpiece` / OOB seam), `crates/fw-match-sim/src/dispatch.rs` (`resolve_tackles` / per-slot loop) — the integration seams.
- IFAB Laws of the Game 2025/26: https://downloads.theifab.com/downloads/laws-of-the-game-2025-26-single-pages?l=en

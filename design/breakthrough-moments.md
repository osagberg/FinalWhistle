---
description: Breakthrough mechanism design — signature awakening, latent-flag unlock, regressive collapse. How players permanently change via ledger-event accumulation in a text-first sim.
last_verified: 2026-05-20
status: Ported from Unity-era archive (design/breakthrough-moments.md, locked 2026-04-24); cinematic apparatus dropped per FW v2 text-first pivot (CLAUDE.md: "No 3D viewer. No manga-broadcast cinematic mode."); reconciled to ADR-0005's per-attribute-family mechanism (the authoritative FW v2 shape); T3-4 unblocked.
---

# Breakthrough Moments

## Purpose

Answer "how do players permanently change mid-career via in-match events, in a way that feels earned and football-grounded, without QTE pop-ups pulling the player out of the management fantasy?"

This doc covers the mechanism design. Numeric tuning values (thresholds, redraw distributions, family-relevance weights, cooldowns) live in `docs/design/progression.md` per `design-docs/RULES.md §4`.

## Mechanism overview

ADR-0005 §"Breakthrough mechanism" is the authoritative FW v2 design. The summary below cross-references it directly; consult the ADR for the canonical schema.

The career system maintains two meters per `(player, attribute_family)` pair:

- `signature_readiness: Q32` in `[0, 1]` — ticks upward on salient family-relevant events; gates a positive PA redraw (`BreakthroughMoment`).
- `regressive_pressure: Q32` in `[0, 1]` — ticks upward on negative-class events; gates a negative PA redraw (`RegressiveCollapse`).

Both meters are `Q32` fixed-point (`fixed::FixedI64<U32>`) per `Sim/RULES.md §1`. No `f32` or `f64` in canonical state.

When a meter crosses its threshold AND a narrative gate passes, the career system emits a `MemoryEvent` with `decay_function: Never` and applies a PA redraw in the affected family. The mechanism is one; the three trigger kinds below are three **gating flavors** of the same path.

## Trigger kinds

### Kind 1 — Signature awakening

A `BreakthroughMoment` in an attribute family where the player holds a pending `SignatureCandidate` tied to that family. The three-part narrative gate (ADR-0005 §"Breakthrough mechanism", conditions 1-3) additionally requires that at least one of the player's `signature_candidates` maps to the breakthrough family.

When the gate passes, the breakthrough event carries a `Consequence::SignatureActivated { signature_id }` alongside the standard PA-redraw consequence. The signature transitions from candidate to active on the player's card. Commentary surfaces this as football-native text: the player is described doing the thing, not being told they have unlocked a thing.

The archive's Kind 1 framing ("viewer punches in, player-isolation shot, crowd-reaction cutaway, aftermath-freeze") is dropped entirely — FW v2 has no cinematic viewer. The two-tier text-recap pattern (Q2) replaces it.

### Kind 2 — Latent-flag unlock

A `BreakthroughMoment` whose narrative gate (condition 2 in ADR-0005) requires one of the four narrative-trigger gene flags — `late_bloomer`, `flow_access`, `peak_ceiling_high`, or `awakening_dormant` — to be present in the player's `GeneSnapshot.narrative_flags` AND for the gating event to match the flag's unlock condition.

The flag is a gating **precondition**, not a separate mechanism. The readiness meter accumulates identically to Kind 1; the difference is that the gating event table for flag-gated families is narrower. A `late_bloomer` player's Finishing breakthrough gates only on `LegacyGoal` or `HatTrickScored` in a high-stakes match — not on routine goals. This makes late-bloomer breakthroughs rarer and more specific, which is the design intent.

After a flag-gated breakthrough fires, the flag's unlock condition is marked met in the player's development-hook record. The flag itself is never surfaced in player-facing UI. Post-match text might read "Something clicked for him today. Scouts will revise." — the flag is architectural, not lexical, per `design/player-generation.md §D`.

Narrative trigger flags are **never observable** by scouts (zero weight in all scout archetypes, per `design/player-generation.md` §"Gene-category visibility"). They surface only retroactively via the events they cause — generating a phenotype label (e.g., `Late Bloomer`) once the unlock condition is met.

### Kind 3 — Regressive collapse

The symmetric negative path. `regressive_pressure` accumulates from negative-class events; at threshold (tuning seed: 0.9, see `docs/design/progression.md`), with a narrative-anchored gating event, the career system emits `MemoryEvent { event_class: RegressiveCollapse, decay_function: Never, ... }` and redraws PA **downward** in the affected family.

ADR-0005 §"Regressive collapse" is the authoritative shape. Key points:

- The redraw is bounded — never below the career-floor (`max(20, current_ca − 30)`), so a collapse degrades but does not erase a player.
- Reversibility: a `RegressiveCollapse` does not auto-recover. The only path back is a subsequent `BreakthroughMoment` in the same family — after the cooldown (18 in-game months for regressive, vs 12 for positive).
- The archive's permanent negative labels (`fragile`, `confidence_fractured`, `ceiling_compressed`) are **replaced** by this reversible mechanism. ADR-0005 wins on this point. Collapses leave PA-level scars; those scars are overwritable by subsequent positive arcs.

**Regressive parity (Q4, reconciled).** The archive's Q4 says "same gravity as positive breakthroughs — same cinema duration, same shot chain, same two-tier text pattern." The cinema is gone in FW v2. The gravity survives as: same `decay_function: Never`, same `salience` weight at emission, same text-recap weight in the post-match report. A save where triumphs get a paragraph and collapses get a stat-line violates pillar 1 (consequences stick). Regressive moments get the same recap treatment as positive ones.

## The three-part narrative gate

Per ADR-0005 §"Breakthrough mechanism", all three trigger kinds share this gate structure:

1. `signature_readiness[player, family] >= breakthrough_threshold` (tuning seed: 0.92 — see `docs/design/progression.md`). Note: the archive doc's seed of 0.85 is superseded by ADR-0005's 0.92; the ADR is authoritative.
2. A **gating event** is emitted in the same tick — an `EventClass` that thematically matches the family and the trigger kind. The gating-event table by family is in `docs/design/progression.md`.
3. The cooldown check passes — the player has not fired a breakthrough or regressive collapse in the last 12 in-game months (18 months for regressive collapses). See `docs/design/progression.md` for the full cooldown seeds.

A meter at 1.0 is necessary but not sufficient. The narrative gate is the load-bearing differentiator from shipped sims (ADR-0005 §"Alternatives considered" — "Breakthrough mechanism = pure threshold" is explicitly rejected).

## Meter accumulation

The readiness delta formula (ADR-0005 §"Breakthrough mechanism"):

```
readiness_delta = projected_salience(event) × family_relevance(event.class, family)
new_readiness   = clamp(old_readiness + readiness_delta, Q32::ZERO, Q32::ONE)
```

`family_relevance` is a per-`(EventClass, attribute_family)` lookup table in `docs/design/progression.md`. Most `(EventClass, family)` pairs have zero relevance; the non-zero cells are the signal.

After a breakthrough fires, `signature_readiness[player, family]` resets to the residue seed (0.15) — a small non-zero residue so the curve feels continuous rather than resetting to zero.

## PA / CA redraw

Per ADR-0005 §"Breakthrough mechanism":

```
delta_pa[family] = redraw_distribution(family, stakes, age_curve_position)
new_pa           = clamp(old_pa + delta_pa, pa_min, pa_max)
new_ca           = clamp(old_ca + ca_lift_fraction · delta_pa, ca_min, new_pa)
```

`ca_lift_fraction` seed: 0.5 — the floor catches up halfway to the new ceiling. The redraw distributions by family are in `docs/design/progression.md`. The `BreakthroughMoment` event carries the deltas in its `consequence` field so readers (commentary, scout, press) can phrase the lift specifically.

All arithmetic is Q32 with checked addition. Any overflow is a `proptest` invariant violation and a `panic!` in release per `Sim/RULES.md §11`.

## Determinism contract

Any RNG draw (the `redraw_distribution` sample at breakthrough fire) uses `ChaCha8Rng::seed_from_u64(seed_fn(match_seed, tick, SeedLayer::SignatureTrigger, site))` per ADR-0009. `SeedLayer::SignatureTrigger` exists for exactly this. No `thread_rng()`, no `Instant::now()`, no float arithmetic. `signature_readiness` and `regressive_pressure` are `Q32` canonical state; they serialize via `serde` in the `fw-save` snapshot.

## Text-recap surfaces (Q2 reconciled)

The archive's Q2 defines a two-tier observational-phrase pattern. In FW v2 this maps to:

**Tier 1 — Quiet observational phrase** (match-day commentary, woven into match events):
> "He's found something." / "That's new." / "Third time today."

**Tier 2 — Match-specific follow-up** (post-match report):
> "He cut inside again — and this time he went through." / "Mendez had been looking for that run all half."

These phrases are Tracery template slots in the commentary phrase bank. The `narrative-director` agent owns tone. Text describes football behavior, not progression mechanics — if copy could appear in a live broadcast commentator's line, it's probably right; if it could appear on a stat-sheet readout, it's probably wrong.

<!-- ui-lint:ignore-start reason="banned-vocabulary callout enumerating the lint's own targets" -->
**Banned vocabulary (enforced via `docs/design/ui-vocabulary.md` lint):**
- ~~"Signature unlocked"~~, ~~"Awakened"~~, ~~"The Hush"~~, ~~"Calling"~~ — no mystical or capitalized state nouns
- ~~"XP gained"~~, ~~"Level up"~~, ~~"+5 finishing"~~ — no progression-mechanic menu vocabulary
<!-- ui-lint:ignore-end -->

## Near-miss handling (Q3)

**1st near-miss in a match:** silent. No text card. Readiness accumulates quietly.

**2nd+ near-miss same match:** post-match text card — e.g., "Found the cutback position twice today. Not quite there yet."

Never a live-match "Close!" popup — that is the farming failure mode. Explicit near-miss surfacing trains players to game the system (selecting for near-miss conditions rather than natural development). Silence-until-pattern keeps breakthroughs feeling earned.

## Surface timing (Q5 reconciled)

The archive's Q5 defines live-fire timing rules for when cinema may interrupt play. In FW v2 there is no cinema and no live match viewer. The surface timing rule reduces to:

**The text recap surfaces in the post-match report.** The triggering event (the gating `EventClass` emission) is identified as part of the match log; the post-match report surfaces the Tier 2 phrase anchored to that event. No live interruption exists.

## Manager indirect influence

Managers shape breakthrough likelihood through:

- **Tactics** — playing a young CM in a box-to-box role accumulates `WorkRate` and `Composure` readiness from relevant event types.
- **Selection** — minutes in role matter; rotating a player out of their affinity role stalls readiness accumulation in the relevant family.
- **Training** — training focus biases which event types are generated (and therefore which readiness meters tick). Implementation detail for T3-4.
- **Promises** — `PromisedYouthMinutes` events affect the confidence curve via `regressive_pressure` if broken (`BrokenPromise` is a pressure-positive event).
- **Pressure exposure** — playing a player in high-stakes matches generates `BigMatchScar` (negative) or `LegacyGoal`-class events (positive) depending on outcome and the player's `composure_floor` gene.

No mid-match pop-up ever lets the player "choose" a breakthrough. The manager earns it over sessions of right and wrong choices.

## MVP boundary and acceptance target

**T3-4 MVP:** all three trigger kinds operational. The acceptance target is: **across a 5-season career, 1-3 breakthroughs fire per player on average**. Players with 2-3 `signature_candidates` (top-flight starters) are at the upper end; depth players and journeymen with 0 candidates fire rarely or not at all. The back-of-envelope for this target is in `docs/design/progression.md` §"Cadence math."

**Post-MVP deferred items:**

<!-- ui-lint:ignore-start reason="deferred-item technical description — references system mechanics by internal names" -->
- Signature evolution (an active signature evolves via continued use after awakening) — post-MVP
<!-- ui-lint:ignore-end -->
- Awakenings triggering rivalry or relationship events — post-MVP
- External trainers or mentors triggering specific awakenings — post-MVP
- A `BigMatchAppetite`-style gene as a breakthrough modifier (amplifies readiness delta in knockout stages) — post-MVP; blocked on T3+ personality long-tail readers

## Cross-references

- `docs/adr/0005-memory-ledger-and-breakthroughs.md` — **authoritative FW v2 mechanism**. This doc is the design layer on top; the ADR is the structural contract.
- `docs/design/progression.md` — all numeric tuning values: thresholds, redraw distributions, family-relevance table, gating-event table, cooldowns, cadence math.
- `design/player-generation.md` §D — narrative trigger flags (the 4 flags that gate Kind 2).
- `crates/fw-memory/src/event.rs` — `EventClass::BreakthroughMoment` and `EventClass::RegressiveCollapse` (shipped at T3-1).
- `docs/design/ui-vocabulary.md` — banned-terms catalog for player-facing copy.
- `docs/research/sports-sims/07-player-attributes-progression.md` lines 71-72 — the explicit rejection of the XP-plus-cinematic pattern.

## Port notes (FW v1 → FW v2)

- **Cinema dropped entirely.** Q1 (cinema beat duration, 3-5s seed) is removed. Q2 / Q3 / Q4 survive with cinema framing replaced by text-recap framing. Q5 reduces to "post-match report is the surface."
- **3 kinds → 3 gating flavors of one mechanism.** The archive treated Kind 1 / Kind 2 / Kind 3 as three distinct mechanically separate systems. ADR-0005 unifies them under one meter model; this port preserves the narrative distinction while aligning the mechanics.
- **Readiness threshold 0.85 → 0.92.** ADR-0005 supersedes the archive doc on this value.
- **Permanent negative labels removed.** Archive's `fragile`, `confidence_fractured`, `ceiling_compressed` as permanent labels are replaced by ADR-0005's reversible `RegressiveCollapse` + career-floor bounding.
- **Phase/milestone numbering aligned to MASTER_PLAN T-N.** Archive references "Month 3 slice" and "Phase 4/5/6" — these map to T3-4 (MVP), T4+ (balance harness).

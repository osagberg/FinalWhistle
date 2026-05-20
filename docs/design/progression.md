# Progression tuning seeds — breakthrough thresholds, family-relevance weights, redraw distributions

**Status:** Phase-3 tuning seeds. All values in this doc are expected to drift through balancing and T3 user testing. Per `design-docs/RULES.md §4`, tuning coefficients live here, NOT in DECISIONS.md and NOT in SPEC.

**Implements:** ADR-0005 §"Breakthrough mechanism" + §"Regressive collapse". Numeric values deferred here from the ADR; this doc locks the Phase-3 seeds.

**Companion doc:** `design/breakthrough-moments.md` (mechanism design).

---

## Scope

This doc defines:

1. The attribute-family list (the coarse groupings `signature_readiness` tracks per player).
2. The `family_relevance` table — per-`(EventClass, attribute_family)` weights.
3. The gating-event table — which `EventClass` instances can serve as the narrative gate for a breakthrough in each family.
4. The `redraw_distribution` per family — PA-redraw magnitude on breakthrough fire.
5. Threshold + cadence seeds: breakthrough threshold, regressive threshold, residue, `ca_lift_fraction`, cooldowns, career-floor formula.
6. Cadence math — the back-of-envelope showing why these seeds yield ~1-3 breakthroughs per player per 5-season career.

---

## Attribute-family list

ADR-0005 §"Breakthrough mechanism" line 270 names examples (`Finishing`, `Passing`, `DefensiveAnticipation`, `AerialPresence`, `Composure`, `Pace`, `WorkRate`, ...) and defers the exact list here.

**10 coarse families (Phase-3 seeds):**

| Family | Gene-model anchors (from `design/player-generation.md`) | Football-role relevance |
|---|---|---|
| `Finishing` | `striking`, `first_touch`, `fast_twitch_ratio` | Strikers, AMs — conversion, composure in the box |
| `Passing` | `pattern_recognition`, `decision_velocity`, `first_touch` | Midfielders, full-backs — range, vision, precision |
| `DefensiveAnticipation` | `pattern_recognition`, `decision_velocity`, `composure_floor` | Centre-backs, DMs — reading the game, positioning |
| `AerialPresence` | `aerial`, `height_ceiling`, `frame_density` | Centre-backs, target men — heading, physical duels |
| `Composure` | `composure_floor`, `mentality`, `ambition` | All positions — pressure response, decision quality under duress |
| `Pace` | `fast_twitch_ratio`, `growth_curve` | Wingers, strikers, attacking full-backs — explosive speed, acceleration |
| `Stamina` | `stamina_recovery`, `aging_curve`, `injury_resilience` | All positions — late-match intensity, injury-load resilience |
| `WorkRate` | `ambition`, `learning_rate`, `mentality` | Pressing mids, box-to-box — tracking-back, pressing trigger, shuttle runs |
| `DeadBallDelivery` | `dead_ball`, `first_touch`, `left_foot` | Specialist DM / AM / full-back — set-pieces, free kicks, penalties |
| `Leadership` | `mentality`, `composure_floor`, `ambition` | Captains, senior figures — dressing-room influence, mentoring yield |

**Why 10, not 8 or 12:** 8 is too coarse — a single "Physical" family would make `InjuryLongTerm` events simultaneously harm `Pace` and `Stamina`, which reads wrong (a hamstring injury shouldn't collapse a player's aerial ability). 12 starts overlapping with ADR-0002's per-attribute granularity; the families are meant to be *coarser* than individual attributes. 10 gives clean separation between contact-dependent (`AerialPresence`), pure-speed (`Pace`), and endurance-load (`Stamina`) without creating redundant families. `Stamina` is explicitly split from `Pace` because `InjuryLongTerm` pressure maps cleanly to endurance, not acceleration.

**`Leadership` note:** this family has no direct `signature_readiness` accumulation in the standard event loop — leadership breakthroughs gate only on `MentorTeammate` (as the mentor) or `CupFinalWin` at high stakes. It is the rarest family and represents the "veteran heartbeat" arc. A player with no `mentality ≥ 0.6` gene and no `ambition ≥ 0.5` gene has effectively zero readiness accumulation rate in this family — it's not a random benefit.

---

## Threshold and cadence seeds

| Parameter | Phase-3 seed | Notes |
|---|---|---|
| `breakthrough_threshold` | 0.92 | Supersedes archive doc's 0.85; ADR-0005 is authoritative. A meter at 1.0 is necessary but the gate may delay firing another 1-2 seasons. |
| `regressive_threshold` | 0.90 | Slightly lower than positive threshold — collapses should be rarer to initiate (harder accumulation path) but faster to tip once close. |
| `readiness_residue` (post-breakthrough reset) | 0.15 | Non-zero residue keeps the curve continuous. A player who just broke through starts the next arc at 15%, not zero. |
| `ca_lift_fraction` | 0.50 | PA lifts first; CA catches up halfway. A PA +8 breakthrough → CA +4 immediate. The player feels the change without instantly becoming their ceiling. |
| `breakthrough_cooldown` | 12 in-game months | Per (player, family). A player who breaks through in Finishing cannot fire another Finishing breakthrough for 12 months. Cross-family cooldowns are independent. |
| `regressive_cooldown` | 18 in-game months | Longer than positive — collapses are rarer and heavier. Pillar 3 intent: regressive moments feel massive, not routine. |
| `career_floor_formula` | `max(20, current_ca − 30)` | A collapse redraws PA downward but never below this floor in the affected family. Prevents a single catastrophic arc from writing a player out of the game. |

All values are Q32 fixed-point in canonical state. The `0.92` breakthrough threshold has Q32.32 raw bits `3_951_369_912` (`0.92 × 2^32 = 395_136_991_232 / 100 = 3_951_369_912.32`, rounded to nearest). The `0.90` regressive threshold has raw bits `3_865_470_566` (`0.90 × 2^32 = 3_865_470_566.4`, rounded). These exact raw values are pinned by literal-assert tests in `crates/fw-memory/src/breakthrough.rs` so a hand-typed-constant slip fails loudly.

---

## `family_relevance` table

Per-`(EventClass, attribute_family)` weights. These drive `readiness_delta = salience × family_relevance`. Zero cells are omitted — most `(class, family)` pairs are 0.

Phase-3 seeds. The `systems-designer` owns re-tuning; re-fit block under a dated header when values change.

| EventClass | Finishing | Passing | DefAnt | Aerial | Composure | Pace | Stamina | WorkRate | DeadBall | Leadership |
|---|---|---|---|---|---|---|---|---|---|---|
| `BreakthroughMoment` | — | — | — | — | — | — | — | — | — | — |
| `SignatureFirstFired` | 0.15 | 0.10 | 0.05 | 0.05 | 0.10 | 0.05 | — | 0.05 | 0.10 | — |
| `LegacyGoal` | **0.45** | 0.05 | — | 0.08 | **0.25** | 0.05 | — | 0.05 | 0.10 | 0.05 |
| `HatTrickScored` | **0.40** | — | — | 0.05 | 0.15 | 0.05 | — | 0.10 | — | — |
| `BigMatchScar` | −0.10 | — | — | — | **−0.30** | — | — | −0.10 | — | −0.10 |
| `RegressiveCollapse` | — | — | — | — | — | — | — | — | — | — |
| `PromisedYouthMinutes` | — | — | — | — | — | — | — | — | — | — |
| `BrokenPromise` | — | — | — | — | −0.15 | — | — | **−0.25** | — | −0.10 |
| `ContractRenewalRejected` | — | — | — | — | −0.05 | — | — | −0.15 | — | — |
| `ContractRenewalAccepted` | — | — | — | — | 0.08 | — | — | 0.08 | — | 0.05 |
| `TransferRequested` | — | — | — | — | — | — | — | −0.05 | — | — |
| `TransferRefused` | — | — | — | — | −0.05 | — | — | — | — | — |
| `SoldUnderProtest` | — | — | — | — | −0.10 | — | — | −0.10 | — | — |
| `BoughtOnDeadlineDay` | — | — | — | — | — | — | 0.05 | — | — | — |
| `RivalryFormed` | 0.05 | — | 0.05 | — | 0.05 | — | — | 0.05 | — | 0.05 |
| `MentorTeammate` | — | 0.12 | 0.10 | — | **0.20** | — | — | 0.08 | — | **0.35** |
| `DerbyControversy` | — | — | — | — | −0.12 | — | — | — | — | −0.08 |
| `FormerClubReunion` | 0.05 | — | — | — | 0.08 | — | — | 0.05 | — | — |
| `CupFinalWin` | 0.15 | 0.10 | 0.10 | 0.08 | **0.30** | — | 0.08 | 0.10 | 0.08 | **0.25** |
| `CupFinalLoss` | — | — | — | — | **−0.25** | — | — | — | — | −0.10 |
| `PromotionWon` | 0.08 | 0.05 | 0.05 | — | 0.15 | — | 0.05 | 0.08 | — | 0.12 |
| `RelegationSuffered` | −0.05 | — | −0.05 | — | **−0.20** | — | — | −0.10 | — | −0.15 |
| `TitleWon` | 0.10 | 0.08 | 0.08 | — | 0.20 | — | 0.08 | 0.10 | — | 0.18 |
| `UnbeatenRunEnded` | — | — | — | — | −0.08 | — | — | — | — | −0.05 |
| `DebutSenior` | 0.05 | 0.05 | 0.05 | — | 0.12 | 0.05 | — | 0.05 | — | — |
| `DebutClub` | 0.03 | 0.03 | 0.03 | — | 0.08 | — | — | 0.03 | — | — |
| `Retirement` | — | — | — | — | — | — | — | — | — | — |
| `InjuryLongTerm` | −0.05 | — | — | — | −0.08 | −0.15 | **−0.30** | −0.08 | — | — |
| `InternationalCallUp` | 0.05 | 0.05 | 0.05 | 0.03 | 0.10 | 0.03 | — | 0.05 | 0.03 | 0.08 |
| `Compaction` | — | — | — | — | — | — | — | — | — | — |

**Design notes on the table:**

- `BreakthroughMoment` and `RegressiveCollapse` carry zero weight in the relevance table — they ARE the result of the mechanism, not an input to it. Self-reinforcing loops are not the intent.
- `Compaction` carries zero weight — it is a system event, not a football event.
- Negative values tick `regressive_pressure` upward (they appear as weights in the regressive_pressure accumulation formula, mirroring the positive formula). Negative weights in `family_relevance` **do not** tick `signature_readiness` downward — `signature_readiness` can only be reset by a breakthrough fire (to the residue) or decay (handled separately). The negative cells in this table are read **only** for `regressive_pressure` accumulation.
- `Composure` receives the widest spread of events because composure is the most cross-cutting attribute — it is affected by wins, losses, promises, injuries, and mentorship. This is by design; a broken player's composure arc is the central regressive-collapse story.
- `Leadership` receives weight only from relational and high-stakes events — no accumulation from routine career events. This deliberately makes Leadership breakthroughs rare (typically one per career per player who has them at all).

---

## Gating-event table

The second condition of the three-part narrative gate: which `EventClass` can serve as the triggering gating event for a breakthrough in each family. All gate events must have `stakes >= 0.5` (medium-stakes or above) to qualify.

| Family | Valid gating event classes | Notes |
|---|---|---|
| `Finishing` | `LegacyGoal`, `HatTrickScored` | A decisive goal or hat-trick is the only honest gate. A routine goal is not sufficient — it must have crossed the `LegacyGoal` salience threshold. |
| `Passing` | `LegacyGoal` (as assist-participant), `CupFinalWin`, `PromotionWon`, `TitleWon` | The playmaker whose pass wins the cup final. Not a gating event for an individual pass — it must be a match-scale moment. |
| `DefensiveAnticipation` | `CupFinalWin`, `PromotionWon`, `TitleWon`, `RelegationSuffered` (survived — i.e. player's club avoids relegation on final day), `UnbeatenRunEnded` (opponent) | A defensive breakthrough gates on structural achievement, not an individual clearance — the ledger records the outcome, not the tackle. |
| `AerialPresence` | `LegacyGoal` (headed), `CupFinalWin`, `PromotionWon` | Aerial breakthroughs gate on headed legacy goals specifically (`consequence` field carries shot-type metadata). |
| `Composure` | `CupFinalWin`, `CupFinalLoss`, `PromotionWon`, `RelegationSuffered`, `BigMatchScar` (recovery — a subsequent high-stakes positive result after a `BigMatchScar`) | The composure gate is the widest because composure is a cross-cutting attribute. Positive and negative composure gates both exist. |
| `Pace` | `InternationalCallUp`, `DebutSenior` (high-stakes), `SignatureFirstFired` (pace-family signature) | Pace breakthroughs are rarest — they require a recognition event that specifically validates speed, not just any high-stakes result. |
| `Stamina` | `InjuryLongTerm` (return — player returns from long-term injury and plays a full match), `PromotionWon`, `TitleWon` | Stamina breakthroughs are typically recovery arcs: surviving an injury and returning stronger. |
| `WorkRate` | `CupFinalWin`, `PromotionWon`, `TitleWon`, `MentorTeammate` (as the mentored player) | WorkRate is shaped by collective achievement and mentorship, not individual moments. |
| `DeadBallDelivery` | `LegacyGoal` (free kick or penalty), `CupFinalWin` (via set-piece), `HatTrickScored` (if one was a penalty or free kick) | Dead-ball gates require the gating goal to have been a set-piece variant — carried in the `consequence` metadata. |
| `Leadership` | `MentorTeammate` (as the mentor, player must be >= 28 years old), `CupFinalWin` (player as designated captain), `TitleWon` (player as designated captain) | Leadership gates are the most specific. The player must be explicitly in the mentor or captain role in the event's `participants`. |

**Regressive gating events (by family):**

| Family | Regressive gating event classes |
|---|---|
| `Composure` | `BigMatchScar` (high-stakes), `CupFinalLoss`, `RelegationSuffered` |
| `WorkRate` | `BrokenPromise` (victim), `SoldUnderProtest` |
| `Finishing` | `BigMatchScar` (in a scoring role, missed-penalty flavor) |
| `Pace` | `InjuryLongTerm` (speed-injury class — hamstring, Achilles) |
| `Stamina` | `InjuryLongTerm` (any class, recurrence) |
| `Leadership` | `DerbyControversy`, `CupFinalLoss` (as captain) |

---

## `redraw_distribution` — PA redraw magnitudes

On breakthrough fire, the career system samples `delta_pa` from a bounded distribution seeded via `ChaCha8Rng` with `SeedLayer::SignatureTrigger`. Phase-3 seeds below. All values in CA/PA integer units (the PA scale is 1–200, aligned with research-doc note on FM's CA/PA; raw Q32 arithmetic applies internally).

| Family | Positive redraw range (PA uplift) | Regressive redraw range (PA decline) | Stakes modifier |
|---|---|---|---|
| `Finishing` | +4 to +9 | −4 to −8 | ×1.3 at `stakes >= 0.85` |
| `Passing` | +3 to +7 | −3 to −6 | ×1.2 at `stakes >= 0.85` |
| `DefensiveAnticipation` | +4 to +8 | −4 to −7 | ×1.2 at `stakes >= 0.85` |
| `AerialPresence` | +4 to +9 | −3 to −6 | ×1.1 at `stakes >= 0.80` |
| `Composure` | +3 to +8 | −5 to −10 | ×1.4 at `stakes >= 0.85` |
| `Pace` | +5 to +11 | −6 to −12 | ×1.3 at `stakes >= 0.90` |
| `Stamina` | +3 to +7 | −4 to −9 | ×1.2 at `stakes >= 0.80` |
| `WorkRate` | +3 to +6 | −3 to −7 | ×1.1 at `stakes >= 0.80` |
| `DeadBallDelivery` | +4 to +8 | −3 to −6 | ×1.1 at `stakes >= 0.80` |
| `Leadership` | +3 to +6 | −3 to −7 | ×1.5 at `stakes >= 0.90` |

**Design rationale for Composure and Pace magnitudes:** Composure regressive redraws are the largest negative range (−5 to −10) because a confidence-broken player's story is the most legible negative arc in football narrative. Pace has the largest positive range (+5 to +11) because physical breakthrough stories ("he found another gear") are the most dramatic positive arc — but also the rarest gate, keeping the high magnitude in check.

**Stakes modifier:** when `event.stakes >= threshold`, the drawn delta is multiplied by the stakes modifier and re-clamped to the range ceiling. A cup-final Finishing breakthrough at stakes 0.91 draws from effectively `+5.2 to +11.7`, then clamps to `+9`. This means high-stakes breakthroughs are biased toward the upper end of the range, not outside it.

**`ca_lift_fraction`:** 0.50 applies universally. A Pace PA +8 breakthrough immediately yields CA +4. The remaining +4 of CA headroom fills via normal career development over subsequent seasons.

---

## Cadence math (falsifiable target: 1-3 breakthroughs per player per 5-season career)

This is the T3-4 acceptance-target derivation. The implementation must produce output in this range; if it doesn't, the tuning seeds above are the dials.

**Assumptions (Phase-3 seeds):**

- A typical player sees 4-8 salient events per season across all 10 families (per `docs/design/memory.md` §"Event cadence" — the 5-8-events-per-season salience ceiling). Call it 6/season.
- Of those 6 events, a given family sees on average 1.5 relevant events per season (events spread roughly across families, with Composure and WorkRate higher, Pace and Leadership lower).
- Median `family_relevance` weight for a relevant event: 0.15 (reading the table above — most non-zero cells are in the 0.05–0.25 range).
- Median event salience: 0.40 (routine-significant event; see ADR-0005 salience formula; full salience ceiling is 1.0, reserved for cup finals and breakthroughs themselves).
- Per-event readiness delta: `0.40 × 0.15 = 0.06`.
- Starting from the post-breakthrough residue (0.15), reaching 0.92 requires `(0.92 − 0.15) / 0.06 ≈ 12.8` relevant events.
- At 1.5 relevant events per season per family: `12.8 / 1.5 ≈ 8.5 seasons` to fill a single family meter from residue.

**But:** the player has multiple families accumulating in parallel, and the gate requires a specific gating event after the threshold is crossed. The gate adds ~1.5 seasons of expected wait (probability of the right gating event per season ≈ 0.5 for common families like Composure; lower for Leadership). Total expected time from residue to fired breakthrough: ~10 seasons for a single family.

**For the career target (1-3 per 5 seasons):** a player with 2 `signature_candidates` accumulating in two families in parallel fires 2 × (5 / 10) = 1.0 breakthroughs in 5 seasons on average — right at the lower bound. A top-flight starter playing consistently in a well-matched role accumulates 2-3× the median event cadence, cutting the fill time to ~4-5 seasons — yielding 2 families × 1 breakthrough per 4.5 seasons ≈ 2-3 total. Depth players and journeymen with 0 candidates fire rarely or never.

This is within the 1-3 target range. The primary tuning dials for cadence are:

1. `family_relevance` weights in the table above — increase to accelerate, decrease to slow.
2. `breakthrough_threshold` (0.92) — lower to accelerate, raise to slow.
3. `readiness_residue` (0.15) — lower to slow the second arc, raise to speed it.
4. Gating-event gate pass rate — narrow the valid gating-event list to slow, widen to accelerate.

**Re-fit discipline:** after the T3-4 balance harness runs a 1000-career simulation, this section gets a dated re-fit block under the heading "**YYYY-MM-DD T3 balance-harness re-fit**" with updated seeds and the empirical cadence distribution from the run. Do not delete the Phase-3 seeds above — audit trail.

---

## Cross-references

- `docs/adr/0005-memory-ledger-and-breakthroughs.md` — authoritative mechanism schema; `family_relevance` formula; PA redraw formula.
- `design/breakthrough-moments.md` — mechanism design (this doc is the tuning layer on top).
- `design/player-generation.md` §D — narrative trigger flags; gene-model anchors for family groupings.
- `crates/fw-memory/src/event.rs` — `EventClass` enum (30 variants, shipped at T3-1).
- `crates/fw-core/src/q32.rs` — Q32 arithmetic (all values above are Q32 in canonical state).
- `docs/design/memory.md` — salience weights, event cadence ceiling (the 5-8-events-per-season figure referenced in cadence math); authored alongside the full 5-term salience blend (deferred per the T3-2 decision).
- `docs/research/sports-sims/07-player-attributes-progression.md` lines 71-72 — explicit XP-pattern rejection.

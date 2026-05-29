# Career-Roster Layer — Design Blueprint

> **Status: PROPOSED** (pending resequence ratification — see §0). Authored 2026-05-29
> as the response to the mid-T4 fresh-eyes review (`docs/audits/mid-t4-fresh-eyes-review-2026-05-29.md`),
> which found that 4 of 5 pillars produce zero player-visible output in a real career
> because no per-club player roster with mutable per-player state exists. Blueprint
> produced by `feature-dev:code-architect` against the live code; grounded in cited
> files. This is the design home for the work; per-sub-row task specs live in MEMORY.md
> at implementation time.

---

## §0. The decision this blueprint asks for

This blueprint proposes **resequencing T4**: build the career-roster layer (sub-rows T4-2.5a..h below) *before* the remaining discretionary polish (T4-7 game-shell, T4-9 stretch viewer). Rationale: the layer is the single unscheduled foundation that gates pillars 3 & 4 plus the already-deferred T4-2b and T4-5b, and the T4 exit gate ("a stranger understands player identity") cannot be met without it.

Two engineering calls are baked into the plan and should be logged on ratification:
1. **Match-engine policy (the key open question):** the player's own club's matches run the real 22-player `play_match` tick engine; the other 360 fixtures stay on seeded-procgen scorelines. This keeps a 10-season career at ~20s (under the T5-5 <60s target) while making the player's own stats/breakthroughs/signatures genuine. (See §6.)
2. **First-increment generation:** populate the 20 clubs by assigning the existing 22 `PlayerTemplate`/`PlayerBio` pool across them (the full procedural ~2000-player compiler stays deferred). (See §3.)

**One open product-vision question this does NOT decide:** whether the EA ship target remains the 6-tier ~96-club LLM-baked pyramid promised in `DESIGN_DOC.md §MVP-scope`, or narrows to a hand-authored single-league slice. This blueprint is compatible with either — it builds the slice now and the `BTreeMap<ClubId, Vec<PlayerInstance>>` shape scales to 96 clubs later. That reconciliation is a separate `/log-decision` (ideally with the GPT-5.5 design partner).

---

## §1. Current reality

**The gap in one paragraph.** `TeamTemplate` (`crates/fw-content/src/team.rs:12`) holds exactly `id`, `qualified_id`, `display_name`. `CareerState` (`crates/fw-tauri/src/state.rs:53-62`) holds `season`, `ledger`, `season_number` — no players. `get_squad_inner` returns all 22 hand-authored `PlayerBio` records as one unpartitioned pool with no `club_id`. The season advances via `play_one_match` (`crates/fw-tauri/src/season.rs:108`) → `MatchState::initial_with_content` + a tick loop — but the only surviving output is `MatchOutcome { home_score, away_score }`. No player participates in more than one match, no stats accumulate, and `emit_season_end_events` (`season.rs:47`) emits one club-only `TitleWon` and returns.

**What already works.** `generate_league` (`fw-content/src/league.rs`) builds a deterministic 20-club structure. `generate_team` (`procgen.rs:143`) already generates 22 `PlayerName` records + a manager per club from a per-club seed — but discards them after attaching names to `TeamTemplate`. `PlayerTemplate` (`player.rs:52`) is a full 55-attribute model with `AbilityCeiling`, `preferred_role`, `signature_candidates`. `BreakthroughState` + `evaluate()` (`fw-memory/src/breakthrough.rs:526,1013`) are complete and correct. The `AttributeFamily` 10-bucket enum is at `breakthrough.rs:57`. `observe_player` (`fw-scouting/src/observe.rs:33`) is correct and pure. `SaveV3` (`fw-save/src/lib.rs:176`) already carries `breakthrough_states: BTreeMap<PlayerId, BreakthroughState>` — always empty today. No production save/load IPC command exists.

**The missing bridge.** `BreakthroughContext` (`breakthrough.rs:645`) needs `pa_by_family` + `ca_by_family` as `BTreeMap<AttributeFamily, i16>` on the 1..=200 scale. No mapping from the 55-field `PlayerAttributes` (Q32 [0,1]) to `AttributeFamily` buckets exists anywhere.

---

## §2. Data model + new types

The roster is career-level mutable state → it belongs on `CareerState` in `fw-tauri`. No new crate is warranted (the roster is a projection of content + ledger, not a new sim domain). The read-only identity layer (`PlayerBio`/`PlayerTemplate`/`GeneSnapshot`) stays in `fw-content` and is referenced by `PlayerId`, not duplicated.

**`PlayerInstance`** (new `crates/fw-tauri/src/roster.rs`):

```
PlayerInstance {
    player_id: PlayerId,                           // stable handle, never reassigned
    club_id: ClubId,                               // current affiliation
    attributes: PlayerAttributes,                  // Q32 [0,1], mutable via breakthroughs
    ceiling: AbilityCeiling,                        // CA + PA, breakthrough-gated mutation
    signature_candidates: Vec<SignatureCandidate>,
    breakthrough_state: BreakthroughState,
    season_stats: PlayerSeasonStats,
    career_apps: u32,
    observation_count: u32,                         // pillar-4 scout draw counter
    last_scout_report: Option<ScoutReport>,         // cached per-player
}
```

**`PlayerSeasonStats`** (same file): `appearances: u16`, `goals: u16`, `assists: u16`, `minutes_played: u32`, `average_rating` kept as Q32 numerator + `rating_sample_count: u16` (divide at DTO time — no float in canonical).

**`CareerState`** gains one field: `roster: BTreeMap<ClubId, Vec<PlayerInstance>>` (outer key deterministic by `ClubId`; inner `Vec` ordered by slot, GK=0).

**DTOs** (`crates/fw-tauri/src/roster_dto.rs`, all `f64` at the boundary per `Tauri/RULES.md §3`): `PlayerRosterDto`, `ScoutReportDto`, `CategoryEstimateDto`, `LabelEstimateDto`.

---

## §3. Generation strategy — Recommendation A (template-assign)

**Option A (recommended for the first increment):** at career start, for each of 20 clubs, draw 22 `PlayerInstance` records from the existing `PlayerTemplate` pool, assign distinct `PlayerId`s derived from `(career_seed, club_idx, slot)`, attach the club affiliation and the name already produced by `generate_team`. Cross-club diversity comes from per-club seed variation in which template lands in which slot. ~50 lines; sufficient to make all four unwired pillars visible.

**Option B (defer):** the runtime player compiler `(career_seed, club_id, role) → PlayerInstance` via the gene model is the T2-4-deferred work; it needs a gene→attribute mapping that doesn't exist yet and is not required for pillar wiring.

**Feeding `generate_team` output:** modify `generate_league` to return its per-club `ProcGenTeam` (name + manager + 22 names) instead of discarding it, so names aren't recomputed.

**Scale:** 440 instances ≈ 220 KB; 96 clubs × 22 ≈ 1 MB. The map shape scales to the pyramid without structural change.

---

## §4. AttributeFamily bucketing bridge (the critical missing piece)

New `crates/fw-core/src/attribute_family_bridge.rs` (in `fw-core` because both `fw-memory` and `fw-tauri` need it).

- A `const` table mapping each of the 10 `AttributeFamily` buckets to the visible attributes that contribute (the 17 hidden personality/durability attributes do not contribute to PA/CA per ADR-0002). The exact attribution weights are a `systems-designer` tuning concern; the structure is fixed.
- `attrs_to_family_pa_ca(attrs, ceiling, role_weights) -> (BTreeMap<AttributeFamily,i16>, BTreeMap<AttributeFamily,i16>)` — projects the Q32 [0,1] family means onto the 1..=200 scale (`value_200 = (q32_mean * 200).to_int_clamped()`), PA from `ceiling.potential()`, CA from `ceiling.current()`.
- `apply_family_delta_to_ceiling(...)` — the inverse: takes a `BreakthroughOutcome { delta_pa, delta_ca, family }` and bumps the canonical `AbilityCeiling` via the existing `redraw_ceiling` API.

---

## §5. Pillar wiring data flows

- **Pillar 3 (breakthrough):** `advance_season_inner` write-locks `career`; for every rostered player builds a `BreakthroughContext` via the §4 bridge (+ a by-name `NarrativeFlag` conversion between the duplicate `fw_content`/`fw_memory` enums), calls `evaluate()`, applies deltas, appends `outcome.event` to the ledger, writes `breakthrough_state` into the save slot. Integration test: 5 seasons on seed `0xfeedbeefcafefade` asserts ≥1 `BreakthroughMoment` — the end-to-end proof the T3 gate lacked.
- **Pillar 4 (scouting):** per match-day, `observe_player` runs for the player's club fixtures; result cached on `PlayerInstance::last_scout_report`. New `get_scout_report(player_id)` IPC command (registered in `src-tauri/main.rs`) projects the Q32 bands → `ScoutReportDto`.
- **Pillar 5 (signatures):** `play_one_match` gains a `slot_signatures: BTreeMap<PlayerSlot, Vec<SignatureCandidate>>` argument built from both clubs' rosters, so all 22 players' candidates enter the sim (not just slot 7).
- **Pillar 2 (memory):** new player-subject emissions (`PlayerDebut`, `LegacyGoal` — both already in `EventClass`; `BreakthroughMoment` flows from pillar 3) in `update_player_stats_from_match`, so `/player` career-moments is non-empty. The blank-`player_name` career-overview render is fixed by routing to the player-name-free `title_won` variant.
- **Stats (T4-2b prerequisite):** `update_player_stats_from_match` accumulates apps/goals/minutes + a rolling Q32 rating (coarse first increment; full per-match stats need the match-event stream, a later sub-row).

---

## §6. Key open question — real engine vs seeded-procgen (RESOLVED: Option 1)

| Option | Per-season sim | 10-season career | T5-5 (<60s)? |
|---|---|---|---|
| **1 (recommended)** — player's club real engine, others seeded | 38 real matches × ~50ms ≈ 2s | ~20s | PASS |
| 2 — all 380 matches real | 380 × ~50ms ≈ 19s | ~190s | FAIL (3×) |

**Recommendation: Option 1.** Matches the FM model (player's club "in full", AI fixtures quick-sim), powers genuine per-player stats/signatures/breakthroughs for the player's own squad, and scales linearly as the league grows (player's club = constant). Log via `/log-decision` with the first roster sub-row.

---

## §7. Save schema — SaveV4

`SaveV4` adds `roster: BTreeMap<ClubId, Vec<SavedPlayerInstance>>` (a save-stable subset of `PlayerInstance` — mutable state only; `PlayerBio` is reloaded from content at load). `SaveV3::breakthrough_states` is absorbed into per-instance `breakthrough_state`; the V3→V4 migration reconstructs instances from the content store + the V3 map. The 4 required tests per `design/specs/save-migration-fixtures.md`: forward-migration, callback-preservation, forward-incompat-failure, round-trip-byte-identical. Two new IPC commands close the "no production load path" gap: `save_career()` and `load_career()`.

---

## §8. Build sequence (proposed sub-rows)

| ID | Title | Crates | Deps | Done-criteria (falsifiable) | Agent | Hash drift |
|---|---|---|---|---|---|---|
| T4-2.5a | `AttributeFamily` bridge + Q32→1..=200 scale | fw-core | — | `attrs_to_family_pa_ca` + `apply_family_delta_to_ceiling`; unit tests on baselines; proptest output ∈ 1..=200 | gameplay-programmer | No |
| T4-2.5b | Roster data model + career-start generation | fw-tauri (`roster.rs`,`roster_dto.rs`,`state.rs`), fw-content (`league.rs`) | a | `CareerState::roster` populated 20×22=440 distinct `PlayerId`s; `get_roster_for_club` IPC; `generate_league` returns per-club `ProcGenTeam` | lead-programmer | No |
| T4-2.5c | Signature candidates onto all 22 slots | fw-tauri (`season.rs`), fw-match-sim (`lib.rs`) | b | `play_one_match` takes `slot_signatures`; `SignatureFirstFired` fires for non-slot-7 in a smoke seed | gameplay-programmer | **Yes — authorized rebaseline** |
| T4-2.5d | Pillar-3: `evaluate()` into `advance_season_inner` | fw-tauri (`commands.rs`), fw-core (bridge) | a,b | breakthroughs apply deltas + append to ledger; 5-season integration test asserts ≥1 `BreakthroughMoment` | gameplay-programmer | No |
| T4-2.5e | Pillar-2: player-subject events (debut/goal/breakthrough) | fw-tauri (`roster.rs`,`season.rs`) | b | `PlayerDebut`+`LegacyGoal` emitted; `get_player_detail` non-empty callbacks; blank-name render fixed | gameplay-programmer | No |
| T4-2.5f | Pillar-4: scouting wiring + `get_scout_report` IPC | fw-tauri (`commands.rs`), fw-scouting | b | `observe_player` per-match; report cached; command registered; report differs at 5 vs 0 observations | lead-programmer | No |
| T4-2.5g | SaveV4 schema + migration + save/load IPC | fw-save, fw-tauri | b,d | `SaveV4` + `migrate_v3_to_v4`; 4 migration tests; `save_career`/`load_career`; round-trip roster count stable | lead-programmer | No |
| T4-2.5h | Per-player match stats + Squad stats UI | fw-tauri, frontend (`Squad.tsx`) | c,e | stats updated post-match; Squad renders apps/goals/minutes; `pnpm test` green + screenshot | ui-programmer | No |

Ordering: `a` unblocks all; `b` creates the roster; `c` is independent of d/e/f but must precede the hash rebaseline; `d`/`e`/`f` parallel after `b`; `g` needs `b`+`d`; `h` is the user-visible payoff.

---

## §9. Out of scope / later phase (deliberately)

1. Full T2-4 runtime player compiler (gene→attribute derivation).
2. 6-tier ~96-club pyramid + ~2000-player corpus + LLM bake pipeline (7/8 baker subcommands are stubs).
3. Transfers / contracts / inter-club movement (the `club_id` field is the hook).
4. Multi-scout disagreement + accumulating track records (this increment is single-scout).
5. Training / conditioning decay / aging curves.
6. Youth academy (DESIGN_DOC §8 defers to Phase 3+).
7. Compaction-retention corpus test (follow-up after T4-2.5e, once player-subject events flow).
8. Per-signature commentary routing — cheap + high-leverage but orthogonal (`narrative-director`+`ui-programmer`); can run in parallel with a-c.

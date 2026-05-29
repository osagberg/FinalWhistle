# Mid-T4 Fresh-Eyes Review — 2026-05-29

> Read-only two-lens review run by a newer model (Opus 4.8) mid-T4, at the user's
> request, before continuing UI polish. **No code was changed by this review.**
>
> Method: a 29-agent workflow — 6 strategic pillar-delivery reviewers + 11 per-crate
> correctness reviewers, each correctness reviewer's findings adversarially verified
> by an independent skeptic and cross-checked against the existing audit docs
> (`post-t1-*`, `post-t2-*`, `post-t3-*`), then synthesized. Raw per-agent findings
> live in the workflow transcript; this doc is the curated synthesis.
>
> Bottom line up front: **no P0/P1 correctness findings** (the determinism gates,
> proptest sweeps, and three prior Codex phase-gates have the correctness surface
> well-covered). The material signal is strategic: 4 of 5 pillars produce zero
> player-visible output in a real career, all converging on one unscheduled
> foundation — the "career-roster layer."

---

## 1. Trajectory verdict

**Final Whistle's engine quality is high but its product is not on track: 4 of 5 pillars produce zero player-visible output in a real career, and all four converge on one unbuilt foundation — the "career-roster layer" — that has no MASTER_PLAN row, no owner, and no gate.** What runs end-to-end today is plumbing (deterministic match sim, ledger/reader machinery, breakthrough/scout engines, a Tauri IPC surface) plus a UI being polished over it. But starting a career generates one 20-club single-tier league of name-only shells, serves the same 22 hand-authored player bios on every seed, fires at most one signature for one player per match, and emits exactly one event class (`TitleWon`, club-only) across a 5-season career — so no breakthrough ever fires, no scout ever disagrees, and the `/player` "career moments" panel is provably empty for every player. The "infrastructure mistaken for feature-complete" pattern the T3 review flagged for pillars 3 & 4 is now the operating mode for the entire player/club data layer, and three consecutive roadmap rows (T2-4, T2-7, T4-2) were each hollowed out at their `/next` gate against this same missing layer. The honest framing: the codebase is a well-built deterministic skeleton whose differentiation moat is unwired, and continuing discretionary UI polish (T4-7, T4-9) ahead of the foundation extends polish-over-vapor.

## 2. Pillar status table

| Pillar | Status | One-line gap |
|---|---|---|
| 1 — Procedural fantasy world | infrastructure-only | Every save shows the identical 22 static bios; world is 1 league of 20 name-only club shells, not a nation/6-tier/~96-club pyramid; LLM bake pipeline is 7-of-8 stubs with no API client. |
| 2 — Careers that remember | infrastructure-only | Only `TitleWon` (club, no player) is ever emitted; `/player` callbacks always empty; the one live callback surface renders blank-`player_name` title prose. |
| 3 — Breakthrough-driven development | infrastructure-only | `evaluate()` has zero production callers; `advance_season_inner` never accumulates or evaluates; no per-player mutable roster to mutate. |
| 4 — Scouting uncertainty | infrastructure-only | `observe_player` has zero gameplay callers; no scouting IPC/DTO/UI/save-state; "Scout traits" is a static label list with no band, no disagreement, no over-seasons emergence. |
| 5 — Signature identity | at-risk | Mechanism is wired end-to-end but only slot 7 ever gets candidates, all signatures share one generic line addressed by a slot number, nothing on the board; catalogue frozen at 3 of 24 since T1. |
| (Cross-cutting) roadmap sequencing | at-risk | The career-roster layer gates pillars 3, 4, T4-2b, T4-5b but is parked as unscheduled "T4+" prose. |

## 3. The #1 systemic risk + sequencing recommendation

**The single point of failure is the unscheduled "career-roster layer."** It does not exist in code: `TeamTemplate` (team.rs:12) holds only id/qualified_id/display_name with no roster; `get_squad_inner` returns all 22 bios as one undifferentiated pool with no `club_id`; `CareerState` (state.rs:53-62) carries season + ledger + season_number and zero players. Four load-bearing deferrals (T2-4 player compiler, T4-2b per-player stats, T4-5b live-mode, and the pillar-3/4 wiring) all name this same blocker, yet there is no `T4-x` row that builds it. A foundation that four features transitively depend on, that nobody has scheduled, is the textbook accreting-deferral risk — it will surface as a multi-row epic discovered mid-T4 or at T5, exactly when polish is supposed to be done.

**Is T4 ordering right? Partly.** The polish rows already shipped (T4-1 PixiJS board, T4-3 theming, T4-4 error states, T4-6a settings) were genuinely independent of the missing layer — doing them first was defensible. But the conclusion "so the order is fine" does not follow: the remaining T4 work (T4-2b, T4-5b, pillars 3 & 4) all converge on the unbuilt foundation, and **T4's own exit gate** ("a stranger watching live-mode understands drama + player identity") **cannot be met without it** — live-mode is deferred and player identity isn't connected to the season. Worse, the league sim driving all T4 UI is decoupled from both the player pool and the attribute match engine: standings come from a per-fixture seeded procgen scoreline (`season::play_one_match(fixture_seed(...))`), never the 22-player `play_match` tick engine — so `/stats` is charting procgen noise and T4-2b per-player stats can't be derived from the league at all.

**Recommendation — resequence, do not stay the course:**
1. Author + build an explicit, ID'd MASTER_PLAN row for the **career-roster layer** now (e.g. T4-2.5): a persistent `BTreeMap<ClubId, Vec<PlayerInstance>>` where each instance owns mutable attributes + gene snapshot + signature_candidates + a per-player BreakthroughState, generated at career start. Give it a phase owner and done-criteria.
2. Wire **pillar 3** (`evaluate` into `advance_season_inner`) and **pillar 4** (`observe_player` into the career loop) on top of it.
3. THEN promote T4-2b + T4-5b (now unblocked).
4. **Defer T4-7 (game-shell) and T4-9 (stretch viewer)** until the foundation lands — every polish row built first is a row that may need rework once clubs are actually populated.

A cheaper interim hedge that does not require the full layer: a read-only `get_scout_report(playerId)` + `ScoutReportDto` over the existing 22-bio pool, and routing `SignatureFirstFired` commentary through per-signature banks (section 5 gaps). Both make a pillar visibly real for low cost.

Also: record via `/log-decision` whether the season is meant to stay seeded-procgen or eventually run the attribute match engine — T5-5's "10-season career in <60s" perf target has very different characteristics for 96 clubs × full matches vs seeded scorelines, and that ambiguity should not be discovered at the perf pass.

## 4. New correctness findings (not previously triaged)

No P0/P1. The real bugs are P2 panic-in-handler sites and a broken dev tool; the rest are P3 doc/honesty drift and latent overflow hygiene. Reachability flagged per item.

### P2 — act on these

| Crate · Location | Finding | Recommendation |
|---|---|---|
| fw-tauri · commands.rs:386-389 + state.rs:340-348 | `advance_week_inner` panics: BTreeMap `[&key]` index on `tactical_archetype_ids` + `league_fixture_index().expect()`. **Reachable (wired), but invariant-guarded — cannot fire for the generated league today; latent vs future content/mod.** Violates Tauri §4. | Map to `IpcError` via `.get(&k).ok_or(...)?` and return an error from the not-found case. |
| fw-tauri · commands.rs:571 | `get_fixtures_inner` opponent-name `.expect()`. Same class: **wired, invariant-guarded, latent.** | Return a structured `IpcError` instead of `.expect()`; a panic poisons the lock and cascades. |
| fw-content + baker · justfile:189 (via scripts/fw:130-138) | `just bake-content` invokes a non-existent `bake` subcommand (CLI has no `bake`/`--output`) — the documented content-regen path is broken, fails at clap parse. **Reachable by any operator running the documented command.** | Point at a real subcommand or print the deferral; add a smoke test asserting the invocation parses. |
| frontend · Transfers.tsx:69-82 → :132-133 | `describeStandingsError` falls through to render raw `err.message`, leaking `IpcShapeError` payload preview into player-facing copy. **Reachable on backend DTO drift (error path, not happy path).** Violates the no-raw-message contract every other route honors. | Replace with `describeRouteError(standings.error, { what: 'the transfer window' })`. |

### P3 — fix opportunistically (grouped)

**Latent overflow / fail-loud hygiene (Sim §11 spirit):**
- fw-core · tick.rs:127-133 — `to_q32_seconds` silently narrows i64→i32 before a debug-only guard (wraps in release). **Zero callers (unwired).** Use `i32::try_from(...).expect(...)`.
- fw-match-sim · separation.rs:96-130 — `resolve_pair` uses bare panic-on-overflow Q32 ops with no documented position bound; players never pitch-clamped. **Reachability very low — velocity is hard-clamped to ±8 m/s, so overflow needs ~347k ticks of a separate divergence bug; not on pinned seeds.** Add a one-line SAFETY comment naming the bound (lighter) or clamp positions in tick step 7 (heavier). *(Originally proposed P2; correctly downgraded to P3.)*
- fw-match-sim · lib.rs:974-991 — `compute_opponent_shape_broken` uses `wrapping_add` on raw Q32 bits. **Wired (feeds CounterAttack decision); no real wrap possible at pitch bounds.** Use `checked_add` or i128 accumulate. (The T2 saturating/wrapping sweep missed this site.)
- fw-match-sim · dispatch.rs:411-561 — a GK/non-bias slot can register a signature firing + emit `SignatureFirstFired` whose bias is never applied. **Unreachable today (all 3 triggers role-gate out the GK).** Gate the evaluate block before any GK-eligible signature ships.

**Doc/comment honesty (no behavior change):**
- fw-save · lib.rs:144 — SaveV2 doc still claims "CURRENT production schema; all new saves are V2," contradicting SaveV3 at :174. **test-only.** Reword to PRESERVED-FOREVER.
- fw-save · lib.rs:170,192-193,300 — claims "the loader regenerates a fresh season from the seed," but `load_envelope` does no regeneration; obligation is actually the caller's. **Unwired (no production load path).** Reword + capture the caller-must-handle-`None`-season note for T4 wiring.
- fw-core · seed.rs:141-143 + dispatch.rs:29-30 — Decision-layer site docstring describes a u32-unsafe `player_id<<16` formula plus a nonexistent `as_u32()` accessor (real is `raw()`); duplicated in two places. **Unwired (no real per-player Decision-layer caller).** Fix both copies.
- fw-core · q32.rs:146-151 — `from_f64_clamped` maps NaN→0 silently; comment names `from_num` but code calls `saturating_from_num`. **test-only.** Fix the comment; assert non-NaN if a non-bake caller is ever added.
- fw-match-sim · ball_physics.rs:113-138 — drag/friction calibration comments internally inconsistent (k=0.01 vs k=0.007; 33m vs 52.5m); shipped values match the 0.007 block. **Comments only.** Collapse into one coherent block.
- fw-match-sim · ball_physics.rs:271-273 — bounce comment understates that `vz` is already post-drag/post-Magnus. **Wired; behaviorally zero today (magnus=0).** Tighten the comment; becomes load-bearing when spin is wired.
- fw-match-sim · tactic_fsm.rs:347-355 — HighPress re-entry cooldown doc says "since prior HighPress entry"; code (and spec) measure since current defensive-state entry. **Wired.** Fix the comment (spec is already correct; not a contract violation). *(Originally proposed P2; correctly reframed to P3.)*
- fw-match-sim · lib.rs:343-345 — WIRING-ONLY note lists `CounterWindowClosed` among "T2-1b/c wire" emissions, but it (and `PressTimeoutExpired`) are never emitted. **Unwired.** Drop from the list.
- fw-match-sim · softmax.rs:67-72 — overflow comment asserts "utility in [0,1]" but biased utilities reach ~1.96 (only `utility_shoot` clamps); exp LUT saturation prevents any actual overflow. **Wired; no live consequence.** Correct the comment or clamp every biased return.
- fw-match-sim · dispatch.rs:402-404 — idempotency comment is false when an earlier slot mutates possession in the same tick. **Wired; deterministic.** Reword to state the order-dependent re-route is intentional.
- fw-content · player.rs:36-37 (+ role_affinity.rs:37-40) — doc claims `load_baked` is "a stub returning `Ok(Self::default())`"; it actually delegates to `load_sources`. **Wired (runs at career-init); doc-only.** Update to match runtime.rs:985-993.
- fw-content-baker · prompts.rs:11 — stale "T2-3 wires bake-names"; bake-names is wired but uses `OFFLINE_PROMPT_TEMPLATE`, leaving `NAMES_PROMPT` dead. Fix comment.
- fw-memory · ledger.rs:268-269 + Cargo.toml:88-91 — `compact()` comment claims plain `+5` panics on overflow, but release builds wrap (no `overflow-checks`). **Overflow unreachable (~65k seasons).** Adjust the comment or set `overflow-checks=true`.
- fw-memory · ledger.rs:292-295 — Compaction event hardcodes `career_date { year:0, day_of_year:1 }`; value is never read. **Wired (written to real saves); cosmetic.** Derive from `current_season` or comment it as intentional.
- fw-scouting · observe.rs:139-148 — confidence draw is half-open `[MIN, MAX)`, so `LABEL_CONFIDENCE_MAX` (0.95) is unreachable; design doc says closed. **Unwired.** Tighten doc prose or treat brackets as informal.
- frontend · Match.tsx:370 (rendered :589-594) — tactical-board frame-load error renders raw `err.message`. **Reachable but inside an opt-in dev panel.** Route through `describeRouteError`.
- frontend · Home.tsx:9-18,33 — Home lacks a tailored error banner; handshake failure falls back to the generic ErrorBoundary (NOT an infinite spinner — the original P2 mechanism was refuted). **Reachable; honest generic panel, not a hang.** Mirror the Squad/League catch-in-fetcher pattern; add `Home.test.tsx` (only route with no test). *(Downgraded P2→P3.)*
- frontend · Tactics.tsx:18,30 + Home.tsx:28,59 + Layout.tsx:47,121 — stale phase-reference copy presents closed T1/T2 phases as future work; Layout still says "T0 scaffold." **Visible self-contradiction.** Refresh tags / drop phase numbers.

**Dead code / unwired surfaces (track, don't necessarily fix):**
- fw-replay · lib.rs:1-72 — the entire corpus loader (`load_entry`/`ReplayCorpusEntry`/etc.) has **zero callers anywhere**; the canonical_hash test uses its own local struct, and `scripts/fw replay` does not exist. Doc comment is false on both halves. Either wire it into the canonical_hash tests (de-duplicating the schema) or delete it and rewrite the crate doc.
- fw-replay · determinism-gate.md:327,440 — references nonexistent `scripts/fw replay --regenerate-corpus`/`--compare-corpus`; §9 already documents the real `scripts/fw hash-pins` mechanism. Drop the §8 stale refs.
- fw-replay · lib.rs:18-48 vs determinism-gate.md:316-329 — loader struct schema drifted from the spec's documented fixture schema (missing `content_pack_version`/`tick_rate_hz`/`generated_by`). Reconcile one direction.
- fw-match-sim · subtree_library.rs:191-198 — the outfield `Pressing` FSM state + its press/track_back arm are dead in real gameplay (`evaluate_transitions` never reaches it). **Unwired scaffolding.** Add a one-line "currently-reachable states" note.
- fw-tauri · season.rs:47-58 — `emit_season_end_events` is a dead `pub fn` with no callers. Delete or `pub(crate)` + `#[cfg(test)]` caller.
- fw-tauri · commands.rs:1011-1093 — `step_live_match` doesn't reject stepping an already-finished match (sim keeps mutating score post-FullTime). **Wired.** Early-return a no-op or return `IpcError::MatchAlreadyFinished`.
- fw-tauri · commands.rs:662 vs state.rs:56-58 — `get_player_detail` takes a WRITE lock (top_n rebuilds a lazy index) while the `career()` doc shows `.read()`. **Wired; functionally fine.** Fix the doc.
- fw-core · seed.rs:60-70 — `derive_q32` is a pub reinterpret-bits affordance with zero callers. Keep+document or prune.
- fw-content + baker · archetype.rs:11-18, prompts.rs, schemas.rs, validators.rs:18 — `BehaviorArchetype` never constructed; baker prompt/schema consts unreferenced under module-wide `allow(dead_code)`. Tighten the allow toward per-item.
- frontend · runtime-validators.ts:531,540-541 — `isMatchSnapshot` doesn't validate `yellowCards` values or bound `minute` to u16. **test-only (no route consumes `get_match_snapshot`).** Tighten when live-match UI lands.

**Tuning gaps (route to systems-designer when scout is wired):**
- fw-scouting · observe.rs:120-125 + band.rs:85-90 — category band width is a constant 0.24; `base_observation_noise` only shifts the band center, never widens it, so `band()` carries no per-scout uncertainty signal (reads Confident across most of the center range). **Unwired.** Make band width a function of noise, or rely on per-label confidence for the pillar-4 signal.

## 5. Strategic gaps worth acting on (beyond section 3)

- **Pillar 1 — world scale & bake pipeline (high).** The 6-tier/~96-club pyramid is the pillar's headline and is 0% built with no row scheduling it; the LLM bake pipeline is vapor (7 of 8 subcommands `stub_unimplemented`, no API client, `content/baked/` empty). Decide and record: is the EA target still a 6-tier pyramid + LLM-baked corpus, or has it narrowed to a hand-authored 20-club slice? If narrowed, update DESIGN_DOC §MVP-scope so the contract stops over-promising; if not, both need scheduled phases.
- **Pillar 1 — composed-name banned-terms gap (medium).** `validate-structural` does not lint composed first×last names against banned-terms/licensed-data (the "Manchester exploit"); the semantic validator that would close it rolled past its T2-4 milestone unscheduled. Low blast-radius today (Markov over 2 curated banks) but becomes load-bearing the moment LLM bake or mod packs land. Re-anchor the semantic validator and make composed-name linting a gate before any non-hand-authored content ships.
- **Pillar 2 — compaction retention (medium).** MASTER_PLAN's own risk register names "compaction loses callback-eligibility = pillar betrayal" and calls for a 100-ledger/10-season retention corpus; that test can't meaningfully exist while the live ledger holds only `TitleWon`-per-season. Build it before callback surfaces go user-facing, once player-subject events flow.
- **Pillar 2 — career-overview blank-`player_name` render (medium, near-term fixable).** The one live callback surface builds its render context with `player_name: String::new()`, and 2 of 3 `title_won` grammar variants reference `#player_name#` — rendering dangling fragments like "...the title in Season N — was there for every game of it." Restrict the live render to the player-name-free variant (or author variants that don't reference it) and add a test asserting no orphaned `' — '`. Cheap, and it stops the one shipping surface from looking broken.
- **Pillar 5 — per-signature commentary (medium, cheap, high-leverage).** All signatures render the same generic bank by `MatchEvent` discriminant; `presentation.commentary_line_bank_id` exists but is never consumed for routing, and the line names a slot number, not a player. Route `SignatureFirstFired` through the per-signature bank and resolve the slot to a name. Small, content-side, and the single fix that most makes the pillar feel real.
- **Pillar 5 — catalogue & counterplay (medium).** Frozen at 3 of 24 since T1 with no growth row; ADR-0011's `not_yet_implemented` stub-catalogue is unused (zero flags); the cancellation/counterplay predicate layer is entirely absent; deferred dispatch-hardening tests T1-25/T1-26 are the only coverage that would catch the production dispatch path regressing. Add a catalogue-growth row (or re-scope the launch number in the ADR) and promote T1-25/T1-26.

## 6. Already-known / refuted (coverage confirmation)

**Confirmed-and-resolved (not re-flagged):**
- **T3-R-B zero-delta breakthrough panic** — fix landed and is genuine: `evaluate()` guards positive emission with `if delta_pa > 0` (breakthrough.rs:1070) and regressive with `if actual_delta_pa < 0` (:1128), `assert!` (not `debug_assert!`) per §11 at :837-844/:904-911, sub-floor-CA capped at :1135-1137, `never_panics` proptest present. `compute_ca_delta` integer `/2` is resolved-as-documented (dead constant removed). No action.

**Already triaged in prior audits (re-surfaced only to confirm status unchanged at mid-T4):**
- Pillars 3 & 4 infrastructure-complete-but-unwired — `post-t3-ultimate-review-2026-05-21.md` convergence #1 (lines 38/80/90).
- SaveV3 schema-complete but zero production save/load callers (`breakthrough_states` always empty) — Codex E5 / `post-t3-codex-gate-2026-05-21.md` + ultimate-review convergence #2. Schema half was the only half scoped for T3-R-E; wiring is genuine T4 work. *(Verifier downgraded the standalone item P2→P3.)*
- `project_salience` Linear/Exponential decay branches never execute in real play (only `TitleWon` with `tick:None`/`Never`) — ultimate-review Track A P2. Decay code correct; wiring gap.
- FanReader future-dated-event recency filter lacks an `elapsed<0` guard — ultimate-review Track C P3. test-only.
- News/memory-callback var-shadowing grammar-key collision silently drops colliding rules — ultimate-review P2; recommended reject/`log::warn!` fix still owed but latent (no shipped grammar collides). *(Downgraded to P3.)*
- CounterAttack tactic-state has no production exit timer — `post-t2-ultimate-review-2026-05-18.md` + spec `[DEFERRED-T3]`. By-design deferral.
- Press/Mark intents target a static formation slot, not the live carrier — `post-t1-ultimate-review-2026-05-16.md` flagged it as a *test-coverage* gap; the **new** contribution here is the doc-comment contradiction (role_states.rs:476-479 claims live targeting) and that the recommended test was never written. Kept P2.

**Refuted / corrected during verification (so you know the noise was filtered):**
- Home-route "infinite spinner on handshake failure" — **refuted.** SolidJS 1.9 re-throws errored resource reads into the wrapping ErrorBoundary, so it renders an honest generic error panel, not a hang. Reduced to a P3 consistency/missing-test gap.
- Scouting "band ALWAYS reads Confident for every player" — **overstated.** Boundary-clamped centers narrow to Settled; the substantive point (noise never widens the band) stands.
- seed.rs "the real Decision-layer caller in dispatch.rs uses slot index" — **inaccurate.** Production uses `UtilityTieBreak`; Decision is reserved/unwired. The unsafe formula is purely documentary (and duplicated in a second spot the finding missed).

## 7. Recommended next actions (ordered)

**Is T4-7 (game-shell polish) the right next `/next`? No — pull the career-roster layer forward.** T4-7 and the T4-9 stretch viewer are exactly the polish-over-vapor that the trajectory verdict warns against, and the T4 exit gate can't be met without the foundation regardless.

1. **Author + schedule the career-roster layer as an ID'd MASTER_PLAN row** (e.g. T4-2.5), sequenced before T4-2b/T4-5b/T4-7, with an owner and done-criteria. This is the highest-leverage action in the report — it unblocks 4 deferrals and 2 pillars. (Per CLAUDE.md §4, a deferral 4 rows point at must be a scheduled deliverable, not a footnote.)
2. **Wire pillar 3 + pillar 4 on top of it:** `evaluate()` and `observe_player()` into `advance_season_inner`; write outcomes to the ledger; add the attribute→`AttributeFamily` bucketing + Q32-[0,1]→1..=200 scale bridge that `BreakthroughContext` needs (doesn't exist yet). Add a played-career integration test asserting ≥1 `BreakthroughMoment` fires across a multi-season run — the end-to-end proof the T3 gate explicitly lacked.
3. **Cheap parallel wins that don't need the full layer** (good for a designer/UI hand while #1 is built): route `SignatureFirstFired` through per-signature commentary banks + resolve slot→name (pillar 5); ship a read-only `get_scout_report` + `ScoutReportDto` over the 22-bio pool (pillar 4 made visible); restrict the live career-overview render to the player-name-free `title_won` variant (pillar 2 stops looking broken).
4. **Fix the four P2s** (two panic-in-handler sites → `IpcError`; Transfers raw-message leak → `describeRouteError`; repair or deprecate `just bake-content`). All small.
5. **One-pass roadmap audit of all remaining TODO rows** (T4-7, T4-9, all of T5) against "what data does this assume exists?" — pre-flag the ones assuming the roster layer or player-match-stats (T5-5 perf, T5-3 Deck UI sweep, T4-9 viewer reading frames the season never generates) before they each get hollowed at their `/next` gate.
6. **Record a DECISIONS.md entry** reconciling the pillar-1 moat with reality: EA target = 6-tier LLM-baked pyramid, or hand-authored 20-club slice? And whether the season runs the attribute match engine or stays seeded-procgen.
7. **Defer T4-7 and T4-9** until the foundation lands; sweep the P3 doc/honesty items opportunistically (most are one-line comment fixes).
